//! Selection order in a tree (F26, F71): display order, except during a range gesture, where it runs from the anchor to the endpoint. That is what makes
//! "⇧-select upward, then patch" patch in reverse. `NSOutlineView` doesn't expose this, so the window reports its anchor and this works out the order.

/// `selected` holds row numbers in display order. With an anchor and endpoint, the rows between them come last, from the anchor to the endpoint, and any other
/// selected rows (⌘-clicked earlier) come first in display order. A selection made some other way has no anchor and stays in display order.
pub fn ordered_rows(selected: &[u32], anchor: Option<u32>, endpoint: Option<u32>) -> Vec<u32> {
    let (Some(anchor), Some(endpoint)) = (anchor, endpoint) else { return selected.to_vec() };
    let (low, high) = (anchor.min(endpoint), anchor.max(endpoint));
    let mut out: Vec<u32> = selected.iter().copied().filter(|r| *r < low || *r > high).collect();
    let in_range = selected.iter().copied().filter(|r| (low..=high).contains(r));
    if anchor <= endpoint {
        out.extend(in_range);
    } else {
        let mut upward: Vec<u32> = in_range.collect();
        upward.reverse();
        out.extend(upward);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // MultiSelectTreeviewRangeSelectionOrderTests and MultiSelectTreeviewKeyboardSelectionTests, in row numbers.
    #[test]
    fn a_plain_selection_stays_in_display_order() {
        assert_eq!(ordered_rows(&[1, 3, 5], None, None), [1, 3, 5]);
    }

    #[test]
    fn a_range_made_upward_is_reversed() {
        assert_eq!(ordered_rows(&[0, 1, 2], Some(2), Some(0)), [2, 1, 0]);
    }

    #[test]
    fn a_range_made_downward_stays_forward() {
        assert_eq!(ordered_rows(&[2, 3, 4], Some(2), Some(4)), [2, 3, 4]);
    }

    #[test]
    fn shrinking_back_towards_the_anchor() {
        assert_eq!(ordered_rows(&[1, 2], Some(2), Some(1)), [2, 1]);
        assert_eq!(ordered_rows(&[2], Some(2), Some(2)), [2]);
    }

    #[test]
    fn crossing_the_anchor_turns_the_order_around() {
        // ⇧↑ twice from row 2, then ⇧↓ three times.
        assert_eq!(ordered_rows(&[0, 1, 2], Some(2), Some(0)), [2, 1, 0]);
        assert_eq!(ordered_rows(&[2, 3], Some(2), Some(3)), [2, 3]);
    }

    #[test]
    fn rows_outside_the_range_come_first_in_display_order() {
        assert_eq!(ordered_rows(&[0, 5, 6, 7], Some(7), Some(5)), [0, 7, 6, 5]);
    }

    #[test]
    fn collapsed_descendants_never_appear_because_rows_are_visible_rows() {
        // The window numbers visible rows only, so a collapsed group's children have no row number to be in the range.
        assert_eq!(ordered_rows(&[3, 4], Some(4), Some(3)), [4, 3]);
    }
}
