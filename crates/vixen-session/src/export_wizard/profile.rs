//! `ExportProfile` and `BulkExportWizardData`: the wizard's saved settings in `SystemData/ModuleStore.xml`, read and written as the
//! data contract serializer does, members in alphabetical order. A profile the user hasn't changed goes back exactly as it was read,
//! Windows paths and all, so the file still loads on Windows.

use std::path::Path;
use uuid::Uuid;
use vixen_export::fpp_config::ControllerInfo;
use vixen_export::ExportFormat;
use vixen_io::dcs::{format_bool, format_guid, parse_bool, parse_guid};
use vixen_io::{ModuleEntry, ModuleStore, SystemData};
use vixen_xml::{XElement, ARRAYS_NS};

/// `ExportWizardDescriptor`'s type id. Its data is the module's static data, keyed by this.
pub const MODULE_TYPE: Uuid = uuid::uuid!("8bf8d0c5-b7d6-417c-9eff-ec1c5e1ef9ab");
const DATA_MODEL_TYPE: &str = "VixenModules.App.ExportWizard.BulkExportWizardData, ExportWizard";
const ASSEMBLY_NAME: &str = "Module.App.ExportWizard";
const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.ExportWizard";
const EXPORT_NS: &str = "http://schemas.datacontract.org/2004/07/Vixen.Export";

/// `Export.Controller` as a profile stores it: names and channel counts come from the live controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileController {
    pub id: Uuid,
    pub index: i32,
    pub is_active: bool,
}

/// `ExportProfile`. Strings are `None` where the file has `i:nil`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportProfile {
    pub name: Option<String>,
    pub id: Uuid,
    pub sequence_files: Vec<String>,
    pub interval: i32,
    pub format: Option<String>,
    pub output_folder: Option<String>,
    pub include_audio: bool,
    pub rename_audio: bool,
    pub audio_output_folder: Option<String>,
    pub create_universe_file: bool,
    pub backup_universe_file: bool,
    pub falcon_output_folder: Option<String>,
    pub fpp_direct_upload: bool,
    pub fpp_host_address: Option<String>,
    /// `None` when the file has none, which `SyncronizeControllerInfo` fills with every controller.
    pub controllers: Option<Vec<ProfileController>>,
    pub enable_compression: bool,
}

impl ExportProfile {
    pub fn name_text(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    pub fn format_text(&self) -> &str {
        self.format.as_deref().unwrap_or("")
    }

    /// The writer the format names, if it names one.
    pub fn export_format(&self) -> Option<ExportFormat> {
        ExportFormat::from_description(self.format_text())
    }

    /// `IsFalconFormat`: by name, so `Falcon Player Sequence` (1.0) counts, unlike `Export.IsFalconFormat`.
    pub fn is_falcon_format(&self) -> bool {
        self.format_text().contains("Falcon")
    }

    /// `IsFalcon2xFormat`.
    pub fn is_falcon_2x_format(&self) -> bool {
        self.format_text().contains("Falcon Player Sequence 2.6+")
    }

    /// `IsFalconEffectFormat`.
    pub fn is_falcon_effect_format(&self) -> bool {
        self.format_text() == "Falcon Player Effect"
    }

    /// `OnDeserialized`: a Falcon profile from before `FalconOutputFolder` existed gets the parent of its `sequences` folder. Done on the
    /// path's text with both separators, since a stored path is usually a Windows one, as `Directory.GetParent` would on Windows. KEEP: a
    /// trailing separator makes the "parent" the `sequences` folder itself, which is what `GetParent` gives too.
    pub fn on_deserialized(&mut self) {
        if !self.is_falcon_format() || self.falcon_output_folder.as_deref().is_some_and(|f| !f.is_empty()) {
            return;
        }
        let Some(output) = self.output_folder.as_deref() else { return };
        if output.ends_with("sequences") || output.ends_with("sequences/") {
            self.falcon_output_folder = Some(directory_name(output).to_string());
        }
    }
}

/// `Path.GetDirectoryName`, on text with either separator.
pub fn directory_name(path: &str) -> &str {
    match path.rfind(['/', '\\']) {
        Some(0) => &path[..1],
        Some(i) if i == 2 && path.as_bytes().get(1) == Some(&b':') => &path[..3],
        Some(i) => &path[..i],
        None => "",
    }
}

/// `Export.CreateControllerInfo(true, false)`: every controller in system order, ticked.
pub fn all_controllers(system: &SystemData) -> Vec<ProfileController> {
    system.controllers.iter().enumerate().map(|(i, c)| ProfileController { id: c.id, index: i as i32, is_active: true }).collect()
}

/// `SyncronizeControllerInfo`: the live controllers, taking the tick and place of each one the profile knows, a new one going last,
/// ticked, in system order; one deleted since is dropped. Then renumbered from 0.
pub fn synchronize_controllers(profile: &mut ExportProfile, system: &SystemData) {
    let Some(stored) = profile.controllers.as_ref() else {
        profile.controllers = Some(all_controllers(system));
        return;
    };
    let mut current: Vec<ProfileController> = system
        .controllers
        .iter()
        .map(|c| match stored.iter().find(|s| s.id == c.id) {
            Some(s) => ProfileController { id: c.id, index: s.index, is_active: s.is_active },
            None => ProfileController { id: c.id, index: i32::MAX, is_active: true },
        })
        .collect();
    current.sort_by_key(|c| c.index);
    for (i, c) in current.iter_mut().enumerate() {
        c.index = i as i32;
    }
    profile.controllers = Some(current);
}

/// The profile's controllers with the live names, channel counts and, when asked, network configuration: what the export runs with.
pub fn controller_infos(profile: &ExportProfile, live: &[ControllerInfo]) -> Vec<ControllerInfo> {
    let stored = profile.controllers.as_deref().unwrap_or(&[]);
    live.iter()
        .filter_map(|l| {
            stored.iter().find(|s| s.id == l.id).map(|s| ControllerInfo { index: s.index, is_active: s.is_active, ..l.clone() })
        })
        .collect()
}

/// `CreateDefaultProfile`: `Default`, the default format, both folders `<data root>/Export`, compression on, 50 ms, every controller.
pub fn default_profile(name: &str, data_root: &Path, system: &SystemData) -> ExportProfile {
    let export = vixen_export::default_export_dir(data_root).to_string_lossy().into_owned();
    ExportProfile {
        name: Some(name.to_string()),
        id: Uuid::new_v4(),
        sequence_files: Vec::new(),
        interval: 50,
        format: Some(ExportFormat::DEFAULT.description().to_string()),
        output_folder: Some(export.clone()),
        include_audio: false,
        rename_audio: false,
        audio_output_folder: Some(export),
        create_universe_file: false,
        backup_universe_file: false,
        falcon_output_folder: None,
        fpp_direct_upload: false,
        fpp_host_address: None,
        controllers: Some(all_controllers(system)),
        enable_compression: true,
    }
}

/// Why a profile name was refused, with the alert each one gets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameError {
    /// Title `Error`.
    Blank,
    /// Title `Warning`.
    Duplicate(String),
}

impl NameError {
    pub fn title(&self) -> &'static str {
        match self {
            NameError::Blank => "Error",
            NameError::Duplicate(_) => "Warning",
        }
    }

    pub fn message(&self) -> String {
        match self {
            NameError::Blank => "Profile name can not be blank.".to_string(),
            NameError::Duplicate(name) => format!("A profile with the name {name} already exists."),
        }
    }
}

/// F19: trimmed, not blank, and not the name of another profile (an exact compare), leaving out `exclude`, the one being renamed.
pub fn validate_profile_name(existing: &[&str], candidate: &str, exclude: Option<&str>) -> Result<String, NameError> {
    let name = candidate.trim();
    if name.is_empty() {
        return Err(NameError::Blank);
    }
    if existing.iter().any(|n| *n == name && Some(*n) != exclude) {
        return Err(NameError::Duplicate(name.to_string()));
    }
    Ok(name.to_string())
}

// The data contract.

fn text_member(e: &XElement, name: &str) -> Option<String> {
    e.child(name).filter(|c| !c.nil).map(XElement::text)
}

fn bool_member(e: &XElement, name: &str) -> bool {
    e.child_text(name).and_then(|t| parse_bool(&t)).unwrap_or(false)
}

fn decode_profile(e: &XElement) -> ExportProfile {
    let controllers = e.child("Controllers").filter(|c| !c.nil).map(|list| {
        list.elements()
            .filter(|c| c.local == "Controller")
            .map(|c| ProfileController {
                id: c.child_text("Id").and_then(|t| parse_guid(&t)).unwrap_or_default(),
                index: c.child_text("Index").and_then(|t| t.trim().parse().ok()).unwrap_or(0),
                is_active: c.child_text("IsActive").and_then(|t| parse_bool(&t)).unwrap_or(false),
            })
            .collect()
    });
    let sequence_files = e
        .child("SequenceFiles")
        .map(|list| list.elements().filter(|s| s.local == "string" && !s.nil).map(XElement::text).collect())
        .unwrap_or_default();
    ExportProfile {
        name: text_member(e, "Name"),
        id: e.child_text("Id").and_then(|t| parse_guid(&t)).unwrap_or_default(),
        sequence_files,
        interval: e.child_text("Interval").and_then(|t| t.trim().parse().ok()).unwrap_or(0),
        format: text_member(e, "Format"),
        output_folder: text_member(e, "OutputFolder"),
        include_audio: bool_member(e, "IncludeAudio"),
        rename_audio: bool_member(e, "RenameAudio"),
        audio_output_folder: text_member(e, "AudioOutputFolder"),
        create_universe_file: bool_member(e, "CreateUniverseFile"),
        backup_universe_file: bool_member(e, "BackupUniverseFile"),
        falcon_output_folder: text_member(e, "FalconOutputFolder"),
        fpp_direct_upload: bool_member(e, "FppDirectUpload"),
        fpp_host_address: text_member(e, "FppHostAddress"),
        controllers,
        enable_compression: bool_member(e, "EnableCompression"),
    }
}

fn string_element(ns: &str, name: &str, value: &Option<String>) -> XElement {
    match value {
        Some(v) => XElement::with_text(ns, name, v.clone()),
        None => XElement::nil(ns, name),
    }
}

fn encode_profile(p: &ExportProfile) -> XElement {
    let mut e = XElement::new(NS, "ExportProfile");
    let flag = |e: &mut XElement, name: &str, value: bool| {
        // `EmitDefaultValue = false`.
        if value {
            e.push(XElement::with_text(NS, name, format_bool(true)));
        }
    };
    e.push(string_element(NS, "AudioOutputFolder", &p.audio_output_folder));
    flag(&mut e, "BackupUniverseFile", p.backup_universe_file);
    match &p.controllers {
        Some(list) => {
            let mut controllers = XElement::new(NS, "Controllers");
            for c in list {
                let mut item = XElement::new(EXPORT_NS, "Controller");
                item.push(XElement::with_text(EXPORT_NS, "Id", format_guid(&c.id)));
                item.push(XElement::with_text(EXPORT_NS, "Index", c.index.to_string()));
                item.push(XElement::with_text(EXPORT_NS, "IsActive", format_bool(c.is_active)));
                controllers.push(item);
            }
            e.push(controllers);
        }
        None => {
            e.push(XElement::nil(NS, "Controllers"));
        }
    }
    flag(&mut e, "CreateUniverseFile", p.create_universe_file);
    flag(&mut e, "EnableCompression", p.enable_compression);
    e.push(string_element(NS, "FalconOutputFolder", &p.falcon_output_folder));
    e.push(string_element(NS, "Format", &p.format));
    flag(&mut e, "FppDirectUpload", p.fpp_direct_upload);
    if let Some(host) = &p.fpp_host_address {
        e.push(XElement::with_text(NS, "FppHostAddress", host.clone()));
    }
    e.push(XElement::with_text(NS, "Id", format_guid(&p.id)));
    flag(&mut e, "IncludeAudio", p.include_audio);
    e.push(XElement::with_text(NS, "Interval", p.interval.to_string()));
    e.push(string_element(NS, "Name", &p.name));
    e.push(string_element(NS, "OutputFolder", &p.output_folder));
    flag(&mut e, "RenameAudio", p.rename_audio);
    let mut files = XElement::new(NS, "SequenceFiles");
    for f in &p.sequence_files {
        files.push(XElement::with_text(ARRAYS_NS, "string", f.clone()));
    }
    e.push(files);
    e
}

fn encode_data(profiles: &[ExportProfile]) -> XElement {
    let mut data = XElement::new(NS, "BulkExportWizardData");
    data.push(XElement::with_text("", "ModuleInstanceId", format_guid(&MODULE_TYPE)));
    data.push(XElement::with_text("", "ModuleTypeId", format_guid(&MODULE_TYPE)));
    let mut list = XElement::new(NS, "Profiles");
    for p in profiles {
        list.push(encode_profile(p));
    }
    data.push(list);
    data
}

fn decode_data(data: &XElement) -> Vec<ExportProfile> {
    data.child("Profiles")
        .map(|list| list.elements().filter(|e| e.local == "ExportProfile").map(decode_profile).collect())
        .unwrap_or_default()
}

/// The saved profiles, each run through `OnDeserialized`. None stored is an empty list, as `OnDeserialization` makes it.
pub fn load_profiles(store: &ModuleStore) -> Vec<ExportProfile> {
    let mut profiles = store.type_data(MODULE_TYPE).and_then(ModuleEntry::data).map(decode_data).unwrap_or_default();
    for p in &mut profiles {
        p.on_deserialized();
    }
    profiles
}

/// Writes the profiles into the store, keeping whatever the file said that the typed form leaves out, and every value that didn't
/// change exactly as it was written. Saving the file is the caller's (`AppCore::save_module_config`).
pub fn save_profiles(store: &mut ModuleStore, profiles: &[ExportProfile]) {
    let new_plain = encode_data(profiles);
    match store.type_data_mut(MODULE_TYPE) {
        Some(entry) => {
            let old_plain = encode_data(&entry.data().map(decode_data).unwrap_or_default());
            entry.merge_data(&old_plain, new_plain);
        }
        None => store.set_type_data(ModuleEntry::new(DATA_MODEL_TYPE, &MODULE_TYPE, &MODULE_TYPE, ASSEMBLY_NAME, new_plain)),
    }
}

/// True for text that isn't an absolute folder on this Mac, such as a Windows path from a profile made there. GUARD: such a folder is
/// never offered for creation; the user picks one instead.
pub fn is_foreign_path(path: &str) -> bool {
    !path.starts_with('/') || path.contains('\\')
}
