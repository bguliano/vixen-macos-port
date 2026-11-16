//! Port of VixenModules.Effect.SetLevel.

use crate::generated::set_level as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor =
    EffectDescriptor { render, validate: Some(validate), ported: true, ..gen::BASE };

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let color = p.color("Color");
    let level = p.double("IntensityLevel");
    let has_discrete = ctx.has_discrete_colors();

    // The C# loop assigns `_elementData = RenderNode(node)` for each target, so the last target wins.
    // Sequences hold one target per effect, so that never matters in practice.
    let mut out = EffectIntents::new();
    for &node in ctx.targets {
        if ctx.cancelled() {
            return out;
        }
        let mut intents = EffectIntents::new();
        for leaf in ctx.tree.distinct_leaves(node) {
            if has_discrete && ctx.is_element_discrete(leaf) && !ctx.tree.valid_colors(leaf, false).contains(&color) {
                continue;
            }
            // A childless group has no Element, so `.Element.Id` throws and the effect stays dark.
            let Some(element) = ctx.tree.node(leaf).element_id else { return EffectIntents::new() };
            let intent = ctx.create_intent(has_discrete, leaf, color, level, ctx.time_span);
            intents.add_intent_for_element(element, intent, TimeSpan::ZERO);
        }
        out = intents;
    }
    out
}

/// `CheckForInvalidColorData`.
fn validate(p: &mut Params, valid: &[Color]) {
    if !valid.is_empty() && !valid.contains(&p.color("Color")) {
        p.set("Color", ParamValue::Rgb(vixen_core::color::Rgb::from_color(valid[0])));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::testing;
    use vixen_core::StateValue;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn lights_every_leaf_at_the_level() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 4);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, |p| {
            p.set("IntensityLevel", ParamValue::Double(0.5));
            p.set("Color", ParamValue::Rgb(vixen_core::color::Rgb::from_color(Color::RED)));
        });
        assert_eq!(out.map.len(), 4);
        for nodes in out.map.values() {
            match nodes[0].intent.state_at(TimeSpan::from_ms_i(500)) {
                StateValue::Lighting(v) => {
                    assert_eq!(v.color, Color::RED);
                    assert_eq!(v.intensity, 0.5);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn an_empty_group_in_the_target_leaves_the_effect_dark() {
        let mut tree = ElementTree::new();
        let group = tree.add_node(Uuid::new_v4(), "G", None);
        let a = tree.add_element("A");
        let empty = tree.add_node(Uuid::new_v4(), "Empty", None);
        tree.add_child(group, a);
        tree.add_child(group, empty);
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[group], 1000, |_| {}).is_empty());
    }

    #[test]
    fn discrete_elements_only_take_their_own_colour() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 2);
        testing::make_discrete(&mut tree, s, Color::RED);
        // Validation corrects a white default to the only colour the string can show.
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, |_| {});
        assert_eq!(out.map.len(), 2);
        let n = out.map.values().next().unwrap();
        assert!(matches!(n[0].intent.state_at(TimeSpan::ZERO), StateValue::Discrete(v) if v.color == Color::RED));
    }
}
