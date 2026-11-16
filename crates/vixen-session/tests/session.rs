//! The session's edit, render, undo and save cycle, on a small tree and on a copy of a real sequence.

use std::sync::Arc;
use uuid::Uuid;
use vixen_core::{Color, ElementTree, TimeSpan};
use vixen_effects::framework::ParamValue;
use vixen_engine::schedule::EvalScratch;
use vixen_io::ModuleStore;
use vixen_session::sequence::{EffectId, SequenceSession, SessionEnvironment};
use vixen_session::undo::{AddRemoveKind, MoveKind, UndoAction};

const SET_LEVEL: Uuid = uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0");

struct Fixture {
    session: SequenceSession,
    node: Uuid,
    element: Uuid,
}

fn fixture() -> Fixture {
    let mut tree = ElementTree::new();
    let n = tree.add_element("Element");
    let node = tree.node(n).id;
    let element = tree.node(n).element_id.unwrap();
    let env = SessionEnvironment { video_cache: None,
        tree: Arc::new(tree),
        store: Arc::new(ModuleStore::new()),
        module_data_root: None,
        frame_ms: 25,
        media_dir: std::env::temp_dir(),
    };
    Fixture { session: SequenceSession::new_untitled(&env), node, element }
}

fn add_set_level(f: &mut Fixture, start_ms: i64, span_ms: i64) -> EffectId {
    f.session
        .add_new_effect(SET_LEVEL, f.node, "Element", TimeSpan::from_ms_i(start_ms), TimeSpan::from_ms_i(span_ms))
        .expect("Set Level exists")
}

/// Whether anything lights the element at a time, through the same schedule playback uses.
fn lit_at(f: &Fixture, ms: i64) -> bool {
    let mut scratch = EvalScratch::default();
    let mut states = Vec::new();
    f.session.schedule().evaluate(TimeSpan::from_ms_i(ms), &mut scratch, &mut states);
    let index = f.session.elements().index_of(&f.element).unwrap() as usize;
    let s = &states[index];
    s.lighting.is_some() || s.rgb.is_some() || !s.discrete.is_empty()
}

#[test]
fn a_new_sequence_starts_modified_with_a_minute_of_time() {
    let f = fixture();
    assert!(f.session.is_modified());
    assert_eq!(f.session.length(), TimeSpan::from_ms_i(60_000));
    assert_eq!(f.session.name(), "Untitled");
}

#[test]
fn an_added_effect_renders_in_the_background_and_reaches_the_schedule() {
    let mut f = fixture();
    let id = add_set_level(&mut f, 1000, 500);
    f.session.wait_for_renders();
    assert!(f.session.effect(id).unwrap().is_rendered());
    assert!(lit_at(&f, 1250));
    assert!(!lit_at(&f, 500));
    assert!(!lit_at(&f, 1600));
}

#[test]
fn moving_updates_the_schedule_straight_away_and_undo_puts_it_back() {
    let mut f = fixture();
    let id = add_set_level(&mut f, 1000, 500);
    f.session.wait_for_renders();
    let before = (id, TimeSpan::from_ms_i(1000), TimeSpan::from_ms_i(500));
    f.session.set_times(&[(id, TimeSpan::from_ms_i(5000), TimeSpan::from_ms_i(500))]);
    assert!(lit_at(&f, 5250), "the old render plays at the new place until the new one lands");
    assert!(!lit_at(&f, 1250));
    f.session.record_undo(UndoAction::TimesChanged { kind: MoveKind::Moved, times: vec![before] });
    f.session.wait_for_renders();
    assert_eq!(f.session.undo_stack().undo_descriptions(), ["Moved 1 effect"]);

    assert_eq!(f.session.undo(1), 1);
    f.session.wait_for_renders();
    assert_eq!(f.session.effect(id).unwrap().node.start, TimeSpan::from_ms_i(1000));
    assert!(lit_at(&f, 1250));
    assert!(!lit_at(&f, 5250));

    assert_eq!(f.session.redo(1), 1);
    f.session.wait_for_renders();
    assert_eq!(f.session.effect(id).unwrap().node.start, TimeSpan::from_ms_i(5000));
    assert!(lit_at(&f, 5250));
}

#[test]
fn delete_and_undo_restore_the_same_effect_on_its_layer() {
    let mut f = fixture();
    let a = add_set_level(&mut f, 0, 100);
    let b = add_set_level(&mut f, 200, 100);
    f.session.wait_for_renders();
    let top = vixen_core::layer::Layer::standard("Top", vixen_core::layer::MixingFilter::HighestValue);
    let layer = top.id;
    f.session.document_mut().layers.add_layer(top);
    f.session.layers_changed();
    f.session.set_layer(&[a], layer);

    let snapshots = f.session.remove_effects(&[a]);
    f.session.record_undo(UndoAction::EffectsRemoved { kind: AddRemoveKind::AddedOrRemoved, effects: snapshots });
    assert!(f.session.effect(a).is_none());
    assert!(!lit_at(&f, 50));
    assert!(lit_at(&f, 250));

    f.session.undo(1);
    f.session.wait_for_renders();
    assert!(f.session.effect(a).is_some());
    assert!(lit_at(&f, 50));
    assert_eq!(f.session.layer_of(a), Some(layer));
    let order: Vec<EffectId> = f.session.effects().map(|e| e.id).collect();
    assert_eq!(order, [a, b]);
    // The redo list keeps the action's own description, as the original's does.
    assert_eq!(f.session.undo_stack().redo_descriptions(), ["Removed 1 effect"]);
}

#[test]
fn a_stale_render_never_overwrites_a_newer_edit() {
    let mut f = fixture();
    let id = add_set_level(&mut f, 0, 100);
    f.session.set_param(id, "IntensityLevel", ParamValue::Double(0.5));
    f.session.render_all_dirty();
    f.session.set_param(id, "IntensityLevel", ParamValue::Double(0.25));
    f.session.render_all_dirty();
    f.session.wait_for_renders();
    let entry = f.session.effect(id).unwrap();
    assert!(entry.is_rendered());
    let intents = entry.intents.as_ref().unwrap();
    let node = &intents.map[&f.element][0];
    let state = node.intent.state_at(TimeSpan::ZERO);
    match state {
        vixen_core::StateValue::Lighting(v) => {
            assert_eq!(v.intensity, 0.25);
            assert_eq!(v.color, Color::WHITE);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn marks_reach_effects_that_read_them_and_their_list_rows() {
    // Wave only has a mark selector inside its rows, and Set Level has none.
    const WAVE: &str = "Wave";
    let mut f = fixture();
    let wave = vixen_effect_registry::all().iter().find(|d| d.name == WAVE).expect("Wave exists").type_id;
    let wave_id = f.session.add_new_effect(wave, f.node, "Element", TimeSpan::ZERO, TimeSpan::from_ms_i(100)).unwrap();
    let level_id = add_set_level(&mut f, 0, 100);
    f.session.wait_for_renders();
    assert!(f.session.effect(wave_id).unwrap().is_rendered());
    assert!(f.session.effect(level_id).unwrap().is_rendered());

    f.session.set_marks(vec![vixen_core::MarkCollection::default()]);
    let wave_generation = f.session.effect(wave_id).unwrap().generation;
    assert_eq!(wave_generation, 2, "Wave reads marks, so it renders again");
    assert_eq!(f.session.effect(level_id).unwrap().generation, 1, "Set Level doesn't");
    f.session.wait_for_renders();
}

#[test]
fn editing_a_library_curve_renders_the_effects_linked_to_it_again() {
    use vixen_core::Curve;
    use vixen_io::CurveLibrary;
    use vixen_session::library_links::Libraries;

    let library_with = |y: f64| Libraries {
        curves: CurveLibrary { curves: vec![("Level".to_string(), Curve::from_points(vec![(0.0, y), (100.0, y)]))] },
        ..Libraries::default()
    };
    let level_points = |f: &Fixture, id: EffectId| f.session.effect(id).unwrap().node.params.curve("LevelCurve").points.clone();

    let mut f = fixture();
    let pulse = vixen_effect_registry::all().iter().find(|d| d.name == "Pulse").expect("Pulse exists").type_id;
    let linked = f.session.add_new_effect(pulse, f.node, "Element", TimeSpan::ZERO, TimeSpan::from_ms_i(200)).unwrap();
    let plain = f.session.add_new_effect(pulse, f.node, "Element", TimeSpan::from_ms_i(300), TimeSpan::from_ms_i(200)).unwrap();
    f.session.libraries_changed(library_with(100.0));

    let mut curve = Curve::from_points(vec![(0.0, 50.0), (100.0, 50.0)]);
    curve.library_reference_name = "Level".to_string();
    f.session.set_param(linked, "LevelCurve", ParamValue::Curve(curve));
    f.session.render_all_dirty();
    assert_eq!(level_points(&f, linked), vec![(0.0, 100.0), (100.0, 100.0)], "a render reads the library, not the copy");
    f.session.wait_for_renders();

    let (linked_before, plain_before) = (f.session.effect(linked).unwrap().generation, f.session.effect(plain).unwrap().generation);
    f.session.libraries_changed(library_with(25.0));
    assert_eq!(level_points(&f, linked), vec![(0.0, 25.0), (100.0, 25.0)]);
    assert_eq!(f.session.effect(linked).unwrap().generation, linked_before + 1, "the linked effect renders again");
    assert_eq!(f.session.effect(plain).unwrap().generation, plain_before, "the other one doesn't");
    f.session.wait_for_renders();
    assert!(f.session.effect(linked).unwrap().is_rendered());

    // An entry that has gone drops the link and keeps the points, so nothing needs to render.
    let linked_before = f.session.effect(linked).unwrap().generation;
    f.session.libraries_changed(Libraries::default());
    assert_eq!(f.session.effect(linked).unwrap().generation, linked_before);
    assert_eq!(level_points(&f, linked), vec![(0.0, 25.0), (100.0, 25.0)]);
    assert!(!f.session.effect(linked).unwrap().node.params.curve("LevelCurve").is_library_reference());
}
