//! The audio master clock and transport.
//!
//! Control calls (`play`, `seek`, ...) only touch atomics; the audio callback applies them at the start of
//! its next buffer. `position()` reads a seqlock-protected anchor written by the callback (media position
//! that becomes audible at a monotonic instant, plus the media rate) and extrapolates from the clock, so
//! it is smooth between callbacks, adjusted for output latency, and takes no locks.

use std::sync::atomic::{fence, AtomicBool, AtomicI64, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use vixen_core::TimeSpan;

use crate::backend::{Backend, Clock, RenderFn, Stream, StreamFormat, StreamRequest};
use crate::clip::{frames_to_time, time_to_frames, AudioClip};
use crate::cpal_backend::CpalBackend;
use crate::error::AudioError;
use crate::wsola::{Src, Wsola};

/// Output frames processed per inner pass (bounds the scratch buffers).
const CHUNK: usize = 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerState {
    /// No clip loaded.
    Empty,
    Stopped,
    Paused,
    Playing,
    /// The device failed or could not be opened. `play()` retries.
    Error(AudioError),
}

const T_STOPPED: u8 = 0;
const T_PAUSED: u8 = 1;
const T_PLAYING: u8 = 2;

struct Shared {
    clock: Arc<dyn Clock>,
    playing: AtomicBool,
    ended: AtomicBool,
    /// EOF may be queued ahead of what is audible. Keep the clock advancing until
    /// the final sample reaches the output, including device latency.
    ended_at_ns: AtomicU64,
    transport: AtomicU8,
    volume: AtomicU32,
    /// Speed bits plus the pitch-preserving mode, committed together by `set_speed`.
    speed: AtomicU64,
    /// Requested media frame, or -1.
    pending_seek: AtomicI64,
    /// Bumped on every discontinuity so the monotonic clamp can restart.
    jump_epoch: AtomicU32,
    clip_rate: AtomicU32,
    clip_frames: AtomicU64,
    frames_rendered: AtomicU64,
    // seqlock-protected anchor
    seq: AtomicU64,
    a_media: AtomicU64,
    a_ns: AtomicU64,
    a_rate: AtomicU64,
    a_epoch: AtomicU32,
    /// Last value returned by `position()`: epoch (16 bits) | ticks (48 bits).
    last: AtomicU64,
    error: Mutex<Option<AudioError>>,
    has_error: AtomicBool,
}

impl Shared {
    fn has_ended(&self) -> bool {
        self.ended.load(Ordering::Acquire) && self.pending_seek.load(Ordering::Acquire) < 0
            && self.clock.now_ns() >= self.ended_at_ns.load(Ordering::Relaxed)
    }
    fn new(clock: Arc<dyn Clock>) -> Shared {
        Shared {
            clock,
            playing: AtomicBool::new(false),
            ended: AtomicBool::new(false),
            ended_at_ns: AtomicU64::new(0),
            transport: AtomicU8::new(T_STOPPED),
            volume: AtomicU32::new(1.0f32.to_bits()),
            speed: AtomicU64::new(1.0f32.to_bits() as u64 | (1 << 32)),
            pending_seek: AtomicI64::new(-1),
            jump_epoch: AtomicU32::new(0),
            clip_rate: AtomicU32::new(0),
            clip_frames: AtomicU64::new(0),
            frames_rendered: AtomicU64::new(0),
            seq: AtomicU64::new(0),
            a_media: AtomicU64::new(0),
            a_ns: AtomicU64::new(0),
            a_rate: AtomicU64::new(0),
            a_epoch: AtomicU32::new(0),
            last: AtomicU64::new(0),
            error: Mutex::new(None),
            has_error: AtomicBool::new(false),
        }
    }

    /// Single writer: the audio callback (or, while no stream exists, the control thread).
    #[inline]
    fn publish(&self, media: f64, ns: u64, rate: f64, epoch: u32) {
        let s = self.seq.load(Ordering::Relaxed);
        self.seq.store(s.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        self.a_media.store(media.to_bits(), Ordering::Relaxed);
        self.a_ns.store(ns, Ordering::Relaxed);
        self.a_rate.store(rate.to_bits(), Ordering::Relaxed);
        self.a_epoch.store(epoch, Ordering::Relaxed);
        self.seq.store(s.wrapping_add(2), Ordering::Release);
    }

    #[inline]
    fn snapshot(&self) -> (f64, u64, f64, u32) {
        loop {
            let s1 = self.seq.load(Ordering::Acquire);
            if s1 & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }
            let m = f64::from_bits(self.a_media.load(Ordering::Relaxed));
            let ns = self.a_ns.load(Ordering::Relaxed);
            let r = f64::from_bits(self.a_rate.load(Ordering::Relaxed));
            let e = self.a_epoch.load(Ordering::Relaxed);
            fence(Ordering::Acquire);
            if self.seq.load(Ordering::Relaxed) == s1 {
                return (m, ns, r, e);
            }
        }
    }

    fn position(&self) -> TimeSpan {
        let rate = self.clip_rate.load(Ordering::Relaxed);
        if rate == 0 {
            return TimeSpan::ZERO;
        }
        let total = self.clip_frames.load(Ordering::Relaxed) as f64;
        let pend = self.pending_seek.load(Ordering::Acquire);
        let (frames, epoch) = if pend >= 0 {
            (pend as f64, self.jump_epoch.load(Ordering::Acquire))
        } else {
            let (m, ns, r, e) = self.snapshot();
            let live = if self.playing.load(Ordering::Relaxed) { r } else { 0.0 };
            let now = self.clock.now_ns();
            let dt = (now as f64 - ns as f64) * 1e-9;
            (m + dt * live, e)
        };
        let frames = frames.clamp(0.0, total);
        let ticks = frames_to_time(frames.round() as u64, rate).ticks() as u64 & ((1 << 48) - 1);
        let ep = (epoch & 0xFFFF) as u64;
        // Never step backwards within one epoch.
        let mut cur = self.last.load(Ordering::Relaxed);
        loop {
            let (ce, cv) = (cur >> 48, cur & ((1 << 48) - 1));
            if ce == ep && cv >= ticks {
                return TimeSpan::from_ticks(cv as i64);
            }
            match self.last.compare_exchange_weak(cur, (ep << 48) | ticks, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return TimeSpan::from_ticks(ticks as i64),
                Err(c) => cur = c,
            }
        }
    }

    fn jump_to(&self, frame: i64) {
        self.jump_epoch.fetch_add(1, Ordering::AcqRel);
        self.pending_seek.store(frame, Ordering::Release);
    }
}

struct Inner {
    backend: Box<dyn Backend>,
    device: Option<String>,
    clip: Option<Arc<AudioClip>>,
    stream: Option<Box<dyn Stream>>,
    format: Option<StreamFormat>,
    latency_ms: Option<u32>,
}

/// Plays one clip through an output device and serves as the show's master clock.
pub struct AudioPlayer {
    shared: Arc<Shared>,
    inner: Mutex<Inner>,
    /// Like upstream UseTempo, changing this selects the mode for the next PlaybackRate assignment.
    rate_mode: AtomicBool,
}

impl AudioPlayer {
    /// Real CoreAudio output. `device` is a name from [`crate::output_devices`], or None for the default.
    /// A missing device is reported through `state()`, not a panic; nothing is opened until a clip is set.
    pub fn new(device: Option<&str>) -> AudioPlayer {
        Self::with_backend(Box::new(CpalBackend::new()), device)
    }

    /// The Options dialog's Latency. Takes effect when the next stream opens, as upstream's does when the next output opens.
    pub fn set_latency_ms(&self, ms: u32) {
        self.inner.lock().latency_ms = Some(ms);
    }

    pub fn with_backend(backend: Box<dyn Backend>, device: Option<&str>) -> AudioPlayer {
        let shared = Arc::new(Shared::new(backend.clock()));
        let p = AudioPlayer {
            shared,
            rate_mode: AtomicBool::new(true),
            inner: Mutex::new(Inner {
                backend,
                device: device.map(|s| s.to_string()),
                clip: None,
                stream: None,
                format: None,
                latency_ms: None,
            }),
        };
        if let Some(name) = device {
            let inner = p.inner.lock();
            if !inner.backend.output_devices().iter().any(|d| d == name) {
                p.set_error(AudioError::NoDevice(name.to_string()));
            }
        }
        p
    }

    fn set_error(&self, e: AudioError) {
        self.shared.playing.store(false, Ordering::SeqCst);
        *self.shared.error.lock() = Some(e);
        self.shared.has_error.store(true, Ordering::SeqCst);
    }

    fn clear_error(&self) {
        self.shared.has_error.store(false, Ordering::SeqCst);
        *self.shared.error.lock() = None;
    }

    /// Load a clip (stopped, at 0). Reopens the output at the clip's sample rate where the device allows.
    pub fn set_clip(&self, clip: Arc<AudioClip>) {
        let mut inner = self.inner.lock();
        // Dropping the stream first guarantees the old callback has finished.
        inner.stream = None;
        let sh = &self.shared;
        sh.playing.store(false, Ordering::SeqCst);
        sh.ended.store(false, Ordering::SeqCst);
        sh.transport.store(T_STOPPED, Ordering::SeqCst);
        sh.clip_rate.store(clip.sample_rate(), Ordering::SeqCst);
        sh.clip_frames.store(clip.frames() as u64, Ordering::SeqCst);
        sh.publish(0.0, sh.clock.now_ns(), 0.0, sh.jump_epoch.load(Ordering::SeqCst) + 1);
        sh.jump_to(0);
        clip.mono(); // prime the cached mixdown so the callback never computes it
        inner.clip = Some(clip);
        self.clear_error();
        self.open_locked(&mut inner);
    }

    /// Switch output device (None = default), keeping clip, position and play state.
    pub fn set_device(&self, device: Option<&str>) {
        let mut inner = self.inner.lock();
        inner.device = device.map(|s| s.to_string());
        let was_playing = self.shared.transport.load(Ordering::SeqCst) == T_PLAYING;
        self.reopen_locked(&mut inner, was_playing);
    }

    /// Retry opening the device after an error, keeping position.
    pub fn reopen(&self) {
        let mut inner = self.inner.lock();
        let was_playing = self.shared.transport.load(Ordering::SeqCst) == T_PLAYING;
        self.reopen_locked(&mut inner, was_playing);
    }

    fn reopen_locked(&self, inner: &mut Inner, resume: bool) {
        if inner.clip.is_none() {
            inner.stream = None;
            self.clear_error();
            return;
        }
        let pos = self.shared.position();
        let rate = self.shared.clip_rate.load(Ordering::SeqCst);
        inner.stream = None;
        self.clear_error();
        let frame = time_to_frames(pos, rate).round() as i64;
        self.shared.jump_to(frame);
        self.open_locked(inner);
        if resume && !self.shared.has_error.load(Ordering::SeqCst) {
            self.shared.playing.store(true, Ordering::SeqCst);
        }
    }

    fn open_locked(&self, inner: &mut Inner) {
        let clip = match &inner.clip {
            Some(c) => c.clone(),
            None => return,
        };
        let shared = self.shared.clone();
        let err_shared = self.shared.clone();
        let request = StreamRequest { sample_rate: clip.sample_rate(), channels: clip.channels().min(2), latency_ms: inner.latency_ms };
        let factory: crate::backend::RenderFactory = Box::new(move |fmt| make_render(shared, clip, fmt));
        let on_error: crate::backend::ErrorFn = Box::new(move |e| {
            err_shared.playing.store(false, Ordering::SeqCst);
            *err_shared.error.lock() = Some(e);
            err_shared.has_error.store(true, Ordering::SeqCst);
        });
        let device = inner.device.clone();
        match inner.backend.open(device.as_deref(), request, factory, on_error) {
            Ok((stream, fmt)) => {
                inner.stream = Some(stream);
                inner.format = Some(fmt);
            }
            Err(e) => {
                inner.stream = None;
                self.set_error(e);
            }
        }
    }

    pub fn play(&self) {
        let sh = &self.shared;
        if sh.clip_rate.load(Ordering::SeqCst) == 0 {
            return;
        }
        if sh.has_error.load(Ordering::SeqCst) {
            self.reopen();
            if sh.has_error.load(Ordering::SeqCst) {
                return;
            }
        }
        // A seek after end may still be waiting for the audio callback. Preserve it when
        // Play follows immediately (including a loop with a nonzero start).
        if self.has_ended() {
            sh.jump_to(0);
        }
        sh.transport.store(T_PLAYING, Ordering::SeqCst);
        sh.playing.store(true, Ordering::SeqCst);
    }

    pub fn pause(&self) {
        self.shared.transport.store(T_PAUSED, Ordering::SeqCst);
        self.shared.playing.store(false, Ordering::SeqCst);
    }

    /// Pause and rewind to the start.
    pub fn stop(&self) {
        self.shared.transport.store(T_STOPPED, Ordering::SeqCst);
        self.shared.playing.store(false, Ordering::SeqCst);
        self.shared.jump_to(0);
    }

    pub fn seek(&self, t: TimeSpan) {
        let rate = self.shared.clip_rate.load(Ordering::SeqCst);
        if rate == 0 {
            return;
        }
        let total = self.shared.clip_frames.load(Ordering::SeqCst) as f64;
        let f = time_to_frames(t, rate).round().clamp(0.0, total);
        self.shared.jump_to(f as i64);
    }

    /// 0.0..=1.0 (clamped).
    pub fn set_volume(&self, v: f32) {
        let v = if v.is_finite() { v.clamp(0.0, 1.0) } else { 1.0 };
        self.shared.volume.store(v.to_bits(), Ordering::Relaxed);
    }

    /// Select pitch-preserving tempo or pitch-changing rate for the next `set_speed` call.
    /// Changing the flag alone leaves the current processing unchanged, like upstream's UseTempo.
    pub fn set_rate_mode(&self, preserve_pitch: bool) {
        self.rate_mode.store(preserve_pitch, Ordering::Relaxed);
    }

    /// 0.1..=4.0 (clamped). Exactly 1.0 bypasses processing in either mode.
    pub fn set_speed(&self, s: f32) {
        let s = if s.is_finite() { s.clamp(0.1, 4.0) } else { 1.0 };
        let mode = u64::from(self.rate_mode.load(Ordering::Relaxed)) << 32;
        self.shared.speed.store(s.to_bits() as u64 | mode, Ordering::Relaxed);
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.shared.volume.load(Ordering::Relaxed))
    }
    pub fn speed(&self) -> f32 {
        f32::from_bits(self.shared.speed.load(Ordering::Relaxed) as u32)
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed) && !self.has_ended()
    }

    /// True once the clip played to its end (cleared by seek/play).
    pub fn has_ended(&self) -> bool {
        self.shared.has_ended()
    }

    /// The master clock: media time audible now. Lock-free, monotonic between seeks.
    pub fn position(&self) -> TimeSpan {
        self.shared.position()
    }

    /// A cheap handle for reading the clock from other threads.
    pub fn clock_handle(&self) -> ClockHandle {
        ClockHandle { shared: self.shared.clone() }
    }

    pub fn duration(&self) -> TimeSpan {
        let r = self.shared.clip_rate.load(Ordering::Relaxed);
        frames_to_time(self.shared.clip_frames.load(Ordering::Relaxed), r)
    }

    pub fn state(&self) -> PlayerState {
        if self.shared.has_error.load(Ordering::SeqCst) {
            if let Some(e) = self.shared.error.lock().clone() {
                return PlayerState::Error(e);
            }
        }
        if self.shared.clip_rate.load(Ordering::SeqCst) == 0 {
            return PlayerState::Empty;
        }
        match self.shared.transport.load(Ordering::SeqCst) {
            T_PLAYING if self.is_playing() => PlayerState::Playing,
            T_PLAYING => PlayerState::Stopped, // ran off the end
            T_PAUSED => PlayerState::Paused,
            _ => PlayerState::Stopped,
        }
    }

    pub fn last_error(&self) -> Option<AudioError> {
        self.shared.error.lock().clone()
    }

    /// Frames the device has been handed since the clip was set (diagnostics).
    pub fn frames_rendered(&self) -> u64 {
        self.shared.frames_rendered.load(Ordering::Relaxed)
    }

    /// The format the device is actually running at, if open.
    pub fn stream_format(&self) -> Option<StreamFormat> {
        self.inner.lock().format
    }
}

/// Lock-free read access to the master clock.
#[derive(Clone)]
pub struct ClockHandle {
    shared: Arc<Shared>,
}

impl ClockHandle {
    pub fn position(&self) -> TimeSpan {
        self.shared.position()
    }
    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed) && !self.shared.has_ended()
    }
}

// ---------------------------------------------------------------------------------------------
// The audio callback
// ---------------------------------------------------------------------------------------------

struct Engine {
    shared: Arc<Shared>,
    clip: Arc<AudioClip>,
    in_ch: usize,
    out_ch: usize,
    clip_rate: f64,
    /// Clip-rate frames consumed per output frame (1.0 when rates match).
    step: f64,
    resample: bool,
    pos: f64,
    epoch: u32,
    ended: bool,
    wsola: Wsola,
    wsola_active: bool,
    gain: f32,
    tmp: Vec<f32>,
    ext: Vec<f32>,
    mix: Vec<f32>,
    last_frame: Vec<f32>,
    frac: f64,
    filt_off: f64,
    filt_valid: bool,
    filt_speed: f32,
}

fn make_render(shared: Arc<Shared>, clip: Arc<AudioClip>, fmt: StreamFormat) -> RenderFn {
    let in_ch = clip.channels() as usize;
    let out_ch = fmt.channels.max(1) as usize;
    let clip_rate = clip.sample_rate() as f64;
    let step = clip_rate / fmt.sample_rate as f64;
    let resample = clip.sample_rate() != fmt.sample_rate;
    let pulled_max = (CHUNK as f64 * step).ceil() as usize + 4;
    let volume = f32::from_bits(shared.volume.load(Ordering::Relaxed));
    let epoch = shared.jump_epoch.load(Ordering::SeqCst);
    let mut e = Engine {
        wsola: Wsola::new(clip.sample_rate(), in_ch),
        shared,
        in_ch,
        out_ch,
        clip_rate,
        step,
        resample,
        pos: 0.0,
        epoch,
        ended: false,
        wsola_active: false,
        gain: volume,
        tmp: vec![0.0; pulled_max * in_ch],
        ext: vec![0.0; (pulled_max + 1) * in_ch],
        mix: vec![0.0; CHUNK * in_ch],
        last_frame: vec![0.0; in_ch],
        frac: 0.0,
        filt_off: 0.0,
        filt_valid: false,
        filt_speed: 1.0,
        clip,
    };
    Box::new(move |out, latency| e.render(out, latency))
}

fn map_channels(src: &[f32], in_ch: usize, dst: &mut [f32], out_ch: usize, n: usize) {
    if in_ch == out_ch {
        dst[..n * out_ch].copy_from_slice(&src[..n * in_ch]);
    } else if in_ch == 1 {
        for i in 0..n {
            let v = src[i];
            for c in 0..out_ch {
                dst[i * out_ch + c] = v;
            }
        }
    } else if out_ch == 1 {
        let inv = 1.0 / in_ch as f32;
        for i in 0..n {
            let mut s = 0.0;
            for c in 0..in_ch {
                s += src[i * in_ch + c];
            }
            dst[i] = s * inv;
        }
    } else {
        for i in 0..n {
            for c in 0..out_ch {
                dst[i * out_ch + c] = if c < in_ch { src[i * in_ch + c] } else { 0.0 };
            }
        }
    }
}

impl Engine {
    fn render(&mut self, out: &mut [f32], latency: Duration) {
        let shared = Arc::clone(&self.shared); // refcount bump only, no allocation
        let sh = &*shared;
        let out_ch = self.out_ch;
        let frames_out = out.len() / out_ch;
        let total = self.clip.frames() as f64;
        let now = sh.clock.now_ns();
        let anchor_ns = now + latency.as_nanos() as u64;

        let pend = sh.pending_seek.load(Ordering::Acquire);
        if pend >= 0 {
            self.epoch = sh.jump_epoch.load(Ordering::Acquire);
            self.pos = (pend as f64).min(total);
            self.wsola_active = false;
            self.frac = 0.0;
            self.filt_valid = false;
            self.last_frame.iter_mut().for_each(|v| *v = 0.0);
            self.ended = self.pos >= total;
            sh.ended_at_ns.store(now, Ordering::Relaxed);
            sh.ended.store(self.ended, Ordering::SeqCst);
        }
        // Later silence buffers must not replace the final audible clock anchor.
        if self.ended && pend < 0 {
            out.fill(0.0);
            sh.frames_rendered.fetch_add(frames_out as u64, Ordering::Relaxed);
            return;
        }
        let playing = sh.playing.load(Ordering::SeqCst) && !self.ended;
        let control = sh.speed.load(Ordering::Relaxed);
        let speed = f32::from_bits(control as u32);
        let preserve_pitch = control & (1 << 32) != 0;
        let rate = if playing { speed as f64 * self.clip_rate } else { 0.0 };
        // Smooth callback-timing jitter: the clock-domain instant at which media time 0 would play is
        // constant for steady playback, so low-pass it instead of trusting each callback's timestamp.
        let mut ns_pub = anchor_ns;
        if playing {
            let media_ns = self.pos / self.clip_rate / speed as f64 * 1e9;
            let off_raw = anchor_ns as f64 - media_ns;
            if !self.filt_valid || self.filt_speed != speed || (off_raw - self.filt_off).abs() > 50e6 {
                self.filt_off = off_raw;
                self.filt_valid = true;
                self.filt_speed = speed;
            } else {
                self.filt_off += 0.1 * (off_raw - self.filt_off);
            }
            ns_pub = (self.filt_off + media_ns).max(0.0) as u64;
        } else {
            self.filt_valid = false;
        }
        sh.publish(self.pos, ns_pub, rate, self.epoch);
        let eof_at_ns = if rate > 0.0 { ns_pub.saturating_add(((total - self.pos).max(0.0) / rate * 1e9).ceil() as u64) } else { now };
        if pend >= 0 {
            let _ = sh.pending_seek.compare_exchange(pend, -1, Ordering::AcqRel, Ordering::Relaxed);
        }
        if !playing {
            out.fill(0.0);
            sh.frames_rendered.fetch_add(frames_out as u64, Ordering::Relaxed);
            return;
        }

        let bypass = (speed - 1.0).abs() < 1e-6;
        if !bypass && preserve_pitch && !self.wsola_active {
            self.wsola.reset(self.pos);
            self.wsola_active = true;
        } else if bypass || !preserve_pitch {
            self.wsola_active = false;
        }
        let volume = f32::from_bits(sh.volume.load(Ordering::Relaxed));

        let mut done = 0;
        while done < frames_out {
            let n = (frames_out - done).min(CHUNK);
            let in_ch = self.in_ch;
            let pulled = if self.resample {
                let a = (self.frac + n as f64 * self.step).floor() as usize;
                let b = (self.frac + (n - 1) as f64 * self.step).floor() as usize + 1;
                a.max(b)
            } else {
                n
            };
            let got = self.produce(pulled, speed, bypass, preserve_pitch);

            let dst = &mut out[done * out_ch..(done + n) * out_ch];
            if self.resample {
                self.ext[..in_ch].copy_from_slice(&self.last_frame);
                self.ext[in_ch..in_ch + pulled * in_ch].copy_from_slice(&self.tmp[..pulled * in_ch]);
                let mut t = self.frac;
                for j in 0..n {
                    let i = t as usize;
                    let f = (t - i as f64) as f32;
                    for c in 0..in_ch {
                        let a = self.ext[i * in_ch + c];
                        let b = self.ext[(i + 1) * in_ch + c];
                        self.mix[j * in_ch + c] = a + (b - a) * f;
                    }
                    t += self.step;
                }
                let adv = t.floor() as usize;
                self.frac = t - adv as f64;
                let (ext, last) = (&self.ext, &mut self.last_frame);
                last.copy_from_slice(&ext[adv * in_ch..(adv + 1) * in_ch]);
                map_channels(&self.mix, in_ch, dst, out_ch, n);
            } else {
                map_channels(&self.tmp, in_ch, dst, out_ch, n);
            }

            // Volume, ramped across the chunk when it changes.
            if self.gain != volume {
                let g0 = self.gain;
                for j in 0..n {
                    let g = g0 + (volume - g0) * ((j + 1) as f32 / n as f32);
                    for c in 0..out_ch {
                        dst[j * out_ch + c] *= g;
                    }
                }
                self.gain = volume;
            } else if volume != 1.0 {
                for v in dst.iter_mut() {
                    *v *= volume;
                }
            }

            done += n;
            if got < pulled {
                self.ended = true;
                self.pos = self.pos.min(total);
                out[done * out_ch..].fill(0.0);
                done = frames_out;
                sh.ended_at_ns.store(eof_at_ns, Ordering::Relaxed);
                sh.ended.store(true, Ordering::Release);
            }
        }
        sh.frames_rendered.fetch_add(frames_out as u64, Ordering::Relaxed);
    }

    /// Produce `frames` clip-rate frames into `tmp` (zero padded). Returns how many were real.
    fn produce(&mut self, frames: usize, speed: f32, bypass: bool, preserve_pitch: bool) -> usize {
        let in_ch = self.in_ch;
        let tmp = &mut self.tmp[..frames * in_ch];
        if bypass {
            let total = self.clip.frames();
            let start = (self.pos.round() as usize).min(total);
            let avail = (total - start).min(frames);
            tmp[..avail * in_ch].copy_from_slice(&self.clip.samples()[start * in_ch..(start + avail) * in_ch]);
            tmp[avail * in_ch..].fill(0.0);
            self.pos = (start + avail) as f64;
            avail
        } else if preserve_pitch {
            let src = Src::of(&self.clip);
            let w = self.wsola.render(src, speed, tmp);
            tmp[w * in_ch..].fill(0.0);
            self.pos = self.wsola.position().min(self.clip.frames() as f64);
            w
        } else {
            // Rate mode consumes the source at the requested rate: pitch moves with speed.
            // Interpolation also handles sub-unit speeds and stereo without allocating in the callback.
            let total = self.clip.frames();
            let samples = self.clip.samples();
            let available = (((total as f64 - self.pos).max(0.0) / speed as f64).ceil() as usize).min(frames);
            for frame in 0..available {
                let index = (self.pos.floor() as usize).min(total - 1);
                let next = (index + 1).min(total - 1);
                let fraction = (self.pos - index as f64) as f32;
                for channel in 0..in_ch {
                    let a = samples[index * in_ch + channel];
                    let b = samples[next * in_ch + channel];
                    tmp[frame * in_ch + channel] = a + (b - a) * fraction;
                }
                self.pos = (self.pos + speed as f64).min(total as f64);
            }
            tmp[available * in_ch..].fill(0.0);
            available
        }
    }
}
