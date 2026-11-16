//! A System.Drawing-style canvas for the effects that draw into bitmaps (Text, CountDown, Shapes,
//! Picture, Morph and others) and for custom timeline thumbnails. Shapes use tiny-skia, and text uses
//! Core Text so the same font names resolve as on a real Mac.
//!
//! Sizes follow GDI+ at 96 DPI: 1 unit is 1 pixel, and a font of N points is N * 96 / 72 pixels tall.

use core_foundation::attributed_string::CFMutableAttributedString;
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;
use core_graphics::base::kCGImageAlphaPremultipliedLast;
use core_graphics::color_space::CGColorSpace;
use core_graphics::context::CGContext;
use core_text::font as ct_font;
use core_text::font_descriptor::{kCTFontBoldTrait, kCTFontItalicTrait};
use core_text::line::CTLine;
use tiny_skia::{FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};
use vixen_core::Color;

use crate::framework::FontDesc;

#[path = "gfx_nominal.rs"]
mod nominal;

/// `System.Drawing.Bitmap`: authoritative straight colors with a derived premultiplied drawing plane.
#[derive(Clone)]
pub struct Bitmap {
    pixmap: Pixmap,
    pixels: Vec<Color>,
}

impl std::fmt::Debug for Bitmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bitmap({}x{})", self.width(), self.height())
    }
}

impl Bitmap {
    /// A cleared, fully transparent bitmap. Zero-sized requests become 1x1, as tiny-skia has no empty pixmap.
    pub fn new(width: i32, height: i32) -> Bitmap {
        let pixmap = Pixmap::new(width.max(1) as u32, height.max(1) as u32).expect("bitmap dimensions");
        let pixels = vec![Color::EMPTY; pixmap.width() as usize * pixmap.height() as usize];
        Bitmap { pixmap, pixels }
    }

    /// Loads PNG, JPEG, GIF or BMP.
    pub fn from_bytes(bytes: &[u8]) -> Result<Bitmap, String> {
        let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?.to_rgba8();
        Ok(Bitmap::from_rgba(img.width() as i32, img.height() as i32, img.as_raw()))
    }

    pub fn from_file(path: &std::path::Path) -> Result<Bitmap, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Bitmap::from_bytes(&bytes)
    }

    /// From straight (not premultiplied) RGBA bytes.
    pub fn from_rgba(width: i32, height: i32, rgba: &[u8]) -> Bitmap {
        let mut b = Bitmap::new(width, height);
        let w = width.max(1) as usize;
        for (i, px) in rgba.chunks_exact(4).enumerate() {
            let (x, y) = (i % w, i / w);
            if y < b.height() as usize {
                b.set_pixel(x as i32, y as i32, Color::argb(px[3], px[0], px[1], px[2]));
            }
        }
        b
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.pixmap.width() as i32
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.pixmap.height() as i32
    }

    /// `Bitmap.GetPixel`. Out of range reads return `Color::EMPTY` rather than throwing.
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Color {
        if x < 0 || y < 0 || x >= self.width() || y >= self.height() {
            return Color::EMPTY;
        }
        self.pixels[y as usize * self.pixmap.width() as usize + x as usize]
    }

    /// `Bitmap.SetPixel`.
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, c: Color) {
        if x < 0 || y < 0 || x >= self.width() || y >= self.height() {
            return;
        }
        let i = y as usize * self.pixmap.width() as usize + x as usize;
        self.pixels[i] = c;
        self.pixmap.pixels_mut()[i] = tiny_skia::ColorU8::from_rgba(c.r, c.g, c.b, c.a).premultiply();
    }

    /// `new Bitmap(source, width, height)`: a bilinear rescale.
    pub fn resized(&self, width: i32, height: i32) -> Bitmap {
        let mut out = Graphics::new(width, height);
        out.draw_image(self, 0.0, 0.0, width as f32, height as f32);
        out.into_bitmap()
    }

    /// Straight RGBA bytes, for handing to the UI.
    pub fn to_rgba(&self) -> Vec<u8> {
        self.pixels.iter().flat_map(|c| [c.r, c.g, c.b, c.a]).collect()
    }

    // GDI's captured drawing readback uses a truncated 16-bit reciprocal. The
    // resulting straight color, rather than the intermediate, is stored for later draws.
    fn drawn_color(a: u8, r: u8, g: u8, b: u8) -> Color {
        if a == 0 { return Color::EMPTY; }
        let reciprocal = (255u32 << 16) / a as u32;
        let straight = |v: u8| ((v as u32 * reciprocal) >> 16).min(255) as u8;
        Color::argb(a, straight(r), straight(g), straight(b))
    }

    fn commit(&mut self, i: usize, c: Color) {
        self.pixels[i] = c;
        self.pixmap.pixels_mut()[i] = tiny_skia::ColorU8::from_rgba(c.r, c.g, c.b, c.a).premultiply();
    }

    // A separate coverage render uses the identical raster operation and transform.
    // It marks actual writes even for alpha-zero paint or unchanged premul bytes.
    // Pixels outside its footprint keep their exact authoritative straight values.
    fn raster(&mut self, color: Option<Color>, bounds: Option<Rect>, draw: impl Fn(&mut Pixmap, bool, Transform)) {
        let (left, top, right, bottom) = if color.is_some() {
            bounds.map(|r| ((r.left().floor() - 1.0).clamp(0.0, self.width() as f32) as i32,
                            (r.top().floor() - 1.0).clamp(0.0, self.height() as f32) as i32,
                            (r.right().ceil() + 1.0).clamp(0.0, self.width() as f32) as i32,
                            (r.bottom().ceil() + 1.0).clamp(0.0, self.height() as f32) as i32))
                .unwrap_or((0, 0, self.width(), self.height()))
        } else { (0, 0, self.width(), self.height()) };
        if right <= left || bottom <= top { return; }
        let mut coverage = Pixmap::new((right-left) as u32, (bottom-top) as u32).unwrap();
        let offset = Transform::from_translate(-left as f32, -top as f32);
        draw(&mut coverage, true, offset);
        if let Some(c) = color {
            let p = tiny_skia::ColorU8::from_rgba(c.r, c.g, c.b, c.a).premultiply();
            for (i, coverage) in coverage.pixels().iter().enumerate() {
                let k = coverage.alpha() as u32;
                if k != 0 {
                    let source = [p.alpha(), p.red(), p.green(), p.blue()].map(|v| ((v as u32 * k + 127) / 255) as u8);
                    self.blend_premultiplied(left + (i % (right-left) as usize) as i32, top + (i / (right-left) as usize) as i32, source);
                }
            }
        } else {
            draw(&mut self.pixmap, false, Transform::identity());
            for (i, coverage) in coverage.pixels().iter().enumerate() {
                if coverage.alpha() != 0 {
                    let p = self.pixmap.pixels()[i];
                    self.commit(i, Self::drawn_color(p.alpha(), p.red(), p.green(), p.blue()));
                }
            }
        }
    }

    fn blend_premultiplied(&mut self, x: i32, y: i32, source: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width() || y >= self.height() { return; }
        let i = y as usize * self.pixmap.width() as usize + x as usize;
        let d = self.pixmap.pixels()[i];
        let inv = 255 - source[0] as u32;
        let over = |s: u8, d: u8| (s as u32 + (d as u32 * inv + 127) / 255).min(255) as u8;
        let a = over(source[0], d.alpha());
        self.commit(i, Self::drawn_color(a, over(source[1], d.red()), over(source[2], d.green()), over(source[3], d.blue())));
    }

    fn blend_color(&mut self, x: i32, y: i32, c: Color) {
        let p = tiny_skia::ColorU8::from_rgba(c.r, c.g, c.b, c.a).premultiply();
        self.blend_premultiplied(x, y, [p.alpha(), p.red(), p.green(), p.blue()]);
    }

    fn draw_resized(&mut self, image: &Bitmap, x: i32, y: i32, w: i32, h: i32) {
        let scale_x = image.width() as f32 / w as f32;
        let scale_y = image.height() as f32 / h as f32;
        let dx0 = (-(x as i64)).max(0).min(w as i64) as i32;
        let dx1 = (self.width() as i64 - x as i64).max(0).min(w as i64) as i32;
        let dy0 = (-(y as i64)).max(0).min(h as i64) as i32;
        let dy1 = (self.height() as i64 - y as i64).max(0).min(h as i64) as i32;
        // Captured bilinear output quantizes each nonnegative source coordinate
        // to 11 fractional bits before deriving the four interpolation weights.
        // Quantize in f64 so large f32 coordinates do not gain a rounding bias;
        // clipped destination indices stay nonnegative even for negative x/y.
        // Keep out-of-source neighbors transparent, including a rounded border.
        for dy in dy0..dy1 {
            let ty = y as i64 + dy as i64;
            if !(0..self.height() as i64).contains(&ty) { continue; }
            let sy = ((((dy as f32 * scale_y) as f64 * 2048.0).round()) / 2048.0) as f32; let iy = sy.floor() as i32; let fy = sy - iy as f32;
            for dx in dx0..dx1 {
                let tx = x as i64 + dx as i64;
                if !(0..self.width() as i64).contains(&tx) { continue; }
                let sx = ((((dx as f32 * scale_x) as f64 * 2048.0).round()) / 2048.0) as f32; let ix = sx.floor() as i32; let fx = sx - ix as f32;
                let weights = [(1.0-fx as f64)*(1.0-fy as f64), fx as f64*(1.0-fy as f64),
                               (1.0-fx as f64)*fy as f64, fx as f64*fy as f64];
                let mut accum = [0.0f32; 4];
                for ((xx, yy), weight) in [(ix,iy),(ix+1,iy),(ix,iy+1),(ix+1,iy+1)].into_iter().zip(weights) {
                    if xx >= 0 && yy >= 0 && xx < image.width() && yy < image.height() {
                        let p = image.pixmap.pixels()[yy as usize*image.width() as usize+xx as usize];
                        for (v,c) in accum.iter_mut().zip([p.alpha(),p.red(),p.green(),p.blue()]) {
                            *v += (c as f64 * weight) as f32;
                        }
                    }
                }
                self.blend_premultiplied(tx as i32, ty as i32, accum.map(|v| v.round().clamp(0.0,255.0) as u8));
            }
        }
    }
}

/// `System.Drawing.Graphics` drawing onto an owned bitmap.
pub struct Graphics {
    bitmap: Bitmap,
    transform: Transform,
    /// `SmoothingMode.AntiAlias`. GDI+ defaults to off.
    pub anti_alias: bool,
    /// `InterpolationMode`: true for nearest neighbour.
    pub nearest_neighbor: bool,
}

impl Graphics {
    pub fn new(width: i32, height: i32) -> Graphics {
        Graphics { bitmap: Bitmap::new(width, height), transform: Transform::identity(), anti_alias: false, nearest_neighbor: false }
    }

    pub fn from_bitmap(bitmap: Bitmap) -> Graphics {
        Graphics { bitmap, transform: Transform::identity(), anti_alias: false, nearest_neighbor: false }
    }

    pub fn into_bitmap(self) -> Bitmap {
        self.bitmap
    }

    pub fn bitmap(&self) -> &Bitmap {
        &self.bitmap
    }

    pub fn bitmap_mut(&mut self) -> &mut Bitmap {
        &mut self.bitmap
    }

    pub fn width(&self) -> i32 {
        self.bitmap.width()
    }

    pub fn height(&self) -> i32 {
        self.bitmap.height()
    }

    /// GDI+'s default `PixelOffsetMode` puts pixel centres on integer coordinates, where tiny-skia puts them half a pixel in.
    /// The two also break ties opposite ways: GDI+ lights a pixel whose centre is exactly on a left or top edge and tiny-skia
    /// doesn't, so the shift stops one step of tiny-skia's 1/64 pixel grid short.
    fn device_transform(&self) -> Transform {
        const OFFSET: f32 = 0.5 - 1.0 / 64.0;
        self.transform.post_translate(OFFSET, OFFSET)
    }

    fn paint(&self, c: Color) -> Paint<'static> {
        let mut p = Paint::default();
        p.set_color_rgba8(c.r, c.g, c.b, c.a);
        p.anti_alias = self.anti_alias;
        p
    }

    /// `Graphics.Clear`.
    pub fn clear(&mut self, c: Color) {
        let p = tiny_skia::ColorU8::from_rgba(c.r, c.g, c.b, c.a).premultiply();
        let c = Bitmap::drawn_color(p.alpha(), p.red(), p.green(), p.blue());
        for i in 0..self.bitmap.pixels.len() { self.bitmap.commit(i, c); }
    }

    /// `TranslateTransform`.
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.transform = self.transform.pre_translate(dx, dy);
    }

    /// `RotateTransform`, in degrees clockwise.
    pub fn rotate(&mut self, degrees: f32) {
        self.transform = self.transform.pre_concat(Transform::from_rotate(degrees));
    }

    /// `ScaleTransform`.
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.transform = self.transform.pre_scale(sx, sy);
    }

    /// `ResetTransform`.
    pub fn reset_transform(&mut self) {
        self.transform = Transform::identity();
    }

    /// `FillRectangle`.
    pub fn fill_rectangle(&mut self, c: Color, x: f32, y: f32, w: f32, h: f32) {
        if let Some(r) = Rect::from_xywh(x, y, w, h) {
            let paint = self.paint(c);
            let coverage = self.paint(Color::WHITE);
            let transform = self.device_transform();
            self.bitmap.raster(Some(c), r.transform(transform), |pixmap, mask, offset| pixmap.fill_rect(r, if mask { &coverage } else { &paint }, transform.post_concat(offset), None));
        }
    }

    /// `DrawRectangle`.
    pub fn draw_rectangle(&mut self, c: Color, pen_width: f32, x: f32, y: f32, w: f32, h: f32) {
        self.draw_polygon(c, pen_width, &[(x, y), (x + w, y), (x + w, y + h), (x, y + h)]);
    }

    /// `DrawLine`.
    pub fn draw_line(&mut self, c: Color, pen_width: f32, x0: f32, y0: f32, x1: f32, y1: f32) {
        if self.is_thin_aliased(pen_width) {
            self.thin_line(c, (x0, y0), (x1, y1));
            return;
        }
        let mut pb = PathBuilder::new();
        pb.move_to(x0, y0);
        pb.line_to(x1, y1);
        self.stroke(pb, c, pen_width);
    }

    /// `DrawLines`.
    pub fn draw_lines(&mut self, c: Color, pen_width: f32, points: &[(f32, f32)]) {
        if self.is_thin_aliased(pen_width) {
            for pair in points.windows(2) {
                self.thin_line(c, pair[0], pair[1]);
            }
            return;
        }
        if let Some(pb) = polyline(points, false) {
            self.stroke(pb, c, pen_width);
        }
    }

    /// `FillPolygon`, whose default is `FillMode.Alternate`.
    pub fn fill_polygon(&mut self, c: Color, points: &[(f32, f32)]) {
        if !self.anti_alias {
            self.fill_aliased_polygon(c, points, false);
            return;
        }
        if let Some(pb) = polyline(points, true) {
            self.fill(pb, c, FillRule::EvenOdd);
        }
    }

    /// `FillPolygon` with `FillMode.Winding`.
    pub fn fill_polygon_winding(&mut self, c: Color, points: &[(f32, f32)]) {
        if !self.anti_alias {
            self.fill_aliased_polygon(c, points, true);
            return;
        }
        if let Some(pb) = polyline(points, true) {
            self.fill(pb, c, FillRule::Winding);
        }
    }

    /// GDI's unsmoothed polygon coverage samples integer device coordinates,
    /// including the left/top boundary and excluding the right/bottom one.
    /// An epsilon translation through tiny-skia loses a vertex exactly on a
    /// scanline; evaluate the transformed edges directly instead.
    fn fill_aliased_polygon(&mut self, c: Color, points: &[(f32, f32)], winding: bool) {
        if points.len() < 3 {
            return;
        }
        let mut points: Vec<_> = points.iter().map(|&(x, y)| tiny_skia::Point::from_xy(x, y)).collect();
        self.transform.map_points(&mut points);
        if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return;
        }
        let min_y = points.iter().map(|p| p.y as f64).fold(f64::INFINITY, f64::min);
        let max_y = points.iter().map(|p| p.y as f64).fold(f64::NEG_INFINITY, f64::max);
        let first_y = min_y.ceil().clamp(0.0, self.height() as f64) as i32;
        let end_y = max_y.ceil().clamp(0.0, self.height() as f64) as i32;
        let mut crossings = Vec::with_capacity(points.len());
        for y in first_y..end_y {
            crossings.clear();
            for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
                let (ay, by) = (a.y as f64, b.y as f64);
                if ay.min(by) <= y as f64 && (y as f64) < ay.max(by) {
                    let x = a.x as f64 + (y as f64 - ay) * (b.x as f64 - a.x as f64) / (by - ay);
                    crossings.push((x, if by > ay { 1isize } else { -1isize }));
                }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut count = 0isize;
            let mut previous = 0.0f64;
            for &(x, direction) in &crossings {
                if if winding { count != 0 } else { count % 2 != 0 } {
                    let left = previous.ceil().clamp(0.0, self.width() as f64) as i32;
                    let right = x.ceil().clamp(0.0, self.width() as f64) as i32;
                    if right > left {
                        for x in left..right { self.bitmap.blend_color(x, y, c); }
                    }
                }
                count += direction;
                previous = x;
            }
        }
    }

    /// `DrawPolygon`.
    pub fn draw_polygon(&mut self, c: Color, pen_width: f32, points: &[(f32, f32)]) {
        if self.is_thin_aliased(pen_width) {
            for (i, a) in points.iter().enumerate() {
                self.thin_line(c, *a, points[(i + 1) % points.len()]);
            }
            return;
        }
        if let Some(pb) = polyline(points, true) {
            self.stroke(pb, c, pen_width);
        }
    }

    /// `FillEllipse`.
    pub fn fill_ellipse(&mut self, c: Color, x: f32, y: f32, w: f32, h: f32) {
        if let Some(r) = Rect::from_xywh(x, y, w, h) {
            let mut pb = PathBuilder::new();
            pb.push_oval(r);
            self.fill(pb, c, FillRule::Winding);
        }
    }

    /// `DrawEllipse`.
    pub fn draw_ellipse(&mut self, c: Color, pen_width: f32, x: f32, y: f32, w: f32, h: f32) {
        if self.is_thin_aliased(pen_width) {
            // Opaque nominal curves use their diamond outline; straight paths
            // use the separate DDA. Keep other paint modes on their existing path.
            if c.a == 255 && self.transform.is_identity() && w > 0.0 && h > 0.0 {
                let curve = nominal::ellipse_points(x, y, w, h);
                let mut device: Vec<_> = curve.into_iter().map(|p| tiny_skia::Point::from_xy(p.0, p.1)).collect();
                self.transform.map_points(&mut device);
                let points: Vec<_> = device.iter().map(|p| (p.x, p.y)).collect();
                if let Some(pixels) = nominal::curved_path(&points, self.width(), self.height()) {
                    for (x, y) in pixels { self.bitmap.set_pixel(x, y, c); }
                } else if let Some(rect) = Rect::from_xywh(x, y, w, h) {
                    // Unsupported fixed-point extents use the bounded normal
                    // stroker instead of an oversized integer step calculation.
                    let mut pb = PathBuilder::new(); pb.push_oval(rect);
                    self.stroke(pb, c, pen_width);
                }
                return;
            }
            // Existing fallback for unsupported geometry and translucent paint.
            let (rx, ry) = (w / 2.0, h / 2.0);
            let radius = rx.abs().max(ry.abs());
            let steps = ((std::f32::consts::PI * radius / 2.0).ceil() as i32 * 2).clamp(8, 4096);
            let at = |i: i32| {
                let t = i as f32 * std::f32::consts::TAU / steps as f32;
                (x + rx + rx * t.cos(), y + ry + ry * t.sin())
            };
            for i in 0..steps {
                self.thin_line(c, at(i), at(i + 1));
            }
            return;
        }
        if let Some(r) = Rect::from_xywh(x, y, w, h) {
            let mut pb = PathBuilder::new();
            pb.push_oval(r);
            self.stroke(pb, c, pen_width);
        }
    }

    /// `FillPie`, with angles in degrees clockwise from the +X axis, as GDI+ measures them.
    pub fn fill_pie(&mut self, c: Color, x: f32, y: f32, w: f32, h: f32, start_deg: f32, sweep_deg: f32) {
        let (cx, cy, rx, ry) = (x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0);
        let steps = ((sweep_deg.abs() / 4.0).ceil() as i32).max(2);
        let mut pts = Vec::with_capacity(steps as usize + 2);
        pts.push((cx, cy));
        for i in 0..=steps {
            let a = (start_deg + sweep_deg * i as f32 / steps as f32).to_radians();
            pts.push((cx + rx * a.cos(), cy + ry * a.sin()));
        }
        self.fill_polygon(c, &pts);
    }

    fn fill(&mut self, pb: PathBuilder, c: Color, rule: FillRule) {
        if let Some(path) = pb.finish() {
            let paint = self.paint(c);
            let transform = self.device_transform();
            let coverage = self.paint(Color::WHITE);
            self.bitmap.raster(Some(c), path.bounds().transform(transform), |pixmap, mask, offset| pixmap.fill_path(&path, if mask { &coverage } else { &paint }, rule, transform.post_concat(offset), None));
        }
    }

    fn stroke(&mut self, pb: PathBuilder, c: Color, width: f32) {
        if let Some(path) = pb.finish() {
            let paint = self.paint(c);
            let stroke = Stroke { width: width.max(0.0), ..Stroke::default() };
            let transform = self.device_transform();
            let coverage = self.paint(Color::WHITE);
            let padding = stroke.width * stroke.miter_limit;
            let bounds = path.bounds().outset(padding, padding).and_then(|r| r.transform(transform));
            self.bitmap.raster(Some(c), bounds, |pixmap, mask, offset| pixmap.stroke_path(&path, if mask { &coverage } else { &paint }, &stroke, transform.post_concat(offset), None));
        }
    }

    /// A one pixel pen with no smoothing, which GDI+ draws differently from a stroked outline: both end points are lit.
    fn is_thin_aliased(&self, pen_width: f32) -> bool {
        !self.anti_alias && pen_width <= 1.0
    }

    fn thin_line(&mut self, c: Color, from: (f32, f32), to: (f32, f32)) {
        if c.a == 255 && self.transform.is_identity() {
            let mut points = [tiny_skia::Point::from_xy(from.0, from.1), tiny_skia::Point::from_xy(to.0, to.1)];
            self.transform.map_points(&mut points);
            if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) { return; }
            if let Some(pixels) = nominal::line((points[0].x, points[0].y), (points[1].x, points[1].y), self.width(), self.height()) {
                for (x, y) in pixels { self.bitmap.set_pixel(x, y, c); }
            } else {
                // Avoid an unbounded integer walk for coordinates beyond the
                // fixed-point kernel's checked range; retain the normal stroker.
                let mut pb = PathBuilder::new();
                pb.move_to(from.0, from.1); pb.line_to(to.0, to.1);
                self.stroke(pb, c, 1.0);
            }
            return;
        }
        let device = |p: (f32, f32), t: &Transform| {
            let mut point = [tiny_skia::Point::from_xy(p.0, p.1)];
            t.map_points(&mut point);
            (point[0].x.round() as i32, point[0].y.round() as i32)
        };
        let (mut x, mut y) = device(from, &self.transform);
        let (x1, y1) = device(to, &self.transform);
        // A zero length line draws nothing.
        if (x, y) == (x1, y1) {
            return;
        }
        let (dx, sx) = ((x1 - x).abs(), if x < x1 { 1 } else { -1 });
        let (dy, sy) = ((y1 - y).abs(), if y < y1 { 1 } else { -1 });
        let mut err = (if dx > dy { dx } else { -dy }) / 2;
        let opaque = c.a == 255;
        loop {
            if opaque {
                self.bitmap.set_pixel(x, y, c);
            } else if x >= 0 && y >= 0 && x < self.width() && y < self.height() {
                self.bitmap.blend_color(x, y, c);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = err;
            if e2 > -dx {
                err -= dy;
                x += sx;
            }
            if e2 < dy {
                err += dx;
                y += sy;
            }
        }
    }

    /// `DrawImage(image, x, y, width, height)`.
    pub fn draw_image(&mut self, image: &Bitmap, x: f32, y: f32, w: f32, h: f32) {
        if image.width() <= 0 || image.height() <= 0 {
            return;
        }
        if !self.nearest_neighbor && self.transform.is_identity()
            && [x, y, w, h].iter().all(|v| v.is_finite() && v.fract() == 0.0 && *v > i32::MIN as f32 && *v < i32::MAX as f32)
            && w > 0.0 && h > 0.0 {
            self.bitmap.draw_resized(image, x as i32, y as i32, w as i32, h as i32);
            return;
        }
        let t = self
            .transform
            .pre_translate(x, y)
            .pre_scale(w / image.width() as f32, h / image.height() as f32);
        let paint = PixmapPaint {
            quality: if self.nearest_neighbor { FilterQuality::Nearest } else { FilterQuality::Bilinear },
            ..PixmapPaint::default()
        };
        let mut footprint = Pixmap::new(image.width() as u32, image.height() as u32).unwrap();
        footprint.fill(tiny_skia::Color::WHITE);
        self.bitmap.raster(None, None, |pixmap, mask, _offset| pixmap.draw_pixmap(0, 0, if mask { footprint.as_ref() } else { image.pixmap.as_ref() }, &paint, t, None));
    }

    /// `DrawString(text, font, brush, x, y)`.
    pub fn draw_string(&mut self, text: &str, font: &FontDesc, c: Color, x: f32, y: f32) {
        let (mask, w, h, _) = render_text(text, font);
        if w == 0 || h == 0 {
            return;
        }
        let mut glyphs = Bitmap::new(w as i32, h as i32);
        let mut footprint = Bitmap::new(w as i32, h as i32);
        for py in 0..h {
            for px in 0..w {
                let cov = mask[py * w + px];
                if cov > 0 {
                    let alpha = (cov as u32 * c.a as u32 / 255) as u8;
                    glyphs.set_pixel(px as i32, py as i32, Color::argb(alpha, c.r, c.g, c.b));
                    footprint.set_pixel(px as i32, py as i32, Color::argb(cov, 255, 255, 255));
                }
            }
        }
        let t = self.transform.pre_translate(x, y);
        self.bitmap.raster(None, None, |pixmap, mask, _offset| pixmap.draw_pixmap(0, 0, if mask { footprint.pixmap.as_ref() } else { glyphs.pixmap.as_ref() }, &PixmapPaint::default(), t, None));
    }
}

fn polyline(points: &[(f32, f32)], close: bool) -> Option<PathBuilder> {
    let (first, rest) = points.split_first()?;
    let mut pb = PathBuilder::new();
    pb.move_to(first.0, first.1);
    for p in rest {
        pb.line_to(p.0, p.1);
    }
    if close {
        pb.close();
    }
    Some(pb)
}

/// The pixel size GDI+ uses for a font at 96 DPI.
#[inline]
pub fn font_pixels(points: f32) -> f64 {
    points as f64 * 96.0 / 72.0
}

fn make_font(font: &FontDesc) -> ct_font::CTFont {
    let size = font_pixels(font.size);
    let base = ct_font::new_from_name(&font.name, size).or_else(|_| ct_font::new_from_name("Arial", size)).expect("Arial is part of macOS");
    if !font.bold && !font.italic {
        return base;
    }
    let mut traits = 0;
    if font.bold {
        traits |= kCTFontBoldTrait;
    }
    if font.italic {
        traits |= kCTFontItalicTrait;
    }
    base.clone_with_symbolic_traits(traits, traits).unwrap_or(base)
}

fn make_line(text: &str, font: &ct_font::CTFont) -> CTLine {
    let cf_text = CFString::new(text);
    let mut attr = CFMutableAttributedString::new();
    attr.replace_str(&cf_text, core_foundation::base::CFRange::init(0, 0));
    let len = cf_text.char_len();
    unsafe {
        let key = core_text::string_attributes::kCTFontAttributeName;
        let value: CFType = TCFType::wrap_under_get_rule(font.as_CFTypeRef());
        attr.set_attribute(core_foundation::base::CFRange::init(0, len), key, &value);
    }
    CTLine::new_with_attributed_string(attr.as_concrete_TypeRef() as _)
}

/// `Graphics.MeasureString(text, font)`: width and line height in pixels.
pub fn measure_string(text: &str, font: &FontDesc) -> (f32, f32) {
    let f = make_font(font);
    let line_height = f.ascent() + f.descent() + f.leading();
    let mut width = 0.0f64;
    let mut lines = 0;
    for l in text.split('\n') {
        lines += 1;
        width = width.max(make_line(l, &f).get_typographic_bounds().width);
    }
    (width.ceil() as f32, (line_height * lines.max(1) as f64).ceil() as f32)
}

/// Renders text to an 8-bit coverage mask. Returns (mask, width, height, baseline).
fn render_text(text: &str, font: &FontDesc) -> (Vec<u8>, usize, usize, f64) {
    let f = make_font(font);
    let (mw, mh) = measure_string(text, font);
    let (w, h) = (mw.ceil() as usize + 2, mh.ceil() as usize + 2);
    if text.is_empty() || w <= 2 {
        return (Vec::new(), 0, 0, 0.0);
    }
    let cs = CGColorSpace::create_device_rgb();
    let mut ctx = CGContext::create_bitmap_context(None, w, h, 8, w * 4, &cs, kCGImageAlphaPremultipliedLast);
    ctx.set_should_antialias(true);
    ctx.set_allows_font_smoothing(false);
    ctx.set_rgb_fill_color(0.0, 0.0, 0.0, 1.0);
    let ascent = f.ascent();
    let line_height = f.ascent() + f.descent() + f.leading();
    for (i, l) in text.split('\n').enumerate() {
        if l.is_empty() {
            continue;
        }
        let line = make_line(l, &f);
        let baseline = h as f64 - ascent - i as f64 * line_height;
        ctx.set_text_position(0.0, baseline);
        line.draw(&ctx);
    }
    let stride = ctx.bytes_per_row();
    let data = ctx.data();
    let mut mask = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            mask[y * w + x] = data[y * stride + x * 4 + 3];
        }
    }
    (mask, w, h, ascent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_and_read_back() {
        let mut g = Graphics::new(10, 10);
        g.clear(Color::BLACK);
        g.fill_rectangle(Color::RED, 2.0, 2.0, 4.0, 4.0);
        assert_eq!(g.bitmap().get_pixel(3, 3), Color::RED);
        assert_eq!(g.bitmap().get_pixel(8, 8), Color::BLACK);
    }

    #[test]
    fn out_of_range_pixels_are_safe() {
        let g = Graphics::new(4, 4);
        assert_eq!(g.bitmap().get_pixel(-1, 99), Color::EMPTY);
    }

    #[test]
    fn transform_moves_drawing() {
        let mut g = Graphics::new(10, 10);
        g.translate(5.0, 5.0);
        g.fill_rectangle(Color::BLUE, 0.0, 0.0, 2.0, 2.0);
        assert_eq!(g.bitmap().get_pixel(6, 6), Color::BLUE);
        assert_eq!(g.bitmap().get_pixel(1, 1), Color::EMPTY);
    }

    #[test]
    fn text_draws_pixels_and_measures() {
        let font = FontDesc::new("Arial", 20.0);
        let (w, h) = measure_string("Vixen", &font);
        assert!(w > 20.0 && h > 15.0, "{w}x{h}");
        let mut g = Graphics::new(200, 60);
        g.draw_string("Vixen", &font, Color::WHITE, 2.0, 2.0);
        let lit = (0..60).flat_map(|y| (0..200).map(move |x| (x, y))).filter(|(x, y)| g.bitmap().get_pixel(*x, *y).a > 128).count();
        assert!(lit > 50, "only {lit} pixels lit");
    }

    #[test]
    fn missing_font_falls_back() {
        let (w, _) = measure_string("x", &FontDesc::new("No Such Font At All", 12.0));
        assert!(w > 0.0);
    }

    #[test]
    fn images_scale_on_draw() {
        let mut src = Bitmap::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                src.set_pixel(x, y, Color::RED);
            }
        }
        let big = src.resized(8, 8);
        assert_eq!(big.width(), 8);
        let c = big.get_pixel(4, 4);
        assert!(c.r > 200 && c.a > 200, "{c:?}");
    }
}

#[cfg(test)]
mod gdi_rules {
    use super::*;

    fn lit(g: &Graphics) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for y in 0..g.height() {
            for x in 0..g.width() {
                if g.bitmap().get_pixel(x, y).a != 0 {
                    out.push((x, y));
                }
            }
        }
        out
    }

    #[test]
    fn a_thin_line_lights_both_end_points() {
        let mut g = Graphics::new(12, 8);
        g.draw_line(Color::RED, 1.0, 1.0, 3.0, 10.0, 3.0);
        assert_eq!(lit(&g), (1..=10).map(|x| (x, 3)).collect::<Vec<_>>());
    }

    #[test]
    fn a_zero_length_line_draws_nothing() {
        let mut g = Graphics::new(4, 4);
        g.draw_line(Color::RED, 1.0, 2.0, 2.0, 2.0, 2.0);
        assert!(lit(&g).is_empty());
    }

    #[test]
    fn filled_shapes_exclude_their_right_and_bottom_edges() {
        let mut g = Graphics::new(12, 12);
        g.fill_polygon(Color::RED, &[(0.0, 0.0), (9.0, 0.0), (9.0, 9.0), (0.0, 9.0)]);
        let pixels = lit(&g);
        assert_eq!(pixels.len(), 81);
        assert!(pixels.contains(&(8, 8)));
        assert!(!pixels.contains(&(9, 8)));

        let mut g = Graphics::new(12, 12);
        g.fill_rectangle(Color::RED, 2.0, 3.0, 4.0, 5.0);
        assert_eq!(lit(&g).len(), 20);
        assert_eq!(g.bitmap().get_pixel(2, 3).a, 255);
        assert_eq!(g.bitmap().get_pixel(5, 7).a, 255);
        assert_eq!(g.bitmap().get_pixel(6, 7).a, 0);
    }

    #[test]
    fn a_rectangle_outline_is_one_pixel_wider_than_its_width() {
        let mut g = Graphics::new(12, 12);
        g.draw_rectangle(Color::RED, 1.0, 1.0, 1.0, 4.0, 3.0);
        assert_eq!(g.bitmap().get_pixel(5, 4).a, 255);
        assert_eq!(g.bitmap().get_pixel(6, 4).a, 0);
        assert_eq!(g.bitmap().get_pixel(3, 2).a, 0);
    }

    #[test]
    fn a_star_has_an_empty_centre_by_default() {
        let star: Vec<(f32, f32)> = (0..5)
            .map(|i| {
                let a = (-90.0 + i as f32 * 144.0).to_radians();
                (20.0 + 18.0 * a.cos(), 20.0 + 18.0 * a.sin())
            })
            .collect();
        let mut g = Graphics::new(40, 40);
        g.fill_polygon(Color::RED, &star);
        assert_eq!(g.bitmap().get_pixel(20, 20).a, 0, "FillMode.Alternate leaves the centre empty");
        let mut g = Graphics::new(40, 40);
        g.fill_polygon_winding(Color::RED, &star);
        assert_eq!(g.bitmap().get_pixel(20, 20).a, 255);
    }

    #[test]
    fn an_ellipse_outline_stays_one_pixel_thick_and_inside_its_box() {
        let mut g = Graphics::new(30, 30);
        g.draw_ellipse(Color::RED, 1.0, 5.0, 5.0, 20.0, 12.0);
        let pixels = lit(&g);
        assert!(!pixels.is_empty());
        assert!(pixels.iter().all(|(x, y)| (5..=25).contains(x) && (5..=17).contains(y)));
        // Across the middle row there are only the two sides.
        assert!(pixels.iter().filter(|(_, y)| *y == 11).count() <= 4);
    }

    #[test]
    fn polygon_masks_match_windows_gdi() {
        // Exact Framework GDI+ output from returned capture 0d8a00e1…78aa,
        // primitive-polygon-{alternate,winding}; every pixel is checked.
        let points = [(20.0, 2.0), (31.0, 35.0), (3.0, 14.0), (37.0, 14.0), (9.0, 35.0)];
        let alternate: [u64; 40] = [
            0x0000000000, 0x0000000000, 0x0000000000, 0x0000100000, 0x0000100000,
            0x0000180000, 0x0000380000, 0x0000380000, 0x00003c0000, 0x00007c0000,
            0x00007c0000, 0x00007e0000, 0x0000fe0000, 0x0000fe0000, 0x1fff00fff8,
            0x0ffe00ffe0, 0x07fe00ffc0, 0x01fe007f80, 0x00fc007e00, 0x007c007c00,
            0x001c003800, 0x0008002000, 0x0000000000, 0x0006006000, 0x000f01e000,
            0x000f83e000, 0x000fe7f000, 0x001feff000, 0x001fc7f000, 0x001f81f800,
            0x003e00f800, 0x003c007800, 0x0038001c00, 0x0060000c00, 0x0040000400,
            0x0000000000, 0x0000000000, 0x0000000000, 0x0000000000, 0x0000000000,
        ];
        let winding: [u64; 40] = [
            0x0000000000, 0x0000000000, 0x0000000000, 0x0000100000, 0x0000100000,
            0x0000180000, 0x0000380000, 0x0000380000, 0x00003c0000, 0x00007c0000,
            0x00007c0000, 0x00007e0000, 0x0000fe0000, 0x0000fe0000, 0x1ffffffff8,
            0x0fffffffe0, 0x07ffffffc0, 0x01ffffff80, 0x00fffffe00, 0x007ffffc00,
            0x001ffff800, 0x000fffe000, 0x0007ffc000, 0x0007ffe000, 0x000fffe000,
            0x000fffe000, 0x000ffff000, 0x001feff000, 0x001fc7f000, 0x001f81f800,
            0x003e00f800, 0x003c007800, 0x0038001c00, 0x0060000c00, 0x0040000400,
            0x0000000000, 0x0000000000, 0x0000000000, 0x0000000000, 0x0000000000,
        ];
        for (winding, expected) in [(false, alternate), (true, winding)] {
            let mut g = Graphics::new(40, 40);
            if winding { g.fill_polygon_winding(Color::WHITE, &points); }
            else { g.fill_polygon(Color::WHITE, &points); }
            for y in 0..40 {
                for x in 0..40 {
                    let expected = if expected[y as usize] & (1 << x) != 0 { Color::WHITE } else { Color::EMPTY };
                    assert_eq!(g.bitmap().get_pixel(x, y), expected, "winding={winding}, ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn aliased_polygon_translation_and_clipping_preserve_coverage() {
        let points = [(20.0, 2.0), (31.0, 35.0), (3.0, 14.0), (37.0, 14.0), (9.0, 35.0)];
        let mut full = Graphics::new(40, 40);
        full.fill_polygon(Color::WHITE, &points);
        let mut clipped = Graphics::new(20, 20);
        clipped.translate(-4.0, -10.0);
        clipped.fill_polygon(Color::WHITE, &points);
        for y in 0..20 {
            for x in 0..20 {
                assert_eq!(clipped.bitmap().get_pixel(x, y), full.bitmap().get_pixel(x + 4, y + 10));
            }
        }
        let before = clipped.bitmap().to_rgba();
        clipped.fill_polygon(Color::RED, &[(f32::NAN, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        assert_eq!(clipped.bitmap().to_rgba(), before);
    }

}

#[cfg(test)]
mod windows_storage {
    use super::*;

    #[test]
    fn integer_centered_resize_and_transparent_border_match_windows_examples() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/golden/picture-resize-windows.json"
        )).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let input: Vec<u8> = case["inputArgb"].as_array().unwrap().iter()
                .map(|x| u8::try_from(x.as_u64().unwrap()).unwrap()).collect();
            let rgba: Vec<_> = input.chunks_exact(4).flat_map(|p| [p[1], p[2], p[3], p[0]]).collect();
            let source = Bitmap::from_rgba(case["input"]["width"].as_i64().unwrap() as i32,
                case["input"]["height"].as_i64().unwrap() as i32, &rgba);
            let output = source.resized(case["outputWidth"].as_i64().unwrap() as i32,
                case["outputHeight"].as_i64().unwrap() as i32);
            let actual: Vec<_> = output.to_rgba().chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
            let expected: Vec<u8> = case["outputArgb"].as_array().unwrap().iter()
                .map(|x| u8::try_from(x.as_u64().unwrap()).unwrap()).collect();
            assert_eq!(actual, expected, "{}", case["id"]);
            assert_eq!(source.to_rgba(), rgba, "source must remain unchanged");
        }
    }

    #[test]
    fn mixed_straight_pixels_and_drawing_match_windows() {
        // Actual Framework GetPixel and ReadOnly LockBits agree on every fixture.
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/golden/bitmap-storage-windows.json"
        )).unwrap();
        let bytes = |v: &serde_json::Value| -> Vec<u8> {
            v.as_array().unwrap().iter().map(|x| u8::try_from(x.as_u64().unwrap()).unwrap()).collect()
        };
        let seed = |argb: &[u8], w: i32, h: i32| {
            let rgba: Vec<_> = argb.chunks_exact(4).flat_map(|p| [p[1], p[2], p[3], p[0]]).collect();
            Bitmap::from_rgba(w, h, &rgba)
        };
        for case in fixture["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let w = case["width"].as_i64().unwrap() as i32;
            let h = case["height"].as_i64().unwrap() as i32;
            let input = bytes(&case["inputArgb"]);
            let observations = &case["observations"];
            let assert_pixels = |label: &str, bitmap: &Bitmap| {
                let actual: Vec<_> = (0..bitmap.height()).flat_map(|y| (0..bitmap.width()).flat_map(move |x| {
                    let c = bitmap.get_pixel(x, y); [c.a, c.r, c.g, c.b]
                })).collect();
                let second: Vec<_> = bitmap.to_rgba().chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
                assert_eq!(actual, second, "{id}/{label}: readback APIs");
                assert_eq!(actual, bytes(&observations[label]), "{id}/{label}: Windows bytes");
            };
            let bitmap = seed(&input, w, h);
            if id == "storage-08-drawn-source" {
                // This fixture begins with captured grayscale output. The Picture
                // tests separately verify its construction from the original input.
                let a = seed(&bytes(&observations["a-before-draw"]), 3, 5);
                let mut copy = Bitmap::new(3, 5);
                for y in 0..5 { for x in 0..3 { copy.set_pixel(x, y, a.get_pixel(x, y)); } }
                assert_pixels("c-before-draw", &copy);
                let mut direct = Graphics::new(3, 5);
                direct.draw_image(&a, 0.0, 0.0, 3.0, 5.0);
                let mut reconstructed = Graphics::new(3, 5);
                reconstructed.draw_image(&copy, 0.0, 0.0, 3.0, 5.0);
                assert_pixels("drawn-a", direct.bitmap());
                assert_pixels("drawn-c", reconstructed.bitmap());
                assert_pixels("a-after-draw", &a);
                assert_pixels("c-after-draw", &copy);
            } else {
                assert_pixels("seed-before", &bitmap);
                assert_pixels("seed-before", &bitmap.clone());
                let mut g = Graphics::from_bitmap(bitmap);
                match id {
                    "storage-01-owner-only" => assert_pixels("after-owner", &g.into_bitmap()),
                    "storage-02-partial-opaque" | "storage-03-partial-alpha0" | "storage-04-partial-alpha1" => {
                        let c = match id {
                            "storage-02-partial-opaque" => Color::argb(255, 19, 101, 203),
                            "storage-03-partial-alpha0" => Color::argb(0, 19, 101, 203),
                            _ => Color::argb(1, 255, 255, 255),
                        };
                        g.fill_rectangle(c, 0.0, 1.0, 6.0, 1.0);
                        assert_pixels("after-fill", g.bitmap());
                    }
                    "storage-05-partial-image" => {
                        let source = seed(&input[..24], 6, 1);
                        g.draw_image(&source, 0.0, 1.0, 6.0, 1.0);
                        assert_pixels("after-image", g.bitmap());
                        assert_pixels("image-source-after", &source);
                    }
                    "storage-06-clear" => {
                        g.clear(Color::argb(254, 177, 177, 177));
                        assert_pixels("after-clear254", g.bitmap());
                        g.clear(Color::argb(0, 17, 43, 89));
                        assert_pixels("after-clear0", g.bitmap());
                    }
                    "storage-07-antialias" => {
                        g.anti_alias = true;
                        g.fill_rectangle(Color::argb(127, 31, 97, 211), 1.25, 0.875, 3.5, 0.25);
                        assert_pixels("after-antialias", g.bitmap());
                        let mut coverage = Graphics::new(6, 3);
                        coverage.anti_alias = true;
                        coverage.fill_rectangle(Color::WHITE, 1.25, 0.875, 3.5, 0.25);
                        assert_pixels("coverage-after", coverage.bitmap());
                    }
                    _ => panic!("unknown storage fixture {id}"),
                }
            }
        }
    }
}

#[cfg(test)]
mod nominal_coverage_regression {
    use super::*;
    #[test]
    fn opaque_identity_extents_are_bounded_before_integer_walks() {
        let mut g = Graphics::new(8, 8);
        g.draw_line(Color::RED, 1.0, f32::NAN, 0.0, 2.0, 2.0);
        g.draw_ellipse(Color::RED, 1.0, 0.0, 0.0, f32::INFINITY, 8.0);
        assert!((0..8).all(|y| (0..8).all(|x| g.bitmap.get_pixel(x,y) == Color::EMPTY)));
        g.draw_line(Color::RED, 1.0, -1_000_000.0, 4.0, 1_000_000.0, 4.0);
        assert!((0..8).all(|x| g.bitmap.get_pixel(x,4) == Color::RED));
        // The unsupported finite extent takes the bounded general stroker.
        g.draw_ellipse(Color::RED, 1.0, 0.0, 0.0, 1.0e30, 8.0);
        assert_eq!((g.width(), g.height()), (8,8));
    }
    #[test]
    fn native_fractional_line_preserves_hidden_argb_outside_coverage() {
        let hidden = Color::argb(0, 251, 117, 63);
        let paint = Color::argb(255, 19, 71, 233);
        let mut g = Graphics::new(12, 8);
        for y in 0..8 { for x in 0..12 { g.bitmap.set_pixel(x, y, hidden); } }
        g.draw_line(paint, 1.0, 2.25, 2.25, 9.75, 5.75);
        // Exact independent Windows detail-line-3-forward footprint.
        let ink = [(3,3),(4,3),(5,3),(6,4),(7,4),(8,5),(9,5)];
        for y in 0..8 { for x in 0..12 {
            assert_eq!(g.bitmap.get_pixel(x,y), if ink.contains(&(x,y)) { paint } else { hidden });
        } }
    }
}
