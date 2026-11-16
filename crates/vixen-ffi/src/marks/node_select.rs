//! LipSync Mapping Node Select, Tools > LipSync > Edit Element Face Mapping. Spec F106 to F111. The logic is
//! `vixen_session::marks::node_select`; this only converts.

use crate::core::VixenCore;
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;
use vixen_session::app::AppCore;
use vixen_session::marks::node_select::NodeSelect;

#[derive(uniffi::Record, Clone, PartialEq, Debug)]
pub struct NodeSelectRow {
    /// The row's path from a root, since an element can sit under several groups. The node id is its last part.
    pub id: String,
    pub node: String,
    pub name: String,
    pub has_children: bool,
}

/// One showing of the window.
#[derive(uniffi::Object)]
pub struct FaceNodeSelect {
    core: Arc<AppCore>,
    state: Mutex<NodeSelect>,
}

fn node_of(row: &str) -> Option<Uuid> {
    Uuid::parse_str(row.rsplit('/').next()?).ok()
}

#[uniffi::export]
impl VixenCore {
    pub fn face_node_select(&self) -> Arc<FaceNodeSelect> {
        Arc::new(FaceNodeSelect { core: Arc::clone(&self.inner), state: Mutex::new(NodeSelect::new()) })
    }
}

#[uniffi::export]
impl FaceNodeSelect {
    /// The element tree, a level at a time: the roots for `None`.
    pub fn children(&self, parent: Option<String>) -> Vec<NodeSelectRow> {
        let system = self.core.system.read();
        let tree = &system.tree;
        let ids = match &parent {
            None => tree.roots.clone(),
            Some(row) => match node_of(row).and_then(|id| tree.find(&id)) {
                Some(node) => tree.node(node).children.clone(),
                None => return Vec::new(),
            },
        };
        ids.into_iter()
            .map(|id| {
                let node = tree.node(id);
                let uuid = node.id.to_string();
                NodeSelectRow {
                    id: match &parent {
                        Some(p) => format!("{p}/{uuid}"),
                        None => uuid.clone(),
                    },
                    node: uuid,
                    name: node.name.clone(),
                    has_children: !node.children.is_empty(),
                }
            })
            .collect()
    }

    /// The right-hand list, in the order added. Each row's `id` is its node id.
    pub fn chosen(&self) -> Vec<NodeSelectRow> {
        let system = self.core.system.read();
        let tree = &system.tree;
        self.state
            .lock()
            .chosen()
            .iter()
            .filter_map(|id| {
                let node = tree.node(tree.find(id)?);
                Some(NodeSelectRow { id: id.to_string(), node: id.to_string(), name: node.name.clone(), has_children: false })
            })
            .collect()
    }

    /// `->`, with the tree rows selected.
    pub fn add(&self, rows: Vec<String>) {
        let ids: Vec<Uuid> = rows.iter().filter_map(|r| node_of(r)).collect();
        // Match chosen(): the asynchronous list load can overlap this call, and a queued system writer
        // otherwise turns the opposite order into a deadlock (state -> system -> state).
        let system = self.core.system.read();
        self.state.lock().add(&system.tree, &ids);
    }

    /// `<-`, with the chosen rows selected.
    pub fn remove(&self, nodes: Vec<String>) {
        let ids: Vec<Uuid> = nodes.iter().filter_map(|r| node_of(r)).collect();
        let system = self.core.system.read();
        self.state.lock().remove(&system.tree, &ids);
    }

    pub fn reset(&self) {
        self.state.lock().reset();
    }

    pub fn allow_groups(&self) -> bool {
        self.state.lock().allow_groups()
    }

    pub fn set_allow_groups(&self, on: bool) {
        self.state.lock().set_allow_groups(on);
    }

    pub fn recurse(&self) -> bool {
        self.state.lock().recurse()
    }

    pub fn set_recurse(&self, on: bool) {
        self.state.lock().set_recurse(on);
    }

    /// What OK hands to Face Setup. Empty means OK does nothing more.
    pub fn chosen_nodes(&self) -> Vec<String> {
        self.state.lock().chosen().iter().map(Uuid::to_string).collect()
    }

    /// Face Setup returned OK: LipSync effects in the open sequences render again and the configuration is saved. The count is the
    /// effects rendered.
    pub fn face_setup_finished(&self) -> u32 {
        self.core.face_mapping_applied() as u32
    }
}
