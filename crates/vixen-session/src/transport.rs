//! Connects the audio player to the frame engine: while a sequence has music, the audio device's
//! position is the master clock, so the lights can't drift from the song.

use std::sync::Arc;
use vixen_audio::AudioPlayer;
use vixen_core::TimeSpan;
use vixen_engine::TimeSource;

/// `TimeSource` over the audio player.
pub struct AudioTimeSource(pub Arc<AudioPlayer>);

impl TimeSource for AudioTimeSource {
    fn position(&self) -> TimeSpan {
        self.0.position()
    }

    fn play(&self) {
        self.0.play();
    }

    fn pause(&self) {
        self.0.pause();
    }

    fn seek(&self, to: TimeSpan) {
        self.0.seek(to);
    }

    fn finished(&self) -> bool {
        self.0.has_ended() || matches!(self.0.state(), vixen_audio::PlayerState::Error(_))
    }
}
