//! Timed sequences: `.tim` files. Port of `TimedSequenceData`, `SequenceTypeDataModelBase` and its surrogates.
//!
//! A sequence is typed on load and encoded again on save. Whatever typed values cannot express is kept as
//! residue (see `fidelity.rs`), so a file that was not edited is written back with nothing lost or changed.

use crate::dcs::{
    child_bool, child_f64, child_guid, child_i32, child_string, child_time, child_value, format_bool, format_double,
    format_guid, opt_string_element,
};
use crate::error::IoError;
use crate::fidelity::{diff, diff_opt, reapply, reapply_opt, Residue};
use crate::file;
use crate::params::{descriptor, effect_model_to_xml, effect_params_from_xml, unknown_params_from_xml};
use crate::sequence_migrate::{self, MigrationContext};
use crate::values::{color_from_xml, color_to_xml};
use crate::xml::{read_xml_file, write_document, DocStyle};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::{uuid, Uuid};
use vixen_core::layer::{ChromaKeySettings, Layer, LayerType, MixingFilter, SequenceLayers};
use vixen_core::marks::{Mark, MarkCollection, MarkCollectionType, MarkDecorator};
use vixen_core::{Color, TimeSpan};
use vixen_effects::framework::Params;
use vixen_xml::{XElement, ARRAYS_NS, XSD_NS, XSI_NS};

/// `TimedSequenceModuleDescriptor.ObjectVersion`.
pub const CURRENT_SEQUENCE_VERSION: i32 = 9;
/// `TimedSequenceModuleDescriptor.TypeId`.
pub const TIMED_SEQUENCE_TYPE_ID: Uuid = uuid!("296bdba2-9bf3-4bff-a9f2-13efac5c8ecb");

pub(crate) const NS_TIMED: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed";
const NS_LAYERS: &str = "http://schemas.datacontract.org/2004/07/Vixen.Sys.LayerMixing";
const NS_MARKS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.Marks";
const NS_LUMA: &str = "http://schemas.datacontract.org/2004/07/VixenModules.LayerMixingFilter.LumaKey";
const NS_CHROMA: &str = "http://schemas.datacontract.org/2004/07/VixenModules.LayerMixingFilter.ChromaKey";
const NS_MASK_FILL: &str = "http://schemas.datacontract.org/2004/07/VixenModules.LayerMixingFilter.MaskFill";

/// The element name DataContractSerializer gives `Dictionary<string, RowSetting>` entries.
const ROW_SETTING_ITEM: &str = "KeyValueOfstringRowSettingEF_SIuqLd";

const SEQUENCE_STYLE: DocStyle = DocStyle { root_prefixes: &[("a", XSD_NS), ("i", XSI_NS)], object_depth: None };

/// The module type ids of the layer mixing filters, from each filter's descriptor. `vixen_core::layer::filter_ids`
/// assigns most of these ids to the wrong filter, and the real files agree with the C#, so they are not used.
pub mod mixing_filter_ids {
    use uuid::{uuid, Uuid};
    pub const PROPORTIONAL_MIX: Uuid = uuid!("1f16d67a-1fff-485c-8e76-27ff7e7d072b");
    pub const HIGHEST_VALUE: Uuid = uuid!("01d6d526-9586-44bb-82f4-10abacf58b61");
    pub const MASK: Uuid = uuid!("ace0f097-7982-4edc-b5dd-8989dce60e1d");
    pub const MASK_FILL: Uuid = uuid!("24878b18-14f8-458e-abaa-3a9c9a34931c");
    pub const MULTIPLY_COLOR: Uuid = uuid!("0c9dfc0b-8e39-4167-8036-79408c6b0b35");
    pub const INTENSITY_OVERLAY: Uuid = uuid!("43f48bf5-4088-403f-ad7a-d0ff2114ca31");
    pub const COLOR_CHANGE: Uuid = uuid!("93d4169e-5138-403a-b71c-a5a1ab089801");
    pub const LUMA_KEY: Uuid = uuid!("7583f7d4-df99-4492-8d10-257c2677ff7b");
    pub const CHROMA_KEY: Uuid = uuid!("2d467733-1a64-45ad-bcdc-84bf074747e6");
}

/// The module type id a mixing filter is saved under.
pub fn mixing_filter_type_id(filter: &MixingFilter) -> Uuid {
    use mixing_filter_ids::*;
    match filter {
        MixingFilter::ProportionalMix => PROPORTIONAL_MIX,
        MixingFilter::HighestValue => HIGHEST_VALUE,
        MixingFilter::Mask => MASK,
        MixingFilter::MaskFill { .. } => MASK_FILL,
        MixingFilter::MultiplyColor => MULTIPLY_COLOR,
        MixingFilter::IntensityOverlay => INTENSITY_OVERLAY,
        MixingFilter::ColorChange => COLOR_CHANGE,
        MixingFilter::LumaKey { .. } => LUMA_KEY,
        MixingFilter::ChromaKey(_) => CHROMA_KEY,
    }
}

/// `SelectedTimingProvider`. Both fields are null when the sequence plays from its own clock.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectedTimingProvider {
    pub provider_type: Option<String>,
    pub source_name: Option<String>,
}

/// One entry of `_mediaSurrogates`, with the module data that goes with it.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaEntry {
    pub type_id: Uuid,
    pub instance_id: Uuid,
    /// The file's name, relative to the profile's Media folder.
    pub file_name: String,
    /// The media module's data model from `_dataModels` (such as `AudioData`), as the file holds it.
    pub data: Option<XElement>,
}

/// One effect on the timeline: an `EffectNodeSurrogate` joined to its data model.
///
/// `params` uses the effect's table when the registry knows `type_id`. For an effect type this build does not
/// know, `params.desc` is empty and `params.unknown_members` holds the whole data model, which is written back
/// unchanged. C# only ever reads the first target; the rest, if a file has them, are kept.
#[derive(Clone, Debug)]
pub struct EffectNode {
    pub start: TimeSpan,
    pub time_span: TimeSpan,
    pub type_id: Uuid,
    pub instance_id: Uuid,
    /// (node id, node name).
    pub targets: Vec<(Uuid, String)>,
    pub params: Params,
}

/// One entry of `_filterNodeSurrogates`, with the filter module's data as the file holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct SequenceFilterNode {
    pub start: TimeSpan,
    pub time_span: TimeSpan,
    pub type_id: Uuid,
    pub instance_id: Uuid,
    pub targets: Vec<(Uuid, String)>,
    pub data: Option<XElement>,
}

/// `RowSetting`: how the editor shows one row.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RowSetting {
    pub expanded: bool,
    pub row_height: i32,
    pub visible: bool,
}

/// A `.tim` file.
#[derive(Clone, Debug)]
pub struct TimedSequence {
    /// The root's `version`. Loading migrates, so this is [`CURRENT_SEQUENCE_VERSION`] unless the file is newer.
    pub version: i32,
    /// The version the file on disk had before migrating.
    pub source_version: i32,
    /// What the migrators reported, such as removed Nutcracker effects, for the UI to show.
    pub migration_notes: Vec<String>,
    /// The matrix image maps this file's LipSync effects named when it was migrated from before version 5. Upstream's migrator
    /// clears `UsingDefaults` on each in the live library; whoever opens the sequence does that here.
    pub migrated_matrix_maps: Vec<String>,
    /// Version 0 media paths before they became relative file names. Parsing
    /// preserves these without writing; an owning session restores reachable files.
    pub(crate) migrated_media_sources: Vec<(PathBuf, String)>,
    /// `ModuleDataModelBase.ModuleInstanceId`.
    pub module_instance_id: Uuid,
    /// `ModuleDataModelBase.ModuleTypeId`.
    pub module_type_id: Uuid,
    /// `SequenceTypeDataModelBase.Version`, which Vixen never sets.
    pub data_version: i32,
    pub length: TimeSpan,
    pub selected_timing_provider: SelectedTimingProvider,
    pub media: Vec<MediaEntry>,
    pub effects: Vec<EffectNode>,
    /// Sequence filter nodes, the `SequenceFilterData` stream.
    pub filters: Vec<SequenceFilterNode>,
    pub layers: SequenceLayers,
    pub marks: Vec<MarkCollection>,
    /// Row settings keyed as in the file, in file order.
    pub row_settings: Vec<(String, RowSetting)>,
    pub time_per_pixel: TimeSpan,
    pub visible_time_start: TimeSpan,
    pub default_row_height: i32,
    pub default_splitter_distance: i32,
    pub default_playback_start_time: Option<TimeSpan>,
    pub default_playback_end_time: Option<TimeSpan>,
    pub(crate) fidelity: SequenceFidelity,
}

/// Everything needed to write back what typed values do not hold.
#[derive(Clone, Debug, Default)]
pub(crate) struct SequenceFidelity {
    /// `_dataModels` entries in file order, as (type id, instance id).
    model_order: Vec<(Uuid, Uuid)>,
    /// Data models no surrogate refers to. C# drops them on its next save, but this port loses nothing.
    orphan_models: HashMap<(Uuid, Uuid), XElement>,
    /// Effects with no data model in the file, with what typed encoding said at load.
    absent_models: HashMap<(Uuid, Uuid), XElement>,
    effect_surrogates: HashMap<Uuid, Residue>,
    filter_surrogates: HashMap<Uuid, Residue>,
    media_surrogates: HashMap<Uuid, Residue>,
    layers: HashMap<Uuid, Residue>,
    filter_surrogate_residue: HashMap<Uuid, Residue>,
    filter_models: HashMap<Uuid, Residue>,
    /// Layers whose filter type this port does not know: (layer id, surrogate, data model).
    unknown_filters: Vec<(Uuid, XElement, Option<XElement>)>,
    /// `_effectLayerMap` keys in file order. `SequenceLayers` keeps them in a `HashMap`.
    effect_layer_order: Vec<Uuid>,
    marks: HashMap<Uuid, Residue>,
    /// Everything else, at the level of the root's members. Empty for a well-formed file.
    root: Option<Residue>,
}

/// What this port's encoding could not reproduce for a loaded sequence, for reports and tests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FidelityReport {
    /// Root members typed encoding did not reproduce from the data below it.
    pub root_members: Vec<String>,
    pub effect_surrogates: usize,
    pub filter_surrogates: usize,
    pub media_surrogates: usize,
    pub layers: usize,
    pub filter_models: usize,
    pub mark_collections: usize,
    pub orphan_models: usize,
    pub absent_models: usize,
}

impl TimedSequence {
    /// What typed values could not express. Useful to confirm that codecs are not silently echoing.
    pub fn fidelity_report(&self) -> FidelityReport {
        let f = &self.fidelity;
        let root_members = match &f.root {
            None => Vec::new(),
            Some(Residue::Whole { .. }) => vec!["<whole document>".to_string()],
            Some(Residue::Members(m)) => m.iter().map(|m| m.name().to_string()).collect(),
        };
        FidelityReport {
            root_members,
            effect_surrogates: f.effect_surrogates.len(),
            filter_surrogates: f.filter_surrogates.len(),
            media_surrogates: f.media_surrogates.len(),
            layers: f.layers.len() + f.filter_surrogate_residue.len(),
            filter_models: f.filter_models.len(),
            mark_collections: f.marks.len(),
            orphan_models: f.orphan_models.len(),
            absent_models: f.absent_models.len(),
        }
    }

    /// The media entry's file in the profile's Media folder.
    pub fn media_path(&self, index: usize, paths: &crate::DataPaths) -> Option<std::path::PathBuf> {
        self.media.get(index).map(|m| paths.media_dir().join(&m.file_name))
    }

    /// The version 0 migrator's File.Copy into Media, after the caller has chosen
    /// the destination profile. Existing files always win, including a file
    /// another opener publishes while this copy is in progress.
    pub fn restore_migrated_media(&mut self, media_dir: &Path) -> Result<(), IoError> {
        for (source, name) in &self.migrated_media_sources {
            let target = media_dir.join(name);
            if !source.is_file() || target.is_file() {
                continue;
            }
            std::fs::create_dir_all(media_dir).map_err(|e| IoError::io(media_dir, e))?;
            let temporary = media_dir.join(format!(".vixen-media-{}.part", Uuid::new_v4()));
            let mut temporary_created = false;
            let copy = (|| {
                let mut input = std::fs::File::open(source).map_err(|e| IoError::io(source, e))?;
                let mut output = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)
                    .map_err(|e| IoError::io(&temporary, e))?;
                temporary_created = true;
                std::io::copy(&mut input, &mut output).map_err(|e| IoError::io(&target, e))?;
                output.sync_all().map_err(|e| IoError::io(&temporary, e))?;
                drop(output);
                match publish_media_copy(&temporary, &target) {
                    Ok(()) => Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && target.is_file() => Ok(()),
                    Err(e) => Err(IoError::io(&target, e)),
                }
            })();
            // The source is only read, and failures never leave a partial target.
            if temporary_created {
                let _ = std::fs::remove_file(&temporary);
            }
            copy?;
        }
        self.migrated_media_sources.clear();
        Ok(())
    }
}

/// Publish without replacing an existing file. macOS's exclusive rename also
/// works on volumes that do not support hard links (for example removable media).
#[cfg(target_os = "macos")]
fn publish_media_copy(temporary: &Path, target: &Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let from = CString::new(temporary.as_os_str().as_bytes())?;
    let to = CString::new(target.as_os_str().as_bytes())?;
    // SAFETY: both NUL-terminated paths remain alive throughout this call.
    if unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(target_os = "macos"))]
fn publish_media_copy(temporary: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::hard_link(temporary, target)
}

/// `new TimedSequenceData()` with the descriptor's identity, as `SequenceService.CreateNew` makes it.
pub fn new_sequence() -> TimedSequence {
    TimedSequence {
        version: CURRENT_SEQUENCE_VERSION,
        source_version: CURRENT_SEQUENCE_VERSION,
        migration_notes: Vec::new(),
        migrated_matrix_maps: Vec::new(),
        migrated_media_sources: Vec::new(),
        module_instance_id: Uuid::new_v4(),
        module_type_id: TIMED_SEQUENCE_TYPE_ID,
        data_version: 0,
        length: TimeSpan::ZERO,
        selected_timing_provider: SelectedTimingProvider::default(),
        media: Vec::new(),
        effects: Vec::new(),
        filters: Vec::new(),
        layers: SequenceLayers::default(),
        marks: Vec::new(),
        row_settings: Vec::new(),
        time_per_pixel: TimeSpan::MIN,
        visible_time_start: TimeSpan::MIN,
        default_row_height: 0,
        default_splitter_distance: 0,
        default_playback_start_time: Some(TimeSpan::ZERO),
        default_playback_end_time: Some(TimeSpan::ZERO),
        fidelity: SequenceFidelity::default(),
    }
}

/// Reads a sequence, migrating it to the current version in memory. Never writes: C# copies the file to
/// `<file>.<old version>` here, and this port does that in [`save_sequence`] instead, so that opening a
/// sequence leaves the folder untouched until the migrated content actually replaces the original.
pub fn load_sequence(path: &Path) -> Result<TimedSequence, IoError> {
    let root = read_xml_file(path)?;
    let mut context = MigrationContext::for_sequence_file(path);
    sequence_from_xml(root, &mut context).map_err(|e| with_context(e, path))
}

/// Like [`load_sequence`], looking up LipSync maps in a store the caller has already loaded.
pub fn load_sequence_with_store(
    path: &Path,
    store: &crate::module_store::ModuleStore,
) -> Result<TimedSequence, IoError> {
    let root = read_xml_file(path)?;
    let mut context = MigrationContext::with_store(store);
    sequence_from_xml(root, &mut context).map_err(|e| with_context(e, path))
}

fn with_context(e: IoError, path: &Path) -> IoError {
    match e {
        IoError::Format { message, .. } => IoError::Format { context: path.display().to_string(), message },
        other => other,
    }
}

/// Writes a sequence through `XElementFileWriter`'s backup rules, atomically.
pub fn save_sequence(sequence: &TimedSequence, path: &Path) -> Result<(), IoError> {
    let root = sequence_to_xml(sequence)?;
    let text = write_document(&root, SEQUENCE_STYLE);
    file::save_with_backups(path, text.as_bytes(), CURRENT_SEQUENCE_VERSION, Some(0), None)
}

/// Serialises a sequence to the text a file holds, without touching the file system.
pub fn sequence_to_document(sequence: &TimedSequence) -> Result<String, IoError> {
    Ok(write_document(&sequence_to_xml(sequence)?, SEQUENCE_STYLE))
}

/// Decodes a parsed `.tim` root, migrating it first.
pub(crate) fn sequence_from_xml(mut root: XElement, context: &mut MigrationContext) -> Result<TimedSequence, IoError> {
    if root.local != "TimedSequenceData" {
        return Err(IoError::format("sequence", format!("the root element is {}, not TimedSequenceData", root.local)));
    }
    // `SequenceXElementWriter.GetContentVersion`: a missing version is 0.
    let source_version = root.attr("version").and_then(|v| v.parse().ok()).filter(|v: &i32| *v > 0).unwrap_or(0);
    let migration_notes = sequence_migrate::migrate(&mut root, source_version, CURRENT_SEQUENCE_VERSION, context)?;
    split_shared_filter_instances(&mut root);
    let mut sequence = decode_root(&root, source_version, migration_notes)?;
    sequence.migrated_matrix_maps = std::mem::take(&mut context.matrix_maps_named);
    sequence.migrated_media_sources = std::mem::take(&mut context.media_sources);
    Ok(sequence)
}

/// FIX: upstream's Add Layer hands every layer added in one showing of the panel the same filter instance, so several surrogates can
/// name one `InstanceId` with one data model between them, and a file Windows saved after that will not open there again. Here each
/// layer owns its settings: a later surrogate naming an id already taken gets its own, with its own copy of the data. Done to the
/// document before it is read, so that nothing kept for the save can bring the shared id back. See docs/DIFFERENCES.md.
fn split_shared_filter_instances(root: &mut XElement) {
    let mut taken: HashSet<Uuid> = HashSet::new();
    let mut copies: Vec<(Uuid, Uuid, Uuid)> = Vec::new();
    let Some(surrogates) = root.child_mut("_layerMixingFilterSurrogates") else { return };
    for s in surrogates.elements_mut() {
        let (Some(instance), Some(type_id)) = (child_guid(s, "InstanceId"), child_guid(s, "TypeId")) else { continue };
        if taken.insert(instance) {
            continue;
        }
        let own = Uuid::new_v4();
        taken.insert(own);
        if let Some(e) = s.child_mut("InstanceId") {
            *e = XElement::with_text(&e.ns.clone(), "InstanceId", format_guid(&own));
        }
        copies.push((type_id, instance, own));
    }
    let Some(models) = root.child_mut("_layerMixingFilterDataModels") else { return };
    for (type_id, instance, own) in copies {
        let found = models.children.iter().position(|n| {
            matches!(n, vixen_xml::XNode::Element(m)
                if child_guid(m, "ModuleTypeId") == Some(type_id) && child_guid(m, "ModuleInstanceId") == Some(instance))
        });
        let Some(at) = found else { continue };
        let vixen_xml::XNode::Element(mut copy) = models.children[at].clone() else { continue };
        if let Some(e) = copy.child_mut("ModuleInstanceId") {
            *e = XElement::with_text(&e.ns.clone(), "ModuleInstanceId", format_guid(&own));
        }
        models.children.push(vixen_xml::XNode::Element(copy));
    }
}

fn decode_root(root: &XElement, source_version: i32, migration_notes: Vec<String>) -> Result<TimedSequence, IoError> {
    let mut f = SequenceFidelity::default();

    // Data models by (type id, instance id), in file order.
    let mut models: HashMap<(Uuid, Uuid), &XElement> = HashMap::new();
    if let Some(list) = root.child("_dataModels") {
        for m in list.elements() {
            let (Some(instance), Some(ty)) = (child_guid(m, "ModuleInstanceId"), child_guid(m, "ModuleTypeId")) else {
                // Not addressable by anything. The root residue keeps it.
                continue;
            };
            if models.insert((ty, instance), m).is_some() {
                f.orphan_models.insert((ty, instance), m.clone());
            }
            f.model_order.push((ty, instance));
        }
    }
    let mut claimed: HashSet<(Uuid, Uuid)> = HashSet::new();

    let mut effects = Vec::new();
    if let Some(list) = root.child("_effectNodeSurrogates") {
        for s in list.elements() {
            let node = decode_effect_surrogate(s, &models, &mut claimed, &mut f)?;
            let plain = effect_surrogate_to_xml(&node);
            if let Some(r) = diff(s, &plain) {
                f.effect_surrogates.insert(node.instance_id, r);
            }
            effects.push(node);
        }
    }

    let mut filters = Vec::new();
    if let Some(list) = root.child("_filterNodeSurrogates") {
        for s in list.elements() {
            let n = decode_node_surrogate(s)?;
            let key = (n.type_id, n.instance_id);
            let data = models.get(&key).map(|m| (*m).clone());
            claimed.insert(key);
            let node = SequenceFilterNode {
                start: n.start,
                time_span: n.time_span,
                type_id: n.type_id,
                instance_id: n.instance_id,
                targets: n.targets,
                data,
            };
            let plain = filter_surrogate_to_xml(&node);
            if let Some(r) = diff(s, &plain) {
                f.filter_surrogates.insert(node.instance_id, r);
            }
            filters.push(node);
        }
    }

    let mut media = Vec::new();
    if let Some(list) = root.child("_mediaSurrogates") {
        for s in list.elements() {
            let type_id = child_guid(s, "TypeId").unwrap_or_default();
            let instance_id = child_guid(s, "InstanceId").unwrap_or_default();
            let data = models.get(&(type_id, instance_id)).map(|m| (*m).clone());
            claimed.insert((type_id, instance_id));
            let entry =
                MediaEntry { type_id, instance_id, file_name: child_string(s, "FileName").unwrap_or_default(), data };
            let plain = media_surrogate_to_xml(&entry);
            if let Some(r) = diff(s, &plain) {
                f.media_surrogates.insert(instance_id, r);
            }
            media.push(entry);
        }
    }

    let layers = decode_layers(root, &mut f);

    let marks = decode_marks(root, &mut f);

    let row_settings = root
        .child("RowSettings")
        .map(|r| {
            r.elements()
                .filter_map(|kv| {
                    let key = child_string(kv, "Key")?;
                    let v = kv.child("Value")?;
                    Some((
                        key,
                        RowSetting {
                            expanded: child_bool(v, "Expanded").unwrap_or(false),
                            row_height: child_i32(v, "RowHeight").unwrap_or(0),
                            visible: child_bool(v, "Visible").unwrap_or(false),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    // Data models nothing claimed.
    for key in &f.model_order {
        if !claimed.contains(key) {
            if let Some(m) = models.get(key) {
                f.orphan_models.insert(*key, (*m).clone());
            }
        }
    }

    let version = root.attr("version").and_then(|v| v.parse().ok()).unwrap_or(CURRENT_SEQUENCE_VERSION);
    let time_or = |name: &str, default: TimeSpan| child_time(root, name).unwrap_or(default);

    let mut sequence = TimedSequence {
        version,
        source_version,
        migration_notes,
        migrated_matrix_maps: Vec::new(),
        migrated_media_sources: Vec::new(),
        module_instance_id: child_guid(root, "ModuleInstanceId").unwrap_or_default(),
        module_type_id: child_guid(root, "ModuleTypeId").unwrap_or(TIMED_SEQUENCE_TYPE_ID),
        data_version: child_i32(root, "Version").unwrap_or(0),
        // Missing members are `default(T)`: DataContractSerializer does not run the constructor.
        length: time_or("Length", TimeSpan::ZERO),
        selected_timing_provider: root
            .child("_selectedTimingProviderSurrogate")
            .map(|s| SelectedTimingProvider {
                provider_type: child_string(s, "ProviderType"),
                source_name: child_string(s, "SourceName"),
            })
            .unwrap_or_default(),
        media,
        effects,
        filters,
        layers,
        marks,
        row_settings,
        time_per_pixel: time_or("TimePerPixel", TimeSpan::ZERO),
        visible_time_start: time_or("VisibleTimeStart", TimeSpan::ZERO),
        default_row_height: child_i32(root, "DefaultRowHeight").unwrap_or(0),
        default_splitter_distance: child_i32(root, "DefaultSplitterDistance").unwrap_or(0),
        default_playback_start_time: child_time(root, "DefaultPlaybackStartTime"),
        default_playback_end_time: child_time(root, "DefaultPlaybackEndTime"),
        fidelity: f,
    };

    let plain = encode_root(&sequence)?;
    sequence.fidelity.root = diff(root, &plain);
    Ok(sequence)
}

/// The members `NodeSurrogate` gives effect and filter surrogates alike.
struct NodeSurrogate {
    start: TimeSpan,
    time_span: TimeSpan,
    type_id: Uuid,
    instance_id: Uuid,
    targets: Vec<(Uuid, String)>,
}

fn decode_node_surrogate(s: &XElement) -> Result<NodeSurrogate, IoError> {
    let instance_id =
        child_guid(s, "InstanceId").ok_or_else(|| IoError::format("sequence", "a surrogate has no InstanceId"))?;
    let type_id = child_guid(s, "TypeId")
        .ok_or_else(|| IoError::format("sequence", format!("surrogate {instance_id} has no TypeId")))?;
    let targets = s
        .child("TargetNodes")
        .map(|t| {
            t.elements()
                .map(|n| (child_guid(n, "NodeId").unwrap_or_default(), child_string(n, "Name").unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default();
    Ok(NodeSurrogate {
        start: child_time(s, "StartTime").unwrap_or_default(),
        time_span: child_time(s, "TimeSpan").unwrap_or_default(),
        type_id,
        instance_id,
        targets,
    })
}

fn decode_effect_surrogate(
    s: &XElement,
    models: &HashMap<(Uuid, Uuid), &XElement>,
    claimed: &mut HashSet<(Uuid, Uuid)>,
    f: &mut SequenceFidelity,
) -> Result<EffectNode, IoError> {
    let NodeSurrogate { start, time_span, type_id, instance_id, targets } = decode_node_surrogate(s)?;
    let key = (type_id, instance_id);
    let found = models.get(&key).copied();
    if found.is_some() {
        claimed.insert(key);
    }
    let params = match (found, descriptor(&type_id)) {
        (Some(m), Some(d)) => effect_params_from_xml(m, d),
        (Some(m), None) => params_for_unknown_type(m),
        (None, Some(d)) => {
            // `_GetOrCreateAsInstanceData`: an effect with no stored data gets a new data object.
            let params = d.new_params();
            f.absent_models.insert(key, effect_model_to_xml(&type_id, &instance_id, &params)?);
            params
        }
        (None, None) => Params { desc: &crate::params::NO_PARAMS, values: Vec::new(), unknown_members: Vec::new() },
    };
    Ok(EffectNode { start, time_span, type_id, instance_id, targets, params })
}

/// No effect has this module type id, though the registry may still know the data class by its contract name.
fn params_for_unknown_type(model: &XElement) -> Params {
    let known = model.xsi_type.as_ref().and_then(|(ns, local)| {
        let d = vixen_effect_registry::by_data_type(local)?;
        (ns.strip_prefix(vixen_xml::CONTRACT_BASE) == Some(d.data_ns)).then_some(d)
    });
    match known {
        Some(d) => {
            let mut params = effect_params_from_xml(model, d);
            crate::params::record_model_type(&mut params, model);
            params
        }
        None => unknown_params_from_xml(model),
    }
}

fn target_nodes_to_xml(targets: &[(Uuid, String)]) -> XElement {
    let mut list = XElement::new("", "TargetNodes");
    for (id, name) in targets {
        let mut n = XElement::new("", "ChannelNodeReferenceSurrogate");
        // A node removed from the system comes back with a null name, far more often than a node is
        // actually named nothing, so an empty name is written as nil.
        if name.is_empty() {
            n.push(XElement::nil("", "Name"));
        } else {
            n.push(XElement::with_text("", "Name", name.as_str()));
        }
        n.push(XElement::with_text("", "NodeId", format_guid(id)));
        list.push(n);
    }
    list
}

fn node_surrogate_to_xml(
    local: &str,
    instance_id: &Uuid,
    start: TimeSpan,
    targets: &[(Uuid, String)],
    time_span: TimeSpan,
    type_id: &Uuid,
) -> XElement {
    let mut e = XElement::new("", local);
    e.push(XElement::with_text("", "InstanceId", format_guid(instance_id)));
    e.push(XElement::with_text("", "StartTime", start.to_xml()));
    e.push(target_nodes_to_xml(targets));
    e.push(XElement::with_text("", "TimeSpan", time_span.to_xml()));
    e.push(XElement::with_text("", "TypeId", format_guid(type_id)));
    e
}

fn effect_surrogate_to_xml(n: &EffectNode) -> XElement {
    node_surrogate_to_xml("EffectNodeSurrogate", &n.instance_id, n.start, &n.targets, n.time_span, &n.type_id)
}

fn filter_surrogate_to_xml(n: &SequenceFilterNode) -> XElement {
    node_surrogate_to_xml("FilterNodeSurrogate", &n.instance_id, n.start, &n.targets, n.time_span, &n.type_id)
}

fn media_surrogate_to_xml(m: &MediaEntry) -> XElement {
    let mut e = XElement::new("", "MediaSurrogate");
    e.push(XElement::with_text("", "FileName", m.file_name.as_str()));
    e.push(XElement::with_text("", "InstanceId", format_guid(&m.instance_id)));
    e.push(XElement::with_text("", "TypeId", format_guid(&m.type_id)));
    e
}

// ---- Layers -----------------------------------------------------------------------------------------------

fn decode_layers(root: &XElement, f: &mut SequenceFidelity) -> SequenceLayers {
    let Some(sl) = root.child("SequenceLayers").filter(|s| !s.nil) else {
        return SequenceLayers::default();
    };

    let surrogates: Vec<&XElement> =
        root.child("_layerMixingFilterSurrogates").map(|s| s.elements().collect()).unwrap_or_default();
    let models: Vec<&XElement> =
        root.child("_layerMixingFilterDataModels").map(|m| m.elements().collect()).unwrap_or_default();

    let mut layers = Vec::new();
    if let Some(list) = sl.child("Layers") {
        for e in list.elements() {
            let id = child_guid(e, "Id").unwrap_or_default();
            let layer_type = match child_value(e, "Type").as_deref() {
                Some("Default") => LayerType::Default,
                _ => LayerType::Standard,
            };
            let mut layer = Layer {
                id,
                layer_type,
                level: child_i32(e, "LayerLevel").unwrap_or(0),
                name: child_string(e, "LayerName").unwrap_or_default(),
                filter: None,
                filter_instance_id: Uuid::nil(),
            };
            // `_layerMixingFilterSurrogates.FirstOrDefault(x => x.LayerReferenceId == layer.Id)`.
            if let Some(s) = surrogates.iter().find(|s| child_guid(s, "LayerReferenceId") == Some(id)) {
                let type_id = child_guid(s, "TypeId").unwrap_or_default();
                let instance_id = child_guid(s, "InstanceId").unwrap_or_default();
                layer.filter_instance_id = instance_id;
                let model = models
                    .iter()
                    .find(|m| {
                        child_guid(m, "ModuleTypeId") == Some(type_id)
                            && child_guid(m, "ModuleInstanceId") == Some(instance_id)
                    })
                    .copied();
                layer.filter = decode_filter(&type_id, model);
                match &layer.filter {
                    Some(_) => {
                        if let Some(r) = diff(s, &filter_surrogate_xml(&layer)) {
                            f.filter_surrogate_residue.insert(id, r);
                        }
                        if let Some(r) = diff_opt(model, filter_model_to_xml(&layer).as_ref()) {
                            f.filter_models.insert(instance_id, r);
                        }
                    }
                    None => f.unknown_filters.push((id, (*s).clone(), model.cloned())),
                }
            }
            if let Some(r) = diff(e, &layer_to_xml(&layer)) {
                f.layers.insert(id, r);
            }
            layers.push(layer);
        }
    }

    let mut map = HashMap::new();
    if let Some(list) = sl.child("_effectLayerMap") {
        for kv in list.elements() {
            if let (Some(k), Some(v)) = (child_guid(kv, "Key"), child_guid(kv, "Value")) {
                map.insert(k, v);
                f.effect_layer_order.push(k);
            }
        }
    }
    SequenceLayers { layers, effect_layer_map: map }
}

fn decode_filter(type_id: &Uuid, model: Option<&XElement>) -> Option<MixingFilter> {
    use mixing_filter_ids::*;
    Some(match *type_id {
        PROPORTIONAL_MIX => MixingFilter::ProportionalMix,
        HIGHEST_VALUE => MixingFilter::HighestValue,
        MASK => MixingFilter::Mask,
        MULTIPLY_COLOR => MixingFilter::MultiplyColor,
        INTENSITY_OVERLAY => MixingFilter::IntensityOverlay,
        COLOR_CHANGE => MixingFilter::ColorChange,
        LUMA_KEY => {
            // `new LumaKeyData()` when the file has no data, `default(T)` for a member the data lacks.
            let lower = model.map_or(0.0, |m| child_f64(m, "LowerLimit").unwrap_or(0.0));
            let upper = model.map_or(1.0, |m| child_f64(m, "UpperLimit").unwrap_or(0.0));
            MixingFilter::LumaKey { lower_limit: lower, upper_limit: upper }
        }
        MASK_FILL => {
            let exclude = model.is_none_or(|m| child_bool(m, "ExcludeZeroValues").unwrap_or(false));
            let partner = model.is_some_and(|m| child_bool(m, "RequiresMixingPartner").unwrap_or(false));
            MixingFilter::MaskFill { exclude_zero_values: exclude, requires_mixing_partner: partner }
        }
        CHROMA_KEY => MixingFilter::ChromaKey(match model {
            None => new_chroma_key_data(),
            Some(m) => ChromaKeySettings {
                lower_limit: child_f64(m, "LowerLimit").unwrap_or(0.0),
                upper_limit: child_f64(m, "UpperLimit").unwrap_or(0.0),
                key_color: m.child("KeyColor").and_then(color_from_xml).unwrap_or(Color::EMPTY),
                key_saturation: child_f64(m, "KeySaturation").unwrap_or(0.0),
                key_hue: child_f64(m, "KeyHue").unwrap_or(0.0),
                hue_tolerance: child_f64(m, "HueTolerance").unwrap_or(0.0),
                saturation_tolerance: child_f64(m, "SaturationTolerance").unwrap_or(0.0),
                transparent_on_zero_brightness: child_bool(m, "TransparentOnZeroBrightness").unwrap_or(false),
            },
        }),
        _ => return None,
    })
}

/// `new ChromaKeyData()`. Its constructor does not set `TransparentOnZeroBrightness`, unlike
/// `ChromaKeySettings::default`.
fn new_chroma_key_data() -> ChromaKeySettings {
    ChromaKeySettings { transparent_on_zero_brightness: false, ..ChromaKeySettings::default() }
}

fn layer_to_xml(l: &Layer) -> XElement {
    let mut e = XElement::new(ARRAYS_NS, "anyType");
    let (type_name, type_text) = match l.layer_type {
        LayerType::Default => ("DefaultLayer", "Default"),
        LayerType::Standard => ("StandardLayer", "Standard"),
    };
    e.xsi_type = Some((NS_LAYERS.to_string(), type_name.to_string()));
    e.push(XElement::with_text(NS_LAYERS, "Id", format_guid(&l.id)));
    e.push(XElement::with_text(NS_LAYERS, "LayerLevel", l.level.to_string()));
    e.push(XElement::with_text(NS_LAYERS, "LayerName", l.name.as_str()));
    e.push(XElement::with_text(NS_LAYERS, "Type", type_text));
    e
}

fn filter_surrogate_xml(l: &Layer) -> XElement {
    let mut e = XElement::new("", "LayerMixingFilterSurrogate");
    e.push(XElement::with_text("", "InstanceId", format_guid(&l.filter_instance_id)));
    e.push(XElement::with_text("", "LayerReferenceId", format_guid(&l.id)));
    let type_id = l.filter.as_ref().map(mixing_filter_type_id).unwrap_or_default();
    e.push(XElement::with_text("", "TypeId", format_guid(&type_id)));
    e
}

/// The filter's data model, for the filter types that have one.
fn filter_model_to_xml(l: &Layer) -> Option<XElement> {
    let filter = l.filter.as_ref()?;
    let (ns, type_name): (&str, &str) = match filter {
        MixingFilter::LumaKey { .. } => (NS_LUMA, "LumaKeyData"),
        MixingFilter::MaskFill { .. } => (NS_MASK_FILL, "MaskAndFillData"),
        MixingFilter::ChromaKey(_) => (NS_CHROMA, "ChromaKeyData"),
        _ => return None,
    };
    let mut e = XElement::new(ARRAYS_NS, "anyType");
    e.xsi_type = Some((ns.to_string(), type_name.to_string()));
    e.push(XElement::with_text("", "ModuleInstanceId", format_guid(&l.filter_instance_id)));
    e.push(XElement::with_text("", "ModuleTypeId", format_guid(&mixing_filter_type_id(filter))));
    let double = |name: &str, v: f64| XElement::with_text(ns, name, format_double(v));
    match filter {
        MixingFilter::LumaKey { lower_limit, upper_limit } => {
            e.push(double("LowerLimit", *lower_limit));
            e.push(double("UpperLimit", *upper_limit));
        }
        MixingFilter::MaskFill { exclude_zero_values, requires_mixing_partner } => {
            e.push(XElement::with_text(ns, "ExcludeZeroValues", format_bool(*exclude_zero_values)));
            e.push(XElement::with_text(ns, "RequiresMixingPartner", format_bool(*requires_mixing_partner)));
        }
        MixingFilter::ChromaKey(d) => {
            e.push(double("HueTolerance", d.hue_tolerance));
            e.push(color_to_xml(ns, "KeyColor", d.key_color));
            e.push(double("KeyHue", d.key_hue));
            e.push(double("KeySaturation", d.key_saturation));
            e.push(double("LowerLimit", d.lower_limit));
            e.push(double("SaturationTolerance", d.saturation_tolerance));
            e.push(XElement::with_text(
                ns,
                "TransparentOnZeroBrightness",
                format_bool(d.transparent_on_zero_brightness),
            ));
            e.push(double("UpperLimit", d.upper_limit));
        }
        _ => {}
    }
    Some(e)
}

fn sequence_layers_to_xml(seq: &TimedSequence) -> XElement {
    let f = &seq.fidelity;
    let mut e = XElement::new("", "SequenceLayers");
    let mut list = XElement::new(NS_LAYERS, "Layers");
    for l in &seq.layers.layers {
        list.push(reapply(layer_to_xml(l), f.layers.get(&l.id)));
    }
    e.push(list);
    let mut map = XElement::new(NS_LAYERS, "_effectLayerMap");
    let table = &seq.layers.effect_layer_map;
    let order = f.effect_layer_order.iter().filter(|k| table.contains_key(k)).copied();
    let known: HashSet<&Uuid> = f.effect_layer_order.iter().collect();
    let mut new_keys: Vec<Uuid> = table.keys().filter(|k| !known.contains(k)).copied().collect();
    new_keys.sort();
    for k in order.chain(new_keys) {
        let mut kv = XElement::new(ARRAYS_NS, "KeyValueOfguidguid");
        kv.push(XElement::with_text(ARRAYS_NS, "Key", format_guid(&k)));
        kv.push(XElement::with_text(ARRAYS_NS, "Value", format_guid(&table[&k])));
        map.push(kv);
    }
    e.push(map);
    e
}

// ---- Marks ------------------------------------------------------------------------------------------------

fn decode_marks(root: &XElement, f: &mut SequenceFidelity) -> Vec<MarkCollection> {
    let labeled = root.child("LabeledMarkCollections").filter(|e| !e.nil);
    match labeled {
        Some(list) => {
            let mut out = Vec::new();
            for e in list.elements() {
                let mc = mark_collection_from_xml(e);
                if let Some(r) = diff(e, &mark_collection_to_xml(&mc)) {
                    f.marks.insert(mc.id, r);
                }
                out.push(mc);
            }
            out
        }
        None => root.child("MarkCollections").filter(|e| !e.nil).map(convert_old_marks).unwrap_or_default(),
    }
}

/// Reads one `VixenModules.App.Marks.MarkCollection` into vixen-core's type, which cannot be extended here.
pub fn mark_collection_from_xml(e: &XElement) -> MarkCollection {
    let decorator = e.child("Decorator").map(|d| MarkDecorator {
        color: d.child("Color").and_then(color_from_xml).unwrap_or(Color::EMPTY),
        is_bold: child_bool(d, "IsBold").unwrap_or(false),
        is_solid_line: child_bool(d, "IsSolidLine").unwrap_or(false),
        compact_mode: false,
    });
    let marks = e
        .child("Marks")
        .map(|list| {
            list.elements()
                .map(|m| {
                    let mut mark = Mark {
                        // Marks have no id in the file. vixen-core's type wants one, so each load makes new ones.
                        id: Uuid::new_v4(),
                        start_time: child_time(m, "StartTime").unwrap_or_default(),
                        duration: child_time(m, "Duration").unwrap_or_default(),
                        text: child_string(m, "Text").unwrap_or_default(),
                    };
                    mark.apply_minimum_duration();
                    mark
                })
                .collect()
        })
        .unwrap_or_default();
    MarkCollection {
        id: child_guid(e, "Id").unwrap_or_default(),
        name: child_string(e, "Name").unwrap_or_default(),
        level: child_i32(e, "Level").unwrap_or(0),
        show_grid_lines: child_bool(e, "ShowGridLines").unwrap_or(false),
        show_marks: child_bool(e, "ShowMarkBar").unwrap_or(false),
        show_tail_lines: child_bool(e, "ShowTailGridLines").unwrap_or(false),
        is_default: child_bool(e, "IsDefault").unwrap_or(false),
        is_locked: child_bool(e, "Locked").unwrap_or(false),
        collection_type: match child_value(e, "CollectionType").as_deref() {
            Some("Phrase") => MarkCollectionType::Phrase,
            Some("Word") => MarkCollectionType::Word,
            Some("Phoneme") => MarkCollectionType::Phoneme,
            Some("State") => MarkCollectionType::State,
            _ => MarkCollectionType::Generic,
        },
        decorator: decorator.unwrap_or_default(),
        marks,
        linked_mark_collection_id: child_guid(e, "LinkedMarkCollectionId").filter(|id| !id.is_nil()),
    }
}

/// Writes one mark collection as the `anyType` item `LabeledMarkCollections` holds.
pub fn mark_collection_to_xml(mc: &MarkCollection) -> XElement {
    let mut e = XElement::new(ARRAYS_NS, "anyType");
    e.xsi_type = Some((NS_MARKS.to_string(), "MarkCollection".to_string()));
    let ty = match mc.collection_type {
        MarkCollectionType::Generic => "Generic",
        MarkCollectionType::Phrase => "Phrase",
        MarkCollectionType::Word => "Word",
        MarkCollectionType::Phoneme => "Phoneme",
        MarkCollectionType::State => "State",
    };
    e.push(XElement::with_text(NS_MARKS, "CollectionType", ty));
    let mut d = XElement::new(NS_MARKS, "Decorator");
    d.xsi_type = Some((NS_MARKS.to_string(), "MarkDecorator".to_string()));
    d.push(color_to_xml(NS_MARKS, "Color", mc.decorator.color));
    d.push(XElement::with_text(NS_MARKS, "IsBold", format_bool(mc.decorator.is_bold)));
    d.push(XElement::with_text(NS_MARKS, "IsSolidLine", format_bool(mc.decorator.is_solid_line)));
    e.push(d);
    e.push(XElement::with_text(NS_MARKS, "Id", format_guid(&mc.id)));
    e.push(XElement::with_text(NS_MARKS, "IsDefault", format_bool(mc.is_default)));
    e.push(XElement::with_text(NS_MARKS, "Level", mc.level.to_string()));
    e.push(XElement::with_text(
        NS_MARKS,
        "LinkedMarkCollectionId",
        format_guid(&mc.linked_mark_collection_id.unwrap_or_default()),
    ));
    e.push(XElement::with_text(NS_MARKS, "Locked", format_bool(mc.is_locked)));
    let mut marks = XElement::new(NS_MARKS, "Marks");
    for m in &mc.marks {
        let mut item = XElement::new(ARRAYS_NS, "anyType");
        item.xsi_type = Some((NS_MARKS.to_string(), "Mark".to_string()));
        item.push(XElement::with_text(NS_MARKS, "Duration", m.duration.to_xml()));
        item.push(XElement::with_text(NS_MARKS, "StartTime", m.start_time.to_xml()));
        item.push(XElement::with_text(NS_MARKS, "Text", m.text.as_str()));
        marks.push(item);
    }
    e.push(marks);
    e.push(XElement::with_text(NS_MARKS, "Name", mc.name.as_str()));
    e.push(XElement::with_text(NS_MARKS, "ShowGridLines", format_bool(mc.show_grid_lines)));
    e.push(XElement::with_text(NS_MARKS, "ShowMarkBar", format_bool(mc.show_marks)));
    e.push(XElement::with_text(NS_MARKS, "ShowTailGridLines", format_bool(mc.show_tail_lines)));
    e
}

/// `TimedSequenceData.ConvertMarksToLabeledMarks`, for files from before `LabeledMarkCollections` existed.
fn convert_old_marks(list: &XElement) -> Vec<MarkCollection> {
    let mut out: Vec<MarkCollection> = Vec::new();
    for old in list.elements() {
        let mut marks: Vec<Mark> = old
            .child("Marks")
            .map(|times| {
                times
                    .elements()
                    .filter_map(|t| TimeSpan::parse_xml(&t.text()))
                    // `new Mark(x)`: its constructor sets a 450 ms duration.
                    .map(|start_time| Mark {
                        id: Uuid::new_v4(),
                        start_time,
                        duration: TimeSpan::from_ms_i(450),
                        text: String::new(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        // `AddMarks` ends with `EnsureOrder`, which orders by start time then end time.
        marks.sort_by_key(|m| (m.start_time, m.start_time + m.duration));
        out.push(MarkCollection {
            id: Uuid::new_v4(),
            name: child_string(old, "Name").unwrap_or_default(),
            level: child_i32(old, "Level").unwrap_or(0),
            // The `MarkCollection()` constructor's values for what the conversion leaves alone.
            show_grid_lines: child_bool(old, "Enabled").unwrap_or(false),
            show_marks: false,
            show_tail_lines: false,
            is_default: false,
            is_locked: false,
            collection_type: MarkCollectionType::Generic,
            decorator: MarkDecorator {
                color: old.child("MarkColor").and_then(color_from_xml).unwrap_or(Color::EMPTY),
                is_bold: child_bool(old, "Bold").unwrap_or(false),
                is_solid_line: child_bool(old, "SolidLine").unwrap_or(false),
                compact_mode: false,
            },
            marks,
            linked_mark_collection_id: None,
        });
    }
    // "Set one of them active": the first visible one, else the first.
    if let Some(i) =
        out.iter().position(|m| m.show_grid_lines || m.show_marks).or(if out.is_empty() { None } else { Some(0) })
    {
        out[i].is_default = true;
    }
    out
}

// ---- Writing ----------------------------------------------------------------------------------------------

fn sequence_to_xml(seq: &TimedSequence) -> Result<XElement, IoError> {
    Ok(reapply(encode_root(seq)?, seq.fidelity.root.as_ref()))
}

fn encode_root(seq: &TimedSequence) -> Result<XElement, IoError> {
    let f = &seq.fidelity;
    let mut root = XElement::new(NS_TIMED, "TimedSequenceData");
    root.set_attr("version", seq.version.to_string());

    // ModuleDataModelBase.
    root.push(XElement::with_text("", "ModuleInstanceId", format_guid(&seq.module_instance_id)));
    root.push(XElement::with_text("", "ModuleTypeId", format_guid(&seq.module_type_id)));

    // SequenceTypeDataModelBase.
    root.push(XElement::with_text("", "Length", seq.length.to_xml()));
    root.push(sequence_layers_to_xml(seq));
    root.push(XElement::with_text("", "Version", seq.data_version.to_string()));
    root.push(data_models_to_xml(seq)?);

    let mut effects = XElement::new("", "_effectNodeSurrogates");
    for n in &seq.effects {
        effects.push(reapply(effect_surrogate_to_xml(n), f.effect_surrogates.get(&n.instance_id)));
    }
    root.push(effects);

    let mut filters = XElement::new("", "_filterNodeSurrogates");
    for n in &seq.filters {
        filters.push(reapply(filter_surrogate_to_xml(n), f.filter_surrogates.get(&n.instance_id)));
    }
    root.push(filters);

    let mut filter_models = XElement::new("", "_layerMixingFilterDataModels");
    let mut filter_surrogates = XElement::new("", "_layerMixingFilterSurrogates");
    for l in &seq.layers.layers {
        if let Some((_, surrogate, model)) = f.unknown_filters.iter().find(|(id, _, _)| *id == l.id) {
            if l.filter.is_none() {
                filter_surrogates.push(surrogate.clone());
                if let Some(m) = model {
                    filter_models.push(m.clone());
                }
                continue;
            }
        }
        if l.filter.is_some() {
            filter_surrogates.push(reapply(filter_surrogate_xml(l), f.filter_surrogate_residue.get(&l.id)));
            if let Some(m) = reapply_opt(filter_model_to_xml(l), f.filter_models.get(&l.filter_instance_id)) {
                filter_models.push(m);
            }
        }
    }
    root.push(filter_models);
    root.push(filter_surrogates);

    let mut media = XElement::new("", "_mediaSurrogates");
    for m in &seq.media {
        media.push(reapply(media_surrogate_to_xml(m), f.media_surrogates.get(&m.instance_id)));
    }
    root.push(media);

    let mut timing = XElement::new("", "_selectedTimingProviderSurrogate");
    timing.push(opt_string_element("", "ProviderType", seq.selected_timing_provider.provider_type.as_deref()));
    timing.push(opt_string_element("", "SourceName", seq.selected_timing_provider.source_name.as_deref()));
    root.push(timing);

    // TimedSequenceData.
    let time = |local: &str, t: Option<TimeSpan>| match t {
        Some(t) => XElement::with_text(NS_TIMED, local, t.to_xml()),
        None => XElement::nil(NS_TIMED, local),
    };
    root.push(time("DefaultPlaybackEndTime", seq.default_playback_end_time));
    root.push(time("DefaultPlaybackStartTime", seq.default_playback_start_time));
    root.push(XElement::with_text(NS_TIMED, "DefaultRowHeight", seq.default_row_height.to_string()));
    root.push(XElement::with_text(NS_TIMED, "DefaultSplitterDistance", seq.default_splitter_distance.to_string()));

    let mut marks = XElement::new(NS_TIMED, "LabeledMarkCollections");
    for mc in &seq.marks {
        marks.push(reapply(mark_collection_to_xml(mc), f.marks.get(&mc.id)));
    }
    root.push(marks);

    let mut rows = XElement::new(NS_TIMED, "RowSettings");
    for (key, r) in &seq.row_settings {
        let mut kv = XElement::new(ARRAYS_NS, ROW_SETTING_ITEM);
        kv.push(XElement::with_text(ARRAYS_NS, "Key", key.as_str()));
        let mut v = XElement::new(ARRAYS_NS, "Value");
        v.push(XElement::with_text(NS_TIMED, "Expanded", format_bool(r.expanded)));
        v.push(XElement::with_text(NS_TIMED, "RowHeight", r.row_height.to_string()));
        v.push(XElement::with_text(NS_TIMED, "Visible", format_bool(r.visible)));
        kv.push(v);
        rows.push(kv);
    }
    root.push(rows);

    root.push(XElement::with_text(NS_TIMED, "TimePerPixel", seq.time_per_pixel.to_xml()));
    root.push(XElement::with_text(NS_TIMED, "VisibleTimeStart", seq.visible_time_start.to_xml()));
    Ok(root)
}

fn data_models_to_xml(seq: &TimedSequence) -> Result<XElement, IoError> {
    let f = &seq.fidelity;
    let mut list = XElement::new("", "_dataModels");

    let effects: HashMap<(Uuid, Uuid), &EffectNode> =
        seq.effects.iter().map(|n| ((n.type_id, n.instance_id), n)).collect();
    let media: HashMap<(Uuid, Uuid), &MediaEntry> = seq.media.iter().map(|m| ((m.type_id, m.instance_id), m)).collect();
    let filters: HashMap<(Uuid, Uuid), &SequenceFilterNode> =
        seq.filters.iter().map(|n| ((n.type_id, n.instance_id), n)).collect();

    let mut written: HashSet<(Uuid, Uuid)> = HashSet::with_capacity(f.model_order.len());
    for key in &f.model_order {
        if !written.insert(*key) {
            // A second model under one key: C# fails on it, so this copy is only kept for the file's sake.
            if let Some(m) = f.orphan_models.get(key) {
                list.push(m.clone());
            }
            continue;
        }
        if let Some(n) = effects.get(key) {
            if let Some(m) = effect_model(n, f)? {
                list.push(m);
            }
        } else if let Some(m) = media.get(key).and_then(|m| m.data.as_ref()) {
            list.push(m.clone());
        } else if let Some(m) = filters.get(key).and_then(|n| n.data.as_ref()) {
            list.push(m.clone());
        } else if let Some(m) = f.orphan_models.get(key) {
            list.push(m.clone());
        }
    }

    // New items go at the end, as they would in C#'s dictionary.
    for n in &seq.effects {
        if written.insert((n.type_id, n.instance_id)) {
            if let Some(m) = effect_model(n, f)? {
                list.push(m);
            }
        }
    }
    for m in &seq.media {
        if written.insert((m.type_id, m.instance_id)) {
            if let Some(d) = &m.data {
                list.push(d.clone());
            }
        }
    }
    for n in &seq.filters {
        if written.insert((n.type_id, n.instance_id)) {
            if let Some(d) = &n.data {
                list.push(d.clone());
            }
        }
    }
    Ok(list)
}

/// `None` when the file had no data for this effect and the effect has not changed since.
fn effect_model(n: &EffectNode, f: &SequenceFidelity) -> Result<Option<XElement>, IoError> {
    let model = effect_model_to_xml(&n.type_id, &n.instance_id, &n.params);
    if let Some(at_load) = f.absent_models.get(&(n.type_id, n.instance_id)) {
        if model.as_ref().is_ok_and(|m| crate::xml::same_tree(m, at_load)) {
            return Ok(None);
        }
    }
    if n.params.desc.is_empty() && n.params.unknown_members.is_empty() {
        // Nothing is known about this effect's data at all, so there is nothing to write.
        return Ok(None);
    }
    model.map(Some)
}

/// The document typed values alone produce, with all fidelity data dropped: what the codecs themselves make of a
/// sequence. For tests and tools, since it loses known colours and adds members an older file lacks.
pub fn typed_only_document(sequence: &TimedSequence) -> Result<XElement, IoError> {
    let mut stripped = sequence.clone();
    stripped.fidelity = SequenceFidelity::default();
    stripped.fidelity.model_order = sequence.fidelity.model_order.clone();
    stripped.fidelity.effect_layer_order = sequence.fidelity.effect_layer_order.clone();
    stripped.fidelity.unknown_filters = sequence.fidelity.unknown_filters.clone();
    for e in &mut stripped.effects {
        crate::params::strip_hints(&mut e.params);
    }
    encode_root(&stripped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::DotNetColor;
    use crate::xml::same_tree;
    use vixen_core::Curve;
    use vixen_effects::framework::ParamValue;
    use vixen_xml::XNode;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    /// A sequence with one layer of each filter that has settings.
    fn layered() -> TimedSequence {
        let mut s = new_sequence();
        s.layers.add_layer(Layer::standard("Luma", MixingFilter::LumaKey { lower_limit: 0.25, upper_limit: 0.75 }));
        s.layers.add_layer(Layer::standard(
            "Fill",
            MixingFilter::MaskFill { exclude_zero_values: false, requires_mixing_partner: true },
        ));
        s.layers.add_layer(Layer::standard(
            "Chroma",
            MixingFilter::ChromaKey(ChromaKeySettings {
                lower_limit: 0.1,
                upper_limit: 0.9,
                key_color: Color::RED,
                key_saturation: 0.5,
                key_hue: 10.0,
                hue_tolerance: 4.0,
                saturation_tolerance: 0.25,
                transparent_on_zero_brightness: true,
            }),
        ));
        s.layers.add_layer(Layer::standard("Overlay", MixingFilter::IntensityOverlay));
        s
    }

    fn reload(s: &TimedSequence) -> TimedSequence {
        let root = XElement::parse(&sequence_to_document(s).unwrap()).unwrap();
        sequence_from_xml(root, &mut MigrationContext::none()).unwrap()
    }

    fn document(s: &TimedSequence) -> XElement {
        XElement::parse(&sequence_to_document(s).unwrap()).unwrap()
    }

    #[test]
    fn layers_sharing_one_filter_instance_each_get_their_own() {
        // What upstream's Add Layer writes for three layers added in one showing of the panel: one instance id, one data model.
        let mut s = new_sequence();
        for name in ["One", "Two", "Three"] {
            s.layers.add_layer(Layer::standard(name, MixingFilter::LumaKey { lower_limit: 0.25, upper_limit: 0.75 }));
        }
        let mut root = document(&s);
        let shared = s.layers.layers.iter().find(|l| l.name == "One").unwrap().filter_instance_id;
        let text = format_guid(&shared);
        for surrogate in root.child_mut("_layerMixingFilterSurrogates").unwrap().elements_mut() {
            *surrogate.child_mut("InstanceId").unwrap() = XElement::with_text("", "InstanceId", text.clone());
        }
        let models = root.child_mut("_layerMixingFilterDataModels").unwrap();
        let mut kept = false;
        models.children.retain(|n| match n {
            XNode::Element(m) => child_guid(m, "ModuleInstanceId") == Some(shared) && !std::mem::replace(&mut kept, true),
            XNode::Text(_) => true,
        });
        assert_eq!(models.elements().count(), 1);

        let loaded = sequence_from_xml(root, &mut MigrationContext::none()).unwrap();
        let standard: Vec<&Layer> = loaded.layers.layers.iter().filter(|l| l.layer_type == LayerType::Standard).collect();
        assert_eq!(standard.len(), 3);
        let ids: std::collections::HashSet<Uuid> = standard.iter().map(|l| l.filter_instance_id).collect();
        assert_eq!(ids.len(), 3, "each layer has its own instance");
        assert_eq!(standard.iter().filter(|l| l.filter_instance_id == shared).count(), 1, "the first keeps the file's id");
        for l in &standard {
            assert_eq!(l.filter, Some(MixingFilter::LumaKey { lower_limit: 0.25, upper_limit: 0.75 }));
        }

        // The save has a surrogate and a data model for each, none twice, which is what Windows needs to open it.
        let saved = document(&loaded);
        let surrogate_ids: Vec<Uuid> =
            saved.child("_layerMixingFilterSurrogates").unwrap().elements().filter_map(|e| child_guid(e, "InstanceId")).collect();
        let model_ids: Vec<Uuid> =
            saved.child("_layerMixingFilterDataModels").unwrap().elements().filter_map(|e| child_guid(e, "ModuleInstanceId")).collect();
        assert_eq!(surrogate_ids.len(), 3);
        assert_eq!(surrogate_ids.iter().collect::<std::collections::HashSet<_>>().len(), 3);
        assert_eq!(model_ids.len(), 3);
        assert_eq!(surrogate_ids.iter().collect::<std::collections::HashSet<_>>(), model_ids.iter().collect());
        // And it reads back the same.
        let again = reload(&loaded);
        let key = |s: &TimedSequence| -> Vec<(Uuid, Uuid, Option<MixingFilter>)> {
            s.layers.layers.iter().map(|l| (l.id, l.filter_instance_id, l.filter.clone())).collect()
        };
        assert_eq!(key(&again), key(&loaded));
    }

    #[test]
    fn mixing_filter_ids_are_the_csharp_descriptors() {
        use mixing_filter_ids::*;
        // From each `*ModuleDescriptor.cs`. A real file confirms it: the layer named "Intensity Overlay" in
        // Carey Grinch.tim has the type 43f48bf5.
        for (filter, id) in [
            (MixingFilter::ProportionalMix, uuid!("1F16D67A-1FFF-485C-8E76-27FF7E7D072B")),
            (MixingFilter::HighestValue, uuid!("01D6D526-9586-44BB-82F4-10ABACF58B61")),
            (MixingFilter::Mask, uuid!("ACE0F097-7982-4EDC-B5DD-8989DCE60E1D")),
            (MixingFilter::MultiplyColor, uuid!("0C9DFC0B-8E39-4167-8036-79408C6B0B35")),
            (MixingFilter::IntensityOverlay, uuid!("43F48BF5-4088-403F-AD7A-D0FF2114CA31")),
            (MixingFilter::ColorChange, uuid!("93D4169E-5138-403A-B71C-A5A1AB089801")),
            (
                MixingFilter::LumaKey { lower_limit: 0.0, upper_limit: 1.0 },
                uuid!("7583F7D4-DF99-4492-8D10-257C2677FF7B"),
            ),
            (
                MixingFilter::MaskFill { exclude_zero_values: true, requires_mixing_partner: false },
                uuid!("24878B18-14F8-458E-ABAA-3A9C9A34931C"),
            ),
            (MixingFilter::ChromaKey(ChromaKeySettings::default()), uuid!("2D467733-1A64-45AD-BCDC-84BF074747E6")),
        ] {
            assert_eq!(mixing_filter_type_id(&filter), id, "{filter:?}");
            let back = decode_filter(&id, None).unwrap();
            assert_eq!(std::mem::discriminant(&back), std::mem::discriminant(&filter));
        }
        assert_eq!(MASK_FILL, uuid!("24878b18-14f8-458e-abaa-3a9c9a34931c"));
        assert!(decode_filter(&Uuid::nil(), None).is_none());
    }

    #[test]
    fn filter_settings_round_trip() {
        let s = layered();
        let back = reload(&s);
        assert_eq!(back.layers.layers.len(), 5);
        for (a, b) in s.layers.layers.iter().zip(&back.layers.layers) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.name, b.name);
            assert_eq!(a.level, b.level);
            assert_eq!(a.layer_type, b.layer_type);
            assert_eq!(a.filter, b.filter);
            assert_eq!(a.filter_instance_id, b.filter_instance_id);
        }
        assert_eq!(back.fidelity_report(), FidelityReport::default());
    }

    #[test]
    fn filter_data_uses_each_class_contract() {
        let doc = document(&layered());
        let models = doc.child("_layerMixingFilterDataModels").unwrap();
        let types: Vec<_> = models.elements().map(|m| m.xsi_type.as_ref().unwrap().1.as_str()).collect();
        // Stateless filters have no data model. Layers run top to bottom.
        assert_eq!(types, ["ChromaKeyData", "MaskAndFillData", "LumaKeyData"]);
        let chroma = models.elements().next().unwrap();
        let names: Vec<_> = chroma.elements().map(|c| c.local.as_str()).collect();
        assert_eq!(
            names,
            [
                "ModuleInstanceId",
                "ModuleTypeId",
                "HueTolerance",
                "KeyColor",
                "KeyHue",
                "KeySaturation",
                "LowerLimit",
                "SaturationTolerance",
                "TransparentOnZeroBrightness",
                "UpperLimit"
            ]
        );
        assert_eq!(chroma.child("KeyColor").unwrap().child_text("value").as_deref(), Some("4294901760"));
        assert_eq!(doc.child("_layerMixingFilterSurrogates").unwrap().elements().count(), 4);
    }

    #[test]
    fn a_filter_with_no_data_gets_its_new_data_object() {
        let t = |type_id: &Uuid| decode_filter(type_id, None).unwrap();
        assert_eq!(t(&mixing_filter_ids::LUMA_KEY), MixingFilter::LumaKey { lower_limit: 0.0, upper_limit: 1.0 });
        assert_eq!(
            t(&mixing_filter_ids::MASK_FILL),
            MixingFilter::MaskFill { exclude_zero_values: true, requires_mixing_partner: false }
        );
        match t(&mixing_filter_ids::CHROMA_KEY) {
            MixingFilter::ChromaKey(d) => {
                assert_eq!(d.key_color, Color::LIME);
                assert_eq!(d.hue_tolerance, 5.0);
                assert!(!d.transparent_on_zero_brightness, "`new ChromaKeyData()` leaves this false");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_filter_type_this_port_does_not_know_is_kept() {
        let mut doc = document(&layered());
        let future = id(0xF);
        let surrogates = doc.child_mut("_layerMixingFilterSurrogates").unwrap();
        let first = surrogates.elements_mut().next().unwrap();
        *first.child_mut("TypeId").unwrap() = XElement::with_text("", "TypeId", format_guid(&future));
        let back = decode_root(&doc, 9, Vec::new()).unwrap();
        assert!(back.layers.layers[0].filter.is_none());
        assert!(same_tree(&doc, &sequence_to_xml(&back).unwrap()));
    }

    #[test]
    fn marks_round_trip_through_vixen_cores_type() {
        let mut s = new_sequence();
        let mut mc = MarkCollection {
            id: id(1),
            name: "Beats & Bars".to_string(),
            level: 3,
            show_grid_lines: true,
            show_marks: true,
            show_tail_lines: true,
            is_default: true,
            is_locked: true,
            collection_type: MarkCollectionType::Phoneme,
            decorator: MarkDecorator {
                color: Color::rgb(1, 2, 3),
                is_bold: true,
                is_solid_line: true,
                compact_mode: false,
            },
            marks: Vec::new(),
            linked_mark_collection_id: Some(id(2)),
        };
        mc.marks.push(Mark {
            id: id(3),
            start_time: TimeSpan::from_ms_i(1500),
            duration: TimeSpan::from_ms_i(450),
            text: "AI".to_string(),
        });
        mc.marks.push(Mark {
            id: id(4),
            start_time: TimeSpan::from_ms_i(2000),
            duration: TimeSpan::from_ms_i(25),
            text: String::new(),
        });
        s.marks.push(mc.clone());
        let back = reload(&s);
        let b = &back.marks[0];
        assert_eq!(b.id, mc.id);
        assert_eq!(b.name, mc.name);
        assert_eq!(b.level, mc.level);
        assert_eq!(b.show_grid_lines, mc.show_grid_lines);
        assert_eq!(b.show_marks, mc.show_marks);
        assert_eq!(b.show_tail_lines, mc.show_tail_lines);
        assert_eq!(b.is_default, mc.is_default);
        assert_eq!(b.is_locked, mc.is_locked);
        assert_eq!(b.collection_type, mc.collection_type);
        assert_eq!(b.decorator, mc.decorator);
        assert_eq!(b.linked_mark_collection_id, mc.linked_mark_collection_id);
        assert_eq!(b.marks.len(), 2);
        assert_eq!(b.marks[0].start_time, mc.marks[0].start_time);
        assert_eq!(b.marks[0].duration, mc.marks[0].duration);
        assert_eq!(b.marks[0].text, "AI");
        assert_eq!(b.marks[1].text, "");
    }

    #[test]
    fn a_known_decorator_colour_stays_when_a_mark_moves() {
        let mut s = new_sequence();
        let mut mc = MarkCollection { id: id(1), ..MarkCollection::default() };
        mc.marks.push(Mark::new(TimeSpan::from_ms_i(100)));
        s.marks.push(mc);
        let mut doc = document(&s);
        let list = doc.child_mut("LabeledMarkCollections").unwrap();
        let color = list.elements_mut().next().unwrap().child_mut("Decorator").unwrap().child_mut("Color").unwrap();
        *color = DotNetColor::from_known(104).to_xml(NS_MARKS, "Color");

        let mut back = decode_root(&doc, 9, Vec::new()).unwrap();
        assert_eq!(back.marks[0].decorator.color, Color::LIME);
        assert!(same_tree(&doc, &sequence_to_xml(&back).unwrap()));

        back.marks[0].marks[0].start_time = TimeSpan::from_ms_i(200);
        let out = sequence_to_xml(&back).unwrap();
        let c = out.child("LabeledMarkCollections").unwrap().elements().next().unwrap();
        assert_eq!(c.child("Decorator").unwrap().child("Color").unwrap().child_text("state").as_deref(), Some("1"));
        assert_eq!(
            c.child("Marks").unwrap().elements().next().unwrap().child_text("StartTime").as_deref(),
            Some("PT0.2S")
        );

        back.marks[0].decorator.color = Color::rgb(9, 9, 9);
        let out = sequence_to_xml(&back).unwrap();
        let c = out.child("LabeledMarkCollections").unwrap().elements().next().unwrap();
        assert_eq!(c.child("Decorator").unwrap().child("Color").unwrap().child_text("state").as_deref(), Some("2"));
    }

    #[test]
    fn members_a_mark_collection_lacks_stay_out_of_the_file() {
        let mut s = new_sequence();
        s.marks.push(MarkCollection {
            id: id(1),
            show_tail_lines: false,
            is_locked: false,
            ..MarkCollection::default()
        });
        let mut doc = document(&s);
        let c = doc.child_mut("LabeledMarkCollections").unwrap().elements_mut().next().unwrap();
        c.children.retain(|n| !matches!(n, XNode::Element(e) if e.local == "Locked" || e.local == "ShowTailGridLines"));
        let mut back = decode_root(&doc, 9, Vec::new()).unwrap();
        assert!(same_tree(&doc, &sequence_to_xml(&back).unwrap()));
        // Locking the collection is a change, so that member appears, in its sorted place.
        back.marks[0].is_locked = true;
        let out = sequence_to_xml(&back).unwrap();
        let c = out.child("LabeledMarkCollections").unwrap().elements().next().unwrap();
        assert_eq!(c.child_text("Locked").as_deref(), Some("true"));
        assert!(c.child("ShowTailGridLines").is_none());
        let names: Vec<_> = c.elements().map(|e| e.local.as_str()).collect();
        assert_eq!(
            names.iter().position(|n| *n == "Locked"),
            Some(names.iter().position(|n| *n == "LinkedMarkCollectionId").unwrap() + 1)
        );
    }

    #[test]
    fn old_mark_collections_convert_like_the_csharp() {
        let timed = |s: &str| format!("<a:TimeSpan xmlns:a=\"{ARRAYS_NS}\">{s}</a:TimeSpan>");
        let color = crate::xml::write_fragment(&DotNetColor::from_known(164).to_xml(NS_TIMED, "MarkColor"));
        let text = format!(
            r#"<TimedSequenceData version="9" xmlns="{NS_TIMED}"><MarkCollections><MarkCollection><Bold>true</Bold><Enabled>false</Enabled><Id>00000000-0000-0000-0000-000000000001</Id><Level>2</Level>{color}<Marks>{}{}</Marks><Name>Old</Name><SolidLine>true</SolidLine></MarkCollection></MarkCollections></TimedSequenceData>"#,
            timed("PT2S"),
            timed("PT1S"),
        );
        let doc = XElement::parse(&text).unwrap();
        let s = decode_root(&doc, 9, Vec::new()).unwrap();
        assert_eq!(s.marks.len(), 1);
        let m = &s.marks[0];
        assert_eq!(m.name, "Old");
        assert_eq!(m.level, 2);
        assert!(!m.show_grid_lines);
        assert_eq!(m.decorator.color, Color::WHITE);
        assert!(m.decorator.is_bold);
        assert!(m.decorator.is_solid_line);
        assert_eq!(m.marks.iter().map(|x| x.start_time.total_ms()).collect::<Vec<_>>(), [1000.0, 2000.0]);
        assert_eq!(m.marks[0].duration, TimeSpan::from_ms_i(450));
        // With none visible, the first becomes the default.
        assert!(m.is_default);
    }

    #[test]
    fn row_settings_keep_their_keys_and_order() {
        let mut s = new_sequence();
        s.row_settings.push(("ZZZ".to_string(), RowSetting { expanded: true, row_height: 40, visible: false }));
        s.row_settings.push(("AAA".to_string(), RowSetting { expanded: false, row_height: 7, visible: true }));
        let back = reload(&s);
        assert_eq!(back.row_settings, s.row_settings);
        let doc = document(&s);
        let item = doc.child("RowSettings").unwrap().elements().next().unwrap();
        assert_eq!(item.local, ROW_SETTING_ITEM);
        assert_eq!(item.ns, ARRAYS_NS);
        assert_eq!(item.child("Value").unwrap().child("RowHeight").unwrap().ns, NS_TIMED);
    }

    #[test]
    fn timing_values_including_unset_ones_round_trip() {
        let mut s = new_sequence();
        s.selected_timing_provider = SelectedTimingProvider {
            provider_type: Some("Media".to_string()),
            source_name: Some("a & b.mp3".to_string()),
        };
        s.default_playback_start_time = None;
        s.default_playback_end_time = Some(TimeSpan::from_ms_i(1250));
        s.time_per_pixel = TimeSpan::from_ms_i(10);
        let back = reload(&s);
        assert_eq!(back.selected_timing_provider, s.selected_timing_provider);
        assert_eq!(back.default_playback_start_time, None);
        assert_eq!(back.default_playback_end_time, s.default_playback_end_time);
        assert_eq!(back.time_per_pixel, s.time_per_pixel);
        assert_eq!(new_sequence().time_per_pixel, TimeSpan::MIN);
        let nil = document(&new_sequence());
        assert!(nil.child("_selectedTimingProviderSurrogate").unwrap().child("ProviderType").unwrap().nil);
    }

    #[test]
    fn the_effect_layer_map_keeps_file_order_and_appends_new_keys() {
        let mut s = layered();
        let layer = s.layers.layers[0].id;
        for k in [5, 1, 9] {
            s.layers.effect_layer_map.insert(id(k), layer);
        }
        s.fidelity.effect_layer_order = vec![id(5), id(1), id(9)];
        let mut back = reload(&s);
        let order = |s: &TimedSequence| -> Vec<String> {
            document(s)
                .child("SequenceLayers")
                .unwrap()
                .child("_effectLayerMap")
                .unwrap()
                .elements()
                .map(|kv| kv.child_text("Key").unwrap())
                .collect()
        };
        let expected: Vec<String> = [5u128, 1, 9].map(|k| format_guid(&id(k))).to_vec();
        assert_eq!(order(&back), expected);
        back.layers.effect_layer_map.insert(id(3), layer);
        back.layers.effect_layer_map.remove(&id(1));
        assert_eq!(order(&back), [5u128, 9, 3].map(|k| format_guid(&id(k))).to_vec());
    }

    fn pulse_effect(instance: u128) -> EffectNode {
        let d = vixen_effect_registry::by_data_type("PulseData").unwrap();
        EffectNode {
            start: TimeSpan::from_ms_i(10),
            time_span: TimeSpan::from_ms_i(20),
            type_id: d.type_id,
            instance_id: id(instance),
            targets: vec![(id(100), "Star".to_string())],
            params: d.new_params(),
        }
    }

    #[test]
    fn effects_can_be_added_and_removed_after_loading() {
        let mut s = new_sequence();
        s.effects.push(pulse_effect(1));
        s.effects.push(pulse_effect(2));
        let mut back = reload(&s);
        back.effects.remove(0);
        back.effects.push(pulse_effect(3));
        let doc = document(&back);
        let models: Vec<_> =
            doc.child("_dataModels").unwrap().elements().map(|m| m.child_text("ModuleInstanceId").unwrap()).collect();
        assert_eq!(models, [format_guid(&id(2)), format_guid(&id(3))]);
        let surrogates: Vec<_> = doc
            .child("_effectNodeSurrogates")
            .unwrap()
            .elements()
            .map(|m| m.child_text("InstanceId").unwrap())
            .collect();
        assert_eq!(surrogates, models);
    }

    #[test]
    fn a_surrogate_with_no_model_gets_defaults_and_writes_nothing_until_it_changes() {
        let mut s = new_sequence();
        s.effects.push(pulse_effect(1));
        let mut doc = document(&s);
        doc.child_mut("_dataModels").unwrap().children.clear();
        let mut back = decode_root(&doc, 9, Vec::new()).unwrap();
        assert_eq!(back.effects[0].params.get("LevelCurve"), s.effects[0].params.get("LevelCurve"));
        assert_eq!(back.fidelity_report().absent_models, 1);
        assert!(same_tree(&doc, &sequence_to_xml(&back).unwrap()));

        back.effects[0].params.set("LevelCurve", ParamValue::Curve(Curve::flat(5.0)));
        let out = sequence_to_xml(&back).unwrap();
        assert_eq!(out.child("_dataModels").unwrap().elements().count(), 1);
    }

    #[test]
    fn a_data_type_known_by_name_decodes_even_under_a_new_type_id() {
        let mut s = new_sequence();
        s.effects.push(pulse_effect(1));
        let mut doc = document(&s);
        let renamed = format_guid(&id(0xABC));
        let model = doc.child_mut("_dataModels").unwrap().elements_mut().next().unwrap();
        *model.child_mut("ModuleTypeId").unwrap() = XElement::with_text("", "ModuleTypeId", renamed.as_str());
        let surrogate = doc.child_mut("_effectNodeSurrogates").unwrap().elements_mut().next().unwrap();
        *surrogate.child_mut("TypeId").unwrap() = XElement::with_text("", "TypeId", renamed.as_str());
        let back = decode_root(&doc, 9, Vec::new()).unwrap();
        assert!(!back.effects[0].params.desc.is_empty());
        assert!(same_tree(&doc, &sequence_to_xml(&back).unwrap()));
    }

    #[test]
    fn unknown_root_members_and_attributes_survive() {
        let s = new_sequence();
        let mut doc = document(&s);
        doc.set_attr("future", "yes");
        let at = doc.children.iter().position(|n| matches!(n, XNode::Element(e) if e.local == "RowSettings")).unwrap();
        doc.children.insert(at, XNode::Element(XElement::with_text(NS_TIMED, "NewThing", "1")));
        let back = decode_root(&doc, 9, Vec::new()).unwrap();
        assert!(same_tree(&doc, &sequence_to_xml(&back).unwrap()));
    }

    #[test]
    fn sequence_files_must_be_sequences() {
        let e = sequence_from_xml(XElement::parse("<Other/>").unwrap(), &mut MigrationContext::none()).unwrap_err();
        assert!(e.to_string().contains("TimedSequenceData"), "{e}");
    }

    #[test]
    fn a_new_sequence_matches_the_csharp_constructor() {
        let s = new_sequence();
        assert_eq!(s.version, 9);
        assert_eq!(s.module_type_id, TIMED_SEQUENCE_TYPE_ID);
        assert_eq!(s.visible_time_start, TimeSpan::MIN);
        assert_eq!(s.default_playback_end_time, Some(TimeSpan::ZERO));
        assert_eq!(s.layers.layers[0].layer_type, LayerType::Default);
        let doc = document(&s);
        assert_eq!(doc.attr("version"), Some("9"));
        let names: Vec<_> = doc.elements().map(|e| e.local.as_str()).collect();
        assert_eq!(
            names,
            [
                "ModuleInstanceId",
                "ModuleTypeId",
                "Length",
                "SequenceLayers",
                "Version",
                "_dataModels",
                "_effectNodeSurrogates",
                "_filterNodeSurrogates",
                "_layerMixingFilterDataModels",
                "_layerMixingFilterSurrogates",
                "_mediaSurrogates",
                "_selectedTimingProviderSurrogate",
                "DefaultPlaybackEndTime",
                "DefaultPlaybackStartTime",
                "DefaultRowHeight",
                "DefaultSplitterDistance",
                "LabeledMarkCollections",
                "RowSettings",
                "TimePerPixel",
                "VisibleTimeStart",
            ]
        );
    }

    #[test]
    fn saving_to_a_path_uses_the_backup_rules() {
        let dir = crate::test_dir("sequence");
        let path = dir.join("Song.tim");
        std::fs::write(&path, document_text_v8()).unwrap();
        let s = load_sequence(&path).unwrap();
        assert_eq!(s.source_version, 8);
        assert_eq!(s.version, 9);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "a load writes nothing");
        save_sequence(&s, &path).unwrap();
        assert!(dir.join("Song.tim.8").is_file());
        assert_eq!(std::fs::read_dir(dir.join("auto_backup")).unwrap().count(), 1);
        assert_eq!(crate::file::root_version(&path), Some(9));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn document_text_v8() -> String {
        sequence_to_document(&new_sequence()).unwrap().replace("version=\"9\"", "version=\"8\"")
    }
}
