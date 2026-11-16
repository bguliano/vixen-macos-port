//! Port of VixenModules.Effect.Meteors.
//!
//! Meteors never calls `EnableTargetPositioning` and does not override `RenderEffectByLocation`, so it
//! renders by strings only.

use crate::generated::meteors as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    ported: true,
    ..gen::BASE
};

/// `UpdateColorAttribute`, `UpdateDirectionAttribute` and `UpdateGroundLevelAttribute`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    v.set("Colors", p.enum_name("ColorType") != "RainBow");

    let effect = p.enum_name("MeteorEffect");
    let mut direction = false;
    let mut variable_direction = false;
    if effect == "Explode" {
        direction = true;
    }
    if effect == "RandomDirection" {
        direction = true;
        variable_direction = true;
    }
    let per_string = p.boolean("MeteorPerString");
    let movement = Movement::from_name(p.enum_name("MeteorMovement"));
    v.set("Direction", !direction && !per_string);
    v.set("MinDirection", variable_direction && !per_string);
    v.set("MaxDirection", variable_direction && !per_string);
    v.set("FlipDirection", per_string);
    v.set("CountPerString", per_string);
    v.set("WobbleCurve", movement >= Movement::Wobble);
    v.set("WobbleVariationCurve", movement >= Movement::Wobble);
    v.set("XCenterSpeedCurve", movement == Movement::Speed);
    v.set("YCenterSpeedCurve", movement == Movement::Speed);
    v.set("XSpeedVariationCurve", movement == Movement::Speed);
    v.set("YSpeedVariationCurve", movement == Movement::Speed);
    v.set("EnableGroundLevel", movement == Movement::None);

    let ground = p.boolean("EnableGroundLevel");
    v.set("GroundLevelCurve", ground);
    v.set("GroundColor", ground);
}

/// The setters that call `UpdateDirectionAttribute` end with
/// `if (MeteorMovement != MeteorMovement.None) EnableGroundLevel = false;`.
fn param_changed(name: &str, p: &mut Params) {
    if matches!(name, "MeteorEffect" | "MeteorMovement" | "MeteorPerString" | "FlipDirection" | "CountPerString")
        && Movement::from_name(p.enum_name("MeteorMovement")) != Movement::None
    {
        p.set("EnableGroundLevel", ParamValue::Bool(false));
    }
}

/// `MeteorMovement`, in declaration order because the C# compares with `>=`.
#[derive(Copy, Clone, PartialEq, PartialOrd)]
enum Movement {
    None,
    Bounce,
    Wrap,
    Speed,
    Wobble,
    Wobble2,
}

impl Movement {
    fn from_name(n: &str) -> Movement {
        match n {
            "Bounce" => Movement::Bounce,
            "Wrap" => Movement::Wrap,
            "Speed" => Movement::Speed,
            "Wobble" => Movement::Wobble,
            "Wobble2" => Movement::Wobble2,
            _ => Movement::None,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Effect {
    None,
    RandomDirection,
    Explode,
}

#[derive(Copy, Clone, PartialEq)]
enum StartPosition {
    Random,
    InitiallyRandom,
    ZeroPosition,
}

#[derive(Copy, Clone, PartialEq)]
enum ColorType {
    RainBow,
    Range,
    Palette,
    Gradient,
}

/// `MeteorClass`.
#[derive(Clone, Copy)]
struct Meteor {
    x: i32,
    y: i32,
    delta_x: f64,
    delta_y: f64,
    delta_x_orig: f64,
    delta_y_orig: f64,
    tail_x: f64,
    tail_y: f64,
    hsv: Hsv,
    expired: bool,
    color: usize,
    hsv_brightness: f64,
    x_speed: f64,
    y_speed: f64,
    wobble_x: f64,
    wobble_y: f64,
    wobble: f64,
}

impl Meteor {
    fn new() -> Meteor {
        Meteor {
            x: 0,
            y: 0,
            delta_x: 0.0,
            delta_y: 0.0,
            delta_x_orig: 0.0,
            delta_y_orig: 0.0,
            tail_x: 0.0,
            tail_y: 0.0,
            hsv: Hsv::new(0.0, 0.0, 0.0),
            expired: false,
            color: 0,
            hsv_brightness: 0.0,
            x_speed: 0.0,
            y_speed: 0.0,
            wobble_x: 0.0,
            wobble_y: 0.0,
            wobble: 0.0,
        }
    }
}

struct Meteors {
    effect: Effect,
    movement: Movement,
    start_position: StartPosition,
    color_type: ColorType,
    direction: i32,
    min_direction: i32,
    max_direction: i32,
    enable_ground_level: bool,
    meteor_per_string: bool,
    flip_direction: bool,
    count_per_string: bool,
    random_brightness: bool,
    center_speed_curve: Curve,
    speed_variation_curve: Curve,
    pixel_count_curve: Curve,
    length_curve: Curve,
    tail_taper_curve: Curve,
    ground_level_curve: Curve,
    wobble_curve: Curve,
    wobble_variation_curve: Curve,
    x_center_speed_curve: Curve,
    x_speed_variation_curve: Curve,
    y_center_speed_curve: Curve,
    y_speed_variation_curve: Curve,
    level_curve: Curve,
    colors: Vec<ColorGradient>,
    ground_color: ColorGradient,

    meteors: Vec<Meteor>,
    gradient_position: f64,
    temp_buffer: PixelFrameBuffer,
    max_ground_height: i32,
    x_speed_adjustment: f64,
    y_speed_adjustment: f64,
    wobble_adjustment: i32,
    max_buffer_size: i32,
    rand: Rand,
    /// `TailTaperCurve` at each tail position of the current frame.
    taper: Vec<f64>,
    wi: i32,
    ht: i32,
    frame_time: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // RainBow doesn't use the list. Otherwise a modulo by its empty count throws in the C#, and a pixel effect that throws
    // has no intents at all.
    if p.enum_name("ColorType") != "RainBow" && p.gradients("Colors").is_empty() {
        return EffectIntents::new();
    }
    let movement = Movement::from_name(p.enum_name("MeteorMovement"));
    let mut e = Meteors {
        effect: match p.enum_name("MeteorEffect") {
            "RandomDirection" => Effect::RandomDirection,
            "Explode" => Effect::Explode,
            _ => Effect::None,
        },
        movement,
        start_position: match p.enum_name("MeteorStartPosition") {
            "Random" => StartPosition::Random,
            "ZeroPosition" => StartPosition::ZeroPosition,
            _ => StartPosition::InitiallyRandom,
        },
        color_type: match p.enum_name("ColorType") {
            "RainBow" => ColorType::RainBow,
            "Range" => ColorType::Range,
            "Gradient" => ColorType::Gradient,
            _ => ColorType::Palette,
        },
        direction: p.int("Direction"),
        min_direction: p.int("MinDirection"),
        max_direction: p.int("MaxDirection"),
        // Upgrading to a movement other than None switches ground level off in the C# setters.
        enable_ground_level: p.boolean("EnableGroundLevel") && movement == Movement::None,
        meteor_per_string: p.boolean("MeteorPerString"),
        flip_direction: p.boolean("FlipDirection"),
        count_per_string: p.boolean("CountPerString"),
        random_brightness: p.boolean("RandomBrightness"),
        center_speed_curve: p.curve("CenterSpeedCurve"),
        speed_variation_curve: p.curve("SpeedVariationCurve"),
        pixel_count_curve: p.curve("PixelCountCurve"),
        length_curve: p.curve("LengthCurve"),
        tail_taper_curve: p.curve("TailTaperCurve"),
        ground_level_curve: p.curve("GroundLevelCurve"),
        wobble_curve: p.curve("WobbleCurve"),
        wobble_variation_curve: p.curve("WobbleVariationCurve"),
        x_center_speed_curve: p.curve("XCenterSpeedCurve"),
        x_speed_variation_curve: p.curve("XSpeedVariationCurve"),
        y_center_speed_curve: p.curve("YCenterSpeedCurve"),
        y_speed_variation_curve: p.curve("YSpeedVariationCurve"),
        level_curve: p.curve("LevelCurve"),
        colors: p.gradients("Colors"),
        ground_color: p.gradient("GroundColor"),
        meteors: Vec::with_capacity(32),
        gradient_position: 0.0,
        temp_buffer: PixelFrameBuffer::new(0, 0),
        max_ground_height: 0,
        x_speed_adjustment: 1.0,
        y_speed_adjustment: 1.0,
        wobble_adjustment: 1,
        max_buffer_size: 0,
        rand: Rand::default(),
        taper: Vec::new(),
        wi: 0,
        ht: 0,
        frame_time: 50,
    };
    let orientation = StringOrientation::from_name(p.enum_name("StringOrientation"));
    // Meteors never enables location rendering.
    render_pixel(ctx, orientation, TargetPositioning::Strings, &mut e)
}

/// `Rand() % 1000 / 1000.0f`.
#[inline]
fn rand_thousandths(rand: &mut Rand) -> f64 {
    ((rand.next() % 1000) as f32 / 1000.0f32) as f64
}

impl Meteors {
    fn calculate_speed_variation(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.speed_variation_curve.get_value(pos), 200.0, 0.0) * self.frame_time as f64 / 50.0
    }

    fn calculate_center_speed(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.center_speed_curve.get_value(pos), 200.0, 0.0) * self.frame_time as f64 / 50.0
    }

    fn calculate_pixel_count(&self, pos: f64) -> i32 {
        let value = scale_curve_to_value(self.pixel_count_curve.get_value(pos), 200.0, 1.0) as i32;
        value.max(1)
    }

    fn calculate_length(&self, pos: f64) -> i32 {
        let value = scale_curve_to_value(self.length_curve.get_value(pos), 100.0, 1.0) as i32;
        value.max(1)
    }

    fn calculate_ground_level(&self, pos: f64) -> f64 {
        let max_ground_height = if self.effect == Effect::Explode { 0 } else { 6 };
        scale_curve_to_value(self.ground_level_curve.get_value(pos), (self.ht - max_ground_height) as f64, 0.0)
    }

    fn calculate_x_center_speed(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.x_center_speed_curve.get_value(pos), 5.0, -5.0)
    }

    fn calculate_x_speed_variation(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.x_speed_variation_curve.get_value(pos), 10.0, 0.0)
    }

    fn calculate_y_center_speed(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.y_center_speed_curve.get_value(pos), 5.0, -5.0)
    }

    fn calculate_y_speed_variation(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.y_speed_variation_curve.get_value(pos), 10.0, 0.0)
    }

    fn calculate_wobble_center(&self, pos: f64) -> f64 {
        let m = self.max_buffer_size as f64;
        scale_curve_to_value(self.wobble_curve.get_value(pos), m, -m).round_ties_even()
    }

    fn calculate_wobble_variation(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.wobble_variation_curve.get_value(pos), self.max_buffer_size as f64, 0.0).round_ties_even()
            as i32
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

    /// `_tempBuffer.GetColorAt`, which throws out of range in C#; those cases read as empty here.
    fn temp_get(&self, x: i32, y: i32) -> Color {
        if x >= 0 && x < self.temp_buffer.width() && y >= 0 && y < self.temp_buffer.height() {
            self.temp_buffer.get_color_at(x, y)
        } else {
            Color::EMPTY
        }
    }

    /// Creates one meteor, from the inner body of the C# creation loop. Returns `None` for the
    /// `continue` taken when the string already has a meteor.
    #[allow(clippy::too_many_arguments)]
    fn create_meteor(
        &mut self,
        j: i32,
        frame: i32,
        pixel_count: i32,
        min_speed: f64,
        max_speed: f64,
        interval_pos_factor: f64,
        tail_length: i32,
        min_direction_in: i32,
        max_direction_in: i32,
        min_xy: (f64, f64, f64, f64),
        min_wobble: i32,
        max_wobble: i32,
    ) -> Option<Meteor> {
        let (wi, ht) = (self.wi, self.ht);
        let (mut min_direction, mut max_direction) = (min_direction_in, max_direction_in);
        let position = (self.rand.next_double() * (max_speed - min_speed) + min_speed) / 20.0;
        let mut m = Meteor::new();

        if self.effect == Effect::RandomDirection {
            min_direction = self.min_direction;
            max_direction = self.max_direction;
        }

        let direction: i32;
        if self.effect == Effect::None {
            // Set Range for standard Meteor as we don't want to just have them going straight down or two
            // directions like the original Meteor effect.
            direction = self.direction;
        } else if max_direction <= min_direction {
            // If someone makes the MaxDirection lower than the Min Direction the new direction is the
            // inverse of the range: an upward motion.
            direction = if self.rand.next_range(1, 3) == 1 {
                self.rand.next_range(1, max_direction)
            } else {
                self.rand.next_range(min_direction, 360)
            };
        } else {
            direction = self.rand.next_range(min_direction, max_direction);
        }

        // Moving
        if !self.count_per_string {
            m.x = self.rand.next() % wi;
        }
        m.y = self.rand.next() % ht;

        let dir = direction as f64;
        if self.meteor_per_string {
            m.tail_y = if self.flip_direction { 1.0 } else { -1.0 };
            if self.count_per_string {
                if self.meteors.iter().any(|meteor| meteor.x == j) {
                    return None;
                }
                m.x = j;
            }
            m.delta_y = m.tail_y * position;
            m.y = self.rand.next() % ht;
            m.wobble_x = -1.0 * ((direction - 270).abs() as f64 / 90.0);
            m.wobble_y = -1.0 * ((direction - 360).abs() as f64 / 90.0);
        } else if (0..=90).contains(&direction) {
            m.tail_x = dir / 90.0;
            m.delta_x = m.tail_x * position;
            m.tail_y = (direction - 90).abs() as f64 / 90.0;
            m.delta_y = m.tail_y * position;
            if self.rand.next_double() >= (90 - direction) as f64 / 100.0 {
                m.x = 0;
                m.y = self.rand.next() % ht;
            } else {
                m.x = self.rand.next() % wi;
                m.y = 0;
            }
            m.wobble_x = (direction - 90).abs() as f64 / 90.0;
            m.wobble_y = -1.0 * (direction.abs() as f64 / 90.0);
        } else if direction > 90 && direction <= 180 {
            m.tail_x = (direction - 180).abs() as f64 / 90.0;
            m.delta_x = m.tail_x * position;
            m.tail_y = -1.0 * ((direction - 90).abs() as f64 / 90.0);
            m.delta_y = m.tail_y * position;
            if self.rand.next_double() >= (180 - direction) as f64 / 100.0 {
                m.x = self.rand.next() % wi;
                m.y = ht;
            } else {
                m.x = 0;
                m.y = self.rand.next() % ht;
            }
            m.wobble_x = -1.0 * ((direction - 90).abs() as f64 / 90.0);
            m.wobble_y = -1.0 * ((direction - 180).abs() as f64 / 90.0);
        } else if direction > 180 && direction <= 270 {
            m.tail_x = -1.0 * ((direction - 180).abs() as f64 / 90.0);
            m.delta_x = m.tail_x * position;
            m.tail_y = -1.0 * ((direction - 270).abs() as f64 / 90.0);
            m.delta_y = m.tail_y * position;
            if self.rand.next_double() >= (270 - direction) as f64 / 100.0 {
                m.x = wi;
                m.y = self.rand.next() % ht;
            } else {
                m.x = self.rand.next() % wi;
                m.y = ht;
            }
            m.wobble_x = (direction - 90).abs() as f64 / 90.0;
            m.wobble_y = -1.0 * ((direction - 180).abs() as f64 / 90.0);
        } else if direction > 270 && direction <= 360 {
            m.tail_x = -1.0 * ((direction - 360).abs() as f64 / 90.0);
            m.delta_x = m.tail_x * position;
            m.tail_y = (270 - direction).abs() as f64 / 90.0;
            m.delta_y = m.tail_y * position;
            if self.rand.next_double() >= (360 - direction) as f64 / 100.0 {
                m.x = self.rand.next() % wi;
                m.y = 0;
            } else {
                m.x = wi;
                m.y = self.rand.next() % ht;
            }
            m.wobble_x = -1.0 * ((direction - 270).abs() as f64 / 90.0);
            m.wobble_y = -1.0 * ((direction - 360).abs() as f64 / 90.0);
        }

        if self.effect == Effect::Explode {
            m.x = (wi - 1) / 2;
            m.y = (ht - 1) / 2;
        } else if self.start_position == StartPosition::ZeroPosition {
            if self.meteor_per_string {
                m.y = if self.flip_direction { 0 } else { ht };
            }
        } else if self.start_position == StartPosition::Random || frame < pixel_count {
            if !self.meteor_per_string {
                m.x = self.rand.next() % wi - 1;
            }
            m.y = if ht - 1 <= self.max_ground_height { 0 } else { self.rand.next_range(self.max_ground_height, ht - 1) };
        }

        m.delta_x_orig = m.delta_x;
        m.delta_y_orig = m.delta_y;

        if self.movement == Movement::Speed {
            let (min_x, max_x, min_y, max_y) = min_xy;
            m.x_speed = self.rand.next_double() * (max_x - min_x) + min_x;
            m.y_speed = self.rand.next_double() * (max_y - min_y) + min_y;
        }

        m.wobble = self.rand.next_range(min_wobble, max_wobble) as f64;
        if self.rand.next_range(0, 2) == 1 && self.movement == Movement::Wobble2 {
            m.wobble = -m.wobble;
        }

        let colorcnt = self.colors.len();
        let pos = interval_pos_factor / 100.0;
        match self.color_type {
            // Random two colors are selected from the list for each meteor.
            ColorType::Range => {
                let a = Hsv::from_color(self.colors[(self.rand.next() as usize) % colorcnt].get_color_at(pos));
                let b = Hsv::from_color(self.colors[(self.rand.next() as usize) % colorcnt].get_color_at(pos));
                m.hsv = self.set_range_color(a, b);
            }
            // All colors are used
            ColorType::Palette => {
                m.hsv = Hsv::from_color(self.colors[(self.rand.next() as usize) % colorcnt].get_color_at(pos));
            }
            ColorType::Gradient => {
                m.color = (self.rand.next() as usize) % colorcnt;
                self.gradient_position = 100.0 / tail_length as f64 / 100.0;
                m.hsv = Hsv::from_color(self.colors[m.color].get_color_at(0.0));
            }
            ColorType::RainBow => {}
        }

        m.hsv_brightness = if self.random_brightness { self.rand.next_double() * (1.0 - 0.20) + 0.20 } else { 1.0 };
        Some(m)
    }
}

impl PixelEffect for Meteors {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.wi = info.buffer_wi();
        self.ht = info.buffer_ht();
        self.frame_time = info.frame_time;
        self.max_ground_height = 0;
        self.x_speed_adjustment = 1.0;
        self.y_speed_adjustment = 1.0;
        self.wobble_adjustment = 1;
        self.gradient_position = 0.0;
        self.rand = Rand::default();
        self.meteors.clear();

        if self.enable_ground_level {
            self.temp_buffer = PixelFrameBuffer::new(self.wi + 10, self.ht + 10);
            for x in 0..self.wi {
                let mut y = 0;
                while (y as f64) < self.calculate_ground_level((100.0 / self.wi as f64) * x as f64) {
                    self.temp_buffer.set_pixel(x, y, self.ground_color.get_color_at(0.0));
                    let temp = self.calculate_ground_level(y as f64) as i32;
                    if temp > self.max_ground_height {
                        self.max_ground_height = temp;
                    }
                    y += 1;
                }
            }
        }

        self.max_buffer_size = (self.ht / 2).max(self.wi / 2);
    }

    fn cleanup_render(&mut self) {
        self.meteors.clear();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let colorcnt = self.colors.len();
        // Every colour type but RainBow takes `Rand() % Colors.Count`, which throws with no colours.
        if colorcnt == 0 && self.color_type != ColorType::RainBow {
            return;
        }
        let (wi, ht) = (self.wi, self.ht);
        let interval_pos_factor = info.interval_position(frame) * 100.0;
        let level = self.level_curve.get_value(interval_pos_factor) / 100.0;
        let length = self.calculate_length(interval_pos_factor);
        let mut tail_length = if ht < 10 { length / 10 } else { ht * length / 100 };
        let min_direction = 1;
        let max_direction = 360;
        let pixel_count = self.calculate_pixel_count(interval_pos_factor);
        let center_speed = self.calculate_center_speed(interval_pos_factor);
        let spread_speed = self.calculate_speed_variation(interval_pos_factor);
        let mut min_speed = center_speed - (spread_speed / 2.0);
        let mut max_speed = center_speed + (spread_speed / 2.0);
        if min_speed < 0.0 {
            min_speed = 0.0;
        }
        if max_speed > 200.0 {
            max_speed = 200.0;
        }
        if tail_length < 1 {
            tail_length = 1;
        }

        let mut x_speed_ratio = 1.0;
        let mut y_speed_ratio = 1.0;
        let (mut min_x_speed, mut max_x_speed, mut min_y_speed, mut max_y_speed) = (1.0, 1.0, 1.0, 1.0);
        let (mut min_wobble, mut max_wobble) = (1, 1);
        let mut wobble_ratio = 1.0;

        match self.movement {
            Movement::Speed => {
                // Horizontal Speed Control
                let x_center_speed = self.calculate_x_center_speed(interval_pos_factor);
                let x_spread_speed = self.calculate_x_speed_variation(interval_pos_factor);
                min_x_speed = x_center_speed - (x_spread_speed / 2.0);
                max_x_speed = x_center_speed + (x_spread_speed / 2.0);
                if min_x_speed < -100.0 {
                    min_x_speed = -100.0;
                }
                if max_x_speed > 100.0 {
                    max_x_speed = 100.0;
                }
                if x_center_speed != 0.0 {
                    if frame != 0 {
                        x_speed_ratio = x_center_speed / self.x_speed_adjustment;
                    }
                    self.x_speed_adjustment = x_center_speed;
                }

                // Vertical Speed Control
                let y_center_speed = self.calculate_y_center_speed(interval_pos_factor);
                let y_spread_speed = self.calculate_y_speed_variation(interval_pos_factor);
                min_y_speed = y_center_speed - (y_spread_speed / 2.0);
                max_y_speed = y_center_speed + (y_spread_speed / 2.0);
                if min_y_speed < -100.0 {
                    min_y_speed = -100.0;
                }
                if max_y_speed > 100.0 {
                    max_y_speed = 100.0;
                }
                if y_center_speed != 0.0 {
                    if frame != 0 {
                        y_speed_ratio = y_center_speed / self.y_speed_adjustment;
                    }
                    self.y_speed_adjustment = y_center_speed;
                }
            }
            // Wobble Control
            Movement::Wobble | Movement::Wobble2 => {
                let wobble_center_position = self.calculate_wobble_center(interval_pos_factor);
                let wobble_spread_position = self.calculate_wobble_variation(interval_pos_factor) as f64;
                min_wobble = (wobble_center_position - (wobble_spread_position / 2.0)) as i32;
                max_wobble = (wobble_center_position + (wobble_spread_position / 2.0)) as i32;
                if min_wobble < -self.max_buffer_size {
                    min_wobble = -self.max_buffer_size;
                }
                if max_wobble > self.max_buffer_size {
                    max_wobble = self.max_buffer_size;
                }
                if wobble_center_position != 0.0 {
                    if frame != 0 {
                        wobble_ratio = wobble_center_position / self.wobble_adjustment as f64;
                    }
                    self.wobble_adjustment = wobble_center_position as i32;
                }
            }
            _ => {}
        }

        // create new meteors and maintain maximum number as per users selection.
        let adjusted_pixel_count;
        if frame < pixel_count {
            if self.start_position == StartPosition::InitiallyRandom && frame > pixel_count {
                adjusted_pixel_count = 1;
            } else if self.start_position == StartPosition::ZeroPosition {
                adjusted_pixel_count = pixel_count;
            } else {
                adjusted_pixel_count =
                    if pixel_count < 10 || self.movement >= Movement::Wobble { pixel_count } else { pixel_count / 10 };
            }
        } else {
            adjusted_pixel_count = pixel_count;
        }

        let mut string_count = 1;
        if self.count_per_string && self.meteor_per_string {
            string_count = wi;
        }

        for _i in 0..adjusted_pixel_count {
            for j in 0..string_count {
                if self.meteors.len() as i64 >= pixel_count as i64 * string_count as i64 {
                    break;
                }
                if let Some(m) = self.create_meteor(
                    j,
                    frame,
                    pixel_count,
                    min_speed,
                    max_speed,
                    interval_pos_factor,
                    tail_length,
                    min_direction,
                    max_direction,
                    (min_x_speed, max_x_speed, min_y_speed, max_y_speed),
                    min_wobble,
                    max_wobble,
                ) {
                    self.meteors.push(m);
                }
            }
        }

        if self.enable_ground_level {
            let mut hsv = Hsv::from_color(self.ground_color.get_color_at(interval_pos_factor / 100.0));
            hsv.set_v(hsv.v * (self.level_curve.get_value(interval_pos_factor) / 100.0));
            for x in 0..wi {
                let mut y = 0;
                while (y as f64) < self.calculate_ground_level((100.0 / wi as f64) * x as f64) {
                    if self.temp_get(x, y) != Color::EMPTY {
                        buf.set_pixel_hsv(x, y, hsv);
                    }
                    y += 1;
                }
            }
        }

        // The tail taper depends only on the position along the tail.
        self.taper.clear();
        for ph in 0..tail_length {
            self.taper.push(self.tail_taper_curve.get_value(ph as f64 / tail_length as f64 * 100.0) / 100.0);
        }

        // render meteors
        for mi in 0..self.meteors.len() {
            let mut meteor = self.meteors[mi];
            let (mut x_offset_adj, mut y_offset_adj) = (0, 0);
            if self.movement >= Movement::Wobble {
                x_offset_adj = (meteor.wobble_x * meteor.wobble) as i32;
                y_offset_adj = (meteor.wobble_y * meteor.wobble) as i32;
            }

            meteor.delta_x += meteor.delta_x_orig + meteor.x_speed;
            meteor.delta_y += meteor.delta_y_orig + meteor.y_speed;

            let mut color_x = x_offset_adj + meteor.x + (meteor.delta_x as i32) - wi / 100;
            let mut color_y = y_offset_adj + meteor.y + (meteor.delta_y as i32) + ht / 100;

            if self.movement >= Movement::Wobble {
                if color_x < 0 {
                    color_x += wi;
                }
                if color_y < 0 {
                    color_y += ht;
                }
                if color_x > wi {
                    color_x -= wi;
                }
                if color_y > ht {
                    color_y -= ht;
                }
            }

            for ph in 0..tail_length {
                match self.color_type {
                    // No user colors are used for the rainbow.
                    ColorType::RainBow => {
                        meteor.hsv.set_h(rand_thousandths(&mut self.rand));
                        meteor.hsv.set_s(1.0);
                        meteor.hsv.set_v(1.0);
                    }
                    ColorType::Gradient => {
                        meteor.hsv = Hsv::from_color(self.colors[meteor.color].get_color_at(self.gradient_position * ph as f64));
                    }
                    _ => {}
                }
                let mut hsv = meteor.hsv;
                hsv.set_v(hsv.v * (meteor.hsv_brightness * self.taper[ph as usize] * level));

                let dec_places = (meteor.tail_x * ph as f64 % 1.0 * 100.0) as i32;
                if dec_places <= 40 || dec_places >= 60 {
                    let tx = (meteor.tail_x * ph as f64).round_ties_even() as i32;
                    let ty = (meteor.tail_y * ph as f64).round_ties_even() as i32;
                    if self.effect == Effect::Explode
                        && ph > 0
                        && (color_x == (wi / 2) + tx
                            || color_x == (wi / 2) - tx
                            || color_y == (ht / 2) + ty
                            || color_y == (ht / 2) - ty)
                    {
                        break;
                    }
                    buf.set_pixel_hsv(color_x - tx, color_y - ty, hsv);
                }
            }

            if color_x > 0 && color_x < wi - 1 && color_y > 0 && color_y < ht && self.enable_ground_level
                && self.temp_get(color_x, color_y) != Color::EMPTY
            {
                if frame > 1 {
                    // The 22 cells `_tempBuffer.SetPixel(.., Color.Empty)` clears, in the C# order.
                    const HOLES: [(i32, i32); 22] = [
                        (0, 0),
                        (0, -1),
                        (-1, 0),
                        (1, 0),
                        (0, 1),
                        (1, 1),
                        (-1, 1),
                        (1, 2),
                        (-1, 2),
                        (1, 3),
                        (-1, 3),
                        (2, 3),
                        (-2, 3),
                        (2, 2),
                        (-2, 2),
                        (0, 2),
                        (0, 3),
                        (-2, 4),
                        (-1, 4),
                        (0, 4),
                        (1, 4),
                        (2, 4),
                    ];
                    for (dx, dy) in HOLES {
                        self.temp_buffer.set_pixel(color_x + dx, color_y + dy, Color::EMPTY);
                    }
                }
                meteor.expired = true;
            }

            if color_x >= wi + tail_length || color_y >= ht + tail_length || color_x < 0 - tail_length || color_y < 0 - tail_length
            {
                // flags Meteors that have reached the end of the grid as expired.
                meteor.expired = true;
            }

            meteor.x_speed *= x_speed_ratio;
            meteor.y_speed *= y_speed_ratio;
            meteor.wobble *= wobble_ratio;

            if meteor.expired {
                match self.movement {
                    Movement::Speed | Movement::Wobble | Movement::Wobble2 | Movement::Wrap => {
                        if color_x < 0 {
                            meteor.delta_x = 0.0;
                            meteor.x = wi;
                        }
                        if color_y < 0 {
                            meteor.delta_y = 0.0;
                            meteor.y = ht;
                        }
                        if color_x > wi {
                            meteor.delta_x = 0.0;
                            meteor.x = 0;
                        }
                        if color_y > ht {
                            meteor.delta_y = 0.0;
                            meteor.y = 0;
                        }
                        meteor.expired = false;
                    }
                    Movement::Bounce => {
                        if color_x < 0 {
                            meteor.x = 0;
                            meteor.delta_x = 0.0;
                            meteor.delta_x_orig = -meteor.delta_x_orig;
                            meteor.tail_x = -meteor.tail_x;
                            meteor.x_speed = -meteor.x_speed;
                        }
                        if color_y < 0 {
                            meteor.y = 0;
                            meteor.delta_y = 0.0;
                            meteor.delta_y_orig = -meteor.delta_y_orig;
                            meteor.tail_y = -meteor.tail_y;
                            meteor.y_speed = -meteor.y_speed;
                        }
                        if color_x > wi {
                            meteor.x = wi;
                            meteor.delta_x = 0.0;
                            meteor.delta_x_orig = -meteor.delta_x_orig;
                            meteor.tail_x = -meteor.tail_x;
                            meteor.x_speed = -meteor.x_speed;
                        }
                        if color_y > ht {
                            meteor.y = ht;
                            meteor.delta_y = 0.0;
                            meteor.delta_y_orig = -meteor.delta_y_orig;
                            meteor.tail_y = -meteor.tail_y;
                            meteor.y_speed = -meteor.y_speed;
                        }
                        meteor.expired = false;
                    }
                    Movement::None => {}
                }
            }
            self.meteors[mi] = meteor;
        }

        if self.movement == Movement::None {
            // delete old meteors
            self.meteors.retain(|m| !m.expired);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{frame, lit};
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn visibility_follows_movement_and_direction_mode() {
        let names = |p: &Params| -> Vec<&'static str> {
            DESCRIPTOR.visible_params(p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect()
        };
        let mut p = DESCRIPTOR.new_params();
        let v = names(&p);
        assert!(v.contains(&"Direction") && !v.contains(&"MinDirection") && v.contains(&"EnableGroundLevel"));
        assert!(!v.contains(&"WobbleCurve") && !v.contains(&"GroundColor"));
        p.set("MeteorEffect", ParamValue::Enum("RandomDirection".to_string()));
        p.set("MeteorMovement", ParamValue::Enum("Wobble".to_string()));
        let v = names(&p);
        assert!(!v.contains(&"Direction") && v.contains(&"MinDirection") && v.contains(&"WobbleCurve"));
        assert!(!v.contains(&"EnableGroundLevel"));
        p.set("MeteorPerString", ParamValue::Bool(true));
        assert!(names(&p).contains(&"FlipDirection") && !names(&p).contains(&"MinDirection"));
    }

    #[test]
    fn moving_off_none_clears_ground_level() {
        let mut p = DESCRIPTOR.new_params();
        p.set("EnableGroundLevel", ParamValue::Bool(true));
        p.set("MeteorMovement", ParamValue::Enum("Bounce".to_string()));
        param_changed("MeteorMovement", &mut p);
        assert!(!p.boolean("EnableGroundLevel"));
    }

    #[test]
    fn falling_meteors_draw_tails_in_palette_colours() {
        // Straight down (180), solid red, a single string wide strip: lit pixels are red shades only.
        let f = frame(&DESCRIPTOR, 10, 40, 2000, 15, |p| {
            p.set("Colors", ParamValue::GradientList(vec![ColorGradient::solid(Color::RED)]));
        });
        assert!(lit(&f) > 0);
        for c in f.iter().flatten().filter(|c| c.r > 0 || c.g > 0 || c.b > 0) {
            assert_eq!((c.g, c.b), (0, 0));
        }
    }

    #[test]
    fn ground_level_is_drawn_when_enabled() {
        // Ground curve 50% of (ht - 6) = 17 pixels of ground colour at the bottom.
        let f = frame(&DESCRIPTOR, 10, 40, 1000, 0, |p| {
            p.set("EnableGroundLevel", ParamValue::Bool(true));
            p.set("GroundLevelCurve", ParamValue::Curve(Curve::flat(50.0)));
            p.set("GroundColor", ParamValue::Gradient(ColorGradient::solid(Color::rgb(0, 200, 0))));
            p.set("PixelCountCurve", ParamValue::Curve(Curve::flat(0.0)));
        });
        // One meteor may overdraw a column, so require it of most of the ten strings.
        let ground = RgbValue { r: 0, g: 200, b: 0 };
        assert!(f.iter().filter(|s| s[0] == ground).count() >= 8);
        assert!(f.iter().filter(|s| s[16] == ground).count() >= 8);
        assert!(f.iter().filter(|s| s[17] == ground).count() == 0);
    }
}
