//! The editable data flow graph: the port of `DataFlowManager`. `vixen_output::OutputGraph` is the compiled form used at run
//! time, and is rebuilt from this when the window closes with OK.

use std::collections::HashMap;
use uuid::Uuid;
use vixen_io::Patch;

/// A component's slot. Stays valid for the session, so a selection or a shape can hold one across edits.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Cid(pub u32);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CompKind {
    Element,
    Filter,
    Output,
}

/// `IDataFlowComponentReference`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SourceRef {
    pub component: Cid,
    pub output: i32,
}

#[derive(Clone, Debug)]
pub struct Comp {
    pub id: Uuid,
    pub kind: CompKind,
    /// `Outputs.Length`. Controller outputs have none (`Outputs == null`), which ends patch point recursion.
    pub output_count: u32,
    pub source: Option<SourceRef>,
    /// In connection order, which decides the order of patch points after a save and reload.
    pub destinations: Vec<Cid>,
    pub alive: bool,
}

/// One end of an element's data flow chain (F61).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PatchPoint {
    pub component: Cid,
    pub output: i32,
    pub patched: bool,
}

/// A patch that `Initialize` ignored at load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlowWarning {
    MissingComponent { component_id: Uuid },
    MissingSource { component_id: Uuid, source_id: Option<Uuid> },
    OutputOutOfRange { component_id: Uuid, source_id: Uuid, output: i32 },
    Circular { component_id: Uuid, source_id: Uuid },
}

#[derive(Clone, Debug, Default)]
pub struct DataFlow {
    comps: Vec<Comp>,
    lookup: HashMap<Uuid, Cid>,
}

impl DataFlow {
    pub fn new() -> DataFlow {
        DataFlow::default()
    }

    /// `AddComponent`. Adding an id that's already there returns what's there.
    pub fn add_component(&mut self, id: Uuid, kind: CompKind, output_count: u32) -> Cid {
        if let Some(existing) = self.lookup.get(&id) {
            return *existing;
        }
        let cid = Cid(self.comps.len() as u32);
        self.comps.push(Comp { id, kind, output_count, source: None, destinations: Vec::new(), alive: true });
        self.lookup.insert(id, cid);
        cid
    }

    /// `DataFlowManager.Initialize`: patches in order, ignoring those with a missing end or a bad index.
    pub fn apply_patches(&mut self, patches: &[Patch]) -> Vec<FlowWarning> {
        let mut warnings = Vec::new();
        for p in patches {
            let Some(component) = self.find(&p.component_id) else {
                warnings.push(FlowWarning::MissingComponent { component_id: p.component_id });
                continue;
            };
            let Some(source_id) = p.source_id else {
                warnings.push(FlowWarning::MissingSource { component_id: p.component_id, source_id: None });
                continue;
            };
            let Some(source) = self.find(&source_id) else {
                warnings.push(FlowWarning::MissingSource { component_id: p.component_id, source_id: Some(source_id) });
                continue;
            };
            if p.output < 0 || p.output as u32 >= self.comp(source).output_count {
                warnings.push(FlowWarning::OutputOutOfRange { component_id: p.component_id, source_id, output: p.output });
                continue;
            }
            if !self.set_source(component, SourceRef { component: source, output: p.output }) {
                warnings.push(FlowWarning::Circular { component_id: p.component_id, source_id });
            }
        }
        warnings
    }

    /// `GetEnumerator`: every destination of every component, components in the order they were added. For a file that hasn't
    /// changed this gives back its patches in the same order.
    pub fn to_patches(&self) -> Vec<Patch> {
        let mut out = Vec::with_capacity(self.comps.len());
        for c in self.comps.iter().filter(|c| c.alive) {
            for d in &c.destinations {
                let dest = self.comp(*d);
                let source = dest.source.expect("a destination has a source");
                out.push(Patch { component_id: dest.id, source_id: Some(c.id), output: source.output });
            }
        }
        out
    }

    #[inline]
    pub fn comp(&self, id: Cid) -> &Comp {
        &self.comps[id.0 as usize]
    }

    #[inline]
    pub fn is_live(&self, id: Cid) -> bool {
        self.comps.get(id.0 as usize).is_some_and(|c| c.alive)
    }

    #[inline]
    pub fn find(&self, id: &Uuid) -> Option<Cid> {
        self.lookup.get(id).copied()
    }

    pub fn slots(&self) -> usize {
        self.comps.len()
    }

    pub fn components(&self) -> impl Iterator<Item = (Cid, &Comp)> {
        self.comps.iter().enumerate().filter(|(_, c)| c.alive).map(|(i, c)| (Cid(i as u32), c))
    }

    #[inline]
    pub fn source(&self, id: Cid) -> Option<SourceRef> {
        self.comp(id).source
    }

    #[inline]
    pub fn destinations(&self, id: Cid) -> &[Cid] {
        &self.comp(id).destinations
    }

    /// `GetDestinationsOfComponentOutput`.
    pub fn destinations_of_output(&self, id: Cid, output: i32) -> impl Iterator<Item = Cid> + '_ {
        self.comp(id).destinations.iter().copied().filter(move |d| self.comp(*d).source.is_some_and(|s| s.output == output))
    }

    /// `_CheckComponentSourceForCircularDependency`: following `source` upward reaches `component`.
    pub fn would_cycle(&self, component: Cid, source: Cid) -> bool {
        let mut current = Some(source);
        while let Some(c) = current {
            if c == component {
                return true;
            }
            current = self.comp(c).source.map(|s| s.component);
        }
        false
    }

    /// `SetComponentSource`: refuses a loop, otherwise detaches the old source and attaches the new. False means nothing changed.
    pub fn set_source(&mut self, component: Cid, source: SourceRef) -> bool {
        if self.would_cycle(component, source.component) {
            return false;
        }
        self.reset_source(component);
        self.comps[component.0 as usize].source = Some(source);
        self.comps[source.component.0 as usize].destinations.push(component);
        true
    }

    /// `ResetComponentSource`.
    pub fn reset_source(&mut self, component: Cid) {
        if let Some(old) = self.comps[component.0 as usize].source.take() {
            let list = &mut self.comps[old.component.0 as usize].destinations;
            // Newest first, because a big fan-out is built and torn down most recent end first.
            if let Some(i) = list.iter().rposition(|d| *d == component) {
                list.remove(i);
            }
        }
    }

    /// `RemoveComponent`: detaches all its destinations, then its own source, then forgets it.
    pub fn remove_component(&mut self, component: Cid) {
        if !self.is_live(component) {
            return;
        }
        for d in std::mem::take(&mut self.comps[component.0 as usize].destinations) {
            self.comps[d.0 as usize].source = None;
        }
        self.reset_source(component);
        let c = &mut self.comps[component.0 as usize];
        c.alive = false;
        let id = c.id;
        self.lookup.remove(&id);
    }

    /// After a filter's setup changes how many outputs it has: anything on a removed output loses its source.
    pub fn set_output_count(&mut self, component: Cid, count: u32) {
        self.comps[component.0 as usize].output_count = count;
        let dropped: Vec<Cid> = self
            .comp(component)
            .destinations
            .iter()
            .copied()
            .filter(|d| self.comp(*d).source.is_some_and(|s| s.output < 0 || s.output as u32 >= count))
            .collect();
        for d in dropped {
            self.reset_source(d);
        }
    }

    /// The upward walk of F84: the first component with no source. Iterative, and a loop in the data can't hang it.
    pub fn root_source(&self, component: Cid) -> Cid {
        let mut current = component;
        let mut steps = 0usize;
        while let Some(s) = self.comp(current).source {
            current = s.component;
            steps += 1;
            if steps > self.comps.len() {
                break;
            }
        }
        current
    }

    /// `_findPatchedAndUnpatchedOutputsFromComponent` (F61).
    pub fn patch_points(&self, component: Cid) -> Vec<PatchPoint> {
        let mut out = Vec::new();
        self.collect_patch_points(component, &mut out);
        out
    }

    /// Appends the points and returns how many it added.
    pub fn collect_patch_points(&self, component: Cid, out: &mut Vec<PatchPoint>) -> usize {
        let c = self.comp(component);
        if c.kind == CompKind::Output {
            return 0;
        }
        let before = out.len();
        for output in 0..c.output_count as i32 {
            let mut any_destination = false;
            let start = out.len();
            for d in c.destinations.iter().copied().filter(|d| self.comp(*d).source.is_some_and(|s| s.output == output)) {
                any_destination = true;
                self.collect_patch_points(d, out);
            }
            if !any_destination {
                out.push(PatchPoint { component, output, patched: false });
            } else if out.len() == start {
                // Every child has no outputs, which in practice means they are controller outputs.
                out.push(PatchPoint { component, output, patched: true });
            }
        }
        out.len() - before
    }

    /// The same counts as `patch_points`, without building the list: `(patched, unpatched)`.
    pub fn count_patch_points(&self, component: Cid) -> (u32, u32) {
        let c = self.comp(component);
        if c.kind == CompKind::Output {
            return (0, 0);
        }
        let (mut patched, mut unpatched) = (0, 0);
        for output in 0..c.output_count as i32 {
            let mut any_destination = false;
            let mut yielded = false;
            for d in c.destinations.iter().copied().filter(|d| self.comp(*d).source.is_some_and(|s| s.output == output)) {
                any_destination = true;
                let (p, u) = self.count_patch_points(d);
                if p + u > 0 {
                    yielded = true;
                }
                patched += p;
                unpatched += u;
            }
            if !any_destination {
                unpatched += 1;
            } else if !yielded {
                patched += 1;
            }
        }
        (patched, unpatched)
    }

    /// `_findFiltersThatDescendFromElement` (F62): every filter under `component`, depth first.
    pub fn reachable_filters(&self, component: Cid) -> Vec<Cid> {
        let mut out = Vec::new();
        let mut stack: Vec<Cid> = self.comp(component).destinations.iter().rev().copied().collect();
        while let Some(c) = stack.pop() {
            let comp = self.comp(c);
            if comp.kind == CompKind::Filter {
                out.push(c);
            }
            stack.extend(comp.destinations.iter().rev().copied());
        }
        out
    }

    /// `_findComponentsOfTypeInTreeFromComponent(child, IDataFlowComponent)`: the component and all below it, depth first.
    pub fn subtree(&self, component: Cid) -> Vec<Cid> {
        let mut out = Vec::new();
        let mut stack = vec![component];
        while let Some(c) = stack.pop() {
            out.push(c);
            stack.extend(self.comp(c).destinations.iter().rev().copied());
        }
        out
    }

    /// `GetOrphanedFilters` then `RemoveFilterChain`: filters with no source, and the filters hanging off each, children first.
    pub fn orphaned_filter_chains(&self) -> Vec<Cid> {
        let mut out = Vec::new();
        for (cid, c) in self.components() {
            if c.kind == CompKind::Filter && c.source.is_none() {
                self.collect_filter_chain(cid, &mut out);
            }
        }
        out
    }

    fn collect_filter_chain(&self, filter: Cid, out: &mut Vec<Cid>) {
        for d in self.destinations(filter) {
            if self.comp(*d).kind == CompKind::Filter {
                self.collect_filter_chain(*d, out);
            }
        }
        out.push(filter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct G {
        f: DataFlow,
    }

    impl G {
        fn new() -> G {
            G { f: DataFlow::new() }
        }
        fn element(&mut self) -> Cid {
            self.f.add_component(Uuid::new_v4(), CompKind::Element, 1)
        }
        fn filter(&mut self, outputs: u32) -> Cid {
            self.f.add_component(Uuid::new_v4(), CompKind::Filter, outputs)
        }
        fn output(&mut self) -> Cid {
            self.f.add_component(Uuid::new_v4(), CompKind::Output, 0)
        }
        fn link(&mut self, from: Cid, output: i32, to: Cid) {
            assert!(self.f.set_source(to, SourceRef { component: from, output }));
        }
        fn counts(&self, c: Cid) -> (usize, usize) {
            let points = self.f.patch_points(c);
            let patched = points.iter().filter(|p| p.patched).count();
            assert_eq!(self.f.count_patch_points(c), (patched as u32, (points.len() - patched) as u32));
            (patched, points.len() - patched)
        }
    }

    #[test]
    fn element_alone_is_one_unpatched_point() {
        let mut g = G::new();
        let e = g.element();
        assert_eq!(g.counts(e), (0, 1));
    }

    #[test]
    fn element_to_one_output_is_one_patched_point() {
        let mut g = G::new();
        let (e, o) = (g.element(), g.output());
        g.link(e, 0, o);
        assert_eq!(g.counts(e), (1, 0));
    }

    #[test]
    fn element_to_two_outputs_on_one_element_output_is_one_patched_point() {
        let mut g = G::new();
        let (e, a, b) = (g.element(), g.output(), g.output());
        g.link(e, 0, a);
        g.link(e, 0, b);
        assert_eq!(g.counts(e), (1, 0));
    }

    #[test]
    fn colour_breakdown_with_two_of_three_connected() {
        let mut g = G::new();
        let (e, f, r, gr) = (g.element(), g.filter(3), g.output(), g.output());
        g.link(e, 0, f);
        g.link(f, 0, r);
        g.link(f, 2, gr);
        assert_eq!(g.counts(e), (2, 1));
        let points = g.f.patch_points(e);
        assert_eq!(points.iter().map(|p| (p.output, p.patched)).collect::<Vec<_>>(), [(0, true), (1, false), (2, true)]);
        assert!(points.iter().all(|p| p.component == f));
    }

    #[test]
    fn a_filter_beside_a_direct_output_replaces_it_as_the_point() {
        // The direct output adds none, because the filter yielded something.
        let mut g = G::new();
        let (e, f, o) = (g.element(), g.filter(3), g.output());
        g.link(e, 0, f);
        g.link(e, 0, o);
        assert_eq!(g.counts(e), (0, 3));
    }

    #[test]
    fn a_filter_chain_is_one_patched_point() {
        let mut g = G::new();
        let (e, a, b, o) = (g.element(), g.filter(1), g.filter(1), g.output());
        g.link(e, 0, a);
        g.link(a, 0, b);
        g.link(b, 0, o);
        assert_eq!(g.counts(e), (1, 0));
    }

    #[test]
    fn a_filter_with_no_outputs_counts_as_the_end_of_the_line() {
        let mut g = G::new();
        let (e, f) = (g.element(), g.filter(0));
        g.link(e, 0, f);
        assert_eq!(g.counts(e), (1, 0));
    }

    #[test]
    fn a_loop_is_refused_and_changes_nothing() {
        let mut g = G::new();
        let (a, b) = (g.filter(1), g.filter(1));
        g.link(a, 0, b);
        assert!(!g.f.set_source(a, SourceRef { component: b, output: 0 }));
        assert_eq!(g.f.source(a), None);
        assert!(!g.f.set_source(a, SourceRef { component: a, output: 0 }));
    }

    #[test]
    fn a_new_source_replaces_the_old_one() {
        let mut g = G::new();
        let (a, b, o) = (g.element(), g.element(), g.output());
        g.link(a, 0, o);
        g.link(b, 0, o);
        assert!(g.f.destinations(a).is_empty());
        assert_eq!(g.f.destinations(b), [o]);
    }

    #[test]
    fn removing_a_component_detaches_both_ways() {
        let mut g = G::new();
        let (e, f, o) = (g.element(), g.filter(1), g.output());
        g.link(e, 0, f);
        g.link(f, 0, o);
        g.f.remove_component(f);
        assert!(g.f.destinations(e).is_empty());
        assert_eq!(g.f.source(o), None);
        assert!(!g.f.is_live(f));
        assert_eq!(g.counts(e), (0, 1));
    }

    #[test]
    fn shrinking_a_filter_drops_what_was_on_the_removed_outputs() {
        let mut g = G::new();
        let (e, f, a, b) = (g.element(), g.filter(3), g.output(), g.output());
        g.link(e, 0, f);
        g.link(f, 0, a);
        g.link(f, 2, b);
        g.f.set_output_count(f, 2);
        assert_eq!(g.f.source(b), None);
        assert_eq!(g.f.destinations(f), [a]);
    }

    #[test]
    fn patches_come_back_in_file_order() {
        let mut flow = DataFlow::new();
        let ids: Vec<Uuid> = (1..=6).map(Uuid::from_u128).collect();
        flow.add_component(ids[0], CompKind::Element, 1);
        flow.add_component(ids[1], CompKind::Element, 1);
        flow.add_component(ids[2], CompKind::Output, 0);
        flow.add_component(ids[3], CompKind::Output, 0);
        flow.add_component(ids[4], CompKind::Filter, 2);
        flow.add_component(ids[5], CompKind::Output, 0);
        let patch = |c: Uuid, s: Uuid, output: i32| Patch { component_id: c, source_id: Some(s), output };
        let patches = vec![patch(ids[4], ids[0], 0), patch(ids[2], ids[1], 0), patch(ids[3], ids[4], 1), patch(ids[5], ids[4], 0)];
        assert_eq!(flow.apply_patches(&patches), vec![]);
        assert_eq!(flow.to_patches(), patches);
    }

    #[test]
    fn bad_patches_are_reported_and_skipped() {
        let mut flow = DataFlow::new();
        let (e, o) = (Uuid::from_u128(1), Uuid::from_u128(2));
        flow.add_component(e, CompKind::Element, 1);
        let oc = flow.add_component(o, CompKind::Output, 0);
        let missing = Uuid::from_u128(9);
        let warnings = flow.apply_patches(&[
            Patch { component_id: missing, source_id: Some(e), output: 0 },
            Patch { component_id: o, source_id: Some(missing), output: 0 },
            Patch { component_id: o, source_id: None, output: 0 },
            Patch { component_id: o, source_id: Some(e), output: 3 },
        ]);
        assert_eq!(warnings.len(), 4);
        assert_eq!(flow.source(oc), None);
    }

    #[test]
    fn orphans_include_the_chain_under_them_but_not_rooted_filters() {
        let mut g = G::new();
        let (e, rooted, orphan, child, o) = (g.element(), g.filter(1), g.filter(1), g.filter(1), g.output());
        g.link(e, 0, rooted);
        g.link(orphan, 0, child);
        g.link(child, 0, o);
        assert_eq!(g.f.orphaned_filter_chains(), [child, orphan]);
    }

    #[test]
    fn walks() {
        let mut g = G::new();
        let (e, f, a, b) = (g.element(), g.filter(2), g.output(), g.output());
        g.link(e, 0, f);
        g.link(f, 0, a);
        g.link(f, 1, b);
        assert_eq!(g.f.reachable_filters(e), [f]);
        assert_eq!(g.f.subtree(f), [f, a, b]);
        assert_eq!(g.f.root_source(b), e);
        assert_eq!(g.f.destinations_of_output(f, 1).collect::<Vec<_>>(), [b]);
    }
}
