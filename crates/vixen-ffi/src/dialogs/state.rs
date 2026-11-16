//! The State mapper. The draft stays in Rust, with one assignment tree shared by every item.

use super::{find_nodes, no_such, parse_id, DialogContext};
use crate::core::FfiError;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::Color;
use vixen_session::dialogs::state as logic;
use vixen_session::dialogs::state::{StateDraft, StateItemColumn};
use vixen_session::dialogs::world::NodeKey;

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogStateItem {
    /// Stable across reorder and deletion of other rows; the assignment view must follow the item, not its row number.
    pub id: String,
    pub name: String,
    pub color: u32,
    pub hex: String,
    pub text_is_black: bool,
    pub count: u32,
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogStateIssues {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub blocking: bool,
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogAssignmentRow {
    /// A stable index into the tree, for this session.
    pub index: u32,
    pub name: String,
    pub has_children: bool,
    pub checked: bool,
    pub enabled: bool,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogStateColumn {
    Name,
    Color,
    Count,
}

impl From<DialogStateColumn> for StateItemColumn {
    fn from(c: DialogStateColumn) -> Self {
        match c {
            DialogStateColumn::Name => StateItemColumn::Name,
            DialogStateColumn::Color => StateItemColumn::Color,
            DialogStateColumn::Count => StateItemColumn::Count,
        }
    }
}

/// What F193's entry finds: a mapper to open, or a message.
#[derive(uniffi::Record, Clone)]
pub struct DialogStatePrompt {
    pub title: String,
    pub text: String,
}

/// One open State Mapper.
#[derive(uniffi::Object)]
pub struct StateSession {
    context: Arc<DialogContext>,
    node: Uuid,
    draft: Mutex<StateDraft>,
    previewing: Mutex<bool>,
}

#[uniffi::export]
impl DialogContext {
    /// F193's check, for the helper. `Some` is the message to show, and the mapper doesn't open.
    pub fn state_mapping_refusal(&self, nodes: Vec<String>) -> Option<DialogStatePrompt> {
        (nodes.len() != 1).then(|| DialogStatePrompt {
            title: logic::ONE_NODE_ONLY.0.to_string(),
            text: logic::ONE_NODE_ONLY.1.to_string(),
        })
    }

    /// F193, F194. Works on a copy until `apply`.
    pub fn open_state_mapper(self: Arc<Self>, node: String) -> Result<Arc<StateSession>, FfiError> {
        let id = parse_id(&node)?;
        let draft = self.world(|world| {
            let found = find_nodes(world, std::slice::from_ref(&node));
            let key: NodeKey = *found.first().ok_or_else(|| no_such("element"))?;
            Ok::<StateDraft, FfiError>(logic::state_helper_begin(world, key))
        })?;
        Ok(Arc::new(StateSession { context: self, node: id, draft: Mutex::new(draft), previewing: Mutex::new(false) }))
    }

    /// F207: what's wrong with a name in the prompt. `None` means OK is available.
    pub fn state_name_error(&self, name: String, current: Option<String>, existing: Vec<String>) -> Option<String> {
        logic::state_normalize_name(&name, current.as_deref(), &existing).err()
    }

    /// F207's warning, which doesn't stop OK.
    pub fn state_name_warning(&self, name: String, current: Option<String>, existing: Vec<String>) -> Option<String> {
        logic::state_name_warning(&name, current.as_deref(), &existing)
    }
}

impl StateSession {
    fn publish_preview(&self, selected: &[u32]) {
        if !*self.previewing.lock() {
            return;
        }
        let selected: Vec<usize> = selected.iter().map(|i| *i as usize).collect();
        let pairs = self.draft.lock().preview_pairs(&selected);
        let context = &self.context;
        let elements: Vec<(Uuid, Color, bool)> = context.world(|world| {
            pairs
                .iter()
                .filter_map(|(node, color)| {
                    let key = world.find_node(node)?;
                    let element = world.node_element(key)?;
                    let discrete = !vixen_session::dialogs::world::valid_colors(world, key, false).is_empty();
                    Some((element, *color, discrete))
                })
                .collect()
        });
        vixen_session::dialogs::state_preview::show(&context.core, &elements);
    }
}

#[uniffi::export]
impl StateSession {
    pub fn definition_names(&self) -> Vec<String> {
        self.draft.lock().definition_names()
    }

    pub fn selected_definition(&self) -> u32 {
        self.draft.lock().selected as u32
    }

    /// F195: blocked while there are errors. Returns what ends up selected.
    pub fn select_definition(&self, index: u32) -> u32 {
        self.draft.lock().select_definition(index as usize) as u32
    }

    /// F196: the name Add's prompt starts with.
    pub fn next_definition_name(&self) -> String {
        self.draft.lock().next_definition_name()
    }

    /// F196. The result is why it can't be done.
    pub fn add_definition(&self, name: String) -> Option<String> {
        self.draft.lock().add_definition(&name).err()
    }

    pub fn delete_definition(&self) {
        self.draft.lock().delete_definition();
    }

    pub fn rename_definition(&self, name: String) -> Option<String> {
        self.draft.lock().rename_definition(&name).err()
    }

    pub fn copy_definition(&self, name: String) -> Option<String> {
        self.draft.lock().copy_definition(&name).err()
    }

    /// F197.
    pub fn description(&self) -> String {
        let draft = self.draft.lock();
        draft.definitions.get(draft.selected).map_or(String::new(), |d| d.description.clone())
    }

    pub fn set_description(&self, text: String) {
        self.draft.lock().set_description(&text);
    }

    /// F199.
    pub fn items(&self) -> Vec<DialogStateItem> {
        let draft = self.draft.lock();
        draft
            .items()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let color = item.color.to_color();
                DialogStateItem {
                    id: item.id.to_string(),
                    name: item.name.clone(),
                    color: color.to_argb_u32(),
                    hex: logic::state_color_hex(color),
                    text_is_black: logic::state_text_is_black(color),
                    count: draft.item_count(i) as u32,
                }
            })
            .collect()
    }

    /// F201. Returns the new row.
    pub fn add_item(&self) -> u32 {
        self.draft.lock().add_item() as u32
    }

    /// F202. Returns the new row.
    pub fn copy_item(&self, index: u32) -> Option<u32> {
        self.draft.lock().copy_item(index as usize).map(|i| i as u32)
    }

    /// F203's confirmation: its title and text.
    pub fn remove_prompt(&self, rows: Vec<u32>) -> DialogStatePrompt {
        let rows: Vec<usize> = rows.iter().map(|i| *i as usize).collect();
        let (title, text) = self.draft.lock().remove_prompt(&rows);
        DialogStatePrompt { title, text }
    }

    /// F203. Returns the row to select.
    pub fn remove_items(&self, rows: Vec<u32>) -> Option<u32> {
        let rows: Vec<usize> = rows.iter().map(|i| *i as usize).collect();
        self.draft.lock().remove_items(&rows).map(|i| i as u32)
    }

    /// F204. Returns where the row went.
    pub fn move_item(&self, index: u32, offset: i32) -> Option<u32> {
        self.draft.lock().move_item(index as usize, offset).map(|i| i as u32)
    }

    pub fn set_item_name(&self, index: u32, name: String) {
        self.draft.lock().set_item_name(index as usize, &name);
    }

    pub fn set_item_color(&self, index: u32, color: u32) {
        self.draft.lock().set_item_color(index as usize, Color::from_argb_u32(color));
    }

    /// F199: a header sort becomes the stored order.
    pub fn sort_items(&self, column: DialogStateColumn, ascending: bool) {
        self.draft.lock().sort_items(column.into(), ascending);
    }

    /// F200: empty means the full colour picker.
    pub fn item_color_choices(&self, index: u32) -> Vec<u32> {
        let draft = self.draft.lock();
        self.context
            .world(|world| draft.item_color_choices(world, index as usize).iter().map(|c| c.to_argb_u32()).collect())
    }

    /// F205: a tree row's children, or the root when `parent` is `None`, with what they show for this item.
    pub fn assignment_children(&self, item: u32, parent: Option<u32>) -> Vec<DialogAssignmentRow> {
        let draft = self.draft.lock();
        let indices: Vec<u32> = match parent {
            None => {
                if draft.tree.nodes.is_empty() {
                    Vec::new()
                } else {
                    vec![0]
                }
            }
            Some(p) => draft.tree.nodes.get(p as usize).map_or(Vec::new(), |n| n.children.clone()),
        };
        let checked: HashSet<Uuid> =
            draft.items().get(item as usize).map_or(HashSet::new(), |i| i.element_node_ids.iter().copied().collect());
        indices
            .into_iter()
            .map(|index| {
                let n = &draft.tree.nodes[index as usize];
                DialogAssignmentRow {
                    index,
                    name: n.name.clone(),
                    has_children: !n.children.is_empty(),
                    checked: checked.contains(&n.id),
                    enabled: draft.tree.is_enabled(index, &checked),
                }
            })
            .collect()
    }

    /// F205: what to expand to show this item's checked nodes.
    pub fn assignment_expanded(&self, item: u32) -> Vec<u32> {
        let draft = self.draft.lock();
        draft.items().get(item as usize).map_or(Vec::new(), |i| draft.tree.expanded_for(&i.element_node_ids))
    }

    pub fn set_assignment(&self, item: u32, node: u32, on: bool) {
        self.draft.lock().set_checked(item as usize, node, on);
    }

    /// F205's Toggle Selected.
    pub fn toggle_assignments(&self, item: u32, nodes: Vec<u32>) {
        self.draft.lock().toggle_checked(item as usize, &nodes);
    }

    /// F208.
    pub fn issues(&self) -> DialogStateIssues {
        let i = self.draft.lock().issues();
        DialogStateIssues { errors: i.errors, warnings: i.warnings, blocking: i.blocking }
    }

    /// F198: turns the live preview on or off, for the given rows.
    pub fn set_preview(&self, on: bool, selected: Vec<u32>) {
        *self.previewing.lock() = on;
        if on {
            self.publish_preview(&selected);
        } else {
            vixen_session::dialogs::state_preview::clear(&self.context.core);
        }
    }

    /// F198: the selection or a colour changed.
    pub fn refresh_preview(&self, selected: Vec<u32>) {
        self.publish_preview(&selected);
    }

    /// F209. The result is why it can't be done.
    pub fn apply(&self) -> Option<String> {
        let draft = self.draft.lock().clone();
        self.context.world(|world| {
            let key = world.find_node(&self.node)?;
            logic::state_helper_commit(world, key, &draft).err()
        })
    }

    /// F198: closing releases the preview.
    pub fn close(&self) {
        *self.previewing.lock() = false;
        self.context.core.live.release("State Preview");
    }
}
