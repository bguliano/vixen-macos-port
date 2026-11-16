//! The row list's input: `RowLabel`, the row handlers in `Grid` and the row menu in `TimelineControl`. F19, F25, F28 to F37.

use super::model::{HostRequest, InputOutcome, Timeline};
use super::rows::LabelZone;
use super::{Button, CursorKind, ElementId, Mods, PointerEvent, RowId};
use uuid::Uuid;
use vixen_core::Color;

/// A tag definition as the row labels and the Tags menu need it.
#[derive(Clone, Debug, PartialEq)]
pub struct TagDisplay {
    pub id: Uuid,
    pub name: String,
    pub sort_order: i32,
    pub color: Option<Color>,
}

/// F37's state over the selected rows' nodes.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TagCheck {
    Unchecked,
    Checked,
    Mixed,
}

/// One line of the Tags submenu.
#[derive(Clone, Debug, PartialEq)]
pub struct TagMenuItem {
    pub tag: Uuid,
    pub name: String,
    pub color: Option<Color>,
    pub state: TagCheck,
}

/// F33: what the row label menu offers.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RowMenuModel {
    /// F36: upstream throws with no selected row, so the item is disabled.
    pub reset_selected_enabled: bool,
    pub tags_enabled: bool,
}

/// What a tag change did to one node, for the host to apply to the system configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct TagChange {
    pub node: Uuid,
    pub tag: Uuid,
    pub added: bool,
}

impl Timeline {
    /// `RowLabel.OnMouseDown`. `x` is relative to the list's left, `y` to the visible top of the list.
    pub fn row_mouse_down(&mut self, ev: PointerEvent, width: i32) -> InputOutcome {
        let y = ev.y + self.vertical_offset;
        self.row_press = self.rows.row_at(y).filter(|_| y >= 0);
        if ev.button == Some(Button::Left) {
            if let Some(r) = self.row_press {
                let top = self.rows.row_top(r).unwrap_or(0);
                if self.rows.row_label_zone(r, ev.x, y - top, width) == LabelZone::ResizeBar {
                    self.row_resizing = Some((r, y - top));
                }
            }
        }
        self.finish()
    }

    /// `RowLabel.OnMouseMove`: a height drag adds the y delta to the row's height, live. F30.
    pub fn row_mouse_move(&mut self, ev: PointerEvent, width: i32) -> InputOutcome {
        let y = ev.y + self.vertical_offset;
        if let Some((r, last)) = self.row_resizing {
            let top = self.rows.row_top(r).unwrap_or(0);
            let local = y - top;
            let delta = local - last;
            if delta != 0 {
                let height = self.rows.row(r).height + delta;
                self.rows.set_row_height(r, height);
                self.rows_changed();
                self.mark_editor_state_modified();
                self.row_resizing = Some((r, local));
            }
            // `RowLabel.OnMouseMove` takes the pointer from the hit test on every move: up-down over the bar, an arrow off it, as when the row
            // hits the 10 point floor and the pointer goes on up. Upstream tests before the height changes, so a fast move flickers to the
            // arrow; this tests after, so the pointer is steady while it tracks the bar (docs/DIFFERENCES.md).
            let zone = self.rows.row_label_zone(r, ev.x, local, width);
            self.cursor_kind = if zone == LabelZone::ResizeBar { CursorKind::ResizeUpDown } else { CursorKind::Arrow };
        } else {
            let zone = self.rows.row_at(y).filter(|_| y >= 0).map(|r| {
                let top = self.rows.row_top(r).unwrap_or(0);
                self.rows.row_label_zone(r, ev.x, y - top, width)
            });
            self.cursor_kind = if zone == Some(LabelZone::ResizeBar) { CursorKind::ResizeUpDown } else { CursorKind::Arrow };
        }
        self.finish()
    }

    /// `RowLabel.OnMouseClick` then `OnMouseUp`. WinForms raises its click on right-button release too, so a right release runs the click
    /// and then the label click again: a plain right-click collapses a multi-selection to the clicked row, ⇧-right-click extends the range,
    /// ⌘-right-click toggles twice with no net change, and a right-click on an expander toggles it. F33. KEEP.
    pub fn row_mouse_up(&mut self, ev: PointerEvent, width: i32) -> InputOutcome {
        let y = ev.y + self.vertical_offset;
        let hit = self.rows.row_at(y).filter(|_| y >= 0);
        let press = self.row_press.take();
        if let Some((r, _)) = self.row_resizing.take() {
            // F30: every row with the same name takes this height, only on release.
            self.rows.finish_row_resize(r);
            self.rows_changed();
            return self.finish();
        }
        let Some(r) = hit.filter(|h| Some(*h) == press) else {
            // The pressed label still has the mouse and gets the release, outside itself: no click, but a right release opens the menu all
            // the same (`RowLabel.cs:132-139`).
            if let (Some(pressed), Some(Button::Right)) = (press, ev.button) {
                self.requests.push(HostRequest::RowContextMenu { row: pressed });
            }
            return self.finish();
        };
        let top = self.rows.row_top(r).unwrap_or(0);
        let zone = self.rows.row_label_zone(r, ev.x, y - top, width);
        // F19: `OnMouseClick`, which WinForms doesn't raise for the second press of a double-click. So a fast double-click acts once.
        if ev.clicks % 2 == 1 {
            match zone {
                LabelZone::Expander => {
                    let open = self.rows.row(r).tree_open;
                    self.rows.set_tree_open(r, !open);
                    self.rows.relayout();
                    self.rows_changed();
                }
                LabelZone::Label => self.click_row_label(r, ev.mods),
                LabelZone::ResizeBar | LabelZone::None => {}
            }
        }
        if ev.button == Some(Button::Right) {
            if matches!(zone, LabelZone::Label | LabelZone::ResizeBar) {
                self.click_row_label(r, ev.mods);
            }
            self.requests.push(HostRequest::RowContextMenu { row: r });
        }
        self.finish()
    }

    /// `LabelClickedHandler` and `RowSelectedChangedHandler`. F25.
    pub fn click_row_label(&mut self, r: RowId, mods: Mods) {
        let selected = if mods.cmd { !self.rows.row(r).selected } else { true };
        self.set_row_selected(r, selected);
        if mods.cmd || mods.shift {
            let any_selected = self.rows.rows.iter().any(|row| row.selected);
            if mods.cmd || !any_selected {
                self.select_single_row_effects(r);
            } else if let Some(first) = self.first_selected_row {
                // Both directions, over rows visible at this moment.
                let (from, to) = (first.0.min(r.0), first.0.max(r.0));
                for i in from..=to {
                    let id = RowId(i);
                    if self.rows.row(id).visible {
                        self.set_row_selected(id, true);
                        self.select_row_effects_filtered(id);
                    }
                }
            } else {
                self.select_single_row_effects(r);
            }
        } else {
            self.clear_selected_quietly();
            self.clear_selected_rows(Some(r));
            self.clear_active_rows(None);
            self.select_row_effects_filtered(r);
            self.first_selected_row = Some(r);
        }
        self.raise_selection_changed();
    }

    fn select_single_row_effects(&mut self, r: RowId) {
        if self.rows.row(r).selected {
            self.select_row_effects_filtered(r);
            self.first_selected_row = Some(r);
        } else {
            if !self.rows.rows.iter().any(|row| row.selected) {
                self.first_selected_row = None;
            }
            for id in self.row_element_ids(r) {
                self.set_selected(id, false);
            }
        }
    }

    /// With the Drag Box Filter on this assigns, so non-matching effects in the row are deselected.
    fn select_row_effects_filtered(&mut self, r: RowId) {
        let filter = self.opts.drag_box_filter.enabled.then(|| self.opts.drag_box_filter.types.clone());
        for id in self.row_element_ids(r) {
            let selected = match &filter {
                Some(types) => self.elements.get(&id).is_some_and(|e| types.contains(&e.desc.type_id)),
                None => true,
            };
            self.set_selected(id, selected);
        }
    }

    fn row_element_ids(&self, r: RowId) -> Vec<ElementId> {
        self.rows.row(r).occurrences.iter().map(|o| o.id).collect()
    }

    /// `MoveActiveRow`. F28: one visible row, no wrap, leaves selection alone, scrolls just enough to show it. False with no active row.
    pub fn move_active_row(&mut self, direction: i32) -> bool {
        if direction == 0 {
            return false;
        }
        let Some(active) = self.active_row() else { return false };
        let Some(index) = self.rows.visible_index_of(active) else { return false };
        let count = self.rows.visible().len() as i32;
        let target = (index as i32 + direction.signum()).clamp(0, count - 1) as usize;
        if target == index {
            return false;
        }
        let target = self.rows.visible()[target];
        self.clear_active_rows(Some(target));
        self.set_row_active(target, true);
        self.ensure_row_visible(target);
        true
    }

    /// `EnsureRowVisible`: top aligned above, bottom aligned below.
    fn ensure_row_visible(&mut self, r: RowId) {
        let Some(top) = self.rows.row_top(r) else { return };
        if top < self.vertical_offset {
            self.set_vertical_offset(top);
            return;
        }
        let bottom = top + self.rows.row(r).height;
        if bottom > self.vertical_offset + self.grid_height {
            self.set_vertical_offset(bottom - self.grid_height);
        }
    }

    /// `ToggleSelectedRows`. F29: T, or ⌘T for the whole subtree. One layout at the end.
    pub fn toggle_selected_rows(&mut self, include_children: bool) {
        let mut targets: Vec<RowId> = self.rows.rows.iter().filter(|r| r.selected).map(|r| r.id).collect();
        if targets.is_empty() {
            targets.extend(self.active_row());
        }
        for r in targets {
            let open = !self.rows.row(r).tree_open;
            if include_children {
                self.rows.toggle_subtree(r, open);
            } else {
                self.rows.set_tree_open(r, open);
            }
        }
        self.rows.relayout();
        self.rows_changed();
    }

    /// F24.
    pub fn set_show_hidden_rows(&mut self, show: bool) {
        self.rows.set_visibility_filter(show);
        self.rows_changed();
    }

    /// F38: the tag manager saved. Upstream's tag dots and hidden row filter read the live tags; here the definitions and each row's tags
    /// are copies, so they come again from the system configuration, `node_tags` by node id, before the filter is applied again (F24).
    pub fn reload_tags(&mut self, tags: Vec<TagDisplay>, node_tags: &dyn Fn(Uuid) -> Option<Vec<Uuid>>) {
        self.tags = tags;
        for row in &mut self.rows.rows {
            if let Some(current) = node_tags(row.node_uuid) {
                row.tags = current;
            }
        }
        let show = self.rows.show_hidden();
        self.set_show_hidden_rows(show);
    }

    /// Every row open.
    pub fn expand_all_rows(&mut self) {
        for i in 0..self.rows.roots.len() {
            let root = self.rows.roots[i];
            self.rows.toggle_subtree(root, true);
        }
        self.rows.relayout();
        self.rows_changed();
    }

    /// F34.
    pub fn collapse_all_rows(&mut self) {
        self.rows.collapse_all();
        self.rows_changed();
    }

    /// F35.
    pub fn reset_row_heights(&mut self) {
        self.rows.reset_row_heights();
        self.rows_changed();
        self.mark_editor_state_modified();
    }

    /// F36.
    pub fn reset_selected_row_heights(&mut self) {
        if self.rows.reset_selected_row_heights() {
            self.rows_changed();
            self.mark_editor_state_modified();
        }
    }

    /// F33's enabling.
    pub fn row_menu_model(&self) -> RowMenuModel {
        let any = self.rows.rows.iter().any(|r| r.selected);
        RowMenuModel { reset_selected_enabled: any, tags_enabled: self.rows.rows.iter().any(|r| r.selected && r.node.is_some()) }
    }

    /// F37: one item per tag definition, with its state over the selected rows' nodes.
    pub fn tag_menu_state(&self) -> Vec<TagMenuItem> {
        let selected: Vec<&super::Row> = self.rows.rows.iter().filter(|r| r.selected).collect();
        let mut defs: Vec<&TagDisplay> = self.tags.iter().collect();
        defs.sort_by_key(|t| t.sort_order);
        defs.iter()
            .map(|t| {
                let tagged = selected.iter().filter(|r| r.tags.contains(&t.id)).count();
                let state = if tagged == 0 {
                    TagCheck::Unchecked
                } else if tagged == selected.len() {
                    TagCheck::Checked
                } else {
                    TagCheck::Mixed
                };
                TagMenuItem { tag: t.id, name: t.name.clone(), color: t.color, state }
            })
            .collect()
    }

    /// F37: checked removes from the targets, anything else adds. `cascade` is ⌘: targets include all descendants of each selected node.
    /// The hidden row filter is applied again (F24). Returns what changed, for the host to save.
    pub fn toggle_tag_on_selected(&mut self, tag: Uuid, cascade: bool) -> Vec<TagChange> {
        let selected: Vec<RowId> = self.rows.rows.iter().filter(|r| r.selected).map(|r| r.id).collect();
        let tagged = selected.iter().filter(|r| self.rows.row(**r).tags.contains(&tag)).count();
        let remove = !selected.is_empty() && tagged == selected.len();
        let mut targets: Vec<RowId> = Vec::new();
        let mut stack = selected;
        while let Some(r) = stack.pop() {
            targets.push(r);
            if cascade {
                stack.extend(self.rows.row(r).children.iter().copied());
            }
        }
        let mut changes: Vec<TagChange> = Vec::new();
        for r in targets {
            let row = self.rows.row(r);
            let has = row.tags.contains(&tag);
            if has != remove {
                continue;
            }
            let node = row.node;
            let uuid = row.node_uuid;
            if changes.iter().any(|c| c.node == uuid) {
                continue;
            }
            changes.push(TagChange { node: uuid, tag, added: !remove });
            // Every row of the node.
            let rows: Vec<RowId> = node.map(|n| self.rows.rows_of_node(n).to_vec()).unwrap_or_else(|| vec![r]);
            for other in rows {
                let tags = &mut self.rows.row_mut(other).tags;
                if remove {
                    tags.retain(|t| *t != tag);
                } else if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
        let show = self.rows.show_hidden();
        self.set_show_hidden_rows(show);
        changes
    }
}
