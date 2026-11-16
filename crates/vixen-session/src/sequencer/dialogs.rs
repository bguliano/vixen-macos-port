//! What the window's dialogs do between their fields: `EffectTimeEditor` and `Form_AddMultipleEffects`. Sections L and H of the spec. Swift holds the state between calls and shows it.

use crate::sequencer::effect_ops::{possible_effects, possible_effects_on_marks, time_exists_for_addition};
use parking_lot::Mutex;
use vixen_core::{MarkCollection, TimeSpan};

/// `EffectTimeEditor`: start, duration and end move together, F379. `TimeControl` clamps every value it's given to its minimum and maximum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectTimeEditorState {
    pub start: TimeSpan,
    pub duration: TimeSpan,
    pub end: TimeSpan,
    pub length: TimeSpan,
    pub start_max: TimeSpan,
    pub duration_min: TimeSpan,
    pub duration_max: TimeSpan,
    pub end_min: TimeSpan,
    pub end_max: TimeSpan,
}

fn clamp(t: TimeSpan, min: TimeSpan, max: TimeSpan) -> TimeSpan {
    if t > max {
        max
    } else if t < min {
        min
    } else {
        t
    }
}

impl EffectTimeEditorState {
    pub fn new(start: TimeSpan, duration: TimeSpan, length: TimeSpan, minimum: TimeSpan) -> EffectTimeEditorState {
        EffectTimeEditorState {
            start,
            duration,
            end: start + duration,
            length,
            start_max: length - minimum,
            duration_min: minimum,
            duration_max: length - start,
            end_min: start + minimum,
            end_max: length,
        }
    }

    /// `Time_Changed` with the start time as sender.
    pub fn set_start(&mut self, start: TimeSpan) {
        self.start = clamp(start, TimeSpan::ZERO, self.start_max);
        self.end = clamp(self.start + self.duration, self.end_min, self.end_max);
        if self.start + self.duration > self.length {
            self.duration = clamp(self.length - self.start, self.duration_min, self.duration_max);
        }
        self.duration_max = self.length - self.start;
        self.end_min = self.start + self.duration_min;
    }

    pub fn set_duration(&mut self, duration: TimeSpan) {
        self.duration = clamp(duration, self.duration_min, self.duration_max);
        self.end = clamp(self.start + self.duration, self.end_min, self.end_max);
    }

    pub fn set_end(&mut self, end: TimeSpan) {
        self.end = clamp(end, self.end_min, self.end_max);
        self.duration = clamp(self.end - self.start, self.duration_min, self.duration_max);
    }

    /// `btnSetFullSequence_Click`: "Match sequence start/end".
    pub fn match_sequence(&mut self) {
        self.set_start(TimeSpan::ZERO);
        self.set_end(self.length);
        self.set_duration(self.length);
    }
}

/// `_properties` for Add Multiple: a static class, so shared between editor windows. F291.
#[derive(Clone, Debug, PartialEq)]
pub struct AddMultipleMemory {
    pub start: TimeSpan,
    pub end: TimeSpan,
    pub duration: TimeSpan,
    pub between: TimeSpan,
    pub align_to_marks: bool,
    pub skip_every_other: bool,
    pub fill: bool,
    pub mark_start_end: bool,
    pub select_effects: bool,
    pub panel_open: bool,
    pub checked_marks: Vec<String>,
}

static MEMORY: Mutex<Option<AddMultipleMemory>> = Mutex::new(None);

#[cfg(test)]
static MEMORY_IN_TEST: Mutex<()> = Mutex::new(());

/// Forgets the process-wide values, for tests. Holding the guard keeps other tests off them.
#[cfg(test)]
pub fn clear_add_multiple_memory() -> parking_lot::MutexGuard<'static, ()> {
    let guard = MEMORY_IN_TEST.lock();
    *MEMORY.lock() = None;
    guard
}

/// `Form_AddMultipleEffects`' fields, and what they've decided so far.
#[derive(Clone, Debug, PartialEq)]
pub struct AddMultipleState {
    pub effect_name: String,
    pub length: TimeSpan,
    pub start: TimeSpan,
    pub end: TimeSpan,
    pub duration: TimeSpan,
    pub between: TimeSpan,
    pub count: i32,
    pub count_max: i32,
    pub ok_enabled: bool,
    pub align_to_marks: bool,
    pub skip_every_other: bool,
    pub fill: bool,
    pub mark_start_end: bool,
    pub select_effects: bool,
    pub panel_open: bool,
    pub checked_marks: Vec<String>,
    pub duration_enabled: bool,
    pub between_enabled: bool,
    /// `AutoCheck`: can it be toggled.
    pub fill_can_toggle: bool,
    pub skip_can_toggle: bool,
    pub mark_start_end_can_toggle: bool,
}

/// What OK does before closing. F392.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddMultipleOk {
    Proceed,
    /// "OOPS!": the dialog stays open.
    ZeroCount { message: String, title: String },
    /// Asks; No stays open.
    Overrun { message: String, title: String },
}

/// The label over the count. F384.
pub fn possible_label(count_max: i32) -> String {
    format!("{count_max} effects possible.")
}

impl AddMultipleState {
    /// `AddMultipleEffects`' set-up. The only caller requires Shift, so the choice is whether the dialog has been used in this process. Previous playback range values win over the
    /// remembered and the defaults. F291.
    pub fn open(effect_name: &str, time: TimeSpan, length: TimeSpan, remembered_start: Option<TimeSpan>, remembered_end: Option<TimeSpan>, marks: &[MarkCollection]) -> AddMultipleState {
        let memory = MEMORY.lock().clone();
        let mut state = AddMultipleState {
            effect_name: effect_name.to_string(),
            length,
            start: remembered_start.unwrap_or(time),
            end: remembered_end.unwrap_or(length),
            duration: TimeSpan::from_secs(2.0),
            between: TimeSpan::from_secs(2.0),
            count: 2,
            count_max: 0,
            ok_enabled: false,
            align_to_marks: false,
            skip_every_other: false,
            fill: false,
            mark_start_end: false,
            select_effects: false,
            panel_open: false,
            checked_marks: Vec::new(),
            duration_enabled: true,
            between_enabled: true,
            fill_can_toggle: false,
            skip_can_toggle: false,
            mark_start_end_can_toggle: false,
        };
        if let Some(m) = memory {
            state.start = remembered_start.unwrap_or(m.start);
            state.end = remembered_end.unwrap_or(m.end);
            state.duration = m.duration;
            state.between = m.between;
            state.skip_every_other = m.skip_every_other;
            state.fill = m.fill;
            state.mark_start_end = m.mark_start_end;
            state.select_effects = m.select_effects;
            state.panel_open = m.panel_open;
            // The setter fires the change handler, which populates the list, and then the checks find items by text (F393).
            if m.align_to_marks {
                state.set_align(true, marks);
            }
            state.checked_marks = m.checked_marks;
            // `checkBoxFillDuration_CheckStateChanged` and its sibling ran when these were set.
            if state.fill {
                state.duration_enabled = false;
                state.mark_start_end_can_toggle = false;
            }
            if state.mark_start_end {
                state.duration_enabled = false;
                state.fill_can_toggle = false;
            }
        }
        state.set_checkbox_states();
        if state.fill {
            state.mark_start_end_can_toggle = false;
        }
        if state.mark_start_end {
            state.fill_can_toggle = false;
        }
        state.recalculate(marks);
        state
    }

    /// `SetCheckboxStates`.
    fn set_checkbox_states(&mut self) {
        self.fill_can_toggle = self.align_to_marks;
        self.skip_can_toggle = self.align_to_marks;
        self.mark_start_end_can_toggle = self.align_to_marks;
    }

    /// `CalculatePossibleEffects`: the count and its maximum follow on every change, overwriting what was typed. F386 to F388, F422.
    pub fn recalculate(&mut self, marks: &[MarkCollection]) {
        if self.align_to_marks {
            let times = self.times_in_range(marks);
            let possible = possible_effects_on_marks(times, self.fill, self.skip_every_other);
            self.count = possible;
            self.count_max = possible;
            self.ok_enabled = times != 0;
        } else {
            let possible = possible_effects(self.start, self.end, self.duration, self.between);
            self.count = possible;
            self.count_max = possible;
            self.ok_enabled = possible != 0;
        }
    }

    fn times_in_range(&self, marks: &[MarkCollection]) -> usize {
        // Once per checked name, so a duplicate name counts each collection it matches.
        let mut count = 0;
        for name in &self.checked_marks {
            for collection in marks.iter().filter(|c| &c.name == name) {
                count += collection.marks.iter().filter(|m| m.start_time >= self.start && m.start_time < self.end).count();
            }
        }
        count
    }

    fn clamp_time(&self, t: TimeSpan) -> TimeSpan {
        clamp(t, TimeSpan::ZERO, self.length)
    }

    pub fn set_start(&mut self, t: TimeSpan, marks: &[MarkCollection]) {
        self.start = self.clamp_time(t);
        self.recalculate(marks);
    }

    pub fn set_end(&mut self, t: TimeSpan, marks: &[MarkCollection]) {
        self.end = self.clamp_time(t);
        self.recalculate(marks);
    }

    pub fn set_duration(&mut self, t: TimeSpan, marks: &[MarkCollection]) {
        self.duration = clamp(t, TimeSpan::from_ms_i(10), self.length);
        self.recalculate(marks);
    }

    pub fn set_between(&mut self, t: TimeSpan, marks: &[MarkCollection]) {
        self.between = self.clamp_time(t);
        self.recalculate(marks);
    }

    /// `checkBoxAlignToBeatMarks_CheckStateChanged`. Returns the duplicate-names warning when turning on finds one.
    pub fn set_align(&mut self, on: bool, marks: &[MarkCollection]) -> Option<(String, String)> {
        self.align_to_marks = on;
        self.between_enabled = !on;
        self.set_checkbox_states();
        // `PopulateMarksList` clears the checks.
        self.checked_marks.clear();
        self.recalculate(marks);
        if on {
            let mut names = std::collections::HashSet::new();
            if marks.iter().any(|m| !names.insert(m.name.as_str())) {
                return Some((
                    "You have Beat Mark collections with duplicate names.\nBecause of this, your results may not be as expected.".to_string(),
                    "Duplicate Names".to_string(),
                ));
            }
        }
        None
    }

    pub fn set_skip_every_other(&mut self, on: bool, marks: &[MarkCollection]) {
        if !self.skip_can_toggle {
            return;
        }
        self.skip_every_other = on;
        self.recalculate(marks);
    }

    /// `checkBoxFillDuration_CheckStateChanged`.
    pub fn set_fill(&mut self, on: bool, marks: &[MarkCollection]) {
        if !self.fill_can_toggle {
            return;
        }
        self.fill = on;
        self.mark_start_end_can_toggle = !on;
        self.duration_enabled = !on;
        self.recalculate(marks);
    }

    /// `chkUseMarkStartEnd_CheckStateChanged`. Turning it off doesn't enable Duration again (KEEP F384).
    pub fn set_mark_start_end(&mut self, on: bool, marks: &[MarkCollection]) {
        if !self.mark_start_end_can_toggle {
            return;
        }
        self.mark_start_end = on;
        self.fill_can_toggle = !on;
        if on {
            self.duration_enabled = false;
        }
        self.recalculate(marks);
    }

    pub fn set_mark_checked(&mut self, name: &str, on: bool, marks: &[MarkCollection]) {
        if on {
            if !self.checked_marks.iter().any(|n| n == name) {
                self.checked_marks.push(name.to_string());
            }
        } else {
            self.checked_marks.retain(|n| n != name);
        }
        self.recalculate(marks);
    }

    /// `txtEffectCount_KeyUp`: typing can enable an OK the sums disabled, and the next change overwrites what was typed. F422.
    pub fn set_count_text(&mut self, text: &str) {
        match text.trim().parse::<i32>() {
            Ok(value) => {
                // `NumericUpDown.Value` stays within its maximum.
                self.count = value.clamp(0, self.count_max);
                self.ok_enabled = true;
            }
            Err(_) => self.ok_enabled = false,
        }
    }

    /// `lblPossibleEffects_DoubleClick`. F389.
    pub fn use_all_possible(&mut self) {
        self.count = self.count_max;
    }

    /// `btnOK_Click`: both checks run, in order. F392.
    pub fn check_ok(&self) -> AddMultipleOk {
        if self.count == 0 {
            return AddMultipleOk::ZeroCount { message: "OOPS! Your effect count is set to 0 (zero)".to_string(), title: "Warning".to_string() };
        }
        if !time_exists_for_addition(self.start, self.end, self.duration, self.between, self.count) && !self.align_to_marks && !self.fill {
            return AddMultipleOk::Overrun {
                message: "At least one effect would be placed beyond the sequence length, and will not be added.\n\nWould you like to proceed anyway?".to_string(),
                title: "Warning".to_string(),
            };
        }
        AddMultipleOk::Proceed
    }

    /// OK saves all values. F291.
    pub fn remember(&self) {
        *MEMORY.lock() = Some(AddMultipleMemory {
            start: self.start,
            end: self.end,
            duration: self.duration,
            between: self.between,
            align_to_marks: self.align_to_marks,
            skip_every_other: self.skip_every_other,
            fill: self.fill,
            mark_start_end: self.mark_start_end,
            select_effects: self.select_effects,
            panel_open: self.panel_open,
            checked_marks: self.checked_marks.clone(),
        });
    }
}

/// `MarkCollection.OffsetMarksByTime` for a time window: F92. Returns how many marks it changed, deleted ones included.
pub fn offset_marks_in_range(collection: &mut MarkCollection, start: TimeSpan, end: TimeSpan, offset: TimeSpan, max_time: TimeSpan) -> usize {
    let mut changed = 0;
    let mut index = 0;
    while index < collection.marks.len() {
        let mark = &mut collection.marks[index];
        if start <= mark.start_time && mark.start_time <= end {
            changed += 1;
            if mark.end_time() + offset < TimeSpan::ZERO || mark.start_time + offset > max_time {
                // A mark wholly past either end goes, and the next one moves into this index.
                collection.marks.remove(index);
                continue;
            }
            mark.start_time += offset;
            if mark.start_time < TimeSpan::ZERO {
                mark.duration += mark.start_time;
                mark.start_time = TimeSpan::ZERO;
            } else if mark.end_time() > max_time {
                // As upstream writes it: this adds the end time rather than clipping (KEEP).
                let end_time = mark.end_time();
                mark.duration += end_time;
            }
        }
        index += 1;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::Mark;

    fn s(secs: f64) -> TimeSpan {
        TimeSpan::from_secs(secs)
    }

    fn collection(name: &str, times: &[f64]) -> MarkCollection {
        let mut c = MarkCollection::default();
        c.name = name.to_string();
        c.marks = times.iter().map(|t| Mark::new(s(*t))).collect();
        c
    }

    #[test]
    fn time_editor_moves_fields_together() {
        let mut e = EffectTimeEditorState::new(s(10.0), s(5.0), s(60.0), s(0.25));
        assert_eq!((e.start, e.duration, e.end), (s(10.0), s(5.0), s(15.0)));
        assert_eq!(e.start_max, s(59.75));
        assert_eq!(e.duration_max, s(50.0));
        assert_eq!(e.end_min, s(10.25));
        e.set_start(s(20.0));
        assert_eq!((e.start, e.duration, e.end), (s(20.0), s(5.0), s(25.0)));
        assert_eq!(e.duration_max, s(40.0));
        assert_eq!(e.end_min, s(20.25));
        e.set_duration(s(10.0));
        assert_eq!(e.end, s(30.0));
        e.set_end(s(26.0));
        assert_eq!(e.duration, s(6.0));
        // Past the sequence's end, the duration shortens.
        e.set_start(s(58.0));
        assert_eq!((e.start, e.duration), (s(58.0), s(2.0)));
        // Clamped to the limits it was given.
        e.set_duration(s(100.0));
        assert_eq!(e.duration, s(2.0));
        e.set_start(s(200.0));
        assert_eq!(e.start, s(59.75));
    }

    #[test]
    fn match_sequence_takes_the_whole_length() {
        let mut e = EffectTimeEditorState::new(s(10.0), s(5.0), s(60.0), s(0.25));
        e.match_sequence();
        assert_eq!((e.start, e.duration, e.end), (TimeSpan::ZERO, s(60.0), s(60.0)));
    }

    #[test]
    fn add_multiple_opens_at_the_possible_count_then_remembers() {
        let _memory = clear_add_multiple_memory();
        let marks = [collection("Beats", &[1.0, 2.0, 3.0])];
        let state = AddMultipleState::open("Pulse", s(0.0), s(60.0), None, None, &marks);
        // Not 2: the dialog recalculates at load.
        assert_eq!(state.count, 15);
        assert_eq!(state.count_max, 15);
        assert!(state.ok_enabled);
        assert_eq!(possible_label(state.count_max), "15 effects possible.");

        let mut chosen = state;
        chosen.select_effects = true;
        chosen.duration = s(3.0);
        chosen.remember();
        let again = AddMultipleState::open("Pulse", s(9.0), s(60.0), None, None, &marks);
        assert_eq!(again.duration, s(3.0));
        assert!(again.select_effects);
        // The remembered start is the dialog's, not the click's.
        assert_eq!(again.start, s(0.0));
        // A remembered playback range wins.
        let ranged = AddMultipleState::open("Pulse", s(9.0), s(60.0), Some(s(5.0)), Some(s(50.0)), &marks);
        assert_eq!((ranged.start, ranged.end), (s(5.0), s(50.0)));
    }

    #[test]
    fn aligning_counts_checked_marks_and_clears_old_checks() {
        let _memory = clear_add_multiple_memory();
        let marks = [collection("Beats", &[1.0, 2.0, 3.0, 70.0]), collection("Other", &[4.0])];
        let mut state = AddMultipleState::open("Pulse", s(0.0), s(60.0), None, None, &marks);
        assert!(!state.fill_can_toggle);
        state.set_fill(true, &marks);
        assert!(!state.fill, "can't be toggled until aligned");
        assert_eq!(state.set_align(true, &marks), None);
        assert!(!state.between_enabled && state.fill_can_toggle);
        assert_eq!((state.count, state.ok_enabled), (0, false));
        state.set_mark_checked("Beats", true, &marks);
        // 70 is past the end of the range.
        assert_eq!((state.count, state.count_max, state.ok_enabled), (3, 3, true));
        state.set_fill(true, &marks);
        assert_eq!(state.count, 2);
        assert!(!state.duration_enabled && !state.mark_start_end_can_toggle);
        state.set_fill(false, &marks);
        assert!(state.duration_enabled);
        state.set_skip_every_other(true, &marks);
        assert_eq!(state.count, 2);
        state.set_mark_start_end(true, &marks);
        assert!(!state.duration_enabled && !state.fill_can_toggle);
        // Turning it off leaves Duration disabled (KEEP).
        state.set_mark_start_end(false, &marks);
        assert!(!state.duration_enabled && state.fill_can_toggle);
    }

    #[test]
    fn duplicate_names_warn_when_aligning() {
        let _memory = clear_add_multiple_memory();
        let marks = [collection("Beats", &[1.0]), collection("Beats", &[2.0])];
        let mut state = AddMultipleState::open("Pulse", s(0.0), s(60.0), None, None, &marks);
        let warning = state.set_align(true, &marks).unwrap();
        assert_eq!(warning.1, "Duplicate Names");
        // The checked name matches both.
        state.set_mark_checked("Beats", true, &marks);
        assert_eq!(state.count, 2);
    }

    #[test]
    fn ok_checks() {
        let _memory = clear_add_multiple_memory();
        let mut state = AddMultipleState::open("Pulse", s(0.0), s(60.0), None, None, &[]);
        assert_eq!(state.check_ok(), AddMultipleOk::Proceed);
        state.count = 0;
        assert!(matches!(state.check_ok(), AddMultipleOk::ZeroCount { .. }));
        state.count = 100;
        assert!(matches!(state.check_ok(), AddMultipleOk::Overrun { .. }));
    }

    #[test]
    fn typing_the_count_enables_ok_until_it_is_recalculated() {
        let _memory = clear_add_multiple_memory();
        let mut state = AddMultipleState::open("Pulse", s(0.0), s(1.0), None, None, &[]);
        assert!(!state.ok_enabled);
        state.set_count_text("3");
        assert!(state.ok_enabled);
        state.set_count_text("x");
        assert!(!state.ok_enabled);
        state.set_duration(s(0.5), &[]);
        assert_eq!(state.count, 1);
    }

    #[test]
    fn bulk_mark_offset_moves_clips_and_deletes() {
        let mut c = collection("Beats", &[0.5, 2.0, 3.0, 9.0]);
        // Marks are 450 ms. Moving back 1.2 s: the first is wholly before zero and goes.
        let changed = offset_marks_in_range(&mut c, s(0.0), s(5.0), TimeSpan(-(s(1.2).0)), s(10.0));
        assert_eq!(changed, 3);
        assert_eq!(c.marks.len(), 3);
        assert_eq!(c.marks[0].start_time, s(0.8));
        assert_eq!(c.marks[1].start_time, s(1.8));
        assert_eq!(c.marks[2].start_time, s(9.0));

        let mut c = collection("Beats", &[1.0]);
        offset_marks_in_range(&mut c, s(0.0), s(5.0), TimeSpan(-(s(1.2).0)), s(10.0));
        assert_eq!(c.marks.len(), 1);
        assert_eq!(c.marks[0].start_time, TimeSpan::ZERO);
        assert_eq!(c.marks[0].duration, s(0.25));

        // Past the end it goes; running over the end hits upstream's sum.
        let mut c = collection("Beats", &[5.0]);
        offset_marks_in_range(&mut c, s(0.0), s(5.0), s(6.0), s(10.0));
        assert!(c.marks.is_empty());
        let mut c = collection("Beats", &[5.0]);
        offset_marks_in_range(&mut c, s(0.0), s(5.0), s(4.8), s(10.0));
        assert_eq!(c.marks[0].start_time, s(9.8));
        assert_eq!(c.marks[0].duration, s(0.45) + s(10.25));
    }
}
