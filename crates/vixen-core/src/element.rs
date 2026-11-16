//! Port of Vixen.Core/Sys/ElementNode + GroupNode and the element property modules that effects read
//! (Color, Orientation, Location, Order).
//!
//! The C# uses an object graph with parent pointers. Here nodes live in an arena and are addressed by
//! `NodeId`, so a whole tree can be shared immutably across render threads through an `Arc`.

use crate::Color;
use std::collections::{HashMap, HashSet};
use uuid::{uuid, Uuid};

/// Index of a node in the arena.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct NodeId(pub u32);

/// `ElementColorType`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum ElementColorType {
    SingleColor,
    MultipleDiscreteColors,
    #[default]
    FullColor,
}

/// `Orientation` property values.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Orientation {
    Horizontal,
    #[default]
    Vertical,
}

pub mod property_ids {
    use super::*;
    pub const ORDER: Uuid = uuid!("47d4a43b-5407-44bd-8be1-4974b148504f");
    pub const GRID: Uuid = uuid!("de3b4ec9-4e7f-4690-b0fe-3780b17a85ec");
    pub const FACE: Uuid = uuid!("eda73851-8350-4d3f-9df2-0d3b34e70bb6");
    pub const STATE: Uuid = uuid!("35b893c3-d10c-4359-82bc-929c0762e809");
    pub const INTELLIGENT_FIXTURE: Uuid = uuid!("fdd74c85-1779-4521-b559-0dd5ea71054e");
    pub const COLOR: Uuid = uuid!("bff34727-6b88-4f87-82b7-68424498c725");
    pub const ORIENTATION: Uuid = uuid!("db0fc9fc-cdd5-4f88-bd66-5e4cf08feae4");
    pub const LOCATION: Uuid = uuid!("3fb53423-dd2a-4719-b3e3-19aa6f062f62");
}

/// Typed payload of an element property. `Unknown` keeps data for property modules this port does not
/// interpret yet, so a load and save never loses it.
#[derive(Clone, PartialEq, Debug)]
pub enum PropertyData {
    Color { color_type: ElementColorType, single_color: Color, color_set_name: Option<String> },
    Orientation(Orientation),
    Location { x: i32, y: i32, z: i32 },
    Order { order: i32 },
    Unknown { raw_xml: String },
}

#[derive(Clone, PartialEq, Debug)]
pub struct PropertyInstance {
    pub type_id: Uuid,
    pub instance_id: Uuid,
    pub data: PropertyData,
}

#[derive(Clone, Debug)]
pub struct ElementNode {
    pub id: Uuid,
    pub name: String,
    /// `ElementNode.Element.Id`, the key that intents and patching use.
    pub element_id: Option<Uuid>,
    pub children: Vec<NodeId>,
    pub parents: Vec<NodeId>,
    pub properties: Vec<PropertyInstance>,
    pub tags: Vec<Uuid>,
}

/// All element nodes in the system, plus the shared colour sets (`ColorStaticData.ColorSets`).
#[derive(Clone, Debug, Default)]
pub struct ElementTree {
    nodes: Vec<ElementNode>,
    by_id: HashMap<Uuid, NodeId>,
    by_element: HashMap<Uuid, NodeId>,
    pub roots: Vec<NodeId>,
    pub color_sets: HashMap<String, Vec<Color>>,
}

impl ElementTree {
    pub fn new() -> ElementTree {
        let mut color_sets = HashMap::new();
        color_sets.insert(
            "RGBW".to_string(),
            vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0), Color::rgb(0, 0, 255), Color::rgb(255, 255, 255)],
        );
        color_sets.insert("RGB".to_string(), vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0), Color::rgb(0, 0, 255)]);
        ElementTree { nodes: Vec::new(), by_id: HashMap::new(), by_element: HashMap::new(), roots: Vec::new(), color_sets }
    }

    /// Adds a node with a new element, as a root. Use `add_child` to place it in the tree.
    pub fn add_node(&mut self, id: Uuid, name: &str, element_id: Option<Uuid>) -> NodeId {
        let nid = NodeId(self.nodes.len() as u32);
        self.nodes.push(ElementNode {
            id,
            name: name.to_string(),
            element_id,
            children: Vec::new(),
            parents: Vec::new(),
            properties: Vec::new(),
            tags: Vec::new(),
        });
        self.by_id.insert(id, nid);
        if let Some(e) = element_id {
            self.by_element.insert(e, nid);
        }
        self.roots.push(nid);
        nid
    }

    /// Convenience for tests and templates: a root node with a fresh element.
    pub fn add_element(&mut self, name: &str) -> NodeId {
        self.add_node(Uuid::new_v4(), name, Some(Uuid::new_v4()))
    }

    /// Convenience for tests and templates: a group of `count` leaf elements named "`name`-N".
    pub fn add_string(&mut self, name: &str, count: usize) -> NodeId {
        let group = self.add_node(Uuid::new_v4(), name, None);
        for i in 0..count {
            let leaf = self.add_element(&format!("{name}-{}", i + 1));
            self.add_child(group, leaf);
        }
        group
    }

    /// `GroupNode.AddChild`: a node can have several parents. Adding one removes it from the roots.
    pub fn add_child(&mut self, parent: NodeId, child: NodeId) {
        if !self.nodes[parent.0 as usize].children.contains(&child) {
            self.nodes[parent.0 as usize].children.push(child);
            let c = &mut self.nodes[child.0 as usize];
            if !c.parents.contains(&parent) {
                c.parents.push(parent);
            }
            self.roots.retain(|r| *r != child);
        }
    }

    /// `GroupNode.RemoveChild`. A node with no parents left returns to the roots.
    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) {
        self.nodes[parent.0 as usize].children.retain(|c| *c != child);
        let c = &mut self.nodes[child.0 as usize];
        c.parents.retain(|p| *p != parent);
        if c.parents.is_empty() && !self.roots.contains(&child) {
            self.roots.push(child);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[inline]
    pub fn node(&self, id: NodeId) -> &ElementNode {
        &self.nodes[id.0 as usize]
    }

    #[inline]
    pub fn node_mut(&mut self, id: NodeId) -> &mut ElementNode {
        &mut self.nodes[id.0 as usize]
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = (NodeId, &ElementNode)> {
        self.nodes.iter().enumerate().map(|(i, n)| (NodeId(i as u32), n))
    }

    pub fn find(&self, id: &Uuid) -> Option<NodeId> {
        self.by_id.get(id).copied()
    }

    pub fn find_by_element(&self, element: &Uuid) -> Option<NodeId> {
        self.by_element.get(element).copied()
    }

    /// Every element id in the system.
    pub fn element_ids(&self) -> impl Iterator<Item = Uuid> + '_ {
        self.nodes.iter().filter_map(|n| n.element_id)
    }

    #[inline]
    pub fn is_leaf(&self, id: NodeId) -> bool {
        self.node(id).children.is_empty()
    }

    /// `GetLeafEnumerator()`. A node reached through two parents appears twice, as in the C#.
    pub fn leaves(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.collect_leaves(id, &mut out);
        out
    }

    fn collect_leaves(&self, id: NodeId, out: &mut Vec<NodeId>) {
        let n = self.node(id);
        if n.children.is_empty() {
            out.push(id);
        } else {
            for c in &n.children {
                self.collect_leaves(*c, out);
            }
        }
    }

    /// `GetLeafEnumerator().Distinct()`.
    pub fn distinct_leaves(&self, id: NodeId) -> Vec<NodeId> {
        distinct(self.leaves(id))
    }

    /// `GetNonLeafEnumerator()`.
    pub fn non_leaves(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.collect_non_leaves(id, &mut out);
        out
    }

    fn collect_non_leaves(&self, id: NodeId, out: &mut Vec<NodeId>) {
        let n = self.node(id);
        if !n.children.is_empty() {
            out.push(id);
            for c in &n.children {
                self.collect_non_leaves(*c, out);
            }
        }
    }

    /// `GetNodeEnumerator()`: this node and all descendants, depth first.
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = vec![id];
        for c in &self.node(id).children {
            out.extend(self.descendants(*c));
        }
        out
    }

    /// `GetElementEnumerator()`, as element ids.
    pub fn elements(&self, id: NodeId) -> Vec<Uuid> {
        self.leaves(id).into_iter().filter_map(|l| self.node(l).element_id).collect()
    }

    /// `node.Distinct()`: distinct element ids in first-seen order.
    pub fn distinct_elements(&self, id: NodeId) -> Vec<Uuid> {
        distinct(self.elements(id))
    }

    /// `node.Count()`: the number of elements, counting a repeated leaf each time.
    pub fn element_count(&self, id: NodeId) -> usize {
        self.elements(id).len()
    }

    /// `GetMaxChildDepth()`.
    pub fn max_child_depth(&self, id: NodeId) -> i32 {
        let n = self.node(id);
        if n.children.is_empty() {
            1
        } else {
            n.children.iter().map(|c| self.max_child_depth(*c)).max().unwrap_or(0) + 1
        }
    }

    /// `GetAllParentNodes()`.
    pub fn all_parents(&self, id: NodeId) -> Vec<NodeId> {
        let mut out: Vec<NodeId> = self.node(id).parents.clone();
        for p in self.node(id).parents.clone() {
            out.extend(self.all_parents(p));
        }
        out
    }

    /// `PixelEffectBase.FindLeafParents`: parents of all leaves that are also inside the target lineage,
    /// in first-seen order.
    pub fn leaf_parents(&self, targets: &[NodeId]) -> Vec<NodeId> {
        let mut non_leaf: HashSet<NodeId> = HashSet::new();
        let mut parents: Vec<NodeId> = Vec::new();
        for &t in targets {
            non_leaf.extend(self.non_leaves(t));
            for leaf in self.leaves(t) {
                parents.extend(self.node(leaf).parents.iter().copied());
            }
        }
        distinct(parents).into_iter().filter(|p| non_leaf.contains(p)).collect()
    }

    /// `GetNodesAtEffectDepth`.
    pub fn nodes_at_depth(&self, node: NodeId, depth: i32) -> Vec<NodeId> {
        let mut render: Vec<NodeId>;
        if depth == 0 {
            render = self.distinct_leaves(node);
        } else {
            render = vec![node];
            for _ in 0..depth {
                let mut next = Vec::new();
                for n in &render {
                    next.extend(self.node(*n).children.iter().copied());
                }
                render = distinct(next);
            }
        }
        if render.is_empty() {
            render = self.distinct_leaves(node);
        }
        render
    }

    pub fn property(&self, id: NodeId, type_id: &Uuid) -> Option<&PropertyInstance> {
        self.node(id).properties.iter().find(|p| &p.type_id == type_id)
    }

    /// `ColorModule.getColorTypeForElementNode`.
    pub fn color_type(&self, id: NodeId) -> ElementColorType {
        for p in &self.node(id).properties {
            if let PropertyData::Color { color_type, .. } = &p.data {
                return *color_type;
            }
        }
        ElementColorType::FullColor
    }

    /// `ColorModule.isElementNodeDiscreteColored`.
    pub fn is_discrete(&self, id: NodeId) -> bool {
        matches!(self.color_type(id), ElementColorType::MultipleDiscreteColors | ElementColorType::SingleColor)
    }

    /// `ColorModule.isElementNodeTreeDiscreteColored`.
    pub fn is_tree_discrete(&self, id: NodeId) -> bool {
        self.leaves(id).into_iter().any(|l| self.is_discrete(l))
    }

    /// `ColorModule.getValidColorsForElementNode`: empty means full colour.
    pub fn valid_colors(&self, id: NodeId, include_children: bool) -> Vec<Color> {
        let mut out = Vec::new();
        self.collect_valid_colors(id, include_children, &mut out);
        out
    }

    fn collect_valid_colors(&self, id: NodeId, include_children: bool, out: &mut Vec<Color>) {
        for p in &self.node(id).properties {
            if let PropertyData::Color { color_type, single_color, color_set_name } = &p.data {
                match color_type {
                    ElementColorType::FullColor => {}
                    ElementColorType::SingleColor => push_unique(out, *single_color),
                    ElementColorType::MultipleDiscreteColors => {
                        if let Some(set) = color_set_name.as_ref().and_then(|n| self.color_sets.get(n)) {
                            for c in set {
                                push_unique(out, *c);
                            }
                        }
                    }
                }
                break;
            }
        }
        if include_children {
            for c in &self.node(id).children {
                self.collect_valid_colors(*c, true, out);
            }
        }
    }

    /// `BaseEffect.GetValidColors` for a set of target nodes.
    pub fn valid_colors_for(&self, targets: &[NodeId]) -> Vec<Color> {
        let mut out = Vec::new();
        for t in targets {
            self.collect_valid_colors(*t, true, &mut out);
        }
        out
    }

    /// `OrientationModule.GetOrientationForElement`, when the node has the property.
    pub fn orientation(&self, id: NodeId) -> Option<Orientation> {
        self.node(id).properties.iter().find_map(|p| match p.data {
            PropertyData::Orientation(o) => Some(o),
            _ => None,
        })
    }

    /// The node's Location property, if it has one.
    pub fn location_opt(&self, id: NodeId) -> Option<(i32, i32)> {
        self.node(id).properties.iter().find_map(|p| match p.data {
            PropertyData::Location { x, y, .. } => Some((x, y)),
            _ => None,
        })
    }

    /// `LocationModule.GetPositionForElement`. Nodes with no location are at the origin.
    pub fn location(&self, id: NodeId) -> (i32, i32) {
        self.location_opt(id).unwrap_or((0, 0))
    }
}

fn distinct<T: Copy + Eq + std::hash::Hash>(items: Vec<T>) -> Vec<T> {
    let mut seen = HashSet::with_capacity(items.len());
    items.into_iter().filter(|i| seen.insert(*i)).collect()
}

fn push_unique(v: &mut Vec<Color>, c: Color) {
    if !v.contains(&c) {
        v.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerators_match_the_csharp_order() {
        let mut t = ElementTree::new();
        let a = t.add_string("A", 2);
        let b = t.add_string("B", 1);
        let root = t.add_node(Uuid::new_v4(), "Root", None);
        t.add_child(root, a);
        t.add_child(root, b);

        assert_eq!(t.leaves(root).len(), 3);
        assert_eq!(t.non_leaves(root), vec![root, a, b]);
        assert_eq!(t.max_child_depth(root), 3);
        assert_eq!(t.max_child_depth(a), 2);
        assert_eq!(t.roots, vec![root]);
    }

    #[test]
    fn shared_leaf_counts_twice_but_is_distinct_once() {
        let mut t = ElementTree::new();
        let leaf = t.add_element("Shared");
        let a = t.add_node(Uuid::new_v4(), "A", None);
        let b = t.add_node(Uuid::new_v4(), "B", None);
        let root = t.add_node(Uuid::new_v4(), "Root", None);
        t.add_child(a, leaf);
        t.add_child(b, leaf);
        t.add_child(root, a);
        t.add_child(root, b);

        assert_eq!(t.element_count(root), 2);
        assert_eq!(t.distinct_elements(root).len(), 1);
    }

    #[test]
    fn valid_colors_follow_the_color_property() {
        let mut t = ElementTree::new();
        let n = t.add_element("Red string");
        t.node_mut(n).properties.push(PropertyInstance {
            type_id: property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Color {
                color_type: ElementColorType::SingleColor,
                single_color: Color::RED,
                color_set_name: None,
            },
        });
        assert!(t.is_discrete(n));
        assert_eq!(t.valid_colors(n, false), vec![Color::RED]);
    }

    #[test]
    fn full_colour_nodes_have_no_valid_colours() {
        let mut t = ElementTree::new();
        let n = t.add_element("Pixel");
        assert!(!t.is_discrete(n));
        assert!(t.valid_colors(n, true).is_empty());
    }

    #[test]
    fn colour_set_lookup() {
        let mut t = ElementTree::new();
        let n = t.add_element("RGB string");
        t.node_mut(n).properties.push(PropertyInstance {
            type_id: property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Color {
                color_type: ElementColorType::MultipleDiscreteColors,
                single_color: Color::EMPTY,
                color_set_name: Some("RGB".to_string()),
            },
        });
        assert_eq!(t.valid_colors(n, false).len(), 3);
    }

    #[test]
    fn leaf_parents_are_inside_the_target() {
        let mut t = ElementTree::new();
        let a = t.add_string("A", 3);
        let b = t.add_string("B", 3);
        let root = t.add_node(Uuid::new_v4(), "Root", None);
        t.add_child(root, a);
        t.add_child(root, b);
        assert_eq!(t.leaf_parents(&[root]), vec![a, b]);
        assert_eq!(t.leaf_parents(&[a]), vec![a]);
    }

    #[test]
    fn removing_the_last_parent_returns_a_node_to_the_roots() {
        let mut t = ElementTree::new();
        let a = t.add_string("A", 1);
        let root = t.add_node(Uuid::new_v4(), "Root", None);
        t.add_child(root, a);
        assert!(!t.roots.contains(&a));
        t.remove_child(root, a);
        assert!(t.roots.contains(&a));
    }
}
