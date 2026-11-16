//! A Custom Prop Editor file into a preview: `VixenPreviewControl.ImportCustomProp` and the element half of `PreviewCustomPropBuilder`.

use crate::app::AppCore;
use crate::SessionError;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;
use vixen_preview::custom_prop_builder::{build_custom_prop_mapped, name_tokens, ElementModel};
use vixen_preview::Shape;
pub use vixen_prop::ColorMode;
use vixen_prop::Prop;
use crate::dialogs::properties::{ensure_property, raw_property, set_raw_property};
use crate::dialogs::world::{DialogWorld, NodeKey, SystemWorld};
use vixen_core::element::property_ids;
use vixen_core::NodeId;
use vixen_io::values::DotNetColor;
use vixen_xml::{contract_ns, XElement, ARRAYS_NS};

/// What the import reads from a prop file.
pub struct PropFile {
    pub model: ElementModel,
    /// Authored face and State metadata, whose element ids are remapped only once all display nodes exist.
    prop: Prop,
    /// `PhysicalMetadata.ColorMode`, which decides the colour question asked after the build.
    pub color_mode: ColorMode,
}

impl PropFile {
    pub fn read(path: &Path) -> Result<PropFile, SessionError> {
        // The date only stamps a legacy prop in memory, and the import never saves the prop.
        let loaded = vixen_prop::load_prop(path, "2000-01-01T00:00:00").map_err(|e| SessionError::Other(format!("{} {}", e.message, e.detail).trim().to_string()))?;
        Ok(PropFile::from_prop(&loaded.prop))
    }

    pub fn from_prop(prop: &Prop) -> PropFile {
        PropFile { model: model_of(prop, prop.root_id(), &mut Vec::new()), prop: prop.clone(), color_mode: prop.physical.color_mode }
    }

    /// The `{1}` tokens to ask for, in the order upstream asks.
    pub fn tokens(&self) -> Vec<String> {
        name_tokens(&self.model)
    }
}

/// An element shared by two groups comes out once under each, with one id, which is how the builder knows it is one element.
fn model_of(prop: &Prop, id: Uuid, path: &mut Vec<Uuid>) -> ElementModel {
    let Some(e) = prop.element(id) else {
        return ElementModel { id, ..ElementModel::default() };
    };
    path.push(id);
    let mut children = Vec::new();
    for child in e.children() {
        // A well-formed prop has no cycles. One that does loses the link that closes it.
        if !path.contains(child) {
            children.push(model_of(prop, *child, path));
        }
    }
    path.pop();
    ElementModel {
        id,
        name: e.name().to_string(),
        order: prop.order(id),
        light_size: prop.light_size(id),
        lights: if e.is_light_node() { e.lights().iter().map(|l| (l.x, l.y)).collect() } else { Vec::new() },
        children,
    }
}

pub struct BuiltProp {
    pub shape: Shape,
    pub root: Uuid,
    /// The elements that got lights, in the prop's order: what the dimming and colour questions apply to.
    pub light_nodes: Vec<Uuid>,
}

/// `PreviewCustomPropBuilder.CreateAsync` up to its questions: the prop's elements join the running display, as upstream adds them to the live
/// nodes, and nothing is saved. `answers` holds a value per token; a token with none becomes empty text, as a cancelled prompt does upstream.
pub fn build_live(core: &AppCore, file: &PropFile, zoom_level: f64, answers: &HashMap<String, String>) -> BuiltProp {
    let mut system = core.system.read().clone();
    let (shape, root, nodes) = build_custom_prop_mapped(&file.model, &mut system.tree, zoom_level, |token| answers.get(token).cloned().unwrap_or_default());
    import_properties(&mut system, &file.prop, &nodes);
    let root = system.tree.node(root).id;
    let mut seen = HashSet::new();
    let mut light_nodes = Vec::new();
    shape.for_each_pixel(&mut |p| {
        if seen.insert(p.node_id) {
            light_nodes.push(p.node_id);
        }
    });
    core.apply_display(system);
    BuiltProp { shape, root, light_nodes }
}

fn property_xml(world: &mut dyn DialogWorld, node: NodeKey, kind: Uuid, namespace: &str, name: &str) -> (usize, XElement) {
    let index = ensure_property(world, node, &kind);
    let mut data = XElement::new(&contract_ns(namespace), name);
    data.push(XElement::with_text("", "ModuleInstanceId", world.properties(node)[index].instance_id.to_string()));
    data.push(XElement::with_text("", "ModuleTypeId", kind.to_string()));
    (index, data)
}

/// `FindOrCreateElementNode`'s Face property and `AddImportedStateDefinitions`. The current .prp reader
/// normalizes legacy definitions into authored models before this point, just as upstream's persistence service does.
fn import_properties(system: &mut vixen_io::SystemData, prop: &Prop, nodes: &HashMap<Uuid, NodeId>) {
    let ids: HashMap<Uuid, Uuid> = nodes.iter().map(|(model, node)| (*model, system.tree.node(*node).id)).collect();
    let mut world = SystemWorld::new(system);
    fn visit(
        prop: &Prop, id: Uuid, nodes: &HashMap<Uuid, NodeId>, ids: &HashMap<Uuid, Uuid>,
        world: &mut dyn DialogWorld, seen: &mut HashSet<Uuid>,
    ) {
        if !seen.insert(id) { return; }
        let Some(model) = prop.element(id) else { return };
        for child in model.children() { visit(prop, *child, nodes, ids, world, seen); }
        let Some(node) = nodes.get(&id).map(|n| NodeKey(n.0)) else { return };

        // Root creation intentionally has no face handling in PreviewCustomPropBuilder.CreateAsync.
        if id != prop.root_id() && model.face.component != vixen_prop::FaceComponent::None {
            let namespace = crate::dialogs::face::FACE_NS;
            let ns = contract_ns(namespace);
            let (index, mut data) = property_xml(world, node, property_ids::FACE, namespace, "FaceData");
            data.push(DotNetColor::from_color(model.face.default_color).to_xml(&ns, "DefaultColor"));
            let mut components = XElement::new(&ns, "FaceComponents");
            let mut phonemes = XElement::new(&ns, "PhonemeList");
            let mut entry = XElement::new(ARRAYS_NS, if model.is_phoneme() {
                crate::dialogs::face::PHONEME_ITEM
            } else {
                crate::dialogs::face::COMPONENT_ITEM
            });
            entry.push(XElement::with_text(ARRAYS_NS, "Key", model.face.component.name()));
            entry.push(XElement::with_text(ARRAYS_NS, "Value", "true"));
            if model.is_phoneme() { phonemes.push(entry); } else { components.push(entry); }
            data.push(components);
            data.push(phonemes);
            set_raw_property(world, node, index, data);
        }

        let ns = contract_ns("VixenModules.Property.State");
        let mut definitions = XElement::new(&ns, "StateDefinitions");
        for definition in &model.state_definitions {
            let mut items = XElement::new(&ns, "Items");
            for item in &definition.items {
                let mut seen = HashSet::new();
                let targets: Vec<Uuid> = item.element_ids.iter().filter_map(|id| ids.get(id).copied()).filter(|id| seen.insert(*id)).collect();
                if targets.is_empty() { continue; }
                let mut data = XElement::new(&ns, "StateItemData");
                data.push(DotNetColor::from_color(item.color).to_xml(&ns, "Color"));
                let mut assigned = XElement::new(&ns, "ElementNodeIds");
                for target in targets { assigned.push(XElement::with_text(ARRAYS_NS, "guid", target.to_string())); }
                data.push(assigned);
                data.push(XElement::with_text(&ns, "Id", nonempty_id(item.id).to_string()));
                data.push(XElement::with_text(&ns, "Name", imported_name(&item.name, vixen_prop::DEFAULT_STATE_ITEM_NAME)));
                items.push(data);
            }
            if items.elements().next().is_none() { continue; }
            let mut data = XElement::new(&ns, "StateDefinitionData");
            data.push(XElement::with_text(&ns, "Description", &definition.description));
            data.push(XElement::with_text(&ns, "Id", nonempty_id(definition.id).to_string()));
            data.push(items);
            data.push(XElement::with_text(&ns, "Name", imported_name(&definition.name, vixen_prop::DEFAULT_STATE_DEFINITION_NAME)));
            definitions.push(data);
        }
        if definitions.elements().next().is_some() {
            let (index, mut data) = property_xml(world, node, property_ids::STATE, "VixenModules.Property.State", "StateData");
            data.push(XElement::with_text(&ns, "Id", nonempty_id(model.state_property_id).to_string()));
            data.push(definitions);
            set_raw_property(world, node, index, data);
        }
    }
    visit(prop, prop.root_id(), nodes, &ids, &mut world, &mut HashSet::new());
}

fn nonempty_id(id: Uuid) -> Uuid { if id.is_nil() { Uuid::new_v4() } else { id } }

fn imported_name<'a>(name: &'a str, default: &'a str) -> &'a str {
    let name = name.trim();
    if name.is_empty() { default } else { name }
}

/// `EnsureFaceMapColors`: run after a non-full-colour helper has been applied. Full-colour nodes keep
/// their authored face colour. Groups use their children's valid colours, matching ColorModule.
pub fn refresh_face_colors(core: &AppCore, root: Uuid) {
    let mut system = core.system.read().clone();
    if refresh_face_colors_in_system(&mut system, root) {
        core.apply_display(system);
    }
}

fn refresh_face_colors_in_system(system: &mut vixen_io::SystemData, root: Uuid) -> bool {
    let Some(root) = system.tree.find(&root) else { return false };
    let mut visited = HashSet::new();
    let nodes: Vec<NodeKey> = system.tree.descendants(root).into_iter().filter(|id| visited.insert(*id)).map(|id| NodeKey(id.0)).collect();
    let mut world = SystemWorld::new(system);
    let mut changed = false;
    for node in nodes {
        let Some(index) = world.properties(node).iter().position(|p| p.type_id == property_ids::FACE) else { continue };
        let Some(color) = crate::dialogs::world::valid_colors(&world, node, true).first().copied() else { continue };
        let Some(mut data) = raw_property(&world, node, index) else { continue };
        if let Some(old) = data.child_mut("DefaultColor") {
            if DotNetColor::from_xml(old).is_some_and(|old| old.to_color() == color) { continue; }
            *old = DotNetColor::from_color(color).to_xml(&contract_ns(crate::dialogs::face::FACE_NS), "DefaultColor");
        }
        set_raw_property(&mut world, node, index, data);
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star() -> Prop {
        let mut prop = Prop::new("Star {1}", "test", "2026-01-01T00:00:00");
        let arms = prop.create_node("Arms", None, true);
        let a = prop.add_light_node(Some(arms), 10.0, 20.0, None, 4, Some("Arm {1}-{2}")).unwrap();
        prop.add_light(Some(a), 30.0, 40.0, 4).unwrap();
        prop.add_light_node(Some(arms), 50.0, 60.0, None, 4, Some("Tip")).unwrap();
        prop.physical.color_mode = ColorMode::Single;
        prop
    }

    #[test]
    fn a_prop_becomes_the_builders_model() {
        let file = PropFile::from_prop(&star());
        assert_eq!(file.color_mode, ColorMode::Single);
        assert_eq!(file.tokens(), ["{1}", "{2}"]);
        assert_eq!(file.model.name, "Star {1}");
        assert!(file.model.lights.is_empty());
        let arms = &file.model.children[0];
        assert_eq!((arms.name.as_str(), arms.children.len()), ("Arms", 2));
        let arm = &arms.children[0];
        assert_eq!(arm.lights, [(10.0, 20.0), (30.0, 40.0)]);
        assert_eq!((arm.order, arm.light_size), (1, 4));
        assert_eq!(arms.children[1].order, 2);
    }

    #[test]
    fn face_and_authored_states_remap_to_imported_elements() {
        use fx_audio_state::property_data::{FaceData, StatePropertyData};
        use vixen_core::Color;
        use vixen_prop::{FaceComponent, FaceDefinition, StateDefinitionModel, StateItemModel};

        let mut prop = star();
        let root_id = prop.root_id();
        let arms = prop.root().children()[0];
        let leaves = prop.element(arms).unwrap().children().to_vec();
        prop.element_mut(root_id).unwrap().face = FaceDefinition { component: FaceComponent::E, default_color: Color::BLUE };
        prop.element_mut(arms).unwrap().face = FaceDefinition { component: FaceComponent::EyesOpen2, default_color: Color::LIME };
        prop.element_mut(leaves[0]).unwrap().face = FaceDefinition { component: FaceComponent::AI, default_color: Color::RED };
        prop.element_mut(leaves[1]).unwrap().face = FaceDefinition { component: FaceComponent::Outlines, default_color: Color::BLUE };
        let definition = StateDefinitionModel {
            id: Uuid::new_v4(), name: " Doors ".into(), description: "Imported state".into(),
            items: vec![
                StateItemModel { id: Uuid::new_v4(), name: " Open ".into(), color: Color::RED,
                    element_ids: vec![leaves[0], Uuid::new_v4(), leaves[0], leaves[1]] },
                StateItemModel { id: Uuid::new_v4(), name: "No surviving targets".into(), color: Color::BLUE, element_ids: vec![Uuid::new_v4()] },
            ],
        };
        prop.element_mut(root_id).unwrap().state_definitions = vec![definition.clone(), StateDefinitionModel::create_default("Empty")];
        let file = PropFile::from_prop(&prop);
        let mut system = vixen_io::SystemData::default();
        let (_, root, nodes) = build_custom_prop_mapped(&file.model, &mut system.tree, 1.0, |_| "1".into());
        import_properties(&mut system, &prop, &nodes);
        assert!(FaceData::for_node(&system.tree, root).is_none(), "the root's face metadata is deliberately not imported upstream");
        let numbered_eye = FaceData::for_node(&system.tree, nodes[&arms]).unwrap();
        assert!(numbered_eye.phoneme_state("EyesOpen2"), "upstream treats numbered eye components as phonemes");
        let face = FaceData::for_node(&system.tree, nodes[&leaves[0]]).unwrap();
        assert!(face.phoneme_state("AI"));
        assert_eq!(face.default_color, Color::RED);
        let state = StatePropertyData::for_node(&system.tree, root).unwrap();
        assert_eq!(state.id, prop.root().state_property_id);
        assert_eq!(state.definitions.len(), 1);
        let imported = &state.definitions[0];
        assert_eq!((imported.id, imported.name.as_str(), imported.description.as_str()), (definition.id, "Doors", "Imported state"));
        assert_eq!(imported.items.len(), 1);
        let item = &imported.items[0];
        assert_eq!((item.id, item.name.as_str(), item.color), (definition.items[0].id, "Open", Color::RED));
        assert_eq!(item.element_node_ids, leaves.iter().map(|id| system.tree.node(nodes[id]).id).collect::<Vec<_>>());
        // Every created property has a module-store entry, so preview Save can persist both the tree and data.
        for (_, node) in system.tree.all_nodes() {
            for property in &node.properties {
                if property.type_id == property_ids::FACE || property.type_id == property_ids::STATE {
                    assert!(system.module_store.instance(property.type_id, property.instance_id).is_some());
                }
            }
        }
    }

    #[test]
    fn imported_face_colors_follow_discrete_setup_but_keep_full_color() {
        use fx_audio_state::property_data::FaceData;
        use vixen_core::{Color, ElementColorType, PropertyData, PropertyInstance};
        use vixen_prop::{FaceComponent, FaceDefinition};
        let mut prop = star();
        let arms = prop.root().children()[0];
        let leaves = prop.element(arms).unwrap().children().to_vec();
        for id in [arms, leaves[0], leaves[1]] {
            prop.element_mut(id).unwrap().face = FaceDefinition { component: FaceComponent::Outlines, default_color: Color::RED };
        }
        let file = PropFile::from_prop(&prop);
        let mut system = vixen_io::SystemData::default();
        let (_, root, nodes) = build_custom_prop_mapped(&file.model, &mut system.tree, 1.0, |_| "1".into());
        import_properties(&mut system, &prop, &nodes);
        for (id, color_type) in [(leaves[0], ElementColorType::SingleColor), (leaves[1], ElementColorType::FullColor)] {
            system.tree.node_mut(nodes[&id]).properties.push(PropertyInstance {
                type_id: property_ids::COLOR, instance_id: Uuid::new_v4(),
                data: PropertyData::Color { color_type, single_color: Color::BLUE, color_set_name: None },
            });
        }
        let root_id = system.tree.node(root).id;
        assert!(refresh_face_colors_in_system(&mut system, root_id));
        for id in [arms, leaves[0]] {
            assert_eq!(FaceData::for_node(&system.tree, nodes[&id]).unwrap().default_color, Color::BLUE);
        }
        assert_eq!(FaceData::for_node(&system.tree, nodes[&leaves[1]]).unwrap().default_color, Color::RED);
        assert!(!refresh_face_colors_in_system(&mut system, root_id), "no second display apply when nothing changes");
    }

    #[test]
    fn a_saved_prop_file_reads_back_and_builds_into_the_running_display() {
        let root = std::env::temp_dir().join(format!("vixen-preview-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let core = AppCore::open(root.clone(), true).unwrap();
        let path = root.join("star.prp");
        vixen_prop::save_prop(&star(), &path, None).unwrap();
        let file = PropFile::read(&path).unwrap();
        assert_eq!(file.tokens(), ["{1}", "{2}"]);

        let before = core.system.read().tree.all_nodes().count();
        let answers = HashMap::from([("{1}".to_string(), "7".to_string())]);
        let built = build_live(&core, &file, 1.0, &answers);
        let system = core.system.read();
        assert_eq!(system.tree.all_nodes().count(), before + 4);
        let node = |id: &Uuid| system.tree.node(system.tree.find(id).unwrap());
        assert_eq!(node(&built.root).name, "Star 7");
        let names: Vec<&str> = built.light_nodes.iter().map(|id| node(id).name.as_str()).collect();
        assert_eq!(names, ["Arm 7-", "Tip"]);
        assert_eq!(built.shape.pixel_count(), 3);
        assert!(built.light_nodes.iter().all(|id| node(id).element_id.is_some()));
        drop(system);
        assert!(PropFile::read(&root.join("missing.prp")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A sample for the app's test harness: `VIXEN_PROP_OUT=/path/star.prp cargo test ... -- --ignored writes_a_sample`.
    /// `VIXEN_PROP_FULL_COLOR` makes it a full colour prop.
    #[test]
    #[ignore]
    fn writes_a_sample_prop_file() {
        let path = std::env::var("VIXEN_PROP_OUT").expect("VIXEN_PROP_OUT");
        let mut prop = star();
        if std::env::var_os("VIXEN_PROP_FULL_COLOR").is_some() {
            prop.physical.color_mode = ColorMode::FullColor;
        }
        if std::env::var_os("VIXEN_PROP_AUTHORED").is_some() {
            use vixen_prop::{FaceComponent, FaceDefinition, StateDefinitionModel, StateItemModel};
            let root = prop.root_id();
            let arms = prop.root().children()[0];
            let leaves = prop.element(arms).unwrap().children().to_vec();
            prop.element_mut(leaves[0]).unwrap().face = FaceDefinition { component: FaceComponent::AI, default_color: vixen_core::Color::BLUE };
            prop.element_mut(root).unwrap().state_definitions = vec![StateDefinitionModel {
                id: Uuid::new_v4(), name: "Doors".into(), description: "Imported authored state".into(),
                items: vec![StateItemModel { id: Uuid::new_v4(), name: "Open".into(), color: vixen_core::Color::RED, element_ids: leaves }],
            }];
        }
        vixen_prop::save_prop(&prop, std::path::Path::new(&path), None).unwrap();
    }
}
