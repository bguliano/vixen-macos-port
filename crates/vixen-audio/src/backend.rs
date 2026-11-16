//! The seam between player logic and the OS audio API.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::error::AudioError;

/// Monotonic nanosecond clock. The player reads it from the audio callback and from `position()`.
pub trait Clock: Send + Sync {
    fn now_ns(&self) -> u64;
}

/// `Instant`-based clock.
pub struct SystemClock {
    base: Instant,
}

impl SystemClock {
    pub fn new() -> SystemClock {
        SystemClock { base: Instant::now() }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    #[inline]
    fn now_ns(&self) -> u64 {
        self.base.elapsed().as_nanos() as u64
    }
}

/// What the player would like. The backend may give something else; the player adapts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamRequest {
    pub sample_rate: u32,
    pub channels: u16,
    /// The buffer length to ask for, the Options dialog's Latency. `None` leaves it to the device.
    pub latency_ms: Option<u32>,
}

/// What the device actually runs at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/// Fills an interleaved f32 output buffer. The `Duration` is the output latency of this buffer
/// (time from the callback until its first frame is audible), zero if unknown. Runs on the audio
/// thread: it must not allocate or lock.
pub type RenderFn = Box<dyn FnMut(&mut [f32], Duration) + Send>;
/// Builds the render function once the device format is known.
pub type RenderFactory = Box<dyn FnOnce(StreamFormat) -> RenderFn + Send>;
/// Reports an asynchronous stream failure (device unplugged, ...). Called off the audio thread.
pub type ErrorFn = Box<dyn Fn(AudioError) + Send + Sync>;

/// A running output stream. Dropping it closes the device and guarantees the render function is gone.
pub trait Stream: Send {
    fn play(&mut self) -> Result<(), AudioError>;
    fn pause(&mut self) -> Result<(), AudioError>;
}

pub trait Backend: Send {
    fn output_devices(&self) -> Vec<String>;
    fn clock(&self) -> Arc<dyn Clock>;
    /// Open `device` (None = system default), build the render function for the chosen format and
    /// start pulling it. Failures are returned, never panics.
    fn open(
        &mut self,
        device: Option<&str>,
        request: StreamRequest,
        factory: RenderFactory,
        on_error: ErrorFn,
    ) -> Result<(Box<dyn Stream>, StreamFormat), AudioError>;
}

// ---------------------------------------------------------------------------------------------
// Fake backend: no device, manual clock, callbacks driven by the test.
// ---------------------------------------------------------------------------------------------

/// A clock that only moves when told to.
#[derive(Default)]
pub struct ManualClock {
    ns: AtomicU64,
}

impl ManualClock {
    pub fn advance(&self, d: Duration) {
        self.ns.fetch_add(d.as_nanos() as u64, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_ns(&self) -> u64 {
        self.ns.load(Ordering::SeqCst)
    }
}

struct FakeInner {
    clock: Arc<ManualClock>,
    render: Mutex<Option<RenderFn>>,
    on_error: Mutex<Option<ErrorFn>>,
    format: Mutex<StreamFormat>,
    latency: Mutex<Duration>,
    fail_open: Mutex<Option<AudioError>>,
    devices: Mutex<Vec<String>>,
    opens: AtomicUsize,
}

/// A fake output device. Clone it before handing one copy to `AudioPlayer::with_backend`; keep the
/// other to drive the audio callback with [`FakeBackend::pump`].
#[derive(Clone)]
pub struct FakeBackend {
    inner: Arc<FakeInner>,
}

impl FakeBackend {
    pub fn new(sample_rate: u32, channels: u16) -> FakeBackend {
        FakeBackend {
            inner: Arc::new(FakeInner {
                clock: Arc::new(ManualClock::default()),
                render: Mutex::new(None),
                on_error: Mutex::new(None),
                format: Mutex::new(StreamFormat { sample_rate, channels }),
                latency: Mutex::new(Duration::ZERO),
                fail_open: Mutex::new(None),
                devices: Mutex::new(vec!["Fake Output".to_string()]),
                opens: AtomicUsize::new(0),
            }),
        }
    }

    pub fn format(&self) -> StreamFormat {
        *self.inner.format.lock()
    }
    pub fn set_latency(&self, d: Duration) {
        *self.inner.latency.lock() = d;
    }
    /// The next `open` fails with this error (once).
    pub fn fail_next_open(&self, e: AudioError) {
        *self.inner.fail_open.lock() = Some(e);
    }
    pub fn open_count(&self) -> usize {
        self.inner.opens.load(Ordering::SeqCst)
    }
    pub fn is_open(&self) -> bool {
        self.inner.render.lock().is_some()
    }
    pub fn advance_clock(&self, d: Duration) {
        self.inner.clock.advance(d);
    }
    pub fn manual_clock(&self) -> Arc<ManualClock> {
        self.inner.clock.clone()
    }

    /// Run one audio callback for `frames` frames at the current clock time. Returns the interleaved
    /// output (empty if no stream is open).
    pub fn pump(&self, frames: usize) -> Vec<f32> {
        let fmt = self.format();
        let mut buf = vec![0.0f32; frames * fmt.channels as usize];
        if !self.pump_into(&mut buf) {
            buf.clear();
        }
        buf
    }

    /// Like [`FakeBackend::pump`] into a caller buffer (no allocation). False if no stream is open.
    pub fn pump_into(&self, buf: &mut [f32]) -> bool {
        let lat = *self.inner.latency.lock();
        match self.inner.render.lock().as_mut() {
            Some(r) => {
                r(buf, lat);
                true
            }
            None => false,
        }
    }

    /// Simulate the device failing under a running stream.
    pub fn trigger_error(&self, e: AudioError) {
        if let Some(f) = self.inner.on_error.lock().as_ref() {
            f(e);
        }
    }
}

struct FakeStream {
    inner: Arc<FakeInner>,
}

impl Stream for FakeStream {
    fn play(&mut self) -> Result<(), AudioError> {
        Ok(())
    }
    fn pause(&mut self) -> Result<(), AudioError> {
        Ok(())
    }
}

impl Drop for FakeStream {
    fn drop(&mut self) {
        *self.inner.render.lock() = None;
        *self.inner.on_error.lock() = None;
    }
}

impl Backend for FakeBackend {
    fn output_devices(&self) -> Vec<String> {
        self.inner.devices.lock().clone()
    }
    fn clock(&self) -> Arc<dyn Clock> {
        self.inner.clock.clone()
    }
    fn open(
        &mut self,
        device: Option<&str>,
        _request: StreamRequest,
        factory: RenderFactory,
        on_error: ErrorFn,
    ) -> Result<(Box<dyn Stream>, StreamFormat), AudioError> {
        self.inner.opens.fetch_add(1, Ordering::SeqCst);
        if let Some(e) = self.inner.fail_open.lock().take() {
            return Err(e);
        }
        if let Some(name) = device {
            if !self.inner.devices.lock().iter().any(|d| d == name) {
                return Err(AudioError::NoDevice(name.to_string()));
            }
        }
        let fmt = self.format();
        *self.inner.render.lock() = Some(factory(fmt));
        *self.inner.on_error.lock() = Some(on_error);
        Ok((Box::new(FakeStream { inner: self.inner.clone() }), fmt))
    }
}
