//! Saved effect defaults: `Vixen.Services.EffectDefaults`. The store is `Effect Defaults/EffectDefaults.vfd`, binary XML of an `EffectDefaultsStore` whose entries each hold a binary XML data model as a `byte[]`.
//! A corrupt or absent file is an empty store, with no error. An effect not in this build keeps its entry, so saving doesn't lose it.

use crate::error::IoError;
use crate::paths::DataPaths;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_effects::framework::{EffectDescriptor, ParamValue, Params};
use vixen_xml::nbfx;
use vixen_xml::{contract_ns, XElement};

const NS_DEFAULTS: &str = "Vixen.Services.EffectDefaults";
const STORE_ROOT: &str = "EffectDefaultsStore";
const ENTRY: &str = "EffectDefaultEntry";
const FILE_NAME: &str = "EffectDefaults.vfd";

/// `EffectDefaultEntry`.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectDefaultEntry {
    pub type_id: Uuid,
    pub effect_name: String,
    pub data_model_type_name: String,
    /// `SavedUtc`, as `xs:dateTime` text.
    pub saved_utc: String,
    /// The scrubbed data model, as binary XML.
    pub payload: Vec<u8>,
}

/// What `SaveDefault` does to the copy: every `[ExcludeFromEffectDefault]` member goes back to the type's default, at any depth. Library links stay (`EffectDefaultScrubber`).
pub fn scrub(params: &mut Params) {
    let desc = params.desc;
    for (d, value) in desc.iter().zip(params.values.iter_mut()) {
        if d.exclude_from_default {
            *value = (d.default)();
            continue;
        }
        match value {
            ParamValue::Record(r) => scrub(r),
            ParamValue::RecordList(list) => {
                for r in list {
                    scrub(r);
                }
            }
            _ => {}
        }
    }
}

/// `EffectDefaultsService`'s store for one profile.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectDefaultsStore {
    path: PathBuf,
    entries: Vec<EffectDefaultEntry>,
}

impl EffectDefaultsStore {
    /// `LoadLocked`: corrupt or absent is an empty store.
    pub fn load(paths: &DataPaths) -> EffectDefaultsStore {
        let path = paths.effect_defaults_dir().join(FILE_NAME);
        let entries = std::fs::read(&path).ok().and_then(|bytes| entries_from_bytes(&bytes)).unwrap_or_default();
        EffectDefaultsStore { path, entries }
    }

    pub fn entries(&self) -> &[EffectDefaultEntry] {
        &self.entries
    }

    pub fn has(&self, type_id: &Uuid) -> bool {
        self.entries.iter().any(|e| e.type_id == *type_id)
    }

    /// `CreateDefaultData`: the saved values for a new effect, or `None` for the built-in ones. An entry saved for another data model is ignored, and one that can't be read falls back with no error.
    pub fn create_params(&self, descriptor: &EffectDescriptor) -> Option<Params> {
        let entry = self.entries.iter().find(|e| e.type_id == descriptor.type_id)?;
        let wanted = format!("{}.{}", descriptor.data_ns, descriptor.data_type);
        if !entry.data_model_type_name.is_empty() && entry.data_model_type_name != wanted {
            return None;
        }
        let model = nbfx::read(&entry.payload).ok()?;
        Some(crate::params::effect_params_from_xml(&model, descriptor))
    }

    /// `SaveDefault`: captures a scrubbed copy of the values, replacing any earlier one, and writes the file. The values passed aren't changed.
    pub fn save(&mut self, descriptor: &EffectDescriptor, params: &Params) -> Result<(), IoError> {
        let mut copy = params.clone();
        scrub(&mut copy);
        let entry = EffectDefaultEntry {
            type_id: descriptor.type_id,
            effect_name: descriptor.name.to_string(),
            data_model_type_name: format!("{}.{}", descriptor.data_ns, descriptor.data_type),
            saved_utc: utc_now_text(),
            payload: nbfx::write(&model_element(descriptor, &copy)),
        };
        match self.entries.iter_mut().find(|e| e.type_id == entry.type_id) {
            Some(existing) => *existing = entry,
            None => self.entries.push(entry),
        }
        self.persist()
    }

    /// `ClearDefault`: true if one existed. Nothing changes in any effect.
    pub fn clear(&mut self, type_id: &Uuid) -> Result<bool, IoError> {
        let before = self.entries.len();
        self.entries.retain(|e| e.type_id != *type_id);
        if self.entries.len() == before {
            return Ok(false);
        }
        self.persist()?;
        Ok(true)
    }

    /// `Export`: the same binary format as the store. An id with no saved default is silently skipped, and the payloads go as captured.
    pub fn export(&self, path: &Path, type_ids: &[Uuid]) -> Result<(), IoError> {
        let chosen: Vec<EffectDefaultEntry> = type_ids.iter().filter_map(|id| self.entries.iter().find(|e| e.type_id == *id).cloned()).collect();
        std::fs::write(path, entries_to_bytes(&chosen)).map_err(|e| IoError::io(path, e))
    }

    /// `Import` with `ImportMode.Overwrite`: every entry upserted by type, the others left alone. Returns `(new, updated)`.
    pub fn import(&mut self, path: &Path) -> Result<(usize, usize), IoError> {
        let bytes = std::fs::read(path).map_err(|e| IoError::io(path, e))?;
        let imported = entries_from_bytes(&bytes)
            .ok_or_else(|| IoError::Format { context: path.display().to_string(), message: "not an effect defaults file".to_string() })?;
        let (mut new, mut updated) = (0, 0);
        for entry in imported {
            match self.entries.iter_mut().find(|e| e.type_id == entry.type_id) {
                Some(existing) => {
                    *existing = entry;
                    updated += 1;
                }
                None => {
                    self.entries.push(entry);
                    new += 1;
                }
            }
        }
        self.persist()?;
        Ok((new, updated))
    }

    /// `WriteDiagnosticDump`: indented XML, with each saved model's settings inline rather than as an opaque payload.
    pub fn write_diagnostic_dump(&self, path: &Path) -> Result<(), IoError> {
        let mut root = XElement::new("", "EffectDefaults");
        for entry in &self.entries {
            let mut item = XElement::new("", "EffectDefault");
            item.push(XElement::with_text("", "TypeId", entry.type_id.to_string()));
            item.push(XElement::with_text("", "EffectName", entry.effect_name.as_str()));
            item.push(XElement::with_text("", "DataModelTypeName", entry.data_model_type_name.as_str()));
            item.push(XElement::with_text("", "SavedUtc", entry.saved_utc.as_str()));
            let mut data = XElement::new("", "ModuleData");
            match nbfx::read(&entry.payload) {
                Ok(model) => {
                    for child in model.elements() {
                        data.push(child.clone());
                    }
                }
                Err(e) => {
                    data.push(XElement::with_text("", "Unreadable", format!("Unable to read the saved settings: {e}")));
                }
            }
            item.push(data);
            root.push(item);
        }
        std::fs::write(path, root.to_document()).map_err(|e| IoError::io(path, e))
    }

    /// `PersistLocked`: written beside the file, then moved over it.
    fn persist(&self) -> Result<(), IoError> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| IoError::io(dir, e))?;
        }
        let temp = self.path.with_extension("vfd.tmp");
        std::fs::write(&temp, entries_to_bytes(&self.entries)).map_err(|e| IoError::io(&temp, e))?;
        std::fs::rename(&temp, &self.path).map_err(|e| IoError::io(&self.path, e))
    }
}

/// The data model as `DataContractSerializer` roots it: named for its class, in the class's contract namespace.
fn model_element(descriptor: &EffectDescriptor, params: &Params) -> XElement {
    let mut root = XElement::new(&contract_ns(descriptor.data_ns), descriptor.data_type);
    root.push(XElement::with_text("", "ModuleInstanceId", crate::dcs::format_guid(&Uuid::nil())));
    root.push(XElement::with_text("", "ModuleTypeId", crate::dcs::format_guid(&descriptor.type_id)));
    for member in crate::params::params_to_members(params) {
        root.push(member);
    }
    root
}

fn entries_from_bytes(bytes: &[u8]) -> Option<Vec<EffectDefaultEntry>> {
    let root = nbfx::read(bytes).ok()?;
    let list = root.child("Entries")?;
    let mut out = Vec::new();
    for e in list.elements() {
        out.push(EffectDefaultEntry {
            type_id: Uuid::parse_str(e.child_text("TypeId")?.trim()).ok()?,
            effect_name: e.child_text("EffectName").unwrap_or_default(),
            data_model_type_name: e.child_text("DataModelTypeName").unwrap_or_default(),
            saved_utc: e.child_text("SavedUtc").unwrap_or_default(),
            payload: nbfx::base64_decode(&e.child_text("Payload")?)?,
        });
    }
    Some(out)
}

fn entries_to_bytes(entries: &[EffectDefaultEntry]) -> Vec<u8> {
    let ns = contract_ns(NS_DEFAULTS);
    let mut root = XElement::new(&ns, STORE_ROOT);
    let mut list = XElement::new(&ns, "Entries");
    for e in entries {
        let mut item = XElement::new(&ns, ENTRY);
        item.push(XElement::with_text(&ns, "DataModelTypeName", e.data_model_type_name.as_str()));
        item.push(XElement::with_text(&ns, "EffectName", e.effect_name.as_str()));
        item.push(XElement::with_text(&ns, "Payload", nbfx::base64_encode(&e.payload)));
        item.push(XElement::with_text(&ns, "SavedUtc", e.saved_utc.as_str()));
        item.push(XElement::with_text(&ns, "TypeId", crate::dcs::format_guid(&e.type_id)));
        list.push(item);
    }
    root.push(list);
    root.push(XElement::with_text(&ns, "Version", "1"));
    nbfx::write(&root)
}

/// `DateTime.UtcNow` as `xs:dateTime`.
fn utc_now_text() -> String {
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let days = seconds / 86_400;
    let rest = seconds % 86_400;
    // Days since 1970-01-01, the civil algorithm's epoch shifted to 0000-03-01.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rest / 3600, (rest % 3600) / 60, rest % 60)
}

/// For a test or a caller that already knows where the file is.
pub fn store_file_in(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::Curve;

    fn descriptor(name: &str) -> &'static EffectDescriptor {
        vixen_effect_registry::all().iter().copied().find(|d| d.name == name).unwrap()
    }

    fn temp_paths(tag: &str) -> DataPaths {
        let root = std::env::temp_dir().join(format!("vixen-defaults-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        DataPaths::new(root)
    }

    #[test]
    fn saved_values_round_trip_and_the_live_values_are_untouched() {
        let d = descriptor("Pulse");
        let mut live = d.new_params();
        live.set("LevelCurve", ParamValue::Curve(Curve::flat(42.0)));
        let snapshot = live.clone();
        let paths = temp_paths("round");
        let mut store = EffectDefaultsStore::load(&paths);
        assert!(!store.has(&d.type_id));
        store.save(d, &live).unwrap();
        assert_eq!(live, snapshot);
        let reread = EffectDefaultsStore::load(&paths);
        assert!(reread.has(&d.type_id));
        assert_eq!(reread.entries()[0].effect_name, "Pulse");
        let params = reread.create_params(d).unwrap();
        assert_eq!(params.curve("LevelCurve"), Curve::flat(42.0));
        // Saving again replaces it, and clearing removes it.
        let mut store = reread;
        live.set("LevelCurve", ParamValue::Curve(Curve::flat(7.0)));
        store.save(d, &live).unwrap();
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.create_params(d).unwrap().curve("LevelCurve"), Curve::flat(7.0));
        assert!(store.clear(&d.type_id).unwrap());
        assert!(!store.clear(&d.type_id).unwrap());
        assert!(EffectDefaultsStore::load(&paths).create_params(d).is_none());
    }

    #[test]
    fn a_corrupt_file_is_an_empty_store() {
        let paths = temp_paths("corrupt");
        std::fs::create_dir_all(paths.effect_defaults_dir()).unwrap();
        std::fs::write(paths.effect_defaults_dir().join(FILE_NAME), b"not binary xml").unwrap();
        assert!(EffectDefaultsStore::load(&paths).entries().is_empty());
    }

    #[test]
    fn scrub_resets_excluded_members_at_any_depth_and_keeps_links() {
        let d = descriptor("Alternating");
        let mut p = d.new_params();
        let id = uuid::Uuid::new_v4();
        p.set("MarkCollectionId", ParamValue::Guid(id));
        scrub(&mut p);
        assert_eq!(p.guid("MarkCollectionId"), Uuid::nil());

        // Inside rows too.
        let wave = descriptor("Wave");
        let mut p = wave.new_params();
        if let Some(ParamValue::RecordList(rows)) = p.index_of("WaveformData").map(|i| p.values[i].clone()) {
            let mut rows = rows;
            rows[0].set("MarkCollectionId", ParamValue::Guid(id));
            p.set("WaveformData", ParamValue::RecordList(rows));
        }
        scrub(&mut p);
        assert_eq!(p.records("WaveformData")[0].guid("MarkCollectionId"), Uuid::nil());

        let mut linked = Curve::ramp_up();
        linked.library_reference_name = "Lib".into();
        let pulse = descriptor("Pulse");
        let mut p = pulse.new_params();
        p.set("LevelCurve", ParamValue::Curve(linked.clone()));
        scrub(&mut p);
        assert_eq!(p.curve("LevelCurve"), linked);
    }

    #[test]
    fn an_entry_for_another_data_model_is_ignored() {
        let d = descriptor("Pulse");
        let paths = temp_paths("model");
        let mut store = EffectDefaultsStore::load(&paths);
        store.save(d, &d.new_params()).unwrap();
        store.entries[0].data_model_type_name = "Something.Else".to_string();
        assert!(store.create_params(d).is_none());
    }

    #[test]
    fn an_entry_for_an_unknown_effect_survives_a_save() {
        let d = descriptor("Pulse");
        let paths = temp_paths("unknown");
        let mut store = EffectDefaultsStore::load(&paths);
        store.entries.push(EffectDefaultEntry {
            type_id: Uuid::new_v4(),
            effect_name: "Gone".into(),
            data_model_type_name: "X.Y".into(),
            saved_utc: "2026-01-02T03:04:05Z".into(),
            payload: vec![1, 2, 3],
        });
        store.save(d, &d.new_params()).unwrap();
        let reread = EffectDefaultsStore::load(&paths);
        assert_eq!(reread.entries().len(), 2);
        assert_eq!(reread.entries()[0].payload, vec![1, 2, 3]);
    }
}
