//! Properties on nodes: `PropertyManager` and the panel of `SetupElementsTree`. Each property's own dialog is a separate area, and writes through
//! `set_property_data` here.

use super::elements::{retarget_data, PropertyClipboard};
use super::tree::Nid;
use super::SetupSession;
use std::collections::HashSet;
use uuid::Uuid;
use vixen_core::element::{property_ids, ElementColorType, Orientation, PropertyData, PropertyInstance};
use vixen_core::Color;
use vixen_io::xml::write_fragment;
use vixen_io::ModuleEntry;
use vixen_xml::XElement;

/// What the lists need to know about a property module.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PropertyDescriptor {
    pub type_id: Uuid,
    pub type_name: &'static str,
    /// `HasSetup`, which `Setup()` opens.
    pub has_setup: bool,
    /// `HasElementSetupHelper`, which `SetupElements(selected)` opens.
    pub has_helper: bool,
}

/// Every property module, sorted by name as the Mac's Add Property list shows them (F148).
pub const PROPERTY_DESCRIPTORS: [PropertyDescriptor; 8] = [
    PropertyDescriptor { type_id: property_ids::COLOR, type_name: "Color", has_setup: true, has_helper: false },
    PropertyDescriptor { type_id: property_ids::FACE, type_name: "Face", has_setup: false, has_helper: true },
    PropertyDescriptor { type_id: property_ids::GRID, type_name: "Grid", has_setup: true, has_helper: false },
    PropertyDescriptor { type_id: property_ids::INTELLIGENT_FIXTURE, type_name: "Intelligent Fixture", has_setup: true, has_helper: false },
    PropertyDescriptor { type_id: property_ids::LOCATION, type_name: "Location", has_setup: true, has_helper: false },
    PropertyDescriptor { type_id: property_ids::ORDER, type_name: "Order", has_setup: true, has_helper: false },
    PropertyDescriptor { type_id: property_ids::ORIENTATION, type_name: "Orientation", has_setup: true, has_helper: false },
    PropertyDescriptor { type_id: property_ids::STATE, type_name: "State", has_setup: false, has_helper: true },
];

pub fn property_descriptor(type_id: &Uuid) -> Option<&'static PropertyDescriptor> {
    PROPERTY_DESCRIPTORS.iter().find(|d| d.type_id == *type_id)
}

/// A row of the Properties list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyRow {
    pub type_id: Uuid,
    pub type_name: String,
    pub instance_id: Uuid,
    pub has_setup: bool,
    pub has_helper: bool,
    pub is_color: bool,
}

const DCS_ARRAYS: &str = "http://schemas.microsoft.com/2003/10/Serialization/Arrays";

/// `XElement` for an empty dictionary member, as the data contract writes one.
fn empty_dictionary(ns: &str, local: &str) -> XElement {
    let mut e = XElement::new(ns, local);
    e.set_attr("xmlns:a", DCS_ARRAYS);
    e
}

fn id_members(e: &mut XElement, type_id: &Uuid, instance_id: &Uuid) {
    e.push(XElement::with_text("", "ModuleInstanceId", instance_id.to_string()));
    e.push(XElement::with_text("", "ModuleTypeId", type_id.to_string()));
}

/// `Properties.Add(typeId)`: a new instance with `SetDefaultValues` applied. The module types `vixen_core` doesn't model need their store entry made here, so it
/// has the proper `dataModelType` and `assemblyName`.
fn new_property(type_id: Uuid, element_count: usize) -> (PropertyInstance, Option<ModuleEntry>) {
    let instance_id = Uuid::new_v4();
    let unknown = |model: &str, assembly: &str, data: XElement| {
        let raw_xml = write_fragment(&data);
        (PropertyData::Unknown { raw_xml }, Some(ModuleEntry::new(model, &type_id, &instance_id, assembly, data)))
    };
    let (data, entry) = match type_id {
        property_ids::COLOR => (
            PropertyData::Color { color_type: ElementColorType::FullColor, single_color: Color::EMPTY, color_set_name: None },
            None,
        ),
        property_ids::LOCATION => (PropertyData::Location { x: 1, y: 1, z: 1 }, None),
        property_ids::ORDER => (PropertyData::Order { order: 0 }, None),
        property_ids::ORIENTATION => (PropertyData::Orientation(Orientation::Vertical), None),
        property_ids::GRID => {
            const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Property.Grid";
            let mut e = XElement::new(NS, "Data");
            id_members(&mut e, &type_id, &instance_id);
            e.push(XElement::with_text(NS, "Height", "1"));
            e.push(XElement::with_text(NS, "Width", element_count.to_string()));
            unknown("VixenModules.Property.Grid.Data, Grid", "Module.Property.Grid", e)
        }
        property_ids::FACE => {
            const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Property.Face";
            let mut e = XElement::new(NS, "FaceData");
            id_members(&mut e, &type_id, &instance_id);
            e.push(vixen_io::values::color_to_xml(NS, "DefaultColor", Color::WHITE));
            e.push(empty_dictionary(NS, "FaceComponents"));
            e.push(empty_dictionary(NS, "PhonemeList"));
            unknown("VixenModules.Property.Face.FaceData, Face", "Module.Property.Face", e)
        }
        property_ids::STATE => {
            const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Property.State";
            let mut e = XElement::new(NS, "StateData");
            id_members(&mut e, &type_id, &instance_id);
            e.push(XElement::with_text(NS, "Id", Uuid::new_v4().to_string()));
            e.push(XElement::new(NS, "StateDefinitions"));
            unknown("VixenModules.Property.State.StateData, State", "Module.Property.State", e)
        }
        property_ids::INTELLIGENT_FIXTURE => {
            let e = super::fixture::new_fixture_property_data(&type_id, &instance_id);
            unknown(
                "VixenModules.Property.IntelligentFixture.IntelligentFixtureData, IntelligentFixture",
                "Module.Property.IntelligentFixture",
                e,
            )
        }
        _ => (PropertyData::Unknown { raw_xml: String::new() }, None),
    };
    (PropertyInstance { type_id, instance_id, data }, entry)
}

impl SetupSession {
    /// F16: the properties of one node, in its order.
    pub fn properties_of(&self, primary: &str) -> Vec<PropertyRow> {
        let Some(node) = self.tree.resolve(primary).and_then(|p| p.last().copied()) else { return Vec::new() };
        self.tree
            .node(node)
            .properties
            .iter()
            .map(|p| {
                let descriptor = property_descriptor(&p.type_id);
                PropertyRow {
                    type_id: p.type_id,
                    type_name: descriptor.map_or_else(|| "Unknown Property".to_string(), |d| d.type_name.to_string()),
                    instance_id: p.instance_id,
                    has_setup: descriptor.is_some_and(|d| d.has_setup),
                    has_helper: descriptor.is_some_and(|d| d.has_helper),
                    is_color: p.type_id == property_ids::COLOR,
                }
            })
            .collect()
    }

    fn element_count(&self, node: Nid) -> usize {
        let mut leaves = Vec::new();
        self.tree.collect_leaves(node, &mut leaves);
        leaves.iter().filter(|l| self.tree.node(**l).element_id.is_some()).count()
    }

    /// F17: on every selected row's node, not their descendants. A node that has the property already keeps it (`Properties.Add` returns null).
    pub fn add_property(&mut self, selection: &[String], type_id: Uuid) -> Vec<Nid> {
        let mut changed = Vec::new();
        let mut seen = HashSet::new();
        for node in self.nodes_of(selection) {
            if !seen.insert(node) || self.tree.node(node).properties.iter().any(|p| p.type_id == type_id) {
                continue;
            }
            let count = if type_id == property_ids::GRID { self.element_count(node) } else { 0 };
            let (property, entry) = new_property(type_id, count);
            if let Some(entry) = entry {
                self.system.module_store.set_instance(entry);
            }
            self.tree.node_mut(node).properties.push(property);
            changed.push(node);
        }
        if !changed.is_empty() {
            self.dirty.tree = true;
        }
        changed
    }

    /// F18: from every selected element that has them, even those the list wasn't filled from.
    pub fn remove_properties(&mut self, selection: &[String], type_ids: &[Uuid]) {
        let mut changed = false;
        let mut seen = HashSet::new();
        for node in self.nodes_of(selection) {
            if !seen.insert(node) {
                continue;
            }
            let n = self.tree.node_mut(node);
            let mut removed = Vec::new();
            n.properties.retain(|p| {
                if type_ids.contains(&p.type_id) {
                    removed.push((p.type_id, p.instance_id));
                    false
                } else {
                    true
                }
            });
            changed |= !removed.is_empty();
            self.removed_properties.extend(removed);
        }
        if changed {
            self.dirty.tree = true;
        }
    }

    /// F19's second branch: after a property's own setup, every other selected element having this type gets a copy of its values.
    pub fn clone_property_to_selection(&mut self, selection: &[String], source: &str, type_id: Uuid) {
        let Some(source_node) = self.tree.resolve(source).and_then(|p| p.last().copied()) else { return };
        let Some(from) = self.tree.node(source_node).properties.iter().find(|p| p.type_id == type_id).cloned() else { return };
        let mut changed = false;
        for node in self.nodes_of(selection) {
            if node == source_node {
                continue;
            }
            if let Some(target) = self.tree.node_mut(node).properties.iter_mut().find(|p| p.type_id == type_id) {
                let data = retarget_data(&from.data, &from.instance_id, &target.instance_id);
                if target.data != data {
                    target.data = data;
                    changed = true;
                }
            }
        }
        if changed {
            self.dirty.tree = true;
        }
    }

    /// F38: this row's properties, as they are now and by reference to the node.
    pub fn copy_properties(&mut self, path: &str) {
        let Some(node) = self.tree.resolve(path).and_then(|p| p.last().copied()) else { return };
        self.property_clipboard = Some(PropertyClipboard { source: node, properties: self.tree.node(node).properties.clone() });
    }

    /// F39: for each selected node and each clipboard property, the node's own property of that type (or a new one) takes the values.
    /// Properties the clipboard lacks stay. Colour filters aren't touched, unlike F19's colour route.
    pub fn paste_properties(&mut self, selection: &[String]) {
        let Some(clipboard) = self.property_clipboard.clone() else { return };
        // Edits to the source since the copy show here, because upstream holds references.
        let current: Vec<PropertyInstance> = clipboard
            .properties
            .iter()
            .map(|snapshot| {
                let live = self.tree.is_live(clipboard.source).then(|| {
                    self.tree.node(clipboard.source).properties.iter().find(|p| p.type_id == snapshot.type_id).cloned()
                });
                live.flatten().unwrap_or_else(|| snapshot.clone())
            })
            .collect();
        let mut seen = HashSet::new();
        for node in self.nodes_of(selection) {
            if !seen.insert(node) {
                continue;
            }
            for source in &current {
                if !self.tree.node(node).properties.iter().any(|p| p.type_id == source.type_id) {
                    let count = if source.type_id == property_ids::GRID { self.element_count(node) } else { 0 };
                    let (property, entry) = new_property(source.type_id, count);
                    if let Some(entry) = entry {
                        self.system.module_store.set_instance(entry);
                    }
                    self.tree.node_mut(node).properties.push(property);
                }
                let target = self
                    .tree
                    .node_mut(node)
                    .properties
                    .iter_mut()
                    .find(|p| p.type_id == source.type_id)
                    .expect("just made sure of it");
                target.data = retarget_data(&source.data, &source.instance_id, &target.instance_id);
            }
            self.dirty.tree = true;
        }
    }

    /// For the property dialogs: the data of one property on one node.
    pub fn property_data(&self, node: Nid, type_id: &Uuid) -> Option<&PropertyData> {
        self.tree.node(node).properties.iter().find(|p| p.type_id == *type_id).map(|p| &p.data)
    }

    /// For the property dialogs: replaces the data of one property on one node.
    pub fn set_property_data(&mut self, node: Nid, type_id: &Uuid, data: PropertyData) -> bool {
        match self.tree.node_mut(node).properties.iter_mut().find(|p| p.type_id == *type_id) {
            Some(p) if p.data != data => {
                p.data = data;
                self.dirty.tree = true;
                true
            }
            _ => false,
        }
    }

    /// Makes sure a copied `Unknown` property has a store entry of its own, with its module's names, so the save doesn't invent one without them.
    pub(crate) fn register_unknown_properties(&mut self, properties: &[PropertyInstance]) {
        for p in properties {
            let PropertyData::Unknown { raw_xml } = &p.data else { continue };
            let Some(data) = vixen_io::xml::parse_fragment(raw_xml) else { continue };
            let (model, assembly) = match self.system.module_store.instance_entries().iter().find(|e| e.module_type() == Some(p.type_id)) {
                Some(e) => (e.data_model_type().unwrap_or_default().to_string(), e.assembly_name().unwrap_or_default().to_string()),
                None => match p.type_id {
                    property_ids::GRID => ("VixenModules.Property.Grid.Data, Grid".to_string(), "Module.Property.Grid".to_string()),
                    property_ids::FACE => ("VixenModules.Property.Face.FaceData, Face".to_string(), "Module.Property.Face".to_string()),
                    property_ids::STATE => ("VixenModules.Property.State.StateData, State".to_string(), "Module.Property.State".to_string()),
                    property_ids::INTELLIGENT_FIXTURE => (
                        "VixenModules.Property.IntelligentFixture.IntelligentFixtureData, IntelligentFixture".to_string(),
                        "Module.Property.IntelligentFixture".to_string(),
                    ),
                    _ => (String::new(), String::new()),
                },
            };
            self.system.module_store.set_instance(ModuleEntry::new(&model, &p.type_id, &p.instance_id, &assembly, data));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    fn row(s: &SetupSession, name: &str) -> String {
        s.row_path_of(s.tree.live_nodes().find(|(_, n)| n.name == name).unwrap().0)
    }

    #[test]
    fn add_gives_each_module_its_defaults_and_leaves_existing_ones() {
        let mut s = session(&["A", "B"], 0);
        let (a, b) = (row(&s, "A"), row(&s, "B"));
        s.add_property(&[a.clone(), b.clone()], property_ids::LOCATION);
        let names: Vec<String> = s.properties_of(&a).into_iter().map(|p| p.type_name).collect();
        assert_eq!(names, ["Location"]);
        let node = s.tree.resolve(&a).unwrap()[0];
        assert_eq!(s.property_data(node, &property_ids::LOCATION), Some(&PropertyData::Location { x: 1, y: 1, z: 1 }));
        s.set_property_data(node, &property_ids::LOCATION, PropertyData::Location { x: 5, y: 5, z: 5 });
        assert!(s.add_property(&[a.clone()], property_ids::LOCATION).is_empty(), "already there");
        assert_eq!(s.property_data(node, &property_ids::LOCATION), Some(&PropertyData::Location { x: 5, y: 5, z: 5 }));
        s.add_property(&[a.clone()], property_ids::ORIENTATION);
        assert_eq!(s.property_data(node, &property_ids::ORIENTATION), Some(&PropertyData::Orientation(Orientation::Vertical)));
        s.add_property(&[a.clone()], property_ids::COLOR);
        assert!(s.properties_of(&a)[2].is_color);
        s.add_property(&[a.clone()], property_ids::ORDER);
        assert_eq!(s.property_data(node, &property_ids::ORDER), Some(&PropertyData::Order { order: 0 }));
    }

    #[test]
    fn grid_counts_the_elements_and_gets_a_named_store_entry() {
        let mut s = session(&["G", "X", "Y"], 0);
        let (g, x, y) = (row(&s, "G"), row(&s, "X"), row(&s, "Y"));
        s.drop_rows(&[x, y], Some(&g), None, false);
        s.add_property(&[g.clone()], property_ids::GRID);
        let instance = s.properties_of(&g)[0].instance_id;
        let entry = s.system.module_store.instance(property_ids::GRID, instance).unwrap();
        assert_eq!(entry.data_model_type(), Some("VixenModules.Property.Grid.Data, Grid"));
        assert_eq!(entry.data().unwrap().child_text("Width").as_deref(), Some("2"));
        assert_eq!(entry.data().unwrap().child_text("Height").as_deref(), Some("1"));
    }

    #[test]
    fn remove_reaches_every_selected_element() {
        let mut s = session(&["A", "B"], 0);
        let (a, b) = (row(&s, "A"), row(&s, "B"));
        s.add_property(&[a.clone(), b.clone()], property_ids::ORDER);
        s.add_property(&[a.clone()], property_ids::LOCATION);
        s.remove_properties(&[a.clone(), b.clone()], &[property_ids::ORDER]);
        assert_eq!(s.properties_of(&a).len(), 1);
        assert!(s.properties_of(&b).is_empty());
        assert_eq!(s.removed_properties.len(), 2);
    }

    #[test]
    fn paste_properties_makes_or_reuses_and_sees_later_edits() {
        let mut s = session(&["A", "B"], 0);
        let (a, b) = (row(&s, "A"), row(&s, "B"));
        let (an, bn) = (s.tree.resolve(&a).unwrap()[0], s.tree.resolve(&b).unwrap()[0]);
        s.add_property(&[a.clone()], property_ids::LOCATION);
        s.add_property(&[b.clone()], property_ids::ORDER);
        s.copy_properties(&a);
        // An edit after the copy shows at the paste.
        s.set_property_data(an, &property_ids::LOCATION, PropertyData::Location { x: 9, y: 8, z: 7 });
        s.paste_properties(&[b.clone()]);
        assert_eq!(s.property_data(bn, &property_ids::LOCATION), Some(&PropertyData::Location { x: 9, y: 8, z: 7 }));
        assert!(s.property_data(bn, &property_ids::ORDER).is_some(), "what the clipboard lacks stays");
        assert_ne!(s.properties_of(&a)[0].instance_id, s.properties_of(&b)[1].instance_id);
    }

    #[test]
    fn clone_to_selection_only_touches_those_having_the_type() {
        let mut s = session(&["A", "B", "C"], 0);
        let (a, b, c) = (row(&s, "A"), row(&s, "B"), row(&s, "C"));
        s.add_property(&[a.clone(), b.clone()], property_ids::ORDER);
        let (an, bn) = (s.tree.resolve(&a).unwrap()[0], s.tree.resolve(&b).unwrap()[0]);
        s.set_property_data(an, &property_ids::ORDER, PropertyData::Order { order: 42 });
        s.clone_property_to_selection(&[a.clone(), b.clone(), c.clone()], &a, property_ids::ORDER);
        assert_eq!(s.property_data(bn, &property_ids::ORDER), Some(&PropertyData::Order { order: 42 }));
        assert!(s.properties_of(&c).is_empty());
    }

    #[test]
    fn the_list_is_sorted_by_name() {
        let names: Vec<&str> = PROPERTY_DESCRIPTORS.iter().map(|d| d.type_name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }
}
