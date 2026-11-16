//! `ModuleStore.xml`: module data the owning module decodes. Port of `XmlModuleStoreFilePolicy`,
//! `XmlModuleDataModelSerializer` and `ModuleStoreXElementMigrator`.
//!
//! Each entry stays an `XElement`, so any module's data survives whether or not this port knows the module.
//! The crate that owns a module decodes its entry when it needs to.

use crate::dcs::{child_string, parse_guid, windows_file_name};
use crate::error::IoError;
use crate::file;
use crate::xml::{read_xml_file, write_document, DocStyle};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;
use vixen_xml::{XElement, XNode};

/// `ObjectVersion.ModuleStore`.
pub const CURRENT_MODULE_STORE_VERSION: i32 = 5;

const ATTR_DATA_MODEL_TYPE: &str = "dataModelType";
const ATTR_MODULE_TYPE: &str = "moduleType";
const ATTR_MODULE_INSTANCE: &str = "moduleInstance";
const ATTR_ASSEMBLY: &str = "assemblyName";

/// Every `<Module>` wraps a pasted-in object, which declares `xmlns:i` for itself.
pub(crate) const MODULE_STORE_STYLE: DocStyle = DocStyle { root_prefixes: &[], object_depth: Some(3) };

/// One `<Module>`: the attributes that say whose it is, and the module's DataContract object.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleEntry {
    /// The whole `<Module>` element.
    pub element: XElement,
}

impl ModuleEntry {
    /// Builds an entry as `XmlModuleDataModelSerializer.WriteObject` does.
    pub fn new(
        data_model_type: &str,
        module_type: &Uuid,
        module_instance: &Uuid,
        assembly_name: &str,
        data: XElement,
    ) -> ModuleEntry {
        let mut e = XElement::new("", "Module");
        e.set_attr(ATTR_DATA_MODEL_TYPE, data_model_type);
        e.set_attr(ATTR_MODULE_TYPE, crate::dcs::format_guid(module_type));
        e.set_attr(ATTR_MODULE_INSTANCE, crate::dcs::format_guid(module_instance));
        e.set_attr(ATTR_ASSEMBLY, assembly_name);
        e.push(data);
        ModuleEntry { element: e }
    }

    /// `dataModelType`, such as `VixenModules.Property.Color.ColorData, Color`.
    pub fn data_model_type(&self) -> Option<&str> {
        self.element.attr(ATTR_DATA_MODEL_TYPE)
    }

    /// `assemblyName`, which entries from before .NET Core do not have.
    pub fn assembly_name(&self) -> Option<&str> {
        self.element.attr(ATTR_ASSEMBLY)
    }

    pub fn module_type(&self) -> Option<Uuid> {
        self.element.attr(ATTR_MODULE_TYPE).and_then(parse_guid)
    }

    pub fn module_instance(&self) -> Option<Uuid> {
        self.element.attr(ATTR_MODULE_INSTANCE).and_then(parse_guid)
    }

    /// The module's data object.
    pub fn data(&self) -> Option<&XElement> {
        self.element.elements().next()
    }

    pub fn data_mut(&mut self) -> Option<&mut XElement> {
        self.element.elements_mut().next()
    }

    /// Replaces the module's data object.
    pub fn set_data(&mut self, data: XElement) {
        self.element.children.retain(|n| !matches!(n, XNode::Element(_)));
        self.element.push(data);
    }

    /// Replaces the module's data with `new_plain`, a typed encoding, keeping what the file said wherever typed encoding has
    /// nothing to say: members it doesn't write, and details of members whose value hasn't changed. `old_plain` is the typed
    /// encoding of the data as it stands now.
    pub fn merge_data(&mut self, old_plain: &XElement, new_plain: XElement) {
        let merged = match self.data() {
            Some(old) => crate::fidelity::reapply(new_plain, crate::fidelity::diff(old, old_plain).as_ref()),
            None => new_plain,
        };
        self.set_data(merged);
    }
}

/// The two data sets of `ModuleStore.xml`.
#[derive(Clone, Debug, Default)]
pub struct ModuleStore {
    /// The root's `version`, after migration.
    pub version: i32,
    /// The version on disk before migrating.
    pub source_version: i32,
    type_entries: Vec<ModuleEntry>,
    instance_entries: Vec<ModuleEntry>,
    type_index: HashMap<Uuid, usize>,
    instance_index: HashMap<(Uuid, Uuid), usize>,
    /// Sections after the two Vixen reads, and any attributes it does not write.
    extra_sections: Vec<XElement>,
    root_attrs: Vec<vixen_xml::XAttr>,
}

impl ModuleStore {
    /// An empty store at the current version, what C# makes when the file does not exist.
    pub fn new() -> ModuleStore {
        ModuleStore {
            version: CURRENT_MODULE_STORE_VERSION,
            source_version: CURRENT_MODULE_STORE_VERSION,
            ..ModuleStore::default()
        }
    }

    /// Instance-level data: `(type id, instance id)`, as `ModuleLocalDataSet` keys it.
    pub fn instance(&self, type_id: impl Borrow<Uuid>, instance_id: impl Borrow<Uuid>) -> Option<&ModuleEntry> {
        let i = *self.instance_index.get(&(*type_id.borrow(), *instance_id.borrow()))?;
        self.instance_entries.get(i)
    }

    /// Mutable access to an instance-level entry. Its type and instance ids must stay as they are.
    pub fn instance_mut(
        &mut self,
        type_id: impl Borrow<Uuid>,
        instance_id: impl Borrow<Uuid>,
    ) -> Option<&mut ModuleEntry> {
        let i = *self.instance_index.get(&(*type_id.borrow(), *instance_id.borrow()))?;
        self.instance_entries.get_mut(i)
    }

    /// Type-level data, one entry per module type, as `ModuleStaticDataSet` keys it.
    pub fn type_data(&self, type_id: impl Borrow<Uuid>) -> Option<&ModuleEntry> {
        let i = *self.type_index.get(type_id.borrow())?;
        self.type_entries.get(i)
    }

    /// Mutable access to a type-level entry. Its type id must stay as it is.
    pub fn type_data_mut(&mut self, type_id: impl Borrow<Uuid>) -> Option<&mut ModuleEntry> {
        let i = *self.type_index.get(type_id.borrow())?;
        self.type_entries.get_mut(i)
    }

    /// Adds or replaces an instance-level entry, keyed by the entry's own attributes.
    pub fn set_instance(&mut self, entry: ModuleEntry) {
        let key = entry.module_type().zip(entry.module_instance());
        match key.and_then(|k| self.instance_index.get(&k).copied()) {
            Some(i) => self.instance_entries[i] = entry,
            None => {
                if let Some(k) = key {
                    self.instance_index.insert(k, self.instance_entries.len());
                }
                self.instance_entries.push(entry);
            }
        }
    }

    /// Adds or replaces a type-level entry, keyed by the entry's own `moduleType`.
    pub fn set_type_data(&mut self, entry: ModuleEntry) {
        let key = entry.module_type();
        match key.and_then(|k| self.type_index.get(&k).copied()) {
            Some(i) => self.type_entries[i] = entry,
            None => {
                if let Some(k) = key {
                    self.type_index.insert(k, self.type_entries.len());
                }
                self.type_entries.push(entry);
            }
        }
    }

    /// Removes an instance-level entry, as `RemoveModuleInstanceData` does.
    pub fn remove_instance(
        &mut self,
        type_id: impl Borrow<Uuid>,
        instance_id: impl Borrow<Uuid>,
    ) -> Option<ModuleEntry> {
        let i = self.instance_index.remove(&(*type_id.borrow(), *instance_id.borrow()))?;
        let entry = self.instance_entries.remove(i);
        self.rebuild_instance_index();
        Some(entry)
    }

    /// Removes many instance-level entries in one pass. `remove_instance` rebuilds the index each time, which is quadratic over a big store.
    pub fn remove_instances(&mut self, keys: &std::collections::HashSet<(Uuid, Uuid)>) {
        if keys.is_empty() {
            return;
        }
        self.instance_entries.retain(|e| !e.module_type().zip(e.module_instance()).is_some_and(|k| keys.contains(&k)));
        self.rebuild_instance_index();
    }

    pub fn type_entries(&self) -> &[ModuleEntry] {
        &self.type_entries
    }

    pub fn instance_entries(&self) -> &[ModuleEntry] {
        &self.instance_entries
    }

    fn rebuild_instance_index(&mut self) {
        self.instance_index.clear();
        for (i, e) in self.instance_entries.iter().enumerate() {
            if let Some(k) = e.module_type().zip(e.module_instance()) {
                self.instance_index.entry(k).or_insert(i);
            }
        }
    }
}

/// Reads `ModuleStore.xml`, migrating in memory. See `load_sequence` for why migrating does not write.
pub fn load_module_store_file(path: &Path) -> Result<ModuleStore, IoError> {
    let root = read_xml_file(path)?;
    module_store_from_xml(root)
}

/// Writes `ModuleStore.xml` through `XElementFileWriter`'s backup rules, atomically.
pub fn save_module_store(store: &ModuleStore, paths: &crate::DataPaths) -> Result<(), IoError> {
    let text = write_document(&module_store_to_xml(store), MODULE_STORE_STYLE);
    let sequence_backups = paths.sequence_backup_dir();
    file::save_with_backups(
        &paths.module_store_file(),
        text.as_bytes(),
        CURRENT_MODULE_STORE_VERSION,
        None,
        Some(&sequence_backups),
    )
}

pub(crate) fn module_store_from_xml(mut root: XElement) -> Result<ModuleStore, IoError> {
    if root.local != "ModuleStore" {
        return Err(IoError::format("module store", format!("the root element is {}, not ModuleStore", root.local)));
    }
    let source_version = root.attr("version").and_then(|v| v.parse().ok()).unwrap_or(-1);
    let version = migrate(&mut root, source_version)?;

    let mut store = ModuleStore { version, source_version, ..ModuleStore::default() };
    store.root_attrs = root.attrs.iter().filter(|a| a.local != "version").cloned().collect();
    let mut sections = root.children.into_iter().filter_map(|n| match n {
        XNode::Element(e) => Some(e),
        XNode::Text(_) => None,
    });
    // `_GetTypeData` takes the first element and `_GetInstanceData` the second, whatever they are called.
    if let Some(section) = sections.next() {
        for module in section.children.into_iter() {
            if let XNode::Element(element) = module {
                store.set_type_entry_from_file(ModuleEntry { element });
            }
        }
    }
    if let Some(section) = sections.next() {
        for module in section.children.into_iter() {
            if let XNode::Element(element) = module {
                store.set_instance_entry_from_file(ModuleEntry { element });
            }
        }
    }
    store.extra_sections = sections.collect();
    Ok(store)
}

impl ModuleStore {
    /// A repeated key keeps both entries in the file's order, and lookups find the first.
    fn set_type_entry_from_file(&mut self, entry: ModuleEntry) {
        if let Some(k) = entry.module_type() {
            self.type_index.entry(k).or_insert(self.type_entries.len());
        }
        self.type_entries.push(entry);
    }

    fn set_instance_entry_from_file(&mut self, entry: ModuleEntry) {
        if let Some(k) = entry.module_type().zip(entry.module_instance()) {
            self.instance_index.entry(k).or_insert(self.instance_entries.len());
        }
        self.instance_entries.push(entry);
    }
}

pub(crate) fn module_store_to_xml(store: &ModuleStore) -> XElement {
    let mut root = XElement::new("", "ModuleStore");
    root.set_attr("version", store.version.to_string());
    root.attrs.extend(store.root_attrs.iter().cloned());
    let section = |entries: &[ModuleEntry]| {
        let mut s = XElement::new("", "ModuleData");
        for e in entries {
            s.push(e.element.clone());
        }
        s
    };
    root.push(section(&store.type_entries));
    root.push(section(&store.instance_entries));
    for s in &store.extra_sections {
        root.push(s.clone());
    }
    root
}

fn migrate(root: &mut XElement, from: i32) -> Result<i32, IoError> {
    if from >= CURRENT_MODULE_STORE_VERSION {
        return Ok(from);
    }
    if from < 1 {
        return Err(IoError::Migration {
            what: "module store".to_string(),
            from,
            to: CURRENT_MODULE_STORE_VERSION,
            message: "the file requires a migration, but a proper migration path is not available".to_string(),
        });
    }
    for version in from..CURRENT_MODULE_STORE_VERSION {
        match version {
            1 => version_1_to_2(root),
            2 => version_2_to_3(root),
            3 => version_3_to_4(root),
            // "Since there is no impact upgrading from 4 to 5, this simply returns the original content."
            _ => {}
        }
    }
    root.set_attr("version", CURRENT_MODULE_STORE_VERSION.to_string());
    Ok(CURRENT_MODULE_STORE_VERSION)
}

fn modules_of<'a>(root: &'a mut XElement, data_model_type: &'a str) -> impl Iterator<Item = &'a mut XElement> {
    root.elements_mut()
        .filter(|s| s.local == "ModuleData")
        .flat_map(|s| s.elements_mut())
        .filter(move |m| m.attr(ATTR_DATA_MODEL_TYPE) == Some(data_model_type))
}

/// Preview background images become file names relative to the profile.
fn version_1_to_2(root: &mut XElement) {
    for module in modules_of(root, "VixenModules.Preview.VixenPreview.VixenPreviewData, VixenPreview") {
        if let Some(name) = module.child_mut("VixenPreviewData").and_then(|d| d.child_mut("BackgroundFileName")) {
            let file = windows_file_name(&name.text()).to_string();
            name.children = vec![XNode::Text(file)];
        }
    }
}

/// Show items' sequence paths become relative to the sequence folder. The C# copies a sequence kept elsewhere
/// into the profile and asks the user about conflicts. Loading does not write and the paths are Windows ones,
/// so here every path becomes its file name, which is where the C# leaves a sequence it could resolve.
fn version_2_to_3(root: &mut XElement) {
    for module in modules_of(root, "VixenModules.App.Shows.ShowsData, Shows") {
        let items = module
            .child_mut("ShowsData")
            .and_then(|d| d.child_mut("Shows"))
            .into_iter()
            .flat_map(|shows| shows.elements_mut())
            .filter(|s| s.local == "Show")
            .filter_map(|show| show.child_mut("Items"))
            .flat_map(|items| items.elements_mut())
            .filter(|i| i.local == "ShowItem");
        for item in items {
            if let Some(name) = item.child_mut("Sequence_FileName") {
                let value = name.text();
                if !value.is_empty() {
                    name.children = vec![XNode::Text(windows_file_name(&value).to_string())];
                }
            }
        }
    }
}

/// LipSync matrix drawings move from the map to bitmap files. Only the XML side is done: the C# paints each
/// phoneme's colours into a bitmap under `Module Data Files/LipSync`, which this port does not write on load.
/// The drawings stay in the `.3` backup that saving makes.
fn version_3_to_4(root: &mut XElement) {
    for module in modules_of(root, "VixenModules.App.LipSyncApp.LipSyncMapStaticData, LipSyncApp") {
        let Some(library) = module.child_mut("LipSyncMapStaticData").and_then(|d| d.child_mut("_library")) else {
            continue;
        };
        for map in library.elements_mut() {
            let Some(value) = map.elements_mut().find(|e| e.local.contains("Value")) else { continue };
            let is_matrix = child_string(value, "IsMatrix").as_deref() == Some("true");
            remove_child(value, "ZoomLevel");
            let per_string: i32 =
                child_string(value, "MatrixPixelsPerString").and_then(|t| t.trim().parse().ok()).unwrap_or(0);
            let strings: i32 =
                child_string(value, "MatrixStringCount").and_then(|t| t.trim().parse().ok()).unwrap_or(0);
            remove_child(value, "MatrixPixelsPerString");
            remove_child(value, "MatrixStringCount");
            if is_matrix && strings > 0 && per_string > 0 {
                if let Some(items) = value.child_mut("MapItems") {
                    items.children.clear();
                }
            } else if let Some(items) = value.child_mut("MapItems") {
                for item in items.elements_mut() {
                    remove_child(item, "ElementColor");
                }
            }
            let ns = value.ns.clone();
            crate::dcs::set_member(value, XElement::with_text(&ns, "Notes", ""));
        }
    }
}

fn remove_child(e: &mut XElement, local: &str) {
    e.children.retain(|n| !matches!(n, XNode::Element(c) if c.local == local));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::same_tree;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<ModuleStore version="5">
  <ModuleData>
    <Module dataModelType="VixenModules.Property.Location.LocationData, Location" moduleType="3fb53423-dd2a-4719-b3e3-19aa6f062f62" moduleInstance="3fb53423-dd2a-4719-b3e3-19aa6f062f62" assemblyName="Module.Property.Location">
      <LocationData xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Property.Location" xmlns:i="http://www.w3.org/2001/XMLSchema-instance">
        <ModuleInstanceId xmlns="">3fb53423-dd2a-4719-b3e3-19aa6f062f62</ModuleInstanceId>
        <ModuleTypeId xmlns="">3fb53423-dd2a-4719-b3e3-19aa6f062f62</ModuleTypeId>
        <X>0</X>
      </LocationData>
    </Module>
  </ModuleData>
  <ModuleData>
    <Module dataModelType="VixenModules.Property.Location.LocationData, Location" moduleType="3fb53423-dd2a-4719-b3e3-19aa6f062f62" moduleInstance="a70d971c-4705-41ad-8f5a-c4a1fc43db4a" assemblyName="Module.Property.Location">
      <LocationData xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Property.Location" xmlns:i="http://www.w3.org/2001/XMLSchema-instance">
        <ModuleInstanceId xmlns="">a70d971c-4705-41ad-8f5a-c4a1fc43db4a</ModuleInstanceId>
        <ModuleTypeId xmlns="">3fb53423-dd2a-4719-b3e3-19aa6f062f62</ModuleTypeId>
        <X>565</X>
      </LocationData>
    </Module>
  </ModuleData>
</ModuleStore>"#;

    fn location_type() -> Uuid {
        parse_guid("3fb53423-dd2a-4719-b3e3-19aa6f062f62").unwrap()
    }

    #[test]
    fn entries_are_found_by_type_and_by_instance() {
        let store = module_store_from_xml(XElement::parse(SAMPLE).unwrap()).unwrap();
        let t = location_type();
        let instance = parse_guid("a70d971c-4705-41ad-8f5a-c4a1fc43db4a").unwrap();
        assert_eq!(store.type_data(t).unwrap().data().unwrap().child_text("X").as_deref(), Some("0"));
        let e = store.instance(t, instance).unwrap();
        assert_eq!(e.data().unwrap().child_text("X").as_deref(), Some("565"));
        assert_eq!(e.assembly_name(), Some("Module.Property.Location"));
        assert_eq!(e.data_model_type(), Some("VixenModules.Property.Location.LocationData, Location"));
        // References work as well as values.
        let (type_ref, instance_ref) = (&t, &instance);
        assert!(store.instance(type_ref, instance_ref).is_some());
        assert!(store.instance(instance, t).is_none());
    }

    #[test]
    fn the_store_round_trips_unchanged() {
        let root = XElement::parse(SAMPLE).unwrap();
        let store = module_store_from_xml(root.clone()).unwrap();
        let out = module_store_to_xml(&store);
        assert!(same_tree(&root, &out), "{:?}", root.first_difference(&out, String::new()));
        let text = write_document(&out, MODULE_STORE_STYLE);
        assert!(same_tree(&root, &XElement::parse(&text).unwrap()));
        assert!(text.contains("<LocationData xmlns=\"http://schemas.datacontract.org/2004/07/VixenModules.Property.Location\" xmlns:i=\"http://www.w3.org/2001/XMLSchema-instance\">"), "{text}");
    }

    #[test]
    fn setters_add_and_replace_in_place() {
        let mut store = module_store_from_xml(XElement::parse(SAMPLE).unwrap()).unwrap();
        let t = location_type();
        let instance = Uuid::from_u128(7);
        let entry = |x: &str| ModuleEntry::new("T, A", &t, &instance, "Module.A", XElement::with_text("", "Data", x));
        store.set_instance(entry("1"));
        assert_eq!(store.instance_entries().len(), 2);
        store.set_instance(entry("2"));
        assert_eq!(store.instance_entries().len(), 2);
        assert_eq!(store.instance(t, instance).unwrap().data().unwrap().text(), "2");

        let mut replacement = store.type_data(t).unwrap().clone();
        replacement.set_data(XElement::with_text("", "Data", "t"));
        store.set_type_data(replacement);
        assert_eq!(store.type_entries().len(), 1);
        assert_eq!(store.type_data(t).unwrap().data().unwrap().text(), "t");

        assert!(store.remove_instance(t, instance).is_some());
        assert!(store.instance(t, instance).is_none());
        assert_eq!(store.instance_entries().len(), 1);
    }

    #[test]
    fn version_4_migrates_with_no_change() {
        let root = XElement::parse(&SAMPLE.replace("version=\"5\"", "version=\"4\"")).unwrap();
        let store = module_store_from_xml(root).unwrap();
        assert_eq!(store.version, 5);
        assert_eq!(store.source_version, 4);
    }

    #[test]
    fn version_1_preview_backgrounds_become_file_names() {
        let text = r#"<ModuleStore version="1"><ModuleData><Module dataModelType="VixenModules.Preview.VixenPreview.VixenPreviewData, VixenPreview" moduleType="f43c31bc-cfcd-4dbc-adb7-d349c9ec623d" moduleInstance="f43c31bc-cfcd-4dbc-adb7-d349c9ec623d"><VixenPreviewData xmlns="urn:p"><BackgroundFileName>C:\Users\me\yard.jpg</BackgroundFileName></VixenPreviewData></Module></ModuleData></ModuleStore>"#;
        let store = module_store_from_xml(XElement::parse(text).unwrap()).unwrap();
        let t = parse_guid("f43c31bc-cfcd-4dbc-adb7-d349c9ec623d").unwrap();
        assert_eq!(
            store.type_data(t).unwrap().data().unwrap().child_text("BackgroundFileName").as_deref(),
            Some("yard.jpg")
        );
    }

    #[test]
    fn version_2_show_items_become_relative() {
        let text = r#"<ModuleStore version="2"><ModuleData><Module dataModelType="VixenModules.App.Shows.ShowsData, Shows" moduleType="f43c31bc-cfcd-4dbc-adb7-d349c9ec623d" moduleInstance="f43c31bc-cfcd-4dbc-adb7-d349c9ec623d"><ShowsData xmlns="urn:s"><Shows><Show><Items><ShowItem><Sequence_FileName>C:\Vixen 3\Sequence\Song.tim</Sequence_FileName></ShowItem></Items></Show></Shows></ShowsData></Module></ModuleData></ModuleStore>"#;
        let store = module_store_from_xml(XElement::parse(text).unwrap()).unwrap();
        let t = parse_guid("f43c31bc-cfcd-4dbc-adb7-d349c9ec623d").unwrap();
        let data = store.type_data(t).unwrap().data().unwrap();
        let item =
            data.child("Shows").unwrap().child("Show").unwrap().child("Items").unwrap().child("ShowItem").unwrap();
        assert_eq!(item.child_text("Sequence_FileName").as_deref(), Some("Song.tim"));
    }

    #[test]
    fn version_3_removes_the_matrix_members() {
        let text = r#"<ModuleStore version="3"><ModuleData><Module dataModelType="VixenModules.App.LipSyncApp.LipSyncMapStaticData, LipSyncApp" moduleType="a475c535-73fa-4d31-a1bc-ed2b50ad21cf" moduleInstance="a475c535-73fa-4d31-a1bc-ed2b50ad21cf"><LipSyncMapStaticData xmlns="urn:l"><_library><KeyValue><Key>M</Key><Value><IsMatrix>true</IsMatrix><MapItems><Item><ElementColor/></Item></MapItems><MatrixPixelsPerString>2</MatrixPixelsPerString><MatrixStringCount>2</MatrixStringCount><ZoomLevel>1</ZoomLevel></Value></KeyValue></_library></LipSyncMapStaticData></Module></ModuleData></ModuleStore>"#;
        let store = module_store_from_xml(XElement::parse(text).unwrap()).unwrap();
        let t = parse_guid("a475c535-73fa-4d31-a1bc-ed2b50ad21cf").unwrap();
        let value = store
            .type_data(t)
            .unwrap()
            .data()
            .unwrap()
            .child("_library")
            .unwrap()
            .elements()
            .next()
            .unwrap()
            .child("Value")
            .unwrap();
        assert!(value.child("ZoomLevel").is_none());
        assert!(value.child("MatrixStringCount").is_none());
        assert_eq!(value.child("MapItems").unwrap().elements().count(), 0);
        assert!(value.child("Notes").is_some());
    }
}
