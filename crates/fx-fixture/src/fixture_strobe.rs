//! Port of VixenModules.Effect.FixtureStrobe (FixtureStrobeModule, FixtureStrobeData).

use crate::fixture_base::{index_effect_selection, render_index_effect, supports_index_function, update_index_capabilities, FixtureTargets};
use crate::generated::fixture_strobe as gen;
use crate::spec::{FixtureIndexItem, FixtureIndexType, FuncId};
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

/// `GetCompatibleIndexValues`: only index values marked as strobe items.
fn strobe_index(i: &FixtureIndexItem) -> bool {
    i.index_type == FixtureIndexType::Strobe
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let tf = FixtureTargets::new(ctx);
    let mut out = EffectIntents::new();
    match render_index_effect(ctx, &tf, &mut out, p.string("FixtureStrobeFunctionName"), p.string("FixtureStrobeIndexValue"), &p.curve("FixtureStrobeCurve"), FuncId::Shutter) {
        Ok(()) => out,
        Err(()) => EffectIntents::new(),
    }
}

fn selection(name: &str, _row: Option<&Params>, p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    index_effect_selection(name, p, t, ("FixtureStrobeFunctionName", "FixtureStrobeIndexValue"), FuncId::Shutter, strobe_index)
}

/// `UpdateFixtureCapabilities`, against the shutter function.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    update_index_capabilities(p, t, "FixtureStrobeFunctionName", "FixtureStrobeIndexValue", FuncId::Shutter, strobe_index, true);
}

/// `IndexHasCurve`. The C# caches this in the index setter, so a loaded effect holds whatever its default data gave until
/// the index is next changed; worked out from the values here, it is right from the start.
fn index_has_curve(p: &Params, t: &TargetInfo) -> bool {
    FixtureTargets::from_info(*t).index_has_curve(p.string("FixtureStrobeIndexValue"), p.string("FixtureStrobeFunctionName"), FuncId::Shutter)
}

/// `UpdateAttributes`.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    v.set("FixtureStrobeIndexValue", supports_index_function(&FixtureTargets::from_info(*t), FuncId::Shutter));
    v.set("FixtureStrobeCurve", index_has_curve(p, t));
}

/// `GenerateVisualRepresentation`: the speed curve when the strobe index uses one, the label otherwise.
fn visual_representation(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    if index_has_curve(p, t) {
        visual::draw_curve(g, rect, Color::YELLOW, "Strobe", 2, 1, &p.curve("FixtureStrobeCurve"), 0);
    } else {
        visual::draw_text(g, rect, Color::YELLOW, "Strobe", 0, rect.1, rect.3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_base::test_support::*;
    use crate::spec::FixtureSpecification;
    use vixen_core::StateValue;
    use vixen_effects::testing;

    fn strobe_spec() -> FixtureSpecification {
        let mut spec = sample_spec();
        let shutter = spec.functions.iter_mut().find(|f| f.name == "Shutter").unwrap();
        shutter.index_data = vec![
            FixtureIndexItem { name: "Closed".into(), start_value: 0, end_value: 0, index_type: FixtureIndexType::ShutterClosed, ..FixtureIndexItem::index() },
            FixtureIndexItem {
                name: "Strobe".into(),
                start_value: 64,
                end_value: 95,
                use_curve: true,
                index_type: FixtureIndexType::Strobe,
                ..FixtureIndexItem::index()
            },
        ];
        spec
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn only_strobe_indexes_are_offered_so_the_first_one_is_chosen() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &strobe_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |_| {});
        let nodes = out.map.values().next().unwrap();
        assert_eq!(nodes.len(), 20);
        // Speed ramps from 64 towards 95 along the default ramp-up curve; 79.5 rounds to even (80)
        assert!(matches!(nodes[0].intent.state_at(TimeSpan::ZERO), StateValue::Command(c) if c.as_u8() == Some(64)));
        assert!(matches!(nodes[10].intent.state_at(TimeSpan::ZERO), StateValue::Command(c) if c.as_u8() == Some(80)));
    }

    #[test]
    fn a_non_strobe_index_is_replaced() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &strobe_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("FixtureStrobeIndexValue", ParamValue::Str("Closed".into()));
        });
        let nodes = out.map.values().next().unwrap();
        assert!(matches!(nodes[0].intent.state_at(TimeSpan::ZERO), StateValue::Command(c) if c.as_u8() == Some(64)));
    }

    #[test]
    fn the_drop_downs_come_from_the_fixture() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &strobe_spec());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        let labels = |name: &str| -> Vec<String> { DESCRIPTOR.selection_values(name, None, &p, &t).into_iter().map(|c| c.label).collect() };
        assert_eq!(labels("FixtureStrobeFunctionName"), ["Shutter"]);
        assert_eq!(labels("FixtureStrobeIndexValue"), ["Strobe"]);
        assert!(DESCRIPTOR.selection_values("FixtureStrobeFunctionName", None, &p, &TargetInfo::detached()).is_empty());
    }

    #[test]
    fn thumbnail_draws() {
        let mut g = Graphics::new(200, 40);
        (DESCRIPTOR.visual.unwrap())(&DESCRIPTOR.new_params(), &TargetInfo::detached(), &mut g, (0, 0, 200, 40));
        assert!(crate::visual::has_ink(&g));
    }
}
