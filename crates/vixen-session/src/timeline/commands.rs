//! Commands on effects, and what wheels and pinches do: F11 to F17, F80 to F90.

use super::host::TimelineHost;
use super::model::{HostRequest, Timeline};
use super::view::{px, scale_span, tilt_pan, vertical_wheel_delta, wheel_pan, wheel_zoom_scale};
use super::{ElementId, Mods, RowId};
use crate::undo::{AddRemoveKind, MoveKind, UndoAction};
use vixen_core::TimeSpan;

/// The warning `OkToUseAlignmentHelper` leads to. KEEP Q16: it says 4, though the limit is 32.
pub const ALIGNMENT_WARNING: &str = "Too many effects selected on the same row for this action.\nMax selected effects per row for this action is 4";
/// `OkToUseAlignmentHelper`.
const ALIGNMENT_ROW_LIMIT: usize = 32;

/// F80.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AlignKind {
    Start,
    End,
    Both,
    Durations,
    StartToEnd,
    EndToStart,
    Centers,
}

impl AlignKind {
    fn move_kind(self) -> MoveKind {
        match self {
            AlignKind::Start => MoveKind::StartAlign,
            AlignKind::End => MoveKind::EndAlign,
            AlignKind::Both => MoveKind::Align,
            AlignKind::Durations => MoveKind::DurationAlign,
            AlignKind::StartToEnd => MoveKind::StartToEndAlign,
            AlignKind::EndToStart => MoveKind::EndToStartAlign,
            AlignKind::Centers => MoveKind::CenterAlign,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AlignResult {
    Done,
    /// No reference: nothing happens (`GridAlignmentNullReferenceTests`).
    NoReference,
    /// Show the warning instead.
    TooManyPerRow,
}

/// Which surface a wheel event is over. F11, F12.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Surface {
    Grid,
    Ruler,
    Waveform,
    RowList,
    MarksBar,
}

/// A scroll wheel or trackpad event. Deltas are `scrollingDeltaX` and `scrollingDeltaY`: points when `precise`, lines otherwise.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ScrollEvent {
    pub surface: Surface,
    pub dx: f64,
    pub dy: f64,
    pub precise: bool,
    pub mods: Mods,
    /// The pointer's distance from the splitter, for zoom under the pointer.
    pub pointer_offset_x: i32,
    /// The pointer's y in the grid's client coordinates, for the row height gesture.
    pub grid_y: i32,
}

impl Timeline {
    // ------------------------------------------------------------------------------------------------------------------
    // Align
    // ------------------------------------------------------------------------------------------------------------------

    /// `OkToUseAlignmentHelper`: false if any effect's last row has more than 32 selected.
    pub fn ok_to_use_alignment(&self, elements: &[ElementId]) -> bool {
        elements.iter().all(|id| {
            let row = self.elements.get(id).and_then(|e| e.last_row);
            row.is_none_or(|r| self.rows.row(r).selected_elements.len() <= ALIGNMENT_ROW_LIMIT)
        })
    }

    /// F80. `hold` is ⇧ on the commands that have that form.
    pub fn align(
        &mut self,
        kind: AlignKind,
        elements: &[ElementId],
        reference: Option<ElementId>,
        hold: bool,
        host: &mut dyn TimelineHost,
    ) -> AlignResult {
        let Some(reference) = reference.and_then(|r| self.elements.get(&r)).map(|r| r.times()) else {
            return AlignResult::NoReference;
        };
        if !self.ok_to_use_alignment(elements) {
            self.requests.push(HostRequest::Warning { text: ALIGNMENT_WARNING.to_string(), title: "Warning".to_string() });
            return AlignResult::TooManyPerRow;
        }
        let (r_start, r_end, r_duration) = (reference.start, reference.end(), reference.duration);
        let total = self.info.total;
        let min_gap = TimeSpan::from_secs(0.05);
        let mut moves = Vec::with_capacity(elements.len());
        for id in elements {
            let Some(e) = self.elements.get(id) else { continue };
            let (s, en, d) = (e.start, e.end(), e.duration);
            let new = match kind {
                AlignKind::Start => {
                    if s == r_start {
                        continue;
                    }
                    if en <= r_start || hold {
                        (r_start, r_start + d)
                    } else {
                        (r_start, en)
                    }
                }
                AlignKind::End => {
                    if en == r_end {
                        continue;
                    }
                    if s >= r_end || hold {
                        (r_end - d, r_end)
                    } else {
                        (s, r_end)
                    }
                }
                AlignKind::Both => {
                    if s == r_start && en == r_end {
                        continue;
                    }
                    (r_start, r_end)
                }
                AlignKind::Durations => {
                    if d == r_duration {
                        continue;
                    }
                    if hold {
                        (en - r_duration, en)
                    } else {
                        (s, s + r_duration)
                    }
                }
                AlignKind::StartToEnd => {
                    if en == r_end {
                        continue;
                    }
                    let start = r_end;
                    let mut end = if hold { en } else { start + d };
                    if end - start < min_gap {
                        end = start + d;
                    }
                    if end > total {
                        end = total;
                    }
                    (start, end)
                }
                AlignKind::EndToStart => {
                    if s == r_start {
                        continue;
                    }
                    let end = r_start;
                    let mut start = if hold { s } else { end - d };
                    if end - start < min_gap {
                        start = end - d;
                    }
                    if start < TimeSpan::ZERO {
                        start = TimeSpan::ZERO;
                    }
                    (start, end)
                }
                AlignKind::Centers => {
                    // KEEP Q15: this skips on an equal start, where it should arguably test centres.
                    if s == r_start {
                        continue;
                    }
                    let center = r_start.total_secs() + r_duration.total_secs() / 2.0;
                    let start = TimeSpan::from_secs(center - d.total_secs() / 2.0);
                    (start, start + d)
                }
            };
            moves.push((*id, new.0, new.1));
        }
        self.move_resize_batch(&moves, kind.move_kind(), host);
        AlignResult::Done
    }

    /// `MoveResizeElements`: `(effect, new start, new end)` as one action. An entry past the total or under 1 ms is skipped, and still
    /// counts in the single undo entry.
    pub fn move_resize_batch(&mut self, moves: &[(ElementId, TimeSpan, TimeSpan)], kind: MoveKind, host: &mut dyn TimelineHost) {
        let one_ms = TimeSpan::from_ms_i(1);
        let mut before = Vec::with_capacity(moves.len());
        let mut changes = Vec::with_capacity(moves.len());
        for (id, start, end) in moves {
            let Some(e) = self.elements.get(id) else { continue };
            before.push((*id, e.start, e.duration));
            if *end > self.info.total || *end - *start < one_ms {
                continue;
            }
            // A start below 0 isn't skipped: the setter makes it 0, and then the end sets the duration.
            let start = (*start).max(TimeSpan::ZERO);
            self.set_element_times(*id, start, *end - start, true);
            if let Some(e) = self.elements.get(id) {
                changes.push((*id, e.start, e.duration));
            }
        }
        self.flush_rows();
        host.commit_times(&changes);
        host.record_undo(UndoAction::TimesChanged { kind, times: before });
        self.requests.push(HostRequest::UndoRecorded);
        host.render_changes();
        for (id, ..) in &changes {
            self.update_picture_state(*id);
        }
        self.dirty.overlay = true;
    }

    /// `MoveResizeElement`: refuses a start or end past the total or a duration under 1 ms.
    pub fn move_resize_element(&mut self, id: ElementId, start: TimeSpan, duration: TimeSpan, kind: MoveKind, host: &mut dyn TimelineHost) -> bool {
        if !self.elements.contains_key(&id) || start > self.info.total || start + duration > self.info.total || duration < TimeSpan::from_ms_i(1) {
            return false;
        }
        let Some(e) = self.elements.get(&id) else { return false };
        let before = vec![(id, e.start, e.duration)];
        self.set_element_times(id, start, duration, true);
        self.flush_rows();
        if let Some(e) = self.elements.get(&id) {
            host.commit_times(&[(id, e.start, e.duration)]);
        }
        host.record_undo(UndoAction::TimesChanged { kind, times: before });
        self.requests.push(HostRequest::UndoRecorded);
        host.render_changes();
        self.update_picture_state(id);
        self.dirty.overlay = true;
        true
    }

    /// `MoveResizeElementByStartEnd`: also refuses `start >= end`.
    pub fn move_resize_element_by_start_end(&mut self, id: ElementId, start: TimeSpan, end: TimeSpan, host: &mut dyn TimelineHost) -> bool {
        if start > self.info.total || end > self.info.total || start >= end {
            return false;
        }
        self.move_resize_element(id, start, end - start, MoveKind::Moved, host)
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Split, close gaps, move by time
    // ------------------------------------------------------------------------------------------------------------------

    /// `SplitElementsAtTime`. F84. KEEP Q17: one "added" entry plus one move entry per effect, per half.
    pub fn split_at(&mut self, elements: &[ElementId], t: TimeSpan, host: &mut dyn TimelineHost) {
        let clones = self.clone_elements(elements, host);
        for id in elements {
            let Some(start) = self.elements.get(id).map(|e| e.start) else { continue };
            self.move_resize_element(*id, start, t - start, MoveKind::Moved, host);
        }
        for id in &clones {
            let Some(end) = self.elements.get(id).map(|e| e.end()) else { continue };
            self.move_resize_element(*id, t, end - t, MoveKind::Moved, host);
        }
    }

    /// `SplitSelectedElementsAtMouseLocation`: `pointer_time` must be strictly inside the visible range.
    pub fn split_selected_at(&mut self, pointer_time: TimeSpan, host: &mut dyn TimelineHost) {
        let selected = self.selected_list();
        if selected.is_empty() {
            return;
        }
        if self.info.visible_start < pointer_time && pointer_time < self.vis_end() {
            let inside: Vec<ElementId> = selected
                .into_iter()
                .filter(|id| self.elements.get(id).is_some_and(|e| e.start < pointer_time && e.end() > pointer_time))
                .collect();
            self.split_at(&inside, pointer_time, host);
        }
    }

    /// `CloseGapsBetweenElements`. F85. Returns false when nothing is selected and the whole sequence would be affected, which needs
    /// the user's say first; call again with `confirmed`.
    pub fn close_gaps(&mut self, threshold_seconds: f64, confirmed: bool, host: &mut dyn TimelineHost) -> bool {
        let any_selected = !self.selected_list().is_empty();
        if !any_selected && !confirmed {
            return false;
        }
        let rows: Vec<RowId> = if any_selected { self.rows.visible().to_vec() } else { self.rows.rows.iter().map(|r| r.id).collect() };
        let mut moves: Vec<(ElementId, TimeSpan, TimeSpan)> = Vec::new();
        let mut queued = std::collections::HashSet::new();
        for r in rows {
            let row = self.rows.row(r);
            let list: Vec<ElementId> = if any_selected { row.selected_elements.clone() } else { row.occurrences.iter().map(|o| o.id).collect() };
            let Some(first) = list.first() else { continue };
            let Some(mut active_end) = self.elements.get(first).map(|e| e.end()) else { continue };
            for id in list.iter().skip(1) {
                let Some(e) = self.elements.get(id) else { continue };
                // The same effect can be in more than one row, and only needs changing once.
                if e.start.total_secs() - active_end.total_secs() < threshold_seconds && e.start != active_end && !queued.contains(id) {
                    queued.insert(*id);
                    moves.push((*id, active_end, e.end()));
                }
                active_end = e.end();
            }
        }
        self.move_resize_batch(&moves, MoveKind::Moved, host);
        true
    }

    /// `MoveElementsByTime`. F87. Returns the offset used. No snapping.
    ///
    /// KEEP: clipping takes an effect shifted entirely out of the sequence off its last row and nothing more, though upstream's comment says "delete it".
    /// It stays in the sequence at its old time, so it keeps playing and comes back on reopen, and shows in any other rows it has.
    pub fn move_by_time(&mut self, elements: &[ElementId], offset: TimeSpan, clip: bool, host: &mut dyn TimelineHost) -> TimeSpan {
        if elements.is_empty() {
            return TimeSpan::ZERO;
        }
        let mut offset = offset;
        let total = self.info.total;
        if !clip {
            let earliest = elements.iter().filter_map(|id| self.elements.get(id)).map(|e| e.start).min().unwrap_or(TimeSpan::ZERO);
            if earliest + offset < TimeSpan::ZERO {
                offset = TimeSpan::ZERO - earliest;
            }
            let latest = elements.iter().filter_map(|id| self.elements.get(id)).map(|e| e.end()).max().unwrap_or(TimeSpan::ZERO);
            if latest + offset > total {
                offset = total - latest;
            }
        }
        let mut before = Vec::with_capacity(elements.len());
        let mut changes = Vec::with_capacity(elements.len());
        let mut clipped = Vec::new();
        for id in elements {
            let Some(e) = self.elements.get(id) else { continue };
            let (start, end, duration) = (e.start, e.end(), e.duration);
            if clip && (end + offset < TimeSpan::ZERO || start + offset > total) {
                if let Some(row) = e.last_row {
                    self.row_remove_element(row, *id);
                }
                clipped.push(*id);
                continue;
            }
            before.push((*id, start, duration));
            let (new_start, new_duration) = if start + offset < TimeSpan::ZERO {
                // KEEP Q18: `duration += 0 + offset`, because start was already zeroed.
                (TimeSpan::ZERO, duration + offset)
            } else if end + offset > total {
                (start + offset, total - (start + offset))
            } else {
                (start + offset, duration)
            };
            self.set_element_times(*id, new_start, new_duration, true);
            if let Some(e) = self.elements.get(id) {
                changes.push((*id, e.start, e.duration));
            }
        }
        self.flush_rows();
        host.commit_times(&changes);
        let moved = before.len() + clipped.len();
        let times = UndoAction::TimesChanged { kind: MoveKind::Moved, times: before };
        if clipped.is_empty() {
            host.record_undo(times);
        } else {
            let label = format!("Moved {moved} effect{}", if moved == 1 { "" } else { "s" });
            host.record_undo(UndoAction::Group { label, actions: vec![times, UndoAction::RowsRestored { ids: clipped }] });
        }
        self.requests.push(HostRequest::UndoRecorded);
        host.render_changes();
        for (id, ..) in &changes {
            self.update_picture_state(*id);
        }
        self.dirty.overlay = true;
        offset
    }

    /// `MoveElementsInRangeByTime`. F86.
    pub fn move_in_range_by_time(
        &mut self,
        start: TimeSpan,
        end: TimeSpan,
        offset: TimeSpan,
        visible_only: bool,
        clip: bool,
        host: &mut dyn TimelineHost,
    ) -> TimeSpan {
        let elements = self.elements_within(start, end, visible_only);
        self.move_by_time(&elements, offset, clip, host)
    }

    /// `MoveSelectedElementsByTime` with two pixels' worth of time. F87: ⌘← and ⌘→.
    pub fn nudge_selected(&mut self, direction: i32, host: &mut dyn TimelineHost) {
        let selected = self.selected_list();
        let offset = TimeSpan(self.tpp().ticks() * if direction < 0 { -2 } else { 2 });
        self.move_by_time(&selected, offset, false, host);
    }

    /// `DisplaySelectedEffects`. F90: selects them, opens every ancestor, and scrolls so the last is in view.
    pub fn reveal_elements(&mut self, elements: &[ElementId]) {
        let Some(last) = elements.last().copied() else { return };
        for id in elements {
            self.set_selected(*id, true);
            let Some(row) = self.elements.get(id).and_then(|e| e.last_row) else { continue };
            self.rows.set_visible(row, true);
            let mut parent = self.rows.row(row).parent;
            while let Some(p) = parent {
                self.rows.set_tree_open(p, true);
                self.rows.set_visible(p, true);
                parent = self.rows.row(p).parent;
            }
        }
        self.rows.relayout();
        self.rows_changed();
        if let Some(e) = self.elements.get(&last) {
            let (start, row) = (e.start, e.last_row);
            self.set_visible_start(start);
            if let Some(top) = row.and_then(|r| self.rows.row_top(r)) {
                self.set_vertical_offset(top);
            }
        }
        self.raise_selection_changed();
    }

    // ------------------------------------------------------------------------------------------------------------------
    // Wheels and pinches
    // ------------------------------------------------------------------------------------------------------------------

    /// F11 to F14.
    pub fn scroll_wheel(&mut self, ev: ScrollEvent) {
        let mods = ev.mods;
        if ev.precise {
            self.scroll_precise(ev);
            return;
        }
        // Tilt: F13, on every time surface because the guide wants gestures to work everywhere. Upstream's delta is positive to the right,
        // where `scrollingDeltaX` is negative.
        if ev.dy == 0.0 && ev.dx != 0.0 && !mods.shift {
            if ev.surface != Surface::RowList {
                let delta = (-ev.dx * 120.0) as i32;
                self.set_visible_start(tilt_pan(self.info.visible_start, self.span(), delta));
            }
            return;
        }
        // macOS turns a wheel with ⇧ held into horizontal deltas, so take whichever axis isn't zero.
        let lines = if mods.shift && ev.dy == 0.0 { ev.dx } else { ev.dy };
        let delta = (lines * 120.0) as i32;
        self.apply_wheel_delta(ev, delta, false);
    }

    /// F14: precise deltas move a point per point. The ⌘ forms convert to a wheel delta.
    fn scroll_precise(&mut self, ev: ScrollEvent) {
        let mods = ev.mods;
        let tpp = self.tpp().ticks();
        let time_of = |points: f64| TimeSpan((points * tpp as f64) as i64);
        if mods.cmd {
            let delta = (ev.dy * 10.0) as i32;
            self.apply_wheel_delta(ev, delta, true);
            return;
        }
        if mods.shift {
            let delta = if ev.dy != 0.0 { ev.dy } else { ev.dx };
            self.set_visible_start(self.info.visible_start - time_of(delta));
            return;
        }
        if ev.dx != 0.0 {
            self.set_visible_start(self.info.visible_start - time_of(ev.dx));
        }
        if ev.dy != 0.0 && ev.surface != Surface::MarksBar {
            self.vertical_remainder -= ev.dy;
            let whole = self.vertical_remainder.trunc();
            if whole != 0.0 {
                self.vertical_remainder -= whole;
                self.set_vertical_offset(self.vertical_offset + whole as i32);
            }
        }
    }

    /// F11's table, and F12's for the marks bar. `accumulate` is the trackpad's form, where row heights step each time the delta passes 120.
    fn apply_wheel_delta(&mut self, ev: ScrollEvent, delta: i32, accumulate: bool) {
        let mods = ev.mods;
        if ev.surface == Surface::MarksBar {
            // Only navigation: plain wheel doesn't scroll rows, ⌘⌥ doesn't change a row height. ⇧ wins.
            if mods.shift {
                self.set_visible_start(wheel_pan(self.info.visible_start, self.span(), delta));
            } else if mods.cmd && !mods.opt {
                self.zoom_by_wheel(delta, ev.pointer_offset_x);
            }
            return;
        }
        if mods.cmd && mods.opt {
            if let Some(step) = self.row_step(delta, accumulate) {
                let y = self.to_canvas((0, ev.grid_y)).1;
                if self.rows.zoom_row_at(y, step) {
                    self.rows_changed();
                    self.mark_editor_state_modified();
                }
            }
        } else if mods.cmd && mods.shift {
            if let Some(step) = self.row_step(delta, accumulate) {
                self.zoom_rows(if step < 0 { 0.8 } else { 1.25 });
            }
        } else if mods.cmd {
            self.zoom_by_wheel(delta, ev.pointer_offset_x);
        } else if mods.shift {
            self.set_visible_start(wheel_pan(self.info.visible_start, self.span(), delta));
        } else {
            self.set_vertical_offset(self.vertical_offset + vertical_wheel_delta(delta));
        }
    }

    fn row_step(&mut self, delta: i32, accumulate: bool) -> Option<i32> {
        if !accumulate {
            return Some(delta);
        }
        self.wheel_accumulator += delta;
        if self.wheel_accumulator.abs() >= 120 {
            let step = self.wheel_accumulator;
            self.wheel_accumulator = 0;
            Some(step)
        } else {
            None
        }
    }

    /// `ZoomTimelineHorizontally`.
    fn zoom_by_wheel(&mut self, delta: i32, pointer_offset_x: i32) {
        let scale = wheel_zoom_scale(delta);
        if self.opts.zoom_under_mouse {
            self.zoom_time_at(pointer_offset_x, scale);
        } else {
            self.zoom_time(scale);
        }
    }

    /// F17: pinch, always through the pointer anchor whatever the menu option says, because a pinch is expected to centre on the fingers.
    /// With ⇧ it scales all rows, at each accumulated factor of 1.25 or 0.8.
    pub fn magnify(&mut self, magnification: f64, pointer_offset_x: i32, mods: Mods) {
        if mods.shift {
            self.pinch_rows *= 1.0 + magnification;
            if self.pinch_rows >= 1.25 {
                self.pinch_rows = 1.0;
                self.zoom_rows(1.25);
            } else if self.pinch_rows <= 0.8 {
                self.pinch_rows = 1.0;
                self.zoom_rows(0.8);
            }
            return;
        }
        if 1.0 + magnification <= 0.0 {
            return;
        }
        self.zoom_time_at(pointer_offset_x, 1.0 / (1.0 + magnification));
    }

    /// F95. Display rate, on playback ticks. A page flip is the only thing that dirties content.
    pub fn follow_playback(&mut self, position: TimeSpan) {
        // A paused position doesn't change, so the view can be scrolled freely while paused (`TimeInfo.cs:95-96`).
        if self.info.playback_current == Some(position) {
            return;
        }
        self.info.playback_current = Some(position);
        if let Some(start) = super::view::playback_follow(position, self.info.visible_start, self.span()) {
            self.set_visible_start(start);
        }
    }

    /// Playback stopped: the line and arrow go.
    pub fn clear_playback_position(&mut self) {
        self.info.playback_current = None;
    }

    /// The horizontal scroller: `VisibleTimeStart = time(x)`, so the bar snaps time to whole pixels. F7.
    pub fn set_scroll_x(&mut self, x: i32) {
        self.set_visible_start(self.time(x));
    }

    /// F105's Start and End buttons. End scrolls to the last view.
    pub fn scroll_to_start(&mut self) {
        self.set_visible_start(TimeSpan::ZERO);
    }

    pub fn scroll_to_end(&mut self) {
        self.set_visible_start(self.info.total - self.span());
    }

    /// PageUp, PageDown and their ⌘ forms: half a view (F149).
    pub fn page_vertical(&mut self, direction: i32) {
        self.set_vertical_offset(self.vertical_offset + direction * self.grid_height / 2);
    }

    pub fn page_time(&mut self, direction: i32) {
        let half = scale_span(self.span(), 0.5);
        self.set_visible_start(if direction < 0 { self.info.visible_start - half } else { self.info.visible_start + half });
    }

    /// F111's ⇧Space and friends need the position in pixels without another call.
    pub fn time_to_px(&self, t: TimeSpan) -> f64 {
        px(t, self.tpp())
    }
}

impl Timeline {
    /// `RemoveSelectedElements` behind ⌫: one undo entry, "Removed N effects". F134's rule that this runs first, when any effect is selected,
    /// is the caller's.
    pub fn delete_selected_effects(&mut self, host: &mut dyn TimelineHost) -> bool {
        let selected = self.selected_list();
        if selected.is_empty() {
            return false;
        }
        let snapshots = host.remove_effects(&selected);
        for id in &selected {
            self.remove_element(*id);
        }
        host.record_undo(UndoAction::EffectsRemoved { kind: AddRemoveKind::AddedOrRemoved, effects: snapshots });
        self.requests.push(HostRequest::UndoRecorded);
        self.raise_selection_changed();
        true
    }
}
