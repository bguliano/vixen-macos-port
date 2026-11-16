//! Port of VixenModules.Effect.Circles.

use crate::generated::circles as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `UpdateColorAttribute`, `UpdateStringOrientationAttributes` and `EnableTargetPositioning`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    v.set("CircleEdgeWidthCurve", p.enum_name("CircleFill") == "Empty");
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

#[derive(Copy, Clone, PartialEq)]
enum Fill {
    Fade,
    Empty,
    GradientOverTime,
    GradientOverElement,
}

#[derive(Copy, Clone, PartialEq)]
enum Radial {
    Out,
    In,
}

struct Circles {
    fill: Fill,
    radial: Radial,
    colors: Vec<ColorGradient>,
    center_speed_curve: Curve,
    circle_count_curve: Curve,
    edge_width_curve: Curve,
    level_curve: Curve,
    size_curve: Curve,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    locations: bool,
    /// `_circleCount`. Deliberately read before it is updated each frame, so frame 0 sees 0 and every
    /// other frame sees the previous frame's value, as in the C#.
    circle_count: f64,
    max_buffer: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // Indexing the empty list throws in the C#, and a pixel effect that throws has no intents at all.
    if p.gradients("Colors").is_empty() {
        return EffectIntents::new();
    }
    let mut e = Circles {
        fill: match p.enum_name("CircleFill") {
            "Empty" => Fill::Empty,
            "GradientOverTime" => Fill::GradientOverTime,
            "GradientOverElement" => Fill::GradientOverElement,
            _ => Fill::Fade,
        },
        radial: if p.enum_name("CircleRadialDirection") == "In" { Radial::In } else { Radial::Out },
        colors: p.gradients("Colors"),
        center_speed_curve: p.curve("CenterSpeedCurve"),
        circle_count_curve: p.curve("CircleCountCurve"),
        edge_width_curve: p.curve("CircleEdgeWidthCurve"),
        level_curve: p.curve("LevelCurve"),
        size_curve: p.curve("SizeCurve"),
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        circle_count: 0.0,
        max_buffer: 0,
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// Everything `CalculatePixel` needs for one frame.
struct FrameState {
    level: f64,
    interval_pos: f64,
    radius: f64,
    radius1: f64,
    current_radius: f64,
    foffset: f64,
    barht: f64,
    block_ht: f64,
    x_offset: i32,
    y_offset: i32,
    edge_width: f64,
}

impl Circles {
    fn calculate_center_speed(&self, p: f64, info: &PixelInfo) -> f64 {
        scale_curve_to_value(self.center_speed_curve.get_value(p), 10.0, 0.0) * info.frame_time as f64 / 50.0
    }

    fn calculate_radial_size(&self, p: f64) -> f64 {
        scale_curve_to_value(self.size_curve.get_value(p), self.max_buffer as f64 * 1.1, 1.0)
    }

    fn calculate_edge_width(&self, p: f64, current_radius: f64) -> f64 {
        let v = scale_curve_to_value(self.edge_width_curve.get_value(p), current_radius, 1.0);
        if v < 1.0 {
            1.0
        } else {
            v
        }
    }

    fn calculate_circle_count(&self, p: f64) -> f64 {
        let v = scale_curve_to_value(self.circle_count_curve.get_value(p), self.max_buffer as f64 / 2.0, 1.0) as i32 as f64;
        if v < 1.0 {
            1.0
        } else {
            v
        }
    }

    fn calculate_x_offset(&self, p: f64) -> i32 {
        scale_curve_to_value(self.x_offset_curve.get_value(p), -(self.max_buffer as f64) * 1.1, self.max_buffer as f64 * 1.1) as i32
    }

    fn calculate_y_offset(&self, p: f64) -> i32 {
        scale_curve_to_value(self.y_offset_curve.get_value(p), -(self.max_buffer as f64) * 1.1, self.max_buffer as f64 * 1.1) as i32
    }

    /// The per-frame values, computed from the previous frame's circle count. The caller then updates
    /// `circle_count`.
    fn frame_state(&self, frame: i32, info: &PixelInfo) -> FrameState {
        let interval_pos = info.interval_position(frame);
        let interval_pos_factor = interval_pos * 100.0;
        let x_offset = self.calculate_x_offset(interval_pos_factor);
        let y_offset = self.calculate_y_offset(interval_pos_factor);
        let level = self.level_curve.get_value(interval_pos_factor) / 100.0;
        let radius1 = self.calculate_radial_size(interval_pos_factor);
        let radius = radius1 / 2.0 / self.circle_count;
        let current_radius = radius;
        let edge_width = self.calculate_edge_width(interval_pos_factor, current_radius);
        let mut barht = self.max_buffer as f64 / self.circle_count;
        if matches!(self.fill, Fill::Empty | Fill::Fade) {
            barht /= self.circle_count;
        }
        if barht < 1.0 {
            barht = 1.0;
        }
        let block_ht = self.colors.len() as f64 * barht;
        let foffset = frame as f64 * self.calculate_center_speed(interval_pos_factor, info) / 4.0 % (block_ht + 1.0);
        barht = if barht > 0.0 { barht } else { 1.0 };
        FrameState { level, interval_pos, radius, radius1, current_radius, foffset, barht, block_ht, x_offset, y_offset, edge_width }
    }

    /// `SetFramePixel`: true when the pixel is finished.
    #[allow(clippy::too_many_arguments)]
    fn set_frame_pixel<B: PixelBuffer>(&self, i: i32, s: &FrameState, buf: &mut B, x_coord: i32, y_coord: i32, distance: i32, radius: f64) -> bool {
        if (distance as f64) <= radius && (distance as f64) >= radius - s.current_radius {
            let n = i as f64 - s.foffset + s.block_ht;
            let color_index = (n % s.block_ht / s.barht) as i32;
            // An index outside the list is the C# exception; nothing is drawn.
            let Some(gradient) = (if color_index >= 0 { self.colors.get(color_index as usize) } else { None }) else { return true };
            let mut color = gradient.get_color_at(s.interval_pos);
            match self.fill {
                Fill::GradientOverElement => {
                    color = gradient.get_color_at((1.0 / s.radius1) * radius);
                }
                Fill::Empty => {
                    if !((distance as f64) >= radius - s.edge_width) {
                        return false;
                    }
                }
                Fill::Fade => {
                    let mut hsv = Hsv::from_color(color);
                    hsv.set_v(hsv.v * (1.0 - distance as f64 / radius * s.level));
                    buf.set_pixel_hsv(x_coord, y_coord, hsv);
                    return true;
                }
                Fill::GradientOverTime => {}
            }
            if s.level < 1.0 {
                let mut hsv = Hsv::from_color(color);
                hsv.set_v(hsv.v * s.level);
                color = hsv.to_color();
            }
            buf.set_pixel(x_coord, y_coord, color);
            return true;
        }
        false
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

        // Skip pixels that cannot be inside the circles. Note the C# uses the width for both axes.
        let mid = ((wi - 1) / 2) as f64;
        if (y as f64 >= mid + s.radius1 + 1.0 || y as f64 <= mid - s.radius1) && (x as f64 >= mid + s.radius1 + 1.0 || x as f64 <= mid - s.radius1) {
            return;
        }

        let distance_from_center = distance_from_point((wi - 1) / 2, (ht - 1) / 2, x + s.x_offset, y + s.y_offset);
        let distance = if distance_from_center > 1.4 && distance_from_center < 1.51 { 2 } else { distance_from_center.round_ties_even() as i32 };

        let mut radius = s.radius * self.circle_count;
        match self.radial {
            Radial::In => {
                let mut i = 0;
                while (i as f64) <= self.circle_count {
                    if self.set_frame_pixel(i, s, buf, x_coord, y_coord, distance, radius) {
                        return;
                    }
                    radius -= s.current_radius;
                    i += 1;
                }
            }
            Radial::Out => {
                let mut i = self.circle_count as i32;
                while i >= 0 {
                    if self.set_frame_pixel(i, s, buf, x_coord, y_coord, distance, radius) {
                        return;
                    }
                    radius -= s.current_radius;
                    i -= 1;
                }
            }
        }
    }
}

impl PixelEffect for Circles {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.max_buffer = (info.buffer_ht() - 1).max(info.buffer_wi() - 1);
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        // With no colours the C# throws on `Colors[0]` as soon as a pixel is drawn.
        if self.colors.is_empty() {
            return;
        }
        let s = self.frame_state(frame, info);
        let interval_pos_factor = s.interval_pos * 100.0;
        self.circle_count = self.calculate_circle_count(interval_pos_factor);
        if matches!(self.fill, Fill::Fade | Fill::Empty) {
            self.circle_count /= 5.0;
        }
        for y in 0..info.buffer_ht() {
            for x in 0..info.buffer_wi() {
                self.calculate_pixel(x, y, &s, info, buf);
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        if self.colors.is_empty() {
            return;
        }
        let mut nodes = buf.element_locations().to_vec();
        nodes.sort_by_key(|l| (l.x, l.y));
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let s = self.frame_state(frame, info);
            let interval_pos_factor = s.interval_pos * 100.0;
            // The location path halves the count as well.
            self.circle_count = self.calculate_circle_count(interval_pos_factor) / 2.0;
            if matches!(self.fill, Fill::Fade | Fill::Empty) {
                self.circle_count /= 5.0;
            }
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
    fn edge_width_curve_only_shows_for_empty_fill() {
        let mut p = DESCRIPTOR.new_params();
        let shown = |p: &Params| DESCRIPTOR.visible_params(p).into_iter().any(|i| DESCRIPTOR.params[i].name == "CircleEdgeWidthCurve");
        assert!(!shown(&p));
        p.set("CircleFill", ParamValue::Enum("Empty".to_string()));
        assert!(shown(&p));
    }

    fn circles(fill: Fill) -> Circles {
        Circles {
            fill,
            radial: Radial::Out,
            colors: vec![ColorGradient::solid(Color::WHITE)],
            center_speed_curve: Curve::flat(0.0),
            circle_count_curve: Curve::flat(70.0),
            edge_width_curve: Curve::flat(0.0),
            level_curve: Curve::flat100(),
            size_curve: Curve::flat100(),
            x_offset_curve: Curve::flat(50.0),
            y_offset_curve: Curve::flat(50.0),
            locations: false,
            circle_count: 0.0,
            max_buffer: 0,
        }
    }

    #[test]
    fn first_frame_is_dark_because_the_circle_count_starts_at_zero() {
        // The C# reads `_circleCount` (0 on the first frame) before assigning it, which makes the radius
        // infinite and the ring test NaN, so nothing is drawn on frame 0.
        let info = crate::test_util::info(9, 9, 1000);
        let mut e = circles(Fill::GradientOverTime);
        e.setup_render(&info);
        let mut buf = PixelFrameBuffer::new(9, 9);
        buf.clear_buffer();
        e.render_effect(0, &info, &mut buf);
        let lit = |b: &PixelFrameBuffer| (0..9).flat_map(|x| (0..9).map(move |y| (x, y))).filter(|&(x, y)| b.get_color_at(x, y) == Color::WHITE).count();
        assert_eq!(lit(&buf), 0);
        // Frame 1 uses the circle count frame 0 produced, so rings appear.
        buf.clear_buffer();
        e.render_effect(1, &info, &mut buf);
        assert!(lit(&buf) > 0);
    }

    #[test]
    fn circle_count_is_floored_and_scaled() {
        let mut e = circles(Fill::Fade);
        e.max_buffer = 20;
        // Curve 70 maps to 1 + 0.7 * (10 - 1) = 7.3, truncated to 7.
        assert_eq!(e.calculate_circle_count(0.0), 7.0);
    }
}
