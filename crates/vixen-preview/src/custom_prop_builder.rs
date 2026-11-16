//! Port of PreviewCustomPropBuilder.cs: elements and a Custom Prop shape from a prop's element models.
//!
//! The Custom Prop Editor's model is adapted to [`ElementModel`] by the session. The questions the C# asks
//! (dimming curve, colour setup) and the face and state properties are the host's, because they need dialogs
//! and property modules that `vixen_core` does not have.

use crate::shape::{CustomPropLightNode, Shape};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use vixen_core::element::{property_ids, PropertyData, PropertyInstance};
use vixen_core::{ElementTree, NodeId};

/// The parts of `VixenModules.App.CustomPropEditor.Model.ElementModel` the builder uses.
#[derive(Clone, Debug, Default)]
pub struct ElementModel {
    pub id: Uuid,
    pub name: String,
    pub order: i32,
    pub light_size: i32,
    pub lights: Vec<(f64, f64)>,
    pub children: Vec<ElementModel>,
}

impl ElementModel {
    /// `IsLightNode`.
    pub fn is_light_node(&self) -> bool {
        !self.lights.is_empty()
    }
}

/// `NamingUtilities.Uniquify`.
fn uniquify(names: &HashSet<String>, name: &str) -> String {
    if !names.contains(name) {
        return name.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{name} - {n}");
        if !names.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// The `{1}` tokens in a name, as `Regex(@"{\d+}")` finds them, in order.
fn tokens(name: &str) -> Vec<&str> {
    let bytes = name.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 && j < bytes.len() && bytes[j] == b'}' {
                out.push(&name[i..=j]);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `TokenizeName`. `substitute` is `GetSubstitutionString`, the dialog asking for a token's value, and is
/// asked once per distinct token.
fn tokenize(name: &str, lookup: &mut HashMap<String, String>, substitute: &mut impl FnMut(&str) -> String) -> String {
    let mut out = name.to_string();
    for t in tokens(name) {
        let value = lookup.entry(t.to_string()).or_insert_with(|| substitute(t)).clone();
        out = out.replace(t, &value);
    }
    out
}

struct Builder<'a, F> {
    tree: &'a mut ElementTree,
    map: HashMap<Uuid, NodeId>,
    names: HashSet<String>,
    lookup: HashMap<String, String>,
    substitute: F,
    prop: Shape,
}

impl<F: FnMut(&str) -> String> Builder<'_, F> {
    /// `FindOrCreateElementNode`.
    fn find_or_create(&mut self, model: &ElementModel, parent: NodeId) -> NodeId {
        if let Some(node) = self.map.get(&model.id) {
            //Our element exists, so add this one as a child.
            self.tree.add_child(parent, *node);
            return *node;
        }
        //Validate we have a name
        let raw = if model.name.is_empty() { "Unnamed" } else { model.name.as_str() };
        let name = tokenize(raw, &mut self.lookup, &mut self.substitute);
        let name = uniquify(&self.names, &name);
        //We have not created our element yet
        let node = self.tree.add_element(&name);
        self.tree.add_child(parent, node);
        self.map.insert(model.id, node);
        self.names.insert(name);
        if model.is_light_node() {
            self.tree.node_mut(node).properties.push(PropertyInstance {
                type_id: property_ids::ORDER,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Order { order: model.order },
            });
            let id = self.tree.node(node).id;
            self.prop.add_light_nodes(&CustomPropLightNode { node: id, light_size: model.light_size, lights: model.lights.clone() });
        }
        node
    }

    /// `CreateElementsForChildren`.
    fn create_children(&mut self, parent: NodeId, model: &ElementModel) {
        for child in &model.children {
            let node = self.find_or_create(child, parent);
            self.create_children(node, child);
        }
    }
}

/// The distinct name tokens of a prop, in the order [`build_custom_prop`] asks for them, so a host can ask before it builds.
pub fn name_tokens(root: &ElementModel) -> Vec<String> {
    fn walk<'a>(model: &'a ElementModel, seen: &mut HashSet<Uuid>, out: &mut Vec<String>) {
        for child in &model.children {
            if seen.insert(child.id) {
                add(&child.name, out);
            }
            walk(child, seen, out);
        }
    }
    fn add(name: &str, out: &mut Vec<String>) {
        for t in tokens(name) {
            if !out.iter().any(|o| o == t) {
                out.push(t.to_string());
            }
        }
    }
    let mut out = Vec::new();
    add(&root.name, &mut out);
    walk(root, &mut HashSet::from([root.id]), &mut out);
    out
}

/// `PreviewCustomPropBuilder.CreateAsync`: adds the elements to `tree` and returns the shape and the root node.
/// Give the shape to [`crate::session::DesignSession::add_custom_prop`] after giving the session the tree.
pub fn build_custom_prop(root: &ElementModel, tree: &mut ElementTree, zoom_level: f64, substitute: impl FnMut(&str) -> String) -> (Shape, NodeId) {
    let (shape, node, _) = build_custom_prop_mapped(root, tree, zoom_level, substitute);
    (shape, node)
}

/// Also returns the model-id → created-node map used by face and State property import. Shared models have
/// one entry even when the hierarchy reaches them through several parents.
pub fn build_custom_prop_mapped(
    root: &ElementModel,
    tree: &mut ElementTree,
    zoom_level: f64,
    substitute: impl FnMut(&str) -> String,
) -> (Shape, NodeId, HashMap<Uuid, NodeId>) {
    //Optimize the name check for performance. We know we are going to create a bunch of them
    let names: HashSet<String> = tree.all_nodes().map(|(_, n)| n.name.clone()).collect();
    let mut b = Builder { tree, map: HashMap::new(), names, lookup: HashMap::new(), substitute, prop: Shape::new_custom_prop(zoom_level) };
    let name = tokenize(&root.name, &mut b.lookup, &mut b.substitute);
    let name = uniquify(&b.names, &name);
    let root_node = b.tree.add_node(Uuid::new_v4(), &name, None);
    b.prop.name = Some(name.clone());
    b.names.insert(name);
    b.map.insert(root.id, root_node);
    b.create_children(root_node, root);
    let mut prop = b.prop;
    prop.layout();
    (prop, root_node, b.map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_tokenized_and_made_unique_and_lights_linked() {
        let mut tree = ElementTree::new();
        tree.add_element("Tree 1 Top");
        let model = ElementModel {
            id: Uuid::from_u128(1),
            name: "Tree {1}".to_string(),
            children: vec![
                ElementModel { id: Uuid::from_u128(2), name: "Tree {1} Top".to_string(), order: 1, light_size: 3, lights: vec![(1.0, 2.0), (3.0, 4.0)], children: vec![] },
                ElementModel { id: Uuid::from_u128(3), name: String::new(), order: 2, light_size: 4, lights: vec![(5.0, 6.0)], children: vec![] },
            ],
            ..ElementModel::default()
        };
        let mut asked = Vec::new();
        let (prop, root) = build_custom_prop(&model, &mut tree, 1.0, |t| {
            asked.push(t.to_string());
            "1".to_string()
        });
        assert_eq!(asked, ["{1}"], "each distinct token is asked once");
        assert_eq!(name_tokens(&model), ["{1}"]);
        assert_eq!(tree.node(root).name, "Tree 1");
        assert_eq!(prop.name(), Some("Tree 1"));
        let names: Vec<&str> = tree.node(root).children.iter().map(|c| tree.node(*c).name.as_str()).collect();
        assert_eq!(names, ["Tree 1 Top - 2", "Unnamed"]);
        let pixels = prop.pixels_vec();
        assert_eq!(pixels.len(), 3);
        assert_eq!(pixels[0].location, (1.0, 2.0));
        assert_eq!(pixels[2].size, 4);
        assert_eq!(pixels[0].node_id, tree.node(tree.node(root).children[0]).id);
        let bounds = prop.custom_prop_bounds().unwrap();
        assert_eq!((bounds.x, bounds.y, bounds.right(), bounds.bottom()), (1.0, 2.0, 9.0, 10.0));
    }

    #[test]
    fn tokens_are_listed_in_the_order_the_build_asks() {
        let leaf = |id: u128, name: &str| ElementModel { id: Uuid::from_u128(id), name: name.to_string(), light_size: 3, lights: vec![(1.0, 1.0)], ..ElementModel::default() };
        let model = ElementModel {
            id: Uuid::from_u128(1),
            name: "Arch {2}".to_string(),
            children: vec![
                ElementModel { id: Uuid::from_u128(2), name: "{1} left {x} {}".to_string(), children: vec![leaf(4, "{3}{2}")], ..ElementModel::default() },
                leaf(5, "{10}"),
                leaf(4, "{3}{2}"),
            ],
            ..ElementModel::default()
        };
        let listed = name_tokens(&model);
        assert_eq!(listed, ["{2}", "{1}", "{3}", "{10}"]);
        let mut asked = Vec::new();
        build_custom_prop(&model, &mut ElementTree::new(), 1.0, |t| {
            asked.push(t.to_string());
            String::new()
        });
        assert_eq!(asked, listed);
    }

    #[test]
    fn a_model_reached_twice_is_one_element_with_two_parents() {
        let shared = ElementModel { id: Uuid::from_u128(9), name: "Shared".to_string(), light_size: 3, lights: vec![(1.0, 1.0)], ..ElementModel::default() };
        let model = ElementModel {
            id: Uuid::from_u128(1),
            name: "Root".to_string(),
            children: vec![
                ElementModel { id: Uuid::from_u128(2), name: "A".to_string(), children: vec![shared.clone()], ..ElementModel::default() },
                ElementModel { id: Uuid::from_u128(3), name: "B".to_string(), children: vec![shared], ..ElementModel::default() },
            ],
            ..ElementModel::default()
        };
        let mut tree = ElementTree::new();
        let (prop, _) = build_custom_prop(&model, &mut tree, 1.0, |_| String::new());
        assert_eq!(prop.pixel_count(), 1);
        let shared_node = tree.find_by_element(&tree.all_nodes().find(|(_, n)| n.name == "Shared").unwrap().1.element_id.unwrap()).unwrap();
        assert_eq!(tree.node(shared_node).parents.len(), 2);
    }
}
