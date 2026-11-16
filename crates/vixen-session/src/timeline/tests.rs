//! Behaviour across the timeline's parts, ported from `Vixen.Tests/Sequencer` where it exists there, and otherwise from the spec's
//! "Tests worth writing". The pure parts have tests in their own modules.

use super::host::TimelineHost;
use super::model::{DragBoxFilter, ElementDescription, HostRequest, Timeline};
use super::rows::{DEPRECATED_TAG, HIDDEN_TAG};
use super::snap::SnapEdge;
use super::*;
use crate::undo::{EffectPlacement, EffectSnapshot, MoveKind, UndoAction};
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::{Color, ElementTree, MarkCollection, TimeSpan};

/// Answers for a session, and keeps a record of what was asked.
#[derive(Default)]
pub(crate) struct RecordingHost {
    pub times: Vec<(ElementId, TimeSpan, TimeSpan)>,
    pub targets: Vec<(ElementId, Uuid)>,
    pub undo: Vec<UndoAction>,
    pub marks: Vec<Vec<MarkCollection>>,
    pub added: Vec<(Uuid, Uuid, TimeSpan, TimeSpan)>,
    pub removed: Vec<ElementId>,
    pub renders: usize,
    pub modified: bool,
    pub quiet_marks: Vec<Vec<MarkCollection>>,
    next: u64,
}

impl TimelineHost for RecordingHost {
    fn commit_times(&mut self, changes: &[(ElementId, TimeSpan, TimeSpan)]) {
        self.times.extend_from_slice(changes);
    }

    fn commit_target(&mut self, id: ElementId, target: Uuid, _name: &str) {
        self.targets.push((id, target));
    }

    fn clone_effect(&mut self, _id: ElementId, target: Uuid, _name: &str, start: TimeSpan, span: TimeSpan) -> Option<ElementId> {
        self.next += 1;
        self.added.push((Uuid::nil(), target, start, span));
        Some(ElementId(10_000 + self.next))
    }

    fn add_effect(&mut self, type_id: Uuid, target: Uuid, _name: &str, start: TimeSpan, span: TimeSpan) -> Option<ElementId> {
        self.next += 1;
        self.added.push((type_id, target, start, span));
        Some(ElementId(10_000 + self.next))
    }

    fn remove_effects(&mut self, ids: &[ElementId]) -> Vec<EffectSnapshot> {
        self.removed.extend_from_slice(ids);
        Vec::new()
    }

    fn record_undo(&mut self, action: UndoAction) {
        self.undo.push(action);
    }

    fn render_changes(&mut self) {
        self.renders += 1;
    }

    fn commit_marks(&mut self, marks: &[MarkCollection]) {
        self.marks.push(marks.to_vec());
    }

    fn describe(&mut self, _id: ElementId) -> Option<ElementDescription> {
        None
    }

    fn mark_modified(&mut self) {
        self.modified = true;
    }

    fn commit_marks_quietly(&mut self, marks: &[MarkCollection]) {
        self.quiet_marks.push(marks.to_vec());
    }
}

const TYPE_A: Uuid = uuid::uuid!("00000000-0000-0000-0000-0000000000aa");
const TYPE_B: Uuid = uuid::uuid!("00000000-0000-0000-0000-0000000000bb");

fn sec(v: f64) -> TimeSpan {
    TimeSpan::from_secs(v)
}

fn e(id: u64) -> ElementId {
    ElementId(id)
}

fn desc(type_id: Uuid, layer: i32) -> ElementDescription {
    ElementDescription { type_id, type_name: Arc::from("Pulse"), layer_level: layer, layer_name: Arc::from("Default") }
}

/// Rows of 20 points. Four root rows; the second and the fourth show one shared element, like the upstream test's two rows of a shared target.
struct Grid {
    t: Timeline,
    host: RecordingHost,
    nodes: Vec<vixen_core::NodeId>,
}

impl Grid {
    /// 100 ms per pixel, 60 seconds, a 1000 by 400 viewport.
    fn new() -> Grid {
        let mut tree = ElementTree::new();
        let r0 = tree.add_element("Row 0");
        let shared = tree.add_element("Shared");
        let r2 = tree.add_element("Row 2");
        // A node with two parents makes two rows, and the groups keep them apart at the top level.
        let group_a = tree.add_node(Uuid::new_v4(), "Group A", None);
        let group_b = tree.add_node(Uuid::new_v4(), "Group B", None);
        tree.add_child(group_a, shared);
        tree.add_child(group_b, shared);
        let mut t = Timeline::new();
        t.info.time_per_pixel = TimeSpan::from_ms_i(100);
        t.info.total = sec(60.0);
        t.rows.load_rows(&tree, &[], 20);
        t.set_viewport(1000, 400, 1000);
        let mut g = Grid { t, host: RecordingHost::default(), nodes: vec![r0, shared, r2, group_a, group_b] };
        g.t.take_outcome();
        g
    }

    /// Four visible rows at 20 points: r0, then r2, then the group rows. Rows are visited by name.
    fn flat() -> Grid {
        let mut tree = ElementTree::new();
        let names = ["A", "B", "C", "D"];
        let mut nodes = Vec::new();
        for n in names {
            nodes.push(tree.add_element(n));
        }
        let mut t = Timeline::new();
        t.info.time_per_pixel = TimeSpan::from_ms_i(100);
        t.info.total = sec(60.0);
        t.rows.load_rows(&tree, &[], 20);
        t.set_viewport(1000, 400, 1000);
        Grid { t, host: RecordingHost::default(), nodes }
    }

    /// `GridCloneDragDuplicateRowsTests`' four rows: the second and fourth share a target.
    fn shared_rows() -> Grid {
        let mut tree = ElementTree::new();
        let a = tree.add_node(Uuid::new_v4(), "G0", None);
        let shared = tree.add_element("Shared");
        let c = tree.add_node(Uuid::new_v4(), "G2", None);
        let d = tree.add_node(Uuid::new_v4(), "G3", None);
        let leaf0 = tree.add_element("L0");
        let leaf2 = tree.add_element("L2");
        tree.add_child(a, leaf0);
        tree.add_child(a, shared);
        tree.add_child(c, leaf2);
        tree.add_child(d, shared);
        let mut t = Timeline::new();
        t.info.time_per_pixel = TimeSpan::from_ms_i(100);
        t.info.total = sec(60.0);
        t.rows.load_rows(&tree, &[], 20);
        t.rows.set_tree_open(RowId(0), true);
        t.rows.set_tree_open(RowId(3), true);
        t.rows.set_tree_open(RowId(5), true);
        t.rows.relayout();
        t.set_viewport(1000, 400, 1000);
        Grid { t, host: RecordingHost::default(), nodes: vec![leaf0, shared, leaf2] }
    }

    fn add(&mut self, id: u64, node: usize, a: f64, b: f64) -> ElementId {
        self.t.add_element(e(id), ElementTimes::new(sec(a), sec(b - a)), Some(self.nodes[node]), desc(TYPE_A, 0));
        self.t.flush_rows();
        if let Some(el) = self.t.elements.get_mut(&e(id)) {
            el.rendered = true;
            el.rendered_duration = el.duration;
        }
        e(id)
    }

    fn times(&self, id: ElementId) -> (TimeSpan, TimeSpan) {
        let el = &self.t.elements[&id];
        (el.start, el.end())
    }

    fn down(&mut self, x: i32, y: i32, mods: Mods) -> InputOutcomeAlias {
        self.t.grid_mouse_down(PointerEvent::new(x, y, Some(Button::Left), mods))
    }

    fn right_down(&mut self, x: i32, y: i32, mods: Mods) -> InputOutcomeAlias {
        self.t.grid_mouse_down(PointerEvent::new(x, y, Some(Button::Right), mods))
    }

    fn drag(&mut self, x: i32, y: i32, mods: Mods) -> InputOutcomeAlias {
        self.t.grid_mouse_move(PointerEvent::new(x, y, Some(Button::Left), mods), &mut self.host)
    }

    fn hover(&mut self, x: i32, y: i32, mods: Mods) -> InputOutcomeAlias {
        self.t.grid_mouse_move(PointerEvent::new(x, y, None, mods), &mut self.host)
    }

    fn up(&mut self, x: i32, y: i32, mods: Mods) -> InputOutcomeAlias {
        self.t.grid_mouse_up(PointerEvent::new(x, y, Some(Button::Left), mods), &mut self.host)
    }

    fn right_up(&mut self, x: i32, y: i32, mods: Mods) -> InputOutcomeAlias {
        self.t.grid_mouse_up(PointerEvent::new(x, y, Some(Button::Right), mods), &mut self.host)
    }

    fn click(&mut self, x: i32, y: i32, mods: Mods) {
        self.hover(x, y, Mods::NONE);
        self.down(x, y, mods);
        self.up(x, y, mods);
    }

    fn selected(&mut self) -> Vec<ElementId> {
        self.t.selected_list()
    }

    fn in_row(&self, r: u32, id: ElementId) -> bool {
        self.t.rows.row(RowId(r)).occurrences.iter().any(|o| o.id == id)
    }
}

type InputOutcomeAlias = super::model::InputOutcome;

const fn tab(rows: u32) -> i32 {
    rows as i32 * 20 + 5
}

// ----------------------------------------------------------------------------------------------------------------------
// Rows: TimelineActiveRowNavigationTests.cs, RowEventScopeTests.cs
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn move_active_row_next_previous_and_ends() {
    let mut g = Grid::flat();
    g.t.set_row_active(RowId(1), true);
    assert!(g.t.move_active_row(1));
    assert_eq!(g.t.active_row(), Some(RowId(2)));
    assert!(g.t.move_active_row(-1));
    assert_eq!(g.t.active_row(), Some(RowId(1)));
    assert!(g.t.move_active_row(-1));
    assert!(!g.t.move_active_row(-1), "no wrap at the top");
    assert_eq!(g.t.active_row(), Some(RowId(0)));
    g.t.clear_active_rows(None);
    g.t.set_row_active(RowId(3), true);
    assert!(!g.t.move_active_row(1), "no wrap at the bottom");
    assert!(!g.t.move_active_row(0));
}

#[test]
fn move_active_row_skips_hidden_rows_and_needs_an_active_one() {
    let mut g = Grid::flat();
    assert!(!g.t.move_active_row(1), "no active row");
    g.t.rows.set_visible(RowId(1), false);
    g.t.rows.relayout();
    g.t.set_row_active(RowId(0), true);
    assert!(g.t.move_active_row(1));
    assert_eq!(g.t.active_row(), Some(RowId(2)));
    // An active row that has gone from view does nothing.
    g.t.rows.set_visible(RowId(2), false);
    g.t.rows.relayout();
    assert!(!g.t.move_active_row(1));
    let mut empty = Timeline::new();
    assert!(!empty.move_active_row(1));
}

#[test]
fn moving_the_active_row_scrolls_just_enough() {
    let mut g = Grid::flat();
    g.t.set_viewport(1000, 30, 1000);
    g.t.set_row_active(RowId(0), true);
    g.t.move_active_row(1);
    assert_eq!(g.t.vertical_offset, 10);
    g.t.move_active_row(1);
    assert_eq!(g.t.vertical_offset, 30);
    g.t.move_active_row(-1);
    assert_eq!(g.t.vertical_offset, 20);
}

#[test]
fn row_clicks_plain_toggle_and_range() {
    let mut g = Grid::flat();
    let (a, b, c) = (g.add(1, 0, 1.0, 2.0), g.add(2, 1, 1.0, 2.0), g.add(3, 2, 1.0, 2.0));
    g.t.click_row_label(RowId(0), Mods::NONE);
    assert_eq!(g.selected(), [a]);
    assert!(g.t.rows.row(RowId(0)).selected);
    // Plain replaces.
    g.t.click_row_label(RowId(1), Mods::NONE);
    assert_eq!(g.selected(), [b]);
    assert!(!g.t.rows.row(RowId(0)).selected);
    // ⌘ toggles.
    g.t.click_row_label(RowId(2), Mods::CMD);
    assert_eq!(g.selected(), [b, c]);
    g.t.click_row_label(RowId(2), Mods::CMD);
    assert_eq!(g.selected(), [b]);
    // ⇧ extends from the first selected row, in both directions and over hidden rows.
    g.t.click_row_label(RowId(0), Mods::NONE);
    g.t.rows.set_visible(RowId(1), false);
    g.t.rows.relayout();
    g.t.click_row_label(RowId(2), Mods::SHIFT);
    assert_eq!(g.selected(), [a, c]);
    assert!(!g.t.rows.row(RowId(1)).selected);
    g.t.rows.set_visible(RowId(1), true);
    g.t.rows.relayout();
    g.t.click_row_label(RowId(2), Mods::NONE);
    g.t.click_row_label(RowId(0), Mods::SHIFT);
    assert_eq!(g.selected(), [a, b, c]);
}

#[test]
fn row_clicks_with_the_drag_box_filter_deselect_other_types() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 0, 3.0, 4.0);
    g.t.elements.get_mut(&b).unwrap().desc.type_id = TYPE_B;
    g.t.select_elements(&[b]);
    let mut filter = DragBoxFilter { enabled: true, ..DragBoxFilter::default() };
    filter.types.insert(TYPE_A);
    g.t.set_drag_box_filter(filter);
    g.t.click_row_label(RowId(0), Mods::CMD);
    assert_eq!(g.selected(), [a]);
}

#[test]
fn clicking_a_row_label_ignores_the_resize_bar_and_the_expander_toggles() {
    let mut g = Grid::new();
    // Group A is the fourth flat row: Row 0, Row 2, then the groups, each with Shared under it.
    let group = g.t.rows.rows.iter().find(|r| r.name == "Group A").unwrap().id;
    let top = g.t.rows.row_top(group).unwrap();
    let press = PointerEvent::new(5, top + 5, Some(Button::Left), Mods::NONE);
    g.t.row_mouse_down(press, 200);
    g.t.row_mouse_up(press, 200);
    assert!(g.t.rows.row(group).tree_open);
    assert!(g.selected().is_empty());
    // The label's bottom 4 points.
    let bar = PointerEvent::new(50, top + 18, Some(Button::Left), Mods::NONE);
    g.t.row_mouse_down(bar, 200);
    g.t.row_mouse_up(bar, 200);
    assert!(!g.t.rows.row(group).selected);
    let label = PointerEvent::new(50, top + 5, Some(Button::Left), Mods::NONE);
    g.t.row_mouse_down(label, 200);
    g.t.row_mouse_up(label, 200);
    assert!(g.t.rows.row(group).selected);
}

#[test]
fn dragging_a_row_label_changes_height_and_syncs_names_on_release() {
    let mut g = Grid::new();
    let first = RowId(0);
    let top = g.t.rows.row_top(first).unwrap();
    g.t.row_mouse_down(PointerEvent::new(50, top + 18, Some(Button::Left), Mods::NONE), 200);
    g.t.row_mouse_move(PointerEvent::new(50, top + 28, Some(Button::Left), Mods::NONE), 200);
    assert_eq!(g.t.rows.row(first).height, 30);
    g.t.row_mouse_up(PointerEvent::new(50, top + 28, Some(Button::Left), Mods::NONE), 200);
    assert_eq!(g.t.rows.row(first).height, 30);
    // Rows named the same take it. Shared has two rows, so resize one of them.
    let shared: Vec<RowId> = g.t.rows.rows.iter().filter(|r| r.name == "Shared").map(|r| r.id).collect();
    g.t.rows.set_tree_open(g.t.rows.row(shared[0]).parent.unwrap(), true);
    g.t.rows.relayout();
    let top = g.t.rows.row_top(shared[0]).unwrap();
    let press = PointerEvent::new(50, top + 18, Some(Button::Left), Mods::NONE);
    g.t.row_mouse_down(press, 200);
    g.t.row_mouse_move(PointerEvent::new(50, top + 38, Some(Button::Left), Mods::NONE), 200);
    assert_eq!(g.t.rows.row(shared[1]).height, 20, "only on release");
    g.t.row_mouse_up(PointerEvent::new(50, top + 38, Some(Button::Left), Mods::NONE), 200);
    assert_eq!(g.t.rows.row(shared[1]).height, 40);
}

#[test]
fn a_height_drag_shows_up_down_over_the_bar_and_an_arrow_off_it_f30() {
    let mut g = Grid::new();
    let first = RowId(0);
    g.t.row_mouse_down(PointerEvent::new(50, 18, Some(Button::Left), Mods::NONE), 200);
    let out = g.t.row_mouse_move(PointerEvent::new(50, 28, Some(Button::Left), Mods::NONE), 200);
    assert_eq!(g.t.rows.row(first).height, 30);
    assert_eq!(out.cursor, CursorKind::ResizeUpDown);
    // Past the 10 point floor the row stops, and the pointer goes on up off its bar.
    let out = g.t.row_mouse_move(PointerEvent::new(50, 2, Some(Button::Left), Mods::NONE), 200);
    assert_eq!(g.t.rows.row(first).height, 10);
    assert_eq!(out.cursor, CursorKind::Arrow);
    g.t.row_mouse_up(PointerEvent::new(50, 2, Some(Button::Left), Mods::NONE), 200);
}

#[test]
fn a_right_press_on_one_label_released_on_another_opens_the_menu_and_clicks_nothing_f33() {
    let mut g = Grid::flat();
    g.t.click_row_label(RowId(0), Mods::NONE);
    let (top1, top2) = (g.t.rows.row_top(RowId(1)).unwrap(), g.t.rows.row_top(RowId(2)).unwrap());
    g.t.row_mouse_down(PointerEvent::new(50, top1 + 5, Some(Button::Right), Mods::NONE), 200);
    let out = g.t.row_mouse_up(PointerEvent::new(50, top2 + 5, Some(Button::Right), Mods::NONE), 200);
    assert_eq!(out.requests, [HostRequest::RowContextMenu { row: RowId(1) }]);
    assert!(g.t.rows.row(RowId(0)).selected && !g.t.rows.row(RowId(1)).selected && !g.t.rows.row(RowId(2)).selected);
    // A left release elsewhere does nothing at all.
    g.t.row_mouse_down(PointerEvent::new(50, top1 + 5, Some(Button::Left), Mods::NONE), 200);
    let out = g.t.row_mouse_up(PointerEvent::new(50, top2 + 5, Some(Button::Left), Mods::NONE), 200);
    assert!(out.requests.is_empty());
    assert!(g.t.rows.row(RowId(0)).selected);
}

#[test]
fn right_clicks_on_a_row_keep_upstreams_double_click_quirk() {
    let mut g = Grid::flat();
    g.t.click_row_label(RowId(0), Mods::NONE);
    g.t.click_row_label(RowId(1), Mods::CMD);
    // ⌘ toggles twice, so the selection stays.
    let top = g.t.rows.row_top(RowId(1)).unwrap();
    let ev = PointerEvent::new(50, top + 5, Some(Button::Right), Mods::CMD);
    g.t.row_mouse_down(ev, 200);
    let out = g.t.row_mouse_up(ev, 200);
    assert!(g.t.rows.row(RowId(0)).selected && g.t.rows.row(RowId(1)).selected);
    assert_eq!(out.requests, [HostRequest::RowContextMenu { row: RowId(1) }]);
    // Plain collapses to the clicked row.
    let ev = PointerEvent::new(50, top + 5, Some(Button::Right), Mods::NONE);
    g.t.row_mouse_down(ev, 200);
    g.t.row_mouse_up(ev, 200);
    assert!(!g.t.rows.row(RowId(0)).selected && g.t.rows.row(RowId(1)).selected);
}

#[test]
fn toggle_selected_rows_uses_the_active_row_with_no_selection() {
    let mut g = Grid::new();
    let group = g.t.rows.rows.iter().find(|r| r.name == "Group A").unwrap().id;
    g.t.set_row_active(group, true);
    let visible = g.t.rows.visible().len();
    g.t.toggle_selected_rows(false);
    assert_eq!(g.t.rows.visible().len(), visible + 1);
    g.t.toggle_selected_rows(true);
    assert_eq!(g.t.rows.visible().len(), visible);
}

#[test]
fn highlight_data_tracks_effects_and_descendants() {
    let mut g = Grid::new();
    g.add(1, 1, 1.0, 2.0);
    let display = g.t.rows.visible_rows(0, 1000);
    let group = display.iter().find(|r| r.name == "Group A").unwrap();
    assert!(group.subtree_has_effects);
    assert!(!group.has_effects);
    assert!(!display.iter().find(|r| r.name == "Row 0").unwrap().subtree_has_effects);
    g.t.remove_element(e(1));
    assert!(!g.t.rows.visible_rows(0, 1000).iter().find(|r| r.name == "Group A").unwrap().subtree_has_effects);
}

#[test]
fn tag_menu_state_and_toggle_with_cascade() {
    let mut g = Grid::new();
    let tag = Uuid::new_v4();
    g.t.tags.push(super::row_input::TagDisplay { id: tag, name: "T".into(), sort_order: 1, color: None });
    let a = g.t.rows.rows.iter().find(|r| r.name == "Group A").unwrap().id;
    let b = g.t.rows.rows.iter().find(|r| r.name == "Group B").unwrap().id;
    g.t.click_row_label(a, Mods::NONE);
    assert_eq!(g.t.tag_menu_state()[0].state, super::row_input::TagCheck::Unchecked);
    assert!(g.t.row_menu_model().tags_enabled);
    let changes = g.t.toggle_tag_on_selected(tag, true);
    // The group and its child.
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().all(|c| c.added));
    assert_eq!(g.t.tag_menu_state()[0].state, super::row_input::TagCheck::Checked);
    // The shared child's other row has it too.
    assert!(g.t.rows.row(g.t.rows.row(b).children[0]).tags.contains(&tag));
    g.t.click_row_label(b, Mods::CMD);
    assert_eq!(g.t.tag_menu_state()[0].state, super::row_input::TagCheck::Mixed);
    g.t.click_row_label(b, Mods::CMD);
    let changes = g.t.toggle_tag_on_selected(tag, false);
    assert_eq!(changes.len(), 1);
    assert!(!changes[0].added);
}

#[test]
fn a_hidden_tag_toggled_on_hides_the_row_at_once() {
    let mut g = Grid::new();
    let row = g.t.rows.rows.iter().find(|r| r.name == "Row 2").unwrap().id;
    g.t.click_row_label(row, Mods::NONE);
    g.t.toggle_tag_on_selected(HIDDEN_TAG, false);
    assert!(!g.t.rows.row(row).visible);
    g.t.set_show_hidden_rows(true);
    assert!(g.t.rows.row(row).visible);
}

#[test]
fn tags_the_tag_manager_saved_reach_the_labels_and_the_hidden_filter_f38() {
    use super::row_input::TagDisplay;
    let mut g = Grid::new();
    g.t.take_outcome();
    let shared_rows = g.t.rows.rows_of_node(g.nodes[1]).to_vec();
    let shared = g.t.rows.row(shared_rows[0]).node_uuid;
    let row2 = g.t.rows.rows.iter().find(|r| r.name == "Row 2").unwrap().id;
    let row2_node = g.t.rows.row(row2).node_uuid;
    assert!(g.t.rows.row(row2).visible);
    // The manager made a tag with a colour, put it on the shared node, and put Hidden on Row 2.
    let fresh = Uuid::new_v4();
    let defs = vec![TagDisplay { id: fresh, name: "Fresh".into(), sort_order: 0, color: Some(Color::RED) }];
    let saved = |node: Uuid| {
        Some(if node == shared {
            vec![fresh]
        } else if node == row2_node {
            vec![HIDDEN_TAG]
        } else {
            Vec::new()
        })
    };
    g.t.reload_tags(defs.clone(), &saved);
    assert_eq!(g.t.tags, defs);
    for r in &shared_rows {
        assert_eq!(g.t.rows.row(*r).tags, [fresh]);
    }
    assert!(!g.t.rows.row(row2).visible);
    assert!(g.t.take_outcome().dirty.row_header);
}

// ----------------------------------------------------------------------------------------------------------------------
// Stacking and hit testing
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn hit_testing_uses_each_rows_own_values_q3() {
    let mut g = Grid::shared_rows();
    // In the row the effect shares with another, make one tall.
    let a = g.add(1, 1, 1.0, 3.0);
    let b = g.add(2, 1, 2.0, 4.0);
    let rows = g.t.elements[&a].rows.clone();
    assert_eq!(rows.len(), 2);
    g.t.rows.set_row_height(rows[1], 40);
    g.t.refresh_view_stacks();
    let top = g.t.rows.row_top(rows[0]).unwrap();
    // Height 20: lanes of 9.
    assert_eq!(g.t.elements_at((25, top + 1)), [a]);
    assert_eq!(g.t.elements_at((25, top + 10)), [b]);
    // Height 40: lanes of 19. The same y offset is still the first lane there.
    let top = g.t.rows.row_top(rows[1]).unwrap();
    assert_eq!(g.t.elements_at((25, top + 10)), [a]);
    assert_eq!(g.t.elements_at((25, top + 20)), [b]);
}

#[test]
fn elements_at_x_is_inclusive_both_ends_and_y_half_open() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    g.t.refresh_view_stacks();
    assert_eq!(g.t.elements_at((10, 5)), [a]);
    assert_eq!(g.t.elements_at((20, 5)), [a]);
    assert!(g.t.elements_at((9, 5)).is_empty());
    assert!(g.t.elements_at((21, 5)).is_empty());
    assert_eq!(g.t.elements_at((15, 0)), [a]);
    // Height 20, one lane: 19 tall, so the last point of the row isn't on it.
    assert_eq!(g.t.elements_at((15, 18)), [a]);
    assert!(g.t.elements_at((15, 19)).is_empty());
    assert_eq!(g.t.element_at((15, 5)), Some(a));
}

#[test]
fn zones_at_each_end() {
    let tpp = TimeSpan::from_ms_i(100);
    let times = ElementTimes::new(sec(10.0), sec(10.0));
    use super::grid_input::resize_zone;
    assert_eq!(resize_zone(times, tpp, 100), ResizeZone::Front);
    assert_eq!(resize_zone(times, tpp, 111), ResizeZone::Front);
    assert_eq!(resize_zone(times, tpp, 112), ResizeZone::None);
    assert_eq!(resize_zone(times, tpp, 188), ResizeZone::None);
    assert_eq!(resize_zone(times, tpp, 189), ResizeZone::Back);
    assert_eq!(resize_zone(times, tpp, 200), ResizeZone::Back);
    let short = ElementTimes::new(sec(10.0), sec(1.0));
    assert_eq!(resize_zone(short, tpp, 104), ResizeZone::Front);
    assert_eq!(resize_zone(short, tpp, 105), ResizeZone::None);
    assert_eq!(resize_zone(short, tpp, 106), ResizeZone::Back);
}

#[test]
fn frames_cover_only_the_rectangle_and_drop_sub_pixel_effects() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    g.add(2, 1, 50.0, 51.0);
    let tiny = e(3);
    g.t.add_element(tiny, ElementTimes::new(sec(1.0), TimeSpan(10)), Some(g.nodes[1]), desc(TYPE_A, 0));
    let frame = g.t.grid_frame(IRect::new(0, 0, 100, 40));
    assert_eq!(frame.rows.len(), 2);
    assert_eq!(frame.elements.iter().map(|q| q.id).collect::<Vec<_>>(), [a]);
    assert_eq!(frame.elements[0].x, 10);
    assert_eq!(frame.elements[0].w, 10);
    assert_eq!(frame.elements[0].h, 19);
    // One-second lines every 10 points, none at zero.
    assert_eq!(frame.gridlines[0], 10.0);
    assert_eq!(frame.canvas_width, 600);
    assert_eq!(frame.canvas_height, 80);
}

#[test]
fn gridlines_stop_when_they_would_be_a_wash() {
    let mut g = Grid::flat();
    g.t.set_time_per_pixel(TimeSpan::from_ms_i(600));
    assert!(g.t.grid_frame(IRect::new(0, 0, 100, 40)).gridlines.is_empty());
}

#[test]
fn a_change_of_view_that_changes_lanes_repaints_only_that_row_q4() {
    let mut g = Grid::flat();
    g.t.set_viewport(100, 400, 100);
    g.add(1, 0, 0.0, 10.0);
    let d = g.add(4, 0, 9.0, 20.0);
    g.add(2, 0, 5.0, 6.0);
    g.add(3, 0, 5.5, 8.0);
    g.t.refresh_view_stacks();
    g.t.take_outcome();
    let slot = |g: &Grid| g.t.rows.row(RowId(0)).occurrences.iter().find(|o| o.id == d).unwrap().slot();
    assert_eq!(slot(&g), StackSlot { index: 1, count: 3 });
    g.t.set_visible_start(sec(15.0));
    let out = g.t.take_outcome();
    assert_eq!(slot(&g), StackSlot { index: 1, count: 2 });
    assert!(!out.dirty.grid_rects.is_empty());
    assert!(!out.dirty.grid_all);
    assert_eq!(out.dirty.grid_rects[0].y, 0);
    assert_eq!(out.dirty.grid_rects[0].h, 20);
}

// ----------------------------------------------------------------------------------------------------------------------
// Selection and the cursor: TimelineCursorSelectionTests.cs, F46 to F57
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn a_click_selects_and_moves_the_cursor_to_its_start() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    g.down(15, 5, Mods::NONE);
    assert_eq!(g.selected(), [a]);
    assert_eq!(g.t.cursor, sec(1.0));
    assert_eq!(g.t.active_row(), Some(RowId(0)));
}

#[test]
fn legacy_mode_leaves_the_cursor_alone() {
    let mut g = Grid::flat();
    g.add(1, 0, 1.0, 2.0);
    g.t.opts.legacy_cursor = true;
    g.down(15, 5, Mods::NONE);
    g.up(15, 5, Mods::NONE);
    assert_eq!(g.t.cursor, TimeSpan::ZERO);
}

#[test]
fn command_click_moves_the_cursor_to_the_one_added_and_deselect_does_not_at_press() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 3.0, 4.0);
    let b = g.add(2, 1, 1.0, 2.0);
    g.click(35, 5, Mods::NONE);
    g.down(15, tab(1), Mods::CMD);
    assert_eq!(g.selected(), [a, b]);
    assert_eq!(g.t.cursor, sec(1.0));
    g.up(15, tab(1), Mods::CMD);
    // Deselecting at press doesn't move it, but release moves to the last pressed.
    g.t.set_cursor(sec(30.0));
    g.down(35, 5, Mods::CMD);
    assert_eq!(g.selected(), [b]);
    assert_eq!(g.t.cursor, sec(30.0));
    g.up(35, 5, Mods::CMD);
    assert_eq!(g.t.cursor, sec(3.0));
}

#[test]
fn background_click_clears_and_moves_the_cursor_to_the_pointer() {
    let mut g = Grid::flat();
    g.add(1, 0, 1.0, 2.0);
    g.click(15, 5, Mods::NONE);
    g.down(100, tab(1), Mods::NONE);
    assert!(g.selected().is_empty());
    assert_eq!(g.t.drag_state(), DragState::Selecting);
    g.up(100, tab(1), Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::Normal);
    assert_eq!(g.t.cursor, sec(10.0));
    assert_eq!(g.t.active_row(), Some(RowId(1)));
}

#[test]
fn command_press_on_the_background_does_nothing_at_all() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    g.click(15, 5, Mods::NONE);
    let out = g.down(100, tab(1), Mods::CMD);
    assert_eq!(g.t.drag_state(), DragState::Normal);
    assert!(!out.selection_changed);
    assert_eq!(g.selected(), [a]);
}

#[test]
fn clicking_inside_a_multi_selection_keeps_it_until_release_and_collapses() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 1, 1.0, 2.0);
    g.t.select_elements(&[a, b]);
    g.hover(15, 5, Mods::NONE);
    g.down(15, 5, Mods::NONE);
    assert_eq!(g.selected(), [a, b]);
    g.up(15, 5, Mods::NONE);
    assert_eq!(g.selected(), [a]);
    assert_eq!(g.t.g.last_single_selected, (15, 5));
}

#[test]
fn shift_click_selects_the_rectangle_from_the_anchor_replacing_the_rest() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 1, 3.0, 4.0);
    let c = g.add(3, 2, 5.0, 6.0);
    let far = g.add(4, 3, 30.0, 31.0);
    g.t.select_elements(&[far]);
    g.click(15, 5, Mods::NONE);
    assert_eq!(g.selected(), [a]);
    g.down(55, tab(2), Mods::SHIFT);
    assert_eq!(g.selected(), [a, b, c]);
    assert_eq!(g.t.active_row(), None);
    g.up(55, tab(2), Mods::SHIFT);
    assert_eq!(g.t.cursor, sec(5.0));
}

#[test]
fn right_click_on_blank_clears_and_on_a_member_keeps() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 1, 1.0, 2.0);
    g.t.select_elements(&[a, b]);
    // On a member.
    g.right_down(15, 5, Mods::NONE);
    assert_eq!(g.selected(), [a, b]);
    let out = g.right_up(15, 5, Mods::NONE);
    assert_eq!(out.requests, [HostRequest::ContextMenu { elements: vec![a], time: sec(1.5), row: RowId(0) }]);
    assert_eq!(g.t.active_row(), Some(RowId(0)));
    // The cursor isn't moved by a right-click on an effect unless in legacy mode.
    assert_eq!(g.t.cursor, sec(1.0));
    g.t.opts.legacy_cursor = true;
    g.right_down(15, 5, Mods::NONE);
    g.right_up(15, 5, Mods::NONE);
    assert_eq!(g.t.cursor, sec(1.5));
    g.t.opts.legacy_cursor = false;
    // On blank.
    g.right_down(100, tab(2), Mods::NONE);
    assert!(g.selected().is_empty());
    g.right_up(100, tab(2), Mods::NONE);
    assert_eq!(g.t.cursor, sec(10.0));
    // Below the rows there's no row, so no menu.
    g.right_down(100, 300, Mods::NONE);
    assert!(g.right_up(100, 300, Mods::NONE).requests.is_empty());
}

#[test]
fn right_press_on_an_unselected_effect_replaces_the_selection() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 1, 1.0, 2.0);
    g.t.select_elements(&[a]);
    g.right_down(15, tab(1), Mods::NONE);
    assert_eq!(g.selected(), [b]);
}

#[test]
fn overlapping_effects_go_to_the_host_and_its_choice_selects() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 3.0);
    let b = g.add(2, 0, 1.0, 3.0);
    // Both on the same row and one lane each, so y decides: make them share a point by giving the row room.
    g.t.refresh_view_stacks();
    // A pointer in the top lane hits one, so stack them with a filter of none.
    g.t.rows.row_mut(RowId(0)).occurrences.iter_mut().for_each(|o| {
        o.index = 0;
        o.count = 1;
    });
    g.t.rows.row_mut(RowId(0)).walk_key = Some((0, 2));
    let press = PointerEvent::new(20, 5, Some(Button::Left), Mods::NONE);
    let out = g.t.grid_mouse_down(press);
    assert_eq!(out.requests, [HostRequest::ChooseOverlapping { elements: vec![a, b], at: (20, 5) }]);
    assert!(g.selected().is_empty());
    assert!(out.selection_changed);
    // FIX Q47: no drag behind the menu.
    assert_eq!(g.t.drag_state(), DragState::Normal);
    g.t.finish_chooser(Some(b));
    assert_eq!(g.selected(), [b]);
    assert_eq!(g.t.overlap_menu_label(b).unwrap(), "Pulse (0:01.000)");
}

#[test]
fn select_all_then_the_list_is_complete_q6() {
    let mut g = Grid::flat();
    let (a, b) = (g.add(1, 0, 1.0, 2.0), g.add(2, 1, 3.0, 4.0));
    g.t.select_all();
    assert_eq!(g.selected(), [a, b]);
    assert!(g.t.take_outcome().selection_changed);
}

#[test]
fn selected_list_is_rows_top_down_then_selection_order() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 0, 3.0, 4.0);
    let c = g.add(3, 1, 1.0, 2.0);
    g.t.select_elements(&[c, b, a]);
    assert_eq!(g.selected(), [b, a, c]);
}

#[test]
fn select_element_and_elements_move_the_cursor_as_upstream() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 5.0, 6.0);
    let b = g.add(2, 1, 1.0, 2.0);
    g.t.select_element(a);
    assert_eq!(g.t.cursor, sec(5.0));
    g.t.select_elements(&[]);
    assert_eq!(g.t.cursor, sec(5.0), "an empty list moves nothing");
    g.t.select_elements(&[b]);
    assert_eq!(g.t.cursor, sec(1.0));
    g.t.deselect_element(b);
    assert_eq!(g.t.cursor, sec(1.0));
    g.t.opts.legacy_cursor = true;
    g.t.set_cursor(sec(9.0));
    g.t.select_element(a);
    assert_eq!(g.t.cursor, sec(9.0));
}

// ----------------------------------------------------------------------------------------------------------------------
// Rubber band
// ----------------------------------------------------------------------------------------------------------------------

fn band(g: &mut Grid, from: (i32, i32), to: (i32, i32), mods: Mods) {
    g.down(from.0, from.1, mods);
    g.drag(to.0, to.1, mods);
}

#[test]
fn crossing_and_enclosed() {
    let mut g = Grid::flat();
    // Lanes in row 1 run from 20 to 38, so a band from row 0 to row 2 is clear of them top and bottom.
    let a = g.add(1, 1, 10.0, 20.0);
    band(&mut g, (150, 5), (250, 45), Mods::NONE);
    assert_eq!(g.selected(), [a], "crossing, over time 15 to 25");
    assert!(!g.t.selection_green);
    g.up(250, 45, Mods::NONE);
    g.t.opts.cad_style = true;
    band(&mut g, (150, 5), (250, 45), Mods::NONE);
    assert!(g.t.selection_green, "rightward is enclosed");
    assert!(g.selected().is_empty(), "enclosed needs all of it");
    g.up(250, 45, Mods::NONE);
    band(&mut g, (50, 5), (250, 45), Mods::NONE);
    assert!(g.t.selection_green);
    assert_eq!(g.selected(), [a], "over time 5 to 25 it's all in");
    g.up(250, 45, Mods::NONE);
    // Leftward crosses, and blue.
    g.t.clear_selection();
    band(&mut g, (250, 5), (150, 45), Mods::NONE);
    assert!(!g.t.selection_green);
    assert_eq!(g.selected(), [a]);
    g.up(150, 45, Mods::NONE);
    // A band that only clips the lane's edge encloses nothing, however wide.
    g.t.clear_selection();
    band(&mut g, (50, 25), (250, 45), Mods::NONE);
    assert!(g.selected().is_empty());
}

#[test]
fn a_tiny_band_is_a_background_click() {
    let mut g = Grid::flat();
    g.t.set_cursor(sec(1.0));
    g.down(100, 5, Mods::NONE);
    g.drag(101, 6, Mods::NONE);
    g.up(101, 6, Mods::NONE);
    assert_eq!(g.t.cursor, sec(10.1));
    assert_eq!(g.t.active_row(), Some(RowId(0)));
    assert!(g.t.g.selection_area.is_empty());
}

#[test]
fn a_real_band_activates_the_press_row_and_moves_the_cursor_to_the_earliest() {
    let mut g = Grid::flat();
    let (b, a) = (g.add(1, 1, 3.0, 4.0), g.add(2, 2, 1.0, 2.0));
    band(&mut g, (5, tab(1)), (100, tab(2)), Mods::NONE);
    assert_eq!(g.selected(), [b, a]);
    g.up(100, tab(2), Mods::NONE);
    assert_eq!(g.t.active_row(), Some(RowId(1)));
    assert_eq!(g.t.cursor, sec(1.0));
}

#[test]
fn shift_keeps_the_remembered_list() {
    let mut g = Grid::flat();
    let (a, b) = (g.add(1, 0, 1.0, 2.0), g.add(2, 1, 1.0, 2.0));
    g.click(15, 5, Mods::NONE);
    assert_eq!(g.selected(), [a]);
    band(&mut g, (100, tab(1)), (5, tab(1) + 2), Mods::SHIFT);
    assert_eq!(g.selected(), [a, b]);
    // Shrinking the band away from b drops b but keeps a.
    g.drag(95, tab(1) + 2, Mods::SHIFT);
    assert_eq!(g.selected(), [a]);
    g.up(95, tab(1) + 2, Mods::SHIFT);
}

#[test]
fn the_filter_limits_a_band() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 0, 3.0, 4.0);
    g.t.elements.get_mut(&b).unwrap().desc.type_id = TYPE_B;
    let mut filter = DragBoxFilter { enabled: true, ..DragBoxFilter::default() };
    filter.types.insert(TYPE_A);
    g.t.set_drag_box_filter(filter);
    band(&mut g, (5, 1), (100, 15), Mods::NONE);
    assert_eq!(g.selected(), [a]);
}

#[test]
fn an_effect_in_two_rows_takes_the_last_rows_answer() {
    let mut g = Grid::shared_rows();
    let a = g.add(1, 1, 1.0, 2.0);
    let rows = g.t.elements[&a].rows.clone();
    let (t0, t1) = (g.t.rows.row_top(rows[0]).unwrap(), g.t.rows.row_top(rows[1]).unwrap());
    // The band's bottom lands exactly on the second row's top: that row is walked, and it says no.
    g.t.select_elements_within(IRect::new(5, t0 + 1, 20, t1 - t0 - 1), false, Mods::NONE);
    assert_eq!(g.selected(), Vec::<ElementId>::new());
    // A point further and the second row says yes.
    g.t.select_elements_within(IRect::new(5, t0 + 1, 20, t1 - t0), false, Mods::NONE);
    assert_eq!(g.selected(), [a]);
}

// ----------------------------------------------------------------------------------------------------------------------
// Moving
// ----------------------------------------------------------------------------------------------------------------------

/// Press, then cross the threshold at the same place the move begins measuring from.
fn begin_move(g: &mut Grid, x: i32, y: i32, mods: Mods) {
    g.hover(x, y, Mods::NONE);
    g.down(x, y, mods);
    g.drag(x + 8, y, mods);
    assert_eq!(g.t.drag_state(), DragState::Moving);
}

#[test]
fn the_threshold_is_half_open() {
    let mut g = Grid::flat();
    g.add(1, 0, 10.0, 20.0);
    g.hover(150, 8, Mods::NONE);
    g.down(150, 8, Mods::NONE);
    g.drag(157, 8, Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::Waiting);
    g.drag(142, 8, Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::Waiting);
    g.drag(158, 8, Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::Moving);
    g.up(158, 8, Mods::NONE);
    g.down(150, 8, Mods::NONE);
    g.drag(141, 8, Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::Moving);
}

#[test]
fn moves_measure_from_where_the_drag_began_and_commit_on_release() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    begin_move(&mut g, 150, 8, Mods::NONE);
    // Crossing the threshold doesn't count, so it hasn't jumped.
    assert_eq!(g.times(a), (sec(10.0), sec(20.0)));
    assert_eq!(g.t.cursor_kind, CursorKind::Move);
    g.drag(168, 8, Mods::NONE);
    assert_eq!(g.times(a), (sec(11.0), sec(21.0)));
    // Nothing reaches the document until release.
    assert!(g.host.times.is_empty());
    let out = g.up(168, 8, Mods::NONE);
    assert_eq!(g.host.times, [(a, sec(11.0), sec(10.0))]);
    assert_eq!(g.host.renders, 1);
    assert_eq!(g.host.undo.len(), 1);
    assert_eq!(g.host.undo[0].description(), "Moved 1 effect");
    assert!(out.requests.contains(&HostRequest::UndoRecorded));
    assert_eq!(g.t.g.last_single_selected, (168, 8));
    assert_eq!(g.t.cursor_kind, CursorKind::Arrow);
}

#[test]
fn the_undo_entry_holds_where_it_was_and_is_recorded_even_with_no_net_change() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    begin_move(&mut g, 150, 8, Mods::NONE);
    g.drag(158, 8, Mods::NONE);
    g.up(158, 8, Mods::NONE);
    match &g.host.undo[0] {
        UndoAction::PlacementChanged { kind, places } => {
            assert_eq!(*kind, MoveKind::Moved);
            let EffectPlacement { id, start, span, .. } = places[0].clone();
            assert_eq!((id, start, span), (a, sec(10.0), sec(10.0)));
        }
        _ => panic!("a placement"),
    }
}

#[test]
fn clamps_to_the_sequence() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 2.0, 3.0);
    let b = g.add(2, 1, 57.0, 58.0);
    g.t.select_elements(&[a, b]);
    g.hover(25, 5, Mods::NONE);
    g.down(25, 5, Mods::NONE);
    g.drag(33, 5, Mods::NONE);
    g.drag(33 - 50, 5, Mods::NONE);
    assert_eq!(g.times(a).0, TimeSpan::ZERO);
    assert_eq!(g.times(b).0, sec(55.0));
    g.drag(33 + 50, 5, Mods::NONE);
    assert_eq!(g.times(b).1, sec(60.0));
    assert_eq!(g.times(a).0, sec(4.0));
}

#[test]
fn a_snap_after_the_clamp_can_pass_the_bounds_and_start_clamps_at_zero_q7() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 5.0, 6.0);
    // A neighbour edge behind zero isn't possible, so use a mark at a negative time.
    let mut c = MarkCollection { show_grid_lines: true, level: 6, ..MarkCollection::default() };
    c.marks.push(vixen_core::Mark::new(sec(-0.5)));
    g.t.load_marks(vec![c]);
    begin_move(&mut g, 55, 8, Mods::NONE);
    g.drag(55 + 8 - 50, 8, Mods::NONE);
    // Clamped to -5 s, then the snap pulls the start to -0.5, which the setter holds at 0.
    assert_eq!(g.times(a).0, TimeSpan::ZERO);
}

#[test]
fn freeze_rules_match_exactly() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    begin_move(&mut g, 150, 8, Mods::NONE);
    g.drag(160, 8, Mods::NONE);
    let at = g.times(a).0;
    // ⇧, ⌘⇧, ⌥⌘ hold x at the previous event's.
    for mods in [Mods::SHIFT, Mods { shift: true, cmd: true, opt: false }, Mods { shift: false, cmd: true, opt: true }] {
        g.drag(300, 8, mods);
        assert_eq!(g.times(a).0, at, "{mods:?}");
    }
    // ⇧⌘⌥ freezes neither.
    g.drag(170, 8, Mods { shift: true, cmd: true, opt: true });
    assert_eq!(g.times(a).0, sec(11.2));
    // ⇧⌥ holds y.
    g.drag(170, tab(2), Mods { shift: true, cmd: false, opt: true });
    assert!(g.in_row(0, a));
}

#[test]
fn freeze_applies_in_every_state_q8() {
    let mut g = Grid::flat();
    g.add(1, 0, 10.0, 20.0);
    g.click(150, 8, Mods::NONE);
    // ⇧ while hovering: x stays at the last value, so the zone can't change.
    g.hover(102, 8, Mods::NONE);
    assert_eq!(g.t.g.resize_zone, ResizeZone::Front);
    g.hover(150, 8, Mods::SHIFT);
    assert_eq!(g.t.g.resize_zone, ResizeZone::Front);
    assert_eq!(g.t.g.last_grid_location.0, 102);
}

#[test]
fn escape_restores_times_rows_and_view_and_leaves_no_undo_q10() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    g.t.set_viewport(100, 400, 100);
    g.t.set_visible_start(sec(5.0));
    let (x, y) = (150 - 50, 8);
    begin_move(&mut g, x, y, Mods::NONE);
    g.drag(x + 50, tab(2), Mods::NONE);
    assert!(g.in_row(2, a));
    g.t.set_visible_start(sec(8.0));
    g.t.grid_cancel_drag();
    assert_eq!(g.times(a), (sec(10.0), sec(20.0)));
    assert!(g.in_row(0, a));
    assert!(!g.in_row(2, a));
    assert_eq!(g.t.elements[&a].target, Some(g.nodes[0]));
    assert_eq!(g.t.info.visible_start, sec(5.0));
    assert_eq!(g.t.drag_state(), DragState::Normal);
    assert!(g.t.g.drag_snap.is_none());
    assert!(g.t.g.move_info.is_none());
    assert!(g.host.undo.is_empty());
    assert!(g.host.times.is_empty());
    // It isn't lifted any more.
    assert!(!g.t.rows.row(RowId(0)).occurrences[0].moving);
    // Esc again does nothing.
    g.t.grid_cancel_drag();
    assert_eq!(g.times(a).0, sec(10.0));
}

#[test]
fn clone_drag_leaves_originals_selects_clones_and_makes_two_undo_entries() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    g.hover(150, 8, Mods::NONE);
    g.down(150, 8, Mods::CMD);
    g.drag(158, 8, Mods::CMD);
    assert_eq!(g.t.drag_state(), DragState::Moving);
    let selected = g.selected();
    assert_eq!(selected.len(), 1);
    let clone = selected[0];
    assert_ne!(clone, a);
    g.drag(168, 8, Mods::CMD);
    assert_eq!(g.times(clone).0, sec(11.0));
    assert_eq!(g.times(a).0, sec(10.0));
    g.up(168, 8, Mods::CMD);
    let labels: Vec<String> = g.host.undo.iter().map(UndoAction::description).collect();
    assert_eq!(labels, ["Added 1 effect", "Moved 1 effect"]);
}

#[test]
fn a_clone_drag_selects_what_is_under_the_press_if_it_was_not() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 20.0);
    let b = g.add(2, 1, 10.0, 20.0);
    g.click(150, 8, Mods::NONE);
    g.down(150, tab(1), Mods::CMD);
    // ⌘ press on an unselected effect adds it, then the drag clones both.
    assert_eq!(g.selected(), [a, b]);
    g.drag(158, tab(1), Mods::CMD);
    assert_eq!(g.selected().len(), 2);
    assert!(g.selected().iter().all(|id| *id != a && *id != b));
}

#[test]
fn hover_tracks_what_is_under_the_pointer_and_describes_it() {
    let mut g = Grid::flat();
    g.add(1, 0, 61.0 - 60.0, 2.5);
    let out = g.hover(15, 5, Mods::NONE);
    assert!(out.dirty.overlay);
    let info = g.t.hover_info().unwrap();
    assert_eq!(info.text, "Pulse \r\n Layer: Default \r\n Start: 0:01.000 \r\n End: 0:02.500 \r\n Length: 0:01.500");
    assert_eq!(info.rect, IRect::new(10, 0, 15, 20));
    // Moving within the same effect changes nothing.
    assert!(!g.hover(16, 5, Mods::NONE).dirty.overlay);
    g.t.opts.show_effect_info = false;
    assert!(g.t.hover_info().is_none());
}

#[test]
fn hover_box_goes_above_unless_it_would_leave_the_clip() {
    use super::model::hover_box_origin;
    // Text 20 tall: box 22. Above: y - (20 + 2 - 4).
    assert_eq!(hover_box_origin(IRect::new(50, 100, 30, 20), 20, 0, 0), (50, 82));
    // Too close to the top: below, y + 20 + 2 - 8.
    assert_eq!(hover_box_origin(IRect::new(50, 10, 30, 20), 20, 0, 0), (50, 24));
    // Left of the clip.
    assert_eq!(hover_box_origin(IRect::new(50, 100, 30, 20), 20, 70, 0), (75, 82));
}

// ----------------------------------------------------------------------------------------------------------------------
// Between rows: GridCloneDragDuplicateRowsTests.cs
// ----------------------------------------------------------------------------------------------------------------------

/// The rows of `shared_rows` in view, top down, whatever else lies between them: the upstream test's rows[0..4].
fn shared_view(g: &Grid) -> [RowId; 4] {
    let vis = |name: &str, nth: usize| g.t.rows.rows.iter().filter(|r| r.name == name).nth(nth).unwrap().id;
    [vis("L0", 0), vis("Shared", 0), vis("L2", 0), vis("Shared", 1)]
}

fn set_context(g: &mut Grid, press: Vec<ElementId>, row: RowId) {
    g.t.g.mouse_down_elements = press;
    g.t.g.mouse_down_row = Some(row);
    g.t.g.current_row_index = row.0 as i32;
}

fn move_to(g: &mut Grid, selected: &[ElementId], row: RowId) {
    let y = g.t.rows.row_top(row).unwrap() + 1;
    g.t.move_elements_vertically(selected, (0, y));
}

#[test]
fn a_clone_crossing_duplicate_rows_tracks_the_pointer_row() {
    let mut g = Grid::shared_rows();
    let rows = shared_view(&g);
    let original = g.add(1, 0, 1.0, 2.0);
    let first = g.add(2, 0, 3.0, 4.0);
    let second = g.add(3, 0, 5.0, 6.0);
    g.t.select_elements(&[first, second]);
    set_context(&mut g, vec![original], rows[0]);
    let list = [first, second];

    move_to(&mut g, &list, rows[1]);
    assert_eq!(g.t.g.mouse_down_row, Some(rows[1]));
    assert!(g.in_row(rows[1].0, first) && g.in_row(rows[3].0, first));
    assert!(g.in_row(rows[1].0, second) && g.in_row(rows[3].0, second));

    move_to(&mut g, &list, rows[2]);
    assert_eq!(g.t.g.mouse_down_row, Some(rows[2]));

    move_to(&mut g, &list, rows[3]);
    assert_eq!(g.t.g.mouse_down_row, Some(rows[3]));

    move_to(&mut g, &list, rows[2]);
    assert_eq!(g.t.g.mouse_down_row, Some(rows[2]));
    assert!(g.in_row(rows[2].0, first) && g.in_row(rows[2].0, second));
    assert!(!g.in_row(rows[0].0, first) && !g.in_row(rows[1].0, first) && !g.in_row(rows[3].0, first));
    assert!(g.in_row(rows[0].0, original));
}

#[test]
fn a_selection_spanning_rows_clips_to_the_last_valid_row() {
    let mut g = Grid::flat();
    let (anchor, lower) = (g.add(1, 0, 1.0, 2.0), g.add(2, 2, 1.0, 2.0));
    g.t.select_elements(&[anchor, lower]);
    let row0 = RowId(0);
    set_context(&mut g, vec![e(99)], row0);
    move_to(&mut g, &[anchor, lower], RowId(3));
    assert_eq!(g.t.g.mouse_down_row, Some(RowId(1)));
    assert_eq!(g.t.g.current_row_index, 1);
    assert!(g.in_row(1, anchor));
    assert!(g.in_row(3, lower));
}

#[test]
fn a_hidden_repeated_occurrence_tracks_the_visible_one() {
    let mut g = Grid::shared_rows();
    let rows = shared_view(&g);
    let clone = g.add(1, 1, 1.0, 2.0);
    g.t.select_elements(&[clone]);
    g.t.rows.set_visible(rows[3], false);
    g.t.rows.relayout();
    set_context(&mut g, vec![e(99)], rows[1]);
    move_to(&mut g, &[clone], rows[2]);
    assert_eq!(g.t.g.mouse_down_row, Some(rows[2]));
    assert_eq!(g.t.g.current_row_index, rows[2].0 as i32);
    assert!(g.in_row(rows[2].0, clone));
    assert!(!g.in_row(rows[3].0, clone));
}

#[test]
fn a_deprecated_destination_leaves_everything() {
    let mut g = Grid::flat();
    let clone = g.add(1, 0, 1.0, 2.0);
    g.t.select_elements(&[clone]);
    g.t.rows.row_mut(RowId(1)).tags.push(DEPRECATED_TAG);
    set_context(&mut g, vec![e(99)], RowId(0));
    move_to(&mut g, &[clone], RowId(1));
    assert_eq!(g.t.g.mouse_down_row, Some(RowId(0)));
    assert_eq!(g.t.g.current_row_index, 0);
    assert!(g.in_row(0, clone));
    assert!(!g.in_row(1, clone));
}

#[test]
fn an_unchanged_destination_does_nothing_and_the_last_row_is_accepted() {
    let mut g = Grid::flat();
    let clone = g.add(1, 0, 1.0, 2.0);
    g.t.select_elements(&[clone]);
    set_context(&mut g, vec![e(99)], RowId(0));
    move_to(&mut g, &[clone], RowId(0));
    assert_eq!(g.t.g.mouse_down_row, Some(RowId(0)));
    move_to(&mut g, &[clone], RowId(3));
    assert_eq!(g.t.g.mouse_down_row, Some(RowId(3)));
    assert!(g.in_row(3, clone));
    assert!(!g.in_row(0, clone));
}

#[test]
fn an_existing_effect_tracks_the_pointer_row() {
    let mut g = Grid::flat();
    let element = g.add(1, 0, 1.0, 2.0);
    g.t.select_elements(&[element]);
    set_context(&mut g, vec![element], RowId(0));
    move_to(&mut g, &[element], RowId(1));
    assert_eq!(g.t.g.mouse_down_row, Some(RowId(1)));
    move_to(&mut g, &[element], RowId(2));
    assert_eq!(g.t.g.mouse_down_row, Some(RowId(2)));
    assert!(g.in_row(2, element));
}

#[test]
fn a_clone_starting_on_a_repeated_occurrence_goes_up_and_down() {
    let mut g = Grid::shared_rows();
    let rows = shared_view(&g);
    let clone = g.add(1, 1, 1.0, 2.0);
    let original = g.add(2, 1, 3.0, 4.0);
    g.t.select_elements(&[clone]);
    set_context(&mut g, vec![original], rows[3]);
    move_to(&mut g, &[clone], rows[2]);
    assert_eq!(g.t.g.mouse_down_row, Some(rows[2]));
    move_to(&mut g, &[clone], rows[1]);
    assert_eq!(g.t.g.mouse_down_row, Some(rows[1]));
    move_to(&mut g, &[clone], rows[2]);
    assert_eq!(g.t.g.mouse_down_row, Some(rows[2]));
    assert!(g.in_row(rows[2].0, clone));
    assert!(!g.in_row(rows[1].0, clone));
    assert!(!g.in_row(rows[3].0, clone));
    assert!(g.in_row(rows[3].0, original));
}

#[test]
fn a_drag_across_rows_retargets_at_commit_and_undo_holds_the_old_target() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    begin_move(&mut g, 150, 8, Mods::NONE);
    g.drag(158, tab(1), Mods::NONE);
    assert!(g.in_row(1, a));
    assert_eq!(g.t.elements[&a].last_row, Some(RowId(1)));
    assert!(g.host.targets.is_empty(), "only at commit");
    g.up(158, tab(1), Mods::NONE);
    let node_b = g.t.rows.row(RowId(1)).node_uuid;
    assert_eq!(g.host.targets, [(a, node_b)]);
    let node_a = g.t.rows.row(RowId(0)).node_uuid;
    match &g.host.undo[0] {
        UndoAction::PlacementChanged { places, .. } => assert_eq!(places[0].target.as_ref().map(|t| t.0), Some(node_a)),
        _ => panic!("placement"),
    }
}

// ----------------------------------------------------------------------------------------------------------------------
// Resizing
// ----------------------------------------------------------------------------------------------------------------------

/// An effect 500 ms long at 10 ms per pixel.
fn resize_grid() -> (Grid, ElementId) {
    let mut g = Grid::flat();
    g.t.set_time_per_pixel(TimeSpan::from_ms_i(10));
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 10.5);
    (g, a)
}

#[test]
fn front_and_back_stop_at_ten_pixels() {
    let (mut g, a) = resize_grid();
    // Front: x 1000 is the start, a drag of +45 points asks for +450 ms.
    g.hover(1001, 5, Mods::NONE);
    assert_eq!(g.t.g.resize_zone, ResizeZone::Front);
    g.down(1001, 5, Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::HResizing);
    g.drag(1046, 5, Mods::NONE);
    assert_eq!(g.times(a), (sec(10.4), sec(10.5)));
    let out = g.up(1046, 5, Mods::NONE);
    assert_eq!(g.host.undo[0].description(), "Resize 1 effect");
    assert!(out.requests.contains(&HostRequest::UndoRecorded));

    let (mut g, a) = resize_grid();
    g.hover(1049, 5, Mods::NONE);
    assert_eq!(g.t.g.resize_zone, ResizeZone::Back);
    g.down(1049, 5, Mods::NONE);
    g.drag(1004, 5, Mods::NONE);
    assert_eq!(g.times(a), (sec(10.0), sec(10.1)));
}

#[test]
fn a_resize_clamps_to_the_sequence_and_a_duration_change_means_a_placeholder() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 1.0, 2.0);
    g.hover(11, 5, Mods::NONE);
    g.down(11, 5, Mods::NONE);
    g.drag(-50, 5, Mods::NONE);
    assert_eq!(g.times(a).0, TimeSpan::ZERO);
    assert!(g.t.elements[&a].rendered, "until release the picture stretches");
    g.up(-50, 5, Mods::NONE);
    assert!(!g.t.elements[&a].rendered);
}

#[test]
fn all_selected_resize_on_the_same_edge_by_the_same_amount() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    let b = g.add(2, 1, 15.0, 30.0);
    g.t.select_elements(&[a, b]);
    g.hover(199, 5, Mods::NONE);
    g.down(199, 5, Mods::NONE);
    g.drag(209, 5, Mods::NONE);
    assert_eq!(g.times(a), (sec(10.0), sec(21.0)));
    assert_eq!(g.times(b), (sec(15.0), sec(31.0)));
}

#[test]
fn command_press_in_a_zone_starts_nothing_and_option_with_many_warns() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 20.0);
    let b = g.add(2, 1, 10.0, 20.0);
    g.hover(101, 5, Mods::NONE);
    g.down(101, 5, Mods::CMD);
    assert_eq!(g.t.drag_state(), DragState::Normal);
    g.up(101, 5, Mods::CMD);
    g.t.select_elements(&[a, b]);
    g.hover(101, 5, Mods::NONE);
    let out = g.down(101, 5, Mods::OPT);
    assert_eq!(
        out.requests,
        [HostRequest::Warning {
            text: "Too many effects selected.\nMaximum selected effects for this action is 1".into(),
            title: "Warning".into()
        }]
    );
    assert_eq!(g.t.drag_state(), DragState::Normal);
}

fn adjoining_grid() -> (Grid, ElementId, ElementId, ElementId) {
    let mut g = Grid::flat();
    g.t.set_time_per_pixel(TimeSpan::from_ms_i(10));
    g.t.opts.snap_enabled = false;
    let left = g.add(1, 0, 1.0, 2.0);
    let middle = g.add(2, 0, 2.0, 3.0);
    let right = g.add(3, 0, 3.0, 4.0);
    (g, left, middle, right)
}

#[test]
fn adjoining_neighbours_follow_and_stop_at_ten_pixels() {
    let (mut g, left, middle, _) = adjoining_grid();
    // Front edge of the middle, ⌥.
    g.hover(201, 5, Mods::NONE);
    g.down(201, 5, Mods::OPT);
    g.drag(221, 5, Mods::OPT);
    assert_eq!(g.times(middle), (sec(2.2), sec(3.0)));
    assert_eq!(g.times(left), (sec(1.0), sec(2.2)));
    // Far enough left that the neighbour is at its minimum: the primary stops there.
    g.drag(100, 5, Mods::OPT);
    assert_eq!(g.times(left), (sec(1.0), sec(1.1)));
    assert_eq!(g.times(middle), (sec(1.1), sec(3.0)));
    g.up(100, 5, Mods::OPT);
    // Undo restores both.
    match &g.host.undo[0] {
        UndoAction::PlacementChanged { places, .. } => {
            assert_eq!(places.len(), 2);
            assert!(places.iter().any(|p| p.id == left && p.span == sec(1.0)));
            assert!(places.iter().any(|p| p.id == middle && p.start == sec(2.0)));
        }
        _ => panic!("placement"),
    }
    assert_eq!(g.host.undo[0].description(), "Resize 2 effects");
}

#[test]
fn adjoining_on_the_back_edge_keeps_the_neighbours_end() {
    let (mut g, _, middle, right) = adjoining_grid();
    g.hover(299, 5, Mods::NONE);
    g.down(299, 5, Mods::OPT);
    g.drag(279, 5, Mods::OPT);
    assert_eq!(g.times(middle), (sec(2.0), sec(2.8)));
    assert_eq!(g.times(right), (sec(2.8), sec(4.0)));
    g.drag(500, 5, Mods::OPT);
    // The neighbour is held at 10 pixels from its end, and the primary stops at its start.
    assert_eq!(g.times(right), (sec(3.9), sec(4.0)));
    assert_eq!(g.times(middle), (sec(2.0), sec(3.9)));
}

#[test]
fn letting_go_of_option_stops_neighbours_following() {
    let (mut g, left, _, _) = adjoining_grid();
    g.hover(201, 5, Mods::NONE);
    g.down(201, 5, Mods::OPT);
    g.drag(221, 5, Mods::OPT);
    g.drag(231, 5, Mods::NONE);
    assert_eq!(g.times(left), (sec(1.0), sec(2.2)));
}

#[test]
fn the_same_layer_form_filters_and_only_twenty_entries_are_inspected() {
    let (mut g, left, middle, _) = adjoining_grid();
    g.t.elements.get_mut(&left).unwrap().desc.layer_level = 1;
    assert_eq!(g.t.adjoining(middle, ResizeZone::Front, Some(0)), Vec::<ElementId>::new());
    assert_eq!(g.t.adjoining(middle, ResizeZone::Front, Some(1)), [left]);
    assert_eq!(g.t.adjoining(middle, ResizeZone::Front, None), [left]);

    let mut g = Grid::flat();
    let last = g.add(1, 0, 100.0 / 10.0, 11.0);
    // 25 short effects after it, and the one that would meet its end is too far along to be looked at.
    for i in 0..21 {
        let s = 12.0 + i as f64 * 0.1;
        g.add(10 + i, 0, s, s + 0.05);
    }
    assert!(!g.t.adjoining(last, ResizeZone::Back, None).is_empty());
    let mut g = Grid::flat();
    let last = g.add(1, 0, 10.0, 11.0);
    // Twenty overlapping starts don't qualify, so the twenty-first, which would, is out of reach.
    for i in 0..20 {
        g.add(10 + i, 0, 10.5, 12.0);
    }
    g.add(50, 0, 20.0, 21.0);
    assert!(g.t.adjoining(last, ResizeZone::Back, None).is_empty());
}

// ----------------------------------------------------------------------------------------------------------------------
// Snapping during a drag
// ----------------------------------------------------------------------------------------------------------------------

/// 100 ms per pixel and strength 2: a level 5 window is ±1 s.
fn snap_grid() -> (Grid, ElementId) {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    (g, a)
}

#[test]
fn a_moving_effect_snaps_to_a_neighbour_in_its_row() {
    let (mut g, a) = snap_grid();
    g.add(2, 0, 20.0, 25.0);
    begin_move(&mut g, 110, 8, Mods::NONE);
    // +7.5 s: its end is 19.5, in range of 20.
    g.drag(110 + 8 + 75, 8, Mods::NONE);
    assert_eq!(g.times(a), (sec(18.0), sec(20.0)));
    g.up(110 + 8 + 75, 8, Mods::NONE);
    assert!(g.t.g.drag_snap.is_none());
}

#[test]
fn snapping_off_builds_nothing() {
    let (mut g, a) = snap_grid();
    g.add(2, 0, 20.0, 25.0);
    g.t.opts.snap_enabled = false;
    begin_move(&mut g, 110, 8, Mods::NONE);
    g.drag(110 + 8 + 75, 8, Mods::NONE);
    assert_eq!(g.times(a).0, sec(17.5));
    // Turning it on mid-drag finds no points.
    g.t.opts.snap_enabled = true;
    g.drag(110 + 8 + 76, 8, Mods::NONE);
    assert_eq!(g.times(a).0, sec(17.6));
}

#[test]
fn row_zero_considers_every_selected_effect_q11() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    let b = g.add(2, 1, 30.0, 32.0);
    g.t.select_elements(&[a, b]);
    g.hover(110, 8, Mods::NONE);
    g.down(110, 8, Mods::NONE);
    g.drag(118, 8, Mods::NONE);
    assert_eq!(g.t.g.current_row_index, 0);
    let candidates = g.t.snap_candidates(&[a, b], 0);
    assert_eq!(candidates.len(), 2);
    // The tracked row above zero only considers what's in that row.
    let candidates = g.t.snap_candidates(&[a, b], 1);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].1, RowId(1));
}

#[test]
fn clones_snap_to_their_originals() {
    let (mut g, a) = snap_grid();
    g.hover(110, 8, Mods::NONE);
    g.down(110, 8, Mods::CMD);
    g.drag(118, 8, Mods::CMD);
    let clone = g.selected()[0];
    // Asking for +2.4 s brings its start to 12.4, in reach of the original's end at 12.
    g.drag(118 + 24, 8, Mods::CMD);
    assert_eq!(g.times(clone).0, sec(12.0));
    assert_eq!(g.times(a).0, sec(10.0));
}

#[test]
fn a_resize_limit_beats_a_snap() {
    let mut g = Grid::flat();
    g.t.set_time_per_pixel(TimeSpan::from_ms_i(10));
    let a = g.add(1, 0, 10.0, 10.5);
    // A point right at the far edge, closer than the minimum width allows.
    g.add(2, 0, 10.45, 11.0);
    g.hover(1001, 5, Mods::NONE);
    g.down(1001, 5, Mods::NONE);
    g.drag(1046, 5, Mods::NONE);
    assert_eq!(g.times(a), (sec(10.4), sec(10.5)));
}

#[test]
fn drag_snap_uses_unselected_effects_of_visible_rows_only() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    let selected_too = g.add(2, 0, 20.0, 25.0);
    g.t.select_elements(&[a, selected_too]);
    g.hover(110, 8, Mods::NONE);
    g.down(110, 8, Mods::NONE);
    g.drag(118, 8, Mods::NONE);
    assert!(g.t.g.drag_snap.as_ref().unwrap().is_empty());
    let _ = SnapEdge::Start;
}

// ----------------------------------------------------------------------------------------------------------------------
// Edge scrolling
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn autoscroll_runs_outside_the_margin_and_replays_the_pointer() {
    let mut g = Grid::flat();
    g.t.info.total = sec(600.0);
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    begin_move(&mut g, 150, 8, Mods::NONE);
    assert!(!g.drag(158, 8 + 100, Mods::NONE).autoscroll);
    // 24 points past the right margin's start means outside 30.
    let out = g.drag(1000 - 24 + 30, 108, Mods::NONE);
    assert!(out.autoscroll);
    assert_eq!(g.t.g.mouse_outside, (30, 0));
    let before = g.times(a).0;
    let out = g.t.grid_autoscroll_tick(Mods::NONE, &mut g.host);
    assert!(out.autoscroll);
    // 30 / 4 is 7 pixels of scroll with integer division, and the drag follows.
    assert_eq!(g.t.scroll_x(), 7);
    assert_eq!(g.times(a).0, before + TimeSpan::from_ms_i(700));
    // Back inside stops it.
    assert!(!g.drag(500, 108, Mods::NONE).autoscroll);
    assert_eq!(g.t.g.mouse_outside, (0, 0));
    assert!(!g.up(500, 108, Mods::NONE).autoscroll);
}

// ----------------------------------------------------------------------------------------------------------------------
// Drawing
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn draw_mode_makes_one_effect_per_row_and_selects_them() {
    let mut g = Grid::flat();
    g.t.opts.draw_mode = true;
    g.t.opts.selected_effect = Some(TYPE_A);
    let out = g.hover(100, 5, Mods::NONE);
    assert_eq!(out.cursor, CursorKind::Crosshair);
    g.down(100, 5, Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::Drawing);
    g.drag(150, tab(1), Mods::NONE);
    // Stretched to whole rows.
    assert_eq!(g.t.g.drawing_area, IRect::new(100, 1, 50, 37));
    g.up(150, tab(1), Mods::NONE);
    assert_eq!(g.host.added.len(), 2);
    assert_eq!(g.host.added[0].2, sec(10.0));
    assert_eq!(g.host.added[0].3, sec(5.0));
    assert_eq!(g.selected().len(), 2);
    assert_eq!(g.host.undo.last().unwrap().description(), "Added 2 effects");
    assert_eq!(g.t.cursor, sec(10.0));
    assert!(g.t.g.drawing_area.is_empty());
}

#[test]
fn a_short_draw_is_a_background_click_and_makes_nothing() {
    let mut g = Grid::flat();
    g.t.opts.draw_mode = true;
    g.t.opts.selected_effect = Some(TYPE_A);
    g.down(100, 5, Mods::NONE);
    g.up(100, 5, Mods::NONE);
    assert!(g.host.added.is_empty());
    assert_eq!(g.t.cursor, sec(10.0));
    assert_eq!(g.t.active_row(), Some(RowId(0)));
}

#[test]
fn option_and_resize_zones_opt_out_of_drawing_and_right_press_does_not_strand_it_q5() {
    let mut g = Grid::flat();
    g.t.opts.draw_mode = true;
    g.t.opts.selected_effect = Some(TYPE_A);
    let a = g.add(1, 0, 10.0, 20.0);
    g.down(150, 5, Mods::OPT);
    assert_ne!(g.t.drag_state(), DragState::Drawing);
    g.up(150, 5, Mods::OPT);
    g.t.clear_selection();
    g.hover(102, 5, Mods::NONE);
    g.down(102, 5, Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::HResizing);
    g.up(102, 5, Mods::NONE);
    g.right_down(150, 5, Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::Normal);
    assert_eq!(g.selected(), [a]);
}

#[test]
fn the_middle_button_draws_in_any_mode_on_a_row() {
    let mut g = Grid::flat();
    g.t.opts.selected_effect = Some(TYPE_A);
    let press = PointerEvent::new(100, 5, Some(Button::Middle), Mods::NONE);
    let out = g.t.grid_mouse_down(press);
    assert_eq!(out.cursor, CursorKind::Crosshair);
    assert_eq!(g.t.drag_state(), DragState::Drawing);
    g.drag(120, 5, Mods::NONE);
    let release = PointerEvent::new(120, 5, Some(Button::Middle), Mods::NONE);
    g.t.grid_mouse_up(release, &mut g.host);
    assert_eq!(g.host.added.len(), 1);
    assert_eq!(g.t.drag_state(), DragState::Normal);
    // Without an effect type chosen, nothing.
    g.t.opts.selected_effect = None;
    g.t.grid_mouse_down(press);
    assert_eq!(g.t.drag_state(), DragState::Normal);
}

#[test]
fn indicator_lines_follow_the_drag() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    g.t.opts.resize_indicator = true;
    let a = g.add(1, 0, 10.0, 20.0);
    g.hover(150, 8, Mods::NONE);
    g.down(150, 8, Mods::NONE);
    assert_eq!(g.t.resize_indicator_lines(), [100.0, 199.0]);
    g.drag(158, 8, Mods::NONE);
    g.drag(168, 8, Mods::NONE);
    assert_eq!(g.t.resize_indicator_lines(), [110.0, 209.0]);
    // Published from the drag, not from paint (FIX Q14).
    assert_eq!(g.t.guides(), [g.times(a).0, g.times(a).1]);
    g.up(168, 8, Mods::NONE);
    assert!(g.t.guides().is_empty());
    assert!(g.t.resize_indicator_lines().is_empty());
}

// ----------------------------------------------------------------------------------------------------------------------
// Align: GridAlignmentNullReferenceTests.cs
// ----------------------------------------------------------------------------------------------------------------------

fn align_grid() -> (Grid, ElementId, ElementId) {
    let mut g = Grid::flat();
    let reference = g.add(1, 0, 10.0, 20.0);
    let other = g.add(2, 1, 12.0, 15.0);
    (g, reference, other)
}

fn run_align(g: &mut Grid, kind: AlignKind, other: ElementId, reference: ElementId, hold: bool) -> (TimeSpan, TimeSpan) {
    assert_eq!(g.t.align(kind, &[other], Some(reference), hold, &mut g.host), AlignResult::Done);
    g.times(other)
}

#[test]
fn align_with_no_reference_changes_nothing() {
    let (mut g, _, other) = align_grid();
    for kind in [AlignKind::Start, AlignKind::End, AlignKind::Both, AlignKind::Durations, AlignKind::StartToEnd, AlignKind::EndToStart, AlignKind::Centers] {
        assert_eq!(g.t.align(kind, &[other], None, false, &mut g.host), AlignResult::NoReference);
    }
    assert_eq!(g.times(other), (sec(12.0), sec(15.0)));
    assert!(g.host.undo.is_empty());
}

#[test]
fn align_start() {
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::Start, other, r, false), (sec(10.0), sec(15.0)));
    assert_eq!(g.host.undo[0].description(), "Start Align 1 effect");
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::Start, other, r, true), (sec(10.0), sec(13.0)));
    // Entirely before the reference: just moved.
    let mut g = Grid::flat();
    let r = g.add(1, 0, 10.0, 20.0);
    let other = g.add(2, 1, 1.0, 2.0);
    assert_eq!(run_align(&mut g, AlignKind::Start, other, r, false), (sec(10.0), sec(11.0)));
}

#[test]
fn align_end() {
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::End, other, r, false), (sec(12.0), sec(20.0)));
    assert_eq!(g.host.undo[0].description(), "End Align 1 effect");
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::End, other, r, true), (sec(17.0), sec(20.0)));
    let mut g = Grid::flat();
    let r = g.add(1, 0, 10.0, 20.0);
    let other = g.add(2, 1, 25.0, 27.0);
    assert_eq!(run_align(&mut g, AlignKind::End, other, r, false), (sec(18.0), sec(20.0)));
}

#[test]
fn align_both_and_durations() {
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::Both, other, r, false), (sec(10.0), sec(20.0)));
    assert_eq!(g.host.undo[0].description(), "Align 1 effect");
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::Durations, other, r, false), (sec(12.0), sec(22.0)));
    assert_eq!(g.host.undo[0].description(), "Duration Align 1 effect");
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::Durations, other, r, true), (sec(5.0), sec(15.0)));
}

#[test]
fn align_start_to_end() {
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::StartToEnd, other, r, false), (sec(20.0), sec(23.0)));
    assert_eq!(g.host.undo[0].description(), "Start to End Align 1 effect");
    // ⇧ moves the start only, unless that would leave under 50 ms.
    let mut g = Grid::flat();
    let r = g.add(1, 0, 10.0, 20.0);
    let other = g.add(2, 1, 12.0, 30.0);
    assert_eq!(run_align(&mut g, AlignKind::StartToEnd, other, r, true), (sec(20.0), sec(30.0)));
    let (mut g, r, other) = align_grid();
    // Its end is before the new start, so it keeps its length.
    assert_eq!(run_align(&mut g, AlignKind::StartToEnd, other, r, true), (sec(20.0), sec(23.0)));
    // Past the total is clamped.
    let mut g = Grid::flat();
    let r = g.add(1, 0, 10.0, 59.0);
    let other = g.add(2, 1, 12.0, 15.0);
    assert_eq!(run_align(&mut g, AlignKind::StartToEnd, other, r, false), (sec(59.0), sec(60.0)));
}

#[test]
fn align_end_to_start() {
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::EndToStart, other, r, false), (sec(7.0), sec(10.0)));
    assert_eq!(g.host.undo[0].description(), "End to Start Align 1 effect");
    let mut g = Grid::flat();
    let r = g.add(1, 0, 10.0, 20.0);
    let other = g.add(2, 1, 1.0, 5.0);
    assert_eq!(run_align(&mut g, AlignKind::EndToStart, other, r, true), (sec(1.0), sec(10.0)));
    // Start clamped at zero.
    let mut g = Grid::flat();
    let r = g.add(1, 0, 1.0, 20.0);
    let other = g.add(2, 1, 5.0, 10.0);
    assert_eq!(run_align(&mut g, AlignKind::EndToStart, other, r, false), (TimeSpan::ZERO, sec(1.0)));
}

#[test]
fn align_centres_skip_on_an_equal_start_q15() {
    let (mut g, r, other) = align_grid();
    assert_eq!(run_align(&mut g, AlignKind::Centers, other, r, false), (sec(13.5), sec(16.5)));
    assert_eq!(g.host.undo[0].description(), "Center Align 1 effect");
    // Same start, different centres: skipped.
    let mut g = Grid::flat();
    let r = g.add(1, 0, 10.0, 20.0);
    let other = g.add(2, 1, 10.0, 12.0);
    assert_eq!(run_align(&mut g, AlignKind::Centers, other, r, false), (sec(10.0), sec(12.0)));
}

#[test]
fn thirty_three_on_a_row_warns_and_thirty_two_is_allowed() {
    let mut g = Grid::flat();
    let reference = g.add(1000, 1, 1.0, 2.0);
    let mut ids = Vec::new();
    for i in 0..33 {
        ids.push(g.add(i + 1, 0, i as f64, i as f64 + 0.5));
    }
    g.t.select_elements(&ids[..32]);
    assert!(g.t.ok_to_use_alignment(&ids[..32]));
    assert_eq!(g.t.align(AlignKind::Both, &ids[..32], Some(reference), false, &mut g.host), AlignResult::Done);
    let out = g.t.take_outcome();
    assert!(!out.requests.iter().any(|r| matches!(r, HostRequest::Warning { .. })));
    g.t.select_elements(&ids);
    assert!(!g.t.ok_to_use_alignment(&ids));
    assert_eq!(g.t.align(AlignKind::Both, &ids, Some(reference), false, &mut g.host), AlignResult::TooManyPerRow);
    let out = g.t.take_outcome();
    assert_eq!(out.requests.last(), Some(&HostRequest::Warning { text: ALIGNMENT_WARNING.to_string(), title: "Warning".into() }));
    // KEEP Q16: the text says 4, though the limit is 32.
    assert!(ALIGNMENT_WARNING.ends_with("is 4"));
}

// ----------------------------------------------------------------------------------------------------------------------
// Batch, split, gaps, move by time
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn a_batch_skips_bad_entries_and_records_one_undo() {
    let mut g = Grid::flat();
    let (a, b, c) = (g.add(1, 0, 1.0, 2.0), g.add(2, 1, 1.0, 2.0), g.add(3, 2, 1.0, 2.0));
    let moves = [(a, sec(3.0), sec(4.0)), (b, sec(3.0), sec(61.0)), (c, sec(3.0), sec(3.0) + TimeSpan::from_ms_i(0))];
    g.t.move_resize_batch(&moves, MoveKind::Moved, &mut g.host);
    assert_eq!(g.times(a), (sec(3.0), sec(4.0)));
    assert_eq!(g.times(b), (sec(1.0), sec(2.0)));
    assert_eq!(g.times(c), (sec(1.0), sec(2.0)));
    assert_eq!(g.host.undo.len(), 1);
    assert_eq!(g.host.undo[0].description(), "Moved 3 effects");
    assert_eq!(g.host.times, [(a, sec(3.0), sec(1.0))]);
}

#[test]
fn single_moves_refuse_what_upstream_refuses() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    assert!(!g.t.move_resize_element(a, sec(61.0), sec(1.0), MoveKind::Moved, &mut g.host));
    assert!(!g.t.move_resize_element(a, sec(59.5), sec(1.0), MoveKind::Moved, &mut g.host));
    assert!(!g.t.move_resize_element(a, sec(1.0), TimeSpan::from_ms_i(0), MoveKind::Moved, &mut g.host));
    assert!(!g.t.move_resize_element_by_start_end(a, sec(5.0), sec(5.0), &mut g.host));
    assert!(g.t.move_resize_element_by_start_end(a, sec(5.0), sec(6.0), &mut g.host));
    assert_eq!(g.times(a), (sec(5.0), sec(6.0)));
    assert_eq!(g.host.undo.len(), 1);
}

#[test]
fn split_makes_a_clone_and_many_undo_entries_q17() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 20.0);
    g.t.split_at(&[a], sec(15.0), &mut g.host);
    assert_eq!(g.times(a), (sec(10.0), sec(15.0)));
    let clone = g.t.rows.row(RowId(0)).occurrences.iter().find(|o| o.id != a).unwrap().id;
    assert_eq!(g.times(clone), (sec(15.0), sec(20.0)));
    let labels: Vec<String> = g.host.undo.iter().map(UndoAction::description).collect();
    assert_eq!(labels, ["Added 1 effect", "Moved 1 effect", "Moved 1 effect"]);
}

#[test]
fn split_selected_needs_the_pointer_inside_the_view_and_the_effect() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 20.0);
    let far = g.add(2, 1, 30.0, 40.0);
    g.t.select_elements(&[a, far]);
    g.t.set_viewport(100, 100, 100);
    g.t.split_selected_at(sec(15.0), &mut g.host);
    // Past the visible end, 10 s.
    assert_eq!(g.times(a), (sec(10.0), sec(20.0)));
    g.t.set_viewport(1000, 100, 1000);
    g.t.split_selected_at(sec(15.0), &mut g.host);
    assert_eq!(g.times(a), (sec(10.0), sec(15.0)));
    assert_eq!(g.times(far), (sec(30.0), sec(40.0)));
}

#[test]
fn close_gaps_asks_first_then_closes_and_shortens_overlaps() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 0, 2.05, 3.0);
    let exact = g.add(3, 0, 3.0, 4.0);
    let wide = g.add(4, 0, 5.0, 6.0);
    assert!(!g.t.close_gaps(0.1, false, &mut g.host), "the whole sequence needs a yes");
    assert!(g.host.undo.is_empty());
    assert!(g.t.close_gaps(0.1, true, &mut g.host));
    assert_eq!(g.times(b).0, sec(2.0));
    // Exact contact skipped; a gap above the threshold stays.
    assert_eq!(g.times(exact).0, sec(3.0));
    assert_eq!(g.times(wide).0, sec(5.0));
    assert_eq!(g.times(a).0, sec(1.0));
    assert_eq!(g.host.undo[0].description(), "Moved 1 effect");
    // An overlap is below the threshold too, so it's shortened.
    let mut g = Grid::flat();
    g.add(1, 0, 1.0, 3.0);
    let late = g.add(2, 0, 2.0, 4.0);
    g.t.close_gaps(0.1, true, &mut g.host);
    assert_eq!(g.times(late), (sec(3.0), sec(4.0)));
}

#[test]
fn close_gaps_with_a_selection_takes_only_selected_effects_in_visible_rows() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 0, 2.05, 3.0);
    let c = g.add(3, 0, 3.05, 4.0);
    g.t.select_elements(&[a, b]);
    assert!(g.t.close_gaps(0.1, false, &mut g.host));
    assert_eq!(g.times(b).0, sec(2.0));
    assert_eq!(g.times(c).0, sec(3.05));
}

#[test]
fn move_by_time_limits_without_clipping() {
    let mut g = Grid::flat();
    let (a, b) = (g.add(1, 0, 2.0, 3.0), g.add(2, 1, 50.0, 59.0));
    assert_eq!(g.t.move_by_time(&[a, b], sec(-5.0), false, &mut g.host), sec(-2.0));
    assert_eq!(g.times(a).0, TimeSpan::ZERO);
    assert_eq!(g.t.move_by_time(&[a, b], sec(5.0), false, &mut g.host), sec(3.0));
    assert_eq!(g.times(b).1, sec(60.0));
    assert_eq!(g.host.undo.len(), 2);
    assert_eq!(g.t.move_by_time(&[], sec(1.0), false, &mut g.host), TimeSpan::ZERO);
    assert_eq!(g.host.undo.len(), 2);
}

#[test]
fn move_by_time_clips_and_deletes_q18() {
    let mut g = Grid::flat();
    let gone_left = g.add(1, 0, 1.0, 2.0);
    let clipped_left = g.add(2, 1, 3.0, 8.0);
    let whole = g.add(3, 2, 10.0, 12.0);
    assert_eq!(g.t.move_by_time(&[gone_left, clipped_left, whole], sec(-5.0), true, &mut g.host), sec(-5.0));
    // KEEP: off its last row and nothing more. It stays in the sequence at its old time.
    assert!(g.t.elements.contains_key(&gone_left));
    assert!(!g.in_row(0, gone_left));
    assert_eq!(g.times(gone_left), (sec(1.0), sec(2.0)));
    assert!(g.host.removed.is_empty());
    // KEEP Q18: the start is zeroed before the duration uses it, so the duration shrinks by the whole offset.
    assert_eq!(g.times(clipped_left), (TimeSpan::ZERO, sec(0.0)));
    assert_eq!(g.t.elements[&clipped_left].duration, TimeSpan::ZERO);
    assert_eq!(g.times(whole), (sec(5.0), sec(7.0)));
    // Past the end.
    let mut g = Grid::flat();
    let clipped = g.add(1, 0, 50.0, 58.0);
    let gone = g.add(2, 1, 58.0, 59.0);
    g.t.move_by_time(&[clipped, gone], sec(5.0), true, &mut g.host);
    assert_eq!(g.times(clipped), (sec(55.0), sec(60.0)));
    assert!(!g.in_row(1, gone));
    assert_eq!(g.host.undo[0].description(), "Moved 2 effects");
    match &g.host.undo[0] {
        UndoAction::Group { actions, .. } => assert!(actions.iter().any(|a| matches!(a, UndoAction::RowsRestored { ids } if ids == &[gone]))),
        _ => panic!("a group with the row to put back"),
    }
}

#[test]
fn nudge_is_two_pixels_worth() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 11.0);
    g.t.select_elements(&[a]);
    g.t.nudge_selected(1, &mut g.host);
    assert_eq!(g.times(a).0, sec(10.2));
    g.t.nudge_selected(-1, &mut g.host);
    assert_eq!(g.times(a).0, sec(10.0));
}

#[test]
fn range_queries() {
    let mut g = Grid::shared_rows();
    let a = g.add(1, 1, 1.0, 2.0);
    let b = g.add(2, 0, 1.5, 3.0);
    assert_eq!(g.t.elements_within(sec(1.0), sec(1.5), false), [b, a]);
    assert_eq!(g.t.elements_within(sec(1.2), sec(1.4), false), Vec::<ElementId>::new());
    // Not distinct: Shared shows twice.
    assert_eq!(g.t.elements_at_time(sec(1.2)).iter().filter(|id| **id == a).count(), 2);
    assert_eq!(g.t.elements_at_time(sec(2.0)), [b], "end is exclusive");
}

#[test]
fn reveal_opens_ancestors_and_scrolls_to_the_last() {
    let mut g = Grid::shared_rows();
    g.t.collapse_all_rows();
    let a = g.add(1, 1, 30.0, 31.0);
    assert!(g.t.rows.row_top(g.t.elements[&a].last_row.unwrap()).is_none());
    g.t.set_viewport(100, 20, 100);
    g.t.reveal_elements(&[a]);
    assert_eq!(g.selected(), [a]);
    assert_eq!(g.t.info.visible_start, sec(30.0));
    let top = g.t.rows.row_top(g.t.elements[&a].last_row.unwrap()).unwrap();
    assert_eq!(g.t.vertical_offset, top);
    assert!(g.t.take_outcome().selection_changed);
}

// ----------------------------------------------------------------------------------------------------------------------
// Wheels: MarksBarMouseWheelTests.cs, TimelineZoomTests.cs
// ----------------------------------------------------------------------------------------------------------------------

fn wheel(surface: Surface, dx: f64, dy: f64, precise: bool, mods: Mods) -> ScrollEvent {
    ScrollEvent { surface, dx, dy, precise, mods, pointer_offset_x: 400, grid_y: 5 }
}

fn view_grid() -> Grid {
    let mut g = Grid::flat();
    g.t.info.total = sec(600.0);
    g.t.set_viewport(1000, 40, 1000);
    g.t.set_visible_start(sec(100.0));
    g
}

#[test]
fn plain_wheel_scrolls_rows_forty_points_a_notch() {
    let mut g = view_grid();
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, -1.0, false, Mods::NONE));
    assert_eq!(g.t.vertical_offset, 40);
    g.t.scroll_wheel(wheel(Surface::RowList, 0.0, 1.0, false, Mods::NONE));
    assert_eq!(g.t.vertical_offset, 0);
}

#[test]
fn command_wheel_zooms_and_shift_pans() {
    let mut g = view_grid();
    g.t.scroll_wheel(wheel(Surface::Ruler, 0.0, 1.0, false, Mods::CMD));
    assert_eq!(g.t.tpp().ticks(), 900_000);
    assert_eq!(g.t.info.visible_start, sec(100.0), "left edge stays");
    g.t.opts.zoom_under_mouse = true;
    g.t.scroll_wheel(wheel(Surface::Waveform, 0.0, 1.0, false, Mods::CMD));
    assert!(g.t.info.visible_start > sec(100.0));

    let mut g = view_grid();
    // macOS turns ⇧-wheel into horizontal deltas.
    g.t.scroll_wheel(wheel(Surface::Grid, -1.0, 0.0, false, Mods::SHIFT));
    assert_eq!(g.t.info.visible_start, sec(110.0));
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, 1.0, false, Mods::SHIFT));
    assert_eq!(g.t.info.visible_start, sec(100.0));
}

#[test]
fn command_shift_wheel_scales_all_rows_and_command_option_one() {
    let mut g = view_grid();
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, 1.0, false, Mods { shift: true, cmd: true, opt: false }));
    assert_eq!(g.t.rows.row(RowId(0)).height, 25);
    assert_eq!(g.t.rows.row_height, 25);
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, -1.0, false, Mods { shift: true, cmd: true, opt: false }));
    assert_eq!(g.t.rows.row(RowId(0)).height, 20);
    // ⌘⌥ on the row under the pointer, with scroll accounted for (FIX Q2).
    g.t.set_vertical_offset(20);
    let at_scroll = g.t.vertical_offset;
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, 1.0, false, Mods { shift: false, cmd: true, opt: true }));
    let row = g.t.rows.row_at(at_scroll + 5).unwrap();
    assert_eq!(g.t.rows.row(row).height, 25);
    assert_eq!(g.t.rows.row(RowId(0)).height, 20);
}

#[test]
fn the_marks_bar_only_navigates() {
    let mut g = view_grid();
    g.t.scroll_wheel(wheel(Surface::MarksBar, 0.0, -1.0, false, Mods::NONE));
    assert_eq!(g.t.vertical_offset, 0);
    assert_eq!(g.t.info.visible_start, sec(100.0));
    g.t.scroll_wheel(wheel(Surface::MarksBar, 0.0, 1.0, false, Mods { shift: false, cmd: true, opt: true }));
    assert_eq!(g.t.rows.row(RowId(0)).height, 20);
    assert_eq!(g.t.tpp().ticks(), 1_000_000);
    g.t.scroll_wheel(wheel(Surface::MarksBar, 0.0, 1.0, false, Mods::CMD));
    assert_eq!(g.t.tpp().ticks(), 900_000);
    // ⇧ wins over ⌘ and ⌥.
    let mut g = view_grid();
    g.t.scroll_wheel(wheel(Surface::MarksBar, 0.0, -1.0, false, Mods { shift: true, cmd: true, opt: true }));
    assert_eq!(g.t.info.visible_start, sec(110.0));
    assert_eq!(g.t.tpp().ticks(), 1_000_000);
}

#[test]
fn tilt_is_ten_percent_whatever_the_size() {
    let mut g = view_grid();
    g.t.scroll_wheel(wheel(Surface::Grid, -1.0, 0.0, false, Mods::NONE));
    assert_eq!(g.t.info.visible_start, sec(110.0));
    g.t.scroll_wheel(wheel(Surface::MarksBar, 3.0, 0.0, false, Mods::NONE));
    assert_eq!(g.t.info.visible_start, sec(100.0));
    g.t.scroll_wheel(wheel(Surface::RowList, -1.0, 0.0, false, Mods::NONE));
    assert_eq!(g.t.info.visible_start, sec(100.0));
}

#[test]
fn trackpad_moves_a_point_per_point_and_steps_row_heights() {
    let mut g = view_grid();
    g.t.scroll_wheel(wheel(Surface::Grid, -7.0, -3.5, true, Mods::NONE));
    assert_eq!(g.t.info.visible_start, sec(100.7));
    assert_eq!(g.t.vertical_offset, 3);
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, -0.5, true, Mods::NONE));
    assert_eq!(g.t.vertical_offset, 4, "fractions add up");
    // Over the marks bar plain vertical does nothing but plain horizontal pans.
    let at = g.t.vertical_offset;
    g.t.scroll_wheel(wheel(Surface::MarksBar, -3.0, -9.0, true, Mods::NONE));
    assert_eq!(g.t.vertical_offset, at);
    assert_eq!(g.t.info.visible_start, sec(101.0));
    // ⌘ zooms a little per event.
    let mut g = view_grid();
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, 12.0, true, Mods::CMD));
    assert_eq!(g.t.tpp().ticks(), 900_000);
    // ⌘⇧ only steps when the accumulated delta passes 120.
    let shiftcmd = Mods { shift: true, cmd: true, opt: false };
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, 6.0, true, shiftcmd));
    assert_eq!(g.t.rows.row(RowId(0)).height, 20);
    g.t.scroll_wheel(wheel(Surface::Grid, 0.0, 6.0, true, shiftcmd));
    assert_eq!(g.t.rows.row(RowId(0)).height, 25);
}

#[test]
fn a_pinch_anchors_on_the_pointer_and_shift_steps_rows() {
    let mut g = view_grid();
    g.t.magnify(0.25, 400, Mods::NONE);
    assert_eq!(g.t.tpp().ticks(), 800_000);
    assert!(g.t.info.visible_start > sec(100.0));
    g.t.magnify(0.1, 400, Mods::SHIFT);
    assert_eq!(g.t.rows.row(RowId(0)).height, 20);
    g.t.magnify(0.2, 400, Mods::SHIFT);
    assert_eq!(g.t.rows.row(RowId(0)).height, 25);
    g.t.magnify(-2.0, 400, Mods::NONE);
    assert_eq!(g.t.tpp().ticks(), 800_000);
}

#[test]
fn view_setters_clamp_and_offset_follows_rows() {
    let mut g = Grid::flat();
    g.t.set_viewport(100, 40, 100);
    g.t.set_vertical_offset(1000);
    assert_eq!(g.t.vertical_offset, 40);
    g.t.set_vertical_offset(-5);
    assert_eq!(g.t.vertical_offset, 0);
    g.t.set_vertical_offset(40);
    g.t.rows.set_visible(RowId(3), false);
    g.t.rows.relayout();
    g.t.rows_changed();
    assert_eq!(g.t.vertical_offset, 20);
    // A window taller than the rows pins it to zero.
    g.t.set_viewport(100, 500, 100);
    assert_eq!(g.t.vertical_offset, 0);
    // A resize that would pass the end moves the start back.
    let mut g = Grid::flat();
    g.t.set_viewport(100, 100, 100);
    g.t.set_visible_start(sec(50.0));
    g.t.set_viewport(200, 100, 200);
    assert_eq!(g.t.info.visible_start, sec(40.0));
}

#[test]
fn zooming_recalculates_mark_windows() {
    let mut g = Grid::flat();
    g.t.zoom_time(0.5);
    assert_eq!(g.t.snap_index.tpp, g.t.tpp());
    let out = g.t.take_outcome();
    assert!(out.dirty.grid_all && out.dirty.ruler && out.dirty.marks_bar && out.dirty.waveform);
}

// ----------------------------------------------------------------------------------------------------------------------
// Rendering state: TimelineGridDrawingTests.cs
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn selection_never_touches_the_tiles() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    g.t.take_outcome();
    g.t.select_element(a);
    let out = g.t.take_outcome();
    assert!(out.dirty.overlay);
    assert!(out.dirty.grid_rects.is_empty(), "{:?}", out.dirty.grid_rects);
    assert!(!out.dirty.grid_all);
    let cover = IRect::new(0, 0, 1000, 400);
    assert_eq!(g.t.overlay_occurrences(cover).iter().map(|q| q.id).collect::<Vec<_>>(), [a]);
}

#[test]
fn a_moving_effect_leaves_the_tiles_for_its_own_layer_and_comes_back() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    let a = g.add(1, 0, 10.0, 20.0);
    let rect = IRect::new(0, 0, 1000, 400);
    g.hover(150, 8, Mods::NONE);
    g.down(150, 8, Mods::NONE);
    assert_eq!(g.t.grid_frame(rect).elements.len(), 1);
    g.drag(158, 8, Mods::NONE);
    assert_eq!(g.t.grid_frame(rect).elements.len(), 0);
    let lifted = g.t.overlay_occurrences(rect);
    assert_eq!(lifted.len(), 1);
    assert!(lifted[0].moving && lifted[0].selected);
    // A move repaints nothing beneath.
    let out = g.drag(168, 8, Mods::NONE);
    assert!(out.dirty.grid_rects.is_empty(), "{:?}", out.dirty.grid_rects);
    assert!(out.dirty.overlay);
    assert_eq!(g.t.overlay_occurrences(rect)[0].x, 110);
    let out = g.up(168, 8, Mods::NONE);
    assert!(!out.dirty.grid_rects.is_empty());
    let frame = g.t.grid_frame(rect);
    assert_eq!(frame.elements.len(), 1);
    assert_eq!(frame.elements[0].x, 110);
    assert_eq!(frame.elements[0].id, a);
}

#[test]
fn a_drag_that_changes_a_neighbours_lane_repaints_that_neighbour() {
    let mut g = Grid::flat();
    g.t.opts.snap_enabled = false;
    g.add(1, 0, 10.0, 20.0);
    g.add(2, 0, 30.0, 40.0);
    g.hover(350, 8, Mods::NONE);
    g.down(350, 8, Mods::NONE);
    g.drag(358, 8, Mods::NONE);
    g.t.take_outcome();
    let out = g.drag(358 - 150, 8, Mods::NONE);
    assert!(!out.dirty.grid_rects.is_empty(), "the stationary one is now in a lane");
}

#[test]
fn undo_through_the_session_shows_up_on_the_next_sync() {
    use crate::sequence::{SequenceSession, SessionEnvironment};
    use vixen_io::ModuleStore;
    const SET_LEVEL: Uuid = uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0");
    let mut tree = ElementTree::new();
    let a = tree.add_element("A");
    let b = tree.add_element("B");
    let (ua, ub) = (tree.node(a).id, tree.node(b).id);
    let env = SessionEnvironment { video_cache: None,
        tree: Arc::new(tree),
        store: Arc::new(ModuleStore::new()),
        module_data_root: None,
        frame_ms: 25,
        media_dir: std::env::temp_dir(),
    };
    let mut session = SequenceSession::new_untitled(&env);
    let id = session.add_new_effect(SET_LEVEL, ua, "A", sec(1.0), sec(2.0)).unwrap();
    session.wait_for_renders();
    let mut t = Timeline::new();
    t.load_from_session(&session, Vec::new());
    t.set_viewport(1000, 400, 1000);
    t.opts.snap_enabled = false;
    t.take_outcome();
    assert_eq!(t.elements.len(), 1);
    assert!(t.elements[&id].rendered);

    // Drag it down a row and along, for real.
    let press = |x, y, button, mods| PointerEvent::new(x, y, button, mods);
    let x = px_of(sec(2.0), &t);
    t.grid_mouse_move(press(x, 5, None, Mods::NONE), &mut session);
    t.grid_mouse_down(press(x, 5, Some(Button::Left), Mods::NONE));
    t.grid_mouse_move(press(x + 8, 5, Some(Button::Left), Mods::NONE), &mut session);
    t.grid_mouse_move(press(x + 28, 37, Some(Button::Left), Mods::NONE), &mut session);
    t.grid_mouse_up(press(x + 28, 37, Some(Button::Left), Mods::NONE), &mut session);
    let entry = session.effect(id).unwrap();
    assert_eq!(entry.node.start, t.elements[&id].start);
    assert_eq!(entry.node.targets[0].0, ub);
    assert_eq!(session.undo_stack().undo_descriptions()[0], "Moved 1 effect");
    let moved_to = t.elements[&id].start;
    assert!(moved_to > sec(1.0));
    assert!(t.rows.row(RowId(1)).occurrences.iter().any(|o| o.id == id));

    // Undo in the session, then sync.
    session.undo(1);
    assert!(t.sync_from_session(&session));
    assert_eq!(t.elements[&id].start, sec(1.0));
    assert!(t.rows.row(RowId(0)).occurrences.iter().any(|o| o.id == id));
    assert!(t.rows.row(RowId(1)).occurrences.is_empty());
    assert_eq!(session.effect(id).unwrap().node.targets[0].0, ua);

    session.redo(1);
    t.sync_from_session(&session);
    assert_eq!(t.elements[&id].start, moved_to);
    assert_eq!(session.effect(id).unwrap().node.targets[0].0, ub);
    assert!(t.rows.row(RowId(1)).occurrences.iter().any(|o| o.id == id));

    // Removed there, gone here.
    session.remove_effects(&[id]);
    t.sync_from_session(&session);
    assert!(t.elements.is_empty());
    assert!(t.rows.row(RowId(1)).occurrences.is_empty());
    let _ = Color::RED;
}

fn px_of(t: TimeSpan, timeline: &Timeline) -> i32 {
    super::view::px(t, timeline.tpp()) as i32
}

#[test]
fn view_state_round_trips_through_the_document() {
    use crate::sequence::{SequenceSession, SessionEnvironment};
    use vixen_io::ModuleStore;
    let mut tree = ElementTree::new();
    tree.add_element("A");
    tree.add_element("B");
    let env = SessionEnvironment { video_cache: None,
        tree: Arc::new(tree),
        store: Arc::new(ModuleStore::new()),
        module_data_root: None,
        frame_ms: 25,
        media_dir: std::env::temp_dir(),
    };
    let mut session = SequenceSession::new_untitled(&env);
    session.set_length(sec(100.0));
    let mut t = Timeline::new();
    t.load_from_session(&session, Vec::new());
    t.set_viewport(400, 100, 400);
    t.set_time_per_pixel(TimeSpan::from_ms_i(50));
    t.set_visible_start(sec(10.0));
    t.info.playback_start = Some(sec(2.0));
    t.info.playback_end = Some(sec(4.0));
    t.rows.row_mut(RowId(1)).height = 90;
    t.store_view_state(&mut session);
    let mut again = Timeline::new();
    again.load_from_session(&session, Vec::new());
    again.set_viewport(400, 100, 400);
    again.apply_pending_view();
    assert_eq!(again.tpp(), TimeSpan::from_ms_i(50));
    assert_eq!(again.info.visible_start, sec(10.0));
    assert_eq!(again.info.playback_end, Some(sec(4.0)));
    assert_eq!(again.rows.row(RowId(1)).height, 90);
    assert_eq!(again.rows.row(RowId(0)).height, 32);
}

// ----------------------------------------------------------------------------------------------------------------------
// The budget: 50,000 rows, 200,000 effects
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn the_budget_holds_at_fifty_thousand_rows_and_two_hundred_thousand_effects() {
    // Timings need optimised code. A debug build still checks it runs and agrees.
    let mut tree = ElementTree::new();
    let mut nodes = Vec::with_capacity(50_000);
    for i in 0..50_000 {
        nodes.push(tree.add_element(&format!("Row {i}")));
    }
    let mut t = Timeline::new();
    t.info.total = sec(3600.0);
    t.rows.load_rows(&tree, &[], 0);
    t.set_viewport(1500, 1000, 1500);
    for i in 0..200_000u64 {
        let row = (i % 50_000) as usize;
        let start = (i / 50_000) as f64 * 30.0 + (row % 17) as f64;
        t.elements.insert(
            e(i + 1),
            model::ElementState {
                id: e(i + 1),
                start: sec(start),
                duration: sec(10.0),
                last_row: Some(RowId(row as u32)),
                rows: vec![RowId(row as u32)],
                selected: false,
                desc: ElementDescription::default(),
                target: Some(nodes[row]),
                rendered: true,
                picture: 1,
                rendered_duration: sec(10.0),
            },
        );
        t.rows.row_mut(RowId(row as u32)).occurrences.push(rows::Occ {
            id: e(i + 1),
            start: sec(start).ticks(),
            end: sec(start + 10.0).ticks(),
            index: 0,
            count: 0,
            moving: false,
        });
    }
    for r in &mut t.rows.rows {
        r.occurrences.sort_by_key(|o| (o.start, o.end));
        r.rebuild_derived();
    }
    t.rows_changed();
    t.take_outcome();

    // Tiles along a scroll.
    let mut host = RecordingHost::default();
    let mut worst_frame = std::time::Duration::ZERO;
    for i in 0..200 {
        let rect = IRect::new(i * 500, i * 300, 1500, 1000);
        let started = std::time::Instant::now();
        let frame = t.grid_frame(rect);
        worst_frame = worst_frame.max(started.elapsed());
        assert!(!frame.rows.is_empty());
    }

    // A thousand effects dragging.
    let ids: Vec<ElementId> = (0..1000u64).map(|i| e(i * 4 + 1)).collect();
    t.select_elements(&ids);
    t.take_outcome();
    let p = t.rows.row_top(RowId(0)).map(|top| (5, top + 5)).unwrap();
    t.set_cursor(TimeSpan::ZERO);
    let times_of_first = t.elements[&ids[0]].start;
    t.grid_mouse_move(PointerEvent::new(p.0, p.1, None, Mods::NONE), &mut host);
    t.grid_mouse_down(PointerEvent::new(p.0 + 40, p.1, Some(Button::Left), Mods::NONE));
    t.grid_mouse_move(PointerEvent::new(p.0 + 40 + 8, p.1, Some(Button::Left), Mods::NONE), &mut host);
    let mut worst_move = std::time::Duration::ZERO;
    for i in 0..50 {
        let ev = PointerEvent::new(p.0 + 40 + 8 + i, p.1, Some(Button::Left), Mods::NONE);
        let started = std::time::Instant::now();
        t.grid_mouse_move(ev, &mut host);
        worst_move = worst_move.max(started.elapsed());
    }
    assert_ne!(t.elements[&ids[0]].start, times_of_first);
    println!("worst grid_frame {worst_frame:?}, worst grid_mouse_move during a 1,000-effect move {worst_move:?}");
    if !cfg!(debug_assertions) {
        assert!(worst_frame < std::time::Duration::from_millis(2), "{worst_frame:?}");
        assert!(worst_move < std::time::Duration::from_millis(2), "{worst_move:?}");
    }
}

// ----------------------------------------------------------------------------------------------------------------------
// MarksBarMouseWheelTests.cs, TimelineZoomTests.cs
// ----------------------------------------------------------------------------------------------------------------------

/// `MarksBarMouseWheelTests`' control: ten minutes at 100 ms a pixel, so a 100 second view.
fn ten_minutes() -> Grid {
    let mut g = Grid::flat();
    g.t.set_total(sec(600.0));
    g
}

fn bar_wheel(g: &mut Grid, lines: f64, mods: Mods) {
    g.t.scroll_wheel(ScrollEvent { surface: Surface::MarksBar, dx: 0.0, dy: lines, precise: false, mods, pointer_offset_x: 400, grid_y: 0 });
}

const SHIFT_OPT: Mods = Mods { shift: true, cmd: false, opt: true };
const SHIFT_CMD: Mods = Mods { shift: true, cmd: true, opt: false };
const CMD_OPT: Mods = Mods { shift: false, cmd: true, opt: true };

#[test]
fn marks_bar_wheel_pans_ten_percent_with_shift_held_whatever_else_is() {
    let mut g = ten_minutes();
    g.t.set_visible_start(sec(50.0));
    bar_wheel(&mut g, -1.0, Mods::SHIFT);
    assert_eq!(g.t.info.visible_start, sec(60.0));
    bar_wheel(&mut g, 1.0, Mods::SHIFT);
    assert_eq!(g.t.info.visible_start, sec(50.0));
    bar_wheel(&mut g, -1.0, SHIFT_OPT);
    assert_eq!(g.t.info.visible_start, sec(60.0));
    bar_wheel(&mut g, -1.0, SHIFT_CMD);
    assert_eq!(g.t.info.visible_start, sec(70.0));
    // Part of a notch pans in proportion.
    bar_wheel(&mut g, -0.25, Mods::SHIFT);
    assert_eq!(g.t.info.visible_start, sec(72.5));
}

#[test]
fn marks_bar_wheel_zooms_with_command_and_otherwise_does_nothing() {
    let mut g = ten_minutes();
    g.t.set_visible_start(sec(10.0));
    bar_wheel(&mut g, 0.0 - 1.0, Mods::NONE);
    bar_wheel(&mut g, -1.0, CMD_OPT);
    assert_eq!((g.t.info.visible_start, g.t.tpp()), (sec(10.0), TimeSpan::from_ms_i(100)));
    bar_wheel(&mut g, -1.0, Mods::CMD);
    assert_eq!(g.t.tpp(), TimeSpan::from_ms_i(110));
}

#[test]
fn marks_bar_wheel_clamps_at_both_ends() {
    let mut g = ten_minutes();
    g.t.set_visible_start(sec(5.0));
    bar_wheel(&mut g, 1.0, Mods::SHIFT);
    assert_eq!(g.t.info.visible_start, TimeSpan::ZERO);
    g.t.set_visible_start(sec(495.0));
    bar_wheel(&mut g, -1.0, Mods::SHIFT);
    assert_eq!(g.t.info.visible_start, sec(500.0));
}

#[test]
fn zooming_in_past_the_floor_changes_nothing_and_out_past_the_total_shows_it_all() {
    let mut g = ten_minutes();
    g.t.set_time_per_pixel(TimeSpan(2222));
    g.t.set_visible_start(sec(1.0));
    assert!(!g.t.zoom_time_at(400, 0.9));
    assert_eq!((g.t.tpp(), g.t.info.visible_start), (TimeSpan(2222), sec(1.0)));
    // Toolbar and pointer forms agree.
    let mut toolbar = ten_minutes();
    let mut pointer = ten_minutes();
    toolbar.t.set_time_per_pixel(sec(1.0));
    pointer.t.set_time_per_pixel(sec(1.0));
    assert!(toolbar.t.zoom_time(1.1));
    assert!(pointer.t.zoom_time_at(400, 1.1));
    assert_eq!(toolbar.t.tpp(), pointer.t.tpp());
    assert_eq!(pointer.t.info.visible_start, TimeSpan::ZERO);
}

// ----------------------------------------------------------------------------------------------------------------------
// Rows
// ----------------------------------------------------------------------------------------------------------------------

/// A group with two children, then a plain row: `RowId`s 0, 1, 2 and 3, group open.
fn group_rows() -> Grid {
    let mut tree = ElementTree::new();
    let group = tree.add_node(Uuid::new_v4(), "Group", None);
    let (a, b) = (tree.add_element("A"), tree.add_element("B"));
    tree.add_child(group, a);
    tree.add_child(group, b);
    let c = tree.add_element("C");
    let mut t = Timeline::new();
    t.info.time_per_pixel = TimeSpan::from_ms_i(100);
    t.info.total = sec(60.0);
    t.rows.load_rows(&tree, &[], 32);
    t.rows.set_tree_open(RowId(0), true);
    t.rows.relayout();
    t.set_viewport(1000, 400, 1000);
    Grid { t, host: RecordingHost::default(), nodes: vec![a, b, c, group] }
}

#[test]
fn move_active_row_with_no_rows_is_false() {
    let mut t = Timeline::new();
    t.set_viewport(100, 100, 100);
    assert!(!t.move_active_row(1));
}

#[test]
fn a_fast_double_click_on_an_expander_or_a_label_acts_once() {
    let mut g = group_rows();
    let press = |clicks: u8, x: i32| PointerEvent { x, y: 5, button: Some(Button::Left), clicks, mods: Mods::NONE };
    let open = g.t.rows.row(RowId(0)).tree_open;
    g.t.row_mouse_down(press(1, 5), 200);
    g.t.row_mouse_up(press(1, 5), 200);
    assert_eq!(g.t.rows.row(RowId(0)).tree_open, !open);
    g.t.row_mouse_down(press(2, 5), 200);
    g.t.row_mouse_up(press(2, 5), 200);
    assert_eq!(g.t.rows.row(RowId(0)).tree_open, !open);
    // The second press on a label doesn't select either: here it would add nothing, so check it runs no click at all.
    g.t.row_mouse_down(press(1, 100), 200);
    g.t.row_mouse_up(press(1, 100), 200);
    assert!(g.t.rows.row(RowId(0)).selected);
    g.t.rows.row_mut(RowId(0)).selected = false;
    g.t.row_mouse_down(press(2, 100), 200);
    g.t.row_mouse_up(press(2, 100), 200);
    assert!(!g.t.rows.row(RowId(0)).selected);
}

#[test]
fn the_default_row_height_has_no_floor_and_sticks_at_zero_q36() {
    let mut g = Grid::flat();
    g.t.rows.reset_row_heights();
    let mut seen = Vec::new();
    for _ in 0..12 {
        g.t.zoom_rows(0.8);
        seen.push(g.t.rows.row_height);
    }
    assert_eq!(seen, [25, 20, 16, 12, 9, 7, 5, 4, 3, 2, 1, 0]);
    assert_eq!(g.t.rows.row(RowId(0)).height, 10);
    g.t.zoom_rows(1.25);
    assert_eq!(g.t.rows.row_height, 0);
    g.t.rows.reset_row_heights();
    assert_eq!(g.t.rows.row_height, 32);
}

#[test]
fn paste_and_add_effect_look_for_their_rows_in_different_orders_f27() {
    let mut g = Grid::flat();
    assert_eq!(g.t.paste_target_row(), Some(RowId(0)));
    assert_eq!(g.t.add_effect_target_row(), None);
    g.t.set_row_selected(RowId(1), true);
    g.t.set_row_active(RowId(2), true);
    assert_eq!(g.t.paste_target_row(), Some(RowId(1)));
    assert_eq!(g.t.add_effect_target_row(), Some(RowId(2)));
    g.t.set_row_active(RowId(2), false);
    assert_eq!(g.t.add_effect_target_row(), Some(RowId(1)));
}

#[test]
fn a_shift_click_on_a_row_ranges_from_the_old_first_row_q37() {
    let mut g = Grid::flat();
    let a = g.add(1, 3, 30.0, 31.0);
    g.t.click_row_label(RowId(1), Mods::NONE);
    // An effect click clears the row selection, and not the first selected row.
    g.click(tab(3) * 0 + 305, tab(3), Mods::NONE);
    assert_eq!(g.selected(), [a]);
    assert!(!g.t.rows.row(RowId(1)).selected);
    g.t.click_row_label(RowId(2), Mods::SHIFT);
    assert!(g.t.rows.row(RowId(1)).selected && g.t.rows.row(RowId(2)).selected);
}

#[test]
fn a_shift_range_of_rows_runs_either_way_over_rows_visible_then() {
    let mut g = Grid::flat();
    g.t.rows.set_visible(RowId(1), false);
    g.t.rows.relayout();
    g.t.click_row_label(RowId(3), Mods::NONE);
    g.t.click_row_label(RowId(0), Mods::SHIFT);
    let selected: Vec<bool> = (0..4).map(|r| g.t.rows.row(RowId(r)).selected).collect();
    assert_eq!(selected, [true, false, true, true]);
}

#[test]
fn rows_shorter_than_the_view_keep_the_offset_at_zero() {
    let mut g = Grid::flat();
    g.t.set_vertical_offset(500);
    assert_eq!(g.t.vertical_offset, 0);
}

// ----------------------------------------------------------------------------------------------------------------------
// Selection and cursor: TimelineCursorSelectionTests.cs and the press table
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn a_band_in_legacy_mode_finalizes_nothing() {
    let mut g = Grid::flat();
    g.add(1, 1, 5.0, 7.0);
    g.t.opts.legacy_cursor = true;
    g.t.set_cursor(sec(3.0));
    g.t.set_row_active(RowId(3), true);
    g.down(30, tab(1), Mods::NONE);
    // Starting one clears active rows at once in legacy mode.
    assert_eq!(g.t.active_row(), None);
    g.drag(90, tab(1) + 10, Mods::NONE);
    g.up(90, tab(1) + 10, Mods::NONE);
    assert_eq!(g.selected().len(), 1);
    assert_eq!(g.t.active_row(), None);
    assert_eq!(g.t.cursor, sec(3.0));
}

#[test]
fn a_real_band_clears_other_active_rows_and_activates_the_press_row() {
    let mut g = Grid::flat();
    let later = g.add(1, 1, 14.0, 16.0);
    let earlier = g.add(2, 0, 5.0, 7.0);
    g.t.set_row_active(RowId(2), true);
    g.down(30, tab(1), Mods::NONE);
    g.drag(160, 2, Mods::NONE);
    g.up(160, 2, Mods::NONE);
    assert!(g.selected().contains(&later) && g.selected().contains(&earlier));
    assert!(g.t.rows.row(RowId(1)).active);
    assert!(!g.t.rows.row(RowId(2)).active);
    assert_eq!(g.t.cursor, sec(5.0));
}

#[test]
fn a_right_press_on_blank_sets_the_anchor_and_active_row_and_the_release_moves_the_cursor_f54() {
    let mut g = Grid::flat();
    g.t.set_cursor(sec(1.0));
    g.right_down(100, tab(1), Mods::NONE);
    assert_eq!(g.t.g.last_single_selected, (100, tab(1)));
    assert_eq!(g.t.active_row(), Some(RowId(1)));
    assert_eq!(g.t.cursor, sec(1.0));
    let out = g.right_up(120, tab(2), Mods::NONE);
    assert_eq!(g.t.cursor, sec(12.0));
    assert_eq!(g.t.active_row(), Some(RowId(2)));
    // The time and row given to the menu are the release point's.
    assert_eq!(out.requests, [HostRequest::ContextMenu { elements: vec![], time: sec(12.0), row: RowId(2) }]);
}

/// Two effects in one lane at the same place, so a press hits both.
fn overlapping() -> (Grid, ElementId, ElementId) {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 3.0);
    let b = g.add(2, 0, 1.0, 3.0);
    g.t.refresh_view_stacks();
    g.t.rows.row_mut(RowId(0)).occurrences.iter_mut().for_each(|o| {
        o.index = 0;
        o.count = 1;
    });
    g.t.rows.row_mut(RowId(0)).walk_key = Some((0, 2));
    (g, a, b)
}

#[test]
fn the_overlap_menu_comes_once_on_a_left_press_q47() {
    let (mut g, a, b) = overlapping();
    let press = g.down(20, 5, Mods::NONE);
    assert_eq!(press.requests, [HostRequest::ChooseOverlapping { elements: vec![a, b], at: (20, 5) }]);
    assert_eq!(g.t.drag_state(), DragState::Normal);
    // Should a release come after the menu, there's no second offer and no stranded drag.
    let release = g.up(20, 5, Mods::NONE);
    assert!(release.requests.is_empty());
    assert_eq!(g.t.drag_state(), DragState::Normal);
}

#[test]
fn a_pick_from_the_overlap_menu_moves_the_cursor_and_sets_no_anchor_or_active_row_f50() {
    let (mut g, _, b) = overlapping();
    g.down(20, 5, Mods::NONE);
    let out = g.t.finish_chooser(Some(b));
    assert_eq!(g.selected(), [b]);
    assert!(out.cursor_moved);
    assert_eq!(g.t.cursor, sec(1.0));
    assert_eq!(g.t.g.last_single_selected, (0, 0));
    assert_eq!(g.t.active_row(), None);
    // Dismissing it leaves the selection empty.
    let (mut g, _, _) = overlapping();
    g.down(20, 5, Mods::NONE);
    g.t.finish_chooser(None);
    assert!(g.selected().is_empty());
}

#[test]
fn with_command_held_a_pick_from_the_overlap_menu_adds_q47() {
    let (mut g, _, b) = overlapping();
    let other = g.add(3, 1, 20.0, 21.0);
    g.t.select_elements(&[other]);
    g.down(20, 5, Mods::CMD);
    g.t.finish_chooser(Some(b));
    let selected = g.selected();
    assert!(selected.contains(&other) && selected.contains(&b));
}

#[test]
fn a_right_press_skips_the_overlap_menu_and_goes_to_the_context_menu_q47() {
    let (mut g, a, b) = overlapping();
    let other = g.add(3, 1, 20.0, 21.0);
    g.t.select_elements(&[other]);
    let press = g.right_down(20, 5, Mods::NONE);
    assert!(press.requests.is_empty());
    assert!(g.selected().is_empty());
    let release = g.right_up(20, 5, Mods::NONE);
    assert_eq!(release.requests, [HostRequest::ContextMenu { elements: vec![a, b], time: sec(2.0), row: RowId(0) }]);
}

#[test]
fn a_release_collapse_keeps_the_old_active_row_beside_the_new_f49() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    let b = g.add(2, 1, 10.0, 12.0);
    g.t.select_elements(&[a, b]);
    g.t.set_row_active(RowId(1), true);
    g.down(110, tab(0), Mods::NONE);
    assert_eq!(g.selected().len(), 2);
    g.up(110, tab(0), Mods::NONE);
    assert_eq!(g.selected(), [a]);
    assert!(g.t.rows.row(RowId(0)).active && g.t.rows.row(RowId(1)).active);
}

#[test]
fn a_command_press_deselects_with_no_cursor_move_and_the_release_moves_it_anyway_f92() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    g.t.select_elements(&[a]);
    g.t.set_cursor(sec(3.0));
    g.down(110, tab(0), Mods::CMD);
    assert!(g.selected().is_empty());
    assert_eq!(g.t.cursor, sec(3.0));
    g.up(110, tab(0), Mods::CMD);
    assert!(g.selected().is_empty());
    assert_eq!(g.t.cursor, sec(10.0));
}

#[test]
fn the_cursor_raises_its_event_even_to_the_same_value_f91() {
    let mut g = Grid::flat();
    g.t.set_cursor(sec(3.0));
    g.t.take_outcome();
    g.t.set_cursor(sec(3.0));
    assert!(g.t.take_outcome().cursor_moved);
}

#[test]
fn the_hover_set_stays_when_the_pointer_leaves_and_loses_a_deleted_effect_q33() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    g.hover(110, tab(0), Mods::NONE);
    assert!(g.t.hover_info().is_some());
    // Nothing tells the grid the pointer has gone, so this is all there is.
    assert!(g.t.hover_info().is_some());
    g.t.remove_element(a);
    assert!(g.t.hover_info().is_none());
}

#[test]
fn hover_text_has_its_exact_form_f45() {
    let text = super::model::hover_info_text("Pulse", "Default", ElementTimes::new(sec(61.5), sec(2.0)));
    assert_eq!(text, "Pulse \r\n Layer: Default \r\n Start: 1:01.500 \r\n End: 1:03.500 \r\n Length: 0:02.000");
}

#[test]
fn cad_is_enclosed_and_green_rightward_crossing_and_blue_leftward_f52() {
    let mut g = Grid::flat();
    let a = g.add(1, 1, 10.0, 20.0);
    g.t.opts.cad_style = true;
    // Rightward, over only part of it: enclosed misses.
    g.down(150, 5, Mods::NONE);
    g.drag(250, 60, Mods::NONE);
    assert!(g.selected().is_empty());
    assert!(g.t.overlay_selection_green());
    g.drag(250, 60, Mods::NONE);
    g.up(250, 60, Mods::NONE);
    // Rightward, around all of it.
    g.t.clear_selection();
    g.down(50, 5, Mods::NONE);
    g.drag(250, 60, Mods::NONE);
    assert_eq!(g.selected(), [a]);
    g.up(250, 60, Mods::NONE);
    // Leftward, over part of it: crossing takes it.
    g.t.clear_selection();
    g.down(250, 60, Mods::NONE);
    g.drag(150, 5, Mods::NONE);
    assert_eq!(g.selected(), [a]);
    assert!(!g.t.overlay_selection_green());
    g.up(150, 5, Mods::NONE);
}

#[test]
fn a_press_with_the_indicator_on_publishes_guides_from_the_press_q14() {
    let mut g = Grid::flat();
    g.add(1, 0, 10.0, 20.0);
    g.t.opts.resize_indicator = true;
    g.hover(150, tab(0), Mods::NONE);
    assert!(g.t.guides().is_empty());
    g.down(150, tab(0), Mods::NONE);
    assert_eq!(g.t.guides(), [sec(10.0), sec(20.0)]);
    g.up(150, tab(0), Mods::NONE);
    assert!(g.t.guides().is_empty());
}

// ----------------------------------------------------------------------------------------------------------------------
// Move and resize
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn a_drag_with_nothing_selected_moves_nothing_and_records_moved_zero_q35() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    g.t.select_elements(&[a]);
    // ⌘ deselects the only selected effect, and the key comes up before the threshold.
    g.down(110, tab(0), Mods::CMD);
    g.drag(130, tab(0), Mods::NONE);
    g.drag(160, tab(0), Mods::NONE);
    assert_eq!(g.times(a), (sec(10.0), sec(12.0)));
    g.up(160, tab(0), Mods::NONE);
    assert_eq!(g.host.undo.last().unwrap().description(), "Moved 0 effects");
}

#[test]
fn the_sequence_is_modified_from_a_drags_first_move_f58() {
    let mut g = Grid::flat();
    g.add(1, 0, 10.0, 12.0);
    g.hover(110, tab(0), Mods::NONE);
    g.down(110, tab(0), Mods::NONE);
    g.drag(120, tab(0), Mods::NONE);
    assert!(!g.host.modified);
    g.drag(130, tab(0), Mods::NONE);
    assert!(g.host.modified);
    // And it stays that way if the drag is cancelled.
    g.t.grid_cancel_drag();
    assert!(g.host.modified);
    assert!(g.host.undo.is_empty());
}

#[test]
fn a_threshold_crossed_below_the_last_row_starts_tracking_on_the_way_back_q38() {
    let mut g = Grid::flat();
    let a = g.add(1, 1, 10.0, 12.0);
    g.down(110, tab(1), Mods::NONE);
    g.drag(110, 300, Mods::SHIFT);
    assert_eq!(g.t.drag_state(), DragState::Moving);
    assert_eq!(g.t.g.current_row_index, -1);
    // Back onto a row: nothing moves this time, and tracking starts there.
    g.drag(110, tab(3), Mods::SHIFT);
    assert!(g.in_row(1, a));
    assert_eq!(g.t.g.current_row_index, 3);
    g.drag(110, tab(2), Mods::SHIFT);
    assert!(g.in_row(0, a));
}

#[test]
fn with_no_moving_effect_in_a_visible_row_an_upward_move_does_nothing_q39() {
    let mut g = Grid::flat();
    let a = g.add(1, 1, 10.0, 12.0);
    g.t.rows.set_visible(RowId(1), false);
    g.t.rows.relayout();
    g.t.g.current_row_index = 2;
    g.t.g.mouse_down_row = Some(RowId(2));
    g.t.move_elements_vertically(&[a], (110, 5));
    assert!(g.in_row(1, a));
    assert_eq!(g.t.g.current_row_index, 2);
}

#[test]
fn a_click_in_a_zone_records_a_resize_and_keeps_a_multi_selection_q34() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 20.0);
    let b = g.add(2, 1, 30.0, 40.0);
    g.t.select_elements(&[a, b]);
    g.hover(102, tab(0), Mods::NONE);
    g.down(102, tab(0), Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::HResizing);
    g.up(102, tab(0), Mods::NONE);
    assert_eq!(g.host.undo.len(), 1);
    assert_eq!(g.host.undo[0].description(), "Resize 2 effects");
    assert_eq!(g.times(a), (sec(10.0), sec(20.0)));
    assert_eq!(g.selected().len(), 2);
}

#[test]
fn any_motion_widens_a_narrow_effect_to_ten_pixels_even_past_the_end_f66() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 59.5, 60.0);
    g.hover(600, tab(0), Mods::NONE);
    g.down(600, tab(0), Mods::NONE);
    g.drag(601, tab(0), Mods::NONE);
    assert_eq!(g.times(a), (sec(59.5), sec(60.5)));
}

#[test]
fn the_minimum_width_is_ten_pixels_of_the_zoom_f66() {
    let mut g = Grid::flat();
    g.t.set_time_per_pixel(TimeSpan::from_ms_i(10));
    let a = g.add(1, 0, 1.0, 1.5);
    g.hover(101, tab(0), Mods::NONE);
    g.down(101, tab(0), Mods::NONE);
    g.drag(146, tab(0), Mods::NONE);
    assert_eq!(g.times(a), (sec(1.4), sec(1.5)));
    g.up(146, tab(0), Mods::NONE);
    let b = g.add(2, 1, 1.0, 1.5);
    g.t.clear_selection();
    g.hover(149, tab(1), Mods::NONE);
    g.down(149, tab(1), Mods::NONE);
    g.drag(104, tab(1), Mods::NONE);
    assert_eq!(g.times(b), (sec(1.0), sec(1.1)));
}

#[test]
fn esc_does_nothing_to_a_band_or_a_draw_box_and_works_for_a_resize_with_a_neighbour_f64() {
    let mut g = Grid::flat();
    g.down(30, tab(1), Mods::NONE);
    g.drag(90, tab(2), Mods::NONE);
    g.t.grid_cancel_drag();
    assert_eq!(g.t.drag_state(), DragState::Selecting);
    g.up(90, tab(2), Mods::NONE);

    g.t.opts.draw_mode = true;
    g.t.opts.selected_effect = Some(TYPE_A);
    g.down(30, tab(1), Mods::NONE);
    g.drag(90, tab(2), Mods::NONE);
    g.t.grid_cancel_drag();
    assert_eq!(g.t.drag_state(), DragState::Drawing);
    g.up(90, tab(2), Mods::NONE);
    g.t.opts.draw_mode = false;
    g.t.opts.selected_effect = None;

    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 20.0);
    let b = g.add(2, 0, 20.0, 30.0);
    g.hover(198, tab(0), Mods::NONE);
    g.down(198, tab(0), Mods::OPT);
    g.drag(210, tab(0), Mods::OPT);
    assert_eq!(g.times(a).1, sec(21.2));
    assert_eq!(g.times(b).0, sec(21.2));
    g.t.grid_cancel_drag();
    assert_eq!(g.times(a), (sec(10.0), sec(20.0)));
    assert_eq!(g.times(b), (sec(20.0), sec(30.0)));
    assert!(g.t.g.drag_snap.is_none());
    assert!(g.host.undo.is_empty());
}

#[test]
fn draw_times_are_clamped_to_the_sequence_q40() {
    use super::grid_input::clamp_draw_times;
    assert_eq!(clamp_draw_times(sec(-1.0), sec(70.0), sec(60.0)), (TimeSpan::ZERO, sec(60.0)));
    assert_eq!(clamp_draw_times(sec(70.0), sec(-1.0), sec(60.0)), (TimeSpan::ZERO, sec(60.0)));
    assert_eq!(clamp_draw_times(sec(5.0), sec(2.0), sec(60.0)), (sec(2.0), sec(3.0)));

    let mut g = Grid::flat();
    g.t.opts.draw_mode = true;
    g.t.opts.selected_effect = Some(TYPE_A);
    g.down(5, tab(0), Mods::NONE);
    g.drag(-50, tab(0), Mods::NONE);
    g.up(-50, tab(0), Mods::NONE);
    assert_eq!(g.host.added.len(), 1);
    assert_eq!((g.host.added[0].2, g.host.added[0].3), (TimeSpan::ZERO, sec(0.5)));
}

#[test]
fn the_pointer_in_each_grid_state_f150() {
    let mut g = Grid::flat();
    g.add(1, 0, 10.0, 20.0);
    // Normal: recomputed on every move.
    assert_eq!(g.hover(102, tab(0), Mods::NONE).cursor, CursorKind::ResizeLeftRight);
    assert_eq!(g.hover(150, tab(0), Mods::NONE).cursor, CursorKind::Arrow);
    g.t.opts.draw_mode = true;
    g.t.opts.selected_effect = Some(TYPE_A);
    assert_eq!(g.hover(50, tab(1), Mods::NONE).cursor, CursorKind::Crosshair);
    assert_eq!(g.hover(50, tab(1), Mods::OPT).cursor, CursorKind::Arrow);
    // Drawing from Draw Mode leaves it, so crosshair, and any drag ended is an arrow until the next move.
    g.hover(50, tab(1), Mods::NONE);
    assert_eq!(g.down(50, tab(1), Mods::NONE).cursor, CursorKind::Crosshair);
    assert_eq!(g.drag(90, tab(1), Mods::NONE).cursor, CursorKind::Crosshair);
    assert_eq!(g.up(90, tab(1), Mods::NONE).cursor, CursorKind::Arrow);
    // An effect type deselected is an arrow.
    g.hover(50, tab(1), Mods::NONE);
    g.t.set_selected_effect(None);
    assert_eq!(g.t.take_outcome().cursor, CursorKind::Arrow);
    g.t.opts.draw_mode = false;
    // Drawing from the middle button is a crosshair, set at press, and an arrow after.
    g.t.opts.selected_effect = Some(TYPE_A);
    let middle = PointerEvent::new(50, tab(1), Some(Button::Middle), Mods::NONE);
    assert_eq!(g.t.grid_mouse_down(middle).cursor, CursorKind::Crosshair);
    assert_eq!(g.t.grid_mouse_up(middle, &mut g.host).cursor, CursorKind::Arrow);
    g.t.opts.selected_effect = None;
    // Waiting leaves what the hover set, and moving is the move pointer from the threshold on.
    g.hover(150, tab(0), Mods::NONE);
    assert_eq!(g.down(150, tab(0), Mods::NONE).cursor, CursorKind::Arrow);
    assert_eq!(g.drag(152, tab(0), Mods::NONE).cursor, CursorKind::Arrow);
    assert_eq!(g.drag(170, tab(0), Mods::NONE).cursor, CursorKind::Move);
    assert_eq!(g.up(170, tab(0), Mods::NONE).cursor, CursorKind::Arrow);
    // Resizing keeps the left-right pointer.
    g.t.clear_selection();
    let x = (g.t.elements[&e(1)].start.ticks() / 1_000_000) as i32 + 2;
    assert_eq!(g.hover(x, tab(0), Mods::NONE).cursor, CursorKind::ResizeLeftRight);
    assert_eq!(g.down(x, tab(0), Mods::NONE).cursor, CursorKind::ResizeLeftRight);
    assert_eq!(g.drag(x + 5, tab(0), Mods::NONE).cursor, CursorKind::ResizeLeftRight);
    g.up(x + 5, tab(0), Mods::NONE);
    // Selecting leaves it too.
    g.hover(50, tab(3), Mods::NONE);
    assert_eq!(g.down(50, tab(3), Mods::NONE).cursor, CursorKind::Arrow);
    assert_eq!(g.drag(90, tab(2), Mods::NONE).cursor, CursorKind::Arrow);
}

#[test]
fn an_effect_with_two_targets_shows_in_both_and_a_clone_takes_the_last_rows_node_f41() {
    let mut g = Grid::flat();
    let (a, b) = (g.nodes[0], g.nodes[2]);
    g.t.add_element_to_targets(e(1), ElementTimes::new(sec(10.0), sec(2.0)), &[a, b], desc(TYPE_A, 0));
    g.t.flush_rows();
    assert_eq!(g.t.elements[&e(1)].rows, [RowId(0), RowId(2)]);
    g.t.select_elements(&[e(1)]);
    let clones = g.t.clone_elements(&[e(1)], &mut g.host);
    assert_eq!(g.t.elements[&clones[0]].rows, [RowId(2)]);
}

// ----------------------------------------------------------------------------------------------------------------------
// Commands
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn a_batch_start_below_zero_becomes_zero_and_the_end_sets_the_duration_f81() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    g.t.move_resize_batch(&[(a, sec(-1.0), sec(3.0))], MoveKind::CenterAlign, &mut g.host);
    assert_eq!(g.times(a), (TimeSpan::ZERO, sec(3.0)));
}

#[test]
fn close_gaps_on_a_selection_walks_selection_order_q41() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 1.0, 2.0);
    let b = g.add(2, 0, 5.0, 6.0);
    // Selected later one first, so the pair runs backwards in time: a negative gap, which is queued.
    g.t.select_elements(&[b]);
    g.t.select_elements(&[a]);
    assert!(g.t.close_gaps(0.1, false, &mut g.host));
    assert_eq!(g.times(a), (sec(6.0), sec(2.0)).min((sec(1.0), sec(2.0))));
    assert_eq!(g.host.undo.len(), 1);
}

#[test]
fn already_aligned_still_records_one_entry_with_no_effects_q35() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    let b = g.add(2, 1, 10.0, 14.0);
    g.t.align(AlignKind::Start, &[a, b], Some(a), false, &mut g.host);
    assert_eq!(g.host.undo.last().unwrap().description(), "Start Align 0 effects");
}

#[test]
fn a_cloning_with_nothing_to_clone_still_records_q35() {
    let mut g = Grid::flat();
    assert!(g.t.clone_elements(&[], &mut g.host).is_empty());
    assert_eq!(g.host.undo.last().unwrap().description(), "Added 0 effects");
}

#[test]
fn shortening_the_sequence_or_changing_the_zoom_reclamps_the_start_q46() {
    let mut g = Grid::flat();
    g.t.set_total(sec(600.0));
    g.t.set_visible_start(sec(500.0));
    g.t.set_total(sec(200.0));
    assert_eq!(g.t.info.visible_start, sec(100.0));
    g.t.set_time_per_pixel(TimeSpan::from_ms_i(150));
    assert_eq!(g.t.info.visible_start, sec(50.0));
}

#[test]
fn following_playback_flips_on_a_new_position_and_not_on_a_paused_one_f95() {
    let mut g = ten_minutes();
    g.t.follow_playback(sec(91.0));
    assert_eq!(g.t.info.visible_start, sec(81.0));
    g.t.set_visible_start(sec(300.0));
    g.t.follow_playback(sec(91.0));
    assert_eq!(g.t.info.visible_start, sec(300.0));
    g.t.follow_playback(sec(92.0));
    assert_eq!(g.t.info.visible_start, sec(82.0));
}

// ----------------------------------------------------------------------------------------------------------------------
// Guides
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn guides_with_hidden_grid_lines_touch_only_the_overlay() {
    let mut g = Grid::flat();
    let mut c = MarkCollection { show_grid_lines: false, ..MarkCollection::default() };
    c.add_mark(vixen_core::marks::Mark::new(sec(1.0)));
    g.t.load_marks(vec![c]);
    g.t.take_outcome();
    g.t.set_guides(vec![sec(2.0)]);
    let out = g.t.take_outcome();
    assert!(out.dirty.overlay);
    assert!(out.dirty.grid_rects.is_empty());
    assert_eq!(g.t.guides(), [sec(2.0)]);
}

#[test]
fn a_guide_at_a_mark_repaints_only_that_column() {
    let mut g = Grid::flat();
    let mut c = MarkCollection { show_grid_lines: true, ..MarkCollection::default() };
    c.add_mark(vixen_core::marks::Mark::new(sec(1.0)));
    g.t.load_marks(vec![c]);
    g.t.take_outcome();
    g.t.set_guides(vec![sec(1.0)]);
    let out = g.t.take_outcome();
    assert!(!out.dirty.grid_all);
    assert_eq!(out.dirty.grid_rects.len(), 1);
    assert_eq!(out.dirty.grid_rects[0].w, 7);
}

// ----------------------------------------------------------------------------------------------------------------------
// Failures: F151
// ----------------------------------------------------------------------------------------------------------------------

#[test]
fn a_failed_frame_shows_one_alert_until_a_frame_succeeds_q45() {
    let f = super::model::DrawFailures::default();
    assert_eq!(f.take(), None);
    f.frame_failed();
    assert!(f.take().unwrap().starts_with("An unexpected error occurred while drawing the grid."));
    f.frame_failed();
    assert_eq!(f.take(), None);
    f.frame_succeeded();
    f.frame_failed();
    assert!(f.take().is_some());
}

// ----------------------------------------------------------------------------------------------------------------------
// Marks through a real session: F152, F153
// ----------------------------------------------------------------------------------------------------------------------

mod with_session {
    use super::super::marks::AddMarkResult;
    use super::*;
    use crate::sequence::{SequenceSession, SessionEnvironment};
    use vixen_core::marks::Mark;
    use vixen_io::ModuleStore;

    const SET_LEVEL: Uuid = uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0");

    fn at(x: i32, y: i32, button: Button, mods: Mods) -> PointerEvent {
        PointerEvent::new(x, y, Some(button), mods)
    }

    fn hover(x: i32, y: i32) -> PointerEvent {
        PointerEvent::new(x, y, None, Mods::NONE)
    }

    /// Two rows, a 100 ms pixel, one effect, and one collection in the bar with marks at 1 s and 3 s, each 1 s long. Saved, so not modified.
    fn opened() -> (Timeline, SequenceSession, Vec<Uuid>, ElementId) {
        let mut tree = ElementTree::new();
        let a = tree.add_element("A");
        tree.add_element("B");
        let ua = tree.node(a).id;
        let env = SessionEnvironment { video_cache: None,
            tree: Arc::new(tree),
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 25,
            media_dir: std::env::temp_dir(),
        };
        let mut session = SequenceSession::new_untitled(&env);
        let id = session.add_new_effect(SET_LEVEL, ua, "A", sec(1.0), sec(2.0)).unwrap();
        session.wait_for_renders();
        let mut collection = MarkCollection { name: "M".to_string(), show_marks: true, show_grid_lines: true, is_default: true, ..MarkCollection::default() };
        for t in [1.0, 3.0] {
            let mut m = Mark::new(sec(t));
            m.duration = sec(1.0);
            collection.add_mark(m);
        }
        let ids = collection.marks.iter().map(|m| m.id).collect();
        session.set_marks(vec![collection]);
        session.set_unmodified();
        let mut t = Timeline::new();
        t.info.time_per_pixel = TimeSpan::from_ms_i(100);
        t.load_from_session(&session, Vec::new());
        t.info.time_per_pixel = TimeSpan::from_ms_i(100);
        t.document_view = (t.info.time_per_pixel, t.info.visible_start);
        t.set_viewport(1000, 400, 1000);
        t.take_outcome();
        (t, session, ids, id)
    }

    fn mark_times(t: &Timeline) -> Vec<TimeSpan> {
        t.marks.collections[0].marks.iter().map(|m| m.start_time).collect()
    }

    #[test]
    fn undoing_a_mark_move_restores_its_times_and_not_modified_f152() {
        let (mut t, mut session, ids, _) = opened();
        t.marks_mouse_down(at(15, 5, Button::Left, Mods::NONE), &mut session);
        t.marks_mouse_move(at(30, 5, Button::Left, Mods::NONE));
        t.marks_mouse_move(at(45, 5, Button::Left, Mods::NONE));
        t.marks_mouse_up(at(45, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(mark_times(&t), [sec(2.5), sec(3.0)]);
        // Moving marks on release marks the sequence modified (F153).
        assert!(session.is_modified());
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Moved 1 effect");
        session.set_unmodified();
        session.undo(1);
        t.sync_from_session(&session);
        assert_eq!(mark_times(&t), [sec(1.0), sec(3.0)]);
        assert!(!session.is_modified());
        session.redo(1);
        t.sync_from_session(&session);
        assert_eq!(t.marks.mark(ids[0]).unwrap().start_time, sec(2.5));
        assert!(!session.is_modified());
    }

    #[test]
    fn undoing_a_mark_move_keeps_a_text_edit_made_since_which_has_no_undo_f152() {
        let (mut t, mut session, ids, _) = opened();
        t.marks_mouse_down(at(15, 5, Button::Left, Mods::NONE), &mut session);
        t.marks_mouse_move(at(30, 5, Button::Left, Mods::NONE));
        t.marks_mouse_up(at(30, 5, Button::Left, Mods::NONE), &mut session);
        t.marks.selected = vec![ids[0]];
        t.set_selected_marks_text("edited", &mut session);
        session.undo(1);
        t.sync_from_session(&session);
        assert_eq!(t.marks.mark(ids[0]).unwrap().text, "edited");
        assert_eq!(t.marks.mark(ids[0]).unwrap().start_time, sec(1.0));
    }

    #[test]
    fn redo_drops_a_mark_no_longer_in_its_collection_f152() {
        let (mut t, mut session, ids, _) = opened();
        t.marks_mouse_down(at(15, 5, Button::Left, Mods::NONE), &mut session);
        t.marks_mouse_move(at(30, 5, Button::Left, Mods::NONE));
        t.marks_mouse_up(at(30, 5, Button::Left, Mods::NONE), &mut session);
        session.undo(1);
        // The docker can take a mark away with no entry on this stack.
        let mut marks = session.marks().to_vec();
        marks[0].marks.retain(|m| m.id != ids[0]);
        session.set_marks(marks);
        session.redo(1);
        t.sync_from_session(&session);
        assert_eq!(t.marks.collections[0].marks.len(), 1);
    }

    #[test]
    fn adds_and_removes_undo_with_the_sequence_modified_f152() {
        let (mut t, mut session, ids, _) = opened();
        // Removed.
        t.marks.selected = vec![ids[0]];
        t.delete_marks_key(&mut session);
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Removed 1 Mark");
        session.set_unmodified();
        session.undo(1);
        t.sync_from_session(&session);
        assert_eq!(mark_times(&t), [sec(1.0), sec(3.0)]);
        assert!(session.is_modified());
        // Added, and an undone add leaves the mark in the selection if it was selected.
        let added = t.add_mark_at(sec(10.0), false, None, &mut session);
        let AddMarkResult::Added(id) = added else { panic!("added") };
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Added 1 Mark");
        t.marks.selected = vec![id];
        session.undo(1);
        t.sync_from_session(&session);
        assert_eq!(t.marks.collections[0].marks.len(), 2);
        assert_eq!(t.mark_selection(), [id]);
        session.redo(1);
        t.sync_from_session(&session);
        assert!(t.marks.mark(id).is_some());
    }

    #[test]
    fn undoing_an_add_keeps_a_new_collection_which_has_no_undo_f136() {
        let (mut t, mut session, ids, _) = opened();
        t.marks.collections[0].collection_type = vixen_core::marks::MarkCollectionType::Phrase;
        t.marks.selected = vec![ids[0]];
        t.marks.mark_mut_for_tests(ids[0]).unwrap().text = "a b".to_string();
        t.breakdown_phrase(&mut session);
        assert_eq!(t.marks.collections.len(), 2);
        assert_eq!(t.marks.collections[1].marks.len(), 2);
        session.undo(1);
        t.sync_from_session(&session);
        assert_eq!(t.marks.collections.len(), 2);
        assert!(t.marks.collections[1].marks.is_empty());
    }

    #[test]
    fn a_clipped_effect_goes_back_on_its_row_with_an_undo_f88() {
        let (mut t, mut session, _, id) = opened();
        t.move_by_time(&[id], sec(-5.0), true, &mut session);
        assert!(t.rows.row(RowId(0)).occurrences.is_empty());
        // Still in the sequence, at its old time.
        assert_eq!(session.effect(id).unwrap().node.start, sec(1.0));
        session.undo(1);
        t.sync_from_session(&session);
        assert!(t.rows.row(RowId(0)).occurrences.iter().any(|o| o.id == id));
        // Undoing and redoing both put it back, as `SwapElementPlacement` does with its null entries.
        session.redo(1);
        t.sync_from_session(&session);
        assert!(t.rows.row(RowId(0)).occurrences.iter().any(|o| o.id == id));
    }

    #[test]
    fn what_marks_the_sequence_modified_and_what_only_the_editor_state_f153() {
        // Effect time changes and undo.
        let (mut t, mut session, _, id) = opened();
        let x = 15;
        t.grid_mouse_move(hover(x, 5), &mut session);
        t.grid_mouse_down(at(x, 5, Button::Left, Mods::NONE));
        assert!(!session.is_modified());
        t.grid_mouse_move(at(x + 9, 5, Button::Left, Mods::NONE), &mut session);
        t.grid_mouse_move(at(x + 30, 5, Button::Left, Mods::NONE), &mut session);
        assert!(session.is_modified());
        t.grid_mouse_up(at(x + 30, 5, Button::Left, Mods::NONE), &mut session);
        assert!(t.elements[&id].start > sec(1.0));
        session.set_unmodified();
        session.undo(1);
        assert!(session.is_modified());

        // A ruler click yes, and a range drag no.
        let (mut t, mut session, _, _) = opened();
        t.ruler_mouse_move(hover(300, 5));
        t.ruler_mouse_down(at(300, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_up(at(300, 5, Button::Left, Mods::NONE), &mut session);
        assert!(session.is_modified());
        assert_eq!(t.cursor, sec(30.0));
        let (mut t, mut session, _, _) = opened();
        t.ruler_mouse_down(at(300, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_move(at(350, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_up(at(350, 5, Button::Left, Mods::NONE), &mut session);
        assert!(!session.is_modified());
        assert_eq!(t.cursor, TimeSpan::ZERO);
        assert_eq!((t.info.playback_start, t.info.playback_end), (Some(sec(30.0)), Some(sec(35.0))));

        // Mark text, add and remove yes.
        let (mut t, mut session, ids, _) = opened();
        t.marks.selected = vec![ids[0]];
        t.set_selected_marks_text("x", &mut session);
        assert!(session.is_modified());

        // Zoom, scroll and follow only set the editor state.
        let (mut t, session, _, _) = opened();
        assert!(!t.is_editor_state_modified());
        t.zoom_time(0.8);
        let out = t.take_outcome();
        assert!(out.editor_state_modified);
        assert!(t.is_editor_state_modified());
        assert!(!session.is_modified());
        // Once: it's never reset, and only says so the first time.
        t.zoom_time(0.8);
        assert!(!t.take_outcome().editor_state_modified);
        assert!(t.is_editor_state_modified());

        // Row height sets it, and expanding, selecting and the cursor don't.
        let (mut t, _session, _, _) = opened();
        t.click_row_label(RowId(0), Mods::NONE);
        t.set_cursor(sec(1.0));
        t.expand_all_rows();
        t.take_outcome();
        assert!(!t.is_editor_state_modified());
        t.zoom_rows(1.25);
        assert!(t.is_editor_state_modified());
    }

    #[test]
    fn opening_a_sequence_does_not_set_the_editor_state_f153() {
        let (mut t, session, _, _) = opened();
        let mut fresh = Timeline::new();
        fresh.load_from_session(&session, Vec::new());
        fresh.set_viewport(1000, 400, 1000);
        fresh.take_outcome();
        assert!(!fresh.is_editor_state_modified());
        t.take_outcome();
        assert!(!t.is_editor_state_modified());
    }

    #[test]
    fn a_mark_dragged_back_to_the_press_x_keeps_its_jump_with_no_undo_entry_q43() {
        let (mut t, mut session, ids, _) = opened();
        // The hit window is 20 ms, and a pixel 100 ms, so zoom in till a pixel off the mark still finds it.
        t.set_time_per_pixel(TimeSpan::from_ms_i(10));
        t.ruler_mouse_down(at(100, 5, Button::Left, Mods::NONE));
        assert_eq!(t.marks.ruler.drag_label.as_ref().map(|l| l.1.as_str()), Some(""));
        // The first move lands the mark on the pointer.
        t.ruler_mouse_move(at(150, 5, Button::Left, Mods::NONE));
        assert_eq!(t.marks.mark(ids[0]).unwrap().start_time, sec(1.5));
        assert!(!t.marks.ruler.drag_label.as_ref().unwrap().1.is_empty());
        t.ruler_mouse_move(at(100, 5, Button::Left, Mods::NONE));
        let before = session.undo_stack().undo_descriptions().len();
        t.ruler_mouse_up(at(100, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(session.undo_stack().undo_descriptions().len(), before);
        assert!(!session.is_modified());
        assert_eq!(session.marks()[0].marks[0].start_time, sec(1.0));
        assert!(t.marks.ruler.drag_label.is_none());
    }

    #[test]
    fn a_mark_drag_on_the_ruler_records_its_entry_q23() {
        let (mut t, mut session, ids, _) = opened();
        t.set_time_per_pixel(TimeSpan::from_ms_i(10));
        t.ruler_mouse_down(at(100, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_move(at(150, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_up(at(150, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Moved 1 effect");
        assert!(session.is_modified());
        assert!(t.guides().is_empty());
        session.undo(1);
        t.sync_from_session(&session);
        assert_eq!(t.marks.mark(ids[0]).unwrap().start_time, sec(1.0));
    }

    #[test]
    fn a_press_and_release_on_a_mark_with_no_movement_leaves_the_guide_up_q43() {
        let (mut t, mut session, _, _) = opened();
        t.set_time_per_pixel(TimeSpan::from_ms_i(10));
        t.ruler_mouse_move(hover(100, 5));
        assert_eq!(t.guides(), [sec(1.0)]);
        t.ruler_mouse_down(at(100, 5, Button::Left, Mods::NONE));
        let before = session.undo_stack().undo_descriptions().len();
        t.ruler_mouse_up(at(100, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(session.undo_stack().undo_descriptions().len(), before);
        assert_eq!(t.guides(), [sec(1.0)]);
    }

    #[test]
    fn ruler_clicks_up_to_two_points_are_clicks_and_three_is_a_drag_f100() {
        let (mut t, mut session, _, _) = opened();
        t.ruler_mouse_down(at(300, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_move(at(302, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_up(at(302, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(t.cursor, sec(30.2));
        assert_eq!(t.info.playback_end, None);
        let (mut t, mut session, _, _) = opened();
        t.ruler_mouse_down(at(300, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_move(at(303, 5, Button::Left, Mods::NONE));
        t.ruler_mouse_up(at(303, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(t.cursor, TimeSpan::ZERO);
        assert_eq!(t.info.playback_end, Some(sec(30.3)));
    }

    #[test]
    fn a_command_click_on_the_ruler_plays_and_leaves_the_cursor_f101() {
        let (mut t, mut session, _, _) = opened();
        t.ruler_mouse_down(at(300, 5, Button::Left, Mods::CMD));
        let out = t.ruler_mouse_up(at(300, 5, Button::Left, Mods::CMD), &mut session);
        assert_eq!(out.requests, [HostRequest::PlayRange { start: sec(30.0), end: None }]);
        assert_eq!(t.cursor, TimeSpan::ZERO);
        assert!(session.is_modified());
    }

    #[test]
    fn a_guide_from_a_mark_stays_moving_into_the_strip_and_goes_on_plain_ruler_q44() {
        let (mut t, _session, _, _) = opened();
        t.set_time_per_pixel(TimeSpan::from_ms_i(10));
        t.ruler_mouse_move(hover(100, 10));
        assert_eq!(t.guides(), [sec(1.0)]);
        // Off the mark, down in the strip.
        t.ruler_mouse_move(hover(160, 47));
        assert_eq!(t.guides(), [sec(1.0)]);
        t.ruler_mouse_move(hover(160, 20));
        assert!(t.guides().is_empty());
    }

    #[test]
    fn a_right_press_on_the_strip_then_release_adds_a_mark_f111() {
        let (mut t, mut session, _, _) = opened();
        t.ruler_mouse_move(hover(600, 47));
        t.ruler_mouse_down(at(600, 47, Button::Right, Mods::NONE));
        let out = t.ruler_mouse_up(at(600, 47, Button::Right, Mods::NONE), &mut session);
        assert_eq!(out.requests, [HostRequest::AddMark { time: sec(60.0), fill_gap: false, label: None }]);
    }

    #[test]
    fn right_clicking_a_mark_opens_its_menu_and_deleting_with_none_selected_records_removed_zero_f112() {
        let (mut t, mut session, _, _) = opened();
        t.set_time_per_pixel(TimeSpan::from_ms_i(10));
        t.ruler_mouse_down(at(100, 5, Button::Right, Mods::NONE));
        assert_eq!(t.marks.selected.len(), 1);
        let out = t.ruler_mouse_up(at(100, 5, Button::Right, Mods::NONE), &mut session);
        assert_eq!(out.requests, [HostRequest::MarkMenu]);
        t.delete_marks_key(&mut session);
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Removed 1 Mark");
        assert!(t.marks.selected.is_empty());
        // ⌘ toggles it off again, and then the menu acts on nothing.
        t.delete_marks_key(&mut session);
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Removed 0 Marks");
    }

    #[test]
    fn the_keys_delete_is_not_gated_but_the_menus_is_q31_q35() {
        let (mut t, mut session, ids, _) = opened();
        let before = session.undo_stack().undo_descriptions().len();
        t.delete_selected_marks(&mut session);
        assert_eq!(session.undo_stack().undo_descriptions().len(), before);
        t.delete_marks_key(&mut session);
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Removed 0 Marks");
        // A second press after a real delete holds nothing, so undoing both can't duplicate the marks.
        t.marks.selected = vec![ids[0]];
        t.delete_marks_key(&mut session);
        t.delete_marks_key(&mut session);
        session.undo(2);
        t.sync_from_session(&session);
        assert_eq!(t.marks.collections[0].marks.len(), 2);
    }

    #[test]
    fn a_click_in_a_marks_zone_records_a_resize_q34_and_a_locked_one_says_zero_q35() {
        let (mut t, mut session, _, _) = opened();
        t.marks_mouse_move(hover(11, 5));
        t.marks_mouse_down(at(11, 5, Button::Left, Mods::NONE), &mut session);
        t.marks_mouse_up(at(11, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Resize 1 effect");
        assert!(session.is_modified());

        let (mut t, mut session, _, _) = opened();
        t.marks.collections[0].is_locked = true;
        t.marks_mouse_move(hover(11, 5));
        t.marks_mouse_down(at(11, 5, Button::Left, Mods::NONE), &mut session);
        t.marks_mouse_up(at(11, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(session.undo_stack().undo_descriptions()[0], "Resize 0 effects");
    }

    #[test]
    fn a_press_on_blank_clears_and_moves_the_anchor_and_a_drag_from_blank_does_nothing_f121() {
        let (mut t, mut session, ids, _) = opened();
        t.marks.selected = vec![ids[0]];
        t.marks_mouse_down(at(25, 5, Button::Left, Mods::NONE), &mut session);
        assert!(t.marks.selected.is_empty());
        let before = mark_times(&t);
        t.marks_mouse_move(at(65, 5, Button::Left, Mods::NONE));
        t.marks_mouse_up(at(65, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(mark_times(&t), before);
        // The anchor moved: ⇧ takes from there.
        t.marks_mouse_down(at(35, 5, Button::Left, Mods::SHIFT), &mut session);
        assert_eq!(t.marks.selected, [ids[1]]);
        t.marks_mouse_up(at(35, 5, Button::Left, Mods::SHIFT), &mut session);
    }

    #[test]
    fn a_middle_click_selects_nothing_and_still_collapses_f121() {
        let (mut t, mut session, ids, _) = opened();
        t.marks.selected = ids.clone();
        t.marks_mouse_down(at(15, 5, Button::Middle, Mods::NONE), &mut session);
        assert_eq!(t.marks.selected.len(), 2);
        t.marks_mouse_up(at(15, 5, Button::Middle, Mods::NONE), &mut session);
        assert_eq!(t.marks.selected, [ids[0]]);
        // On blank, to nothing.
        t.marks.selected = ids.clone();
        t.marks_mouse_down(at(25, 5, Button::Middle, Mods::NONE), &mut session);
        t.marks_mouse_up(at(25, 5, Button::Middle, Mods::NONE), &mut session);
        assert!(t.marks.selected.is_empty());
    }

    #[test]
    fn an_empty_selection_shows_all_three_type_items_q42() {
        let (mut t, mut session, _, _) = opened();
        let id = t.marks.collections[0].marks[0].id;
        t.marks.selected = vec![id];
        // ⌘-right-click deselects the only selected mark, and "all selected" is true of nothing.
        t.marks_mouse_down(at(15, 5, Button::Right, Mods::CMD), &mut session);
        assert!(t.marks.selected.is_empty());
        t.marks_mouse_up(at(15, 5, Button::Right, Mods::CMD), &mut session);
        let menu = t.marks_context_menu();
        assert!(menu.phoneme && menu.breakdown_phrase && menu.breakdown_word);
    }

    #[test]
    fn the_bar_is_no_height_with_no_rows_shown_f117() {
        let (mut t, _session, _, _) = opened();
        assert_eq!(t.marks.bar_height, 20);
        t.set_all_mark_bars_visible(false);
        t.take_outcome();
        assert_eq!(t.marks.bar_height, 0);
    }

    #[test]
    fn what_clears_the_mark_selection_f155() {
        // A non-right press on the grid does, and a right press doesn't.
        let (mut t, _session, ids, _) = opened();
        t.marks.selected = vec![ids[0]];
        t.grid_mouse_down(at(500, 300, Button::Right, Mods::NONE));
        assert_eq!(t.mark_selection(), [ids[0]]);
        t.grid_mouse_down(at(500, 300, Button::Left, Mods::NONE));
        assert!(t.mark_selection().is_empty());
        t.marks.selected = vec![ids[0]];
        t.grid_mouse_down(at(500, 300, Button::Middle, Mods::NONE));
        assert!(t.mark_selection().is_empty());

        // A ruler press off a mark and off the strip does, and one in the strip doesn't.
        let (mut t, _session, ids, _) = opened();
        t.marks.selected = vec![ids[0]];
        t.ruler_mouse_move(hover(600, 47));
        t.ruler_mouse_down(at(600, 47, Button::Left, Mods::NONE));
        assert_eq!(t.mark_selection(), [ids[0]]);
        t.ruler_mouse_move(hover(600, 5));
        t.ruler_mouse_down(at(600, 5, Button::Left, Mods::NONE));
        assert!(t.mark_selection().is_empty());

        // A ruler press on a mark without exactly ⌘ does, and with it adds.
        let (mut t, _session, ids, _) = opened();
        t.set_time_per_pixel(TimeSpan::from_ms_i(10));
        t.marks.selected = vec![ids[1]];
        t.ruler_mouse_down(at(100, 5, Button::Left, Mods::CMD));
        assert_eq!(t.mark_selection(), [ids[1], ids[0]]);
        t.ruler_mouse_down(at(100, 5, Button::Left, Mods::NONE));
        assert_eq!(t.mark_selection(), [ids[0]]);

        // A bar press on an unselected mark or blank with no modifier does.
        let (mut t, mut session, ids, _) = opened();
        t.marks.selected = vec![ids[1]];
        t.marks_mouse_down(at(15, 5, Button::Left, Mods::NONE), &mut session);
        assert_eq!(t.mark_selection(), [ids[0]]);

        // Ruler delete does, and a paste replaces it with what was pasted.
        let (mut t, mut session, ids, _) = opened();
        t.marks.selected = vec![ids[0]];
        t.copy_marks();
        t.paste_marks((900, 5), &mut |_| true, &mut session);
        assert_eq!(t.mark_selection().len(), 1);
        assert_ne!(t.mark_selection(), [ids[0]]);
        t.delete_marks_key(&mut session);
        assert!(t.mark_selection().is_empty());
    }

    #[test]
    fn a_double_click_on_a_mark_outside_the_sequence_plays_from_the_range_or_the_ends_f124() {
        let (mut t, mut session, ids, _) = opened();
        t.info.total = sec(90.0);
        t.info.playback_start = Some(sec(7.0));
        t.info.playback_end = Some(sec(50.0));
        t.marks.mark_mut_for_tests(ids[0]).unwrap().start_time = sec(-2.0);
        t.marks.mark_mut_for_tests(ids[0]).unwrap().duration = sec(5.0);
        t.marks.mark_mut_for_tests(ids[1]).unwrap().start_time = sec(100.0);
        t.marks.set_collections(t.marks.collections.clone());
        t.refresh_marks_layout();
        // Ends past the sequence: the range's end.
        let mut ev = at(1000, 5, Button::Left, Mods::NONE);
        ev.clicks = 2;
        let out = t.marks_mouse_down(ev, &mut session);
        assert_eq!(out.requests, [HostRequest::PlayRange { start: sec(100.0), end: Some(sec(50.0)) }]);
        // Starts before the sequence: the range's start.
        let mut ev = at(10, 5, Button::Left, Mods::NONE);
        ev.clicks = 2;
        let out = t.marks_mouse_down(ev, &mut session);
        assert_eq!(out.requests, [HostRequest::PlayRange { start: sec(7.0), end: Some(sec(3.0)) }]);
        // With no range, the ends.
        t.info.playback_start = None;
        t.info.playback_end = None;
        let out = t.marks_mouse_down(ev, &mut session);
        assert_eq!(out.requests, [HostRequest::PlayRange { start: TimeSpan::ZERO, end: Some(sec(3.0)) }]);
    }

    #[test]
    fn fill_gap_passes_over_a_touching_previous_mark_f125() {
        use super::super::marks::fill_gap_times;
        let mark = |t: f64, d: f64| {
            let mut m = Mark::new(sec(t));
            m.duration = sec(d);
            m
        };
        // With no earlier mark the start stays.
        let mut c = MarkCollection::default();
        c.add_mark(mark(1.0, 1.0));
        c.add_mark(mark(2.0, 1.0));
        let id = c.marks[1].id;
        fill_gap_times(&mut c, id);
        assert_eq!(c.marks[1].start_time, sec(2.0));
        // With one earlier still, the start goes to that one's end + 1 ms, over the previous mark.
        let mut c = MarkCollection::default();
        c.add_mark(mark(0.0, 0.5));
        c.add_mark(mark(1.0, 1.0));
        c.add_mark(mark(2.0, 1.0));
        let id = c.marks[2].id;
        fill_gap_times(&mut c, id);
        assert_eq!(c.marks[2].start_time, TimeSpan::from_ms_i(501));
    }

    #[test]
    fn the_waveform_is_blank_while_building_and_past_the_media_end_f141() {
        let (mut t, _session, _, _) = opened();
        assert!(!t.waveform_blank());
        t.set_audio(true, sec(30.0));
        assert!(t.waveform_blank());
        t.waveform.peaks = None;
        t.info.total = sec(120.0);
        t.set_viewport(100, 100, 100);
        t.set_visible_start(sec(10.0));
        assert!(t.waveform_blank());
    }
}

#[test]
fn snap_candidates_come_from_the_tracked_row_even_when_it_lags_the_pointer_f71() {
    let mut g = Grid::flat();
    g.t.opts.snap_strength = 2;
    let a = g.add(1, 3, 10.0, 12.0);
    let neighbour_in_row_three = g.add(2, 3, 20.0, 22.0);
    let in_row_two = g.add(3, 2, 18.0, 19.0);
    g.hover(110, tab(3), Mods::NONE);
    g.down(110, tab(3), Mods::NONE);
    g.drag(120, tab(3), Mods::NONE);
    assert_eq!(g.t.drag_state(), DragState::Moving);
    // Past the last row, the move clips and the tracked row stays where it was.
    g.drag(191, 500, Mods::NONE);
    assert_eq!(g.t.g.current_row_index, 3);
    // 7.1 s puts the end at 19.1 s. The neighbour in the tracked row is in reach, and row two's edges are ignored.
    assert_eq!(g.times(a), (sec(18.0), sec(20.0)));
    let _ = (neighbour_in_row_three, in_row_two);
}

#[test]
fn a_selection_in_flat_row_zero_acts_unscoped_q11() {
    let mut g = Grid::flat();
    let a = g.add(1, 0, 10.0, 12.0);
    g.add(2, 1, 20.0, 21.0);
    g.t.select_elements(&[a]);
    g.hover(110, tab(0), Mods::NONE);
    g.down(110, tab(0), Mods::NONE);
    g.drag(120, tab(0), Mods::NONE);
    // The row above zero test fails, so `a` pairs with the first row holding it, and row one's edges still don't match it.
    g.drag(190, tab(0), Mods::NONE);
    assert_eq!(g.times(a), (sec(17.0), sec(19.0)));
}
