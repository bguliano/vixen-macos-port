//! What an intelligent fixture shows on each frame: `MovingHeadIntentHandler`, `MovingHeadTimer` and the geometry numbers the
//! OpenGL form hands its moving heads. Nothing here draws. A host gives [`FixtureSet::frame`] each fixture element's states,
//! calls [`FixtureSet::tick`] from a clock of its own for the strobes, and reads [`FixtureSet::state`] to draw.

use crate::shape::{MovingHeadData, YesNo};
use std::cmp::Ordering;
use vixen_core::{Color, CombinedState, CommandValue, FixtureIndexType, FunctionIdentity, NamedCommand, RangeValue, StateValue};

/// One slot of a colour wheel function (`FixtureColorWheel`).
#[derive(Clone, Debug, PartialEq)]
pub struct WheelEntry {
    pub start_value: i32,
    pub use_curve: bool,
    pub color1: Color,
    pub color2: Color,
}

/// A colour wheel function of the fixture's profile, by the function's name, which is the tag its commands carry.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ColorWheel {
    pub function: String,
    pub entries: Vec<WheelEntry>,
}

/// Everything `PreviewMovingHead.CreateIntentHandler` copies into the handler.
#[derive(Clone, Debug, PartialEq)]
pub struct FixtureSetup {
    pub pan_start: i32,
    pub pan_stop: i32,
    pub tilt_start: i32,
    pub tilt_stop: i32,
    pub invert_pan: bool,
    pub invert_tilt: bool,
    pub strobe_rate_minimum: i32,
    pub strobe_rate_maximum: i32,
    pub max_pan_travel_time: f64,
    pub max_tilt_travel_time: f64,
    pub min_color_wheel_rotation_speed: f64,
    pub max_color_wheel_rotation_speed: f64,
    pub maximum_strobe_duration: i32,
    pub zoom_narrow_to_wide: bool,
    pub beam_length: i32,
    /// A Shutter filter under the element says colour opens the shutter.
    pub convert_color_into_shutter: bool,
    /// A Dimming filter under the element says a discrete colour's intensity dims the beam.
    pub convert_color_into_dimmer: bool,
    pub color_wheels: Vec<ColorWheel>,
    /// `VixenSystem.DefaultUpdateInterval`, in milliseconds.
    pub update_interval_ms: i32,
}

impl FixtureSetup {
    /// The shape's own settings. The two filter flags and the colour wheels come from the element, so the caller adds them.
    pub fn from_shape(d: &MovingHeadData, update_interval_ms: i32) -> FixtureSetup {
        FixtureSetup {
            pan_start: d.pan_start_position,
            pan_stop: d.pan_stop_position,
            tilt_start: d.tilt_start_position,
            tilt_stop: d.tilt_stop_position,
            invert_pan: d.invert_pan_direction == YesNo::Yes,
            invert_tilt: d.invert_tilt_direction == YesNo::Yes,
            strobe_rate_minimum: d.strobe_rate_minimum,
            strobe_rate_maximum: d.strobe_rate_maximum,
            max_pan_travel_time: d.max_pan_travel_time.value,
            max_tilt_travel_time: d.max_tilt_travel_time.value,
            min_color_wheel_rotation_speed: d.min_color_wheel_rotation_speed.value,
            max_color_wheel_rotation_speed: d.max_color_wheel_rotation_speed.value,
            maximum_strobe_duration: d.maximum_strobe_duration,
            zoom_narrow_to_wide: d.zoom_narrow_to_wide,
            beam_length: d.beam_length,
            convert_color_into_shutter: false,
            convert_color_into_dimmer: false,
            color_wheels: Vec::new(),
            update_interval_ms,
        }
    }
}

/// `IMovingHead`: what the drawing reads.
#[derive(Clone, Debug, PartialEq)]
pub struct FixtureState {
    /// Degrees, kept within -360...360.
    pub pan_angle: f64,
    pub tilt_angle: f64,
    /// A percentage of the longest beam that fits the window.
    pub beam_length: f64,
    pub on_off: bool,
    pub beam_color_left: Color,
    pub beam_color_right: Color,
    /// 0...100.
    pub intensity: i32,
    /// 0...100, the beam's spread.
    pub focus: i32,
    pub legend: String,
    /// Milliseconds between flashes.
    pub strobe_rate: i32,
    pub commanded_pan_angle: f64,
    pub commanded_tilt_angle: f64,
    pub unlimited_pan_angle: f64,
    pub unlimited_tilt_angle: f64,
}

impl Default for FixtureState {
    /// `new MovingHeadSettings()`.
    fn default() -> FixtureState {
        FixtureState {
            pan_angle: 0.0,
            tilt_angle: 0.0,
            beam_length: 1.0,
            on_off: true,
            beam_color_left: Color::YELLOW,
            beam_color_right: Color::YELLOW,
            intensity: 100,
            focus: 100,
            legend: String::new(),
            strobe_rate: 0,
            commanded_pan_angle: 0.0,
            commanded_tilt_angle: 0.0,
            unlimited_pan_angle: 0.0,
            unlimited_tilt_angle: 0.0,
        }
    }
}

impl FixtureState {
    /// `MovingHeadsMoving`: the window keeps drawing while this is true, though no new frame arrives.
    pub fn is_moving(&self) -> bool {
        self.commanded_pan_angle != self.unlimited_pan_angle || self.commanded_tilt_angle != self.unlimited_tilt_angle
    }
}

/// A strobe timer's identity: the fixture's longest flash and the time between flashes, both in milliseconds.
type TimerKey = (i32, i32);

/// `MovingHeadIntentHandler` for one fixture.
#[derive(Clone, Debug)]
struct Handler {
    setup: FixtureSetup,
    state: FixtureState,
    legend_values: Vec<(String, String)>,
    strobe_mode: bool,
    strobe_intent: bool,
    color_present: bool,
    wheel_slot: i32,
    frame_counter: i32,
    spin_frames: i32,
    wheel_entry: Option<WheelEntry>,
    strobe_interval: i32,
    timer: Option<TimerKey>,
}

impl Handler {
    fn new(setup: FixtureSetup) -> Handler {
        Handler {
            setup,
            state: FixtureState::default(),
            legend_values: Vec::new(),
            strobe_mode: false,
            strobe_intent: false,
            color_present: false,
            wheel_slot: -1,
            frame_counter: 0,
            spin_frames: 0,
            wheel_entry: None,
            strobe_interval: 0,
            timer: None,
        }
    }

    /// `Reset`.
    fn reset(&mut self) {
        self.legend_values.clear();
        self.state.legend.clear();
        self.state.intensity = 0;
        self.color_present = false;
        self.strobe_intent = false;
        if !self.strobe_mode {
            self.state.on_off = false;
        }
        self.state.beam_color_left = Color::TRANSPARENT;
        self.state.beam_color_right = Color::TRANSPARENT;
        self.state.commanded_tilt_angle = if self.setup.invert_tilt { self.setup.tilt_stop } else { self.setup.tilt_start } as f64;
        self.state.commanded_pan_angle = self.setup.pan_start as f64;
    }

    /// `Dispatch`. The colour states go first: a combined state doesn't keep the order its parts arrived in.
    fn dispatch(&mut self, states: &CombinedState) {
        if let Some(l) = states.lighting {
            self.color(l.color, Some(l.intensity));
        }
        if let Some(rgb) = states.rgb {
            self.color(rgb.full_color(), Some(rgb.intensity()));
        }
        for d in &states.discrete {
            self.color(d.color, self.setup.convert_color_into_dimmer.then_some(d.intensity));
        }
        for other in &states.other {
            match other {
                StateValue::Range(r) => self.range(r),
                StateValue::Command(CommandValue::Named8(c)) => self.command(c),
                _ => {}
            }
        }
        self.frame_counter = self.frame_counter.wrapping_add(1);
    }

    fn color(&mut self, color: Color, intensity: Option<f64>) {
        self.open_shutter();
        self.state.beam_color_left = color;
        self.state.beam_color_right = color;
        if let Some(i) = intensity {
            self.state.intensity = (i * 100.0) as i32;
        }
        self.state.beam_length = self.setup.beam_length as f64;
    }

    fn open_shutter(&mut self) {
        self.color_present = true;
        if self.setup.convert_color_into_shutter && !self.strobe_mode {
            self.state.on_off = true;
        }
    }

    fn range(&mut self, r: &RangeValue) {
        let s = &self.setup;
        match r.tag_type {
            FunctionIdentity::Pan => {
                let span = (s.pan_stop - s.pan_start) as f64;
                self.state.commanded_pan_angle = if s.invert_pan { s.pan_start as f64 - r.value * span } else { r.value * span + s.pan_start as f64 };
            }
            FunctionIdentity::Tilt => {
                let span = (s.tilt_stop - s.tilt_start) as f64;
                self.state.commanded_tilt_angle = if s.invert_tilt { s.tilt_stop as f64 - r.value * span } else { r.value * span + s.tilt_start as f64 };
            }
            FunctionIdentity::Zoom => {
                self.state.focus = if s.zoom_narrow_to_wide { (r.value * 100.0) as i32 } else { ((1.0 - r.value) * 100.0) as i32 };
            }
            FunctionIdentity::Dim => self.state.intensity = (r.value * 100.0) as i32,
            _ => {}
        }
        self.update_legend(&r.label, ((r.value * 255.0) as i32).to_string());
    }

    fn command(&mut self, c: &NamedCommand) {
        self.update_legend(&c.label, c.value.to_string());
        match c.index_type {
            FixtureIndexType::LampOff | FixtureIndexType::ShutterClosed => self.state.on_off = false,
            FixtureIndexType::LampOn | FixtureIndexType::ShutterOpen => {
                if !self.strobe_mode {
                    self.state.on_off = true;
                }
            }
            FixtureIndexType::ColorWheel => self.color_wheel(c),
            FixtureIndexType::Strobe => {
                self.strobe_mode = true;
                self.strobe_intent = true;
                let slow_ms = 1.0 / self.setup.strobe_rate_minimum as f64 * 1000.0;
                let fast_ms = 1.0 / self.setup.strobe_rate_maximum as f64 * 1000.0;
                let ratio = (c.value as f64 - c.range_minimum as f64) / (c.range_maximum as f64 - c.range_minimum as f64);
                let rate_ms = ratio * (slow_ms - fast_ms) + fast_ms;
                // The two casts are upstream's: the sum is truncated, then the rate, so the result can be a millisecond off the exact flip.
                self.state.strobe_rate = ((slow_ms + fast_ms) as i32).wrapping_sub(rate_ms as i32);
            }
            _ => {}
        }
        self.state.beam_length = self.setup.beam_length as f64;
    }

    /// `HandleColorWheelCommand`. A command for a wheel the profile doesn't have throws upstream; here it does nothing.
    fn color_wheel(&mut self, c: &NamedCommand) {
        let Some(wheel) = self.setup.color_wheels.iter().find(|w| *w.function == *c.tag) else {
            return;
        };
        let value = c.value as i32;
        match wheel.entries.iter().find(|e| e.start_value == value) {
            Some(entry) if !entry.use_curve => {
                self.state.beam_color_left = entry.color1;
                self.state.beam_color_right = entry.color2;
                self.color_present = true;
            }
            _ => {
                let colors: Vec<WheelEntry> = wheel.entries.iter().filter(|e| !e.use_curve).cloned().collect();
                if colors.is_empty() {
                    return;
                }
                let ratio = 1.0 - (value - c.range_minimum as i32) as f64 / (c.range_maximum as i32 - c.range_minimum as i32) as f64;
                let s = &self.setup;
                let spin_seconds = ratio * (s.min_color_wheel_rotation_speed - s.max_color_wheel_rotation_speed) + s.max_color_wheel_rotation_speed;
                self.spin_frames = (spin_seconds * 1000.0 / colors.len() as f64 / s.update_interval_ms as f64) as i32;
                self.spin(&colors);
            }
        }
        // A spinning wheel alone doesn't count as colour: upstream sets the flag only for a fixed slot.
        if self.color_present {
            self.open_shutter();
        }
    }

    /// `SpinColorWheel`: the next colour once the current one has shown for its share of a turn.
    fn spin(&mut self, colors: &[WheelEntry]) {
        if self.wheel_entry.is_none() || self.frame_counter > self.spin_frames {
            self.frame_counter = 0;
            self.wheel_slot += 1;
            if self.wheel_slot > colors.len() as i32 - 1 {
                self.wheel_slot = 0;
            }
            self.wheel_entry = Some(colors[self.wheel_slot as usize].clone());
        }
        if let Some(entry) = &self.wheel_entry {
            self.state.beam_color_left = entry.color1;
            self.state.beam_color_right = entry.color2;
        }
        self.state.on_off = true;
    }

    fn update_legend(&mut self, label: &str, value: String) {
        if label.is_empty() {
            return;
        }
        match self.legend_values.iter_mut().find(|(l, _)| l == label) {
            Some(pair) => pair.1 = value,
            None => self.legend_values.push((label.to_string(), value)),
        }
        let mut items: Vec<String> = self.legend_values.iter().map(|(l, v)| format!("{l}{v}")).collect();
        items.sort_by(|a, b| legend_order(a, b));
        self.state.legend = items.join(", ");
    }

    /// The pan and tilt half of `FinalizeFixtureState`, for a standard frame.
    fn travel(&mut self) {
        let s = &self.setup;
        // Upstream names this the update interval, and divides whole numbers.
        let frames_per_second = (1000 / s.update_interval_ms.max(1)) as f64;
        let pan_rate = (s.pan_stop - s.pan_start) as f64 / s.max_pan_travel_time / frames_per_second;
        let tilt_rate = (s.tilt_stop - s.tilt_start) as f64 / s.max_tilt_travel_time / frames_per_second;
        self.state.unlimited_pan_angle = approach(self.state.unlimited_pan_angle, self.state.commanded_pan_angle, pan_rate);
        self.state.unlimited_tilt_angle = approach(self.state.unlimited_tilt_angle, self.state.commanded_tilt_angle, tilt_rate);
        self.state.tilt_angle = limit_angle(self.state.unlimited_tilt_angle);
        self.state.pan_angle = limit_angle(self.state.unlimited_pan_angle);
    }
}

/// One frame's step of an angle towards where it was told to go. Kept as written: with a rate of zero or less (a stop position
/// at or below the start) the angle never lands, it drifts.
fn approach(current: f64, commanded: f64, rate: f64) -> f64 {
    if current > commanded {
        if current - commanded < rate {
            commanded
        } else {
            current - rate
        }
    } else if commanded - current < rate {
        commanded
    } else {
        current + rate
    }
}

/// `LimitAngle`.
fn limit_angle(mut angle: f64) -> f64 {
    if !angle.is_finite() {
        return angle;
    }
    while angle > 360.0 {
        angle -= 360.0;
    }
    while angle < -360.0 {
        angle += 360.0;
    }
    angle
}

/// `List<string>.Sort()` compares by culture: letters before their case matters, lower case first on a tie. This is that much of
/// it and no more, which is enough for labels like "Pan", "pan" and "Tilt".
fn legend_order(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| b.cmp(a))
}

/// `MovingHeadTimer`: every fixture strobing at one rate flashes together.
#[derive(Clone, Debug)]
struct StrobeTimer {
    key: TimerKey,
    /// How long a flash lasts: a quarter of the interval, at most the fixture's longest.
    flash_ms: f64,
    heads: Vec<usize>,
    next_tick: f64,
    off_at: Option<f64>,
    /// The heads a flash turned on, which are the ones it turns off even if some have left since.
    flashing: Vec<usize>,
}

/// Every fixture of one live window, with the strobe timers they share.
#[derive(Clone, Debug, Default)]
pub struct FixtureSet {
    handlers: Vec<Handler>,
    timers: Vec<StrobeTimer>,
}

impl FixtureSet {
    /// A new set starts with no timers, as the OpenGL form resets them whenever it initialises.
    pub fn new(setups: Vec<FixtureSetup>) -> FixtureSet {
        FixtureSet { handlers: setups.into_iter().map(Handler::new).collect(), timers: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn state(&self, index: usize) -> &FixtureState {
        &self.handlers[index].state
    }

    pub fn any_moving(&self) -> bool {
        self.handlers.iter().any(|h| h.state.is_moving())
    }

    /// `PreviewMovingHead.UpdateVolumes` without the drawing: one fixture's state for a frame. `standard` is false for the redraw a
    /// strobe flash asks for, which moves no head. `now_ms` is the clock [`tick`](Self::tick) uses.
    pub fn frame(&mut self, index: usize, states: Option<&CombinedState>, standard: bool, now_ms: f64) {
        let h = &mut self.handlers[index];
        h.reset();
        if let Some(states) = states {
            h.dispatch(states);
        }
        if h.strobe_intent && h.color_present {
            h.strobe_mode = true;
            h.strobe_interval = h.state.strobe_rate;
            if h.timer.is_none() {
                self.register(index, now_ms);
            }
        } else {
            h.strobe_mode = false;
            if let Some(key) = h.timer.take() {
                self.unregister(index, key);
            }
        }
        if standard {
            self.handlers[index].travel();
        }
    }

    /// `CreateOrRegisterWithTimer`. An interval the system timer would refuse gets no timer, so the head doesn't flash.
    fn register(&mut self, index: usize, now_ms: f64) {
        let h = &mut self.handlers[index];
        if h.strobe_interval <= 0 {
            return;
        }
        let key = (h.setup.maximum_strobe_duration, h.strobe_interval);
        h.timer = Some(key);
        match self.timers.iter_mut().find(|t| t.key == key) {
            Some(timer) => {
                // A stopped timer starts again from now.
                if timer.heads.is_empty() {
                    timer.next_tick = now_ms + key.1 as f64;
                }
                timer.heads.push(index);
            }
            None => self.timers.push(StrobeTimer {
                key,
                flash_ms: (key.1 / 4).min(key.0) as f64,
                heads: vec![index],
                next_tick: now_ms + key.1 as f64,
                off_at: None,
                flashing: Vec::new(),
            }),
        }
    }

    fn unregister(&mut self, index: usize, key: TimerKey) {
        if let Some(timer) = self.timers.iter_mut().find(|t| t.key == key) {
            timer.heads.retain(|h| *h != index);
        }
    }

    /// Runs the strobe timers up to `now_ms`. True when a beam went on or off, so the window has to draw again, with
    /// [`frame`](Self::frame) called first with `standard` false as upstream's redraw does.
    pub fn tick(&mut self, now_ms: f64) -> bool {
        let mut changed = false;
        let mut moved: Vec<usize> = Vec::new();
        for timer in &mut self.timers {
            if timer.off_at.is_some_and(|at| at <= now_ms) {
                timer.off_at = None;
                for &head in &timer.flashing {
                    self.handlers[head].state.on_off = false;
                    // `DetermineStrobeTimer`, which the timer calls after each flash: a head whose rate changed moves to that rate's timer.
                    // A head that left the timer during the flash stays out, where upstream could put it back on one.
                    let h = &self.handlers[head];
                    if h.timer == Some(timer.key) && h.state.strobe_rate != timer.key.1 && h.strobe_interval != timer.key.1 {
                        moved.push(head);
                    }
                }
                changed |= !timer.flashing.is_empty();
                timer.flashing.clear();
            }
            if !timer.heads.is_empty() && timer.next_tick <= now_ms {
                // A late clock flashes once and takes up the rhythm again, where upstream's timer would queue every tick it missed.
                while timer.next_tick <= now_ms {
                    timer.next_tick += timer.key.1 as f64;
                }
                timer.flashing = timer.heads.clone();
                for &head in &timer.flashing {
                    self.handlers[head].state.on_off = true;
                }
                timer.off_at = Some(now_ms + timer.flash_ms);
                changed = true;
            }
        }
        for head in moved {
            if let Some(key) = self.handlers[head].timer.take() {
                self.unregister(head, key);
            }
            self.register(head, now_ms);
        }
        changed
    }

    /// When [`tick`](Self::tick) next has something to do, for the clock that calls it.
    pub fn next_deadline(&self) -> Option<f64> {
        self.timers
            .iter()
            .flat_map(|t| [t.off_at, (!t.heads.is_empty()).then_some(t.next_tick)])
            .flatten()
            .min_by(|a, b| a.total_cmp(b))
    }
}

/// The two numbers `RenderStaticPreviewShapes` works out for its moving heads: how much the window scales the reference picture,
/// and the longest beam in reference units. The reference is the background's size, or the window's when there is none.
pub fn beam_geometry(client: (f32, f32), reference: (f32, f32)) -> (f32, i32) {
    let size_scale = (client.0 / reference.0 + client.1 / reference.1) / 2.0;
    // `Math.Round(..., MidpointRounding.AwayFromZero)`, which is what `round` does.
    let max_beam_length = (client.0.max(client.1) as f64 / size_scale as f64).round() as i32;
    (size_scale, max_beam_length)
}

/// `(100 - BeamTransparency) / 100`.
pub fn beam_alpha(beam_transparency: i32) -> f64 {
    (100.0 - beam_transparency as f64) / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{DiscreteValue, LightingValue, RgbValue};

    fn setup() -> FixtureSetup {
        FixtureSetup {
            pan_start: 0,
            pan_stop: 540,
            tilt_start: 0,
            tilt_stop: 180,
            invert_pan: false,
            invert_tilt: false,
            strobe_rate_minimum: 1,
            strobe_rate_maximum: 25,
            max_pan_travel_time: 2.0,
            max_tilt_travel_time: 1.5,
            min_color_wheel_rotation_speed: 10.0,
            max_color_wheel_rotation_speed: 1.0,
            maximum_strobe_duration: 50,
            zoom_narrow_to_wide: true,
            beam_length: 20,
            convert_color_into_shutter: true,
            convert_color_into_dimmer: false,
            color_wheels: Vec::new(),
            update_interval_ms: 50,
        }
    }

    fn range(identity: FunctionIdentity, value: f64, label: &str) -> StateValue {
        StateValue::Range(RangeValue::new(identity, "tag", value, label))
    }

    fn command(index_type: FixtureIndexType, value: u8, tag: &str, label: &str, min: u8, max: u8) -> StateValue {
        StateValue::Command(CommandValue::Named8(NamedCommand { value, index_type, tag: tag.into(), label: label.into(), range_minimum: min, range_maximum: max }))
    }

    fn states(other: Vec<StateValue>) -> CombinedState {
        CombinedState { other: other.into_iter().collect(), ..CombinedState::default() }
    }

    fn lit(mut s: CombinedState) -> CombinedState {
        s.lighting = Some(LightingValue::new(Color::RED, 0.5));
        s
    }

    #[test]
    fn a_frame_with_no_states_is_dark_and_parked() {
        let mut set = FixtureSet::new(vec![setup(), FixtureSetup { invert_tilt: true, ..setup() }]);
        set.frame(0, None, false, 0.0);
        set.frame(1, Some(&CombinedState::default()), false, 0.0);
        let s = set.state(0);
        assert!(!s.on_off);
        assert_eq!((s.intensity, s.beam_color_left, s.beam_color_right), (0, Color::TRANSPARENT, Color::TRANSPARENT));
        assert_eq!((s.commanded_pan_angle, s.commanded_tilt_angle), (0.0, 0.0));
        assert_eq!(s.legend, "");
        assert_eq!(set.state(1).commanded_tilt_angle, 180.0);
    }

    #[test]
    fn range_intents_set_angles_zoom_and_dimmer() {
        let frame = states(vec![
            range(FunctionIdentity::Pan, 0.5, ""),
            range(FunctionIdentity::Tilt, 0.25, ""),
            range(FunctionIdentity::Zoom, 0.25, ""),
            range(FunctionIdentity::Dim, 0.75, ""),
            range(FunctionIdentity::Gobo, 0.9, ""),
        ]);
        let mut set = FixtureSet::new(vec![
            setup(),
            FixtureSetup { invert_pan: true, invert_tilt: true, zoom_narrow_to_wide: false, pan_start: 90, pan_stop: 450, ..setup() },
        ]);
        set.frame(0, Some(&frame), false, 0.0);
        set.frame(1, Some(&frame), false, 0.0);
        let s = set.state(0);
        assert_eq!((s.commanded_pan_angle, s.commanded_tilt_angle, s.focus, s.intensity), (270.0, 45.0, 25, 75));
        // Nothing lit it: a range intent opens no shutter.
        assert!(!s.on_off);
        let s = set.state(1);
        assert_eq!((s.commanded_pan_angle, s.commanded_tilt_angle, s.focus), (-90.0, 135.0, 75));
    }

    #[test]
    fn colour_intents_set_the_beam_and_open_the_shutter_when_the_filter_says_so() {
        let mut set = FixtureSet::new(vec![setup(), FixtureSetup { convert_color_into_shutter: false, ..setup() }, FixtureSetup { convert_color_into_dimmer: true, ..setup() }]);
        let lighting = lit(CombinedState::default());
        set.frame(0, Some(&lighting), false, 0.0);
        set.frame(1, Some(&lighting), false, 0.0);
        let s = set.state(0);
        assert!(s.on_off);
        assert_eq!((s.beam_color_left, s.beam_color_right, s.intensity, s.beam_length), (Color::RED, Color::RED, 50, 20.0));
        assert!(!set.state(1).on_off);
        assert_eq!(set.state(1).beam_color_left, Color::RED);

        let rgb = CombinedState { rgb: Some(RgbValue { r: 0, g: 128, b: 0 }), ..CombinedState::default() };
        set.frame(0, Some(&rgb), false, 0.0);
        assert_eq!((set.state(0).beam_color_left, set.state(0).intensity), (Color::rgb(0, 128, 0), 50));

        // A discrete colour dims only under a Dimming filter that converts colour.
        let discrete = CombinedState { discrete: [DiscreteValue::new(Color::BLUE, 0.4)].into_iter().collect(), ..CombinedState::default() };
        set.frame(0, Some(&discrete), false, 0.0);
        set.frame(2, Some(&discrete), false, 0.0);
        assert_eq!((set.state(0).beam_color_left, set.state(0).intensity), (Color::BLUE, 0));
        assert_eq!(set.state(2).intensity, 40);
    }

    #[test]
    fn lamp_and_shutter_commands_switch_the_beam() {
        let mut set = FixtureSet::new(vec![FixtureSetup { convert_color_into_shutter: false, ..setup() }]);
        set.frame(0, Some(&states(vec![command(FixtureIndexType::ShutterOpen, 255, "Shutter", "", 0, 255)])), false, 0.0);
        assert!(set.state(0).on_off);
        assert_eq!(set.state(0).beam_length, 20.0);
        set.frame(0, Some(&states(vec![command(FixtureIndexType::LampOn, 1, "Lamp", "", 0, 255), command(FixtureIndexType::ShutterClosed, 0, "Shutter", "", 0, 255)])), false, 0.0);
        assert!(!set.state(0).on_off);
        // A plain 8-bit command says nothing to a fixture.
        set.frame(0, Some(&states(vec![StateValue::Command(CommandValue::U8(200))])), false, 0.0);
        assert!(!set.state(0).on_off);
    }

    #[test]
    fn pan_and_tilt_travel_at_the_fixtures_speed_and_land_exactly() {
        let mut set = FixtureSet::new(vec![setup()]);
        let frame = states(vec![range(FunctionIdentity::Pan, 0.25, ""), range(FunctionIdentity::Tilt, 1.0, "")]);
        // 540 degrees in 2 s at 20 frames a second is 13.5 degrees a frame. Tilt: 180 in 1.5 s is 6 a frame.
        set.frame(0, Some(&frame), true, 0.0);
        assert_eq!((set.state(0).pan_angle, set.state(0).tilt_angle), (13.5, 6.0));
        assert!(set.any_moving());
        // A strobe redraw moves nothing.
        set.frame(0, Some(&frame), false, 0.0);
        assert_eq!(set.state(0).pan_angle, 13.5);
        for _ in 0..5 {
            set.frame(0, Some(&frame), true, 0.0);
        }
        assert_eq!((set.state(0).pan_angle, set.state(0).tilt_angle), (81.0, 36.0));
        for _ in 0..30 {
            set.frame(0, Some(&frame), true, 0.0);
        }
        assert_eq!((set.state(0).pan_angle, set.state(0).tilt_angle), (135.0, 180.0));
        assert!(!set.any_moving());
        // With the intents gone the head goes back to its start, as fast.
        set.frame(0, None, true, 0.0);
        assert_eq!((set.state(0).pan_angle, set.state(0).tilt_angle), (121.5, 174.0));
    }

    #[test]
    fn angles_stay_within_a_full_turn_either_way() {
        assert_eq!(limit_angle(540.0), 180.0);
        assert_eq!(limit_angle(-725.0), -5.0);
        assert_eq!(limit_angle(360.0), 360.0);
        let mut set = FixtureSet::new(vec![FixtureSetup { max_pan_travel_time: 0.03125, ..setup() }]);
        set.frame(0, Some(&states(vec![range(FunctionIdentity::Pan, 1.0, "")])), true, 0.0);
        assert_eq!((set.state(0).unlimited_pan_angle, set.state(0).pan_angle), (540.0, 180.0));
    }

    #[test]
    fn legend_lists_labelled_values_sorted() {
        let mut set = FixtureSet::new(vec![setup()]);
        set.frame(
            0,
            Some(&states(vec![
                range(FunctionIdentity::Tilt, 0.5, "T: "),
                range(FunctionIdentity::Pan, 1.0, "P: "),
                range(FunctionIdentity::Zoom, 0.2, ""),
                command(FixtureIndexType::ShutterOpen, 255, "Shutter", "S: ", 0, 255),
                range(FunctionIdentity::Pan, 0.0, "P: "),
            ])),
            false,
            0.0,
        );
        assert_eq!(set.state(0).legend, "P: 0, S: 255, T: 127");
        set.frame(0, None, false, 0.0);
        assert_eq!(set.state(0).legend, "");
        assert_eq!(legend_order("pan1", "Pan1"), Ordering::Less);
        assert_eq!(legend_order("Tilt", "pan"), Ordering::Greater);
    }

    fn wheel() -> ColorWheel {
        let entry = |start_value, use_curve, color1| WheelEntry { start_value, use_curve, color1, color2: color1 };
        ColorWheel {
            function: "Color".into(),
            entries: vec![entry(0, false, Color::WHITE), entry(10, false, Color::RED), entry(20, false, Color::BLUE), entry(128, true, Color::BLACK)],
        }
    }

    #[test]
    fn a_fixed_wheel_slot_colours_the_beam_and_counts_as_colour() {
        let mut set = FixtureSet::new(vec![FixtureSetup { color_wheels: vec![wheel()], ..setup() }]);
        set.frame(0, Some(&states(vec![command(FixtureIndexType::ColorWheel, 10, "Color", "", 0, 255)])), false, 0.0);
        assert_eq!((set.state(0).beam_color_left, set.state(0).on_off), (Color::RED, true));
        // A wheel the profile doesn't have changes nothing.
        set.frame(0, Some(&states(vec![command(FixtureIndexType::ColorWheel, 10, "Other", "", 0, 255)])), false, 0.0);
        assert_eq!((set.state(0).beam_color_left, set.state(0).on_off), (Color::TRANSPARENT, false));
    }

    #[test]
    fn a_spinning_wheel_steps_through_its_colours() {
        let mut set = FixtureSet::new(vec![FixtureSetup { color_wheels: vec![wheel()], ..setup() }]);
        // Fastest spin: one second a turn, three colours, 50 ms frames: (int)(1000 / 3 / 50) = 6 frames a colour.
        let spin = states(vec![command(FixtureIndexType::ColorWheel, 255, "Color", "", 128, 255)]);
        let mut seen = Vec::new();
        for _ in 0..24 {
            set.frame(0, Some(&spin), true, 0.0);
            assert!(set.state(0).on_off);
            seen.push(set.state(0).beam_color_left);
        }
        assert_eq!(seen[0], Color::WHITE);
        let changes: Vec<usize> = (1..seen.len()).filter(|i| seen[*i] != seen[*i - 1]).collect();
        assert_eq!(changes, vec![7, 14, 21]);
        assert_eq!((seen[7], seen[14], seen[21]), (Color::RED, Color::BLUE, Color::WHITE));
    }

    #[test]
    fn strobe_rate_maps_the_command_range_onto_the_fixtures_rates() {
        let mut set = FixtureSet::new(vec![setup()]);
        // 1 Hz is 1000 ms, 25 Hz is 40 ms. The bottom of the command's range is the slow end.
        set.frame(0, Some(&states(vec![command(FixtureIndexType::Strobe, 10, "Strobe", "", 10, 110)])), false, 0.0);
        assert_eq!(set.state(0).strobe_rate, 1040 - 40);
        set.frame(0, Some(&states(vec![command(FixtureIndexType::Strobe, 110, "Strobe", "", 10, 110)])), false, 0.0);
        assert_eq!(set.state(0).strobe_rate, 1040 - 1000);
        set.frame(0, Some(&states(vec![command(FixtureIndexType::Strobe, 60, "Strobe", "", 10, 110)])), false, 0.0);
        assert_eq!(set.state(0).strobe_rate, 1040 - 520);
    }

    #[test]
    fn strobing_heads_flash_together_and_stop_with_the_intent() {
        let mut set = FixtureSet::new(vec![setup(), setup(), setup()]);
        let strobe = lit(states(vec![command(FixtureIndexType::Strobe, 110, "Strobe", "", 10, 110)]));
        // No colour, so no strobe mode: the strobe intent alone leaves the beam as it was.
        set.frame(2, Some(&states(vec![command(FixtureIndexType::Strobe, 110, "Strobe", "", 10, 110)])), true, 0.0);
        assert_eq!(set.next_deadline(), None);

        set.frame(0, Some(&strobe), true, 0.0);
        set.frame(1, Some(&strobe), true, 0.0);
        // Colour came before the strobe intent on the first frame, so the shutter opened. From the next frame the timer owns it.
        assert_eq!(set.next_deadline(), Some(40.0));
        assert!(!set.tick(39.0));
        assert!(set.tick(40.0));
        assert!(set.state(0).on_off && set.state(1).on_off);
        // 40 ms between flashes, each a quarter of that long.
        assert_eq!(set.next_deadline(), Some(50.0));
        set.frame(0, Some(&strobe), false, 41.0);
        assert!(set.state(0).on_off, "a frame during a flash leaves the beam on");
        assert!(set.tick(50.0));
        assert!(!set.state(0).on_off && !set.state(1).on_off);
        set.frame(0, Some(&strobe), true, 55.0);
        assert!(!set.state(0).on_off, "between flashes colour doesn't open the shutter");
        assert_eq!(set.next_deadline(), Some(80.0));

        // One head stops strobing: it leaves the timer and its beam follows its intents again.
        set.frame(1, Some(&lit(CombinedState::default())), true, 60.0);
        set.frame(1, Some(&lit(CombinedState::default())), true, 61.0);
        assert!(set.state(1).on_off);
        assert!(set.tick(80.0));
        assert!(set.state(0).on_off);
        set.frame(0, None, true, 81.0);
        assert!(set.tick(90.0));
        assert_eq!(set.next_deadline(), None);
    }

    #[test]
    fn a_head_whose_rate_changes_moves_to_another_timer_after_its_flash() {
        let mut set = FixtureSet::new(vec![FixtureSetup { maximum_strobe_duration: 5, ..setup() }]);
        let fast = lit(states(vec![command(FixtureIndexType::Strobe, 110, "Strobe", "", 10, 110)]));
        let slow = lit(states(vec![command(FixtureIndexType::Strobe, 10, "Strobe", "", 10, 110)]));
        set.frame(0, Some(&fast), true, 0.0);
        set.frame(0, Some(&slow), true, 10.0);
        // Still on the 40 ms timer until its next flash is over. The flash is cut to the fixture's longest, 5 ms.
        assert_eq!(set.next_deadline(), Some(40.0));
        assert!(set.tick(40.0));
        assert_eq!(set.next_deadline(), Some(45.0));
        assert!(set.tick(45.0));
        assert_eq!(set.next_deadline(), Some(45.0 + 1000.0));
    }

    #[test]
    fn beam_geometry_follows_the_window_and_its_reference() {
        assert_eq!(beam_geometry((800.0, 600.0), (800.0, 600.0)), (1.0, 800));
        let (scale, length) = beam_geometry((1600.0, 600.0), (800.0, 600.0));
        assert_eq!((scale, length), (1.5, 1067));
        assert_eq!(beam_alpha(40), 0.6);
    }
}
