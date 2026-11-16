//! Port of `HardwareUpdateThread` and `OutputDeviceExecution`: each enabled controller gets its own thread, which sends the
//! latest frame at the controller's update interval.
//!
//! The engine thread only ever copies a frame into a mailbox, which takes a lock for as long as that copy. The controller's
//! thread holds the lock just long enough to swap buffers, and does its writes with no lock held, so a slow serial port can
//! fall behind without the engine noticing. Frames it had no time for are simply replaced by newer ones.

use crate::command::OutputCommand;
use crate::graph::OutputGraph;
use crate::modules::{
    create_module, ConfigMismatch, ControllerConfig, DummyLightingView, E131Counters, E131Stats, ModuleShared, ModuleStatus,
    OutputModule,
};
use parking_lot::{Condvar, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// `HardwareUpdateThread.STOP_TIMEOUT`.
const STOP_TIMEOUT: Duration = Duration::from_secs(4);

/// `SystemConfig.DEFAULT_UPDATE_INTERVAL`, for a file with none saved.
pub const DEFAULT_UPDATE_INTERVAL: Duration = Duration::from_millis(50);

/// The application supplies its asynchronous log writer without making output depend on the session crate.
/// Arguments are the upstream logger, message, and optional exception detail. Set before starting controllers.
pub type ControllerLogSink = Arc<dyn Fn(&str, &str, Option<&str>) + Send + Sync>;

const DEVICE_LOGGER: &str = "Vixen.Sys.Managers.OutputDeviceExecution";
const THREAD_LOGGER: &str = "Vixen.Sys.Managers.HardwareUpdateThread";
const OUTPUT_LOGGER: &str = "Vixen.Sys.OutputController";

fn report(sink: &Option<ControllerLogSink>, logger: &str, message: &str, detail: Option<&str>) {
    if let Some(sink) = sink {
        // A faulty diagnostic callback must not take down a controller or its caller.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sink(logger, message, detail)));
    }
}

fn panic_detail(payload: Box<dyn std::any::Any + Send>) -> String {
    payload.downcast_ref::<String>().cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_else(|| "Controller code panicked".into())
}

fn attach_module_log(shared: &ModuleShared, name: &str, sink: &Option<ControllerLogSink>) {
    let name = name.to_string();
    let sink = sink.clone();
    shared.set_error_sink(sink.as_ref().map(|_| {
        let sink = sink.clone();
        Arc::new(move |detail: &str| {
            report(&sink, OUTPUT_LOGGER, &format!("An error occurred outputting data for controller {name}"), Some(detail));
        }) as Arc<dyn Fn(&str) + Send + Sync>
    }));
}

/// Holds the newest frame for one controller.
#[derive(Debug, Default)]
pub struct Mailbox {
    inner: Mutex<MailboxInner>,
}

#[derive(Debug, Default)]
struct MailboxInner {
    frame: Vec<OutputCommand>,
    generation: u64,
    taken: u64,
}

impl Mailbox {
    pub fn new(outputs: usize) -> Mailbox {
        Mailbox { inner: Mutex::new(MailboxInner { frame: vec![OutputCommand::None; outputs], generation: 0, taken: 0 }) }
    }

    /// Replaces whatever is waiting with this frame. Doesn't allocate once the size is settled.
    #[inline]
    pub fn publish(&self, commands: &[OutputCommand]) {
        let mut inner = self.inner.lock();
        if inner.frame.len() == commands.len() {
            inner.frame.clone_from_slice(commands);
        } else {
            inner.frame.clear();
            inner.frame.extend_from_slice(commands);
        }
        inner.generation += 1;
    }

    /// Swaps the newest frame into `dest`. Returns `false`, leaving `dest` alone, when nothing new has come in since last time.
    #[inline]
    pub fn take_latest(&self, dest: &mut Vec<OutputCommand>) -> bool {
        self.take_latest_with_replaced(dest).is_some()
    }

    /// Counts overwritten frames in the same lock as the swap. `taken` survives worker restarts,
    /// and a later publication cannot change the count for the frame just consumed.
    #[inline]
    fn take_latest_with_replaced(&self, dest: &mut Vec<OutputCommand>) -> Option<u64> {
        let mut inner = self.inner.lock();
        if inner.generation == inner.taken {
            return None;
        }
        let replaced = inner.generation.saturating_sub(inner.taken).saturating_sub(1);
        inner.taken = inner.generation;
        std::mem::swap(&mut inner.frame, dest);
        Some(replaced)
    }

    /// How many frames have been published.
    pub fn generation(&self) -> u64 {
        self.inner.lock().generation
    }
}

/// Timing and counts for the Instrumentation window: the values `HardwareUpdateThread` and `OutputController` register.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ControllerStats {
    /// `OutputDeviceRefreshRateValue`'s count: calls to `update`.
    pub updates: u64,
    /// `"{0} Output Total Update"`.
    pub last_update_us: u64,
    pub max_update_us: u64,
    /// `"{0} delta"`: how far the last cycle was from the update interval.
    pub last_interval_delta_ms: u64,
    /// `"{0} sleep time"`.
    pub last_sleep_ms: u64,
    /// Frames the engine published that were replaced before this controller could send them.
    pub frames_replaced: u64,
    /// Errors the module recorded.
    pub errors: u64,
    /// Packets or serial frames put on the wire.
    pub packets_sent: u64,
    pub bytes_sent: u64,
    /// Panics caught in `update`.
    pub panics: u64,
}

/// A measurement since it was last reset: minimum, maximum and total in milliseconds, and how many.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TimingSummary {
    pub min_ms: f64,
    pub max_ms: f64,
    pub total_ms: f64,
    pub samples: u64,
}

/// The rows the Instrumentation window shows for a running controller.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ControllerTimings {
    /// `"{0} Output Total Update"`.
    pub update: TimingSummary,
    /// `"{0} delta"`.
    pub delta: TimingSummary,
    /// `"{0} sleep time"`.
    pub sleep: TimingSummary,
}

/// One thread writes this. A reset from another can lose a sample, which a statistics display doesn't care about.
#[derive(Debug, Default)]
struct Timing {
    min_us: AtomicU64,
    max_us: AtomicU64,
    total_us: AtomicU64,
    samples: AtomicU64,
}

impl Timing {
    fn add(&self, us: u64) {
        if self.samples.fetch_add(1, Ordering::Relaxed) == 0 {
            self.min_us.store(us, Ordering::Relaxed);
        } else {
            self.min_us.fetch_min(us, Ordering::Relaxed);
        }
        self.max_us.fetch_max(us, Ordering::Relaxed);
        self.total_us.fetch_add(us, Ordering::Relaxed);
    }

    fn summary(&self) -> TimingSummary {
        let ms = |slot: &AtomicU64| slot.load(Ordering::Relaxed) as f64 / 1000.0;
        TimingSummary {
            min_ms: ms(&self.min_us),
            max_ms: ms(&self.max_us),
            total_ms: ms(&self.total_us),
            samples: self.samples.load(Ordering::Relaxed),
        }
    }

    fn reset(&self) {
        self.samples.store(0, Ordering::Relaxed);
        self.min_us.store(0, Ordering::Relaxed);
        self.max_us.store(0, Ordering::Relaxed);
        self.total_us.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
struct Counters {
    updates: AtomicU64,
    update_timing: Timing,
    delta_timing: Timing,
    sleep_timing: Timing,
    last_update_us: AtomicU64,
    max_update_us: AtomicU64,
    last_interval_delta_ms: AtomicU64,
    last_sleep_ms: AtomicU64,
    frames_replaced: AtomicU64,
    panics: AtomicU64,
}

/// What the runner and a controller's thread share.
#[derive(Debug)]
struct Control {
    state: Mutex<ThreadState>,
    wake: Condvar,
    interval_ms: AtomicU64,
    running: AtomicBool,
}

#[derive(Debug, Default)]
struct ThreadState {
    stop: bool,
    pause: bool,
    finished: bool,
    /// A new configuration for the running module.
    pending_config: Option<ControllerConfig>,
    config_requested: bool,
    config_reply: Option<ControllerConfig>,
}

struct Entry {
    id: Uuid,
    name: String,
    enabled: bool,
    /// The module's output count, which can't be asked while its thread has it.
    outputs: usize,
    mailbox: Arc<Mailbox>,
    control: Arc<Control>,
    shared: Arc<ModuleShared>,
    counters: Arc<Counters>,
    /// `None` while its thread has it.
    module: Option<Box<dyn OutputModule>>,
    thread: Option<JoinHandle<Box<dyn OutputModule>>>,
    type_id: Uuid,
    e131_counters: Option<Arc<E131Counters>>,
    dummy_view: Option<Arc<Mutex<DummyLightingView>>>,
    debug_frame: Option<Arc<Mutex<Vec<OutputCommand>>>>,
}

/// Runs a show's controllers. Add them, `start_all`, then `publish` a frame after each `OutputGraph::evaluate`.
pub struct ControllerRunner {
    entries: Vec<Entry>,
    interval: Duration,
    log_sink: Option<ControllerLogSink>,
}

impl ControllerRunner {
    /// `interval` is `VixenSystem.DefaultUpdateInterval`, which every controller descriptor uses.
    pub fn new(interval: Duration) -> ControllerRunner {
        ControllerRunner { entries: Vec::new(), interval, log_sink: None }
    }

    pub fn set_log_sink(&mut self, sink: ControllerLogSink) {
        self.log_sink = Some(sink);
        for entry in &self.entries { attach_module_log(&entry.shared, &entry.name, &self.log_sink); }
    }

    /// Creates a module for every controller in the graph that has one, in graph order. `disabled` holds the
    /// `DisabledDevices` ids from `SystemConfig.xml`.
    pub fn from_graph(graph: &OutputGraph, disabled: &[Uuid], interval: Duration) -> ControllerRunner {
        let mut runner = ControllerRunner::new(interval);
        for controller in graph.controllers() {
            if let Some(config) = &controller.config {
                let mut module = create_module(config.clone());
                module.set_output_count(controller.output_count());
                runner.add_controller(controller.id, &controller.name, module, !disabled.contains(&controller.id));
            }
        }
        runner
    }

    /// Adds a controller. A disabled one stays in the model but never starts. Returns its index.
    pub fn add_controller(&mut self, id: Uuid, name: &str, module: Box<dyn OutputModule>, enabled: bool) -> usize {
        let outputs = module.output_count();
        attach_module_log(module.shared(), name, &self.log_sink);
        self.entries.push(Entry {
            id,
            name: name.to_string(),
            enabled,
            outputs,
            mailbox: Arc::new(Mailbox::new(outputs)),
            control: Arc::new(Control {
                state: Mutex::new(ThreadState::default()),
                wake: Condvar::new(),
                interval_ms: AtomicU64::new(self.interval.as_millis() as u64),
                running: AtomicBool::new(false),
            }),
            shared: module.shared().clone(),
            counters: Arc::new(Counters::default()),
            type_id: module.type_id(),
            e131_counters: module.e131_counters(),
            dummy_view: module.dummy_view(),
            debug_frame: module.debug_frame(),
            module: Some(module),
            thread: None,
        });
        self.entries.len() - 1
    }

    /// Brings the runner into line with a rebuilt graph, for Display Setup's OK. A controller that's still there keeps running on its thread unless its configuration or
    /// output count changed, which restarts just that one. `start` says whether the engine is open, so that a newly enabled controller should start now.
    pub fn reconcile(&mut self, graph: &OutputGraph, disabled: &[Uuid], start: bool) {
        for i in (0..self.entries.len()).rev() {
            if graph.controller_index(&self.entries[i].id).is_none() {
                self.stop(i);
                self.entries.remove(i);
            }
        }
        for controller in graph.controllers() {
            let Some(config) = &controller.config else { continue };
            let enabled = !disabled.contains(&controller.id);
            let index = match self.controller_index(&controller.id) {
                Some(i) => {
                    let running = self.is_running(i);
                    self.entries[i].name.clone_from(&controller.name);
                    attach_module_log(&self.entries[i].shared, &controller.name, &self.log_sink);
                    let same_outputs = self.entries[i].outputs == controller.output_count();
                    if !same_outputs || self.config(i).as_ref() != Some(config) {
                        self.stop(i);
                        if let Some(module) = self.entries[i].module.as_mut() {
                            let _ = module.set_config(config.clone());
                            module.set_output_count(controller.output_count());
                        }
                        self.entries[i].outputs = controller.output_count();
                        if running && enabled {
                            self.start(i);
                        }
                    }
                    i
                }
                None => {
                    let mut module = create_module(config.clone());
                    module.set_output_count(controller.output_count());
                    self.add_controller(controller.id, &controller.name, module, enabled)
                }
            };
            self.set_enabled(index, enabled);
            if enabled && start {
                self.start(index);
            }
        }
        // `publish` finds a controller by position first, so keep the graph's order.
        self.entries.sort_by_key(|e| graph.controller_index(&e.id).unwrap_or(usize::MAX));
    }

    /// How many controllers it has, running or not.
    pub fn controller_count(&self) -> usize {
        self.entries.len()
    }

    pub fn controller_id(&self, index: usize) -> Option<Uuid> {
        self.entries.get(index).map(|e| e.id)
    }

    pub fn controller_name(&self, index: usize) -> Option<String> {
        self.entries.get(index).map(|e| e.name.clone())
    }

    pub fn controller_index(&self, id: &Uuid) -> Option<usize> {
        self.entries.iter().position(|e| e.id == *id)
    }

    /// `OutputController.UpdateInterval`'s setter, for one controller. It takes effect on the next cycle.
    pub fn set_update_interval(&self, index: usize, interval: Duration) {
        if let Some(e) = self.entries.get(index) {
            e.control.interval_ms.store(interval.as_millis() as u64, Ordering::Relaxed);
        }
    }

    /// Whether a controller is allowed to run.
    pub fn set_enabled(&mut self, index: usize, enabled: bool) {
        if !enabled {
            self.stop(index);
        }
        if let Some(e) = self.entries.get_mut(index) {
            e.enabled = enabled;
        }
    }

    pub fn is_enabled(&self, index: usize) -> bool {
        self.entries.get(index).is_some_and(|e| e.enabled)
    }

    pub fn is_running(&self, index: usize) -> bool {
        self.entries.get(index).is_some_and(|e| e.control.running.load(Ordering::Relaxed))
    }

    pub fn is_paused(&self, index: usize) -> bool {
        self.entries.get(index).is_some_and(|e| e.control.running.load(Ordering::Relaxed) && e.control.state.lock().pause)
    }

    /// Starts every enabled controller.
    pub fn start_all(&mut self) {
        for i in 0..self.entries.len() {
            if self.entries[i].enabled {
                self.start(i);
            }
        }
    }

    /// Starts one controller's thread. `module.start` runs there too, so a slow DNS lookup or port open only delays that controller.
    pub fn start(&mut self, index: usize) {
        // A failed worker still owns the module in its completed JoinHandle. The UI's Start command must recover it
        // directly; requiring a hidden Stop first leaves a visibly stopped controller impossible to restart.
        if self.entries.get(index).is_some_and(|e| e.thread.is_some() && e.control.state.lock().finished) {
            self.stop(index);
        }
        let Some(e) = self.entries.get_mut(index) else {
            return;
        };
        if e.thread.is_some() {
            return;
        }
        let Some(module) = e.module.take() else {
            return;
        };
        *e.control.state.lock() = ThreadState::default();
        e.shared.request_stop(false);
        e.control.running.store(true, Ordering::Relaxed);
        let (control, mailbox, counters) = (e.control.clone(), e.mailbox.clone(), e.counters.clone());
        let name = e.name.clone();
        let log_sink = self.log_sink.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("{} update", e.name))
            .spawn(move || controller_thread(module, control, mailbox, counters, name, log_sink));
        match spawned {
            Ok(handle) => e.thread = Some(handle),
            Err(err) => {
                e.control.running.store(false, Ordering::Relaxed);
                e.shared.record_error_without_log(format!("Couldn't start the controller's thread: {err}"));
                e.shared.set_status(ModuleStatus::Error(format!("Couldn't start the controller's thread: {err}")));
                report(&self.log_sink, DEVICE_LOGGER, &format!("Error starting device {}", e.name), Some(&err.to_string()));
            }
        }
    }

    /// Stops every controller and waits for each thread, up to four seconds each.
    pub fn stop_all(&mut self) {
        for e in &self.entries {
            e.request_stop();
        }
        for i in 0..self.entries.len() {
            self.stop(i);
        }
    }

    /// `HardwareUpdateThread.Stop` and `WaitForFinish`. A controller that doesn't finish in time is left to end alone, and is
    /// reported the way the C#'s timeout exception is logged.
    pub fn stop(&mut self, index: usize) {
        let Some(e) = self.entries.get_mut(index) else {
            return;
        };
        e.request_stop();
        let Some(handle) = e.thread.take() else {
            return;
        };
        let finished = {
            let mut state = e.control.state.lock();
            let deadline = Instant::now() + STOP_TIMEOUT;
            while !state.finished {
                if e.control.wake.wait_until(&mut state, deadline).timed_out() {
                    break;
                }
            }
            state.finished
        };
        e.control.running.store(false, Ordering::Relaxed);
        if finished {
            match handle.join() {
                Ok(module) => e.module = Some(module),
                Err(payload) => {
                    let detail = panic_detail(payload);
                    e.shared.record_error_without_log(detail.clone());
                    e.shared.set_status(ModuleStatus::Error(detail.clone()));
                    report(&self.log_sink, DEVICE_LOGGER, &format!("Error trying to stop device {}", e.name), Some(&detail));
                }
            }
        } else {
            let message = format!("Controller {} failed to stop in the required amount of time.", e.name);
            e.shared.record_error_without_log(message.clone());
            e.shared.set_status(ModuleStatus::Error(message.clone()));
            report(&self.log_sink, DEVICE_LOGGER, &format!("Error trying to stop device {}", e.name), Some(&message));
        }
    }

    /// `HardwareUpdateThread.Pause`: the thread finishes its cycle and then waits.
    pub fn pause_all(&self) {
        for e in &self.entries {
            e.set_paused(true);
        }
    }

    pub fn resume_all(&self) {
        for e in &self.entries {
            e.set_paused(false);
        }
    }

    /// Pauses or resumes one controller, as while its setup dialog is open (F51). A stopped one isn't affected.
    pub fn set_paused(&self, index: usize, paused: bool) {
        if let Some(e) = self.entries.get(index) {
            if e.control.running.load(Ordering::Relaxed) {
                e.set_paused(paused);
            }
        }
    }

    /// Hands every running controller its commands from the graph. Call on the engine thread after `evaluate`. Doesn't block on
    /// any controller's I/O, and allocates nothing.
    ///
    /// Controllers are matched to the graph by position when `from_graph` built this runner, and otherwise by id.
    pub fn publish(&self, graph: &OutputGraph) {
        let controllers = graph.controllers();
        for (i, e) in self.entries.iter().enumerate() {
            if !e.control.running.load(Ordering::Relaxed) {
                continue;
            }
            let controller = match controllers.get(i) {
                Some(c) if c.id == e.id => Some(c),
                _ => controllers.iter().find(|c| c.id == e.id),
            };
            if let Some(controller) = controller {
                e.mailbox.publish(controller.commands());
            }
        }
    }

    /// Hands one controller its commands.
    pub fn publish_controller(&self, index: usize, commands: &[OutputCommand]) {
        if let Some(e) = self.entries.get(index) {
            e.mailbox.publish(commands);
        }
    }

    pub fn status(&self, index: usize) -> ModuleStatus {
        self.entries.get(index).map_or(ModuleStatus::Stopped, |e| e.shared.status())
    }

    pub fn shared(&self, index: usize) -> Option<&Arc<ModuleShared>> {
        self.entries.get(index).map(|e| &e.shared)
    }

    pub fn stats(&self, index: usize) -> ControllerStats {
        let Some(e) = self.entries.get(index) else {
            return ControllerStats::default();
        };
        let c = &e.counters;
        ControllerStats {
            updates: c.updates.load(Ordering::Relaxed),
            last_update_us: c.last_update_us.load(Ordering::Relaxed),
            max_update_us: c.max_update_us.load(Ordering::Relaxed),
            last_interval_delta_ms: c.last_interval_delta_ms.load(Ordering::Relaxed),
            last_sleep_ms: c.last_sleep_ms.load(Ordering::Relaxed),
            frames_replaced: c.frames_replaced.load(Ordering::Relaxed),
            errors: e.shared.error_count(),
            packets_sent: e.shared.packets_sent(),
            bytes_sent: e.shared.bytes_sent(),
            panics: c.panics.load(Ordering::Relaxed),
        }
    }

    /// The Instrumentation window's numbers for a controller, since it started or was last reset.
    pub fn timings(&self, index: usize) -> ControllerTimings {
        let Some(e) = self.entries.get(index) else {
            return ControllerTimings::default();
        };
        let c = &e.counters;
        ControllerTimings {
            update: c.update_timing.summary(),
            delta: c.delta_timing.summary(),
            sleep: c.sleep_timing.summary(),
        }
    }

    /// The Instrumentation window's Reset button.
    pub fn reset_timings(&self, index: usize) {
        if let Some(e) = self.entries.get(index) {
            e.counters.update_timing.reset();
            e.counters.delta_timing.reset();
            e.counters.sleep_timing.reset();
        }
    }

    /// A stopped controller's module, for reading its configuration back or changing it.
    pub fn module(&mut self, index: usize) -> Option<&mut Box<dyn OutputModule>> {
        self.entries.get_mut(index).and_then(|e| e.module.as_mut())
    }

    /// A copy of a controller's configuration, including what the module has changed in it, such as E1.31 sequence numbers.
    /// A running controller answers between frames. `None` if it doesn't within a second, which means its hardware is holding it
    /// up, or if there's no such controller.
    pub fn config(&self, index: usize) -> Option<ControllerConfig> {
        let e = self.entries.get(index)?;
        if let Some(module) = &e.module {
            return Some(module.config());
        }
        let mut state = e.control.state.lock();
        if state.finished || state.stop {
            return None;
        }
        state.config_reply = None;
        state.config_requested = true;
        e.control.wake.notify_all();
        let deadline = Instant::now() + Duration::from_secs(1);
        while state.config_reply.is_none() && !state.finished && !state.stop {
            if e.control.wake.wait_until(&mut state, deadline).timed_out() {
                break;
            }
        }
        state.config_reply.take()
    }

    /// `Setup()` finishing with OK: gives a controller a new configuration. A running one stops, takes it and starts again between
    /// frames, as the C# setup dialogs make it do. Remember [`OutputGraph::set_controller_config`] for its data policy.
    pub fn set_config(&mut self, index: usize, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let Some(e) = self.entries.get_mut(index) else {
            return Err(ConfigMismatch);
        };
        if let Some(module) = e.module.as_mut() {
            return module.set_config(config);
        }
        if config.ids().type_id != e.type_id {
            return Err(ConfigMismatch);
        }
        let mut state = e.control.state.lock();
        state.pending_config = Some(config);
        e.control.wake.notify_all();
        Ok(())
    }

    /// The statistics an E1.31 controller shows when it stops. `None` on any other module.
    pub fn e131_stats(&self, index: usize) -> Option<E131Stats> {
        self.entries.get(index)?.e131_counters.as_ref().map(|c| c.snapshot())
    }

    /// What a dummy controller's window shows. `None` on any other module.
    pub fn dummy_view(&self, index: usize) -> Option<Arc<Mutex<DummyLightingView>>> {
        self.entries.get(index)?.dummy_view.clone()
    }

    /// A debug controller's latest frame. `None` on any other module.
    pub fn debug_frame(&self, index: usize) -> Option<Arc<Mutex<Vec<OutputCommand>>>> {
        self.entries.get(index)?.debug_frame.clone()
    }
}

impl Drop for ControllerRunner {
    fn drop(&mut self) {
        self.stop_all();
    }
}

impl Entry {
    fn request_stop(&self) {
        self.shared.request_stop(true);
        let mut state = self.control.state.lock();
        state.stop = true;
        self.control.wake.notify_all();
    }

    fn set_paused(&self, paused: bool) {
        let mut state = self.control.state.lock();
        state.pause = paused;
        self.control.wake.notify_all();
    }
}

fn set_thread_priority() {
    // The highest class macOS has without special privileges, as for the engine's frame thread.
    // SAFETY: only changes the calling thread's own QoS class.
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
    }
}

/// `HardwareUpdateThread._ThreadFunc`, with `IntervalUpdateSignaler`.
fn controller_thread(
    mut module: Box<dyn OutputModule>,
    control: Arc<Control>,
    mailbox: Arc<Mailbox>,
    counters: Arc<Counters>,
    name: String,
    sink: Option<ControllerLogSink>,
) -> Box<dyn OutputModule> {
    set_thread_priority();
    let shared = module.shared().clone();
    let diagnostics = Diagnostics { name: &name, sink: &sink, shared: &shared, counters: &counters };
    let mut execution_failed = false;
    let failure = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        controller_loop(module.as_mut(), &control, &mailbox, &counters, &diagnostics)
    })) {
        Ok(failure) => failure,
        Err(payload) => {
            execution_failed = true;
            counters.panics.fetch_add(1, Ordering::Relaxed);
            let detail = panic_detail(payload);
            shared.record_error_without_log(detail.clone());
            report(&sink, THREAD_LOGGER, &format!("Controller {name} error"), Some(&detail));
            Some(detail)
        }
    };
    if !diagnostics.guarded(DEVICE_LOGGER, || format!("Error trying to stop device {name}"), || module.stop()) {
        shared.set_status(ModuleStatus::Error(shared.last_error().unwrap_or_default()));
    }
    if let Some(failure) = failure {
        shared.set_status(ModuleStatus::Error(failure));
    }
    let mut state = control.state.lock();
    state.finished = true;
    control.running.store(false, Ordering::Relaxed);
    control.wake.notify_all();
    drop(state);
    if execution_failed {
        report(&sink, DEVICE_LOGGER,
            &format!("Device {name} experienced an error during execution and was shutdown."), None);
    }
    module
}

fn controller_loop(
    module: &mut dyn OutputModule,
    control: &Control,
    mailbox: &Mailbox,
    counters: &Counters,
    diagnostics: &Diagnostics<'_>,
) -> Option<String> {
    let name = diagnostics.name;
    if !diagnostics.guarded(DEVICE_LOGGER, || format!("Error starting device {name}"), || module.start()) {
        // A failed start must not proceed into update, as HardwareUpdateThread.Start never starts its thread after a throw.
        return diagnostics.shared.last_error();
    }

    let clock = Instant::now();
    let mut frame = vec![OutputCommand::None; module.output_count()];
    let mut next_update_ms: u64 = 0;
    let mut last_ms: u64 = 0;

    loop {
        let (stop, pending, requested) = {
            let mut state = control.state.lock();
            (state.stop, state.pending_config.take(), std::mem::take(&mut state.config_requested))
        };
        if stop {
            break;
        }
        if let Some(config) = pending {
            if !diagnostics.guarded(DEVICE_LOGGER, || format!("Error starting device {name}"), || {
                module.stop();
                let _ = module.set_config(config);
                module.start();
            }) {
                return diagnostics.shared.last_error();
            }
        }
        if requested {
            let config = module.config();
            let mut state = control.state.lock();
            state.config_reply = Some(config);
            control.wake.notify_all();
        }

        let now_ms = clock.elapsed().as_millis() as u64;
        let interval = control.interval_ms.load(Ordering::Relaxed);
        let delta_ms = interval.abs_diff(now_ms - last_ms);
        counters.last_interval_delta_ms.store(delta_ms, Ordering::Relaxed);
        counters.delta_timing.add(delta_ms * 1000);
        last_ms = now_ms;

        if let Some(replaced) = mailbox.take_latest_with_replaced(&mut frame) {
            counters.frames_replaced.fetch_add(replaced, Ordering::Relaxed);
        }
        let update_started = Instant::now();
        diagnostics.guarded(OUTPUT_LOGGER, || format!("An error occurred outputting data for controller {name}"), || module.update(&frame));
        let update_us = update_started.elapsed().as_micros() as u64;
        counters.last_update_us.store(update_us, Ordering::Relaxed);
        counters.max_update_us.fetch_max(update_us, Ordering::Relaxed);
        counters.updates.fetch_add(1, Ordering::Relaxed);
        counters.update_timing.add(update_us);

        // `IntervalUpdateSignaler.RaiseSignal`: the schedule doesn't drift, and a cycle that ran long is made up by not
        // sleeping until it's caught up. Sleeping is skipped below 2 ms, as it is there.
        let before_sleep = clock.elapsed().as_millis() as u64;
        if next_update_ms == 0 {
            next_update_ms = before_sleep + interval;
        }
        let remaining = next_update_ms.saturating_sub(before_sleep);
        if remaining > 1 {
            let deadline = Instant::now() + Duration::from_millis(remaining);
            let mut state = control.state.lock();
            while !state.stop {
                if control.wake.wait_until(&mut state, deadline).timed_out() {
                    break;
                }
            }
        }
        next_update_ms += interval;
        let sleep_ms = clock.elapsed().as_millis() as u64 - before_sleep;
        counters.last_sleep_ms.store(sleep_ms, Ordering::Relaxed);
        counters.sleep_timing.add(sleep_ms * 1000);

        // `_WaitOnPause`.
        let paused = control.state.lock().pause;
        if paused {
            diagnostics.guarded(DEVICE_LOGGER, || format!("Error trying to pause device {name}"), || module.pause());
            let mut state = control.state.lock();
            while state.pause && !state.stop {
                control.wake.wait(&mut state);
            }
            let stopping = state.stop;
            drop(state);
            if !stopping {
                // The signaler's clock kept running, so after a long pause it sends without sleeping until it has caught up, as in
                // the C#.
                diagnostics.guarded(DEVICE_LOGGER, || format!("Error trying to resume device {name}"), || module.resume());
            }
        }
    }

    None
}

struct Diagnostics<'a> {
    name: &'a str,
    sink: &'a Option<ControllerLogSink>,
    shared: &'a ModuleShared,
    counters: &'a Counters,
}

impl Diagnostics<'_> {
    /// Update failures lose only one frame, as OutputController.Update's catch does. Ordinary module I/O errors
    /// report directly through ModuleShared, including those raised on driver-owned threads between updates.
    fn guarded(&self, logger: &str, message: impl FnOnce() -> String, f: impl FnOnce()) -> bool {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => true,
            Err(payload) => {
                self.counters.panics.fetch_add(1, Ordering::Relaxed);
                let detail = panic_detail(payload);
                self.shared.record_error_without_log(detail.clone());
                report(self.sink, logger, &message(), Some(&detail));
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailbox_keeps_only_the_newest_frame() {
        let m = Mailbox::new(2);
        let mut dest = vec![OutputCommand::None; 2];
        assert!(!m.take_latest(&mut dest));
        m.publish(&[OutputCommand::U8(1), OutputCommand::U8(2)]);
        m.publish(&[OutputCommand::U8(3), OutputCommand::U8(4)]);
        assert!(m.take_latest(&mut dest));
        assert_eq!(dest, vec![OutputCommand::U8(3), OutputCommand::U8(4)]);
        assert!(!m.take_latest(&mut dest));
        assert_eq!(dest, vec![OutputCommand::U8(3), OutputCommand::U8(4)]);
    }

    #[test]
    fn mailbox_adapts_to_a_new_size() {
        let m = Mailbox::new(1);
        let mut dest = Vec::new();
        m.publish(&[OutputCommand::U8(1), OutputCommand::U8(2)]);
        assert!(m.take_latest(&mut dest));
        assert_eq!(dest.len(), 2);
        assert_eq!(m.generation(), 1);
    }

    #[test]
    fn mailbox_counts_only_frames_overwritten_before_the_take() {
        let m = Mailbox::new(1);
        let mut dest = vec![OutputCommand::None];
        assert_eq!(m.take_latest_with_replaced(&mut dest), None);
        m.publish(&[OutputCommand::U8(1)]);
        m.publish(&[OutputCommand::U8(2)]);
        m.publish(&[OutputCommand::U8(3)]);
        assert_eq!(m.take_latest_with_replaced(&mut dest), Some(2));
        assert_eq!(dest[0], OutputCommand::U8(3));
        assert_eq!(m.take_latest_with_replaced(&mut dest), None);
        m.publish(&[OutputCommand::U8(4)]);
        assert!(m.take_latest(&mut dest));
        m.publish(&[OutputCommand::U8(5)]);
        assert_eq!(m.take_latest_with_replaced(&mut dest), Some(0));
        assert_eq!(dest[0], OutputCommand::U8(5));
    }

    #[test]
    fn mailbox_publication_after_take_cannot_change_its_replacement_count() {
        let m = Mailbox::new(1);
        let mut dest = vec![OutputCommand::None];
        m.publish(&[OutputCommand::U8(1)]);
        let first = m.take_latest_with_replaced(&mut dest).unwrap();
        assert_eq!(dest[0], OutputCommand::U8(1));
        // Producer publishes before the consumer accounts for its completed take.
        m.publish(&[OutputCommand::U8(2)]);
        let second = m.take_latest_with_replaced(&mut dest).unwrap();
        assert_eq!(dest[0], OutputCommand::U8(2));
        assert_eq!((first, second), (0, 0));
    }
}
