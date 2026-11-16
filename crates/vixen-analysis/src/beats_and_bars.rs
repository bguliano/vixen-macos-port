//! Port of the non-UI logic of `BeatsAndBars.cs`: driving the tracker and turning its features into mark collections.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use vixen_core::{Mark, MarkCollection, TimeSpan};

use crate::bar_beat_track::BarBeatTracker;
use crate::samples::{float_samples_from_raw, raw_audio_bytes, RawSampleFormat, BYTES_PER_SAMPLE};
use crate::settings::{BeatBarPreviewData, BeatBarSettingsData};
use crate::vamp::{Feature, FeatureSet, RealTime};
use crate::Cancelled;

const PREVIEW_TIME: i32 = 10;

/// Feature outputs the mark builders read.
const OUTPUT_BEATS: i32 = 0;
const OUTPUT_BARS: i32 = 1;
const OUTPUT_BEAT_COUNTS: i32 = 2;

/// `IProgress<(int Value, bool IsFinalizing)>` in the C#.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct AnalysisProgress {
    pub value: i32,
    pub is_finalizing: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AnalysisError {
    Cancelled,
    /// The tracker refused to initialise.
    InitialiseFailed,
    /// An output the settings need has no features, as with a clip too short to hold a bar. The C# indexes the feature
    /// dictionary and throws `KeyNotFoundException`, having already removed the old collection of that name. Here
    /// the caller's collections are left alone.
    MissingOutput(i32),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalysisError::Cancelled => write!(f, "beat analysis was cancelled"),
            AnalysisError::InitialiseFailed => write!(f, "the beat tracker could not be initialised"),
            AnalysisError::MissingOutput(output) => {
                write!(f, "beat analysis returned no features for output {output}")
            }
        }
    }
}

impl std::error::Error for AnalysisError {}

impl From<Cancelled> for AnalysisError {
    fn from(_: Cancelled) -> AnalysisError {
        AnalysisError::Cancelled
    }
}

/// The loop in `GenerateFeatures`: the steps through `samples`, then the zero padded final block with the frame to time
/// conversion. Returns the number of blocks, which `LogFeatureSet` reports upstream. The percentage reaches `progress`
/// only when it changes, which is all `BeatsAndBarsProgress.UpdateProgress` acts on.
pub fn for_each_block(
    samples: &[f32],
    step_size: usize,
    block_size: usize,
    sample_rate: u32,
    process: &mut dyn FnMut(&[f32], RealTime),
    progress: &mut dyn FnMut(AnalysisProgress),
    cancel: &AtomicBool,
) -> Result<i32, Cancelled> {
    let mut processed_block_count = 0;
    let mut last_reported = -1;
    let mut j = 0;
    while samples.len() - j >= block_size {
        if cancel.load(Ordering::Relaxed) {
            return Err(Cancelled);
        }

        let value = ((j as f64 / samples.len() as f64) * 100.0) as i32;
        if value != last_reported {
            progress(AnalysisProgress { value, is_finalizing: false });
            last_reported = value;
        }

        process(&samples[j..j + block_size], RealTime::frame2real_time(j as i64, sample_rate));
        processed_block_count += 1;
        j += step_size;
    }

    let mut block = vec![0.0; block_size];
    block[..samples.len() - j].copy_from_slice(&samples[j..]);
    process(&block, RealTime::frame2real_time(j as i64, sample_rate));
    Ok(processed_block_count + 1)
}

/// `GenerateFeatures`. A frequency that truncates to zero gives no features, as in the C#.
pub fn generate_features(
    plugin: &mut BarBeatTracker,
    samples: &[f32],
    frequency: f32,
    progress: &mut dyn FnMut(AnalysisProgress),
    cancel: &AtomicBool,
) -> Result<FeatureSet, Cancelled> {
    let sample_rate = frequency as u32;
    if sample_rate == 0 {
        return Ok(FeatureSet::new());
    }

    let (step_size, block_size) = (plugin.preferred_step_size(), plugin.preferred_block_size());

    let mut process = |block: &[f32], timestamp: RealTime| plugin.process(block, timestamp);
    for_each_block(samples, step_size, block_size, sample_rate, &mut process, progress, cancel)?;

    progress(AnalysisProgress { value: 100, is_finalizing: true });
    plugin.remaining_features_cancellable(cancel)
}

/// `new MarkCollection()` with the name set. vixen-core's `Default` differs from the C# constructor on the mark bar and
/// tail lines, so those are spelled out.
fn new_collection(name: &str) -> MarkCollection {
    MarkCollection {
        name: name.to_string(),
        show_grid_lines: true,
        show_marks: false,
        show_tail_lines: false,
        ..MarkCollection::default()
    }
}

/// `new Mark(TimeSpan.FromMilliseconds(ms))`. The C# default duration is 450 ms; `vixen_core::Mark::new` uses 500.
fn new_mark(ms: f64) -> Mark {
    Mark { duration: TimeSpan::from_ms(450.0), ..Mark::new(TimeSpan::from_ms(ms)) }
}

fn remove_duplicate_marks(mc_orig: &mut MarkCollection, other_collections: &[MarkCollection]) {
    for other in other_collections {
        mc_orig.marks.retain(|x| !other.marks.iter().any(|m| m.start_time == x.start_time && m.duration == x.duration));
    }
}

/// `ExtractAllMarksFromFeatureSet`. As in the C#, the first beat comes out twice (from the `else` branch, then as
/// division 0 of the next beat) and the last beat is never added.
pub fn extract_all_marks(features: &[Feature], settings: &BeatBarSettingsData) -> MarkCollection {
    let mut mc = new_collection(&settings.all_collection_name());

    let mut last_feature_ms = -1.0;

    for feature in features.iter().filter(|f| f.has_timestamp) {
        let feature_ms = feature.timestamp.total_milliseconds();
        if last_feature_ms != -1.0 {
            let interval = (feature_ms - last_feature_ms) / settings.divisions as f64;
            for j in 0..settings.divisions {
                mc.add_mark(new_mark(last_feature_ms + (interval * j as f64)));
            }
        } else {
            mc.add_mark(new_mark(feature_ms));
        }
        last_feature_ms = feature_ms;
    }
    mc
}

/// `ExtractBarMarksFromFeatureSet`.
pub fn extract_bar_marks(features: &[Feature], settings: &BeatBarSettingsData) -> MarkCollection {
    let mut mc = new_collection(&settings.bars_collection_name());

    for feature in features.iter().filter(|f| f.has_timestamp) {
        mc.add_mark(new_mark(feature.timestamp.total_milliseconds()));
    }
    mc
}

/// `ExtractBeatCollectionsFromFeatureSet`.
pub fn extract_beat_collections(
    features: &[Feature],
    settings: &BeatBarSettingsData,
    other_marks: &[MarkCollection],
) -> Vec<MarkCollection> {
    let mut ret_val = Vec::new();

    for (index, name) in settings.beat_collection_names(false).iter().enumerate() {
        let mut mc = new_collection(name);
        let label = (index + 1).to_string();

        for feature in features.iter().filter(|f| f.has_timestamp && f.label == label) {
            mc.add_mark(new_mark(feature.timestamp.total_milliseconds()));
        }
        remove_duplicate_marks(&mut mc, other_marks);
        ret_val.push(mc);
    }
    ret_val
}

/// `ExtractSplitCollectionsFromFeatureSet`. A pair of marks comes from each beat and the one after it, so the last beat
/// adds nothing.
pub fn extract_split_collections(
    features: &[Feature],
    settings: &BeatBarSettingsData,
    other_marks: &[MarkCollection],
) -> Vec<MarkCollection> {
    let mut ts_value_pairs = Vec::with_capacity(features.len() * 2);

    for pair in features.windows(2) {
        let (last_feature, feature) = (&pair[0], &pair[1]);
        // Convert.ToInt32 throws on a label that isn't a number. Output 2's labels always are.
        let label_val = (last_feature.label.parse::<i32>().unwrap_or(0) * 2) - 1;
        let last_feature_ms = last_feature.timestamp.total_milliseconds();

        ts_value_pairs.push((label_val, last_feature_ms));

        let feature_ms = feature.timestamp.total_milliseconds();
        ts_value_pairs
            .push((label_val + 1, last_feature_ms + ((feature_ms - last_feature_ms) / settings.divisions as f64)));
    }

    let mut ret_val = Vec::new();
    for (index, name) in settings.beat_collection_names(true).iter().enumerate() {
        let mut mc = new_collection(name);
        let wanted = index as i32 + 1;
        for &(_, value) in ts_value_pairs.iter().filter(|(key, _)| *key == wanted) {
            mc.add_mark(new_mark(value));
        }
        remove_duplicate_marks(&mut mc, other_marks);
        ret_val.push(mc);
    }
    ret_val
}

/// `EstimateBeatPeriod`, in milliseconds. Written as the C# has it: the running average starts with the first interval
/// counted twice.
pub fn estimate_beat_period(features: &[Feature]) -> f64 {
    let mut ret_val = 0.0;
    let mut last_feature_ms = -1.0;
    let mut start_calcs = false;

    for feature in features {
        start_calcs = if feature.label == "1" { true } else { start_calcs };

        if feature.has_timestamp && start_calcs {
            let feature_ms = feature.timestamp.total_milliseconds();

            if last_feature_ms != -1.0 && ret_val == 0.0 {
                ret_val = feature_ms - last_feature_ms;
            }

            if last_feature_ms > 0.0 {
                ret_val = (ret_val + (feature_ms - last_feature_ms)) / 2.0;
            }

            last_feature_ms = feature_ms;
        }
    }
    ret_val
}

/// The body of `BuildMarkCollections` after the features exist. Replaces any collection with a generated name, and adds
/// the new non-empty ones in name order. `OrderBy` compares names by culture; the generated names differ only in
/// spaces, punctuation, digits and letters that sort the same way as plain byte order, so that is what this uses.
pub fn build_mark_collections(
    mark_collection: &mut Vec<MarkCollection>,
    feature_set: &FeatureSet,
    settings: &BeatBarSettingsData,
) -> Result<(), AnalysisError> {
    let output = |index: i32| feature_set.get(&index).ok_or(AnalysisError::MissingOutput(index));
    if settings.bars_enabled {
        output(OUTPUT_BARS)?;
    }
    if settings.beat_collections_enabled || settings.beat_splits_enabled {
        output(OUTPUT_BEAT_COUNTS)?;
    }
    if settings.all_features_enabled {
        output(OUTPUT_BEATS)?;
    }

    let mut ret_val: Vec<MarkCollection> = Vec::new();

    let beat_collection_names = settings.beat_collection_names(false);
    let split_collection_names = settings.beat_collection_names(true);

    if settings.bars_enabled {
        let name = settings.bars_collection_name();
        mark_collection.retain(|x| x.name != name);
        let mut mc = extract_bar_marks(output(OUTPUT_BARS)?, settings);
        mc.decorator.color = settings.bars_color;
        ret_val.push(mc);
    }

    if settings.beat_collections_enabled {
        mark_collection.retain(|x| !beat_collection_names.contains(&x.name));
        let mut mcl = extract_beat_collections(output(OUTPUT_BEAT_COUNTS)?, settings, &ret_val);
        for mc in &mut mcl {
            mc.decorator.color = settings.beat_counts_color;
        }
        ret_val.append(&mut mcl);
    }

    if settings.beat_splits_enabled {
        mark_collection.retain(|x| !split_collection_names.contains(&x.name));
        let mut mcl = extract_split_collections(output(OUTPUT_BEAT_COUNTS)?, settings, &ret_val);
        for mc in &mut mcl {
            mc.decorator.color = settings.beat_splits_color;
        }
        ret_val.append(&mut mcl);
    }

    if settings.all_features_enabled {
        let name = settings.all_collection_name();
        mark_collection.retain(|x| x.name != name);
        let mut mc = extract_all_marks(output(OUTPUT_BEATS)?, settings);
        mc.decorator.color = settings.all_features_color;
        ret_val.push(mc);
    }

    ret_val.retain(|x| !x.marks.is_empty());

    ret_val.sort_by(|a, b| a.name.cmp(&b.name));
    mark_collection.append(&mut ret_val);
    Ok(())
}

/// The end of `DoBeatBarDetection`: when nothing is the default collection yet, the first one becomes it.
pub fn ensure_default_collection(mark_collection: &mut [MarkCollection]) {
    if !mark_collection.iter().any(|x| x.is_default) {
        if let Some(first) = mark_collection.first_mut() {
            first.is_default = true;
        }
    }
}

/// One audio file's worth of analysis: `BeatsAndBars` with the dialog taken out. It holds plain data, so build it on any
/// thread and run the analysis on a background one. The steps in `DoBeatBarDetection` order are
/// [`generate_preview_data`](Self::generate_preview_data), then show the dialog, then
/// [`do_beat_bar_detection`](Self::do_beat_bar_detection).
pub struct BeatsAndBars {
    frequency: f32,
    channels: i32,
    samples_all: Vec<f32>,
    samples_preview: Vec<f32>,
}

impl BeatsAndBars {
    /// From decoded interleaved samples and the file's format, which is the usual way in:
    ///
    /// ```no_run
    /// # use vixen_analysis::{BeatsAndBars, RawSampleFormat};
    /// # let (interleaved, path) = (Vec::<f32>::new(), std::path::Path::new("song.mp3"));
    /// let format = RawSampleFormat::for_path(path).unwrap();
    /// let analysis = BeatsAndBars::from_interleaved(&interleaved, format, 44100.0, 2);
    /// ```
    pub fn from_interleaved(
        interleaved: &[f32],
        format: RawSampleFormat,
        frequency: f32,
        channels: i32,
    ) -> BeatsAndBars {
        BeatsAndBars::from_raw_bytes(&raw_audio_bytes(interleaved, format), frequency, channels)
    }

    /// From what `Audio.GetRawAudioSamples` returns. See [`crate::samples`] for what this does to the audio.
    pub fn from_raw_bytes(raw: &[u8], frequency: f32, channels: i32) -> BeatsAndBars {
        BeatsAndBars::from_samples(float_samples_from_raw(raw, BYTES_PER_SAMPLE), frequency, channels)
    }

    /// From samples that are already in the form the tracker sees.
    pub fn from_samples(samples_all: Vec<f32>, frequency: f32, channels: i32) -> BeatsAndBars {
        // `Frequency * PreviewTime` is a float product and Math.Min picks its float overload. A negative length throws in
        // the C#, and the rounded length can pass the real one only above a 1.6 MHz sample rate.
        let preview_len = frequency * PREVIEW_TIME as f32;
        let mut samples_preview = vec![0.0; (preview_len as i32).max(0) as usize];
        let count = (preview_len.min(samples_all.len() as f32) as i32).max(0) as usize;
        let count = count.min(samples_all.len()).min(samples_preview.len());
        samples_preview[..count].copy_from_slice(&samples_all[..count]);

        BeatsAndBars { frequency, channels, samples_all, samples_preview }
    }

    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples_all
    }

    /// The first ten seconds, zero padded to ten seconds when the audio is shorter.
    pub fn preview_samples(&self) -> &[f32] {
        &self.samples_preview
    }

    fn new_plugin(&self, beats_per_bar: i32) -> Result<BarBeatTracker, AnalysisError> {
        let mut plugin = BarBeatTracker::new(self.frequency);
        plugin.set_parameter("bpb", beats_per_bar as f32);
        let (step_size, block_size) = (plugin.preferred_step_size(), plugin.preferred_block_size());
        if !plugin.initialise(1, step_size, block_size) {
            return Err(AnalysisError::InitialiseFailed);
        }
        Ok(plugin)
    }

    /// `GeneratePreviewData`. The C# shows no progress dialog for this step, so there is no progress callback either.
    pub fn generate_preview_data(&self, cancel: &AtomicBool) -> Result<BeatBarPreviewData, AnalysisError> {
        let mut plugin = self.new_plugin(4)?;

        let feature_set = generate_features(&mut plugin, &self.samples_preview, self.frequency, &mut |_| {}, cancel)?;
        preview_data_from_features(&feature_set)
    }

    /// The part of `DoBeatBarDetection` after the dialog returns OK, including its default collection rule. With no
    /// channels, only the default rule runs, as in the C#.
    pub fn do_beat_bar_detection(
        &self,
        mark_collection: &mut Vec<MarkCollection>,
        settings: &BeatBarSettingsData,
        progress: &mut dyn FnMut(AnalysisProgress),
        cancel: &AtomicBool,
    ) -> Result<(), AnalysisError> {
        if self.channels != 0 {
            let mut plugin = self.new_plugin(settings.beats_per_bar)?;
            let feature_set = generate_features(&mut plugin, &self.samples_all, self.frequency, progress, cancel)?;
            build_mark_collections(mark_collection, &feature_set, settings)?;
        }
        ensure_default_collection(mark_collection);
        Ok(())
    }
}

/// The part of `GeneratePreviewData` after the tracker has run.
pub fn preview_data_from_features(feature_set: &FeatureSet) -> Result<BeatBarPreviewData, AnalysisError> {
    let beat_features = feature_set.get(&OUTPUT_BEAT_COUNTS).ok_or(AnalysisError::MissingOutput(OUTPUT_BEAT_COUNTS))?;
    let beat_period = estimate_beat_period(beat_features);

    let mut settings = BeatBarSettingsData::new("Preview");
    settings.divisions = 1;
    settings.beat_splits_enabled = false;
    settings.note_size = 4;
    settings.beats_per_bar = 4;

    let collections = extract_beat_collections(beat_features, &settings, &[]);
    let preview_collection = merge_collections("Beat Marks", &collections);

    settings.beat_splits_enabled = true;
    settings.divisions = 2;
    let collections = extract_split_collections(beat_features, &settings, &[]);
    let preview_split_collection = merge_collections("Beat Split Marks", &collections);

    Ok(BeatBarPreviewData { beat_period, preview_collection, preview_split_collection })
}

fn merge_collections(name: &str, collections: &[MarkCollection]) -> MarkCollection {
    let mut all = new_collection(name);
    for collection in collections {
        all.marks.extend(collection.marks.iter().cloned());
    }
    all.sort_marks();
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beat(ms: i32, label: &str) -> Feature {
        Feature {
            has_timestamp: true,
            timestamp: RealTime::new(ms / 1000, (ms % 1000) * 1_000_000),
            label: label.to_string(),
            ..Default::default()
        }
    }

    fn times(c: &MarkCollection) -> Vec<f64> {
        c.marks.iter().map(|m| m.start_time.total_ms()).collect()
    }

    fn settings(bpb: i32, divisions: i32) -> BeatBarSettingsData {
        let mut s = BeatBarSettingsData::new("Beats");
        s.beats_per_bar = bpb;
        s.note_size = crate::settings::calc_note_size(bpb);
        s.divisions = divisions;
        s
    }

    #[test]
    fn marks_match_the_csharp_constructors() {
        let mark = new_mark(12.5);
        assert_eq!(mark.start_time, TimeSpan::from_ticks(125_000));
        assert_eq!(mark.duration, TimeSpan::from_ms_i(450));
        assert_eq!(mark.text, "");

        let c = new_collection("A");
        assert_eq!(c.name, "A");
        assert_eq!(c.level, 1);
        assert!(c.show_grid_lines);
        assert!(!c.show_marks && !c.show_tail_lines && !c.is_default && !c.is_locked);
    }

    #[test]
    fn all_marks_repeat_the_first_beat_and_lose_the_last() {
        let features = [beat(1000, "1"), beat(1500, "2"), beat(2000, "3")];
        let mc = extract_all_marks(&features, &settings(4, 1));
        assert_eq!(mc.name, "Beats - All");
        assert_eq!(times(&mc), vec![1000.0, 1000.0, 1500.0]);
    }

    #[test]
    fn all_marks_with_two_divisions_split_each_interval() {
        let features = [beat(1000, "1"), beat(1500, "2"), beat(2000, "3")];
        let mc = extract_all_marks(&features, &settings(4, 2));
        assert_eq!(times(&mc), vec![1000.0, 1000.0, 1250.0, 1500.0, 1750.0]);
    }

    #[test]
    fn a_first_beat_at_time_zero_is_treated_as_any_other() {
        let features = [beat(0, "1"), beat(500, "2")];
        assert_eq!(times(&extract_all_marks(&features, &settings(4, 1))), vec![0.0, 0.0]);
    }

    #[test]
    fn beat_collections_follow_the_labels_and_drop_duplicates() {
        let features = [beat(1000, "1"), beat(1500, "2"), beat(2000, "1"), beat(2500, "2")];
        let s = settings(2, 1);
        let mut other = new_collection("Other");
        other.add_mark(new_mark(2000.0));
        let collections = extract_beat_collections(&features, &s, &[other]);
        assert_eq!(collections.len(), 2);
        assert_eq!(collections[0].name, "Beats Beat #1");
        assert_eq!(times(&collections[0]), vec![1000.0]);
        assert_eq!(times(&collections[1]), vec![1500.0, 2500.0]);
    }

    #[test]
    fn duplicates_need_the_same_duration_as_well() {
        let features = [beat(1000, "1")];
        let mut other = new_collection("Other");
        let mut longer = new_mark(1000.0);
        longer.duration = TimeSpan::from_ms_i(451);
        other.add_mark(longer);
        let collections = extract_beat_collections(&features, &settings(1, 1), &[other]);
        assert_eq!(times(&collections[0]), vec![1000.0]);
    }

    #[test]
    fn split_collections_pair_each_beat_with_the_next() {
        let features = [beat(1000, "1"), beat(1500, "2"), beat(2100, "1")];
        let collections = extract_split_collections(&features, &settings(2, 2), &[]);
        assert_eq!(collections.len(), 4);
        assert_eq!(collections[0].name, "Beats Beat #1a");
        assert_eq!(times(&collections[0]), vec![1000.0]);
        assert_eq!(collections[1].name, "Beats Beat #1b");
        assert_eq!(times(&collections[1]), vec![1250.0]);
        assert_eq!(times(&collections[2]), vec![1500.0]);
        assert_eq!(times(&collections[3]), vec![1800.0]);
    }

    #[test]
    fn beat_period_estimate_starts_at_the_first_downbeat() {
        let features = [beat(100, "4"), beat(1000, "1"), beat(1500, "2"), beat(2000, "3"), beat(2600, "4")];
        // (500 + 500) / 2, then (500 + 500) / 2, then (500 + 600) / 2
        assert_eq!(estimate_beat_period(&features), 550.0);
        assert_eq!(estimate_beat_period(&[]), 0.0);
        assert_eq!(estimate_beat_period(&[beat(100, "2")]), 0.0);
    }

    #[test]
    fn default_rule_marks_the_first_collection_only_when_none_is_default() {
        let mut none: Vec<MarkCollection> = vec![];
        ensure_default_collection(&mut none);

        let mut two = vec![new_collection("A"), new_collection("B")];
        ensure_default_collection(&mut two);
        assert!(two[0].is_default && !two[1].is_default);

        two[0].is_default = false;
        two[1].is_default = true;
        ensure_default_collection(&mut two);
        assert!(!two[0].is_default && two[1].is_default);
    }

    fn sample_features() -> FeatureSet {
        let beats = vec![beat(1000, "1"), beat(1500, "2"), beat(2000, "1"), beat(2500, "2")];
        let mut set = FeatureSet::new();
        set.insert(OUTPUT_BEATS, beats.clone());
        set.insert(OUTPUT_BARS, vec![beat(1000, "1"), beat(2000, "2")]);
        set.insert(OUTPUT_BEAT_COUNTS, beats);
        set
    }

    #[test]
    fn build_replaces_same_named_collections_and_keeps_others() {
        let mut s = settings(2, 1);
        s.bars_enabled = true;
        s.beat_collections_enabled = true;
        s.all_features_enabled = true;
        s.bars_color = vixen_core::Color::RED;
        s.beat_counts_color = vixen_core::Color::BLUE;
        s.all_features_color = vixen_core::Color::YELLOW;

        let mut existing = vec![new_collection("Mine"), new_collection("Beats - All"), new_collection("Beats Beat #2")];
        build_mark_collections(&mut existing, &sample_features(), &s).unwrap();

        let names: Vec<&str> = existing.iter().map(|c| c.name.as_str()).collect();
        // Beat #1 is empty after its duplicates of the bars go, so it is dropped.
        assert_eq!(names, vec!["Mine", "Beats - All", "Beats 1/2 Beat #1 (Bar)", "Beats Beat #2"]);
        assert_eq!(existing[1].decorator.color, vixen_core::Color::YELLOW);
        assert_eq!(existing[2].decorator.color, vixen_core::Color::RED);
        assert_eq!(existing[3].decorator.color, vixen_core::Color::BLUE);
        assert_eq!(times(&existing[3]), vec![1500.0, 2500.0]);
    }

    #[test]
    fn missing_outputs_are_errors_and_leave_the_collections_alone() {
        let mut s = settings(2, 1);
        s.bars_enabled = true;
        let mut set = sample_features();
        set.remove(&OUTPUT_BARS);
        let mut existing = vec![new_collection("Beats 1/2 Beat #1 (Bar)")];
        assert_eq!(build_mark_collections(&mut existing, &set, &s), Err(AnalysisError::MissingOutput(OUTPUT_BARS)));
        assert_eq!(existing.len(), 1);

        s.bars_enabled = false;
        s.all_features_enabled = true;
        set.remove(&OUTPUT_BEATS);
        assert_eq!(build_mark_collections(&mut existing, &set, &s), Err(AnalysisError::MissingOutput(OUTPUT_BEATS)));

        s.all_features_enabled = false;
        s.beat_splits_enabled = true;
        set.remove(&OUTPUT_BEAT_COUNTS);
        assert_eq!(
            build_mark_collections(&mut existing, &set, &s),
            Err(AnalysisError::MissingOutput(OUTPUT_BEAT_COUNTS))
        );
    }

    #[test]
    fn preview_is_exactly_ten_seconds() {
        let shorter = BeatsAndBars::from_samples(vec![1.0; 100], 8000.0, 2);
        assert_eq!(shorter.preview_samples().len(), 80000);
        assert_eq!(shorter.preview_samples()[99], 1.0);
        assert_eq!(shorter.preview_samples()[100], 0.0);

        let longer = BeatsAndBars::from_samples(vec![1.0; 100000], 8000.0, 2);
        assert_eq!(longer.preview_samples().len(), 80000);
        assert_eq!(longer.preview_samples()[79999], 1.0);
    }

    #[test]
    fn zero_frequency_produces_nothing_rather_than_failing() {
        let mut plugin = BarBeatTracker::new(0.0);
        let cancel = AtomicBool::new(false);
        let features = generate_features(&mut plugin, &[0.0; 100], 0.0, &mut |_| {}, &cancel).unwrap();
        assert!(features.is_empty());

        let analysis = BeatsAndBars::from_samples(vec![], 0.0, 0);
        assert!(analysis.preview_samples().is_empty());
        let mut collections = vec![new_collection("A")];
        analysis.do_beat_bar_detection(&mut collections, &settings(4, 1), &mut |_| {}, &cancel).unwrap();
        assert!(collections[0].is_default);
    }

    #[test]
    fn progress_counts_up_then_finalizes() {
        let analysis = BeatsAndBars::from_samples(vec![0.0; 100_000], 44100.0, 2);
        let mut plugin = analysis.new_plugin(4).unwrap();
        let cancel = AtomicBool::new(false);
        let mut seen = Vec::new();
        generate_features(&mut plugin, analysis.samples(), 44100.0, &mut |p| seen.push(p), &cancel).unwrap();
        assert_eq!(seen[0], AnalysisProgress { value: 0, is_finalizing: false });
        assert_eq!(seen[seen.len() - 1], AnalysisProgress { value: 100, is_finalizing: true });
        assert!(seen.windows(2).all(|w| w[0].value < w[1].value));
    }

    #[test]
    fn cancelling_during_the_block_loop_stops_it() {
        let analysis = BeatsAndBars::from_samples(vec![0.0; 100_000], 44100.0, 2);
        let cancel = AtomicBool::new(false);
        let mut collections = Vec::new();
        let mut progress = |p: AnalysisProgress| {
            if p.value >= 50 {
                cancel.store(true, Ordering::Relaxed);
            }
        };
        let result = analysis.do_beat_bar_detection(&mut collections, &settings(4, 1), &mut progress, &cancel);
        assert_eq!(result, Err(AnalysisError::Cancelled));
        assert!(collections.is_empty());
    }

    #[test]
    fn cancelling_before_finalizing_stops_there_too() {
        let mut plugin = BarBeatTracker::new(44100.0);
        assert!(plugin.initialise(1, 512, 1024));
        for _ in 0..1000 {
            plugin.process(&[0.0; 1024], RealTime::ZERO);
        }
        let cancel = AtomicBool::new(true);
        assert_eq!(plugin.remaining_features_cancellable(&cancel), Err(Cancelled));
    }

    #[test]
    fn bad_frequencies_do_not_panic() {
        for frequency in [-44100.0, f32::NAN, 0.0] {
            let analysis = BeatsAndBars::from_samples(vec![1.0; 10], frequency, 2);
            assert!(analysis.preview_samples().is_empty());
        }
    }

    #[test]
    fn analysis_runs_on_a_background_thread_and_can_be_cancelled_from_another() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BeatsAndBars>();
        assert_send_sync::<BarBeatTracker>();

        let analysis = std::sync::Arc::new(BeatsAndBars::from_samples(vec![0.5; 44100 * 3], 44100.0, 2));

        let finished = {
            let analysis = analysis.clone();
            std::thread::spawn(move || {
                let mut collections = Vec::new();
                let cancel = AtomicBool::new(false);
                analysis
                    .do_beat_bar_detection(&mut collections, &settings(4, 1), &mut |_| {}, &cancel)
                    .map(|_| collections)
            })
        };
        assert!(finished.join().unwrap().is_ok());

        let cancel = std::sync::Arc::new(AtomicBool::new(true));
        let cancelled = {
            let (analysis, cancel) = (analysis.clone(), cancel.clone());
            std::thread::spawn(move || analysis.generate_preview_data(&cancel))
        };
        assert_eq!(cancelled.join().unwrap().unwrap_err(), AnalysisError::Cancelled);
    }
}
