//! Port of VixenModules.Effect.Spirograph.

use crate::generated::spirograph as gen;
use vixen_effects::prelude::*;

// The C# constructor never calls `EnableTargetPositioning` and has no `RenderEffectByLocation`.
pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    ported: true,
    ..gen::BASE
};

/// The `OCR` and `ICR` property setters keep the outer radius above the inner one.
fn param_changed(name: &str, p: &mut Params) {
    match name {
        "OCR" if p.int("ICR") > p.int("OCR") => {
            let v = p.int("ICR") + 2;
            p.set("OCR", ParamValue::Int(v));
        }
        "ICR" if p.int("OCR") <= p.int("ICR") => {
            let v = p.int("OCR") - 2;
            p.set("ICR", ParamValue::Int(v));
        }
        _ => {}
    }
}

/// `UpdateColorAttribute`: the gradients are unused by the Rainbow colour type.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    v.set("Colors", p.enum_name("Type") != "Rainbow");
}

#[derive(Copy, Clone, PartialEq)]
enum ColorType {
    Standard,
    Rainbow,
    Random,
}

struct Spirograph {
    color_type: ColorType,
    colors: Vec<ColorGradient>,
    animate: bool,
    ocr: i32,
    icr: i32,
    level_curve: Curve,
    speed_curve: Curve,
    distance_curve: Curve,
    range_curve: Curve,
    spirograph_range_curve: Curve,
    rand: Rand,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Spirograph {
        color_type: match p.enum_name("Type") {
            "Rainbow" => ColorType::Rainbow,
            "Random" => ColorType::Random,
            _ => ColorType::Standard,
        },
        colors: p.gradients("Colors"),
        animate: p.boolean("Animate"),
        ocr: p.int("OCR"),
        icr: p.int("ICR"),
        level_curve: p.curve("LevelCurve"),
        speed_curve: p.curve("SpeedCurve"),
        distance_curve: p.curve("DistanceCurve"),
        range_curve: p.curve("RangeCurve"),
        spirograph_range_curve: p.curve("SpirographRangeCurve"),
        rand: Rand::default(),
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// `Convert.ToInt32(double)`. `None` is the `OverflowException` the C# catches around the whole loop.
fn to_int32(v: f64) -> Option<i32> {
    if v.is_nan() || v >= 2147483647.5 || v <= -2147483648.5 {
        None
    } else {
        Some(v.round_ties_even() as i32)
    }
}

impl Spirograph {
    fn calculate_speed(&self, p: f64) -> f64 {
        scale_curve_to_value(self.speed_curve.get_value(p), 100.0, 0.0)
    }

    fn calculate_distance(&self, p: f64) -> f64 {
        let v = scale_curve_to_value(self.distance_curve.get_value(p), 100.0, 1.0);
        if v < 1.0 {
            1.0
        } else {
            v
        }
    }

    fn calculate_range(&self, p: f64) -> f64 {
        scale_curve_to_value(self.range_curve.get_value(p), 200.0, 1.0)
    }

    fn calculate_spirograph_range(&self, p: f64) -> f64 {
        scale_curve_to_value(self.spirograph_range_curve.get_value(p), 300.0, 1.0)
    }
}

impl PixelEffect for Spirograph {
    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        let interval_pos = info.interval_position(frame);
        let interval_pos_factor = interval_pos * 100.0;
        let level = self.level_curve.get_value(interval_pos_factor) / 100.0;
        let range_adjust = self.calculate_range(interval_pos_factor);
        let state = frame as f64 * self.calculate_speed(interval_pos_factor);
        let color_count = self.colors.len() as i32;

        let xc = wi / 2;
        let yc = ht / 2;
        // The C# keeps these in `float`.
        let big_r = (xc as f64 * (self.ocr as f64 / 100.0)) as f32;
        let mut small_r = (xc as f64 * (self.icr as f64 / 100.0)) as f32;
        if small_r > big_r {
            small_r = big_r;
        }
        let mut d = (xc as f64 * (self.calculate_distance(interval_pos_factor) / 100.0)) as f32;

        // A hypotrochoid: a point at distance d from the centre of a circle of radius r rolling inside a
        // fixed circle of radius R. The C# wraps the loop in try/catch, so any exception just ends it.
        let Some(mod1440) = to_int32(state % 1440.0) else { return };
        let d_orig = d;
        let limit = self.calculate_spirograph_range(interval_pos_factor) * 18.0;
        let mut i = 1;
        while (i as f64) <= limit {
            if self.animate {
                d = (((d_orig as f64 + state / 2.0) as i32) % 100) as f32;
            }
            let t = ((i + mod1440) as f64 * std::f64::consts::PI / 180.0) as f32;
            let rr = big_r - small_r;
            let ratio_t = (rr / small_r) * t;
            let Some(x) = to_int32(rr as f64 * (t as f64).cos() + d as f64 * (ratio_t as f64).cos() + xc as f64) else { return };
            let Some(y) = to_int32(rr as f64 * (t as f64).sin() + d as f64 * (ratio_t as f64).sin() + yc as f64) else { return };

            let d_mod = if color_count > 0 { (range_adjust / color_count as f64).round_ties_even() as i32 } else { 1 };
            let x2 = ((x - xc) as f64).powi(2);
            let y2 = ((y - yc) as f64).powi(2);
            let hyp = (x2 + y2).sqrt() / range_adjust * 100.0;

            let mut color_idx = match self.color_type {
                ColorType::Random => self.rand.next_range(0, color_count),
                ColorType::Rainbow => 1,
                ColorType::Standard => (hyp / d_mod as f64) as i32,
            };
            if color_idx >= color_count {
                color_idx = color_count - 1;
            }

            let col;
            if self.color_type != ColorType::Rainbow {
                // An index outside the list is the C# IndexOutOfRangeException, which ends the loop.
                let Some(g) = self.colors.get(color_idx.max(-1) as usize).filter(|_| color_idx >= 0) else { return };
                let mut c = g.get_color_at(interval_pos_factor / 100.0);
                if level < 1.0 {
                    let mut hsv = Hsv::from_color(c);
                    hsv.set_v(hsv.v * level);
                    c = hsv.to_color();
                }
                col = c;
            } else {
                let h = ((self.rand.next() % 1000) as f32 / 1000.0f32) as f64;
                col = Hsv::new(h, 1.0, 1.0 * level).to_color();
            }
            buf.set_pixel(x, y, col);
            i += 1;
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
    fn radius_setters_keep_outer_above_inner() {
        let mut p = DESCRIPTOR.new_params();
        p.set("ICR", ParamValue::Int(100));
        param_changed("ICR", &mut p);
        assert_eq!(p.int("ICR"), 86);
        p.set("OCR", ParamValue::Int(1));
        param_changed("OCR", &mut p);
        assert_eq!(p.int("OCR"), 88);
    }

    #[test]
    fn rainbow_hides_the_gradients() {
        let mut p = DESCRIPTOR.new_params();
        let shown = |p: &Params| DESCRIPTOR.visible_params(p).into_iter().any(|i| DESCRIPTOR.params[i].name == "Colors");
        assert!(shown(&p));
        p.set("Type", ParamValue::Enum("Rainbow".to_string()));
        assert!(!shown(&p));
    }

    #[test]
    fn convert_to_int32_rounds_to_even_and_rejects_overflow() {
        assert_eq!(to_int32(2.5), Some(2));
        assert_eq!(to_int32(3.5), Some(4));
        assert_eq!(to_int32(-0.5), Some(0));
        assert_eq!(to_int32(f64::NAN), None);
        assert_eq!(to_int32(3.0e9), None);
    }

    #[test]
    fn first_point_lands_where_the_equations_put_it() {
        // 20 wide: xc = 10, R = 8.8, r = 2.1, and the smallest distance gives d = 0.1. At frame 0 the first
        // point has t = 1 degree: x = 6.7 cos(1) + 0.1 cos(3.19) + 10 = 16.8 -> 17, y = 10.1 -> 10.
        let mut e = Spirograph {
            color_type: ColorType::Standard,
            colors: vec![ColorGradient::solid(Color::WHITE)],
            animate: false,
            ocr: 88,
            icr: 21,
            level_curve: Curve::flat100(),
            speed_curve: Curve::flat(0.0),
            distance_curve: Curve::flat(0.0),
            range_curve: Curve::flat(35.0),
            spirograph_range_curve: Curve::flat(0.0),
            rand: Rand::seeded(3),
        };
        let info = crate::test_util::info(20, 20, 1000);
        let mut buf = PixelFrameBuffer::new(20, 20);
        buf.clear_buffer();
        e.render_effect(0, &info, &mut buf);
        assert_eq!(buf.get_color_at(17, 10), Color::WHITE);
        // A range of 0 maps to 1, so there are only 18 points.
        let lit = (0..20).flat_map(|x| (0..20).map(move |y| (x, y))).filter(|&(x, y)| buf.get_color_at(x, y) == Color::WHITE).count();
        assert!(lit <= 18, "{lit}");
    }
}
