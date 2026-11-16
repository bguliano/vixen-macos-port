//! Diagnostic-only Vertical Meter oracle input from an actual returned capture.
//! No decoder or device is opened. The caller validates capture/input hashes and
//! records source/program hashes; this example checks shape and fixture params.
//! Usage: vertical_meter_capture_probe <private profile> <capture dir> <NEW out>
use std::{path::PathBuf, sync::atomic::AtomicBool};
use vixen_effects::framework::{read_effect_audio, RenderCtx};
use vixen_io::{load_sequence_with_store, load_system, system::effective_store, DataPaths};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 4, "private profile, capture directory, fresh output required");
    let profile = PathBuf::from(&args[1]);
    let capture = PathBuf::from(&args[2]);
    let output = PathBuf::from(&args[3]);
    assert!(profile.is_absolute() && profile.components().any(|c| c.as_os_str() == "target"));
    assert!(capture.is_absolute() && output.is_absolute());
    assert!(!output.starts_with(&profile) && !output.starts_with(&capture) && !output.exists());
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(capture.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema"], 1);
    assert_eq!(manifest["status"], "captured-on-windows");
    assert_eq!(manifest["referenceBinaryContract"]["id"], "vixen-3.12.4-net8-installed-audio-1");
    assert_eq!(manifest["input"]["sha256"], "c1724f9916c61c623b9a19abbc80a7b161a0df79c8c504e427b8ba576289ada3");
    assert_eq!(manifest["pcm"]["file"], "cached.f32le");
    assert_eq!(manifest["pcm"]["format"]["Encoding"], "IeeeFloat");
    assert_eq!(manifest["pcm"]["format"]["SampleRate"], 44100);
    assert_eq!(manifest["pcm"]["format"]["Channels"], 2);
    assert_eq!(manifest["pcm"]["format"]["BitsPerSample"], 32);
    let bytes = std::fs::read(capture.join("cached.f32le")).unwrap();
    assert_eq!(bytes.len() as u64, manifest["pcm"]["bytes"].as_u64().unwrap());
    assert!(!bytes.is_empty() && bytes.len() % 8 == 0);
    let samples: Vec<f32> = bytes.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
    assert!(samples.iter().all(|s| s.is_finite()));
    assert_eq!(samples.len() as u64, manifest["pcm"]["samples"].as_u64().unwrap());
    assert_eq!(samples.len() as u64, manifest["decoder"]["separateReaderSamples"].as_u64().unwrap());
    assert_eq!(manifest["pcm"]["sha256"], manifest["decoder"]["separateReaderPcmSha256"]);
    let audio = vixen_audio::AudioClip::from_interleaved(samples, 2, 44100);
    let paths = DataPaths::new(profile);
    let system = load_system(&paths).unwrap();
    assert_eq!(system.default_update_interval, 10);
    let store = effective_store(&system);
    let mut sequence = load_sequence_with_store(&paths.sequence_dir().join("Coldest Winter.tim"), &store).unwrap();
    vixen_session::library_links::apply_library_links(&mut sequence, &store);
    vixen_session::render::apply_load_rules(&mut sequence, &system.tree);
    let id = uuid::Uuid::parse_str("f234e165-a96c-4ff1-8187-f549bc990c00").unwrap();
    let effect = sequence.effects.iter().find(|e| e.instance_id == id).unwrap();
    assert_eq!(vixen_effect_registry::by_type_id(&effect.type_id).unwrap().name, "Vertical Meter");
    assert_eq!(effect.start.total_ms(), 116807.0);
    assert_eq!(effect.time_span.total_ms(), 3106.0);
    for (name, expected) in [("AttackTime", 50), ("DecayTime", 1500), ("Gain", 0), ("HighPassFreq", 500), ("Range", 13), ("DepthOfEffect", 0)] { assert_eq!(effect.params.int(name), expected, "{name}"); }
    for (name, expected) in [("Normalize", false), ("LowPass", false), ("HighPass", true), ("Inverted", true)] { assert_eq!(effect.params.boolean(name), expected, "{name}"); }
    let targets = vixen_session::render::resolve_targets(&system.tree, effect);
    let cancel = AtomicBool::new(false);
    let module_root = paths.module_data_dir();
    let ctx = RenderCtx { video_cache: None, tree: &system.tree, targets: &targets, start_time: effect.start,
        time_span: effect.time_span, frame_ms: system.default_update_interval, marks: &sequence.marks,
        audio: Some(&audio), module_data_root: Some(&module_root), cancel: &cancel };
    let helper = fx_audio_state::audio_helper::AudioHelper::load(&ctx, &effect.params).unwrap();
    let mono = read_effect_audio(&audio, effect.start, effect.time_span).unwrap();
    assert_eq!(mono.len(), 273949);
    let rows: Vec<_> = (0..3106).step_by(30).map(|ms| (ms, helper.closest_sample(ms), helper.volume_at_time(ms))).collect();
    assert_eq!(rows.len(), 104);
    assert!(rows.iter().all(|(_, _, value)| value.is_finite()));
    std::fs::create_dir(&output).unwrap();
    let mono_bytes: Vec<u8> = mono.iter().flat_map(|sample| sample.to_le_bytes()).collect();
    std::fs::write(output.join("windows-captured-mono.f64"), mono_bytes).unwrap();
    let csv: String = rows.iter().map(|(ms, index, value)| format!("{ms},{index},{value:.17}\n")).collect();
    std::fs::write(output.join("windows-captured-levels.csv"), csv).unwrap();
    let level = helper.volume_at_time(540);
    let threshold = (147.0 / 563.0 - 1.0) * 13.0;
    let report = serde_json::json!({"scope":"Diagnostic raw captured PCM only; no production decoder invoked or changed. Caller validates cryptographic hashes.",
        "capture":capture,"profile":paths.root(),"effectId":id.to_string(),"inputFrames":audio.frames(),"sampleRate":44100,"channels":2,
        "monoSamples":mono.len(),"levels":rows.len(),"level540":level,"sample540":helper.closest_sample(540),"pixelThreshold":threshold,"margin540":level-threshold,
        "startMs":effect.start.total_ms(),"durationMs":effect.time_span.total_ms(),"rawOrigin":"Captured AudioData[0]; no trim, shift, resampling or quantization",
        "monoFile":"windows-captured-mono.f64","levelsFile":"windows-captured-levels.csv"});
    std::fs::write(output.join("probe.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{report}");
}
