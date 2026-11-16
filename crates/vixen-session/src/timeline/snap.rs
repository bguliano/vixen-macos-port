//! Snapping: the points effects can snap to, their windows, and the search. F68 to F76.

use super::{ElementTimes, ResizeZone, RowId};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};
use uuid::Uuid;
use vixen_core::{Color, MarkCollection, TimeSpan};

/// `SnapPriorityForElements`.
pub const ELEMENT_SNAP_LEVEL: i32 = 5;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SnapEdge {
    Start,
    End,
}

/// F70. Strict at both ends.
#[inline]
pub fn snap_window(time: TimeSpan, tpp: TimeSpan, level: i32, strength: i32) -> (TimeSpan, TimeSpan) {
    let half = TimeSpan(tpp.ticks() * level as i64 * strength as i64);
    (time - half, time + half)
}

/// One mark's boundary in the static map. Identity is the mark and the edge, never the time (F75).
#[derive(Clone, PartialEq, Debug)]
pub struct MarkSnapDetail {
    pub mark: Uuid,
    pub edge: SnapEdge,
    pub level: i32,
    pub color: Color,
    pub bold: bool,
    pub solid: bool,
}

/// `StaticSnapPoints`: a time-sorted map to one detail per mark and per edge.
#[derive(Clone, Debug)]
pub struct MarkSnapIndex {
    points: BTreeMap<i64, Vec<MarkSnapDetail>>,
    /// The times each registered mark's details are filed under: `MarkSnapPointRegistration`.
    registrations: HashMap<Uuid, (i64, Option<i64>)>,
    /// Mark windows are recalculated on zoom: they read these instead of being stored.
    pub tpp: TimeSpan,
    pub strength: i32,
    max_level: i32,
}

impl Default for MarkSnapIndex {
    fn default() -> Self {
        MarkSnapIndex { points: BTreeMap::new(), registrations: HashMap::new(), tpp: super::view::DEFAULT_TIME_PER_PIXEL, strength: 2, max_level: 0 }
    }
}

impl MarkSnapIndex {
    /// `CreateSnapPointsFromMarks`.
    pub fn rebuild(&mut self, collections: &[MarkCollection], strength: i32) {
        self.points.clear();
        self.registrations.clear();
        self.strength = strength;
        self.max_level = 0;
        for collection in collections {
            if !collection.show_grid_lines {
                continue;
            }
            for mark in &collection.marks {
                self.register(collection, mark);
            }
        }
    }

    fn register(&mut self, collection: &MarkCollection, mark: &vixen_core::marks::Mark) {
        let detail = |edge| MarkSnapDetail {
            mark: mark.id,
            edge,
            level: collection.level,
            color: collection.decorator.color,
            bold: collection.decorator.is_bold,
            solid: collection.decorator.is_solid_line,
        };
        let start = mark.start_time.ticks();
        self.points.entry(start).or_default().push(detail(SnapEdge::Start));
        let end = collection.show_tail_lines.then(|| mark.end_time().ticks());
        if let Some(end) = end {
            self.points.entry(end).or_default().push(detail(SnapEdge::End));
        }
        self.registrations.insert(mark.id, (start, end));
        self.max_level = self.max_level.max(collection.level);
    }

    fn remove_registration(&mut self, mark: Uuid) -> bool {
        let Some((start, end)) = self.registrations.remove(&mark) else { return false };
        self.remove_detail(start, mark, SnapEdge::Start);
        if let Some(end) = end {
            self.remove_detail(end, mark, SnapEdge::End);
        }
        true
    }

    fn remove_detail(&mut self, time: i64, mark: Uuid, edge: SnapEdge) {
        let Some(list) = self.points.get_mut(&time) else { return };
        if let Some(i) = list.iter().position(|d| d.mark == mark && d.edge == edge) {
            list.remove(i);
        }
        if list.is_empty() {
            self.points.remove(&time);
        }
    }

    /// `TimeLineGlobalEventManager_MarksMoving`: replaces exactly these marks' details, each once however often it is listed.
    /// Returns whether anything changed.
    pub fn update_marks(&mut self, ids: &[Uuid], collections: &[MarkCollection]) -> bool {
        let wanted: HashSet<Uuid> = ids.iter().copied().collect();
        let mut changed = false;
        for id in &wanted {
            changed |= self.remove_registration(*id);
        }
        let mut remaining = wanted.len();
        'outer: for collection in collections {
            if !collection.show_grid_lines {
                continue;
            }
            for mark in &collection.marks {
                if wanted.contains(&mark.id) {
                    self.register(collection, mark);
                    changed = true;
                    remaining -= 1;
                    if remaining == 0 {
                        break 'outer;
                    }
                }
            }
        }
        changed
    }

    /// Details filed at one time, as `ContainsStaticSnapPoint` tests need.
    pub fn details_at(&self, time: TimeSpan) -> &[MarkSnapDetail] {
        self.points.get(&time.ticks()).map_or(&[], Vec::as_slice)
    }

    pub fn is_registered(&self, mark: Uuid) -> bool {
        self.registrations.contains_key(&mark)
    }

    pub fn registration_count(&self) -> usize {
        self.registrations.len()
    }

    pub fn registration(&self, mark: Uuid) -> Option<(TimeSpan, Option<TimeSpan>)> {
        self.registrations.get(&mark).map(|(s, e)| (TimeSpan(*s), e.map(TimeSpan)))
    }

    pub fn has_points_at(&self, time: TimeSpan) -> bool {
        self.points.contains_key(&time.ticks())
    }

    pub(crate) fn range(&self, from: i64, to: i64) -> impl Iterator<Item = (&i64, &Vec<MarkSnapDetail>)> {
        self.points.range(from..=to)
    }
}

/// F76: one drawn line per time.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MarkLine {
    pub time: TimeSpan,
    pub x: f64,
    pub color: Color,
    pub width: i32,
    /// `[dash, dash]`, or 0 for a solid line.
    pub dash: i32,
}

/// `_drawSnapPoints`: visible times, skipping those in the alignment list, which draw a guide instead.
pub fn mark_lines(index: &MarkSnapIndex, visible_start: TimeSpan, visible_end: TimeSpan, guides: &[TimeSpan], tpp: TimeSpan) -> Vec<MarkLine> {
    let mut out = Vec::new();
    for (time, details) in index.points.range(visible_start.ticks()..visible_end.ticks()) {
        let time = TimeSpan(*time);
        if guides.contains(&time) {
            continue;
        }
        let Some(mut chosen) = details.first() else { continue };
        for d in details {
            if d.level > chosen.level && d.color != Color::EMPTY {
                chosen = d;
            }
        }
        out.push(MarkLine {
            time,
            x: super::view::px(time, tpp),
            color: chosen.color,
            width: if chosen.bold { 3 } else { 1 },
            // A level of 0 makes an invalid dash pattern upstream. GUARD.
            dash: if chosen.solid { 0 } else { chosen.level.max(1) },
        });
    }
    out
}

/// One edge of an unselected effect, in the snapshot taken at drag start.
#[derive(Copy, Clone, Debug)]
struct EdgePoint {
    time: i64,
    edge: SnapEdge,
}

/// `CurrentDragSnapPoints`, without the static points, which `find_snap` reads live from the index. Filed by row: an edge only applies to an effect
/// paired with its own row (F72), so a search never has to step over the other rows' edges.
#[derive(Clone, Debug, Default)]
pub struct DragSnap {
    /// Each row's edges, sorted by time. Stable, so the order they came in holds at equal times, as in the original's lists.
    edges: HashMap<RowId, Vec<EdgePoint>, BuildHasherDefault<RowHasher>>,
    /// KEEP: effect-edge windows keep the zoom at drag start if the user zooms mid-drag.
    edge_half_window: i64,
}

/// Row ids are small integers.
#[derive(Default)]
pub struct RowHasher(u64);

impl Hasher for RowHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ *b as u64).wrapping_mul(0x100000001b3);
        }
    }

    #[inline]
    fn write_u32(&mut self, v: u32) {
        self.0 = (v as u64).wrapping_mul(0x9E3779B97F4A7C15);
    }
}

impl DragSnap {
    /// `edges` is `(time, row, edge)` for every unselected effect in every visible row, in that order.
    pub fn new(edges: Vec<(TimeSpan, RowId, SnapEdge)>, tpp: TimeSpan, strength: i32) -> DragSnap {
        let mut by_row: HashMap<RowId, Vec<EdgePoint>, BuildHasherDefault<RowHasher>> = HashMap::default();
        for (t, row, edge) in edges {
            by_row.entry(row).or_default().push(EdgePoint { time: t.ticks(), edge });
        }
        for list in by_row.values_mut() {
            list.sort_by_key(|e| e.time);
        }
        DragSnap { edges: by_row, edge_half_window: tpp.ticks() * ELEMENT_SNAP_LEVEL as i64 * strength as i64 }
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

struct Search<'a> {
    original: ElementTimes,
    row: RowId,
    offset: TimeSpan,
    zone: ResizeZone,
    best_level: Option<i32>,
    snapped: TimeSpan,
    marks: &'a MarkSnapIndex,
}

impl Search<'_> {
    fn consider(&mut self, time: i64, level: i32, half_window: i64, row: Option<RowId>, edge: Option<SnapEdge>) {
        if self.best_level.is_some_and(|best| level <= best) {
            return;
        }
        if row.is_some_and(|r| r != self.row) {
            return;
        }
        let time = TimeSpan(time);
        let orig = self.original;
        // F74: an effect can't snap itself shut.
        let opposing = match (self.zone, edge) {
            (ResizeZone::Front, Some(SnapEdge::Start)) => time == orig.end(),
            (ResizeZone::Back, Some(SnapEdge::End)) => time == orig.start,
            _ => false,
        };
        if opposing {
            return;
        }
        let half = TimeSpan(half_window);
        let (snap_start, snap_end) = (time - half, time + half);
        let moved_start = orig.start + self.offset;
        let moved_end = orig.end() + self.offset;
        let mut start_in = moved_start > snap_start && moved_start < snap_end;
        let mut end_in = moved_end > snap_start && moved_end < snap_end;
        match self.zone {
            ResizeZone::Front => end_in = false,
            ResizeZone::Back => start_in = false,
            ResizeZone::None => {}
        }
        if !start_in && !end_in {
            return;
        }
        self.snapped = if start_in && end_in {
            // KEEP Q13: this grouping, so a rightward drag snaps the end edge and a leftward drag the start edge.
            if (time - orig.start + self.offset) < (orig.end() + self.offset - time) {
                time - orig.start
            } else {
                time - orig.end()
            }
        } else if start_in {
            time - orig.start
        } else {
            time - orig.end()
        };
        self.best_level = Some(level);
    }
}

/// `FindSnapTimeForElements`, for snapping on: the caller returns the offset as it is when snapping is off. F71 and F74.
///
/// A range query around the moved edges visits what a full scan in time order would find, in the same order: nothing outside
/// the widest window can match.
pub fn find_snap(snap: &DragSnap, marks: &MarkSnapIndex, candidates: &[(ElementTimes, RowId)], offset: TimeSpan, zone: ResizeZone) -> TimeSpan {
    if offset == TimeSpan::ZERO {
        return offset;
    }
    let mark_unit = marks.tpp.ticks() * marks.strength as i64;
    let reach = snap.edge_half_window.max(mark_unit * marks.max_level as i64);
    let mut search = Search { original: ElementTimes::default(), row: RowId(0), offset, zone, best_level: None, snapped: offset, marks };
    for (original, row) in candidates {
        search.original = *original;
        search.row = *row;
        let start = (original.start + offset).ticks();
        let end = (original.end() + offset).ticks();
        let (low, high) = (start.min(end), start.max(end));
        if high - low <= 2 * reach {
            visit(snap, &mut search, low - reach, high + reach, mark_unit);
        } else {
            visit(snap, &mut search, low - reach, low + reach, mark_unit);
            visit(snap, &mut search, high - reach, high + reach, mark_unit);
        }
    }
    search.snapped
}

/// Time order, with each time's effect edges before its marks, as `calculateSnapPoints` files them.
fn visit(snap: &DragSnap, search: &mut Search, from: i64, to: i64, mark_unit: i64) {
    let edges: &[EdgePoint] = snap.edges.get(&search.row).map_or(&[], Vec::as_slice);
    let mut edge_at = edges.partition_point(|e| e.time < from);
    let marks = search.marks;
    let mut mark_points = marks.range(from, to).peekable();
    loop {
        let edge = edges.get(edge_at).filter(|e| e.time <= to);
        match (edge, mark_points.peek()) {
            (None, None) => return,
            (Some(e), Some((t, _))) if e.time > **t => {
                let (t, details) = mark_points.next().expect("peeked");
                visit_marks(search, *t, details, mark_unit);
            }
            (Some(e), _) => {
                search.consider(e.time, ELEMENT_SNAP_LEVEL, snap.edge_half_window, Some(search.row), Some(e.edge));
                edge_at += 1;
            }
            (None, Some(_)) => {
                let (t, details) = mark_points.next().expect("peeked");
                visit_marks(search, *t, details, mark_unit);
            }
        }
    }
}

fn visit_marks(search: &mut Search, time: i64, details: &[MarkSnapDetail], mark_unit: i64) {
    for d in details {
        search.consider(time, d.level, mark_unit * d.level as i64, None, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::marks::{Mark, MarkCollection, MarkDecorator};

    const TPP: TimeSpan = TimeSpan(1_000_000); // 100 ms

    fn sec(v: f64) -> TimeSpan {
        TimeSpan::from_secs(v)
    }

    fn eff(a: f64, b: f64) -> ElementTimes {
        ElementTimes::new(sec(a), sec(b - a))
    }

    fn collection(level: i32, times: &[f64]) -> MarkCollection {
        let mut c = MarkCollection { level, show_grid_lines: true, decorator: MarkDecorator { color: Color::RED, ..MarkDecorator::default() }, ..MarkCollection::default() };
        for t in times {
            c.add_mark(Mark::new(sec(*t)));
        }
        c
    }

    fn index(collections: &[MarkCollection]) -> MarkSnapIndex {
        let mut index = MarkSnapIndex { tpp: TPP, ..MarkSnapIndex::default() };
        index.rebuild(collections, 2);
        index
    }

    fn edges(points: &[(f64, u32, SnapEdge)]) -> DragSnap {
        DragSnap::new(points.iter().map(|(t, r, e)| (sec(*t), RowId(*r), *e)).collect(), TPP, 2)
    }

    fn snapped(snap: &DragSnap, marks: &MarkSnapIndex, e: ElementTimes, dt: f64, zone: ResizeZone) -> TimeSpan {
        find_snap(snap, marks, &[(e, RowId(1))], sec(dt), zone)
    }

    #[test]
    fn windows_scale_with_level_and_strength() {
        assert_eq!(snap_window(sec(20.0), TPP, 5, 2), (sec(19.0), sec(21.0)));
        assert_eq!(snap_window(sec(20.0), TPP, 1, 2), (sec(19.8), sec(20.2)));
    }

    #[test]
    fn nothing_near_leaves_the_offset_alone() {
        let snap = edges(&[(50.0, 1, SnapEdge::Start)]);
        let none = index(&[]);
        assert_eq!(snapped(&snap, &none, eff(10.0, 12.0), 7.5, ResizeZone::None), sec(7.5));
        assert_eq!(snapped(&snap, &none, eff(10.0, 12.0), 0.0, ResizeZone::None), TimeSpan::ZERO);
    }

    #[test]
    fn a_neighbours_edge_pulls_the_end() {
        let snap = edges(&[(20.0, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), eff(10.0, 12.0), 7.5, ResizeZone::None), sec(8.0));
    }

    #[test]
    fn level_decides_between_a_mark_and_an_edge() {
        let snap = edges(&[(20.0, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[collection(1, &[19.6])]), eff(10.0, 12.0), 7.5, ResizeZone::None), sec(8.0));
        assert_eq!(snapped(&snap, &index(&[collection(6, &[19.6])]), eff(10.0, 12.0), 7.5, ResizeZone::None), sec(7.6));
    }

    #[test]
    fn equal_level_takes_the_earliest_not_the_nearest_q12() {
        let snap = edges(&[(19.0, 1, SnapEdge::End), (20.0, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), eff(10.0, 12.0), 7.9, ResizeZone::None), sec(7.0));
    }

    #[test]
    fn the_window_is_strict() {
        // End lands exactly on SnapEnd (21.0) of a point at 20.0.
        let snap = edges(&[(20.0, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), eff(10.0, 12.0), 9.0, ResizeZone::None), sec(9.0));
    }

    #[test]
    fn both_edges_in_range_use_the_odd_comparison_q13() {
        let snap = edges(&[(19.5, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), eff(10.0, 11.0), 9.0, ResizeZone::None), sec(8.5));
        assert_eq!(snapped(&snap, &index(&[]), eff(30.0, 31.0), -11.0, ResizeZone::None), sec(-10.5));
    }

    #[test]
    fn a_point_inside_the_original_span_asks_which_original_edge_is_nearer_q13() {
        let e = eff(10.0, 11.0);
        // 2 * 10.4 is below 21: the start. And the offset is on both sides and cancels, so a bigger one gives the same answer.
        let snap = edges(&[(10.4, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), e, 0.1, ResizeZone::None), sec(0.4));
        assert_eq!(snapped(&snap, &index(&[]), e, 0.2, ResizeZone::None), sec(0.4));
        // A tie goes to the end.
        let snap = edges(&[(10.5, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), e, 0.1, ResizeZone::None), sec(-0.5));
        assert_eq!(snapped(&snap, &index(&[]), e, 0.2, ResizeZone::None), sec(-0.5));
    }

    #[test]
    fn the_best_level_carries_across_candidates() {
        // The first effect is in range of a level 5 point, and the second of a level 1 mark: the level 5 result stands.
        let snap = edges(&[(20.0, 1, SnapEdge::Start)]);
        let marks = index(&[collection(1, &[40.1])]);
        let candidates = [(eff(10.0, 12.0), RowId(1)), (eff(30.0, 32.0), RowId(1))];
        assert_eq!(find_snap(&snap, &marks, &candidates, sec(7.5), ResizeZone::None), sec(8.0));
    }

    #[test]
    fn edges_only_apply_in_their_own_row_but_marks_apply_everywhere() {
        let snap = edges(&[(20.0, 7, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), eff(10.0, 12.0), 7.5, ResizeZone::None), sec(7.5));
        assert_eq!(snapped(&snap, &index(&[collection(5, &[20.0])]), eff(10.0, 12.0), 7.5, ResizeZone::None), sec(8.0));
    }

    #[test]
    fn opposing_edges_are_skipped_on_resize() {
        let e = eff(10.0, 12.0);
        // Resizing Front, the start edge dragged close to its own end.
        let snap = edges(&[(12.0, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), e, 1.5, ResizeZone::Front), sec(1.5));
        let snap = edges(&[(12.0, 1, SnapEdge::End)]);
        assert_eq!(snapped(&snap, &index(&[]), e, 1.5, ResizeZone::Front), sec(2.0));
        assert_eq!(snapped(&edges(&[]), &index(&[collection(5, &[12.0])]), e, 1.5, ResizeZone::Front), sec(2.0));
        // Resizing Back, the end edge dragged close to its own start.
        let snap = edges(&[(10.0, 1, SnapEdge::End)]);
        assert_eq!(snapped(&snap, &index(&[]), e, -1.5, ResizeZone::Back), sec(-1.5));
        let snap = edges(&[(10.0, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), e, -1.5, ResizeZone::Back), sec(-2.0));
    }

    #[test]
    fn resizing_only_watches_the_active_edge() {
        let e = eff(10.0, 12.0);
        // A point near the end, resizing the front: ignored.
        let snap = edges(&[(14.0, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), e, 1.5, ResizeZone::Front), sec(1.5));
        // A point near the start, resizing the back: ignored.
        let snap = edges(&[(11.5, 1, SnapEdge::Start)]);
        assert_eq!(snapped(&snap, &index(&[]), e, 1.5, ResizeZone::Back), sec(1.5));
    }

    #[test]
    fn edge_windows_keep_the_drag_start_zoom_but_mark_windows_follow() {
        let snap = edges(&[(25.0, 1, SnapEdge::Start)]);
        let mut marks = index(&[collection(5, &[30.0])]);
        let e = eff(10.0, 12.0);
        // At the zoom the drag started with, neither is in reach.
        assert_eq!(snapped(&snap, &marks, e, 9.5, ResizeZone::None), sec(9.5));
        // Zoomed out: the mark's window grew to ±10 s, the edge's stayed at ±1 s.
        marks.tpp = TimeSpan(TPP.ticks() * 10);
        assert_eq!(snapped(&snap, &marks, e, 9.5, ResizeZone::None), sec(18.0));
        assert_eq!(snapped(&snap, &index(&[]), e, 9.5, ResizeZone::None), sec(9.5));
    }

    #[test]
    fn range_query_matches_a_full_scan() {
        // A scan of everything in time order, written the way the original does it.
        fn full_scan(points: &[(f64, u32, SnapEdge)], e: ElementTimes, dt: TimeSpan) -> TimeSpan {
            let mut sorted: Vec<_> = points.to_vec();
            sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let (mut best, mut out) = (None::<i32>, dt);
            for (t, row, _) in sorted {
                if best.is_some_and(|b| ELEMENT_SNAP_LEVEL <= b) || row != 1 {
                    continue;
                }
                let (lo, hi) = snap_window(sec(t), TPP, ELEMENT_SNAP_LEVEL, 2);
                let (s, en) = (e.start + dt, e.end() + dt);
                let (si, ei) = (s > lo && s < hi, en > lo && en < hi);
                if !si && !ei {
                    continue;
                }
                out = if si && ei {
                    if (sec(t) - e.start + dt) < (e.end() + dt - sec(t)) { sec(t) - e.start } else { sec(t) - e.end() }
                } else if si {
                    sec(t) - e.start
                } else {
                    sec(t) - e.end()
                };
                best = Some(ELEMENT_SNAP_LEVEL);
            }
            out
        }
        let mut seed = 99u64;
        let mut next = move |m: u64| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % m
        };
        for _ in 0..200 {
            let points: Vec<(f64, u32, SnapEdge)> = (0..next(40)).map(|_| (next(600) as f64 / 10.0, 1 + next(2) as u32, SnapEdge::Start)).collect();
            let start = next(300) as f64 / 10.0;
            let e = eff(start, start + 0.5 + next(50) as f64 / 10.0);
            let dt = sec(next(400) as f64 / 10.0 - 20.0);
            let got = find_snap(&edges(&points), &index(&[]), &[(e, RowId(1))], dt, ResizeZone::None);
            assert_eq!(got, full_scan(&points, e, dt), "{points:?} {e:?} {dt:?}");
        }
    }

    // GridMarkSnapPointTests.cs
    #[test]
    fn registers_each_visible_mark_boundary() {
        let mut first = collection(1, &[1.0]);
        first.show_tail_lines = true;
        first.marks[0].duration = sec(2.0);
        let second = collection(1, &[5.0]);
        let (a, b) = (first.marks[0].id, second.marks[0].id);
        let idx = index(&[first, second]);
        assert_eq!(idx.registration_count(), 2);
        assert_eq!(idx.registration(a), Some((sec(1.0), Some(sec(3.0)))));
        assert_eq!(idx.registration(b), Some((sec(5.0), None)));
    }

    #[test]
    fn marks_sharing_a_time_get_separate_details() {
        let idx = index(&[collection(1, &[1.0, 1.0])]);
        assert_eq!(idx.details_at(sec(1.0)).len(), 2);
        assert_ne!(idx.details_at(sec(1.0))[0].mark, idx.details_at(sec(1.0))[1].mark);
    }

    #[test]
    fn hidden_collections_register_nothing_and_tails_only_when_asked() {
        let mut hidden = collection(1, &[1.0]);
        hidden.show_grid_lines = false;
        let visible = collection(1, &[3.0]);
        let (h, v) = (hidden.marks[0].id, visible.marks[0].id);
        let idx = index(&[hidden, visible]);
        assert!(!idx.is_registered(h));
        assert_eq!(idx.registration(v).unwrap().1, None);
    }

    #[test]
    fn moving_one_mark_replaces_only_its_details() {
        let mut c = collection(1, &[1.0, 4.0]);
        c.show_tail_lines = true;
        let (moving, still) = (c.marks[0].id, c.marks[1].id);
        let mut cs = vec![c];
        let mut idx = index(&cs);
        let still_before = idx.registration(still);
        cs[0].marks[0].start_time = sec(2.0);
        assert!(idx.update_marks(&[moving], &cs));
        assert_eq!(idx.details_at(sec(1.0)).len(), 0);
        assert!(!idx.has_points_at(sec(1.0)));
        assert_eq!(idx.registration(moving).unwrap().0, sec(2.0));
        assert_eq!(idx.registration(still), still_before);
        assert_eq!(idx.details_at(sec(4.0)).len(), 1);
    }

    #[test]
    fn moving_a_mark_at_a_shared_time_keeps_the_other() {
        let c = collection(1, &[1.0, 1.0]);
        let (moving, other) = (c.marks[0].id, c.marks[1].id);
        let mut cs = vec![c];
        let mut idx = index(&cs);
        cs[0].marks[0].start_time = sec(2.0);
        idx.update_marks(&[moving], &cs);
        assert_eq!(idx.details_at(sec(1.0)).len(), 1);
        assert_eq!(idx.details_at(sec(1.0))[0].mark, other);
    }

    #[test]
    fn a_mark_listed_twice_moves_once() {
        let c = collection(1, &[1.0]);
        let id = c.marks[0].id;
        let mut cs = vec![c];
        let mut idx = index(&cs);
        cs[0].marks[0].start_time = sec(2.0);
        idx.update_marks(&[id, id, id], &cs);
        assert_eq!(idx.details_at(sec(2.0)).len(), 1);
        assert_eq!(idx.registration_count(), 1);
    }

    #[test]
    fn marks_in_several_collections_move_independently() {
        let (a, b) = (collection(1, &[1.0]), collection(2, &[1.0]));
        let (ida, idb) = (a.marks[0].id, b.marks[0].id);
        let mut cs = vec![a, b];
        let mut idx = index(&cs);
        cs[0].marks[0].start_time = sec(3.0);
        cs[1].marks[0].start_time = sec(4.0);
        idx.update_marks(&[ida, idb], &cs);
        assert!(!idx.has_points_at(sec(1.0)));
        assert_eq!(idx.details_at(sec(3.0))[0].mark, ida);
        assert_eq!(idx.details_at(sec(4.0))[0].mark, idb);
    }

    #[test]
    fn mark_lines_pick_one_detail_per_time_and_skip_guides() {
        let mut high = collection(5, &[2.0]);
        high.decorator.is_bold = true;
        high.decorator.color = Color::BLUE;
        let low = collection(1, &[2.0, 3.0]);
        let idx = index(&[low, high]);
        let lines = mark_lines(&idx, sec(0.0), sec(10.0), &[], TPP);
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[0].color, lines[0].width, lines[0].dash), (Color::BLUE, 3, 5));
        assert_eq!((lines[1].width, lines[1].dash, lines[1].x), (1, 1, 30.0));
        let lines = mark_lines(&idx, sec(0.0), sec(10.0), &[sec(2.0)], TPP);
        assert_eq!(lines.len(), 1);
        // The end of the range is exclusive, as `kvp.Key >= VisibleTimeEnd` stops the walk.
        assert_eq!(mark_lines(&idx, sec(0.0), sec(3.0), &[], TPP).len(), 1);
    }

    #[test]
    fn level_zero_does_not_make_a_bad_dash() {
        let idx = index(&[collection(0, &[1.0])]);
        assert_eq!(mark_lines(&idx, sec(0.0), sec(2.0), &[], TPP)[0].dash, 1);
    }
}
