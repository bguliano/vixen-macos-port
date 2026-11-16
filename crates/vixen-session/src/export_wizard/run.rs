//! F31 to F35: the wizard's export, on a thread of its own. Each sequence is loaded and rendered at the profile's interval, as upstream
//! renders after setting `DefaultUpdateInterval` to it, though here nothing global changes: the interval goes to this run's sessions only.

use super::profile::{controller_infos, ExportProfile};
use crate::app::AppCore;
use crate::export::{audio_file_name, controller_info_with_network, default_controller_info, ExportPause, ExportRequest, PreparedExport};
use crate::logging;
use crate::sequence::SequenceSession;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use vixen_export::fpp_config::{write_universe_file_2x, ControllerInfo};
use vixen_export::{file_name_of, format_audio_file_name};
use vixen_fpp::{ArchiveEntry, BatchEvent, DirectUpload, FppClient, FppError};
use vixen_io::LocalDateTime;

const LOGGER: &str = "VixenModules.App.ExportWizard.BulkExportSummaryStage";

/// Where a run reports, from its own thread.
pub trait BulkListener: Send + Sync {
    /// `lblTaskProgress` and its bar.
    fn task(&self, text: &str, percent: i32);
    /// `lblOverallProgress` and its bar.
    fn overall(&self, text: &str, percent: i32);
    /// F31 step 4: a sequence couldn't be loaded. Upstream asks `An error occurred opening the sequence {path}. \nDo you wish to continue
    /// with the export of the remaining ones?` (title `Error Loading Sequence`); true continues.
    fn load_failed(&self, path: &str, message: &str) -> bool;
    /// F31 step 1: the device a direct upload goes to turned out to be an ESPixelStick, which changes the Summary (F28).
    fn esp_pixel_stick(&self, _supports_zip: bool) {}
    fn finished(&self, result: BulkResult);
}

/// How a run ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BulkResult {
    /// `Completed`, `Cancelled` or `Failed`.
    pub final_text: String,
    /// A failure upstream lets escape to its crash handler, for an alert.
    pub error: Option<String>,
    /// A direct upload's failure, for the alert `FPP direct upload failed:\n{message}`, title `FPP Upload Error`. Upstream shows it
    /// the moment it fails, and nothing more is exported after it either way.
    pub upload_error: Option<String>,
    /// File Path: the files written. Direct Upload: nothing, since the files are temporary.
    pub exported: Vec<PathBuf>,
    /// Direct Upload: the names the device now has, sequences and audio, in the order sent. Sequences that went in ZIP archives
    /// are named once their archive has been taken.
    pub uploaded: Vec<String>,
    pub universe_file: Option<PathBuf>,
}

/// `steps`: two per sequence, plus the ZIP batches' when uploading to an ESPixelStick, at least 1.
pub fn overall_steps(sequences: usize, zip: bool) -> usize {
    let batches = if zip { sequences.div_ceil(5) * 2 + 1 } else { 0 };
    (sequences * 2 + batches).max(1)
}

/// `(int)(step / steps × 100)`.
pub fn overall_percent(step: usize, steps: usize) -> i32 {
    (step as f64 / steps.max(1) as f64 * 100.0) as i32
}

/// The name a backed-up universe file takes: `co-universes.json_{M}{d}{yyyy}-{H}{m}{s}`, unpadded local time.
pub fn backup_name(now: LocalDateTime) -> String {
    format!("co-universes.json_{}{}{}-{}{}{}", now.month, now.day, now.year, now.hour, now.minute, now.second)
}

/// A run in progress. Dropping it cancels and waits.
pub struct BulkRun {
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl BulkRun {
    /// `StageCancelled` during a run: stops at the next interval or between sequences.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    pub fn join(mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for BulkRun {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.cancel.store(true, Ordering::Relaxed);
            let _ = thread.join();
        }
    }
}

/// F31: exports every sequence in the profile, then the universe file.
pub fn start_bulk(core: Arc<AppCore>, profile: ExportProfile, listener: Arc<dyn BulkListener>) -> std::io::Result<BulkRun> {
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    let thread = std::thread::Builder::new().name("vixen-export-wizard".to_string()).spawn(move || {
        let (result, zip_percent) = catch_unwind(AssertUnwindSafe(|| run(&core, &profile, &flag, listener.as_ref()))).unwrap_or_else(|_| {
            let failed = BulkResult {
                final_text: "Failed".to_string(),
                error: Some("The export stopped unexpectedly.".to_string()),
                ..BulkResult::default()
            };
            (failed, None)
        });
        match zip_percent {
            // With ZIP batches the last task text stays, and the bar fills only on success.
            Some(percent) => listener.overall(&result.final_text, if result.final_text == "Completed" { 100 } else { percent }),
            None => {
                listener.task("", 0);
                listener.overall(&result.final_text, if result.final_text == "Completed" { 100 } else { 0 });
            }
        }
        listener.finished(result);
    })?;
    Ok(BulkRun { cancel, thread: Some(thread) })
}

/// A direct upload's device, and the exported files waiting for its next ZIP.
struct Direct<'a> {
    service: DirectUpload<'a>,
    cancel: vixen_fpp::Cancel,
    host: String,
    waiting: Vec<ArchiveEntry>,
    batches: usize,
    planned: usize,
}

impl Drop for Direct<'_> {
    fn drop(&mut self) {
        for entry in &self.waiting {
            let _ = std::fs::remove_file(&entry.path);
        }
    }
}

/// `_cancelled` and `_uploadFailed`, with the text for F34's alert: nothing more is exported. A cancel's own error isn't a failure.
fn upload_failed(result: &mut BulkResult, cancel: &AtomicBool, what: &str, error: &FppError) {
    if *error == FppError::Cancelled && cancel.load(Ordering::Relaxed) {
        return;
    }
    cancel.store(true, Ordering::Relaxed);
    logging::error(LOGGER, what, Some(&error.to_string()));
    result.upload_error.get_or_insert_with(|| error.to_string());
}

/// The first `.fseq` name two sequences would share, ignoring case (F31 step 3).
pub fn duplicate_fseq_name(sequence_files: &[String]) -> Option<String> {
    let names: Vec<String> = sequence_files.iter().map(|file| format!("{}.fseq", stem(file))).collect();
    names.iter().find(|name| names.iter().filter(|other| other.to_lowercase() == name.to_lowercase()).count() > 1).cloned()
}

fn stem(file: &str) -> String {
    Path::new(file).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn final_text(result: &BulkResult, cancel: &AtomicBool) -> String {
    if result.upload_error.is_some() || result.error.is_some() {
        "Failed"
    } else if cancel.load(Ordering::Relaxed) {
        "Cancelled"
    } else {
        "Completed"
    }
    .to_string()
}

/// The run, and for one that sends ZIP archives the overall bar's last value.
fn run(core: &AppCore, profile: &ExportProfile, cancel: &Arc<AtomicBool>, listener: &dyn BulkListener) -> (BulkResult, Option<i32>) {
    let mut result = BulkResult::default();
    let Some(format) = profile.export_format() else {
        result.final_text = "Failed".to_string();
        result.error = Some(format!("Unknown export format {}.", profile.format_text()));
        return (result, None);
    };

    // F31 step 1. FIX: only 2.6+ uploads, so a profile left on Direct Upload exports any other format to its folder.
    let host = profile.fpp_host_address.clone().unwrap_or_default();
    let fpp_cancel = vixen_fpp::Cancel::from_flag(Arc::clone(cancel));
    let client = if profile.is_falcon_2x_format() && profile.fpp_direct_upload {
        match FppClient::for_host(&host) {
            Ok(client) => Some(client),
            Err(error) => {
                upload_failed(&mut result, cancel, &format!("Export for '{host}' failed"), &error);
                result.final_text = final_text(&result, cancel);
                return (result, None);
            }
        }
    } else {
        None
    };
    let mut direct = match client.as_ref().map(|client| DirectUpload::detect(client, &fpp_cancel)).transpose() {
        Ok(service) => service.map(|service| Direct {
            service,
            cancel: fpp_cancel.clone(),
            host: host.clone(),
            waiting: Vec::new(),
            batches: 0,
            planned: profile.sequence_files.len().div_ceil(vixen_fpp::BATCH_SIZE),
        }),
        Err(error) => {
            upload_failed(&mut result, cancel, &format!("Export for '{host}' failed"), &error);
            result.final_text = final_text(&result, cancel);
            return (result, None);
        }
    };
    let zip = direct.as_ref().is_some_and(|d| d.service.supports_zip_archives());
    if direct.as_ref().is_some_and(|d| d.service.is_esp_pixel_stick()) {
        listener.esp_pixel_stick(zip);
    }
    if zip {
        if let Some(name) = duplicate_fseq_name(&profile.sequence_files) {
            let error = FppError::Argument(format!("Duplicate sequence filename '{name}' cannot be uploaded in a ZIP archive."));
            upload_failed(&mut result, cancel, &format!("Export for '{host}' failed"), &error);
            result.final_text = final_text(&result, cancel);
            return (result, None);
        }
    }

    let _pause = ExportPause::begin(core);
    let mut env = core.session_environment();
    env.frame_ms = profile.interval;
    let wants_universes = profile.is_falcon_2x_format() && profile.create_universe_file;
    let live = {
        let system = core.system.read();
        // Network configuration is only loaded when the universe file is wanted (F27), as upstream.
        if wants_universes {
            controller_info_with_network(&system)
        } else {
            default_controller_info(&system)
        }
    };
    let controllers = controller_infos(profile, &live);

    let mut steps = overall_steps(profile.sequence_files.len(), zip);
    let mut step = 0;
    listener.overall("Overall Progress", 0);
    for file in &profile.sequence_files {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let path = Path::new(file);
        let name = stem(file);
        listener.task(&format!("Loading {name}"), 0);
        let mut session = match SequenceSession::open_cancellable(path, &env, cancel) {
            Ok(session) => session,
            Err(error) => {
                if cancel.load(Ordering::Relaxed) { break; }
                logging::error(LOGGER, &format!("An error occurred loading sequence {file}"), Some(&error.to_string()));
                if !listener.load_failed(file, &error.to_string()) {
                    cancel.store(true, Ordering::Relaxed);
                    break;
                }
                step += 2;
                listener.overall("Overall Progress", overall_percent(step, steps));
                continue;
            }
        };
        listener.task(&format!("Loading any media for {name}"), 0);
        listener.task(&format!("Rendering {name}"), 0);
        render(&mut session, cancel, &mut |percent| listener.task(&format!("Rendering {name}"), percent));
        step += 1;
        listener.overall("Overall Progress", overall_percent(step, steps));
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        listener.task(&format!("Exporting {name}"), 0);
        match direct.as_mut() {
            Some(direct) => export_direct(core, profile, format, &controllers, &mut session, &name, cancel, listener, direct, &mut result),
            None => match export_one(core, profile, format, &controllers, &mut session, &name, cancel, listener) {
                Ok(Some(out)) => result.exported.push(out),
                Ok(None) => {}
                Err(error) => {
                    result.final_text = "Failed".to_string();
                    result.error = Some(error);
                    return (result, None);
                }
            },
        }
        step += 1;
        listener.overall("Overall Progress", overall_percent(step, steps));
        if let Some(direct) = direct.as_mut().filter(|d| zip && d.waiting.len() == vixen_fpp::BATCH_SIZE) {
            if !cancel.load(Ordering::Relaxed) {
                flush_zip(direct, listener, &mut step, steps, cancel, &mut result);
            }
        }
    }

    // F31 step 5.
    if let Some(direct) = direct.as_mut().filter(|_| zip) {
        if !cancel.load(Ordering::Relaxed) && !direct.waiting.is_empty() {
            flush_zip(direct, listener, &mut step, steps, cancel, &mut result);
        }
        if !cancel.load(Ordering::Relaxed) && direct.batches > 0 {
            // KEEP: the reboot is always the last step, however many sequences were skipped on the way.
            steps = step + 1;
            let service = &direct.service;
            let asked = vixen_fpp::request_reboot(
                &mut || service.reboot_esp_pixel_stick(&direct.cancel),
                &mut |event| match event {
                    BatchEvent::Task { text, percent } => listener.task(&text, percent as i32),
                    BatchEvent::PhaseCompleted => {
                        step += 1;
                        listener.overall("Overall Progress", overall_percent(step, steps));
                    }
                },
                &direct.cancel,
            );
            if let Err(error) = asked {
                upload_failed(&mut result, cancel, &format!("ESPixelStick reboot request to '{}' failed", direct.host), &error);
            }
        }
        result.final_text = final_text(&result, cancel);
        return (result, Some(overall_percent(step, steps)));
    }
    if !cancel.load(Ordering::Relaxed) {
        match direct.as_ref() {
            // An ESPixelStick has no universe file to send.
            Some(direct) if direct.service.supports_fpp_extras() => universe_direct(profile, &controllers, direct, cancel, &mut result),
            Some(_) => {}
            None if wants_universes => match write_universe_file(profile, &controllers) {
                Ok(file) => result.universe_file = file,
                Err(error) => {
                    result.final_text = "Failed".to_string();
                    result.error = Some(error);
                    return (result, None);
                }
            },
            None => {}
        }
    }
    result.final_text = final_text(&result, cancel);
    (result, None)
}

/// `UploadBatchAsync` for the files waiting, which are removed whatever happens.
fn flush_zip(direct: &mut Direct, listener: &dyn BulkListener, step: &mut usize, steps: usize, cancel: &AtomicBool, result: &mut BulkResult) {
    let batch = std::mem::take(&mut direct.waiting);
    direct.batches += 1;
    let part = direct.batches;
    let service = &direct.service;
    let sent = vixen_fpp::upload_batch(
        &batch,
        part,
        direct.planned.max(part),
        &std::env::temp_dir(),
        &mut |path, name| service.upload_archive_file(path, name, &direct.cancel),
        &mut |event| match event {
            BatchEvent::Task { text, percent } => listener.task(&text, percent as i32),
            BatchEvent::PhaseCompleted => {
                *step += 1;
                listener.overall("Overall Progress", overall_percent(*step, steps));
            }
        },
        &direct.cancel,
    );
    match sent {
        Ok(()) => result.uploaded.extend(batch.iter().map(|entry| entry.filename.clone())),
        Err(error) => upload_failed(result, cancel, &format!("ZIP batch {part} upload to '{}' failed", direct.host), &error),
    }
    for entry in &batch {
        let _ = std::fs::remove_file(&entry.path);
    }
}

/// A file name in the temporary folder that nothing else has, where upstream uses `Path.GetRandomFileName`.
fn temp_file(extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vixen-export-{}.{extension}", uuid::Uuid::new_v4().simple()))
}

/// F34, `ExportDirect`: the sequence goes to a temporary file, which is sent, or kept for the next ZIP; then its audio.
#[allow(clippy::too_many_arguments)]
fn export_direct(
    core: &AppCore,
    profile: &ExportProfile,
    format: vixen_export::ExportFormat,
    controllers: &[ControllerInfo],
    session: &mut SequenceSession,
    name: &str,
    cancel: &AtomicBool,
    listener: &dyn BulkListener,
    direct: &mut Direct,
    result: &mut BulkResult,
) {
    let audio = audio_file_name(session.document(), &core.paths);
    let temp = temp_file(format.file_type());
    let filename = format!("{name}.{}", format.file_type());
    let mut kept = false;
    let sent = match write_file(core, profile, format, controllers, session, name, &audio, &temp, cancel, listener) {
        Err(message) => Err(FppError::Io(message)),
        // Cancelled part way: the half-written file goes nowhere.
        Ok(false) => Ok(()),
        Ok(true) if direct.service.supports_zip_archives() => {
            direct.waiting.push(ArchiveEntry { path: temp.clone(), filename: filename.clone() });
            kept = true;
            Ok(())
        }
        Ok(true) => direct.service.upload_sequence_file(&temp, &filename, &mut |text, percent| listener.task(text, percent as i32), &direct.cancel).map(|outcome| {
            log_warning(&outcome);
            result.uploaded.push(filename.clone());
        }),
    };
    if !kept {
        let _ = std::fs::remove_file(&temp);
    }
    if let Err(error) = sent {
        upload_failed(result, cancel, &format!("Direct upload of sequence '{name}' failed"), &error);
    }
    if direct.service.supports_fpp_extras() && !cancel.load(Ordering::Relaxed) && profile.include_audio && !audio.is_empty() {
        let file = if profile.rename_audio { format_audio_file_name(name, &audio) } else { file_name_of(&audio).to_string() };
        match direct.service.upload_audio_file(Path::new(&audio), &file, &mut |text, percent| listener.task(text, percent as i32), &direct.cancel) {
            Ok(outcome) => {
                log_warning(&outcome);
                result.uploaded.push(file);
            }
            Err(error) => upload_failed(result, cancel, &format!("Direct upload of audio for '{name}' failed"), &error),
        }
    }
}

fn log_warning(outcome: &vixen_fpp::UploadOutcome) {
    if let Some(warning) = &outcome.warning {
        logging::warn(LOGGER, warning);
    }
}

/// F35, `CreateUniverseFileDirect`: the device's own file is renamed aside, then the new one is sent and fppd restarted.
fn universe_direct(profile: &ExportProfile, controllers: &[ControllerInfo], direct: &Direct, cancel: &AtomicBool, result: &mut BulkResult) {
    let sent = (|| {
        if profile.backup_universe_file {
            direct.service.backup_universe_file(&backup_name(LocalDateTime::now()), &direct.cancel)?;
        }
        if profile.create_universe_file {
            let temp = temp_file("json");
            let sent = write_universe_file_2x(&temp, controllers)
                .map_err(|e| FppError::Io(e.to_string()))
                .and_then(|()| direct.service.upload_universe_file(&temp, &direct.cancel))
                .and_then(|outcome| {
                    log_warning(&outcome);
                    direct.service.restart_fppd(false, &direct.cancel)
                });
            let _ = std::fs::remove_file(&temp);
            sent?;
            result.uploaded.push("config/co-universes.json".to_string());
        }
        Ok(())
    })();
    if let Err(error) = sent {
        upload_failed(result, cancel, &format!("Direct upload of universe file to '{}' failed", direct.host), &error);
    }
}

/// Renders every effect, reporting the share done.
fn render(session: &mut SequenceSession, cancel: &AtomicBool, progress: &mut dyn FnMut(i32)) {
    let total = session.effect_count().max(1);
    let mut reported = -1;
    loop {
        session.apply_finished_renders();
        let waiting = session.unrendered_count();
        let percent = ((total - waiting.min(total)) * 100 / total) as i32;
        if percent != reported {
            progress(percent);
            reported = percent;
        }
        if waiting == 0 || cancel.load(Ordering::Relaxed) {
            return;
        }
        let queued = session.counters.queued.load(Ordering::Acquire);
        let finished = session.counters.finished.load(Ordering::Acquire);
        if finished >= queued {
            session.render_all_dirty();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// `ExportToFilePath` for one sequence: the audio copy, then the file. `Ok(None)` when a folder couldn't be made and the sequence was
/// skipped, as upstream logs and skips it.
#[allow(clippy::too_many_arguments)]
fn export_one(
    core: &AppCore,
    profile: &ExportProfile,
    format: vixen_export::ExportFormat,
    controllers: &[ControllerInfo],
    session: &mut SequenceSession,
    name: &str,
    cancel: &AtomicBool,
    listener: &dyn BulkListener,
) -> Result<Option<PathBuf>, String> {
    let audio = audio_file_name(session.document(), &core.paths);
    let mut can_output = true;
    if profile.include_audio && !audio.is_empty() {
        let folder = profile.audio_output_folder.clone().unwrap_or_default();
        can_output = ensure_folder(&folder);
        if can_output {
            let file = if profile.rename_audio { format_audio_file_name(name, &audio) } else { file_name_of(&audio).to_string() };
            copy_audio(Path::new(&audio), &Path::new(&folder).join(file))?;
        }
    }
    let output = profile.output_folder.clone().unwrap_or_default();
    if !Path::new(&output).is_dir() {
        // KEEP: making the sequence folder decides on its own, so a failed audio folder skips the sequence only when this one existed.
        can_output = ensure_folder(&output);
    }
    if !can_output {
        return Ok(None);
    }
    let out = Path::new(&output).join(format!("{name}.{}", format.file_type()));
    let written = write_file(core, profile, format, controllers, session, name, &audio, &out, cancel, listener)?;
    Ok(written.then_some(out))
}

/// `DoExport` to `out`. False when it was cancelled part way.
#[allow(clippy::too_many_arguments)]
fn write_file(
    core: &AppCore,
    profile: &ExportProfile,
    format: vixen_export::ExportFormat,
    controllers: &[ControllerInfo],
    session: &mut SequenceSession,
    name: &str,
    audio: &str,
    out: &Path,
    cancel: &AtomicBool,
    listener: &dyn BulkListener,
) -> Result<bool, String> {
    let request = ExportRequest {
        format,
        compress: profile.enable_compression,
        out_file_name: out.to_path_buf(),
        update_interval_ms: profile.interval,
        audio_filename: audio.to_string(),
        // The wizard names the audio after the sequence when Rename is ticked, whether or not the audio is included.
        match_audio_name: profile.rename_audio,
        controllers: controllers.to_vec(),
    };
    let prepared = {
        let system = core.system.read();
        PreparedExport::prepare(session, &system, &request).map_err(|e| e.to_string())?
    };
    let message = format!("Exporting {name}");
    let outcome = prepared.run(cancel, &mut |percent, _| listener.task(&message, percent)).map_err(|e| e.to_string())?;
    Ok(!outcome.cancelled)
}

/// `CreateDirectory`, which logs a failure. GUARD: a folder that isn't an absolute path on this Mac, such as one from a profile made on
/// Windows, is never made.
fn ensure_folder(folder: &str) -> bool {
    if Path::new(folder).is_dir() {
        return true;
    }
    if folder.is_empty() || super::profile::is_foreign_path(folder) {
        logging::error(LOGGER, &format!("Unable to create the target directory. {folder}"), None);
        return false;
    }
    match std::fs::create_dir_all(folder) {
        Ok(()) => true,
        Err(error) => {
            logging::error(LOGGER, &format!("Unable to create the target directory. {folder}"), Some(&error.to_string()));
            false
        }
    }
}

/// `File.Copy(..., overwrite: true)`. GUARD: copying a file onto itself throws upstream, which happens when the audio folder is the folder
/// the audio is in and Rename is off; that's skipped here.
fn copy_audio(from: &Path, to: &Path) -> Result<(), String> {
    if let (Ok(a), Ok(b)) = (from.canonicalize(), to.canonicalize()) {
        if a == b {
            return Ok(());
        }
    }
    std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("The audio file {} could not be copied: {e}", from.display()))
}

/// F33's file path branch: `<FalconOutputFolder>/config/co-universes.json`, the old one renamed first when Backup is ticked. FIX: only
/// when Create Universe File is ticked, where upstream writes an empty list over the device's file otherwise.
fn write_universe_file(profile: &ExportProfile, controllers: &[ControllerInfo]) -> Result<Option<PathBuf>, String> {
    let falcon = profile.falcon_output_folder.clone().unwrap_or_default();
    let config = Path::new(&falcon).join("config");
    if !ensure_folder(&config.to_string_lossy()) {
        return Ok(None);
    }
    let file = config.join("co-universes.json");
    if profile.backup_universe_file && file.exists() {
        let backup = config.join(backup_name(LocalDateTime::now()));
        std::fs::rename(&file, &backup).map_err(|e| e.to_string())?;
    }
    write_universe_file_2x(&file, controllers).map_err(|e| e.to_string())?;
    Ok(Some(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_from_the_loading_callback_skips_load_prompts_media_and_export() {
        use parking_lot::Mutex;
        use uuid::Uuid;
        use vixen_io::{MediaEntry, SelectedTimingProvider};
        use vixen_xml::{XElement, XNode};

        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
        }
        struct CancelOnLoad {
            cancel: Arc<AtomicBool>,
            tasks: Mutex<Vec<String>>,
            prompts: Mutex<Vec<String>>,
        }
        impl BulkListener for CancelOnLoad {
            fn task(&self, text: &str, _: i32) {
                self.tasks.lock().push(text.to_owned());
                if text.starts_with("Loading ") {
                    self.cancel.store(true, Ordering::Release);
                }
            }
            fn overall(&self, _: &str, _: i32) {}
            fn load_failed(&self, path: &str, _: &str) -> bool {
                self.prompts.lock().push(path.to_owned());
                true
            }
            fn finished(&self, _: BulkResult) {}
        }

        let fixture = Fixture(std::env::temp_dir().join(format!("vixen-export-load-cancel-{}", Uuid::new_v4())));
        let paths = vixen_io::DataPaths::new(fixture.0.join("profile"));
        paths.ensure_directories().unwrap();
        let source = fixture.0.join("outside-profile.wma");
        let audio = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../vixen-audio/tests/assets/tone.wma")).unwrap();
        std::fs::write(&source, &audio).unwrap();
        let mut sequence = vixen_io::new_sequence();
        sequence.length = vixen_core::TimeSpan::from_ms_i(250);
        sequence.media.push(MediaEntry {
            type_id: uuid::uuid!("fe460392-3fab-4c63-99dd-d76c48354150"),
            instance_id: Uuid::new_v4(), file_name: String::new(), data: None,
        });
        sequence.selected_timing_provider = SelectedTimingProvider {
            provider_type: Some("Media".into()), source_name: Some(source.to_string_lossy().into_owned()),
        };
        let mut xml = XElement::parse(&vixen_io::sequence::sequence_to_document(&sequence).unwrap()).unwrap();
        xml.set_attr("version", "0");
        let media = xml.child_mut("_mediaSurrogates").unwrap().child_mut("MediaSurrogate").unwrap();
        let name = media.child_mut("FileName").unwrap();
        name.local = "FilePath".into();
        name.children = vec![XNode::Text(source.to_string_lossy().into_owned())];
        let sequence_path = paths.sequence_dir().join("Cancelled.tim");
        let original_xml = xml.to_document();
        std::fs::write(&sequence_path, &original_xml).unwrap();
        // A real load would restore this reachable media into the profile and decode it.
        vixen_io::load_sequence(&sequence_path).unwrap();
        let core = AppCore::assemble(paths, Default::default(), Default::default(), None,
            crate::boot::LaunchOptions { muted: true, ..Default::default() }, None);
        let mut profile = super::super::profile::default_profile("Cancelled", &fixture.0, &core.system.read());
        let output = fixture.0.join("result");
        profile.output_folder = Some(output.to_string_lossy().into_owned());
        profile.audio_output_folder = Some(output.join("audio").to_string_lossy().into_owned());
        profile.include_audio = true;
        profile.sequence_files = vec![sequence_path.to_string_lossy().into_owned()];
        let cancel = Arc::new(AtomicBool::new(false));
        let listener = CancelOnLoad { cancel: cancel.clone(), tasks: Mutex::new(Vec::new()), prompts: Mutex::new(Vec::new()) };
        let (result, _) = run(&core, &profile, &cancel, &listener);
        assert_eq!(result.final_text, "Cancelled");
        assert_eq!(result.error, None);
        assert_eq!(result.upload_error, None);
        assert!(result.exported.is_empty() && result.uploaded.is_empty() && result.universe_file.is_none());
        assert_eq!(*listener.tasks.lock(), ["Loading Cancelled"]);
        assert!(listener.prompts.lock().is_empty(), "cancellation must not ask to skip a failed sequence");
        assert_eq!(std::fs::read_dir(core.paths.media_dir()).unwrap().count(), 0, "no migration or audio load occurs");
        assert!(!output.exists(), "no sequence or audio output was published");
        assert_eq!(std::fs::read(&source).unwrap(), audio);
        assert_eq!(std::fs::read_to_string(sequence_path).unwrap(), original_xml);
    }

    #[test]
    fn overall_steps_follow_the_original() {
        assert_eq!(overall_steps(0, false), 1);
        assert_eq!(overall_steps(3, false), 6);
        assert_eq!(overall_steps(6, true), 12 + 4 + 1);
        assert_eq!(overall_percent(2, 6), 33);
        assert_eq!(overall_percent(4, 6), 66);
        assert_eq!(overall_percent(6, 6), 100);
    }

    #[test]
    fn a_backup_name_has_unpadded_local_time() {
        let at = LocalDateTime { year: 2024, month: 12, day: 9, hour: 22, minute: 40, second: 4, millisecond: 0 };
        assert_eq!(backup_name(at), "co-universes.json_1292024-22404");
    }

    #[test]
    fn two_sequences_of_one_name_are_found_whatever_their_case_or_folder() {
        let files = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        assert_eq!(duplicate_fseq_name(&files(&["/a/One.tim", "/a/Two.tim"])), None);
        assert_eq!(duplicate_fseq_name(&files(&["/a/One.tim", "/a/Two.tim", "/b/one.tim"])), Some("One.fseq".to_string()));
        assert_eq!(duplicate_fseq_name(&[]), None);
    }

    #[test]
    fn copying_audio_onto_itself_is_skipped() {
        let dir = std::env::temp_dir().join(format!("vixen-wizard-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let song = dir.join("song.mp3");
        std::fs::write(&song, b"audio").unwrap();
        copy_audio(&song, &song).unwrap();
        copy_audio(&song, &dir.join("copy.mp3")).unwrap();
        assert_eq!(std::fs::read(dir.join("copy.mp3")).unwrap(), b"audio");
        assert!(copy_audio(&dir.join("missing.mp3"), &dir.join("x.mp3")).is_err());
    }

    #[test]
    fn a_windows_folder_is_never_made() {
        assert!(!ensure_folder("C:\\Users\\olduser\\Export"));
        assert!(!ensure_folder("relative/folder"));
        assert!(!Path::new("C:\\Users\\olduser\\Export").exists());
    }
}
