//! Compares this port's output for each sequence with the FSEQ the Windows version exported, channel by channel.
//!   cargo run --release -p vixen-session --example compare_fseq -- "<data folder>" [frame ms] [name filter]
//! Reads only. Point it at a copy of the data folder.
//! Diagnostic audio overrides: AUDIO_QUANTIZE=round16|trunc16|round8 or AUDIO_RAW_F32=<aligned interleaved float file>.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use vixen_core::TimeSpan;
use vixen_engine::RenderProgress;
use vixen_export::fseq_read::FseqFile;
use vixen_export::run::IntervalSource;
use vixen_io::{load_sequence_with_store, load_system, system::effective_store, DataPaths};
use vixen_session::export::ScheduleSource;
use vixen_session::render::{apply_load_rules, render_sequence, RenderInputs};
use vixen_session::show::build_output_graph;

struct Shifted {
    samples: Vec<f32>,
    rate: u32,
    duration: TimeSpan,
    channels: u32,
}

impl vixen_effects::framework::AudioSource for Shifted {
    fn samples(&self) -> &[f32] {
        &self.samples
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn duration(&self) -> TimeSpan {
        self.duration
    }
    fn interleaved(&self) -> &[f32] {
        &self.samples
    }
    fn channels(&self) -> u32 {
        self.channels
    }
}

fn modified(path: &std::path::Path) -> String {
    let Ok(meta) = std::fs::metadata(path) else {
        return "?".into();
    };
    let Ok(t) = meta.modified() else {
        return "?".into();
    };
    let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    format!("{}d", (secs / 86400) as i64 - 20000)
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("data folder"));
    let frame_override: Option<i32> = std::env::args().nth(2).and_then(|s| s.parse().ok());
    let filter = std::env::args().nth(3);
    let export_dir = std::env::var("FSEQ_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("Export"));
    let paths = DataPaths::new(&root);
    let system = load_system(&paths).expect("system");
    let store = effective_store(&system).into_owned();
    let module_root = paths.module_data_dir();

    let mut files: Vec<PathBuf> = std::fs::read_dir(&export_dir)
        .expect("export folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "fseq"))
        .collect();
    files.sort();

    for fseq in files {
        let name = fseq.file_stem().unwrap().to_string_lossy().to_string();
        if filter.as_ref().is_some_and(|f| !name.contains(f.as_str())) {
            continue;
        }
        let tim = paths.sequence_dir().join(format!("{name}.tim"));
        let mut sequence = match load_sequence_with_store(&tim, &store) {
            Ok(s) => s,
            Err(e) => {
                println!("{name}: no sequence: {e}");
                continue;
            }
        };
        vixen_session::library_links::apply_library_links(&mut sequence, &store);
        apply_load_rules(&mut sequence, &system.tree);
        let mut original = FseqFile::open(&fseq).expect("fseq");
        let h = original.header.clone();
        let reference = original.read_all_frames().expect("frames");
        let frame_size = h.channels as usize;

        let clip = sequence.media.iter().enumerate().find_map(|(i, _)| {
            let path = sequence.media_path(i, &paths)?;
            vixen_audio::AudioClip::load(&path).ok()
        });
        assert!(
            std::env::var_os("AUDIO_RAW_F32").is_none() || clip.is_some(),
            "AUDIO_RAW_F32 was requested but the sequence's original audio could not be loaded; refusing a comparison that silently ignores captured PCM"
        );
        // Diagnostic-only decoder comparison. Raw samples must already be
        // interleaved and aligned to this clip's timeline/rate/channel count.
        let clip = clip.map(|clip| {
            let raw = std::env::var_os("AUDIO_RAW_F32");
            let quantize = std::env::var("AUDIO_QUANTIZE").unwrap_or_default();
            if raw.is_none() && quantize.is_empty() {
                return clip;
            }
            let mut samples = raw
                .map(|path| {
                    let bytes = std::fs::read(path).expect("raw audio");
                    assert_eq!(bytes.len() % 4, 0);
                    bytes
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| clip.samples().to_vec());
            for sample in &mut samples {
                *sample = match quantize.as_str() {
                    "round8" => (*sample * 128.0).round().clamp(-128.0, 127.0) / 128.0,
                    "round16" => (*sample * 32768.0).round().clamp(-32768.0, 32767.0) / 32768.0,
                    "trunc16" => (*sample * 32768.0).trunc().clamp(-32768.0, 32767.0) / 32768.0,
                    "" => *sample,
                    _ => panic!("unknown AUDIO_QUANTIZE value"),
                };
            }
            vixen_audio::AudioClip::from_interleaved(samples, clip.channels(), clip.sample_rate())
        });
        let view = clip.as_ref().map(|c| c.mono_view());
        let shift: i64 = std::env::var("AUDIO_SHIFT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let shifted = view.as_ref().filter(|_| shift != 0).map(|v| {
            use vixen_effects::framework::AudioSource;
            let channels = v.channels() as i64;
            let source = v.interleaved();
            let mut samples = vec![0f32; source.len()];
            for (i, slot) in samples.iter_mut().enumerate() {
                let from = i as i64 + shift * channels;
                if from >= 0 && (from as usize) < source.len() {
                    *slot = source[from as usize];
                }
            }
            Shifted {
                samples,
                rate: v.sample_rate(),
                duration: v.duration(),
                channels: v.channels(),
            }
        });
        let cancel = AtomicBool::new(false);
        let progress = RenderProgress::default();
        let inputs = RenderInputs {
            sequence: &sequence,
            tree: &system.tree,
            frame_ms: frame_override.unwrap_or(system.default_update_interval),
            audio: shifted
                .as_ref()
                .map(|v| v as &dyn vixen_effects::framework::AudioSource)
                .or(view
                    .as_ref()
                    .map(|v| v as &dyn vixen_effects::framework::AudioSource)),
            module_data_root: Some(&module_root),
        };
        let rendered = render_sequence(&inputs, &cancel, &progress).expect("render");
        let schedule = rendered.schedule.clone();
        let graph = build_output_graph(&system, &schedule.elements().ids);
        let warnings = graph.warnings().len();
        let outputs = graph.output_count();
        let expected_frames = (sequence.length.total_ms() as i64) / h.step_ms as i64;
        if outputs != frame_size {
            println!("{name}: {outputs} outputs here, {frame_size} channels in the file");
        }

        // Which effects are on an element at a time, for blaming differences.
        let mut type_names: HashMap<_, &'static str> = HashMap::new();
        let element_of_output: Vec<Option<usize>> = {
            let mut v = Vec::new();
            for (c, controller) in graph.controllers().iter().enumerate() {
                for o in 0..controller.output_count() {
                    v.push(graph.element_for_output(vixen_output::OutputAddress {
                        controller: c,
                        output: o,
                    }));
                }
            }
            v
        };
        for node in &sequence.effects {
            if let Some(d) = vixen_effect_registry::by_type_id(&node.type_id) {
                type_names.insert(node.instance_id, d.name);
            }
        }
        // element index -> effects whose targets include it. Uses targets, not this run's intents, because a random
        // effect may have given another run's element a pulse that this run didn't.
        let mut by_element: HashMap<usize, Vec<(i64, i64, &'static str, usize)>> = HashMap::new();
        for (index, node) in sequence.effects.iter().enumerate() {
            let name = type_names.get(&node.instance_id).copied().unwrap_or("?");
            let (s, t) = (node.start.0, (node.start + node.time_span).0);
            for target in vixen_session::render::resolve_targets(&system.tree, node) {
                for leaf in system.tree.distinct_leaves(target) {
                    let Some(element) = system.tree.node(leaf).element_id else {
                        continue;
                    };
                    if let Some(i) = schedule.elements().index_of(&element) {
                        by_element
                            .entry(i as usize)
                            .or_default()
                            .push((s, t, name, index));
                    }
                }
            }
        }
        const RANDOM: [&str; 3] = ["Twinkle", "Dissolve", "Candle Flicker"];
        let show_details = std::env::var_os("SHOW_DETAILS").is_some();
        let mut details: Vec<String> = Vec::new();
        let mut plain_diffs = 0usize;
        let mut plain_max = 0i32;
        let mut plain_frames = std::collections::BTreeSet::new();

        let mut source = ScheduleSource::all_controllers(Arc::clone(&schedule), graph);
        let mut row = vec![0u8; outputs];
        let mut same_frames = 0usize;
        let mut diff_bytes = 0usize;
        let mut lit_ref = 0usize;
        let mut max_delta = 0i32;
        let mut delta_hist = [0usize; 4]; // 1, 2-3, 4-15, 16+
        let mut blame: BTreeMap<String, usize> = BTreeMap::new();
        let mut first: Vec<String> = Vec::new();
        let frames = (h.frames as usize).min(expected_frames.max(0) as usize);
        let lit_range: Option<(usize, usize)> = std::env::var("LIT_RANGE").ok().and_then(|v| {
            let (a, b) = v.split_once(':')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        });
        for frame in 0..frames {
            let t = TimeSpan::from_ms_i(frame as i64 * h.step_ms as i64);
            source.sample(t, &mut row);
            let r = &reference[frame * frame_size..(frame + 1) * frame_size];
            lit_ref += r.iter().filter(|b| **b != 0).count();
            if let Some((a, b)) = lit_range {
                if frame >= a && frame <= b {
                    let ol = row.iter().filter(|x| **x != 0).count();
                    let tl = r.iter().filter(|x| **x != 0).count();
                    let os: u64 = row.iter().map(|x| *x as u64).sum();
                    let ts: u64 = r.iter().map(|x| *x as u64).sum();
                    println!(
                        "   f{frame} t={:.3} ours lit {ol} sum {os} | theirs lit {tl} sum {ts}",
                        t.total_ms() / 1000.0
                    );
                }
            }
            if row[..outputs.min(frame_size)] == r[..outputs.min(frame_size)] {
                same_frames += 1;
                continue;
            }
            for c in 0..outputs.min(frame_size) {
                if row[c] != r[c] {
                    diff_bytes += 1;
                    let d = (row[c] as i32 - r[c] as i32).abs();
                    max_delta = max_delta.max(d);
                    delta_hist[match d {
                        1 => 0,
                        2..=3 => 1,
                        4..=15 => 2,
                        _ => 3,
                    }] += 1;
                    let on_element: Vec<&(i64, i64, &'static str, usize)> = element_of_output[c]
                        .and_then(|e| by_element.get(&e))
                        .map(|list| list.iter().filter(|x| x.0 <= t.0 && t.0 <= x.1).collect())
                        .unwrap_or_default();
                    let active = {
                        let mut names: Vec<&str> = on_element.iter().map(|x| x.2).collect();
                        names.sort();
                        names.dedup();
                        names.join("+")
                    };
                    let key = if active.is_empty() {
                        "(no effect here)".to_string()
                    } else {
                        active
                    };
                    if first.len() < 6 {
                        first.push(format!(
                            "f{frame} ch{c} ours {} theirs {} [{key}]",
                            row[c], r[c]
                        ));
                    }
                    if !RANDOM.iter().any(|n| key.contains(n)) {
                        plain_diffs += 1;
                        plain_max = plain_max.max(d);
                        plain_frames.insert(frame);
                    }
                    if show_details && details.len() < 60 && !RANDOM.iter().any(|n| key.contains(n))
                    {
                        let which: Vec<String> = on_element
                            .iter()
                            .map(|x| {
                                let n = &sequence.effects[x.3];
                                format!(
                                    "{} start {:.3} span {:.3} id {} layer {}",
                                    x.2,
                                    n.start.total_ms() / 1000.0,
                                    n.time_span.total_ms() / 1000.0,
                                    n.instance_id,
                                    sequence.layers.layer_index_for_effect(&n.instance_id)
                                )
                            })
                            .collect();
                        details.push(format!(
                            "f{frame} t={:.3}s ch{c} ours {} theirs {} :: {}",
                            t.total_ms() / 1000.0,
                            row[c],
                            r[c],
                            which.join(" | ")
                        ));
                    }
                    *blame.entry(key).or_default() += 1;
                }
            }
        }
        println!(
            "{name}: tim {} fseq {} | {frames}/{} frames ({} expected) | identical frames {} ({:.1}%) | diff bytes {} of {} lit, max delta {}, hist 1:{} 2-3:{} 4-15:{} 16+:{} | warnings {warnings} | problems {}",
            modified(&tim),
            modified(&fseq),
            h.frames,
            expected_frames,
            same_frames,
            same_frames as f64 * 100.0 / frames.max(1) as f64,
            diff_bytes,
            lit_ref,
            max_delta,
            delta_hist[0],
            delta_hist[1],
            delta_hist[2],
            delta_hist[3],
            rendered.problems.len(),
        );
        println!(
            "    not random: {plain_diffs} bytes in {} frames, largest difference {plain_max}, frames {:?}",
            plain_frames.len(),
            plain_frames.iter().take(30).collect::<Vec<_>>()
        );
        let mut b: Vec<_> = blame.into_iter().collect();
        b.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (k, v) in b.iter().take(8) {
            println!("    {v:>9}  {k}");
        }
        for f in &first {
            println!("      {f}");
        }
        for d in &details {
            println!("      > {d}");
        }
    }
}
