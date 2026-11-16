//! Port of Shapes/PreviewSingle.cs.

use super::{Sel, Shape, ShapeKind, SELECT_POINT_SIZE};
use crate::point::PreviewPoint;
use crate::tools::to_int32;

impl Shape {
    /// `new PreviewSingle(point, selectedNode, zoomLevel)`.
    pub fn new_single(point: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Single);
        s.zoom_level = zoom_level;
        s.points = vec![s.point_to_zoom_point(point)];
        let p = s.add_pixel(10, 10);
        if let Some(n) = node.filter(|n| n.is_leaf()) {
            p.node_id = n.uuid();
        }
        s.show_rotation = false;
        // Lay out the pixels
        s.layout();
        s
    }

    pub(crate) fn layout_single(&mut self) {
        let p1 = self.points[0];
        if let Some(p) = self.pixels.first_mut() {
            p.x = p1.x;
            p.y = p1.y;
            self.set_pixel_zoom();
        }
    }

    pub(crate) fn mouse_move_single(&mut self, x: i32, y: i32, change_x: i32, change_y: i32) {
        let point = self.point_to_zoom_point(PreviewPoint::new(x, y));
        if let Some(sel) = self.selected_point {
            self.set_point_xy(sel, point.x, point.y);
            self.layout();
            self.select_drag_points();
        } else {
            // If we get here, we're moving
            let Some(start) = self.start_points.first().copied() else { return };
            let zoom = self.zoom();
            let moved = PreviewPoint::new(
                to_int32(start.x as f64 * zoom).wrapping_add(change_x),
                to_int32(start.y as f64 * zoom).wrapping_add(change_y),
            );
            let moved = self.point_to_zoom_point(moved);
            self.points[0].x = moved.x;
            self.points[0].y = moved.y;
            self.layout();
        }
    }

    pub(crate) fn point_in_shape_single(&self, point: PreviewPoint) -> bool {
        let p = self.point_to_zoom_point(point);
        let p1 = self.points[0];
        let size = self.pixel_size;
        if size < 5 {
            let half = SELECT_POINT_SIZE / 2;
            let (rx, ry) = (p1.x.wrapping_sub(half), p1.y.wrapping_sub(half));
            p.x >= rx && p.x <= rx.wrapping_add(SELECT_POINT_SIZE) && p.y >= ry && p.y <= ry.wrapping_add(SELECT_POINT_SIZE)
        } else {
            p.x >= p1.x.wrapping_sub(size / 2) && p.x <= p1.x.wrapping_add(size / 2) && p.y >= p1.y.wrapping_sub(size / 2) && p.y <= p1.y.wrapping_add(size / 2)
        }
    }
}
