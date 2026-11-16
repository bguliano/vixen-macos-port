//! Port of VixenModules.Effect.Shockwave (Shockwave.cs and Shockwave_Diamond.cs).

use crate::generated::shockwave as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `InitAllAttributes` and `EnableTargetPositioning(true, true)`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

struct Shockwave {
    diamond: bool,
    vertical: bool,
    scaled_radius: bool,
    blend_edges: bool,
    width_curve: Curve,
    radius_curve: Curve,
    center_x_curve: Curve,
    center_y_curve: Curve,
    acceleration_curve: Curve,
    color: ColorGradient,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Shockwave {
        diamond: p.enum_name("ShockWaveType") == "Diamond",
        vertical: p.enum_name("StringOrientation") != "Horizontal",
        scaled_radius: p.boolean("ScaledRadius"),
        blend_edges: p.boolean("BlendEdges"),
        width_curve: p.curve("WidthCurve"),
        radius_curve: p.curve("RadiusCurve"),
        center_x_curve: p.curve("CenterXCurve"),
        center_y_curve: p.curve("CenterYCurve"),
        acceleration_curve: p.curve("AccelerationCurve"),
        color: p.gradient("Color"),
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// `hsv.V = hsv.V * intensity` on the colour's HSV.
#[inline]
fn scaled(c: Color, intensity: f64) -> Hsv {
    let mut hsv = Hsv::from_color(c);
    hsv.set_v(hsv.v * intensity);
    hsv
}

#[inline]
fn contains_point_circle(distance: f64, inner: f64, outer: f64) -> bool {
    distance <= outer && distance >= inner
}

/// `DistanceFromCenter`.
#[inline]
fn distance_from_center(cx: i32, cy: i32, x: i32, y: i32) -> f64 {
    let (dx, dy) = ((x - cx) as f64, (y - cy) as f64);
    (dx.powi(2) + dy.powi(2)).sqrt()
}

impl Shockwave {
    fn calculate_width(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.width_curve.get_value(pos), 255.0, 0.0)
    }
    fn calculate_center_x(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.center_x_curve.get_value(pos), 100.0, 0.0)
    }
    fn calculate_center_y(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.center_y_curve.get_value(pos), 100.0, 0.0)
    }
    fn calculate_acceleration(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.acceleration_curve.get_value(pos), 10.0, -10.0)
    }
    fn calculate_radius(&self, pos: f64, scale: i32) -> f64 {
        scale_curve_to_value(self.radius_curve.get_value(pos), scale as f64, 0.0)
    }

    fn draw_circular<B: PixelBuffer>(&self, pos_x: f64, pos_y: f64, buf: &mut B, c: Color, width: f64, center_radius: f64, wi: i32, ht: i32) {
        let half_width = width / 2.0;
        let (cx, cy) = (pos_x as i32, pos_y as i32);
        let radius1 = (center_radius - half_width).max(0.0);
        let radius2 = center_radius + half_width;
        for x in 0..wi {
            for y in 0..ht {
                let distance = distance_from_center(cx, cy, x, y);
                if contains_point_circle(distance, radius1, radius2) {
                    if self.blend_edges {
                        let color_pct = 1.0 - (distance - center_radius).abs() / half_width;
                        if color_pct > 0.0 {
                            buf.set_pixel_hsv(x, y, scaled(c, color_pct));
                        }
                    } else {
                        buf.set_pixel(x, y, c);
                    }
                }
            }
        }
    }

    // ---- Shockwave_Diamond.cs, string methods ----

    fn draw_diamond_shockwave<B: PixelBuffer>(&self, buf: &mut B, pos_x: f64, pos_y: f64, color: Color, width: f64, outer_radius: f64) {
        let outside_diameter = 2.0 * outer_radius;
        self.draw_diamond(outside_diameter as i32, width as i32, buf, pos_x as i32, pos_y as i32, color);
    }

    fn draw_diamond<B: PixelBuffer>(&self, outer_length: i32, thickness: i32, buf: &mut B, center_x: i32, center_y: i32, color: Color) {
        // thickness 0 gives an infinite increment in C#, but the loops below then never run.
        let intensity_increment = 1.0 / thickness as f64;
        let mut y_start = center_y + outer_length / 2;
        let mut row_width: i32 = 1;
        for h in 0..outer_length {
            let mut intensity = intensity_increment;
            for rw in 0..thickness {
                if (center_x - row_width / 2 + rw) <= center_x {
                    let hsv = scaled(color, if self.blend_edges { intensity } else { 1.0 });
                    buf.set_pixel_hsv(center_x - row_width / 2 + rw, y_start, hsv);
                    intensity = update_pixel_intensity(rw, thickness, intensity, intensity_increment);
                }
            }
            let mut intensity2 = intensity_increment;
            for r in 0..thickness {
                if center_x + row_width / 2 - r > center_x {
                    let hsv = scaled(color, if self.blend_edges { intensity2 } else { 1.0 });
                    buf.set_pixel_hsv(center_x + row_width / 2 - r, y_start, hsv);
                    intensity2 = update_pixel_intensity(r, thickness, intensity2, intensity_increment);
                }
            }
            row_width = update_row_width(h, row_width, outer_length);
            y_start -= 1;
        }
    }
}

/// `UpdateRowWidth`.
fn update_row_width(y: i32, row_width: i32, height: i32) -> i32 {
    if y < height / 2 {
        row_width + 2
    } else {
        row_width - 2
    }
}

/// `UpdatePixelIntensity`.
fn update_pixel_intensity(r: i32, thickness: i32, intensity: f64, inc: f64) -> f64 {
    if r <= thickness / 2 {
        intensity + inc
    } else {
        intensity - inc
    }
}

// ---- Shockwave_Diamond.cs, location methods ----

/// `ContainsPoint(x, y, outerLength, centerX, centerY, width, out intensity)`.
fn diamond_contains_point(x: i32, y: i32, outer_length: f64, center_x: i32, center_y: i32, mut width: i32) -> (bool, f64) {
    let mut intensity = 1.0;
    let y_start = (center_y as f64 - outer_length / 2.0) as i32;
    let mut contains = false;
    if y >= y_start && (y as f64) < y_start as f64 + outer_length {
        let row_width = calculate_row_width(y_start, y, center_y);
        if width > row_width {
            width = row_width;
        }
        if (x > center_x - row_width / 2 && x < center_x - row_width / 2 + width)
            || (x > center_x + row_width / 2 - width && x < center_x + row_width / 2)
        {
            contains = true;
            intensity = calculate_intensity(center_x, x, width, row_width);
        }
    }
    (contains, intensity)
}

/// `CalculateIntensity`.
fn calculate_intensity(center_x: i32, x: i32, width: i32, row_width: i32) -> f64 {
    if x >= center_x {
        let center = (center_x + row_width / 2 - width / 2) as f64;
        intensity_from_distance(center, x, width)
    } else {
        let center = (center_x - row_width / 2 + width / 2) as f64;
        intensity_from_distance(center, x, width)
    }
}

/// `CalculateIntensityBasedOnDistanceFromCenter`.
fn intensity_from_distance(center: f64, x: i32, width: i32) -> f64 {
    let mut d = (x as f64 - center).abs();
    if d > width as f64 {
        d = width as f64;
    }
    1.0 - d / width as f64
}

/// `CalculateRowWidth`.
fn calculate_row_width(y_start: i32, y: i32, center_y: i32) -> i32 {
    let mut row_width = 1;
    let mut h = y_start;
    while h <= y {
        if h < center_y {
            row_width += 2;
        } else {
            row_width -= 2;
        }
        h += 1;
    }
    row_width
}

impl PixelEffect for Shockwave {
    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let interval_pos = info.interval_position(frame);
        let factor = interval_pos * 100.0;
        let effect_position_adjust =
            calculate_acceleration(interval_pos, self.calculate_acceleration(factor)) * 100.0;
        let color = self.color.get_color_at(interval_pos);
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());

        let (pos_x, pos_y);
        if self.vertical {
            pos_x = wi as f64 * self.calculate_center_y(factor) / 100.0;
            pos_y = ht as f64 * self.calculate_center_x(factor) / 100.0;
        } else {
            pos_x = wi as f64 * self.calculate_center_x(factor) / 100.0;
            pos_y = ht as f64 * self.calculate_center_y(factor) / 100.0;
        }

        let center_radius =
            self.calculate_radius(effect_position_adjust, if self.scaled_radius { ht.max(wi) } else { 750 });
        let width = self.calculate_width(effect_position_adjust);

        if self.diamond {
            self.draw_diamond_shockwave(buf, pos_x, pos_y, color, width, center_radius);
        } else {
            self.draw_circular(pos_x, pos_y, buf, color, width, center_radius, wi, ht);
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let mut nodes = buf.element_locations().to_vec();
        nodes.sort_by_key(|l| (l.x, l.y));
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());

        for effect_frame in 0..num_frames {
            let interval_pos = info.interval_position(effect_frame);
            let factor = interval_pos * 100.0;
            let pos_x = wi as f64 * self.calculate_center_x(factor) / 100.0 + info.buffer_wi_offset() as f64;
            let pos_y = ht as f64 * (100.0 - self.calculate_center_y(factor)) / 100.0 + info.buffer_ht_offset() as f64;
            let (cx, cy) = (pos_x as i32, pos_y as i32);

            buf.current_frame = effect_frame;
            let effect_position_adjust =
                calculate_acceleration(interval_pos, self.calculate_acceleration(factor)) * 100.0;
            let c = self.color.get_color_at(interval_pos);
            let center_radius =
                self.calculate_radius(effect_position_adjust, if self.scaled_radius { ht.max(wi) } else { 750 });
            let half_width = self.calculate_width(effect_position_adjust) / 2.0;
            let radius1 = (center_radius - half_width).max(0.0);
            let radius2 = center_radius + half_width;

            for l in &nodes {
                let distance = distance_from_center(cx, cy, l.x, l.y);
                let mut diamond_intensity = 1.0;
                let inside = if self.diamond {
                    let (inside, intensity) =
                        diamond_contains_point(l.x, l.y, radius2 * 2.0, cx, cy, (half_width * 2.0) as i32);
                    diamond_intensity = intensity;
                    inside
                } else {
                    contains_point_circle(distance, radius1, radius2)
                };

                if inside {
                    if self.blend_edges {
                        if !self.diamond {
                            let color_pct = 1.0 - (distance - center_radius).abs() / half_width;
                            if color_pct > 0.0 {
                                buf.set_pixel_hsv(l.x, l.y, scaled(c, color_pct));
                            } else {
                                buf.set_pixel(l.x, l.y, Color::TRANSPARENT);
                            }
                        } else {
                            buf.set_pixel_hsv(l.x, l.y, scaled(c, diamond_intensity));
                        }
                    } else {
                        buf.set_pixel(l.x, l.y, c);
                    }
                } else {
                    buf.set_pixel(l.x, l.y, Color::TRANSPARENT);
                }
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
    fn diamond_smoke() {
        let f = frame(&DESCRIPTOR, 8, 25, 2000, 10, |p| {
            p.set("ShockWaveType", ParamValue::Enum("Diamond".to_string()));
        });
        assert!(lit(&f) > 0);
    }

    #[test]
    fn row_width_grows_then_shrinks() {
        // Diamond rows widen by 2 until the centre row, then narrow.
        assert_eq!(calculate_row_width(0, 0, 3), 3);
        assert_eq!(calculate_row_width(0, 2, 3), 7);
        assert_eq!(calculate_row_width(0, 3, 3), 5);
    }

    #[test]
    fn circle_ring_is_centred_and_hollow() {
        // 21 strings x 21 pixels, no blending, flat radius 5 and width 2 (curve values are percent of 750
        // or of the buffer: scaled radius 21 * 25% = 5.25, width 255 * 1% = 2.55 -> pixels 4.0..6.5 away).
        let f = frame(&DESCRIPTOR, 21, 21, 1000, 5, |p| {
            p.set("BlendEdges", ParamValue::Bool(false));
            p.set("RadiusCurve", ParamValue::Curve(Curve::flat(25.0)));
            p.set("WidthCurve", ParamValue::Curve(Curve::flat(1.0)));
            p.set("AccelerationCurve", ParamValue::Curve(Curve::flat(50.0)));
            p.set("Color", ParamValue::Gradient(ColorGradient::solid(Color::WHITE)));
        });
        // Centre is (10, 10) from 21 * 50% truncated; the centre pixel is inside the hole.
        assert_eq!(f[10][10], RgbValue::BLACK);
        // Distance 5 along the axis is on the ring.
        assert_eq!(f[15][10], RgbValue { r: 255, g: 255, b: 255 });
        assert_eq!(f[10][5], RgbValue { r: 255, g: 255, b: 255 });
        // Far corner is outside it.
        assert_eq!(f[0][0], RgbValue::BLACK);
    }
}
