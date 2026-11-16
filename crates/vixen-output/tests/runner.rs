//! `ControllerRunner`: one thread per enabled controller, a latest-frame mailbox, and the interval schedule.

mod common;

use common::{Pty, UdpReceiver};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use vixen_core::{CombinedState, RgbValue};
use vixen_output::graph::{ControllerDef, GraphDefinition, NoModuleData, OutputDef, OutputGraph, PatchDef};
use vixen_output::modules::e131::{packet, E131Config, E131Module, UniverseEntry};
use vixen_output::modules::generic_serial::{GenericSerialConfig, GenericSerialModule};
use vixen_output::modules::{ConfigMismatch, DebugConfig, ModuleShared, OpenDmxConfig, OpenDmxModule};
use vixen_output::runner::Mailbox;
use vixen_output::serial::{Parity, SerialSettings, StopBits};
use vixen_output::{ids, ControllerConfig, ControllerRunner, ModuleIds, ModuleStatus, OutputCommand, OutputModule};

/// A controller whose `update` takes as long as the test says, and records every frame it sees.
struct SlowModule {
    shared: Arc<ModuleShared>,
    delay: Duration,
    seen: Arc<Mutex<Vec<u8>>>,
    panic_next: Arc<AtomicBool>,
    started: Arc<AtomicUsize>,
    stopped: Arc<AtomicUsize>,
    start_thread: Arc<Mutex<Option<std::thread::ThreadId>>>,
    faults: Arc<Mutex<Vec<&'static str>>>,
}

#[derive(Clone, Default)]
struct Probe {
    seen: Arc<Mutex<Vec<u8>>>,
    panic_next: Arc<AtomicBool>,
    started: Arc<AtomicUsize>,
    stopped: Arc<AtomicUsize>,
    start_thread: Arc<Mutex<Option<std::thread::ThreadId>>>,
    faults: Arc<Mutex<Vec<&'static str>>>,
}

impl SlowModule {
    fn new(delay: Duration) -> (SlowModule, Probe) {
        let probe = Probe::default();
        let module = SlowModule {
            shared: ModuleShared::new(),
            delay,
            seen: probe.seen.clone(),
            panic_next: probe.panic_next.clone(),
            started: probe.started.clone(),
            stopped: probe.stopped.clone(),
            start_thread: probe.start_thread.clone(),
            faults: probe.faults.clone(),
        };
        (module, probe)
    }

    fn fault(&self, operation: &'static str) {
        let fail = {
            let mut faults = self.faults.lock();
            faults.iter().position(|name| *name == operation).map(|i| faults.remove(i)).is_some()
        };
        if fail { panic!("bad {operation}"); }
    }
}

impl OutputModule for SlowModule {
    fn type_id(&self) -> Uuid {
        Uuid::nil()
    }
    fn start(&mut self) {
        self.fault("start");
        self.started.fetch_add(1, Ordering::Relaxed);
        *self.start_thread.lock() = Some(std::thread::current().id());
        self.shared.set_status(ModuleStatus::Running);
    }
    fn stop(&mut self) {
        self.fault("stop");
        self.stopped.fetch_add(1, Ordering::Relaxed);
        self.shared.set_status(ModuleStatus::Stopped);
    }
    fn update(&mut self, commands: &[OutputCommand]) {
        if self.panic_next.swap(false, Ordering::Relaxed) {
            panic!("a bad frame");
        }
        let io_error = {
            let mut faults = self.faults.lock();
            faults.iter().position(|name| *name == "io").map(|i| faults.remove(i)).is_some()
        };
        if io_error { self.shared.record_error("port write failed"); }
        self.seen.lock().push(commands.first().map_or(0, OutputCommand::byte));
        std::thread::sleep(self.delay);
    }
    fn output_count(&self) -> usize {
        1
    }
    fn set_output_count(&mut self, _count: usize) {}
    fn config(&self) -> ControllerConfig {
        self.fault("config");
        ControllerConfig::Debug(DebugConfig::default())
    }
    fn set_config(&mut self, _config: ControllerConfig) -> Result<(), ConfigMismatch> {
        Ok(())
    }
    fn shared(&self) -> &Arc<ModuleShared> {
        &self.shared
    }
}

fn wait_until(what: &str, timeout: Duration, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("timed out waiting for {what}");
}

#[test]
fn a_slow_controller_doesnt_slow_the_engine_and_only_sees_the_latest_frame() {
    let (module, probe) = SlowModule::new(Duration::from_millis(200));
    let mut runner = ControllerRunner::new(Duration::from_millis(1));
    let c = runner.add_controller(Uuid::from_u128(1), "Slow", Box::new(module), true);
    runner.start_all();
    wait_until("the module to start", Duration::from_secs(2), || probe.started.load(Ordering::Relaxed) == 1);

    // The engine publishes far faster than the controller can go. Publishing never waits for it.
    let mut worst = Duration::ZERO;
    let started = Instant::now();
    let mut published = 0u32;
    while started.elapsed() < Duration::from_millis(350) {
        let before = Instant::now();
        runner.publish_controller(c, &[OutputCommand::U8((published % 256) as u8)]);
        worst = worst.max(before.elapsed());
        published += 1;
        std::thread::sleep(Duration::from_micros(200));
    }
    // A single update takes 200 ms, so this would be obvious if publishing waited on it.
    assert!(worst < Duration::from_millis(100), "publishing took {worst:?}");
    assert!(published > 200, "published {published}");

    // The newest frame is what goes out next.
    runner.publish_controller(c, &[OutputCommand::U8(200)]);
    wait_until("the newest frame", Duration::from_secs(2), || probe.seen.lock().last() == Some(&200));
    let seen = probe.seen.lock().clone();
    assert!(seen.len() < published as usize / 10, "the controller should have skipped frames: saw {} of {published}", seen.len());
    let stats = runner.stats(c);
    assert!(stats.frames_replaced > 0);
    assert!(stats.max_update_us >= 190_000, "{stats:?}");
    // The count goes up when an update returns, so the one in progress may not be in it yet.
    assert!(stats.updates as usize + 1 >= seen.len());
    runner.stop_all();
    assert_eq!(probe.stopped.load(Ordering::Relaxed), 1);
}


#[test]
fn consumed_frames_are_not_counted_again_when_a_controller_restarts() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_secs(1));
    let c = runner.add_controller(Uuid::from_u128(1), "Restart", Box::new(module), true);
    let mut replaced = Vec::new();
    for value in 1..=4u8 {
        runner.publish_controller(c, &[OutputCommand::U8(value)]);
        runner.start(c);
        wait_until("the published frame", Duration::from_secs(2), || probe.seen.lock().last() == Some(&value));
        runner.stop(c);
        assert!(!runner.is_running(c));
        assert_eq!(runner.status(c), ModuleStatus::Stopped);
        replaced.push(runner.stats(c).frames_replaced);
    }
    assert_eq!(replaced, vec![0, 0, 0, 0]);
}

#[test]
fn frames_overwritten_while_stopped_are_counted_once_after_restart() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_secs(1));
    let c = runner.add_controller(Uuid::from_u128(1), "Stopped overwrite", Box::new(module), true);
    let mut replaced = Vec::new();
    for values in [&[1u8][..], &[2u8, 3, 4][..], &[5u8][..]] {
        for &value in values {
            runner.publish_controller(c, &[OutputCommand::U8(value)]);
        }
        runner.start(c);
        wait_until("the newest frame", Duration::from_secs(2), || probe.seen.lock().last() == values.last());
        runner.stop(c);
        assert!(!runner.is_running(c));
        replaced.push(runner.stats(c).frames_replaced);
    }
    assert_eq!(replaced, vec![0, 2, 2]);
}

#[test]
fn a_controller_keeps_sending_its_last_frame_when_nothing_new_comes() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let c = runner.add_controller(Uuid::from_u128(1), "Repeat", Box::new(module), true);
    runner.start_all();
    runner.publish_controller(c, &[OutputCommand::U8(42)]);
    wait_until("several sends of the one frame", Duration::from_secs(2), || {
        probe.seen.lock().iter().filter(|v| **v == 42).count() >= 5
    });
    runner.stop_all();
}

#[test]
fn a_controller_runs_at_its_interval_without_drifting() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(10));
    runner.add_controller(Uuid::from_u128(1), "Timed", Box::new(module), true);
    runner.start_all();
    std::thread::sleep(Duration::from_millis(505));
    let count = probe.seen.lock().len();
    runner.stop_all();
    // 50 cycles and the first one, with room for a busy machine.
    assert!((25..=56).contains(&count), "{count} updates in half a second at 10 ms");
}

#[test]
fn a_cycle_that_ran_long_is_made_up_without_sleeping() {
    // `IntervalUpdateSignaler` keeps its schedule: the time lost in a slow update is caught up on.
    let (module, probe) = SlowModule::new(Duration::from_millis(30));
    let mut runner = ControllerRunner::new(Duration::from_millis(10));
    let c = runner.add_controller(Uuid::from_u128(1), "Behind", Box::new(module), true);
    runner.start_all();
    wait_until("a few updates", Duration::from_secs(2), || probe.seen.lock().len() >= 5);
    // Sleeping would take the 10 ms interval. A busy machine can still add a little.
    assert!(runner.stats(c).last_sleep_ms < 5, "{:?}", runner.stats(c));
    runner.stop_all();
}

#[test]
fn the_module_starts_and_stops_on_its_own_thread_with_a_name() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let c = runner.add_controller(Uuid::from_u128(1), "Threaded", Box::new(module), true);
    assert!(!runner.is_running(c));
    runner.start_all();
    assert!(runner.is_running(c));
    wait_until("the module to start", Duration::from_secs(2), || probe.started.load(Ordering::Relaxed) == 1);
    assert_ne!(*probe.start_thread.lock(), Some(std::thread::current().id()));
    assert_eq!(runner.status(c), ModuleStatus::Running);
    runner.stop(c);
    assert!(!runner.is_running(c));
    assert_eq!(runner.status(c), ModuleStatus::Stopped);
    // Stopped controllers can start again, with the same module.
    runner.start(c);
    wait_until("the module to restart", Duration::from_secs(2), || probe.started.load(Ordering::Relaxed) == 2);
    runner.stop(c);
    assert_eq!(probe.stopped.load(Ordering::Relaxed), 2);
    assert!(runner.module(c).is_some());
}

#[test]
fn a_disabled_controller_never_starts() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let c = runner.add_controller(Uuid::from_u128(1), "Off", Box::new(module), false);
    runner.start_all();
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(probe.started.load(Ordering::Relaxed), 0);
    assert!(!runner.is_running(c));
    assert!(!runner.is_enabled(c));
    runner.set_enabled(c, true);
    runner.start_all();
    wait_until("the module to start", Duration::from_secs(2), || probe.started.load(Ordering::Relaxed) == 1);
    runner.set_enabled(c, false);
    assert!(!runner.is_running(c));
}

#[test]
fn a_panic_in_a_module_loses_a_frame_and_nothing_else() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let lines = Arc::new(Mutex::new(Vec::new()));
    let capture = lines.clone();
    runner.set_log_sink(Arc::new(move |logger, message, detail| {
        capture.lock().push((logger.to_string(), message.to_string(), detail.map(str::to_string)));
    }));
    let c = runner.add_controller(Uuid::from_u128(1), "Panics", Box::new(module), true);
    runner.start_all();
    wait_until("a first update", Duration::from_secs(2), || !probe.seen.lock().is_empty());
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    probe.panic_next.store(true, Ordering::Relaxed);
    wait_until("the panic", Duration::from_secs(2), || runner.stats(c).panics == 1);
    std::panic::set_hook(hook);
    runner.publish_controller(c, &[OutputCommand::U8(7)]);
    wait_until("updates to carry on", Duration::from_secs(2), || probe.seen.lock().last() == Some(&7));
    runner.stop_all();
    assert_eq!(*lines.lock(), vec![("Vixen.Sys.OutputController".into(),
        "An error occurred outputting data for controller Panics".into(), Some("a bad frame".into()))]);
}

#[test]
fn failed_start_and_stop_are_logged_and_the_module_can_be_recovered() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let lines = Arc::new(Mutex::new(Vec::new()));
    let capture = lines.clone();
    runner.set_log_sink(Arc::new(move |_, message, detail| {
        capture.lock().push((message.to_string(), detail.unwrap_or("").to_string()));
    }));
    let c = runner.add_controller(Uuid::from_u128(1), "Faulty", Box::new(module), true);
    probe.faults.lock().push("start");
    runner.start(c);
    wait_until("failed start to finish", Duration::from_secs(2), || !runner.is_running(c));
    assert!(probe.seen.lock().is_empty(), "a failed start must never send an update");
    assert_eq!(runner.status(c), ModuleStatus::Error("bad start".into()));
    runner.start(c);
    wait_until("recovered updates", Duration::from_secs(2), || !probe.seen.lock().is_empty());
    probe.faults.lock().push("stop");
    runner.stop(c);
    assert!(!runner.is_running(c));
    assert!(runner.module(c).is_some());
    assert_eq!(*lines.lock(), vec![("Error starting device Faulty".into(), "bad start".into()),
        ("Error trying to stop device Faulty".into(), "bad stop".into())]);
}

#[test]
fn recorded_io_errors_reach_the_log_once_without_stopping_updates() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let lines = Arc::new(Mutex::new(Vec::new()));
    let capture = lines.clone();
    runner.set_log_sink(Arc::new(move |_, message, detail| {
        capture.lock().push((message.to_string(), detail.unwrap_or("").to_string()));
    }));
    let c = runner.add_controller(Uuid::from_u128(1), "Retry", Box::new(module), true);
    probe.faults.lock().push("io");
    runner.start(c);
    wait_until("updates following the failed write", Duration::from_secs(2), || probe.seen.lock().len() >= 3);
    runner.stop(c);
    assert_eq!(*lines.lock(), vec![("An error occurred outputting data for controller Retry".into(),
        "port write failed".into())]);
}

#[test]
fn an_unexpected_worker_panic_is_logged_stops_only_its_controller_and_wakes_waiters() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let (other, other_probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let lines = Arc::new(Mutex::new(Vec::new()));
    let capture = lines.clone();
    runner.set_log_sink(Arc::new(move |logger, message, detail| {
        capture.lock().push((logger.to_string(), message.to_string(), detail.map(str::to_string)));
    }));
    let c = runner.add_controller(Uuid::from_u128(1), "Broken", Box::new(module), true);
    let other_c = runner.add_controller(Uuid::from_u128(2), "Healthy", Box::new(other), true);
    runner.start_all();
    wait_until("both controllers", Duration::from_secs(2), || !probe.seen.lock().is_empty() && !other_probe.seen.lock().is_empty());
    probe.faults.lock().push("config");
    let started = Instant::now();
    assert!(runner.config(c).is_none());
    assert!(started.elapsed() < Duration::from_millis(500), "a finished worker must wake its pending config request");
    wait_until("shutdown diagnostics", Duration::from_secs(2), || lines.lock().len() == 2);
    assert!(runner.is_running(other_c));
    assert_eq!(runner.status(c), ModuleStatus::Error("bad config".into()));
    assert_eq!(*lines.lock(), vec![
        ("Vixen.Sys.Managers.HardwareUpdateThread".into(), "Controller Broken error".into(), Some("bad config".into())),
        ("Vixen.Sys.Managers.OutputDeviceExecution".into(), "Device Broken experienced an error during execution and was shutdown.".into(), None),
    ]);
    runner.start(c);
    wait_until("restarted controller", Duration::from_secs(2), || probe.started.load(Ordering::Relaxed) == 2);
    runner.stop_all();
}

#[test]
fn a_diagnostic_callback_cannot_take_down_the_controller() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    runner.set_log_sink(Arc::new(|_, _, _| panic!("broken log callback")));
    let c = runner.add_controller(Uuid::from_u128(1), "Logging", Box::new(module), true);
    probe.panic_next.store(true, Ordering::Relaxed);
    runner.start(c);
    wait_until("frames following both panics", Duration::from_secs(2), || probe.seen.lock().len() >= 3);
    assert!(runner.is_running(c));
    runner.stop_all();
}

#[test]
fn a_background_module_error_reaches_the_log_even_after_stop() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let shared = module.shared.clone();
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let c = runner.add_controller(Uuid::from_u128(1), "Background", Box::new(module), true);
    let lines = Arc::new(Mutex::new(Vec::new()));
    let capture = lines.clone();
    // ShowPlayback installs logging after from_graph has already added its controllers.
    let observed = Arc::downgrade(&shared);
    runner.set_log_sink(Arc::new(move |_, message, detail| {
        let state = observed.upgrade().unwrap();
        assert_eq!(state.last_error().as_deref(), detail, "callbacks may inspect the state without a shared lock being held");
        assert_eq!(state.status(), ModuleStatus::Stopped);
        capture.lock().push((message.to_string(), detail.unwrap_or("").to_string()));
        if detail == Some("late background write failed") { panic!("first diagnostic delivery failed"); }
    }));
    runner.start(c);
    wait_until("first update", Duration::from_secs(2), || !probe.seen.lock().is_empty());
    runner.stop(c);
    let (done, received) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        shared.record_error("late background write failed");
        shared.record_error("retry background write failed");
        done.send(()).unwrap();
    });
    received.recv_timeout(Duration::from_secs(2)).expect("diagnostics must not deadlock the driver-owned worker");
    worker.join().unwrap();
    assert_eq!(*lines.lock(), vec![
        ("An error occurred outputting data for controller Background".into(), "late background write failed".into()),
        ("An error occurred outputting data for controller Background".into(), "retry background write failed".into()),
    ]);
    assert_eq!(runner.stats(c).errors, 2);
    assert_eq!(runner.status(c), ModuleStatus::Stopped);
}

#[test]
fn pause_stops_updates_and_resume_restarts_them() {
    let (module, probe) = SlowModule::new(Duration::ZERO);
    let mut runner = ControllerRunner::new(Duration::from_millis(2));
    runner.add_controller(Uuid::from_u128(1), "Paused", Box::new(module), true);
    runner.start_all();
    wait_until("updates", Duration::from_secs(2), || probe.seen.lock().len() >= 3);
    runner.pause_all();
    std::thread::sleep(Duration::from_millis(30));
    let count = probe.seen.lock().len();
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(probe.seen.lock().len(), count);
    runner.resume_all();
    wait_until("updates to carry on", Duration::from_secs(2), || probe.seen.lock().len() > count);
    // Stopping a paused controller works too.
    runner.pause_all();
    runner.stop_all();
    assert_eq!(probe.stopped.load(Ordering::Relaxed), 1);
}

#[test]
fn e131_through_the_runner_from_a_real_graph_evaluation() {
    let rx = UdpReceiver::new();
    let element = Uuid::from_u128(1);
    let controller = Uuid::from_u128(2);
    let output = Uuid::from_u128(3);
    let instance = Uuid::from_u128(4);
    let def = GraphDefinition {
        elements: vec![element],
        controllers: vec![ControllerDef {
            id: controller,
            name: "sACN".to_string(),
            hardware_type_id: ids::E131,
            hardware_instance_id: instance,
            outputs: vec![OutputDef { id: output, name: "Output 1".to_string() }],
        }],
        filters: vec![],
        patches: vec![PatchDef { component_id: output, source_id: Some(element), output_index: 0 }],
    };
    let mut graph = OutputGraph::build(&def, &NoModuleData);
    let config = E131Config {
        ids: ModuleIds::new(ids::E131, instance),
        unicast: Some("127.0.0.1".to_string()),
        universes: Some(vec![UniverseEntry::new(true, 9, 0, 1)]),
        ..Default::default()
    };
    let mut module = E131Module::with_port(config, rx.port);
    module.set_output_count(1);
    let counters = module.counters();
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let c = runner.add_controller(controller, "sACN", Box::new(module), true);
    runner.start_all();

    let mut states = vec![CombinedState::default()];
    states[0].rgb = Some(RgbValue { r: 12, g: 200, b: 3 });
    graph.evaluate(&states);
    runner.publish(&graph);

    // A dark frame or two may arrive before that one.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut seen = false;
    let mut last_sequence = None;
    while Instant::now() < deadline && !seen {
        let p = rx.recv();
        assert_eq!(p.len(), packet::SLOTS_OFFSET + 1);
        assert_eq!(&p[packet::UNIVERSE_OFFSET..packet::UNIVERSE_OFFSET + 2], &[0, 9]);
        if let Some(last) = last_sequence {
            assert_eq!(p[packet::SEQUENCE_OFFSET], u8::wrapping_add(last, 1));
        }
        last_sequence = Some(p[packet::SEQUENCE_OFFSET]);
        seen = p[packet::SLOTS_OFFSET] == 200;
    }
    assert!(seen, "the frame should have reached the wire");
    // It keeps going out while nothing changes, as there's no suppression set.
    let p = rx.recv();
    assert_eq!(p[packet::SLOTS_OFFSET], 200);
    runner.stop_all();
    assert!(counters.snapshot().universes[0].packets >= 2);
    assert_eq!(runner.status(c), ModuleStatus::Stopped);
    assert!(runner.stats(c).packets_sent >= 2);
}

#[test]
fn generic_serial_through_the_runner_on_a_pseudo_terminal() {
    let pty = Pty::new();
    let mut module = GenericSerialModule::new(GenericSerialConfig {
        serial: SerialSettings {
            port_name: Some(pty.path.clone()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        },
        header: Some("<".to_string()),
        footer: Some(">".to_string()),
        ..Default::default()
    });
    module.set_output_count(2);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let c = runner.add_controller(Uuid::from_u128(1), "Arduino", Box::new(module), true);
    runner.start_all();
    wait_until("the port to open", Duration::from_secs(2), || runner.status(c) == ModuleStatus::Running);
    runner.publish_controller(c, &[OutputCommand::U8(10), OutputCommand::U8(20)]);
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        let frame = pty.read_exactly(4, Duration::from_millis(200));
        assert_eq!(frame.len(), 4);
        assert_eq!(frame[0], b'<');
        assert_eq!(frame[3], b'>');
        seen.push((frame[1], frame[2]));
        if seen.last() == Some(&(10, 20)) {
            break;
        }
    }
    assert_eq!(seen.last(), Some(&(10, 20)));
    runner.stop_all();
}

#[test]
fn a_port_a_controller_has_open_reports_in_use() {
    use vixen_output::serial::{port_availability, port_in_use, PortAvailability};
    let pty = Pty::new();
    assert_eq!(port_availability(&pty.path), PortAvailability::Available);
    assert!(!port_in_use(&pty.path));
    let mut module = GenericSerialModule::new(GenericSerialConfig {
        serial: SerialSettings {
            port_name: Some(pty.path.clone()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        },
        ..Default::default()
    });
    module.start();
    assert_eq!(module.status(), ModuleStatus::Running);
    assert!(port_in_use(&pty.path));
    module.stop();
    assert!(!port_in_use(&pty.path));
}

#[test]
fn a_port_nobody_reads_doesnt_stop_the_engine_or_shutting_down() {
    // Nothing reads the master, so the terminal's queue fills and the module's write blocks.
    let pty = Pty::new();
    let mut module = GenericSerialModule::new(GenericSerialConfig {
        serial: SerialSettings {
            port_name: Some(pty.path.clone()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        },
        ..Default::default()
    });
    module.set_output_count(4096);
    let mut runner = ControllerRunner::new(Duration::from_millis(1));
    let c = runner.add_controller(Uuid::from_u128(1), "Jammed", Box::new(module), true);
    runner.start_all();
    let frame = vec![OutputCommand::U8(1); 4096];
    let started = Instant::now();
    let mut worst = Duration::ZERO;
    while started.elapsed() < Duration::from_millis(200) {
        let before = Instant::now();
        runner.publish_controller(c, &frame);
        worst = worst.max(before.elapsed());
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(worst < Duration::from_millis(50), "publishing took {worst:?}");
    let before = Instant::now();
    runner.stop_all();
    assert!(before.elapsed() < Duration::from_secs(2), "stopping took {:?}", before.elapsed());
    assert_eq!(runner.status(c), ModuleStatus::Stopped);
}

#[test]
fn from_graph_builds_every_controller_and_honours_disabled_devices() {
    let Some(real) = common::load_real() else { return };
    let graph = OutputGraph::build(&real.def, &real.modules);
    let mut runner = ControllerRunner::from_graph(&graph, &real.disabled, Duration::from_millis(real.default_update_interval_ms));
    assert_eq!(runner.controller_count(), 2);
    // Everything in this show is disabled, so starting touches no hardware.
    runner.start_all();
    assert!(!runner.is_running(0));
    assert!(!runner.is_running(1));
    assert_eq!(runner.status(0), ModuleStatus::Stopped);
    runner.publish(&graph);
    assert_eq!(runner.module(0).unwrap().output_count(), 16);
    assert_eq!(runner.module(1).unwrap().output_count(), 3351);

    // Enabling the real Arduino only finds COM9 isn't a port here. No hardware is touched.
    runner.set_enabled(0, true);
    runner.start(0);
    wait_until("the port to report", Duration::from_secs(2), || matches!(runner.status(0), ModuleStatus::Unavailable(_)));
    let ModuleStatus::Unavailable(reason) = runner.status(0) else { panic!() };
    assert!(reason.contains("COM9"), "{reason}");
    assert!(runner.is_running(0));
    runner.stop(0);
    let ControllerConfig::GenericSerial(saved) = runner.module(0).unwrap().config() else { panic!() };
    assert_eq!(saved.serial.port_name.as_deref(), Some("COM9"));

    // The dummy controller has no hardware at all.
    runner.set_enabled(1, true);
    runner.start(1);
    wait_until("the dummy controller", Duration::from_secs(2), || runner.status(1) == ModuleStatus::Running);
    runner.stop_all();
}

#[test]
fn a_running_controller_answers_for_its_config_and_can_take_a_new_one() {
    let rx = UdpReceiver::new();
    let instance = Uuid::from_u128(5);
    let config = E131Config {
        ids: ModuleIds::new(ids::E131, instance),
        unicast: Some("127.0.0.1".to_string()),
        universes: Some(vec![UniverseEntry::new(true, 9, 0, 1)]),
        ..Default::default()
    };
    let mut module = E131Module::with_port(config.clone(), rx.port);
    module.set_output_count(1);
    let mut runner = ControllerRunner::new(Duration::from_millis(2));
    let c = runner.add_controller(Uuid::from_u128(1), "sACN", Box::new(module), true);
    assert!(matches!(runner.config(c), Some(ControllerConfig::E131(_))));
    runner.start_all();
    rx.recv();
    rx.recv();
    // The sequence number has moved on, and the answer includes that.
    wait_until("a sequence number to show up", Duration::from_secs(2), || {
        let Some(ControllerConfig::E131(now)) = runner.config(c) else { panic!() };
        now.universes.unwrap()[0].seq_num > 0
    });
    assert!(runner.e131_stats(c).unwrap().events > 0);

    // A change takes effect without stopping the controller. The new universe number shows up in the packets.
    let mut changed = config;
    changed.universes = Some(vec![UniverseEntry::new(true, 11, 0, 1)]);
    assert_eq!(runner.set_config(c, ControllerConfig::DummyLighting(Default::default())), Err(ConfigMismatch));
    assert_eq!(runner.set_config(c, ControllerConfig::E131(changed)), Ok(()));
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut seen = false;
    while Instant::now() < deadline && !seen {
        let p = rx.recv();
        seen = p[packet::UNIVERSE_OFFSET..packet::UNIVERSE_OFFSET + 2] == [0, 11];
    }
    assert!(seen);
    assert_eq!(runner.stats(c).panics, 0);
    runner.stop_all();
    let Some(ControllerConfig::E131(saved)) = runner.config(c) else { panic!() };
    assert_eq!(saved.universes.unwrap()[0].universe, 11);
    assert!(runner.dummy_view(c).is_none());
    assert!(runner.debug_frame(c).is_none());
}

#[test]
fn views_of_dummy_and_debug_controllers_follow_the_frames() {
    use vixen_output::modules::{DebugModule, DummyLightingConfig, DummyLightingModule};
    let mut dummy = DummyLightingModule::new(DummyLightingConfig::default());
    dummy.set_output_count(2);
    let mut debug = DebugModule::new(DebugConfig::default());
    debug.set_output_count(1);
    let mut runner = ControllerRunner::new(Duration::from_millis(2));
    let d = runner.add_controller(Uuid::from_u128(1), "Dummy", Box::new(dummy), true);
    let g = runner.add_controller(Uuid::from_u128(2), "Debug", Box::new(debug), true);
    runner.start_all();
    runner.publish_controller(d, &[OutputCommand::U8(1), OutputCommand::U8(2)]);
    runner.publish_controller(g, &[OutputCommand::U16(300)]);
    let view = runner.dummy_view(d).unwrap();
    let frame = runner.debug_frame(g).unwrap();
    wait_until("the dummy view", Duration::from_secs(2), || view.lock().values == [1, 2]);
    wait_until("the debug frame", Duration::from_secs(2), || *frame.lock() == [OutputCommand::U16(300)]);
    assert!(runner.e131_stats(d).is_none());
    runner.stop_all();
}

#[test]
fn missing_hardware_runs_quietly() {
    let mut module = OpenDmxModule::with_device_list(OpenDmxConfig::default(), Vec::new);
    module.set_output_count(2);
    let mut runner = ControllerRunner::new(Duration::from_millis(5));
    let c = runner.add_controller(Uuid::from_u128(1), "Open DMX", Box::new(module), true);
    runner.start_all();
    wait_until("the status", Duration::from_secs(2), || matches!(runner.status(c), ModuleStatus::Unavailable(_)));
    runner.publish_controller(c, &[OutputCommand::U8(1), OutputCommand::U8(2)]);
    wait_until("an update", Duration::from_secs(2), || runner.stats(c).updates > 2);
    assert_eq!(runner.stats(c).panics, 0);
    runner.stop_all();
}

#[test]
fn a_mailbox_can_be_used_from_two_threads_at_once() {
    let mailbox = Arc::new(Mailbox::new(1000));
    let writer = {
        let mailbox = mailbox.clone();
        std::thread::spawn(move || {
            for i in 0..2000u32 {
                let frame = vec![OutputCommand::U8((i % 256) as u8); 1000];
                mailbox.publish(&frame);
            }
        })
    };
    let mut dest = vec![OutputCommand::None; 1000];
    while !writer.is_finished() {
        if mailbox.take_latest(&mut dest) {
            // A frame is never torn: every output comes from the same publish.
            assert!(dest.iter().all(|c| *c == dest[0]));
        }
    }
    writer.join().unwrap();
}
