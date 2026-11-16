//! Deterministic render ownership across Delete/Undo. The real render queue is
//! replaced before adding effects, so tests explicitly choose result arrival order.
use super::*;
use crate::undo::AddRemoveKind;
use std::time::Duration;
use vixen_core::StateValue;

fn fixture() -> (SequenceSession, EffectId, mpsc::Receiver<RenderBatch>) {
    let mut tree = ElementTree::new();
    let node = tree.add_element("Light");
    let target = tree.node(node).id;
    let env = SessionEnvironment {
        video_cache: None, tree: Arc::new(tree), store: Arc::new(ModuleStore::new()),
        module_data_root: None, frame_ms: 50, media_dir: std::env::temp_dir(),
    };
    let mut session = SequenceSession::new_untitled(&env);
    let (send, receive) = mpsc::channel();
    session.render_queue = Some(send);
    let id = session.add_new_effect(uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0"),
        target, "Light", TimeSpan::ZERO, TimeSpan::from_ms_i(1000)).unwrap();
    (session, id, receive)
}

fn rendered(mut batch: RenderBatch) -> RenderResult {
    assert_eq!(batch.tasks.len(), 1);
    let task = batch.tasks.pop().unwrap();
    let shared = batch.shared;
    let env = RenderEnv {
        video_cache: None, tree: &shared.tree, frame_ms: shared.frame_ms,
        marks: &shared.marks, audio: None, module_data_root: None,
    };
    let job = EffectJob {
        id: task.instance_id, descriptor: task.descriptor, params: &task.params,
        targets: &task.targets, start: task.start, span: task.span,
    };
    let intents = render_effect(&env, &job, &AtomicBool::new(false)).unwrap();
    RenderResult { id: task.id, generation: task.generation, intents: Arc::new(intents), video_metadata: None }
}
fn receive(queue: &mpsc::Receiver<RenderBatch>) -> RenderBatch {
    queue.recv_timeout(Duration::from_secs(1)).expect("the edit queued a render")
}
fn apply(session: &mut SequenceSession, result: RenderResult) -> usize {
    session.results.lock().push(result);
    session.apply_finished_renders()
}
fn delete_and_undo(session: &mut SequenceSession, id: EffectId) {
    let effects = session.remove_effects(&[id]);
    session.record_undo(UndoAction::EffectsRemoved { kind: AddRemoveKind::AddedOrRemoved, effects });
    assert_eq!(session.undo(1), 1);
}
fn level(session: &SequenceSession, id: EffectId) -> f64 {
    let intents = session.effect(id).unwrap().intents.as_ref().unwrap();
    let node = &intents.map.values().next().unwrap()[0];
    let StateValue::Lighting(value) = node.intent.state_at(TimeSpan::ZERO) else { panic!("Set Level lighting intent") };
    value.intensity
}

#[test]
fn undo_deleted_dirty_effect_keeps_it_dirty_and_renders_current_parameters() {
    let (mut session, id, queue) = fixture();
    assert_eq!(apply(&mut session, rendered(receive(&queue))), 1);
    let old_intents = session.effect(id).unwrap().intents.clone().unwrap();
    assert_eq!(level(&session, id), 1.0);
    session.set_param(id, "IntensityLevel", ParamValue::Double(0.25));
    session.render_all_dirty();
    let late_before_delete = rendered(receive(&queue));
    let removed_generation = session.effect(id).unwrap().generation;
    delete_and_undo(&mut session, id);
    let restored = session.effect(id).unwrap();
    assert!(!restored.is_rendered(), "cached A must not mark dirty B complete");
    assert_eq!(session.unrendered_count(), 1);
    assert!(restored.generation > removed_generation, "restoration retires every pre-delete job");
    assert!(Arc::ptr_eq(restored.intents.as_ref().unwrap(), &old_intents));
    assert_eq!(apply(&mut session, late_before_delete), 0, "even a matching old B job no longer owns the restored entry");
    assert_eq!(apply(&mut session, rendered(receive(&queue))), 1);
    assert!(session.effect(id).unwrap().is_rendered());
    assert_eq!(level(&session, id), 0.25);
}

#[test]
fn undo_deleted_completed_effect_reuses_valid_cache_and_rejects_old_generation_one() {
    let (mut session, id, queue) = fixture();
    let late_a = rendered(receive(&queue)); // A is deliberately held while B completes.
    session.set_param(id, "IntensityLevel", ParamValue::Double(0.25));
    session.render_all_dirty();
    assert_eq!(apply(&mut session, rendered(receive(&queue))), 1);
    let completed = session.effect(id).unwrap().intents.clone().unwrap();
    let removed_generation = session.effect(id).unwrap().generation;
    delete_and_undo(&mut session, id);
    assert!(session.effect(id).unwrap().is_rendered());
    assert!(queue.try_recv().is_err(), "completed cached intents need no fresh render");
    assert!(Arc::ptr_eq(session.effect(id).unwrap().intents.as_ref().unwrap(), &completed));
    assert_eq!(apply(&mut session, late_a), 0, "old generation 1 cannot overwrite restored B");
    assert_eq!(level(&session, id), 0.25);
    assert!(session.effect(id).unwrap().generation > removed_generation);
    let restored_generation = session.effect(id).unwrap().generation;
    assert_eq!(session.redo(1), 1);
    assert!(session.effect(id).is_none());
    assert_eq!(session.undo(1), 1);
    assert!(session.effect(id).unwrap().generation > restored_generation);
    assert!(Arc::ptr_eq(session.effect(id).unwrap().intents.as_ref().unwrap(), &completed));
    assert!(queue.try_recv().is_err());
}
