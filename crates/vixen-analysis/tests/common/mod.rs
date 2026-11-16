//! What the golden tests share: the input generators (the same ones as tools/qm-golden/qm_golden.cpp), the driver loops,
//! and the comparison against the C++ output.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use serde_json::Value;
use vixen_analysis::detection_function::{DetectionFunction, DfConfig};
use vixen_analysis::down_beat::DownBeat;
use vixen_analysis::math_utilities::next_power_of_two;
use vixen_analysis::tempo_track_v2::TempoTrackV2;
use vixen_analysis::{for_each_block, generate_features, BarBeatTracker, BeatsAndBars, FeatureSet, RealTime};

/// Floating-point results are compared bit for bit where the goldens were made: this libm, this instruction set.
/// Elsewhere libm can round the last bit differently, so features get a small tolerance and the hashes are skipped.
pub const EXACT: bool = cfg!(all(target_os = "macos", target_arch = "aarch64"));

pub fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/analysis")
}

pub fn load_goldens(filter: impl Fn(&Value) -> bool) -> Vec<(String, Value)> {
    let mut goldens = Vec::new();
    for entry in fs::read_dir(golden_dir()).expect("tests/golden/analysis exists") {
        let path = entry.expect("readable directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("readable golden file");
        let value: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if filter(&value) {
            goldens.push((path.file_stem().unwrap().to_string_lossy().into_owned(), value));
        }
    }
    goldens.sort_by(|a, b| a.0.cmp(&b.0));
    goldens
}

pub fn fnv1a64(bytes: impl Iterator<Item = u8>) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn hash_f32(values: &[f32]) -> u64 {
    fnv1a64(values.iter().flat_map(|v| v.to_le_bytes()))
}

pub fn hash_f64(values: &[f64]) -> u64 {
    fnv1a64(values.iter().flat_map(|v| v.to_le_bytes()))
}

pub fn parse_hash(value: &Value) -> u64 {
    let text = value.as_str().expect("hash is a string");
    u64::from_str_radix(text.trim_start_matches("0x"), 16).expect("hash is hex")
}

fn xorshift32(x: &mut u32) -> u32 {
    *x ^= *x << 13;
    *x ^= *x >> 17;
    *x ^= *x << 5;
    *x
}

fn get_i32(input: &Value, key: &str) -> i32 {
    input[key].as_i64().unwrap_or(0) as i32
}

fn get_f64(input: &Value, key: &str) -> f64 {
    input[key].as_f64().unwrap_or(0.0)
}

fn beat_starts(input: &Value) -> (Vec<i32>, usize) {
    let rate = get_i32(input, "rate");
    let beats_per_bar = get_i32(input, "beats_per_bar");
    let mut starts = Vec::new();
    let mut cursor = 0;
    for section in input["sections"].as_array().expect("sections") {
        let bars = section[0].as_i64().unwrap() as i32;
        let bpm = section[1].as_f64().unwrap();
        let samples_per_beat = rate as f64 * 60.0 / bpm;
        for beat in 0..bars * beats_per_bar {
            starts.push(cursor + (beat as f64 * samples_per_beat).round_ties_even() as i32);
        }
        cursor += ((bars * beats_per_bar) as f64 * samples_per_beat).round_ties_even() as i32;
    }
    (starts, cursor as usize + 257)
}

/// `AddClick` from QmBarBeatTrackTests.cs.
fn add_click(samples: &mut [f32], start: i32, rate: i32, amplitude: f32, frequency: f64) {
    let length = (rate as f64 * 0.025) as i32;
    let mut offset = 0;
    while offset < length && ((start + offset) as usize) < samples.len() {
        let envelope = (-7.0 * offset as f64 / length as f64).exp();
        let phase = 2.0 * std::f64::consts::PI * frequency * offset as f64 / rate as f64;
        samples[(start + offset) as usize] += (amplitude as f64 * envelope * phase.sin()) as f32;
        offset += 1;
    }
}

/// `CreateClickTrack` from QmBarBeatTrackTests.cs.
pub fn click_track(rate: i32, beats_per_bar: i32, sections: &[(i32, f64)]) -> Vec<f32> {
    let input = serde_json::json!({
        "rate": rate,
        "beats_per_bar": beats_per_bar,
        "sections": sections.iter().map(|&(bars, bpm)| serde_json::json!([bars, bpm])).collect::<Vec<_>>(),
    });
    let (starts, total) = beat_starts(&input);
    let mut samples = vec![0.0f32; total];
    for (i, &start) in starts.iter().enumerate() {
        let downbeat = i as i32 % beats_per_bar == 0;
        add_click(
            &mut samples,
            start,
            rate,
            if downbeat { 28000.0 } else { 18000.0 },
            if downbeat { 880.0 } else { 1760.0 },
        );
    }
    samples
}

pub fn generate(input: &Value) -> Vec<f32> {
    let kind = input["kind"].as_str().expect("input kind");
    let rate = get_i32(input, "rate");
    let length = get_i32(input, "length") as usize;
    let amplitude = get_f64(input, "amplitude");
    let mut state = input["seed"].as_u64().unwrap_or(1) as u32;

    let mut samples: Vec<f32> = match kind {
        "click" => {
            let sections: Vec<(i32, f64)> = input["sections"]
                .as_array()
                .expect("sections")
                .iter()
                .map(|s| (s[0].as_i64().unwrap() as i32, s[1].as_f64().unwrap()))
                .collect();
            click_track(rate, get_i32(input, "beats_per_bar"), &sections)
        }
        "bursts" => {
            let (starts, total) = beat_starts(input);
            let beats_per_bar = get_i32(input, "beats_per_bar");
            let mut samples = vec![0.0f32; total];
            let burst = (rate as f64 * 0.05) as i32;
            for (i, &start) in starts.iter().enumerate() {
                let amp = if i as i32 % beats_per_bar == 0 { amplitude } else { amplitude * 0.6 };
                let mut offset = 0;
                while offset < burst && ((start + offset) as usize) < total {
                    let envelope = (-5.0 * offset as f64 / burst as f64).exp();
                    let noise = xorshift32(&mut state) as i32 as f64 / 2147483648.0;
                    samples[(start + offset) as usize] += (amp * envelope * noise) as f32;
                    offset += 1;
                }
            }
            samples
        }
        "noise" => (0..length).map(|_| (xorshift32(&mut state) as i32 as f64 * amplitude) as f32).collect(),
        "silence" => vec![0.0; length],
        "dc" => vec![amplitude as f32; length],
        "impulse" => {
            let mut samples = vec![0.0; length];
            samples[get_i32(input, "position") as usize] = amplitude as f32;
            samples
        }
        "tone" => {
            let frequency = get_f64(input, "frequency");
            (0..length)
                .map(|i| (amplitude * (2.0 * std::f64::consts::PI * frequency * i as f64 / rate as f64).sin()) as f32)
                .collect()
        }
        other => panic!("unknown input kind {other}"),
    };

    let lead = get_i32(input, "lead") as usize;
    if lead > 0 {
        samples.splice(0..0, std::iter::repeat_n(0.0, lead));
    }
    samples.extend(std::iter::repeat_n(0.0, get_i32(input, "tail") as usize));
    samples
}

/// The block and timestamp sequence of `GenerateFeatures`, with no plugin attached.
pub fn for_each_block_generate_features(
    data: &[f32],
    step_size: usize,
    block_size: usize,
    frequency: u32,
    process: &mut dyn FnMut(&[f32], RealTime),
) -> i32 {
    for_each_block(data, step_size, block_size, frequency, process, &mut |_| {}, &AtomicBool::new(false))
        .expect("not cancelled")
}

/// `ProcessSamples` from QmBarBeatTrackTests.cs.
pub fn for_each_block_upstream_test(
    data: &[f32],
    step_size: usize,
    block_size: usize,
    frequency: u32,
    process: &mut dyn FnMut(&[f32], RealTime),
) -> i32 {
    let mut processed = 0;
    let mut offset = 0;
    while offset < data.len() {
        let mut block = vec![0.0f32; block_size];
        let count = block_size.min(data.len() - offset);
        block[..count].copy_from_slice(&data[offset..offset + count]);
        process(&block, RealTime::frame2real_time(offset as i64, frequency));
        processed += 1;
        offset += step_size;
    }
    processed
}

fn set_parameters(plugin: &mut BarBeatTracker, params: &Value) {
    for name in ["bpb", "alpha", "inputtempo", "constraintempo"] {
        if let Some(value) = params[name].as_f64() {
            plugin.set_parameter(name, value as f32);
        }
    }
}

pub struct RunResult {
    pub step_size: usize,
    pub block_size: usize,
    pub blocks: i32,
    pub features: FeatureSet,
}

/// Runs the plugin the way a golden case says the C++ ran it.
pub fn run_plugin(golden: &Value, data: &[f32]) -> RunResult {
    let rate = golden["input"]["rate"].as_i64().expect("rate") as f32;
    let driver = golden["driver"].as_str().expect("driver");
    let cancel = AtomicBool::new(false);

    let mut plugin = BarBeatTracker::new(rate);
    set_parameters(&mut plugin, &golden["params"]);
    let (step_size, block_size) = (plugin.preferred_step_size(), plugin.preferred_block_size());
    assert!(plugin.initialise(1, step_size, block_size));

    let frequency = rate as u32;
    let mut noop = |_: &[f32], _: RealTime| {};
    let (blocks, features) = match driver {
        "beatsandbars" => {
            let features = generate_features(&mut plugin, data, rate, &mut |_| {}, &cancel).expect("not cancelled");
            (for_each_block_generate_features(data, step_size, block_size, frequency, &mut noop), features)
        }
        "preview" => {
            let analysis = BeatsAndBars::from_samples(data.to_vec(), rate, 1);
            let preview = analysis.preview_samples();
            let features = generate_features(&mut plugin, preview, rate, &mut |_| {}, &cancel).expect("not cancelled");
            (for_each_block_generate_features(preview, step_size, block_size, frequency, &mut noop), features)
        }
        "upstream_test" => {
            let mut process = |block: &[f32], t: RealTime| plugin.process(block, t);
            let blocks = for_each_block_upstream_test(data, step_size, block_size, frequency, &mut process);
            (blocks, plugin.remaining_features())
        }
        "reset_rerun" => {
            let half = &data[..data.len() / 2];
            let mut process = |block: &[f32], t: RealTime| plugin.process(block, t);
            for_each_block_generate_features(half, step_size, block_size, frequency, &mut process);
            plugin.reset();
            let mut process = |block: &[f32], t: RealTime| plugin.process(block, t);
            let blocks = for_each_block_generate_features(data, step_size, block_size, frequency, &mut process);
            (blocks, plugin.remaining_features())
        }
        "remaining_twice" => {
            let mut process = |block: &[f32], t: RealTime| plugin.process(block, t);
            let blocks = for_each_block_generate_features(data, step_size, block_size, frequency, &mut process);
            plugin.remaining_features();
            (blocks, plugin.remaining_features())
        }
        other => panic!("unknown driver {other}"),
    };

    RunResult { step_size, block_size, blocks, features }
}

/// The values the plugin keeps to itself, rebuilt from the same parts, as the harness does.
#[derive(Default)]
pub struct Intermediate {
    pub df: Vec<f64>,
    pub beat_period: Vec<f64>,
    pub tempi: Vec<f64>,
    pub beats: Vec<f64>,
    pub downbeats: Vec<i32>,
    pub beatsd: Vec<f64>,
    pub downsampled: Vec<f32>,
}

pub fn run_shadow(golden: &Value, data: &[f32], step_size: usize, block_size: usize) -> Intermediate {
    let mut out = Intermediate::default();
    let rate = golden["input"]["rate"].as_i64().expect("rate") as f32;
    let params = &golden["params"];
    let driver = golden["driver"].as_str().expect("driver");
    let cancel = AtomicBool::new(false);

    let config = DfConfig { step_size: step_size as i32, frame_length: block_size as i32 };
    let mut df = DetectionFunction::new(config).expect("even block");
    let factor = next_power_of_two((rate / 3000.0) as i32) as usize;
    let mut downbeat = DownBeat::new(rate, factor, step_size);

    let bpb = params["bpb"].as_f64().map_or(4, |b| (b as f32).round_ties_even() as i32);
    downbeat.set_beats_per_bar(bpb);

    let frequency = rate as u32;
    let mut df_output = Vec::new();
    let mut dfinput = vec![0.0; block_size];

    let mut run = |df: &mut DetectionFunction,
                   downbeat: &mut DownBeat,
                   df_output: &mut Vec<f64>,
                   samples: &[f32],
                   upstream: bool| {
        let mut process = |block: &[f32], _: RealTime| {
            for (out, value) in dfinput.iter_mut().zip(block) {
                *out = *value as f64;
            }
            df_output.push(df.process_time_domain(&dfinput));
            downbeat.push_audio_block(block);
        };
        if upstream {
            for_each_block_upstream_test(samples, step_size, block_size, frequency, &mut process);
        } else {
            for_each_block_generate_features(samples, step_size, block_size, frequency, &mut process);
        }
    };

    match driver {
        "upstream_test" => run(&mut df, &mut downbeat, &mut df_output, data, true),
        "preview" => {
            let analysis = BeatsAndBars::from_samples(data.to_vec(), rate, 1);
            run(&mut df, &mut downbeat, &mut df_output, analysis.preview_samples(), false);
        }
        "reset_rerun" => {
            run(&mut df, &mut downbeat, &mut df_output, &data[..data.len() / 2], false);
            df = DetectionFunction::new(config).expect("even block");
            df_output.clear();
            downbeat.reset_audio_buffer();
            run(&mut df, &mut downbeat, &mut df_output, data, false);
        }
        _ => run(&mut df, &mut downbeat, &mut df_output, data, false),
    }

    out.df = df_output.get(2..).unwrap_or(&[]).to_vec();
    if out.df.is_empty() {
        return out;
    }
    out.beat_period = vec![0.0; out.df.len()];

    let alpha = params["alpha"].as_f64().map_or(0.9, |a| (a as f32) as f64);
    let tempo = params["inputtempo"].as_f64().map_or(120.0, |t| (t as f32) as f64);
    let constrain = params["constraintempo"].as_f64().is_some_and(|c| (c as f32) > 0.5);

    let tt = TempoTrackV2::new(rate, step_size as i32);
    tt.calculate_beat_period(&out.df, &mut out.beat_period, &mut out.tempi, tempo, constrain, &cancel).unwrap();
    tt.calculate_beats(&out.df, &out.beat_period, &mut out.beats, alpha, 4.0, &cancel).unwrap();

    out.downsampled = downbeat.buffered_audio().to_vec();
    downbeat.find_down_beats_in_buffer(&out.beats, &mut out.downbeats, &cancel).unwrap();
    out.beatsd = downbeat.beat_sd().to_vec();
    out
}

/// The largest differences one case showed, for the report.
#[derive(Copy, Clone, Default, Debug)]
pub struct Differences {
    pub max_timestamp_ns: i64,
    pub max_value_abs: f64,
    pub features: usize,
}

impl Differences {
    pub fn merge(&mut self, other: Differences) {
        self.max_timestamp_ns = self.max_timestamp_ns.max(other.max_timestamp_ns);
        self.max_value_abs = self.max_value_abs.max(other.max_value_abs);
        self.features += other.features;
    }
}

pub fn compare_features(golden: &Value, features: &FeatureSet, errors: &mut Vec<String>) -> Differences {
    let mut diff = Differences::default();
    let expected = golden["features"].as_object().expect("features object");

    let got_keys: Vec<String> = features.keys().map(|k| k.to_string()).collect();
    let mut want_keys: Vec<String> = expected.keys().cloned().collect();
    want_keys.sort();
    if got_keys != want_keys {
        errors.push(format!("outputs differ: got {got_keys:?}, want {want_keys:?}"));
    }

    for (key, want_list) in expected {
        let want_list = want_list.as_array().expect("feature list");
        let Some(got_list) = key.parse::<i32>().ok().and_then(|k| features.get(&k)) else {
            continue;
        };
        if got_list.len() != want_list.len() {
            errors.push(format!("output {key}: {} features, want {}", got_list.len(), want_list.len()));
        }
        for (i, (got, want)) in got_list.iter().zip(want_list).enumerate() {
            diff.features += 1;
            let want_sec = want[0].as_i64().expect("sec");
            let want_nsec = want[1].as_i64().expect("nsec");
            let delta = (got.timestamp.sec as i64 - want_sec) * 1_000_000_000 + (got.timestamp.nsec as i64 - want_nsec);
            diff.max_timestamp_ns = diff.max_timestamp_ns.max(delta.abs());
            if delta != 0 {
                errors.push(format!(
                    "output {key}[{i}]: timestamp {}.{:09}, want {want_sec}.{want_nsec:09}",
                    got.timestamp.sec, got.timestamp.nsec
                ));
            }
            if !got.has_timestamp {
                errors.push(format!("output {key}[{i}]: no timestamp"));
            }
            let want_label = want[2].as_str().expect("label");
            if got.label != want_label {
                errors.push(format!("output {key}[{i}]: label {:?}, want {want_label:?}", got.label));
            }
            let want_values = want[3].as_array().expect("values");
            if got.values.len() != want_values.len() {
                errors.push(format!("output {key}[{i}]: {} values, want {}", got.values.len(), want_values.len()));
            }
            for (value, want_value) in got.values.iter().zip(want_values) {
                let want_value = want_value.as_f64().map_or(f32::NAN, |v| v as f32);
                let same_nan = value.is_nan() && want_value.is_nan();
                let abs = (*value as f64 - want_value as f64).abs();
                if !same_nan {
                    diff.max_value_abs = diff.max_value_abs.max(abs);
                }
                let matches = same_nan
                    || if EXACT {
                        value.to_bits() == want_value.to_bits()
                    } else {
                        abs <= 1e-5 * (value.abs().max(want_value.abs()) as f64).max(1e-30)
                    };
                if !matches {
                    errors.push(format!("output {key}[{i}]: value {value:e}, want {want_value:e}"));
                }
            }
        }
    }
    diff
}

fn rle(values: &[f64]) -> Vec<(f64, u64)> {
    let mut runs: Vec<(f64, u64)> = Vec::new();
    for &value in values {
        match runs.last_mut() {
            Some((v, count)) if *v == value => *count += 1,
            _ => runs.push((value, 1)),
        }
    }
    runs
}

pub fn compare_intermediate(golden: &Value, shadow: &Intermediate, errors: &mut Vec<String>) {
    let want = &golden["intermediate"];

    let mut check_len = |name: &str, got: usize, key: &str| {
        let expected = want[key].as_u64().expect("length") as usize;
        if got != expected {
            errors.push(format!("{name}: length {got}, want {expected}"));
        }
    };
    check_len("df", shadow.df.len(), "df_len");
    check_len("tempi", shadow.tempi.len(), "tempi_len");
    check_len("beatsd", shadow.beatsd.len(), "beatsd_len");
    check_len("downsampled", shadow.downsampled.len(), "downsampled_len");

    let want_rle: Vec<(f64, u64)> = want["beat_period_rle"]
        .as_array()
        .expect("beat_period_rle")
        .iter()
        .map(|r| (r[0].as_f64().unwrap(), r[1].as_u64().unwrap()))
        .collect();
    if rle(&shadow.beat_period) != want_rle {
        errors.push("beat period path differs".to_string());
    }

    let want_beats: Vec<f64> = want["beats"].as_array().expect("beats").iter().map(|b| b.as_f64().unwrap()).collect();
    if shadow.beats != want_beats {
        let first = shadow.beats.iter().zip(&want_beats).position(|(a, b)| a != b);
        errors.push(format!(
            "beats differ, first at {first:?}: {} beats, want {}",
            shadow.beats.len(),
            want_beats.len()
        ));
    }

    let want_downbeats: Vec<i32> =
        want["downbeats"].as_array().expect("downbeats").iter().map(|b| b.as_i64().unwrap() as i32).collect();
    if shadow.downbeats != want_downbeats {
        errors.push("downbeats differ".to_string());
    }

    if EXACT {
        let mut check_hash = |name: &str, got: u64, key: &str| {
            let expected = parse_hash(&want[key]);
            if got != expected {
                errors.push(format!("{name}: hash {got:#018x}, want {expected:#018x}"));
            }
        };
        check_hash("df", hash_f64(&shadow.df), "df_fnv1a64");
        check_hash("tempi", hash_f64(&shadow.tempi), "tempi_fnv1a64");
        check_hash("beatsd", hash_f64(&shadow.beatsd), "beatsd_fnv1a64");
        check_hash("downsampled", hash_f32(&shadow.downsampled), "downsampled_fnv1a64");
    }
}

/// One complete comparison. Returns the differences and any mismatches found.
pub fn run_golden_case(golden: &Value, data: &[f32]) -> (Differences, Vec<String>) {
    let mut errors = Vec::new();

    let want_len = golden["input_len"].as_u64().expect("input_len") as usize;
    let want_hash = parse_hash(&golden["input_fnv1a64"]);
    if data.len() != want_len || (EXACT && hash_f32(data) != want_hash) {
        errors.push(format!(
            "the input differs from the one the C++ ran ({} samples, want {want_len}). If the decoder changed on \
             purpose, regenerate with tools/qm-golden/generate.sh",
            data.len()
        ));
        return (Differences::default(), errors);
    }

    let result = run_plugin(golden, data);

    for (got, key) in [
        (result.step_size as i64, "step_size"),
        (result.block_size as i64, "block_size"),
        (result.blocks as i64, "blocks"),
    ] {
        let want = golden[key].as_i64().expect("size");
        if got != want {
            errors.push(format!("{key}: {got}, want {want}"));
        }
    }

    let diff = compare_features(golden, &result.features, &mut errors);
    let shadow = run_shadow(golden, data, result.step_size, result.block_size);
    compare_intermediate(golden, &shadow, &mut errors);
    (diff, errors)
}

/// Runs `work` over the cases on all cores and returns each case's outcome. A mismatch doesn't stop the others.
pub fn run_in_parallel<T: Send>(
    cases: &[(String, Value)],
    work: impl Fn(&str, &Value) -> T + Sync,
) -> Vec<(String, T)> {
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some((name, golden)) = cases.get(index) else {
                    break;
                };
                let outcome = work(name, golden);
                results.lock().unwrap().push((name.clone(), outcome));
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}
