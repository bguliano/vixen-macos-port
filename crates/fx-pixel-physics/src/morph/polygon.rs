//! The parts of `VixenModules.App.Polygon` rendering needs: shapes, moving them and scaling them.

use crate::generated::morph as gen;
use vixen_effects::prelude::*;

/// `PolygonPoint`.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct PolygonPoint {
    pub x: f64,
    pub y: f64,
}

impl PolygonPoint {
    pub fn new(x: f64, y: f64) -> PolygonPoint {
        PolygonPoint { x, y }
    }

    fn from_record(r: &Params) -> PolygonPoint {
        PolygonPoint::new(r.double("X"), r.double("Y"))
    }
}

/// `Ellipse`: the rectangle around it in `points`, and the values it is drawn from.
#[derive(Clone, PartialEq, Debug)]
pub struct Ellipse {
    pub points: Vec<PolygonPoint>,
    pub angle: f64,
    pub center: PolygonPoint,
    pub width: f64,
    pub height: f64,
}

/// `RotateTransform(angle, centerX, centerY)` from WPF, with its `Matrix.MultiplyPoint` evaluation order.
struct Rotation {
    sin: f64,
    cos: f64,
    dx: f64,
    dy: f64,
}

impl Rotation {
    fn new(angle: f64, center: PolygonPoint) -> Rotation {
        let radians = (angle % 360.0) * (std::f64::consts::PI / 180.0);
        let (sin, cos) = (radians.sin(), radians.cos());
        Rotation { sin, cos, dx: center.x * (1.0 - cos) + center.y * sin, dy: center.y * (1.0 - cos) - center.x * sin }
    }

    fn apply(&self, p: &mut PolygonPoint) {
        let x_add = p.y * -self.sin + self.dx;
        let y_add = p.x * self.sin + self.dy;
        p.x = p.x * self.cos + x_add;
        p.y = p.y * self.cos + y_add;
    }
}

/// How many times `Ellipse.Scale` will try to fit an ellipse. It stops earlier once a pass changes nothing.
const MAX_FIT_PASSES: i32 = 2_000_000;

impl Ellipse {
    /// `Ellipse.Scale`.
    pub fn scale(&mut self, x_scale: f64, y_scale: f64, width: i32, height: i32) {
        self.scale_internal(x_scale, y_scale);
        let mut passes = 0;
        let mut previous = Vec::new();
        while self.is_outside_limits(width, height) && passes < MAX_FIT_PASSES {
            previous.clone_from(&self.points);
            // Deliberate: `(height - 1) / height` is integer division in the C#, so the height factor is zero and
            // the first pass flattens the ellipse onto y = 0.
            let height_factor = if height != 0 { (height - 1) / height } else { 0 };
            self.scale_internal((width as f64 - 1.0) / width as f64, height_factor as f64);
            // A rotated ellipse shrinks towards nothing and can get stuck among denormal numbers with a tiny negative
            // y. From there the C# loops forever. Stopping leaves an ellipse too small to draw.
            if self.points == previous {
                break;
            }
            passes += 1;
        }
    }

    /// `ScaleInternal`.
    fn scale_internal(&mut self, x_scale: f64, y_scale: f64) {
        let reverse = Rotation::new(-self.angle, self.center);
        for pt in &mut self.points {
            reverse.apply(pt);
        }
        for pt in &mut self.points {
            pt.x *= x_scale;
            pt.y *= y_scale;
        }
        self.center.x *= x_scale;
        self.center.y *= y_scale;
        self.width *= x_scale;
        self.height *= y_scale;
        let forward = Rotation::new(self.angle, self.center);
        for pt in &mut self.points {
            forward.apply(pt);
        }
    }

    /// `IsOutsideLimits`.
    fn is_outside_limits(&self, width: i32, height: i32) -> bool {
        let (max_x, max_y) = ((width - 1) as f64, (height - 1) as f64);
        self.points.iter().any(|p| p.x > max_x || p.y > max_y || p.x < 0.0 || p.y < 0.0)
    }
}

/// What a `MorphPolygon` holds. In the C# these are three nullable properties that clear each other.
#[derive(Clone, PartialEq, Debug)]
pub enum Shape {
    Polygon(Vec<PolygonPoint>),
    Line(Vec<PolygonPoint>),
    Ellipse(Ellipse),
}

impl Shape {
    /// The polygon `new MorphPolygon()` creates: `width` x `height` at the origin.
    pub fn rectangle(width: i32, height: i32) -> Shape {
        let (w, h) = ((width - 1) as f64, (height - 1) as f64);
        let corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
        Shape::Polygon(corners.iter().map(|c| PolygonPoint::new(c.0, c.1)).collect())
    }

    /// The shape of a `MorphPolygonData`, the first of its members that is set, as `UpdatePolygonModel` reads them.
    pub fn from_record(record: &Params) -> Option<Shape> {
        let points = |r: &Params| r.records("Points").iter().map(PolygonPoint::from_record).collect();
        if let Some(r) = record.record("Polygon") {
            Some(Shape::Polygon(points(r)))
        } else if let Some(r) = record.record("Line") {
            Some(Shape::Line(points(r)))
        } else {
            record.record("Ellipse").map(|r| {
                Shape::Ellipse(Ellipse {
                    points: points(r),
                    angle: r.double("Angle"),
                    center: r.record("Center").map(PolygonPoint::from_record).unwrap_or_default(),
                    width: r.double("Width"),
                    height: r.double("Height"),
                })
            })
        }
    }

    /// `GetPolygonPoints`.
    pub fn points(&self) -> &[PolygonPoint] {
        match self {
            Shape::Polygon(p) | Shape::Line(p) => p,
            Shape::Ellipse(e) => &e.points,
        }
    }

    /// `PointBasedShape.MovePoints`.
    pub fn move_points(&mut self, x: i32, y: i32) {
        let points = match self {
            Shape::Polygon(p) | Shape::Line(p) => p,
            Shape::Ellipse(e) => &mut e.points,
        };
        for pt in points {
            pt.x += x as f64;
            pt.y += y as f64;
        }
    }

    /// `Scale`, which only the ellipse overrides.
    pub fn scale(&mut self, x_scale: f64, y_scale: f64, width: i32, height: i32) {
        match self {
            Shape::Polygon(p) | Shape::Line(p) => {
                for pt in p {
                    pt.x *= x_scale;
                    pt.y *= y_scale;
                }
            }
            Shape::Ellipse(e) => e.scale(x_scale, y_scale, width, height),
        }
    }

    /// Writes the points, and an ellipse's values, back into the record this shape was read from.
    pub fn store_in(&self, record: &mut Params) {
        let points: Vec<Params> = self.points().iter().map(|pt| gen::point_record(pt.x, pt.y)).collect();
        record.set("Points", ParamValue::RecordList(points));
        if let Shape::Ellipse(e) = self {
            record.set("Center", ParamValue::Record(Box::new(gen::point_record(e.center.x, e.center.y))));
            record.set("Width", ParamValue::Double(e.width));
            record.set("Height", ParamValue::Double(e.height));
        }
    }

    /// Whether the coordinates are something a drawing could use. A damaged file can hold anything, and the
    /// integer arithmetic downstream assumes they fit.
    pub fn is_sane(&self) -> bool {
        let fits = |v: f64| v.abs() < 1e6;
        let ellipse = match self {
            Shape::Ellipse(e) => fits(e.center.x) && fits(e.center.y) && fits(e.width) && fits(e.height) && fits(e.angle),
            _ => true,
        };
        ellipse && self.points().iter().all(|p| fits(p.x) && fits(p.y))
    }
}

/// A `Polygon` record, for tests and for anything building shapes in code.
pub fn polygon_record(points: &[(f64, f64)]) -> Params {
    shape_record(&gen::POLYGON_PARAMS, points)
}

/// A `Line` record.
pub fn line_record(points: &[(f64, f64)]) -> Params {
    shape_record(&gen::LINE_PARAMS, points)
}

fn shape_record(desc: &'static [ParamDesc], points: &[(f64, f64)]) -> Params {
    let mut r = Params::defaults(desc);
    r.set("Points", ParamValue::RecordList(points.iter().map(|p| gen::point_record(p.0, p.1)).collect()));
    r
}

/// An `Ellipse` record with the given bounding rectangle corners.
pub fn ellipse_record(points: &[(f64, f64)], center: (f64, f64), width: f64, height: f64, angle: f64) -> Params {
    let mut r = shape_record(&gen::ELLIPSE_PARAMS, points);
    r.set("Center", ParamValue::Record(Box::new(gen::point_record(center.0, center.1))));
    r.set("Width", ParamValue::Double(width));
    r.set("Height", ParamValue::Double(height));
    r.set("Angle", ParamValue::Double(angle));
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ellipse(points: &[(f64, f64)], center: (f64, f64), width: f64, height: f64, angle: f64) -> Ellipse {
        Ellipse {
            points: points.iter().map(|p| PolygonPoint::new(p.0, p.1)).collect(),
            angle,
            center: PolygonPoint::new(center.0, center.1),
            width,
            height,
        }
    }

    #[test]
    fn scaling_an_unrotated_ellipse_scales_everything() {
        let mut e = ellipse(&[(2.0, 2.0), (6.0, 2.0), (6.0, 4.0), (2.0, 4.0)], (4.0, 3.0), 4.0, 2.0, 0.0);
        e.scale(2.0, 3.0, 100, 100);
        assert_eq!(e.points[2], PolygonPoint::new(12.0, 12.0));
        assert_eq!(e.center, PolygonPoint::new(8.0, 9.0));
        assert_eq!((e.width, e.height), (8.0, 6.0));
    }

    #[test]
    fn an_ellipse_that_no_longer_fits_is_flattened() {
        // The C# quirk: fitting multiplies y by the integer (height - 1) / height, which is zero.
        let mut e = ellipse(&[(2.0, 2.0), (16.0, 2.0), (16.0, 8.0), (2.0, 8.0)], (9.0, 5.0), 14.0, 6.0, 0.0);
        e.scale(1.0, 1.0, 15, 15);
        assert_eq!(e.height, 0.0);
        assert_eq!(e.center.y, 0.0);
        assert!(e.points.iter().all(|p| p.y == 0.0 && p.x <= 14.0));
        // Width shrinks by (width - 1) / width on each pass: two of them bring x = 16 under 14.
        assert_eq!(e.width, 14.0 * (14.0 / 15.0) * (14.0 / 15.0));
    }

    #[test]
    fn fitting_a_rotated_ellipse_ends_where_the_csharp_would_hang() {
        let mut e = ellipse(&[(-1.0, 3.0), (9.0, 3.0), (9.0, 7.0), (-1.0, 7.0)], (4.0, 5.0), 10.0, 4.0, 30.0);
        e.scale(1.0, 1.0, 20, 20);
        assert_eq!(e.width as i32, 0, "nothing left to draw");
        assert_eq!(e.height, 0.0);
    }

    #[test]
    fn rotation_matches_wpf() {
        // RotateTransform(90, 1, 1) sends (2, 1) to (1, 2): positive angles turn +x towards +y.
        let mut p = PolygonPoint::new(2.0, 1.0);
        Rotation::new(90.0, PolygonPoint::new(1.0, 1.0)).apply(&mut p);
        assert!((p.x - 1.0).abs() < 1e-12 && (p.y - 2.0).abs() < 1e-12, "{p:?}");
    }

    #[test]
    fn the_first_shape_set_wins() {
        let mut r = Params::defaults(&gen::MORPH_POLYGON_PARAMS);
        assert!(matches!(Shape::from_record(&r), Some(Shape::Polygon(p)) if p.len() == 4));
        r.set("Polygon", ParamValue::None);
        assert_eq!(Shape::from_record(&r), None);
        r.set("Line", ParamValue::Record(Box::new(line_record(&[(0.0, 0.0), (3.0, 4.0)]))));
        assert!(matches!(Shape::from_record(&r), Some(Shape::Line(p)) if p.len() == 2));
    }

    #[test]
    fn insane_coordinates_are_refused() {
        assert!(Shape::rectangle(5, 5).is_sane());
        assert!(!Shape::Line(vec![PolygonPoint::new(f64::NAN, 0.0)]).is_sane());
        assert!(!Shape::Line(vec![PolygonPoint::new(1e12, 0.0)]).is_sane());
    }
}
