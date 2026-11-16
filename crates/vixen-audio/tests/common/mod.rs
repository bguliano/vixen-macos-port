#![allow(dead_code)]
use std::path::PathBuf;
use std::sync::Arc;
use vixen_audio::AudioClip;

pub fn tmp_dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("vixen-audio-tests").join(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

pub fn write_wav_i16(path: &std::path::Path, channels: u16, rate: u32, data: &[i16]) {
    let bytes = data.len() * 2;
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + bytes as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    v.extend_from_slice(&(channels * 2).to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(bytes as u32).to_le_bytes());
    for s in data {
        v.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, v).unwrap();
}

pub fn write_wav_f32(path: &std::path::Path, channels: u16, rate: u32, data: &[f32]) {
    let bytes = data.len() * 4;
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + bytes as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&3u16.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * channels as u32 * 4).to_le_bytes());
    v.extend_from_slice(&(channels * 4).to_le_bytes());
    v.extend_from_slice(&32u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(bytes as u32).to_le_bytes());
    for s in data {
        v.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, v).unwrap();
}

/// Deterministic pseudo-random in -1..1.
pub struct Lcg(pub u64);
impl Lcg {
    pub fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
}

pub fn sine_clip(rate: u32, channels: u16, secs: f64, freq: f64, amp: f32) -> Arc<AudioClip> {
    let frames = (rate as f64 * secs) as usize;
    let mut s = Vec::with_capacity(frames * channels as usize);
    for i in 0..frames {
        let v = (amp as f64 * (2.0 * std::f64::consts::PI * freq * i as f64 / rate as f64).sin()) as f32;
        for _ in 0..channels {
            s.push(v);
        }
    }
    Arc::new(AudioClip::from_interleaved(s, channels, rate))
}

pub fn noise_clip(rate: u32, channels: u16, frames: usize, seed: u64) -> Arc<AudioClip> {
    let mut r = Lcg(seed);
    let s: Vec<f32> = (0..frames * channels as usize).map(|_| r.next_f32() * 0.9).collect();
    Arc::new(AudioClip::from_interleaved(s, channels, rate))
}
