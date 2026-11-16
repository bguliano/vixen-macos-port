//! The arithmetic behind Add Effect, Add Multiple Effects, Distribute and Align To Marks, section H of the spec, as pure functions.
//! The commands that use them are on [`crate::sequencer::SequencerState`].

use vixen_core::{Mark, MarkCollection, TimeSpan};

const TWO_SECONDS: TimeSpan = TimeSpan(2 * 10_000_000);
const MIN_EFFECT: TimeSpan = TimeSpan(250 * 10_000);

/// `ExtensionMethods.Scale`: truncates to a tick.
fn scale(t: TimeSpan, by: f64) -> TimeSpan {
    TimeSpan((t.0 as f64 * by) as i64)
}

/// `GetDefaultEffectDuration(start, visibleDuration = true)`. F286.
pub fn default_effect_duration(visible_end: TimeSpan, start: TimeSpan) -> TimeSpan {
    let mut duration = TWO_SECONDS;
    if visible_end.total_secs() - start.total_secs() <= 2.0 {
        duration = scale(visible_end - start, 0.8);
    }
    if duration.total_ms() <= 250.0 {
        duration = MIN_EFFECT;
    }
    duration
}

/// `AddEffectInstance`'s adjustment: an effect already at the minimum that would still run past the end moves back. F285.
pub fn fit_new_effect(start: TimeSpan, duration: TimeSpan, length: TimeSpan) -> (TimeSpan, TimeSpan) {
    if duration <= MIN_EFFECT && start + duration > length {
        let moved = length - MIN_EFFECT;
        if moved < TimeSpan::ZERO {
            return (TimeSpan::ZERO, length);
        }
        return (moved, duration);
    }
    (start, duration)
}

/// `CalculatePossibleEffects`. The test is strict, so a set that fits exactly isn't counted (KEEP F386).
pub fn possible_effects(start: TimeSpan, end: TimeSpan, duration: TimeSpan, between: TimeSpan) -> i32 {
    let working = end - start;
    let uses = |i: i64| TimeSpan(duration.0.saturating_mul(i).saturating_add(between.0.saturating_mul(i - 1)));
    let mut i: i64 = 1;
    let mut used = uses(i);
    while used > TimeSpan::ZERO && used < working {
        i += 1;
        used = uses(i);
    }
    (i - 1) as i32
}

/// `CalculatePossibleEffectsByBeatMarks`, given how many marks are in range. F387.
pub fn possible_effects_on_marks(times: usize, fill: bool, skip_every_other: bool) -> i32 {
    let possible = if fill { times as i64 - 1 } else { times as i64 };
    let possible = possible.max(0);
    (if skip_every_other { (possible + 1) / 2 } else { possible }) as i32
}

/// The check on OK in `Form_AddMultipleEffects`. F392: false means effects would pass the end and the user is asked.
pub fn time_exists_for_addition(start: TimeSpan, end: TimeSpan, duration: TimeSpan, between: TimeSpan, count: i32) -> bool {
    let count = count as i64;
    start.0.saturating_add(duration.0.saturating_mul(count)).saturating_add(between.0.saturating_mul(count - 1)) <= end.0
}

/// The plain path of `AddMultipleEffects`. F291. Stops when `next + duration` passes the sequence's end.
pub fn plan_add_multiple(start: TimeSpan, duration: TimeSpan, between: TimeSpan, count: i32, length: TimeSpan) -> Vec<(TimeSpan, TimeSpan)> {
    let mut placed = Vec::new();
    let mut next = start;
    for _ in 0..count {
        if next + duration > length {
            break;
        }
        placed.push((next, duration));
        next = next + duration + between;
    }
    placed
}

/// The options of `AddEffectsToBeatMarks`. F292.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AddOnMarksOptions {
    pub count: i32,
    pub duration: TimeSpan,
    pub fill: bool,
    pub skip_every_other: bool,
    pub align_to_mark_duration: bool,
}

/// `AddEffectsToBeatMarks`: `marks` are `(start, duration)` from the checked collections with start at or past the dialog's start.
/// Sorted here. Walks by index, where upstream's `IndexOf` makes a long list quadratic.
pub fn plan_add_on_marks(marks: &[(TimeSpan, TimeSpan)], options: AddOnMarksOptions) -> Vec<(TimeSpan, TimeSpan)> {
    let mut times = marks.to_vec();
    // `Mark.CompareTo`: start, then end.
    times.sort_by_key(|(start, duration)| (*start, *start + *duration));
    let ten_ms = TimeSpan::from_secs(0.01);
    let mut placed = Vec::new();
    let mut skip = false;
    let mut duration = options.duration;
    for (i, (start, mark_duration)) in times.iter().enumerate() {
        if placed.len() as i32 >= options.count {
            break;
        }
        if !skip {
            if options.fill {
                let Some((next, _)) = times.get(i + 1) else { break };
                duration = *next - *start;
                if duration < ten_ms {
                    duration = ten_ms;
                }
            } else if options.align_to_mark_duration {
                duration = *mark_duration;
            }
            placed.push((*start, duration));
        }
        if options.skip_every_other {
            skip = !skip;
        }
    }
    placed
}

/// Why Distribute Equally didn't run.
#[derive(Clone, PartialEq, Debug)]
pub enum DistributeError {
    /// The message and title of the warning. F293.
    TooShort { message: String, title: &'static str },
}

/// `DistributeSelectedEffectsEqually`: new `(start, end)` for each effect in selection order, the walk starting from the last selected
/// when the first starts after the last. F293.
pub fn distribute_equally(selected: &[(TimeSpan, TimeSpan)]) -> Result<Vec<(TimeSpan, TimeSpan)>, DistributeError> {
    let (Some(first), Some(last)) = (selected.first(), selected.last()) else { return Ok(Vec::new()) };
    let end_of = |e: &(TimeSpan, TimeSpan)| e.0 + e.1;
    let (start, end, from_last) =
        if first.0 > last.0 { (last.0, end_of(first), true) } else { (first.0, end_of(last), false) };
    let each = (end - start).total_secs() / selected.len() as f64;
    if each < 0.050 {
        return Err(DistributeError::TooShort {
            message: format!(
                "Unable to complete request. The resulting duration would fall below 50 milliseconds.\nCalculated duration: {}",
                csharp_double(each)
            ),
            title: "Warning",
        });
    }
    let span = TimeSpan::from_secs(each);
    let mut result = vec![(TimeSpan::ZERO, TimeSpan::ZERO); selected.len()];
    let mut at = start;
    let order: Box<dyn Iterator<Item = usize>> =
        if from_last { Box::new((0..selected.len()).rev()) } else { Box::new(0..selected.len()) };
    for i in order {
        result[i] = (at, at + span);
        at += span;
    }
    Ok(result)
}

/// `double.ToString()`: the shortest text that round-trips, which is Rust's `{}` too.
fn csharp_double(v: f64) -> String {
    format!("{v}")
}

/// `RadioEqualDuration`, `RadioDoNotChangeDuration`, `RadioSpecifiedDuration`.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DurationMode {
    Equal,
    DoNotChange,
    Specified(TimeSpan),
}

/// `RadioStairStep`, `RadioEffectPlacementOverlap`, `RadioPlacementSpacedDuration`.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PlacementMode {
    StairStep,
    Overlap(TimeSpan),
    Spaced(TimeSpan),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DistributeParams {
    pub start: TimeSpan,
    pub end: TimeSpan,
    pub duration: DurationMode,
    pub placement: PlacementMode,
    pub start_with_first: bool,
}

/// The dialog's start and end for a selection: first start to last end, swapped only when start is past end. F294.
pub fn distribute_dialog_span(selected: &[(TimeSpan, TimeSpan)]) -> Option<(TimeSpan, TimeSpan)> {
    let (first, last) = (selected.first()?, selected.last()?);
    let (mut start, mut end) = (first.0, last.0 + last.1);
    if start > end {
        start = last.0;
        end = first.0 + first.1;
    }
    Some((start, end))
}

/// `DistributeSelectedEffects`, in double seconds as upstream. New `(start, end)` indexed like `selected`. F294.
pub fn distribute(selected: &[(TimeSpan, TimeSpan)], p: &DistributeParams) -> Vec<(TimeSpan, TimeSpan)> {
    let count = selected.len();
    let mut result = vec![(TimeSpan::ZERO, TimeSpan::ZERO); count];
    if count == 0 {
        return result;
    }
    let range_start = p.start.total_secs();
    let offset = (p.end - p.start).total_secs() / count as f64;
    let order: Vec<usize> = if p.start_with_first { (0..count).collect() } else { (0..count).rev().collect() };
    let mut previous_end: Option<f64> = None;
    for (place, &i) in order.iter().enumerate() {
        let mut start = range_start;
        if let Some(prev) = previous_end {
            start = prev;
            match p.placement {
                PlacementMode::Overlap(by) => start -= by.total_secs(),
                PlacementMode::Spaced(by) => start += by.total_secs(),
                PlacementMode::StairStep => {}
            }
        }
        if p.duration == DurationMode::DoNotChange && p.placement == PlacementMode::StairStep {
            start = range_start + offset * place as f64;
        }
        let end = match p.duration {
            DurationMode::Equal => start + offset,
            DurationMode::DoNotChange => start + selected[i].1.total_secs(),
            DurationMode::Specified(d) => start + d.total_secs(),
        };
        // Upstream reads the previous end back through `TimeSpan.FromSeconds`, which truncates to a tick.
        let (start_ts, end_ts) = (TimeSpan::from_secs(start), TimeSpan::from_secs(end));
        result[i] = (start_ts, end_ts);
        previous_end = Some(end_ts.total_secs());
    }
    result
}

/// `AlignEffectsToNearestMarks`' choice.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MarkAlign {
    Start,
    End,
    Both,
}

/// Mark times from every collection showing grid lines, sorted, for `FindNearestMark` without a scan per effect edge. `order` keeps
/// upstream's tie rule: the first found in collection order wins, and a mark's start before its end.
pub struct MarkIndex {
    entries: Vec<(TimeSpan, u32)>,
}

impl MarkIndex {
    pub fn new(collections: &[MarkCollection]) -> MarkIndex {
        let mut entries = Vec::new();
        let mut order = 0u32;
        for collection in collections.iter().filter(|c| c.show_grid_lines) {
            for mark in &collection.marks {
                entries.push((mark.start_time, order));
                order += 1;
                if collection.show_tail_lines {
                    entries.push((mark.end_time(), order));
                    order += 1;
                }
            }
        }
        entries.sort();
        MarkIndex { entries }
    }

    /// `FindNearestMark`: within the threshold, strictly. Returns zero when there's none.
    pub fn find_nearest(&self, t: TimeSpan, threshold: TimeSpan) -> TimeSpan {
        let low = self.entries.partition_point(|(time, _)| *time - threshold < t && !(t < *time + threshold) && *time < t);
        let mut best: Option<(i64, u32, TimeSpan)> = None;
        for (time, order) in &self.entries[low..] {
            if !(t < *time + threshold) {
                continue;
            }
            if !(*time - threshold < t) {
                break;
            }
            let distance = (time.0 - t.0).abs();
            if best.is_none_or(|(d, o, _)| (distance, *order) < (d, o)) {
                best = Some((distance, *order, *time));
            }
        }
        best.map_or(TimeSpan::ZERO, |(.., time)| time)
    }
}

/// `Convert.ToDouble(AlignTo_Threshold, InvariantCulture)`, as a time. Text that isn't a number means no alignment at all.
pub fn parse_threshold(text: &str) -> Option<TimeSpan> {
    text.trim().parse::<f64>().ok().map(TimeSpan::from_secs)
}

/// `AlignEffectsToNearestMarks`' loop: new `(start, end)` per effect, or `None` when it's skipped. An effect whose new start or end is
/// exactly zero is skipped, so a real start at 0 skips too (KEEP F295).
pub fn align_to_marks(kind: MarkAlign, effects: &[(TimeSpan, TimeSpan)], threshold: TimeSpan, index: &MarkIndex) -> Vec<Option<(TimeSpan, TimeSpan)>> {
    effects
        .iter()
        .map(|(start, duration)| {
            let mut new_start = *start;
            let mut new_end = *start + *duration;
            if matches!(kind, MarkAlign::Start | MarkAlign::Both) {
                new_start = index.find_nearest(new_start, threshold);
            }
            if matches!(kind, MarkAlign::End | MarkAlign::Both) {
                new_end = index.find_nearest(new_end, threshold);
            }
            (new_start != TimeSpan::ZERO && new_end != TimeSpan::ZERO).then_some((new_start, new_end))
        })
        .collect()
}

/// The result of `divideMarksEvenlyToolStripMenuItem_Click`'s checks. F82.
#[derive(Clone, PartialEq, Debug)]
pub struct EvenMarksPlan {
    pub name: String,
    pub marks: Vec<(TimeSpan, TimeSpan)>,
}

pub const EVEN_MARKS_TOO_SMALL: &str = "Individual mark length is smaller than allowed.\n\nEither increase the difference in time between Start and End times or decrease the number of divisions.";
pub const EVEN_MARKS_TOO_SMALL_TITLE: &str = "Mark Length Error";

/// Create Evenly Divided Marks. `min_ms` is the time of 12 points at the current zoom. `existing` holds the names left after removing
/// an exact match, which the caller finds with [`matching_even_collection`]. The error is F82's message: the dialog then reopens.
pub fn plan_even_marks(start: TimeSpan, end: TimeSpan, divisions: i32, min_ms: f64, existing: &[&str]) -> Result<EvenMarksPlan, &'static str> {
    let divisions = divisions.max(1);
    let total = (end - start).total_ms();
    let each_ms = total / divisions as f64;
    if each_ms < min_ms {
        return Err(EVEN_MARKS_TOO_SMALL);
    }
    let base = format!("{divisions} Divisions");
    let mut name = base.clone();
    let mut next = 1i64;
    for other in existing {
        if let Some(rest) = other.strip_prefix(&base) {
            // `Regex(@"\d+")`: the first run of digits, or 1 when there's none.
            let digits: String = rest.chars().skip_while(|c| !c.is_ascii_digit()).take_while(char::is_ascii_digit).collect();
            let number = digits.parse::<i64>().unwrap_or(1);
            if number >= next {
                next = number + 1;
            }
        }
    }
    if next > 1 {
        name = format!("{base} #{next}");
    }
    let each = TimeSpan::from_ms(each_ms);
    let mark_duration = TimeSpan::from_ms(each_ms * 0.8);
    let marks = (0..=divisions).map(|i| (start + TimeSpan(each.0 * i as i64), mark_duration)).collect();
    Ok(EvenMarksPlan { name, marks })
}

/// The first collection that exactly matches what would be made: divisions + 1 marks, first at start, last at end. It gets replaced, with
/// no undo (KEEP F82).
pub fn matching_even_collection(collections: &[MarkCollection], start: TimeSpan, end: TimeSpan, divisions: i32) -> Option<usize> {
    collections.iter().position(|c| {
        c.marks.len() as i64 == divisions as i64 + 1
            && c.marks.first().is_some_and(|m: &Mark| m.start_time == start)
            && c.marks.last().is_some_and(|m: &Mark| m.start_time == end)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(secs: f64) -> TimeSpan {
        TimeSpan::from_secs(secs)
    }

    fn ms(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v)
    }

    #[test]
    fn default_duration() {
        assert_eq!(default_effect_duration(s(10.0), s(0.0)), s(2.0));
        assert_eq!(default_effect_duration(s(10.0), s(9.0)), ms(800));
        // 80 ms is floored.
        assert_eq!(default_effect_duration(s(10.0), s(9.9)), ms(250));
        assert_eq!(default_effect_duration(s(10.0), s(8.0)), s(1.6));
    }

    #[test]
    fn fit() {
        assert_eq!(fit_new_effect(s(59.9), ms(250), s(60.0)), (s(59.75), ms(250)));
        assert_eq!(fit_new_effect(s(0.05), ms(250), ms(100)), (s(0.0), ms(100)));
        assert_eq!(fit_new_effect(s(59.9), ms(300), s(60.0)), (s(59.9), ms(300)));
    }

    #[test]
    fn possible_count_is_strict() {
        let two = s(2.0);
        assert_eq!(possible_effects(s(0.0), s(10.0), two, two), 2);
        assert_eq!(possible_effects(s(0.0), s(10.001), two, two), 3);
        assert_eq!(possible_effects(s(0.0), s(14.001), two, two), 4);
        assert_eq!(possible_effects(s(0.0), s(10.0), s(0.0), two), 0);
        assert_eq!(possible_effects(s(0.0), s(1.0), two, two), 0);
        // The dialog opens here on a one-minute sequence.
        assert_eq!(possible_effects(s(0.0), s(60.0), two, two), 15);
    }

    #[test]
    fn possible_on_marks() {
        assert_eq!(possible_effects_on_marks(5, false, false), 5);
        assert_eq!(possible_effects_on_marks(5, true, false), 4);
        assert_eq!(possible_effects_on_marks(5, false, true), 3);
        assert_eq!(possible_effects_on_marks(5, true, true), 2);
        assert_eq!(possible_effects_on_marks(0, true, false), 0);
    }

    #[test]
    fn plain_plan_stops_at_the_end() {
        let plan = plan_add_multiple(s(0.0), s(2.0), s(2.0), 10, s(7.0));
        assert_eq!(plan, vec![(s(0.0), s(2.0)), (s(4.0), s(2.0))]);
        assert!(plan_add_multiple(s(0.0), s(2.0), s(2.0), 0, s(7.0)).is_empty());
    }

    #[test]
    fn overrun_check() {
        assert!(time_exists_for_addition(s(0.0), s(10.0), s(2.0), s(2.0), 3));
        assert!(!time_exists_for_addition(s(0.0), s(9.0), s(2.0), s(2.0), 3));
    }

    fn opts(count: i32) -> AddOnMarksOptions {
        AddOnMarksOptions { count, duration: s(1.0), fill: false, skip_every_other: false, align_to_mark_duration: false }
    }

    #[test]
    fn marks_plan() {
        let marks = [(s(3.0), ms(100)), (s(1.0), ms(200)), (s(2.0), ms(300)), (s(2.005), ms(400))];
        assert_eq!(plan_add_on_marks(&marks[..3], opts(9)), vec![(s(1.0), s(1.0)), (s(2.0), s(1.0)), (s(3.0), s(1.0))]);
        // Count caps it.
        assert_eq!(plan_add_on_marks(&marks[..3], opts(2)).len(), 2);
        // Fill: up to the next mark, floored at 10 ms, nothing on the last mark.
        let fill = plan_add_on_marks(&marks, AddOnMarksOptions { fill: true, ..opts(9) });
        assert_eq!(fill, vec![(s(1.0), s(1.0)), (s(2.0), s(0.01)), (s(2.005), s(3.0) - s(2.005))]);
        // Skip takes marks 1, 3, 5.
        let skip = plan_add_on_marks(&marks, AddOnMarksOptions { skip_every_other: true, ..opts(9) });
        assert_eq!(skip.iter().map(|p| p.0).collect::<Vec<_>>(), [s(1.0), s(2.005)]);
        // Align uses each mark's duration.
        let align = plan_add_on_marks(&marks[..3], AddOnMarksOptions { align_to_mark_duration: true, ..opts(9) });
        assert_eq!(align, vec![(s(1.0), ms(200)), (s(2.0), ms(300)), (s(3.0), ms(100))]);
    }

    #[test]
    fn distribute_equally_cases() {
        let effects = [(s(0.0), s(1.0)), (s(1.0), s(1.0)), (s(3.0), s(1.0)), (s(6.0), s(2.0))];
        assert_eq!(
            distribute_equally(&effects).unwrap(),
            vec![(s(0.0), s(2.0)), (s(2.0), s(4.0)), (s(4.0), s(6.0)), (s(6.0), s(8.0))]
        );
        let reversed = [effects[3], effects[2], effects[1], effects[0]];
        assert_eq!(
            distribute_equally(&reversed).unwrap(),
            vec![(s(6.0), s(8.0)), (s(4.0), s(6.0)), (s(2.0), s(4.0)), (s(0.0), s(2.0))]
        );
        // Below 50 ms per effect fails; exactly 50 passes.
        let short = [(s(0.0), ms(10)), (ms(20), ms(10)), (ms(50), ms(10)), (ms(90), ms(10))];
        let Err(DistributeError::TooShort { message, title }) = distribute_equally(&short) else { panic!("should fail") };
        assert!(message.ends_with("Calculated duration: 0.025"));
        assert_eq!(title, "Warning");
        let exact = [(s(0.0), ms(10)), (ms(190), ms(10))];
        assert!(distribute_equally(&exact).is_ok());
        // One effect stays where it is.
        assert_eq!(distribute_equally(&[(s(1.0), s(1.0))]).unwrap(), vec![(s(1.0), s(2.0))]);
        assert!(distribute_equally(&[]).unwrap().is_empty());
    }

    fn params(duration: DurationMode, placement: PlacementMode, first: bool) -> DistributeParams {
        DistributeParams { start: s(0.0), end: s(8.0), duration, placement, start_with_first: first }
    }

    #[test]
    fn distribute_dialog_combinations() {
        // Four effects, durations 1, 2, 3, 4. Offset is 2 s.
        let e = [(s(0.0), s(1.0)), (s(10.0), s(2.0)), (s(20.0), s(3.0)), (s(30.0), s(4.0))];
        let (o, sp, d) = (PlacementMode::Overlap(s(0.5)), PlacementMode::Spaced(s(0.5)), PlacementMode::StairStep);
        let times = |dm, pm, first| distribute(&e, &params(dm, pm, first));

        assert_eq!(
            times(DurationMode::Equal, d, true),
            vec![(s(0.0), s(2.0)), (s(2.0), s(4.0)), (s(4.0), s(6.0)), (s(6.0), s(8.0))]
        );
        assert_eq!(
            times(DurationMode::Equal, o, true),
            vec![(s(0.0), s(2.0)), (s(1.5), s(3.5)), (s(3.0), s(5.0)), (s(4.5), s(6.5))]
        );
        assert_eq!(
            times(DurationMode::Equal, sp, true),
            vec![(s(0.0), s(2.0)), (s(2.5), s(4.5)), (s(5.0), s(7.0)), (s(7.5), s(9.5))]
        );
        // Do Not Change plus Stair Step: start + offset * i.
        assert_eq!(
            times(DurationMode::DoNotChange, d, true),
            vec![(s(0.0), s(1.0)), (s(2.0), s(4.0)), (s(4.0), s(7.0)), (s(6.0), s(10.0))]
        );
        assert_eq!(
            times(DurationMode::DoNotChange, o, true),
            vec![(s(0.0), s(1.0)), (s(0.5), s(2.5)), (s(2.0), s(5.0)), (s(4.5), s(8.5))]
        );
        assert_eq!(
            times(DurationMode::DoNotChange, sp, true),
            vec![(s(0.0), s(1.0)), (s(1.5), s(3.5)), (s(4.0), s(7.0)), (s(7.5), s(11.5))]
        );
        let one = DurationMode::Specified(s(1.0));
        assert_eq!(times(one, d, true), vec![(s(0.0), s(1.0)), (s(1.0), s(2.0)), (s(2.0), s(3.0)), (s(3.0), s(4.0))]);
        assert_eq!(times(one, o, true), vec![(s(0.0), s(1.0)), (s(0.5), s(1.5)), (s(1.0), s(2.0)), (s(1.5), s(2.5))]);
        // Specified with Spaced adds the spacing to the previous end.
        assert_eq!(times(one, sp, true), vec![(s(0.0), s(1.0)), (s(1.5), s(2.5)), (s(3.0), s(4.0)), (s(4.5), s(5.5))]);

        // Starting with the last effect walks backwards, and place count drives the stair step.
        assert_eq!(
            times(DurationMode::DoNotChange, d, false),
            vec![(s(6.0), s(7.0)), (s(4.0), s(6.0)), (s(2.0), s(5.0)), (s(0.0), s(4.0))]
        );
        assert_eq!(
            times(DurationMode::Equal, d, false),
            vec![(s(6.0), s(8.0)), (s(4.0), s(6.0)), (s(2.0), s(4.0)), (s(0.0), s(2.0))]
        );
    }

    #[test]
    fn dialog_span_swaps_only_when_start_passes_end() {
        let e = [(s(10.0), s(1.0)), (s(2.0), s(1.0))];
        assert_eq!(distribute_dialog_span(&e), Some((s(2.0), s(11.0))));
        let f = [(s(1.0), s(1.0)), (s(2.0), s(1.0))];
        assert_eq!(distribute_dialog_span(&f), Some((s(1.0), s(3.0))));
        assert_eq!(distribute_dialog_span(&[]), None);
    }

    fn collection(times: &[f64], grid: bool, tails: bool) -> MarkCollection {
        let mut c = MarkCollection::default();
        c.show_grid_lines = grid;
        c.show_tail_lines = tails;
        c.marks = times.iter().map(|t| Mark::new(s(*t))).collect();
        c
    }

    #[test]
    fn nearest_mark_rules() {
        let threshold = s(0.8);
        // Strict: exactly the threshold away isn't matched.
        let index = MarkIndex::new(&[collection(&[10.0], true, false)]);
        assert_eq!(index.find_nearest(s(10.8), threshold), TimeSpan::ZERO);
        assert_eq!(index.find_nearest(s(10.5), threshold), s(10.0));
        assert_eq!(index.find_nearest(s(9.5), threshold), s(10.0));
        // A collection not showing grid lines is ignored.
        let index = MarkIndex::new(&[collection(&[10.0], false, false)]);
        assert_eq!(index.find_nearest(s(10.0), threshold), TimeSpan::ZERO);
        // The tail counts only with tail lines. Marks are 450 ms long.
        let index = MarkIndex::new(&[collection(&[10.0], true, false)]);
        assert_eq!(index.find_nearest(s(10.45), s(0.5)), s(10.0));
        let index = MarkIndex::new(&[collection(&[10.0], true, true)]);
        assert_eq!(index.find_nearest(s(10.45), s(0.5)), s(10.45));
        // Two equal distances: the first found wins, by collection order and not by time.
        let index = MarkIndex::new(&[collection(&[11.0], true, false), collection(&[9.0], true, false)]);
        assert_eq!(index.find_nearest(s(10.0), s(1.5)), s(11.0));
        // Closest wins.
        let index = MarkIndex::new(&[collection(&[9.0, 10.4], true, false)]);
        assert_eq!(index.find_nearest(s(10.0), s(1.5)), s(10.4));
    }

    #[test]
    fn a_scan_and_the_index_agree() {
        let c = [collection(&[1.0, 2.0, 2.5, 7.0, 9.0], true, true), collection(&[2.2, 3.3, 8.9], true, false)];
        let index = MarkIndex::new(&c);
        let threshold = s(0.8);
        let scan = |t: TimeSpan| {
            let mut result = TimeSpan::ZERO;
            let mut shortest = i64::MAX;
            for mc in c.iter().filter(|m| m.show_grid_lines) {
                for m in &mc.marks {
                    if m.start_time - threshold < t && t < m.start_time + threshold {
                        let d = (m.start_time.0 - t.0).abs();
                        if d < shortest {
                            shortest = d;
                            result = m.start_time;
                        }
                    }
                    if mc.show_tail_lines && m.end_time() - threshold < t && t < m.end_time() + threshold {
                        let d = (m.end_time().0 - t.0).abs();
                        if d < shortest {
                            shortest = d;
                            result = m.end_time();
                        }
                    }
                }
            }
            result
        };
        for i in 0..1200 {
            let t = TimeSpan::from_ms_i(i * 10);
            assert_eq!(index.find_nearest(t, threshold), scan(t), "at {i}");
        }
    }

    #[test]
    fn align_skips_zero_results() {
        let index = MarkIndex::new(&[collection(&[10.0], true, false)]);
        let t = s(0.8);
        // No mark near: skipped.
        assert_eq!(align_to_marks(MarkAlign::Start, &[(s(5.0), s(1.0))], t, &index), vec![None]);
        // Moves the one edge.
        assert_eq!(align_to_marks(MarkAlign::Start, &[(s(9.5), s(1.0))], t, &index), vec![Some((s(10.0), s(10.5)))]);
        // A real start at 0 skips even with End mode (KEEP).
        assert_eq!(align_to_marks(MarkAlign::End, &[(s(0.0), s(10.1))], t, &index), vec![None]);
        // Both needs both.
        assert_eq!(align_to_marks(MarkAlign::Both, &[(s(9.5), s(30.0))], t, &index), vec![None]);
        // A found mark at time 0 counts as no match.
        let zero = MarkIndex::new(&[collection(&[0.0], true, false)]);
        assert_eq!(align_to_marks(MarkAlign::Start, &[(s(0.2), s(1.0))], t, &zero), vec![None]);
    }

    #[test]
    fn even_marks() {
        let plan = plan_even_marks(s(0.0), s(8.0), 4, 1.0, &[]).unwrap();
        assert_eq!(plan.name, "4 Divisions");
        assert_eq!(plan.marks.iter().map(|m| m.0).collect::<Vec<_>>(), [s(0.0), s(2.0), s(4.0), s(6.0), s(8.0)]);
        assert_eq!(plan.marks[0].1, s(1.6));
        assert_eq!(plan_even_marks(s(0.0), s(8.0), 4, 1.0, &["4 Divisions", "4 Divisions #2"]).unwrap().name, "4 Divisions #3");
        assert_eq!(plan_even_marks(s(0.0), s(8.0), 4, 1.0, &["4 Divisions"]).unwrap().name, "4 Divisions #2");
        assert_eq!(plan_even_marks(s(0.0), s(1.0), 1000, 12.0, &[]), Err(EVEN_MARKS_TOO_SMALL));
    }

    #[test]
    fn exact_match_is_found() {
        let c = [collection(&[0.0, 4.0, 8.0], true, false), collection(&[0.0, 2.0, 4.0, 6.0, 8.0], true, false)];
        assert_eq!(matching_even_collection(&c, s(0.0), s(8.0), 4), Some(1));
        assert_eq!(matching_even_collection(&c, s(0.0), s(8.0), 3), None);
    }
}
