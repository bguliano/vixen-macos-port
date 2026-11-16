//! The client against a stand-in device on 127.0.0.1: nothing here leaves the machine.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use vixen_fpp::{Cancel, DirectUpload, FppClient, FppError, Options};

#[derive(Debug, Clone, Default)]
struct Seen {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

enum Answer {
    Json(u16, &'static str),
    Chunked(&'static str),
    UntilClose(&'static str),
    Silence,
}

struct Device {
    port: u16,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Device {
    /// Answers each request in turn with what `answer` says for it, on a thread that ends with the test process.
    fn start(answer: impl Fn(&Seen) -> Answer + Send + 'static) -> Device {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = seen.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let Some(request) = read_request(&mut stream) else { continue };
                record.lock().unwrap().push(request.clone());
                match answer(&request) {
                    Answer::Json(status, body) => {
                        let _ = write!(stream, "HTTP/1.1 {status} Whatever\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    }
                    Answer::Chunked(body) => {
                        let (a, b) = body.split_at(body.len() / 2);
                        let _ = write!(stream, "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{a}\r\n", a.len());
                        let _ = stream.flush();
                        std::thread::sleep(Duration::from_millis(30));
                        let _ = write!(stream, "{:X};ext=1\r\n{b}\r\n0\r\n\r\n", b.len());
                    }
                    Answer::UntilClose(body) => {
                        let _ = write!(stream, "HTTP/1.0 200 OK\r\n\r\n{body}");
                    }
                    Answer::Silence => {
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_secs(5));
                            drop(stream);
                        });
                    }
                }
            }
        });
        Device { port, seen }
    }

    fn client(&self) -> FppClient {
        FppClient::new(&format!("http://127.0.0.1:{}", self.port), Options::default()).unwrap()
    }

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }

    fn last(&self) -> Seen {
        self.seen().last().cloned().expect("a request arrived")
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Seen> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 16384];
    let head_end = loop {
        if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
        match stream.read(&mut chunk).ok()? {
            0 => return None,
            n => data.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&data[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let mut seen = Seen { method: first.next()?.to_string(), path: first.next()?.to_string(), ..Seen::default() };
    seen.headers = lines.filter_map(|l| l.split_once(':')).map(|(n, v)| (n.trim().to_string(), v.trim().to_string())).collect();
    let length: usize = seen.header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut body = data[head_end + 4..].to_vec();
    while body.len() < length {
        match stream.read(&mut chunk).ok()? {
            0 => return None,
            n => body.extend_from_slice(&chunk[..n]),
        }
    }
    seen.body = body;
    Some(seen)
}

fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vixen-fpp-loopback-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn message<T: std::fmt::Debug>(result: Result<T, FppError>) -> String {
    result.unwrap_err().to_string()
}

#[test]
fn system_information_comes_from_the_api_root() {
    let device = Device::start(|_| Answer::Json(200, r#"{"HostName":"FPP","Platform":"Raspberry Pi","majorVersion":8,"Utilization":{"CPU":1.5,"Memory":20,"Uptime":61000}}"#));
    let info = device.client().system_info(&Cancel::new()).unwrap().unwrap();
    assert_eq!((info.host_name.as_str(), info.platform.as_str(), info.major_version), ("FPP", "Raspberry Pi", 8));
    assert_eq!(info.utilization.unwrap().uptime, "00:01:01");
    let seen = device.last();
    assert_eq!((seen.method.as_str(), seen.path.as_str()), ("GET", "/api/system/info"));
    assert_eq!(seen.header("host"), Some(format!("127.0.0.1:{}", device.port).as_str()));
    assert_eq!(seen.header("connection"), Some("close"));
    assert!(seen.header("content-length").is_none());
}

#[test]
fn a_base_address_may_carry_a_path_and_a_trailing_slash() {
    let device = Device::start(|_| Answer::Json(200, r#"["Carol","Bells"]"#));
    for base in [format!("http://127.0.0.1:{}/player/", device.port), format!("HTTP://127.0.0.1:{}/player", device.port)] {
        let client = FppClient::new(&base, Options::default()).unwrap();
        assert_eq!(client.sequences(&Cancel::new()).unwrap(), ["Carol", "Bells"]);
        assert_eq!(device.last().path, "/player/api/sequence");
    }
    for bad in ["ftp://fpp.local/", "fpp.local", "http://", "http://host:port/", "http://user@host/"] {
        assert!(matches!(FppClient::new(bad, Options::default()), Err(FppError::Argument(_))), "{bad}");
    }
    assert!(FppClient::new("https://fpp.local/", Options::default()).is_ok());
    assert!(FppClient::for_host("::1").is_ok());
    assert!(FppClient::for_host("fpp.local").is_ok());
}

#[test]
fn lists_may_be_null_and_bodies_may_come_chunked_or_until_the_close() {
    let device = Device::start(|seen| match seen.path.as_str() {
        "/api/sequence" => Answer::Json(200, "null"),
        "/api/playlists" => Answer::Chunked(r#"["Main Show","Test"]"#),
        _ => Answer::UntilClose(r#"{"name":"Main Show","mainPlaylist":[{"type":"both","sequenceName":"Carol.fseq"},{"type":"pause"}],"leadOut":[{"type":"sequence","sequenceName":"Out.fseq"}]}"#),
    });
    let client = device.client();
    assert_eq!(client.sequences(&Cancel::new()).unwrap(), Vec::<String>::new());
    assert_eq!(client.playlists(&Cancel::new()).unwrap(), ["Main Show", "Test"]);
    assert_eq!(client.playlist_sequences("Main Show/1", &Cancel::new()).unwrap(), ["Carol.fseq", "Out.fseq"]);
    assert_eq!(device.last().path, "/api/playlist/Main%20Show%2F1");
}

#[test]
fn a_failed_request_carries_its_status_and_bad_json_is_an_error() {
    let device = Device::start(|seen| if seen.path.ends_with("info") { Answer::Json(200, "<html>") } else { Answer::Json(404, "{}") });
    let client = device.client();
    let missing = client.playlist("Nope", &Cancel::new()).unwrap_err();
    assert_eq!(missing.to_string(), "Request to 'playlist/Nope' failed with HTTP 404.");
    assert_eq!(missing.status(), Some(404));
    assert!(matches!(client.system_info(&Cancel::new()), Err(FppError::Response(_))));
}

#[test]
fn an_upload_sends_the_file_as_the_body() {
    let device = Device::start(|seen| match seen.path.as_str() {
        "/api/file/sequences/Song%201.fseq" => Answer::Json(200, r#"{"Status":"OK","file":"Song 1.fseq","dir":"sequences"}"#),
        "/api/file/music/Song.mp3" => Answer::Json(200, r#"{"status":"Failed"}"#),
        "/api/file/videos/clip.mp4" => Answer::Json(201, "saved"),
        _ => Answer::Json(500, "{}"),
    });
    let client = device.client();
    let bytes: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    let file = temp_file("upload.bin", &bytes);
    let cancel = Cancel::new();

    assert_eq!(client.upload_sequence("Song 1.fseq", &file, &cancel).unwrap().warning, None);
    let seen = device.last();
    assert_eq!(seen.method, "POST");
    assert_eq!(seen.header("content-type"), Some("application/octet-stream"));
    assert_eq!(seen.header("content-length"), Some("300000"));
    assert_eq!(seen.body, bytes);

    assert_eq!(client.upload_music("Song.mp3", &file, &cancel).unwrap().warning.as_deref(), Some("Upload of Song.mp3 returned non-OK status: Failed"));
    assert_eq!(client.upload_video("clip.mp4", &file, &cancel).unwrap().warning.as_deref(), Some("Could not parse upload response body for clip.mp4; treating as success"));

    let failed = client.upload_file("config", "co-universes.json", &file, &cancel).unwrap_err();
    assert_eq!(failed.to_string(), "Upload of 'co-universes.json' to 'config' failed with HTTP 500.");
    assert_eq!(failed.status(), Some(500));

    let before = device.seen().len();
    assert!(matches!(client.upload_file(" ", "a", &file, &cancel), Err(FppError::Argument(_))));
    assert!(matches!(client.upload_file("config", "", &file, &cancel), Err(FppError::Argument(_))));
    assert!(matches!(client.upload_sequence("a.fseq", &file.with_extension("gone"), &cancel), Err(FppError::Io(_))));
    assert_eq!(device.seen().len(), before, "nothing is sent for a bad argument or a missing file");
}

#[test]
fn an_espixelstick_takes_files_at_its_root_and_names_what_it_stored() {
    let device = Device::start(|seen| match seen.path.as_str() {
        "/fpp?path=uploadFile&filename=Song%201.fseq" => Answer::Json(200, r#"{"Name":"Song 1.fseq","Length":4}"#),
        "/fpp?path=uploadFile&filename=other.fseq" => Answer::Json(200, r#"{"Name":"OTHER.fseq"}"#),
        "/fpp?path=uploadFile&filename=lower.fseq" => Answer::Json(200, r#"{"name":"lower.fseq"}"#),
        "/fpp?path=uploadFile&filename=bad.zip" => Answer::Json(200, "not json"),
        _ => Answer::Json(503, ""),
    });
    let client = FppClient::new(&format!("http://127.0.0.1:{}/ignored/", device.port), Options::default()).unwrap();
    let file = temp_file("esp.bin", b"FSEQ");
    let cancel = Cancel::new();
    client.upload_esp_sequence("Song 1.fseq", &file, &cancel).unwrap();
    assert_eq!(device.last().body, b"FSEQ");
    assert_eq!(message(client.upload_esp_sequence("other.fseq", &file, &cancel)), "ESPixelStick sequence upload of 'other.fseq' returned unexpected file metadata.");
    assert_eq!(message(client.upload_esp_sequence("lower.fseq", &file, &cancel)), "ESPixelStick sequence upload of 'lower.fseq' returned unexpected file metadata.");
    assert_eq!(message(client.upload_esp_archive("bad.zip", &file, &cancel)), "ESPixelStick archive upload of 'bad.zip' returned invalid JSON.");
    let failed = client.upload_esp_archive("down.zip", &file, &cancel).unwrap_err();
    assert_eq!(failed.to_string(), "ESPixelStick archive upload of 'down.zip' failed with HTTP 503.");
    assert_eq!(failed.status(), Some(503));
    let before = device.seen().len();
    assert_eq!(message(client.upload_esp_sequence("sub/a.fseq", &file, &cancel)), "The filename must not contain path separators.");
    assert_eq!(message(client.upload_esp_sequence("sub\\a.fseq", &file, &cancel)), "The filename must not contain path separators.");
    assert_eq!(device.seen().len(), before);
}

#[test]
fn a_reboot_must_be_acknowledged() {
    let answers = Arc::new(Mutex::new(vec![Answer::Json(500, "{}"), Answer::Json(200, "Rebooting"), Answer::Json(200, r#"{"status":"rebooting"}"#), Answer::Json(200, r#"{"status":"Rebooting"}"#)]));
    let device = Device::start(move |_| answers.lock().unwrap().pop().unwrap());
    let client = device.client();
    let cancel = Cancel::new();
    client.reboot_esp(&cancel).unwrap();
    let seen = device.last();
    assert_eq!((seen.method.as_str(), seen.path.as_str(), seen.header("content-length")), ("POST", "/X6", Some("0")));
    assert_eq!(message(client.reboot_esp(&cancel)), "ESPixelStick reboot request returned an unexpected acknowledgement.");
    assert_eq!(message(client.reboot_esp(&cancel)), "ESPixelStick reboot request returned invalid JSON.");
    assert_eq!(message(client.reboot_esp(&cancel)), "ESPixelStick reboot request failed with HTTP 500.");
}

#[test]
fn a_rename_of_a_missing_file_is_fine_and_a_restart_may_be_quick() {
    let device = Device::start(|seen| match seen.path.as_str() {
        "/api/file/config/rename/co-universes.json/co-universes.json.bak%201" => Answer::Json(200, r#"{"Status":"OK"}"#),
        "/api/file/config/rename/gone.json/x" => Answer::Json(404, ""),
        "/api/system/fppd/restart" | "/api/system/fppd/restart?quick=1" => Answer::Json(200, "{}"),
        _ => Answer::Json(500, ""),
    });
    let client = device.client();
    let cancel = Cancel::new();
    client.rename_file("config", "co-universes.json", "co-universes.json.bak 1", &cancel).unwrap();
    let seen = device.last();
    assert_eq!((seen.method.as_str(), seen.header("content-length")), ("POST", Some("0")));
    client.rename_file("config", "gone.json", "x", &cancel).unwrap();
    assert_eq!(message(client.rename_file("config", "locked.json", "x", &cancel)), "Rename of 'locked.json' in 'config' failed with HTTP 500.");
    assert!(matches!(client.rename_file("config", " ", "x", &cancel), Err(FppError::Argument(_))));

    client.restart_fppd(false, &cancel).unwrap();
    assert_eq!((device.last().method.as_str(), device.last().path.as_str()), ("GET", "/api/system/fppd/restart"));
    client.restart_fppd(true, &cancel).unwrap();
    assert_eq!(device.last().path, "/api/system/fppd/restart?quick=1");

    let down = Device::start(|_| Answer::Json(502, ""));
    assert_eq!(message(down.client().restart_fppd(false, &cancel)), "FPPD restart failed with HTTP 502.");
}

#[test]
fn a_device_that_never_answers_times_out_and_a_cancel_stops_the_wait() {
    let device = Device::start(|_| Answer::Silence);
    let quick = FppClient::new(&format!("http://127.0.0.1:{}", device.port), Options { timeout: Duration::from_millis(400), upload_timeout: Duration::from_millis(400) }).unwrap();
    let started = Instant::now();
    assert_eq!(quick.system_info(&Cancel::new()), Err(FppError::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(3));

    let cancel = Cancel::new();
    let stopper = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        stopper.cancel();
    });
    let started = Instant::now();
    assert_eq!(device.client().system_info(&cancel), Err(FppError::Cancelled));
    assert!(started.elapsed() < Duration::from_secs(3));

    let before = device.seen().len();
    assert_eq!(device.client().sequences(&cancel), Err(FppError::Cancelled));
    assert_eq!(device.seen().len(), before, "a call that starts cancelled sends nothing");
}

#[test]
fn nothing_listening_is_an_io_error() {
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let client = FppClient::new(&format!("http://127.0.0.1:{port}/"), Options::default()).unwrap();
    assert!(matches!(client.system_info(&Cancel::new()), Err(FppError::Io(_))));
}

#[test]
fn detection_picks_the_routes() {
    let cancel = Cancel::new();
    let file = temp_file("route.bin", b"data");

    let player = Device::start(|seen| match seen.path.as_str() {
        "/api/system/info" => Answer::Json(200, r#"{"Platform":"Raspberry Pi","zip":true}"#),
        _ => Answer::Json(200, r#"{"Status":"OK"}"#),
    });
    let client = player.client();
    let route = DirectUpload::detect(&client, &cancel).unwrap();
    assert_eq!((route.is_esp_pixel_stick(), route.supports_zip_archives(), route.supports_fpp_extras()), (false, false, true));
    let mut points = Vec::new();
    route.upload_sequence_file(&file, "Carol.fseq", &mut |text, percent| points.push((text.to_string(), percent)), &cancel).unwrap();
    assert_eq!(player.last().path, "/api/file/sequences/Carol.fseq");
    assert_eq!(points, [("Uploading Carol.fseq".to_string(), 0), ("Uploaded Carol.fseq".to_string(), 100)]);
    points.clear();
    route.upload_audio_file(&file, "Carol.mp3", &mut |text, percent| points.push((text.to_string(), percent)), &cancel).unwrap();
    assert_eq!(player.last().path, "/api/file/music/Carol.mp3");
    assert_eq!(points, [("Uploading Carol.mp3".to_string(), 0), ("Uploaded Carol.mp3".to_string(), 100)]);
    route.backup_universe_file("co-universes.json.bak", &cancel).unwrap();
    assert_eq!(player.last().path, "/api/file/config/rename/co-universes.json/co-universes.json.bak");
    route.upload_universe_file(&file, &cancel).unwrap();
    assert_eq!(player.last().path, "/api/file/config/co-universes.json");
    route.restart_fppd(true, &cancel).unwrap();
    assert_eq!(player.last().path, "/api/system/fppd/restart?quick=1");
    let before = player.seen().len();
    assert_eq!(message(route.upload_archive_file(&file, "a.zip", &cancel)), "The detected device does not support ZIP archives.");
    assert_eq!(player.seen().len(), before);

    let stick = Device::start(|seen| match seen.path.as_str() {
        "/api/system/info" => Answer::Json(200, r#"{"Platform":"ESPixelStick","Zip":true}"#),
        "/X6" => Answer::Json(200, r#"{"status":"Rebooting"}"#),
        "/fpp?path=uploadFile&filename=Carol.fseq" => Answer::Json(200, r#"{"Name":"Carol.fseq"}"#),
        _ => Answer::Json(200, r#"{"Name":"batch.zip"}"#),
    });
    let client = stick.client();
    let route = DirectUpload::detect(&client, &cancel).unwrap();
    assert_eq!((route.is_esp_pixel_stick(), route.supports_zip_archives(), route.supports_fpp_extras()), (true, true, false));
    route.upload_sequence_file(&file, "Carol.fseq", &mut |_, _| {}, &cancel).unwrap();
    assert_eq!(stick.last().path, "/fpp?path=uploadFile&filename=Carol.fseq");
    route.upload_archive_file(&file, "batch.zip", &cancel).unwrap();
    assert_eq!(stick.last().path, "/fpp?path=uploadFile&filename=batch.zip");
    route.reboot_esp_pixel_stick(&cancel).unwrap();

    let plain = Device::start(|_| Answer::Json(200, r#"{"Platform":"ESPixelStick"}"#));
    let client = plain.client();
    let route = DirectUpload::detect(&client, &cancel).unwrap();
    assert_eq!((route.is_esp_pixel_stick(), route.supports_zip_archives()), (true, false));

    let wrong_case = Device::start(|_| Answer::Json(200, r#"{"Platform":"espixelstick","Zip":true}"#));
    let client = wrong_case.client();
    assert!(!DirectUpload::detect(&client, &cancel).unwrap().is_esp_pixel_stick());

    let nothing = Device::start(|_| Answer::Json(200, "null"));
    let client = nothing.client();
    assert_eq!(message(DirectUpload::detect(&client, &cancel).map(|_| ())), "The device did not provide valid system information.");
}
