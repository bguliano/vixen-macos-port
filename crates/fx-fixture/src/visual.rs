//! The timeline thumbnail helpers of `FixtureEffectBase` (`DrawText`, `DrawVisualRepresentation`,
//! `DrawVisualRepresentationWithBackground`) and `Curve.GenerateGenericCurveImage`.

use vixen_core::{Color, Curve};
use vixen_effects::framework::FontDesc;
use vixen_effects::gfx::{measure_string, Bitmap, Graphics};

/// `(x, y, width, height)` of the drawing area.
pub type Rect = (i32, i32, i32, i32);

pub const LIGHT_SKY_BLUE: Color = Color::rgb(135, 206, 250);
pub const HOT_PINK: Color = Color::rgb(255, 105, 180);

/// `SystemFonts.MessageBoxFont` on Windows. Core Text falls back to Arial where it is missing.
const MESSAGE_BOX_FONT: &str = "Segoe UI";
const MESSAGE_BOX_FONT_SIZE: f32 = 9.0;

/// `Vixen.Common.Graphics.GetAdjustedFont(graphics, text, rect, name, 24, MessageBoxFont)`.
fn adjusted_font(text: &str, rect: Rect) -> FontDesc {
    let original = FontDesc::new(MESSAGE_BOX_FONT, MESSAGE_BOX_FONT_SIZE);
    let (w, h) = measure_string(text, &original);
    let min_ratio = if w > 0.0 && h > 0.0 {
        f64::min(rect.3 as f64 / h as f64, (rect.2 + 15) as f64 / w as f64)
    } else {
        1.0
    };
    let mut size = (original.size as f64 * min_ratio) as f32 - 2.0;
    // minFontSize defaults to 10 and maxFontSize is 24
    if size < 10.0 {
        size = 10.0;
    } else if size > 24.0 {
        size = 24.0;
    }
    FontDesc::new(MESSAGE_BOX_FONT, size)
}

/// `FixtureEffectBase.DrawText`.
pub fn draw_text(g: &mut Graphics, rect: Rect, color: Color, text: &str, draw_string_y_offset: i32, y: i32, height: i32) {
    let text_rect = (rect.0, rect.1, rect.2, height);
    let font = adjusted_font(text, text_rect);
    g.draw_string(text, &font, color, rect.0 as f32, (y + draw_string_y_offset) as f32);
}

/// `Curve.GenerateGenericCurveImage(size, false, false, false, lineColor[, backgroundColors])`.
pub fn curve_image(curve: &Curve, width: i32, height: i32, line_color: Color, backgrounds: Option<&[Color]>) -> Bitmap {
    let mut g = Graphics::new(width, height);
    let (w, h) = (g.width(), g.height());
    match backgrounds {
        None => g.fill_rectangle(Color::BLACK, 0.0, 0.0, w as f32, h as f32),
        Some(colors) => {
            if !colors.is_empty() {
                let each = width / colors.len() as i32;
                let mut x = 0;
                for c in colors {
                    g.fill_rectangle(*c, x as f32, 0.0, each as f32, height as f32);
                    x += each;
                }
            }
        }
    }
    // TransformPoint: truncating ints, float scaling
    let transform = |p: (f64, f64)| -> (f32, f32) {
        let x = p.0 as i32;
        let y = ((p.1 as i32) - 100).abs();
        let ty = (y as f32 * (height as f32 / 100.0)) as i32;
        let tx = (x as f32 * ((width - 1) as f32 / 100.0)) as i32;
        (tx as f32, ty as f32)
    };
    for pair in curve.points.windows(2) {
        let (a, b) = (transform(pair[0]), transform(pair[1]));
        g.draw_line(line_color, 2.0, a.0, a.1, b.0, b.1);
    }
    g.into_bitmap()
}

/// `FixtureEffectBase.DrawVisualRepresentation`.
#[allow(clippy::too_many_arguments)]
pub fn draw_curve(
    g: &mut Graphics,
    rect: Rect,
    color: Color,
    text: &str,
    draw_curve_y_offset: i32,
    draw_string_y_offset: i32,
    curve: &Curve,
    y: i32,
) {
    if rect.2 > 0 && rect.3 > 0 {
        let image = curve_image(curve, rect.2, rect.3, color, None);
        g.draw_image(&image, rect.0 as f32, draw_curve_y_offset as f32, image.width() as f32, image.height() as f32);
    }
    draw_text(g, rect, color, text, draw_string_y_offset, y, rect.3 / 2);
}

/// `FixtureEffectBase.DrawVisualRepresentationWithBackground`.
#[allow(clippy::too_many_arguments)]
pub fn draw_curve_with_background(
    g: &mut Graphics,
    rect: Rect,
    color: Color,
    text: &str,
    draw_curve_y_offset: i32,
    draw_string_y_offset: i32,
    curve: &Curve,
    y: i32,
    backgrounds: &[Color],
) {
    if rect.2 > 0 && rect.3 > 0 {
        let image = curve_image(curve, rect.2, rect.3, color, Some(backgrounds));
        g.draw_image(&image, rect.0 as f32, draw_curve_y_offset as f32, image.width() as f32, image.height() as f32);
    }
    draw_text(g, rect, color, text, draw_string_y_offset, y, rect.3 / 2);
}

/// Test helper: whether any pixel of the canvas is not fully transparent.
#[cfg(test)]
pub fn has_ink(g: &Graphics) -> bool {
    let b = g.bitmap();
    (0..b.height()).any(|y| (0..b.width()).any(|x| b.get_pixel(x, y).a > 0))
}
