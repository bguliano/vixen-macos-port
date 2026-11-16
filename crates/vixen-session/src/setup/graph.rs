//! The graphical patching view without its canvas: what `SetupPatchingGraphical` shows, how it lays things out and what its gestures do.
//! A native rebuild from the behaviour, not a port of NShape. The window gets flat arrays and draws them.

use super::flow::{Cid, CompKind, SourceRef};
use super::tree::{Nid, RowPath};
use super::controllers::OutputSelection;
use super::{FilterEntry, SetupSession};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use vixen_output::ids::{filter_descriptor, FILTER_DESCRIPTORS};
use vixen_output::FilterConfig;

const SHAPE_Y_TOP: i32 = 10;
const SHAPE_MAX_WIDTH: i32 = 160;
const SHAPE_MIN_WIDTH: i32 = 32;
const SHAPE_DEFAULT_HEIGHT: i32 = 20;
const SHAPE_MIN_HEIGHT: i32 = 10;
const SHAPE_VERTICAL_SPACING: i32 = 6;
const CHILD_WIDTH_REDUCTION: i32 = 10;
const GROUP_HEADER_HEIGHT: i32 = 20;
const GROUP_FOOTER_HEIGHT: i32 = 6;
/// `FakeShapeConnection`: how far a dashed stub runs from its port.
pub const STUB_LENGTH: f32 = 40.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    ElementGroup,
    ElementLeaf,
    Filter,
    Controller,
    Output,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A shape on the canvas. `x` and `y` are its top left, unlike NShape's centre: upstream's columns then land where its own comments say they are (F115).
#[derive(Clone, Debug, PartialEq)]
pub struct GraphShape {
    /// Stable across rebuilds, so selection and collapse state follow it.
    pub id: u64,
    pub kind: ShapeKind,
    pub title: String,
    pub rect: Rect,
    /// F113's stacking: root element and controller shapes 1, nested 2 and up, filters 10.
    pub z: u8,
    /// Inside a collapsed group. Not drawn, though lines still run to where it would be.
    pub hidden: bool,
    pub inputs: u32,
    pub outputs: u32,
    /// `None` for a group and a controller, which aren't data flow components.
    pub component: Option<Cid>,
    /// Index of the shape it nests in.
    pub parent: Option<u32>,
    /// Whether it can collapse: group and controller shapes with something inside.
    pub expandable: bool,
    pub collapsed: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LineKind {
    Real,
    /// The input's source isn't displayed.
    SourceStub,
    /// An output's destinations aren't displayed.
    DestinationStub,
}

/// A line between ports, resolved to coordinates so a redraw does no lookups.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GraphLine {
    pub id: u64,
    pub kind: LineKind,
    /// Shape indexes. `None` at a stub's free end.
    pub from_shape: Option<u32>,
    pub from_port: u32,
    pub to_shape: Option<u32>,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    /// What Delete cuts: this component's source. `None` on a destination stub, which has nothing to cut.
    pub destination: Option<Cid>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphScene {
    pub shapes: Vec<GraphShape>,
    pub lines: Vec<GraphLine>,
    pub content_width: f32,
    pub content_height: f32,
}

/// What the view remembers between rebuilds. Never saved (F115, F121).
#[derive(Default, Debug)]
struct GraphInner {
    revision: u64,
    collapsed: HashSet<u64>,
    /// Filters moved by hand, or laid out when something else collapsed. Dropped at the next element selection change or resize.
    placed: HashMap<Cid, Rect>,
    /// Added or pasted filters stay on the canvas until the window closes, where upstream loses them at the next selection (F121).
    floating: Vec<Cid>,
    last_elements: Vec<Nid>,
    last_width: i32,
    last_tree_revision: u64,
    /// The filters the last scene showed, and where. Between layouts they stay, where they were: upstream lays filters out only on an element change or a resize (F115).
    last_filter_rects: HashMap<Cid, Rect>,
    last_filters: Vec<Cid>,
}

#[derive(Debug, Default)]
pub struct GraphState {
    inner: Mutex<GraphInner>,
}

/// Port position as a fraction of a shape's height (F114).
#[inline]
pub fn port_fraction(index: u32, count: u32) -> f32 {
    if count <= 1 {
        0.5
    } else {
        0.1 + 0.8 * index as f32 / (count - 1) as f32
    }
}

/// `ZoomToFitAll`, as a fraction: never above 1, never below 0.2. Upstream takes one percent off below 100 for its scrollbars, which aren't in the way here.
pub fn zoom_to_fit(view_width: f32, view_height: f32, content_width: f32, content_height: f32) -> f32 {
    if content_width <= 0.0 || content_height <= 0.0 {
        return 1.0;
    }
    (view_width / content_width).min(view_height / content_height).clamp(0.2, 1.0)
}

/// The Zoom In and Zoom Out buttons, in whole percent: `level * 1.1` or `level * 0.9`, truncated, within 1..4000.
pub fn zoom_step(level: i32, zoom_in: bool) -> i32 {
    let next = if zoom_in { (f64::from(level) * 1.1) as i32 } else { (f64::from(level) * 0.9) as i32 };
    // Truncating can stick at a small level, where 1.1 times 5 is still 5.
    let next = if zoom_in && next == level { level + 1 } else { next };
    next.clamp(1, 4000)
}

fn mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x0000_0100_0000_01B3)
}

fn shape_id(kind: u64, parts: impl IntoIterator<Item = u64>) -> u64 {
    let mut h = mix(0xcbf2_9ce4_8422_2325, kind);
    for p in parts {
        h = mix(h, p);
    }
    h
}

struct Builder<'a> {
    session: &'a SetupSession,
    inner: &'a GraphInner,
    shapes: Vec<GraphShape>,
    /// Layout children per shape, in order.
    children: Vec<Vec<u32>>,
    by_component: HashMap<Cid, Vec<u32>>,
}

impl Builder<'_> {
    fn push(&mut self, mut shape: GraphShape) -> u32 {
        let index = self.shapes.len() as u32;
        shape.collapsed = self.inner.collapsed.contains(&shape.id);
        if let Some(c) = shape.component {
            self.by_component.entry(c).or_default().push(index);
        }
        self.shapes.push(shape);
        self.children.push(Vec::new());
        index
    }

    /// `_MakeElementNodeShape`: a shape per node, with nested shapes for its whole subtree.
    fn element_shape(&mut self, node: Nid, path: &mut RowPath, parent: Option<u32>, z: u8) -> u32 {
        path.push(node);
        let tree = &self.session.tree;
        let n = tree.node(node);
        let is_group = !n.children.is_empty();
        let component = if is_group { None } else { n.element_id.and_then(|e| self.session.flow.find(&e)) };
        let outputs = component.map_or(0, |c| self.session.flow.comp(c).output_count);
        let id = shape_id(1, path.iter().map(|p| u64::from(p.0)));
        let children = if is_group { n.children.clone() } else { Vec::new() };
        let index = self.push(GraphShape {
            id,
            kind: if is_group { ShapeKind::ElementGroup } else { ShapeKind::ElementLeaf },
            title: n.name.clone(),
            rect: Rect::default(),
            z,
            hidden: false,
            inputs: 0,
            outputs,
            component,
            parent,
            expandable: is_group,
            collapsed: false,
        });
        for child in children {
            let c = self.element_shape(child, path, Some(index), z.saturating_add(1));
            self.children[index as usize].push(c);
        }
        path.pop();
        index
    }

    fn filter_shape(&mut self, entry: &FilterEntry) -> u32 {
        self.push(GraphShape {
            id: shape_id(2, [u64::from(entry.cid.0)]),
            kind: ShapeKind::Filter,
            title: entry.name(),
            rect: Rect::default(),
            z: 10,
            hidden: false,
            inputs: 1,
            outputs: self.session.flow.comp(entry.cid).output_count,
            component: Some(entry.cid),
            parent: None,
            expandable: false,
            collapsed: false,
        })
    }

    fn controller_shape(&mut self, slot: u32, ranges: &[std::ops::Range<u32>]) -> u32 {
        let c = &self.session.controllers[slot as usize];
        let index = self.push(GraphShape {
            id: shape_id(3, [u64::from(slot)]),
            kind: ShapeKind::Controller,
            title: c.name.clone(),
            rect: Rect::default(),
            z: 1,
            hidden: false,
            inputs: 0,
            outputs: 0,
            component: None,
            parent: None,
            expandable: true,
            collapsed: false,
        });
        for range in ranges {
            for i in range.start..range.end.min(c.outputs.len() as u32) {
                let output = &c.outputs[i as usize];
                let title = if output.name.is_empty() { format!("{} [{}]", c.name, i + 1) } else { output.name.clone() };
                let o = self.push(GraphShape {
                    id: shape_id(4, [u64::from(slot), u64::from(output.cid.0)]),
                    kind: ShapeKind::Output,
                    title,
                    rect: Rect::default(),
                    z: 2,
                    hidden: false,
                    inputs: 1,
                    outputs: 0,
                    component: Some(output.cid),
                    parent: Some(index),
                    expandable: false,
                    collapsed: false,
                });
                self.children[index as usize].push(o);
            }
        }
        index
    }

    /// `_ResizeAndPositionNestingShape`.
    fn nest(&mut self, shape: u32, width: i32, x: i32, y: i32, visible: bool) -> i32 {
        let width = width.max(SHAPE_MIN_WIDTH);
        let expanded = !self.shapes[shape as usize].collapsed;
        let children = std::mem::take(&mut self.children[shape as usize]);
        let height;
        if visible && expanded && !children.is_empty() {
            let mut cur_y = y + GROUP_HEADER_HEIGHT;
            for &c in &children {
                // Children share the parent's centre, so they sit centred in it.
                let child_width = (width - CHILD_WIDTH_REDUCTION).max(SHAPE_MIN_WIDTH);
                let h = self.nest(c, width - CHILD_WIDTH_REDUCTION, x + (width - child_width) / 2, cur_y, true);
                cur_y += h + SHAPE_VERTICAL_SPACING;
            }
            height = (cur_y - SHAPE_VERTICAL_SPACING + GROUP_FOOTER_HEIGHT) - y;
        } else {
            height = SHAPE_DEFAULT_HEIGHT;
            for &c in &children {
                self.nest(c, width, x, y, false);
            }
        }
        self.children[shape as usize] = children;
        let s = &mut self.shapes[shape as usize];
        s.rect = Rect { x: x as f32, y: y as f32, w: width as f32, h: height as f32 };
        s.hidden = !visible;
        height
    }
}

impl SetupSession {
    pub(crate) fn bump_graph(&mut self) {
        self.graph.inner.get_mut().revision += 1;
    }

    /// Bumps whenever a scene built now would differ from one built before.
    pub fn graph_revision(&self) -> u64 {
        self.graph.inner.lock().revision
    }

    /// `_allNodeParentsAreInSet`, with hash sets and a memo where upstream re-lists and rescans on each recursion. The root isn't a parent in the set.
    fn all_parents_in_set(&self, node: Nid, set: &HashSet<Nid>, memo: &mut HashMap<Nid, bool>) -> bool {
        if let Some(known) = memo.get(&node) {
            return *known;
        }
        let n = self.tree.node(node);
        let result = !n.at_root
            && !n.parents.is_empty()
            && n.parents.iter().all(|p| set.contains(p) || self.all_parents_in_set(*p, set, memo));
        memo.insert(node, result);
        result
    }

    /// The display for these selections at this view width: F112, F113, F114, F115, F116.
    pub fn graph_scene(&self, selection: &[String], outputs: &OutputSelection, width: i32) -> GraphScene {
        let nodes: Vec<Nid> = {
            let mut seen = HashSet::new();
            self.nodes_of(selection).into_iter().filter(|n| seen.insert(*n)).collect()
        };
        let mut guard = self.graph.inner.lock();
        // F115: filters are laid out again only on an element selection change, an element edit or a resize. Until then each stays where it is, even one a patch change would
        // put elsewhere, and so does one that lost its source and would no longer be listed (F120).
        if guard.last_elements != nodes || guard.last_width != width || guard.last_tree_revision != self.tree_revision() {
            guard.placed.clear();
            guard.last_filters.clear();
            guard.last_elements.clone_from(&nodes);
            guard.last_width = width;
            guard.last_tree_revision = self.tree_revision();
        } else {
            let inner = &mut *guard;
            for (c, r) in &inner.last_filter_rects {
                inner.placed.entry(*c).or_insert(*r);
            }
        }
        guard.floating.retain(|c| self.flow.is_live(*c));
        let mut filters = self.displayed_filters(&nodes, &guard.floating);
        if !guard.last_filters.is_empty() {
            let shown: HashSet<Cid> = filters.iter().copied().collect();
            filters.extend(guard.last_filters.iter().copied().filter(|c| !shown.contains(c) && self.flow.is_live(*c) && self.filter_for(*c).is_some_and(|f| f.alive)));
        }
        guard.last_filters.clone_from(&filters);

        let mut b = Builder {
            session: self,
            inner: &guard,
            shapes: Vec::new(),
            children: Vec::new(),
            by_component: HashMap::new(),
        };

        // F112: a root shape per selected node, except one that will appear nested inside what's selected.
        let set: HashSet<Nid> = nodes.iter().copied().collect();
        let mut memo = HashMap::new();
        let mut element_roots: Vec<u32> = Vec::new();
        let mut path: RowPath = Vec::new();
        for &n in &nodes {
            if !self.all_parents_in_set(n, &set, &mut memo) {
                element_roots.push(b.element_shape(n, &mut path, None, 1));
            }
        }

        // Stacks (F115). `Width * 20 / 100` in integers, as upstream has it.
        let element_width = width * 20 / 100;
        let element_x = width * 5 / 100;
        let mut y = SHAPE_Y_TOP;
        for r in &element_roots {
            y += b.nest(*r, element_width, element_x, y, true) + SHAPE_VERTICAL_SPACING;
        }

        let filter_shapes: Vec<u32> = filters.iter().map(|c| b.filter_shape(self.filter_for(*c).expect("a filter"))).collect();

        let mut output_shapes: Vec<u32> = Vec::new();
        let channel_width = width * 20 / 100;
        let channel_x = width * 75 / 100;
        let mut y = SHAPE_Y_TOP;
        for (slot, ranges) in &outputs.items {
            if !self.controllers.get(*slot as usize).is_some_and(|c| c.alive) {
                continue;
            }
            let c = b.controller_shape(*slot, ranges);
            y += b.nest(c, channel_width, channel_x, y, true) + SHAPE_VERTICAL_SPACING;
            output_shapes.extend_from_slice(&b.children[c as usize]);
        }

        self.layout_filters(&mut b, &filter_shapes, width);

        let Builder { mut shapes, by_component, .. } = b;
        // Filters a gesture has placed keep their place. Everything laid out from them, above, ignored it, as upstream's stored positions do.
        let mut filter_rects = HashMap::with_capacity(filter_shapes.len());
        for &i in &filter_shapes {
            let shape = &mut shapes[i as usize];
            let cid = shape.component.expect("a filter has a component");
            if let Some(rect) = guard.placed.get(&cid) {
                shape.rect = *rect;
            }
            filter_rects.insert(cid, shape.rect);
        }
        guard.last_filter_rects = filter_rects;

        let lines = self.graph_lines(&shapes, &by_component, &output_shapes, &filter_shapes);

        let (mut right, mut bottom) = (0.0f32, 0.0f32);
        for s in shapes.iter().filter(|s| !s.hidden) {
            right = right.max(s.rect.x + s.rect.w);
            bottom = bottom.max(s.rect.y + s.rect.h);
        }
        for l in &lines {
            right = right.max(l.x1).max(l.x2);
        }
        GraphScene { shapes, lines, content_width: right + f32::from(SHAPE_Y_TOP as i16), content_height: bottom + f32::from(SHAPE_Y_TOP as i16) }
    }

    /// F112: every filter reachable from the selection's leaves, each list post-order as `_findComponentsOfTypeInTreeFromComponent` gives it, then any floating ones.
    fn displayed_filters(&self, nodes: &[Nid], floating: &[Cid]) -> Vec<Cid> {
        let mut seen: HashSet<Cid> = HashSet::new();
        let mut out = Vec::new();
        for leaf in self.tree.distinct_leaves(nodes.iter().copied()) {
            let Some(c) = self.tree.node(leaf).element_id.and_then(|e| self.flow.find(&e)) else { continue };
            self.collect_filters_post_order(c, &mut seen, &mut out);
        }
        for c in floating {
            if self.filter_for(*c).is_some() && seen.insert(*c) {
                out.push(*c);
            }
        }
        out
    }

    fn collect_filters_post_order(&self, c: Cid, seen: &mut HashSet<Cid>, out: &mut Vec<Cid>) {
        let comp = self.flow.comp(c);
        for d in &comp.destinations {
            self.collect_filters_post_order(*d, seen, out);
        }
        if comp.kind == CompKind::Filter && seen.insert(c) {
            out.push(c);
        }
    }

    /// `_calculateFilterDepthFromSource`: from 1, adding one per filter step, stopping at a displayed element. 0 if the chain ends with no element. Also says which element.
    fn filter_depth(&self, filter: Cid, by_component: &HashMap<Cid, Vec<u32>>, shapes: &[GraphShape]) -> (i32, Option<Cid>) {
        let mut depth = 1;
        let mut source = self.flow.source(filter).map(|s| s.component);
        while let Some(current) = source {
            if let Some(list) = by_component.get(&current) {
                if matches!(shapes[list[0] as usize].kind, ShapeKind::ElementLeaf | ShapeKind::ElementGroup) {
                    return (depth, Some(current));
                }
            }
            match self.flow.source(current) {
                Some(up) => {
                    source = Some(up.component);
                    depth += 1;
                }
                None => return (0, None),
            }
        }
        (0, None)
    }

    /// `_ResizeAndPositionFilterShapes`.
    fn layout_filters(&self, b: &mut Builder<'_>, filters: &[u32], width: i32) {
        let filter_width = width * 30 / 100;
        let filter_x = width * 35 / 100;

        let mut depths: Vec<(i32, Option<Cid>)> = Vec::with_capacity(filters.len());
        let mut max_depth = 0;
        let mut element_max: HashMap<Cid, i32> = HashMap::new();
        for &f in filters {
            let cid = b.shapes[f as usize].component.expect("a filter has a component");
            let (depth, element) = self.filter_depth(cid, &b.by_component, &b.shapes);
            depths.push((depth, element));
            max_depth = max_depth.max(depth);
            if let Some(e) = element {
                let entry = element_max.entry(e).or_insert(depth);
                *entry = (*entry).max(depth);
            }
        }

        // `_dataFlowComponentToChildFilterShapes`.
        let mut sibling_counts: HashMap<Cid, i32> = HashMap::new();
        for &f in filters {
            let cid = b.shapes[f as usize].component.expect("a filter has a component");
            if let Some(s) = self.flow.source(cid) {
                *sibling_counts.entry(s.component).or_insert(0) += 1;
            }
        }

        // Each depth, parents first, because a filter's place comes from its source's.
        let mut laid_out: HashMap<Cid, i32> = HashMap::new();
        for depth in 1..=max_depth {
            for (&f, &(d, element)) in filters.iter().zip(&depths) {
                if d != depth {
                    continue;
                }
                let cid = b.shapes[f as usize].component.expect("a filter has a component");
                let Some(source) = self.flow.source(cid).map(|s| s.component) else { continue };
                let total = sibling_counts.get(&source).copied().unwrap_or(1);
                let so_far = laid_out.entry(source).or_insert(0);
                let index = *so_far;
                *so_far += 1;

                let source_rect = b.by_component.get(&source).map(|list| b.shapes[list[0] as usize].rect);
                let (height, top) = match source_rect {
                    Some(r) => {
                        let available = r.h as i32 - SHAPE_VERTICAL_SPACING * (total - 1);
                        let height = (available / total).max(SHAPE_MIN_HEIGHT);
                        // Upstream's `Y` is a centre on both sides of this sum.
                        let centre = r.y as i32 + r.h as i32 / 2 + index * (height + SHAPE_VERTICAL_SPACING);
                        (height, centre - height / 2)
                    }
                    None => (SHAPE_DEFAULT_HEIGHT, SHAPE_Y_TOP + index * (SHAPE_DEFAULT_HEIGHT + SHAPE_VERTICAL_SPACING)),
                };

                let columns = element.and_then(|e| element_max.get(&e).copied()).unwrap_or(depth).max(1);
                let proportion = (depth - 1) as f32 / columns as f32;
                let column_width = filter_width / columns;
                let mut shape_width = column_width.min(SHAPE_MAX_WIDTH).max(SHAPE_MIN_WIDTH);
                if shape_width < SHAPE_MAX_WIDTH && shape_width > SHAPE_MIN_WIDTH && column_width - shape_width < 20 {
                    shape_width = column_width - 20;
                }
                let offset = (column_width - shape_width) / 2;
                let x = filter_x + (proportion * filter_width as f32) as i32 + offset;
                b.shapes[f as usize].rect = Rect { x: x as f32, y: top as f32, w: shape_width as f32, h: height as f32 };
            }
        }

        // Free-floating filters in a grid: at the top, as upstream has them, unless connected filters are there already, which they would cover. Upstream only ever shows both
        // together through its depth bug (F115); here an added or pasted filter outlives a selection change (F121), so it goes under them instead.
        let floating: Vec<u32> = filters.iter().zip(&depths).filter(|(_, d)| d.0 == 0).map(|(f, _)| *f).collect();
        if !floating.is_empty() {
            let below = filters
                .iter()
                .zip(&depths)
                .filter(|(_, d)| d.0 > 0)
                .map(|(f, _)| {
                    let r = b.shapes[*f as usize].rect;
                    (r.y + r.h) as i32 + SHAPE_VERTICAL_SPACING
                })
                .max();
            let top = below.unwrap_or(SHAPE_Y_TOP);
            let per_row = (filter_width / (SHAPE_MIN_WIDTH + SHAPE_VERTICAL_SPACING)).max(1);
            let columns = (floating.len() as i32).min(per_row);
            let column_width = filter_width / columns;
            let shape_width = column_width.min(SHAPE_MAX_WIDTH).max(SHAPE_MIN_WIDTH);
            let offset = (column_width - shape_width) / 2;
            for (i, &f) in floating.iter().enumerate() {
                let i = i as i32;
                let (row, column) = (i / columns, i % columns);
                b.shapes[f as usize].rect = Rect {
                    x: (filter_x + column * column_width + offset) as f32,
                    y: (top + row * (SHAPE_DEFAULT_HEIGHT + SHAPE_VERTICAL_SPACING)) as f32,
                    w: shape_width as f32,
                    h: SHAPE_DEFAULT_HEIGHT as f32,
                };
            }
        }
    }

    /// `UpdateConnections` (F116). Every leaf gets the destination pass, where upstream only runs it on root shapes: the only effect is a stub that tells the truth.
    fn graph_lines(&self, shapes: &[GraphShape], by_component: &HashMap<Cid, Vec<u32>>, outputs: &[u32], filters: &[u32]) -> Vec<GraphLine> {
        let mut lines = Vec::new();
        let input_point = |s: &GraphShape| (s.rect.x, s.rect.y + s.rect.h * port_fraction(0, 1));
        let output_point = |s: &GraphShape, i: u32| (s.rect.x + s.rect.w, s.rect.y + s.rect.h * port_fraction(i, s.outputs));

        for &index in outputs.iter().chain(filters) {
            let shape = &shapes[index as usize];
            let cid = shape.component.expect("has a component");
            let Some(source) = self.flow.source(cid) else { continue };
            let (x2, y2) = input_point(shape);
            match by_component.get(&source.component) {
                Some(list) => {
                    for &from in list {
                        let from_shape = &shapes[from as usize];
                        let (x1, y1) = output_point(from_shape, source.output as u32);
                        lines.push(GraphLine {
                            id: shape_id(5, [from_shape.id, source.output as u64, shape.id]),
                            kind: LineKind::Real,
                            from_shape: Some(from),
                            from_port: source.output as u32,
                            to_shape: Some(index),
                            x1,
                            y1,
                            x2,
                            y2,
                            destination: Some(cid),
                        });
                    }
                }
                None => lines.push(GraphLine {
                    id: shape_id(6, [shape.id]),
                    kind: LineKind::SourceStub,
                    from_shape: None,
                    from_port: 0,
                    to_shape: Some(index),
                    x1: x2 - STUB_LENGTH,
                    y1: y2,
                    x2,
                    y2,
                    destination: Some(cid),
                }),
            }
        }

        for (index, shape) in shapes.iter().enumerate() {
            if !matches!(shape.kind, ShapeKind::ElementLeaf | ShapeKind::Filter) {
                continue;
            }
            let Some(cid) = shape.component else { continue };
            for i in 0..shape.outputs {
                let mut destinations = self.flow.destinations_of_output(cid, i as i32).peekable();
                if destinations.peek().is_none() {
                    continue;
                }
                if !destinations.any(|d| by_component.contains_key(&d)) {
                    let (x1, y1) = output_point(shape, i);
                    lines.push(GraphLine {
                        id: shape_id(7, [shape.id, u64::from(i)]),
                        kind: LineKind::DestinationStub,
                        from_shape: Some(index as u32),
                        from_port: i,
                        to_shape: None,
                        x1,
                        y1,
                        x2: x1 + STUB_LENGTH,
                        y2: y1,
                        destination: None,
                    });
                }
            }
        }
        lines
    }

    // ---- Gestures -------------------------------------------------------------------------------------------------------

    /// F124: collapses or expands a group or controller shape. Only that column re-lays out, so every filter stays where it is until the next selection change.
    pub fn graph_toggle_collapsed(&mut self, shape: u64) {
        let inner = self.graph.inner.get_mut();
        if !inner.collapsed.remove(&shape) {
            inner.collapsed.insert(shape);
        }
        inner.revision += 1;
    }

    /// F118, on mouse up. Nothing about data flow changes.
    pub fn graph_place_filters(&mut self, placed: &[(Cid, Rect)]) {
        let inner = self.graph.inner.get_mut();
        for (c, r) in placed {
            inner.placed.insert(*c, *r);
        }
        inner.revision += 1;
    }

    /// F119's refusals while hovering: the source itself, anything that would form a loop, and anything with no input (groups, controllers, elements).
    pub fn graph_connect_candidate(&self, source: Cid, target: Cid) -> bool {
        if source == target || !self.flow.is_live(source) || !self.flow.is_live(target) {
            return false;
        }
        if self.flow.comp(target).kind == CompKind::Element {
            return false;
        }
        !self.flow.would_cycle(target, source)
    }

    /// F119, on release: replaces any old source. False if refused.
    pub fn graph_connect(&mut self, target: Cid, source: Cid, output: u32) -> bool {
        if !self.graph_connect_candidate(source, target) {
            return false;
        }
        if !self.flow.set_source(target, SourceRef { component: source, output: output as i32 }) {
            return false;
        }
        self.patching_changed();
        self.bump_graph();
        true
    }

    /// F120: lines (by the component they feed) and filters. One update for the whole gesture, where upstream raises one per port.
    pub fn graph_delete(&mut self, destinations: &[Cid], filters: &[Cid]) {
        let mut changed = false;
        for d in destinations {
            if self.flow.is_live(*d) && self.flow.source(*d).is_some() {
                self.flow.reset_source(*d);
                changed = true;
            }
        }
        for f in filters {
            if self.flow.is_live(*f) && self.flow.comp(*f).kind == CompKind::Filter {
                self.remove_filter(*f);
                changed = true;
            }
        }
        if changed {
            self.patching_changed();
            self.bump_graph();
        }
    }

    /// `Filters.AddFilter`: a new instance, unconnected, with its own ids. `None` for a module type this port doesn't have. The config's ids don't matter: they get replaced.
    pub fn add_filter_instance(&mut self, type_id: Uuid, config: Option<FilterConfig>) -> Option<Cid> {
        let instance_id = Uuid::new_v4();
        let mut config = config.or_else(|| FilterConfig::new_default(&type_id, instance_id))?;
        *config.ids_mut() = vixen_output::ModuleIds::new(type_id, instance_id);
        let count = config.output_names().len() as u32;
        let cid = self.flow.add_component(instance_id, CompKind::Filter, count);
        self.filter_lookup.insert(cid, self.filters.len());
        self.filters.push(FilterEntry { cid, type_id, instance_id, config: Some(config), changed: true, alive: true });
        self.dirty.filters = true;
        self.bump_graph();
        Some(cid)
    }

    fn new_filter(&mut self, type_id: Uuid, config: Option<FilterConfig>) -> Option<Cid> {
        let cid = self.add_filter_instance(type_id, config)?;
        self.graph.inner.get_mut().floating.push(cid);
        Some(cid)
    }

    /// F121: unconnected, and it stays on the canvas until the window closes. `rect` is where to put it.
    pub fn graph_add_filter(&mut self, type_id: Uuid, rect: Rect) -> Option<Cid> {
        let cid = self.new_filter(type_id, None)?;
        self.graph.inner.get_mut().placed.insert(cid, rect);
        Some(cid)
    }

    /// F122, F123: `copies` of each source, each a new instance of the same type with a copy of its data, unconnected, spaced downward from `at`.
    /// A filter cut since the copy still pastes, because its type and data live on in the old entry.
    pub fn graph_paste_filters(&mut self, sources: &[Cid], copies: u32, at: (f32, f32)) -> Vec<Cid> {
        let mut out = Vec::new();
        let (x, mut y) = at;
        for _ in 0..copies {
            for s in sources {
                let Some(entry) = self.filter_for(*s) else { continue };
                let (type_id, config) = (entry.type_id, entry.config.clone());
                if let Some(cid) = self.new_filter(type_id, config) {
                    let rect = Rect { x, y, w: SHAPE_MAX_WIDTH as f32, h: SHAPE_DEFAULT_HEIGHT as f32 };
                    self.graph.inner.get_mut().placed.insert(cid, rect);
                    y += (SHAPE_DEFAULT_HEIGHT + SHAPE_VERTICAL_SPACING) as f32;
                    out.push(cid);
                }
            }
        }
        out
    }

    /// The Add Filter list, sorted by name (F148).
    pub fn filter_types() -> Vec<(Uuid, &'static str)> {
        let mut list: Vec<(Uuid, &'static str)> = FILTER_DESCRIPTORS.iter().map(|d| (d.type_id, d.type_name)).collect();
        list.sort_by_key(|t| t.1);
        list
    }

    /// `HasSetup` for a filter: all eight modules have one. `None` if the component isn't a filter, or is a type this port doesn't have.
    pub fn filter_type_name(&self, cid: Cid) -> Option<&'static str> {
        filter_descriptor(&self.filter_for(cid)?.type_id).map(|d| d.type_name)
    }

    /// For the filter dialogs: replaces a filter's data. Its output count can change, which drops what was on removed outputs (F124).
    pub fn set_filter_config(&mut self, cid: Cid, config: FilterConfig) -> bool {
        let Some(i) = self.filter_lookup.get(&cid).copied() else { return false };
        let count = config.output_names().len() as u32;
        let entry = &mut self.filters[i];
        entry.config = Some(config);
        entry.changed = true;
        self.dirty.filters = true;
        self.flow.set_output_count(cid, count);
        self.patching_changed();
        self.bump_graph();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::patching::PatchOptions;
    use super::super::test_support::*;
    use super::*;

    fn row(s: &SetupSession, name: &str) -> String {
        s.row_path_of(s.tree.live_nodes().find(|(_, n)| n.name == name).unwrap().0)
    }

    fn element(s: &SetupSession, name: &str) -> Cid {
        let node = s.tree.live_nodes().find(|(_, x)| x.name == name).unwrap().1;
        s.flow.find(&node.element_id.unwrap()).unwrap()
    }

    fn none() -> OutputSelection {
        OutputSelection::default()
    }

    fn titled<'a>(scene: &'a GraphScene, title: &str) -> Vec<&'a GraphShape> {
        scene.shapes.iter().filter(|s| s.title == title).collect()
    }

    /// `G` over leaves `L1`, `L2`.
    fn with_group() -> (SetupSession, String) {
        let mut s = session(&["G", "L1", "L2"], 4);
        let (g, l1, l2) = (row(&s, "G"), row(&s, "L1"), row(&s, "L2"));
        s.drop_rows(&[l1, l2], Some(&g), None, false);
        (s, g)
    }

    #[test]
    fn a_child_with_its_parent_appears_once_nested() {
        let (s, g) = with_group();
        let child = format!("{g}/{}", row(&s, "L1").rsplit('/').next().unwrap());
        let scene = s.graph_scene(&[g.clone(), child], &none(), 1000);
        assert_eq!(titled(&scene, "L1").len(), 1);
        assert!(titled(&scene, "L1")[0].parent.is_some());
        assert_eq!(titled(&scene, "G").len(), 1);
    }

    #[test]
    fn a_child_with_an_unselected_parent_appears_as_a_root_too() {
        let (mut s, g) = with_group();
        s.add_element(None, "Other");
        let other = row(&s, "Other");
        let l1_node = s.tree.live_nodes().find(|(_, n)| n.name == "L1").unwrap().0;
        let l1_under_g = format!("{g}/{}", l1_node.0);
        s.copy_rows(&[l1_under_g.clone()]);
        s.paste_links(Some(&other));
        // L1 has parents G and Other. Only G is selected.
        let scene = s.graph_scene(&[g, l1_under_g], &none(), 1000);
        let shapes = titled(&scene, "L1");
        assert_eq!(shapes.len(), 2);
        assert!(shapes.iter().any(|x| x.parent.is_none()));
        assert_ne!(shapes[0].id, shapes[1].id);
    }

    #[test]
    fn stacks_follow_the_numbers() {
        let mut s = session(&["Leaf"], 0);
        let leaf = row(&s, "Leaf");
        let scene = s.graph_scene(&[leaf.clone()], &none(), 1000);
        let r = titled(&scene, "Leaf")[0].rect;
        assert_eq!((r.x, r.y, r.w, r.h), (50.0, 10.0, 200.0, 20.0));

        s.add_element(None, "G");
        let g = row(&s, "G");
        s.add_element(Some(&g), "A");
        s.add_element(Some(&g), "B");
        let scene = s.graph_scene(&[g], &none(), 1000);
        let group = titled(&scene, "G")[0];
        assert_eq!(group.rect.h, 20.0 + 2.0 * 20.0 + 6.0 + 6.0);
        let a = titled(&scene, "A")[0].rect;
        assert_eq!(a.w, 190.0);
        assert_eq!(a.x, 55.0, "centred in the parent");
        assert_eq!(a.y, 30.0);
        assert_eq!(titled(&scene, "B")[0].rect.y, 56.0);
        // The stack's width floor.
        let tiny = s.graph_scene(&[row(&s, "Leaf")], &none(), 100);
        assert_eq!(titled(&tiny, "Leaf")[0].rect.w, 32.0);
    }

    #[test]
    fn ports_and_counts() {
        assert_eq!(port_fraction(0, 1), 0.5);
        assert!((port_fraction(0, 3) - 0.1).abs() < 1e-6);
        assert!((port_fraction(1, 3) - 0.5).abs() < 1e-6);
        assert!((port_fraction(2, 3) - 0.9).abs() < 1e-6);

        let mut s = session(&["L"], 1);
        let e = element(&s, "L");
        add_breakdown(&mut s, e, 3);
        s.patching_changed();
        let scene = s.graph_scene(&[row(&s, "L")], &s.all_outputs_selection(), 1000);
        let leaf = titled(&scene, "L")[0];
        assert_eq!((leaf.inputs, leaf.outputs, leaf.kind), (0, 1, ShapeKind::ElementLeaf));
        let filter = scene.shapes.iter().find(|x| x.kind == ShapeKind::Filter).unwrap();
        assert_eq!((filter.inputs, filter.outputs, filter.title.as_str()), (1, 3, "Color Breakdown"));
        let controller = titled(&scene, "Main")[0];
        assert_eq!((controller.inputs, controller.outputs), (0, 0));
        let output = titled(&scene, "Output 1")[0];
        assert_eq!((output.inputs, output.outputs, output.kind), (1, 0, ShapeKind::Output));
    }

    #[test]
    fn group_shapes_have_no_ports_and_controllers_sit_at_seventy_five_percent() {
        let (s, g) = with_group();
        let scene = s.graph_scene(&[g], &s.all_outputs_selection(), 1000);
        let group = titled(&scene, "G")[0];
        assert_eq!((group.inputs, group.outputs, group.kind), (0, 0, ShapeKind::ElementGroup));
        assert_eq!(group.component, None);
        let controller = titled(&scene, "Main")[0];
        assert_eq!((controller.rect.x, controller.rect.w), (750.0, 200.0));
        let child = titled(&scene, "Output 1")[0];
        assert_eq!(child.parent, Some(scene.shapes.iter().position(|x| x.title == "Main").unwrap() as u32));
        assert_eq!(child.z, 2);
    }

    #[test]
    fn output_names_fall_back_to_the_controller_and_position() {
        let mut s = session(&[], 1);
        s.controllers[0].outputs[0].name.clear();
        let scene = s.graph_scene(&[], &s.all_outputs_selection(), 1000);
        assert_eq!(titled(&scene, "Main [1]").len(), 1);
    }

    #[test]
    fn filter_depth_and_columns() {
        let mut s = session(&["E", "F"], 0);
        let e = element(&s, "E");
        let a = add_breakdown(&mut s, e, 1);
        let b = add_breakdown(&mut s, a, 1);
        s.patching_changed();
        let scene = s.graph_scene(&[row(&s, "E")], &none(), 1000);
        let filters: Vec<&GraphShape> = scene.shapes.iter().filter(|x| x.kind == ShapeKind::Filter).collect();
        assert_eq!(filters.len(), 2);
        let (fa, fb) = (
            filters.iter().find(|f| f.component == Some(a)).unwrap(),
            filters.iter().find(|f| f.component == Some(b)).unwrap(),
        );
        // Two columns of 150 across 300 from x = 350. Width 150 is under the maximum and leaves no 20 margin, so it becomes 130.
        assert_eq!((fa.rect.x, fa.rect.w), (360.0, 130.0));
        assert_eq!((fb.rect.x, fb.rect.w), (510.0, 130.0));
    }

    #[test]
    fn siblings_share_the_sources_height() {
        let mut s = session(&["G"], 0);
        let g = row(&s, "G");
        for n in ["A", "B", "C"] {
            s.add_element(Some(&g), n);
        }
        s.add_element(None, "E");
        let e = element(&s, "E");
        add_breakdown(&mut s, e, 1);
        add_breakdown(&mut s, e, 1);
        s.patching_changed();
        let scene = s.graph_scene(&[row(&s, "E")], &none(), 1000);
        let filters: Vec<&GraphShape> = scene.shapes.iter().filter(|x| x.kind == ShapeKind::Filter).collect();
        // The source is 20 high, so (20 - 6) / 2 = 7, raised to the minimum of 10.
        assert!(filters.iter().all(|f| f.rect.h == 10.0));
        // Centres: the source's centre (20), then 16 further on.
        assert_eq!(filters[0].rect.y, 15.0);
        assert_eq!(filters[1].rect.y, 31.0);
    }

    #[test]
    fn floating_filters_lay_out_in_a_grid_and_survive_selection_changes() {
        let mut s = session(&["A"], 0);
        let f1 = s.graph_add_filter(vixen_output::ids::DIMMING_CURVE, Rect { x: 5.0, y: 6.0, w: 160.0, h: 20.0 }).unwrap();
        let f2 = s.graph_add_filter(vixen_output::ids::DIMMING_CURVE, Rect { x: 5.0, y: 6.0, w: 160.0, h: 20.0 }).unwrap();
        let a = row(&s, "A");
        // A gesture's position holds while nothing has changed since the add.
        let scene = s.graph_scene(&[], &none(), 1000);
        // The first scene since the add has a new width to record, so the grid applies.
        let filters: Vec<&GraphShape> = scene.shapes.iter().filter(|x| x.kind == ShapeKind::Filter).collect();
        assert_eq!(filters.len(), 2);
        // perRow = 300 / 38 = 7, columns = 2, column width 150, so width 150 centred.
        assert_eq!((filters[0].rect.x, filters[0].rect.y, filters[0].rect.w), (350.0, 10.0, 150.0));
        assert_eq!(filters[1].rect.x, 500.0);

        let placed = Rect { x: 700.0, y: 300.0, w: 150.0, h: 20.0 };
        s.graph_place_filters(&[(f1, placed)]);
        let scene = s.graph_scene(&[], &none(), 1000);
        assert_eq!(scene.shapes.iter().find(|x| x.component == Some(f1)).unwrap().rect, placed);

        // The next element selection re-lays it out, but it stays on the canvas.
        let scene = s.graph_scene(&[a], &none(), 1000);
        assert_eq!(scene.shapes.iter().filter(|x| x.kind == ShapeKind::Filter).count(), 2);
        assert_ne!(scene.shapes.iter().find(|x| x.component == Some(f1)).unwrap().rect, placed);

        s.graph_delete(&[], &[f2]);
        let scene = s.graph_scene(&[], &none(), 1000);
        assert_eq!(scene.shapes.iter().filter(|x| x.kind == ShapeKind::Filter).count(), 1);
    }

    #[test]
    fn stub_rules() {
        let mut s = session(&["A"], 2);
        let a = row(&s, "A");
        s.patch_elements(&[a.clone()], &OutputSelection::single(0, 0..1), &PatchOptions::default(), false);

        // The output's source isn't displayed: a stub to its left.
        let scene = s.graph_scene(&[], &s.all_outputs_selection(), 1000);
        let stubs: Vec<&GraphLine> = scene.lines.iter().filter(|l| l.kind == LineKind::SourceStub).collect();
        assert_eq!(stubs.len(), 1);
        assert_eq!(stubs[0].x2 - stubs[0].x1, STUB_LENGTH);
        assert!(stubs[0].destination.is_some());

        // The element's destination isn't displayed: a stub to its right, which Delete has nothing to cut on.
        let scene = s.graph_scene(&[a.clone()], &none(), 1000);
        let stubs: Vec<&GraphLine> = scene.lines.iter().filter(|l| l.kind == LineKind::DestinationStub).collect();
        assert_eq!(stubs.len(), 1);
        assert_eq!(stubs[0].destination, None);

        // Both shown: a real line replaces both stubs.
        let scene = s.graph_scene(&[a], &s.all_outputs_selection(), 1000);
        assert_eq!(scene.lines.len(), 1);
        assert_eq!(scene.lines[0].kind, LineKind::Real);
        let leaf = titled(&scene, "A")[0].rect;
        assert_eq!((scene.lines[0].x1, scene.lines[0].y1), (leaf.x + leaf.w, leaf.y + leaf.h / 2.0));
    }

    #[test]
    fn a_nested_leaf_gets_a_stub_too() {
        // Upstream only runs the destination pass on root shapes.
        let (mut s, g) = with_group();
        let l1 = row(&s, "L1");
        s.patch_elements(&[l1], &OutputSelection::single(0, 0..1), &PatchOptions::default(), false);
        let scene = s.graph_scene(&[g], &none(), 1000);
        assert_eq!(scene.lines.iter().filter(|l| l.kind == LineKind::DestinationStub).count(), 1);
    }

    #[test]
    fn an_element_shown_twice_gets_two_lines_to_one_input() {
        let (mut s, g) = with_group();
        s.add_element(None, "Other");
        let other = row(&s, "Other");
        let l1_node = s.tree.live_nodes().find(|(_, n)| n.name == "L1").unwrap().0;
        s.copy_rows(&[format!("{g}/{}", l1_node.0)]);
        s.paste_links(Some(&other));
        let l1 = format!("{g}/{}", l1_node.0);
        s.patch_elements(&[l1], &OutputSelection::single(0, 0..1), &PatchOptions::default(), false);
        let scene = s.graph_scene(&[g, other], &OutputSelection::single(0, 0..1), 1000);
        assert_eq!(scene.lines.iter().filter(|l| l.kind == LineKind::Real).count(), 2);
    }

    #[test]
    fn connect_refusals_and_replacing_a_source() {
        let mut s = session(&["A", "B"], 1);
        let (a, b) = (element(&s, "A"), element(&s, "B"));
        let f = add_breakdown(&mut s, a, 1);
        let g = add_breakdown(&mut s, f, 1);
        let output = s.controllers[0].outputs[0].cid;
        assert!(!s.graph_connect_candidate(f, f), "itself");
        assert!(!s.graph_connect_candidate(g, f), "a loop");
        assert!(!s.graph_connect_candidate(a, b), "an element has no input");
        assert!(s.graph_connect_candidate(a, output));
        assert!(s.graph_connect(output, a, 0));
        assert!(s.graph_connect(output, b, 0));
        assert_eq!(s.flow.source(output).unwrap().component, b);
        assert!(s.flow.destinations(a).iter().all(|d| *d != output));
        assert!(!s.graph_connect(f, g, 0));
    }

    #[test]
    fn delete_cuts_lines_and_removes_filters_both_ways() {
        let mut s = session(&["A"], 1);
        let a = element(&s, "A");
        let f = add_breakdown(&mut s, a, 1);
        let output = s.controllers[0].outputs[0].cid;
        s.graph_connect(output, f, 0);
        s.graph_delete(&[output], &[]);
        assert_eq!(s.flow.source(output), None);
        s.graph_connect(output, f, 0);
        s.graph_delete(&[], &[f]);
        assert_eq!(s.flow.source(output), None);
        assert!(s.flow.destinations(a).is_empty());
        assert!(!s.flow.is_live(f));
    }

    #[test]
    fn collapsing_hides_children_and_leaves_filters_alone() {
        let (mut s, g) = with_group();
        let l1 = s.tree.live_nodes().find(|(_, n)| n.name == "L1").unwrap().1.element_id.unwrap();
        let l1c = s.flow.find(&l1).unwrap();
        let f = add_breakdown(&mut s, l1c, 1);
        s.patching_changed();
        let scene = s.graph_scene(&[g.clone()], &none(), 1000);
        let before = scene.shapes.iter().find(|x| x.component == Some(f)).unwrap().rect;
        let group = titled(&scene, "G")[0];
        assert!(group.expandable && !group.collapsed);

        s.graph_toggle_collapsed(group.id);
        let scene = s.graph_scene(&[g.clone()], &none(), 1000);
        let group = titled(&scene, "G")[0];
        assert!(group.collapsed);
        assert_eq!(group.rect.h, 20.0);
        let leaf = titled(&scene, "L1")[0];
        assert!(leaf.hidden);
        assert_eq!(leaf.rect.y, group.rect.y);
        // Lines still run to where the hidden child is, which is the collapsed group.
        let line = scene.lines.iter().find(|l| l.destination == Some(f)).unwrap();
        assert_eq!(line.from_shape, Some(scene.shapes.iter().position(|x| x.title == "L1").unwrap() as u32));
        assert_eq!(scene.shapes.iter().find(|x| x.component == Some(f)).unwrap().rect, before);

        s.graph_toggle_collapsed(group.id);
        let scene = s.graph_scene(&[g], &none(), 1000);
        assert!(!titled(&scene, "L1")[0].hidden);
    }

    #[test]
    fn paste_makes_new_unconnected_copies_even_of_a_cut_filter() {
        let mut s = session(&["A"], 0);
        let a = element(&s, "A");
        let f = add_breakdown(&mut s, a, 2);
        s.graph_delete(&[], &[f]);
        let copies = s.graph_paste_filters(&[f], 3, (100.0, 200.0));
        assert_eq!(copies.len(), 3);
        for c in &copies {
            assert_eq!(s.flow.comp(*c).output_count, 2);
            assert_eq!(s.flow.source(*c), None);
            assert_ne!(*c, f);
        }
        let entry = s.filter_for(copies[0]).unwrap();
        assert_eq!(entry.config.as_ref().unwrap().ids().instance_id, entry.instance_id);
        let scene_rects: Vec<Rect> = {
            let inner = s.graph.inner.lock();
            copies.iter().map(|c| inner.placed[c]).collect()
        };
        assert_eq!(scene_rects[0].y, 200.0);
        assert_eq!(scene_rects[1].y, 226.0);
    }

    #[test]
    fn a_setup_changing_output_count_drops_lines_on_removed_outputs() {
        let mut s = session(&["A"], 2);
        let a = element(&s, "A");
        let f = add_breakdown(&mut s, a, 2);
        let (o0, o1) = (s.controllers[0].outputs[0].cid, s.controllers[0].outputs[1].cid);
        s.graph_connect(o0, f, 0);
        s.graph_connect(o1, f, 1);
        let mut config = s.filter_for(f).unwrap().config.clone().unwrap();
        if let FilterConfig::ColorBreakdown(c) = &mut config {
            c.breakdown_items.pop();
        }
        assert!(s.set_filter_config(f, config));
        assert_eq!(s.flow.source(o1), None);
        assert!(s.flow.source(o0).is_some());
        assert_eq!(s.flow.comp(f).output_count, 1);
    }

    #[test]
    fn patching_on_the_canvas_leaves_the_filters_where_they_are() {
        let (mut s, g) = with_group();
        let element = |s: &SetupSession, name: &str| {
            let id = s.tree.live_nodes().find(|(_, n)| n.name == name).unwrap().1.element_id.unwrap();
            s.flow.find(&id).unwrap()
        };
        let (l1, l2) = (element(&s, "L1"), element(&s, "L2"));
        let f1 = add_breakdown(&mut s, l1, 2);
        let f2 = add_breakdown(&mut s, l2, 2);
        s.patching_changed();
        let outputs = none();
        let rects = |scene: &GraphScene| -> HashMap<Cid, Rect> { scene.shapes.iter().filter_map(|x| x.component.map(|c| (c, x.rect))).collect() };
        let before = rects(&s.graph_scene(std::slice::from_ref(&g), &outputs, 1000));

        // The second filter now hangs off the first, which a new layout would show one column along and half as high.
        assert!(s.graph_connect(f2, f1, 0));
        let after = rects(&s.graph_scene(std::slice::from_ref(&g), &outputs, 1000));
        assert_eq!(before[&f1], after[&f1]);
        assert_eq!(before[&f2], after[&f2]);

        // Its line deleted, the second filter has no source and nothing lists it, but it stays on the canvas where it was until the next layout.
        s.graph_delete(&[f2], &[]);
        let after = rects(&s.graph_scene(std::slice::from_ref(&g), &outputs, 1000));
        assert_eq!(before[&f2], after[&f2]);

        // A new width is a new layout, and it sees the patch as it is now.
        assert!(s.graph_connect(f2, f1, 0));
        let wider = rects(&s.graph_scene(std::slice::from_ref(&g), &outputs, 1200));
        let laid_out = rects(&s.graph_scene(std::slice::from_ref(&g), &outputs, 1000));
        assert_ne!(wider[&f2], before[&f2]);
        assert_ne!(laid_out[&f2], before[&f2], "depth 2 goes in the next column");

        // A filter left with no source goes at the next layout.
        s.graph_delete(&[f2], &[]);
        let other = vec![row(&s, "L1")];
        let gone = rects(&s.graph_scene(&other, &outputs, 1000));
        assert!(!gone.contains_key(&f2));

        // A deleted filter goes at once.
        s.graph_delete(&[], &[f1]);
        let gone = rects(&s.graph_scene(&other, &outputs, 1000));
        assert!(!gone.contains_key(&f1));
    }

    #[test]
    fn floating_filters_go_under_connected_ones() {
        let (mut s, g) = with_group();
        let id = s.tree.live_nodes().find(|(_, n)| n.name == "L1").unwrap().1.element_id.unwrap();
        let l1 = s.flow.find(&id).unwrap();
        let connected = add_breakdown(&mut s, l1, 1);
        s.patching_changed();
        let added = s.graph_add_filter(vixen_output::ids::COLOR_BREAKDOWN, Rect { x: 300.0, y: 300.0, w: 160.0, h: 20.0 }).unwrap();
        // A selection change lays everything out again.
        let scene = s.graph_scene(&[g.clone()], &none(), 1000);
        let rect = |c: Cid| scene.shapes.iter().find(|x| x.component == Some(c)).unwrap().rect;
        let (a, b) = (rect(connected), rect(added));
        assert!(b.y >= a.y + a.h, "{a:?} {b:?}");

        // With nothing connected showing, the grid is at the top.
        let scene = s.graph_scene(&[], &none(), 1000);
        let alone = scene.shapes.iter().find(|x| x.component == Some(added)).unwrap().rect;
        assert_eq!(alone.y, 10.0);
    }

    #[test]
    fn zoom_formulas() {
        assert_eq!(zoom_to_fit(500.0, 500.0, 1000.0, 1000.0), 0.5);
        assert_eq!(zoom_to_fit(500.0, 500.0, 100.0, 100.0), 1.0);
        assert_eq!(zoom_to_fit(500.0, 500.0, 100000.0, 100.0), 0.2);
        assert_eq!(zoom_step(100, true), 110);
        assert_eq!(zoom_step(100, false), 90);
        assert_eq!(zoom_step(1, false), 1);
        assert_eq!(zoom_step(4000, true), 4000);
    }

    #[test]
    fn shape_ids_are_stable_between_scenes() {
        let (s, g) = with_group();
        let first = s.graph_scene(&[g.clone()], &s.all_outputs_selection(), 1000);
        let second = s.graph_scene(&[g], &s.all_outputs_selection(), 800);
        let ids = |scene: &GraphScene| scene.shapes.iter().map(|x| x.id).collect::<Vec<_>>();
        assert_eq!(ids(&first), ids(&second));
    }

    #[test]
    fn filter_list_is_sorted_and_complete() {
        let list = SetupSession::filter_types();
        assert_eq!(list.len(), 8);
        assert!(list.windows(2).all(|w| w[0].1 <= w[1].1));
    }

    #[test]
    #[ignore = "a time budget, for release builds"]
    fn a_scene_of_ten_thousand_shapes_is_quick() {
        let names: Vec<String> = (0..5000).map(|i| format!("L{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut s = session(&refs, 5000);
        let rows: Vec<String> = s.tree.roots.iter().map(|n| s.row_path_of(*n)).collect();
        s.patch_elements(&rows, &s.all_outputs_selection(), &PatchOptions::default(), false);
        let outputs = s.all_outputs_selection();
        let start = std::time::Instant::now();
        let scene = s.graph_scene(&rows, &outputs, 1000);
        let spent = start.elapsed();
        assert_eq!(scene.shapes.len(), 10_001);
        assert_eq!(scene.lines.len(), 5000);
        assert!(spent.as_millis() < 100, "{spent:?}");
    }
}
