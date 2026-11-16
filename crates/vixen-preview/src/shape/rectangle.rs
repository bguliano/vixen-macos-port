//! Port of Shapes/PreviewRectangle.cs.

use super::{Direction, Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringType, BL, BR, TL, TR};
use crate::point::{PointType, PreviewPoint};

/// Sets up one side of a rectangle: zoom one, the given padding, the given ends, then a layout.
pub(crate) fn set_side(line: &mut Shape, start_pad: bool, end_pad: bool, p1: PreviewPoint, p2: PreviewPoint) {
    line.zoom_level = 1.0;
    if let ShapeKind::Line { add_start_padding, add_end_padding, .. } = &mut line.kind {
        *add_start_padding = start_pad;
        *add_end_padding = end_pad;
    }
    line.set_line_points(p1, p2);
}

impl Shape {
    /// `new PreviewRectangle(point1, selectedNode, zoomLevel)`.
    pub fn new_rectangle(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Rectangle { direction: Direction::Clockwise, lock_xy: false });
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
        ];
        s.reconfigure_rectangle(node);
        s
    }

    pub(crate) fn reconfigure_rectangle(&mut self, node: Option<Sel>) {
        let zoom = self.zoom();
        let mut strings: Vec<Shape> = Vec::new();
        let a = PreviewPoint::new(10, 10);
        let b = PreviewPoint::new(20, 20);
        if let Some(n) = node {
            let parents = n.parent_children();
            // Do we have the 4 sides of the rectangle defined in our elements?
            if parents.len() == 4 {
                for pixel_string in parents {
                    strings.push(Shape::new_line(a, b, pixel_string.children().len() as i32, Some(pixel_string), zoom));
                }
            } else {
                let children = n.leaf_children();
                if children.len() >= 8 {
                    let increment = children.len() as i32 / 4;
                    let mut pixels_left = children.len() as i32;
                    self.set_string_type(StringType::Pixel);
                    // Just add lines, they will be laid out in Layout()
                    for _ in 0..4 {
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
        }
        if strings.is_empty() {
            // Just add lines, they will be laid out in Layout()
            for _ in 0..4 {
                strings.push(Shape::new_line(a, b, 10, node, zoom));
            }
        }
        self.strings = Some(strings);
        self.layout();
    }

    /// `Strings` getter of the base class, which every shape that does not override it runs through.
    pub(crate) fn sync_strings_view(&mut self) {
        let (connect, string_type) = (self.connect_standard_strings, self.string_type);
        if let Some(strings) = &mut self.strings {
            for line in strings {
                line.connect_standard_strings = connect;
                line.set_string_type(string_type);
            }
        }
    }

    pub(crate) fn layout_rectangle(&mut self) {
        let ShapeKind::Rectangle { direction, .. } = self.kind else { return };
        let zoom = self.zoom();
        // Calculate the relative position of all vertices
        let bl = self.transform_preview_point(PreviewPoint::new(self.points[BL].x, self.points[BL].y), zoom, RotateTypes::Clockwise);
        let br = self.transform_preview_point(PreviewPoint::new(self.points[BR].x, self.points[BR].y), zoom, RotateTypes::Clockwise);
        let tr = self.transform_preview_point(PreviewPoint::new(self.points[TR].x, self.points[TR].y), zoom, RotateTypes::Clockwise);
        let tl = self.transform_preview_point(PreviewPoint::new(self.points[TL].x, self.points[TL].y), zoom, RotateTypes::Clockwise);
        self.sync_strings_view();
        let Some(lines) = self.strings.as_mut().filter(|l| l.len() >= 4) else { return };
        // Paint 4 lines based upon the adjusted vertices. Set the Zoom Level to 1, since the vertices
        // themselves have already been zoomed.
        if direction == Direction::CounterClockwise {
            // Start in the lower left corner and move counter clockwise around the rectangle.
            set_side(&mut lines[0], false, false, bl, br);
            set_side(&mut lines[1], true, true, PreviewPoint::new(br.x, br.y.wrapping_sub(1)), PreviewPoint::new(tr.x, tr.y.wrapping_add(1)));
            set_side(&mut lines[2], false, false, tr, tl);
            set_side(&mut lines[3], true, true, PreviewPoint::new(tl.x, tl.y.wrapping_add(1)), PreviewPoint::new(bl.x, bl.y.wrapping_sub(1)));
        } else {
            // Start in the lower left corner and move clockwise around the rectangle.
            set_side(&mut lines[0], false, false, bl, tl);
            set_side(&mut lines[1], true, true, PreviewPoint::new(tl.x.wrapping_add(1), tl.y), PreviewPoint::new(tr.x.wrapping_sub(1), tr.y));
            set_side(&mut lines[2], false, false, tr, br);
            set_side(&mut lines[3], true, true, PreviewPoint::new(br.x.wrapping_sub(1), br.y), PreviewPoint::new(bl.x.wrapping_add(1), bl.y));
        }
    }

    pub(crate) fn mouse_move_rectangle(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            self.set_point_xy(sel, point.x, point.y);
            let lock_xy = matches!(self.kind, ShapeKind::Rectangle { lock_xy: true, .. });
            if lock_xy || (sel == PointRef::Pt(BR) && m.only_ctrl()) {
                self.points[TR].x = point.x;
                self.points[BL].y = point.y;
            }
            self.base_mouse_move(x, y, change_x, change_y, m);
            self.layout();
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(TL, change_x, change_y);
            self.translate_points(change_x, change_y);
            self.base_mouse_move(x, y, change_x, change_y, m);
            self.layout();
        }
    }

    pub(crate) fn match_rectangle(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        let o = &other.points;
        let tl = self.points[TL];
        // The Y offsets are all taken from the other's top right, not its top left. Deliberate, as in the C#.
        self.points[TR].x = tl.x.wrapping_add(o[TR].x.wrapping_sub(o[TL].x));
        self.points[TR].y = tl.y.wrapping_add(o[TR].y.wrapping_sub(o[TR].y));
        self.points[BR].x = tl.x.wrapping_add(o[BR].x.wrapping_sub(o[TL].x));
        self.points[BR].y = tl.y.wrapping_add(o[BR].y.wrapping_sub(o[TR].y));
        self.points[BL].x = tl.x.wrapping_add(o[BL].x.wrapping_sub(o[TL].x));
        self.points[BL].y = tl.y.wrapping_add(o[BL].y.wrapping_sub(o[TR].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }

    /// `LightCountString1` and so on.
    pub(crate) fn set_string_light_count(&mut self, string: usize, value: i32) {
        if let Some(line) = self.strings.as_mut().and_then(|l| l.get_mut(string)) {
            line.set_line_pixel_count(value);
        }
        self.layout();
    }
}
