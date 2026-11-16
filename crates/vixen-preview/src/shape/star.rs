//! Port of Shapes/PreviewStar.cs.

use super::{Direction, Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, StringType, BR, TL};
use crate::point::{PointType, PreviewPoint};
use crate::tools::{get_ellipse_points, round};

impl Shape {
    /// `new PreviewStar(point, selectedNode, zoomLevel)`.
    pub fn new_star(point: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Star {
            point_count: 0,
            pixel_count: 0,
            inside_size: 0,
            string_direction: Direction::Clockwise,
            xy_rotation: 18,
            pixels_per_point: 0,
        });
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point);
        s.points = vec![p.with_type(PointType::SizeTopLeft), p.with_type(PointType::SizeBottomRight)];
        s.reconfigure_star(node);
        s
    }

    fn add_all_children(&mut self, selected: Sel, point_count: &mut i32) {
        for child in selected.children() {
            if !child.is_leaf() {
                self.add_all_children(child, point_count);
                *point_count += 1;
            } else {
                self.add_pixel(10, 10).node_id = child.uuid();
            }
        }
    }

    pub(crate) fn reconfigure_star(&mut self, node: Option<Sel>) {
        let mut point_count = 0;
        let mut pixel_count = 0;
        self.pixels.clear();
        if let Some(n) = node {
            self.add_all_children(n, &mut point_count);
            pixel_count = self.pixels.len() as i32;
            // IsPixelStar
            if !n.is_leaf() && n.tree.leaves(n.node).len() >= 2 {
                self.set_string_type(StringType::Pixel);
            }
        }
        if self.pixels.len() >= 5 && point_count == 0 {
            configure_point_count(self.pixels.len() as i32, &mut point_count);
        }
        if point_count == 0 {
            configure_point_count(self.pixels.len() as i32 + 1, &mut point_count);
        }
        if self.pixels.len() < 5 {
            pixel_count = 40;
            point_count = 5;
            // Just add the pixels, they will get laid out next
            for _ in 0..pixel_count {
                let p = self.add_pixel(10, 10);
                if let Some(n) = node.filter(|n| n.is_leaf()) {
                    p.node_id = n.uuid();
                }
            }
        }
        if let ShapeKind::Star { point_count: pc, pixel_count: px, inside_size, .. } = &mut self.kind {
            *pc = point_count;
            *px = pixel_count;
            *inside_size = 40;
        }
        // Lay out the pixels
        self.layout();
    }

    /// `PixelCount` setter.
    pub(crate) fn set_star_pixel_count(&mut self, value: i32) {
        let ShapeKind::Star { pixel_count, .. } = self.kind else { return };
        let mut count = pixel_count;
        while count < value {
            self.add_pixel(10, 10);
            count += 1;
        }
        while count > value {
            self.pixels.pop();
            count -= 1;
        }
        if let ShapeKind::Star { pixel_count, .. } = &mut self.kind {
            *pixel_count = count;
        }
        self.layout();
    }

    /// `RecalcPoints`.
    fn recalc_star_points(&mut self) {
        let ShapeKind::Star { point_count, pixel_count, .. } = self.kind else { return };
        let mut per_point = pixel_count / point_count;
        while per_point % 2 != 0 {
            per_point -= 1;
        }
        if per_point < 2 {
            per_point = 2;
        }
        let new_count = per_point.wrapping_mul(point_count);
        let mut count = pixel_count;
        while count > new_count {
            self.pixels.pop();
            count -= 1;
        }
        while count < new_count {
            self.add_pixel(10, 10);
            count += 1;
        }
        if let ShapeKind::Star { pixel_count, pixels_per_point, .. } = &mut self.kind {
            *pixel_count = count;
            *pixels_per_point = per_point;
        }
    }

    pub(crate) fn layout_star(&mut self) {
        let (tl, br) = (self.points[TL], self.points[BR]);
        let w = br.x.wrapping_sub(tl.x).wrapping_abs();
        let h = br.y.wrapping_sub(tl.y).wrapping_abs();
        if !(w > 10 && h > 10) {
            return;
        }
        // RecalcPoints divides by the point count, which throws in C# when it is zero.
        if !matches!(self.kind, ShapeKind::Star { point_count, .. } if point_count > 0) {
            return;
        }
        self.recalc_star_points();
        let ShapeKind::Star { point_count, pixel_count, inside_size, string_direction, xy_rotation, pixels_per_point } = self.kind else { return };
        let outer_width = br.x.wrapping_sub(tl.x);
        let outer_height = br.y.wrapping_sub(tl.y);
        let degree_offset = 360 / point_count / 2;
        let outer = get_ellipse_points(
            tl.x as f64,
            tl.y as f64,
            outer_width as f64,
            outer_height as f64,
            point_count as f64,
            360.0,
            degree_offset.wrapping_add(xy_rotation) as f64,
        );
        let mut inner_width = (outer_width.wrapping_mul(inside_size) as f64 * 0.01) as i32;
        if inner_width < 4 {
            inner_width = 4;
        }
        let mut inner_height = (outer_height.wrapping_mul(inside_size) as f64 * 0.01) as i32;
        if inner_height < 4 {
            inner_height = 4;
        }
        let width_offset = outer_width.wrapping_sub(inner_width) / 2;
        let height_offset = outer_height.wrapping_sub(inner_height) / 2;
        let inner_left = tl.x.wrapping_add(width_offset);
        let inner_top = tl.y.wrapping_add(height_offset);
        let inner = get_ellipse_points(inner_left as f64, inner_top as f64, inner_width as f64, inner_height as f64, point_count as f64, 360.0, xy_rotation as f64);

        let mut pixel_num = 0i32;
        let line_count = pixels_per_point as f64 / 2.0;
        for i in 0..point_count {
            let (point1, point2, point3);
            if string_direction == Direction::Clockwise {
                let n = i as usize;
                let (Some(a), Some(b)) = (inner.get(n), outer.get(n)) else { continue };
                point1 = *a;
                point2 = *b;
                let next = if i < point_count - 1 { n + 1 } else { 0 };
                let Some(c) = inner.get(next) else { continue };
                point3 = *c;
            } else {
                let n = (point_count - i) as usize;
                let Some(a) = inner.get(n) else { continue };
                point1 = *a;
                let (Some(b), Some(c)) = (outer.get(n - 1), inner.get(n - 1)) else { continue };
                point2 = *b;
                point3 = *c;
            }
            // line1PixelCount + line2PixelCount always equals pixelsPerPoint, which is even, so the
            // C#'s adjustment never fires.
            for (from, to) in [(point1, point2), (point2, point3)] {
                let x_spacing = from.0.wrapping_sub(to.0) as f64 / line_count;
                let y_spacing = from.1.wrapping_sub(to.1) as f64 / line_count;
                let mut x = from.0 as f64;
                let mut y = from.1 as f64;
                let mut n = 0.0;
                while n < line_count {
                    if pixel_num < pixel_count {
                        if let Some(p) = self.pixels.get_mut(pixel_num as usize) {
                            p.x = round(x);
                            p.y = round(y);
                            x -= x_spacing;
                            y -= y_spacing;
                        }
                    }
                    pixel_num = pixel_num.wrapping_add(1);
                    n += 1.0;
                }
            }
        }
        self.set_pixel_zoom_rotate();
    }

    pub(crate) fn mouse_move_star(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            self.set_point_xy(sel, point.x, point.y);
            if sel == PointRef::Pt(BR) && m.only_ctrl() {
                self.points[BR].y = self.points[TL].y.wrapping_add(self.points[BR].x.wrapping_sub(self.points[TL].x));
            }
            self.layout();
            self.select_drag_points();
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(TL, change_x, change_y);
            self.translate_some(&[TL, BR], change_x, change_y);
            self.layout();
        }
        self.base_mouse_move(x, y, change_x, change_y, m);
    }

    pub(crate) fn match_star(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        if let (ShapeKind::Star { point_count, inside_size, xy_rotation, .. }, ShapeKind::Star { point_count: opc, inside_size: ois, xy_rotation: oxy, .. }) = (&mut self.kind, &other.kind) {
            *point_count = *opc;
            *inside_size = *ois;
            *xy_rotation = *oxy;
        }
        let tl = self.points[TL];
        self.points[BR].x = tl.x.wrapping_add(other.points[BR].x.wrapping_sub(other.points[TL].x));
        self.points[BR].y = tl.y.wrapping_add(other.points[BR].y.wrapping_sub(other.points[TL].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}

/// `ConfigurePointCount`.
fn configure_point_count(light_count: i32, point_count: &mut i32) {
    if light_count % 5 == 0 {
        *point_count = 5;
    } else if light_count % 3 == 0 {
        *point_count = 3;
    } else if light_count % 2 == 0 {
        *point_count = 4;
    }
}
