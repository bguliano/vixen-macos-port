//! Port of VixenModules.Effect.SetZoom (SetZoomModule, SetZoomData).

use crate::fixture_base::{render_curve, FixtureTargets};
use crate::generated::set_zoom as gen;
use crate::spec::FuncId;
use crate::visual;
use std::collections::HashMap;
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    force_visual: true,
    visual: Some(visual_representation),
    ported: true,
    ..gen::BASE
};

/// `_canZoom`.
fn can_zoom(t: &TargetInfo) -> bool {
    let mut tags: HashMap<NodeId, String> = HashMap::new();
    !FixtureTargets::from_info(*t).render_nodes_for_identity(FuncId::Zoom, &mut tags).is_empty()
}

/// `UpdateAttributes`.
fn visibility(_p: &Params, t: &TargetInfo, v: &mut Visibility) {
    v.set("Zoom", can_zoom(t));
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let tf = FixtureTargets::new(ctx);
    let mut out = EffectIntents::new();
    // RenderCurve(Zoom, FunctionIdentity.Zoom, _zoomTags): the first target's fixtures with a zoom function.
    let mut tags: HashMap<NodeId, String> = HashMap::new();
    let nodes = tf.render_nodes_for_identity(FuncId::Zoom, &mut tags);
    render_curve(ctx, &mut out, &nodes, &p.curve("Zoom"), FuncId::Zoom, "", Some(&tags));
    out
}

/// `GenerateVisualRepresentation`.
fn visual_representation(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    if can_zoom(t) {
        visual::draw_curve(g, rect, Color::RED, "Zoom", 2, 1, &p.curve("Zoom"), 0);
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
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn plain_elements_produce_nothing() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, |_| {}).is_empty());
    }

    #[test]
    fn zoom_curve_is_tagged_with_the_function_name() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 3, &sample_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 2000, |p| {
            p.set("Zoom", ParamValue::Curve(Curve::from_points(vec![(0.0, 100.0), (100.0, 0.0)])));
        });
        assert_eq!(out.map.len(), 3);
        for nodes in out.map.values() {
            match nodes[0].intent.state_at(TimeSpan::from_ms_i(500)) {
                StateValue::Range(v) => {
                    assert_eq!(v.value, 0.75);
                    assert_eq!(&*v.tag, "Zoom");
                    assert_eq!(v.tag_type, vixen_core::value::FunctionIdentity::Zoom);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn only_the_first_target_is_searched() {
        // GetRenderNodesForFunctionIdentity(type, tags) looks at TargetNodes.FirstOrDefault() only.
        let mut tree = ElementTree::new();
        let plain = tree.add_string("Plain", 1);
        let heads = fixtures(&mut tree, "Heads", 2, &sample_spec());
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[plain, heads], 1000, |_| {}).is_empty());
        assert_eq!(testing::render_with(&DESCRIPTOR, &tree, &[heads, plain], 1000, |_| {}).map.len(), 2);
    }

    #[test]
    fn the_targets_decide_the_curve_and_the_thumbnail() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let plain = tree.add_string("S", 2);
        let p = DESCRIPTOR.new_params();
        for (targets, can) in [(vec![heads], true), (vec![plain], false), (vec![], false)] {
            let t = testing::target_info(&tree, &targets);
            assert_eq!(testing::visible_names(&DESCRIPTOR, &p, &t).contains(&"Zoom"), can);
            let mut g = Graphics::new(200, 40);
            (DESCRIPTOR.visual.unwrap())(&p, &t, &mut g, (0, 0, 200, 40));
            assert_eq!(crate::visual::has_ink(&g), can);
        }
    }
}
