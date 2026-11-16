//! Port of VixenModules.Effect.VerticalMeter.

use crate::audio_helper::{self, render_meter};
use crate::generated::vertical_meter as gen;
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

/// `AudioPluginBase.CheckForInvalidColorData` (VerticalMeter does not override it).
fn validate(p: &mut Params, valid_colors: &[Color]) {
    audio_helper::check_for_invalid_color_data(p, valid_colors, false);
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let inverted = p.boolean("Inverted");
    let total_ms = ctx.time_span.total_ms();
    let steps = (total_ms / SPACING as f64) as i32;
    render_meter(ctx, p, |env, node, out| {
        let leaves = ctx.tree.leaves(node);
        let element_count = leaves.len() as i32;
        let range = env.range as f64;
        let mut current_element: i32 = 0;
        for leaf in leaves {
            let Some(element) = ctx.tree.node(leaf).element_id else {
                continue;
            };
            let discrete = ctx.is_element_discrete(leaf);
            let colors = if discrete { env.discrete_colors(leaf) } else { Vec::new() };
            let mut last_time = TimeSpan::ZERO;

            let mut gradient_position = current_element as f64 / element_count as f64;

            // Audio max is at 0db. The threshold gets shifted from 0 to 1 to -1 to 0 and then scaled.
            let threshold;
            if !inverted {
                threshold = ((element_count - current_element) as f64 / element_count as f64 - 1.0) * range;
                gradient_position = 1.0 - gradient_position;
            } else {
                threshold = (current_element as f64 / element_count as f64 - 1.0) * range;
            }

            let mut last_value = env.audio.volume_at_time(0) >= threshold;
            for i in 1..steps {
                // Current time in ms = i * spacing
                let current_value = env.audio.volume_at_time(i * SPACING) >= threshold;
                if current_value != last_value {
                    let start = last_time;
                    if last_value {
                        env.generate_effect_intents(
                            out,
                            element,
                            &colors,
                            gradient_position,
                            gradient_position,
                            TimeSpan::from_ms((i * SPACING) as f64) - last_time,
                            start,
                            discrete,
                        );
                    }
                    last_time = TimeSpan::from_ms((i * SPACING) as f64);
                    last_value = current_value;
                }
            }
            if last_value {
                let start = last_time;
                env.generate_effect_intents(
                    out,
                    element,
                    &colors,
                    gradient_position,
                    gradient_position,
                    ctx.time_span - last_time,
                    start,
                    discrete,
                );
            }
            current_element += 1;
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
        let g = testing::grid(&mut tree, "G", 4, 1);
        assert!(render_audio(&DESCRIPTOR, &tree, &[g], 1000, None, |_| {}).is_empty());
    }

    /// A slow attack and a wide range spread the thresholds far enough apart to see at 30 ms resolution.
    fn slow_attack(p: &mut Params) {
        p.set("AttackTime", ParamValue::Int(300));
        p.set("Range", ParamValue::Int(20));
    }

    fn start_of_first(out: &EffectIntents, id: &Uuid) -> Option<f64> {
        out.nodes_for(id).and_then(|n| n.first()).map(|n| n.start.total_ms())
    }

    #[test]
    fn lower_elements_light_first_and_colours_follow_position() {
        // Silence then a full-scale signal. The bottom element of 4 has the highest threshold (0 dB).
        // Each element has the threshold -k/4 * Range (20 dB here), so lower elements light first as the level rises.
        let audio = FakeAudio::dc_after(8000, 2.0, 500.0);
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 4, 1);
        let ids: Vec<Uuid> = tree.leaves(g).iter().map(|l| tree.node(*l).element_id.unwrap()).collect();
        let out = render_audio(&DESCRIPTOR, &tree, &[g], 2000, Some(&audio), slow_attack);
        let starts: Vec<f64> = ids.iter().map(|i| start_of_first(&out, i).unwrap()).collect();
        // Element 3 (threshold -15 dB) lights before element 0 (threshold 0 dB).
        assert!(starts[3] < starts[2] && starts[2] < starts[1] && starts[1] < starts[0], "{starts:?}");
        // Not inverted: element 0 is the top of the meter, gradient position 1 (red); element 3 is 0.25.
        let colour = |i: usize| match out.nodes_for(&ids[i]).unwrap()[0].intent.state_at(TimeSpan::ZERO) {
            vixen_core::StateValue::Lighting(v) => v.color,
            _ => panic!(),
        };
        assert_eq!(colour(0), Color::RED);
        assert_eq!(colour(3), Color::YELLOW);
        // each lights until the end of the effect: one intent ending at 2000 ms
        let last = out.nodes_for(&ids[0]).unwrap().last().unwrap();
        assert_eq!(last.start + last.intent.span, TimeSpan::from_ms_i(2000));
    }

    #[test]
    fn inverted_flips_the_meter() {
        let audio = FakeAudio::dc_after(8000, 2.0, 500.0);
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 4, 1);
        let ids: Vec<Uuid> = tree.leaves(g).iter().map(|l| tree.node(*l).element_id.unwrap()).collect();
        let out = render_audio(&DESCRIPTOR, &tree, &[g], 2000, Some(&audio), |p| {
            slow_attack(p);
            p.set("Inverted", ParamValue::Bool(true));
        });
        let starts: Vec<f64> = ids.iter().map(|i| start_of_first(&out, i).unwrap()).collect();
        // threshold of element k is (k/4 - 1) * 20: element 0 now has the lowest threshold
        assert!(starts[0] < starts[1] && starts[1] < starts[2] && starts[2] < starts[3], "{starts:?}");
    }

    #[test]
    fn silence_at_start_with_no_signal_stays_dark_without_normalize() {
        let audio = FakeAudio { samples: vec![0.0; 16000], rate: 8000 };
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 4, 1);
        let out = render_audio(&DESCRIPTOR, &tree, &[g], 2000, Some(&audio), |p| {
            p.set("Normalize", ParamValue::Bool(false));
        });
        assert!(out.is_empty());
    }
}
