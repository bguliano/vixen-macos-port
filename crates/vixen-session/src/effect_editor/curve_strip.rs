//! The inline curve row's gestures, F38 to F43: `BaseInlineCurveEditor`'s mouse handlers as pure functions over a copy held for one press.
//! Positions are in points against the 32 pt strip, y growing downwards.

use super::{Mods, StripCursor};
use vixen_core::Curve;

const DRAG_TOLERANCE: f64 = 2.0;
/// In curve units, so 8% of the width but only about 2.5 pt vertically.
const DISTANCE_TOLERANCE: f64 = 8.0;

/// `TransformPoint`: the points to draw in `width` by `height`, y downwards. The integer truncation is upstream's, and test 27 pins it.
pub fn curve_strip_points(points: &[(f64, f64)], width: f64, height: f64) -> Vec<(f32, f32)> {
    points
        .iter()
        .map(|&(x, y)| {
            let px = x as i32;
            let py = (y as i32 - 100).abs();
            let py = (py as f32 * (height as f32 / 100.0)) as i32;
            let px = (px as f32 * ((width as f32 - 1.0) / 100.0)) as i32;
            (px as f32, py as f32)
        })
        .collect()
}

/// `TranslateMouseLocation`.
fn to_curve_units(x: f64, y: f64, width: f64, height: f64) -> (f64, f64) {
    let width = width.max(1.0);
    let height = height.max(1.0);
    (100.0 * (x / width), 100.0 * ((height - y) / height))
}

/// `EnforcePointBounds`: rounded away from zero to whole units and clamped.
fn bounded(p: (f64, f64)) -> (f64, f64) {
    (p.0.round().clamp(0.0, 100.0), p.1.round().clamp(0.0, 100.0))
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

/// `FindClosestPoint`.
fn closest_point(points: &[(f64, f64)], p: (f64, f64)) -> Option<usize> {
    let mut best = None;
    let mut min = f64::MAX;
    for (i, q) in points.iter().enumerate() {
        let d = dist(p, *q);
        if d < min {
            min = d;
            best = Some(i);
        }
    }
    best
}

fn near_a_point(points: &[(f64, f64)], p: (f64, f64)) -> bool {
    closest_point(points, p).is_some_and(|i| dist(p, points[i]) < DISTANCE_TOLERANCE)
}

fn sorted(mut points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    points
}

/// What to draw for a move, while the press lasts. Nothing is committed.
#[derive(Clone, Debug, PartialEq)]
pub struct CurvePreview {
    pub points: Vec<(f64, f64)>,
    pub tooltip: Option<String>,
}

/// One press on a curve strip.
#[derive(Clone, Debug)]
pub struct CurveGesture {
    /// `GetCurveValue()`: what the row holds. It doesn't change until the release commits.
    curve: Curve,
    hold: Curve,
    drag_start: (f64, f64),
    index: Option<usize>,
    captured: bool,
    dragging: bool,
    width: f64,
    height: f64,
}

/// `OnMouseDown`. `None` for a library-linked curve, which ignores every gesture here, so any drag on it is a value drag (F67).
pub fn curve_mouse_down(curve: &Curve, x: f64, y: f64, width: f64, height: f64, mods: Mods) -> Option<CurveGesture> {
    if curve.is_library_reference() {
        return None;
    }
    let point = to_curve_units(x, y, width, height);
    let index = closest_point(&curve.points, point);
    let near = index.is_some_and(|i| dist(point, curve.points[i]) < DISTANCE_TOLERANCE);
    Some(CurveGesture {
        curve: curve.clone(),
        hold: curve.clone(),
        drag_start: (x, y),
        index,
        captured: mods.exactly_shift() || mods.exactly_command() || near,
        dragging: false,
        width,
        height,
    })
}

impl CurveGesture {
    /// Whether the press was taken. When it wasn't, a drag past the system distance is a value drag (F67), and the release can still act (F43).
    pub fn captured(&self) -> bool {
        self.captured
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// `OnMouseMove`, while pressed. `None` means nothing changed.
    pub fn mouse_move(&mut self, x: f64, y: f64, mods: Mods) -> Option<CurvePreview> {
        if !self.captured {
            return None;
        }
        if !self.dragging {
            if dist((x, y), self.drag_start) > DRAG_TOLERANCE {
                self.dragging = true;
                // `_dragStartPoint` moves here, so a ⌘-drag's new point lands where the drag began, not where the press was.
                self.drag_start = (x, y);
            }
            return None;
        }
        if mods.exactly_shift() {
            Some(self.set_level(x, y))
        } else {
            self.move_point(x, y)
        }
    }

    /// `SetLevel`.
    fn set_level(&mut self, x: f64, y: f64) -> CurvePreview {
        let level = bounded(to_curve_units(x, y, self.width, self.height)).1;
        self.hold = Curve::from_points(vec![(0.0, level), (100.0, level)]);
        CurvePreview { points: self.hold.points.clone(), tooltip: Some(format!("{level:.0}")) }
    }

    /// `MovePoint`: no check on the left or the top, and no sort, so a point can pass its neighbours (KEEP, F39).
    fn move_point(&mut self, x: f64, y: f64) -> Option<CurvePreview> {
        if x > self.width + 5.0 || y > self.height + 5.0 {
            return None;
        }
        let index = self.index?;
        let p = bounded(to_curve_units(x, y, self.width, self.height));
        let mut points = self.curve.points.clone();
        points[index] = p;
        self.hold = Curve::from_points(points);
        Some(CurvePreview { points: self.hold.points.clone(), tooltip: Some(format!("{:.0}, {:.0}", p.0, p.1)) })
    }

    /// `OnMouseUp`. The modifiers are read now, not at the press. `Some` is what to commit; a value equal to the row's commits nothing (F62).
    pub fn mouse_up(mut self, mods: Mods) -> Option<Curve> {
        if mods.shift && (mods.command || mods.option) {
            // ⇧⌘ inverts, ⇧⌥ reverses, and with all three reverse wins. The press needs no nearness.
            let points = if mods.option {
                self.curve.points.iter().map(|&(x, y)| (100.0 - x, y)).collect()
            } else {
                self.curve.points.iter().map(|&(x, y)| (x, 100.0 - y)).collect()
            };
            self.hold = Curve::from_points(sorted(points));
        } else if mods.exactly_command() {
            // A ⌘-drag's preview is thrown away here, and the new point goes where the drag began, on the original points (KEEP, F40).
            let mut points = self.curve.points.clone();
            points.push(bounded(to_curve_units(self.drag_start.0, self.drag_start.1, self.width, self.height)));
            self.hold = Curve::from_points(sorted(points));
        } else if mods.exactly_option() && self.captured && self.curve.points.len() >= 3 {
            let at = to_curve_units(self.drag_start.0, self.drag_start.1, self.width, self.height);
            if let Some(i) = closest_point(&self.curve.points, at) {
                let mut points = self.curve.points.clone();
                points.remove(i);
                self.hold = Curve::from_points(points);
            }
        }
        (self.dragging || self.captured || self.curve != self.hold).then_some(self.hold)
    }

    /// The value as it stands, for the row's redraw.
    pub fn hold(&self) -> &Curve {
        &self.hold
    }
}

/// The mouse cursor over a strip with no button down (F39, F42).
pub fn curve_hover(curve: &Curve, x: f64, y: f64, width: f64, height: f64, mods: Mods, over_handle: bool) -> StripCursor {
    if curve.is_library_reference() {
        return StripCursor::Arrow;
    }
    if mods.exactly_shift() {
        return if over_handle { StripCursor::ResizeHorizontal } else { StripCursor::ResizeVertical };
    }
    if near_a_point(&curve.points, to_curve_units(x, y, width, height)) {
        StripCursor::Cross
    } else {
        StripCursor::Arrow
    }
}

/// `UpdateLevelSliderPosition`: `y/100` when the curve is flat with 2 points, else 0.
pub fn curve_level_position(curve: &Curve) -> f64 {
    match curve.points.as_slice() {
        [a, b] if a.1 == b.1 => a.1 / 100.0,
        _ => 0.0,
    }
}

/// Dragging the level handle: `Round(pos * 100, AwayFromZero)` makes a flat curve (F42).
pub fn curve_level_from_handle(position: f64) -> Curve {
    let level = (position * 100.0).round();
    Curve::from_points(vec![(0.0, level), (100.0, level)])
}

/// A strip handle's tooltip, `string.Format("{0:0}", position * 100)`: .NET's custom format rounds halves away from zero.
pub fn curve_level_tooltip(position: f64) -> String {
    format!("{}", (position * 100.0).round())
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f64 = 300.0;
    const H: f64 = 32.0;

    #[test]
    fn handle_tooltip_rounds_halves_away_from_zero() {
        assert_eq!(curve_level_tooltip(0.125), "13");
        assert_eq!(curve_level_tooltip(0.625), "63");
        assert_eq!(curve_level_tooltip(0.124), "12");
        assert_eq!(curve_level_tooltip(0.0), "0");
        assert_eq!(curve_level_tooltip(1.0), "100");
    }

    fn ramp() -> Curve {
        Curve::ramp_up()
    }

    fn none() -> Mods {
        Mods::default()
    }

    fn cmd() -> Mods {
        Mods { command: true, ..none() }
    }

    fn opt() -> Mods {
        Mods { option: true, ..none() }
    }

    fn shift() -> Mods {
        Mods { shift: true, ..none() }
    }

    fn down(c: &Curve, x: f64, y: f64, m: Mods) -> CurveGesture {
        curve_mouse_down(c, x, y, W, H, m).unwrap()
    }

    #[test]
    fn mapping() {
        assert_eq!(to_curve_units(150.0, 16.0, W, H), (50.0, 50.0));
    }

    #[test]
    fn near_rule_is_in_curve_units() {
        let c = ramp();
        // 7.9% of the width from (0,0), on the bottom edge: captured.
        assert!(down(&c, 0.079 * W, H, none()).captured());
        // 8.1%: not captured.
        assert!(!down(&c, 0.081 * W, H, none()).captured());
        // 3 pt up is 9.4 units: not captured.
        assert!(!down(&c, 0.0, H - 3.0, none()).captured());
    }

    #[test]
    fn exact_shift_and_exact_command_capture_anywhere() {
        let c = ramp();
        assert!(down(&c, 150.0, 2.0, shift()).captured());
        assert!(down(&c, 150.0, 2.0, cmd()).captured());
        let both = Mods { shift: true, command: true, option: false };
        let g = down(&c, 150.0, 2.0, both);
        assert!(!g.captured());
        // But the release still inverts.
        assert_eq!(g.mouse_up(both).unwrap().points, vec![(0.0, 100.0), (100.0, 0.0)]);
    }

    #[test]
    fn move_rounds_away_from_zero_clamps_and_does_not_sort() {
        let c = Curve::from_points(vec![(0.0, 0.0), (50.0, 50.0), (100.0, 100.0)]);
        let mut g = down(&c, 150.0, 16.0, none());
        assert!(g.captured());
        assert_eq!(g.mouse_move(160.0, 16.0, none()), None);
        // 87.5% of the width rounds to 88: past the last point's neighbour order, unsorted.
        let p = g.mouse_move(0.875 * W, 16.0, none()).unwrap();
        assert_eq!(p.points[1], (88.0, 50.0));
        assert_eq!(p.tooltip.as_deref(), Some("88, 50"));
        // Past the top: clamped.
        let p = g.mouse_move(0.5 * W, -20.0, none()).unwrap();
        assert_eq!(p.points[1], (50.0, 100.0));
        // Past the right by more than 5: ignored.
        assert_eq!(g.mouse_move(W + 6.0, 16.0, none()), None);
        assert_eq!(g.mouse_up(none()).unwrap().points[1], (50.0, 100.0));
    }

    #[test]
    fn command_click_adds_sorted() {
        let c = ramp();
        let g = down(&c, 0.5 * W, 0.8 * H, cmd());
        assert_eq!(g.mouse_up(cmd()).unwrap().points, vec![(0.0, 0.0), (50.0, 20.0), (100.0, 100.0)]);
    }

    #[test]
    fn command_drag_adds_where_the_drag_began() {
        let c = ramp();
        let mut g = down(&c, 0.5 * W, 0.8 * H, cmd());
        // Tolerance crossed here, which is where the drag began.
        g.mouse_move(0.51 * W, 0.8 * H, cmd());
        let p = g.mouse_move(0.6 * W, 0.3 * H, cmd()).unwrap();
        // The closest point moves as a preview.
        assert_eq!(p.points.len(), 2);
        let result = g.mouse_up(cmd()).unwrap();
        assert_eq!(result.points, vec![(0.0, 0.0), (51.0, 20.0), (100.0, 100.0)]);
    }

    #[test]
    fn option_click_removes_but_never_below_two_points() {
        let c = Curve::from_points(vec![(0.0, 0.0), (50.0, 50.0), (100.0, 100.0)]);
        let g = down(&c, 0.5 * W, 0.5 * H, opt());
        assert_eq!(g.mouse_up(opt()).unwrap().points, vec![(0.0, 0.0), (100.0, 100.0)]);
        let c = ramp();
        let g = down(&c, 0.0, H, opt());
        assert_eq!(g.mouse_up(opt()).unwrap().points, c.points);
        // Not near: not captured, and nothing happens.
        let c = Curve::from_points(vec![(0.0, 0.0), (50.0, 50.0), (100.0, 100.0)]);
        let g = down(&c, 0.3 * W, 2.0, opt());
        assert_eq!(g.mouse_up(opt()), None);
    }

    #[test]
    fn invert_and_reverse() {
        let c = ramp();
        let inv = Mods { shift: true, command: true, option: false };
        let rev = Mods { shift: true, command: false, option: true };
        let all = Mods { shift: true, command: true, option: true };
        assert_eq!(down(&c, 1.0, 1.0, inv).mouse_up(inv).unwrap().points, vec![(0.0, 100.0), (100.0, 0.0)]);
        assert_eq!(down(&c, 1.0, 1.0, rev).mouse_up(rev).unwrap().points, vec![(0.0, 100.0), (100.0, 0.0)]);
        let c = Curve::from_points(vec![(0.0, 10.0), (100.0, 40.0)]);
        assert_eq!(down(&c, 1.0, 1.0, all).mouse_up(all).unwrap().points, vec![(0.0, 40.0), (100.0, 10.0)]);
    }

    #[test]
    fn level_from_a_shift_drag_and_the_handle() {
        let c = ramp();
        let mut g = down(&c, 10.0, 20.0, shift());
        g.mouse_move(10.0, 25.0, shift());
        // y = 33.4 units is 21.312 pt from the top.
        let y = H - 33.4 * H / 100.0;
        let p = g.mouse_move(10.0, y, shift()).unwrap();
        assert_eq!(p.points, vec![(0.0, 33.0), (100.0, 33.0)]);
        assert_eq!(p.tooltip.as_deref(), Some("33"));
        assert_eq!(g.mouse_up(shift()).unwrap().points, vec![(0.0, 33.0), (100.0, 33.0)]);
        assert_eq!(curve_level_from_handle(0.335).points, vec![(0.0, 34.0), (100.0, 34.0)]);
        assert_eq!(curve_level_position(&Curve::flat(25.0)), 0.25);
        assert_eq!(curve_level_position(&ramp()), 0.0);
    }

    #[test]
    fn shift_click_with_no_movement_changes_nothing() {
        let c = ramp();
        let g = down(&c, 10.0, 20.0, shift());
        assert_eq!(g.mouse_up(shift()).unwrap(), c);
    }

    #[test]
    fn a_linked_curve_ignores_all_of_it() {
        let mut c = ramp();
        c.library_reference_name = "Lib".to_string();
        assert!(curve_mouse_down(&c, 0.0, H, W, H, none()).is_none());
        assert_eq!(curve_hover(&c, 0.0, H, W, H, none(), false), StripCursor::Arrow);
    }

    #[test]
    fn hover_cursors() {
        let c = ramp();
        assert_eq!(curve_hover(&c, 0.0, H, W, H, none(), false), StripCursor::Cross);
        assert_eq!(curve_hover(&c, 150.0, 2.0, W, H, none(), false), StripCursor::Arrow);
        assert_eq!(curve_hover(&c, 150.0, 2.0, W, H, shift(), false), StripCursor::ResizeVertical);
        assert_eq!(curve_hover(&c, 150.0, 2.0, W, H, shift(), true), StripCursor::ResizeHorizontal);
    }

    #[test]
    fn transform_point() {
        assert_eq!(curve_strip_points(&[(50.0, 50.0)], 300.0, 30.0), vec![(149.0, 15.0)]);
        assert_eq!(curve_strip_points(&[(100.0, 0.0)], 300.0, 30.0), vec![(299.0, 30.0)]);
    }
}
