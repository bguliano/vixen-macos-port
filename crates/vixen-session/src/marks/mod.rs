//! The Mark Collections panel without its window: the rows, the header toggles, every row command, and mark import's choice and commit.
//! `MarkDockerViewModel`, `MarkCollectionViewModel`, `MarkCollectionNameService`, `MarkCollectionImportCommitter`. Spec:
//! `docs/spec/marks-layers-analysis.md`, sections A, B, C and G. The file formats are `vixen_io::marks_files`.
//!
//! Upstream's panel actions have no undo, and neither do these. Each command works on a list of collections and says what it changed
//! (`MarksChange`), and `publish` hands the list to the session the way that change needs.

pub mod beats;
pub mod lipsync;
pub mod lyric_converter;
pub mod map_library;
pub mod node_select;
pub mod string_maps;
mod import;

pub use import::*;

use crate::sequence::SequenceSession;
use std::collections::HashSet;
use uuid::Uuid;
use vixen_core::marks::{Mark, MarkCollection, MarkCollectionType};
use vixen_core::{Color, TimeSpan};

/// `MarkCollectionNameService.DefaultName`.
pub const DEFAULT_NAME: &str = "Mark Collection";

/// FIX F8, for the owner's veto: the Tail Lines header's first value is worked out like the other three. Upstream starts it inverted, so
/// with no tail lines anywhere its first click sets them all off and only the second turns them on. `false` gives upstream's behaviour.
pub const TAIL_HEADER_FIX: bool = true;

/// What an edit changed, which decides what the session does about it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MarksChange {
    None,
    /// A name, flag, colour, type or link: the sequence is modified and the timeline follows; no effect renders again, since effects read
    /// only marks and ids.
    Properties,
    /// Collections came or went, or marks moved: effects see the new list (their selections normalised, F52) and those that read marks
    /// render again.
    Collections,
}

/// Gives the session an edited list.
pub fn publish(session: &mut SequenceSession, marks: Vec<MarkCollection>, change: MarksChange) {
    match change {
        MarksChange::None => {}
        MarksChange::Properties => {
            session.set_marks_quietly(marks);
            session.set_modified();
        }
        MarksChange::Collections => session.set_marks(marks),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Names, F50
// ---------------------------------------------------------------------------------------------------------------------

/// `NormalizeName`.
pub fn normalize_name(name: &str) -> &str {
    name.trim()
}

/// `StringComparison.OrdinalIgnoreCase`, which upper-cases one character at a time, so `ß` stays `ß`.
fn fold(name: &str) -> String {
    name.chars()
        .map(|c| {
            let mut upper = c.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(u), None) => u,
                _ => c,
            }
        })
        .collect()
}

fn names_match(a: &str, normalized: &str) -> bool {
    fold(normalize_name(a)) == fold(normalized)
}

/// `IsUniqueName`: false for a blank name, true when no other collection has it, ignoring case and surrounding space.
pub fn is_unique_name(collections: &[MarkCollection], proposed: &str, excluded: Option<Uuid>) -> bool {
    let normalized = normalize_name(proposed);
    if normalized.is_empty() {
        return false;
    }
    collections.iter().filter(|c| Some(c.id) != excluded).all(|c| !names_match(&c.name, normalized))
}

/// `GetUniqueName`: the trimmed name if it's free, else `<name> - 2`, `- 3` and so on. GUARD: upstream throws on a blank name, which an
/// imported file can hold (F36); that takes `Mark Collection` instead.
pub fn unique_name(desired: &str, collections: &[MarkCollection], excluded: Option<Uuid>) -> String {
    let base = match normalize_name(desired) {
        "" => DEFAULT_NAME,
        b => b,
    };
    let names: HashSet<String> =
        collections.iter().filter(|c| Some(c.id) != excluded).map(|c| normalize_name(&c.name)).filter(|n| !n.is_empty()).map(fold).collect();
    if !names.contains(&fold(base)) {
        return base.to_string();
    }
    (2..).map(|n| format!("{base} - {n}")).find(|candidate| !names.contains(&fold(candidate))).unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------------------------------
// Rows, F1 and F7 to F14
// ---------------------------------------------------------------------------------------------------------------------

/// One row of the panel, as it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkRow {
    pub id: Uuid,
    pub name: String,
    pub color: Color,
    pub is_default: bool,
    pub show_lines: bool,
    /// F11: the Tail checkbox is greyed while Lines is off, and keeps its value.
    pub tail_enabled: bool,
    pub show_tail: bool,
    pub show_bar: bool,
    pub locked: bool,
    pub level: i32,
    pub kind: MarkCollectionType,
    pub linked_to: Option<Uuid>,
    pub solid: bool,
    pub bold: bool,
    pub mark_count: usize,
}

/// The table, in list order. There is no way to reorder collections in the panel.
pub fn rows(collections: &[MarkCollection]) -> Vec<MarkRow> {
    collections
        .iter()
        .map(|c| MarkRow {
            id: c.id,
            name: c.name.clone(),
            color: c.decorator.color,
            is_default: c.is_default,
            show_lines: c.show_grid_lines,
            tail_enabled: c.show_grid_lines,
            show_tail: c.show_tail_lines,
            show_bar: c.show_marks,
            locked: c.is_locked,
            level: c.level,
            kind: c.collection_type,
            linked_to: c.linked_mark_collection_id,
            solid: c.decorator.is_solid_line,
            bold: c.decorator.is_bold,
            mark_count: c.marks.len(),
        })
        .collect()
}

/// `IsVisible`: lines or a bar.
pub fn is_visible(c: &MarkCollection) -> bool {
    c.show_grid_lines || c.show_marks
}

/// The first collection with this id. A `.v3m` imported into the sequence it came from can make two (F28, KEEP).
fn index_of(collections: &[MarkCollection], id: Uuid) -> Option<usize> {
    collections.iter().position(|c| c.id == id)
}

// ---------------------------------------------------------------------------------------------------------------------
// Header toggles, F8
// ---------------------------------------------------------------------------------------------------------------------

/// The four clickable column headers.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MarkHeader {
    Lines,
    TailLines,
    Bar,
    Lock,
}

/// Each header's next value. Worked out once when the panel is made (and again whenever it's shown after being hidden, F114), then
/// flipped by each click: KEEP, a click with rows in mixed states applies the remembered value, not the inverse of what shows.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HeaderFlags {
    pub lines: bool,
    pub tail: bool,
    pub bar: bool,
    pub lock: bool,
}

impl HeaderFlags {
    /// `All` on an empty list is true, so a sequence with no collections starts Lines, Bar and Lock at false.
    pub fn new(collections: &[MarkCollection]) -> HeaderFlags {
        let all_tail = collections.iter().all(|c| c.show_tail_lines);
        HeaderFlags {
            lines: !collections.iter().all(|c| c.show_grid_lines),
            tail: if TAIL_HEADER_FIX { !all_tail } else { all_tail },
            bar: !collections.iter().all(|c| c.show_marks),
            lock: !collections.iter().all(|c| c.is_locked),
        }
    }

    pub fn next_value(&self, header: MarkHeader) -> bool {
        match header {
            MarkHeader::Lines => self.lines,
            MarkHeader::TailLines => self.tail,
            MarkHeader::Bar => self.bar,
            MarkHeader::Lock => self.lock,
        }
    }

    /// Sets the flag on every collection, then flips it. Only Bar has an enable rule, at least one collection, so on an empty list a Bar
    /// click leaves its flag alone while the others still flip.
    pub fn click(&mut self, header: MarkHeader, collections: &mut [MarkCollection]) -> MarksChange {
        if header == MarkHeader::Bar && collections.is_empty() {
            return MarksChange::None;
        }
        let value = self.next_value(header);
        let mut changed = false;
        for c in collections.iter_mut() {
            let field = match header {
                MarkHeader::Lines => &mut c.show_grid_lines,
                MarkHeader::TailLines => &mut c.show_tail_lines,
                MarkHeader::Bar => &mut c.show_marks,
                MarkHeader::Lock => &mut c.is_locked,
            };
            if *field != value {
                *field = value;
                changed = true;
            }
        }
        let flag = match header {
            MarkHeader::Lines => &mut self.lines,
            MarkHeader::TailLines => &mut self.tail,
            MarkHeader::Bar => &mut self.bar,
            MarkHeader::Lock => &mut self.lock,
        };
        *flag = !*flag;
        if changed {
            MarksChange::Properties
        } else {
            MarksChange::None
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Row commands
// ---------------------------------------------------------------------------------------------------------------------

/// F4: `Mark Collection`, then `Mark Collection - 2`..., at the end, with the constructor's defaults. The first in a list is the default.
pub fn add_collection(collections: &mut Vec<MarkCollection>) -> Uuid {
    let mut c = MarkCollection { name: unique_name(DEFAULT_NAME, collections, None), ..MarkCollection::default() };
    c.is_default = collections.is_empty();
    let id = c.id;
    collections.push(c);
    id
}

/// F17: no confirmation. A deleted default passes to the first visible collection, else the first. KEEP: links to it are left dangling.
pub fn delete_collection(collections: &mut Vec<MarkCollection>, id: Uuid) -> MarksChange {
    let Some(i) = index_of(collections, id) else { return MarksChange::None };
    let removed = collections.remove(i);
    if removed.is_default && !collections.is_empty() {
        let at = collections.iter().position(is_visible).unwrap_or(0);
        collections[at].is_default = true;
    }
    MarksChange::Collections
}

/// F9: the radio group. Choosing one clears the rest.
pub fn set_active(collections: &mut [MarkCollection], id: Uuid) -> MarksChange {
    if index_of(collections, id).is_none() {
        return MarksChange::None;
    }
    let mut changed = false;
    for c in collections.iter_mut() {
        let on = c.id == id;
        if c.is_default != on {
            c.is_default = on;
            changed = true;
        }
    }
    if changed {
        MarksChange::Properties
    } else {
        MarksChange::None
    }
}

/// A row's checkbox or check menu item.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MarkFlag {
    Lines,
    TailLines,
    Bar,
    Lock,
    SolidLine,
    Bold,
}

/// F10 to F13, F19, F20.
pub fn set_flag(collections: &mut [MarkCollection], id: Uuid, flag: MarkFlag, on: bool) -> MarksChange {
    let Some(i) = index_of(collections, id) else { return MarksChange::None };
    let c = &mut collections[i];
    let field = match flag {
        MarkFlag::Lines => &mut c.show_grid_lines,
        MarkFlag::TailLines => &mut c.show_tail_lines,
        MarkFlag::Bar => &mut c.show_marks,
        MarkFlag::Lock => &mut c.is_locked,
        MarkFlag::SolidLine => &mut c.decorator.is_solid_line,
        MarkFlag::Bold => &mut c.decorator.is_bold,
    };
    if *field == on {
        return MarksChange::None;
    }
    *field = on;
    MarksChange::Properties
}

/// F15's message while typing, or `None` when the text is fine. A blank name is fine here: ending the edit restores the old one.
pub fn rename_error(collections: &[MarkCollection], id: Uuid, text: &str) -> Option<String> {
    let proposed = normalize_name(text);
    if proposed.is_empty() || is_unique_name(collections, proposed, Some(id)) {
        None
    } else {
        Some(format!("A Mark Collection named \"{proposed}\" already exists."))
    }
}

/// How F15's edit ended.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum RenameEnd {
    /// The name is now the trimmed text.
    Renamed,
    /// The trimmed text is the name already.
    Unchanged,
    /// Blank: the old name stays.
    Restored,
    /// A duplicate: the field stays open with its message, even after focus has gone.
    StillInvalid,
}

/// F15, on Return or loss of focus. A case-only rename works, since the comparison with the old name is exact.
pub fn end_rename(collections: &mut [MarkCollection], id: Uuid, text: &str) -> RenameEnd {
    let proposed = normalize_name(text);
    let Some(i) = index_of(collections, id) else { return RenameEnd::Restored };
    if proposed.is_empty() {
        return RenameEnd::Restored;
    }
    if !is_unique_name(collections, proposed, Some(id)) {
        return RenameEnd::StillInvalid;
    }
    if collections[i].name == proposed {
        return RenameEnd::Unchanged;
    }
    collections[i].name = proposed.to_string();
    RenameEnd::Renamed
}

/// The `Mark.StartTime` setter: a different start raises a duration under 25 ms to 25 ms.
pub fn set_mark_start(mark: &mut Mark, start: TimeSpan) {
    if mark.start_time == start {
        return;
    }
    mark.start_time = start;
    let minimum = TimeSpan::from_ms_i(25);
    if mark.duration < minimum {
        mark.duration = minimum;
    }
}

/// F18's prompt: title, text, default and range.
pub const OFFSET_TITLE: &str = "Offset Mark Time";
pub const OFFSET_PROMPT: &str = "Time to offset (in milliseconds):";
pub const OFFSET_DEFAULT_MS: i32 = 1000;
pub const OFFSET_RANGE_MS: (i32, i32) = (-30_000, 30_000);

/// F18: every mark's start moves by `ms`; durations stay. 0 changes nothing. KEEP: no clamp, so marks can go below zero or past the end,
/// and can be moved back by offsetting the other way. FIX, for veto: upstream raises no change here, so the sequence isn't modified and
/// the grid keeps the old lines; this reports the change like any other.
pub fn offset_time(collections: &mut [MarkCollection], id: Uuid, ms: i32) -> MarksChange {
    let Some(i) = index_of(collections, id) else { return MarksChange::None };
    if ms == 0 || collections[i].marks.is_empty() {
        return MarksChange::None;
    }
    let delta = TimeSpan::from_ms_i(ms as i64);
    for m in &mut collections[i].marks {
        let start = m.start_time + delta;
        set_mark_start(m, start);
    }
    MarksChange::Collections
}

/// F21.
pub fn set_color(collections: &mut [MarkCollection], id: Uuid, color: Color) -> MarksChange {
    let Some(i) = index_of(collections, id) else { return MarksChange::None };
    if collections[i].decorator.color == color {
        return MarksChange::None;
    }
    collections[i].decorator.color = color;
    MarksChange::Properties
}

/// F22's items, 1 to 6. Old files can hold other values, and then none is checked.
pub const LEVELS: std::ops::RangeInclusive<i32> = 1..=6;

/// F22. FIX: a native picker keeps the current value checked, where upstream's re-click unchecks it until the panel is rebuilt.
pub fn set_level(collections: &mut [MarkCollection], id: Uuid, level: i32) -> MarksChange {
    let Some(i) = index_of(collections, id) else { return MarksChange::None };
    if collections[i].level == level {
        return MarksChange::None;
    }
    collections[i].level = level;
    MarksChange::Properties
}

/// F23. KEEP: the link stays, though its menu hides, and comes back if the type returns.
pub fn set_kind(collections: &mut [MarkCollection], id: Uuid, kind: MarkCollectionType) -> MarksChange {
    let Some(i) = index_of(collections, id) else { return MarksChange::None };
    if collections[i].collection_type == kind {
        return MarksChange::None;
    }
    collections[i].collection_type = kind;
    MarksChange::Properties
}

/// The five types in enum order, for F23's submenu.
pub const KINDS: [MarkCollectionType; 5] =
    [MarkCollectionType::Generic, MarkCollectionType::Phrase, MarkCollectionType::Word, MarkCollectionType::Phoneme, MarkCollectionType::State];

/// One item of F24's Linked To submenu. `id` is `None` for `None`.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkChoice {
    pub id: Option<Uuid>,
    pub name: String,
    pub checked: bool,
}

/// What a type links to: phonemes to words, words to phrases.
pub fn link_target(kind: MarkCollectionType) -> Option<MarkCollectionType> {
    match kind {
        MarkCollectionType::Phoneme => Some(MarkCollectionType::Word),
        MarkCollectionType::Word => Some(MarkCollectionType::Phrase),
        _ => None,
    }
}

/// F24, built when the menu opens: every collection of the target type in list order, then `None`. `None` hides the submenu. A link to a
/// deleted collection checks nothing, not even `None` (F17).
pub fn link_choices(collections: &[MarkCollection], id: Uuid) -> Option<Vec<LinkChoice>> {
    let c = &collections[index_of(collections, id)?];
    let target = link_target(c.collection_type)?;
    let mut out: Vec<LinkChoice> = collections
        .iter()
        .filter(|o| o.collection_type == target)
        .map(|o| LinkChoice { id: Some(o.id), name: o.name.clone(), checked: c.linked_mark_collection_id == Some(o.id) })
        .collect();
    out.push(LinkChoice { id: None, name: "None".to_string(), checked: c.linked_mark_collection_id.is_none() });
    Some(out)
}

/// F24's choice. A real parent first loses whatever child it had, so one parent has at most one child; `None` touches nothing else. FIX, for
/// veto: upstream's link is a plain property that raises no change, so a link alone doesn't modify the sequence; here it does.
pub fn set_link(collections: &mut [MarkCollection], id: Uuid, parent: Option<Uuid>) -> MarksChange {
    let Some(i) = index_of(collections, id) else { return MarksChange::None };
    if collections[i].linked_mark_collection_id == parent {
        return MarksChange::None;
    }
    if let Some(p) = parent {
        for c in collections.iter_mut() {
            if c.linked_mark_collection_id == Some(p) {
                c.linked_mark_collection_id = None;
            }
        }
    }
    collections[i].linked_mark_collection_id = parent;
    MarksChange::Properties
}

/// F6: Export needs a collection.
pub fn can_export(collections: &[MarkCollection]) -> bool {
    !collections.is_empty()
}

/// F68 step 6, also run after an import elsewhere: with collections and no default, the first becomes the default.
pub fn ensure_default_collection(collections: &mut [MarkCollection]) -> bool {
    if collections.is_empty() || collections.iter().any(|c| c.is_default) {
        return false;
    }
    collections[0].is_default = true;
    true
}

#[cfg(test)]
mod tests;
