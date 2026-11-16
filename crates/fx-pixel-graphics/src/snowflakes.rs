//! Port of VixenModules.Effect.Snowflakes. A particle system of flakes in several shapes, with build-up, fade,
//! wobble and grid modes. The C# keeps state in fields across frames; so do we.

use crate::generated::snowflakes as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    ported: true,
    ..gen::BASE
};

/// `SnowflakeEffect` as the C# enum's ordinal: None, RandomDirection, Explode, Grid, GridOffset.
fn effect_ord(name: &str) -> i32 {
    match name {
        "RandomDirection" => 1,
        "Explode" => 2,
        "Grid" => 3,
        "GridOffset" => 4,
        _ => 0,
    }
}

/// `SnowFlakeMovement` ordinal: None, Bounce, Wrap, Speed, Wobble, Wobble2.
fn movement_ord(name: &str) -> i32 {
    match name {
        "Bounce" => 1,
        "Wrap" => 2,
        "Speed" => 3,
        "Wobble" => 4,
        "Wobble2" => 5,
        _ => 0,
    }
}

const MV_NONE: i32 = 0;
const MV_BOUNCE: i32 = 1;
const MV_WRAP: i32 = 2;
const MV_SPEED: i32 = 3;
const MV_WOBBLE: i32 = 4;
const MV_WOBBLE2: i32 = 5;

const EFF_NONE: i32 = 0;
const EFF_RANDOM: i32 = 1;
const EFF_EXPLODE: i32 = 2;
const EFF_GRID: i32 = 3;
const EFF_GRID_OFFSET: i32 = 4;

/// `UpdateAttributes` and the `Update*Attribute` methods.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let eff = effect_ord(p.enum_name("SnowflakeEffect"));
    let mv = movement_ord(p.enum_name("SnowFlakeMovement"));
    let fade = p.enum_name("FadeType");
    let color_type = p.enum_name("ColorType");
    let build_up = p.boolean("SnowBuildUp");

    // UpdateColorAttribute
    v.set("InnerColor", color_type != "RainBow");
    v.set("OutSideColor", color_type != "RainBow");
    // UpdateFadeAttribute
    v.set("FadeSpeed", fade != "None");
    v.set("CycleColor", fade != "None" && color_type != "Alternate");
    // UpdateDirectionAttribute
    let (direction, variable_direction) = match eff {
        EFF_EXPLODE => (true, false),
        EFF_RANDOM => (true, true),
        _ => (false, false),
    };
    let _ = direction;
    v.set("MinDirection", variable_direction && eff <= 2);
    v.set("MaxDirection", variable_direction && eff <= 2);
    v.set("SnowBuildUp", eff != EFF_EXPLODE && mv == MV_NONE && eff <= 2);
    v.set("WobbleCurve", mv >= MV_WOBBLE && eff <= 2);
    v.set("WobbleVariationCurve", mv >= MV_WOBBLE && eff <= 2);
    let speed_ctl = mv == MV_SPEED || eff > 2;
    v.set("XCenterSpeedCurve", speed_ctl);
    v.set("YCenterSpeedCurve", speed_ctl);
    v.set("XSpeedVariationCurve", speed_ctl);
    v.set("YSpeedVariationCurve", speed_ctl);
    v.set("HFlakeCount", eff > 2);
    v.set("VFlakeCount", eff > 2);
    v.set("FlakeCountCurve", eff <= 2);
    v.set("SnowFlakeMovement", eff <= 2);
    v.set("CenterSpeedCurve", eff <= 2);
    v.set("SpeedVariationCurve", eff <= 2);
    // UpdateFlakeAttribute
    v.set("PointFlake45", p.enum_name("SnowflakeType") == "Random");
    // UpdateFlakeBuildUpAttribute
    v.set("InitialBuildUp", build_up && eff <= 2);
    v.set("BuildUpSpeedCurve", build_up && eff <= 2);
}

/// Setter side effects: Explode and any movement other than None turn SnowBuildUp off.
fn param_changed(name: &str, p: &mut Params) {
    if name == "SnowflakeEffect" || name == "SnowFlakeMovement" {
        let eff = effect_ord(p.enum_name("SnowflakeEffect"));
        let mv = movement_ord(p.enum_name("SnowFlakeMovement"));
        if (name == "SnowflakeEffect" && eff == EFF_EXPLODE) || mv != MV_NONE {
            p.set("SnowBuildUp", ParamValue::Bool(false));
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum FlakeType {
    Random,
    Single,
    Three,
    Five,
    Nine,
    Thirteen,
    ThirteenV2,
    Seventeen,
    TwentyNine,
    FortyFive,
}

const FLAKE_TYPES: [FlakeType; 10] = [
    FlakeType::Random,
    FlakeType::Single,
    FlakeType::Three,
    FlakeType::Five,
    FlakeType::Nine,
    FlakeType::Thirteen,
    FlakeType::ThirteenV2,
    FlakeType::Seventeen,
    FlakeType::TwentyNine,
    FlakeType::FortyFive,
];

#[derive(Copy, Clone, PartialEq)]
enum Fade {
    None,
    In,
    Out,
    InOut,
}

fn fade_from(name: &str) -> Fade {
    match name {
        "In" => Fade::In,
        "Out" => Fade::Out,
        "InOut" => Fade::InOut,
        _ => Fade::None,
    }
}

#[derive(Copy, Clone, PartialEq)]
enum ColorKind {
    Range,
    Palette,
    Rainbow,
    Alternate,
}

#[derive(Clone)]
struct Flake {
    x: i32,
    y: i32,
    delta_x: f64,
    delta_y: f64,
    delta_x_orig: f64,
    delta_y_orig: f64,
    outer: Hsv,
    inner: Hsv,
    expired: bool,
    kind: FlakeType,
    outer_brightness: f64,
    build_up: bool,
    build_up_x: i32,
    build_up_y: i32,
    x_speed: f64,
    y_speed: f64,
    wobble_x: f64,
    wobble_y: f64,
    wobble: f64,
    width: i32,
    fade: Fade,
    direction: bool,
}

impl Flake {
    fn new() -> Flake {
        Flake {
            x: 0,
            y: 0,
            delta_x: 0.0,
            delta_y: 0.0,
            delta_x_orig: 0.0,
            delta_y_orig: 0.0,
            outer: Hsv { h: 0.0, s: 0.0, v: 0.0 },
            inner: Hsv { h: 0.0, s: 0.0, v: 0.0 },
            expired: false,
            kind: FlakeType::Random,
            outer_brightness: 0.0,
            build_up: false,
            build_up_x: 0,
            build_up_y: 0,
            x_speed: 0.0,
            y_speed: 0.0,
            wobble_x: 0.0,
            wobble_y: 0.0,
            wobble: 0.0,
            width: 1,
            fade: Fade::None,
            direction: false,
        }
    }
}

struct Snowflakes {
    flake_type: FlakeType,
    point_flake_45: bool,
    effect: i32,
    movement: i32,
    fade_type: Fade,
    color_type: ColorKind,
    cycle_color: bool,
    snow_build_up: bool,
    initial_build_up: i32,
    min_direction: i32,
    max_direction: i32,
    h_flake_count: i32,
    v_flake_count: i32,
    random_brightness: bool,
    build_up_speed_curve: Curve,
    fade_speed: Curve,
    center_speed_curve: Curve,
    speed_variation_curve: Curve,
    flake_count_curve: Curve,
    wobble_curve: Curve,
    wobble_variation_curve: Curve,
    x_center_speed_curve: Curve,
    x_speed_variation_curve: Curve,
    y_center_speed_curve: Curve,
    y_speed_variation_curve: Curve,
    level_curve: Curve,
    inner_color: Vec<ColorGradient>,
    outside_color: Vec<ColorGradient>,
    // State
    flakes: Vec<Flake>,
    increase_flake_count: i32,
    flake_count_adjust: i32,
    x_speed_adjustment: f64,
    y_speed_adjustment: f64,
    wobble_adjustment: i32,
    max_buffer_size: i32,
    x_position: f64,
    y_position: f64,
    offset: bool,
    rand: Rand,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Snowflakes {
        flake_type: match p.enum_name("SnowflakeType") {
            "Single" => FlakeType::Single,
            "Three" => FlakeType::Three,
            "Five" => FlakeType::Five,
            "Nine" => FlakeType::Nine,
            "Thirteen" => FlakeType::Thirteen,
            "ThirteenV2" => FlakeType::ThirteenV2,
            "Seventeen" => FlakeType::Seventeen,
            "TwentyNine" => FlakeType::TwentyNine,
            "FortyFive" => FlakeType::FortyFive,
            _ => FlakeType::Random,
        },
        point_flake_45: p.boolean("PointFlake45"),
        effect: effect_ord(p.enum_name("SnowflakeEffect")),
        movement: movement_ord(p.enum_name("SnowFlakeMovement")),
        fade_type: fade_from(p.enum_name("FadeType")),
        color_type: match p.enum_name("ColorType") {
            "Range" => ColorKind::Range,
            "RainBow" => ColorKind::Rainbow,
            "Alternate" => ColorKind::Alternate,
            _ => ColorKind::Palette,
        },
        cycle_color: p.boolean("CycleColor"),
        snow_build_up: p.boolean("SnowBuildUp"),
        initial_build_up: p.int("InitialBuildUp"),
        min_direction: p.int("MinDirection"),
        max_direction: p.int("MaxDirection"),
        h_flake_count: p.int("HFlakeCount"),
        v_flake_count: p.int("VFlakeCount"),
        random_brightness: p.boolean("RandomBrightness"),
        build_up_speed_curve: p.curve("BuildUpSpeedCurve"),
        fade_speed: p.curve("FadeSpeed"),
        center_speed_curve: p.curve("CenterSpeedCurve"),
        speed_variation_curve: p.curve("SpeedVariationCurve"),
        flake_count_curve: p.curve("FlakeCountCurve"),
        wobble_curve: p.curve("WobbleCurve"),
        wobble_variation_curve: p.curve("WobbleVariationCurve"),
        x_center_speed_curve: p.curve("XCenterSpeedCurve"),
        x_speed_variation_curve: p.curve("XSpeedVariationCurve"),
        y_center_speed_curve: p.curve("YCenterSpeedCurve"),
        y_speed_variation_curve: p.curve("YSpeedVariationCurve"),
        level_curve: p.curve("LevelCurve"),
        inner_color: p.gradients("InnerColor"),
        outside_color: p.gradients("OutSideColor"),
        flakes: Vec::new(),
        increase_flake_count: 0,
        flake_count_adjust: 0,
        x_speed_adjustment: 1.0,
        y_speed_adjustment: 1.0,
        wobble_adjustment: 1,
        max_buffer_size: 0,
        x_position: 0.0,
        y_position: 0.0,
        offset: false,
        rand: Rand::default(),
    };
    render_pixel_effect(ctx, p, &mut e)
}

impl Snowflakes {
    fn calculate_count(&self, pos: f64) -> f64 {
        let mut v = scale_curve_to_value(self.flake_count_curve.get_value(pos), 100.0, 1.0);
        if v < 1.0 {
            v = 1.0;
        }
        v
    }

    fn calculate_speed_variation(&self, pos: f64, frame_time: i32) -> f64 {
        let mut v = scale_curve_to_value(self.speed_variation_curve.get_value(pos), 60.0, 1.0) * frame_time as f64 / 50.0;
        if v < 1.0 {
            v = 1.0;
        }
        v
    }

    fn calculate_center_speed(&self, pos: f64, frame_time: i32) -> f64 {
        let mut v = scale_curve_to_value(self.center_speed_curve.get_value(pos), 60.0, 1.0) * frame_time as f64 / 50.0;
        if v < 1.0 {
            v = 1.0;
        }
        v
    }

    fn set_range_color(&mut self, h1: Hsv, h2: Hsv) -> Hsv {
        let h = self.random_range(h1.h as f32, h2.h as f32);
        let s = self.random_range(h1.s as f32, h2.s as f32);
        Hsv::new(h as f64, s as f64, 1.0)
    }

    fn random_range(&mut self, a: f32, b: f32) -> f32 {
        let (lo, hi) = if a < b { (a as f64, b as f64) } else { (b as f64, a as f64) };
        (self.rand.next_double() * (hi - lo) + lo) as f32
    }

    fn random_flake_type(&mut self) -> FlakeType {
        loop {
            let t = FLAKE_TYPES[self.rand.next_range(0, FLAKE_TYPES.len() as i32) as usize];
            if !self.point_flake_45 && t == FlakeType::FortyFive {
                continue;
            }
            return t;
        }
    }

    fn color_from(list: &[ColorGradient], idx: usize, pos: f64) -> Hsv {
        Hsv::from_color(list[idx].get_color_at(pos / 100.0))
    }

    /// `UpdateSnowFlakes`.
    fn update_flakes(&mut self, fade_speed: f64, cnt_out: usize, cnt_in: usize, pos: f64) {
        let mut i = 0usize;
        for k in 0..self.flakes.len() {
            if self.flakes[k].outer_brightness + fade_speed >= 1.0 {
                if self.fade_type == Fade::InOut {
                    self.flakes[k].fade = if self.flakes[k].fade == Fade::In { Fade::Out } else { Fade::In };
                }
                self.flakes[k].outer_brightness = -0.7;
                if self.cycle_color && self.flakes[k].fade == Fade::In {
                    match self.color_type {
                        ColorKind::Range => {
                            let a = Self::color_from(&self.outside_color, self.rand.next() as usize % cnt_out, pos);
                            let b = Self::color_from(&self.outside_color, self.rand.next() as usize % cnt_out, pos);
                            self.flakes[k].outer = self.set_range_color(a, b);
                            let a = Self::color_from(&self.inner_color, self.rand.next() as usize % cnt_in, pos);
                            let b = Self::color_from(&self.inner_color, self.rand.next() as usize % cnt_in, pos);
                            self.flakes[k].inner = self.set_range_color(a, b);
                        }
                        ColorKind::Palette => {
                            self.flakes[k].outer = Self::color_from(&self.outside_color, self.rand.next() as usize % cnt_out, pos);
                            self.flakes[k].inner = Self::color_from(&self.inner_color, self.rand.next() as usize % cnt_in, pos);
                        }
                        ColorKind::Alternate => {
                            self.flakes[k].outer = Self::color_from(&self.outside_color, i % cnt_out, pos);
                            self.flakes[k].inner = Self::color_from(&self.inner_color, i % cnt_in, pos);
                        }
                        ColorKind::Rainbow => {
                            self.flakes[k].inner = Self::color_from(&self.inner_color, self.rand.next() as usize % cnt_in, pos);
                        }
                    }
                }
                // Deliberate quirk: the C# only advances `i` on this branch.
                i += 1;
            } else {
                self.flakes[k].outer_brightness += fade_speed;
            }
        }
    }

    fn draw_flake(kind: FlakeType, direction: bool, cx: i32, cy: i32, c: Color, buf: &mut PixelFrameBuffer) {
        let mut sp = |dx: i32, dy: i32| buf.set_pixel(cx + dx, cy + dy, c);
        match kind {
            FlakeType::Three => {
                if direction {
                    sp(-1, 0);
                    sp(1, 0);
                } else {
                    sp(0, -1);
                    sp(0, 1);
                }
            }
            FlakeType::Five => {
                sp(-1, 0);
                sp(1, 0);
                sp(0, -1);
                sp(0, 1);
            }
            FlakeType::Nine => {
                for j in 1..=2 {
                    sp(-j, 0);
                    sp(j, 0);
                    sp(0, -j);
                    sp(0, j);
                }
            }
            FlakeType::Thirteen => {
                sp(-1, 0);
                sp(1, 0);
                sp(0, -1);
                sp(0, 1);
                sp(-1, 2);
                sp(1, 2);
                sp(-1, -2);
                sp(1, -2);
                sp(2, -1);
                sp(2, 1);
                sp(-2, -1);
                sp(-2, 1);
            }
            FlakeType::ThirteenV2 => {
                for j in 1..=3 {
                    sp(-j, 0);
                    sp(j, 0);
                    sp(0, -j);
                    sp(0, j);
                }
            }
            FlakeType::Seventeen => {
                for j in 1..=3 {
                    sp(-j, 0);
                    sp(j, 0);
                    sp(0, -j);
                    sp(0, j);
                }
                sp(-1, -1);
                sp(1, 1);
                sp(-1, 1);
                sp(1, -1);
            }
            FlakeType::TwentyNine => {
                for j in -1..2 {
                    for jj in -2..3 {
                        sp(j, jj);
                    }
                }
                sp(-2, 1);
                sp(-2, 0);
                sp(-2, -1);
                sp(2, 1);
                sp(2, 0);
                sp(2, -1);
                sp(-3, 0);
                sp(-4, 0);
                sp(3, 0);
                sp(4, 0);
                sp(0, -3);
                sp(0, -4);
                sp(0, 3);
                sp(0, 4);
            }
            FlakeType::FortyFive => {
                let mut ii = 4;
                for j in -4..5 {
                    sp(j, ii);
                    ii -= 1;
                }
                for j in -4..5 {
                    sp(j, j);
                }
                sp(-2, 3);
                sp(-3, 2);
                sp(-3, -2);
                sp(-2, -3);
                sp(2, 3);
                sp(2, -3);
                sp(3, 2);
                sp(3, -2);
                for j in -5..6 {
                    sp(0, j);
                }
                for j in -5..6 {
                    sp(j, 0);
                }
            }
            FlakeType::Random | FlakeType::Single => {}
        }
    }
}

impl PixelEffect for Snowflakes {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.increase_flake_count = 0;
        self.flake_count_adjust = 0;
        self.flakes = Vec::new();
        self.x_speed_adjustment = 1.0;
        self.y_speed_adjustment = 1.0;
        self.wobble_adjustment = 1;
        self.max_buffer_size = (info.buffer_ht() / 2).max(info.buffer_wi() / 2);
    }

    fn cleanup_render(&mut self) {
        self.flakes = Vec::new();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let cnt_out = self.outside_color.len();
        let cnt_in = self.inner_color.len();
        // The C# indexes these lists with `% count`, which throws for an empty list and leaves the effect dark.
        if cnt_out == 0 || cnt_in == 0 {
            return;
        }
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        let frame_time = info.frame_time;
        let mut min_direction = 1;
        let mut max_direction = 360;
        let pos = info.interval_position(frame) * 100.0;
        let level = self.level_curve.get_value(pos) / 100.0;
        let count_now = self.calculate_count(pos);

        let flake_count = if self.effect < 3 {
            (if self.effect == EFF_EXPLODE && (frame as f64) < count_now { 1.0 } else { count_now }) as i32
        } else {
            self.v_flake_count.wrapping_mul(self.h_flake_count)
        };

        let initial_build_up = if !self.snow_build_up {
            0
        } else {
            (scale_curve_to_value(self.initial_build_up as f64, ht as f64, 0.0)
                + scale_curve_to_value(self.build_up_speed_curve.get_value(pos), 80.0, 1.0) * (frame as f64 / 100.0)) as i32
        };

        let center_speed = self.calculate_center_speed(pos, frame_time);
        let spread_speed = self.calculate_speed_variation(pos, frame_time);
        let mut fade_speed = scale_curve_to_value(self.fade_speed.get_value(pos), 0.1, 0.0) * frame_time as f64 / 50.0;
        let mut min_speed = center_speed - spread_speed / 2.0;
        let mut max_speed = center_speed + spread_speed / 2.0;
        if min_speed < 1.0 {
            min_speed = 1.0;
        }
        if max_speed > 60.0 {
            max_speed = 60.0;
        }

        let mut x_speed_ratio = 1.0;
        let mut y_speed_ratio = 1.0;
        let (mut min_x_speed, mut max_x_speed) = (1.0, 1.0);
        let (mut min_y_speed, mut max_y_speed) = (1.0, 1.0);
        let (mut min_wobble, mut max_wobble) = (1, 1);
        let mut wobble_ratio = 1.0;
        let x_grid_spacing = (wi as f64 / self.h_flake_count as f64 - 1.0) + 1.0;
        let y_grid_spacing = (ht as f64 / self.v_flake_count as f64 - 1.0) + 1.0;
        self.y_position = y_grid_spacing / 2.0;
        self.x_position = x_grid_spacing / 2.0;

        if self.movement == MV_SPEED || self.effect > 2 {
            // Horizontal speed control
            let x_center = scale_curve_to_value(self.x_center_speed_curve.get_value(pos), 5.0, -5.0);
            let x_spread = scale_curve_to_value(self.x_speed_variation_curve.get_value(pos), 10.0, 0.0);
            min_x_speed = x_center - x_spread / 2.0;
            max_x_speed = x_center + x_spread / 2.0;
            if min_x_speed < -100.0 {
                min_x_speed = -100.0;
            }
            if max_x_speed > 100.0 {
                max_x_speed = 100.0;
            }
            if x_center != 0.0 {
                if frame != 0 {
                    x_speed_ratio = x_center / self.x_speed_adjustment;
                }
                self.x_speed_adjustment = x_center;
            }
            // Vertical speed control
            let y_center = scale_curve_to_value(self.y_center_speed_curve.get_value(pos), 5.0, -5.0);
            let y_spread = scale_curve_to_value(self.y_speed_variation_curve.get_value(pos), 10.0, 0.0);
            min_y_speed = y_center - y_spread / 2.0;
            max_y_speed = y_center + y_spread / 2.0;
            if min_y_speed < -100.0 {
                min_y_speed = -100.0;
            }
            if max_y_speed > 100.0 {
                max_y_speed = 100.0;
            }
            if y_center != 0.0 {
                if frame != 0 {
                    y_speed_ratio = y_center / self.y_speed_adjustment;
                }
                self.y_speed_adjustment = y_center;
            }
        } else if self.movement == MV_WOBBLE || self.movement == MV_WOBBLE2 {
            let m = self.max_buffer_size as f64;
            let center = scale_curve_to_value(self.wobble_curve.get_value(pos), m, -m).round_ties_even();
            let spread = scale_curve_to_value(self.wobble_variation_curve.get_value(pos), m, 0.0).round_ties_even() as i32 as f64;
            min_wobble = (center - spread / 2.0) as i32;
            max_wobble = (center + spread / 2.0) as i32;
            if min_wobble < -self.max_buffer_size {
                min_wobble = -self.max_buffer_size;
            }
            if max_wobble > self.max_buffer_size {
                max_wobble = self.max_buffer_size;
            }
            if center != 0.0 {
                if frame != 0 {
                    wobble_ratio = center / self.wobble_adjustment as f64;
                }
                self.wobble_adjustment = center as i32;
            }
        }

        let grid = self.effect == EFF_GRID || self.effect == EFF_GRID_OFFSET;
        for i in 0..flake_count {
            let mut m = Flake::new();
            m.kind = if self.flake_type == FlakeType::Random { self.random_flake_type() } else { self.flake_type };
            m.width = match m.kind {
                FlakeType::Five | FlakeType::Nine => 2,
                FlakeType::Thirteen | FlakeType::ThirteenV2 | FlakeType::Seventeen => 3,
                FlakeType::FortyFive | FlakeType::TwentyNine => 5,
                _ => 1,
            };

            if !grid {
                if self.flakes.len() as f64 >= count_now + self.increase_flake_count as f64 - self.flake_count_adjust as f64 {
                    break;
                }
                let position = (self.rand.next_double() * ((max_speed + 1.0) - min_speed) + min_speed) / 5.0;
                if self.effect == EFF_RANDOM {
                    min_direction = self.min_direction;
                    max_direction = self.max_direction;
                }

                let direction = if self.effect == EFF_NONE {
                    // A range that keeps standard flakes from falling straight down.
                    self.rand.next_range(145, 215)
                } else if max_direction <= min_direction {
                    // A maximum below the minimum inverts the range, turning a downward motion upward.
                    if self.rand.next_range(1, 3) == 1 {
                        self.rand.next_range(1, max_direction)
                    } else {
                        self.rand.next_range(min_direction, 360)
                    }
                } else {
                    self.rand.next_range(min_direction, max_direction)
                };

                let d = direction as f64;
                if direction > 0 && direction <= 90 {
                    m.delta_x = (d / 90.0) * position;
                    m.delta_y = ((direction - 90).abs() as f64 / 90.0) * position;
                    m.wobble_x = (direction - 90).abs() as f64 / 90.0;
                    m.wobble_y = -1.0 * (direction.abs() as f64 / 90.0);
                    if self.rand.next_double() >= (90 - direction) as f64 / 100.0 {
                        m.x = 0;
                        m.y = self.rand.next() % ht;
                    } else {
                        m.x = self.rand.next() % wi;
                        m.y = 0;
                    }
                } else if direction > 90 && direction <= 180 {
                    m.delta_x = ((direction - 180).abs() as f64 / 90.0) * position;
                    m.delta_y = (-1.0 * ((direction - 90).abs() as f64 / 90.0)) * position;
                    m.wobble_x = -1.0 * ((direction - 90).abs() as f64 / 90.0);
                    m.wobble_y = -1.0 * ((direction - 180).abs() as f64 / 90.0);
                    if self.rand.next_double() >= (180 - direction) as f64 / 100.0 {
                        m.x = self.rand.next() % wi;
                        m.y = ht;
                    } else {
                        m.x = 0;
                        m.y = self.rand.next() % ht;
                    }
                } else if direction > 180 && direction <= 270 {
                    m.delta_x = (-1.0 * ((direction - 180).abs() as f64 / 90.0)) * position;
                    m.delta_y = (-1.0 * ((direction - 270).abs() as f64 / 90.0)) * position;
                    m.wobble_x = (direction - 90).abs() as f64 / 90.0;
                    m.wobble_y = -1.0 * ((direction - 180).abs() as f64 / 90.0);
                    if self.rand.next_double() >= (270 - direction) as f64 / 100.0 {
                        m.x = wi;
                        m.y = self.rand.next() % ht;
                    } else {
                        m.x = self.rand.next() % wi;
                        m.y = ht;
                    }
                } else if direction > 270 && direction <= 360 {
                    m.delta_x = (-1.0 * ((direction - 360).abs() as f64 / 90.0)) * position;
                    m.delta_y = ((270 - direction).abs() as f64 / 90.0) * position;
                    m.wobble_x = -1.0 * ((direction - 270).abs() as f64 / 90.0);
                    m.wobble_y = -1.0 * ((direction - 360).abs() as f64 / 90.0);
                    if self.rand.next_double() >= (360 - direction) as f64 / 100.0 {
                        m.x = self.rand.next() % wi;
                        m.y = 0;
                    } else {
                        m.x = wi;
                        m.y = self.rand.next() % ht;
                    }
                }

                if self.effect == EFF_EXPLODE {
                    // Starts in the centre of the grid.
                    m.x = wi / 2;
                    m.y = ht / 2;
                }

                // The C# assigns the start position twice on frame 0; the second assignment wins.
                if frame == 0 {
                    m.x = self.rand.next() % wi - 1;
                    m.y = self.rand.next_range(0, ht - 1);
                }
                if frame == 0 {
                    m.x = self.rand.next_range(0, wi);
                    m.y = self.rand.next_range(0, ht);
                }

                m.delta_x_orig = m.delta_x;
                m.delta_y_orig = m.delta_y;

                m.wobble = self.rand.next_range(min_wobble, max_wobble) as f64;
                if self.rand.next_range(0, 2) == 1 && self.movement == MV_WOBBLE2 {
                    m.wobble = -m.wobble;
                }
            } else {
                if frame != 0 {
                    break;
                }
                // Grid location of the flakes for Grid and Grid Offset.
                let grid_offset = if self.offset {
                    if self.effect == EFF_GRID_OFFSET {
                        y_grid_spacing / 2.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                m.x = self.x_position as i32;
                m.y = ((self.y_position as i32) as f64 + grid_offset) as i32;

                self.x_position += x_grid_spacing;
                if self.h_flake_count != 0 && (i + 1) % self.h_flake_count == 0 {
                    self.y_position += y_grid_spacing;
                    self.x_position = x_grid_spacing / 2.0;
                    if self.h_flake_count % 2 != 0 {
                        self.offset = !self.offset;
                    }
                }
                self.offset = !self.offset;
            }

            if self.movement == MV_SPEED || self.effect > 2 {
                m.x_speed = self.rand.next_double() * (max_x_speed - min_x_speed) + min_x_speed;
                m.y_speed = self.rand.next_double() * (max_y_speed - min_y_speed) + min_y_speed;
            }

            // Colours are fixed when the flake is created.
            let idx_in = |r: &mut Rand| r.next() as usize % cnt_in;
            let idx_out = |r: &mut Rand| r.next() as usize % cnt_out;
            match self.color_type {
                ColorKind::Range => {
                    let a = Self::color_from(&self.outside_color, idx_out(&mut self.rand), pos);
                    let b = Self::color_from(&self.outside_color, idx_out(&mut self.rand), pos);
                    m.outer = self.set_range_color(a, b);
                    let a = Self::color_from(&self.inner_color, idx_in(&mut self.rand), pos);
                    let b = Self::color_from(&self.inner_color, idx_in(&mut self.rand), pos);
                    m.inner = self.set_range_color(a, b);
                }
                ColorKind::Palette => {
                    m.outer = Self::color_from(&self.outside_color, idx_out(&mut self.rand), pos);
                    m.inner = Self::color_from(&self.inner_color, idx_in(&mut self.rand), pos);
                }
                ColorKind::Alternate if self.effect <= 2 => {
                    m.outer = Self::color_from(&self.outside_color, idx_out(&mut self.rand), pos);
                    m.inner = Self::color_from(&self.inner_color, idx_in(&mut self.rand), pos);
                }
                ColorKind::Alternate => {
                    m.outer = Self::color_from(&self.outside_color, i as usize % cnt_out, pos);
                    m.inner = Self::color_from(&self.inner_color, i as usize % cnt_in, pos);
                }
                ColorKind::Rainbow => {
                    m.inner = Self::color_from(&self.inner_color, idx_in(&mut self.rand), pos);
                }
            }

            m.outer_brightness = if self.random_brightness { self.rand.next_double() * (1.0 - 0.20) } else { 1.0 };
            m.build_up = false;
            m.direction = self.rand.next_range(0, 2) == 0;

            // Deliberate quirk: only flakes created on frame 0 get a fade type; later ones stay at None.
            if frame == 0 {
                m.fade = match self.fade_type {
                    Fade::None => Fade::None,
                    Fade::Out => Fade::Out,
                    Fade::In => Fade::In,
                    Fade::InOut => {
                        if self.rand.next_range(0, 2) == 0 {
                            Fade::In
                        } else {
                            Fade::Out
                        }
                    }
                };
            }

            self.flakes.push(m);
        }

        if self.fade_type == Fade::InOut {
            fade_speed *= 2.0;
        }
        if self.fade_type != Fade::None && frame != 0 {
            self.update_flakes(fade_speed, cnt_out, cnt_in, pos);
        }

        if self.snow_build_up {
            // The snow on the ground uses the first outside colour.
            let mut col = self.outside_color[0].get_color_at(pos / 100.0);
            if level < 1.0 {
                let mut hsv = Hsv::from_color(col);
                hsv.set_v(hsv.v * level);
                col = hsv.to_color();
            }
            for x in 0..wi {
                for y in 0..initial_build_up {
                    buf.set_pixel(x, y, col);
                }
            }
        }

        // Render all flakes.
        for fi in 0..self.flakes.len() {
            let (mut x_off, mut y_off) = (0, 0);
            if self.movement >= MV_WOBBLE {
                x_off = (self.flakes[fi].wobble_x * self.flakes[fi].wobble) as i32;
                y_off = (self.flakes[fi].wobble_y * self.flakes[fi].wobble) as i32;
            }
            {
                let f = &mut self.flakes[fi];
                f.delta_x += f.delta_x_orig + f.x_speed;
                f.delta_y += f.delta_y_orig + f.y_speed;
            }
            let overshoot = self.flakes[fi].width * 2;

            let (color_x, color_y);
            if !self.flakes[fi].build_up {
                let f = &self.flakes[fi];
                let mut cx = x_off.wrapping_add(f.x).wrapping_add(f.delta_x as i32);
                let mut cy = y_off.wrapping_add(f.y).wrapping_add(f.delta_y as i32);

                if self.effect != EFF_EXPLODE {
                    if self.movement >= MV_WOBBLE {
                        if cx < 0 {
                            cx += wi;
                        }
                        if cy < 0 {
                            cy += ht;
                        }
                        if cx > wi {
                            cx -= wi;
                        }
                        if cy > ht {
                            cy -= ht;
                        }
                    }

                    if self.snow_build_up {
                        // A flake landing on one already at the bottom stacks on it (up to 3 pixels wide).
                        let hit = self
                            .flakes
                            .iter()
                            .find(|o| o.build_up && cx >= o.build_up_x - 1 && cx <= o.build_up_x + 1 && cy <= o.build_up_y)
                            .map(|o| o.build_up_y);
                        if let Some(by) = hit {
                            let inner = Self::color_from(&self.inner_color, 0, pos);
                            let outer = Self::color_from(&self.outside_color, 0, pos);
                            let f = &mut self.flakes[fi];
                            f.build_up = true;
                            f.build_up_x = cx;
                            f.inner = inner;
                            f.outer = outer;
                            f.build_up_y = if f.width < 3 { by + 1 } else { by + 2 };
                            self.increase_flake_count += 1;
                        }
                    }
                }

                if self.color_type == ColorKind::Rainbow && !self.flakes[fi].build_up {
                    let r = self.rand.next() % 1000;
                    let f = &mut self.flakes[fi];
                    f.outer.set_h((r as f32 / 1000.0f32) as f64);
                    f.outer.set_s(1.0);
                    f.outer.set_v(1.0);
                }

                if cx >= wi + overshoot || cy >= ht + overshoot || cx <= -overshoot || cy <= initial_build_up - overshoot {
                    // Flakes past the edge expire, or settle when build-up is on.
                    if self.snow_build_up && cy <= initial_build_up && !self.flakes[fi].build_up && self.effect != EFF_EXPLODE {
                        let inner = Self::color_from(&self.inner_color, 0, pos);
                        let outer = Self::color_from(&self.outside_color, 0, pos);
                        let f = &mut self.flakes[fi];
                        f.build_up = true;
                        f.build_up_x = cx;
                        f.build_up_y = initial_build_up + 1;
                        f.inner = inner;
                        f.outer = outer;
                        self.increase_flake_count += 1;
                    } else if !self.flakes[fi].build_up {
                        self.flakes[fi].expired = true;
                    }
                }
                color_x = cx;
                color_y = cy;
            } else {
                color_x = self.flakes[fi].build_up_x;
                color_y = self.flakes[fi].build_up_y;
            }

            // Colour and brightness from effect position, random brightness and level.
            let f = &self.flakes[fi];
            let mut hsv_inner = f.inner;
            let mut hsv_outer = f.outer;
            hsv_inner.set_v(
                hsv_inner.v
                    * if f.outer_brightness > -0.5 && f.outer_brightness < -0.2 && f.fade == Fade::In { 0.0 } else { level },
            );
            let inner_color = hsv_inner.to_color();
            match f.fade {
                Fade::In | Fade::None => hsv_outer.set_v(hsv_outer.v * (f.outer_brightness * level)),
                Fade::Out => hsv_outer.set_v(hsv_outer.v * ((1.0 - f.outer_brightness) * level)),
                Fade::InOut => {}
            }
            let outer_color = hsv_outer.to_color();

            if initial_build_up < ht && color_y >= initial_build_up - overshoot {
                if f.build_up {
                    // A flat flake on the ground, as wide as the flake.
                    for y in 0..=(color_y - initial_build_up) {
                        for x in (-y - f.width)..=(y + f.width) {
                            buf.set_pixel(color_x + x, color_y - y, outer_color);
                        }
                    }
                } else {
                    Self::draw_flake(f.kind, f.direction, color_x, color_y, outer_color, buf);
                    buf.set_pixel(color_x, color_y, inner_color);
                }
            }

            {
                let f = &mut self.flakes[fi];
                f.x_speed *= x_speed_ratio;
                f.y_speed *= y_speed_ratio;
                f.wobble *= wobble_ratio;
            }

            let expired = self.flakes[fi].expired;
            let wrap_block = match self.movement {
                MV_SPEED | MV_WOBBLE | MV_WOBBLE2 | MV_WRAP => expired,
                _ => self.effect > 2,
            };
            let f = &mut self.flakes[fi];
            if wrap_block {
                if color_x < 0 {
                    f.delta_x = 0.0;
                    f.x = wi;
                }
                if color_y < 0 {
                    f.delta_y = 0.0;
                    f.y = ht;
                }
                if color_x > wi {
                    f.delta_x = 0.0;
                    f.x = 0;
                }
                if color_y > ht {
                    f.delta_y = 0.0;
                    f.y = 0;
                }
                f.expired = false;
            } else if self.movement == MV_BOUNCE {
                if color_x < 0 {
                    f.x = 0;
                    f.delta_x = 0.0;
                    f.delta_x_orig = -f.delta_x_orig;
                    f.x_speed = -f.x_speed;
                }
                if color_y < 0 {
                    f.y = 0;
                    f.delta_y = 0.0;
                    f.delta_y_orig = -f.delta_y_orig;
                    f.y_speed = -f.y_speed;
                }
                if color_x >= wi {
                    f.x = wi;
                    f.delta_x = 0.0;
                    f.delta_x_orig = -f.delta_x_orig;
                    f.x_speed = -f.x_speed;
                }
                if color_y >= ht {
                    f.y = ht;
                    f.delta_y = 0.0;
                    f.delta_y_orig = -f.delta_y_orig;
                    f.y_speed = -f.y_speed;
                }
                f.expired = false;
            }
        }

        if self.movement == MV_NONE {
            // Expired flakes are removed so new ones can be created.
            self.flakes.retain(|f| !f.expired);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn explode_turns_build_up_off() {
        let mut p = DESCRIPTOR.new_params();
        p.set("SnowBuildUp", ParamValue::Bool(true));
        p.set("SnowflakeEffect", ParamValue::Enum("Explode".to_string()));
        param_changed("SnowflakeEffect", &mut p);
        assert!(!p.boolean("SnowBuildUp"));
    }

    #[test]
    fn visibility_follows_effect() {
        let mut p = DESCRIPTOR.new_params();
        p.set("SnowflakeEffect", ParamValue::Enum("Grid".to_string()));
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(vis.contains(&"HFlakeCount"));
        assert!(!vis.contains(&"FlakeCountCurve"));
        let mut p = DESCRIPTOR.new_params();
        p.set("SnowflakeEffect", ParamValue::Enum("RandomDirection".to_string()));
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(vis.contains(&"MinDirection"));
    }

    #[test]
    fn grid_flakes_sit_on_a_grid_with_inner_colour() {
        // Grid of 2x2 flakes of the 1 point type on an 8x8 matrix: flakes at (2,2), (6,2), (2,6), (6,6)
        // (spacing 4, start 2) with a blue centre, standing still because their speeds are zero.
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 8, 8);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 100, |p| {
            p.set("SnowflakeEffect", ParamValue::Enum("Grid".into()));
            p.set("SnowflakeType", ParamValue::Enum("Single".into()));
            p.set("HFlakeCount", ParamValue::Int(2));
            p.set("VFlakeCount", ParamValue::Int(2));
            p.set("XCenterSpeedCurve", ParamValue::Curve(Curve::from_points(vec![(0.0, 50.0), (100.0, 50.0)])));
            p.set("YCenterSpeedCurve", ParamValue::Curve(Curve::from_points(vec![(0.0, 50.0), (100.0, 50.0)])));
        });
        // Vertical orientation: string = x, pixel = y.
        assert_eq!(color_at(&tree, m, &out, 2, 2, 0), (0, 0, 255));
        assert_eq!(color_at(&tree, m, &out, 6, 6, 0), (0, 0, 255));
        assert_eq!(lit_count(&tree, m, &out, 0), 4);
    }
}
