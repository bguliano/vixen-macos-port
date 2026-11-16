//! Native launches retain an owned helper and a private control socket. In
//! particular, `open` is only a window launcher; it is never the process to stop.
use super::actions::{split_command_line, Done};
use crate::{logging, shows::ShowItem};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) fn application(item: &ShowItem) -> bool {
    let path = item.launch_program_name.as_deref().unwrap_or("");
    path.trim_end_matches('/')
        .to_ascii_lowercase()
        .ends_with(".app")
        && Path::new(path).is_dir()
}

pub(super) fn needed(item: &ShowItem) -> bool {
    application(item) || item.launch_show_command_window
}

fn helper_path() -> io::Result<PathBuf> {
    if let Some(contents) = std::env::current_exe()?.parent().and_then(Path::parent) {
        let bundled = contents.join("Helpers/vixen-launch-helper");
        if bundled.is_file() {
            return Ok(bundled);
        }
        if contents.file_name().is_some_and(|name| name == "Contents") {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "The application launch helper is missing from this bundle",
            ));
        }
    }
    // Cargo tests and standalone developer tools use the matching build output.
    // App-bundle builds leave this out, so the build machine's path isn't shipped.
    #[cfg(all(target_os = "macos", not(vixen_app_bundle)))]
    {
        Ok(PathBuf::from(env!("VIXEN_LAUNCH_HELPER")))
    }
    #[cfg(all(target_os = "macos", vixen_app_bundle))]
    {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The application launch helper is missing from this bundle",
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Native launches require macOS",
        ))
    }
}

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Prepared {
    directory: Directory,
    listener: UnixListener,
    child: Child,
    direct: bool,
    application: bool,
    started: bool,
    token: String,
}

fn prepare(item: &ShowItem, direct_terminal: bool) -> io::Result<Prepared> {
    let helper = helper_path()?;
    prepare_with(item, direct_terminal, &helper, |_| {})
}

fn prepare_with(
    item: &ShowItem,
    direct_terminal: bool,
    helper: &Path,
    edit_request: impl FnOnce(&mut serde_json::Value),
) -> io::Result<Prepared> {
    let token = uuid::Uuid::new_v4().to_string();
    // A short path also fits sockaddr_un on macOS, unlike some TMPDIR values.
    let path = PathBuf::from("/private/tmp").join(format!("vixen-launch-{token}"));
    std::fs::DirBuilder::new().mode(0o700).create(&path)?;
    let directory = Directory(path);
    let listener = UnixListener::bind(directory.0.join("control.sock"))?;
    listener.set_nonblocking(true)?;
    let is_app = application(item);
    let program = item.launch_program_name.as_deref().unwrap_or("");
    let arguments = split_command_line(item.launch_command_line.as_deref().unwrap_or(""));
    if program.contains('\0') || arguments.iter().any(|arg| arg.contains('\0')) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Program arguments contain a NUL byte",
        ));
    }
    let mut request = serde_json::json!({
        "token": token, "application": is_app,
        "program": program, "arguments": arguments,
    });
    edit_request(&mut request);
    std::fs::write(
        directory.0.join("request.json"),
        serde_json::to_vec(&request)?,
    )?;
    let direct = is_app || direct_terminal;
    let mut command = if is_app {
        let mut command = Command::new(helper);
        command.arg(&directory.0);
        command
    } else {
        let script = directory.0.join("launch.command");
        let quote = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"));
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nexec {} {}\n",
                quote(&helper),
                quote(&directory.0)
            ),
        )?;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700))?;
        // Tests execute this exact wrapper directly, without opening Terminal.
        let mut command = Command::new(if direct_terminal {
            "/bin/sh"
        } else {
            "/usr/bin/open"
        });
        command.arg(script);
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = command.spawn()?;
    Ok(Prepared {
        directory,
        listener,
        child,
        direct,
        application: is_app,
        started: false,
        token,
    })
}

#[cfg(test)]
mod tests;

fn read_hello(stream: &mut UnixStream, token: &str) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut hello = Vec::new();
    while hello.len() <= 64 {
        let mut byte = [0];
        if stream.read(&mut byte)? == 0 {
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        hello.push(byte[0]);
    }
    if hello != token.as_bytes() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Invalid launch helper handshake",
        ));
    }
    stream.set_read_timeout(Some(Duration::from_millis(50)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))
}

enum Completion {
    Finished(String),
    Pending(String),
}

fn monitor(
    prepared: &mut Prepared,
    stop: &AtomicBool,
    waiting: bool,
    done: &Done,
) -> io::Result<Completion> {
    let started = Instant::now();
    let mut stream = loop {
        if stop.load(Ordering::SeqCst) {
            return Ok(Completion::Finished(String::new()));
        }
        match prepared.listener.accept() {
            Ok((mut stream, _)) => {
                read_hello(&mut stream, &prepared.token)?;
                break stream;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error),
        }
        if let Some(status) = prepared.child.try_wait()? {
            if prepared.direct || !status.success() {
                return Err(io::Error::other(format!(
                    "Application launch helper exited before starting ({status})"
                )));
            }
        }
        if started.elapsed() > Duration::from_secs(30) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Application launch helper did not connect",
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if stop.load(Ordering::SeqCst) {
        stream.write_all(b"STOP\n")?;
        return Ok(Completion::Finished(String::new()));
    }
    stream.write_all(b"START\n")?;
    prepared.started = true;
    let mut stopped_at = None;
    let mut pending = Vec::new();
    loop {
        if stop.load(Ordering::SeqCst) && stopped_at.is_none() {
            stream.write_all(b"STOP\n")?;
            stopped_at = Some(Instant::now());
        }
        // Launch Services can complete asynchronously after cancellation; allow
        // the helper's bounded launch + owned-instance termination to finish.
        if stopped_at.is_some_and(|at| at.elapsed() > Duration::from_secs(37)) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Application launch did not finish stopping",
            ));
        }
        let mut bytes = [0; 1024];
        match stream.read(&mut bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Application launch helper disconnected",
                ))
            }
            Ok(n) => pending.extend_from_slice(&bytes[..n]),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                continue
            }
            Err(error) => return Err(error),
        }
        if pending.len() > 8192 {
            return Err(io::Error::other("Invalid launch helper response"));
        }
        while let Some(end) = pending.iter().position(|b| *b == b'\n') {
            let value: serde_json::Value = serde_json::from_slice(&pending[..end])?;
            pending.drain(..=end);
            match value.get("event").and_then(|v| v.as_str()) {
                Some("started") if !waiting => done.complete(),
                Some("started") => {}
                Some("done") => {
                    return Ok(Completion::Finished(
                        value
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    ))
                }
                Some("pending") => {
                    return Ok(Completion::Pending(
                        value
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Application launch is pending cancellation")
                            .to_string(),
                    ))
                }
                _ => return Err(io::Error::other("Invalid launch helper event")),
            }
        }
    }
}

pub(super) fn launch(
    item: &ShowItem,
    result: Box<dyn FnOnce(String) + Send>,
    done: Done,
    stop: Arc<AtomicBool>,
) {
    launch_with_mode(item, result, done, stop, false)
}

fn launch_with_mode(
    item: &ShowItem,
    result: Box<dyn FnOnce(String) + Send>,
    done: Done,
    stop: Arc<AtomicBool>,
    direct_terminal: bool,
) {
    let prepared = match prepare(item, direct_terminal) {
        Ok(prepared) => prepared,
        Err(error) => {
            result(error.to_string());
            done.complete();
            return;
        }
    };
    let waiting = item.launch_wait_for_exit;
    run_prepared(prepared, waiting, result, done, stop);
}

fn run_prepared(
    mut prepared: Prepared,
    waiting: bool,
    result: Box<dyn FnOnce(String) + Send>,
    done: Done,
    stop: Arc<AtomicBool>,
) {
    std::thread::Builder::new()
        .name("native-show-launch".into())
        .spawn(move || {
            let (message, retained_guard) = match monitor(&mut prepared, &stop, waiting, &done) {
                Ok(Completion::Finished(message)) => (message, false),
                Ok(Completion::Pending(message)) => (message, true),
                Err(error) => (error.to_string(), prepared.application && prepared.started),
            };
            // A live helper treats a lost socket as cancellation. Wait for its exit
            // before removing private files; `open` itself has generally exited.
            let Prepared {
                directory,
                listener,
                mut child,
                ..
            } = prepared;
            drop(listener); // abort helpers that connected just as Stop was requested
            if retained_guard {
                // The socket's EOF marks the helper cancelled. Report promptly, then
                // this detached worker becomes its reaper. Never kill a guard before
                // Launch Services delivers a potentially late application instance.
                logging::error(
                    "Scheduler",
                    "An error occurred during launch event.",
                    Some(&message),
                );
                result(message);
                done.complete();
                let _ = child.wait();
                drop(directory);
                return;
            }
            let deadline = Instant::now() + Duration::from_secs(6);
            while child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
            drop(directory);
            if !message.is_empty() {
                logging::error(
                    "Scheduler",
                    "An error occurred during launch event.",
                    Some(&message),
                );
            }
            result(message);
            done.complete();
        })
        .expect("native launch thread");
}
