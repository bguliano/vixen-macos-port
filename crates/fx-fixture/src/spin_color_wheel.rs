//! Port of VixenModules.Effect.SpinColorWheel (SpinColorWheelModule, SpinColorWheelData).

use crate::fixture_base::{index_effect_selection, render_curve, render_index_effect, supports_index_function, update_index_capabilities, FixtureTargets};
use crate::generated::spin_color_wheel as gen;
use crate::spec::{FixtureIndexItem, FuncId};
use crate::visual;
use std::collections::HashMap;
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    targets_changed: Some(targets_changed),
    force_visual: true,
    visual: Some(visual_representation),
    selection: Some(selection),
    ported: true,
    ..gen::BASE
};

/// `GetCompatibleIndexValues`: only index values that support curves.
fn curve_index(i: &FixtureIndexItem) -> bool {
    i.use_curve
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let tf = FixtureTargets::new(ctx);
    let mut out = EffectIntents::new();
    // The wheel itself
    if render_index_effect(
        ctx,
        &tf,
        &mut out,
        p.string("SpinColorWheelFunctionName"),
        p.string("SpinColorWheelIndexValue"),
        &p.curve("SpinColorWheelCurve"),
        FuncId::SpinColorWheel,
    )
    .is_err()
    {
        return EffectIntents::new();
    }

    // RenderCurve(Intensity, FunctionIdentity.Dim, _dimmerTags)
    let mut tags: HashMap<NodeId, String> = HashMap::new();
    let nodes = tf.render_nodes_for_identity(FuncId::Dim, &mut tags);
    render_curve(ctx, &mut out, &nodes, &p.curve("Intensity"), FuncId::Dim, "", Some(&tags));
    out
}

fn selection(name: &str, _row: Option<&Params>, p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    index_effect_selection(name, p, t, ("SpinColorWheelFunctionName", "SpinColorWheelIndexValue"), FuncId::SpinColorWheel, curve_index)
}

/// `UpdateFixtureCapabilities`.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    update_index_capabilities(p, t, "SpinColorWheelFunctionName", "SpinColorWheelIndexValue", FuncId::SpinColorWheel, curve_index, true);
}

/// `IndexHasCurve`. The C# caches this in the index setter, so a loaded effect holds whatever its default data gave until
/// the index is next changed; worked out from the values here, it is right from the start.
fn index_has_curve(tf: &FixtureTargets, p: &Params) -> bool {
    tf.index_has_curve(p.string("SpinColorWheelIndexValue"), p.string("SpinColorWheelFunctionName"), FuncId::SpinColorWheel)
}

/// `UpdateAttributes`.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let tf = FixtureTargets::from_info(*t);
    v.set("SpinColorWheelIndexValue", supports_index_function(&tf, FuncId::SpinColorWheel));
    v.set("SpinColorWheelCurve", index_has_curve(&tf, p));
    let mut tags: HashMap<NodeId, String> = HashMap::new();
    v.set("Intensity", !tf.render_nodes_for_identity(FuncId::Dim, &mut tags).is_empty());
}

/// `GenerateVisualRepresentation`: the curve over the wheel's solid colours when the index uses one, otherwise the label.
/// With no fixture among the target's leaves, or no function selected, the C# draws nothing.
fn visual_representation(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    let tf = FixtureTargets::from_info(*t);
    let Some(first) = t.targets.first() else { return };
    let Some(leaf) = tf.leaves_with_fixture(*first).first().copied() else { return };
    let name = p.string("SpinColorWheelFunctionName");
    if name.is_empty() {
        return;
    }
    let Some(spec) = tf.spec(leaf) else { return };
    // GetInUseFunction returning nothing is an exception in the C#, so the box stays empty.
    let Some(func) = spec.get_in_use_function(name, FuncId::SpinColorWheel) else { return };
    let mut colors: Vec<Color> = Vec::new();
    for entry in func.color_wheel_data.iter().filter(|e| !e.use_curve && !e.half_step) {
        if !colors.contains(&entry.color1) {
            colors.push(entry.color1);
        }
    }
    if index_has_curve(&tf, p) {
        visual::draw_curve_with_background(g, rect, Color::BLACK, "Spin Color Wheel", 2, 1, &p.curve("SpinColorWheelCurve"), 0, &colors);
    } else {
        visual::draw_text(g, rect, visual::HOT_PINK, "Spin Color Wheel", 0, rect.1, rect.3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_base::test_support::*;
    use crate::spec::{FixtureChannel, FixtureFunction, FixtureFunctionType, FixtureSpecification};
    use vixen_core::StateValue;
    use vixen_effects::testing;

    fn wheel_spec() -> FixtureSpecification {
        let mut spec = sample_spec();
        spec.functions.push(FixtureFunction {
            name: "Color Wheel".into(),
            function_type: FixtureFunctionType::ColorWheel,
            identity: FuncId::SpinColorWheel,
            color_wheel_data: vec![
                FixtureIndexItem { name: "Red".into(), start_value: 5, end_value: 5, color1: Color::RED, ..FixtureIndexItem::color_wheel() },
                FixtureIndexItem {
                    name: "Spin".into(),
                    start_value: 128,
                    end_value: 255,
                    use_curve: true,
                    ..FixtureIndexItem::color_wheel()
                },
            ],
            ..Default::default()
        });
        spec.channels.push(FixtureChannel { name: "Wheel".into(), function: "Color Wheel".into(), channel_number: 9 });
        spec
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn plain_elements_produce_nothing() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, |_| {}).is_empty());
    }

    #[test]
    fn spins_with_the_curve_and_dims_with_the_intensity_curve() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &wheel_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |_| {});
        let nodes = out.map.values().next().unwrap();
        let commands: Vec<u8> = nodes
            .iter()
            .filter_map(|n| match n.intent.state_at(TimeSpan::ZERO) {
                StateValue::Command(c) => c.as_u8(),
                _ => None,
            })
            .collect();
        // Only the curve-capable index is offered, so "Spin" is chosen: 128 up towards 255
        assert_eq!(commands.len(), 20);
        assert_eq!(commands[0], 128);
        assert_eq!(commands[10], 128 + 64);
        // The intensity curve is flat 100%: one dimmer intent at full
        let dim = nodes.iter().find_map(|n| match n.intent.state_at(TimeSpan::ZERO) {
            StateValue::Range(v) => Some(v),
            _ => None,
        });
        let dim = dim.expect("dimmer intent");
        assert_eq!(dim.value, 1.0);
        assert_eq!(&*dim.tag, "Dimmer");
    }

    #[test]
    fn assigning_targets_repairs_the_selection_and_loading_keeps_it() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &wheel_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        p.set("SpinColorWheelFunctionName", ParamValue::Str("Gone".into()));
        let mut loaded = p.clone();
        DESCRIPTOR.load_data(&mut loaded, &t);
        assert_eq!(loaded.string("SpinColorWheelFunctionName"), "Gone");
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(p.string("SpinColorWheelFunctionName"), "Color Wheel");
        assert_eq!(p.string("SpinColorWheelIndexValue"), "Spin");
    }

    #[test]
    fn a_saved_function_the_targets_no_longer_have_renders_nothing() {
        let mut tree = ElementTree::new();
        let mut spec = wheel_spec();
        spec.functions.retain(|f| f.name != "Dimmer");
        spec.channels.retain(|c| c.function != "Dimmer");
        let heads = fixtures(&mut tree, "Heads", 1, &spec);
        let out = testing::render_loaded(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("SpinColorWheelFunctionName", ParamValue::Str("Gone".into()));
            p.set("SpinColorWheelIndexValue", ParamValue::Str("Spin".into()));
        });
        assert!(out.is_empty());
    }

    #[test]
    fn the_fixture_decides_what_the_editor_offers() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &wheel_spec());
        let plain = tree.add_string("S", 2);
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        let names = testing::visible_names(&DESCRIPTOR, &p, &t);
        assert!(names.contains(&"SpinColorWheelIndexValue") && names.contains(&"SpinColorWheelCurve") && names.contains(&"Intensity"));
        let plain_targets = [plain];
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &plain_targets));
        assert!(!names.contains(&"SpinColorWheelIndexValue") && !names.contains(&"SpinColorWheelCurve") && !names.contains(&"Intensity"));
    }

    #[test]
    fn the_drop_downs_come_from_the_fixture() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &wheel_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        let labels = |name: &str| -> Vec<String> { DESCRIPTOR.selection_values(name, None, &p, &t).into_iter().map(|c| c.label).collect() };
        assert_eq!(labels("SpinColorWheelFunctionName"), ["Color Wheel"]);
        assert_eq!(labels("SpinColorWheelIndexValue"), ["Spin"]);
        assert!(DESCRIPTOR.selection_values("SpinColorWheelFunctionName", None, &p, &TargetInfo::detached()).is_empty());
    }

    #[test]
    fn thumbnail_draws_for_a_fixture_with_the_function() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &wheel_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        let mut g = Graphics::new(200, 40);
        (DESCRIPTOR.visual.unwrap())(&p, &t, &mut g, (0, 0, 200, 40));
        assert!(crate::visual::has_ink(&g));
        let mut g = Graphics::new(200, 40);
        (DESCRIPTOR.visual.unwrap())(&DESCRIPTOR.new_params(), &TargetInfo::detached(), &mut g, (0, 0, 200, 40));
        assert!(!crate::visual::has_ink(&g));
    }
}
