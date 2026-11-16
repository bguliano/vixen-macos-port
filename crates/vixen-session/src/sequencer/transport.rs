//! The window's transport rules, F262 to F280 and F423: what Play, Stop, Loop, the delay options and the speed controls do.
//! `TimedSequenceEditorForm`'s playback code without the engine: each call returns what the engine should do, and
//! [`crate::sequencer::SequencerState`] does it.

use crate::playback::Transport;
use std::time::{Duration, Instant};
use vixen_core::TimeSpan;

/// `_timingChangeDelta`.
const SPEED_DELTA: f32 = 0.1;
/// `timerDelayCountdown`'s interval.
const COUNTDOWN_STEP: Duration = Duration::from_secs(1);

/// Sequence > Playback > Play Options.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum DelayOption {
    #[default]
    Off,
    Seconds5,
    Seconds10,
    Seconds20,
    Seconds30,
    Seconds60,
}

impl DelayOption {
    pub const ALL: [DelayOption; 6] = [
        DelayOption::Off,
        DelayOption::Seconds5,
        DelayOption::Seconds10,
        DelayOption::Seconds20,
        DelayOption::Seconds30,
        DelayOption::Seconds60,
    ];

    /// `timerPostponePlay.Interval`.
    pub fn interval_ms(self) -> i32 {
        match self {
            DelayOption::Off => 100,
            DelayOption::Seconds5 => 5000,
            DelayOption::Seconds10 => 10000,
            DelayOption::Seconds20 => 20000,
            DelayOption::Seconds30 => 30000,
            DelayOption::Seconds60 => 60000,
        }
    }
}

/// `TimelineControl.PlaybackStartTime` and `PlaybackEndTime`, or the remembered `_mPrevPlaybackStart` and `_mPrevPlaybackEnd`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct PlaybackRange {
    pub start: Option<TimeSpan>,
    pub end: Option<TimeSpan>,
}

/// What the engine should do after a transport call.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EngineCommand {
    None,
    /// `_context.Play` or `PlayLoop`.
    Play { start: TimeSpan, end: TimeSpan, looping: bool },
    Resume,
    Pause,
    Stop,
}

/// What `tick` found.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct TransportEvents {
    pub command: EngineCommand,
    /// `context_ContextEnded`: the live range goes back to this and the playback time clears.
    pub restore_range: Option<PlaybackRange>,
    /// `context_SequenceStarted`.
    pub started: bool,
}

/// The Play button's picture. F264, F268.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PlayIcon {
    Play,
    Pause,
    Hourglass,
}

/// What the Sequence > Playback items, the PlayBack strip and the status bar's delay label show. One read per change.
#[derive(Clone, PartialEq, Debug)]
pub struct TransportUi {
    pub play_enabled: bool,
    pub play_menu_enabled: bool,
    /// Whether F5 is on the Play item: `ShortcutKeys`.
    pub play_has_shortcut: bool,
    pub stop_enabled: bool,
    /// Always off, as in the original: pausing is done by Play. F28.
    pub pause_menu_enabled: bool,
    pub icon: PlayIcon,
    pub play_title: &'static str,
    pub play_tooltip: &'static str,
    pub delay_visible: bool,
    pub delay_text: String,
    pub delay: DelayOption,
    pub looping: bool,
    pub running: bool,
    pub paused: bool,
}

/// `_SetTimingSpeed`'s result.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SpeedControls {
    /// `_timingSpeed`.
    pub speed: f32,
    /// What the gauge shows: not updated when `_SetTimingSpeed` returns early.
    pub display_speed: f32,
    pub decrease_enabled: bool,
    pub increase_enabled: bool,
    pub gauge_enabled: bool,
}

struct Countdown {
    started: Instant,
    interval: Duration,
    steps_done: u32,
}

/// One per window. Drives `AppCore.playback` through the commands it returns, and holds speed, loop, delay and the remembered
/// range.
pub struct TransportController {
    has_context: bool,
    looping: bool,
    delay: DelayOption,
    /// `timerPostponePlay.Interval`. Loop turns the delay off without resetting this, which does no harm with Delay Off checked.
    postpone_ms: i32,
    delay_visible: bool,
    /// `_delayCountDown`, or the label's fixed text outside a countdown.
    delay_seconds: i32,
    countdown: Option<Countdown>,
    remembered: PlaybackRange,

    speed: f32,
    display_speed: f32,
    decrease_enabled: bool,
    increase_enabled: bool,
    gauge_enabled: bool,

    /// The engine's state at the last look, to catch a running engine stopping. The context gives events; this polls `tick`.
    last_engine: Transport,
}

impl Default for TransportController {
    fn default() -> Self {
        TransportController::new()
    }
}

impl TransportController {
    pub fn new() -> TransportController {
        TransportController {
            has_context: true,
            looping: false,
            delay: DelayOption::Off,
            postpone_ms: DelayOption::Off.interval_ms(),
            delay_visible: false,
            delay_seconds: 0,
            countdown: None,
            remembered: PlaybackRange::default(),
            speed: 1.0,
            display_speed: 1.0,
            decrease_enabled: false,
            increase_enabled: false,
            gauge_enabled: false,
            last_engine: Transport::Stopped,
        }
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Context, F262
    // ------------------------------------------------------------------------------------------------------------------

    /// `OnExecutionStateChanged`: live execution opened or closed. Without a context nothing plays.
    pub fn set_has_context(&mut self, has_context: bool) {
        self.has_context = has_context;
    }

    pub fn has_context(&self) -> bool {
        self.has_context
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Play, pause, stop, F263 to F266
    // ------------------------------------------------------------------------------------------------------------------

    fn stop_button_enabled(&self, engine: Transport) -> bool {
        self.countdown.is_some() || engine != Transport::Stopped
    }

    /// `PlayPauseToggle`. Upstream throws with no context, here it does nothing (section D's GUARD).
    pub fn play_pause_toggle(&mut self, engine: Transport, live: PlaybackRange, now: Instant) -> EngineCommand {
        if !self.has_context {
            return EngineCommand::None;
        }
        if engine != Transport::Playing {
            self.play_sequence(engine, live, now)
        } else {
            EngineCommand::Pause
        }
    }

    /// `HandleSpacebarAction`: `pause` for ⌥Space, otherwise a running sequence stops.
    pub fn spacebar(&mut self, pause: bool, engine: Transport, live: PlaybackRange, now: Instant) -> EngineCommand {
        if !self.has_context {
            return EngineCommand::None;
        }
        match engine {
            Transport::Stopped | Transport::Paused => self.play_sequence(engine, live, now),
            Transport::Playing if pause => EngineCommand::Pause,
            Transport::Playing => self.stop(engine),
        }
    }

    /// `PlaySequence`.
    pub fn play_sequence(&mut self, engine: Transport, live: PlaybackRange, now: Instant) -> EngineCommand {
        if self.delay != DelayOption::Off && self.countdown.is_none() && !self.stop_button_enabled(engine) {
            self.delay_seconds = self.postpone_ms / 1000;
            self.countdown = Some(Countdown { started: now, interval: Duration::from_millis(self.postpone_ms as u64), steps_done: 0 });
        }
        if self.countdown.is_some() {
            // Waiting for a delayed start: the button does nothing.
            return EngineCommand::None;
        }
        self.begin_playing(engine, live)
    }

    /// The part of `PlaySequence` past the delay check.
    fn begin_playing(&mut self, engine: Transport, live: PlaybackRange) -> EngineCommand {
        self.delay_visible = false;
        if !self.has_context {
            return EngineCommand::None;
        }
        if engine == Transport::Paused {
            // `_PlaySequence` resumes and ignores the times, so the original range still holds.
            return EngineCommand::Resume;
        }
        EngineCommand::Play {
            start: live.start.unwrap_or(TimeSpan::ZERO),
            end: live.end.unwrap_or(TimeSpan::MAX),
            looping: self.looping,
        }
    }

    /// `PlaySequenceBetween`: how the marks bar plays a range. Doesn't touch the range or the remembered range. Paused: stop first,
    /// which the caller does before starting the returned play.
    pub fn play_between(&mut self, start: TimeSpan, end: TimeSpan, total: TimeSpan, live: PlaybackRange) -> EngineCommand {
        if !self.has_context {
            return EngineCommand::None;
        }
        let start = if start < TimeSpan::ZERO { live.start.unwrap_or(TimeSpan::ZERO) } else { start };
        let end = if end > total { live.end.unwrap_or(TimeSpan::MAX) } else { end };
        EngineCommand::Play { start, end, looping: self.looping }
    }

    /// `StopSequence`. A cancelled countdown leaves the Play menu item enabled here, where upstream leaves it disabled (F266's FIX).
    pub fn stop(&mut self, engine: Transport) -> EngineCommand {
        if self.delay != DelayOption::Off {
            self.delay_visible = true;
            self.delay_seconds = self.postpone_ms / 1000;
        }
        if self.countdown.is_some() {
            self.countdown = None;
            return EngineCommand::None;
        }
        if !self.has_context || engine == Transport::Stopped {
            return EngineCommand::None;
        }
        EngineCommand::Stop
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Loop and delay options, F30 to F36
    // ------------------------------------------------------------------------------------------------------------------

    /// `toolStripMenuItem_Loop_CheckedChanged`: no looping with a delay.
    pub fn set_loop(&mut self, on: bool) {
        self.looping = on;
        if on && self.delay != DelayOption::Off {
            self.delay = DelayOption::Off;
            self.delay_visible = false;
        }
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    /// The Play Options items. Any of them, Delay Off included, turns Loop off: `ClearDelayPlayItemChecks` (KEEP).
    pub fn set_delay(&mut self, delay: DelayOption) {
        self.postpone_ms = delay.interval_ms();
        self.looping = false;
        self.delay = delay;
        if delay == DelayOption::Off {
            self.delay_visible = false;
        } else {
            self.delay_visible = true;
            self.delay_seconds = delay.interval_ms() / 1000;
        }
    }

    pub fn delay(&self) -> DelayOption {
        self.delay
    }

    /// Whether anything is waiting on a clock, so the window knows to keep calling `tick`.
    pub fn is_counting_down(&self) -> bool {
        self.countdown.is_some()
    }

    /// A stopped engine still needs one tick when its end transition has not yet
    /// restored the range/cleared the position. It can stop between a tick's read
    /// and the caller deciding whether to schedule the next tick.
    pub fn needs_tick(&self, engine: Transport) -> bool {
        self.countdown.is_some() || engine != Transport::Stopped || self.last_engine != Transport::Stopped
    }

    /// F267, F270. Called from the display link and the countdown's timer, only while something's active.
    pub fn tick(&mut self, engine: Transport, live: PlaybackRange, now: Instant) -> TransportEvents {
        let mut events = TransportEvents { command: EngineCommand::None, restore_range: None, started: false };
        if let Some(countdown) = &mut self.countdown {
            let elapsed = now.saturating_duration_since(countdown.started);
            let steps = (elapsed.as_millis() / COUNTDOWN_STEP.as_millis()) as u32;
            // `timerDelayCountdown_Tick`, once per second.
            while countdown.steps_done < steps {
                countdown.steps_done += 1;
                self.delay_seconds -= 1;
            }
            if elapsed >= countdown.interval {
                // `timerPostponePlay_Tick`: Stop is still enabled, so `PlaySequence` goes straight through to playing.
                self.countdown = None;
                events.command = self.begin_playing(engine, live);
            }
        }
        if engine != self.last_engine {
            if self.last_engine == Transport::Stopped {
                events.started = true;
            }
            if engine == Transport::Stopped {
                events.restore_range = Some(self.context_ended());
            }
            self.last_engine = engine;
        }
        events
    }

    /// Tells the controller what the engine just did, so a started play is not taken for a stop by the next tick.
    pub fn note_engine(&mut self, engine: Transport) -> Option<PlaybackRange> {
        if engine == self.last_engine {
            return None;
        }
        let was = self.last_engine;
        self.last_engine = engine;
        if engine == Transport::Stopped && was != Transport::Stopped {
            return Some(self.context_ended());
        }
        None
    }

    /// `context_SequenceEnded` and `context_ContextEnded`.
    fn context_ended(&mut self) -> PlaybackRange {
        if self.delay != DelayOption::Off {
            self.delay_visible = true;
            self.delay_seconds = self.postpone_ms / 1000;
        }
        self.remembered
    }

    /// `UpdateButtonStates`, `UpdatePlayButton` and the delay state, as one value.
    pub fn transport_ui(&self, engine: Transport) -> TransportUi {
        let counting = self.countdown.is_some();
        let running = engine != Transport::Stopped;
        let paused = engine == Transport::Paused;
        let show_pause = engine == Transport::Playing;
        let icon = if counting {
            PlayIcon::Hourglass
        } else if show_pause {
            PlayIcon::Pause
        } else {
            PlayIcon::Play
        };
        TransportUi {
            play_enabled: self.has_context,
            // F264: disabled through a countdown. Upstream forgets to enable it again after a cancelled one; here it follows.
            play_menu_enabled: self.has_context && !counting,
            play_has_shortcut: self.has_context,
            stop_enabled: self.has_context && (counting || running),
            pause_menu_enabled: false,
            icon,
            play_title: if show_pause && !counting { "Pause" } else { "Play" },
            play_tooltip: if show_pause && !counting { "Pause F5" } else { "Play F5" },
            delay_visible: self.delay_visible,
            delay_text: format!("{} Seconds", self.delay_seconds),
            delay: self.delay,
            looping: self.looping,
            running,
            paused,
        }
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Remembered range, F105, F108, F273, F274, F423
    // ------------------------------------------------------------------------------------------------------------------

    pub fn remembered(&self) -> PlaybackRange {
        self.remembered
    }

    /// Load (F1), the Start and End buttons, and a kept ruler range set the remembered range.
    pub fn set_remembered(&mut self, range: PlaybackRange) {
        self.remembered = range;
    }

    /// `timelineControl_RulerClicked`, left button. Returns the new live range and what to do; the caller marks the sequence
    /// modified (a plain click does, KEEP F273) and moves the cursor unless this played.
    pub fn ruler_clicked(&mut self, time: TimeSpan, cmd_held: bool, live: PlaybackRange) -> RulerClick {
        if !self.has_context {
            return RulerClick { live, command: EngineCommand::None, move_cursor: false, modified: false };
        }
        if cmd_held {
            self.remembered = live;
        } else {
            self.remembered = PlaybackRange { start: Some(time), end: None };
        }
        let live = PlaybackRange { start: Some(time), end: None };
        let command = if cmd_held {
            EngineCommand::Play { start: time, end: TimeSpan::MAX, looping: self.looping }
        } else {
            EngineCommand::None
        };
        RulerClick { live, command, move_cursor: !cmd_held, modified: true }
    }

    /// `timelineControl_RulerBeginDragTimeRange`.
    pub fn ruler_range_begin(&mut self, live: PlaybackRange) {
        self.remembered = live;
    }

    /// `timelineControl_TimeRangeDragged`. Not marked modified here (F423).
    pub fn ruler_range_end(&mut self, cmd_held: bool, live: PlaybackRange) -> EngineCommand {
        if !self.has_context {
            return EngineCommand::None;
        }
        if cmd_held {
            match (live.start, live.end) {
                (Some(start), Some(end)) => EngineCommand::Play { start, end, looping: self.looping },
                _ => EngineCommand::None,
            }
        } else {
            self.remembered = live;
            EngineCommand::None
        }
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Speed, F129 to F133, F271, F272
    // ------------------------------------------------------------------------------------------------------------------

    /// `_SetTimingSpeed`. Returns the speed to give the timing source, when there's anything to give it.
    ///
    /// `speed` and the delta are `float` and `0.10` is a `double`, so exactly `0.1f` widens to 0.10000000149 and passes the first
    /// test (KEEP F271): the +/- buttons step without rounding, so from 1.0 they stop at a shown 20% and 400% with their enabled state
    /// stale, and only the wheel reaches 10%.
    pub fn set_speed(&mut self, speed: f32) -> Option<f32> {
        if (speed as f64) <= 0.10 {
            self.speed = 0.10f32;
            return None;
        }
        if speed > 4.0f32 {
            self.speed = 4.0f32;
            return None;
        }
        self.speed = speed;
        self.display_speed = speed;
        self.decrease_enabled = self.speed > SPEED_DELTA;
        self.increase_enabled = self.speed < 4.0f32;
        Some(speed)
    }

    /// `audioToolStripLabel_TimingSpeed_Click`.
    pub fn speed_reset(&mut self) -> Option<f32> {
        self.set_speed(1.0)
    }

    /// `audioToolStripLabel_TimingSpeed_DoubleClick`.
    pub fn speed_max(&mut self) -> Option<f32> {
        self.set_speed(4.0)
    }

    /// `toolStripButton_IncreaseTimingSpeed_Click`.
    pub fn speed_increase(&mut self) -> Option<f32> {
        self.set_speed(self.speed + SPEED_DELTA)
    }

    /// `toolStripButton_DecreaseTimingSpeed_Click`.
    pub fn speed_decrease(&mut self) -> Option<f32> {
        self.set_speed(self.speed - SPEED_DELTA)
    }

    /// `toolStripAudio_MouseWheel`. The wheel is not gated by the gauge's enabled state (KEEP F130).
    pub fn speed_wheel(&mut self, up: bool) -> Option<f32> {
        self.set_speed(speed_wheel(self.speed, up))
    }

    /// `_SetTimingToolStripEnabledState`: the +/- buttons and the gauge work only with a timing source that takes variable speeds,
    /// which means audio.
    pub fn set_speed_controls_enabled(&mut self, enabled: bool) {
        self.decrease_enabled = enabled;
        self.increase_enabled = enabled;
        self.gauge_enabled = enabled;
    }

    /// The `TimingSource` setter, when playback starts. Returns the speed to give a source that takes one.
    pub fn timing_source_started(&mut self, supports_variable_speeds: bool) -> Option<f32> {
        if supports_variable_speeds {
            self.increase_enabled = true;
            self.gauge_enabled = true;
            self.decrease_enabled = self.speed > SPEED_DELTA;
            Some(self.speed)
        } else {
            self.display_speed = self.speed;
            self.decrease_enabled = false;
            self.increase_enabled = false;
            self.gauge_enabled = false;
            None
        }
    }

    pub fn speed_controls(&self) -> SpeedControls {
        SpeedControls {
            speed: self.speed,
            display_speed: self.display_speed,
            decrease_enabled: self.decrease_enabled,
            increase_enabled: self.increase_enabled,
            gauge_enabled: self.gauge_enabled,
        }
    }
}

/// The result of a left click on the ruler.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RulerClick {
    pub live: PlaybackRange,
    pub command: EngineCommand,
    pub move_cursor: bool,
    pub modified: bool,
}

/// `Math.Round(_timingSpeed, 1)` then a step, in `float` where the C# is. `Math.Round` on a float widens it first.
pub fn speed_wheel(current: f32, up: bool) -> f32 {
    let rounded = ((current as f64) * 10.0).round_ties_even() / 10.0;
    if up {
        rounded as f32 + SPEED_DELTA
    } else {
        rounded as f32 - SPEED_DELTA
    }
}

/// F276: `Play` clamps both ends to the sequence length.
pub fn coerce_range(start: TimeSpan, end: TimeSpan, length: TimeSpan) -> (TimeSpan, TimeSpan) {
    (start.min(length), end.min(length))
}

/// F277: `position >= end || position == 0`, but only after position has once passed start. With end before start it never ends.
pub fn is_natural_end(position: TimeSpan, start: TimeSpan, end: TimeSpan, advanced: &mut bool) -> bool {
    if end < start {
        return false;
    }
    if position > start {
        *advanced = true;
    }
    *advanced && (position >= end || position == TimeSpan::ZERO)
}

/// What the speed gauge draws (`SpeedVisualisation`), F133. Swift draws it in a 32 by 32 box.
#[derive(Clone, PartialEq, Debug)]
pub struct SpeedGauge {
    /// `(Math.Round(speed, 1) * 100).ToString("F0")`.
    pub text: String,
    /// Left of the text: 3 at 97% and over, where the text is wider.
    pub text_x: i32,
    /// `(byte)` hue, as ZedGraph's `HSBColor` holds it.
    pub hue: u8,
    /// The hue as RGB, at full saturation and brightness.
    pub rgb: (u8, u8, u8),
    /// Needle angle in degrees.
    pub needle_degrees: i32,
    /// Where the needle ends. It starts at (16, 32).
    pub needle_end: (i32, i32),
}

pub fn speed_gauge(speed: f32) -> SpeedGauge {
    let hue = if speed >= 1.0 {
        120 - (7.5f32 * speed * 4.0f32) as i32
    } else {
        (2.5f32 * speed * 60.0f32) as i32 - 20
    };
    let hue = hue as u8;
    let needle_degrees = 350 - (160 - (10.0f32 * speed * 4.0f32) as i32);
    let angle = std::f64::consts::PI / 180.0 * needle_degrees as f64;
    let needle_end = ((16.0 + angle.cos() * 19.0) as i32, (32.0 + angle.sin() * 19.0) as i32);
    let rounded = ((speed as f64) * 10.0).round_ties_even() / 10.0;
    SpeedGauge {
        text: format!("{:.0}", rounded * 100.0),
        text_x: if speed >= 0.97f32 { 3 } else { 6 },
        hue,
        rgb: hsb_to_rgb(hue, 255, 255),
        needle_degrees,
        needle_end,
    }
}

/// `HSBColor.ToRGB`.
pub fn hsb_to_rgb(h: u8, s: u8, b: u8) -> (u8, u8, u8) {
    let sector = (h as f64 / 42.5).floor() as i32;
    let fraction = h as f64 / 42.5 - sector as f64;
    let s_frac = s as f64 / 255.0;
    let v = b as f64;
    let p = (v * (1.0 - s_frac) + 0.5) as u8;
    let q = (v * (1.0 - s_frac * fraction) + 0.5) as u8;
    let t = (v * (1.0 - s_frac * (1.0 - fraction)) + 0.5) as u8;
    match sector {
        0 => (b, t, p),
        1 => (q, b, p),
        2 => (p, b, t),
        3 => (p, q, b),
        4 => (t, p, b),
        _ => (b, p, q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: i64) -> TimeSpan {
        TimeSpan::from_ms_i(s * 1000)
    }

    fn live() -> PlaybackRange {
        PlaybackRange::default()
    }

    #[test]
    fn enable_table_matches_update_button_states() {
        let mut t = TransportController::new();
        let ui = t.transport_ui(Transport::Stopped);
        assert!(ui.play_enabled && ui.play_menu_enabled && ui.play_has_shortcut);
        assert!(!ui.stop_enabled && !ui.pause_menu_enabled);
        assert_eq!((ui.icon, ui.play_title, ui.play_tooltip), (PlayIcon::Play, "Play", "Play F5"));

        let ui = t.transport_ui(Transport::Playing);
        assert!(ui.play_enabled && ui.stop_enabled && !ui.pause_menu_enabled);
        assert_eq!((ui.icon, ui.play_title, ui.play_tooltip), (PlayIcon::Pause, "Pause", "Pause F5"));

        let ui = t.transport_ui(Transport::Paused);
        assert!(ui.play_enabled && ui.stop_enabled);
        assert_eq!((ui.icon, ui.play_tooltip), (PlayIcon::Play, "Play F5"));

        t.set_has_context(false);
        let ui = t.transport_ui(Transport::Stopped);
        assert!(!ui.play_enabled && !ui.play_menu_enabled && !ui.play_has_shortcut && !ui.stop_enabled);
    }

    #[test]
    fn play_uses_the_live_range_and_pause_resumes() {
        let mut t = TransportController::new();
        let now = Instant::now();
        let range = PlaybackRange { start: Some(secs(3)), end: None };
        assert_eq!(
            t.play_pause_toggle(Transport::Stopped, range, now),
            EngineCommand::Play { start: secs(3), end: TimeSpan::MAX, looping: false }
        );
        assert_eq!(t.play_pause_toggle(Transport::Playing, range, now), EngineCommand::Pause);
        assert_eq!(t.play_pause_toggle(Transport::Paused, range, now), EngineCommand::Resume);
    }

    #[test]
    fn spacebar_stops_and_option_space_pauses() {
        let mut t = TransportController::new();
        let now = Instant::now();
        assert!(matches!(t.spacebar(false, Transport::Stopped, live(), now), EngineCommand::Play { .. }));
        assert_eq!(t.spacebar(false, Transport::Playing, live(), now), EngineCommand::Stop);
        assert_eq!(t.spacebar(true, Transport::Playing, live(), now), EngineCommand::Pause);
        assert_eq!(t.spacebar(false, Transport::Paused, live(), now), EngineCommand::Resume);
    }

    #[test]
    fn no_context_does_nothing() {
        let mut t = TransportController::new();
        t.set_has_context(false);
        let now = Instant::now();
        assert_eq!(t.play_pause_toggle(Transport::Stopped, live(), now), EngineCommand::None);
        assert_eq!(t.spacebar(true, Transport::Stopped, live(), now), EngineCommand::None);
        assert_eq!(t.stop(Transport::Stopped), EngineCommand::None);
    }

    #[test]
    fn delay_counts_down_then_plays() {
        let mut t = TransportController::new();
        t.set_delay(DelayOption::Seconds5);
        let ui = t.transport_ui(Transport::Stopped);
        assert!(ui.delay_visible);
        assert_eq!(ui.delay_text, "5 Seconds");

        let start = Instant::now();
        assert_eq!(t.play_pause_toggle(Transport::Stopped, live(), start), EngineCommand::None);
        let ui = t.transport_ui(Transport::Stopped);
        assert_eq!(ui.icon, PlayIcon::Hourglass);
        assert!(ui.play_enabled && !ui.play_menu_enabled && ui.stop_enabled);

        // A second Play is ignored.
        assert_eq!(t.play_pause_toggle(Transport::Stopped, live(), start + Duration::from_millis(500)), EngineCommand::None);

        let mut seen = Vec::new();
        for i in 1..=4 {
            let e = t.tick(Transport::Stopped, live(), start + Duration::from_secs(i));
            assert_eq!(e.command, EngineCommand::None);
            seen.push(t.transport_ui(Transport::Stopped).delay_text);
        }
        assert_eq!(seen, ["4 Seconds", "3 Seconds", "2 Seconds", "1 Seconds"]);
        let e = t.tick(Transport::Stopped, live(), start + Duration::from_secs(5));
        assert!(matches!(e.command, EngineCommand::Play { looping: false, .. }));
        // Hidden at the moment playback really starts.
        assert!(!t.transport_ui(Transport::Playing).delay_visible);

        // Shown again at the end, at its full value.
        t.note_engine(Transport::Playing);
        let restored = t.note_engine(Transport::Stopped);
        assert!(restored.is_some());
        let ui = t.transport_ui(Transport::Stopped);
        assert!(ui.delay_visible);
        assert_eq!(ui.delay_text, "5 Seconds");
    }

    #[test]
    fn stop_during_countdown_cancels_and_leaves_play_enabled() {
        let mut t = TransportController::new();
        t.set_delay(DelayOption::Seconds10);
        let start = Instant::now();
        t.play_pause_toggle(Transport::Stopped, live(), start);
        t.tick(Transport::Stopped, live(), start + Duration::from_secs(3));
        assert_eq!(t.transport_ui(Transport::Stopped).delay_text, "7 Seconds");
        assert_eq!(t.stop(Transport::Stopped), EngineCommand::None);
        let ui = t.transport_ui(Transport::Stopped);
        assert_eq!(ui.icon, PlayIcon::Play);
        assert!(!ui.stop_enabled);
        // Upstream leaves this disabled. Listed as a FIX for the owner's veto.
        assert!(ui.play_menu_enabled);
        assert_eq!(ui.delay_text, "10 Seconds");
        assert!(ui.delay_visible);
    }

    #[test]
    fn delay_and_loop_exclude_each_other() {
        let mut t = TransportController::new();
        t.set_delay(DelayOption::Seconds20);
        t.set_loop(true);
        assert_eq!(t.delay(), DelayOption::Off);
        assert!(!t.transport_ui(Transport::Stopped).delay_visible);
        t.set_delay(DelayOption::Seconds5);
        assert!(!t.looping());
        // Any Play Options choice turns Loop off, Delay Off included (KEEP F31).
        t.set_loop(true);
        t.set_delay(DelayOption::Off);
        assert!(!t.looping());
    }

    #[test]
    fn loop_turned_on_plays_looping() {
        let mut t = TransportController::new();
        t.set_loop(true);
        assert!(matches!(
            t.play_pause_toggle(Transport::Stopped, live(), Instant::now()),
            EngineCommand::Play { looping: true, .. }
        ));
    }

    #[test]
    fn plain_ruler_click_keeps_the_range_and_marks_modified() {
        let mut t = TransportController::new();
        let click = t.ruler_clicked(secs(5), false, PlaybackRange { start: Some(secs(1)), end: Some(secs(2)) });
        assert_eq!(click.live, PlaybackRange { start: Some(secs(5)), end: None });
        assert_eq!(click.command, EngineCommand::None);
        assert!(click.move_cursor && click.modified);
        assert_eq!(t.remembered(), click.live);
    }

    #[test]
    fn command_ruler_click_plays_and_restores_the_range_at_the_end() {
        let mut t = TransportController::new();
        let before = PlaybackRange { start: Some(secs(1)), end: Some(secs(2)) };
        let click = t.ruler_clicked(secs(5), true, before);
        assert_eq!(click.command, EngineCommand::Play { start: secs(5), end: TimeSpan::MAX, looping: false });
        assert!(!click.move_cursor);
        // F423: still marked modified, and the temporary range stays live until the end.
        assert!(click.modified);
        assert_eq!(click.live, PlaybackRange { start: Some(secs(5)), end: None });
        t.note_engine(Transport::Playing);
        assert_eq!(t.note_engine(Transport::Stopped), Some(before));
    }

    #[test]
    fn ruler_drag_keeps_or_plays_the_range() {
        let mut t = TransportController::new();
        let before = PlaybackRange { start: Some(secs(1)), end: None };
        let dragged = PlaybackRange { start: Some(secs(3)), end: Some(secs(4)) };
        t.ruler_range_begin(before);
        assert_eq!(t.ruler_range_end(false, dragged), EngineCommand::None);
        assert_eq!(t.remembered(), dragged);

        t.ruler_range_begin(before);
        assert_eq!(t.ruler_range_end(true, dragged), EngineCommand::Play { start: secs(3), end: secs(4), looping: false });
        assert_eq!(t.remembered(), before);
    }

    #[test]
    fn play_between_falls_back_to_the_range() {
        let mut t = TransportController::new();
        let range = PlaybackRange { start: Some(secs(2)), end: Some(secs(9)) };
        assert_eq!(
            t.play_between(TimeSpan(-1), secs(100), secs(50), range),
            EngineCommand::Play { start: secs(2), end: secs(9), looping: false }
        );
        assert_eq!(
            t.play_between(secs(1), secs(3), secs(50), range),
            EngineCommand::Play { start: secs(1), end: secs(3), looping: false }
        );
    }

    #[test]
    fn speed_early_returns_leave_display_and_source_alone() {
        let mut t = TransportController::new();
        // Widens to 0.10000000149, so it goes through.
        assert_eq!(t.set_speed(0.1), Some(0.1));
        assert_eq!(t.speed_controls().display_speed, 0.1);
        assert!(!t.speed_controls().decrease_enabled);

        assert_eq!(t.set_speed(1.0), Some(1.0));
        assert_eq!(t.set_speed(0.09999993), None);
        let c = t.speed_controls();
        assert_eq!(c.speed, 0.1);
        assert_eq!(c.display_speed, 1.0);

        assert_eq!(t.set_speed(0.5), Some(0.5));
        assert_eq!(t.set_speed(4.0), Some(4.0));
        assert!(!t.speed_controls().increase_enabled);
        assert_eq!(t.set_speed(4.1), None);
        assert_eq!(t.speed_controls().speed, 4.0);
    }

    #[test]
    fn stepping_down_without_rounding_stops_at_a_shown_twenty_percent() {
        let mut t = TransportController::new();
        t.set_speed(1.0);
        for _ in 0..8 {
            assert!(t.speed_decrease().is_some());
        }
        assert_eq!(t.speed_controls().speed, 0.19999993);
        assert_eq!(speed_gauge(t.speed_controls().display_speed).text, "20");
        // The ninth stores 0.1, the gauge stays and Decrease stays enabled.
        assert!(t.speed_decrease().is_none());
        let c = t.speed_controls();
        assert_eq!(c.speed, 0.1);
        assert_eq!(speed_gauge(c.display_speed).text, "20");
        assert!(c.decrease_enabled);
    }

    #[test]
    fn stepping_up_without_rounding_stops_at_a_shown_four_hundred() {
        let mut t = TransportController::new();
        t.set_speed(1.0);
        for _ in 0..30 {
            assert!(t.speed_increase().is_some());
        }
        assert_eq!(t.speed_controls().speed, 3.9999983);
        assert!(t.speed_increase().is_none());
        let c = t.speed_controls();
        assert_eq!(c.speed, 4.0);
        assert_eq!(speed_gauge(c.display_speed).text, "400");
        assert!(c.increase_enabled);
    }

    #[test]
    fn the_wheel_rounds_first_so_it_lands_on_the_ends() {
        let mut t = TransportController::new();
        t.set_speed(1.0);
        for _ in 0..9 {
            assert!(t.speed_wheel(false).is_some());
        }
        assert_eq!(t.speed_controls().speed, 0.1);
        assert!(t.speed_wheel(false).is_none());
        assert_eq!(t.speed_controls().speed, 0.1);

        t.set_speed(1.0);
        for _ in 0..30 {
            assert!(t.speed_wheel(true).is_some());
        }
        assert_eq!(t.speed_controls().speed, 4.0);
        assert!(t.speed_wheel(true).is_none());
    }

    #[test]
    fn speed_controls_follow_the_timing_source() {
        let mut t = TransportController::new();
        assert!(!t.speed_controls().gauge_enabled);
        t.set_speed_controls_enabled(true);
        assert!(t.speed_controls().gauge_enabled && t.speed_controls().increase_enabled);
        assert_eq!(t.timing_source_started(false), None);
        assert!(!t.speed_controls().gauge_enabled);
        t.set_speed(0.5);
        assert_eq!(t.timing_source_started(true), Some(0.5));
        assert!(t.speed_controls().gauge_enabled);
    }

    #[test]
    fn gauge_matches_the_spec_table() {
        let g = speed_gauge(1.0);
        assert_eq!((g.text.as_str(), g.text_x, g.needle_degrees, g.needle_end, g.hue), ("100", 3, 230, (3, 17), 90));
        let g = speed_gauge(4.0);
        assert_eq!((g.text.as_str(), g.needle_degrees, g.hue), ("400", 350, 0));
        let g = speed_gauge(0.5);
        assert_eq!((g.text.as_str(), g.text_x, g.needle_degrees, g.hue), ("50", 6, 210, 55));
        let g = speed_gauge(0.1);
        assert_eq!((g.text.as_str(), g.needle_degrees, g.hue), ("10", 194, 251));
    }

    #[test]
    fn an_unobserved_end_keeps_the_final_tick_required() {
        let mut t = TransportController::new();
        let remembered = PlaybackRange { start: Some(secs(2)), end: Some(secs(5)) };
        t.set_remembered(remembered);
        assert!(!t.needs_tick(Transport::Stopped));
        t.note_engine(Transport::Playing);
        // The frame worker stops after the UI's last read, before its active check.
        assert!(t.needs_tick(Transport::Stopped));
        let event = t.tick(Transport::Stopped, PlaybackRange::default(), Instant::now());
        assert_eq!(event.restore_range, Some(remembered));
        assert!(!t.needs_tick(Transport::Stopped));
        assert_eq!(t.tick(Transport::Stopped, PlaybackRange::default(), Instant::now()).restore_range, None);
    }

    #[test]
    fn natural_end_waits_for_time_to_advance() {
        let mut advanced = false;
        let (start, end) = (secs(5), secs(10));
        assert!(!is_natural_end(secs(5), start, end, &mut advanced));
        // Position 0 before advancing isn't an end: the media hasn't started yet.
        assert!(!is_natural_end(TimeSpan::ZERO, start, end, &mut advanced));
        assert!(!is_natural_end(secs(6), start, end, &mut advanced));
        assert!(is_natural_end(TimeSpan::ZERO, start, end, &mut advanced));
        assert!(is_natural_end(secs(10), start, end, &mut advanced));
        let mut advanced = false;
        assert!(!is_natural_end(secs(20), secs(10), secs(5), &mut advanced));
    }

    #[test]
    fn range_clamps_to_length() {
        assert_eq!(coerce_range(secs(70), TimeSpan::MAX, secs(60)), (secs(60), secs(60)));
        assert_eq!(coerce_range(secs(1), secs(2), secs(60)), (secs(1), secs(2)));
    }
}
