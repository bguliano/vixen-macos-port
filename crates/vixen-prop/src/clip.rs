//! The element clipboard: Cut and Copy capture whole element trees, Paste links or re-creates them and
//! Paste as New makes fresh copies. The clip is text so the pasteboard can carry it between editor windows.

use std::collections::{HashMap, HashSet};

use serde_json::Value;
use uuid::Uuid;

use crate::file::{element_doc, element_from_doc, parse_element, write_element, ElementDoc};
use crate::model::*;

const CLIP_KEY: &str = "vixenPropClip";

#[derive(Clone, Debug)]
pub struct PropClip {
    top: Vec<Uuid>,
    elements: HashMap<Uuid, ElementDoc>,
    order: Vec<Uuid>,
}

fn bad_clip() -> PropError {
    PropError::new("The clipboard does not hold prop elements.", "not a prop element clip")
}

impl PropClip {
    /// The given elements and everything below them, as they are right now.
    pub fn capture(prop: &Prop, top: &[Uuid]) -> Option<PropClip> {
        let mut clip = PropClip { top: Vec::new(), elements: HashMap::new(), order: Vec::new() };
        for id in top {
            if !prop.contains(*id) || clip.top.contains(id) {
                continue;
            }
            clip.top.push(*id);
            for node in prop.node_enumerator(*id) {
                if clip.elements.contains_key(&node) {
                    continue;
                }
                if let Some(doc) = element_doc(prop, node) {
                    clip.elements.insert(node, doc);
                    clip.order.push(node);
                }
            }
        }
        (!clip.top.is_empty()).then_some(clip)
    }

    pub fn is_empty(&self) -> bool {
        self.top.is_empty()
    }

    pub fn top(&self) -> &[Uuid] {
        &self.top
    }

    pub fn to_json(&self) -> String {
        let mut json = format!("{{\"{CLIP_KEY}\":1,\"top\":[");
        for (i, id) in self.top.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!("\"{}\"", id.hyphenated()));
        }
        json.push_str("],\"elements\":[");
        for (i, id) in self.order.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            if let Some(doc) = self.elements.get(id) {
                write_element(&mut json, doc);
            }
        }
        json.push_str("]}");
        json
    }

    pub fn from_json(text: &str) -> Result<PropClip, PropError> {
        let value: Value = serde_json::from_str(text).map_err(|_| bad_clip())?;
        let object = value.as_object().ok_or_else(bad_clip)?;
        if object.get(CLIP_KEY).and_then(Value::as_i64) != Some(1) {
            return Err(bad_clip());
        }
        let mut clip = PropClip { top: Vec::new(), elements: HashMap::new(), order: Vec::new() };
        for entry in object.get("elements").and_then(Value::as_array).ok_or_else(bad_clip)? {
            let doc = parse_element(entry.as_object().ok_or_else(bad_clip)?)?;
            // Checked here so a damaged clip fails before anything is pasted.
            element_from_doc(&doc)?;
            clip.order.push(doc.id);
            clip.elements.insert(doc.id, doc);
        }
        for entry in object.get("top").and_then(Value::as_array).ok_or_else(bad_clip)? {
            let id = entry.as_str().and_then(|s| Uuid::parse_str(s).ok()).ok_or_else(bad_clip)?;
            if !clip.elements.contains_key(&id) {
                return Err(bad_clip());
            }
            clip.top.push(id);
        }
        if clip.top.is_empty() {
            return Err(bad_clip());
        }
        Ok(clip)
    }

    fn lights(doc: &ElementDoc) -> Vec<Light> {
        doc.lights.iter().flatten().flatten().cloned().collect()
    }

    fn children(doc: &ElementDoc) -> Vec<Uuid> {
        doc.child_ids.clone().unwrap_or_default()
    }

    fn is_light_node(doc: &ElementDoc) -> bool {
        Self::children(doc).is_empty() && !Self::lights(doc).is_empty()
    }

    fn all_light_nodes(&self) -> bool {
        self.top.iter().filter_map(|id| self.elements.get(id)).all(Self::is_light_node)
    }

    fn all_group_nodes(&self) -> bool {
        self.top.iter().filter_map(|id| self.elements.get(id)).all(|doc| Self::lights(doc).is_empty())
    }

    fn fits(&self, prop: &Prop, target: Uuid) -> bool {
        (self.all_light_nodes() && prop.can_add_light_nodes(target)) || (self.all_group_nodes() && prop.can_add_group_nodes(target))
    }

    /// `CanPaste`: one kind of element, none of them already a child of the target, and a target that takes that kind.
    pub fn can_paste(&self, prop: &Prop, target: Uuid) -> bool {
        let Some(t) = prop.element(target) else { return false };
        !self.top.iter().any(|id| t.children().contains(id)) && self.fits(prop, target)
    }

    /// `CanPasteAsNew`: as `can_paste` without the already-a-child test, since everything pasted is new.
    pub fn can_paste_as_new(&self, prop: &Prop, target: Uuid) -> bool {
        prop.contains(target) && self.fits(prop, target)
    }

    /// `Paste`: an element still in the prop gains the target as one more parent; one that is gone comes back with
    /// its old id, a unique name and its lights. Returns the top elements now under the target.
    pub fn paste(&self, prop: &mut Prop, target: Uuid) -> Vec<Uuid> {
        let mut pasted = Vec::new();
        for id in &self.top {
            let already = prop.element(target).is_some_and(|t| t.children().contains(id));
            if *id == target || already {
                continue;
            }
            let mut visiting = HashSet::new();
            if self.find_or_create(prop, *id, target, &mut visiting) {
                pasted.push(*id);
            }
        }
        pasted
    }

    fn find_or_create(&self, prop: &mut Prop, id: Uuid, parent: Uuid, visiting: &mut HashSet<Uuid>) -> bool {
        if prop.contains(id) {
            if prop.element(parent).is_some_and(|p| p.children().contains(&id)) {
                return true;
            }
            return prop.insert_to_parent(id, parent, usize::MAX);
        }
        let Some(doc) = self.elements.get(&id) else { return false };
        if !visiting.insert(id) {
            return false;
        }
        let Ok(mut element) = element_from_doc(doc) else { return false };
        element.name = prop.uniquify(&doc.name, 2, None);
        let lights = Self::lights(doc);
        let light_node = Self::is_light_node(doc);
        if light_node {
            element.order = if prop.order_exists(doc.order) { prop.next_order() } else { doc.order };
        }
        prop.insert_element(element);
        if !prop.insert_to_parent(id, parent, usize::MAX) {
            return false;
        }
        if light_node {
            let size = doc.light_size.max(1) as f64;
            prop.push_lights(id, lights.into_iter().map(|l| Light { size, ..l }).collect());
        }
        for child in Self::children(doc) {
            self.find_or_create(prop, child, id, visiting);
        }
        visiting.remove(&id);
        true
    }

    /// `PasteAsNew`: fresh copies under the target. The top copies take the typed name, everything below takes its
    /// new parent's name numbered from 1. Returns the top copies.
    pub fn paste_as_new(&self, prop: &mut Prop, target: Uuid, name: &str) -> Vec<Uuid> {
        let mut created = Vec::new();
        for id in &self.top {
            let mut visiting = HashSet::new();
            if let Some(new_id) = self.create_tree(prop, *id, target, name, false, &mut visiting) {
                created.push(new_id);
            }
        }
        created
    }

    fn create_tree(&self, prop: &mut Prop, id: Uuid, parent: Uuid, name: &str, recursed: bool, visiting: &mut HashSet<Uuid>) -> Option<Uuid> {
        let doc = self.elements.get(&id)?;
        if !visiting.insert(id) {
            return None;
        }
        let new_id = prop.create_node(name, Some(parent), recursed);
        if Self::is_light_node(doc) {
            let size = doc.light_size.max(1);
            let order = prop.next_order();
            let lights = Self::lights(doc)
                .into_iter()
                .map(|l| Light { id: Uuid::new_v4(), size: size as f64, ..l })
                .collect();
            prop.push_lights(new_id, lights);
            if let Some(e) = prop.element_mut(new_id) {
                e.order = order;
                e.light_size = size;
            }
        }
        let new_name = prop.element(new_id).map(|e| e.name().to_string()).unwrap_or_default();
        for child in Self::children(doc) {
            self.create_tree(prop, child, new_id, &new_name, true, visiting);
        }
        visiting.remove(&id);
        Some(new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop_with_group() -> (Prop, Uuid, Uuid, Uuid) {
        let mut prop = Prop::new("Star", "me", "2024-01-01T00:00:00");
        let a = prop.add_light(None, 10.0, 10.0, 3).unwrap();
        let b = prop.add_light(None, 20.0, 10.0, 3).unwrap();
        let group = prop.element(a).unwrap().parents()[0];
        (prop, group, a, b)
    }

    #[test]
    fn clip_text_round_trips() {
        let (prop, group, a, _) = prop_with_group();
        let clip = PropClip::capture(&prop, &[group]).unwrap();
        let again = PropClip::from_json(&clip.to_json()).unwrap();
        assert_eq!(again.top(), &[group]);
        assert_eq!(again.elements.len(), 3);
        assert_eq!(PropClip::lights(&again.elements[&a]).len(), 1);
        assert!(PropClip::from_json("{}").is_err());
        assert!(PropClip::from_json("hello").is_err());
    }

    #[test]
    fn paste_links_an_element_that_still_exists() {
        let (mut prop, group, a, _) = prop_with_group();
        let other = prop.create_node("Other", None, false);
        let clip = PropClip::capture(&prop, &[a]).unwrap();
        assert!(clip.can_paste(&prop, other));
        assert!(!clip.can_paste(&prop, group), "already a child there");
        assert_eq!(clip.paste(&mut prop, other), vec![a]);
        assert_eq!(prop.element(a).unwrap().parents(), &[group, other]);
        assert_eq!(prop.light_count(), 2);
    }

    #[test]
    fn paste_after_cut_brings_the_element_back_with_its_lights() {
        let (mut prop, group, a, b) = prop_with_group();
        let other = prop.create_node("Other", None, false);
        let clip = PropClip::from_json(&PropClip::capture(&prop, &[a]).unwrap().to_json()).unwrap();
        let name = prop.element(a).unwrap().name().to_string();
        let order = prop.order(a);
        prop.remove_from_parent(a, group);
        assert!(!prop.contains(a));
        assert_eq!(clip.paste(&mut prop, other), vec![a]);
        let back = prop.element(a).unwrap();
        assert_eq!(back.name(), name);
        assert_eq!(back.lights().len(), 1);
        assert_eq!(back.parents(), &[other]);
        assert_eq!(prop.order(a), order, "its order was free again");
        assert!(prop.contains(b));
    }

    #[test]
    fn paste_of_a_cut_group_restores_the_tree_below_it() {
        let (mut prop, group, a, b) = prop_with_group();
        let root = prop.root_id();
        let holder = prop.create_node("Holder", None, false);
        let clip = PropClip::capture(&prop, &[group]).unwrap();
        prop.remove_from_parent(group, root);
        assert_eq!(prop.light_count(), 0);
        assert!(clip.can_paste(&prop, holder));
        clip.paste(&mut prop, holder);
        assert_eq!(prop.element(group).unwrap().children(), &[a, b]);
        assert_eq!(prop.light_count(), 2);
    }

    #[test]
    fn paste_as_new_makes_fresh_numbered_copies() {
        let (mut prop, group, a, _) = prop_with_group();
        let root = prop.root_id();
        let clip = PropClip::capture(&prop, &[group]).unwrap();
        assert!(clip.can_paste_as_new(&prop, root));
        let made = clip.paste_as_new(&mut prop, root, "Copy");
        assert_eq!(made.len(), 1);
        let copy = prop.element(made[0]).unwrap();
        assert_eq!(copy.name(), "Copy");
        let children = copy.children().to_vec();
        assert_eq!(children.len(), 2);
        assert_eq!(prop.element(children[0]).unwrap().name(), "Copy - 1");
        assert_eq!(prop.element(children[1]).unwrap().name(), "Copy - 2");
        assert_ne!(children[0], a);
        assert_eq!(prop.light_count(), 4);
        assert_eq!(prop.order(children[0]), 3);
        assert_eq!(prop.order(children[1]), 4);
        let old = &prop.element(a).unwrap().lights()[0];
        let new = &prop.element(children[0]).unwrap().lights()[0];
        assert_ne!(old.id, new.id);
        assert_eq!((old.x, old.y), (new.x, new.y));
    }

    #[test]
    fn light_nodes_do_not_paste_onto_the_root_or_a_group_of_groups() {
        let (mut prop, group, a, _) = prop_with_group();
        let root = prop.root_id();
        let clip = PropClip::capture(&prop, &[a]).unwrap();
        assert!(!clip.can_paste(&prop, root));
        let groups = PropClip::capture(&prop, &[group]).unwrap();
        assert!(!groups.can_paste(&prop, group), "a group of lights takes no groups");
        let empty = prop.create_node("Empty", None, false);
        assert!(groups.can_paste(&prop, empty));
        assert!(clip.can_paste(&prop, empty));
    }
}
