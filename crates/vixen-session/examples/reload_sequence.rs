//! Loads one sequence the way the app does and saves it elsewhere.  reload_sequence <profile folder> <.tim> <output .tim>
use std::path::PathBuf;
use vixen_io::{load_sequence_with_store, load_system, save_sequence, system::effective_store, DataPaths};
use vixen_session::library_links::apply_library_links;
use vixen_session::render::apply_load_rules;

fn main() {
    let mut args = std::env::args().skip(1);
    let paths = DataPaths::new(PathBuf::from(args.next().expect("profile folder")));
    let (from, to) = (PathBuf::from(args.next().expect("sequence")), PathBuf::from(args.next().expect("output")));
    let system = load_system(&paths).expect("system");
    let store = effective_store(&system).into_owned();
    let mut sequence = load_sequence_with_store(&from, &store).expect("sequence");
    apply_library_links(&mut sequence, &store);
    apply_load_rules(&mut sequence, &system.tree);
    save_sequence(&sequence, &to).expect("save");
}
