use super::*;
use crate::modules::rds::{RdsConfig, RdsModule};
use crate::modules::{ControllerConfig, OutputModule};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Fixture {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    http: String,
    other: String,
    https: String,
    bad_hostname: String,
    ca: String,
}
impl Fixture {
    fn new() -> Self {
        let mut child = Command::new("python3")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/rds_http.py"
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let values: Vec<_> = line.trim().splitn(4, ' ').collect();
        if values.len() != 4 {
            let _ = child.kill();
            let _ = child.wait();
            panic!("fixture failed: {line}");
        }
        Self {
            http: format!("http://127.0.0.1:{}", values[0]),
            other: format!("http://127.0.0.1:{}", values[1]),
            https: format!("https://localhost:{}", values[2]),
            bad_hostname: format!("https://127.0.0.1:{}", values[2]),
            ca: values[3].into(),
            child,
            input,
            output,
        }
    }
    fn command(&mut self, command: &str) -> String {
        writeln!(self.input, "{command}").unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        line
    }
    fn get(&self, url: &str, credentials: bool) -> Result<(u16, String), Error> {
        get_with_options(
            url,
            credentials.then_some(("user", "pass")),
            &AtomicBool::new(false),
            None,
            Options {
                ca_file: Some(&self.ca),
                timeout_ms: 3000,
                ..Default::default()
            },
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // A graceful pipe command removes the private key/cert directory. Bound
        // cleanup even if a failed fixture is stuck in a socket read.
        let _ = writeln!(self.input, "quit");
        let _ = self.input.flush();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn http_and_https_match_httpclient_redirect_auth_and_status_rules() {
    let mut f = Fixture::new();
    assert_eq!(
        f.get(&format!("{}/ok", f.http), false).unwrap(),
        (200, "OK".into())
    );
    assert_eq!(
        f.get(&format!("{}/ok", f.https), false).unwrap(),
        (200, "OK".into())
    );
    assert_eq!(
        f.get(&format!("{}/status?code=418", f.https), false)
            .unwrap()
            .0,
        418
    );
    // Neither a private CA nor a wrong hostname is accepted by production defaults.
    assert!(matches!(
        get(
            &format!("{}/ok", f.https),
            None,
            &AtomicBool::new(false),
            None
        ),
        Err(Error::Transport(_))
    ));
    assert!(matches!(
        f.get(&format!("{}/ok", f.bad_hostname), false),
        Err(Error::Transport(_))
    ));
    f.command("clear");
    assert_eq!(f.get(&format!("{}/auth", f.https), true).unwrap().0, 200);
    let requests = f.command("requests");
    assert_eq!(
        requests.matches("\"authorization\": null").count(),
        1,
        "{requests}"
    );
    assert_eq!(
        requests.matches("Basic dXNlcjpwYXNz").count(),
        1,
        "{requests}"
    );
    assert_eq!(
        requests.matches("\"path\": \"/auth\"").count(),
        2,
        "{requests}"
    );
    for destination in [
        format!("{}/auth", f.http),
        format!("{}/auth", f.other),
        format!("{}/auth", f.https),
    ] {
        f.command("clear");
        let url = format!(
            "{}/redirect?auth=1&to={}",
            f.http,
            crate::modules::rds::url_encode(&destination)
        );
        assert_eq!(f.get(&url, true).unwrap().0, 401);
        let requests = f.command("requests");
        assert_eq!(
            requests.matches("\"authorization\": null").count(),
            2,
            "{requests}"
        );
        assert_eq!(
            requests.matches("Basic dXNlcjpwYXNz").count(),
            1,
            "{requests}"
        );
        assert_eq!(
            requests.matches("\"path\": \"/auth\"").count(),
            1,
            "{requests}"
        );
    }
    f.command("clear");
    assert_eq!(f.get(&format!("{}/digest", f.https), true).unwrap().0, 200);
    let requests = f.command("requests");
    assert_eq!(requests.matches("\"authorization\": null").count(), 1);
    assert_eq!(
        requests.matches("Digest username=").count(),
        1,
        "{requests}"
    );
    assert!(!requests.contains("Basic"));
    assert_eq!(f.get(&format!("{}/cookie", f.https), false).unwrap().0, 200);
    assert_eq!(
        f.get(&format!("{}/cookie-check", f.https), false)
            .unwrap()
            .0,
        403,
        "cookies must not survive a new HttpClient"
    );
    // Relative URI resolution, supported redirect statuses, and HTTP→HTTPS.
    for status in [300, 301, 302, 303, 307, 308] {
        assert_eq!(
            f.get(&format!("{}/redirect?status={status}&to=ok", f.http), false)
                .unwrap()
                .0,
            200
        );
    }
    for status in [304, 305, 306] {
        assert_eq!(
            f.get(&format!("{}/redirect?status={status}&to=ok", f.http), false)
                .unwrap()
                .0,
            status
        );
    }
    f.command("clear");
    let userinfo = f.http.replace("http://", "http://ignored:secret@");
    assert_eq!(f.get(&format!("{userinfo}/auth"), false).unwrap().0, 401);
    assert!(!f.command("requests").contains("Basic"));
    assert_eq!(f.get(&format!("{userinfo}/auth"), true).unwrap().0, 200);
    assert!(!f.command("requests").contains("ignored"));
    assert!(matches!(
        f.get(
            &format!("{}/redirect?to=file%3a%2f%2f%2fetc%2fhosts", f.http),
            false
        ),
        Err(Error::Transport(_))
    ));
    assert_eq!(
        f.get(
            &format!(
                "{}/redirect?to={}",
                f.http,
                crate::modules::rds::url_encode(&format!("{}/ok", f.https))
            ),
            false
        )
        .unwrap()
        .0,
        200
    );
    f.command("clear");
    assert_eq!(
        f.get(
            &format!(
                "{}/redirect?to={}",
                f.https,
                crate::modules::rds::url_encode(&format!("{}/ok", f.http))
            ),
            false
        )
        .unwrap()
        .0,
        302
    );
    assert_eq!(f.command("requests").matches("\"path\":").count(), 1);
    f.command("clear");
    assert_eq!(f.get(&format!("{}/loop", f.http), false).unwrap().0, 302);
    assert_eq!(f.command("requests").matches("\"path\":").count(), 51);
    assert!(matches!(
        f.get("file:///etc/hosts", false),
        Err(Error::Transport(_))
    ));
    assert!(matches!(
        f.get("http://invalid\0host/", false),
        Err(Error::Transport(_))
    ));
}

#[test]
fn requests_are_bounded_and_controller_stop_joins_then_can_restart() {
    let mut f = Fixture::new();
    for (path, expected) in [("large", "body"), ("headers", "header")] {
        assert!(
            matches!(f.get(&format!("{}/{path}", f.https), false), Err(Error::Transport(s)) if s.contains(expected))
        );
    }
    let start = Instant::now();
    assert!(matches!(
        get_with_options(
            &format!("{}/slow", f.http),
            None,
            &AtomicBool::new(false),
            None,
            Options {
                timeout_ms: 150,
                ..Default::default()
            }
        ),
        Err(Error::Transport(_))
    ));
    assert!(start.elapsed() < Duration::from_secs(2));
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let url = format!("{}/slow", f.https);
    let ca = f.ca.clone();
    let worker = std::thread::spawn(move || {
        get_with_options(
            &url,
            None,
            &worker_stop,
            None,
            Options {
                ca_file: Some(&ca),
                ..Default::default()
            },
        )
    });
    std::thread::sleep(Duration::from_millis(150));
    let start = Instant::now();
    stop.store(true, Ordering::Relaxed);
    assert_eq!(worker.join().unwrap(), Err(Error::Cancelled));
    assert!(start.elapsed() < Duration::from_secs(1));
    f.command("clear");
    let mut module = RdsModule::new(RdsConfig {
        http_url: Some(format!("{}/slow", f.http)),
        ..Default::default()
    });
    module.start();
    assert!(module.send_test("module cancellation").unwrap());
    let deadline = Instant::now() + Duration::from_secs(2);
    while !f.command("requests").contains("/slow") {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    let start = Instant::now();
    module.stop();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(
        module.shared().error_count(),
        0,
        "intentional stop must not report a transport failure"
    );
    assert!(module.workers.lock().is_empty());
    module
        .set_config(ControllerConfig::Rds(RdsConfig {
            http_url: Some(format!("{}/ok", f.http)),
            ..Default::default()
        }))
        .unwrap();
    module.start();
    assert!(module.send_test("after restart").unwrap());
    let deadline = Instant::now() + Duration::from_secs(2);
    while !f.command("requests").contains("/ok") {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    module.stop();
    assert_eq!(module.shared().error_count(), 0);
}
