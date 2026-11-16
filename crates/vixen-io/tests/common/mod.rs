//! Shared by the integration tests. Everything they write goes under Cargo's own temporary folder for tests.

use std::path::{Path, PathBuf};
use vixen_xml::XElement;

/// A fresh folder under `target/`.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn read(path: &Path) -> XElement {
    XElement::from_file(path).unwrap()
}
