//! Port of Shapes/PreviewStarBurst.cs.

use super::{Direction, Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringType, BL, BR, TL, TR};
use crate::point::{PointType, PreviewPoint};
use crate::tools::get_ellipse_points;

impl Shape {
    /// `new PreviewStarBurst(point1, selectedNode, zoomLevel)`.
    pub fn new_star_burst(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::StarBurst { direction: Direction::Clockwise, xy_rotation: 0, inner_circle_size: 0 });
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
        ];
        s.reconfigure_star_burst(node);
        s
    }

    pub(crate) fn reconfigure_star_burst(&mut self, node: Option<Sel>) {
        let zoom = self.zoom();
        self.pixels.clear();
        let mut strings: Vec<Shape> = Vec::new();
        let a = PreviewPoint::new(10, 10);
        let b = PreviewPoint::new(20, 20);
        if let Some(n) = node {
            if n.parent_children().len() >= 4 {
                let spoke_pixel_count = n.children().iter().map(|c| c.children().len() as i32).max().unwrap_or(0);
                self.set_string_type(StringType::Pixel);
                for c in n.children() {
                    strings.push(Shape::new_line(a, b, spoke_pixel_count, Some(c), zoom));
                }
            }
        }
        if strings.is_empty() {
            // Just add lines, they will be laid out in Layout()
            self.set_string_type(StringType::Standard);
            for _ in 0..8 {
                let mut line = Shape::new_line(a, b, 10, node, zoom);
                line.set_string_type(StringType::Standard);
                strings.push(line);
            }
        }
        self.strings = Some(strings);
        self.layout();
    }

    pub(crate) fn layout_star_burst(&mut self) {
        let ShapeKind::StarBurst { xy_rotation, inner_circle_size, .. } = self.kind else { return };
        let (tl, br) = (self.points[TL], self.points[BR]);
        self.points[TR].x = br.x;
        self.points[TR].y = tl.y;
        self.points[BL].x = tl.x;
        self.points[BL].y = br.y;
        let width = br.x.wrapping_sub(tl.x).wrapping_abs();
        let height = br.y.wrapping_sub(tl.y).wrapping_abs();
        let center_x = self.right().wrapping_sub(width / 2);
        let center_y = self.bottom().wrapping_sub(height / 2);
        self.sync_strings_view();
        let count = self.strings.as_ref().map_or(0, Vec::len) as f64;
        let outer = get_ellipse_points(tl.x as f64, tl.y as f64, width as f64, height as f64, count, 360.0, xy_rotation as f64);
        let half_inner = inner_circle_size / 2;
        let inner = get_ellipse_points(
            center_x.wrapping_sub(half_inner) as f64,
            center_y.wrapping_sub(half_inner) as f64,
            inner_circle_size as f64,
            inner_circle_size as f64,
            count,
            360.0,
            xy_rotation as f64,
        );
        let zoom = self.zoom();
        let n = self.strings.as_ref().map_or(0, Vec::len);
        for i in 0..n {
            let (Some(a), Some(b)) = (inner.get(i), outer.get(i)) else { break };
            let i_pt = self.transform_preview_point(PreviewPoint::new(a.0, a.1), zoom, RotateTypes::Clockwise);
            let o_pt = self.transform_preview_point(PreviewPoint::new(b.0, b.1), zoom, RotateTypes::Clockwise);
            if let Some(line) = self.strings.as_mut().and_then(|l| l.get_mut(i)) {
                line.zoom_level = 1.0;
                line.set_line_points(i_pt, o_pt);
            }
        }
    }

    pub(crate) fn mouse_move_star_burst(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            // Should the height = the width?
            if sel == PointRef::Pt(BR) && m.only_ctrl() {
                let height = point.y.wrapping_sub(self.points[TR].y);
                self.points[TR].x = self.points[TL].x.wrapping_add(height);
                self.points[TR].y = self.points[TL].y;
                self.points[BL].x = self.points[TL].x;
                self.points[BL].y = self.points[TR].y.wrapping_add(height);
                let (tx, by) = (self.points[TR].x, self.points[BL].y);
                self.set_point_xy(sel, tx, by);
            } else {
                self.set_point_xy(sel, point.x, point.y);
            }
            match sel {
                PointRef::Pt(TR) => {
                    self.points[TL].y = self.points[TR].y;
                    self.points[BR].x = self.points[TR].x;
                }
                PointRef::Pt(BL) => {
                    self.points[TL].x = self.points[BL].x;
                    self.points[BR].y = self.points[BL].y;
                }
                PointRef::Pt(TL) => {
                    self.points[BL].x = self.points[TL].x;
                    self.points[TR].y = self.points[TL].y;
                }
                PointRef::Pt(BR) => {
                    self.points[TR].x = self.points[BR].x;
                    self.points[BL].y = self.points[BR].y;
                }
                _ => {}
            }
            self.layout();
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(TL, change_x, change_y);
            self.translate_points(change_x, change_y);
            self.layout();
        }
        self.base_mouse_move(x, y, change_x, change_y, m);
    }

    /// `StringCount` setter.
    pub(crate) fn set_star_burst_string_count(&mut self, value: i32) {
        let zoom = self.zoom();
        let Some(lights_per_string) = self.strings.as_ref().and_then(|s| s.first()).map(|s| s.pixels.len() as i32) else { return };
        if let Some(strings) = &mut self.strings {
            strings.truncate(value.max(0) as usize);
            while (strings.len() as i32) < value {
                strings.push(Shape::new_line(PreviewPoint::new(10, 10), PreviewPoint::new(10, 10), lights_per_string, None, zoom));
            }
        }
        self.layout();
    }

    /// `LightCount` setter.
    pub(crate) fn set_star_burst_light_count(&mut self, value: i32) {
        if let Some(strings) = &mut self.strings {
            for line in strings {
                line.set_line_pixel_count(value);
            }
        }
        self.layout();
    }

    pub(crate) fn match_star_burst(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        let other_count = other.strings.as_ref().map_or(0, Vec::len) as i32;
        self.set_star_burst_string_count(other_count);
        if let (ShapeKind::StarBurst { direction, xy_rotation, inner_circle_size }, ShapeKind::StarBurst { direction: od, xy_rotation: ox, inner_circle_size: oi }) = (&mut self.kind, &other.kind) {
            *inner_circle_size = *oi;
            *direction = *od;
            *xy_rotation = *ox;
        }
        let tl = self.points[TL];
        let o = &other.points;
        self.points[TR].x = tl.x.wrapping_add(o[TR].x.wrapping_sub(o[TL].x));
        self.points[TR].y = tl.y.wrapping_add(o[TR].y.wrapping_sub(o[TL].y));
        self.points[BR].x = tl.x.wrapping_add(o[BR].x.wrapping_sub(o[TL].x));
        self.points[BR].y = tl.y.wrapping_add(o[BR].y.wrapping_sub(o[TL].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}
