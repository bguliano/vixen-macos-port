//! Cut, copy and paste, section J of the spec: `ClipboardAddData`, `ClipboardPaste` and `UpdatePasteMenuStates`.
//!
//! The clipboard is JSON from here, carrying each effect's parameters in `vixen-io`'s XML text form. The window puts it on the pasteboard
//! under `org.vixenlights.timeline-elements`, with the count as a separate small item, so hovering over the Edit strip never decodes it.

use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;
use vixen_core::{MarkCollection, TimeSpan};
use vixen_effects::framework::Params;
use vixen_io::{effect_model_to_xml, effect_params_from_xml};
use vixen_xml::XElement;

/// The pasteboard type holding the JSON.
pub const PASTEBOARD_TYPE: &str = "org.vixenlights.timeline-elements";
/// The pasteboard type holding only how many effects the JSON has.
pub const PASTEBOARD_COUNT_TYPE: &str = "org.vixenlights.timeline-elements.count";

/// `PastingMode`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum PastingMode {
    #[default]
    Default,
    Invert,
    VisibleMarks,
}

/// `EffectModelCandidate` and its row offset.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardEffect {
    pub type_id: Uuid,
    pub start: TimeSpan,
    pub duration: TimeSpan,
    pub layer_id: Uuid,
    pub layer_name: String,
    pub layer_type_id: Uuid,
    /// The module data as XML text.
    pub data: String,
    pub row_offset: i32,
}

/// `TimelineElementsClipboardData`.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardData {
    pub effects: Vec<ClipboardEffect>,
    pub first_visible_row: i32,
    pub earliest_start: TimeSpan,
}

impl ClipboardData {
    pub fn to_json(&self) -> String {
        json!({
            "EffectModelCandidates": self.effects.iter().map(|e| json!({
                "EffectModel": {
                    "TypeId": e.type_id.to_string(),
                    "StartTime": e.start.0,
                    "Duration": e.duration.0,
                    "LayerId": e.layer_id.to_string(),
                    "LayerName": e.layer_name,
                    "LayerTypeId": e.layer_type_id.to_string(),
                    "EffectData": e.data,
                },
                "RowOffset": e.row_offset,
            })).collect::<Vec<_>>(),
            "FirstVisibleRow": self.first_visible_row,
            "EarliestStartTime": self.earliest_start.0,
        })
        .to_string()
    }

    /// Anything that isn't a clipboard of effects gives `None`: `TryGetData` failing.
    pub fn from_json(text: &str) -> Option<ClipboardData> {
        let value: Value = serde_json::from_str(text).ok()?;
        let mut effects = Vec::new();
        for record in value.get("EffectModelCandidates")?.as_array()? {
            let model = record.get("EffectModel")?;
            let uuid = |key: &str| model.get(key).and_then(Value::as_str).and_then(|s| Uuid::parse_str(s).ok());
            effects.push(ClipboardEffect {
                type_id: uuid("TypeId")?,
                start: TimeSpan(model.get("StartTime")?.as_i64()?),
                duration: TimeSpan(model.get("Duration")?.as_i64()?),
                layer_id: uuid("LayerId").unwrap_or_else(Uuid::nil),
                layer_name: model.get("LayerName").and_then(Value::as_str).unwrap_or("").to_string(),
                layer_type_id: uuid("LayerTypeId").unwrap_or_else(Uuid::nil),
                data: model.get("EffectData")?.as_str()?.to_string(),
                row_offset: record.get("RowOffset")?.as_i64()? as i32,
            });
        }
        Some(ClipboardData {
            effects,
            first_visible_row: value.get("FirstVisibleRow").and_then(Value::as_i64).unwrap_or(-1) as i32,
            earliest_start: TimeSpan(value.get("EarliestStartTime").and_then(Value::as_i64).unwrap_or(i64::MAX)),
        })
    }
}

/// `new EffectModelCandidate(effect)`: the module data, as text. `None` for an effect whose data can't be written.
pub fn effect_data_text(type_id: &Uuid, instance_id: &Uuid, params: &Params) -> Option<String> {
    effect_model_to_xml(type_id, instance_id, params).ok().map(|x| x.to_fragment())
}

/// `EffectModelCandidate.GetEffectData`. `None` when it can't be read back: "Effect(s) on clipboard are incomplete."
pub fn effect_data_from_text(type_id: &Uuid, data: &str) -> Option<Params> {
    let descriptor = vixen_effect_registry::by_type_id(type_id)?;
    let element = XElement::parse(data).ok()?;
    Some(effect_params_from_xml(&element, descriptor))
}

/// One selected effect, as `plan_copy` sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct CopyCandidate<Id> {
    pub id: Id,
    pub start: TimeSpan,
    pub duration: TimeSpan,
}

/// One row for `plan_copy`: its name and its selected effects. `row.SelectedElements`.
pub struct CopyRow<Id> {
    pub name: String,
    pub selected: Vec<CopyCandidate<Id>>,
}

/// What `ClipboardAddData` takes from the rows: `(effect, row offset)` in clipboard order, plus the first row number and the earliest start.
#[derive(Clone, Debug, PartialEq)]
pub struct CopyPlan<Id> {
    pub picks: Vec<(CopyCandidate<Id>, i32)>,
    pub first_visible_row: i32,
    pub earliest_start: TimeSpan,
}

/// `ClipboardAddData`'s walk. A row whose name was already seen is skipped and doesn't advance the row number, even in another group. A row
/// holding an effect already taken does count. So selected effects on rows that aren't visible are left out, without a word (KEEP F317).
pub fn plan_copy<Id: Copy + Eq + std::hash::Hash>(rows: &[CopyRow<Id>]) -> CopyPlan<Id> {
    let mut plan = CopyPlan { picks: Vec::new(), first_visible_row: -1, earliest_start: TimeSpan::MAX };
    let mut names: HashSet<&str> = HashSet::new();
    let mut taken: HashSet<Id> = HashSet::new();
    let mut row_number = 0;
    for row in rows {
        if !names.insert(&row.name) {
            continue;
        }
        if row.selected.iter().any(|e| taken.contains(&e.id)) {
            row_number += 1;
            continue;
        }
        for effect in &row.selected {
            taken.insert(effect.id);
            if plan.first_visible_row == -1 {
                plan.first_visible_row = row_number;
            }
            plan.picks.push((effect.clone(), row_number - plan.first_visible_row));
            if effect.start < plan.earliest_start {
                plan.earliest_start = effect.start;
            }
        }
        row_number += 1;
    }
    plan
}

/// A visible row, as paste sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct PasteRow {
    pub name: String,
    pub deprecated: bool,
}

/// One effect to place: which clipboard record, where, and for how long.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PasteItem {
    pub record: usize,
    pub row: usize,
    pub time: TimeSpan,
    pub duration: TimeSpan,
}

/// The result of `plan_paste`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PastePlan {
    pub items: Vec<PasteItem>,
    /// Row names skipped for being Deprecated, each once, in order met.
    pub skipped_deprecated: Vec<String>,
}

/// `ClipboardPaste`'s placement, with no effects touched. `anchor` is the visible index of `SelectedRow ?? ActiveRow ?? TopVisibleRow`, and -1
/// when that row isn't visible, which shifts everything up one row.
///
/// The count of effects pasted so far drives both the mark and the invert lookup, so after a skipped effect the next one takes the skipped one's
/// mark or inverted row (KEEP F318).
pub fn plan_paste(
    data: &ClipboardData,
    mode: PastingMode,
    paste_time: TimeSpan,
    anchor: i32,
    rows: &[PasteRow],
    total: TimeSpan,
    marks: &[MarkCollection],
) -> PastePlan {
    let mut plan = PastePlan::default();
    let mut order: Vec<usize> = (0..data.effects.len()).collect();
    let mut mark_times = Vec::new();
    let mut inverted = Vec::new();
    match mode {
        PastingMode::VisibleMarks => {
            // Ordered by start, as they're otherwise ordered by row.
            order.sort_by_key(|i| data.effects[*i].start);
            mark_times = visible_mark_times(marks, paste_time, data.effects.len());
        }
        PastingMode::Invert => {
            if let Some(last) = data.effects.last() {
                inverted = data.effects.iter().map(|e| last.row_offset - e.row_offset).collect();
            }
        }
        PastingMode::Default => {}
    }
    let offset = if paste_time == TimeSpan::ZERO { TimeSpan::ZERO } else { data.earliest_start };
    let mut pasted = 0usize;
    for record in order {
        let effect = &data.effects[record];
        let mut relative = effect.row_offset;
        let mut target = effect.start - offset + paste_time;
        match mode {
            PastingMode::VisibleMarks => {
                let Some(time) = mark_times.get(pasted) else { break };
                target = *time;
            }
            PastingMode::Invert => relative = inverted[pasted],
            PastingMode::Default => {}
        }
        if target > total {
            continue;
        }
        let mut duration = effect.duration;
        if target + duration > total {
            duration = total - target;
        }
        let row = anchor + relative;
        if row < 0 || row as usize >= rows.len() {
            continue;
        }
        let row = row as usize;
        if rows[row].deprecated {
            if !plan.skipped_deprecated.contains(&rows[row].name) {
                plan.skipped_deprecated.push(rows[row].name.clone());
            }
            continue;
        }
        plan.items.push(PasteItem { record, row, time: target, duration });
        pasted += 1;
    }
    plan
}

/// `IsVisible`: `ShowGridLines || ShowMarkBar`.
pub fn is_collection_visible(collection: &MarkCollection) -> bool {
    collection.show_grid_lines || collection.show_marks
}

/// For each visible collection, the first mark at or after the paste time and the ones after it, as many as there are effects. All sorted.
fn visible_mark_times(marks: &[MarkCollection], paste_time: TimeSpan, wanted: usize) -> Vec<TimeSpan> {
    let mut times = Vec::new();
    for collection in marks.iter().filter(|c| is_collection_visible(c)) {
        if let Some(first) = collection.marks.iter().position(|m| paste_time <= m.start_time) {
            times.extend(collection.marks[first..].iter().take(wanted).map(|m| m.start_time));
        }
    }
    times.sort();
    times
}

/// `GetMarksPresent`: any visible collection has a mark at or after the cursor. Marks are kept sorted, so this looks at the last one.
pub fn marks_present_after(cursor: TimeSpan, marks: &[MarkCollection]) -> bool {
    marks.iter().filter(|c| is_collection_visible(c)).any(|c| c.marks.iter().any(|m| cursor <= m.start_time))
}

/// What the Edit menu and strip show for paste. F316.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PasteMenuState {
    /// The Paste button and item.
    pub paste: bool,
    /// The Visible Marks item's enabled state and the button's visible state.
    pub paste_marks: bool,
    /// The Invert item's enabled state and the button's visible state.
    pub paste_invert: bool,
    /// The drop-down and the Paste Special item.
    pub paste_special: bool,
}

pub fn paste_menu_state(count: usize, marks_present: bool) -> PasteMenuState {
    let paste_marks = marks_present && count > 0;
    let paste_invert = count > 1;
    PasteMenuState { paste: count > 0, paste_marks, paste_invert, paste_special: paste_marks || paste_invert }
}

/// The context menu's paste items. F248 to F251.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ContextPasteState {
    /// The Paste Special submenu shows, holding Invert and Visible Marks.
    pub submenu: bool,
    /// Paste - Visible Mark/s shows on its own.
    pub marks_alone: bool,
    /// Paste - Invert shows on its own.
    pub invert_alone: bool,
    pub paste_enabled: bool,
    pub children_enabled: bool,
}

pub fn context_paste_state(count: usize, marks_present: bool, deprecated_row: bool) -> ContextPasteState {
    let submenu = count > 1 && marks_present;
    ContextPasteState {
        submenu,
        marks_alone: !submenu && marks_present && count > 0,
        invert_alone: !submenu && count > 1 && !marks_present,
        paste_enabled: count > 0 && !deprecated_row,
        children_enabled: !deprecated_row,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::Mark;

    fn s(secs: f64) -> TimeSpan {
        TimeSpan::from_secs(secs)
    }

    fn candidate(id: u32, start: f64) -> CopyCandidate<u32> {
        CopyCandidate { id, start: s(start), duration: s(1.0) }
    }

    fn row(name: &str, selected: Vec<CopyCandidate<u32>>) -> CopyRow<u32> {
        CopyRow { name: name.to_string(), selected }
    }

    #[test]
    fn copy_offsets_are_relative_to_the_first_row_with_a_selection() {
        let rows = vec![
            row("r0", vec![]),
            row("r1", vec![]),
            row("r2", vec![candidate(1, 5.0)]),
            row("r3", vec![]),
            row("r4", vec![]),
            row("r5", vec![candidate(2, 3.0)]),
        ];
        let plan = plan_copy(&rows);
        assert_eq!(plan.first_visible_row, 2);
        assert_eq!(plan.picks.iter().map(|p| (p.0.id, p.1)).collect::<Vec<_>>(), [(1, 0), (2, 3)]);
        assert_eq!(plan.earliest_start, s(3.0));
    }

    #[test]
    fn a_row_with_a_duplicate_name_is_skipped_without_counting() {
        let rows = vec![row("a", vec![candidate(1, 0.0)]), row("a", vec![candidate(9, 0.0)]), row("b", vec![candidate(2, 0.0)])];
        let plan = plan_copy(&rows);
        assert_eq!(plan.picks.iter().map(|p| (p.0.id, p.1)).collect::<Vec<_>>(), [(1, 0), (2, 1)]);
    }

    #[test]
    fn an_effect_already_taken_counts_the_row_and_takes_nothing_more() {
        let rows = vec![
            row("a", vec![candidate(1, 0.0)]),
            row("other group a", vec![candidate(1, 0.0), candidate(7, 0.0)]),
            row("b", vec![candidate(2, 0.0)]),
        ];
        let plan = plan_copy(&rows);
        assert_eq!(plan.picks.iter().map(|p| (p.0.id, p.1)).collect::<Vec<_>>(), [(1, 0), (2, 2)]);
    }

    #[test]
    fn nothing_selected_leaves_the_sentinels() {
        let plan = plan_copy(&[row("a", vec![])]);
        assert!(plan.picks.is_empty());
        assert_eq!(plan.first_visible_row, -1);
    }

    fn record(start: f64, row_offset: i32) -> ClipboardEffect {
        ClipboardEffect {
            type_id: Uuid::nil(),
            start: s(start),
            duration: s(1.0),
            layer_id: Uuid::nil(),
            layer_name: String::new(),
            layer_type_id: Uuid::nil(),
            data: String::new(),
            row_offset,
        }
    }

    fn data(effects: Vec<ClipboardEffect>, earliest: f64) -> ClipboardData {
        ClipboardData { effects, first_visible_row: 0, earliest_start: s(earliest) }
    }

    fn rows(n: usize) -> Vec<PasteRow> {
        (0..n).map(|i| PasteRow { name: format!("r{i}"), deprecated: false }).collect()
    }

    fn collection(times: &[f64], visible: bool) -> MarkCollection {
        let mut c = MarkCollection::default();
        c.show_grid_lines = visible;
        c.show_marks = visible;
        c.marks = times.iter().map(|t| Mark::new(s(*t))).collect();
        c
    }

    #[test]
    fn default_paste_moves_by_the_earliest_start() {
        let d = data(vec![record(10.0, 0), record(12.0, 1)], 10.0);
        let p = plan_paste(&d, PastingMode::Default, s(30.0), 0, &rows(5), s(100.0), &[]);
        assert_eq!(p.items.iter().map(|i| (i.time, i.row)).collect::<Vec<_>>(), [(s(30.0), 0), (s(32.0), 1)]);
        // Paste time 0 keeps original times.
        let p = plan_paste(&d, PastingMode::Default, TimeSpan::ZERO, 2, &rows(5), s(100.0), &[]);
        assert_eq!(p.items.iter().map(|i| (i.time, i.row)).collect::<Vec<_>>(), [(s(10.0), 2), (s(12.0), 3)]);
    }

    #[test]
    fn past_the_end_drops_and_overrun_shortens() {
        let d = data(vec![record(10.0, 0), record(12.0, 0), record(11.5, 0)], 10.0);
        let p = plan_paste(&d, PastingMode::Default, s(99.0), 0, &rows(1), s(100.0), &[]);
        // 99 fits in full (100 is the end), 101 is past it, and 100.5 is past it.
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.items[0].duration, s(1.0));
        let p = plan_paste(&d, PastingMode::Default, s(99.5), 0, &rows(1), s(100.0), &[]);
        assert_eq!(p.items[0].duration, s(0.5));
    }

    #[test]
    fn rows_out_of_range_and_deprecated_rows_are_skipped() {
        let d = data(vec![record(0.0, 0), record(1.0, 1), record(2.0, 2), record(3.0, 1)], 0.0);
        let mut list = rows(2);
        list[1].deprecated = true;
        let p = plan_paste(&d, PastingMode::Default, s(5.0), 0, &list, s(100.0), &[]);
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.skipped_deprecated, ["r1"]);
        // An anchor that isn't visible shifts everything up one row.
        let p = plan_paste(&data(vec![record(0.0, 0), record(1.0, 1)], 0.0), PastingMode::Default, s(5.0), -1, &rows(3), s(100.0), &[]);
        assert_eq!(p.items.iter().map(|i| i.row).collect::<Vec<_>>(), [0]);
    }

    #[test]
    fn visible_marks_paste() {
        let d = data(vec![record(0.0, 0), record(5.0, 0), record(2.0, 0)], 0.0);
        let m = [collection(&[1.0, 2.0, 3.0, 4.0], true)];
        let p = plan_paste(&d, PastingMode::VisibleMarks, s(2.0), 0, &rows(1), s(100.0), &m);
        assert_eq!(p.items.iter().map(|i| i.time).collect::<Vec<_>>(), [s(2.0), s(3.0), s(4.0)]);
        // Ordered by start.
        assert_eq!(p.items.iter().map(|i| i.record).collect::<Vec<_>>(), [0, 2, 1]);
        // Only two marks left: two pasted.
        let p = plan_paste(&d, PastingMode::VisibleMarks, s(3.0), 0, &rows(1), s(100.0), &m);
        assert_eq!(p.items.len(), 2);
        // Two visible collections merge sorted, and a hidden one doesn't count.
        let m = [collection(&[10.0, 30.0], true), collection(&[20.0, 25.0], true), collection(&[0.5], false)];
        let p = plan_paste(&d, PastingMode::VisibleMarks, s(0.0), 0, &rows(1), s(100.0), &m);
        assert_eq!(p.items.iter().map(|i| i.time).collect::<Vec<_>>(), [s(10.0), s(20.0), s(25.0)]);
    }

    #[test]
    fn invert_paste_and_its_index_quirk() {
        let d = data(vec![record(0.0, 0), record(1.0, 1), record(2.0, 3)], 0.0);
        let p = plan_paste(&d, PastingMode::Invert, s(5.0), 0, &rows(5), s(100.0), &[]);
        assert_eq!(p.items.iter().map(|i| i.row).collect::<Vec<_>>(), [3, 2, 0]);
        // The first record starting past the end is skipped, so the second takes index 0 of the inverted list: row 3, not 2.
        let d = data(vec![record(500.0, 0), record(1.0, 1), record(2.0, 3)], 1.0);
        let p = plan_paste(&d, PastingMode::Invert, s(5.0), 0, &rows(5), s(100.0), &[]);
        assert_eq!(p.items.iter().map(|i| i.row).collect::<Vec<_>>(), [3, 2]);
    }

    #[test]
    fn a_marks_paste_with_a_skip_takes_the_next_mark() {
        let d = data(vec![record(0.0, 0), record(1.0, 9), record(2.0, 0)], 0.0);
        let m = [collection(&[1.0, 2.0, 3.0], true)];
        let p = plan_paste(&d, PastingMode::VisibleMarks, s(0.0), 0, &rows(1), s(100.0), &m);
        // The middle effect's row is out of range, so the third takes its mark.
        assert_eq!(p.items.iter().map(|i| (i.record, i.time)).collect::<Vec<_>>(), [(0, s(1.0)), (2, s(2.0))]);
    }

    #[test]
    fn menu_states() {
        let cases = [
            ((0, false), (false, false, false, false)),
            ((0, true), (false, false, false, false)),
            ((1, false), (true, false, false, false)),
            ((1, true), (true, true, false, true)),
            ((2, false), (true, false, true, true)),
            ((2, true), (true, true, true, true)),
        ];
        for ((count, marks), (paste, m, invert, special)) in cases {
            let st = paste_menu_state(count, marks);
            assert_eq!((st.paste, st.paste_marks, st.paste_invert, st.paste_special), (paste, m, invert, special), "{count} {marks}");
        }
    }

    #[test]
    fn context_menu_variant() {
        let st = context_paste_state(2, true, false);
        assert!(st.submenu && !st.marks_alone && !st.invert_alone && st.paste_enabled);
        let st = context_paste_state(1, true, false);
        assert!(!st.submenu && st.marks_alone && !st.invert_alone);
        let st = context_paste_state(2, false, false);
        assert!(!st.submenu && !st.marks_alone && st.invert_alone);
        let st = context_paste_state(0, false, false);
        assert!(!st.submenu && !st.marks_alone && !st.invert_alone && !st.paste_enabled);
        let st = context_paste_state(2, true, true);
        assert!(st.submenu && !st.paste_enabled && !st.children_enabled);
    }

    #[test]
    fn marks_present_needs_a_visible_mark_at_or_past_the_cursor() {
        let m = [collection(&[1.0, 2.0], true), collection(&[9.0], false)];
        assert!(marks_present_after(s(2.0), &m));
        assert!(!marks_present_after(s(2.1), &m));
    }

    #[test]
    fn json_round_trips() {
        let mut e = record(1.5, 3);
        e.type_id = Uuid::new_v4();
        e.layer_id = Uuid::new_v4();
        e.layer_name = "Top".into();
        e.data = "<a b=\"1\"/>".into();
        let d = data(vec![e, record(2.0, 0)], 1.5);
        assert_eq!(ClipboardData::from_json(&d.to_json()), Some(d));
        assert_eq!(ClipboardData::from_json("nonsense"), None);
        assert_eq!(ClipboardData::from_json("{}"), None);
    }

    #[test]
    fn effect_data_round_trips_through_text() {
        let d = vixen_effect_registry::all().iter().find(|d| d.name == "Set Level").unwrap();
        let params = d.new_params();
        let text = effect_data_text(&d.type_id, &Uuid::new_v4(), &params).unwrap();
        let back = effect_data_from_text(&d.type_id, &text).unwrap();
        assert!(back == params, "parameters survive the clipboard");
        assert!(effect_data_from_text(&d.type_id, "<<<").is_none());
        assert!(effect_data_from_text(&Uuid::nil(), &text).is_none());
    }
}
