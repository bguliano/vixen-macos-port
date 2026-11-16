//! The grid's input state machine: `Grid_Mouse.cs` and the drag parts of `Grid.cs`. F45 to F67, F77 to F79. Every function here is
//! one pointer event from Swift, and returns what changed.

use super::host::TimelineHost;
use super::model::{HostRequest, IdMap, InputOutcome, Timeline};
use super::rows::RowTree;
use super::snap::{find_snap, DragSnap, SnapEdge};
use super::stack::lane_geometry;
use super::view::px;
use super::{Button, CursorKind, DragState, ElementId, ElementTimes, IRect, Mods, ResizeZone, RowId};
use crate::undo::{EffectPlacement, MoveKind, UndoAction};
use std::collections::HashSet;
use vixen_core::{NodeId, TimeSpan};

/// `AutoScrollMargin`, on all edges.
const AUTOSCROLL_MARGIN: i32 = 24;
/// `AutoScrollPxScaleFactor`.
const AUTOSCROLL_SCALE: i32 = 4;
/// `DragThreshold`.
const DRAG_THRESHOLD: i32 = 8;
/// `ElementResizeThreshold`.
const RESIZE_THRESHOLD: i32 = 12;
/// `MinElemWidthPx`.
const MIN_ELEMENT_WIDTH_PX: i32 = 10;
/// How far back and forward the adjoining search looks.
const ADJOINING_LIMIT: usize = 20;

/// One pointer event, in the surface's client coordinates. A surface floors its points before making this.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PointerEvent {
    pub x: i32,
    pub y: i32,
    pub button: Option<Button>,
    pub clicks: u8,
    pub mods: Mods,
}

impl PointerEvent {
    pub fn new(x: i32, y: i32, button: Option<Button>, mods: Mods) -> PointerEvent {
        PointerEvent { x, y, button, clicks: 1, mods }
    }
}

/// `ElementTimeInfo`: where an effect was before the drag.
#[derive(Copy, Clone, Debug)]
pub(crate) struct OriginalPlacement {
    pub start: TimeSpan,
    pub duration: TimeSpan,
    pub row: Option<RowId>,
    pub target: Option<NodeId>,
}

/// `ElementMoveInfo`.
#[derive(Clone, Debug, Default)]
pub(crate) struct MoveInfo {
    pub initial: (i32, i32),
    pub originals: IdMap<OriginalPlacement>,
    pub order: Vec<ElementId>,
    pub visible_start: TimeSpan,
    pub earliest: TimeSpan,
    pub latest: TimeSpan,
    pub shortest: TimeSpan,
}

/// The fields of `Grid` that hold a drag.
#[derive(Default)]
pub(crate) struct GridInput {
    pub drag_state: DragState,
    /// `m_lastGridLocation`.
    pub last_grid_location: (i32, i32),
    /// `mouseDownGridLocation`.
    pub mouse_down_location: (i32, i32),
    /// `m_mouseDownElementRow`. Tracked as the drag moves between rows.
    pub mouse_down_row: Option<RowId>,
    /// `m_mouseDownElements`. KEEP Q8: this persists after release, which is why the freeze rules apply in every state.
    pub mouse_down_elements: Vec<ElementId>,
    /// `m_lastSingleSelectedElementLocation`. (0, 0) is `Point.Empty`, which means none.
    pub last_single_selected: (i32, i32),
    /// `_workingElement`.
    pub working_element: Option<ElementId>,
    /// `m_mouseResizeZone`.
    pub resize_zone: ResizeZone,
    /// `m_ignoreDragArea`.
    pub ignore_drag_area: IRect,
    /// `m_waitingBeginGridLocation`.
    pub waiting_begin: (i32, i32),
    /// `m_elemMoveInfo`.
    pub move_info: Option<MoveInfo>,
    /// `adjoiningElements`.
    pub adjoining: Option<Vec<ElementId>>,
    /// `CurrentRowIndexUnderMouse`. -1 when there was no row.
    pub current_row_index: i32,
    /// `m_selectionRectangleStart`.
    pub selection_start: (i32, i32),
    /// `SelectionArea`.
    pub selection_area: IRect,
    /// `tempSelectedElements`.
    pub temp_selected: Vec<ElementId>,
    /// `m_drawingRectangleStart`.
    pub drawing_start: (i32, i32),
    /// `DrawingArea`.
    pub drawing_area: IRect,
    /// `_beginEffectDraw`.
    pub begin_effect_draw: bool,
    /// `effectDrawMouseDownTime`.
    pub draw_down_time: TimeSpan,
    /// `m_lastMouseMove`.
    pub last_mouse_move: Option<PointerEvent>,
    /// `m_mouseOutside`.
    pub mouse_outside: (i32, i32),
    /// `m_autoScrollTimer.Enabled`.
    pub autoscroll: bool,
    /// `CurrentDragSnapPoints`, without the static points.
    pub drag_snap: Option<DragSnap>,
    /// `capturedElements`: what the hover info shows.
    pub captured: Vec<ElementId>,
    /// This press offered the overlap menu, whose tracking loop takes the release (F50).
    pub chooser_pressed: bool,
    /// The sequence has heard that this drag changed something (F58).
    pub modified_noted: bool,
}

impl GridInput {
    #[inline]
    pub fn is_moving(&self, id: ElementId) -> bool {
        self.move_info.as_ref().is_some_and(|m| m.originals.contains_key(&id))
    }

    pub fn adjoining_ids(&self) -> &[ElementId] {
        self.adjoining.as_deref().unwrap_or(&[])
    }
}

/// F77, FIX Q40: the pointer can leave the grid while drawing, and upstream stores those times as they are, which gives an effect outside the sequence.
/// Start and duration.
pub fn clamp_draw_times(press: TimeSpan, release: TimeSpan, total: TimeSpan) -> (TimeSpan, TimeSpan) {
    let (a, b) = (press.max(TimeSpan::ZERO).min(total), release.max(TimeSpan::ZERO).min(total));
    (a.min(b), a.max(b) - a.min(b))
}

/// F65: which end of an effect a canvas x is grabbing, if any. Marks use this too, with the same constant.
pub fn resize_zone(times: ElementTimes, tpp: TimeSpan, x: i32) -> ResizeZone {
    let grab = RESIZE_THRESHOLD.min((px(times.duration, tpp) / 2.0) as i32) as f64;
    let start = px(times.start, tpp);
    let end = px(times.end(), tpp);
    let x = x as f64;
    if x >= start && x < start + grab {
        ResizeZone::Front
    } else if x <= end && x > end - grab {
        ResizeZone::Back
    } else {
        ResizeZone::None
    }
}

impl Timeline {
    /// `IsResizeDragInProgress`: any state other than Normal.
    pub fn is_drag_in_progress(&self) -> bool {
        self.g.drag_state != DragState::Normal
    }

    /// `m_dragState`, for hotkeys and tests.
    pub fn drag_state(&self) -> DragState {
        self.g.drag_state
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Press
    // ------------------------------------------------------------------------------------------------------------------

    /// `OnMouseDown`. F46, F77, F78.
    pub fn grid_mouse_down(&mut self, ev: PointerEvent) -> InputOutcome {
        self.refresh_view_stacks();
        self.g.chooser_pressed = false;
        // F155: the form's handler for any press here but a right one.
        if ev.button != Some(Button::Right) {
            self.marks_clear_selected();
        }
        let p = self.to_canvas((ev.x, ev.y));
        self.g.mouse_down_location = p;
        self.g.last_grid_location = p;
        self.g.mouse_down_row = self.rows.row_at(p.1);
        self.g.mouse_down_elements = self.elements_at(p);
        // A tap can arrive with no move before it, so the hover state may not be up to date.
        if self.g.drag_state == DragState::Normal {
            self.update_resize_zone(p, ev.mods);
        }
        self.press(ev, p);
        self.publish_indicator_guides();
        self.finish()
    }

    fn press(&mut self, ev: PointerEvent, p: (i32, i32)) {
        let mods = ev.mods;
        let Some(button) = ev.button else { return };
        if button == Button::Middle && self.opts.selected_effect.is_some() {
            // F78: in any mode, ignoring ⌥ and resize zones.
            if self.g.mouse_down_row.is_none() {
                return;
            }
            self.g.begin_effect_draw = true;
            self.cursor_kind = CursorKind::Crosshair;
            self.g.draw_down_time = self.time(p.0);
            self.begin_draw_box(p);
            self.g.last_single_selected = (0, 0);
        }
        if button != Button::Left && button != Button::Right {
            return;
        }
        self.g.working_element = self.element_at(p);
        // FIX Q5: a right press in Draw Mode starts a box upstream that nothing ever ends. On the Mac it goes straight to the menu path.
        if button == Button::Left
            && self.opts.draw_mode
            && !mods.opt
            && self.opts.selected_effect.is_some()
            && self.g.resize_zone == ResizeZone::None
        {
            if self.g.mouse_down_row.is_none() {
                return;
            }
            self.g.begin_effect_draw = true;
            self.g.draw_down_time = self.time(p.0);
            self.begin_draw_box(p);
            self.g.last_single_selected = (0, 0);
            return;
        }
        let elements = self.g.mouse_down_elements.clone();
        if elements.is_empty() && button != Button::Right {
            // The background: clear anything selected and begin a selection drag.
            if !mods.cmd {
                self.begin_drag_select(p, mods);
                self.g.last_single_selected = (0, 0);
            }
            return;
        }
        if elements.iter().any(|id| self.is_selected(*id)) {
            if mods.cmd {
                for id in &elements {
                    if self.is_selected(*id) {
                        self.set_selected(*id, false);
                    }
                }
                self.raise_selection_changed();
            } else if mods.shift && self.g.last_single_selected != (0, 0) {
                self.select_elements_between(self.g.last_single_selected, p, mods);
            }
        } else if mods.shift && self.g.last_single_selected != (0, 0) {
            self.select_elements_between(self.g.last_single_selected, p, mods);
        } else {
            // Unless adding to the selection, this replaces it. The host may take over the choice (F50).
            if !mods.cmd {
                self.clear_selected_quietly();
                self.clear_selected_rows(None);
                self.clear_active_rows(None);
            }
            if self.offer_to_host(&elements, ev) {
                for id in &elements {
                    self.set_selected(*id, true);
                }
                self.g.last_single_selected = p;
                if let Some(row) = self.rows.row_at(p.1) {
                    self.set_row_active(row, true);
                }
                self.raise_selection_changed();
                if mods.cmd {
                    self.move_cursor_to_last_of(&elements);
                } else {
                    self.move_cursor_to_earliest(&elements);
                }
            } else {
                if !mods.cmd {
                    self.raise_selection_changed();
                }
                // FIX Q47: upstream goes on to wait for a drag behind the menu, and the release offers the list again.
                if button == Button::Left {
                    self.g.chooser_pressed = true;
                    return;
                }
            }
        }
        if button == Button::Left {
            if self.g.resize_zone == ResizeZone::None {
                self.wait_for_drag_move(p);
            } else if !mods.cmd {
                if mods.opt && self.selected_list().len() > 1 {
                    self.requests.push(HostRequest::Warning {
                        text: "Too many effects selected.\nMaximum selected effects for this action is 1".to_string(),
                        title: "Warning".to_string(),
                    });
                    return;
                }
                self.begin_h_resize(p);
            }
        }
    }

    #[inline]
    pub(crate) fn is_selected(&self, id: ElementId) -> bool {
        self.elements.get(&id).is_some_and(|e| e.selected)
    }

    /// `_ElementsSelected`, answered as `timelineControl_ElementsSelected` does: more than one effect goes to a menu. F50.
    fn offer_to_host(&mut self, elements: &[ElementId], ev: PointerEvent) -> bool {
        if elements.len() > 1 {
            // FIX Q47: the release of a right press opens the context menu with the same effects, which would close this one.
            if ev.button != Some(Button::Right) {
                self.requests.push(HostRequest::ChooseOverlapping { elements: elements.to_vec(), at: (ev.x, ev.y) });
            }
            return false;
        }
        true
    }

    /// F50: the overlap menu has closed, with a choice or without. `SelectElement`: a pick moves the cursor and sets no anchor or active row, and with ⌘
    /// held it adds to the selection.
    pub fn finish_chooser(&mut self, chosen: Option<ElementId>) -> InputOutcome {
        if let Some(id) = chosen {
            self.select_element(id);
        }
        self.finish()
    }

    /// `elementAt` and the overlap menu's label. F50.
    pub fn overlap_menu_label(&self, id: ElementId) -> Option<String> {
        let e = self.elements.get(&id)?;
        Some(format!("{} ({})", e.desc.type_name, super::model::format_m_ss_fff(e.start)))
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Release
    // ------------------------------------------------------------------------------------------------------------------

    /// `OnMouseUp`. F49, F54, F58, F66.
    pub fn grid_mouse_up(&mut self, ev: PointerEvent, host: &mut dyn TimelineHost) -> InputOutcome {
        self.refresh_view_stacks();
        self.set_guides(Vec::new());
        let p = self.to_canvas((ev.x, ev.y));
        let mods = ev.mods;
        // FIX Q47: no second offer, should a release still come after the menu.
        if std::mem::take(&mut self.g.chooser_pressed) && ev.button == Some(Button::Left) {
            self.g.working_element = None;
            return self.finish();
        }
        match ev.button {
            Some(Button::Middle) if self.g.begin_effect_draw => match self.g.drag_state {
                DragState::Drawing => {
                    self.g.begin_effect_draw = false;
                    self.cursor_kind = CursorKind::Arrow;
                    self.finish_drawing(p, host);
                }
                _ => self.end_all_drag(),
            },
            Some(Button::Left) => {
                self.g.working_element = None;
                match self.g.drag_state {
                    DragState::Moving => self.mouse_up_drag_moving(p, host),
                    DragState::Drawing => {
                        self.g.begin_effect_draw = false;
                        self.finish_drawing(p, host);
                    }
                    DragState::Selecting => self.mouse_up_drag_select(p),
                    DragState::HResizing => self.mouse_up_h_resizing(host),
                    _ => self.mouse_up_no_drag(ev, p, mods),
                }
            }
            Some(Button::Right) => {
                self.clear_active_rows(None);
                if let Some(row) = self.rows.row_at(p.1) {
                    self.set_row_active(row, true);
                    let elements = self.g.mouse_down_elements.clone();
                    // `ClickingGridSetsCursor` is always on.
                    if self.opts.legacy_cursor || elements.is_empty() {
                        self.set_cursor(self.time(p.0));
                    }
                    self.requests.push(HostRequest::ContextMenu { elements, time: self.time(p.0), row });
                }
            }
            _ => {}
        }
        self.finish()
    }

    /// F49: a click on one of several selected effects selects only that one.
    fn mouse_up_no_drag(&mut self, ev: PointerEvent, p: (i32, i32), mods: Mods) {
        self.end_all_drag();
        let elements = self.g.mouse_down_elements.clone();
        if elements.is_empty() {
            return;
        }
        if !mods.cmd && !mods.shift {
            self.clear_selection();
            if self.offer_to_host(&elements, ev) {
                let first = elements[0];
                self.set_selected(first, true);
                self.g.last_single_selected = p;
                self.raise_selection_changed();
                if let Some(row) = self.rows.row_at(p.1) {
                    self.set_row_active(row, true);
                }
                self.move_cursor_to_last_of(&[first]);
            }
        }
        if mods.shift || mods.cmd {
            self.clear_active_rows(None);
            if let Some(row) = self.rows.row_at(p.1) {
                self.set_row_active(row, true);
            }
            self.move_cursor_to_last_of(&elements);
        }
    }

    /// `OnMouseDoubleClick`. F55. Nothing in the editor listens, and the two presses have already acted as F46.
    pub fn grid_double_click(&mut self, ev: PointerEvent) -> Option<ElementId> {
        self.refresh_view_stacks();
        let p = self.to_canvas((ev.x, ev.y));
        let id = self.element_at(p);
        if let Some(id) = id {
            self.requests.push(HostRequest::ElementDoubleClicked(id));
        }
        id
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Move
    // ------------------------------------------------------------------------------------------------------------------

    /// `OnMouseMove`.
    pub fn grid_mouse_move(&mut self, ev: PointerEvent, host: &mut dyn TimelineHost) -> InputOutcome {
        self.refresh_view_stacks();
        self.show_tooltip((ev.x, ev.y));
        if matches!(self.g.drag_state, DragState::Moving | DragState::Selecting | DragState::Drawing | DragState::HResizing) {
            self.g.mouse_outside = (
                edge_distance(ev.x, self.grid_width),
                edge_distance(ev.y, self.grid_height),
            );
            self.g.autoscroll = self.g.mouse_outside != (0, 0);
        }
        self.g.last_mouse_move = Some(ev);
        self.handle_mouse_move(ev, host);
        self.finish()
    }

    /// `ShowToolTip`: tracks the effects under the pointer on every move. F45.
    fn show_tooltip(&mut self, client: (i32, i32)) {
        let elements = self.elements_at(self.to_canvas(client));
        if elements != self.g.captured {
            self.g.captured = elements;
            self.dirty.overlay = true;
        }
    }

    /// `HandleMouseMove`: applies the freeze rules, then dispatches on the drag state.
    fn handle_mouse_move(&mut self, ev: PointerEvent, host: &mut dyn TimelineHost) {
        let mut p = self.to_canvas((ev.x, ev.y));
        let mods = ev.mods;
        if !self.g.mouse_down_elements.is_empty() {
            // KEEP Q8: these apply in every state, because the press list persists after release.
            let vertical_only = mods.exactly(true, false, false) || mods.exactly(true, true, false) || mods.exactly(false, true, true);
            if vertical_only {
                p.0 = self.g.last_grid_location.0;
            }
            if mods.exactly(true, false, true) {
                p.1 = self.g.last_grid_location.1;
            }
        }
        self.g.last_grid_location = p;
        match self.g.drag_state {
            DragState::Normal => self.mouse_move_normal(p, mods),
            DragState::Waiting => {
                if !self.g.ignore_drag_area.contains(p) {
                    if mods.cmd {
                        // F60: ensure what's under the pointer is selected, then drag clones of the selection.
                        let elements = self.elements_at(self.g.waiting_begin);
                        if !elements.is_empty() {
                            for id in &elements {
                                self.set_selected(*id, true);
                            }
                            self.raise_selection_changed();
                        }
                        self.clone_selected_for_move(host);
                    }
                    self.begin_drag_move(p);
                }
            }
            DragState::Moving => {
                if self.opts.resize_indicator {
                    for id in self.elements_at(p) {
                        if self.is_selected(id) {
                            self.g.working_element = Some(id);
                        }
                    }
                }
                if self.mouse_move_drag_moving(p) {
                    self.note_drag_changed(host);
                }
            }
            DragState::Selecting => self.mouse_move_drag_select(p, mods),
            DragState::Drawing => self.mouse_move_draw_select(p),
            DragState::HResizing => {
                if self.mouse_move_h_resizing(p, mods) {
                    self.note_drag_changed(host);
                }
            }
        }
        self.publish_indicator_guides();
    }

    /// F58, F153: the time change notification reaches the form on every move, so the sequence is modified from the first, whether or not the drag is
    /// later cancelled. This is the one thing a drag takes the session's lock for before release.
    fn note_drag_changed(&mut self, host: &mut dyn TimelineHost) {
        if !self.g.modified_noted {
            self.g.modified_noted = true;
            host.mark_modified();
        }
    }

    fn update_resize_zone(&mut self, p: (i32, i32), mods: Mods) {
        let zone = match self.element_at(p).and_then(|id| self.elements.get(&id)) {
            Some(e) => resize_zone(e.times(), self.tpp(), p.0),
            None => ResizeZone::None,
        };
        self.g.resize_zone = zone;
        self.cursor_kind = if zone != ResizeZone::None {
            CursorKind::ResizeLeftRight
        } else if self.opts.draw_mode && !mods.opt && self.opts.selected_effect.is_some() {
            CursorKind::Crosshair
        } else {
            CursorKind::Arrow
        };
    }

    /// `MouseMove_Normal`.
    fn mouse_move_normal(&mut self, p: (i32, i32), mods: Mods) {
        self.update_resize_zone(p, mods);
    }

    /// FIX Q14: upstream publishes these from paint, and the event asks for another paint, so a drag repaints continuously.
    fn publish_indicator_guides(&mut self) {
        if !self.opts.resize_indicator {
            return;
        }
        let Some(e) = self.g.working_element.and_then(|id| self.elements.get(&id)) else { return };
        let (start, end) = (e.start, e.end());
        match self.g.drag_state {
            DragState::HResizing => {
                let edge = if self.g.resize_zone == ResizeZone::Front { start } else { end };
                self.set_guides(vec![edge]);
            }
            DragState::Waiting | DragState::Moving => self.set_guides(vec![start, end]),
            _ => {}
        }
    }

    /// `_drawResizeIndicator` and the end of `_drawDrawBox`. F79.
    pub(crate) fn resize_indicator_lines(&self) -> Vec<f64> {
        if !self.opts.resize_indicator {
            return Vec::new();
        }
        if self.g.drag_state == DragState::Drawing {
            let a = self.g.drawing_area;
            return vec![a.x as f64, (a.x + a.w - 1) as f64];
        }
        let Some(e) = self.g.working_element.and_then(|id| self.elements.get(&id)) else { return Vec::new() };
        let tpp = self.tpp();
        match self.g.drag_state {
            DragState::HResizing => vec![if self.g.resize_zone == ResizeZone::Front { px(e.start, tpp) } else { px(e.end(), tpp) - 1.0 }],
            DragState::Waiting | DragState::Moving => vec![px(e.start, tpp), px(e.end(), tpp) - 1.0],
            _ => Vec::new(),
        }
    }

    /// `OnAlignmentActivity`. A guide replaces the mark line at its time, so those columns repaint (F76).
    pub(crate) fn set_guides(&mut self, guides: Vec<TimeSpan>) {
        if guides == self.guides {
            return;
        }
        let mut columns: Vec<TimeSpan> = Vec::new();
        for t in self.guides.iter().chain(guides.iter()) {
            if self.snap_index.has_points_at(*t) {
                columns.push(*t);
            }
        }
        self.guides = guides;
        self.dirty.overlay = true;
        for t in columns {
            let x = px(t, self.tpp()) as i32;
            self.dirty.grid_rects.push(IRect::new(x - 3, i32::MIN / 2, 7, i32::MAX));
        }
    }

    /// The alignment times now in force.
    pub fn guides(&self) -> &[TimeSpan] {
        &self.guides
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Autoscroll and cancel
    // ------------------------------------------------------------------------------------------------------------------

    /// `m_autoScrollTimer_Tick`, every 50 ms while `autoscroll` is true. F63. The rate grows with distance, with no upper limit.
    pub fn grid_autoscroll_tick(&mut self, mods: Mods, host: &mut dyn TimelineHost) -> InputOutcome {
        if !self.g.autoscroll {
            return self.finish();
        }
        let x = (self.scroll_x() + self.g.mouse_outside.0 / AUTOSCROLL_SCALE).clamp(0, self.canvas_width().max(0));
        let y = (self.vertical_offset + self.g.mouse_outside.1 / AUTOSCROLL_SCALE).clamp(0, self.rows.canvas_height().max(0));
        self.set_visible_start(self.time(x));
        self.set_vertical_offset(y);
        self.refresh_view_stacks();
        if let Some(mut ev) = self.g.last_mouse_move {
            ev.mods = mods;
            self.handle_mouse_move(ev, host);
        }
        self.finish()
    }

    /// `elementsCancelMove`. F64. FIX Q10: this restores rows too, and leaves no half state or undo entry. Clones made by this drag
    /// remain, with their undo entry. KEEP.
    pub fn grid_cancel_drag(&mut self) -> InputOutcome {
        self.flush_rows();
        let Some(info) = self.g.move_info.take() else { return self.finish() };
        self.set_visible_start(info.visible_start);
        for id in &info.order {
            let Some(orig) = info.originals.get(id).copied() else { continue };
            self.restore_placement(*id, &orig);
        }
        self.flush_rows();
        for id in &info.order {
            self.set_lifted(*id, false);
        }
        self.g.adjoining = None;
        self.g.drag_snap = None;
        self.end_all_drag();
        self.finish()
    }

    /// Puts an effect back in the rows and at the times it had.
    fn restore_placement(&mut self, id: ElementId, orig: &OriginalPlacement) {
        self.set_element_times(id, orig.start, orig.duration, false);
        let Some(e) = self.elements.get(&id) else { return };
        if e.target != orig.target {
            let current = e.rows.clone();
            for r in current {
                self.row_remove_element(r, id);
            }
            if let Some(e) = self.elements.get_mut(&id) {
                e.target = orig.target;
            }
            if let Some(node) = orig.target {
                for r in self.rows.rows_of_node(node).to_vec() {
                    self.row_add_element(r, id);
                }
            }
            if let Some(e) = self.elements.get_mut(&id) {
                e.last_row = orig.row;
            }
        }
    }

    /// `endAllDrag`.
    pub(crate) fn end_all_drag(&mut self) {
        self.g.drag_state = DragState::Normal;
        self.cursor_kind = CursorKind::Arrow;
        self.g.autoscroll = false;
        self.g.mouse_outside = (0, 0);
        self.dirty.overlay = true;
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Drawing
    // ------------------------------------------------------------------------------------------------------------------

    /// `beginDrawBox`.
    fn begin_draw_box(&mut self, p: (i32, i32)) {
        self.g.drag_state = DragState::Drawing;
        self.clear_selection();
        let row = self.g.mouse_down_row;
        self.clear_selected_rows(row);
        self.clear_active_rows(row);
        self.g.drawing_area = IRect::new(p.0, p.1, 0, 0);
        self.g.drawing_start = p;
        self.dirty.overlay = true;
    }

    /// `MouseMove_DrawSelect`: the box stretched to whole rows.
    fn mouse_move_draw_select(&mut self, p: (i32, i32)) {
        let start = self.g.drawing_start;
        let top_left = (start.0.min(p.0), start.1.min(p.1));
        let bottom_right = (start.0.max(p.0), start.1.max(p.1));
        let rows = self.rows.rows_within(IRect::from_points(top_left, bottom_right));
        // GUARD: upstream's `First()` throws on an empty list.
        let (Some(first), Some(last)) = (rows.first(), rows.last()) else { return };
        let first_top = self.rows.row_top(*first).unwrap_or(0);
        let last_bottom = self.rows.row_top(*last).unwrap_or(0) + self.rows.row(*last).height;
        let top_left = (start.0.min(p.0), start.1.min(first_top) + 1);
        let bottom_right = (start.0.max(p.0), start.1.max(last_bottom) - 2);
        self.g.drawing_area = IRect::from_points(top_left, bottom_right);
        self.dirty.overlay = true;
    }

    /// `StartDrawMode` and `MouseUp_DrawSelect`. The form's `DrawElement` is here too: above 10 ms makes one effect per row with default
    /// data, selected, with one undo entry. Times are not snapped.
    fn finish_drawing(&mut self, p: (i32, i32), host: &mut dyn TimelineHost) {
        let (start, duration) = clamp_draw_times(self.g.draw_down_time, self.time(p.0), self.info.total);
        let rows = self.rows.rows_within(self.g.drawing_area);
        if let Some(effect_type) = self.opts.selected_effect {
            if duration > TimeSpan::from_ms_i(10) {
                let mut added = Vec::new();
                for r in rows {
                    let (node, uuid, name) = {
                        let row = self.rows.row(r);
                        (row.node, row.node_uuid, row.name.clone())
                    };
                    let Some(id) = host.add_effect(effect_type, uuid, &name, start, duration) else { continue };
                    let desc = host.describe(id).unwrap_or_default();
                    self.add_element(id, ElementTimes::new(start, duration), node, desc);
                    added.push(id);
                }
                if !added.is_empty() {
                    host.record_undo(UndoAction::EffectsAdded { kind: crate::undo::AddRemoveKind::AddedOrRemoved, ids: added.clone() });
                    self.requests.push(HostRequest::UndoRecorded);
                    self.select_elements(&added);
                }
            }
        }
        if self.g.drawing_area.w < 2 && self.g.drawing_area.h < 2 {
            self.on_background_click(p);
        }
        self.g.drawing_start = (0, 0);
        self.g.drawing_area = IRect::default();
        self.end_all_drag();
    }

    /// `OnBackgroundClick`.
    fn on_background_click(&mut self, p: (i32, i32)) {
        if let Some(row) = self.rows.row_at(p.1) {
            self.set_row_active(row, true);
        }
        self.set_cursor(self.time(p.0));
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Rubber band
    // ------------------------------------------------------------------------------------------------------------------

    /// `beginDragSelect`.
    fn begin_drag_select(&mut self, p: (i32, i32), mods: Mods) {
        self.g.drag_state = DragState::Selecting;
        if !mods.shift && !self.selected_list().is_empty() {
            self.clear_selection();
        } else {
            self.g.temp_selected = self.selected_list();
        }
        let row = self.g.mouse_down_row;
        self.clear_selected_rows(row);
        if self.opts.legacy_cursor {
            self.clear_active_rows(row);
        }
        self.g.selection_area = IRect::new(p.0, p.1, 0, 0);
        self.g.selection_start = p;
    }

    /// `MouseMove_DragSelect`.
    fn mouse_move_drag_select(&mut self, p: (i32, i32), mods: Mods) {
        let start = self.g.selection_start;
        let top_left = (start.0.min(p.0), start.1.min(p.1));
        let bottom_right = (start.0.max(p.0), start.1.max(p.1));
        self.g.selection_area = IRect::from_points(top_left, bottom_right);
        self.select_elements_within(self.g.selection_area, true, mods);
        self.dirty.overlay = true;
    }

    /// `MouseUp_DragSelect`.
    fn mouse_up_drag_select(&mut self, p: (i32, i32)) {
        let area = self.g.selection_area;
        if area.w < 2 && area.h < 2 {
            // Didn't move, or not very far: just a click on the background.
            if !self.opts.legacy_cursor {
                let row = self.g.mouse_down_row;
                self.clear_active_rows(row);
            }
            self.on_background_click(p);
        } else {
            self.finalize_lasso();
        }
        self.g.selection_start = (0, 0);
        self.g.selection_area = IRect::default();
        self.g.temp_selected.clear();
        self.end_all_drag();
    }

    /// `FinalizeMoveCursorLassoSelection`.
    fn finalize_lasso(&mut self) {
        if self.opts.legacy_cursor {
            return;
        }
        if let Some(row) = self.g.mouse_down_row {
            self.clear_active_rows(Some(row));
            self.set_row_active(row, true);
        }
        let selected = self.selected_list();
        self.move_cursor_to_earliest(&selected);
    }

    /// `SelectElementsBetween`. F48. Replaces the selection, always crossing.
    fn select_elements_between(&mut self, a: (i32, i32), b: (i32, i32), mods: Mods) {
        self.clear_selected_rows(None);
        self.clear_active_rows(None);
        let rect = IRect::new(a.0.min(b.0), a.1.min(b.1), (a.0 - b.0).abs(), (a.1 - b.1).abs());
        self.select_elements_within(rect, false, mods);
    }

    /// `selectElementsWithin`. F51, F52, F53. Selection events fire once at the end, and the layers show the difference.
    pub(crate) fn select_elements_within(&mut self, area: IRect, use_cad: bool, mods: Mods) {
        if area.size_is_empty() {
            return;
        }
        let rows_in = self.rows.rows_in(area);
        let sel_start = self.time(area.x).ticks();
        let sel_end = self.time(area.right()).ticks();
        let sel_bottom = area.y + area.h;
        let enclosed = use_cad && self.opts.cad_style && area.x >= self.g.mouse_down_location.0;
        self.selection_green = enclosed;
        self.clear_selected_quietly();
        let keep: HashSet<ElementId> = if mods.shift { self.g.temp_selected.iter().copied().collect() } else { HashSet::new() };
        if mods.shift {
            for i in 0..self.g.temp_selected.len() {
                let id = self.g.temp_selected[i];
                self.set_selected(id, true);
            }
        }
        let filter = self.opts.drag_box_filter.enabled.then(|| self.opts.drag_box_filter.types.clone());
        let mut decisions: Vec<(ElementId, bool)> = Vec::new();
        for (r, top) in rows_in {
            self.ensure_slots(r, sel_start, sel_end);
            let row = self.rows.row(r);
            decisions.clear();
            for occ in &row.occurrences {
                if occ.start > sel_end {
                    break;
                }
                let (h, offset, _) = lane_geometry(occ.slot(), row.height);
                let occ_top = top + offset;
                let occ_bottom = occ_top + h;
                let inside = if enclosed {
                    occ.start > sel_start && occ.end < sel_end && occ_top > area.y && occ_bottom < sel_bottom
                } else {
                    occ.start < sel_end && occ.end > sel_start && occ_top < sel_bottom && occ_bottom > area.y
                };
                let passes = inside
                    && filter.as_ref().is_none_or(|types| self.elements.get(&occ.id).is_some_and(|e| types.contains(&e.desc.type_id)));
                decisions.push((occ.id, keep.contains(&occ.id) || passes));
            }
            // KEEP: an effect in two rows inside the rectangle takes the last row's answer.
            for i in 0..decisions.len() {
                let (id, selected) = decisions[i];
                self.set_selected(id, selected);
            }
        }
        self.raise_selection_changed();
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Moving effects
    // ------------------------------------------------------------------------------------------------------------------

    /// `waitForDragMove`.
    fn wait_for_drag_move(&mut self, p: (i32, i32)) {
        self.g.drag_state = DragState::Waiting;
        self.g.waiting_begin = p;
        self.g.ignore_drag_area = IRect::new(p.0 - DRAG_THRESHOLD, p.1 - DRAG_THRESHOLD, DRAG_THRESHOLD * 2, DRAG_THRESHOLD * 2);
    }

    /// `CloneSelectedElementsForMove`: the form's `CloneElements`, then selection moves to the clones. One undo entry. F60.
    fn clone_selected_for_move(&mut self, host: &mut dyn TimelineHost) {
        let selected = self.selected_list();
        let clones = self.clone_elements(&selected, host);
        self.clear_selected_quietly();
        for id in &clones {
            self.set_selected(*id, true);
        }
        self.raise_selection_changed();
    }

    /// `CloneElements`: same time, same last row, same layer. Skips an effect with no row, as upstream logs and skips.
    pub(crate) fn clone_elements(&mut self, ids: &[ElementId], host: &mut dyn TimelineHost) -> Vec<ElementId> {
        let mut clones = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(e) = self.elements.get(id) else { continue };
            let (start, duration, desc) = (e.start, e.duration, e.desc.clone());
            let Some(row) = e.last_row else { continue };
            let (node, uuid, name) = {
                let row = self.rows.row(row);
                (row.node, row.node_uuid, row.name.clone())
            };
            let Some(clone) = host.clone_effect(*id, uuid, &name, start, duration) else { continue };
            let desc = host.describe(clone).unwrap_or(desc);
            self.add_element(clone, ElementTimes::new(start, duration), node, desc);
            clones.push(clone);
        }
        // KEEP: recorded even with nothing cloned, like the other entries in Q35.
        host.mark_modified();
        host.record_undo(UndoAction::EffectsAdded { kind: crate::undo::AddRemoveKind::AddedOrRemoved, ids: clones.clone() });
        self.requests.push(HostRequest::UndoRecorded);
        clones
    }

    /// `beginDragMove`. Motion before crossing the threshold doesn't count, so the effect doesn't jump.
    fn begin_drag_move(&mut self, p: (i32, i32)) {
        self.g.drag_state = DragState::Moving;
        self.g.ignore_drag_area = IRect::default();
        self.cursor_kind = CursorKind::Move;
        self.g.current_row_index = self.rows.row_at(p.1).map_or(-1, |r| r.0 as i32);
        self.calculate_snap_points();
        self.elements_begin_move(p);
    }

    /// `calculateSnapPoints`: every unselected effect in every visible row. F69. Clones snap to their originals this way.
    fn calculate_snap_points(&mut self) {
        if !self.opts.snap_enabled {
            self.g.drag_snap = None;
            return;
        }
        let mut edges = Vec::new();
        for r in self.rows.visible() {
            for occ in &self.rows.row(*r).occurrences {
                if self.elements.get(&occ.id).is_some_and(|e| e.selected) {
                    continue;
                }
                edges.push((TimeSpan(occ.start), *r, SnapEdge::Start));
                edges.push((TimeSpan(occ.end), *r, SnapEdge::End));
            }
        }
        self.g.drag_snap = Some(DragSnap::new(edges, self.tpp(), self.opts.snap_strength));
    }

    /// `elementsBeginMove`: saves where the selection is and lifts it out of the tiles.
    fn elements_begin_move(&mut self, p: (i32, i32)) {
        self.g.modified_noted = false;
        let selected = self.selected_list();
        let mut info = MoveInfo { initial: p, visible_start: self.info.visible_start, ..MoveInfo::default() };
        info.earliest = TimeSpan::MAX;
        info.latest = TimeSpan::MIN;
        info.shortest = TimeSpan::MAX;
        self.g.move_info = Some(info);
        for id in selected {
            self.append_move_info(id);
        }
    }

    /// `AppendElementMoveInfo` and `BeginUpdate`.
    fn append_move_info(&mut self, id: ElementId) {
        let Some(e) = self.elements.get(&id) else { return };
        let orig = OriginalPlacement { start: e.start, duration: e.duration, row: e.last_row, target: e.target };
        let Some(info) = self.g.move_info.as_mut() else { return };
        if info.originals.insert(id, orig).is_some() {
            return;
        }
        info.order.push(id);
        info.earliest = info.earliest.min(orig.start);
        info.latest = info.latest.max(orig.start + orig.duration);
        info.shortest = info.shortest.min(orig.duration);
        self.set_lifted(id, true);
    }

    /// Moves an effect's occurrences between the tiles and their own layers. Either way, what the tiles show there has to repaint.
    pub(crate) fn set_lifted(&mut self, id: ElementId, lifted: bool) {
        let Some(e) = self.elements.get(&id) else { return };
        let rows = e.rows.clone();
        let (start, end) = (e.start.ticks(), e.end().ticks());
        for r in rows {
            let row = self.rows.row_mut(r);
            let at = row.find_occ(id, start, end).or_else(|| row.occurrences.iter().position(|o| o.id == id));
            if let Some(at) = at {
                row.occurrences[at].moving = lifted;
            }
            self.dirty_time_range_in_row(r, start, end);
        }
        self.dirty.overlay = true;
    }

    /// `MouseMove_DragMoving`. F58.
    fn mouse_move_drag_moving(&mut self, p: (i32, i32)) -> bool {
        let selected = self.selected_list();
        if selected.is_empty() {
            return false;
        }
        let Some(info) = self.g.move_info.as_ref() else { return false };
        let mut dt = self.time(p.0 - info.initial.0);
        let dy = p.1 - info.initial.1;
        if dt == TimeSpan::ZERO && dy == 0 {
            return false;
        }
        if info.earliest + dt < TimeSpan::ZERO {
            dt = TimeSpan::ZERO - info.earliest;
        }
        if info.latest + dt > self.info.total {
            dt = self.info.total - info.latest;
        }
        // KEEP Q7: snap comes after the clamp, so a snap can push a selection slightly out of bounds. The start setter clamps at 0.
        dt = self.find_snap_time(&selected, dt, ResizeZone::None);
        let mut changed = false;
        for id in &selected {
            let Some(orig) = self.g.move_info.as_ref().and_then(|m| m.originals.get(id)).copied() else { continue };
            let duration = self.elements.get(id).map_or(orig.duration, |e| e.duration);
            changed |= self.set_element_times(*id, orig.start + dt, duration, false);
        }
        self.flush_rows();
        if dy != 0 {
            let row_before = self.g.current_row_index;
            self.move_elements_vertically(&selected, p);
            changed |= row_before >= 0 && self.g.current_row_index != row_before;
        }
        self.dirty.overlay = true;
        changed
    }

    /// `MouseUp_DragMoving`.
    fn mouse_up_drag_moving(&mut self, p: (i32, i32), host: &mut dyn TimelineHost) {
        self.g.last_single_selected = p;
        self.elements_finished_moving(MoveKind::Moved, host);
        self.end_all_drag();
        self.g.drag_snap = None;
    }

    /// `elementsFinishedMoving`: everything commits and queues a render, then one undo entry, recorded even if nothing changed net. KEEP.
    fn elements_finished_moving(&mut self, kind: MoveKind, host: &mut dyn TimelineHost) {
        self.flush_rows();
        let Some(info) = self.g.move_info.take() else { return };
        let mut changes = Vec::with_capacity(info.order.len());
        let mut places = Vec::with_capacity(info.order.len());
        for id in &info.order {
            let Some(orig) = info.originals.get(id) else { continue };
            let Some(e) = self.elements.get(id) else { continue };
            changes.push((*id, e.start, e.duration));
            let target = |node: Option<NodeId>, tree: &RowTree| {
                node.and_then(|n| tree.rows_of_node(n).first()).map(|r| {
                    let row = tree.row(*r);
                    (row.node_uuid, row.name.clone())
                })
            };
            places.push(EffectPlacement { id: *id, start: orig.start, span: orig.duration, target: target(orig.target, &self.rows) });
        }
        host.commit_times(&changes);
        for id in &info.order {
            let (Some(orig), Some(e)) = (info.originals.get(id), self.elements.get(id)) else { continue };
            if e.target != orig.target {
                if let Some(row) = e.target.and_then(|n| self.rows.rows_of_node(n).first().copied()) {
                    let row = self.rows.row(row);
                    host.commit_target(*id, row.node_uuid, &row.name);
                }
            }
        }
        host.record_undo(UndoAction::PlacementChanged { kind, places });
        self.requests.push(HostRequest::UndoRecorded);
        host.render_changes();
        for id in &info.order {
            self.set_lifted(*id, false);
            self.update_picture_state(*id);
        }
        self.dirty.overlay = true;
    }

    /// A duration change waits for a render, and shows a placeholder until then. A plain move doesn't dirty an effect (F43).
    pub(crate) fn update_picture_state(&mut self, id: ElementId) {
        if let Some(e) = self.elements.get_mut(&id) {
            if e.duration != e.rendered_duration {
                e.rendered = false;
            }
        }
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Vertical moves
    // ------------------------------------------------------------------------------------------------------------------

    /// `MoveElementsVerticallyToLocation`. F61. Effects move between visible rows relative to the pointer. An effect in several rows is
    /// moved by its occurrence in the tracked row if it has one, and otherwise by the first it can.
    pub(crate) fn move_elements_vertically(&mut self, selected: &[ElementId], p: (i32, i32)) {
        let Some(dest) = self.rows.row_at(p.1) else { return };
        // GUARD Q38: the threshold was crossed below the last row, which upstream goes on to index at -1. Tracking starts here, and nothing moves this time.
        if self.g.current_row_index < 0 {
            self.g.current_row_index = dest.0 as i32;
            return;
        }
        if dest.0 as i32 == self.g.current_row_index {
            return;
        }
        // F62: a Deprecated row refuses effects, the same way a fresh drop is refused.
        if self.rows.row(dest).has_deprecated_tag() {
            return;
        }
        let Some(anchor) = self.g.mouse_down_row else { return };
        let current = RowId(self.g.current_row_index as u32);
        let (Some(dest_index), Some(current_index)) = (self.rows.visible_index_of(dest), self.rows.visible_index_of(current)) else { return };
        let mut rows_to_move = dest_index as i64 - current_index as i64;
        let visible_count = self.rows.visible().len() as i64;

        // Every occurrence in a visible row, as (visible index, order in the list).
        let mut occurrences: Vec<(usize, usize, ElementId)> = Vec::new();
        for (i, id) in selected.iter().enumerate() {
            let Some(e) = self.elements.get(id) else { continue };
            for r in &e.rows {
                if let Some(v) = self.rows.visible_index_of(*r) {
                    occurrences.push((v, i, *id));
                }
            }
        }
        occurrences.sort_unstable();
        let mut counts: IdMap<i32> = IdMap::default();
        for (_, _, id) in &occurrences {
            *counts.entry(*id).or_insert(0) += 1;
        }

        let anchor_visible = self.rows.visible_index_of(anchor);
        // `GetVerticalLimitRowForElements`: duplicates in other rows don't limit movement.
        let limit = |sequence: &mut dyn Iterator<Item = &(usize, usize, ElementId)>| -> Option<usize> {
            let mut left = counts.clone();
            for (v, _, id) in sequence {
                if Some(*v) == anchor_visible {
                    return Some(*v);
                }
                let c = left.entry(*id).or_insert(0);
                *c -= 1;
                if *c <= 0 {
                    return Some(*v);
                }
            }
            None
        };
        // GUARD Q39: with no moving effect in a visible row, upstream's limits are -1, and that turns any upward move into +1.
        let (Some(top_limit), Some(bottom_limit)) = (limit(&mut occurrences.iter()), limit(&mut bottom_up(&occurrences))) else { return };
        if rows_to_move < 0 && -rows_to_move > top_limit as i64 {
            rows_to_move = -(top_limit as i64);
        }
        if rows_to_move > 0 && rows_to_move > visible_count - 1 - bottom_limit as i64 {
            rows_to_move = visible_count - 1 - bottom_limit as i64;
        }
        if rows_to_move == 0 {
            return;
        }

        // Effects in the tracked row move by that occurrence only.
        let in_anchor: HashSet<ElementId> = self.rows.row(anchor).selected_elements.iter().copied().collect();
        let mut moved: HashSet<ElementId> = HashSet::with_capacity(selected.len());
        let mut left = counts;
        let mut new_anchor = anchor;
        let visible_rows: Vec<RowId> = self.rows.visible().to_vec();
        for (v, _, id) in &occurrences {
            let source = visible_rows[*v];
            if moved.contains(id) || (in_anchor.contains(id) && source != anchor) {
                continue;
            }
            let c = left.entry(*id).or_insert(0);
            *c -= 1;
            let target = *v as i64 + rows_to_move;
            // GUARD Q9: upstream throws when none remain. Skip this occurrence and try the next either way.
            if target < 0 || target >= visible_count {
                continue;
            }
            let destination = visible_rows[target as usize];
            let advances = source == anchor && in_anchor.contains(id);
            self.row_remove_element(source, *id);
            self.row_add_element(destination, *id);
            moved.insert(*id);
            self.element_changed_rows(*id, source, destination);
            if advances {
                new_anchor = destination;
            }
        }
        let new_current = visible_rows[(current_index as i64 + rows_to_move) as usize];
        self.g.current_row_index = new_current.0 as i32;
        self.g.mouse_down_row = Some(new_anchor);
        self.dirty.overlay = true;
    }

    /// `ElementChangedRowsHandler`: retarget, then remove from all other rows of the old node and add to all other rows of the new one.
    /// The target reaches the effect at commit.
    pub(crate) fn element_changed_rows(&mut self, id: ElementId, old_row: RowId, new_row: RowId) {
        let (old_node, new_node) = (self.rows.row(old_row).node, self.rows.row(new_row).node);
        if let Some(e) = self.elements.get_mut(&id) {
            e.target = new_node;
        }
        let mut affected: Vec<RowId> = Vec::new();
        for node in [old_node, new_node].into_iter().flatten() {
            affected.extend_from_slice(self.rows.rows_of_node(node));
        }
        affected.sort_unstable();
        affected.dedup();
        for r in affected {
            let node = self.rows.row(r).node;
            if node == old_node && r != old_row {
                self.row_remove_element(r, id);
            }
            if node == new_node && r != new_row {
                self.row_add_element(r, id);
            }
        }
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Resizing effects
    // ------------------------------------------------------------------------------------------------------------------

    /// `beginHResize`.
    fn begin_h_resize(&mut self, p: (i32, i32)) {
        self.g.drag_state = DragState::HResizing;
        self.g.current_row_index = self.rows.row_at(p.1).map_or(-1, |r| r.0 as i32);
        self.calculate_snap_points();
        self.elements_begin_move(p);
        self.g.adjoining = Some(Vec::new());
    }

    /// `MouseMove_HResizing`. F66, F67.
    fn mouse_move_h_resizing(&mut self, p: (i32, i32), mods: Mods) -> bool {
        let selected = self.selected_list();
        let Some(info) = self.g.move_info.as_ref() else { return false };
        let mut dt = self.time(p.0 - info.initial.0);
        if dt == TimeSpan::ZERO || (mods.opt && selected.len() > 1) {
            return false;
        }
        let zone = self.g.resize_zone;
        // Snap first, then the bounds, which override snap (F73).
        dt = self.find_snap_time(&selected, dt, zone);
        let Some(info) = self.g.move_info.as_ref() else { return false };
        // Only the selected count, not any neighbours since joined.
        let mut shortest = TimeSpan::MAX;
        let mut earliest = self.info.total;
        let mut latest = TimeSpan::ZERO;
        let mut selected_layer = 0;
        for id in &selected {
            let Some(o) = info.originals.get(id) else { continue };
            shortest = shortest.min(o.duration);
            earliest = earliest.min(o.start);
            latest = latest.max(o.start + o.duration);
            selected_layer = self.elements.get(id).map_or(0, |e| e.desc.layer_level);
        }
        let min_width = self.time(MIN_ELEMENT_WIDTH_PX);
        match zone {
            ResizeZone::Front => {
                if earliest + dt < TimeSpan::ZERO {
                    dt = TimeSpan::ZERO - earliest;
                }
                if shortest - dt < min_width {
                    dt = shortest - min_width;
                }
            }
            ResizeZone::Back => {
                if latest + dt > self.info.total {
                    dt = self.info.total - latest;
                }
                if shortest + dt < min_width {
                    dt = min_width - shortest;
                }
            }
            ResizeZone::None => {}
        }
        if mods.opt && self.g.adjoining_ids().is_empty() {
            if let Some(first) = selected.first().copied() {
                let found = self.adjoining(first, zone, mods.shift.then_some(selected_layer));
                self.g.adjoining = Some(found.clone());
                for id in found {
                    self.append_move_info(id);
                }
            }
        }
        let mut changed = false;
        for id in &selected {
            let Some(o) = self.g.move_info.as_ref().and_then(|m| m.originals.get(id)).copied() else { continue };
            let end = o.start + o.duration;
            match zone {
                ResizeZone::Front => {
                    let start = (o.start + dt).max(TimeSpan::ZERO);
                    changed |= self.set_element_times(*id, start, end - start, false);
                }
                ResizeZone::Back => {
                    let start = self.elements.get(id).map_or(o.start, |e| e.start);
                    changed |= self.set_element_times(*id, start, end + dt - start, false);
                }
                ResizeZone::None => {}
            }
        }
        if mods.opt {
            self.apply_adjoining(selected.first().copied(), zone, min_width);
        }
        self.flush_rows();
        self.dirty.overlay = true;
        changed
    }

    /// F67: neighbours follow the primary's edge, down to 10 pixels, at which point the primary stops. Releasing ⌥ stops them following,
    /// and they keep their state. KEEP.
    fn apply_adjoining(&mut self, primary: Option<ElementId>, zone: ResizeZone, min_width: TimeSpan) {
        let Some(primary) = primary else { return };
        let tpp = self.tpp();
        for id in self.g.adjoining_ids().to_vec() {
            let Some(neighbour) = self.elements.get(&id).map(|e| e.times()) else { continue };
            let Some(first) = self.elements.get(&primary).map(|e| e.times()) else { continue };
            match zone {
                ResizeZone::Front => {
                    let duration = first.start - neighbour.start;
                    if px(duration, tpp) <= MIN_ELEMENT_WIDTH_PX as f64 {
                        let neighbour_end = neighbour.start + min_width;
                        self.set_element_times(id, neighbour.start, min_width, false);
                        let orig_end = self.g.move_info.as_ref().and_then(|m| m.originals.get(&primary)).map(|o| o.start + o.duration);
                        let end = orig_end.unwrap_or(first.end());
                        self.set_element_times(primary, neighbour_end, end - neighbour_end, false);
                    } else {
                        self.set_element_times(id, neighbour.start, duration, false);
                    }
                }
                ResizeZone::Back => {
                    let saved_end = neighbour.end();
                    let start = first.end();
                    let duration = saved_end - start;
                    if px(duration, tpp) <= MIN_ELEMENT_WIDTH_PX as f64 {
                        let start = saved_end - min_width;
                        self.set_element_times(id, start, min_width, false);
                        self.set_element_times(primary, first.start, start - first.start, false);
                    } else {
                        self.set_element_times(id, start, duration, false);
                    }
                }
                ResizeZone::None => {}
            }
        }
    }

    /// `MouseUp_HResizing`.
    fn mouse_up_h_resizing(&mut self, host: &mut dyn TimelineHost) {
        self.elements_finished_moving(MoveKind::Resize, host);
        self.end_all_drag();
        self.g.drag_snap = None;
        self.g.adjoining = None;
    }

    /// `GetPriorsOfElement` or `GetFollowersOfElement` and the search around them: the neighbours whose edge meets this one's, from the
    /// effect's last row. F67. `same_layer` is the ⌥⇧ form.
    pub fn adjoining(&self, primary: ElementId, zone: ResizeZone, same_layer: Option<i32>) -> Vec<ElementId> {
        let Some(e) = self.elements.get(&primary) else { return Vec::new() };
        let Some(r) = e.last_row else { return Vec::new() };
        let row = self.rows.row(r);
        let (start, end) = (e.start.ticks(), e.end().ticks());
        let Some(at) = row.occurrences.iter().position(|o| o.id == primary) else { return Vec::new() };
        let layer_ok = |id: ElementId| same_layer.is_none_or(|l| self.elements.get(&id).is_some_and(|n| n.desc.layer_level == l));
        let in_change = |id: ElementId| self.g.is_moving(id);
        let mut out = Vec::new();
        match zone {
            ResizeZone::Front => {
                // `IndexOfElement(SearchBy.EndTime)`: the last entry before one ending later than this.
                let mut index = at + 1;
                while index < row.occurrences.len() && row.occurrences[index].end <= end {
                    index += 1;
                }
                let from = index - 1;
                let priors: Vec<&super::Occ> = row.occurrences[..=from].iter().rev().take(ADJOINING_LIMIT).filter(|o| o.end <= start).collect();
                let mut latest = 0i64;
                for o in &priors {
                    if o.end > latest && layer_ok(o.id) {
                        latest = o.end;
                    }
                }
                for o in priors {
                    if o.end == latest && layer_ok(o.id) && !in_change(o.id) {
                        out.push(o.id);
                    }
                }
            }
            ResizeZone::Back => {
                let followers: Vec<&super::Occ> = row.occurrences[at + 1..].iter().take(ADJOINING_LIMIT).filter(|o| o.start >= end).collect();
                let mut earliest = i64::MAX;
                for o in &followers {
                    if o.start < earliest && layer_ok(o.id) {
                        earliest = o.start;
                    }
                }
                for o in followers {
                    if o.start == earliest && layer_ok(o.id) && !in_change(o.id) {
                        out.push(o.id);
                    }
                }
            }
            ResizeZone::None => {}
        }
        out
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Snapping
    // ------------------------------------------------------------------------------------------------------------------

    /// F71 step 2. KEEP Q11: `tracked_row` above 0 checks only the row under the pointer, but flat row 0 acts unscoped, because upstream's
    /// test is `> 0`. Times are each effect's original ones; an effect with none saved is skipped.
    pub fn snap_candidates(&self, selected: &[ElementId], tracked_row: i32) -> Vec<(ElementTimes, RowId)> {
        let Some(info) = self.g.move_info.as_ref() else { return Vec::new() };
        let mut out = Vec::with_capacity(selected.len());
        for id in selected {
            let (Some(e), Some(o)) = (self.elements.get(id), info.originals.get(id)) else { continue };
            let times = ElementTimes::new(o.start, o.duration);
            if tracked_row > 0 {
                let target = RowId(tracked_row as u32);
                if e.rows.binary_search(&target).is_ok() {
                    out.push((times, target));
                }
            } else if let Some(first) = e.rows.first() {
                out.push((times, *first));
            }
        }
        out
    }

    /// `FindSnapTimeForElements`.
    pub(crate) fn find_snap_time(&self, selected: &[ElementId], offset: TimeSpan, zone: ResizeZone) -> TimeSpan {
        if offset == TimeSpan::ZERO || !self.opts.snap_enabled {
            return offset;
        }
        let Some(snap) = self.g.drag_snap.as_ref() else { return offset };
        let candidates = self.snap_candidates(selected, self.g.current_row_index);
        find_snap(snap, &self.snap_index, &candidates, offset, zone)
    }
}

/// The distance past an edge's margin, signed. `m_mouseOutside`. F63.
#[inline]
pub(crate) fn edge_distance(v: i32, size: i32) -> i32 {
    if v <= AUTOSCROLL_MARGIN {
        -(AUTOSCROLL_MARGIN - v)
    } else if v > size - AUTOSCROLL_MARGIN {
        v - size + AUTOSCROLL_MARGIN
    } else {
        0
    }
}

/// Bottom row first, but within a row still in list order, as the original's nested loops go.
fn bottom_up(occurrences: &[(usize, usize, ElementId)]) -> impl Iterator<Item = &(usize, usize, ElementId)> {
    let mut indices: Vec<usize> = (0..occurrences.len()).collect();
    indices.sort_by(|a, b| occurrences[*b].0.cmp(&occurrences[*a].0).then(occurrences[*a].1.cmp(&occurrences[*b].1)));
    indices.into_iter().map(move |i| &occurrences[i])
}
