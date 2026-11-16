//! The Export Sequence dialog, `ExportDialog`: F1 to F13 of `docs/spec/shows-scheduler-export.md`. Everything but the window: what it
//! remembers in `Settings.xml`, the controller list, the file it offers to write, and the run, which happens off the main thread.
//!
//! This is the owner's route to the FSEQ files their Pi plays, so the file it writes must be exactly what the original writes. The run goes
//! through [`PreparedExport`], the same path `tests/real_show.rs` compares frame by frame with the owner's real exports.

use crate::app::AppCore;
use crate::export::{audio_file_name, ExportPause, ExportRequest, PreparedExport};
use crate::sequence::SequenceSession;
use crate::sequencer::format_timespan_raw;
use parking_lot::Mutex;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use uuid::Uuid;
pub use vixen_export::fpp_config::ControllerInfo;
use vixen_export::fpp_config::{all_selected_support_universes, channel_ranges, controller_info_xml, reindex, write_universe_file_1x, write_universe_file_2x};
use vixen_export::{output_paths, sorted_formats, validate_interval, ControllerFilePlan, ExportFormat};
use vixen_io::settings::{ProfileSettings, SettingSection};
use vixen_io::{DataPaths, IoError};

const KEY_FORMAT: &str = "ExportDialog/ExportFormat";
const KEY_RESOLUTION: &str = "ExportDialog/ExportResolution";
const KEY_GENERATE: &str = "ExportDialog/GenerateUniverse";
const KEY_UNIVERSE_2X: &str = "ExportDialog/Universe2x";
const KEY_COMPRESS: &str = "ExportDialog/Compress";

/// The Timing list's items.
pub const TIMING_CHOICES: [&str; 3] = ["25", "50", "100"];
/// `exportResolutionDefault`: 50 ms. Every one of the owner's exports used 25, which they chose; their own setting is on the Windows machine.
pub const DEFAULT_RESOLUTION_INDEX: usize = 1;
/// FIX (F3): with nothing saved, `Falcon Player Sequence 2.6+`, as `Export.DefaultFormatType()` says. Upstream's `exportTypeDefault` is 0,
/// `CSV File`, which on a new Mac would make the first export a CSV.
pub const DEFAULT_FORMAT_INDEX: usize = 3;

/// `textBox1`, F6.
pub const NOTE: &str = "You must have all of your required display elements to controller outputs fully patched in the Display Setup for the export to produce correct results.";
/// F9 step 7's alert, title `Warning`.
pub const FALCON_WARNING: &str =
    "Some of the selected controllers do not support universes\nThese controllers will not be included in the universes file.\nSome manual FPP output configuration will be required.";

/// The Format list, sorted as the original sorts it, which is the order its saved index counts in.
pub fn format_names() -> Vec<&'static str> {
    sorted_formats().iter().map(|f| f.description()).collect()
}

/// One row of the Network list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerRow {
    pub id: Uuid,
    pub name: String,
    pub channels: i32,
    pub active: bool,
    /// 1-based first and last channel, blank for an unticked row.
    pub start: Option<i32>,
    pub end: Option<i32>,
}

/// What the save panel starts with, F9 steps 2 and 3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRequest {
    /// `<data root>/Export`, made if it's missing, as `Export.ExportDir` does.
    pub directory: PathBuf,
    /// `<sequence file name without extension>.<ext>`.
    pub file_name: String,
    /// The filter's name: the format's.
    pub format_name: String,
    pub extension: String,
}

/// The dialog's settings and list.
#[derive(Clone, Debug)]
pub struct ExportDialogState {
    format_index: usize,
    timing_text: String,
    generate: bool,
    fpp2x: bool,
    compress: bool,
    /// Every controller, with its tick and its place in `index`.
    controllers: Vec<ControllerInfo>,
}

fn section() -> SettingSection {
    SettingSection::AppSettings
}

impl ExportDialogState {
    /// `ExportForm_Load`. GUARD: an index outside the list throws upstream; it gives the default here. Unlike upstream, opening the
    /// dialog writes nothing: assigning the saved values there fires the change handlers, which save them straight back.
    pub fn load(settings: &ProfileSettings, controllers: Vec<ControllerInfo>) -> ExportDialogState {
        let index = |key: &str, default: usize, count: usize| {
            let saved = settings.get_i32(section(), key, default as i32);
            if saved >= 0 && (saved as usize) < count {
                saved as usize
            } else {
                default
            }
        };
        let resolution = index(KEY_RESOLUTION, DEFAULT_RESOLUTION_INDEX, TIMING_CHOICES.len());
        ExportDialogState {
            format_index: index(KEY_FORMAT, DEFAULT_FORMAT_INDEX, sorted_formats().len()),
            timing_text: TIMING_CHOICES[resolution].to_string(),
            generate: settings.get_bool(section(), KEY_GENERATE, false),
            fpp2x: settings.get_bool(section(), KEY_UNIVERSE_2X, true),
            compress: settings.get_bool(section(), KEY_COMPRESS, true),
            controllers,
        }
    }

    pub fn format(&self) -> ExportFormat {
        sorted_formats()[self.format_index]
    }

    pub fn format_index(&self) -> usize {
        self.format_index
    }

    pub fn timing_text(&self) -> &str {
        &self.timing_text
    }

    pub fn generate(&self) -> bool {
        self.generate
    }

    pub fn fpp2x(&self) -> bool {
        self.fpp2x
    }

    pub fn compress(&self) -> bool {
        self.compress
    }

    pub fn controllers(&self) -> &[ControllerInfo] {
        &self.controllers
    }

    /// The Format list's choice, saved at once as its index.
    pub fn set_format_index(&mut self, index: usize, settings: &ProfileSettings) -> Result<(), IoError> {
        if index >= sorted_formats().len() || index == self.format_index {
            return Ok(());
        }
        self.format_index = index;
        settings.put_i32(section(), KEY_FORMAT, index as i32)
    }

    /// A Timing list item, saved at once as its index.
    pub fn choose_timing(&mut self, index: usize, settings: &ProfileSettings) -> Result<(), IoError> {
        let Some(choice) = TIMING_CHOICES.get(index) else { return Ok(()) };
        self.timing_text = choice.to_string();
        settings.put_i32(section(), KEY_RESOLUTION, index as i32)
    }

    /// KEEP: text typed into the Timing box is used for the next export but never saved, because typing doesn't change the list's
    /// selection, which is what upstream saves.
    pub fn set_timing_text(&mut self, text: &str) {
        self.timing_text = text.to_string();
    }

    pub fn set_generate(&mut self, on: bool, settings: &ProfileSettings) -> Result<(), IoError> {
        self.generate = on;
        settings.put_bool(section(), KEY_GENERATE, on)
    }

    pub fn set_fpp2x(&mut self, on: bool, settings: &ProfileSettings) -> Result<(), IoError> {
        self.fpp2x = on;
        settings.put_bool(section(), KEY_UNIVERSE_2X, on)
    }

    pub fn set_compress(&mut self, on: bool, settings: &ProfileSettings) -> Result<(), IoError> {
        self.compress = on;
        settings.put_bool(section(), KEY_COMPRESS, on)
    }

    /// `SetUniverseVersionEnabled`: the FPP radios, only with the box ticked and a format whose writer says it's Falcon. KEEP: that's
    /// false for `Falcon Player Sequence`, so its radios stay off, though Start still writes a universe file for it by name.
    pub fn universe_radios_enabled(&self) -> bool {
        self.generate && self.format().is_falcon_format()
    }

    /// FIX (F8): Enable Compression follows the format alone. Upstream also enables it from the FPP 2.x radio and disables it from
    /// FPP 1.x, whatever the format, which has nothing to do with compression.
    pub fn compress_enabled(&self) -> bool {
        self.format().can_compress()
    }

    /// The list, in `Index` order.
    pub fn rows(&self) -> Vec<ControllerRow> {
        let ranges = channel_ranges(&self.controllers);
        self.in_row_order()
            .into_iter()
            .zip(ranges)
            .map(|(c, range)| ControllerRow {
                id: c.id,
                name: c.name.clone(),
                channels: c.channels,
                active: c.is_active,
                start: range.map(|r| r.0),
                end: range.map(|r| r.1),
            })
            .collect()
    }

    fn in_row_order(&self) -> Vec<&ControllerInfo> {
        let mut rows: Vec<&ControllerInfo> = self.controllers.iter().collect();
        rows.sort_by_key(|c| c.index);
        rows
    }

    /// Ticks or unticks a row, then `ReIndexControllerChannels`.
    pub fn set_row_active(&mut self, row: usize, on: bool) {
        let (order, mut active) = self.order_and_ticks();
        let Some(tick) = active.get_mut(row) else { return };
        *tick = on;
        reindex(&mut self.controllers, &order, &active);
    }

    /// A drag in the list, as SwiftUI's `move(fromOffsets:toOffset:)` reports it: the rows at `from` go, in their order, before the row
    /// that was at `to`. Then `ReIndexControllerChannels`.
    pub fn move_rows(&mut self, from: &[usize], to: usize) {
        move_controller_rows(&mut self.controllers, from, to);
    }

    fn order_and_ticks(&self) -> (Vec<Uuid>, Vec<bool>) {
        order_and_ticks(&self.controllers)
    }

    /// F9 steps 1 to 3: `None` for a sequence that has never been saved, for which Start does nothing.
    pub fn save_request(&self, sequence_file: Option<&Path>, paths: &DataPaths) -> Result<Option<SaveRequest>, IoError> {
        let Some(file) = sequence_file else { return Ok(None) };
        let directory = paths.export_dir();
        std::fs::create_dir_all(&directory).map_err(|e| IoError::Io { path: directory.clone(), source: e })?;
        let format = self.format();
        let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Some(SaveRequest {
            directory,
            file_name: format!("{stem}.{}", format.file_type()),
            format_name: format.description().to_string(),
            extension: format.file_type().to_string(),
        }))
    }

    /// F9 step 4, F4's GUARD: the interval, or the alert's text.
    pub fn interval(&self) -> Result<i32, String> {
        validate_interval(&self.timing_text).map_err(|e| e.to_string())
    }

    /// The file F9 writes beside the export.
    pub fn controller_file(&self, out: &Path) -> ControllerFilePlan {
        output_paths(self.format(), out, self.generate, self.fpp2x)
    }

    /// FIX (F9): upstream warns after every export to a format named `Falcon` that some ticked controller has no universes, whether or not
    /// it's writing a universe file. For the owner, whose controllers have none, that was every export. Here it only warns when it writes one.
    pub fn falcon_warning_needed(&self) -> bool {
        self.format().name_has_falcon() && self.generate && !all_selected_support_universes(&self.controllers)
    }
}

fn order_and_ticks(controllers: &[ControllerInfo]) -> (Vec<Uuid>, Vec<bool>) {
    let mut rows: Vec<&ControllerInfo> = controllers.iter().collect();
    rows.sort_by_key(|c| c.index);
    (rows.iter().map(|c| c.id).collect(), rows.iter().map(|c| c.is_active).collect())
}

/// A drag in a controller list (the dialog's and the wizard's), in `move(fromOffsets:toOffset:)`'s terms, then `ReIndexControllerChannels`.
pub fn move_controller_rows(controllers: &mut [ControllerInfo], from: &[usize], to: usize) {
    let (order, active) = order_and_ticks(controllers);
    let mut rows: Vec<(Uuid, bool)> = order.into_iter().zip(active).collect();
    let mut moving: Vec<usize> = from.iter().copied().filter(|i| *i < rows.len()).collect();
    moving.sort_unstable();
    moving.dedup();
    if moving.is_empty() {
        return;
    }
    let before = moving.iter().filter(|i| **i < to).count();
    let picked: Vec<(Uuid, bool)> = moving.iter().map(|i| rows[*i]).collect();
    for i in moving.iter().rev() {
        rows.remove(*i);
    }
    let at = to.saturating_sub(before).min(rows.len());
    rows.splice(at..at, picked);
    let order: Vec<Uuid> = rows.iter().map(|r| r.0).collect();
    let active: Vec<bool> = rows.iter().map(|r| r.1).collect();
    reindex(controllers, &order, &active);
}

/// F1: the Effect Preview stops while the dialog is up, and comes back when the last one closes.
pub struct PreviewHold {
    core: Arc<AppCore>,
}

static PREVIEW_HOLDS: AtomicUsize = AtomicUsize::new(0);

impl PreviewHold {
    pub fn new(core: Arc<AppCore>) -> PreviewHold {
        if PREVIEW_HOLDS.fetch_add(1, Ordering::AcqRel) == 0 {
            core.playback.hold_effect_preview(true);
        }
        PreviewHold { core }
    }
}

impl Drop for PreviewHold {
    fn drop(&mut self) {
        if PREVIEW_HOLDS.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.core.playback.hold_effect_preview(false);
        }
    }
}

/// How a run went, for the status strip and the alerts after it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportResult {
    pub cancelled: bool,
    pub frames: i32,
    /// The `Export Failed` alert's text. GUARD: upstream lets a failure escape to its crash handler.
    pub error: Option<String>,
    /// Show [`FALCON_WARNING`].
    pub falcon_warning: bool,
    /// The controller or universe file written beside the export.
    pub controller_file: Option<PathBuf>,
}

/// Where a run reports. Called from the export's own thread.
pub trait ExportListener: Send + Sync {
    /// The percent before an interval, at most 20 times a second.
    fn progress(&self, percent: i32);
    fn finished(&self, result: ExportResult);
}

/// A run in progress. Dropping it stops the run and waits for it, so nothing is left half written.
pub struct ExportRun {
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ExportRun {
    /// `Export.Cancel()`. The run stops at its next interval.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Waits for the run to end.
    pub fn join(mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for ExportRun {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.cancel.store(true, Ordering::Relaxed);
            let _ = thread.join();
        }
    }
}

/// What a run needs, taken from the dialog when Start is pressed. The list is fixed from here on.
#[derive(Clone, Debug)]
pub struct ExportJobSpec {
    pub format: ExportFormat,
    pub compress: bool,
    pub interval_ms: i32,
    pub controllers: Vec<ControllerInfo>,
    pub controller_file: ControllerFilePlan,
    pub falcon_warning: bool,
    pub out_file: PathBuf,
}

impl ExportDialogState {
    /// F9 step 4 onward, for the file the save panel gave: the job, or the alert's text for a bad interval.
    pub fn job(&self, out_file: &Path) -> Result<ExportJobSpec, String> {
        Ok(ExportJobSpec {
            format: self.format(),
            compress: self.compress,
            interval_ms: self.interval()?,
            controllers: self.controllers.clone(),
            controller_file: self.controller_file(out_file),
            falcon_warning: self.falcon_warning_needed(),
            out_file: out_file.to_path_buf(),
        })
    }
}

/// F9 steps 5 to 8 and F12, on a thread of its own: waits for any renders in progress, exports the sequence as it stands in memory, unsaved
/// edits included, then writes the controller file. As upstream, the controller file is written after a cancelled run too.
pub fn start_export(core: Arc<AppCore>, session: Arc<Mutex<SequenceSession>>, job: ExportJobSpec, listener: Arc<dyn ExportListener>) -> std::io::Result<ExportRun> {
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    let thread = std::thread::Builder::new().name("vixen-export".to_string()).spawn(move || {
        let outcome = catch_unwind(AssertUnwindSafe(|| run(&core, &session, &job, &flag, listener.as_ref())));
        let result = outcome.unwrap_or_else(|panic| {
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "The export stopped unexpectedly.".to_string());
            let _ = std::fs::remove_file(vixen_export::run::part_path(&job.out_file));
            ExportResult { error: Some(message), ..ExportResult::default() }
        });
        listener.finished(result);
    })?;
    Ok(ExportRun { cancel, thread: Some(thread) })
}

fn run(core: &AppCore, session: &Mutex<SequenceSession>, job: &ExportJobSpec, cancel: &AtomicBool, listener: &dyn ExportListener) -> ExportResult {
    let _pause = ExportPause::begin(core);
    wait_for_renders(session, cancel);
    let length_text = format_timespan_raw(session.lock().length());
    let mut result = ExportResult::default();
    if !cancel.load(Ordering::Relaxed) {
        let prepared = {
            let mut session = session.lock();
            let audio_filename = audio_file_name(session.document(), &core.paths);
            let request = ExportRequest {
                format: job.format,
                compress: job.compress,
                out_file_name: job.out_file.clone(),
                update_interval_ms: job.interval_ms,
                audio_filename,
                match_audio_name: false,
                controllers: job.controllers.clone(),
            };
            let system = core.system.read();
            PreparedExport::prepare(&mut session, &system, &request)
        };
        let outcome = prepared.and_then(|p| p.run(cancel, &mut |percent, _| listener.progress(percent)));
        match outcome {
            Ok(outcome) => {
                result.cancelled = outcome.cancelled;
                result.frames = outcome.frames_written;
            }
            Err(error) => {
                result.error = Some(error.to_string());
                return result;
            }
        }
    } else {
        result.cancelled = true;
    }
    match write_controller_file(job, &length_text) {
        Ok(written) => result.controller_file = written,
        Err(error) => result.error = Some(error.to_string()),
    }
    result.falcon_warning = job.falcon_warning;
    result
}

/// `Write2xUniverseFile`, `WriteUniverseFile` or `WriteControllerInfo`.
fn write_controller_file(job: &ExportJobSpec, sequence_length: &str) -> Result<Option<PathBuf>, vixen_export::ExportError> {
    match &job.controller_file {
        ControllerFilePlan::None => Ok(None),
        ControllerFilePlan::Universes2x(path) => write_universe_file_2x(path, &job.controllers).map(|_| Some(path.clone())),
        ControllerFilePlan::Universes1x(path) => write_universe_file_1x(path, &job.controllers).map(|_| Some(path.clone())),
        ControllerFilePlan::Network(path) => {
            let xml = controller_info_xml(job.interval_ms, &job.out_file.to_string_lossy(), sequence_length, &job.controllers);
            std::fs::write(path, xml)?;
            Ok(Some(path.clone()))
        }
    }
}

/// Like `SequenceSession::wait_for_renders`, but holding the session's lock only for moments, so the sequencer window stays responsive
/// while the last effects render. Gives up when the run is cancelled.
fn wait_for_renders(session: &Mutex<SequenceSession>, cancel: &AtomicBool) {
    loop {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        {
            let mut session = session.lock();
            session.apply_finished_renders();
            if session.unrendered_count() == 0 {
                return;
            }
            let queued = session.counters.queued.load(Ordering::Acquire);
            let finished = session.counters.finished.load(Ordering::Acquire);
            if finished >= queued {
                // Everything sent came back and something is still dirty: it changed since. Send it.
                session.render_all_dirty();
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-export-dialog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn settings(name: &str) -> ProfileSettings {
        ProfileSettings::open(temp_dir(name).join("Settings.xml"))
    }

    fn controller(id: u128, name: &str, index: i32, channels: i32) -> ControllerInfo {
        ControllerInfo { id: Uuid::from_u128(id), name: name.to_string(), index, is_active: true, channels, network: None }
    }

    /// The owner's two controllers.
    fn owner() -> Vec<ControllerInfo> {
        vec![controller(1, "Arduino", 0, 16), controller(2, "Light Strip Controller", 1, 3351)]
    }

    fn names(state: &ExportDialogState) -> Vec<String> {
        state.rows().into_iter().map(|r| r.name).collect()
    }

    #[test]
    fn with_nothing_saved_it_offers_fseq_2_at_50_ms_with_compression() {
        let s = settings("defaults");
        let state = ExportDialogState::load(&s, owner());
        assert_eq!(state.format(), ExportFormat::FseqV2);
        assert_eq!(state.format_index(), 3);
        assert_eq!(state.timing_text(), "50");
        assert!(!state.generate() && state.fpp2x() && state.compress());
        assert!(!s.path().exists(), "opening writes nothing");
    }

    #[test]
    fn saved_indexes_load_and_ones_outside_the_lists_fall_back() {
        let s = settings("saved");
        s.put_i32(SettingSection::AppSettings, KEY_FORMAT, 0).unwrap();
        s.put_i32(SettingSection::AppSettings, KEY_RESOLUTION, 0).unwrap();
        s.put_bool(SettingSection::AppSettings, KEY_GENERATE, true).unwrap();
        s.put_bool(SettingSection::AppSettings, KEY_UNIVERSE_2X, false).unwrap();
        s.put_bool(SettingSection::AppSettings, KEY_COMPRESS, false).unwrap();
        let state = ExportDialogState::load(&s, owner());
        assert_eq!(state.format(), ExportFormat::Csv);
        assert_eq!(state.timing_text(), "25");
        assert!(state.generate() && !state.fpp2x() && !state.compress());

        s.put_i32(SettingSection::AppSettings, KEY_FORMAT, 9).unwrap();
        s.put_i32(SettingSection::AppSettings, KEY_RESOLUTION, -1).unwrap();
        let state = ExportDialogState::load(&s, owner());
        assert_eq!(state.format(), ExportFormat::FseqV2);
        assert_eq!(state.timing_text(), "50");
    }

    #[test]
    fn a_choice_saves_at_once_under_the_original_keys() {
        let s = settings("choices");
        let mut state = ExportDialogState::load(&s, owner());
        state.set_format_index(2, &s).unwrap();
        state.choose_timing(0, &s).unwrap();
        state.set_generate(true, &s).unwrap();
        state.set_fpp2x(false, &s).unwrap();
        state.set_compress(false, &s).unwrap();
        let text = std::fs::read_to_string(s.path()).unwrap();
        for wanted in [
            "<ExportFormat>2</ExportFormat>",
            "<ExportResolution>0</ExportResolution>",
            "<GenerateUniverse>True</GenerateUniverse>",
            "<Universe2x>False</Universe2x>",
            "<Compress>False</Compress>",
        ] {
            assert!(text.contains(wanted), "{wanted} in {text}");
        }
        assert!(text.contains("<AppSettings>") && text.contains("<ExportDialog>"), "{text}");
        assert_eq!(state.format(), ExportFormat::FseqV1);
    }

    #[test]
    fn typed_timing_is_used_but_not_saved() {
        let s = settings("typed");
        let mut state = ExportDialogState::load(&s, owner());
        state.choose_timing(0, &s).unwrap();
        state.set_timing_text("40");
        assert_eq!(state.interval(), Ok(40));
        assert_eq!(s.get_i32(SettingSection::AppSettings, KEY_RESOLUTION, 9), 0);
        assert_eq!(ExportDialogState::load(&s, owner()).timing_text(), "25");
    }

    #[test]
    fn a_bad_interval_gives_the_original_alert_text() {
        let s = settings("interval");
        let mut state = ExportDialogState::load(&s, owner());
        for bad in ["abc", "0", "300", ""] {
            state.set_timing_text(bad);
            assert_eq!(state.interval(), Err(format!("Invalid timing interval {bad}, check the value and try again.")));
            assert!(state.job(Path::new("/x/a.fseq")).is_err());
        }
    }

    #[test]
    fn the_universe_radios_follow_the_box_and_the_writer_flag() {
        let s = settings("radios");
        let mut state = ExportDialogState::load(&s, owner());
        assert!(!state.universe_radios_enabled(), "off until the box is ticked");
        state.set_generate(true, &s).unwrap();
        assert!(state.universe_radios_enabled());
        // KEEP: the older FSEQ writer doesn't call itself Falcon.
        state.set_format_index(2, &s).unwrap();
        assert!(!state.universe_radios_enabled());
        state.set_format_index(1, &s).unwrap();
        assert!(state.universe_radios_enabled(), "Falcon Player Effect");
        state.set_format_index(0, &s).unwrap();
        assert!(!state.universe_radios_enabled());
    }

    #[test]
    fn compression_is_offered_for_fseq_2_alone_whatever_the_radios_say() {
        let s = settings("compress");
        let mut state = ExportDialogState::load(&s, owner());
        assert!(state.compress_enabled());
        state.set_fpp2x(false, &s).unwrap();
        assert!(state.compress_enabled(), "FIX: FPP 1.x doesn't grey it");
        for index in [0, 1, 2, 4, 5, 6] {
            state.set_format_index(index, &s).unwrap();
            assert!(!state.compress_enabled(), "{:?}", state.format());
        }
    }

    #[test]
    fn rows_show_ticked_ranges_in_index_order() {
        let s = settings("rows");
        let mut state = ExportDialogState::load(&s, owner());
        let rows = state.rows();
        assert_eq!((rows[0].start, rows[0].end), (Some(1), Some(16)));
        assert_eq!((rows[1].start, rows[1].end), (Some(17), Some(3367)));
        state.set_row_active(0, false);
        let rows = state.rows();
        assert!(!rows[0].active);
        assert_eq!((rows[0].start, rows[0].end), (None, None));
        assert_eq!((rows[1].start, rows[1].end), (Some(1), Some(3351)));
        state.set_row_active(0, true);
        state.move_rows(&[1], 0);
        assert_eq!(names(&state), ["Light Strip Controller", "Arduino"]);
        let rows = state.rows();
        assert_eq!((rows[0].start, rows[0].end), (Some(1), Some(3351)));
        assert_eq!((rows[1].start, rows[1].end), (Some(3352), Some(3367)));
        // The order reaches the export: indexes are row numbers.
        let indexes: Vec<(String, i32)> = state.controllers().iter().map(|c| (c.name.clone(), c.index)).collect();
        assert!(indexes.contains(&("Light Strip Controller".to_string(), 0)) && indexes.contains(&("Arduino".to_string(), 1)));
    }

    #[test]
    fn moves_follow_swiftuis_offsets() {
        let s = settings("moves");
        let three = vec![controller(1, "A", 0, 1), controller(2, "B", 1, 1), controller(3, "C", 2, 1)];
        let moved = |from: &[usize], to: usize| {
            let mut state = ExportDialogState::load(&s, three.clone());
            state.move_rows(from, to);
            names(&state).concat()
        };
        assert_eq!(moved(&[0], 3), "BCA");
        assert_eq!(moved(&[0], 2), "BAC");
        assert_eq!(moved(&[2], 0), "CAB");
        assert_eq!(moved(&[0, 2], 1), "ACB");
        assert_eq!(moved(&[1], 1), "ABC");
        assert_eq!(moved(&[7], 0), "ABC");
    }

    #[test]
    fn the_falcon_warning_appears_only_when_a_universe_file_is_written() {
        let s = settings("warning");
        let mut state = ExportDialogState::load(&s, owner());
        assert!(!state.falcon_warning_needed(), "FIX: no file, no warning");
        state.set_generate(true, &s).unwrap();
        assert!(state.falcon_warning_needed());
        state.set_format_index(2, &s).unwrap();
        assert!(state.falcon_warning_needed(), "by name, so the older FSEQ format too");
        state.set_format_index(0, &s).unwrap();
        assert!(!state.falcon_warning_needed());
        state.set_format_index(3, &s).unwrap();
        state.set_row_active(0, false);
        state.set_row_active(1, false);
        assert!(!state.falcon_warning_needed(), "nothing ticked lacks universes");
    }

    #[test]
    fn the_save_panel_starts_in_export_with_the_sequence_name() {
        let s = settings("save");
        let root = temp_dir("save-root");
        let paths = DataPaths::new(&root);
        let mut state = ExportDialogState::load(&s, owner());
        assert_eq!(state.save_request(None, &paths).unwrap(), None);
        let request = state.save_request(Some(Path::new("/elsewhere/Coldest Winter.tim")), &paths).unwrap().unwrap();
        assert_eq!(request.directory, root.join("Export"));
        assert!(request.directory.is_dir(), "made if missing");
        assert_eq!(request.file_name, "Coldest Winter.fseq");
        assert_eq!(request.format_name, "Falcon Player Sequence 2.6+");
        assert_eq!(request.extension, "fseq");
        state.set_format_index(6, &s).unwrap();
        assert_eq!(state.save_request(Some(Path::new("/x/A.b.tim")), &paths).unwrap().unwrap().file_name, "A.b.vix");
    }

    #[test]
    fn the_preview_hold_lasts_until_the_last_dialog_closes() {
        let root = temp_dir("hold");
        let core = Arc::new(AppCore::open(root, true).unwrap());
        let first = PreviewHold::new(Arc::clone(&core));
        let second = PreviewHold::new(Arc::clone(&core));
        assert!(!core.playback.effect_preview_may_start());
        drop(first);
        assert!(!core.playback.effect_preview_may_start());
        drop(second);
        assert!(core.playback.effect_preview_may_start());
    }

    struct Collect(Mutex<Vec<i32>>, Mutex<Option<mpsc::Sender<ExportResult>>>);
    impl ExportListener for Collect {
        fn progress(&self, percent: i32) {
            self.0.lock().push(percent);
        }
        fn finished(&self, result: ExportResult) {
            if let Some(tx) = self.1.lock().take() {
                let _ = tx.send(result);
            }
        }
    }

    fn listener() -> (Arc<Collect>, mpsc::Receiver<ExportResult>) {
        let (tx, rx) = mpsc::channel();
        (Arc::new(Collect(Mutex::new(Vec::new()), Mutex::new(Some(tx)))), rx)
    }

    fn core_and_session(name: &str, length_ms: f64) -> (Arc<AppCore>, Arc<Mutex<SequenceSession>>, PathBuf) {
        let root = temp_dir(name);
        let core = Arc::new(AppCore::open(root.clone(), true).unwrap());
        let mut session = SequenceSession::new_untitled(&core.session_environment());
        session.set_length(vixen_core::TimeSpan::from_ms(length_ms));
        (core, Arc::new(Mutex::new(session)), root)
    }

    #[test]
    fn a_run_writes_the_export_and_the_controller_file_and_reports() {
        let (core, session, root) = core_and_session("run", 2000.0);
        let s = settings("run-settings");
        let mut state = ExportDialogState::load(&s, Vec::new());
        state.set_format_index(0, &s).unwrap();
        state.set_generate(true, &s).unwrap();
        let out = root.join("Export").join("Song.csv");
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        let (collect, rx) = listener();
        let run = start_export(Arc::clone(&core), session, state.job(&out).unwrap(), collect.clone()).unwrap();
        let result = rx.recv_timeout(Duration::from_secs(30)).unwrap();
        run.join();
        assert_eq!(result.error, None);
        assert!(!result.cancelled && !result.falcon_warning);
        assert_eq!(result.frames, 40);
        assert_eq!(std::fs::read_to_string(&out).unwrap().lines().count(), 40);
        let network = root.join("Export").join("Song_Network.xml");
        assert_eq!(result.controller_file.as_deref(), Some(network.as_path()));
        let xml = std::fs::read_to_string(&network).unwrap();
        assert!(xml.contains("<Resolution>50</Resolution>") && xml.contains("<OutFile>Song.csv</OutFile>"), "{xml}");
        assert!(xml.contains("<Duration>00:00:02</Duration>"), "{xml}");
        assert_eq!(collect.0.lock().first(), Some(&0));
    }

    #[test]
    fn a_cancelled_run_leaves_the_old_export_and_still_writes_the_universe_file() {
        let (core, session, root) = core_and_session("cancel", 600_000.0);
        let s = settings("cancel-settings");
        let mut state = ExportDialogState::load(&s, Vec::new());
        state.set_generate(true, &s).unwrap();
        state.choose_timing(0, &s).unwrap();
        let out = root.join("Song.fseq");
        std::fs::write(&out, b"the earlier export").unwrap();
        let (collect, rx) = listener();
        let run = start_export(Arc::clone(&core), session, state.job(&out).unwrap(), collect).unwrap();
        run.cancel();
        let result = rx.recv_timeout(Duration::from_secs(30)).unwrap();
        run.join();
        assert!(result.cancelled);
        assert_eq!(result.error, None);
        assert_eq!(std::fs::read(&out).unwrap(), b"the earlier export");
        assert!(!vixen_export::run::part_path(&out).exists());
        assert!(root.join("Song-co-universes.json").is_file(), "as upstream, after a cancel too");
        assert!(!result.falcon_warning, "no controllers, so none lacks universes");
    }

    #[test]
    fn a_folder_that_isnt_there_is_an_error_for_the_alert() {
        let (core, session, root) = core_and_session("fails", 1000.0);
        let s = settings("fails-settings");
        let state = ExportDialogState::load(&s, Vec::new());
        let out = root.join("missing").join("Song.fseq");
        let (collect, rx) = listener();
        let run = start_export(core, session, state.job(&out).unwrap(), collect).unwrap();
        let result = rx.recv_timeout(Duration::from_secs(30)).unwrap();
        run.join();
        assert!(result.error.is_some_and(|e| !e.is_empty()));
        assert!(!out.exists());
    }

    #[test]
    fn dropping_a_run_stops_it() {
        let (core, session, root) = core_and_session("drop", 3_600_000.0);
        let s = settings("drop-settings");
        let mut state = ExportDialogState::load(&s, Vec::new());
        state.set_format_index(0, &s).unwrap();
        state.choose_timing(0, &s).unwrap();
        let out = root.join("Long.csv");
        let (collect, rx) = listener();
        let run = start_export(core, session, state.job(&out).unwrap(), collect).unwrap();
        drop(run);
        let result = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(result.cancelled || result.frames > 0);
        assert!(!out.exists() || result.frames == 144_000);
        assert!(!vixen_export::run::part_path(&out).exists());
    }
}
