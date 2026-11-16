//! The panel's rows for a selection, and what a change does: F2 to F4, F13, F14, F21 to F37, F52 to F64. Table-driven and cheap, so one call per selection and one per commit.

use super::drag::DragPayload;
use super::gradient_strip::gradient_same_value;
use super::numeric::{double_text, percent_from_slider, slider_tooltip, to_int32, DoubleField, IntField, SliderKind};
use super::values::{color_text, font_display_text};
use crate::sequence::{EffectEntry, EffectId, SequenceSession};
use crate::undo::UndoAction;
use std::path::Path;
use uuid::Uuid;
use vixen_core::{Color, ColorGradient, Curve, GradientLevelPair};
pub use vixen_effects::framework::FontDesc;
use vixen_effects::framework::{
    Editor, EffectDescriptor, ParamDesc, ParamKind, ParamValue, Params, RecordDesc, SelectionValue, TargetInfo, Visibility,
};

/// `MultiEditLimit`'s default. Exactly this many is allowed, though the message says `<` (KEEP, F4).
pub const DEFAULT_MULTI_EDIT_LIMIT: usize = 75;
const NOTHING_SELECTED: &str = "Select an Effect to edit.";
const DEFAULT_INFO_LINK: &str = "https://www.vixenlights.com/docs/usage/sequencer/sequencer-basics/effect-editor/";

/// Identifies a row. A row of a list is `(param, index, sub_param)`; a gradient and level pair's two strips are `sub_param` `Curve` and `ColorGradient`. Stable across commits, so Swift keeps view identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RowKey {
    pub param: String,
    pub index: Option<u32>,
    pub sub_param: Option<String>,
}

impl RowKey {
    pub fn top(param: &str) -> RowKey {
        RowKey { param: param.to_string(), index: None, sub_param: None }
    }

    pub fn item(param: &str, index: u32) -> RowKey {
        RowKey { param: param.to_string(), index: Some(index), sub_param: None }
    }

    pub fn sub(param: &str, index: u32, sub_param: &str) -> RowKey {
        RowKey { param: param.to_string(), index: Some(index), sub_param: Some(sub_param.to_string()) }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceItem {
    pub label: String,
    pub value: ParamValue,
}

/// How a `RecordList` looks: F52's table, which is in the XAML templates upstream rather than on the data.
#[derive(Clone, Debug, PartialEq)]
pub struct ListStyle {
    /// `Emitter` gives `Emitter 1`, `Emitter 2`... `None` means no header, or the item's own text when `item_text` names a member.
    pub numbered: Option<&'static str>,
    /// The member whose text is the header, between two rules in `rule_color`'s colour: fixture functions.
    pub item_text: Option<&'static str>,
    pub rule_color: Option<&'static str>,
    pub can_add_remove: bool,
    pub buttons_above: bool,
    /// One line with no labels: a custom State item.
    pub inline: bool,
    pub min_items: u32,
}

fn list_style(record: &RecordDesc, p: &Params) -> ListStyle {
    let numbered = |word| ListStyle {
        numbered: Some(word),
        item_text: None,
        rule_color: None,
        can_add_remove: true,
        buttons_above: false,
        inline: false,
        min_items: 1,
    };
    match record.xml_type {
        "WaveformData" => numbered("Wave"),
        "MorphPolygonData" => numbered("Polygon"),
        "WhirlData" => ListStyle { can_add_remove: false, ..numbered("Whirl") },
        "FixtureFunctionData" => ListStyle {
            numbered: None,
            item_text: Some("FunctionName"),
            rule_color: Some("TimelineColor"),
            can_add_remove: true,
            buttons_above: true,
            inline: false,
            // `FixtureFunctionExpandoCollection.GetMinimumItemCount`.
            min_items: 0,
        },
        "CustomStateItemData" => ListStyle {
            numbered: None,
            item_text: None,
            rule_color: None,
            can_add_remove: true,
            buttons_above: false,
            inline: true,
            // `CustomStateItemCollection.GetMinimumItemCount`.
            min_items: u32::from(p.index_of("RenderSource").is_some() && p.enum_name("RenderSource") == "Custom"),
        },
        _ => numbered("Emitter"),
    }
}

/// One item of a record list.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordItem {
    pub header: String,
    pub header_color: Option<Color>,
    pub rows: Vec<RowModel>,
}

/// What a row edits, in the terms its editor needs.
#[derive(Clone, Debug, PartialEq)]
pub enum RowControl {
    Check { value: bool },
    /// `selected` is `None` for a blank row: nothing in the list matches, or F60's mixed enum.
    Choice { items: Vec<ChoiceItem>, selected: Option<u32> },
    Int { text: String, field: IntField },
    Double { text: String, field: DoubleField },
    /// `value` is what's on the slider, in shown units.
    Slider { value: f64, min: f64, max: f64, tick: f64, snap: bool, tooltip: String },
    Text { value: String },
    Label { value: String },
    /// `kind` is F34's: Image, Video, Svg, Gled, File.
    Path { value: String, kind: &'static str },
    Font { text: String, font: FontDesc },
    Color { value: Color, text: String, valid: Vec<Color> },
    Curve { value: Curve },
    Gradient { value: ColorGradient, discrete: bool },
    GradientList { items: Vec<ColorGradient>, discrete: bool, min_items: u32 },
    ColorList { items: Vec<Color>, valid: Vec<Color>, min_items: u32 },
    GradientLevels { items: Vec<GradientLevelPair>, discrete: bool, min_items: u32 },
    StringList { items: Vec<String>, min_items: u32 },
    RecordList { style: ListStyle, items: Vec<RecordItem> },
    /// F37's button. The polygon editor is another worker's.
    Polygons,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RowModel {
    pub key: RowKey,
    pub label: String,
    /// The description, or the display name when it has none (F17).
    pub tooltip: String,
    pub read_only: bool,
    pub control: RowControl,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryModel {
    pub name: String,
    pub order: i32,
    /// F13: Video's Advanced Settings starts folded.
    pub starts_collapsed: bool,
    pub rows: Vec<RowModel>,
}

/// The whole panel for a selection.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelModel {
    /// Empty collapses the header bar (F3, F7).
    pub effect_name: String,
    pub information: String,
    pub information_link: String,
    pub effect_count: u32,
    pub categories: Vec<CategoryModel>,
    pub discrete: bool,
    pub valid_colors: Vec<Color>,
}

/// One selected effect, with what its rows read.
struct Selected<'a> {
    entry: &'a EffectEntry,
    descriptor: &'static EffectDescriptor,
    info: TargetInfo<'a>,
}

impl Selected<'_> {
    fn params(&self) -> &Params {
        &self.entry.node.params
    }
}

fn selected<'a>(s: &'a SequenceSession, effects: &[EffectId]) -> Vec<Selected<'a>> {
    effects
        .iter()
        .filter_map(|id| {
            let entry = s.effect(*id)?;
            let descriptor = entry.descriptor?;
            let info = s.target_info(*id)?;
            Some(Selected { entry, descriptor, info })
        })
        .collect()
}

/// F102: the union of each effect's valid colours. Empty means full colour.
fn union_of_valid_colors(chosen: &[Selected]) -> Vec<Color> {
    let mut out = Vec::new();
    for e in chosen {
        for c in e.info.valid_colors() {
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}

/// A property's descriptor equality is name and type (F60).
fn same_kind(a: ParamKind, b: ParamKind) -> bool {
    match (a, b) {
        (ParamKind::Enum(x), ParamKind::Enum(y)) => {
            x.name == y.name && x.members.len() == y.members.len() && x.members.iter().zip(y.members).all(|(m, n)| m.name == n.name)
        }
        (ParamKind::Record(x), ParamKind::Record(y)) | (ParamKind::RecordList(x), ParamKind::RecordList(y)) => x.xml_type == y.xml_type,
        (ParamKind::Other(x), ParamKind::Other(y)) => x == y,
        _ => std::mem::discriminant(&a) == std::mem::discriminant(&b),
    }
}

/// F61's reading of `[PropertyEditor]` and `[NumberRange]`: an editor that differs falls to the type's default, and a range that differs falls to the editor's fallbacks.
fn merged_editor(a: Editor, b: Editor) -> Editor {
    if a == b {
        return a;
    }
    match (a, b) {
        (Editor::Slider { .. }, Editor::Slider { .. }) => Editor::Slider { min: 0.0, max: 100.0, step: 1.0 },
        (Editor::SliderDouble { .. }, Editor::SliderDouble { .. }) => Editor::SliderDouble { min: 0.0, max: 100.0, step: 1.0 },
        _ => Editor::Default,
    }
}

/// A row of the selection: the first effect's property, with the metadata F61 merges.
struct MergedParam {
    desc: &'static ParamDesc,
    category: &'static str,
    category_order: i32,
    order: i32,
    description: Option<&'static str>,
    editor: Editor,
}

fn category_name(desc: &ParamDesc) -> &'static str {
    if desc.category.is_empty() {
        "Misc"
    } else {
        desc.category
    }
}

/// F60 and F61. One effect's rows are simply its visible ones.
fn merged_params(chosen: &[Selected]) -> Vec<MergedParam> {
    let Some(first) = chosen.first() else { return Vec::new() };
    let visible_first = first.descriptor.visible_params_for(first.params(), &first.info);
    if chosen.len() == 1 {
        return visible_first
            .into_iter()
            .map(|i| {
                let desc = &first.descriptor.params[i];
                MergedParam {
                    desc,
                    category: category_name(desc),
                    category_order: desc.category_order,
                    order: desc.order,
                    description: Some(desc.description),
                    editor: desc.editor,
                }
            })
            .collect();
    }
    let others: Vec<(&Selected, Vec<usize>)> =
        chosen[1..].iter().map(|e| (e, e.descriptor.visible_params_for(e.params(), &e.info))).collect();
    let mut merged = Vec::new();
    for i in visible_first {
        let desc = &first.descriptor.params[i];
        let mut m = MergedParam {
            desc,
            category: category_name(desc),
            category_order: desc.category_order,
            order: desc.order,
            description: Some(desc.description),
            editor: desc.editor,
        };
        let mut common = true;
        for (e, visible) in &others {
            let Some(j) = e.descriptor.params.iter().position(|d| d.name == desc.name && same_kind(d.kind, desc.kind)) else {
                common = false;
                break;
            };
            // Browsable if all effects' are.
            if !visible.contains(&j) {
                common = false;
                break;
            }
            let other = &e.descriptor.params[j];
            if category_name(other) != m.category {
                m.category = "Misc";
                m.category_order = -1;
            }
            if other.order != m.order {
                m.order = 0;
            }
            if m.description != Some(other.description) {
                m.description = None;
            }
            m.editor = merged_editor(m.editor, other.editor);
        }
        if common {
            merged.push(m);
        }
    }
    merged.sort_by_cached_key(|m| (m.category_order, m.category.to_lowercase(), m.order, m.desc.label.to_lowercase(), m.desc.name));
    merged
}

fn tooltip_of(description: Option<&str>, label: &str) -> String {
    match description {
        Some(d) if !d.is_empty() => d.to_string(),
        _ => label.to_string(),
    }
}

/// The panel for a selection: F2 to F4, F13, F14, F21, F60, F61. Empty and too-many are states of it.
pub fn panel_model(s: &SequenceSession, effects: &[EffectId], limit: usize) -> PanelModel {
    let chosen = selected(s, effects);
    if chosen.is_empty() || chosen.len() > limit {
        let information = if chosen.is_empty() { NOTHING_SELECTED.to_string() } else { format!("Multi-Edit is limited to < {limit} Effects") };
        return PanelModel {
            effect_name: String::new(),
            information,
            information_link: DEFAULT_INFO_LINK.to_string(),
            effect_count: chosen.len() as u32,
            categories: Vec::new(),
            discrete: false,
            valid_colors: Vec::new(),
        };
    }
    let valid = union_of_valid_colors(&chosen);
    let first = &chosen[0];
    let mut categories: Vec<CategoryModel> = Vec::new();
    for m in merged_params(&chosen) {
        let row = top_row(&m, &chosen, &valid);
        // By name: a category's order is that of the first of its properties met, and a later one with another order joins it (F14).
        match categories.iter_mut().find(|c| c.name == m.category) {
            Some(c) => c.rows.push(row),
            None => categories.push(CategoryModel {
                name: m.category.to_string(),
                order: m.category_order,
                starts_collapsed: first.descriptor.name == "Video" && m.category == "Advanced Settings",
                rows: vec![row],
            }),
        }
    }
    PanelModel {
        effect_name: first.descriptor.name.to_string(),
        information: first.descriptor.information.to_string(),
        information_link: first.descriptor.info_link.to_string(),
        effect_count: chosen.len() as u32,
        categories,
        discrete: !valid.is_empty(),
        valid_colors: valid,
    }
}

/// F28, F60: the depth lists that go blank when mixed are `TargetElementDepthConverter`'s. Fire's and Wipe's own converters don't derive from it, and First's converter decides.
fn blank_when_mixed(first: &EffectDescriptor, desc: &ParamDesc) -> bool {
    match desc.kind {
        ParamKind::Enum(_) => true,
        ParamKind::Int => matches!(desc.editor, Editor::Selection) && desc.name == "DepthOfEffect" && !matches!(first.name, "Fire" | "Wipe"),
        _ => false,
    }
}

fn values_differ(chosen: &[Selected], name: &str) -> bool {
    let first = chosen[0].params().get(name);
    chosen[1..].iter().any(|e| e.params().get(name) != first)
}

/// What a row's list offers. F29: `[Browsable(false)]` enum members load and save but aren't offered, and today that's `TilePictures.None`.
fn selection_items(descriptor: &EffectDescriptor, desc: &ParamDesc, row: Option<&Params>, p: &Params, info: &TargetInfo) -> Vec<ChoiceItem> {
    let mut items: Vec<ChoiceItem> = descriptor
        .selection_values(desc.name, row, p, info)
        .into_iter()
        .map(|SelectionValue { value, label }| ChoiceItem { label, value })
        .collect();
    if let ParamKind::Enum(e) = desc.kind {
        if e.name == "TilePictures" {
            items.retain(|i| i.value != ParamValue::Enum("None".to_string()));
        }
    }
    items
}

/// F105: the list for a selection. First's converter decides, and it's handed the whole selection: depth lists use the smallest depth among the effects with a target, the effect-list converters
/// the union in first-seen order over the effects of First's type, boolean words fall back to True and False where they differ, and the rest are First's.
fn merged_choices(chosen: &[Selected], m: &MergedParam) -> Vec<ChoiceItem> {
    let first = &chosen[0];
    let mut items = selection_items(first.descriptor, m.desc, None, first.params(), &first.info);
    if chosen.len() < 2 {
        return items;
    }
    match m.desc.kind {
        ParamKind::Bool => {
            let differ = chosen[1..].iter().any(|e| {
                e.descriptor.params.iter().find(|d| d.name == m.desc.name).is_some_and(|d| d.bool_labels != m.desc.bool_labels)
            });
            if differ {
                if let [yes, no] = items.as_mut_slice() {
                    yes.label = "True".to_string();
                    no.label = "False".to_string();
                }
            }
        }
        ParamKind::Int => {
            let depth = chosen.iter().filter_map(|e| e.info.targets.first().map(|n| e.info.tree.max_child_depth(*n))).min().unwrap_or(0);
            // Fire's and Wipe's converters list the depths between the target and its leaves; `TargetElementDepthConverter` lists them all.
            let (from, to) = if matches!(first.descriptor.name, "Fire" | "Wipe") { (1, depth - 1) } else { (0, depth) };
            items = (from..to).map(|i| ChoiceItem { label: i.to_string(), value: ParamValue::Int(i) }).collect();
        }
        ParamKind::Str => {
            for e in chosen[1..].iter().filter(|e| e.descriptor.type_id == first.descriptor.type_id) {
                if let Some(d) = e.descriptor.params.iter().find(|d| d.name == m.desc.name) {
                    for item in selection_items(e.descriptor, d, None, e.params(), &e.info) {
                        if !items.iter().any(|i| i.label == item.label) {
                            items.push(item);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    items
}

fn top_row(m: &MergedParam, chosen: &[Selected], valid: &[Color]) -> RowModel {
    let first = &chosen[0];
    let value = first.params().get(m.desc.name);
    let is_choice = matches!(m.desc.kind, ParamKind::Enum(_))
        || matches!(m.editor, Editor::Selection)
        || (matches!(m.desc.kind, ParamKind::Bool) && m.desc.bool_labels.is_some());
    let control = if is_choice {
        let items = merged_choices(chosen, m);
        let mut selected = items.iter().position(|i| same_choice(&i.value, value)).map(|i| i as u32);
        if chosen.len() > 1 && blank_when_mixed(first.descriptor, m.desc) && values_differ(chosen, m.desc.name) {
            selected = None;
        }
        RowControl::Choice { items, selected }
    } else {
        row_control(first.descriptor, m.desc, m.editor, value, None, first.params(), &first.info, m.desc.name, valid, true)
    };
    RowModel {
        key: RowKey::top(m.desc.name),
        label: m.desc.label.to_string(),
        tooltip: tooltip_of(m.description, m.desc.label),
        read_only: matches!(m.editor, Editor::Label),
        control,
    }
}

fn choice_control(
    descriptor: &EffectDescriptor,
    desc: &ParamDesc,
    row: Option<&Params>,
    value: &ParamValue,
    p: &Params,
    info: &TargetInfo,
) -> RowControl {
    let items = selection_items(descriptor, desc, row, p, info);
    let selected = items.iter().position(|i| same_choice(&i.value, value)).map(|i| i as u32);
    RowControl::Choice { items, selected }
}

/// The current item is found by its text upstream. Here by the value, which comes to the same thing except that an `Int` list's value and a shown string can differ in type.
fn same_choice(item: &ParamValue, value: &ParamValue) -> bool {
    match (item, value) {
        (ParamValue::Str(a), ParamValue::Enum(b)) | (ParamValue::Enum(a), ParamValue::Str(b)) => a == b,
        (a, b) => a == b,
    }
}

fn int_of(value: &ParamValue) -> i128 {
    match value {
        ParamValue::Int(v) => *v as i128,
        ParamValue::Long(v) => *v as i128,
        ParamValue::Double(v) | ParamValue::Percentage(v) => *v as i128,
        ParamValue::Raw(s) | ParamValue::Str(s) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

fn double_of(value: &ParamValue) -> f64 {
    match value {
        ParamValue::Double(v) | ParamValue::Percentage(v) => *v,
        ParamValue::Int(v) => *v as f64,
        ParamValue::Long(v) => *v as f64,
        _ => 0.0,
    }
}

/// The type's limits, F26. The tables' `Long` is a `uint`, and `Other("ulong")` a `ulong`.
fn int_bounds(kind: ParamKind) -> (i128, i128) {
    match kind {
        ParamKind::Long => (0, u32::MAX as i128),
        ParamKind::Other("ulong") => (0, u64::MAX as i128),
        _ => (i32::MIN as i128, i32::MAX as i128),
    }
}

pub fn int_field(kind: ParamKind, editor: Editor) -> IntField {
    let bounds = int_bounds(kind);
    match editor {
        Editor::Number { min, max, step } => IntField::ranged(min, max, step, bounds),
        _ => IntField::unranged(bounds.1),
    }
}

pub fn double_field(editor: Editor) -> DoubleField {
    match editor {
        Editor::Number { min, max, step } => DoubleField { min, max, step, precision: 0 },
        _ => DoubleField::unranged(),
    }
}

fn slider_control(kind: SliderKind, value: f64, min: f64, max: f64, tick: f64) -> RowControl {
    RowControl::Slider { value, min, max, tick, snap: kind.snaps(), tooltip: slider_tooltip(kind, value) }
}

fn time_text(t: vixen_core::TimeSpan) -> String {
    double_text(t.total_ms())
}

/// F21's search, with the type first: PinWheel's Number on a curve, for one, is harmless because the type's editor wins.
#[allow(clippy::too_many_arguments)]
fn row_control(
    descriptor: &EffectDescriptor,
    desc: &'static ParamDesc,
    editor: Editor,
    value: &ParamValue,
    row: Option<&Params>,
    p: &Params,
    info: &TargetInfo,
    list_param: &str,
    valid: &[Color],
    in_effect: bool,
) -> RowControl {
    if matches!(editor, Editor::Label) {
        return RowControl::Label { value: value_text(value) };
    }
    match desc.kind {
        ParamKind::Bool => {
            if matches!(editor, Editor::Selection) || desc.bool_labels.is_some() {
                choice_control(descriptor, desc, row, value, p, info)
            } else {
                RowControl::Check { value: matches!(value, ParamValue::Bool(true)) }
            }
        }
        ParamKind::Enum(_) => choice_control(descriptor, desc, row, value, p, info),
        ParamKind::Int | ParamKind::Long | ParamKind::Other("ulong") => match editor {
            Editor::Slider { min, max, step } => {
                let shown = desc.display.shown(int_of(value) as i32);
                slider_control(SliderKind::Int, shown as f64, min, max, step)
            }
            Editor::Selection => choice_control(descriptor, desc, row, value, p, info),
            _ => {
                let stored = int_of(value);
                let shown = if matches!(desc.kind, ParamKind::Int) { desc.display.shown(stored as i32) as i128 } else { stored };
                let field = int_field(desc.kind, editor);
                // F24: a stored value outside the range shows clamped. Showing never writes.
                RowControl::Int { text: field.enforce(shown).to_string(), field }
            }
        },
        ParamKind::Double | ParamKind::Percentage => {
            let stored = double_of(value);
            match editor {
                Editor::SliderDouble { min, max, step } | Editor::Slider { min, max, step } => slider_control(SliderKind::Double, stored, min, max, step),
                Editor::Level => slider_control(SliderKind::Level, stored * 100.0, 0.0, 100.0, 1.0),
                Editor::Percent => slider_control(SliderKind::Percent, stored * 100.0, 0.0, 100.0, 1.0),
                Editor::Selection => choice_control(descriptor, desc, row, value, p, info),
                _ => {
                    let field = double_field(editor);
                    RowControl::Double { text: double_text(field.enforce(stored)), field }
                }
            }
        }
        ParamKind::Str => match editor {
            Editor::Path(kind) => RowControl::Path { value: value_text(value), kind },
            Editor::Selection => choice_control(descriptor, desc, row, value, p, info),
            _ => RowControl::Text { value: value_text(value) },
        },
        ParamKind::Guid => match editor {
            Editor::Selection => choice_control(descriptor, desc, row, value, p, info),
            _ => RowControl::Text { value: value_text(value) },
        },
        ParamKind::Time => RowControl::Text { value: value_text(value) },
        ParamKind::Curve => match value {
            ParamValue::Curve(c) => RowControl::Curve { value: c.clone() },
            _ => RowControl::Curve { value: Curve::default() },
        },
        ParamKind::Gradient => match value {
            ParamValue::Gradient(g) => RowControl::Gradient { value: g.clone(), discrete: in_effect && !valid.is_empty() },
            _ => RowControl::Gradient { value: ColorGradient::default(), discrete: in_effect && !valid.is_empty() },
        },
        ParamKind::Color | ParamKind::Rgb => {
            let color = match value {
                ParamValue::Color(c) => *c,
                ParamValue::Rgb(c) => c.to_color(),
                _ => Color::EMPTY,
            };
            RowControl::Color { value: color, text: color_text(color), valid: valid.to_vec() }
        }
        ParamKind::Font => {
            let font = match value {
                ParamValue::Font(f) => f.clone(),
                _ => FontDesc::new("Arial", 8.0),
            };
            RowControl::Font { text: font_display_text(&font), font }
        }
        ParamKind::GradientList => {
            let items = if let ParamValue::GradientList(l) = value { l.clone() } else { Vec::new() };
            RowControl::GradientList { items, discrete: !valid.is_empty(), min_items: 1 }
        }
        ParamKind::ColorList => {
            let items = if let ParamValue::ColorList(l) = value { l.clone() } else { Vec::new() };
            RowControl::ColorList { items, valid: valid.to_vec(), min_items: 1 }
        }
        ParamKind::GradientLevels => {
            let items = if let ParamValue::GradientLevels(l) = value { l.clone() } else { Vec::new() };
            RowControl::GradientLevels { items, discrete: !valid.is_empty(), min_items: 1 }
        }
        ParamKind::RecordList(record) => {
            let items = if let ParamValue::RecordList(l) = value { l.as_slice() } else { &[] };
            record_list_control(descriptor, record, items, p, info, list_param, valid)
        }
        ParamKind::Other("List<string>") => RowControl::StringList { items: string_list_of(value), min_items: 1 },
        ParamKind::Other("PolygonContainer") => RowControl::Polygons,
        ParamKind::Record(_) | ParamKind::Other(_) => RowControl::Label { value: value_text(value) },
    }
}

fn record_list_control(
    descriptor: &EffectDescriptor,
    record: &'static RecordDesc,
    items: &[Params],
    p: &Params,
    info: &TargetInfo,
    list_param: &str,
    _valid: &[Color],
) -> RowControl {
    let style = list_style(record, p);
    let is_state_item = record.xml_type == "CustomStateItemData";
    let built = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut vis = Visibility::default();
            if let Some(f) = record.visibility {
                f(item, p, &mut vis);
            }
            let mut order: Vec<&'static ParamDesc> = record.params.iter().filter(|d| vis.is_visible(d)).collect();
            order.sort_by_cached_key(|d| (d.order, d.label.to_lowercase(), d.name));
            let rows = order
                .into_iter()
                .map(|d| {
                    // F51: a row of a list isn't an effect, so gradients in emitters and waves are never discrete. A custom State item asks itself (F102, F106).
                    let own_colors = if is_state_item { fx_audio_state::state::custom_item_valid_colors(item, p, info) } else { Vec::new() };
                    let row_valid: &[Color] = &own_colors;
                    RowModel {
                        key: RowKey::sub(list_param, i as u32, d.name),
                        label: d.label.to_string(),
                        tooltip: tooltip_of(Some(d.description), d.label),
                        read_only: matches!(d.editor, Editor::Label),
                        control: row_control(descriptor, d, d.editor, item.get(d.name), Some(item), p, info, list_param, row_valid, false),
                    }
                })
                .collect();
            RecordItem {
                header: match (style.numbered, style.item_text) {
                    (Some(word), _) => format!("{word} {}", i + 1),
                    (None, Some(member)) => value_text(item.get(member)),
                    _ => String::new(),
                },
                header_color: style.rule_color.map(|member| item.color(member)),
                rows,
            }
        })
        .collect();
    RowControl::RecordList { style, items: built }
}

/// A value's text in a text box or label (F30, F31).
fn value_text(value: &ParamValue) -> String {
    match value {
        ParamValue::None => String::new(),
        ParamValue::Bool(v) => if *v { "True".to_string() } else { "False".to_string() },
        ParamValue::Int(v) => v.to_string(),
        ParamValue::Long(v) => v.to_string(),
        ParamValue::Double(v) | ParamValue::Percentage(v) => double_text(*v),
        ParamValue::Str(s) | ParamValue::Enum(s) | ParamValue::Raw(s) => s.clone(),
        ParamValue::Guid(g) => g.to_string(),
        ParamValue::Time(t) => time_text(*t),
        ParamValue::Color(c) => color_text(*c),
        ParamValue::Rgb(c) => color_text(c.to_color()),
        ParamValue::Font(f) => font_display_text(f),
        _ => String::new(),
    }
}

/// Text's `TextLines`, stored as raw XML while it's the only `List<string>` (`docs/spec/effect-editor.md`, Table metadata).
fn string_list_of(value: &ParamValue) -> Vec<String> {
    fx_pixel_graphics::text::text_lines_of(value)
}

fn string_list_value(lines: &[String]) -> ParamValue {
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    fx_pixel_graphics::text::text_lines_value(&lines)
}

/// What a row hands back.
#[derive(Clone, Debug, PartialEq)]
pub enum ShownValue {
    Bool(bool),
    /// An index in the row's `Choice` items.
    Choice(u32),
    /// Number and text fields.
    Text(String),
    /// What's on the slider.
    Slider(f64),
    Color(Color),
    Curve(Curve),
    Gradient(ColorGradient),
    Font(FontDesc),
    /// F34: a chosen file. An absolute path is adopted into the effect's folder.
    Path(String),
}

/// `CommitOptions`: the same token makes the same undo step (F32's FIX and F63), for slider, strip and colour panel gestures.
#[derive(Copy, Clone, Debug, Default)]
pub struct CommitOptions {
    pub gesture: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CommitOutcome {
    NoChange,
    /// `undo_label` is what the Edit menu names. `new_step` is false when this joined the gesture's step already on the stack.
    Committed { undo_label: String, new_step: bool },
}

/// What the panel remembers between commits: the gesture whose undo step is on the stack.
#[derive(Default, Debug)]
pub struct PanelEdits {
    gesture: Option<u64>,
    /// Where chosen files go. `None` keeps paths as given.
    pub module_data_root: Option<std::path::PathBuf>,
}

impl PanelEdits {
    pub fn new(module_data_root: Option<std::path::PathBuf>) -> PanelEdits {
        PanelEdits { gesture: None, module_data_root }
    }

    /// One action per gesture: the stack holds what to go back to, so later commits of a gesture need no step of their own.
    fn record(&mut self, s: &mut SequenceSession, options: CommitOptions, label: &str, before: Vec<(EffectId, Params)>) -> CommitOutcome {
        let same_gesture = options.gesture.is_some() && options.gesture == self.gesture;
        self.gesture = options.gesture;
        if same_gesture {
            return CommitOutcome::Committed { undo_label: format!("{} Effect(s) {label} modified", before.len()), new_step: false };
        }
        let action = UndoAction::ParamsChanged { label: label.to_string(), params: before };
        let undo_label = action.description();
        s.record_undo(action);
        CommitOutcome::Committed { undo_label, new_step: true }
    }
}

/// F94's and F62's equality, which is what decides a commit is a no-op.
fn same_value(a: &ParamValue, b: &ParamValue) -> bool {
    match (a, b) {
        (ParamValue::Gradient(x), ParamValue::Gradient(y)) => gradient_same_value(x, y),
        (ParamValue::GradientList(x), ParamValue::GradientList(y)) => x.len() == y.len() && x.iter().zip(y).all(|(g, h)| gradient_same_value(g, h)),
        _ => a == b,
    }
}

fn display_of(s: &SequenceSession, id: EffectId, name: &str) -> Option<vixen_effects::framework::Display> {
    s.effect(id)?.descriptor?.params.iter().find(|d| d.name == name).map(|d| d.display)
}

/// F62: a new value is compared with First's; if equal nothing happens. Otherwise each effect gets its own copy through its own setter, and one undo step covers them all.
fn apply_to_all(
    s: &mut SequenceSession,
    edits: &mut PanelEdits,
    effects: &[EffectId],
    name: &str,
    label: &str,
    value: ParamValue,
    options: CommitOptions,
    force: bool,
) -> CommitOutcome {
    let ids: Vec<EffectId> = effects.iter().copied().filter(|id| s.effect(*id).is_some_and(|e| e.descriptor.is_some())).collect();
    let Some(first) = ids.first().copied() else { return CommitOutcome::NoChange };
    let unchanged = s.effect(first).is_some_and(|e| same_value(e.node.params.get(name), &value));
    if unchanged && !force {
        return CommitOutcome::NoChange;
    }
    let before: Vec<(EffectId, Params)> = ids.iter().filter_map(|id| s.effect(*id).map(|e| (*id, e.node.params.clone()))).collect();
    let first_display = display_of(s, first, name);
    for id in &ids {
        // F33, F61: each effect's own `display` applies to its own value. Liquid's Range is `Minus(20)` and Spirograph's is stored as shown.
        let own = match (&value, first_display, display_of(s, *id, name)) {
            (ParamValue::Int(stored), Some(from), Some(to)) if from != to => ParamValue::Int(to.stored(from.shown(*stored))),
            _ => value.clone(),
        };
        s.set_param(*id, name, own);
    }
    let outcome = edits.record(s, options, label, before);
    s.render_all_dirty();
    outcome
}

/// `commit`: F62, F63, F64.
pub fn commit(
    s: &mut SequenceSession,
    edits: &mut PanelEdits,
    effects: &[EffectId],
    key: &RowKey,
    value: ShownValue,
    options: CommitOptions,
) -> CommitOutcome {
    let chosen = selected(s, effects);
    let merged = merged_params(&chosen);
    let Some(m) = merged.iter().find(|m| m.desc.name == key.param) else { return CommitOutcome::NoChange };
    let (name, label, desc, editor) = (m.desc.name, m.desc.label, m.desc, m.editor);
    match (key.index, key.sub_param.as_deref()) {
        (None, _) => {
            let first = &chosen[0];
            // F60: an enum or depth row reads as null when mixed, so no choice can equal it.
            let force = blank_when_mixed(first.descriptor, desc) && chosen.len() > 1 && values_differ(&chosen, name);
            let merged_list = merged_choices(&chosen, m);
            let Some(stored) = stored_value(first.descriptor, desc, editor, None, first.params(), &first.info, &value, edits, Some(&merged_list)) else {
                return CommitOutcome::NoChange;
            };
            // The chooser may have copied new bytes over the existing basename.
            // An equal adopted Video path still runs its metadata lifecycle,
            // while preserving the normal no-op property/Undo semantics.
            let video_path = first.descriptor.name == "Video" && name == "FileName" && matches!(value, ShownValue::Path(_));
            let same_paths = video_path && chosen.iter().all(|choice| same_value(choice.params().get(name), &stored));
            if same_paths {
                drop(chosen);
                for id in effects { s.refresh_video_metadata(*id); }
                s.render_all_dirty();
                return CommitOutcome::NoChange;
            }
            // First may already have this filename while another selected Video
            // does not; all must receive the same chosen file in one Undo step.
            let force = force || video_path;
            drop(chosen);
            apply_to_all(s, edits, effects, name, label, stored, options, force)
        }
        (Some(index), sub) => {
            let first = &chosen[0];
            let Some(list) = list_with_item(first.descriptor, desc, first.params(), &first.info, index as usize, sub, &value, edits) else {
                return CommitOutcome::NoChange;
            };
            drop(chosen);
            apply_to_all(s, edits, effects, name, label, list, options, false)
        }
    }
}

/// `value` for this property, in stored form. `None` means revert: unparseable text, or a choice no longer there.
#[allow(clippy::too_many_arguments)]
fn stored_value(
    descriptor: &EffectDescriptor,
    desc: &ParamDesc,
    editor: Editor,
    row: Option<&Params>,
    p: &Params,
    info: &TargetInfo,
    value: &ShownValue,
    edits: &PanelEdits,
    choices: Option<&[ChoiceItem]>,
) -> Option<ParamValue> {
    match value {
        ShownValue::Bool(v) => Some(ParamValue::Bool(*v)),
        ShownValue::Choice(i) => {
            // F105: the index is into the list the row showed, merged across the selection.
            let items = choices.map_or_else(|| selection_items(descriptor, desc, row, p, info), <[ChoiceItem]>::to_vec);
            items.into_iter().nth(*i as usize).map(|c| match (desc.kind, c.value) {
                // A list of plain text on an enum or the reverse keeps the property's own form.
                (ParamKind::Enum(_), ParamValue::Str(s)) => ParamValue::Enum(s),
                (ParamKind::Str, ParamValue::Enum(s)) => ParamValue::Str(s),
                (_, v) => v,
            })
        }
        ShownValue::Text(text) => text_to_value(desc, editor, text),
        ShownValue::Slider(v) => match editor {
            Editor::Slider { .. } if matches!(desc.kind, ParamKind::Int | ParamKind::Long | ParamKind::Other("ulong")) => {
                Some(ParamValue::Int(desc.display.stored(to_int32(*v))))
            }
            Editor::Level => Some(ParamValue::Double(*v / 100.0)),
            Editor::Percent => {
                let f = percent_from_slider(*v);
                Some(if matches!(desc.kind, ParamKind::Percentage) { ParamValue::Percentage(f) } else { ParamValue::Double(f) })
            }
            _ => Some(ParamValue::Double(*v)),
        },
        ShownValue::Color(c) => Some(match desc.kind {
            ParamKind::Rgb => ParamValue::Rgb(vixen_core::Rgb::from_color(*c)),
            _ => ParamValue::Color(*c),
        }),
        ShownValue::Curve(c) => Some(ParamValue::Curve(c.clone())),
        ShownValue::Gradient(g) => Some(ParamValue::Gradient(g.clone())),
        ShownValue::Font(f) => Some(ParamValue::Font(f.clone())),
        ShownValue::Path(path) => Some(ParamValue::Str(adopted_path(descriptor, desc, path, edits.module_data_root.as_deref()))),
    }
}

fn text_to_value(desc: &ParamDesc, editor: Editor, text: &str) -> Option<ParamValue> {
    match desc.kind {
        ParamKind::Int => {
            let shown = int_field(desc.kind, editor).parse(text)?;
            Some(ParamValue::Int(desc.display.stored(shown as i32)))
        }
        ParamKind::Long => Some(ParamValue::Long(int_field(desc.kind, editor).parse(text)? as i64)),
        ParamKind::Other("ulong") => Some(ParamValue::Raw(int_field(desc.kind, editor).parse(text)?.to_string())),
        ParamKind::Double => Some(ParamValue::Double(double_field(editor).parse(text)?)),
        ParamKind::Percentage => Some(ParamValue::Percentage(double_field(editor).parse(text)?)),
        ParamKind::Str => Some(ParamValue::Str(text.to_string())),
        // Empty text means null for non-string types, which for a Guid is `Guid.Empty`.
        ParamKind::Guid => {
            if text.trim().is_empty() {
                Some(ParamValue::Guid(Uuid::nil()))
            } else {
                Uuid::parse_str(text.trim()).ok().map(ParamValue::Guid)
            }
        }
        _ => None,
    }
}

/// F34's data table: Picture, Glediator, Shapes and Video keep their files in their own folder. Choosing an absolute path copies the file there, overwriting one of that name, and stores just the name. A relative path, or a Launcher's executable, stays as given.
fn adopted_path(descriptor: &EffectDescriptor, desc: &ParamDesc, path: &str, root: Option<&Path>) -> String {
    let source = Path::new(path);
    let folder = match desc.editor {
        Editor::Path("Image" | "Video" | "Svg" | "Gled") => descriptor.name,
        _ => return path.to_string(),
    };
    let (Some(root), true, Some(file)) = (root, source.is_absolute(), source.file_name()) else { return path.to_string() };
    let target_dir = root.join(folder);
    let target = target_dir.join(file);
    if target != source {
        let copied = std::fs::create_dir_all(&target_dir).and_then(|_| std::fs::copy(source, &target));
        if copied.is_err() {
            return path.to_string();
        }
    }
    file.to_string_lossy().into_owned()
}

/// A copy of First's whole list with this item changed, the way every list edit goes (F55, F62). `None` means nothing to do.
#[allow(clippy::too_many_arguments)]
fn list_with_item(
    descriptor: &EffectDescriptor,
    desc: &ParamDesc,
    p: &Params,
    info: &TargetInfo,
    index: usize,
    sub: Option<&str>,
    value: &ShownValue,
    edits: &PanelEdits,
) -> Option<ParamValue> {
    match (p.get(desc.name), desc.kind) {
        (ParamValue::GradientList(list), _) => {
            let mut list = list.clone();
            let ShownValue::Gradient(g) = value else { return None };
            *list.get_mut(index)? = g.clone();
            Some(ParamValue::GradientList(list))
        }
        (ParamValue::ColorList(list), _) => {
            let mut list = list.clone();
            let ShownValue::Color(c) = value else { return None };
            *list.get_mut(index)? = *c;
            Some(ParamValue::ColorList(list))
        }
        (ParamValue::GradientLevels(list), _) => {
            let mut list = list.clone();
            let pair = list.get_mut(index)?;
            match value {
                ShownValue::Curve(c) => pair.curve = c.clone(),
                ShownValue::Gradient(g) => pair.color_gradient = g.clone(),
                _ => return None,
            }
            Some(ParamValue::GradientLevels(list))
        }
        (raw, ParamKind::Other("List<string>")) => {
            let mut lines = string_list_of(raw);
            let ShownValue::Text(text) = value else { return None };
            *lines.get_mut(index)? = text.clone();
            Some(string_list_value(&lines))
        }
        (ParamValue::RecordList(list), ParamKind::RecordList(_)) => {
            let sub = sub?;
            let mut list = list.clone();
            let item = list.get_mut(index)?;
            let sub_desc = item.desc.iter().find(|d| d.name == sub)?;
            let snapshot = item.clone();
            let stored = stored_value(descriptor, sub_desc, sub_desc.editor, Some(&snapshot), p, info, value, edits, None)?;
            if !same_value(snapshot.get(sub), &stored) {
                item.set(sub, stored);
            } else {
                return None;
            }
            Some(ParamValue::RecordList(list))
        }
        _ => None,
    }
}

fn first_param(s: &SequenceSession, effects: &[EffectId], name: &str) -> Option<(&'static ParamDesc, ParamValue)> {
    let chosen = selected(s, effects);
    let first = chosen.first()?;
    let desc = first.descriptor.params.iter().find(|d| d.name == name)?;
    Some((desc, first.params().get(name).clone()))
}

/// F56: appends the default of the first item's type. An empty list adds only if it's a record list, which knows its declared type. Every effect gets First's list.
pub fn list_add(s: &mut SequenceSession, edits: &mut PanelEdits, effects: &[EffectId], param: &str) -> CommitOutcome {
    let Some((desc, value)) = first_param(s, effects, param) else { return CommitOutcome::NoChange };
    let new_value = match (value, desc.kind) {
        (ParamValue::GradientList(mut list), _) if !list.is_empty() => {
            list.push(ColorGradient::default());
            ParamValue::GradientList(list)
        }
        // `Color.Empty`, which is transparent (KEEP, F56).
        (ParamValue::ColorList(mut list), _) if !list.is_empty() => {
            list.push(Color::EMPTY);
            ParamValue::ColorList(list)
        }
        (ParamValue::GradientLevels(mut list), _) if !list.is_empty() => {
            list.push(GradientLevelPair::new(ColorGradient::default(), Curve::flat100()));
            ParamValue::GradientLevels(list)
        }
        // Morph's list isn't expando-backed, so an empty one can't add.
        (ParamValue::RecordList(mut list), ParamKind::RecordList(record)) if !list.is_empty() || record.xml_type != "MorphPolygonData" => {
            // F56: with several effects and every list empty, the merged read is null and there's nothing to add to.
            let everyone_empty = effects.iter().filter_map(|id| s.effect(*id)).all(|e| e.node.params.records(desc.name).is_empty());
            if list.is_empty() && effects.len() > 1 && everyone_empty {
                return CommitOutcome::NoChange;
            }
            let item = if record.xml_type == "FixtureFunctionData" {
                let chosen = selected(s, effects);
                let Some(first) = chosen.first() else { return CommitOutcome::NoChange };
                let Some(item) = fx_fixture::fixture::new_function_data(first.params(), &first.info) else { return CommitOutcome::NoChange };
                item
            } else {
                Params::defaults(record.params)
            };
            list.push(item);
            ParamValue::RecordList(list)
        }
        (raw, ParamKind::Other("List<string>")) => {
            let mut lines = string_list_of(&raw);
            if lines.is_empty() {
                return CommitOutcome::NoChange;
            }
            lines.push(String::new());
            string_list_value(&lines)
        }
        _ => return CommitOutcome::NoChange,
    };
    apply_to_all(s, edits, effects, desc.name, desc.label, new_value, CommitOptions::default(), false)
}

fn list_len(value: &ParamValue, kind: ParamKind) -> usize {
    match (value, kind) {
        (ParamValue::GradientList(l), _) => l.len(),
        (ParamValue::ColorList(l), _) => l.len(),
        (ParamValue::GradientLevels(l), _) => l.len(),
        (ParamValue::RecordList(l), _) => l.len(),
        (raw, ParamKind::Other("List<string>")) => string_list_of(raw).len(),
        _ => 0,
    }
}

/// F57's minimum: 1, or the record list's own.
fn min_items(desc: &ParamDesc, p: &Params) -> usize {
    match desc.kind {
        ParamKind::RecordList(record) => list_style(record, p).min_items as usize,
        _ => 1,
    }
}

/// F57: enabled with a row selected and more items than the minimum.
pub fn can_remove(s: &SequenceSession, effects: &[EffectId], param: &str, selected_row: Option<u32>) -> bool {
    let chosen = selected(s, effects);
    let Some(first) = chosen.first() else { return false };
    let Some(desc) = first.descriptor.params.iter().find(|d| d.name == param) else { return false };
    let Some(i) = selected_row else { return false };
    let len = list_len(first.params().get(param), desc.kind);
    (i as usize) < len && len > min_items(desc, first.params())
}

/// F57.
pub fn list_remove(s: &mut SequenceSession, edits: &mut PanelEdits, effects: &[EffectId], param: &str, index: u32) -> CommitOutcome {
    if !can_remove(s, effects, param, Some(index)) {
        return CommitOutcome::NoChange;
    }
    let Some((desc, value)) = first_param(s, effects, param) else { return CommitOutcome::NoChange };
    let i = index as usize;
    let new_value = match (value, desc.kind) {
        (ParamValue::GradientList(mut l), _) => {
            l.remove(i);
            ParamValue::GradientList(l)
        }
        (ParamValue::ColorList(mut l), _) => {
            l.remove(i);
            ParamValue::ColorList(l)
        }
        (ParamValue::GradientLevels(mut l), _) => {
            l.remove(i);
            ParamValue::GradientLevels(l)
        }
        (ParamValue::RecordList(mut l), _) => {
            l.remove(i);
            ParamValue::RecordList(l)
        }
        (raw, ParamKind::Other("List<string>")) => {
            let mut lines = string_list_of(&raw);
            lines.remove(i);
            string_list_value(&lines)
        }
        _ => return CommitOutcome::NoChange,
    };
    apply_to_all(s, edits, effects, desc.name, desc.label, new_value, CommitOptions::default(), false)
}

/// `PropertyItem.TryMoveItem`: both ends in range and different.
fn valid_move(from: i64, to: i64, count: i64) -> bool {
    from != to && from >= 0 && to >= 0 && from < count && to < count
}

/// F59: only on the three list kinds; enabled with a row selected and the target in range.
pub fn can_move(s: &SequenceSession, effects: &[EffectId], param: &str, selected_row: Option<u32>, delta: i32) -> bool {
    let Some((desc, value)) = first_param(s, effects, param) else { return false };
    if !matches!(desc.kind, ParamKind::GradientList | ParamKind::ColorList | ParamKind::GradientLevels) {
        return false;
    }
    let Some(i) = selected_row else { return false };
    valid_move(i as i64, i as i64 + delta as i64, list_len(&value, desc.kind) as i64)
}

/// F59.
pub fn list_move(s: &mut SequenceSession, edits: &mut PanelEdits, effects: &[EffectId], param: &str, from: u32, to: u32) -> CommitOutcome {
    let Some((desc, value)) = first_param(s, effects, param) else { return CommitOutcome::NoChange };
    if !matches!(desc.kind, ParamKind::GradientList | ParamKind::ColorList | ParamKind::GradientLevels) {
        return CommitOutcome::NoChange;
    }
    if !valid_move(from as i64, to as i64, list_len(&value, desc.kind) as i64) {
        return CommitOutcome::NoChange;
    }
    fn moved<T>(mut l: Vec<T>, from: u32, to: u32) -> Vec<T> {
        let item = l.remove(from as usize);
        l.insert(to as usize, item);
        l
    }
    let new_value = match value {
        ParamValue::GradientList(l) => ParamValue::GradientList(moved(l, from, to)),
        ParamValue::ColorList(l) => ParamValue::ColorList(moved(l, from, to)),
        ParamValue::GradientLevels(l) => ParamValue::GradientLevels(moved(l, from, to)),
        _ => return CommitOutcome::NoChange,
    };
    apply_to_all(s, edits, effects, desc.name, desc.label, new_value, CommitOptions::default(), false)
}

/// What kind of value a row holds, for F68's table.
#[derive(Copy, Clone, Debug, PartialEq)]
enum DropTarget {
    Curve,
    Gradient,
    Color,
}

fn drop_target(desc: &ParamDesc, key: &RowKey) -> Option<(DropTarget, bool)> {
    match (desc.kind, key.index, key.sub_param.as_deref()) {
        (ParamKind::Curve, None, _) => Some((DropTarget::Curve, true)),
        (ParamKind::Gradient, None, _) => Some((DropTarget::Gradient, true)),
        (ParamKind::Color | ParamKind::Rgb, None, _) => Some((DropTarget::Color, true)),
        (ParamKind::GradientList, Some(_), _) => Some((DropTarget::Gradient, true)),
        (ParamKind::ColorList, Some(_), _) => Some((DropTarget::Color, true)),
        (ParamKind::GradientLevels, Some(_), Some("Curve")) => Some((DropTarget::Curve, true)),
        (ParamKind::GradientLevels, Some(_), Some("ColorGradient")) => Some((DropTarget::Gradient, true)),
        (ParamKind::RecordList(record), Some(_), Some(sub)) => {
            let d = record.params.iter().find(|d| d.name == sub)?;
            // The rule reads the row's own component, which isn't discrete but for a State item.
            let effect_colors = record.xml_type == "CustomStateItemData";
            match d.kind {
                ParamKind::Curve => Some((DropTarget::Curve, false)),
                ParamKind::Gradient => Some((DropTarget::Gradient, false)),
                ParamKind::Color | ParamKind::Rgb => Some((DropTarget::Color, effect_colors)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// F68's table and discrete rule. FIX: a gradient on a colour row is refused, not accepted and ignored.
pub fn can_drop(s: &SequenceSession, effects: &[EffectId], key: &RowKey, payload: &DragPayload) -> bool {
    let chosen = selected(s, effects);
    let merged = merged_params(&chosen);
    let Some(m) = merged.iter().find(|m| m.desc.name == key.param) else { return false };
    let Some((target, uses_discrete)) = drop_target(m.desc, key) else { return false };
    let valid = if uses_discrete { union_of_valid_colors(&chosen) } else { Vec::new() };
    let allowed = |c: Color| valid.is_empty() || valid.contains(&c);
    match (target, payload) {
        (DropTarget::Curve, DragPayload::Curve(_)) => true,
        (DropTarget::Gradient, DragPayload::Gradient(g)) => g.colors_in_gradient().iter().all(|c| allowed(*c)),
        (DropTarget::Gradient | DropTarget::Color, DragPayload::Color(c)) => allowed(*c),
        _ => false,
    }
}

/// F68: one undo step, as usual. A colour on a gradient row makes a solid gradient.
pub fn drop_value(s: &mut SequenceSession, edits: &mut PanelEdits, effects: &[EffectId], key: &RowKey, payload: &DragPayload) -> CommitOutcome {
    if !can_drop(s, effects, key, payload) {
        return CommitOutcome::NoChange;
    }
    let chosen = selected(s, effects);
    let merged = merged_params(&chosen);
    let Some(m) = merged.iter().find(|m| m.desc.name == key.param) else { return CommitOutcome::NoChange };
    let Some((target, _)) = drop_target(m.desc, key) else { return CommitOutcome::NoChange };
    let value = match (target, payload) {
        (DropTarget::Curve, DragPayload::Curve(c)) => ShownValue::Curve(c.clone()),
        (DropTarget::Gradient, DragPayload::Gradient(g)) => ShownValue::Gradient(g.clone()),
        (DropTarget::Gradient, DragPayload::Color(c)) => ShownValue::Gradient(ColorGradient::solid(*c)),
        (DropTarget::Color, DragPayload::Color(c)) => ShownValue::Color(*c),
        _ => return CommitOutcome::NoChange,
    };
    drop(chosen);
    commit(s, edits, effects, key, value, CommitOptions::default())
}

/// F102, F51: the colours a row's editors offer. Empty means free colour.
pub fn discrete_colors(s: &SequenceSession, effects: &[EffectId]) -> Vec<Color> {
    union_of_valid_colors(&selected(s, effects))
}

/// For the test window.
pub fn descriptor_named(name: &str) -> Option<&'static EffectDescriptor> {
    vixen_effect_registry::all().iter().copied().find(|d| d.name == name)
}

/// Every effect module's name.
pub fn effect_names() -> Vec<String> {
    vixen_effect_registry::all().iter().map(|d| d.name.to_string()).collect()
}

/// For tests and the harness: a short description of an effect's rendered intents, which changes when its render does.
pub fn render_summary(s: &SequenceSession, id: EffectId) -> String {
    let Some(entry) = s.effect(id) else { return "no effect".to_string() };
    let elements = entry.intents.as_ref().map_or(0, |i| i.map.len());
    let intents: usize = entry.intents.as_ref().map_or(0, |i| i.map.values().map(|n| n.len()).sum());
    format!("generation {} rendered {} elements {elements} intents {intents} {}",
        entry.generation, entry.rendered_generation, render_color_sample(entry.intents.as_deref()))
}

// Read the existing cached intents only. In particular, a dirty effect must still
// report its old colors until its normal render-result owner applies a new render.
// Bounds cover sorting, node sampling and the resulting histogram, even for a
// very large selection. An oversized element map is not sampled arbitrarily.
const RENDER_SAMPLE_LIMIT: usize = 4096;

fn render_color_sample(intents: Option<&vixen_core::EffectIntents>) -> String {
    use std::collections::BTreeMap;
    use vixen_core::{StateValue, TimeSpan};
    let at = TimeSpan::from_ms_i(50);
    let mut colors = BTreeMap::<u32, usize>::new();
    let (mut inspected, mut active, mut non_color, mut truncated) = (0, 0, 0, false);
    if let Some(intents) = intents {
        if intents.map.len() > RENDER_SAMPLE_LIMIT {
            truncated = true;
        } else {
            let mut elements: Vec<_> = intents.map.iter().collect();
            elements.sort_unstable_by_key(|(id, _)| **id);
            'elements: for (_, nodes) in elements {
                for node in nodes {
                    if inspected == RENDER_SAMPLE_LIMIT {
                        truncated = true;
                        break 'elements;
                    }
                    inspected += 1;
                    if !node.contains(at) { continue; }
                    active += 1;
                    let color = match node.intent.state_at(at - node.start) {
                        StateValue::Rgb(value) => value.full_color(),
                        StateValue::Lighting(value) => value.full_color(),
                        _ => { non_color += 1; continue; }
                    };
                    *colors.entry(color.to_argb_u32()).or_default() += 1;
                }
            }
        }
    }
    let colors = colors.iter().map(|(argb, count)| format!("{argb:08X}:{count}")).collect::<Vec<_>>().join(",");
    format!("sampleMs 50 inspected {inspected} active {active} nonColor {non_color} truncated {truncated} colors [{colors}]")
}

#[cfg(test)]
#[path = "render_probe_tests.rs"]
mod render_probe_tests;

#[cfg(test)]
#[path = "fixture_authoring_tests.rs"]
mod fixture_authoring_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::SessionEnvironment;
    use std::sync::Arc;
    use vixen_core::{ElementTree, TimeSpan};
    use vixen_io::ModuleStore;

    fn descriptor(name: &str) -> &'static EffectDescriptor {
        vixen_effect_registry::all().iter().copied().find(|d| d.name == name).unwrap_or_else(|| panic!("no effect {name}"))
    }

    struct Fixture {
        session: SequenceSession,
        target: Uuid,
    }

    fn fixture() -> Fixture {
        let mut tree = ElementTree::new();
        let string = tree.add_string("Arch", 8);
        let target = tree.node(string).id;
        let env = SessionEnvironment { video_cache: None,
            tree: Arc::new(tree),
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 50,
            media_dir: std::env::temp_dir(),
        };
        Fixture { session: SequenceSession::new_untitled(&env), target }
    }

    impl Fixture {
        fn add(&mut self, name: &str) -> EffectId {
            let d = descriptor(name);
            self.session.add_new_effect(d.type_id, self.target, "Arch", TimeSpan::ZERO, TimeSpan::from_ms(1000.0)).unwrap()
        }
    }

    fn row<'a>(m: &'a PanelModel, param: &str) -> &'a RowModel {
        m.categories.iter().flat_map(|c| &c.rows).find(|r| r.key.param == param).unwrap_or_else(|| panic!("no row {param}"))
    }

    fn has_row(m: &PanelModel, param: &str) -> bool {
        m.categories.iter().flat_map(|c| &c.rows).any(|r| r.key.param == param)
    }

    #[test]
    fn nothing_selected_has_the_message_and_link() {
        let f = fixture();
        let m = panel_model(&f.session, &[], DEFAULT_MULTI_EDIT_LIMIT);
        assert!(m.categories.is_empty());
        assert_eq!(m.effect_name, "");
        assert_eq!(m.information, "Select an Effect to edit.");
        assert_eq!(m.information_link, DEFAULT_INFO_LINK);
    }

    #[test]
    fn seventy_five_are_allowed_and_seventy_six_are_not() {
        let mut f = fixture();
        let ids: Vec<EffectId> = (0..76).map(|_| f.add("Pulse")).collect();
        let m = panel_model(&f.session, &ids[..75], DEFAULT_MULTI_EDIT_LIMIT);
        assert_eq!(m.effect_count, 75);
        assert!(!m.categories.is_empty());
        let m = panel_model(&f.session, &ids, DEFAULT_MULTI_EDIT_LIMIT);
        assert!(m.categories.is_empty());
        assert_eq!(m.information, "Multi-Edit is limited to < 75 Effects");
    }

    #[test]
    fn rows_follow_the_tables_order() {
        let mut f = fixture();
        for name in ["Pulse", "Butterfly", "Chase", "Wipe", "Twinkle", "Bars", "Spiral"] {
            let id = f.add(name);
            let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
            let d = descriptor(name);
            let entry = f.session.effect(id).unwrap();
            let info = f.session.target_info(id).unwrap();
            let expected: Vec<&str> = d.visible_params_for(&entry.node.params, &info).iter().map(|i| d.params[*i].name).collect();
            let actual: Vec<&str> = m.categories.iter().flat_map(|c| &c.rows).map(|r| r.key.param.as_str()).collect();
            assert_eq!(actual, expected, "{name}");
            assert_eq!(m.effect_name, name);
        }
    }

    #[test]
    fn pulse_is_two_rows_in_their_categories() {
        let mut f = fixture();
        let id = f.add("Pulse");
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let names: Vec<&str> = m.categories.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Color", "Brightness"]);
        assert!(matches!(row(&m, "ColorGradient").control, RowControl::Gradient { .. }));
        assert!(matches!(row(&m, "LevelCurve").control, RowControl::Curve { .. }));
    }

    #[test]
    fn a_mixed_selection_shows_firsts_value_and_blank_enums() {
        let mut f = fixture();
        let a = f.add("Chase");
        let b = f.add("Chase");
        f.session.set_param(b, "ColorHandling", ParamValue::Enum("GradientThroughWholeEffect".to_string()));
        let mut edits = PanelEdits::default();
        let first = panel_model(&f.session, &[a], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Choice { selected, .. } = &row(&first, "ColorHandling").control else { panic!() };
        assert!(selected.is_some());
        let both = panel_model(&f.session, &[a, b], DEFAULT_MULTI_EDIT_LIMIT);
        let current = f.session.effect(a).unwrap().node.params.enum_name("ColorHandling").to_string();
        let other = f.session.effect(b).unwrap().node.params.enum_name("ColorHandling").to_string();
        if current != other {
            let RowControl::Choice { selected, items } = &row(&both, "ColorHandling").control else { panic!() };
            assert_eq!(*selected, None);
            // From blank, First's own value applies to all.
            let index = items.iter().position(|i| i.value == ParamValue::Enum(current.clone())).unwrap() as u32;
            let outcome = commit(&mut f.session, &mut edits, &[a, b], &RowKey::top("ColorHandling"), ShownValue::Choice(index), CommitOptions::default());
            assert!(matches!(outcome, CommitOutcome::Committed { .. }));
            assert_eq!(f.session.effect(b).unwrap().node.params.enum_name("ColorHandling"), current);
        }
    }

    #[test]
    fn different_effects_merge_their_common_rows() {
        let mut f = fixture();
        let pulse = f.add("Pulse");
        let spin = f.add("Spin");
        f.session.set_param(spin, "ColorHandling", ParamValue::Enum("GradientThroughWholeEffect".to_string()));
        let m = panel_model(&f.session, &[pulse, spin], DEFAULT_MULTI_EDIT_LIMIT);
        assert_eq!(m.effect_name, "Pulse");
        assert!(has_row(&m, "ColorGradient"));
        assert!(!has_row(&m, "PulseLevelCurve"));
    }

    #[test]
    fn setting_firsts_value_on_a_mixed_row_does_nothing() {
        let mut f = fixture();
        let a = f.add("Pulse");
        let b = f.add("Pulse");
        f.session.set_param(b, "LevelCurve", ParamValue::Curve(Curve::flat(30.0)));
        let mut edits = PanelEdits::default();
        let first_value = f.session.effect(a).unwrap().node.params.curve("LevelCurve");
        let outcome = commit(&mut f.session, &mut edits, &[a, b], &RowKey::top("LevelCurve"), ShownValue::Curve(first_value), CommitOptions::default());
        assert_eq!(outcome, CommitOutcome::NoChange);
        assert!(!f.session.undo_stack().can_undo());
        assert_eq!(f.session.effect(b).unwrap().node.params.curve("LevelCurve"), Curve::flat(30.0));
        // Another value goes to all, and undo restores each.
        let outcome = commit(&mut f.session, &mut edits, &[a, b], &RowKey::top("LevelCurve"), ShownValue::Curve(Curve::flat(70.0)), CommitOptions::default());
        assert_eq!(outcome, CommitOutcome::Committed { undo_label: "2 Effect(s) Intensity modified".to_string(), new_step: true });
        assert_eq!(f.session.effect(a).unwrap().node.params.curve("LevelCurve"), Curve::flat(70.0));
        assert_eq!(f.session.effect(b).unwrap().node.params.curve("LevelCurve"), Curve::flat(70.0));
        f.session.undo(1);
        assert_eq!(f.session.effect(b).unwrap().node.params.curve("LevelCurve"), Curve::flat(30.0));
    }

    #[test]
    fn gain_converts_and_a_write_that_stores_nothing_new_is_no_step() {
        let mut f = fixture();
        let id = f.add("Vertical Meter");
        let mut edits = PanelEdits::default();
        f.session.set_param(id, "Gain", ParamValue::Int(3));
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Slider { value, .. } = row(&m, "Gain").control else { panic!() };
        assert_eq!(value, 30.0);
        let key = RowKey::top("Gain");
        assert_eq!(commit(&mut f.session, &mut edits, &[id], &key, ShownValue::Slider(37.0), CommitOptions::default()), CommitOutcome::NoChange);
        assert!(!f.session.undo_stack().can_undo());
        let outcome = commit(&mut f.session, &mut edits, &[id], &key, ShownValue::Slider(47.0), CommitOptions::default());
        assert!(matches!(outcome, CommitOutcome::Committed { .. }));
        assert_eq!(f.session.effect(id).unwrap().node.params.int("Gain"), 4);
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Slider { value, .. } = row(&m, "Gain").control else { panic!() };
        assert_eq!(value, 40.0);
    }

    #[test]
    fn range_and_scroll_speed_are_shown_as_a_minus() {
        let mut f = fixture();
        let id = f.add("Vertical Meter");
        let mut edits = PanelEdits::default();
        f.session.set_param(id, "Range", ParamValue::Int(10));
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Slider { value, .. } = row(&m, "Range").control else { panic!() };
        assert_eq!(value, 10.0);
        commit(&mut f.session, &mut edits, &[id], &RowKey::top("Range"), ShownValue::Slider(15.0), CommitOptions::default());
        assert_eq!(f.session.effect(id).unwrap().node.params.int("Range"), 5);

        let w = f.add("Waveform");
        f.session.set_param(w, "ScrollSpeed", ParamValue::Int(10));
        let m = panel_model(&f.session, &[w], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Slider { value, .. } = row(&m, "ScrollSpeed").control else { panic!() };
        assert_eq!(value, 40.0);
        commit(&mut f.session, &mut edits, &[w], &RowKey::top("ScrollSpeed"), ShownValue::Slider(50.0), CommitOptions::default());
        assert_eq!(f.session.effect(w).unwrap().node.params.int("ScrollSpeed"), 0);
    }

    #[test]
    fn a_gesture_is_one_undo_step() {
        let mut f = fixture();
        let id = f.add("Pulse");
        let mut edits = PanelEdits::default();
        let before = f.session.effect(id).unwrap().node.params.curve("LevelCurve");
        let key = RowKey::top("LevelCurve");
        let options = CommitOptions { gesture: Some(7) };
        let a = commit(&mut f.session, &mut edits, &[id], &key, ShownValue::Curve(Curve::flat(10.0)), options);
        let b = commit(&mut f.session, &mut edits, &[id], &key, ShownValue::Curve(Curve::flat(20.0)), options);
        assert!(matches!(a, CommitOutcome::Committed { new_step: true, .. }));
        assert!(matches!(b, CommitOutcome::Committed { new_step: false, .. }));
        assert_eq!(f.session.undo_stack().undo_descriptions().len(), 1);
        f.session.undo(1);
        assert_eq!(f.session.effect(id).unwrap().node.params.curve("LevelCurve"), before);
        // Redo returns to the gesture's end.
        f.session.redo(1);
        assert_eq!(f.session.effect(id).unwrap().node.params.curve("LevelCurve"), Curve::flat(20.0));
    }

    #[test]
    fn list_edits_give_every_effect_firsts_list() {
        let mut f = fixture();
        let a = f.add("Bars");
        let b = f.add("Bars");
        f.session.set_param(b, "Colors", ParamValue::GradientList(vec![ColorGradient::solid(Color::RED)]));
        let mut edits = PanelEdits::default();
        let count = f.session.effect(a).unwrap().node.params.gradients("Colors").len();
        let outcome = list_add(&mut f.session, &mut edits, &[a, b], "Colors");
        assert!(matches!(outcome, CommitOutcome::Committed { .. }));
        assert_eq!(f.session.effect(a).unwrap().node.params.gradients("Colors").len(), count + 1);
        assert_eq!(f.session.effect(b).unwrap().node.params.gradients("Colors"), f.session.effect(a).unwrap().node.params.gradients("Colors"));
        assert!(can_remove(&f.session, &[a], "Colors", Some(0)));
        assert!(!can_remove(&f.session, &[a], "Colors", None));
        assert!(!can_move(&f.session, &[a], "Colors", Some(0), -1));
        assert!(can_move(&f.session, &[a], "Colors", Some(0), 1));
        list_move(&mut f.session, &mut edits, &[a], "Colors", 0, 1);
        list_remove(&mut f.session, &mut edits, &[a], "Colors", 1);
        assert_eq!(f.session.effect(a).unwrap().node.params.gradients("Colors").len(), count);
        // Down to one, Remove refuses.
        let c = f.add("Bars");
        f.session.set_param(c, "Colors", ParamValue::GradientList(vec![ColorGradient::default()]));
        assert!(!can_remove(&f.session, &[c], "Colors", Some(0)));
        assert_eq!(list_remove(&mut f.session, &mut edits, &[c], "Colors", 0), CommitOutcome::NoChange);
    }

    #[test]
    fn move_item_rules_match_upstreams_tests() {
        for (from, to) in [(-1, 0), (0, -1), (1, 1), (3, 0), (0, 3), (5, 0), (0, 5)] {
            assert!(!valid_move(from, to, 3), "{from} {to}");
        }
        assert!(valid_move(1, 0, 3));
        assert!(valid_move(1, 2, 3));
        assert!(!valid_move(0, 1, 0));
        assert!(!valid_move(0, 1, 1));
    }

    #[test]
    fn record_lists_edit_a_row_and_keep_their_identity() {
        let mut f = fixture();
        let id = f.add("Wave");
        let mut edits = PanelEdits::default();
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::RecordList { items, style } = &row(&m, "WaveformData").control else { panic!() };
        assert_eq!(style.numbered, Some("Wave"));
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].header, "Wave 2");
        assert!(items[0].rows.iter().all(|r| r.key.param == "WaveformData" && r.key.index == Some(0)));
        // Sliders, curves and enums all get their own editors in a row.
        assert!(items[0].rows.iter().any(|r| matches!(r.control, RowControl::Curve { .. })));
        let key = RowKey::sub("WaveformData", 1, "Mirror");
        let outcome = commit(&mut f.session, &mut edits, &[id], &key, ShownValue::Bool(true), CommitOptions::default());
        assert!(matches!(outcome, CommitOutcome::Committed { .. }));
        assert!(f.session.effect(id).unwrap().node.params.records("WaveformData")[1].boolean("Mirror"));
        // F51: a gradient in a row isn't discrete.
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::RecordList { items, .. } = &row(&m, "WaveformData").control else { panic!() };
        let gradient = items[0].rows.iter().find(|r| r.key.sub_param.as_deref() == Some("Color")).unwrap();
        assert!(matches!(gradient.control, RowControl::Gradient { discrete: false, .. }));
        assert!(list_remove(&mut f.session, &mut edits, &[id], "WaveformData", 0) != CommitOutcome::NoChange);
        assert_eq!(f.session.effect(id).unwrap().node.params.records("WaveformData").len(), 1);
        assert_eq!(list_remove(&mut f.session, &mut edits, &[id], "WaveformData", 0), CommitOutcome::NoChange);
        assert!(matches!(list_add(&mut f.session, &mut edits, &[id], "WaveformData"), CommitOutcome::Committed { .. }));
    }

    #[test]
    fn numbers_revert_on_bad_text() {
        let mut f = fixture();
        let id = f.add("Twinkle");
        let mut edits = PanelEdits::default();
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let number = m.categories.iter().flat_map(|c| &c.rows).find(|r| matches!(r.control, RowControl::Int { .. })).map(|r| r.key.clone());
        if let Some(key) = number {
            let before = f.session.undo_stack().undo_descriptions().len();
            assert_eq!(commit(&mut f.session, &mut edits, &[id], &key, ShownValue::Text("12.7".into()), CommitOptions::default()), CommitOutcome::NoChange);
            assert_eq!(f.session.undo_stack().undo_descriptions().len(), before);
        }
    }

    #[test]
    fn custom_value_keeps_its_types() {
        let mut f = fixture();
        let id = f.add("Custom Value");
        let mut edits = PanelEdits::default();
        f.session.set_param(id, "ValueType", ParamValue::Enum("_64Bit".into()));
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Int { field, .. } = &row(&m, "Value64Bit").control else { panic!() };
        assert_eq!(field.max, u64::MAX as i128);
        commit(&mut f.session, &mut edits, &[id], &RowKey::top("Value64Bit"), ShownValue::Text("18446744073709551615".into()), CommitOptions::default());
        assert_eq!(f.session.effect(id).unwrap().node.params.get("Value64Bit"), &ParamValue::Raw("18446744073709551615".into()));
        f.session.set_param(id, "ValueType", ParamValue::Enum("_8Bit".into()));
        commit(&mut f.session, &mut edits, &[id], &RowKey::top("Value8Bit"), ShownValue::Text("300".into()), CommitOptions::default());
        assert_eq!(f.session.effect(id).unwrap().node.params.int("Value8Bit"), 255);
    }

    #[test]
    fn levels_and_percentages_convert() {
        let mut f = fixture();
        let id = f.add("Set Level");
        let mut edits = PanelEdits::default();
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Slider { snap, min, max, .. } = row(&m, "IntensityLevel").control else { panic!() };
        assert!(snap);
        assert_eq!((min, max), (0.0, 100.0));
        commit(&mut f.session, &mut edits, &[id], &RowKey::top("IntensityLevel"), ShownValue::Slider(29.0), CommitOptions::default());
        assert_eq!(f.session.effect(id).unwrap().node.params.double("IntensityLevel"), 0.29);
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Slider { tooltip, .. } = &row(&m, "IntensityLevel").control else { panic!() };
        assert_eq!(tooltip, "28");
    }

    #[test]
    fn drops_follow_the_table() {
        let mut f = fixture();
        let id = f.add("Pulse");
        let mut edits = PanelEdits::default();
        let curve = DragPayload::Curve(Curve::flat(40.0));
        let gradient = DragPayload::Gradient(ColorGradient::solid(Color::BLUE));
        let color = DragPayload::Color(Color::RED);
        let curve_row = RowKey::top("LevelCurve");
        let gradient_row = RowKey::top("ColorGradient");
        assert!(can_drop(&f.session, &[id], &curve_row, &curve));
        assert!(!can_drop(&f.session, &[id], &curve_row, &gradient));
        assert!(!can_drop(&f.session, &[id], &curve_row, &color));
        assert!(can_drop(&f.session, &[id], &gradient_row, &gradient));
        assert!(can_drop(&f.session, &[id], &gradient_row, &color));
        assert!(!can_drop(&f.session, &[id], &gradient_row, &curve));
        drop_value(&mut f.session, &mut edits, &[id], &gradient_row, &color);
        assert_eq!(f.session.effect(id).unwrap().node.params.gradient("ColorGradient"), ColorGradient::solid(Color::RED));
        drop_value(&mut f.session, &mut edits, &[id], &curve_row, &curve);
        assert_eq!(f.session.effect(id).unwrap().node.params.curve("LevelCurve"), Curve::flat(40.0));
    }

    #[test]
    fn depth_rows_go_blank_when_mixed_except_fire_and_wipe() {
        let mut f = fixture();
        let a = f.add("Chase");
        let b = f.add("Chase");
        f.session.set_param(b, "DepthOfEffect", ParamValue::Int(1));
        let m = panel_model(&f.session, &[a, b], DEFAULT_MULTI_EDIT_LIMIT);
        if has_row(&m, "DepthOfEffect") {
            let RowControl::Choice { selected, .. } = &row(&m, "DepthOfEffect").control else { panic!() };
            assert_eq!(*selected, None);
        }
        let w = descriptor("Wipe");
        let depth = w.params.iter().find(|d| d.name == "DepthOfEffect").unwrap();
        assert!(!blank_when_mixed(w, depth));
        let c = descriptor("Chase");
        assert!(blank_when_mixed(c, c.params.iter().find(|d| d.name == "DepthOfEffect").unwrap()));
    }

    #[test]
    fn a_hidden_enum_member_is_not_offered() {
        let mut f = fixture();
        let id = f.add("Picture");
        f.session.set_param(id, "Type", ParamValue::Enum("None".to_string()));
        f.session.set_param(id, "TilePictures", ParamValue::Enum("None".to_string()));
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        if has_row(&m, "TilePictures") {
            let RowControl::Choice { items, selected } = &row(&m, "TilePictures").control else { panic!() };
            assert!(items.iter().all(|i| i.value != ParamValue::Enum("None".to_string())));
            assert_eq!(*selected, None);
        }
    }

    #[test]
    fn a_stored_value_outside_the_range_shows_clamped_and_isnt_written() {
        let mut f = fixture();
        let id = f.add("Alternating");
        // Straight into the values: a saved file can hold this, and the setter would have corrected it.
        let mut params = f.session.effect(id).unwrap().node.params.clone();
        params.set("GroupLevel", ParamValue::Int(0));
        f.session.set_params(id, params);
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let RowControl::Int { text, .. } = &row(&m, "GroupLevel").control else { panic!() };
        assert_eq!(text, "1");
        assert_eq!(f.session.effect(id).unwrap().node.params.int("GroupLevel"), 0);
    }

    #[test]
    fn video_rows_follow_the_table() {
        let mut f = fixture();
        let id = f.add("Video");
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        let advanced = m.categories.iter().find(|c| c.name == "Advanced Settings").unwrap();
        assert!(advanced.starts_collapsed);
        assert!(m.categories.iter().filter(|c| c.name != "Advanced Settings").all(|c| !c.starts_collapsed));
    }

    #[test]
    fn morph_cant_add_to_an_empty_list() {
        let mut f = fixture();
        let id = f.add("Morph");
        let mut edits = PanelEdits::default();
        f.session.set_param(id, "MorphPolygonData", ParamValue::RecordList(Vec::new()));
        assert_eq!(list_add(&mut f.session, &mut edits, &[id], "MorphPolygonData"), CommitOutcome::NoChange);
    }

    #[test]
    fn an_edit_changes_the_render() {
        let mut f = fixture();
        let id = f.add("Pulse");
        let mut edits = PanelEdits::default();
        f.session.wait_for_renders();
        let before = f.session.effect(id).unwrap().intents.clone().unwrap();
        assert!(f.session.effect(id).unwrap().is_rendered());
        let outcome = commit(&mut f.session, &mut edits, &[id], &RowKey::top("LevelCurve"), ShownValue::Curve(Curve::flat(25.0)), CommitOptions::default());
        assert!(matches!(outcome, CommitOutcome::Committed { .. }));
        // Marked dirty at once; the render itself happens away from the caller.
        assert!(!f.session.effect(id).unwrap().is_rendered());
        f.session.wait_for_renders();
        let entry = f.session.effect(id).unwrap();
        assert!(entry.is_rendered());
        let after = entry.intents.clone().unwrap();
        assert!(!std::sync::Arc::ptr_eq(&before, &after));
    }

    #[test]
    fn choosing_a_file_copies_it_into_the_effects_folder() {
        let mut f = fixture();
        let id = f.add("Picture");
        let root = std::env::temp_dir().join(format!("vixen-adopt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("chosen.png");
        std::fs::write(&source, b"picture").unwrap();
        let mut edits = PanelEdits::new(Some(root.join("Module Data Files")));
        let desc = descriptor("Picture");
        let name = desc.params.iter().find(|d| matches!(d.editor, Editor::Path(_))).unwrap().name;
        let path = ShownValue::Path(source.to_string_lossy().into_owned());
        // The row can be hidden until the source is a file: the rule is the file's, not the row's.
        let stored = stored_value(desc, desc.params.iter().find(|d| d.name == name).unwrap(), Editor::Path("Image"), None, &desc.new_params(), &TargetInfo::detached(), &path, &edits, None);
        assert_eq!(stored, Some(ParamValue::Str("chosen.png".to_string())));
        assert_eq!(std::fs::read(root.join("Module Data Files/Picture/chosen.png")).unwrap(), b"picture");
        // A relative path is stored as given.
        let relative = ShownValue::Path("already.png".to_string());
        let stored = stored_value(desc, desc.params.iter().find(|d| d.name == name).unwrap(), Editor::Path("Image"), None, &desc.new_params(), &TargetInfo::detached(), &relative, &edits, None);
        assert_eq!(stored, Some(ParamValue::Str("already.png".to_string())));
        edits.module_data_root = None;
        let _ = (id, &mut f);
    }

    /// A chain of groups `depth` levels deep, ending in one element, with the target at its top.
    fn chain(tree: &mut ElementTree, name: &str, depth: usize) -> Uuid {
        let mut below = tree.add_element(&format!("{name} leaf"));
        for i in 1..depth {
            let group = tree.add_node(Uuid::new_v4(), &format!("{name} {i}"), None);
            tree.add_child(group, below);
            below = group;
        }
        tree.node(below).id
    }

    fn fixture_with_depths() -> (Fixture, Uuid, Uuid) {
        let mut tree = ElementTree::new();
        let three = chain(&mut tree, "Three", 3);
        let five = chain(&mut tree, "Five", 5);
        let target = three;
        let env = SessionEnvironment { video_cache: None,
            tree: Arc::new(tree),
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 50,
            media_dir: std::env::temp_dir(),
        };
        (Fixture { session: SequenceSession::new_untitled(&env), target }, three, five)
    }

    fn add_on(f: &mut Fixture, name: &str, target: Uuid) -> EffectId {
        f.session.add_new_effect(descriptor(name).type_id, target, "T", TimeSpan::ZERO, TimeSpan::from_ms(1000.0)).unwrap()
    }

    fn choice_labels(m: &PanelModel, param: &str) -> (Vec<String>, Option<u32>) {
        let RowControl::Choice { items, selected } = &row(m, param).control else { panic!("{param} isn't a choice") };
        (items.iter().map(|i| i.label.clone()).collect(), *selected)
    }

    fn depth_row_is_listed(m: &PanelModel) -> bool {
        has_row(m, "DepthOfEffect")
    }

    /// F105's depth lists are worked out from the rows directly, because whether a depth row shows at all is each effect's own rule.
    fn depth_items(f: &Fixture, ids: &[EffectId]) -> Vec<String> {
        let chosen = selected(&f.session, ids);
        let desc = chosen[0].descriptor.params.iter().find(|d| d.name == "DepthOfEffect").unwrap();
        let m = MergedParam { desc, category: desc.category, category_order: desc.category_order, order: desc.order, description: None, editor: desc.editor };
        merged_choices(&chosen, &m).into_iter().map(|i| i.label).collect()
    }

    #[test]
    fn test_49_a_depth_list_uses_the_smallest_depth_and_goes_blank_when_mixed() {
        let (mut f, three, five) = fixture_with_depths();
        let a = add_on(&mut f, "Chase", three);
        let b = add_on(&mut f, "Chase", five);
        assert_eq!(depth_items(&f, &[a, b]), ["0", "1", "2"]);
        assert_eq!(depth_items(&f, &[b]), ["0", "1", "2", "3", "4"]);
        f.session.set_param(a, "DepthOfEffect", ParamValue::Int(1));
        f.session.set_param(b, "DepthOfEffect", ParamValue::Int(2));
        let m = panel_model(&f.session, &[a, b], DEFAULT_MULTI_EDIT_LIMIT);
        if depth_row_is_listed(&m) {
            assert_eq!(choice_labels(&m, "DepthOfEffect").1, None);
        }
        // From blank, First's own value applies to all.
        let outcome = commit(&mut f.session, &mut PanelEdits::default(), &[a, b], &RowKey::top("DepthOfEffect"), ShownValue::Choice(1), CommitOptions::default());
        if depth_row_is_listed(&m) {
            assert!(matches!(outcome, CommitOutcome::Committed { .. }));
            assert_eq!(f.session.effect(b).unwrap().node.params.int("DepthOfEffect"), 1);
        }
    }

    #[test]
    fn test_50_fire_and_wipe_list_the_depths_between_and_show_firsts_value() {
        let (mut f, three, five) = fixture_with_depths();
        for name in ["Fire", "Wipe"] {
            let a = add_on(&mut f, name, three);
            let b = add_on(&mut f, name, five);
            assert_eq!(depth_items(&f, &[a, b]), ["1"], "{name}");
            let d = descriptor(name);
            assert!(!blank_when_mixed(d, d.params.iter().find(|p| p.name == "DepthOfEffect").unwrap()), "{name}");
        }
        // In a mix, First's converter decides: Wipe first lists 1..depth-1 and isn't blank, Chase first lists 0..depth and is.
        let wipe = add_on(&mut f, "Wipe", three);
        let chase = add_on(&mut f, "Chase", five);
        assert_eq!(depth_items(&f, &[wipe, chase]), ["1"]);
        assert_eq!(depth_items(&f, &[chase, wipe]), ["0", "1", "2"]);
    }

    #[test]
    fn test_4_rows_in_different_categories_go_under_misc_first() {
        let mut f = fixture();
        let pulse = f.add("Pulse");
        let spin = f.add("Spin");
        let m = panel_model(&f.session, &[pulse, spin], DEFAULT_MULTI_EDIT_LIMIT);
        assert_eq!(m.effect_name, "Pulse");
        let names: Vec<&str> = m.categories.iter().map(|c| c.name.as_str()).collect();
        // No name twice, whatever the order of the rows that fell in them.
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "{names:?}");
        if let Some(i) = names.iter().position(|n| *n == "Misc") {
            assert_eq!(m.categories[i].order, -1);
            assert_eq!(i, 0, "{names:?}");
        }
    }

    #[test]
    fn test_5_a_row_order_that_differs_falls_to_zero() {
        // Whatever the pair, a merged row's order is the shared one or 0, and rows sort by it and then by label.
        let mut f = fixture();
        let a = f.add("Chase");
        let b = f.add("Spin");
        let chosen = selected(&f.session, &[a, b]);
        let merged = merged_params(&chosen);
        assert!(!merged.is_empty());
        for m in &merged {
            let other = chosen[1].descriptor.params.iter().find(|d| d.name == m.desc.name).unwrap();
            let own = chosen[0].descriptor.params.iter().find(|d| d.name == m.desc.name).unwrap();
            assert_eq!(m.order, if own.order == other.order { own.order } else { 0 }, "{}", m.desc.name);
        }
    }

    #[test]
    fn test_47_add_when_every_list_is_empty_does_nothing() {
        let mut f = fixture();
        let a = f.add("Wave");
        let b = f.add("Wave");
        let mut edits = PanelEdits::default();
        f.session.set_param(a, "WaveformData", ParamValue::RecordList(Vec::new()));
        f.session.set_param(b, "WaveformData", ParamValue::RecordList(Vec::new()));
        assert_eq!(list_add(&mut f.session, &mut edits, &[a, b], "WaveformData"), CommitOutcome::NoChange);
        // One effect alone can: it knows its declared type.
        assert!(matches!(list_add(&mut f.session, &mut edits, &[a], "WaveformData"), CommitOutcome::Committed { .. }));
        // First empty and another not: it adds, and First's list goes to all.
        let c = f.add("Wave");
        f.session.set_param(c, "WaveformData", ParamValue::RecordList(Vec::new()));
        let d = f.add("Wave");
        let outcome = list_add(&mut f.session, &mut edits, &[c, d], "WaveformData");
        assert!(matches!(outcome, CommitOutcome::Committed { .. }));
        assert_eq!(f.session.effect(c).unwrap().node.params.records("WaveformData").len(), 1);
        assert_eq!(f.session.effect(d).unwrap().node.params.records("WaveformData").len(), 1);
    }

    #[test]
    fn test_57_a_pair_row_that_stays_the_same_is_no_step() {
        let mut f = fixture();
        let d = vixen_effect_registry::all().iter().copied().find(|d| d.params.iter().any(|p| matches!(p.kind, ParamKind::GradientLevels))).expect("an effect with pairs");
        let id = f.add(d.name);
        let param = d.params.iter().find(|p| matches!(p.kind, ParamKind::GradientLevels)).unwrap().name;
        let mut edits = PanelEdits::default();
        let ParamValue::GradientLevels(pairs) = f.session.effect(id).unwrap().node.params.get(param).clone() else { panic!() };
        let steps = f.session.undo_stack().undo_descriptions().len();
        let curve = commit(&mut f.session, &mut edits, &[id], &RowKey::sub(param, 0, "Curve"), ShownValue::Curve(pairs[0].curve.clone()), CommitOptions::default());
        let gradient = commit(&mut f.session, &mut edits, &[id], &RowKey::sub(param, 0, "ColorGradient"), ShownValue::Gradient(pairs[0].color_gradient.clone()), CommitOptions::default());
        assert_eq!(curve, CommitOutcome::NoChange);
        assert_eq!(gradient, CommitOutcome::NoChange);
        assert_eq!(f.session.undo_stack().undo_descriptions().len(), steps);
        // A real change is one step, on just that half.
        let outcome = commit(&mut f.session, &mut edits, &[id], &RowKey::sub(param, 0, "Curve"), ShownValue::Curve(Curve::flat(20.0)), CommitOptions::default());
        assert!(matches!(outcome, CommitOutcome::Committed { new_step: true, .. }));
        let ParamValue::GradientLevels(after) = f.session.effect(id).unwrap().node.params.get(param).clone() else { panic!() };
        assert_eq!(after[0].curve, Curve::flat(20.0));
        assert!(gradient_same_value(&after[0].color_gradient, &pairs[0].color_gradient));
    }

    #[test]
    fn test_46_the_hidden_tile_stays_out_of_the_list_and_keeps_its_value() {
        let mut f = fixture();
        let id = f.add("Picture");
        let d = descriptor("Picture");
        let tile = d.params.iter().find(|p| p.name == "TilePictures").unwrap();
        let entry = f.session.effect(id).unwrap();
        let info = f.session.target_info(id).unwrap();
        let items = selection_items(d, tile, None, &entry.node.params, &info);
        assert_eq!(items.len(), 12);
        assert_eq!(items[0].label, "Blue Glow Dots");
        assert!(items.iter().all(|i| i.value != ParamValue::Enum("None".to_string())));
        f.session.set_param(id, "TilePictures", ParamValue::Enum("None".to_string()));
        assert_eq!(f.session.effect(id).unwrap().node.params.enum_name("TilePictures"), "None");
    }

    #[test]
    fn test_53_bool_words_that_differ_fall_back_to_true_and_false() {
        // Two effects with one `Bool` name and different words. Found from the tables, so a new effect can't be missed.
        let all = vixen_effect_registry::all();
        let mut f = fixture();
        let mut tried = false;
        for a in all {
            for pa in a.params.iter().filter(|p| matches!(p.kind, ParamKind::Bool) && p.bool_labels.is_some()) {
                for b in all.iter().filter(|b| b.type_id != a.type_id) {
                    if let Some(pb) = b.params.iter().find(|p| p.name == pa.name && matches!(p.kind, ParamKind::Bool)) {
                        if pb.bool_labels != pa.bool_labels {
                            let x = f.add(a.name);
                            let y = f.add(b.name);
                            let chosen = selected(&f.session, &[x, y]);
                            let m = MergedParam { desc: pa, category: pa.category, category_order: pa.category_order, order: pa.order, description: None, editor: pa.editor };
                            let items = merged_choices(&chosen, &m);
                            assert_eq!(items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(), ["True", "False"], "{} {}", a.name, pa.name);
                            tried = true;
                        }
                    }
                }
            }
        }
        // The same words stay, and one effect keeps its own.
        for a in all {
            if let Some(pa) = a.params.iter().find(|p| matches!(p.kind, ParamKind::Bool) && p.bool_labels.is_some()) {
                let x = f.add(a.name);
                let chosen = selected(&f.session, &[x]);
                let m = MergedParam { desc: pa, category: pa.category, category_order: pa.category_order, order: pa.order, description: None, editor: pa.editor };
                let (yes, no) = pa.bool_labels.unwrap();
                assert_eq!(merged_choices(&chosen, &m).iter().map(|i| i.label.as_str()).collect::<Vec<_>>(), [yes, no]);
                tried = true;
            }
        }
        assert!(tried);
    }

    #[test]
    fn every_category_appears_once_for_any_two_effects() {
        // A category twice would give the panel two sections with one name.
        let mut f = fixture();
        let all = vixen_effect_registry::all();
        let ids: Vec<EffectId> = all.iter().map(|d| f.session.add_new_effect(d.type_id, f.target, "Arch", TimeSpan::ZERO, TimeSpan::from_ms(1000.0)).unwrap()).collect();
        for a in &ids {
            let m = panel_model(&f.session, &[*a], DEFAULT_MULTI_EDIT_LIMIT);
            let mut names: Vec<&str> = m.categories.iter().map(|c| c.name.as_str()).collect();
            let count = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), count, "{}", m.effect_name);
        }
        for pair in ids.chunks(2) {
            let m = panel_model(&f.session, pair, DEFAULT_MULTI_EDIT_LIMIT);
            let mut names: Vec<&str> = m.categories.iter().map(|c| c.name.as_str()).collect();
            let count = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), count, "{}", m.effect_name);
        }
    }

    #[test]
    fn a_shared_property_goes_through_each_effects_own_display() {
        // F61. Liquid's Range is shown as `20 - stored`, and Spirograph's is stored as shown.
        let mut f = fixture();
        let liquid = f.add("Liquid");
        let spirograph = f.add("Spirograph");
        let mut edits = PanelEdits::default();
        f.session.set_param(liquid, "Range", ParamValue::Int(4));
        // Spirograph's Range isn't browsable, so this pair never shares a row today; the rule is for whichever pair does.
        let outcome = apply_to_all(&mut f.session, &mut edits, &[liquid, spirograph], "Range", "Zoom", ParamValue::Int(5), CommitOptions::default(), false);
        assert!(matches!(outcome, CommitOutcome::Committed { .. }));
        assert_eq!(f.session.effect(liquid).unwrap().node.params.int("Range"), 5);
        assert_eq!(f.session.effect(spirograph).unwrap().node.params.int("Range"), 15);
    }

    #[test]
    fn a_row_with_words_is_always_a_choice() {
        // F23: the words only reach the user through a drop-down.
        for d in vixen_effect_registry::all() {
            for p in d.params.iter().filter(|p| matches!(p.kind, ParamKind::Bool) && p.bool_labels.is_some()) {
                assert!(matches!(p.editor, Editor::Selection), "{} {}", d.name, p.name);
            }
        }
    }

    /// A member of a record list's rows, from the effect's own table.
    fn record_param(effect: &str, list: &str, name: &str) -> &'static ParamDesc {
        let d = descriptor(effect);
        let ParamKind::RecordList(record) = d.params.iter().find(|p| p.name == list).unwrap().kind else { panic!("{list} isn't a record list") };
        record.params.iter().find(|p| p.name == name).unwrap()
    }

    fn choice_index(record_param: &ParamDesc, member: &str) -> u32 {
        let ParamKind::Enum(e) = record_param.kind else { panic!("{} isn't an enum", record_param.name) };
        e.members.iter().position(|m| m.name == member).unwrap_or_else(|| panic!("no {member}")) as u32
    }

    #[test]
    fn test_35_drops_on_a_discrete_target_follow_its_colours() {
        let mut tree = ElementTree::new();
        let n = tree.add_element("Red and green");
        tree.color_sets.insert("RG".to_string(), vec![Color::RED, Color::GREEN]);
        tree.node_mut(n).properties.push(vixen_core::PropertyInstance {
            type_id: vixen_core::element::property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: vixen_core::PropertyData::Color {
                color_type: vixen_core::ElementColorType::MultipleDiscreteColors,
                single_color: Color::WHITE,
                color_set_name: Some("RG".to_string()),
            },
        });
        let target = tree.node(n).id;
        let env = SessionEnvironment { video_cache: None, tree: Arc::new(tree), store: Arc::new(ModuleStore::new()), module_data_root: None, frame_ms: 50, media_dir: std::env::temp_dir() };
        let mut f = Fixture { session: SequenceSession::new_untitled(&env), target };
        let id = f.add("Pulse");
        let gradient_row = RowKey::top("ColorGradient");
        assert!(can_drop(&f.session, &[id], &gradient_row, &DragPayload::Color(Color::RED)));
        assert!(!can_drop(&f.session, &[id], &gradient_row, &DragPayload::Color(Color::BLUE)));
        let mixed = ColorGradient::from_parts(
            vec![vixen_core::ColorPoint::new(Color::RED, 0.0), vixen_core::ColorPoint::new(Color::BLUE, 1.0)],
            vec![vixen_core::AlphaPoint::new(255, 0.0), vixen_core::AlphaPoint::new(255, 1.0)],
        );
        assert!(!can_drop(&f.session, &[id], &gradient_row, &DragPayload::Gradient(mixed)));
        assert!(can_drop(&f.session, &[id], &gradient_row, &DragPayload::Gradient(ColorGradient::solid(Color::GREEN))));
        // The panel says so too: the strip is discrete, with those colours.
        let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
        assert!(matches!(row(&m, "ColorGradient").control, RowControl::Gradient { discrete: true, .. }));
        assert_eq!(m.valid_colors, vec![Color::RED, Color::GREEN]);
    }

    #[test]
    fn test_36_37_a_library_change_reaches_linked_effects_and_a_removal_unlinks_them() {
        let mut f = fixture();
        let id = f.add("Pulse");
        let library = |points: Vec<(f64, f64)>| crate::library_links::Libraries {
            curves: vixen_io::CurveLibrary { curves: vec![("Ramp".to_string(), Curve::from_points(points))] },
            gradients: Default::default(),
        };
        f.session.libraries_changed(library(vec![(0.0, 0.0), (100.0, 100.0)]));
        let mut linked = Curve::from_points(vec![(0.0, 0.0), (100.0, 100.0)]);
        linked.library_reference_name = "Ramp".to_string();
        f.session.set_param(id, "LevelCurve", ParamValue::Curve(linked));
        f.session.wait_for_renders();
        assert!(f.session.effect(id).unwrap().node.params.curve("LevelCurve").is_library_reference());
        let generation = f.session.effect(id).unwrap().generation;
        // The entry is edited: the linked effect takes its points and renders again.
        let entry = Curve::from_points(vec![(0.0, 10.0), (100.0, 90.0)]);
        f.session.libraries_changed(library(entry.points.clone()));
        let curve = f.session.effect(id).unwrap().node.params.curve("LevelCurve");
        assert_eq!(curve.points, entry.points);
        assert_eq!(curve.library_reference_name, "Ramp");
        assert!(f.session.effect(id).unwrap().generation > generation, "marked for a new render");
        // The entry goes: the effect keeps its points and loses the link.
        f.session.libraries_changed(crate::library_links::Libraries::default());
        let curve = f.session.effect(id).unwrap().node.params.curve("LevelCurve");
        assert_eq!(curve.points, entry.points);
        assert!(!curve.is_library_reference());
    }

    #[test]
    fn test_54_a_whirl_row_becoming_a_meteor_takes_the_single_colour_mode() {
        let mut f = fixture();
        let id = f.add("Whirlpool");
        let mut edits = PanelEdits::default();
        let mode = record_param("Whirlpool", "WhirlData", "WhirlMode");
        // The list shows for individual configuration of several panels.
        f.session.set_param(id, "Rows", ParamValue::Int(2));
        f.session.set_param(id, "IndividualConfiguration", ParamValue::Bool(true));
        let mut rows = f.session.effect(id).unwrap().node.params.records("WhirlData").to_vec();
        rows[0].set("ColorMode", ParamValue::Enum("Bands".to_string()));
        rows[0].set("WhirlMode", ParamValue::Enum("RecurrentWhirls".to_string()));
        assert!(rows.len() >= 2, "two panels, two whirls");
        rows[1] = rows[0].clone();
        rows[1].set("ColorMode", ParamValue::Enum("LegColors".to_string()));
        f.session.set_param(id, "WhirlData", ParamValue::RecordList(rows));
        assert!(has_row(&panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT), "WhirlData"));
        let steps = f.session.undo_stack().undo_descriptions().len();
        let outcome = commit(&mut f.session, &mut edits, &[id], &RowKey::sub("WhirlData", 0, "WhirlMode"), ShownValue::Choice(choice_index(mode, "Meteor")), CommitOptions::default());
        assert!(matches!(outcome, CommitOutcome::Committed { new_step: true, .. }));
        let after = f.session.effect(id).unwrap().node.params.records("WhirlData").to_vec();
        assert_eq!(after[0].enum_name("WhirlMode"), "Meteor");
        assert_eq!(after[0].enum_name("ColorMode"), "GradientOverTime");
        assert_eq!(after[1].enum_name("ColorMode"), "LegColors", "other rows are left alone");
        assert_eq!(f.session.undo_stack().undo_descriptions().len(), steps + 1);
    }

    #[test]
    fn test_56_a_row_edit_and_its_setters_side_effect_are_one_undo_step() {
        let mut f = fixture();
        let id = f.add("Wave");
        let mut edits = PanelEdits::default();
        let movement = record_param("Wave", "WaveformData", "MovementType");
        let mut rows = f.session.effect(id).unwrap().node.params.records("WaveformData").to_vec();
        rows[0].set("MovementType", ParamValue::Enum(match movement.kind { ParamKind::Enum(e) => e.members.iter().find(|m| m.name != "Snake").unwrap().name.to_string(), _ => unreachable!() }));
        rows[0].set("PrimeWave", ParamValue::Bool(false));
        f.session.set_param(id, "WaveformData", ParamValue::RecordList(rows));
        let before = f.session.effect(id).unwrap().node.params.records("WaveformData")[0].clone();
        let steps = f.session.undo_stack().undo_descriptions().len();
        commit(&mut f.session, &mut edits, &[id], &RowKey::sub("WaveformData", 0, "MovementType"), ShownValue::Choice(choice_index(movement, "Snake")), CommitOptions::default());
        let after = f.session.effect(id).unwrap().node.params.records("WaveformData")[0].clone();
        assert_eq!(after.enum_name("MovementType"), "Snake");
        assert!(after.boolean("PrimeWave"), "the setter primed the wave");
        assert_eq!(f.session.undo_stack().undo_descriptions().len(), steps + 1);
        // Undo restores both, the setter's change too.
        f.session.undo(1);
        let undone = f.session.effect(id).unwrap().node.params.records("WaveformData")[0].clone();
        assert_eq!(undone.enum_name("MovementType"), before.enum_name("MovementType"));
        assert!(!undone.boolean("PrimeWave"));
    }

    #[test]
    fn every_effect_builds_rows_and_every_row_can_be_read() {
        let mut f = fixture();
        for d in vixen_effect_registry::all() {
            let id = f.session.add_new_effect(d.type_id, f.target, "Arch", TimeSpan::ZERO, TimeSpan::from_ms(1000.0)).unwrap();
            let m = panel_model(&f.session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
            assert_eq!(m.effect_name, d.name);
            for category in &m.categories {
                assert!(!category.rows.is_empty(), "{}", d.name);
            }
        }
    }
}
