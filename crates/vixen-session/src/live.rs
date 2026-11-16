//! Named live contexts shared by remote element commands and State preview (F66/F75).

use crate::app::AppCore;
use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use vixen_core::{Color, EffectIntents, Intent, TimeSpan};
use vixen_core::layer::SequenceLayers;
use vixen_engine::{ContextPlayback, ElementMap, Schedule, ScheduledEffect};
use vixen_effects::framework::{EffectDescriptor, Params, RenderCtx};

pub const INDEFINITE: TimeSpan = TimeSpan(30 * 24 * 60 * 60 * 10_000_000);

struct LiveEffect { target: Uuid, scheduled: ScheduledEffect }
struct LiveContext {
    elements: Arc<ElementMap>,
    playback: Arc<ContextPlayback>,
    effects: Vec<LiveEffect>,
}
impl LiveContext {
    fn publish(&self) {
        let copies: Vec<_> = self.effects.iter().map(|effect| effect.scheduled.clone()).collect();
        self.playback.set_schedule(Arc::new(Schedule::build(self.elements.clone(), SequenceLayers::default().layers, &copies)));
    }
}

#[derive(Default)]
struct Shared {
    groups: Mutex<HashMap<String, LiveContext>>,
    wake: Condvar,
    stop: AtomicBool,
}

/// One worker releases expired rendered intents; playback itself stays on the engine
/// frame thread. Context names are case-insensitive, as LivePreviewService's dictionary.
pub struct LiveContexts {
    shared: Arc<Shared>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Default for LiveContexts {
    fn default() -> Self {
        let shared = Arc::new(Shared::default());
        let worker_shared = shared.clone();
        let worker = std::thread::Builder::new().name("live-contexts".into()).spawn(move || {
            let mut groups = worker_shared.groups.lock();
            while !worker_shared.stop.load(Ordering::Acquire) {
                groups.retain(|_, context| !context.playback.is_finished());
                for context in groups.values_mut() {
                    let now = context.playback.position();
                    let count = context.effects.len();
                    context.effects.retain(|effect| effect.scheduled.start + effect.scheduled.span > now);
                    if count != context.effects.len() { context.publish(); }
                }
                if groups.values().all(|context| context.effects.is_empty()) {
                    worker_shared.wake.wait(&mut groups);
                } else {
                    worker_shared.wake.wait_for(&mut groups, Duration::from_millis(100));
                }
            }
        }).ok();
        Self { shared, worker }
    }
}

impl Drop for LiveContexts {
    fn drop(&mut self) {
        // Pair the stop with the wait's mutex so a wake cannot be lost.
        let groups = self.shared.groups.lock();
        self.shared.stop.store(true, Ordering::Release);
        self.shared.wake.notify_all();
        drop(groups);
        if let Some(worker) = self.worker.take() { let _ = worker.join(); }
    }
}

impl LiveContexts {
    fn name(name: &str) -> String { if name.is_empty() { "livepreview".into() } else { name.to_lowercase() } }

    fn context<'a>(groups: &'a mut HashMap<String, LiveContext>, core: &AppCore, name: &str) -> &'a mut LiveContext {
        let key = Self::name(name);
        let elements = core.playback.elements().clone();
        if groups.get(&key).is_some_and(|c| c.playback.is_finished() || !Arc::ptr_eq(&c.elements, &elements)) { groups.remove(&key); }
        groups.entry(key).or_insert_with(|| {
            let schedule = Arc::new(Schedule::build(elements.clone(), SequenceLayers::default().layers, &[]));
            let playback = core.playback.start_live_context(if name.is_empty() { "LivePreview" } else { name }, schedule);
            LiveContext { elements, playback, effects: Vec::new() }
        })
    }

    pub fn ensure(&self, core: &AppCore, name: &str) {
        Self::context(&mut self.shared.groups.lock(), core, name);
    }

    pub fn insert(&self, core: &AppCore, name: &str, target: Uuid, intents: EffectIntents, duration: TimeSpan) {
        let mut groups = self.shared.groups.lock();
        let context = Self::context(&mut groups, core, name);
        let start = context.playback.position();
        context.effects.push(LiveEffect { target, scheduled: ScheduledEffect { id: Uuid::new_v4(), start, span: duration, layer: 0, intents: Arc::new(intents) } });
        context.publish();
        self.shared.wake.notify_all();
    }

    pub fn turn_off(&self, name: &str, target: Uuid) {
        if let Some(context) = self.shared.groups.lock().get_mut(&Self::name(name)) {
            // Exact original target only: turning off a leaf does not stop its parent.
            context.effects.retain(|effect| effect.target != target);
            context.publish();
        }
    }

    pub fn clear(&self, name: &str) {
        if let Some(context) = self.shared.groups.lock().get_mut(&Self::name(name)) { context.effects.clear(); context.publish(); }
    }

    pub fn release(&self, name: &str) { self.shared.groups.lock().remove(&Self::name(name)); }
    pub fn release_all(&self) { self.shared.groups.lock().clear(); }

    pub fn render(&self, core: &AppCore, name: &str, target: Uuid, descriptor: &'static EffectDescriptor, mut params: Params, duration: TimeSpan) -> Result<(), String> {
        let env = core.session_environment();
        let node = env.tree.find(&target).ok_or("Invalid element id.")?;
        let targets = [node];
        let cancel = AtomicBool::new(false);
        let cache_lease = core.video_cache.session();
        let context = RenderCtx { video_cache: Some(vixen_video::cache::CacheContext { session: cache_lease.session(), instance: Uuid::new_v4() }), tree: &env.tree, targets: &targets, start_time: TimeSpan::ZERO, time_span: duration,
            frame_ms: env.frame_ms, marks: &[], audio: None, module_data_root: env.module_data_root.as_deref(), cancel: &cancel };
        descriptor.set_targets(&mut params, &context.target_info());
        let intents = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (descriptor.render)(&context, &params)))
            .map_err(|_| "The effect could not be rendered.".to_string())?;
        self.insert(core, name, target, intents, duration);
        Ok(())
    }

    /// The State mapper supplies leaf Element IDs and its authored colors directly.
    pub fn state_preview(&self, core: &AppCore, elements: &[(Uuid, Color, bool)]) {
        let duration = TimeSpan(i32::MAX as i64 * 10_000_000);
        let mut intents = EffectIntents::new();
        for &(id, color, discrete) in elements {
            let intent = if discrete { Intent::static_discrete(color, 1.0, duration) } else { Intent::static_lighting(color, 1.0, duration) };
            intents.add_intent_for_element(id, intent, TimeSpan::ZERO);
        }
        self.clear("State Preview");
        self.insert(core, "State Preview", Uuid::nil(), intents, duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{CombinedState, ElementTree};
    use vixen_effects::framework::ParamValue;
    use vixen_engine::FrameSink;
    use crate::sequence::SequenceSession;

    struct Recorder(Arc<Mutex<Vec<CombinedState>>>);
    impl FrameSink for Recorder {
        fn frame(&mut self, _: TimeSpan, states: &[CombinedState], _: bool) { *self.0.lock() = states.to_vec(); }
    }
    fn wait(mut done: impl FnMut() -> bool) -> bool {
        for _ in 0..300 { if done() { return true; } std::thread::sleep(Duration::from_millis(5)); }
        false
    }
    fn core() -> (Arc<AppCore>, Uuid, Uuid, Uuid) {
        let mut system = vixen_io::SystemData::default();
        let mut tree = ElementTree::new();
        let parent = tree.add_node(Uuid::new_v4(), "Group", None);
        let child = tree.add_element("Leaf");
        tree.add_child(parent, child);
        let ids = (tree.node(parent).id, tree.node(child).id, tree.node(child).element_id.unwrap());
        system.tree = tree;
        system.default_update_interval = 2;
        let core = AppCore::assemble(vixen_io::DataPaths::new(std::env::temp_dir().join(format!("vixen-live-{}", Uuid::new_v4()))), system,
            Default::default(), None, crate::boot::LaunchOptions { muted: true, ..Default::default() }, None);
        (Arc::new(core), ids.0, ids.1, ids.2)
    }
    fn descriptor() -> &'static EffectDescriptor { vixen_effect_registry::all().iter().find(|d| d.name == "Set Level").unwrap() }
    fn color(states: &Mutex<Vec<CombinedState>>) -> Option<Color> {
        states.lock().first().and_then(|s| s.lighting.map(|v| v.full_color()).or_else(|| s.rgb.map(|v| v.full_color())))
    }
    fn render(core: &AppCore, name: &str, target: Uuid, color: Color, duration: TimeSpan) {
        let mut params = descriptor().new_params();
        params.set("Color", ParamValue::Rgb(vixen_core::color::Rgb::from_color(color)));
        core.live.render(core, name, target, descriptor(), params, duration).unwrap();
    }
    #[test]
    fn state_preview_persists_and_mixes_without_stopping_a_sequence() {
        let (core, _, node, element) = core();
        let states = Arc::new(Mutex::new(Vec::new()));
        core.playback.add_sink(Box::new(Recorder(states.clone())));
        let mut session = SequenceSession::new_untitled(&core.session_environment());
        let id = session.add_new_effect(descriptor().type_id, node, "Leaf", TimeSpan::ZERO, TimeSpan::from_secs(10.0)).unwrap();
        session.set_param(id, "Color", ParamValue::Rgb(vixen_core::color::Rgb::from_color(Color::RED)));
        session.wait_for_renders();
        core.playback.play(&mut session, TimeSpan::ZERO, TimeSpan::from_secs(10.0), false);
        assert!(wait(|| color(&states) == Some(Color::RED)));
        core.live.state_preview(&core, &[(element, Color::BLUE, false)]);
        assert!(wait(|| color(&states) == Some(Color::rgb(255, 0, 255))));
        std::thread::sleep(Duration::from_millis(25));
        assert_eq!(color(&states), Some(Color::rgb(255, 0, 255)));
        core.live.release("state preview");
        assert!(wait(|| color(&states) == Some(Color::RED)));
        assert_eq!(core.playback.transport(), crate::playback::Transport::Playing);
    }
    #[test]
    fn off_matches_exact_targets_and_names_ignore_case() {
        let (core, parent, child, _) = core();
        let states = Arc::new(Mutex::new(Vec::new()));
        core.playback.add_sink(Box::new(Recorder(states.clone())));
        render(&core, "Web Server", parent, Color::RED, INDEFINITE);
        render(&core, "WEB SERVER", child, Color::BLUE, INDEFINITE);
        assert_eq!(core.playback.context_states().len(), 1);
        assert!(wait(|| color(&states) == Some(Color::rgb(255, 0, 255))));
        core.live.turn_off("web server", child);
        assert!(wait(|| color(&states) == Some(Color::RED)));
        core.live.turn_off("web server", child);
        assert_eq!(core.live.shared.groups.lock()["web server"].effects.len(), 1);
        core.live.clear("web server");
        assert!(wait(|| color(&states).is_none()));
    }
    #[test]
    fn expiry_releases_intents_and_closed_execution_recreates_clean_contexts() {
        let (core, _, child, _) = core();
        core.playback.open_execution();
        render(&core, "LivePreview", child, Color::RED, TimeSpan::from_ms_i(10));
        let old = {
            let groups = core.live.shared.groups.lock();
            Arc::downgrade(&groups["livepreview"].effects[0].scheduled.intents)
        };
        assert!(wait(|| old.upgrade().is_none()));
        core.playback.close_execution();
        core.playback.open_execution();
        render(&core, "LivePreview", child, Color::BLUE, INDEFINITE);
        let groups = core.live.shared.groups.lock();
        assert_eq!(groups["livepreview"].effects.len(), 1);
        assert!(!groups["livepreview"].playback.is_finished());
    }
    #[test]
    fn empty_live_contexts_do_not_block_effect_preview() {
        let (core, ..) = core();
        core.live.ensure(&core, "Web Server");
        assert!(core.playback.effect_preview_may_start());
        let session = SequenceSession::new_untitled(&core.session_environment());
        let claim = core.playback.begin_effect_preview(session.schedule()).unwrap();
        core.live.ensure(&core, "Other");
        assert!(core.playback.effect_preview_is_playing(claim));
        assert!(core.playback.end_effect_preview(claim));
    }
}
