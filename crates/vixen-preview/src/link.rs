//! Port of the logic in Shapes/PreviewSetElements.cs, the Linked Elements dialog: which element each pixel of
//! a shape's strings shows. The dialog is the host's: it drives an [`ElementLinker`] and calls
//! [`ElementLinker::apply`] for OK. For a plain assignment there is [`link_elements`].

use crate::pixel::PreviewPixel;
use crate::shape::{Shape, ShapeKind, StringType};
use uuid::Uuid;
use vixen_core::{ElementTree, NodeId};

/// `PreviewSetElementString`.
#[derive(Clone, Debug)]
pub struct LinkString {
    pub name: String,
    /// The linked node of each entry. A Standard string has one entry, standing for the whole string.
    pub nodes: Vec<Uuid>,
    /// The string this came from in the shape's `Strings`.
    shape_index: usize,
    /// The pixels themselves, which go back into the shape on OK.
    pixels: Vec<PreviewPixel>,
}

/// The dialog's state between opening and OK.
#[derive(Clone, Debug)]
pub struct ElementLinker {
    strings: Vec<LinkString>,
    connect_standard_strings: bool,
    /// `_shapes[0].StringType`.
    first_is_standard: bool,
    light_count_editable: bool,
    shape_count: usize,
}

/// `shape.Strings`: the string shapes, or the shape itself.
fn strings_of(shape: &Shape) -> Vec<&Shape> {
    match shape.strings() {
        Some(s) if !matches!(shape.kind(), ShapeKind::Custom { .. }) => s.iter().collect(),
        _ => vec![shape],
    }
}

impl ElementLinker {
    /// `new PreviewSetElements(shape.Strings)`.
    pub fn new(shape: &Shape) -> ElementLinker {
        let shapes = strings_of(shape);
        let connect_standard_strings = shape.connect_standard_strings;
        let mut strings = Vec::with_capacity(shapes.len());
        let mut i = 1;
        for (shape_index, s) in shapes.iter().enumerate() {
            if s.pixel_count() == 0 {
                continue;
            }
            let pixels: Vec<PreviewPixel> = match s.string_type() {
                // If this is a Standard string, only set the first pixel of the string
                StringType::Standard => s.first_pixel().cloned().into_iter().collect(),
                // If this is a pixel string, let them set every pixel
                StringType::Pixel | StringType::Custom => s.pixels_vec(),
            };
            let nodes = pixels.iter().map(|p| p.node_id).collect();
            strings.push(LinkString { name: format!("String {i}"), nodes, shape_index, pixels });
            i += 1;
        }
        let first_is_standard = shapes.first().is_some_and(|s| s.string_type() == StringType::Standard);
        let light_count_editable = match shape.kind() {
            ShapeKind::Icicle { .. } => !first_is_standard,
            ShapeKind::MultiString { .. } => true,
            _ => false,
        };
        ElementLinker { strings, connect_standard_strings, first_is_standard, light_count_editable, shape_count: shapes.len() }
    }

    /// `PopulateStringList`: how many strings the dialog's list shows. Connected Standard strings are one.
    pub fn visible_string_count(&self) -> usize {
        if self.connect_standard_strings && self.first_is_standard {
            self.strings.len().min(1)
        } else {
            self.strings.len()
        }
    }

    pub fn strings(&self) -> &[LinkString] {
        &self.strings
    }

    /// `tblLightCountControls.Visible`: Icicles of pixels and Multi-Strings can change a string's light count.
    pub fn light_count_editable(&self) -> bool {
        self.light_count_editable
    }

    /// `SetLinkedElementItems`: links an element to this entry, or a group's children to this entry and the ones
    /// after it. Returns the entry after the last one used.
    pub fn set_linked(&mut self, string: usize, entry: usize, node: NodeId, tree: &ElementTree) -> usize {
        let Some(s) = self.strings.get_mut(string) else { return entry };
        if entry >= s.pixels.len() {
            return entry;
        }
        // Is this node an element?
        if tree.node(node).element_id.is_some() {
            s.pixels[entry].node_id = tree.node(node).id;
            s.nodes[entry] = tree.node(node).id;
            return entry + 1;
        }
        // This node is a group, iterate
        let mut next = entry;
        for child in tree.node(node).children.clone() {
            if tree.node(child).element_id.is_some() {
                next = self.set_linked(string, next, child, tree);
                if next >= self.strings[string].pixels.len() {
                    return next;
                }
            }
        }
        next
    }

    /// `listLinkedElements_DragDrop`: dropped nodes go to this entry and the ones after it.
    pub fn drop_nodes(&mut self, string: usize, entry: usize, nodes: &[NodeId], tree: &ElementTree) {
        for (item, node) in (entry..).zip(nodes) {
            let last = self.strings.get(string).map_or(0, |s| s.pixels.len().saturating_sub(1));
            // The group's own next entry is not what the drop advances by: each dropped node moves one on.
            self.set_linked(string, item, *node, tree);
            if item == last {
                break;
            }
        }
    }

    /// `copyToAllElementsAllStringsToolStripMenuItem_Click`.
    pub fn copy_to_all_strings(&mut self, string: usize, entry: usize) {
        let Some(node) = self.strings.get(string).and_then(|s| s.nodes.get(entry)).copied() else { return };
        for s in &mut self.strings {
            s.set_all(node);
        }
    }

    /// `copyToAllElementsInThisStringToolStripMenuItem_Click`.
    pub fn copy_to_this_string(&mut self, string: usize, entry: usize) {
        let Some(s) = self.strings.get_mut(string) else { return };
        let Some(node) = s.nodes.get(entry).copied() else { return };
        s.set_all(node);
    }

    /// `reverseElementLinkingInThisStringToolStripMenuItem_Click`.
    pub fn reverse_string(&mut self, string: usize) {
        if let Some(s) = self.strings.get_mut(string) {
            s.pixels.reverse();
            s.nodes.reverse();
        }
    }

    /// `buttonSetLightCount_Click`.
    pub fn set_light_count(&mut self, string: usize, count: usize) {
        if let Some(s) = self.strings.get_mut(string) {
            s.pixels.resize_with(count, PreviewPixel::default);
            s.nodes.resize(count, Uuid::nil());
        }
    }

    /// `buttonOK_Click`, then the editor's `Parent.Layout()`.
    pub fn apply(self, shape: &mut Shape) {
        if (self.connect_standard_strings || self.shape_count == 1) && self.first_is_standard {
            // The first string's first entry goes to every pixel of every string.
            let node = self.strings.first().and_then(|s| s.nodes.first()).copied().unwrap_or_else(Uuid::nil);
            match_strings_mut(shape, &mut |s, _| {
                for p in &mut s.pixels {
                    p.node_id = node;
                }
            });
        } else {
            let strings = self.strings;
            match_strings_mut(shape, &mut |target, index| {
                let Some(ls) = strings.iter().find(|s| s.shape_index == index) else { return };
                match target.string_type() {
                    // If this is a standard light string, assign ALL pixels to the first node
                    StringType::Standard => {
                        let node = ls.nodes.first().copied().unwrap_or_else(Uuid::nil);
                        for p in &mut target.pixels {
                            p.node_id = node;
                        }
                    }
                    StringType::Pixel | StringType::Custom => {
                        let size = target.pixel_size();
                        target.replace_pixels(&ls.pixels, size);
                    }
                }
            });
        }
        for_each_string_layout(shape);
        shape.layout();
    }
}

/// One row of the Linked Elements list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRow {
    /// Counted from 1.
    pub number: u32,
    pub node: Option<Uuid>,
    /// Empty when there's no link, or its element no longer exists.
    pub element_name: String,
}

impl ElementLinker {
    /// What `UpdateListLinkedElements` does to each row as the dialog opens: a pixel linked to a node with no element takes
    /// that node's first child with an element, and stays as it is when there's none. So just opening the dialog rewrites
    /// such a link, and OK makes it permanent, which is enough to clear the warning at close.
    pub fn resolve_group_links(&mut self, tree: &ElementTree) {
        for s in &mut self.strings {
            for entry in 0..s.pixels.len() {
                let id = s.nodes[entry];
                if id.is_nil() {
                    continue;
                }
                let Some(node) = tree.find(&id) else { continue };
                if tree.node(node).element_id.is_some() {
                    continue;
                }
                let child = tree.node(node).children.iter().copied().find(|c| tree.node(*c).element_id.is_some());
                if let Some(child) = child {
                    let child_id = tree.node(child).id;
                    s.pixels[entry].node_id = child_id;
                    s.nodes[entry] = child_id;
                }
            }
        }
    }

    /// What the list shows for a string, in bulk, with the names looked up here and not row by row across the FFI.
    pub fn rows(&self, string: usize, tree: &ElementTree) -> Vec<LinkRow> {
        let Some(s) = self.strings.get(string) else { return Vec::new() };
        s.nodes
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let node = (!id.is_nil()).then_some(*id);
                let element_name = node.and_then(|id| tree.find(&id)).map_or_else(String::new, |n| tree.node(n).name.clone());
                LinkRow { number: i as u32 + 1, node, element_name }
            })
            .collect()
    }

    /// The light count box shows the string's count, at least 1.
    pub fn light_count(&self, string: usize) -> usize {
        self.strings.get(string).map_or(1, |s| s.pixels.len().max(1))
    }
}

impl LinkString {
    fn set_all(&mut self, node: Uuid) {
        for p in &mut self.pixels {
            p.node_id = node;
        }
        self.nodes.fill(node);
    }
}

/// The dialog's `_shapes`, mutably, with their index.
fn match_strings_mut(shape: &mut Shape, f: &mut impl FnMut(&mut Shape, usize)) {
    if !matches!(shape.kind(), ShapeKind::Custom { .. }) {
        if let Some(strings) = shape.strings.as_mut() {
            for (i, s) in strings.iter_mut().enumerate() {
                f(s, i);
            }
            return;
        }
    }
    f(shape, 0);
}

/// `_shapes.ForEach(x => x.Layout())`.
fn for_each_string_layout(shape: &mut Shape) {
    if !matches!(shape.kind(), ShapeKind::Custom { .. }) {
        if let Some(strings) = shape.strings.as_mut() {
            for s in strings {
                s.layout();
            }
        }
    }
}

/// `SelectFixtureNodeViewModel`: every node with an Intelligent Fixture property, with its name, for the Link Element dialog.
pub fn fixture_nodes(tree: &ElementTree) -> Vec<(Uuid, String)> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stack: Vec<NodeId> = tree.roots.iter().rev().copied().collect();
    while let Some(n) = stack.pop() {
        let node = tree.node(n);
        if !seen.insert(node.id) {
            continue;
        }
        if node.properties.iter().any(|p| p.type_id == vixen_core::element::property_ids::INTELLIGENT_FIXTURE) {
            out.push((node.id, node.name.clone()));
        }
        stack.extend(node.children.iter().rev().copied());
    }
    out
}

/// Links a shape to elements in one call, as dropping them on the dialog's first entry and pressing OK would.
/// A group fills consecutive pixels with its children. With Standard strings connected, or one string, only the
/// first node matters, and with several unconnected strings each takes the next node or group in turn.
pub fn link_elements(shape: &mut Shape, nodes: &[NodeId], tree: &ElementTree) {
    let mut linker = ElementLinker::new(shape);
    let count = linker.visible_string_count();
    if count <= 1 {
        linker.drop_nodes(0, 0, nodes, tree);
    } else {
        for (string, node) in nodes.iter().enumerate().take(count) {
            linker.set_linked(string, 0, *node, tree);
        }
    }
    linker.apply(shape);
}

impl Shape {
    /// The pixels the dialog edited replace the string's, keeping each light's place in its layout.
    pub(crate) fn replace_pixels(&mut self, edited: &[PreviewPixel], size: i32) {
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            c.prop_pixels.truncate(edited.len());
            for (i, e) in edited.iter().enumerate() {
                // Linking does not move a custom prop's lights: the edit list reads the shown pixels, which are
                // zoomed and rotated, and only the link is taken back to the model.
                match c.prop_pixels.get_mut(i) {
                    Some(p) => p.node_id = e.node_id,
                    None => c.prop_pixels.push(e.clone()),
                }
            }
            return;
        }
        self.pixels.clear();
        self.pixels.extend_from_slice(edited);
        for p in &mut self.pixels {
            p.size = size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::PreviewPoint;

    fn tree() -> (ElementTree, NodeId) {
        let mut t = ElementTree::new();
        let g = t.add_string("Group", 5);
        (t, g)
    }

    fn line(n: i32) -> Shape {
        Shape::new_line(PreviewPoint::new(0, 0), PreviewPoint::new(100, 0), n, None, 1.0)
    }

    #[test]
    fn a_standard_line_links_every_pixel_to_the_one_node() {
        let (t, g) = tree();
        let leaf = t.node(g).children[2];
        let mut s = line(4);
        link_elements(&mut s, &[leaf], &t);
        assert!(s.pixels.iter().all(|p| p.node_id == t.node(leaf).id));
    }

    #[test]
    fn a_pixel_line_takes_a_group_in_order() {
        let (t, g) = tree();
        let mut s = line(5);
        s.set_string_type(StringType::Pixel);
        link_elements(&mut s, &[g], &t);
        let ids: Vec<Uuid> = t.node(g).children.iter().map(|c| t.node(*c).id).collect();
        assert_eq!(s.pixels.iter().map(|p| p.node_id).collect::<Vec<_>>(), ids);
    }

    #[test]
    fn a_group_longer_than_the_string_stops_at_its_end() {
        let (t, g) = tree();
        let mut s = line(3);
        s.set_string_type(StringType::Pixel);
        link_elements(&mut s, &[g], &t);
        assert_eq!(s.pixels.len(), 3);
        assert_eq!(s.pixels[2].node_id, t.node(t.node(g).children[2]).id);
    }

    #[test]
    fn standard_strings_of_a_selected_rectangle_are_one() {
        let (t, g) = tree();
        let leaf = t.node(g).children[0];
        let mut s = Shape::new_rectangle(PreviewPoint::new(0, 0), None, 1.0);
        s.select(false); // sets connectStandardStrings, as selecting it to edit it does
        let linker = ElementLinker::new(&s);
        assert_eq!(linker.strings().len(), 4);
        assert_eq!(linker.visible_string_count(), 1);
        link_elements(&mut s, &[leaf], &t);
        assert!(s.pixels_vec().iter().all(|p| p.node_id == t.node(leaf).id));
    }

    #[test]
    fn unconnected_strings_keep_their_own_links() {
        let (t, g) = tree();
        let (a, b) = (t.node(g).children[0], t.node(g).children[1]);
        let mut s = Shape::new_mega_tree(PreviewPoint::new(0, 0), None, 1.0);
        s.set_mega_tree_string_count(2);
        link_elements(&mut s, &[a, b], &t);
        let strings = s.strings().unwrap();
        assert!(strings[0].pixels.iter().all(|p| p.node_id == t.node(a).id));
        assert!(strings[1].pixels.iter().all(|p| p.node_id == t.node(b).id));
    }

    #[test]
    fn reverse_copy_and_light_count_edit_the_list() {
        let (t, g) = tree();
        let mut s = Shape::new_multi_string(PreviewPoint::new(0, 0), None, 1.0);
        s.mouse_up(50, 0, true, None);
        s.mouse_up(100, 0, true, None);
        s.select(true); // which makes every string a pixel string
        let mut linker = ElementLinker::new(&s);
        assert!(linker.light_count_editable());
        assert_eq!(linker.strings().len(), 2);
        linker.set_light_count(0, 5);
        linker.drop_nodes(0, 0, &[g], &t);
        linker.reverse_string(0);
        let ids: Vec<Uuid> = t.node(g).children.iter().rev().map(|c| t.node(*c).id).collect();
        assert_eq!(linker.strings()[0].nodes, ids);
        linker.copy_to_this_string(0, 1);
        assert!(linker.strings()[0].nodes.iter().all(|n| *n == ids[1]));
        linker.copy_to_all_strings(0, 0);
        assert!(linker.strings()[1].nodes.iter().all(|n| *n == ids[1]));
        linker.apply(&mut s);
        assert_eq!(s.strings().unwrap()[0].pixels.len(), 5);
    }
}
