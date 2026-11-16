//! The dialog's real background exporter must apply controller selection to frame data and sidecars together.
//! Upstream Export.cs ProcessExport filters IsActive, sorts Index, then concatenates Outputs; WriteControllerInfo
//! uses the same order and 1-based contiguous ranges. This fixture needs no original show or running hardware.

use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use uuid::Uuid;
use vixen_core::TimeSpan;
use vixen_effects::framework::ParamValue;
use vixen_export::ExportFormat;
use vixen_io::settings::ProfileSettings;
use vixen_io::system::{Channel, Controller, ControllerOutput, Patch};
use vixen_io::{DataPaths, SystemData};
use vixen_session::app::AppCore;
use vixen_session::export::default_controller_info;
use vixen_session::export_dialog::{start_export, ExportDialogState, ExportListener, ExportResult};
use vixen_session::sequence::SequenceSession;
use vixen_xml::XElement;

const SET_LEVEL: Uuid = uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0");

struct Listener {
    done: Mutex<Option<mpsc::Sender<ExportResult>>>,
    callback_threads: Mutex<Vec<std::thread::ThreadId>>,
}

impl ExportListener for Listener {
    fn progress(&self, _: i32) {
        self.callback_threads.lock().push(std::thread::current().id());
    }
    fn finished(&self, result: ExportResult) {
        self.callback_threads.lock().push(std::thread::current().id());
        self.done.lock().take().unwrap().send(result).unwrap();
    }
}

// This integration binary has exactly one test. Set the private settings location before creating any
// render/export workers, and restore the process environment after all workers/core have been dropped.
struct PrivateSettings(Option<std::ffi::OsString>);
impl PrivateSettings {
    fn new(path: &Path) -> Self {
        let prior = std::env::var_os("VIXEN_SETTINGS_DIR");
        std::env::set_var("VIXEN_SETTINGS_DIR", path);
        Self(prior)
    }
}
impl Drop for PrivateSettings {
    fn drop(&mut self) {
        if let Some(prior) = &self.0 { std::env::set_var("VIXEN_SETTINGS_DIR", prior); }
        else { std::env::remove_var("VIXEN_SETTINGS_DIR"); }
    }
}

fn add_level(session: &mut SequenceSession, node: Uuid, start: i64, length: i64, level: f64) {
    let id = session.add_new_effect(SET_LEVEL, node, "Private channel", TimeSpan::from_ms_i(start), TimeSpan::from_ms_i(length)).unwrap();
    let mut params = session.effect(id).unwrap().node.params.clone();
    params.set("IntensityLevel", ParamValue::Double(level));
    session.set_params(id, params);
}

fn export(core: &Arc<AppCore>, session: &Arc<Mutex<SequenceSession>>, state: &ExportDialogState, path: &Path) -> ExportResult {
    let caller = std::thread::current().id();
    let (tx, rx) = mpsc::channel();
    let listener = Arc::new(Listener { done: Mutex::new(Some(tx)), callback_threads: Mutex::new(Vec::new()) });
    let run = start_export(core.clone(), session.clone(), state.job(path).unwrap(), listener.clone()).unwrap();
    let result = rx.recv_timeout(Duration::from_secs(15)).expect("six-frame export completes within 15 seconds");
    run.join();
    assert_eq!(result.error, None);
    assert!(!result.cancelled && !result.falcon_warning);
    assert_eq!(result.frames, 6);
    let threads = listener.callback_threads.lock();
    assert!(!threads.is_empty() && threads.iter().all(|thread| *thread != caller), "the actual background worker reports completion");
    assert!(!vixen_export::run::part_path(path).exists());
    result
}

#[test]
fn selected_controller_order_reaches_nonzero_fseq_frames_and_network_ranges() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("export-controller-layout-{}", Uuid::new_v4()));
    let _settings_guard = PrivateSettings::new(&root.join("settings"));
    let paths = DataPaths::new(root.join("profile"));
    paths.ensure_directories().unwrap();
    let mut system = SystemData::default();
    let mut nodes = Vec::new();
    for (index, name, output_count) in [(0u128, "A", 2usize), (1, "B", 1), (2, "C", 1)] {
        let node = system.tree.add_element(&format!("Element {name}"));
        let element = system.tree.node(node).element_id.unwrap();
        nodes.push(system.tree.node(node).id);
        system.channels.push(Channel { id: element, name: format!("Element {name}") });
        let controller = Controller {
            id: Uuid::from_u128(100 + index), name: name.into(),
            hardware_type_id: vixen_output::ids::GENERIC_SERIAL, hardware_instance_id: Uuid::from_u128(200 + index),
            outputs: (0..output_count).map(|output| ControllerOutput {
                id: Uuid::from_u128(300 + index * 10 + output as u128), name: format!("{name}{output}"),
            }).collect(),
        };
        system.patches.push(Patch { component_id: controller.outputs[0].id, source_id: Some(element), output: 0 });
        // All hardware disabled independently of export selection; the second A output deliberately has no patch.
        system.disabled_devices.push(controller.id);
        system.controllers.push(controller);
    }
    vixen_session::web::store_settings(&mut system.module_store, vixen_session::web::Settings { port: 8888, enabled: false });
    vixen_io::save_system(&system, &paths).unwrap();
    let original_system = std::fs::read(paths.system_config_file()).unwrap();
    let original_store = std::fs::read(paths.module_store_file()).unwrap();
    let core = Arc::new(AppCore::open(paths.root().to_path_buf(), true).unwrap());
    let mut sequence = SequenceSession::new_untitled(&core.session_environment());
    sequence.set_length(TimeSpan::from_ms_i(300));
    // 8BitEvaluator truncates 255 * level: 1/4 -> 63, 1/2 -> 127, 3/4 -> 191.
    // ContextCurrentEffectsFull removes an effect only when currentTime > EndTime, and
    // ContextBase uses TimeNode.IntersectsInclusively for intents. Test both the exact endpoint
    // and the next 50-ms sample: C remains lit at 150 but is zero at 200; A remains lit at 200 but is zero at 250.
    add_level(&mut sequence, nodes[0], 0, 100, 0.25);
    add_level(&mut sequence, nodes[0], 100, 100, 0.5);
    add_level(&mut sequence, nodes[1], 0, 300, 1.0); // Never exported: B's 255 must not appear.
    add_level(&mut sequence, nodes[2], 50, 100, 0.75);
    let sequence_path = paths.sequence_dir().join("Controller order.tim");
    sequence.save(Some(&sequence_path)).unwrap();
    let original_sequence = std::fs::read(&sequence_path).unwrap();
    // Reopen the real serialized sequence, then let the exporter wait for its background renders.
    drop(sequence);
    let session = Arc::new(Mutex::new(SequenceSession::open(&sequence_path, &core.session_environment()).unwrap()));
    let settings = ProfileSettings::open(root.join("settings/Settings.xml"));
    let mut state = ExportDialogState::load(&settings, default_controller_info(&core.system.read()));
    state.set_row_active(1, false); // A, disabled B, C.
    state.move_rows(&[2], 0); // C, A, disabled B.
    let rows = state.rows();
    assert_eq!(rows.iter().map(|r| (r.name.as_str(), r.active, r.start, r.end)).collect::<Vec<_>>(),
        [("C", true, Some(1), Some(1)), ("A", true, Some(2), Some(3)), ("B", false, None, None)]);
    assert_eq!(state.format(), ExportFormat::FseqV2);
    state.set_compress(false, &settings).unwrap();
    assert_eq!(state.timing_text(), "50");
    let fseq_path = paths.export_dir().join("Selected.fseq");
    std::fs::create_dir_all(paths.export_dir()).unwrap();
    assert_eq!(export(&core, &session, &state, &fseq_path).controller_file, None);

    // Independent byte oracle: no production FSEQ reader, schedule, graph or layout helper builds these expectations.
    // Only the eight-byte time-derived file ID (24..32) varies.
    let expected_frames = [0, 63, 0, 191, 63, 0, 191, 127, 0, 191, 127, 0, 0, 127, 0, 0, 0, 0];
    let fseq = std::fs::read(&fseq_path).unwrap();
    let expected_prefix = [b'P', b'S', b'E', b'Q', 52, 0, 0, 2, 40, 0, 3, 0, 0, 0, 6, 0, 0, 0, 50, 0, 0, 1, 0, 0];
    assert_eq!(&fseq[..24], &expected_prefix);
    assert_eq!(&fseq[32..52], &[0, 0, 0, 0, 18, 0, 0, 0, 12, 0, b's', b'p', b'V', b'i', b'x', b'e', b'n', b' ', b'3', 0]);
    assert_eq!(&fseq[52..], &expected_frames);

    // Network.xml belongs to non-Falcon formats. Exercise the actual CSV branch rather than inventing an FSEQ/XML pairing.
    let csv_index = vixen_export::sorted_formats().iter().position(|f| *f == ExportFormat::Csv).unwrap();
    state.set_format_index(csv_index, &settings).unwrap();
    state.set_generate(true, &settings).unwrap();
    let csv_path = paths.export_dir().join("Selected.csv");
    let network_path = paths.export_dir().join("Selected_Network.xml");
    assert_eq!(export(&core, &session, &state, &csv_path).controller_file.as_ref(), Some(&network_path));
    assert_eq!(std::fs::read(&csv_path).unwrap(), b"000,063,000\r\n191,063,000\r\n191,127,000\r\n191,127,000\r\n000,127,000\r\n000,000,000\r\n");
    let network = XElement::from_file(&network_path).unwrap();
    assert_eq!(network.local, "Vixen3_Export");
    assert_eq!(network.child_text("Resolution").as_deref(), Some("50"));
    assert_eq!(network.child_text("OutFile").as_deref(), Some("Selected.csv"));
    assert_eq!(network.child_text("Duration").as_deref(), Some("00:00:00.3000000"));
    let controllers = network.child("Network").unwrap().children_named("Controller").map(|c|
        ["Index", "Name", "StartChan", "Channels"].map(|field| c.child_text(field).unwrap())
    ).collect::<Vec<_>>();
    assert_eq!(controllers, [["0", "C", "1", "1"].map(str::to_owned), ["1", "A", "2", "2"].map(str::to_owned)]);
    let mut files = std::fs::read_dir(paths.export_dir()).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect::<Vec<_>>();
    files.sort();
    assert_eq!(files, ["Selected.csv", "Selected.fseq", "Selected_Network.xml"]);
    assert_eq!(std::fs::read(paths.system_config_file()).unwrap(), original_system);
    assert_eq!(std::fs::read(paths.module_store_file()).unwrap(), original_store);
    assert_eq!(std::fs::read(sequence_path).unwrap(), original_sequence);
    eprintln!("verified 6 frames x 3 channels for C then A; B omitted; unpatched A1 zero; artifacts: {}", root.display());
}
