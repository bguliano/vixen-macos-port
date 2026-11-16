//! Sequence contexts, their clocks, and the shared output stage. Editor transport and
//! scheduled sequences have independent lifetimes; their raw layers mix in the engine.

use crate::execution::{ExecutionDevices, ExecutionEngine};
pub use crate::execution::ExecutionState;
use crate::instrumentation::{format_milliseconds, InstrumentationValue, MillisecondsValue};
use crate::preview::PreviewHub;
use crate::preview_devices::PreviewDevices;
use crate::sequence::SequenceSession;
use crate::show::build_output_graph;
use crate::transport::AudioTimeSource;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use uuid::Uuid;
use vixen_audio::{AudioClip, AudioPlayer};
use vixen_core::{CombinedState, TimeSpan};
use vixen_engine::schedule::{ElementMap, Schedule};
use vixen_engine::{ContextPlayback, Engine, EngineStats, FrameSink, InternalClock, TimeSource, TimingTotals};
pub use vixen_engine::Transport;
use vixen_io::SystemData;
use vixen_output::{ControllerRunner, OutputGraph, TimingSummary};

/// The compiled output graph and the controllers it drives.
struct OutputStage {
    graph: OutputGraph,
    runner: ControllerRunner,
    /// The engine is open, so frames reach the controllers.
    publishing: bool,
}

struct OutputSink {
    stage: Arc<Mutex<OutputStage>>,
    filters_update: Arc<MillisecondsValue>,
}

impl FrameSink for OutputSink {
    fn frame(&mut self, _time: TimeSpan, states: &[CombinedState], _any_state: bool) {
        let mut guard = self.stage.lock();
        let stage = &mut *guard;
        let started = Instant::now();
        stage.graph.evaluate(states);
        self.filters_update.set(started.elapsed().as_secs_f64() * 1000.0);
        if stage.publishing {
            stage.runner.publish(&stage.graph);
        }
    }
}

/// What opening and closing the engine do to the outside world. Muted, controllers stay stopped and only the state changes.
struct StageDevices {
    engine: Arc<Engine>,
    output: Arc<Mutex<OutputStage>>,
    previews: Arc<PreviewDevices>,
    muted: bool,
}

impl ExecutionDevices for StageDevices {
    fn start_devices(&self) {
        {
            let mut stage = self.output.lock();
            if !self.muted {
                stage.runner.start_all();
            }
            stage.publishing = true;
        }
        self.previews.start_all();
    }

    fn stop_devices(&self) {
        self.engine.stop_all();
        {
            let mut stage = self.output.lock();
            stage.publishing = false;
            if !self.muted {
                stage.runner.stop_all();
            }
        }
        self.previews.stop_all();
    }
}

fn format_engine_timing(timing: TimingTotals) -> String {
    let ms = |us: u64| us as f64 / 1000.0;
    let min = if timing.samples == 0 { 999.0 } else { ms(timing.min_us) };
    format_milliseconds(min, ms(timing.total_us), ms(timing.max_us), timing.samples as i64 + 1)
}

fn format_controller_timing(timing: TimingSummary) -> String {
    let min = if timing.samples == 0 { 999.0 } else { timing.min_ms };
    format_milliseconds(min, timing.total_ms, timing.max_ms, timing.samples as i64 + 1)
}

/// Playback for the whole app.
pub struct ShowPlayback {
    engine: Arc<Engine>,
    clock: Arc<InternalClock>,
    /// Created the first time a sequence with audio plays, so an app that never plays never opens a sound device.
    player: Mutex<Option<Arc<AudioPlayer>>>,
    audio_device: Mutex<Option<String>>,
    /// No sound and no output hardware, for tests.
    muted: bool,
    output: Arc<Mutex<OutputStage>>,
    /// The frame order, newest last. Each map is boxed and stays until playback goes away, because `elements` hands out a plain
    /// reference and a reload can replace the current one.
    elements: Mutex<Vec<Box<Arc<ElementMap>>>>,
    speed: Mutex<f32>,
    preserve_pitch: AtomicBool,
    previews: PreviewHub,
    preview_devices: Arc<PreviewDevices>,
    execution: ExecutionEngine,
    filters_update: Arc<MillisecondsValue>,
    /// The Options dialog's Latency, or 0 until it has been read or set.
    audio_latency_ms: AtomicI32,
    /// Each running controller's update count at the last Instrumentation reading, for its refresh rate.
    refresh_readings: Mutex<HashMap<Uuid, (u64, Instant)>>,
    effect_preview: Mutex<EffectPreviewGate>,
    primary_name: Mutex<(String, String)>,
    /// Serializes primary-editor starts and session-scoped reads/commands. Scheduled
    /// and live contexts keep their independent handles and never acquire ownership.
    primary_session: Mutex<Option<Uuid>>,
    contexts: Mutex<Vec<NamedContext>>,
}

struct NamedContext {
    name: String,
    file_name: String,
    playback: Weak<ContextPlayback>,
}

#[derive(Clone, Debug)]
pub struct ContextStatus {
    pub name: String,
    pub file_name: String,
    pub transport: Transport,
    pub position: TimeSpan,
}

/// Who has the engine: the Effect Preview's live context, or everything else. One sequence plays at a time here, so what upstream does with a second context, this does by taking turns: real playback
/// takes the engine from a running preview, and the preview comes back when the engine is free again (`docs/spec/effect-editor.md` F6).
#[derive(Default)]
struct EffectPreviewGate {
    /// Bumps each time the engine changes hands. A preview holding an older number has lost it.
    epoch: u64,
    /// The engine is playing a preview: the transport the windows see stays stopped, and their Stop, Pause and Seek are for someone else.
    active: bool,
    /// Export, or anyone else who asked: no preview starts until it's let go.
    held: bool,
}

impl ShowPlayback {
    /// `elements` fixes the order of each frame's states, and has to be the sessions' order.
    pub fn new(system: &SystemData, elements: Arc<ElementMap>, muted: bool) -> ShowPlayback {
        let preferences = vixen_io::load_audio_preferences(&vixen_io::settings_dir());
        Self::with_audio_preferences(system, elements, muted, &preferences, |name| {
            // Muted tests never open a device and must not enumerate hardware.
            muted || vixen_audio::output_devices().iter().any(|device| device == name)
        })
    }

    fn with_audio_preferences(
        system: &SystemData,
        elements: Arc<ElementMap>,
        muted: bool,
        preferences: &vixen_io::AudioPreferences,
        device_available: impl FnOnce(&str) -> bool,
    ) -> ShowPlayback {
        // AudioOutputManager restores this before any player is constructed,
        // including scheduler/web playback with no sequencer window open.
        let device = &preferences.sound_out_device_id;
        let audio_device = (device != "-0" && device_available(device)).then(|| device.clone());
        let interval = Duration::from_millis(system.default_update_interval.max(1) as u64);
        let graph = build_output_graph(system, &elements.ids);
        let disabled: Vec<Uuid> = system.disabled_devices.clone();
        let mut runner = ControllerRunner::from_graph(&graph, &disabled, interval);
        runner.set_log_sink(Arc::new(crate::logging::error));
        let output = Arc::new(Mutex::new(OutputStage { graph, runner, publishing: false }));
        let engine = Arc::new(Engine::new(interval));
        engine.set_elements(elements.clone());
        let clock = Arc::new(InternalClock::default());
        engine.set_source(clock.clone());
        let filters_update = Arc::new(MillisecondsValue::new("   Filters update"));
        engine.add_sink(Box::new(OutputSink { stage: Arc::clone(&output), filters_update: Arc::clone(&filters_update) }));
        let previews = PreviewHub::default();
        engine.add_sink(Box::new(previews.sink()));
        let preview_devices = Arc::new(PreviewDevices::new(&system.previews, &disabled));
        let execution = ExecutionEngine::new(Box::new(StageDevices {
            engine: Arc::clone(&engine),
            output: Arc::clone(&output),
            previews: Arc::clone(&preview_devices),
            muted,
        }));
        ShowPlayback {
            engine,
            clock,
            player: Mutex::new(None),
            audio_device: Mutex::new(audio_device),
            muted,
            output,
            elements: Mutex::new(vec![Box::new(elements)]),
            speed: Mutex::new(1.0),
            preserve_pitch: AtomicBool::new(false),
            previews,
            preview_devices,
            execution,
            filters_update,
            audio_latency_ms: AtomicI32::new(0),
            refresh_readings: Mutex::new(HashMap::new()),
            effect_preview: Mutex::new(EffectPreviewGate::default()),
            primary_name: Mutex::new((String::new(), String::new())),
            primary_session: Mutex::new(None),
            contexts: Mutex::new(Vec::new()),
        }
    }

    pub fn elements(&self) -> &Arc<ElementMap> {
        let latest: *const Arc<ElementMap> = &**self.elements.lock().last().expect("there is always one");
        // SAFETY: each map is boxed, so it stays put when the list grows, and the list only grows until `self` goes away.
        unsafe { &*latest }
    }

    /// Drain contexts that an in-flight service request could have started while
    /// shutdown was joining that service's workers.
    pub fn stop_all_contexts(&self) { self.engine.stop_all(); }

    /// Replaces the output graph, the controllers and the frame order, after a config load or an edit. Close the engine first.
    pub fn rebuild(&self, system: &SystemData, elements: Arc<ElementMap>) {
        self.engine.stop_all();
        self.engine.set_elements(elements.clone());
        let interval = Duration::from_millis(system.default_update_interval.max(1) as u64);
        let graph = build_output_graph(system, &elements.ids);
        let mut runner = ControllerRunner::from_graph(&graph, &system.disabled_devices, interval);
        runner.set_log_sink(Arc::new(crate::logging::error));
        {
            let mut stage = self.output.lock();
            stage.graph = graph;
            stage.runner = runner;
        }
        self.elements.lock().push(Box::new(elements));
        self.preview_devices.configure(&system.previews, &system.disabled_devices);
        self.engine.set_interval(interval);
    }

    /// Display Setup's OK: the new output graph and frame order, with no device restarted unless its own configuration or output count changed, or Display Setup started
    /// or stopped it. What's playing stops, because its schedule is built against the old elements.
    pub fn replace_display(&self, system: &SystemData, elements: Arc<ElementMap>) {
        self.engine.stop_all();
        self.engine.set_elements(elements.clone());
        let graph = build_output_graph(system, &elements.ids);
        {
            let mut guard = self.output.lock();
            let stage = &mut *guard;
            stage.graph = graph;
            stage.runner.reconcile(&stage.graph, &system.disabled_devices, self.execution.is_open() && !self.muted);
        }
        self.elements.lock().push(Box::new(elements));
        self.preview_devices.configure(&system.previews, &system.disabled_devices);
    }

    /// False when muted: nothing may touch hardware.
    pub fn hardware_allowed(&self) -> bool {
        !self.muted
    }

    /// Which controllers are really running, or `None` when that says nothing because the engine is closed or muted.
    pub fn running_controllers(&self) -> Option<std::collections::HashSet<Uuid>> {
        if self.muted || !self.execution.is_open() {
            return None;
        }
        let stage = self.output.lock();
        Some((0..stage.runner.controller_count()).filter(|i| stage.runner.is_running(*i)).filter_map(|i| stage.runner.controller_id(i)).collect())
    }

    pub fn controller_states(&self) -> Vec<(Uuid, bool, bool)> {
        let stage = self.output.lock();
        (0..stage.runner.controller_count()).filter_map(|index| {
            stage.runner.controller_id(index).map(|id| (id, stage.runner.is_running(index), stage.runner.is_paused(index)))
        }).collect()
    }

    /// Display Setup's Start and Stop: at once, as upstream. With the engine closed it only decides what the engine starts when it opens. Blocks until a stopping controller has stopped.
    pub fn set_controller_running(&self, id: &Uuid, running: bool) {
        let mut stage = self.output.lock();
        let Some(index) = stage.runner.controller_index(id) else { return };
        stage.runner.set_enabled(index, running);
        if running { stage.runner.set_paused(index, false); }
        if running && !self.muted && self.execution.is_open() {
            stage.runner.start(index);
        }
    }

    /// Adds a consumer of every frame.
    pub fn add_sink(&self, sink: Box<dyn FrameSink>) {
        self.engine.add_sink(sink);
    }

    /// Open live previews, which get every frame.
    pub fn previews(&self) -> &PreviewHub {
        &self.previews
    }

    /// The engine's state machine.
    pub fn execution(&self) -> &ExecutionEngine {
        &self.execution
    }

    /// Previews as output devices: which should be showing.
    pub fn preview_devices(&self) -> &Arc<PreviewDevices> {
        &self.preview_devices
    }

    /// `Execution.OpenExecution`: starts every enabled controller. Muted, it only changes the state.
    pub fn open_execution(&self) {
        self.execution.open();
    }

    /// `Execution.CloseExecution`: stops whatever's playing and every controller.
    pub fn close_execution(&self) {
        self.execution.close();
    }

    pub fn execution_state(&self) -> ExecutionState {
        self.execution.state()
    }

    /// The system's controllers that aren't running, in system order, for `SaveDisabledDevices`.
    pub fn controllers_not_running(&self, system: &SystemData) -> Vec<Uuid> {
        let stage = self.output.lock();
        system
            .controllers
            .iter()
            .filter(|c| !stage.runner.controller_index(&c.id).is_some_and(|i| stage.runner.is_running(i)))
            .map(|c| c.id)
            .collect()
    }

    /// `DisabledDevices` has been recorded: from now on the engine starts these and only these.
    pub fn apply_disabled_controllers(&self, disabled: &[Uuid]) {
        let mut stage = self.output.lock();
        for i in 0..stage.runner.controller_count() {
            if let Some(id) = stage.runner.controller_id(i) {
                stage.runner.set_enabled(i, !disabled.contains(&id));
            }
        }
    }

    /// `DefaultUpdateInterval`'s setter. The dialog allows 0, which the engine would take as a microsecond, so it gets at least 1 ms.
    pub fn set_update_interval(&self, ms: i32) {
        let interval = Duration::from_millis(ms.max(1) as u64);
        self.engine.set_interval(interval);
        let stage = self.output.lock();
        for i in 0..stage.runner.controller_count() {
            stage.runner.set_update_interval(i, interval);
        }
    }

    /// The Audio Devices choice, by name; `None` is the system default. The open stream moves to it at once.
    pub fn set_audio_device(&self, name: Option<&str>) {
        *self.audio_device.lock() = name.map(str::to_string);
        if let Some(player) = self.player.lock().as_ref() {
            player.set_device(name);
        }
    }

    /// The Options dialog's Latency. Takes effect when the next stream opens.
    pub fn set_audio_latency_ms(&self, ms: i32) {
        self.audio_latency_ms.store(ms, Ordering::Relaxed);
        if let Some(player) = self.player.lock().as_ref() {
            player.set_latency_ms(ms.max(1) as u32);
        }
    }

    /// `System allowed update`, and the two parts of a frame that have their own rows. The original's `allowed block`, `denied update`, `denied
    /// block` and `Elements update` have no counterpart: one frame thread runs without an adjudicator, and elements come out of the
    /// same pass as contexts.
    pub fn engine_instrumentation_lines(&self) -> Vec<String> {
        let stats = self.engine.stats();
        vec![
            format!("System allowed update: {}", format_engine_timing(stats.frame_timing)),
            format!("   Contexts update: {}", format_engine_timing(stats.evaluate_timing)),
            format!("{}: {}", self.filters_update.name(), self.filters_update.formatted()),
        ]
    }

    /// Each running controller's rows. `triggered engine update` has no counterpart, since a controller doesn't drive the engine here.
    pub fn controller_instrumentation_lines(&self) -> Vec<String> {
        let stage = self.output.lock();
        let mut readings = self.refresh_readings.lock();
        let mut lines = Vec::new();
        for i in 0..stage.runner.controller_count() {
            let Some(id) = stage.runner.controller_id(i) else {
                continue;
            };
            if !stage.runner.is_running(i) {
                readings.remove(&id);
                continue;
            }
            let name = stage.runner.controller_name(i).unwrap_or_default();
            let timings = stage.runner.timings(i);
            let updates = stage.runner.stats(i).updates;
            let now = Instant::now();
            let (last_updates, last_read) = readings.insert(id, (updates, now)).unwrap_or((updates, now));
            let elapsed = now.duration_since(last_read).as_secs_f64();
            let rate = if elapsed > 0.0 { (updates - last_updates) as f64 / elapsed } else { 0.0 };
            lines.push(format!("{name} Output Total Update: {}", format_controller_timing(timings.update)));
            lines.push(format!("{name} delta: {}", format_controller_timing(timings.delta)));
            lines.push(format!("{name} refresh rate: {rate:.2} Hz"));
            lines.push(format!("{name} sleep time: {}", format_controller_timing(timings.sleep)));
        }
        lines
    }

    /// The Instrumentation window's Reset, for the values kept here.
    pub fn reset_instrumentation(&self) {
        self.engine.reset_timings();
        self.filters_update.reset();
        let stage = self.output.lock();
        for i in 0..stage.runner.controller_count() {
            stage.runner.reset_timings(i);
        }
    }

    fn audio_player(&self) -> Arc<AudioPlayer> {
        let mut slot = self.player.lock();
        Arc::clone(slot.get_or_insert_with(|| {
            let player = AudioPlayer::new(self.audio_device.lock().as_deref());
            let latency = match self.audio_latency_ms.load(Ordering::Relaxed) {
                0 => vixen_io::load_audio_preferences(&vixen_io::settings_dir()).latency_ms,
                set => set,
            };
            player.set_latency_ms(latency.max(1) as u32);
            Arc::new(player)
        }))
    }

    /// A show's Sequence action owns its own audio/clock and never takes over the editor.
    /// In muted tests this advances on a monotonic clock without opening an audio device.
    pub fn start_sequence_context(&self, schedule: Arc<Schedule>, audio: Option<Arc<AudioClip>>, length: TimeSpan) -> ContextPlayback {
        let mut gate = self.effect_preview.lock();
        if gate.active {
            gate.epoch += 1;
            gate.active = false;
            self.engine.stop();
        }
        let source: Arc<dyn TimeSource> = match audio.filter(|_| !self.muted) {
            Some(clip) => {
                let player = AudioPlayer::new(self.audio_device.lock().as_deref());
                let configured = self.audio_latency_ms.load(Ordering::Relaxed);
                let latency = if configured == 0 { vixen_io::load_audio_preferences(&vixen_io::settings_dir()).latency_ms } else { configured };
                player.set_latency_ms(latency.max(1) as u32);
                player.set_clip(clip);
                Arc::new(AudioTimeSource(Arc::new(player)))
            }
            None => Arc::new(InternalClock::default()),
        };
        self.engine.start_context(schedule, source, length)
    }

    pub fn start_named_context(&self, name: &str, file_name: &str, schedule: Arc<Schedule>, audio: Option<Arc<AudioClip>>, length: TimeSpan) -> Arc<ContextPlayback> {
        let playback = Arc::new(self.start_sequence_context(schedule, audio, length));
        self.register_context(name, file_name, playback)
    }

    pub fn start_live_context(&self, name: &str, schedule: Arc<Schedule>) -> Arc<ContextPlayback> {
        self.register_context(name, "", Arc::new(self.engine.start_live_context(schedule)))
    }

    fn register_context(&self, name: &str, file_name: &str, playback: Arc<ContextPlayback>) -> Arc<ContextPlayback> {
        let mut contexts = self.contexts.lock();
        contexts.retain(|c| c.playback.upgrade().is_some_and(|p| !p.is_finished()));
        contexts.push(NamedContext { name: name.into(), file_name: file_name.into(), playback: Arc::downgrade(&playback) });
        playback
    }

    /// Context owners retain the handles; this registry never prolongs their lifetime.
    pub fn context_states(&self) -> Vec<ContextStatus> {
        let mut contexts = self.contexts.lock();
        contexts.retain(|c| c.playback.upgrade().is_some_and(|p| !p.is_finished()));
        let mut result: Vec<_> = contexts.iter().filter_map(|c| c.playback.upgrade().map(|p| ContextStatus {
            name: c.name.clone(), file_name: c.file_name.clone(), transport: p.transport(), position: p.position(),
        })).collect();
        drop(contexts);
        let transport = self.engine.transport();
        if transport != Transport::Stopped {
            let (name, file_name) = self.primary_name.lock().clone();
            result.insert(0, ContextStatus { name, file_name, transport, position: self.engine.position() });
        }
        result
    }

    /// A web command matches every context of that name, including scheduled copies.
    pub fn control_contexts(&self, name: &str, command: Transport) -> usize {
        let contexts: Vec<_> = self.contexts.lock().iter().filter(|c| c.name == name).filter_map(|c| c.playback.upgrade()).collect();
        let mut changed = 0;
        for context in contexts {
            let state = context.transport();
            if state == Transport::Stopped || (command == Transport::Playing && state != Transport::Paused) { continue; }
            match command { Transport::Stopped => context.stop(), Transport::Paused => context.pause(), Transport::Playing => context.resume() }
            changed += 1;
        }
        let primary_matches = self.primary_name.lock().0 == name;
        let state = self.engine.transport();
        if primary_matches && state != Transport::Stopped && (command != Transport::Playing || state == Transport::Paused) {
            match command { Transport::Stopped => self.stop(), Transport::Paused => self.pause(), Transport::Playing => self.resume() }
            changed += 1;
        }
        changed
    }

    /// `SequenceExecutor.Play`: starts `session` from `start` to `end`, clamped to its length. The session's future schedules
    /// follow along, so edits show as soon as they finish rendering.
    pub fn play(self: &Arc<Self>, session: &mut SequenceSession, start: TimeSpan, end: TimeSpan, looping: bool) {
        let mut owner = self.primary_session.lock();
        *self.primary_name.lock() = (session.name(), session.path().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        let length = session.length();
        let start = start.min(length);
        let end = end.min(length);

        {
            // F6's rule: real playback stops the Effect Preview. Under the gate, so a loop can't put its schedule over this one.
            let mut gate = self.effect_preview.lock();
            gate.epoch += 1;
            gate.active = false;
        }
        self.engine.stop();
        self.attach_session_inner(session);

        let source: Arc<dyn TimeSource> = match session.audio().filter(|_| !self.muted) {
            Some(clip) => {
                let player = self.audio_player();
                player.set_clip(Arc::clone(clip));
                player.set_rate_mode(self.preserve_pitch.load(Ordering::Relaxed));
                player.set_speed(*self.speed.lock());
                Arc::new(AudioTimeSource(player))
            }
            None => {
                self.clock.set_speed(*self.speed.lock() as f64);
                self.clock.clone()
            }
        };
        source.seek(start);
        self.engine.set_source(source);
        self.engine.play(start, end, looping);
        *owner = Some(session.playback_id());
    }

    /// The session's schedules go to the engine from here on, starting with the current one.
    pub fn attach_session(self: &Arc<Self>, session: &mut SequenceSession) {
        let _owner = self.primary_session.lock();
        self.attach_session_inner(session);
    }

    fn attach_session_inner(self: &Arc<Self>, session: &mut SequenceSession) {
        let claim = {
            let mut gate = self.effect_preview.lock();
            // A failed or retired preview must not reclaim a newly playing editor.
            if gate.active || self.engine.transport() != Transport::Stopped { return; }
            gate.epoch = gate.epoch.wrapping_add(1);
            gate.epoch
        };
        let weak = Arc::downgrade(self);
        session.set_schedule_listener(Some(Box::new(move |schedule| {
            if let Some(playback) = weak.upgrade() {
                let gate = playback.effect_preview.lock();
                if gate.epoch == claim && !gate.active { playback.engine.set_schedule(schedule); }
            }
        })));
    }

    /// F6: whether a preview could start now: nothing else is playing and nobody is holding it off.
    pub fn effect_preview_may_start(&self) -> bool {
        !self.effect_preview.lock().held && self.engine.transport() == Transport::Stopped && !self.engine.has_sequence_contexts()
    }

    /// F6: a schedule of its own, from 0 on the internal clock, with no end and no sound. The Effect Preview's live context. The number it returns is its claim on the engine; `None` means
    /// the engine isn't free.
    pub fn begin_effect_preview(&self, schedule: Arc<Schedule>) -> Option<u64> {
        // A preview must not enter the stopped interval in an editor's Play setup.
        let mut owner = self.primary_session.lock();
        let mut gate = self.effect_preview.lock();
        if gate.held || self.engine.transport() != Transport::Stopped || self.engine.has_sequence_contexts() {
            return None;
        }
        gate.epoch += 1;
        gate.active = true;
        *owner = None;
        *self.primary_name.lock() = ("LivePreview".into(), String::new());
        self.clock.set_speed(1.0);
        let source: Arc<dyn TimeSource> = self.clock.clone();
        self.engine.set_source(source);
        self.engine.set_schedule(schedule);
        self.engine.play(TimeSpan::ZERO, TimeSpan::MAX, false);
        Some(gate.epoch)
    }

    /// Replaces what the preview is playing, keeping the position. False means the engine has gone to someone else.
    pub fn update_effect_preview(&self, claim: u64, schedule: Arc<Schedule>) -> bool {
        let gate = self.effect_preview.lock();
        if !gate.active || gate.epoch != claim {
            return false;
        }
        self.engine.set_schedule(schedule);
        true
    }

    /// Whether the preview with this claim is really still playing. Something can stop the engine under it: Display Setup's OK, closing the execution.
    pub fn effect_preview_is_playing(&self, claim: u64) -> bool {
        let gate = self.effect_preview.lock();
        gate.active && gate.epoch == claim && self.engine.transport() == Transport::Playing
    }

    /// F6's `PreviewStop`: what's lit clears, without waiting for the frame thread. False means the engine had already gone to someone else, and nothing was touched.
    pub fn end_effect_preview(&self, claim: u64) -> bool {
        let mut gate = self.effect_preview.lock();
        if !gate.active || gate.epoch != claim {
            return false;
        }
        gate.epoch += 1;
        gate.active = false;
        self.engine.stop();
        true
    }

    /// Export's `PreviewStop` and `ResumePreview`, for whoever needs the preview out of the way with no panel to hand: it stops now and comes back once this is let go.
    pub fn hold_effect_preview(&self, held: bool) {
        let mut gate = self.effect_preview.lock();
        gate.held = held;
        if held && gate.active {
            gate.epoch += 1;
            gate.active = false;
            self.engine.stop();
        }
    }

    pub fn pause(&self) {
        if !self.effect_preview.lock().active {
            self.engine.pause();
        }
    }

    pub fn resume(&self) {
        if !self.effect_preview.lock().active {
            self.engine.resume();
        }
    }

    /// `SequenceExecutor.Stop`.
    pub fn stop(&self) {
        if !self.effect_preview.lock().active {
            self.engine.stop();
        }
    }

    pub fn seek(&self, to: TimeSpan) {
        if !self.effect_preview.lock().active {
            self.engine.seek(to);
        }
    }

    /// Evaluates the current position once, after an edit or a scrub while stopped.
    pub fn refresh(&self) {
        self.engine.refresh();
    }

    /// The sequence's play position. The Effect Preview's isn't one, so it shows nothing.
    pub fn position(&self) -> TimeSpan {
        if self.effect_preview.lock().active {
            return TimeSpan::ZERO;
        }
        self.engine.position()
    }

    /// The sequence's transport, which an Effect Preview doesn't change.
    pub fn transport(&self) -> Transport {
        if self.effect_preview.lock().active {
            return Transport::Stopped;
        }
        self.engine.transport()
    }

    /// True once after playback reaches its end, for the UI to reset itself.
    pub fn take_finished(&self) -> bool {
        self.engine.take_finished()
    }

    /// A window may observe/control only the editor session it started. Keep the
    /// ownership check and operation under one lock so a stale close/completion
    /// cannot stop or consume the replacement editor's state.
    fn for_session<T>(&self, id: Uuid, action: impl FnOnce() -> T) -> Option<T> {
        let owner = self.primary_session.lock();
        (*owner == Some(id)).then(action)
    }

    pub fn owns_session(&self, id: Uuid) -> bool {
        *self.primary_session.lock() == Some(id)
    }

    pub fn session_transport(&self, id: Uuid) -> Transport {
        self.for_session(id, || self.transport()).unwrap_or(Transport::Stopped)
    }

    pub fn session_position(&self, id: Uuid) -> TimeSpan {
        self.for_session(id, || self.position()).unwrap_or(TimeSpan::ZERO)
    }

    pub fn pause_session(&self, id: Uuid) { self.for_session(id, || self.pause()); }
    pub fn resume_session(&self, id: Uuid) { self.for_session(id, || self.resume()); }
    pub fn stop_session(&self, id: Uuid) { self.for_session(id, || self.stop()); }
    pub fn seek_session(&self, id: Uuid, to: TimeSpan) { self.for_session(id, || self.seek(to)); }
    pub fn set_session_speed(&self, id: Uuid, speed: f32) { self.for_session(id, || self.set_speed(speed)); }
    pub fn set_session_rate_mode(&self, id: Uuid, preserve_pitch: bool) { self.for_session(id, || self.set_rate_mode(preserve_pitch)); }
    pub fn take_session_finished(&self, id: Uuid) -> bool {
        self.for_session(id, || self.take_finished()).unwrap_or(false)
    }

    pub fn stats(&self) -> EngineStats {
        self.engine.stats()
    }

    /// Upstream UseTempo: selects how the next speed change processes audio.
    pub fn set_rate_mode(&self, preserve_pitch: bool) {
        self.preserve_pitch.store(preserve_pitch, Ordering::Relaxed);
    }

    /// The sequencer's speed control, in rate or pitch-preserving tempo mode.
    pub fn set_speed(&self, speed: f32) {
        *self.speed.lock() = speed;
        self.clock.set_speed(speed as f64);
        if let Some(player) = self.player.lock().as_ref() {
            player.set_rate_mode(self.preserve_pitch.load(Ordering::Relaxed));
            player.set_speed(speed);
        }
    }

    /// Runs code with the output graph, for the dialogs that change it live and for the Dummy controller's monitor.
    pub fn with_output<R>(&self, f: impl FnOnce(&mut OutputGraph, &mut ControllerRunner) -> R) -> R {
        let mut guard = self.output.lock();
        let stage = &mut *guard;
        f(&mut stage.graph, &mut stage.runner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::{SequenceSession, SessionEnvironment};
    use vixen_core::ElementTree;
    use vixen_io::ModuleStore;

    const SET_LEVEL: Uuid = uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0");

    fn setup() -> (Arc<ShowPlayback>, SequenceSession, Uuid) {
        let mut system = SystemData::default();
        let mut tree = ElementTree::new();
        let n = tree.add_element("E");
        let node = tree.node(n).id;
        system.tree = tree.clone();
        system.default_update_interval = 10;
        let tree = Arc::new(tree);
        let elements = Arc::new(ElementMap::from_tree(&tree));
        let playback = Arc::new(ShowPlayback::new(&system, elements, true));
        let env = SessionEnvironment { video_cache: None,
            tree,
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 25,
            media_dir: std::env::temp_dir(),
        };
        (playback, SequenceSession::new_untitled(&env), node)
    }

    fn wait_for(mut done: impl FnMut() -> bool) -> bool {
        for _ in 0..300 {
            if done() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn saved_audio_device_is_ready_before_any_editor_or_scheduled_context() {
        let root = std::env::temp_dir().join(format!("vixen-playback-device-{}", Uuid::new_v4()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
        let _cleanup = Cleanup(root.clone());
        std::fs::create_dir(&root).unwrap();
        let mut preferences = vixen_io::AudioPreferences::default();
        preferences.sound_out_device_id = "Saved speakers".into();
        vixen_io::save_audio_preferences(&preferences, &root).unwrap();
        let preferences = vixen_io::load_audio_preferences(&root);
        let elements = Arc::new(ElementMap::default());
        let playback = ShowPlayback::with_audio_preferences(&SystemData::default(), elements.clone(), true, &preferences, |name| name == "Saved speakers");
        assert_eq!(playback.audio_device.lock().as_deref(), Some("Saved speakers"));
        assert!(playback.player.lock().is_none(), "restoring a preference must not open audio");
        let context = playback.start_named_context("Scheduled", "song.tim", Arc::new(Schedule::empty(elements, Vec::new())), None, TimeSpan::from_ms_i(10_000));
        assert_eq!(playback.audio_device.lock().as_deref(), Some("Saved speakers"));
        context.stop();

        let mut core = crate::app::AppCore::open(root.join("profile"), true).unwrap();
        core.playback = Arc::new(playback);
        let core = Arc::new(core);
        core.playback.set_audio_device(Some("Current explicit choice"));
        let sequence = Arc::new(Mutex::new(SequenceSession::new_untitled(&core.session_environment())));
        let settings = Arc::new(vixen_io::ProfileSettings::open(root.join("Settings.xml")));
        let _editor = crate::sequencer::SequencerState::with_settings(core.clone(), sequence, settings, root);
        assert_eq!(core.playback.audio_device.lock().as_deref(), Some("Current explicit choice"), "opening an editor must not restore an older choice again");
        assert!(core.playback.player.lock().is_none());
    }

    #[test]
    fn default_or_missing_saved_audio_device_resolves_without_opening_a_stream() {
        let mut preferences = vixen_io::AudioPreferences::default();
        let elements = Arc::new(ElementMap::default());
        let playback = ShowPlayback::with_audio_preferences(&SystemData::default(), elements.clone(), true, &preferences, |_| panic!("system default needs no enumeration"));
        assert!(playback.audio_device.lock().is_none());
        preferences.sound_out_device_id = "Disconnected speakers".into();
        let playback = ShowPlayback::with_audio_preferences(&SystemData::default(), elements, true, &preferences, |_| false);
        assert!(playback.audio_device.lock().is_none());
        assert!(playback.player.lock().is_none());
    }

    #[test]
    fn plays_to_the_end_of_the_range_and_reports_it_once() {
        let (playback, mut session, node) = setup();
        session.add_new_effect(SET_LEVEL, node, "E", TimeSpan::ZERO, TimeSpan::from_ms_i(50)).unwrap();
        session.wait_for_renders();
        playback.play(&mut session, TimeSpan::ZERO, TimeSpan::from_ms_i(60), false);
        assert_eq!(playback.transport(), Transport::Playing);
        assert!(wait_for(|| playback.take_finished()), "playback should finish");
        assert!(!playback.take_finished());
        assert_eq!(playback.transport(), Transport::Stopped);
        assert!(playback.stats().frames > 0);
    }

    #[test]
    fn a_retired_editors_render_cannot_replace_the_new_editors_schedule() {
        let (playback, mut first, node) = setup();
        let (_, mut second, _) = setup();
        playback.play(&mut first, TimeSpan::ZERO, TimeSpan::from_ms_i(10000), false);
        playback.play(&mut second, TimeSpan::ZERO, TimeSpan::from_ms_i(10000), false);
        let current = playback.engine.schedule().unwrap();
        first.add_new_effect(SET_LEVEL, node, "E", TimeSpan::ZERO, TimeSpan::from_ms_i(1000)).unwrap();
        first.wait_for_renders();
        assert!(Arc::ptr_eq(&current, &playback.engine.schedule().unwrap()));
        // A preview that failed to start is also unable to reclaim the playing editor.
        playback.attach_session(&mut first);
        assert!(Arc::ptr_eq(&current, &playback.engine.schedule().unwrap()));
    }

    #[test]
    fn named_context_control_reaches_all_copies_and_does_not_retain_finished_owners() {
        let (playback, session, _) = setup();
        let a = playback.start_named_context("Song", "Song.tim", session.schedule(), None, TimeSpan::from_ms_i(10000));
        let b = playback.start_named_context("Song", "Song.tim", session.schedule(), None, TimeSpan::from_ms_i(10000));
        let other = playback.start_live_context("State Preview", session.schedule());
        assert_eq!(playback.context_states().len(), 3);
        assert_eq!(playback.control_contexts("Song", Transport::Paused), 2);
        assert_eq!(a.transport(), Transport::Paused);
        assert_eq!(b.transport(), Transport::Paused);
        assert_eq!(other.transport(), Transport::Playing);
        assert_eq!(playback.control_contexts("Song", Transport::Playing), 2);
        assert_eq!(playback.control_contexts("Song", Transport::Stopped), 2);
        assert_eq!(playback.context_states().len(), 1);
        drop(other);
        assert!(playback.context_states().is_empty());
    }

    #[test]
    fn session_transport_and_commands_follow_identity_even_when_names_match() {
        let (playback, mut first, _) = setup();
        let (_, mut second, _) = setup();
        let a = first.playback_id();
        let b = second.playback_id();
        assert_eq!(first.name(), second.name());
        assert_ne!(a, b);
        playback.play(&mut first, TimeSpan::from_ms_i(1100), TimeSpan::from_ms_i(10_000), false);
        playback.pause_session(a);
        assert_eq!(playback.session_transport(a), Transport::Paused);
        assert!(playback.session_position(a) >= TimeSpan::from_ms_i(1100));
        assert_eq!(playback.session_transport(b), Transport::Stopped);
        assert_eq!(playback.session_position(b), TimeSpan::ZERO);

        playback.play(&mut second, TimeSpan::from_ms_i(2200), TimeSpan::from_ms_i(10_000), false);
        playback.pause_session(b);
        let at = playback.session_position(b);
        playback.resume_session(a);
        playback.seek_session(a, TimeSpan::ZERO);
        playback.set_session_speed(a, 4.0);
        playback.set_session_rate_mode(a, true);
        playback.stop_session(a);
        assert!(!playback.take_session_finished(a));
        assert_eq!(playback.session_transport(a), Transport::Stopped);
        assert_eq!(playback.session_position(a), TimeSpan::ZERO);
        assert_eq!(playback.session_transport(b), Transport::Paused);
        assert_eq!(playback.session_position(b), at);
        assert_eq!(*playback.speed.lock(), 1.0);
        assert!(!playback.preserve_pitch.load(Ordering::Relaxed));

        playback.play(&mut second, TimeSpan::ZERO, TimeSpan::from_ms_i(30), false);
        assert!(wait_for(|| playback.session_transport(b) == Transport::Stopped));
        assert!(!playback.take_session_finished(a), "old window must not consume the new owner's completion");
        assert!(playback.take_session_finished(b));
        assert!(!playback.take_session_finished(b));
    }

    #[test]
    fn scheduled_and_live_contexts_never_become_an_editors_playhead() {
        let (playback, mut session, _) = setup();
        let (_, other, _) = setup();
        let scheduled = playback.start_named_context("Untitled", "Untitled.tim", session.schedule(), None, TimeSpan::from_ms_i(10_000));
        let live = playback.start_live_context("State Preview", session.schedule());
        assert!(wait_for(|| scheduled.position() > TimeSpan::from_ms_i(20)));
        for id in [session.playback_id(), other.playback_id()] {
            assert_eq!(playback.session_transport(id), Transport::Stopped);
            assert_eq!(playback.session_position(id), TimeSpan::ZERO);
            playback.stop_session(id);
        }
        assert_eq!(scheduled.transport(), Transport::Playing);
        assert_eq!(live.transport(), Transport::Playing);
        playback.play(&mut session, TimeSpan::from_ms_i(2000), TimeSpan::from_ms_i(10_000), false);
        playback.stop_session(session.playback_id());
        assert_eq!(scheduled.transport(), Transport::Playing);
        assert_eq!(live.transport(), Transport::Playing);
        assert_eq!(playback.session_transport(other.playback_id()), Transport::Stopped);
    }

    #[test]
    fn stale_session_commands_cannot_race_a_replacement_start() {
        let (playback, mut old, _) = setup();
        let (_, mut replacement, _) = setup();
        let old_id = old.playback_id();
        playback.play(&mut old, TimeSpan::ZERO, TimeSpan::from_ms_i(10_000), false);
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let worker = {
            let playback = playback.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..200 {
                    playback.stop_session(old_id);
                    playback.seek_session(old_id, TimeSpan::ZERO);
                    playback.take_session_finished(old_id);
                }
            })
        };
        barrier.wait();
        playback.play(&mut replacement, TimeSpan::from_ms_i(2000), TimeSpan::from_ms_i(10_000), false);
        worker.join().unwrap();
        assert_eq!(playback.session_transport(replacement.playback_id()), Transport::Playing);
        assert!(playback.session_position(replacement.playback_id()) >= TimeSpan::from_ms_i(2000));
    }

    #[test]
    fn a_preview_claim_retires_the_previous_editor_owner() {
        let (playback, mut session, _) = setup();
        let id = session.playback_id();
        playback.play(&mut session, TimeSpan::ZERO, TimeSpan::from_ms_i(10_000), false);
        assert!(playback.begin_effect_preview(session.schedule()).is_none());
        assert!(playback.owns_session(id), "failed preview must leave the editor owner intact");
        playback.stop_session(id);
        let claim = playback.begin_effect_preview(session.schedule()).unwrap();
        assert!(!playback.owns_session(id));
        playback.set_session_speed(id, 4.0);
        playback.set_session_rate_mode(id, true);
        playback.seek_session(id, TimeSpan::from_ms_i(2000));
        playback.stop_session(id);
        assert!(!playback.take_session_finished(id));
        assert!(playback.effect_preview_is_playing(claim));
        assert!(playback.engine.position() < TimeSpan::from_ms_i(2000));
        assert_eq!(*playback.speed.lock(), 1.0);
        assert!(!playback.preserve_pitch.load(Ordering::Relaxed));
        playback.end_effect_preview(claim);
    }

    #[test]
    fn pause_resume_and_stop() {
        let (playback, mut session, _) = setup();
        playback.play(&mut session, TimeSpan::ZERO, TimeSpan::from_ms_i(10_000), false);
        assert!(wait_for(|| playback.position() > TimeSpan::from_ms_i(20)));
        playback.pause();
        assert_eq!(playback.transport(), Transport::Paused);
        let at = playback.position();
        std::thread::sleep(Duration::from_millis(40));
        assert!(playback.position().total_ms() - at.total_ms() < 15.0, "time holds while paused");
        playback.resume();
        assert!(wait_for(|| playback.position() > at + TimeSpan::from_ms_i(20)));
        playback.stop();
        assert_eq!(playback.transport(), Transport::Stopped);
    }

    #[test]
    fn execution_state_follows_open_and_close_without_touching_hardware_when_muted() {
        let (playback, ..) = setup();
        assert_eq!(playback.execution_state(), ExecutionState::Closed);
        playback.open_execution();
        assert_eq!(playback.execution_state().text(), "Open");
        playback.close_execution();
        assert_eq!(playback.execution_state().text(), "Closed");
    }

    #[test]
    fn range_is_clamped_to_the_sequence_length() {
        let (playback, mut session, _) = setup();
        session.set_length(TimeSpan::from_ms_i(30));
        playback.play(&mut session, TimeSpan::from_ms_i(10), TimeSpan::from_ms_i(i64::MAX / 100_000), false);
        assert!(wait_for(|| playback.take_finished()));
        assert_eq!(playback.transport(), Transport::Stopped);
    }
}
