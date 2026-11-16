//! Beats and Bars audio analysis.
//!
//! A port of the QM Bar and Beat Tracker (QM DSP, QM Vamp Plugins, KissFFT) and of the non-UI logic of Vixen's
//! `BeatsAndBars` module. The numbers are meant to match the C++ bit for bit, so float versus double, the order of
//! operations and the upstream quirks are all kept. This crate is GPL-2.0-or-later; see the COPYING files beside it.

// The DSP loops keep upstream's indexing, so they can be read side by side with the C++.
#![allow(clippy::needless_range_loop)]

pub mod bar_beat_track;
pub mod beats_and_bars;
pub mod decimator;
pub mod detection_function;
pub mod down_beat;
pub mod fft;
pub mod kissfft;
pub mod math_utilities;
pub mod phase_vocoder;
pub mod samples;
pub mod settings;
pub mod tempo_track_v2;
pub mod vamp;
pub mod window;

pub use bar_beat_track::BarBeatTracker;
pub use beats_and_bars::{
    build_mark_collections, ensure_default_collection, estimate_beat_period, extract_all_marks, extract_bar_marks,
    extract_beat_collections, extract_split_collections, for_each_block, generate_features, preview_data_from_features,
    AnalysisError, AnalysisProgress, BeatsAndBars,
};
pub use samples::{float_samples_from_raw, raw_audio_bytes, RawSampleFormat, BYTES_PER_SAMPLE};
pub use settings::{calc_note_size, BeatBarPreviewData, BeatBarSettingsData, BeatsAndBarsDialogModel};
pub use vamp::{Feature, FeatureSet, InputDomain, OutputDescriptor, ParameterDescriptor, RealTime, SampleType};

/// The cancel flag was set while analysis ran.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Cancelled;
