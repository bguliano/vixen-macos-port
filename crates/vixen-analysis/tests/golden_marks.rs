//! Compares mark building with upstream's own `BeatsAndBars.cs` and `MarkCollection.cs`, compiled and run on this Mac by
//! tools/qm-golden/marks. Only the Windows-only parts are stubbed there: the C# takes scripted features from the other
//! golden files, so what this checks is everything after the tracker, and the samples and blocks going into it.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use common::{fnv1a64, golden_dir, hash_f32, parse_hash};
use serde_json::Value;
use vixen_analysis::{
    build_mark_collections, ensure_default_collection, float_samples_from_raw, for_each_block,
    preview_data_from_features, AnalysisError, BarBeatTracker, BeatBarSettingsData, BeatsAndBars, Feature, FeatureSet,
    RealTime,
};
use vixen_core::marks::MarkCollectionType;
use vixen_core::{Color, Mark, MarkCollection, TimeSpan};

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("readable golden")).expect("valid JSON")
}

fn features_from_golden(name: &str) -> FeatureSet {
    let golden = read_json(&golden_dir().join(format!("{name}.json")));
    let mut set = FeatureSet::new();
    for (key, list) in golden["features"].as_object().expect("features") {
        let features = list
            .as_array()
            .expect("feature list")
            .iter()
            .map(|entry| Feature {
                has_timestamp: true,
                timestamp: RealTime::new(entry[0].as_i64().unwrap() as i32, entry[1].as_i64().unwrap() as i32),
                label: entry[2].as_str().unwrap().to_string(),
                values: entry[3].as_array().unwrap().iter().map(|v| v.as_f64().unwrap() as f32).collect(),
                ..Default::default()
            })
            .collect();
        set.insert(key.parse().unwrap(), features);
    }
    set
}

fn crafted_features(case: &Value, no_bars: bool) -> FeatureSet {
    let beats: Vec<Feature> = case["crafted_beats"]
        .as_array()
        .expect("crafted beats")
        .iter()
        .map(|b| Feature {
            has_timestamp: true,
            timestamp: RealTime::new(b[0].as_i64().unwrap() as i32, b[1].as_i64().unwrap() as i32),
            label: b[2].as_str().unwrap().to_string(),
            ..Default::default()
        })
        .collect();
    let mut set = BTreeMap::new();
    set.insert(0, beats.clone());
    if !no_bars {
        set.insert(1, beats.iter().filter(|f| f.label == "1").cloned().collect());
    }
    set.insert(2, beats);
    set
}

fn color_from(value: &Value) -> Color {
    let part = |i: usize| value[i].as_u64().unwrap() as u8;
    Color::argb(part(0), part(1), part(2), part(3))
}

fn settings_from(case: &Value) -> BeatBarSettingsData {
    let s = &case["settings"];
    let flag = |key: &str| s[key].as_bool().unwrap();
    let mut settings = BeatBarSettingsData::new(s["collection_base_name"].as_str().unwrap());
    settings.bars_enabled = flag("bars_enabled");
    settings.beat_collections_enabled = flag("beat_collections_enabled");
    settings.beat_splits_enabled = flag("beat_splits_enabled");
    settings.all_features_enabled = flag("all_features_enabled");
    settings.divisions = s["divisions"].as_i64().unwrap() as i32;
    settings.all_features_color = color_from(&s["all_features_color"]);
    settings.bars_color = color_from(&s["bars_color"]);
    settings.beat_counts_color = color_from(&s["beat_counts_color"]);
    settings.beat_splits_color = color_from(&s["beat_splits_color"]);
    settings.beats_per_bar = s["beats_per_bar"].as_i64().unwrap() as i32;
    settings.note_size = s["note_size"].as_i64().unwrap() as i32;
    settings
}

fn names_of(value: &Value) -> Vec<String> {
    value.as_array().unwrap().iter().map(|n| n.as_str().unwrap().to_string()).collect()
}

fn existing_from(case: &Value) -> Vec<MarkCollection> {
    case["existing"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let mut collection = MarkCollection {
                name: e["name"].as_str().unwrap().to_string(),
                is_default: e["is_default"].as_bool().unwrap(),
                show_grid_lines: true,
                show_marks: false,
                show_tail_lines: false,
                ..MarkCollection::default()
            };
            collection.add_mark(Mark { duration: TimeSpan::from_ms(450.0), ..Mark::new(TimeSpan::from_ms(123.0)) });
            collection
        })
        .collect()
}

fn check_collection(context: &str, got: &MarkCollection, want: &Value, errors: &mut Vec<String>) {
    let context = format!("{context}/{}", got.name);
    let mut expect = |what: &str, same: bool| {
        if !same {
            errors.push(format!("{context}: {what} differs"));
        }
    };
    expect("name", got.name == want["name"].as_str().unwrap());
    expect("color", got.decorator.color == color_from(&want["color"]));
    expect("bold", got.decorator.is_bold == want["is_bold"].as_bool().unwrap());
    expect("solid line", got.decorator.is_solid_line == want["is_solid_line"].as_bool().unwrap());
    expect("default", got.is_default == want["is_default"].as_bool().unwrap());
    expect("level", got.level as i64 == want["level"].as_i64().unwrap());
    expect("grid lines", got.show_grid_lines == want["show_grid_lines"].as_bool().unwrap());
    expect("mark bar", got.show_marks == want["show_mark_bar"].as_bool().unwrap());
    expect("tail lines", got.show_tail_lines == want["show_tail_grid_lines"].as_bool().unwrap());
    expect("locked", got.is_locked == want["locked"].as_bool().unwrap());
    expect("type", got.collection_type == MarkCollectionType::Generic && want["collection_type"] == "Generic");
    expect("link", got.linked_mark_collection_id.is_some() == want["linked"].as_bool().unwrap());

    let marks = want["marks"].as_array().unwrap();
    expect("mark count", got.marks.len() == marks.len());
    for (i, (mark, want_mark)) in got.marks.iter().zip(marks).enumerate() {
        let (start, duration) = (want_mark[0].as_i64().unwrap(), want_mark[1].as_i64().unwrap());
        expect(&format!("mark {i} start ({} vs {start})", mark.start_time.ticks()), mark.start_time.ticks() == start);
        expect(&format!("mark {i} duration"), mark.duration.ticks() == duration);
        expect(&format!("mark {i} text"), mark.text == want_mark[2].as_str().unwrap());
    }
}

fn raw_bytes(count: usize) -> Vec<u8> {
    let mut state: u32 = 2463534242;
    (0..count)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 24) as u8
        })
        .collect()
}

/// The stub plugin's record of a run: every block's floats and timestamp, through the crate's own block loop.
fn block_recording(samples: &[f32], frequency: f32) -> (i32, u64) {
    let plugin = BarBeatTracker::new(frequency);
    let mut bytes = Vec::new();
    let mut process = |block: &[f32], timestamp: RealTime| {
        for value in block {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&timestamp.sec.to_le_bytes());
        bytes.extend_from_slice(&timestamp.nsec.to_le_bytes());
    };
    let blocks = for_each_block(
        samples,
        plugin.preferred_step_size(),
        plugin.preferred_block_size(),
        frequency as u32,
        &mut process,
        &mut |_| {},
        &AtomicBool::new(false),
    )
    .expect("not cancelled");
    (blocks, fnv1a64(bytes.into_iter()))
}

fn check_audio(case: &Value, errors: &mut Vec<String>) {
    let audio = &case["audio"];
    let frequency = audio["frequency"].as_f64().unwrap() as f32;
    let raw = raw_bytes(audio["raw_bytes"].as_u64().unwrap() as usize);
    let samples = float_samples_from_raw(&raw, audio["bytes_per_sample"].as_u64().unwrap() as usize);

    // The partial sample case throws partway through the conversion, so the C# has no result to compare.
    if case["exception"] == "ArgumentException" {
        if samples.len() != raw.len() / 4 {
            errors.push("the leftover bytes should be dropped".to_string());
        }
        return;
    }
    if audio["samples_fnv1a64"].is_null() {
        return;
    }

    if samples.len() as u64 != audio["samples_len"].as_u64().unwrap() {
        errors.push(format!("{} samples, want {}", samples.len(), audio["samples_len"]));
    }
    if hash_f32(&samples) != parse_hash(&audio["samples_fnv1a64"]) {
        errors.push("the samples differ".to_string());
    }

    let analysis = BeatsAndBars::from_samples(samples.clone(), frequency, audio["channels"].as_i64().unwrap() as i32);
    let preview = analysis.preview_samples();
    if preview.len() as u64 != audio["preview_len"].as_u64().unwrap() {
        errors.push(format!("{} preview samples, want {}", preview.len(), audio["preview_len"]));
    }
    if hash_f32(preview) != parse_hash(&audio["preview_fnv1a64"]) {
        errors.push("the preview samples differ".to_string());
    }

    // Plugins are created in this order in DoBeatBarDetection: the one for the whole song, then the preview's.
    let plugins = audio["plugins"].as_array().unwrap();
    let main_runs = plugins[0]["blocks"].as_i64().unwrap() != 0;
    if main_runs {
        let (blocks, hash) = block_recording(&samples, frequency);
        if blocks as i64 != plugins[0]["blocks"].as_i64().unwrap() || hash != parse_hash(&plugins[0]["blocks_fnv1a64"])
        {
            errors.push(format!("the blocks of the whole song differ ({blocks} blocks)"));
        }
        let want_bpb = plugins[0]["bpb"].as_i64().unwrap();
        if want_bpb != case["settings"]["beats_per_bar"].as_i64().unwrap() {
            errors.push(format!("the whole song's plugin got bpb {want_bpb}"));
        }
    }
    if let Some(entry) = plugins.get(1) {
        let (blocks, hash) = block_recording(preview, frequency);
        if blocks as i64 != entry["blocks"].as_i64().unwrap() || hash != parse_hash(&entry["blocks_fnv1a64"]) {
            errors.push(format!("the preview's blocks differ ({blocks} blocks)"));
        }
        if entry["bpb"].as_i64().unwrap() != 4 {
            errors.push("the preview's plugin should get bpb 4".to_string());
        }
    }
}

fn run_case(case: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let settings = settings_from(case);
    let s = &case["settings"];

    if settings.all_collection_name() != s["all_collection_name"].as_str().unwrap() {
        errors.push("all collection name differs".to_string());
    }
    if settings.bars_collection_name() != s["bars_collection_name"].as_str().unwrap() {
        errors.push("bars collection name differs".to_string());
    }
    if settings.beat_collection_names(false) != names_of(&s["beat_collection_names"]) {
        errors.push("beat collection names differ".to_string());
    }
    if settings.beat_collection_names(true) != names_of(&s["split_collection_names"]) {
        errors.push("split collection names differ".to_string());
    }

    check_audio(case, &mut errors);

    let crafted = !case["crafted_beats"].is_null();
    let preview_features = if crafted {
        crafted_features(case, false)
    } else {
        features_from_golden(case["preview_features"].as_str().unwrap())
    };
    let main_features = if crafted {
        crafted_features(case, case["no_bars"].as_bool().unwrap_or(false))
    } else {
        features_from_golden(case["main_features"].as_str().unwrap())
    };

    let exception = case["exception"].as_str();

    // Two cases never reach the mark building: no channels, and the conversion throwing first.
    if case["audio"]["channels"].as_i64().unwrap() == 0 || exception == Some("ArgumentException") {
        if exception.is_none() {
            let analysis = BeatsAndBars::from_samples(Vec::new(), 44100.0, 0);
            let mut collections = existing_from(case);
            analysis
                .do_beat_bar_detection(&mut collections, &settings, &mut |_| {}, &AtomicBool::new(false))
                .expect("nothing to fail");
            compare_all(&collections, case, &mut errors);
        }
        return errors;
    }

    match (preview_data_from_features(&preview_features), case["preview"].is_null()) {
        (Ok(preview), false) => {
            let want = &case["preview"];
            if preview.beat_period.to_bits() != want["beat_period"].as_f64().unwrap().to_bits() {
                errors.push(format!("beat period {}, want {}", preview.beat_period, want["beat_period"]));
            }
            check_collection("preview", &preview.preview_collection, &want["collection"], &mut errors);
            check_collection("preview", &preview.preview_split_collection, &want["split_collection"], &mut errors);
        }
        (Err(AnalysisError::MissingOutput(2)), true) => {
            if exception != Some("KeyNotFoundException") {
                errors.push("expected the C# to throw on the preview too".to_string());
            }
            return errors;
        }
        (got, _) => {
            errors.push(format!("preview outcome differs: {:?}", got.map(|_| "a preview")));
            return errors;
        }
    }

    let mut collections = existing_from(case);
    let before = collections.clone();
    match build_mark_collections(&mut collections, &main_features, &settings) {
        Ok(()) => {
            if exception.is_some() {
                errors.push(format!("the C# threw {exception:?}"));
            }
            ensure_default_collection(&mut collections);
            compare_all(&collections, case, &mut errors);
        }
        Err(AnalysisError::MissingOutput(_)) => {
            if exception != Some("KeyNotFoundException") {
                errors.push("the C# did not throw here".to_string());
            }
            // A deliberate difference: the C# has already removed the old collection of that name.
            if collections != before {
                errors.push("a failed build should leave the collections alone".to_string());
            }
        }
        Err(other) => errors.push(format!("unexpected error {other}")),
    }
    errors
}

fn compare_all(collections: &[MarkCollection], case: &Value, errors: &mut Vec<String>) {
    let want = case["collections"].as_array().unwrap();
    let got_names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
    let want_names: Vec<&str> = want.iter().map(|c| c["name"].as_str().unwrap()).collect();
    if got_names != want_names {
        errors.push(format!("collections: {got_names:?}, want {want_names:?}"));
        return;
    }
    for (got, want) in collections.iter().zip(want) {
        check_collection("collections", got, want, errors);
    }
}

#[test]
fn mark_building_matches_the_csharp() {
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(golden_dir().join("marks")).expect("marks goldens exist") {
        let path = entry.expect("readable directory entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            cases.push(read_json(&path));
        }
    }
    assert!(cases.len() >= 30, "only {} mark cases found", cases.len());

    let mut failures = Vec::new();
    let mut marks = 0;
    for case in &cases {
        marks +=
            case["collections"].as_array().unwrap().iter().map(|c| c["marks"].as_array().unwrap().len()).sum::<usize>();
        let errors = run_case(case);
        if !errors.is_empty() {
            let shown: Vec<&str> = errors.iter().take(5).map(String::as_str).collect();
            failures.push(format!("{}: {} mismatches, first: {}", case["name"], errors.len(), shown.join("; ")));
        }
    }

    println!("{} cases, {marks} marks compared with the C#", cases.len());
    assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), cases.len(), failures.join("\n"));
}
