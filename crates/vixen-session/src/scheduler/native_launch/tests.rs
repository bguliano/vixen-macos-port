use super::*;
use crate::shows::ShowItemType;
use std::sync::mpsc::{self, Receiver};

const WAIT: Duration = Duration::from_secs(12);

struct Fixture {
    root: PathBuf,
    stop: Arc<AtomicBool>,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("vixen-native-launch-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        Self {
            root,
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
    fn script(&self, body: &str) -> PathBuf {
        let path = self.root.join("program.sh");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn item(&self, path: &Path, arguments: &str, waiting: bool) -> ShowItem {
        let mut item = ShowItem::new(ShowItemType::Startup, "");
        item.launch_program_name = Some(path.to_string_lossy().into_owned());
        item.launch_command_line = Some(arguments.into());
        item.launch_wait_for_exit = waiting;
        item.launch_show_command_window = true;
        item
    }
    fn run(&self, item: &ShowItem) -> (Receiver<String>, Receiver<()>) {
        let (tx, result) = mpsc::channel();
        let (end, done) = mpsc::channel();
        let end = parking_lot::Mutex::new(end);
        launch_with_mode(
            item,
            Box::new(move |text| {
                let _ = tx.send(text);
            }),
            Done::new(move || {
                let _ = end.lock().send(());
            }),
            self.stop.clone(),
            true,
        );
        (result, done)
    }
    fn wait_file(&self, name: &str) -> String {
        let path = self.root.join(name);
        let end = Instant::now() + WAIT;
        while Instant::now() < end {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if !text.is_empty() {
                    return text;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("{} was not created", path.display());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn terminal_wrapper_stops_the_owned_process_group() {
    let f = Fixture::new();
    let pidfile = f.root.join("pid");
    let late = f.root.join("late");
    let script = f.script(&format!(
        "echo $$ > '{}'; (sleep 0.8; touch '{}') & wait",
        pidfile.display(),
        late.display()
    ));
    let (result, done) = f.run(&f.item(&script, "", true));
    let pid: i32 = f.wait_file("pid").trim().parse().unwrap();
    assert!(done.try_recv().is_err());
    f.stop.store(true, Ordering::SeqCst);
    assert_eq!(result.recv_timeout(WAIT).unwrap(), "");
    done.recv_timeout(WAIT).unwrap();
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "owned direct child survived Stop"
    );
    std::thread::sleep(Duration::from_millis(850));
    assert!(!late.exists(), "owned descendant survived Stop");
    assert_eq!(done.try_iter().count(), 0);
}

#[test]
fn terminal_wrapper_preserves_argv_without_shell_expansion() {
    let f = Fixture::new();
    let output = f.root.join("args");
    let injected = f.root.join("must-not-exist");
    let script = f.script(&format!("printf '%s\\n' \"$@\" > '{}'", output.display()));
    let arguments = format!(
        "\"two words\" \"$(touch {})\" \"it's\" \"\"",
        injected.display()
    );
    let (result, done) = f.run(&f.item(&script, &arguments, true));
    done.recv_timeout(WAIT).unwrap();
    assert_eq!(result.recv_timeout(WAIT).unwrap(), "");
    assert_eq!(
        std::fs::read_to_string(output).unwrap(),
        format!("two words\n$(touch {})\nit's\n\n", injected.display())
    );
    assert!(!injected.exists());
}

#[test]
fn stopping_before_handshake_does_not_start_the_program() {
    let f = Fixture::new();
    let touched = f.root.join("started");
    let script = f.script(&format!("touch '{}'", touched.display()));
    f.stop.store(true, Ordering::SeqCst);
    let (result, done) = f.run(&f.item(&script, "", true));
    done.recv_timeout(WAIT).unwrap();
    assert_eq!(result.recv_timeout(WAIT).unwrap(), "");
    assert!(!touched.exists());
}

#[test]
fn non_waited_native_program_completes_on_start_and_tester_can_still_stop_it() {
    let f = Fixture::new();
    let script = f.script(&format!(
        "echo $$ > '{}'; exec /bin/sleep 600",
        f.root.join("pid").display()
    ));
    let (result, done) = f.run(&f.item(&script, "", false));
    done.recv_timeout(WAIT).unwrap();
    let pid: i32 = f.wait_file("pid").trim().parse().unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    assert!(result.try_recv().is_err());
    // Program Output Window Close is unconditional, unlike scheduler Stop for a
    // non-waited completed action. Both plain/native paths share Launched::stop.
    f.stop.store(true, Ordering::SeqCst);
    assert_eq!(result.recv_timeout(WAIT).unwrap(), "");
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(done.try_iter().count(), 0);
}

#[test]
fn lost_owner_socket_stops_the_program_and_private_files_are_removed() {
    let f = Fixture::new();
    let script = f.script(&format!(
        "echo $$ > '{}'; exec /bin/sleep 600",
        f.root.join("pid").display()
    ));
    let mut prepared = prepare(&f.item(&script, "", true), true).unwrap();
    let directory = prepared.directory.0.clone();
    let end = Instant::now() + WAIT;
    let mut stream = loop {
        match prepared.listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < end => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(error) => panic!("{error}"),
        }
    };
    read_hello(&mut stream, &prepared.token).unwrap();
    stream.write_all(b"START\n").unwrap();
    let pid: i32 = f.wait_file("pid").trim().parse().unwrap();
    drop(stream);
    while prepared.child.try_wait().unwrap().is_none() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(prepared.child.try_wait().unwrap().is_some());
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert!(
        !directory.exists(),
        "helper left private files after losing its owner"
    );
    drop(prepared);
    assert!(!directory.exists());
}

#[test]
fn native_launch_failure_reports_error_once() {
    let f = Fixture::new();
    let (result, done) = f.run(&f.item(&f.root.join("missing"), "", true));
    assert!(!result.recv_timeout(WAIT).unwrap().is_empty());
    done.recv_timeout(WAIT).unwrap();
    assert_eq!(done.try_iter().count(), 0);
}

#[test]
fn stop_kills_the_owned_direct_child_after_it_changes_groups() {
    let f = Fixture::new();
    let source = f.root.join("change-group.c");
    let program = f.root.join("change-group");
    std::fs::write(
        &source,
        r#"
#include <unistd.h>
#include <stdio.h>
int main(int argc, char **argv) {
    if (argc != 2 || setpgid(0, getpgid(getppid())) != 0) return 2;
    FILE *f = fopen(argv[1], "w"); if (!f) return 3;
    fprintf(f, "%d\n", getpid()); fclose(f);
    for (;;) pause();
}
"#,
    )
    .unwrap();
    assert!(Command::new("/usr/bin/clang")
        .arg(&source)
        .arg("-o")
        .arg(&program)
        .status()
        .unwrap()
        .success());
    let (result, done) = f.run(&f.item(
        &program,
        &format!("\"{}\"", f.root.join("pid").display()),
        true,
    ));
    let pid: i32 = f.wait_file("pid").trim().parse().unwrap();
    f.stop.store(true, Ordering::SeqCst);
    assert_eq!(result.recv_timeout(WAIT).unwrap(), "");
    done.recv_timeout(WAIT).unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
}

fn background_app(fixture: &Fixture) -> PathBuf {
    let bundle = fixture.root.join("OwnedBackground.app");
    let contents = bundle.join("Contents");
    std::fs::create_dir_all(contents.join("MacOS")).unwrap();
    let source = fixture.root.join("background.m");
    std::fs::write(&source, r#"
#import <AppKit/AppKit.h>
#include <unistd.h>
int main(int argc, const char **argv) {
 @autoreleasepool {
  [NSApplication sharedApplication];
  [NSApp setActivationPolicy:NSApplicationActivationPolicyProhibited];
  if (argc > 1) [[NSString stringWithFormat:@"%d\n", getpid()] writeToFile:[NSString stringWithUTF8String:argv[1]] atomically:YES encoding:NSUTF8StringEncoding error:nil];
  [NSApp run];
 }
 return 0;
}
"#).unwrap();
    std::fs::write(contents.join("Info.plist"), format!(r#"<?xml version="1.0"?><plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>org.vixen.test.{}</string><key>CFBundleExecutable</key><string>background</string>
<key>CFBundlePackageType</key><string>APPL</string><key>CFBundleName</key><string>OwnedBackground</string>
<key>LSBackgroundOnly</key><true/><key>LSUIElement</key><true/><key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>"#, uuid::Uuid::new_v4())).unwrap();
    let output = Command::new("/usr/bin/clang")
        .args(["-fobjc-arc", "-framework", "AppKit"])
        .arg(&source)
        .arg("-o")
        .arg(contents.join("MacOS/background"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-", "--timestamp=none"])
        .arg(&bundle)
        .status()
        .unwrap()
        .success());
    bundle
}

/// This is deliberately outside default cargo tests: it needs a logged-in macOS
/// Launch Services session. The only app launched is this generated background
/// fixture; it has no windows and cannot activate over the user's foreground app.
#[test]
#[ignore = "requires macOS Launch Services; run the generated background-app ownership probe explicitly"]
fn generated_background_apps_stop_only_the_owned_instance() {
    let first = Fixture::new();
    let second = Fixture::new();
    let bundle = background_app(&first);
    let (first_result, first_done) = first.run(&first.item(
        &bundle,
        &format!("\"{}\"", first.root.join("pid").display()),
        true,
    ));
    let first_pid: i32 = first.wait_file("pid").trim().parse().unwrap();
    let (second_result, second_done) = second.run(&second.item(
        &bundle,
        &format!("\"{}\"", second.root.join("pid").display()),
        true,
    ));
    let second_pid: i32 = second.wait_file("pid").trim().parse().unwrap();
    assert_ne!(
        first_pid, second_pid,
        "Launch Services reused an existing instance"
    );
    second.stop.store(true, Ordering::SeqCst);
    assert_eq!(second_result.recv_timeout(WAIT).unwrap(), "");
    second_done.recv_timeout(WAIT).unwrap();
    assert_eq!(unsafe { libc::kill(second_pid, 0) }, -1);
    assert_eq!(
        unsafe { libc::kill(first_pid, 0) },
        0,
        "stopping the new instance killed the earlier one"
    );
    assert!(first_done.try_recv().is_err());
    first.stop.store(true, Ordering::SeqCst);
    assert_eq!(first_result.recv_timeout(WAIT).unwrap(), "");
    first_done.recv_timeout(WAIT).unwrap();
    assert_eq!(unsafe { libc::kill(first_pid, 0) }, -1);
}

#[test]
#[ignore = "requires macOS Launch Services; deterministically delays a generated background-app callback"]
fn timed_out_launch_retains_its_guard_and_stops_the_late_owned_app() {
    let fixture = Fixture::new();
    let bundle = background_app(&fixture);
    let helper = fixture.root.join("test-launch-helper");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("native/launch_helper.m");
    let output = Command::new("/usr/bin/clang")
        .args([
            "-fobjc-arc",
            "-fblocks",
            "-DVIXEN_LAUNCH_TESTING",
            "-framework",
            "Foundation",
            "-framework",
            "AppKit",
        ])
        .arg(source)
        .arg("-o")
        .arg(&helper)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let observed = fixture.root.join("observed");
    let item = fixture.item(&bundle, "", true);
    let prepared = prepare_with(&item, true, &helper, |request| {
        request["testLaunchDelayMs"] = 1500.into();
        request["testTimeoutMs"] = 50.into();
        request["testObservedPid"] = observed.to_string_lossy().into_owned().into();
    })
    .unwrap();
    let private_directory = prepared.directory.0.clone();
    let (tx, result) = mpsc::channel();
    let (end, done) = mpsc::channel();
    let end = parking_lot::Mutex::new(end);
    let started = Instant::now();
    run_prepared(
        prepared,
        true,
        Box::new(move |text| {
            let _ = tx.send(text);
        }),
        Done::new(move || {
            let _ = end.lock().send(());
        }),
        fixture.stop.clone(),
    );
    let message = result.recv_timeout(WAIT).unwrap();
    done.recv_timeout(WAIT).unwrap();
    assert!(message.contains("still pending"), "{message}");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "caller waited for late callback"
    );
    assert!(!observed.exists(), "callback wasn't delayed");
    assert!(
        private_directory.exists(),
        "guard was discarded before callback"
    );
    fixture.stop.store(true, Ordering::SeqCst);
    let pid: i32 = fixture.wait_file("observed").trim().parse().unwrap();
    let end = Instant::now() + WAIT;
    while private_directory.exists() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !private_directory.exists(),
        "retired guard did not clean up"
    );
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "late application survived timeout and Stop"
    );
    assert_eq!(done.try_iter().count(), 0);
}
