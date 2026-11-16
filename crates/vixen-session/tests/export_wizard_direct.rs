//! The Export Wizard's direct upload (F31 steps 1, 3 and 5, F34, F35) against a stand-in device on 127.0.0.1: nothing here leaves the
//! machine. The sequences are the test copy of the owner's show (`target/show-scheduler`); passes quietly on a machine without it.

use parking_lot::Mutex;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use vixen_session::app::AppCore;
use vixen_session::export_wizard::profile::{default_profile, ExportProfile};
use vixen_session::export_wizard::run::{start_bulk, BulkListener, BulkResult};

fn show_copy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/show-scheduler")
}

#[derive(Debug, Clone, Default)]
struct Seen {
    method: String,
    path: String,
    body: Vec<u8>,
}

struct Device {
    port: u16,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Device {
    /// Answers each request with the status and JSON `answer` gives it, on a thread that ends with the test process.
    fn start(answer: impl Fn(&Seen) -> (u16, String) + Send + 'static) -> Device {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = seen.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let Some(request) = read_request(&mut stream) else { continue };
                let (status, body) = answer(&request);
                record.lock().push(request);
                let _ = write!(stream, "HTTP/1.1 {status} Whatever\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        Device { port, seen }
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn requests(&self) -> Vec<String> {
        self.seen.lock().iter().map(|s| format!("{} {}", s.method, s.path)).collect()
    }

    fn body(&self, path: &str) -> Vec<u8> {
        self.seen.lock().iter().find(|s| s.path == path).map(|s| s.body.clone()).unwrap_or_else(|| panic!("no request for {path}"))
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Seen> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 65536];
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
    let length: usize = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0);
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

/// A Falcon Player that takes everything.
fn player(seen: &Seen) -> (u16, String) {
    match seen.path.as_str() {
        "/api/system/info" => (200, r#"{"HostName":"FPP","Platform":"Raspberry Pi","Variant":"Pi 4"}"#.to_string()),
        _ => (200, r#"{"Status":"OK"}"#.to_string()),
    }
}

/// An ESPixelStick, which answers an upload with the name it stored.
fn esp(zip: bool) -> impl Fn(&Seen) -> (u16, String) + Send + 'static {
    move |seen| match seen.path.as_str() {
        "/api/system/info" => (200, format!(r#"{{"HostName":"esps","Platform":"ESPixelStick","Zip":{zip}}}"#)),
        "/X6" => (200, r#"{"status":"Rebooting"}"#.to_string()),
        path => match path.strip_prefix("/fpp?path=uploadFile&filename=") {
            Some(name) => (200, format!(r#"{{"Name":"{}"}}"#, name.replace("%20", " "))),
            None => (404, String::new()),
        },
    }
}

#[derive(Default)]
struct Recorder {
    tasks: Mutex<Vec<String>>,
    overall: Mutex<Vec<(String, i32)>>,
    esp: Mutex<Vec<bool>>,
    done: Mutex<Option<mpsc::Sender<BulkResult>>>,
}

impl BulkListener for Recorder {
    fn task(&self, text: &str, _percent: i32) {
        let mut tasks = self.tasks.lock();
        if tasks.last().map(String::as_str) != Some(text) {
            tasks.push(text.to_string());
        }
    }
    fn overall(&self, text: &str, percent: i32) {
        self.overall.lock().push((text.to_string(), percent));
    }
    fn load_failed(&self, _path: &str, _message: &str) -> bool {
        true
    }
    fn esp_pixel_stick(&self, supports_zip: bool) {
        self.esp.lock().push(supports_zip);
    }
    fn finished(&self, result: BulkResult) {
        if let Some(tx) = self.done.lock().take() {
            let _ = tx.send(result);
        }
    }
}

fn run(core: &Arc<AppCore>, profile: ExportProfile) -> (BulkResult, Arc<Recorder>) {
    let (tx, rx) = mpsc::channel();
    let recorder = Arc::new(Recorder { done: Mutex::new(Some(tx)), ..Recorder::default() });
    let run = start_bulk(Arc::clone(core), profile, recorder.clone()).unwrap();
    let result = rx.recv_timeout(Duration::from_secs(600)).expect("the run finishes");
    run.join();
    (result, recorder)
}

fn leftovers() -> Vec<String> {
    std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("vixen-export-"))
        .collect()
}

/// One test, so the temporary folder can be checked for leftovers with nothing else writing there.
#[test]
fn a_direct_upload_sends_what_each_kind_of_device_takes() {
    let root = show_copy();
    if !root.is_dir() || !root.join("Media").is_dir() {
        return;
    }
    let before = leftovers();
    let core = Arc::new(AppCore::open(root.clone(), true).unwrap());
    let sequence = |name: &str| root.join("Sequence").join(format!("{name}.tim")).to_string_lossy().into_owned();
    let nowhere = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wizard-direct-unused");
    let _ = std::fs::remove_dir_all(&nowhere);
    let base = {
        let mut profile = default_profile("Direct", &root, &core.system.read());
        profile.interval = 25;
        profile.fpp_direct_upload = true;
        profile.include_audio = true;
        profile.rename_audio = true;
        profile.create_universe_file = true;
        profile.backup_universe_file = true;
        profile.falcon_output_folder = Some(nowhere.to_string_lossy().into_owned());
        profile.output_folder = Some(nowhere.join("sequences").to_string_lossy().into_owned());
        profile.audio_output_folder = Some(nowhere.join("music").to_string_lossy().into_owned());
        profile.sequence_files = vec![sequence("Christmas Intro 1")];
        profile
    };
    assert!(base.is_falcon_2x_format(), "the default profile is 2.6+");

    // A Falcon Player: the sequence, its audio under the sequence's name, the old universe file moved aside, the new one, a restart.
    let device = Device::start(player);
    let mut profile = base.clone();
    profile.fpp_host_address = Some(device.host());
    let (result, recorder) = run(&core, profile);
    assert_eq!((result.error.clone(), result.upload_error.clone()), (None, None));
    assert_eq!(result.final_text, "Completed");
    assert!(result.exported.is_empty() && result.universe_file.is_none(), "nothing is kept on this machine");
    assert_eq!(result.uploaded, ["Christmas Intro 1.fseq", "Christmas Intro 1.mp3", "config/co-universes.json"]);
    let requests = device.requests();
    assert_eq!(requests.len(), 6, "{requests:?}");
    assert_eq!(requests[0], "GET /api/system/info");
    assert_eq!(requests[1], "POST /api/file/sequences/Christmas%20Intro%201.fseq");
    assert_eq!(requests[2], "POST /api/file/music/Christmas%20Intro%201.mp3");
    assert!(requests[3].starts_with("POST /api/file/config/rename/co-universes.json/co-universes.json_"), "{requests:?}");
    assert_eq!(requests[4], "POST /api/file/config/co-universes.json");
    assert_eq!(requests[5], "GET /api/system/fppd/restart");
    let fseq = device.body("/api/file/sequences/Christmas%20Intro%201.fseq");
    assert_eq!(&fseq[..4], b"PSEQ");
    assert!(fseq.len() > 1000);
    let audio = device.body("/api/file/music/Christmas%20Intro%201.mp3");
    assert!(audio.len() > 100_000 && std::fs::read_dir(root.join("Media")).unwrap().flatten().any(|e| std::fs::read(e.path()).is_ok_and(|bytes| bytes == audio)));
    assert!(String::from_utf8_lossy(&device.body("/api/file/config/co-universes.json")).contains("\"universes\""));
    assert!(recorder.esp.lock().is_empty());
    let tasks = recorder.tasks.lock().clone();
    for wanted in ["Exporting Christmas Intro 1", "Uploading Christmas Intro 1.fseq", "Uploaded Christmas Intro 1.fseq", "Uploading Christmas Intro 1.mp3"] {
        assert!(tasks.iter().any(|t| t == wanted), "{wanted} in {tasks:?}");
    }
    let overall = recorder.overall.lock().clone();
    assert_eq!(overall, [("Overall Progress", 0), ("Overall Progress", 50), ("Overall Progress", 100), ("Completed", 100)].map(|(t, p)| (t.to_string(), p)));
    assert!(!nowhere.exists(), "the profile's folders aren't touched");

    // With neither universe box ticked and no audio wanted, only the sequence goes. The audio keeps its own name unless renamed.
    let device = Device::start(player);
    let mut profile = base.clone();
    profile.fpp_host_address = Some(device.host());
    profile.create_universe_file = false;
    profile.backup_universe_file = false;
    profile.rename_audio = false;
    let (result, _) = run(&core, profile);
    assert_eq!(result.final_text, "Completed");
    let requests = device.requests();
    assert_eq!(requests.len(), 3, "{requests:?}");
    assert!(requests[2].starts_with("POST /api/file/music/") && requests[2] != "POST /api/file/music/Christmas%20Intro%201.mp3", "{requests:?}");

    // An ESPixelStick without ZIP support: each sequence on its own, and nothing else.
    let device = Device::start(esp(false));
    let mut profile = base.clone();
    profile.fpp_host_address = Some(device.host());
    let (result, recorder) = run(&core, profile);
    assert_eq!((result.final_text.as_str(), result.upload_error.clone()), ("Completed", None));
    assert_eq!(device.requests(), ["GET /api/system/info", "POST /fpp?path=uploadFile&filename=Christmas%20Intro%201.fseq"]);
    assert_eq!(result.uploaded, ["Christmas Intro 1.fseq"]);
    assert_eq!(*recorder.esp.lock(), [false]);

    // An ESPixelStick with ZIP support: archives of five as they fill, the rest at the end, then one reboot.
    let device = Device::start(esp(true));
    let mut profile = base.clone();
    profile.fpp_host_address = Some(device.host());
    let names = ["Christmas Intro 1", "Christmas Canon", "start", "led strip test", "The Night Before Christmas", "Carol of the Bells"];
    profile.sequence_files = names.iter().map(|n| sequence(n)).collect();
    let (result, recorder) = run(&core, profile);
    assert_eq!((result.final_text.as_str(), result.error.clone(), result.upload_error.clone()), ("Completed", None, None));
    assert_eq!(*recorder.esp.lock(), [true]);
    assert_eq!(result.uploaded, names.map(|n| format!("{n}.fseq")));
    let requests = device.requests();
    assert_eq!(requests.len(), 4, "{requests:?}");
    assert!(requests[1].starts_with("POST /fpp?path=uploadFile&filename=vixen-export-") && requests[1].ends_with("-part-001.zip"), "{requests:?}");
    assert!(requests[2].ends_with("-part-002.zip"), "{requests:?}");
    assert_eq!(requests[3], "POST /X6");
    for (request, count) in [(&requests[1], 5), (&requests[2], 1)] {
        let archive = device.body(request.strip_prefix("POST ").unwrap());
        assert_eq!(&archive[..2], b"PK");
        let entries = u16::from_le_bytes([archive[archive.len() - 12], archive[archive.len() - 11]]);
        assert_eq!(entries, count, "the end record's entry count");
    }
    let tasks = recorder.tasks.lock().clone();
    let at = |text: &str| tasks.iter().position(|t| t == text).unwrap_or_else(|| panic!("{text} in {tasks:?}"));
    assert!(at("Uploaded ZIP 1/2") < at("Loading Carol of the Bells"), "the first archive goes as soon as it's full");
    assert!(at("Uploaded ZIP 2/2") < at("ESPixelStick reboot acknowledged"));
    let overall = recorder.overall.lock().clone();
    // 12 steps for the sequences, 4 for the archives and 1 for the reboot.
    assert_eq!(overall.len(), 19, "{overall:?}");
    assert_eq!(overall[12], ("Overall Progress".to_string(), 70));
    assert_eq!(overall[17], ("Overall Progress".to_string(), 100));
    assert_eq!(overall[18], ("Completed".to_string(), 100));

    // Two sequences that would share a name inside the archive: refused before anything is loaded.
    let device = Device::start(esp(true));
    let mut profile = base.clone();
    profile.fpp_host_address = Some(device.host());
    profile.sequence_files = vec![sequence("Christmas Intro 1"), root.join("Sequence/Old/christmas intro 1.tim").to_string_lossy().into_owned()];
    let (result, recorder) = run(&core, profile);
    assert_eq!(result.final_text, "Failed");
    assert_eq!(result.upload_error.as_deref(), Some("Duplicate sequence filename 'Christmas Intro 1.fseq' cannot be uploaded in a ZIP archive."));
    assert_eq!(device.requests(), ["GET /api/system/info"]);
    assert!(recorder.tasks.lock().iter().all(|t| !t.starts_with("Loading")));

    // A device that refuses the first sequence: nothing more is exported or sent, and the run says why.
    let device = Device::start(|seen| if seen.path == "/api/system/info" { player(seen) } else { (500, "{}".to_string()) });
    let mut profile = base.clone();
    profile.fpp_host_address = Some(device.host());
    profile.sequence_files.push(sequence("Christmas Canon"));
    let (result, recorder) = run(&core, profile);
    assert_eq!(result.final_text, "Failed");
    assert_eq!(result.upload_error.as_deref(), Some("Upload of 'Christmas Intro 1.fseq' to 'sequences' failed with HTTP 500."));
    assert_eq!(result.error, None);
    assert!(result.uploaded.is_empty());
    assert_eq!(device.requests().len(), 2, "{:?}", device.requests());
    assert!(recorder.tasks.lock().iter().all(|t| t != "Loading Christmas Canon"));
    assert_eq!(recorder.overall.lock().last(), Some(&("Failed".to_string(), 0)));

    // An archive the ESPixelStick refuses: no reboot is asked for.
    let device = Device::start(|seen| if seen.path == "/api/system/info" { esp(true)(seen) } else { (507, String::new()) });
    let mut profile = base.clone();
    profile.fpp_host_address = Some(device.host());
    let (result, recorder) = run(&core, profile);
    assert_eq!(result.final_text, "Failed");
    assert!(result.upload_error.as_deref().is_some_and(|m| m.starts_with("ESPixelStick archive upload of 'vixen-export-") && m.ends_with("failed with HTTP 507.")), "{:?}", result.upload_error);
    assert_eq!(device.requests().len(), 2, "{:?}", device.requests());
    assert_eq!(recorder.overall.lock().last().map(|(t, _)| t.as_str()), Some("Failed"));
    assert!(recorder.overall.lock().last().unwrap().1 < 100);

    // Nothing listening: the run fails at detection.
    let closed = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let mut profile = base.clone();
    profile.fpp_host_address = Some(format!("127.0.0.1:{closed}"));
    let (result, recorder) = run(&core, profile);
    assert_eq!(result.final_text, "Failed");
    assert!(result.upload_error.is_some());
    assert!(recorder.tasks.lock().iter().all(|t| !t.starts_with("Loading")));

    // Direct Upload left ticked on another format is File Path (FIX): no device is asked.
    let device = Device::start(player);
    let mut profile = base.clone();
    profile.fpp_host_address = Some(device.host());
    profile.format = Some("Falcon Player Sequence".to_string());
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wizard-direct-file-path");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    profile.output_folder = Some(out.to_string_lossy().into_owned());
    profile.include_audio = false;
    if profile.export_format().is_some() && !profile.is_falcon_2x_format() {
        let (result, _) = run(&core, profile);
        assert_eq!(result.final_text, "Completed");
        assert_eq!(result.exported.len(), 1);
        assert!(device.requests().is_empty());
    }

    assert_eq!(leftovers(), before, "every temporary file is removed");
}
