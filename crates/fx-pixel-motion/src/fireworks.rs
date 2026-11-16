//! Port of VixenModules.Effect.Fireworks (Fireworks.cs and RgbFireworks.cs).
//!
//! Fireworks never calls `EnableTargetPositioning` and its `StringOrientation` is a fixed read-only
//! `Vertical`, so it always renders by strings with vertical orientation. It does not override
//! `RenderEffectByLocation`.

use crate::audio::{AudioSettings, AudioVolume};
use crate::generated::fireworks as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed_ctx: Some(param_changed_ctx),
    ported: true,
    ..gen::BASE
};

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    match c.name {
        // The setter calls ActivateMarkCollectionSelections.
        MARK_COLLECTIONS | "FireworksSource" => {
            normalize_mark_selection(p, c.info, p.enum_name("FireworksSource") == "MarkCollection", None, true);
        }
        // The audio setters write through to `_audioUtilities`.
        "Gain" | "AttackTime" | "DecayTime" | "Normalize" | "LowPass" | "LowPassFreq" | "HighPass" | "HighPassFreq" => {
            let value = p.get(c.name).clone();
            p.set(&format!("AudioUtilities{}", c.name), value);
        }
        // Each limit keeps clear of the other.
        "MinVelocity" => keep_below(p, "MinVelocity", "MaxVelocity"),
        "MaxVelocity" => keep_above(p, "MaxVelocity", "MinVelocity"),
        "MinParticles" => keep_below(p, "MinParticles", "MaxParticles"),
        "MaxParticles" => keep_above(p, "MaxParticles", "MinParticles"),
        _ => {}
    }
}

fn keep_below(p: &mut Params, min: &str, max: &str) {
    if p.int(max) <= p.int(min) {
        let v = p.int(max) - 1;
        p.set(min, ParamValue::Int(v));
    }
}

fn keep_above(p: &mut Params, max: &str, min: &str) {
    if p.int(min) > p.int(max) {
        let v = p.int(min) + 1;
        p.set(max, ParamValue::Int(v));
    }
}

/// `UpdateAudioAttributes`, `UpdateRandomVelocityAttribute`, `UpdateColorAttribute`,
/// `UpdateRandomParticlesAttribute` and `UpdateLowHighPassAttributes`.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let source = p.enum_name("FireworksSource");
    let audio = source == "Audio" && t.audio_loaded;
    for name in ["Sensitivity", "Gain", "Normalize", "DecayTime", "AttackTime", "ExplosionSensitivity", "HighPass", "LowPass"] {
        v.set(name, audio);
    }
    // UpdateAudioAttributes ends by calling UpdateLowHighPassAttributes, which wins.
    v.set("LowPassFreq", p.boolean("LowPass"));
    v.set("HighPassFreq", p.boolean("HighPass"));
    v.set("Explosions", source == "None");
    v.set("MarkCollectionId", source == "MarkCollection");

    let random_velocity = p.boolean("RandomVelocity");
    v.set("Velocity", !random_velocity);
    v.set("MaxVelocity", random_velocity);
    v.set("MinVelocity", random_velocity);

    let random_particles = p.boolean("RandomParticles");
    v.set("Particles", !random_particles);
    v.set("MaxParticles", random_particles);
    v.set("MinParticles", random_particles);

    let color_type = p.enum_name("ColorType");
    v.set("ColorGradients", color_type != "RainBow" && color_type != "Random");
}

#[derive(Copy, Clone, PartialEq)]
enum ColorType {
    Standard,
    RainBow,
    Range,
    Palette,
    Random,
}

#[derive(Copy, Clone, PartialEq)]
enum Source {
    None,
    MarkCollection,
    Audio,
}

/// `Spacing`.
const SPACING: i32 = 50;

/// `RgbFireworks`. X/Y/Dx/Dy are floats in C#.
#[derive(Clone, Copy)]
struct Burst {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    active: bool,
    cycles: i32,
    color_location: usize,
    start_period: i32,
    hsv: Hsv,
}

struct Fireworks {
    source: Source,
    color_type: ColorType,
    explosions: i32,
    explosion_sensitivity: i32,
    random_velocity: bool,
    velocity: i32,
    min_velocity: i32,
    max_velocity: i32,
    random_particles: bool,
    particles: i32,
    min_particles: i32,
    max_particles: i32,
    particle_fade: i32,
    sensitivity: i32,
    audio: AudioSettings,
    mark_collection_id: Uuid,
    level_curve: Curve,
    gradients: Vec<ColorGradient>,
    /// The marks' offsets from the effect start, in 50 ms frames, gathered in `begin_group`.
    mark_frames: Vec<i32>,
    audio_volume: Option<AudioVolume>,
    /// The C# would have thrown in `SetupRender`, and a pixel effect that throws has no intents at all.
    threw: bool,

    bursts: Vec<Burst>,
    max_flakes: i32,
    rand: Rand,
    /// Each gradient as HSV at the current frame, computed once per frame.
    frame_hsv: Vec<Hsv>,
    wi: i32,
    ht: i32,
    string_count: i32,
    number_frames: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Fireworks {
        source: match p.enum_name("FireworksSource") {
            "MarkCollection" => Source::MarkCollection,
            "Audio" => Source::Audio,
            _ => Source::None,
        },
        color_type: match p.enum_name("ColorType") {
            "RainBow" => ColorType::RainBow,
            "Range" => ColorType::Range,
            "Palette" => ColorType::Palette,
            "Random" => ColorType::Random,
            _ => ColorType::Standard,
        },
        explosions: p.int("Explosions"),
        explosion_sensitivity: p.int("ExplosionSensitivity"),
        random_velocity: p.boolean("RandomVelocity"),
        velocity: p.int("Velocity"),
        min_velocity: p.int("MinVelocity"),
        max_velocity: p.int("MaxVelocity"),
        random_particles: p.boolean("RandomParticles"),
        particles: p.int("Particles"),
        min_particles: p.int("MinParticles"),
        max_particles: p.int("MaxParticles"),
        particle_fade: p.int("ParticalFade"),
        sensitivity: p.int("Sensitivity"),
        // Not the values with those names. The C# setters write through to `_audioUtilities` but its `ModuleData` setter does
        // not, so an effect loaded from a file analyses its audio with `AudioUtilities`' own defaults, whatever the file
        // says, and only properties set since then count.
        audio: AudioSettings {
            low_pass: p.boolean("AudioUtilitiesLowPass"),
            low_pass_freq: p.int("AudioUtilitiesLowPassFreq") as f64,
            high_pass: p.boolean("AudioUtilitiesHighPass"),
            high_pass_freq: p.int("AudioUtilitiesHighPassFreq") as f64,
            normalize: p.boolean("AudioUtilitiesNormalize"),
            attack_time: p.int("AudioUtilitiesAttackTime"),
            decay_time: p.int("AudioUtilitiesDecayTime"),
            gain: p.int("AudioUtilitiesGain") as f64,
        },
        mark_collection_id: p.guid("MarkCollectionId"),
        level_curve: p.curve("LevelCurve"),
        gradients: p.gradients("ColorGradients"),
        mark_frames: Vec::new(),
        audio_volume: None,
        threw: false,
        bursts: Vec::new(),
        max_flakes: 0,
        rand: Rand::default(),
        frame_hsv: Vec::new(),
        wi: 0,
        ht: 0,
        string_count: 0,
        number_frames: 0,
    };
    // The C# `StringOrientation` getter is always Vertical and TargetPositioning is never enabled.
    let out = render_pixel(ctx, StringOrientation::Vertical, TargetPositioning::Strings, &mut e);
    if e.threw {
        return EffectIntents::new();
    }
    out
}

/// `Rand() % 1000 / 1000.0f`: the int remainder divided as a float.
#[inline]
fn rand_thousandths(rand: &mut Rand) -> f64 {
    ((rand.next() % 1000) as f32 / 1000.0f32) as f64
}

impl Fireworks {
    /// `RgbFireworks.Reset`. Arithmetic is float where the C# is.
    fn reset(rand: &mut Rand, b: &mut Burst, x: i32, y: i32, velocity: i32, hsv: Hsv, start: i32, color_location: usize) {
        const HALF_MAX: i32 = i32::MAX / 2;
        b.x = x as f32;
        b.y = y as f32;
        let vel: f32 = (rand.next() - HALF_MAX) as f32 * velocity as f32 / HALF_MAX as f32;
        let angle: f32 = (2.0 * std::f64::consts::PI * rand.next() as f64 / i32::MAX as f64) as f32;
        b.dx = (vel as f64 * (angle as f64).cos()) as f32;
        b.dy = (vel as f64 * (angle as f64).sin()) as f32;
        b.active = false;
        b.cycles = 0;
        b.hsv = hsv;
        b.color_location = color_location;
        b.start_period = start;
    }

    /// `InitFireworksBuffer`.
    fn init_buffer(&mut self) {
        self.bursts.clear();
        let n = self.max_flakes.max(0) as usize;
        self.bursts.resize(
            n,
            Burst {
                x: 0.0,
                y: 0.0,
                dx: 0.0,
                dy: 0.0,
                active: false,
                cycles: 0,
                color_location: 0,
                // Ensures there is no false firework at pixel 0,0 on Frame 0
                start_period: -1,
                hsv: Hsv::new(0.0, 0.0, 0.0),
            },
        );
    }

    /// `CreateExplosions`, with `explosion` standing for the `_explosion` field.
    fn create_explosions(&mut self, explosion: i32, ii: i32, x75: i32, x25: i32, y75: i32, y25: i32) {
        let mut hsv = Hsv::new(0.0, 0.0, 0.0);
        if self.color_type == ColorType::Random {
            hsv.set_h(rand_thousandths(&mut self.rand));
            hsv.set_s(1.0);
            hsv.set_v(1.0);
        }
        let start = if self.source != Source::None {
            ii
        } else {
            (self.rand.next_double() * self.number_frames as f64) as i32
        };
        let start_x = if (x75 - x25) > 0 { x25 + self.rand.next() % (x75 - x25) } else { 0 };
        let start_y = if (y75 - y25) > 0 { y25 + self.rand.next() % (y75 - y25) } else { 0 };
        let color_location = (self.rand.next() as usize) % self.gradients.len();
        let random_particles =
            if self.random_particles { self.rand.next_range(self.min_particles, self.max_particles) } else { self.particles };
        for i in 0..random_particles {
            let velocity =
                if self.random_velocity { self.rand.next_range(self.min_velocity, self.max_velocity) } else { self.velocity };
            // `_fireworkBursts[_explosion * randomParticles + i]` throws when out of range in C#.
            let idx = (explosion as i64) * (random_particles as i64) + i as i64;
            if idx >= 0 && (idx as usize) < self.bursts.len() {
                let mut b = self.bursts[idx as usize];
                Self::reset(&mut self.rand, &mut b, start_x, start_y, velocity, hsv, start, color_location);
                self.bursts[idx as usize] = b;
            }
        }
    }

    fn random_range(&mut self, num1: f32, num2: f32) -> f32 {
        let (lo, hi) = if num1 < num2 { (num1 as f64, num2 as f64) } else { (num2 as f64, num1 as f64) };
        (self.rand.next_double() * (hi - lo) + lo) as f32
    }

    /// `SetRangeColor`.
    fn set_range_color(&mut self, hsv1: Hsv, hsv2: Hsv) -> Hsv {
        let h = self.random_range(hsv1.h as f32, hsv2.h as f32);
        let s = self.random_range(hsv1.s as f32, hsv2.s as f32);
        Hsv::new(h as f64, s as f64, 1.0)
    }
}

impl PixelEffect for Fireworks {
    fn begin_group(&mut self, _targets: &[NodeId], ctx: &RenderCtx) {
        // `GetAudioSettings` and `SetupMarks` need the render context. The audio loads whatever the source is, and audio
        // that cannot be read throws.
        self.audio_volume = AudioVolume::load(ctx, &self.audio);
        if ctx.audio.is_some() && self.audio_volume.is_none() {
            self.threw = true;
        }
        self.mark_frames.clear();
        match self.source {
            Source::Audio => {}
            Source::MarkCollection => {
                if let Some(mc) = ctx.mark_collection(self.mark_collection_id).or_else(|| ctx.marks.first()) {
                    let start = ctx.start_time;
                    for mark in mc.marks_inclusive_of_time(start, start + ctx.time_span) {
                        self.mark_frames.push(((mark.start_time.total_ms() - start.total_ms()) / 50.0) as i32);
                    }
                }
            }
            Source::None => {}
        }
    }

    fn setup_render(&mut self, info: &PixelInfo) {
        self.wi = info.buffer_wi();
        self.ht = info.buffer_ht();
        self.string_count = info.string_count;
        self.number_frames = info.number_frames();
        self.rand = Rand::default();
        self.bursts.clear();
        self.max_flakes = 0;

        let x25 = (self.wi as f64 * 0.25) as i32;
        let x75 = (self.wi as f64 * 0.75) as i32;
        let y25 = (self.ht as f64 * 0.25) as i32;
        let y75 = (self.ht as f64 * 0.75) as i32;
        let per_explosion = if self.random_particles { self.max_particles } else { self.particles };

        let explosions: Vec<i32> = match self.source {
            Source::Audio => {
                let mut list = Vec::new();
                if let Some(audio) = &self.audio_volume {
                    // Determines the frame number for each explosion from the audio.
                    let count = (info.time_span.total_ms() / SPACING as f64) as i32;
                    let threshold = self.sensitivity as f64 / 10.0;
                    let mut i = 0;
                    while i < count {
                        let current_value = audio.volume_at_time(i.wrapping_mul(SPACING));
                        if current_value > threshold {
                            list.push(i);
                            i += self.explosion_sensitivity;
                        }
                        i += 1;
                    }
                }
                list
            }
            Source::MarkCollection => self.mark_frames.clone(),
            // Standard random explosions all start at frame 0, then pick a random start frame.
            Source::None => vec![0; self.explosions.max(0) as usize],
        };

        self.max_flakes = (explosions.len() as i32).wrapping_mul(per_explosion);
        self.init_buffer();
        // `ColorGradients.Count` is a divisor in `CreateExplosions`, which throws with none.
        if self.gradients.is_empty() && !explosions.is_empty() {
            self.threw = true;
            return;
        }
        for (x, frame) in explosions.iter().enumerate() {
            self.create_explosions(x as i32, *frame, x75, x25, y75, y25);
        }
    }

    fn cleanup_render(&mut self) {
        self.bursts.clear();
        self.audio_volume = None;
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        if self.string_count == 1 {
            return;
        }
        let colorcnt = self.gradients.len();
        if colorcnt == 0 {
            return;
        }
        let pos = info.interval_position(frame);
        let level = self.level_curve.get_value(pos * 100.0) / 100.0;
        let particle_fade = self.particle_fade as f64 * 10.0;
        let (wi, ht) = (self.wi as f32, self.ht as f32);
        let max_flakes = self.max_flakes;

        for b in self.bursts.iter_mut() {
            if b.start_period == frame {
                b.active = true;
            }
            // Active flakes
            if b.active {
                // Update position
                b.x += b.dx;
                // Cycles * Cycles is an int product in C#.
                let cc = b.cycles.wrapping_mul(b.cycles);
                b.y += (-(b.dy as f64) - cc as f64 / 10000000.0) as f32;

                // If this flake run for more than maxCycle, time to switch it off
                b.cycles += 20;
                if b.cycles >= max_flakes {
                    b.active = false;
                    continue;
                }

                // If this flake hit the earth or is out of bounds, time to switch it off
                if b.y >= ht || b.y < 0.0 || b.x < 0.0 || b.x >= wi {
                    b.active = false;
                }
            }
        }

        // Gradients sampled at the frame position, once each.
        self.frame_hsv.clear();
        for g in &self.gradients {
            self.frame_hsv.push(Hsv::from_color(g.get_color_at(pos)));
        }

        for i in 0..self.bursts.len() {
            if !self.bursts[i].active {
                continue;
            }
            let mut v = ((particle_fade - self.bursts[i].cycles as f64) / particle_fade) as f32;
            if v < 0.0 {
                v = 0.0;
            }
            match self.color_type {
                // Random two colors are selected from the list for each Firework.
                ColorType::Range => {
                    let a = self.frame_hsv[(self.rand.next() as usize) % colorcnt];
                    let b = self.frame_hsv[(self.rand.next() as usize) % colorcnt];
                    self.bursts[i].hsv = self.set_range_color(a, b);
                }
                // All colors are used
                ColorType::Palette => {
                    self.bursts[i].hsv = self.frame_hsv[(self.rand.next() as usize) % colorcnt];
                }
                // No user colors are used for the rainbow.
                ColorType::RainBow => {
                    let h = rand_thousandths(&mut self.rand);
                    let b = &mut self.bursts[i];
                    b.hsv.set_h(h);
                    b.hsv.set_s(1.0);
                    b.hsv.set_v(1.0);
                }
                ColorType::Standard => {
                    let loc = self.bursts[i].color_location;
                    if let Some(h) = self.frame_hsv.get(loc) {
                        self.bursts[i].hsv = *h;
                    }
                }
                ColorType::Random => {}
            }
            let mut hsv = self.bursts[i].hsv;
            hsv.set_v(v as f64 * level);
            buf.set_pixel_hsv(self.bursts[i].x as i32, self.bursts[i].y as i32, hsv);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{frame, lit};
    use vixen_effects::testing;

    #[test]
    fn the_audio_settings_need_audio() {
        let mut p = DESCRIPTOR.new_params();
        p.set("FireworksSource", ParamValue::Enum("Audio".to_string()));
        let names: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(!names.contains(&"Sensitivity") && !names.contains(&"Gain"));
    }

    #[test]
    fn the_limits_keep_clear_of_each_other() {
        let t = TargetInfo::detached();
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_param(&mut p, "MaxVelocity", ParamValue::Int(5), &t);
        DESCRIPTOR.set_param(&mut p, "MinVelocity", ParamValue::Int(7), &t);
        assert_eq!(p.int("MinVelocity"), 4);
        DESCRIPTOR.set_param(&mut p, "MaxVelocity", ParamValue::Int(1), &t);
        assert_eq!(p.int("MaxVelocity"), 5);
    }

    #[test]
    fn a_mark_source_selects_a_collection() {
        let marks = [MarkCollection::default()];
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        let targets = [s];
        let t = TargetInfo { marks: &marks, ..vixen_effects::testing::target_info(&tree, &targets) };
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_param(&mut p, "FireworksSource", ParamValue::Enum("MarkCollection".to_string()), &t);
        assert_eq!(p.guid("MarkCollectionId"), marks[0].id);
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn visibility_follows_source_and_randomness() {
        let mut p = DESCRIPTOR.new_params();
        let names = |p: &Params| -> Vec<&'static str> {
            let with_audio = TargetInfo { audio_loaded: true, ..TargetInfo::detached() };
            DESCRIPTOR.visible_params_for(p, &with_audio).into_iter().map(|i| DESCRIPTOR.params[i].name).collect()
        };
        let v = names(&p);
        assert!(v.contains(&"Explosions"));
        assert!(!v.contains(&"Sensitivity"));
        assert!(v.contains(&"MinParticles") && !v.contains(&"Particles"));
        p.set("FireworksSource", ParamValue::Enum("Audio".to_string()));
        p.set("RandomParticles", ParamValue::Bool(false));
        let v = names(&p);
        assert!(!v.contains(&"Explosions"));
        assert!(v.contains(&"Sensitivity") && v.contains(&"Particles"));
        p.set("ColorType", ParamValue::Enum("RainBow".to_string()));
        assert!(!names(&p).contains(&"ColorGradients"));
    }

    #[test]
    fn single_string_renders_nothing() {
        // `if (StringCount == 1) return;`
        let f = frame(&DESCRIPTOR, 1, 30, 2000, 5, |_| {});
        assert_eq!(lit(&f), 0);
    }

    #[test]
    fn standard_bursts_use_the_burst_gradient_colour() {
        // One explosion of red particles: every lit pixel at any frame is pure red.
        let frames = crate::test_util::all_frames(&DESCRIPTOR, 20, 20, 2000, |p| {
            p.set("Explosions", ParamValue::Int(1));
            p.set("ColorGradients", ParamValue::GradientList(vec![ColorGradient::solid(Color::RED)]));
        });
        let mut seen = 0;
        for f in &frames {
            for c in f.iter().flatten().filter(|c| c.r > 0 || c.g > 0 || c.b > 0) {
                assert_eq!((c.g, c.b), (0, 0));
                seen += 1;
            }
        }
        assert!(seen > 0);
    }

    #[test]
    fn reset_velocity_is_bounded() {
        let mut rand = Rand::default();
        let mut b = Burst {
            x: 0.0,
            y: 0.0,
            dx: 0.0,
            dy: 0.0,
            active: false,
            cycles: 0,
            color_location: 0,
            start_period: 0,
            hsv: Hsv::new(0.0, 0.0, 0.0),
        };
        for _ in 0..100 {
            Fireworks::reset(&mut rand, &mut b, 3, 4, 5, Hsv::new(0.0, 0.0, 0.0), 7, 0);
            assert!(b.dx.abs() <= 5.001 && b.dy.abs() <= 5.001);
            assert_eq!((b.x, b.y, b.start_period), (3.0, 4.0, 7));
        }
    }
}
