//! Simple patching: `SetupPatchingSimple`'s counts, summary and the patch and unpatch algorithms, plus the two "find" buttons.

use super::controllers::OutputSelection;
use super::flow::{Cid, CompKind, PatchPoint, SourceRef};
use super::tree::{BitSet, Nid};
use super::SetupSession;
use std::collections::HashSet;
use vixen_core::element::{property_ids, PropertyData};

/// `_UpdateElementDetails`'s numbers (F62).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ElementStats {
    pub total: u32,
    pub connected: u32,
    pub unconnected: u32,
    pub items: u32,
    pub groups: u32,
    pub leaves: u32,
    pub filters: u32,
}

/// `_updateControllerDetails`'s numbers (F65).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControllerStats {
    pub outputs: u32,
    pub patched: u32,
    pub unpatched: u32,
    pub controllers: u32,
    pub first_label: String,
    pub last_label: String,
}

/// The radios and checkboxes of "Patching Options" (F67).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct PatchOptions {
    /// "Use all available patch points", rather than unconnected only.
    pub all_points: bool,
    /// "Use all selected controller outputs", rather than unpatched only.
    pub all_outputs: bool,
    pub reverse_elements: bool,
    pub reverse_outputs: bool,
}

/// F68.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatchSummary {
    pub sources: u32,
    pub destinations: u32,
    pub text: String,
    pub warning: String,
    pub can_patch: bool,
}

/// F69 step 2.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OrderCheck {
    TreeOrder,
    UseOrder,
    MissingSome,
    Duplicates,
}

/// `_updatePatchingSummary`.
pub fn patch_summary(elements: &ElementStats, controllers: &ControllerStats, options: &PatchOptions) -> PatchSummary {
    let sources = if options.all_points { elements.total } else { elements.unconnected };
    let destinations = if options.all_outputs { controllers.outputs } else { controllers.unpatched };
    let warning = if sources == 0 || destinations == 0 {
        ""
    } else if sources > destinations {
        "WARNING: too many elements, some will not be patched"
    } else if destinations > sources {
        "WARNING: too many outputs, some will not be patched"
    } else {
        ""
    };
    PatchSummary {
        sources,
        destinations,
        text: format!("This will patch {sources} element patch points to {destinations} controller outputs."),
        warning: warning.to_string(),
        can_patch: sources > 0 && destinations > 0,
    }
}

/// Above this many, unpatching asks first (F63, F66).
pub const UNPATCH_PROMPT_THRESHOLD: u32 = 20;

/// Per-element patch counts, kept for a whole patching revision so a selection's numbers are a sum over its distinct leaves. Upstream walks all data flow per click,
/// nesting LINQ once per level, which is the "every click recomputes everything" hazard.
#[derive(Default)]
pub struct LeafTable {
    revision: u64,
    /// By node slot. A node with no element reads as all zeros.
    entries: Vec<LeafEntry>,
}

#[derive(Copy, Clone, Default)]
struct LeafEntry {
    patched: u32,
    unpatched: u32,
    filters: u32,
}

impl SetupSession {
    fn fill_leaf_table(&self) -> parking_lot::MutexGuard<'_, LeafTable> {
        let mut table = self.leaf_table.lock();
        // Revision 0 is never current, and a table sized for fewer slots than there are nodes has gone stale too.
        if table.revision != self.patch_revision() || table.entries.len() != self.tree.slots() {
            table.revision = self.patch_revision();
            table.entries.clear();
            table.entries.resize(self.tree.slots(), LeafEntry::default());
            for (nid, node) in self.tree.live_nodes() {
                let Some(c) = node.element_id.and_then(|e| self.flow.find(&e)) else { continue };
                let (patched, unpatched) = self.flow.count_patch_points(c);
                let filters = self.count_filters_under(c);
                table.entries[nid.0 as usize] = LeafEntry { patched, unpatched, filters };
            }
        }
        table
    }

    fn count_filters_under(&self, component: Cid) -> u32 {
        let mut count = 0;
        let mut stack: Vec<Cid> = self.flow.destinations(component).to_vec();
        while let Some(c) = stack.pop() {
            let comp = self.flow.comp(c);
            if comp.kind == CompKind::Filter {
                count += 1;
                stack.extend_from_slice(&comp.destinations);
            }
        }
        count
    }

    /// `_countTypesDescendingFromElements` (F62): a node reached twice counts once. `items` is the selected rows, before any de-duplicating.
    pub fn element_stats(&self, selection: &[String]) -> ElementStats {
        let nodes = self.nodes_of(selection);
        let table = self.fill_leaf_table();
        let mut visited = BitSet::new(self.tree.slots());
        let mut stats = ElementStats { items: nodes.len() as u32, ..ElementStats::default() };
        let mut stack: Vec<Nid> = Vec::new();
        for node in nodes {
            stack.push(node);
            while let Some(n) = stack.pop() {
                if !visited.insert(n.0 as usize) {
                    continue;
                }
                let children = &self.tree.node(n).children;
                if children.is_empty() {
                    stats.leaves += 1;
                } else {
                    stats.groups += 1;
                    stack.extend_from_slice(children);
                }
                let entry = table.entries[n.0 as usize];
                stats.connected += entry.patched;
                stats.unconnected += entry.unpatched;
                stats.filters += entry.filters;
            }
        }
        stats.total = stats.connected + stats.unconnected;
        stats
    }

    /// F65, walking the selection in pane order with outputs ascending. Labels come from the range ends, before any reversing.
    pub fn controller_stats(&self, selection: &OutputSelection) -> ControllerStats {
        let mut stats = ControllerStats { controllers: selection.items.len() as u32, ..ControllerStats::default() };
        let mut first: Option<(u32, u32)> = None;
        let mut last: Option<(u32, u32)> = None;
        for (slot, ranges) in &selection.items {
            let Some(controller) = self.controllers.get(*slot as usize).filter(|c| c.alive) else { continue };
            for range in ranges {
                let end = range.end.min(controller.outputs.len() as u32);
                if range.start >= end {
                    continue;
                }
                if first.is_none() {
                    first = Some((*slot, range.start));
                }
                last = Some((*slot, end - 1));
                for output in &controller.outputs[range.start as usize..end as usize] {
                    if self.flow.source(output.cid).is_some() {
                        stats.patched += 1;
                    } else {
                        stats.unpatched += 1;
                    }
                }
            }
        }
        stats.outputs = stats.patched + stats.unpatched;
        let label = |(slot, index): (u32, u32)| format!("{} #{}", self.controllers[slot as usize].name, index + 1);
        stats.first_label = first.map(label).unwrap_or_default();
        stats.last_label = last.map(label).unwrap_or_default();
        stats
    }

    /// F69 step 1: the leaves of the selection in selection order, each once.
    fn ordered_leaves(&self, selection: &[String]) -> Vec<Nid> {
        self.tree.distinct_leaves(self.nodes_of(selection))
    }

    fn order_value(&self, node: Nid) -> Option<i32> {
        self.tree.node(node).properties.iter().find(|p| p.type_id == property_ids::ORDER).map(|p| match p.data {
            PropertyData::Order { order } => order,
            _ => 0,
        })
    }

    /// F69 step 2: what the leaves' preferred patching orders say.
    pub fn order_check(&self, selection: &[String]) -> OrderCheck {
        let leaves = self.ordered_leaves(selection);
        let orders: Vec<Option<i32>> = leaves.iter().map(|l| self.order_value(*l)).collect();
        let with = orders.iter().filter(|o| o.is_some()).count();
        if with == 0 {
            return OrderCheck::TreeOrder;
        }
        if with < orders.len() {
            return OrderCheck::MissingSome;
        }
        let distinct: HashSet<i32> = orders.iter().flatten().copied().collect();
        if distinct.len() < orders.len() {
            OrderCheck::Duplicates
        } else {
            OrderCheck::UseOrder
        }
    }

    /// `GetOrderedElementOutputs` once the prompts are answered. `use_order` is whether to sort by the Order property.
    fn ordered_sources(&self, selection: &[String], options: &PatchOptions, use_order: bool) -> Vec<PatchPoint> {
        let mut leaves = self.ordered_leaves(selection);
        if use_order {
            // A stable sort, as `OrderBy` is. A leaf the order helper hasn't reached sorts as 0, the module's default.
            leaves.sort_by_key(|l| self.order_value(*l).unwrap_or(0));
        }
        if options.reverse_elements {
            leaves.reverse();
        }
        let mut out = Vec::with_capacity(leaves.len());
        for leaf in leaves {
            if let Some(c) = self.tree.node(leaf).element_id.and_then(|e| self.flow.find(&e)) {
                self.flow.collect_patch_points(c, &mut out);
            }
        }
        out
    }

    /// F69 steps 3 to 7. Returns how many were patched, for "Patched {n} element patch points to controllers."
    pub fn patch_elements(&mut self, selection: &[String], outputs: &OutputSelection, options: &PatchOptions, use_order: bool) -> u32 {
        let mut sources = self.ordered_sources(selection, options, use_order);
        if !options.all_points {
            sources.retain(|p| !p.patched);
        }
        let mut destinations: Vec<Cid> = Vec::new();
        for (slot, ranges) in &outputs.items {
            let Some(controller) = self.controllers.get(*slot as usize).filter(|c| c.alive) else { continue };
            for range in ranges {
                let end = range.end.min(controller.outputs.len() as u32);
                if range.start < end {
                    destinations.extend(controller.outputs[range.start as usize..end as usize].iter().map(|o| o.cid));
                }
            }
        }
        if options.reverse_outputs {
            destinations.reverse();
        }
        if !options.all_outputs {
            destinations.retain(|d| self.flow.source(*d).is_none());
        }
        let count = sources.len().min(destinations.len());
        for i in 0..count {
            // Simple patching ignores a refusal, which can't happen here: a controller output has no outputs of its own.
            self.flow.set_source(destinations[i], SourceRef { component: sources[i].component, output: sources[i].output });
        }
        if count > 0 {
            self.patching_changed();
            self.bump_graph();
        }
        count as u32
    }

    /// F63 steps 1 to 3: how many points are patched, and whether any element is patched to a filter.
    pub fn unpatch_elements_preview(&self, selection: &[String]) -> (u32, bool) {
        let stats = self.element_stats(selection);
        let mut any_filter = false;
        'leaves: for leaf in self.ordered_leaves(selection) {
            let Some(c) = self.tree.node(leaf).element_id.and_then(|e| self.flow.find(&e)) else { continue };
            for child in self.flow.destinations(c) {
                if self.flow.subtree(*child).iter().any(|d| self.flow.comp(*d).kind == CompKind::Filter) {
                    any_filter = true;
                    break 'leaves;
                }
            }
        }
        (stats.connected, any_filter)
    }

    /// F63 steps 4 to 6. With `remove_filters` false the filters stay connected to their outputs with no source, so those outputs still read as patched until OK's
    /// clean-up removes the filters. Upstream's quirk, kept.
    pub fn unpatch_elements(&mut self, selection: &[String], remove_filters: bool) {
        let mut direct: Vec<Cid> = Vec::new();
        let mut below: Vec<Cid> = Vec::new();
        for leaf in self.ordered_leaves(selection) {
            let Some(c) = self.tree.node(leaf).element_id.and_then(|e| self.flow.find(&e)) else { continue };
            for child in self.flow.destinations(c).to_vec() {
                below.extend(self.flow.subtree(child));
                direct.push(child);
            }
        }
        let changed = !direct.is_empty();
        for child in &direct {
            self.flow.reset_source(*child);
        }
        if remove_filters {
            for c in below {
                if self.flow.is_live(c) && self.flow.comp(c).kind == CompKind::Filter {
                    self.remove_filter(c);
                }
            }
        }
        if changed {
            self.patching_changed();
            self.bump_graph();
        }
    }

    /// F66, F96, F126: every selected output loses its source. Filters above them stay connected to their elements.
    pub fn unpatch_outputs(&mut self, selection: &OutputSelection) {
        let mut changed = false;
        for (slot, ranges) in &selection.items {
            let Some(controller) = self.controllers.get(*slot as usize).filter(|c| c.alive) else { continue };
            let cids: Vec<Cid> = ranges
                .iter()
                .flat_map(|r| {
                    let end = r.end.min(controller.outputs.len() as u32);
                    (r.start..end).map(|i| controller.outputs[i as usize].cid)
                })
                .collect();
            for c in cids {
                changed |= self.flow.source(c).is_some();
                self.flow.reset_source(c);
            }
        }
        if changed {
            self.patching_changed();
            self.bump_graph();
        }
    }

    /// F14: the outputs these elements reach through any filters.
    pub fn find_outputs(&self, selection: &[String]) -> OutputSelection {
        let mut found: Vec<Vec<u32>> = vec![Vec::new(); self.controllers.len()];
        let mut visited = BitSet::new(self.flow.slots());
        let lookup = self.output_lookup();
        for leaf in self.ordered_leaves(selection) {
            let Some(c) = self.tree.node(leaf).element_id.and_then(|e| self.flow.find(&e)) else { continue };
            let mut stack = vec![c];
            while let Some(current) = stack.pop() {
                if !visited.insert(current.0 as usize) {
                    continue;
                }
                let comp = self.flow.comp(current);
                if comp.kind == CompKind::Output {
                    if let Some((slot, index)) = lookup.get(&current) {
                        found[*slot as usize].push(*index);
                    }
                }
                stack.extend_from_slice(&comp.destinations);
            }
        }
        OutputSelection::from_indexes(&self.controller_order, found)
    }

    /// F84: the element rows above these outputs. Duplicates are allowed, as upstream's list has them.
    pub fn find_elements(&self, selection: &OutputSelection) -> Vec<Nid> {
        let mut out = Vec::new();
        for (slot, ranges) in &selection.items {
            let Some(controller) = self.controllers.get(*slot as usize).filter(|c| c.alive) else { continue };
            for range in ranges {
                let end = range.end.min(controller.outputs.len() as u32);
                for i in range.start..end {
                    let cid = controller.outputs[i as usize].cid;
                    if self.flow.source(cid).is_none() {
                        continue;
                    }
                    let root = self.flow.root_source(cid);
                    let comp = self.flow.comp(root);
                    if comp.kind == CompKind::Element {
                        if let Some(n) = self.tree.find_by_element(&comp.id) {
                            out.push(n);
                        }
                    }
                }
            }
        }
        out
    }

    /// `Filters.RemoveFilter`.
    pub(crate) fn remove_filter(&mut self, cid: Cid) {
        self.flow.remove_component(cid);
        if let Some(i) = self.filter_lookup.get(&cid).copied() {
            let entry = &mut self.filters[i];
            entry.alive = false;
            self.removed_filter_data.push((entry.type_id, entry.instance_id));
        }
        self.dirty.filters = true;
    }
}

#[cfg(test)]
mod tests {
    use super::super::controllers::OutputSelection;
    use super::super::test_support::*;
    use super::*;
    use vixen_core::element::PropertyInstance;
    use uuid::Uuid;

    fn rows(s: &SetupSession, names: &[&str]) -> Vec<String> {
        names.iter().map(|n| s.row_path_of(s.tree.live_nodes().find(|(_, x)| x.name == *n).unwrap().0)).collect()
    }

    fn element(s: &SetupSession, name: &str) -> Cid {
        let node = s.tree.live_nodes().find(|(_, x)| x.name == name).unwrap().1;
        s.flow.find(&node.element_id.unwrap()).unwrap()
    }

    /// Which element feeds each output of the first controller, as `name` or `name:filter output`.
    fn wired(s: &SetupSession) -> Vec<Option<String>> {
        s.controllers[0]
            .outputs
            .iter()
            .map(|o| {
                let source = s.flow.source(o.cid)?;
                let root = s.flow.root_source(source.component);
                let name = s.tree.node(s.tree.find_by_element(&s.flow.comp(root).id).unwrap()).name.clone();
                Some(if s.flow.comp(source.component).kind == CompKind::Filter { format!("{name}:{}", source.output) } else { name })
            })
            .collect()
    }

    fn wiring(values: &[Option<&str>]) -> Vec<Option<String>> {
        values.iter().map(|v| v.map(str::to_string)).collect()
    }

    fn all(s: &SetupSession) -> OutputSelection {
        s.all_outputs_selection()
    }

    #[test]
    fn summary_text_and_warnings() {
        let elements = ElementStats { total: 3, unconnected: 3, ..Default::default() };
        let controllers = ControllerStats { outputs: 5, unpatched: 5, ..Default::default() };
        let s = patch_summary(&elements, &controllers, &PatchOptions::default());
        assert_eq!(s.text, "This will patch 3 element patch points to 5 controller outputs.");
        assert_eq!(s.warning, "WARNING: too many outputs, some will not be patched");
        assert!(s.can_patch);

        let elements = ElementStats { total: 5, unconnected: 5, ..Default::default() };
        let controllers = ControllerStats { outputs: 3, unpatched: 3, ..Default::default() };
        assert_eq!(patch_summary(&elements, &controllers, &PatchOptions::default()).warning, "WARNING: too many elements, some will not be patched");

        let none = ControllerStats::default();
        let s = patch_summary(&elements, &none, &PatchOptions::default());
        assert_eq!(s.warning, "");
        assert!(!s.can_patch);

        // The radios change what counts.
        let elements = ElementStats { total: 4, connected: 1, unconnected: 3, ..Default::default() };
        let controllers = ControllerStats { outputs: 4, patched: 2, unpatched: 2, ..Default::default() };
        let options = PatchOptions { all_points: true, all_outputs: true, ..Default::default() };
        let s = patch_summary(&elements, &controllers, &options);
        assert_eq!((s.sources, s.destinations, s.warning.as_str()), (4, 4, ""));
    }

    #[test]
    fn plain_patching_and_reversals() {
        let mut s = session(&["A", "B", "C"], 5);
        let selection = rows(&s, &["A", "B", "C"]);
        assert_eq!(s.patch_elements(&selection, &all(&s), &PatchOptions::default(), false), 3);
        assert_eq!(wired(&s), wiring(&[Some("A"), Some("B"), Some("C"), None, None]));

        let mut s = session(&["A", "B", "C"], 5);
        let selection = rows(&s, &["A", "B", "C"]);
        let options = PatchOptions { reverse_elements: true, ..Default::default() };
        s.patch_elements(&selection, &all(&s), &options, false);
        assert_eq!(wired(&s), wiring(&[Some("C"), Some("B"), Some("A"), None, None]));

        let mut s = session(&["A", "B", "C"], 5);
        let selection = rows(&s, &["A", "B", "C"]);
        let options = PatchOptions { reverse_outputs: true, ..Default::default() };
        s.patch_elements(&selection, &all(&s), &options, false);
        assert_eq!(wired(&s), wiring(&[None, None, Some("C"), Some("B"), Some("A")]));
    }

    #[test]
    fn selection_order_is_patching_order() {
        // SetupPatchingSimpleElementOrderTests: C selected, then ⇧-click A, gives C, B, A.
        let mut s = session(&["A", "B", "C"], 3);
        let selection = rows(&s, &["C", "B", "A"]);
        s.patch_elements(&selection, &all(&s), &PatchOptions::default(), false);
        assert_eq!(wired(&s), wiring(&[Some("C"), Some("B"), Some("A")]));
    }

    #[test]
    fn reversing_elements_keeps_colour_order_within_an_element() {
        let mut s = session(&["L1", "L2"], 6);
        for name in ["L1", "L2"] {
            let e = element(&s, name);
            add_breakdown(&mut s, e, 3);
        }
        s.patching_changed();
        let selection = rows(&s, &["L1", "L2"]);
        let options = PatchOptions { reverse_elements: true, ..Default::default() };
        s.patch_elements(&selection, &all(&s), &options, false);
        assert_eq!(
            wired(&s),
            wiring(&[Some("L2:0"), Some("L2:1"), Some("L2:2"), Some("L1:0"), Some("L1:1"), Some("L1:2")])
        );
    }

    #[test]
    fn the_radios_choose_what_gets_used() {
        // Unpatched only, with output 1 patched.
        let mut s = session(&["X", "A", "B", "C"], 4);
        let x = rows(&s, &["X"]);
        s.patch_elements(&x, &OutputSelection::single(0, 1..2), &PatchOptions::default(), false);
        let selection = rows(&s, &["A", "B", "C"]);
        s.patch_elements(&selection, &all(&s), &PatchOptions::default(), false);
        assert_eq!(wired(&s), wiring(&[Some("A"), Some("X"), Some("B"), Some("C")]));

        // All outputs: the second one's source is replaced.
        let mut s = session(&["X", "A", "B", "C"], 4);
        let x = rows(&s, &["X"]);
        s.patch_elements(&x, &OutputSelection::single(0, 1..2), &PatchOptions::default(), false);
        let selection = rows(&s, &["A", "B", "C"]);
        let options = PatchOptions { all_outputs: true, ..Default::default() };
        s.patch_elements(&selection, &all(&s), &options, false);
        assert_eq!(wired(&s), wiring(&[Some("A"), Some("B"), Some("C"), None]));
    }

    #[test]
    fn pressing_again_patches_the_next_unconnected_points() {
        let mut s = session(&["A", "B", "C", "D"], 2);
        let selection = rows(&s, &["A", "B", "C", "D"]);
        assert_eq!(s.patch_elements(&selection, &all(&s), &PatchOptions::default(), false), 2);
        assert_eq!(wired(&s), wiring(&[Some("A"), Some("B")]));
        s.set_output_count(s.controllers[0].id, 4);
        assert_eq!(s.patch_elements(&selection, &all(&s), &PatchOptions::default(), false), 2);
        assert_eq!(wired(&s), wiring(&[Some("A"), Some("B"), Some("C"), Some("D")]));
    }

    #[test]
    fn counts_visit_each_node_once() {
        let mut s = session(&["G", "L1", "L2"], 0);
        let names = rows(&s, &["G", "L1", "L2"]);
        s.drop_rows(&names[1..], Some(&names[0]), None, false);
        let l1 = format!("{}/{}", names[0], names[1]);
        let stats = s.element_stats(&[names[0].clone(), l1]);
        assert_eq!(stats, ElementStats { total: 2, connected: 0, unconnected: 2, items: 2, groups: 1, leaves: 2, filters: 0 });
    }

    #[test]
    fn counts_include_filters_and_points_through_them() {
        let mut s = session(&["A"], 2);
        let e = element(&s, "A");
        let f = add_breakdown(&mut s, e, 3);
        s.patching_changed();
        let selection = rows(&s, &["A"]);
        assert_eq!(s.element_stats(&selection), ElementStats { total: 3, connected: 0, unconnected: 3, items: 1, groups: 0, leaves: 1, filters: 1 });
        let output = s.controllers[0].outputs[0].cid;
        s.flow.set_source(output, SourceRef { component: f, output: 1 });
        s.patching_changed();
        let stats = s.element_stats(&selection);
        assert_eq!((stats.connected, stats.unconnected), (1, 2));
    }

    #[test]
    fn controller_labels_keep_pane_order_and_ignore_reversal() {
        // SetupPatchingSimpleOutputOrderTests.
        let mut system = system(&[], 0);
        system.controllers.push(controller("Main", 1704));
        let s = SetupSession::from_system(system, &HashSet::new(), false);
        let selection = OutputSelection::from_indexes(&s.controller_order, vec![vec![1700, 1638, 1703, 1650]]);
        let stats = s.controller_stats(&selection);
        assert_eq!((stats.first_label.as_str(), stats.last_label.as_str()), ("Main #1639", "Main #1704"));
        assert_eq!(stats.outputs, 4);

        let mut system = self::system(&[], 0);
        system.controllers.push(controller("First", 6));
        system.controllers.push(controller("Second", 4));
        let s = SetupSession::from_system(system, &HashSet::new(), false);
        let selection = OutputSelection::from_indexes(&s.controller_order, vec![vec![5, 1], vec![3, 0]]);
        let stats = s.controller_stats(&selection);
        assert_eq!((stats.first_label.as_str(), stats.last_label.as_str(), stats.controllers), ("First #2", "Second #4", 2));

        assert_eq!(s.controller_stats(&OutputSelection::default()), ControllerStats::default());
    }

    #[test]
    fn destination_order_with_and_without_reversal() {
        let mut system = self::system(&["A", "B", "C", "D"], 0);
        system.controllers.push(controller("First", 6));
        system.controllers.push(controller("Second", 4));
        let mut s = SetupSession::from_system(system, &HashSet::new(), false);
        let selection = rows(&s, &["A", "B", "C", "D"]);
        let outputs = OutputSelection::from_indexes(&s.controller_order, vec![vec![5, 1], vec![3, 0]]);
        s.patch_elements(&selection, &outputs, &PatchOptions::default(), false);
        let source_name = |s: &SetupSession, slot: usize, i: usize| {
            let e = s.flow.comp(s.flow.source(s.controllers[slot].outputs[i].cid).unwrap().component).id;
            s.tree.node(s.tree.find_by_element(&e).unwrap()).name.clone()
        };
        assert_eq!(source_name(&s, 0, 1), "A");
        assert_eq!(source_name(&s, 0, 5), "B");
        assert_eq!(source_name(&s, 1, 0), "C");
        assert_eq!(source_name(&s, 1, 3), "D");

        s.unpatch_outputs(&outputs);
        let options = PatchOptions { reverse_outputs: true, ..Default::default() };
        s.patch_elements(&selection, &outputs, &options, false);
        assert_eq!(source_name(&s, 1, 3), "A");
        assert_eq!(source_name(&s, 1, 0), "B");
        assert_eq!(source_name(&s, 0, 5), "C");
        assert_eq!(source_name(&s, 0, 1), "D");
    }

    fn give_order(s: &mut SetupSession, name: &str, order: i32) {
        let node = s.tree.live_nodes().find(|(_, x)| x.name == name).unwrap().0;
        s.tree.node_mut(node).properties.push(PropertyInstance {
            type_id: property_ids::ORDER,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Order { order },
        });
    }

    #[test]
    fn order_check_and_sorting() {
        let mut s = session(&["A", "B", "C"], 3);
        let selection = rows(&s, &["A", "B", "C"]);
        assert_eq!(s.order_check(&selection), OrderCheck::TreeOrder);
        give_order(&mut s, "A", 3);
        assert_eq!(s.order_check(&selection), OrderCheck::MissingSome);
        give_order(&mut s, "B", 3);
        give_order(&mut s, "C", 1);
        assert_eq!(s.order_check(&selection), OrderCheck::Duplicates);

        let mut s = session(&["A", "B", "C"], 3);
        give_order(&mut s, "A", 3);
        give_order(&mut s, "B", 2);
        give_order(&mut s, "C", 1);
        assert_eq!(s.order_check(&selection), OrderCheck::UseOrder);
        s.patch_elements(&selection, &all(&s), &PatchOptions::default(), true);
        assert_eq!(wired(&s), wiring(&[Some("C"), Some("B"), Some("A")]));
    }

    #[test]
    fn unpatching_elements_with_and_without_their_filters() {
        let build = || {
            let mut s = session(&["A"], 3);
            let e = element(&s, "A");
            add_breakdown(&mut s, e, 3);
            s.patching_changed();
            let selection = rows(&s, &["A"]);
            s.patch_elements(&selection, &all(&s), &PatchOptions::default(), false);
            (s, selection)
        };
        let (mut s, selection) = build();
        assert_eq!(s.unpatch_elements_preview(&selection), (3, true));
        s.unpatch_elements(&selection, true);
        assert_eq!(s.filters.iter().filter(|f| f.alive).count(), 0);
        assert_eq!(wired_count(&s), 0);
        assert_eq!(s.element_stats(&selection).unconnected, 1);

        // Keep them: the filters lose only their source, so the outputs still show as patched.
        let (mut s, selection) = build();
        s.unpatch_elements(&selection, false);
        assert_eq!(s.filters.iter().filter(|f| f.alive).count(), 1);
        assert_eq!(s.controllers[0].outputs.iter().filter(|o| s.flow.source(o.cid).is_some()).count(), 3);
        assert_eq!(s.element_stats(&selection).unconnected, 1);
    }

    fn wired_count(s: &SetupSession) -> usize {
        s.controllers[0].outputs.iter().filter(|o| s.flow.source(o.cid).is_some()).count()
    }

    #[test]
    fn unpatching_outputs_leaves_filters_on_their_elements() {
        let mut s = session(&["A"], 3);
        let e = element(&s, "A");
        let f = add_breakdown(&mut s, e, 3);
        s.patching_changed();
        let selection = rows(&s, &["A"]);
        s.patch_elements(&selection, &all(&s), &PatchOptions::default(), false);
        s.unpatch_outputs(&OutputSelection::single(0, 0..2));
        assert_eq!(wired_count(&s), 1);
        assert_eq!(s.flow.source(f).unwrap().component, e);
        let stats = s.element_stats(&selection);
        assert_eq!((stats.connected, stats.unconnected), (1, 2));
    }

    #[test]
    fn finding_goes_through_filter_chains_both_ways() {
        let mut s = session(&["A", "B"], 4);
        let e = element(&s, "A");
        add_breakdown(&mut s, e, 2);
        s.patching_changed();
        let selection = rows(&s, &["A", "B"]);
        s.patch_elements(&selection, &all(&s), &PatchOptions::default(), false);
        // A's two colours and B.
        let found = s.find_outputs(&selection);
        assert_eq!(found.items, vec![(0, vec![0..3])]);
        let a_only = s.find_outputs(&rows(&s, &["A"]));
        assert_eq!(a_only.items, vec![(0, vec![0..2])]);

        let elements = s.find_elements(&OutputSelection::single(0, 0..3));
        let names: Vec<String> = elements.iter().map(|n| s.tree.node(*n).name.clone()).collect();
        assert_eq!(names, ["A", "A", "B"]);
        assert!(s.find_elements(&OutputSelection::single(0, 3..4)).is_empty());
    }

    #[test]
    #[ignore = "a time budget, for release builds"]
    fn stats_over_a_hundred_thousand_leaves_from_the_table() {
        let names: Vec<String> = (0..100_000).map(|i| format!("L{i}")).collect();
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut s = session(&name_refs, 0);
        let rows: Vec<String> = s.tree.roots.iter().map(|n| s.row_path_of(*n)).collect();
        let _ = s.element_stats(&rows);
        let start = std::time::Instant::now();
        let stats = s.element_stats(&rows);
        let spent = start.elapsed();
        assert_eq!(stats.leaves, 100_000);
        assert!(spent.as_millis() < 50, "{spent:?}");
        let _ = &mut s;
    }
}
