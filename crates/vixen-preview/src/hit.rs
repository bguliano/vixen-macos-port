//! Fast hit testing. The C# tests the mouse against every pixel of every shape. Here each display item
//! keeps a bounding box, built when first needed after the shape changes, so a miss costs one comparison
//! per item, and a shape big enough for a pixel scan to matter also gets a uniform grid.

use crate::data::DisplayItem;
use crate::point::PreviewPoint;
use crate::shape::{Shape, ShapeKind, SELECT_POINT_SIZE};
use crate::tools::{to_int32, Rect};

/// A shape with fewer pixels than this is just scanned once its bounding box is hit.
const GRID_THRESHOLD: usize = 512;
const GRID_MIN_CELL: i32 = 16;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct IRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl IRect {
    #[inline]
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }

    #[inline]
    fn grow(&mut self, r: IRect) {
        self.x0 = self.x0.min(r.x0);
        self.y0 = self.y0.min(r.y0);
        self.x1 = self.x1.max(r.x1);
        self.y1 = self.y1.max(r.y1);
    }
}

/// Pixel hit rectangles bucketed by cell: `starts` indexes `rects`, one run per cell.
#[derive(Clone, Debug)]
struct HitGrid {
    x0: i64,
    y0: i64,
    cell: i64,
    cols: i64,
    rows: i64,
    starts: Vec<u32>,
    rects: Vec<IRect>,
}

impl HitGrid {
    fn build(bounds: IRect, source: &[IRect]) -> HitGrid {
        let width = bounds.x1 as i64 - bounds.x0 as i64 + 1;
        let height = bounds.y1 as i64 - bounds.y0 as i64 + 1;
        // About one rectangle per cell, with a floor on the cell size and a ceiling on the cell count.
        let ideal = ((width as f64 * height as f64) / source.len().max(1) as f64).sqrt() as i64;
        let mut cell = ideal.max(GRID_MIN_CELL as i64);
        while (width / cell + 1) * (height / cell + 1) > 1 << 20 {
            cell *= 2;
        }
        let cols = width / cell + 1;
        let rows = height / cell + 1;
        let span = |r: &IRect| {
            let cx0 = (r.x0 as i64 - bounds.x0 as i64) / cell;
            let cx1 = (r.x1 as i64 - bounds.x0 as i64) / cell;
            let cy0 = (r.y0 as i64 - bounds.y0 as i64) / cell;
            let cy1 = (r.y1 as i64 - bounds.y0 as i64) / cell;
            (cx0, cx1, cy0, cy1)
        };
        let mut starts = vec![0u32; (cols * rows + 1) as usize];
        for r in source {
            let (cx0, cx1, cy0, cy1) = span(r);
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    starts[(cy * cols + cx) as usize + 1] += 1;
                }
            }
        }
        for i in 1..starts.len() {
            starts[i] += starts[i - 1];
        }
        let mut fill = starts.clone();
        let mut rects = vec![IRect { x0: 0, y0: 0, x1: 0, y1: 0 }; *starts.last().unwrap_or(&0) as usize];
        for r in source {
            let (cx0, cx1, cy0, cy1) = span(r);
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    let slot = &mut fill[(cy * cols + cx) as usize];
                    rects[*slot as usize] = *r;
                    *slot += 1;
                }
            }
        }
        HitGrid { x0: bounds.x0 as i64, y0: bounds.y0 as i64, cell, cols, rows, starts, rects }
    }

    fn hit(&self, x: i32, y: i32) -> bool {
        let cx = (x as i64 - self.x0).div_euclid(self.cell);
        let cy = (y as i64 - self.y0).div_euclid(self.cell);
        if cx < 0 || cy < 0 || cx >= self.cols || cy >= self.rows {
            return false;
        }
        let i = (cy * self.cols + cx) as usize;
        let (a, b) = (self.starts[i] as usize, self.starts[i + 1] as usize);
        self.rects[a..b].iter().any(|r| r.contains(x, y))
    }
}

/// What a display item knows about where it is, for as long as its shape does not change.
#[derive(Clone, Debug, Default)]
pub struct HitCache {
    hit_bounds: Option<IRect>,
    pixel_bounds: Option<(f64, f64, f64, f64)>,
    grid: Option<HitGrid>,
}

/// How a shape's `PointInShape` works, for choosing how to speed it up.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum HitModel {
    /// A test against each pixel's rectangle.
    Pixels,
    /// The ellipse's: the same test, in doubly zoomed space.
    ZoomedPixels,
    /// Something that does not scan pixels at all.
    Direct,
}

fn hit_model(s: &Shape) -> HitModel {
    match s.kind() {
        ShapeKind::Ellipse => HitModel::ZoomedPixels,
        ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } => HitModel::Pixels,
        _ if s.pixel_hit_width().is_some() => HitModel::Pixels,
        _ => HitModel::Direct,
    }
}

/// Runs `f` on each pixel's hit rectangle in `Pixels` order.
fn for_each_hit_rect(s: &Shape, f: &mut impl FnMut(IRect)) {
    let half = SELECT_POINT_SIZE / 2;
    match s.kind() {
        ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } => {
            // Each line's own test, with the line's own light size.
            for line in s.strings().unwrap_or(&[]) {
                for_each_hit_rect(line, f);
            }
        }
        ShapeKind::Ellipse => {
            let zoom = s.zoom();
            let w = SELECT_POINT_SIZE + s.pixel_size();
            s.for_each_pixel(&mut |p| {
                let x0 = to_int32(p.x as f64 * zoom).wrapping_sub(half);
                let y0 = to_int32(p.y as f64 * zoom).wrapping_sub(half);
                f(IRect { x0, y0, x1: x0.wrapping_add(w), y1: y0.wrapping_add(w) });
            });
        }
        _ => {
            if let Some(w) = s.pixel_hit_width() {
                s.for_each_pixel(&mut |p| {
                    let (x0, y0) = (p.x.wrapping_sub(half), p.y.wrapping_sub(half));
                    f(IRect { x0, y0, x1: x0.wrapping_add(w), y1: y0.wrapping_add(w) });
                });
            }
        }
    }
}

impl HitCache {
    fn build(s: &Shape) -> HitCache {
        let mut cache = HitCache::default();
        if s.is_light_shape() {
            // The positions ShapeInRect compares: Location on a high precision shape, X and Y otherwise.
            let hp = s.is_high_precision();
            let mut b: Option<(f64, f64, f64, f64)> = None;
            s.for_each_pixel(&mut |p| {
                let (x, y) = if hp { p.location } else { (p.x as f64, p.y as f64) };
                b = Some(match b {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            });
            cache.pixel_bounds = b;
        }
        if hit_model(s) == HitModel::Direct {
            return cache;
        }
        let large = s.pixel_count() >= GRID_THRESHOLD;
        let mut rects: Vec<IRect> = Vec::with_capacity(if large { s.pixel_count() } else { 0 });
        let mut bounds: Option<IRect> = None;
        for_each_hit_rect(s, &mut |r| {
            match &mut bounds {
                Some(b) => b.grow(r),
                None => bounds = Some(r),
            }
            if large {
                rects.push(r);
            }
        });
        cache.hit_bounds = bounds;
        if let (true, Some(b)) = (large, bounds) {
            cache.grid = Some(HitGrid::build(b, &rects));
        }
        cache
    }
}

impl DisplayItem {
    /// Forgets the cached bounds. Call after anything that can move the shape's lights.
    #[inline]
    pub(crate) fn invalidate_hit(&mut self) {
        self.hit = None;
    }

    fn hit_cache(&mut self) -> &HitCache {
        if self.hit.is_none() {
            self.hit = Some(Box::new(HitCache::build(&self.shape)));
        }
        self.hit.as_deref().expect("just built")
    }

    /// `Shape.PointInShape`, with the bounds in front of it.
    pub fn point_in_shape(&mut self, point: PreviewPoint) -> bool {
        let model = hit_model(&self.shape);
        if model == HitModel::Direct {
            return self.shape.point_in_shape(point);
        }
        // The ellipse's PointInShape compares in doubly zoomed space, so its rectangles are in that space.
        let q = if model == HitModel::ZoomedPixels { self.shape.point_to_zoom_point_add(point) } else { point };
        let cache = self.hit_cache();
        match &cache.hit_bounds {
            Some(b) if b.contains(q.x, q.y) => {}
            _ => return false,
        }
        if let Some(g) = &cache.grid {
            return g.hit(q.x, q.y);
        }
        self.shape.point_in_shape(point)
    }

    /// `Shape.ShapeInRect`, with the bounds in front of it: only a shape straddling the rectangle's edge,
    /// when any one light is enough, needs its pixels looked at.
    pub fn shape_in_rect(&mut self, rect: Rect, all_in: bool) -> bool {
        if !self.shape.is_light_shape() {
            return self.shape.shape_in_rect(rect, all_in);
        }
        let (x1, y1, x2, y2) = rect.normalized();
        let (x1, y1, x2, y2) = (x1 as f64, y1 as f64, x2 as f64, y2 as f64);
        let Some((bx0, by0, bx1, by1)) = self.hit_cache().pixel_bounds else { return false };
        if bx0 >= x1 && bx1 <= x2 && by0 >= y1 && by1 <= y2 {
            return true;
        }
        if bx1 < x1 || bx0 > x2 || by1 < y1 || by0 > y2 {
            return false;
        }
        if all_in {
            return false;
        }
        self.shape.shape_in_rect(rect, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ShapeId;

    fn line(n: i32) -> DisplayItem {
        let s = Shape::new_line(PreviewPoint::new(0, 0), PreviewPoint::new(n * 2, 0), n, None, 1.0);
        DisplayItem::new(ShapeId(1), s)
    }

    /// Whatever the bounds and the grid decide must match the exact port.
    fn assert_matches_exact(item: &mut DisplayItem, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>) {
        for y in ys {
            for x in xs.clone() {
                let p = PreviewPoint::new(x, y);
                assert_eq!(item.point_in_shape(p), item.shape.point_in_shape(p), "at {x},{y}");
            }
        }
    }

    #[test]
    fn bounds_agree_with_the_pixel_scan() {
        let mut item = line(20);
        assert_matches_exact(&mut item, -8..50, -12..12);
    }

    #[test]
    fn the_grid_agrees_with_the_pixel_scan() {
        let mut item = line(2000);
        assert!(item.hit_cache().grid.is_some());
        assert_matches_exact(&mut item, -8..200, -12..12);
        assert_matches_exact(&mut item, 3900..4020, -12..12);
    }

    #[test]
    fn rubber_band_shortcuts_agree_with_the_pixel_scan() {
        let mut item = line(20);
        for rect in [Rect::new(-5, -5, 60, 10), Rect::new(10, -5, 10, 10), Rect::new(100, 100, 5, 5), Rect::new(40, 5, -30, -10), Rect::new(0, 1, 40, 1)] {
            for all_in in [false, true] {
                assert_eq!(item.shape_in_rect(rect, all_in), item.shape.shape_in_rect(rect, all_in), "{rect:?} {all_in}");
            }
        }
    }
}
