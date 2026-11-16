//! Port of VixenModules.App.Curves.Curve and ZedGraph's PointPairList.InterpolateX.

/// A curve maps 0..100 on X to 0..100 on Y.
#[derive(Clone, PartialEq, Debug)]
pub struct Curve {
    pub points: Vec<(f64, f64)>,
    pub library_reference_name: String,
    pub is_current_library_curve: bool,
}

impl Default for Curve {
    /// `new Curve()` is a ramp with x = y.
    fn default() -> Self {
        Curve::ramp_up()
    }
}

impl Curve {
    fn with_points(points: Vec<(f64, f64)>) -> Curve {
        Curve { points, library_reference_name: String::new(), is_current_library_curve: false }
    }

    pub fn from_points(points: Vec<(f64, f64)>) -> Curve {
        Curve::with_points(points)
    }

    pub fn ramp_up() -> Curve {
        Curve::with_points(vec![(0.0, 0.0), (100.0, 100.0)])
    }

    pub fn ramp_down() -> Curve {
        Curve::with_points(vec![(0.0, 100.0), (100.0, 0.0)])
    }

    pub fn flat100() -> Curve {
        Curve::with_points(vec![(0.0, 100.0), (100.0, 100.0)])
    }

    /// `new Curve(double intensity)`: a flat curve at the given level.
    pub fn flat(intensity: f64) -> Curve {
        Curve::with_points(vec![(0.0, intensity), (100.0, intensity)])
    }

    pub fn is_library_reference(&self) -> bool {
        !self.library_reference_name.is_empty()
    }

    /// `PointPairList.Sort()`: ascending by X, stable.
    pub fn sort_points(&mut self) {
        self.points.sort_by(|a, b| a.0.total_cmp(&b.0));
    }

    /// `Curve.GetValue(double)`.
    pub fn get_value(&self, x: f64) -> f64 {
        let x = if x > 100.0 {
            100.0
        } else if x < 0.0 {
            0.0
        } else {
            x
        };
        let mut v = self.interpolate_x(x);
        if v > 100.0 {
            v = 100.0;
        }
        if v < 0.0 {
            v = 0.0;
        }
        if v.is_nan() {
            v = 100.0;
        }
        v
    }

    /// `Curve.GetIntValue(double)`, with C# banker's rounding.
    pub fn get_int_value(&self, x: f64) -> i32 {
        self.get_value(x).round_ties_even() as i32
    }

    /// `PointPairList.InterpolateX`. The C# throws for fewer than two points; a damaged file
    /// must not take the app down, so that case returns NaN and `get_value` maps it to 100
    /// as it does for any other NaN.
    fn interpolate_x(&self, x_target: f64) -> f64 {
        let p = &self.points;
        let count = p.len();
        if count < 2 {
            return f64::NAN;
        }
        let (lo, hi);
        if x_target <= p[0].0 {
            lo = 0;
            hi = 1;
        } else if x_target >= p[count - 1].0 {
            lo = count - 2;
            hi = count - 1;
        } else {
            let mut l = 0usize;
            let mut h = count - 1;
            let mut j = 0;
            while j < 1000 && h > l + 1 {
                let mid = (h + l) / 2;
                if x_target > p[mid].0 {
                    l = mid;
                } else {
                    h = mid;
                }
                j += 1;
            }
            lo = l;
            hi = h;
        }
        if p[hi].1 == p[lo].1 {
            return p[hi].1;
        }
        (x_target - p[lo].0) / (p[hi].0 - p[lo].0) * (p[hi].1 - p[lo].1) + p[lo].1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ramp() {
        let c = Curve::default();
        assert_eq!(c.get_value(0.0), 0.0);
        assert_eq!(c.get_value(50.0), 50.0);
        assert_eq!(c.get_value(100.0), 100.0);
        assert_eq!(c.get_value(150.0), 100.0);
        assert_eq!(c.get_value(-5.0), 0.0);
    }

    #[test]
    fn interpolates_between_points() {
        let c = Curve::from_points(vec![(0.0, 0.0), (50.0, 100.0), (100.0, 0.0)]);
        assert_eq!(c.get_value(25.0), 50.0);
        assert_eq!(c.get_value(75.0), 50.0);
    }

    #[test]
    fn identical_y_never_produces_nan() {
        let c = Curve::from_points(vec![(50.0, 30.0), (50.0, 30.0)]);
        assert_eq!(c.get_value(50.0), 30.0);
    }

    #[test]
    fn int_value_uses_bankers_rounding() {
        assert_eq!(Curve::flat(2.5).get_int_value(10.0), 2);
        assert_eq!(Curve::flat(3.5).get_int_value(10.0), 4);
    }

    #[test]
    fn degenerate_curve_does_not_panic() {
        assert_eq!(Curve::from_points(vec![(0.0, 10.0)]).get_value(5.0), 100.0);
    }
}
