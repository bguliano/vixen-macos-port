//! Renders a sequence's effects and places them on the schedule: the C# `PreRender` plus the lookups
//! `EffectNode` does through `VixenSystem`.

use crate::SessionError;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use vixen_core::{ElementTree, NodeId, Uuid};
use vixen_effects::framework::{AudioSource, EffectDescriptor, TargetInfo};
use vixen_engine::render::{render_all, EffectJob, RenderEnv, RenderProgress};
use vixen_engine::schedule::{ElementMap, Schedule, ScheduledEffect};
use vixen_io::{EffectNode, TimedSequence};

/// Everything needed to render a sequence's effects.
pub struct RenderInputs<'a> {
    pub sequence: &'a TimedSequence,
    pub tree: &'a ElementTree,
    pub frame_ms: i32,
    pub audio: Option<&'a dyn AudioSource>,
    pub module_data_root: Option<&'a Path>,
}

/// The target an effect resolves to. `EffectNodeSurrogate.CreateEffectNode` only ever looks at the first one, and
/// an effect whose element has gone holds a `ProxyElementNode` that renders nothing.
pub fn resolve_targets(tree: &ElementTree, node: &EffectNode) -> Vec<NodeId> {
    node.targets.first().and_then(|(id, _)| tree.find(id)).into_iter().collect()
}

/// What assigning each effect's module data and the sequence's mark collections does on load: effects that check their
/// colours replace any their target can't show, some correct their targeting values, and a selected mark collection that
/// has gone falls back to another. See `EffectDescriptor::load_data`. Saving keeps the result, as it does in the original.
pub fn apply_load_rules(sequence: &mut TimedSequence, tree: &ElementTree) {
    let TimedSequence { effects, marks, .. } = sequence;
    for node in effects {
        let Some(descriptor) = vixen_effect_registry::by_type_id(&node.type_id) else { continue };
        let targets = resolve_targets(tree, node);
        let info = TargetInfo { tree, targets: &targets, marks, start_time: node.start, time_span: node.time_span, audio_loaded: false };
        descriptor.load_data(&mut node.params, &info);
    }
}

/// The layer an effect sits on: its position in `SequenceLayers`, default layer when unassigned.
pub fn layer_of(sequence: &TimedSequence, effect: &Uuid) -> u16 {
    sequence.layers.layer_index_for_effect(effect) as u16
}

/// One effect's failure to render or to be matched to a module.
#[derive(Clone, Debug)]
pub struct RenderProblem {
    pub effect: Uuid,
    pub message: String,
}

/// The result of rendering a whole sequence.
pub struct RenderedSequence {
    pub effects: Vec<ScheduledEffect>,
    pub schedule: Arc<Schedule>,
    pub problems: Vec<RenderProblem>,
}

/// Renders every effect of a sequence in parallel and builds the schedule.
pub fn render_sequence(inputs: &RenderInputs, cancel: &AtomicBool, progress: &RenderProgress) -> Result<RenderedSequence, SessionError> {
    let sequence = inputs.sequence;
    let mut problems = Vec::new();

    struct Prepared<'a> {
        node: &'a EffectNode,
        descriptor: &'static EffectDescriptor,
        targets: Vec<NodeId>,
    }
    let mut prepared = Vec::with_capacity(sequence.effects.len());
    for node in &sequence.effects {
        match vixen_effect_registry::by_type_id(&node.type_id) {
            Some(descriptor) => prepared.push(Prepared { node, descriptor, targets: resolve_targets(inputs.tree, node) }),
            None => problems.push(RenderProblem {
                effect: node.instance_id,
                message: format!("no effect module with type {}", node.type_id),
            }),
        }
    }

    let jobs: Vec<EffectJob> = prepared
        .iter()
        .map(|p| EffectJob {
            id: p.node.instance_id,
            descriptor: p.descriptor,
            params: &p.node.params,
            targets: &p.targets,
            start: p.node.start,
            span: p.node.time_span,
        })
        .collect();

    let env = RenderEnv { video_cache: None,
        tree: inputs.tree,
        frame_ms: inputs.frame_ms,
        marks: &sequence.marks,
        audio: inputs.audio,
        module_data_root: inputs.module_data_root,
    };
    let errors = parking_lot::Mutex::new(Vec::new());
    let rendered = render_all(&env, &jobs, cancel, progress, &|message| errors.lock().push(message));
    problems.extend(errors.into_inner().into_iter().map(|message| RenderProblem { effect: Uuid::nil(), message }));

    let effects: Vec<ScheduledEffect> = prepared
        .iter()
        .zip(rendered)
        .map(|(p, intents)| ScheduledEffect {
            id: p.node.instance_id,
            start: p.node.start,
            span: p.node.time_span,
            layer: layer_of(sequence, &p.node.instance_id),
            intents,
        })
        .collect();

    let elements = Arc::new(ElementMap::from_tree(inputs.tree));
    let schedule = Arc::new(Schedule::build(elements, sequence.layers.layers.clone(), &effects));
    Ok(RenderedSequence { effects, schedule, problems })
}
