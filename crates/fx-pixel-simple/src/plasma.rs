//! Port of VixenModules.Effect.Plasma.

use crate::generated::plasma as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

const PI: f64 = std::f64::consts::PI;

/// `UpdateColorAttribute` (the gradients only matter for the Custom colour type),
/// `UpdateStringOrientationAttributes` and `EnableTargetPositioning`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    v.set("Colors", p.enum_name("ColorType") == "Normal");
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

#[derive(Copy, Clone, PartialEq)]
enum ColorType {
    Normal,
    Preset1,
    Preset2,
    Preset3,
    Preset4,
}

struct Plasma {
    color_type: ColorType,
    colors: Vec<ColorGradient>,
    plasma_style: i32,
    level_curve: Curve,
    speed_curve: Curve,
    line_density_curve: Curve,
    locations: bool,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // The presets don't use the list. Otherwise indexing it when empty throws in the C#, and a pixel effect that throws
    // has no intents at all.
    if p.enum_name("ColorType") == "Normal" && p.gradients("Colors").is_empty() {
        return EffectIntents::new();
    }
    let mut e = Plasma {
        color_type: match p.enum_name("ColorType") {
            "Preset1" => ColorType::Preset1,
            "Preset2" => ColorType::Preset2,
            "Preset3" => ColorType::Preset3,
            "Preset4" => ColorType::Preset4,
            _ => ColorType::Normal,
        },
        colors: p.gradients("Colors"),
        plasma_style: p.int("PlasmaStyle"),
        level_curve: p.curve("LevelCurve"),
        speed_curve: p.curve("SpeedCurve"),
        line_density_curve: p.curve("LineDensityCurve"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// The per-frame values both render paths share.
struct FrameState {
    time: f64,
    level: f64,
    line_density: f64,
    frame: i32,
}

impl Plasma {
    fn calculate_speed(&self, interval_pos: f64) -> f64 {
        scale_curve_to_value(self.speed_curve.get_value(interval_pos), 100.0, 0.0)
    }

    fn calculate_line_density(&self, interval_pos: f64) -> f64 {
        let value = scale_curve_to_value(self.line_density_curve.get_value(interval_pos), 10.0, 1.0);
        if value < 1.0 {
            1.0
        } else {
            value
        }
    }

    fn frame_state(&self, frame: i32, info: &PixelInfo) -> FrameState {
        let interval_pos_factor = info.interval_position(frame) * 100.0;
        let speed = self.calculate_speed(interval_pos_factor);
        let position = interval_pos_factor * speed;
        let plasma_speed = (101.0 - speed) * 3.0;
        FrameState {
            time: (position + 1.0) / plasma_speed,
            level: self.level_curve.get_value(interval_pos_factor) / 100.0,
            line_density: self.calculate_line_density(interval_pos_factor) * PI,
            frame,
        }
    }

    fn gradient_at(&self, idx: usize, pos: f64) -> Color {
        match self.colors.get(idx) {
            Some(g) => g.get_color_at(pos),
            None => Color::EMPTY,
        }
    }

    fn multi_color_blend(&self, mut n: f64, frame: i32, info: &PixelInfo) -> Color {
        let count = self.colors.len();
        if count <= 1 {
            return self.gradient_at(0, info.interval_position(frame));
        }
        if n >= 1.0 {
            n = 0.99999;
        }
        if n < 0.0 {
            n = 0.0;
        }
        let idx1 = (n * count as f64).floor() as i32;
        let idx2 = (idx1 + 1) % count as i32;
        let ratio = n * count as f64 - idx1 as f64;
        let pos = info.interval_position(frame);
        let c1 = self.gradient_at(idx1 as usize, pos);
        let c2 = self.gradient_at(idx2 as usize, pos);
        let blend = |a: u8, b: u8| a as i32 + (ratio * (b as i32 - a as i32) as f64 + 0.5).floor() as i32;
        Color::from_rgb_i32(blend(c1.r, c2.r), blend(c1.g, c2.g), blend(c1.b, c2.b))
    }

    fn calculate_pixel<B: PixelBuffer>(&self, x_in: i32, y_in: i32, s: &FrameState, info: &PixelInfo, buf: &mut B) {
        let (x_coord, y_coord) = (x_in, y_in);
        let (mut x, mut y) = (x_in, y_in);
        if self.locations {
            // Flip and offset the coordinates so the location version acts like the string version.
            y = ((info.buffer_ht_offset() - y) + (info.buffer_ht() - 1 + info.buffer_ht_offset())).abs();
            y -= info.buffer_ht_offset();
            x -= info.buffer_wi_offset();
        }
        let time = s.time;
        let rx = (x as f32 / info.buffer_wi() as f32) as f64;
        let ry = (y as f32 / info.buffer_ht() as f32) as f64;

        // 1st equation
        let mut v = (rx * 10.0 + time).sin();
        // 2nd equation
        v += (10.0 * (rx * (time / 2.0).sin() + ry * (time / 3.0).cos()) + time).sin();
        // 3rd equation
        let cx = rx + 0.5 * (time / 5.0).sin();
        let cy = ry + 0.5 * (time / 3.0).cos();
        v += (((self.plasma_style * 50) as f64) * ((cx * cx) + (cy * cy)) + time).sqrt().sin();
        v += (rx + time).sin();
        v += ((ry + time) / 2.0).sin();
        v += ((rx + ry + time) / 2.0).sin();
        v += ((rx * rx + ry * ry).sqrt() + time).sin();
        v /= 2.0;

        let d = v * s.line_density;
        let mut color = match self.color_type {
            ColorType::Normal => {
                // `1 * 0.5` is deliberate: the C# adds 0.5, not (… + 1) * 0.5.
                let h = (d + 2.0 * PI / 3.0).sin() + 1.0 * 0.5;
                self.multi_color_blend(h / 2.0, s.frame, info)
            }
            ColorType::Preset1 => {
                Color::from_rgb_i32(((d.sin() + 1.0) * 128.0) as i32, ((d.cos() + 1.0) * 128.0) as i32, 0)
            }
            ColorType::Preset2 => {
                Color::from_rgb_i32(1, ((d.cos() + 1.0) * 128.0) as i32, ((d.sin() + 1.0) * 128.0) as i32)
            }
            ColorType::Preset3 => {
                Color::from_rgb_i32(
                    ((d.sin() + 1.0) * 128.0) as i32,
                    (((d + 2.0 * PI / 3.0).sin() + 1.0) * 128.0) as i32,
                    (((d + 4.0 * PI / 3.0).sin() + 1.0) * 128.0) as i32,
                )
            }
            ColorType::Preset4 => {
                let g = ((d.sin() + 1.0) * 128.0) as i32;
                Color::from_rgb_i32(g, g, g)
            }
        };
        if s.level < 1.0 {
            let mut hsv = Hsv::from_color(color);
            hsv.set_v(hsv.v * s.level);
            color = hsv.to_color();
        }
        buf.set_pixel(x_coord, y_coord, color);
    }
}

impl PixelEffect for Plasma {
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
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn preset_colour_types_hide_the_gradients() {
        let mut p = DESCRIPTOR.new_params();
        let shown = |p: &Params| DESCRIPTOR.visible_params(p).into_iter().any(|i| DESCRIPTOR.params[i].name == "Colors");
        assert!(shown(&p));
        p.set("ColorType", ParamValue::Enum("Preset3".to_string()));
        assert!(!shown(&p));
    }

    #[test]
    fn black_and_white_preset_is_grey() {
        let e = Plasma {
            color_type: ColorType::Preset4,
            colors: Vec::new(),
            plasma_style: 1,
            level_curve: Curve::flat100(),
            speed_curve: Curve::flat(10.0),
            line_density_curve: Curve::flat(10.0),
            locations: false,
        };
        let info = crate::test_util::info(4, 4, 1000);
        let s = e.frame_state(0, &info);
        // Speed curve 10 maps to 10, so time = (0 + 1) / ((101 - 10) * 3) at the first frame.
        assert!((s.time - 1.0 / 273.0).abs() < 1e-12);
        let mut buf = PixelFrameBuffer::new(4, 4);
        e.calculate_pixel(1, 2, &s, &info, &mut buf);
        let c = buf.get_color_at(1, 2);
        assert_eq!((c.r, c.g), (c.g, c.b));
    }
}
