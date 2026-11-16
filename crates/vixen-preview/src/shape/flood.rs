//! Port of Shapes/PreviewFlood.cs. There is no toolbar button for this shape any more, but old files can
//! hold one. In C# its Left, Top, Right and Bottom all throw NotImplementedException, which fails the load
//! of any preview holding one. Here they are its point, so such a file loads.

use super::{Sel, Shape, ShapeKind, StringType};
use crate::point::PreviewPoint;

impl Shape {
    /// `new PreviewFlood(point, selectedNode)`.
    pub fn new_flood(point: PreviewPoint, node: Option<Sel>) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Flood);
        s.points = vec![point];
        s.pixel_size = 40;
        s.string_type = StringType::Standard;
        let p = s.add_pixel(10, 10);
        if let Some(n) = node.filter(|n| n.is_leaf()) {
            p.node_id = n.uuid();
        }
        // Lay out the flood
        s.layout();
        s
    }

    pub(crate) fn layout_flood(&mut self) {
        let p1 = self.points[0];
        if let Some(p) = self.pixels.first_mut() {
            p.x = p1.x;
            p.y = p1.y;
            p.size = 3;
            self.set_pixel_zoom();
        }
    }

    pub(crate) fn mouse_move_flood(&mut self, x: i32, y: i32, change_x: i32, change_y: i32) {
        if let Some(sel) = self.selected_point {
            self.set_point_xy(sel, x, y);
            self.layout();
            self.select_drag_points();
        } else {
            // If we get here, we're moving
            let Some(start) = self.start_points.first().copied() else { return };
            self.points[0].x = start.x.wrapping_add(change_x);
            self.points[0].y = start.y.wrapping_add(change_y);
            self.layout();
        }
    }
}
