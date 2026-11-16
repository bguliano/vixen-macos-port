//! Loads a profile the way the app does and saves everything back: its sequences through the session's load rules, then its
//! system files. For checking the result with the original Vixen's loader (`tools/golden`, `--resave`). The profile is overwritten,
//! so pass a copy.  resave_show <profile folder>
use std::path::PathBuf;
use vixen_io::{load_sequence_with_store, load_system, save_sequence, save_system, system::effective_store, DataPaths};
use vixen_session::library_links::apply_library_links;
use vixen_session::render::apply_load_rules;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("profile folder"));
    let paths = DataPaths::new(root);
    let system = load_system(&paths).expect("system");
    let store = effective_store(&system).into_owned();
    let mut files: Vec<_> = std::fs::read_dir(paths.sequence_dir())
        .expect("sequence folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "tim"))
        .collect();
    files.sort();
    for file in files {
        let mut sequence = load_sequence_with_store(&file, &store).expect("sequence");
        apply_library_links(&mut sequence, &store);
        apply_load_rules(&mut sequence, &system.tree);
        save_sequence(&sequence, &file).expect("save sequence");
        println!("{}: {} effects", file.file_name().unwrap().to_string_lossy(), sequence.effects.len());
    }
    save_system(&system, &paths).expect("save system");
    println!("system saved");
}
