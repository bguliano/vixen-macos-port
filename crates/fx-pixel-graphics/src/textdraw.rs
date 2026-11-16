//! System.Drawing text helpers shared by Text and CountDown: gradient brushes (`LinearGradientBrush` with
//! `InterpolationColors`), a canvas that mimics drawing with `TextRenderingHint.SingleBitPerPixelGridFit`
//! (no anti-aliasing), and sub-pixel text measurement for `MeasureCharacterRanges`.

use vixen_core::{Color, ColorGradient};
use vixen_effects::framework::FontDesc;
use vixen_effects::gfx::{measure_string, Graphics};

/// `LinearGradientMode`.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum GradMode {
    Horizontal,
    Vertical,
    ForwardDiagonal,
    BackwardDiagonal,
}

/// A `LinearGradientBrush` over a rectangle with a colour blend. GDI+ tiles the gradient outside the rectangle.
pub struct Brush {
    pub rect: (i32, i32, i32, i32),
    pub mode: GradMode,
    pub gradient: ColorGradient,
}

impl Brush {
    /// `None` for an empty rectangle, where GDI+ throws.
    pub fn new(rect: (i32, i32, i32, i32), mode: GradMode, gradient: ColorGradient) -> Option<Brush> {
        if rect.2 <= 0 || rect.3 <= 0 {
            return None;
        }
        Some(Brush { rect, mode, gradient })
    }

    pub fn color_at(&self, x: f32, y: f32) -> Color {
        let u = (x - self.rect.0 as f32) / self.rect.2 as f32;
        let v = (y - self.rect.1 as f32) / self.rect.3 as f32;
        let mut t = match self.mode {
            GradMode::Horizontal => u,
            GradMode::Vertical => v,
            GradMode::ForwardDiagonal => (u + v) / 2.0,
            GradMode::BackwardDiagonal => ((1.0 - u) + v) / 2.0,
        };
        if !(0.0..=1.0).contains(&t) {
            t -= t.floor();
        }
        self.gradient.get_color_at_f32(t)
    }
}

/// Where a drawn pixel was last painted. A pixel whose alpha is 0 is untouched (`Color.FromArgb(0,0,0,0)`).
pub struct Canvas {
    pub w: i32,
    pub h: i32,
    px: Vec<Color>,
    /// `TranslateTransform(c) RotateTransform(angle) TranslateTransform(-c)`: (degrees, centre x, centre y).
    rotation: Option<(f32, f32, f32)>,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Canvas {
        let (w, h) = (w.max(0), h.max(0));
        Canvas { w, h, px: vec![Color::EMPTY; (w as usize) * (h as usize)], rotation: None }
    }

    pub fn set_rotation(&mut self, degrees: f32, cx: f32, cy: f32) {
        self.rotation = Some((degrees, cx, cy));
    }

    pub fn get(&self, x: i32, y: i32) -> Color {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return Color::EMPTY;
        }
        self.px[(y * self.w + x) as usize]
    }

    /// Draws a shape described by `draw` (white, no anti-aliasing) and paints covered pixels from `brush`.
    pub fn fill_shape(&mut self, brush: &Brush, draw: impl FnOnce(&mut Graphics)) {
        if self.w == 0 || self.h == 0 {
            return;
        }
        let mut g = Graphics::new(self.w, self.h);
        if let Some((deg, cx, cy)) = self.rotation {
            g.translate(cx, cy);
            g.rotate(deg);
            g.translate(-cx, -cy);
        }
        draw(&mut g);
        let bmp = g.bitmap();
        let inverse = self.rotation.map(|(deg, cx, cy)| {
            let r = (-deg).to_radians();
            (r.cos(), r.sin(), cx, cy)
        });
        for y in 0..self.h {
            for x in 0..self.w {
                if bmp.get_pixel(x, y).a < 128 {
                    continue;
                }
                let (mut sx, mut sy) = (x as f32 + 0.5, y as f32 + 0.5);
                if let Some((c, s, cx, cy)) = inverse {
                    let (dx, dy) = (sx - cx, sy - cy);
                    sx = cx + dx * c - dy * s;
                    sy = cy + dx * s + dy * c;
                }
                let color = brush.color_at(sx, sy);
                // A brush colour with alpha 0 paints nothing.
                if color.a > 0 {
                    self.px[(y * self.w + x) as usize] = color;
                }
            }
        }
    }

    /// `Graphics.DrawString(text, font, brush, x, y)`.
    pub fn draw_string(&mut self, text: &str, font: &FontDesc, brush: &Brush, x: i32, y: i32) {
        if text.is_empty() {
            return;
        }
        self.fill_shape(brush, |g| g.draw_string(text, font, Color::WHITE, x as f32, y as f32));
    }
}

/// `Graphics.MeasureString` size, in pixels.
pub fn measure(text: &str, font: &FontDesc) -> (f32, f32) {
    measure_string(text, font)
}

/// A measurement without the whole-pixel rounding, for the positions of single characters.
pub fn measure_precise(text: &str, font: &FontDesc) -> (f32, f32) {
    const K: f32 = 16.0;
    let mut big = font.clone();
    big.size = font.size * K;
    let (w, h) = measure_string(text, &big);
    (w / K, h / K)
}

/// `MeasureCharacterRanges` for one character per range: the left edge and advance of each character, relative
/// to the start of the string (the C# subtracts the leading offset GDI+ adds).
pub fn char_ranges(word: &str, font: &FontDesc) -> Vec<(f32, f32)> {
    let chars: Vec<char> = word.chars().collect();
    let mut out = Vec::with_capacity(chars.len());
    let mut prefix = String::new();
    let mut prev = 0.0f32;
    for c in &chars {
        prefix.push(*c);
        let w = measure_precise(&prefix, font).0;
        out.push((prev, (w - prev).max(0.0)));
        prev = w;
    }
    out
}

/// The line height of `font` in pixels.
pub fn line_height(font: &FontDesc) -> f32 {
    measure_precise("A", font).1
}
