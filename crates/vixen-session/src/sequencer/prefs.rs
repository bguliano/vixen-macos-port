//! The sequencer's user settings: the `AppSettings/TimedSequenceEditorForm/` keys of `Settings.xml`, read once when a window
//! opens and written once when it closes. See "Data and persistence" in `docs/spec/sequencer-shell.md`.

use vixen_io::{IoError, ProfileSettings, SettingSection};

const FORM: &str = "TimedSequenceEditorForm";

/// One field per key. Values that don't parse give the default (the spec's GUARD), which `ProfileSettings` already does.
#[derive(Clone, Debug, PartialEq)]
pub struct EditorPrefs {
    /// `Preferences/.../AutoSaveInterval`, which nothing in the original writes, so it's 5 minutes unless hand edited.
    pub auto_save_interval_ms: i32,
    pub dock_left_portion: i32,
    pub dock_right_portion: i32,
    pub auto_save_enabled: bool,
    pub snap_to_selected: bool,
    pub full_waveform: bool,
    pub snap_strength: i32,
    pub close_gap_threshold: String,
    pub align_to_threshold: String,
    pub resize_indicator_enabled: bool,
    pub resize_indicator_color: String,
    pub draw_mode_selected: bool,
    pub selection_mode_selected: bool,
    pub curve_link_curves: bool,
    pub gradient_link_gradients: bool,
    pub cad_style_selection_box: bool,
    pub legacy_cursor_active_row: bool,
    pub zoom_under_mouse_position: bool,
    pub highlight_active_elements: bool,
    pub wave_form_height: i32,
    pub ruler_height: i32,
    pub lock_ruler_height: bool,
    pub lock_waveform_height: bool,
    pub show_effect_info: bool,
    pub use_tempo_for_speed: bool,
    pub splitter_distance: i32,
    pub window_width: i32,
    pub window_height: i32,
    pub window_location_x: i32,
    pub window_location_y: i32,
    pub window_state: String,
    pub effect_label_index: String,
    pub basic_effect_strip: bool,
    pub pixel_effect_strip: bool,
    pub device_effect_strip: bool,
    pub fixture_effect_strip: bool,
}

impl Default for EditorPrefs {
    fn default() -> Self {
        EditorPrefs {
            auto_save_interval_ms: 300_000,
            dock_left_portion: 150,
            dock_right_portion: 150,
            auto_save_enabled: true,
            snap_to_selected: true,
            full_waveform: false,
            snap_strength: 2,
            close_gap_threshold: ".100".to_string(),
            align_to_threshold: ".800".to_string(),
            resize_indicator_enabled: false,
            resize_indicator_color: "Red".to_string(),
            draw_mode_selected: false,
            selection_mode_selected: true,
            curve_link_curves: false,
            gradient_link_gradients: false,
            cad_style_selection_box: false,
            legacy_cursor_active_row: false,
            zoom_under_mouse_position: false,
            highlight_active_elements: false,
            wave_form_height: 50,
            ruler_height: 50,
            lock_ruler_height: false,
            lock_waveform_height: false,
            show_effect_info: true,
            use_tempo_for_speed: false,
            splitter_distance: 200,
            window_width: 0,
            window_height: 0,
            window_location_x: 0,
            window_location_y: 0,
            window_state: "Normal".to_string(),
            effect_label_index: "2".to_string(),
            basic_effect_strip: true,
            pixel_effect_strip: true,
            device_effect_strip: true,
            fixture_effect_strip: true,
        }
    }
}

fn key(name: &str) -> String {
    format!("{FORM}/{name}")
}

impl EditorPrefs {
    pub fn read(settings: &ProfileSettings) -> EditorPrefs {
        use SettingSection::{AppSettings, Preferences};
        let d = EditorPrefs::default();
        let int = |name: &str, default: i32| settings.get_i32(AppSettings, &key(name), default);
        let flag = |name: &str, default: bool| settings.get_bool(AppSettings, &key(name), default);
        let text = |name: &str, default: &str| settings.get_string(AppSettings, &key(name), default);
        EditorPrefs {
            auto_save_interval_ms: settings.get_i32(Preferences, &key("AutoSaveInterval"), d.auto_save_interval_ms),
            dock_left_portion: int("DockLeftPortion", d.dock_left_portion),
            dock_right_portion: int("DockRightPortion", d.dock_right_portion),
            auto_save_enabled: flag("AutoSaveEnabled", d.auto_save_enabled),
            snap_to_selected: flag("SnapToSelected", d.snap_to_selected),
            full_waveform: flag("FullWaveform", d.full_waveform),
            snap_strength: int("SnapStrength", d.snap_strength),
            close_gap_threshold: text("CloseGapThreshold", &d.close_gap_threshold),
            align_to_threshold: text("AlignToThreshold", &d.align_to_threshold),
            resize_indicator_enabled: flag("ResizeIndicatorEnabled", d.resize_indicator_enabled),
            resize_indicator_color: text("ResizeIndicatorColor", &d.resize_indicator_color),
            draw_mode_selected: flag("DrawModeSelected", d.draw_mode_selected),
            selection_mode_selected: flag("SelectionModeSelected", d.selection_mode_selected),
            curve_link_curves: flag("CurveLinkCurves", d.curve_link_curves),
            gradient_link_gradients: flag("GradientLinkGradients", d.gradient_link_gradients),
            cad_style_selection_box: flag("CadStyleSelectionBox", d.cad_style_selection_box),
            legacy_cursor_active_row: flag("LegacyCursorActiveRow", d.legacy_cursor_active_row),
            zoom_under_mouse_position: flag("ZoomUnderMousePosition", d.zoom_under_mouse_position),
            highlight_active_elements: flag("HighlightActiveElements", d.highlight_active_elements),
            wave_form_height: int("WaveFormHeight", d.wave_form_height),
            ruler_height: int("RulerHeight", d.ruler_height),
            lock_ruler_height: flag("LockRulerHeight", d.lock_ruler_height),
            lock_waveform_height: flag("LockWaveformHeight", d.lock_waveform_height),
            show_effect_info: flag("ShowEffectInfo", d.show_effect_info),
            use_tempo_for_speed: flag("UseTempoForSpeed", d.use_tempo_for_speed),
            splitter_distance: int("SplitterDistance", d.splitter_distance),
            window_width: int("WindowWidth", d.window_width),
            window_height: int("WindowHeight", d.window_height),
            window_location_x: int("WindowLocationX", d.window_location_x),
            window_location_y: int("WindowLocationY", d.window_location_y),
            window_state: text("WindowState", &d.window_state),
            effect_label_index: text("EffectToolStrip/EffectLabelIndex", &d.effect_label_index),
            basic_effect_strip: flag("EffectToolStrip/BasicEffectToolStrip", d.basic_effect_strip),
            pixel_effect_strip: flag("EffectToolStrip/PixelEffectToolStrip", d.pixel_effect_strip),
            device_effect_strip: flag("EffectToolStrip/DeviceEffectToolStrip", d.device_effect_strip),
            // Upstream reads this key and writes `IntelligentFixtureEffectToolStrip`, so the group always restores checked. One key
            // here (F163's FIX).
            fixture_effect_strip: flag("EffectToolStrip/FixtureEffectToolStrip", d.fixture_effect_strip),
        }
    }

    /// `EditorClosing` and `Save_ToolsStripItemsFile`, as one write instead of about sixty.
    pub fn write(&self, settings: &ProfileSettings) -> Result<(), IoError> {
        let int = |name: &str, value: i32| (key(name), value.to_string());
        let flag = |name: &str, value: bool| (key(name), if value { "True" } else { "False" }.to_string());
        let text = |name: &str, value: &str| (key(name), value.to_string());
        let values = vec![
            int("DockLeftPortion", self.dock_left_portion),
            int("DockRightPortion", self.dock_right_portion),
            flag("AutoSaveEnabled", self.auto_save_enabled),
            flag("DrawModeSelected", self.draw_mode_selected),
            flag("SelectionModeSelected", self.selection_mode_selected),
            flag("SnapToSelected", self.snap_to_selected),
            int("WindowHeight", self.window_height),
            int("WindowWidth", self.window_width),
            int("WindowLocationX", self.window_location_x),
            int("WindowLocationY", self.window_location_y),
            text("WindowState", &self.window_state),
            int("SnapStrength", self.snap_strength),
            text("CloseGapThreshold", &self.close_gap_threshold),
            text("AlignToThreshold", &self.align_to_threshold),
            flag("ResizeIndicatorEnabled", self.resize_indicator_enabled),
            flag("CadStyleSelectionBox", self.cad_style_selection_box),
            flag("LegacyCursorActiveRow", self.legacy_cursor_active_row),
            text("ResizeIndicatorColor", &self.resize_indicator_color),
            flag("CurveLinkCurves", self.curve_link_curves),
            flag("GradientLinkGradients", self.gradient_link_gradients),
            flag("ZoomUnderMousePosition", self.zoom_under_mouse_position),
            flag("HighlightActiveElements", self.highlight_active_elements),
            int("WaveFormHeight", self.wave_form_height),
            int("RulerHeight", self.ruler_height),
            flag("LockRulerHeight", self.lock_ruler_height),
            flag("LockWaveformHeight", self.lock_waveform_height),
            int("SplitterDistance", self.splitter_distance),
            flag("ShowEffectInfo", self.show_effect_info),
            flag("FullWaveform", self.full_waveform),
            flag("UseTempoForSpeed", self.use_tempo_for_speed),
            text("EffectToolStrip/EffectLabelIndex", &self.effect_label_index),
            flag("EffectToolStrip/BasicEffectToolStrip", self.basic_effect_strip),
            flag("EffectToolStrip/PixelEffectToolStrip", self.pixel_effect_strip),
            flag("EffectToolStrip/DeviceEffectToolStrip", self.device_effect_strip),
            flag("EffectToolStrip/FixtureEffectToolStrip", self.fixture_effect_strip),
        ];
        let refs: Vec<(&str, String)> = values.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        settings.put_many(SettingSection::AppSettings, &refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_settings(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-prefs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("Settings.xml")
    }

    #[test]
    fn missing_file_gives_the_tables_defaults() {
        let s = ProfileSettings::open(temp_settings("defaults"));
        let p = EditorPrefs::read(&s);
        assert_eq!(p, EditorPrefs::default());
        assert_eq!(p.auto_save_interval_ms, 300_000);
        assert_eq!(p.close_gap_threshold, ".100");
        assert_eq!(p.align_to_threshold, ".800");
        assert_eq!(p.resize_indicator_color, "Red");
        assert!(p.auto_save_enabled && p.snap_to_selected && p.show_effect_info && p.selection_mode_selected);
        assert!(!p.draw_mode_selected);
    }

    #[test]
    fn round_trips_every_key_with_upstream_booleans_and_keeps_unknown_keys() {
        let path = temp_settings("trip");
        let s = ProfileSettings::open(&path);
        s.put_string(SettingSection::AppSettings, "SomeOtherWindow/Thing", "kept").unwrap();
        let p = EditorPrefs {
            dock_left_portion: 210,
            dock_right_portion: 320,
            auto_save_enabled: false,
            snap_to_selected: false,
            full_waveform: true,
            snap_strength: 4,
            close_gap_threshold: ".300".into(),
            align_to_threshold: "2.000".into(),
            resize_indicator_enabled: true,
            resize_indicator_color: "Blue".into(),
            draw_mode_selected: true,
            selection_mode_selected: false,
            curve_link_curves: true,
            gradient_link_gradients: true,
            cad_style_selection_box: true,
            legacy_cursor_active_row: true,
            zoom_under_mouse_position: true,
            highlight_active_elements: true,
            wave_form_height: 77,
            ruler_height: 66,
            lock_ruler_height: true,
            lock_waveform_height: true,
            show_effect_info: false,
            use_tempo_for_speed: true,
            splitter_distance: 345,
            window_width: 1000,
            window_height: 700,
            window_location_x: 10,
            window_location_y: 20,
            window_state: "Maximized".into(),
            effect_label_index: "0".into(),
            basic_effect_strip: false,
            pixel_effect_strip: false,
            device_effect_strip: false,
            fixture_effect_strip: false,
            ..EditorPrefs::default()
        };
        p.write(&s).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(">False<") && text.contains(">True<"), "upstream writes True and False");
        let again = ProfileSettings::open(&path);
        assert_eq!(EditorPrefs::read(&again), p);
        assert_eq!(again.get_string(SettingSection::AppSettings, "SomeOtherWindow/Thing", ""), "kept");
    }

    #[test]
    fn values_that_do_not_parse_give_the_default() {
        let s = ProfileSettings::open(temp_settings("bad"));
        s.put_string(SettingSection::AppSettings, "TimedSequenceEditorForm/SnapStrength", "lots").unwrap();
        s.put_string(SettingSection::AppSettings, "TimedSequenceEditorForm/AutoSaveEnabled", "maybe").unwrap();
        let p = EditorPrefs::read(&s);
        assert_eq!(p.snap_strength, 2);
        assert!(p.auto_save_enabled);
    }

    #[test]
    fn fixture_group_uses_one_key() {
        let s = ProfileSettings::open(temp_settings("fixture"));
        let p = EditorPrefs { fixture_effect_strip: false, ..EditorPrefs::default() };
        p.write(&s).unwrap();
        assert!(!EditorPrefs::read(&s).fixture_effect_strip);
    }
}
