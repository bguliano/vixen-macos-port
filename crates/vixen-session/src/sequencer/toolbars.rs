//! The twelve tool strips: their rows, order, visibility, image size and which items are shown. F98 to F104, F177. The band itself is
//! SwiftUI; the rules and the settings live here.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use vixen_io::file::write_atomic;
use vixen_io::{IoError, ProfileSettings, SettingSection};
use vixen_xml::XElement;

const FORM: &str = "TimedSequenceEditorForm";

/// A tool strip.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum StripId {
    Alignment,
    Audio,
    Edit,
    Effects,
    File,
    LibraryColor,
    LibraryCurve,
    LibraryGradient,
    Mode,
    PlayBack,
    Tools,
    View,
}

impl StripId {
    /// The Toolbars list's order: alphabetical by `Text`. F99.
    pub const ALL: [StripId; 12] = [
        StripId::Alignment,
        StripId::Audio,
        StripId::Edit,
        StripId::Effects,
        StripId::File,
        StripId::LibraryColor,
        StripId::LibraryCurve,
        StripId::LibraryGradient,
        StripId::Mode,
        StripId::PlayBack,
        StripId::Tools,
        StripId::View,
    ];

    /// The control's `Name`, which the settings and the state files key on.
    pub fn name(self) -> &'static str {
        match self {
            StripId::Alignment => "toolStripAlignment",
            StripId::Audio => "toolStripAudio",
            StripId::Edit => "toolStripEdit",
            StripId::Effects => "toolStripEffects",
            StripId::File => "toolStripFile",
            StripId::LibraryColor => "toolStripColorLibrary",
            StripId::LibraryCurve => "toolStripCurveLibrary",
            StripId::LibraryGradient => "toolStripGradientLibrary",
            StripId::Mode => "toolStripMode",
            StripId::PlayBack => "toolStripPlayBack",
            StripId::Tools => "toolStripTools",
            StripId::View => "toolStripView",
        }
    }

    pub fn from_name(name: &str) -> Option<StripId> {
        StripId::ALL.into_iter().find(|s| s.name() == name)
    }

    /// `Text`, as the Toolbars list shows it.
    pub fn title(self) -> &'static str {
        match self {
            StripId::Alignment => "Alignment",
            StripId::Audio => "Audio",
            StripId::Edit => "Edit",
            StripId::Effects => "Effects",
            StripId::File => "File",
            StripId::LibraryColor => "Library Color",
            StripId::LibraryCurve => "Library Curve",
            StripId::LibraryGradient => "Library Gradient",
            StripId::Mode => "Mode",
            StripId::PlayBack => "PlayBack",
            StripId::Tools => "Tools",
            StripId::View => "View",
        }
    }

    pub fn is_library(self) -> bool {
        matches!(self, StripId::LibraryColor | StripId::LibraryCurve | StripId::LibraryGradient)
    }

    /// The designer's default.
    fn visible_by_default(self) -> bool {
        matches!(self, StripId::Audio | StripId::View | StripId::Edit | StripId::Mode | StripId::PlayBack)
    }

    /// The `ToolStrip/<Strip>` size key. The Effects strip has no size toggle.
    fn size_key(self) -> Option<&'static str> {
        Some(match self {
            StripId::Alignment => "Alignment",
            StripId::Audio => "Audio",
            StripId::Edit => "Edit",
            StripId::Effects => return None,
            StripId::File => "File",
            StripId::LibraryColor => "ColorLibrary",
            StripId::LibraryCurve => "CurveLibrary",
            StripId::LibraryGradient => "GradientLibrary",
            StripId::Mode => "Mode",
            StripId::PlayBack => "PlayBack",
            StripId::Tools => "Tools",
            StripId::View => "View",
        })
    }

    /// `ToolStrip.Items` with each item's tooltip, for Show/Hide Items. Library and Effects strips list what they hold at the time.
    pub fn items(self) -> &'static [(&'static str, &'static str)] {
        match self {
            StripId::PlayBack => &[
                ("playBackToolStripButton_Start", "Start"),
                ("playBackToolStripButton_Play", "Play F5"),
                ("playBackToolStripButton_Stop", "Stop F8"),
                ("playBackToolStripButton_End", "End"),
                ("playBackToolStripButton_Loop", "Loop F9"),
            ],
            StripId::Edit => &[
                ("editToolStripButton_Undo", "Undo"),
                ("editToolStripButton_Redo", "Redo"),
                ("editToolStripButton_Cut", "Cut"),
                ("editToolStripButton_Copy", "Copy"),
                ("editToolStripButton_Paste", "Paste"),
                ("editToolStripButton_PasteDropDown", "Paste Special"),
            ],
            StripId::Mode => &[
                ("modeToolStripButton_DrawMode", "Draw Mode"),
                ("modeToolStripButton_SelectionMode", "Selection Mode"),
                ("modeToolStripButton_SnapTo", "Snap To Marks / Effect"),
                ("modeToolStripDropDownButton_SnapToStrength", "Snap Strength"),
                ("modeToolStripButton_DragBoxFilter", "Drag Box Filter"),
                ("modeToolStripDropDownButton_DragBoxFilter", "Drag Box Filter"),
            ],
            StripId::View => &[
                ("viewToolStripButton_ZoomTimeIn", "Zoom Time In"),
                ("viewToolStripButton_ZoomTimeOut", "Zoom Time Out"),
                ("viewToolStripButton_ZoomRowIn", "Zoom Row In"),
                ("viewToolStripButton_ZoomRowOut", "Zoom Row Out"),
            ],
            StripId::Audio => &[
                ("toolStripButton_AssociateAudio", "Associate Audio"),
                ("toolStripButton_DecreaseTimingSpeed", "Decrease Timing speed"),
                ("audioToolStripLabel_TimingSpeed", "Timing speed"),
                ("toolStripButton_IncreaseTimingSpeed", "Increasing Timing speed"),
                ("audioToolStripButton_Audio_Devices", "Audio Devices"),
            ],
            StripId::File => &[
                ("fileToolStripButton_Save", "Save"),
                ("fileToolStripButton_SaveAs", "Save As"),
                ("fileToolStripButton_AutoSave", "Auto Save"),
                ("fileToolStripButton_Export", "Export"),
                ("fileToolStripButton_Close", "Close"),
            ],
            StripId::Alignment => &[
                ("alignmentToolStripButton_CloseGaps", "Close Gaps"),
                ("alignmentToolStripDropDownButton_CloseGaps", "Close Gaps"),
                ("alignmentToolStripDropDownButton_AlignTo", "Align To"),
                ("alignmentToolStripButton_Start", "Align Start to nearest mark"),
                ("alignmentToolStripButton_End", "Align End to nearest mark"),
                ("alignmentToolStripButton_Both", "Align Both to nearest mark"),
                ("alignmentToolStripButton_Distribute", "Distribute Mark"),
            ],
            StripId::Tools => &[
                ("toolsToolStripButton_LipSync", "Edit LipSync Map"),
                ("toolsToolStripButton_CurveLibrary", "Curve Library"),
                ("toolsToolStripButton_ColorLibrary", "Color Library"),
                ("toolsToolStripButton_ColorGradient", "Color Gradient Library"),
            ],
            StripId::Effects | StripId::LibraryColor | StripId::LibraryCurve | StripId::LibraryGradient => &[],
        }
    }
}

/// The strips by row, top to bottom, and each row's order. Every strip is in exactly one row, shown or not.
#[derive(Clone, PartialEq, Debug)]
pub struct ToolbarLayout {
    pub rows: Vec<Vec<StripId>>,
    pub visible: HashMap<StripId, bool>,
    /// Large means 28 points with the image above the text; small is 16. `ChangeToolStripImageSize`.
    pub large: HashMap<StripId, bool>,
}

impl ToolbarLayout {
    /// F98: nothing saved. PlayBack, Edit, Mode, View and Audio side by side, and the rest off, in the designer's order of rows.
    pub fn default_layout() -> ToolbarLayout {
        ToolbarLayout {
            rows: vec![
                vec![StripId::PlayBack, StripId::Edit, StripId::Mode, StripId::View, StripId::Audio],
                vec![StripId::Effects, StripId::Alignment],
                vec![StripId::File, StripId::Tools],
                vec![StripId::LibraryColor, StripId::LibraryCurve, StripId::LibraryGradient],
            ],
            visible: StripId::ALL.into_iter().map(|s| (s, s.visible_by_default())).collect(),
            large: StripId::ALL.into_iter().map(|s| (s, !s.is_library())).collect(),
        }
    }

    pub fn is_visible(&self, strip: StripId) -> bool {
        self.visible.get(&strip).copied().unwrap_or(false)
    }

    pub fn is_large(&self, strip: StripId) -> bool {
        self.large.get(&strip).copied().unwrap_or(false)
    }

    /// `toolbarToolStripMenuItem` and its copies: one source of truth (F99's FIX).
    pub fn set_visible(&mut self, strip: StripId, visible: bool) {
        self.visible.insert(strip, visible);
    }

    /// Toggle Image Size. The Effects strip has none.
    pub fn toggle_size(&mut self, strip: StripId) {
        if strip.size_key().is_some() {
            let large = self.is_large(strip);
            self.large.insert(strip, !large);
        }
    }

    /// `toolStrip_EndDrag`: dropped into a row at a position, or into a new row at `row == rows.len()`. Empty rows close up, and every
    /// strip packs left because rows hold only order.
    pub fn move_strip(&mut self, strip: StripId, row: usize, index: usize) {
        let mut target_row = row;
        for (r, list) in self.rows.iter_mut().enumerate() {
            if let Some(at) = list.iter().position(|s| *s == strip) {
                list.remove(at);
                if r == row && at < index {
                    self.insert(strip, target_row, index - 1);
                    self.rows.retain(|l| !l.is_empty());
                    return;
                }
                break;
            }
        }
        target_row = target_row.min(self.rows.len());
        self.insert(strip, target_row, index);
        self.rows.retain(|l| !l.is_empty());
    }

    fn insert(&mut self, strip: StripId, row: usize, index: usize) {
        if row >= self.rows.len() {
            self.rows.push(vec![strip]);
            return;
        }
        let list = &mut self.rows[row];
        let index = index.min(list.len());
        list.insert(index, strip);
    }

    /// `InitializeToolBars`: rows by saved Y, order by saved X. A strip the file doesn't mention keeps its default place.
    pub fn restore(settings: &ProfileSettings) -> ToolbarLayout {
        use SettingSection::AppSettings;
        let mut layout = ToolbarLayout::default_layout();
        for strip in StripId::ALL {
            let Some(size) = strip.size_key() else { continue };
            let default = if strip.is_library() { 16 } else { 28 };
            let saved = settings.get_i32(AppSettings, &format!("{FORM}/ToolStrip/{size}"), default);
            // Upstream compares rather than storing a size: anything but the current 16 means large.
            layout.large.insert(strip, saved != 16);
        }
        let count = settings.get_i32(AppSettings, &format!("{FORM}/ToolStrip/ToolStripCount"), 0);
        if count <= 0 {
            return layout;
        }
        let mut placed: Vec<(i32, i32, StripId)> = Vec::new();
        for i in 0..=count {
            let name = settings.get_string(AppSettings, &format!("{FORM}/ToolStripName{i}"), "");
            let Some(strip) = StripId::from_name(&name) else { continue };
            if placed.iter().any(|(.., s)| *s == strip) {
                continue;
            }
            let x = settings.get_i32(AppSettings, &format!("{FORM}/ToolStripX{i}"), 0);
            let y = settings.get_i32(AppSettings, &format!("{FORM}/ToolStripY{i}"), 0);
            let visible = settings.get_bool(AppSettings, &format!("{FORM}/ToolStripVisible{i}"), false);
            layout.visible.insert(strip, visible);
            placed.push((y, x, strip));
        }
        if placed.is_empty() {
            return layout;
        }
        placed.sort_by_key(|(y, x, _)| (*y, *x));
        let mut rows: Vec<Vec<StripId>> = Vec::new();
        let mut last_y = None;
        for (y, _, strip) in &placed {
            if last_y != Some(*y) {
                rows.push(Vec::new());
                last_y = Some(*y);
            }
            rows.last_mut().expect("just pushed").push(*strip);
        }
        for default_row in &ToolbarLayout::default_layout().rows {
            let missing: Vec<StripId> = default_row.iter().copied().filter(|s| !placed.iter().any(|(.., p)| p == s)).collect();
            if !missing.is_empty() {
                rows.push(missing);
            }
        }
        layout.rows = rows;
        layout
    }

    /// `Save_ToolsStripItemsFile`'s layout half. Y and X hold only the order: rows are 40 points apart and strips 300, which is how a
    /// Windows file reads back too.
    pub fn save(&self, settings: &ProfileSettings) -> Result<(), IoError> {
        let mut values: Vec<(String, String)> = Vec::new();
        let mut i = 0;
        for (r, row) in self.rows.iter().enumerate() {
            for (c, strip) in row.iter().enumerate() {
                values.push((format!("{FORM}/ToolStripName{i}"), strip.name().to_string()));
                values.push((format!("{FORM}/ToolStripVisible{i}"), py_bool(self.is_visible(*strip))));
                values.push((format!("{FORM}/ToolStripX{i}"), (3 + c as i32 * 300).to_string()));
                values.push((format!("{FORM}/ToolStripY{i}"), (r as i32 * 40).to_string()));
                i += 1;
            }
        }
        values.push((format!("{FORM}/ToolStrip/ToolStripCount"), i.to_string()));
        for strip in StripId::ALL {
            if let Some(size) = strip.size_key() {
                values.push((format!("{FORM}/ToolStrip/{size}"), if self.is_large(strip) { "28" } else { "16" }.to_string()));
            }
        }
        let refs: Vec<(&str, String)> = values.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        settings.put_many(SettingSection::AppSettings, &refs)
    }
}

fn py_bool(v: bool) -> String {
    if v { "True" } else { "False" }.to_string()
}

/// `effectGroupsToolStripMenuItem`: a button shows when its group and its effect are both checked. F152.
pub fn effect_button_visible(group_on: bool, effect_on: bool) -> bool {
    group_on && effect_on
}

// ------------------------------------------------------------------------------------------------------------------
// Show/Hide Items: `ToolStripState.xml`
// ------------------------------------------------------------------------------------------------------------------

const STATES_NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Editor.TimedSequenceEditor";
const ARRAYS_NS: &str = "http://schemas.microsoft.com/2003/10/Serialization/Arrays";
const STRIP_ENTRY: &str = "KeyValueOfstringArrayOfKeyValueOfstringbooleanty7Ep6D1";
const ITEM_ENTRY: &str = "KeyValueOfstringboolean";

/// `ToolStripStates`: strip name to item name to visible, with the entries in file order.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ToolStripStates {
    pub states: Vec<(String, Vec<(String, bool)>)>,
}

impl ToolStripStates {
    /// An item with no entry is shown.
    pub fn is_item_visible(&self, strip: &str, item: &str) -> bool {
        self.states
            .iter()
            .find(|(s, _)| s == strip)
            .and_then(|(_, items)| items.iter().find(|(i, _)| i == item))
            .is_none_or(|(_, visible)| *visible)
    }

    /// `ToolStripItemState_Changed`. Upstream stores it only if the strip has an entry, so a later profile's library strips never store
    /// anything. Here the entry is made on demand (F177's FIX).
    pub fn set_item_visible(&mut self, strip: &str, item: &str, visible: bool) {
        let index = match self.states.iter().position(|(s, _)| s == strip) {
            Some(i) => i,
            None => {
                self.states.push((strip.to_string(), Vec::new()));
                self.states.len() - 1
            }
        };
        let items = &mut self.states[index].1;
        match items.iter_mut().find(|(i, _)| i == item) {
            Some(entry) => entry.1 = visible,
            None => items.push((item.to_string(), visible)),
        }
    }

    /// Reset Toolbar: every item shown, states true. F104.
    pub fn reset_strip(&mut self, strip: &str) {
        if let Some((_, items)) = self.states.iter_mut().find(|(s, _)| s == strip) {
            for item in items {
                item.1 = true;
            }
        }
    }

    pub fn from_xml(root: &XElement) -> ToolStripStates {
        let mut states = Vec::new();
        let Some(list) = root.child("States") else { return ToolStripStates { states } };
        for entry in list.elements() {
            let Some(strip) = entry.child_text("Key") else { continue };
            let mut items = Vec::new();
            if let Some(value) = entry.child("Value") {
                for item in value.elements() {
                    if let (Some(name), Some(visible)) = (item.child_text("Key"), item.child_text("Value")) {
                        items.push((name, visible.trim().eq_ignore_ascii_case("true")));
                    }
                }
            }
            states.push((strip, items));
        }
        ToolStripStates { states }
    }

    pub fn to_xml(&self) -> XElement {
        let mut root = XElement::new(STATES_NS, "TimedSequenceEditorForm.ToolStripStates");
        root.set_attr("xmlns:i", "http://www.w3.org/2001/XMLSchema-instance");
        let mut list = XElement::new(STATES_NS, "States");
        list.set_attr("xmlns:a", ARRAYS_NS);
        for (strip, items) in &self.states {
            let mut entry = XElement::new(ARRAYS_NS, STRIP_ENTRY);
            entry.push(XElement::with_text(ARRAYS_NS, "Key", strip.clone()));
            let mut value = XElement::new(ARRAYS_NS, "Value");
            for (name, visible) in items {
                let mut item = XElement::new(ARRAYS_NS, ITEM_ENTRY);
                item.push(XElement::with_text(ARRAYS_NS, "Key", name.clone()));
                item.push(XElement::with_text(ARRAYS_NS, "Value", if *visible { "true" } else { "false" }));
                value.push(item);
            }
            entry.push(value);
            list.push(entry);
        }
        root.push(list);
        root
    }
}

fn standard_states_path(dir: &Path) -> PathBuf {
    dir.join("ToolStripState.xml")
}

fn profile_states_path(dir: &Path, profile: &str) -> PathBuf {
    dir.join(format!("{profile}_ToolStripState.xml"))
}

/// `Load_ToolsStripItemsFile`: the per-profile file is read only when the standard one exists. A file that doesn't parse is logged by
/// upstream and ignored.
pub fn load_tool_strip_states(dir: &Path, profile: &str) -> ToolStripStates {
    let Ok(root) = XElement::from_file(&standard_states_path(dir)) else {
        return ToolStripStates::default();
    };
    let mut states = ToolStripStates::from_xml(&root);
    if let Ok(root) = XElement::from_file(&profile_states_path(dir, profile)) {
        states.states.extend(ToolStripStates::from_xml(&root).states);
    }
    states
}

/// `Save_ToolsStripItemsFile`'s state half: library strips go in the per-profile file, because libraries differ between profiles.
pub fn save_tool_strip_states(dir: &Path, profile: &str, states: &ToolStripStates) -> Result<(), IoError> {
    let library = |name: &str| StripId::from_name(name).is_some_and(StripId::is_library);
    let regular = ToolStripStates { states: states.states.iter().filter(|(s, _)| !library(s)).cloned().collect() };
    let by_profile = ToolStripStates { states: states.states.iter().filter(|(s, _)| library(s)).cloned().collect() };
    std::fs::create_dir_all(dir).map_err(|source| IoError::Io { path: dir.to_path_buf(), source })?;
    write_xml(&standard_states_path(dir), &regular.to_xml())?;
    write_xml(&profile_states_path(dir, profile), &by_profile.to_xml())
}

fn write_xml(path: &Path, root: &XElement) -> Result<(), IoError> {
    write_atomic(path, root.to_document().as_bytes())
}

/// Whether an element has anything but text, for tests.
#[cfg(test)]
fn element_names(e: &XElement) -> Vec<String> {
    e.children
        .iter()
        .filter_map(|n| match n {
            vixen_xml::XNode::Element(c) => Some(c.local.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-toolbars-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_match_the_designer() {
        let l = ToolbarLayout::default_layout();
        assert_eq!(l.rows[0], [StripId::PlayBack, StripId::Edit, StripId::Mode, StripId::View, StripId::Audio]);
        for s in [StripId::Audio, StripId::View, StripId::Edit, StripId::Mode, StripId::PlayBack] {
            assert!(l.is_visible(s));
        }
        for s in [StripId::Alignment, StripId::Effects, StripId::File, StripId::Tools, StripId::LibraryColor] {
            assert!(!l.is_visible(s));
        }
        assert!(l.is_large(StripId::PlayBack));
        assert!(!l.is_large(StripId::LibraryCurve));
    }

    #[test]
    fn toolbars_list_is_alphabetical() {
        let titles: Vec<&str> = StripId::ALL.iter().map(|s| s.title()).collect();
        let mut sorted = titles.clone();
        sorted.sort();
        assert_eq!(titles, sorted);
    }

    #[test]
    fn size_compare_rule() {
        let path = temp("size").join("Settings.xml");
        let s = ProfileSettings::open(&path);
        s.put_i32(SettingSection::AppSettings, "TimedSequenceEditorForm/ToolStrip/PlayBack", 16).unwrap();
        s.put_i32(SettingSection::AppSettings, "TimedSequenceEditorForm/ToolStrip/ColorLibrary", 28).unwrap();
        let l = ToolbarLayout::restore(&s);
        assert!(!l.is_large(StripId::PlayBack));
        assert!(l.is_large(StripId::LibraryColor));
        assert!(l.is_large(StripId::Edit));
    }

    #[test]
    fn toggling_size_flips_and_effects_has_none() {
        let mut l = ToolbarLayout::default_layout();
        l.toggle_size(StripId::Edit);
        assert!(!l.is_large(StripId::Edit));
        l.toggle_size(StripId::Edit);
        assert!(l.is_large(StripId::Edit));
        let before = l.is_large(StripId::Effects);
        l.toggle_size(StripId::Effects);
        assert_eq!(l.is_large(StripId::Effects), before);
    }

    #[test]
    fn dragging_a_strip_moves_it_and_closes_up_empty_rows() {
        let mut l = ToolbarLayout::default_layout();
        l.move_strip(StripId::Audio, 0, 0);
        assert_eq!(l.rows[0][0], StripId::Audio);
        // Moving rightward in its own row counts the gap it left.
        l.move_strip(StripId::Audio, 0, 2);
        assert_eq!(l.rows[0], [StripId::PlayBack, StripId::Audio, StripId::Edit, StripId::Mode, StripId::View]);
        // To another row.
        l.move_strip(StripId::Audio, 1, 1);
        assert_eq!(l.rows[1], [StripId::Effects, StripId::Audio, StripId::Alignment]);
        // To a new row at the bottom.
        let bottom = l.rows.len();
        l.move_strip(StripId::Audio, bottom, 0);
        assert_eq!(l.rows.last().unwrap(), &vec![StripId::Audio]);
        // Off a row that had only it: the empty row goes.
        l.move_strip(StripId::Audio, 0, 0);
        assert!(l.rows.iter().all(|r| !r.is_empty()));
        assert_eq!(l.rows[0][0], StripId::Audio);
    }

    #[test]
    fn layout_round_trips_through_settings() {
        let path = temp("trip").join("Settings.xml");
        let s = ProfileSettings::open(&path);
        let mut l = ToolbarLayout::default_layout();
        l.set_visible(StripId::Effects, true);
        l.set_visible(StripId::Audio, false);
        l.toggle_size(StripId::View);
        l.move_strip(StripId::Effects, 0, 1);
        l.save(&s).unwrap();
        assert_eq!(ToolbarLayout::restore(&ProfileSettings::open(&path)), l);
    }

    #[test]
    fn windows_settings_read_by_row_and_position() {
        let path = temp("win").join("Settings.xml");
        let s = ProfileSettings::open(&path);
        use SettingSection::AppSettings as A;
        let f = "TimedSequenceEditorForm";
        s.put_i32(A, &format!("{f}/ToolStrip/ToolStripCount"), 3).unwrap();
        for (i, (name, x, y, vis)) in
            [("toolStripEdit", 400, 5, true), ("toolStripPlayBack", 3, 5, true), ("toolStripEffects", 3, 45, false)].iter().enumerate()
        {
            s.put_string(A, &format!("{f}/ToolStripName{i}"), name).unwrap();
            s.put_i32(A, &format!("{f}/ToolStripX{i}"), *x).unwrap();
            s.put_i32(A, &format!("{f}/ToolStripY{i}"), *y).unwrap();
            s.put_bool(A, &format!("{f}/ToolStripVisible{i}"), *vis).unwrap();
        }
        let l = ToolbarLayout::restore(&s);
        assert_eq!(l.rows[0], [StripId::PlayBack, StripId::Edit]);
        assert_eq!(l.rows[1], [StripId::Effects]);
        assert!(!l.is_visible(StripId::Effects));
        // Strips it doesn't mention keep a place.
        assert!(l.rows.iter().flatten().any(|s| *s == StripId::Mode));
    }

    #[test]
    fn states_split_between_two_files() {
        let dir = temp("split");
        let mut st = ToolStripStates::default();
        st.set_item_visible("toolStripEdit", "editToolStripButton_Cut", false);
        st.set_item_visible("toolStripColorLibrary", "R: 1 G: 2 B: 3", false);
        save_tool_strip_states(&dir, "P", &st).unwrap();
        let standard = XElement::from_file(&dir.join("ToolStripState.xml")).unwrap();
        assert_eq!(ToolStripStates::from_xml(&standard).states.len(), 1);
        let profile = XElement::from_file(&dir.join("P_ToolStripState.xml")).unwrap();
        assert_eq!(ToolStripStates::from_xml(&profile).states[0].0, "toolStripColorLibrary");
        assert_eq!(element_names(&standard), ["States"]);

        let back = load_tool_strip_states(&dir, "P");
        assert!(!back.is_item_visible("toolStripEdit", "editToolStripButton_Cut"));
        assert!(!back.is_item_visible("toolStripColorLibrary", "R: 1 G: 2 B: 3"));
        assert!(back.is_item_visible("toolStripEdit", "editToolStripButton_Copy"));

        // The standard file gone: the per-profile one isn't read.
        std::fs::remove_file(dir.join("ToolStripState.xml")).unwrap();
        assert_eq!(load_tool_strip_states(&dir, "P"), ToolStripStates::default());
    }

    #[test]
    fn a_later_profile_creates_its_entry_on_demand() {
        let mut st = ToolStripStates::default();
        assert!(st.is_item_visible("toolStripCurveLibrary", "Ramp"));
        st.set_item_visible("toolStripCurveLibrary", "Ramp", false);
        assert!(!st.is_item_visible("toolStripCurveLibrary", "Ramp"));
        st.reset_strip("toolStripCurveLibrary");
        assert!(st.is_item_visible("toolStripCurveLibrary", "Ramp"));
    }

    #[test]
    fn reads_a_real_shape_data_contract_file() {
        let text = r#"<TimedSequenceEditorForm.ToolStripStates xmlns:i="http://www.w3.org/2001/XMLSchema-instance" xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Editor.TimedSequenceEditor"><States xmlns:a="http://schemas.microsoft.com/2003/10/Serialization/Arrays"><a:KeyValueOfstringArrayOfKeyValueOfstringbooleanty7Ep6D1><a:Key>toolStripEdit</a:Key><a:Value><a:KeyValueOfstringboolean><a:Key>editToolStripButton_Cut</a:Key><a:Value>false</a:Value></a:KeyValueOfstringboolean></a:Value></a:KeyValueOfstringArrayOfKeyValueOfstringbooleanty7Ep6D1></States></TimedSequenceEditorForm.ToolStripStates>"#;
        let root = XElement::parse(text).unwrap();
        let st = ToolStripStates::from_xml(&root);
        assert!(!st.is_item_visible("toolStripEdit", "editToolStripButton_Cut"));
    }
}
