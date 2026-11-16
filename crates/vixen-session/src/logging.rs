//! The log files, as `NLog.config` lays them out. Lines go through a channel to one writer thread, so logging never touches the disk
//! on the caller's thread, and a file isn't kept open between writes.
//!
//! Nothing is written until [`init`] has been called, so tests and tools that never start the app log nowhere.

use parking_lot::Mutex;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use std::time::SystemTime;
use vixen_io::LocalDateTime;

const ARCHIVE_ABOVE_BYTES: u64 = 1_000_000;
const ARCHIVE_FOLDER: &str = "archive";

/// The first thing a fatal log line says.
pub const FATAL_PREFIX: &str = "An application error occurred. Please contact the Vixen Dev Team with the following information:\n\n";

/// `Program`'s logger, which writes the fatal lines.
const PROGRAM_LOGGER: &str = "VixenApplication.Program";

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    /// `${level}`, part of the file names.
    fn name(self) -> &'static str {
        match self {
            LogLevel::Debug => "Debug",
            LogLevel::Info => "Info",
            LogLevel::Warn => "Warn",
            LogLevel::Error => "Error",
            LogLevel::Fatal => "Fatal",
        }
    }
}

/// `NLog.config`'s targets.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Target {
    /// `{prefix}.{level}.log`.
    prefix: &'static str,
    /// The archive's prefix. `WebRequestAccessLogs` really does archive under `Application`.
    archive_prefix: &'static str,
    keep: usize,
}

const LOGS: Target = Target { prefix: "Application", archive_prefix: "Application", keep: 30 };
const WEB_ACCESS: Target = Target { prefix: "WebAccess", archive_prefix: "Application", keep: 5 };
const SCHEDULE: Target = Target { prefix: "Scheduler", archive_prefix: "Scheduler", keep: 30 };
const V2_CONVERT: Target = Target { prefix: "V2Convert", archive_prefix: "V2Convert", keep: 30 };

/// One `<logger>` rule. A name ending in `*` matches by prefix.
struct Rule {
    name: &'static str,
    min: LogLevel,
    max: LogLevel,
    target: Option<Target>,
}

/// `NLog.config`'s rules, in order. All but the last are `final`.
const RULES: [Rule; 6] = [
    Rule { name: "VixenModules.App.WebServer.RequestLoggingMiddleware", min: LogLevel::Warn, max: LogLevel::Fatal, target: Some(WEB_ACCESS) },
    Rule { name: "VixenModules.SequenceType.Vixen2x.*", min: LogLevel::Info, max: LogLevel::Fatal, target: Some(V2_CONVERT) },
    Rule { name: "Scheduler", min: LogLevel::Debug, max: LogLevel::Fatal, target: Some(SCHEDULE) },
    Rule { name: "Microsoft.Hosting.Lifetime", min: LogLevel::Info, max: LogLevel::Fatal, target: Some(LOGS) },
    Rule { name: "Microsoft.*", min: LogLevel::Debug, max: LogLevel::Info, target: None },
    Rule { name: "*", min: LogLevel::Info, max: LogLevel::Fatal, target: Some(LOGS) },
];

/// Where a line goes. NLog's `final` only stops the levels its rule takes, so a level outside a rule's range falls through to the
/// rules after it: the web server's Info request lines reach `Application.Info.log`.
fn route(logger: &str, level: LogLevel) -> Option<Target> {
    let matches = |pattern: &str| match pattern.strip_suffix('*') {
        Some(prefix) => logger.starts_with(prefix),
        None => logger == pattern,
    };
    RULES.iter().find(|rule| matches(rule.name) && rule.min <= level && level <= rule.max).and_then(|rule| rule.target)
}

/// `${date:format=MM/dd/yyyy HH\:mm\:ss.fff} [${logger}] ${message} ${exception:format=ToString,StackTrace}`. The space before an empty
/// exception is in the original's files too.
fn format_line(time: &LocalDateTime, logger: &str, message: &str, error: Option<&str>) -> String {
    format!("{} [{}] {} {}", time.log_text(), logger, message, error.unwrap_or(""))
}

enum Message {
    Line { target: Target, level: LogLevel, text: String, day: String },
    Flush(Sender<()>),
}

/// One folder of log files. Separate from the global so tests can use their own.
struct Writer {
    folder: PathBuf,
}

impl Writer {
    fn write(&self, target: Target, level: LogLevel, text: &str, day: &str) {
        let path = self.folder.join(format!("{}.{}.log", target.prefix, level.name()));
        self.roll_if_needed(&path, target, level, day, text.len() as u64);
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{text}");
        }
    }

    /// `archiveAboveSize` and `archiveEvery="Day"`, numbered by date: `Application.Info.2025-12-20.0.log`.
    fn roll_if_needed(&self, path: &Path, target: Target, level: LogLevel, today: &str, incoming: u64) {
        let Ok(meta) = fs::metadata(path) else {
            return;
        };
        let file_day = meta.modified().ok().map(|t| day_of(t)).unwrap_or_else(|| today.to_string());
        let too_big = meta.len() + incoming > ARCHIVE_ABOVE_BYTES;
        if !too_big && file_day == today {
            return;
        }
        let archive = self.folder.join(ARCHIVE_FOLDER);
        if fs::create_dir_all(&archive).is_err() {
            return;
        }
        let stem = format!("{}.{}.{}.", target.archive_prefix, level.name(), file_day);
        let number = (0..).find(|n| !archive.join(format!("{stem}{n}.log")).exists()).unwrap_or(0);
        if fs::rename(path, archive.join(format!("{stem}{number}.log"))).is_ok() {
            self.prune(&archive, target, level);
        }
    }

    /// `maxArchiveFiles`.
    fn prune(&self, archive: &Path, target: Target, level: LogLevel) {
        let prefix = format!("{}.{}.", target.archive_prefix, level.name());
        let Ok(entries) = fs::read_dir(archive) else {
            return;
        };
        let mut files: Vec<(SystemTime, PathBuf)> = entries
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        if files.len() <= target.keep {
            return;
        }
        files.sort();
        for (_, old) in &files[..files.len() - target.keep] {
            let _ = fs::remove_file(old);
        }
    }
}

fn day_of(time: SystemTime) -> String {
    LocalDateTime::from_system_time(time).short_date()
}

struct Logger {
    sender: Sender<Message>,
    folder: PathBuf,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Starts logging to `logs_dir`. Later calls change nothing: logs stay in one folder for the whole run, whatever the profile.
pub fn init(logs_dir: &Path) {
    LOGGER.get_or_init(|| {
        let _ = fs::create_dir_all(logs_dir);
        let (sender, receiver) = channel::<Message>();
        let writer = Writer { folder: logs_dir.to_path_buf() };
        let _ = std::thread::Builder::new().name("log writer".to_string()).spawn(move || {
            for message in receiver {
                match message {
                    Message::Line { target, level, text, day } => writer.write(target, level, &text, &day),
                    Message::Flush(done) => {
                        let _ = done.send(());
                    }
                }
            }
        });
        Logger { sender, folder: logs_dir.to_path_buf() }
    });
}

/// Where logs are going, once [`init`] has run.
pub fn logs_folder() -> Option<PathBuf> {
    LOGGER.get().map(|l| l.folder.clone())
}

/// Logs a line under one of the original's logger names.
pub fn log(level: LogLevel, logger: &str, message: &str, error: Option<&str>) {
    #[cfg(test)]
    CAPTURED.with(|lines| lines.borrow_mut().push(format!("{}|{logger}|{message}", level.name())));
    let Some(state) = LOGGER.get() else {
        return;
    };
    let Some(target) = route(logger, level) else {
        return;
    };
    let now = LocalDateTime::now();
    let text = format_line(&now, logger, message, error);
    let _ = state.sender.send(Message::Line { target, level, text, day: now.short_date() });
}

#[cfg(test)]
thread_local! {
    static CAPTURED: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// What this thread has logged since the last call, as `LEVEL|logger|message`, so a test can check the lines a path writes.
#[cfg(test)]
pub(crate) fn take_captured() -> Vec<String> {
    CAPTURED.with(|lines| std::mem::take(&mut *lines.borrow_mut()))
}

pub fn info(logger: &str, message: &str) {
    log(LogLevel::Info, logger, message, None);
}

pub fn warn(logger: &str, message: &str) {
    log(LogLevel::Warn, logger, message, None);
}

pub fn error(logger: &str, message: &str, error: Option<&str>) {
    log(LogLevel::Error, logger, message, error);
}

/// Waits for everything logged so far to reach the disk.
pub fn flush() {
    let Some(state) = LOGGER.get() else {
        return;
    };
    let (done, wait) = channel();
    if state.sender.send(Message::Flush(done)).is_ok() {
        let _ = wait.recv_timeout(std::time::Duration::from_secs(2));
    }
}

/// `di.GetFiles()`: the files directly in the folder, in NTFS's order, which compares names upper-cased. A missing folder gives none, where
/// the original would throw. Hidden files, such as the `.DS_Store` Finder leaves, aren't logs.
pub fn list_log_files(logs_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(logs_dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort_by_cached_key(|name| (name.to_uppercase(), name.clone()));
    names
}

struct CrashState {
    lock_path: Option<PathBuf>,
    is_saving: Option<Box<dyn Fn() -> bool + Send + Sync>>,
}

static CRASH: Mutex<CrashState> = Mutex::new(CrashState { lock_path: None, is_saving: None });

/// What the crash path needs. Set the lock as soon as it's taken, and the save check once the core exists.
pub fn set_crash_lock(lock_path: Option<PathBuf>) {
    CRASH.lock().lock_path = lock_path;
}

pub fn set_crash_is_saving(is_saving: Box<dyn Fn() -> bool + Send + Sync>) {
    CRASH.lock().is_saving = Some(is_saving);
}

/// Logs Rust panics as the original's fatal handler does. Unlike that handler it doesn't end the process: a panic caught further up, in
/// one controller's thread or one effect's render, must not take a running show down. A panic nothing catches ends its thread, or
/// reaches Swift through UniFFI and stops the app there, and then the lock left behind reads as stale at the next launch.
pub fn install_crash_hooks() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            log(LogLevel::Fatal, PROGRAM_LOGGER, FATAL_PREFIX, Some(&info.to_string()));
            flush();
            previous(info);
        }));
    });
}

/// `LogMessageAndExit`, for an exception the app can't survive: logs, gives a save in progress 5 seconds, removes the lock, exits.
pub fn fatal_exit(message: &str) -> ! {
    log(LogLevel::Fatal, PROGRAM_LOGGER, FATAL_PREFIX, Some(message));
    let saving = CRASH.lock().is_saving.as_ref().is_some_and(|f| f());
    if saving {
        log(
            LogLevel::Fatal,
            PROGRAM_LOGGER,
            "Save was in progress during the fatal crash. Trying to pause 5 seconds to give it a chance to complete.",
            None,
        );
        flush();
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    flush();
    if let Some(path) = &CRASH.lock().lock_path {
        vixen_io::lock::remove_lock_file(path);
    }
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-logging-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_rules_route_as_nlog_config_does() {
        assert_eq!(route("Scheduler", LogLevel::Debug), Some(SCHEDULE));
        assert_eq!(route("Microsoft.Foo", LogLevel::Info), None);
        assert_eq!(route("Microsoft.Foo", LogLevel::Debug), None);
        assert_eq!(route("Microsoft.Foo", LogLevel::Warn), Some(LOGS));
        assert_eq!(route("Microsoft.Hosting.Lifetime", LogLevel::Info), Some(LOGS));
        assert_eq!(route("Microsoft.Hosting.Lifetime", LogLevel::Debug), None);
        // Below its rule's Warn, a request line falls through to the catch-all, as NLog's per-level `final` does.
        assert_eq!(route("VixenModules.App.WebServer.RequestLoggingMiddleware", LogLevel::Info), Some(LOGS));
        assert_eq!(route("VixenModules.App.WebServer.RequestLoggingMiddleware", LogLevel::Debug), None);
        assert_eq!(route("VixenModules.App.WebServer.RequestLoggingMiddleware", LogLevel::Warn), Some(WEB_ACCESS));
        assert_eq!(route("VixenModules.App.WebServer.RequestLoggingMiddleware", LogLevel::Fatal), Some(WEB_ACCESS));
        assert_eq!(route("VixenModules.SequenceType.Vixen2x", LogLevel::Info), Some(LOGS), "the wildcard needs the dot");
        assert_eq!(route("Scheduler.Other", LogLevel::Debug), None);
        assert_eq!(route("VixenModules.SequenceType.Vixen2x.Importer", LogLevel::Info), Some(V2_CONVERT));
        assert_eq!(route("VixenModules.SequenceType.Vixen2x.Importer", LogLevel::Debug), None);
        assert_eq!(route("Vixen.Sys.VixenSystem", LogLevel::Debug), None);
        assert_eq!(route("Vixen.Sys.VixenSystem", LogLevel::Info), Some(LOGS));
        assert_eq!(route("Vixen.Sys.VixenSystem", LogLevel::Fatal), Some(LOGS));
    }

    #[test]
    fn the_line_format_matches_a_real_line() {
        let time = LocalDateTime { year: 2025, month: 12, day: 20, hour: 17, minute: 43, second: 19, millisecond: 588 };
        assert_eq!(
            format_line(&time, "Vixen.Sys.VixenSystem", "Vixen System starting up...", None),
            "12/20/2025 17:43:19.588 [Vixen.Sys.VixenSystem] Vixen System starting up... "
        );
        assert_eq!(format_line(&time, "L", "m", Some("boom")), "12/20/2025 17:43:19.588 [L] m boom");
        // `Logging.Fatal(ex, ErrorMsg)`: the exception follows the message after the layout's space.
        assert_eq!(
            format_line(&time, PROGRAM_LOGGER, FATAL_PREFIX, Some("Oops")),
            "12/20/2025 17:43:19.588 [VixenApplication.Program] An application error occurred. Please contact the Vixen Dev Team with the following information:\n\n Oops"
        );
    }

    #[test]
    fn levels_get_their_own_files() {
        let folder = temp_folder("levels");
        let writer = Writer { folder: folder.clone() };
        let today = LocalDateTime::now().short_date();
        writer.write(SCHEDULE, LogLevel::Debug, "a line", &today);
        writer.write(LOGS, LogLevel::Warn, "w1", &today);
        writer.write(LOGS, LogLevel::Warn, "w2", &today);
        assert_eq!(fs::read_to_string(folder.join("Scheduler.Debug.log")).unwrap(), "a line\n");
        assert_eq!(fs::read_to_string(folder.join("Application.Warn.log")).unwrap(), "w1\nw2\n");
        assert_eq!(list_log_files(&folder), ["Application.Warn.log", "Scheduler.Debug.log"]);
    }

    #[test]
    fn above_a_million_bytes_rolls_to_the_archive_folder() {
        let folder = temp_folder("roll");
        let writer = Writer { folder: folder.clone() };
        let today = LocalDateTime::now().short_date();
        let big = "x".repeat(ARCHIVE_ABOVE_BYTES as usize);
        writer.write(LOGS, LogLevel::Info, &big, &today);
        writer.write(LOGS, LogLevel::Info, "next", &today);
        assert_eq!(fs::read_to_string(folder.join("Application.Info.log")).unwrap(), "next\n");
        let archived = folder.join(ARCHIVE_FOLDER).join(format!("Application.Info.{today}.0.log"));
        assert_eq!(fs::metadata(archived).unwrap().len(), ARCHIVE_ABOVE_BYTES + 1);
    }

    #[test]
    fn a_new_day_rolls_too_and_numbers_count_up() {
        let folder = temp_folder("day");
        let writer = Writer { folder: folder.clone() };
        let today = LocalDateTime::now().short_date();
        writer.write(LOGS, LogLevel::Info, "old", &today);
        writer.write(LOGS, LogLevel::Info, "new", "2099-01-01");
        assert_eq!(fs::read_to_string(folder.join("Application.Info.log")).unwrap(), "new\n");
        assert!(folder.join(ARCHIVE_FOLDER).join(format!("Application.Info.{today}.0.log")).exists());
    }

    #[test]
    fn the_web_access_log_archives_under_application_and_keeps_five() {
        let folder = temp_folder("prune");
        let writer = Writer { folder: folder.clone() };
        let today = LocalDateTime::now().short_date();
        let big = "x".repeat(ARCHIVE_ABOVE_BYTES as usize);
        for _ in 0..7 {
            writer.write(WEB_ACCESS, LogLevel::Warn, &big, &today);
        }
        let names = list_log_files(&folder.join(ARCHIVE_FOLDER));
        assert!(names.len() <= 5, "{names:?}");
        assert!(names.iter().all(|n| n.starts_with("Application.Warn.")), "{names:?}");
    }

    #[test]
    fn listing_is_by_name_and_leaves_out_subfolders_and_a_missing_folder_gives_none() {
        let folder = temp_folder("list");
        for name in ["Scheduler.Info.log", "Application.Warn.log", "Application.Error.log"] {
            fs::write(folder.join(name), "").unwrap();
        }
        fs::create_dir_all(folder.join(ARCHIVE_FOLDER)).unwrap();
        assert_eq!(list_log_files(&folder), ["Application.Error.log", "Application.Warn.log", "Scheduler.Info.log"]);
        assert!(list_log_files(&folder.join("missing")).is_empty());
    }

    #[test]
    fn listing_ignores_case_as_ntfs_does_and_skips_hidden_files() {
        let folder = temp_folder("list-case");
        for name in ["Scheduler.Info.log", "nlog-internal.log", "Application.Info.log", ".DS_Store", "_old.log"] {
            fs::write(folder.join(name), "").unwrap();
        }
        // NTFS compares upper-cased names, so `_` (after `Z`) sorts after the letters.
        assert_eq!(list_log_files(&folder), ["Application.Info.log", "nlog-internal.log", "Scheduler.Info.log", "_old.log"]);
    }

    #[test]
    fn logging_before_init_goes_nowhere_and_flush_returns() {
        // Whichever test runs first, neither call may block or panic.
        log(LogLevel::Info, "Test", "nothing listens", None);
        flush();
    }
}
