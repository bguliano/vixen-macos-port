//! Port of VixenModules.Effect.Gobo (GoboModule, GoboData).

use crate::fixture_base::{index_effect_selection, render_index_effect, supports_index_function, update_index_capabilities, FixtureTargets};
use crate::generated::gobo as gen;
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
    match render_index_effect(ctx, &tf, &mut out, p.string("GoboFunctionName"), p.string("GoboIndexValue"), &p.curve("GoboCurve"), FuncId::Gobo) {
        Ok(()) => out,
        Err(()) => EffectIntents::new(),
    }
}

fn selection(name: &str, _row: Option<&Params>, p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    index_effect_selection(name, p, t, ("GoboFunctionName", "GoboIndexValue"), FuncId::Gobo, any_index)
}

/// `UpdateFixtureCapabilities` repairs the function name. The C# then assigns `GoboIndexValue = GoboIndexValue` instead of
/// the repaired local, so the stored index value is never changed (deliberate quirk; the other index effects assign the
/// repaired value).
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    update_index_capabilities(p, t, "GoboFunctionName", "GoboIndexValue", FuncId::Gobo, any_index, false);
}

/// `UpdateAttributes`. `IndexHasCurve` is cached in the C# index setter, so a loaded effect holds whatever its default data
/// gave until the index is next changed; worked out from the values here, it is right from the start.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let tf = FixtureTargets::from_info(*t);
    v.set("GoboIndexValue", supports_index_function(&tf, FuncId::Gobo));
    v.set("GoboCurve", tf.index_has_curve(p.string("GoboIndexValue"), p.string("GoboFunctionName"), FuncId::Gobo));
}

/// `GenerateVisualRepresentation`. The C# draws the selected gobo's image (from the fixture library's gobo
/// image folder) repeated across the effect when there is one; the library's folder is outside `TargetInfo`, so only the
/// "Gobo" label fallback is drawn.
fn visual_representation(_p: &Params, _t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    visual::draw_text(g, rect, Color::RED, "Gobo", 0, rect.1, rect.3);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_base::test_support::*;
    use crate::spec::FixtureIndexItem;
    use vixen_core::StateValue;
    use vixen_effects::testing;

    fn spec_with_gobos() -> crate::spec::FixtureSpecification {
        let mut spec = sample_spec();
        let wheel = spec.functions.iter_mut().find(|f| f.name == "Gobo Wheel").unwrap();
        wheel.index_data.push(FixtureIndexItem { name: "Open".into(), start_value: 10, end_value: 10, ..FixtureIndexItem::index() });
        wheel.index_data.push(FixtureIndexItem {
            name: "Rotate".into(),
            start_value: 100,
            end_value: 200,
            use_curve: true,
            ..FixtureIndexItem::index()
        });
        spec
    }

    fn commands(out: &EffectIntents) -> Vec<Vec<u8>> {
        out.map
            .values()
            .map(|nodes| {
                nodes
                    .iter()
                    .map(|n| match n.intent.state_at(TimeSpan::ZERO) {
                        StateValue::Command(c) => c.as_u8().unwrap(),
                        other => panic!("{other:?}"),
                    })
                    .collect()
            })
            .collect()
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
    fn fixed_index_holds_its_start_value_every_frame() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &spec_with_gobos());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("GoboIndexValue", ParamValue::Str("Open".into()));
        });
        let cmds = commands(&out);
        assert_eq!(cmds.len(), 2);
        // 1000 ms at the 50 ms frame time
        for c in cmds {
            assert_eq!(c, vec![10u8; 20]);
        }
    }

    #[test]
    fn curve_index_scales_between_start_and_end() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &spec_with_gobos());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("GoboIndexValue", ParamValue::Str("Rotate".into()));
        });
        let nodes = out.map.values().next().unwrap();
        let value = |i: usize| match nodes[i].intent.state_at(TimeSpan::ZERO) {
            StateValue::Command(c) => c.as_u8().unwrap(),
            other => panic!("{other:?}"),
        };
        // Ramp-up curve: frame 0 at 100, frame 10 (position 0.5) at 150, frame 19 (0.95) at 195
        assert_eq!(value(0), 100);
        assert_eq!(value(10), 150);
        assert_eq!(value(19), 195);
        assert_eq!(nodes[1].start, TimeSpan::from_ms_i(50));
        assert_eq!(nodes[1].intent.span, TimeSpan::from_ms_i(50));
    }

    #[test]
    fn index_value_is_not_defaulted_for_gobo() {
        // The C# never stores the repaired index value, so an unset index renders nothing.
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &spec_with_gobos());
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |_| {}).is_empty());
    }

    #[test]
    fn the_drop_downs_come_from_the_fixture() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &spec_with_gobos());
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        let labels = |name: &str| -> Vec<String> { DESCRIPTOR.selection_values(name, None, &p, &t).into_iter().map(|c| c.label).collect() };
        assert_eq!(labels("GoboFunctionName"), ["Gobo Wheel"]);
        assert_eq!(labels("GoboIndexValue"), ["Open", "Rotate"]);
        assert!(DESCRIPTOR.selection_values("GoboFunctionName", None, &p, &TargetInfo::detached()).is_empty());
    }

    #[test]
    fn thumbnail_draws() {
        let mut g = Graphics::new(200, 40);
        (DESCRIPTOR.visual.unwrap())(&DESCRIPTOR.new_params(), &TargetInfo::detached(), &mut g, (0, 0, 200, 40));
        assert!(crate::visual::has_ink(&g));
    }
}
