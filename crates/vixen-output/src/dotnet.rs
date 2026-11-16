//! The .NET behaviours the C# output code relies on: small-integer casts from floating point,
//! `System.Drawing.Color` as a data contract with its known-colour table, `ColorTranslator`, and the
//! number and text formats `DataContractSerializer` writes.

use vixen_core::Color;

/// `(byte)double`. The JIT lowers a floating point cast to a small integer as double -> int32 -> byte,
/// and .NET 9 and later saturate the first step. In range this is the same as `as u8`; out of range it
/// wraps like the C# does (`(byte)300.0` is 44, not 255).
#[inline]
pub fn byte_from_f64(d: f64) -> u8 {
    d as i32 as u8
}

/// `(ushort)double`, two steps for the same reason as [`byte_from_f64`].
#[inline]
pub fn ushort_from_f64(d: f64) -> u16 {
    d as i32 as u16
}

/// `(short)double`, two steps for the same reason as [`byte_from_f64`].
#[inline]
pub fn short_from_f64(d: f64) -> i16 {
    d as i32 as i16
}

const STATE_KNOWN_COLOR_VALID: i16 = 0x0001;
const STATE_ARGB_VALUE_VALID: i16 = 0x0002;
const STATE_NAME_VALID: i16 = 0x0008;

/// `System.Drawing.Color` exactly as `DataContractSerializer` stores it.
///
/// Rust equality is .NET `Color.Equals`: value, state, known colour and name must all agree, so
/// `Color.FromArgb(255, 255, 0, 0)` is not equal to `Color.Red`. `RGBColorBreakdownMixingFilter`
/// depends on that difference.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DotNetColor {
    pub known_color: i16,
    pub name: Option<String>,
    pub state: i16,
    pub value: i64,
}

impl DotNetColor {
    /// `Color.Empty`.
    pub const EMPTY: DotNetColor = DotNetColor { known_color: 0, name: None, state: 0, value: 0 };

    /// `Color.FromKnownColor`.
    pub const fn known(known_color: i16) -> DotNetColor {
        DotNetColor { known_color, name: None, state: STATE_KNOWN_COLOR_VALID, value: 0 }
    }

    pub const RED: DotNetColor = DotNetColor::known(known::RED);
    pub const LIME: DotNetColor = DotNetColor::known(known::LIME);
    pub const BLUE: DotNetColor = DotNetColor::known(known::BLUE);
    pub const WHITE: DotNetColor = DotNetColor::known(known::WHITE);
    pub const BLACK: DotNetColor = DotNetColor::known(known::BLACK);
    pub const YELLOW: DotNetColor = DotNetColor::known(known::YELLOW);

    /// `Color.FromArgb(a, r, g, b)`.
    pub fn from_argb(a: u8, r: u8, g: u8, b: u8) -> DotNetColor {
        let v = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        DotNetColor { known_color: 0, name: None, state: STATE_ARGB_VALUE_VALID, value: v as i64 }
    }

    /// `Color.FromArgb(r, g, b)`.
    pub fn from_rgb(r: u8, g: u8, b: u8) -> DotNetColor {
        DotNetColor::from_argb(255, r, g, b)
    }

    /// The colour of a `vixen_core::Color`, as a value colour rather than a known one.
    pub fn from_color(c: Color) -> DotNetColor {
        DotNetColor::from_argb(c.a, c.r, c.g, c.b)
    }

    /// `Color.IsKnownColor`.
    pub fn is_known_color(&self) -> bool {
        self.state & STATE_KNOWN_COLOR_VALID != 0
    }

    /// `Color.IsEmpty`: state is 0.
    pub fn is_empty(&self) -> bool {
        self.state == 0
    }

    /// `Color.ToArgb()`, with the bits as an unsigned value. System colours use the fixed table .NET
    /// falls back to where there is no Windows system palette.
    pub fn to_argb_u32(&self) -> u32 {
        if self.state & STATE_ARGB_VALUE_VALID != 0 {
            return self.value as u32;
        }
        if self.is_known_color() {
            return known::argb(self.known_color).unwrap_or(0);
        }
        0
    }

    /// The channel values, as `vixen_core::Color`.
    pub fn color(&self) -> Color {
        Color::from_argb_u32(self.to_argb_u32())
    }

    /// `ColorTranslator.ToHtml`.
    pub fn to_html(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        if self.is_known_color() {
            if self.known_color == known::LIGHT_GRAY {
                return "LightGrey".to_string();
            }
            if let Some(name) = known::name(self.known_color) {
                return name.to_string();
            }
        }
        if self.state & STATE_NAME_VALID != 0 {
            if let Some(name) = &self.name {
                return name.clone();
            }
        }
        let c = self.color();
        format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
    }

    /// `ColorTranslator.FromHtml`. `None` where the C# would throw.
    pub fn from_html(text: &str) -> Option<DotNetColor> {
        if text.is_empty() {
            return Some(DotNetColor::EMPTY);
        }
        let bytes = text.as_bytes();
        if bytes[0] == b'#' && (text.len() == 7 || text.len() == 4) {
            let hex = |s: &str| u8::from_str_radix(s, 16).ok();
            if text.len() == 7 {
                let r = hex(text.get(1..3)?)?;
                let g = hex(text.get(3..5)?)?;
                let b = hex(text.get(5..7)?)?;
                return Some(DotNetColor::from_rgb(r, g, b));
            }
            let doubled = |s: &str| hex(&format!("{s}{s}"));
            let r = doubled(text.get(1..2)?)?;
            let g = doubled(text.get(2..3)?)?;
            let b = doubled(text.get(3..4)?)?;
            return Some(DotNetColor::from_rgb(r, g, b));
        }
        if text.eq_ignore_ascii_case("LightGrey") {
            return Some(DotNetColor::known(known::LIGHT_GRAY));
        }
        known::by_name(text.trim()).map(DotNetColor::known)
    }
}

/// `System.Drawing.KnownColor` values and their colours.
pub mod known {
    pub const BLACK: i16 = 35;
    pub const BLUE: i16 = 37;
    pub const LIGHT_GRAY: i16 = 95;
    pub const LIME: i16 = 104;
    pub const RED: i16 = 141;
    pub const WHITE: i16 = 164;
    pub const YELLOW: i16 = 166;

    /// Indexed by `KnownColor - 1`. System colours hold .NET's fixed defaults.
    pub(super) const TABLE: [(&str, u32); 175] = [
        ("ActiveBorder", 0xFFD4D0C8),
        ("ActiveCaption", 0xFF0054E3),
        ("ActiveCaptionText", 0xFFFFFFFF),
        ("AppWorkspace", 0xFF808080),
        ("Control", 0xFFECE9D8),
        ("ControlDark", 0xFFACA899),
        ("ControlDarkDark", 0xFF716F64),
        ("ControlLight", 0xFFF1EFE2),
        ("ControlLightLight", 0xFFFFFFFF),
        ("ControlText", 0xFF000000),
        ("Desktop", 0xFF004E98),
        ("GrayText", 0xFFACA899),
        ("Highlight", 0xFF316AC5),
        ("HighlightText", 0xFFFFFFFF),
        ("HotTrack", 0xFF000080),
        ("InactiveBorder", 0xFFD4D0C8),
        ("InactiveCaption", 0xFF7A96DF),
        ("InactiveCaptionText", 0xFFD8E4F8),
        ("Info", 0xFFFFFFE1),
        ("InfoText", 0xFF000000),
        ("Menu", 0xFFFFFFFF),
        ("MenuText", 0xFF000000),
        ("ScrollBar", 0xFFD4D0C8),
        ("Window", 0xFFFFFFFF),
        ("WindowFrame", 0xFF000000),
        ("WindowText", 0xFF000000),
        ("Transparent", 0x00FFFFFF),
        ("AliceBlue", 0xFFF0F8FF),
        ("AntiqueWhite", 0xFFFAEBD7),
        ("Aqua", 0xFF00FFFF),
        ("Aquamarine", 0xFF7FFFD4),
        ("Azure", 0xFFF0FFFF),
        ("Beige", 0xFFF5F5DC),
        ("Bisque", 0xFFFFE4C4),
        ("Black", 0xFF000000),
        ("BlanchedAlmond", 0xFFFFEBCD),
        ("Blue", 0xFF0000FF),
        ("BlueViolet", 0xFF8A2BE2),
        ("Brown", 0xFFA52A2A),
        ("BurlyWood", 0xFFDEB887),
        ("CadetBlue", 0xFF5F9EA0),
        ("Chartreuse", 0xFF7FFF00),
        ("Chocolate", 0xFFD2691E),
        ("Coral", 0xFFFF7F50),
        ("CornflowerBlue", 0xFF6495ED),
        ("Cornsilk", 0xFFFFF8DC),
        ("Crimson", 0xFFDC143C),
        ("Cyan", 0xFF00FFFF),
        ("DarkBlue", 0xFF00008B),
        ("DarkCyan", 0xFF008B8B),
        ("DarkGoldenrod", 0xFFB8860B),
        ("DarkGray", 0xFFA9A9A9),
        ("DarkGreen", 0xFF006400),
        ("DarkKhaki", 0xFFBDB76B),
        ("DarkMagenta", 0xFF8B008B),
        ("DarkOliveGreen", 0xFF556B2F),
        ("DarkOrange", 0xFFFF8C00),
        ("DarkOrchid", 0xFF9932CC),
        ("DarkRed", 0xFF8B0000),
        ("DarkSalmon", 0xFFE9967A),
        ("DarkSeaGreen", 0xFF8FBC8B),
        ("DarkSlateBlue", 0xFF483D8B),
        ("DarkSlateGray", 0xFF2F4F4F),
        ("DarkTurquoise", 0xFF00CED1),
        ("DarkViolet", 0xFF9400D3),
        ("DeepPink", 0xFFFF1493),
        ("DeepSkyBlue", 0xFF00BFFF),
        ("DimGray", 0xFF696969),
        ("DodgerBlue", 0xFF1E90FF),
        ("Firebrick", 0xFFB22222),
        ("FloralWhite", 0xFFFFFAF0),
        ("ForestGreen", 0xFF228B22),
        ("Fuchsia", 0xFFFF00FF),
        ("Gainsboro", 0xFFDCDCDC),
        ("GhostWhite", 0xFFF8F8FF),
        ("Gold", 0xFFFFD700),
        ("Goldenrod", 0xFFDAA520),
        ("Gray", 0xFF808080),
        ("Green", 0xFF008000),
        ("GreenYellow", 0xFFADFF2F),
        ("Honeydew", 0xFFF0FFF0),
        ("HotPink", 0xFFFF69B4),
        ("IndianRed", 0xFFCD5C5C),
        ("Indigo", 0xFF4B0082),
        ("Ivory", 0xFFFFFFF0),
        ("Khaki", 0xFFF0E68C),
        ("Lavender", 0xFFE6E6FA),
        ("LavenderBlush", 0xFFFFF0F5),
        ("LawnGreen", 0xFF7CFC00),
        ("LemonChiffon", 0xFFFFFACD),
        ("LightBlue", 0xFFADD8E6),
        ("LightCoral", 0xFFF08080),
        ("LightCyan", 0xFFE0FFFF),
        ("LightGoldenrodYellow", 0xFFFAFAD2),
        ("LightGray", 0xFFD3D3D3),
        ("LightGreen", 0xFF90EE90),
        ("LightPink", 0xFFFFB6C1),
        ("LightSalmon", 0xFFFFA07A),
        ("LightSeaGreen", 0xFF20B2AA),
        ("LightSkyBlue", 0xFF87CEFA),
        ("LightSlateGray", 0xFF778899),
        ("LightSteelBlue", 0xFFB0C4DE),
        ("LightYellow", 0xFFFFFFE0),
        ("Lime", 0xFF00FF00),
        ("LimeGreen", 0xFF32CD32),
        ("Linen", 0xFFFAF0E6),
        ("Magenta", 0xFFFF00FF),
        ("Maroon", 0xFF800000),
        ("MediumAquamarine", 0xFF66CDAA),
        ("MediumBlue", 0xFF0000CD),
        ("MediumOrchid", 0xFFBA55D3),
        ("MediumPurple", 0xFF9370DB),
        ("MediumSeaGreen", 0xFF3CB371),
        ("MediumSlateBlue", 0xFF7B68EE),
        ("MediumSpringGreen", 0xFF00FA9A),
        ("MediumTurquoise", 0xFF48D1CC),
        ("MediumVioletRed", 0xFFC71585),
        ("MidnightBlue", 0xFF191970),
        ("MintCream", 0xFFF5FFFA),
        ("MistyRose", 0xFFFFE4E1),
        ("Moccasin", 0xFFFFE4B5),
        ("NavajoWhite", 0xFFFFDEAD),
        ("Navy", 0xFF000080),
        ("OldLace", 0xFFFDF5E6),
        ("Olive", 0xFF808000),
        ("OliveDrab", 0xFF6B8E23),
        ("Orange", 0xFFFFA500),
        ("OrangeRed", 0xFFFF4500),
        ("Orchid", 0xFFDA70D6),
        ("PaleGoldenrod", 0xFFEEE8AA),
        ("PaleGreen", 0xFF98FB98),
        ("PaleTurquoise", 0xFFAFEEEE),
        ("PaleVioletRed", 0xFFDB7093),
        ("PapayaWhip", 0xFFFFEFD5),
        ("PeachPuff", 0xFFFFDAB9),
        ("Peru", 0xFFCD853F),
        ("Pink", 0xFFFFC0CB),
        ("Plum", 0xFFDDA0DD),
        ("PowderBlue", 0xFFB0E0E6),
        ("Purple", 0xFF800080),
        ("Red", 0xFFFF0000),
        ("RosyBrown", 0xFFBC8F8F),
        ("RoyalBlue", 0xFF4169E1),
        ("SaddleBrown", 0xFF8B4513),
        ("Salmon", 0xFFFA8072),
        ("SandyBrown", 0xFFF4A460),
        ("SeaGreen", 0xFF2E8B57),
        ("SeaShell", 0xFFFFF5EE),
        ("Sienna", 0xFFA0522D),
        ("Silver", 0xFFC0C0C0),
        ("SkyBlue", 0xFF87CEEB),
        ("SlateBlue", 0xFF6A5ACD),
        ("SlateGray", 0xFF708090),
        ("Snow", 0xFFFFFAFA),
        ("SpringGreen", 0xFF00FF7F),
        ("SteelBlue", 0xFF4682B4),
        ("Tan", 0xFFD2B48C),
        ("Teal", 0xFF008080),
        ("Thistle", 0xFFD8BFD8),
        ("Tomato", 0xFFFF6347),
        ("Turquoise", 0xFF40E0D0),
        ("Violet", 0xFFEE82EE),
        ("Wheat", 0xFFF5DEB3),
        ("White", 0xFFFFFFFF),
        ("WhiteSmoke", 0xFFF5F5F5),
        ("Yellow", 0xFFFFFF00),
        ("YellowGreen", 0xFF9ACD32),
        ("ButtonFace", 0xFFECE9D8),
        ("ButtonHighlight", 0xFFFFFFFF),
        ("ButtonShadow", 0xFFACA899),
        ("GradientActiveCaption", 0xFF3D95FF),
        ("GradientInactiveCaption", 0xFF9DB9EB),
        ("MenuBar", 0xFFECE9D8),
        ("MenuHighlight", 0xFF316AC5),
        ("RebeccaPurple", 0xFF663399),
    ];

    /// `KnownColorTable.KnownColorToArgb`.
    pub fn argb(known_color: i16) -> Option<u32> {
        let index = (known_color as i32 - 1).try_into().ok()?;
        TABLE.get::<usize>(index).map(|e| e.1)
    }

    pub fn name(known_color: i16) -> Option<&'static str> {
        let index = (known_color as i32 - 1).try_into().ok()?;
        TABLE.get::<usize>(index).map(|e| e.0)
    }

    /// The case-insensitive name lookup `ColorConverter` does.
    pub fn by_name(name: &str) -> Option<i16> {
        TABLE.iter().position(|e| e.0.eq_ignore_ascii_case(name)).map(|i| (i + 1) as i16)
    }
}

/// `XmlConverter.ToString(double)`, which is `ToString("R")`: the shortest text that round-trips,
/// switching to scientific notation at 1E-05 and below, and from 1E+15.
pub fn format_double(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v == f64::INFINITY {
        return "INF".to_string();
    }
    if v == f64::NEG_INFINITY {
        return "-INF".to_string();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0".to_string() } else { "0".to_string() };
    }
    let sci = format!("{v:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if !(-4..15).contains(&exp) {
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{mantissa}E{sign}{:02}", exp.abs());
    }
    format!("{v}")
}

/// `Encoding.ASCII.GetBytes`: anything outside ASCII becomes `?`.
pub fn ascii_bytes(text: &str) -> Vec<u8> {
    text.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).collect()
}

/// `Guid.ToByteArray()`: the first three groups are little-endian.
pub fn guid_bytes(id: &uuid::Uuid) -> [u8; 16] {
    id.to_bytes_le()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casts_match_the_jit_two_step_lowering() {
        assert_eq!(byte_from_f64(127.5), 127);
        assert_eq!(byte_from_f64(255.0), 255);
        assert_eq!(byte_from_f64(300.0), 44);
        assert_eq!(byte_from_f64(-1.0), 255);
        assert_eq!(byte_from_f64(f64::NAN), 0);
        assert_eq!(ushort_from_f64(65535.0), 65535);
        assert_eq!(short_from_f64(40000.0), -25536);
    }

    #[test]
    fn known_colour_table_lines_up_with_the_values_in_real_files() {
        // These numbers are what Vixen wrote into a real ModuleStore.xml.
        assert_eq!(known::by_name("Red"), Some(141));
        assert_eq!(known::by_name("lime"), Some(104));
        assert_eq!(known::by_name("Blue"), Some(37));
        assert_eq!(known::by_name("White"), Some(164));
        assert_eq!(DotNetColor::known(141).to_argb_u32(), 0xFFFF_0000);
        assert_eq!(DotNetColor::known(104).to_argb_u32(), 0xFF00_FF00);
        assert_eq!(DotNetColor::known(37).to_argb_u32(), 0xFF00_00FF);
        assert_eq!(DotNetColor::known(164).to_argb_u32(), 0xFFFF_FFFF);
        assert_eq!(DotNetColor::known(79).color(), Color::GREEN);
        assert_eq!(DotNetColor::known(27).to_argb_u32(), 0x00FF_FFFF);
    }

    #[test]
    fn color_equality_includes_state() {
        assert_ne!(DotNetColor::RED, DotNetColor::from_rgb(255, 0, 0));
        assert_eq!(DotNetColor::RED.to_argb_u32(), DotNetColor::from_rgb(255, 0, 0).to_argb_u32());
        assert_eq!(DotNetColor::from_rgb(255, 0, 0).value, 4_294_901_760);
    }

    #[test]
    fn color_translator_round_trips() {
        assert_eq!(DotNetColor::from_html("#FF0000"), Some(DotNetColor::from_rgb(255, 0, 0)));
        assert_eq!(DotNetColor::from_html("#F00"), Some(DotNetColor::from_rgb(255, 0, 0)));
        assert_eq!(DotNetColor::from_html("Red"), Some(DotNetColor::RED));
        assert_eq!(DotNetColor::from_html("red"), Some(DotNetColor::RED));
        assert_eq!(DotNetColor::from_html("LightGrey"), Some(DotNetColor::known(known::LIGHT_GRAY)));
        assert_eq!(DotNetColor::from_html(""), Some(DotNetColor::EMPTY));
        assert_eq!(DotNetColor::from_html("not a colour"), None);
        assert_eq!(DotNetColor::RED.to_html(), "Red");
        assert_eq!(DotNetColor::from_rgb(1, 2, 3).to_html(), "#010203");
        assert_eq!(DotNetColor::EMPTY.to_html(), "");
    }

    #[test]
    fn doubles_format_like_xml_converter() {
        assert_eq!(format_double(0.0), "0");
        assert_eq!(format_double(100.0), "100");
        assert_eq!(format_double(0.5), "0.5");
        assert_eq!(format_double(33.333333333333336), "33.333333333333336");
        assert_eq!(format_double(0.0001), "0.0001");
        assert_eq!(format_double(0.00001), "1E-05");
        assert_eq!(format_double(1e15), "1E+15");
        assert_eq!(format_double(123456789012345.0), "123456789012345");
        assert_eq!(format_double(1.5e300), "1.5E+300");
    }

    #[test]
    fn ascii_encoding_replaces_everything_else() {
        assert_eq!(ascii_bytes("A~\u{e9}\u{1F600}"), vec![b'A', b'~', b'?', b'?']);
    }

    #[test]
    fn guid_bytes_are_mixed_endian() {
        let id = uuid::uuid!("00112233-4455-6677-8899-aabbccddeeff");
        assert_eq!(
            guid_bytes(&id),
            [0x33, 0x22, 0x11, 0x00, 0x55, 0x44, 0x77, 0x66, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]
        );
    }
}
