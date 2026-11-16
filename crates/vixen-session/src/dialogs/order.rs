//! The Patching Order helper (F170 to F177) and `OrderModule`'s patching order functions.

use super::properties::ensure_property;
use super::world::{distinct_leaves, property_data, DialogWorld, NodeKey};
use std::collections::HashSet;
use vixen_core::element::{property_ids, PropertyData};

/// One row of the list. Its number is its position plus one, so it isn't stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderRow {
    pub node: NodeKey,
    pub name: String,
}

fn order_of(world: &dyn DialogWorld, node: NodeKey) -> i32 {
    match property_data(world, node, &property_ids::ORDER) {
        Some(PropertyData::Order { order }) => *order,
        // `Int32.MaxValue`: anything with no Order property goes last.
        _ => i32::MAX,
    }
}

/// F171: the distinct leaves, stable-sorted by their existing order.
pub fn order_helper_rows(world: &dyn DialogWorld, nodes: &[NodeKey]) -> Vec<OrderRow> {
    let mut rows: Vec<(i32, OrderRow)> = distinct_leaves(world, nodes)
        .into_iter()
        .map(|leaf| (order_of(world, leaf), OrderRow { node: leaf, name: world.node_name(leaf) }))
        .collect();
    rows.sort_by_key(|(order, _)| *order);
    rows.into_iter().map(|(_, row)| row).collect()
}

/// F172: moves rows to where the insertion line shows. `to` is an index into the list before the move. Several selected rows move
/// together, keeping their order.
pub fn order_move(rows: &mut Vec<OrderRow>, selected: &[usize], to: usize) {
    let mut selected: Vec<usize> = selected.iter().copied().filter(|i| *i < rows.len()).collect();
    selected.sort_unstable();
    selected.dedup();
    if selected.is_empty() {
        return;
    }
    let chosen: HashSet<usize> = selected.iter().copied().collect();
    let to = to.min(rows.len());
    let before = selected.iter().filter(|i| **i < to).count();
    let mut moving = Vec::with_capacity(selected.len());
    let mut rest = Vec::with_capacity(rows.len() - selected.len());
    for (i, row) in rows.drain(..).enumerate() {
        if chosen.contains(&i) {
            moving.push(row);
        } else {
            rest.push(row);
        }
    }
    let at = to - before;
    rest.splice(at..at, moving);
    *rows = rest;
}

/// F175: reverses the rows in the selected positions. Everything else stays put.
pub fn order_reverse(rows: &mut [OrderRow], selected: &[usize]) {
    let mut selected: Vec<usize> = selected.iter().copied().filter(|i| *i < rows.len()).collect();
    selected.sort_unstable();
    selected.dedup();
    let (mut start, mut end) = (0usize, selected.len());
    while start + 1 < end {
        end -= 1;
        rows.swap(selected[start], selected[end]);
        start += 1;
    }
}

/// F176's prompt: `(default, minimum, maximum)`.
pub fn order_zigzag_bounds(selected: usize) -> (i32, i32, i32) {
    let selected = selected.min(i32::MAX as usize) as i32;
    (selected.min(50), 2, selected)
}

/// F176: reverses every second group of `every` selected rows. Upstream's spelling of the message is kept.
pub fn order_zigzag(rows: &mut [OrderRow], selected: &[usize], every: i32) -> Result<(), String> {
    let mut selected: Vec<usize> = selected.iter().copied().filter(|i| *i < rows.len()).collect();
    selected.sort_unstable();
    selected.dedup();
    let every = every.max(1) as usize;
    if selected.len() % every != 0 {
        return Err("The total selected pixels must be evenly divisable by the zigzag length".to_string());
    }
    let mut i = every;
    while i < selected.len() {
        let (mut start, mut end) = (i, i + every - 1);
        while start < end {
            rows.swap(selected[start], selected[end]);
            start += 1;
            end -= 1;
        }
        i += 2 * every;
    }
    Ok(())
}

/// F177: numbers the rows from 1, adding the property to any leaf without one.
pub fn order_helper_apply(world: &mut dyn DialogWorld, rows: &[OrderRow]) {
    let nodes: Vec<NodeKey> = rows.iter().map(|r| r.node).collect();
    add_patching_order(world, &nodes);
}

/// `OrderModule.AddPatchingOrder(nodes)`.
pub fn add_patching_order(world: &mut dyn DialogWorld, nodes: &[NodeKey]) {
    for (i, node) in nodes.iter().enumerate() {
        let index = ensure_property(world, *node, &property_ids::ORDER);
        world.properties_mut(*node)[index].data = PropertyData::Order { order: i as i32 + 1 };
    }
}

/// `OrderModule.AddPatchingOrder(nodes, zigZagEvery)`.
pub fn add_patching_order_zigzag(world: &mut dyn DialogWorld, nodes: &[NodeKey], every: i32) {
    let mut total = 1;
    let mut group = 0;
    let mut forward = true;
    let mut group_start = 1;
    for node in nodes {
        let index = ensure_property(world, *node, &property_ids::ORDER);
        let order = if forward { total } else { group_start - group };
        world.properties_mut(*node)[index].data = PropertyData::Order { order };
        group += 1;
        if group >= every {
            forward = !forward;
            group = 0;
            if !forward {
                group_start = total + every;
            }
        }
        total += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::super::flow::test_support::*;
    use super::super::world::SystemWorld;
    use super::*;

    fn rows(count: u32) -> Vec<OrderRow> {
        (1..=count).map(|i| OrderRow { node: NodeKey(i), name: i.to_string() }).collect()
    }

    fn ids(rows: &[OrderRow]) -> Vec<u32> {
        rows.iter().map(|r| r.node.0).collect()
    }

    /// 1-based row numbers, as upstream's tests write them.
    fn positions(numbers: &[usize]) -> Vec<usize> {
        numbers.iter().map(|n| n - 1).collect()
    }

    #[test]
    fn zigzag_matches_upstreams_tests() {
        let cases: [(u32, &[usize], i32, &[u32]); 5] = [
            (10, &[3, 4, 5, 6], 2, &[1, 2, 3, 4, 6, 5, 7, 8, 9, 10]),
            (10, &[2, 4, 6, 8], 2, &[1, 2, 3, 4, 5, 8, 7, 6, 9, 10]),
            (10, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 2, &[1, 2, 4, 3, 5, 6, 8, 7, 9, 10]),
            (10, &[2, 4, 6, 7, 9, 10], 3, &[1, 2, 3, 4, 5, 6, 10, 8, 9, 7]),
            (16, &[2, 4, 6, 8, 10, 12, 14, 16], 2, &[1, 2, 3, 4, 5, 8, 7, 6, 9, 10, 11, 12, 13, 16, 15, 14]),
        ];
        for (count, selected, every, expected) in cases {
            let mut r = rows(count);
            order_zigzag(&mut r, &positions(selected), every).unwrap();
            assert_eq!(ids(&r), expected, "{selected:?} by {every}");
        }
    }

    #[test]
    fn zigzag_of_the_whole_selection_changes_nothing() {
        let mut r = rows(4);
        order_zigzag(&mut r, &positions(&[1, 2, 3, 4]), 4).unwrap();
        assert_eq!(ids(&r), [1, 2, 3, 4]);
    }

    #[test]
    fn zigzag_refuses_a_length_that_does_not_divide() {
        let mut r = rows(5);
        assert_eq!(
            order_zigzag(&mut r, &positions(&[1, 2, 3]), 2).unwrap_err(),
            "The total selected pixels must be evenly divisable by the zigzag length"
        );
        assert_eq!(ids(&r), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn reverse_leaves_everything_else_where_it_is() {
        let mut r = rows(6);
        order_reverse(&mut r, &positions(&[2, 3, 5]));
        assert_eq!(ids(&r), [1, 5, 3, 4, 2, 6]);
    }

    #[test]
    fn zigzag_bounds() {
        assert_eq!(order_zigzag_bounds(10), (10, 2, 10));
        assert_eq!(order_zigzag_bounds(200), (50, 2, 200));
    }

    #[test]
    fn moving_one_row_or_several() {
        let mut r = rows(5);
        order_move(&mut r, &[0], 3);
        assert_eq!(ids(&r), [2, 3, 1, 4, 5]);
        let mut r = rows(5);
        order_move(&mut r, &[4], 0);
        assert_eq!(ids(&r), [5, 1, 2, 3, 4]);
        let mut r = rows(6);
        order_move(&mut r, &[0, 2], 5);
        assert_eq!(ids(&r), [2, 4, 5, 1, 3, 6]);
        let mut r = rows(3);
        order_move(&mut r, &[1], 3);
        assert_eq!(ids(&r), [1, 3, 2]);
    }

    #[test]
    fn rows_sort_by_existing_order_with_the_rest_last_and_stable() {
        let (mut system, root, leaves) = system_with_leaves(4);
        let mut world = SystemWorld::new(&mut system);
        let i = ensure_property(&mut world, leaves[2], &property_ids::ORDER);
        world.properties_mut(leaves[2])[i].data = PropertyData::Order { order: 5 };
        let i = ensure_property(&mut world, leaves[3], &property_ids::ORDER);
        world.properties_mut(leaves[3])[i].data = PropertyData::Order { order: 2 };
        let r = order_helper_rows(&world, &[root]);
        assert_eq!(r.iter().map(|x| x.node).collect::<Vec<_>>(), [leaves[3], leaves[2], leaves[0], leaves[1]]);
    }

    #[test]
    fn apply_numbers_from_one_and_adds_what_is_missing() {
        let (mut system, root, leaves) = system_with_leaves(3);
        let mut world = SystemWorld::new(&mut system);
        let mut r = order_helper_rows(&world, &[root]);
        r.reverse();
        order_helper_apply(&mut world, &r);
        assert_eq!(property_data(&world, leaves[2], &property_ids::ORDER), Some(&PropertyData::Order { order: 1 }));
        assert_eq!(property_data(&world, leaves[0], &property_ids::ORDER), Some(&PropertyData::Order { order: 3 }));
    }

    #[test]
    fn patching_order_zigzag_matches_upstream() {
        let (mut system, _, leaves) = system_with_leaves(9);
        let mut world = SystemWorld::new(&mut system);
        add_patching_order_zigzag(&mut world, &leaves, 3);
        let orders: Vec<i32> = leaves
            .iter()
            .map(|n| match property_data(&world, *n, &property_ids::ORDER) {
                Some(PropertyData::Order { order }) => *order,
                _ => 0,
            })
            .collect();
        assert_eq!(orders, [1, 2, 3, 6, 5, 4, 7, 8, 9]);
    }
}
