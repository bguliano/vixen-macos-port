//! Shared by the tests that read the user's real Vixen data.
//!
//! That data is read-only. Everything these tests write goes under Cargo's own temporary folder for tests, and
//! `TempDir` refuses any location inside the data root.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use vixen_xml::XElement;

const DEFAULT_DATA: &str = "/Users/developer/Documents/Vixen 3";

/// `VIXEN_TEST_DATA`, or the default install. `None` means the data is not on this machine, so a test should skip.
pub fn data_root() -> Option<PathBuf> {
    let root = std::env::var_os("VIXEN_TEST_DATA").map_or_else(|| PathBuf::from(DEFAULT_DATA), PathBuf::from);
    if root.is_dir() {
        Some(root)
    } else {
        eprintln!("skipping: {} is not here", root.display());
        None
    }
}

/// Whether the data is the install the exact counts in these tests describe.
pub fn is_default_data(root: &Path) -> bool {
    root == Path::new(DEFAULT_DATA)
}

/// A fresh folder under `target/`.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
        if let Some(root) = data_root() {
            assert!(!dir.starts_with(&root), "tests must not write into the real data");
        }
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

/// The `.tim` files directly in the Sequence folder: the current ones, not the backups.
pub fn current_sequences(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("Sequence"))
        .unwrap()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "tim"))
        .collect();
    files.sort();
    files
}

/// Every file that could hold a sequence: backups by version, `_backup` copies, and the `auto_backup` folder.
pub fn all_sequence_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in [root.join("Sequence"), root.join("Sequence").join("auto_backup")] {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if p.is_file() && name.contains(".tim") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
}

pub fn read(path: &Path) -> XElement {
    XElement::from_file(path).unwrap()
}

/// The root with `version` replaced, for comparing a migrated file with its original.
pub fn with_version(mut e: XElement, version: &str) -> XElement {
    e.set_attr("version", version);
    e
}

pub fn file_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}
