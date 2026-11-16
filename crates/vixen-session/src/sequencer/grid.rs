//! What the window needs from the timeline control, as a trait, so the window's commands can be written and tested while the
//! timeline is built by its own worker, and take its real selection and rows the moment it's linked in.
//!
//! [`ShellGrid`] is a plain implementation: rows from the element tree, a selection list and the view values. It's what tests use and
//! what a build without `VixenTimeline` runs on. The real one is an adapter over `timeline::Timeline`.

use crate::sequence::EffectId;
use crate::sequencer::keys::HotkeyCommand;
use crate::sequencer::prefs::EditorPrefs;
use crate::sequencer::transport::PlaybackRange;
use crate::sequencer::CommandOutcome;
use uuid::{uuid, Uuid};
use vixen_core::{ElementTree, NodeId, TimeSpan};
use vixen_io::RowSetting;

/// `BuiltInElementTags.DeprecatedId`.
pub const DEPRECATED_TAG: Uuid = uuid!("753115da-e27a-4261-a136-8222ccc3f22e");
/// `BuiltInElementTags.HiddenId`.
pub const HIDDEN_TAG: Uuid = uuid!("2f21bfcb-07a3-4cac-bdc5-4b6e37773c04");

/// One visible row, as the window's commands read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowSummary {
    /// One per row occurrence: the same element reached through two groups makes two rows.
    pub id: u32,
    /// The row's element node.
    pub node: Uuid,
    pub name: String,
    pub deprecated: bool,
    /// `Row.Selected`.
    pub selected: bool,
}

/// `TimelineControl` as `TimedSequenceEditorForm` uses it.
pub trait GridPort: Send {
    /// `VisibleRows`, in order.
    fn visible_rows(&self) -> Vec<RowSummary>;
    /// `SelectedRow`.
    fn selected_row(&self) -> Option<u32>;
    /// `ActiveRow`.
    fn active_row(&self) -> Option<u32>;
    /// `TopVisibleRow`.
    fn top_visible_row(&self) -> Option<u32>;
    /// `MoveActiveRow`: true when the key was consumed.
    fn move_active_row(&mut self, direction: i32) -> bool;
    /// The row id of every row occurrence of a node, in row order.
    fn rows_of_node(&self, node: &Uuid) -> Vec<u32>;
    /// `CollapseAllRows`.
    fn collapse_all(&mut self);
    /// `ResetRowHeight`.
    fn reset_row_heights(&mut self);
    /// `ToggleSelectedRows`.
    fn toggle_selected_rows(&mut self, include_children: bool);
    /// `ShowHiddenRows`.
    fn set_show_hidden(&mut self, show: bool);

    /// `SelectedElements`, in selection order.
    fn selected_elements(&self) -> Vec<EffectId>;
    /// `ClearSelectedElements` then select these.
    fn set_selection(&mut self, ids: &[EffectId]);
    fn remove_from_selection(&mut self, ids: &[EffectId]);

    /// `CursorPosition`.
    fn cursor(&self) -> TimeSpan;
    fn set_cursor(&mut self, t: TimeSpan);
    /// `TotalTime`.
    fn set_total(&mut self, total: TimeSpan);
    /// `TimePerPixel`.
    fn time_per_pixel(&self) -> TimeSpan;
    fn set_time_per_pixel(&mut self, tpp: TimeSpan);
    /// `VisibleTimeStart`.
    fn visible_start(&self) -> TimeSpan;
    fn set_visible_start(&mut self, t: TimeSpan);
    /// `VisibleTimeSpan`.
    fn visible_span(&self) -> TimeSpan;
    /// `VerticalOffset`.
    fn vertical_offset(&self) -> i32;
    fn set_vertical_offset(&mut self, y: i32);
    /// `VisibleHeight`.
    fn visible_height(&self) -> i32;
    /// `Zoom`.
    fn zoom_time(&mut self, scale: f64);
    /// `ZoomRows`.
    fn zoom_rows(&mut self, scale: f64);

    /// `PlaybackStartTime` and `PlaybackEndTime`.
    fn playback_range(&self) -> PlaybackRange;
    fn set_playback_range(&mut self, range: PlaybackRange);
    /// `PlaybackCurrentTime`.
    fn set_playback_current(&mut self, t: Option<TimeSpan>);

    /// `row.TreeId()`, with its setting, for a save: `SaveGridRowSettings`' list. `None` means this grid doesn't know them, so what loaded stays.
    fn row_settings(&self) -> Option<Vec<(String, RowSetting)>> {
        None
    }
    /// `TimelineControl.rowHeight`, for a save. `None` as above.
    fn default_row_height(&self) -> Option<i32> {
        None
    }
    /// One row by id, visible or not.
    fn row_summary(&self, row: u32) -> Option<RowSummary>;
    /// Effects the window just added, removed, moved or brought back with an undo.
    fn effects_added(&mut self, _ids: &[EffectId]) {}
    fn effects_removed(&mut self, _ids: &[EffectId]) {}
    fn effects_moved(&mut self, _moves: &[(EffectId, TimeSpan, TimeSpan)]) {}
    fn effects_reloaded(&mut self) {}
    /// `OkToUseAlignmentHelper`: at most 32 selected per row.
    fn ok_to_use_alignment(&self, _ids: &[EffectId]) -> bool {
        true
    }
    /// The keys whose work is in the timeline: nudges, pointer alignment, split and marks at the play position. A grid that can't do one leaves the
    /// key unhandled.
    fn run_hotkey(&mut self, _command: HotkeyCommand) -> CommandOutcome {
        CommandOutcome::default()
    }

    /// Stands in for a click on a row's label where there's no row list to click.
    fn select_row_for_tests(&mut self, _row: Option<u32>) {}

    /// The toggles in the menus and strips changed: the timeline's options follow.
    fn apply_prefs(&mut self, _prefs: &EditorPrefs) {}
    /// What the timeline holds of the settings the window saves at close: bar heights and the splitter.
    fn collect_prefs(&self, _prefs: &mut EditorPrefs) {}
    /// `SaveGridRowSettings`, when the grid stores its own view state on the document. Runs with no session lock held. False means the window
    /// stores the range and leaves row settings as they loaded.
    fn store_view_state(&mut self) -> bool {
        false
    }
    /// `context_ContextEnded`'s part: the live range goes back and the playback time clears.
    fn playback_ended(&mut self, remembered: PlaybackRange) {
        self.set_playback_range(remembered);
        self.set_playback_current(None);
    }
    /// The Start button, when the grid keeps the remembered range itself. False means the window does it.
    fn goto_start(&mut self) -> bool {
        false
    }
    /// The End button, likewise.
    fn goto_end(&mut self) -> bool {
        false
    }
    /// The remembered range, when the grid keeps it itself.
    fn remembered_range(&self) -> Option<PlaybackRange> {
        None
    }
    /// F122, F123.
    fn set_drag_box_filter(&mut self, _enabled: bool, _types: &[Uuid]) {}
    /// `grid.SelectedEffect`, for Draw Mode.
    fn set_selected_effect(&mut self, _type_id: Option<Uuid>) {}
    /// The document changed under the grid: marks, length or audio.
    fn sync_from_document(&mut self) {}
    /// `AddMarkAtTime(position, false)`: F193. The timeline asks about a missing or hidden default collection.
    fn add_mark_at(&mut self, _time: TimeSpan) {}
    /// `CloseGapsBetweenElements`. The timeline spec has the rules.
    fn close_gaps(&mut self, _threshold_seconds: f64, _confirmed: bool) {}
    /// `DisplaySelectedEffects`: selects them, opens their rows and scrolls to the last.
    fn display_selected(&mut self, elements: &[EffectId]) {
        self.set_selection(elements);
    }
    /// `MoveElementsInRangeByTime`: F92. Returns the offset used, which clipping can have limited.
    fn move_in_range_by_time(&mut self, _start: TimeSpan, _end: TimeSpan, offset: TimeSpan, _visible_only: bool, _clip: bool) -> TimeSpan {
        offset
    }

    /// `grid.IsResizeDragInProgress`.
    fn is_drag_in_progress(&self) -> bool {
        false
    }
    /// `grid.ElementAtPosition(MousePosition)`.
    fn element_under_pointer(&self) -> Option<EffectId> {
        None
    }
    /// `_beginEffectDraw`: drawing an effect, when Esc does nothing.
    fn is_drawing(&self) -> bool {
        false
    }
    /// `IsResizeDragInProgress`: a press has started something that Esc might cancel.
    fn is_dragging(&self) -> bool {
        false
    }
}

struct ShellRow {
    id: u32,
    node: Uuid,
    name: String,
    parent: Option<usize>,
    children: Vec<usize>,
    open: bool,
    deprecated: bool,
    hidden: bool,
    height: i32,
}

/// Default `Row.Height`, in points. A `DefaultRowHeight` of 0 means this. The timeline spec's F18.
const DEFAULT_ROW_HEIGHT: i32 = 32;
/// `TimelineControl`'s starting `TimePerPixel`, 10 ms per point.
const DEFAULT_TIME_PER_PIXEL: TimeSpan = TimeSpan(100_000);

/// A grid with no surface. Rows and selection behave, and the view values hold what they're given.
pub struct ShellGrid {
    rows: Vec<ShellRow>,
    roots: Vec<usize>,
    show_hidden: bool,
    selection: Vec<EffectId>,
    selected_row: Option<u32>,
    active_row: Option<u32>,
    cursor: TimeSpan,
    total: TimeSpan,
    tpp: TimeSpan,
    visible_start: TimeSpan,
    vertical_offset: i32,
    /// Assumed surface size: a build with no timeline has no real one.
    width: i32,
    height: i32,
    range: PlaybackRange,
    current: Option<TimeSpan>,
}

impl ShellGrid {
    pub fn new(tree: &ElementTree) -> ShellGrid {
        let mut grid = ShellGrid {
            rows: Vec::with_capacity(tree.len()),
            roots: Vec::new(),
            show_hidden: false,
            selection: Vec::new(),
            selected_row: None,
            active_row: None,
            cursor: TimeSpan::ZERO,
            total: TimeSpan::ZERO,
            tpp: DEFAULT_TIME_PER_PIXEL,
            visible_start: TimeSpan::ZERO,
            vertical_offset: 0,
            width: 1000,
            height: 600,
            range: PlaybackRange::default(),
            current: None,
        };
        for root in &tree.roots {
            let index = grid.add_row(tree, *root, None);
            grid.roots.push(index);
        }
        grid
    }

    fn add_row(&mut self, tree: &ElementTree, node: NodeId, parent: Option<usize>) -> usize {
        let n = tree.node(node);
        let index = self.rows.len();
        self.rows.push(ShellRow {
            id: index as u32,
            node: n.id,
            name: n.name.clone(),
            parent,
            children: Vec::new(),
            open: false,
            deprecated: n.tags.contains(&DEPRECATED_TAG),
            hidden: n.tags.contains(&HIDDEN_TAG),
            height: DEFAULT_ROW_HEIGHT,
        });
        let children: Vec<usize> = n.children.iter().map(|c| self.add_row(tree, *c, Some(index))).collect();
        self.rows[index].children = children;
        index
    }

    /// Selects a row the way a click on its label does: for tests and the shell-only build.
    pub fn select_row(&mut self, id: Option<u32>) {
        self.selected_row = id;
    }

    pub fn set_active_row(&mut self, id: Option<u32>) {
        self.active_row = id;
    }

    /// Opens a row, for tests.
    pub fn set_open(&mut self, id: u32, open: bool) {
        if let Some(row) = self.rows.get_mut(id as usize) {
            row.open = open;
        }
    }

    fn row_visible(&self, index: usize) -> bool {
        let mut at = &self.rows[index];
        if at.hidden && !self.show_hidden {
            return false;
        }
        while let Some(parent) = at.parent {
            at = &self.rows[parent];
            if !at.open || (at.hidden && !self.show_hidden) {
                return false;
            }
        }
        true
    }

    fn summary(&self, row: &ShellRow) -> RowSummary {
        RowSummary {
            id: row.id,
            node: row.node,
            name: row.name.clone(),
            deprecated: row.deprecated,
            selected: self.selected_row == Some(row.id),
        }
    }
}

impl GridPort for ShellGrid {
    fn visible_rows(&self) -> Vec<RowSummary> {
        (0..self.rows.len()).filter(|i| self.row_visible(*i)).map(|i| self.summary(&self.rows[i])).collect()
    }

    fn row_summary(&self, row: u32) -> Option<RowSummary> {
        self.rows.get(row as usize).map(|r| self.summary(r))
    }

    fn select_row_for_tests(&mut self, row: Option<u32>) {
        self.selected_row = row;
    }

    fn selected_row(&self) -> Option<u32> {
        self.selected_row
    }

    fn active_row(&self) -> Option<u32> {
        self.active_row
    }

    fn top_visible_row(&self) -> Option<u32> {
        (0..self.rows.len()).find(|i| self.row_visible(*i)).map(|i| i as u32)
    }

    fn move_active_row(&mut self, direction: i32) -> bool {
        let visible: Vec<u32> = (0..self.rows.len()).filter(|i| self.row_visible(*i)).map(|i| i as u32).collect();
        if visible.is_empty() {
            return false;
        }
        let next = match self.active_row.and_then(|a| visible.iter().position(|v| *v == a)) {
            Some(at) => (at as i32 + direction).clamp(0, visible.len() as i32 - 1) as usize,
            None => 0,
        };
        self.active_row = Some(visible[next]);
        true
    }

    fn rows_of_node(&self, node: &Uuid) -> Vec<u32> {
        self.rows.iter().filter(|r| &r.node == node).map(|r| r.id).collect()
    }

    fn collapse_all(&mut self) {
        for row in &mut self.rows {
            row.open = false;
        }
    }

    fn reset_row_heights(&mut self) {
        for row in &mut self.rows {
            row.height = DEFAULT_ROW_HEIGHT;
        }
    }

    fn toggle_selected_rows(&mut self, include_children: bool) {
        let Some(id) = self.selected_row else { return };
        let Some(open) = self.rows.get(id as usize).map(|r| !r.open) else { return };
        let mut stack = vec![id as usize];
        while let Some(index) = stack.pop() {
            self.rows[index].open = open;
            if include_children {
                stack.extend(self.rows[index].children.iter().copied());
            }
        }
    }

    fn set_show_hidden(&mut self, show: bool) {
        self.show_hidden = show;
    }

    fn selected_elements(&self) -> Vec<EffectId> {
        self.selection.clone()
    }

    fn set_selection(&mut self, ids: &[EffectId]) {
        self.selection = ids.to_vec();
    }

    fn remove_from_selection(&mut self, ids: &[EffectId]) {
        self.selection.retain(|s| !ids.contains(s));
    }

    fn cursor(&self) -> TimeSpan {
        self.cursor
    }

    fn set_cursor(&mut self, t: TimeSpan) {
        self.cursor = t;
    }

    fn set_total(&mut self, total: TimeSpan) {
        self.total = total;
    }

    fn time_per_pixel(&self) -> TimeSpan {
        self.tpp
    }

    fn set_time_per_pixel(&mut self, tpp: TimeSpan) {
        self.tpp = tpp;
    }

    fn visible_start(&self) -> TimeSpan {
        self.visible_start
    }

    fn set_visible_start(&mut self, t: TimeSpan) {
        let span = self.visible_span();
        let max = (self.total - span).max(TimeSpan::ZERO);
        self.visible_start = t.clamp(TimeSpan::ZERO, max);
    }

    fn visible_span(&self) -> TimeSpan {
        TimeSpan(self.tpp.0 * self.width as i64)
    }

    fn vertical_offset(&self) -> i32 {
        self.vertical_offset
    }

    fn set_vertical_offset(&mut self, y: i32) {
        let content: i32 = (0..self.rows.len()).filter(|i| self.row_visible(*i)).map(|i| self.rows[i].height).sum();
        self.vertical_offset = y.clamp(0, (content - self.height).max(0));
    }

    fn visible_height(&self) -> i32 {
        self.height
    }

    fn zoom_time(&mut self, scale: f64) {
        let next = TimeSpan((self.tpp.0 as f64 * scale) as i64);
        if next.0 > 0 {
            self.tpp = next;
        }
    }

    fn zoom_rows(&mut self, scale: f64) {
        for row in &mut self.rows {
            row.height = ((row.height as f64 * scale) as i32).max(10);
        }
    }

    fn playback_range(&self) -> PlaybackRange {
        self.range
    }

    fn set_playback_range(&mut self, range: PlaybackRange) {
        self.range = range;
    }

    fn set_playback_current(&mut self, t: Option<TimeSpan>) {
        self.current = t;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> ElementTree {
        let mut t = ElementTree::new();
        let group = t.add_node(Uuid::new_v4(), "Group", None);
        let a = t.add_element("A");
        let b = t.add_element("B");
        t.add_child(group, a);
        t.add_child(group, b);
        let c = t.add_element("C");
        t.node_mut(c).tags.push(DEPRECATED_TAG);
        let h = t.add_element("H");
        t.node_mut(h).tags.push(HIDDEN_TAG);
        t
    }

    #[test]
    fn only_open_groups_show_their_children_and_hidden_rows_stay_out() {
        let t = tree();
        let mut g = ShellGrid::new(&t);
        let names = |g: &ShellGrid| g.visible_rows().iter().map(|r| r.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&g), ["Group", "C"]);
        assert!(g.visible_rows()[1].deprecated);
        g.set_open(0, true);
        assert_eq!(names(&g), ["Group", "A", "B", "C"]);
        g.set_show_hidden(true);
        assert_eq!(names(&g), ["Group", "A", "B", "C", "H"]);
        g.collapse_all();
        g.set_show_hidden(false);
        assert_eq!(names(&g), ["Group", "C"]);
    }

    #[test]
    fn visible_start_is_capped_by_the_total() {
        let t = tree();
        let mut g = ShellGrid::new(&t);
        g.set_total(TimeSpan::from_ms_i(60_000));
        g.set_visible_start(TimeSpan::from_ms_i(-5));
        assert_eq!(g.visible_start(), TimeSpan::ZERO);
        g.set_visible_start(TimeSpan::from_ms_i(90_000));
        assert_eq!(g.visible_start(), TimeSpan::from_ms_i(50_000));
    }

    #[test]
    fn active_row_moves_within_the_visible_rows() {
        let t = tree();
        let mut g = ShellGrid::new(&t);
        assert!(g.move_active_row(1));
        assert_eq!(g.active_row(), Some(0));
        assert!(g.move_active_row(1));
        assert_eq!(g.active_row(), Some(3));
        assert!(g.move_active_row(1));
        assert_eq!(g.active_row(), Some(3));
    }
}
