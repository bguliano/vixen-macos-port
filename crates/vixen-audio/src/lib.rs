//! Audio layer: decode, waveform peaks, and the audio-clock player.
//!
//! The audio position is the show's master clock. See [`AudioPlayer::position`].

mod backend;
mod clip;
mod cpal_backend;
mod error;
mod peaks;
mod player;
mod wsola;

pub use backend::{
    Backend, Clock, ErrorFn, FakeBackend, ManualClock, RenderFactory, RenderFn, Stream, StreamFormat,
    StreamRequest, SystemClock,
};
pub use clip::{frames_to_time, time_to_frames, AudioClip, MonoView};
pub use cpal_backend::{output_devices, CpalBackend};
pub use error::AudioError;
pub use peaks::Peaks;
pub use player::{AudioPlayer, ClockHandle, PlayerState};
pub use wsola::{time_stretch, Src, Wsola};
