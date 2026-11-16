//! Port of Shapes/PreviewNet.cs. The C# finds which lattice points are inside the net's quadrilateral by
//! filling it into a bitmap with GDI+ and reading pixels back. This asks the same question geometrically,
//! with GDI+'s non-antialiased rule: a pixel's sample point is its integer coordinate, inside by the
//! even-odd rule, with top and left edges in and bottom and right edges out.

use super::{Modifiers, PointRef, RotateTypes, Sel, Shape, ShapeKind, BL, BR, TL, TR};
use crate::pixel::PreviewPixel;
use crate::point::{PointType, PreviewPoint};
use uuid::Uuid;

/// GDI+ allocates a 32 bit bitmap of the net's size, and no more than 2 GB of one.
const MAX_BITMAP_PIXELS: i64 = 1 << 29;

/// The `fp.GetPixel(x, y).A != 0` of the C#: whether `FillPolygon` would have painted the pixel.
fn polygon_covers(points: &[(i32, i32); 4], x: i32, y: i32) -> bool {
    let (px, py) = (x as f64, y as f64);
    let mut inside = false;
    for i in 0..points.len() {
        let (x0, y0) = points[i];
        let (x1, y1) = points[(i + 1) % points.len()];
        if (y0 <= y) != (y1 <= y) {
            let t = (py - y0 as f64) / (y1 as f64 - y0 as f64);
            let xc = x0 as f64 + t * (x1 as f64 - x0 as f64);
            if xc > px {
                inside = !inside;
            }
        }
    }
    inside
}

impl Shape {
    /// `new PreviewNet(point1, selectedNode, zoomLevel)`.
    pub fn new_net(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let mut s = Shape::constructed(ShapeKind::Net { pixel_spacing: 8, lock_xy: false, initially_assigned_node: None, loaded_pixels: None });
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
        ];
        s.reconfigure_net(node);
        s
    }

    pub(crate) fn reconfigure_net(&mut self, node: Option<Sel>) {
        self.pixels.clear();
        let leaf = node.filter(|n| n.is_leaf()).map(|n| n.uuid());
        if let ShapeKind::Net { initially_assigned_node, .. } = &mut self.kind {
            // Only a leaf is ever used, so only a leaf needs keeping.
            *initially_assigned_node = leaf;
        }
        self.layout();
    }

    /// Lays out a net just read from a file. If this port would place a different number of lights than
    /// the file holds, the file's list stays for saving until the net is next edited.
    pub(crate) fn layout_net_after_load(&mut self) {
        let stored = self.pixels.clone();
        self.layout_net();
        let same = stored.len() == self.pixels.len() && stored.iter().zip(&self.pixels).all(|(a, b)| a.node_id == b.node_id);
        if !same {
            if let ShapeKind::Net { loaded_pixels, .. } = &mut self.kind {
                *loaded_pixels = Some(stored);
            }
        }
    }

    pub(crate) fn layout_net(&mut self) {
        let ShapeKind::Net { pixel_spacing, initially_assigned_node, .. } = self.kind else { return };
        if let ShapeKind::Net { loaded_pixels, .. } = &mut self.kind {
            *loaded_pixels = None;
        }
        let mut node = Uuid::nil();
        if let Some(p) = self.pixels.first() {
            node = p.node_id;
            self.pixels.clear();
        } else if let Some(n) = initially_assigned_node {
            node = n;
        }

        let p = &self.points;
        let (tl, tr, bl, br) = (p[TL], p[TR], p[BL], p[BR]);
        let bx = tl.x.min(tr.x.min(br.x).min(bl.x));
        let by = tl.y.min(tr.y.min(br.y).min(bl.y));
        let ex = tl.x.max(tr.x.max(br.x).max(bl.x));
        let ey = tl.y.max(tr.y.max(br.y).max(bl.y));
        let (w, h) = (ex.wrapping_sub(bx), ey.wrapping_sub(by));
        let rel = |q: PreviewPoint| (q.x.wrapping_sub(bx), q.y.wrapping_sub(by));
        let polygon = [rel(tl), rel(tr), rel(br), rel(bl)];

        // A spacing of zero divides by zero in C#, and `new Bitmap` throws for a size it cannot allocate.
        let step = pixel_spacing.checked_abs().unwrap_or(0);
        if w > 0 && h > 0 && step != 0 && (w as i64) * (h as i64) <= MAX_BITMAP_PIXELS {
            let size = self.pixel_size;
            let mut x_count = 1;
            let mut y = 0;
            while y < h {
                // spacingY starts at the spacing and goes up with y, so this is y % spacing == 0.
                if y % step == 0 {
                    let x_div = if x_count % 2 == 0 { pixel_spacing } else { pixel_spacing / 2 };
                    // The x values that pass (x + xDiv) % spacing == 0, without trying the others.
                    let mut x = x_div.rem_euclid(step);
                    if x != 0 {
                        x = step - x;
                    }
                    while x < w {
                        if polygon_covers(&polygon, x, y) {
                            let mut pixel = PreviewPixel::new(x.wrapping_add(bx), y.wrapping_add(by), 0, size);
                            pixel.node_id = node;
                            self.pixels.push(pixel);
                        }
                        x += step;
                    }
                    x_count += 1;
                }
                y += 1;
            }
            self.set_pixel_zoom_rotate();
        }
    }

    pub(crate) fn mouse_move_net(&mut self, x: i32, y: i32, mut change_x: i32, mut change_y: i32, m: Modifiers) {
        if let Some(sel) = self.selected_point {
            let zoom = self.zoom();
            let point = self.transform_preview_point(PreviewPoint::new(x, y), -zoom, RotateTypes::Counterclockwise);
            self.set_point_xy(sel, point.x, point.y);
            let lock_xy = matches!(self.kind, ShapeKind::Net { lock_xy: true, .. });
            if lock_xy || (sel == PointRef::Pt(BR) && m.only_ctrl()) {
                self.points[TR].x = point.x;
                self.points[BL].y = point.y;
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

    pub(crate) fn match_net(&mut self, other: &Shape) {
        self.set_pixel_size(other.pixel_size());
        if let (ShapeKind::Net { pixel_spacing, .. }, ShapeKind::Net { pixel_spacing: o, .. }) = (&mut self.kind, &other.kind) {
            *pixel_spacing = *o;
        }
        let tl = self.points[TL];
        let o = &other.points;
        for i in [BR, BL, TR] {
            self.points[i].x = tl.x.wrapping_add(o[i].x.wrapping_sub(o[TL].x));
            self.points[i].y = tl.y.wrapping_add(o[i].y.wrapping_sub(o[TL].y));
        }
        self.rotation_angle = other.rotation_angle;
        self.layout();
    }
}

#[cfg(test)]
mod tests {
    use super::polygon_covers;

    #[test]
    fn fill_rule_is_top_left_in_bottom_right_out() {
        let square = [(0, 0), (10, 0), (10, 10), (0, 10)];
        assert!(polygon_covers(&square, 0, 0));
        assert!(polygon_covers(&square, 9, 9));
        assert!(!polygon_covers(&square, 10, 5));
        assert!(!polygon_covers(&square, 5, 10));
    }
}
