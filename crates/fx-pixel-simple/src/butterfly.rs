//! Port of VixenModules.Effect.Butterfly. This is the reference for how a pixel effect is ported.

use crate::generated::butterfly as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

const PI2: f64 = 6.283185307;

/// `InitAllAttributes`, `UpdateMovementTypeAttribute`, `UpdateGradientColorAttribute` and
/// `UpdateStringOrientationAttributes`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let speed = p.enum_name("MovementType") == "Speed";
    v.set("SpeedCurve", speed);
    v.set("Iterations", !speed);
    v.set("Direction", !speed);
    v.set("Color", p.enum_name("ColorScheme") == "Gradient");
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

#[derive(Copy, Clone, PartialEq)]
enum ButterflyType {
    Type1,
    Type2,
    Type3,
    Type4,
    Type5,
}

struct Butterfly {
    butterfly_type: ButterflyType,
    movement_speed: bool,
    direction_forward: bool,
    gradient_scheme: bool,
    iterations: i32,
    repeat: i32,
    background_chunks: i32,
    background_skips: i32,
    level_curve: Curve,
    speed_curve: Curve,
    color: ColorGradient,
    locations: bool,
    position: f64,
    neg_position: bool,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Butterfly {
        butterfly_type: match p.enum_name("ButterflyType") {
            "Type2" => ButterflyType::Type2,
            "Type3" => ButterflyType::Type3,
            "Type4" => ButterflyType::Type4,
            "Type5" => ButterflyType::Type5,
            _ => ButterflyType::Type1,
        },
        movement_speed: p.enum_name("MovementType") == "Speed",
        direction_forward: p.enum_name("Direction") == "Forward",
        gradient_scheme: p.enum_name("ColorScheme") == "Gradient",
        iterations: p.int("Iterations"),
        repeat: p.int("Repeat"),
        background_chunks: p.int("BackgroundChunks"),
        background_skips: p.int("BackgroundSkips"),
        level_curve: p.curve("LevelCurve"),
        speed_curve: p.curve("SpeedCurve"),
        color: p.gradient("Color"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        position: 0.0,
        neg_position: false,
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// The per-frame values `RenderEffect` and `RenderEffectByLocation` share.
struct FrameState {
    buffer_dim: f32,
    offset: f64,
    frame: i32,
    max_frame: i32,
    level: f64,
}

impl Butterfly {
    fn configure_repeat(&self) -> i32 {
        match self.butterfly_type {
            ButterflyType::Type1 | ButterflyType::Type5 => self.repeat * 3,
            ButterflyType::Type4 => self.repeat * 6,
            _ => self.repeat,
        }
    }

    fn calculate_speed(&self, interval_pos: f64, info: &PixelInfo) -> f64 {
        scale_curve_to_value(self.speed_curve.get_value(interval_pos), 80.0, -80.0) * info.frame_time as f64 / 50.0
    }

    fn frame_state(&mut self, effect_frame: i32, info: &PixelInfo) -> FrameState {
        let repeat = self.configure_repeat();
        let max_frame = info.buffer_ht();
        let interval_pos = info.interval_position(effect_frame) * 100.0;
        let position;
        if !self.movement_speed {
            position = (info.interval_position(effect_frame) * self.iterations as f64) % 1.0;
        } else {
            self.position += self.calculate_speed(interval_pos, info) / 200.0;
            if self.position < 0.0 {
                self.neg_position = true;
                position = -self.position % 1.0;
            } else {
                self.neg_position = false;
                position = self.position % 1.0;
            }
        }

        let total_ms = info.time_span.total_ms();
        let cur_state = (total_ms * position * repeat as f64) as i32;
        // Unchecked in the C#, so a long effect wraps and keeps rendering.
        let frame = (info.buffer_ht().wrapping_mul(cur_state) / total_ms as i32) % max_frame;
        let mut offset = cur_state as f64 / total_ms;
        let level = self.level_curve.get_value(info.interval_position(effect_frame) * 100.0) / 100.0;

        if self.movement_speed {
            if !self.neg_position {
                offset = -offset;
            }
        } else if self.direction_forward {
            offset = -offset;
        }

        let buffer_dim = match self.butterfly_type {
            ButterflyType::Type1 | ButterflyType::Type4 => (info.buffer_ht() + info.buffer_wi()) as f32,
            ButterflyType::Type5 => (info.buffer_ht() * info.buffer_wi()) as f32,
            _ => 0.0,
        };
        FrameState { buffer_dim, offset, frame, max_frame, level }
    }

    fn calculate_pixel<B: PixelBuffer>(&self, x_in: i32, y_in: i32, s: &FrameState, info: &PixelInfo, buf: &mut B) {
        let (x_coord, y_coord) = (x_in, y_in);
        let (mut x, mut y) = (x_in, y_in);
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        if self.locations {
            // Flip and offset the coordinates so the location version acts like the string version.
            y = ((info.buffer_ht_offset() - y) + (ht - 1 + info.buffer_ht_offset())).abs();
            y -= info.buffer_ht_offset();
            x -= info.buffer_wi_offset();
        }

        let dim = s.buffer_dim as f64;
        let wave = |a: i32, b: i32, offset: f64| ((a * a - b * b) as f64) * (offset + ((a + b) as f64 * PI2 / dim)).sin();

        let mut h = 0.0;
        match self.butterfly_type {
            ButterflyType::Type1 => {
                let mut n = wave(x, y, s.offset).abs();
                let mut d = x * x + y * y;
                // Fixes the colours of the pixels at {0,1} and {1,0}.
                if x == 0 && y == 1 {
                    let y0 = y + 1;
                    n = wave(x, y0, s.offset).abs();
                    d = x * x + y0 * y0;
                }
                if x == 1 && y == 0 {
                    let x0 = x + 1;
                    n = wave(x0, y, s.offset).abs();
                    d = x0 * x0 + y * y;
                }
                h = if d as f64 > 0.001 { n / d as f64 } else { 0.0 };
            }
            ButterflyType::Type2 => {
                let f = if s.frame < s.max_frame / 2 { s.frame + 1 } else { s.max_frame - s.frame } as f64;
                let x1 = (x as f64 - wi as f64 / 2.0) / f;
                let y1 = (y as f64 - ht as f64 / 2.0) / f;
                h = (x1 * x1 + y1 * y1).sqrt();
            }
            ButterflyType::Type3 => {
                let f = if s.frame < s.max_frame / 2 { s.frame + 1 } else { s.max_frame - s.frame } as f64;
                let f = f * 0.1 + ht as f64 / 60.0;
                let x1 = (x as f64 - wi as f64 / 2.0) / f;
                let y1 = (y as f64 - ht as f64 / 2.0) / f;
                h = x1.sin() * y1.cos();
            }
            ButterflyType::Type4 => {
                let mut n = wave(x, y, s.offset);
                let mut d = x * x + y * y;
                let (x0, y0) = (x + 1, y + 1);
                if x == 0 && y == 1 {
                    n = wave(x, y0, s.offset);
                    d = x * x + y0 * y0;
                }
                if x == 1 && y == 0 {
                    n = wave(x0, y, s.offset);
                    d = x0 * x0 + y * y;
                }
                h = if d as f64 > 0.001 { n / d as f64 } else { 0.0 };
                h -= h.floor();
                if h < 0.0 {
                    h += 1.0;
                }
            }
            ButterflyType::Type5 => {
                let mut n = wave(x, y, s.offset).abs();
                let mut d = x * x + y * y;
                let (x0, y0) = (x + 1, y + 1);
                if x == 0 && y == 1 {
                    n = wave(x, y0, s.offset).abs();
                    d = x * x + y0 * y0;
                }
                if x == 1 && y == 0 {
                    n = wave(x0, y, s.offset).abs();
                    d = x0 * x0 + y * y;
                }
                h = if d as f64 > 0.001 { n / d as f64 } else { 0.0 };
            }
        }

        let skip = self.background_skips.max(1);
        if self.background_chunks <= 1 || ((h * self.background_chunks as f64) as i32) % skip != 0 {
            if self.gradient_scheme {
                let color = self.color.get_color_at(h);
                if s.level < 1.0 {
                    let mut hsv = Hsv::from_color(color);
                    hsv.set_v(hsv.v * s.level);
                    buf.set_pixel_hsv(x_coord, y_coord, hsv);
                } else {
                    buf.set_pixel(x_coord, y_coord, color);
                }
            } else {
                buf.set_pixel_hsv(x_coord, y_coord, Hsv::new(h, 1.0, s.level));
            }
        } else {
            buf.set_pixel(x_coord, y_coord, Color::TRANSPARENT);
        }
    }
}

impl PixelEffect for Butterfly {
    fn setup_render(&mut self, _info: &PixelInfo) {
        self.position = 0.0;
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let s = self.frame_state(frame, info);
        for x in 0..info.buffer_wi() {
            for y in 0..info.buffer_ht() {
                self.calculate_pixel(x, y, &s, info, buf);
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let mut nodes = buf.element_locations().to_vec();
        nodes.sort_by_key(|l| (l.x, l.y));
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let s = self.frame_state(frame, info);
            for l in &nodes {
                self.calculate_pixel(l.x, l.y, &s, info, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::testing;

    #[test]
    fn choosing_locations_forces_vertical_and_strings_follow_the_target() {
        use vixen_core::element::{property_ids, Orientation, PropertyData, PropertyInstance};
        let mut tree = ElementTree::new();
        let s = testing::grid(&mut tree, "M", 4, 12);
        tree.node_mut(s).properties.push(PropertyInstance {
            type_id: property_ids::ORIENTATION,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Orientation(Orientation::Horizontal),
        });
        let targets = [s];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();

        // A saved orientation wins over the target's when loading.
        p.set("StringOrientation", ParamValue::Enum("Vertical".into()));
        DESCRIPTOR.load_data(&mut p, &t);
        assert_eq!(p.enum_name("StringOrientation"), "Vertical");

        // Giving it targets follows them.
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(p.enum_name("StringOrientation"), "Horizontal");

        // Locations are always vertical, and going back follows the target again.
        DESCRIPTOR.set_param(&mut p, "TargetPositioning", ParamValue::Enum("Locations".into()), &t);
        assert_eq!(p.enum_name("StringOrientation"), "Vertical");
        DESCRIPTOR.set_param(&mut p, "TargetPositioning", ParamValue::Enum("Strings".into()), &t);
        assert_eq!(p.enum_name("StringOrientation"), "Horizontal");
    }

    #[test]
    fn a_very_long_effect_wraps_instead_of_panicking() {
        // 100 pixels over 180 s with a large repeat overflows `BufferHt * curState`, which wraps in the C#.
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 100);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 180_000, |p| {
            p.set("ButterflyType", ParamValue::Enum("Type4".into()));
            p.set("Repeat", ParamValue::Int(20));
            p.set("Iterations", ParamValue::Int(1));
        });
        assert!(!out.is_empty());
    }

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
}
