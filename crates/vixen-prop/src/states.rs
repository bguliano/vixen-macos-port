//! CustomPropEditor's authored State tab. All edits join the document's snapshot undo.
use crate::{ElementModelType, PanelTab, PropEditor, PropError, StateDefinitionModel, StateItemModel};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use vixen_core::Color;

fn next_name(base: &str, names: impl Iterator<Item = String>) -> String {
    let names: HashSet<_> = names.collect();
    for i in 1.. {
        let candidate = format!("{} - {i}", base.trim());
        if !names.contains(&candidate) { return candidate; }
    }
    unreachable!()
}

impl PropEditor {
    pub fn state_model_id(&self) -> Uuid {
        self.prop.breadth_first().into_iter().find(|id| self.prop.element(*id).is_some_and(|e| e.model_type == ElementModelType::Model)).unwrap_or(self.prop.root_id())
    }
    pub fn state_definitions(&self) -> &[StateDefinitionModel] {
        &self.prop.element(self.state_model_id()).expect("root always exists").state_definitions
    }
    pub fn selected_state_definition(&self) -> Option<Uuid> {
        self.selected_definition.filter(|id| self.state_definitions().iter().any(|d| d.id == *id)).or_else(|| self.state_definitions().first().map(|d| d.id))
    }
    pub fn state_items(&self) -> &[StateItemModel] {
        self.state_definitions().iter().find(|d| Some(d.id) == self.selected_state_definition()).map(|d| d.items.as_slice()).unwrap_or(&[])
    }
    pub fn state_selected_items(&self) -> Vec<Uuid> {
        let selected: Vec<_> = self.state_items().iter().filter(|i| self.selected_state_items.contains(&i.id)).map(|i| i.id).collect();
        if selected.is_empty() { self.state_items().first().map(|i| vec![i.id]).unwrap_or_default() } else { selected }
    }
    pub fn select_state_definition(&mut self, id: Option<Uuid>) {
        self.selected_definition = id; self.selected_state_items.clear(); self.state_drag = None;
    }
    pub fn select_state_items(&mut self, ids: Vec<Uuid>) { self.selected_state_items = ids; self.state_drag = None; }
    pub fn next_state_definition_name(&self) -> String { next_name("State", self.state_definitions().iter().map(|d| d.name.clone())) }
    fn state_name(&self, name: &str, except: Option<Uuid>) -> Result<String, PropError> {
        let name = name.trim();
        if name.is_empty() { return Err(PropError::new("Invalid State Definition", "State definition names cannot be empty.")); }
        // Upstream's name prompt compares ordinally; save validation below catches case-only duplicates.
        if self.state_definitions().iter().any(|d| Some(d.id) != except && d.name == name) {
            return Err(PropError::new("Invalid State Definition", format!("State definition name \"{name}\" is duplicated.")));
        }
        Ok(name.to_string())
    }
    pub fn add_state_definition(&mut self, name: &str) -> Result<Uuid, PropError> {
        let name = self.state_name(name, None)?; let definition = StateDefinitionModel::create_default(&name); let id = definition.id;
        let model = self.state_model_id(); self.begin("Add State Definition");
        self.prop.element_mut(model).unwrap().state_definitions.push(definition); self.select_state_definition(Some(id)); Ok(id)
    }
    pub fn rename_state_definition(&mut self, name: &str) -> Result<bool, PropError> {
        let Some(id) = self.selected_state_definition() else { return Ok(false) };
        let name = self.state_name(name, Some(id))?;
        let model = self.state_model_id(); let index = self.state_definitions().iter().position(|d| d.id == id).unwrap();
        if self.state_definitions()[index].name == name { return Ok(false) }
        self.begin("Rename State Definition"); self.prop.element_mut(model).unwrap().state_definitions[index].name = name; Ok(true)
    }
    pub fn copy_state_definition(&mut self, name: &str) -> Result<bool, PropError> {
        let name = self.state_name(name, None)?;
        let Some(mut definition) = self.state_definitions().iter().find(|d| Some(d.id) == self.selected_state_definition()).cloned() else { return Ok(false) };
        definition.id = Uuid::new_v4(); definition.name = name;
        for item in &mut definition.items { item.id = Uuid::new_v4(); }
        let model = self.state_model_id(); let id = definition.id; self.begin("Copy State Definition");
        self.prop.element_mut(model).unwrap().state_definitions.push(definition); self.select_state_definition(Some(id)); Ok(true)
    }
    pub fn delete_state_definition(&mut self) -> bool {
        let Some(id) = self.selected_state_definition() else { return false };
        let index = self.state_definitions().iter().position(|d| d.id == id).unwrap(); let model = self.state_model_id(); self.begin("Delete State Definition");
        let definitions = &mut self.prop.element_mut(model).unwrap().state_definitions; definitions.remove(index);
        let next = definitions.get(index.min(definitions.len().saturating_sub(1))).map(|d| d.id);
        self.select_state_definition(next); true
    }
    fn definition_index(&self) -> Option<usize> { self.state_definitions().iter().position(|d| Some(d.id) == self.selected_state_definition()) }
    pub fn set_state_description(&mut self, text: &str) -> bool {
        let Some(index) = self.definition_index() else { return false };
        if self.state_definitions()[index].description == text { return false }
        let model = self.state_model_id(); self.begin("Edit State Description"); self.prop.element_mut(model).unwrap().state_definitions[index].description = text.into(); true
    }
    pub fn add_state_item(&mut self, copy: bool) -> bool {
        let Some(index) = self.definition_index() else { return false };
        let mut item = if copy {
            let selected = self.state_selected_items(); if selected.len() != 1 { return false }
            let Some(item) = self.state_items().iter().find(|i| i.id == selected[0]) else { return false };
            let mut item = item.clone(); item.name = next_name(&item.name, self.state_items().iter().map(|i| i.name.clone())); item
        } else { StateItemModel::new(&next_name("State Item", self.state_items().iter().map(|i| i.name.clone()))) };
        item.id = Uuid::new_v4(); let id = item.id; let model = self.state_model_id(); self.begin(if copy { "Copy State Item" } else { "Add State Item" });
        self.prop.element_mut(model).unwrap().state_definitions[index].items.push(item); self.selected_state_items = vec![id]; true
    }
    pub fn remove_state_items(&mut self) -> bool {
        let Some(index) = self.definition_index() else { return false }; let selected = self.state_selected_items();
        let Some(at) = self.state_items().iter().position(|i| selected.contains(&i.id)) else { return false };
        let model = self.state_model_id(); self.begin("Remove State Items");
        let items = &mut self.prop.element_mut(model).unwrap().state_definitions[index].items; items.retain(|i| !selected.contains(&i.id));
        self.selected_state_items = items.get(at.min(items.len().saturating_sub(1))).map(|i| vec![i.id]).unwrap_or_default(); true
    }
    pub fn move_state_item(&mut self, down: bool) -> bool {
        let Some(index) = self.definition_index() else { return false }; let selected = self.state_selected_items();
        if selected.len() != 1 { return false }
        let Some(at) = self.state_items().iter().position(|i| i.id == selected[0]) else { return false };
        if (!down && at == 0) || (down && at + 1 == self.state_items().len()) { return false }
        let model = self.state_model_id(); self.begin("Move State Item");
        self.prop.element_mut(model).unwrap().state_definitions[index].items.swap(at, if down { at + 1 } else { at - 1 }); true
    }
    pub fn edit_state_item(&mut self, id: Uuid, name: &str, color: Color) -> bool {
        let Some(index) = self.definition_index() else { return false };
        let Some(at) = self.state_items().iter().position(|i| i.id == id) else { return false };
        let item = &self.state_items()[at]; if item.name == name && item.color == color { return false }
        let model = self.state_model_id(); self.begin("Edit State Item"); let item = &mut self.prop.element_mut(model).unwrap().state_definitions[index].items[at]; item.name = name.into(); item.color = color; true
    }
    pub fn reorder_state_items(&mut self, ids: &[Uuid]) -> bool {
        let Some(index) = self.definition_index() else { return false }; let current: Vec<_> = self.state_items().iter().map(|i| i.id).collect();
        if current == ids || ids.len() != current.len() || ids.iter().copied().collect::<HashSet<_>>() != current.iter().copied().collect() { return false }
        let items = ids.iter().map(|id| self.state_items().iter().find(|i| i.id == *id).unwrap().clone()).collect();
        let model = self.state_model_id(); self.begin("Sort State Items"); self.prop.element_mut(model).unwrap().state_definitions[index].items = items; true
    }
    pub fn assign_state_elements(&mut self, ids: &[Uuid], remove: bool, toggle: bool) -> bool {
        self.assign_state_elements_impl(ids, remove, toggle, true)
    }
    pub(crate) fn assign_state_elements_impl(&mut self, ids: &[Uuid], remove: bool, toggle: bool, record: bool) -> bool {
        let Some(index) = self.definition_index() else { return false }; let selected = self.state_selected_items(); if selected.len() != 1 { return false }
        let Some(at) = self.state_items().iter().position(|i| i.id == selected[0]) else { return false };
        let mut wanted: Vec<_> = self.state_items()[at].element_ids.iter().copied().filter(|id| !id.is_nil() && self.prop.contains(*id)).collect();
        let mut seen = HashSet::new(); wanted.retain(|id| seen.insert(*id));
        let mut seen = HashSet::new();
        for id in ids.iter().copied().filter(|id| !id.is_nil() && self.prop.contains(*id) && seen.insert(*id)) {
            if remove || toggle && wanted.contains(&id) { wanted.retain(|i| *i != id); } else if !wanted.contains(&id) { wanted.push(id); }
        }
        if wanted == self.state_items()[at].element_ids { return false }
        let model = self.state_model_id(); if record { self.begin("Assign State Elements"); } self.prop.element_mut(model).unwrap().state_definitions[index].items[at].element_ids = wanted; true
    }
    pub fn state_preview_color(&self, owner: Uuid) -> Option<Color> {
        self.state_preview_colors().map(|colors| colors.get(&owner).copied().unwrap_or(self.settings.state_preview_base_color))
    }
    /// Compute once per canvas snapshot; avoid scanning every assignment for every light.
    pub fn state_preview_colors(&self) -> Option<HashMap<Uuid, Color>> {
        if self.tab != PanelTab::State { return None }
        let selected = self.state_selected_items(); let mut sums: HashMap<Uuid, (u64, u64, u64, u64)> = HashMap::new();
        for item in self.state_items().iter().filter(|i| selected.contains(&i.id)) {
            let mut seen = HashSet::new();
            for id in item.element_ids.iter().filter(|id| seen.insert(**id)) {
                let sum = sums.entry(*id).or_default(); sum.0 += item.color.r as u64; sum.1 += item.color.g as u64; sum.2 += item.color.b as u64; sum.3 += 1;
            }
        }
        Some(sums.into_iter().map(|(id, (r,g,b,n))| (id, Color::rgb((r as f64 / n as f64).round_ties_even() as u8, (g as f64 / n as f64).round_ties_even() as u8, (b as f64 / n as f64).round_ties_even() as u8))).collect())
    }
    pub fn state_validation(&self) -> Vec<String> {
        let mut errors = Vec::new(); let models: Vec<_> = self.prop.elements().filter(|e| e.model_type == ElementModelType::Model).collect();
        if models.len() > 1 { errors.push("Only one element can have Model Type set to Model.".into()); }
        for model in models { if model.is_leaf() && !(model.id == self.prop.root_id() && model.children().is_empty()) { errors.push(format!("Element \"{}\" cannot be a Model because it is a leaf element.", model.name())); } }
        let mut names: Vec<(String, usize)> = Vec::new();
        for definition in self.state_definitions() {
            let name = if definition.name.trim().is_empty() { "<unnamed>" } else { definition.name.trim() };
            if definition.name.trim().is_empty() { errors.push("State definition names cannot be empty.".into()); }
            if definition.items.is_empty() { errors.push(format!("State definition \"{name}\" must contain at least one item.")); }
            for item in &definition.items {
                if item.name.trim().is_empty() { errors.push(format!("State definition \"{name}\" contains an item with no name.")); }
                if !item.element_ids.iter().any(|id| self.prop.contains(*id)) { errors.push(format!("State item \"{}\" in \"{name}\" must assign at least one element.", if item.name.trim().is_empty() { "<unnamed>" } else { item.name.trim() })); }
            }
            if !definition.name.trim().is_empty() {
                if let Some((_, count)) = names.iter_mut().find(|(n,_)| n.to_lowercase() == name.to_lowercase()) { *count += 1; }
                else { names.push((name.to_string(),1)); }
            }
        }
        errors.extend(names.into_iter().filter(|(_, count)| *count > 1).map(|(name,_)| format!("State definition name \"{name}\" is duplicated."))); errors
    }
}

#[cfg(test)] mod tests {
    use super::*;
    use crate::{CanvasMode, Modifiers, PropEditorSettings};
    fn editor() -> PropEditor {
        let mut e = PropEditor::new_document("Test", PropEditorSettings::default(), "now");
        e.pointer_down(30.0, 40.0, Modifiers::default()); e.pointer_up(); e
    }
    #[test] fn validation_blocks_save_until_assignment_and_copy_preserves_assignments() {
        let mut e = editor(); let owner = e.light_dots()[0].owner;
        e.add_state_definition(" Door ").unwrap(); assert_eq!(e.state_definitions()[0].name, "Door");
        assert_eq!(e.state_validation().len(), 1);
        assert!(e.save_as(&std::env::temp_dir().join("invalid-state-must-not-exist.prp"), "now").is_err());
        e.assign_state_elements(&[owner, owner], false, false); assert!(e.state_validation().is_empty());
        let original = e.state_items()[0].clone(); e.copy_state_definition("Door Copy").unwrap();
        assert_eq!(e.state_items()[0].element_ids, vec![owner]); assert_ne!(e.state_items()[0].id, original.id);
        assert!(e.add_state_definition(" Door ").is_err());
        assert!(e.add_state_definition("door").is_ok());
        assert!(e.state_validation().iter().any(|s| s.contains("duplicated")));
    }
    #[test] fn selected_items_average_colors_and_never_change_lights() {
        let mut e = editor(); let owner = e.light_dots()[0].owner; e.add_state_definition("Colors").unwrap();
        let red = e.state_items()[0].id; e.edit_state_item(red, "Red", Color::RED); e.assign_state_elements(&[owner], false, false);
        e.add_state_item(false); let blue = e.state_selected_items()[0]; e.edit_state_item(blue, "Blue", Color::BLUE); e.assign_state_elements(&[owner], false, false);
        e.select_state_items(vec![red, blue]); e.set_tab(PanelTab::State); e.set_mode(CanvasMode::Select);
        assert_eq!(e.state_preview_color(owner), Some(Color::rgb(128, 0, 128)));
        let before = e.light_dots(); e.pointer_down(30.0, 40.0, Modifiers::default()); e.pointer_dragged(90.0, 80.0, Modifiers::default()); e.pointer_up();
        assert_eq!(e.light_dots(), before);
    }
    #[test] fn sorting_requires_every_item_once_and_undo_restores_order() {
        let mut e = editor(); e.add_state_definition("States").unwrap(); e.add_state_item(false); e.add_state_item(false);
        let ids: Vec<_> = e.state_items().iter().map(|i| i.id).collect(); assert!(!e.reorder_state_items(&[ids[0], ids[0], ids[2]]));
        let mut reversed = ids.clone(); reversed.reverse(); assert!(e.reorder_state_items(&reversed)); e.undo();
        assert_eq!(e.state_items().iter().map(|i| i.id).collect::<Vec<_>>(), ids);
    }
    #[test] fn click_and_band_assign_elements_and_command_removes_without_moving() {
        let mut e = editor(); e.pointer_down(60.0, 40.0, Modifiers::default()); e.pointer_up();
        let first = e.light_dots()[0].owner; let before = e.light_dots();
        e.add_state_definition("Door").unwrap(); e.set_tab(PanelTab::State); e.set_mode(CanvasMode::Select);
        e.pointer_down(30.0, 40.0, Modifiers::default()); e.pointer_dragged(70.0, 90.0, Modifiers::default()); e.pointer_up();
        assert_eq!(e.state_items()[0].element_ids, vec![first]); assert_eq!(e.light_dots(), before);
        e.pointer_down(20.0, 30.0, Modifiers::default()); e.pointer_dragged(70.0, 50.0, Modifiers::default()); e.pointer_up();
        assert_eq!(e.state_items()[0].element_ids.len(), 2);
        let command = Modifiers { command: true, shift: false };
        e.pointer_down(20.0, 30.0, command); e.pointer_dragged(40.0, 50.0, command); e.pointer_up();
        assert_eq!(e.state_items()[0].element_ids.len(), 1); assert!(!e.state_items()[0].element_ids.contains(&first));
        e.undo(); assert_eq!(e.state_items()[0].element_ids.len(), 2);
    }
    #[test] fn state_draw_and_assignment_share_one_undo_and_delete_prunes() {
        let mut e = editor(); e.add_state_definition("Door").unwrap(); e.set_tab(PanelTab::State);
        e.pointer_down(80.0, 80.0, Modifiers::default()); e.pointer_up();
        assert_eq!(e.light_dots().len(), 2); assert_eq!(e.state_items()[0].element_ids.len(), 1);
        e.undo(); assert_eq!(e.light_dots().len(), 1); assert!(e.state_items()[0].element_ids.is_empty());
        e.redo(); assert_eq!(e.light_dots().len(), 2); assert_eq!(e.state_items()[0].element_ids.len(), 1);
        e.delete_selected_lights(); assert!(e.state_items()[0].element_ids.is_empty());
        e.undo(); assert_eq!(e.state_items()[0].element_ids.len(), 1);
    }
}
