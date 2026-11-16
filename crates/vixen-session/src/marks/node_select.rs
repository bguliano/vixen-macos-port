//! LipSync Mapping Node Select without its window, `LipSyncNodeSelect.cs`: which elements Tools > LipSync > Edit Element Face Mapping
//! hands to Face Setup. Spec F106 to F111.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::element::{ElementTree, NodeId};

/// The chosen list and the two checkboxes, as `Menu.cs:586-591` sets them: groups allowed, no recursion, no limit on the count.
#[derive(Clone, Debug)]
pub struct NodeSelect {
    chosen: Vec<Uuid>,
    names: HashSet<String>,
    allow_groups: bool,
    recurse: bool,
    user_touched: bool,
}

impl Default for NodeSelect {
    fn default() -> NodeSelect {
        NodeSelect::new()
    }
}

impl NodeSelect {
    pub fn new() -> NodeSelect {
        NodeSelect { chosen: Vec::new(), names: HashSet::new(), allow_groups: true, recurse: false, user_touched: false }
    }

    /// Node ids in the order they were added.
    pub fn chosen(&self) -> &[Uuid] {
        &self.chosen
    }

    pub fn allow_groups(&self) -> bool {
        self.allow_groups
    }

    pub fn recurse(&self) -> bool {
        self.recurse
    }

    pub fn set_recurse(&mut self, on: bool) {
        self.recurse = on;
    }

    /// `buttonAdd_Click`. KEEP: a tree row stands for its name, so every element with that name is a candidate, and a name already
    /// chosen is skipped, which leaves the first element of each name in the system's own order.
    pub fn add(&mut self, tree: &ElementTree, selected: &[Uuid]) {
        self.user_touched = true;
        let mut by_name: HashMap<&str, Vec<NodeId>> = HashMap::new();
        for (id, node) in tree.all_nodes() {
            by_name.entry(node.name.as_str()).or_default().push(id);
        }
        for id in selected {
            let Some(row) = tree.find(id) else { continue };
            let Some(same_name) = by_name.get(tree.node(row).name.as_str()) else { continue };
            for &node in same_name {
                self.add_node(tree, node);
            }
        }
    }

    /// `addElementNodes`, depth first in child order.
    fn add_node(&mut self, tree: &ElementTree, node: NodeId) {
        let mut stack = vec![node];
        while let Some(id) = stack.pop() {
            let n = tree.node(id);
            if (self.allow_groups || n.children.is_empty()) && !self.names.contains(&n.name) {
                self.names.insert(n.name.clone());
                self.chosen.push(n.id);
            }
            if self.recurse {
                stack.extend(n.children.iter().rev());
            }
        }
    }

    /// `buttonRemove_Click`. It counts as the user's first move even with nothing selected.
    pub fn remove(&mut self, tree: &ElementTree, ids: &[Uuid]) {
        self.user_touched = true;
        let gone: HashSet<&Uuid> = ids.iter().collect();
        self.chosen.retain(|id| !gone.contains(id));
        self.names = self.chosen.iter().filter_map(|id| tree.find(id)).map(|n| tree.node(n).name.clone()).collect();
    }

    /// `buttonReset_Click`. KEEP: unlike add and remove it is not the user's first move, so Allow Groups still clears after it.
    pub fn reset(&mut self) {
        self.chosen.clear();
        self.names.clear();
    }

    /// `allowGroupsCheckbox_CheckedChanged`: until the first add or remove, a change empties the list (there are no preset names to
    /// put back from this menu item). Afterwards it only decides what later adds take.
    pub fn set_allow_groups(&mut self, on: bool) {
        if on == self.allow_groups {
            return;
        }
        self.allow_groups = on;
        if !self.user_touched {
            self.reset();
        }
    }
}

impl crate::app::AppCore {
    /// Face Setup returned OK from Tools > LipSync > Edit Element Face Mapping, F111: every LipSync effect in the open sequences renders
    /// again against the new Face properties, and the system and module configurations are saved. As upstream, the sequences are not
    /// marked modified: nothing in them changed.
    pub fn face_mapping_applied(&self) -> usize {
        let tree = Arc::new(self.system.read().tree.clone());
        let library = self.lip_sync_library();
        let mut rendered = 0;
        for session in self.open_sequence_sessions() {
            let mut session = session.lock();
            session.element_properties_changed(Arc::clone(&tree));
            rendered += session.reset_lip_sync_effects(&library);
        }
        for result in [self.save_system_config(), self.save_module_config()] {
            if let Err(e) = result {
                crate::logging::error("LipSync", "Could not save the face mapping.", Some(&e.to_string()));
            }
        }
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two groups of three, the second also holding the first group's first leaf, and a lone element.
    fn tree() -> (ElementTree, Uuid, Uuid, Uuid) {
        let mut t = ElementTree::new();
        let a = t.add_string("A", 3);
        let b = t.add_string("B", 3);
        let shared = t.node(a).children[0];
        t.add_child(b, shared);
        let lone = t.add_element("Lone");
        (t.clone(), t.node(a).id, t.node(b).id, t.node(lone).id)
    }

    fn names(t: &ElementTree, s: &NodeSelect) -> Vec<String> {
        s.chosen().iter().map(|id| t.node(t.find(id).unwrap()).name.clone()).collect()
    }

    #[test]
    fn add_takes_groups_and_leaves_in_add_order() {
        let (t, a, _, lone) = tree();
        let mut s = NodeSelect::new();
        s.add(&t, &[lone, a]);
        assert_eq!(names(&t, &s), ["Lone", "A"]);
        s.add(&t, &[a]);
        assert_eq!(s.chosen().len(), 2);
    }

    #[test]
    fn groups_are_skipped_when_not_allowed() {
        let (t, a, _, lone) = tree();
        let mut s = NodeSelect::new();
        s.add(&t, &[]);
        s.set_allow_groups(false);
        s.add(&t, &[a, lone]);
        assert_eq!(names(&t, &s), ["Lone"]);
    }

    #[test]
    fn recursion_adds_descendants_once_each() {
        let (t, a, b, _) = tree();
        let mut s = NodeSelect::new();
        s.set_recurse(true);
        s.add(&t, &[a, b]);
        assert_eq!(names(&t, &s), ["A", "A-1", "A-2", "A-3", "B", "B-1", "B-2", "B-3"]);

        let mut leaves = NodeSelect::new();
        leaves.set_recurse(true);
        leaves.add(&t, &[]);
        leaves.set_allow_groups(false);
        leaves.add(&t, &[b]);
        assert_eq!(names(&t, &leaves), ["B-1", "B-2", "B-3", "A-1"]);
    }

    #[test]
    fn a_row_stands_for_every_element_of_its_name_and_the_first_wins() {
        let mut t = ElementTree::new();
        let first = t.add_element("Twin");
        let second = t.add_element("Twin");
        let mut s = NodeSelect::new();
        s.add(&t, &[t.node(second).id]);
        assert_eq!(s.chosen(), [t.node(first).id]);
    }

    #[test]
    fn remove_and_reset() {
        let (t, a, b, lone) = tree();
        let mut s = NodeSelect::new();
        s.add(&t, &[a, b, lone]);
        s.remove(&t, &[b]);
        assert_eq!(names(&t, &s), ["A", "Lone"]);
        s.add(&t, &[b]);
        assert_eq!(names(&t, &s), ["A", "Lone", "B"]);
        s.reset();
        assert!(s.chosen().is_empty());
        s.add(&t, &[a]);
        assert_eq!(names(&t, &s), ["A"]);
    }

    #[test]
    fn allow_groups_clears_only_before_the_first_add_or_remove() {
        let (t, a, _, _) = tree();
        let mut untouched = NodeSelect::new();
        untouched.set_allow_groups(false);
        assert!(!untouched.allow_groups());
        untouched.add(&t, &[a]);
        assert!(untouched.chosen().is_empty());
        untouched.set_allow_groups(true);
        untouched.add(&t, &[a]);
        untouched.set_allow_groups(false);
        assert_eq!(names(&t, &untouched), ["A"]);

        let mut after_remove = NodeSelect::new();
        after_remove.remove(&t, &[]);
        after_remove.add(&t, &[a]);
        after_remove.set_allow_groups(false);
        assert_eq!(after_remove.chosen().len(), 1);

        // Reset doesn't count as a move, but there's nothing left for the toggle to clear.
        let mut after_reset = NodeSelect::new();
        after_reset.reset();
        after_reset.set_allow_groups(false);
        after_reset.set_allow_groups(true);
        after_reset.add(&t, &[a]);
        assert_eq!(names(&t, &after_reset), ["A"]);
    }

    #[test]
    fn face_setup_reaches_open_sequences_and_renders_lip_sync_again() {
        use crate::dialogs::face::{face_helper_apply, FaceRow, COMPONENTS, PHONEMES};
        use crate::dialogs::world::{CoreWorld, WorldProvider};
        use crate::sequence::SequenceSession;
        use vixen_core::element::property_ids;
        use vixen_core::TimeSpan;
        const SET_LEVEL: Uuid = uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0");

        let root = std::env::temp_dir().join(format!("vixen-face-mapping-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let core = Arc::new(crate::app::AppCore::open(root.clone(), true).unwrap());
        let mouth = {
            let mut system = core.system.write();
            let node = system.tree.add_element("Mouth");
            system.tree.node(node).id
        };
        let mut sequence = SequenceSession::new_untitled(&core.session_environment());
        let span = TimeSpan::from_ms_i(100);
        let lip_sync = sequence.add_new_effect(fx_audio_state::lip_sync::DESCRIPTOR.type_id, mouth, "Mouth", TimeSpan::ZERO, span).unwrap();
        let other = sequence.add_new_effect(SET_LEVEL, mouth, "Mouth", TimeSpan::ZERO, span).unwrap();
        sequence.wait_for_renders();
        sequence.set_unmodified();
        let before = (sequence.effect(lip_sync).unwrap().generation, sequence.effect(other).unwrap().generation);
        let shared = Arc::new(parking_lot::Mutex::new(sequence));
        core.register_sequence_session(&shared);

        CoreWorld(Arc::clone(&core)).with_world(&mut |world| {
            let row = FaceRow {
                node: mouth,
                name: "Mouth".into(),
                phonemes: vec![true; PHONEMES.len()],
                components: vec![false; COMPONENTS.len()],
                color: vixen_core::Color::rgb(255, 0, 0),
            };
            face_helper_apply(world, &[row]);
        });
        let has_face = |s: &SequenceSession| s.tree().node(s.tree().find(&mouth).unwrap()).properties.iter().any(|p| p.type_id == property_ids::FACE);
        assert!(!has_face(&shared.lock()), "an open sequence keeps its own copy of the elements until told");

        assert_eq!(core.face_mapping_applied(), 1);
        let mut sequence = shared.lock();
        assert!(has_face(&sequence));
        sequence.wait_for_renders();
        let after = (sequence.effect(lip_sync).unwrap().generation, sequence.effect(other).unwrap().generation);
        assert!(after.0 > before.0, "the LipSync effect rendered again");
        assert_eq!(after.1, before.1, "nothing else did");
        assert!(sequence.effect(lip_sync).unwrap().is_rendered());
        assert!(!sequence.is_modified());
        // The system and module configurations were saved with the new property.
        let saved = std::fs::read_to_string(root.join("SystemData").join("ModuleStore.xml")).unwrap();
        assert!(saved.contains("PhonemeList"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn fifty_thousand_elements_add_at_once() {
        let mut t = ElementTree::new();
        let root = t.add_string("Big", 50_000);
        let ids: Vec<Uuid> = t.node(root).children.iter().map(|c| t.node(*c).id).collect();
        let mut s = NodeSelect::new();
        let started = std::time::Instant::now();
        s.add(&t, &ids);
        assert_eq!(s.chosen().len(), 50_000);
        assert!(started.elapsed().as_secs() < 5);
    }
}
