//! Read-only diagnostics for Vertical Meter's real-show FSEQ differences.
//! Writes only the explicitly supplied diagnostic folder, never the source profile.
//! cargo run -p vixen-session --example vertical_meter_probe -- <profile> <output> [aligned raw f32 inputs...]
use std::{collections::BTreeSet, path::PathBuf, sync::atomic::AtomicBool};
use vixen_effects::framework::RenderCtx;
use vixen_io::{load_sequence_with_store, load_system, system::effective_store, DataPaths};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let paths = DataPaths::new(PathBuf::from(&args[1]));
    let output = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&output).unwrap();
    let system = load_system(&paths).unwrap();
    let fseq_path = paths.root().join("Export/Coldest Winter.fseq");
    let header = vixen_export::fseq_read::FseqFile::open(&fseq_path)
        .unwrap()
        .header;
    let step_ms = f64::from(header.step_ms);
    println!(
        "render_interval_ms={} export_step_ms={}",
        system.default_update_interval, header.step_ms
    );
    let store = effective_store(&system);
    let mut sequence =
        load_sequence_with_store(&paths.sequence_dir().join("Coldest Winter.tim"), &store).unwrap();
    vixen_session::library_links::apply_library_links(&mut sequence, &store);
    vixen_session::render::apply_load_rules(&mut sequence, &system.tree);
    let audio_path = sequence.media_path(0, &paths).unwrap();
    let clip = vixen_audio::AudioClip::load(&audio_path).unwrap();
    println!(
        "audio={} rate={} channels={} frames={} concealed={}",
        audio_path.display(),
        clip.sample_rate(),
        clip.channels(),
        clip.frames(),
        clip.concealed_frames()
    );
    let bytes: Vec<u8> = clip
        .samples()
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    std::fs::write(output.join("symphonia.f32"), bytes).unwrap();
    let mut variants = Vec::new();
    for quantize in ["round16", "trunc16"] {
        let samples = clip
            .samples()
            .iter()
            .map(|s| {
                let s = *s * 32768.0;
                (if quantize == "round16" {
                    s.round()
                } else {
                    s.trunc()
                })
                .clamp(-32768.0, 32767.0)
                    / 32768.0
            })
            .collect();
        variants.push((
            quantize.to_string(),
            vixen_audio::AudioClip::from_interleaved(samples, clip.channels(), clip.sample_rate()),
        ));
    }
    for raw in args.iter().skip(3) {
        let bytes = std::fs::read(raw).unwrap();
        assert_eq!(bytes.len() % 4, 0);
        let samples = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        variants.push((
            raw.clone(),
            vixen_audio::AudioClip::from_interleaved(samples, clip.channels(), clip.sample_rate()),
        ));
    }
    let result = vixen_session::compare::compare_with_fseq(
        &system,
        &paths,
        "Coldest Winter",
        &fseq_path,
        100,
    )
    .unwrap();
    println!(
        "unexplained={} max={} samples={:#?}",
        result.unexplained, result.largest_unexplained, result.unexplained_samples
    );
    let ids: Vec<_> = system.tree.element_ids().collect();
    let graph = vixen_session::show::build_output_graph(&system, &ids);
    let mut channels = Vec::new();
    for (controller, item) in graph.controllers().iter().enumerate() {
        for channel in 0..item.output_count() {
            let address = vixen_output::OutputAddress {
                controller,
                output: channel,
            };
            channels.push((
                graph.element_for_output(address),
                graph.channel_name(address, &|id| {
                    system
                        .tree
                        .find_by_element(id)
                        .map(|n| system.tree.node(n).name.clone())
                        .unwrap_or_default()
                }),
            ));
        }
    }
    let cancel = AtomicBool::new(false);
    let module_root = paths.module_data_dir();
    let mut seen = BTreeSet::new();
    for difference in &result.unexplained_samples {
        let (element_index, channel_name) = &channels[difference.channel];
        let element = ids[element_index.unwrap()];
        for (_, _, _, effect_id) in &difference.effects {
            let effect = sequence
                .effects
                .iter()
                .find(|e| e.instance_id == *effect_id)
                .unwrap();
            if vixen_effect_registry::by_type_id(&effect.type_id)
                .unwrap()
                .name
                != "Vertical Meter"
            {
                continue;
            }
            let targets = vixen_session::render::resolve_targets(&system.tree, effect);
            let ctx = RenderCtx { video_cache: None,
                tree: &system.tree,
                targets: &targets,
                start_time: effect.start,
                time_span: effect.time_span,
                frame_ms: system.default_update_interval,
                marks: &sequence.marks,
                audio: Some(&clip),
                module_data_root: Some(&module_root),
                cancel: &cancel,
            };
            let helper =
                fx_audio_state::audio_helper::AudioHelper::load(&ctx, &effect.params).unwrap();
            if seen.insert(*effect_id) {
                println!(
                    "effect={effect_id} start={} duration={} targets={:?}",
                    effect.start.total_ms(),
                    effect.time_span.total_ms(),
                    effect.targets
                );
                for name in [
                    "Range",
                    "AttackTime",
                    "DecayTime",
                    "Normalize",
                    "Gain",
                    "LowPass",
                    "LowPassFreq",
                    "HighPass",
                    "HighPassFreq",
                    "Inverted",
                    "DepthOfEffect",
                ] {
                    println!("  {name}={:?}", effect.params.get(name));
                }
                let mono = vixen_effects::framework::read_effect_audio(
                    &clip,
                    effect.start,
                    effect.time_span,
                )
                .unwrap();
                let bytes: Vec<u8> = mono.iter().flat_map(|s| s.to_le_bytes()).collect();
                std::fs::write(output.join(format!("{effect_id}-mono.f64")), bytes).unwrap();
                let csv: String = (0..effect.time_span.total_ms() as i32)
                    .step_by(30)
                    .map(|ms| {
                        format!(
                            "{ms},{},{:.17}\n",
                            helper.closest_sample(ms),
                            helper.volume_at_time(ms)
                        )
                    })
                    .collect();
                std::fs::write(output.join(format!("{effect_id}-levels.csv")), csv).unwrap();
                for (name, audio) in &variants {
                    let variant_ctx = RenderCtx {
                        audio: Some(audio),
                        ..ctx
                    };
                    let variant = fx_audio_state::audio_helper::AudioHelper::load(
                        &variant_ctx,
                        &effect.params,
                    )
                    .unwrap();
                    println!(
                        "variant={name} level540={:.12}",
                        variant.volume_at_time(540)
                    );
                }
            }
            let mut nodes = targets.clone();
            for _ in 0..effect.params.int("DepthOfEffect") {
                nodes = nodes
                    .iter()
                    .flat_map(|n| system.tree.node(*n).children.clone())
                    .collect();
            }
            if nodes.is_empty() {
                nodes = targets
                    .iter()
                    .flat_map(|n| system.tree.leaves(*n))
                    .collect();
            }
            for target in nodes {
                let leaves = system.tree.leaves(target);
                let Some(index) = leaves
                    .iter()
                    .position(|n| system.tree.node(*n).element_id == Some(element))
                else {
                    continue;
                };
                let count = leaves.len();
                let range = effect.params.int("Range") as f64;
                let threshold = if effect.params.boolean("Inverted") {
                    (index as f64 / count as f64 - 1.0) * range
                } else {
                    ((count - index) as f64 / count as f64 - 1.0) * range
                };
                let relative = difference.frame as f64 * step_ms - effect.start.total_ms();
                let gate_ms = (relative / 30.0).floor() as i32 * 30;
                println!("frame={} t={} channel={} name={:?} element={} target={} index={}/{} relative={} gate_ms={} sample={} threshold={:.12} level={:.12} margin={:.12}", difference.frame, difference.frame as f64*step_ms, difference.channel, channel_name, element, system.tree.node(target).name, index, count, relative, gate_ms, helper.closest_sample(gate_ms), threshold, helper.volume_at_time(gate_ms), helper.volume_at_time(gate_ms)-threshold);
                for delta in [-30, 0, 30] {
                    println!(
                        "  gate {} level {:.12}",
                        gate_ms + delta,
                        helper.volume_at_time(gate_ms + delta)
                    );
                }
            }
        }
    }
}
