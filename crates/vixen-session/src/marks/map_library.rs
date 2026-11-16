//! The LipSync image map library and its editors without their windows: `LipSyncMapLibrary`, `LipSyncMapData`, `LipSyncMapSelector`,
//! `LipSyncMapMatrixEditor` and `LipSyncMultiPicSelect`. Spec: `docs/spec/marks-layers-analysis.md`, sections L and M, F89 to F105, F118, F119.
//!
//! The library lives in the module store; a map's pictures live in `<module data>/LipSync/<map name>/<PHONEME>.bmp`. Every `root` below is
//! the module data folder (`DataPaths::module_data_dir`).

use super::lipsync::{PhonemeType, MAP_PHONEMES};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, OnceLock};
use uuid::Uuid;
use vixen_io::{ModuleEntry, ModuleStore};
use vixen_xml::{XElement, XNode};

pub const LIP_SYNC_MAP_TYPE_ID: Uuid = Uuid::from_u128(0xa475c535_73fa_4d31_a1bc_ed2b50ad21cf);
const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.LipSyncApp";
const ARRAYS_NS: &str = "http://schemas.microsoft.com/2003/10/Serialization/Arrays";
const ITEM: &str = "KeyValueOfstringLipSyncMapDataiatCCBjB";
const DATA_MODEL_TYPE: &str = "VixenModules.App.LipSyncApp.LipSyncMapStaticData, LipSyncApp";
const ASSEMBLY: &str = "Module.App.LipSyncApp";
const ZERO_GUID: &str = "00000000-0000-0000-0000-000000000000";
const LOGGER: &str = "LipSyncMapLibrary";

pub const NEW_MAP_NAME: &str = "New Map";
pub const EXISTS_MESSAGE: &str = "Can not rename, this new name already exists!";
pub const BAD_NAME_MESSAGE: &str = "Can not set new name!";
pub const ERROR_TITLE: &str = "Error!";

/// The bundled `*_Transparent.png` pictures a map on its defaults is given, F98.
const DEFAULT_PICTURES: [(PhonemeType, &[u8]); 10] = [
    (PhonemeType::Ai, include_bytes!("../../assets/lipsync/AI.png")),
    (PhonemeType::E, include_bytes!("../../assets/lipsync/E.png")),
    (PhonemeType::Etc, include_bytes!("../../assets/lipsync/ETC.png")),
    (PhonemeType::Fv, include_bytes!("../../assets/lipsync/FV.png")),
    (PhonemeType::L, include_bytes!("../../assets/lipsync/L.png")),
    (PhonemeType::Mbp, include_bytes!("../../assets/lipsync/MBP.png")),
    (PhonemeType::O, include_bytes!("../../assets/lipsync/O.png")),
    (PhonemeType::Rest, include_bytes!("../../assets/lipsync/REST.png")),
    (PhonemeType::U, include_bytes!("../../assets/lipsync/U.png")),
    (PhonemeType::Wq, include_bytes!("../../assets/lipsync/WQ.png")),
];

/// Every `PhonemeType`, `Unknown` included: the files an editor loads, deletes and saves.
const ALL_PHONEMES: [PhonemeType; 11] = [
    PhonemeType::Ai,
    PhonemeType::E,
    PhonemeType::O,
    PhonemeType::U,
    PhonemeType::Fv,
    PhonemeType::L,
    PhonemeType::Mbp,
    PhonemeType::Wq,
    PhonemeType::Etc,
    PhonemeType::Rest,
    PhonemeType::Unknown,
];

fn picture_file(phoneme: PhonemeType) -> String {
    format!("{}.bmp", phoneme.name())
}

/// `uniqueKeyIndex`: one rising counter for the whole run, as upstream's library object lives that long.
fn process_counter() -> Arc<AtomicI32> {
    static COUNTER: OnceLock<Arc<AtomicI32>> = OnceLock::new();
    Arc::clone(COUNTER.get_or_init(|| Arc::new(AtomicI32::new(0))))
}

/// GUARD F105, F118: a name that would leave the LipSync folder, or name it or its parent.
pub fn name_is_unsafe(name: &str) -> bool {
    name.contains('/') || name.contains('\\') || name == "." || name == ".."
}

fn lipsync_dir(root: &Path) -> PathBuf {
    root.join("LipSync")
}

/// A map's picture folder, or none for a name that can't safely be one.
fn map_dir(root: &Path, folder: &str) -> Option<PathBuf> {
    if folder.trim().is_empty() || name_is_unsafe(folder) {
        return None;
    }
    Some(lipsync_dir(root).join(folder))
}

/// `LipSyncMapData`.
#[derive(Clone, Debug)]
pub struct MapData {
    /// The library key.
    pub name: String,
    /// `LibraryReferenceName`, which names the picture folder. Upstream keeps it equal to the key.
    folder: String,
    pub notes: String,
    pub is_default: bool,
    pub is_matrix: bool,
    pub using_defaults: bool,
    pub is_current: bool,
    strings_are_rows: bool,
    /// The deprecated `MapItems`, which only a map nobody has edited or copied still carries.
    map_items: Option<XElement>,
    /// The dictionary entry as the file had it, written back untouched while nothing above differs from it.
    raw: Option<XElement>,
}

impl Default for MapData {
    fn default() -> MapData {
        MapData {
            name: String::new(),
            folder: String::new(),
            notes: String::new(),
            is_default: false,
            is_matrix: true,
            using_defaults: true,
            is_current: false,
            strings_are_rows: false,
            map_items: None,
            raw: None,
        }
    }
}

impl MapData {
    /// The copy constructor: everything but `MapItems` and `StringsAreRows`.
    fn copy(&self) -> MapData {
        MapData { strings_are_rows: false, map_items: None, raw: None, ..self.clone() }
    }

    /// `Clone()`: the copy, less the default flag and `IsCurrentLibraryMapping`.
    fn cloned(&self) -> MapData {
        MapData { is_default: false, is_current: false, ..self.copy() }
    }

    fn same_fields(&self, o: &MapData) -> bool {
        self.name == o.name
            && self.folder == o.folder
            && self.notes == o.notes
            && self.is_default == o.is_default
            && self.is_matrix == o.is_matrix
            && self.using_defaults == o.using_defaults
            && self.is_current == o.is_current
            && self.strings_are_rows == o.strings_are_rows
            && self.map_items.is_some() == o.map_items.is_some()
    }

    fn decode(kv: &XElement) -> Option<MapData> {
        let key = kv.child("Key")?.text();
        let v = kv.child("Value")?;
        let flag = |name: &str, default: bool| match v.child(name) {
            Some(c) if !c.nil => c.text().trim().eq_ignore_ascii_case("true"),
            _ => default,
        };
        let text = |name: &str| v.child(name).filter(|c| !c.nil).map(|c| c.text());
        Some(MapData {
            folder: text("LibraryReferenceName").unwrap_or_else(|| key.clone()),
            name: key,
            notes: text("Notes").unwrap_or_default(),
            is_default: flag("IsDefaultMapping", false),
            is_matrix: flag("IsMatrix", true),
            using_defaults: flag("UsingDefaults", true),
            is_current: flag("IsCurrentLibraryMapping", false),
            strings_are_rows: flag("StringsAreRows", false),
            map_items: v.child("MapItems").filter(|c| !c.nil).cloned(),
            raw: Some(kv.clone()),
        })
    }

    /// The shape Windows writes: the base class's two members, then every public property in alphabetical order. `LipSyncMapData` has no
    /// `[DataContract]`, so its `EmitDefaultValue` attributes are ignored and `_libraryReferenceName` is never written.
    fn encode(&self, item: &str) -> XElement {
        let old = self.raw.as_ref().and_then(|kv| kv.child("Value"));
        let id = |name: &str| old.and_then(|v| v.child_text(name)).unwrap_or_else(|| ZERO_GUID.to_string());
        let bool_text = |b: bool| if b { "true" } else { "false" };
        let mut value = XElement::new(ARRAYS_NS, "Value");
        value.push(XElement::with_text("", "ModuleInstanceId", id("ModuleInstanceId")));
        value.push(XElement::with_text("", "ModuleTypeId", id("ModuleTypeId")));
        value.push(XElement::with_text(NS, "IsCurrentLibraryMapping", bool_text(self.is_current)));
        value.push(XElement::with_text(NS, "IsDefaultMapping", bool_text(self.is_default)));
        value.push(XElement::with_text(NS, "IsMatrix", bool_text(self.is_matrix)));
        value.push(XElement::with_text(NS, "LibraryReferenceName", self.folder.as_str()));
        value.push(self.map_items.clone().unwrap_or_else(|| XElement::nil(NS, "MapItems")));
        value.push(XElement::with_text(NS, "Notes", self.notes.as_str()));
        value.push(XElement::with_text(NS, "StringsAreRows", bool_text(self.strings_are_rows)));
        value.push(XElement::with_text(NS, "UsingDefaults", bool_text(self.using_defaults)));
        let mut kv = XElement::new(ARRAYS_NS, item);
        kv.push(XElement::with_text(ARRAYS_NS, "Key", self.name.as_str()));
        kv.push(value);
        kv
    }

    fn to_xml(&self, item: &str) -> XElement {
        match &self.raw {
            Some(raw) if MapData::decode(raw).is_some_and(|d| d.same_fields(self)) => raw.clone(),
            _ => self.encode(item),
        }
    }
}

/// One row of the Image Maps list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSummary {
    pub name: String,
    pub notes: String,
    /// The map that acts as default: the flagged one, or the first in the library when none is.
    pub is_default: bool,
}

/// What Clone did. The library entry is made even when the pictures could not be copied, as upstream leaves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneOutcome {
    pub name: String,
    /// "Unable to copy mapping data!" and the reason.
    pub error: Option<String>,
}

/// `LipSyncMapLibrary` over the module store's entry.
#[derive(Clone, Debug)]
pub struct LipSyncLibrary {
    /// A .NET `Dictionary` enumerates in slot order and gives the slot a removal frees to the next add. The library's order is the
    /// Default Image Map submenu's, and with no map flagged its first map is the default, so the slots are modelled.
    slots: Vec<Option<MapData>>,
    free: Vec<usize>,
    counter: Arc<AtomicI32>,
    item_name: String,
    /// F96, with the FIX: any change since this object was read, never cleared by a later cancelled action.
    changed: bool,
    /// The map New made and whether its folder was already there, until its editor closes.
    pending_new: Option<(String, bool)>,
}

impl LipSyncLibrary {
    pub fn from_store(store: &ModuleStore) -> LipSyncLibrary {
        LipSyncLibrary::from_store_with_counter(store, process_counter())
    }

    pub fn from_store_with_counter(store: &ModuleStore, counter: Arc<AtomicI32>) -> LipSyncLibrary {
        let list = store
            .type_data(LIP_SYNC_MAP_TYPE_ID)
            .and_then(|e| e.data())
            .and_then(|d| d.child("_library"))
            .filter(|l| !l.nil);
        let mut library = LipSyncLibrary {
            slots: Vec::new(),
            free: Vec::new(),
            counter,
            item_name: ITEM.to_string(),
            changed: false,
            pending_new: None,
        };
        match list {
            Some(list) => {
                if let Some(first) = list.elements().next() {
                    library.item_name = first.local.clone();
                }
                library.slots = list.elements().filter_map(MapData::decode).map(Some).collect();
            }
            // F119: with nothing stored, the first read makes one map on the constructor's values.
            None => {
                library.slots.push(Some(MapData {
                    name: NEW_MAP_NAME.to_string(),
                    folder: NEW_MAP_NAME.to_string(),
                    ..MapData::default()
                }));
            }
        }
        library
    }

    /// Writes the library back. A map that has not changed keeps its exact XML.
    pub fn write_to(&self, store: &mut ModuleStore) {
        let mut data = match store.type_data(LIP_SYNC_MAP_TYPE_ID).and_then(|e| e.data()) {
            Some(d) => d.clone(),
            None => {
                let mut d = XElement::new(NS, "LipSyncMapStaticData");
                let id = LIP_SYNC_MAP_TYPE_ID.hyphenated().to_string();
                d.push(XElement::with_text("", "ModuleInstanceId", id.as_str()));
                d.push(XElement::with_text("", "ModuleTypeId", id.as_str()));
                d
            }
        };
        let mut list = XElement::new(NS, "_library");
        for map in self.maps() {
            list.push(map.to_xml(&self.item_name));
        }
        match data.children.iter().position(|n| matches!(n, XNode::Element(e) if e.local == "_library")) {
            Some(i) => data.children[i] = XNode::Element(list),
            None => {
                data.push(list);
            }
        }
        match store.type_data_mut(LIP_SYNC_MAP_TYPE_ID) {
            Some(entry) => entry.set_data(data),
            None => store.set_type_data(ModuleEntry::new(
                DATA_MODEL_TYPE,
                &LIP_SYNC_MAP_TYPE_ID,
                &LIP_SYNC_MAP_TYPE_ID,
                ASSEMBLY,
                data,
            )),
        }
    }

    fn maps(&self) -> impl Iterator<Item = &MapData> {
        self.slots.iter().flatten()
    }

    fn slot_of(&self, name: &str) -> Option<usize> {
        self.slots.iter().position(|s| s.as_ref().is_some_and(|m| m.name == name))
    }

    pub fn get(&self, name: &str) -> Option<&MapData> {
        self.maps().find(|m| m.name == name)
    }

    pub fn len(&self) -> usize {
        self.maps().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Library order: the Default Image Map submenu's, and the LipSync effect's Phoneme Mapping list's.
    pub fn names(&self) -> Vec<String> {
        self.maps().map(|m| m.name.clone()).collect()
    }

    /// The Image Maps list, sorted by name.
    pub fn list(&self) -> Vec<MapSummary> {
        let default = self.default_name();
        let mut rows: Vec<MapSummary> = self
            .maps()
            .map(|m| MapSummary {
                name: m.name.clone(),
                notes: m.notes.clone(),
                is_default: default.as_deref() == Some(m.name.as_str()),
            })
            .collect();
        rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.name.cmp(&b.name)));
        rows
    }

    /// `DefaultMapping`: the first flagged map, else the first in the library.
    pub fn default_name(&self) -> Option<String> {
        self.maps().find(|m| m.is_default).or_else(|| self.maps().next()).map(|m| m.name.clone())
    }

    /// F89: all flags clear and this one's sets. False when there is no such map.
    pub fn set_default(&mut self, name: &str) -> bool {
        if !self.contains(name) {
            return false;
        }
        for map in self.slots.iter_mut().flatten() {
            let flag = map.name == name;
            if map.is_default != flag {
                map.is_default = flag;
                self.changed = true;
            }
        }
        true
    }

    /// F119: a sequence from before data version 5 names these maps, so their pictures are the user's and F98 must not overwrite them.
    pub fn clear_using_defaults(&mut self, names: &[String]) -> bool {
        let mut any = false;
        for map in self.slots.iter_mut().flatten() {
            if map.is_matrix && map.using_defaults && names.iter().any(|n| *n == map.name) {
                map.using_defaults = false;
                any = true;
            }
        }
        self.changed |= any;
        any
    }

    /// `NeedsStringMapMigration`.
    pub fn has_string_maps(&self) -> bool {
        self.maps().any(|m| !m.is_matrix)
    }

    /// F113's two halves for one map: a string map leaves the library and hands over its items; a matrix map only drops its
    /// leftover `MapItems`.
    pub(super) fn take_for_migration(&mut self, name: &str) -> Option<Option<XElement>> {
        let i = self.slot_of(name)?;
        self.changed = true;
        let map = self.slots[i].as_mut()?;
        if map.is_matrix {
            map.map_items = None;
            return None;
        }
        let items = map.map_items.take();
        self.slots[i] = None;
        self.free.push(i);
        Some(items)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.slot_of(name).is_some()
    }

    /// `AddMapping`'s naming: a blank name is `New Map`, and a taken one has `(n)` appended from the run's counter until it is free.
    /// GUARD-free KEEP: plain concatenation, so `New Map(1)` taken gives `New Map(1)(2)`.
    pub fn unique_name(&self, name: &str) -> String {
        let mut name = if name.trim().is_empty() { NEW_MAP_NAME.to_string() } else { name.to_string() };
        while self.contains(&name) {
            let n = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
            name = format!("{name}({n})");
        }
        name
    }

    fn put(&mut self, map: MapData) {
        self.changed = true;
        if let Some(i) = self.slot_of(&map.name) {
            self.slots[i] = Some(map);
        } else if let Some(i) = self.free.pop() {
            self.slots[i] = Some(map);
        } else {
            self.slots.push(Some(map));
        }
    }

    /// `AddMapping`.
    fn add(&mut self, insert_new: bool, name: &str, mut map: MapData) -> String {
        let name = if insert_new { self.unique_name(name) } else { name.to_string() };
        map.is_current = true;
        map.name = name.clone();
        map.folder = name.clone();
        map.is_matrix = true;
        self.put(if insert_new { map.cloned() } else { map });
        name
    }

    /// `RemoveMapping(name)`, the entry alone.
    fn take(&mut self, name: &str) -> Option<MapData> {
        let i = self.slot_of(name)?;
        self.free.push(i);
        self.changed = true;
        self.slots[i].take()
    }

    /// After an edit or rename, the map that was the default still is (FIX F95, F105; upstream's clone drops the flag).
    fn keep_default(&mut self, was_default: bool, name: &str) {
        if was_default && self.default_name().as_deref() != Some(name) {
            self.set_default(name);
        }
    }

    // -----------------------------------------------------------------------------------------------------------------
    // The Image Maps window, F91 to F95, F118
    // -----------------------------------------------------------------------------------------------------------------

    /// New, F91. The caller opens the editor on the name, and calls `cancel_new` if that is cancelled.
    pub fn new_map(&mut self, root: &Path) -> String {
        let name = self.add(true, "", MapData::default());
        let existed = map_dir(root, &name).is_some_and(|d| d.exists());
        self.pending_new = Some((name.clone(), existed));
        name
    }

    /// New's editor was cancelled: the entry goes, and so does its folder unless it was there before (FIX F91; upstream leaves the
    /// folder and the default pictures the editor wrote on opening).
    pub fn cancel_new(&mut self, root: &Path) {
        let Some((name, existed)) = self.pending_new.take() else { return };
        self.take(&name);
        if !existed {
            if let Some(dir) = map_dir(root, &name) {
                let _ = std::fs::remove_dir_all(dir);
            }
        }
    }

    /// New's editor was accepted.
    pub fn keep_new(&mut self) {
        self.pending_new = None;
    }

    /// Remove, F93: each map goes with its picture folder. A folder that can't be deleted leaves its map, and the message is returned
    /// after the others are done. GUARD: a selection covering every map removes nothing, so the library is never empty.
    pub fn remove_maps(&mut self, names: &[String], root: &Path) -> Result<(), String> {
        let known: Vec<&String> = names.iter().filter(|n| self.contains(n)).collect();
        if known.len() >= self.len() {
            return Ok(());
        }
        let mut failure = None;
        for name in known {
            let dir = self.get(name).and_then(|m| map_dir(root, &m.folder));
            match dir.filter(|d| d.exists()).map(std::fs::remove_dir_all) {
                Some(Err(e)) => failure = Some(format!("Unable to remove mapping data!\n\n{e}")),
                _ => {
                    self.take(name);
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }

    /// Clone, F94: a copy under the same name made unique, with the pictures copied. None when there is no such map.
    pub fn clone_map(&mut self, name: &str, root: &Path) -> Option<CloneOutcome> {
        let source = self.get(name)?.clone();
        let new_name = self.add(true, name, source.copy());
        let error = copy_pictures(&source, &new_name, root).err().map(|e| format!("Unable to copy mapping data!\n\n{e}"));
        Some(CloneOutcome { name: new_name, error })
    }

    /// Rename in place, F95: the map is copied to the new name, made unique, and the old one removed with its files. `Ok(None)` when
    /// nothing was to be done.
    ///
    /// GUARDs, F118: a failed move keeps the old map and its folder; a case-only rename goes through a temporary name, since upstream's
    /// copy deletes the "target" folder, which on a case-insensitive volume is the source; and a label with a path separator, or `.` or
    /// `..`, is refused. One more, not in upstream: a target folder that belongs to another map (names differing only by case) is
    /// refused, where upstream would delete that map's pictures.
    pub fn rename_map(&mut self, old: &str, new: &str, root: &Path) -> Result<Option<String>, String> {
        if new.is_empty() || new == old {
            return Ok(None);
        }
        let Some(source) = self.get(old).cloned() else { return Ok(None) };
        if name_is_unsafe(new) {
            return Err(BAD_NAME_MESSAGE.to_string());
        }
        let target = self.unique_name(new);
        let folder_taken = self.maps().any(|m| m.name != old && m.folder.to_lowercase() == target.to_lowercase());
        if folder_taken {
            return Err(EXISTS_MESSAGE.to_string());
        }
        move_pictures(&source, &target, root).map_err(|e| format!("Unable to copy mapping data!\n\n{e}"))?;
        let was_default = self.default_name().as_deref() == Some(old);
        self.add(true, &target, source.copy());
        self.take(old);
        self.keep_default(was_default, &target);
        Ok(Some(target))
    }

    // -----------------------------------------------------------------------------------------------------------------
    // The Image Map editor, F97 to F105
    // -----------------------------------------------------------------------------------------------------------------

    /// Opens a map for editing: the editor works on a copy. F98, KEEP: a map still on its defaults has the bundled pictures written to
    /// its folder here, before anything is accepted, and its notes read blank.
    pub fn begin_edit(&self, name: &str, root: &Path) -> Result<MapEdit, String> {
        let Some(map) = self.get(name) else { return Err(format!("There is no map named {name}.")) };
        if !map.is_matrix {
            crate::logging::error(LOGGER, &format!("Trying to edit a string map: {name}!"), None);
            return Err(format!("Trying to edit a string map: {name}!"));
        }
        let mut data = map.copy();
        let dir = map_dir(root, &map.folder);
        if let Some(dir) = &dir {
            if let Err(e) = std::fs::create_dir_all(dir) {
                crate::logging::error(LOGGER, "Could not create the map's picture folder.", Some(&e.to_string()));
            }
        }
        if data.using_defaults {
            if let Some(dir) = &dir {
                write_default_pictures(dir);
            }
            data.notes = String::new();
            data.using_defaults = false;
        }
        let mut images = HashMap::new();
        if let Some(dir) = &dir {
            for phoneme in ALL_PHONEMES {
                let path = dir.join(picture_file(phoneme));
                if !path.exists() {
                    continue;
                }
                // GUARD: upstream's `new Bitmap(path)` throws, uncaught, on a file that is not a picture.
                match load_png(&path) {
                    Ok(png) => {
                        images.insert(phoneme, png);
                    }
                    Err(e) => crate::logging::error(LOGGER, &format!("Could not load {}", path.display()), Some(&e)),
                }
            }
        }
        Ok(MapEdit {
            original: name.to_string(),
            name: map.folder.clone(),
            notes: data.notes.clone(),
            data,
            images,
            current: 0,
            external: None,
        })
    }

    /// OK in the editor, F105. `overwrite` answers "Overwrite existing <name> mapping?": `None` asks, before anything is written.
    pub fn commit_edit(&mut self, edit: &MapEdit, overwrite: Option<bool>, root: &Path) -> EditCommit {
        let original = edit.original.as_str();
        let Some(old) = self.get(original).cloned() else {
            return EditCommit::Saved { name: original.to_string(), alert: Some(format!("There is no map named {original}.")) };
        };
        let old_dir = map_dir(root, &old.folder);
        let mut name = edit.name.clone();
        let mut alert = None;
        if name != old.folder {
            let new_dir = map_dir(root, &name);
            if name_is_unsafe(&name) {
                alert = Some(BAD_NAME_MESSAGE.to_string());
            } else if new_dir.as_ref().is_none_or(|d| d.exists()) {
                // A blank name's target is the LipSync folder itself, which is there.
                alert = Some(EXISTS_MESSAGE.to_string());
            } else if self.contains(&name) && overwrite.is_none() {
                return EditCommit::NeedsOverwriteAnswer(name);
            } else if let (Some(from), Some(to)) = (&old_dir, &new_dir) {
                if from.exists() {
                    if let Err(e) = std::fs::rename(from, to) {
                        alert = Some(format!("{BAD_NAME_MESSAGE}\n\n{e}"));
                    }
                }
            }
            if alert.is_some() {
                name = old.folder.clone();
            }
        }

        if let Some(dir) = map_dir(root, &name) {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                crate::logging::error(LOGGER, "Could not create the map's picture folder.", Some(&e.to_string()));
            }
            for phoneme in ALL_PHONEMES {
                let path = dir.join(picture_file(phoneme));
                let _ = std::fs::remove_file(&path);
                if let Some(png) = edit.images.get(&phoneme) {
                    if let Err(e) = std::fs::write(&path, png) {
                        crate::logging::error(LOGGER, &format!("Could not save {}", path.display()), Some(&e.to_string()));
                    }
                }
            }
        }

        let mut data = edit.data.copy();
        data.notes = edit.notes.clone();
        data.using_defaults = false;
        let renamed = name != original;
        let keep_both = renamed && self.contains(&name) && overwrite == Some(false);
        let was_default = {
            let default = self.default_name();
            default.as_deref() == Some(original) || (renamed && !keep_both && default.as_deref() == Some(name.as_str()))
        };
        let saved = if keep_both {
            self.add(true, &name, data)
        } else {
            // Upstream removes the old entry and adds the new, which the freed slot puts back in the same place.
            self.take(original);
            let saved = self.add(false, &name, MapData { is_default: false, ..data });
            self.keep_default(was_default, &saved);
            saved
        };
        if self.pending_new.as_ref().is_some_and(|(n, _)| n == original) {
            self.pending_new = None;
        }
        EditCommit::Saved { name: saved, alert }
    }
}

/// What OK in the editor came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditCommit {
    /// Saved under this name. `alert` is a rename that was refused (title "Error!"): the name went back and the save carried on, as
    /// upstream does.
    Saved { name: String, alert: Option<String> },
    /// Another map has the new name: ask "Overwrite existing <name> mapping?" (title "Map exists"), then commit again with the answer.
    NeedsOverwriteAnswer(String),
}

/// `LipSyncMapMatrixEditor`'s state: a copy of the map and its pictures, as PNG data.
#[derive(Clone, Debug)]
pub struct MapEdit {
    original: String,
    pub name: String,
    pub notes: String,
    data: MapData,
    images: HashMap<PhonemeType, Vec<u8>>,
    current: usize,
    external: Option<(PathBuf, PhonemeType)>,
}

impl MapEdit {
    pub fn original_name(&self) -> &str {
        &self.original
    }

    /// F99: `>`, wrapping.
    pub fn next(&mut self) {
        self.current = (self.current + 1) % MAP_PHONEMES.len();
    }

    /// F99: `<`, wrapping.
    pub fn prev(&mut self) {
        self.current = (self.current + MAP_PHONEMES.len() - 1) % MAP_PHONEMES.len();
    }

    pub fn current_phoneme(&self) -> PhonemeType {
        MAP_PHONEMES[self.current]
    }

    pub fn image(&self, phoneme: PhonemeType) -> Option<&[u8]> {
        self.images.get(&phoneme).map(Vec::as_slice)
    }

    pub fn current_image(&self) -> Option<&[u8]> {
        self.image(self.current_phoneme())
    }

    /// The phonemes with a picture, in the editor's order.
    pub fn mapped(&self) -> Vec<PhonemeType> {
        MAP_PHONEMES.iter().copied().filter(|p| self.images.contains_key(p)).collect()
    }

    /// F101: a file that won't load is logged and the old picture stays.
    pub fn set_image(&mut self, phoneme: PhonemeType, file: &Path) -> bool {
        match load_png(file) {
            Ok(png) => {
                self.images.insert(phoneme, png);
                true
            }
            Err(e) => {
                crate::logging::error(LOGGER, &format!("Could not load {}", file.display()), Some(&e));
                false
            }
        }
    }

    /// F104.
    pub fn clear_image(&mut self, phoneme: PhonemeType) {
        self.images.remove(&phoneme);
    }

    /// F101: one file replaces the current phoneme's picture. More than one is the Image Mapper's, and the caller shows it.
    pub fn load_files(&mut self, files: &[PathBuf]) -> bool {
        match files {
            [file] => {
                let phoneme = self.current_phoneme();
                self.set_image(phoneme, file);
                false
            }
            [] => false,
            _ => true,
        }
    }

    /// F102's first choices for these files.
    pub fn mapper_initial(&self, file_count: usize) -> Vec<MapperChoice> {
        mapper_initial(self.current_phoneme(), file_count, &self.mapped())
    }

    /// F102, Accept: a file sets its phoneme, `<Unmapped>` clears it, `<Mapped>` leaves it alone.
    pub fn apply_mapper(&mut self, choices: &[MapperChoice], files: &[PathBuf]) {
        for (phoneme, choice) in MAP_PHONEMES.iter().zip(choices) {
            match choice {
                MapperChoice::Unmapped => self.clear_image(*phoneme),
                MapperChoice::Mapped => {}
                MapperChoice::File(i) => {
                    if let Some(file) = files.get(*i) {
                        self.set_image(*phoneme, file);
                    }
                }
            }
        }
    }

    /// F103, Edit: the current picture, or a blank 128x128 one, as a PNG file for another app to change. Upstream writes
    /// `<PHONEME>_tmp.bmp` into the map's folder and waits for Paint to exit; here it is a file in the temporary folder, and
    /// `finish_external_edit` reads it back.
    pub fn begin_external_edit(&mut self) -> std::io::Result<PathBuf> {
        self.cancel_external_edit();
        let phoneme = self.current_phoneme();
        let dir = std::env::temp_dir().join(format!("vixen-image-map-{}", Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir)?;
        let file = dir.join(format!("{}.png", phoneme.name()));
        let png = match self.images.get(&phoneme) {
            Some(png) => png.clone(),
            None => encode_png(&image::RgbaImage::new(128, 128)).map_err(std::io::Error::other)?,
        };
        std::fs::write(&file, png)?;
        self.external = Some((file.clone(), phoneme));
        Ok(file)
    }

    pub fn external_edit_file(&self) -> Option<&Path> {
        self.external.as_ref().map(|(f, _)| f.as_path())
    }

    /// Done: reads the file back into the phoneme it was made from. GUARD F103: the old picture stays unless the new one loads.
    pub fn finish_external_edit(&mut self) -> Result<(), String> {
        let Some((file, phoneme)) = self.external.take() else { return Ok(()) };
        let loaded = load_png(&file);
        if let Some(dir) = file.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
        self.images.insert(phoneme, loaded?);
        Ok(())
    }

    pub fn cancel_external_edit(&mut self) {
        if let Some((file, _)) = self.external.take() {
            if let Some(dir) = file.parent() {
                let _ = std::fs::remove_dir_all(dir);
            }
        }
    }
}

/// A row's choice in the Image Mapper.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MapperChoice {
    Unmapped,
    Mapped,
    /// An index into the dropped files.
    File(usize),
}

/// F102's first choices, one for each of `MAP_PHONEMES`: the current phoneme takes the first file, each one after it the next file
/// while any remain, and the rest say whether they have a picture. Phonemes before the current one are never given a file.
pub fn mapper_initial(current: PhonemeType, file_count: usize, mapped: &[PhonemeType]) -> Vec<MapperChoice> {
    let mut next = 0;
    MAP_PHONEMES
        .iter()
        .map(|p| {
            if *p == current || (next > 0 && next < file_count) {
                next += 1;
                MapperChoice::File(next - 1)
            } else if mapped.contains(p) {
                MapperChoice::Mapped
            } else {
                MapperChoice::Unmapped
            }
        })
        .collect()
}

/// F101: a drop is taken only when every file is a `.jpg`, `.png` or `.bmp`, in any case. KEEP: upstream means to allow GIF and
/// compares the wrong string, so GIF is refused.
pub fn accepts_drop(files: &[PathBuf]) -> bool {
    !files.is_empty()
        && files.iter().all(|f| {
            f.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| ["jpg", "png", "bmp"].iter().any(|k| e.eq_ignore_ascii_case(k)))
        })
}

fn encode_png(image: &image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

/// Reads a picture by its bytes, whatever its extension says: upstream's `.bmp` files hold PNG or JPEG data too.
fn load_png(file: &Path) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    let image = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    encode_png(&image.to_rgba8())
}

/// `savePicBitmaps` as the editor's constructor runs it: each phoneme's file is deleted and the ten defaults saved.
fn write_default_pictures(dir: &Path) {
    for phoneme in ALL_PHONEMES {
        let _ = std::fs::remove_file(dir.join(picture_file(phoneme)));
    }
    for (phoneme, png) in DEFAULT_PICTURES {
        if let Err(e) = std::fs::write(dir.join(picture_file(phoneme)), png) {
            crate::logging::error(LOGGER, "Could not save a default picture.", Some(&e.to_string()));
        }
    }
}

/// The folder upstream's editor object makes, and fills when the map is on its defaults, whenever one is constructed for a map.
fn prepare_source(source: &MapData, root: &Path) -> std::io::Result<Option<PathBuf>> {
    let Some(dir) = map_dir(root, &source.folder) else { return Ok(None) };
    std::fs::create_dir_all(&dir)?;
    if source.using_defaults {
        write_default_pictures(&dir);
    }
    Ok(Some(dir))
}

/// `CloneMappingFiles`: the target folder is deleted if there, and the source's files copied.
fn copy_pictures(source: &MapData, new_name: &str, root: &Path) -> std::io::Result<()> {
    let Some(from) = prepare_source(source, root)? else { return Ok(()) };
    let Some(to) = map_dir(root, new_name) else { return Ok(()) };
    if to.exists() {
        std::fs::remove_dir_all(&to)?;
    }
    copy_dir(&from, &to)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A rename's files: upstream copies to the new folder, deleting it first if there, and then deletes the old one. Moving the folder
/// comes to the same, and cannot lose the pictures half way.
fn move_pictures(source: &MapData, new_name: &str, root: &Path) -> std::io::Result<()> {
    let Some(from) = prepare_source(source, root)? else { return Ok(()) };
    let Some(to) = map_dir(root, new_name) else { return Ok(()) };
    let same_folder = source.folder.to_lowercase() == new_name.to_lowercase();
    if same_folder {
        let temporary = lipsync_dir(root).join(format!(".rename-{}", Uuid::new_v4().simple()));
        std::fs::rename(&from, &temporary)?;
        if let Err(e) = std::fs::rename(&temporary, &to) {
            let _ = std::fs::rename(&temporary, &from);
            return Err(e);
        }
        return Ok(());
    }
    if to.exists() {
        std::fs::remove_dir_all(&to)?;
    }
    std::fs::rename(&from, &to)
}

// ---------------------------------------------------------------------------------------------------------------------
// The open profile's library
// ---------------------------------------------------------------------------------------------------------------------

/// `LipSync.RenderNodes`' fallback, which the effect crate can't do because it has no library: a picture-map effect naming a map that
/// is not in the library takes the default's name. True if the effect changed.
pub fn resolve_lipsync_mapping(params: &mut vixen_effects::framework::Params, library: &LipSyncLibrary) -> bool {
    if params.enum_name("MappingType") != "Map" || library.contains(params.string("PhonemeMapping")) {
        return false;
    }
    let Some(default) = library.default_name() else { return false };
    params.set("PhonemeMapping", vixen_effects::framework::ParamValue::Str(default));
    true
}

fn is_lip_sync(entry: &crate::sequence::EffectEntry) -> bool {
    entry.descriptor.is_some_and(|d| d.type_id == fx_audio_state::lip_sync::DESCRIPTOR.type_id)
}

impl crate::sequence::SequenceSession {
    /// A sequence that has just opened: its LipSync effects that name a map the library doesn't have take the default, as they would at
    /// their first render upstream. This is not an edit.
    pub fn resolve_lip_sync_mappings(&mut self, library: &LipSyncLibrary) -> usize {
        self.rerender_effects_where(|e| is_lip_sync(e) && resolve_lipsync_mapping(&mut e.node.params, library))
    }

    /// `ResetLipSyncNodes`: every LipSync effect is dirty and renders again.
    pub fn reset_lip_sync_effects(&mut self, library: &LipSyncLibrary) -> usize {
        self.rerender_effects_where(|e| {
            if is_lip_sync(e) {
                resolve_lipsync_mapping(&mut e.node.params, library);
                true
            } else {
                false
            }
        })
    }
}

impl crate::app::AppCore {
    pub fn lip_sync_root(&self) -> PathBuf {
        self.paths.module_data_dir()
    }

    pub fn lip_sync_library(&self) -> LipSyncLibrary {
        LipSyncLibrary::from_store(&self.system.read().module_store)
    }

    /// Puts a changed library where everything reads it. Nothing is saved to disk.
    pub fn set_lip_sync_library(&self, library: &LipSyncLibrary) {
        library.write_to(&mut self.system.write().module_store);
    }

    /// F119, the library's half of `TimedSequenceMigrator`'s version 4 to 5 step: the matrix maps an old sequence's LipSync effects
    /// named stop using the default pictures. KEEP: the change stays in memory until something else writes the module store.
    pub fn apply_lip_sync_migration(&self, matrix_maps: &[String]) -> bool {
        if matrix_maps.is_empty() {
            return false;
        }
        let mut library = self.lip_sync_library();
        let changed = library.clear_using_defaults(matrix_maps);
        if changed {
            self.set_lip_sync_library(&library);
        }
        changed
    }

    /// Tools > LipSync > Default Image Map, F89: the open sequences count as modified, as upstream. FIX: the module store is written
    /// now, where upstream leaves the choice in memory until something else saves it.
    pub fn set_default_image_map(&self, name: &str) -> bool {
        let mut library = self.lip_sync_library();
        if !library.set_default(name) {
            return false;
        }
        self.set_lip_sync_library(&library);
        for session in self.open_sequence_sessions() {
            session.lock().set_modified();
        }
        if let Err(e) = self.save_module_config() {
            crate::logging::error(LOGGER, "Could not save the default image map.", Some(&e.to_string()));
        }
        true
    }

    /// The Image Maps window closed, F96: with changes, the open sequences are modified, their LipSync effects render again, and the
    /// system and module configurations are saved.
    pub fn image_maps_closed(&self, library: &LipSyncLibrary) -> bool {
        if !library.changed() {
            return false;
        }
        self.set_lip_sync_library(library);
        for session in self.open_sequence_sessions() {
            let mut session = session.lock();
            session.set_modified();
            session.reset_lip_sync_effects(library);
        }
        for result in [self.save_system_config(), self.save_module_config()] {
            if let Err(e) = result {
                crate::logging::error(LOGGER, "Could not save the image maps.", Some(&e.to_string()));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_io::module_store::load_module_store_file;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-map-library-{tag}-{}", Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn library() -> LipSyncLibrary {
        LipSyncLibrary::from_store_with_counter(&ModuleStore::new(), Arc::new(AtomicI32::new(0)))
    }

    /// A library of these maps, none on its defaults, each with a one-pixel REST picture.
    fn library_of(names: &[&str], root: &Path) -> LipSyncLibrary {
        let mut lib = library();
        lib.take(NEW_MAP_NAME);
        lib.free.clear();
        lib.slots.clear();
        for name in names {
            lib.put(MapData { name: name.to_string(), folder: name.to_string(), using_defaults: false, ..MapData::default() });
            let dir = lipsync_dir(root).join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("REST.bmp"), pixel(name.len() as u8)).unwrap();
        }
        lib.changed = false;
        lib
    }

    fn pixel(red: u8) -> Vec<u8> {
        encode_png(&image::RgbaImage::from_pixel(1, 1, image::Rgba([red, 0, 0, 255]))).unwrap()
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> =
            std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    #[test]
    fn nothing_stored_reads_as_one_new_map() {
        let lib = library();
        assert_eq!(lib.names(), ["New Map"]);
        let map = lib.get("New Map").unwrap();
        assert!(map.is_matrix && map.using_defaults && !map.is_default && map.notes.is_empty());
        assert_eq!(lib.default_name().as_deref(), Some("New Map"));
    }

    #[test]
    fn unique_names_concatenate_from_one_counter() {
        let root = temp_root("names");
        let mut lib = library();
        assert_eq!(lib.new_map(&root), "New Map(1)");
        assert_eq!(lib.new_map(&root), "New Map(2)");
        // Counter at 0 with `New Map(1)` present: the next number is appended to the name tried so far.
        let mut other = library();
        other.put(MapData { name: "New Map(1)".into(), folder: "New Map(1)".into(), ..MapData::default() });
        assert_eq!(other.unique_name(""), "New Map(1)(2)");
        assert_eq!(other.unique_name("   "), "New Map(3)");
    }

    #[test]
    fn a_new_map_writes_windows_shape() {
        let root = temp_root("shape");
        let mut lib = library();
        lib.new_map(&root);
        let mut store = ModuleStore::new();
        lib.write_to(&mut store);
        let value = store.type_data(LIP_SYNC_MAP_TYPE_ID).unwrap().data().unwrap().child("_library").unwrap().elements().nth(1).unwrap();
        let members: Vec<&str> = value.child("Value").unwrap().elements().map(|e| e.local.as_str()).collect();
        assert_eq!(
            members,
            [
                "ModuleInstanceId",
                "ModuleTypeId",
                "IsCurrentLibraryMapping",
                "IsDefaultMapping",
                "IsMatrix",
                "LibraryReferenceName",
                "MapItems",
                "Notes",
                "StringsAreRows",
                "UsingDefaults"
            ]
        );
        let v = value.child("Value").unwrap();
        assert_eq!(value.child("Key").unwrap().text(), "New Map(1)");
        assert_eq!(v.child_text("LibraryReferenceName").as_deref(), Some("New Map(1)"));
        assert!(v.child("MapItems").unwrap().nil);
        assert_eq!(v.child("Notes").unwrap().text(), "");
        assert_eq!(v.child_text("StringsAreRows").as_deref(), Some("false"));
        assert!(v.child("_libraryReferenceName").is_none());
        assert_eq!(v.child("IsMatrix").unwrap().ns, NS);
        assert_eq!(v.child("ModuleTypeId").unwrap().ns, "");
        let again = LipSyncLibrary::from_store(&store);
        assert_eq!(again.names(), ["New Map", "New Map(1)"]);
        assert!(!again.get("New Map(1)").unwrap().is_current, "a new map is stored as a clone, which is not 'current'");
    }

    #[test]
    fn remove_keeps_one_map_and_deletes_folders() {
        let root = temp_root("remove");
        let mut lib = library_of(&["A", "B", "C"], &root);
        lib.remove_maps(&["A".into(), "B".into(), "C".into()], &root).unwrap();
        assert_eq!(lib.len(), 3);
        assert!(!lib.changed());
        lib.remove_maps(&["A".into(), "C".into()], &root).unwrap();
        assert_eq!(lib.names(), ["B"]);
        assert_eq!(files(&lipsync_dir(&root)), ["B"]);
        assert!(lib.changed());
        lib.remove_maps(&["B".into()], &root).unwrap();
        assert_eq!(lib.names(), ["B"]);
    }

    #[test]
    fn clone_copies_pictures_and_drops_the_default_flag() {
        let root = temp_root("clone");
        let mut lib = library_of(&["Faces"], &root);
        lib.set_default("Faces");
        let outcome = lib.clone_map("Faces", &root).unwrap();
        assert_eq!(outcome, CloneOutcome { name: "Faces(1)".into(), error: None });
        assert!(!lib.get("Faces(1)").unwrap().is_default);
        assert_eq!(files(&lipsync_dir(&root).join("Faces(1)")), ["REST.bmp"]);
        assert_eq!(files(&lipsync_dir(&root).join("Faces")), ["REST.bmp"]);
    }

    #[test]
    fn cloning_a_map_on_its_defaults_writes_them_first() {
        let root = temp_root("clone-defaults");
        let mut lib = library();
        let outcome = lib.clone_map("New Map", &root).unwrap();
        assert_eq!(outcome.name, "New Map(1)");
        assert_eq!(files(&lipsync_dir(&root).join("New Map")).len(), 10);
        assert_eq!(files(&lipsync_dir(&root).join("New Map(1)")).len(), 10);
        assert!(lib.get("New Map").unwrap().using_defaults);
    }

    #[test]
    fn rename_moves_the_files_and_keeps_the_default() {
        let root = temp_root("rename");
        let mut lib = library_of(&["A", "B"], &root);
        lib.set_default("B");
        assert_eq!(lib.rename_map("B", "Faces", &root), Ok(Some("Faces".into())));
        assert_eq!(lib.names(), ["A", "Faces"]);
        assert_eq!(lib.default_name().as_deref(), Some("Faces"));
        assert_eq!(files(&lipsync_dir(&root)), ["A", "Faces"]);
        assert_eq!(files(&lipsync_dir(&root).join("Faces")), ["REST.bmp"]);
        // A taken name is made unique, not refused.
        assert_eq!(lib.rename_map("Faces", "A", &root), Ok(Some("A(1)".into())));
        // Nothing to do.
        assert_eq!(lib.rename_map("A", "", &root), Ok(None));
        assert_eq!(lib.rename_map("A", "A", &root), Ok(None));
        // A label of only spaces becomes New Map.
        assert_eq!(lib.rename_map("A", "  ", &root), Ok(Some("New Map".into())));
    }

    #[test]
    fn an_unflagged_default_stays_default_through_a_rename() {
        let root = temp_root("rename-first");
        let mut lib = library_of(&["A", "B"], &root);
        assert_eq!(lib.default_name().as_deref(), Some("A"));
        lib.rename_map("A", "Z", &root).unwrap();
        assert_eq!(lib.default_name().as_deref(), Some("Z"));
    }

    #[test]
    fn a_case_only_rename_keeps_the_pictures() {
        let root = temp_root("rename-case");
        let mut lib = library_of(&["Faces"], &root);
        assert_eq!(lib.rename_map("Faces", "faces", &root), Ok(Some("faces".into())));
        assert_eq!(files(&lipsync_dir(&root)), ["faces"]);
        assert_eq!(files(&lipsync_dir(&root).join("faces")), ["REST.bmp"]);
    }

    #[test]
    fn rename_refuses_path_names_and_keeps_the_map_when_the_move_fails() {
        let root = temp_root("rename-guard");
        let mut lib = library_of(&["A", "B"], &root);
        for bad in ["..", ".", "a/b", "a\\b"] {
            assert_eq!(lib.rename_map("A", bad, &root), Err(BAD_NAME_MESSAGE.to_string()));
        }
        assert_eq!(lib.rename_map("A", "b", &root), Err(EXISTS_MESSAGE.to_string()), "another map's folder, by case");
        assert_eq!(lib.names(), ["A", "B"]);
        // A file where the LipSync folder's child should go makes the move fail.
        let blocked = temp_root("rename-blocked");
        std::fs::write(blocked.join("LipSync"), b"not a folder").unwrap();
        let mut lib = library_of(&[], &root);
        lib.put(MapData { name: "A".into(), folder: "A".into(), using_defaults: false, ..MapData::default() });
        lib.put(MapData { name: "B".into(), folder: "B".into(), using_defaults: false, ..MapData::default() });
        assert!(lib.rename_map("A", "C", &blocked).unwrap_err().starts_with("Unable to copy mapping data!"));
        assert_eq!(lib.names(), ["A", "B"]);
    }

    #[test]
    fn begin_edit_writes_defaults_only_for_a_map_on_its_defaults() {
        let root = temp_root("defaults");
        let mut lib = library();
        lib.slots[0].as_mut().unwrap().notes = "kept in the library".into();
        let edit = lib.begin_edit("New Map", &root).unwrap();
        assert_eq!(edit.mapped().len(), 10);
        assert_eq!(edit.notes, "", "a map on its defaults reads blank notes");
        assert_eq!(files(&lipsync_dir(&root).join("New Map")).len(), 10);
        assert!(lib.get("New Map").unwrap().using_defaults, "cancelling leaves the flag");

        let lib = library_of(&["Faces"], &root);
        let edit = lib.begin_edit("Faces", &root).unwrap();
        assert_eq!(edit.mapped(), [PhonemeType::Rest]);
        assert_eq!(files(&lipsync_dir(&root).join("Faces")), ["REST.bmp"]);
    }

    #[test]
    fn a_bmp_name_holding_png_bytes_loads_and_a_bad_file_is_skipped() {
        let root = temp_root("sniff");
        let lib = library_of(&["Faces"], &root);
        std::fs::write(lipsync_dir(&root).join("Faces").join("AI.bmp"), DEFAULT_PICTURES[0].1).unwrap();
        std::fs::write(lipsync_dir(&root).join("Faces").join("E.bmp"), b"not a picture").unwrap();
        let mut edit = lib.begin_edit("Faces", &root).unwrap();
        assert_eq!(edit.mapped(), [PhonemeType::Ai, PhonemeType::Rest]);
        // F101: a file that won't load leaves the old picture.
        let before = edit.image(PhonemeType::Ai).unwrap().to_vec();
        assert!(!edit.set_image(PhonemeType::Ai, &lipsync_dir(&root).join("Faces").join("E.bmp")));
        assert_eq!(edit.image(PhonemeType::Ai).unwrap(), before);
    }

    #[test]
    fn navigation_wraps_both_ways() {
        let root = temp_root("nav");
        let lib = library_of(&["Faces"], &root);
        let mut edit = lib.begin_edit("Faces", &root).unwrap();
        assert_eq!(edit.current_phoneme(), PhonemeType::Ai);
        edit.prev();
        assert_eq!(edit.current_phoneme(), PhonemeType::Wq);
        edit.next();
        edit.next();
        assert_eq!(edit.current_phoneme(), PhonemeType::E);
    }

    #[test]
    fn mapper_choices_run_from_the_current_phoneme() {
        use MapperChoice::*;
        assert_eq!(
            mapper_initial(PhonemeType::E, 3, &[PhonemeType::Ai]),
            [Mapped, File(0), File(1), File(2), Unmapped, Unmapped, Unmapped, Unmapped, Unmapped, Unmapped]
        );
        // Surplus files are unused when the run reaches the end.
        assert_eq!(mapper_initial(PhonemeType::Wq, 3, &[]).last(), Some(&File(0)));
    }

    #[test]
    fn mapper_accept_sets_clears_and_leaves() {
        let root = temp_root("mapper");
        let lib = library_of(&["Faces"], &root);
        let mut edit = lib.begin_edit("Faces", &root).unwrap();
        let a = root.join("a.png");
        std::fs::write(&a, pixel(9)).unwrap();
        edit.set_image(PhonemeType::E, &a);
        let mut choices = vec![MapperChoice::Mapped; 10];
        choices[0] = MapperChoice::File(0);
        choices[1] = MapperChoice::Unmapped;
        edit.apply_mapper(&choices, &[a]);
        assert_eq!(edit.mapped(), [PhonemeType::Ai, PhonemeType::Rest]);
    }

    #[test]
    fn drops_take_only_jpg_png_bmp() {
        let p = |s: &str| PathBuf::from(s);
        assert!(accepts_drop(&[p("a.PNG"), p("b.jpg"), p("c.Bmp")]));
        assert!(!accepts_drop(&[p("a.gif")]));
        assert!(!accepts_drop(&[p("a.png"), p("b.gif")]));
        assert!(!accepts_drop(&[]));
        assert!(!accepts_drop(&[p("noextension")]));
    }

    #[test]
    fn commit_saves_pictures_notes_and_keeps_the_place_and_default() {
        let root = temp_root("commit");
        let mut lib = library_of(&["A", "B", "C"], &root);
        lib.set_default("B");
        let mut edit = lib.begin_edit("B", &root).unwrap();
        edit.notes = "front porch".into();
        edit.clear_image(PhonemeType::Rest);
        let a = root.join("a.png");
        std::fs::write(&a, pixel(9)).unwrap();
        assert!(edit.set_image(PhonemeType::O, &a));
        assert_eq!(lib.commit_edit(&edit, None, &root), EditCommit::Saved { name: "B".into(), alert: None });
        assert_eq!(lib.names(), ["A", "B", "C"]);
        assert_eq!(files(&lipsync_dir(&root).join("B")), ["O.bmp"]);
        let map = lib.get("B").unwrap();
        assert!(map.is_default && map.is_current && map.is_matrix && !map.using_defaults);
        assert_eq!(map.notes, "front porch");
    }

    #[test]
    fn commit_renames_the_folder_in_place() {
        let root = temp_root("commit-rename");
        let mut lib = library_of(&["A", "B", "C"], &root);
        let mut edit = lib.begin_edit("B", &root).unwrap();
        edit.name = "Faces".into();
        assert_eq!(lib.commit_edit(&edit, None, &root), EditCommit::Saved { name: "Faces".into(), alert: None });
        assert_eq!(lib.names(), ["A", "Faces", "C"]);
        assert_eq!(files(&lipsync_dir(&root)), ["A", "C", "Faces"]);
        assert_eq!(files(&lipsync_dir(&root).join("Faces")), ["REST.bmp"]);
    }

    #[test]
    fn commit_onto_an_existing_folder_keeps_the_old_name_and_saves() {
        let root = temp_root("commit-exists");
        let mut lib = library_of(&["A", "B"], &root);
        for (bad, message) in [("A", EXISTS_MESSAGE), ("", EXISTS_MESSAGE), ("x/y", BAD_NAME_MESSAGE), ("..", BAD_NAME_MESSAGE)] {
            let mut edit = lib.begin_edit("B", &root).unwrap();
            edit.name = bad.into();
            edit.notes = format!("tried {bad}");
            assert_eq!(
                lib.commit_edit(&edit, None, &root),
                EditCommit::Saved { name: "B".into(), alert: Some(message.to_string()) }
            );
            assert_eq!(lib.get("B").unwrap().notes, format!("tried {bad}"));
            assert_eq!(lib.names(), ["A", "B"]);
        }
    }

    #[test]
    fn commit_onto_a_name_without_a_folder_asks() {
        let root = temp_root("commit-ask");
        let mut lib = library_of(&["A", "B"], &root);
        lib.put(MapData { name: "Ghost".into(), folder: "Ghost".into(), ..MapData::default() });
        let mut edit = lib.begin_edit("A", &root).unwrap();
        edit.name = "Ghost".into();
        assert_eq!(lib.commit_edit(&edit, None, &root), EditCommit::NeedsOverwriteAnswer("Ghost".into()));
        assert_eq!(files(&lipsync_dir(&root)), ["A", "B"], "nothing is written before the answer");

        let mut keep = lib.clone();
        assert_eq!(keep.commit_edit(&edit, Some(false), &root), EditCommit::Saved { name: "Ghost(1)".into(), alert: None });
        assert_eq!(keep.names(), ["A", "B", "Ghost", "Ghost(1)"]);

        // The folder moved with the first answer; put it back for the second.
        std::fs::rename(lipsync_dir(&root).join("Ghost"), lipsync_dir(&root).join("A")).unwrap();
        assert_eq!(lib.commit_edit(&edit, Some(true), &root), EditCommit::Saved { name: "Ghost".into(), alert: None });
        assert_eq!(lib.names(), ["B", "Ghost"]);
        assert_eq!(lib.default_name().as_deref(), Some("Ghost"), "A was the default by being first");
    }

    #[test]
    fn cancelled_new_removes_the_entry_and_the_folder_it_made() {
        let root = temp_root("new-cancel");
        let mut lib = library_of(&["A"], &root);
        let name = lib.new_map(&root);
        lib.begin_edit(&name, &root).unwrap();
        assert!(lipsync_dir(&root).join(&name).exists());
        lib.cancel_new(&root);
        assert_eq!(lib.names(), ["A"]);
        assert_eq!(files(&lipsync_dir(&root)), ["A"]);
        assert!(lib.changed(), "F96: the flag is sticky");
    }

    #[test]
    fn external_edit_reloads_only_what_loads() {
        let root = temp_root("external");
        let lib = library_of(&["Faces"], &root);
        let mut edit = lib.begin_edit("Faces", &root).unwrap();
        // No picture: a blank 128x128 one.
        let file = edit.begin_external_edit().unwrap();
        assert_eq!(image::open(&file).unwrap().to_rgba8().dimensions(), (128, 128));
        std::fs::write(&file, pixel(77)).unwrap();
        edit.finish_external_edit().unwrap();
        assert!(edit.image(PhonemeType::Ai).is_some());
        assert!(!file.exists());
        let before = edit.image(PhonemeType::Ai).unwrap().to_vec();
        let file = edit.begin_external_edit().unwrap();
        std::fs::write(&file, b"broken").unwrap();
        assert!(edit.finish_external_edit().is_err());
        assert_eq!(edit.image(PhonemeType::Ai).unwrap(), before);
    }

    #[test]
    fn default_flags_and_old_sequence_names() {
        let root = temp_root("flags");
        let mut lib = library_of(&["A", "B"], &root);
        assert!(!lib.set_default("Nope"));
        assert!(lib.set_default("B"));
        assert_eq!(lib.list().iter().map(|r| (r.name.as_str(), r.is_default)).collect::<Vec<_>>(), [("A", false), ("B", true)]);
        let mut lib = library();
        assert!(lib.clear_using_defaults(&["New Map".into()]));
        assert!(!lib.get("New Map").unwrap().using_defaults);
        assert!(!lib.clear_using_defaults(&["New Map".into()]));
    }

    #[test]
    fn opening_a_sequence_migrated_from_before_version_5_takes_its_matrix_maps_off_the_defaults() {
        use crate::sequence::SequenceSession;
        let root = std::env::temp_dir().join(format!("vixen-lipsync-migration-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let core = std::sync::Arc::new(crate::app::AppCore::open(root.clone(), true).unwrap());
        assert!(core.lip_sync_library().get(NEW_MAP_NAME).unwrap().using_defaults);
        let store = root.join("SystemData").join("ModuleStore.xml");
        let on_disk = std::fs::read(&store).ok();

        let plain = std::sync::Arc::new(parking_lot::Mutex::new(SequenceSession::new_untitled(&core.session_environment())));
        core.register_sequence_session(&plain);
        assert!(core.lip_sync_library().get(NEW_MAP_NAME).unwrap().using_defaults, "a sequence that wasn't migrated changes nothing");

        let mut old = SequenceSession::new_untitled(&core.session_environment());
        old.document_mut().migrated_matrix_maps = vec![NEW_MAP_NAME.to_string(), "Gone".to_string()];
        let old = std::sync::Arc::new(parking_lot::Mutex::new(old));
        core.register_sequence_session(&old);
        assert!(!core.lip_sync_library().get(NEW_MAP_NAME).unwrap().using_defaults);
        assert_eq!(std::fs::read(&store).ok(), on_disk, "the change waits in memory for the next save of the module store");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_effect_naming_a_missing_map_takes_the_default() {
        use vixen_effects::framework::ParamValue;
        let root = temp_root("resolve");
        let mut lib = library_of(&["A", "B"], &root);
        lib.set_default("B");
        let mut params = fx_audio_state::lip_sync::DESCRIPTOR.new_params();
        params.set("MappingType", ParamValue::Enum("Map".into()));
        params.set("PhonemeMapping", ParamValue::Str("Gone".into()));
        assert!(resolve_lipsync_mapping(&mut params, &lib));
        assert_eq!(params.string("PhonemeMapping"), "B");
        assert!(!resolve_lipsync_mapping(&mut params, &lib), "a map the library has is left alone");

        params.set("PhonemeMapping", ParamValue::Str("A".into()));
        assert!(!resolve_lipsync_mapping(&mut params, &lib));
        assert_eq!(params.string("PhonemeMapping"), "A");

        let mut other = fx_audio_state::lip_sync::DESCRIPTOR.new_params();
        other.set("PhonemeMapping", ParamValue::Str("Gone".into()));
        if other.enum_name("MappingType") != "Map" {
            assert!(!resolve_lipsync_mapping(&mut other, &lib), "only Map mode looks a map up");
            assert_eq!(other.string("PhonemeMapping"), "Gone");
        }
    }

    #[test]
    fn the_owners_library_round_trips_untouched_and_a_flag_change_keeps_map_items() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/show-copy/SystemData/ModuleStore.xml");
        if !path.exists() {
            return;
        }
        let mut store = load_module_store_file(&path).unwrap();
        let before = vixen_xml::XElement::to_fragment(store.type_data(LIP_SYNC_MAP_TYPE_ID).unwrap().data().unwrap());
        let mut lib = LipSyncLibrary::from_store(&store);
        assert_eq!(lib.names(), ["New Map"]);
        lib.write_to(&mut store);
        let after = vixen_xml::XElement::to_fragment(store.type_data(LIP_SYNC_MAP_TYPE_ID).unwrap().data().unwrap());
        assert_eq!(before, after);
        lib.set_default("New Map");
        lib.write_to(&mut store);
        let data = store.type_data(LIP_SYNC_MAP_TYPE_ID).unwrap().data().unwrap();
        let value = data.child("_library").unwrap().elements().next().unwrap().child("Value").unwrap();
        assert_eq!(value.child_text("IsDefaultMapping").as_deref(), Some("true"));
        let items = value.child("MapItems").unwrap();
        assert!(!items.nil && items.elements().count() == 1, "a flag change is not an edit: MapItems stays");
    }
}
