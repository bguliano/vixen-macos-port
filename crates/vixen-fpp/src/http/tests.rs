//! Local HTTP/TLS tests exercise the same native transport as public FppClient.
//! A private CA is supplied directly here, never by profile/environment options.
use super::*;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

struct Server {
    child: Child,
    root: PathBuf,
    http: u16,
    tls: u16,
    ca: CString,
}
impl Server {
    fn start() -> Self {
        static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!("vixen-fpp-tls-{}-{}", std::process::id(), ID.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&root).unwrap();
        let mut child = Command::new("python3")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/transport_server.py"))
            .arg(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
        let config: Value = serde_json::from_str(&line).expect("local server ports");
        let ca = CString::new(root.join("cert.pem").to_str().unwrap()).unwrap();
        Self {
            child,
            root,
            http: config["http"].as_u64().unwrap() as u16,
            tls: config["tls"].as_u64().unwrap() as u16,
            ca,
        }
    }
    fn target(&self, tls: bool) -> Target {
        Target {
            cookies: CookieJar::new().unwrap(),
            loopback_only: true,
            https: tls,
            host: if tls { "localhost" } else { "127.0.0.1" }.into(),
            port: if tls { self.tls } else { self.http },
        }
    }
    fn request(&self, tls: bool, path: &str, body: Body<'_>, timeout: Duration, cancel: &Cancel) -> Result<Response, FppError> {
        let method = if matches!(body, Body::None) { "GET" } else { "POST" };
        send_with_ca(&self.target(tls), method, path, body, timeout, cancel, Some(&self.ca))
    }
    fn get(&self, tls: bool, path: &str) -> Response {
        self.request(tls, path, Body::None, Duration::from_secs(5), &Cancel::new()).unwrap()
    }
    fn requests(&self) -> Vec<Value> {
        serde_json::from_slice(&std::fs::read(self.root.join("requests.json")).unwrap()).unwrap()
    }
    fn upload(&self, size: usize) -> File {
        let path = self.root.join("upload.bin");
        let mut file = File::create(&path).unwrap();
        file.write_all(&vec![b'x'; size]).unwrap();
        drop(file);
        File::open(path).unwrap()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn json(response: Response) -> Value {
    assert_eq!(response.status, 200);
    serde_json::from_slice(&response.body).unwrap()
}

#[test]
fn redirects_match_httpclient_method_and_stream_replay_rules() {
    let server = Server::start();
    let timeout = Duration::from_secs(5);
    for status in [300, 301, 302, 303, 307, 308] {
        let route = format!("/redirect/{status}/1");
        let value = json(server.get(false, &route));
        assert_eq!(value["path"], "/final");
        assert_eq!(value["method"], "GET");
        let mut file = server.upload(300_123);
        let value = json(server.request(false, &route, Body::File(&mut file, 300_123), timeout, &Cancel::new()).unwrap());
        let preserve = matches!(status, 307 | 308);
        assert_eq!(value["method"], if preserve { "POST" } else { "GET" }, "{status}");
        assert_eq!(value["length"], if preserve { 300_123 } else { 0 }, "{status}");
        if preserve {
            let records = server.requests();
            let last = &records[records.len() - 3..];
            assert!(last.iter().all(|record| record["digest"] == value["digest"] && record["length"] == 300_123));
            assert_eq!(value["contentType"], "application/octet-stream");
        } else {
            assert!(value["contentType"].is_null());
        }
        let value = json(server.request(false, &route, Body::Empty, timeout, &Cancel::new()).unwrap());
        assert_eq!(value["method"], if preserve { "POST" } else { "GET" });
        assert_eq!(value["length"], 0);
    }
    for status in [304, 305, 306] {
        assert_eq!(server.get(false, &format!("/redirect/{status}/0")).status, status);
    }
    assert_eq!(json(server.get(false, "/relative"))["path"], "/final?relative=yes");
    assert_eq!(json(server.get(false, "/absolute"))["path"], "/final");
    assert_eq!(server.get(false, "/missing").status, 302);
    assert_eq!(server.get(false, "/file").status, 302);
    let before = server.requests().len();
    assert_eq!(server.get(false, "/redirect/302/100").status, 302);
    assert_eq!(server.requests().len() - before, 51, "50 followed redirects then the final response");
}

#[test]
fn tls_verifies_chain_and_hostname_and_never_downgrades_at_any_hop() {
    let server = Server::start();
    assert_eq!(json(server.get(true, "/final"))["server"], "tls");
    assert_eq!(json(server.get(false, "/tohttps"))["server"], "tls");
    assert_eq!(server.get(true, "/down").status, 302);
    let before = server.requests().len();
    assert_eq!(server.get(false, "/updown").status, 302);
    assert_eq!(server.requests().len() - before, 2, "HTTP→HTTPS→HTTP stops before downgrade");
    let cancel = Cancel::new();
    assert!(
        matches!(
            send(&server.target(true), "GET", "/final", Body::None, Duration::from_secs(5), &cancel),
            Err(FppError::Io(_))
        ),
        "an untrusted certificate must fail"
    );
    let wrong_host = Target {
        host: "127.0.0.1".into(),
        ..server.target(true)
    };
    assert!(
        matches!(
            send_with_ca(&wrong_host, "GET", "/final", Body::None, Duration::from_secs(5), &cancel, Some(&server.ca)),
            Err(FppError::Io(_))
        ),
        "a trusted certificate for a different hostname must fail"
    );
    let mut file = server.upload(300_123);
    let value = json(
        server
            .request(true, "/redirect/307/1", Body::File(&mut file, 300_123), Duration::from_secs(5), &cancel)
            .unwrap(),
    );
    assert_eq!(value["length"], 300_123);
}

#[test]
fn deadlines_cancellation_early_rejection_and_limits_cover_the_whole_transfer() {
    let server = Server::start();
    let cancel = Cancel::new();
    let started = Instant::now();
    assert!(matches!(
        server.request(false, "/redirect/302/10?delay=90", Body::None, Duration::from_millis(230), &cancel),
        Err(FppError::TimedOut)
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
    for (tls, path, size) in [(false, "/stall", 0), (true, "/stall", 0), (true, "/slow-upload", 16 * 1024 * 1024)] {
        let cancel = Cancel::new();
        let stop = cancel.clone();
        let mut file = server.upload(size);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            stop.cancel();
        });
        let started = Instant::now();
        let body = if size == 0 { Body::None } else { Body::File(&mut file, size as u64) };
        assert!(matches!(server.request(tls, path, body, Duration::from_secs(5), &cancel), Err(FppError::Cancelled)));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
    let mut file = server.upload(16 * 1024 * 1024);
    assert_eq!(
        server
            .request(false, "/early-reject", Body::File(&mut file, 16 * 1024 * 1024), Duration::from_secs(5), &cancel)
            .unwrap()
            .status,
        413
    );
    for path in ["/large", "/large-header"] {
        assert!(matches!(server.request(false, path, Body::None, Duration::from_secs(5), &cancel), Err(FppError::Io(_))));
    }
    let mut file = server.upload(20);
    assert!(
        matches!(
            server.request(false, "/final", Body::File(&mut file, 100), Duration::from_secs(5), &cancel),
            Err(FppError::Io(_))
        ),
        "a truncated file fails rather than silently ending the body"
    );
    assert!(matches!(server.request(false, "/final", Body::None, Duration::ZERO, &cancel), Err(FppError::TimedOut)));
}

#[test]
fn cookies_persist_for_one_client_across_redirects_calls_and_concurrent_requests() {
    let server = Server::start();
    let cancel = Cancel::new();
    let timeout = Duration::from_secs(5);
    let target = server.target(false);
    let call = |path| send_with_ca(&target, "GET", path, Body::None, timeout, &cancel, Some(&server.ca)).unwrap();
    assert_eq!(json(call("/cookie/start"))["cookie"], "ticket=redirect");
    assert_eq!(json(call("/final"))["cookie"], "ticket=redirect");
    assert!(json(server.get(false, "/final"))["cookie"].is_null(), "another client has its own jar");
    assert!(json(server.get(false, "/cookie/cross"))["cookie"].is_null(), "host-only cookies do not cross hostnames");

    let client = Arc::new(crate::FppClient::new(&format!("http://127.0.0.1:{}/", server.http), crate::Options::default()).unwrap());
    assert_eq!(client.system_info(&cancel).unwrap().unwrap().host_name, "no-cookie");
    assert_eq!(client.system_info(&cancel).unwrap().unwrap().host_name, "ticket=client");
    let mut workers = Vec::new();
    for _ in 0..8 {
        let client = client.clone();
        workers.push(std::thread::spawn(move || client.system_info(&Cancel::new()).unwrap().unwrap().host_name));
    }
    for worker in workers {
        assert_eq!(worker.join().unwrap(), "ticket=client");
    }
    let _file = server.upload(123);
    client.upload_sequence("cookie.fseq", &server.root.join("upload.bin"), &cancel).unwrap();
    assert_eq!(server.requests().last().unwrap()["cookie"], "ticket=client");
    let independent = crate::FppClient::new(&format!("http://127.0.0.1:{}/", server.http), crate::Options::default()).unwrap();
    assert_eq!(independent.system_info(&cancel).unwrap().unwrap().host_name, "no-cookie");
}

#[test]
fn ui_test_redirects_cannot_escape_the_loopback_boundary() {
    let server = Server::start();
    for path in ["/external", "/external-relative"] {
        let before = server.requests().len();
        let result = server.request(false, path, Body::None, Duration::from_secs(1), &Cancel::new());
        assert!(matches!(result, Err(FppError::Argument(ref message)) if message == "External devices are disabled in UI tests."));
        assert_eq!(server.requests().len() - before, 1, "only the allowed initial request was sent");
    }
    let target = Target {
        host: "192.0.2.1".into(),
        ..server.target(false)
    };
    assert!(matches!(
        send(&target, "GET", "/never", Body::None, Duration::from_secs(1), &Cancel::new()),
        Err(FppError::Argument(_))
    ));
}
