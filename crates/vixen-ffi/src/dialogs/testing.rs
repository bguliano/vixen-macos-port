//! What the dialogs' test window uses, so each dialog can be opened with real data. Nothing else calls these.

use super::{no_such, parse_id, DialogContext};
use crate::core::FfiError;
use uuid::Uuid;
use vixen_core::element::property_ids;
use vixen_io::{Controller, ControllerOutput, OutputFilter};
use vixen_output::command::OutputCommand;
use vixen_output::filters::FilterConfig;
use vixen_output::ids;
use vixen_output::modules::{create_module, ControllerConfig};
use vixen_session::dialogs::properties::ensure_property;
use vixen_session::dialogs::store;
use vixen_session::dialogs::world::{CoreWorld, DialogWorld, WorldProvider};

/// A node to try a dialog on.
#[derive(uniffi::Record, Clone)]
pub struct DialogTestNode {
    pub id: String,
    pub name: String,
    pub is_leaf: bool,
    pub properties: Vec<String>,
}

#[uniffi::export]
impl DialogContext {
    /// A new controller of a module type, in the profile, with no outputs patched. Returns its id.
    pub fn test_add_controller(&self, type_id: String, name: String, outputs: u32) -> Result<String, FfiError> {
        let type_id = parse_id(&type_id)?;
        let instance = Uuid::new_v4();
        let id = Uuid::new_v4();
        let config = ControllerConfig::new_default(&type_id, instance).ok_or_else(|| no_such("controller type"))?;
        {
            let mut system = self.core.system.write();
            store::store_controller_config(&mut system.module_store, &config);
            system.controllers.push(Controller {
                name: name.clone(),
                hardware_type_id: type_id,
                hardware_instance_id: instance,
                id,
                outputs: (0..outputs)
                    .map(|i| ControllerOutput { name: format!("Output {}", i + 1), id: Uuid::new_v4() })
                    .collect(),
            });
        }
        self.core.playback.with_output(|_, runner| {
            let mut module = create_module(config);
            module.set_output_count(outputs as usize);
            runner.add_controller(id, &name, module, true);
        });
        Ok(id.to_string())
    }

    /// A new filter of a module type, with nothing patched to it. Returns its id.
    pub fn test_add_filter(&self, type_id: String) -> Result<String, FfiError> {
        let type_id = parse_id(&type_id)?;
        let instance = Uuid::new_v4();
        let config = FilterConfig::new_default(&type_id, instance).ok_or_else(|| no_such("filter type"))?;
        let mut system = self.core.system.write();
        system.filters.push(OutputFilter { type_id, instance_id: instance });
        store::store_filter_config(&mut system.module_store, &config);
        Ok(instance.to_string())
    }

    /// Nodes to try dialogs on: the roots, and the first few under each.
    pub fn test_nodes(&self) -> Vec<DialogTestNode> {
        let system = self.core.system.read();
        let mut out = Vec::new();
        for root in &system.tree.roots {
            for id in std::iter::once(root).chain(system.tree.node(*root).children.iter().take(3)) {
                let n = system.tree.node(*id);
                out.push(DialogTestNode {
                    id: n.id.to_string(),
                    name: n.name.clone(),
                    is_leaf: n.children.is_empty(),
                    properties: n.properties.iter().map(|p| p.type_id.to_string()).collect(),
                });
            }
        }
        out
    }

    /// Adds a property to a node if it hasn't got one, as Display Setup's Add does. Returns its type id.
    pub fn test_add_property(&self, node: String, kind: String) -> Result<String, FfiError> {
        let id = parse_id(&node)?;
        let type_id = match kind.as_str() {
            "Grid" => property_ids::GRID,
            "Order" => property_ids::ORDER,
            "Location" => property_ids::LOCATION,
            "Orientation" => property_ids::ORIENTATION,
            "Color" => property_ids::COLOR,
            _ => return Err(no_such("property type")),
        };
        CoreWorld(self.core.clone()).with_world(&mut |world: &mut dyn DialogWorld| {
            if let Some(node) = world.find_node(&id) {
                ensure_property(world, node, &type_id);
            }
        });
        Ok(type_id.to_string())
    }

    /// Writes the profile's files, as Display Setup's OK does.
    pub fn test_save_profile(&self) -> Result<(), FfiError> {
        let mut system = self.core.system.write();
        // Display Setup's commit does this too: a property added at its default value has no store entry yet.
        vixen_io::system::ensure_property_entries(&mut system, &Default::default());
        Ok(vixen_io::save_system(&system, &self.core.paths)?)
    }

    /// Starts one controller and gives it a frame. Only for controllers that have no hardware, whatever the app's mute says.
    pub fn test_start_monitor(&self, controller: String, value: u8) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        let outputs = self.core.system.read().controllers.iter().find(|c| c.id == id).map_or(0, |c| c.outputs.len());
        let commands: Vec<OutputCommand> = (0..outputs).map(|o| OutputCommand::U8(((o as u32 * 7) as u8).wrapping_add(value))).collect();
        self.core.playback.with_output(|_, runner| {
            let Some(i) = runner.controller_index(&id) else {
                return Err(no_such("controller"));
            };
            if !matches!(runner.config(i), Some(ControllerConfig::DummyLighting(_)) | Some(ControllerConfig::Debug(_))) {
                return Err(FfiError::new("Only controllers with no hardware can be started from the test window."));
            }
            runner.start(i);
            runner.publish_controller(i, &commands);
            Ok(())
        })
    }

    /// Stops a controller that [`DialogContext::test_start_monitor`] started.
    pub fn test_stop_monitor(&self, controller: String) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        self.core.playback.with_output(|_, runner| {
            if let Some(i) = runner.controller_index(&id) {
                runner.stop(i);
            }
        });
        Ok(())
    }

    /// The descriptors' ids and names, for the test window's list.
    pub fn test_controller_types(&self) -> Vec<String> {
        ids::CONTROLLER_DESCRIPTORS.iter().map(|d| format!("{}|{}", d.type_id, d.type_name)).collect()
    }
}

/// Opt-in observation of real engine output for an offscreen native UI test.
#[derive(uniffi::Object)]
pub struct DialogFrameProbe(vixen_session::dialogs::state_preview::FrameProbe);
#[uniffi::export]
impl DialogFrameProbe {
    pub fn snapshot(&self) -> String { self.0.snapshot() }
}
#[uniffi::export]
impl DialogContext {
    pub fn test_watch_frames(&self) -> Option<std::sync::Arc<DialogFrameProbe>> {
        vixen_session::dialogs::state_preview::FrameProbe::attach(&self.core).map(|p| std::sync::Arc::new(DialogFrameProbe(p)))
    }
}
