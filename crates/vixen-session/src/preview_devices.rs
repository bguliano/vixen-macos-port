//! Previews as output devices: which run, what the Setup Previews dialog does to them, and what happens to their data. Port of
//! `ConfigPreviews` without its window, and the preview half of `PreviewManager`.
//!
//! "Running" here means the preview should be showing. Its window belongs to the preview area, which follows this state through
//! [`AppCore::subscribe_previews`] and [`AppCore::running_previews`].

use crate::app::AppCore;
use crate::logging;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use uuid::Uuid;
use vixen_io::system::Preview;
use vixen_io::{ModuleEntry, ProfileSettings, SettingSection};
use vixen_preview::{PreviewData, VIXEN_PREVIEW_MODULE_TYPE};
use vixen_xml::XNode;

const DATA_MODEL_TYPE: &str = "VixenModules.Preview.VixenPreview.VixenPreviewData, VixenPreview";
const ASSEMBLY_NAME: &str = "Module.Preview.VixenPreview";
const TYPE_NAME: &str = "Vixen Display Preview";

/// A row of Setup Previews.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewDeviceRow {
    pub id: Uuid,
    pub name: String,
    pub type_name: String,
    pub is_running: bool,
}

#[derive(Debug, Default)]
struct Devices {
    /// The manager has been started, so a new or ticked preview starts too.
    started: bool,
    /// In system order, each with whether `DisabledDevices` lets it start with the engine.
    list: Vec<(Uuid, bool)>,
    running: HashSet<Uuid>,
}

/// `OutputDeviceExecution<OutputPreview>`.
#[derive(Default)]
pub struct PreviewDevices {
    devices: Mutex<Devices>,
    listeners: Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>,
    player_listeners: Mutex<Vec<Arc<dyn Fn(bool) + Send + Sync>>>,
}

impl PreviewDevices {
    pub fn new(previews: &[Preview], disabled: &[Uuid]) -> PreviewDevices {
        let devices = PreviewDevices::default();
        devices.configure(previews, disabled);
        devices
    }

    /// Replaces the list, after a config load. Nothing is running afterwards.
    pub fn configure(&self, previews: &[Preview], disabled: &[Uuid]) {
        let mut devices = self.devices.lock();
        devices.list = previews.iter().map(|p| (p.instance_id, !disabled.contains(&p.instance_id))).collect();
        devices.running.clear();
    }

    /// `StartAll`: runs only from stopped.
    pub fn start_all(&self) {
        {
            let mut devices = self.devices.lock();
            if devices.started {
                return;
            }
            devices.started = true;
            let enabled: Vec<Uuid> = devices.list.iter().filter(|(_, enabled)| *enabled).map(|(id, _)| *id).collect();
            devices.running.extend(enabled);
        }
        self.notify();
    }

    /// `StopAll`.
    pub fn stop_all(&self) {
        {
            let mut devices = self.devices.lock();
            if !devices.started {
                return;
            }
            devices.started = false;
            devices.running.clear();
        }
        self.notify();
    }

    fn add(&self, id: Uuid) {
        {
            let mut devices = self.devices.lock();
            devices.list.push((id, true));
            if devices.started {
                devices.running.insert(id);
            }
        }
        self.notify();
    }

    fn remove(&self, id: &Uuid) {
        {
            let mut devices = self.devices.lock();
            devices.list.retain(|(i, _)| i != id);
            devices.running.remove(id);
        }
        self.notify();
    }

    pub fn is_running(&self, id: &Uuid) -> bool {
        self.devices.lock().running.contains(id)
    }

    /// A start does nothing unless the manager is running, as `OutputDeviceExecution.Start`. Returns whether it's running now.
    pub fn set_running(&self, id: &Uuid, running: bool) -> bool {
        let is_running = {
            let mut devices = self.devices.lock();
            if running {
                if devices.started && devices.list.iter().any(|(i, _)| i == id) {
                    devices.running.insert(*id);
                }
            } else {
                devices.running.remove(id);
            }
            devices.running.contains(id)
        };
        self.notify();
        is_running
    }

    /// Those not running, in system order.
    pub fn not_running(&self) -> Vec<Uuid> {
        let devices = self.devices.lock();
        devices.list.iter().filter(|(id, _)| !devices.running.contains(id)).map(|(id, _)| *id).collect()
    }

    /// Those running, in system order.
    pub fn running(&self) -> Vec<Uuid> {
        let devices = self.devices.lock();
        devices.list.iter().filter(|(id, _)| devices.running.contains(id)).map(|(id, _)| *id).collect()
    }

    /// `SaveDisabledDevices` has run: from now on the engine starts these and only these.
    pub fn apply_disabled(&self, disabled: &[Uuid]) {
        for (id, enabled) in self.devices.lock().list.iter_mut() {
            *enabled = !disabled.contains(id);
        }
    }

    pub fn subscribe(&self, listener: Arc<dyn Fn() + Send + Sync>) {
        self.listeners.lock().push(listener);
    }

    pub fn subscribe_player_active(&self, listener: Arc<dyn Fn(bool) + Send + Sync>) {
        self.player_listeners.lock().push(listener);
    }

    pub fn notify_player_active(&self, active: bool) {
        let listeners: Vec<_> = self.player_listeners.lock().clone();
        for listener in listeners {
            listener(active);
        }
    }

    fn notify(&self) {
        let listeners: Vec<_> = self.listeners.lock().clone();
        for listener in listeners {
            listener();
        }
    }
}

impl AppCore {
    /// `_PopulatePreviewList`'s rows.
    pub fn previews(&self) -> Vec<PreviewDeviceRow> {
        let devices = self.playback.preview_devices();
        self.system
            .read()
            .previews
            .iter()
            .map(|p| PreviewDeviceRow {
                id: p.instance_id,
                name: p.name.clone(),
                type_name: if p.type_id == VIXEN_PREVIEW_MODULE_TYPE { TYPE_NAME.to_string() } else { p.type_id.to_string() },
                is_running: devices.is_running(&p.instance_id),
            })
            .collect()
    }

    /// The previews that should be showing now, for the preview area to follow.
    pub fn running_previews(&self) -> Vec<Uuid> {
        self.playback.preview_devices().running()
    }

    /// Calls `listener` after any preview starts or stops, on the thread that made the change.
    pub fn subscribe_previews(&self, listener: Arc<dyn Fn() + Send + Sync>) {
        self.playback.preview_devices().subscribe(listener);
    }

    /// `IPreview.PlayerStarted` and `PlayerEnded`, for Auto On Top. Only the sequencer's play path calls
    /// [`notify_player_active`](Self::notify_player_active), as `SetPreviewsTopMost` is its only caller upstream, so a scheduled show doesn't raise a window.
    pub fn subscribe_player_active(&self, listener: Arc<dyn Fn(bool) + Send + Sync>) {
        self.playback.preview_devices().subscribe_player_active(listener);
    }

    /// The sequencer's `SetPreviewsTopMost`: playback in an editor started or ended.
    pub fn notify_player_active(&self, active: bool) {
        self.playback.preview_devices().notify_player_active(active);
    }

    /// Add New Preview: the first preview type, named after it, with a new id and new data. Starts if the engine is open. The caller
    /// opens the designer next.
    pub fn add_preview(&self) -> Uuid {
        let id = Uuid::new_v4();
        {
            let mut system = self.system.write();
            system.previews.push(Preview {
                name: TYPE_NAME.to_string(),
                type_id: VIXEN_PREVIEW_MODULE_TYPE,
                instance_id: id,
            });
            let mut data = PreviewData::new();
            data.module_instance_id = Some(id.to_string());
            data.module_type_id = Some(VIXEN_PREVIEW_MODULE_TYPE.to_string());
            let entry = ModuleEntry::new(DATA_MODEL_TYPE, &VIXEN_PREVIEW_MODULE_TYPE, &id, ASSEMBLY_NAME, data.to_xml());
            system.module_store.set_instance(entry);
        }
        self.playback.preview_devices().add(id);
        self.mark_preview_changed();
        id
    }

    /// Duplicate Selected: a new device named `{name}-copy` for each, with a clone of its module data under the new id.
    pub fn duplicate_previews(&self, ids: &[Uuid]) -> Vec<Uuid> {
        let mut made = Vec::new();
        {
            let mut system = self.system.write();
            for id in ids {
                let Some(original) = system.previews.iter().find(|p| p.instance_id == *id).cloned() else {
                    continue;
                };
                let new_id = Uuid::new_v4();
                let clone = system.module_store.instance(original.type_id, original.instance_id).map(|entry| {
                    let mut element = entry.element.clone();
                    element.set_attr("moduleInstance", new_id.to_string());
                    if let Some(data) = element.elements_mut().next() {
                        if let Some(member) = data.child_mut("ModuleInstanceId") {
                            member.children = vec![XNode::Text(new_id.to_string())];
                        }
                    }
                    ModuleEntry { element }
                });
                if let Some(clone) = clone {
                    system.module_store.set_instance(clone);
                }
                system.previews.push(Preview {
                    name: format!("{}-copy", original.name),
                    type_id: original.type_id,
                    instance_id: new_id,
                });
                made.push(new_id);
            }
        }
        for id in &made {
            self.playback.preview_devices().add(*id);
        }
        if !made.is_empty() {
            self.mark_preview_changed();
        }
        made
    }

    /// Delete Selected, after its alert. The window position goes from `Settings.xml` at once, so Cancel doesn't bring it back, as
    /// upstream. Both keys go: upstream leaves its newer `OpenGLPreview_` one behind.
    pub fn delete_previews(&self, ids: &[Uuid]) {
        let settings = ProfileSettings::shared();
        for id in ids {
            for key in [format!("Preview_{id}"), format!("OpenGLPreview_{id}")] {
                if let Err(error) = settings.delete_node(SettingSection::AppSettings, &key) {
                    logging::error("VixenApplication.ConfigPreviews", "Couldn't remove a preview's window settings.", Some(&error.to_string()));
                }
            }
            {
                let mut system = self.system.write();
                let type_id = system.previews.iter().find(|p| p.instance_id == *id).map(|p| p.type_id);
                system.previews.retain(|p| p.instance_id != *id);
                if let Some(type_id) = type_id {
                    system.module_store.remove_instance(type_id, *id);
                }
            }
            self.playback.preview_devices().remove(id);
        }
        if !ids.is_empty() {
            self.mark_preview_changed();
        }
    }

    /// Update: no validation, so an empty or a repeated name is allowed.
    pub fn rename_preview(&self, id: &Uuid, name: &str) {
        if let Some(preview) = self.system.write().previews.iter_mut().find(|p| p.instance_id == *id) {
            preview.name = name.to_string();
        }
        self.mark_preview_changed();
    }

    /// Ticking a preview. It raises no prompt at Cancel, as upstream, but Cancel still reloads, since that's what puts the running states back.
    /// Returns whether it's running after the call, so the box follows the device where upstream's stays ticked with the engine closed.
    pub fn set_preview_running(&self, id: &Uuid, running: bool) -> bool {
        self.preview_needs_reload.store(true, Ordering::Relaxed);
        self.playback.preview_devices().set_running(id, running)
    }

    /// `_changesMade`. Configure Preview sets it whatever happened in the designer.
    pub fn mark_preview_changed(&self) {
        self.preview_changes.store(true, Ordering::Relaxed);
        self.preview_needs_reload.store(true, Ordering::Relaxed);
    }

    /// Cancel has to ask first.
    pub fn preview_changes_made(&self) -> bool {
        self.preview_changes.load(Ordering::Relaxed)
    }

    /// Cancel has to reload. A list edit or a tick sets this.
    pub fn preview_needs_reload(&self) -> bool {
        self.preview_needs_reload.load(Ordering::Relaxed)
    }

    pub fn reset_preview_changes(&self) {
        self.preview_changes.store(false, Ordering::Relaxed);
        self.preview_needs_reload.store(false, Ordering::Relaxed);
    }
}
