//! Port of Shapes/PreviewIcicle.cs.

use super::{Modifiers, RotateTypes, Sel, Shape, ShapeKind, StringType};
use crate::point::{PointType, PreviewPoint};
use crate::tools::round;
use uuid::Uuid;

const INITIAL_STRING_SPACING: i32 = 10;
const INITIAL_LIGHTS_PER_STRING: i32 = 10;

impl Shape {
    /// `new PreviewIcicle(point1, point2, selectedNode, zoomLevel)`.
    pub fn new_icicle(point1: PreviewPoint, point2: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        // If we are creating this fresh, we need to know so we can add strings, etc. as drawn.
        let mut s = Shape::constructed(ShapeKind::Icicle { string_count: 0, vertical_spacing: 0, creating: true, initial_node: node.map(|n| n.uuid()) });
        s.zoom_level = zoom_level;
        s.points = vec![
            s.point_to_zoom_point(point1.with_type(PointType::SizeTopLeft)),
            s.point_to_zoom_point(point2.with_type(PointType::SizeTopRight)),
        ];
        s.strings = Some(Vec::new()); // the Layout below creates this through the Strings getter
        s.reconfigure_icicle(node);
        s
    }

    pub(crate) fn reconfigure_icicle(&mut self, node: Option<Sel>) {
        if let Some(n) = node {
            if !n.children().is_empty() && n.leaf_children().is_empty() {
                self.set_string_type(StringType::Pixel);
                let zoom = self.zoom();
                let a = PreviewPoint::new(10, 10);
                let strings: Vec<Shape> = n.children().into_iter().map(|c| Shape::new_line(a, a, c.children().len() as i32, Some(c), zoom)).collect();
                let count = strings.len() as i32;
                self.strings = Some(strings);
                if let ShapeKind::Icicle { string_count, creating, .. } = &mut self.kind {
                    *string_count = count;
                    *creating = false;
                }
            } else {
                self.set_string_type(StringType::Standard);
            }
        }
        self.layout();
    }

    /// The `Strings` getter's list: the first `StringCount` strings.
    fn icicle_string_len(&self) -> usize {
        let ShapeKind::Icicle { string_count, .. } = self.kind else { return 0 };
        let have = self.strings.as_ref().map_or(0, Vec::len);
        (string_count.max(0) as usize).min(have)
    }

    /// `VerticalSpacing` getter, which raises the stored value to at least 2.
    pub(crate) fn icicle_vertical_spacing(&mut self) -> i32 {
        if let ShapeKind::Icicle { vertical_spacing, .. } = &mut self.kind {
            *vertical_spacing = (*vertical_spacing).max(2);
            *vertical_spacing
        } else {
            2
        }
    }

    pub(crate) fn layout_icicle(&mut self) {
        let ShapeKind::Icicle { string_count, .. } = self.kind else { return };
        if self.points.len() < 2 {
            return;
        }
        let (p0, p1) = (self.points[0], self.points[1]);
        let x_spacing = p0.x.wrapping_sub(p1.x) as f64 / (string_count - 1) as f64;
        let y_spacing = p0.y.wrapping_sub(p1.y) as f64 / (string_count - 1) as f64;
        let mut x = p0.x as f64;
        let mut y = p0.y as f64;
        let n = self.icicle_string_len();
        if n == 0 {
            return;
        }
        let spacing = self.icicle_vertical_spacing();
        let zoom = self.zoom();
        for i in 0..n {
            let count = self.strings.as_ref().map_or(0, |s| s[i].pixel_count()) as i32;
            let line_length = spacing.wrapping_mul(count);
            let a = PreviewPoint::new(round(x), round(y));
            let b = PreviewPoint::new(round(x), round(y).wrapping_add(line_length));
            let a = self.transform_preview_point(a, zoom, RotateTypes::Clockwise);
            let b = self.transform_preview_point(b, zoom, RotateTypes::Clockwise);
            if let Some(line) = self.strings.as_mut().and_then(|l| l.get_mut(i)) {
                line.zoom_level = 1.0;
                line.set_line_points(a, b);
            }
            x -= x_spacing;
            y -= y_spacing;
        }
    }

    /// `SetupInitialString`.
    fn setup_initial_string(&mut self) {
        let Some(sel) = self.selected_point else { return };
        let current_x_spacing = self.point_of(sel).x.wrapping_sub(self.points[0].x).wrapping_abs();
        let mut string_count = 2;
        if current_x_spacing > 0 {
            string_count = current_x_spacing / INITIAL_STRING_SPACING + 1;
            // The stringCount needs to be at least 2!
            string_count = string_count.max(2);
        }
        self.set_icicle_string_count(string_count);
    }

    /// `StringCount` setter.
    pub(crate) fn set_icicle_string_count(&mut self, value: i32) {
        let ShapeKind::Icicle { creating, initial_node, .. } = self.kind else { return };
        if let ShapeKind::Icicle { string_count, .. } = &mut self.kind {
            *string_count = value;
        }
        let zoom = self.zoom();
        let string_type = self.string_type;
        if let Some(strings) = &mut self.strings {
            strings.truncate(value.max(0) as usize);
            let a = PreviewPoint::new(10, 10);
            while (strings.len() as i32) < value {
                strings.push(Shape::new_line(a, a, INITIAL_LIGHTS_PER_STRING, None, zoom));
                if string_type == StringType::Standard {
                    // Every pixel of every string goes to the first pixel's node.
                    if creating {
                        if let Some(p) = strings[0].pixels.first_mut() {
                            p.node_id = initial_node.unwrap_or_else(Uuid::nil);
                        }
                    }
                    let node = strings[0].pixels.first().map_or_else(Uuid::nil, |p| p.node_id);
                    for line in strings.iter_mut() {
                        line.connect_standard_strings = true;
                        line.set_string_type(StringType::Standard);
                        for p in &mut line.pixels {
                            p.node_id = node;
                        }
                    }
                }
            }
        }
        self.layout();
    }

    pub(crate) fn mouse_move_icicle(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        // See if we're resizing
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            self.set_point_xy(sel, point.x, point.y);
            if matches!(self.kind, ShapeKind::Icicle { creating: true, .. }) {
                self.setup_initial_string();
            }
            self.layout();
            self.select_drag_points();
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(0, change_x, change_y);
            self.translate_some(&[0, 1], change_x, change_y);
            self.layout();
        }
        self.base_mouse_move(x, y, change_x, change_y, m);
    }

    pub(crate) fn match_icicle(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        self.points[1].x = self.points[0].x.wrapping_add(other.points[1].x.wrapping_sub(other.points[0].x));
        self.points[1].y = self.points[0].y.wrapping_add(other.points[1].y.wrapping_sub(other.points[0].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}
