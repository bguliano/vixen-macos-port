//! The Intelligent Fixture template and its wizard (F110, F139 to F147): `IntelligentFixtureTemplate`, the wizard's page models and view models, and the
//! profile files of `FixtureSpecificationManager`. The wizard is a plain value the window drives, so its rules run without a window.

use super::elements::EditResult;
use super::flow::{Cid, CompKind, SourceRef};
use super::tree::{child_path, Nid};
use super::SetupSession;
use crate::logging;
use fx_fixture::spec::{
    FixtureFunctionType, FixtureIndexType, FixtureRotationLimits, FixtureZoomType, FuncId,
};
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;
use vixen_core::element::{property_ids, ElementColorType, PropertyData, PropertyInstance};
use vixen_core::Color;
use vixen_io::file::write_atomic;
use vixen_io::xml::{parse_fragment, read_xml_file, write_fragment};
use vixen_io::ModuleEntry;
use vixen_output::filters::{
    CoarseFineConfig, ColorBreakdownConfig, ColorWheelFilterConfig, CurveData, DimmingCurveConfig,
    DimmingFilterConfig, FixtureColorWheelEntry, PrismFilterConfig, ShutterFilterConfig,
    TaggedFilterConfig,
};
use vixen_output::{DotNetColor, FilterConfig};
use vixen_xml::{contract_ns, XElement};

const LOGGER: &str = "VixenModules.App.FixtureSpecificationManager.FixtureSpecificationManager";

// ---- The profile ----------------------------------------------------------------------------------------------------

/// `FixtureChannel`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileChannel {
    pub name: Option<String>,
    pub function: Option<String>,
    pub channel_number: i32,
}

/// `FixtureIndex`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileIndex {
    pub name: Option<String>,
    pub start_value: i32,
    pub end_value: i32,
    pub use_curve: bool,
    pub index_type: FixtureIndexType,
    pub image: Option<String>,
}

/// `FixtureColorWheel`. Its index type is always `ColorWheel`: the constructor sets it, and `CreateInstanceForClone`, which every load and wizard pass through, doesn't copy it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileWheelEntry {
    pub name: Option<String>,
    pub start_value: i32,
    pub end_value: i32,
    pub use_curve: bool,
    pub half_step: bool,
    pub color1: DotNetColor,
    pub color2: DotNetColor,
    pub exclude_color_property: bool,
}

/// `FixtureFunction`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileFunction {
    pub name: Option<String>,
    pub function_type: FixtureFunctionType,
    pub identity: FuncId,
    pub index_data: Vec<ProfileIndex>,
    pub color_wheel_data: Vec<ProfileWheelEntry>,
    pub label: Option<String>,
    pub rotation_limits: FixtureRotationLimits,
    pub zoom_type: FixtureZoomType,
    pub timeline_color: DotNetColor,
    pub associated_function_name: Option<String>,
}

impl Default for ProfileFunction {
    /// `new FixtureFunction()`.
    fn default() -> Self {
        ProfileFunction {
            name: None,
            function_type: FixtureFunctionType::Range,
            identity: FuncId::Custom,
            index_data: Vec::new(),
            color_wheel_data: Vec::new(),
            label: None,
            rotation_limits: FixtureRotationLimits::default(),
            zoom_type: FixtureZoomType::NarrowToWide,
            timeline_color: DotNetColor::WHITE,
            associated_function_name: None,
        }
    }
}

/// `FixtureSpecification` as its file and module data hold it. `fx_fixture::spec::FixtureSpecification` is the effects' reading of the same data: it has no `CreatedBy` or `Revision`, and it
/// reduces colours to channel values, which loses the difference between `Red` and `#FF0000` that a colour wheel filter's matching depends on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixtureProfile {
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub created_by: Option<String>,
    pub revision: Option<String>,
    pub channels: Vec<ProfileChannel>,
    pub functions: Vec<ProfileFunction>,
}

/// `Environment.UserName`.
fn user_name() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_default()
}

/// `InitializeBuiltInFunctions`.
fn built_in_functions() -> Vec<ProfileFunction> {
    use FixtureFunctionType as T;
    let function = |name: &str, function_type: T, identity: FuncId, color: &str| ProfileFunction {
        name: Some(name.to_string()),
        function_type,
        identity,
        timeline_color: DotNetColor::from_html(color).unwrap_or(DotNetColor::WHITE),
        ..ProfileFunction::default()
    };
    vec![
        function("Pan", T::Range, FuncId::Pan, "Red"),
        function("Tilt", T::Range, FuncId::Tilt, "Green"),
        function("Color Wheel", T::ColorWheel, FuncId::Custom, "White"),
        function("Dimmer", T::Range, FuncId::Dim, "Purple"),
        function("Color", T::RgbwColor, FuncId::Custom, "DarkGray"),
        function("Zoom", T::Range, FuncId::Zoom, "Blue"),
        function("Shutter", T::Indexed, FuncId::Shutter, "Black"),
        function("Gobo Wheel", T::Indexed, FuncId::Gobo, "Pink"),
        function(
            "Open Close Prism",
            T::Indexed,
            FuncId::OpenClosePrism,
            "Orange",
        ),
        function("Prism", T::Indexed, FuncId::Prism, "Yellow"),
        function("Frost", T::Range, FuncId::Frost, "LightBlue"),
        function("None", T::None, FuncId::Custom, "Transparent"),
    ]
}

impl FixtureProfile {
    /// `new FixtureSpecification()` plus `InitializeBuiltInFunctions`.
    pub fn new_blank() -> FixtureProfile {
        FixtureProfile {
            name: None,
            manufacturer: None,
            created_by: Some(user_name()),
            revision: Some("1.0".to_string()),
            channels: Vec::new(),
            functions: built_in_functions(),
        }
    }

    pub fn name_text(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    fn function_of(&self, channel: &ProfileChannel) -> Option<&ProfileFunction> {
        self.functions
            .iter()
            .find(|f| f.name.is_some() && f.name == channel.function)
    }

    fn uses_function_type(&self, types: &[FixtureFunctionType]) -> bool {
        self.channels
            .iter()
            .filter_map(|c| self.function_of(c))
            .any(|f| types.contains(&f.function_type))
    }

    /// `IsColorMixing`.
    pub fn is_color_mixing(&self) -> bool {
        self.uses_function_type(&[
            FixtureFunctionType::RgbColor,
            FixtureFunctionType::RgbwColor,
        ])
    }

    /// `ContainsColorWheel`.
    pub fn contains_color_wheel(&self) -> bool {
        self.uses_function_type(&[FixtureFunctionType::ColorWheel])
    }

    /// `SupportsFunction(FunctionIdentity)`: the first function with the identity is used on a channel.
    pub fn supports_function(&self, identity: FuncId) -> bool {
        match self.functions.iter().find(|f| f.identity == identity) {
            Some(f) => self.channels.iter().any(|c| c.function == f.name),
            None => false,
        }
    }

    /// `GetNumberOfColorChannels`.
    fn color_channel_count(&self) -> usize {
        self.channels
            .iter()
            .filter_map(|c| self.function_of(c))
            .filter(|f| {
                matches!(
                    f.function_type,
                    FixtureFunctionType::RgbColor | FixtureFunctionType::RgbwColor
                )
            })
            .count()
    }

    /// `GetAllColorWheelColors`: by channel, so a wheel on two channels gives its colours twice. `Color.FromArgb(Color1.ToArgb())` makes them value colours.
    fn color_wheel_colors(&self) -> Vec<Color> {
        self.channels
            .iter()
            .filter_map(|c| self.function_of(c))
            .filter(|f| f.function_type == FixtureFunctionType::ColorWheel)
            .flat_map(|f| f.color_wheel_data.iter())
            .filter(|e| !e.half_step && !e.use_curve && !e.exclude_color_property)
            .map(|e| e.color1.color())
            .collect()
    }

    /// `GetFileName`. `>` really does become `(`.
    pub fn file_name(&self) -> String {
        let mut name: String = self
            .name_text()
            .chars()
            .map(|c| match c {
                '<' | '>' => '(',
                ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                other => other,
            })
            .collect();
        name.push_str(".xml");
        name
    }
}

// ---- Reading a profile ----------------------------------------------------------------------------------------------

fn optional_text(e: &XElement, local: &str) -> Option<String> {
    let c = e.child(local)?;
    if c.nil {
        None
    } else {
        Some(c.text())
    }
}

fn read_int(e: &XElement, local: &str, default: i32) -> Result<i32, String> {
    match e.child(local) {
        None => Ok(default),
        Some(c) => c
            .text()
            .trim()
            .parse()
            .map_err(|_| format!("{local} isn't a number")),
    }
}

fn read_flag(e: &XElement, local: &str) -> Result<bool, String> {
    match e.child(local).map(XElement::text) {
        None => Ok(false),
        Some(t) => match t.trim() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(format!("{local} isn't true or false")),
        },
    }
}

/// `ColorTranslator.FromHtml`, which throws on what it can't read, and that's a malformed file.
fn read_color(e: &XElement, local: &str, default: DotNetColor) -> Result<DotNetColor, String> {
    match optional_text(e, local) {
        None => Ok(default),
        Some(t) => DotNetColor::from_html(&t).ok_or_else(|| format!("{local} isn't a colour")),
    }
}

/// An enum member. The serializer throws on a name it doesn't know, where `from_name` would quietly take a default.
fn read_member<T: Copy>(
    e: &XElement,
    local: &str,
    default: T,
    from: fn(&str) -> T,
    to: fn(T) -> &'static str,
) -> Result<T, String> {
    match optional_text(e, local) {
        None => Ok(default),
        Some(t) => {
            let value = from(&t);
            if to(value) == t.trim() {
                Ok(value)
            } else {
                Err(format!("{local} has an unknown value"))
            }
        }
    }
}

fn read_list<T>(
    e: &XElement,
    list: &str,
    item: &str,
    read: impl Fn(&XElement) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    match e.child(list) {
        Some(l) => l.children_named(item).map(read).collect(),
        None => Ok(Vec::new()),
    }
}

fn read_channel(e: &XElement) -> Result<ProfileChannel, String> {
    Ok(ProfileChannel {
        name: optional_text(e, "Name"),
        function: optional_text(e, "Function"),
        channel_number: read_int(e, "ChannelNumber", 0)?,
    })
}

fn read_index(e: &XElement) -> Result<ProfileIndex, String> {
    Ok(ProfileIndex {
        name: optional_text(e, "Name"),
        start_value: read_int(e, "StartValue", 0)?,
        end_value: read_int(e, "EndValue", 0)?,
        use_curve: read_flag(e, "UseCurve")?,
        index_type: read_member(
            e,
            "IndexType",
            FixtureIndexType::Custom,
            FixtureIndexType::from_name,
            FixtureIndexType::name,
        )?,
        image: optional_text(e, "Image"),
    })
}

fn read_wheel_entry(e: &XElement) -> Result<ProfileWheelEntry, String> {
    Ok(ProfileWheelEntry {
        name: optional_text(e, "Name"),
        start_value: read_int(e, "StartValue", 0)?,
        end_value: read_int(e, "EndValue", 0)?,
        use_curve: read_flag(e, "UseCurve")?,
        half_step: read_flag(e, "HalfStep")?,
        color1: read_color(e, "Color1Html", DotNetColor::WHITE)?,
        color2: read_color(e, "Color2Html", DotNetColor::WHITE)?,
        exclude_color_property: read_flag(e, "ExcludeColorProperty")?,
    })
}

fn zoom_name(z: FixtureZoomType) -> &'static str {
    match z {
        FixtureZoomType::NarrowToWide => "NarrowToWide",
        FixtureZoomType::WideToNarrow => "WideToNarrow",
    }
}

fn zoom_from_name(s: &str) -> FixtureZoomType {
    if s.trim() == "WideToNarrow" {
        FixtureZoomType::WideToNarrow
    } else {
        FixtureZoomType::NarrowToWide
    }
}

fn read_function(e: &XElement) -> Result<ProfileFunction, String> {
    let defaults = ProfileFunction::default();
    let rotation_limits = match e.child("RotationLimits").filter(|r| !r.nil) {
        Some(r) => FixtureRotationLimits {
            start_position: read_int(r, "StartPosition", 0)?,
            stop_position: read_int(r, "StopPosition", 360)?,
        },
        None => defaults.rotation_limits,
    };
    // `TimelineColor1Html`'s setter turns an empty colour into White, for files from an old bug.
    let timeline_color = match optional_text(e, "TimelineColor1Html") {
        Some(t) if t.is_empty() => DotNetColor::WHITE,
        _ => read_color(e, "TimelineColor1Html", DotNetColor::WHITE)?,
    };
    Ok(ProfileFunction {
        name: optional_text(e, "Name"),
        function_type: read_member(
            e,
            "FunctionType",
            defaults.function_type,
            FixtureFunctionType::from_name,
            FixtureFunctionType::name,
        )?,
        identity: read_member(
            e,
            "FunctionIdentity",
            defaults.identity,
            FuncId::from_name,
            FuncId::name,
        )?,
        index_data: read_list(e, "IndexData", "FixtureIndex", read_index)?,
        color_wheel_data: read_list(e, "ColorWheelData", "FixtureColorWheel", read_wheel_entry)?,
        label: optional_text(e, "Label"),
        rotation_limits,
        zoom_type: read_member(e, "ZoomType", defaults.zoom_type, zoom_from_name, zoom_name)?,
        timeline_color,
        associated_function_name: optional_text(e, "AssociatedFunctionName"),
    })
}

/// A `FixtureSpecification` element, from a profile file or a property's module data. The constructor's defaults fill in what's missing, as they do under `XmlSerializer`.
pub fn read_profile(e: &XElement) -> Result<FixtureProfile, String> {
    let with_default = |local: &str, default: String| match e.child(local) {
        None => Some(default),
        Some(_) => optional_text(e, local),
    };
    Ok(FixtureProfile {
        name: optional_text(e, "Name"),
        manufacturer: optional_text(e, "Manufacturer"),
        created_by: with_default("CreatedBy", user_name()),
        revision: with_default("Revision", "1.0".to_string()),
        channels: read_list(e, "ChannelDefinitions", "FixtureChannel", read_channel)?,
        functions: read_list(e, "FunctionDefinitions", "FixtureFunction", read_function)?,
    })
}

// ---- Writing a profile: the file ------------------------------------------------------------------------------------

/// What `XmlSerializer` and an `XmlWriter` with tab indents make: base class members first, then declaration order, a null member left out, and BOM and CRLF as on Windows.
struct ProfileFile {
    out: String,
    depth: usize,
}

impl ProfileFile {
    fn new_line(&mut self) {
        self.out.push_str("\r\n");
        for _ in 0..self.depth {
            self.out.push('\t');
        }
    }

    fn open(&mut self, tag: &str) {
        self.new_line();
        self.out.push('<');
        self.out.push_str(tag);
        self.out.push('>');
        self.depth += 1;
    }

    fn close(&mut self, tag: &str) {
        self.depth -= 1;
        self.new_line();
        self.out.push_str("</");
        self.out.push_str(tag);
        self.out.push('>');
    }

    fn value(&mut self, tag: &str, text: &str) {
        self.new_line();
        self.out.push('<');
        self.out.push_str(tag);
        if text.is_empty() {
            self.out.push_str(" />");
            return;
        }
        self.out.push('>');
        self.out.push_str(
            &text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;"),
        );
        self.out.push_str("</");
        self.out.push_str(tag);
        self.out.push('>');
    }

    fn optional(&mut self, tag: &str, text: &Option<String>) {
        if let Some(t) = text {
            self.value(tag, t);
        }
    }

    fn list<T>(&mut self, tag: &str, items: &[T], mut write: impl FnMut(&mut ProfileFile, &T)) {
        if items.is_empty() {
            self.value(tag, "");
            return;
        }
        self.open(tag);
        for item in items {
            write(self, item);
        }
        self.close(tag);
    }
}

fn file_flag(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

/// `FixtureSpecificationManager.Save`'s serialization, with the whole file in the result.
pub fn profile_file_text(profile: &FixtureProfile) -> String {
    let mut w = ProfileFile {
        out: String::with_capacity(4096),
        depth: 0,
    };
    w.out
        .push_str("\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n");
    w.out.push_str("<FixtureSpecification xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" version=\"1\">");
    w.depth = 1;
    w.optional("Name", &profile.name);
    w.optional("Manufacturer", &profile.manufacturer);
    w.optional("CreatedBy", &profile.created_by);
    w.optional("Revision", &profile.revision);
    w.list("ChannelDefinitions", &profile.channels, |w, c| {
        w.open("FixtureChannel");
        w.optional("Name", &c.name);
        w.optional("Function", &c.function);
        w.value("ChannelNumber", &c.channel_number.to_string());
        w.close("FixtureChannel");
    });
    w.list("FunctionDefinitions", &profile.functions, |w, f| {
        w.open("FixtureFunction");
        w.optional("Name", &f.name);
        w.value("FunctionType", f.function_type.name());
        w.value("FunctionIdentity", f.identity.name());
        w.list("IndexData", &f.index_data, |w, i| {
            w.open("FixtureIndex");
            w.optional("Name", &i.name);
            w.value("StartValue", &i.start_value.to_string());
            w.value("EndValue", &i.end_value.to_string());
            w.value("UseCurve", file_flag(i.use_curve));
            w.value("IndexType", i.index_type.name());
            w.optional("Image", &i.image);
            w.close("FixtureIndex");
        });
        w.list("ColorWheelData", &f.color_wheel_data, |w, e| {
            w.open("FixtureColorWheel");
            w.optional("Name", &e.name);
            w.value("StartValue", &e.start_value.to_string());
            w.value("EndValue", &e.end_value.to_string());
            w.value("UseCurve", file_flag(e.use_curve));
            w.value("IndexType", FixtureIndexType::ColorWheel.name());
            w.value("HalfStep", file_flag(e.half_step));
            w.value("Color1Html", &e.color1.to_html());
            w.value("Color2Html", &e.color2.to_html());
            w.value("ExcludeColorProperty", file_flag(e.exclude_color_property));
            w.close("FixtureColorWheel");
        });
        w.optional("Label", &f.label);
        w.open("RotationLimits");
        w.value(
            "StartPosition",
            &f.rotation_limits.start_position.to_string(),
        );
        w.value("StopPosition", &f.rotation_limits.stop_position.to_string());
        w.close("RotationLimits");
        w.value("ZoomType", zoom_name(f.zoom_type));
        w.value("TimelineColor1Html", &f.timeline_color.to_html());
        w.optional("AssociatedFunctionName", &f.associated_function_name);
        w.close("FixtureFunction");
    });
    w.depth = 0;
    w.new_line();
    w.out.push_str("</FixtureSpecification>");
    w.out
}

// ---- Writing a profile: module data ---------------------------------------------------------------------------------

const PROPERTY_NS: &str = "VixenModules.Property.IntelligentFixture";
const FIXTURE_NS: &str = "VixenModules.App.Fixture";

fn contract_text(ns: &str, local: &str, text: &Option<String>) -> XElement {
    match text {
        Some(t) => contract_value(ns, local, t),
        None => XElement::nil(ns, local),
    }
}

fn contract_value(ns: &str, local: &str, text: &str) -> XElement {
    if text.is_empty() {
        XElement::new(ns, local)
    } else {
        XElement::with_text(ns, local, text)
    }
}

/// `FixtureSpecification` as `DataContractSerializer` lays it out: the base class's members first, then ordinal order.
fn specification_element(ns: &str, profile: &FixtureProfile) -> XElement {
    let a = contract_ns(FIXTURE_NS);
    let a = a.as_str();
    let mut e = XElement::new(ns, "FixtureSpecification");
    let mut channels = XElement::new(a, "ChannelDefinitions");
    for c in &profile.channels {
        let mut x = XElement::new(a, "FixtureChannel");
        x.push(contract_text(a, "Name", &c.name));
        x.push(XElement::with_text(
            a,
            "ChannelNumber",
            c.channel_number.to_string(),
        ));
        x.push(contract_text(a, "Function", &c.function));
        channels.push(x);
    }
    e.push(channels);
    e.push(contract_text(a, "CreatedBy", &profile.created_by));
    let mut functions = XElement::new(a, "FunctionDefinitions");
    for f in &profile.functions {
        let mut x = XElement::new(a, "FixtureFunction");
        x.push(contract_text(a, "Name", &f.name));
        x.push(contract_text(
            a,
            "AssociatedFunctionName",
            &f.associated_function_name,
        ));
        let mut wheel = XElement::new(a, "ColorWheelData");
        for w in &f.color_wheel_data {
            let mut y = XElement::new(a, "FixtureColorWheel");
            y.push(contract_text(a, "Name", &w.name));
            y.push(XElement::with_text(a, "EndValue", w.end_value.to_string()));
            y.push(XElement::with_text(
                a,
                "IndexType",
                FixtureIndexType::ColorWheel.name(),
            ));
            y.push(XElement::with_text(
                a,
                "StartValue",
                w.start_value.to_string(),
            ));
            y.push(XElement::with_text(a, "UseCurve", file_flag(w.use_curve)));
            y.push(contract_value(a, "Color1Html", &w.color1.to_html()));
            y.push(contract_value(a, "Color2Html", &w.color2.to_html()));
            y.push(XElement::with_text(
                a,
                "ExcludeColorProperty",
                file_flag(w.exclude_color_property),
            ));
            y.push(XElement::with_text(a, "HalfStep", file_flag(w.half_step)));
            wheel.push(y);
        }
        x.push(wheel);
        x.push(XElement::with_text(
            a,
            "FunctionIdentity",
            f.identity.name(),
        ));
        x.push(XElement::with_text(
            a,
            "FunctionType",
            f.function_type.name(),
        ));
        let mut index = XElement::new(a, "IndexData");
        for i in &f.index_data {
            let mut y = XElement::new(a, "FixtureIndex");
            y.push(contract_text(a, "Name", &i.name));
            y.push(XElement::with_text(a, "EndValue", i.end_value.to_string()));
            y.push(XElement::with_text(a, "IndexType", i.index_type.name()));
            y.push(XElement::with_text(
                a,
                "StartValue",
                i.start_value.to_string(),
            ));
            y.push(XElement::with_text(a, "UseCurve", file_flag(i.use_curve)));
            y.push(contract_text(a, "Image", &i.image));
            index.push(y);
        }
        x.push(index);
        x.push(contract_text(a, "Label", &f.label));
        let mut limits = XElement::new(a, "RotationLimits");
        limits.push(XElement::with_text(
            a,
            "StartPosition",
            f.rotation_limits.start_position.to_string(),
        ));
        limits.push(XElement::with_text(
            a,
            "StopPosition",
            f.rotation_limits.stop_position.to_string(),
        ));
        x.push(limits);
        x.push(contract_value(
            a,
            "TimelineColor1Html",
            &f.timeline_color.to_html(),
        ));
        x.push(XElement::with_text(a, "ZoomType", zoom_name(f.zoom_type)));
        functions.push(x);
    }
    e.push(functions);
    e.push(contract_text(a, "Manufacturer", &profile.manufacturer));
    e.push(contract_text(a, "Name", &profile.name));
    e.push(contract_text(a, "Revision", &profile.revision));
    e.push(XElement::with_text(a, "version", "1"));
    e
}

/// `IntelligentFixtureData` holding the profile.
pub(crate) fn fixture_property_data(
    profile: &FixtureProfile,
    type_id: &Uuid,
    instance_id: &Uuid,
) -> XElement {
    let ns = contract_ns(PROPERTY_NS);
    let mut e = XElement::new(&ns, "IntelligentFixtureData");
    e.push(XElement::with_text(
        "",
        "ModuleInstanceId",
        instance_id.to_string(),
    ));
    e.push(XElement::with_text("", "ModuleTypeId", type_id.to_string()));
    e.push(specification_element(&ns, profile));
    e
}

/// `new IntelligentFixtureData()` plus `SetDefaultValues`: the built-in functions and one default channel. What Add Property makes.
pub(crate) fn new_fixture_property_data(type_id: &Uuid, instance_id: &Uuid) -> XElement {
    let mut profile = FixtureProfile::new_blank();
    profile.channels.push(ProfileChannel {
        name: None,
        function: None,
        channel_number: 1,
    });
    fixture_property_data(&profile, type_id, instance_id)
}

// ---- The profile folder ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FixtureProfiles {
    /// Sorted by name.
    pub profiles: Vec<FixtureProfile>,
    /// A message for each file left out because its name was taken: `MessageBoxForm`, titled [`INVALID_PROFILE_TITLE`].
    pub warnings: Vec<String>,
}

pub const INVALID_PROFILE_TITLE: &str = "Invalid Fixture Profile";

fn sort_profiles(list: &mut [FixtureProfile]) {
    // `OrderBy(item => item.Name)`, culture-aware: without case first, and lower before upper to break ties.
    list.sort_by_cached_key(|p| {
        let name = p.name_text();
        (
            name.to_lowercase(),
            name.chars()
                .map(|c| if c.is_uppercase() { 'U' } else { 'L' })
                .collect::<String>(),
        )
    });
}

/// `LoadFixtureSpecifications`: every `*.xml` here, in name order so the same file wins each time. A malformed file is logged and left out. Makes the folders it expects.
pub fn load_fixture_profiles(dir: &Path) -> FixtureProfiles {
    let _ = std::fs::create_dir_all(dir.join("Images"));
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")))
        .collect();
    files.sort_by_cached_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    let mut result = FixtureProfiles::default();
    for path in files {
        let read = read_xml_file(&path)
            .map_err(|e| e.to_string())
            .and_then(|root| {
                if root.local == "FixtureSpecification" {
                    read_profile(&root)
                } else {
                    Err(format!("the root element is {}", root.local))
                }
            });
        match read {
            Ok(profile) if profile.name_text().is_empty() => logging::error(
                LOGGER,
                &format!("{}is malformed!", path.display()),
                Some("no Name"),
            ),
            Ok(profile) => {
                if result.profiles.iter().any(|p| p.name == profile.name) {
                    let file = path
                        .file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    result.warnings.push(format!("A Intelligent Fixture with a name of '{}' already exists.  The file: '{file}' was ignored.", profile.name_text()));
                } else {
                    result.profiles.push(profile);
                }
            }
            Err(e) => logging::error(
                LOGGER,
                &format!("{}is malformed!", path.display()),
                Some(&e),
            ),
        }
    }
    sort_profiles(&mut result.profiles);
    result
}

/// `IntelligentFixtureModule.ModuleData`'s setter: a named profile on a property joins the repository if it has no file, so it can be picked though it was never saved here.
pub fn add_profiles_in_tree(profiles: &mut FixtureProfiles, session: &SetupSession) {
    let mut added = false;
    for (_, node) in session.tree.live_nodes() {
        for property in node
            .properties
            .iter()
            .filter(|p| p.type_id == property_ids::INTELLIGENT_FIXTURE)
        {
            let PropertyData::Unknown { raw_xml } = &property.data else {
                continue;
            };
            let Some(data) = parse_fragment(raw_xml) else {
                continue;
            };
            let Some(Ok(profile)) = data.child("FixtureSpecification").map(read_profile) else {
                continue;
            };
            if !profile.name_text().is_empty()
                && !profiles.profiles.iter().any(|p| p.name == profile.name)
            {
                profiles.profiles.push(profile);
                added = true;
            }
        }
    }
    if added {
        sort_profiles(&mut profiles.profiles);
    }
}

/// `FixtureSpecificationManager.Save`: under an existing name, this overwrites that profile. Display Setup's Cancel doesn't undo it (F110).
pub fn save_fixture_profile(dir: &Path, profile: &FixtureProfile) -> Result<(), String> {
    write_atomic(
        &dir.join(profile.file_name()),
        profile_file_text(profile).as_bytes(),
    )
    .map_err(|e| e.to_string())
}

// ---- The wizard's choices -------------------------------------------------------------------------------------------

/// F143.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum ColorSupport {
    #[default]
    None,
    Mixing,
    Wheel,
}

/// `DimmingCurveSelection`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum DimmingCurveSelection {
    #[default]
    NoDimmingCurve,
    OneDimmingCurvePerFixture,
    OneDimmingCurvePerColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixtureWizardChoices {
    pub profile: FixtureProfile,
    /// `SelectedFixture`, the summary's line for the profile page.
    pub selected_fixture: String,
    pub color_support: ColorSupport,
    pub auto_shutter: bool,
    pub auto_color_wheel: bool,
    pub auto_dimmer: bool,
    pub auto_prism: bool,
    pub dimming: DimmingCurveSelection,
    pub curve: CurveData,
    pub count: u32,
    pub prefix: String,
    pub create_group: bool,
    pub group_name: String,
}

/// F143: mixing, else wheel, else none.
pub fn default_color_support(profile: &FixtureProfile) -> ColorSupport {
    if profile.is_color_mixing() {
        ColorSupport::Mixing
    } else if profile.contains_color_wheel() {
        ColorSupport::Wheel
    } else {
        ColorSupport::None
    }
}

/// One flag per box of the Automation page.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct AutomationBoxes {
    pub shutter: bool,
    pub color_wheel: bool,
    pub dimmer: bool,
    pub prism: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct AutomationState {
    pub enabled: AutomationBoxes,
    pub checked: AutomationBoxes,
}

/// F144. A box is on by default exactly when it's enabled.
pub fn automation_defaults(profile: &FixtureProfile, support: ColorSupport) -> AutomationState {
    let enabled = AutomationBoxes {
        shutter: support != ColorSupport::None && profile.supports_function(FuncId::Shutter),
        color_wheel: support == ColorSupport::Wheel,
        dimmer: profile.supports_function(FuncId::Dim),
        prism: profile.supports_function(FuncId::OpenClosePrism),
    };
    AutomationState {
        enabled,
        checked: enabled,
    }
}

/// F146: with a group, its name first. Then the fixtures.
pub fn wizard_preview_names(prefix: &str, count: u32, group: Option<&str>) -> Vec<String> {
    let mut names = Vec::with_capacity(count as usize + 1);
    names.extend(group.map(str::to_string));
    names.extend((1..=count).map(|i| format!("{prefix}{i}")));
    names
}

/// F147: `(page title, summary text)` for each page that has one.
pub fn wizard_summary(choices: &FixtureWizardChoices) -> Vec<(String, String)> {
    let color = match choices.color_support {
        ColorSupport::None => "No Color Support",
        ColorSupport::Mixing => "Color Mixing",
        ColorSupport::Wheel => "Color Wheel",
    };
    // The Dimmer item has no space after its comma: "Shutter, Color Wheel, Dimmer,Prism".
    let mut automation = String::new();
    if choices.auto_shutter {
        automation.push_str("Shutter, ");
    }
    if choices.auto_color_wheel {
        automation.push_str("Color Wheel, ");
    }
    if choices.auto_dimmer {
        automation.push_str("Dimmer,");
    }
    if choices.auto_prism {
        automation.push_str("Prism, ");
    }
    let automation = automation.trim_end_matches([',', ' ']).to_string();
    let dimming = match choices.dimming {
        DimmingCurveSelection::NoDimmingCurve => "No Dimming Selected",
        DimmingCurveSelection::OneDimmingCurvePerFixture => "Dimming Curve for Fixture",
        DimmingCurveSelection::OneDimmingCurvePerColor => "Dimming Curve for Each Color Channel",
    };
    let mut grouping = format!("# Fixtures: {}\n", choices.count);
    if choices.create_group {
        grouping.push_str(&format!("Group Name: {}\n", choices.group_name));
    }
    grouping.push_str(&format!("Element Prefix: {}", choices.prefix));
    vec![
        (
            WizardPage::SelectProfile.title().to_string(),
            choices.selected_fixture.clone(),
        ),
        (
            WizardPage::ColorSupport.title().to_string(),
            color.to_string(),
        ),
        (WizardPage::Automation.title().to_string(), automation),
        (
            WizardPage::DimmingCurves.title().to_string(),
            dimming.to_string(),
        ),
        (WizardPage::Grouping.title().to_string(), grouping),
    ]
}

// ---- The wizard -----------------------------------------------------------------------------------------------------

const HELP_BASE: &str = "http://www.vixenlights.com/vixen-3-documentation/setup-configuration/setup-display-elements/intelligent-fixture-wizard/";

pub const PROFILE_NAME_PROBLEM: &str = "Profile Name is empty.  Profile Name is a required field.";

/// F139, in order.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum WizardPage {
    SelectProfile,
    EditFunctions,
    EditChannels,
    ColorSupport,
    Automation,
    DimmingCurves,
    Grouping,
    Summary,
}

impl WizardPage {
    pub const ALL: [WizardPage; 8] = [
        WizardPage::SelectProfile,
        WizardPage::EditFunctions,
        WizardPage::EditChannels,
        WizardPage::ColorSupport,
        WizardPage::Automation,
        WizardPage::DimmingCurves,
        WizardPage::Grouping,
        WizardPage::Summary,
    ];

    pub fn title(self) -> &'static str {
        match self {
            WizardPage::SelectProfile => "Select Profile",
            WizardPage::EditFunctions => "Edit Functions",
            WizardPage::EditChannels => "Edit Channels",
            WizardPage::ColorSupport => "Color Support",
            WizardPage::Automation => "Automation",
            WizardPage::DimmingCurves => "Dimming Curves",
            WizardPage::Grouping => "Grouping",
            WizardPage::Summary => "Summary",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            WizardPage::SelectProfile => "Select or create the profile for the fixture(s).",
            WizardPage::EditFunctions => "Define the fixture functions.",
            WizardPage::EditChannels => "Define the fixture's channels.",
            WizardPage::ColorSupport => {
                "Select the color support applicable to the fixture.  Color Mixing should be selected when the fixture uses a multiple color light source or multiple light sources of different colors.  Color wheel should be selected when the fixture has a single white light source and is equipped with a color wheel to change colors.  "
            }
            WizardPage::Automation => "These options configure special filters that make it easier to control the fixtures when sequencing.",
            WizardPage::DimmingCurves => "These options configure if any dimming curves are applied to the fixture outputs.",
            WizardPage::Grouping => "Configure how many fixtures to create and if they should be grouped.",
            WizardPage::Summary => "Below is a summary of the fixture selections.",
        }
    }

    pub fn help_url(self) -> String {
        let leaf = match self {
            WizardPage::SelectProfile => "select-profile/",
            WizardPage::EditFunctions => "edit-functions/",
            WizardPage::EditChannels => "edit-channels/",
            WizardPage::ColorSupport => "color-support/",
            WizardPage::Automation => "automation/",
            WizardPage::DimmingCurves => "dimming-curves/",
            WizardPage::Grouping => "grouping/",
            WizardPage::Summary => "summary/",
        };
        format!("{HELP_BASE}{leaf}")
    }

    fn position(self) -> usize {
        WizardPage::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }
}

/// The wizard while it's open: the page models, with each view model's constructor run each time its page opens, because views aren't cached (F139).
#[derive(Clone, Debug)]
pub struct FixtureWizard {
    page: WizardPage,
    visited: HashSet<WizardPage>,
    profiles: Vec<FixtureProfile>,
    pub existing_enabled: bool,
    pub select_existing: bool,
    pub selected_fixture: Option<String>,
    pub profile: FixtureProfile,
    pub color_support: ColorSupport,
    pub automation_enabled: AutomationBoxes,
    pub automation: AutomationBoxes,
    pub dimming: DimmingCurveSelection,
    pub per_color_enabled: bool,
    pub curve: CurveData,
    pub count: u32,
    pub prefix: String,
    pub create_group: bool,
    pub group_name: String,
    pub editor_errors: Vec<String>,
    default_profile: Option<FixtureProfile>,
    default_support: Option<ColorSupport>,
    automation_profile: Option<FixtureProfile>,
}

impl FixtureWizard {
    /// The window opening on Select Profile. `profiles` is the sorted repository.
    pub fn new(profiles: Vec<FixtureProfile>) -> FixtureWizard {
        let mut wizard = FixtureWizard {
            page: WizardPage::SelectProfile,
            visited: HashSet::new(),
            profiles,
            existing_enabled: false,
            // The page model's constructor.
            select_existing: true,
            selected_fixture: None,
            profile: FixtureProfile::new_blank(),
            color_support: ColorSupport::None,
            automation_enabled: AutomationBoxes::default(),
            automation: AutomationBoxes::default(),
            dimming: DimmingCurveSelection::NoDimmingCurve,
            per_color_enabled: false,
            curve: CurveData::default(),
            count: 0,
            prefix: String::new(),
            create_group: false,
            group_name: String::new(),
            editor_errors: Vec::new(),
            default_profile: None,
            default_support: None,
            automation_profile: None,
        };
        wizard.enter();
        wizard
    }

    pub fn page(&self) -> WizardPage {
        self.page
    }

    pub fn has_visited(&self, page: WizardPage) -> bool {
        self.visited.contains(&page)
    }

    pub fn profile_names(&self) -> Vec<String> {
        self.profiles
            .iter()
            .map(|p| p.name_text().to_string())
            .collect()
    }

    /// What the current page's view model reports as a validation error. It blocks moving on.
    pub fn problem(&self) -> Option<&str> {
        if self.page == WizardPage::SelectProfile && self.profile.name_text().is_empty() {
            return Some(PROFILE_NAME_PROBLEM);
        }
        if matches!(
            self.page,
            WizardPage::EditFunctions | WizardPage::EditChannels
        ) {
            return self.editor_errors.first().map(String::as_str);
        }
        None
    }

    pub fn can_go_back(&self) -> bool {
        self.page != WizardPage::SelectProfile && self.problem().is_none()
    }

    pub fn can_go_next(&self) -> bool {
        self.page != WizardPage::Summary && self.problem().is_none()
    }

    /// Finish only on Summary: whether Orc allows it earlier couldn't be read (F139).
    pub fn can_finish(&self) -> bool {
        self.page == WizardPage::Summary
    }

    /// Quick navigation: any page already visited.
    pub fn can_go_to(&self, page: WizardPage) -> bool {
        self.has_visited(page) && self.problem().is_none()
    }

    pub fn go_next(&mut self) {
        if self.can_go_next() {
            self.move_to(WizardPage::ALL[self.page.position() + 1]);
        }
    }

    pub fn go_back(&mut self) {
        if self.can_go_back() {
            self.move_to(WizardPage::ALL[self.page.position() - 1]);
        }
    }

    pub fn go_to(&mut self, page: WizardPage) {
        if page != self.page && self.can_go_to(page) {
            self.move_to(page);
        }
    }

    fn move_to(&mut self, page: WizardPage) {
        // Edit Channels' SaveAsync: the shared editor writes the validated profile before navigation.
        if self.page == WizardPage::EditChannels {
            self.selected_fixture.clone_from(&self.profile.name);
        }
        self.page = page;
        self.enter();
    }

    fn enter(&mut self) {
        let first_visit = self.visited.insert(self.page);
        let profile_changed = self.default_profile.as_ref() != Some(&self.profile);
        match self.page {
            WizardPage::SelectProfile => {
                // The radio goes off on every visit and only comes back while Select Existing is still chosen, so having picked Create New and
                // left the page it stays off, as upstream.
                self.existing_enabled = self.select_existing && !self.profiles.is_empty();
                if self.existing_enabled {
                    if self.selected_fixture.is_none() {
                        self.select_first();
                    }
                } else if self.select_existing {
                    self.select_existing = false;
                    self.profile = FixtureProfile::new_blank();
                }
            }
            WizardPage::ColorSupport => {
                if first_visit || profile_changed {
                    self.color_support = default_color_support(&self.profile);
                }
                self.default_profile = Some(self.profile.clone());
            }
            WizardPage::Automation => {
                // F144: a disabled box is false here, where upstream leaves it as it was.
                let state = automation_defaults(&self.profile, self.color_support);
                self.automation_enabled = state.enabled;
                if first_visit
                    || self.automation_profile.as_ref() != Some(&self.profile)
                    || self.default_support != Some(self.color_support)
                {
                    self.automation = state.checked;
                }
                self.default_support = Some(self.color_support);
                self.automation_profile = Some(self.profile.clone());
            }
            WizardPage::DimmingCurves => {
                self.per_color_enabled = self.color_support == ColorSupport::Mixing;
                if !self.per_color_enabled
                    && self.dimming == DimmingCurveSelection::OneDimmingCurvePerColor
                {
                    self.dimming = DimmingCurveSelection::NoDimmingCurve;
                }
            }
            WizardPage::Grouping if first_visit => {
                self.count = 1;
                self.prefix = self.profile.name_text().to_string();
                self.create_group = true;
                self.group_name = "Fixtures".to_string();
            }
            WizardPage::EditFunctions
            | WizardPage::EditChannels
            | WizardPage::Summary
            | WizardPage::Grouping => {}
        }
    }

    fn select_first(&mut self) {
        if let Some(first) = self.profiles.first() {
            self.selected_fixture = first.name.clone();
            self.profile = first.clone();
        }
    }

    /// The Select Existing Profile radio. Going back to it picks the first profile again, as nothing is selected after Create New.
    pub fn choose_existing(&mut self) {
        if self.existing_enabled && !self.select_existing {
            self.select_existing = true;
            if self.selected_fixture.is_none() {
                self.select_first();
            }
        }
    }

    /// The Create New Profile radio: a blank profile with the built-in functions. A click on it when it's on already changes nothing.
    pub fn choose_create_new(&mut self) {
        if self.select_existing {
            self.select_existing = false;
            self.selected_fixture = None;
            self.profile = FixtureProfile::new_blank();
        }
    }

    /// The combo: a fresh copy of that profile, whatever was typed over the last one. Picking the name already showing changes nothing.
    pub fn select_profile(&mut self, name: &str) {
        if !self.select_existing || self.selected_fixture.as_deref() == Some(name) {
            return;
        }
        if let Some(found) = self.profiles.iter().find(|p| p.name_text() == name) {
            self.selected_fixture = found.name.clone();
            self.profile = found.clone();
        }
    }

    pub fn set_auto_shutter(&mut self, on: bool) {
        self.automation.shutter = on && self.automation_enabled.shutter;
    }

    pub fn set_auto_color_wheel(&mut self, on: bool) {
        self.automation.color_wheel = on && self.automation_enabled.color_wheel;
    }

    pub fn set_auto_dimmer(&mut self, on: bool) {
        self.automation.dimmer = on && self.automation_enabled.dimmer;
    }

    pub fn set_auto_prism(&mut self, on: bool) {
        self.automation.prism = on && self.automation_enabled.prism;
    }

    pub fn set_dimming(&mut self, selection: DimmingCurveSelection) {
        if selection != DimmingCurveSelection::OneDimmingCurvePerColor || self.per_color_enabled {
            self.dimming = selection;
        }
    }

    /// `Add`. No upper bound, as upstream.
    pub fn add_fixture(&mut self) {
        self.count += 1;
    }

    /// `Subtract`, whose `CanSubtract` keeps at least one.
    pub fn remove_fixture(&mut self) {
        if self.count > 1 {
            self.count -= 1;
        }
    }

    pub fn preview_names(&self) -> Vec<String> {
        wizard_preview_names(
            &self.prefix,
            self.count,
            self.create_group.then_some(self.group_name.as_str()),
        )
    }

    pub fn choices(&self) -> FixtureWizardChoices {
        // F144 again: a box the current selections would disable is false, even when its page wasn't revisited after they changed.
        let enabled = automation_defaults(&self.profile, self.color_support).enabled;
        FixtureWizardChoices {
            profile: self.profile.clone(),
            selected_fixture: self.selected_fixture.clone().unwrap_or_default(),
            color_support: self.color_support,
            auto_shutter: self.automation.shutter && enabled.shutter,
            auto_color_wheel: self.automation.color_wheel && enabled.color_wheel,
            auto_dimmer: self.automation.dimmer && enabled.dimmer,
            auto_prism: self.automation.prism && enabled.prism,
            dimming: self.dimming,
            curve: self.curve.clone(),
            count: self.count,
            prefix: self.prefix.clone(),
            create_group: self.create_group,
            group_name: self.group_name.clone(),
        }
    }
}

// ---- Generation -----------------------------------------------------------------------------------------------------

/// What one generation made. The group's children aren't also root rows, so there's nothing for a later step to delete (F110's bug has no place to live).
#[derive(Clone, Debug, Default)]
pub struct FixtureResult {
    /// The group, or the first fixture.
    pub head: Option<Nid>,
    pub fixtures: Vec<Nid>,
    /// The new rows, with the group expanded and its first fixture selected, or the first fixture selected.
    pub edit: EditResult,
}

/// What the C# would throw on halfway through, or the wizard's validation stops first. Run before anything is saved or created.
pub fn check_fixture_choices(choices: &FixtureWizardChoices) -> Result<(), String> {
    if choices.profile.name_text().is_empty() {
        return Err(PROFILE_NAME_PROBLEM.to_string());
    }
    for channel in &choices.profile.channels {
        if choices.profile.function_of(channel).is_none() {
            return Err(format!(
                "The channel \"{}\" uses the function \"{}\", which the profile doesn't define.",
                channel.name.as_deref().unwrap_or(""),
                channel.function.as_deref().unwrap_or("")
            ));
        }
    }
    Ok(())
}

/// F110's generation, in channel order. The profile's file is saved separately, by [`save_fixture_profile`].
pub fn create_fixture_elements(
    session: &mut SetupSession,
    choices: &FixtureWizardChoices,
) -> Result<FixtureResult, String> {
    check_fixture_choices(choices)?;
    let mut fixtures = Vec::with_capacity(choices.count as usize);
    for index in 0..choices.count {
        fixtures.push(create_fixture_node(
            session,
            choices,
            &format!("{}{}", choices.prefix, index + 1),
        )?);
    }
    let mut result = FixtureResult {
        head: fixtures.first().copied(),
        ..FixtureResult::default()
    };
    let mut edit = EditResult {
        reload_roots: true,
        scroll_to_top: true,
        ..EditResult::default()
    };
    if choices.create_group {
        // After the fixtures, because this name is made unique against theirs.
        let (group, effects) =
            session
                .tree
                .create_node(None, Some(&choices.group_name), false, true);
        session.apply_effects(effects);
        for fixture in &fixtures {
            let effects = session.tree.move_node(*fixture, Some(group), None, None);
            session.apply_effects(effects);
        }
        result.head = Some(group);
        let group_row = session.row_path_of(group);
        edit.select = fixtures
            .first()
            .map(|first| vec![child_path(Some(&group_row), *first)]);
        edit.expand.push(group_row);
    } else {
        edit.select = fixtures
            .first()
            .map(|first| vec![session.row_path_of(*first)]);
    }
    session.tree_changed();
    session.patching_changed();
    session.bump_graph();
    result.fixtures = fixtures;
    result.edit = edit;
    Ok(result)
}

/// `CreateFixtureNode`.
fn create_fixture_node(
    session: &mut SetupSession,
    choices: &FixtureWizardChoices,
    name: &str,
) -> Result<Nid, String> {
    let (node, effects) = session.tree.create_node(None, Some(name), true, true);
    session.apply_effects(effects);
    let element = session
        .tree
        .node(node)
        .element_id
        .and_then(|e| session.flow.find(&e))
        .ok_or("The new element has no data flow component.")?;

    let type_id = property_ids::INTELLIGENT_FIXTURE;
    let instance_id = Uuid::new_v4();
    let data = fixture_property_data(&choices.profile, &type_id, &instance_id);
    let raw_xml = write_fragment(&data);
    session.system.module_store.set_instance(ModuleEntry::new(
        "VixenModules.Property.IntelligentFixture.IntelligentFixtureData, IntelligentFixture",
        &type_id,
        &instance_id,
        "Module.Property.IntelligentFixture",
        data,
    ));
    session
        .tree
        .node_mut(node)
        .properties
        .push(PropertyInstance {
            type_id,
            instance_id,
            data: PropertyData::Unknown { raw_xml },
        });

    let mut builder = FixtureBuilder {
        session,
        choices,
        node,
        element,
        color_mixing: choices.color_support == ColorSupport::Mixing,
        color_property_created: false,
        found_color_mixing_channel: false,
        color_mixing_channel_count: 0,
    };
    if builder.color_mixing {
        builder.add_full_color_property();
    }

    let channels = &choices.profile.channels;
    let mut last_function: Option<&str> = Some("");
    for (index, channel) in channels.iter().enumerate() {
        // `check_fixture_choices` has seen that every channel has its function.
        let Some(function) = choices.profile.function_of(channel) else {
            continue;
        };
        let next = channels
            .get(index + 1)
            .and_then(|c| choices.profile.function_of(c));
        // A run of channels on one function yields one filter, except that every "None" channel gets its own.
        let current = channel.function.as_deref();
        if current != last_function || current == Some(FixtureFunctionType::None.name()) {
            last_function = current;
            builder.process_function(function, next)?;
        }
    }
    Ok(node)
}

struct FixtureBuilder<'a> {
    session: &'a mut SetupSession,
    choices: &'a FixtureWizardChoices,
    node: Nid,
    element: Cid,
    color_mixing: bool,
    color_property_created: bool,
    found_color_mixing_channel: bool,
    color_mixing_channel_count: usize,
}

impl FixtureBuilder<'_> {
    /// `AddRGBColorProperty`.
    fn add_full_color_property(&mut self) {
        self.color_property_created = true;
        self.add_color_property(ElementColorType::FullColor, None);
    }

    /// `AddDiscreteColorProperty`. The set is named after the profile, and `SetColorSet` replaces one of that name.
    fn add_discrete_color_property(&mut self) {
        self.color_property_created = true;
        let profile = &self.choices.profile;
        self.session.system.tree.color_sets.insert(
            profile.name_text().to_string(),
            profile.color_wheel_colors(),
        );
        self.add_color_property(
            ElementColorType::MultipleDiscreteColors,
            profile.name.clone(),
        );
    }

    fn add_color_property(&mut self, color_type: ElementColorType, color_set_name: Option<String>) {
        self.session
            .tree
            .node_mut(self.node)
            .properties
            .push(PropertyInstance {
                type_id: property_ids::COLOR,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Color {
                    color_type,
                    single_color: Color::EMPTY,
                    color_set_name,
                },
            });
    }

    fn add_filter(&mut self, config: FilterConfig) -> Result<Cid, String> {
        let type_id = config.ids().type_id;
        self.session
            .add_filter_instance(type_id, Some(config))
            .ok_or_else(|| "Cannot create a filter of that type.".to_string())
    }

    /// `AddFilterToNode`.
    fn add_to_element(&mut self, config: FilterConfig) -> Result<Cid, String> {
        let filter = self.add_filter(config)?;
        self.session.flow.set_source(
            filter,
            SourceRef {
                component: self.element,
                output: 0,
            },
        );
        Ok(filter)
    }

    /// `AddFilterToNodesLastFilter`.
    fn add_to_last_leaf(&mut self, config: FilterConfig) -> Result<Cid, String> {
        let filter = self.add_filter(config)?;
        if let Some(last) = self.leaves().last() {
            self.session.flow.set_source(filter, *last);
        }
        Ok(filter)
    }

    /// `FindLeafOutputsOrBreakdownFilters` from the element.
    fn leaves(&self) -> Vec<SourceRef> {
        let mut out = Vec::new();
        self.collect_leaves(self.element, &mut out);
        out
    }

    fn collect_leaves(&self, component: Cid, out: &mut Vec<SourceRef>) {
        if self
            .session
            .filter_for(component)
            .is_some_and(|f| f.type_id == vixen_output::ids::COLOR_BREAKDOWN)
        {
            out.push(SourceRef {
                component,
                output: -1,
            });
        }
        let c = self.session.flow.comp(component);
        if c.kind == CompKind::Output {
            return;
        }
        for output in 0..c.output_count as i32 {
            let mut any = false;
            for child in self.session.flow.destinations_of_output(component, output) {
                any = true;
                self.collect_leaves(child, out);
            }
            if !any {
                out.push(SourceRef { component, output });
            }
        }
    }

    fn dimming_curve(&self) -> FilterConfig {
        FilterConfig::DimmingCurve(DimmingCurveConfig {
            curve: self.choices.curve.clone(),
            ..DimmingCurveConfig::default()
        })
    }

    fn coarse_fine() -> FilterConfig {
        FilterConfig::CoarseFine(CoarseFineConfig::default())
    }

    /// `ProcessFunction`.
    fn process_function(
        &mut self,
        function: &ProfileFunction,
        next: Option<&ProfileFunction>,
    ) -> Result<(), String> {
        match function.function_type {
            FixtureFunctionType::ColorWheel if self.choices.auto_color_wheel => {
                self.process_color_wheel(function)?
            }
            FixtureFunctionType::ColorWheel | FixtureFunctionType::Indexed => {
                self.process_indexed(function)?
            }
            FixtureFunctionType::RgbwColor => self.process_color_mixing("RGBW", 4)?,
            FixtureFunctionType::RgbColor => self.process_color_mixing("RGB", 3)?,
            FixtureFunctionType::Range => {
                if function.identity == FuncId::Dim && self.choices.auto_dimmer {
                    self.process_dimmer(function)?;
                } else {
                    self.process_tagged(function)?;
                }
            }
            FixtureFunctionType::None => {
                self.add_to_element(FilterConfig::Tagged(TaggedFilterConfig {
                    tag: Some("Ignore".to_string()),
                    ..TaggedFilterConfig::default()
                }))?;
            }
        }
        // Two channels of a range function: how a 16-bit range gets its two outputs.
        if function.function_type == FixtureFunctionType::Range
            && next.is_some_and(|n| {
                n.function_type == FixtureFunctionType::Range && n.name == function.name
            })
        {
            self.add_to_last_leaf(Self::coarse_fine())?;
        }
        Ok(())
    }

    fn process_tagged(&mut self, function: &ProfileFunction) -> Result<(), String> {
        self.add_to_element(FilterConfig::Tagged(TaggedFilterConfig {
            tag: function.name.clone(),
            ..TaggedFilterConfig::default()
        }))?;
        Ok(())
    }

    /// `ProcessIndexedFunction`. The auto prism option is passed upstream but never read, so an Open Close Prism function always gets its filter.
    fn process_indexed(&mut self, function: &ProfileFunction) -> Result<(), String> {
        if function.identity == FuncId::Shutter && self.choices.auto_shutter {
            self.process_shutter(function)
        } else if function.identity == FuncId::OpenClosePrism {
            self.process_prism(function)
        } else {
            self.process_tagged(function)
        }
    }

    /// `ProcessColorWheelFunction`.
    fn process_color_wheel(&mut self, function: &ProfileFunction) -> Result<(), String> {
        let entries = function
            .color_wheel_data
            .iter()
            .map(|e| FixtureColorWheelEntry {
                name: e.name.clone(),
                end_value: e.end_value,
                index_type: FixtureIndexType::ColorWheel.name().to_string(),
                start_value: e.start_value,
                use_curve: e.use_curve,
                color1_html: Some(e.color1.to_html()),
                color2_html: Some(e.color2.to_html()),
                exclude_color_property: e.exclude_color_property,
                half_step: e.half_step,
            })
            .collect();
        self.add_to_element(FilterConfig::ColorWheel(ColorWheelFilterConfig {
            tag: function.name.clone(),
            color_wheel_data: Some(entries),
            convert_color_intents_into_index_commands: !self.color_mixing,
            ..ColorWheelFilterConfig::default()
        }))?;
        if !self.color_property_created {
            self.add_discrete_color_property();
        }
        Ok(())
    }

    /// `ProcessDimmerFunction`.
    fn process_dimmer(&mut self, function: &ProfileFunction) -> Result<(), String> {
        self.add_to_element(FilterConfig::Dimming(DimmingFilterConfig {
            tag: function.name.clone(),
            convert_rgb_into_dimming_intents: !self.color_mixing,
            ..DimmingFilterConfig::default()
        }))?;
        if self.choices.dimming != DimmingCurveSelection::NoDimmingCurve && !self.color_mixing {
            self.add_to_last_leaf(self.dimming_curve())?;
        }
        Ok(())
    }

    /// `ProcessShutterFunction`: the first entry of each type with a nonzero start.
    fn process_shutter(&mut self, function: &ProfileFunction) -> Result<(), String> {
        let (open, close) = first_index_values(
            function,
            FixtureIndexType::ShutterOpen,
            FixtureIndexType::ShutterClosed,
        );
        self.add_to_element(FilterConfig::Shutter(ShutterFilterConfig {
            tag: function.name.clone(),
            convert_color_into_shutter_intents: true,
            open_shutter_index_value: open,
            close_shutter_index_value: close,
            ..ShutterFilterConfig::default()
        }))?;
        Ok(())
    }

    /// `ProcessOpenClosePrismFunction`.
    fn process_prism(&mut self, function: &ProfileFunction) -> Result<(), String> {
        let (open, close) = first_index_values(
            function,
            FixtureIndexType::PrismOpen,
            FixtureIndexType::PrismClose,
        );
        self.add_to_element(FilterConfig::Prism(PrismFilterConfig {
            tag: function.name.clone(),
            convert_prism_intents_into_open_prism_intents: true,
            open_prism_index_value: open,
            close_prism_index_value: close,
            associated_function_name: function.associated_function_name.clone(),
            ..PrismFilterConfig::default()
        }))?;
        Ok(())
    }

    /// `ProcessColorMixingFunction`. Colour channels on one function never reach the counting branch, because only the first of a run gets here.
    fn process_color_mixing(
        &mut self,
        template: &str,
        expected_colors: usize,
    ) -> Result<(), String> {
        let color_channels = self.choices.profile.color_channel_count();
        if self.found_color_mixing_channel {
            self.color_mixing_channel_count += 1;
            if self.color_mixing_channel_count == color_channels {
                self.color_mixing_channel_count = 0;
                self.found_color_mixing_channel = false;
            }
            return Ok(());
        }
        let selection = self.choices.dimming;
        if selection == DimmingCurveSelection::OneDimmingCurvePerFixture {
            self.add_to_element(self.dimming_curve())?;
        }
        self.found_color_mixing_channel = true;
        self.color_mixing_channel_count += 1;

        let sixteen_bit = color_channels > 4;
        let breakdown = FilterConfig::ColorBreakdown(ColorBreakdownConfig {
            breakdown_items: ColorBreakdownConfig::template(template).unwrap_or_default(),
            mix_colors: true,
            sixteen_bit,
            ..ColorBreakdownConfig::default()
        });
        if selection == DimmingCurveSelection::OneDimmingCurvePerFixture {
            self.add_to_last_leaf(breakdown)?;
        } else {
            self.add_to_element(breakdown)?;
        }

        // The breakdown's last output is the last leaf now.
        let Some(flow) = self.leaves().last().map(|l| l.component) else {
            return Ok(());
        };
        if selection == DimmingCurveSelection::OneDimmingCurvePerColor {
            for output in 0..expected_colors as i32 {
                let curve = self.add_filter(self.dimming_curve())?;
                self.session.flow.set_source(
                    curve,
                    SourceRef {
                        component: flow,
                        output,
                    },
                );
            }
            if sixteen_bit {
                // From the end of the leaves, so these are made last colour first.
                let updated = self.leaves();
                for dimming_flow in updated.iter().rev().take(expected_colors) {
                    let coarse_fine = self.add_filter(Self::coarse_fine())?;
                    self.session.flow.set_source(
                        coarse_fine,
                        SourceRef {
                            component: dimming_flow.component,
                            output: 0,
                        },
                    );
                }
            }
        } else if sixteen_bit {
            for output in 0..expected_colors as i32 {
                let coarse_fine = self.add_filter(Self::coarse_fine())?;
                self.session.flow.set_source(
                    coarse_fine,
                    SourceRef {
                        component: flow,
                        output,
                    },
                );
            }
        }
        Ok(())
    }
}

/// The DMX value of the first entry of each type with a nonzero start. `(byte)StartValue` wraps.
fn first_index_values(
    function: &ProfileFunction,
    open: FixtureIndexType,
    close: FixtureIndexType,
) -> (u8, u8) {
    let (mut open_value, mut close_value) = (0u8, 0u8);
    for index in &function.index_data {
        if index.index_type == open && open_value == 0 {
            open_value = index.start_value as u8;
        }
        if index.index_type == close && close_value == 0 {
            close_value = index.start_value as u8;
        }
    }
    (open_value, close_value)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use vixen_io::load_system;
    use vixen_output::ids;

    fn function(
        name: &str,
        function_type: FixtureFunctionType,
        identity: FuncId,
    ) -> ProfileFunction {
        ProfileFunction {
            name: Some(name.to_string()),
            function_type,
            identity,
            ..ProfileFunction::default()
        }
    }

    fn channels(names: &[&str]) -> Vec<ProfileChannel> {
        names
            .iter()
            .enumerate()
            .map(|(i, f)| ProfileChannel {
                name: Some(format!("Ch {}", i + 1)),
                function: Some((*f).to_string()),
                channel_number: i as i32 + 1,
            })
            .collect()
    }

    fn index(name: &str, start: i32, index_type: FixtureIndexType) -> ProfileIndex {
        ProfileIndex {
            name: Some(name.to_string()),
            start_value: start,
            end_value: start,
            use_curve: false,
            index_type,
            image: None,
        }
    }

    fn wheel_entry(name: &str, start: i32, html: &str) -> ProfileWheelEntry {
        let color = DotNetColor::from_html(html).unwrap();
        ProfileWheelEntry {
            name: Some(name.to_string()),
            start_value: start,
            end_value: start,
            use_curve: false,
            half_step: false,
            color1: color.clone(),
            color2: color,
            exclude_color_property: false,
        }
    }

    /// A profile with these functions and these channels, in order.
    fn profile(functions: Vec<ProfileFunction>, on_channels: &[&str]) -> FixtureProfile {
        FixtureProfile {
            name: Some("Head".to_string()),
            manufacturer: None,
            created_by: Some("Tester".to_string()),
            revision: Some("1.0".to_string()),
            channels: channels(on_channels),
            functions,
        }
    }

    fn choices(profile: FixtureProfile) -> FixtureWizardChoices {
        FixtureWizardChoices {
            selected_fixture: profile.name_text().to_string(),
            color_support: default_color_support(&profile),
            profile,
            auto_shutter: false,
            auto_color_wheel: false,
            auto_dimmer: false,
            auto_prism: false,
            dimming: DimmingCurveSelection::NoDimmingCurve,
            curve: CurveData::default(),
            count: 1,
            prefix: "MH".to_string(),
            create_group: false,
            group_name: "Fixtures".to_string(),
        }
    }

    fn generate(c: &FixtureWizardChoices) -> (SetupSession, FixtureResult) {
        let mut s = session(&[], 0);
        let result = create_fixture_elements(&mut s, c).unwrap();
        (s, result)
    }

    fn component_of(s: &SetupSession, node: Nid) -> Cid {
        s.flow.find(&s.tree.node(node).element_id.unwrap()).unwrap()
    }

    fn config_of(s: &SetupSession, c: Cid) -> &FilterConfig {
        s.filter_for(c).unwrap().config.as_ref().unwrap()
    }

    /// What hangs off a component's output, in order.
    fn children(s: &SetupSession, c: Cid, output: i32) -> Vec<Cid> {
        s.flow.destinations_of_output(c, output).collect()
    }

    /// Each patch point's output name, in order: what Simple Patching wires to consecutive controller outputs.
    fn patch_names(s: &SetupSession, node: Nid) -> Vec<String> {
        s.flow
            .patch_points(component_of(s, node))
            .iter()
            .map(|p| match config_of(s, p.component) {
                FilterConfig::ColorBreakdown(_) | FilterConfig::Tagged(_) => {
                    config_of(s, p.component).output_names()[p.output as usize].clone()
                }
                other => other.display_name(),
            })
            .collect()
    }

    fn type_ids(s: &SetupSession) -> Vec<Uuid> {
        s.filters
            .iter()
            .filter(|f| f.alive)
            .map(|f| f.type_id)
            .collect()
    }

    fn rgb_functions() -> Vec<ProfileFunction> {
        vec![function(
            "Color",
            FixtureFunctionType::RgbColor,
            FuncId::Custom,
        )]
    }

    // ---- F143, F144, F146, F147 --------------------------------------------------------------------------------------

    #[test]
    fn default_color_support_prefers_mixing_then_wheel_and_only_counts_used_functions() {
        let all = vec![
            function("Color", FixtureFunctionType::RgbwColor, FuncId::Custom),
            function("Wheel", FixtureFunctionType::ColorWheel, FuncId::Custom),
            function("Pan", FixtureFunctionType::Range, FuncId::Pan),
        ];
        assert_eq!(
            default_color_support(&profile(all.clone(), &["Color", "Wheel"])),
            ColorSupport::Mixing
        );
        assert_eq!(
            default_color_support(&profile(all.clone(), &["Wheel", "Pan"])),
            ColorSupport::Wheel
        );
        assert_eq!(
            default_color_support(&profile(all, &["Pan"])),
            ColorSupport::None
        );
        assert_eq!(
            default_color_support(&FixtureProfile::new_blank()),
            ColorSupport::None
        );
    }

    #[test]
    fn automation_follows_the_profile_and_the_colour_support() {
        let all = vec![
            function("Shutter", FixtureFunctionType::Indexed, FuncId::Shutter),
            function("Dimmer", FixtureFunctionType::Range, FuncId::Dim),
            function(
                "Open Close Prism",
                FixtureFunctionType::Indexed,
                FuncId::OpenClosePrism,
            ),
        ];
        let p = profile(all.clone(), &["Shutter", "Dimmer", "Open Close Prism"]);
        let none = automation_defaults(&p, ColorSupport::None);
        assert_eq!(
            none.enabled,
            AutomationBoxes {
                shutter: false,
                color_wheel: false,
                dimmer: true,
                prism: true
            }
        );
        assert_eq!(none.checked, none.enabled);
        let wheel = automation_defaults(&p, ColorSupport::Wheel).enabled;
        assert_eq!(
            wheel,
            AutomationBoxes {
                shutter: true,
                color_wheel: true,
                dimmer: true,
                prism: true
            }
        );
        let mixing = automation_defaults(&p, ColorSupport::Mixing).enabled;
        assert!(mixing.shutter && !mixing.color_wheel);
        // Defined but on no channel.
        assert_eq!(
            automation_defaults(&profile(all, &[]), ColorSupport::Wheel).enabled,
            AutomationBoxes {
                color_wheel: true,
                ..AutomationBoxes::default()
            }
        );
    }

    #[test]
    fn the_first_function_with_an_identity_decides_support() {
        let p = profile(
            vec![
                function("Dim 1", FixtureFunctionType::Range, FuncId::Dim),
                function("Dim 2", FixtureFunctionType::Range, FuncId::Dim),
            ],
            &["Dim 2"],
        );
        assert!(!p.supports_function(FuncId::Dim));
    }

    #[test]
    fn preview_names_put_the_group_first() {
        assert_eq!(
            wizard_preview_names("MH", 2, Some("Fixtures")),
            ["Fixtures", "MH1", "MH2"]
        );
        assert_eq!(wizard_preview_names("MH", 2, None), ["MH1", "MH2"]);
    }

    #[test]
    fn the_summary_reads_as_upstream() {
        let mut c = choices(profile(rgb_functions(), &["Color"]));
        c.auto_shutter = true;
        c.auto_color_wheel = true;
        c.auto_dimmer = true;
        c.auto_prism = true;
        c.count = 3;
        c.create_group = true;
        let summary = wizard_summary(&c);
        let titles: Vec<&str> = summary.iter().map(|s| s.0.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Select Profile",
                "Color Support",
                "Automation",
                "Dimming Curves",
                "Grouping"
            ]
        );
        assert_eq!(summary[0].1, "Head");
        assert_eq!(summary[1].1, "Color Mixing");
        assert_eq!(summary[2].1, "Shutter, Color Wheel, Dimmer,Prism");
        assert_eq!(summary[3].1, "No Dimming Selected");
        assert_eq!(
            summary[4].1,
            "# Fixtures: 3\nGroup Name: Fixtures\nElement Prefix: MH"
        );

        c.auto_shutter = false;
        c.auto_color_wheel = false;
        c.auto_prism = false;
        c.create_group = false;
        c.color_support = ColorSupport::Wheel;
        c.dimming = DimmingCurveSelection::OneDimmingCurvePerColor;
        let summary = wizard_summary(&c);
        assert_eq!(summary[1].1, "Color Wheel");
        assert_eq!(summary[2].1, "Dimmer");
        assert_eq!(summary[3].1, "Dimming Curve for Each Color Channel");
        assert_eq!(summary[4].1, "# Fixtures: 3\nElement Prefix: MH");

        c.auto_dimmer = false;
        c.auto_prism = true;
        c.color_support = ColorSupport::None;
        c.dimming = DimmingCurveSelection::OneDimmingCurvePerFixture;
        let summary = wizard_summary(&c);
        assert_eq!(summary[1].1, "No Color Support");
        assert_eq!(summary[2].1, "Prism");
        assert_eq!(summary[3].1, "Dimming Curve for Fixture");
    }

    // ---- F139, F141 to F146: the pages ----------------------------------------------------------------------------------

    fn saved(name: &str) -> FixtureProfile {
        let mut p = profile(rgb_functions(), &["Color"]);
        p.name = Some(name.to_string());
        p.manufacturer = Some(format!("{name} Co"));
        p
    }

    fn run_to(w: &mut FixtureWizard, page: WizardPage) {
        while w.page() != page {
            w.go_next();
        }
    }

    #[test]
    fn pages_have_upstreams_order_titles_descriptions_and_help() {
        let titles: Vec<&str> = WizardPage::ALL.iter().map(|p| p.title()).collect();
        assert_eq!(
            titles,
            [
                "Select Profile",
                "Edit Functions",
                "Edit Channels",
                "Color Support",
                "Automation",
                "Dimming Curves",
                "Grouping",
                "Summary"
            ]
        );
        assert_eq!(
            WizardPage::Automation.help_url(),
            "http://www.vixenlights.com/vixen-3-documentation/setup-configuration/setup-display-elements/intelligent-fixture-wizard/automation/"
        );
        assert!(WizardPage::ColorSupport
            .description()
            .ends_with("to change colors.  "));
    }

    #[test]
    fn with_profiles_the_first_is_chosen_and_with_none_a_blank_one() {
        let w = FixtureWizard::new(vec![saved("A"), saved("B")]);
        assert!(w.select_existing && w.existing_enabled);
        assert_eq!(w.selected_fixture.as_deref(), Some("A"));
        assert_eq!(w.profile, saved("A"));
        assert_eq!(w.problem(), None);

        let w = FixtureWizard::new(Vec::new());
        assert!(!w.select_existing && !w.existing_enabled);
        assert_eq!(w.selected_fixture, None);
        assert_eq!(w.profile.functions.len(), 12);
        assert_eq!(w.profile.revision.as_deref(), Some("1.0"));
        assert_eq!(
            w.problem(),
            Some("Profile Name is empty.  Profile Name is a required field.")
        );
        assert!(!w.can_go_next());
    }

    #[test]
    fn the_radios_and_combo_make_fresh_copies_or_a_blank_profile() {
        let mut w = FixtureWizard::new(vec![saved("A"), saved("B")]);
        w.profile.name = Some("Typed over".to_string());
        w.select_profile("B");
        assert_eq!(w.profile, saved("B"));
        w.profile.name = Some("Kept".to_string());
        w.select_profile("B");
        assert_eq!(w.profile_names(), ["A", "B"]);
        assert_eq!(
            w.profile.name.as_deref(),
            Some("Kept"),
            "picking the name already showing is no change"
        );

        w.choose_create_new();
        assert_eq!(w.profile.name, None);
        assert_eq!(w.profile.functions.len(), 12);
        w.profile.name = Some("Mine".to_string());
        w.choose_create_new();
        assert_eq!(
            w.profile.name.as_deref(),
            Some("Mine"),
            "a click on the radio that's on is no change"
        );

        w.choose_existing();
        assert_eq!(w.selected_fixture.as_deref(), Some("A"));
        assert_eq!(w.profile, saved("A"));
    }

    #[test]
    fn a_blank_name_blocks_moving_on_and_quick_navigation() {
        let mut w = FixtureWizard::new(vec![saved("A")]);
        run_to(&mut w, WizardPage::ColorSupport);
        w.go_to(WizardPage::SelectProfile);
        assert_eq!(w.page(), WizardPage::SelectProfile);
        w.profile.name = Some(String::new());
        assert_eq!(w.problem(), Some(PROFILE_NAME_PROBLEM));
        w.go_next();
        w.go_to(WizardPage::EditFunctions);
        assert_eq!(w.page(), WizardPage::SelectProfile);
        assert!(!w.can_go_to(WizardPage::EditFunctions));
        w.profile.name = Some("B".to_string());
        w.go_to(WizardPage::ColorSupport);
        assert_eq!(w.page(), WizardPage::ColorSupport);
        // Not visited yet.
        w.go_to(WizardPage::Summary);
        assert_eq!(w.page(), WizardPage::ColorSupport);
    }

    #[test]
    fn create_new_then_coming_back_leaves_the_existing_radio_off_as_upstream() {
        let mut w = FixtureWizard::new(vec![saved("A")]);
        w.choose_create_new();
        assert!(w.existing_enabled, "still on within the visit");
        w.profile.name = Some("New".to_string());
        w.go_next();
        w.go_back();
        assert!(!w.existing_enabled);
        w.choose_existing();
        assert!(!w.select_existing);
    }

    #[test]
    fn leaving_edit_channels_names_the_selection() {
        let mut w = FixtureWizard::new(vec![saved("A")]);
        w.profile.name = Some("Renamed".to_string());
        assert_eq!(w.selected_fixture.as_deref(), Some("A"));
        run_to(&mut w, WizardPage::EditChannels);
        assert_eq!(w.selected_fixture.as_deref(), Some("A"));
        w.go_next();
        assert_eq!(w.selected_fixture.as_deref(), Some("Renamed"));
    }

    #[test]
    fn revisiting_preserves_answers_until_profile_changes() {
        let mut w = FixtureWizard::new(vec![saved("A")]);
        run_to(&mut w, WizardPage::ColorSupport);
        assert_eq!(w.color_support, ColorSupport::Mixing);
        w.color_support = ColorSupport::None;
        w.go_next();
        w.go_back();
        assert_eq!(w.color_support, ColorSupport::None);
        w.profile.name = Some("Changed".into());
        w.go_next();
        w.go_back();
        assert_eq!(w.color_support, ColorSupport::Mixing);
        w.profile.name = Some("A".into());

        run_to(&mut w, WizardPage::Grouping);
        assert_eq!(
            (
                w.count,
                w.prefix.as_str(),
                w.create_group,
                w.group_name.as_str()
            ),
            (1, "A", true, "Fixtures")
        );
        w.add_fixture();
        w.prefix = "X".to_string();
        w.create_group = false;
        w.group_name = "G".to_string();
        w.go_back();
        w.go_next();
        assert_eq!(
            (
                w.count,
                w.prefix.as_str(),
                w.create_group,
                w.group_name.as_str()
            ),
            (2, "X", false, "G")
        );
    }

    #[test]
    fn count_stops_at_one() {
        let mut w = FixtureWizard::new(vec![saved("A")]);
        run_to(&mut w, WizardPage::Grouping);
        w.remove_fixture();
        assert_eq!(w.count, 1);
        w.add_fixture();
        w.add_fixture();
        w.remove_fixture();
        assert_eq!(w.count, 2);
        assert_eq!(w.preview_names(), ["Fixtures", "A1", "A2"]);
        w.create_group = false;
        assert_eq!(w.preview_names(), ["A1", "A2"]);
    }

    #[test]
    fn automation_choices_survive_revisit_and_follow_changed_support() {
        let functions = vec![
            function("Wheel", FixtureFunctionType::ColorWheel, FuncId::Custom),
            function("Shutter", FixtureFunctionType::Indexed, FuncId::Shutter),
        ];
        let mut wheel_profile = profile(functions, &["Wheel", "Shutter"]);
        wheel_profile.name = Some("W".to_string());
        let mut w = FixtureWizard::new(vec![wheel_profile]);
        run_to(&mut w, WizardPage::Automation);
        assert_eq!(w.color_support, ColorSupport::Wheel);
        assert!(w.automation.shutter && w.automation.color_wheel);
        w.set_auto_shutter(false);
        assert!(!w.automation.shutter);
        w.go_back();
        w.go_next();
        assert!(!w.automation.shutter, "user choice survives return");
        w.set_auto_dimmer(true);
        assert!(!w.automation.dimmer, "no dimmer on this profile");

        // Back to Color Mixing: the wheel box is disabled, and the Mac says false where upstream keeps the old true.
        w.go_back();
        w.color_support = ColorSupport::Mixing;
        run_to(&mut w, WizardPage::Automation);
        assert_eq!(
            w.color_support,
            ColorSupport::Mixing,
            "Automation doesn't reset the page before it"
        );
        assert!(!w.automation_enabled.color_wheel && !w.automation.color_wheel);
    }

    #[test]
    fn a_stale_box_is_false_even_if_its_page_was_skipped_on_the_way_to_finish() {
        let functions = vec![function(
            "Wheel",
            FixtureFunctionType::ColorWheel,
            FuncId::Custom,
        )];
        let mut wheel_profile = profile(functions, &["Wheel"]);
        wheel_profile.name = Some("W".to_string());
        let mut w = FixtureWizard::new(vec![wheel_profile]);
        run_to(&mut w, WizardPage::Summary);
        assert!(w.choices().auto_color_wheel);
        w.go_to(WizardPage::ColorSupport);
        w.color_support = ColorSupport::None;
        w.go_to(WizardPage::Summary);
        assert!(!w.choices().auto_color_wheel);
    }

    #[test]
    fn per_colour_curves_need_colour_mixing() {
        let mut w = FixtureWizard::new(vec![saved("A")]);
        run_to(&mut w, WizardPage::DimmingCurves);
        assert!(w.per_color_enabled);
        w.set_dimming(DimmingCurveSelection::OneDimmingCurvePerColor);
        assert_eq!(w.dimming, DimmingCurveSelection::OneDimmingCurvePerColor);

        let mut w = FixtureWizard::new(Vec::new());
        w.profile.name = Some("N".to_string());
        run_to(&mut w, WizardPage::DimmingCurves);
        assert!(!w.per_color_enabled);
        w.set_dimming(DimmingCurveSelection::OneDimmingCurvePerColor);
        assert_eq!(w.dimming, DimmingCurveSelection::NoDimmingCurve);
        w.set_dimming(DimmingCurveSelection::OneDimmingCurvePerFixture);
        assert_eq!(w.dimming, DimmingCurveSelection::OneDimmingCurvePerFixture);
    }

    #[test]
    fn finish_only_on_summary() {
        let mut w = FixtureWizard::new(vec![saved("A")]);
        assert!(!w.can_finish() && !w.can_go_back() && w.can_go_next());
        run_to(&mut w, WizardPage::Summary);
        assert!(w.can_finish() && w.can_go_back() && !w.can_go_next());
        assert_eq!(w.choices().selected_fixture, "A");
    }

    // ---- F110: generation -----------------------------------------------------------------------------------------------

    #[test]
    fn rgb_on_one_function_gets_a_colour_property_and_one_breakdown() {
        let c = choices(profile(rgb_functions(), &["Color", "Color", "Color"]));
        let (s, r) = generate(&c);
        let node = r.fixtures[0];
        let color = s
            .tree
            .node(node)
            .properties
            .iter()
            .find(|p| p.type_id == property_ids::COLOR)
            .unwrap();
        assert_eq!(
            color.data,
            PropertyData::Color {
                color_type: ElementColorType::FullColor,
                single_color: Color::EMPTY,
                color_set_name: None
            }
        );
        assert_eq!(type_ids(&s), [ids::COLOR_BREAKDOWN]);
        let breakdown = children(&s, component_of(&s, node), 0)[0];
        let FilterConfig::ColorBreakdown(config) = config_of(&s, breakdown) else {
            panic!()
        };
        assert!(config.mix_colors && !config.sixteen_bit);
        assert_eq!(config.breakdown_items[1].color, DotNetColor::LIME);
        assert_eq!(s.flow.comp(breakdown).output_count, 3);
        assert_eq!(patch_names(&s, node), ["Red", "Green", "Blue"]);
    }

    #[test]
    fn rgbw_has_a_white_output() {
        let c = choices(profile(
            vec![function(
                "Color",
                FixtureFunctionType::RgbwColor,
                FuncId::Custom,
            )],
            &["Color"; 4],
        ));
        let (s, r) = generate(&c);
        assert_eq!(
            patch_names(&s, r.fixtures[0]),
            ["Red", "Green", "Blue", "White"]
        );
    }

    #[test]
    fn six_rgb_channels_are_sixteen_bit_with_coarse_fine_on_each_colour() {
        let c = choices(profile(rgb_functions(), &["Color"; 6]));
        let (s, r) = generate(&c);
        let node = r.fixtures[0];
        let breakdown = children(&s, component_of(&s, node), 0)[0];
        let FilterConfig::ColorBreakdown(config) = config_of(&s, breakdown) else {
            panic!()
        };
        assert!(config.sixteen_bit);
        for output in 0..3 {
            let on_output = children(&s, breakdown, output);
            assert_eq!(on_output.len(), 1);
            assert!(matches!(
                config_of(&s, on_output[0]),
                FilterConfig::CoarseFine(_)
            ));
        }
        assert_eq!(s.flow.patch_points(component_of(&s, node)).len(), 6);
        assert_eq!(type_ids(&s).len(), 4);
    }

    #[test]
    fn a_curve_for_the_fixture_sits_above_the_breakdown() {
        let mut c = choices(profile(rgb_functions(), &["Color"; 3]));
        c.dimming = DimmingCurveSelection::OneDimmingCurvePerFixture;
        let (s, r) = generate(&c);
        let on_element = children(&s, component_of(&s, r.fixtures[0]), 0);
        assert_eq!(on_element.len(), 1);
        assert!(matches!(
            config_of(&s, on_element[0]),
            FilterConfig::DimmingCurve(_)
        ));
        let below = children(&s, on_element[0], 0);
        assert!(matches!(
            config_of(&s, below[0]),
            FilterConfig::ColorBreakdown(_)
        ));
        assert_eq!(patch_names(&s, r.fixtures[0]), ["Red", "Green", "Blue"]);
    }

    #[test]
    fn a_curve_per_colour_sits_below_each_output_and_takes_a_copy_of_the_curve() {
        let mut c = choices(profile(rgb_functions(), &["Color"; 3]));
        c.dimming = DimmingCurveSelection::OneDimmingCurvePerColor;
        c.curve = CurveData::from_points(&[(0.0, 10.0), (100.0, 90.0)]);
        let (s, r) = generate(&c);
        let breakdown = children(&s, component_of(&s, r.fixtures[0]), 0)[0];
        assert!(matches!(
            config_of(&s, breakdown),
            FilterConfig::ColorBreakdown(_)
        ));
        for output in 0..3 {
            let on_output = children(&s, breakdown, output);
            let FilterConfig::DimmingCurve(curve) = config_of(&s, on_output[0]) else {
                panic!()
            };
            assert_eq!(curve.curve, c.curve);
        }
        assert_eq!(
            s.flow.patch_points(component_of(&s, r.fixtures[0])).len(),
            3
        );
    }

    #[test]
    fn a_curve_per_colour_with_sixteen_bit_hangs_coarse_fine_under_each_curve_last_colour_first() {
        let mut c = choices(profile(rgb_functions(), &["Color"; 6]));
        c.dimming = DimmingCurveSelection::OneDimmingCurvePerColor;
        let (s, r) = generate(&c);
        let breakdown = children(&s, component_of(&s, r.fixtures[0]), 0)[0];
        let curves: Vec<Cid> = (0..3).map(|o| children(&s, breakdown, o)[0]).collect();
        let coarse: Vec<Cid> = curves
            .iter()
            .map(|curve| children(&s, *curve, 0)[0])
            .collect();
        assert!(coarse
            .iter()
            .all(|f| matches!(config_of(&s, *f), FilterConfig::CoarseFine(_))));
        // Made by walking back from the last leaf.
        assert!(coarse[2] < coarse[1] && coarse[1] < coarse[0]);
        assert_eq!(
            s.flow.patch_points(component_of(&s, r.fixtures[0])).len(),
            6
        );
    }

    #[test]
    fn colour_functions_on_separate_channels_share_one_breakdown() {
        let functions = vec![
            function("R", FixtureFunctionType::RgbColor, FuncId::Custom),
            function("G", FixtureFunctionType::RgbColor, FuncId::Custom),
            function("B", FixtureFunctionType::RgbColor, FuncId::Custom),
            function("Dim", FixtureFunctionType::Range, FuncId::Custom),
            function("Tail", FixtureFunctionType::RgbColor, FuncId::Custom),
        ];
        let (s, _) = generate(&choices(profile(
            functions,
            &["R", "G", "B", "Dim", "Tail"],
        )));
        // Four colour channels: the first makes the breakdown, the rest only count, and the count hasn't reached four until Tail, which then resets it.
        assert_eq!(type_ids(&s), [ids::COLOR_BREAKDOWN, ids::TAGGED_FILTER]);
    }

    #[test]
    fn pan_on_two_channels_gets_a_tagged_filter_and_coarse_fine() {
        let c = choices(profile(
            vec![function("Pan", FixtureFunctionType::Range, FuncId::Pan)],
            &["Pan", "Pan"],
        ));
        let (s, r) = generate(&c);
        assert_eq!(
            type_ids(&s),
            [ids::TAGGED_FILTER, ids::COARSE_FINE_BREAKDOWN]
        );
        let tagged = children(&s, component_of(&s, r.fixtures[0]), 0)[0];
        let FilterConfig::Tagged(config) = config_of(&s, tagged) else {
            panic!()
        };
        assert_eq!(config.tag.as_deref(), Some("Pan"));
        assert!(matches!(
            config_of(&s, children(&s, tagged, 0)[0]),
            FilterConfig::CoarseFine(_)
        ));
        assert_eq!(
            s.flow.patch_points(component_of(&s, r.fixtures[0])).len(),
            2
        );
    }

    #[test]
    fn range_functions_next_to_each_other_with_different_names_stay_single() {
        let functions = vec![
            function("Pan", FixtureFunctionType::Range, FuncId::Pan),
            function("Tilt", FixtureFunctionType::Range, FuncId::Tilt),
        ];
        let (s, _) = generate(&choices(profile(functions, &["Pan", "Tilt"])));
        assert_eq!(type_ids(&s), [ids::TAGGED_FILTER, ids::TAGGED_FILTER]);
    }

    #[test]
    fn each_none_channel_gets_its_own_ignored_filter() {
        let c = choices(profile(
            vec![function("None", FixtureFunctionType::None, FuncId::Custom)],
            &["None", "None"],
        ));
        let (s, r) = generate(&c);
        let filters = children(&s, component_of(&s, r.fixtures[0]), 0);
        assert_eq!(filters.len(), 2);
        for f in filters {
            let FilterConfig::Tagged(config) = config_of(&s, f) else {
                panic!()
            };
            assert_eq!(config.tag.as_deref(), Some("Ignore"));
        }
    }

    #[test]
    fn a_run_on_any_other_function_gets_one_filter() {
        let c = choices(profile(
            vec![function("Gobo", FixtureFunctionType::Indexed, FuncId::Gobo)],
            &["Gobo", "Gobo", "Gobo"],
        ));
        let (s, _) = generate(&c);
        assert_eq!(type_ids(&s), [ids::TAGGED_FILTER]);
    }

    fn shutter_function() -> ProfileFunction {
        let mut f = function("Shutter", FixtureFunctionType::Indexed, FuncId::Shutter);
        f.index_data = vec![
            index("Zero", 0, FixtureIndexType::ShutterOpen),
            index("Open", 200, FixtureIndexType::ShutterOpen),
            index("Open 2", 210, FixtureIndexType::ShutterOpen),
            index("Closed", 10, FixtureIndexType::ShutterClosed),
            index("Wraps", 300, FixtureIndexType::ShutterClosed),
        ];
        f
    }

    #[test]
    fn auto_shutter_gets_a_shutter_filter_and_otherwise_a_tagged_one() {
        let mut c = choices(profile(vec![shutter_function()], &["Shutter"]));
        c.auto_shutter = true;
        let (s, r) = generate(&c);
        let filter = children(&s, component_of(&s, r.fixtures[0]), 0)[0];
        let FilterConfig::Shutter(config) = config_of(&s, filter) else {
            panic!()
        };
        assert_eq!(config.tag.as_deref(), Some("Shutter"));
        assert!(config.convert_color_into_shutter_intents);
        assert_eq!(
            (
                config.open_shutter_index_value,
                config.close_shutter_index_value
            ),
            (200, 10)
        );

        c.auto_shutter = false;
        let (s, _) = generate(&c);
        assert_eq!(type_ids(&s), [ids::TAGGED_FILTER]);
    }

    #[test]
    fn a_start_value_wraps_to_a_byte() {
        let mut f = function("Shutter", FixtureFunctionType::Indexed, FuncId::Shutter);
        f.index_data = vec![index("Wraps", 300, FixtureIndexType::ShutterOpen)];
        let mut c = choices(profile(vec![f], &["Shutter"]));
        c.auto_shutter = true;
        let (s, r) = generate(&c);
        let FilterConfig::Shutter(config) =
            config_of(&s, children(&s, component_of(&s, r.fixtures[0]), 0)[0])
        else {
            panic!()
        };
        assert_eq!(config.open_shutter_index_value, 44);
    }

    #[test]
    fn prism_is_always_a_prism_filter_even_with_its_box_off() {
        let mut f = function(
            "Open Close Prism",
            FixtureFunctionType::Indexed,
            FuncId::OpenClosePrism,
        );
        f.index_data = vec![
            index("In", 20, FixtureIndexType::PrismOpen),
            index("Out", 5, FixtureIndexType::PrismClose),
        ];
        f.associated_function_name = Some("Prism".to_string());
        let mut c = choices(profile(vec![f], &["Open Close Prism"]));
        c.auto_prism = false;
        let (s, r) = generate(&c);
        let FilterConfig::Prism(config) =
            config_of(&s, children(&s, component_of(&s, r.fixtures[0]), 0)[0])
        else {
            panic!()
        };
        assert_eq!(config.tag.as_deref(), Some("Open Close Prism"));
        assert!(config.convert_prism_intents_into_open_prism_intents);
        assert_eq!(
            (
                config.open_prism_index_value,
                config.close_prism_index_value
            ),
            (20, 5)
        );
        assert_eq!(config.associated_function_name.as_deref(), Some("Prism"));
    }

    fn dimmer_and_color(on_channels: &[&str]) -> FixtureProfile {
        profile(
            vec![
                function("Dimmer", FixtureFunctionType::Range, FuncId::Dim),
                function("Color", FixtureFunctionType::RgbColor, FuncId::Custom),
            ],
            on_channels,
        )
    }

    #[test]
    fn a_lamp_dimmer_converts_colour_and_takes_a_curve_when_one_is_chosen() {
        let mut c = choices(dimmer_and_color(&["Dimmer"]));
        c.auto_dimmer = true;
        assert_eq!(c.color_support, ColorSupport::None);
        let (s, r) = generate(&c);
        let filter = children(&s, component_of(&s, r.fixtures[0]), 0)[0];
        let FilterConfig::Dimming(config) = config_of(&s, filter) else {
            panic!()
        };
        assert!(config.convert_rgb_into_dimming_intents);
        assert_eq!(config.tag.as_deref(), Some("Dimmer"));
        assert!(children(&s, filter, 0).is_empty());

        c.dimming = DimmingCurveSelection::OneDimmingCurvePerFixture;
        let (s, r) = generate(&c);
        let filter = children(&s, component_of(&s, r.fixtures[0]), 0)[0];
        assert!(matches!(
            config_of(&s, children(&s, filter, 0)[0]),
            FilterConfig::DimmingCurve(_)
        ));
    }

    #[test]
    fn a_colour_mixing_dimmer_doesnt_convert_or_take_the_curve() {
        let mut c = choices(dimmer_and_color(&["Dimmer", "Color", "Color", "Color"]));
        c.auto_dimmer = true;
        c.dimming = DimmingCurveSelection::OneDimmingCurvePerColor;
        assert_eq!(c.color_support, ColorSupport::Mixing);
        let (s, r) = generate(&c);
        let on_element = children(&s, component_of(&s, r.fixtures[0]), 0);
        let FilterConfig::Dimming(config) = config_of(&s, on_element[0]) else {
            panic!()
        };
        assert!(!config.convert_rgb_into_dimming_intents);
        assert!(children(&s, on_element[0], 0).is_empty());
    }

    #[test]
    fn a_dimmer_with_auto_off_is_tagged() {
        let (s, _) = generate(&choices(dimmer_and_color(&["Dimmer"])));
        assert_eq!(type_ids(&s), [ids::TAGGED_FILTER]);
    }

    #[test]
    fn a_sixteen_bit_auto_dimmer_with_a_curve_has_coarse_fine_under_the_curve() {
        let mut c = choices(dimmer_and_color(&["Dimmer", "Dimmer"]));
        c.auto_dimmer = true;
        c.dimming = DimmingCurveSelection::OneDimmingCurvePerFixture;
        let (s, _) = generate(&c);
        assert_eq!(
            type_ids(&s),
            [
                ids::DIMMING_FILTER,
                ids::DIMMING_CURVE,
                ids::COARSE_FINE_BREAKDOWN
            ]
        );
    }

    fn wheel_function() -> ProfileFunction {
        let mut f = function("Wheel", FixtureFunctionType::ColorWheel, FuncId::Custom);
        let mut half = wheel_entry("Half", 10, "Lime");
        half.half_step = true;
        let mut curve = wheel_entry("Spin", 20, "Blue");
        curve.use_curve = true;
        let mut excluded = wheel_entry("Open", 30, "White");
        excluded.exclude_color_property = true;
        f.color_wheel_data = vec![
            wheel_entry("Red", 0, "Red"),
            half,
            curve,
            excluded,
            wheel_entry("Orange", 40, "#FF8000"),
        ];
        f
    }

    #[test]
    fn auto_colour_wheel_on_a_lamp_converts_and_adds_a_discrete_colour_set() {
        let mut c = choices(profile(vec![wheel_function()], &["Wheel"]));
        c.auto_color_wheel = true;
        assert_eq!(c.color_support, ColorSupport::Wheel);
        let (s, r) = generate(&c);
        let node = r.fixtures[0];
        let FilterConfig::ColorWheel(config) =
            config_of(&s, children(&s, component_of(&s, node), 0)[0])
        else {
            panic!()
        };
        assert!(config.convert_color_intents_into_index_commands);
        assert_eq!(config.tag.as_deref(), Some("Wheel"));
        let wheel = config.color_wheel_data.as_ref().unwrap();
        assert_eq!(wheel.len(), 5);
        // The spelling reaches the filter, because a colour saved by name never matches a value colour.
        assert_eq!(wheel[0].color1_html.as_deref(), Some("Red"));
        assert_eq!(wheel[4].color1_html.as_deref(), Some("#FF8000"));
        assert_eq!(wheel[0].index_type, "ColorWheel");

        let color = s
            .tree
            .node(node)
            .properties
            .iter()
            .find(|p| p.type_id == property_ids::COLOR)
            .unwrap();
        assert_eq!(
            color.data,
            PropertyData::Color {
                color_type: ElementColorType::MultipleDiscreteColors,
                single_color: Color::EMPTY,
                color_set_name: Some("Head".to_string())
            }
        );
        assert_eq!(
            s.system.tree.color_sets.get("Head").unwrap(),
            &vec![Color::rgb(255, 0, 0), Color::rgb(255, 128, 0)]
        );
    }

    #[test]
    fn a_wheel_on_two_channels_lists_its_colours_twice_and_makes_one_filter() {
        let mut c = choices(profile(vec![wheel_function()], &["Wheel", "Wheel"]));
        c.auto_color_wheel = true;
        let (s, _) = generate(&c);
        assert_eq!(s.system.tree.color_sets.get("Head").unwrap().len(), 4);
        assert_eq!(type_ids(&s), [ids::COLOR_WHEEL_FILTER]);
    }

    #[test]
    fn a_wheel_with_auto_off_is_tagged_with_no_colour_property() {
        let c = choices(profile(vec![wheel_function()], &["Wheel"]));
        let (s, r) = generate(&c);
        assert_eq!(type_ids(&s), [ids::TAGGED_FILTER]);
        assert!(!s
            .tree
            .node(r.fixtures[0])
            .properties
            .iter()
            .any(|p| p.type_id == property_ids::COLOR));
        assert!(!s.system.tree.color_sets.contains_key("Head"));
    }

    #[test]
    fn a_wheel_beside_colour_mixing_doesnt_convert_or_add_a_second_colour_property() {
        let functions = vec![
            function("Color", FixtureFunctionType::RgbColor, FuncId::Custom),
            wheel_function(),
        ];
        let mut c = choices(profile(functions, &["Color", "Wheel"]));
        c.auto_color_wheel = true;
        let (s, r) = generate(&c);
        let filters = children(&s, component_of(&s, r.fixtures[0]), 0);
        let FilterConfig::ColorWheel(config) = config_of(&s, filters[1]) else {
            panic!()
        };
        assert!(!config.convert_color_intents_into_index_commands);
        let colors = s
            .tree
            .node(r.fixtures[0])
            .properties
            .iter()
            .filter(|p| p.type_id == property_ids::COLOR)
            .count();
        assert_eq!(colors, 1);
    }

    #[test]
    fn patch_points_come_out_in_channel_order() {
        let functions = vec![
            function("Pan", FixtureFunctionType::Range, FuncId::Pan),
            function("Tilt", FixtureFunctionType::Range, FuncId::Tilt),
            function("Color", FixtureFunctionType::RgbColor, FuncId::Custom),
            function("Gobo", FixtureFunctionType::Indexed, FuncId::Gobo),
            function("None", FixtureFunctionType::None, FuncId::Custom),
        ];
        let c = choices(profile(
            functions,
            &[
                "Pan", "Pan", "Tilt", "Color", "Color", "Color", "None", "Gobo", "None",
            ],
        ));
        let (s, r) = generate(&c);
        assert_eq!(
            patch_names(&s, r.fixtures[0]),
            [
                "Coarse Fine Breakdown",
                "Coarse Fine Breakdown",
                "Tilt",
                "Red",
                "Green",
                "Blue",
                "Ignore",
                "Gobo",
                "Ignore"
            ]
        );
    }

    #[test]
    fn a_group_holds_the_fixtures_and_they_arent_root_rows() {
        let mut c = choices(profile(rgb_functions(), &["Color"]));
        c.count = 3;
        c.create_group = true;
        let (s, r) = generate(&c);
        assert_eq!(s.tree.roots.len(), 1);
        let group = s.tree.roots[0];
        assert_eq!(Some(group), r.head);
        assert_eq!(s.tree.node(group).name, "Fixtures");
        assert_eq!(s.tree.node(group).element_id, None);
        let names: Vec<&str> = s
            .tree
            .node(group)
            .children
            .iter()
            .map(|n| s.tree.node(*n).name.as_str())
            .collect();
        assert_eq!(names, ["MH1", "MH2", "MH3"]);
        assert!(r
            .fixtures
            .iter()
            .all(|f| !s.tree.node(*f).at_root && s.tree.node(*f).element_id.is_some()));

        let group_row = s.row_path_of(group);
        assert_eq!(r.edit.expand, [group_row.clone()]);
        assert_eq!(
            r.edit.select,
            Some(vec![child_path(Some(&group_row), r.fixtures[0])])
        );
        assert!(r.edit.reload_roots);
    }

    #[test]
    fn with_no_group_the_fixtures_are_roots_and_the_first_is_selected() {
        let mut c = choices(profile(rgb_functions(), &["Color"]));
        c.count = 2;
        let (s, r) = generate(&c);
        assert_eq!(s.tree.roots, r.fixtures);
        assert_eq!(r.edit.select, Some(vec![s.row_path_of(r.fixtures[0])]));
        assert!(r.edit.expand.is_empty());
        assert!(r.edit.scroll_to_top);
    }

    #[test]
    fn names_are_made_unique_and_an_empty_prefix_is_just_the_number() {
        let mut s = session(&["MH1"], 0);
        let mut c = choices(profile(rgb_functions(), &["Color"]));
        c.create_group = true;
        let r = create_fixture_elements(&mut s, &c).unwrap();
        assert_eq!(s.tree.node(r.fixtures[0]).name, "MH1 - 2");
        let second = create_fixture_elements(&mut s, &c).unwrap();
        assert_eq!(s.tree.node(second.head.unwrap()).name, "Fixtures - 2");
        assert_eq!(s.tree.node(second.fixtures[0]).name, "MH1 - 3");

        c.group_name = "MH4".to_string();
        c.prefix = "MH".to_string();
        c.count = 4;
        let third = create_fixture_elements(&mut s, &c).unwrap();
        assert_eq!(s.tree.node(third.fixtures[3]).name, "MH4");
        assert_eq!(
            s.tree.node(third.head.unwrap()).name,
            "MH4 - 2",
            "the group is named after its fixtures"
        );

        c.prefix = String::new();
        c.count = 1;
        c.create_group = false;
        let fourth = create_fixture_elements(&mut s, &c).unwrap();
        assert_eq!(s.tree.node(fourth.fixtures[0]).name, "1");
    }

    #[test]
    fn a_run_without_a_group_after_one_with_a_group_is_fine() {
        let mut s = session(&[], 0);
        let mut c = choices(profile(rgb_functions(), &["Color"]));
        c.create_group = true;
        create_fixture_elements(&mut s, &c).unwrap();
        c.create_group = false;
        let r = create_fixture_elements(&mut s, &c).unwrap();
        assert_eq!(s.tree.roots.len(), 2);
        assert_eq!(
            s.tree.node(s.tree.roots[0]).children.len(),
            1,
            "the earlier group keeps its fixture"
        );
        assert!(s.tree.node(r.fixtures[0]).at_root);
    }

    #[test]
    fn a_channel_on_an_undefined_function_stops_before_anything_is_made() {
        let c = choices(profile(rgb_functions(), &["Missing"]));
        let mut s = session(&[], 0);
        let error = create_fixture_elements(&mut s, &c).unwrap_err();
        assert!(error.contains("Missing"));
        assert!(s.tree.roots.is_empty());
        assert!(!s.is_dirty());

        let mut nameless = choices(profile(rgb_functions(), &["Color"]));
        nameless.profile.name = Some(String::new());
        assert_eq!(
            create_fixture_elements(&mut s, &nameless).unwrap_err(),
            PROFILE_NAME_PROBLEM
        );
    }

    #[test]
    fn a_profile_with_no_channels_makes_a_fixture_with_no_filters() {
        let mut p = FixtureProfile::new_blank();
        p.name = Some("Blank".to_string());
        let (s, r) = generate(&choices(p));
        assert!(type_ids(&s).is_empty());
        assert_eq!(
            s.flow.patch_points(component_of(&s, r.fixtures[0])).len(),
            1,
            "the element's own output"
        );
    }

    #[test]
    fn each_fixture_has_its_own_property_filters_and_store_entry() {
        let mut c = choices(profile(rgb_functions(), &["Color"]));
        c.count = 2;
        let (s, r) = generate(&c);
        assert_eq!(type_ids(&s).len(), 2);
        let property = |n: Nid| {
            s.tree
                .node(n)
                .properties
                .iter()
                .find(|p| p.type_id == property_ids::INTELLIGENT_FIXTURE)
                .unwrap()
                .clone()
        };
        let (a, b) = (property(r.fixtures[0]), property(r.fixtures[1]));
        assert_ne!(a.instance_id, b.instance_id);
        let entry = s
            .system
            .module_store
            .instance(property_ids::INTELLIGENT_FIXTURE, a.instance_id)
            .unwrap();
        assert_eq!(entry.data_model_type(), Some("VixenModules.Property.IntelligentFixture.IntelligentFixtureData, IntelligentFixture"));
        assert_eq!(
            entry.assembly_name(),
            Some("Module.Property.IntelligentFixture")
        );
        assert_eq!(
            entry
                .data()
                .unwrap()
                .child_text("ModuleInstanceId")
                .as_deref(),
            Some(a.instance_id.to_string().as_str())
        );
        assert_eq!(
            s.tree
                .node(r.fixtures[0])
                .properties
                .iter()
                .map(|p| p.type_id)
                .collect::<Vec<_>>(),
            [property_ids::INTELLIGENT_FIXTURE, property_ids::COLOR]
        );
    }

    #[test]
    fn generation_marks_what_it_changed() {
        let (s, _) = generate(&choices(profile(rgb_functions(), &["Color"])));
        assert!(s.dirty.tree && s.dirty.flow && s.dirty.filters);
    }

    // ---- Profiles ---------------------------------------------------------------------------------------------------------

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-fixture-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// A profile as `XmlSerializer` writes it, with a null `Manufacturer` and `Label` left out and `Image` empty.
    const SAMPLE_FILE: &str = "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\
<FixtureSpecification xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" version=\"1\">\r\n\
\t<Name>Moving &amp; Head</Name>\r\n\
\t<CreatedBy>Owner</CreatedBy>\r\n\
\t<Revision>2.5</Revision>\r\n\
\t<ChannelDefinitions>\r\n\
\t\t<FixtureChannel>\r\n\
\t\t\t<Name>Pan</Name>\r\n\
\t\t\t<Function>Pan</Function>\r\n\
\t\t\t<ChannelNumber>1</ChannelNumber>\r\n\
\t\t</FixtureChannel>\r\n\
\t\t<FixtureChannel>\r\n\
\t\t\t<Name>Wheel</Name>\r\n\
\t\t\t<Function>Wheel</Function>\r\n\
\t\t\t<ChannelNumber>2</ChannelNumber>\r\n\
\t\t</FixtureChannel>\r\n\
\t</ChannelDefinitions>\r\n\
\t<FunctionDefinitions>\r\n\
\t\t<FixtureFunction>\r\n\
\t\t\t<Name>Pan</Name>\r\n\
\t\t\t<FunctionType>Range</FunctionType>\r\n\
\t\t\t<FunctionIdentity>Pan</FunctionIdentity>\r\n\
\t\t\t<IndexData />\r\n\
\t\t\t<ColorWheelData />\r\n\
\t\t\t<RotationLimits>\r\n\
\t\t\t\t<StartPosition>0</StartPosition>\r\n\
\t\t\t\t<StopPosition>540</StopPosition>\r\n\
\t\t\t</RotationLimits>\r\n\
\t\t\t<ZoomType>NarrowToWide</ZoomType>\r\n\
\t\t\t<TimelineColor1Html>Red</TimelineColor1Html>\r\n\
\t\t</FixtureFunction>\r\n\
\t\t<FixtureFunction>\r\n\
\t\t\t<Name>Wheel</Name>\r\n\
\t\t\t<FunctionType>ColorWheel</FunctionType>\r\n\
\t\t\t<FunctionIdentity>Custom</FunctionIdentity>\r\n\
\t\t\t<IndexData />\r\n\
\t\t\t<ColorWheelData>\r\n\
\t\t\t\t<FixtureColorWheel>\r\n\
\t\t\t\t\t<Name>Orange</Name>\r\n\
\t\t\t\t\t<StartValue>10</StartValue>\r\n\
\t\t\t\t\t<EndValue>19</EndValue>\r\n\
\t\t\t\t\t<UseCurve>false</UseCurve>\r\n\
\t\t\t\t\t<IndexType>ColorWheel</IndexType>\r\n\
\t\t\t\t\t<HalfStep>true</HalfStep>\r\n\
\t\t\t\t\t<Color1Html>#FF0000</Color1Html>\r\n\
\t\t\t\t\t<Color2Html>DarkOrange</Color2Html>\r\n\
\t\t\t\t\t<ExcludeColorProperty>true</ExcludeColorProperty>\r\n\
\t\t\t\t</FixtureColorWheel>\r\n\
\t\t\t</ColorWheelData>\r\n\
\t\t\t<Label>W</Label>\r\n\
\t\t\t<RotationLimits>\r\n\
\t\t\t\t<StartPosition>0</StartPosition>\r\n\
\t\t\t\t<StopPosition>360</StopPosition>\r\n\
\t\t\t</RotationLimits>\r\n\
\t\t\t<ZoomType>WideToNarrow</ZoomType>\r\n\
\t\t\t<TimelineColor1Html>#010203</TimelineColor1Html>\r\n\
\t\t\t<AssociatedFunctionName>Pan</AssociatedFunctionName>\r\n\
\t\t</FixtureFunction>\r\n\
\t\t<FixtureFunction>\r\n\
\t\t\t<Name>Gobo</Name>\r\n\
\t\t\t<FunctionType>Indexed</FunctionType>\r\n\
\t\t\t<FunctionIdentity>Gobo</FunctionIdentity>\r\n\
\t\t\t<IndexData>\r\n\
\t\t\t\t<FixtureIndex>\r\n\
\t\t\t\t\t<Name>Open</Name>\r\n\
\t\t\t\t\t<StartValue>1</StartValue>\r\n\
\t\t\t\t\t<EndValue>2</EndValue>\r\n\
\t\t\t\t\t<UseCurve>true</UseCurve>\r\n\
\t\t\t\t\t<IndexType>PrismOpen</IndexType>\r\n\
\t\t\t\t\t<Image />\r\n\
\t\t\t\t</FixtureIndex>\r\n\
\t\t\t</IndexData>\r\n\
\t\t\t<ColorWheelData />\r\n\
\t\t\t<RotationLimits>\r\n\
\t\t\t\t<StartPosition>0</StartPosition>\r\n\
\t\t\t\t<StopPosition>360</StopPosition>\r\n\
\t\t\t</RotationLimits>\r\n\
\t\t\t<ZoomType>NarrowToWide</ZoomType>\r\n\
\t\t\t<TimelineColor1Html>Transparent</TimelineColor1Html>\r\n\
\t\t</FixtureFunction>\r\n\
\t</FunctionDefinitions>\r\n\
</FixtureSpecification>";

    #[test]
    fn a_file_written_by_upstream_reads_and_writes_back_the_same_bytes() {
        let root = XElement::parse(SAMPLE_FILE).unwrap();
        let p = read_profile(&root).unwrap();
        assert_eq!(p.name.as_deref(), Some("Moving & Head"));
        assert_eq!(p.manufacturer, None);
        assert_eq!(p.created_by.as_deref(), Some("Owner"));
        assert_eq!(p.revision.as_deref(), Some("2.5"));
        assert_eq!(p.channels[1].function.as_deref(), Some("Wheel"));
        assert_eq!(p.functions[0].rotation_limits.stop_position, 540);
        assert_eq!(
            p.functions[1].color_wheel_data[0].color2.color(),
            Color::rgb(255, 140, 0)
        );
        assert_eq!(p.functions[1].zoom_type, FixtureZoomType::WideToNarrow);
        assert_eq!(
            p.functions[2].index_data[0].index_type,
            FixtureIndexType::PrismOpen
        );
        assert_eq!(p.functions[2].index_data[0].image.as_deref(), Some(""));
        assert_eq!(p.functions[2].timeline_color.to_html(), "Transparent");
        assert_eq!(profile_file_text(&p), SAMPLE_FILE);
    }

    #[test]
    fn a_blank_profile_writes_the_built_in_functions_with_their_colour_names() {
        let mut p = FixtureProfile::new_blank();
        p.name = Some("N".to_string());
        p.created_by = Some("Me".to_string());
        let text = profile_file_text(&p);
        assert!(text.contains("<Name>N</Name>\r\n\t<CreatedBy>Me</CreatedBy>\r\n\t<Revision>1.0</Revision>\r\n\t<ChannelDefinitions />"));
        for color in [
            "Red",
            "Green",
            "White",
            "Purple",
            "DarkGray",
            "Blue",
            "Black",
            "Pink",
            "Orange",
            "Yellow",
            "LightBlue",
            "Transparent",
        ] {
            assert!(
                text.contains(&format!("<TimelineColor1Html>{color}</TimelineColor1Html>")),
                "{color}"
            );
        }
        assert_eq!(read_profile(&XElement::parse(&text).unwrap()).unwrap(), p);
    }

    #[test]
    fn what_a_file_leaves_out_takes_the_constructors_defaults() {
        let p = read_profile(
            &XElement::parse(
                "<FixtureSpecification><Name>X</Name><FunctionDefinitions><FixtureFunction><Name>F</Name><ColorWheelData><FixtureColorWheel><IndexType>Custom</IndexType></FixtureColorWheel></ColorWheelData></FixtureFunction></FunctionDefinitions></FixtureSpecification>",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(p.revision.as_deref(), Some("1.0"));
        assert_eq!(p.created_by, Some(user_name()));
        let f = &p.functions[0];
        assert_eq!(
            (f.function_type, f.identity),
            (FixtureFunctionType::Range, FuncId::Custom)
        );
        assert_eq!(f.timeline_color, DotNetColor::WHITE);
        assert_eq!(f.rotation_limits.stop_position, 360);
        let entry = &f.color_wheel_data[0];
        assert_eq!(entry.color1, DotNetColor::WHITE);
        assert!(
            profile_file_text(&p).contains("<IndexType>ColorWheel</IndexType>"),
            "a wheel entry's type never survives a clone"
        );
    }

    #[test]
    fn what_the_serializer_would_throw_on_is_a_malformed_file() {
        for bad in [
            "<FixtureSpecification><ChannelDefinitions><FixtureChannel><ChannelNumber>x</ChannelNumber></FixtureChannel></ChannelDefinitions></FixtureSpecification>",
            "<FixtureSpecification><FunctionDefinitions><FixtureFunction><FunctionType>Nope</FunctionType></FixtureFunction></FunctionDefinitions></FixtureSpecification>",
            "<FixtureSpecification><FunctionDefinitions><FixtureFunction><TimelineColor1Html>NotAColor</TimelineColor1Html></FixtureFunction></FunctionDefinitions></FixtureSpecification>",
            "<FixtureSpecification><FunctionDefinitions><FixtureFunction><IndexData><FixtureIndex><UseCurve>maybe</UseCurve></FixtureIndex></IndexData></FixtureFunction></FunctionDefinitions></FixtureSpecification>",
        ] {
            assert!(read_profile(&XElement::parse(bad).unwrap()).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_empty_timeline_colour_becomes_white_for_files_from_the_old_bug() {
        let p = read_profile(&XElement::parse("<FixtureSpecification><FunctionDefinitions><FixtureFunction><TimelineColor1Html /></FixtureFunction></FunctionDefinitions></FixtureSpecification>").unwrap()).unwrap();
        assert_eq!(p.functions[0].timeline_color, DotNetColor::WHITE);
    }

    #[test]
    fn file_names_replace_what_windows_forbids() {
        let mut p = FixtureProfile::new_blank();
        p.name = Some("a<b>c:d\"e/f\\g|h?i*j".to_string());
        assert_eq!(p.file_name(), "a(b(c_d_e_f_g_h_i_j.xml");
    }

    #[test]
    fn the_folder_loads_sorted_without_malformed_files_or_taken_names() {
        let dir = temp_dir("load");
        std::fs::create_dir_all(&dir).unwrap();
        let write = |file: &str, name: &str| {
            let mut p = profile(rgb_functions(), &["Color"]);
            p.name = Some(name.to_string());
            std::fs::write(dir.join(file), profile_file_text(&p)).unwrap();
        };
        write("z.xml", "beta");
        write("a.XML", "Alpha");
        write("c.xml", "beta");
        write("d.xml", "Beta");
        write("e.txt", "Ignored");
        std::fs::write(dir.join("broken.xml"), "<FixtureSpecification>").unwrap();
        std::fs::write(
            dir.join("other.xml"),
            "<Something><Name>Other</Name></Something>",
        )
        .unwrap();
        std::fs::write(dir.join("nameless.xml"), "<FixtureSpecification />").unwrap();

        let loaded = load_fixture_profiles(&dir);
        let names: Vec<&str> = loaded
            .profiles
            .iter()
            .map(FixtureProfile::name_text)
            .collect();
        assert_eq!(names, ["Alpha", "beta", "Beta"]);
        assert_eq!(loaded.warnings, ["A Intelligent Fixture with a name of 'beta' already exists.  The file: 'z.xml' was ignored."]);
        assert!(dir.join("Images").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_folder_is_made_and_empty() {
        let dir = temp_dir("missing");
        assert_eq!(load_fixture_profiles(&dir), FixtureProfiles::default());
        assert!(dir.join("Images").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_overwrites_under_the_same_name_and_loads_back() {
        let dir = temp_dir("save");
        let mut p = profile(rgb_functions(), &["Color"]);
        save_fixture_profile(&dir, &p).unwrap();
        p.revision = Some("2.0".to_string());
        save_fixture_profile(&dir, &p).unwrap();
        let files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(files, ["Head.xml"]);
        assert_eq!(load_fixture_profiles(&dir).profiles, [p]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_profile_only_on_a_property_joins_the_list() {
        let mut s = session(&[], 0);
        let mut p = profile(rgb_functions(), &["Color"]);
        p.name = Some("Only Here".to_string());
        create_fixture_elements(&mut s, &choices(p.clone())).unwrap();
        // Add Property's blank one has no name, so it stays out.
        s.add_element(None, "Plain");
        let plain = s.row_path_of(*s.tree.roots.last().unwrap());
        s.add_property(&[plain], property_ids::INTELLIGENT_FIXTURE);
        let mut list = FixtureProfiles {
            profiles: vec![saved("Zed")],
            warnings: Vec::new(),
        };
        add_profiles_in_tree(&mut list, &s);
        let names: Vec<&str> = list
            .profiles
            .iter()
            .map(FixtureProfile::name_text)
            .collect();
        assert_eq!(names, ["Only Here", "Zed"]);
        assert_eq!(list.profiles[0], p);
    }

    // ---- Module data --------------------------------------------------------------------------------------------------------

    fn raw_xml_of(s: &SetupSession, node: Nid) -> String {
        match &s
            .tree
            .node(node)
            .properties
            .iter()
            .find(|p| p.type_id == property_ids::INTELLIGENT_FIXTURE)
            .unwrap()
            .data
        {
            PropertyData::Unknown { raw_xml } => raw_xml.clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn new_property_data_has_the_built_in_functions_and_one_default_channel() {
        let (t, i) = (property_ids::INTELLIGENT_FIXTURE, Uuid::new_v4());
        let data = new_fixture_property_data(&t, &i);
        let spec = data.child("FixtureSpecification").unwrap();
        let p = read_profile(spec).unwrap();
        assert_eq!(p.functions.len(), 12);
        assert_eq!(
            p.channels,
            [ProfileChannel {
                name: None,
                function: None,
                channel_number: 1
            }]
        );
        assert_eq!(p.name, None);
        assert_eq!(data.child_text("ModuleInstanceId"), Some(i.to_string()));
        assert_eq!(data.child_text("ModuleTypeId"), Some(t.to_string()));
        let channel = spec
            .child("ChannelDefinitions")
            .unwrap()
            .elements()
            .next()
            .unwrap();
        assert!(channel.child("Function").unwrap().nil);
        assert!(spec.child("Name").unwrap().nil);
        let members: Vec<&str> = spec.elements().map(|e| e.local.as_str()).collect();
        assert_eq!(
            members,
            [
                "ChannelDefinitions",
                "CreatedBy",
                "FunctionDefinitions",
                "Manufacturer",
                "Name",
                "Revision",
                "version"
            ]
        );
        assert_eq!(spec.ns, contract_ns(PROPERTY_NS));
        assert_eq!(channel.ns, contract_ns(FIXTURE_NS));
    }

    #[test]
    fn member_order_is_the_data_contracts() {
        let root = XElement::parse(SAMPLE_FILE).unwrap();
        let p = read_profile(&root).unwrap();
        let data = fixture_property_data(&p, &property_ids::INTELLIGENT_FIXTURE, &Uuid::new_v4());
        let spec = data.child("FixtureSpecification").unwrap();
        let names = |e: &XElement| {
            e.elements()
                .map(|c| c.local.clone())
                .collect::<Vec<String>>()
        };
        let functions: Vec<&XElement> = spec
            .child("FunctionDefinitions")
            .unwrap()
            .elements()
            .collect();
        assert_eq!(
            names(functions[0]),
            [
                "Name",
                "AssociatedFunctionName",
                "ColorWheelData",
                "FunctionIdentity",
                "FunctionType",
                "IndexData",
                "Label",
                "RotationLimits",
                "TimelineColor1Html",
                "ZoomType"
            ]
        );
        assert_eq!(
            names(
                functions[1]
                    .child("ColorWheelData")
                    .unwrap()
                    .elements()
                    .next()
                    .unwrap()
            ),
            [
                "Name",
                "EndValue",
                "IndexType",
                "StartValue",
                "UseCurve",
                "Color1Html",
                "Color2Html",
                "ExcludeColorProperty",
                "HalfStep"
            ]
        );
        assert_eq!(
            names(
                functions[2]
                    .child("IndexData")
                    .unwrap()
                    .elements()
                    .next()
                    .unwrap()
            ),
            [
                "Name",
                "EndValue",
                "IndexType",
                "StartValue",
                "UseCurve",
                "Image"
            ]
        );
        assert_eq!(
            names(
                spec.child("ChannelDefinitions")
                    .unwrap()
                    .elements()
                    .next()
                    .unwrap()
            ),
            ["Name", "ChannelNumber", "Function"]
        );
        assert_eq!(
            read_profile(spec).unwrap(),
            p,
            "module data reads back whole"
        );
    }

    #[test]
    fn the_effects_can_read_what_generation_stores() {
        let root = XElement::parse(SAMPLE_FILE).unwrap();
        let (s, r) = generate(&choices(read_profile(&root).unwrap()));
        let raw_xml = raw_xml_of(&s, r.fixtures[0]);
        let spec = fx_fixture::spec::parse_specification(&raw_xml).unwrap();
        assert_eq!(spec.name, "Moving & Head");
        assert_eq!(spec.channels.len(), 2);
        assert_eq!(spec.channels[1].function, "Wheel");
        assert!(spec.supports_function(fx_fixture::spec::FuncId::Pan));
        assert_eq!(spec.functions[1].color_wheel_data[0].start_value, 10);
        assert!(spec.functions[1].color_wheel_data[0].half_step);
    }

    #[test]
    fn it_all_survives_a_save_and_reload() {
        let root =
            std::env::temp_dir().join(format!("vixen-fixture-commit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let core = crate::app::AppCore::open(root.clone(), true).unwrap();
        let mut s = SetupSession::open(&core);
        let mut c = choices(profile(
            vec![
                wheel_function(),
                function("Pan", FixtureFunctionType::Range, FuncId::Pan),
                shutter_function(),
            ],
            &["Pan", "Pan", "Wheel", "Shutter"],
        ));
        c.auto_color_wheel = true;
        c.auto_shutter = true;
        c.count = 2;
        c.create_group = true;
        let r = create_fixture_elements(&mut s, &c).unwrap();
        let raw_xml = raw_xml_of(&s, r.fixtures[0]);
        assert!(s.commit(&core, |_, _| {}).unwrap());

        let loaded = load_system(&core.paths).unwrap();
        assert_eq!(loaded.tree.roots.len(), 1);
        assert_eq!(loaded.channels.len(), 2);
        assert_eq!(loaded.filters.len(), 8);
        assert_eq!(loaded.tree.color_sets.get("Head").unwrap().len(), 2);
        let group = loaded.tree.node(loaded.tree.roots[0]);
        let fixture = loaded.tree.node(group.children[0]);
        assert_eq!(fixture.name, "MH1");
        let property = fixture
            .properties
            .iter()
            .find(|p| p.type_id == property_ids::INTELLIGENT_FIXTURE)
            .unwrap();
        assert_eq!(property.data, PropertyData::Unknown { raw_xml });
        let entry = loaded
            .module_store
            .instance(property_ids::INTELLIGENT_FIXTURE, property.instance_id)
            .unwrap();
        assert_eq!(
            entry.assembly_name(),
            Some("Module.Property.IntelligentFixture")
        );
        assert!(fixture.properties.iter().any(|p| matches!(&p.data, PropertyData::Color { color_set_name: Some(n), .. } if n == "Head")));

        // Reopened, the patches are where generation put them, and nothing has changed.
        let mut again = SetupSession::open(&core);
        assert!(again.warnings.is_empty());
        let node = again
            .tree
            .live_nodes()
            .find(|(_, n)| n.name == "MH1")
            .unwrap()
            .0;
        let element = again
            .flow
            .find(&again.tree.node(node).element_id.unwrap())
            .unwrap();
        assert_eq!(again.flow.patch_points(element).len(), 4);
        assert!(!again.commit(&core, |_, _| {}).unwrap());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn colour_set_for_a_profile_replaces_one_of_the_same_name() {
        let mut s = session(&[], 0);
        s.system
            .tree
            .color_sets
            .insert("Head".to_string(), vec![Color::BLACK]);
        let mut c = choices(profile(vec![wheel_function()], &["Wheel"]));
        c.auto_color_wheel = true;
        create_fixture_elements(&mut s, &c).unwrap();
        assert_eq!(s.system.tree.color_sets.get("Head").unwrap().len(), 2);
    }

    #[test]
    fn built_in_function_list_matches_upstream() {
        let functions = built_in_functions();
        let names: Vec<&str> = functions
            .iter()
            .map(|f| f.name.as_deref().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "Pan",
                "Tilt",
                "Color Wheel",
                "Dimmer",
                "Color",
                "Zoom",
                "Shutter",
                "Gobo Wheel",
                "Open Close Prism",
                "Prism",
                "Frost",
                "None"
            ]
        );
        assert_eq!(functions[11].function_type, FixtureFunctionType::None);
        assert_eq!(functions[4].function_type, FixtureFunctionType::RgbwColor);
        assert_eq!(functions[3].identity, FuncId::Dim);
    }
}
