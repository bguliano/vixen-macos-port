//! Port of VixenModules.Effect.Frost (FrostModule, FrostData).

use crate::fixture_base::{render_curve, repair_function_name, FixtureTargets};
use crate::generated::frost as gen;
use crate::spec::FuncId;
use crate::visual;
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

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let tf = FixtureTargets::new(ctx);

    // RenderCurve(Frost, FunctionIdentity.Frost, FrostFunctionName)
    let name = p.string("FrostFunctionName");
    let mut out = EffectIntents::new();
    let nodes = tf.render_nodes_for_function_name(name, FuncId::Frost);
    render_curve(ctx, &mut out, &nodes, &p.curve("Frost"), FuncId::Frost, name, None);
    out
}

/// `FrostFunctionNameConverter`.
fn selection(name: &str, _row: Option<&Params>, _p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    (name == "FrostFunctionName").then(|| FixtureTargets::from_info(*t).matching_function_names(FuncId::Frost).into_iter().map(SelectionValue::text).collect())
}

/// The function-name half of `UpdateFixtureCapabilities`. A loaded effect keeps its saved name.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    let tf = FixtureTargets::from_info(*t);
    let mut name = p.string("FrostFunctionName").to_string();
    repair_function_name(&tf, &mut name, FuncId::Frost);
    p.set("FrostFunctionName", ParamValue::Str(name));
}

/// `_canFrost`, which `UpdateFixtureCapabilities` works out from the name it has just repaired. A name no target has
/// falls back to the first one that does, as the default data a loaded effect first receives its targets on does.
fn can_frost(p: &Params, t: &TargetInfo) -> bool {
    let tf = FixtureTargets::from_info(*t);
    let mut name = p.string("FrostFunctionName").to_string();
    repair_function_name(&tf, &mut name, FuncId::Frost);
    !tf.render_nodes_for_function_name(&name, FuncId::Frost).is_empty()
}

/// `UpdateAttributes`.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    v.set("Frost", can_frost(p, t));
}

/// `GenerateVisualRepresentation`.
fn visual_representation(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    if can_frost(p, t) {
        visual::draw_curve(g, rect, visual::LIGHT_SKY_BLUE, "Frost", 2, 1, &p.curve("Frost"), 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_base::test_support::*;
    use vixen_core::StateValue;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        // Plain elements have no fixture property, so no intents: assert_produces_light does not apply.
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn the_drop_down_comes_from_the_fixture() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let p = DESCRIPTOR.new_params();
        let labels: Vec<String> = DESCRIPTOR.selection_values("FrostFunctionName", None, &p, &t).into_iter().map(|c| c.label).collect();
        assert_eq!(labels, ["Frost"]);
        assert!(DESCRIPTOR.selection_values("FrostFunctionName", None, &p, &TargetInfo::detached()).is_empty());
    }

    #[test]
    fn plain_elements_produce_nothing() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, |_| {}).is_empty());
    }

    #[test]
    fn frost_ramps_on_each_fixture_with_function_tag_and_label() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &sample_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |_| {});
        assert_eq!(out.map.len(), 2);
        for nodes in out.map.values() {
            assert_eq!(nodes.len(), 1);
            match nodes[0].intent.state_at(TimeSpan::from_ms_i(500)) {
                StateValue::Range(v) => {
                    assert_eq!(v.value, 0.5);
                    assert_eq!(&*v.tag, "Frost");
                    assert_eq!(&*v.label, "F");
                    assert_eq!(v.tag_type, vixen_core::value::FunctionIdentity::Frost);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn unsupported_function_name_is_replaced_by_the_first_frost_function() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("FrostFunctionName", ParamValue::Str("Nope".into()));
        });
        assert_eq!(out.map.len(), 1);
    }

    #[test]
    fn curve_points_split_the_effect_into_segments() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("Frost", ParamValue::Curve(Curve::from_points(vec![(0.0, 0.0), (25.0, 100.0), (100.0, 100.0)])));
        });
        let nodes = out.map.values().next().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].intent.span, TimeSpan::from_ms_i(250));
        assert_eq!(nodes[1].start, TimeSpan::from_ms_i(250));
    }

    #[test]
    fn a_loaded_name_the_fixture_lacks_renders_nothing() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let out = testing::render_loaded(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("FrostFunctionName", ParamValue::Str("Nope".into()));
        });
        assert!(out.is_empty());
    }

    #[test]
    fn the_targets_decide_the_curve_and_the_thumbnail() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let plain = tree.add_string("S", 2);
        let p = DESCRIPTOR.new_params();
        for (targets, can) in [(vec![heads], true), (vec![plain], false), (vec![], false)] {
            let t = testing::target_info(&tree, &targets);
            assert_eq!(testing::visible_names(&DESCRIPTOR, &p, &t).contains(&"Frost"), can);
            let mut g = Graphics::new(200, 40);
            (DESCRIPTOR.visual.unwrap())(&p, &t, &mut g, (0, 0, 200, 40));
            assert_eq!(crate::visual::has_ink(&g), can);
        }
    }
}
