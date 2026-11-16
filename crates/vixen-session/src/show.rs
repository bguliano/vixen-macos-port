//! Connects the loaded system data to the output layer.

use std::collections::HashMap;
use uuid::Uuid;
use vixen_io::{ModuleEntry, ModuleStore, SystemData};
use vixen_output::{ControllerDef, FilterDef, GraphDefinition, ModuleDataLookup, OutputDef, OutputGraph, PatchDef};
use vixen_xml::XElement;

/// The module store's instance data, as the output layer reads it.
pub struct StoreLookup<'a> {
    entries: HashMap<(Uuid, Uuid), &'a ModuleEntry>,
}

impl<'a> StoreLookup<'a> {
    pub fn new(store: &'a ModuleStore) -> StoreLookup<'a> {
        let mut entries = HashMap::with_capacity(store.instance_entries().len());
        for entry in store.instance_entries() {
            if let (Some(t), Some(i)) = (entry.module_type(), entry.module_instance()) {
                entries.insert((t, i), entry);
            }
        }
        StoreLookup { entries }
    }
}

impl ModuleDataLookup for StoreLookup<'_> {
    fn module_data(&self, type_id: &Uuid, instance_id: &Uuid) -> Option<&XElement> {
        self.entries.get(&(*type_id, *instance_id)).and_then(|e| e.data())
    }
}

/// What the output graph is built from. `elements` fixes the order of the per-frame states.
pub fn graph_definition(system: &SystemData, elements: &[Uuid]) -> GraphDefinition {
    GraphDefinition {
        elements: elements.to_vec(),
        controllers: system
            .controllers
            .iter()
            .map(|c| ControllerDef {
                id: c.id,
                name: c.name.clone(),
                hardware_type_id: c.hardware_type_id,
                hardware_instance_id: c.hardware_instance_id,
                outputs: c.outputs.iter().map(|o| OutputDef { id: o.id, name: o.name.clone() }).collect(),
            })
            .collect(),
        filters: system.filters.iter().map(|f| FilterDef { type_id: f.type_id, instance_id: f.instance_id }).collect(),
        patches: system
            .patches
            .iter()
            .map(|p| PatchDef { component_id: p.component_id, source_id: p.source_id, output_index: p.output })
            .collect(),
    }
}

/// Builds the output graph for a profile, with the frame states in `elements` order.
pub fn build_output_graph(system: &SystemData, elements: &[Uuid]) -> OutputGraph {
    let store = vixen_io::system::effective_store(system);
    OutputGraph::build(&graph_definition(system, elements), &StoreLookup::new(&store))
}
