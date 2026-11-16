//! How `BeatsAndBars.cs` gets its samples: `Audio.GetRawAudioSamples` plus the conversion in `DoBeatBarDetection`.
//!
//! This is not a mixdown, and the values are not normalised. `GetRawAudioSamples` returns whatever bytes the file's
//! reader stream produces, while `BytesPerSample` comes from the player's output format, which is always 32-bit float.
//! So every 4 raw bytes are read as one little-endian `Int32`, whatever they hold:
//!
//! - 16-bit stereo (every MP3 and M4A, through Media Foundation): each frame becomes `(right << 16) | (left as u16)`.
//!   The right channel decides the result, and one sample comes out per frame, so timing is right.
//! - 16-bit mono: two consecutive samples pack into one value, so the analysis sees the audio at double speed and every
//!   mark lands at half its true time.
//! - Anything else (24-bit, 32-bit float, Ogg Vorbis): the 4-byte reads cut across the real samples, or reinterpret float
//!   bits as integers. Vixen analyses that noise, and the time scale depends on the format.
//!
//! All of that is what the Windows build does, so it is ported as it stands.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// `Audio.BytesPerSample`. `CoreAudioPlayer` always outputs through `SampleToWaveProvider` as IEEE float.
pub const BYTES_PER_SAMPLE: usize = 4;

/// The layout of the bytes NAudio's reader stream hands to `AudioFileReader.ReadRaw`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum RawSampleFormat {
    Pcm8,
    Pcm16,
    Pcm24,
    Pcm32,
    Float32,
    Float64,
}

impl RawSampleFormat {
    /// `AudioFileReader.CreateReaderStream`: WAV and AIFF hand over the file's own PCM, Ogg Vorbis hands over 32-bit
    /// float, and MP3 and everything else go through Media Foundation, which decodes to 16-bit PCM.
    pub fn for_path(path: &Path) -> io::Result<RawSampleFormat> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        match extension.as_str() {
            "wav" => Self::read_wav_format(&mut File::open(path)?),
            "aiff" | "aif" => Self::read_aiff_format(&mut File::open(path)?),
            "ogg" => Ok(RawSampleFormat::Float32),
            _ => Ok(RawSampleFormat::Pcm16),
        }
    }

    fn read_wav_format<R: Read + Seek>(reader: &mut R) -> io::Result<RawSampleFormat> {
        let mut riff = [0u8; 12];
        reader.read_exact(&mut riff)?;
        if &riff[0..4] != b"RIFF" || &riff[8..12] != b"WAVE" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not a RIFF WAVE file"));
        }
        loop {
            let mut header = [0u8; 8];
            reader.read_exact(&mut header)?;
            let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
            if &header[0..4] == b"fmt " {
                let mut fmt = [0u8; 16];
                reader.read_exact(&mut fmt)?;
                let tag = u16::from_le_bytes([fmt[0], fmt[1]]);
                let bits = u16::from_le_bytes([fmt[14], fmt[15]]);
                return Ok(match (tag, bits) {
                    (1, 8) => RawSampleFormat::Pcm8,
                    (1, 16) => RawSampleFormat::Pcm16,
                    (1, 24) => RawSampleFormat::Pcm24,
                    (1, 32) => RawSampleFormat::Pcm32,
                    (3, 32) => RawSampleFormat::Float32,
                    (3, 64) => RawSampleFormat::Float64,
                    // Other encodings go through WaveFormatConversionStream.CreatePcmStream, normally to 16-bit.
                    _ => RawSampleFormat::Pcm16,
                });
            }
            // Chunks are word aligned.
            reader.seek(SeekFrom::Current(size as i64 + (size & 1) as i64))?;
        }
    }

    fn read_aiff_format<R: Read + Seek>(reader: &mut R) -> io::Result<RawSampleFormat> {
        let mut form = [0u8; 12];
        reader.read_exact(&mut form)?;
        if &form[0..4] != b"FORM" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not an AIFF file"));
        }
        loop {
            let mut header = [0u8; 8];
            reader.read_exact(&mut header)?;
            let size = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
            if &header[0..4] == b"COMM" {
                let mut comm = [0u8; 8];
                reader.read_exact(&mut comm)?;
                return Ok(match i16::from_be_bytes([comm[6], comm[7]]) {
                    8 => RawSampleFormat::Pcm8,
                    24 => RawSampleFormat::Pcm24,
                    32 => RawSampleFormat::Pcm32,
                    _ => RawSampleFormat::Pcm16,
                });
            }
            reader.seek(SeekFrom::Current(size as i64 + (size & 1) as i64))?;
        }
    }
}

/// `Audio.GetRawAudioSamples`, rebuilt from decoded interleaved samples such as `AudioClip::samples`. Decoding here is
/// not Media Foundation's, so for MP3 and M4A the PCM can differ slightly from what Windows gets.
pub fn raw_audio_bytes(interleaved: &[f32], format: RawSampleFormat) -> Vec<u8> {
    match format {
        RawSampleFormat::Pcm8 => {
            interleaved.iter().map(|&x| ((x * 128.0).round_ties_even() + 128.0).clamp(0.0, 255.0) as u8).collect()
        }
        RawSampleFormat::Pcm16 => {
            let mut bytes = Vec::with_capacity(interleaved.len() * 2);
            for &x in interleaved {
                let value = (x * 32768.0).round_ties_even().clamp(-32768.0, 32767.0) as i16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes
        }
        RawSampleFormat::Pcm24 => {
            let mut bytes = Vec::with_capacity(interleaved.len() * 3);
            for &x in interleaved {
                let value = (x as f64 * 8388608.0).round_ties_even().clamp(-8388608.0, 8388607.0) as i32;
                bytes.extend_from_slice(&value.to_le_bytes()[..3]);
            }
            bytes
        }
        RawSampleFormat::Pcm32 => {
            let mut bytes = Vec::with_capacity(interleaved.len() * 4);
            for &x in interleaved {
                let value = (x as f64 * 2147483648.0).round_ties_even().clamp(-2147483648.0, 2147483647.0) as i32;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes
        }
        RawSampleFormat::Float32 => interleaved.iter().flat_map(|x| x.to_le_bytes()).collect(),
        RawSampleFormat::Float64 => interleaved.iter().flat_map(|&x| (x as f64).to_le_bytes()).collect(),
    }
}

/// The loop in `DoBeatBarDetection`: `dataStep == 2 ? BitConverter.ToInt16 : BitConverter.ToInt32`, each assigned to a
/// `float`. When the bytes don't divide evenly the C# throws from `BitConverter`; here the leftover bytes are dropped.
pub fn float_samples_from_raw(raw: &[u8], bytes_per_sample: usize) -> Vec<f32> {
    if bytes_per_sample == 0 {
        return Vec::new();
    }

    let count = raw.len() / bytes_per_sample;
    let mut samples = Vec::with_capacity(count);
    for sample_num in 0..count {
        let j = sample_num * bytes_per_sample;
        let value = if bytes_per_sample == 2 {
            i16::from_le_bytes([raw[j], raw[j + 1]]) as i32
        } else if let Some(bytes) = raw.get(j..j + 4) {
            i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        } else {
            break;
        };
        samples.push(value as f32);
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn sixteen_bit_stereo_packs_right_over_left() {
        let raw = raw_audio_bytes(&[0.0, 0.5, -1.0, 0.25], RawSampleFormat::Pcm16);
        assert_eq!(raw, vec![0, 0, 0, 0x40, 0, 0x80, 0, 0x20]);
        let samples = float_samples_from_raw(&raw, BYTES_PER_SAMPLE);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0], (16384i32 << 16) as f32);
        assert_eq!(samples[1], ((8192i32 << 16) | 0x8000) as f32);
    }

    #[test]
    fn a_negative_left_channel_only_touches_the_low_bits() {
        let raw = raw_audio_bytes(&[-0.5, 0.5], RawSampleFormat::Pcm16);
        let samples = float_samples_from_raw(&raw, BYTES_PER_SAMPLE);
        assert_eq!(samples, vec![((16384i32 << 16) | 0xC000) as f32]);
    }

    #[test]
    fn sixteen_bit_mono_consumes_two_samples_per_value() {
        let raw = raw_audio_bytes(&[0.25, 0.5, 0.75, 0.125], RawSampleFormat::Pcm16);
        let samples = float_samples_from_raw(&raw, BYTES_PER_SAMPLE);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0], ((16384i32 << 16) | 8192) as f32);
    }

    #[test]
    fn leftover_bytes_are_dropped_rather_than_throwing() {
        let raw = raw_audio_bytes(&[0.25, 0.5, 0.75], RawSampleFormat::Pcm16);
        assert_eq!(float_samples_from_raw(&raw, BYTES_PER_SAMPLE).len(), 1);
        assert_eq!(float_samples_from_raw(&raw, 0).len(), 0);
    }

    #[test]
    fn the_two_byte_path_reads_int16() {
        let raw = raw_audio_bytes(&[0.5, -0.5], RawSampleFormat::Pcm16);
        assert_eq!(float_samples_from_raw(&raw, 2), vec![16384.0, -16384.0]);
    }

    #[test]
    fn clipping_and_rounding_of_pcm16() {
        let raw = raw_audio_bytes(&[1.5, -1.5, 1.0], RawSampleFormat::Pcm16);
        assert_eq!(raw, vec![0xFF, 0x7F, 0x00, 0x80, 0xFF, 0x7F]);
    }

    #[test]
    fn other_layouts_keep_their_width() {
        let input = [0.5f32, -0.25];
        assert_eq!(raw_audio_bytes(&input, RawSampleFormat::Pcm8), vec![192, 96]);
        assert_eq!(raw_audio_bytes(&input, RawSampleFormat::Pcm24), vec![0, 0, 0x40, 0, 0, 0xE0]);
        assert_eq!(raw_audio_bytes(&input, RawSampleFormat::Pcm32).len(), 8);
        assert_eq!(
            raw_audio_bytes(&input, RawSampleFormat::Float32),
            [0.5f32.to_le_bytes(), (-0.25f32).to_le_bytes()].concat()
        );
        assert_eq!(raw_audio_bytes(&input, RawSampleFormat::Float64).len(), 16);
    }

    fn wav_header(tag: u16, bits: u16) -> Vec<u8> {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF\0\0\0\0WAVE");
        wav.extend_from_slice(b"LIST\x03\0\0\0abc\0");
        wav.extend_from_slice(b"fmt \x10\0\0\0");
        wav.extend_from_slice(&tag.to_le_bytes());
        wav.extend_from_slice(&[2, 0, 0x44, 0xAC, 0, 0, 0, 0, 0, 0, 0, 0]);
        wav.extend_from_slice(&bits.to_le_bytes());
        wav
    }

    #[test]
    fn wav_format_comes_from_the_fmt_chunk() {
        let read = |tag, bits| RawSampleFormat::read_wav_format(&mut Cursor::new(wav_header(tag, bits))).unwrap();
        assert_eq!(read(1, 8), RawSampleFormat::Pcm8);
        assert_eq!(read(1, 16), RawSampleFormat::Pcm16);
        assert_eq!(read(1, 24), RawSampleFormat::Pcm24);
        assert_eq!(read(1, 32), RawSampleFormat::Pcm32);
        assert_eq!(read(3, 32), RawSampleFormat::Float32);
        assert_eq!(read(3, 64), RawSampleFormat::Float64);
        assert_eq!(read(0x55, 16), RawSampleFormat::Pcm16);
        assert!(RawSampleFormat::read_wav_format(&mut Cursor::new(b"nope".to_vec())).is_err());
    }

    #[test]
    fn aiff_format_comes_from_the_comm_chunk() {
        let mut aiff = Vec::new();
        aiff.extend_from_slice(b"FORM\0\0\0\0AIFF");
        aiff.extend_from_slice(b"COMM\0\0\0\x12");
        aiff.extend_from_slice(&[0, 2, 0, 0, 0, 0]);
        aiff.extend_from_slice(&24i16.to_be_bytes());
        assert_eq!(RawSampleFormat::read_aiff_format(&mut Cursor::new(aiff)).unwrap(), RawSampleFormat::Pcm24);
    }

    #[test]
    fn extension_picks_the_reader() {
        for name in ["a.mp3", "a.m4a", "a.MP3", "a.flac", "noext"] {
            assert_eq!(RawSampleFormat::for_path(Path::new(name)).unwrap(), RawSampleFormat::Pcm16, "{name}");
        }
        assert_eq!(RawSampleFormat::for_path(Path::new("a.OGG")).unwrap(), RawSampleFormat::Float32);
        assert!(RawSampleFormat::for_path(Path::new("/nonexistent/a.wav")).is_err());
    }
}
