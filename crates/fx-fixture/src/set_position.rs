//! Port of VixenModules.Effect.SetPosition (SetPositionModule, SetPositionData).

use crate::fixture_base::{render_curve, FixtureTargets};
use crate::generated::set_position as gen;
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

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let tf = FixtureTargets::new(ctx);
    let mut out = EffectIntents::new();

    // _canPan / _canTilt come from UpdateFixtureCapabilities: some fixture of the first target has the function.
    if p.boolean("EnablePan") {
        let mut tags: HashMap<NodeId, String> = HashMap::new();
        let nodes = tf.render_nodes_for_identity(FuncId::Pan, &mut tags);
        if !nodes.is_empty() {
            if !render_curve(ctx, &mut out, &nodes, &p.curve("Pan"), FuncId::Pan, "", Some(&tags)) {
                return out;
            }
        }
    }
    if p.boolean("EnableTilt") {
        let mut tags: HashMap<NodeId, String> = HashMap::new();
        let nodes = tf.render_nodes_for_identity(FuncId::Tilt, &mut tags);
        if !nodes.is_empty() {
            render_curve(ctx, &mut out, &nodes, &p.curve("Tilt"), FuncId::Tilt, "", Some(&tags));
        }
    }
    out
}

/// `_canPan` and `_canTilt` with `EnablePan` and `EnableTilt`.
fn enabled(p: &Params, t: &TargetInfo) -> (bool, bool) {
    let tf = FixtureTargets::from_info(*t);
    let mut tags: HashMap<NodeId, String> = HashMap::new();
    let can_pan = !tf.render_nodes_for_identity(FuncId::Pan, &mut tags).is_empty();
    let can_tilt = !tf.render_nodes_for_identity(FuncId::Tilt, &mut tags).is_empty();
    (can_pan && p.boolean("EnablePan"), can_tilt && p.boolean("EnableTilt"))
}

/// `UpdateAttributes`.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let (pan, tilt) = enabled(p, t);
    v.set("Pan", pan);
    v.set("Tilt", tilt);
}

/// `GenerateVisualRepresentation`.
fn visual_representation(p: &Params, t: &TargetInfo, g: &mut Graphics, clip: (i32, i32, i32, i32)) {
    let (enable_pan, enable_tilt) = enabled(p, t);
    let show_both = enable_pan && enable_tilt;
    let rect = (clip.0, clip.1, clip.2, if show_both { (clip.3 - 4) / 2 } else { clip.3 - 4 });

    if enable_pan {
        visual::draw_curve(g, rect, Color::GREEN, "Pan", 2, if show_both { -2 } else { 1 }, &p.curve("Pan"), 0);
    }
    if enable_tilt {
        let tilt_color = Color::rgb(0, 128, 255);
        if show_both {
            // The tilt curve goes in the bottom half of the timeline
            visual::draw_curve(g, rect, tilt_color, "Tilt", rect.3 + 3, rect.3, &p.curve("Tilt"), rect.1);
        } else {
            visual::draw_curve(g, rect, tilt_color, "Tilt", 2, 1, &p.curve("Tilt"), rect.1);
        }
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

    fn value_at(out: &EffectIntents, tag: &str, ms: i64) -> f64 {
        for nodes in out.map.values() {
            for n in nodes {
                if let StateValue::Range(v) = n.intent.state_at(TimeSpan::from_ms_i(ms) - n.start) {
                    if &*v.tag == tag {
                        return v.value;
                    }
                }
            }
        }
        panic!("no {tag} intent");
    }

    #[test]
    fn pan_ramps_up_and_tilt_ramps_down_by_default() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |_| {});
        assert_eq!(out.map.values().next().unwrap().len(), 2);
        assert_eq!(value_at(&out, "Pan", 250), 0.25);
        assert_eq!(value_at(&out, "Tilt", 250), 0.75);
    }

    #[test]
    fn disabled_axes_are_not_rendered() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("EnablePan", ParamValue::Bool(false));
        });
        let nodes = out.map.values().next().unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(value_at(&out, "Tilt", 0), 1.0);
    }

    #[test]
    fn fixtures_without_the_function_are_skipped() {
        let mut spec = sample_spec();
        spec.channels.retain(|c| c.function != "Tilt");
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &spec);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |_| {});
        assert_eq!(out.map.values().next().unwrap().len(), 1);
    }

    #[test]
    fn visibility_follows_the_enable_switches_and_the_fixture() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let mut pan_only = sample_spec();
        pan_only.functions.retain(|f| f.name != "Tilt");
        pan_only.channels.retain(|c| c.function != "Tilt");
        let pans = fixtures(&mut tree, "Pans", 1, &pan_only);
        let mut p = DESCRIPTOR.new_params();
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[pans]));
        assert!(names.contains(&"Pan") && !names.contains(&"Tilt"));
        p.set("EnableTilt", ParamValue::Bool(false));
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[heads]));
        assert!(names.contains(&"Pan") && !names.contains(&"Tilt"));
        let names = testing::visible_names(&DESCRIPTOR, &DESCRIPTOR.new_params(), &TargetInfo::detached());
        assert!(!names.contains(&"Pan") && !names.contains(&"Tilt"));
    }

    #[test]
    fn thumbnail_draws_both_curves() {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 1, &sample_spec());
        let targets = [heads];
        let mut g = Graphics::new(200, 60);
        (DESCRIPTOR.visual.unwrap())(&DESCRIPTOR.new_params(), &testing::target_info(&tree, &targets), &mut g, (0, 0, 200, 60));
        assert!(crate::visual::has_ink(&g));
        // The tilt curve sits in the lower half
        let b = g.bitmap();
        assert!((30..60).any(|y| (0..200).any(|x| b.get_pixel(x, y).a > 0)));
    }
}
