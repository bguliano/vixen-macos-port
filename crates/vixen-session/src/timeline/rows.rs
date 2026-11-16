//! The row tree: `Row`, `RowList`, `RowLabel` and the row parts of `TimelineControl`. F18 to F38, and the row settings saved
//! with a sequence.

use super::md5::md5_hex_upper;
use super::{IRect, RowId};
use std::collections::HashMap;
use uuid::{uuid, Uuid};
use vixen_core::{ElementTree, NodeId};
use vixen_io::RowSetting;

pub const DEFAULT_ROW_HEIGHT: i32 = 32;
pub const MIN_ROW_HEIGHT: i32 = 10;
/// The sequencer's `ToggleTreeButtonWidth`: an icon's 16 plus 8.
pub const TOGGLE_BUTTON_WIDTH: i32 = 24;
/// `ResizeBarWidth`: the bottom points of a label that drag its height.
pub const RESIZE_BAR_HEIGHT: i32 = 4;

/// `BuiltInElementTags.HiddenId`.
pub const HIDDEN_TAG: Uuid = uuid!("2f21bfcb-07a3-4cac-bdc5-4b6e37773c04");
/// `BuiltInElementTags.DeprecatedId`.
pub const DEPRECATED_TAG: Uuid = uuid!("753115da-e27a-4261-a136-8222ccc3f22e");

/// One effect in one row. Times are copied here so stacking, hit testing and painting never touch the effect table.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Occ {
    pub id: super::ElementId,
    pub start: i64,
    pub end: i64,
    pub index: i32,
    pub count: i32,
    /// Being moved or resized: drawn by its own layer instead of a tile.
    pub moving: bool,
}

impl Occ {
    #[inline]
    pub fn slot(&self) -> super::StackSlot {
        super::StackSlot { index: self.index, count: self.count }
    }
}

/// One row.
#[derive(Clone, Debug)]
pub struct Row {
    pub id: RowId,
    pub node: Option<NodeId>,
    pub node_uuid: Uuid,
    pub name: String,
    pub parent: Option<RowId>,
    pub children: Vec<RowId>,
    pub depth: i32,
    pub height: i32,
    pub tree_open: bool,
    /// What `Visible` was last set to. The effective value also needs the filter.
    pub requested_visible: bool,
    pub visible: bool,
    pub selected: bool,
    /// Separate from selection: the paste and add-effect target.
    pub active: bool,
    pub tags: Vec<Uuid>,
    /// Sorted by start then end.
    pub occurrences: Vec<Occ>,
    /// `prefix_max_end[i]` is the latest end among occurrences `0..=i`, so the first one still running at a time is a binary search.
    pub prefix_max_end: Vec<i64>,
    /// Whether any two occurrences overlap. Nearly all rows don't, and then every slot is index 0 of 1 whatever the view.
    pub has_overlap: bool,
    /// The range the current slots were walked for, in occurrence indices.
    pub(crate) walk_key: Option<(usize, usize)>,
    /// In the order selected: `m_selectedElements`.
    pub selected_elements: Vec<super::ElementId>,
    /// How many occurrences are in rows below this one, so `IsTreeActive` needs no walk.
    pub subtree_occurrences: u32,
    /// Waiting in the timeline's list for its occurrences to be refreshed and sorted.
    pub(crate) queued_sort: bool,
    /// All of them, not only the ones being moved.
    pub(crate) queued_sort_all: bool,
}

impl Row {
    #[inline]
    pub fn has_hidden_tag(&self) -> bool {
        self.tags.contains(&HIDDEN_TAG)
    }

    #[inline]
    pub fn has_deprecated_tag(&self) -> bool {
        self.tags.contains(&DEPRECATED_TAG)
    }

    /// Recomputes what is derived from the sorted list. Returns whether overlap appeared or went away.
    pub(crate) fn rebuild_derived(&mut self) {
        self.prefix_max_end.clear();
        self.prefix_max_end.reserve(self.occurrences.len());
        let mut max_end = i64::MIN;
        let mut overlap = false;
        for o in &self.occurrences {
            if o.start < max_end {
                overlap = true;
            }
            max_end = max_end.max(o.end);
            self.prefix_max_end.push(max_end);
        }
        self.has_overlap = overlap;
        self.walk_key = None;
    }

    /// Position of an occurrence: a binary search on its times, then a scan of equals.
    pub(crate) fn find_occ(&self, id: super::ElementId, start: i64, end: i64) -> Option<usize> {
        let mut i = self.occurrences.partition_point(|o| (o.start, o.end) < (start, end));
        while let Some(o) = self.occurrences.get(i) {
            if (o.start, o.end) != (start, end) {
                return None;
            }
            if o.id == id {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

/// Where a point lands in a row's label. F19, F25, F30. Upstream works these areas out during paint.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LabelZone {
    Expander,
    Label,
    ResizeBar,
    None,
}

/// One row as the row header draws it.
#[derive(Clone, PartialEq, Debug)]
pub struct RowDisplay {
    pub id: RowId,
    pub name: String,
    pub depth: i32,
    pub height: i32,
    pub top: i32,
    pub expandable: bool,
    pub open: bool,
    pub has_effects: bool,
    pub subtree_has_effects: bool,
    pub selected: bool,
    pub active: bool,
    pub tags: Vec<Uuid>,
}

/// A dotted connector on the row list's background. F22.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ConnectorLine {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

/// `Rows` plus the layout worked out from them. Row ids are positions in depth-first order, upstream's flat `Rows` list.
#[derive(Clone, Debug)]
pub struct RowTree {
    pub rows: Vec<Row>,
    pub roots: Vec<RowId>,
    /// `TimelineControl.rowHeight`, what new rows and the reset commands use.
    pub row_height: i32,
    visible: Vec<RowId>,
    /// `tops[i]` is the top of visible row `i`; one extra entry holds the canvas height.
    tops: Vec<i32>,
    /// A row's position among visible rows, or `u32::MAX`.
    visible_index: Vec<u32>,
    node_rows: HashMap<NodeId, Vec<RowId>>,
    show_hidden: bool,
    /// Set once the host has assigned a filter, which it does at every load.
    filtering: bool,
    /// Bumps with every layout change.
    pub revision: u64,
}

impl Default for RowTree {
    fn default() -> Self {
        RowTree {
            rows: Vec::new(),
            roots: Vec::new(),
            row_height: DEFAULT_ROW_HEIGHT,
            visible: Vec::new(),
            tops: vec![0],
            visible_index: Vec::new(),
            node_rows: HashMap::new(),
            show_hidden: false,
            filtering: false,
            revision: 0,
        }
    }
}

impl RowTree {
    /// `LoadSystemNodesToRows`: one row per occurrence of each node, depth first, then saved settings in flat order, then the
    /// hidden row filter. A `default_height` of 0 means 32.
    pub fn load_rows(&mut self, tree: &ElementTree, settings: &[(String, RowSetting)], default_height: i32) {
        *self = RowTree { row_height: if default_height != 0 { default_height } else { DEFAULT_ROW_HEIGHT }, ..RowTree::default() };
        for root in &tree.roots {
            let id = self.add_node_as_row(tree, *root, None);
            self.roots.push(id);
            self.set_visible(id, true);
        }
        if !settings.is_empty() {
            let by_id: HashMap<&str, &RowSetting> = settings.iter().map(|(k, v)| (k.as_str(), v)).collect();
            for i in 0..self.rows.len() {
                let id = RowId(i as u32);
                if let Some(setting) = by_id.get(self.tree_id(id).as_str()) {
                    self.set_tree_open(id, setting.expanded);
                    self.rows[i].height = setting.row_height.max(MIN_ROW_HEIGHT);
                    self.set_visible(id, setting.visible);
                }
            }
        }
        self.set_visibility_filter(false);
        self.relayout();
    }

    fn add_node_as_row(&mut self, tree: &ElementTree, node: NodeId, parent: Option<RowId>) -> RowId {
        let id = RowId(self.rows.len() as u32);
        let n = tree.node(node);
        let open = parent.is_some_and(|p| self.rows[p.idx()].tree_open);
        self.rows.push(Row {
            id,
            node: Some(node),
            node_uuid: n.id,
            name: n.name.clone(),
            parent,
            children: Vec::new(),
            depth: parent.map_or(0, |p| self.rows[p.idx()].depth + 1),
            height: self.row_height,
            tree_open: false,
            requested_visible: open,
            visible: open,
            selected: false,
            active: false,
            tags: n.tags.clone(),
            occurrences: Vec::new(),
            prefix_max_end: Vec::new(),
            has_overlap: false,
            walk_key: None,
            selected_elements: Vec::new(),
            subtree_occurrences: 0,
            queued_sort: false,
            queued_sort_all: false,
        });
        if let Some(p) = parent {
            self.rows[p.idx()].children.push(id);
        }
        self.node_rows.entry(node).or_default().push(id);
        for child in &n.children {
            self.add_node_as_row(tree, *child, Some(id));
        }
        id
    }

    #[inline]
    pub fn row(&self, id: RowId) -> &Row {
        &self.rows[id.idx()]
    }

    #[inline]
    pub fn row_mut(&mut self, id: RowId) -> &mut Row {
        &mut self.rows[id.idx()]
    }

    /// Every row that shows a node.
    pub fn rows_of_node(&self, node: NodeId) -> &[RowId] {
        self.node_rows.get(&node).map_or(&[], Vec::as_slice)
    }

    /// `Row.TreePathName`: this name then each parent's, with nothing between.
    pub fn tree_path_name(&self, id: RowId) -> String {
        let mut s = String::new();
        let mut at = Some(id);
        while let Some(r) = at {
            s.push_str(&self.rows[r.idx()].name);
            at = self.rows[r.idx()].parent;
        }
        s
    }

    /// `Row.TreeId`. Not unique across same-named siblings. KEEP.
    pub fn tree_id(&self, id: RowId) -> String {
        md5_hex_upper(&self.tree_path_name(id))
    }

    /// `SaveGridRowSettings`: rows differing from the default height by more than 7, and open rows. Upstream's `Dictionary.Add`
    /// would throw on a repeated key; here the last write wins, in the first one's place.
    pub fn save_row_settings(&self) -> Vec<(String, RowSetting)> {
        let mut out: Vec<(String, RowSetting)> = Vec::new();
        let mut at: HashMap<String, usize> = HashMap::new();
        for row in &self.rows {
            let differs = row.height > self.row_height + 7 || row.height < self.row_height - 7;
            if !differs && !row.tree_open {
                continue;
            }
            let setting = RowSetting { row_height: row.height, expanded: row.tree_open, visible: row.visible };
            let key = self.tree_id(row.id);
            match at.get(&key) {
                Some(i) => out[*i].1 = setting,
                None => {
                    at.insert(key.clone(), out.len());
                    out.push((key, setting));
                }
            }
        }
        out
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Visibility
    // ------------------------------------------------------------------------------------------------------------------

    fn passes_filter(&self, id: RowId) -> bool {
        !self.filtering || self.show_hidden || !self.rows[id.idx()].has_hidden_tag()
    }

    /// `HasExpandableChildRows`: at least one direct child passes the filter.
    pub fn is_expandable(&self, id: RowId) -> bool {
        self.rows[id.idx()].children.iter().any(|c| self.passes_filter(*c))
    }

    /// `RecomputeVisibility`. `TreeOpen` never sets children directly, so a hidden parent can't leak "visible" onto a filtered child.
    fn recompute_visibility(&mut self, id: RowId) {
        let effective = self.rows[id.idx()].requested_visible && self.passes_filter(id);
        let child_visible = effective && self.rows[id.idx()].tree_open;
        for i in 0..self.rows[id.idx()].children.len() {
            let child = self.rows[id.idx()].children[i];
            self.set_visible(child, child_visible);
        }
        self.rows[id.idx()].visible = effective;
    }

    /// The `Visible` setter. Call `relayout` when a batch of changes is done.
    pub fn set_visible(&mut self, id: RowId, visible: bool) {
        self.rows[id.idx()].requested_visible = visible;
        self.recompute_visibility(id);
    }

    /// The `TreeOpen` setter. Call `relayout` when a batch of changes is done.
    pub fn set_tree_open(&mut self, id: RowId, open: bool) {
        if self.rows[id.idx()].tree_open == open {
            return;
        }
        self.rows[id.idx()].tree_open = open;
        self.recompute_visibility(id);
    }

    /// `ToggleTree`: the row and all descendants.
    pub fn toggle_subtree(&mut self, id: RowId, open: bool) {
        self.set_tree_open(id, open);
        for i in 0..self.rows[id.idx()].children.len() {
            let child = self.rows[id.idx()].children[i];
            self.toggle_subtree(child, open);
        }
    }

    /// `ApplyHiddenRowFilter`: rows with the built-in Hidden tag stay hidden unless `show_hidden`. F24. Also collapses a row left
    /// with nothing to expand, so a stale open state can't resurface when the filter relaxes.
    pub fn set_visibility_filter(&mut self, show_hidden: bool) {
        self.show_hidden = show_hidden;
        self.filtering = true;
        for i in 0..self.roots.len() {
            let root = self.roots[i];
            self.assign_filter(root);
        }
        self.relayout();
    }

    fn assign_filter(&mut self, id: RowId) {
        for i in 0..self.rows[id.idx()].children.len() {
            let child = self.rows[id.idx()].children[i];
            self.assign_filter(child);
        }
        self.recompute_visibility(id);
        if self.rows[id.idx()].tree_open && !self.is_expandable(id) {
            self.set_tree_open(id, false);
        }
    }

    pub fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Layout
    // ------------------------------------------------------------------------------------------------------------------

    /// Rebuilds visible rows and their tops. One per user action.
    pub fn relayout(&mut self) {
        self.visible.clear();
        self.tops.clear();
        self.visible_index.clear();
        self.visible_index.resize(self.rows.len(), u32::MAX);
        let mut top = 0;
        for row in &self.rows {
            if row.visible {
                self.visible_index[row.id.idx()] = self.visible.len() as u32;
                self.visible.push(row.id);
                self.tops.push(top);
                top += row.height;
            }
        }
        self.tops.push(top);
        self.revision += 1;
    }

    /// `VisibleRows`.
    #[inline]
    pub fn visible(&self) -> &[RowId] {
        &self.visible
    }

    /// Cumulative tops for the row header, with the canvas height as one extra entry.
    #[inline]
    pub fn row_tops(&self) -> &[i32] {
        &self.tops
    }

    /// `CalculateAllRowsHeight`.
    #[inline]
    pub fn canvas_height(&self) -> i32 {
        *self.tops.last().unwrap_or(&0)
    }

    #[inline]
    pub fn visible_index_of(&self, id: RowId) -> Option<usize> {
        match self.visible_index.get(id.idx()) {
            Some(&i) if i != u32::MAX => Some(i as usize),
            _ => None,
        }
    }

    /// `DisplayTop`, for a visible row.
    #[inline]
    pub fn row_top(&self, id: RowId) -> Option<i32> {
        self.visible_index_of(id).map(|i| self.tops[i])
    }

    /// F44. Negative y gives the first row, below the end gives none.
    pub fn row_at(&self, canvas_y: i32) -> Option<RowId> {
        let i = self.tops[1..].partition_point(|&bottom| bottom <= canvas_y);
        self.visible.get(i).copied()
    }

    /// `RowsIn`: rows touching a rectangle with their tops. Can include one extra row when a boundary lands exactly on a row edge.
    pub fn rows_in(&self, r: IRect) -> Vec<(RowId, i32)> {
        let mut out = Vec::new();
        let first = self.tops[1..].partition_point(|&bottom| bottom <= r.y);
        let bottom = r.bottom();
        for i in first..self.visible.len() {
            out.push((self.visible[i], self.tops[i]));
            if self.tops[i + 1] > bottom {
                break;
            }
        }
        out
    }

    /// `GetRowsWithin`: visible rows from the one at the top-left to the one at the bottom-right. A bottom past the last row runs to
    /// the last row.
    pub fn rows_within(&self, r: IRect) -> Vec<RowId> {
        let Some(start) = self.row_at(r.y).and_then(|s| self.visible_index_of(s)) else { return Vec::new() };
        let end = self.row_at(r.bottom()).and_then(|e| self.visible_index_of(e));
        match end {
            Some(end) if end >= start => self.visible[start..=end].to_vec(),
            Some(_) => Vec::new(),
            None => self.visible[start..].to_vec(),
        }
    }

    /// The rows the row header draws, top to bottom, for a span of the canvas.
    pub fn visible_rows(&self, top: i32, bottom: i32) -> Vec<RowDisplay> {
        let first = self.tops[1..].partition_point(|&b| b <= top);
        let mut out = Vec::new();
        for i in first..self.visible.len() {
            if self.tops[i] >= bottom {
                break;
            }
            let row = &self.rows[self.visible[i].idx()];
            out.push(RowDisplay {
                id: row.id,
                name: row.name.clone(),
                depth: row.depth,
                height: row.height,
                top: self.tops[i],
                expandable: self.is_expandable(row.id),
                open: row.tree_open,
                has_effects: !row.occurrences.is_empty(),
                subtree_has_effects: row.subtree_occurrences > 0,
                selected: row.selected,
                active: row.active,
                tags: row.tags.clone(),
            });
        }
        out
    }

    /// F22 for a span of the canvas. Upstream walks every visible row; a branch's line only depends on its parent's bottom and the
    /// middle of its last visible child, so this needs the rows in view and their ancestors.
    pub fn connector_lines(&self, top: i32, bottom: i32, inset: i32) -> Vec<ConnectorLine> {
        let first = self.tops[1..].partition_point(|&b| b <= top);
        let mut out = Vec::new();
        let mut parents: Vec<RowId> = Vec::new();
        for i in first..self.visible.len() {
            if self.tops[i] >= bottom {
                break;
            }
            let row = &self.rows[self.visible[i].idx()];
            if row.depth > 0 {
                let mid = (self.tops[i] as f64 + row.height as f64 * 0.5) as i32;
                out.push(ConnectorLine { x0: stub_x(row.depth, inset), y0: mid, x1: row.depth * inset, y1: mid });
            }
            let mut at = row.parent;
            while let Some(p) = at {
                if parents.contains(&p) {
                    break;
                }
                parents.push(p);
                at = self.rows[p.idx()].parent;
            }
        }
        for p in parents {
            let parent = &self.rows[p.idx()];
            let (Some(pi), Some(last)) = (self.visible_index_of(p), parent.children.iter().rev().copied().find(|c| self.rows[c.idx()].visible))
            else {
                continue;
            };
            let Some(li) = self.visible_index_of(last) else { continue };
            let last_row = &self.rows[last.idx()];
            let y1 = (self.tops[li] as f64 + last_row.height as f64 * 0.5) as i32;
            let y0 = self.tops[pi + 1];
            if y0 < bottom && y1 >= top {
                let x = stub_x(last_row.depth, inset);
                out.push(ConnectorLine { x0: x, y0, x1: x, y1 });
            }
        }
        out
    }

    /// `row_label_zone`: `x` and `y` are relative to the row list's left and the row's top, `width` is the list's.
    pub fn row_label_zone(&self, id: RowId, x: i32, y: i32, width: i32) -> LabelZone {
        let row = &self.rows[id.idx()];
        let label_x = x - row.depth * TOGGLE_BUTTON_WIDTH;
        let label_width = width - row.depth * TOGGLE_BUTTON_WIDTH;
        if label_x < 0 || label_x >= label_width || y < 0 || y >= row.height {
            return LabelZone::None;
        }
        let expandable = self.is_expandable(id);
        let icon_width = if expandable || row.depth == 0 { TOGGLE_BUTTON_WIDTH } else { TOGGLE_BUTTON_WIDTH / 2 };
        let in_icon = label_x < icon_width;
        if in_icon {
            return if expandable { LabelZone::Expander } else { LabelZone::Label };
        }
        if y > row.height - RESIZE_BAR_HEIGHT {
            return LabelZone::ResizeBar;
        }
        LabelZone::Label
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Heights
    // ------------------------------------------------------------------------------------------------------------------

    /// The `Height` setter. Floor 10.
    pub fn set_row_height(&mut self, id: RowId, height: i32) {
        self.rows[id.idx()].height = height.max(MIN_ROW_HEIGHT);
        self.relayout();
    }

    /// On release of a height drag every row with the same name takes that height. KEEP: this matches by display name, not by node.
    pub fn finish_row_resize(&mut self, id: RowId) {
        let (height, name) = {
            let row = &self.rows[id.idx()];
            (row.height, row.name.clone())
        };
        for row in &mut self.rows {
            if row.name == name {
                row.height = height;
            }
        }
        self.relayout();
    }

    /// `ZoomRows`. A row stays as it is when scaling would make it taller than the grid, while the default still scales.
    pub fn zoom_rows(&mut self, scale: f64, grid_height: i32) {
        if scale <= 0.0 {
            return;
        }
        self.row_height = (self.row_height as f64 * scale) as i32;
        for row in &mut self.rows {
            if row.height as f64 * scale > grid_height as f64 {
                continue;
            }
            row.height = ((row.height as f64 * scale) as i32).max(MIN_ROW_HEIGHT);
        }
        self.relayout();
    }

    /// `zoomRowHeight`. FIX Q2: uses the row at the pointer's canvas y rather than upstream's offset that ignores scroll and the marks bar.
    pub fn zoom_row_at(&mut self, canvas_y: i32, delta: i32) -> bool {
        let Some(id) = self.row_at(canvas_y) else { return false };
        // Negative y means above the first row, which isn't under the pointer.
        if canvas_y < 0 {
            return false;
        }
        let scale = if delta < 0 { 0.8 } else { 1.25 };
        let height = (self.rows[id.idx()].height as f64 * scale) as i32;
        self.set_row_height(id, height);
        true
    }

    /// F35.
    pub fn reset_row_heights(&mut self) {
        self.row_height = DEFAULT_ROW_HEIGHT;
        for row in &mut self.rows {
            row.height = DEFAULT_ROW_HEIGHT;
        }
        self.relayout();
    }

    /// F36: the first selected row and all descendants take the current default, which F31 may have changed.
    pub fn reset_selected_row_heights(&mut self) -> bool {
        let Some(first) = self.rows.iter().find(|r| r.selected).map(|r| r.id) else { return false };
        let height = self.row_height;
        let mut stack = vec![first];
        while let Some(id) = stack.pop() {
            self.rows[id.idx()].height = height.max(MIN_ROW_HEIGHT);
            stack.extend(self.rows[id.idx()].children.iter().copied());
        }
        self.relayout();
        true
    }

    /// F34.
    pub fn collapse_all(&mut self) {
        for i in 0..self.rows.len() {
            self.set_tree_open(RowId(i as u32), false);
        }
        self.relayout();
    }
}

#[inline]
fn stub_x(depth: i32, inset: i32) -> i32 {
    ((depth as f32 - 0.5) * inset as f32) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::ElementTree;

    /// Arches (Arch 1, Arch 2), Star. Arch 1 also hangs off Star, so it makes two rows.
    fn tree() -> ElementTree {
        let mut t = ElementTree::new();
        let arches = t.add_node(Uuid::new_v4(), "Arches", None);
        let a1 = t.add_element("Arch 1");
        let a2 = t.add_element("Arch 2");
        t.add_child(arches, a1);
        t.add_child(arches, a2);
        let star = t.add_node(Uuid::new_v4(), "Star", None);
        t.add_child(star, a1);
        t
    }

    fn loaded() -> RowTree {
        let mut r = RowTree::default();
        r.load_rows(&tree(), &[], 0);
        r
    }

    fn names(r: &RowTree) -> Vec<&str> {
        r.visible().iter().map(|v| r.row(*v).name.as_str()).collect()
    }

    #[test]
    fn rows_come_depth_first_and_a_shared_node_makes_two() {
        let r = loaded();
        let all: Vec<&str> = r.rows.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(all, ["Arches", "Arch 1", "Arch 2", "Star", "Arch 1"]);
        assert_eq!(names(&r), ["Arches", "Star"]);
        assert_eq!(r.row_tops(), [0, 32, 64]);
        assert_eq!(r.rows_of_node(r.row(RowId(1)).node.unwrap()), [RowId(1), RowId(4)]);
        assert_eq!(r.row(RowId(1)).depth, 1);
    }

    #[test]
    fn expanding_shows_children_and_collapsing_hides_them() {
        let mut r = loaded();
        r.set_tree_open(RowId(0), true);
        r.relayout();
        assert_eq!(names(&r), ["Arches", "Arch 1", "Arch 2", "Star"]);
        assert_eq!(r.canvas_height(), 128);
        r.set_tree_open(RowId(0), false);
        r.relayout();
        assert_eq!(names(&r), ["Arches", "Star"]);
    }

    #[test]
    fn rows_at_points() {
        let mut r = loaded();
        r.set_tree_open(RowId(0), true);
        r.relayout();
        assert_eq!(r.row_at(-5), Some(RowId(0)));
        assert_eq!(r.row_at(0), Some(RowId(0)));
        assert_eq!(r.row_at(31), Some(RowId(0)));
        assert_eq!(r.row_at(32), Some(RowId(1)));
        assert_eq!(r.row_at(127), Some(RowId(3)));
        assert_eq!(r.row_at(128), None);
    }

    #[test]
    fn rows_in_includes_the_one_at_an_exact_boundary() {
        let mut r = loaded();
        r.set_tree_open(RowId(0), true);
        r.relayout();
        let ids = |rect| -> Vec<u32> { r.rows_in(rect).iter().map(|(id, _)| id.0).collect() };
        assert_eq!(ids(IRect::new(0, 10, 5, 5)), [0]);
        // The rectangle's bottom lands exactly on the second row's top.
        assert_eq!(ids(IRect::new(0, 10, 5, 22)), [0, 1]);
        assert_eq!(ids(IRect::new(0, 40, 5, 60)), [1, 2, 3]);
        assert_eq!(r.rows_in(IRect::new(0, 40, 5, 60))[1], (RowId(2), 64));
    }

    #[test]
    fn rows_within_runs_from_top_left_to_bottom_right() {
        let mut r = loaded();
        r.set_tree_open(RowId(0), true);
        r.relayout();
        assert_eq!(r.rows_within(IRect::new(0, 40, 5, 30)), [RowId(1), RowId(2)]);
        assert_eq!(r.rows_within(IRect::new(0, 40, 5, 5000)), [RowId(1), RowId(2), RowId(3)]);
    }

    #[test]
    fn height_floor_and_same_name_sync_only_on_release() {
        let mut r = loaded();
        r.set_row_height(RowId(1), 5);
        assert_eq!(r.row(RowId(1)).height, 10);
        r.set_row_height(RowId(1), 44);
        assert_eq!(r.row(RowId(4)).height, 32);
        r.finish_row_resize(RowId(1));
        assert_eq!(r.row(RowId(4)).height, 44);
        assert_eq!(r.row(RowId(0)).height, 32);
    }

    #[test]
    fn zoom_rows_truncates_and_skips_rows_taller_than_the_grid() {
        let mut r = loaded();
        r.zoom_rows(1.25, 1000);
        assert_eq!((r.row_height, r.row(RowId(0)).height), (40, 40));
        let mut r = loaded();
        r.zoom_rows(0.8, 1000);
        assert_eq!(r.row(RowId(0)).height, 25);
        r.zoom_rows(0.8, 1000);
        assert_eq!(r.row(RowId(0)).height, 20);
        let mut r = loaded();
        r.zoom_rows(2.0, 50);
        assert_eq!(r.row(RowId(0)).height, 32);
        assert_eq!(r.row_height, 64);
        r.zoom_rows(0.0, 50);
        assert_eq!(r.row_height, 64);
        let mut empty = RowTree::default();
        empty.zoom_rows(1.25, 100);
        empty.reset_row_heights();
    }

    #[test]
    fn zoom_row_at_uses_the_row_under_the_pointer() {
        let mut r = loaded();
        assert!(r.zoom_row_at(40, 120));
        assert_eq!(r.row(RowId(3)).height, 40);
        assert!(r.zoom_row_at(40, -120));
        assert_eq!(r.row(RowId(3)).height, 32);
        assert!(!r.zoom_row_at(10_000, 120));
    }

    #[test]
    fn tree_ids_are_the_exact_md5_form() {
        let mut t = ElementTree::new();
        let arches = t.add_node(Uuid::new_v4(), "Arches", None);
        let a1 = t.add_element("Arch 1");
        t.add_child(arches, a1);
        let mut r = RowTree::default();
        r.load_rows(&t, &[], 0);
        assert_eq!(r.tree_id(RowId(0)), "0A9E1587CEC47A218E3B7B1E84D5F22F");
        assert_eq!(r.tree_id(RowId(1)), "D490FDFA296E6757D918AB22E8F6F39D");
    }

    #[test]
    fn row_settings_follow_the_seven_point_rule() {
        let mut r = loaded();
        r.rows[0].height = 40;
        r.rows[2].height = 39;
        r.rows[3].tree_open = true;
        let saved = r.save_row_settings();
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[0], (r.tree_id(RowId(0)), RowSetting { row_height: 40, expanded: false, visible: true }));
        assert_eq!(saved[1].1, RowSetting { row_height: 32, expanded: true, visible: true });
    }

    #[test]
    fn saved_settings_apply_on_load() {
        let t = tree();
        let mut first = RowTree::default();
        first.load_rows(&t, &[], 40);
        assert_eq!(first.row_height, 40);
        let key = first.tree_id(RowId(0));
        let settings = vec![(key, RowSetting { row_height: 77, expanded: true, visible: true })];
        let mut r = RowTree::default();
        r.load_rows(&t, &settings, 0);
        assert_eq!(r.row_height, 32);
        assert_eq!(r.row(RowId(0)).height, 77);
        assert!(r.row(RowId(0)).tree_open);
        assert_eq!(names(&r), ["Arches", "Arch 1", "Arch 2", "Star"]);
    }

    #[test]
    fn hidden_rows_stay_hidden_when_a_parent_opens() {
        let mut t = tree();
        let a2 = t.all_nodes().find(|(_, n)| n.name == "Arch 2").unwrap().0;
        t.node_mut(a2).tags.push(HIDDEN_TAG);
        let mut r = RowTree::default();
        r.load_rows(&t, &[], 0);
        r.set_tree_open(RowId(0), true);
        r.relayout();
        assert_eq!(names(&r), ["Arches", "Arch 1", "Star"]);
        r.set_visibility_filter(true);
        assert_eq!(names(&r), ["Arches", "Arch 1", "Arch 2", "Star"]);
    }

    #[test]
    fn a_filter_that_removes_all_children_closes_the_parent() {
        let mut t = ElementTree::new();
        let group = t.add_node(Uuid::new_v4(), "Group", None);
        let only = t.add_element("Only");
        t.add_child(group, only);
        t.node_mut(only).tags.push(HIDDEN_TAG);
        let mut r = RowTree::default();
        r.load_rows(&t, &[], 0);
        assert!(!r.is_expandable(RowId(0)));
        r.set_visibility_filter(true);
        assert!(r.is_expandable(RowId(0)));
        r.set_tree_open(RowId(0), true);
        r.relayout();
        assert_eq!(r.visible().len(), 2);
        r.set_visibility_filter(false);
        assert!(!r.row(RowId(0)).tree_open);
        assert!(!r.is_expandable(RowId(0)));
        assert_eq!(r.visible().len(), 1);
    }

    #[test]
    fn opening_a_row_under_a_closed_parent_does_not_leak_visibility() {
        let mut t = ElementTree::new();
        let a = t.add_node(Uuid::new_v4(), "A", None);
        let b = t.add_node(Uuid::new_v4(), "B", None);
        let c = t.add_element("C");
        t.add_child(a, b);
        t.add_child(b, c);
        let mut r = RowTree::default();
        r.load_rows(&t, &[], 0);
        r.set_tree_open(RowId(1), true);
        r.relayout();
        assert_eq!(names(&r), ["A"]);
        r.set_tree_open(RowId(0), true);
        r.relayout();
        assert_eq!(names(&r), ["A", "B", "C"]);
    }

    #[test]
    fn subtree_toggle_and_collapse_all() {
        let mut r = loaded();
        r.toggle_subtree(RowId(0), true);
        assert!(r.row(RowId(1)).tree_open);
        r.relayout();
        r.collapse_all();
        assert_eq!(names(&r), ["Arches", "Star"]);
        assert!(!r.rows.iter().any(|x| x.tree_open));
    }

    #[test]
    fn reset_selected_rows_needs_a_selection() {
        let mut r = loaded();
        assert!(!r.reset_selected_row_heights());
        r.zoom_rows(1.25, 1000);
        r.rows[1].height = 90;
        r.rows[0].height = 80;
        r.rows[0].selected = true;
        assert!(r.reset_selected_row_heights());
        assert_eq!(r.row(RowId(0)).height, 40);
        assert_eq!(r.row(RowId(1)).height, 40);
    }

    #[test]
    fn label_zones() {
        let mut r = loaded();
        r.set_tree_open(RowId(0), true);
        r.relayout();
        // A root with children: expander in the first 24, then the label, then the resize bar in the bottom 4 points.
        assert_eq!(r.row_label_zone(RowId(0), 5, 10, 200), LabelZone::Expander);
        assert_eq!(r.row_label_zone(RowId(0), 50, 10, 200), LabelZone::Label);
        assert_eq!(r.row_label_zone(RowId(0), 50, 29, 200), LabelZone::ResizeBar);
        assert_eq!(r.row_label_zone(RowId(0), 50, 28, 200), LabelZone::Label);
        // An indented leaf has a half-width icon area that counts as the label, and no resize bar there.
        assert_eq!(r.row_label_zone(RowId(1), 26, 30, 200), LabelZone::Label);
        assert_eq!(r.row_label_zone(RowId(1), 60, 30, 200), LabelZone::ResizeBar);
        // Left of the indent is the list's background.
        assert_eq!(r.row_label_zone(RowId(1), 5, 10, 200), LabelZone::None);
    }

    #[test]
    fn connectors_match_a_full_walk() {
        let mut r = loaded();
        r.set_tree_open(RowId(0), true);
        r.relayout();
        let lines = r.connector_lines(0, 1000, 24);
        // Stubs for Arch 1 and Arch 2, and one vertical from the bottom of Arches to the middle of Arch 2.
        assert!(lines.contains(&ConnectorLine { x0: 12, y0: 48, x1: 24, y1: 48 }));
        assert!(lines.contains(&ConnectorLine { x0: 12, y0: 80, x1: 24, y1: 80 }));
        assert!(lines.contains(&ConnectorLine { x0: 12, y0: 32, x1: 12, y1: 80 }));
        assert_eq!(lines.len(), 3);
    }
}
