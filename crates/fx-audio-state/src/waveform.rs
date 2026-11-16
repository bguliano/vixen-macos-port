//! Port of VixenModules.Effect.Waveform.

use crate::audio_helper::{self, render_meter};
use crate::generated::waveform as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    targets_changed: Some(targets_changed),
    param_changed_ctx: Some(param_changed_ctx),
    ported: true,
    ..gen::BASE
};

const SPACING: i32 = 30;

fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    audio_helper::visibility(p, t, v);
}

/// `TargetNodesChanged` calls Waveform's own private `CheckForInvalidColorData`, which builds a transition for several
/// colours, so `validate` stays unset and this replaces it.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    audio_helper::check_for_invalid_color_data(p, &t.valid_colors(), true);
    reset_depth_beyond_first_target(p, t);
}

/// The `ModuleData` setter is `AudioPluginBase`'s, and it calls the base class's private `CheckForInvalidColorData`, which
/// always picks the first colour. So a loaded effect and a newly targeted one differ, as they do in the C#.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if c.name == MODULE_DATA {
        audio_helper::check_for_invalid_color_data(p, &c.info.valid_colors(), false);
    }
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let inverted = p.boolean("Inverted");
    // `((WaveformData)Data).ScrollSpeed`, not the inverted `ScrollSpeed` property.
    let scroll_speed = p.int("ScrollSpeed");
    let total_ms = ctx.time_span.total_ms();
    let steps = (total_ms / SPACING as f64) as i32;
    render_meter(ctx, p, |env, node, out| {
        let leaves = ctx.tree.leaves(node);
        let node_count = if inverted { leaves.len() as i32 } else { 0 };
        let mut current_element: i32 = 0;
        for leaf in leaves {
            let Some(element) = ctx.tree.node(leaf).element_id else {
                continue;
            };
            let discrete = ctx.is_element_discrete(leaf);
            let colors = if discrete { env.discrete_colors(leaf) } else { Vec::new() };
            for i in 1..steps {
                let (start_audio_time, end_audio_time);
                if inverted {
                    start_audio_time = i * SPACING - (node_count - current_element) * scroll_speed + 1;
                    end_audio_time = (i + 1) * SPACING - (node_count - current_element) * scroll_speed;
                } else {
                    start_audio_time = i * SPACING - current_element * scroll_speed + 1;
                    end_audio_time = (i + 1) * SPACING - current_element * scroll_speed;
                }
                let start_time = TimeSpan::from_ms((i * SPACING) as f64);
                if start_audio_time > 0
                    && (start_audio_time as f64) < total_ms
                    && end_audio_time > 0
                    && (end_audio_time as f64) < total_ms
                {
                    let g1 = env.gradient_position(start_audio_time);
                    let g2 = env.gradient_position(end_audio_time);
                    env.generate_effect_intents(
                        out,
                        element,
                        &colors,
                        g1,
                        g2,
                        TimeSpan::from_ms(SPACING as f64),
                        start_time,
                        discrete,
                    );
                }
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
        // No audio is legitimately dark (the C# renders nothing), so light is checked with audio below.
    }

    #[test]
    fn no_audio_renders_nothing() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        assert!(render_audio(&DESCRIPTOR, &tree, &[e], 1000, None, |_| {}).is_empty());
    }

    #[test]
    fn produces_light_with_audio_and_is_robust() {
        let audio = FakeAudio::sine(8000, 4.0, 200.0, 0.8);
        let mut tree = ElementTree::new();
        let matrix = testing::grid(&mut tree, "M", 8, 25);
        let out = render_audio(&DESCRIPTOR, &tree, &[matrix], 2000, Some(&audio), |_| {});
        assert!(!out.is_empty());
        // zero-size and degenerate settings must not panic
        for ms in [0, 1, 29, 31, 100] {
            render_audio(&DESCRIPTOR, &tree, &[matrix], ms, Some(&audio), |_| {});
        }
        render_audio(&DESCRIPTOR, &tree, &[matrix], 1000, Some(&audio), |p| {
            p.set("Range", ParamValue::Int(0));
            p.set("AttackTime", ParamValue::Int(0));
            p.set("LowPass", ParamValue::Bool(true));
            p.set("HighPass", ParamValue::Bool(true));
        });
    }

    #[test]
    fn intent_count_and_timing() {
        // i runs 1..(int)(1000/30)=33 exclusive: 32 intents of 30 ms starting at 30 ms.
        let audio = FakeAudio::dc_after(8000, 1.0, 0.0);
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let eid = tree.node(e).element_id.unwrap();
        let out = render_audio(&DESCRIPTOR, &tree, &[e], 1000, Some(&audio), |_| {});
        let nodes = out.nodes_for(&eid).unwrap();
        assert_eq!(nodes.len(), 32);
        assert_eq!(nodes[0].start, TimeSpan::from_ms_i(30));
        assert_eq!(nodes[31].start, TimeSpan::from_ms_i(960));
        // 0 dB is the top of the meter: red.
        match nodes[0].intent.state_at(TimeSpan::ZERO) {
            vixen_core::StateValue::Lighting(v) => {
                assert_eq!(v.color, Color::RED);
                assert!((v.intensity - 1.0).abs() < 1e-9);
            }
            _ => panic!("expected a lighting intent"),
        }
    }

    #[test]
    fn scroll_speed_delays_later_elements() {
        // Element k reads the audio k*ScrollSpeed ms earlier, so a step in the audio reaches it later.
        let audio = FakeAudio::dc_after(8000, 2.0, 600.0);
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 3, 1);
        let ids: Vec<Uuid> = tree.leaves(g).iter().map(|l| tree.node(*l).element_id.unwrap()).collect();
        let out = render_audio(&DESCRIPTOR, &tree, &[g], 2000, Some(&audio), |p| {
            p.set("ScrollSpeed", ParamValue::Int(100));
            p.set("Normalize", ParamValue::Bool(false));
        });
        let first_lit = |id: &Uuid| {
            out.nodes_for(id)
                .unwrap()
                .iter()
                .find(|n| matches!(n.intent.state_at(TimeSpan::ZERO), vixen_core::StateValue::Lighting(v) if v.color == Color::RED))
                .map(|n| n.start.total_ms())
                .unwrap()
        };
        let (a, b, c) = (first_lit(&ids[0]), first_lit(&ids[1]), first_lit(&ids[2]));
        assert!(a < b && b < c, "{a} {b} {c}");
        assert!((b - a - 90.0).abs() <= 30.0 && (c - b - 90.0).abs() <= 30.0, "{a} {b} {c}");
    }

    #[test]
    fn discrete_targets_get_discrete_intents() {
        let audio = FakeAudio::dc_after(8000, 1.0, 0.0);
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 2, 2);
        testing::make_discrete(&mut tree, g, Color::RED);
        let out = render_audio(&DESCRIPTOR, &tree, &[g], 1000, Some(&audio), |_| {});
        assert!(!out.is_empty());
        let any = out.map.values().flatten().next().unwrap();
        assert!(matches!(any.intent.state_at(TimeSpan::ZERO), vixen_core::StateValue::Discrete(_)));
    }

    #[test]
    fn defaults_and_visibility() {
        let p = DESCRIPTOR.new_params();
        assert_eq!(p.gradient("MeterColorGradient").get_color_at(0.25), Color::YELLOW);
        let vis = DESCRIPTOR.visible_params(&p);
        let names: Vec<_> = vis.iter().map(|i| DESCRIPTOR.params[*i].name).collect();
        assert!(names.contains(&"RedColorPosition") && !names.contains(&"MeterColorGradient"));
        assert!(!names.contains(&"LowPassFreq"));
    }
}
