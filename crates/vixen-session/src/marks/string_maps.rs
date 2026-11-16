//! Old LipSync string maps become Face properties when the profile loads, `LipSyncMapStaticData.MigrateMaps`. Spec F113.

use super::map_library::LipSyncLibrary;
use crate::dialogs::face::{COMPONENT_ITEM, FACE_NS, PHONEME_ITEM};
use crate::dialogs::properties::{ensure_property, raw_property, set_id_members, set_raw_property};
use crate::dialogs::world::{DialogWorld, SystemWorld};
use uuid::Uuid;
use vixen_core::element::property_ids;
use vixen_core::Color;
use vixen_io::system::SystemData;
use vixen_io::values::DotNetColor;
use vixen_xml::{contract_ns, XElement, ARRAYS_NS};

const LOGGER: &str = "LipSyncMapStaticData";

fn pair(item: &str, key: &str, value: bool) -> XElement {
    let mut kv = XElement::new(ARRAYS_NS, item);
    kv.push(XElement::with_text(ARRAYS_NS, "Key", key));
    kv.push(XElement::with_text(ARRAYS_NS, "Value", if value { "true" } else { "false" }));
    kv
}

/// A dictionary member's pairs. GUARD: of two keys that differ only by case the first stays, where upstream's case-insensitive copy
/// throws and takes the whole module's load with it.
fn pairs(list: Option<&XElement>) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for kv in list.into_iter().flat_map(|l| l.elements()) {
        let Some(key) = kv.child_text("Key") else { continue };
        if out.iter().any(|(k, _)| k.eq_ignore_ascii_case(&key)) {
            continue;
        }
        out.push((key, kv.child_text("Value").is_some_and(|v| v.trim() == "true")));
    }
    out
}

fn replace_member(e: &mut XElement, value: XElement) {
    match e.child_mut(&value.local) {
        Some(existing) => *existing = value,
        None => {
            e.push(value);
        }
    }
}

/// One `LipSyncMapItem` onto its element. False when the element is gone.
fn migrate_item(world: &mut dyn DialogWorld, item: &XElement) -> bool {
    let Some(node) = item.child_text("ElementGuid").and_then(|t| Uuid::parse_str(t.trim()).ok()).and_then(|id| world.find_node(&id))
    else {
        return false;
    };
    let ns = contract_ns(FACE_NS);
    let index = ensure_property(world, node, &property_ids::FACE);
    let instance = world.properties(node)[index].instance_id;
    let mut face = raw_property(world, node, index).unwrap_or_else(|| {
        let mut e = XElement::new(&ns, "FaceData");
        e.push(XElement::with_text("", "ModuleInstanceId", instance.to_string()));
        e.push(XElement::with_text("", "ModuleTypeId", property_ids::FACE.to_string()));
        e
    });

    let color = item.child("ElementColor").and_then(DotNetColor::from_xml).unwrap_or_else(|| DotNetColor::from_color(Color::EMPTY));
    let phonemes = pairs(item.child("PhonemeList"));
    // `OnDeserialized`: an item from before face components has a mouth if any phoneme lights it.
    let components = match item.child("FaceComponents").filter(|c| !c.nil) {
        Some(list) => pairs(Some(list)),
        None if phonemes.iter().any(|(_, on)| *on) => vec![("Mouth".to_string(), true)],
        None => Vec::new(),
    };

    replace_member(&mut face, color.to_xml(&ns, "DefaultColor"));
    let mut list = XElement::new(&ns, "FaceComponents");
    for (key, on) in &components {
        list.push(pair(COMPONENT_ITEM, key, *on));
    }
    replace_member(&mut face, list);
    let mut list = XElement::new(&ns, "PhonemeList");
    for (key, on) in &phonemes {
        list.push(pair(PHONEME_ITEM, key, *on));
    }
    replace_member(&mut face, list);
    set_id_members(&mut face, &instance, &property_ids::FACE);
    set_raw_property(world, node, index, face);
    true
}

impl LipSyncLibrary {
    /// `MigrateMaps`: with any string map in the library, each one's items become Face properties on their elements and the map goes;
    /// the matrix maps lose their leftover `MapItems`. Returns how many maps went. KEEP: the library can end up empty, and effects
    /// that named a map which went draw the default from then on.
    pub fn migrate_string_maps(&mut self, world: &mut dyn DialogWorld) -> usize {
        if !self.has_string_maps() {
            return 0;
        }
        let mut migrated = 0;
        for name in self.names() {
            let Some(items) = self.take_for_migration(&name) else { continue };
            crate::logging::info(LOGGER, &format!("Migrating map {name} to Face properties."));
            // GUARD: a string map with no `MapItems` at all just goes, where upstream throws on the null list.
            for item in items.iter().flat_map(|l| l.elements()) {
                migrate_item(world, item);
            }
            migrated += 1;
        }
        migrated
    }
}

/// The profile has just loaded. Nothing is saved, as upstream: the change waits in memory for the next save of the module store.
pub fn migrate_string_maps_at_load(system: &mut SystemData) -> usize {
    let mut library = LipSyncLibrary::from_store(&system.module_store);
    if !library.has_string_maps() {
        return 0;
    }
    let migrated = library.migrate_string_maps(&mut SystemWorld::new(system));
    library.write_to(&mut system.module_store);
    migrated
}

#[cfg(test)]
mod tests {
    use super::*;
    use fx_audio_state::property_data::{FaceComponent, FaceData};
    use vixen_io::module_store::load_module_store_file;

    const DRAWING: &str = "http://schemas.datacontract.org/2004/07/System.Drawing";

    fn item(element: &Uuid, argb: u32, phonemes: &[(&str, bool)], components: Option<&[(&str, bool)]>) -> String {
        let pairs = |name: &str, list: &[(&str, bool)]| -> String {
            list.iter().map(|(k, v)| format!("<a:{name}><a:Key>{k}</a:Key><a:Value>{v}</a:Value></a:{name}>")).collect()
        };
        let components = components
            .map(|c| format!("<FaceComponents>{}</FaceComponents>", pairs("KeyValueOfFaceComponentbooleanIiw0dy87", c)))
            .unwrap_or_default();
        format!(
            "<LipSyncMapItem><ElementColor xmlns:b=\"{DRAWING}\"><b:knownColor>0</b:knownColor><b:name i:nil=\"true\" />\
             <b:state>2</b:state><b:value>{argb}</b:value></ElementColor><ElementGuid>{element}</ElementGuid>{components}\
             <PhonemeList>{}</PhonemeList><StringNum>1</StringNum><_stringName>String 1</_stringName></LipSyncMapItem>",
            pairs("KeyValueOfstringboolean", phonemes)
        )
    }

    fn map(name: &str, is_matrix: bool, items: &str) -> String {
        format!(
            "<a:KeyValueOfstringLipSyncMapDataiatCCBjB><a:Key>{name}</a:Key><a:Value>\
             <ModuleInstanceId xmlns=\"\">00000000-0000-0000-0000-000000000000</ModuleInstanceId>\
             <ModuleTypeId xmlns=\"\">00000000-0000-0000-0000-000000000000</ModuleTypeId>\
             <IsCurrentLibraryMapping>false</IsCurrentLibraryMapping><IsDefaultMapping>false</IsDefaultMapping>\
             <IsMatrix>{is_matrix}</IsMatrix><LibraryReferenceName>{name}</LibraryReferenceName><MapItems>{items}</MapItems>\
             <Notes /><StringsAreRows>false</StringsAreRows><UsingDefaults>true</UsingDefaults></a:Value>\
             </a:KeyValueOfstringLipSyncMapDataiatCCBjB>"
        )
    }

    fn system_with(maps: &str, names: &[&str]) -> (SystemData, Vec<Uuid>) {
        let id = "a475c535-73fa-4d31-a1bc-ed2b50ad21cf";
        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<ModuleStore version=\"5\"><ModuleData>\
             <Module dataModelType=\"VixenModules.App.LipSyncApp.LipSyncMapStaticData, LipSyncApp\" moduleType=\"{id}\" \
             moduleInstance=\"{id}\" assemblyName=\"Module.App.LipSyncApp\">\
             <LipSyncMapStaticData xmlns=\"http://schemas.datacontract.org/2004/07/VixenModules.App.LipSyncApp\" \
             xmlns:i=\"http://www.w3.org/2001/XMLSchema-instance\"><ModuleInstanceId xmlns=\"\">{id}</ModuleInstanceId>\
             <ModuleTypeId xmlns=\"\">{id}</ModuleTypeId>\
             <_library xmlns:a=\"http://schemas.microsoft.com/2003/10/Serialization/Arrays\">{maps}</_library>\
             </LipSyncMapStaticData></Module></ModuleData></ModuleStore>"
        );
        let dir = std::env::temp_dir().join(format!("vixen-string-maps-{}", Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ModuleStore.xml");
        std::fs::write(&path, xml).unwrap();
        let mut system = SystemData::default();
        system.module_store = load_module_store_file(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let ids = names
            .iter()
            .map(|n| {
                let node = system.tree.add_element(n);
                system.tree.node(node).id
            })
            .collect();
        (system, ids)
    }

    fn face(system: &SystemData, node: &Uuid) -> Option<FaceData> {
        FaceData::for_node(&system.tree, system.tree.find(node)?)
    }

    #[test]
    fn nothing_happens_without_a_string_map() {
        let mut system = SystemData::default();
        assert_eq!(migrate_string_maps_at_load(&mut system), 0);
        let (mut system, _) = system_with(&map("Pictures", true, &item(&Uuid::nil(), 0, &[], Some(&[]))), &[]);
        let before = LipSyncLibrary::from_store(&system.module_store);
        assert_eq!(migrate_string_maps_at_load(&mut system), 0);
        // The matrix map keeps its leftover item: upstream only clears it when a string map made the migration run.
        let after = LipSyncLibrary::from_store(&system.module_store);
        assert_eq!(after.names(), before.names());
        let entry = system.module_store.type_data(super::super::map_library::LIP_SYNC_MAP_TYPE_ID).unwrap();
        let text = vixen_io::xml::write_fragment(entry.data().unwrap());
        assert!(text.contains("LipSyncMapItem"));
    }

    #[test]
    fn a_string_map_becomes_face_properties_and_leaves_the_library() {
        let placeholder = Uuid::new_v4();
        let (mut system, ids) = system_with("", &["Mouth Top", "Eyes", "Old"]);
        let gone = Uuid::new_v4();
        let items = [
            item(&ids[0], 0xFFFF0000, &[("AI", true), ("O", false), ("rest", true)], Some(&[("Mouth", true), ("Outlines", false)])),
            item(&ids[1], 0xFF00FF00, &[("AI", false)], Some(&[("EyesOpen", true)])),
            item(&ids[2], 0xFF0000FF, &[("E", true)], None),
            item(&gone, 0xFFFFFFFF, &[("E", true)], None),
        ]
        .concat();
        let maps = [map("Pictures", true, &item(&placeholder, 0, &[], Some(&[]))), map("Singing Tree", false, &items)].concat();
        let (store_only, _) = system_with(&maps, &[]);
        system.module_store = store_only.module_store;

        assert_eq!(migrate_string_maps_at_load(&mut system), 1);

        let top = face(&system, &ids[0]).unwrap();
        assert_eq!(top.default_color, Color::rgb(255, 0, 0));
        assert!(top.phoneme_state("AI") && top.phoneme_state("REST") && !top.phoneme_state("O") && !top.phoneme_state("E"));
        assert!(top.is_face_component(FaceComponent::Mouth) && !top.is_face_component(FaceComponent::Outlines));
        let eyes = face(&system, &ids[1]).unwrap();
        assert_eq!(eyes.default_color, Color::rgb(0, 255, 0));
        assert!(eyes.is_face_component(FaceComponent::EyesOpen) && !eyes.is_face_component(FaceComponent::Mouth));
        // An item from before face components: a mouth, because a phoneme lights it.
        let old = face(&system, &ids[2]).unwrap();
        assert!(old.is_face_component(FaceComponent::Mouth) && old.phoneme_state("e"));

        let library = LipSyncLibrary::from_store(&system.module_store);
        assert_eq!(library.names(), ["Pictures"]);
        let entry = system.module_store.type_data(super::super::map_library::LIP_SYNC_MAP_TYPE_ID).unwrap();
        let text = vixen_io::xml::write_fragment(entry.data().unwrap());
        assert!(!text.contains("LipSyncMapItem"), "the matrix map's leftover items go when the migration runs");

        // The property is in the module store too, where a save and the next load find it.
        let node = system.tree.node(system.tree.find(&ids[0]).unwrap());
        let property = node.properties.iter().find(|p| p.type_id == property_ids::FACE).unwrap();
        assert!(system.module_store.instance_entries().iter().any(|e| e.module_instance() == Some(property.instance_id)));
        // A second load has nothing left to do.
        assert_eq!(migrate_string_maps_at_load(&mut system), 0);
    }

    #[test]
    fn an_element_that_already_has_a_face_keeps_its_instance() {
        let (mut system, ids) = system_with("", &["A"]);
        let first = map("One", false, &item(&ids[0], 0xFFFF0000, &[("AI", true)], Some(&[])));
        let (store_only, _) = system_with(&first, &[]);
        system.module_store = store_only.module_store;
        assert_eq!(migrate_string_maps_at_load(&mut system), 1);
        let instance = system.tree.node(system.tree.find(&ids[0]).unwrap()).properties[0].instance_id;

        let second = map("Two", false, &item(&ids[0], 0xFF00FF00, &[("O", true)], Some(&[("Outlines", true)])));
        let (store_only, _) = system_with(&second, &[]);
        LipSyncLibrary::from_store(&store_only.module_store).write_to(&mut system.module_store);
        assert_eq!(migrate_string_maps_at_load(&mut system), 1);
        let node = system.tree.node(system.tree.find(&ids[0]).unwrap());
        assert_eq!(node.properties.len(), 1);
        assert_eq!(node.properties[0].instance_id, instance);
        let face = face(&system, &ids[0]).unwrap();
        assert!(face.phoneme_state("O") && !face.phoneme_state("AI") && face.is_face_component(FaceComponent::Outlines));
        assert_eq!(face.default_color, Color::rgb(0, 255, 0));
        // The library is allowed to end up empty.
        assert!(LipSyncLibrary::from_store(&system.module_store).is_empty());
    }
}
