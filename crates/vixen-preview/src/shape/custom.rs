//! Port of Shapes/PreviewCustom.cs: a group of shapes, or a template once placed.

use super::{Modifiers, PointRef, SelectArg, Shape, ShapeKind};
use crate::point::{PointType, PreviewPoint};

pub(crate) const TOP_LEFT: usize = 0;
/// The transient handle `SelectDragPoints` builds.
pub(crate) const BOTTOM_RIGHT_HANDLE: usize = 1;

impl Shape {
    /// `new PreviewCustom(point, shapes)`.
    pub fn new_custom(point: PreviewPoint, shapes: Vec<Shape>) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Custom { start_width: 0, start_height: 0 });
        s.points = vec![point, PreviewPoint::new(0, 0)];
        s.strings = Some(shapes);
        s.show_rotation = false;
        s.layout();
        s
    }

    pub(crate) fn layout_custom(&mut self) {
        if let Some(strings) = &mut self.strings {
            for s in strings {
                s.layout();
            }
        }
        self.set_pixel_zoom();
    }

    pub(crate) fn mouse_move_custom(&mut self, x: i32, y: i32, change_x: i32, change_y: i32, m: Modifiers) {
        // See if we're resizing
        if let Some(sel) = self.selected_point {
            let ShapeKind::Custom { start_width, .. } = self.kind else { return };
            let aspect = start_width.wrapping_add(change_x) as f64 / start_width as f64;
            if let Some(strings) = &mut self.strings {
                for s in strings {
                    s.resize_from_original(aspect);
                }
            }
            let start = self.start_points.get(TOP_LEFT).copied().unwrap_or(self.points[TOP_LEFT]);
            self.move_to_custom(start.x, start.y);
            //This is still not great, but it acts a little better until I can put more time into it.
            let (r, b) = (self.right(), self.bottom());
            self.set_point_xy(sel, r, b);
        } else {
            // If we get here, we're moving
            if let Some(start) = self.start_points.get(TOP_LEFT).copied() {
                self.points[TOP_LEFT].x = start.x.wrapping_add(change_x);
                self.points[TOP_LEFT].y = start.y.wrapping_add(change_y);
            }
            self.select_drag_points();
            if let Some(strings) = &mut self.strings {
                for s in strings {
                    s.mouse_move(x, y, change_x, change_y, m);
                }
            }
        }
    }

    pub(crate) fn set_select_point_custom(&mut self, new_selected: Option<PointRef>) {
        if let Some(strings) = &mut self.strings {
            for s in strings {
                s.set_select_point(SelectArg::Null);
            }
        }
        self.capture_start_points();
        let (l, t) = (self.left(), self.top());
        let (w, h) = (self.right().wrapping_sub(l), self.bottom().wrapping_sub(t));
        if let ShapeKind::Custom { start_width, start_height } = &mut self.kind {
            *start_width = w;
            *start_height = h;
        }
        self.selected_point = new_selected;
    }

    pub(crate) fn select_drag_points_custom(&mut self) {
        let (r, b) = (self.right(), self.bottom());
        self.points[BOTTOM_RIGHT_HANDLE] = PreviewPoint::typed(r, b, PointType::SizeBottomRight);
        self.set_select_points(vec![PointRef::Pt(BOTTOM_RIGHT_HANDLE)]);
    }

    pub(crate) fn move_to_custom(&mut self, x: i32, y: i32) {
        let (dx, dy) = (x.wrapping_sub(self.left()), y.wrapping_sub(self.top()));
        if let Some(strings) = &mut self.strings {
            for s in strings {
                let (l, t) = (s.left().wrapping_add(dx), s.top().wrapping_add(dy));
                s.move_to(l, t);
            }
        }
        self.points[TOP_LEFT].x = x;
        self.points[TOP_LEFT].y = y;
        self.layout();
    }
}
