//! Port of Shapes/PreviewArch.cs.

use super::{Modifiers, RotateTypes, Sel, Shape, ShapeKind, StringType, BL, BR, TL, TR};
use crate::point::{PointType, PreviewPoint};
use crate::tools::get_arc_points;

impl Shape {
    /// `new PreviewArch(point1, selectedNode, zoomLevel)`.
    pub fn new_arch(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Arch);
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
        ];
        s.reconfigure_arch(node);
        s
    }

    pub(crate) fn reconfigure_arch(&mut self, node: Option<Sel>) {
        self.pixels.clear();
        let light_count = 25;
        if let Some(n) = node {
            let children = n.leaf_children();
            // is this a single node?
            if children.len() >= 4 {
                self.set_string_type(StringType::Pixel);
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

    /// What the `TopRight` and `BottomLeft` getters do: compute the corners from the two real points.
    pub(crate) fn sync_arch_corners(&mut self) {
        let (tl, br) = (self.points[TL], self.points[BR]);
        self.points[TR] = PreviewPoint::typed(br.x, tl.y, PointType::SizeTopRight);
        self.points[BL] = PreviewPoint::typed(tl.x, br.y, PointType::SizeBottomLeft);
    }

    pub(crate) fn layout_arch(&mut self) {
        let (tl, br) = (self.points[TL], self.points[BR]);
        let width = br.x.wrapping_sub(tl.x);
        let height = br.y.wrapping_sub(tl.y);
        let points = get_arc_points(width as f64, height as f64, self.pixels.len() as f64);
        for (p, a) in self.pixels.iter_mut().zip(points) {
            p.x = a.0.wrapping_add(tl.x);
            p.y = a.1.wrapping_add(tl.y);
        }
        self.set_pixel_zoom_rotate();
    }

    pub(crate) fn mouse_move_arch(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        // See if we're resizing
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            match self.point_of(sel).point_type {
                PointType::SizeTopRight => {
                    self.points[TL].y = point.y;
                    self.points[BR].x = point.x;
                }
                PointType::SizeBottomLeft => {
                    self.points[TL].x = point.x;
                    self.points[BR].y = point.y;
                }
                _ => {}
            }
            self.set_point_xy(sel, point.x, point.y);
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(TL, change_x, change_y);
            self.translate_some(&[TL, BR], change_x, change_y);
        }
        self.sync_arch_corners();
        self.base_mouse_move(x, y, change_x, change_y, m);
        self.layout();
    }

    pub(crate) fn move_to_arch(&mut self, x: i32, y: i32) {
        let (tl, br) = (self.points[TL], self.points[BR]);
        let (dx, dy) = (x.wrapping_sub(tl.x.min(br.x)), y.wrapping_sub(tl.y.min(br.y)));
        self.translate_some(&[TL, BR], dx, dy);
        self.sync_arch_corners();
        // Through the TopLeft and BottomRight setters, which reset the rotation axis.
        self.reset_rotation_axis();
        self.layout();
    }

    /// `Width` getter.
    pub(crate) fn arch_width(&self) -> i32 {
        self.points[BR].x.wrapping_sub(self.points[TL].x).wrapping_abs()
    }

    /// `Height` getter.
    pub(crate) fn arch_height(&self) -> i32 {
        self.points[BR].y.wrapping_sub(self.points[TL].y).wrapping_abs()
    }

    /// `Width` setter.
    pub(crate) fn set_arch_width(&mut self, value: i32) {
        self.points[BR].x = self.points[TL].x.wrapping_add(value);
        self.points[TR].x = self.points[BR].x;
        self.layout();
    }

    /// `Height` setter.
    pub(crate) fn set_arch_height(&mut self, value: i32) {
        self.points[TL].y = self.points[BR].y.wrapping_sub(value);
        self.points[TR].y = self.points[TL].y;
        self.layout();
    }

    pub(crate) fn match_arch(&mut self, other: &Shape) {
        self.set_arch_width(other.arch_width());
        self.set_arch_height(other.arch_height());
        self.set_pixel_size(other.pixel_size());
        self.rotation_angle = other.rotation_angle;
    }
}
