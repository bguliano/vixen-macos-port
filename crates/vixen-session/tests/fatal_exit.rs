//! Fatal-path assertions run in disposable subprocesses: process::exit must never end the test runner.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use vixen_session::logging;

const CHILD_ROOT: &str = "VIXEN_FATAL_TEST_CHILD_ROOT";
const CHILD_MODE: &str = "VIXEN_FATAL_TEST_CHILD_MODE";

#[test]
fn fatal_exit_child() {
    let Some(root) = std::env::var_os(CHILD_ROOT).map(PathBuf::from) else {
        return;
    };
    let mode = std::env::var(CHILD_MODE).unwrap();
    logging::init(&root.join("logs"));
    let lock = vixen_io::lock::create_lock_file(&root).unwrap();
    logging::set_crash_lock(Some(lock));
    for i in 0..500 {
        logging::info("fatal-test", &format!("queued before fatal {i}"));
    }
    if mode == "caught-panic" {
        logging::install_crash_hooks();
        let caught = std::panic::catch_unwind(|| panic!("isolated render panic fixture"));
        assert!(caught.is_err());
        logging::info("fatal-test", "caught panic kept the process alive");
        logging::flush();
        return;
    }
    let saving = mode == "saving";
    logging::set_crash_is_saving(Box::new(move || saving));
    if saving {
        let root = root.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            fs::write(
                root.join("save-finished"),
                "private save completed during grace period",
            )
            .unwrap();
        });
    }
    logging::fatal_exit(&format!("fatal subprocess fixture: {mode}"));
}

#[test]
fn fatal_subprocess_flushes_waits_removes_lock_and_preserves_caught_panics() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vixen-fatal-{}-{nonce}", std::process::id()));
    for mode in ["idle", "saving", "caught-panic"] {
        let case = root.join(mode);
        fs::create_dir_all(&case).unwrap();
        let output = fs::File::create(case.join("child-output.txt")).unwrap();
        let started = Instant::now();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "fatal_exit_child", "--nocapture"])
            .env(CHILD_ROOT, &case)
            .env(CHILD_MODE, mode)
            .stdout(Stdio::from(output.try_clone().unwrap()))
            .stderr(Stdio::from(output))
            .spawn()
            .unwrap();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > Duration::from_secs(15) {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "fatal child {mode} did not exit; artifacts: {}",
                    case.display()
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let fatal = fs::read_to_string(case.join("logs/Application.Fatal.log")).unwrap();
        let info = fs::read_to_string(case.join("logs/Application.Info.log")).unwrap();
        assert!(
            info.contains("queued before fatal 499"),
            "queued log was not flushed: {mode}"
        );
        assert!(fatal.contains(logging::FATAL_PREFIX));
        if mode == "caught-panic" {
            assert!(status.success());
            assert!(fatal.contains("isolated render panic fixture"));
            assert!(info.contains("caught panic kept the process alive"));
            assert!(
                case.join(vixen_io::lock::LOCK_FILE).exists(),
                "caught panic must not release a live profile"
            );
        } else {
            assert_eq!(status.code(), Some(1));
            assert!(fatal.contains(&format!("fatal subprocess fixture: {mode}")));
            assert!(!case.join(vixen_io::lock::LOCK_FILE).exists());
            assert_eq!(
                fatal.contains("Save was in progress during the fatal crash"),
                mode == "saving"
            );
            if mode == "saving" {
                assert!(
                    started.elapsed() >= Duration::from_secs(5),
                    "save grace was skipped"
                );
                assert!(case.join("save-finished").exists());
            }
        }
    }
    fs::remove_dir_all(root).unwrap();
}
