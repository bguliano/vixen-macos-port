//! What a dialog reads and writes, wherever it lives. Display Setup works on a copy of the system, so its dialogs go to that copy,
//! and Cancel on the window drops them. Anything else, such as the sequencer's Edit Element Face Mapping, goes to the open profile.
//!
//! A dialog's logic is written against [`DialogWorld`], and the bindings get a world through a [`WorldProvider`].

use super::store;
use crate::app::AppCore;
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::element::{ElementColorType, NodeId, PropertyData, PropertyInstance};
use vixen_core::Color;
use vixen_io::{color_sets, set_color_sets, ModuleEntry, OutputFilter, Patch, SystemData};
use vixen_output::filters::FilterConfig;
use vixen_output::modules::ControllerConfig;
use vixen_xml::XElement;

/// A node of the world's tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeKey(pub u32);

/// What the dialogs need to know about a controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerInfo {
    pub id: Uuid,
    pub name: String,
    pub type_id: Uuid,
    pub output_count: u32,
    pub running: bool,
}

/// The system, as the dialogs see it. Data flow components go by id: a leaf's element, a filter's instance or a controller output.
pub trait DialogWorld {
    fn find_node(&self, id: &Uuid) -> Option<NodeKey>;
    fn node_id(&self, node: NodeKey) -> Uuid;
    fn node_name(&self, node: NodeKey) -> String;
    /// `ElementNode.Element.Id`. Groups have none.
    fn node_element(&self, node: NodeKey) -> Option<Uuid>;
    fn node_children(&self, node: NodeKey) -> Vec<NodeKey>;
    fn properties(&self, node: NodeKey) -> &[PropertyInstance];
    fn properties_mut(&mut self, node: NodeKey) -> &mut Vec<PropertyInstance>;
    /// Keeps the store's copy of a property kept only as XML, so a save has nothing to invent.
    fn set_property_store_entry(&mut self, type_id: &Uuid, instance_id: &Uuid, data: XElement);

    /// The colour sets, in the order they were added.
    fn color_set_names(&self) -> Vec<String>;
    fn color_set(&self, name: &str) -> Option<Vec<Color>>;
    fn set_color_sets(&mut self, sets: &[(String, Vec<Color>)]);

    fn filter_type(&self, component: &Uuid) -> Option<Uuid>;
    fn filter_config(&self, component: &Uuid) -> Option<FilterConfig>;
    fn set_filter_config(&mut self, config: FilterConfig);
    fn add_filter(&mut self, config: FilterConfig);
    /// `Outputs.Length`. An element has one and a controller output none.
    fn output_count(&self, component: &Uuid) -> usize;
    /// `GetDestinationsOfComponentOutput`.
    fn destinations(&self, component: &Uuid, output: i32) -> Vec<Uuid>;
    /// `SetComponentSource`, replacing whatever source it had.
    fn set_source(&mut self, component: Uuid, source: Uuid, output: i32);
    /// `ResetComponentSource`.
    fn reset_source(&mut self, component: &Uuid);

    fn controllers(&self) -> Vec<ControllerInfo>;
    fn controller_config(&self, id: &Uuid) -> Option<ControllerConfig>;
    fn set_controller_config(&mut self, id: &Uuid, config: ControllerConfig);
    fn set_output_names(&mut self, id: &Uuid, names: &[String]);
}

/// Gives a dialog a world for the length of one call.
pub trait WorldProvider: Send + Sync {
    fn with_world(&self, f: &mut dyn FnMut(&mut dyn DialogWorld));
}

/// [`WorldProvider::with_world`], with a result.
pub fn with_world<R>(provider: &dyn WorldProvider, f: impl FnOnce(&mut dyn DialogWorld) -> R) -> R {
    let mut f = Some(f);
    let mut result = None;
    provider.with_world(&mut |world| {
        if let Some(f) = f.take() {
            result = Some(f(world));
        }
    });
    result.expect("a provider calls its closure")
}

// ---- Things every world can answer -----------------------------------------------------------------------------------------

/// `GetLeafEnumerator()`: a leaf reached through two parents appears twice.
pub fn leaves(world: &dyn DialogWorld, node: NodeKey) -> Vec<NodeKey> {
    let mut out = Vec::new();
    collect_leaves(world, node, &mut out);
    out
}

fn collect_leaves(world: &dyn DialogWorld, node: NodeKey, out: &mut Vec<NodeKey>) {
    let children = world.node_children(node);
    if children.is_empty() {
        out.push(node);
    } else {
        for child in children {
            collect_leaves(world, child, out);
        }
    }
}

/// `selectedNodes.SelectMany(GetLeafEnumerator).Distinct()`.
pub fn distinct_leaves(world: &dyn DialogWorld, nodes: &[NodeKey]) -> Vec<NodeKey> {
    let mut seen = HashSet::new();
    nodes.iter().flat_map(|n| leaves(world, *n)).filter(|l| seen.insert(*l)).collect()
}

/// `node.Count()`, counting a repeated leaf each time.
pub fn element_count(world: &dyn DialogWorld, node: NodeKey) -> usize {
    leaves(world, node).iter().filter(|l| world.node_element(**l).is_some()).count()
}

pub fn property_data<'a>(world: &'a dyn DialogWorld, node: NodeKey, type_id: &Uuid) -> Option<&'a PropertyData> {
    world.properties(node).iter().find(|p| p.type_id == *type_id).map(|p| &p.data)
}

/// `ColorModule.getValidColorsForElementNode`: empty means full colour.
pub fn valid_colors(world: &dyn DialogWorld, node: NodeKey, include_children: bool) -> Vec<Color> {
    let mut out = Vec::new();
    collect_valid_colors(world, node, include_children, &mut out);
    out
}

fn collect_valid_colors(world: &dyn DialogWorld, node: NodeKey, include_children: bool, out: &mut Vec<Color>) {
    let push = |out: &mut Vec<Color>, c: Color| {
        if !out.contains(&c) {
            out.push(c);
        }
    };
    for p in world.properties(node) {
        if let PropertyData::Color { color_type, single_color, color_set_name } = &p.data {
            match color_type {
                ElementColorType::FullColor => {}
                ElementColorType::SingleColor => push(out, *single_color),
                ElementColorType::MultipleDiscreteColors => {
                    for c in color_set_name.as_deref().and_then(|n| world.color_set(n)).unwrap_or_default() {
                        push(out, c);
                    }
                }
            }
            break;
        }
    }
    if include_children {
        for child in world.node_children(node) {
            collect_valid_colors(world, child, true, out);
        }
    }
}

// ---- The open profile ----------------------------------------------------------------------------------------------------------

/// An index of the patches, made the first time something asks.
struct FlowIndex {
    destinations: HashMap<(Uuid, i32), Vec<Uuid>>,
    elements: HashSet<Uuid>,
}

impl FlowIndex {
    fn new(system: &SystemData) -> FlowIndex {
        let mut destinations: HashMap<(Uuid, i32), Vec<Uuid>> = HashMap::with_capacity(system.patches.len());
        for p in &system.patches {
            if let Some(source) = p.source_id {
                destinations.entry((source, p.output)).or_default().push(p.component_id);
            }
        }
        FlowIndex { destinations, elements: system.channels.iter().map(|c| c.id).chain(system.tree.element_ids()).collect() }
    }
}

/// What a call changed, for whatever has to follow: the running graph and controllers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldChanges {
    pub filters: Vec<Uuid>,
    pub controllers: Vec<Uuid>,
    /// Filters or patches came or went, so the graph has to be built again.
    pub structure: bool,
}

/// A profile's data as a world.
pub struct SystemWorld<'a> {
    pub system: &'a mut SystemData,
    running: HashSet<Uuid>,
    index: OnceCell<FlowIndex>,
    filter_types: OnceCell<HashMap<Uuid, Uuid>>,
    pub changes: WorldChanges,
}

impl<'a> SystemWorld<'a> {
    pub fn new(system: &'a mut SystemData) -> SystemWorld<'a> {
        SystemWorld::with_running(system, HashSet::new())
    }

    pub fn with_running(system: &'a mut SystemData, running: HashSet<Uuid>) -> SystemWorld<'a> {
        SystemWorld {
            system,
            running,
            index: OnceCell::new(),
            filter_types: OnceCell::new(),
            changes: WorldChanges::default(),
        }
    }

    fn index(&self) -> &FlowIndex {
        self.index.get_or_init(|| FlowIndex::new(self.system))
    }

    fn filter_types(&self) -> &HashMap<Uuid, Uuid> {
        self.filter_types.get_or_init(|| self.system.filters.iter().map(|f| (f.instance_id, f.type_id)).collect())
    }
}

fn node_id(node: NodeKey) -> NodeId {
    NodeId(node.0)
}

/// The colour sets' names, in the order they were added.
pub(crate) fn color_set_names_in(system: &SystemData) -> Vec<String> {
    let mut names: Vec<String> =
        color_sets(&system.module_store).into_iter().map(|(name, _)| name).filter(|name| system.tree.color_sets.contains_key(name)).collect();
    let mut extra: Vec<String> = system.tree.color_sets.keys().filter(|n| !names.contains(n)).cloned().collect();
    extra.sort();
    names.extend(extra);
    names
}

impl DialogWorld for SystemWorld<'_> {
    fn find_node(&self, id: &Uuid) -> Option<NodeKey> {
        self.system.tree.find(id).map(|n| NodeKey(n.0))
    }

    fn node_id(&self, node: NodeKey) -> Uuid {
        self.system.tree.node(node_id(node)).id
    }

    fn node_name(&self, node: NodeKey) -> String {
        self.system.tree.node(node_id(node)).name.clone()
    }

    fn node_element(&self, node: NodeKey) -> Option<Uuid> {
        self.system.tree.node(node_id(node)).element_id
    }

    fn node_children(&self, node: NodeKey) -> Vec<NodeKey> {
        self.system.tree.node(node_id(node)).children.iter().map(|c| NodeKey(c.0)).collect()
    }

    fn properties(&self, node: NodeKey) -> &[PropertyInstance] {
        &self.system.tree.node(node_id(node)).properties
    }

    fn properties_mut(&mut self, node: NodeKey) -> &mut Vec<PropertyInstance> {
        &mut self.system.tree.node_mut(node_id(node)).properties
    }

    fn set_property_store_entry(&mut self, type_id: &Uuid, instance_id: &Uuid, data: XElement) {
        match self.system.module_store.instance_mut(*type_id, *instance_id) {
            Some(entry) => entry.set_data(data),
            None => {
                if let Some((model, assembly)) = store::property_module_names(type_id) {
                    self.system.module_store.set_instance(ModuleEntry::new(model, type_id, instance_id, assembly, data));
                }
            }
        }
    }

    fn color_set_names(&self) -> Vec<String> {
        color_set_names_in(self.system)
    }

    fn color_set(&self, name: &str) -> Option<Vec<Color>> {
        self.system.tree.color_sets.get(name).cloned()
    }

    fn set_color_sets(&mut self, sets: &[(String, Vec<Color>)]) {
        self.system.tree.color_sets = sets.iter().cloned().collect();
        set_color_sets(&mut self.system.module_store, sets);
    }

    fn filter_type(&self, component: &Uuid) -> Option<Uuid> {
        self.filter_types().get(component).copied()
    }

    fn filter_config(&self, component: &Uuid) -> Option<FilterConfig> {
        let type_id = self.filter_type(component)?;
        store::filter_config_in(&self.system.module_store, &type_id, component)
    }

    fn set_filter_config(&mut self, config: FilterConfig) {
        let instance = config.ids().instance_id;
        let count = config.output_names().len() as i32;
        // What was on an output that's gone loses its source, as it does in Display Setup.
        let dropped: Vec<Uuid> = self
            .system
            .patches
            .iter()
            .filter(|p| p.source_id == Some(instance) && (p.output < 0 || p.output >= count))
            .map(|p| p.component_id)
            .collect();
        for component in &dropped {
            self.reset_source(component);
        }
        self.changes.filters.push(instance);
        store::store_filter_config(&mut self.system.module_store, &config);
    }

    fn add_filter(&mut self, config: FilterConfig) {
        let ids = *config.ids();
        self.system.filters.push(OutputFilter { type_id: ids.type_id, instance_id: ids.instance_id });
        if let Some(types) = self.filter_types.get_mut() {
            types.insert(ids.instance_id, ids.type_id);
        }
        store::store_filter_config(&mut self.system.module_store, &config);
        self.changes.structure = true;
    }

    fn output_count(&self, component: &Uuid) -> usize {
        if self.index().elements.contains(component) {
            return 1;
        }
        self.filter_config(component).map_or(0, |c| c.output_names().len())
    }

    fn destinations(&self, component: &Uuid, output: i32) -> Vec<Uuid> {
        self.index().destinations.get(&(*component, output)).cloned().unwrap_or_default()
    }

    fn set_source(&mut self, component: Uuid, source: Uuid, output: i32) {
        self.reset_source(&component);
        let patch = Patch { component_id: component, source_id: Some(source), output };
        match self.system.patches.iter_mut().find(|p| p.component_id == component) {
            Some(existing) => *existing = patch,
            None => self.system.patches.push(patch),
        }
        if self.index.get().is_some() {
            if let Some(index) = self.index.get_mut() {
                index.destinations.entry((source, output)).or_default().push(component);
            }
        }
        self.changes.structure = true;
    }

    fn reset_source(&mut self, component: &Uuid) {
        let Some(existing) = self.system.patches.iter_mut().find(|p| p.component_id == *component) else {
            return;
        };
        let old = existing.source_id.take().map(|source| (source, existing.output));
        existing.output = 0;
        if let (Some(key), Some(index)) = (old, self.index.get_mut()) {
            if let Some(list) = index.destinations.get_mut(&key) {
                list.retain(|c| c != component);
            }
        }
        self.changes.structure = true;
    }

    fn controllers(&self) -> Vec<ControllerInfo> {
        self.system
            .controllers
            .iter()
            .map(|c| ControllerInfo {
                id: c.id,
                name: c.name.clone(),
                type_id: c.hardware_type_id,
                output_count: c.outputs.len() as u32,
                running: self.running.contains(&c.id),
            })
            .collect()
    }

    fn controller_config(&self, id: &Uuid) -> Option<ControllerConfig> {
        store::controller_config(self.system, id)
    }

    fn set_controller_config(&mut self, id: &Uuid, config: ControllerConfig) {
        store::store_controller_config(&mut self.system.module_store, &config);
        self.changes.controllers.push(*id);
    }

    fn set_output_names(&mut self, id: &Uuid, names: &[String]) {
        if let Some(controller) = self.system.controllers.iter_mut().find(|c| c.id == *id) {
            for (output, name) in controller.outputs.iter_mut().zip(names) {
                output.name = name.clone();
            }
        }
    }
}

/// The open profile as a world. What a call changes reaches the running graph and controllers straight away.
pub struct CoreWorld(pub Arc<AppCore>);

impl WorldProvider for CoreWorld {
    fn with_world(&self, f: &mut dyn FnMut(&mut dyn DialogWorld)) {
        let core = &self.0;
        let running = store::running_controllers(core);
        let changes = {
            let mut system = core.system.write();
            let mut world = SystemWorld::with_running(&mut system, running);
            f(&mut world);
            world.changes
        };
        if changes.controllers.is_empty() {
            store::follow_changes(core, &changes);
        } else {
            // Restarting a controller opens ports and sockets, and the sheet that asked for it closes straight away.
            let core = Arc::clone(core);
            std::thread::spawn(move || store::follow_changes(&core, &changes));
        }
    }
}
