//! Typed form of the intelligent fixture description: `FixtureSpecification` and the types under it
//! (App/Fixture), as stored in the `IntelligentFixture` element property (Property/IntelligentFixture).
//!
//! `vixen_core` keeps that property as `PropertyData::Unknown { raw_xml }`, so the specification is
//! parsed from the DataContract XML on first use (see `FixtureTargets` in `fixture_base.rs`).

use vixen_core::value::FunctionIdentity;
use vixen_core::{Color, Uuid};

/// `IntelligentFixtureDescriptor._typeId`.
pub const INTELLIGENT_FIXTURE_TYPE_ID: Uuid = uuid::uuid!("fdd74c85-1779-4521-b559-0dd5ea71054e");

/// `FixtureFunctionType`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FixtureFunctionType {
    #[default]
    Range,
    Indexed,
    ColorWheel,
    RgbColor,
    RgbwColor,
    None,
}

impl FixtureFunctionType {
    /// The enum member name, as DataContractSerializer writes it.
    pub fn name(self) -> &'static str {
        match self {
            FixtureFunctionType::Range => "Range",
            FixtureFunctionType::Indexed => "Indexed",
            FixtureFunctionType::ColorWheel => "ColorWheel",
            FixtureFunctionType::RgbColor => "RGBColor",
            FixtureFunctionType::RgbwColor => "RGBWColor",
            FixtureFunctionType::None => "None",
        }
    }

    pub fn from_name(s: &str) -> FixtureFunctionType {
        match s.trim() {
            "Indexed" => FixtureFunctionType::Indexed,
            "ColorWheel" => FixtureFunctionType::ColorWheel,
            "RGBColor" => FixtureFunctionType::RgbColor,
            "RGBWColor" => FixtureFunctionType::RgbwColor,
            "None" => FixtureFunctionType::None,
            _ => FixtureFunctionType::Range,
        }
    }
}

/// `Vixen.Data.Value.FunctionIdentity` as the C# defines it. `vixen_core::value::FunctionIdentity` has no
/// `SpinColorWheel` (and has `Color` and `Other`, which the C# lacks), so matching against fixture
/// descriptions uses this enum and `to_core` converts for the intents.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum FuncId {
    Pan,
    Tilt,
    Zoom,
    Dim,
    Shutter,
    Gobo,
    Frost,
    Prism,
    OpenClosePrism,
    SpinColorWheel,
    #[default]
    Custom,
}

impl FuncId {
    pub fn name(self) -> &'static str {
        match self {
            FuncId::Pan => "Pan",
            FuncId::Tilt => "Tilt",
            FuncId::Zoom => "Zoom",
            FuncId::Dim => "Dim",
            FuncId::Shutter => "Shutter",
            FuncId::Gobo => "Gobo",
            FuncId::Frost => "Frost",
            FuncId::Prism => "Prism",
            FuncId::OpenClosePrism => "OpenClosePrism",
            FuncId::SpinColorWheel => "SpinColorWheel",
            FuncId::Custom => "Custom",
        }
    }

    pub fn from_name(s: &str) -> FuncId {
        match s.trim() {
            "Pan" => FuncId::Pan,
            "Tilt" => FuncId::Tilt,
            "Zoom" => FuncId::Zoom,
            "Dim" => FuncId::Dim,
            "Shutter" => FuncId::Shutter,
            "Gobo" => FuncId::Gobo,
            "Frost" => FuncId::Frost,
            "Prism" => FuncId::Prism,
            "OpenClosePrism" => FuncId::OpenClosePrism,
            "SpinColorWheel" => FuncId::SpinColorWheel,
            _ => FuncId::Custom,
        }
    }

    /// The closest `vixen_core` member. `SpinColorWheel` has none, so it becomes `Color`.
    pub fn to_core(self) -> FunctionIdentity {
        match self {
            FuncId::Pan => FunctionIdentity::Pan,
            FuncId::Tilt => FunctionIdentity::Tilt,
            FuncId::Zoom => FunctionIdentity::Zoom,
            FuncId::Dim => FunctionIdentity::Dim,
            FuncId::Shutter => FunctionIdentity::Shutter,
            FuncId::Gobo => FunctionIdentity::Gobo,
            FuncId::Frost => FunctionIdentity::Frost,
            FuncId::Prism => FunctionIdentity::Prism,
            FuncId::OpenClosePrism => FunctionIdentity::OpenClosePrism,
            FuncId::SpinColorWheel => FunctionIdentity::SpinColorWheel,
            FuncId::Custom => FunctionIdentity::Custom,
        }
    }
}

/// `Vixen.Commands.FixtureIndexType`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FixtureIndexType {
    #[default]
    Custom,
    ShutterOpen,
    ShutterClosed,
    LampOn,
    LampOff,
    ColorWheel,
    Prism,
    PrismOpen,
    PrismClose,
    Strobe,
}

impl From<FixtureIndexType> for vixen_core::value::FixtureIndexType {
    fn from(t: FixtureIndexType) -> Self {
        use vixen_core::value::FixtureIndexType as Core;
        match t {
            FixtureIndexType::Custom => Core::Custom,
            FixtureIndexType::ShutterOpen => Core::ShutterOpen,
            FixtureIndexType::ShutterClosed => Core::ShutterClosed,
            FixtureIndexType::LampOn => Core::LampOn,
            FixtureIndexType::LampOff => Core::LampOff,
            FixtureIndexType::ColorWheel => Core::ColorWheel,
            FixtureIndexType::Prism => Core::Prism,
            FixtureIndexType::PrismOpen => Core::PrismOpen,
            FixtureIndexType::PrismClose => Core::PrismClose,
            FixtureIndexType::Strobe => Core::Strobe,
        }
    }
}

impl FixtureIndexType {
    pub fn name(self) -> &'static str {
        match self {
            FixtureIndexType::Custom => "Custom",
            FixtureIndexType::ShutterOpen => "ShutterOpen",
            FixtureIndexType::ShutterClosed => "ShutterClosed",
            FixtureIndexType::LampOn => "LampOn",
            FixtureIndexType::LampOff => "LampOff",
            FixtureIndexType::ColorWheel => "ColorWheel",
            FixtureIndexType::Prism => "Prism",
            FixtureIndexType::PrismOpen => "PrismOpen",
            FixtureIndexType::PrismClose => "PrismClose",
            FixtureIndexType::Strobe => "Strobe",
        }
    }

    pub fn from_name(s: &str) -> FixtureIndexType {
        match s.trim() {
            "ShutterOpen" => FixtureIndexType::ShutterOpen,
            "ShutterClosed" => FixtureIndexType::ShutterClosed,
            "LampOn" => FixtureIndexType::LampOn,
            "LampOff" => FixtureIndexType::LampOff,
            "ColorWheel" => FixtureIndexType::ColorWheel,
            "Prism" => FixtureIndexType::Prism,
            "PrismOpen" => FixtureIndexType::PrismOpen,
            "PrismClose" => FixtureIndexType::PrismClose,
            "Strobe" => FixtureIndexType::Strobe,
            _ => FixtureIndexType::Custom,
        }
    }
}

/// `FixtureZoomType`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FixtureZoomType {
    #[default]
    NarrowToWide,
    WideToNarrow,
}

/// `FixtureChannel`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct FixtureChannel {
    pub name: String,
    pub function: String,
    pub channel_number: i32,
}

/// `FixtureRotationLimits`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FixtureRotationLimits {
    pub start_position: i32,
    pub stop_position: i32,
}

impl Default for FixtureRotationLimits {
    fn default() -> Self {
        FixtureRotationLimits { start_position: 0, stop_position: 360 }
    }
}

/// `FixtureIndexBase` and its two concrete kinds, `FixtureIndex` and `FixtureColorWheel`.
#[derive(Clone, PartialEq, Debug)]
pub struct FixtureIndexItem {
    pub name: String,
    pub start_value: i32,
    pub end_value: i32,
    pub use_curve: bool,
    pub index_type: FixtureIndexType,
    /// `FixtureIndex.Image`.
    pub image: String,
    /// `FixtureColorWheel.Color1`.
    pub color1: Color,
    pub color2: Color,
    pub half_step: bool,
    pub exclude_color_property: bool,
}

impl FixtureIndexItem {
    /// `new FixtureIndex()`.
    pub fn index() -> FixtureIndexItem {
        FixtureIndexItem {
            name: String::new(),
            start_value: 0,
            end_value: 0,
            use_curve: false,
            index_type: FixtureIndexType::Custom,
            image: String::new(),
            color1: Color::WHITE,
            color2: Color::WHITE,
            half_step: false,
            exclude_color_property: false,
        }
    }

    /// `new FixtureColorWheel()`: white colours and the `ColorWheel` index type.
    pub fn color_wheel() -> FixtureIndexItem {
        FixtureIndexItem { index_type: FixtureIndexType::ColorWheel, ..FixtureIndexItem::index() }
    }
}

/// `FixtureFunction`.
#[derive(Clone, PartialEq, Debug)]
pub struct FixtureFunction {
    pub name: String,
    pub function_type: FixtureFunctionType,
    pub identity: FuncId,
    pub index_data: Vec<FixtureIndexItem>,
    pub color_wheel_data: Vec<FixtureIndexItem>,
    pub label: String,
    pub rotation_limits: FixtureRotationLimits,
    pub zoom_type: FixtureZoomType,
    pub timeline_color: Color,
    pub associated_function_name: String,
}

impl Default for FixtureFunction {
    /// `new FixtureFunction()`.
    fn default() -> Self {
        FixtureFunction {
            name: String::new(),
            function_type: FixtureFunctionType::Range,
            identity: FuncId::Custom,
            index_data: Vec::new(),
            color_wheel_data: Vec::new(),
            label: String::new(),
            rotation_limits: FixtureRotationLimits::default(),
            zoom_type: FixtureZoomType::NarrowToWide,
            timeline_color: Color::WHITE,
            associated_function_name: String::new(),
        }
    }
}

impl FixtureFunction {
    /// `GetIndexDataBase()`.
    pub fn index_data_base(&self) -> &[FixtureIndexItem] {
        match self.function_type {
            FixtureFunctionType::Indexed => &self.index_data,
            FixtureFunctionType::ColorWheel => &self.color_wheel_data,
            _ => &[],
        }
    }
}

/// `FixtureSpecification`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct FixtureSpecification {
    pub name: String,
    pub manufacturer: String,
    pub channels: Vec<FixtureChannel>,
    pub functions: Vec<FixtureFunction>,
}

impl FixtureSpecification {
    /// `SupportsFunction(string)`: defined, and used on at least one channel.
    pub fn supports_function_name(&self, name: &str) -> bool {
        self.functions.iter().any(|f| f.name == name) && self.channels.iter().any(|c| c.function == name)
    }

    /// `SupportsFunction(FunctionIdentity)`: the first function with the identity is used on a channel.
    pub fn supports_function(&self, identity: FuncId) -> bool {
        match self.functions.iter().find(|f| f.identity == identity) {
            Some(f) => self.channels.iter().any(|c| c.function == f.name),
            None => false,
        }
    }

    /// `GetFunction`: the function with the name and identity. The C# uses `SingleOrDefault`, which
    /// throws when a profile defines two; the first one wins here.
    pub fn get_function(&self, name: &str, identity: FuncId) -> Option<&FixtureFunction> {
        self.functions.iter().find(|f| f.identity == identity && f.name == name)
    }

    /// `IsFunctionUsed`.
    pub fn is_function_used(&self, name: &str, identity: FuncId) -> bool {
        self.get_function(name, identity).is_some() && self.channels.iter().any(|c| c.function == name)
    }

    /// `GetInUseFunction`.
    pub fn get_in_use_function(&self, name: &str, identity: FuncId) -> Option<&FixtureFunction> {
        if self.is_function_used(name, identity) {
            self.get_function(name, identity)
        } else {
            None
        }
    }

    /// Serialises to the DataContract XML that `parse` reads, for tests and for tools that build fixtures.
    pub fn to_raw_xml(&self) -> String {
        let mut x = String::new();
        x.push_str("<IntelligentFixtureData><FixtureSpecification>");
        x.push_str("<ChannelDefinitions>");
        for c in &self.channels {
            x.push_str(&format!(
                "<FixtureChannel><Name>{}</Name><ChannelNumber>{}</ChannelNumber><Function>{}</Function></FixtureChannel>",
                esc(&c.name),
                c.channel_number,
                esc(&c.function)
            ));
        }
        x.push_str("</ChannelDefinitions><FunctionDefinitions>");
        for f in &self.functions {
            x.push_str("<FixtureFunction>");
            x.push_str(&format!("<Name>{}</Name>", esc(&f.name)));
            x.push_str(&format!("<AssociatedFunctionName>{}</AssociatedFunctionName>", esc(&f.associated_function_name)));
            x.push_str("<ColorWheelData>");
            for i in &f.color_wheel_data {
                x.push_str(&index_xml("FixtureColorWheel", i, true));
            }
            x.push_str("</ColorWheelData>");
            x.push_str(&format!("<FunctionIdentity>{}</FunctionIdentity>", f.identity.name()));
            x.push_str(&format!("<FunctionType>{}</FunctionType>", f.function_type.name()));
            x.push_str("<IndexData>");
            for i in &f.index_data {
                x.push_str(&index_xml("FixtureIndex", i, false));
            }
            x.push_str("</IndexData>");
            x.push_str(&format!("<Label>{}</Label>", esc(&f.label)));
            x.push_str(&format!(
                "<RotationLimits><StartPosition>{}</StartPosition><StopPosition>{}</StopPosition></RotationLimits>",
                f.rotation_limits.start_position, f.rotation_limits.stop_position
            ));
            x.push_str(&format!("<TimelineColor1Html>{}</TimelineColor1Html>", color_to_html(f.timeline_color)));
            x.push_str(&format!(
                "<ZoomType>{}</ZoomType>",
                if f.zoom_type == FixtureZoomType::NarrowToWide { "NarrowToWide" } else { "WideToNarrow" }
            ));
            x.push_str("</FixtureFunction>");
        }
        x.push_str("</FunctionDefinitions>");
        x.push_str(&format!("<Manufacturer>{}</Manufacturer><Name>{}</Name>", esc(&self.manufacturer), esc(&self.name)));
        x.push_str("</FixtureSpecification></IntelligentFixtureData>");
        x
    }
}

fn index_xml(tag: &str, i: &FixtureIndexItem, wheel: bool) -> String {
    let mut s = format!("<{tag}>");
    s.push_str(&format!("<Name>{}</Name><EndValue>{}</EndValue>", esc(&i.name), i.end_value));
    s.push_str(&format!("<IndexType>{}</IndexType><StartValue>{}</StartValue>", i.index_type.name(), i.start_value));
    s.push_str(&format!("<UseCurve>{}</UseCurve>", i.use_curve));
    if wheel {
        s.push_str(&format!(
            "<Color1Html>{}</Color1Html><Color2Html>{}</Color2Html><ExcludeColorProperty>{}</ExcludeColorProperty><HalfStep>{}</HalfStep>",
            color_to_html(i.color1),
            color_to_html(i.color2),
            i.exclude_color_property,
            i.half_step
        ));
    } else {
        s.push_str(&format!("<Image>{}</Image>", esc(&i.image)));
    }
    s.push_str(&format!("</{tag}>"));
    s
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// `ColorTranslator.ToHtml`, always as `#RRGGBB`.
pub fn color_to_html(c: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

/// `ColorTranslator.FromHtml`: `#RRGGBB`, `#RGB` or one of the common HTML colour names. Anything else is
/// white, which is also what the C# falls back to for an empty timeline colour.
pub fn color_from_html(s: &str) -> Color {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        let v = |i: usize, n: usize| u8::from_str_radix(hex.get(i..i + n).unwrap_or(""), 16).ok();
        if hex.len() == 6 {
            if let (Some(r), Some(g), Some(b)) = (v(0, 2), v(2, 2), v(4, 2)) {
                return Color::rgb(r, g, b);
            }
        } else if hex.len() == 3 {
            if let (Some(r), Some(g), Some(b)) = (v(0, 1), v(1, 1), v(2, 1)) {
                return Color::rgb(r * 17, g * 17, b * 17);
            }
        }
        return Color::WHITE;
    }
    match s.to_ascii_lowercase().as_str() {
        "black" => Color::BLACK,
        "red" => Color::RED,
        "green" => Color::GREEN,
        "lime" => Color::LIME,
        "blue" => Color::BLUE,
        "yellow" => Color::YELLOW,
        "cyan" | "aqua" => Color::CYAN,
        "magenta" | "fuchsia" => Color::MAGENTA,
        "orange" => Color::ORANGE,
        "gray" | "grey" => Color::GRAY,
        "darkgray" | "darkgrey" => Color::DARK_GRAY,
        "dimgray" | "dimgrey" => Color::DIM_GRAY,
        "purple" => Color::rgb(128, 0, 128),
        "pink" => Color::rgb(255, 192, 203),
        "lightblue" => Color::rgb(173, 216, 230),
        "lightskyblue" => Color::rgb(135, 206, 250),
        "hotpink" => Color::rgb(255, 105, 180),
        "brown" => Color::rgb(165, 42, 42),
        "silver" => Color::rgb(192, 192, 192),
        "transparent" => Color::TRANSPARENT,
        _ => Color::WHITE,
    }
}

// ---- parsing -------------------------------------------------------------------------------------------

type Node<'a, 'b> = roxmltree::Node<'a, 'b>;

fn child<'a, 'b>(n: Node<'a, 'b>, name: &str) -> Option<Node<'a, 'b>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

fn text(n: Node, name: &str) -> String {
    // DataContract strings preserve whitespace; function names are exact lookup keys.
    child(n, name).and_then(|c| c.text()).unwrap_or("").to_string()
}

fn int(n: Node, name: &str, default: i32) -> i32 {
    child(n, name).and_then(|c| c.text()).and_then(|t| t.trim().parse().ok()).unwrap_or(default)
}

fn boolean(n: Node, name: &str) -> bool {
    matches!(child(n, name).and_then(|c| c.text()).map(|t| t.trim()), Some("true") | Some("1"))
}

fn items<'a, 'b>(n: Node<'a, 'b>, list: &str) -> Vec<Node<'a, 'b>> {
    match child(n, list) {
        Some(l) => l.children().filter(|c| c.is_element()).collect(),
        None => Vec::new(),
    }
}

fn parse_index(n: Node, wheel: bool) -> FixtureIndexItem {
    let mut i = if wheel { FixtureIndexItem::color_wheel() } else { FixtureIndexItem::index() };
    i.name = text(n, "Name");
    i.start_value = int(n, "StartValue", 0);
    i.end_value = int(n, "EndValue", 0);
    i.use_curve = boolean(n, "UseCurve");
    if let Some(t) = child(n, "IndexType").and_then(|c| c.text()) {
        i.index_type = FixtureIndexType::from_name(t);
    }
    if wheel {
        // `FixtureColorWheel.UseCurve`'s setter forces the colour wheel index type when it is true.
        if i.use_curve {
            i.index_type = FixtureIndexType::ColorWheel;
        }
        i.half_step = boolean(n, "HalfStep");
        i.exclude_color_property = boolean(n, "ExcludeColorProperty");
        if let Some(t) = child(n, "Color1Html").and_then(|c| c.text()) {
            i.color1 = color_from_html(t);
        }
        if let Some(t) = child(n, "Color2Html").and_then(|c| c.text()) {
            i.color2 = color_from_html(t);
        }
    } else {
        i.image = text(n, "Image");
    }
    i
}

fn parse_function(n: Node) -> FixtureFunction {
    let mut f = FixtureFunction {
        name: text(n, "Name"),
        function_type: FixtureFunctionType::from_name(&text(n, "FunctionType")),
        identity: FuncId::from_name(&text(n, "FunctionIdentity")),
        label: text(n, "Label"),
        associated_function_name: text(n, "AssociatedFunctionName"),
        ..FixtureFunction::default()
    };
    f.zoom_type =
        if text(n, "ZoomType") == "WideToNarrow" { FixtureZoomType::WideToNarrow } else { FixtureZoomType::NarrowToWide };
    // An empty colour is the "White" the C# setter substitutes for files written by an old bug.
    let html = text(n, "TimelineColor1Html");
    f.timeline_color = if html.is_empty() { Color::WHITE } else { color_from_html(&html) };
    if let Some(r) = child(n, "RotationLimits") {
        f.rotation_limits = FixtureRotationLimits { start_position: int(r, "StartPosition", 0), stop_position: int(r, "StopPosition", 360) };
    }
    f.index_data = items(n, "IndexData").into_iter().map(|i| parse_index(i, false)).collect();
    f.color_wheel_data = items(n, "ColorWheelData").into_iter().map(|i| parse_index(i, true)).collect();
    f
}

/// Parses the `IntelligentFixtureData` XML (or a bare `FixtureSpecification`). Missing members take the C#
/// constructors' defaults, and unparseable XML yields `None`.
pub fn parse_specification(raw_xml: &str) -> Option<FixtureSpecification> {
    let doc = roxmltree::Document::parse(raw_xml).ok()?;
    let root = doc.root_element();
    let spec_node = if root.tag_name().name() == "FixtureSpecification" {
        root
    } else {
        root.descendants().find(|n| n.is_element() && n.tag_name().name() == "FixtureSpecification")?
    };
    let channels = items(spec_node, "ChannelDefinitions")
        .into_iter()
        .map(|c| FixtureChannel { name: text(c, "Name"), function: text(c, "Function"), channel_number: int(c, "ChannelNumber", 0) })
        .collect();
    let functions = items(spec_node, "FunctionDefinitions").into_iter().map(parse_function).collect();
    Some(FixtureSpecification { name: text(spec_node, "Name"), manufacturer: text(spec_node, "Manufacturer"), channels, functions })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A moving head with pan, tilt, dimmer, zoom, frost, a gobo wheel, a strobe shutter and a colour wheel.
    pub fn sample() -> FixtureSpecification {
        let mut spec = FixtureSpecification { name: "Test Head".into(), ..Default::default() };
        let mut add = |name: &str, ty: FixtureFunctionType, id: FuncId, label: &str| {
            let f = FixtureFunction { name: name.into(), function_type: ty, identity: id, label: label.into(), ..Default::default() };
            spec.functions.push(f);
            spec.channels.push(FixtureChannel { name: name.into(), function: name.into(), channel_number: spec.channels.len() as i32 + 1 });
        };
        add("Pan", FixtureFunctionType::Range, FuncId::Pan, "P");
        add("Tilt", FixtureFunctionType::Range, FuncId::Tilt, "T");
        add("Dimmer", FixtureFunctionType::Range, FuncId::Dim, "D");
        add("Zoom", FixtureFunctionType::Range, FuncId::Zoom, "Z");
        add("Frost", FixtureFunctionType::Range, FuncId::Frost, "F");
        add("Gobo Wheel", FixtureFunctionType::Indexed, FuncId::Gobo, "G");
        add("Shutter", FixtureFunctionType::Indexed, FuncId::Shutter, "S");
        spec
    }

    #[test]
    fn round_trips_through_xml() {
        let mut spec = sample();
        spec.functions[5].index_data.push(FixtureIndexItem {
            name: "Open".into(),
            start_value: 5,
            end_value: 9,
            use_curve: true,
            ..FixtureIndexItem::index()
        });
        spec.functions[0].rotation_limits = FixtureRotationLimits { start_position: 0, stop_position: 540 };
        let parsed = parse_specification(&spec.to_raw_xml()).unwrap();
        assert_eq!(parsed.functions.len(), spec.functions.len());
        assert_eq!(parsed.functions[5].index_data, spec.functions[5].index_data);
        assert_eq!(parsed.functions[0].rotation_limits.stop_position, 540);
        assert!(parsed.supports_function(FuncId::Pan));
        assert!(!parsed.supports_function(FuncId::Prism));
    }

    #[test]
    fn fixture_strings_keep_authored_whitespace() {
        let mut spec = sample();
        spec.name = "  Named Head  ".into();
        spec.functions[0].name = " Pan ".into();
        spec.functions[0].label = " P ".into();
        spec.channels[0].function = " Pan ".into();
        let parsed = parse_specification(&spec.to_raw_xml()).unwrap();
        assert_eq!(parsed.name, "  Named Head  ");
        let pan = parsed.get_in_use_function(" Pan ", FuncId::Pan).unwrap();
        assert_eq!(pan.label, " P ");
        assert!(parsed.get_in_use_function("Pan", FuncId::Pan).is_none());
    }

    #[test]
    fn identity_must_match_to_be_in_use() {
        let spec = sample();
        assert!(spec.get_in_use_function("Pan", FuncId::Pan).is_some());
        assert!(spec.get_in_use_function("Pan", FuncId::Tilt).is_none());
        let mut unused = sample();
        unused.channels.clear();
        assert!(!unused.supports_function(FuncId::Pan));
    }

    #[test]
    fn html_colours() {
        assert_eq!(color_from_html("#FF8000"), Color::rgb(255, 128, 0));
        assert_eq!(color_from_html("LightBlue"), Color::rgb(173, 216, 230));
        assert_eq!(color_from_html(""), Color::WHITE);
    }
}
