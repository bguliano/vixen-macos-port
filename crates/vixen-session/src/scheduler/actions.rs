//! What a show's items do when they run (F63): the runner interface the executor drives, and the Pause and Launch actions.

use crate::logging;
use crate::shows::{ActionType, ShowItem};
use parking_lot::{Condvar, Mutex};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Told when an action ends, by itself or because it was stopped. Only the first call counts.
#[derive(Clone)]
pub struct Done {
    fired: Arc<AtomicBool>,
    notify: Arc<dyn Fn() + Send + Sync>,
}

impl Done {
    pub fn new(notify: impl Fn() + Send + Sync + 'static) -> Done {
        Done {
            fired: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(notify),
        }
    }

    pub fn complete(&self) {
        if !self.fired.swap(true, Ordering::SeqCst) {
            (self.notify)();
        }
    }
}

/// An action that has been started.
pub trait RunningAction: Send {
    /// Ends it early. Its `Done` still fires.
    fn stop(&mut self);
}

/// One that was over as soon as it began.
pub struct Finished;

impl RunningAction for Finished {
    fn stop(&mut self) {}
}

/// Runs show items. The executor never blocks in these: `execute` returns at once and `done` reports the end, from any thread.
pub trait ActionRunner: Send + Sync {
    /// A run's resource leases must be independent of another scheduled copy of the same show.
    fn for_run(&self) -> Option<Arc<dyn ActionRunner>> {
        None
    }

    /// Whether `execute` would first have to get the item ready: never loaded, or its file changed since.
    fn needs_pre_process(&self, _item: &ShowItem) -> bool {
        false
    }

    /// Gets an item ready ahead of its turn. May take long; called off the executor's thread.
    fn pre_process(&self, _item: &ShowItem) {}

    fn execute(&self, item: &ShowItem, done: Done) -> Box<dyn RunningAction>;

    /// The show these items belong to is over: let go of what they hold (F64).
    fn release(&self, _items: &[ShowItem]) {}
}

/// The runner the app uses: Pause and Launch here, sequences through the one given.
pub struct StandardRunner {
    pub sequence: Arc<dyn ActionRunner>,
}

impl ActionRunner for StandardRunner {
    fn for_run(&self) -> Option<Arc<dyn ActionRunner>> {
        self.sequence
            .for_run()
            .map(|sequence| Arc::new(StandardRunner { sequence }) as Arc<dyn ActionRunner>)
    }

    fn needs_pre_process(&self, item: &ShowItem) -> bool {
        item.action == ActionType::Sequence && self.sequence.needs_pre_process(item)
    }

    fn pre_process(&self, item: &ShowItem) {
        if item.action == ActionType::Sequence {
            self.sequence.pre_process(item);
        }
    }

    fn execute(&self, item: &ShowItem, done: Done) -> Box<dyn RunningAction> {
        match &item.action {
            ActionType::Sequence => self.sequence.execute(item, done),
            ActionType::Pause => pause(item.pause_seconds, done),
            ActionType::Launch => {
                let launched = launch(item, Box::new(|_| {}), done);
                Box::new(launched)
            }
            // GUARD: an action upstream's enum no longer has runs as a pause of no length.
            ActionType::Other(_) => {
                done.complete();
                Box::new(Finished)
            }
        }
    }

    fn release(&self, items: &[ShowItem]) {
        self.sequence.release(items);
    }
}

struct PauseHandle {
    cancel: Arc<(Mutex<bool>, Condvar)>,
}

impl RunningAction for PauseHandle {
    fn stop(&mut self) {
        *self.cancel.0.lock() = true;
        self.cancel.1.notify_all();
    }
}

/// `PauseAction`: done after the seconds, or as soon as it's stopped. GUARD: 0 seconds, which upstream's timer throws on, is done at
/// once.
pub fn pause(seconds: i32, done: Done) -> Box<dyn RunningAction> {
    if seconds <= 0 {
        done.complete();
        return Box::new(Finished);
    }
    let cancel = Arc::new((Mutex::new(false), Condvar::new()));
    let waiter = cancel.clone();
    std::thread::Builder::new()
        .name("show-pause".into())
        .spawn(move || {
            let mut cancelled = waiter.0.lock();
            let deadline = std::time::Instant::now() + Duration::from_secs(seconds as u64);
            while !*cancelled {
                if waiter.1.wait_until(&mut cancelled, deadline).timed_out() {
                    break;
                }
            }
            drop(cancelled);
            done.complete();
        })
        .expect("pause thread");
    Box::new(PauseHandle { cancel })
}

/// .NET's reading of `ProcessStartInfo.Arguments` on Unix: white space separates, double quotes group, `\"` is a quote, and inside
/// quotes `""` is one too. Backslashes are literal except before a quote, where each pair is one backslash.
pub fn split_command_line(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut args = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i == chars.len() {
            break;
        }
        let mut arg = String::new();
        let mut in_quotes = false;
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' {
                let mut slashes = 0;
                while i < chars.len() && chars[i] == '\\' {
                    slashes += 1;
                    i += 1;
                }
                if i < chars.len() && chars[i] == '"' {
                    arg.extend(std::iter::repeat('\\').take(slashes / 2));
                    if slashes % 2 == 1 {
                        arg.push('"');
                        i += 1;
                    }
                } else {
                    arg.extend(std::iter::repeat('\\').take(slashes));
                }
                continue;
            }
            if c == '"' {
                if in_quotes && chars.get(i + 1) == Some(&'"') {
                    arg.push('"');
                    i += 2;
                } else {
                    in_quotes = !in_quotes;
                    i += 1;
                }
                continue;
            }
            if c.is_whitespace() && !in_quotes {
                break;
            }
            arg.push(c);
            i += 1;
        }
        args.push(arg);
    }
    args
}

/// A launched program. `stop` kills one that is being waited for; one the show didn't wait for is left alone (upstream kills it if it's
/// still running at Stop Now, by an accident of bookkeeping).
pub struct Launched {
    child: Option<Arc<Mutex<Child>>>,
    stopped: Arc<AtomicBool>,
    waiting: bool,
}

impl Launched {
    fn none() -> Launched {
        Launched {
            child: None,
            stopped: Arc::new(AtomicBool::new(false)),
            waiting: false,
        }
    }

    /// The tester's Close: stops the program whether or not the item waits for it.
    pub fn kill(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Some(child) = &self.child {
            let _ = child.lock().kill();
        }
    }
}

impl RunningAction for Launched {
    fn stop(&mut self) {
        if self.waiting {
            self.kill();
        }
    }
}

/// Plain executables remain direct children; app/Terminal ownership lives in
/// native_launch so stopping never targets an unrelated `open` process.
fn command_for(item: &ShowItem) -> Command {
    let program = item.launch_program_name.clone().unwrap_or_default();
    let args = split_command_line(item.launch_command_line.as_deref().unwrap_or(""));
    let mut command = Command::new(&program);
    command.args(&args);
    command
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

/// `LaunchAction.Execute`. `result` gets `ResultString` when the program ends: its standard output, or its standard error if that
/// was empty, or the reason it couldn't start. `done` fires on start unless the item waits for exit. Both pipes are read as they
/// fill, where upstream reads them only at exit and so hangs on a program that writes more than a pipe holds.
pub fn launch(item: &ShowItem, result: Box<dyn FnOnce(String) + Send>, done: Done) -> Launched {
    let waiting = item.launch_wait_for_exit;
    if super::native_launch::needed(item) {
        let stopped = Arc::new(AtomicBool::new(false));
        super::native_launch::launch(item, result, done, stopped.clone());
        return Launched {
            child: None,
            stopped,
            waiting,
        };
    }
    let spawned = {
        let mut command = command_for(item);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.spawn()
    };
    let mut child = match spawned {
        Ok(started) => started,
        Err(error) => {
            logging::error(
                "Scheduler",
                "An error occurred during launch event.",
                Some(&error.to_string()),
            );
            result(error.to_string());
            done.complete();
            return Launched::none();
        }
    };
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let child = Arc::new(Mutex::new(child));
    let stopped = Arc::new(AtomicBool::new(false));
    if !waiting {
        done.complete();
    }
    let watched = child.clone();
    std::thread::Builder::new()
        .name("show-launch".into())
        .spawn(move || {
            loop {
                match watched.lock().try_wait() {
                    Ok(None) => {}
                    _ => break,
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let output = out.join().unwrap_or_default();
            let errors = err.join().unwrap_or_default();
            result(if output.is_empty() { errors } else { output });
            done.complete();
        })
        .expect("launch thread");
    Launched {
        child: Some(child),
        stopped,
        waiting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shows::ShowItemType;
    use std::sync::mpsc;

    fn signal() -> (Done, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        (Done::new(move || tx.lock().send(()).unwrap()), rx)
    }

    fn launch_item(program: &str, command_line: &str, wait: bool) -> ShowItem {
        let mut item = ShowItem::new(ShowItemType::Startup, "");
        item.launch_program_name = Some(program.to_string());
        item.launch_command_line = Some(command_line.to_string());
        item.launch_wait_for_exit = wait;
        item
    }

    fn run(item: &ShowItem) -> (Launched, mpsc::Receiver<String>, mpsc::Receiver<()>) {
        let (tx, text) = mpsc::channel();
        let (done, finished) = signal();
        (
            launch(item, Box::new(move |t| tx.send(t).unwrap()), done),
            text,
            finished,
        )
    }

    const LONG: Duration = Duration::from_secs(20);

    #[test]
    fn command_lines_split_as_dotnet_does() {
        assert_eq!(split_command_line(""), Vec::<String>::new());
        assert_eq!(split_command_line("  a  b\tc "), ["a", "b", "c"]);
        assert_eq!(split_command_line(r#""ho ho" ho"#), ["ho ho", "ho"]);
        assert_eq!(
            split_command_line(r#"say \"hi\" now"#),
            ["say", "\"hi\"", "now"]
        );
        assert_eq!(split_command_line(r#"a"b c"d e"#), ["ab cd", "e"]);
        assert_eq!(split_command_line(r#""a ""b"" c""#), ["a \"b\" c"]);
        assert_eq!(
            split_command_line(r#"C:\dir\file "C:\my dir\\" x"#),
            [r"C:\dir\file", r"C:\my dir\", "x"]
        );
        assert_eq!(split_command_line(r#""" x"#), ["", "x"]);
    }

    #[test]
    fn done_fires_once() {
        let (done, fired) = signal();
        done.complete();
        done.clone().complete();
        assert_eq!(fired.try_iter().count(), 1);
    }

    #[test]
    fn a_pause_ends_early_when_stopped_and_at_once_for_zero() {
        let (done, finished) = signal();
        pause(0, done);
        assert!(finished.try_recv().is_ok());

        let (done, finished) = signal();
        let mut handle = pause(900, done);
        assert!(finished.recv_timeout(Duration::from_millis(50)).is_err());
        handle.stop();
        assert!(finished.recv_timeout(LONG).is_ok());

        let (done, finished) = signal();
        let started = std::time::Instant::now();
        let _handle = pause(1, done);
        assert!(finished.recv_timeout(LONG).is_ok());
        assert!(started.elapsed() >= Duration::from_millis(950));
    }

    #[test]
    fn a_waited_program_reports_its_output_then_completes() {
        let (_handle, text, finished) = run(&launch_item("/bin/echo", "\"ho ho\" ho", true));
        assert_eq!(text.recv_timeout(LONG).unwrap(), "ho ho ho\n");
        assert!(finished.recv_timeout(LONG).is_ok());

        // Standard error stands in when there's no output.
        let (_handle, text, _finished) =
            run(&launch_item("/bin/sh", "-c \"echo oops 1>&2\"", true));
        assert_eq!(text.recv_timeout(LONG).unwrap(), "oops\n");

        // More than a pipe holds, which hangs upstream.
        let (_handle, text, _finished) = run(&launch_item(
            "/bin/sh",
            "-c \"head -c 300000 /dev/zero | tr '\\0' x\"",
            true,
        ));
        assert_eq!(text.recv_timeout(LONG).unwrap().len(), 300_000);
    }

    #[test]
    fn a_program_that_isnt_waited_for_completes_on_start_and_survives_stop() {
        let (mut handle, text, finished) = run(&launch_item(
            "/bin/sh",
            "-c \"sleep 0.4; echo late\"",
            false,
        ));
        assert!(finished.recv_timeout(Duration::from_millis(200)).is_ok());
        handle.stop();
        assert_eq!(text.recv_timeout(LONG).unwrap(), "late\n");
    }

    #[test]
    fn stop_kills_a_waited_program() {
        let (mut handle, text, finished) = run(&launch_item("/bin/sleep", "600", true));
        assert!(finished.recv_timeout(Duration::from_millis(200)).is_err());
        handle.stop();
        assert!(finished.recv_timeout(LONG).is_ok());
        assert_eq!(text.recv_timeout(LONG).unwrap(), "");
    }

    #[test]
    fn a_program_that_cant_start_gives_its_reason_and_completes() {
        let (mut handle, text, finished) = run(&launch_item("C:\\Windows\\notepad.exe", "", true));
        assert!(finished.recv_timeout(LONG).is_ok());
        assert!(!text.recv_timeout(LONG).unwrap().is_empty());
        // GUARD: upstream's Close throws here.
        handle.kill();
    }
}
