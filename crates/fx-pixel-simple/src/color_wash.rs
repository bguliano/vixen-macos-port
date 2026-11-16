//! Port of VixenModules.Effect.ColorWash.

use crate::generated::color_wash as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `UpdateMovementTypeAttribute`, `UpdateStringOrientationAttributes` and `EnableTargetPositioning`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let speed = p.enum_name("MovementType") == "Speed";
    v.set("SpeedCurve", speed);
    v.set("Iterations", !speed);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

#[derive(Copy, Clone, PartialEq)]
enum WashType {
    Center,
    Outer,
    Invert,
}

struct ColorWash {
    wash_type: WashType,
    movement_iterations: bool,
    iterations: i32,
    speed_curve: Curve,
    horizontal_fade: bool,
    vertical_fade: bool,
    shimmer: bool,
    color: ColorGradient,
    level_curve: Curve,
    half_ht: f64,
    half_wi: f64,
    position: f64,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = ColorWash {
        wash_type: match p.enum_name("Type") {
            "Outer" => WashType::Outer,
            "Invert" => WashType::Invert,
            _ => WashType::Center,
        },
        movement_iterations: p.enum_name("MovementType") == "Iterations",
        iterations: p.int("Iterations"),
        speed_curve: p.curve("SpeedCurve"),
        horizontal_fade: p.boolean("HorizontalFade"),
        vertical_fade: p.boolean("VerticalFade"),
        shimmer: p.boolean("Shimmer"),
        color: p.gradient("Color"),
        level_curve: p.curve("LevelCurve"),
        half_ht: 0.0,
        half_wi: 0.0,
        position: 0.0,
    };
    render_pixel_effect(ctx, p, &mut e)
}

impl ColorWash {
    fn calculate_speed(&self, interval_pos: f64) -> f64 {
        scale_curve_to_value(self.speed_curve.get_value(interval_pos), 100.0, 0.0)
    }

    fn adjusted_v(&self, mut v: f64, x: i32, y: i32) -> f64 {
        let (hw, hh) = (self.half_wi, self.half_ht);
        let fx = ((hw - x as f64).abs() / hw) as f32 as f64;
        let fy = ((hh - y as f64).abs() / hh) as f32 as f64;
        match self.wash_type {
            WashType::Center => {
                if self.horizontal_fade && hw > 0.0 {
                    v *= (1.0 - (hw - x as f64).abs() / hw) as f32 as f64;
                }
                if self.vertical_fade && hh > 0.0 {
                    v *= (1.0 - (hh - y as f64).abs() / hh) as f32 as f64;
                }
            }
            WashType::Outer => {
                if self.horizontal_fade && hw > 0.0 {
                    v *= fx;
                }
                if self.vertical_fade && hh > 0.0 {
                    v *= fy;
                }
            }
            WashType::Invert => {
                if self.horizontal_fade && hw > 0.0 {
                    v /= (1.0 - (hw - x as f64).abs() / hw) as f32 as f64;
                }
                if self.vertical_fade && hh > 0.0 {
                    v /= (1.0 - (hh - y as f64).abs() / hh) as f32 as f64;
                }
            }
        }
        v
    }

    fn base_hsv(&self, position: f64) -> Hsv {
        Hsv::from_color(if position < 0.0 { self.color.get_color_at(1.0 + position) } else { self.color.get_color_at(position) })
    }
}

impl PixelEffect for ColorWash {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.half_ht = (info.buffer_ht() - 1) as f64 / 2.0;
        self.half_wi = (info.buffer_wi() - 1) as f64 / 2.0;
        self.position = 0.0;
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        if self.shimmer && frame % 2 != 0 {
            return;
        }
        let level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
        let total_frames = info.number_frames();
        let iteration_frame = if total_frames == 0 { 0 } else { frame.wrapping_mul(self.iterations) % total_frames };
        let interval_pos_factor = info.interval_position(frame) * 100.0;

        let position;
        if self.movement_iterations {
            position = info.interval_position(iteration_frame);
        } else {
            self.position += self.calculate_speed(interval_pos_factor) / 1000.0 * info.frame_time as f64 / 50.0;
            position = self.position % 1.0;
        }

        let hsv = self.base_hsv(position);
        for x in 0..info.buffer_wi() {
            for y in 0..info.buffer_ht() {
                let mut v = self.adjusted_v(hsv.v, x, y);
                v *= level;
                let mut hsv2 = hsv;
                hsv2.set_v(v);
                buf.set_pixel_hsv(x, y, hsv2);
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let mut nodes = buf.element_locations().to_vec();
        nodes.sort_by_key(|l| (l.x, l.y));
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
            let iteration_frame = if num_frames == 0 { 0 } else { frame.wrapping_mul(self.iterations) % num_frames };
            let interval_pos_factor = info.interval_position(frame) * 100.0;

            let position;
            if self.movement_iterations {
                position = info.interval_position(iteration_frame);
            } else {
                // Deliberate: unlike RenderEffect, the location version does not scale by FrameTime / 50.
                self.position += self.calculate_speed(interval_pos_factor) / 1000.0;
                position = self.position % 1.0;
            }

            let mut hsv = self.base_hsv(position);
            if self.shimmer && frame % 2 != 0 {
                hsv.set_v(0.0);
            }
            for l in &nodes {
                // Offsets give zero-based coordinates; the effect is symmetrical so no flipping.
                let mut v = self.adjusted_v(hsv.v, l.x - info.buffer_wi_offset(), l.y - info.buffer_ht_offset());
                v *= level;
                let mut hsv2 = hsv;
                hsv2.set_v(v);
                buf.set_pixel_hsv(l.x, l.y, hsv2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn speed_mode_hides_iterations() {
        let mut p = DESCRIPTOR.new_params();
        p.set("MovementType", ParamValue::Enum("Speed".to_string()));
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(visible.contains(&"SpeedCurve"));
        assert!(!visible.contains(&"Iterations"));
    }

    #[test]
    fn center_fade_dims_the_edges() {
        let mut e = ColorWash {
            wash_type: WashType::Center,
            movement_iterations: true,
            iterations: 1,
            speed_curve: Curve::flat(0.0),
            horizontal_fade: true,
            vertical_fade: false,
            shimmer: false,
            color: ColorGradient::solid(Color::WHITE),
            level_curve: Curve::flat100(),
            half_ht: 0.0,
            half_wi: 2.0,
            position: 0.0,
        };
        assert_eq!(e.adjusted_v(1.0, 0, 0), 0.0);
        assert_eq!(e.adjusted_v(1.0, 2, 0), 1.0);
        e.wash_type = WashType::Outer;
        assert_eq!(e.adjusted_v(1.0, 0, 0), 1.0);
        assert_eq!(e.adjusted_v(1.0, 2, 0), 0.0);
    }
}
