//! The text a colour and a font show (F35, F36).

use vixen_core::Color;
use vixen_effects::framework::FontDesc;

/// `ColorConverter`'s text, without the known-colour names Rust doesn't keep (`docs/audit/editor-pass.md`).
pub fn color_text(c: Color) -> String {
    if c.a == 255 {
        format!("{}, {}, {}", c.r, c.g, c.b)
    } else {
        format!("{}, {}, {}, {}", c.a, c.r, c.g, c.b)
    }
}

/// `FontConverter`'s text: `Arial, 8pt`, with the style after it when there is one.
pub fn font_display_text(f: &FontDesc) -> String {
    let style = match (f.bold, f.italic) {
        (false, false) => "",
        (true, false) => ", style=Bold",
        (false, true) => ", style=Italic",
        (true, true) => ", style=Bold, Italic",
    };
    format!("{}, {}pt{style}", f.name, f.size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_text_has_alpha_only_when_it_is_not_opaque() {
        assert_eq!(color_text(Color::rgb(255, 128, 0)), "255, 128, 0");
        assert_eq!(color_text(Color::argb(10, 255, 128, 0)), "10, 255, 128, 0");
    }

    #[test]
    fn font_text() {
        assert_eq!(font_display_text(&FontDesc::new("Arial", 8.0)), "Arial, 8pt");
        let f = FontDesc { name: "Arial".into(), size: 8.25, bold: true, italic: true };
        assert_eq!(font_display_text(&f), "Arial, 8.25pt, style=Bold, Italic");
    }
}
