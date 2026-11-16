//! Find Effects/Layers: `FindEffectForm`. F321 to F327.

use crate::sequence::{EffectEntry, EffectId, SequenceSession};
use vixen_core::TimeSpan;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FindMode {
    Effects,
    Layers,
}

/// One row of the list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FindRow {
    pub element: EffectId,
    /// `element.Row.RowLabel.Name`.
    pub row_name: String,
    /// `StartTime.ToString()`, F325.
    pub start_text: String,
    /// The other attribute: the layer in effect mode, the effect in layer mode.
    pub other: String,
    pub start: TimeSpan,
}

/// `TimeSpan.ToString()`: `hh:mm:ss.fffffff`, and no fraction at all when it's zero.
pub fn time_span_text(t: TimeSpan) -> String {
    let ticks = t.0.abs();
    let sign = if t.0 < 0 { "-" } else { "" };
    let total_seconds = ticks / 10_000_000;
    let fraction = ticks % 10_000_000;
    let (days, hours, minutes, seconds) = (total_seconds / 86_400, total_seconds / 3_600 % 24, total_seconds / 60 % 60, total_seconds % 60);
    let days = if days > 0 { format!("{days}.") } else { String::new() };
    if fraction == 0 {
        format!("{sign}{days}{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{sign}{days}{hours:02}:{minutes:02}:{seconds:02}.{fraction:07}")
    }
}

fn effect_name(entry: &EffectEntry) -> String {
    entry.descriptor.map_or_else(String::new, |d| d.name.to_string())
}

fn layer_name(session: &SequenceSession, entry: &EffectEntry) -> String {
    let index = session.document().layers.layer_index_for_effect(&entry.node.instance_id);
    session.document().layers.layers.get(index).map_or_else(String::new, |l| l.name.clone())
}

/// The second combo: unique effect or layer names in use, sorted. F323.
pub fn available_names(session: &SequenceSession, mode: FindMode) -> Vec<String> {
    let mut names: Vec<String> = session
        .effects()
        .map(|e| match mode {
            FindMode::Effects => effect_name(e),
            FindMode::Layers => layer_name(session, e),
        })
        .filter(|n| !n.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The list: one row per effect whose name or layer matches, by start. F325.
pub fn find_rows(session: &SequenceSession, mode: FindMode, search: &str) -> Vec<FindRow> {
    let mut rows: Vec<FindRow> = session
        .effects()
        .filter_map(|e| {
            let (name, layer) = (effect_name(e), layer_name(session, e));
            let (matched, other) = match mode {
                FindMode::Effects => (name == search, layer),
                FindMode::Layers => (layer == search, name),
            };
            matched.then(|| FindRow {
                element: e.id,
                row_name: e.node.targets.first().map_or_else(String::new, |t| t.1.clone()),
                start_text: time_span_text(e.node.start),
                other,
                start: e.node.start,
            })
        })
        .collect();
    rows.sort_by_key(|r| r.start);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_text_is_dotnet_s() {
        assert_eq!(time_span_text(TimeSpan::ZERO), "00:00:00");
        assert_eq!(time_span_text(TimeSpan::from_secs(61.0)), "00:01:01");
        assert_eq!(time_span_text(TimeSpan(12_500_000)), "00:00:01.2500000");
        assert_eq!(time_span_text(TimeSpan(36_000_000_000)), "01:00:00");
    }
}
