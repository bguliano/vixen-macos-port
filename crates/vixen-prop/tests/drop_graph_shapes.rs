//! Focused graph contracts absent from the accepted simple-tree pointer suites.
//! Expectations come from upstream ElementTreeViewModel.CanAcceptData/Drop and
//! PropModelServices' add-before-remove behavior, not native-input simulation.
use std::path::PathBuf;

use serde_json::Value;
use uuid::Uuid;
use vixen_core::Color;
use vixen_prop::{manifest_json, read_package, write_package, ElementModelType, Prop, PropEditor, PropEditorSettings, RowPath, StateDefinitionModel};

const NOW: &str = "2026-11-12T12:00:00Z";

struct Fixture {
    directory: PathBuf,
    editor: PropEditor,
    root: Uuid,
    a: Uuid,
    b: Uuid,
    alias: Uuid,
    target: Uuid,
    empty: Uuid,
    x: Uuid,
    y: Uuid,
    z: Uuid,
}

impl Fixture {
    fn new() -> Self {
        let mut prop = Prop::with_size("Shared graph Café", 8, 8, "Private graph test", NOW);
        let root = prop.root_id();
        let a = prop.create_node("A", None, false);
        let b = prop.create_node("B", None, false);
        let alias = prop.create_node("Alias", None, false);
        let target = prop.create_node("Target", None, false);
        let empty = prop.create_node("Empty", None, false);
        let x = prop.add_light_node(Some(a), 1.25, 2.5, Some(11), 3, Some("X")).unwrap();
        let y = prop.add_light_node(Some(b), 3.75, 4.5, Some(23), 4, Some("Y")).unwrap();
        let z = prop.add_light_node(Some(target), 5.25, 6.5, Some(37), 5, Some("Z")).unwrap();
        prop.add_to_parent(x, alias);
        let mut state = StateDefinitionModel::create_default("Shared State");
        state.description = "Keep IDs, references, color, and order".into();
        state.items[0].element_ids = vec![x, y];
        state.items[0].color = Color::rgb(17, 119, 203);
        prop.element_mut(root).unwrap().model_type = ElementModelType::Model;
        prop.element_mut(root).unwrap().state_definitions.push(state);
        let directory = std::env::temp_dir().join(format!("vixen-prop-graph-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("private.prp");
        std::fs::write(&path, write_package(&prop).unwrap()).unwrap();
        let editor = PropEditor::open(&path, PropEditorSettings::default(), NOW).unwrap();
        assert!(!editor.is_dirty());
        Self { directory, editor, root, a, b, alias, target, empty, x, y, z }
    }

    fn path(&self, tail: &[Uuid]) -> RowPath {
        std::iter::once(self.root).chain(tail.iter().copied()).collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) { std::fs::remove_dir_all(&self.directory).unwrap(); }
}

fn document(prop: &Prop) -> Value {
    let mut value: Value = serde_json::from_str(&manifest_json(prop).unwrap()).unwrap();
    // Records are serialized breadth-first, so moves change their file order.
    // Compare their identities exactly; preserve every childIds/State/light order.
    value["elements"].as_array_mut().unwrap().sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    value
}

fn children(expected: &mut Value, parent: Uuid, ids: &[Uuid]) {
    let node = expected["elements"].as_array_mut().unwrap().iter_mut().find(|e| e["id"] == parent.to_string()).unwrap();
    node["childIds"] = ids.iter().map(|id| Value::String(id.to_string())).collect();
}

fn parents(prop: &Prop, node: Uuid, expected: &[Uuid]) {
    let mut got = prop.element(node).unwrap().parents().to_vec();
    let mut want = expected.to_vec();
    got.sort(); want.sort();
    assert_eq!(got, want);
}

fn reject_unchanged(f: &mut Fixture, dragged: Vec<RowPath>, destination: RowPath, index: Option<usize>) {
    f.editor.set_tree_selection(dragged.clone());
    let before = document(f.editor.prop());
    let revision = f.editor.revision();
    let lights = f.editor.light_selection().to_vec();
    let order = f.editor.order_selection().to_vec();
    assert!(!f.editor.can_drop(&dragged, &destination, index));
    assert!(!f.editor.drop_rows(&dragged, &destination, index, false));
    assert_eq!(document(f.editor.prop()), before);
    assert_eq!(f.editor.tree_selection(), dragged);
    assert_eq!(f.editor.light_selection(), lights);
    assert_eq!(f.editor.order_selection(), order);
    assert_eq!(f.editor.revision(), revision);
    assert!(!f.editor.is_dirty());
    assert_eq!(f.editor.undo_name(), None);
    assert_eq!(f.editor.redo_name(), None);
    assert_eq!(document(&read_package(&write_package(f.editor.prop()).unwrap()).unwrap()), before);
}

#[test]
fn mixed_inside_outside_insertion_refuses_the_entire_move() {
    // CanAcceptData refuses an insertion when the destination already contains
    // one selected model and another selected row comes from a different parent.
    for index in [0, 1] {
        let mut f = Fixture::new();
        let dragged = vec![f.path(&[f.a, f.x]), f.path(&[f.b, f.y])];
        let destination = f.path(&[f.a]);
        reject_unchanged(&mut f, dragged, destination, Some(index));
    }
}

#[test]
fn already_linked_alias_rejects_center_and_insertion_without_partial_move() {
    // X is selected through Alias, but A already owns that same model identity.
    // Y must not be moved partially when that one conflicting alias is refused.
    for index in [None, Some(0), Some(1)] {
        let mut f = Fixture::new();
        let dragged = vec![f.path(&[f.alias, f.x]), f.path(&[f.b, f.y])];
        let destination = f.path(&[f.a]);
        reject_unchanged(&mut f, dragged, destination, index);
    }
}

#[test]
fn mixed_group_and_light_selection_preserves_graph_history_and_selection() {
    for index in [None, Some(0)] {
        let mut f = Fixture::new();
        let dragged = vec![f.path(&[f.a]), f.path(&[f.b, f.y])];
        let destination = f.path(&[f.empty]);
        reject_unchanged(&mut f, dragged, destination, index);
    }
}

fn verify_move(reverse: bool, index: Option<usize>) {
    let mut f = Fixture::new();
    let dragged = vec![f.path(&[f.a, f.x]), f.path(&[f.b, f.y])];
    let destination = f.path(&[f.target]);
    f.editor.set_tree_selection(dragged.clone());
    let before = document(f.editor.prop());
    let mut expected = before.clone();
    let moved = if reverse { vec![f.y, f.x] } else { vec![f.x, f.y] };
    let target_children = if index == Some(0) { vec![moved[0], moved[1], f.z] } else { vec![f.z, moved[0], moved[1]] };
    children(&mut expected, f.a, &[]);
    children(&mut expected, f.b, &[]);
    children(&mut expected, f.target, &target_children);
    if reverse && index == Some(0) {
        // Upstream ElementModel.LightSize is the first leaf's size, not a fixed
        // group field. Moving Y (size4) ahead of Z (size5) changes this aggregate.
        expected["elements"].as_array_mut().unwrap().iter_mut().find(|e| e["id"] == f.target.to_string()).unwrap()["lightSize"] = Value::from(4);
    }
    assert!(f.editor.can_drop(&dragged, &destination, index));
    assert!(f.editor.drop_rows(&dragged, &destination, index, reverse));
    assert_eq!(document(f.editor.prop()), expected);
    assert_eq!(f.editor.tree_selection(), moved.iter().map(|id| f.path(&[f.target, *id])).collect::<Vec<_>>());
    parents(f.editor.prop(), f.x, &[f.alias, f.target]);
    parents(f.editor.prop(), f.y, &[f.target]);
    assert_eq!(f.editor.prop().element(f.alias).unwrap().children(), &[f.x]);
    assert_eq!(f.editor.prop().light_count(), 3);
    assert_eq!(f.editor.order_rows().iter().map(|r| (r.id, r.order)).collect::<Vec<_>>(), vec![(f.x, 11), (f.y, 23), (f.z, 37)]);
    assert_eq!(f.editor.undo_name(), Some("Move"));
    assert!(f.editor.is_dirty());
    let reloaded = read_package(&write_package(f.editor.prop()).unwrap()).unwrap();
    assert_eq!(document(&reloaded), expected);
    parents(&reloaded, f.x, &[f.alias, f.target]);
    assert!(f.editor.undo());
    assert_eq!(document(f.editor.prop()), before);
    assert_eq!(f.editor.tree_selection(), dragged);
    assert!(!f.editor.is_dirty());
    assert_eq!(f.editor.redo_name(), Some("Move"));
    assert!(f.editor.redo());
    assert_eq!(document(f.editor.prop()), expected);
    assert!(f.editor.is_dirty());
}

#[test]
fn cross_parent_center_move_preserves_unselected_alias_and_all_model_data() { verify_move(false, None); }

#[test]
fn reversed_cross_parent_insertion_preserves_alias_and_exact_block_order() { verify_move(true, Some(0)); }
