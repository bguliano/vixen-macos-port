//! Compares this port's export of a sequence with an FSEQ file the original wrote, channel by channel. Twinkle, Dissolve
//! and Candle Flicker use unseeded random numbers in the original, so an output with one of them active can't be expected
//! to match; everything else should.

use crate::export::ScheduleSource;
use crate::render::{apply_load_rules, render_sequence, resolve_targets, RenderInputs};
use crate::show::build_output_graph;
use crate::SessionError;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use vixen_core::TimeSpan;
use vixen_engine::RenderProgress;
use vixen_export::fseq_read::FseqFile;
use vixen_export::run::IntervalSource;
use vixen_io::{load_sequence_with_store, DataPaths, SystemData};
use vixen_output::OutputAddress;

/// Effects whose output differs on every run of the original too.
pub const RANDOM_EFFECTS: [&str; 3] = ["Twinkle", "Dissolve", "Candle Flicker"];

/// One differing channel value.
#[derive(Clone, Debug)]
pub struct Difference {
    pub frame: usize,
    pub channel: usize,
    pub ours: u8,
    pub theirs: u8,
    /// `(effect name, start ms, length ms, instance id)` of each effect whose targets include this channel at that time.
    pub effects: Vec<(&'static str, f64, f64, uuid::Uuid)>,
}

/// The outcome for one sequence.
#[derive(Clone, Debug, Default)]
pub struct Comparison {
    pub frames: usize,
    pub identical_frames: usize,
    /// Every differing value, whether or not it's explained by a random effect.
    pub differing_values: usize,
    /// The values each combination of active effects accounts for.
    pub by_effects: BTreeMap<String, usize>,
    /// Differences with no random effect active.
    pub unexplained: usize,
    pub largest_unexplained: i32,
    /// The first of those.
    pub unexplained_samples: Vec<Difference>,
}

/// Compares one sequence, by name, with an FSEQ file.
pub fn compare_with_fseq(
    system: &SystemData,
    paths: &DataPaths,
    sequence_name: &str,
    fseq: &Path,
    keep_samples: usize,
) -> Result<Comparison, SessionError> {
    let store = vixen_io::system::effective_store(system);
    let tim = paths.sequence_dir().join(format!("{sequence_name}.tim"));
    let mut sequence = load_sequence_with_store(&tim, &store)?;
    crate::library_links::apply_library_links(&mut sequence, &store);
    apply_load_rules(&mut sequence, &system.tree);

    let mut original = FseqFile::open(fseq)?;
    let step_ms = original.header.step_ms as i64;
    let frame_size = original.header.channels as usize;
    let frames = original.header.frames as usize;
    let reference = original.read_all_frames()?;

    let clip = sequence.media.iter().enumerate().find_map(|(i, _)| {
        let path = sequence.media_path(i, paths)?;
        vixen_audio::AudioClip::load(&path).ok()
    });
    let module_root = paths.module_data_dir();
    let cancel = AtomicBool::new(false);
    let progress = RenderProgress::default();
    let inputs = RenderInputs {
        sequence: &sequence,
        tree: &system.tree,
        frame_ms: system.default_update_interval,
        audio: clip.as_ref().map(|c| c as &dyn vixen_effects::framework::AudioSource),
        module_data_root: Some(&module_root),
    };
    let rendered = render_sequence(&inputs, &cancel, &progress)?;
    let schedule = Arc::clone(&rendered.schedule);
    let graph = build_output_graph(system, &schedule.elements().ids);
    if graph.output_count() != frame_size {
        return Err(SessionError::Other(format!(
            "{} outputs in this profile, {frame_size} channels in the file",
            graph.output_count()
        )));
    }

    let mut element_of_output = Vec::with_capacity(frame_size);
    for (controller, c) in graph.controllers().iter().enumerate() {
        for output in 0..c.output_count() {
            element_of_output.push(graph.element_for_output(OutputAddress { controller, output }));
        }
    }
    // By target rather than by this run's intents: a random effect may have lit an element in the original's run that
    // it left dark in this one.
    let mut on_element: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut names = Vec::with_capacity(sequence.effects.len());
    for (index, node) in sequence.effects.iter().enumerate() {
        names.push(vixen_effect_registry::by_type_id(&node.type_id).map_or("?", |d| d.name));
        for target in resolve_targets(&system.tree, node) {
            for leaf in system.tree.distinct_leaves(target) {
                let Some(element) = system.tree.node(leaf).element_id else { continue };
                if let Some(i) = schedule.elements().index_of(&element) {
                    on_element.entry(i as usize).or_default().push(index);
                }
            }
        }
    }

    let mut source = ScheduleSource::all_controllers(schedule, graph);
    let mut row = vec![0u8; frame_size];
    let mut result = Comparison { frames, ..Comparison::default() };
    for frame in 0..frames {
        let time = TimeSpan::from_ms_i(frame as i64 * step_ms);
        source.sample(time, &mut row);
        let theirs = &reference[frame * frame_size..(frame + 1) * frame_size];
        if row == theirs {
            result.identical_frames += 1;
            continue;
        }
        for channel in (0..frame_size).filter(|c| row[*c] != theirs[*c]) {
            result.differing_values += 1;
            let active: Vec<usize> = element_of_output[channel]
                .and_then(|e| on_element.get(&e))
                .map(|list| {
                    list.iter()
                        .copied()
                        .filter(|i| {
                            let n = &sequence.effects[*i];
                            n.start <= time && time <= n.start + n.time_span
                        })
                        .collect()
                })
                .unwrap_or_default();
            let mut key: Vec<&str> = active.iter().map(|i| names[*i]).collect();
            key.sort();
            key.dedup();
            *result.by_effects.entry(key.join("+")).or_default() += 1;
            if key.iter().any(|name| RANDOM_EFFECTS.contains(name)) {
                continue;
            }
            result.unexplained += 1;
            result.largest_unexplained = result.largest_unexplained.max((row[channel] as i32 - theirs[channel] as i32).abs());
            if result.unexplained_samples.len() < keep_samples {
                result.unexplained_samples.push(Difference {
                    frame,
                    channel,
                    ours: row[channel],
                    theirs: theirs[channel],
                    effects: active
                        .iter()
                        .map(|i| {
                            let n = &sequence.effects[*i];
                            (names[*i], n.start.total_ms(), n.time_span.total_ms(), n.instance_id)
                        })
                        .collect(),
                });
            }
        }
    }
    Ok(result)
}
