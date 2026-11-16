//! Port of Shapes/PreviewMultiString.cs.

use super::{Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringType};
use crate::point::{PointType, PreviewPoint};
use vixen_core::ElementTree;

const INITIAL_LIGHTS_PER_STRING: i32 = 10;

impl Shape {
    /// `new PreviewMultiString(point1, point2, selectedNode, zoomLevel)`. The C# ignores point2.
    pub fn new_multi_string(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::MultiString { string_count: 0, input_elements: node.map(|n| n.uuid()) });
        s.zoom_level = zoom_level;
        s.points = vec![s.point_to_zoom_point(point1.with_type(PointType::SizeTopLeft))];
        s.strings = Some(Vec::new());
        s.creating = true;
        s
    }

    /// `StringCount` setter.
    pub(crate) fn set_multi_string_count(&mut self, value: i32) {
        let zoom = self.zoom();
        if let ShapeKind::MultiString { string_count, .. } = &mut self.kind {
            *string_count = value;
        }
        if let Some(strings) = &mut self.strings {
            strings.truncate(value.max(0) as usize);
            let mut first = true;
            let a = PreviewPoint::new(10, 10);
            while (strings.len() as i32) < value {
                let mut line = Shape::new_line(a, a, INITIAL_LIGHTS_PER_STRING, None, zoom);
                if let ShapeKind::Line { add_start_padding, .. } = &mut line.kind {
                    *add_start_padding = !first;
                }
                strings.push(line);
                first = false;
            }
        }
    }

    pub(crate) fn layout_multi_string(&mut self) {
        let ShapeKind::MultiString { string_count, .. } = self.kind else { return };
        let has_strings = self.strings.as_ref().is_some_and(|s| !s.is_empty());
        if self.points.len() < 2 || !has_strings {
            return;
        }
        let wanted = self.points.len() as i32 - 1;
        if string_count != wanted {
            self.set_multi_string_count(wanted);
        }
        let zoom = self.zoom();
        for string_num in 0..wanted as usize {
            let a = self.transform_preview_point(self.points[string_num], zoom, RotateTypes::Clockwise);
            let b = self.transform_preview_point(self.points[string_num + 1], zoom, RotateTypes::Clockwise);
            if let Some(line) = self.strings.as_mut().and_then(|l| l.get_mut(string_num)) {
                if let ShapeKind::Line { add_start_padding, .. } = &mut line.kind {
                    *add_start_padding = string_num > 0;
                }
                line.zoom_level = 1.0;
                line.set_line_points(a, b);
            }
        }
    }

    pub(crate) fn mouse_move_multi_string(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        // See if we're resizing
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            self.set_point_xy(sel, point.x, point.y);
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

    pub(crate) fn mouse_up_multi_string(&mut self, x: i32, y: i32, left_button: bool, tree: Option<&ElementTree>) {
        if left_button && self.creating {
            let new_point = self.point_to_zoom_point(PreviewPoint::new(x, y)).with_type(PointType::Size);
            self.points.push(new_point);
            self.create_string_multi_string(tree);
            self.selected_point = Some(PointRef::Pt(self.points.len() - 1));
        }
        self.select_drag_points();
        self.layout();
    }

    /// `CreateString`.
    fn create_string_multi_string(&mut self, tree: Option<&ElementTree>) {
        if !self.creating {
            return;
        }
        let ShapeKind::MultiString { input_elements, string_count } = self.kind else { return };
        let zoom = self.zoom();
        let a = PreviewPoint::new(10, 10);
        let input: Option<Sel> = match (input_elements, tree) {
            (Some(id), Some(t)) => Sel::from_uuid(t, id),
            _ => None,
        };
        let new_line = match input {
            // There is no ElementNode selected in the tree
            None => {
                self.set_multi_string_count(string_count + 1);
                return;
            }
            // We've got an ElementNode selected in the tree, now figure out what to do with it.
            Some(n) => {
                // PreviewTools.CountPixelsAndStrings
                let children = n.children();
                let input_pixel_count = children.iter().filter(|c| c.is_leaf()).count();
                let input_string_count = children.len() - input_pixel_count;
                let string_num = self.strings.as_ref().map_or(0, Vec::len);
                // is this a single node with no children?
                if input_pixel_count == 0 && input_string_count == 0 {
                    Shape::new_line(a, a, 10, Some(n), zoom)
                } else if string_num < input_string_count && input_pixel_count == 0 {
                    // Do we have multiple child strings in this group and no individual pixels selected?
                    self.set_string_type(StringType::Pixel);
                    let child = if input_string_count > 0 { children[string_num] } else { n };
                    Shape::new_line(a, a, child.element_count() as i32, Some(child), zoom)
                } else if input_pixel_count > 2 && input_string_count == 0 && string_num == 0 {
                    // If we're here, we have multiple pixels in a single string and no strings in our parent node
                    self.set_string_type(StringType::Pixel);
                    Shape::new_line(a, a, n.element_count() as i32, Some(n), zoom)
                } else {
                    // If we get here, there is nothing valid selected so add a string and move on
                    let mut line = Shape::new_line(a, a, 10, None, zoom);
                    line.set_string_type(self.string_type());
                    line
                }
            }
        };
        if let Some(strings) = &mut self.strings {
            strings.push(new_line);
            let n = strings.len() as i32;
            if let ShapeKind::MultiString { string_count, .. } = &mut self.kind {
                *string_count = n;
            }
        }
    }

    /// `EndCreation`.
    pub fn end_creation_multi_string(&mut self) {
        if self.creating {
            self.creating = false;
            if let Some(PointRef::Pt(i)) = self.selected_point {
                if i < self.points.len() {
                    self.points.remove(i);
                }
            }
            if let Some(strings) = &mut self.strings {
                strings.pop();
            }
            self.select_drag_points();
            self.layout();
        }
    }
}
