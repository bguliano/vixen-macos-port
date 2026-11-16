//! Port of Shapes/PreviewLine.cs.

use super::{Modifiers, RotateTypes, Sel, Shape, ShapeKind, StringType};
use crate::point::{PointType, PreviewPoint};
use crate::tools::round;

impl Shape {
    /// `new PreviewLine(point1, point2, lightCount, selectedNode, zoomLevel)`.
    pub fn new_line(point1: PreviewPoint, point2: PreviewPoint, light_count: i32, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Line { add_start_padding: false, add_end_padding: false, light_count });
        s.zoom_level = zoom_level;
        // The C# sets these point types from the Point1 and Point2 getters, when the property grid first
        // reads them, always before the first draw.
        s.points = vec![
            s.point_to_zoom_point(point1).with_type(PointType::SizeTopLeft),
            s.point_to_zoom_point(point2).with_type(PointType::SizeBottomRight),
        ];
        s.reconfigure_line(node);
        s
    }

    pub(crate) fn reconfigure_line(&mut self, node: Option<Sel>) {
        let ShapeKind::Line { light_count, .. } = self.kind else { return };
        self.pixels.clear();
        match node {
            Some(n) => {
                let children = n.leaf_children();
                // is this a single node?
                if children.is_empty() {
                    self.set_string_type(StringType::Standard);
                    // Just add the pixels, they will get laid out next
                    for _ in 0..light_count {
                        let p = self.add_pixel(10, 10);
                        if n.is_leaf() {
                            p.node_id = n.uuid();
                        }
                    }
                } else {
                    self.set_string_type(StringType::Pixel);
                    for c in children {
                        self.add_pixel(10, 10).node_id = c.uuid();
                    }
                }
            }
            None => {
                for _ in 0..light_count {
                    self.add_pixel(10, 10);
                }
            }
        }
        // Lay out the pixels
        self.layout();
    }

    pub(crate) fn layout_line(&mut self) {
        let ShapeKind::Line { add_start_padding, add_end_padding, .. } = self.kind else { return };
        if self.points.len() < 2 {
            return;
        }
        let pixel_count = self.pixels.len() as i32;
        let mut count = if add_start_padding { pixel_count } else { pixel_count - 1 };
        count = if add_end_padding { count + 1 } else { count };
        let (p0, p1) = (self.points[0], self.points[1]);
        let x_spacing = p0.x.wrapping_sub(p1.x) as f64 / count as f64;
        let y_spacing = p0.y.wrapping_sub(p1.y) as f64 / count as f64;
        let mut x = p0.x as f64;
        let mut y = p0.y as f64;
        if add_start_padding {
            x -= x_spacing;
            y -= y_spacing;
        }
        for p in &mut self.pixels {
            p.x = round(x);
            p.y = round(y);
            x -= x_spacing;
            y -= y_spacing;
        }
        self.set_pixel_zoom_rotate();
    }

    /// `line.Point1 = ...; line.Point2 = ...` with the layout each setter runs, for a string's owner.
    pub(crate) fn set_line_points(&mut self, p1: PreviewPoint, p2: PreviewPoint) {
        if self.points.len() < 2 {
            return;
        }
        self.points[0].x = p1.x;
        self.points[0].y = p1.y;
        self.points[1].x = p2.x;
        self.points[1].y = p2.y;
        self.reset_rotation_axis();
        self.layout_line();
    }

    /// `PixelCount` setter of a line.
    pub fn set_line_pixel_count(&mut self, value: i32) {
        self.resize_pixel_list(value);
        self.layout();
    }

    /// The `while (Pixels.Count > value) ... while (Pixels.Count < value) ...` the light count setters share.
    pub(crate) fn resize_pixel_list(&mut self, value: i32) {
        let value = value.max(0) as usize;
        let size = self.pixel_size();
        if self.pixels.len() > value {
            self.pixels.truncate(value);
        }
        while self.pixels.len() < value {
            self.pixels.push(crate::pixel::PreviewPixel::new(10, 10, 0, size));
        }
    }

    pub(crate) fn mouse_move_line(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        // See if we're resizing
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            self.set_point_xy(sel, point.x, point.y);
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(0, change_x, change_y);
            self.translate_some(&[0, 1], change_x, change_y);
        }
        self.base_mouse_move(x, y, change_x, change_y, m);
        self.layout();
    }

    pub(crate) fn match_line(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        self.points[1].x = self.points[0].x.wrapping_add(other.points[1].x.wrapping_sub(other.points[0].x));
        self.points[1].y = self.points[0].y.wrapping_add(other.points[1].y.wrapping_sub(other.points[0].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }

    /// `line.Point1` setter.
    pub(crate) fn set_point_setter(&mut self, index: usize, value: PreviewPoint) {
        if let Some(p) = self.points.get_mut(index) {
            p.x = value.x;
            p.y = value.y;
        }
        self.reset_rotation_axis();
        self.layout();
    }
}
