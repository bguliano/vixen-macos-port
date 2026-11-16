//! Port of VixenModules.Effect.VUMeter.

use crate::audio_helper::{self, render_meter};
use crate::generated::vu_meter as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    validate: Some(validate),
    targets_changed: Some(reset_depth_beyond_first_target),
    ported: true,
    ..gen::BASE
};

const SPACING: i32 = 30;

fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    audio_helper::visibility(p, t, v);
}

fn validate(p: &mut Params, valid_colors: &[Color]) {
    audio_helper::check_for_invalid_color_data(p, valid_colors, false);
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let steps = (ctx.time_span.total_ms() / SPACING as f64 - 1.0) as i32;
    render_meter(ctx, p, |env, node, out| {
        for leaf in ctx.tree.leaves(node) {
            let Some(element) = ctx.tree.node(leaf).element_id else {
                continue;
            };
            let discrete = ctx.is_element_discrete(leaf);
            let colors = if discrete { env.discrete_colors(leaf) } else { Vec::new() };
            for i in 0..steps {
                let g1 = env.gradient_position(i * SPACING);
                let g2 = env.gradient_position((i + 1) * SPACING);
                let start_time = TimeSpan::from_ms((i * SPACING) as f64);
                env.generate_effect_intents(out, element, &colors, g1, g2, TimeSpan::from_ms(SPACING as f64), start_time, discrete);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_helper::test_support::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn no_audio_renders_nothing() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        assert!(render_audio(&DESCRIPTOR, &tree, &[e], 1000, None, |_| {}).is_empty());
    }

    #[test]
    fn white_ramp_follows_the_volume() {
        // Default: Custom style with a white gradient and a 0..100 brightness ramp, so the intensity of a
        // slice is its gradient position. Full-scale DC is 0 dB = position 1 = full brightness.
        let audio = FakeAudio::dc_after(8000, 1.0, 0.0);
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let eid = tree.node(e).element_id.unwrap();
        let out = render_audio(&DESCRIPTOR, &tree, &[e], 1000, Some(&audio), |_| {});
        let nodes = out.nodes_for(&eid).unwrap();
        // (int)(1000/30 - 1) = 32 slices starting at 0
        assert_eq!(nodes.len(), 32);
        assert_eq!(nodes[0].start, TimeSpan::ZERO);
        match nodes[5].intent.state_at(TimeSpan::ZERO) {
            vixen_core::StateValue::Lighting(v) => {
                assert_eq!(v.color, Color::WHITE);
                assert!((v.intensity - 1.0).abs() < 1e-9);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn quiet_audio_is_dimmer_without_normalize() {
        let loud = FakeAudio::sine(8000, 1.0, 100.0, 1.0);
        let quiet = FakeAudio::sine(8000, 1.0, 100.0, 0.05);
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let eid = tree.node(e).element_id.unwrap();
        let sum = |a: &FakeAudio| -> f64 {
            let out = render_audio(&DESCRIPTOR, &tree, &[e], 1000, Some(a), |p| {
                p.set("Normalize", ParamValue::Bool(false));
            });
            out.nodes_for(&eid)
                .map(|n| {
                    n.iter()
                        .map(|x| match x.intent.state_at(TimeSpan::ZERO) {
                            vixen_core::StateValue::Lighting(v) => v.intensity,
                            _ => 0.0,
                        })
                        .sum()
                })
                .unwrap_or(0.0)
        };
        assert!(sum(&loud) > sum(&quiet));
    }

    #[test]
    fn depth_is_hidden() {
        let p = DESCRIPTOR.new_params();
        let names: Vec<_> = DESCRIPTOR.visible_params(&p).iter().map(|i| DESCRIPTOR.params[*i].name).collect();
        assert!(!names.contains(&"DepthOfEffect"));
        assert!(names.contains(&"MeterColorGradient"));
    }
}
