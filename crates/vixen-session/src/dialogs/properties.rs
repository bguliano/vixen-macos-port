//! Element property setup: what Display Setup's Configure button routes to, and the simple property dialogs (F3, F178 to F183).

use super::world::{element_count, DialogWorld, NodeKey};
use uuid::Uuid;
use vixen_core::element::{property_ids, ElementColorType, Orientation, PropertyData, PropertyInstance};
use vixen_core::Color;
use vixen_xml::{contract_ns, XElement};

/// What a property's Configure opens (F3). `None` does nothing.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropertySetupKind {
    /// The Color helper, over the whole selection, not the property's own dialog.
    Color,
    Location,
    Orientation,
    Order,
    Grid,
    Face,
    State,
    /// Opens the fixture editor, which another area owns.
    IntelligentFixture,
    None,
}

pub fn property_setup_kind(type_id: &Uuid) -> PropertySetupKind {
    match *type_id {
        property_ids::COLOR => PropertySetupKind::Color,
        property_ids::LOCATION => PropertySetupKind::Location,
        property_ids::ORIENTATION => PropertySetupKind::Orientation,
        property_ids::ORDER => PropertySetupKind::Order,
        property_ids::GRID => PropertySetupKind::Grid,
        property_ids::FACE => PropertySetupKind::Face,
        property_ids::STATE => PropertySetupKind::State,
        property_ids::INTELLIGENT_FIXTURE => PropertySetupKind::IntelligentFixture,
        _ => PropertySetupKind::None,
    }
}

/// The setup helpers, in the fixed order the combo lists them (F4).
pub const SETUP_HELPERS: [&str; 6] =
    ["Color Handling", "Dimming Curve", "Face Mapping", "Orientation", "Patching Order", "State Mapping"];

/// What `SetDefaultValues` gives a new property. Modules kept as XML make their data when they're first written.
pub fn property_defaults(type_id: &Uuid) -> PropertyData {
    match *type_id {
        property_ids::COLOR => {
            PropertyData::Color { color_type: ElementColorType::FullColor, single_color: Color::EMPTY, color_set_name: None }
        }
        property_ids::LOCATION => PropertyData::Location { x: 1, y: 1, z: 1 },
        property_ids::ORIENTATION => PropertyData::Orientation(Orientation::Vertical),
        property_ids::ORDER => PropertyData::Order { order: 0 },
        _ => PropertyData::Unknown { raw_xml: String::new() },
    }
}

/// `Properties.Add`, and returns the new property's index.
pub fn add_property(world: &mut dyn DialogWorld, node: NodeKey, type_id: &Uuid) -> usize {
    let data = property_defaults(type_id);
    let properties = world.properties_mut(node);
    properties.push(PropertyInstance { type_id: *type_id, instance_id: Uuid::new_v4(), data });
    let index = properties.len() - 1;
    if *type_id == property_ids::GRID {
        let count = element_count(world, node) as i32;
        write_grid(world, node, index, count, 1);
    }
    index
}

/// The property's index on the node, adding it when it isn't there.
pub fn ensure_property(world: &mut dyn DialogWorld, node: NodeKey, type_id: &Uuid) -> usize {
    match world.properties(node).iter().position(|p| p.type_id == *type_id) {
        Some(index) => index,
        None => add_property(world, node, type_id),
    }
}

pub(crate) fn set_id_members(element: &mut XElement, instance_id: &Uuid, type_id: &Uuid) {
    if let Some(e) = element.child_mut("ModuleInstanceId") {
        *e = XElement::with_text("", "ModuleInstanceId", instance_id.to_string());
    }
    if let Some(e) = element.child_mut("ModuleTypeId") {
        *e = XElement::with_text("", "ModuleTypeId", type_id.to_string());
    }
}

/// Stores a property the tree keeps only as XML: in the node, and in the module store so saving finds nothing to change.
pub(crate) fn set_raw_property(world: &mut dyn DialogWorld, node: NodeKey, index: usize, element: XElement) {
    let (type_id, instance_id) = {
        let p = &mut world.properties_mut(node)[index];
        p.data = PropertyData::Unknown { raw_xml: vixen_io::xml::write_fragment(&element) };
        (p.type_id, p.instance_id)
    };
    world.set_property_store_entry(&type_id, &instance_id, element);
}

/// The XML of a property kept only as XML, when there is any.
pub(crate) fn raw_property(world: &dyn DialogWorld, node: NodeKey, index: usize) -> Option<XElement> {
    match &world.properties(node)[index].data {
        PropertyData::Unknown { raw_xml } if !raw_xml.is_empty() => vixen_io::xml::parse_fragment(raw_xml),
        _ => None,
    }
}

/// F3 after a property's own dialog: sets the data on the node it was opened for, then copies it to the same property on every other
/// selected element that has one.
pub fn apply_to_selection(world: &mut dyn DialogWorld, primary: NodeKey, selected: &[NodeKey], type_id: &Uuid, data: &PropertyData) {
    let index = ensure_property(world, primary, type_id);
    world.properties_mut(primary)[index].data = data.clone();
    for &node in selected.iter().filter(|n| **n != primary) {
        if let Some(p) = world.properties_mut(node).iter_mut().find(|p| p.type_id == *type_id) {
            p.data = data.clone();
        }
    }
}

// ---- Location (F181, F182) ------------------------------------------------------------------------------------------------

/// `(x, y, z)`. Negative values load as 0, which reading the file has already done.
pub fn location_values(data: Option<&PropertyData>) -> (i32, i32, i32) {
    match data {
        // `LocationData` reads a negative as 0.
        Some(PropertyData::Location { x, y, z }) => ((*x).max(0), (*y).max(0), (*z).max(0)),
        _ => (1, 1, 1),
    }
}

/// F182: upstream's up/down allows far more, then throws reading anything above `Int32.MaxValue`.
pub fn clamp_location(value: i64) -> i32 {
    value.clamp(0, i32::MAX as i64) as i32
}

// ---- Orientation (F179, F180) ---------------------------------------------------------------------------------------------

pub const ORIENTATIONS: [&str; 2] = ["Horizontal", "Vertical"];

pub fn orientation_name(o: Orientation) -> &'static str {
    match o {
        Orientation::Horizontal => "Horizontal",
        Orientation::Vertical => "Vertical",
    }
}

pub fn orientation_from_name(name: &str) -> Orientation {
    if name == "Horizontal" {
        Orientation::Horizontal
    } else {
        Orientation::Vertical
    }
}

/// F180: the helper sets or adds the property on the selected nodes themselves, not their leaves.
pub fn orientation_helper_apply(world: &mut dyn DialogWorld, nodes: &[NodeKey], value: Orientation) {
    for &node in nodes {
        let index = ensure_property(world, node, &property_ids::ORIENTATION);
        world.properties_mut(node)[index].data = PropertyData::Orientation(value);
    }
}

// ---- Grid (F183) ---------------------------------------------------------------------------------------------------------------

const GRID_NS: &str = "VixenModules.Property.Grid";

/// `Data`, keeping any member it doesn't know about.
fn grid_xml(old: Option<XElement>, type_id: &Uuid, instance_id: &Uuid, width: i32, height: i32) -> XElement {
    let ns = contract_ns(GRID_NS);
    let mut e = old.unwrap_or_else(|| {
        let mut e = XElement::new(&ns, "Data");
        e.push(XElement::with_text("", "ModuleInstanceId", instance_id.to_string()));
        e.push(XElement::with_text("", "ModuleTypeId", type_id.to_string()));
        e.push(XElement::new(&ns, "Height"));
        e.push(XElement::new(&ns, "Width"));
        e
    });
    for (member, value) in [("Height", height), ("Width", width)] {
        match e.child_mut(member) {
            Some(x) => *x = XElement::with_text(&ns, member, value.to_string()),
            None => {
                e.push(XElement::with_text(&ns, member, value.to_string()));
            }
        }
    }
    e
}

fn write_grid(world: &mut dyn DialogWorld, node: NodeKey, index: usize, width: i32, height: i32) {
    let old = raw_property(world, node, index);
    let (type_id, instance) = {
        let p = &world.properties(node)[index];
        (p.type_id, p.instance_id)
    };
    let element = grid_xml(old, &type_id, &instance, width, height);
    set_raw_property(world, node, index, element);
}

/// `(width, height)` from a Grid property's XML, or the defaults for a node with this many elements.
pub fn grid_values(data: Option<&PropertyData>, element_count: i32) -> (i32, i32) {
    let parsed = match data {
        Some(PropertyData::Unknown { raw_xml }) => vixen_io::xml::parse_fragment(raw_xml),
        _ => None,
    };
    let member = |e: &XElement, name: &str| e.child_text(name).and_then(|t| t.parse::<i32>().ok()).unwrap_or(0);
    match parsed {
        Some(e) => (member(&e, "Width"), member(&e, "Height")),
        None => (element_count, 1),
    }
}

/// F183's red message, when `width * height` doesn't match. `None` means OK is available.
pub fn grid_check(width: i32, height: i32, required: i32) -> Option<String> {
    (width as i64 * height as i64 != required as i64).then(|| format!("The product of these needs to be {required}"))
}

/// Applies Grid's dialog to the node it was opened for, and copies it to the other selected elements that have one. Upstream's Grid has
/// no `CloneValues`, so there the others reset to their defaults (F3).
pub fn grid_apply(world: &mut dyn DialogWorld, primary: NodeKey, selected: &[NodeKey], width: i32, height: i32) {
    let index = ensure_property(world, primary, &property_ids::GRID);
    write_grid(world, primary, index, width, height);
    for &node in selected.iter().filter(|n| **n != primary) {
        if let Some(index) = world.properties(node).iter().position(|p| p.type_id == property_ids::GRID) {
            write_grid(world, node, index, width, height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::flow::test_support::*;
    use super::super::world::{property_data, SystemWorld};
    use super::*;
    use vixen_io::SystemData;

    #[test]
    fn each_type_routes_to_its_sheet() {
        assert_eq!(property_setup_kind(&property_ids::COLOR), PropertySetupKind::Color);
        assert_eq!(property_setup_kind(&property_ids::GRID), PropertySetupKind::Grid);
        assert_eq!(property_setup_kind(&property_ids::INTELLIGENT_FIXTURE), PropertySetupKind::IntelligentFixture);
        assert_eq!(property_setup_kind(&Uuid::nil()), PropertySetupKind::None);
    }

    #[test]
    fn helpers_list_in_alphabetical_order() {
        let mut sorted = SETUP_HELPERS;
        sorted.sort();
        assert_eq!(sorted, SETUP_HELPERS);
        assert_eq!(SETUP_HELPERS[0], "Color Handling");
    }

    #[test]
    fn a_new_location_is_one_one_one() {
        let (mut system, _, leaves) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        add_property(&mut world, leaves[0], &property_ids::LOCATION);
        assert_eq!(location_values(property_data(&world, leaves[0], &property_ids::LOCATION)), (1, 1, 1));
        assert_eq!(location_values(Some(&PropertyData::Location { x: -3, y: 2, z: -1 })), (0, 2, 0));
        assert_eq!(clamp_location(-5), 0);
        assert_eq!(clamp_location(99999999999999), i32::MAX);
    }

    #[test]
    fn a_new_orientation_is_vertical_and_the_helper_sets_the_nodes_themselves() {
        let (mut system, root, leaves) = system_with_leaves(2);
        let mut world = SystemWorld::new(&mut system);
        orientation_helper_apply(&mut world, &[leaves[0], root], Orientation::Horizontal);
        assert_eq!(property_data(&world, root, &property_ids::ORIENTATION), Some(&PropertyData::Orientation(Orientation::Horizontal)));
        assert_eq!(property_data(&world, leaves[1], &property_ids::ORIENTATION), None);
        assert_eq!(property_defaults(&property_ids::ORIENTATION), PropertyData::Orientation(Orientation::Vertical));
    }

    #[test]
    fn location_copies_only_to_selected_elements_that_have_it() {
        let (mut system, _, leaves) = system_with_leaves(3);
        let mut world = SystemWorld::new(&mut system);
        add_property(&mut world, leaves[0], &property_ids::LOCATION);
        add_property(&mut world, leaves[1], &property_ids::LOCATION);
        let data = PropertyData::Location { x: 3, y: 4, z: 5 };
        apply_to_selection(&mut world, leaves[0], &leaves, &property_ids::LOCATION, &data);
        assert_eq!(property_data(&world, leaves[0], &property_ids::LOCATION), Some(&data));
        assert_eq!(property_data(&world, leaves[1], &property_ids::LOCATION), Some(&data));
        assert_eq!(property_data(&world, leaves[2], &property_ids::LOCATION), None);
    }

    #[test]
    fn grid_check_matches_upstream() {
        assert_eq!(grid_check(4, 3, 12), None);
        assert_eq!(grid_check(4, 2, 12).as_deref(), Some("The product of these needs to be 12"));
    }

    #[test]
    fn a_new_grid_is_one_row_of_every_element_and_copies_to_others() {
        let mut system = SystemData::default();
        let a = key(system.tree.add_string("A", 12));
        let b = key(system.tree.add_string("B", 12));
        {
            let mut world = SystemWorld::new(&mut system);
            add_property(&mut world, a, &property_ids::GRID);
            assert_eq!(grid_values(property_data(&world, a, &property_ids::GRID), 12), (12, 1));
            add_property(&mut world, b, &property_ids::GRID);
            grid_apply(&mut world, a, &[a, b], 4, 3);
            for n in [a, b] {
                assert_eq!(grid_values(property_data(&world, n, &property_ids::GRID), 12), (4, 3));
            }
        }
        // The store knows it by its real name, each keeps its own ids, and a save has nothing to change.
        let tree_node = vixen_core::element::NodeId(a.0);
        let p = system.tree.property(tree_node, &property_ids::GRID).unwrap();
        let entry = system.module_store.instance(p.type_id, p.instance_id).unwrap();
        assert_eq!(entry.data_model_type(), Some("VixenModules.Property.Grid.Data, Grid"));
        assert_eq!(entry.data().unwrap().child_text("Width").as_deref(), Some("4"));
        assert_eq!(entry.data().unwrap().child_text("ModuleInstanceId"), Some(p.instance_id.to_string()));
        assert!(matches!(vixen_io::system::effective_store(&system), std::borrow::Cow::Borrowed(_)));
    }
}
