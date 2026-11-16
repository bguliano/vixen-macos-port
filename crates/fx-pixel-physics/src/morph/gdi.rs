//! GDI+ aliased drawing, for the `Graphics` calls Morph makes on a fresh bitmap: `FillPolygon`, `DrawPolygon`,
//! `DrawLine`, `FillEllipse` and `DrawEllipse` with a one pixel pen.
//!
//! These are local rather than `vixen_effects::gfx::Graphics` because that canvas differs from GDI+ in the ways
//! this effect shows: its polygons fill by winding where `FillPolygon` defaults to `FillMode.Alternate`, its lines
//! stop short of their last pixel, and its pixel centres are half a pixel away from GDI+'s, which with the default
//! `PixelOffsetMode` sit on integer coordinates.
//!
//! The rules here: a pixel is filled when its integer centre is inside the shape, with left and top edges in and
//! right and bottom edges out. Lines include both end points, except that a zero length line draws nothing.
//! Vertices snap to the 1/16 pixel grid GDI+ rasterizes on.

/// `DrawLine` with a one pixel pen.
pub fn draw_line(x0: i32, y0: i32, x1: i32, y1: i32, plot: &mut impl FnMut(i32, i32)) {
    if x0 == x1 && y0 == y1 {
        return;
    }
    let (mut x, mut y) = (x0, y0);
    let (dx, sx) = ((x1 - x0).abs(), if x0 < x1 { 1 } else { -1 });
    let (dy, sy) = ((y1 - y0).abs(), if y0 < y1 { 1 } else { -1 });
    let mut err = (if dx > dy { dx } else { -dy }) / 2;
    loop {
        plot(x, y);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = err;
        if e2 > -dx {
            err -= dy;
            x += sx;
        }
        if e2 < dy {
            err += dx;
            y += sy;
        }
    }
}

/// `DrawPolygon` with a one pixel pen: every side, closing back to the first point.
pub fn draw_polygon(points: &[(i32, i32)], plot: &mut impl FnMut(i32, i32)) {
    for (i, a) in points.iter().enumerate() {
        let b = points[(i + 1) % points.len()];
        draw_line(a.0, a.1, b.0, b.1, plot);
    }
}

/// `FillPolygon` with `FillMode.Alternate`, clipped to `width` x `height`.
pub fn fill_polygon(points: &[(f64, f64)], width: i32, height: i32, plot: &mut impl FnMut(i32, i32)) {
    if points.len() < 3 {
        return;
    }
    let snapped: Vec<(f64, f64)> = points.iter().map(|p| (snap(p.0), snap(p.1))).collect();
    let min_y = snapped.iter().fold(f64::INFINITY, |m, p| m.min(p.1));
    let max_y = snapped.iter().fold(f64::NEG_INFINITY, |m, p| m.max(p.1));
    let first = (min_y.ceil() as i32).max(0);
    let last = (max_y.ceil() as i32).min(height);
    let mut crossings: Vec<f64> = Vec::with_capacity(8);
    for y in first..last {
        let sample = y as f64;
        crossings.clear();
        for (i, a) in snapped.iter().enumerate() {
            let b = snapped[(i + 1) % snapped.len()];
            let (top, bottom) = if a.1 <= b.1 { (*a, b) } else { (b, *a) };
            // Half open, so that a vertex shared by two edges only counts once.
            if top.1 <= sample && sample < bottom.1 {
                crossings.push(top.0 + (sample - top.1) * (bottom.0 - top.0) / (bottom.1 - top.1));
            }
        }
        crossings.sort_by(f64::total_cmp);
        for pair in crossings.as_chunks::<2>().0 {
            let from = (pair[0].ceil() as i32).max(0);
            let to = (pair[1].ceil() as i32).min(width);
            for x in from..to {
                plot(x, y);
            }
        }
    }
}

/// The ellipse Morph draws: `TranslateTransform(center)`, `RotateTransform(angle)`, then the rectangle
/// `((int)(-w / 2), (int)(-h / 2), (int)w, (int)h)`. The truncation is the C#'s, and it moves odd sized ellipses
/// half a pixel off their centre.
#[derive(Copy, Clone, Debug)]
pub struct EllipseDraw {
    pub center_x: f64,
    pub center_y: f64,
    pub width: f64,
    pub height: f64,
    pub angle: f64,
}

impl EllipseDraw {
    /// `steps` points around the ellipse, in bitmap coordinates.
    fn vertices(&self, steps: i32) -> Vec<(f64, f64)> {
        let rect_x = (-self.width / 2.0) as i32 as f64;
        let rect_y = (-self.height / 2.0) as i32 as f64;
        let (rect_w, rect_h) = ((self.width as i32) as f64, (self.height as i32) as f64);
        let (a, b) = (rect_w / 2.0, rect_h / 2.0);
        let (ox, oy) = (rect_x + a, rect_y + b);
        let (tx, ty) = ((self.center_x as i32) as f64, (self.center_y as i32) as f64);
        // GDI+ transforms are single precision.
        let radians = ((self.angle as f32) as f64).to_radians();
        let (sin, cos) = radians.sin_cos();
        (0..steps)
            .map(|i| {
                let t = i as f64 * std::f64::consts::TAU / steps as f64;
                let (lx, ly) = (ox + a * t.cos(), oy + b * t.sin());
                (tx + lx * cos - ly * sin, ty + lx * sin + ly * cos)
            })
            .collect()
    }

    fn largest_radius(&self) -> f64 {
        (self.width as i32).abs().max((self.height as i32).abs()) as f64 / 2.0
    }

    /// `FillEllipse`. The steps are fine enough that the outline stays within a tenth of a pixel of the curve.
    pub fn fill(&self, width: i32, height: i32, plot: &mut impl FnMut(i32, i32)) {
        if (self.width as i32) <= 0 || (self.height as i32) <= 0 {
            return;
        }
        let steps = (8.0 * self.largest_radius().ceil()).clamp(16.0, 4096.0) as i32;
        fill_polygon(&self.vertices(steps), width, height, plot);
    }

    /// `DrawEllipse` with a one pixel pen. GDI+ flattens the curve into short lines, and so does this, about
    /// two pixels long so that the result stays one pixel thick.
    pub fn outline(&self, plot: &mut impl FnMut(i32, i32)) {
        let steps = (4.0 * (std::f64::consts::PI * self.largest_radius() / 4.0).ceil()).clamp(8.0, 4096.0) as i32;
        let vertices = self.vertices(steps);
        let points: Vec<(i32, i32)> = vertices.iter().map(|p| (p.0.round() as i32, p.1.round() as i32)).collect();
        draw_polygon(&points, plot);
    }
}

#[inline]
fn snap(v: f64) -> f64 {
    (v * 16.0).round() / 16.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    type Pixels = BTreeSet<(i32, i32)>;

    fn line(x0: i32, y0: i32, x1: i32, y1: i32) -> Pixels {
        let mut set = Pixels::new();
        draw_line(x0, y0, x1, y1, &mut |x, y| {
            set.insert((x, y));
        });
        set
    }

    #[test]
    fn lines_include_both_end_points() {
        assert_eq!(line(1, 3, 4, 3), Pixels::from([(1, 3), (2, 3), (3, 3), (4, 3)]));
        assert_eq!(line(3, 1, 3, 3), Pixels::from([(3, 1), (3, 2), (3, 3)]));
        assert_eq!(line(0, 0, 2, 2), Pixels::from([(0, 0), (1, 1), (2, 2)]));
    }

    #[test]
    fn a_zero_length_line_draws_nothing() {
        assert!(line(5, 5, 5, 5).is_empty());
    }

    #[test]
    fn filled_rectangles_leave_out_their_right_and_bottom_edges() {
        let mut set = Pixels::new();
        fill_polygon(&[(1.0, 1.0), (4.0, 1.0), (4.0, 3.0), (1.0, 3.0)], 10, 10, &mut |x, y| {
            set.insert((x, y));
        });
        // 3 x 2, as `FillRectangle(1, 1, 3, 2)` is. Drawing the outline over it adds the rest.
        assert_eq!(set, Pixels::from([(1, 1), (2, 1), (3, 1), (1, 2), (2, 2), (3, 2)]));
    }

    #[test]
    fn self_intersecting_polygons_alternate() {
        // A pentagram. FillMode.Alternate leaves the pentagon where the star's arms overlap empty.
        let star: Vec<(f64, f64)> = (0..5)
            .map(|i| {
                let a = -std::f64::consts::FRAC_PI_2 + i as f64 * 2.0 * std::f64::consts::TAU / 5.0;
                (20.0 + 18.0 * a.cos(), 20.0 + 18.0 * a.sin())
            })
            .collect();
        let mut set = Pixels::new();
        fill_polygon(&star, 40, 40, &mut |x, y| {
            set.insert((x, y));
        });
        assert!(!set.contains(&(20, 20)), "the centre is wound twice");
        assert!(set.contains(&(20, 6)), "the top arm is wound once");
    }

    #[test]
    fn filling_clips_to_the_bitmap() {
        let mut set = Pixels::new();
        fill_polygon(&[(-5.0, -5.0), (50.0, -5.0), (50.0, 50.0), (-5.0, 50.0)], 3, 2, &mut |x, y| {
            set.insert((x, y));
        });
        assert_eq!(set.len(), 6);
    }

    fn circle(width: f64, height: f64, angle: f64) -> EllipseDraw {
        EllipseDraw { center_x: 10.0, center_y: 10.0, width, height, angle }
    }

    fn fill(e: EllipseDraw) -> Pixels {
        let mut set = Pixels::new();
        e.fill(30, 30, &mut |x, y| {
            set.insert((x, y));
        });
        set
    }

    fn extents(set: &Pixels) -> (i32, i32, i32, i32) {
        let xs = set.iter().map(|p| p.0);
        let ys = set.iter().map(|p| p.1);
        (xs.clone().min().unwrap(), xs.max().unwrap(), ys.clone().min().unwrap(), ys.max().unwrap())
    }

    #[test]
    fn an_even_ellipse_centres_on_its_pixel() {
        // Across, the left extreme lands on a pixel centre and counts, and the right one does not. Down, the top
        // extreme is a span of no width, so it draws nothing, and the bottom one is on an excluded edge.
        assert_eq!(extents(&fill(circle(8.0, 8.0, 0.0))), (6, 13, 7, 13));
    }

    #[test]
    fn an_odd_ellipse_shifts_with_the_truncated_rectangle() {
        // (int)(-5 / 2.0) is -2, so the rectangle runs from 8 to 13, not from 7.5 to 12.5, and its centre is 10.5.
        assert_eq!(extents(&fill(circle(5.0, 5.0, 0.0))), (9, 12, 9, 12));
    }

    #[test]
    fn rotation_is_clockwise_about_the_center() {
        // Wide and flat, turned a quarter: now tall and thin. A rotation by 90 degrees maps +x to +y.
        let flat = extents(&fill(circle(12.0, 2.0, 0.0)));
        let turned = extents(&fill(circle(12.0, 2.0, 90.0)));
        assert_eq!(flat, (4, 15, 9, 10));
        assert!(turned.3 - turned.2 >= 10, "{turned:?}");
        assert!(turned.1 - turned.0 <= 2, "{turned:?}");
    }

    #[test]
    fn empty_ellipses_do_not_fill() {
        assert!(fill(circle(0.0, 8.0, 0.0)).is_empty());
        assert!(fill(circle(8.0, 0.9, 0.0)).is_empty());
    }

    #[test]
    fn an_outline_rings_the_fill() {
        let mut set = Pixels::new();
        circle(8.0, 8.0, 0.0).outline(&mut |x, y| {
            set.insert((x, y));
        });
        // Unlike the fill, DrawEllipse reaches the far side: 6..=14 in both directions.
        assert_eq!(extents(&set), (6, 14, 6, 14));
        assert!(!set.contains(&(10, 10)));
    }
}
