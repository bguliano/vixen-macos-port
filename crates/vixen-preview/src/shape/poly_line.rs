//! Port of Shapes/PreviewPolyLine.cs.

use super::{Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringType};
use crate::point::{PointType, PreviewPoint};
use crate::tools::{calculate_point_on_line, to_int32, triangle_hypotenuse};

const INITIAL_PIXEL_SPACING: i32 = 10;

impl Shape {
    /// `new PreviewPolyLine(point1, point2, selectedNode, zoomLevel)`. The C# ignores point2.
    pub fn new_poly_line(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::PolyLine { create_default_pixels: false });
        s.zoom_level = zoom_level;
        s.points = vec![s.point_to_zoom_point(point1.with_type(PointType::SizeTopLeft))];
        s.reconfigure_poly_line(node);
        s.creating = true;
        s
    }

    pub(crate) fn reconfigure_poly_line(&mut self, node: Option<Sel>) {
        match node {
            Some(n) => {
                let children = n.leaf_children();
                // is this a single node?
                if children.is_empty() {
                    self.set_string_type(StringType::Standard);
                    self.set_poly_line_pixel_count(2);
                    if n.is_leaf() {
                        if let Some(p) = self.pixels.first_mut() {
                            p.node_id = n.uuid();
                        }
                    }
                    self.set_create_default_pixels(true);
                } else {
                    self.set_string_type(StringType::Pixel);
                    self.set_poly_line_pixel_count(children.len() as i32);
                    // Just add the pixels, they will get laid out next
                    for (p, c) in self.pixels.iter_mut().zip(children) {
                        p.node_id = c.uuid();
                    }
                }
            }
            None => {
                self.set_create_default_pixels(true);
                self.set_poly_line_pixel_count(2);
            }
        }
        if self.creating {
            self.end_creation_poly_line();
        }
        self.layout();
    }

    fn set_create_default_pixels(&mut self, v: bool) {
        if let ShapeKind::PolyLine { create_default_pixels } = &mut self.kind {
            *create_default_pixels = v;
        }
    }

    fn create_default_pixels(&self) -> bool {
        matches!(self.kind, ShapeKind::PolyLine { create_default_pixels: true })
    }

    /// `PixelCount` setter.
    pub fn set_poly_line_pixel_count(&mut self, value: i32) {
        self.resize_pixel_list(value);
        self.assign_standard_pixels();
        self.layout();
    }

    /// `AssignStandardPixels`.
    fn assign_standard_pixels(&mut self) {
        if self.string_type == StringType::Standard {
            if let Some(first) = self.pixels.first().map(|p| p.node_id) {
                for p in &mut self.pixels {
                    p.node_id = first;
                }
            }
        }
    }

    /// `SegmentLength` and the sum in `LineLength`, which only differ in how the hypotenuse is spelled.
    fn segment_length(&self, n: usize) -> f64 {
        let (a, b) = (self.points[n], self.points[n + 1]);
        if a.x == b.x {
            a.y.wrapping_sub(b.y).wrapping_abs() as f64
        } else if a.y == b.y {
            a.x.wrapping_sub(b.x).wrapping_abs() as f64
        } else {
            triangle_hypotenuse(a.x.wrapping_sub(b.x).wrapping_abs() as f64, a.y.wrapping_sub(b.y).wrapping_abs() as f64)
        }
    }

    fn line_length(&self) -> f64 {
        (0..self.points.len().saturating_sub(1)).map(|i| self.segment_length(i)).sum()
    }

    pub(crate) fn layout_poly_line(&mut self) {
        if self.points.is_empty() {
            return;
        }
        let line_length = self.line_length();
        let pixel_count = self.pixels.len();
        if line_length > 0.0 && pixel_count > 0 {
            let pixel_spacing = line_length / (pixel_count as f64 - 1.0);
            let mut current_pixel_num = 0usize;
            // Tracks the spacing between each of the pixels without worrying about the points.
            let mut current_empty_space = 0.0f64;
            let last_point = self.points[self.points.len() - 1];
            // Length of the entire segment
            for point_num in 0..self.points.len() - 1 {
                let this_full_line_length = self.segment_length(point_num);
                // First pixel is a special case
                if current_empty_space + this_full_line_length > pixel_spacing || point_num == 0 {
                    let this_empty_start_length = if point_num == 0 { 0.0 } else { pixel_spacing - current_empty_space };
                    // Active length of the segment without the start
                    let mut this_active_line_length = this_full_line_length - this_empty_start_length;
                    // Get the pixels in this line. No hangers on the end.
                    let pixel_spaces_in_this_line = (this_active_line_length / pixel_spacing).trunc();
                    let pixels_in_this_line = pixel_spaces_in_this_line + 1.0;
                    // Re-calculate the active line length
                    this_active_line_length = pixel_spaces_in_this_line * pixel_spacing;
                    current_empty_space = this_full_line_length - this_empty_start_length - this_active_line_length;

                    let a = (self.points[point_num].x, self.points[point_num].y);
                    let b = (self.points[point_num + 1].x, self.points[point_num + 1].y);
                    let line_start = calculate_point_on_line(a, b, to_int32(this_empty_start_length));
                    let line_end = calculate_point_on_line(a, b, to_int32(this_empty_start_length + this_active_line_length));

                    let x_spacing = line_start.0.wrapping_sub(line_end.0) as f64 / pixel_spaces_in_this_line;
                    let y_spacing = line_start.1.wrapping_sub(line_end.1) as f64 / pixel_spaces_in_this_line;
                    let mut x = line_start.0 as f64;
                    let mut y = line_start.1 as f64;

                    let mut pixel_num = 0.0;
                    while pixel_num < pixels_in_this_line {
                        if current_pixel_num < pixel_count - 1 {
                            self.pixels[current_pixel_num].x = to_int32(x);
                            self.pixels[current_pixel_num].y = to_int32(y);
                            x -= x_spacing;
                            y -= y_spacing;
                        }
                        current_pixel_num += 1;
                        pixel_num += 1.0;
                    }
                } else {
                    current_empty_space += this_full_line_length;
                }
                // Finally, put the last dot on the line
                self.pixels[pixel_count - 1].x = last_point.x;
                self.pixels[pixel_count - 1].y = last_point.y;
            }
        }
        self.set_pixel_zoom_rotate();
    }

    pub(crate) fn mouse_move_poly_line(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        // See if we're resizing
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            self.set_point_xy(sel, point.x, point.y);
            if self.create_default_pixels() {
                let n = (to_int32(self.line_length()) / INITIAL_PIXEL_SPACING).max(2);
                self.set_poly_line_pixel_count(n);
            }
            self.layout();
            self.select_drag_points();
        } else if self.start_points.len() == self.points.len() {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(0, change_x, change_y);
            for p in &mut self.points {
                p.translate(change_x, change_y);
            }
            self.layout();
        }
        self.base_mouse_move(x, y, change_x, change_y, m);
    }

    pub(crate) fn mouse_up_poly_line(&mut self, x: i32, y: i32, left_button: bool) {
        if left_button {
            if self.creating {
                let new_point = self.point_to_zoom_point(PreviewPoint::typed(x, y, PointType::Size));
                self.points.push(new_point);
                self.selected_point = Some(PointRef::Pt(self.points.len() - 1));
            }
            if self.create_default_pixels() {
                let n = (to_int32(self.line_length()) / INITIAL_PIXEL_SPACING).max(2);
                self.set_poly_line_pixel_count(n);
            }
        }
        self.select_drag_points();
        self.layout();
    }

    /// `EndCreation`.
    pub fn end_creation_poly_line(&mut self) {
        if let Some(PointRef::Pt(i)) = self.selected_point {
            if i < self.points.len() {
                self.points.remove(i);
            }
        }
        if self.create_default_pixels() {
            let n = (to_int32(self.line_length()) / INITIAL_PIXEL_SPACING).max(2);
            self.set_poly_line_pixel_count(n);
        }
        self.creating = false;
        self.set_create_default_pixels(false);
        self.select_drag_points();
        self.layout();
    }
}
