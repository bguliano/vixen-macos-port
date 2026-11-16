//! The Effect Editor panel's bindings. The shared values and controls' side is `kit.rs`; this is what needs a sequence.

use crate::kit::{CurveValue, GradientValue, NumberLimits};
use crate::sequence::SequencerSession;
use parking_lot::Mutex;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use vixen_core::{Color, ColorGradient, Curve};
use vixen_session::effect_editor::panel::FontDesc;
use vixen_session::app::AppCore;
use vixen_session::effect_editor::drag::DragPayload;
use vixen_session::effect_editor::preview::EffectPreview;
use vixen_session::effect_editor::panel::{
    self, CommitOptions, CommitOutcome, ListStyle, PanelEdits, RecordItem, RowControl, RowKey, RowModel, ShownValue,
};
use vixen_session::sequence::{EffectId, RenderCounters, SequenceSession};

/// Identifies a row across commits. A row of a list has `index`, and `sub_param` for a member of that row.
#[derive(uniffi::Record, Clone, PartialEq, Eq, Hash)]
pub struct EeRowKey {
    pub param: String,
    pub index: Option<u32>,
    pub sub_param: Option<String>,
}

impl From<&RowKey> for EeRowKey {
    fn from(k: &RowKey) -> EeRowKey {
        EeRowKey { param: k.param.clone(), index: k.index, sub_param: k.sub_param.clone() }
    }
}

impl From<&EeRowKey> for RowKey {
    fn from(k: &EeRowKey) -> RowKey {
        RowKey { param: k.param.clone(), index: k.index, sub_param: k.sub_param.clone() }
    }
}

#[derive(uniffi::Record, Clone, PartialEq)]
pub struct EePair {
    pub gradient: GradientValue,
    pub curve: CurveValue,
}

/// F52's list look.
#[derive(uniffi::Record, Clone, PartialEq)]
pub struct EeListStyle {
    pub has_add_remove: bool,
    pub buttons_above: bool,
    pub inline: bool,
    pub min_items: u32,
}

impl From<&ListStyle> for EeListStyle {
    fn from(s: &ListStyle) -> EeListStyle {
        EeListStyle { has_add_remove: s.can_add_remove, buttons_above: s.buttons_above, inline: s.inline, min_items: s.min_items }
    }
}

#[derive(uniffi::Record, Clone, PartialEq)]
pub struct EeRecordItem {
    pub header: String,
    /// The rules around the header, in the item's timeline colour: fixture functions.
    pub rule_argb: Option<u32>,
    pub rows: Vec<EeRow>,
}

/// What a row edits.
#[derive(uniffi::Enum, Clone, PartialEq)]
pub enum EeRowControl {
    Check { value: bool },
    /// `selected` is `nil` for a blank row.
    Choice { labels: Vec<String>, selected: Option<u32> },
    Number { text: String, limits: NumberLimits },
    Slider { value: f64, min: f64, max: f64, tick: f64, snap: bool, tooltip: String },
    Text { value: String },
    Label { value: String },
    /// `title` is `nil` for the open panel's default; `extensions` empty means any file.
    Path { value: String, title_is_row_name: bool, filter_name: String, extensions: Vec<String> },
    Font { text: String, name: String, size: f32, bold: bool, italic: bool },
    Color { argb: u32, text: String, valid: Vec<u32> },
    Curve { value: CurveValue },
    Gradient { value: GradientValue, discrete: bool, valid: Vec<u32> },
    GradientList { items: Vec<GradientValue>, discrete: bool, valid: Vec<u32>, min_items: u32 },
    ColorList { items: Vec<u32>, valid: Vec<u32>, min_items: u32 },
    GradientLevels { items: Vec<EePair>, discrete: bool, valid: Vec<u32>, min_items: u32 },
    StringList { items: Vec<String>, min_items: u32 },
    RecordList { style: EeListStyle, items: Vec<EeRecordItem> },
    Polygons,
}

#[derive(uniffi::Record, Clone, PartialEq)]
pub struct EeRow {
    pub key: EeRowKey,
    pub label: String,
    pub tooltip: String,
    pub read_only: bool,
    pub control: EeRowControl,
}

#[derive(uniffi::Record, Clone, PartialEq)]
pub struct EeCategory {
    pub name: String,
    /// F13: starts folded, the first time a selection shows it.
    pub starts_collapsed: bool,
    pub rows: Vec<EeRow>,
}

/// The whole panel for a selection.
#[derive(uniffi::Record, Clone, PartialEq)]
pub struct EePanel {
    pub effect_name: String,
    pub information: String,
    pub information_link: String,
    pub effect_count: u32,
    /// F8: exactly one effect selected.
    pub can_save_default: bool,
    /// F9: that, and its type has a saved default.
    pub has_stored_default: bool,
    pub categories: Vec<EeCategory>,
}

fn argbs(colors: &[Color]) -> Vec<u32> {
    colors.iter().map(|c| c.to_argb_u32()).collect()
}

/// F34's filters.
fn path_filter(kind: &str) -> (bool, String, Vec<String>) {
    let list = |v: &[&str]| v.iter().map(|e| e.to_string()).collect();
    match kind {
        "Image" => (true, "Image Files (*.jpg, *.jpeg, *.png, *.gif, *.bmp)".to_string(), list(&["jpg", "jpeg", "png", "gif", "bmp"])),
        "Video" => (true, "Video Files (*.mp4, *.avi, *.mov, *.MTS)".to_string(), list(&["mp4", "avi", "mov", "MTS"])),
        "Svg" => (true, "Image Files (*.svg)".to_string(), list(&["svg"])),
        "Gled" => (true, "Glediator Files (*.gled)".to_string(), list(&["gled"])),
        _ => (false, String::new(), Vec::new()),
    }
}

fn control_of(c: &RowControl, valid: &[Color]) -> EeRowControl {
    match c {
        RowControl::Check { value } => EeRowControl::Check { value: *value },
        RowControl::Choice { items, selected } => {
            EeRowControl::Choice { labels: items.iter().map(|i| i.label.clone()).collect(), selected: *selected }
        }
        RowControl::Int { text, field } => EeRowControl::Number { text: text.clone(), limits: NumberLimits::int(*field) },
        RowControl::Double { text, field } => EeRowControl::Number { text: text.clone(), limits: NumberLimits::double(*field) },
        RowControl::Slider { value, min, max, tick, snap, tooltip } => {
            EeRowControl::Slider { value: *value, min: *min, max: *max, tick: *tick, snap: *snap, tooltip: tooltip.clone() }
        }
        RowControl::Text { value } => EeRowControl::Text { value: value.clone() },
        RowControl::Label { value } => EeRowControl::Label { value: value.clone() },
        RowControl::Path { value, kind } => {
            let (title_is_row_name, filter_name, extensions) = path_filter(kind);
            EeRowControl::Path { value: value.clone(), title_is_row_name, filter_name, extensions }
        }
        RowControl::Font { text, font } => {
            EeRowControl::Font { text: text.clone(), name: font.name.clone(), size: font.size, bold: font.bold, italic: font.italic }
        }
        RowControl::Color { value, text, valid } => EeRowControl::Color { argb: value.to_argb_u32(), text: text.clone(), valid: argbs(valid) },
        RowControl::Curve { value } => EeRowControl::Curve { value: value.into() },
        RowControl::Gradient { value, discrete } => {
            EeRowControl::Gradient { value: value.into(), discrete: *discrete, valid: if *discrete { argbs(valid) } else { Vec::new() } }
        }
        RowControl::GradientList { items, discrete, min_items } => EeRowControl::GradientList {
            items: items.iter().map(GradientValue::from).collect(),
            discrete: *discrete,
            valid: argbs(valid),
            min_items: *min_items,
        },
        RowControl::ColorList { items, valid, min_items } => EeRowControl::ColorList { items: argbs(items), valid: argbs(valid), min_items: *min_items },
        RowControl::GradientLevels { items, discrete, min_items } => EeRowControl::GradientLevels {
            items: items.iter().map(|p| EePair { gradient: (&p.color_gradient).into(), curve: (&p.curve).into() }).collect(),
            discrete: *discrete,
            valid: argbs(valid),
            min_items: *min_items,
        },
        RowControl::StringList { items, min_items } => EeRowControl::StringList { items: items.clone(), min_items: *min_items },
        RowControl::RecordList { style, items } => {
            EeRowControl::RecordList { style: style.into(), items: items.iter().map(|i| record_item_of(i, valid)).collect() }
        }
        RowControl::Polygons => EeRowControl::Polygons,
    }
}

fn record_item_of(item: &RecordItem, valid: &[Color]) -> EeRecordItem {
    EeRecordItem {
        header: item.header.clone(),
        rule_argb: item.header_color.map(|c| c.to_argb_u32()),
        rows: item.rows.iter().map(|r| row_of(r, valid)).collect(),
    }
}

fn row_of(r: &RowModel, valid: &[Color]) -> EeRow {
    EeRow { key: (&r.key).into(), label: r.label.clone(), tooltip: r.tooltip.clone(), read_only: r.read_only, control: control_of(&r.control, valid) }
}

/// What a row hands back.
#[derive(uniffi::Enum, Clone)]
pub enum EeValue {
    Bool { value: bool },
    Choice { index: u32 },
    Text { value: String },
    Slider { value: f64 },
    Color { argb: u32 },
    Curve { value: CurveValue },
    Gradient { value: GradientValue },
    Font { name: String, size: f32, bold: bool, italic: bool },
    Path { value: String },
}

impl From<&EeValue> for ShownValue {
    fn from(v: &EeValue) -> ShownValue {
        match v {
            EeValue::Bool { value } => ShownValue::Bool(*value),
            EeValue::Choice { index } => ShownValue::Choice(*index),
            EeValue::Text { value } => ShownValue::Text(value.clone()),
            EeValue::Slider { value } => ShownValue::Slider(*value),
            EeValue::Color { argb } => ShownValue::Color(Color::from_argb_u32(*argb)),
            EeValue::Curve { value } => ShownValue::Curve(Curve::from(value)),
            EeValue::Gradient { value } => ShownValue::Gradient(ColorGradient::from(value)),
            EeValue::Font { name, size, bold, italic } => {
                ShownValue::Font(FontDesc { name: name.clone(), size: *size, bold: *bold, italic: *italic })
            }
            EeValue::Path { value } => ShownValue::Path(value.clone()),
        }
    }
}

#[derive(uniffi::Record, Clone)]
pub struct EeCommitResult {
    pub changed: bool,
    /// A new step went on the undo stack; false when this joined its gesture's step.
    pub new_undo_step: bool,
    /// What the Edit menu names.
    pub undo_label: String,
}

impl From<CommitOutcome> for EeCommitResult {
    fn from(o: CommitOutcome) -> EeCommitResult {
        match o {
            CommitOutcome::NoChange => EeCommitResult { changed: false, new_undo_step: false, undo_label: String::new() },
            CommitOutcome::Committed { undo_label, new_step } => EeCommitResult { changed: true, new_undo_step: new_step, undo_label },
        }
    }
}

/// One effect, for the test window's list.
#[derive(uniffi::Record, Clone)]
pub struct EeEffectItem {
    pub id: u64,
    pub name: String,
    pub target: String,
    pub start_ms: f64,
    pub span_ms: f64,
}

/// The panel's way in to a sequence.
#[derive(uniffi::Object)]
pub struct EffectEditorApi {
    session: Arc<Mutex<SequenceSession>>,
    core: Arc<AppCore>,
    edits: Mutex<PanelEdits>,
    preview: Mutex<PreviewState>,
    counters: Arc<RenderCounters>,
}

/// F5 and F6.
#[derive(Default)]
struct PreviewState {
    on: bool,
    effects: Vec<EffectId>,
    /// Real playback or Export is running: the preview stops and comes back afterwards.
    suspended: bool,
    running: Option<EffectPreview>,
}

#[uniffi::export]
impl SequencerSession {
    /// One per panel.
    pub fn effect_editor(&self) -> Arc<EffectEditorApi> {
        let edits = PanelEdits::new(Some(self.core.paths.module_data_dir()));
        let counters = Arc::clone(&self.session.lock().counters);
        Arc::new(EffectEditorApi {
            counters,
            session: Arc::clone(&self.session),
            core: Arc::clone(&self.core),
            edits: Mutex::new(edits),
            preview: Mutex::new(PreviewState::default()),
        })
    }
}

fn ids(effects: &[u64]) -> Vec<EffectId> {
    effects.iter().copied().map(EffectId).collect()
}

#[uniffi::export]
impl EffectEditorApi {
    /// Every row for a selection. Call once per selection and once after a commit: Swift keeps the views of rows whose keys stay.
    pub fn panel(&self, effects: Vec<u64>) -> EePanel {
        let session = self.session.lock();
        let m = panel::panel_model(&session, &ids(&effects), panel::DEFAULT_MULTI_EDIT_LIMIT);
        let valid = m.valid_colors.clone();
        let only = if effects.len() == 1 { session.effect(EffectId(effects[0])).and_then(|e| e.descriptor) } else { None };
        let has_stored_default = only.is_some_and(|d| self.core.effect_defaults().lock().has(&d.type_id));
        EePanel {
            can_save_default: only.is_some(),
            has_stored_default,
            effect_name: m.effect_name,
            information: m.information,
            information_link: m.information_link,
            effect_count: m.effect_count,
            categories: m
                .categories
                .iter()
                .map(|c| EeCategory {
                    name: c.name.clone(),
                    starts_collapsed: c.starts_collapsed,
                    rows: c
                        .rows
                        .iter()
                        .map(|r| {
                            let mut row = row_of(r, &valid);
                            // F105: `PhonemeMappingConverter`'s list comes from outside the effect: the LipSync map library.
                            if r.key.param == "PhonemeMapping" {
                                let stored = session.effect(EffectId(effects[0])).map_or("", |e| e.node.params.string("PhonemeMapping")).to_string();
                                let labels = self.phoneme_maps();
                                row.control = EeRowControl::Choice { selected: labels.iter().position(|n| *n == stored).map(|i| i as u32), labels };
                            }
                            row
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// One commit. `gesture` joins commits into one undo step: a slider's or a strip's drag.
    pub fn commit(&self, effects: Vec<u64>, key: EeRowKey, value: EeValue, gesture: Option<u64>) -> EeCommitResult {
        // F105: the choice is the map's name, and each effect's setter takes the text.
        let value = match value {
            EeValue::Choice { index } if key.param == "PhonemeMapping" => match self.phoneme_maps().into_iter().nth(index as usize) {
                Some(name) => EeValue::Text { value: name },
                None => return CommitOutcome::NoChange.into(),
            },
            other => other,
        };
        let mut session = self.session.lock();
        let mut edits = self.edits.lock();
        panel::commit(&mut session, &mut edits, &ids(&effects), &(&key).into(), (&value).into(), CommitOptions { gesture }).into()
    }

    /// F56.
    pub fn list_add(&self, effects: Vec<u64>, param: String) -> EeCommitResult {
        let mut session = self.session.lock();
        let mut edits = self.edits.lock();
        panel::list_add(&mut session, &mut edits, &ids(&effects), &param).into()
    }

    /// F57.
    pub fn list_remove(&self, effects: Vec<u64>, param: String, index: u32) -> EeCommitResult {
        let mut session = self.session.lock();
        let mut edits = self.edits.lock();
        panel::list_remove(&mut session, &mut edits, &ids(&effects), &param, index).into()
    }

    pub fn list_can_remove(&self, effects: Vec<u64>, param: String, selected: Option<u32>) -> bool {
        panel::can_remove(&self.session.lock(), &ids(&effects), &param, selected)
    }

    /// F59.
    pub fn list_move(&self, effects: Vec<u64>, param: String, from: u32, to: u32) -> EeCommitResult {
        let mut session = self.session.lock();
        let mut edits = self.edits.lock();
        panel::list_move(&mut session, &mut edits, &ids(&effects), &param, from, to).into()
    }

    pub fn list_can_move(&self, effects: Vec<u64>, param: String, selected: Option<u32>, delta: i32) -> bool {
        panel::can_move(&self.session.lock(), &ids(&effects), &param, selected, delta)
    }

    /// F68's table and discrete rule, for a value drag's text.
    pub fn can_drop(&self, effects: Vec<u64>, key: EeRowKey, drag_text: String) -> bool {
        let Some(payload) = DragPayload::decode(&drag_text) else { return false };
        panel::can_drop(&self.session.lock(), &ids(&effects), &(&key).into(), &payload)
    }

    pub fn drop_value(&self, effects: Vec<u64>, key: EeRowKey, drag_text: String) -> EeCommitResult {
        let Some(payload) = DragPayload::decode(&drag_text) else { return CommitOutcome::NoChange.into() };
        let mut session = self.session.lock();
        let mut edits = self.edits.lock();
        panel::drop_value(&mut session, &mut edits, &ids(&effects), &(&key).into(), &payload).into()
    }

    /// Bumps with every edit to the document, from whichever window, so the panel knows to read its values again.
    pub fn edit_revision(&self) -> u64 {
        self.counters.edits.load(Ordering::Relaxed)
    }

    /// Effects in document order, for the test window.
    pub fn effect_list(&self) -> Vec<EeEffectItem> {
        let session = self.session.lock();
        session
            .effects()
            .map(|e| EeEffectItem {
                id: e.id.0,
                name: e.descriptor.map_or_else(|| "Unknown".to_string(), |d| d.name.to_string()),
                target: e.node.targets.first().map_or_else(String::new, |t| t.1.clone()),
                start_ms: e.node.start.total_ms(),
                span_ms: e.node.time_span.total_ms(),
            })
            .collect()
    }

    /// For checking that an edit reached the render.
    pub fn render_summary(&self, effect: u64) -> String {
        panel::render_summary(&self.session.lock(), EffectId(effect))
    }

    /// F8: the effect's values become its type's saved default. No prompt, and the effect doesn't change.
    pub fn save_as_default(&self, effect: u64) -> Result<(), crate::core::FfiError> {
        let session = self.session.lock();
        let Some(entry) = session.effect(EffectId(effect)) else { return Ok(()) };
        let Some(descriptor) = entry.descriptor else { return Ok(()) };
        self.core.effect_defaults().lock().save(descriptor, &entry.node.params)?;
        Ok(())
    }

    /// F9: forgets the saved default of the effect's type. Changes no effect, the selected one included.
    pub fn reset_default(&self, effect: u64) -> Result<(), crate::core::FfiError> {
        let session = self.session.lock();
        let Some(descriptor) = session.effect(EffectId(effect)).and_then(|e| e.descriptor) else { return Ok(()) };
        self.core.effect_defaults().lock().clear(&descriptor.type_id)?;
        Ok(())
    }

    /// F5: the switch. On with nothing selected only remembers, and plays from the next selection.
    pub fn preview_set_on(&self, on: bool) {
        let mut state = self.preview.lock();
        state.on = on;
        self.restart_preview(&mut state);
    }

    /// F6: each selection stops the loop and plays again if on, an identical selection too.
    pub fn preview_set_selection(&self, effects: Vec<u64>) {
        let mut state = self.preview.lock();
        state.effects = ids(&effects);
        self.restart_preview(&mut state);
    }

    /// Real playback or Export is starting: F6's Rules. The sequencer calls this before it plays.
    pub fn preview_suspend(&self) {
        let mut state = self.preview.lock();
        state.suspended = true;
        state.running = None;
    }

    /// `ResumePreview`, after playback or Export.
    pub fn preview_resume(&self) {
        let mut state = self.preview.lock();
        state.suspended = false;
        self.restart_preview(&mut state);
    }

    /// A few times a second from the panel: real playback or Export can take the engine from a running loop at any moment, and the loop comes back when the engine is free again (F6's Rules, F267's `ResumePreview`).
    pub fn preview_poll(&self) {
        let mut state = self.preview.lock();
        if !state.on || state.suspended || state.effects.is_empty() {
            return;
        }
        if state.running.as_ref().is_some_and(EffectPreview::is_playing) {
            return;
        }
        self.restart_preview(&mut state);
    }

    /// Whether a loop is running, for tests.
    pub fn preview_is_running(&self) -> bool {
        self.preview.lock().running.as_ref().is_some_and(EffectPreview::is_playing)
    }

    /// The profile's data folder, which relative paths in a row are under.
    pub fn data_root(&self) -> String {
        self.core.paths.root().to_string_lossy().into_owned()
    }
}

impl EffectEditorApi {
    fn phoneme_maps(&self) -> Vec<String> {
        vixen_io::lip_sync_map_names(&self.core.system.read().module_store)
    }

    fn restart_preview(&self, state: &mut PreviewState) {
        state.running = None;
        if state.on && !state.suspended && !state.effects.is_empty() {
            state.running = EffectPreview::start(Arc::clone(&self.core.playback), Arc::clone(&self.session), state.effects.clone());
        }
    }
}

#[uniffi::export]
impl SequencerSession {
    /// The application object this session belongs to, for what the panel's dialogs read and save: the libraries.
    pub fn application_core(&self) -> Arc<crate::core::VixenCore> {
        Arc::new(crate::core::VixenCore { inner: Arc::clone(&self.core) })
    }
}

/// For the test window: the first effect of this type, or a new one on the first element if the sequence has none.
#[uniffi::export]
impl EffectEditorApi {
    pub fn test_effect_of_type(&self, name: String) -> Option<u64> {
        let mut session = self.session.lock();
        if let Some(e) = session.effects().find(|e| e.descriptor.is_some_and(|d| d.name == name)) {
            return Some(e.id.0);
        }
        let descriptor = vixen_session::effect_editor::panel::descriptor_named(&name)?;
        let tree = session.tree().clone();
        let (target, target_name) = tree.all_nodes().next().map(|(_, n)| (n.id, n.name.clone()))?;
        let span = vixen_core::TimeSpan::from_ms(2000.0);
        session.add_new_effect(descriptor.type_id, target, &target_name, vixen_core::TimeSpan::ZERO, span).map(|id| id.0)
    }

    /// The second effect of the type of `effect`, for trying a mixed selection.
    pub fn test_next_of_same_type(&self, effect: u64) -> Option<u64> {
        let session = self.session.lock();
        let type_id = session.effect(EffectId(effect))?.node.type_id;
        let next = session.effects().find(|e| e.node.type_id == type_id && e.id.0 != effect).map(|e| e.id.0);
        next
    }

}

/// Every effect module's name, for the test window's menu.
#[uniffi::export]
pub fn all_effect_names() -> Vec<String> {
    vixen_session::effect_editor::panel::effect_names()
}
