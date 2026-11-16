//! Port of VixenModules.Effect.Liquid: emitters pour particles into a LiquidFun world, and each particle lights the
//! pixel it is on. The physics is LiquidFun itself, built from `vendor/Box2D` and reached through `world`.
//!
//! The C# keeps its emitters between renders, so an emitter's velocity sign, nozzle angle and colour index carry over
//! when an effect is rendered again. Here each render starts from the values a freshly loaded effect has.

mod audio;
mod world;

use crate::generated::liquid as gen;
use audio::{AudioSettings, AudioUtilities};
use std::sync::atomic::{AtomicBool, Ordering};
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;
use world::{Barriers, LfEmitter, LfParticle, LiquidWorld};

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    param_changed_ctx: Some(param_changed_ctx),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

const SPACING: i32 = 50;
const MAX_EMITTER_ANIMATE_SPEED: f64 = 10.0;
const MAX_EMITTER_LIFE_TIME: f64 = 1000.0;
const MAX_EMITTER_PARTICLE_VELOCITY: f64 = 100.0;
const MAX_EMITTER_SOURCE_SIZE: f64 = 100.0;
const MAX_EMITTER_FLOW: f64 = 200.0;
const MAX_EMITTER_MUSIC_FLOW: i32 = 100;

/// `EnableTargetPositioning(true, true)` and `UpdateAllAttributes`.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let locations = p.enum_name("TargetPositioning") == "Locations";
    v.set("TargetPositioning", true);
    v.set("StringOrientation", !locations);
    v.set("RenderScaleFactor", locations);

    let emitters = p.records("EmitterData");
    let musical = t.audio_loaded && emitters.iter().any(|e| e.enum_name("FlowControl") == "Musical");
    for name in ["LowPass", "HighPass", "Range", "Normalize", "DecayTime", "AttackTime", "Gain"] {
        v.set(name, musical);
    }
    // `UpdateLowHighPassAttributes` runs last, so the frequencies follow their switches alone.
    v.set("LowPassFreq", p.boolean("LowPass"));
    v.set("HighPassFreq", p.boolean("HighPass"));

    v.set("Colors", emitters.iter().any(|e| e.boolean("UseColorArray")));
}

/// `Emitter.InitAllAttributes`, for an editor showing an `EmitterData` record.
pub(crate) fn emitter_visibility(e: &Params, _effect: &Params, v: &mut Visibility) {
    let use_color_array = e.boolean("UseColorArray");
    v.set("FramesPerColor", use_color_array);
    v.set("Color", !use_color_array);

    let animate = e.boolean("Animate");
    let random_start = e.boolean("RandomStartingPosition");
    v.set("VelocityX", animate);
    v.set("VelocityY", animate);
    v.set("EdgeHandling", animate);
    v.set("X", !animate);
    v.set("Y", !animate);
    v.set("AnimateXStart", animate && !random_start);
    v.set("AnimateYStart", animate && !random_start);
    v.set("RandomStartingPosition", animate);

    let movement = e.enum_name("NozzleMovement");
    v.set("NozzleAngle", movement == "FixedAngle");
    v.set("OscillateStartAngle", movement == "Oscillate");
    v.set("OscillateEndAngle", movement == "Oscillate");
    v.set("OscillationSpeed", matches!(movement, "Oscillate" | "SpinClockwise" | "SpinCounterClockwise"));

    let flow = e.enum_name("FlowControl");
    v.set("OnTime", flow == "Pulsating");
    v.set("OffTime", flow == "Pulsating");
    v.set("Flow", flow == "Pulsating" || flow == "Continuous");
    v.set("MarkCollectionId", flow == "UseMarks");
}

/// The side effect of the base class's `TargetPositioning` setter.
fn param_changed(name: &str, p: &mut Params) {
    if name == "TargetPositioning" && p.enum_name("TargetPositioning") == "Locations" {
        p.set("StringOrientation", ParamValue::Enum("Vertical".to_string()));
    }
}

/// `GetMarkCollectionSelections`: every emitter holds its own selection, active while its flow follows marks. The C# setters
/// of an emitter's row normalise it too, and an edited row reaches this as the whole list.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if !matches!(c.name, MARK_COLLECTIONS | "EmitterData") || (c.info.targets.is_empty() && c.info.marks.is_empty()) {
        return;
    }
    let Some(i) = p.index_of("EmitterData") else { return };
    if let ParamValue::RecordList(emitters) = &mut p.values[i] {
        for e in emitters {
            if e.enum_name("FlowControl") == "UseMarks" {
                let id = normalized_mark_selection(e.guid("MarkCollectionId"), c.info.marks, None, true);
                e.set("MarkCollectionId", ParamValue::Guid(id));
            }
        }
    }
}

/// `EdgeHandling`.
#[derive(Copy, Clone, PartialEq, Debug)]
enum EdgeHandling {
    Bounce,
    Wrap,
}

/// `NozzleMovement`.
#[derive(Copy, Clone, PartialEq, Debug)]
enum NozzleMovement {
    FixedAngle,
    Oscillate,
    SpinClockwise,
    SpinCounterClockwise,
}

/// `FlowControl`.
#[derive(Copy, Clone, PartialEq, Debug)]
enum FlowControl {
    Continuous,
    Pulsating,
    UseMarks,
    Musical,
}

/// `WrapperParticleType`, whose index the shim takes.
fn particle_type_index(name: &str) -> i32 {
    match name {
        "Elastic" => 0,
        "Powder" => 1,
        "Tensile" => 2,
        "Spring" => 3,
        "Viscous" => 4,
        "StaticPressure" => 5,
        "Reactive" => 7,
        "Repulsive" => 8,
        _ => 6, // Water
    }
}

/// `IEmitter`: the settings, plus the state that is not persisted but is used for rendering.
struct Emitter<'a> {
    particle_type: i32,
    use_color_array: bool,
    frames_per_color: i32,
    color: ColorGradient,
    brightness: Curve,
    lifetime: Curve,
    particle_velocity: Curve,
    animate: bool,
    random_starting_position: bool,
    animate_x_start: i32,
    animate_y_start: i32,
    edge_handling: EdgeHandling,
    velocity_x: Curve,
    velocity_y: Curve,
    x: Curve,
    y: Curve,
    flow: Curve,
    source_size: Curve,
    nozzle_movement: NozzleMovement,
    nozzle_angle: Curve,
    oscillate_start_angle: i32,
    oscillate_end_angle: i32,
    oscillation_speed: Curve,
    flow_control: FlowControl,
    /// The selected collection, when `MarkCollectionName` would come back non-empty.
    marks: Option<&'a MarkCollection>,
    on_time: i32,
    off_time: i32,

    int_angle: i32,
    oscillation_clockwise: bool,
    angle_oscillation_delta: i32,
    location_x: f64,
    location_y: f64,
    vel_x: f64,
    vel_y: f64,
    on: bool,
    on_timer: i32,
    off_timer: i32,
    frame_color_counter: i32,
    color_array_index: usize,
}

impl<'a> Emitter<'a> {
    /// One iteration of `UpdateEmitterModel`.
    fn from_record(r: &Params, collections: &'a [MarkCollection]) -> Emitter<'a> {
        let flow_control = match r.enum_name("FlowControl") {
            "Pulsating" => FlowControl::Pulsating,
            "UseMarks" => FlowControl::UseMarks,
            "Musical" => FlowControl::Musical,
            _ => FlowControl::Continuous,
        };
        Emitter {
            particle_type: particle_type_index(r.enum_name("ParticleType")),
            use_color_array: r.boolean("UseColorArray"),
            frames_per_color: r.int("FramesPerColor"),
            color: r.gradient("Color"),
            // Files from the first release of this effect have no Brightness. Null keeps the constructor's 100%.
            brightness: match r.get("Brightness") {
                ParamValue::Curve(c) => c.clone(),
                _ => Curve::flat100(),
            },
            lifetime: r.curve("Lifetime"),
            particle_velocity: r.curve("ParticleVelocity"),
            animate: r.boolean("Animate"),
            random_starting_position: r.boolean("RandomStartingPosition"),
            animate_x_start: r.int("AnimateXStart"),
            animate_y_start: r.int("AnimateYStart"),
            edge_handling: if r.enum_name("EdgeHandling") == "Wrap" { EdgeHandling::Wrap } else { EdgeHandling::Bounce },
            velocity_x: r.curve("VelocityX"),
            velocity_y: r.curve("VelocityY"),
            x: r.curve("X"),
            y: r.curve("Y"),
            flow: r.curve("Flow"),
            source_size: r.curve("SourceSize"),
            nozzle_movement: match r.enum_name("NozzleMovement") {
                "Oscillate" => NozzleMovement::Oscillate,
                "SpinClockwise" => NozzleMovement::SpinClockwise,
                "SpinCounterClockwise" => NozzleMovement::SpinCounterClockwise,
                _ => NozzleMovement::FixedAngle,
            },
            nozzle_angle: r.curve("NozzleAngle"),
            oscillate_start_angle: r.int("OscillateStartAngle"),
            oscillate_end_angle: r.int("OscillateEndAngle"),
            oscillation_speed: r.curve("OscillationSpeed"),
            flow_control,
            marks: match flow_control {
                FlowControl::UseMarks => selected_collection(r.guid("MarkCollectionId"), collections),
                _ => None,
            },
            on_time: r.int("OnTime"),
            off_time: r.int("OffTime"),

            // `new Emitter()`.
            int_angle: 5,
            oscillation_clockwise: true,
            angle_oscillation_delta: 5,
            location_x: 0.0,
            location_y: 0.0,
            vel_x: 1.0,
            vel_y: 1.0,
            on: true,
            on_timer: 0,
            off_timer: 0,
            frame_color_counter: 0,
            color_array_index: 0,
        }
    }
}

/// The mark collection a mark controlled emitter ends up with. `MarkCollectionSelectionService.Normalize` runs whenever
/// the sequence's collections change: without a valid selection an emitter takes the first collection. The result counts
/// only if it has a name, because `UpdateEmitterFlow` tests `MarkCollectionName`.
fn selected_collection(id: Uuid, collections: &[MarkCollection]) -> Option<&MarkCollection> {
    collections.iter().find(|m| !id.is_nil() && m.id == id).or(collections.first()).filter(|m| !m.name.is_empty())
}

struct Liquid<'a> {
    barriers: Barriers,
    mix_colors: bool,
    particle_size: i32,
    warm_up_frames: i32,
    despeckle_threshold: i32,
    render_scale_factor: i32,
    colors: Vec<ColorGradient>,
    locations: bool,
    audio: Option<&'a dyn AudioSource>,
    audio_settings: AudioSettings,
    cancel: &'a AtomicBool,
    rand: Rand,
    emitters: Vec<Emitter<'a>>,

    /// `_bufferWt` and `_bufferHt`, the logical render area.
    buffer_wt: i32,
    buffer_ht: i32,
    frame_time: i32,
    world: Option<LiquidWorld>,
    wrappers: Vec<LfEmitter>,
    particles: Vec<LfParticle>,
    frame_volumes: Vec<f64>,
    min_volume: f64,
    max_volume: f64,
    /// The render threw in the C#, which leaves the effect dark.
    failed: bool,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    render_seeded(ctx, p, Rand::default())
}

fn render_seeded(ctx: &RenderCtx, p: &Params, rand: Rand) -> EffectIntents {
    // `_renderEmitterList[0]` throws in SetupRender when there are no emitters.
    if p.records("EmitterData").is_empty() {
        return EffectIntents::new();
    }
    let positioning = TargetPositioning::from_name(p.enum_name("TargetPositioning"));
    // The TargetPositioning setter forces vertical strings for locations, so the two always go together.
    let orientation = match positioning {
        TargetPositioning::Locations => StringOrientation::Vertical,
        TargetPositioning::Strings => StringOrientation::from_name(p.enum_name("StringOrientation")),
    };
    let mut e = Liquid::new(ctx, p, rand);
    let out = render_pixel(ctx, orientation, positioning, &mut e);
    if e.failed {
        EffectIntents::new()
    } else {
        out
    }
}

impl<'a> Liquid<'a> {
    fn new(ctx: &RenderCtx<'a>, p: &Params, rand: Rand) -> Liquid<'a> {
        Liquid {
            barriers: Barriers {
                bottom: p.boolean("BottomBarrier"),
                top: p.boolean("TopBarrier"),
                left: p.boolean("LeftBarrier"),
                right: p.boolean("RightBarrier"),
            },
            mix_colors: p.boolean("MixColors"),
            particle_size: p.int("ParticleSize"),
            warm_up_frames: p.int("WarmUpFrames"),
            despeckle_threshold: p.int("DespeckleThreshold"),
            render_scale_factor: p.int("RenderScaleFactor"),
            colors: p.gradients("Colors"),
            locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
            audio: ctx.audio,
            audio_settings: AudioSettings {
                attack_time: p.int("AttackTime"),
                decay_time: p.int("DecayTime"),
                normalize: p.boolean("Normalize"),
                // `AudioUtilities.Gain = Gain`: that is the property, which is the stored value times ten.
                gain: (p.int("Gain").saturating_mul(10)) as f64,
                low_pass: p.boolean("LowPass"),
                low_pass_freq: p.int("LowPassFreq") as f64,
                high_pass: p.boolean("HighPass"),
                high_pass_freq: p.int("HighPassFreq") as f64,
            },
            cancel: ctx.cancel,
            rand,
            emitters: p.records("EmitterData").iter().map(|r| Emitter::from_record(r, ctx.marks)).collect(),
            buffer_wt: 0,
            buffer_ht: 0,
            frame_time: ctx.frame_ms,
            world: None,
            wrappers: Vec::new(),
            particles: Vec::new(),
            frame_volumes: Vec::new(),
            min_volume: 0.0,
            max_volume: 0.0,
            failed: false,
        }
    }

    /// `GetAudioSettings` and `CacheOffAudioVolumes`. Only a musical emitter reads the volumes, so the work of loading
    /// the audio is skipped without one. That can't change the output.
    fn cache_off_audio_volumes(&mut self, info: &PixelInfo) {
        if !self.emitters.iter().any(|e| e.flow_control == FlowControl::Musical) {
            return;
        }
        let Some(source) = self.audio else { return };
        let Some(audio) = AudioUtilities::reload(source, info.start_time, info.time_span, &self.audio_settings) else { return };

        self.frame_volumes.clear();
        self.min_volume = audio.volume_at_time(0);
        self.max_volume = self.min_volume;
        for frame_index in 1..=(info.time_span.total_ms() / SPACING as f64) as i32 {
            let current_volume = audio.volume_at_time(frame_index * SPACING);
            self.frame_volumes.push(current_volume);
            if current_volume < self.min_volume {
                self.min_volume = current_volume;
            } else if current_volume > self.max_volume {
                self.max_volume = current_volume;
            }
        }
    }

    /// `CalculateEmitterMusicalFlow`. With no audio, the empty range makes this NaN and the cast makes that zero.
    fn musical_flow(&self, frame: i32) -> i32 {
        let volume = self.frame_volumes.get(frame.max(0) as usize).copied().unwrap_or(0.0);
        let val = convert_range(self.min_volume, self.max_volume, 0.0, 1.0, volume);
        (val * MAX_EMITTER_MUSIC_FLOW as f64) as i32
    }

    /// `UpdateEmitterFlow`.
    fn update_emitter_flow(&mut self, wrapper: &mut LfEmitter, index: usize, frame: i32, factor: f64, info: &PixelInfo) {
        let frame_time = self.frame_time;
        let flow_control = self.emitters[index].flow_control;
        match flow_control {
            FlowControl::Continuous => wrapper.flow = curve_flow(&self.emitters[index], factor),
            FlowControl::Pulsating => {
                let e = &mut self.emitters[index];
                if e.on {
                    e.on_timer += frame_time;
                    if e.on_timer >= e.on_time {
                        e.on = false;
                        e.on_timer = 0;
                    }
                } else {
                    e.off_timer += frame_time;
                    if e.off_timer >= e.off_time {
                        e.on = true;
                        e.off_timer = 0;
                    }
                }
            }
            FlowControl::UseMarks => {
                let e = &mut self.emitters[index];
                // Deliberate: with no collection selected this leaves `on` alone, so the emitter keeps flowing.
                if let Some(marks) = e.marks {
                    let now = info.start_time + TimeSpan::from_ms_i((frame_time * frame) as i64);
                    let end = info.start_time + info.time_span;
                    e.on = false;
                    // The first mark `MarksInclusiveOfTime(now, end)` returns.
                    if let Some(mark) = marks.marks_inclusive_of_time(now, end).first() {
                        if mark.start_time <= now && now <= mark.end_time() {
                            e.on = true;
                        }
                    }
                }
            }
            FlowControl::Musical => wrapper.flow = self.musical_flow(frame),
        }

        // If the emitter is intermittently On/Off then...
        if flow_control == FlowControl::Pulsating || flow_control == FlowControl::UseMarks {
            let e = &self.emitters[index];
            wrapper.flow = if e.on { curve_flow(e, factor) } else { 0 };
        }
    }

    /// `UpdateEmitterPosition`. Deliberate: the animation inside moves every emitter, and it runs once per emitter, so
    /// with two emitters they each move twice a frame.
    fn update_emitter_position(&mut self, wrapper: &mut LfEmitter, index: usize, interval_pos: f64, factor: f64) {
        self.animate_emitters(interval_pos, factor);
        let e = &self.emitters[index];
        if e.animate {
            wrapper.x = e.location_x as i32;
            wrapper.y = e.location_y as i32;
        } else {
            wrapper.x = round_curve(&e.x, factor, self.buffer_wt as f64);
            wrapper.y = round_curve(&e.y, factor, self.buffer_ht as f64);
        }
    }

    /// `AnimateEmitters`.
    fn animate_emitters(&mut self, interval_pos: f64, factor: f64) {
        let (buffer_wt, buffer_ht) = (self.buffer_wt, self.buffer_ht);
        for i in 0..self.emitters.len() {
            if self.emitters[i].animate {
                let e = &mut self.emitters[i];
                let previous_x = e.location_x;
                let previous_y = e.location_y;

                // The curves give the speed. The direction is kept from before.
                let negative_x = e.vel_x < 0.0;
                e.vel_x = scale_curve_to_value(e.velocity_x.get_value(factor), MAX_EMITTER_ANIMATE_SPEED, 0.0);
                if negative_x {
                    e.vel_x = -e.vel_x;
                }
                let negative_y = e.vel_y < 0.0;
                e.vel_y = scale_curve_to_value(e.velocity_y.get_value(factor), MAX_EMITTER_ANIMATE_SPEED, 0.0);
                if negative_y {
                    e.vel_y = -e.vel_y;
                }

                e.location_x += e.vel_x;
                e.location_y += e.vel_y;

                let delta_x = e.location_x - previous_x;
                let delta_y = e.location_y - previous_y;
                let angle_radians = delta_y.atan2(delta_x);
                e.int_angle = (angle_radians * 180.0 / std::f64::consts::PI) as i32;

                // Look for collisions. Deliberate: this includes the emitter itself, whose distance is zero, and
                // whose velocity therefore flips twice, which changes nothing.
                for j in 0..self.emitters.len() {
                    let (a, b) = (&self.emitters[i], &self.emitters[j]);
                    let distance_between_centers = ((a.location_x - b.location_x) * (a.location_x - b.location_x)
                        + (a.location_y - b.location_y) * (a.location_y - b.location_y))
                        .sqrt();
                    let radius_sum = (source_size(a, factor) + source_size(b, factor)) as f64;
                    if distance_between_centers <= radius_sum {
                        let other = &mut self.emitters[j];
                        other.vel_x = -other.vel_x;
                        other.vel_y = -other.vel_y;
                        let this = &mut self.emitters[i];
                        this.vel_x = -this.vel_x;
                        this.vel_y = -this.vel_y;
                    }
                }
            }

            // Deliberate: the 0..1 position goes into the curve here where everything else passes 0..100.
            let e = &mut self.emitters[i];
            let radius = source_size(e, interval_pos);
            let r = radius as f64;
            match e.edge_handling {
                EdgeHandling::Bounce => {
                    if e.location_x - r < 0.0 {
                        e.location_x = r;
                        e.vel_x = -e.vel_x;
                    } else if e.location_x + r >= buffer_wt as f64 {
                        e.location_x = (buffer_wt - radius - 1) as f64;
                        e.vel_x = -e.vel_x;
                    }
                    if e.location_y - r < 0.0 {
                        e.location_y = r;
                        e.vel_y = -e.vel_y;
                    } else if e.location_y + r >= buffer_ht as f64 {
                        e.location_y = (buffer_ht - radius - 1) as f64;
                        e.vel_y = -e.vel_y;
                    }
                }
                EdgeHandling::Wrap => {
                    if e.location_x + r < 0.0 {
                        e.location_x += (buffer_wt + radius * 2) as f64;
                    }
                    if e.location_y + r < 0.0 {
                        e.location_y += (buffer_ht + radius * 2) as f64;
                    }
                    if e.location_x - r > buffer_wt as f64 {
                        e.location_x -= (buffer_wt + radius * 2) as f64;
                    }
                    if e.location_y - r > buffer_ht as f64 {
                        e.location_y -= (buffer_ht + radius * 2) as f64;
                    }
                }
            }
        }
    }

    /// `UpdateEmitterNozzleAngle`.
    fn update_emitter_nozzle_angle(&mut self, wrapper: &mut LfEmitter, index: usize, factor: f64) {
        let e = &mut self.emitters[index];
        match e.nozzle_movement {
            NozzleMovement::Oscillate => {
                e.angle_oscillation_delta = oscillation_speed(e, factor);
                if e.oscillation_clockwise {
                    e.int_angle -= e.angle_oscillation_delta;
                    if e.int_angle < e.oscillate_start_angle {
                        e.oscillation_clockwise = false;
                    }
                } else {
                    e.int_angle += e.angle_oscillation_delta;
                    if e.int_angle > e.oscillate_end_angle {
                        e.oscillation_clockwise = true;
                    }
                }
                wrapper.angle = e.int_angle;
            }
            NozzleMovement::FixedAngle => {
                // An animated emitter points its nozzle behind the movement.
                wrapper.angle = if e.animate { e.int_angle } else { round_curve(&e.nozzle_angle, factor, 360.0) };
            }
            NozzleMovement::SpinClockwise => {
                e.angle_oscillation_delta = oscillation_speed(e, factor);
                e.int_angle -= e.angle_oscillation_delta;
                // Deliberate: this angle only decreases, so the C#'s wrap at 360 never happens.
                if e.int_angle > 360 {
                    e.int_angle -= 360;
                }
                wrapper.angle = e.int_angle;
            }
            NozzleMovement::SpinCounterClockwise => {
                e.angle_oscillation_delta = oscillation_speed(e, factor);
                e.int_angle += e.angle_oscillation_delta;
                if e.int_angle <= 0 {
                    e.int_angle += 360;
                }
                wrapper.angle = e.int_angle;
            }
        }
    }

    /// `UpdateEmitterColor`, with `GetColorArrayItem` and `IncrementColorIndex`.
    fn update_emitter_color(&mut self, wrapper: &mut LfEmitter, index: usize, interval_pos: f64, factor: f64) -> bool {
        let color_count = self.colors.len();
        let e = &mut self.emitters[index];
        let emitter_color = if e.use_color_array {
            // Defend against the user removing colors.
            if e.color_array_index >= color_count {
                e.color_array_index = 0;
            }
            // Indexing an empty list throws.
            let Some(gradient) = self.colors.get(e.color_array_index) else { return false };
            let color = gradient.get_color_at(interval_pos);

            e.frame_color_counter += 1;
            if e.frame_color_counter >= e.frames_per_color {
                e.color_array_index += 1;
                e.frame_color_counter = 0;
            }
            if e.color_array_index >= color_count {
                e.color_array_index = 0;
            }
            color
        } else {
            e.color.get_color_at(interval_pos)
        };

        let mut hsv = Hsv::from_color(emitter_color);
        hsv.set_v(hsv.v * (e.brightness.get_value(factor) / 100.0));
        let color = hsv.to_color();
        wrapper.r = color.r;
        wrapper.g = color.g;
        wrapper.b = color.b;
        true
    }

    /// `ConvertEmittersToEmitterWrappers`, into the reused `wrappers`.
    fn convert_emitters(&mut self, frame: i32, interval_pos: f64, factor: f64, info: &PixelInfo) {
        let mut wrappers = std::mem::take(&mut self.wrappers);
        wrappers.clear();
        for index in 0..self.emitters.len() {
            let mut wrapper = LfEmitter::default();
            self.update_emitter_flow(&mut wrapper, index, frame, factor, info);
            self.update_emitter_position(&mut wrapper, index, interval_pos, factor);
            self.update_emitter_nozzle_angle(&mut wrapper, index, factor);
            if !self.update_emitter_color(&mut wrapper, index, interval_pos, factor) {
                self.failed = true;
            }

            let e = &self.emitters[index];
            wrapper.lifetime = round_curve(&e.lifetime, factor, MAX_EMITTER_LIFE_TIME);
            wrapper.velocity = round_curve(&e.particle_velocity, factor, MAX_EMITTER_PARTICLE_VELOCITY);
            wrapper.source_size = source_size(e, factor);
            wrapper.particle_type = e.particle_type;
            wrappers.push(wrapper);
        }
        self.wrappers = wrappers;
    }

    /// `_liquidFunWrapper.StepWorld(FrameTime / 1000f, _data.MixColors, ConvertEmittersToEmitterWrappers(...))`.
    fn step_world(&mut self, frame: i32, interval_pos: f64, factor: f64, info: &PixelInfo) {
        self.convert_emitters(frame, interval_pos, factor, info);
        if let Some(world) = &mut self.world {
            world.step(self.frame_time as f32 / 1000.0, self.mix_colors, &self.wrappers);
        }
    }

    /// Steps the world for a frame and fetches the particles.
    fn advance_frame(&mut self, frame: i32, info: &PixelInfo) {
        let interval_pos = info.interval_position(frame);
        self.step_world(frame, interval_pos, interval_pos * 100.0, info);
        if let Some(world) = &mut self.world {
            world.particles(&mut self.particles);
        }
    }
}

/// `(int)Math.Round(ScaleCurveToValue(curve.GetValue(x), maximum, 0))`.
#[inline]
fn round_curve(curve: &Curve, x: f64, maximum: f64) -> i32 {
    scale_curve_to_value(curve.get_value(x), maximum, 0.0).round_ties_even() as i32
}

/// The flow from the curve.
#[inline]
fn curve_flow(e: &Emitter, factor: f64) -> i32 {
    round_curve(&e.flow, factor, MAX_EMITTER_FLOW)
}

/// `CalculateEmitterSourceSize`.
#[inline]
fn source_size(e: &Emitter, x: f64) -> i32 {
    round_curve(&e.source_size, x, MAX_EMITTER_SOURCE_SIZE)
}

/// `CalculateEmitterOscillationSpeed`.
#[inline]
fn oscillation_speed(e: &Emitter, x: f64) -> i32 {
    round_curve(&e.oscillation_speed, x, 45.0)
}

/// `GetDespeckleColor`. Deliberate: a transparent neighbour under the threshold still adds to the average, as the white
/// that `Color.Transparent` is.
fn despeckle_color(buf: &PixelFrameBuffer, x: i32, y: i32, despeckle: i32, wi: i32, ht: i32) -> Color {
    let (mut red, mut green, mut blue, mut count, mut blacks) = (0, 0, 0, 0, 0);
    let startx = (x - 1).max(0);
    let starty = (y - 1).max(0);
    let endx = (x + 1).min(wi - 1);
    let endy = (y + 1).min(ht - 1);
    for yy in starty..=endy {
        for xx in startx..=endx {
            if yy != y || xx != x {
                let c = buf.get_color_at(xx, yy);
                if c == Color::TRANSPARENT {
                    blacks += 1;
                    if blacks >= despeckle {
                        return Color::TRANSPARENT;
                    }
                }
                red += c.r as i32;
                green += c.g as i32;
                blue += c.b as i32;
                count += 1;
            }
        }
    }
    if count == 0 {
        return Color::TRANSPARENT;
    }
    Color::from_rgb_i32(red / count, green / count, blue / count)
}

impl PixelEffect for Liquid<'_> {
    /// `SetupRender`.
    fn setup_render(&mut self, info: &PixelInfo) {
        self.frame_time = info.frame_time;
        if self.locations {
            // One pixel per particle looks better on a smaller matrix scaled up, because location mode is often sparse.
            // Dividing by zero throws, and a negative size fails when the bitmap is created.
            if self.render_scale_factor <= 0 {
                self.failed = true;
                return;
            }
            let scale = self.render_scale_factor;
            self.buffer_wt = info.buffer_wi() / scale;
            self.buffer_ht = info.buffer_ht() / scale;
            if info.buffer_ht() % scale != 0 {
                self.buffer_ht += 1;
            }
            if info.buffer_wi() % scale != 0 {
                self.buffer_wt += 1;
            }
        } else {
            self.buffer_wt = info.buffer_wi();
            self.buffer_ht = info.buffer_ht();
        }

        // Nothing can be rendered on an empty target. The C# would build a world and never use it.
        if self.buffer_wt <= 0 || self.buffer_ht <= 0 {
            return;
        }

        let first_lifetime = round_curve(&self.emitters[0].lifetime, 0.0, MAX_EMITTER_LIFE_TIME);
        let seed = self.rand.next() as u32;
        self.world = LiquidWorld::new(self.buffer_wt, self.buffer_ht, self.barriers, self.particle_size, first_lifetime, seed);
        if self.world.is_none() {
            self.failed = true;
            return;
        }

        for i in 0..self.emitters.len() {
            if self.emitters[i].animate {
                if self.emitters[i].random_starting_position {
                    self.emitters[i].location_x = self.rand.next_range(0, self.buffer_wt) as f64;
                    self.emitters[i].location_y = self.rand.next_range(0, self.buffer_ht) as f64;
                } else {
                    // Zero is left or bottom, 100 is right or top.
                    self.emitters[i].location_x = (self.emitters[i].animate_x_start as f64 / 100.0) * self.buffer_wt as f64;
                    self.emitters[i].location_y = (self.emitters[i].animate_y_start as f64 / 100.0) * self.buffer_ht as f64;
                }
            }
            self.emitters[i].on_timer = 0;
            self.emitters[i].off_timer = 0;
        }

        self.cache_off_audio_volumes(info);

        // Run the engine ahead of the effect, so that there can be particles on the first frame.
        for _ in 0..self.warm_up_frames {
            if self.cancel.load(Ordering::Relaxed) {
                return;
            }
            self.step_world(0, 0.0, 0.0, info);
        }
    }

    /// `RenderEffect`.
    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        if self.failed {
            return;
        }
        self.advance_frame(frame, info);

        for p in &self.particles {
            buf.set_pixel(p.x, p.y, Color::rgb(p.r, p.g, p.b));
        }

        if self.despeckle_threshold > 0 {
            let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
            for y in 0..ht {
                for x in 0..wi {
                    if buf.get_color_at(x, y) == Color::TRANSPARENT {
                        let color = despeckle_color(buf, x, y, self.despeckle_threshold, wi, ht);
                        buf.set_pixel(x, y, color);
                    }
                }
            }
        }
    }

    /// `RenderEffectByLocation`.
    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        if self.failed {
            return;
        }
        let local_buffer_ht = info.buffer_ht();
        let (ht_offset, wi_offset) = (info.buffer_ht_offset(), info.buffer_wi_offset());
        let scale = self.render_scale_factor;
        let locations = buf.element_locations().to_vec();
        // The bitmap the size of the logical render area, reused between frames.
        let mut bitmap = Graphics::new(self.buffer_wt, self.buffer_ht);

        for frame in 0..num_frames {
            if self.failed || self.cancel.load(Ordering::Relaxed) {
                return;
            }
            buf.current_frame = frame;
            self.advance_frame(frame, info);

            bitmap.clear(Color::EMPTY);
            let pixels = bitmap.bitmap_mut();
            for p in &self.particles {
                if p.x >= 0 && p.x < self.buffer_wt && p.y >= 0 && p.y < self.buffer_ht {
                    pixels.set_pixel(p.x, p.y, Color::rgb(p.r, p.g, p.b));
                }
            }

            // `UpdateFrameBufferForLocationPixel`: flip and offset, then scale down to the render bitmap.
            for l in &locations {
                let y = ((ht_offset - l.y) + (local_buffer_ht - 1 + ht_offset)).abs() - ht_offset;
                let x = l.x - wi_offset;
                let color = pixels.get_pixel(x / scale, y / scale);
                buf.set_pixel(l.x, l.y, color);
            }
        }
    }

    /// `CleanUpRender`.
    fn cleanup_render(&mut self) {
        self.world = None;
    }
}

#[cfg(test)]
mod tests;
