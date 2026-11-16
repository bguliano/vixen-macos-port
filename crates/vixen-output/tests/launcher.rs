//! The launcher controller really starting a process. This has its own test binary because starting a process momentarily
//! duplicates every open descriptor, including another test's serial port and its advisory lock.

use std::sync::Arc;
use std::time::Duration;
use vixen_output::{OutputCommand, OutputModule};

#[test]
fn launcher_runs_its_command_once_per_change() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("launcher-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("run.sh");
    let log = dir.join("log.txt");
    std::fs::write(&script, format!("#!/bin/sh\necho \"$1\" >> '{}'\n", log.display())).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut m = vixen_output::modules::LauncherModule::new(vixen_output::modules::LauncherConfig::default());
    m.start();
    let command: Arc<str> = Arc::from(format!("Launcher|{},hello", script.display()));
    m.update(&[OutputCommand::Str(command.clone())]);
    m.update(&[OutputCommand::Str(command.clone())]);
    m.update(&[OutputCommand::Str(command)]);

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut contents = String::new();
    while std::time::Instant::now() < deadline {
        contents = std::fs::read_to_string(&log).unwrap_or_default();
        if !contents.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(contents, "hello\n", "{:?}", m.shared().last_error());
    m.stop();
    std::fs::remove_dir_all(&dir).ok();
}
