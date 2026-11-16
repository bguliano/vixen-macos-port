//! The State mapper (F193 to F211).

use super::properties::{raw_property, set_raw_property};
use super::world::{valid_colors, DialogWorld, NodeKey};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use vixen_core::element::{property_ids, PropertyInstance};
use vixen_core::Color;
use vixen_io::values::DotNetColor;
use vixen_xml::{contract_ns, XElement, ARRAYS_NS};

const STATE_NS: &str = "VixenModules.Property.State";

pub const DEFINITION_BASE: &str = "State";
pub const ITEM_BASE: &str = "State Item";

/// F193, shown when the helper is given anything but one node.
pub const ONE_NODE_ONLY: (&str, &str) = ("Too many nodes selected", "State Mapping can only be applied to one Element Node");

/// F211: `{base} - {n}`, the first unused from 1.
pub fn state_next_name(base: &str, existing: &[String]) -> String {
    let mut n = 1;
    loop {
        let candidate = format!("{base} - {n}");
        if !existing.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// `TryNormalizeName`: trimmed, not blank, and not the same as another name. A case-only match is a warning, not an error.
pub fn state_normalize_name(name: &str, current: Option<&str>, existing: &[String]) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("State definition name is required.".to_string());
    }
    if existing.iter().any(|e| Some(e.as_str()) != current && e == name) {
        return Err("State definition names must be unique.".to_string());
    }
    Ok(name.to_string())
}

/// F207's warning, which upstream's view never shows.
pub fn state_name_warning(name: &str, current: Option<&str>, existing: &[String]) -> Option<String> {
    let name = name.trim();
    existing
        .iter()
        .any(|e| Some(e.as_str()) != current && e != name && e.eq_ignore_ascii_case(name))
        .then(|| "State definition names differ only by casing. Check you don't have a typo.".to_string())
}

/// `StateItemData`. The colour stays as stored, so one nobody touched writes back exactly.
#[derive(Clone, Debug, PartialEq)]
pub struct StateItem {
    pub id: Uuid,
    pub name: String,
    pub color: DotNetColor,
    /// The checked nodes. A checked group stands for what's below it, so its descendants aren't listed.
    pub element_node_ids: Vec<Uuid>,
}

/// `StateDefinitionData`.
#[derive(Clone, Debug, PartialEq)]
pub struct StateDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub items: Vec<StateItem>,
}

/// One node of the assignment tree. Nodes are in depth-first order, so a parent always comes before what's under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssignmentNode {
    pub id: Uuid,
    pub name: String,
    pub parent: Option<u32>,
    pub children: Vec<u32>,
}

/// The node's hierarchy, shared by every item, where upstream builds a whole tree per item per definition.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AssignmentTree {
    pub nodes: Vec<AssignmentNode>,
}

impl AssignmentTree {
    pub fn from_world(world: &dyn DialogWorld, root: NodeKey) -> AssignmentTree {
        let mut tree = AssignmentTree::default();
        let mut path = Vec::new();
        tree.add(world, root, None, &mut path);
        tree
    }

    fn add(&mut self, world: &dyn DialogWorld, node: NodeKey, parent: Option<u32>, path: &mut Vec<NodeKey>) -> u32 {
        let index = self.nodes.len() as u32;
        self.nodes.push(AssignmentNode { id: world.node_id(node), name: world.node_name(node), parent, children: Vec::new() });
        path.push(node);
        for child in world.node_children(node) {
            if !path.contains(&child) {
                let c = self.add(world, child, Some(index), path);
                self.nodes[index as usize].children.push(c);
            }
        }
        path.pop();
        index
    }

    fn selected_ids(&self, index: u32, checked: &HashSet<Uuid>, out: &mut Vec<Uuid>) {
        let node = &self.nodes[index as usize];
        if checked.contains(&node.id) {
            if !out.contains(&node.id) {
                out.push(node.id);
            }
        } else {
            for c in &node.children {
                self.selected_ids(*c, checked, out);
            }
        }
    }

    /// `GetSelectedNodeIds`: the checked nodes from the top down, leaving out anything implied by a checked group.
    pub fn normalized(&self, ids: &[Uuid]) -> Vec<Uuid> {
        if self.nodes.is_empty() {
            return Vec::new();
        }
        let checked: HashSet<Uuid> = ids.iter().copied().collect();
        let mut out = Vec::new();
        self.selected_ids(0, &checked, &mut out);
        out
    }

    fn collect_leaves(&self, index: u32, out: &mut HashSet<Uuid>) {
        let node = &self.nodes[index as usize];
        if node.children.is_empty() {
            out.insert(node.id);
        } else {
            for c in &node.children {
                self.collect_leaves(*c, out);
            }
        }
    }

    fn effective(&self, index: u32, checked: &HashSet<Uuid>, out: &mut HashSet<Uuid>) {
        let node = &self.nodes[index as usize];
        if checked.contains(&node.id) {
            self.collect_leaves(index, out);
        } else {
            for c in &node.children {
                self.effective(*c, checked, out);
            }
        }
    }

    /// `GetEffectiveLeafNodeIds`, distinct.
    pub fn effective_leaves(&self, ids: &[Uuid]) -> HashSet<Uuid> {
        let mut out = HashSet::new();
        if !self.nodes.is_empty() {
            let checked: HashSet<Uuid> = ids.iter().copied().collect();
            self.effective(0, &checked, &mut out);
        }
        out
    }

    /// `IsEnabled`: nothing above it is checked.
    pub fn is_enabled(&self, index: u32, checked: &HashSet<Uuid>) -> bool {
        let mut parent = self.nodes[index as usize].parent;
        while let Some(p) = parent {
            let node = &self.nodes[p as usize];
            if checked.contains(&node.id) {
                return false;
            }
            parent = node.parent;
        }
        true
    }

    fn descendants(&self, index: u32, out: &mut Vec<u32>) {
        for c in &self.nodes[index as usize].children {
            out.push(*c);
            self.descendants(*c, out);
        }
    }

    /// `ExpandCheckedDescendants`: what to expand to show every checked node.
    pub fn expanded_for(&self, ids: &[Uuid]) -> Vec<u32> {
        let checked: HashSet<Uuid> = ids.iter().copied().collect();
        let mut out = Vec::new();
        for index in 0..self.nodes.len() as u32 {
            let node = &self.nodes[index as usize];
            if checked.contains(&node.id) {
                let mut parent = node.parent;
                while let Some(p) = parent {
                    if !out.contains(&p) {
                        out.push(p);
                    }
                    parent = self.nodes[p as usize].parent;
                }
            }
        }
        out.sort_unstable();
        out
    }
}

/// F208's result.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StateIssues {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub blocking: bool,
}

/// A sort of the items grid, which becomes the stored order (F199).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StateItemColumn {
    Name,
    Color,
    Count,
}

/// A mapper's draft: nothing here touches the property until OK (F210).
#[derive(Clone, Debug, PartialEq)]
pub struct StateDraft {
    pub property_id: Uuid,
    pub definitions: Vec<StateDefinition>,
    pub selected: usize,
    pub tree: AssignmentTree,
    /// `TryGetFirstCommonDiscreteColor`, or white.
    pub default_color: DotNetColor,
}

fn default_item(name: String, color: DotNetColor) -> StateItem {
    StateItem { id: Uuid::new_v4(), name, color, element_node_ids: Vec::new() }
}

fn default_definition(name: String, color: DotNetColor) -> StateDefinition {
    StateDefinition {
        id: Uuid::new_v4(),
        name,
        description: String::new(),
        items: vec![default_item(state_next_name(ITEM_BASE, &[]), color)],
    }
}

fn parse_item(e: &XElement) -> StateItem {
    StateItem {
        id: e.child_text("Id").and_then(|t| Uuid::parse_str(&t).ok()).filter(|i| !i.is_nil()).unwrap_or_else(Uuid::new_v4),
        // `Name ??= ...`: only null is replaced, so an empty name stays.
        name: match e.child("Name") {
            Some(n) if !n.nil => n.text(),
            _ => state_next_name(ITEM_BASE, &[]),
        },
        color: e.child("Color").and_then(DotNetColor::from_xml).unwrap_or_else(|| DotNetColor::from_known(164)),
        element_node_ids: e
            .child("ElementNodeIds")
            .map(|l| l.elements().filter_map(|g| Uuid::parse_str(g.text().trim()).ok()).collect())
            .unwrap_or_default(),
    }
}

fn parse_definition(e: &XElement) -> StateDefinition {
    let name = e.child_text("Name").map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    StateDefinition {
        id: e.child_text("Id").and_then(|t| Uuid::parse_str(&t).ok()).filter(|i| !i.is_nil()).unwrap_or_else(Uuid::new_v4),
        name: name.unwrap_or_else(|| state_next_name(DEFINITION_BASE, &[])),
        description: e.child_text("Description").unwrap_or_default(),
        items: e.child("Items").map(|l| l.elements().map(parse_item).collect()).unwrap_or_default(),
    }
}

/// The common discrete colour of the node's leaves, or white (F201).
pub fn state_common_discrete_color(world: &dyn DialogWorld, node: NodeKey) -> Color {
    let mut per_leaf: Vec<Vec<Color>> = Vec::new();
    for leaf in super::world::leaves(world, node) {
        let colors = valid_colors(world, leaf, false);
        if !colors.is_empty() {
            per_leaf.push(colors);
        }
    }
    let Some(first) = per_leaf.first() else {
        return Color::WHITE;
    };
    first.iter().find(|c| per_leaf.iter().all(|colors| colors.contains(c))).copied().unwrap_or(Color::WHITE)
}

/// F193: a draft of the node's State property, or of a new one, which attaches only on OK.
pub fn state_helper_begin(world: &dyn DialogWorld, node: NodeKey) -> StateDraft {
    let tree = AssignmentTree::from_world(world, node);
    let common = state_common_discrete_color(world, node);
    let default_color = if common == Color::WHITE { DotNetColor::from_known(164) } else { DotNetColor::from_color(common) };
    let index = world.properties(node).iter().position(|p| p.type_id == property_ids::STATE);
    let xml = index.and_then(|i| raw_property(world, node, i));
    let property_id = xml
        .as_ref()
        .and_then(|e| e.child_text("Id"))
        .and_then(|t| Uuid::parse_str(&t).ok())
        .filter(|i| !i.is_nil())
        .unwrap_or_else(Uuid::new_v4);
    let mut definitions: Vec<StateDefinition> = xml
        .as_ref()
        .and_then(|e| e.child("StateDefinitions"))
        .map(|l| l.elements().map(parse_definition).collect())
        .unwrap_or_default();
    if definitions.is_empty() {
        // `StateData`'s constructor: its one item is white. The mapper's own new definitions take the common colour.
        definitions.push(default_definition(state_next_name(DEFINITION_BASE, &[]), DotNetColor::from_known(164)));
    }
    for definition in &mut definitions {
        for item in &mut definition.items {
            item.element_node_ids = tree.normalized(&item.element_node_ids);
        }
    }
    StateDraft { property_id, definitions, selected: 0, tree, default_color }
}

impl StateDraft {
    pub fn definition_names(&self) -> Vec<String> {
        self.definitions.iter().map(|d| d.name.clone()).collect()
    }

    fn current(&self) -> Option<&StateDefinition> {
        self.definitions.get(self.selected)
    }

    fn current_mut(&mut self) -> Option<&mut StateDefinition> {
        self.definitions.get_mut(self.selected)
    }

    pub fn items(&self) -> &[StateItem] {
        self.current().map_or(&[], |d| d.items.as_slice())
    }

    /// `HasBlockingValidationErrors`.
    pub fn has_blocking_errors(&self) -> bool {
        if self.definitions.is_empty() {
            return true;
        }
        let mut seen = HashSet::new();
        self.definitions.iter().any(|d| {
            d.name.trim().is_empty() || d.items.iter().any(|i| i.name.trim().is_empty()) || !seen.insert(d.name.as_str())
        })
    }

    /// F195: blocked while there are errors, when the choice snaps back. Returns what ends up selected.
    pub fn select_definition(&mut self, index: usize) -> usize {
        if index < self.definitions.len() && !self.has_blocking_errors() {
            self.selected = index;
        }
        self.selected
    }

    /// F196's Add, with a name from the prompt.
    pub fn add_definition(&mut self, name: &str) -> Result<(), String> {
        let existing = self.definition_names();
        let name = state_normalize_name(name, None, &existing)?;
        let color = self.default_color.clone();
        self.definitions.push(default_definition(name, color));
        self.selected = self.definitions.len() - 1;
        Ok(())
    }

    /// F196: the name Add's prompt offers.
    pub fn next_definition_name(&self) -> String {
        state_next_name(DEFINITION_BASE, &self.definition_names())
    }

    /// F196's Delete, which needs another definition to leave behind.
    pub fn delete_definition(&mut self) {
        if self.definitions.len() > 1 && self.selected < self.definitions.len() {
            self.definitions.remove(self.selected);
            self.selected = self.selected.min(self.definitions.len() - 1);
        }
    }

    /// F196's Rename.
    pub fn rename_definition(&mut self, name: &str) -> Result<(), String> {
        let existing = self.definition_names();
        let current = self.current().map(|d| d.name.clone());
        let name = state_normalize_name(name, current.as_deref(), &existing)?;
        if let Some(d) = self.current_mut() {
            d.name = name;
        }
        Ok(())
    }

    /// F196's Copy: new ids, the same assignments.
    pub fn copy_definition(&mut self, name: &str) -> Result<(), String> {
        let existing = self.definition_names();
        let name = state_normalize_name(name, None, &existing)?;
        let Some(source) = self.current().cloned() else {
            return Ok(());
        };
        let items = source.items.into_iter().map(|i| StateItem { id: Uuid::new_v4(), ..i }).collect();
        self.definitions.push(StateDefinition { id: Uuid::new_v4(), name, description: source.description, items });
        self.selected = self.definitions.len() - 1;
        Ok(())
    }

    /// F197.
    pub fn set_description(&mut self, text: &str) {
        if let Some(d) = self.current_mut() {
            d.description = text.to_string();
        }
    }

    fn item_names(&self) -> Vec<String> {
        self.items().iter().map(|i| i.name.clone()).collect()
    }

    /// F201. Returns the new row's index.
    pub fn add_item(&mut self) -> usize {
        let name = state_next_name(ITEM_BASE, &self.item_names());
        let color = self.default_color.clone();
        match self.current_mut() {
            Some(d) => {
                d.items.push(default_item(name, color));
                d.items.len() - 1
            }
            None => 0,
        }
    }

    /// F202: `{source} - {n}`, with the same assignments.
    pub fn copy_item(&mut self, index: usize) -> Option<usize> {
        let source = self.items().get(index)?.clone();
        let base = source.name.trim();
        let base = if base.is_empty() { ITEM_BASE } else { base };
        let name = state_next_name(base, &self.item_names());
        let d = self.current_mut()?;
        d.items.push(StateItem { id: Uuid::new_v4(), name, ..source });
        Some(d.items.len() - 1)
    }

    /// F203's confirmation.
    pub fn remove_prompt(&self, indices: &[usize]) -> (String, String) {
        if let [only] = indices {
            let name = self.items().get(*only).map_or(String::new(), |i| i.name.clone());
            ("Delete State Item".to_string(), format!("Delete State item '{name}'?"))
        } else {
            ("Delete State Items".to_string(), format!("Delete {} State items?", indices.len()))
        }
    }

    /// F203. After one row, the row now at that place is selected. After several, none.
    pub fn remove_items(&mut self, indices: &[usize]) -> Option<usize> {
        let mut sorted: Vec<usize> = indices.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        let first = *sorted.first()?;
        let count = sorted.len();
        let d = self.current_mut()?;
        for i in sorted.iter().rev() {
            if *i < d.items.len() {
                d.items.remove(*i);
            }
        }
        (count == 1 && !d.items.is_empty()).then(|| first.min(d.items.len() - 1))
    }

    /// F204. Returns where the row went, or `None` when it has nowhere to go.
    pub fn move_item(&mut self, index: usize, offset: i32) -> Option<usize> {
        let d = self.current_mut()?;
        let target = index as i64 + offset as i64;
        if index >= d.items.len() || target < 0 || target >= d.items.len() as i64 {
            return None;
        }
        let item = d.items.remove(index);
        d.items.insert(target as usize, item);
        Some(target as usize)
    }

    /// F199: a name is trimmed as it commits.
    pub fn set_item_name(&mut self, index: usize, name: &str) {
        if let Some(item) = self.current_mut().and_then(|d| d.items.get_mut(index)) {
            item.name = name.trim().to_string();
        }
    }

    pub fn set_item_color(&mut self, index: usize, color: Color) {
        if let Some(item) = self.current_mut().and_then(|d| d.items.get_mut(index)) {
            item.color = DotNetColor::from_color(color);
        }
    }

    /// F205: the Count column.
    pub fn item_count(&self, index: usize) -> usize {
        self.items().get(index).map_or(0, |i| self.tree.effective_leaves(&i.element_node_ids).len())
    }

    /// F199: sorted rows become the stored order.
    pub fn sort_items(&mut self, column: StateItemColumn, ascending: bool) {
        let counts: HashMap<Uuid, usize> =
            self.items().iter().map(|i| (i.id, self.tree.effective_leaves(&i.element_node_ids).len())).collect();
        let Some(d) = self.current_mut() else {
            return;
        };
        let compare = |a: &StateItem, b: &StateItem| match column {
            StateItemColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            StateItemColumn::Color => state_color_hex(a.color.to_color()).cmp(&state_color_hex(b.color.to_color())),
            StateItemColumn::Count => counts.get(&a.id).cmp(&counts.get(&b.id)),
        };
        if ascending {
            d.items.sort_by(compare);
        } else {
            d.items.sort_by(|a, b| compare(b, a));
        }
    }

    /// F200: the colours the discrete picker offers for an item: its leaves', or the whole node's if it has none. Empty means free choice.
    pub fn item_color_choices(&self, world: &dyn DialogWorld, index: usize) -> Vec<Color> {
        let Some(item) = self.items().get(index) else {
            return Vec::new();
        };
        let leaves = self.tree.effective_leaves(&item.element_node_ids);
        let mut out: Vec<Color> = Vec::new();
        let mut add = |node: NodeKey| {
            for c in valid_colors(world, node, true) {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
        };
        if leaves.is_empty() {
            if let Some(root) = self.tree.nodes.first().and_then(|n| world.find_node(&n.id)) {
                add(root);
            }
        } else {
            for leaf in &leaves {
                if let Some(node) = world.find_node(leaf) {
                    add(node);
                }
            }
        }
        out
    }

    /// F205: a tree row's children, with what they show for this item.
    pub fn is_checked(&self, item: usize, node: u32) -> bool {
        match (self.items().get(item), self.tree.nodes.get(node as usize)) {
            (Some(i), Some(n)) => i.element_node_ids.contains(&n.id),
            _ => false,
        }
    }

    pub fn is_enabled(&self, item: usize, node: u32) -> bool {
        let Some(i) = self.items().get(item) else {
            return false;
        };
        let checked: HashSet<Uuid> = i.element_node_ids.iter().copied().collect();
        self.tree.is_enabled(node, &checked)
    }

    /// F205: checking a group clears what's below it, and those rows go disabled.
    pub fn set_checked(&mut self, item: usize, node: u32, on: bool) {
        if !self.is_enabled(item, node) {
            return;
        }
        let Some(id) = self.tree.nodes.get(node as usize).map(|n| n.id) else {
            return;
        };
        let mut below = Vec::new();
        self.tree.descendants(node, &mut below);
        let below: HashSet<Uuid> = below.iter().map(|i| self.tree.nodes[*i as usize].id).collect();
        let tree = self.tree.clone();
        let Some(i) = self.current_mut().and_then(|d| d.items.get_mut(item)) else {
            return;
        };
        if on {
            i.element_node_ids.retain(|n| !below.contains(n));
            i.element_node_ids.push(id);
        } else {
            i.element_node_ids.retain(|n| *n != id);
        }
        i.element_node_ids = tree.normalized(&i.element_node_ids);
    }

    /// F205's Toggle Selected: from the top down, so a group that goes on disables what's below it, which then doesn't toggle.
    pub fn toggle_checked(&mut self, item: usize, nodes: &[u32]) {
        let mut sorted = nodes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        for node in sorted {
            if self.is_enabled(item, node) {
                let on = self.is_checked(item, node);
                self.set_checked(item, node, !on);
            }
        }
    }

    /// F208.
    pub fn issues(&self) -> StateIssues {
        let mut issues = StateIssues { blocking: self.has_blocking_errors(), ..Default::default() };
        let name = self.current().map_or("", |d| d.name.as_str());
        if name.trim().is_empty() {
            issues.errors.push("State definition name is required.".to_string());
        } else if name.chars().count() < 3 {
            issues.warnings.push("State definition names shorter than three characters may be unclear.".to_string());
        }
        if self.items().iter().any(|i| i.name.trim().is_empty()) {
            issues.errors.push("Every State item requires a name.".to_string());
        }
        let mut exact = HashSet::new();
        let duplicates = self.definitions.iter().any(|d| !d.name.trim().is_empty() && !exact.insert(d.name.as_str()));
        if duplicates {
            issues.errors.push("State definition names must be unique.".to_string());
        }
        let mut by_folded: HashMap<String, HashSet<&str>> = HashMap::new();
        for d in &self.definitions {
            by_folded.entry(d.name.to_lowercase()).or_default().insert(d.name.as_str());
        }
        if by_folded.values().any(|names| names.len() > 1) {
            issues.warnings.push("State definition names differ only by casing. Check you don't have a typo.".to_string());
        }
        issues
    }

    /// F198: `(leaf element node, colour)` for the selected items, what the live preview shows.
    pub fn preview_pairs(&self, selected: &[usize]) -> Vec<(Uuid, Color)> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for index in selected {
            if let Some(item) = self.items().get(*index) {
                let mut leaves: Vec<Uuid> = self.tree.effective_leaves(&item.element_node_ids).into_iter().collect();
                leaves.sort();
                for leaf in leaves {
                    if seen.insert((leaf, item.color.to_color().to_argb_u32())) {
                        out.push((leaf, item.color.to_color()));
                    }
                }
            }
        }
        out
    }
}

/// F200: `#RRGGBB`.
pub fn state_color_hex(c: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

/// F200: whether the hex text on a coloured cell should be black, as `BackgroundColorToTextBrushConverter` chooses.
pub fn state_text_is_black(c: Color) -> bool {
    let delta = (c.r as f64 * 0.299 + c.g as f64 * 0.587 + c.b as f64 * 0.114).round_ties_even() as i32;
    255 - delta < 105
}

fn guid_el(ns: &str, local: &str, id: &Uuid) -> XElement {
    XElement::with_text(ns, local, id.to_string())
}

/// `StateData`, in the order `DataContractSerializer` writes it. Members this doesn't know about stay.
fn state_xml(old: Option<XElement>, instance_id: &Uuid, draft: &StateDraft) -> XElement {
    let ns = contract_ns(STATE_NS);
    let mut list = XElement::new(&ns, "StateDefinitions");
    for d in &draft.definitions {
        let mut definition = XElement::new(&ns, "StateDefinitionData");
        definition.push(if d.description.is_empty() {
            XElement::new(&ns, "Description")
        } else {
            XElement::with_text(&ns, "Description", d.description.as_str())
        });
        definition.push(guid_el(&ns, "Id", &d.id));
        let mut items = XElement::new(&ns, "Items");
        for item in &d.items {
            let mut x = XElement::new(&ns, "StateItemData");
            x.push(item.color.to_xml(&ns, "Color"));
            let mut ids = XElement::new(&ns, "ElementNodeIds");
            for id in &item.element_node_ids {
                ids.push(XElement::with_text(ARRAYS_NS, "guid", id.to_string()));
            }
            x.push(ids);
            x.push(guid_el(&ns, "Id", &item.id));
            x.push(if item.name.is_empty() { XElement::new(&ns, "Name") } else { XElement::with_text(&ns, "Name", item.name.as_str()) });
            items.push(x);
        }
        definition.push(items);
        definition.push(XElement::with_text(&ns, "Name", d.name.as_str()));
        list.push(definition);
    }
    let mut e = old.unwrap_or_else(|| {
        let mut e = XElement::new(&ns, "StateData");
        e.push(XElement::with_text("", "ModuleInstanceId", instance_id.to_string()));
        e.push(XElement::with_text("", "ModuleTypeId", property_ids::STATE.to_string()));
        e.push(guid_el(&ns, "Id", &draft.property_id));
        e.push(XElement::new(&ns, "StateDefinitions"));
        e
    });
    match e.child_mut("StateDefinitions") {
        Some(existing) => *existing = list,
        None => {
            e.push(list);
        }
    }
    e
}

/// F209: names are trimmed, and the draft replaces the property's definitions. A node with no property gets one only here (F193).
pub fn state_helper_commit(world: &mut dyn DialogWorld, node: NodeKey, draft: &StateDraft) -> Result<(), String> {
    let mut draft = draft.clone();
    for d in &mut draft.definitions {
        d.name = d.name.trim().to_string();
        for item in &mut d.items {
            item.name = item.name.trim().to_string();
        }
    }
    if draft.has_blocking_errors() {
        return Err(draft.issues().errors.first().cloned().unwrap_or_else(|| "State definition name is required.".to_string()));
    }
    let index = match world.properties(node).iter().position(|p| p.type_id == property_ids::STATE) {
        Some(i) => i,
        None => {
            let properties = world.properties_mut(node);
            properties.push(PropertyInstance {
                type_id: property_ids::STATE,
                instance_id: Uuid::new_v4(),
                data: vixen_core::element::PropertyData::Unknown { raw_xml: String::new() },
            });
            properties.len() - 1
        }
    };
    let old = raw_property(world, node, index);
    let instance = world.properties(node)[index].instance_id;
    set_raw_property(world, node, index, state_xml(old, &instance, &draft));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::flow::test_support::*;
    use super::super::properties::add_property;
    use super::super::world::{property_data, SystemWorld};
    use super::*;
    use vixen_core::element::{ElementColorType, PropertyData};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn names_count_up_from_one() {
        assert_eq!(state_next_name("State", &[]), "State - 1");
        assert_eq!(state_next_name("State", &names(&["State - 1"])), "State - 2");
        assert_eq!(state_next_name("State", &names(&["State - 2"])), "State - 1");
        assert_eq!(state_next_name("State", &names(&["state - 1"])), "State - 1", "case-sensitive");
    }

    #[test]
    fn name_prompt_rules() {
        let existing = names(&["Gate", "Door"]);
        assert_eq!(state_normalize_name("  New ", None, &existing).unwrap(), "New");
        assert_eq!(state_normalize_name("  ", None, &existing).unwrap_err(), "State definition name is required.");
        assert_eq!(state_normalize_name("Gate", None, &existing).unwrap_err(), "State definition names must be unique.");
        assert_eq!(state_normalize_name("Gate", Some("Gate"), &existing).unwrap(), "Gate");
        assert!(state_normalize_name("gate", None, &existing).is_ok());
        assert!(state_name_warning("gate", None, &existing).is_some());
        assert!(state_name_warning("Gate", Some("Gate"), &existing).is_none());
        assert!(state_name_warning("Other", None, &existing).is_none());
    }

    fn draft_for(world: &SystemWorld<'_>, root: NodeKey) -> StateDraft {
        state_helper_begin(world, root)
    }

    #[test]
    fn a_new_property_starts_with_one_definition_and_one_item() {
        let (mut system, root, _) = system_with_leaves(2);
        let world = SystemWorld::new(&mut system);
        let d = draft_for(&world, root);
        assert_eq!(d.definition_names(), ["State - 1"]);
        assert_eq!(d.items().len(), 1);
        assert_eq!(d.items()[0].name, "State Item - 1");
        assert_eq!(d.items()[0].color.to_color(), Color::WHITE);
        assert_eq!(d.tree.nodes.len(), 3);
    }

    #[test]
    fn cancelling_leaves_no_property_and_ok_adds_one() {
        let (mut system, root, _) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        let d = draft_for(&world, root);
        assert!(property_data(&world, root, &property_ids::STATE).is_none());
        state_helper_commit(&mut world, root, &d).unwrap();
        assert!(property_data(&world, root, &property_ids::STATE).is_some());
        let p = &world.properties(root)[0];
        let entry = world.system.module_store.instance(p.type_id, p.instance_id).unwrap();
        assert_eq!(entry.data_model_type(), Some("VixenModules.Property.State.StateData, State"));
    }

    #[test]
    fn a_draft_round_trips_with_stable_ids() {
        let (mut system, root, leaves) = system_with_leaves(2);
        let mut world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        d.set_item_name(0, "  On ");
        d.set_item_color(0, Color::rgb(1, 2, 3));
        let leaf_index = d.tree.nodes.iter().position(|n| n.id == world.node_id(leaves[1])).unwrap() as u32;
        d.set_checked(0, leaf_index, true);
        d.set_description("About");
        state_helper_commit(&mut world, root, &d).unwrap();
        let back = draft_for(&world, root);
        assert_eq!(back.property_id, d.property_id);
        assert_eq!(back.definitions[0].id, d.definitions[0].id);
        assert_eq!(back.items()[0].id, d.items()[0].id);
        assert_eq!(back.items()[0].name, "On");
        assert_eq!(back.items()[0].color.to_color(), Color::rgb(1, 2, 3));
        assert_eq!(back.items()[0].element_node_ids, [world.node_id(leaves[1])]);
        assert_eq!(back.definitions[0].description, "About");
    }

    #[test]
    fn checking_a_group_clears_and_disables_what_is_below() {
        let (mut system, root, _) = system_with_leaves(3);
        let world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        d.set_checked(0, 1, true);
        d.set_checked(0, 2, true);
        assert_eq!(d.item_count(0), 2);
        d.set_checked(0, 0, true);
        assert_eq!(d.items()[0].element_node_ids, [d.tree.nodes[0].id]);
        assert_eq!(d.item_count(0), 3);
        assert!(!d.is_enabled(0, 1));
        // A disabled row can't change.
        d.set_checked(0, 1, true);
        assert_eq!(d.items()[0].element_node_ids.len(), 1);
        d.set_checked(0, 0, false);
        assert!(d.is_enabled(0, 1));
        assert_eq!(d.item_count(0), 0);
    }

    #[test]
    fn toggling_goes_from_the_top_down() {
        let (mut system, root, _) = system_with_leaves(2);
        let world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        d.toggle_checked(0, &[1, 0]);
        assert_eq!(d.items()[0].element_node_ids, [d.tree.nodes[0].id], "the group went on, and the leaf below it stayed out");
    }

    #[test]
    fn expansion_reaches_every_checked_node() {
        let mut system = vixen_io::SystemData::default();
        let a = system.tree.add_string("A", 2);
        let b = system.tree.add_string("B", 2);
        let root = system.tree.add_node(Uuid::new_v4(), "Root", None);
        system.tree.add_child(root, a);
        system.tree.add_child(root, b);
        let world = SystemWorld::new(&mut system);
        let d = draft_for(&world, key(root));
        // Root 0, A 1, its leaves 2 and 3, B 4, its leaves 5 and 6.
        let ids = vec![d.tree.nodes[6].id];
        assert_eq!(d.tree.expanded_for(&ids), [0, 4]);
        assert!(d.tree.expanded_for(&[]).is_empty());
    }

    #[test]
    fn ids_that_are_not_in_the_tree_drop_out_on_opening() {
        let (mut system, root, leaves) = system_with_leaves(2);
        let mut world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        let real = world.node_id(leaves[0]);
        d.definitions[0].items[0].element_node_ids = vec![Uuid::new_v4(), real];
        state_helper_commit(&mut world, root, &d).unwrap();
        let back = draft_for(&world, root);
        assert_eq!(back.items()[0].element_node_ids, [real]);
    }

    #[test]
    fn items_add_copy_remove_and_move() {
        let (mut system, root, _) = system_with_leaves(2);
        let world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        assert_eq!(d.add_item(), 1);
        assert_eq!(d.items()[1].name, "State Item - 2");
        d.set_checked(0, 1, true);
        assert_eq!(d.copy_item(0), Some(2));
        assert_eq!(d.items()[2].name, "State Item - 1 - 1");
        assert_eq!(d.items()[2].element_node_ids, d.items()[0].element_node_ids);
        assert_ne!(d.items()[2].id, d.items()[0].id);
        assert_eq!(d.move_item(2, -1), Some(1));
        assert_eq!(d.items()[1].name, "State Item - 1 - 1");
        assert_eq!(d.move_item(0, -1), None);
        assert_eq!(d.move_item(2, 1), None);
        assert_eq!(d.remove_prompt(&[1]), ("Delete State Item".to_string(), "Delete State item 'State Item - 1 - 1'?".to_string()));
        assert_eq!(d.remove_prompt(&[0, 1]), ("Delete State Items".to_string(), "Delete 2 State items?".to_string()));
        assert_eq!(d.remove_items(&[2]), Some(1), "the row now at that place");
        assert_eq!(d.items().len(), 2);
        assert_eq!(d.remove_items(&[0, 1]), None, "after several, none");
    }

    #[test]
    fn definitions_add_delete_rename_copy() {
        let (mut system, root, _) = system_with_leaves(2);
        let world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        assert_eq!(d.next_definition_name(), "State - 2");
        d.add_definition("Gate").unwrap();
        assert_eq!(d.selected, 1);
        assert_eq!(d.add_definition("Gate").unwrap_err(), "State definition names must be unique.");
        d.set_checked(0, 1, true);
        d.copy_definition("Gate Copy").unwrap();
        assert_eq!(d.selected, 2);
        assert_eq!(d.items()[0].element_node_ids.len(), 1);
        assert_ne!(d.definitions[2].items[0].id, d.definitions[1].items[0].id);
        d.rename_definition("Latch").unwrap();
        assert_eq!(d.definition_names(), ["State - 1", "Gate", "Latch"]);
        d.delete_definition();
        assert_eq!(d.definition_names(), ["State - 1", "Gate"]);
        assert_eq!(d.selected, 1);
        d.delete_definition();
        d.delete_definition();
        assert_eq!(d.definition_names(), ["State - 1"], "the last one stays");
    }

    #[test]
    fn errors_block_the_selection_and_ok() {
        let (mut system, root, _) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        d.add_definition("Gate").unwrap();
        d.set_item_name(0, "");
        let issues = d.issues();
        assert!(issues.blocking);
        assert_eq!(issues.errors, ["Every State item requires a name."]);
        assert_eq!(d.select_definition(0), 1, "snaps back");
        assert!(state_helper_commit(&mut world, root, &d).is_err());
        d.set_item_name(0, "On");
        assert_eq!(d.select_definition(0), 0);
    }

    #[test]
    fn warnings_do_not_block() {
        let (mut system, root, _) = system_with_leaves(1);
        let world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        d.rename_definition("Ab").unwrap();
        d.add_definition("AB").unwrap();
        d.select_definition(0);
        let issues = d.issues();
        assert!(!issues.blocking);
        assert_eq!(
            issues.warnings,
            [
                "State definition names shorter than three characters may be unclear.",
                "State definition names differ only by casing. Check you don't have a typo."
            ]
        );
    }

    #[test]
    fn sorting_becomes_the_stored_order() {
        let (mut system, root, _) = system_with_leaves(2);
        let world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        d.add_item();
        d.set_item_name(0, "b");
        d.set_item_name(1, "A");
        d.set_checked(0, 1, true);
        d.sort_items(StateItemColumn::Name, true);
        assert_eq!(d.items()[0].name, "A");
        d.sort_items(StateItemColumn::Count, false);
        assert_eq!(d.items()[0].name, "b");
    }

    #[test]
    fn a_new_item_takes_the_colour_common_to_all_leaves() {
        let (mut system, root, leaves) = system_with_leaves(2);
        let mut world = SystemWorld::new(&mut system);
        assert_eq!(state_common_discrete_color(&world, root), Color::WHITE);
        for (leaf, set) in [(leaves[0], "RGB"), (leaves[1], "RGBW")] {
            let i = add_property(&mut world, leaf, &property_ids::COLOR);
            world.properties_mut(leaf)[i].data = PropertyData::Color {
                color_type: ElementColorType::MultipleDiscreteColors,
                single_color: Color::EMPTY,
                color_set_name: Some(set.to_string()),
            };
        }
        assert_eq!(state_common_discrete_color(&world, root), Color::rgb(255, 0, 0));
        let mut d = draft_for(&world, root);
        let i = d.add_item();
        assert_eq!(d.items()[i].color.to_color(), Color::rgb(255, 0, 0));
        assert_eq!(d.item_color_choices(&world, i).len(), 4, "the root's colours, while it has no leaves of its own");
    }

    #[test]
    fn hex_text_and_its_contrast() {
        assert_eq!(state_color_hex(Color::rgb(1, 171, 255)), "#01ABFF");
        assert!(state_text_is_black(Color::WHITE));
        assert!(!state_text_is_black(Color::BLACK));
    }

    #[test]
    fn preview_pairs_list_each_leaf_of_the_selected_items() {
        let (mut system, root, _) = system_with_leaves(2);
        let world = SystemWorld::new(&mut system);
        let mut d = draft_for(&world, root);
        d.set_checked(0, 0, true);
        d.set_item_color(0, Color::RED);
        assert_eq!(d.preview_pairs(&[0]).len(), 2);
        assert!(d.preview_pairs(&[]).is_empty());
    }
}
