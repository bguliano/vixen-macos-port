//! The element tree's rows and commands: `Common.Controls.ElementTree` and `SetupElementsTree`, without the tree control.

use super::tree::{child_path, format_path, parent_of, Nid, RowPath};
use super::{RowIcon, SetupSession};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use vixen_core::element::{PropertyData, PropertyInstance};

/// One row of the element tree, fetched in bulk per parent.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementRow {
    pub path: String,
    pub name: String,
    pub icon: RowIcon,
    pub has_children: bool,
    pub child_count: u32,
    /// Resolved here, so a row's paint does no lookups (F25).
    pub tag_colors: Vec<u32>,
}

/// What changed, so the window reloads only that (performance hazard: "Repopulate on each edit").
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditResult {
    pub reload_roots: bool,
    /// Nodes whose children changed. A node with two parents has two rows, and both reload.
    pub reload_nodes: Vec<Nid>,
    /// Nodes whose own row changed: name, icon, or whether it has children.
    pub refresh_nodes: Vec<Nid>,
    /// The new selection, when an edit changes it (F60).
    pub select: Option<Vec<String>>,
    /// Rows to expand first, in order.
    pub expand: Vec<String>,
    /// Put the first selected row at the top.
    pub scroll_to_top: bool,
}

impl EditResult {
    fn note_parent(&mut self, parent: Option<Nid>) {
        match parent {
            None => self.reload_roots = true,
            Some(p) => {
                if !self.reload_nodes.contains(&p) {
                    self.reload_nodes.push(p);
                }
                if !self.refresh_nodes.contains(&p) {
                    self.refresh_nodes.push(p);
                }
            }
        }
    }
}

/// Every rule of the tree's context menu (F32). Cut, Add, Add Multiple and Collapse All have fixed answers and aren't here.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeMenuState {
    pub copy: bool,
    pub paste: bool,
    pub paste_as_new: bool,
    pub item_properties: bool,
    pub copy_properties: bool,
    pub paste_properties: bool,
    pub delete: bool,
    pub create_group: bool,
    pub rename: bool,
    pub find_replace: bool,
    pub reverse: bool,
    pub sort: bool,
    pub export_wire_diagram: bool,
    pub export_element_tree: bool,
    pub tags: bool,
}

/// `_clipboardProperties` holds references to the source's property objects, so edits made after the copy show at paste time. This keeps the source
/// and a snapshot for when the source has gone since.
#[derive(Clone, Debug)]
pub struct PropertyClipboard {
    pub source: Nid,
    pub properties: Vec<PropertyInstance>,
}

impl SetupSession {
    // ---- Rows -------------------------------------------------------------------------------------------------------

    /// The rows under `parent`, or the roots. A parent path that has gone stale has no rows.
    pub fn element_rows(&self, parent: Option<&str>) -> Vec<ElementRow> {
        let list: &[Nid] = match parent {
            None => &self.tree.roots,
            Some(text) => match self.tree.resolve(text) {
                Some(path) => &self.tree.node(*path.last().expect("not empty")).children,
                None => return Vec::new(),
            },
        };
        list.iter().map(|n| self.row_for(*n, child_path(parent, *n))).collect()
    }

    /// Rows for these paths, in order, `None` where a path has gone stale. For refreshing what's on screen.
    pub fn element_rows_at(&self, paths: &[String]) -> Vec<Option<ElementRow>> {
        paths
            .iter()
            .map(|text| {
                let path = self.tree.resolve(text)?;
                Some(self.row_for(*path.last()?, text.clone()))
            })
            .collect()
    }

    fn row_for(&self, node: Nid, path: String) -> ElementRow {
        let n = self.tree.node(node);
        ElementRow {
            path,
            name: n.name.clone(),
            icon: self.node_icon(node),
            has_children: !n.children.is_empty(),
            child_count: n.children.len() as u32,
            tag_colors: self.tag_table.colors_of(&n.tags),
        }
    }

    /// `UpdateTreeNodeImage` (F22). `Element.Masked` is a run-time switch that nothing in this window can set, so it never shows here unless a later feature adds it.
    pub fn node_icon(&self, node: Nid) -> RowIcon {
        let n = self.tree.node(node);
        if !n.children.is_empty() {
            return RowIcon::Group;
        }
        match n.element_id.and_then(|e| self.flow.find(&e)) {
            Some(c) if !self.flow.destinations(c).is_empty() => RowIcon::Patched,
            _ => RowIcon::Unpatched,
        }
    }

    /// Icons for these paths: the refresh after a patching change, for visible rows only. A stale path reads as unpatched, and goes with the next reload.
    pub fn element_icons(&self, paths: &[String]) -> Vec<RowIcon> {
        paths
            .iter()
            .map(|text| match self.tree.resolve(text).and_then(|p| p.last().copied()) {
                Some(n) => self.node_icon(n),
                None => RowIcon::Unpatched,
            })
            .collect()
    }

    pub(crate) fn rows(&self, paths: &[String]) -> Vec<RowPath> {
        paths.iter().filter_map(|p| self.tree.resolve(p)).collect()
    }

    fn row_of_node(&self, node: Nid) -> String {
        format_path(&self.tree.first_parent_path(node))
    }

    /// The nodes of these rows in order, with repeats kept.
    pub(crate) fn nodes_of(&self, paths: &[String]) -> Vec<Nid> {
        paths.iter().filter_map(|p| self.tree.resolve(p)?.last().copied()).collect()
    }

    // ---- Menu rules -------------------------------------------------------------------------------------------------

    /// F32. `primary` is the row Paste and Sort act on.
    pub fn context_menu_state(&self, selection: &[String], primary: Option<&str>) -> TreeMenuState {
        let rows = self.rows(selection);
        let count = rows.len();
        let any = count > 0;
        let one_node = (count == 1).then(|| *rows[0].last().expect("not empty"));
        let destination = primary.and_then(|p| self.tree.resolve(p)).and_then(|p| p.last().copied());
        let (paste, paste_as_new) = self.clipboard_state(destination);
        TreeMenuState {
            copy: any,
            paste,
            paste_as_new,
            item_properties: any,
            copy_properties: count == 1,
            paste_properties: any && self.property_clipboard.is_some(),
            delete: any,
            create_group: any,
            rename: any,
            find_replace: any,
            reverse: self.tree.can_reverse(&rows),
            sort: one_node.is_some_and(|n| !self.tree.is_leaf(n)),
            export_wire_diagram: one_node.is_some_and(|n| !self.tree.is_leaf(n)),
            export_element_tree: !self.tree.roots.is_empty(),
            tags: any,
        }
    }

    fn live_clipboard(&self) -> Option<Vec<Nid>> {
        // Upstream can paste a node deleted since the copy, which re-adds a leaf with no element. Those are ignored here.
        // With nothing left, there's nothing to paste: the menu items are off and a keyboard paste beeps (F35).
        let live: Vec<Nid> = self.clipboard.as_ref()?.iter().copied().filter(|n| self.tree.is_live(*n)).collect();
        (!live.is_empty()).then_some(live)
    }

    /// `(CanPaste(), CanPaste(true))` for a destination (F36).
    pub fn clipboard_state(&self, destination: Option<Nid>) -> (bool, bool) {
        let clipboard = self.live_clipboard();
        (
            self.tree.can_paste(clipboard.as_deref(), destination, false),
            self.tree.can_paste(clipboard.as_deref(), destination, true),
        )
    }

    /// F43's condition: a leaf with an element and at least one data flow destination.
    pub fn would_lose_patches(&self, destination: Option<&str>) -> bool {
        let Some(node) = destination.and_then(|d| self.tree.resolve(d)).and_then(|p| p.last().copied()) else {
            return false;
        };
        self.tree
            .element_of_leaf(node)
            .and_then(|e| self.flow.find(&e))
            .is_some_and(|c| !self.flow.destinations(c).is_empty())
    }

    // ---- Adding ------------------------------------------------------------------------------------------------------

    /// `AddNewNode` (F40). An empty name is "New Element". The caller has asked F43 if needed.
    pub fn add_element(&mut self, parent: Option<&str>, name: &str) -> EditResult {
        let name = if name.is_empty() { "New Element" } else { name };
        self.add_elements(parent, &[name.to_string()])
    }

    /// F41: one node per non-empty name. Selects the new rows and shows the last.
    pub fn add_elements(&mut self, parent: Option<&str>, names: &[String]) -> EditResult {
        let parent_path = parent.and_then(|p| self.tree.resolve(p));
        if parent.is_some() && parent_path.is_none() {
            return EditResult::default();
        }
        let parent_node = parent_path.as_ref().and_then(|p| p.last().copied());
        let mut result = EditResult::default();
        let mut select = Vec::with_capacity(names.len());
        for name in names.iter().filter(|n| !n.is_empty()) {
            let (node, effects) = self.tree.create_node(parent_node, Some(name), true, true);
            self.apply_effects(effects);
            select.push(child_path(parent, node));
        }
        if select.is_empty() {
            return result;
        }
        self.tree_changed();
        result.note_parent(parent_node);
        if let Some(p) = parent {
            result.expand.push(p.to_string());
        }
        result.select = Some(select);
        result
    }

    /// F45: a new root group holding links to the selected nodes, which also stay where they were.
    pub fn create_group(&mut self, selection: &[String], name: &str) -> EditResult {
        let name = if name.is_empty() { "New Element" } else { name };
        let nodes = self.nodes_of(selection);
        let (group, effects) = self.tree.create_node(None, Some(name), true, true);
        self.apply_effects(effects);
        for n in nodes {
            let effects = self.tree.add_child_to_parent(n, Some(group), None);
            self.apply_effects(effects);
        }
        self.tree_changed();
        let path = format_path(&[group]);
        EditResult { reload_roots: true, select: Some(vec![path]), ..EditResult::default() }
    }

    // ---- Deleting ------------------------------------------------------------------------------------------------------

    /// `DeleteNode` for each row (F44): only this row's parent link goes. A node left with no links goes with its element and properties.
    pub fn delete_rows(&mut self, selection: &[String]) -> EditResult {
        let rows = self.rows(selection);
        let mut result = EditResult::default();
        for row in &rows {
            let node = *row.last().expect("not empty");
            // An earlier row in this selection may have taken this one with it.
            if !self.tree.is_live(node) {
                continue;
            }
            let parent = parent_of(row);
            let effects = self.tree.remove_node(node, parent, true);
            self.apply_effects(effects);
            result.note_parent(parent);
        }
        if !rows.is_empty() {
            self.tree_changed();
            result.select = Some(Vec::new());
        }
        result
    }

    // ---- Renaming ------------------------------------------------------------------------------------------------------

    /// F47, F49: rows renamed by index, stopping if there are fewer names. A row whose name doesn't change keeps it (F138).
    pub fn rename_rows(&mut self, selection: &[String], names: &[String]) -> EditResult {
        let rows = self.rows(selection);
        let mut result = EditResult::default();
        let mut changed = false;
        for (row, name) in rows.iter().zip(names) {
            let node = *row.last().expect("not empty");
            if self.tree.rename_node(node, name) {
                changed = true;
                if !result.refresh_nodes.contains(&node) {
                    result.refresh_nodes.push(node);
                }
            }
        }
        if changed {
            self.tree_changed();
            // A patching view showing the element's name has to redraw too.
            self.bump_graph();
        }
        result
    }

    /// The names of these rows, in order, for the rename dialogs.
    pub fn names_of(&self, selection: &[String]) -> Vec<String> {
        self.nodes_of(selection).iter().map(|n| self.tree.node(*n).name.clone()).collect()
    }

    // ---- Copy and paste --------------------------------------------------------------------------------------------------

    /// F34: remembers the rows' nodes in this window's own clipboard.
    pub fn copy_rows(&mut self, selection: &[String]) {
        self.clipboard = Some(self.nodes_of(selection));
    }

    /// F35: each clipboard node gains another parent. Links, not copies.
    pub fn paste_links(&mut self, destination: Option<&str>) -> EditResult {
        let parent = destination.and_then(|d| self.tree.resolve(d)).and_then(|p| p.last().copied());
        let Some(clipboard) = self.live_clipboard() else { return EditResult::default() };
        if !self.tree.can_paste(Some(&clipboard), parent, false) {
            return EditResult::default();
        }
        for n in clipboard {
            let effects = self.tree.add_child_to_parent(n, parent, None);
            self.apply_effects(effects);
        }
        self.finish_paste(destination, parent)
    }

    /// F37: new nodes with new elements, unpatched. Properties come along on the top node and the leaves; tags and patches don't.
    pub fn paste_as_new(&mut self, destination: Option<&str>) -> EditResult {
        let parent = destination.and_then(|d| self.tree.resolve(d)).and_then(|p| p.last().copied());
        let Some(clipboard) = self.live_clipboard() else { return EditResult::default() };
        if !self.tree.can_paste(Some(&clipboard), parent, true) {
            return EditResult::default();
        }
        for n in clipboard {
            self.duplicate_subtree(n, parent);
        }
        self.finish_paste(destination, parent)
    }

    fn finish_paste(&mut self, destination: Option<&str>, parent: Option<Nid>) -> EditResult {
        self.tree_changed();
        let mut result = EditResult::default();
        result.note_parent(parent);
        if let Some(d) = destination {
            result.expand.push(d.to_string());
        }
        result
    }

    /// `DuplicateNodes`.
    pub fn duplicate_subtree(&mut self, node: Nid, parent: Option<Nid>) -> Nid {
        let mut leaf_map: HashMap<Nid, Nid> = HashMap::new();
        let source = self.tree.node(node);
        let (name, is_leaf) = (source.name.clone(), source.children.is_empty());
        let (new_node, effects) = self.tree.create_node(parent, Some(&name), is_leaf, true);
        self.apply_effects(effects);
        self.duplicate_properties(new_node, node);
        if !is_leaf {
            self.duplicate_children(node, new_node, &mut leaf_map);
        }
        new_node
    }

    fn duplicate_children(&mut self, node: Nid, new_node: Nid, leaf_map: &mut HashMap<Nid, Nid>) {
        for child in self.tree.node(node).children.clone() {
            let child_node = self.tree.node(child);
            let (name, is_leaf) = (child_node.name.clone(), child_node.children.is_empty());
            if is_leaf {
                if let Some(existing) = leaf_map.get(&child).copied() {
                    let effects = self.tree.add_child_to_parent(existing, Some(new_node), None);
                    self.apply_effects(effects);
                } else {
                    let (new_child, effects) = self.tree.create_node(Some(new_node), Some(&name), true, true);
                    self.apply_effects(effects);
                    self.duplicate_properties(new_child, child);
                    leaf_map.insert(child, new_child);
                }
            } else {
                // Upstream quirk: a child group doesn't get its properties copied, unlike the top node and leaves, and a group reached twice is duplicated twice.
                let (new_child, effects) = self.tree.create_node(Some(new_node), Some(&name), false, true);
                self.apply_effects(effects);
                self.duplicate_children(child, new_child, leaf_map);
            }
        }
    }

    fn duplicate_properties(&mut self, target: Nid, source: Nid) {
        let copies: Vec<PropertyInstance> = self
            .tree
            .node(source)
            .properties
            .iter()
            .map(|p| {
                let instance_id = Uuid::new_v4();
                PropertyInstance { type_id: p.type_id, instance_id, data: retarget_data(&p.data, &p.instance_id, &instance_id) }
            })
            .collect();
        self.register_unknown_properties(&copies);
        self.tree.node_mut(target).properties.extend(copies);
    }

    // ---- Dropping -------------------------------------------------------------------------------------------------------

    /// F59 plus F75, for the position `NSOutlineView` proposes: `parent` (`None` for the root) and a child index, or `None` for a drop on the row itself.
    pub fn can_drop_rows(&self, sources: &[String], parent: Option<&str>, index: Option<u32>, copy: bool) -> bool {
        let rows = self.rows(sources);
        if rows.is_empty() {
            return false;
        }
        let parent_path = match parent {
            Some(p) => match self.tree.resolve(p) {
                Some(path) => Some(path),
                None => return false,
            },
            None => None,
        };
        if parent.is_none() && index.is_none() {
            return false;
        }
        // F75, as a real ancestor test on rows: the target can't be a dragged row or anything under one.
        if let Some(path) = &parent_path {
            if rows.iter().any(|r| path.starts_with(r)) {
                return false;
            }
        }
        let parent_node = parent_path.as_ref().and_then(|p| p.last().copied());
        let on_row = index.is_none();
        let (invalid, permitted): (HashSet<Nid>, HashSet<Nid>) = match parent_node {
            Some(p) if on_row => (self.tree.invalid_children(p), HashSet::new()),
            Some(p) => {
                let permitted = if copy { HashSet::new() } else { self.tree.node(p).children.iter().copied().collect() };
                (self.tree.invalid_children(p), permitted)
            }
            None => {
                let permitted = if copy { HashSet::new() } else { self.tree.roots.iter().copied().collect() };
                (self.tree.invalid_root_nodes(), permitted)
            }
        };
        rows.iter().map(|r| *r.last().expect("not empty")).all(|n| !invalid.contains(&n) || permitted.contains(&n))
    }

    /// F57, F58. The caller has asked F43 for the new parent. Rows move out of the parent they were selected under; ⌥ gives links instead.
    pub fn drop_rows(&mut self, sources: &[String], parent: Option<&str>, index: Option<u32>, copy: bool) -> EditResult {
        if !self.can_drop_rows(sources, parent, index, copy) {
            return EditResult::default();
        }
        let rows = self.rows(sources);
        let new_parent = parent.and_then(|p| self.tree.resolve(p)).and_then(|p| p.last().copied());
        let mut index = index.map(|i| i as usize);
        let mut result = EditResult::default();
        result.note_parent(new_parent);
        // `treeNode.Index`: where each row sat when the drag began. Upstream's tree isn't redrawn until every row has moved.
        let starts: Vec<usize> = rows
            .iter()
            .map(|row| {
                let node = *row.last().expect("not empty");
                self.tree.child_list(parent_of(row)).iter().position(|n| *n == node).unwrap_or(0)
            })
            .collect();
        for (row, current) in rows.iter().zip(starts) {
            let node = *row.last().expect("not empty");
            if copy {
                let effects = self.tree.add_child_to_parent(node, new_parent, index);
                self.apply_effects(effects);
                if let Some(i) = index.as_mut() {
                    *i += 1;
                }
                continue;
            }
            let old_parent = parent_of(row);
            let effects = self.tree.move_node(node, new_parent, old_parent, index);
            self.apply_effects(effects);
            result.note_parent(old_parent);
            if let Some(i) = index.as_mut() {
                // Moving later in one group, the shuffle already offsets the index. Otherwise it moves on.
                if new_parent != old_parent || *i < current {
                    *i += 1;
                }
            }
        }
        self.tree_changed();
        if let Some(p) = parent {
            result.expand.push(p.to_string());
        }
        // The rows moved, so the old paths are stale. Follow the nodes to where they are now.
        let parent_text = parent.map(str::to_string);
        result.select =
            Some(rows.iter().map(|r| child_path(parent_text.as_deref(), *r.last().expect("not empty"))).collect());
        result
    }

    // ---- Sort, reverse --------------------------------------------------------------------------------------------------

    /// F51, for one selected group.
    pub fn sort_group(&mut self, path: &str) -> EditResult {
        let Some(node) = self.tree.resolve(path).and_then(|p| p.last().copied()) else { return EditResult::default() };
        if self.tree.is_leaf(node) {
            return EditResult::default();
        }
        self.tree.sort_children(node);
        self.tree_changed();
        let mut result = EditResult::default();
        result.note_parent(Some(node));
        result
    }

    /// F50. Selection stays on the same paths.
    pub fn reverse_rows(&mut self, selection: &[String]) -> EditResult {
        let rows = self.rows(selection);
        if !self.tree.reverse_rows(&rows) {
            return EditResult::default();
        }
        self.tree_changed();
        let mut result = EditResult::default();
        result.note_parent(parent_of(&rows[0]));
        result
    }

    // ---- Selecting from elsewhere ----------------------------------------------------------------------------------------

    /// `PopulateNodeTree(elements)` (F27): a row for each node through its first parents, with the ancestors to expand, first to last.
    pub fn reveal_nodes(&self, nodes: &[Nid]) -> EditResult {
        let mut result = EditResult { scroll_to_top: true, ..EditResult::default() };
        let mut seen_expand: HashSet<String> = HashSet::new();
        let mut select = Vec::with_capacity(nodes.len());
        for &n in nodes {
            let path = self.tree.first_parent_path(n);
            for end in 1..path.len() {
                let text = format_path(&path[..end]);
                if seen_expand.insert(text.clone()) {
                    result.expand.push(text);
                }
            }
            select.push(format_path(&path));
        }
        result.select = Some(select);
        result
    }

    /// The row a new node is reached through, for tests and for selecting a template's head.
    pub fn row_path_of(&self, node: Nid) -> String {
        self.row_of_node(node)
    }
}

/// A property's data for another instance: `Unknown` data carries the instance id inside its XML.
pub(crate) fn retarget_data(data: &PropertyData, from: &Uuid, to: &Uuid) -> PropertyData {
    match data {
        PropertyData::Unknown { raw_xml } => {
            PropertyData::Unknown { raw_xml: raw_xml.replace(&from.to_string(), &to.to_string()) }
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    fn path(s: &SetupSession, name: &str) -> String {
        let node = s.tree.live_nodes().find(|(_, n)| n.name == name).unwrap().0;
        s.row_path_of(node)
    }

    fn root_names(s: &SetupSession) -> Vec<String> {
        s.element_rows(None).into_iter().map(|r| r.name).collect()
    }

    fn child_names(s: &SetupSession, parent: &str) -> Vec<String> {
        s.element_rows(Some(parent)).into_iter().map(|r| r.name).collect()
    }

    #[test]
    fn rows_report_icons_children_and_paths() {
        let mut s = session(&["A", "B"], 2);
        let a = path(&s, "A");
        let rows = s.element_rows(None);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].icon, RowIcon::Unpatched);
        assert!(!rows[0].has_children);

        // Patched shows green.
        let summary = s.patch_elements(&[a.clone()], &s.all_outputs_selection(), &Default::default(), false);
        assert_eq!(summary, 1);
        assert_eq!(s.element_icons(&[a.clone()]), [RowIcon::Patched]);

        let group = s.add_element(Some(&a), "Kid");
        assert_eq!(group.reload_nodes.len(), 1);
        assert_eq!(s.element_icons(&[a.clone()]), [RowIcon::Group]);
    }

    #[test]
    fn stale_paths_are_harmless() {
        let mut s = session(&["A"], 0);
        let a = path(&s, "A");
        s.delete_rows(&[a.clone()]);
        assert!(s.element_rows(Some(&a)).is_empty());
        assert_eq!(s.element_rows_at(&[a.clone()]), vec![None]);
        assert_eq!(s.element_icons(&[a.clone()]), [RowIcon::Unpatched]);
        assert_eq!(s.delete_rows(&[a.clone()]), EditResult::default());
        assert!(s.element_rows_at(&["garbage".to_string()])[0].is_none());
    }

    #[test]
    fn add_names_and_selects_and_converts_the_parent() {
        let mut s = session(&["A"], 1);
        let a = path(&s, "A");
        s.patch_elements(&[a.clone()], &s.all_outputs_selection(), &Default::default(), false);
        assert!(s.would_lose_patches(Some(&a)));
        let result = s.add_element(Some(&a), "");
        assert_eq!(child_names(&s, &a), ["New Element"]);
        assert_eq!(result.select.as_ref().unwrap().len(), 1);
        assert_eq!(result.expand, [a.clone()]);
        // The element went, and its patch with it.
        assert!(!s.would_lose_patches(Some(&a)));
        assert_eq!(s.flow.to_patches().len(), 0);
        assert!(s.dirty.flow && s.dirty.tree);

        let result = s.add_element(None, "New Element");
        assert!(result.reload_roots);
        assert_eq!(root_names(&s), ["A", "New Element - 2"]);
    }

    #[test]
    fn add_multiple_skips_empty_names() {
        let mut s = session(&[], 0);
        let names = vec!["X-1".to_string(), String::new(), "X-2".to_string()];
        let result = s.add_elements(None, &names);
        assert_eq!(root_names(&s), ["X-1", "X-2"]);
        assert_eq!(result.select.unwrap().len(), 2);
        assert_eq!(s.add_elements(None, &[String::new()]).select, None);
    }

    #[test]
    fn delete_only_takes_this_rows_link() {
        let mut s = session(&["N"], 0);
        let n = path(&s, "N");
        let g = {
            s.add_element(None, "G");
            path(&s, "G")
        };
        s.copy_rows(&[n.clone()]);
        s.paste_links(Some(&g));
        let under_group = format!("{g}/{}", n);
        assert_eq!(child_names(&s, &g), ["N"]);
        s.delete_rows(&[under_group]);
        // The group emptied, so it's a leaf again, with a new element.
        assert_eq!(s.element_rows(None).iter().find(|r| r.name == "G").unwrap().has_children, false);
        assert!(root_names(&s).contains(&"N".to_string()));
        s.delete_rows(&[n]);
        assert_eq!(root_names(&s), ["G"]);
    }

    #[test]
    fn create_group_links_and_leaves_the_nodes_where_they_were() {
        let mut s = session(&["A", "B"], 0);
        let (a, b) = (path(&s, "A"), path(&s, "B"));
        let result = s.create_group(&[a, b], "Group");
        assert!(result.reload_roots);
        assert_eq!(root_names(&s), ["A", "B", "Group"]);
        let g = path(&s, "Group");
        assert_eq!(child_names(&s, &g), ["A", "B"]);
        assert_eq!(s.element_icons(&[g]), [RowIcon::Group]);
    }

    #[test]
    fn find_replace_leaves_unmatched_rows_alone() {
        let mut s = session(&["Tree 1", "Other"], 0);
        let rows = vec![path(&s, "Tree 1"), path(&s, "Other")];
        let old = s.names_of(&rows);
        let new = crate::setup::naming::replace_patterns(&old, &[("Tree".to_string(), "Arch".to_string())]);
        let result = s.rename_rows(&rows, &new);
        assert_eq!(root_names(&s), ["Arch 1", "Other"]);
        assert_eq!(result.refresh_nodes.len(), 1);
        // Fewer names than rows stops early.
        s.rename_rows(&rows, &["Only".to_string()]);
        assert_eq!(root_names(&s), ["Only", "Other"]);
        // A taken name is uniquified.
        s.rename_rows(&rows[..1], &["Other".to_string()]);
        assert_eq!(root_names(&s), ["Other - 2", "Other"]);
    }

    #[test]
    fn paste_rules() {
        let mut s = session(&["A", "B"], 0);
        let (a, b) = (path(&s, "A"), path(&s, "B"));
        let (a_node, b_node) = (*s.tree.resolve(&a).unwrap().last().unwrap(), *s.tree.resolve(&b).unwrap().last().unwrap());
        assert_eq!(s.clipboard_state(None), (false, false));
        s.copy_rows(&[a.clone()]);
        // A is a root already, and can't go inside itself.
        assert_eq!(s.clipboard_state(None), (false, true));
        assert_eq!(s.clipboard_state(Some(a_node)), (false, false));
        assert_eq!(s.clipboard_state(Some(b_node)), (true, true));
        assert_eq!(s.paste_links(None), EditResult::default());

        s.paste_links(Some(&b));
        assert_eq!(child_names(&s, &b), ["A"]);
        // It's still the same node: renaming shows in both places.
        let under = format!("{b}/{a}");
        s.rename_rows(&[under], &["Renamed".to_string()]);
        assert_eq!(root_names(&s)[0], "Renamed");
    }

    #[test]
    fn paste_as_new_duplicates_with_upstream_gaps() {
        let mut s = session(&["Top", "Mid", "Shared"], 1);
        let (top, mid, shared) = (path(&s, "Top"), path(&s, "Mid"), path(&s, "Shared"));
        // Top > Mid > Shared, and Top > Shared too.
        s.copy_rows(&[mid.clone()]);
        s.paste_links(Some(&top));
        s.copy_rows(&[shared.clone()]);
        s.paste_links(Some(&format!("{top}/{mid}")));
        s.paste_links(Some(&top));
        // Now the roots Mid and Shared can go.
        s.delete_rows(&[shared.clone(), mid.clone()]);
        assert_eq!(root_names(&s), ["Top"]);

        // Properties on the top node, on the child group and on the leaf.
        let add = |s: &mut SetupSession, row: &str| {
            let node = *s.tree.resolve(row).unwrap().last().unwrap();
            s.tree.node_mut(node).properties.push(PropertyInstance {
                type_id: vixen_core::element::property_ids::ORDER,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Order { order: 7 },
            });
            s.tree.node_mut(node).tags.push(Uuid::new_v4());
        };
        add(&mut s, &top);
        add(&mut s, &format!("{top}/{mid}"));
        add(&mut s, &format!("{top}/{mid}/{shared}"));
        // Patch the leaf, to show patches don't come along.
        let leaf = format!("{top}/{mid}/{shared}");
        s.patch_elements(&[leaf], &s.all_outputs_selection(), &Default::default(), false);

        s.copy_rows(&[top.clone()]);
        s.paste_as_new(None);
        assert_eq!(root_names(&s), ["Top", "Top - 2"]);
        let copy = path(&s, "Top - 2");
        let copy_node = *s.tree.resolve(&copy).unwrap().last().unwrap();
        assert_eq!(child_names(&s, &copy), ["Mid - 2", "Shared - 2"]);
        assert_eq!(s.tree.node(copy_node).properties.len(), 1, "the top node's properties come");
        assert!(s.tree.node(copy_node).tags.is_empty(), "tags don't");
        let mid_copy = s.tree.node(copy_node).children[0];
        assert!(s.tree.node(mid_copy).properties.is_empty(), "a child group's don't");
        let leaf_copy = s.tree.node(copy_node).children[1];
        assert_eq!(s.tree.node(mid_copy).children, [leaf_copy], "a leaf reached twice stays one element");
        assert_eq!(s.tree.node(leaf_copy).properties.len(), 1);
        let leaf_element = s.flow.find(&s.tree.node(leaf_copy).element_id.unwrap()).unwrap();
        assert!(s.flow.destinations(leaf_element).is_empty(), "patches don't");
        assert_ne!(s.tree.node(leaf_copy).properties[0].instance_id, s.tree.node(*s.tree.resolve(&format!("{top}/{mid}/{shared}")).unwrap().last().unwrap()).properties[0].instance_id);
    }

    #[test]
    fn drop_reorders_at_the_root() {
        let mut s = session(&["A", "B", "C", "D"], 0);
        let rows: Vec<String> = ["A", "B", "C", "D"].iter().map(|n| path(&s, n)).collect();
        // A, B to below D.
        s.drop_rows(&rows[..2], None, Some(4), false);
        assert_eq!(root_names(&s), ["C", "D", "A", "B"]);

        let mut s = session(&["A", "B", "C", "D"], 0);
        let rows: Vec<String> = ["A", "B", "C", "D"].iter().map(|n| path(&s, n)).collect();
        // C, D to above A.
        s.drop_rows(&rows[2..], None, Some(0), false);
        assert_eq!(root_names(&s), ["C", "D", "A", "B"]);

        let mut s = session(&["A", "B", "C", "D"], 0);
        let rows: Vec<String> = ["A", "B", "C", "D"].iter().map(|n| path(&s, n)).collect();
        // A to below B.
        s.drop_rows(&rows[..1], None, Some(2), false);
        assert_eq!(root_names(&s), ["B", "A", "C", "D"]);
    }

    /// `treeviewDragFinishingHandler` compares the target with `treeNode.Index`, which is where the row sat when the drag began:
    /// the tree isn't redrawn until every row has moved. With a row from elsewhere landing just above two of the group's own, the
    /// first of those doesn't push the target on, and the second lands above it.
    #[test]
    fn drop_uses_each_rows_position_from_before_the_drag() {
        let mut s = session(&["X", "P", "A", "B", "C"], 0);
        let (x, p) = (path(&s, "X"), path(&s, "P"));
        let kids: Vec<String> = ["A", "B", "C"].iter().map(|n| path(&s, n)).collect();
        s.drop_rows(&kids, Some(&p), None, false);
        assert_eq!(child_names(&s, &p), ["A", "B", "C"]);
        let (b, c) = (format!("{p}/{}", kids[1]), format!("{p}/{}", kids[2]));
        s.drop_rows(&[x, b, c], Some(&p), Some(0), false);
        assert_eq!(child_names(&s, &p), ["X", "C", "B", "A"]);

        // Inside one group the two agree. B, D to above B: B stays, and D lands above it.
        let mut s = session(&["A", "B", "C", "D", "E"], 0);
        let rows = vec![path(&s, "B"), path(&s, "D")];
        s.drop_rows(&rows, None, Some(1), false);
        assert_eq!(root_names(&s), ["A", "D", "B", "C", "E"]);
    }

    #[test]
    fn drop_on_a_row_and_copy_mode() {
        let mut s = session(&["G", "N", "M"], 0);
        let (g, n, m) = (path(&s, "G"), path(&s, "N"), path(&s, "M"));
        let result = s.drop_rows(&[n.clone()], Some(&g), None, false);
        assert_eq!(child_names(&s, &g), ["N"]);
        assert_eq!(root_names(&s), ["G", "M"]);
        assert_eq!(result.expand, [g.clone()]);
        assert_eq!(result.select.unwrap(), [format!("{g}/{n}")]);

        // ⌥: a link, and the source stays.
        s.drop_rows(&[m.clone()], Some(&g), Some(0), true);
        assert_eq!(child_names(&s, &g), ["M", "N"]);
        assert_eq!(root_names(&s), ["G", "M"]);
    }

    #[test]
    fn drop_validity() {
        let mut s = session(&["Parent", "Tree", "Tree 2", "Free"], 0);
        let (parent, tree, tree2, free) = (path(&s, "Parent"), path(&s, "Tree"), path(&s, "Tree 2"), path(&s, "Free"));
        s.drop_rows(&[tree.clone()], Some(&parent), None, false);
        let child = format!("{parent}/{tree}");

        // Reorder inside a parent: allowed on a move, refused on a copy.
        assert!(s.can_drop_rows(&[child.clone()], Some(&parent), Some(0), false));
        assert!(!s.can_drop_rows(&[child.clone()], Some(&parent), Some(0), true));
        // Onto its own parent as a drop on the row is refused: it's already a child.
        assert!(!s.can_drop_rows(&[child.clone()], Some(&parent), None, false));
        // A node into its own descendant, or onto itself.
        assert!(!s.can_drop_rows(&[parent.clone()], Some(&child), None, false));
        assert!(!s.can_drop_rows(&[free.clone()], Some(&free), None, false));
        // F75: no text prefix test, so "Tree" can sit beside "Tree 2".
        assert!(s.can_drop_rows(&[child.clone()], None, Some(0), false));
        assert!(s.can_drop_rows(&[child], Some(&tree2), None, false));
        // At the root.
        assert!(s.can_drop_rows(&[free.clone()], None, Some(0), false));
        assert!(!s.can_drop_rows(&[free.clone()], None, Some(0), true), "a second root link");
        // No target at all.
        assert!(!s.can_drop_rows(&[free], None, None, false));
    }

    #[test]
    fn menu_rules_follow_f32() {
        let mut s = session(&["A", "B", "C"], 0);
        let (a, b, c) = (path(&s, "A"), path(&s, "B"), path(&s, "C"));
        let none = s.context_menu_state(&[], None);
        assert!(!none.copy && !none.delete && !none.tags && !none.sort && none.export_element_tree);

        let one = s.context_menu_state(&[a.clone()], Some(&a));
        assert!(one.copy && one.copy_properties && !one.paste_properties && !one.reverse && !one.sort && !one.export_wire_diagram);

        let two = s.context_menu_state(&[a.clone(), b.clone()], Some(&b));
        assert!(two.reverse && !two.copy_properties && two.rename);
        assert!(!s.context_menu_state(&[a.clone(), c.clone()], Some(&c)).reverse);

        s.add_element(Some(&a), "Kid");
        let group = s.context_menu_state(&[a.clone()], Some(&a));
        assert!(group.sort && group.export_wire_diagram);

        s.copy_properties(&a);
        assert!(s.context_menu_state(&[b.clone()], Some(&b)).paste_properties);
        s.delete_rows(&[a, b, c]);
        assert!(!s.context_menu_state(&[], None).export_element_tree);
    }

    #[test]
    fn reveal_expands_ancestors_and_uses_the_first_parent() {
        let mut s = session(&["G", "N"], 0);
        let (g, n) = (path(&s, "G"), path(&s, "N"));
        s.drop_rows(&[n.clone()], Some(&g), None, false);
        let node = *s.tree.resolve(&format!("{g}/{n}")).unwrap().last().unwrap();
        let reveal = s.reveal_nodes(&[node]);
        assert_eq!(reveal.expand, [g.clone()]);
        assert_eq!(reveal.select.unwrap(), [format!("{g}/{n}")]);
        assert!(reveal.scroll_to_top);
    }

    #[test]
    fn sort_and_reverse_report_the_parent() {
        let mut s = session(&["G", "b", "a"], 0);
        let (g, b, a) = (path(&s, "G"), path(&s, "b"), path(&s, "a"));
        s.drop_rows(&[b.clone(), a.clone()], Some(&g), None, false);
        let result = s.sort_group(&g);
        assert_eq!(result.reload_nodes.len(), 1);
        assert_eq!(child_names(&s, &g), ["a", "b"]);
        let result = s.reverse_rows(&[format!("{g}/{a}"), format!("{g}/{b}")]);
        assert_eq!(result.reload_nodes.len(), 1);
        assert_eq!(child_names(&s, &g), ["b", "a"]);
        assert_eq!(s.sort_group(&format!("{g}/{a}")), EditResult::default());
    }
}
