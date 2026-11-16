//! Codecs for the value types effects share: colours, curves, gradients, fonts.
//!
//! Every `*_from_xml` reads a member element's content and returns `None` when the content is not what the
//! type's contract says, so a caller can keep the XML raw instead. Every `*_to_xml` builds a member element
//! with the given name and namespace.

use crate::dcs::{child_bool, child_f64, child_string, format_bool, format_double, opt_string_element, parse_double};
use vixen_core::color::{Rgb, Xyz};
use vixen_core::{AlphaPoint, Color, ColorGradient, ColorPoint, Curve, GradientLevelPair};
use vixen_effects::framework::FontDesc;
use vixen_xml::{XElement, XSD_NS};

pub const NS_DRAWING: &str = "http://schemas.datacontract.org/2004/07/System.Drawing";
pub const NS_COLOR_MODELS: &str = "http://schemas.datacontract.org/2004/07/Common.Controls.ColorManagement.ColorModels";
pub const NS_GRADIENTS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.ColorGradients";
pub const NS_CURVES: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.Curves";
pub const NS_ZEDGRAPH: &str = "http://schemas.datacontract.org/2004/07/ZedGraph";
pub const NS_VALUE_TYPES: &str = "http://schemas.datacontract.org/2004/07/Common.ValueTypes";

// ---- System.Drawing.Color -------------------------------------------------------------------------------

/// `Color.StateKnownColorValid`.
const STATE_KNOWN: i32 = 1;
/// `Color.StateARGBValueValid`.
const STATE_ARGB: i32 = 2;

/// System.Drawing.Color's serialized fields. `vixen_core::Color` is only the four channels, so this is what
/// remembers that `Color.White` and `Color.FromArgb(255, 255, 255)` are written differently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DotNetColor {
    pub known_color: i32,
    pub name: Option<String>,
    pub state: i32,
    pub value: i64,
}

impl DotNetColor {
    pub fn from_xml(e: &XElement) -> Option<DotNetColor> {
        let known_color = e.child("knownColor")?.text().trim().parse().ok()?;
        let name = child_string(e, "name");
        let state = e.child("state")?.text().trim().parse().ok()?;
        let value = e.child("value")?.text().trim().parse().ok()?;
        Some(DotNetColor { known_color, name, state, value })
    }

    pub fn to_xml(&self, ns: &str, local: &str) -> XElement {
        let mut e = XElement::new(ns, local);
        e.push(XElement::with_text(NS_DRAWING, "knownColor", self.known_color.to_string()));
        e.push(opt_string_element(NS_DRAWING, "name", self.name.as_deref()));
        e.push(XElement::with_text(NS_DRAWING, "state", self.state.to_string()));
        e.push(XElement::with_text(NS_DRAWING, "value", self.value.to_string()));
        e
    }

    /// `Color.Value`: the stored ARGB, else the known colour's, else 0 for a name that is neither.
    pub fn to_color(&self) -> Color {
        if self.state & STATE_ARGB != 0 {
            return Color::from_argb_u32(self.value as u32);
        }
        if self.state & STATE_KNOWN != 0 {
            if let Some(argb) = known_color_argb(self.known_color) {
                return Color::from_argb_u32(argb);
            }
        }
        Color::EMPTY
    }

    /// What `Color.FromArgb` stores, with `Color.Empty` and `Color.Transparent` written as C# writes those.
    /// Every other colour loses any known-colour name: .NET reads the result as the same four channels.
    pub fn from_color(c: Color) -> DotNetColor {
        if c == Color::EMPTY {
            return DotNetColor { known_color: 0, name: None, state: 0, value: 0 };
        }
        if c == Color::TRANSPARENT {
            return DotNetColor { known_color: KNOWN_TRANSPARENT, name: None, state: STATE_KNOWN, value: 0 };
        }
        DotNetColor { known_color: 0, name: None, state: STATE_ARGB, value: c.to_argb_u32() as i64 }
    }

    /// `Color.FromKnownColor`.
    pub fn from_known(known_color: i32) -> DotNetColor {
        DotNetColor { known_color, name: None, state: STATE_KNOWN, value: 0 }
    }
}

const KNOWN_TRANSPARENT: i32 = 27;

pub use vixen_core::known_color::{known_color_argb, known_color_name, known_color_named};

/// Reads a System.Drawing.Color member.
pub fn color_from_xml(e: &XElement) -> Option<Color> {
    DotNetColor::from_xml(e).map(|c| c.to_color())
}

/// Writes a colour the way `Color.FromArgb` would.
pub fn color_to_xml(ns: &str, local: &str, c: Color) -> XElement {
    DotNetColor::from_color(c).to_xml(ns, local)
}

/// Reads a `List<Color>`.
pub fn color_list_from_xml(e: &XElement) -> Option<Vec<Color>> {
    e.elements().map(color_from_xml).collect()
}

pub fn color_list_to_xml(ns: &str, local: &str, colors: &[Color]) -> XElement {
    let mut e = XElement::new(ns, local);
    for c in colors {
        e.push(color_to_xml(NS_DRAWING, "Color", *c));
    }
    e
}

// ---- ColorModels ----------------------------------------------------------------------------------------

/// `ColorModels.RGB`, stored as `_b`, `_g`, `_r`. The fields are read directly, with no clipping, because
/// DataContractSerializer writes fields without going through the C# constructor.
pub fn rgb_from_xml(e: &XElement) -> Option<Rgb> {
    Some(Rgb { r: child_f64(e, "_r")?, g: child_f64(e, "_g")?, b: child_f64(e, "_b")? })
}

pub fn rgb_to_xml(ns: &str, local: &str, c: Rgb) -> XElement {
    let mut e = XElement::new(ns, local);
    e.push(XElement::with_text(NS_COLOR_MODELS, "_b", format_double(c.b)));
    e.push(XElement::with_text(NS_COLOR_MODELS, "_g", format_double(c.g)));
    e.push(XElement::with_text(NS_COLOR_MODELS, "_r", format_double(c.r)));
    e
}

/// `ColorModels.XYZ`, stored as `_x`, `_y`, `_z`.
pub fn xyz_from_xml(e: &XElement) -> Option<Xyz> {
    Some(Xyz { x: child_f64(e, "_x")?, y: child_f64(e, "_y")?, z: child_f64(e, "_z")? })
}

pub fn xyz_to_xml(ns: &str, local: &str, c: Xyz) -> XElement {
    let mut e = XElement::new(ns, local);
    e.push(XElement::with_text(NS_COLOR_MODELS, "_x", format_double(c.x)));
    e.push(XElement::with_text(NS_COLOR_MODELS, "_y", format_double(c.y)));
    e.push(XElement::with_text(NS_COLOR_MODELS, "_z", format_double(c.z)));
    e
}

// ---- VixenModules.App.Curves.Curve ----------------------------------------------------------------------

/// Reads a `Curve`: `IsCurrentLibraryCurve`, `Points` and `_libraryReferenceName`. Each point is a ZedGraph
/// `PointPair`, which is `ISerializable` and so lists its values with their own `i:type`. A nil name reads as
/// empty, as `Curve.LibraryReferenceName` does.
pub fn curve_from_xml(e: &XElement) -> Option<Curve> {
    let points = e.child("Points")?;
    let mut list = Vec::new();
    for p in points.elements() {
        list.push((child_f64(p, "X")?, child_f64(p, "Y")?));
    }
    let mut curve = Curve::from_points(list);
    curve.is_current_library_curve = child_bool(e, "IsCurrentLibraryCurve").unwrap_or(false);
    curve.library_reference_name = child_string(e, "_libraryReferenceName").unwrap_or_default();
    Some(curve)
}

pub fn curve_to_xml(ns: &str, local: &str, curve: &Curve) -> XElement {
    let mut e = XElement::new(ns, local);
    e.push(XElement::with_text(NS_CURVES, "IsCurrentLibraryCurve", format_bool(curve.is_current_library_curve)));
    let mut points = XElement::new(NS_CURVES, "Points");
    for (x, y) in &curve.points {
        points.push(point_pair_to_xml(*x, *y));
    }
    e.push(points);
    e.push(XElement::with_text(NS_CURVES, "_libraryReferenceName", curve.library_reference_name.as_str()));
    e
}

fn point_pair_to_xml(x: f64, y: f64) -> XElement {
    let typed = |local: &str, xsd: &str, text: String| {
        let mut v = XElement::with_text("", local, text);
        v.xsi_type = Some((XSD_NS.to_string(), xsd.to_string()));
        v
    };
    let mut p = XElement::new(NS_ZEDGRAPH, "PointPair");
    p.push(typed("schema", "int", "11".to_string()));
    p.push(typed("X", "double", format_double(x)));
    p.push(typed("Y", "double", format_double(y)));
    p.push(typed("schema2", "int", "11".to_string()));
    p.push(typed("Z", "double", "0".to_string()));
    p.push(XElement::nil("", "Tag"));
    p
}

// ---- VixenModules.App.ColorGradients.ColorGradient --------------------------------------------------------

/// Reads a `ColorGradient`.
pub fn gradient_from_xml(e: &XElement) -> Option<ColorGradient> {
    let mut colors = Vec::new();
    for p in e.child("_colors")?.elements() {
        let color = xyz_from_xml(p.child("_color")?)?;
        colors.push(ColorPoint::from_xyz(color, child_f64(p, "_focus")?, child_f64(p, "_position")?));
    }
    let mut alphas = Vec::new();
    for p in e.child("_alphas")?.elements() {
        alphas.push(AlphaPoint {
            alpha: child_f64(p, "_alpha")?,
            position: child_f64(p, "_position")?,
            focus: child_f64(p, "_focus")?,
        });
    }
    let mut g = ColorGradient::from_parts(colors, alphas);
    g.gammacorrected = child_bool(e, "_gammacorrected").unwrap_or(false);
    g.title = child_string(e, "_title");
    g.library_reference_name = child_string(e, "_libraryReferenceName").unwrap_or_default();
    g.is_current_library_gradient = child_bool(e, "IsCurrentLibraryGradient").unwrap_or(false);
    Some(g)
}

pub fn gradient_to_xml(ns: &str, local: &str, g: &ColorGradient) -> XElement {
    let mut e = XElement::new(ns, local);
    e.push(XElement::with_text(NS_GRADIENTS, "IsCurrentLibraryGradient", format_bool(g.is_current_library_gradient)));
    let mut alphas = XElement::new(NS_GRADIENTS, "_alphas");
    for p in g.alphas() {
        let mut a = XElement::new(NS_GRADIENTS, "AlphaPoint");
        a.push(XElement::with_text(NS_GRADIENTS, "_focus", format_double(p.focus)));
        a.push(XElement::with_text(NS_GRADIENTS, "_position", format_double(p.position)));
        a.push(XElement::with_text(NS_GRADIENTS, "_alpha", format_double(p.alpha)));
        alphas.push(a);
    }
    e.push(alphas);
    let mut colors = XElement::new(NS_GRADIENTS, "_colors");
    for p in g.colors() {
        let mut c = XElement::new(NS_GRADIENTS, "ColorPoint");
        c.push(XElement::with_text(NS_GRADIENTS, "_focus", format_double(p.focus)));
        c.push(XElement::with_text(NS_GRADIENTS, "_position", format_double(p.position)));
        c.push(xyz_to_xml(NS_GRADIENTS, "_color", p.color));
        colors.push(c);
    }
    e.push(colors);
    e.push(XElement::with_text(NS_GRADIENTS, "_gammacorrected", format_bool(g.gammacorrected)));
    e.push(XElement::with_text(NS_GRADIENTS, "_libraryReferenceName", g.library_reference_name.as_str()));
    e.push(opt_string_element(NS_GRADIENTS, "_title", g.title.as_deref()));
    e
}

/// Reads a `List<ColorGradient>`.
pub fn gradient_list_from_xml(e: &XElement) -> Option<Vec<ColorGradient>> {
    e.elements().map(gradient_from_xml).collect()
}

pub fn gradient_list_to_xml(ns: &str, local: &str, list: &[ColorGradient]) -> XElement {
    let mut e = XElement::new(ns, local);
    for g in list {
        e.push(gradient_to_xml(NS_GRADIENTS, "ColorGradient", g));
    }
    e
}

/// Reads a `GradientLevelPair`.
pub fn level_pair_from_xml(e: &XElement) -> Option<GradientLevelPair> {
    Some(GradientLevelPair {
        color_gradient: gradient_from_xml(e.child("ColorGradient")?)?,
        curve: curve_from_xml(e.child("Curve")?)?,
    })
}

pub fn level_pair_to_xml(ns: &str, local: &str, pair: &GradientLevelPair) -> XElement {
    let mut e = XElement::new(ns, local);
    e.push(gradient_to_xml(NS_GRADIENTS, "ColorGradient", &pair.color_gradient));
    e.push(curve_to_xml(NS_GRADIENTS, "Curve", &pair.curve));
    e
}

/// Reads a `List<GradientLevelPair>`.
pub fn level_pair_list_from_xml(e: &XElement) -> Option<Vec<GradientLevelPair>> {
    e.elements().map(level_pair_from_xml).collect()
}

pub fn level_pair_list_to_xml(ns: &str, local: &str, list: &[GradientLevelPair]) -> XElement {
    let mut e = XElement::new(ns, local);
    for p in list {
        e.push(level_pair_to_xml(NS_GRADIENTS, "GradientLevelPair", p));
    }
    e
}

// ---- Common.ValueTypes.Percentage ------------------------------------------------------------------------

/// `Percentage` holds a single `float` named `Value`. C#'s implicit conversion to double keeps the float's
/// exact value, so `0.2` reads as `0.20000000298`, and writing narrows back to the same float.
pub fn percentage_from_xml(e: &XElement) -> Option<f64> {
    let v = parse_double(&e.child("Value")?.text())?;
    Some((v as f32) as f64)
}

pub fn percentage_to_xml(ns: &str, local: &str, v: f64) -> XElement {
    let mut e = XElement::new(ns, local);
    e.push(XElement::with_text(NS_VALUE_TYPES, "Value", crate::dcs::format_single(v as f32)));
    e
}

// ---- SerializableFont -------------------------------------------------------------------------------------

/// The member `SerializableFont` stores `FontConverter`'s string under.
const FONT_MEMBER: &str = "SerializeFontAttribute";

/// Parses `FontConverter.ConvertToString`: `Arial, 8pt` or `Arial, 8.25pt, style=Bold, Italic`. A culture
/// with a comma decimal separator uses `;` between parts. Underline and strikeout have no place in
/// `FontDesc`, and the generic codec's fidelity data keeps the exact text when a font uses them.
pub fn font_from_text(text: &str) -> Option<FontDesc> {
    let separator = if text.contains(';') { ';' } else { ',' };
    let mut parts = text.split(separator).map(str::trim);
    let name = parts.next().filter(|n| !n.is_empty())?;
    let size = parts.next()?;
    let size = size.strip_suffix("pt").unwrap_or(size);
    let size: f32 = size.trim().replace(',', ".").parse().ok()?;
    let mut font = FontDesc::new(name, size);
    for part in parts {
        let style = part.strip_prefix("style=").unwrap_or(part).trim();
        match style {
            "Bold" => font.bold = true,
            "Italic" => font.italic = true,
            _ => {}
        }
    }
    Some(font)
}

pub fn font_to_text(font: &FontDesc) -> String {
    let mut text = format!("{}, {}pt", font.name, font.size);
    match (font.bold, font.italic) {
        (true, true) => text.push_str(", style=Bold, Italic"),
        (true, false) => text.push_str(", style=Bold"),
        (false, true) => text.push_str(", style=Italic"),
        (false, false) => {}
    }
    text
}

/// Reads a `SerializableFont` member.
pub fn font_from_xml(e: &XElement) -> Option<FontDesc> {
    let c = e.child(FONT_MEMBER)?;
    if c.nil {
        return None;
    }
    font_from_text(c.text().trim())
}

/// Writes a `SerializableFont` member. The class lives in the effect's own namespace.
pub fn font_to_xml(ns: &str, local: &str, font: &FontDesc) -> XElement {
    let mut e = XElement::new(ns, local);
    e.push(XElement::with_text(ns, FONT_MEMBER, font_to_text(font)));
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::same_tree;

    const SPEC_NS: &str = "http://schemas.datacontract.org/2004/07/Test";

    fn parse(text: &str) -> XElement {
        XElement::parse(text).unwrap()
    }

    fn color_xml(known: i32, state: i32, value: i64) -> XElement {
        DotNetColor { known_color: known, name: None, state, value }.to_xml(SPEC_NS, "Color")
    }

    #[test]
    fn known_colour_numbers_match_the_real_files() {
        // Lime and White appear as mark collection colours in the user's sequences.
        assert_eq!(color_from_xml(&color_xml(104, 1, 0)), Some(Color::LIME));
        assert_eq!(color_from_xml(&color_xml(164, 1, 0)), Some(Color::WHITE));
        assert_eq!(known_color_named("Lime"), Some(104));
        assert_eq!(known_color_named("White"), Some(164));
        assert_eq!(known_color_named("Red"), Some(141));
        assert_eq!(known_color_named("Transparent"), Some(27));
        assert_eq!(known_color_named("ActiveBorder"), Some(1));
        assert_eq!(known_color_named("RebeccaPurple"), Some(175));
    }

    #[test]
    fn known_colour_values_match_dotnet() {
        for (name, argb) in [
            ("Green", 0xFF008000),
            ("Gray", 0xFF808080),
            ("Maroon", 0xFF800000),
            ("Olive", 0xFF808000),
            ("Purple", 0xFF800080),
            ("Silver", 0xFFC0C0C0),
            ("Navy", 0xFF000080),
            ("Teal", 0xFF008080),
            ("DarkSeaGreen", 0xFF8FBC8F),
            ("ForestGreen", 0xFF228B22),
            ("Transparent", 0x00FFFFFF),
        ] {
            assert_eq!(known_color_argb(known_color_named(name).unwrap()), Some(argb), "{name}");
        }
        assert_eq!(known_color_argb(0), None);
        assert_eq!(known_color_argb(176), None);
        assert_eq!(known_color_argb(-1), None);
    }

    #[test]
    fn known_colour_table_is_in_enum_order() {
        // A spot check across the alphabetic run, where an omission would shift everything after it.
        for (n, name) in
            [(28, "AliceBlue"), (79, "Green"), (104, "Lime"), (141, "Red"), (167, "YellowGreen"), (168, "ButtonFace")]
        {
            assert_eq!(known_color_named(name), Some(n));
        }
    }

    #[test]
    fn argb_colours_read_their_stored_value() {
        assert_eq!(color_from_xml(&color_xml(0, 2, 4290607616)), Some(Color::argb(0xFF, 189, 122, 0)));
        assert_eq!(color_from_xml(&color_xml(0, 2, 4294901760)), Some(Color::RED));
    }

    #[test]
    fn empty_and_named_colours_read_as_zero() {
        assert_eq!(color_from_xml(&color_xml(0, 0, 0)), Some(Color::EMPTY));
        let named = DotNetColor { known_color: 0, name: Some("Plaid".to_string()), state: 8, value: 0 };
        assert_eq!(named.to_color(), Color::EMPTY);
    }

    #[test]
    fn a_colour_writes_the_fields_csharp_writes() {
        let e = color_to_xml(SPEC_NS, "Color", Color::RED);
        assert_eq!(e.child("knownColor").unwrap().text(), "0");
        assert!(e.child("name").unwrap().nil);
        assert_eq!(e.child("state").unwrap().text(), "2");
        assert_eq!(e.child("value").unwrap().text(), "4294901760");
        assert_eq!(DotNetColor::from_color(Color::EMPTY).state, 0);
        let t = DotNetColor::from_color(Color::TRANSPARENT);
        assert_eq!((t.known_color, t.state, t.value), (27, 1, 0));
        assert_eq!(t.to_color(), Color::TRANSPARENT);
    }

    #[test]
    fn known_colours_survive_field_for_field() {
        let known = DotNetColor::from_known(164);
        let back = DotNetColor::from_xml(&known.to_xml(SPEC_NS, "Color")).unwrap();
        assert_eq!(back, known);
    }

    const CURVE: &str = r#"<LevelCurve xmlns="http://schemas.datacontract.org/2004/07/Test" xmlns:a="http://www.w3.org/2001/XMLSchema" xmlns:i="http://www.w3.org/2001/XMLSchema-instance" xmlns:c="http://schemas.datacontract.org/2004/07/VixenModules.App.Curves" xmlns:z="http://schemas.datacontract.org/2004/07/ZedGraph">
  <c:IsCurrentLibraryCurve>false</c:IsCurrentLibraryCurve>
  <c:Points>
    <z:PointPair>
      <schema xmlns="" i:type="a:int">11</schema>
      <X xmlns="" i:type="a:double">0</X>
      <Y xmlns="" i:type="a:double">50.09116921142579</Y>
      <schema2 xmlns="" i:type="a:int">11</schema2>
      <Z xmlns="" i:type="a:double">0</Z>
      <Tag xmlns="" i:nil="true" />
    </z:PointPair>
    <z:PointPair>
      <schema xmlns="" i:type="a:int">11</schema>
      <X xmlns="" i:type="a:double">100</X>
      <Y xmlns="" i:type="a:double">100</Y>
      <schema2 xmlns="" i:type="a:int">11</schema2>
      <Z xmlns="" i:type="a:double">0</Z>
      <Tag xmlns="" i:nil="true" />
    </z:PointPair>
  </c:Points>
  <c:_libraryReferenceName></c:_libraryReferenceName>
</LevelCurve>"#;

    #[test]
    fn curve_round_trips_through_typed_values() {
        let e = parse(CURVE);
        let curve = curve_from_xml(&e).unwrap();
        assert_eq!(curve.points, vec![(0.0, 50.09116921142579), (100.0, 100.0)]);
        assert_eq!(curve.library_reference_name, "");
        assert!(!curve.is_current_library_curve);
        let out = curve_to_xml(SPEC_NS, "LevelCurve", &curve);
        assert!(same_tree(&e, &out), "{:?}", e.first_difference(&out, String::new()));
    }

    #[test]
    fn curve_library_references_are_read() {
        let mut curve = Curve::flat100();
        curve.library_reference_name = "Accelerate".to_string();
        curve.is_current_library_curve = true;
        let back = curve_from_xml(&curve_to_xml(SPEC_NS, "C", &curve)).unwrap();
        assert_eq!(back, curve);
    }

    #[test]
    fn a_nil_curve_name_reads_as_empty_like_the_csharp_property() {
        let mut e = curve_to_xml(SPEC_NS, "C", &Curve::flat100());
        *e.child_mut("_libraryReferenceName").unwrap() = XElement::nil(NS_CURVES, "_libraryReferenceName");
        assert_eq!(curve_from_xml(&e).unwrap().library_reference_name, "");
    }

    #[test]
    fn a_curve_without_points_is_not_a_curve() {
        assert!(curve_from_xml(&XElement::new(SPEC_NS, "C")).is_none());
    }

    #[test]
    fn gradient_round_trips_including_clipped_out_of_range_values() {
        let mut g = ColorGradient::from_parts(
            vec![
                ColorPoint::from_xyz(Xyz { x: 95.047, y: 100.0, z: 108.883 }, 0.5, 0.0),
                // Direct field values, which `Xyz::new` would clip: serialized fields bypass the constructor.
                ColorPoint::from_xyz(Xyz { x: 120.5, y: -1.0, z: 3.0 }, 0.25, 0.75),
            ],
            vec![
                AlphaPoint { alpha: 1.0, position: 0.0, focus: 0.5 },
                AlphaPoint { alpha: 0.5, position: 1.0, focus: 0.5 },
            ],
        );
        g.title = Some("t".to_string());
        g.library_reference_name = "lib".to_string();
        g.is_current_library_gradient = true;
        g.gammacorrected = true;
        let e = gradient_to_xml(SPEC_NS, "G", &g);
        let back = gradient_from_xml(&e).unwrap();
        assert_eq!(back, g);
        assert!(same_tree(&e, &gradient_to_xml(SPEC_NS, "G", &back)));
        assert_eq!(
            e.elements().map(|c| c.local.as_str()).collect::<Vec<_>>(),
            ["IsCurrentLibraryGradient", "_alphas", "_colors", "_gammacorrected", "_libraryReferenceName", "_title",]
        );
    }

    #[test]
    fn gradient_title_keeps_nil_and_empty_apart() {
        let mut g = ColorGradient::solid(Color::RED);
        assert!(gradient_to_xml(SPEC_NS, "G", &g).child("_title").unwrap().nil);
        g.title = Some(String::new());
        let e = gradient_to_xml(SPEC_NS, "G", &g);
        assert!(!e.child("_title").unwrap().nil);
        assert_eq!(gradient_from_xml(&e).unwrap().title.as_deref(), Some(""));
    }

    #[test]
    fn level_pairs_and_lists_round_trip() {
        let pairs = vec![GradientLevelPair::solid(Color::RED), GradientLevelPair::solid(Color::BLUE)];
        let e = level_pair_list_to_xml(SPEC_NS, "Colors", &pairs);
        assert_eq!(e.children_named("GradientLevelPair").count(), 2);
        assert_eq!(level_pair_list_from_xml(&e).unwrap(), pairs);
        let list = vec![ColorGradient::solid(Color::RED)];
        assert_eq!(gradient_list_from_xml(&gradient_list_to_xml(SPEC_NS, "Colors", &list)).unwrap(), list);
    }

    #[test]
    fn rgb_is_written_blue_first() {
        let e = rgb_to_xml(SPEC_NS, "color", Rgb { r: 1.0, g: 0.5, b: 0.0392156862745098 });
        assert_eq!(e.elements().map(|c| c.local.as_str()).collect::<Vec<_>>(), ["_b", "_g", "_r"]);
        assert_eq!(rgb_from_xml(&e).unwrap(), Rgb { r: 1.0, g: 0.5, b: 0.0392156862745098 });
    }

    #[test]
    fn percentage_holds_a_float() {
        let e = percentage_to_xml(SPEC_NS, "P", 0.2f32 as f64);
        assert_eq!(e.child("Value").unwrap().text(), "0.2");
        assert_eq!(percentage_from_xml(&e), Some(0.2f32 as f64));
    }

    #[test]
    fn fonts_use_the_converter_string() {
        for text in [
            "Arial, 8pt",
            "Microsoft Sans Serif, 8.25pt, style=Bold",
            "Arial, 12pt, style=Italic",
            "Arial, 8pt, style=Bold, Italic",
        ] {
            assert_eq!(font_to_text(&font_from_text(text).unwrap()), text);
        }
        let f = font_from_text("Arial, 8.25pt, style=Bold, Italic").unwrap();
        assert_eq!((f.name.as_str(), f.size, f.bold, f.italic), ("Arial", 8.25, true, true));
        assert_eq!(font_from_text("Arial; 8,25pt").unwrap().size, 8.25);
        assert!(font_from_text("").is_none());
        assert!(font_from_text("Arial").is_none());
        let e = font_to_xml(SPEC_NS, "Font", &FontDesc::new("Arial", 8.0));
        assert_eq!(e.child("SerializeFontAttribute").unwrap().text(), "Arial, 8pt");
        assert_eq!(font_from_xml(&e), Some(FontDesc::new("Arial", 8.0)));
    }
}
