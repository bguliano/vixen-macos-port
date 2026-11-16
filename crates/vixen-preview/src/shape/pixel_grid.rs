//! Port of Shapes/PreviewPixelGrid.cs.

use super::{Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringOrientation, StringType, BL, BR, TL, TR};
use crate::point::{PointType, PreviewPoint};
use vixen_core::Orientation;

impl Shape {
    /// `new PreviewPixelGrid(point1, selectedNode, zoomLevel)`.
    pub fn new_pixel_grid(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::PixelGrid { string_count: 0, lights_per_string: 0, string_orientation: StringOrientation::Vertical });
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
        ];
        s.reconfigure_pixel_grid(node);
        s
    }

    pub(crate) fn reconfigure_pixel_grid(&mut self, node: Option<Sel>) {
        let default_string_count = 16;
        let default_lights_per_string = 50;
        let zoom = self.zoom();
        let mut strings: Vec<Shape> = Vec::new();
        self.pixels.clear();
        let a = PreviewPoint::new(10, 10);
        let child_light_count = is_pixel_grid_selected(node);
        if let Some(count) = child_light_count {
            self.set_string_type(StringType::Pixel);
            if let Some(n) = node {
                for c in n.children() {
                    strings.push(Shape::new_line(a, a, count, Some(c), zoom));
                }
            }
        } else if is_standard_grid_selected(node) {
            self.set_string_type(StringType::Standard);
            if let Some(n) = node {
                for c in n.children() {
                    strings.push(Shape::new_line(a, a, default_lights_per_string, Some(c), zoom));
                }
            }
        } else {
            // Just add the pixels, we don't care where they go... they get positioned in Layout()
            for _ in 0..default_string_count {
                strings.push(Shape::new_line(a, a, default_lights_per_string, None, zoom));
            }
        }
        self.strings = Some(strings);
        self.set_pixel_grid_lights_per_string(child_light_count.unwrap_or(default_lights_per_string));
        let count = self.strings.as_ref().map_or(0, Vec::len) as i32;
        if let Some(n) = node {
            if n.tree.orientation(n.node) == Some(Orientation::Horizontal) {
                if let ShapeKind::PixelGrid { string_orientation, .. } = &mut self.kind {
                    *string_orientation = StringOrientation::Horizontal;
                }
            }
        }
        self.set_pixel_grid_string_count(count);
    }

    /// `LightsPerString` setter.
    pub(crate) fn set_pixel_grid_lights_per_string(&mut self, value: i32) {
        if let ShapeKind::PixelGrid { lights_per_string, .. } = &mut self.kind {
            *lights_per_string = value;
        }
        if let Some(strings) = &mut self.strings {
            for line in strings {
                line.set_line_pixel_count(value);
            }
        }
    }

    /// `StringCount` setter.
    pub(crate) fn set_pixel_grid_string_count(&mut self, value: i32) {
        let ShapeKind::PixelGrid { lights_per_string, .. } = self.kind else { return };
        if let ShapeKind::PixelGrid { string_count, .. } = &mut self.kind {
            *string_count = value;
        }
        let zoom = self.zoom();
        if let Some(strings) = &mut self.strings {
            strings.truncate(value.max(0) as usize);
            let a = PreviewPoint::new(10, 10);
            while (strings.len() as i32) < value {
                strings.push(Shape::new_line(a, a, lights_per_string, None, zoom));
            }
        }
        self.layout();
    }

    pub(crate) fn layout_pixel_grid(&mut self) {
        let ShapeKind::PixelGrid { string_count, string_orientation, .. } = self.kind else { return };
        let zoom = self.zoom();
        let have = self.strings.as_ref().map_or(0, Vec::len);
        let n = (string_count.max(0) as usize).min(have);
        let (tl, br, bl) = (self.points[TL], self.points[BR], self.points[BL]);
        if string_orientation == StringOrientation::Vertical {
            let width = br.x.wrapping_sub(tl.x);
            let height = br.y.wrapping_sub(tl.y);
            let string_x_spacing = width as f64 / (string_count - 1) as f64;
            let mut x = tl.x as f64;
            let y = tl.y;
            for string_num in 0..n {
                // Math.Round(x, MidpointRounding.AwayFromZero)
                let x1 = x.round() as i32;
                let a = self.transform_preview_point(PreviewPoint::new(x1, y.wrapping_add(height)), zoom, RotateTypes::Clockwise);
                let b = self.transform_preview_point(PreviewPoint::new(x1, y), zoom, RotateTypes::Clockwise);
                if let Some(line) = self.strings.as_mut().and_then(|l| l.get_mut(string_num)) {
                    line.zoom_level = 1.0;
                    line.set_line_points(a, b);
                }
                x += string_x_spacing;
            }
        } else {
            let width = br.x.wrapping_sub(bl.x);
            let height = bl.y.wrapping_sub(tl.y);
            let string_y_spacing = height as f64 / (string_count - 1) as f64;
            let x = bl.x;
            let mut y = bl.y as f64;
            for string_num in 0..n {
                let y1 = y.round() as i32;
                let a = self.transform_preview_point(PreviewPoint::new(x, y1), zoom, RotateTypes::Clockwise);
                let b = self.transform_preview_point(PreviewPoint::new(x.wrapping_add(width), y1), zoom, RotateTypes::Clockwise);
                if let Some(line) = self.strings.as_mut().and_then(|l| l.get_mut(string_num)) {
                    line.zoom_level = 1.0;
                    line.set_line_points(a, b);
                }
                y -= string_y_spacing;
            }
        }
    }

    pub(crate) fn mouse_move_pixel_grid(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        // See if we're resizing
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
        let (tl, br) = (self.points[TL], self.points[BR]);
        self.points[TR].x = br.x;
        self.points[TR].y = tl.y;
        self.points[BL].x = tl.x;
        self.points[BL].y = br.y;
        self.point_nil_mask = 0;
        self.base_mouse_move(x, y, change_x, change_y, m);
        // Layout the standard shape
        self.layout();
    }

    pub(crate) fn match_pixel_grid(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        if let (ShapeKind::PixelGrid { string_orientation, .. }, ShapeKind::PixelGrid { string_orientation: o, .. }) = (&mut self.kind, &other.kind) {
            *string_orientation = *o;
        }
        self.layout();
        let tl = self.points[TL];
        let o = &other.points;
        self.points[BR].x = tl.x.wrapping_add(o[BR].x.wrapping_sub(o[TL].x));
        // The Y offset is taken from the other's top right. Deliberate, as in the C#.
        self.points[BR].y = tl.y.wrapping_add(o[BR].y.wrapping_sub(o[TR].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}

/// `IsPixelGridSelected`.
fn is_pixel_grid_selected(selected: Option<Sel>) -> Option<i32> {
    let n = selected?;
    let strings = n.children();
    let mut last = -1;
    // Selected node has to be a group!
    if !n.is_leaf() && strings.len() >= 2 {
        // Iterate through the strings in the grid
        for parent in &strings {
            let lights = parent.children();
            let count = lights.len() as i32;
            if last == -1 {
                last = count;
            } else if last != count {
                // All the strings have to have the same light count for this to work!
                return None;
            }
            // If there are sub-groups this is not a grid element!
            if lights.iter().any(|c| !c.is_leaf()) {
                return None;
            }
        }
    }
    (last >= 2 && strings.len() >= 2).then_some(last)
}

/// `IsStandardGridSelected`.
fn is_standard_grid_selected(selected: Option<Sel>) -> bool {
    let mut parent_string_count = 0;
    // Selected node has to be a group!
    if let Some(n) = selected.filter(|n| !n.is_leaf()) {
        // Iterate through the strings in the grid
        for parent in n.children() {
            parent_string_count += 1;
            // If there are more groups, this is not a Grid
            if !parent.is_leaf() {
                return false;
            }
        }
    }
    // Gotta have at least 4 strings to make a Grid!
    parent_string_count >= 4
}
