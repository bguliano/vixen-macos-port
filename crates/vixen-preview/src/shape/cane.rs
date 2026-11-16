//! Port of Shapes/PreviewCane.cs.

use super::{Modifiers, RotateTypes, Sel, Shape, ShapeKind, StringType, BR, TL};
use crate::point::{PointType, PreviewPoint};
use crate::tools::get_arc_points;

pub(crate) const ARCH: usize = 2;

impl Shape {
    /// `new PreviewCane(point, selectedNode, zoomLevel)`.
    pub fn new_cane(point: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Cane { arch_pixel_count: 0, line_pixel_count: 0, just_placed: false });
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(PreviewPoint::new(point.x, point.y));
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::Size),
        ];
        s.reconfigure_cane(node);
        s
    }

    pub(crate) fn reconfigure_cane(&mut self, node: Option<Sel>) {
        let (mut arch, mut line) = (8, 8);
        self.pixels.clear();
        let mut light_count = arch + line;
        if let Some(n) = node {
            let children = n.leaf_children();
            // is this a single node?
            if children.len() >= 8 {
                self.set_string_type(StringType::Pixel);
                let count = children.len() as i32;
                arch = count / 2;
                line = count - arch;
                // These two adjustments can never fire, since the sum is always the count. Kept from the C#.
                if arch + line > count {
                    arch -= 1;
                } else if arch + line < count {
                    line -= 1;
                }
                light_count = count;
                // Just add the pixels, they will get laid out next
                for c in children {
                    self.add_pixel(10, 10).node_id = c.uuid();
                }
            }
        }
        if let ShapeKind::Cane { arch_pixel_count, line_pixel_count, .. } = &mut self.kind {
            *arch_pixel_count = arch;
            *line_pixel_count = line;
        }
        self.add_pixels_for_node(node, light_count);
        // Lay out the pixels
        self.layout();
    }

    /// `LinePixelCount` setter.
    pub(crate) fn set_cane_line_pixel_count(&mut self, value: i32) {
        let ShapeKind::Cane { line_pixel_count, .. } = self.kind else { return };
        let mut line = line_pixel_count;
        while line < value {
            self.add_pixel(10, 10);
            line += 1;
        }
        while line > value {
            if !self.pixels.is_empty() {
                self.pixels.remove(0);
            }
            line -= 1;
        }
        if let ShapeKind::Cane { line_pixel_count, .. } = &mut self.kind {
            *line_pixel_count = line;
        }
        self.layout();
    }

    /// `ArchPixelCount` setter.
    pub(crate) fn set_cane_arch_pixel_count(&mut self, value: i32) {
        let ShapeKind::Cane { arch_pixel_count, .. } = self.kind else { return };
        let mut arch = arch_pixel_count;
        while arch < value {
            self.add_pixel(10, 10);
            arch += 1;
        }
        while arch > value {
            if !self.pixels.is_empty() {
                self.pixels.remove(0);
            }
            arch -= 1;
        }
        if let ShapeKind::Cane { arch_pixel_count, .. } = &mut self.kind {
            *arch_pixel_count = arch;
        }
        self.layout();
    }

    pub(crate) fn layout_cane(&mut self) {
        let ShapeKind::Cane { arch_pixel_count, line_pixel_count, .. } = self.kind else { return };
        let (tl, br, arch) = (self.points[TL], self.points[BR], self.points[ARCH]);
        let pixel_spacing = br.y.wrapping_sub(arch.y) as f64 / line_pixel_count as f64;
        for i in 0..line_pixel_count {
            let Some(p) = self.pixels.get_mut(i as usize) else { break };
            p.x = tl.x;
            p.y = br.y.wrapping_sub((i as f64 * pixel_spacing) as i32);
        }
        let arc_width = br.x.wrapping_sub(tl.x);
        let arc_height = arch.y.wrapping_sub(tl.y);
        let points = get_arc_points(arc_width as f64, arc_height as f64, arch_pixel_count as f64);
        for (i, a) in points.into_iter().enumerate() {
            let Some(p) = self.pixels.get_mut(i + line_pixel_count.max(0) as usize) else { break };
            p.x = a.0.wrapping_add(tl.x);
            p.y = a.1.wrapping_add(tl.y);
        }
        self.set_pixel_zoom_rotate();
    }

    pub(crate) fn mouse_move_cane(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            self.set_point_xy(sel, point.x, point.y);
            self.points[ARCH].x = self.points[TL].x;
            if matches!(self.kind, ShapeKind::Cane { just_placed: true, .. }) {
                self.points[ARCH].y = self.points[TL].y.wrapping_add(self.points[BR].y.wrapping_sub(self.points[TL].y) / 4);
            }
            self.layout();
            self.select_drag_points();
        } else {
            // If we get here, we're moving
            (change_x, change_y) = self.moving_delta(TL, change_x, change_y);
            self.translate_some(&[TL, BR, ARCH], change_x, change_y);
            self.layout();
        }
        self.base_mouse_move(x, y, change_x, change_y, m);
    }

    pub(crate) fn match_cane(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        let tl = self.points[TL];
        self.points[BR].x = tl.x.wrapping_add(other.points[BR].x.wrapping_sub(other.points[TL].x));
        self.points[BR].y = tl.y.wrapping_add(other.points[BR].y.wrapping_sub(other.points[TL].y));
        self.points[ARCH].y = self.points[BR].y.wrapping_sub(other.points[BR].y.wrapping_sub(other.points[ARCH].y));
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}
