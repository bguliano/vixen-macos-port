//! Port of Shapes/PreviewEllipse.cs.

use super::{Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringType, BL, BR, SELECT_POINT_SIZE, TL, TR};
use crate::point::{PointType, PreviewPoint};
use crate::tools::{get_ellipse_points, to_int32};

impl Shape {
    /// `new PreviewEllipse(point1, selectedNode, zoomLevel)`.
    pub fn new_ellipse(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Ellipse);
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
        ];
        s.reconfigure_ellipse(node);
        s
    }

    pub(crate) fn reconfigure_ellipse(&mut self, node: Option<Sel>) {
        let mut light_count = 50;
        self.pixels.clear();
        if let Some(n) = node {
            let children = n.leaf_children();
            // is this a single node?
            if children.len() >= 4 {
                self.set_string_type(StringType::Pixel);
                light_count = children.len() as i32;
                // Just add the pixels, they will get laid out next
                for c in children {
                    self.add_pixel(10, 10).node_id = c.uuid();
                }
            }
        }
        self.add_pixels_for_node(node, light_count);
        // Lay out the pixels
        self.layout();
    }

    pub(crate) fn layout_ellipse(&mut self) {
        let (tl, br) = (self.points[TL], self.points[BR]);
        let width = br.x.wrapping_sub(tl.x);
        let height = br.y.wrapping_sub(tl.y);
        let points = get_ellipse_points(0.0, 0.0, width as f64, height as f64, self.pixels.len() as f64, 360.0, 0.0);
        for (p, a) in self.pixels.iter_mut().zip(points) {
            p.x = a.0.wrapping_add(tl.x);
            p.y = a.1.wrapping_add(tl.y);
        }
        self.set_pixel_zoom_rotate();
    }

    pub(crate) fn mouse_move_ellipse(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            if sel == PointRef::Pt(TR) {
                self.points[TL].y = point.y;
                self.points[BR].x = point.x;
            } else if sel == PointRef::Pt(BL) {
                self.points[TL].x = point.x;
                self.points[BR].y = point.y;
            }
            self.set_point_xy(sel, point.x, point.y);
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(TL, change_x, change_y);
            self.translate_some(&[TL, BR], change_x, change_y);
        }
        if self.selected_point == Some(PointRef::Pt(BR)) && m.only_ctrl() {
            self.points[BR].y = self.points[TL].y.wrapping_add(self.points[BR].x.wrapping_sub(self.points[TL].x));
        }
        self.sync_ellipse_corners();
        self.base_mouse_move(x, y, change_x, change_y, m);
        // Layout the standard shape
        self.layout();
    }

    fn sync_ellipse_corners(&mut self) {
        let (tl, br) = (self.points[TL], self.points[BR]);
        self.points[TR].x = br.x;
        self.points[TR].y = tl.y;
        self.points[BL].x = tl.x;
        self.points[BL].y = br.y;
    }

    pub(crate) fn move_to_ellipse(&mut self, x: i32, y: i32) {
        let (dx, dy) = (x.wrapping_sub(self.points[TL].x), y.wrapping_sub(self.points[TL].y));
        self.points[TL].x = x;
        self.points[TL].y = y;
        self.points[BR].translate(dx, dy);
        self.sync_ellipse_corners();
        // Through the TopLeft and BottomRight setters, which reset the rotation axis.
        self.reset_rotation_axis();
        self.layout();
    }

    /// `PointToZoomPointAdd`: the ellipse compares in doubly zoomed space, deliberately as in the C#.
    pub(crate) fn ellipse_hit_point(&self, p: PreviewPoint) -> (i32, i32) {
        let q = self.point_to_zoom_point_add(p);
        (q.x, q.y)
    }

    pub(crate) fn ellipse_pixels_hit(&self, x: i32, y: i32) -> bool {
        let zoom = self.zoom();
        let half = SELECT_POINT_SIZE / 2;
        let w = SELECT_POINT_SIZE + self.pixel_size;
        self.pixels.iter().any(|p| {
            let rx = to_int32(p.x as f64 * zoom).wrapping_sub(half);
            let ry = to_int32(p.y as f64 * zoom).wrapping_sub(half);
            x >= rx && x <= rx.wrapping_add(w) && y >= ry && y <= ry.wrapping_add(w)
        })
    }

    pub(crate) fn match_ellipse(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        let tl = self.points[TL];
        self.points[BR].x = tl.x.wrapping_add(other.points[BR].x.wrapping_sub(other.points[TL].x));
        self.points[BR].y = tl.y.wrapping_add(other.points[BR].y.wrapping_sub(other.points[TL].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}
