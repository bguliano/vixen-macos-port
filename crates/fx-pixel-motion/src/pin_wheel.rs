//! Port of VixenModules.Effect.PinWheel.

use crate::generated::pin_wheel as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `UpdateColorAttribute`, `UpdateStringOrientationAttributes` and `UpdateOffsetAttribute`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let color_type = p.enum_name("ColorType");
    v.set("Colors", color_type != "Rainbow" && color_type != "Random");
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    let offset_percentage = p.boolean("OffsetPercentage");
    v.set("OffsetPercentage", !offset_percentage);
    v.set("SizeScaleBasis", offset_percentage);
    v.set("TargetPositioning", true);
}

#[derive(Copy, Clone, PartialEq)]
enum ColorType {
    Standard,
    Gradient,
    Random,
    Rainbow,
}

#[derive(Copy, Clone, PartialEq)]
enum BladeType {
    Flat,
    ThreeD,
    Inverted3D,
    Fan,
}

#[derive(Copy, Clone, PartialEq)]
enum ScaleBasis {
    Height,
    Width,
    LargestDimension,
}

struct PinWheel {
    iterations: bool,
    arms: i32,
    color_type: ColorType,
    blade_type: BladeType,
    offset_percentage: bool,
    size_scale_basis: ScaleBasis,
    speed_curve: Curve,
    thickness_curve: Curve,
    size_curve: Curve,
    twist_curve: Curve,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    center_hub_curve: Curve,
    rotation_curve: Curve,
    level_curve: Curve,
    colors: Vec<GradientLevelPair>,
    rand: Rand,
    /// `_newColors`, for the Random colour type.
    new_colors: Vec<ColorGradient>,
    /// Reused across frames: each arm's angle and colour.
    arm_list: Vec<(i32, Hsv)>,
    frame_time: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // A modulo by the empty list's count throws in the C#, and a pixel effect that throws has no intents at all.
    if p.gradient_levels("Colors").is_empty() {
        return EffectIntents::new();
    }
    let mut e = PinWheel {
        iterations: p.enum_name("MovementType") == "Iterations",
        arms: p.int("Arms"),
        color_type: match p.enum_name("ColorType") {
            "Gradient" => ColorType::Gradient,
            "Random" => ColorType::Random,
            "Rainbow" => ColorType::Rainbow,
            _ => ColorType::Standard,
        },
        blade_type: match p.enum_name("PinWheelBladeType") {
            "ThreeD" => BladeType::ThreeD,
            "Inverted3D" => BladeType::Inverted3D,
            "Fan" => BladeType::Fan,
            _ => BladeType::Flat,
        },
        offset_percentage: p.boolean("OffsetPercentage"),
        size_scale_basis: match p.enum_name("SizeScaleBasis") {
            "Width" => ScaleBasis::Width,
            "LargestDimension" => ScaleBasis::LargestDimension,
            _ => ScaleBasis::Height,
        },
        speed_curve: p.curve("SpeedCurve"),
        thickness_curve: p.curve("ThicknessCurve"),
        size_curve: p.curve("SizeCurve"),
        twist_curve: p.curve("TwistCurve"),
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        center_hub_curve: p.curve("CenterHubCurve"),
        rotation_curve: p.curve("RotationCurve"),
        level_curve: p.curve("LevelCurve"),
        colors: p.gradient_levels("Colors"),
        rand: Rand::default(),
        new_colors: Vec::new(),
        arm_list: Vec::new(),
        frame_time: 50,
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// Everything `RenderEffect` computes once per frame before the pixel loop.
struct FrameState {
    current_twist: f64,
    rotation: f64,
    origin: (i32, i32),
    max_radius: f64,
    angle_range: f64,
    overall_level: f64,
    center_start_pct: f64,
}

impl PinWheel {
    fn calculate_x_offset(&self, pos: f64, info: &PixelInfo) -> i32 {
        let wi = info.buffer_wi() as f64;
        if self.offset_percentage {
            scale_curve_to_value(self.x_offset_curve.get_value(pos), wi, -wi).round_ties_even() as i32
        } else {
            scale_curve_to_value(self.x_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32
        }
    }

    fn calculate_y_offset(&self, pos: f64, info: &PixelInfo) -> i32 {
        let ht = info.buffer_ht() as f64;
        if self.offset_percentage {
            scale_curve_to_value(self.y_offset_curve.get_value(pos), -ht, ht).round_ties_even() as i32
        } else {
            scale_curve_to_value(self.y_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32
        }
    }

    fn calculate_radius_basis(&self, origin: (i32, i32), info: &PixelInfo) -> f64 {
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        if !self.offset_percentage {
            return distance_from_point(origin.0, origin.1, info.buffer_wi_offset() + wi, info.buffer_ht_offset() + ht);
        }
        match self.size_scale_basis {
            ScaleBasis::Height => ht as f64,
            ScaleBasis::Width => wi as f64,
            ScaleBasis::LargestDimension => wi.max(ht) as f64,
        }
    }

    fn calculate_speed(&self, pos: f64) -> f64 {
        let value = scale_curve_to_value(self.speed_curve.get_value(pos), 50.0, 1.0) * self.frame_time as f64 / 50.0;
        value.max(1.0)
    }

    fn calculate_size(&self, pos: f64) -> f64 {
        let value = scale_curve_to_value(self.size_curve.get_value(pos), 400.0, 1.0);
        value.max(1.0)
    }

    fn calculate_twist(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.twist_curve.get_value(pos), 500.0, -500.0)
    }

    fn calculate_rotation(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.rotation_curve.get_value(pos), 1.0, -1.0)
    }

    fn calculate_thickness(&self, pos: f64) -> f64 {
        let value = self.thickness_curve.get_int_value(pos);
        value.max(1) as f64
    }

    fn calculate_center_start_pct(&self, pos: f64) -> f64 {
        let mut value = self.center_hub_curve.get_value(pos);
        if value < 0.0 {
            value = 0.0;
        }
        value / 100.0
    }

    /// The frame preamble shared by both render paths. Also fills `arm_list`.
    fn begin_frame(&mut self, frame: i32, info: &PixelInfo) -> FrameState {
        let degrees_per_arm = if self.arms > 0 { 360 / self.arms } else { 1 };
        let interval_pos = info.interval_position(frame);
        let factor = interval_pos * 100.0;
        let pos = if self.iterations {
            interval_pos * self.calculate_speed(factor) * 360.0
        } else {
            interval_pos * self.calculate_speed(factor) * info.number_frames() as f64
        };
        let overall_level = self.level_curve.get_value(factor) / 100.0;
        let current_twist = self.calculate_twist(factor);
        let rotation = self.calculate_rotation(factor);
        self.create_arms(degrees_per_arm, pos, rotation, frame, overall_level, info);
        let armsize = (self.calculate_size(factor) / 100.0) as f32;
        let origin = (
            info.buffer_wi() / 2 + info.buffer_wi_offset() + self.calculate_x_offset(factor, info),
            info.buffer_ht() / 2 + info.buffer_ht_offset() + self.calculate_y_offset(factor, info),
        );
        let max_radius = self.calculate_radius_basis(origin, info) * armsize as f64;
        let angle_range = self.calculate_thickness(factor) / 100.0 * degrees_per_arm as f64 / 2.0;
        let center_start_pct = self.calculate_center_start_pct(factor);
        FrameState { current_twist, rotation, origin, max_radius, angle_range, overall_level, center_start_pct }
    }

    fn create_arms(&mut self, degrees_per_arm: i32, pos: f64, rotation: f64, frame: i32, overall_level: f64, info: &PixelInfo) {
        self.arm_list.clear();
        let colorcnt = self.colors.len();
        // `(a - 1) % Colors.Count` throws in C# with no colours, which leaves the effect dark.
        if colorcnt == 0 {
            return;
        }
        let pos_pct = info.interval_position(frame) * 100.0;
        for a in 1..=self.arms {
            let arm_angle = add_degrees(((a + 1) * degrees_per_arm) as f64, -pos * rotation) as i32;
            let color_idx = ((a - 1) as usize) % colorcnt;
            let mut hsv = self.get_color(color_idx, frame, info);
            hsv.set_v(hsv.v * self.colors[color_idx].curve.get_value(pos_pct) / 100.0);
            hsv.set_v(hsv.v * overall_level);
            self.arm_list.push((arm_angle, hsv));
        }
    }

    fn get_color(&mut self, color_idx: usize, frame: i32, info: &PixelInfo) -> Hsv {
        let mut hsv = Hsv::new(0.0, 0.0, 0.0);
        let pos = (info.interval_position(frame) * 100.0) / 100.0;
        match self.color_type {
            ColorType::Rainbow => {
                // No user colors are used for the rainbow.
                hsv.set_h(self.rand.next_double());
                hsv.set_s(1.0);
                hsv.set_v(1.0);
            }
            ColorType::Random => {
                if let Some(g) = self.new_colors.get(color_idx) {
                    hsv = Hsv::from_color(g.get_color_at(pos));
                }
            }
            ColorType::Standard => {
                hsv = Hsv::from_color(self.colors[color_idx].color_gradient.get_color_at(pos));
            }
            ColorType::Gradient => {}
        }
        hsv
    }

    #[allow(clippy::too_many_arguments)]
    fn render_point<B: PixelBuffer>(&self, buf: &mut B, s: &FrameState, x: i32, y: i32, invert_y: bool, ht: i32) {
        let radius = distance_from_point(s.origin.0, s.origin.1, x, y);
        if radius > s.max_radius || radius <= (s.center_start_pct * s.max_radius) {
            return;
        }
        let angle = angle_degree(s.origin.0, s.origin.1, x, y);
        let degrees_twist = radius / s.max_radius * s.current_twist;
        for (i, arm) in self.arm_list.iter().enumerate() {
            let degrees = add_degrees(arm.0 as f64, degrees_twist);
            if is_angle_between(add_degrees(degrees, -s.angle_range), add_degrees(degrees, s.angle_range), angle) {
                let mut hsv;
                if self.color_type == ColorType::Gradient {
                    let pair = &self.colors[i % self.colors.len()];
                    hsv = Hsv::from_color(pair.color_gradient.get_color_at(radius / s.max_radius));
                    hsv.set_v(hsv.v * pair.curve.get_value(radius / s.max_radius * 100.0) / 100.0);
                    hsv.set_v(hsv.v * s.overall_level);
                } else {
                    hsv = arm.1;
                }
                match self.blade_type {
                    BladeType::ThreeD => {
                        hsv.set_v(hsv.v * (1.0 - (degrees_difference(degrees, angle) / s.angle_range)));
                    }
                    BladeType::Inverted3D => {
                        hsv.set_v(hsv.v * (degrees_difference(degrees, angle) / s.angle_range));
                    }
                    BladeType::Fan => {
                        let front_angle = if s.rotation >= 0.0 { s.angle_range } else { -s.angle_range };
                        hsv.set_v(
                            hsv.v * (degrees_difference(add_degrees(degrees, front_angle), angle) / (2.0 * s.angle_range)),
                        );
                    }
                    BladeType::Flat => {}
                }
                if invert_y {
                    buf.set_pixel_hsv(x, ht - 1 - y, hsv);
                } else {
                    buf.set_pixel_hsv(x, y, hsv);
                }
            }
        }
    }
}

impl PixelEffect for PinWheel {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.frame_time = info.frame_time;
        self.rand = Rand::default();
        self.new_colors.clear();
        if self.color_type == ColorType::Random {
            for _ in 0..self.arms.max(0) {
                let h = self.rand.next_double();
                self.new_colors.push(ColorGradient::solid(Hsv::new(h, 1.0, 1.0).to_color()));
            }
        }
    }

    fn cleanup_render(&mut self) {
        self.new_colors.clear();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let s = self.begin_frame(frame, info);
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        for x in 0..wi {
            for y in 0..ht {
                self.render_point(buf, &s, x, y, true, ht);
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let locations = buf.element_locations().to_vec();
        let ht = info.buffer_ht();
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let s = self.begin_frame(frame, info);
            for l in &locations {
                self.render_point(buf, &s, l.x, l.y, false, ht);
            }
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
    fn offset_visibility_swaps_basis() {
        let mut p = DESCRIPTOR.new_params();
        p.set("OffsetPercentage", ParamValue::Bool(false));
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(!visible.contains(&"SizeScaleBasis"));
        p.set("ColorType", ParamValue::Enum("Rainbow".to_string()));
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(!visible.contains(&"Colors"));
    }

    #[test]
    fn hub_is_dark_and_arms_are_in_range() {
        // 4 arms of thickness 100 cover every angle. Size curve 10 gives a radius of about 8.6 pixels
        // (21 * (1 + 0.1 * 399) / 100) and the hub excludes the inner half of it.
        let f = frame(&DESCRIPTOR, 21, 21, 1000, 0, |p| {
            p.set("Arms", ParamValue::Int(4));
            p.set("ThicknessCurve", ParamValue::Curve(Curve::flat(100.0)));
            p.set("CenterHubCurve", ParamValue::Curve(Curve::flat(50.0)));
            p.set("SizeCurve", ParamValue::Curve(Curve::flat(10.0)));
            p.set("TwistCurve", ParamValue::Curve(Curve::flat(50.0)));
            p.set("Colors", ParamValue::GradientLevels(vec![GradientLevelPair::solid(Color::WHITE)]));
        });
        // The origin is (10, 10): inside the hub (radius <= 4.29) is dark, 6 away is lit, 14 away is dark.
        assert_eq!(f[10][10], RgbValue::BLACK);
        assert_eq!(f[10][14], RgbValue::BLACK);
        assert_eq!(f[10][16], RgbValue { r: 255, g: 255, b: 255 });
        assert_eq!(f[16][10], RgbValue { r: 255, g: 255, b: 255 });
        assert_eq!(f[0][0], RgbValue::BLACK);
        assert!(lit(&f) > 50);
    }
}
