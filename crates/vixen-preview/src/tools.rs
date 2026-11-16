//! Port of the geometry in Shapes/PreviewTools.cs, plus the WPF transforms the shapes rely on.

use std::f64::consts::PI;

/// `Math.Round(x)`.
#[inline]
pub fn round(x: f64) -> i32 {
    x.round_ties_even() as i32
}

/// `Convert.ToInt32(x)`. It rounds, unlike a cast, and a NaN throws in C#.
#[inline]
pub fn to_int32(x: f64) -> i32 {
    x.round_ties_even() as i32
}

/// `System.Windows.Media.Matrix`, restricted to what RotateTransform and ScaleTransform build.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Matrix {
    pub m11: f64,
    pub m12: f64,
    pub m21: f64,
    pub m22: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Matrix {
    /// `new RotateTransform(angle, centerX, centerY).Value`.
    pub fn rotate_at(angle: f64, cx: f64, cy: f64) -> Matrix {
        let angle = angle % 360.0; // Matrix.RotateAt does the modulo before converting to radians
        let rad = angle * (PI / 180.0);
        let sin = rad.sin();
        let cos = rad.cos();
        let dx = (cx * (1.0 - cos)) + (cy * sin);
        let dy = (cy * (1.0 - cos)) - (cx * sin);
        Matrix { m11: cos, m12: sin, m21: -sin, m22: cos, offset_x: dx, offset_y: dy }
    }

    /// `Matrix.MultiplyPoint`, in the same order of operations.
    #[inline]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let xadd = y * self.m21 + self.offset_x;
        let yadd = x * self.m12 + self.offset_y;
        (x * self.m11 + xadd, y * self.m22 + yadd)
    }

    /// `Transform.TransformBounds` for a rectangle given as (x, y, width, height).
    pub fn transform_bounds(&self, r: DRect) -> DRect {
        let corners = [
            self.apply(r.x, r.y),
            self.apply(r.x + r.width, r.y),
            self.apply(r.x, r.y + r.height),
            self.apply(r.x + r.width, r.y + r.height),
        ];
        let mut min = corners[0];
        let mut max = corners[0];
        for c in corners {
            min.0 = min.0.min(c.0);
            min.1 = min.1.min(c.1);
            max.0 = max.0.max(c.0);
            max.1 = max.1.max(c.1);
        }
        DRect { x: min.0, y: min.1, width: max.0 - min.0, height: max.1 - min.1 }
    }
}

/// `new ScaleTransform(scaleX, scaleY, centerX, centerY)` applied to a point.
#[inline]
pub fn scale_at(x: f64, y: f64, sx: f64, sy: f64, cx: f64, cy: f64) -> (f64, f64) {
    (x * sx + (cx - sx * cx), y * sy + (cy - sy * cy))
}

/// `System.Windows.Rect`.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct DRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl DRect {
    #[inline]
    pub fn right(&self) -> f64 {
        self.x + self.width
    }
    #[inline]
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
    /// `Rect.Contains`.
    #[inline]
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x - self.width <= self.x && y >= self.y && y - self.height <= self.y
    }
}

/// `System.Drawing.Rectangle`, for the rubber band and shape bounds.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    #[inline]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect { x, y, width, height }
    }

    /// `Rectangle.Contains`: the right and bottom edges are outside.
    #[inline]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.x <= x && x < self.x.wrapping_add(self.width) && self.y <= y && y < self.y.wrapping_add(self.height)
    }

    /// The (x1, y1, x2, y2) the shapes normalise a rectangle to.
    #[inline]
    pub fn normalized(&self) -> (i32, i32, i32, i32) {
        let x2 = self.x.wrapping_add(self.width);
        let y2 = self.y.wrapping_add(self.height);
        (self.x.min(x2), self.y.min(y2), self.x.max(x2), self.y.max(y2))
    }
}

/// `PreviewWindowBounds.IsRecoverable`: can a saved window go back where it was? It has to have a size and an
/// upper corner on a screen.
pub fn window_is_recoverable(window: Rect, working_areas: &[Rect]) -> bool {
    if window.width <= 0 || window.height <= 0 {
        return false;
    }
    let right = window.x.wrapping_add(window.width).wrapping_sub(1);
    working_areas.iter().any(|area| area.contains(window.x, window.y) || area.contains(right, window.y))
}

/// `PreviewTools.GetArcPoints`.
pub fn get_arc_points(width: f64, height: f64, num_points: f64) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    if num_points.is_nan() || num_points <= 0.0 {
        return points;
    }
    points.reserve(num_points as usize);
    let c_x = width / 2.0;
    let c_y = height;
    let radian_increment = PI / (num_points - 1.0);
    let mut t = PI;
    while (points.len() as f64) < num_points {
        let x = c_x + (width / 2.0) * t.cos();
        let y = c_y + height * t.sin();
        points.push((x as i32, y as i32));
        t += radian_increment;
    }
    points
}

/// `PreviewTools.GetEllipsePoints`. A zero or negative increment loops forever in C#; here it yields nothing.
pub fn get_ellipse_points(
    left_offset: f64,
    top_offset: f64,
    width: f64,
    height: f64,
    total_points: f64,
    degrees: f64,
    degree_offset: f64,
) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    let total_radians = (degrees * PI) / 180.0;
    let num_points = total_points / 2.0;
    let c_x = width / 2.0;
    let c_y = height / 2.0;
    let radian_offset = (degree_offset * PI) / 180.0;
    let start_radian = radian_offset;
    let end_radian = total_radians + radian_offset;
    let radian_increment = if degrees <= 180.0 { PI / (num_points - 1.0) } else { (PI * 2.0) / total_points };
    if radian_increment.is_nan() || radian_increment <= 0.0 {
        return points;
    }
    // watch out for rounding on the fp adds
    let limit = end_radian + radian_increment / 10.0;
    let mut t = start_radian;
    while t < limit {
        let x = (c_x + (width / 2.0) * t.cos()) + left_offset;
        let y = (c_y + (height / 2.0) * t.sin()) + top_offset;
        points.push((x as i32, y as i32));
        t += radian_increment;
    }
    points
}

/// `PreviewTools.GetAngle`, with its argument order: `screen` is the rotation axis, `center` the mouse.
pub fn get_angle(screen: (i32, i32), center: (i32, i32)) -> i32 {
    let dx = screen.0.wrapping_sub(center.0);
    let dy = screen.1.wrapping_sub(center.1);
    let in_rads = (dy as f64).atan2(dx as f64);
    // Convert from radians and adjust the angle from the 0 at 9:00 position.
    ((in_rads * (180.0 / PI) + 270.0) as i32) % 360
}

/// `PreviewTools.CalculatePointOnLine`. The C# `operator -` on Vector2 is b - a, which this reproduces.
pub fn calculate_point_on_line(a: (i32, i32), b: (i32, i32), distance: i32) -> (i32, i32) {
    let vx = b.0 as f64 - a.0 as f64;
    let vy = b.1 as f64 - a.1 as f64;
    let len = (vx.powi(2) + vy.powi(2)).sqrt();
    let (ux, uy) = (vx / len, vy / len);
    let d = distance as f64;
    // Vector2 + Vector2 adds the other way round, which changes nothing, and the Point cast truncates.
    ((ux * d + a.0 as f64) as i32, (uy * d + a.1 as f64) as i32)
}

/// `PreviewTools.TriangleHypotenuse`.
#[inline]
pub fn triangle_hypotenuse(a: f64, b: f64) -> f64 {
    (a.abs().powi(2) + b.abs().powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_is_recoverable_with_an_upper_corner_on_a_screen() {
        let screens = [Rect::new(0, 0, 1000, 800), Rect::new(1000, 0, 500, 500)];
        assert!(window_is_recoverable(Rect::new(10, 10, 100, 100), &screens));
        // Only the upper right corner is on a screen.
        assert!(window_is_recoverable(Rect::new(-50, 10, 100, 100), &screens));
        // The right edge is one past the window, and a working area's far edge is outside it.
        assert!(!window_is_recoverable(Rect::new(-100, 10, 100, 100), &screens));
        assert!(window_is_recoverable(Rect::new(-99, 10, 100, 100), &screens));
        assert!(!window_is_recoverable(Rect::new(1200, 500, 100, 100), &screens));
        assert!(!window_is_recoverable(Rect::new(10, 10, 0, 100), &screens), "no size");
        assert!(!window_is_recoverable(Rect::new(10, 10, 100, -1), &screens));
        assert!(!window_is_recoverable(Rect::new(10, 10, 100, 100), &[]));
        assert!(!window_is_recoverable(Rect::new(i32::MAX, i32::MIN, i32::MAX, 1), &screens), "must not overflow");
    }

    #[test]
    fn rotation_matrix_matches_wpf() {
        let m = Matrix::rotate_at(90.0, 10.0, 10.0);
        let (x, y) = m.apply(20.0, 10.0);
        assert!((x - 10.0).abs() < 1e-9 && (y - 20.0).abs() < 1e-9, "{x},{y}");
    }

    #[test]
    fn arc_points_truncate_and_run_left_over_top_to_right() {
        // Width 100, height 50, 3 points: t = pi, 3pi/2, 2pi. X = 50 + 50 cos t, Y = 50 + 50 sin t. Floating
        // point puts cos(3pi/2) and sin(2pi) just below zero, and the cast then truncates to 49.
        assert_eq!(get_arc_points(100.0, 50.0, 3.0), vec![(0, 50), (49, 0), (100, 49)]);
    }

    #[test]
    fn arc_with_one_point_has_no_divide_problem() {
        assert_eq!(get_arc_points(10.0, 10.0, 1.0), vec![(0, 10)]);
        assert!(get_arc_points(10.0, 10.0, 0.0).is_empty());
    }

    #[test]
    fn ellipse_points_include_the_closing_point() {
        // 4 points over 360 degrees of a 100x100 circle, plus the repeat at 2pi from the loop's tolerance.
        let p = get_ellipse_points(0.0, 0.0, 100.0, 100.0, 4.0, 360.0, 0.0);
        assert_eq!(p, vec![(100, 50), (50, 100), (0, 50), (49, 0), (100, 49)]);
    }

    #[test]
    fn ellipse_with_no_points_does_not_hang() {
        assert!(get_ellipse_points(0.0, 0.0, 10.0, 10.0, 0.0, 180.0, 0.0).is_empty());
        assert_eq!(get_ellipse_points(0.0, 0.0, 10.0, 10.0, 0.0, 360.0, 0.0).len(), 1);
    }

    #[test]
    fn angle_is_measured_with_the_csharp_argument_order() {
        // Mouse directly above the axis: dy = +10, dx = 0, atan2 = 90 degrees, +270 = 360, % 360 = 0.
        assert_eq!(get_angle((50, 50), (50, 40)), 0);
        // Mouse to the right: dx = -10, dy = 0, atan2 = 180, +270 = 450, % 360 = 90.
        assert_eq!(get_angle((50, 50), (60, 50)), 90);
    }

    #[test]
    fn point_on_line_truncates() {
        assert_eq!(calculate_point_on_line((0, 0), (3, 4), 5), (3, 4));
        assert_eq!(calculate_point_on_line((0, 0), (3, 4), 2), (1, 1));
    }
}

/// `IsBackgroundAllBlack`: every pixel exactly opaque black. A pixel with no alpha is not, because the C# compares the whole
/// ARGB. Stops at the first other pixel, so a real image costs almost nothing. `rgba` is tightly packed.
pub fn is_all_black(rgba: &[u8]) -> bool {
    rgba.chunks_exact(4).all(|p| p == [0, 0, 0, 255])
}
