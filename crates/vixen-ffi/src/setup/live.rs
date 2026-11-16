//! For a window that edits elements from outside Display Setup, as the preview designer does after a template wizard: a working copy opened for one
//! operation, applied to the running show without saving, and the ids of what it made.

use super::DisplaySetupSession;
use uuid::Uuid;
use vixen_session::setup::tree::{format_path, Nid};

#[uniffi::export]
impl DisplaySetupSession {
    /// The working copy becomes the running display and nothing is written, so the caller's own save or reload decides. False when nothing was changed.
    /// What's playing stops, as on Display Setup's OK. Off the main thread.
    pub fn apply_live(&self) -> bool {
        self.inner.write().apply_live(&self.core)
    }

    /// Nodes are numbered as they're made and numbers aren't reused, so this taken before an operation marks what it then created.
    pub fn node_watermark(&self) -> u32 {
        self.inner.read().tree.slots() as u32
    }

    /// The element nodes made since `watermark` that are still there and have no children, in the order made: the Intelligent Fixture wizard's fixtures.
    pub fn leaves_created_since(&self, watermark: u32) -> Vec<String> {
        let s = self.inner.read();
        (watermark..s.tree.slots() as u32)
            .map(Nid)
            .filter(|n| s.tree.is_live(*n) && s.tree.node(*n).children.is_empty())
            .map(|n| s.tree.node(n).id.to_string())
            .collect()
    }

    /// Rows named by node ids, `"<uuid>/<uuid>"` from a root down, as rows here. One that isn't in the working copy any more is left out.
    pub fn rows_for_node_paths(&self, paths: Vec<String>) -> Vec<String> {
        let s = self.inner.read();
        paths
            .iter()
            .filter_map(|p| {
                let path = p.split('/').map(|id| s.tree.find(&Uuid::parse_str(id).ok()?)).collect::<Option<Vec<Nid>>>()?;
                s.tree.check_path(&path).then(|| format_path(&path))
            })
            .collect()
    }

    /// The other way: rows here as paths of node ids.
    pub fn node_paths_of(&self, rows: Vec<String>) -> Vec<String> {
        let s = self.inner.read();
        rows.iter()
            .filter_map(|r| {
                let path = s.tree.resolve(r)?;
                Some(path.iter().map(|n| s.tree.node(*n).id.to_string()).collect::<Vec<_>>().join("/"))
            })
            .collect()
    }
}
