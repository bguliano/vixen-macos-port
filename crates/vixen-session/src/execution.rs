//! The execution engine's state machine. Port of `Vixen.Core/Sys/State/Execution` and the state half of `Execution`.

use crate::logging;
use parking_lot::Mutex;

/// `Execution.State`. The names are what the status bar shows.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ExecutionState {
    Closed,
    Opening,
    Open,
    Closing,
    OpeningForTesting,
    OpenForTesting,
}

impl ExecutionState {
    pub fn name(self) -> &'static str {
        match self {
            ExecutionState::Closed => "Closed",
            ExecutionState::Opening => "Opening",
            ExecutionState::Open => "Open",
            ExecutionState::Closing => "Closing",
            ExecutionState::OpeningForTesting => "Opening for testing",
            ExecutionState::OpenForTesting => "Open for testing",
        }
    }

    pub fn text(self) -> &'static str {
        self.name()
    }

    pub fn is_open(self) -> bool {
        matches!(self, ExecutionState::Open | ExecutionState::Opening)
    }

    pub fn is_closed(self) -> bool {
        matches!(self, ExecutionState::Closed | ExecutionState::Closing)
    }

    pub fn is_in_test(self) -> bool {
        matches!(self, ExecutionState::OpeningForTesting | ExecutionState::OpenForTesting)
    }

    /// `VixenApplication.executionStateChangedHandler`'s light. ForestGreen, Firebrick, DodgerBlue, and Gold for a state that doesn't
    /// exist, kept as written.
    fn light(self) -> (u8, u8, u8) {
        if self.is_open() {
            (34, 139, 34)
        } else if self.is_closed() {
            (178, 34, 34)
        } else if self.is_in_test() {
            (30, 144, 255)
        } else {
            (255, 215, 0)
        }
    }
}

/// What the status bar and the Execution Engine menu show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionStatus {
    pub text: String,
    pub light_rgb: (u8, u8, u8),
    pub start_enabled: bool,
    pub stop_enabled: bool,
}

impl ExecutionStatus {
    pub fn of(state: ExecutionState) -> ExecutionStatus {
        ExecutionStatus {
            text: format!("Execution: {}", state.name()),
            light_rgb: state.light(),
            start_enabled: !state.is_open(),
            stop_enabled: !state.is_closed(),
        }
    }
}

/// What opening and closing do to the outside world: `StandardOpeningBehavior` and `StandardClosingBehavior`.
pub trait ExecutionDevices: Send + Sync {
    /// Starts every output device that isn't disabled. One failing must not stop the others.
    fn start_devices(&self);
    /// Releases every context, so whatever's playing stops, then stops all output devices.
    fn stop_devices(&self);
}

/// Called with each state as it's entered, in order, on the thread that asked for the change. It mustn't ask for another change.
pub type ExecutionListener = Box<dyn Fn(ExecutionState) + Send + Sync>;

/// `ExecutionStateEngine`. It starts in Closed without running Closed's entry.
pub struct ExecutionEngine {
    devices: Box<dyn ExecutionDevices>,
    state: Mutex<ExecutionState>,
    /// Held for a whole change, since starting devices can take a while and two changes mustn't interleave.
    changing: Mutex<()>,
    listeners: Mutex<Vec<ExecutionListener>>,
}

impl ExecutionEngine {
    pub fn new(devices: Box<dyn ExecutionDevices>) -> ExecutionEngine {
        ExecutionEngine {
            devices,
            state: Mutex::new(ExecutionState::Closed),
            changing: Mutex::new(()),
            listeners: Mutex::new(Vec::new()),
        }
    }

    pub fn state(&self) -> ExecutionState {
        *self.state.lock()
    }

    pub fn is_open(&self) -> bool {
        self.state().is_open()
    }

    pub fn is_closed(&self) -> bool {
        self.state().is_closed()
    }

    pub fn is_in_test(&self) -> bool {
        self.state().is_in_test()
    }

    pub fn status(&self) -> ExecutionStatus {
        ExecutionStatus::of(self.state())
    }

    pub fn subscribe(&self, listener: ExecutionListener) {
        self.listeners.lock().push(listener);
    }

    /// `OpenExecution`. Only Closed reacts.
    pub fn open(&self) {
        let _changing = self.changing.lock();
        if self.state() != ExecutionState::Closed {
            return;
        }
        self.enter(ExecutionState::Opening);
        logging::info("Vixen.Sys.State.Execution.OpeningState", "Vixen execution engine entering the open state...");
        self.devices.start_devices();
        self.enter(ExecutionState::Open);
        logging::info("Vixen.Sys.State.Execution.OpenState", "Vixen execution engine entered the open state.");
    }

    /// `OpenTest`. Nothing in the original calls it, and it's kept as written.
    pub fn open_test(&self) {
        let _changing = self.changing.lock();
        if self.state() != ExecutionState::Closed {
            return;
        }
        self.enter(ExecutionState::OpeningForTesting);
        logging::info("Vixen.Sys.State.Execution.TestOpeningState", "Vixen execution engine entering the testing state...");
        self.devices.start_devices();
        self.enter(ExecutionState::OpenForTesting);
        logging::info("Vixen.Sys.State.Execution.TestOpenState", "Vixen execution engine entered the testing state.");
    }

    /// `CloseExecution`. Open and Open for testing react.
    pub fn close(&self) {
        let _changing = self.changing.lock();
        if !matches!(self.state(), ExecutionState::Open | ExecutionState::OpenForTesting) {
            return;
        }
        self.enter(ExecutionState::Closing);
        logging::info("Vixen.Sys.State.Execution.ClosingState", "Vixen execution engine entering the closing state...");
        self.devices.stop_devices();
        self.enter(ExecutionState::Closed);
        logging::info("Vixen.Sys.State.Execution.ClosedState", "Vixen execution engine entered the closed state.");
    }

    /// `CloseTest`, which is `CloseExecution` under another name.
    pub fn close_test(&self) {
        self.close();
    }

    fn enter(&self, state: ExecutionState) {
        *self.state.lock() = state;
        for listener in self.listeners.lock().iter() {
            listener(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Default)]
    struct Recorder {
        calls: Mutex<Vec<&'static str>>,
    }

    struct Devices(Arc<Recorder>);

    impl ExecutionDevices for Devices {
        fn start_devices(&self) {
            self.0.calls.lock().push("start");
        }
        fn stop_devices(&self) {
            self.0.calls.lock().push("stop");
        }
    }

    fn engine() -> (ExecutionEngine, Arc<Mutex<Vec<ExecutionState>>>, Arc<Recorder>) {
        let recorder = Arc::new(Recorder::default());
        let engine = ExecutionEngine::new(Box::new(Devices(recorder.clone())));
        let events = Arc::new(Mutex::new(Vec::new()));
        let log = events.clone();
        engine.subscribe(Box::new(move |state| log.lock().push(state)));
        (engine, events, recorder)
    }

    #[test]
    fn open_gives_opening_then_open_and_again_gives_nothing() {
        let (engine, events, recorder) = engine();
        assert_eq!(engine.state(), ExecutionState::Closed);
        engine.open();
        assert_eq!(*events.lock(), [ExecutionState::Opening, ExecutionState::Open]);
        assert_eq!(*recorder.calls.lock(), ["start"]);
        engine.open();
        assert_eq!(events.lock().len(), 2);
        assert_eq!(*recorder.calls.lock(), ["start"]);
    }

    #[test]
    fn close_gives_closing_then_closed_and_again_gives_nothing() {
        let (engine, events, recorder) = engine();
        engine.close();
        assert!(events.lock().is_empty(), "closed ignores close");
        engine.open();
        events.lock().clear();
        engine.close();
        assert_eq!(*events.lock(), [ExecutionState::Closing, ExecutionState::Closed]);
        assert_eq!(*recorder.calls.lock(), ["start", "stop"]);
        engine.close();
        assert_eq!(events.lock().len(), 2);
    }

    #[test]
    fn the_test_states_open_ignore_open_and_close_normally() {
        let (engine, events, _) = engine();
        engine.open_test();
        assert_eq!(*events.lock(), [ExecutionState::OpeningForTesting, ExecutionState::OpenForTesting]);
        engine.open();
        assert_eq!(engine.state(), ExecutionState::OpenForTesting);
        engine.open_test();
        assert_eq!(events.lock().len(), 2);
        engine.close();
        assert_eq!(engine.state(), ExecutionState::Closed);
    }

    #[test]
    fn close_test_closes_the_normal_state_too() {
        let (engine, _, _) = engine();
        engine.open();
        engine.close_test();
        assert_eq!(engine.state(), ExecutionState::Closed);
    }

    #[test]
    fn the_status_table() {
        let closed = ExecutionStatus::of(ExecutionState::Closed);
        assert_eq!(closed.text, "Execution: Closed");
        assert_eq!(closed.light_rgb, (178, 34, 34));
        assert!(closed.start_enabled && !closed.stop_enabled);

        let open = ExecutionStatus::of(ExecutionState::Open);
        assert_eq!(open.text, "Execution: Open");
        assert_eq!(open.light_rgb, (34, 139, 34));
        assert!(!open.start_enabled && open.stop_enabled);

        let test = ExecutionStatus::of(ExecutionState::OpenForTesting);
        assert_eq!(test.text, "Execution: Open for testing");
        assert_eq!(test.light_rgb, (30, 144, 255));
        assert!(test.start_enabled && test.stop_enabled);

        assert_eq!(ExecutionStatus::of(ExecutionState::Opening).light_rgb, (34, 139, 34));
        assert_eq!(ExecutionStatus::of(ExecutionState::Closing).light_rgb, (178, 34, 34));
        assert_eq!(ExecutionStatus::of(ExecutionState::OpeningForTesting).text, "Execution: Opening for testing");
    }

    #[test]
    fn a_listener_can_read_the_state_it_was_told() {
        let (engine, _, _) = engine();
        let engine = Arc::new(engine);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (weak, log) = (Arc::downgrade(&engine), seen.clone());
        engine.subscribe(Box::new(move |_| {
            if let Some(engine) = weak.upgrade() {
                log.lock().push(engine.status().text);
            }
        }));
        engine.open();
        assert_eq!(*seen.lock(), ["Execution: Opening", "Execution: Open"]);
    }
}
