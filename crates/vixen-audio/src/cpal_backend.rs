//! CoreAudio output through cpal.

use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SampleRate, SizedSample};

use crate::backend::*;
use crate::error::AudioError;

/// Largest chunk converted per pass for non-f32 devices (preallocated, so the callback never allocates).
const CONVERT_FRAMES: usize = 4096;

pub struct CpalBackend {
    clock: Arc<SystemClock>,
}

impl CpalBackend {
    pub fn new() -> CpalBackend {
        CpalBackend { clock: Arc::new(SystemClock::new()) }
    }
}

impl Default for CpalBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Names of the available output devices.
pub fn output_devices() -> Vec<String> {
    CpalBackend::new().output_devices()
}

fn find_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device, AudioError> {
    match name {
        None => host
            .default_output_device()
            .ok_or_else(|| AudioError::NoDevice("no default output device".into())),
        Some(n) => host
            .output_devices()
            .map_err(|e| AudioError::Device(e.to_string()))?
            .find(|d| d.name().map(|dn| dn == n).unwrap_or(false))
            .ok_or_else(|| AudioError::NoDevice(n.to_string())),
    }
}

/// `latency_ms` of audio in frames, kept within what the device allows.
fn buffer_frames(latency_ms: u32, sample_rate: u32, range: &cpal::SupportedBufferSize) -> u32 {
    let frames = (latency_ms as u64 * sample_rate as u64 / 1000).min(u32::MAX as u64) as u32;
    match range {
        cpal::SupportedBufferSize::Range { min, max } => frames.clamp(*min, (*max).max(*min)),
        cpal::SupportedBufferSize::Unknown => frames,
    }
}

fn pick_config(device: &cpal::Device, req: StreamRequest) -> Result<cpal::SupportedStreamConfig, AudioError> {
    let mut best: Option<(i32, cpal::SupportedStreamConfig)> = None;
    if let Ok(ranges) = device.supported_output_configs() {
        for r in ranges {
            if !matches!(r.sample_format(), SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16) {
                continue;
            }
            if r.min_sample_rate().0 <= req.sample_rate && req.sample_rate <= r.max_sample_rate().0 {
                let mut score = 0;
                if r.sample_format() == SampleFormat::F32 {
                    score += 4;
                }
                if r.channels() == req.channels {
                    score += 2;
                } else if r.channels() == 2 {
                    score += 1;
                }
                if best.as_ref().map(|b| score > b.0).unwrap_or(true) {
                    best = Some((score, r.with_sample_rate(SampleRate(req.sample_rate))));
                }
            }
        }
    }
    match best {
        Some((_, c)) => Ok(c),
        None => device.default_output_config().map_err(|e| AudioError::Device(e.to_string())),
    }
}

struct CpalStream(cpal::Stream);

// SAFETY: a cpal stream is only ever played, paused and dropped through `&mut self`/ownership, one
// thread at a time, which CoreAudio permits. The render closure is `Send`.
unsafe impl Send for CpalStream {}

impl Stream for CpalStream {
    fn play(&mut self) -> Result<(), AudioError> {
        self.0.play().map_err(|e| AudioError::Device(e.to_string()))
    }
    fn pause(&mut self) -> Result<(), AudioError> {
        self.0.pause().map_err(|e| AudioError::Device(e.to_string()))
    }
}

fn latency_of(info: &cpal::OutputCallbackInfo) -> Duration {
    let ts = info.timestamp();
    ts.playback.duration_since(&ts.callback).unwrap_or(Duration::ZERO)
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut render: RenderFn,
    on_error: Arc<ErrorFn>,
) -> Result<cpal::Stream, AudioError>
where
    T: SizedSample + FromSample<f32> + Send + 'static,
{
    let ch = config.channels as usize;
    let mut scratch = vec![0.0f32; CONVERT_FRAMES * ch];
    let err_fn = move |e: cpal::StreamError| on_error(AudioError::Device(e.to_string()));
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
                let latency = latency_of(info);
                for chunk in data.chunks_mut(CONVERT_FRAMES * ch) {
                    let tmp = &mut scratch[..chunk.len()];
                    render(tmp, latency);
                    for (o, s) in chunk.iter_mut().zip(tmp.iter()) {
                        *o = T::from_sample(*s);
                    }
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| AudioError::Device(e.to_string()))?;
    Ok(stream)
}

impl Backend for CpalBackend {
    fn output_devices(&self) -> Vec<String> {
        let host = cpal::default_host();
        match host.output_devices() {
            Ok(it) => it.filter_map(|d| d.name().ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    fn clock(&self) -> Arc<dyn Clock> {
        self.clock.clone()
    }

    fn open(
        &mut self,
        device: Option<&str>,
        request: StreamRequest,
        factory: RenderFactory,
        on_error: ErrorFn,
    ) -> Result<(Box<dyn Stream>, StreamFormat), AudioError> {
        let host = cpal::default_host();
        let device = find_device(&host, device)?;
        let supported = pick_config(&device, request)?;
        let format = StreamFormat { sample_rate: supported.sample_rate().0, channels: supported.channels() };
        let sample_format = supported.sample_format();
        let buffer_range = supported.buffer_size().clone();
        let mut config: cpal::StreamConfig = supported.into();
        if let Some(ms) = request.latency_ms {
            config.buffer_size = cpal::BufferSize::Fixed(buffer_frames(ms, format.sample_rate, &buffer_range));
        }
        let on_error = Arc::new(on_error);

        let render = factory(format);
        let stream = match sample_format {
            SampleFormat::F32 => {
                let mut render = render;
                let err = on_error.clone();
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], info: &cpal::OutputCallbackInfo| render(data, latency_of(info)),
                        move |e| err(AudioError::Device(e.to_string())),
                        None,
                    )
                    .map_err(|e| AudioError::Device(e.to_string()))?
            }
            SampleFormat::I16 => build::<i16>(&device, &config, render, on_error)?,
            SampleFormat::U16 => build::<u16>(&device, &config, render, on_error)?,
            other => return Err(AudioError::Unsupported(format!("device sample format {other:?}"))),
        };
        let mut s = CpalStream(stream);
        s.play()?;
        Ok((Box::new(s), format))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_becomes_frames_within_the_devices_range() {
        let range = cpal::SupportedBufferSize::Range { min: 64, max: 4096 };
        assert_eq!(buffer_frames(25, 48000, &range), 1200);
        assert_eq!(buffer_frames(5, 8000, &range), 64);
        assert_eq!(buffer_frames(500, 48000, &range), 4096);
        assert_eq!(buffer_frames(25, 44100, &cpal::SupportedBufferSize::Unknown), 1102);
    }
}
