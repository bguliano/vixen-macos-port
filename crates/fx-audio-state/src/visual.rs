//! Helpers for the custom timeline thumbnails of LipSync and State.

use vixen_effects::gfx::measure_string;
use vixen_effects::prelude::FontDesc;

/// The font Vixen embeds for these thumbnails. If it is not installed the canvas falls back to Arial, and
/// measuring uses the same fallback, so the fitting stays consistent.
pub const FONT_NAME: &str = "DigitalDream";

/// `Vixen.Common.Graphics.GetAdjustedFont(g, text, clipRectangle, "Vixen.Fonts.DigitalDream.ttf", 48)`:
/// scales a 100 pt font so the text fits the rectangle (plus 15 px of slack in width), then clamps the size
/// to 10..48.
pub fn adjusted_font(text: &str, clip_w: i32, clip_h: i32, max_font_size: i32) -> FontDesc {
    let min_font_size = 10.0_f32;
    let original = FontDesc::new(FONT_NAME, 100.0);
    let (w, h) = measure_string(text, &original);
    let min_ratio = (clip_h as f64 / h as f64).min((clip_w as f64 + 15.0) / w as f64);
    let mut size = (original.size as f64 * min_ratio) as f32 - 2.0;
    if size.is_nan() || size < min_font_size {
        size = min_font_size;
    } else if size > max_font_size as f32 {
        size = max_font_size as f32;
    }
    FontDesc::new(FONT_NAME, size)
}
