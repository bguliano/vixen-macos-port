//! Port of VixenModules.Effect.Prism (PrismModule, PrismData).

use crate::fixture_base::{index_effect_selection, render_index_effect, supports_index_function, update_index_capabilities, FixtureTargets};
use crate::generated::prism as gen;
use crate::spec::{FixtureIndexItem, FuncId};
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

fn any_index(_: &FixtureIndexItem) -> bool {
    true
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let tf = FixtureTargets::new(ctx);
    let mut out = EffectIntents::new();
    match render_index_effect(ctx, &tf, &mut out, p.string("PrismFunctionName"), p.string("PrismIndexValue"), &p.curve("PrismCurve"), FuncId::Prism) {
        Ok(()) => out,
        Err(()) => EffectIntents::new(),
    }
}

fn selection(name: &str, _row: Option<&Params>, p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    index_effect_selection(name, p, t, ("PrismFunctionName", "PrismIndexValue"), FuncId::Prism, any_index)
}

/// `UpdateFixtureCapabilities`.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    update_index_capabilities(p, t, "PrismFunctionName", "PrismIndexValue", FuncId::Prism, any_index, true);
}

/// `IndexHasCurve`. The C# caches this in the index setter, so a loaded effect holds whatever its default data gave until
/// the index is next changed; worked out from the values here, it is right from the start.
fn index_has_curve(p: &Params, t: &TargetInfo) -> bool {
    FixtureTargets::from_info(*t).index_has_curve(p.string("PrismIndexValue"), p.string("PrismFunctionName"), FuncId::Prism)
}

/// `UpdateAttributes`.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    v.set("PrismIndexValue", supports_index_function(&FixtureTargets::from_info(*t), FuncId::Prism));
    v.set("PrismCurve", index_has_curve(p, t));
}

/// `GenerateVisualRepresentation`.
fn visual_representation(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    if index_has_curve(p, t) {
        visual::draw_curve(g, rect, Color::YELLOW, "Prism", 2, 1, &p.curve("PrismCurve"), 0);
    } else {
        visual::draw_text(g, rect, Color::YELLOW, "Prism", 0, rect.1, rect.3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_base::test_support::*;
    use crate::spec::{FixtureChannel, FixtureFunction, FixtureFunctionType, FixtureIndexItem, FixtureSpecification};
    use vixen_core::StateValue;
    use vixen_effects::testing;

    fn prism_spec() -> FixtureSpecification {
        let mut spec = sample_spec();
        spec.functions.push(FixtureFunction {
            name: "Prism".into(),
            function_type: FixtureFunctionType::Indexed,
            identity: FuncId::Prism,
            label: "Pr".into(),
            index_data: vec![
                FixtureIndexItem { name: "Open".into(), start_value: 0, end_value: 0, ..FixtureIndexItem::index() },
                FixtureIndexItem { name: "Three".into(), start_value: 30, end_value: 30, ..FixtureIndexItem::index() },
            ],
            ..Default::default()
        });
        spec.channels.push(FixtureChannel { name: "Prism".into(), function: "Prism".into(), channel_number: 9 });
        spec
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn unset_function_and_index_default_to_the_first_ones() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &prism_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 500, |_| {});
        let nodes = out.map.values().next().unwrap();
        assert_eq!(nodes.len(), 10);
        assert!(matches!(nodes[0].intent.state_at(TimeSpan::ZERO), StateValue::Command(c) if c.as_u8() == Some(0)));
    }

    #[test]
    fn selected_index_is_rendered() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &prism_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 500, |p| {
            p.set("PrismFunctionName", ParamValue::Str("Prism".into()));
            p.set("PrismIndexValue", ParamValue::Str("Three".into()));
        });
        assert_eq!(out.map.len(), 2);
        for nodes in out.map.values() {
            assert!(nodes.iter().all(|n| matches!(n.intent.state_at(TimeSpan::ZERO), StateValue::Command(c) if c.as_u8() == Some(30))));
        }
    }

    #[test]
    fn fixtures_without_a_prism_produce_nothing() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &sample_spec());
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[heads], 500, |_| {}).is_empty());
    }

    #[test]
    fn thumbnail_draws() {
        let mut g = Graphics::new(200, 40);
        (DESCRIPTOR.visual.unwrap())(&DESCRIPTOR.new_params(), &TargetInfo::detached(), &mut g, (0, 0, 200, 40));
        assert!(crate::visual::has_ink(&g));
    }

    #[test]
    fn the_drop_downs_come_from_the_fixture() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &prism_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        let names = DESCRIPTOR.selection_values("PrismFunctionName", None, &p, &t);
        assert_eq!(names.len(), 1);
        assert!(!DESCRIPTOR.selection_values("PrismIndexValue", None, &p, &t).is_empty());
        assert!(DESCRIPTOR.selection_values("PrismFunctionName", None, &p, &TargetInfo::detached()).is_empty());
    }

    #[test]
    fn the_fixture_decides_what_the_editor_offers() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &prism_spec());
        let plain = fixtures(&mut tree, "Plain", 1, &sample_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        assert!(testing::visible_names(&DESCRIPTOR, &p, &t).contains(&"PrismIndexValue"));
        let others = [plain];
        assert!(!testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &others)).contains(&"PrismIndexValue"));
    }
}
