//! Compares every effect's Rust output with what the original C# produced.
//!
//! `tools/golden` renders the original Vixen effects through the real Vixen runtime and writes `tests/golden/*.json.gz`. Each
//! case records the exact parameters and targets it used. This test rebuilds the same element tree, renders the same
//! parameters with the Rust `EffectDescriptor`, and compares the evaluated intent states. Every case renders in both the ways
//! a real one does: the instance being edited, set up through the property setters, and the one a saved sequence loads, which
//! has been through the file format.
//!
//! * `check: "exact"`: the C# uses no random numbers here. Every element at every time must match: exact RGB bytes, exact
//!   lighting colour bytes, intensity within 1e-6, and the same intents with the same start and end ticks.
//! * `check: "stats"`: the C# uses random numbers here. Statistics over many renders must agree: intent counts, timing, which
//!   elements light and when, level envelopes, value ranges and distributions. Then `seeded` holds one more render, with the
//!   C# reading a random stream that reproduces what `fastrand` gives the Rust port, compared like an exact case. That only
//!   works where both draw their numbers in the same order, so it is the stronger of the two.
//! * `check: "threw"`: the C# threw while rendering, so the Rust must not panic. If the C# had built some intents by then
//!   (basic effects build as they go) those are compared like an exact case; otherwise the Rust must render nothing.
//!
//! Environment variables, for working on one effect:
//! * `GOLDEN_CASE=text`: only cases whose id contains `text`.
//! * `GOLDEN_MAX=n`: how many mismatches to print per case (default 3).
//!
//! Effects in `REPORT_ONLY` belong to crates still being worked on: their mismatches print but do not fail.

use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;
use vixen_core::color::{Hsv, Rgb, Xyz};
use vixen_core::element::{property_ids, ElementColorType, Orientation, PropertyData, PropertyInstance};
use vixen_core::gradient::AlphaPoint;
use vixen_core::layer::{ChromaKeySettings, CombinedState, Layer, LayeredState, MixingFilter, SequenceLayers, StateCombinator};
use vixen_core::value::{CommandValue, DiscreteValue, LightingValue, RgbValue};
use vixen_core::{
    Color, ColorGradient, ColorPoint, Curve, EffectIntents, ElementTree, GradientLevelPair, IntentData, Mark, MarkCollection, NodeId,
    StateValue, TimeSpan, Uuid,
};
use vixen_effects::framework::{AudioSource, EffectDescriptor, ParamKind, ParamValue, Params, RenderCtx, TargetInfo};

const INTENSITY_TOLERANCE: f64 = 1e-6;
/// Effects in crates another agent owns. Failures are printed, not raised.
const REPORT_ONLY: &[&str] = &[];

fn golden_dir() -> PathBuf {
    std::env::var_os("GOLDEN_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden"))
}

fn read_json(name: &str) -> Option<Value> {
    let path = golden_dir().join(name);
    let file = std::fs::File::open(&path).ok()?;
    let parsed = if name.ends_with(".gz") {
        serde_json::from_reader(std::io::BufReader::new(flate2::read::GzDecoder::new(file)))
    } else {
        serde_json::from_reader(std::io::BufReader::new(file))
    };
    Some(parsed.unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

// ---------------------------------------------------------------------------------------------------------------------
// Targets
// ---------------------------------------------------------------------------------------------------------------------

struct Targets {
    tree: ElementTree,
    roots: HashMap<String, NodeId>,
}

fn build_targets() -> Targets {
    let json = read_json("targets.json").expect("tests/golden/targets.json: run tools/golden/build.sh run");
    let mut tree = ElementTree::new();
    let mut roots = HashMap::new();
    let mut by_name: HashMap<String, NodeId> = HashMap::new();
    for (key, spec) in json.as_object().unwrap() {
        let id = build_node(&mut tree, spec, &mut by_name);
        roots.insert(key.clone(), id);
    }
    Targets { tree, roots }
}

fn build_node(tree: &mut ElementTree, spec: &Value, by_name: &mut HashMap<String, NodeId>) -> NodeId {
    if let Some(shared) = spec.get("shared") {
        return by_name[shared.as_str().unwrap()];
    }
    let name = spec["n"].as_str().unwrap();
    let children = spec.get("c").and_then(|c| c.as_array());
    let id = match children {
        Some(_) => tree.add_node(Uuid::new_v4(), name, None),
        None => tree.add_element(name),
    };
    by_name.insert(name.to_string(), id);
    if let Some(l) = spec.get("loc") {
        tree.node_mut(id).properties.push(PropertyInstance {
            type_id: property_ids::LOCATION,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Location { x: l[0].as_i64().unwrap() as i32, y: l[1].as_i64().unwrap() as i32, z: 1 },
        });
    }
    if let Some(c) = spec.get("color") {
        tree.node_mut(id).properties.push(PropertyInstance {
            type_id: property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Color {
                color_type: ElementColorType::SingleColor,
                single_color: hex_color(c.as_str().unwrap()),
                color_set_name: None,
            },
        });
    }
    if let Some(s) = spec.get("set") {
        tree.node_mut(id).properties.push(PropertyInstance {
            type_id: property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Color {
                color_type: ElementColorType::MultipleDiscreteColors,
                single_color: Color::EMPTY,
                color_set_name: Some(s.as_str().unwrap().to_string()),
            },
        });
    }
    if let Some(o) = spec.get("orient") {
        tree.node_mut(id).properties.push(PropertyInstance {
            type_id: property_ids::ORIENTATION,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Orientation(if o == "Horizontal" { Orientation::Horizontal } else { Orientation::Vertical }),
        });
    }
    if let Some(children) = children {
        for c in children {
            let child = build_node(tree, c, by_name);
            tree.add_child(id, child);
        }
    }
    id
}

fn hex_color(s: &str) -> Color {
    let v = u32::from_str_radix(s, 16).unwrap();
    Color::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

// ---------------------------------------------------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------------------------------------------------

fn to_curve(v: &Value) -> Curve {
    Curve::from_points(v.as_array().unwrap().iter().map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap())).collect())
}

fn to_gradient(v: &Value) -> ColorGradient {
    let colors = v["c"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let color = hex_color(s[1].as_str().unwrap());
            ColorPoint::from_xyz(Xyz::from_rgb(Rgb::from_color(color)), s[2].as_f64().unwrap(), s[0].as_f64().unwrap())
        })
        .collect::<Vec<_>>();
    let alphas = match v.get("a") {
        Some(a) => a
            .as_array()
            .unwrap()
            .iter()
            .map(|s| AlphaPoint { position: s[0].as_f64().unwrap(), alpha: s[1].as_f64().unwrap(), focus: s[2].as_f64().unwrap() })
            .collect::<Vec<_>>(),
        None => vec![AlphaPoint::new(255, 0.0), AlphaPoint::new(255, 1.0)],
    };
    ColorGradient::from_parts(colors, alphas)
}

fn to_param(kind: ParamKind, v: &Value) -> ParamValue {
    match kind {
        ParamKind::Bool => ParamValue::Bool(v.as_bool().unwrap()),
        ParamKind::Int => ParamValue::Int(v.as_f64().unwrap() as i32),
        ParamKind::Long => ParamValue::Long(v.as_f64().unwrap() as i64),
        ParamKind::Double => ParamValue::Double(v.as_f64().unwrap()),
        ParamKind::Percentage => ParamValue::Percentage(v.as_f64().unwrap()),
        ParamKind::Str => ParamValue::Str(v.as_str().unwrap().to_string()),
        ParamKind::Enum(_) => ParamValue::Enum(v.as_str().unwrap().to_string()),
        ParamKind::Curve => ParamValue::Curve(to_curve(v)),
        ParamKind::Gradient => ParamValue::Gradient(to_gradient(v)),
        ParamKind::GradientList => ParamValue::GradientList(v.as_array().unwrap().iter().map(to_gradient).collect()),
        ParamKind::GradientLevels => ParamValue::GradientLevels(
            v.as_array().unwrap().iter().map(|p| GradientLevelPair::new(to_gradient(&p["g"]), to_curve(&p["l"]))).collect(),
        ),
        ParamKind::Color => ParamValue::Color(hex_color(v.as_str().unwrap())),
        ParamKind::Rgb => ParamValue::Rgb(Rgb::from_color(hex_color(v.as_str().unwrap()))),
        ParamKind::Guid => ParamValue::Guid(Uuid::parse_str(v.as_str().unwrap()).unwrap()),
        ParamKind::RecordList(record) => ParamValue::RecordList(v.as_array().unwrap().iter().map(|item| {
            let mut params = Params::defaults(record.params);
            for pair in item.as_array().unwrap() {
                let (name, value) = (pair[0].as_str().unwrap(), &pair[1]);
                let desc = record.params.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("unknown {} field {name}", record.xml_type));
                params.set(name, to_param(desc.kind, value));
            }
            params
        }).collect()),
        ParamKind::Other("ulong") => ParamValue::Raw(v.as_str().unwrap().to_string()),
        other => panic!("unhandled parameter kind {other:?}"),
    }
}

/// The values as the editor sets them: through the property setters, on an effect with its targets and mark collections.
fn edit_params(d: &'static EffectDescriptor, case: &Value, info: &TargetInfo) -> Result<Params, String> {
    let mut p = d.new_params();
    d.set_targets(&mut p, info);
    d.set_marks(&mut p, info);
    for pair in case["params"].as_array().unwrap() {
        let (name, value) = (pair[0].as_str().unwrap(), &pair[1]);
        if name == "MarkCollectionByName" {
            let wanted = value.as_str().unwrap();
            let mark = case["marks"].as_array().and_then(|m| m.iter().find(|x| x["name"] == wanted));
            let id = mark.map(|m| Uuid::parse_str(m["id"].as_str().unwrap()).unwrap()).unwrap_or(Uuid::nil());
            d.set_param(&mut p, "MarkCollectionId", ParamValue::Guid(id), info);
            continue;
        }
        let Some(desc) = d.params.iter().find(|x| x.name == name) else {
            return Err(format!("the Rust table has no parameter {name}"));
        };
        if let ParamKind::RecordList(record) = desc.kind {
            // Each child setter is a separate editor transaction in both runtimes.
            // Applying the final record in one call changes setter ordering (for
            // example Meteor resets ColorMode before a later ColorMode edit).
            let records = value.as_array().unwrap();
            let defaults = records.iter().map(|_| Params::defaults(record.params)).collect();
            d.set_param(&mut p, name, ParamValue::RecordList(defaults), info);
            for (index, fields) in records.iter().enumerate() {
                for pair in fields.as_array().unwrap() {
                    let (field, value) = (pair[0].as_str().unwrap(), &pair[1]);
                    let field_desc = record.params.iter().find(|p| p.name == field)
                        .ok_or_else(|| format!("unknown {} field {field}", record.xml_type))?;
                    let mut rows = p.records(name).to_vec();
                    rows[index].set(field, to_param(field_desc.kind, value));
                    d.set_param(&mut p, name, ParamValue::RecordList(rows), info);
                }
            }
            continue;
        }
        let value = match to_param(desc.kind, value) {
            ParamValue::Int(v) => ParamValue::Int(desc.display.stored(v)),
            other => other,
        };
        d.set_param(&mut p, name, value, info);
    }
    Ok(p)
}

/// A track the audio-driven effects read. `tools/golden/AudioSignal.cs` computes the same samples.
struct TestAudio {
    rate: u32,
    channels: u32,
    samples: Vec<f32>,
    mono: Vec<f32>,
}

impl TestAudio {
    fn new(rate: i32, channels: i32, seconds: i32) -> TestAudio {
        let mut samples = vec![0.0_f32; (rate * seconds * channels) as usize];
        let beat = rate * 3 / 10;
        let mut state = 0x2468ace1_u32;
        for frame in 0..rate * seconds {
            let envelope = 1.0_f32 - (frame % beat) as f32 / beat as f32;
            let second = frame / rate;
            let level = if second == 1 {
                0.125_f32
            } else if second == 2 && frame % rate < rate / 2 {
                0.0
            } else {
                1.0
            };
            for channel in 0..channels {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = ((state >> 16) & 0xffff) as f32 / 32768.0 - 1.0;
                samples[(frame * channels + channel) as usize] = noise * envelope * level;
            }
        }
        let mono = samples.chunks(channels as usize).map(|f| f.iter().sum::<f32>() / channels as f32).collect();
        TestAudio { rate: rate as u32, channels: channels as u32, samples, mono }
    }

    fn named(name: &str) -> &'static TestAudio {
        static MONO: OnceLock<TestAudio> = OnceLock::new();
        static STEREO: OnceLock<TestAudio> = OnceLock::new();
        match name {
            "mono22" => MONO.get_or_init(|| TestAudio::new(22050, 1, 6)),
            "stereo44" => STEREO.get_or_init(|| TestAudio::new(44100, 2, 6)),
            other => panic!("unknown audio {other}"),
        }
    }
}

impl AudioSource for TestAudio {
    fn samples(&self) -> &[f32] {
        &self.mono
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn duration(&self) -> TimeSpan {
        TimeSpan::from_secs(self.mono.len() as f64 / self.rate as f64)
    }
    fn interleaved(&self) -> &[f32] {
        &self.samples
    }
    fn channels(&self) -> u32 {
        self.channels
    }
}

fn marks_of(case: &Value) -> Vec<MarkCollection> {
    let Some(list) = case.get("marks").and_then(|m| m.as_array()) else { return Vec::new() };
    list.iter()
        .map(|m| {
            let mut c = MarkCollection {
                id: Uuid::parse_str(m["id"].as_str().unwrap()).unwrap(),
                name: m["name"].as_str().unwrap().to_string(),
                ..MarkCollection::default()
            };
            c.marks = m["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| {
                    let start = i[0].as_f64().unwrap();
                    let end = i[1].as_f64().unwrap();
                    let mut mark = Mark::new(TimeSpan::from_ms_i(start as i64));
                    mark.start_time = TimeSpan::from_ticks((start * 10_000.0) as i64);
                    mark.duration = TimeSpan::from_ticks(((end - start) * 10_000.0) as i64);
                    mark.text = i[2].as_str().unwrap().to_string();
                    mark
                })
                .collect();
            c
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Clone, PartialEq, Debug)]
enum S {
    Rgb(u8, u8, u8),
    Lighting(Color, f64),
    Discrete(Color, f64),
    /// A command's type and value, as `CommandText` in `tools/golden/Program.cs` writes them.
    Command(String),
}

impl S {
    fn key(&self) -> (u8, u32, u64) {
        match self {
            S::Rgb(r, g, b) => (0, (*r as u32) << 16 | (*g as u32) << 8 | *b as u32, 0),
            S::Lighting(c, i) => (1, c.to_argb_u32(), i.to_bits()),
            S::Discrete(c, i) => (2, c.to_argb_u32(), i.to_bits()),
            S::Command(_) => (3, 0, 0),
        }
    }

    fn brightness_and_color(&self) -> (f64, i32) {
        match self {
            S::Rgb(r, g, b) => ((*r).max(*g).max(*b) as f64 / 255.0, (*r as i32) << 16 | (*g as i32) << 8 | *b as i32),
            S::Lighting(c, i) | S::Discrete(c, i) => {
                (*i * c.r.max(c.g).max(c.b) as f64 / 255.0, (c.r as i32) << 16 | (c.g as i32) << 8 | c.b as i32)
            }
            S::Command(_) => (0.0, -1),
        }
    }
}

impl std::fmt::Display for S {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S::Rgb(r, g, b) => write!(f, "RGB({r},{g},{b})"),
            S::Lighting(c, i) => write!(f, "Lighting(a{} {},{},{} i={i:.9})", c.a, c.r, c.g, c.b),
            S::Discrete(c, i) => write!(f, "Discrete(a{} {},{},{} i={i:.9})", c.a, c.r, c.g, c.b),
            S::Command(text) => write!(f, "Command({text})"),
        }
    }
}

fn fmt_states(s: &[S]) -> String {
    if s.is_empty() {
        return "none".to_string();
    }
    s.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" + ")
}

fn parse_state(text: &str) -> S {
    let (kind, rest) = text.split_at(1);
    if kind == "R" {
        let v = u32::from_str_radix(rest, 16).unwrap();
        return S::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    }
    if kind == "C" {
        return S::Command(rest.to_string());
    }
    let (hex, intensity) = rest.split_once(',').unwrap();
    let v = u32::from_str_radix(hex, 16).unwrap();
    let color = if hex.len() == 8 {
        Color::from_argb_u32(v)
    } else {
        Color::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    };
    let i: f64 = intensity.parse().unwrap();
    match kind {
        "L" => S::Lighting(color, i),
        "D" => S::Discrete(color, i),
        _ => panic!("bad state {text}"),
    }
}

fn parse_state_set(text: &str) -> Vec<S> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut v: Vec<S> = text.split('|').map(parse_state).collect();
    v.sort_by_key(|s| s.key());
    v
}

fn state_of(v: &StateValue) -> S {
    match v {
        StateValue::Rgb(c) => S::Rgb(c.r, c.g, c.b),
        StateValue::Lighting(l) => S::Lighting(l.color, l.intensity),
        StateValue::Discrete(d) => S::Discrete(d.color, d.intensity),
        StateValue::Command(c) => S::Command(match c {
            CommandValue::U8(v) => format!("8:{v}"),
            CommandValue::U16(v) => format!("16:{v}"),
            CommandValue::U32(v) => format!("32:{v}"),
            CommandValue::U64(v) => format!("64:{v}"),
            CommandValue::Color(c) => format!("color:{:02x}{:02x}{:02x}", c.r, c.g, c.b),
            CommandValue::Str(s) => format!("text:{}", s.bytes().map(|b| format!("{b:02x}")).collect::<String>()),
            other => panic!("unexpected command {other:?}"),
        }),
        other => panic!("unexpected state {other:?}"),
    }
}

fn rust_states(intents: &EffectIntents, element: &Uuid, t: TimeSpan) -> Vec<S> {
    let mut out = Vec::new();
    if let Some(nodes) = intents.nodes_for(element) {
        for n in nodes {
            if n.contains(t) {
                out.push(state_of(&n.intent.state_at(t - n.start)));
            }
        }
    }
    out.sort_by_key(|s| s.key());
    out
}

fn states_match(expected: &[S], actual: &[S]) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(e, a)| match (e, a) {
            (S::Rgb(..), S::Rgb(..)) | (S::Command(_), S::Command(_)) => e == a,
            (S::Lighting(c1, i1), S::Lighting(c2, i2)) | (S::Discrete(c1, i1), S::Discrete(c2, i2)) => {
                c1 == c2 && (i1 - i2).abs() <= INTENSITY_TOLERANCE
            }
            _ => false,
        })
}

fn kind_letter(d: &IntentData) -> &'static str {
    match d {
        IntentData::Lighting { .. } => "L",
        IntentData::StaticLighting(_) => "l",
        IntentData::Discrete { .. } => "D",
        IntentData::StaticDiscrete(_) => "d",
        IntentData::ArrayRgb(_) => "A",
        IntentData::ArrayLighting(_) => "B",
        IntentData::ArrayDiscrete(_) => "C",
        IntentData::StaticRgb(_) => "S",
        IntentData::Rgb { .. } => "G",
        IntentData::Range { .. } => "range",
        IntentData::StaticRange(_) => "static-range",
        IntentData::Command(_) => "command",
    }
}

fn rust_intent_list(intents: &EffectIntents, element: &Uuid) -> Vec<(String, i64, i64)> {
    let mut v: Vec<(String, i64, i64)> = intents
        .nodes_for(element)
        .map(|nodes| nodes.iter().map(|n| (kind_letter(&n.intent.data).to_string(), n.start.ticks(), n.end.ticks())).collect())
        .unwrap_or_default();
    v.sort_by(|a, b| (a.1, a.2, &a.0).cmp(&(b.1, b.2, &b.0)));
    v
}

fn parse_intent_list(text: &str) -> Vec<(String, i64, i64)> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut v: Vec<(String, i64, i64)> = text
        .split(';')
        .map(|p| {
            let mut parts = p.split(':');
            let k = parts.next().unwrap().to_string();
            (k, parts.next().unwrap().parse().unwrap(), parts.next().unwrap().parse().unwrap())
        })
        .collect();
    v.sort_by(|a, b| (a.1, a.2, &a.0).cmp(&(b.1, b.2, &b.0)));
    v
}

// ---------------------------------------------------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------------------------------------------------

/// Which of the two ways a real effect gets rendered: the instance being edited, or the one a saved sequence loads.
#[derive(Copy, Clone, PartialEq)]
enum Lifecycle {
    Edited,
    Reloaded,
}

#[allow(clippy::too_many_arguments)]
fn render(d: &'static EffectDescriptor, targets: &Targets, nodes: &[NodeId], ms: i64, marks: &[MarkCollection], case: &Value, lifecycle: Lifecycle) -> Result<EffectIntents, String> {
    let cancel = AtomicBool::new(false);
    let ctx = RenderCtx { video_cache: None,
        tree: &targets.tree,
        targets: nodes,
        start_time: TimeSpan::from_ms_i(case.get("start_ms").and_then(|v| v.as_i64()).unwrap_or(0)),
        time_span: TimeSpan::from_ms_i(ms),
        frame_ms: case.get("frame_ms").and_then(|v| v.as_i64()).unwrap_or(50) as i32,
        marks,
        audio: case.get("audio").and_then(|a| a.as_str()).map(|a| TestAudio::named(a) as &dyn AudioSource),
        module_data_root: None,
        cancel: &cancel,
    };
    let info = ctx.target_info();
    let mut p = edit_params(d, case, &info)?;
    if lifecycle == Lifecycle::Reloaded {
        p = through_file(d, &p)?;
        d.load_data(&mut p, &info);
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (d.render)(&ctx, &p))).map_err(|e| {
        let text = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
        format!("the Rust render panicked: {text}")
    })
}

/// The data as a sequence file gives it back: written and read through the real file format, on-load fixups included.
fn through_file(d: &'static EffectDescriptor, p: &Params) -> Result<Params, String> {
    let model = vixen_io::params::effect_model_to_xml(&d.type_id, &Uuid::nil(), p).map_err(|e| format!("could not save the effect: {e}"))?;
    Ok(vixen_io::params::effect_params_from_xml(&model, d))
}

fn elements_of(targets: &Targets, nodes: &[NodeId]) -> Vec<(Uuid, String)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for n in nodes {
        for leaf in targets.tree.leaves(*n) {
            let node = targets.tree.node(leaf);
            if let Some(id) = node.element_id {
                if seen.insert(id) {
                    out.push((id, node.name.clone()));
                }
            }
        }
    }
    out
}

#[derive(Default)]
struct Summary {
    cases: usize,
    exact_cases: usize,
    stats_cases: usize,
    seeded_cases: usize,
    seeded_mismatches: Vec<String>,
    failed_cases: usize,
    frames_compared: u64,
    mismatched_cases: Vec<String>,
    mismatched_frames: u64,
    default_problems: Vec<String>,
    lines: Vec<String>,
}

fn params_text(case: &Value) -> String {
    let mut t = case["params"].to_string();
    if t.len() > 300 {
        t.truncate(300);
        t.push_str("...");
    }
    t
}

/// `el` and `iel` are pairs of value and count.
fn expand_runs(v: &Value) -> Vec<usize> {
    let mut out = Vec::new();
    for pair in v.as_array().unwrap().chunks(2) {
        for _ in 0..pair[1].as_u64().unwrap() {
            out.push(pair[0].as_u64().unwrap() as usize);
        }
    }
    out
}

/// Compares one render frame by frame. `data` is the stored render: the case itself, or its `seeded` object, which came
/// from a random stream `fastrand::seed(seed)` makes the Rust `Rand` reproduce.
#[allow(clippy::too_many_arguments)]
fn compare_exact(file: &str, d: &'static EffectDescriptor, targets: &Targets, nodes: &[NodeId], case: &Value, data: &Value, seed: Option<u64>, lifecycle: Lifecycle, summary: &mut Summary, max_print: usize) -> bool {
    let id = case["id"].as_str().unwrap();
    let ms = case["ms"].as_i64().unwrap();
    let marks = marks_of(case);
    let kind = match (seed.is_some(), lifecycle) {
        (true, _) => " (seeded)",
        (false, Lifecycle::Edited) => " (as edited)",
        (false, Lifecycle::Reloaded) => "",
    };
    let header = format!("{file} [{id}]{kind} targets={} ms={ms} params={}", case["targets"], params_text(case));
    if let Some(seed) = seed {
        fastrand::seed(seed);
    }
    let intents = match render(d, targets, nodes, ms, &marks, case, lifecycle) {
        Ok(i) => i,
        Err(e) => {
            summary.lines.push(format!("{header}\n    {e}"));
            return false;
        }
    };

    let times: Vec<i64> = case["times"].as_array().unwrap().iter().map(|t| t.as_i64().unwrap()).collect();
    let palette: Vec<Vec<S>> = data["pal"].as_array().unwrap().iter().map(|p| parse_state_set(p.as_str().unwrap())).collect();
    let tracks: Vec<Vec<usize>> = data["tracks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            let mut out = Vec::with_capacity(times.len());
            let mut last = 0usize;
            for v in t.as_array().unwrap() {
                let v = v.as_i64().unwrap();
                if v >= 0 {
                    last = v as usize;
                    out.push(last);
                } else {
                    for _ in 0..-v {
                        out.push(last);
                    }
                }
            }
            assert_eq!(out.len(), times.len(), "{header}: track length");
            out
        })
        .collect();
    let element_tracks = expand_runs(&data["el"]);
    let element_intents = expand_runs(&data["iel"]);
    let intent_lists: Vec<Vec<(String, i64, i64)>> = data["ints"].as_array().unwrap().iter().map(|p| parse_intent_list(p.as_str().unwrap())).collect();

    let elements = elements_of(targets, nodes);
    assert_eq!(elements.len(), element_tracks.len(), "{header}: element count");
    let known: BTreeSet<Uuid> = elements.iter().map(|e| e.0).collect();
    let mut ok = true;
    let mut printed = 0usize;
    let mut bad_frames = 0u64;
    for k in intents.map.keys() {
        if !known.contains(k) {
            ok = false;
            summary.lines.push(format!("{header}\n    intents for an element outside the target"));
        }
    }
    for (e, (element, name)) in elements.iter().enumerate() {
        let track = &tracks[element_tracks[e]];
        let expected_intents = &intent_lists[element_intents[e]];
        let actual_intents = rust_intent_list(&intents, element);
        if expected_intents != &actual_intents {
            ok = false;
            if printed < max_print {
                printed += 1;
                let fmt = |v: &[(String, i64, i64)]| {
                    let mut s = v.iter().take(6).map(|x| format!("{}:{}..{}", x.0, x.1, x.2)).collect::<Vec<_>>().join(" ");
                    if v.len() > 6 {
                        s.push_str(&format!(" (+{} more)", v.len() - 6));
                    }
                    s
                };
                summary.lines.push(format!(
                    "{header}\n    element {name}: intents differ (counts {} vs {})\n      expected {}\n      actual   {}",
                    expected_intents.len(),
                    actual_intents.len(),
                    fmt(expected_intents),
                    fmt(&actual_intents)
                ));
            }
        }
        for (ti, &t) in times.iter().enumerate() {
            summary.frames_compared += 1;
            let expected = &palette[track[ti]];
            let actual = rust_states(&intents, element, TimeSpan::from_ticks(t));
            if !states_match(expected, &actual) {
                ok = false;
                bad_frames += 1;
                if printed < max_print {
                    printed += 1;
                    summary.lines.push(format!(
                        "{header}\n    element {name} t={} ms: expected {} actual {}",
                        t as f64 / 10_000.0,
                        fmt_states(expected),
                        fmt_states(&actual)
                    ));
                }
            }
        }
    }
    summary.mismatched_frames += bad_frames;
    if !ok && bad_frames > 0 {
        summary.lines.push(format!("{file} [{id}]{kind}: {bad_frames} mismatched frame values"));
    }
    ok
}

// ---------------------------------------------------------------------------------------------------------------------
// Statistics: the same numbers tools/golden/Stats.cs computes
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Default, Clone)]
struct Series(Vec<f64>);

impl Series {
    fn add(&mut self, v: f64) {
        self.0.push(v);
    }
    fn mean(&self) -> f64 {
        if self.0.is_empty() {
            0.0
        } else {
            self.0.iter().sum::<f64>() / self.0.len() as f64
        }
    }
    fn std(&self) -> f64 {
        if self.0.len() < 2 {
            return 0.0;
        }
        let m = self.mean();
        (self.0.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / (self.0.len() as f64 - 1.0)).sqrt()
    }
    fn min(&self) -> f64 {
        self.0.iter().copied().fold(f64::INFINITY, f64::min)
    }
    fn max(&self) -> f64 {
        self.0.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }
}

const SCALARS: [&str; 12] = [
    "elements_with_intents",
    "intent_count",
    "span_min_ms",
    "span_max_ms",
    "span_mean_ms",
    "start_min_ms",
    "end_max_ms",
    "lit_fraction",
    "mean_brightness",
    "max_brightness",
    "min_lit_brightness",
    "distinct_colors",
];

struct StatRuns {
    scalars: Vec<Series>,
    hist: Vec<Series>,
    lit_by_time: Vec<Series>,
    level_by_time: Vec<Series>,
    lit_by_element: Vec<Series>,
    colors: BTreeSet<i32>,
}

fn collect_stats(renders: &[EffectIntents], elements: &[(Uuid, String)], times: &[i64]) -> StatRuns {
    let mut out = StatRuns {
        scalars: vec![Series::default(); SCALARS.len()],
        hist: vec![Series::default(); 10],
        lit_by_time: vec![Series::default(); times.len()],
        level_by_time: vec![Series::default(); times.len()],
        lit_by_element: vec![Series::default(); elements.len()],
        colors: BTreeSet::new(),
    };
    for intents in renders {
        let (mut with_intents, mut count) = (0u64, 0u64);
        let (mut span_min, mut span_max, mut span_sum, mut start_min, mut end_max) = (f64::MAX, 0.0f64, 0.0f64, f64::MAX, 0.0f64);
        for (id, _) in elements {
            let Some(nodes) = intents.nodes_for(id) else { continue };
            if !nodes.is_empty() {
                with_intents += 1;
            }
            for n in nodes {
                count += 1;
                let span = (n.end.ticks() - n.start.ticks()) as f64 / 10_000.0;
                span_min = span_min.min(span);
                span_max = span_max.max(span);
                span_sum += span;
                start_min = start_min.min(n.start.ticks() as f64 / 10_000.0);
                end_max = end_max.max(n.end.ticks() as f64 / 10_000.0);
            }
        }
        let (mut samples, mut lit) = (0u64, 0u64);
        let (mut sum, mut max, mut min_lit) = (0.0f64, 0.0f64, f64::MAX);
        let mut histogram = [0u64; 10];
        let mut colors = BTreeSet::new();
        let mut per_time_lit = vec![0u64; times.len()];
        let mut per_time_sum = vec![0.0f64; times.len()];
        for (e, (id, _)) in elements.iter().enumerate() {
            let mut element_lit = 0u64;
            for (ti, &t) in times.iter().enumerate() {
                let states = rust_states(intents, id, TimeSpan::from_ticks(t));
                let mut best = -1.0f64;
                let mut key = -1;
                for s in &states {
                    let (b, k) = s.brightness_and_color();
                    if b > best {
                        best = b;
                        key = k;
                    }
                }
                let b = best.max(0.0);
                samples += 1;
                sum += b;
                per_time_sum[ti] += b;
                if b > 0.0 {
                    lit += 1;
                    element_lit += 1;
                    per_time_lit[ti] += 1;
                    max = max.max(b);
                    min_lit = min_lit.min(b);
                    histogram[9.min((b * 10.0) as usize)] += 1;
                    colors.insert(key);
                }
            }
            out.lit_by_element[e].add(element_lit as f64 / times.len() as f64);
        }
        let values = [
            with_intents as f64,
            count as f64,
            if count == 0 { 0.0 } else { span_min },
            span_max,
            if count == 0 { 0.0 } else { span_sum / count as f64 },
            if count == 0 { 0.0 } else { start_min },
            end_max,
            if samples == 0 { 0.0 } else { lit as f64 / samples as f64 },
            if samples == 0 { 0.0 } else { sum / samples as f64 },
            max,
            if lit == 0 { 0.0 } else { min_lit },
            colors.len() as f64,
        ];
        for (i, v) in values.iter().enumerate() {
            out.scalars[i].add(*v);
        }
        for h in 0..10 {
            out.hist[h].add(if lit == 0 { 0.0 } else { histogram[h] as f64 / lit as f64 });
        }
        for t in 0..times.len() {
            out.lit_by_time[t].add(if elements.is_empty() { 0.0 } else { per_time_lit[t] as f64 / elements.len() as f64 });
            out.level_by_time[t].add(if elements.is_empty() { 0.0 } else { per_time_sum[t] / elements.len() as f64 });
        }
        out.colors.extend(colors);
    }
    out
}

/// Two sample means from independent renders agree when they are within `SIGMAS` standard errors of each other,
/// plus a small floor for rounding and for quantities that barely vary in one runtime and not the other.
const SIGMAS: f64 = 6.0;

fn differs(expected_mean: f64, expected_std: f64, actual: &Series, renders: f64, floor: f64) -> Option<f64> {
    let se = ((expected_std * expected_std + actual.std() * actual.std()) / renders).sqrt();
    let tolerance = SIGMAS * se + floor;
    if (expected_mean - actual.mean()).abs() > tolerance {
        Some(tolerance)
    } else {
        None
    }
}

fn compare_stats(file: &str, d: &'static EffectDescriptor, targets: &Targets, nodes: &[NodeId], case: &Value, summary: &mut Summary) -> bool {
    let id = case["id"].as_str().unwrap();
    let ms = case["ms"].as_i64().unwrap();
    let marks = marks_of(case);
    let header = format!("{file} [{id}] targets={} ms={ms} params={}", case["targets"], params_text(case));
    let renders_wanted = case["renders"].as_u64().unwrap() as usize;
    let mut renders = Vec::with_capacity(renders_wanted);
    for _ in 0..renders_wanted {
        match render(d, targets, nodes, ms, &marks, case, Lifecycle::Reloaded) {
            Ok(i) => renders.push(i),
            Err(e) => {
                summary.lines.push(format!("{header}\n    {e}"));
                return false;
            }
        }
    }
    let times: Vec<i64> = case["times"].as_array().unwrap().iter().map(|t| t.as_i64().unwrap()).collect();
    let elements = elements_of(targets, nodes);
    let stats = collect_stats(&renders, &elements, &times);
    let n = renders_wanted as f64;
    let mut problems: Vec<String> = Vec::new();

    for (i, name) in SCALARS.iter().enumerate() {
        let e = &case["scalars"][*name];
        let (mean, std) = (e[0].as_f64().unwrap(), e[1].as_f64().unwrap());
        // Counts and times get a relative floor; fractions get an absolute one.
        let floor = match *name {
            "elements_with_intents" | "intent_count" | "distinct_colors" => (mean.abs() * 0.05).max(1.0),
            "span_min_ms" | "span_max_ms" | "span_mean_ms" | "start_min_ms" | "end_max_ms" => (mean.abs() * 0.05).max(2.0),
            _ => 0.02,
        };
        if let Some(t) = differs(mean, std, &stats.scalars[i], n, floor) {
            problems.push(format!(
                "{name}: C# {mean:.5} (std {std:.5}) vs Rust {:.5} (std {:.5}, range {:.4}..{:.4}), allowed {t:.5}",
                stats.scalars[i].mean(),
                stats.scalars[i].std(),
                stats.scalars[i].min(),
                stats.scalars[i].max()
            ));
        }
    }
    let series = |name: &str, actual: &[Series], floor: f64, problems: &mut Vec<String>| {
        let expected = case[name].as_array().unwrap();
        assert_eq!(expected.len(), actual.len(), "{header}: {name}");
        let mut count = 0;
        let mut first = String::new();
        for (i, e) in expected.iter().enumerate() {
            let (mean, std) = (e[0].as_f64().unwrap(), e[1].as_f64().unwrap());
            if let Some(t) = differs(mean, std, &actual[i], n, floor) {
                if count == 0 {
                    first = format!("first at index {i}: C# {mean:.5} (std {std:.5}) vs Rust {:.5} (std {:.5}), allowed {t:.5}", actual[i].mean(), actual[i].std());
                }
                count += 1;
            }
        }
        if count > 0 {
            problems.push(format!("{name}: {count} of {} differ, {first}", expected.len()));
        }
    };
    series("hist", &stats.hist, 0.03, &mut problems);
    series("lit_by_time", &stats.lit_by_time, 0.04, &mut problems);
    series("level_by_time", &stats.level_by_time, 0.04, &mut problems);
    series("lit_by_element", &stats.lit_by_element, 0.05, &mut problems);

    if let Some(colors) = case.get("colors").and_then(|c| c.as_array()) {
        let expected: BTreeSet<i32> = colors.iter().map(|c| c.as_i64().unwrap() as i32).collect();
        // Only a small palette can be compared: every colour Rust shows must be one the C# showed.
        if expected.len() <= 32 {
            let extra: Vec<String> = stats.colors.difference(&expected).take(6).map(|c| format!("{c:06x}")).collect();
            if !extra.is_empty() {
                problems.push(format!("colours the C# never shows: {} (C# palette {} colours, Rust {})", extra.join(" "), expected.len(), stats.colors.len()));
            }
        }
    }
    let usize_expected = case["color_union_size"].as_f64().unwrap();
    if (usize_expected <= 8.0) && (stats.colors.len() as f64 > usize_expected + 1.0) {
        problems.push(format!("the C# uses {usize_expected} colours, Rust {}", stats.colors.len()));
    }

    if !problems.is_empty() {
        summary.lines.push(format!("{header}\n    {}", problems.join("\n    ")));
        return false;
    }
    true
}

// ---------------------------------------------------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------------------------------------------------

/// A case where the C# threw and has nothing to compare frame by frame: the Rust must not panic, and with `dark` must
/// render nothing either.
#[allow(clippy::too_many_arguments)]
fn compare_threw(file: &str, d: &'static EffectDescriptor, targets: &Targets, nodes: &[NodeId], case: &Value, data: &Value, lifecycle: Lifecycle, summary: &mut Summary) -> bool {
    let id = case["id"].as_str().unwrap();
    let marks = marks_of(case);
    match render(d, targets, nodes, case["ms"].as_i64().unwrap(), &marks, case, lifecycle) {
        Ok(out) => {
            let lit = data.get("dark").is_some() && out.map.values().any(|n| !n.is_empty());
            if lit {
                summary.lines.push(format!("{file} [{id}]: the C# throws here and has no output ({}), but the Rust renders", data["failure"]));
            }
            !lit
        }
        Err(e) => {
            summary.lines.push(format!("{file} [{id}]: the C# throws here ({}), and the Rust must not: {e}", data["failure"]));
            false
        }
    }
}

/// The C# defaults against the Rust parameter table, so a newly created effect matches too.
fn compare_defaults(file: &str, d: &'static EffectDescriptor, json: &Value, summary: &mut Summary) {
    let p = d.new_params();
    for (name, expected) in json["defaults"].as_object().unwrap() {
        let Some(desc) = d.params.iter().find(|x| x.name == name) else {
            summary.default_problems.push(format!("{file}: the Rust table has no parameter {name}"));
            continue;
        };
        let actual = p.get(name);
        let same = match actual {
            ParamValue::Bool(v) => expected.as_bool() == Some(*v),
            ParamValue::Int(v) => expected.as_f64() == Some(desc.display.shown(*v) as f64),
            ParamValue::Long(v) => expected.as_f64() == Some(*v as f64),
            ParamValue::Double(v) | ParamValue::Percentage(v) => (expected.as_f64().unwrap_or(f64::NAN) - *v).abs() < 1e-6,
            ParamValue::Str(v) | ParamValue::Enum(v) => expected.as_str() == Some(v.as_str()),
            ParamValue::Color(c) => expected.as_str().map(hex_color) == Some(*c),
            ParamValue::Rgb(c) => expected.as_str().map(hex_color) == Some(c.to_color()),
            ParamValue::Curve(c) => same_curve(c, expected),
            ParamValue::Gradient(g) => same_gradient(g, expected),
            ParamValue::GradientList(list) => {
                let e = expected.as_array().unwrap();
                e.len() == list.len() && e.iter().zip(list).all(|(x, g)| same_gradient(g, x))
            }
            ParamValue::GradientLevels(list) => {
                let e = expected.as_array().unwrap();
                e.len() == list.len() && e.iter().zip(list).all(|(x, g)| same_gradient(&g.color_gradient, &x["g"]) && same_curve(&g.curve, &x["l"]))
            }
            _ => true,
        };
        if !same {
            let mut a = format!("{actual:?}");
            a.truncate(200);
            let mut e = expected.to_string();
            e.truncate(200);
            summary.default_problems.push(format!("{file}: default of {name}: C# {e} Rust {a}"));
        }
    }
}

fn same_curve(c: &Curve, v: &Value) -> bool {
    to_curve(v).points == c.points
}

fn same_gradient(g: &ColorGradient, v: &Value) -> bool {
    let e = to_gradient(v);
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    e.colors().len() == g.colors().len()
        && e.alphas().len() == g.alphas().len()
        && e.colors().iter().zip(g.colors()).all(|(a, b)| a.rgb_color() == b.rgb_color() && close(a.position, b.position) && close(a.focus, b.focus))
        && e.alphas().iter().zip(g.alphas()).all(|(a, b)| close(a.alpha, b.alpha) && close(a.position, b.position) && close(a.focus, b.focus))
}

fn run_effect(file: &str) {
    let json = read_json(&format!("{file}.json.gz")).unwrap_or_else(|| panic!("{file}: no golden file. Run tools/golden/build.sh run"));
    let type_id = Uuid::parse_str(json["type_id"].as_str().unwrap()).unwrap();
    let d = vixen_effect_registry::all().iter().copied().find(|d| d.type_id == type_id).unwrap_or_else(|| panic!("{file}: no such effect in this build"));
    if !d.ported {
        println!("{file}: ported is false, not compared");
        return;
    }
    let targets = build_targets();
    let filter = std::env::var("GOLDEN_CASE").ok();
    let max_print: usize = std::env::var("GOLDEN_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    let mut summary = Summary::default();
    compare_defaults(file, d, &json, &mut summary);
    for case in json["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if let Some(f) = &filter {
            if !id.contains(f.as_str()) {
                continue;
            }
        }
        // Renders that do not script their stream still draw from fastrand's thread-local generator. Seeding it keeps a
        // statistical comparison from flickering between test runs, or depending on which cases ran before it.
        fastrand::seed(0x5eed);
        summary.cases += 1;
        let nodes: Vec<NodeId> = case["targets"].as_array().unwrap().iter().map(|t| targets.roots[t.as_str().unwrap()]).collect();
        let ok = match case["check"].as_str().unwrap() {
            "exact" => {
                summary.exact_cases += 1;
                let reloaded = compare_exact(file, d, &targets, &nodes, case, case, None, Lifecycle::Reloaded, &mut summary, max_print);
                // `edited` holds the render of the edited instance only when it differs from the reloaded one.
                let edited = case.get("edited").unwrap_or(case);
                let edited_ok = if edited.get("pal").is_some() {
                    compare_exact(file, d, &targets, &nodes, case, edited, None, Lifecycle::Edited, &mut summary, max_print)
                } else {
                    compare_threw(file, d, &targets, &nodes, case, edited, Lifecycle::Edited, &mut summary)
                };
                reloaded && edited_ok
            }
            "stats" => {
                summary.stats_cases += 1;
                let mut ok = compare_stats(file, d, &targets, &nodes, case, &mut summary);
                if let Some(seeded) = case.get("seeded") {
                    summary.seeded_cases += 1;
                    let seed: u64 = seeded["seed"].as_str().unwrap().parse().unwrap();
                    let reloaded = compare_exact(file, d, &targets, &nodes, case, seeded, Some(seed), Lifecycle::Reloaded, &mut summary, max_print);
                    // `seeded_edited` holds the render of the edited instance only when it differs from the reloaded one.
                    let edited = case.get("seeded_edited").unwrap_or(seeded);
                    let edited_ok = if edited.get("pal").is_some() {
                        compare_exact(file, d, &targets, &nodes, case, edited, Some(seed), Lifecycle::Edited, &mut summary, max_print)
                    } else {
                        compare_threw(file, d, &targets, &nodes, case, edited, Lifecycle::Edited, &mut summary)
                    };
                    if !reloaded || !edited_ok {
                        summary.seeded_mismatches.push(id.to_string());
                        ok = false;
                    }
                }
                ok
            }
            "threw" => {
                // The C# threw. A pixel effect then has nothing (`dark`); a basic effect keeps what it built before
                // the exception, and that is what a real show plays.
                summary.failed_cases += 1;
                if case.get("pal").is_some() {
                    compare_exact(file, d, &targets, &nodes, case, case, None, Lifecycle::Reloaded, &mut summary, max_print)
                } else {
                    compare_threw(file, d, &targets, &nodes, case, case, Lifecycle::Reloaded, &mut summary)
                }
            }
            other => panic!("unknown check {other}"),
        };
        if !ok {
            summary.mismatched_cases.push(id.to_string());
        }
    }
    for l in &summary.lines {
        println!("{l}");
    }
    for l in &summary.default_problems {
        println!("DEFAULTS {l}");
    }
    println!(
        "SUMMARY {file}: {} cases ({} exact, {} stats of which {} also seeded, {} C#-threw), {} frame values compared, {} mismatched frame values, {} cases mismatched ({} seeded)",
        summary.cases,
        summary.exact_cases,
        summary.stats_cases,
        summary.seeded_cases,
        summary.failed_cases,
        summary.frames_compared,
        summary.mismatched_frames,
        summary.mismatched_cases.len(),
        summary.seeded_mismatches.len(),
    );
    if !summary.mismatched_cases.is_empty() {
        println!("MISMATCHED {file}: {}", summary.mismatched_cases.join(", "));
    }
    if (!summary.mismatched_cases.is_empty() || !summary.default_problems.is_empty()) && !REPORT_ONLY.contains(&file) {
        panic!(
            "{file}: {} of {} cases differ from the C#, {} defaults differ",
            summary.mismatched_cases.len(),
            summary.cases,
            summary.default_problems.len()
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Layer mixing: vixen_core::layer against the original LayeredStateCombinator and filter modules
// ---------------------------------------------------------------------------------------------------------------------

fn mixing_filter(v: &Value) -> MixingFilter {
    let id = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();
    let number = |name: &str| v[name].as_f64().unwrap();
    let flag = |name: &str| v[name].as_bool().unwrap();
    use vixen_core::layer::filter_ids as ids;
    match id {
        ids::PROPORTIONAL_MIX => MixingFilter::ProportionalMix,
        ids::HIGHEST_VALUE => MixingFilter::HighestValue,
        ids::MASK => MixingFilter::Mask,
        ids::MASK_FILL => MixingFilter::MaskFill { exclude_zero_values: flag("exclude"), requires_mixing_partner: flag("partner") },
        ids::MULTIPLY_COLOR => MixingFilter::MultiplyColor,
        ids::INTENSITY_OVERLAY => MixingFilter::IntensityOverlay,
        ids::COLOR_CHANGE => MixingFilter::ColorChange,
        ids::LUMA_KEY => MixingFilter::LumaKey { lower_limit: number("lower"), upper_limit: number("upper") },
        ids::CHROMA_KEY if v.get("defaults").is_some() => MixingFilter::ChromaKey(ChromaKeySettings::default()),
        ids::CHROMA_KEY => {
            let key = hex_color(v["key"].as_str().unwrap());
            MixingFilter::ChromaKey(ChromaKeySettings {
                lower_limit: number("lower"),
                upper_limit: number("upper"),
                key_color: key,
                key_saturation: Hsv::from_color(key).s,
                key_hue: key.hue() as f64,
                hue_tolerance: number("hue_tolerance"),
                saturation_tolerance: number("saturation_tolerance"),
                transparent_on_zero_brightness: flag("transparent"),
            })
        }
        other => panic!("unknown filter {other}"),
    }
}

fn combined_states(out: &CombinedState) -> Vec<S> {
    let mut v: Vec<S> = Vec::new();
    if let Some(l) = out.lighting {
        v.push(S::Lighting(l.color, l.intensity));
    }
    if let Some(r) = out.rgb {
        v.push(S::Rgb(r.r, r.g, r.b));
    }
    for d in &out.discrete {
        v.push(S::Discrete(d.color, d.intensity));
    }
    v.sort_by_key(|s| s.key());
    v
}

#[test]
fn layer_mixing() {
    let json = read_json("layers.json.gz").expect("layers: no golden file. Run tools/golden/build.sh run");
    let (mut cases, mut mismatches) = (0u64, 0u64);
    let mut printed = 0;
    for stack in json["stacks"].as_array().unwrap() {
        let mut stack_layers = SequenceLayers::default();
        let filters = stack["filters"].as_array().unwrap();
        for f in filters.iter().rev() {
            stack_layers.add_layer(Layer::standard(f["name"].as_str().unwrap(), mixing_filter(f)));
        }
        let names: Vec<&str> = filters.iter().map(|f| f["name"].as_str().unwrap()).collect();
        let layers = &stack_layers.layers;
        let mut combinator = StateCombinator::default();
        let mut out = CombinedState::default();
        for case in stack["cases"].as_array().unwrap() {
            cases += 1;
            let mut states: Vec<LayeredState> = case[0]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| {
                    let index = s[0].as_u64().unwrap() as usize;
                    let color = hex_color(s[2].as_str().unwrap());
                    let intensity = s[3].as_f64().unwrap();
                    let value = match s[1].as_str().unwrap() {
                        "L" => StateValue::Lighting(LightingValue::new(color, intensity)),
                        "R" => StateValue::Rgb(RgbValue::new(color)),
                        _ => StateValue::Discrete(DiscreteValue::new(color, intensity)),
                    };
                    LayeredState { value, level: layers[index].level, requires_partner: layers[index].requires_mixing_partner(), layer_index: index as u16 }
                })
                .collect();
            combinator.combine(&mut states, layers, &mut out);
            let actual = combined_states(&out);
            let mut expected: Vec<S> = case[1].as_array().unwrap().iter().map(|e| parse_state(e.as_str().unwrap())).collect();
            expected.sort_by_key(|s| s.key());
            if !states_match(&expected, &actual) {
                mismatches += 1;
                if printed < 40 {
                    printed += 1;
                    println!("layers {names:?} states {}: expected {} actual {}", case[0], fmt_states(&expected), fmt_states(&actual));
                }
            }
        }
    }
    println!("SUMMARY layers: {cases} combinations compared, {mismatches} mismatched");
    assert_eq!(mismatches, 0, "layer mixing differs from the C#");
}

macro_rules! golden_tests {
    ($($name:ident => $file:literal),* $(,)?) => {
        $(#[test] fn $name() { run_effect($file); })*
    };
}

golden_tests! {
    set_level => "SetLevel",
    pulse => "Pulse",
    wipe => "Wipe",
    dissolve => "Dissolve",
    twinkle => "Twinkle",
    chase => "Chase",
    spin => "Spin",
    plasma => "Plasma",
    bars => "Bars",
    candle => "Candle",
    strobe => "Strobe",
    color_wash => "ColorWash",
    alternating => "Alternating",
    butterfly => "Butterfly",
    spiral => "Spiral",
    vertical_meter => "VerticalMeter",
    circles => "Circles",
    curtain => "Curtain",
    garlands => "Garlands",
    life => "Life",
    meteors => "Meteors",
    pin_wheel => "PinWheel",
    snow_storm => "SnowStorm",
    spirograph => "Spirograph",
    tree => "Tree",
    border => "Border",
    fire => "Fire",
    balls => "Balls",
    shockwave => "Shockwave",
    fireworks => "Fireworks",
    custom_value => "CustomValue",
    launcher => "Launcher",
    rds => "RDS",
    pattern => "Pattern",
    wave => "Wave",
    whirlpool => "Whirlpool",
    vu_meter => "VUMeter",
    waveform => "Waveform",
}
