//! Port of Shapes/PreviewMegaTree.cs.

use super::{Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringType, BL, BR, TL, TR};
use crate::point::{PointType, PreviewPoint};
use crate::tools::get_ellipse_points;

impl Shape {
    /// `new PreviewMegaTree(point1, selectedNode, zoomLevel)`.
    pub fn new_mega_tree(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let top_width = 20;
        let mut s = Shape::constructed(ShapeKind::MegaTree {
            string_count: 0,
            top_height: top_width / 2,
            top_width,
            base_height: 40,
            lights_per_string: 0,
            degrees: 360,
            degree_offset: 0,
        });
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
        ];
        // `_topRight` and `_bottomLeft` stay null until the first SelectDragPoints.
        s.point_nil_mask = (1 << TR) | (1 << BL);
        s.reconfigure_mega_tree(node);
        s
    }

    pub(crate) fn reconfigure_mega_tree(&mut self, node: Option<Sel>) {
        let zoom = self.zoom();
        let mut string_count = 16;
        let mut lights_per_string = 50;
        self.pixels.clear();
        let mut strings: Vec<Shape> = Vec::new();
        let a = PreviewPoint::new(10, 10);
        if let Some(child_light_count) = is_pixel_tree_selected(node) {
            self.set_string_type(StringType::Pixel);
            lights_per_string = child_light_count;
            if let Some(n) = node {
                for c in n.children() {
                    strings.push(Shape::new_line(a, a, lights_per_string, Some(c), zoom));
                }
            }
            string_count = strings.len() as i32;
        } else if is_standard_tree_selected(node) {
            self.set_string_type(StringType::Standard);
            if let Some(n) = node {
                for c in n.children() {
                    strings.push(Shape::new_line(a, a, lights_per_string, Some(c), zoom));
                }
            }
            string_count = strings.len() as i32;
        } else {
            // Just add the pixels, we don't care where they go... they get positioned in Layout()
            for _ in 0..string_count {
                strings.push(Shape::new_line(a, a, lights_per_string, None, zoom));
            }
        }
        self.strings = Some(strings);
        if let ShapeKind::MegaTree { string_count: sc, lights_per_string: lp, .. } = &mut self.kind {
            *sc = string_count;
            *lp = lights_per_string;
        }
        // Lay out the pixels
        self.layout();
    }

    /// `LightsPerString` setter.
    pub(crate) fn set_mega_tree_lights_per_string(&mut self, value: i32) {
        if let ShapeKind::MegaTree { lights_per_string, .. } = &mut self.kind {
            *lights_per_string = value;
        }
        if let Some(strings) = &mut self.strings {
            for line in strings {
                line.set_line_pixel_count(value);
            }
        }
    }

    /// `StringCount` setter.
    pub(crate) fn set_mega_tree_string_count(&mut self, value: i32) {
        let ShapeKind::MegaTree { lights_per_string, .. } = self.kind else { return };
        if let ShapeKind::MegaTree { string_count, .. } = &mut self.kind {
            *string_count = value;
        }
        let zoom = self.zoom();
        if let Some(strings) = &mut self.strings {
            strings.truncate(value.max(0) as usize);
            let a = PreviewPoint::new(10, 10);
            while (strings.len() as i32) < value {
                let mut line = Shape::new_line(a, a, lights_per_string, None, zoom);
                if let Some(first) = strings.first() {
                    line.set_string_type(first.string_type());
                }
                strings.push(line);
            }
        }
        self.layout();
    }

    pub(crate) fn layout_mega_tree(&mut self) {
        let ShapeKind::MegaTree { string_count, top_height, top_width, base_height, degrees, degree_offset, .. } = self.kind else { return };
        let (tl, br) = (self.points[TL], self.points[BR]);
        let width = br.x.wrapping_sub(tl.x);
        let top_left_offset = tl.x.wrapping_add(width / 2).wrapping_sub(top_width / 2) as f64;
        let bottom_top_offset = br.y.wrapping_sub(base_height) as f64;
        let total = ((360.0 / degrees as f64) * string_count as f64).ceil();
        let top = get_ellipse_points(top_left_offset, tl.y as f64, top_width as f64, top_height as f64, total, degrees as f64, degree_offset as f64);
        let base = get_ellipse_points(tl.x as f64, bottom_top_offset, width as f64, base_height as f64, total, degrees as f64, degree_offset as f64);
        let zoom = self.zoom();
        for string_num in 0..string_count.max(0) as usize {
            if string_num >= top.len() {
                continue;
            }
            let index = string_count as usize - 1 - string_num;
            let (Some(top_pixel), Some(base_pixel)) = (top.get(index), base.get(index)) else { continue };
            let t = self.transform_preview_point(PreviewPoint::new(top_pixel.0, top_pixel.1), zoom, RotateTypes::Clockwise);
            let b = self.transform_preview_point(PreviewPoint::new(base_pixel.0, base_pixel.1), zoom, RotateTypes::Clockwise);
            if let Some(line) = self.strings.as_mut().and_then(|l| l.get_mut(string_num)) {
                if matches!(line.kind, ShapeKind::Line { .. }) && line.points.len() >= 2 {
                    line.points[0].x = b.x;
                    line.points[0].y = b.y;
                    line.points[1].x = t.x;
                    line.points[1].y = t.y;
                    line.zoom_level = 1.0;
                    line.layout_line();
                }
            }
        }
    }

    pub(crate) fn mouse_move_mega_tree(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
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
        // Layout the standard shape
        self.base_mouse_move(x, y, change_x, change_y, m);
        self.layout();
    }

    pub(crate) fn match_mega_tree(&mut self, other: &Shape) {
        let ShapeKind::MegaTree { lights_per_string: olp, string_count: osc, top_height: oth, top_width: otw, base_height: obh, degrees: od, .. } = other.kind else { return };
        self.set_mega_tree_lights_per_string(olp);
        self.set_mega_tree_string_count(osc);
        self.set_pixel_size(other.pixel_size());
        if let ShapeKind::MegaTree { top_height, top_width, base_height, degrees, .. } = &mut self.kind {
            *top_height = oth;
            *top_width = otw;
            *base_height = obh;
            *degrees = od;
        }
        let tl = self.points[TL];
        self.points[BR].x = tl.x.wrapping_add(other.points[BR].x.wrapping_sub(other.points[TL].x));
        self.points[BR].y = tl.y.wrapping_add(other.points[BR].y.wrapping_sub(other.points[TL].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}

/// `IsPixelTreeSelected`: the light count when the node is a group of at least 4 equal strings of leaves.
fn is_pixel_tree_selected(selected: Option<Sel>) -> Option<i32> {
    let n = selected?;
    let strings = n.children();
    let mut last = -1;
    // Selected node has to be a group!
    if !n.is_leaf() && strings.len() >= 4 {
        // Iterate through the strings in the tree
        for parent in &strings {
            let lights = parent.children();
            let count = lights.len() as i32;
            if last == -1 {
                last = count;
            } else if last != count {
                // All the strings have to have the same light count for this to work!
                return None;
            }
            // If there are sub-groups this is not a mega tree element!
            if lights.iter().any(|c| !c.is_leaf()) {
                return None;
            }
        }
    }
    (last > 4 && strings.len() >= 4).then_some(last)
}

/// `IsStandardTreeSelected`.
fn is_standard_tree_selected(selected: Option<Sel>) -> bool {
    let mut parent_string_count = 0;
    // Selected node has to be a group!
    if let Some(n) = selected.filter(|n| !n.is_leaf()) {
        // Iterate through the strings in the tree
        for parent in n.children() {
            parent_string_count += 1;
            // If there are more groups, this is not a Mega Tree
            if !parent.is_leaf() {
                return false;
            }
        }
    }
    // Gotta have at least 4 strings to make a Mega Tree!
    parent_string_count >= 4
}
