//! Port of upstream `Vixen.Tests/Analysis/QmBarBeatTrackTests.cs`. `BarBeatTracker_DisposeReleasesNativePlugin` has no
//! runtime form here: using a dropped tracker doesn't compile, which the doctest on `BarBeatTracker` checks.

mod common;

use common::{click_track, for_each_block_upstream_test};
use vixen_analysis::{BarBeatTracker, Feature, FeatureSet, InputDomain, RealTime, SampleType};

const SAMPLE_RATE: u32 = 44_100;
const BEATS_PER_BAR: i32 = 4;
const BEAT_LABELS: [&str; 4] = ["1", "2", "3", "4"];

#[test]
fn bar_beat_tracker_exposes_expected_contract() {
    let mut plugin = BarBeatTracker::new(SAMPLE_RATE as f32);

    let parameters = plugin.parameter_descriptors();
    let outputs = plugin.output_descriptors();

    assert_eq!(plugin.identifier(), "qm-barbeattracker");
    assert_eq!(plugin.input_domain(), InputDomain::TimeDomain);

    let parameter_ids: Vec<&str> = parameters.iter().map(|p| p.identifier.as_str()).collect();
    assert_eq!(parameter_ids, ["bpb", "alpha", "inputtempo", "constraintempo"]);

    let beats_per_bar = parameters.iter().find(|p| p.identifier == "bpb").expect("bpb is listed");
    assert_eq!(beats_per_bar.min_value, 2.0);
    assert_eq!(beats_per_bar.max_value, 16.0);
    assert_eq!(beats_per_bar.default_value, BEATS_PER_BAR as f32);
    plugin.set_parameter("bpb", 3.0);
    assert_eq!(plugin.parameter("bpb"), 3.0);

    let output_ids: Vec<&str> = outputs.iter().map(|o| o.identifier.as_str()).collect();
    assert_eq!(output_ids, ["beats", "bars", "beatcounts", "beatsd"]);
}

#[test]
fn descriptors_keep_the_values_the_cpp_leaves_behind() {
    let plugin = BarBeatTracker::new(SAMPLE_RATE as f32);

    assert_eq!(plugin.name(), "Bar and Beat Tracker");
    assert_eq!(plugin.description(), "Estimate bar and beat locations");
    assert_eq!(plugin.maker(), "Queen Mary, University of London");
    assert_eq!(plugin.plugin_version(), 3);
    assert_eq!((plugin.min_channel_count(), plugin.max_channel_count()), (1, 1));

    let parameters = plugin.parameter_descriptors();
    let alpha = &parameters[1];
    assert_eq!((alpha.min_value, alpha.max_value, alpha.default_value), (0.1, 0.99, 0.9));
    assert!(!alpha.is_quantized);
    assert_eq!(alpha.quantize_step, 1.0);
    let tempo = &parameters[2];
    assert_eq!((tempo.min_value, tempo.max_value, tempo.default_value), (50.0, 250.0, 120.0));
    assert_eq!(tempo.unit, "BPM");
    assert!(tempo.is_quantized);
    let constrain = &parameters[3];
    assert_eq!((constrain.min_value, constrain.max_value, constrain.default_value), (0.0, 1.0, 0.0));
    assert_eq!(constrain.unit, "");

    let outputs = plugin.output_descriptors();
    assert_eq!(outputs[0].bin_count, 0);
    assert_eq!(outputs[2].bin_count, 1);
    for output in &outputs {
        assert!(output.has_fixed_bin_count);
        assert_eq!(output.sample_type, SampleType::VariableSampleRate);
        assert_eq!(output.sample_rate, (1.0 / 0.01161f32 as f64) as f32);
    }
}

fn process_samples(plugin: &mut BarBeatTracker, samples: &[f32], step_size: usize, block_size: usize) -> FeatureSet {
    let mut process = |block: &[f32], timestamp: RealTime| plugin.process(block, timestamp);
    for_each_block_upstream_test(samples, step_size, block_size, SAMPLE_RATE, &mut process);
    plugin.remaining_features()
}

fn assert_feature_baseline(
    features: &[Feature],
    expected_count: usize,
    expected_first_label: &str,
    expected_first_timestamp_ms: f64,
    expected_last_timestamp_ms: f64,
) {
    assert_eq!(features.len(), expected_count);
    let (first, last) = (features.first().unwrap(), features.last().unwrap());
    assert_eq!(first.label, expected_first_label);
    assert_eq!(first.timestamp.total_milliseconds(), expected_first_timestamp_ms);
    assert_eq!(last.timestamp.total_milliseconds(), expected_last_timestamp_ms);
}

#[test]
fn bar_beat_tracker_processes_deterministic_click_track() {
    let samples = click_track(SAMPLE_RATE as i32, BEATS_PER_BAR, &[(32, 120.0)]);
    let mut plugin = BarBeatTracker::new(SAMPLE_RATE as f32);
    plugin.set_parameter("bpb", BEATS_PER_BAR as f32);

    let step_size = plugin.preferred_step_size();
    let block_size = plugin.preferred_block_size();
    assert!(plugin.initialise(1, step_size, block_size));

    let features = process_samples(&mut plugin, &samples, step_size, block_size);

    assert_eq!(features.keys().copied().collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert_feature_baseline(&features[&0], 127, "2", 476.0, 63_494.0);
    assert_feature_baseline(&features[&1], 31, "1", 1_973.0, 61_974.0);
    assert_feature_baseline(&features[&2], 127, "2", 476.0, 63_494.0);
    assert_feature_baseline(&features[&3], 125, "", 975.0, 62_972.0);
    for output in [0, 1, 2, 3] {
        assert!(features[&output].iter().all(|f| f.has_timestamp));
    }
    assert!(features[&0].iter().all(|f| BEAT_LABELS.contains(&f.label.as_str())));
}

#[test]
fn feature_contents_follow_the_output() {
    let samples = click_track(SAMPLE_RATE as i32, BEATS_PER_BAR, &[(16, 120.0)]);
    let mut plugin = BarBeatTracker::new(SAMPLE_RATE as f32);
    let (step_size, block_size) = (plugin.preferred_step_size(), plugin.preferred_block_size());
    assert!(plugin.initialise(1, step_size, block_size));
    let features = process_samples(&mut plugin, &samples, step_size, block_size);

    for (beat, count) in features[&0].iter().zip(&features[&2]) {
        assert!(beat.values.is_empty());
        assert_eq!(count.values, vec![count.label.parse::<f32>().unwrap()]);
        assert_eq!(beat.label, count.label);
        assert_eq!(beat.timestamp, count.timestamp);
    }
    for (i, bar) in features[&1].iter().enumerate() {
        assert!(bar.values.is_empty());
        assert_eq!(bar.label, (i + 1).to_string());
    }
    for sd in &features[&3] {
        assert_eq!(sd.values.len(), 1);
        assert_eq!(sd.label, "");
    }
}
