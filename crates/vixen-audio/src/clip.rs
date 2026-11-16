//! Whole-file decode into interleaved f32.

use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_MP3, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use vixen_core::TimeSpan;
use vixen_effects::framework::AudioSource;

use crate::error::AudioError;

mod fallback;

const TICKS_PER_SEC: u128 = 10_000_000;

/// Frame count to `TimeSpan`, rounded to the nearest tick (exact whenever it divides evenly).
pub fn frames_to_time(frames: u64, sample_rate: u32) -> TimeSpan {
    if sample_rate == 0 {
        return TimeSpan::ZERO;
    }
    let sr = sample_rate as u128;
    TimeSpan::from_ticks(((frames as u128 * TICKS_PER_SEC + sr / 2) / sr) as i64)
}

/// `TimeSpan` to (fractional) frames. Negative times give 0.
pub fn time_to_frames(t: TimeSpan, sample_rate: u32) -> f64 {
    (t.ticks().max(0) as f64) / TICKS_PER_SEC as f64 * sample_rate as f64
}

/// The original reads MP3s through Windows Media Foundation, which removes this many frames of decoder delay from the
/// start and ignores any encoder delay in the file's LAME tag. Matching it keeps a sequence's timing with its music, and
/// makes the audio-reactive effects match the original: comparing a real export with this offset leaves 6 of 11,055
/// differing channel values, the rest of them gone.
const MP3_DECODER_DELAY_FRAMES: usize = 528;

/// A fully decoded audio file.
pub struct AudioClip {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
    declared_frames: Option<u64>,
    concealed_frames: u64,
    mono: OnceLock<Vec<f32>>,
}

impl AudioClip {
    /// Build a clip from interleaved samples (used by tests and by generators).
    pub fn from_interleaved(samples: Vec<f32>, channels: u16, sample_rate: u32) -> AudioClip {
        let channels = channels.max(1);
        AudioClip {
            samples,
            channels,
            sample_rate,
            declared_frames: None,
            concealed_frames: 0,
            mono: OnceLock::new(),
        }
    }

    /// Decode with Symphonia first. The bundled local media tools cover advertised
    /// container/codec paths the primary decoder cannot read, preserving the
    /// existing MP3 timeline and every successful primary decode unchanged.
    pub fn load(path: &Path) -> Result<AudioClip, AudioError> {
        Self::load_cancellable(path, &AtomicBool::new(false))
    }

    pub fn load_cancellable(path: &Path, cancel: &AtomicBool) -> Result<AudioClip, AudioError> {
        if cancel.load(Ordering::Acquire) {
            return Err(AudioError::Cancelled);
        }
        match Self::load_primary(path, cancel) {
            Ok(clip) => Ok(clip),
            Err(AudioError::Cancelled) => Err(AudioError::Cancelled),
            Err(primary) if !path.is_file() => Err(primary),
            Err(primary) => fallback::load(path, cancel).map_err(|error| match error {
                AudioError::Cancelled => AudioError::Cancelled,
                other => AudioError::Decode(format!("{primary}; fallback: {other}")),
            }),
        }
    }

    fn load_primary(path: &Path, cancel: &AtomicBool) -> Result<AudioClip, AudioError> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        // No gapless trimming, so the MP3 timeline stays where the original's is: see `MP3_DECODER_DELAY_FRAMES`.
        let fmt_opts = FormatOptions {
            enable_gapless: false,
            ..Default::default()
        };
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &MetadataOptions::default())
            .map_err(map_err)?;
        let mut format = probed.format;
        let track = format
            .default_track()
            .filter(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .or_else(|| {
                format
                    .tracks()
                    .iter()
                    .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            })
            .ok_or_else(|| AudioError::Unsupported("no audio track".into()))?;
        let track_id = track.id;
        let params = track.codec_params.clone();
        let mut decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(map_err)?;

        let declared_frames = params.n_frames;
        let mut out: Vec<f32> = Vec::new();
        if let (Some(n), Some(ch)) = (params.n_frames, params.channels) {
            out.reserve_exact((n as usize).saturating_mul(ch.count()).min(1 << 30));
        }
        let mut sample_buf: Option<SampleBuffer<f32>> = None;
        let mut channels = params.channels.map(|c| c.count() as u16).unwrap_or(0);
        let mut sample_rate = params.sample_rate.unwrap_or(0);
        let mut concealed_frames = 0u64;

        loop {
            if cancel.load(Ordering::Acquire) {
                return Err(AudioError::Cancelled);
            }
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(SymError::ResetRequired) => break,
                Err(e) => {
                    if out.is_empty() {
                        return Err(map_err(e));
                    }
                    break;
                }
            };
            if packet.track_id() != track_id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(buf) => {
                    let spec = *buf.spec();
                    if sample_buf.is_none() {
                        channels = spec.channels.count() as u16;
                        sample_rate = spec.rate;
                        sample_buf = Some(SampleBuffer::<f32>::new(buf.capacity() as u64, spec));
                    }
                    if spec.channels.count() as u16 != channels {
                        return Err(AudioError::Unsupported(
                            "channel count changed mid-stream".into(),
                        ));
                    }
                    let sb = sample_buf.as_mut().unwrap();
                    if (buf.capacity() * channels as usize) > sb.capacity() {
                        *sb = SampleBuffer::<f32>::new(buf.capacity() as u64, spec);
                    }
                    sb.copy_interleaved_ref(buf);
                    out.extend_from_slice(sb.samples());
                }
                // A corrupt packet (e.g. an mp3 frame whose bit reservoir points before the start of a
                // cut file). Keep the timeline intact: the show is timed to the audio, so substitute
                // silence for the packet's duration instead of dropping it.
                Err(SymError::DecodeError(_)) | Err(SymError::IoError(_)) => {
                    if channels > 0 && packet.dur > 0 {
                        out.resize(out.len() + packet.dur as usize * channels as usize, 0.0);
                        concealed_frames += packet.dur;
                    }
                    continue;
                }
                Err(e) => return Err(map_err(e)),
            }
        }
        if channels == 0 || sample_rate == 0 || out.is_empty() {
            return Err(AudioError::Decode("no audio decoded".into()));
        }
        if params.codec == CODEC_TYPE_MP3 {
            out.drain(..(MP3_DECODER_DELAY_FRAMES * channels as usize).min(out.len()));
        }
        Ok(AudioClip {
            samples: out,
            channels,
            sample_rate,
            declared_frames,
            concealed_frames,
            mono: OnceLock::new(),
        })
    }

    #[inline]
    pub fn channels(&self) -> u16 {
        self.channels
    }
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    /// Number of sample frames (samples per channel).
    #[inline]
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels as usize
    }
    pub fn duration(&self) -> TimeSpan {
        frames_to_time(self.frames() as u64, self.sample_rate)
    }
    /// Interleaved samples.
    #[inline]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
    /// Frame count the container claimed (None if it did not say). For checking decodes.
    pub fn declared_duration(&self) -> Option<TimeSpan> {
        self.declared_frames
            .map(|n| frames_to_time(n, self.sample_rate))
    }

    /// Frames of silence substituted for packets that failed to decode (normally 0).
    pub fn concealed_frames(&self) -> u64 {
        self.concealed_frames
    }

    /// Mono mixdown (average of channels), computed once and cached. A mono clip returns its own samples.
    pub fn mono(&self) -> &[f32] {
        if self.channels == 1 {
            return &self.samples;
        }
        self.mono.get_or_init(|| {
            let ch = self.channels as usize;
            let inv = 1.0 / ch as f32;
            self.samples
                .chunks_exact(ch)
                .map(|f| f.iter().sum::<f32>() * inv)
                .collect()
        })
    }

    /// A mono `AudioSource` view for effects.
    pub fn mono_view(&self) -> MonoView<'_> {
        MonoView(self)
    }
}

impl AudioSource for AudioClip {
    fn samples(&self) -> &[f32] {
        self.mono()
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn duration(&self) -> TimeSpan {
        AudioClip::duration(self)
    }
    fn interleaved(&self) -> &[f32] {
        AudioClip::samples(self)
    }
    fn channels(&self) -> u32 {
        AudioClip::channels(self) as u32
    }
}

/// Borrowed mono view of a clip.
pub struct MonoView<'a>(&'a AudioClip);

impl AudioSource for MonoView<'_> {
    fn samples(&self) -> &[f32] {
        self.0.mono()
    }
    fn sample_rate(&self) -> u32 {
        self.0.sample_rate
    }
    fn duration(&self) -> TimeSpan {
        self.0.duration()
    }
    fn interleaved(&self) -> &[f32] {
        self.0.samples()
    }
    fn channels(&self) -> u32 {
        self.0.channels() as u32
    }
}

fn map_err(e: SymError) -> AudioError {
    match e {
        SymError::IoError(e) => AudioError::Io(e.to_string()),
        SymError::Unsupported(s) => AudioError::Unsupported(s.to_string()),
        other => AudioError::Decode(other.to_string()),
    }
}
