//! Loads each sequence of a Vixen data folder, renders it and evaluates the schedule, reading only.
//!   cargo run --release -p vixen-session --example render_show -- "<data folder>"

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use vixen_core::TimeSpan;
use vixen_engine::schedule::EvalScratch;
use vixen_engine::RenderProgress;
use vixen_io::{load_sequence_with_store, load_system, system::effective_store, DataPaths};
use vixen_session::render::{apply_load_rules, render_sequence, RenderInputs};

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("data folder"));
    let paths = DataPaths::new(&root);
    let t = Instant::now();
    let system = load_system(&paths).expect("system config");
    println!(
        "system: {} nodes, {} controllers, {} filters, {} patches, update interval {} ms ({:.0} ms)",
        system.tree.len(),
        system.controllers.len(),
        system.filters.len(),
        system.patches.len(),
        system.default_update_interval,
        t.elapsed().as_secs_f64() * 1000.0
    );
    let store = effective_store(&system);
    let module_root = paths.module_data_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(paths.sequence_dir())
        .expect("sequence folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tim"))
        .collect();
    files.sort();
    for file in files {
        let name = file.file_stem().unwrap().to_string_lossy().to_string();
        let t = Instant::now();
        let mut sequence = match load_sequence_with_store(&file, &store) {
            Ok(s) => s,
            Err(e) => {
                println!("{name}: LOAD FAILED: {e}");
                continue;
            }
        };
        vixen_session::library_links::apply_library_links(&mut sequence, &store);
        apply_load_rules(&mut sequence, &system.tree);
        let load_ms = t.elapsed().as_secs_f64() * 1000.0;
        let clip = sequence.media.iter().enumerate().find_map(|(i, _)| {
            let path = sequence.media_path(i, &paths)?;
            vixen_audio::AudioClip::load(&path).ok()
        });
        let view = clip.as_ref().map(|c| c.mono_view());
        let t = Instant::now();
        let cancel = AtomicBool::new(false);
        let progress = RenderProgress::default();
        let inputs = RenderInputs {
            sequence: &sequence,
            tree: &system.tree,
            frame_ms: system.default_update_interval,
            audio: view.as_ref().map(|v| v as &dyn vixen_effects::framework::AudioSource),
            module_data_root: Some(&module_root),
        };
        let rendered = render_sequence(&inputs, &cancel, &progress).expect("render");
        let render_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let mut scratch = EvalScratch::default();
        let mut out = Vec::new();
        let mut frames = 0u64;
        let mut lit = 0u64;
        let mut at = TimeSpan::ZERO;
        let step = TimeSpan::from_ms_i(25);
        while at + step <= sequence.length {
            if rendered.schedule.evaluate(at, &mut scratch, &mut out) {
                lit += 1;
            }
            frames += 1;
            at = at + step;
        }
        let eval_ms = t.elapsed().as_secs_f64() * 1000.0;
        println!(
            "{name}: v{} {} effects, {} layers, {} entries, audio {} | load {load_ms:.0} ms, render {render_ms:.0} ms, {} frames ({} active) evaluated in {eval_ms:.0} ms ({:.1} us/frame), {} problems",
            sequence.source_version,
            sequence.effects.len(),
            sequence.layers.layers.len(),
            rendered.schedule.entry_count(),
            view.is_some(),
            frames,
            lit,
            eval_ms * 1000.0 / frames.max(1) as f64,
            rendered.problems.len(),
        );
        for p in rendered.problems.iter().take(5) {
            println!("    {} {}", p.effect, p.message);
        }
    }
}
