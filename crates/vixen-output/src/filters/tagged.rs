//! Port of the tagged filter family: Vixen.Modules/OutputFilter/TaggedFilter, DimmingFilter, PrismFilter,
//! ShutterFilter and ColorWheelFilter. They share an output that keeps a list of the states that apply to it.

use crate::dotnet::DotNetColor;
use crate::flow::{Flow, StateRef, States};
use crate::ids;
use crate::xmlutil::{self, bool_el, data_root, num_el, opt_text_el, text_el, ModuleIds};
use std::sync::Arc;
use vixen_core::value::FixtureIndexType;
use vixen_core::{Color, CommandValue, FunctionIdentity, RangeValue, StateValue};
use vixen_xml::{contract_ns, XElement};

/// `TaggedFilterDataBase.Tag` lives in the base class's namespace, whatever the derived filter.
const TAGGED_NS: &str = "VixenModules.OutputFilter.TaggedFilter";
const DIMMING_NS: &str = "VixenModules.OutputFilter.DimmingFilter";
const PRISM_NS: &str = "VixenModules.OutputFilter.PrismFilter";
const SHUTTER_NS: &str = "VixenModules.OutputFilter.ShutterFilter";
const COLOR_WHEEL_NS: &str = "VixenModules.OutputFilter.ColorWheelFilter";
const FIXTURE_NS: &str = "VixenModules.App.Fixture";

fn read_tag(e: &XElement) -> Option<String> {
    xmlutil::get_opt_string(e, "Tag")
}

fn tag_el(tag: Option<&str>) -> XElement {
    opt_text_el(&contract_ns(TAGGED_NS), "Tag", tag)
}

/// `TaggedFilterData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaggedFilterConfig {
    pub ids: ModuleIds,
    pub tag: Option<String>,
}

impl Default for TaggedFilterConfig {
    fn default() -> Self {
        TaggedFilterConfig { ids: ModuleIds::new(ids::TAGGED_FILTER, Default::default()), tag: None }
    }
}

impl TaggedFilterConfig {
    pub fn from_xml(e: &XElement) -> TaggedFilterConfig {
        TaggedFilterConfig { ids: ModuleIds::read(e), tag: read_tag(e) }
    }

    pub fn to_xml(&self) -> XElement {
        let mut e = data_root(&contract_ns(TAGGED_NS), "TaggedFilterData", &self.ids);
        e.push(tag_el(self.tag.as_deref()));
        e
    }
}

/// `DimmingFilterData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DimmingFilterConfig {
    pub ids: ModuleIds,
    pub tag: Option<String>,
    pub convert_rgb_into_dimming_intents: bool,
}

impl Default for DimmingFilterConfig {
    fn default() -> Self {
        DimmingFilterConfig {
            ids: ModuleIds::new(ids::DIMMING_FILTER, Default::default()),
            tag: None,
            convert_rgb_into_dimming_intents: false,
        }
    }
}

impl DimmingFilterConfig {
    pub fn from_xml(e: &XElement) -> DimmingFilterConfig {
        DimmingFilterConfig {
            ids: ModuleIds::read(e),
            tag: read_tag(e),
            convert_rgb_into_dimming_intents: xmlutil::get_bool(e, "ConvertRGBIntoDimmingIntents"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(DIMMING_NS);
        let mut e = data_root(&ns, "DimmingFilterData", &self.ids);
        e.push(tag_el(self.tag.as_deref()));
        e.push(bool_el(&ns, "ConvertRGBIntoDimmingIntents", self.convert_rgb_into_dimming_intents));
        e
    }
}

/// `PrismFilterData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrismFilterConfig {
    pub ids: ModuleIds,
    pub tag: Option<String>,
    pub associated_function_name: Option<String>,
    pub close_prism_index_value: u8,
    pub convert_prism_intents_into_open_prism_intents: bool,
    pub open_prism_index_value: u8,
}

impl Default for PrismFilterConfig {
    fn default() -> Self {
        PrismFilterConfig {
            ids: ModuleIds::new(ids::PRISM_FILTER, Default::default()),
            tag: None,
            associated_function_name: None,
            close_prism_index_value: 0,
            convert_prism_intents_into_open_prism_intents: false,
            open_prism_index_value: 0,
        }
    }
}

impl PrismFilterConfig {
    pub fn from_xml(e: &XElement) -> PrismFilterConfig {
        PrismFilterConfig {
            ids: ModuleIds::read(e),
            tag: read_tag(e),
            associated_function_name: xmlutil::get_opt_string(e, "AssociatedFunctionName"),
            close_prism_index_value: xmlutil::get_u8(e, "ClosePrismIndexValue"),
            convert_prism_intents_into_open_prism_intents: xmlutil::get_bool(e, "ConvertPrismIntentsIntoOpenPrismIntents"),
            open_prism_index_value: xmlutil::get_u8(e, "OpenPrismIndexValue"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(PRISM_NS);
        let mut e = data_root(&ns, "PrismFilterData", &self.ids);
        e.push(tag_el(self.tag.as_deref()));
        e.push(opt_text_el(&ns, "AssociatedFunctionName", self.associated_function_name.as_deref()));
        e.push(num_el(&ns, "ClosePrismIndexValue", self.close_prism_index_value));
        e.push(bool_el(&ns, "ConvertPrismIntentsIntoOpenPrismIntents", self.convert_prism_intents_into_open_prism_intents));
        e.push(num_el(&ns, "OpenPrismIndexValue", self.open_prism_index_value));
        e
    }
}

/// `ShutterFilterData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutterFilterConfig {
    pub ids: ModuleIds,
    pub tag: Option<String>,
    pub close_shutter_index_value: u8,
    pub convert_color_into_shutter_intents: bool,
    pub open_shutter_index_value: u8,
}

impl Default for ShutterFilterConfig {
    fn default() -> Self {
        ShutterFilterConfig {
            ids: ModuleIds::new(ids::SHUTTER_FILTER, Default::default()),
            tag: None,
            close_shutter_index_value: 0,
            convert_color_into_shutter_intents: false,
            open_shutter_index_value: 0,
        }
    }
}

impl ShutterFilterConfig {
    pub fn from_xml(e: &XElement) -> ShutterFilterConfig {
        ShutterFilterConfig {
            ids: ModuleIds::read(e),
            tag: read_tag(e),
            close_shutter_index_value: xmlutil::get_u8(e, "CloseShutterIndexValue"),
            convert_color_into_shutter_intents: xmlutil::get_bool(e, "ConvertColorIntoShutterIntents"),
            open_shutter_index_value: xmlutil::get_u8(e, "OpenShutterIndexValue"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(SHUTTER_NS);
        let mut e = data_root(&ns, "ShutterFilterData", &self.ids);
        e.push(tag_el(self.tag.as_deref()));
        e.push(num_el(&ns, "CloseShutterIndexValue", self.close_shutter_index_value));
        e.push(bool_el(&ns, "ConvertColorIntoShutterIntents", self.convert_color_into_shutter_intents));
        e.push(num_el(&ns, "OpenShutterIndexValue", self.open_shutter_index_value));
        e
    }
}

/// `FixtureColorWheel`. `UseCurve` is declared on the base class, and DataContractSerializer skips the
/// override, so it appears once with the base members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixtureColorWheelEntry {
    pub name: Option<String>,
    pub end_value: i32,
    /// `FixtureIndexType` member name.
    pub index_type: String,
    pub start_value: i32,
    pub use_curve: bool,
    pub color1_html: Option<String>,
    pub color2_html: Option<String>,
    pub exclude_color_property: bool,
    pub half_step: bool,
}

impl Default for FixtureColorWheelEntry {
    /// The C# constructor: white colours and the colour wheel index type.
    fn default() -> Self {
        FixtureColorWheelEntry {
            name: None,
            end_value: 0,
            index_type: "ColorWheel".to_string(),
            start_value: 0,
            use_curve: false,
            color1_html: Some("White".to_string()),
            color2_html: Some("White".to_string()),
            exclude_color_property: false,
            half_step: false,
        }
    }
}

impl FixtureColorWheelEntry {
    /// `Color1`, from `ColorTranslator.FromHtml`. `None` where the C# would throw.
    pub fn color1(&self) -> Option<DotNetColor> {
        DotNetColor::from_html(self.color1_html.as_deref().unwrap_or(""))
    }

    pub fn set_color1(&mut self, c: &DotNetColor) {
        self.color1_html = Some(c.to_html());
    }

    /// `Color2`, from `ColorTranslator.FromHtml`.
    pub fn color2(&self) -> Option<DotNetColor> {
        DotNetColor::from_html(self.color2_html.as_deref().unwrap_or(""))
    }

    pub fn set_color2(&mut self, c: &DotNetColor) {
        self.color2_html = Some(c.to_html());
    }

    fn from_xml(e: &XElement) -> FixtureColorWheelEntry {
        FixtureColorWheelEntry {
            name: xmlutil::get_opt_string(e, "Name"),
            end_value: xmlutil::get_i32(e, "EndValue"),
            index_type: xmlutil::get_opt_string(e, "IndexType").unwrap_or_default(),
            start_value: xmlutil::get_i32(e, "StartValue"),
            use_curve: xmlutil::get_bool(e, "UseCurve"),
            color1_html: xmlutil::get_opt_string(e, "Color1Html"),
            color2_html: xmlutil::get_opt_string(e, "Color2Html"),
            exclude_color_property: xmlutil::get_bool(e, "ExcludeColorProperty"),
            half_step: xmlutil::get_bool(e, "HalfStep"),
        }
    }

    fn to_xml(&self) -> XElement {
        let ns = contract_ns(FIXTURE_NS);
        let mut e = XElement::new(&ns, "FixtureColorWheel");
        e.push(opt_text_el(&ns, "Name", self.name.as_deref()));
        e.push(num_el(&ns, "EndValue", self.end_value));
        e.push(text_el(&ns, "IndexType", &self.index_type));
        e.push(num_el(&ns, "StartValue", self.start_value));
        e.push(bool_el(&ns, "UseCurve", self.use_curve));
        e.push(opt_text_el(&ns, "Color1Html", self.color1_html.as_deref()));
        e.push(opt_text_el(&ns, "Color2Html", self.color2_html.as_deref()));
        e.push(bool_el(&ns, "ExcludeColorProperty", self.exclude_color_property));
        e.push(bool_el(&ns, "HalfStep", self.half_step));
        e
    }
}

/// `ColorWheelFilterData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorWheelFilterConfig {
    pub ids: ModuleIds,
    pub tag: Option<String>,
    /// Null in a new data object.
    pub color_wheel_data: Option<Vec<FixtureColorWheelEntry>>,
    pub convert_color_intents_into_index_commands: bool,
}

impl Default for ColorWheelFilterConfig {
    fn default() -> Self {
        ColorWheelFilterConfig {
            ids: ModuleIds::new(ids::COLOR_WHEEL_FILTER, Default::default()),
            tag: None,
            color_wheel_data: None,
            convert_color_intents_into_index_commands: false,
        }
    }
}

impl ColorWheelFilterConfig {
    pub fn from_xml(e: &XElement) -> ColorWheelFilterConfig {
        ColorWheelFilterConfig {
            ids: ModuleIds::read(e),
            tag: read_tag(e),
            color_wheel_data: e
                .child("ColorWheelData")
                .filter(|l| !l.nil)
                .map(|l| l.children_named("FixtureColorWheel").map(FixtureColorWheelEntry::from_xml).collect()),
            convert_color_intents_into_index_commands: xmlutil::get_bool(e, "ConvertColorIntentsIntoIndexCommands"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(COLOR_WHEEL_NS);
        let mut e = data_root(&ns, "ColorWheelFilterData", &self.ids);
        e.push(tag_el(self.tag.as_deref()));
        match &self.color_wheel_data {
            None => {
                e.push(XElement::nil(&ns, "ColorWheelData"));
            }
            Some(items) => {
                let mut list = XElement::new(&ns, "ColorWheelData");
                for item in items {
                    list.push(item.to_xml());
                }
                e.push(list);
            }
        }
        e.push(bool_el(&ns, "ConvertColorIntentsIntoIndexCommands", self.convert_color_intents_into_index_commands));
        e
    }
}

#[derive(Clone, Debug)]
struct WheelEntry {
    /// `None` for a colour `FromHtml` couldn't read, which never matches.
    color1: Option<DotNetColor>,
    half_step: bool,
    start_value: u8,
}

/// What makes each filter in the family different.
#[derive(Clone, Debug)]
enum TaggedKind {
    /// `TaggedFilter`.
    Tagged,
    /// `DimmingFilter`.
    Dimming { convert: bool, empty_label: Arc<str> },
    /// `PrismFilter`. `associated` is `AssociatedFunctionName`, empty when null.
    Prism { convert: bool, open: u8, close: u8, associated: Arc<str> },
    /// `ShutterFilter`.
    Shutter { convert: bool, open: u8, close: u8 },
    /// `ColorWheelFilter`.
    ColorWheel { convert: bool, wheel: Vec<WheelEntry> },
}

/// A tagged filter module with its output.
#[derive(Clone, Debug)]
pub(crate) struct TaggedRt {
    tag: Arc<str>,
    kind: TaggedKind,
    /// `IntentData.Value`.
    list: Vec<StateValue>,
}

impl TaggedRt {
    fn with_kind(tag: Option<&str>, kind: TaggedKind) -> TaggedRt {
        let mut rt = TaggedRt { tag: Arc::from(tag.unwrap_or("")), kind, list: Vec::with_capacity(4) };
        // A filter with no source never runs, but reading its output still goes through the getter.
        rt.intent_data_getter();
        rt
    }

    pub(crate) fn tagged(cfg: &TaggedFilterConfig) -> TaggedRt {
        TaggedRt::with_kind(cfg.tag.as_deref(), TaggedKind::Tagged)
    }

    pub(crate) fn dimming(cfg: &DimmingFilterConfig) -> TaggedRt {
        TaggedRt::with_kind(
            cfg.tag.as_deref(),
            TaggedKind::Dimming { convert: cfg.convert_rgb_into_dimming_intents, empty_label: Arc::from("") },
        )
    }

    pub(crate) fn prism(cfg: &PrismFilterConfig) -> TaggedRt {
        TaggedRt::with_kind(
            cfg.tag.as_deref(),
            TaggedKind::Prism {
                convert: cfg.convert_prism_intents_into_open_prism_intents,
                open: cfg.open_prism_index_value,
                close: cfg.close_prism_index_value,
                associated: Arc::from(cfg.associated_function_name.as_deref().unwrap_or("")),
            },
        )
    }

    pub(crate) fn shutter(cfg: &ShutterFilterConfig) -> TaggedRt {
        TaggedRt::with_kind(
            cfg.tag.as_deref(),
            TaggedKind::Shutter {
                convert: cfg.convert_color_into_shutter_intents,
                open: cfg.open_shutter_index_value,
                close: cfg.close_shutter_index_value,
            },
        )
    }

    pub(crate) fn color_wheel(cfg: &ColorWheelFilterConfig) -> TaggedRt {
        let wheel = cfg
            .color_wheel_data
            .iter()
            .flatten()
            .map(|e| WheelEntry { color1: e.color1(), half_step: e.half_step, start_value: e.start_value as u8 })
            .collect();
        TaggedRt::with_kind(
            cfg.tag.as_deref(),
            TaggedKind::ColorWheel { convert: cfg.convert_color_intents_into_index_commands, wheel },
        )
    }

    /// `TaggedFilter.Handle(IIntentState<CommandValue>)`: only a named command with this tag.
    #[inline]
    fn base_command(&self, c: &CommandValue) -> Option<StateValue> {
        match c {
            CommandValue::Named8(n) if *n.tag == *self.tag => Some(StateValue::Command(c.clone())),
            _ => None,
        }
    }

    /// `TaggedFilter.Handle(IIntentState<RangeValue<FunctionIdentity>>)`.
    #[inline]
    fn base_range(&self, r: &RangeValue) -> Option<StateValue> {
        if *r.tag == *self.tag {
            Some(StateValue::Range(r.clone()))
        } else {
            None
        }
    }

    /// `ITaggedFilter.Filter`: the state to add to the output, if this output handles it.
    #[inline]
    fn filter(&self, s: StateRef<'_>) -> Option<StateValue> {
        match &self.kind {
            TaggedKind::Tagged => match s {
                StateRef::Command(c) => self.base_command(c),
                StateRef::Range(r) => self.base_range(r),
                _ => None,
            },
            TaggedKind::Dimming { convert, empty_label } => {
                let dim = |intensity: f64| {
                    // With the conversion off, the dimmer is simply held at 100%.
                    let value = if *convert { intensity } else { 1.0 };
                    Some(StateValue::Range(RangeValue {
                        tag_type: FunctionIdentity::Dim,
                        tag: self.tag.clone(),
                        value,
                        label: empty_label.clone(),
                    }))
                };
                match s {
                    StateRef::Discrete(d) => dim(d.intensity),
                    StateRef::Rgb(r) => dim(r.intensity()),
                    StateRef::Lighting(l) => dim(l.intensity),
                    StateRef::Command(c) => self.base_command(c),
                    StateRef::Range(r) => self.base_range(r),
                }
            }
            TaggedKind::Prism { convert, open, associated, .. } => {
                let handle_intent = || if *convert { Some(StateValue::Command(CommandValue::U8(*open))) } else { None };
                // `A && B && C || D`: with no associated function, every command and range opens the prism.
                let no_function = associated.is_empty();
                match s {
                    StateRef::Command(c) => {
                        let is_prism = matches!(c, CommandValue::Named8(n) if n.index_type == FixtureIndexType::Prism && *n.tag == **associated);
                        if is_prism || no_function {
                            handle_intent()
                        } else {
                            self.base_command(c)
                        }
                    }
                    StateRef::Range(r) => {
                        if (r.tag_type == FunctionIdentity::Prism && *r.tag == **associated) || no_function {
                            handle_intent()
                        } else {
                            self.base_range(r)
                        }
                    }
                    _ => None,
                }
            }
            TaggedKind::Shutter { convert, open, .. } => {
                let handle_intent = || if *convert { Some(StateValue::Command(CommandValue::U8(*open))) } else { None };
                match s {
                    // Any colour opens the shutter, whatever its intensity.
                    StateRef::Discrete(_) | StateRef::Rgb(_) | StateRef::Lighting(_) => handle_intent(),
                    StateRef::Command(c) => {
                        if matches!(c, CommandValue::Named8(n) if n.index_type == FixtureIndexType::ColorWheel) {
                            handle_intent()
                        } else {
                            self.base_command(c)
                        }
                    }
                    StateRef::Range(r) => self.base_range(r),
                }
            }
            TaggedKind::ColorWheel { convert, wheel } => match s {
                StateRef::Discrete(d) if *convert => wheel_command(wheel, d.color),
                StateRef::Rgb(r) if *convert => wheel_command(wheel, r.full_color()),
                StateRef::Lighting(l) if *convert => wheel_command(wheel, l.color),
                StateRef::Command(c) => self.base_command(c),
                StateRef::Range(r) => self.base_range(r),
                _ => None,
            },
        }
    }

    /// The `IntentData` property. Shutter and Prism override its getter with side effects, and the base class
    /// reads it before every change to the list as well as when the output is read.
    ///
    /// - Shutter: when there's more than one state and one is a strobe, everything else goes.
    /// - Both: an empty list gets a close command, if conversion is on.
    #[inline]
    fn intent_data_getter(&mut self) {
        match &self.kind {
            TaggedKind::Shutter { convert, close, .. } => {
                let (convert, close) = (*convert, *close);
                if self.list.len() > 1 {
                    // The C# casts every state to CommandValue here and throws for a range with this
                    // tag, which stops the update thread. A range simply isn't a strobe in this port.
                    let is_strobe = |s: &StateValue| matches!(s, StateValue::Command(CommandValue::Named8(n)) if n.index_type == FixtureIndexType::Strobe);
                    if self.list.iter().any(is_strobe) {
                        self.list.retain(is_strobe);
                    }
                }
                if self.list.is_empty() && convert {
                    self.list.push(StateValue::Command(CommandValue::U8(close)));
                }
            }
            TaggedKind::Prism { convert: true, close, .. } if self.list.is_empty() => {
                self.list.push(StateValue::Command(CommandValue::U8(*close)));
            }
            _ => {}
        }
    }

    /// `TaggedFilterOutputBase.ProcessInputData`.
    ///
    /// Because `IntentData.Value.Add` runs the getter first, the first state added to an empty shutter or
    /// prism list lands after a close command, and with a highest-wins policy a close value above the
    /// open value keeps the fixture closed. Deliberately kept.
    pub(crate) fn update(&mut self, states: Option<States<'_>>) {
        self.list.clear();
        if let Some(states) = states {
            states.for_each(|s| {
                if let Some(handled) = self.filter(s) {
                    self.intent_data_getter();
                    self.list.push(handled);
                }
            });
        }
        // A null list throws in the C# foreach after the clear, in the middle of the frame's filter update. Here
        // the output just stays cleared. Reading the output next applies the getter.
        self.intent_data_getter();
    }

    #[inline]
    pub(crate) fn output(&self) -> Flow<'_> {
        Flow::Intents(States::List(&self.list))
    }
}

/// `ColorWheelFilter.ConvertColorToIndexCommand`: the first whole-step entry whose `Color1` equals the colour.
///
/// `==` on `System.Drawing.Color` compares state too, so an entry saved under a colour name never equals a
/// value colour. `vixen_core::Color` has no such state, so an incoming colour is taken to be a value colour,
/// which is what `RGBValue.FullColor` always is and what discrete colours are built from.
fn wheel_command(wheel: &[WheelEntry], color: Color) -> Option<StateValue> {
    let color = if color == Color::EMPTY { DotNetColor::EMPTY } else { DotNetColor::from_color(color) };
    wheel
        .iter()
        .find(|e| !e.half_step && e.color1.as_ref() == Some(&color))
        .map(|e| StateValue::Command(CommandValue::U8(e.start_value)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::value::NamedCommand;
    use vixen_core::{CombinedState, DiscreteValue, LightingValue, RgbValue};

    fn named(value: u8, tag: &str, index_type: FixtureIndexType) -> StateValue {
        StateValue::Command(CommandValue::Named8(NamedCommand {
            value,
            index_type,
            tag: Arc::from(tag),
            label: Arc::from(""),
            range_minimum: 0,
            range_maximum: 0,
        }))
    }

    fn range(id: FunctionIdentity, tag: &str, value: f64) -> StateValue {
        StateValue::Range(RangeValue::new(id, tag, value, ""))
    }

    fn cmd(v: u8) -> StateValue {
        StateValue::Command(CommandValue::U8(v))
    }

    fn rgb(r: u8, g: u8, b: u8) -> StateValue {
        StateValue::Rgb(RgbValue { r, g, b })
    }

    fn run(rt: &mut TaggedRt, states: &[StateValue]) -> Vec<StateValue> {
        rt.update(Some(States::List(states)));
        rt.list.clone()
    }

    fn tagged(tag: &str) -> TaggedRt {
        TaggedRt::tagged(&TaggedFilterConfig { tag: Some(tag.to_string()), ..Default::default() })
    }

    #[test]
    fn tagged_filter_passes_only_matching_ranges_and_named_commands() {
        let mut rt = tagged("Pan");
        let a = range(FunctionIdentity::Pan, "Pan", 0.5);
        let b = range(FunctionIdentity::Tilt, "Tilt", 0.5);
        let c = named(4, "Pan", FixtureIndexType::Custom);
        let d = named(4, "Gobo", FixtureIndexType::Custom);
        let out = run(&mut rt, &[a.clone(), b, c.clone(), d, cmd(1), rgb(255, 0, 0)]);
        assert_eq!(out, vec![a, c]);
    }

    #[test]
    fn tagged_filter_clears_between_frames_and_starts_empty() {
        let mut rt = tagged("Pan");
        assert!(rt.list.is_empty());
        run(&mut rt, &[range(FunctionIdentity::Pan, "Pan", 0.5)]);
        assert!(run(&mut rt, &[]).is_empty());
    }

    #[test]
    fn a_null_tag_only_matches_an_empty_tag() {
        let mut rt = TaggedRt::tagged(&TaggedFilterConfig::default());
        let a = range(FunctionIdentity::Pan, "", 0.5);
        assert_eq!(run(&mut rt, &[a.clone(), range(FunctionIdentity::Pan, "Pan", 0.5)]), vec![a]);
    }

    fn dimming(convert: bool) -> TaggedRt {
        TaggedRt::dimming(&DimmingFilterConfig {
            tag: Some("Dim".to_string()),
            convert_rgb_into_dimming_intents: convert,
            ..Default::default()
        })
    }

    #[test]
    fn dimming_filter_converts_colour_intensity() {
        let mut rt = dimming(true);
        let dim = |v| range(FunctionIdentity::Dim, "Dim", v);
        assert_eq!(run(&mut rt, &[StateValue::Lighting(LightingValue::new(Color::RED, 0.25))]), vec![dim(0.25)]);
        assert_eq!(run(&mut rt, &[rgb(51, 0, 0)]), vec![dim(0.2)]);
        assert_eq!(run(&mut rt, &[StateValue::Discrete(DiscreteValue::new(Color::BLUE, 0.75))]), vec![dim(0.75)]);
    }

    #[test]
    fn dimming_filter_holds_full_without_conversion() {
        let mut rt = dimming(false);
        let dim = |v| range(FunctionIdentity::Dim, "Dim", v);
        assert_eq!(run(&mut rt, &[StateValue::Lighting(LightingValue::new(Color::RED, 0.25))]), vec![dim(1.0)]);
        assert_eq!(run(&mut rt, &[rgb(0, 0, 0)]), vec![dim(1.0)]);
    }

    #[test]
    fn dimming_filter_still_passes_its_own_tag() {
        let mut rt = dimming(true);
        let a = range(FunctionIdentity::Dim, "Dim", 0.1);
        assert_eq!(run(&mut rt, &[a.clone(), range(FunctionIdentity::Pan, "Pan", 1.0)]), vec![a]);
    }

    fn prism(convert: bool, associated: Option<&str>) -> TaggedRt {
        TaggedRt::prism(&PrismFilterConfig {
            tag: Some("OpenPrism".to_string()),
            associated_function_name: associated.map(str::to_string),
            convert_prism_intents_into_open_prism_intents: convert,
            open_prism_index_value: 200,
            close_prism_index_value: 10,
            ..Default::default()
        })
    }

    #[test]
    fn prism_closes_when_idle_and_only_when_converting() {
        let mut rt = prism(true, Some("Prism"));
        assert_eq!(rt.list, vec![cmd(10)]);
        assert_eq!(run(&mut rt, &[]), vec![cmd(10)]);
        let mut rt = prism(false, Some("Prism"));
        assert!(rt.list.is_empty());
        assert!(run(&mut rt, &[]).is_empty());
    }

    #[test]
    fn prism_intent_adds_open_after_the_close_command() {
        let mut rt = prism(true, Some("Prism"));
        let out = run(&mut rt, &[named(1, "Prism", FixtureIndexType::Prism)]);
        assert_eq!(out, vec![cmd(10), cmd(200)]);
        let out = run(&mut rt, &[range(FunctionIdentity::Prism, "Prism", 0.5)]);
        assert_eq!(out, vec![cmd(10), cmd(200)]);
    }

    #[test]
    fn prism_without_an_associated_function_opens_on_any_command_or_range() {
        let mut rt = prism(true, None);
        let out = run(&mut rt, &[range(FunctionIdentity::Pan, "Pan", 0.5)]);
        assert_eq!(out, vec![cmd(10), cmd(200)]);
        // The named-command check short-circuits to true, so it doesn't matter what the command is.
        let out = run(&mut rt, &[cmd(1)]);
        assert_eq!(out, vec![cmd(10), cmd(200)]);
    }

    #[test]
    fn prism_falls_back_to_the_tag_for_other_functions() {
        let mut rt = prism(true, Some("Prism"));
        let own = named(5, "OpenPrism", FixtureIndexType::Custom);
        assert_eq!(run(&mut rt, &[own.clone()]), vec![cmd(10), own]);
        assert_eq!(run(&mut rt, &[named(5, "Other", FixtureIndexType::Prism)]), vec![cmd(10)]);
    }

    #[test]
    fn prism_ignores_colours() {
        let mut rt = prism(true, Some("Prism"));
        assert_eq!(run(&mut rt, &[rgb(255, 255, 255)]), vec![cmd(10)]);
    }

    fn shutter(convert: bool) -> TaggedRt {
        TaggedRt::shutter(&ShutterFilterConfig {
            tag: Some("Shutter".to_string()),
            convert_color_into_shutter_intents: convert,
            open_shutter_index_value: 255,
            close_shutter_index_value: 0,
            ..Default::default()
        })
    }

    #[test]
    fn shutter_closes_when_idle() {
        let mut rt = shutter(true);
        assert_eq!(rt.list, vec![cmd(0)]);
        assert_eq!(run(&mut rt, &[]), vec![cmd(0)]);
        assert!(shutter(false).list.is_empty());
    }

    #[test]
    fn any_colour_opens_the_shutter_even_at_zero_intensity() {
        let mut rt = shutter(true);
        assert_eq!(run(&mut rt, &[rgb(0, 0, 0)]), vec![cmd(0), cmd(255)]);
        assert_eq!(run(&mut rt, &[StateValue::Discrete(DiscreteValue::new(Color::RED, 0.0))]), vec![cmd(0), cmd(255)]);
        assert_eq!(run(&mut rt, &[StateValue::Lighting(LightingValue::new(Color::RED, 0.0))]), vec![cmd(0), cmd(255)]);
    }

    #[test]
    fn shutter_needs_conversion_on_to_respond_to_colour() {
        let mut rt = shutter(false);
        assert!(run(&mut rt, &[rgb(255, 0, 0)]).is_empty());
    }

    #[test]
    fn spinning_the_colour_wheel_opens_the_shutter() {
        let mut rt = shutter(true);
        let wheel = named(9, "Wheel", FixtureIndexType::ColorWheel);
        assert_eq!(run(&mut rt, &[wheel]), vec![cmd(0), cmd(255)]);
    }

    #[test]
    fn a_strobe_replaces_everything_else_on_the_shutter() {
        let mut rt = shutter(true);
        let strobe = named(77, "Shutter", FixtureIndexType::Strobe);
        // Alone: the close command is added first, then the getter removes it.
        assert_eq!(run(&mut rt, std::slice::from_ref(&strobe)), vec![strobe.clone()]);
        // With a colour, in either order.
        assert_eq!(run(&mut rt, &[strobe.clone(), rgb(255, 0, 0)]), vec![strobe.clone()]);
        assert_eq!(run(&mut rt, &[rgb(255, 0, 0), strobe.clone()]), vec![strobe.clone()]);
    }

    #[test]
    fn shutter_keeps_a_tagged_non_strobe_command_next_to_the_close() {
        let mut rt = shutter(true);
        let open = named(50, "Shutter", FixtureIndexType::ShutterOpen);
        assert_eq!(run(&mut rt, std::slice::from_ref(&open)), vec![cmd(0), open]);
    }

    fn wheel_config(convert: bool) -> ColorWheelFilterConfig {
        let entry = |html: &str, start: i32, half: bool| FixtureColorWheelEntry {
            color1_html: Some(html.to_string()),
            start_value: start,
            half_step: half,
            ..Default::default()
        };
        ColorWheelFilterConfig {
            tag: Some("Wheel".to_string()),
            color_wheel_data: Some(vec![
                entry("#FF0000", 10, true),
                entry("#FF0000", 20, false),
                entry("#00FF00", 300, false),
                entry("Blue", 40, false),
                entry("#FF0000", 50, false),
            ]),
            convert_color_intents_into_index_commands: convert,
            ..Default::default()
        }
    }

    #[test]
    fn colour_wheel_finds_the_first_whole_step_entry() {
        let mut rt = TaggedRt::color_wheel(&wheel_config(true));
        assert_eq!(run(&mut rt, &[rgb(255, 0, 0)]), vec![cmd(20)]);
        assert_eq!(run(&mut rt, &[StateValue::Discrete(DiscreteValue::new(Color::RED, 0.1))]), vec![cmd(20)]);
        assert_eq!(run(&mut rt, &[StateValue::Lighting(LightingValue::new(Color::RED, 0.1))]), vec![cmd(20)]);
    }

    #[test]
    fn colour_wheel_start_value_wraps_to_a_byte() {
        let mut rt = TaggedRt::color_wheel(&wheel_config(true));
        assert_eq!(run(&mut rt, &[rgb(0, 255, 0)]), vec![cmd(44)]);
    }

    #[test]
    fn colour_wheel_entry_saved_by_name_never_equals_a_value_colour() {
        let mut rt = TaggedRt::color_wheel(&wheel_config(true));
        assert!(run(&mut rt, &[rgb(0, 0, 255)]).is_empty());
    }

    #[test]
    fn colour_wheel_ignores_unknown_colours_and_brightness_changes_the_rgb_match() {
        let mut rt = TaggedRt::color_wheel(&wheel_config(true));
        assert!(run(&mut rt, &[rgb(1, 2, 3)]).is_empty());
        // RGB compares the brightness-applied colour; lighting compares the base colour.
        assert!(run(&mut rt, &[rgb(128, 0, 0)]).is_empty());
        assert_eq!(run(&mut rt, &[StateValue::Lighting(LightingValue::new(Color::RED, 0.5))]), vec![cmd(20)]);
    }

    #[test]
    fn colour_wheel_without_conversion_only_passes_its_tag() {
        let mut rt = TaggedRt::color_wheel(&wheel_config(false));
        assert!(run(&mut rt, &[rgb(255, 0, 0)]).is_empty());
        let own = named(5, "Wheel", FixtureIndexType::ColorWheel);
        assert_eq!(run(&mut rt, std::slice::from_ref(&own)), vec![own]);
    }

    #[test]
    fn null_wheel_data_never_matches() {
        let cfg = ColorWheelFilterConfig { convert_color_intents_into_index_commands: true, ..Default::default() };
        let mut rt = TaggedRt::color_wheel(&cfg);
        assert!(run(&mut rt, &[rgb(255, 0, 0)]).is_empty());
    }

    #[test]
    fn reading_element_states_works_like_a_list() {
        let mut rt = shutter(true);
        let mut s = CombinedState::default();
        s.rgb = Some(RgbValue { r: 1, g: 1, b: 1 });
        rt.update(Some(States::Element(&s)));
        assert_eq!(rt.list, vec![cmd(0), cmd(255)]);
    }

    #[test]
    fn a_null_list_leaves_the_output_cleared() {
        let mut rt = shutter(true);
        run(&mut rt, &[rgb(255, 0, 0)]);
        rt.update(None);
        assert_eq!(rt.list, vec![cmd(0)]);
    }
}
