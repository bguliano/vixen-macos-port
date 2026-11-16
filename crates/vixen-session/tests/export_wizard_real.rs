//! The Export Wizard's run against the test copy of the owner's show (`target/show-scheduler`, whose `Media` links to the real one). Passes
//! quietly on a machine without it.

use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use vixen_export::fseq_read::FseqFile;
use vixen_session::app::AppCore;
use vixen_session::export_wizard::profile::default_profile;
use vixen_session::export_wizard::run::{start_bulk, BulkListener, BulkResult};

const REAL_EXPORT: &str = "/Users/developer/Documents/Vixen 3/Export";

fn show_copy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/show-scheduler")
}

#[derive(Default)]
struct Recorder {
    tasks: Mutex<Vec<String>>,
    overall: Mutex<Vec<(String, i32)>>,
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
    fn finished(&self, result: BulkResult) {
        if let Some(tx) = self.done.lock().take() {
            let _ = tx.send(result);
        }
    }
}

fn run(core: &Arc<AppCore>, profile: vixen_session::export_wizard::profile::ExportProfile) -> (BulkResult, Arc<Recorder>) {
    let (tx, rx) = mpsc::channel();
    let recorder = Arc::new(Recorder { done: Mutex::new(Some(tx)), ..Recorder::default() });
    let run = start_bulk(Arc::clone(core), profile, recorder.clone()).unwrap();
    let result = rx.recv_timeout(Duration::from_secs(600)).expect("the run finishes");
    run.join();
    (result, recorder)
}

#[test]
fn a_falcon_profile_exports_renames_the_audio_and_writes_the_universe_file() {
    let root = show_copy();
    if !root.is_dir() || !Path::new(REAL_EXPORT).is_dir() {
        return;
    }
    let core = Arc::new(AppCore::open(root.clone(), true).unwrap());
    let falcon = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wizard-falcon");
    let _ = std::fs::remove_dir_all(&falcon);
    std::fs::create_dir_all(&falcon).unwrap();

    let mut profile = default_profile("Pi", &root, &core.system.read());
    profile.interval = 25;
    profile.include_audio = true;
    profile.rename_audio = true;
    profile.create_universe_file = true;
    profile.backup_universe_file = true;
    profile.falcon_output_folder = Some(falcon.to_string_lossy().into_owned());
    profile.output_folder = Some(falcon.join("sequences").to_string_lossy().into_owned());
    profile.audio_output_folder = Some(falcon.join("music").to_string_lossy().into_owned());
    profile.sequence_files = ["Snoopy Christmas", "Missing Song", "Christmas Intro 1"]
        .iter()
        .map(|n| root.join("Sequence").join(format!("{n}.tim")).to_string_lossy().into_owned())
        .collect();

    let (result, recorder) = run(&core, profile.clone());
    assert_eq!(result.error, None);
    assert_eq!(result.final_text, "Completed");
    assert_eq!(result.exported.len(), 2, "the missing file is skipped after the prompt says continue");
    let tasks = recorder.tasks.lock().clone();
    for wanted in ["Loading Snoopy Christmas", "Rendering Snoopy Christmas", "Exporting Snoopy Christmas", "Loading Missing Song", "Exporting Christmas Intro 1"] {
        assert!(tasks.iter().any(|t| t == wanted), "{wanted} in {tasks:?}");
    }
    let overall = recorder.overall.lock().clone();
    assert_eq!(overall.first(), Some(&("Overall Progress".to_string(), 0)));
    assert_eq!(overall.last(), Some(&("Completed".to_string(), 100)));
    assert!(overall.contains(&("Overall Progress".to_string(), 33)), "{overall:?}");

    // Rename on: the audio takes the sequence's name, in the file and in the `mf` header.
    let snoopy = FseqFile::open(&falcon.join("sequences/Snoopy Christmas.fseq")).unwrap();
    assert_eq!(snoopy.header.media_file().as_deref(), Some("Snoopy Christmas.mp3"));
    assert_eq!(snoopy.header.step_ms, 25);
    assert_eq!(snoopy.header.channels, 3367);
    let real = FseqFile::open(&Path::new(REAL_EXPORT).join("Snoopy Christmas.fseq")).unwrap();
    assert_eq!(snoopy.header.frames, real.header.frames);
    let original_audio = std::fs::read(root.join("Media/18 - Snoopys Christmas (1999 Digital Remaster).mp3")).unwrap();
    assert_eq!(std::fs::read(falcon.join("music/Snoopy Christmas.mp3")).unwrap(), original_audio);
    assert!(falcon.join("music/Christmas Intro 1.mp3").is_file());

    let universes = falcon.join("config/co-universes.json");
    assert_eq!(result.universe_file.as_deref(), Some(universes.as_path()));
    assert!(std::fs::read_to_string(&universes).unwrap().contains("\"universes\": []"));

    // A second run backs the old file up first.
    profile.sequence_files.truncate(1);
    let (again, _) = run(&core, profile);
    assert_eq!(again.final_text, "Completed");
    let backups: Vec<String> = std::fs::read_dir(falcon.join("config"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("co-universes.json_"))
        .collect();
    assert_eq!(backups.len(), 1, "{backups:?}");
}
