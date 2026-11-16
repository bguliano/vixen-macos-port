use super::{render_color_sample, render_summary, RENDER_SAMPLE_LIMIT};
use crate::sequence::{SequenceSession, SessionEnvironment};
use std::sync::{atomic::Ordering, Arc};
use uuid::Uuid;
use vixen_core::{
    Color, EffectIntents, ElementTree, FrameArray, Intent, IntentData, RgbValue, TimeSpan,
};
use vixen_effects::framework::ParamValue;
use vixen_io::ModuleStore;

fn ms(value: i64) -> TimeSpan {
    TimeSpan::from_ms_i(value)
}

#[test]
fn render_probe_samples_active_nodes_at_effect_time_with_local_offsets_and_intensity() {
    let mut intents = EffectIntents::new();
    let low = Uuid::from_u128(1);
    let high = Uuid::from_u128(2);
    // At effect time 50ms, this node is 25ms into its array: green, not red.
    let frames = FrameArray::from_vec(
        vec![
            RgbValue { r: 255, g: 0, b: 0 },
            RgbValue { r: 0, g: 255, b: 0 },
        ],
        ms(25),
    );
    intents.add_intent_for_element(
        high,
        Intent::new(IntentData::ArrayRgb(frames), ms(100)),
        ms(25),
    );
    intents.add_intent_for_element(
        low,
        Intent::static_lighting(Color::WHITE, 0.5, ms(100)),
        ms(0),
    );
    // Past and future nodes must not appear in the sample. Unsupported types are
    // counted separately rather than silently presented as RGB/Lighting output.
    intents.add_intent_for_element(low, Intent::static_lighting(Color::RED, 1.0, ms(20)), ms(0));
    intents.add_intent_for_element(
        high,
        Intent::static_lighting(Color::BLUE, 1.0, ms(100)),
        ms(75),
    );
    intents.add_intent_for_element(
        high,
        Intent::static_discrete(Color::RED, 1.0, ms(100)),
        ms(0),
    );
    let expected = "sampleMs 50 inspected 5 active 3 nonColor 1 truncated false colors [FF00FF00:1,FF7F7F7F:1]";
    assert_eq!(render_color_sample(Some(&intents)), expected);
    let reversed = EffectIntents {
        map: [
            (low, intents.map[&low].clone()),
            (high, intents.map[&high].clone()),
        ]
        .into_iter()
        .collect(),
    };
    assert_eq!(render_color_sample(Some(&reversed)), expected);
}

#[test]
fn render_probe_reports_bounded_node_and_element_sampling() {
    let mut intents = EffectIntents::new();
    for _ in 0..RENDER_SAMPLE_LIMIT + 1 {
        intents.add_intent_for_element(
            Uuid::from_u128(1),
            Intent::static_lighting(Color::RED, 1.0, ms(100)),
            ms(0),
        );
    }
    assert_eq!(render_color_sample(Some(&intents)), format!(
        "sampleMs 50 inspected {RENDER_SAMPLE_LIMIT} active {RENDER_SAMPLE_LIMIT} nonColor 0 truncated true colors [FFFF0000:{RENDER_SAMPLE_LIMIT}]"));
    let mut oversized = EffectIntents::new();
    for id in 0..RENDER_SAMPLE_LIMIT + 1 {
        oversized.add_intent_for_element(
            Uuid::from_u128(id as u128),
            Intent::static_lighting(Color::RED, 1.0, ms(100)),
            ms(0),
        );
    }
    assert_eq!(
        render_color_sample(Some(&oversized)),
        "sampleMs 50 inspected 0 active 0 nonColor 0 truncated true colors []"
    );
}

#[test]
fn render_probe_reads_actual_cached_render_without_applying_pending_edits_or_mutating_generations()
{
    let mut tree = ElementTree::new();
    let group = tree.add_string("Probe", 8);
    let target = tree.node(group).id;
    let env = SessionEnvironment {
        video_cache: None,
        tree: Arc::new(tree),
        store: Arc::new(ModuleStore::new()),
        module_data_root: None,
        frame_ms: 50,
        media_dir: std::env::temp_dir(),
    };
    let mut session = SequenceSession::new_untitled(&env);
    let descriptor = super::descriptor_named("Set Level").unwrap();
    let id = session
        .add_new_effect(descriptor.type_id, target, "Probe", ms(0), ms(1000))
        .unwrap();
    session.wait_for_renders();
    assert!(render_summary(&session, id).ends_with("truncated false colors [FFFFFFFF:8]"));
    let cached = session.effect(id).unwrap().intents.clone().unwrap();
    session.set_param(
        id,
        "Color",
        ParamValue::Rgb(vixen_core::color::Rgb::from_color(Color::RED)),
    );
    session.set_param(id, "IntensityLevel", ParamValue::Double(0.5));
    let entry = session.effect(id).unwrap();
    let generations = (entry.generation, entry.rendered_generation);
    assert_ne!(generations.0, generations.1);
    let schedule = session.schedule();
    let revision = session.counters.revision.load(Ordering::Relaxed);
    let dirty = render_summary(&session, id);
    assert!(dirty.starts_with(&format!(
        "generation {} rendered {}",
        generations.0, generations.1
    )));
    assert!(
        dirty.ends_with("truncated false colors [FFFFFFFF:8]"),
        "new parameters must not masquerade as rendered colors: {dirty}"
    );
    assert_eq!(render_summary(&session, id), dirty);
    let entry = session.effect(id).unwrap();
    assert_eq!((entry.generation, entry.rendered_generation), generations);
    assert!(Arc::ptr_eq(entry.intents.as_ref().unwrap(), &cached));
    assert!(Arc::ptr_eq(&session.schedule(), &schedule));
    assert_eq!(session.counters.revision.load(Ordering::Relaxed), revision);
    session.wait_for_renders();
    assert!(session.effect(id).unwrap().is_rendered());
    assert!(render_summary(&session, id).ends_with("truncated false colors [FF7F0000:8]"));
    assert!(!Arc::ptr_eq(
        session.effect(id).unwrap().intents.as_ref().unwrap(),
        &cached
    ));
}
