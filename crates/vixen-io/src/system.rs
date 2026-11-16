//! `SystemConfig.xml` joined to the module store. Port of `XmlSystemConfigFilePolicy`, the `Xml*Serializer`s it
//! uses and `SystemConfigXElementMigrator`.
//!
//! The element tree in the file names each property's module but keeps its data in the module store, so loading
//! resolves the common property modules into typed `PropertyData`. Saving writes back any property that changed.

use crate::dcs::{child_guid, child_i32, child_value, format_bool, format_guid, parse_bool, parse_guid};
use crate::error::IoError;
use crate::fidelity::{diff, reapply, Residue};
use crate::file;
use crate::library::{color_sets, set_color_sets};
use crate::module_store::{load_module_store_file, module_store_to_xml, ModuleEntry, ModuleStore, MODULE_STORE_STYLE};
use crate::paths::DataPaths;
use crate::values::{color_from_xml, color_to_xml};
use crate::xml::{read_xml_file, write_document, write_fragment, DocStyle};
use std::borrow::Cow;
use std::collections::HashMap;
use uuid::{uuid, Uuid};
use vixen_core::element::{
    property_ids, ElementColorType, ElementTree, NodeId, Orientation, PropertyData, PropertyInstance,
};
use vixen_core::Color;
use vixen_xml::{XElement, XNode};

/// `ObjectVersion.SystemConfig`.
pub const CURRENT_SYSTEM_CONFIG_VERSION: i32 = 16;
/// `SystemConfig.DEFAULT_UPDATE_INTERVAL`.
pub const DEFAULT_UPDATE_INTERVAL: i32 = 50;
/// `OrderDescriptor.Id`, which vixen-core's `property_ids` does not list.
pub const ORDER_PROPERTY_TYPE_ID: Uuid = uuid!("47d4a43b-5407-44bd-8be1-4974b148504f");

const NS_COLOR: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Property.Color";
const NS_LOCATION: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Property.Location";
const NS_ORIENTATION: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Property.Orientation";
const NS_ORDER: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Property.Order";

/// An `Element`: what a leaf node's `channelId` refers to and what patches connect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
}

/// An `OutputController`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Controller {
    pub name: String,
    /// `hardwareId`: the controller module's type.
    pub hardware_type_id: Uuid,
    /// `hardwareInstanceId`: its data in the module store.
    pub hardware_instance_id: Uuid,
    pub id: Uuid,
    pub outputs: Vec<ControllerOutput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerOutput {
    pub name: String,
    pub id: Uuid,
}

/// An `OutputPreview`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    pub name: String,
    pub type_id: Uuid,
    pub instance_id: Uuid,
}

/// An output filter module, a `FilterNode`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OutputFilter {
    pub type_id: Uuid,
    pub instance_id: Uuid,
}

/// A `DataFlowPatch`: `source_id`'s output `output` feeds `component_id`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Patch {
    pub component_id: Uuid,
    pub source_id: Option<Uuid>,
    pub output: i32,
}

/// An `ElementTagDefinition`. Files from before tags existed have none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagDefinition {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub is_built_in: bool,
    pub sort_order: i32,
    pub description: Option<String>,
    pub display_color: Option<String>,
}

/// A profile's system configuration and module data.
#[derive(Clone, Debug)]
pub struct SystemData {
    /// The root's `version`, after migration.
    pub version: i32,
    /// The version on disk before migrating.
    pub source_version: i32,
    /// `isContext`: the file is a system context, not the main configuration.
    pub is_context: bool,
    pub identity: Uuid,
    pub allow_filter_evaluation: bool,
    /// Milliseconds.
    pub default_update_interval: i32,
    pub clear_effect_cache_on_exit: bool,
    pub video_effect_cache_file_type: String,
    /// `Channels`, in file order. A node's name and its element's name can differ.
    pub channels: Vec<Channel>,
    /// The nodes, with properties resolved from the module store and colour sets from `ColorStaticData`.
    pub tree: ElementTree,
    pub controllers: Vec<Controller>,
    pub previews: Vec<Preview>,
    pub filters: Vec<OutputFilter>,
    pub tags: Vec<TagDefinition>,
    pub patches: Vec<Patch>,
    pub disabled_devices: Vec<Uuid>,
    pub module_store: ModuleStore,
    residue: Option<Residue>,
}

impl Default for SystemData {
    /// `new SystemConfig()` with an empty module store, for a profile with no files yet.
    fn default() -> SystemData {
        SystemData {
            version: CURRENT_SYSTEM_CONFIG_VERSION,
            source_version: CURRENT_SYSTEM_CONFIG_VERSION,
            is_context: false,
            identity: Uuid::new_v4(),
            allow_filter_evaluation: true,
            default_update_interval: DEFAULT_UPDATE_INTERVAL,
            clear_effect_cache_on_exit: true,
            video_effect_cache_file_type: "bmp".to_string(),
            channels: Vec::new(),
            tree: ElementTree::new(),
            controllers: Vec::new(),
            previews: Vec::new(),
            filters: Vec::new(),
            tags: Vec::new(),
            patches: Vec::new(),
            disabled_devices: Vec::new(),
            module_store: ModuleStore::new(),
            residue: None,
        }
    }
}

impl SystemData {
    /// The root members typed encoding did not reproduce, for reports and tests.
    pub fn residue_members(&self) -> Vec<String> {
        match &self.residue {
            None => Vec::new(),
            Some(Residue::Whole { .. }) => vec!["<whole document>".to_string()],
            Some(Residue::Members(m)) => m.iter().map(|m| m.name().to_string()).collect(),
        }
    }

    /// The channel with this id.
    pub fn channel(&self, id: &Uuid) -> Option<&Channel> {
        self.channels.iter().find(|c| &c.id == id)
    }
}

/// Reads the module store and then the system configuration, as `VixenSystem.LoadSystemConfig` does. A profile
/// with neither file gets a new configuration. Never writes: see `load_sequence` for how migration backups work.
pub fn load_system(paths: &DataPaths) -> Result<SystemData, IoError> {
    load_system_with(paths, || {})
}

/// [`load_system`], calling `module_store_loaded` between the two files, for a progress report.
pub fn load_system_with(paths: &DataPaths, module_store_loaded: impl FnOnce()) -> Result<SystemData, IoError> {
    let store_path = paths.module_store_file();
    let store = if store_path.is_file() { load_module_store_file(&store_path)? } else { ModuleStore::new() };
    module_store_loaded();
    let config_path = paths.system_config_file();
    if !config_path.is_file() {
        let mut data = SystemData { module_store: store, ..SystemData::default() };
        apply_color_sets(&mut data.tree, &data.module_store);
        return Ok(data);
    }
    load_system_config_file(&config_path, store)
}

/// Reads a `SystemConfig.xml` from anywhere, such as a backup, against a module store already loaded.
pub fn load_system_config_file(path: &std::path::Path, store: ModuleStore) -> Result<SystemData, IoError> {
    let root = read_xml_file(path)?;
    system_from_xml(root, store).map_err(|e| match e {
        IoError::Format { message, .. } => IoError::Format { context: path.display().to_string(), message },
        other => other,
    })
}

/// Writes both system files through `XElementFileWriter`'s backup rules, each atomically. Properties and colour
/// sets that changed in the tree go into the module store first.
pub fn save_system(data: &SystemData, paths: &DataPaths) -> Result<(), IoError> {
    save_module_config_file(data, paths)?;
    save_system_config_file(data, paths)
}

/// `SaveModuleConfigAsync`'s file alone, with changed properties and colour sets applied.
pub fn save_module_config_file(data: &SystemData, paths: &DataPaths) -> Result<(), IoError> {
    let store = effective_store(data);
    let store_text = write_document(&module_store_to_xml(&store), MODULE_STORE_STYLE);
    file::save_with_backups(
        &paths.module_store_file(),
        store_text.as_bytes(),
        crate::module_store::CURRENT_MODULE_STORE_VERSION,
        None,
        Some(&paths.sequence_backup_dir()),
    )
}

/// `SaveSystemConfigAsync`'s file alone.
pub fn save_system_config_file(data: &SystemData, paths: &DataPaths) -> Result<(), IoError> {
    let config_text = write_document(&system_config_xml(data), DocStyle::default());
    file::save_with_backups(
        &paths.system_config_file(),
        config_text.as_bytes(),
        CURRENT_SYSTEM_CONFIG_VERSION,
        None,
        Some(&paths.sequence_backup_dir()),
    )
}

/// The `SystemConfig` element a save would write.
pub fn system_config_xml(data: &SystemData) -> XElement {
    reapply(encode_config(data), data.residue.as_ref())
}

/// The module store a save would write, with changed properties and colour sets applied.
pub fn effective_store(data: &SystemData) -> Cow<'_, ModuleStore> {
    let mut updates: Vec<&PropertyInstance> = Vec::new();
    for (_, node) in data.tree.all_nodes() {
        for prop in &node.properties {
            let entry = data.module_store.instance(prop.type_id, prop.instance_id);
            if decode_property(&prop.type_id, entry) != prop.data {
                updates.push(prop);
            }
        }
    }
    let current_sets: HashMap<String, Vec<Color>> = color_sets(&data.module_store).into_iter().collect();
    let sets_changed = current_sets != data.tree.color_sets;
    if updates.is_empty() && !sets_changed {
        return Cow::Borrowed(&data.module_store);
    }
    let mut store = data.module_store.clone();
    for prop in updates {
        write_property(&mut store, prop);
    }
    if sets_changed {
        // Sets keep the file's order, and new ones follow by name so saves are repeatable.
        let existing = color_sets(&store);
        let mut sets: Vec<(String, Vec<Color>)> = existing
            .iter()
            .filter_map(|(name, _)| data.tree.color_sets.get(name).map(|c| (name.clone(), c.clone())))
            .collect();
        let mut added: Vec<_> =
            data.tree.color_sets.iter().filter(|(n, _)| !existing.iter().any(|(e, _)| e == *n)).collect();
        added.sort_by(|a, b| a.0.cmp(b.0));
        sets.extend(added.into_iter().map(|(n, c)| (n.clone(), c.clone())));
        set_color_sets(&mut store, &sets);
    }
    Cow::Owned(store)
}

/// Gives every property in the tree a store entry, as `Properties.Add` does for a new one. A default value otherwise looks unchanged to `effective_store`, and would stay out of the file.
/// `skip` names instances whose entry was dropped on purpose and stays out.
pub fn ensure_property_entries(data: &mut SystemData, skip: &std::collections::HashSet<Uuid>) {
    let missing: Vec<PropertyInstance> = data
        .tree
        .all_nodes()
        .flat_map(|(_, n)| n.properties.iter())
        .filter(|p| !skip.contains(&p.instance_id) && data.module_store.instance(p.type_id, p.instance_id).is_none())
        .cloned()
        .collect();
    for prop in &missing {
        write_property(&mut data.module_store, prop);
    }
}

/// Decodes a parsed `SystemConfig` root, migrating it first.
pub(crate) fn system_from_xml(mut root: XElement, store: ModuleStore) -> Result<SystemData, IoError> {
    if root.local != "SystemConfig" {
        return Err(IoError::format("system config", format!("the root element is {}, not SystemConfig", root.local)));
    }
    let source_version = root.attr("version").and_then(|v| v.parse().ok()).unwrap_or(-1);
    migrate(&mut root, source_version, &store)?;
    let version = root.attr("version").and_then(|v| v.parse().ok()).unwrap_or(CURRENT_SYSTEM_CONFIG_VERSION);

    let mut tree = ElementTree::new();
    apply_color_sets(&mut tree, &store);
    if let Some(nodes) = root.child("Nodes") {
        let mut path = Vec::new();
        let mut roots = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for n in nodes.elements() {
            if let Some(node) = read_node(n, &mut tree, &store, &mut path) {
                if seen.insert(node) {
                    roots.push(node);
                }
            }
        }
        // `Nodes.AddNodes`: a node listed at the top stays there even if a group holds it too, which `add_child` would have undone.
        tree.roots = roots;
    }

    let mut data = SystemData {
        version,
        source_version,
        is_context: root.attr("isContext").is_some(),
        // `new SystemConfig()` makes an identity when the file has none.
        identity: child_guid(&root, "Identity").unwrap_or_else(Uuid::new_v4),
        // "If it can't be determined, default to true."
        allow_filter_evaluation: child_value(&root, "AllowFilterEvaluation")
            .and_then(|t| parse_bool(&t))
            .unwrap_or(true),
        default_update_interval: child_i32(&root, "DefaultUpdateInterval").unwrap_or(DEFAULT_UPDATE_INTERVAL),
        clear_effect_cache_on_exit: child_value(&root, "ClearEffectCacheOnExit")
            .and_then(|t| parse_bool(&t))
            .unwrap_or(true),
        video_effect_cache_file_type: root
            .child("VideoEffectCacheFileType")
            .map_or_else(|| "bmp".to_string(), |e| e.text()),
        channels: read_channels(&root),
        tree,
        controllers: read_controllers(&root),
        previews: read_previews(&root),
        filters: read_filters(&root),
        tags: read_tags(&root),
        patches: read_patches(&root),
        disabled_devices: root
            .child("DisabledDevices")
            .map(|d| d.elements().filter_map(|e| e.attr("id").and_then(parse_guid)).collect())
            .unwrap_or_default(),
        module_store: store,
        residue: None,
    };
    data.residue = diff(&root, &encode_config(&data));
    Ok(data)
}

fn apply_color_sets(tree: &mut ElementTree, store: &ModuleStore) {
    tree.color_sets = color_sets(store).into_iter().collect();
}

// ---- Reading ----------------------------------------------------------------------------------------------

fn attr_guid(e: &XElement, name: &str) -> Option<Uuid> {
    e.attr(name).and_then(parse_guid)
}

fn read_channels(root: &XElement) -> Vec<Channel> {
    let Some(list) = root.child("Channels") else { return Vec::new() };
    list.elements()
        .filter_map(|e| Some(Channel { id: attr_guid(e, "id")?, name: e.attr("name").unwrap_or_default().to_string() }))
        .collect()
}

/// `XmlElementNodeSerializer.ReadObject`. A node listed twice, under two groups, is one node: the first listing
/// creates it. Unlike the C#, a leaf naming a channel the file does not have stays in the tree, because pruning
/// it would lose the node on the next save.
fn read_node(e: &XElement, tree: &mut ElementTree, store: &ModuleStore, path: &mut Vec<Uuid>) -> Option<NodeId> {
    let name = e.attr("name")?;
    let id = attr_guid(e, "id")?;
    if let Some(existing) = tree.find(&id) {
        return Some(existing);
    }
    let element_id = attr_guid(e, "channelId");
    let node = tree.add_node(id, name, element_id);
    if element_id.is_none() {
        path.push(id);
        let mut children = Vec::new();
        for child in e.children_named("Node") {
            // A node inside itself could only come from a damaged file, and following it would never end.
            let is_cycle = attr_guid(child, "id").is_some_and(|c| path.contains(&c));
            if !is_cycle {
                children.extend(read_node(child, tree, store, path));
            }
        }
        path.pop();
        // The `GroupNode` constructor links its children once all of them are read, so a child that a deeper group
        // also holds gets that group as its first parent.
        for c in children {
            tree.add_child(node, c);
        }
    }
    if let Some(list) = e.child("Properties") {
        for p in list.children_named("Property") {
            if let (Some(type_id), Some(instance_id)) = (attr_guid(p, "typeId"), attr_guid(p, "instanceId")) {
                let data = decode_property(&type_id, store.instance(type_id, instance_id));
                tree.node_mut(node).properties.push(PropertyInstance { type_id, instance_id, data });
            }
        }
    }
    if let Some(list) = e.child("Tags") {
        for t in list.children_named("Tag") {
            if let Some(tag) = attr_guid(t, "id") {
                tree.node_mut(node).tags.push(tag);
            }
        }
    }
    Some(node)
}

/// A property's typed data. With no store entry, these are the module's `new XxxData()`, what
/// `AssignModuleInstanceData` creates. Anything else keeps the entry's XML.
pub fn decode_property(type_id: &Uuid, entry: Option<&ModuleEntry>) -> PropertyData {
    let data = entry.and_then(|e| e.data());
    match *type_id {
        property_ids::COLOR => match data {
            Some(d) => decode_color(d),
            None => PropertyData::Color {
                color_type: ElementColorType::FullColor,
                single_color: Color::EMPTY,
                color_set_name: None,
            },
        },
        property_ids::LOCATION => match data {
            Some(d) => decode_location(d),
            None => PropertyData::Location { x: 0, y: 0, z: 0 },
        },
        property_ids::ORIENTATION => match data {
            Some(d) => decode_orientation(d),
            // `Orientation` is `default(T)` here, which is Horizontal. Only a new property gets Vertical.
            None => PropertyData::Orientation(Orientation::Horizontal),
        },
        ORDER_PROPERTY_TYPE_ID => match data {
            Some(d) => PropertyData::Order { order: child_i32(d, "Order").unwrap_or(0) },
            None => PropertyData::Order { order: 0 },
        },
        _ => PropertyData::Unknown { raw_xml: data.map(write_fragment).unwrap_or_default() },
    }
}

/// `ColorData`, including its `[OnDeserialized]`.
fn decode_color(d: &XElement) -> PropertyData {
    let color_type = match child_value(d, "ElementColorType").as_deref() {
        Some("MultipleDiscreteColors") => ElementColorType::MultipleDiscreteColors,
        Some("FullColor") => ElementColorType::FullColor,
        // Member 0 of the enum, which is also what a missing member reads as.
        _ => ElementColorType::SingleColor,
    };
    let single_color = if color_type == ElementColorType::FullColor {
        Color::EMPTY
    } else {
        d.child("SingleColor").and_then(color_from_xml).unwrap_or(Color::EMPTY)
    };
    let color_set_name = crate::dcs::child_string(d, "ColorSetName").filter(|s| !s.is_empty());
    PropertyData::Color { color_type, single_color, color_set_name }
}

/// `LocationData`, including its `[OnDeserialized]`, which clamps negative values.
fn decode_location(d: &XElement) -> PropertyData {
    let field = |n: &str| child_i32(d, n).unwrap_or(0).max(0);
    PropertyData::Location { x: field("X"), y: field("Y"), z: field("Z") }
}

fn decode_orientation(d: &XElement) -> PropertyData {
    match child_value(d, "Orientation").as_deref() {
        Some("Vertical") => PropertyData::Orientation(Orientation::Vertical),
        Some("Horizontal") | None => PropertyData::Orientation(Orientation::Horizontal),
        Some(_) => PropertyData::Unknown { raw_xml: write_fragment(d) },
    }
}

fn read_controllers(root: &XElement) -> Vec<Controller> {
    let Some(list) = root.child("Controllers") else { return Vec::new() };
    list.elements()
        .filter_map(|e| {
            Some(Controller {
                name: e.attr("name")?.to_string(),
                hardware_type_id: attr_guid(e, "hardwareId")?,
                hardware_instance_id: attr_guid(e, "hardwareInstanceId")?,
                id: attr_guid(e, "id")?,
                outputs: e
                    .child("Outputs")
                    .map(|o| {
                        o.children_named("Output")
                            .map(|x| ControllerOutput {
                                name: x.attr("name").unwrap_or("Unnamed output").to_string(),
                                id: attr_guid(x, "id").unwrap_or_default(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn read_previews(root: &XElement) -> Vec<Preview> {
    let Some(list) = root.child("Previews") else { return Vec::new() };
    list.elements()
        .filter_map(|e| {
            Some(Preview {
                name: e.attr("name")?.to_string(),
                type_id: attr_guid(e, "typeId")?,
                instance_id: attr_guid(e, "instanceId")?,
            })
        })
        .collect()
}

fn read_filters(root: &XElement) -> Vec<OutputFilter> {
    let Some(list) = root.child("Filters") else { return Vec::new() };
    list.elements()
        .filter_map(|e| {
            Some(OutputFilter { type_id: attr_guid(e, "typeId")?, instance_id: attr_guid(e, "instanceId")? })
        })
        .collect()
}

fn read_tags(root: &XElement) -> Vec<TagDefinition> {
    let Some(list) = root.child("Tags") else { return Vec::new() };
    list.elements()
        .filter_map(|e| {
            Some(TagDefinition {
                id: attr_guid(e, "id")?,
                key: e.attr("key").unwrap_or_default().to_string(),
                name: e.attr("name").unwrap_or_default().to_string(),
                is_built_in: e.attr("isBuiltIn").and_then(parse_bool).unwrap_or(false),
                sort_order: e.attr("sortOrder").and_then(|v| v.parse().ok()).unwrap_or(0),
                description: e.attr("description").map(str::to_string),
                display_color: e.attr("displayColor").map(str::to_string),
            })
        })
        .collect()
}

fn read_patches(root: &XElement) -> Vec<Patch> {
    let Some(list) = root.child("Patches") else { return Vec::new() };
    list.elements()
        .filter_map(|e| {
            Some(Patch {
                component_id: attr_guid(e, "componentId")?,
                source_id: attr_guid(e, "sourceId"),
                output: e.attr("output").and_then(|v| v.parse().ok()).unwrap_or(0),
            })
        })
        .collect()
}

// ---- Writing ----------------------------------------------------------------------------------------------

fn encode_config(data: &SystemData) -> XElement {
    let mut root = XElement::new("", "SystemConfig");
    root.set_attr("version", data.version.to_string());
    if data.is_context {
        root.set_attr("isContext", "true");
    }
    root.push(XElement::with_text("", "Identity", format_guid(&data.identity)));
    root.push(XElement::with_text("", "AllowFilterEvaluation", format_bool(data.allow_filter_evaluation)));
    root.push(XElement::with_text("", "DefaultUpdateInterval", data.default_update_interval.to_string()));
    root.push(XElement::with_text("", "ClearEffectCacheOnExit", format_bool(data.clear_effect_cache_on_exit)));
    root.push(XElement::with_text("", "VideoEffectCacheFileType", data.video_effect_cache_file_type.as_str()));

    let mut channels = XElement::new("", "Channels");
    for c in &data.channels {
        let mut e = XElement::new("", "Channel");
        e.set_attr("id", format_guid(&c.id));
        e.set_attr("name", c.name.as_str());
        channels.push(e);
    }
    root.push(channels);

    let mut nodes = XElement::new("", "Nodes");
    for n in &data.tree.roots {
        nodes.push(node_to_xml(&data.tree, *n));
    }
    root.push(nodes);

    let mut controllers = XElement::new("", "Controllers");
    for c in &data.controllers {
        let mut e = XElement::new("", "Controller");
        e.set_attr("name", c.name.as_str());
        e.set_attr("hardwareId", format_guid(&c.hardware_type_id));
        e.set_attr("hardwareInstanceId", format_guid(&c.hardware_instance_id));
        e.set_attr("id", format_guid(&c.id));
        let mut outputs = XElement::new("", "Outputs");
        for o in &c.outputs {
            let mut x = XElement::new("", "Output");
            x.set_attr("name", o.name.as_str());
            x.set_attr("id", format_guid(&o.id));
            outputs.push(x);
        }
        e.push(outputs);
        controllers.push(e);
    }
    root.push(controllers);

    let mut previews = XElement::new("", "Previews");
    for p in &data.previews {
        let mut e = XElement::new("", "Preview");
        e.set_attr("name", p.name.as_str());
        e.set_attr("typeId", format_guid(&p.type_id));
        e.set_attr("instanceId", format_guid(&p.instance_id));
        previews.push(e);
    }
    root.push(previews);

    let mut filters = XElement::new("", "Filters");
    for f in &data.filters {
        let mut e = XElement::new("", "FilterNode");
        e.set_attr("typeId", format_guid(&f.type_id));
        e.set_attr("instanceId", format_guid(&f.instance_id));
        filters.push(e);
    }
    root.push(filters);

    let mut tags = XElement::new("", "Tags");
    for t in &data.tags {
        let mut e = XElement::new("", "Tag");
        e.set_attr("id", format_guid(&t.id));
        e.set_attr("key", t.key.as_str());
        e.set_attr("name", t.name.as_str());
        if t.is_built_in {
            e.set_attr("isBuiltIn", "true");
        }
        if let Some(d) = t.description.as_deref().filter(|d| !d.is_empty()) {
            e.set_attr("description", d);
        }
        if let Some(c) = t.display_color.as_deref().filter(|c| !c.is_empty()) {
            e.set_attr("displayColor", c);
        }
        if t.sort_order > 0 {
            e.set_attr("sortOrder", t.sort_order.to_string());
        }
        tags.push(e);
    }
    root.push(tags);

    let mut patches = XElement::new("", "Patches");
    for p in &data.patches {
        let mut e = XElement::new("", "Patch");
        e.set_attr("componentId", format_guid(&p.component_id));
        if let Some(source) = &p.source_id {
            e.set_attr("sourceId", format_guid(source));
            e.set_attr("output", p.output.to_string());
        }
        patches.push(e);
    }
    root.push(patches);

    let mut disabled = XElement::new("", "DisabledDevices");
    for id in &data.disabled_devices {
        let mut e = XElement::new("", "Device");
        e.set_attr("id", format_guid(id));
        disabled.push(e);
    }
    root.push(disabled);
    root
}

/// `XmlElementNodeSerializer.WriteObject`. A node under two groups is written out in full under both.
fn node_to_xml(tree: &ElementTree, id: NodeId) -> XElement {
    let n = tree.node(id);
    let mut e = XElement::new("", "Node");
    e.set_attr("name", n.name.as_str());
    e.set_attr("id", format_guid(&n.id));
    if n.children.is_empty() {
        if let Some(element) = &n.element_id {
            e.set_attr("channelId", format_guid(element));
        }
    }
    if !n.properties.is_empty() {
        let mut list = XElement::new("", "Properties");
        for p in &n.properties {
            let mut x = XElement::new("", "Property");
            x.set_attr("typeId", format_guid(&p.type_id));
            x.set_attr("instanceId", format_guid(&p.instance_id));
            list.push(x);
        }
        e.push(list);
    }
    if !n.tags.is_empty() {
        let mut list = XElement::new("", "Tags");
        for t in &n.tags {
            let mut x = XElement::new("", "Tag");
            x.set_attr("id", format_guid(t));
            list.push(x);
        }
        e.push(list);
    }
    for c in &n.children {
        e.push(node_to_xml(tree, *c));
    }
    e
}

/// Puts a changed property's data into the store, keeping whatever the old entry said beyond typed values.
fn write_property(store: &mut ModuleStore, prop: &PropertyInstance) {
    let new_plain = match encode_property(prop) {
        Some(e) => e,
        None => {
            // An `Unknown` property whose XML no longer parses, or was emptied: drop the entry.
            store.remove_instance(prop.type_id, prop.instance_id);
            return;
        }
    };
    match store.instance_mut(prop.type_id, prop.instance_id) {
        Some(entry) => {
            let merged = match entry.data() {
                Some(old) => match encode_property(&PropertyInstance {
                    type_id: prop.type_id,
                    instance_id: prop.instance_id,
                    data: decode_property(&prop.type_id, Some(entry)),
                }) {
                    Some(old_plain) => {
                        let residue = diff(old, &old_plain);
                        reapply(new_plain, residue.as_ref())
                    }
                    None => new_plain,
                },
                None => new_plain,
            };
            entry.set_data(merged);
        }
        None => {
            let (model, assembly) = match prop.data {
                PropertyData::Color { .. } => ("VixenModules.Property.Color.ColorData, Color", "Module.Property.Color"),
                PropertyData::Location { .. } => {
                    ("VixenModules.Property.Location.LocationData, Location", "Module.Property.Location")
                }
                PropertyData::Orientation(_) => {
                    ("VixenModules.Property.Orientation.OrientationData, Orientation", "Module.Property.Orientation")
                }
                PropertyData::Order { .. } => ("VixenModules.Property.Order.OrderData, Order", "Module.Property.Order"),
                PropertyData::Unknown { .. } => ("", ""),
            };
            store.set_instance(ModuleEntry::new(model, &prop.type_id, &prop.instance_id, assembly, new_plain));
        }
    }
}

fn encode_property(prop: &PropertyInstance) -> Option<XElement> {
    let id_members = |e: &mut XElement| {
        e.push(XElement::with_text("", "ModuleInstanceId", format_guid(&prop.instance_id)));
        e.push(XElement::with_text("", "ModuleTypeId", format_guid(&prop.type_id)));
    };
    Some(match &prop.data {
        PropertyData::Color { color_type, single_color, color_set_name } => {
            let mut e = XElement::new(NS_COLOR, "ColorData");
            id_members(&mut e);
            // `SingleColor` and `ColorSetName` are `EmitDefaultValue = false`.
            if let Some(name) = color_set_name {
                e.push(XElement::with_text(NS_COLOR, "ColorSetName", name.as_str()));
            }
            let type_text = match color_type {
                ElementColorType::SingleColor => "SingleColor",
                ElementColorType::MultipleDiscreteColors => "MultipleDiscreteColors",
                ElementColorType::FullColor => "FullColor",
            };
            e.push(XElement::with_text(NS_COLOR, "ElementColorType", type_text));
            if *single_color != Color::EMPTY {
                e.push(color_to_xml(NS_COLOR, "SingleColor", *single_color));
            }
            e
        }
        PropertyData::Location { x, y, z } => {
            let mut e = XElement::new(NS_LOCATION, "LocationData");
            id_members(&mut e);
            e.push(XElement::with_text(NS_LOCATION, "X", x.to_string()));
            e.push(XElement::with_text(NS_LOCATION, "Y", y.to_string()));
            e.push(XElement::with_text(NS_LOCATION, "Z", z.to_string()));
            e
        }
        PropertyData::Orientation(o) => {
            let mut e = XElement::new(NS_ORIENTATION, "OrientationData");
            id_members(&mut e);
            let text = match o {
                Orientation::Horizontal => "Horizontal",
                Orientation::Vertical => "Vertical",
            };
            e.push(XElement::with_text(NS_ORIENTATION, "Orientation", text));
            e
        }
        PropertyData::Order { order } => {
            let mut e = XElement::new(NS_ORDER, "OrderData");
            id_members(&mut e);
            e.push(XElement::with_text(NS_ORDER, "Order", order.to_string()));
            e
        }
        PropertyData::Unknown { raw_xml } => return crate::xml::parse_fragment(raw_xml),
    })
}

// ---- Migration: SystemConfigXElementMigrator -----------------------------------------------------------------

fn migrate(root: &mut XElement, from: i32, store: &ModuleStore) -> Result<(), IoError> {
    if from >= CURRENT_SYSTEM_CONFIG_VERSION {
        return Ok(());
    }
    if from < 1 {
        return Err(IoError::Migration {
            what: "system config".to_string(),
            from,
            to: CURRENT_SYSTEM_CONFIG_VERSION,
            message: "the file requires a migration, but a proper migration path is not available".to_string(),
        });
    }
    for version in from..CURRENT_SYSTEM_CONFIG_VERSION {
        match version {
            1 => {
                root.push(XElement::with_text("", "Identity", format_guid(&Uuid::new_v4())));
            }
            2 => {
                root.push(XElement::new("", "Controllers"));
            }
            4 => {
                root.push(XElement::new("", "DisabledControllers"));
            }
            5 => {
                root.push(XElement::new("", "ControllerLinks"));
            }
            6 => version_6_to_7(root),
            7 => {
                root.push(XElement::new("", "Previews"));
            }
            8 => {
                root.push(XElement::with_text("", "AllowFilterEvaluation", "true"));
            }
            9 => {
                root.push(XElement::with_text("", "AllowSubordinateEffects", "true"));
            }
            10 => {
                root.push(XElement::new("", "SmartControllers"));
            }
            11 => {
                add_device_instance_ids(root, "Controllers", "Controller", store);
                add_device_instance_ids(root, "SmartControllers", "SmartController", store);
                add_device_instance_ids(root, "Previews", "Preview", store);
            }
            12 => version_12_to_13(root),
            // 14 repeats 13 for "the full depth of nested nodes", and is written the same way.
            13 | 14 => reconnect_leaf_nodes(root),
            15 => {
                // Back to 50 ms if still at the old 46 ms default. Any other value was the user's choice.
                if let Some(e) = root.child_mut("DefaultUpdateInterval") {
                    if e.text() == "46" {
                        e.children = vec![XNode::Text("50".to_string())];
                    }
                }
            }
            // 3 to 4 changed nothing.
            _ => {}
        }
    }
    root.set_attr("version", CURRENT_SYSTEM_CONFIG_VERSION.to_string());
    Ok(())
}

fn version_6_to_7(root: &mut XElement) {
    let Some(controllers) = root.child_mut("Controllers") else { return };
    for controller in controllers.elements_mut().filter(|c| c.local == "Controller") {
        if let Some(outputs) = controller.child_mut("Outputs") {
            for output in outputs.elements_mut().filter(|o| o.local == "Output") {
                output.push(XElement::new("", "FilterNodes"));
            }
        }
    }
}

/// `_FixDeviceData_AddInstanceIds`. The C# pairs devices with stored instances by position, across all device
/// types, and accepts that a repeated type repeats its instances. A device past the end of that list would
/// throw there and blank the whole configuration. Here it just keeps no instance id.
fn add_device_instance_ids(root: &mut XElement, list: &str, item: &str, store: &ModuleStore) {
    let Some(devices) = root.child_mut(list) else { return };
    let type_ids: Vec<Uuid> = devices.children_named(item).filter_map(|d| attr_guid(d, "hardwareId")).collect();
    let instance_ids: Vec<Uuid> = type_ids
        .iter()
        .flat_map(|t| {
            store
                .instance_entries()
                .iter()
                .filter(move |e| e.module_type().as_ref() == Some(t))
                .filter_map(|e| e.module_instance())
        })
        .collect();
    let mut next = instance_ids.into_iter();
    for device in devices.elements_mut().filter(|d| d.local == item && d.attr("hardwareId").is_some()) {
        if let Some(id) = next.next() {
            device.set_attr("hardwareInstanceId", format_guid(&id));
        }
    }
}

/// "Changed the disabled controllers element to reflect disabled devices."
fn version_12_to_13(root: &mut XElement) {
    let mut disabled = XElement::new("", "DisabledDevices");
    if let Some(i) =
        root.children.iter().position(|n| matches!(n, XNode::Element(e) if e.local == "DisabledControllers"))
    {
        if let XNode::Element(old) = root.children.remove(i) {
            disabled = old;
            disabled.local = "DisabledDevices".to_string();
            for device in disabled.elements_mut().filter(|d| d.local == "Controller") {
                device.local = "Device".to_string();
            }
        }
    }
    root.push(disabled);
}

/// "Correct disconnected nodes that were not converted back after being elements."
fn reconnect_leaf_nodes(root: &mut XElement) {
    let mut new_channels = Vec::new();
    if let Some(nodes) = root.child_mut("Nodes") {
        for top in nodes.elements_mut() {
            for child in top.elements_mut().filter(|c| c.local == "Node") {
                reconnect(child, &mut new_channels);
            }
        }
    }
    if let Some(channels) = root.child_mut("Channels") {
        for (id, name) in new_channels {
            let mut e = XElement::new("", "Channel");
            e.set_attr("id", format_guid(&id));
            e.set_attr("name", name);
            channels.push(e);
        }
    }
}

fn reconnect(node: &mut XElement, new_channels: &mut Vec<(Uuid, String)>) {
    let has_nodes = node.elements().any(|c| c.local == "Node");
    if has_nodes {
        for child in node.elements_mut().filter(|c| c.local == "Node") {
            reconnect(child, new_channels);
        }
    } else if node.attr("channelId").is_none() {
        let id = Uuid::new_v4();
        node.set_attr("channelId", format_guid(&id));
        new_channels.push((id, node.attr("name").unwrap_or_default().to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::DotNetColor;
    use crate::xml::same_tree;

    const ORIENTATION_ID: &str = "db0fc9fc-cdd5-4f88-bd66-5e4cf08feae4";
    const LOCATION_ID: &str = "3fb53423-dd2a-4719-b3e3-19aa6f062f62";
    const COLOR_ID: &str = "bff34727-6b88-4f87-82b7-68424498c725";

    const CONFIG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<SystemConfig version="16">
  <Identity>99640bbf-a99f-4b83-8e19-c09357003a06</Identity>
  <AllowFilterEvaluation>true</AllowFilterEvaluation>
  <DefaultUpdateInterval>10</DefaultUpdateInterval>
  <ClearEffectCacheOnExit>false</ClearEffectCacheOnExit>
  <VideoEffectCacheFileType>png</VideoEffectCacheFileType>
  <Channels>
    <Channel id="00000000-0000-0000-0000-00000000000a" name="Channel A" />
    <Channel id="00000000-0000-0000-0000-00000000000b" name="Channel B" />
  </Channels>
  <Nodes>
    <Node name="Group" id="00000000-0000-0000-0000-000000000001">
      <Properties>
        <Property typeId="db0fc9fc-cdd5-4f88-bd66-5e4cf08feae4" instanceId="00000000-0000-0000-0000-000000000011" />
      </Properties>
      <Node name="A" id="00000000-0000-0000-0000-000000000002" channelId="00000000-0000-0000-0000-00000000000a">
        <Properties>
          <Property typeId="3fb53423-dd2a-4719-b3e3-19aa6f062f62" instanceId="00000000-0000-0000-0000-000000000012" />
          <Property typeId="bff34727-6b88-4f87-82b7-68424498c725" instanceId="00000000-0000-0000-0000-000000000013" />
          <Property typeId="11111111-1111-1111-1111-111111111111" instanceId="00000000-0000-0000-0000-000000000014" />
          <Property typeId="47d4a43b-5407-44bd-8be1-4974b148504f" instanceId="00000000-0000-0000-0000-000000000015" />
        </Properties>
        <Tags>
          <Tag id="00000000-0000-0000-0000-000000000021" />
        </Tags>
      </Node>
    </Node>
    <Node name="Other" id="00000000-0000-0000-0000-000000000003">
      <Node name="A" id="00000000-0000-0000-0000-000000000002" channelId="00000000-0000-0000-0000-00000000000a">
        <Properties>
          <Property typeId="3fb53423-dd2a-4719-b3e3-19aa6f062f62" instanceId="00000000-0000-0000-0000-000000000012" />
          <Property typeId="bff34727-6b88-4f87-82b7-68424498c725" instanceId="00000000-0000-0000-0000-000000000013" />
          <Property typeId="11111111-1111-1111-1111-111111111111" instanceId="00000000-0000-0000-0000-000000000014" />
          <Property typeId="47d4a43b-5407-44bd-8be1-4974b148504f" instanceId="00000000-0000-0000-0000-000000000015" />
        </Properties>
        <Tags>
          <Tag id="00000000-0000-0000-0000-000000000021" />
        </Tags>
      </Node>
    </Node>
  </Nodes>
  <Controllers>
    <Controller name="Arduino" hardwareId="d22c9233-16e9-4008-a215-e577f78a7a2e" hardwareInstanceId="ee79fee2-2d7d-4984-a178-288cee39a361" id="064d8c70-9fb5-4c75-8d69-2c716667647b">
      <Outputs>
        <Output name="Output 1" id="4c6d369c-384a-45be-a601-4d2867b54fc5" />
      </Outputs>
    </Controller>
  </Controllers>
  <Previews>
    <Preview name="Tree" typeId="f43c31bc-cfcd-4dbc-adb7-d349c9ec623d" instanceId="da658e06-505c-456f-b5b5-e199b438cdc1" />
  </Previews>
  <Filters>
    <FilterNode typeId="ab38a16f-0de1-4f6e-a8c0-ae5b20d347e0" instanceId="034ce5a2-fb68-4296-b13d-0e8632391309" />
  </Filters>
  <Patches>
    <Patch componentId="4c6d369c-384a-45be-a601-4d2867b54fc5" sourceId="00000000-0000-0000-0000-00000000000a" output="2" />
    <Patch componentId="034ce5a2-fb68-4296-b13d-0e8632391309" />
  </Patches>
  <DisabledDevices>
    <Device id="064d8c70-9fb5-4c75-8d69-2c716667647b" />
  </DisabledDevices>
</SystemConfig>"#;

    fn guid(text: &str) -> Uuid {
        parse_guid(text).unwrap()
    }

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn store() -> ModuleStore {
        let mut s = ModuleStore::new();
        let entry = |type_id: &str, instance: Uuid, data: XElement| {
            ModuleEntry::new("T, A", &guid(type_id), &instance, "M", data)
        };
        let mut orientation = XElement::new(NS_ORIENTATION, "OrientationData");
        orientation.push(XElement::with_text(NS_ORIENTATION, "Orientation", "Vertical"));
        s.set_instance(entry(ORIENTATION_ID, id(0x11), orientation));
        let mut location = XElement::new(NS_LOCATION, "LocationData");
        location.push(XElement::with_text(NS_LOCATION, "X", "5"));
        location.push(XElement::with_text(NS_LOCATION, "Y", "-3"));
        location.push(XElement::with_text(NS_LOCATION, "Z", "1"));
        s.set_instance(entry(LOCATION_ID, id(0x12), location));
        let mut color = XElement::new(NS_COLOR, "ColorData");
        color.push(XElement::with_text(NS_COLOR, "ElementColorType", "SingleColor"));
        color.push(DotNetColor::from_known(141).to_xml(NS_COLOR, "SingleColor"));
        s.set_instance(entry(COLOR_ID, id(0x13), color));
        s.set_instance(entry(
            "11111111-1111-1111-1111-111111111111",
            id(0x14),
            XElement::with_text("urn:x", "Mystery", "keep me"),
        ));
        let mut order = XElement::new(NS_ORDER, "OrderData");
        order.push(XElement::with_text(NS_ORDER, "Order", "7"));
        s.set_instance(entry("47d4a43b-5407-44bd-8be1-4974b148504f", id(0x15), order));
        s
    }

    fn load() -> SystemData {
        system_from_xml(XElement::parse(CONFIG).unwrap(), store()).unwrap()
    }

    #[test]
    fn settings_read() {
        let d = load();
        assert_eq!(d.identity, guid("99640bbf-a99f-4b83-8e19-c09357003a06"));
        assert!(d.allow_filter_evaluation);
        assert_eq!(d.default_update_interval, 10);
        assert!(!d.clear_effect_cache_on_exit);
        assert_eq!(d.video_effect_cache_file_type, "png");
        assert_eq!(d.channels.len(), 2);
    }

    #[test]
    fn the_tree_shares_a_node_listed_twice() {
        let d = load();
        assert_eq!(d.tree.roots.len(), 2);
        assert_eq!(d.tree.len(), 3);
        let a = d.tree.find(&id(2)).unwrap();
        assert_eq!(d.tree.node(a).parents.len(), 2);
        assert_eq!(d.tree.node(a).element_id, Some(id(0xa)));
        assert_eq!(d.tree.node(a).tags, vec![id(0x21)]);
    }

    #[test]
    fn parents_are_in_the_order_the_groups_finish_reading() {
        // XmlElementNodeSerializer.cs:94-95 with GroupNode.cs:10-18: a group links its children after reading them all.
        let leaf = r#"<Node name="X" id="00000000-0000-0000-0000-000000000003" channelId="00000000-0000-0000-0000-00000000000a" />"#;
        let xml = format!(
            r#"<SystemConfig version="16"><Channels><Channel id="00000000-0000-0000-0000-00000000000a" name="A" /></Channels><Nodes><Node name="Outer" id="00000000-0000-0000-0000-000000000001">{leaf}<Node name="Inner" id="00000000-0000-0000-0000-000000000002">{leaf}</Node></Node></Nodes></SystemConfig>"#
        );
        let d = system_from_xml(XElement::parse(&xml).unwrap(), ModuleStore::new()).unwrap();
        let [outer, inner, x] = [1, 2, 3].map(|n| d.tree.find(&id(n)).unwrap());
        assert_eq!(d.tree.node(x).parents, vec![inner, outer]);
        assert_eq!(d.tree.node(outer).children, vec![x, inner]);
        assert_eq!(d.tree.roots, vec![outer]);
    }

    #[test]
    fn a_node_listed_at_the_top_stays_a_root_when_a_group_holds_it_too() {
        // `NodeManager.AddNodes` gives it the root as one more parent, whichever listing comes first.
        let leaf = r#"<Node name="A" id="00000000-0000-0000-0000-000000000002" channelId="00000000-0000-0000-0000-00000000000a" />"#;
        let group = format!(r#"<Node name="Group" id="00000000-0000-0000-0000-000000000001">{leaf}</Node>"#);
        for (nodes, order) in [(format!("{leaf}{leaf}{group}"), [2, 1]), (format!("{group}{leaf}"), [1, 2])] {
            let xml = format!(
                r#"<SystemConfig version="16"><Channels><Channel id="00000000-0000-0000-0000-00000000000a" name="A" /></Channels><Nodes>{nodes}</Nodes></SystemConfig>"#
            );
            let d = system_from_xml(XElement::parse(&xml).unwrap(), ModuleStore::new()).unwrap();
            let roots: Vec<Uuid> = d.tree.roots.iter().map(|r| d.tree.node(*r).id).collect();
            assert_eq!(roots, order.map(id));
            let a = d.tree.find(&id(2)).unwrap();
            let g = d.tree.find(&id(1)).unwrap();
            assert_eq!(d.tree.node(a).parents, vec![g]);
            assert_eq!(d.tree.node(g).children, vec![a]);
            let saved = encode_config(&d);
            let top: Vec<Uuid> = saved.child("Nodes").unwrap().elements().filter_map(|n| attr_guid(n, "id")).collect();
            assert_eq!(top, order.map(id));
        }
    }

    #[test]
    fn properties_resolve_to_typed_data() {
        let d = load();
        let group = d.tree.find(&id(1)).unwrap();
        assert_eq!(d.tree.orientation(group), Some(Orientation::Vertical));
        let a = d.tree.find(&id(2)).unwrap();
        let data: Vec<_> = d.tree.node(a).properties.iter().map(|p| p.data.clone()).collect();
        // Negative location values are clamped by `LocationData.OnDeserialized`.
        assert_eq!(data[0], PropertyData::Location { x: 5, y: 0, z: 1 });
        assert_eq!(
            data[1],
            PropertyData::Color {
                color_type: ElementColorType::SingleColor,
                single_color: Color::RED,
                color_set_name: None
            }
        );
        match &data[2] {
            PropertyData::Unknown { raw_xml } => assert!(raw_xml.contains("keep me")),
            other => panic!("{other:?}"),
        }
        assert_eq!(data[3], PropertyData::Order { order: 7 });
    }

    #[test]
    fn a_full_colour_property_forgets_any_old_single_color() {
        let mut d = XElement::new(NS_COLOR, "ColorData");
        d.push(XElement::with_text(NS_COLOR, "ElementColorType", "FullColor"));
        d.push(color_to_xml(NS_COLOR, "SingleColor", Color::RED));
        d.push(XElement::with_text(NS_COLOR, "ColorSetName", ""));
        assert_eq!(
            decode_color(&d),
            PropertyData::Color {
                color_type: ElementColorType::FullColor,
                single_color: Color::EMPTY,
                color_set_name: None
            }
        );
    }

    #[test]
    fn the_config_round_trips_unchanged() {
        let root = XElement::parse(CONFIG).unwrap();
        let d = load();
        assert_eq!(
            d.residue_members(),
            Vec::<String>::new().into_iter().chain(["Tags".to_string()]).collect::<Vec<_>>()
        );
        let out = system_config_xml(&d);
        assert!(same_tree(&root, &out), "{:?}", root.first_difference(&out, String::new()));
        // The tree holds no edits, so the module store stays as it was.
        assert!(matches!(effective_store(&d), Cow::Borrowed(_)));
    }

    #[test]
    fn devices_and_patches_read_and_write() {
        let d = load();
        assert_eq!(d.controllers[0].name, "Arduino");
        assert_eq!(d.controllers[0].outputs[0].name, "Output 1");
        assert_eq!(d.previews[0].name, "Tree");
        assert_eq!(d.filters.len(), 1);
        assert_eq!(d.patches[0].source_id, Some(id(0xa)));
        assert_eq!(d.patches[0].output, 2);
        assert_eq!(d.patches[1].source_id, None);
        assert_eq!(d.disabled_devices, vec![guid("064d8c70-9fb5-4c75-8d69-2c716667647b")]);
    }

    #[test]
    fn a_changed_property_reaches_the_module_store() {
        let mut d = load();
        let a = d.tree.find(&id(2)).unwrap();
        d.tree.node_mut(a).properties[0].data = PropertyData::Location { x: 9, y: 9, z: 9 };
        let store = effective_store(&d);
        let e = store.instance(guid(LOCATION_ID), id(0x12)).unwrap();
        assert_eq!(e.data().unwrap().child_text("X").as_deref(), Some("9"));
        assert_eq!(e.data().unwrap().child_text("Y").as_deref(), Some("9"));
        // Other properties are untouched.
        let c = store.instance(guid(COLOR_ID), id(0x13)).unwrap();
        assert_eq!(c.data().unwrap().child("SingleColor").unwrap().child_text("state").as_deref(), Some("1"));
    }

    #[test]
    fn a_new_property_gets_a_new_store_entry() {
        let mut d = load();
        let a = d.tree.find(&id(3)).unwrap();
        d.tree.node_mut(a).properties.push(PropertyInstance {
            type_id: guid(ORIENTATION_ID),
            instance_id: id(0x99),
            data: PropertyData::Orientation(Orientation::Vertical),
        });
        let store = effective_store(&d);
        let e = store.instance(guid(ORIENTATION_ID), id(0x99)).unwrap();
        assert_eq!(e.data_model_type(), Some("VixenModules.Property.Orientation.OrientationData, Orientation"));
        assert_eq!(e.data().unwrap().child_text("Orientation").as_deref(), Some("Vertical"));
    }

    #[test]
    fn colour_sets_come_from_the_store_and_changes_go_back() {
        let mut d = load();
        assert_eq!(d.tree.color_sets.len(), 2);
        d.tree.color_sets.insert("Warm".to_string(), vec![Color::ORANGE]);
        let store = effective_store(&d);
        let sets = color_sets(&store);
        assert_eq!(sets.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["RGBW", "RGB", "Warm"]);
    }

    #[test]
    fn typed_edits_write_through() {
        let mut d = load();
        d.default_update_interval = 25;
        d.channels[1].name = "Renamed".to_string();
        d.patches.pop();
        let out = system_config_xml(&d);
        assert_eq!(out.child_text("DefaultUpdateInterval").as_deref(), Some("25"));
        assert_eq!(out.child("Channels").unwrap().elements().nth(1).unwrap().attr("name"), Some("Renamed"));
        assert_eq!(out.child("Patches").unwrap().elements().count(), 1);
    }

    #[test]
    fn migrations_add_what_each_version_adds() {
        let mut root = XElement::parse(r#"<SystemConfig version="7"><DefaultUpdateInterval>46</DefaultUpdateInterval>
            <DisabledControllers><Controller id="064d8c70-9fb5-4c75-8d69-2c716667647b"/></DisabledControllers>
            <Channels/>
            <Nodes><Node name="G" id="00000000-0000-0000-0000-000000000001"><Node name="Lost" id="00000000-0000-0000-0000-000000000002"/></Node></Nodes>
            </SystemConfig>"#).unwrap();
        migrate(&mut root, 7, &ModuleStore::new()).unwrap();
        assert_eq!(root.attr("version"), Some("16"));
        assert!(root.child("Previews").is_some());
        assert_eq!(root.child_text("AllowFilterEvaluation").as_deref(), Some("true"));
        assert_eq!(root.child_text("AllowSubordinateEffects").as_deref(), Some("true"));
        assert!(root.child("SmartControllers").is_some());
        assert!(root.child("DisabledControllers").is_none());
        assert_eq!(root.child("DisabledDevices").unwrap().elements().next().unwrap().local, "Device");
        assert_eq!(root.child_text("DefaultUpdateInterval").as_deref(), Some("50"));
        // The leaf with no channel got one.
        let leaf = root.child("Nodes").unwrap().elements().next().unwrap().elements().next().unwrap();
        let channel = leaf.attr("channelId").unwrap();
        let channels = root.child("Channels").unwrap();
        assert_eq!(channels.elements().count(), 1);
        assert_eq!(channels.elements().next().unwrap().attr("id"), Some(channel));
        assert_eq!(channels.elements().next().unwrap().attr("name"), Some("Lost"));
    }

    #[test]
    fn a_user_chosen_update_interval_stays() {
        let mut root = XElement::parse(
            r#"<SystemConfig version="15"><DefaultUpdateInterval>25</DefaultUpdateInterval></SystemConfig>"#,
        )
        .unwrap();
        migrate(&mut root, 15, &ModuleStore::new()).unwrap();
        assert_eq!(root.child_text("DefaultUpdateInterval").as_deref(), Some("25"));
    }

    #[test]
    fn version_6_outputs_get_filter_nodes() {
        let mut root = XElement::parse(r#"<SystemConfig version="6"><Controllers><Controller><Outputs><Output/></Outputs></Controller></Controllers></SystemConfig>"#).unwrap();
        version_6_to_7(&mut root);
        let output = root
            .child("Controllers")
            .unwrap()
            .child("Controller")
            .unwrap()
            .child("Outputs")
            .unwrap()
            .child("Output")
            .unwrap();
        assert!(output.child("FilterNodes").is_some());
    }

    #[test]
    fn version_11_finds_instance_ids_in_the_store() {
        let mut root = XElement::parse(r#"<SystemConfig version="11"><Controllers><Controller hardwareId="d22c9233-16e9-4008-a215-e577f78a7a2e"/></Controllers></SystemConfig>"#).unwrap();
        let ty = guid("d22c9233-16e9-4008-a215-e577f78a7a2e");
        let mut s = ModuleStore::new();
        s.set_instance(ModuleEntry::new("T, A", &ty, &id(5), "M", XElement::new("", "D")));
        add_device_instance_ids(&mut root, "Controllers", "Controller", &s);
        let c = root.child("Controllers").unwrap().child("Controller").unwrap();
        assert_eq!(c.attr("hardwareInstanceId"), Some(format_guid(&id(5)).as_str()));
    }

    #[test]
    fn a_file_with_no_version_cannot_be_migrated() {
        let e = system_from_xml(XElement::parse("<SystemConfig/>").unwrap(), ModuleStore::new()).unwrap_err();
        assert!(matches!(e, IoError::Migration { .. }), "{e}");
    }

    #[test]
    fn missing_settings_take_the_csharp_defaults() {
        let d =
            system_from_xml(XElement::parse(r#"<SystemConfig version="16"/>"#).unwrap(), ModuleStore::new()).unwrap();
        assert!(d.allow_filter_evaluation);
        assert_eq!(d.default_update_interval, 50);
        assert!(d.clear_effect_cache_on_exit);
        assert_eq!(d.video_effect_cache_file_type, "bmp");
        // ...and stay out of the file until they change.
        let out = system_config_xml(&d);
        assert_eq!(out.elements().count(), 0, "{}", write_fragment(&out));
    }

    #[test]
    fn a_node_cycle_in_a_damaged_file_does_not_hang() {
        let text = r#"<SystemConfig version="16"><Nodes><Node name="A" id="00000000-0000-0000-0000-000000000001"><Node name="A" id="00000000-0000-0000-0000-000000000001"/></Node></Nodes></SystemConfig>"#;
        let d = system_from_xml(XElement::parse(text).unwrap(), ModuleStore::new()).unwrap();
        assert_eq!(d.tree.len(), 1);
        let _ = system_config_xml(&d);
    }

    #[test]
    fn saving_and_loading_a_folder() {
        let dir = crate::test_dir("system");
        let paths = DataPaths::new(&dir);
        let mut d = load();
        let a = d.tree.find(&id(2)).unwrap();
        d.tree.node_mut(a).properties[0].data = PropertyData::Location { x: 1, y: 2, z: 3 };
        save_system(&d, &paths).unwrap();
        let back = load_system(&paths).unwrap();
        let a = back.tree.find(&id(2)).unwrap();
        assert_eq!(back.tree.node(a).properties[0].data, PropertyData::Location { x: 1, y: 2, z: 3 });
        assert_eq!(back.default_update_interval, 10);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_profile_with_no_files_gets_a_new_configuration() {
        let base = crate::test_dir("system-empty");
        let dir = base.join("Vixen 3");
        let d = load_system(&DataPaths::new(&dir)).unwrap();
        assert!(d.tree.is_empty());
        assert_eq!(d.default_update_interval, 50);
        assert_eq!(d.tree.color_sets.len(), 2);
        assert!(!dir.exists(), "loading must not create anything");
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn element_with_children_and_channel_is_a_leaf_as_in_csharp() {
        // `elementId != null` makes the node a leaf and skips its children.
        let text = r#"<SystemConfig version="16"><Nodes><Node name="A" id="00000000-0000-0000-0000-000000000001" channelId="00000000-0000-0000-0000-00000000000a"><Node name="B" id="00000000-0000-0000-0000-000000000002"/></Node></Nodes></SystemConfig>"#;
        let d = system_from_xml(XElement::parse(text).unwrap(), ModuleStore::new()).unwrap();
        assert_eq!(d.tree.len(), 1);
    }

    #[test]
    fn the_library_uses_the_colour_property_id() {
        assert_eq!(crate::library::COLOR_PROPERTY_TYPE_ID, property_ids::COLOR);
    }
}
