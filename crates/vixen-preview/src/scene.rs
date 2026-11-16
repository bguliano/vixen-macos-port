//! The flattened output for the renderer: every light's position and size, and what it shows.
//!
//! The scene is computed from the shapes' `Layout`, and shapes are not asked to redraw anything. Each shape
//! owns a contiguous range of lights. After a shape moves, only its range is rewritten in place, and the
//! returned [`SceneDelta`] says which lights to send to the GPU. No colours are stored here.
//!
//! A discretely coloured element gets one light per colour in its colour set, offset as `PreviewPixel.Draw`
//! offsets them: two across, then the next row down. The C# packs the colours that are lit right now into the
//! first of those positions, so a colour moves when another one turns off. Here each colour keeps its own
//! slot, so the geometry does not depend on the show that is playing.
//!
//! # Coordinates
//!
//! The C# bakes the zoom into each pixel's position, truncating, and draws it there. The renderer has a view zoom
//! of its own, which scales the background along with the lights. So a light's position here is the pixel's
//! position divided by the zoom: the renderer's view multiplies it back, and it lands on the pixel the C# draws it
//! on, while the background zooms without being resampled on the CPU. Give the renderer [`Scene::zoom`].

use crate::data::{DisplayItem, PreviewData, ShapeId};
use crate::pixel::PreviewPixel;
use crate::shape::{Shape, ShapeKind};
use std::collections::HashMap;
use std::ops::Range;
use uuid::Uuid;
use vixen_core::{ElementTree, NodeId};

/// One light: `#[repr(C)]` three `f32`s, the renderer's own layout. (x, y) is the centre, because
/// `FastPixel.DrawCircle` centres on the corner of `PreviewPixel.Bounds`, in the units described in the
/// [module documentation](self). `size` is `PixelSize`, unzoomed: zoom moves a light but doesn't resize it.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct LightGeom {
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

impl LightGeom {
    /// These lights as `x, y, size, x, y, size, ...`, without copying, for a renderer with a type of its own for
    /// the same layout. A mouse event sends part of the scene to the GPU, and that shouldn't take an allocation.
    #[inline]
    pub fn as_floats(lights: &[LightGeom]) -> &[f32] {
        // SAFETY: repr(C) with three f32 fields, so an array of them is an array of f32 with no padding.
        unsafe { std::slice::from_raw_parts(lights.as_ptr().cast::<f32>(), lights.len() * 3) }
    }
}

/// `LightLink::node` of a light that is not linked to anything.
pub const NO_NODE: u32 = u32::MAX;

/// What a light shows.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct LightLink {
    /// Index into [`Scene::nodes`], or [`NO_NODE`].
    pub node: u32,
    /// For a discretely coloured element, which of its colours this light shows, in
    /// `ElementTree::valid_colors` order. Zero otherwise.
    pub color_slot: u16,
    /// How many lights this element has per pixel: 1 for full colour.
    pub color_count: u16,
}

/// Which lights changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneDelta {
    /// The shape the change came from, or None for the whole scene.
    pub shape: Option<ShapeId>,
    /// Lights whose values changed. Send `Scene::lights[range]` to the GPU.
    pub range: Range<usize>,
    /// `Scene::links` changed in that range too.
    pub links_changed: bool,
    /// The number of lights changed, and everything from `range.start` on moved to a new index.
    pub resized: bool,
}

/// An intelligent fixture's place in the scene. It has no lights: the renderer draws it from this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixtureGeom {
    pub shape: ShapeId,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// The Intelligent Fixture element it is linked to.
    pub node: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct ShapeRange {
    id: ShapeId,
    start: u32,
    len: u32,
    /// Pixels, which is fewer than lights when an element has several colours.
    pixels: u32,
}

/// Interns node ids, and remembers how many colours each has.
#[derive(Clone, Debug, Default)]
struct NodeTable {
    ids: Vec<Uuid>,
    by_id: HashMap<Uuid, (u32, u16)>,
}

impl NodeTable {
    fn lookup(&mut self, id: Uuid, tree: Option<&ElementTree>) -> (u32, u16) {
        if id.is_nil() {
            return (NO_NODE, 1);
        }
        if let Some(found) = self.by_id.get(&id) {
            return *found;
        }
        let count = tree
            .and_then(|t| t.find(&id).map(|n| (t, n)))
            .filter(|(t, n)| t.is_discrete(*n))
            .map_or(1, |(t, n)| t.valid_colors(n, false).len().clamp(1, u16::MAX as usize) as u16);
        let entry = (self.ids.len() as u32, count);
        self.ids.push(id);
        self.by_id.insert(id, entry);
        entry
    }
}

/// See the [module documentation](self).
#[derive(Clone, Debug)]
pub struct Scene {
    /// Every light of every shape, in display item order, so later shapes draw on top.
    pub lights: Vec<LightGeom>,
    /// Parallel to `lights`.
    pub links: Vec<LightLink>,
    /// The fixtures, in display item order.
    pub fixtures: Vec<FixtureGeom>,
    nodes: NodeTable,
    shapes: Vec<ShapeRange>,
    index: HashMap<ShapeId, u32>,
    zoom: f64,
}

impl Default for Scene {
    fn default() -> Scene {
        Scene { lights: Vec::new(), links: Vec::new(), fixtures: Vec::new(), nodes: NodeTable::default(), shapes: Vec::new(), index: HashMap::new(), zoom: 1.0 }
    }
}

#[inline]
fn light_at(x: f32, y: f32, size: f32, slot: u16, zoom: f32) -> LightGeom {
    // PreviewPixel.Draw: odd colours step right, even colours step down a row and back to the left. The step is
    // the unzoomed size in screen pixels, so it goes on before the zoom comes off.
    LightGeom { x: (x + (slot % 2) as f32 * size) / zoom, y: (y + (slot / 2) as f32 * size) / zoom, size }
}

fn append_pixels(shape: &Shape, nodes: &mut NodeTable, tree: Option<&ElementTree>, zoom: f32, lights: &mut Vec<LightGeom>, links: &mut Vec<LightLink>) -> u32 {
    let mut pixels = 0u32;
    shape.for_each_pixel(&mut |p: &PreviewPixel| {
        pixels += 1;
        let (node, count) = nodes.lookup(p.node_id, tree);
        let (x, y) = p.draw_position();
        let size = p.size as f32;
        for slot in 0..count {
            lights.push(light_at(x, y, size, slot, zoom));
            links.push(LightLink { node, color_slot: slot, color_count: count });
        }
    });
    pixels
}

fn fixture_of(item: &DisplayItem) -> Option<FixtureGeom> {
    match item.shape.kind() {
        ShapeKind::MovingHead(d) => Some(FixtureGeom {
            shape: item.id,
            left: item.shape.left(),
            top: item.shape.top(),
            right: item.shape.right(),
            bottom: item.shape.bottom(),
            node: (!d.node_id.is_nil()).then_some(d.node_id),
        }),
        _ => None,
    }
}

impl Scene {
    /// Builds a scene from scratch, for shapes at zoom 1. Without a tree, every element counts as full colour.
    pub fn build(data: &PreviewData, tree: Option<&ElementTree>) -> Scene {
        Scene::build_at_zoom(data, tree, 1.0)
    }

    /// [`Scene::build`] for shapes laid out at `zoom`, which is what a [`DesignSession`] keeps its shapes at.
    ///
    /// [`DesignSession`]: crate::session::DesignSession
    pub fn build_at_zoom(data: &PreviewData, tree: Option<&ElementTree>, zoom: f64) -> Scene {
        let mut scene = Scene { zoom, ..Scene::default() };
        scene.rebuild_all(data, tree);
        scene
    }

    /// The zoom to give the renderer's view. A light lands at `(x, y) * zoom`, plus the pan.
    #[inline]
    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// The shapes are now laid out at this zoom. Their lights are wrong until they are written again.
    pub(crate) fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom;
    }

    /// Rebuilds everything, keeping the allocations.
    pub fn rebuild_all(&mut self, data: &PreviewData, tree: Option<&ElementTree>) {
        self.lights.clear();
        self.links.clear();
        self.fixtures.clear();
        self.shapes.clear();
        self.index.clear();
        self.nodes = NodeTable::default();
        let total = data.pixel_count();
        self.lights.reserve(total);
        self.links.reserve(total);
        for item in &data.display_items {
            self.push_item(item, tree);
        }
    }

    fn push_item(&mut self, item: &DisplayItem, tree: Option<&ElementTree>) {
        let start = self.lights.len() as u32;
        let pixels = append_pixels(&item.shape, &mut self.nodes, tree, self.zoom as f32, &mut self.lights, &mut self.links);
        let len = self.lights.len() as u32 - start;
        self.index.insert(item.id, self.shapes.len() as u32);
        self.shapes.push(ShapeRange { id: item.id, start, len, pixels });
        self.fixtures.extend(fixture_of(item));
    }

    /// The position of this shape in the display item list, which the scene's order always matches.
    #[inline]
    pub(crate) fn index_of(&self, id: ShapeId) -> Option<usize> {
        self.index.get(&id).map(|i| *i as usize)
    }

    /// The whole scene as one change, for after a zoom, which moves every light.
    pub(crate) fn delta_all(&self) -> SceneDelta {
        SceneDelta { shape: None, range: 0..self.lights.len(), links_changed: false, resized: false }
    }

    /// Rewrites every light's position in place. Fails if any shape's light count changed.
    pub(crate) fn refresh_geometry(&mut self, data: &PreviewData) -> bool {
        if data.display_items.len() != self.shapes.len() {
            return false;
        }
        for item in &data.display_items {
            if !self.write_geometry(item) {
                return false;
            }
        }
        true
    }

    #[inline]
    pub fn light_count(&self) -> usize {
        self.lights.len()
    }

    /// The status bar's `Lights:`, `PixelMap`'s or `UpdatePixelCache`'s count: pixels whose node exists and has an element,
    /// once each however many colours it has. Not `lights.len()`, which has every pixel, linked or not, and one per colour.
    pub fn linked_pixel_count(&self, tree: &ElementTree) -> u32 {
        let has_element: Vec<bool> = self.nodes.ids.iter().map(|id| tree.find(id).is_some_and(|n| tree.node(n).element_id.is_some())).collect();
        self.links.iter().filter(|l| l.color_slot == 0 && has_element.get(l.node as usize).copied().unwrap_or(false)).count() as u32
    }

    /// The element nodes the lights link to. `LightLink::node` indexes this.
    #[inline]
    pub fn nodes(&self) -> &[Uuid] {
        &self.nodes.ids
    }

    /// The node a light shows, if it is linked.
    #[inline]
    pub fn node_of_light(&self, light: usize) -> Option<Uuid> {
        let link = self.links.get(light)?;
        self.nodes.ids.get(link.node as usize).copied()
    }

    /// The range of lights a shape owns.
    pub fn range_of(&self, id: ShapeId) -> Option<Range<usize>> {
        let r = self.shapes[*self.index.get(&id)? as usize];
        Some(r.start as usize..(r.start + r.len) as usize)
    }

    /// The shape a light belongs to.
    pub fn shape_of_light(&self, light: usize) -> Option<ShapeId> {
        if light >= self.lights.len() {
            return None;
        }
        let light = light as u32;
        let i = self.shapes.partition_point(|r| r.start + r.len <= light);
        self.shapes.get(i).map(|r| r.id)
    }

    /// For each entry of [`Scene::nodes`], where it is in the tree, for looking up colours each frame.
    pub fn resolve_nodes(&self, tree: &ElementTree) -> Vec<Option<NodeId>> {
        self.nodes.ids.iter().map(|id| tree.find(id)).collect()
    }

    /// The hot path: positions only, no allocation, no hashing beyond finding the shape.
    fn write_geometry(&mut self, item: &DisplayItem) -> bool {
        let Some(&index) = self.index.get(&item.id) else { return false };
        let r = self.shapes[index as usize];
        if item.shape.pixel_count() != r.pixels as usize {
            return false;
        }
        let range = r.start as usize..(r.start + r.len) as usize;
        let lights = &mut self.lights[range.clone()];
        let links = &self.links[range];
        let n = lights.len();
        let zoom = self.zoom as f32;
        let mut i = 0;
        item.shape.for_each_pixel(&mut |p: &PreviewPixel| {
            let (x, y) = p.draw_position();
            let size = p.size as f32;
            // A pixel's lights run from its slot zero up to the next slot zero.
            while i < n {
                lights[i] = light_at(x, y, size, links[i].color_slot, zoom);
                i += 1;
                if i >= n || links[i].color_slot == 0 {
                    break;
                }
            }
        });
        if matches!(item.shape.kind(), ShapeKind::MovingHead(_)) {
            if let (Some(g), Some(f)) = (fixture_of(item), self.fixtures.iter_mut().find(|f| f.shape == item.id)) {
                *f = g;
            }
        }
        true
    }

    /// After a shape moved or resized: rewrites only its range. A shape whose light count changed takes the
    /// slower [`Scene::rebuild_shape`] path. None if the scene does not hold this shape.
    pub fn update_shape(&mut self, item: &DisplayItem, tree: Option<&ElementTree>) -> Option<SceneDelta> {
        if self.write_geometry(item) {
            return Some(SceneDelta { shape: Some(item.id), range: self.range_of(item.id)?, links_changed: false, resized: false });
        }
        self.rebuild_shape(item, tree)
    }

    /// After a shape's links or light count changed: rebuilds its range, moving later lights if it must.
    pub fn rebuild_shape(&mut self, item: &DisplayItem, tree: Option<&ElementTree>) -> Option<SceneDelta> {
        let index = *self.index.get(&item.id)? as usize;
        let r = self.shapes[index];
        let start = r.start as usize;
        let end = start + r.len as usize;
        let mut new_lights = Vec::with_capacity(r.len as usize);
        let mut new_links = Vec::with_capacity(r.len as usize);
        let pixels = append_pixels(&item.shape, &mut self.nodes, tree, self.zoom as f32, &mut new_lights, &mut new_links);
        let new_len = new_lights.len() as u32;
        let resized = new_len != r.len;
        self.lights.splice(start..end, new_lights);
        self.links.splice(start..end, new_links);
        self.shapes[index].len = new_len;
        self.shapes[index].pixels = pixels;
        if resized {
            let mut at = r.start + new_len;
            for s in &mut self.shapes[index + 1..] {
                s.start = at;
                at += s.len;
            }
        }
        if let (Some(g), Some(f)) = (fixture_of(item), self.fixtures.iter_mut().find(|f| f.shape == item.id)) {
            *f = g;
        }
        let range = if resized { start..self.lights.len() } else { start..start + new_len as usize };
        Some(SceneDelta { shape: Some(item.id), range, links_changed: true, resized })
    }

    /// A new display item, at the end of the list.
    pub fn add_shape(&mut self, item: &DisplayItem, tree: Option<&ElementTree>) -> SceneDelta {
        let start = self.lights.len();
        self.push_item(item, tree);
        SceneDelta { shape: Some(item.id), range: start..self.lights.len(), links_changed: true, resized: self.lights.len() != start }
    }

    /// A display item is gone.
    pub fn remove_shape(&mut self, id: ShapeId) -> Option<SceneDelta> {
        let index = self.index.remove(&id)? as usize;
        let r = self.shapes.remove(index);
        let start = r.start as usize;
        let end = start + r.len as usize;
        self.lights.drain(start..end);
        self.links.drain(start..end);
        self.fixtures.retain(|f| f.shape != id);
        for (i, s) in self.shapes.iter_mut().enumerate().skip(index) {
            s.start -= r.len;
            self.index.insert(s.id, i as u32);
        }
        Some(SceneDelta { shape: Some(id), range: start..self.lights.len(), links_changed: true, resized: r.len != 0 })
    }

    /// A check for tests: is the scene what a fresh build would produce?
    #[cfg(test)]
    pub(crate) fn matches_fresh_build(&self, data: &PreviewData, tree: Option<&ElementTree>) -> bool {
        let fresh = Scene::build_at_zoom(data, tree, self.zoom);
        let same_links = self.links.len() == fresh.links.len()
            && self.links.iter().zip(&fresh.links).all(|(a, b)| {
                let id = |s: &Scene, l: &LightLink| s.nodes.ids.get(l.node as usize).copied();
                id(self, a) == id(&fresh, b) && a.color_slot == b.color_slot && a.color_count == b.color_count
            });
        self.lights == fresh.lights && same_links && self.shapes == fresh.shapes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::PreviewPoint;
    use vixen_core::element::{property_ids, PropertyData, PropertyInstance};
    use vixen_core::{Color, ElementColorType};

    fn tree_with_rgb() -> (ElementTree, Uuid, Uuid) {
        let mut t = ElementTree::new();
        let rgb = t.add_element("rgb");
        t.node_mut(rgb).properties.push(PropertyInstance {
            type_id: property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Color { color_type: ElementColorType::MultipleDiscreteColors, single_color: Color::EMPTY, color_set_name: Some("RGBW".to_string()) },
        });
        let full = t.add_element("full");
        let (a, b) = (t.node(rgb).id, t.node(full).id);
        (t, a, b)
    }

    fn line_data(tree_node: Option<Uuid>) -> PreviewData {
        let mut d = PreviewData::new();
        for i in 0..3 {
            let mut s = Shape::new_line(PreviewPoint::new(0, 10 * i), PreviewPoint::new(40, 10 * i), 5, None, 1.0);
            if let Some(n) = tree_node {
                for p in &mut s.pixels {
                    p.node_id = n;
                }
            }
            d.add_shape(s);
        }
        d
    }

    #[test]
    fn discrete_elements_get_one_light_per_colour_offset_like_preview_pixel() {
        let (tree, rgb, _) = tree_with_rgb();
        let mut d = PreviewData::new();
        let mut s = Shape::new_single(PreviewPoint::new(100, 200), None, 1.0);
        s.pixels[0].node_id = rgb;
        s.set_pixel_size(4);
        d.add_shape(s);
        let scene = Scene::build(&d, Some(&tree));
        // PreviewPixel.Draw with four lit colours, size 4: (x, y), (x+4, y), (x, y+4), (x+4, y+4).
        let at: Vec<(f32, f32)> = scene.lights.iter().map(|l| (l.x, l.y)).collect();
        assert_eq!(at, [(100.0, 200.0), (104.0, 200.0), (100.0, 204.0), (104.0, 204.0)]);
        assert!(scene.links.iter().enumerate().all(|(i, l)| l.color_slot == i as u16 && l.color_count == 4 && scene.nodes()[l.node as usize] == rgb));
    }

    #[test]
    fn lights_read_as_floats_without_a_copy() {
        let lights = [LightGeom { x: 1.0, y: 2.0, size: 3.0 }, LightGeom { x: 4.0, y: 5.0, size: 6.0 }];
        assert_eq!(LightGeom::as_floats(&lights), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert!(LightGeom::as_floats(&[]).is_empty());
    }

    #[test]
    fn full_colour_and_unlinked_lights_have_one_slot() {
        let (tree, _, full) = tree_with_rgb();
        let scene = Scene::build(&line_data(Some(full)), Some(&tree));
        assert_eq!(scene.light_count(), 15);
        assert!(scene.links.iter().all(|l| l.color_slot == 0 && l.color_count == 1));
        let scene = Scene::build(&line_data(None), Some(&tree));
        assert!(scene.links.iter().all(|l| l.node == NO_NODE));
        assert_eq!(scene.node_of_light(0), None);
    }

    #[test]
    fn moving_a_shape_rewrites_only_its_range() {
        let (tree, rgb, _) = tree_with_rgb();
        let mut d = line_data(Some(rgb));
        let mut scene = Scene::build(&d, Some(&tree));
        let before = scene.lights.clone();
        let id = d.display_items[1].id;
        d.display_items[1].shape.move_to(500, 500);
        let delta = scene.update_shape(&d.display_items[1], Some(&tree)).unwrap();
        assert_eq!(delta, SceneDelta { shape: Some(id), range: 20..40, links_changed: false, resized: false });
        assert_eq!(scene.lights[..20], before[..20]);
        assert_eq!(scene.lights[40..], before[40..]);
        assert_ne!(scene.lights[20..40], before[20..40]);
        assert!(scene.matches_fresh_build(&d, Some(&tree)));
    }

    #[test]
    fn a_change_of_light_count_shifts_the_shapes_after_it() {
        let mut d = line_data(None);
        let mut scene = Scene::build(&d, None);
        d.display_items[0].shape.set_line_pixel_count(8);
        let delta = scene.update_shape(&d.display_items[0], None).unwrap();
        assert!(delta.resized);
        assert_eq!(delta.range, 0..18);
        assert_eq!(scene.range_of(d.display_items[1].id), Some(8..13));
        assert!(scene.matches_fresh_build(&d, None));
    }

    #[test]
    fn adding_and_removing_shapes_stays_consistent() {
        let mut d = line_data(None);
        let mut scene = Scene::build(&d, None);
        let removed = d.display_items.remove(1);
        let delta = scene.remove_shape(removed.id).unwrap();
        assert_eq!(delta.range, 5..10);
        assert!(scene.matches_fresh_build(&d, None));
        assert_eq!(scene.range_of(removed.id), None);
        assert_eq!(scene.shape_of_light(7), Some(d.display_items[1].id));

        d.display_items.push(removed);
        let delta = scene.add_shape(&d.display_items[2], None);
        assert_eq!(delta.range, 10..15);
        assert!(scene.matches_fresh_build(&d, None));
    }
}
