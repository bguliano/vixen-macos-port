//! Streaming ACN (E1.31) setup (F51 to F94).

use parking_lot::Mutex;
use std::net::Ipv4Addr;
use vixen_output::modules::{E131Config, UniverseEntry};
use vixen_output::net::MulticastChoice;

const MAX_UNIVERSE: i32 = 64000;
const NO_MORE_UNIVERSES: &str = "No additional universes can be added. Universe numbers must be between 1 and 64000 inclusive.";

/// A row of the universe grid. `start` is one-based here, and zero-based in the file.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UniverseRow {
    pub start: i32,
    pub active: bool,
    pub universe: i32,
    pub size: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UniverseColumn {
    Start,
    Active,
    Universe,
    Size,
}

/// F51: each stored universe with `Start + 1`. With automatic starts, opening recalculates and discards the stored ones.
pub fn e131_rows_from_data(data: &E131Config) -> Vec<UniverseRow> {
    let mut rows: Vec<UniverseRow> = data
        .universes
        .iter()
        .flatten()
        .map(|u| UniverseRow { start: u.start.saturating_add(1), active: u.active, universe: u.universe, size: u.size })
        .collect();
    if data.auto_populate {
        e131_recompute_starts(&mut rows);
    }
    rows
}

/// F62: the first row is 1, and each next row follows the one before.
pub fn e131_recompute_starts(rows: &mut [UniverseRow]) {
    let mut next = 1i32;
    for row in rows {
        row.start = next;
        next = next.saturating_add(row.size);
    }
}

fn highest_universe(rows: &[UniverseRow]) -> i32 {
    rows.iter().map(|r| r.universe).max().unwrap_or(0)
}

/// F69: the next universe number, with the last row's size or 510. Upstream happily adds 64001, and gives a manual row a start
/// its OK checks throw on.
pub fn e131_add_universe(rows: &mut Vec<UniverseRow>, manual: bool) -> Result<usize, String> {
    if e131_max_to_add(rows) < 1 {
        return Err(NO_MORE_UNIVERSES.to_string());
    }
    e131_add_universes(rows, 1, manual);
    Ok(rows.len() - 1)
}

/// F70: how many more universes fit.
pub fn e131_max_to_add(rows: &[UniverseRow]) -> i32 {
    MAX_UNIVERSE - highest_universe(rows)
}

/// F70, in one pass: up to 63,999 rows must not rescan for each one.
pub fn e131_add_universes(rows: &mut Vec<UniverseRow>, count: i32, manual: bool) {
    let mut universe = highest_universe(rows);
    let size = rows.last().map_or(510, |r| r.size);
    let mut start = rows.last().map_or(1, |r| r.start.saturating_add(r.size));
    let count = count.clamp(0, MAX_UNIVERSE - universe);
    rows.reserve(count as usize);
    for _ in 0..count {
        universe += 1;
        rows.push(UniverseRow { start, active: true, universe, size });
        start = start.saturating_add(size);
    }
    if !manual {
        e131_recompute_starts(rows);
    }
}

/// F68, F71: removes the given rows.
pub fn e131_delete_rows(rows: &mut Vec<UniverseRow>, indices: &[usize], manual: bool) {
    let mut sorted: Vec<usize> = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut next = sorted.iter().copied().peekable();
    let mut position = 0;
    rows.retain(|_| {
        let remove = next.peek() == Some(&position);
        if remove {
            next.next();
        }
        position += 1;
        !remove
    });
    if !manual {
        e131_recompute_starts(rows);
    }
}

/// F72: one default row is left, universe 1, start 1, size 510, on.
pub fn e131_delete_all(rows: &mut Vec<UniverseRow>) {
    rows.clear();
    rows.push(UniverseRow { start: 1, active: true, universe: 1, size: 510 });
}

/// F67: numeric, where upstream compares the numbers as text.
pub fn e131_sort(rows: &mut [UniverseRow], column: UniverseColumn, ascending: bool) {
    let key = |r: &UniverseRow| match column {
        UniverseColumn::Start => r.start,
        UniverseColumn::Active => r.active as i32,
        UniverseColumn::Universe => r.universe,
        UniverseColumn::Size => r.size,
    };
    if ascending {
        rows.sort_by_key(key);
    } else {
        rows.sort_by_key(|r| std::cmp::Reverse(key(r)));
    }
}

/// F62, F64, F65: the text a cell may leave with. Anything that isn't a number reads as 0, which is out of range.
pub fn e131_validate_cell(column: UniverseColumn, text: &str) -> Result<i32, String> {
    let (low, high, message) = match column {
        UniverseColumn::Start => (1, 99999, "Start must be between 1 and 99999 inclusive"),
        UniverseColumn::Universe => (1, MAX_UNIVERSE, "Universe must be between 1 and 64000 inclusive"),
        UniverseColumn::Size => (1, 512, "Size must be between 1 and 512 inclusive"),
        UniverseColumn::Active => return Ok(0),
    };
    let value = text.trim().parse::<i32>().unwrap_or(0);
    if (low..=high).contains(&value) {
        Ok(value)
    } else {
        Err(message.to_string())
    }
}

/// F79. An invalid value beeps and keeps the focus.
pub fn e131_validate_repeat(text: &str) -> Option<i32> {
    text.trim().parse::<i32>().ok().filter(|v| (0..=99).contains(v))
}

/// F80. The validator allows 10000, though the box takes two characters, so 99 is the most a user can type.
pub fn e131_validate_suppress(text: &str) -> Option<i32> {
    text.trim().parse::<i32>().ok().filter(|v| (0..=10000).contains(v))
}

/// Where the universes go. Exactly one is set, or neither for a controller that hasn't been set up.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct E131Destination {
    pub unicast: Option<String>,
    pub multicast: Option<String>,
}

/// Another running E1.31 controller (F86).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct E131Peer {
    pub unicast: Option<String>,
    pub multicast: Option<String>,
    pub universes: Vec<i32>,
}

impl E131Peer {
    pub fn from_config(config: &E131Config) -> E131Peer {
        E131Peer {
            unicast: config.unicast.clone(),
            multicast: config.multicast.clone(),
            universes: config.universes.iter().flatten().map(|u: &UniverseEntry| u.universe).collect(),
        }
    }
}

/// One alert OK may have to show before it saves (F87).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum E131OkStep {
    /// "Save Anyway" goes on, "Go Back" stays.
    Warn { title: String, text: String },
    /// "Review Settings" stays, "Continue" goes on. Upstream's OK saves here and neither button lets you review.
    ReviewOrContinue { text: String },
    /// OK stays.
    Block { title: String, text: String },
}

/// F84's text.
const OVERLAP_TEXT: &str = "The start values seem to be setup in an unusual way. You are sending identical lighting values to multiple sACN outputs. The start value column refers to where a given universe starts reading values in from the list of output channel data from Vixen. For example, setting Universe 1's start value to 5 will map Channel 1 in Universe 1 to output channel #5 in the Vixen controller setup. Would you like to review your settings?";

fn starts_overlap(rows: &[UniverseRow]) -> bool {
    let bounds = |r: &UniverseRow| (r.start as i64, r.start as i64 + r.size as i64 - 1);
    // Sorted by start, a range can only touch those that begin before it ends, so this isn't quadratic for thousands of rows.
    let mut sorted: Vec<(i64, i64)> = rows.iter().map(bounds).collect();
    sorted.sort_unstable();
    let mut highest_end = i64::MIN;
    for (i, (low, high)) in sorted.iter().enumerate() {
        if i > 0 && *low <= highest_end {
            return true;
        }
        highest_end = highest_end.max(*high);
    }
    false
}

/// F83 to F87: all rows count, active or not. Steps after a `Block` still appear, because the UI stops at the first that stays.
pub fn e131_ok_steps(rows: &[UniverseRow], destination: &E131Destination, output_count: u32, others: &[E131Peer]) -> Vec<E131OkStep> {
    let mut steps = Vec::new();
    let count = output_count as i64;
    if count > 0 {
        let mut messages: Vec<String> = Vec::new();
        let mut total = 0i64;
        for row in rows {
            let (start, size) = (row.start as i64, row.size as i64);
            if start > count {
                messages.push(format!("Universe {} start channel exceeds controller output count of {count}", row.universe));
            }
            if start + size - 1 > count {
                messages.push(format!("Universe {} {start} + {size} exceeds controller output count of {count}", row.universe));
            }
            total += size;
        }
        if total != count {
            messages.push(format!(
                "Total universe channel count of {total} does not match the controller output count of {count}.\n\nCheck your universe setup or adjust the amount of output channels to match."
            ));
        }
        if !messages.is_empty() {
            steps.push(E131OkStep::Warn { title: "Universe Channel / Output count conflict.".to_string(), text: messages.join("\n\n") });
        }
    }
    if starts_overlap(rows) {
        steps.push(E131OkStep::ReviewOrContinue { text: OVERLAP_TEXT.to_string() });
    }
    let mut seen = std::collections::HashSet::with_capacity(rows.len());
    if !rows.iter().all(|r| seen.insert(r.universe)) {
        steps.push(E131OkStep::Block { title: "Error".to_string(), text: "Universe numbers must be unique.".to_string() });
    }
    for peer in others {
        // Unicast against multicast counts as a conflict, as upstream: harmless, and safer.
        let different_unicast = matches!((&peer.unicast, &destination.unicast), (Some(a), Some(b)) if a != b);
        let different_multicast = matches!((&peer.multicast, &destination.multicast), (Some(a), Some(b)) if a != b);
        if different_unicast || different_multicast {
            continue;
        }
        if let Some(row) = rows.iter().find(|r| peer.universes.contains(&r.universe)) {
            steps.push(E131OkStep::Block {
                title: "Existing Universe".to_string(),
                text: format!(
                    "Universe {} already exists on another controller transmitting to the same device or network. Please configure a different universe to prevent hardware errors.",
                    row.universe
                ),
            });
            break;
        }
    }
    steps
}

/// The options on the Advanced tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E131Options {
    pub warnings: bool,
    pub statistics: bool,
    pub repeat_count: i32,
    pub suppress_count: i32,
    pub auto_populate: bool,
    pub blind: bool,
    pub priority: i32,
}

/// F88: what a name for each output becomes. The same text `E131Config::output_names` gives, from the same data.
pub fn e131_output_names(rows: &[UniverseRow], unicast: Option<&str>, output_count: u32) -> Vec<String> {
    let config = E131Config {
        unicast: unicast.map(str::to_string),
        universes: Some(rows.iter().map(|r| UniverseEntry::new(r.active, r.universe, r.start - 1, r.size)).collect()),
        ..Default::default()
    };
    config.output_names(output_count as usize)
}

/// F88.
pub fn e131_apply(data: &mut E131Config, rows: &[UniverseRow], options: &E131Options, destination: &E131Destination) {
    data.warnings = options.warnings;
    data.statistics = options.statistics;
    data.event_repeat_count = options.repeat_count;
    data.event_suppress_count = options.suppress_count;
    data.auto_populate = options.auto_populate;
    data.blind = options.blind;
    data.priority = options.priority;
    data.unicast = destination.unicast.clone();
    data.multicast = destination.multicast.clone();
    data.universes = Some(
        rows.iter()
            .map(|r| UniverseEntry {
                unicast: destination.unicast.clone(),
                multicast: destination.multicast.clone(),
                ..UniverseEntry::new(r.active, r.universe, r.start - 1, r.size)
            })
            .collect(),
    );
}

/// One entry of the destination picker. It maps to an id or address, not to its text, so `Multicast UNKNOWN` keeps the saved id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E131DestinationEntry {
    pub text: String,
    pub unicast: Option<String>,
    pub multicast: Option<String>,
}

impl E131DestinationEntry {
    pub fn is_multicast(&self) -> bool {
        self.multicast.is_some()
    }
}

/// F53.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct E131DestinationList {
    pub entries: Vec<E131DestinationEntry>,
    pub selected: Option<usize>,
    /// Show "The selected multicast interface was not found, be sure to verify your destination."
    pub missing_interface: bool,
}

pub const MISSING_INTERFACE_TEXT: &str = "The selected multicast interface was not found, be sure to verify your destination.";

fn unicast_entry(address: &str) -> E131DestinationEntry {
    E131DestinationEntry { text: format!("Unicast {address}"), unicast: Some(address.to_string()), multicast: None }
}

/// F53: multicast interfaces, then the known unicasts. A saved interface this machine doesn't have stays selected as UNKNOWN, so OK
/// keeps the old id. `primary` stands in for upstream's default of "Multicast Local Area Connection".
pub fn e131_destination_list(
    choices: &[MulticastChoice],
    unicasts: &[String],
    saved: &E131Destination,
    primary: Option<&str>,
) -> E131DestinationList {
    let mut sorted: Vec<&MulticastChoice> = choices.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    let mut entries: Vec<E131DestinationEntry> = sorted
        .iter()
        .map(|c| E131DestinationEntry { text: c.text.clone(), unicast: None, multicast: Some(c.id.clone()) })
        .collect();
    let mut missing_interface = false;
    if let Some(id) = saved.multicast.as_deref().filter(|id| !entries.iter().any(|e| e.multicast.as_deref() == Some(id))) {
        entries.push(E131DestinationEntry {
            text: "Multicast UNKNOWN".to_string(),
            unicast: None,
            multicast: Some(id.to_string()),
        });
        missing_interface = true;
    }
    let multicast_count = entries.len();
    entries.extend(unicasts.iter().map(|u| unicast_entry(u)));
    let selected = if let Some(address) = &saved.unicast {
        entries[multicast_count..].iter().position(|e| e.unicast.as_ref() == Some(address)).map(|i| i + multicast_count)
    } else if let Some(id) = &saved.multicast {
        entries.iter().position(|e| e.multicast.as_ref() == Some(id))
    } else {
        primary.and_then(|p| entries.iter().position(|e| e.multicast.as_deref() == Some(p)))
    };
    E131DestinationList { entries, selected, missing_interface }
}

/// The process-wide table of unicast destinations, filled from every controller's saved value and not saved separately (F53).
static KNOWN_UNICASTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Adds to the process-wide table, which stays sorted.
pub fn remember_unicast(address: &str) {
    let mut known = KNOWN_UNICASTS.lock();
    if let Err(i) = known.binary_search_by(|k| k.as_str().cmp(address)) {
        known.insert(i, address.to_string());
    }
}

pub fn known_unicasts() -> Vec<String> {
    KNOWN_UNICASTS.lock().clone()
}

/// F54.
pub fn e131_add_unicast(known: &mut Vec<String>, text: &str) -> Result<(), String> {
    match known.binary_search_by(|k| k.as_str().cmp(text)) {
        Ok(_) => Err("Error - Duplicate IP Address".to_string()),
        Err(i) => {
            known.insert(i, text.to_string());
            Ok(())
        }
    }
}

/// F54, on the process-wide table.
pub fn add_known_unicast(text: &str) -> Result<(), String> {
    e131_add_unicast(&mut KNOWN_UNICASTS.lock(), text)
}

/// F55.
pub fn e131_remove_unicast(known: &mut Vec<String>, text: &str, others: &[E131Peer]) -> Result<(), String> {
    if others.iter().any(|p| p.unicast.as_deref() == Some(text)) {
        return Err("This destination is in use by another instance of the plugin and cannot be removed.".to_string());
    }
    known.retain(|k| k != text);
    Ok(())
}

/// F55, on the process-wide table.
pub fn remove_known_unicast(text: &str, others: &[E131Peer]) -> Result<(), String> {
    e131_remove_unicast(&mut KNOWN_UNICASTS.lock(), text, others)
}

pub const MULTICAST_NOT_REMOVABLE: &str = "Multicast destinations cannot be removed.";

/// F57: OK needs something that parses as IPv4.
pub fn unicast_ip_is_valid(text: &str) -> bool {
    text.trim().parse::<Ipv4Addr>().is_ok()
}

/// F58: a name needs to be there.
pub fn unicast_name_is_valid(text: &str) -> bool {
    !text.trim().is_empty()
}

/// What the sheet edits. The unicast table is shared, so it isn't here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E131Draft {
    pub title: String,
    pub output_count: u32,
    pub rows: Vec<UniverseRow>,
    pub manual: bool,
    pub warnings: bool,
    pub statistics: bool,
    pub blind: bool,
    pub priority: i32,
    pub repeat_text: String,
    pub suppress_text: String,
    /// F67: the warning about sorting shows once per dialog.
    pub shown_sort_warning: bool,
}

/// F51: `Streaming ACN (E1.31) Configuration - {controller name}`.
pub fn e131_draft(data: &E131Config, controller_name: &str, output_count: u32) -> E131Draft {
    E131Draft {
        title: format!("Streaming ACN (E1.31) Configuration - {controller_name}"),
        output_count,
        rows: e131_rows_from_data(data),
        manual: !data.auto_populate,
        warnings: data.warnings,
        statistics: data.statistics,
        blind: data.blind,
        priority: data.priority.clamp(0, 200),
        repeat_text: data.event_repeat_count.to_string(),
        suppress_text: data.event_suppress_count.to_string(),
        shown_sort_warning: false,
    }
}

impl E131Draft {
    /// F74. Toggling back to automatic recalculates at once.
    pub fn set_manual(&mut self, manual: bool) {
        self.manual = manual;
        if !manual {
            e131_recompute_starts(&mut self.rows);
        }
    }

    /// F66: a cell's new text. `Err` is the alert's text, and the cell stays in edit.
    pub fn set_cell(&mut self, row: usize, column: UniverseColumn, text: &str) -> Result<(), String> {
        let value = e131_validate_cell(column, text)?;
        let Some(target) = self.rows.get_mut(row) else {
            return Ok(());
        };
        match column {
            UniverseColumn::Start => target.start = value,
            UniverseColumn::Universe => target.universe = value,
            UniverseColumn::Size => target.size = value,
            UniverseColumn::Active => {}
        }
        if !self.manual && matches!(column, UniverseColumn::Start | UniverseColumn::Size) {
            e131_recompute_starts(&mut self.rows);
        }
        Ok(())
    }

    /// F67: `Some` is the warning to show. Only the first click in a dialog warns.
    pub fn sort(&mut self, column: UniverseColumn, ascending: bool) -> Option<String> {
        if !self.manual {
            if self.shown_sort_warning {
                return None;
            }
            self.shown_sort_warning = true;
            return Some("Sorting is not available while Vixen manages the start values.".to_string());
        }
        e131_sort(&mut self.rows, column, ascending);
        None
    }

    /// F79, F80, F88: an unreadable box saves as 0, as upstream's getter reads it.
    pub fn options(&self) -> E131Options {
        E131Options {
            warnings: self.warnings,
            statistics: self.statistics,
            repeat_count: self.repeat_text.trim().parse().unwrap_or(0),
            suppress_count: self.suppress_text.trim().parse().unwrap_or(0),
            auto_populate: !self.manual,
            blind: self.blind,
            priority: self.priority,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(universe: i32, start: i32, size: i32) -> UniverseRow {
        UniverseRow { start, active: true, universe, size }
    }

    #[test]
    fn automatic_starts_follow_the_sizes() {
        let mut rows = vec![row(1, 0, 510), row(2, 0, 510), row(3, 0, 100)];
        e131_recompute_starts(&mut rows);
        assert_eq!(rows.iter().map(|r| r.start).collect::<Vec<_>>(), [1, 511, 1021]);
    }

    #[test]
    fn adding_a_universe() {
        let mut rows = Vec::new();
        assert_eq!(e131_add_universe(&mut rows, false), Ok(0));
        assert_eq!(rows[0], UniverseRow { start: 1, active: true, universe: 1, size: 510 });
        let mut rows = vec![row(7, 1, 170)];
        e131_add_universe(&mut rows, false).unwrap();
        assert_eq!((rows[1].universe, rows[1].size, rows[1].start), (8, 170, 171));
        // A manual row follows the row before it.
        let mut rows = vec![row(7, 100, 170)];
        e131_add_universe(&mut rows, true).unwrap();
        assert_eq!(rows[1].start, 270);
        assert_eq!(rows[0].start, 100, "manual starts stay");
    }

    #[test]
    fn universe_64000_is_the_last() {
        let mut rows = vec![row(64000, 1, 10)];
        assert_eq!(e131_max_to_add(&rows), 0);
        assert_eq!(e131_add_universe(&mut rows, false).unwrap_err(), NO_MORE_UNIVERSES);
        assert_eq!(rows.len(), 1);
        assert_eq!(e131_max_to_add(&[row(63990, 1, 10)]), 10);
    }

    #[test]
    fn adding_many_goes_in_one_pass_and_stops_at_the_limit() {
        let mut rows = vec![row(1, 1, 512)];
        e131_add_universes(&mut rows, 63999, false);
        assert_eq!(rows.len(), 64000);
        assert_eq!(rows.last().unwrap().universe, 64000);
        assert_eq!(rows[1].start, 513);
        assert_eq!(rows[63999].start, 1 + 63999 * 512);
        let mut rows = vec![row(63999, 1, 10)];
        e131_add_universes(&mut rows, 50, false);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn deleting_rows_and_all_rows() {
        let mut rows = vec![row(1, 1, 10), row(2, 11, 10), row(3, 21, 10), row(4, 31, 10)];
        e131_delete_rows(&mut rows, &[2, 0, 2], false);
        assert_eq!(rows.iter().map(|r| r.universe).collect::<Vec<_>>(), [2, 4]);
        assert_eq!(rows.iter().map(|r| r.start).collect::<Vec<_>>(), [1, 11]);
        // The Delete key isn't limited by the row count.
        e131_delete_rows(&mut rows, &[0, 1], false);
        assert!(rows.is_empty());
        e131_delete_all(&mut rows);
        assert_eq!(rows, [UniverseRow { start: 1, active: true, universe: 1, size: 510 }]);
    }

    #[test]
    fn deleting_keeps_manual_starts() {
        let mut rows = vec![row(1, 5, 10), row(2, 100, 10)];
        e131_delete_rows(&mut rows, &[0], true);
        assert_eq!(rows[0].start, 100);
    }

    #[test]
    fn sorting_is_numeric() {
        let mut rows = vec![row(10, 1, 1), row(9, 2, 1), row(100, 3, 1)];
        e131_sort(&mut rows, UniverseColumn::Universe, true);
        assert_eq!(rows.iter().map(|r| r.universe).collect::<Vec<_>>(), [9, 10, 100]);
        e131_sort(&mut rows, UniverseColumn::Universe, false);
        assert_eq!(rows.iter().map(|r| r.universe).collect::<Vec<_>>(), [100, 10, 9]);
    }

    #[test]
    fn cell_limits_and_messages() {
        for bad in ["0", "64001", "", "x"] {
            assert_eq!(e131_validate_cell(UniverseColumn::Universe, bad).unwrap_err(), "Universe must be between 1 and 64000 inclusive");
        }
        assert_eq!(e131_validate_cell(UniverseColumn::Universe, "1"), Ok(1));
        assert_eq!(e131_validate_cell(UniverseColumn::Universe, "64000"), Ok(64000));
        for bad in ["0", "100000"] {
            assert_eq!(e131_validate_cell(UniverseColumn::Start, bad).unwrap_err(), "Start must be between 1 and 99999 inclusive");
        }
        for bad in ["0", "513"] {
            assert_eq!(e131_validate_cell(UniverseColumn::Size, bad).unwrap_err(), "Size must be between 1 and 512 inclusive");
        }
    }

    #[test]
    fn the_count_warning_reads_like_upstream() {
        let steps = e131_ok_steps(&[row(1, 1, 510)], &E131Destination::default(), 512, &[]);
        assert_eq!(
            steps,
            [E131OkStep::Warn {
                title: "Universe Channel / Output count conflict.".to_string(),
                text: "Total universe channel count of 510 does not match the controller output count of 512.\n\nCheck your universe setup or adjust the amount of output channels to match.".to_string(),
            }]
        );
    }

    #[test]
    fn out_of_range_rows_each_get_a_line() {
        let steps = e131_ok_steps(&[row(5, 11, 5)], &E131Destination::default(), 10, &[]);
        let E131OkStep::Warn { text, .. } = &steps[0] else { panic!() };
        assert!(text.starts_with("Universe 5 start channel exceeds controller output count of 10\n\nUniverse 5 11 + 5 exceeds controller output count of 10\n\nTotal"));
    }

    #[test]
    fn no_outputs_skips_the_count_checks() {
        assert!(e131_ok_steps(&[row(1, 1, 510)], &E131Destination::default(), 0, &[]).is_empty());
    }

    #[test]
    fn overlap_and_duplicates_count_inactive_rows() {
        let overlapping = [row(1, 1, 510), row(2, 510, 510)];
        assert!(matches!(e131_ok_steps(&overlapping, &E131Destination::default(), 0, &[])[0], E131OkStep::ReviewOrContinue { .. }));
        assert!(e131_ok_steps(&[row(1, 1, 510), row(2, 511, 510)], &E131Destination::default(), 0, &[]).is_empty());
        let mut inactive = row(2, 510, 510);
        inactive.active = false;
        assert!(!e131_ok_steps(&[row(1, 1, 510), inactive], &E131Destination::default(), 0, &[]).is_empty());
        let same = [row(3, 1, 10), row(3, 11, 10)];
        assert_eq!(
            e131_ok_steps(&same, &E131Destination::default(), 0, &[]),
            [E131OkStep::Block { title: "Error".to_string(), text: "Universe numbers must be unique.".to_string() }]
        );
    }

    #[test]
    fn peers_conflict_unless_clearly_elsewhere() {
        let rows = [row(5, 1, 10)];
        let unicast = |a: &str| E131Destination { unicast: Some(a.to_string()), multicast: None };
        let multicast = |a: &str| E131Destination { unicast: None, multicast: Some(a.to_string()) };
        let peer = |d: &E131Destination| E131Peer { unicast: d.unicast.clone(), multicast: d.multicast.clone(), universes: vec![5] };
        let conflicts = |mine: &E131Destination, theirs: &E131Destination| !e131_ok_steps(&rows, mine, 0, &[peer(theirs)]).is_empty();
        assert!(conflicts(&unicast("a"), &unicast("a")));
        assert!(!conflicts(&unicast("a"), &unicast("b")));
        assert!(!conflicts(&multicast("en0"), &multicast("en1")));
        assert!(conflicts(&multicast("en0"), &multicast("en0")));
        assert!(conflicts(&unicast("a"), &multicast("en0")), "one of each counts, as upstream");
        let steps = e131_ok_steps(&rows, &unicast("a"), 0, &[peer(&unicast("a"))]);
        assert_eq!(
            steps,
            [E131OkStep::Block {
                title: "Existing Universe".to_string(),
                text: "Universe 5 already exists on another controller transmitting to the same device or network. Please configure a different universe to prevent hardware errors.".to_string(),
            }]
        );
        let other_universe = E131Peer { universes: vec![6], ..peer(&unicast("a")) };
        assert!(e131_ok_steps(&rows, &unicast("a"), 0, &[other_universe]).is_empty());
    }

    #[test]
    fn output_names_match_the_table() {
        let rows = [row(5, 3, 4)];
        let names = e131_output_names(&rows, None, 10);
        assert_eq!(names[0], "Output #1");
        assert_eq!(names[2], "#3 5-1: Multicast");
        assert_eq!(names[5], "#6 5-4: Multicast");
        assert_eq!(names[6], "Output #7");
        assert_eq!(e131_output_names(&rows, Some("10.0.0.5"), 10)[2], "#3 5-1: 10.0.0.5");
    }

    #[test]
    fn apply_stores_starts_zero_based_and_reads_back() {
        let mut data = E131Config::default();
        let rows = [row(5, 3, 4)];
        let options = E131Options {
            warnings: true,
            statistics: true,
            repeat_count: 2,
            suppress_count: 3,
            auto_populate: false,
            blind: true,
            priority: 150,
        };
        let destination = E131Destination { unicast: Some("10.0.0.5".to_string()), multicast: None };
        e131_apply(&mut data, &rows, &options, &destination);
        let universe = &data.universes.as_ref().unwrap()[0];
        assert_eq!((universe.start, universe.universe, universe.size, universe.ttl), (2, 5, 4, 64));
        assert_eq!(data.unicast.as_deref(), Some("10.0.0.5"));
        assert_eq!(data.multicast, None);
        let back = E131Config::from_xml(&data.to_xml());
        assert_eq!(back, data);
        assert_eq!(e131_rows_from_data(&back), rows);
    }

    #[test]
    fn opening_recalculates_automatic_starts_and_keeps_manual_ones() {
        // A new controller's default says start 1 in the file, which would read 2.
        let data = E131Config::default();
        assert_eq!(e131_rows_from_data(&data)[0].start, 1);
        let manual = E131Config { auto_populate: false, ..E131Config::default() };
        assert_eq!(e131_rows_from_data(&manual)[0].start, 2);
    }

    fn choice(id: &str, name: &str) -> MulticastChoice {
        MulticastChoice { id: id.to_string(), name: name.to_string(), text: format!("Multicast {name} ({id})") }
    }

    #[test]
    fn a_missing_interface_stays_selected_as_unknown() {
        let saved = E131Destination { unicast: None, multicast: Some("{GUID}".to_string()) };
        let list = e131_destination_list(&[choice("en0", "Wi-Fi")], &["10.0.0.5".to_string()], &saved, Some("en0"));
        assert!(list.missing_interface);
        let selected = &list.entries[list.selected.unwrap()];
        assert_eq!(selected.text, "Multicast UNKNOWN");
        assert_eq!(selected.multicast.as_deref(), Some("{GUID}"));
        let texts: Vec<&str> = list.entries.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(texts, ["Multicast Wi-Fi (en0)", "Multicast UNKNOWN", "Unicast 10.0.0.5"]);
    }

    #[test]
    fn nothing_saved_defaults_to_the_primary_interface() {
        let choices = [choice("en1", "Ethernet"), choice("en0", "Wi-Fi")];
        let list = e131_destination_list(&choices, &[], &E131Destination::default(), Some("en0"));
        assert!(!list.missing_interface);
        assert_eq!(list.entries[list.selected.unwrap()].multicast.as_deref(), Some("en0"));
        assert_eq!(e131_destination_list(&choices, &[], &E131Destination::default(), None).selected, None);
    }

    #[test]
    fn a_saved_unicast_is_selected() {
        let saved = E131Destination { unicast: Some("10.0.0.5".to_string()), multicast: None };
        let list = e131_destination_list(&[choice("en0", "Wi-Fi")], &["10.0.0.5".to_string()], &saved, None);
        assert_eq!(list.entries[list.selected.unwrap()].unicast.as_deref(), Some("10.0.0.5"));
    }

    #[test]
    fn unicast_table_refuses_duplicates_and_addresses_in_use() {
        let mut known = Vec::new();
        e131_add_unicast(&mut known, "b").unwrap();
        e131_add_unicast(&mut known, "a").unwrap();
        assert_eq!(known, ["a", "b"]);
        assert_eq!(e131_add_unicast(&mut known, "a").unwrap_err(), "Error - Duplicate IP Address");
        let peers = [E131Peer { unicast: Some("a".to_string()), ..Default::default() }];
        assert_eq!(
            e131_remove_unicast(&mut known, "a", &peers).unwrap_err(),
            "This destination is in use by another instance of the plugin and cannot be removed."
        );
        e131_remove_unicast(&mut known, "b", &peers).unwrap();
        assert_eq!(known, ["a"]);
    }

    #[test]
    fn unicast_fields() {
        assert!(unicast_ip_is_valid("127.0.0.1"));
        assert!(!unicast_ip_is_valid("127.0.0"));
        assert!(unicast_name_is_valid("myComputer"));
        assert!(!unicast_name_is_valid("  "));
    }

    #[test]
    fn repeat_and_suppress_values() {
        for bad in ["-1", "100", "ab"] {
            assert_eq!(e131_validate_repeat(bad), None, "{bad}");
        }
        assert_eq!(e131_validate_repeat("0"), Some(0));
        assert_eq!(e131_validate_repeat("99"), Some(99));
        assert_eq!(e131_validate_suppress("10000"), Some(10000));
        assert_eq!(e131_validate_suppress("10001"), None);
    }

    #[test]
    fn the_draft_applies_upstreams_rules() {
        let mut d = e131_draft(&E131Config::default(), "Tree", 510);
        assert_eq!(d.title, "Streaming ACN (E1.31) Configuration - Tree");
        assert!(!d.manual);
        assert_eq!(d.sort(UniverseColumn::Size, true).as_deref(), Some("Sorting is not available while Vixen manages the start values."));
        assert_eq!(d.sort(UniverseColumn::Size, true), None, "once per dialog");
        e131_add_universe(&mut d.rows, d.manual).unwrap();
        d.set_cell(0, UniverseColumn::Size, "100").unwrap();
        assert_eq!(d.rows[1].start, 101, "a size edit moves the next start");
        assert_eq!(d.set_cell(0, UniverseColumn::Size, "600").unwrap_err(), "Size must be between 1 and 512 inclusive");
        d.set_manual(true);
        d.set_cell(1, UniverseColumn::Start, "7").unwrap();
        assert_eq!(d.rows[1].start, 7);
        d.set_manual(false);
        assert_eq!(d.rows[1].start, 101);
        d.repeat_text = "x".to_string();
        assert_eq!(d.options().repeat_count, 0);
        assert!(d.options().auto_populate);
    }
}
