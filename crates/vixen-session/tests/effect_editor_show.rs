//! The Effect Editor against the owner's real show: rows for every effect in a sequence, and that a commit never waits for a render.

use std::path::PathBuf;
use std::time::Instant;
use vixen_core::Curve;
use vixen_session::app::AppCore;
use vixen_session::effect_editor::panel::{self, CommitOptions, CommitOutcome, PanelEdits, RowControl, RowKey, ShownValue};
use vixen_session::sequence::{EffectId, SequenceSession};

fn show() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/show-copy");
    root.join("Sequence").is_dir().then_some(root)
}

fn open_sequence(name: &str) -> Option<SequenceSession> {
    let root = show()?;
    let core = AppCore::open(root.clone(), true).ok()?;
    let env = core.session_environment();
    SequenceSession::open(&root.join("Sequence").join(name), &env).ok()
}

#[test]
fn every_effect_in_a_real_sequence_has_rows_quickly() {
    let Some(session) = open_sequence("Carol of the Bells.tim") else { return };
    let ids: Vec<EffectId> = session.effects().map(|e| e.id).collect();
    assert!(!ids.is_empty());
    let started = Instant::now();
    for id in &ids {
        let m = panel::panel_model(&session, &[*id], panel::DEFAULT_MULTI_EDIT_LIMIT);
        assert!(!m.effect_name.is_empty());
    }
    let per_effect = started.elapsed().as_secs_f64() / ids.len() as f64;
    // Far under a frame each, so selecting through a sequence never waits on this.
    assert!(per_effect < 0.005, "{per_effect} s per effect");
}

#[test]
fn a_big_selection_and_a_commit_dont_wait_for_rendering() {
    let Some(mut session) = open_sequence("Carol of the Bells.tim") else { return };
    let first_type = session.effects().find(|e| e.descriptor.is_some_and(|d| d.name == "Set Level")).map(|e| e.node.type_id);
    let Some(wanted) = first_type else { return };
    let ids: Vec<EffectId> = session.effects().filter(|e| e.node.type_id == wanted).take(75).map(|e| e.id).collect();

    let started = Instant::now();
    let m = panel::panel_model(&session, &ids, panel::DEFAULT_MULTI_EDIT_LIMIT);
    assert!(started.elapsed().as_millis() < 100, "{:?}", started.elapsed());
    assert_eq!(m.effect_count as usize, ids.len());

    let key = m
        .categories
        .iter()
        .flat_map(|c| &c.rows)
        .find(|r| matches!(r.control, RowControl::Slider { .. } | RowControl::Curve { .. }))
        .map(|r| r.key.clone())
        .unwrap_or_else(|| RowKey::top("IntensityLevel"));
    let value = match m.categories.iter().flat_map(|c| &c.rows).find(|r| r.key == key).map(|r| &r.control) {
        Some(RowControl::Curve { .. }) => ShownValue::Curve(Curve::flat(33.0)),
        _ => ShownValue::Slider(37.0),
    };
    let mut edits = PanelEdits::default();
    let started = Instant::now();
    let outcome = panel::commit(&mut session, &mut edits, &ids, &key, value, CommitOptions::default());
    let commit_time = started.elapsed();
    assert!(matches!(outcome, CommitOutcome::Committed { .. }));
    assert!(commit_time.as_millis() < 100, "{commit_time:?}");
    // The renders went to their thread: they finish later, and the undo step restores all of it.
    session.wait_for_renders();
    assert!(session.undo_stack().can_undo());
    session.undo(1);
    session.wait_for_renders();
}
