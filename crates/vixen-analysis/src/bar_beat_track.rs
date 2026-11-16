/*
    QM Vamp Plugin Set

    Centre for Digital Music, Queen Mary, University of London.

    This program is free software; you can redistribute it and/or
    modify it under the terms of the GNU General Public License as
    published by the Free Software Foundation; either version 2 of the
    License, or (at your option) any later version.  See the file
    COPYING included with this distribution for more information.
*/

//! Port of `BarBeatTrack.cpp`, the QM Bar and Beat Tracker Vamp plugin.

use std::sync::atomic::AtomicBool;

use crate::detection_function::{DetectionFunction, DfConfig};
use crate::down_beat::DownBeat;
use crate::math_utilities::next_power_of_two;
use crate::tempo_track_v2::TempoTrackV2;
use crate::vamp::{Feature, FeatureSet, InputDomain, OutputDescriptor, ParameterDescriptor, RealTime, SampleType};
use crate::Cancelled;

/// `BarBeatTracker::m_stepSecs`: 512 samples at 44100.
const STEP_SECS: f32 = 0.01161;

struct BarBeatTrackerData {
    df_config: DfConfig,
    df: DetectionFunction,
    down_beat: DownBeat,
    df_output: Vec<f64>,
    origin: RealTime,
}

impl BarBeatTrackerData {
    fn new(rate: f32, config: DfConfig) -> Option<BarBeatTrackerData> {
        // decimation factor aims at resampling to c. 3KHz; must be power of 2
        let factor = next_power_of_two((rate / 3000.0) as i32) as usize;
        Some(BarBeatTrackerData {
            df_config: config,
            df: DetectionFunction::new(config)?,
            down_beat: DownBeat::new(rate, factor, config.step_size as usize),
            df_output: Vec::new(),
            origin: RealTime::ZERO,
        })
    }
}

/// The tracker. Dropping it is `Dispose` in the C#, and the compiler refuses any later use:
///
/// ```compile_fail
/// let tracker = vixen_analysis::BarBeatTracker::new(44100.0);
/// drop(tracker);
/// tracker.identifier();
/// ```
pub struct BarBeatTracker {
    input_sample_rate: f32,
    d: Option<BarBeatTrackerData>,
    bpb: i32,
    alpha: f64,
    tightness: f64,
    input_tempo: f64,
    constrain_tempo: bool,
    dfinput: Vec<f64>,
    block: Vec<f32>,
}

impl BarBeatTracker {
    pub fn new(input_sample_rate: f32) -> BarBeatTracker {
        BarBeatTracker {
            input_sample_rate,
            d: None,
            bpb: 4,
            alpha: 0.9,
            tightness: 4.0,
            input_tempo: 120.0,
            constrain_tempo: false,
            dfinput: Vec::new(),
            block: Vec::new(),
        }
    }

    pub fn identifier(&self) -> &'static str {
        "qm-barbeattracker"
    }

    pub fn name(&self) -> &'static str {
        "Bar and Beat Tracker"
    }

    pub fn description(&self) -> &'static str {
        "Estimate bar and beat locations"
    }

    pub fn maker(&self) -> &'static str {
        "Queen Mary, University of London"
    }

    pub fn plugin_version(&self) -> i32 {
        3
    }

    pub fn copyright(&self) -> &'static str {
        "Plugin by Matthew Davies, Christian Landone and Chris Cannam.  Copyright (c) 2006-2013 QMUL - All Rights Reserved"
    }

    pub fn input_domain(&self) -> InputDomain {
        InputDomain::TimeDomain
    }

    pub fn min_channel_count(&self) -> usize {
        1
    }

    pub fn max_channel_count(&self) -> usize {
        1
    }

    pub fn parameter_descriptors(&self) -> Vec<ParameterDescriptor> {
        // The C++ reuses one descriptor, so a field a later parameter doesn't set keeps the earlier value:
        // alpha and inputtempo inherit quantizeStep 1.
        let mut list = Vec::new();

        let mut desc = ParameterDescriptor {
            identifier: "bpb".into(),
            name: "Beats per Bar".into(),
            description: "The number of beats in each bar".into(),
            min_value: 2.0,
            max_value: 16.0,
            default_value: 4.0,
            is_quantized: true,
            quantize_step: 1.0,
            ..Default::default()
        };
        list.push(desc.clone());

        desc.identifier = "alpha".into();
        desc.name = "Alpha".into();
        desc.description = "Inertia - Flexibility Trade Off".into();
        desc.min_value = 0.1;
        desc.max_value = 0.99;
        desc.default_value = 0.90;
        desc.unit = "".into();
        desc.is_quantized = false;
        list.push(desc.clone());

        // We aren't exposing tightness as a parameter, it's fixed at 4

        desc.identifier = "inputtempo".into();
        desc.name = "Tempo Hint".into();
        desc.description = "User-defined tempo on which to centre the tempo preference function".into();
        desc.min_value = 50.0;
        desc.max_value = 250.0;
        desc.default_value = 120.0;
        desc.unit = "BPM".into();
        desc.is_quantized = true;
        list.push(desc.clone());

        desc.identifier = "constraintempo".into();
        desc.name = "Constrain Tempo".into();
        desc.description =
            "Constrain more tightly around the tempo hint, using a Gaussian weighting instead of Rayleigh".into();
        desc.min_value = 0.0;
        desc.max_value = 1.0;
        desc.default_value = 0.0;
        desc.is_quantized = true;
        desc.quantize_step = 1.0;
        desc.unit = "".into();
        desc.value_names.clear();
        list.push(desc);

        list
    }

    pub fn parameter(&self, name: &str) -> f32 {
        match name {
            "bpb" => self.bpb as f32,
            "alpha" => self.alpha as f32,
            "inputtempo" => self.input_tempo as f32,
            "constraintempo" if self.constrain_tempo => 1.0,
            _ => 0.0,
        }
    }

    pub fn set_parameter(&mut self, name: &str, value: f32) {
        match name {
            // lrintf: the default rounding mode, ties to even.
            "bpb" => self.bpb = value.round_ties_even() as i32,
            "alpha" => self.alpha = value as f64,
            "inputtempo" => self.input_tempo = value as f64,
            "constraintempo" => self.constrain_tempo = value > 0.5,
            _ => {}
        }
    }

    /// False for the same bad channel count or step size the C++ refuses, and for a zero or odd block size, where it
    /// would throw or crash. Set `bpb` first: the downbeat finder takes its copy here.
    pub fn initialise(&mut self, channels: usize, step_size: usize, block_size: usize) -> bool {
        self.d = None;

        if channels < self.min_channel_count() || channels > self.max_channel_count() {
            return false;
        }

        if step_size != self.preferred_step_size() {
            return false;
        }

        if block_size == 0 {
            return false;
        }

        let config = DfConfig { step_size: step_size as i32, frame_length: block_size as i32 };

        let Some(mut data) = BarBeatTrackerData::new(self.input_sample_rate, config) else {
            return false;
        };
        data.down_beat.set_beats_per_bar(self.bpb);
        self.d = Some(data);
        self.dfinput = vec![0.0; block_size];
        self.block = vec![0.0; block_size.max(step_size)];
        true
    }

    pub fn reset(&mut self) {
        let rebuilt = self.d.as_ref().and_then(|d| DetectionFunction::new(d.df_config));
        if let (Some(d), Some(df)) = (self.d.as_mut(), rebuilt) {
            d.df = df;
            d.df_output.clear();
            d.down_beat.reset_audio_buffer();
            d.origin = RealTime::ZERO;
        }
    }

    pub fn preferred_step_size(&self) -> usize {
        // A float multiply, then a double add, as C++ promotes them.
        let step = ((self.input_sample_rate * STEP_SECS) as f64 + 0.0001) as usize;
        step.max(1)
    }

    pub fn preferred_block_size(&self) -> usize {
        self.preferred_step_size() * 2
    }

    pub fn output_descriptors(&self) -> Vec<OutputDescriptor> {
        let sample_rate = (1.0 / STEP_SECS as f64) as f32;
        let output = |identifier: &str, name: &str, description: &str, bin_count| OutputDescriptor {
            identifier: identifier.into(),
            name: name.into(),
            description: description.into(),
            has_fixed_bin_count: true,
            bin_count,
            sample_type: SampleType::VariableSampleRate,
            sample_rate,
            ..Default::default()
        };

        vec![
            output("beats", "Beats", "Beat locations labelled with metrical position", 0),
            output("bars", "Bars", "Bar locations", 0),
            output("beatcounts", "Beat Count", "Beat counter function", 1),
            output(
                "beatsd",
                "Beat Spectral Difference",
                "Beat spectral difference function used for bar-line detection",
                1,
            ),
        ]
    }

    /// Takes one block of time domain input for the single channel. Every feature comes from
    /// [`remaining_features`](Self::remaining_features), so this returns nothing. An input shorter than the block is
    /// read as if zero padded, where the C++ would run off the end of the buffer. Before a successful
    /// [`initialise`](Self::initialise) this does nothing.
    pub fn process(&mut self, input: &[f32], timestamp: RealTime) {
        let Some(d) = self.d.as_mut() else {
            return;
        };

        // We use time domain input, because DownBeat requires it -- so we
        // use the time-domain version of DetectionFunction::process which
        // does its own FFT.  It requires doubles as input, so we need to
        // make a temporary copy

        let fl = d.df_config.frame_length as usize;
        let needed = fl.max(d.df_config.step_size as usize);
        let input = if input.len() >= needed {
            input
        } else {
            self.block[..input.len()].copy_from_slice(input);
            self.block[input.len()..].fill(0.0);
            &self.block[..]
        };

        for (out, value) in self.dfinput.iter_mut().zip(&input[..fl]) {
            *out = *value as f64;
        }
        let output = d.df.process_time_domain(&self.dfinput);

        if d.df_output.is_empty() {
            d.origin = timestamp;
        }

        d.df_output.push(output);

        // Downsample and store the incoming audio block.
        // We have an overlap on the incoming audio stream (step size is
        // half block size) -- this function is configured to take only a
        // step size's worth, so effectively ignoring the overlap.  Note
        // however that this means we omit the last blocksize - stepsize
        // samples completely for the purposes of barline detection
        // (hopefully not a problem)
        d.down_beat.push_audio_block(input);
    }

    pub fn remaining_features(&mut self) -> FeatureSet {
        // A flag nobody else can see stays false.
        self.remaining_features_cancellable(&AtomicBool::new(false)).unwrap_or_default()
    }

    /// `getRemainingFeatures`, stopping early when `cancel` is set. This is the slow "finalizing" stage.
    pub fn remaining_features_cancellable(&mut self, cancel: &AtomicBool) -> Result<FeatureSet, Cancelled> {
        if self.d.is_none() {
            return Ok(FeatureSet::new());
        }
        self.bar_beat_track(cancel)
    }

    fn bar_beat_track(&mut self, cancel: &AtomicBool) -> Result<FeatureSet, Cancelled> {
        let Some(d) = self.d.as_mut() else {
            return Ok(FeatureSet::new());
        };

        // discard first two elts
        let df: Vec<f64> = d.df_output.get(2..).unwrap_or(&[]).to_vec();
        if df.is_empty() {
            return Ok(FeatureSet::new());
        }
        let mut beat_period = vec![0.0; df.len()];
        let mut tempi = Vec::new();

        let tt = TempoTrackV2::new(self.input_sample_rate, d.df_config.step_size);

        tt.calculate_beat_period(&df, &mut beat_period, &mut tempi, self.input_tempo, self.constrain_tempo, cancel)?;

        let mut beats = Vec::new();
        tt.calculate_beats(&df, &beat_period, &mut beats, self.alpha, self.tightness, cancel)?;

        let mut downbeats = Vec::new();
        d.down_beat.find_down_beats_in_buffer(&beats, &mut downbeats, cancel)?;

        let beatsd = d.down_beat.beat_sd().to_vec();

        let mut return_features = FeatureSet::new();

        let mut dbi = 0;
        let mut beat = 0;
        let mut bar = 0;

        if let Some(&first_down) = downbeats.first() {
            // get the right number for the first beat; this will be
            // incremented before use (at top of the following loop)
            beat = self.bpb - first_down - 1;
            if beat == self.bpb {
                beat = 0;
            }
        }

        // lrintf: the default rounding mode, ties to even.
        let rate = self.input_sample_rate.round_ties_even() as u32;
        let step = d.df_config.step_size as usize;

        for i in 0..beats.len() as i32 {
            let frame = beats[i as usize] as usize * step;

            if dbi < downbeats.len() && i == downbeats[dbi] {
                beat = 0;
                bar += 1;
                dbi += 1;
            } else {
                beat += 1;
            }

            // outputs are:
            //
            // 0 -> beats
            // 1 -> bars
            // 2 -> beat counter function

            let mut feature = Feature {
                has_timestamp: true,
                timestamp: d.origin + RealTime::frame2real_time(frame as i64, rate),
                ..Default::default()
            };

            feature.label = (beat + 1).to_string();
            return_features.entry(0).or_default().push(feature.clone()); // labelled beats

            feature.values.push((beat + 1) as f32);
            return_features.entry(2).or_default().push(feature.clone()); // beat function

            if i > 0 && i as usize <= beatsd.len() {
                feature.values.clear();
                feature.values.push(beatsd[i as usize - 1] as f32);
                feature.label = String::new();
                return_features.entry(3).or_default().push(feature.clone()); // beat spectral difference
            }

            if beat == 0 {
                feature.values.clear();
                feature.label = bar.to_string();
                return_features.entry(1).or_default().push(feature); // bars
            }
        }

        Ok(return_features)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_sizes() {
        for (rate, step) in [
            (44100.0, 512),
            (48000.0, 557),
            (22050.0, 256),
            (32000.0, 371),
            (16000.0, 185),
            (11025.0, 128),
            (8000.0, 92),
            (96000.0, 1114),
            (30000.0, 348),
            (1.0, 1),
            (0.0, 1),
        ] {
            let plugin = BarBeatTracker::new(rate);
            assert_eq!(plugin.preferred_step_size(), step, "rate {rate}");
            assert_eq!(plugin.preferred_block_size(), step * 2, "rate {rate}");
        }
    }

    #[test]
    fn initialise_refuses_what_the_cpp_refuses() {
        let mut plugin = BarBeatTracker::new(44100.0);
        assert!(!plugin.initialise(2, 512, 1024));
        assert!(!plugin.initialise(0, 512, 1024));
        assert!(!plugin.initialise(1, 256, 1024));
        assert!(!plugin.initialise(1, 512, 0));
        assert!(!plugin.initialise(1, 512, 1023));
        assert!(plugin.initialise(1, 512, 1024));
        // A sub-optimal block size only warns in the C++.
        assert!(plugin.initialise(1, 512, 2048));
    }

    #[test]
    fn uninitialised_plugin_is_harmless() {
        let mut plugin = BarBeatTracker::new(44100.0);
        plugin.process(&[0.0; 1024], RealTime::ZERO);
        plugin.reset();
        assert!(plugin.remaining_features().is_empty());
    }

    #[test]
    fn a_short_block_is_read_as_zero_padded() {
        let mut short = BarBeatTracker::new(44100.0);
        let mut full = BarBeatTracker::new(44100.0);
        assert!(short.initialise(1, 512, 1024));
        assert!(full.initialise(1, 512, 1024));
        let mut block = vec![0.0; 1024];
        block[..4].copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        for _ in 0..4 {
            short.process(&block[..4], RealTime::ZERO);
            full.process(&block, RealTime::ZERO);
        }
        assert_eq!(short.remaining_features(), full.remaining_features());
    }

    #[test]
    fn parameter_round_trip() {
        let mut plugin = BarBeatTracker::new(44100.0);
        assert_eq!(plugin.parameter("bpb"), 4.0);
        assert_eq!(plugin.parameter("alpha"), 0.9);
        assert_eq!(plugin.parameter("inputtempo"), 120.0);
        assert_eq!(plugin.parameter("constraintempo"), 0.0);
        assert_eq!(plugin.parameter("nope"), 0.0);
        plugin.set_parameter("bpb", 3.5);
        assert_eq!(plugin.parameter("bpb"), 4.0);
        plugin.set_parameter("bpb", 2.5);
        assert_eq!(plugin.parameter("bpb"), 2.0);
        plugin.set_parameter("constraintempo", 0.6);
        assert_eq!(plugin.parameter("constraintempo"), 1.0);
        plugin.set_parameter("constraintempo", 0.5);
        assert_eq!(plugin.parameter("constraintempo"), 0.0);
    }
}
