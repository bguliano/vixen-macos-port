//! Port of Vixen.Core/Marks and VixenModules.App.Marks.

use crate::time::TimeSpan;
use crate::Color;
use uuid::Uuid;

#[derive(Clone, PartialEq, Debug)]
pub struct Mark {
    pub id: Uuid,
    pub start_time: TimeSpan,
    pub duration: TimeSpan,
    pub text: String,
}

impl Mark {
    /// `new Mark(TimeSpan)`.
    pub fn new(start_time: TimeSpan) -> Mark {
        Mark { id: Uuid::new_v4(), start_time, duration: TimeSpan::from_ms_i(450), text: String::new() }
    }

    pub fn end_time(&self) -> TimeSpan {
        self.start_time + self.duration
    }

    /// The `Duration` and `StartTime` setters, which deserialization runs: a duration under 25 ms becomes 25 ms. Both being zero
    /// leaves the setters nothing to do, so that mark stays at zero.
    pub fn apply_minimum_duration(&mut self) {
        let minimum = TimeSpan::from_ms_i(25);
        if self.duration < minimum && !(self.duration == TimeSpan::ZERO && self.start_time == TimeSpan::ZERO) {
            self.duration = minimum;
        }
    }
}

/// `MarkCollectionType`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum MarkCollectionType {
    #[default]
    Generic,
    Phrase,
    Word,
    Phoneme,
    State,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MarkDecorator {
    pub color: Color,
    pub is_bold: bool,
    pub is_solid_line: bool,
    pub compact_mode: bool,
}

impl Default for MarkDecorator {
    fn default() -> Self {
        MarkDecorator { color: Color::WHITE, is_bold: false, is_solid_line: false, compact_mode: false }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct MarkCollection {
    pub id: Uuid,
    pub name: String,
    pub level: i32,
    pub show_grid_lines: bool,
    pub show_marks: bool,
    pub show_tail_lines: bool,
    pub is_default: bool,
    pub is_locked: bool,
    pub collection_type: MarkCollectionType,
    pub decorator: MarkDecorator,
    pub marks: Vec<Mark>,
    pub linked_mark_collection_id: Option<Uuid>,
}

impl Default for MarkCollection {
    fn default() -> Self {
        MarkCollection {
            id: Uuid::new_v4(),
            name: "Mark Collection".to_string(),
            level: 1,
            show_grid_lines: true,
            show_marks: false,
            show_tail_lines: false,
            is_default: false,
            is_locked: false,
            collection_type: MarkCollectionType::Generic,
            decorator: MarkDecorator::default(),
            marks: Vec::new(),
            linked_mark_collection_id: None,
        }
    }
}

impl MarkCollection {
    /// `Mark.CompareTo`: start time, then end time for equal starts.
    pub fn sort_marks(&mut self) {
        self.marks.sort_by_key(|m| (m.start_time, m.end_time()));
    }

    pub fn add_mark(&mut self, mark: Mark) {
        let key = (mark.start_time, mark.end_time());
        let at = self.marks.partition_point(|m| (m.start_time, m.end_time()) <= key);
        self.marks.insert(at, mark);
    }

    /// Marks whose start falls inside `[start, end]`.
    pub fn marks_in_range(&self, start: TimeSpan, end: TimeSpan) -> impl Iterator<Item = &Mark> {
        self.marks.iter().filter(move |m| m.start_time >= start && m.start_time <= end)
    }

    /// `MarksInclusiveOfTime`: marks with some part inside `[start, end]`, so a mark that began earlier but is still
    /// running counts. Stops at the first mark that starts after `end`, as the C# does.
    pub fn marks_inclusive_of_time(&self, start: TimeSpan, end: TimeSpan) -> Vec<&Mark> {
        let mut out = Vec::new();
        for mark in &self.marks {
            if mark.end_time() < start {
                continue;
            }
            if mark.start_time > end {
                break;
            }
            out.push(mark);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusive_marks_count_one_still_running_at_the_start() {
        let mut c = MarkCollection::default();
        for (s, d) in [(0, 50), (80, 100), (200, 50), (400, 50)] {
            let mut m = Mark::new(TimeSpan::from_ms_i(s));
            m.duration = TimeSpan::from_ms_i(d);
            c.add_mark(m);
        }
        let got: Vec<f64> = c
            .marks_inclusive_of_time(TimeSpan::from_ms_i(100), TimeSpan::from_ms_i(300))
            .iter()
            .map(|m| m.start_time.total_ms())
            .collect();
        assert_eq!(got, vec![80.0, 200.0]);
    }

    #[test]
    fn add_keeps_marks_sorted() {
        let mut c = MarkCollection::default();
        c.add_mark(Mark::new(TimeSpan::from_ms_i(200)));
        c.add_mark(Mark::new(TimeSpan::from_ms_i(100)));
        c.add_mark(Mark::new(TimeSpan::from_ms_i(150)));
        let times: Vec<_> = c.marks.iter().map(|m| m.start_time.total_ms()).collect();
        assert_eq!(times, vec![100.0, 150.0, 200.0]);
    }

    #[test]
    fn equal_starts_are_ordered_by_end_for_insert_and_resort() {
        let mut c = MarkCollection::default();
        for duration in [300, 100, 200] {
            let mut mark = Mark::new(TimeSpan::from_ms_i(100));
            mark.duration = TimeSpan::from_ms_i(duration);
            c.add_mark(mark);
        }
        assert_eq!(c.marks.iter().map(|m| m.duration.total_ms()).collect::<Vec<_>>(), [100.0, 200.0, 300.0]);
        c.marks[0].duration = TimeSpan::from_ms_i(400);
        c.sort_marks();
        assert_eq!(c.marks.iter().map(|m| m.duration.total_ms()).collect::<Vec<_>>(), [200.0, 300.0, 400.0]);
    }
}

#[cfg(test)]
mod minimum_duration_tests {
    use super::*;

    fn loaded(start_ms: i64, duration_ms: i64) -> TimeSpan {
        let mut mark = Mark::new(TimeSpan::from_ms_i(start_ms));
        mark.duration = TimeSpan::from_ms_i(duration_ms);
        mark.apply_minimum_duration();
        mark.duration
    }

    #[test]
    fn short_marks_become_25_ms_unless_start_and_duration_are_both_zero() {
        assert_eq!(loaded(100, 10), TimeSpan::from_ms_i(25));
        assert_eq!(loaded(100, 0), TimeSpan::from_ms_i(25));
        assert_eq!(loaded(0, 10), TimeSpan::from_ms_i(25));
        assert_eq!(loaded(0, 0), TimeSpan::ZERO);
        assert_eq!(loaded(100, 25), TimeSpan::from_ms_i(25));
        assert_eq!(loaded(100, 450), TimeSpan::from_ms_i(450));
    }
}
