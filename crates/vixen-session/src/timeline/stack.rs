//! Stacking overlapping effects in a row (F40) and the rectangle an occurrence draws in (F39). Pure.

use super::{ElementTimes, IRect};
use super::view::{px, TimeInfo};
use vixen_core::TimeSpan;

/// `Element.StackIndex` and `StackCount`, per occurrence rather than per effect (Q3). A count of 0 means never stacked, which
/// upstream draws at full height.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct StackSlot {
    pub index: i32,
    pub count: i32,
}

/// Reused between walks so stacking allocates once.
#[derive(Default)]
pub struct StackScratch {
    back: Vec<usize>,
    cluster: Vec<(usize, i32)>,
    lane_ends: Vec<i64>,
}

/// The walk behind `Row.SetStackIndexes` and `MarkRow`'s equivalent. `prefix_max_end`, when there is one, only saves scanning:
/// `prefix_max_end[i]` is the latest end among items `0..=i`. The results are the same either way.
///
/// KEEP Q4: the walk starts at the first item still running at `range_start`, so an item's lanes depend on the scroll position.
pub fn walk_stacks(
    len: usize,
    start_of: impl Fn(usize) -> i64,
    end_of: impl Fn(usize) -> i64,
    prefix_max_end: Option<&[i64]>,
    range_start: i64,
    range_end: i64,
    scratch: &mut StackScratch,
    mut assign: impl FnMut(usize, StackSlot),
) {
    let mut i = match prefix_max_end {
        Some(p) => p.partition_point(|&m| m < range_start),
        None => 0,
    };
    while i < len {
        if end_of(i) < range_start {
            i += 1;
            continue;
        }
        if start_of(i) > range_end {
            break;
        }
        // Forward until no more overlap. Only forward additions extend the cover.
        let mut cover_end = end_of(i);
        let mut forward_end = i + 1;
        while forward_end < len && start_of(forward_end) < cover_end {
            cover_end = cover_end.max(end_of(forward_end));
            forward_end += 1;
        }
        // Backward through everything earlier. Upstream never stops early; the running max end proves when nothing further can match.
        scratch.back.clear();
        let mut cover_start = start_of(i);
        let mut k = i;
        while k > 0 {
            k -= 1;
            if let Some(p) = prefix_max_end {
                if p[k] <= cover_start {
                    break;
                }
            }
            if end_of(k) > cover_start {
                scratch.back.push(k);
                cover_start = cover_start.min(start_of(k));
            }
        }
        let cluster_len = scratch.back.len() + (forward_end - i);
        if cluster_len > 1 {
            // First fit, in list order.
            scratch.cluster.clear();
            scratch.lane_ends.clear();
            let back = std::mem::take(&mut scratch.back);
            for item in back.iter().rev().copied().chain(i..forward_end) {
                let s = start_of(item);
                let lane = match scratch.lane_ends.iter().position(|&end| s >= end) {
                    Some(lane) => lane,
                    None => {
                        scratch.lane_ends.push(0);
                        scratch.lane_ends.len() - 1
                    }
                };
                scratch.lane_ends[lane] = end_of(item);
                scratch.cluster.push((item, lane as i32));
            }
            scratch.back = back;
            let count = scratch.lane_ends.len() as i32;
            for &(item, lane) in &scratch.cluster {
                assign(item, StackSlot { index: lane, count });
            }
        } else {
            assign(i, StackSlot { index: 0, count: 1 });
        }
        i = forward_end;
    }
}

/// F40 for one row, indexed like the input, which must be sorted by start then end. Items the walk does not reach keep count 0.
pub fn stack_row(elements: &[ElementTimes], range_start: TimeSpan, range_end: TimeSpan) -> Vec<StackSlot> {
    let mut slots = vec![StackSlot::default(); elements.len()];
    let mut scratch = StackScratch::default();
    walk_stacks(
        elements.len(),
        |i| elements[i].start.ticks(),
        |i| elements[i].end().ticks(),
        None,
        range_start.ticks(),
        range_end.ticks(),
        &mut scratch,
        |i, slot| slots[i] = slot,
    );
    slots
}

/// What `Grid.DrawElement` works out for one occurrence.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct OccurrenceRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub row_top_offset: i32,
    pub red_border: bool,
    pub overall_width: i32,
}

impl OccurrenceRect {
    pub fn rect(&self) -> IRect {
        IRect::new(self.x, self.y, self.w, self.h)
    }
}

/// Height and vertical offset within the row. Hit testing and painting share this.
#[inline]
pub fn lane_geometry(slot: StackSlot, row_height: i32) -> (i32, i32, bool) {
    let h = if slot.count != 0 { (row_height - 1) / slot.count } else { row_height - 1 };
    let offset = h * slot.index;
    if h == 0 {
        // Too many lanes. The y values stay as computed with 0.
        (row_height, offset, true)
    } else {
        (h, offset, false)
    }
}

/// F39. `w` at or below 0 means not drawn.
pub fn occurrence_rect(times: ElementTimes, slot: StackSlot, row_top: i32, row_height: i32, info: &TimeInfo, vis_end: TimeSpan) -> OccurrenceRect {
    let tpp = info.time_per_pixel;
    let vis_start = info.visible_start;
    let (h, offset, red_border) = lane_geometry(slot, row_height);
    let (start, end) = (times.start, times.end());
    let w = if start >= vis_start {
        if end < vis_end {
            px(times.duration, tpp) as i32
        } else {
            (px(vis_end, tpp) - px(start, tpp)) as i32
        }
    } else if end <= vis_end {
        (px(end, tpp) - px(vis_start, tpp)) as i32
    } else {
        (px(vis_end, tpp) - px(vis_start, tpp)) as i32
    };
    OccurrenceRect {
        x: px(start.max(vis_start), tpp).floor() as i32,
        y: row_top + offset,
        w,
        h,
        row_top_offset: offset,
        red_border,
        overall_width: px(times.duration, tpp) as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(start: f64, end: f64) -> ElementTimes {
        ElementTimes::new(TimeSpan::from_secs(start), TimeSpan::from_secs(end - start))
    }

    fn slots(items: &[ElementTimes], from: f64, to: f64) -> Vec<(i32, i32)> {
        stack_row(items, TimeSpan::from_secs(from), TimeSpan::from_secs(to)).iter().map(|s| (s.index, s.count)).collect()
    }

    #[test]
    fn touching_effects_share_a_lane() {
        let items = [t(0.0, 10.0), t(5.0, 15.0), t(10.0, 20.0)];
        assert_eq!(slots(&items, 0.0, 100.0), [(0, 2), (1, 2), (0, 2)]);
    }

    #[test]
    fn lane_count_depends_on_the_scroll_position_q4() {
        let items = [t(0.0, 10.0), t(5.0, 6.0), t(5.5, 8.0), t(9.0, 20.0)];
        assert_eq!(slots(&items, 0.0, 100.0)[3], (1, 3));
        assert_eq!(slots(&items, 15.0, 100.0)[3], (1, 2));
    }

    #[test]
    fn single_effects_get_one_lane_and_unreached_keep_zero() {
        let items = [t(0.0, 1.0), t(2.0, 3.0), t(50.0, 51.0)];
        assert_eq!(slots(&items, 1.5, 3.0), [(0, 0), (0, 1), (0, 0)]);
    }

    #[test]
    fn the_scan_shortcut_matches_a_full_scan() {
        // Pseudo random effects, compared with and without the running max.
        let mut seed = 12345u64;
        let mut next = move |m: u64| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % m
        };
        for _ in 0..50 {
            let n = 1 + next(60) as usize;
            let mut items: Vec<ElementTimes> =
                (0..n).map(|_| ElementTimes::new(TimeSpan(next(5000) as i64), TimeSpan(1 + next(600) as i64))).collect();
            items.sort_by_key(|e| (e.start, e.end()));
            let mut prefix = Vec::with_capacity(n);
            let mut m = i64::MIN;
            for e in &items {
                m = m.max(e.end().ticks());
                prefix.push(m);
            }
            for _ in 0..8 {
                let a = next(5500) as i64;
                let b = a + next(2000) as i64;
                let full = stack_row(&items, TimeSpan(a), TimeSpan(b));
                let mut fast = vec![StackSlot::default(); n];
                let mut scratch = StackScratch::default();
                walk_stacks(n, |i| items[i].start.ticks(), |i| items[i].end().ticks(), Some(&prefix), a, b, &mut scratch, |i, s| fast[i] = s);
                assert_eq!(full, fast);
            }
        }
    }

    fn info(start: f64) -> TimeInfo {
        TimeInfo { visible_start: TimeSpan::from_secs(start), ..TimeInfo::default() }
    }

    #[test]
    fn lanes_divide_the_row_height() {
        let two = StackSlot { index: 1, count: 2 };
        assert_eq!(lane_geometry(two, 32), (15, 15, false));
        assert_eq!(lane_geometry(StackSlot { index: 0, count: 2 }, 32), (15, 0, false));
        let (h, offset, red) = lane_geometry(StackSlot { index: 3, count: 40 }, 32);
        assert_eq!((h, offset, red), (32, 0, true));
        assert_eq!(lane_geometry(StackSlot::default(), 32), (31, 0, false));
    }

    #[test]
    fn clipped_widths_take_all_four_branches() {
        let slot = StackSlot { index: 0, count: 1 };
        let vis_end = TimeSpan::from_secs(20.0);
        // Inside the view.
        let r = occurrence_rect(t(11.0, 12.0), slot, 0, 32, &info(10.0), vis_end);
        assert_eq!((r.x, r.w), (1100, 100));
        // Runs past the end of the view.
        let r = occurrence_rect(t(19.0, 25.0), slot, 0, 32, &info(10.0), vis_end);
        assert_eq!((r.x, r.w, r.overall_width), (1900, 100, 600));
        // Started before the view.
        let r = occurrence_rect(t(9.0, 11.0), slot, 0, 32, &info(10.0), vis_end);
        assert_eq!((r.x, r.w), (1000, 100));
        // Covers the whole view.
        let r = occurrence_rect(t(5.0, 25.0), slot, 0, 32, &info(10.0), vis_end);
        assert_eq!((r.x, r.w), (1000, 1000));
        // Under a pixel wide is not drawn.
        let r = occurrence_rect(ElementTimes::new(TimeSpan::from_secs(11.0), TimeSpan(10)), slot, 0, 32, &info(10.0), vis_end);
        assert!(r.w <= 0);
    }

    #[test]
    fn x_is_floored() {
        let slot = StackSlot { index: 0, count: 1 };
        let times = ElementTimes::new(TimeSpan(100_000 * 15 + 50_000), TimeSpan::from_secs(1.0));
        let r = occurrence_rect(times, slot, 7, 32, &info(0.0), TimeSpan::from_secs(20.0));
        assert_eq!((r.x, r.y), (15, 7));
    }
}
