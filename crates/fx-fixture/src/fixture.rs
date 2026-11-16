//! Port of VixenModules.Effect.Fixture (FixtureModule, FixtureData, FixtureFunctionData,
//! FixtureFunctionExpando).

use crate::fixture_base::{number_of_frames, render_curve, render_indexed, FixtureTargets, LabeledNode};
use crate::generated::fixture as gen;
use crate::spec::{FixtureFunction, FixtureFunctionType, FixtureIndexItem};
use crate::visual;
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;
use vixen_effects::pulse_renderer;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    targets_changed: Some(targets_changed),
    param_changed_ctx: Some(param_changed_ctx),
    force_visual: true,
    visual: Some(visual_representation),
    selection: Some(selection),
    ported: true,
    ..gen::BASE
};

/// A fixture function as `FixtureData.FixtureFunctions` stores it.
fn function_record(f: &FixtureFunction) -> Params {
    let index_item = |i: &FixtureIndexItem| {
        let mut r = Params::defaults(&gen::FIXTURE_INDEX_PARAMS);
        r.set("Name", ParamValue::Str(i.name.clone()));
        r.set("EndValue", ParamValue::Int(i.end_value));
        r.set("IndexType", ParamValue::Enum(i.index_type.name().to_string()));
        r.set("StartValue", ParamValue::Int(i.start_value));
        r.set("UseCurve", ParamValue::Bool(i.use_curve));
        r.set("Image", ParamValue::Str(i.image.clone()));
        r
    };
    let wheel_item = |i: &FixtureIndexItem| {
        let mut r = Params::defaults(&gen::FIXTURE_COLOR_WHEEL_PARAMS);
        r.set("Name", ParamValue::Str(i.name.clone()));
        r.set("EndValue", ParamValue::Int(i.end_value));
        r.set("IndexType", ParamValue::Enum(i.index_type.name().to_string()));
        r.set("StartValue", ParamValue::Int(i.start_value));
        r.set("Color1Html", ParamValue::Str(crate::spec::color_to_html(i.color1)));
        r.set("Color2Html", ParamValue::Str(crate::spec::color_to_html(i.color2)));
        r.set("ExcludeColorProperty", ParamValue::Bool(i.exclude_color_property));
        r.set("HalfStep", ParamValue::Bool(i.half_step));
        r.set("UseCurve", ParamValue::Bool(i.use_curve));
        r
    };
    let mut limits = Params::defaults(&gen::FIXTURE_ROTATION_LIMITS_PARAMS);
    limits.set("StartPosition", ParamValue::Int(f.rotation_limits.start_position));
    limits.set("StopPosition", ParamValue::Int(f.rotation_limits.stop_position));

    let mut r = Params::defaults(&gen::FIXTURE_FUNCTION_PARAMS);
    r.set("Name", ParamValue::Str(f.name.clone()));
    r.set("AssociatedFunctionName", ParamValue::Str(f.associated_function_name.clone()));
    r.set("ColorWheelData", ParamValue::RecordList(f.color_wheel_data.iter().map(wheel_item).collect()));
    r.set("FunctionIdentity", ParamValue::Enum(f.identity.name().to_string()));
    r.set("FunctionType", ParamValue::Enum(f.function_type.name().to_string()));
    r.set("IndexData", ParamValue::RecordList(f.index_data.iter().map(index_item).collect()));
    r.set("Label", ParamValue::Str(f.label.clone()));
    r.set("RotationLimits", ParamValue::Record(Box::new(limits)));
    r.set("TimelineColor1Html", ParamValue::Str(crate::spec::color_to_html(f.timeline_color)));
    let zoom = if f.zoom_type == crate::spec::FixtureZoomType::NarrowToWide { "NarrowToWide" } else { "WideToNarrow" };
    r.set("ZoomType", ParamValue::Enum(zoom.to_string()));
    r
}

/// `UpdateFixtureCapabilities`: drops the settings for functions the new targets don't have and selects the first function
/// in the drop down. `UpdateModelData` keeps the targets' functions in the data, where the rows read their index values.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    let functions = FixtureTargets::from_info(*t).fixture_functions();
    let mut rows = p.records("FunctionData").to_vec();
    rows.retain(|r| functions.iter().any(|f| f.name == r.string("FunctionName")));
    p.set("FixtureFunctions", ParamValue::RecordList(functions.iter().map(function_record).collect()));
    p.set("FunctionData", ParamValue::RecordList(rows));
    if let Some(first) = functions.first() {
        p.set("SelectedFunction", ParamValue::Str(first.name.clone()));
    }
    repair_row_indexes(p);
}

/// `UpdateFunctionViewModel` builds a row with the `IndexValue` and `ColorIndexValue` setters, which replace a value the
/// function's list doesn't have with its first entry.
fn repair_row_indexes(p: &mut Params) {
    let mut rows = p.records("FunctionData").to_vec();
    for row in rows.iter_mut() {
        let index = row_index_names(row, p, "IndexData");
        let wheel = row_index_names(row, p, "ColorWheelData");
        for (name, list) in [("IndexValue", index), ("ColorIndexValue", wheel)] {
            let value = row.string(name);
            if !value.is_empty() && !list.iter().any(|n| n == value) {
                if let Some(first) = list.first() {
                    let first = first.clone();
                    row.set(name, ParamValue::Str(first));
                }
            }
        }
    }
    p.set("FunctionData", ParamValue::RecordList(rows));
}

/// The names in one list of a row's function, from `FixtureData.FixtureFunctions`.
fn row_index_names(row: &Params, effect: &Params, list: &str) -> Vec<String> {
    effect
        .records("FixtureFunctions")
        .iter()
        .find(|f| f.string("Name") == row.string("FunctionName"))
        .map(|f| f.records(list).iter().map(|i| i.string("Name").to_string()).collect())
        .unwrap_or_default()
}

/// `FunctionCollectionNameConverter`, and for a row the index names of its function.
fn selection(name: &str, row: Option<&Params>, p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    match (name, row) {
        ("SelectedFunction", _) => {
            let mut names: Vec<String> = Vec::new();
            for f in FixtureTargets::from_info(*t).fixture_functions() {
                if !names.contains(&f.name) {
                    names.push(f.name);
                }
            }
            Some(names.into_iter().map(SelectionValue::text).collect())
        }
        ("IndexValue", Some(r)) => Some(row_index_names(r, p, "IndexData").into_iter().map(SelectionValue::text).collect()),
        ("ColorIndexValue", Some(r)) => Some(row_index_names(r, p, "ColorWheelData").into_iter().map(SelectionValue::text).collect()),
        _ => None,
    }
}

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if c.name == MODULE_DATA {
        // The C# module keeps this selection outside ModuleData. Target assignment
        // initializes it before the saved data replaces the defaults.
        let functions = FixtureTargets::from_info(*c.info).fixture_functions();
        p.set("SelectedFunction", ParamValue::Str(functions.first().map(|f| f.name.clone()).unwrap_or_default()));
        repair_row_indexes(p);
    } else if c.name == "FunctionData" {
        // UpdateModelData stores the current target capabilities when the
        // expando collection is committed. A newly added row must not read
        // obsolete index choices/UseCurve flags from the sequence's old copy.
        let functions = FixtureTargets::from_info(*c.info).fixture_functions();
        p.set("FixtureFunctions", ParamValue::RecordList(functions.iter().map(function_record).collect()));
    }
}

/// `FixtureFunctionExpando()` for the editor's Add button. The selected function
/// belongs to this effect's targets (the upstream static ActiveFunction is only
/// a bridge from that selection to the expando constructor).
pub fn new_function_data(p: &Params, t: &TargetInfo) -> Option<Params> {
    let mut row = Params::defaults(&gen::FIXTURE_FUNCTION_DATA_PARAMS);
    row.set("Color", ParamValue::Gradient(ColorGradient::solid(Color::WHITE)));
    let functions = FixtureTargets::from_info(*t).fixture_functions();
    let function = functions.iter().find(|f| f.name == p.string("SelectedFunction"))?;
    row.set("FunctionName", ParamValue::Str(function.name.clone()));
    row.set("FunctionIdentity", ParamValue::Enum(function.identity.name().into()));
    row.set("FunctionType", ParamValue::Enum(function.function_type.name().into()));
    row.set("TimelineColor", ParamValue::Color(function.timeline_color));
    if let Some(first) = function.index_data.first() {
        row.set("IndexValue", ParamValue::Str(first.name.clone()));
    }
    if let Some(first) = function.color_wheel_data.first() {
        row.set("ColorIndexValue", ParamValue::Str(first.name.clone()));
    }
    // UpdateModelData serializes RGB/W Intensity as Range; other functions
    // retain the constructor's zero Range.
    if matches!(function.function_type, FixtureFunctionType::RgbColor | FixtureFunctionType::RgbwColor) {
        row.set("Range", ParamValue::Curve(Curve::flat100()));
    }
    Some(row)
}

/// `FixtureFunctionExpando.IndexMapsToRange`, which the index setters work out when a row is built.
fn index_maps_to_range(row: &Params, effect: &Params) -> bool {
    let Some(function) = effect.records("FixtureFunctions").iter().find(|f| f.string("Name") == row.string("FunctionName")) else {
        return false;
    };
    let uses_curve = |value: &str, list: &str| {
        if value.is_empty() {
            return false;
        }
        let items = function.records(list);
        items.iter().find(|i| i.string("Name") == value).or(items.first()).is_some_and(|i| i.boolean("UseCurve"))
    };
    uses_curve(row.string("IndexValue"), "IndexData") || uses_curve(row.string("ColorIndexValue"), "ColorWheelData")
}

/// `FixtureFunctionExpando.UpdateFunctionTypeAttributes`. The C# row has an `Intensity` curve for the colour functions and a
/// `Range` curve for the others, and `UpdateModelData` stores whichever applies as `Range`: here both are the one `Range`.
pub(crate) fn function_row_visibility(row: &Params, effect: &Params, v: &mut Visibility) {
    let ty = row.enum_name("FunctionType");
    let rgb = ty == "RGBColor" || ty == "RGBWColor";
    v.set("IndexValue", ty == "Indexed");
    v.set("Range", ty == "Range" || rgb || index_maps_to_range(row, effect));
    v.set("ColorIndexValue", ty == "ColorWheel");
    v.set("Color", rgb);
}

/// One entry of `FixtureData.FunctionData`, as the render needs it.
struct FunctionSetting {
    name: String,
    function_type: FixtureFunctionType,
    range: Curve,
    index_value: String,
    color_index_value: String,
    color: ColorGradient,
}

fn read_settings(p: &Params) -> Vec<FunctionSetting> {
    p.records("FunctionData")
        .iter()
        .map(|r| FunctionSetting {
            name: r.string("FunctionName").to_string(),
            function_type: FixtureFunctionType::from_name(r.enum_name("FunctionType")),
            range: r.curve("Range"),
            index_value: r.string("IndexValue").to_string(),
            color_index_value: r.string("ColorIndexValue").to_string(),
            color: r.gradient("Color"),
        })
        .collect()
}

/// The `IndexValue` / `ColorIndexValue` setters of `FixtureFunctionExpando`: a value that is not in the
/// function's list becomes the first entry.
fn repaired_index(value: &str, items: &[FixtureIndexItem]) -> String {
    if !value.is_empty() && !items.iter().any(|i| i.name == value) {
        if let Some(first) = items.first() {
            return first.name.clone();
        }
    }
    value.to_string()
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let tf = FixtureTargets::new(ctx);
    let mut out = EffectIntents::new();

    // UpdateFixtureCapabilities: the functions of the first target's fixtures. Settings for functions the
    // fixtures do not have are dropped (which also covers the blank name of an effect on a non-fixture).
    let fixture_functions = tf.fixture_functions();

    for setting in read_settings(p) {
        if setting.name.is_empty() {
            continue;
        }
        let Some(function) = fixture_functions.iter().find(|f| f.name == setting.name) else { continue };
        match setting.function_type {
            FixtureFunctionType::Range => {
                let nodes = render_nodes_for_function_type(&tf, &setting.name, |t| t == FixtureFunctionType::Range);
                if !render_curve(ctx, &mut out, &nodes, &setting.range, function.identity, &function.name, None) {
                    break;
                }
            }
            FixtureFunctionType::Indexed => {
                let index = repaired_index(&setting.index_value, &function.index_data);
                if render_indexed(ctx, &tf, &mut out, &index, &setting.range, function).is_err() {
                    break;
                }
            }
            FixtureFunctionType::ColorWheel => {
                let index = repaired_index(&setting.color_index_value, &function.color_wheel_data);
                if render_indexed(ctx, &tf, &mut out, &index, &setting.range, function).is_err() {
                    break;
                }
            }
            FixtureFunctionType::RgbColor | FixtureFunctionType::RgbwColor => {
                if !render_rgb(ctx, &tf, &mut out, &setting.color, &setting.range, function) {
                    break;
                }
            }
            // The C# asserts on "Unsupported Function Type" and renders nothing.
            FixtureFunctionType::None => {}
        }
        if ctx.cancelled() {
            break;
        }
    }
    out
}

/// `GetRenderNodesForRangeFunctionType` / `GetRenderNodesForRGBFunctionType`: every leaf of the first target
/// (a leaf under two parents counts twice, as `GetLeafEnumerator` yields it) whose fixture supports the
/// function with a type the filter accepts.
fn render_nodes_for_function_type(
    tf: &FixtureTargets,
    function_name: &str,
    accept: impl Fn(FixtureFunctionType) -> bool,
) -> Vec<LabeledNode> {
    let mut out = Vec::new();
    let Some(node) = tf.first_target() else { return out };
    for leaf in tf.info.tree.leaves(node) {
        let Some(spec) = tf.spec(leaf) else { continue };
        if spec.supports_function_name(function_name) {
            if let Some(f) = spec.functions.iter().find(|f| f.name == function_name && accept(f.function_type)) {
                out.push((leaf, f.label.clone()));
            }
        }
    }
    out
}

/// `RenderRGB`. The C# runs the pulse for every node once per frame inside the frame loop, so each pulse's
/// intents are added `GetNumberFrames()` times at the same position. That is wasteful but it is what the
/// effect outputs, so it is kept; the pulse is rendered once and the result repeated.
fn render_rgb(ctx: &RenderCtx, tf: &FixtureTargets, out: &mut EffectIntents, gradient: &ColorGradient, intensity: &Curve, function: &FixtureFunction) -> bool {
    let nodes = render_nodes_for_function_type(tf, &function.name, |t| {
        t == FixtureFunctionType::RgbColor || t == FixtureFunctionType::RgbwColor
    });
    if nodes.is_empty() {
        return true;
    }
    let frames = number_of_frames(ctx);
    if frames <= 0 {
        return true;
    }
    // Pulse uses Curve.GetValue too; the C# effect aborts at this row on malformed curves.
    if intensity.points.len() < 2 {
        return false;
    }
    let pulses: Vec<EffectIntents> = nodes
        .iter()
        .map(|(node, _)| pulse_renderer::render_node(ctx.tree, *node, intensity, gradient, ctx.time_span, false, false))
        .collect();
    for _ in 0..frames {
        for pulse in &pulses {
            out.add(pulse.clone());
        }
    }
    true
}

/// `GenerateVisualRepresentation`: horizontal bars in the timeline colours of the functions, with the
/// effect name over them.
fn visual_representation(p: &Params, _t: &TargetInfo, g: &mut Graphics, clip: (i32, i32, i32, i32)) {
    let clipping = (clip.0, clip.1, clip.2, clip.3 - 4);
    let colors: Vec<Color> = p.records("FunctionData").iter().map(|r| r.color("TimelineColor")).collect();
    draw_horizontal_bars(g, (clip.2, clip.3), &colors);
    let text_color = if colors.is_empty() { Color::WHITE } else { Color::BLACK };
    visual::draw_text(g, clipping, text_color, "Fixture", 0, 0, clipping.3);
}

/// `DrawHorizontalBars`, drawn at the canvas origin as `graphics.DrawImage(bars, 0, 0)` does.
fn draw_horizontal_bars(g: &mut Graphics, size: (i32, i32), colors: &[Color]) {
    let default = [Color::BLACK];
    let colors: &[Color] = if colors.is_empty() { &default } else { colors };
    let height_of_bar = size.1 / colors.len() as i32;
    let mut y = 0;
    for (i, c) in colors.iter().enumerate() {
        let h = if i == colors.len() - 1 { size.1 - y } else { height_of_bar };
        g.fill_rectangle(*c, 0.0, y as f32, size.0 as f32, h as f32);
        y += height_of_bar;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_base::test_support::*;
    use crate::spec::{FixtureChannel, FixtureFunction as SpecFunction, FixtureSpecification, FuncId};
    use vixen_core::StateValue;
    use vixen_effects::testing;

    fn setting(name: &str, ty: &str, identity: &str) -> Params {
        let mut r = Params::defaults(&gen::FIXTURE_FUNCTION_DATA_PARAMS);
        r.set("FunctionName", ParamValue::Str(name.into()));
        r.set("FunctionType", ParamValue::Enum(ty.into()));
        r.set("FunctionIdentity", ParamValue::Enum(identity.into()));
        r
    }

    fn with_settings(list: Vec<Params>) -> impl FnOnce(&mut Params) {
        move |p| {
            p.set("FunctionData", ParamValue::RecordList(list));
        }
    }

    fn rich_spec() -> FixtureSpecification {
        let mut spec = sample_spec();
        spec.functions.iter_mut().find(|f| f.name == "Gobo Wheel").unwrap().index_data = vec![
            FixtureIndexItem { name: "Open".into(), start_value: 7, end_value: 7, ..FixtureIndexItem::index() },
            FixtureIndexItem { name: "Star".into(), start_value: 20, end_value: 20, ..FixtureIndexItem::index() },
        ];
        spec.functions.push(SpecFunction {
            name: "Color".into(),
            function_type: FixtureFunctionType::RgbColor,
            identity: FuncId::Custom,
            label: "C".into(),
            ..Default::default()
        });
        spec.channels.push(FixtureChannel { name: "R".into(), function: "Color".into(), channel_number: 20 });
        spec
    }

    #[test]
    fn smoke() {
        // Plain elements have no fixtures and the default effect has no functions: nothing renders.
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn assigning_targets_drops_other_functions_and_stores_the_fixture_functions() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &rich_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        p.set("FunctionData", ParamValue::RecordList(vec![setting("Pan", "Range", "Pan"), setting("Gone", "Range", "Custom")]));
        DESCRIPTOR.set_targets(&mut p, &t);
        let rows: Vec<&str> = p.records("FunctionData").iter().map(|r| r.string("FunctionName")).collect();
        assert_eq!(rows, ["Pan"]);
        assert_eq!(p.string("SelectedFunction"), "Pan");
        assert!(p.records("FixtureFunctions").iter().any(|f| f.string("Name") == "Gobo Wheel"));

        // A loaded effect keeps its rows: the render skips what the fixture doesn't have.
        let mut loaded = DESCRIPTOR.new_params();
        loaded.set("FunctionData", ParamValue::RecordList(vec![setting("Gone", "Range", "Custom")]));
        DESCRIPTOR.load_data(&mut loaded, &t);
        assert_eq!(loaded.records("FunctionData").len(), 1);
    }

    #[test]
    fn the_drop_downs_come_from_the_fixture() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &rich_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        let labels = |v: Vec<SelectionValue>| v.into_iter().map(|c| c.label).collect::<Vec<_>>();
        let functions = labels(DESCRIPTOR.selection_values("SelectedFunction", None, &p, &t));
        assert_eq!(functions.iter().filter(|n| *n == "Gobo Wheel").count(), 1, "two heads, one entry");
        let row = setting("Gobo Wheel", "Indexed", "Gobo");
        assert_eq!(labels(DESCRIPTOR.selection_values("IndexValue", Some(&row), &p, &t)), ["Open", "Star"]);
    }

    #[test]
    fn new_function_rows_copy_the_selected_capability_and_its_constructor_defaults() {
        let mut spec = rich_spec();
        for (name, ty) in [("White", FixtureFunctionType::RgbwColor), ("Wheel", FixtureFunctionType::ColorWheel)] {
            spec.functions.push(SpecFunction {
                name: name.into(), function_type: ty, timeline_color: Color::BLUE,
                color_wheel_data: vec![FixtureIndexItem { name: "Red".into(), start_value: 31, end_value: 31, ..FixtureIndexItem::color_wheel() }],
                ..Default::default()
            });
            spec.channels.push(FixtureChannel { name: name.into(), function: name.into(), channel_number: spec.channels.len() as i32 + 1 });
        }
        spec.functions[0].timeline_color = Color::RED;
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &spec);
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        for (name, ty, range, index, wheel, color) in [
            ("Pan", "Range", 0.0, "", "", Color::RED),
            ("Gobo Wheel", "Indexed", 0.0, "Open", "", Color::WHITE),
            ("Color", "RGBColor", 100.0, "", "", Color::WHITE),
            ("White", "RGBWColor", 100.0, "", "Red", Color::BLUE),
            ("Wheel", "ColorWheel", 0.0, "", "Red", Color::BLUE),
        ] {
            p.set("SelectedFunction", ParamValue::Str(name.into()));
            let row = new_function_data(&p, &t).unwrap();
            assert_eq!(row.string("FunctionName"), name);
            assert_eq!(row.enum_name("FunctionType"), ty);
            assert_eq!(row.curve("Range"), Curve::flat(range));
            assert_eq!(row.string("IndexValue"), index);
            assert_eq!(row.string("ColorIndexValue"), wheel);
            assert_eq!(row.color("TimelineColor"), color);
            assert_eq!(row.gradient("Color"), ColorGradient::solid(Color::WHITE));
        }
        p.set("SelectedFunction", ParamValue::Str("Pan".into()));
        assert_eq!(new_function_data(&p, &t).unwrap().enum_name("FunctionIdentity"), "Pan");
        p.set("SelectedFunction", ParamValue::Str("Gone".into()));
        assert!(new_function_data(&p, &t).is_none());
        assert!(new_function_data(&p, &TargetInfo::detached()).is_none());
    }

    #[test]
    fn loading_hydrates_only_the_transient_function_selection() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &rich_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        p.set("FunctionData", ParamValue::RecordList(vec![setting("Gone", "Range", "Custom")]));
        p.set("SelectedFunction", ParamValue::Str(String::new()));
        let rows = p.records("FunctionData").to_vec();
        let functions = p.records("FixtureFunctions").to_vec();
        DESCRIPTOR.load_data(&mut p, &t);
        assert_eq!(p.string("SelectedFunction"), "Pan");
        assert_eq!(p.records("FunctionData"), rows);
        assert_eq!(p.records("FixtureFunctions"), functions);
        DESCRIPTOR.load_data(&mut p, &TargetInfo::detached());
        assert!(p.string("SelectedFunction").is_empty());
        assert_eq!(p.records("FunctionData"), rows);
    }

    #[test]
    fn new_rows_use_first_targets_capabilities_and_first_shared_function_definition() {
        let mut tree = ElementTree::new();
        let mut a = rich_spec();
        a.functions[0].timeline_color = Color::RED;
        let first = fixtures(&mut tree, "First", 2, &a);
        let mut b = rich_spec();
        b.functions[0].timeline_color = Color::BLUE;
        b.functions.push(SpecFunction { name: "Other only".into(), ..Default::default() });
        b.channels.push(FixtureChannel { name: "Other only".into(), function: "Other only".into(), channel_number: 22 });
        let other = fixtures(&mut tree, "Other", 1, &b);
        let targets = [first, other];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(new_function_data(&p, &t).unwrap().color("TimelineColor"), Color::RED);
        p.set("SelectedFunction", ParamValue::Str("Other only".into()));
        assert!(new_function_data(&p, &t).is_none(), "same first-target rule as the converter/render");

        let group = tree.add_node(Uuid::new_v4(), "Mixed profiles", None);
        tree.add_child(group, first);
        tree.add_child(group, other);
        let plain = tree.add_element("No fixture");
        tree.add_child(group, plain);
        let targets = [group];
        let t = testing::target_info(&tree, &targets);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(new_function_data(&p, &t).unwrap().color("TimelineColor"), Color::RED);
        p.set("SelectedFunction", ParamValue::Str("Other only".into()));
        let row = new_function_data(&p, &t).unwrap();
        let out = testing::render_with(&DESCRIPTOR, &tree, &targets, 500, with_settings(vec![row]));
        assert_eq!(out.map.len(), 1, "mixed fixture groups only render on supporting leaves");
    }

    #[test]
    fn a_row_shows_what_its_function_type_uses() {
        let mut spec = rich_spec();
        spec.functions.iter_mut().find(|f| f.name == "Gobo Wheel").unwrap().index_data[1].use_curve = true;
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &spec);
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        let shown = |row: &Params, p: &Params, name: &'static str| {
            let mut v = Visibility::default();
            (gen::FIXTURE_FUNCTION_DATA.visibility.unwrap())(row, p, &mut v);
            v.overrides.iter().find(|o| o.0 == name).map(|o| o.1).unwrap()
        };

        let range = setting("Pan", "Range", "Pan");
        assert!(shown(&range, &p, "Range") && !shown(&range, &p, "IndexValue") && !shown(&range, &p, "Color"));

        let mut indexed = setting("Gobo Wheel", "Indexed", "Gobo");
        indexed.set("IndexValue", ParamValue::Str("Open".into()));
        assert!(shown(&indexed, &p, "IndexValue") && !shown(&indexed, &p, "Range"));
        indexed.set("IndexValue", ParamValue::Str("Star".into()));
        assert!(shown(&indexed, &p, "Range"), "an index with a curve maps to the range");

        let color = setting("Color", "RGBColor", "Custom");
        assert!(shown(&color, &p, "Color") && shown(&color, &p, "Range") && !shown(&color, &p, "ColorIndexValue"));
    }

    #[test]
    fn a_missing_index_becomes_the_first_entry_when_the_data_is_assigned() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &rich_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut placed = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut placed, &t);
        let mut row = setting("Gobo Wheel", "Indexed", "Gobo");
        row.set("IndexValue", ParamValue::Str("Gone".into()));
        placed.set("FunctionData", ParamValue::RecordList(vec![row]));
        DESCRIPTOR.load_data(&mut placed, &t);
        assert_eq!(placed.records("FunctionData")[0].string("IndexValue"), "Open");
    }

    #[test]
    fn range_function_renders_its_curve_with_the_fixture_identity() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &rich_spec());
        let mut pan = setting("Pan", "Range", "Pan");
        pan.set("Range", ParamValue::Curve(Curve::ramp_up()));
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, with_settings(vec![pan]));
        assert_eq!(out.map.len(), 2);
        for nodes in out.map.values() {
            match nodes[0].intent.state_at(TimeSpan::from_ms_i(400)) {
                StateValue::Range(v) => {
                    assert_eq!(v.value, 0.4);
                    assert_eq!(&*v.tag, "Pan");
                    assert_eq!(&*v.label, "P");
                    assert_eq!(v.tag_type, vixen_core::value::FunctionIdentity::Pan);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn functions_the_fixture_lacks_are_dropped() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &rich_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, with_settings(vec![setting("Prism", "Range", "Prism")]));
        assert!(out.is_empty());
    }

    #[test]
    fn indexed_function_holds_the_index_and_unknown_values_fall_back_to_the_first() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &rich_spec());
        let mut gobo = setting("Gobo Wheel", "Indexed", "Gobo");
        gobo.set("IndexValue", ParamValue::Str("Star".into()));
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 500, with_settings(vec![gobo]));
        let nodes = out.map.values().next().unwrap();
        assert_eq!(nodes.len(), 10);
        assert!(matches!(nodes[0].intent.state_at(TimeSpan::ZERO), StateValue::Command(c) if c.as_u8() == Some(20)));

        let mut stale = setting("Gobo Wheel", "Indexed", "Gobo");
        stale.set("IndexValue", ParamValue::Str("Removed".into()));
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 500, with_settings(vec![stale]));
        assert!(matches!(out.map.values().next().unwrap()[0].intent.state_at(TimeSpan::ZERO), StateValue::Command(c) if c.as_u8() == Some(7)));
    }

    #[test]
    fn rgb_function_adds_the_pulse_once_per_frame() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &rich_spec());
        let mut color = setting("Color", "RGBColor", "Custom");
        color.set("Color", ParamValue::Gradient(ColorGradient::solid(Color::RED)));
        color.set("Range", ParamValue::Curve(Curve::flat100()));
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 500, with_settings(vec![color]));
        let nodes = out.map.values().next().unwrap();
        // 500 ms / 50 ms = 10 frames, each repeating the same single-segment pulse (deliberate C# quirk)
        assert_eq!(nodes.len(), 10);
        match nodes[3].intent.state_at(TimeSpan::from_ms_i(100)) {
            StateValue::Lighting(v) => {
                assert_eq!(v.color, Color::RED);
                assert_eq!(v.intensity, 1.0);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn thumbnail_stacks_a_bar_per_function() {
        let mut p = DESCRIPTOR.new_params();
        let mut a = setting("Pan", "Range", "Pan");
        a.set("TimelineColor", ParamValue::Color(Color::RED));
        let mut b = setting("Tilt", "Range", "Tilt");
        b.set("TimelineColor", ParamValue::Color(Color::BLUE));
        p.set("FunctionData", ParamValue::RecordList(vec![a, b]));
        let mut g = Graphics::new(100, 40);
        (DESCRIPTOR.visual.unwrap())(&p, &TargetInfo::detached(), &mut g, (0, 0, 100, 40));
        assert_eq!(g.bitmap().get_pixel(98, 2), Color::RED);
        assert_eq!(g.bitmap().get_pixel(98, 38), Color::BLUE);
    }

    #[test]
    fn thumbnail_without_functions_is_black_with_white_text() {
        let mut g = Graphics::new(100, 40);
        (DESCRIPTOR.visual.unwrap())(&DESCRIPTOR.new_params(), &TargetInfo::detached(), &mut g, (0, 0, 100, 40));
        assert_eq!(g.bitmap().get_pixel(98, 38), Color::BLACK);
        assert!(crate::visual::has_ink(&g));
    }
}
