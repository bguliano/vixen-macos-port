//! The Export Sequence dialog's route, as the window drives it, against the owner's real exports: the dialog's state from a fresh
//! `Settings.xml`, its controller list, the default file name, and a run on its own thread. The profile is the test copy in
//! `target/show-scheduler`, whose `Media` links back to the real show, so nothing of the owner's is written. These tests pass quietly
//! on a machine without either folder.

use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use vixen_export::fseq_read::FseqFile;
use vixen_export::ExportFormat;
use vixen_io::settings::ProfileSettings;
use vixen_session::app::AppCore;
use vixen_session::export::controller_info_with_network;
use vixen_session::export_dialog::{start_export, ExportDialogState, ExportListener, ExportResult};
use vixen_session::sequence::SequenceSession;

const REAL_EXPORT: &str = "/Users/developer/Documents/Vixen 3/Export";

fn show_copy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/show-scheduler")
}

struct Listener {
    percents: Mutex<Vec<i32>>,
    done: Mutex<Option<mpsc::Sender<ExportResult>>>,
}

impl ExportListener for Listener {
    fn progress(&self, percent: i32) {
        self.percents.lock().push(percent);
    }
    fn finished(&self, result: ExportResult) {
        if let Some(tx) = self.done.lock().take() {
            let _ = tx.send(result);
        }
    }
}

/// Exports one sequence the way the dialog does, with the owner's settings: the default format, Timing 25, compression on, every
/// controller ticked in system order.
fn export_like_the_dialog(core: &Arc<AppCore>, name: &str, out_dir: &Path) -> (PathBuf, ExportResult, Vec<i32>) {
    let settings = ProfileSettings::open(out_dir.join(format!("{name}-Settings.xml")));
    let controllers = controller_info_with_network(&core.system.read());
    let mut state = ExportDialogState::load(&settings, controllers);
    assert_eq!(state.format(), ExportFormat::FseqV2, "the Mac's first-run format");
    state.choose_timing(0, &settings).unwrap();
    assert_eq!(state.timing_text(), "25");

    let tim = core.paths.sequence_dir().join(format!("{name}.tim"));
    let request = state.save_request(Some(&tim), &core.paths).unwrap().unwrap();
    // Where the owner's Pi looks for it: `<data root>/Export/<sequence name>.fseq`.
    assert_eq!(request.directory, core.paths.export_dir());
    assert_eq!(request.file_name, format!("{name}.fseq"));

    let session = SequenceSession::open(&tim, &core.session_environment()).unwrap();
    let out = out_dir.join(&request.file_name);
    let (tx, rx) = mpsc::channel();
    let listener = Arc::new(Listener { percents: Mutex::new(Vec::new()), done: Mutex::new(Some(tx)) });
    let run = start_export(Arc::clone(core), Arc::new(Mutex::new(session)), state.job(&out).unwrap(), listener.clone()).unwrap();
    let result = rx.recv_timeout(Duration::from_secs(600)).expect("the export finishes");
    run.join();
    let percents = listener.percents.lock().clone();
    (out, result, percents)
}

#[test]
fn the_dialog_exports_match_the_real_files_frame_by_frame() {
    let root = show_copy();
    if !root.is_dir() || !Path::new(REAL_EXPORT).is_dir() {
        return;
    }
    let core = Arc::new(AppCore::open(root, true).unwrap());
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("export-dialog-real");
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();

    // These three have no random effects, so every byte of every frame can match.
    for name in ["Snoopy Christmas", "Carol of the Bells", "Christmas Can-Can"] {
        let (out, result, percents) = export_like_the_dialog(&core, name, &out_dir);
        assert_eq!(result.error, None, "{name}");
        assert!(!result.cancelled, "{name}");
        assert!(!result.falcon_warning, "{name}: Generate Controller File is off");
        assert_eq!(result.controller_file, None, "{name}");
        assert_eq!(percents.first(), Some(&0), "{name}");
        assert_eq!(percents.last(), Some(&99), "{name}");
        assert!(!vixen_export::run::part_path(&out).exists(), "{name}");

        let mut ours = FseqFile::open(&out).unwrap();
        let mut real = FseqFile::open(&Path::new(REAL_EXPORT).join(format!("{name}.fseq"))).unwrap();
        let (h, r) = (ours.header.clone(), real.header.clone());
        assert_eq!(result.frames as u32, r.frames, "{name}");
        assert_eq!((h.major, h.minor), (2, 0), "{name}");
        assert_eq!(h.flags, 0, "{name}");
        assert_eq!(h.channels, 3367, "{name}");
        assert_eq!(h.channels, r.channels, "{name}");
        assert_eq!(h.frames, r.frames, "{name}");
        assert_eq!(h.step_ms, r.step_ms, "{name}");
        assert_eq!(h.compression, r.compression, "{name}");
        assert_eq!(h.data_offset, r.data_offset, "{name}");
        assert_eq!(h.header_field, r.header_field, "{name}");
        assert_eq!(h.declared_blocks, r.declared_blocks, "{name}");
        assert_eq!(h.variable_headers, r.variable_headers, "{name}");
        let starts = |blocks: &[(u32, u32)]| blocks.iter().map(|b| b.0).collect::<Vec<u32>>();
        assert_eq!(starts(&h.blocks), starts(&r.blocks), "{name}: block boundaries");
        let (a, b) = (ours.read_all_frames().unwrap(), real.read_all_frames().unwrap());
        assert_eq!(a.len(), b.len(), "{name}");
        let frame = h.channels as usize;
        if let Some(first) = (0..a.len() / frame).find(|f| a[f * frame..(f + 1) * frame] != b[f * frame..(f + 1) * frame]) {
            panic!("{name}: frame {first} differs");
        }
    }
}

#[test]
fn coldest_winter_names_its_audio_and_has_every_frame() {
    let root = show_copy();
    if !root.is_dir() || !Path::new(REAL_EXPORT).is_dir() {
        return;
    }
    let core = Arc::new(AppCore::open(root, true).unwrap());
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("export-dialog-coldest");
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();
    let (out, result, _) = export_like_the_dialog(&core, "Coldest Winter", &out_dir);
    assert_eq!(result.error, None);
    let ours = FseqFile::open(&out).unwrap();
    assert_eq!(ours.header.media_file().as_deref(), Some("Coldest Winter Pentatonix.mp3"));
    assert_eq!(ours.header.frames, 5946);
    let real = FseqFile::open(&Path::new(REAL_EXPORT).join("Coldest Winter.fseq")).unwrap();
    assert_eq!(ours.header.variable_headers, real.header.variable_headers);
    assert_eq!(ours.header.data_offset, real.header.data_offset);
}
