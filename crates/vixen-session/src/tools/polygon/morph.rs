use super::*;
use crate::{
    sequence::{EffectId, SequenceSession},
    undo::UndoAction,
};
use vixen_effects::{
    framework::{ParamKind, ParamValue, RecordDesc},
    pixel::{display_element_size_as, StringOrientation, TargetPositioning},
};

pub fn morph_defaults() -> Params {
    vixen_effect_registry::by_data_type("MorphData")
        .expect("registered Morph effect")
        .new_params()
}
fn record_desc(p: &Params, name: &str) -> Option<&'static RecordDesc> {
    match p.desc.iter().find(|d| d.name == name)?.kind {
        ParamKind::Record(r) | ParamKind::RecordList(r) => Some(r),
        _ => None,
    }
}
fn geometry(row: &Params) -> Option<(Kind, &Params)> {
    [Kind::Polygon, Kind::Line, Kind::Ellipse]
        .into_iter()
        .find_map(|k| row.record(k.name()).map(|p| (k, p)))
}
fn shape_id(row: &Params) -> Option<Uuid> {
    geometry(row).and_then(|(_, g)| match g.get("ID") {
        ParamValue::Guid(id) => Some(*id),
        _ => None,
    })
}
fn read_shapes(p: &Params) -> Vec<Shape> {
    p.records("MorphPolygonData")
        .iter()
        .filter_map(|row| {
            let (kind, g) = geometry(row)?;
            let id = shape_id(row)?;
            let points = g
                .records("Points")
                .iter()
                .map(|p| Point::new(p.double("X"), p.double("Y")))
                .collect();
            let mut s = Shape::new(kind, points, row.double("Time"), false);
            s.id = id;
            s.start_lime = false;
            s.label = g.string("Label").into();
            s.fill = g.enum_name("FillType").into();
            s.original = Some(g.clone());
            if kind == Kind::Ellipse {
                s.angle = g.double("Angle");
                s.start_side = g.int("StartSideRotation");
                s.recalculate();
            }
            Some(s)
        })
        .collect()
}
pub fn open_from_sequence(
    s: &SequenceSession,
    effects: &[EffectId],
    screen: (f64, f64),
) -> Result<Editor, String> {
    let id = *effects.first().ok_or("Select a Morph effect to edit.")?;
    let e = s.effect(id).ok_or("The selected effect was removed.")?;
    let p = &e.node.params;
    if vixen_effect_registry::by_type_id(&e.node.type_id).is_none_or(|d| d.data_type != "MorphData")
    {
        return Err("The selected effect does not contain polygons.".into());
    }
    let info = s
        .target_info(id)
        .ok_or("The selected effect was removed.")?;
    let positioning = TargetPositioning::from_name(p.enum_name("TargetPositioning"));
    let orientation = if matches!(positioning, TargetPositioning::Locations) {
        StringOrientation::Vertical
    } else {
        StringOrientation::from_name(p.enum_name("StringOrientation"))
    };
    let size = display_element_size_as(&info, orientation, positioning);
    let display = (f64::from(size.0.max(1)), f64::from(size.1.max(1)));
    let margin = |n: f64| (n * p.int("Margin") as f64 / 100.).round_ties_even();
    let full = (
        display.0 + 2. * margin(display.0),
        display.1 + 2. * margin(display.1),
    );
    let mode = match p.enum_name("PolygonType") {
        "FreeForm" => MorphMode::FreeForm,
        "TimeBased" => MorphMode::TimeBased,
        _ => MorphMode::Pattern,
    };
    Ok(Editor::new(read_shapes(p), mode, full, display, screen))
}
fn write_geometry(shape: &Shape, row: &Params) -> Params {
    let mut g = shape.original.clone().unwrap_or_else(|| {
        Params::defaults(
            record_desc(row, shape.kind.name())
                .expect("Morph shape descriptor")
                .params,
        )
    });
    g.set("ID", ParamValue::Guid(shape.id));
    if !shape.label.is_empty() || !matches!(g.get("Label"), ParamValue::None) {
        g.set("Label", ParamValue::Str(shape.label.clone()));
    }
    g.set("FillType", ParamValue::Enum(shape.fill.clone()));
    let point_desc = record_desc(&g, "Points").expect("Morph points descriptor");
    let point = |p: Point| {
        let mut r = Params::defaults(point_desc.params);
        r.set("X", ParamValue::Double(p.x));
        r.set("Y", ParamValue::Double(p.y));
        r
    };
    g.set(
        "Points",
        ParamValue::RecordList(shape.points.iter().copied().map(point).collect()),
    );
    if shape.kind == Kind::Ellipse {
        g.set("Center", ParamValue::Record(Box::new(point(shape.center))));
        g.set("Width", ParamValue::Double(shape.width));
        g.set("Height", ParamValue::Double(shape.height));
        g.set("Angle", ParamValue::Double(shape.angle));
        g.set("StartSideRotation", ParamValue::Int(shape.start_side));
    }
    g
}
/// Mirrors Morph's container setter: retain every other setting on a matching ID, and append new rows with defaults.
/// Existing rows retain their order; only TimeBased sorts the final collection.
pub fn apply_shapes(p: &Params, shapes: &[Shape]) -> Params {
    let mut result = p.clone();
    let Some(desc) = record_desc(p, "MorphPolygonData") else {
        return result;
    };
    let time_mode = p.enum_name("PolygonType") == "TimeBased";
    let mut rows = Vec::new();
    let mut used = std::collections::HashSet::new();
    let update = |mut row: Params, s: &Shape| {
        for k in [Kind::Polygon, Kind::Line, Kind::Ellipse] {
            row.set(k.name(), ParamValue::None);
        }
        row.set(
            s.kind.name(),
            ParamValue::Record(Box::new(write_geometry(s, &row))),
        );
        row.set("FillType", ParamValue::Enum(s.fill.clone()));
        if !s.label.is_empty() || !matches!(row.get("Label"), ParamValue::None) {
            row.set("Label", ParamValue::Str(s.label.clone()));
        }
        row.set(
            "Time",
            ParamValue::Double(if time_mode { s.time } else { 0. }),
        );
        row
    };
    for row in p.records("MorphPolygonData") {
        if let Some(shape) = shape_id(row).and_then(|id| shapes.iter().find(|s| s.id == id)) {
            used.insert(shape.id);
            rows.push(update(row.clone(), shape));
        }
    }
    for shape in shapes {
        if used.insert(shape.id) {
            rows.push(update(Params::defaults(desc.params), shape));
        }
    }
    if time_mode {
        rows.sort_by(|a, b| a.double("Time").total_cmp(&b.double("Time")));
    }
    if p.enum_name("PolygonType") == "Pattern" {
        if let Some(first) = rows.first() {
            result.set("FillType", first.get("FillType").clone());
        }
    }
    result.set("MorphPolygonData", ParamValue::RecordList(rows));
    result
}
pub fn apply_to_sequence(
    s: &mut SequenceSession,
    effects: &[EffectId],
    ed: &Editor,
) -> Option<String> {
    let shapes = ed.finish();
    let mut before = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in effects {
        if !seen.insert(*id) {
            continue;
        }
        let Some(e) = s.effect(*id) else {
            continue;
        };
        if vixen_effect_registry::by_type_id(&e.node.type_id)
            .is_none_or(|d| d.data_type != "MorphData")
        {
            continue;
        }
        let old = e.node.params.clone();
        let new = apply_shapes(&old, &shapes);
        if new != old {
            before.push((*id, old));
            s.set_params(*id, new);
        }
    }
    if before.is_empty() {
        return None;
    }
    let action = UndoAction::ParamsChanged {
        label: "Polygons".into(),
        params: before,
    };
    let label = action.description();
    s.record_undo(action);
    s.render_all_dirty();
    Some(label)
}
