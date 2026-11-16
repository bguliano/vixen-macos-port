//! The Face Mapping helper (F184 to F192).

use super::properties::{ensure_property, raw_property, set_id_members, set_raw_property};
use super::world::{valid_colors, DialogWorld, NodeKey};
use uuid::Uuid;
use vixen_core::element::property_ids;
use vixen_core::Color;
use vixen_io::values::DotNetColor;
use vixen_xml::{contract_ns, XElement, ARRAYS_NS};

pub(crate) const FACE_NS: &str = "VixenModules.Property.Face";

/// `Dictionary<FaceComponent, bool>`'s item. .NET ends the name with a digest of the parameters' namespaces. The same formula gives
/// the three such names in a real ModuleStore.xml, though no real file with a Face property has been seen to check this one against.
pub(crate) const COMPONENT_ITEM: &str = "KeyValueOfFaceComponentbooleanIiw0dy87";
pub(crate) const PHONEME_ITEM: &str = "KeyValueOfstringboolean";

/// The Mouth tab's columns, in order (F185).
pub const PHONEMES: [&str; 10] = ["AI", "E", "ETC", "FV", "L", "MBP", "O", "REST", "U", "WQ"];

/// The Outlines/Eyes tab's columns: the stored name, and the header (F186).
pub const COMPONENTS: [(&str, &str); 3] = [("Outlines", "Outlines"), ("EyesOpen", "Eyes Open"), ("EyesClosed", "Eyes Closed")];

/// One row of both tabs. Rows go by node, where upstream finds them by name, so two nodes with one name stay separate (F190).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceRow {
    pub node: Uuid,
    pub name: String,
    /// One per [`PHONEMES`].
    pub phonemes: Vec<bool>,
    /// One per [`COMPONENTS`].
    pub components: Vec<bool>,
    pub color: Color,
}

fn dictionary_value(list: Option<&XElement>, key: &str) -> bool {
    list.into_iter()
        .flat_map(|l| l.elements())
        .find(|kv| kv.child_text("Key").is_some_and(|k| k.trim().eq_ignore_ascii_case(key)))
        .and_then(|kv| kv.child_text("Value"))
        .is_some_and(|v| v.trim() == "true")
}

/// F189: a colour the element's Color property doesn't allow becomes its first allowed colour.
fn validate_color(world: &dyn DialogWorld, node: NodeKey, color: Color) -> Color {
    let allowed = valid_colors(world, node, false);
    match allowed.first() {
        Some(first) if !allowed.contains(&color) => *first,
        _ => color,
    }
}

/// F184 to F186: one row per node passed in, not its leaves.
pub fn face_helper_model(world: &dyn DialogWorld, nodes: &[NodeKey]) -> Vec<FaceRow> {
    nodes
        .iter()
        .map(|&node| {
            let index = world.properties(node).iter().position(|p| p.type_id == property_ids::FACE);
            let xml = index.and_then(|i| raw_property(world, node, i));
            let phoneme_list = xml.as_ref().and_then(|e| e.child("PhonemeList"));
            let component_list = xml.as_ref().and_then(|e| e.child("FaceComponents"));
            let color = xml
                .as_ref()
                .and_then(|e| e.child("DefaultColor"))
                .and_then(DotNetColor::from_xml)
                .map_or(Color::WHITE, |c| c.to_color());
            FaceRow {
                node: world.node_id(node),
                name: world.node_name(node),
                phonemes: PHONEMES.iter().map(|p| dictionary_value(phoneme_list, p)).collect(),
                components: COMPONENTS.iter().map(|(key, _)| dictionary_value(component_list, key)).collect(),
                color: validate_color(world, node, color),
            }
        })
        .collect()
}

/// F187: a click on a checkbox sets every selected checkbox cell to the same value. It's on when unchecked cells are the majority or
/// tied.
pub fn face_toggle(cells: &[bool]) -> bool {
    let bias: i64 = cells.iter().map(|on| if *on { 1 } else { -1 }).sum();
    bias <= 0
}

/// F188: empty means the full colour picker. Otherwise, the discrete picker offers these.
pub fn face_color_choices(world: &dyn DialogWorld, nodes: &[NodeKey]) -> Vec<Color> {
    let mut out: Vec<Color> = Vec::new();
    for node in nodes {
        for c in valid_colors(world, *node, true) {
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}

fn dictionary(ns: &str, member: &str, item: &str, entries: impl Iterator<Item = (String, bool)>) -> XElement {
    let mut list = XElement::new(ns, member);
    for (key, value) in entries {
        let mut kv = XElement::new(ARRAYS_NS, item);
        kv.push(XElement::with_text(ARRAYS_NS, "Key", key));
        kv.push(XElement::with_text(ARRAYS_NS, "Value", if value { "true" } else { "false" }));
        list.push(kv);
    }
    list
}

fn replace_member(e: &mut XElement, local: &str, value: XElement) {
    match e.child_mut(local) {
        Some(existing) => *existing = value,
        None => {
            e.push(value);
        }
    }
}

/// The name a file already uses for a dictionary's items wins.
fn item_name(old: Option<&XElement>, member: &str, default: &str) -> String {
    old.and_then(|e| e.child(member)).and_then(|m| m.elements().next()).map_or(default.to_string(), |kv| kv.local.clone())
}

/// `FaceData`, in the order `DataContractSerializer` writes it.
fn face_xml(old: Option<XElement>, instance_id: &Uuid, row: &FaceRow, color: Color) -> XElement {
    let ns = contract_ns(FACE_NS);
    let component_item = item_name(old.as_ref(), "FaceComponents", COMPONENT_ITEM);
    let phoneme_item = item_name(old.as_ref(), "PhonemeList", PHONEME_ITEM);
    // Upstream works this out from a brand-new empty item, so it's always false. Nothing reads it either way.
    let mouth = row.phonemes.iter().any(|on| *on);
    let components = std::iter::once(("Mouth".to_string(), mouth))
        .chain(COMPONENTS.iter().zip(&row.components).map(|((key, _), on)| (key.to_string(), *on)));
    let phonemes = PHONEMES.iter().zip(&row.phonemes).map(|(key, on)| (key.to_string(), *on));
    let mut e = old.unwrap_or_else(|| {
        let mut e = XElement::new(&ns, "FaceData");
        e.push(XElement::with_text("", "ModuleInstanceId", instance_id.to_string()));
        e.push(XElement::with_text("", "ModuleTypeId", property_ids::FACE.to_string()));
        e.push(XElement::new(&ns, "DefaultColor"));
        e.push(XElement::new(&ns, "FaceComponents"));
        e.push(XElement::new(&ns, "PhonemeList"));
        e
    });
    // A colour that hasn't changed keeps what the file said about it.
    let same = e.child("DefaultColor").and_then(DotNetColor::from_xml).is_some_and(|c| c.to_color() == color);
    if !same {
        replace_member(&mut e, "DefaultColor", DotNetColor::from_color(color).to_xml(&ns, "DefaultColor"));
    }
    replace_member(&mut e, "FaceComponents", dictionary(&ns, "FaceComponents", &component_item, components));
    replace_member(&mut e, "PhonemeList", dictionary(&ns, "PhonemeList", &phoneme_item, phonemes));
    set_id_members(&mut e, instance_id, &property_ids::FACE);
    e
}

/// F190: gets or adds a Face property on each row's node and fills it in.
pub fn face_helper_apply(world: &mut dyn DialogWorld, rows: &[FaceRow]) {
    for row in rows {
        let Some(node) = world.find_node(&row.node) else {
            continue;
        };
        let index = ensure_property(world, node, &property_ids::FACE);
        let old = raw_property(world, node, index);
        let instance = world.properties(node)[index].instance_id;
        let color = validate_color(world, node, row.color);
        set_raw_property(world, node, index, face_xml(old, &instance, row, color));
    }
}

#[cfg(test)]
mod tests {
    use super::super::flow::test_support::*;
    use super::super::properties::add_property;
    use super::super::world::SystemWorld;
    use super::*;
    use vixen_core::element::{ElementColorType, PropertyData};

    #[test]
    fn the_toggle_follows_the_majority_rule() {
        assert!(!face_toggle(&[true, true, false]));
        assert!(face_toggle(&[true, false]));
        assert!(face_toggle(&[false]));
        assert!(!face_toggle(&[true]));
    }

    #[test]
    fn a_new_row_is_all_off_and_white() {
        let (mut system, _, leaves) = system_with_leaves(1);
        let world = SystemWorld::new(&mut system);
        let rows = face_helper_model(&world, &leaves);
        assert_eq!(rows[0].phonemes, vec![false; 10]);
        assert_eq!(rows[0].components, vec![false; 3]);
        assert_eq!(rows[0].color, Color::WHITE);
    }

    #[test]
    fn a_colour_the_element_does_not_allow_becomes_its_first() {
        let (mut system, _, leaves) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        let i = add_property(&mut world, leaves[0], &property_ids::COLOR);
        world.properties_mut(leaves[0])[i].data = PropertyData::Color {
            color_type: ElementColorType::MultipleDiscreteColors,
            single_color: Color::EMPTY,
            color_set_name: Some("RGB".to_string()),
        };
        let rows = face_helper_model(&world, &leaves);
        assert_eq!(rows[0].color, Color::rgb(255, 0, 0));
        assert_eq!(face_color_choices(&world, &leaves).len(), 3);
    }

    #[test]
    fn apply_writes_what_the_lip_sync_effect_reads_and_reads_back() {
        let (mut system, _, leaves) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        let mut rows = face_helper_model(&world, &leaves);
        rows[0].phonemes[0] = true;
        rows[0].phonemes[7] = true;
        rows[0].components[1] = true;
        rows[0].color = Color::rgb(1, 2, 3);
        face_helper_apply(&mut world, &rows);
        assert_eq!(face_helper_model(&world, &leaves), rows);
        let xml = raw_property(&world, leaves[0], 0).unwrap();
        assert_eq!(xml.local, "FaceData");
        assert_eq!(xml.child_text("ModuleTypeId"), Some(property_ids::FACE.to_string()));
        // Mouth comes from the row's phonemes.
        assert!(dictionary_value(xml.child("FaceComponents"), "Mouth"));
        assert!(dictionary_value(xml.child("FaceComponents"), "EyesOpen"));
        assert!(!dictionary_value(xml.child("FaceComponents"), "Outlines"));
        assert_eq!(xml.child("PhonemeList").unwrap().elements().count(), 10);
        // Case doesn't matter to the keys.
        assert!(dictionary_value(xml.child("PhonemeList"), "ai"));
        let first = xml.child("FaceComponents").unwrap().elements().next().unwrap();
        assert_eq!(first.local, COMPONENT_ITEM);
        // The store knows it too, so a save finds nothing different.
        let p = &world.properties(leaves[0])[0];
        let entry = world.system.module_store.instance(p.type_id, p.instance_id).unwrap();
        assert_eq!(entry.data_model_type(), Some("VixenModules.Property.Face.FaceData, Face"));
    }

    #[test]
    fn two_nodes_with_one_name_stay_separate() {
        let (mut system, _, leaves) = system_with_leaves(2);
        let second_name = system.tree.node(vixen_core::element::NodeId(leaves[0].0)).name.clone();
        system.tree.node_mut(vixen_core::element::NodeId(leaves[1].0)).name = second_name;
        let mut world = SystemWorld::new(&mut system);
        let mut rows = face_helper_model(&world, &leaves);
        rows[1].phonemes[3] = true;
        face_helper_apply(&mut world, &rows);
        let back = face_helper_model(&world, &leaves);
        assert!(!back[0].phonemes[3]);
        assert!(back[1].phonemes[3]);
    }

    #[test]
    fn a_file_that_names_its_items_differently_keeps_them() {
        let (mut system, _, leaves) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        let i = add_property(&mut world, leaves[0], &property_ids::FACE);
        let xml = r#"<FaceData xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Property.Face"><FaceComponents xmlns:a="http://schemas.microsoft.com/2003/10/Serialization/Arrays"><a:KeyValueOfFaceComponentbooleanABC><a:Key>Outlines</a:Key><a:Value>true</a:Value></a:KeyValueOfFaceComponentbooleanABC></FaceComponents><Mystery>keep</Mystery></FaceData>"#;
        set_raw_property(&mut world, leaves[0], i, XElement::parse(xml).unwrap());
        let rows = face_helper_model(&world, &leaves);
        assert!(rows[0].components[0]);
        face_helper_apply(&mut world, &rows);
        let back = raw_property(&world, leaves[0], i).unwrap();
        assert_eq!(back.child("FaceComponents").unwrap().elements().next().unwrap().local, "KeyValueOfFaceComponentbooleanABC");
        assert_eq!(back.child_text("Mystery").as_deref(), Some("keep"));
    }
}
