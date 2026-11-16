//! Prints every effect's descriptor and parameter tables as JSON, for `scripts/check_effect_metadata.py`.

use serde_json::{json, Value};
use vixen_core::color::Rgb;
use vixen_core::{Color, ColorGradient, Curve};
use vixen_effects::framework::{Display, EffectDescriptor, Editor, ParamDesc, ParamKind, ParamValue, Params, RecordDesc};

fn color(c: Color) -> Value {
    json!([c.a, c.r, c.g, c.b])
}

fn curve(c: &Curve) -> Value {
    json!({"points": c.points})
}

fn gradient(g: &ColorGradient) -> Value {
    let colors: Vec<Value> = g.colors().iter().map(|p| json!({"color": color(p.rgb_color()), "position": p.position})).collect();
    let alphas: Vec<Value> = g.alphas().iter().map(|p| json!({"alpha": p.alpha, "position": p.position})).collect();
    json!({"colors": colors, "alphas": alphas})
}

fn rgb(c: &Rgb) -> Value {
    json!([c.r, c.g, c.b])
}

fn value(v: &ParamValue) -> Value {
    match v {
        ParamValue::None => json!(null),
        ParamValue::Bool(b) => json!(b),
        ParamValue::Int(i) => json!(i),
        ParamValue::Long(l) => json!(l),
        ParamValue::Double(d) | ParamValue::Percentage(d) => json!(d),
        ParamValue::Str(s) | ParamValue::Enum(s) | ParamValue::Raw(s) => json!(s),
        ParamValue::Curve(c) => curve(c),
        ParamValue::Gradient(g) => gradient(g),
        ParamValue::GradientList(l) => Value::Array(l.iter().map(gradient).collect()),
        ParamValue::GradientLevels(l) => {
            Value::Array(l.iter().map(|p| json!({"gradient": gradient(&p.color_gradient), "curve": curve(&p.curve)})).collect())
        }
        ParamValue::Color(c) => color(*c),
        ParamValue::ColorList(l) => Value::Array(l.iter().map(|c| color(*c)).collect()),
        ParamValue::Rgb(c) => rgb(c),
        ParamValue::Font(f) => json!({"name": f.name, "size": f.size, "bold": f.bold, "italic": f.italic}),
        ParamValue::Guid(g) => json!(g.to_string()),
        ParamValue::Time(t) => json!(t.total_ms()),
        ParamValue::Record(r) => params(r),
        ParamValue::RecordList(l) => Value::Array(l.iter().map(params).collect()),
    }
}

fn params(p: &Params) -> Value {
    let mut map = serde_json::Map::new();
    for (d, v) in p.desc.iter().zip(&p.values) {
        map.insert(d.name.to_string(), value(v));
    }
    Value::Object(map)
}

fn record(r: &RecordDesc) -> Value {
    json!({
        "xml_type": r.xml_type,
        "xml_ns": r.xml_ns,
        "has_visibility": r.visibility.is_some(),
        "params": r.params.iter().map(param).collect::<Vec<_>>(),
    })
}

fn kind(k: &ParamKind) -> Value {
    match k {
        ParamKind::Bool => json!({"kind": "Bool"}),
        ParamKind::Int => json!({"kind": "Int"}),
        ParamKind::Long => json!({"kind": "Long"}),
        ParamKind::Double => json!({"kind": "Double"}),
        ParamKind::Str => json!({"kind": "Str"}),
        ParamKind::Enum(e) => json!({
            "kind": "Enum",
            "enum": e.name,
            "members": e.members.iter().map(|m| json!([m.name, m.label])).collect::<Vec<_>>(),
        }),
        ParamKind::Curve => json!({"kind": "Curve"}),
        ParamKind::Gradient => json!({"kind": "Gradient"}),
        ParamKind::GradientList => json!({"kind": "GradientList"}),
        ParamKind::GradientLevels => json!({"kind": "GradientLevels"}),
        ParamKind::Color => json!({"kind": "Color"}),
        ParamKind::ColorList => json!({"kind": "ColorList"}),
        ParamKind::Rgb => json!({"kind": "Rgb"}),
        ParamKind::Percentage => json!({"kind": "Percentage"}),
        ParamKind::Font => json!({"kind": "Font"}),
        ParamKind::Guid => json!({"kind": "Guid"}),
        ParamKind::Time => json!({"kind": "Time"}),
        ParamKind::Record(r) => json!({"kind": "Record", "record": record(r)}),
        ParamKind::RecordList(r) => json!({"kind": "RecordList", "record": record(r)}),
        ParamKind::Other(t) => json!({"kind": "Other", "type": t}),
    }
}

fn editor(e: &Editor) -> Value {
    match e {
        Editor::Default => json!({"type": "Default"}),
        Editor::Number { min, max, step } => json!({"type": "Number", "min": min, "max": max, "step": step}),
        Editor::Slider { min, max, step } => json!({"type": "Slider", "min": min, "max": max, "step": step}),
        Editor::SliderDouble { min, max, step } => json!({"type": "SliderDouble", "min": min, "max": max, "step": step}),
        Editor::Level => json!({"type": "Level"}),
        Editor::Percent => json!({"type": "Percent"}),
        Editor::Selection => json!({"type": "Selection"}),
        Editor::Label => json!({"type": "Label"}),
        Editor::Path(k) => json!({"type": "Path", "path": k}),
    }
}

fn display(d: &Display) -> Value {
    match d {
        Display::Stored => json!({"type": "Stored"}),
        Display::Times(k) => json!({"type": "Times", "k": k}),
        Display::Minus(k) => json!({"type": "Minus", "k": k}),
    }
}

fn param(d: &ParamDesc) -> Value {
    json!({
        "name": d.name,
        "xml": d.xml,
        "xml_ns": d.xml_ns,
        "xml_level": d.xml_level,
        "kind": kind(&d.kind),
        "category": d.category,
        "category_order": d.category_order,
        "label": d.label,
        "description": d.description,
        "order": d.order,
        "browsable": d.browsable,
        "is_value": d.is_value,
        "exclude_from_default": d.exclude_from_default,
        "editor": editor(&d.editor),
        "display": display(&d.display),
        "bool_labels": d.bool_labels.map(|(t, f)| json!([t, f])),
        "default": value(&(d.default)()),
    })
}

fn effect(d: &EffectDescriptor) -> Value {
    json!({
        "type_id": d.type_id.to_string(),
        "name": d.name,
        "group": format!("{:?}", d.group),
        "author": d.author,
        "description": d.description,
        "version": d.version,
        "data_type": d.data_type,
        "data_ns": d.data_ns,
        "info_link": d.info_link,
        "information": d.information,
        "icon": d.icon,
        "force_visual": d.force_visual,
        "force_visual_when": d.force_visual_when.is_some(),
        "has_visual": d.visual.is_some(),
        "supports_location": d.supports_location,
        "ported": d.ported,
        "params": d.params.iter().map(param).collect::<Vec<_>>(),
    })
}

fn main() {
    let all: Vec<Value> = vixen_effect_registry::all().iter().map(|d| effect(d)).collect();
    println!("{}", serde_json::to_string(&all).expect("the tables are valid JSON"));
}
