//! Port of Shapes/PreviewCustomProp.cs. The C# keeps `Pixels` as the same list as `PropPixels` at zoom one
//! with no rotation, and as clones otherwise. Here `view_pixels` is always its own list.

use super::transform::detent;
use super::{Modifiers, PointRef, SelectArg, Shape, ShapeKind, StringType, SELECT_POINT_SIZE};
use crate::pixel::PreviewPixel;
use crate::point::{PointType, PreviewDoublePoint, PreviewPoint};
use crate::tools::{scale_at, DRect, Matrix, Rect};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct CustomPropData {
    /// `PropPixels`, the model: unzoomed and unrotated.
    pub prop_pixels: Vec<PreviewPixel>,
    /// `Pixels`, what is shown.
    pub(crate) view_pixels: Vec<PreviewPixel>,
    /// The override of `PixelSize`, which just stores what it is given.
    pub(crate) pixel_size: i32,
    pub(crate) drag_points: Vec<PreviewDoublePoint>,
    pub(crate) p1_start: Option<(i32, i32)>,
    pub(crate) bounds: DRect,
    pub(crate) rotation_center: (f64, f64),
    pub(crate) selection_point: Option<usize>,
}

/// One element model's lights, for building a Custom Prop: `AddLightNodes`.
#[derive(Clone, Debug)]
pub struct CustomPropLightNode {
    pub node: Uuid,
    pub light_size: i32,
    pub lights: Vec<(f64, f64)>,
}

impl Shape {
    /// `new PreviewCustomProp(zoomLevel)`.
    pub fn new_custom_prop(zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::CustomProp(Box::default()));
        s.string_type = StringType::Custom;
        s.zoom_level = 0.0;
        s.set_zoom_level(zoom_level);
        s
    }

    /// `AddLightNodes`.
    pub fn add_light_nodes(&mut self, model: &CustomPropLightNode) {
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            for light in &model.lights {
                let mut p = PreviewPixel::new_high_precision(*light, model.light_size);
                p.node_id = model.node;
                c.prop_pixels.push(p);
            }
        }
    }

    pub(crate) fn layout_custom_prop(&mut self) {
        let zoom = self.zoom_custom_prop();
        let rotation = self.rotation_angle;
        let selected = self.selected;
        let ShapeKind::CustomProp(c) = &mut self.kind else { return };
        c.view_pixels.clone_from(&c.prop_pixels);
        if zoom != 1.0 {
            for p in &mut c.view_pixels {
                p.location = (p.location.0 * zoom, p.location.1 * zoom);
            }
        }
        c.update_bounds(selected, rotation);
        if rotation != 0 {
            let m = Matrix::rotate_at(rotation as f64, c.rotation_center.0, c.rotation_center.1);
            for p in &mut c.view_pixels {
                p.location = m.apply(p.location.0, p.location.1);
            }
        }
    }

    /// The override of `ZoomLevel` keeps its own field and has no `<= 0` fallback in the getter, but a file
    /// sets it to one on load and the setter never stores less.
    #[inline]
    fn zoom_custom_prop(&self) -> f64 {
        if self.zoom_level == 0.0 {
            1.0
        } else {
            self.zoom_level
        }
    }

    pub(crate) fn configure_drag_points(&mut self) {
        let rotation = self.rotation_angle;
        let ShapeKind::CustomProp(c) = &mut self.kind else { return };
        let b = c.bounds;
        c.drag_points.clear();
        c.drag_points.push(PreviewDoublePoint::typed(b.x, b.y, PointType::Size));
        c.drag_points.push(PreviewDoublePoint::typed(b.right(), b.y, PointType::Size));
        c.drag_points.push(PreviewDoublePoint::typed(b.right(), b.bottom(), PointType::Size));
        c.drag_points.push(PreviewDoublePoint::typed(b.x, b.bottom(), PointType::Size));
        c.drag_points.push(PreviewDoublePoint::typed(b.x + (b.right() - b.x) / 2.0, b.y - 10.0, PointType::RotateHandle));
        if rotation != 0 {
            c.rotate_drag_points(rotation);
        }
    }

    pub(crate) fn set_select_point_custom_prop(&mut self, arg: SelectArg) {
        let (point, reset) = match arg {
            SelectArg::Null => (None, true),
            SelectArg::Reset => (Some(PreviewPoint::new(0, 0)), true),
            SelectArg::Point(r) => {
                let p = self.point_of(r);
                (Some(p), p.x == 0 && p.y == 0)
            }
        };
        let ShapeKind::CustomProp(c) = &mut self.kind else { return };
        if reset {
            c.p1_start = Some((c.bounds.x as i32, c.bounds.y as i32));
        }
        c.selection_point = point.and_then(|p| c.point_in_drag_points(p));
    }

    pub(crate) fn point_in_select_point_custom_prop(&mut self, point: PreviewPoint) -> Option<PointRef> {
        let ShapeKind::CustomProp(c) = &self.kind else { return None };
        c.point_in_drag_points(point)?;
        // The C# returns the mouse point itself, not the handle.
        self.detached = point;
        Some(PointRef::Detached)
    }

    pub(crate) fn mouse_move_custom_prop(&mut self, x: i32, y: i32, change_x: i32, change_y: i32, m: Modifiers) {
        let zoom = self.zoom_custom_prop();
        let ShapeKind::CustomProp(c) = &self.kind else { return };
        let rt = Matrix::rotate_at(-(self.rotation_angle as f64), c.rotation_center.0, c.rotation_center.1);
        let point = rt.apply(x as f64, y as f64);
        let selection = c.selection_point.filter(|i| {
            c.drag_points.get(*i).is_some_and(|p| matches!(p.point_type, PointType::Size | PointType::RotateHandle))
        });
        let b = c.bounds;
        let drag = |i: usize| c.drag_points.get(i).copied().unwrap_or_default();
        // See if we're resizing
        let min_size = if zoom > 1.0 { 20.0 * zoom } else { 20.0 }; // ClampSize
        let shift = m.only_shift();
        match selection {
            Some(0) => {
                // Top Left
                let new_height = b.height + (b.y - point.1);
                let new_width = b.width + (b.x - point.0);
                let (sx, sy) = scales(new_width, new_height, min_size, b, shift);
                let anchor = drag(2);
                self.layout();
                self.scale_custom_prop(sx, sy, anchor.x / zoom, anchor.y / zoom);
            }
            Some(1) => {
                // Top Right
                let new_height = b.height + (b.y - point.1);
                let new_width = b.width + (point.0 - b.right());
                let (sx, sy) = scales(new_width, new_height, min_size, b, shift);
                let anchor = drag(3);
                self.scale_custom_prop(sx, sy, anchor.x / zoom, anchor.y / zoom);
                self.layout();
            }
            Some(2) => {
                // Bottom Right
                let new_height = b.height + (point.1 - b.bottom());
                let new_width = b.width + (point.0 - b.right());
                let (sx, sy) = scales(new_width, new_height, min_size, b, shift);
                let anchor = drag(0);
                self.scale_custom_prop(sx, sy, anchor.x / zoom, anchor.y / zoom);
                self.layout();
            }
            Some(3) => {
                // Bottom Left
                let new_height = b.height + (point.1 - b.bottom());
                let new_width = b.width + (b.x - point.0);
                let (sx, sy) = scales(new_width, new_height, min_size, b, shift);
                let anchor = drag(1);
                self.scale_custom_prop(sx, sy, anchor.x / zoom, anchor.y / zoom);
                self.layout();
            }
            Some(4) => {
                //We are rotating!
                let center = c.rotation_center;
                let mut angle = custom_prop_angle(center, (x as f64, y as f64));
                // Use Detents of 0, 45, 90, 135, 180, 225, 270 and 315 when holding the Ctrl key down.
                if m.only_ctrl() {
                    angle = detent(angle);
                }
                self.set_rotation_angle(angle.round() as i32);
            }
            _ => {
                let Some(start) = c.p1_start else { return };
                self.move_to_custom_prop(start.0.wrapping_add(change_x), start.1.wrapping_add(change_y));
            }
        }
    }

    fn scale_custom_prop(&mut self, sx: f64, sy: f64, cx: f64, cy: f64) {
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            for p in &mut c.prop_pixels {
                p.location = scale_at(p.location.0, p.location.1, sx, sy, cx, cy);
            }
        }
        self.layout();
    }

    pub(crate) fn point_in_shape_custom_prop(&self, point: PreviewPoint) -> bool {
        let ShapeKind::CustomProp(c) = &self.kind else { return false };
        let rt = Matrix::rotate_at(-(self.rotation_angle as f64), c.rotation_center.0, c.rotation_center.1);
        let (x, y) = rt.apply(point.x as f64, point.y as f64);
        c.bounds.contains(x, y)
    }

    pub(crate) fn shape_all_in_rect_custom_prop(&self, rect: Rect) -> bool {
        let ShapeKind::CustomProp(c) = &self.kind else { return false };
        let p1 = self.point_to_zoom_point(PreviewPoint::new(rect.x, rect.y));
        let p2 = self.point_to_zoom_point(PreviewPoint::new(rect.x.wrapping_add(rect.width), rect.y.wrapping_add(rect.height)));
        let (x1, x2) = (p1.x.min(p2.x) as f64, p1.x.max(p2.x) as f64);
        let (y1, y2) = (p1.y.min(p2.y) as f64, p1.y.max(p2.y) as f64);
        let rt = Matrix::rotate_at(self.rotation_angle as f64, c.rotation_center.0, c.rotation_center.1);
        let b = rt.transform_bounds(c.bounds);
        b.y >= y1 && b.bottom() <= y2 && b.x >= x1 && b.right() <= x2
    }

    pub(crate) fn move_to_custom_prop(&mut self, mut x: i32, mut y: i32) {
        if x < 0 || y < 0 {
            x = 10;
            y = 10;
        }
        let zoom = self.zoom_custom_prop();
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            let x_offset = (x as f64 - c.bounds.x) / zoom;
            let y_offset = (y as f64 - c.bounds.y) / zoom;
            for p in &mut c.prop_pixels {
                p.location = (p.location.0 + x_offset, p.location.1 + y_offset);
            }
        }
        self.layout();
    }

    pub(crate) fn resize_custom_prop(&mut self, aspect: f64) {
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            for p in &mut c.prop_pixels {
                p.location = (p.location.0 * aspect, p.location.1 * aspect);
            }
        }
        self.layout();
    }

    pub(crate) fn match_custom_prop(&mut self, other: &Shape) {
        // Match size for now
        let y_aspect = other.bottom().wrapping_sub(other.top()) as f64 / self.bottom().wrapping_sub(self.top()) as f64;
        let x_aspect = other.right().wrapping_sub(other.left()) as f64 / self.right().wrapping_sub(self.left()) as f64;
        // match up the rotation angle and set it directly
        self.set_rotation_angle(other.rotation_angle);
        let (l, t) = (self.left() as f64, self.top() as f64);
        self.scale_custom_prop(x_aspect, y_aspect, l, t);
    }

    /// `Bounds`.
    pub fn custom_prop_bounds(&self) -> Option<DRect> {
        match &self.kind {
            ShapeKind::CustomProp(c) => Some(c.bounds),
            _ => None,
        }
    }

    /// The `TopLeft` and `BottomRight` setters: they behave as a drag of that corner.
    pub(crate) fn set_custom_prop_corner(&mut self, index: usize, x: i32, y: i32, m: Modifiers) {
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            if index >= c.drag_points.len() {
                return;
            }
            c.selection_point = Some(index);
        }
        self.mouse_move_custom_prop(x, y, 0, 0, m);
    }
}

impl CustomPropData {
    fn point_in_drag_points(&self, point: PreviewPoint) -> Option<usize> {
        let half = SELECT_POINT_SIZE as f64 / 2.0;
        let (x, y) = (point.x as f64, point.y as f64);
        self.drag_points.iter().position(|s| x >= s.x - half && y >= s.y - half && x <= s.x + half && y <= s.y + half)
    }

    /// `UpdateBounds`.
    fn update_bounds(&mut self, selected: bool, rotation: i32) {
        let mut any = false;
        let (mut x_min, mut y_min) = (f64::INFINITY, f64::INFINITY);
        let (mut x_max, mut y_max) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &self.view_pixels {
            any = true;
            let (x, y, s) = (p.location.0, p.location.1, p.size as f64);
            x_min = x_min.min(x);
            y_min = y_min.min(y);
            x_max = x_max.max(x + s);
            y_max = y_max.max(y + s);
        }
        self.bounds = if any { DRect { x: x_min, y: y_min, width: x_max - x_min, height: y_max - y_min } } else { DRect::default() };
        let b = self.bounds;
        self.rotation_center = (b.x + b.width / 2.0, b.y + b.height / 2.0);
        if selected {
            self.update_drag_points(rotation);
        }
    }

    /// `UpdateDragPoints`. It puts the rotate handle 12 above where `ConfigureDragPoints` puts it 10 above,
    /// deliberately as in the C#.
    fn update_drag_points(&mut self, rotation: i32) {
        if self.drag_points.len() < 5 {
            return;
        }
        let b = self.bounds;
        let targets = [(b.x, b.y), (b.right(), b.y), (b.right(), b.bottom()), (b.x, b.bottom()), (b.x + (b.right() - b.x) / 2.0, b.y - 12.0)];
        for (p, t) in self.drag_points.iter_mut().zip(targets) {
            p.x = t.0;
            p.y = t.1;
        }
        if rotation != 0 {
            self.rotate_drag_points(rotation);
        }
    }

    fn rotate_drag_points(&mut self, rotation: i32) {
        let m = Matrix::rotate_at(rotation as f64, self.rotation_center.0, self.rotation_center.1);
        for p in &mut self.drag_points {
            (p.x, p.y) = m.apply(p.x, p.y);
        }
    }
}

/// The scale factors of a corner drag, with the shift key's aspect lock.
fn scales(new_width: f64, new_height: f64, min_size: f64, b: DRect, shift: bool) -> (f64, f64) {
    let mut sy = if new_height > min_size { new_height / b.height } else { 1.0 };
    let mut sx = if new_width > min_size { new_width / b.width } else { 1.0 };
    if shift {
        let v = max_value(sx, sy);
        sx = v;
        sy = v;
    }
    (sx, sy)
}

/// `MaxValue`.
fn max_value(a: f64, b: f64) -> f64 {
    if a >= 0.0 && b >= 0.0 {
        return a.max(b);
    }
    if a < 0.0 && b < 0.0 {
        return a.min(b);
    }
    if a.abs() > b.abs() {
        return a;
    }
    b
}

/// `GetAngle`: fetches the angle relative to the screen center point.
fn custom_prop_angle(screen: (f64, f64), center: (f64, f64)) -> f64 {
    let dx = screen.0 - center.0;
    let dy = screen.1 - center.1;
    let in_rads = dy.atan2(dx);
    // Convert from radians and adjust the angle from the 0 at 9:00 position.
    (in_rads * (180.0 / PI) + 270.0) % 360.0
}
