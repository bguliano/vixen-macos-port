//! Port of Shapes/PreviewTriangle.cs.

use super::{Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringType};
use crate::point::{PointType, PreviewPoint};
use crate::tools::to_int32;

pub(crate) const P1: usize = 0;
pub(crate) const P2: usize = 1;
pub(crate) const P3: usize = 2;
/// `_bottomRightPoint`, the handle that exists only while the triangle is first drawn.
pub(crate) const BOTTOM_RIGHT: usize = 3;

impl Shape {
    /// `new PreviewTriangle(point1, selectedNode, zoomLevel)`.
    pub fn new_triangle(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Triangle { bottom_right_start: None });
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::new(p.x, p.y),
        ];
        s.reconfigure_triangle(node);
        s
    }

    pub(crate) fn reconfigure_triangle(&mut self, node: Option<Sel>) {
        let zoom = self.zoom();
        let mut strings: Vec<Shape> = Vec::new();
        self.pixels.clear();
        let a = PreviewPoint::new(10, 10);
        let b = PreviewPoint::new(20, 20);
        if let Some(n) = node {
            let children = n.leaf_children();
            if children.len() >= 6 {
                let increment = children.len() as i32 / 3;
                let mut pixels_left = children.len() as i32;
                self.set_string_type(StringType::Pixel);
                // Just add lines, they will be laid out in Layout()
                for _ in 0..3 {
                    let count = if pixels_left >= increment { increment } else { pixels_left };
                    strings.push(Shape::new_line(a, b, count, None, zoom));
                    pixels_left -= increment;
                }
                let mut it = children.iter();
                for line in &mut strings {
                    for p in &mut line.pixels {
                        if let Some(c) = it.next() {
                            p.node_id = c.uuid();
                        }
                    }
                }
            }
        }
        if strings.is_empty() {
            // Just add lines, they will be laid out in Layout()
            for _ in 0..3 {
                strings.push(Shape::new_line(a, b, 10, node, zoom));
            }
        }
        self.strings = Some(strings);
        self.layout();
    }

    pub(crate) fn layout_triangle(&mut self) {
        let zoom = self.zoom();
        // Calculate the relative position of all vertices
        let point1 = self.transform_preview_point(self.points[P1], zoom, RotateTypes::Clockwise);
        let point2 = self.transform_preview_point(self.points[P2], zoom, RotateTypes::Clockwise);
        let point3 = self.transform_preview_point(self.points[P3], zoom, RotateTypes::Clockwise);
        self.sync_strings_view();
        // Paint 3 lines based upon the adjusted vertices. Set the Zoom Level to 1, since the vertices
        // themselves have already been zoomed.
        let Some(lines) = self.strings.as_mut().filter(|l| l.len() == 3) else { return };
        for (line, (a, b)) in lines.iter_mut().zip([(point1, point2), (point2, point3), (point3, point1)]) {
            line.zoom_level = 1.0;
            line.set_line_points(a, b);
        }
    }

    pub(crate) fn mouse_move_triangle(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        // See if we're resizing
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            if sel == PointRef::Pt(BOTTOM_RIGHT) {
                let start = match self.kind {
                    ShapeKind::Triangle { bottom_right_start: Some(s), .. } => s,
                    _ => self.points[BOTTOM_RIGHT],
                };
                let left = start.x;
                let right = to_int32(left as f64 + change_x as f64 / zoom);
                let width = right.wrapping_sub(left);
                let top = start.y;
                let bottom = to_int32(top as f64 + change_y as f64 / zoom);
                self.points[P1].x = left.wrapping_add(width / 2);
                self.points[P1].y = top;
                self.points[P2].x = right;
                self.points[P2].y = bottom;
                self.points[P3].x = left;
                self.points[P3].y = bottom;
                self.layout();
            } else {
                self.set_point_xy(sel, point.x, point.y);
                self.layout();
            }
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(P1, change_x, change_y);
            self.translate_some(&[P1, P2, P3], change_x, change_y);
            self.layout();
        }
        self.base_mouse_move(x, y, change_x, change_y, m);
    }

    /// The triangle is strange. When first drawn, the default select point should be the bottom right
    /// corner. This point will not be available again...
    pub(crate) fn select_default_select_point_triangle(&mut self) {
        let brx = self.points[P1].x.max(self.points[P2].x).max(self.points[P3].x);
        let bry = self.points[P1].y.max(self.points[P2].y).max(self.points[P3].y);
        self.points[BOTTOM_RIGHT] = PreviewPoint::new(brx, bry);
        if let Some(list) = &mut self.select_points {
            list.push(PointRef::Pt(BOTTOM_RIGHT));
        }
        self.selected_point = Some(PointRef::Pt(BOTTOM_RIGHT));
        if let ShapeKind::Triangle { bottom_right_start, .. } = &mut self.kind {
            *bottom_right_start = Some(PreviewPoint::new(brx, bry));
        }
    }

    /// `Width` getter: really the rightmost X, as in the C#.
    pub(crate) fn triangle_width(&self) -> i32 {
        self.points[P1].x.max(self.points[P2].x).max(self.points[P3].x)
    }

    /// `Height` getter: really the bottom Y, as in the C#.
    pub(crate) fn triangle_height(&self) -> i32 {
        self.points[P1].y.max(self.points[P2].y).max(self.points[P3].y)
    }

    pub(crate) fn match_triangle(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        let p1 = self.points[P1];
        let o = &other.points;
        self.points[P2].x = p1.x.wrapping_add(o[P2].x.wrapping_sub(o[P1].x));
        self.points[P2].y = p1.y.wrapping_add(o[P2].y.wrapping_sub(o[P1].y));
        self.points[P3].x = p1.x.wrapping_add(o[P3].x.wrapping_sub(o[P1].x));
        self.points[P3].y = p1.y.wrapping_add(o[P3].y.wrapping_sub(o[P1].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}
