//! Prints a summary of a Vixen data folder, and how long its largest sequence takes to load and save. Only reads
//! the folder: the timed save goes to a temporary folder beside this program, under `target/`.
//!
//!     cargo run --release -p vixen-io --example inspect -- "/path/to/Vixen 3"

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use vixen_io::{
    curve_library, gradient_library, load_application_data, load_sequence, load_system, sequence, DataPaths,
};

fn main() {
    let Some(root) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: inspect <data folder>");
        std::process::exit(2);
    };
    let paths = DataPaths::new(&root);
    println!("Data folder: {}", root.display());

    let started = Instant::now();
    match load_system(&paths) {
        Ok(s) => {
            println!(
                "System config v{} (file v{}), module store v{} (file v{}), loaded in {:.1} ms",
                s.version,
                s.source_version,
                s.module_store.version,
                s.module_store.source_version,
                started.elapsed().as_secs_f64() * 1e3
            );
            println!("  update interval {} ms, identity {}", s.default_update_interval, s.identity);
            let names: Vec<_> = s.tree.roots.iter().map(|r| s.tree.node(*r).name.as_str()).collect();
            println!("  {} nodes, {} channels, top level: {}", s.tree.len(), s.channels.len(), names.join(", "));
            println!(
                "  {} controllers, {} previews, {} output filters, {} patches, {} disabled devices",
                s.controllers.len(),
                s.previews.len(),
                s.filters.len(),
                s.patches.len(),
                s.disabled_devices.len()
            );
            for c in &s.controllers {
                println!("    controller {:?}: {} outputs", c.name, c.outputs.len());
            }
            println!(
                "  module store: {} type entries, {} instance entries",
                s.module_store.type_entries().len(),
                s.module_store.instance_entries().len()
            );
            let mut sets: Vec<_> = s.tree.color_sets.keys().map(String::as_str).collect();
            sets.sort_unstable();
            println!(
                "  libraries: {} curves, {} gradients, colour sets: {}",
                curve_library(&s.module_store).curves.len(),
                gradient_library(&s.module_store).gradients.len(),
                sets.join(", ")
            );
        }
        Err(e) => println!("System files: {e}"),
    }

    match load_application_data(&paths) {
        Ok(a) => {
            let recents = a.resolve_recent_sequences(&paths);
            let found = recents.iter().filter(|r| r.exists).count();
            println!("Recent sequences: {} listed, {found} found in this profile", recents.len());
        }
        Err(e) => println!("Application data: {e}"),
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(paths.sequence_dir())
        .map(|d| d.filter_map(|e| Some(e.ok()?.path())).filter(|p| p.extension().is_some_and(|e| e == "tim")).collect())
        .unwrap_or_default();
    files.sort();
    println!("Sequences: {}", files.len());
    let mut largest: Option<(u64, PathBuf)> = None;
    for path in &files {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if largest.as_ref().is_none_or(|(s, _)| size > *s) {
            largest = Some((size, path.clone()));
        }
        match load_sequence(path) {
            Ok(s) => {
                let mut kinds = BTreeMap::new();
                for e in &s.effects {
                    let name = vixen_effect_registry::by_type_id(&e.type_id).map_or("unknown", |d| d.name);
                    *kinds.entry(name).or_insert(0usize) += 1;
                }
                let kinds: Vec<_> = kinds.iter().map(|(k, n)| format!("{k} {n}")).collect();
                println!(
                    "  {}: v{} {:.1}s, {} effects, {} mark collections ({} marks), {} layers, media {:?}",
                    name_of(path),
                    s.source_version,
                    s.length.total_secs(),
                    s.effects.len(),
                    s.marks.len(),
                    s.marks.iter().map(|m| m.marks.len()).sum::<usize>(),
                    s.layers.layers.len(),
                    s.media.iter().map(|m| m.file_name.as_str()).collect::<Vec<_>>()
                );
                println!("      {}", kinds.join(", "));
            }
            Err(e) => println!("  {}: {e}", name_of(path)),
        }
    }

    if let Some((size, path)) = largest {
        timings(&path, size, &root);
    }
}

fn name_of(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Times the largest sequence, taking the best of several runs so a busy machine does not hide the real cost.
fn timings(path: &Path, size: u64, data_root: &Path) {
    const RUNS: usize = 20;
    let Some(temp) =
        std::env::current_exe().ok().and_then(|e| Some(e.parent()?.join(format!("inspect-{}", std::process::id()))))
    else {
        return;
    };
    if temp.starts_with(data_root) || std::fs::create_dir_all(&temp).is_err() {
        return;
    }
    let out = temp.join("timing.tim");
    let mut best_load = f64::MAX;
    let mut best_encode = f64::MAX;
    let mut best_save = f64::MAX;
    let mut effects = 0;
    for _ in 0..RUNS {
        let started = Instant::now();
        let Ok(seq) = load_sequence(path) else { return };
        best_load = best_load.min(started.elapsed().as_secs_f64());
        effects = seq.effects.len();

        let started = Instant::now();
        let Ok(text) = sequence::sequence_to_document(&seq) else { return };
        best_encode = best_encode.min(started.elapsed().as_secs_f64());
        std::hint::black_box(text);

        let _ = std::fs::remove_file(&out);
        let started = Instant::now();
        if vixen_io::save_sequence(&seq, &out).is_err() {
            return;
        }
        best_save = best_save.min(started.elapsed().as_secs_f64());
    }
    let _ = std::fs::remove_dir_all(&temp);
    println!(
        "Largest sequence: {} ({:.1} MB, {effects} effects). Best of {RUNS}: load {:.1} ms (includes reading and migrating), encode {:.1} ms, save to disk {:.1} ms",
        name_of(path),
        size as f64 / 1e6,
        best_load * 1e3,
        best_encode * 1e3,
        best_save * 1e3
    );
}
