//! Display Setup's working copy as a world: what a dialog does here waits for the window's OK, and the window's Cancel drops it.

use super::store;
use super::world::{color_set_names_in, ControllerInfo, DialogWorld, NodeKey};
use crate::setup::flow::{Cid, CompKind, SourceRef};
use crate::setup::tree::Nid;
use crate::setup::{FilterEntry, SetupSession};
use uuid::Uuid;
use vixen_core::element::PropertyInstance;
use vixen_core::Color;
use vixen_io::{set_color_sets, ModuleEntry};
use vixen_output::filters::FilterConfig;
use vixen_output::modules::ControllerConfig;
use vixen_xml::XElement;

pub struct SetupWorld<'a>(pub &'a mut SetupSession);

impl SetupWorld<'_> {
    fn cid(&self, component: &Uuid) -> Option<Cid> {
        self.0.flow.find(component).filter(|c| self.0.flow.is_live(*c))
    }

    fn filter(&self, component: &Uuid) -> Option<&FilterEntry> {
        self.0.filter_for(self.cid(component)?).filter(|f| f.alive)
    }
}

impl DialogWorld for SetupWorld<'_> {
    fn find_node(&self, id: &Uuid) -> Option<NodeKey> {
        self.0.tree.find(id).map(|n| NodeKey(n.0))
    }

    fn node_id(&self, node: NodeKey) -> Uuid {
        self.0.tree.node(Nid(node.0)).id
    }

    fn node_name(&self, node: NodeKey) -> String {
        self.0.tree.node(Nid(node.0)).name.clone()
    }

    fn node_element(&self, node: NodeKey) -> Option<Uuid> {
        self.0.tree.node(Nid(node.0)).element_id
    }

    fn node_children(&self, node: NodeKey) -> Vec<NodeKey> {
        self.0.tree.node(Nid(node.0)).children.iter().map(|c| NodeKey(c.0)).collect()
    }

    fn properties(&self, node: NodeKey) -> &[PropertyInstance] {
        &self.0.tree.node(Nid(node.0)).properties
    }

    fn properties_mut(&mut self, node: NodeKey) -> &mut Vec<PropertyInstance> {
        self.0.dirty.tree = true;
        &mut self.0.tree.node_mut(Nid(node.0)).properties
    }

    fn set_property_store_entry(&mut self, type_id: &Uuid, instance_id: &Uuid, data: XElement) {
        self.0.dirty.tree = true;
        match self.0.system.module_store.instance_mut(*type_id, *instance_id) {
            Some(entry) => entry.set_data(data),
            None => {
                if let Some((model, assembly)) = store::property_module_names(type_id) {
                    self.0.system.module_store.set_instance(ModuleEntry::new(model, type_id, instance_id, assembly, data));
                }
            }
        }
    }

    fn color_set_names(&self) -> Vec<String> {
        color_set_names_in(&self.0.system)
    }

    fn color_set(&self, name: &str) -> Option<Vec<Color>> {
        self.0.system.tree.color_sets.get(name).cloned()
    }

    fn set_color_sets(&mut self, sets: &[(String, Vec<Color>)]) {
        self.0.system.tree.color_sets = sets.iter().cloned().collect();
        set_color_sets(&mut self.0.system.module_store, sets);
        self.0.dirty.tree = true;
    }

    fn filter_type(&self, component: &Uuid) -> Option<Uuid> {
        self.filter(component).map(|f| f.type_id)
    }

    fn filter_config(&self, component: &Uuid) -> Option<FilterConfig> {
        self.filter(component)?.config.clone()
    }

    fn set_filter_config(&mut self, config: FilterConfig) {
        if let Some(cid) = self.cid(&config.ids().instance_id) {
            self.0.set_filter_config(cid, config);
        }
    }

    fn add_filter(&mut self, config: FilterConfig) {
        let ids = *config.ids();
        let cid = self.0.flow.add_component(ids.instance_id, CompKind::Filter, config.output_names().len() as u32);
        self.0.filter_lookup.insert(cid, self.0.filters.len());
        self.0.filters.push(FilterEntry { cid, type_id: ids.type_id, instance_id: ids.instance_id, config: Some(config), changed: true, alive: true });
        self.0.dirty.filters = true;
        self.0.bump_graph();
    }

    fn output_count(&self, component: &Uuid) -> usize {
        self.cid(component).map_or(0, |c| self.0.flow.comp(c).output_count as usize)
    }

    fn destinations(&self, component: &Uuid, output: i32) -> Vec<Uuid> {
        let Some(c) = self.cid(component) else { return Vec::new() };
        self.0.flow.destinations_of_output(c, output).map(|d| self.0.flow.comp(d).id).collect()
    }

    fn set_source(&mut self, component: Uuid, source: Uuid, output: i32) {
        let (Some(component), Some(source)) = (self.cid(&component), self.cid(&source)) else { return };
        if self.0.flow.set_source(component, SourceRef { component: source, output }) {
            self.0.patching_changed();
            self.0.bump_graph();
        }
    }

    fn reset_source(&mut self, component: &Uuid) {
        let Some(c) = self.cid(component) else { return };
        if self.0.flow.source(c).is_some() {
            self.0.flow.reset_source(c);
            self.0.patching_changed();
            self.0.bump_graph();
        }
    }

    fn controllers(&self) -> Vec<ControllerInfo> {
        self.0
            .controller_order
            .iter()
            .map(|slot| &self.0.controllers[*slot as usize])
            .filter(|c| c.alive)
            .map(|c| ControllerInfo { id: c.id, name: c.name.clone(), type_id: c.type_id, output_count: c.outputs.len() as u32, running: c.running })
            .collect()
    }

    fn controller_config(&self, id: &Uuid) -> Option<ControllerConfig> {
        self.0.controllers[self.0.slot_of(*id)? as usize].config.clone()
    }

    fn set_controller_config(&mut self, id: &Uuid, config: ControllerConfig) {
        let Some(slot) = self.0.slot_of(*id) else { return };
        let controller = &mut self.0.controllers[slot as usize];
        controller.config = Some(config);
        controller.config_changed = true;
        self.0.dirty.controllers = true;
        self.0.bump_graph();
    }

    fn set_output_names(&mut self, id: &Uuid, names: &[String]) {
        let Some(slot) = self.0.slot_of(*id) else { return };
        for (output, name) in self.0.controllers[slot as usize].outputs.iter_mut().zip(names) {
            output.name.clone_from(name);
        }
        self.0.dirty.controllers = true;
        self.0.bump_graph();
    }
}

#[cfg(test)]
mod tests {
    use super::super::properties::add_property;
    use super::*;
    use crate::app::AppCore;
    use crate::setup::test_support::{add_breakdown, session, system};
    use std::collections::HashSet;
    use vixen_core::element::{property_ids, ElementColorType, PropertyData};
    use vixen_output::filters::{ColorBreakdownConfig, ColorBreakdownItem};
    use vixen_output::modules::RenderStyle;
    use vixen_output::DotNetColor;

    fn leaf(s: &SetupSession, name: &str) -> NodeKey {
        let id = s.tree.live_nodes().find(|(_, n)| n.name == name).unwrap().0;
        NodeKey(id.0)
    }

    fn temp_core(name: &str, data: vixen_io::SystemData) -> (AppCore, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("vixen-dialogs-world-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let core = AppCore::open(root.clone(), true).unwrap();
        *core.system.write() = data;
        (core, root)
    }

    #[test]
    fn a_property_edit_stays_in_the_copy_until_commit() {
        let (core, root) = temp_core("property", system(&["A"], 0));
        let mut s = SetupSession::open(&core);
        let node = leaf(&s, "A");
        {
            let mut world = SetupWorld(&mut s);
            add_property(&mut world, node, &property_ids::COLOR);
            let p = world.properties_mut(node).iter_mut().find(|p| p.type_id == property_ids::COLOR).unwrap();
            p.data = PropertyData::Color {
                color_type: ElementColorType::SingleColor,
                single_color: Color::RED,
                color_set_name: None,
            };
        }
        assert!(s.is_dirty());
        assert!(core.system.read().tree.all_nodes().all(|(_, n)| n.properties.is_empty()));

        assert!(s.commit(&core, |_, _| {}).unwrap());
        let saved = vixen_io::load_system(&core.paths).unwrap();
        let (_, node) = saved.tree.all_nodes().find(|(_, n)| n.name == "A").unwrap();
        assert!(matches!(
            node.properties[0].data,
            PropertyData::Color { color_type: ElementColorType::SingleColor, single_color: Color::RED, .. }
        ));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn dropping_the_copy_loses_everything_a_dialog_did() {
        let (core, root) = temp_core("cancel", system(&["A"], 2));
        let before = core.system.read().clone();
        let controller = before.controllers[0].id;
        {
            let mut s = SetupSession::open(&core);
            let node = leaf(&s, "A");
            let mut world = SetupWorld(&mut s);
            add_property(&mut world, node, &property_ids::ORDER);
            world.set_output_names(&controller, &["One".to_string()]);
            world.set_color_sets(&[("Set".to_string(), vec![Color::RED])]);
        }
        let after = core.system.read();
        assert_eq!(after.controllers[0].outputs[0].name, before.controllers[0].outputs[0].name);
        assert_eq!(after.tree.color_sets, before.tree.color_sets);
        assert!(!core.paths.system_config_file().exists());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_filter_edit_reaches_the_file_and_drops_what_was_on_a_removed_output() {
        let (core, root) = temp_core("filter", system(&["A"], 3));
        let mut s = SetupSession::open(&core);
        let element = s.flow.find(&s.tree.node(Nid(leaf(&s, "A").0)).element_id.unwrap()).unwrap();
        let filter = add_breakdown(&mut s, element, 3);
        let filter_id = s.flow.comp(filter).id;
        let outputs: Vec<Uuid> = s.controllers[0].outputs.iter().map(|o| o.id).collect();
        let mut world = SetupWorld(&mut s);
        for (i, output) in outputs.iter().enumerate() {
            world.set_source(*output, filter_id, i as i32);
        }
        assert_eq!(world.destinations(&filter_id, 2), [outputs[2]]);

        let Some(FilterConfig::ColorBreakdown(mut config)) = world.filter_config(&filter_id) else { panic!() };
        config.breakdown_items.truncate(2);
        config.breakdown_items[0] = ColorBreakdownItem { color: DotNetColor::from_color(Color::RED), name: "Renamed".to_string() };
        world.set_filter_config(FilterConfig::ColorBreakdown(config));
        assert_eq!(world.output_count(&filter_id), 2);
        assert!(world.destinations(&filter_id, 2).is_empty());
        assert_eq!(world.destinations(&filter_id, 1), [outputs[1]]);

        assert!(s.commit(&core, |_, _| {}).unwrap());
        let saved = vixen_io::load_system(&core.paths).unwrap();
        let entry = saved.module_store.instance(vixen_output::ids::COLOR_BREAKDOWN, filter_id).unwrap();
        let config = ColorBreakdownConfig::from_xml(entry.data().unwrap());
        assert_eq!(config.breakdown_items.len(), 2);
        assert_eq!(config.breakdown_items[0].name, "Renamed");
        assert!(!saved.patches.iter().any(|p| p.component_id == outputs[2] && p.source_id.is_some()));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_controller_edit_and_its_output_names_reach_the_file() {
        let (core, root) = temp_core("controller", system(&[], 2));
        let id = core.system.read().controllers[0].id;
        let mut s = SetupSession::open(&core);
        let mut world = SetupWorld(&mut s);
        let info = world.controllers();
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].output_count, 2);
        let Some(ControllerConfig::DummyLighting(mut config)) = world.controller_config(&id) else { panic!() };
        config.render_style = RenderStyle::RGBSingleChannel;
        world.set_controller_config(&id, ControllerConfig::DummyLighting(config));
        world.set_output_names(&id, &["First".to_string(), "Second".to_string()]);

        assert!(s.commit(&core, |_, _| {}).unwrap());
        let saved = vixen_io::load_system(&core.paths).unwrap();
        assert_eq!(saved.controllers[0].outputs[1].name, "Second");
        let reloaded = SetupSession::from_system(saved, &HashSet::new(), false);
        let Some(ControllerConfig::DummyLighting(config)) = reloaded.controllers[0].config.clone() else { panic!() };
        assert_eq!(config.render_style, RenderStyle::RGBSingleChannel);
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn nothing_is_dirty_until_a_dialog_writes() {
        let mut s = session(&["A"], 1);
        let node = leaf(&s, "A");
        {
            let world = SetupWorld(&mut s);
            assert_eq!(world.node_name(node), "A");
            assert_eq!(world.controllers().len(), 1);
            let _ = world.color_set_names();
        }
        assert!(!s.is_dirty());
    }
}
