//! The Export Wizard, `BulkExportWizard`: F14 to F36 of `docs/spec/shows-scheduler-export.md`. Its stages, the profile being edited,
//! and every rule for moving on, without a window. The run is in [`run`].
//!
//! As upstream, every edit happens on a copy of the chosen profile, `active`. The list itself, `profiles`, changes only by adding,
//! deleting and renaming (F17, F18, F20) and by `Save export settings` (F29). Saving the module store is the caller's.

pub mod device;
pub mod profile;
pub mod run;

use crate::export_dialog::{move_controller_rows, ControllerRow};
use profile::{default_profile, synchronize_controllers, validate_profile_name, ExportProfile, NameError};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_export::fpp_config::{all_selected_support_universes, channel_ranges, reindex, ControllerInfo};
use vixen_export::{sorted_formats, validate_interval, ExportFormat};
use vixen_io::{DataPaths, SystemData};

/// The six stages, in order.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    Config,
    Sources,
    Controllers,
    OutputFormat,
    Summary,
    Finished,
}

impl Stage {
    const ORDER: [Stage; 6] = [Stage::Config, Stage::Sources, Stage::Controllers, Stage::OutputFormat, Stage::Summary, Stage::Finished];

    pub fn index(self) -> usize {
        Stage::ORDER.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// `GetAllSequenceFileNames(ExcludeBackup)`: every `.tim` and `.vix` under the data root, with that exact extension, leaving out the
/// sequence backup folder. KEEP: it searches the whole data root, so a `.vix` export in `Export` is included. GUARD: upstream builds the
/// backup folder's name with a backslash; here it's a real path component. Sorted by path for a stable list.
pub fn all_sequence_files(paths: &DataPaths, exclude_backup: bool) -> Vec<PathBuf> {
    let backup = paths.sequence_backup_dir();
    let mut found = Vec::new();
    let mut pending = vec![paths.root().to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() || kind.is_symlink() {
                let ext = vixen_export::extension_of(path.to_str().unwrap_or(""));
                if (ext == ".tim" || ext == ".vix") && !(exclude_backup && path.starts_with(&backup)) {
                    found.push(path);
                }
            }
        }
    }
    found.sort();
    found
}

/// What the Summary stage shows, F27.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub sequences: usize,
    pub format: String,
    pub timing: String,
    pub destination: String,
    pub audio: String,
    /// Shown with Include Audio only.
    pub audio_destination: Option<String>,
    /// Shown for a Falcon-named format with Create Universe File.
    pub universe_destination: Option<String>,
    /// The warning line, when there is one.
    pub warning: Option<String>,
    /// F28, with Direct Upload: `FPP Device Info`, or the line that says it couldn't be read. None until the device has answered.
    pub device_heading: Option<String>,
    /// `Host Name`, `Description`, `Platform` and `Variant` with their values.
    pub device_rows: Vec<(String, String)>,
}

/// What the Summary knows of the device a direct upload goes to, F28.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DeviceInfo {
    /// File Path, or the device hasn't answered yet.
    #[default]
    Unknown,
    Unavailable,
    Known { host_name: String, description: String, platform: String, variant: String, zip: bool },
}

impl DeviceInfo {
    /// Upstream compares the platform exactly.
    pub fn esp_pixel_stick_zip(&self) -> Option<bool> {
        match self {
            DeviceInfo::Known { platform, zip, .. } if platform == "ESPixelStick" => Some(*zip),
            _ => None,
        }
    }
}

pub const DEVICE_HEADING: &str = "FPP Device Info";
pub const DEVICE_UNAVAILABLE: &str = "FPP Device Info (unavailable — check host address)";
pub const ESP_NOT_UPLOADED: &str = "Not uploaded to ESPixelStick.";

/// The warning line an ESPixelStick puts on the Summary.
pub fn esp_warning(zip: bool) -> String {
    let route = if zip {
        "FSEQ sequences will be sent in ZIP archives of up to five files, followed by one reboot request."
    } else {
        "FSEQ sequences will be uploaded individually; no reboot is requested."
    };
    format!("ESPixelStick direct upload sends FSEQ sequences only; audio and universe settings are not uploaded. {route}")
}

/// The alert for a host that didn't answer, title [`HOST_UNREACHABLE_TITLE`].
pub fn host_unreachable_message(host: &str) -> String {
    format!("FPP host '{host}' could not be reached. Verify the host is online and try again.")
}

pub const HOST_UNREACHABLE_TITLE: &str = "FPP Host Unreachable";

/// The `Not all controllers selected...` line.
pub const SUMMARY_WARNING: &str =
    "Not all controllers selected for export support universes.\nThese controllers will not be included in the universes file.\nSome manual FPP output configuration will be required.";

/// F26's note, reworded for the Mac (FIX): there are no UNC paths here.
pub const FALCON_NOTE: &str = "Output folder or FPP network location. To save directly to FPP, mount its media share and select that folder.";

/// F27: cached preparation belongs to one visit to Summary, not to a later retry.
enum NetworkPreparation {
    NotNeeded,
    Pending,
    Ready(Vec<ControllerInfo>),
    Failed(String),
}

/// The wizard.
pub struct ExportWizard {
    paths: DataPaths,
    /// `_data.Profiles`.
    profiles: Vec<ExportProfile>,
    /// The Configurations list's choice.
    selected: usize,
    /// `ActiveProfile`: the copy being edited.
    active: Option<ExportProfile>,
    stage: Stage,
    /// The Timing box's text on the format stage.
    interval_text: String,
    /// The Falcon group's Include Audio box and the Audio group's, which upstream keeps apart.
    falcon_audio_box: bool,
    audio_box: bool,
    /// `_fppHostReachable`: the host box's address answered the last check. Any change to the address or the mode clears it.
    host_reachable: bool,
    network_generation: u64,
    network_preparation: NetworkPreparation,
    /// What the device said of itself, for the Summary.
    device: DeviceInfo,
    /// What the run found at its first step: an ESPixelStick, and whether it takes ZIP archives.
    esp_detected: Option<bool>,
    /// Summary: `Save export settings` and its name.
    pub save_settings: bool,
    pub save_name: String,
    /// The final text, F36.
    final_text: String,
}

impl ExportWizard {
    /// F14: a wizard over the saved profiles, on its first stage.
    pub fn new(paths: DataPaths, profiles: Vec<ExportProfile>, system: &SystemData) -> ExportWizard {
        let mut wizard = ExportWizard {
            paths,
            profiles,
            selected: 0,
            active: None,
            stage: Stage::Config,
            interval_text: String::new(),
            falcon_audio_box: false,
            audio_box: false,
            host_reachable: false,
            network_generation: 0,
            network_preparation: NetworkPreparation::NotNeeded,
            device: DeviceInfo::Unknown,
            esp_detected: None,
            save_settings: false,
            save_name: String::new(),
            final_text: String::new(),
        };
        wizard.stage_start(system);
        wizard
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn profiles(&self) -> &[ExportProfile] {
        &self.profiles
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn active(&self) -> Option<&ExportProfile> {
        self.active.as_ref()
    }

    fn active_mut(&mut self) -> Option<&mut ExportProfile> {
        self.active.as_mut()
    }

    pub fn paths(&self) -> &DataPaths {
        &self.paths
    }

    /// A test's shortcut past the stages before the one it's about.
    #[cfg(test)]
    pub(crate) fn set_stage_for_test(&mut self, stage: Stage) {
        self.stage = stage;
    }

    /// `StageStart` for the stage now showing.
    fn stage_start(&mut self, system: &SystemData) {
        self.network_generation = self.network_generation.wrapping_add(1);
        self.network_preparation = NetworkPreparation::NotNeeded;
        match self.stage {
            Stage::Config => {
                // KEEP: coming back here reselects the first profile and drops the edits made to the copy.
                if self.profiles.is_empty() {
                    self.profiles.push(default_profile("Default", self.paths.root(), system));
                }
                self.selected = 0;
                self.active = self.profiles.first().cloned();
            }
            Stage::Sources => {
                // Files that have gone are dropped without a word, as upstream.
                if let Some(active) = self.active_mut() {
                    active.sequence_files.retain(|f| Path::new(f).is_file());
                }
            }
            Stage::Controllers => {
                if let Some(active) = self.active_mut() {
                    synchronize_controllers(active, system);
                }
            }
            Stage::OutputFormat => {
                if let Some(active) = &self.active {
                    self.interval_text = active.interval.to_string();
                    self.falcon_audio_box = active.include_audio;
                    self.audio_box = active.include_audio;
                }
                // A saved host is checked again each time the stage opens: see `host_to_check`.
                self.host_reachable = false;
            }
            Stage::Summary => {
                if self.active.as_ref().is_some_and(|a| a.is_falcon_2x_format() && a.create_universe_file) {
                    self.network_preparation = NetworkPreparation::Pending;
                }
                self.device = DeviceInfo::Unknown;
                self.esp_detected = None;
                self.save_settings = false;
                self.save_name = self.active.as_ref().map(|a| a.name_text().to_string()).unwrap_or_default();
            }
            Stage::Finished => {}
        }
    }

    /// `Next >>` on any stage but Summary, whose Next runs the export ([`ExportWizard::begin_run`]).
    pub fn next(&mut self, system: &SystemData, exists: &dyn Fn(&str) -> bool) -> bool {
        if !self.can_move_next(exists) || matches!(self.stage, Stage::Summary | Stage::Finished) {
            return false;
        }
        self.stage = Stage::ORDER[self.stage.index() + 1];
        self.stage_start(system);
        true
    }

    /// `<< Previous`: back a stage, with no `StageEnd`, and that stage's `StageStart` again.
    pub fn previous(&mut self, system: &SystemData) -> bool {
        if !self.can_move_previous() {
            return false;
        }
        self.stage = Stage::ORDER[self.stage.index() - 1];
        self.stage_start(system);
        true
    }

    pub fn can_move_previous(&self) -> bool {
        !matches!(self.stage, Stage::Config | Stage::Finished)
    }

    /// `CanMoveNext` of the stage showing. `exists` says whether a folder exists; a path from another computer never does.
    pub fn can_move_next(&self, exists: &dyn Fn(&str) -> bool) -> bool {
        let Some(active) = &self.active else { return false };
        match self.stage {
            Stage::Config => !self.profiles.is_empty(),
            // FIX: from state, so removing the last file disables Next.
            Stage::Sources => !active.sequence_files.is_empty(),
            // GUARD: at least one ticked, where upstream allows a file of no channels.
            Stage::Controllers => active.controllers.as_ref().is_some_and(|c| c.iter().any(|c| c.is_active)),
            Stage::OutputFormat => {
                if validate_interval(&self.interval_text).is_err() {
                    return false;
                }
                let usable = |folder: &Option<String>| folder.as_deref().is_some_and(|f| !profile::is_foreign_path(f) && exists(f));
                if self.direct_upload() {
                    self.host_reachable && vixen_fpp::is_host_address_valid(self.host_text())
                } else if active.is_falcon_format() {
                    usable(if active.is_falcon_2x_format() { &active.falcon_output_folder } else { &active.output_folder })
                } else {
                    usable(&active.output_folder) && (!active.include_audio || usable(&active.audio_output_folder))
                }
            }
            // GUARD: a blank name with Save ticked would save a nameless profile.
            Stage::Summary => !matches!(self.network_preparation, NetworkPreparation::Pending | NetworkPreparation::Failed(_))
                && (!self.save_settings || !self.save_name.trim().is_empty()),
            Stage::Finished => true,
        }
    }

    // Stage 1: configurations.

    /// Choosing another profile in the list: a fresh copy of it.
    pub fn select_profile(&mut self, index: usize) {
        if let Some(p) = self.profiles.get(index) {
            self.selected = index;
            self.active = Some(p.clone());
        }
    }

    pub fn profile_names(&self) -> Vec<&str> {
        self.profiles.iter().map(|p| p.name_text()).collect()
    }

    /// F17: a new default profile with that name, added and chosen. Not saved until something else saves.
    pub fn add_profile(&mut self, name: &str, system: &SystemData) -> Result<(), NameError> {
        let name = validate_profile_name(&self.profile_names(), name, None)?;
        self.profiles.push(default_profile(&name, self.paths.root(), system));
        self.select_profile(self.profiles.len() - 1);
        Ok(())
    }

    /// F18, after its confirmation: removes the chosen profile, adds `Default` if none remain, chooses the first. The caller saves.
    pub fn delete_profile(&mut self, system: &SystemData) {
        if self.selected < self.profiles.len() {
            self.profiles.remove(self.selected);
        }
        if self.profiles.is_empty() {
            self.profiles.push(default_profile("Default", self.paths.root(), system));
        }
        self.select_profile(0);
    }

    /// F20: renames the list's entry, not the copy being edited (KEEP). `Ok(false)` for an unchanged name; on `Ok(true)` the caller saves.
    pub fn rename_profile(&mut self, name: &str) -> Result<bool, NameError> {
        let current = self.profiles.get(self.selected).map(|p| p.name_text().to_string()).unwrap_or_default();
        let name = validate_profile_name(&self.profile_names(), name, Some(&current))?;
        if name == current {
            return Ok(false);
        }
        if let Some(p) = self.profiles.get_mut(self.selected) {
            p.name = Some(name);
        }
        Ok(true)
    }

    // Stage 2: sequences.

    pub fn sequence_files(&self) -> &[String] {
        self.active.as_ref().map(|a| a.sequence_files.as_slice()).unwrap_or(&[])
    }

    /// F22: files chosen in the panel, which only offers the profile's `Sequence` folder. Duplicates are skipped.
    pub fn add_sequences(&mut self, files: &[PathBuf]) {
        let folder = self.paths.sequence_dir();
        let Some(active) = self.active_mut() else { return };
        for file in files {
            if !file.starts_with(&folder) {
                continue;
            }
            let text = file.to_string_lossy().into_owned();
            if !active.sequence_files.contains(&text) {
                active.sequence_files.push(text);
            }
        }
    }

    /// F23.
    pub fn add_all_sequences(&mut self) {
        let all = all_sequence_files(&self.paths, true);
        let Some(active) = self.active_mut() else { return };
        for file in all {
            let text = file.to_string_lossy().into_owned();
            if !active.sequence_files.contains(&text) {
                active.sequence_files.push(text);
            }
        }
    }

    /// F24.
    pub fn remove_sequences(&mut self, rows: &[usize]) {
        let Some(active) = self.active_mut() else { return };
        let mut rows: Vec<usize> = rows.iter().copied().filter(|r| *r < active.sequence_files.len()).collect();
        rows.sort_unstable();
        rows.dedup();
        for r in rows.into_iter().rev() {
            active.sequence_files.remove(r);
        }
    }

    // Stage 3: controllers.

    /// The list, with the live names and channel counts.
    pub fn controller_rows(&self, live: &[ControllerInfo]) -> Vec<ControllerRow> {
        let Some(active) = &self.active else { return Vec::new() };
        let infos = profile::controller_infos(active, live);
        let mut ordered: Vec<&ControllerInfo> = infos.iter().collect();
        ordered.sort_by_key(|c| c.index);
        let ranges = channel_ranges(&infos);
        ordered
            .into_iter()
            .zip(ranges)
            .map(|(c, r)| ControllerRow { id: c.id, name: c.name.clone(), channels: c.channels, active: c.is_active, start: r.map(|r| r.0), end: r.map(|r| r.1) })
            .collect()
    }

    fn apply_to_controllers(&mut self, live: &[ControllerInfo], change: impl FnOnce(&mut Vec<ControllerInfo>)) {
        let Some(active) = self.active.as_mut() else { return };
        let mut infos = profile::controller_infos(active, live);
        change(&mut infos);
        if let Some(stored) = active.controllers.as_mut() {
            for s in stored.iter_mut() {
                if let Some(i) = infos.iter().find(|i| i.id == s.id) {
                    s.index = i.index;
                    s.is_active = i.is_active;
                }
            }
        }
    }

    /// Ticks or unticks rows, each by its row number, then reindexes.
    pub fn set_controllers_active(&mut self, rows: &[usize], on: bool, live: &[ControllerInfo]) {
        self.apply_to_controllers(live, |infos| {
            let mut ordered: Vec<(Uuid, bool)> = {
                let mut v: Vec<&ControllerInfo> = infos.iter().collect();
                v.sort_by_key(|c| c.index);
                v.into_iter().map(|c| (c.id, c.is_active)).collect()
            };
            for r in rows {
                if let Some(row) = ordered.get_mut(*r) {
                    row.1 = on;
                }
            }
            let order: Vec<Uuid> = ordered.iter().map(|r| r.0).collect();
            let ticks: Vec<bool> = ordered.iter().map(|r| r.1).collect();
            reindex(infos, &order, &ticks);
        });
    }

    /// Enable All and Disable All.
    pub fn set_all_controllers(&mut self, on: bool, live: &[ControllerInfo]) {
        let count = self.active.as_ref().and_then(|a| a.controllers.as_ref()).map_or(0, Vec::len);
        let rows: Vec<usize> = (0..count).collect();
        self.set_controllers_active(&rows, on, live);
    }

    /// Each enables only when it would change something.
    pub fn enable_all_enabled(&self) -> bool {
        self.active.as_ref().and_then(|a| a.controllers.as_ref()).is_some_and(|c| c.iter().any(|c| !c.is_active))
    }

    pub fn disable_all_enabled(&self) -> bool {
        self.active.as_ref().and_then(|a| a.controllers.as_ref()).is_some_and(|c| c.iter().any(|c| c.is_active))
    }

    /// A drag, in `move(fromOffsets:toOffset:)`'s terms.
    pub fn move_controllers(&mut self, from: &[usize], to: usize, live: &[ControllerInfo]) {
        self.apply_to_controllers(live, |infos| move_controller_rows(infos, from, to));
    }

    // Stage 4: format and destination.

    pub fn interval_text(&self) -> &str {
        &self.interval_text
    }

    /// The Timing box: stored when it's a usable integer, as `IsIntervalValid` stores it.
    pub fn set_interval_text(&mut self, text: &str) {
        self.interval_text = text.to_string();
        if let Ok(value) = validate_interval(text) {
            if let Some(active) = self.active_mut() {
                active.interval = value;
            }
        }
    }

    pub fn falcon_audio_box(&self) -> bool {
        self.falcon_audio_box
    }

    pub fn audio_box(&self) -> bool {
        self.audio_box
    }

    /// Which group shows: `grpFalcon` for the writers that call themselves Falcon, `grpSequence` and `grpAudio` otherwise. KEEP: the
    /// older FSEQ format shows the Sequence and Audio groups though it's validated as Falcon.
    pub fn shows_falcon_group(&self) -> bool {
        self.active.as_ref().and_then(ExportProfile::export_format).is_some_and(ExportFormat::is_falcon_format)
    }

    /// FIX: Create and Backup Universe File only do anything for 2.6+, so they only show there.
    pub fn shows_universe_options(&self) -> bool {
        self.active.as_ref().is_some_and(ExportProfile::is_falcon_2x_format)
    }

    pub fn compress_enabled(&self) -> bool {
        self.active.as_ref().and_then(ExportProfile::export_format).is_some_and(ExportFormat::can_compress)
    }

    /// The Format list.
    pub fn set_format(&mut self, index: usize) {
        let Some(format) = sorted_formats().get(index).copied() else { return };
        let Some(active) = self.active.as_mut() else { return };
        active.format = Some(format.description().to_string());
        let (falcon, include) = (active.is_falcon_format(), active.include_audio);
        let falcon_folder = active.falcon_output_folder.clone();
        // The check was for the mode the old format had.
        self.host_reachable = false;
        if falcon {
            if self.falcon_audio_box != include {
                self.falcon_audio_box = include;
                self.include_audio_changed(true);
            }
            if !self.direct_upload() {
                self.update_falcon_paths(falcon_folder.as_deref());
            }
        } else {
            self.audio_box = include;
        }
    }

    /// Either Include Audio box. On 2.6+ it sets Rename too, so the audio is always renamed when it's included there. FIX: on
    /// `Falcon Player Effect` the visible box works, where upstream reads the hidden one.
    pub fn set_include_audio(&mut self, falcon_box: bool, on: bool) {
        if falcon_box {
            self.falcon_audio_box = on;
        } else {
            self.audio_box = on;
        }
        self.include_audio_changed(falcon_box);
    }

    fn include_audio_changed(&mut self, from_falcon_box: bool) {
        let (falcon, audio) = (self.falcon_audio_box, self.audio_box);
        let Some(active) = self.active_mut() else { return };
        if active.is_falcon_2x_format() {
            active.rename_audio = falcon;
            active.include_audio = falcon;
        } else if active.is_falcon_effect_format() && from_falcon_box {
            active.include_audio = falcon;
        } else {
            active.include_audio = audio;
        }
    }

    pub fn set_rename_audio(&mut self, on: bool) {
        if let Some(active) = self.active_mut() {
            active.rename_audio = on;
        }
    }

    pub fn set_create_universe_file(&mut self, on: bool) {
        if let Some(active) = self.active_mut() {
            active.create_universe_file = on;
        }
    }

    pub fn set_backup_universe_file(&mut self, on: bool) {
        if let Some(active) = self.active_mut() {
            active.backup_universe_file = on;
        }
    }

    pub fn set_compression(&mut self, on: bool) {
        if let Some(active) = self.active_mut() {
            active.enable_compression = on;
        }
    }

    /// The Sequence group's folder.
    pub fn set_output_folder(&mut self, folder: &str) {
        if let Some(active) = self.active_mut() {
            active.output_folder = Some(folder.to_string());
        }
    }

    /// The Audio group's folder.
    pub fn set_audio_folder(&mut self, folder: &str) {
        if let Some(active) = self.active_mut() {
            active.audio_output_folder = Some(folder.to_string());
        }
    }

    /// The Falcon group's folder, chosen or typed: `UpdateFalconPaths`, which also points the sequence and audio folders inside it.
    pub fn set_falcon_folder(&mut self, folder: &str) {
        self.update_falcon_paths(Some(folder));
    }

    fn update_falcon_paths(&mut self, folder: Option<&str>) {
        let Some(active) = self.active_mut() else { return };
        active.falcon_output_folder = folder.map(str::to_string);
        if let Some(folder) = folder.filter(|f| !f.is_empty()) {
            let sub = if active.is_falcon_effect_format() { "effects" } else { "sequences" };
            active.output_folder = Some(join_text(folder, sub));
            active.audio_output_folder = Some(join_text(folder, "music"));
        }
    }

    /// The `File Path` and `Direct Upload` radios. FIX: they show for 2.6+ only, the one format whose export uploads.
    pub fn shows_direct_upload(&self) -> bool {
        self.active.as_ref().is_some_and(ExportProfile::is_falcon_2x_format)
    }

    /// Whether this export uploads. FIX: a profile left on Direct Upload counts as File Path on any other format, where upstream
    /// keeps demanding a host from a box that isn't showing and then writes to the folder anyway.
    pub fn direct_upload(&self) -> bool {
        self.active.as_ref().is_some_and(|a| a.is_falcon_2x_format() && a.fpp_direct_upload)
    }

    pub fn set_direct_upload(&mut self, on: bool) {
        self.host_reachable = false;
        if let Some(active) = self.active_mut() {
            active.fpp_direct_upload = on;
        }
    }

    pub fn host_text(&self) -> &str {
        self.active.as_ref().and_then(|a| a.fpp_host_address.as_deref()).unwrap_or_default()
    }

    /// Typing in the host box: the trimmed text is kept, and the last check no longer counts.
    pub fn set_host_text(&mut self, text: &str) {
        self.host_reachable = false;
        if let Some(active) = self.active_mut() {
            active.fpp_host_address = Some(text.trim().to_string());
        }
    }

    pub fn host_reachable(&self) -> bool {
        self.host_reachable
    }

    /// The address to ping, off the main thread, on leaving the host box or opening the stage: only a valid one in Direct Upload.
    pub fn host_to_check(&self) -> Option<String> {
        let host = self.host_text();
        (self.stage == Stage::OutputFormat && self.direct_upload() && vixen_fpp::is_host_address_valid(host)).then(|| host.to_string())
    }

    /// The ping's answer. One for an address that has since changed is dropped. Returns whether it was taken.
    pub fn set_host_reachable(&mut self, host: &str, reachable: bool) -> bool {
        if self.host_to_check().as_deref() != Some(host) {
            return false;
        }
        self.host_reachable = reachable;
        true
    }

    // Stage 5: summary.

    /// The host the Summary asks for its system info (F28): any non-blank one in Direct Upload.
    pub fn device_host(&self) -> Option<String> {
        let host = self.host_text();
        (self.stage == Stage::Summary && self.direct_upload() && !host.trim().is_empty()).then(|| host.to_string())
    }

    /// The device's answer, or that it gave none. One for another host or a stage since left is dropped.
    pub fn set_device_info(&mut self, host: &str, info: DeviceInfo) -> bool {
        if self.device_host().as_deref() != Some(host) {
            return false;
        }
        self.device = info;
        true
    }

    pub fn device(&self) -> &DeviceInfo {
        &self.device
    }

    pub fn network_request(&self) -> Option<u64> {
        (self.stage == Stage::Summary && matches!(self.network_preparation, NetworkPreparation::Pending)).then_some(self.network_generation)
    }

    pub fn complete_network(&mut self, generation: u64, result: Result<Vec<ControllerInfo>, String>) -> bool {
        if self.network_request() != Some(generation) { return false; }
        self.network_preparation = match result {
            Ok(controllers) => NetworkPreparation::Ready(controllers),
            Err(message) => NetworkPreparation::Failed(format!("Controller network preparation failed: {message}. Go back and return to retry.")),
        };
        true
    }

    pub fn network_error(&self) -> Option<&str> {
        match &self.network_preparation { NetworkPreparation::Failed(message) => Some(message), _ => None }
    }

    pub fn cancel_network(&mut self) {
        self.network_generation = self.network_generation.wrapping_add(1);
        self.network_preparation = NetworkPreparation::NotNeeded;
    }

    /// F31 step 1: the run found an ESPixelStick, whatever the Summary's own question came to.
    pub fn set_esp_detected(&mut self, supports_zip: bool) {
        self.esp_detected = Some(supports_zip);
    }

    /// F27. `live` is every controller with its network configuration, which the warning needs.
    pub fn summary(&self, live: &[ControllerInfo]) -> Option<Summary> {
        let live = match &self.network_preparation { NetworkPreparation::Ready(controllers) => controllers.as_slice(), _ => live };
        let active = self.active.as_ref()?;
        let audio = if !active.include_audio {
            "Not included."
        } else if active.rename_audio {
            "Rename to match sequence name."
        } else {
            "Include as is."
        };
        let controllers = profile::controller_infos(active, live);
        let mut summary = Summary {
            sequences: active.sequence_files.len(),
            format: active.format_text().to_string(),
            timing: format!("{} ms", active.interval),
            destination: active.output_folder.clone().unwrap_or_default(),
            audio: audio.to_string(),
            audio_destination: active.include_audio.then(|| active.audio_output_folder.clone().unwrap_or_default()),
            universe_destination: (active.is_falcon_format() && active.create_universe_file)
                .then(|| active.falcon_output_folder.clone().unwrap_or_default()),
            warning: (active.is_falcon_2x_format() && active.create_universe_file && !all_selected_support_universes(&controllers))
                .then(|| SUMMARY_WARNING.to_string()),
            device_heading: None,
            device_rows: Vec::new(),
        };
        if !self.direct_upload() {
            return Some(summary);
        }
        // FIX: the destinations name the device, where upstream goes on showing the folders File Path would write to.
        let host = self.host_text().to_string();
        summary.destination = host.clone();
        summary.audio_destination = summary.audio_destination.map(|_| host.clone());
        summary.universe_destination = active.create_universe_file.then_some(host);
        // `UpdateEspPixelStickSummary`: such a device takes sequences and nothing else.
        if let Some(zip) = self.esp_detected.or(self.device.esp_pixel_stick_zip()) {
            summary.audio = ESP_NOT_UPLOADED.to_string();
            summary.audio_destination = None;
            summary.universe_destination = (active.create_universe_file || active.backup_universe_file).then(|| ESP_NOT_UPLOADED.to_string());
            summary.warning = Some(esp_warning(zip));
        }
        match &self.device {
            DeviceInfo::Unknown => {}
            DeviceInfo::Unavailable => summary.device_heading = Some(DEVICE_UNAVAILABLE.to_string()),
            DeviceInfo::Known { host_name, description, platform, variant, .. } => {
                summary.device_heading = Some(DEVICE_HEADING.to_string());
                summary.device_rows = [("Host Name", host_name), ("Description", description), ("Platform", platform), ("Variant", variant)]
                    .into_iter()
                    .map(|(label, value)| (label.to_string(), value.clone()))
                    .collect();
            }
        }
        Some(summary)
    }

    /// F29, at the Summary's Next: with the box ticked, a name from the list replaces that profile and a new name adds one. An exact
    /// name match counts as choosing that entry, where upstream depends on the combo's selection and can save two of one name. Returns
    /// true when the list changed and should be saved.
    pub fn apply_save_settings(&mut self) -> bool {
        if !self.save_settings {
            return false;
        }
        let name = self.save_name.trim().to_string();
        let Some(mut copy) = self.active.clone() else { return false };
        if name.is_empty() {
            return false;
        }
        copy.name = Some(name.clone());
        match self.profiles.iter().position(|p| p.name_text() == name) {
            Some(i) => {
                copy.id = self.profiles[i].id;
                self.profiles[i] = copy;
            }
            None => {
                copy.id = Uuid::new_v4();
                self.profiles.push(copy);
            }
        }
        true
    }

    /// The Summary's Next has started the run: the wizard moves to Finished when it ends, even after a failure, as upstream.
    pub fn finish(&mut self, final_text: &str) {
        self.stage = Stage::Finished;
        self.active = None;
        // FIX: upstream says `Export Complete` even after a failure or a cancel.
        self.final_text = if final_text == "Completed" { "Export Complete".to_string() } else { final_text.to_string() };
    }

    pub fn final_text(&self) -> &str {
        &self.final_text
    }
}

/// `Path.Combine` on text: a separator is added only when the first part doesn't end in one. A Windows path keeps its backslashes.
fn join_text(folder: &str, name: &str) -> String {
    if folder.ends_with('/') || folder.ends_with('\\') {
        format!("{folder}{name}")
    } else if folder.contains('\\') && !folder.contains('/') {
        format!("{folder}\\{name}")
    } else {
        format!("{folder}/{name}")
    }
}

#[cfg(test)]
mod tests;
