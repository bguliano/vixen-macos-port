//! Bounded whole-file float PCM fallback. The successful Symphonia path and its
//! MP3-specific trimming never pass through this module.
use super::AudioClip;
use crate::AudioError;
use std::ffi::OsString;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;
use std::time::Duration;
use vixen_media_tools::{collect, stream, Error, Tool};

const METADATA_LIMIT: usize = 1024 * 1024;
const PCM_LIMIT: usize = 2 * 1024 * 1024 * 1024;

fn error(error: Error) -> AudioError {
    match error {
        Error::Cancelled => AudioError::Cancelled,
        other => AudioError::Decode(other.to_string()),
    }
}

pub(super) fn load(path: &Path, cancel: &AtomicBool) -> Result<AudioClip, AudioError> {
    let mut probe: Vec<OsString> = [
        "-max_alloc",
        "67108864",
        "-select_streams",
        "a:0",
        "-show_entries",
        "stream=sample_rate,channels",
        "-of",
        "json",
        "-i",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    probe.push(path.as_os_str().into());
    let metadata = collect(
        Tool::Ffprobe,
        &probe,
        cancel,
        Some(Duration::from_secs(15)),
        METADATA_LIMIT,
    )
    .map_err(error)?;
    let value: serde_json::Value = serde_json::from_slice(&metadata)
        .map_err(|e| AudioError::Decode(format!("Invalid audio metadata: {e}")))?;
    let track = value
        .get("streams")
        .and_then(|v| v.as_array())
        .and_then(|v| v.first())
        .ok_or_else(|| AudioError::Unsupported("no audio track".into()))?;
    let sample_rate = track
        .get("sample_rate")
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|r| *r > 0)
        .ok_or_else(|| AudioError::Decode("Invalid audio sample rate".into()))?;
    let channels = track
        .get("channels")
        .and_then(|v| v.as_u64())
        .filter(|c| (1..=64).contains(c))
        .ok_or_else(|| AudioError::Decode("Audio channel count is outside 1–64".into()))?
        as u16;
    let mut args: Vec<OsString> = ["-xerror", "-max_alloc", "67108864", "-threads", "2", "-i"]
        .into_iter()
        .map(OsString::from)
        .collect();
    args.push(path.as_os_str().into());
    args.extend(
        [
            "-map",
            "0:a:0",
            "-vn",
            "-sn",
            "-dn",
            "-acodec",
            "pcm_f32le",
            "-ar",
        ]
        .into_iter()
        .map(OsString::from),
    );
    args.push(sample_rate.to_string().into());
    args.push("-ac".into());
    args.push(channels.to_string().into());
    args.extend(["-f", "f32le", "pipe:1"].into_iter().map(OsString::from));
    let mut pcm = Pcm::default();
    stream(
        Tool::Ffmpeg,
        &args,
        cancel,
        Some(Duration::from_secs(300)),
        |bytes| pcm.push(bytes),
    )
    .map_err(error)?;
    if !pcm.tail.is_empty() || pcm.samples.is_empty() || pcm.samples.len() % channels as usize != 0
    {
        return Err(AudioError::Decode("Incomplete audio sample frames".into()));
    }
    Ok(AudioClip {
        samples: pcm.samples,
        channels,
        sample_rate,
        declared_frames: None,
        concealed_frames: 0,
        mono: OnceLock::new(),
    })
}

#[derive(Default)]
struct Pcm {
    samples: Vec<f32>,
    tail: Vec<u8>,
}
impl Pcm {
    fn push(&mut self, mut bytes: &[u8]) -> Result<(), String> {
        let have = self
            .samples
            .len()
            .checked_mul(4)
            .and_then(|n| n.checked_add(self.tail.len()))
            .ok_or("Audio size overflow")?;
        if bytes.len() > PCM_LIMIT.saturating_sub(have) {
            return Err("Decoded audio exceeds the 2 GiB limit".into());
        }
        self.samples
            .try_reserve((bytes.len() + self.tail.len()) / 4)
            .map_err(|_| "Not enough memory for decoded audio")?;
        if !self.tail.is_empty() {
            let take = (4 - self.tail.len()).min(bytes.len());
            self.tail.extend_from_slice(&bytes[..take]);
            bytes = &bytes[take..];
            if self.tail.len() == 4 {
                self.samples
                    .push(f32::from_le_bytes(self.tail.as_slice().try_into().unwrap()));
                self.tail.clear();
            }
        }
        let mut chunks = bytes.chunks_exact(4);
        self.samples.extend(
            chunks
                .by_ref()
                .map(|b| f32::from_le_bytes(b.try_into().unwrap())),
        );
        self.tail.extend_from_slice(chunks.remainder());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_float_chunks_can_split_at_every_byte_boundary() {
        let expected = [0.0f32, -0.5, 0.25, 1.0, -1.0];
        let data: Vec<u8> = expected.iter().flat_map(|v| v.to_le_bytes()).collect();
        for size in 1..=9 {
            let mut pcm = Pcm::default();
            for chunk in data.chunks(size) {
                pcm.push(chunk).unwrap();
            }
            assert!(pcm.tail.is_empty());
            assert_eq!(pcm.samples, expected);
        }
    }
}
