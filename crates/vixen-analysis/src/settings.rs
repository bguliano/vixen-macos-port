//! Port of `BeatBarSettingsData` and `BeatBarPreviewData` (BeatsAndBars.cs), and the logic of `BeatsAndBarsDialog`
//! (BeatsAndBarsSettings.cs) and `MusicStaff` (MusicStaff.cs) with the drawing left out.
//!
//! None of this is saved upstream: `BeatBarSettingsData` has no `[DataContract]`, and the dialog keeps it in a static
//! field that its constructor overwrites from freshly built controls, so each open starts from these defaults. What
//! persists is the mark collections themselves, inside the sequence file. Field names stay the C# names in snake case.

use vixen_core::{Color, MarkCollection, TimeSpan};

#[derive(Clone, PartialEq, Debug)]
pub struct BeatBarPreviewData {
    pub beat_period: f64,
    pub preview_collection: MarkCollection,
    pub preview_split_collection: MarkCollection,
}

#[derive(Clone, PartialEq, Debug)]
pub struct BeatBarSettingsData {
    pub bars_enabled: bool,
    pub beat_collections_enabled: bool,
    pub beat_splits_enabled: bool,
    pub all_features_enabled: bool,
    pub collection_base_name: String,
    pub divisions: i32,
    pub all_features_color: Color,
    pub bars_color: Color,
    pub beat_counts_color: Color,
    pub beat_splits_color: Color,
    pub beats_per_bar: i32,
    pub note_size: i32,
    pub preview_data: Option<BeatBarPreviewData>,
}

impl BeatBarSettingsData {
    pub fn new(collection_base_name: &str) -> BeatBarSettingsData {
        BeatBarSettingsData {
            bars_enabled: false,
            beat_collections_enabled: false,
            beat_splits_enabled: false,
            all_features_enabled: false,
            collection_base_name: collection_base_name.to_string(),
            divisions: 1,
            all_features_color: Color::WHITE,
            bars_color: Color::WHITE,
            beat_counts_color: Color::WHITE,
            beat_splits_color: Color::WHITE,
            beats_per_bar: 0,
            note_size: 0,
            preview_data: None,
        }
    }

    pub fn all_collection_name(&self) -> String {
        format!("{} - All", self.collection_base_name)
    }

    pub fn bars_collection_name(&self) -> String {
        format!("{} 1/{} Beat #1 (Bar)", self.collection_base_name, self.note_size)
    }

    /// The numbering always assumes two divisions: with one division and splits on, four beats give
    /// "#1a", "#1b", "#2a", "#2b". The dialog allows that combination, so it is kept.
    pub fn beat_collection_names(&self, add_divisions: bool) -> Vec<String> {
        // `new string[n]` throws for a negative n; here there are simply no names.
        let collections = (self.beats_per_bar * if add_divisions { self.divisions } else { 1 }).max(0);

        (0..collections)
            .map(|j| {
                let col_num = (if add_divisions { j / 2 } else { j }) + 1;
                let suffix = match (add_divisions, j % 2 == 0) {
                    (true, true) => "a",
                    (true, false) => "b",
                    (false, _) => "",
                };
                format!("{} Beat #{}{}", self.collection_base_name, col_num, suffix)
            })
            .collect()
    }
}

/// `MusicStaff.CalcNoteSize`.
pub fn calc_note_size(bpb: i32) -> i32 {
    if bpb == 2 {
        2
    } else if bpb <= 4 {
        4
    } else if bpb <= 8 {
        8
    } else if bpb <= 16 {
        16
    } else {
        0
    }
}

/// What `BeatsAndBarsDialog` and its `MusicStaff` hold, for a front end to bind to.
#[derive(Clone, PartialEq, Debug)]
pub struct BeatsAndBarsDialogModel {
    pub bars_checked: bool,
    pub beat_counts_checked: bool,
    pub beat_splits_checked: bool,
    pub all_features_checked: bool,

    pub bars_color_enabled: bool,
    pub beat_counts_color_enabled: bool,
    pub beat_splits_color_enabled: bool,
    pub all_color_enabled: bool,
    pub beat_splits_checkbox_enabled: bool,

    pub collection_base_name: String,
    pub all_features_color: Color,
    pub bars_color: Color,
    pub beat_counts_color: Color,
    pub beat_splits_color: Color,

    pub beats_per_bar: i32,
    pub split_beats: bool,
    pub beat_period: f64,
}

impl Default for BeatsAndBarsDialogModel {
    /// The state after the dialog's constructor, with the designer's colours and name.
    fn default() -> BeatsAndBarsDialogModel {
        BeatsAndBarsDialogModel {
            bars_checked: true,
            beat_counts_checked: true,
            beat_splits_checked: false,
            all_features_checked: true,

            bars_color_enabled: true,
            beat_counts_color_enabled: true,
            // The constructor sets BeatSplitsCB.Checked to false, its default, so no change event disables these.
            beat_splits_color_enabled: true,
            all_color_enabled: true,
            beat_splits_checkbox_enabled: true,

            collection_base_name: "Beats".to_string(),
            all_features_color: Color::WHITE,
            bars_color: Color::rgb(255, 255, 128),
            beat_counts_color: Color::rgb(255, 128, 0),
            beat_splits_color: Color::LIME,

            beats_per_bar: 4,
            split_beats: false,
            beat_period: 0.0,
        }
    }
}

impl BeatsAndBarsDialogModel {
    pub fn set_bars_checked(&mut self, checked: bool) {
        self.bars_checked = checked;
        self.bars_color_enabled = checked;
    }

    pub fn set_beat_counts_checked(&mut self, checked: bool) {
        self.beat_counts_checked = checked;
        self.beat_counts_color_enabled = checked;
    }

    pub fn set_beat_splits_checked(&mut self, checked: bool) {
        self.beat_splits_checked = checked;
        self.beat_splits_color_enabled = checked;
    }

    pub fn set_all_features_checked(&mut self, checked: bool) {
        self.all_features_checked = checked;
        self.all_color_enabled = checked;
    }

    /// The Split Beats box: `MusicStaff.splitBeatsCB_CheckedChanged`, which raises `SettingChanged`.
    pub fn set_split_beats(&mut self, split_beats: bool) {
        self.split_beats = split_beats;
        self.music_staff_settings_changed();
    }

    /// Choosing a time signature: `MusicStaff.ContextTsChanged`, which raises `SettingChanged`.
    pub fn set_beats_per_bar(&mut self, beats_per_bar: i32) {
        self.beats_per_bar = beats_per_bar;
        self.music_staff_settings_changed();
    }

    fn music_staff_settings_changed(&mut self) {
        // The check box change event runs first, then these overwrite what it did.
        self.set_beat_splits_checked(self.split_beats);
        self.beat_splits_color_enabled = self.split_beats;
        self.beat_splits_checkbox_enabled = self.split_beats;
    }

    pub fn note_size(&self) -> i32 {
        calc_note_size(self.beats_per_bar)
    }

    pub fn divisions(&self) -> i32 {
        if self.split_beats {
            2
        } else {
            1
        }
    }

    /// `GenerateButton.Enabled`.
    pub fn generate_enabled(&self) -> bool {
        self.all_features_checked || self.bars_checked || self.beat_counts_checked || self.beat_splits_checked
    }

    /// `SetBeatBarOutputSettings`.
    pub fn settings(&self) -> BeatBarSettingsData {
        BeatBarSettingsData {
            bars_enabled: self.bars_checked,
            beat_collections_enabled: self.beat_counts_checked,
            beat_splits_enabled: self.beat_splits_checked,
            all_features_enabled: self.all_features_checked,
            collection_base_name: self.collection_base_name.clone(),
            divisions: self.divisions(),
            all_features_color: self.all_features_color,
            bars_color: self.bars_color,
            beat_counts_color: self.beat_counts_color,
            beat_splits_color: self.beat_splits_color,
            beats_per_bar: self.beats_per_bar,
            note_size: self.note_size(),
            preview_data: None,
        }
    }

    /// `BPMLabelVal`: `60000 / beatPeriod`. Infinite until there is a period, as in the C#.
    pub fn bpm(&self) -> f64 {
        60000.0 / self.beat_period
    }

    /// `BarPeriodLabelVal`, in seconds.
    pub fn bar_period_seconds(&self) -> f64 {
        (self.beat_period * self.beats_per_bar as f64) / 1000.0
    }

    /// `DivTimeLabelVal`, in milliseconds.
    pub fn div_time_ms(&self) -> f64 {
        if self.split_beats {
            self.beat_period / 2.0
        } else {
            self.beat_period
        }
    }

    /// `UpdatePreviewWaveform`: the times the preview draws lines at.
    pub fn preview_marks(&self, preview: &BeatBarPreviewData) -> Vec<TimeSpan> {
        let collection = if self.split_beats { &preview.preview_split_collection } else { &preview.preview_collection };
        collection.marks.iter().map(|m| m.start_time).collect()
    }

    /// `UpdatePreviewWaveform`: the length the preview covers, which comes from the split marks either way. `None` when
    /// there are none, where the C# leaves the previous value in place.
    pub fn preview_period(preview: &BeatBarPreviewData) -> Option<TimeSpan> {
        preview.preview_split_collection.marks.iter().map(|m| m.start_time).max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::Mark;

    // Port of BeatBarSettingsDataTests.BeatCollectionNames_ReturnsExpectedNamesForStandardAndSplitBeats.
    #[test]
    fn beat_collection_names_returns_expected_names_for_standard_and_split_beats() {
        let mut settings = BeatBarSettingsData::new("Beats");
        settings.beats_per_bar = 4;
        settings.divisions = 2;
        settings.note_size = 4;

        assert_eq!(
            settings.beat_collection_names(false),
            vec!["Beats Beat #1", "Beats Beat #2", "Beats Beat #3", "Beats Beat #4"]
        );
        assert_eq!(
            settings.beat_collection_names(true),
            vec![
                "Beats Beat #1a",
                "Beats Beat #1b",
                "Beats Beat #2a",
                "Beats Beat #2b",
                "Beats Beat #3a",
                "Beats Beat #3b",
                "Beats Beat #4a",
                "Beats Beat #4b",
            ]
        );
    }

    #[test]
    fn split_names_with_one_division_keep_the_two_division_numbering() {
        let mut settings = BeatBarSettingsData::new("X");
        settings.beats_per_bar = 4;
        settings.divisions = 1;
        assert_eq!(settings.beat_collection_names(true), vec!["X Beat #1a", "X Beat #1b", "X Beat #2a", "X Beat #2b"]);
    }

    #[test]
    fn a_negative_count_gives_no_names() {
        let mut settings = BeatBarSettingsData::new("X");
        settings.beats_per_bar = -1;
        assert!(settings.beat_collection_names(false).is_empty());
    }

    #[test]
    fn collection_names() {
        let mut settings = BeatBarSettingsData::new("Beats");
        settings.note_size = 8;
        assert_eq!(settings.all_collection_name(), "Beats - All");
        assert_eq!(settings.bars_collection_name(), "Beats 1/8 Beat #1 (Bar)");
    }

    #[test]
    fn constructor_defaults() {
        let settings = BeatBarSettingsData::new("Preview");
        assert!(!settings.bars_enabled && !settings.beat_collections_enabled);
        assert!(!settings.beat_splits_enabled && !settings.all_features_enabled);
        assert_eq!(settings.divisions, 1);
        assert_eq!(settings.bars_color, Color::WHITE);
        assert_eq!(settings.beats_per_bar, 0);
        assert!(settings.preview_data.is_none());
    }

    #[test]
    fn note_sizes_for_each_time_signature() {
        let expected = [(2, 2), (3, 4), (4, 4), (5, 8), (6, 8), (7, 8), (8, 8), (9, 16), (12, 16), (16, 16), (17, 0)];
        for (bpb, note) in expected {
            assert_eq!(calc_note_size(bpb), note, "bpb {bpb}");
        }
    }

    #[test]
    fn dialog_starts_as_the_constructor_leaves_it() {
        let model = BeatsAndBarsDialogModel::default();
        let settings = model.settings();
        assert!(settings.bars_enabled && settings.beat_collections_enabled && settings.all_features_enabled);
        assert!(!settings.beat_splits_enabled);
        assert_eq!(settings.collection_base_name, "Beats");
        assert_eq!(settings.divisions, 1);
        assert_eq!(settings.beats_per_bar, 4);
        assert_eq!(settings.note_size, 4);
        assert_eq!(settings.bars_color, Color::rgb(255, 255, 128));
        assert_eq!(settings.beat_counts_color, Color::rgb(255, 128, 0));
        assert_eq!(settings.beat_splits_color, Color::rgb(0, 255, 0));
        assert_eq!(settings.all_features_color, Color::WHITE);
        assert!(model.generate_enabled());
        assert!(model.beat_splits_checkbox_enabled);
    }

    #[test]
    fn generate_needs_one_output_ticked() {
        let mut model = BeatsAndBarsDialogModel::default();
        model.set_bars_checked(false);
        model.set_beat_counts_checked(false);
        model.set_all_features_checked(false);
        assert!(!model.generate_enabled());
        assert!(!model.bars_color_enabled && !model.beat_counts_color_enabled && !model.all_color_enabled);
        model.set_beat_splits_checked(true);
        assert!(model.generate_enabled());
    }

    #[test]
    fn split_beats_drives_the_split_checkbox() {
        let mut model = BeatsAndBarsDialogModel::default();

        model.set_split_beats(true);
        assert!(model.beat_splits_checked && model.beat_splits_checkbox_enabled && model.beat_splits_color_enabled);
        assert_eq!(model.settings().divisions, 2);

        model.set_split_beats(false);
        assert!(!model.beat_splits_checked && !model.beat_splits_checkbox_enabled && !model.beat_splits_color_enabled);
        assert_eq!(model.settings().divisions, 1);
    }

    #[test]
    fn changing_time_signature_recomputes_note_size_and_rechecks_splits() {
        let mut model = BeatsAndBarsDialogModel::default();
        model.set_split_beats(true);
        model.set_beat_splits_checked(false);
        model.set_beats_per_bar(7);
        assert!(model.beat_splits_checked);
        let settings = model.settings();
        assert_eq!(settings.beats_per_bar, 7);
        assert_eq!(settings.note_size, 8);
    }

    #[test]
    fn music_staff_read_outs() {
        let mut model = BeatsAndBarsDialogModel { beat_period: 500.0, ..Default::default() };
        assert_eq!(model.bpm(), 120.0);
        assert_eq!(model.bar_period_seconds(), 2.0);
        assert_eq!(model.div_time_ms(), 500.0);
        model.set_split_beats(true);
        assert_eq!(model.div_time_ms(), 250.0);
    }

    #[test]
    fn preview_uses_the_collection_for_the_split_setting() {
        let times = |ms: &[i64]| {
            let mut c = MarkCollection::default();
            for &t in ms {
                c.add_mark(Mark::new(TimeSpan::from_ms_i(t)));
            }
            c
        };
        let preview = BeatBarPreviewData {
            beat_period: 500.0,
            preview_collection: times(&[0, 500]),
            preview_split_collection: times(&[0, 250, 500, 750]),
        };
        let mut model = BeatsAndBarsDialogModel::default();
        assert_eq!(model.preview_marks(&preview), vec![TimeSpan::from_ms_i(0), TimeSpan::from_ms_i(500)]);
        model.set_split_beats(true);
        assert_eq!(model.preview_marks(&preview).len(), 4);
        assert_eq!(BeatsAndBarsDialogModel::preview_period(&preview), Some(TimeSpan::from_ms_i(750)));
    }
}
