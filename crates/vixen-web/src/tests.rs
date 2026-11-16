use super::*;
use serde_json::json;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[derive(Default)]
struct FakeApi {
    calls: Mutex<Vec<(Action, ApiInput)>>,
    broadcasts: AtomicUsize,
}
impl WebApi for FakeApi {
    fn invoke(&self, action: Action, input: ApiInput) -> Result<ApiResponse, ApiError> {
        self.calls.lock().unwrap().push((action, input.clone()));
        if input.get("fail") == Some("argument") {
            return Err(ApiError::BadRequest("Invalid id.\r\nSecond line.".into()));
        }
        if input.get("fail") == Some("internal") {
            return Err(ApiError::Internal("test exception".into()));
        }
        if action == Action::Controller {
            return Ok(ApiResponse::NoContent);
        }
        Ok(ApiResponse::Json(
            json!({"Message":"OK","Details":[],"IsSuccessful":true}),
        ))
    }
    fn context_states(&self) -> Vec<serde_json::Value> {
        self.broadcasts.fetch_add(1, Ordering::Relaxed);
        vec![
            json!({"Message":null,"Details":[],"IsSuccessful":true,"State":1,"Sequence":{"Name":"test","FileName":"test.tim"},"Position":"00:00:01"}),
        ]
    }
}
struct Fixture {
    server: Server,
    root: PathBuf,
    api: Arc<FakeApi>,
    logs: Arc<Mutex<Vec<String>>>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("vixen-web-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("index.htm"), b"<title>Vixen test</title>").unwrap();
        let api = Arc::new(FakeApi::default());
        let logs = Arc::new(Mutex::new(Vec::new()));
        let out = logs.clone();
        let server = Server::start(
            "127.0.0.1:0".parse().unwrap(),
            root.clone(),
            api.clone(),
            Arc::new(move |line| out.lock().unwrap().push(line.to_owned())),
        )
        .unwrap();
        Self {
            server,
            root,
            api,
            logs,
        }
    }
    fn request(
        &self,
        method: &str,
        path: &str,
        headers: &str,
        body: &str,
    ) -> (u16, String, Vec<u8>) {
        request(self.server.address(), method, path, headers, body)
    }
    fn websocket(&self) -> BufReader<TcpStream> {
        let (status, _, body) = self.request(
            "POST",
            "/hubs/contextStates/negotiate?negotiateVersion=1",
            "",
            "",
        );
        assert_eq!(status, 200);
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            response["availableTransports"][0]["transport"],
            "WebSockets"
        );
        let token = response["connectionToken"].as_str().unwrap();
        let mut stream = TcpStream::connect(self.server.address()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(stream, "GET /hubs/contextStates?id={token} HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive, Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
        let mut reader = BufReader::new(stream);
        let mut header = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            header.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        assert!(header.starts_with("HTTP/1.1 101"));
        assert!(header.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
        reader
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.stop();
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    headers: &str,
    body: &str,
) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    let split = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    let header = String::from_utf8(response[..split].to_vec()).unwrap();
    (
        header.split_whitespace().nth(1).unwrap().parse().unwrap(),
        header,
        response[split..].to_vec(),
    )
}
fn client_frame(stream: &mut TcpStream, opcode: u8, fin: bool, body: &[u8]) {
    let mut wire = vec![opcode | if fin { 128 } else { 0 }];
    if body.len() < 126 {
        wire.push(128 | body.len() as u8);
    } else if body.len() <= 65535 {
        wire.push(254);
        wire.extend_from_slice(&(body.len() as u16).to_be_bytes());
    } else {
        wire.push(255);
        wire.extend_from_slice(&(body.len() as u64).to_be_bytes());
    }
    let mask = [1, 2, 3, 4];
    wire.extend_from_slice(&mask);
    wire.extend(body.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    stream.write_all(&wire).unwrap();
}
fn server_frame(reader: &mut BufReader<TcpStream>) -> (u8, Vec<u8>) {
    let mut header = [0u8; 2];
    reader.read_exact(&mut header).unwrap();
    assert_eq!(header[1] & 128, 0);
    let size = match header[1] & 127 {
        126 => {
            let mut size = [0u8; 2];
            reader.read_exact(&mut size).unwrap();
            u16::from_be_bytes(size) as usize
        }
        127 => {
            let mut size = [0u8; 8];
            reader.read_exact(&mut size).unwrap();
            u64::from_be_bytes(size) as usize
        }
        value => value as usize,
    };
    let mut body = vec![0u8; size];
    reader.read_exact(&mut body).unwrap();
    (header[0] & 15, body)
}

#[test]
fn every_controller_route_and_verb_matches() {
    let fixture = Fixture::new();
    use Action::*;
    let routes = [
        ("element/get", Elements, "PUT"),
        ("element/getElements", Elements, "GET"),
        ("element/searchElements", SearchElements, "GET"),
        ("element/getChildElements/a", ChildElements, "GET"),
        ("element/getParentElements/a", ParentElements, "GET"),
        ("element/on", ElementOn, "POST"),
        ("element/groupOn", GroupOn, "POST"),
        ("element/groupOff", GroupOff, "POST"),
        ("element/off", ElementOff, "POST"),
        ("element/clearAll", ClearElements, "POST"),
        ("element/effect", ElementEffect, "POST"),
        ("play/status", PlayStatus, "GET"),
        ("play/getSequences", Sequences, "DELETE"),
        ("play/playSequence", PlaySequence, "POST"),
        ("play/pauseSequence", PauseSequence, "POST"),
        ("play/stopSequence", StopSequence, "POST"),
        ("system/getControllers", Controllers, "GET"),
        ("system/get/id", Controller, "GET"),
        ("system/getController/id", Controller, "POST"),
        ("system/setControllerState", SetControllerState, "POST"),
        (
            "system/setAllControllersState",
            SetAllControllersState,
            "POST",
        ),
        ("system/save", SaveSystem, "POST"),
    ];
    for (path, action, method) in routes {
        let response = fixture.request(method, &format!("/API/{}", path.to_uppercase()), "", "");
        assert_eq!(
            response.0,
            if action == Controller { 204 } else { 200 },
            "{path}"
        );
        assert_eq!(fixture.api.calls.lock().unwrap().last().unwrap().0, action);
    }
    for path in [
        "/api/element",
        "/api/element/missing",
        "/api/play",
        "/api/system",
    ] {
        assert_eq!(fixture.request("GET", path, "", "").0, 404);
    }
    for path in [
        "/api/element/on",
        "/api/element/groupOn",
        "/api/element/off",
        "/api/play/playSequence",
        "/api/system/save",
    ] {
        assert_eq!(fixture.request("GET", path, "", "").0, 405);
    }
    assert_eq!(fixture.request("POST", "/api/play/status", "", "").0, 405);
    assert_eq!(
        fixture
            .request("POST", "/api/element/searchElements", "", "")
            .0,
        405
    );
}

#[test]
fn form_route_query_precedence_and_json_binding() {
    let fixture = Fixture::new();
    fixture.request(
        "POST",
        "/api/element/effect/route?id=query&Name=query&Options%5BMode%5D=2",
        "Content-Type: application/x-www-form-urlencoded; charset=UTF-8\r\n",
        "ID=form&name=Snow+%2B+Ice&Color%5B%23FF0000%5D=0.5&Options%5BMode%5D=1",
    );
    let calls = fixture.api.calls.lock().unwrap();
    let input = &calls.last().unwrap().1;
    assert_eq!(input.get("id"), Some("form"));
    assert_eq!(input.get("NAME"), Some("Snow + Ice"));
    assert_eq!(input.dictionary("color"), vec![("#FF0000", "0.5")]);
    assert_eq!(input.dictionary("Options"), vec![("Mode", "1")]);
    drop(calls);
    fixture.request(
        "POST",
        "/api/element/on?Intensity=3",
        "Content-Type: application/json\r\n",
        "{\"Intensity\":99}",
    );
    let calls = fixture.api.calls.lock().unwrap();
    let input = &calls.last().unwrap().1;
    assert_eq!(input.json, None);
    assert_eq!(input.get("intensity"), Some("3"));
    drop(calls);
    fixture.request(
        "POST",
        "/api/element/groupOn",
        "Content-Type: application/json\r\n",
        "[{\"Id\":\"abc\"}]",
    );
    assert_eq!(
        fixture.api.calls.lock().unwrap().last().unwrap().1.json,
        Some(json!([{"Id":"abc"}]))
    );
    fixture.request(
        "POST",
        "/api/element/groupOff",
        "Content-Type: application/json\r\n",
        "bad",
    );
    assert_eq!(
        fixture.api.calls.lock().unwrap().last().unwrap().1.json,
        None
    );
}

#[test]
fn error_negotiation_and_request_log() {
    let fixture = Fixture::new();
    let (status, header, body) = fixture.request(
        "GET",
        "/api/element/get?fail=argument",
        "Accept: application/json, */*; q=0.01\r\n",
        "",
    );
    assert_eq!(status, 400);
    assert!(header.contains("text/plain"));
    assert_eq!(body, b"Invalid id. Second line.");
    let (_, header, body) = fixture.request(
        "GET",
        "/api/element/get?fail=argument",
        "Accept: application/json\r\n",
        "",
    );
    assert!(header.contains("application/json"));
    assert_eq!(body, b"\"Invalid id. Second line.\"");
    let (status, _, body) = fixture.request("GET", "/api/element/get?fail=internal", "", "");
    assert_eq!(status, 500);
    assert!(body.is_empty());
    let logs = fixture.logs.lock().unwrap();
    assert!(logs
        .iter()
        .any(|line| line == "Exception in web server: test exception"));
    assert!(logs
        .iter()
        .any(|line| line == "127.0.0.1 - \"GET /api/element/get HTTP/1.1\" 400 24"));
}

#[test]
fn static_default_mime_head_and_traversal_guard() {
    let fixture = Fixture::new();
    fs::write(fixture.root.join("test.js"), "var x=1;").unwrap();
    let (status, header, body) = fixture.request("GET", "/", "", "");
    assert_eq!(status, 200);
    assert!(header.contains("text/html"));
    assert_eq!(body, b"<title>Vixen test</title>");
    let (status, header, body) = fixture.request("HEAD", "/test.js", "", "");
    assert_eq!(status, 200);
    assert!(header.contains("application/javascript"));
    assert!(body.is_empty());
    for path in [
        "/../outside",
        "/%2e%2e/outside",
        "/%2F..%2Foutside",
        "/%00",
        "/%5c..%5coutside",
    ] {
        assert_eq!(fixture.request("GET", path, "", "").0, 404);
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/hosts", fixture.root.join("escape")).unwrap();
        assert_eq!(fixture.request("GET", "/escape", "", "").0, 404);
    }
}

#[test]
fn signalr_handshake_fragmentation_ping_broadcast_and_shared_timer() {
    let fixture = Fixture::new();
    let mut first = fixture.websocket();
    let mut second = fixture.websocket();
    client_frame(first.get_mut(), 1, false, b"{\"protocol\":\"json\",");
    client_frame(first.get_mut(), 0, true, b"\"version\":1}\x1e");
    client_frame(
        second.get_mut(),
        1,
        true,
        b"{\"protocol\":\"json\",\"version\":1}\x1e",
    );
    assert_eq!(server_frame(&mut first), (1, b"{}\x1e".to_vec()));
    assert_eq!(server_frame(&mut second), (1, b"{}\x1e".to_vec()));
    client_frame(first.get_mut(), 9, true, b"test");
    assert_eq!(server_frame(&mut first), (10, b"test".to_vec()));
    client_frame(first.get_mut(), 1, true, b"{\"type\":6}\x1e");
    let (opcode, body) = server_frame(&mut first);
    assert_eq!(opcode, 1);
    assert_eq!(body.last(), Some(&0x1e));
    let record: serde_json::Value = serde_json::from_slice(&body[..body.len() - 1]).unwrap();
    assert_eq!(record["target"], "updatePlayingContextStates");
    assert_eq!(record["arguments"][0][0]["Sequence"]["Name"], "test");
    assert_eq!(server_frame(&mut second).1, body);
    assert_eq!(fixture.api.broadcasts.load(Ordering::Relaxed), 1);
    client_frame(first.get_mut(), 1, true, b"{\"type\":7}\x1e");
    assert_eq!(server_frame(&mut first).0, 8);
    client_frame(second.get_mut(), 8, true, &1000u16.to_be_bytes());
    assert_eq!(server_frame(&mut second).0, 8);
    std::thread::sleep(Duration::from_millis(1150));
    assert_eq!(fixture.api.broadcasts.load(Ordering::Relaxed), 1);
}

#[test]
fn websocket_rejects_unmasked_frames_and_invalid_handshake() {
    let fixture = Fixture::new();
    let mut stream = fixture.websocket();
    stream.get_mut().write_all(b"\x81\x02{}").unwrap();
    assert_eq!(
        server_frame(&mut stream),
        (8, 1002u16.to_be_bytes().to_vec())
    );
    let mut stream = fixture.websocket();
    client_frame(
        stream.get_mut(),
        1,
        true,
        b"{\"protocol\":\"messagepack\",\"version\":1}\x1e",
    );
    let (opcode, body) = server_frame(&mut stream);
    assert_eq!(opcode, 1);
    assert!(String::from_utf8(body)
        .unwrap()
        .contains("Unsupported protocol"));
    assert_eq!(fixture.request("GET","/hubs/contextStates?id=invalid","Connection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n","").0,404);
}

#[test]
fn bounded_requests_stop_join_and_immediate_rebind() {
    let mut fixture = Fixture::new();
    let mut stream = TcpStream::connect(fixture.server.address()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .write_all(b"POST /api/element/on HTTP/1.1\r\nContent-Length: 1048577\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 413"));
    let mut stalled = TcpStream::connect(fixture.server.address()).unwrap();
    stalled
        .write_all(b"POST /api/element/on HTTP/1.1\r\nContent-Length: 5\r\n\r\n")
        .unwrap();
    let mut ws = fixture.websocket();
    client_frame(
        ws.get_mut(),
        1,
        true,
        b"{\"protocol\":\"json\",\"version\":1}\x1e",
    );
    server_frame(&mut ws);
    let started = Instant::now();
    fixture.server.stop();
    assert!(started.elapsed() < Duration::from_secs(1));
    let listener = TcpListener::bind(fixture.server.address()).unwrap();
    drop(listener);
}

#[test]
fn chunked_forms_continue_and_conflicting_lengths() {
    let fixture = Fixture::new();
    let mut stream = TcpStream::connect(fixture.server.address()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream.write_all(b"POST /api/element/on HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nTransfer-Encoding: chunked\r\nExpect: 100-continue\r\n\r\n").unwrap();
    let mut interim = [0u8; 25];
    stream.read_exact(&mut interim).unwrap();
    assert_eq!(&interim, b"HTTP/1.1 100 Continue\r\n\r\n");
    stream
        .write_all(b"4\r\nid=a\r\nB;example=yes\r\n&Name=A%2BB\r\n0\r\nX-Trailer: value\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let calls = fixture.api.calls.lock().unwrap();
    let input = &calls.last().unwrap().1;
    assert_eq!(input.get("id"), Some("a"));
    assert_eq!(input.get("name"), Some("A+B"));
    drop(calls);
    let mut stream = TcpStream::connect(fixture.server.address()).unwrap();
    stream.write_all(b"POST /api/element/on HTTP/1.1\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 400"));
}

#[test]
fn websocket_close_and_frame_size_guards() {
    let fixture = Fixture::new();
    for (payload, expected) in [
        (vec![1], 1002u16),
        (1005u16.to_be_bytes().to_vec(), 1002),
        (vec![3, 232, 255], 1007),
    ] {
        let mut stream = fixture.websocket();
        client_frame(stream.get_mut(), 8, true, &payload);
        assert_eq!(
            server_frame(&mut stream),
            (8, expected.to_be_bytes().to_vec())
        );
    }
    let mut stream = fixture.websocket();
    stream
        .get_mut()
        .write_all(&[0x81, 0xff, 0, 0, 0, 0, 0, 0x10, 0, 1])
        .unwrap();
    assert_eq!(
        server_frame(&mut stream),
        (8, 1009u16.to_be_bytes().to_vec())
    );
    let mut stream = fixture.websocket();
    client_frame(stream.get_mut(), 9, false, b"fragmented ping");
    assert_eq!(
        server_frame(&mut stream),
        (8, 1002u16.to_be_bytes().to_vec())
    );
}

#[test]
fn bundled_assets_are_byte_identical_to_upstream() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let upstream = workspace.join("upstream/Vixen/src/Vixen.Modules/App/WebServer/wwwroot");
    let bundled = workspace.join("app/Resources/wwwroot");
    fn compare(a: &std::path::Path, b: &std::path::Path) -> usize {
        let mut count = 0;
        for entry in fs::read_dir(a).unwrap() {
            let entry = entry.unwrap();
            let other = b.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                count += compare(&entry.path(), &other);
            } else {
                assert_eq!(
                    fs::read(entry.path()).unwrap(),
                    fs::read(other).unwrap(),
                    "{:?}",
                    entry.path()
                );
                count += 1;
            }
        }
        count
    }
    assert!(compare(&upstream, &bundled) > 20);
    assert_eq!(compare(&upstream, &bundled), compare(&bundled, &upstream));
}

#[test]
fn chunk_metadata_limit_and_hex_grammar_apply_to_buffered_lines() {
    let fixture = Fixture::new();
    for chunk in [
        format!("1;{}\r\nx\r\n0\r\n\r\n", "a".repeat(2048)),
        "+1\r\nx\r\n0\r\n\r\n".into(),
    ] {
        let mut stream = TcpStream::connect(fixture.server.address()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(stream, "POST /api/element/on HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n{chunk}").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 400"), "{response}");
    }
    assert!(fixture.api.calls.lock().unwrap().is_empty());
}

#[test]
fn text_messages_require_utf8_but_continuation_frames_can_split_a_character() {
    let fixture = Fixture::new();
    let first = b"{\"protocol\":\"json\",\"version\":1,\"text\":\"\xc3";
    let last = b"\xa9\"}\x1e";
    // Repeated split writes exercise the close-with-input-in-flight TCP race.
    for _ in 0..16 {
        let mut invalid = fixture.websocket();
        client_frame(invalid.get_mut(), 1, true, first);
        client_frame(invalid.get_mut(), 1, true, last);
        assert_eq!(
            server_frame(&mut invalid),
            (8, 1007u16.to_be_bytes().to_vec())
        );
        let mut valid = fixture.websocket();
        client_frame(valid.get_mut(), 1, false, first);
        client_frame(valid.get_mut(), 0, true, last);
        assert_eq!(server_frame(&mut valid), (1, b"{}\x1e".to_vec()));
    }
}

#[test]
fn callback_can_release_its_own_server_without_self_join() {
    struct LastOwner {
        server: Mutex<Option<Server>>,
    }
    impl WebApi for LastOwner {
        fn invoke(&self, _: Action, _: ApiInput) -> Result<ApiResponse, ApiError> {
            let server = self.server.lock().unwrap().take();
            drop(server); // Models final AppCore drop on its request worker.
            Ok(ApiResponse::Json(json!({"closed": true})))
        }
        fn context_states(&self) -> Vec<serde_json::Value> {
            vec![]
        }
    }
    let f = Fixture::new();
    let api = Arc::new(LastOwner {
        server: Mutex::new(None),
    });
    let server = Server::start(
        "127.0.0.1:0".parse().unwrap(),
        f.root.clone(),
        api.clone(),
        Arc::new(|_| {}),
    )
    .unwrap();
    let address = server.address();
    *api.server.lock().unwrap() = Some(server);
    let begin = Instant::now();
    assert_eq!(
        request(address, "GET", "/api/element/getElements", "", "").0,
        200
    );
    assert!(begin.elapsed() < Duration::from_secs(2));
    let weak = Arc::downgrade(&api);
    drop(api);
    for _ in 0..100 {
        if weak.upgrade().is_none() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("shutdown reaper must release the API after its worker exits");
}

#[test]
fn wildcard_ipv6_listener_explicitly_accepts_both_ip_families() {
    let f = Fixture::new();
    let mut server = Server::start(
        "[::]:0".parse().unwrap(),
        f.root.clone(),
        Arc::new(FakeApi::default()),
        Arc::new(|_| {}),
    )
    .unwrap();
    let port = server.address().port();
    for host in ["127.0.0.1", "[::1]"] {
        assert_eq!(
            request(
                format!("{host}:{port}").parse().unwrap(),
                "GET",
                "/",
                "",
                ""
            )
            .0,
            200
        );
    }
    server.stop();
}
