//! Port of Vixen.Modules/OutputFilter/DimmingCurve.

use crate::command::OutputCommand;
use crate::dotnet::{byte_from_f64, format_double, short_from_f64};
use crate::flow::{Flow, StateRef, States};
use crate::ids;
use crate::xmlutil::{self, bool_el, data_root, opt_text_el, ModuleIds};
use std::sync::Arc;
use vixen_core::{
    CombinedState, CommandValue, Curve, DiscreteValue, FunctionIdentity, Hsv, LightingValue, RangeValue, RgbValue, StateValue,
};
use vixen_xml::{contract_ns, XElement, XSD_NS};

const CLR_NS: &str = "VixenModules.OutputFilter.DimmingCurve";
const CURVES_NS: &str = "VixenModules.App.Curves";
const ZEDGRAPH_NS: &str = "ZedGraph";

/// One serialized `ZedGraph.PointPair`.
#[derive(Clone, Debug, PartialEq)]
pub struct PointPairData {
    pub schema: i32,
    pub x: f64,
    pub y: f64,
    pub schema2: i32,
    pub z: f64,
    /// `Tag`, which is an arbitrary object. Curves leave it null. Anything else is kept as written.
    pub tag: Option<XElement>,
}

impl PointPairData {
    /// `new PointPair(x, y)`. Both schema fields are 11 in the ZedGraph source.
    pub fn new(x: f64, y: f64) -> PointPairData {
        PointPairData { schema: 11, x, y, schema2: 11, z: 0.0, tag: None }
    }
}

/// `VixenModules.App.Curves.Curve` as a data contract.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveData {
    pub is_current_library_curve: bool,
    pub points: Vec<PointPairData>,
    pub library_reference_name: Option<String>,
}

impl Default for CurveData {
    /// `new Curve()`: a ramp with x = y. A new curve's `_libraryReferenceName` is `string.Empty`.
    fn default() -> Self {
        CurveData::from_points(&[(0.0, 0.0), (100.0, 100.0)])
    }
}

impl CurveData {
    pub fn from_points(points: &[(f64, f64)]) -> CurveData {
        CurveData {
            is_current_library_curve: false,
            points: points.iter().map(|p| PointPairData::new(p.0, p.1)).collect(),
            library_reference_name: Some(String::new()),
        }
    }

    /// The curve the filter evaluates. `Curve.Points` also consults the curve library for a reference, but
    /// Vixen writes the library's points back into this data whenever it saves, so these are current.
    pub fn to_curve(&self) -> Curve {
        let mut c = Curve::from_points(self.points.iter().map(|p| (p.x, p.y)).collect());
        c.library_reference_name = self.library_reference_name.clone().unwrap_or_default();
        c.is_current_library_curve = self.is_current_library_curve;
        c
    }

    pub(crate) fn from_xml(e: &XElement) -> CurveData {
        let mut points = Vec::new();
        if let Some(list) = e.child("Points") {
            for p in list.children_named("PointPair") {
                let num = |name: &str| p.child_text(name).and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0);
                let int = |name: &str| p.child_text(name).and_then(|t| t.parse::<i32>().ok()).unwrap_or(0);
                points.push(PointPairData {
                    schema: int("schema"),
                    x: num("X"),
                    y: num("Y"),
                    schema2: int("schema2"),
                    z: num("Z"),
                    tag: p.child("Tag").filter(|t| !t.nil).cloned(),
                });
            }
        }
        CurveData {
            is_current_library_curve: xmlutil::get_bool(e, "IsCurrentLibraryCurve"),
            points,
            library_reference_name: xmlutil::get_opt_string(e, "_libraryReferenceName"),
        }
    }

    pub(crate) fn to_xml(&self, ns: &str, local: &str) -> XElement {
        let curves = contract_ns(CURVES_NS);
        let zed = contract_ns(ZEDGRAPH_NS);
        let mut e = XElement::new(ns, local);
        e.push(bool_el(&curves, "IsCurrentLibraryCurve", self.is_current_library_curve));
        let mut list = XElement::new(&curves, "Points");
        // PointPair is ISerializable, so its members are unqualified and carry their own xsi:type.
        for p in &self.points {
            let typed = |name: &str, ty: &str, text: String| {
                let mut x = XElement::with_text("", name, text);
                x.xsi_type = Some((XSD_NS.to_string(), ty.to_string()));
                x
            };
            let mut pair = XElement::new(&zed, "PointPair");
            pair.push(typed("schema", "int", p.schema.to_string()));
            pair.push(typed("X", "double", format_double(p.x)));
            pair.push(typed("Y", "double", format_double(p.y)));
            pair.push(typed("schema2", "int", p.schema2.to_string()));
            pair.push(typed("Z", "double", format_double(p.z)));
            pair.push(p.tag.clone().unwrap_or_else(|| XElement::nil("", "Tag")));
            list.push(pair);
        }
        e.push(list);
        e.push(opt_text_el(&curves, "_libraryReferenceName", self.library_reference_name.as_deref()));
        e
    }
}

/// `DimmingCurveData`.
#[derive(Clone, Debug, PartialEq)]
pub struct DimmingCurveConfig {
    pub ids: ModuleIds,
    pub curve: CurveData,
}

impl Default for DimmingCurveConfig {
    fn default() -> Self {
        DimmingCurveConfig { ids: ModuleIds::new(ids::DIMMING_CURVE, Default::default()), curve: CurveData::default() }
    }
}

impl DimmingCurveConfig {
    pub fn from_xml(e: &XElement) -> DimmingCurveConfig {
        DimmingCurveConfig {
            ids: ModuleIds::read(e),
            curve: e.child("Curve").filter(|c| !c.nil).map(CurveData::from_xml).unwrap_or_default(),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "DimmingCurveData", &self.ids);
        e.push(self.curve.to_xml(&ns, "Curve"));
        e
    }
}

/// `DimmingCurveFilter._intentValue`. The field is never cleared before a dispatch, and one handler leaves it
/// alone, so what it last held matters. The lighting and RGB states are one reused object each, so a list that
/// holds one of those twice shows the newest value in both places.
#[derive(Clone, Debug)]
enum IntentValue {
    Null,
    LvState,
    RgbState,
    Value(StateValue),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum InternalData {
    Null,
    Intents,
    Command,
}

/// `DimmingCurveModule` with its one output.
#[derive(Clone, Debug)]
pub(crate) struct DimmingCurveRt {
    curve: Curve,
    empty_label: Arc<str>,
    lv_state: LightingValue,
    rgb_state: RgbValue,
    intent_value: IntentValue,
    /// `_states`, which is `_intentData.Value` unless `intents_null`.
    states: Vec<StateValue>,
    /// Positions in `states` that alias the reused lighting and RGB objects.
    lv_refs: Vec<u32>,
    rgb_refs: Vec<u32>,
    intents_null: bool,
    command_state: OutputCommand,
    internal: InternalData,
}

impl DimmingCurveRt {
    pub(crate) fn new(cfg: &DimmingCurveConfig) -> DimmingCurveRt {
        DimmingCurveRt {
            curve: cfg.curve.to_curve(),
            empty_label: Arc::from(""),
            lv_state: LightingValue::default(),
            rgb_state: RgbValue::default(),
            intent_value: IntentValue::Null,
            states: Vec::with_capacity(8),
            lv_refs: Vec::with_capacity(4),
            rgb_refs: Vec::with_capacity(4),
            // `new IntentsDataFlowData(empty list)`, though `InternalData` starts null.
            intents_null: false,
            command_state: OutputCommand::U8(0),
            internal: InternalData::Null,
        }
    }

    #[inline]
    fn sample(&self, intensity: f64) -> f64 {
        self.curve.get_value(intensity * 100.0) / 100.0
    }

    /// `DimmingCurveFilter.Filter(IIntentState)`.
    #[inline]
    fn filter_state(&mut self, s: StateRef<'_>) {
        match s {
            StateRef::Lighting(l) => {
                if l.intensity > 0.0 {
                    // `new LightingValue(lv, i)` doesn't clamp, unlike the (color, i) constructor.
                    self.lv_state = LightingValue { color: l.color, intensity: self.sample(l.intensity) };
                    self.intent_value = IntentValue::LvState;
                } else {
                    self.intent_value = IntentValue::Null;
                }
            }
            StateRef::Rgb(r) => {
                let i = r.intensity();
                if i > 0.0 {
                    let new_intensity = self.sample(i);
                    let mut hsv = Hsv::from_rgb(r.r as f64, r.g as f64, r.b as f64);
                    hsv.set_v(new_intensity);
                    self.rgb_state = RgbValue::new(hsv.to_color());
                    self.intent_value = IntentValue::RgbState;
                } else {
                    self.intent_value = IntentValue::Null;
                }
            }
            StateRef::Discrete(d) => {
                self.intent_value = if d.intensity > 0.0 {
                    // Set straight into the state's value, so no clip here either.
                    IntentValue::Value(StateValue::Discrete(DiscreteValue {
                        color: d.color,
                        intensity: self.sample(d.intensity),
                    }))
                } else {
                    IntentValue::Null
                };
            }
            StateRef::Range(r) => {
                self.intent_value = if r.tag_type == FunctionIdentity::Dim {
                    IntentValue::Value(StateValue::Range(RangeValue {
                        tag_type: FunctionIdentity::Dim,
                        tag: r.tag.clone(),
                        value: self.sample(r.value),
                        label: self.empty_label.clone(),
                    }))
                } else {
                    IntentValue::Null
                };
            }
            StateRef::Command(c) => {
                // Divides by 255 even though the command is 16-bit, and goes through a short.
                // A command of any other type leaves `_intentValue` as it was, so whatever the previous state
                // produced is added to the list again.
                if let CommandValue::U16(v) = c {
                    let new_intensity = self.curve.get_value(*v as f64 / 255.0 * 100.0) / 100.0 * 65535.0;
                    self.intent_value =
                        IntentValue::Value(StateValue::Command(CommandValue::U16(short_from_f64(new_intensity) as u16)));
                }
            }
        }
    }

    /// Appends the current `_intentValue` to the list.
    #[inline]
    fn push_intent_value(&mut self) {
        let index = self.states.len() as u32;
        match &self.intent_value {
            IntentValue::Null => {}
            IntentValue::LvState => {
                self.lv_refs.push(index);
                self.states.push(StateValue::Lighting(self.lv_state));
            }
            IntentValue::RgbState => {
                self.rgb_refs.push(index);
                self.states.push(StateValue::Rgb(self.rgb_state));
            }
            IntentValue::Value(v) => self.states.push(v.clone()),
        }
    }

    #[inline]
    fn begin_list(&mut self) {
        self.states.clear();
        self.lv_refs.clear();
        self.rgb_refs.clear();
    }

    /// Entries that alias the reused objects read their final value.
    #[inline]
    fn finish_list(&mut self) {
        for &i in &self.lv_refs {
            self.states[i as usize] = StateValue::Lighting(self.lv_state);
        }
        for &i in &self.rgb_refs {
            self.states[i as usize] = StateValue::Rgb(self.rgb_state);
        }
    }

    /// `ProcessInputData(IntentsDataFlowData)`.
    pub(crate) fn update_intents(&mut self, states: Option<States<'_>>) {
        self.internal = InternalData::Intents;
        match states {
            Some(states) if !states.is_empty() => {
                self.begin_list();
                states.for_each(|s| {
                    self.filter_state(s);
                    self.push_intent_value();
                });
                self.finish_list();
                self.intents_null = false;
            }
            _ => self.intents_null = true,
        }
    }

    /// `ProcessInputData` for an element whose discrete states other consumers also read.
    ///
    /// The C# writes the new discrete intensity into the state object it was given, and every intent in
    /// a frame is a `StaticIntentState`, so anything else reading that element later in the frame sees the
    /// curve already applied.
    pub(crate) fn update_element_in_place(&mut self, element: &CombinedState, discrete: &mut [DiscreteValue]) {
        self.internal = InternalData::Intents;
        if element.is_empty() {
            self.intents_null = true;
            return;
        }
        self.begin_list();
        for s in element.other.iter() {
            self.filter_state(s.into());
            self.push_intent_value();
        }
        if let Some(l) = element.lighting {
            self.filter_state(StateRef::Lighting(l));
            self.push_intent_value();
        }
        if let Some(r) = element.rgb {
            self.filter_state(StateRef::Rgb(r));
            self.push_intent_value();
        }
        for d in discrete.iter_mut() {
            self.filter_state(StateRef::Discrete(*d));
            if let IntentValue::Value(StateValue::Discrete(new_value)) = &self.intent_value {
                d.intensity = new_value.intensity;
            }
            self.push_intent_value();
        }
        self.finish_list();
        self.intents_null = false;
    }

    /// `ProcessInputData(CommandDataFlowData)`.
    pub(crate) fn update_command(&mut self, command: &OutputCommand) {
        self.internal = InternalData::Command;
        self.command_state = match command {
            OutputCommand::None => OutputCommand::U8(0),
            other => self.filter_command(other),
        };
    }

    /// `DimmingCurveFilter.Filter(ICommand)`.
    #[inline]
    fn filter_command(&self, command: &OutputCommand) -> OutputCommand {
        match command {
            OutputCommand::U8(v) => {
                let new_intensity = self.curve.get_value(*v as f64 / 255.0 * 100.0) / 100.0 * 255.0;
                OutputCommand::U8(byte_from_f64(new_intensity))
            }
            OutputCommand::U16(v) => {
                // Through a short and back, which for a whole-number result leaves the low 16 bits intact.
                let new_intensity = self.curve.get_value(*v as f64 / 65535.0 * 100.0) / 100.0 * 65535.0;
                OutputCommand::U16(short_from_f64(new_intensity) as u16)
            }
            other => other.clone(),
        }
    }

    /// `IDataFlowOutput.Data`, which is `InternalData`.
    #[inline]
    pub(crate) fn output(&self) -> Flow<'_> {
        match self.internal {
            InternalData::Null => Flow::Null,
            InternalData::Command => Flow::Command(&self.command_state),
            InternalData::Intents => {
                if self.intents_null {
                    Flow::NullIntents
                } else {
                    Flow::Intents(States::List(&self.states))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{Color, NamedCommand};

    /// x maps to x / 2, so a value's change is visible.
    fn half() -> DimmingCurveRt {
        let cfg = DimmingCurveConfig { curve: CurveData::from_points(&[(0.0, 0.0), (100.0, 50.0)]), ..Default::default() };
        DimmingCurveRt::new(&cfg)
    }

    fn list(rt: &DimmingCurveRt) -> Vec<StateValue> {
        match rt.output() {
            Flow::Intents(States::List(l)) => l.to_vec(),
            _ => panic!("expected an intent list"),
        }
    }

    fn run(rt: &mut DimmingCurveRt, states: &[StateValue]) -> Vec<StateValue> {
        rt.update_intents(Some(States::List(states)));
        list(rt)
    }

    #[test]
    fn output_data_is_null_until_it_first_runs() {
        assert!(matches!(half().output(), Flow::Null));
    }

    #[test]
    fn no_input_states_gives_a_null_list() {
        let mut rt = half();
        rt.update_intents(Some(States::List(&[])));
        assert!(matches!(rt.output(), Flow::NullIntents));
        rt.update_intents(None);
        assert!(matches!(rt.output(), Flow::NullIntents));
    }

    #[test]
    fn lighting_intensity_goes_through_the_curve_and_keeps_its_colour() {
        let mut rt = half();
        let out = run(&mut rt, &[StateValue::Lighting(LightingValue::new(Color::RED, 0.5))]);
        assert_eq!(out, vec![StateValue::Lighting(LightingValue { color: Color::RED, intensity: 0.25 })]);
    }

    #[test]
    fn zero_intensity_states_are_dropped() {
        let mut rt = half();
        assert!(run(&mut rt, &[StateValue::Lighting(LightingValue::new(Color::RED, 0.0))]).is_empty());
        assert!(run(&mut rt, &[StateValue::Rgb(RgbValue::BLACK)]).is_empty());
        assert!(run(&mut rt, &[StateValue::Discrete(DiscreteValue::new(Color::RED, 0.0))]).is_empty());
    }

    #[test]
    fn rgb_is_rescaled_through_hsv() {
        let mut rt = half();
        let out = run(&mut rt, &[StateValue::Rgb(RgbValue { r: 255, g: 0, b: 0 })]);
        assert_eq!(out, vec![StateValue::Rgb(RgbValue { r: 127, g: 0, b: 0 })]);
        let out = run(&mut rt, &[StateValue::Rgb(RgbValue { r: 200, g: 100, b: 0 })]);
        // V = 200/255 goes through the curve, and the hue and saturation stay.
        let hsv = Hsv::from_rgb(200.0, 100.0, 0.0);
        let mut h = hsv;
        h.set_v(100.0 * (200.0 / 255.0) / 2.0 / 100.0);
        assert_eq!(out, vec![StateValue::Rgb(RgbValue::new(h.to_color()))]);
    }

    #[test]
    fn discrete_intensity_goes_through_the_curve() {
        let mut rt = half();
        let out = run(&mut rt, &[StateValue::Discrete(DiscreteValue::new(Color::BLUE, 1.0))]);
        assert_eq!(out, vec![StateValue::Discrete(DiscreteValue::new(Color::BLUE, 0.5))]);
    }

    #[test]
    fn only_dim_ranges_pass_and_they_lose_their_label() {
        let mut rt = half();
        let out = run(&mut rt, &[StateValue::Range(RangeValue::new(FunctionIdentity::Dim, "Dimmer", 1.0, "label"))]);
        assert_eq!(out, vec![StateValue::Range(RangeValue::new(FunctionIdentity::Dim, "Dimmer", 0.5, ""))]);
        let out = run(&mut rt, &[StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "Pan", 1.0, ""))]);
        assert!(out.is_empty());
    }

    #[test]
    fn a_16_bit_command_state_uses_the_255_divisor() {
        let mut rt = DimmingCurveRt::new(&DimmingCurveConfig::default());
        // 51 / 255 * 100 = 20, through a ramp stays 20, / 100 * 65535 = 13107.
        let out = run(&mut rt, &[StateValue::Command(CommandValue::U16(51))]);
        assert_eq!(out, vec![StateValue::Command(CommandValue::U16(13107))]);
        // 255 and above read as the top of the curve.
        let out = run(&mut rt, &[StateValue::Command(CommandValue::U16(1000))]);
        assert_eq!(out, vec![StateValue::Command(CommandValue::U16(65535))]);
    }

    #[test]
    fn a_non_16_bit_command_state_repeats_the_previous_result() {
        let mut rt = half();
        let named = StateValue::Command(CommandValue::Named8(NamedCommand {
            value: 9,
            index_type: Default::default(),
            tag: Arc::from("Gobo"),
            label: Arc::from(""),
            range_minimum: 0,
            range_maximum: 0,
        }));
        // Nothing has set `_intentValue` yet, so the command is dropped.
        assert!(run(&mut rt, std::slice::from_ref(&named)).is_empty());

        let dim = StateValue::Range(RangeValue::new(FunctionIdentity::Dim, "Dimmer", 1.0, ""));
        let expected = StateValue::Range(RangeValue::new(FunctionIdentity::Dim, "Dimmer", 0.5, ""));
        assert_eq!(run(&mut rt, std::slice::from_ref(&dim)), vec![expected.clone()]);
        assert_eq!(run(&mut rt, std::slice::from_ref(&named)), vec![expected.clone()]);

        // A range that isn't a dimmer clears it again.
        let pan = StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "Pan", 1.0, ""));
        assert!(run(&mut rt, &[pan, named]).is_empty());
    }

    #[test]
    fn a_repeated_rgb_state_shows_the_newest_value_because_it_is_one_object() {
        let mut rt = half();
        run(&mut rt, &[StateValue::Rgb(RgbValue { r: 255, g: 0, b: 0 })]);
        let named = StateValue::Command(CommandValue::U8(1));
        let out = run(&mut rt, &[named, StateValue::Rgb(RgbValue { r: 0, g: 255, b: 0 })]);
        let green = StateValue::Rgb(RgbValue { r: 0, g: 127, b: 0 });
        assert_eq!(out, vec![green.clone(), green]);
    }

    #[test]
    fn command_flow_uses_the_curve_and_null_becomes_zero() {
        let mut rt = half();
        rt.update_command(&OutputCommand::U8(255));
        assert!(matches!(rt.output(), Flow::Command(OutputCommand::U8(127))));
        rt.update_command(&OutputCommand::U16(65535));
        assert!(matches!(rt.output(), Flow::Command(OutputCommand::U16(32767))));
        rt.update_command(&OutputCommand::None);
        assert!(matches!(rt.output(), Flow::Command(OutputCommand::U8(0))));
        rt.update_command(&OutputCommand::U32(77));
        assert!(matches!(rt.output(), Flow::Command(OutputCommand::U32(77))));
    }

    #[test]
    fn a_16_bit_command_above_32767_survives_the_short() {
        let mut rt = DimmingCurveRt::new(&DimmingCurveConfig::default());
        rt.update_command(&OutputCommand::U16(65535));
        assert!(matches!(rt.output(), Flow::Command(OutputCommand::U16(65535))));
        rt.update_command(&OutputCommand::U16(40000));
        assert!(matches!(rt.output(), Flow::Command(OutputCommand::U16(40000))));
    }

    #[test]
    fn curve_xml_round_trips() {
        let cfg = DimmingCurveConfig {
            curve: CurveData::from_points(&[(0.0, 0.0), (33.3, 12.5), (100.0, 100.0)]),
            ..Default::default()
        };
        let xml = cfg.to_xml();
        let back = DimmingCurveConfig::from_xml(&xml);
        assert_eq!(back, cfg);
        let reparsed = XElement::parse(&xml.to_document()).unwrap();
        assert!(reparsed.semantic_eq(&xml), "{:?}", reparsed.first_difference(&xml, String::new()));
    }
}
