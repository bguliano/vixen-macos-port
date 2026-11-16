//! Drag and drop onto the grid, section I of the spec: `IsValidDataObject`, `HandleColorDrop`, `HandleCurveDrop`, `HandleGradientDrop` and `HandleFileDrop`.

use crate::sequence::{EffectId, SequenceSession};
use crate::sequencer::effect_props::{collect_props, contains_types, set_nested, EffectProp, PropType, ALL_COLOR_TYPES, COLOR_TYPES, CURVE_TYPES, GRADIENT_TYPES};
use crate::sequencer::keys::Mods;
use crate::sequencer::state::{Alert, AlertAction, AlertConfirm, AlertStyle, SequencerState};
use crate::undo::UndoAction;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_core::{Color, ColorGradient, Curve, GradientLevelPair, TimeSpan};
use vixen_effects::framework::{EffectDescriptor, ParamValue, Params};

/// What's being dragged.
#[derive(Clone, PartialEq, Debug)]
pub enum DropValue {
    Color(Color),
    Gradient(ColorGradient),
    Curve(Curve),
}

impl DropValue {
    fn word(&self) -> &'static str {
        match self {
            DropValue::Color(_) => "Color",
            DropValue::Gradient(_) => "Gradient",
            DropValue::Curve(_) => "Curve",
        }
    }
}

/// `GetDiscreteColors`: an effect's targets' valid colours. Empty means anything goes.
fn discrete_colors(session: &SequenceSession, id: EffectId) -> Vec<Color> {
    session.target_info(id).map_or_else(Vec::new, |info| info.valid_colors())
}

fn same_color(a: Color, b: Color) -> bool {
    a.r == b.r && a.g == b.g && a.b == b.b
}

/// `IsValidDataObject`'s answers for a value over an effect. F306.
pub fn value_accepted(session: &SequenceSession, id: EffectId, value: &DropValue) -> bool {
    let Some(entry) = session.effect(id) else { return false };
    let Some(descriptor) = entry.descriptor else { return false };
    let Some(info) = session.target_info(id) else { return false };
    let params = &entry.node.params;
    match value {
        DropValue::Color(color) => {
            if !contains_types(descriptor, params, &info, ALL_COLOR_TYPES) {
                return false;
            }
            let discrete = discrete_colors(session, id);
            discrete.is_empty() || discrete.iter().any(|c| same_color(*c, *color))
        }
        DropValue::Gradient(gradient) => {
            if !contains_types(descriptor, params, &info, GRADIENT_TYPES) {
                return false;
            }
            let discrete = discrete_colors(session, id);
            discrete.is_empty() || gradient.colors().iter().all(|p| discrete.iter().any(|c| same_color(*c, p.rgb_color())))
        }
        DropValue::Curve(_) => contains_types(descriptor, params, &info, CURVE_TYPES),
    }
}

/// One choice in the parameter picker: F312. What to draw for it is in `image`.
#[derive(Clone, PartialEq, Debug)]
pub struct PickerChoice {
    pub label: String,
    pub group: Option<String>,
    pub prop: EffectProp,
    /// The item in a list, for a list property.
    pub index: Option<usize>,
    pub image: PickerImage,
}

/// The thumbnail each picker item shows: F310.
#[derive(Clone, PartialEq, Debug)]
pub enum PickerImage {
    Color(Color),
    Gradient(ColorGradient),
    Curve(Curve),
    /// An effect's own picture, by its resource name.
    Effect(String),
}

/// `CreateGradientListPickerControls`: `<property> n`, from 1.
fn gradient_list_choices(prop: &EffectProp, list: &[ColorGradient]) -> Vec<PickerChoice> {
    list.iter()
        .enumerate()
        .map(|(i, g)| PickerChoice {
            label: format!("{} {}", prop.label, i + 1),
            group: None,
            prop: prop.clone(),
            index: Some(i),
            image: PickerImage::Gradient(g.clone()),
        })
        .collect()
}

/// `CreateGradientLevelPairPickerControls`: `Gradient n` or `Curve n`, grouped by the property.
fn level_pair_choices(prop: &EffectProp, list: &[GradientLevelPair], gradients: bool) -> Vec<PickerChoice> {
    list.iter()
        .enumerate()
        .map(|(i, pair)| PickerChoice {
            label: format!("{} {}", if gradients { "Gradient" } else { "Curve" }, i + 1),
            group: Some(prop.label.to_string()),
            prop: prop.clone(),
            index: Some(i),
            image: if gradients { PickerImage::Gradient(pair.color_gradient.clone()) } else { PickerImage::Curve(pair.curve.clone()) },
        })
        .collect()
}

fn direct_choice(prop: &EffectProp, params: &Params) -> Option<PickerChoice> {
    let value = prop.value(params)?;
    let image = match value {
        ParamValue::Color(c) => PickerImage::Color(*c),
        ParamValue::Rgb(c) => PickerImage::Color(c.to_color()),
        ParamValue::Gradient(g) => PickerImage::Gradient(g.clone()),
        ParamValue::Curve(c) => PickerImage::Curve(c.clone()),
        _ => return None,
    };
    Some(PickerChoice { label: prop.label.to_string(), group: None, prop: prop.clone(), index: None, image })
}

/// What dropping on an effect needs next.
#[derive(Clone, PartialEq, Debug)]
pub enum DropPlan {
    /// Nothing to do: no property takes it, or the effect has gone.
    Nothing,
    /// Goes straight onto this property, on every effect.
    Direct(PickerChoice),
    /// The picker: status text "Choose the property to set, press Escape to cancel." for 8 s.
    Picker(Vec<PickerChoice>),
}

/// `HandleColorDropOnElements`, `HandleCurveDrop` and `HandleGradientDropOnElements`' choice, for the first effect of the set. A colour with no plain Color property becomes a gradient of that one colour.
pub fn plan_for_effect(descriptor: &EffectDescriptor, params: &Params, info: &vixen_effects::framework::TargetInfo, value: &DropValue) -> (DropPlan, DropValue) {
    match value {
        DropValue::Color(color) => {
            let mut props = collect_props(descriptor, params, info, COLOR_TYPES);
            if props.is_empty() {
                let gradient = DropValue::Gradient(ColorGradient::solid(*color));
                return plan_for_effect(descriptor, params, info, &gradient);
            }
            props.extend(collect_props(descriptor, params, info, GRADIENT_TYPES));
            if props.len() == 1 {
                let plan = direct_choice(&props[0], params).map_or(DropPlan::Nothing, DropPlan::Direct);
                return (plan, value.clone());
            }
            (DropPlan::Picker(gradient_family_choices(&props, params)), value.clone())
        }
        DropValue::Gradient(_) => {
            let props = collect_props(descriptor, params, info, GRADIENT_TYPES);
            if props.is_empty() {
                return (DropPlan::Nothing, value.clone());
            }
            if props.len() == 1 && props[0].kind == PropType::Gradient {
                let plan = direct_choice(&props[0], params).map_or(DropPlan::Nothing, DropPlan::Direct);
                return (plan, value.clone());
            }
            (choose(gradient_family_choices(&props, params)), value.clone())
        }
        DropValue::Curve(_) => {
            let props = collect_props(descriptor, params, info, CURVE_TYPES);
            if props.is_empty() {
                return (DropPlan::Nothing, value.clone());
            }
            if props.len() == 1 && props[0].kind == PropType::Curve {
                let plan = direct_choice(&props[0], params).map_or(DropPlan::Nothing, DropPlan::Direct);
                return (plan, value.clone());
            }
            let mut choices: Vec<PickerChoice> = props.iter().filter(|p| p.kind == PropType::Curve).filter_map(|p| direct_choice(p, params)).collect();
            for prop in props.iter().filter(|p| p.kind == PropType::GradientLevels) {
                if let Some(ParamValue::GradientLevels(list)) = prop.value(params) {
                    choices.extend(level_pair_choices(prop, list, false));
                }
            }
            (choose(choices), value.clone())
        }
    }
}

/// One control means no picker.
fn choose(mut choices: Vec<PickerChoice>) -> DropPlan {
    match choices.len() {
        0 => DropPlan::Nothing,
        1 => DropPlan::Direct(choices.remove(0)),
        _ => DropPlan::Picker(choices),
    }
}

fn gradient_family_choices(props: &[EffectProp], params: &Params) -> Vec<PickerChoice> {
    let mut choices: Vec<PickerChoice> =
        props.iter().filter(|p| matches!(p.kind, PropType::Color | PropType::Gradient)).filter_map(|p| direct_choice(p, params)).collect();
    for prop in props.iter().filter(|p| p.kind == PropType::GradientList) {
        if let Some(ParamValue::GradientList(list)) = prop.value(params) {
            choices.extend(gradient_list_choices(prop, list));
        }
    }
    for prop in props.iter().filter(|p| p.kind == PropType::GradientLevels) {
        if let Some(ParamValue::GradientLevels(list)) = prop.value(params) {
            choices.extend(level_pair_choices(prop, list, true));
        }
    }
    choices
}

/// The new value of a property, given what's on it now. `None` when the item's index doesn't exist on this effect: it's skipped.
fn new_value(current: &ParamValue, index: Option<usize>, value: &DropValue) -> Option<ParamValue> {
    match (current, value) {
        (ParamValue::Color(_), DropValue::Color(c)) => Some(ParamValue::Color(*c)),
        (ParamValue::Rgb(_), DropValue::Color(c)) => Some(ParamValue::Rgb(vixen_core::color::Rgb::from_color(*c))),
        (ParamValue::Gradient(_), DropValue::Color(c)) => Some(ParamValue::Gradient(ColorGradient::solid(*c))),
        (ParamValue::Gradient(_), DropValue::Gradient(g)) => Some(ParamValue::Gradient(g.clone())),
        (ParamValue::Curve(_), DropValue::Curve(c)) => Some(ParamValue::Curve(c.clone())),
        (ParamValue::GradientList(list), DropValue::Gradient(_) | DropValue::Color(_)) => {
            let at = index?;
            if at >= list.len() {
                return None;
            }
            let mut list = list.clone();
            list[at] = match value {
                DropValue::Color(c) => ColorGradient::solid(*c),
                DropValue::Gradient(g) => g.clone(),
                DropValue::Curve(_) => return None,
            };
            Some(ParamValue::GradientList(list))
        }
        (ParamValue::GradientLevels(list), _) => {
            let at = index?;
            if at >= list.len() {
                return None;
            }
            let mut list = list.clone();
            match value {
                // Replacing a level pair's gradient keeps its curve, and the other way round.
                DropValue::Color(c) => list[at].color_gradient = ColorGradient::solid(*c),
                DropValue::Gradient(g) => list[at].color_gradient = g.clone(),
                DropValue::Curve(c) => list[at].curve = c.clone(),
            }
            Some(ParamValue::GradientLevels(list))
        }
        _ => None,
    }
}

/// What `plan_drop` found.
#[derive(Clone, PartialEq, Debug)]
pub struct DropRequest {
    /// `ShowMultipleEffectDropMessage`, when the selection holds other types: Cancel gives up.
    pub warning: Option<Alert>,
    pub plan: DropPlan,
    /// The value to apply, which a colour with nowhere to go has turned into a gradient.
    pub value: DropValue,
    pub elements: Vec<EffectId>,
}

/// One file's effect. F311.
#[derive(Clone, PartialEq, Debug)]
pub enum FileDropItem {
    /// The file's extension belongs to no effect: skipped, though the sequence counter still moves on.
    Skip,
    Effect { path: PathBuf, type_id: Uuid, name: &'static str, index_in_sequence: i32 },
    /// More than one effect takes it: ask. No two do today.
    Ask { path: PathBuf, choices: Vec<Uuid>, index_in_sequence: i32 },
}

/// The extensions each effect takes: `SupportedFileExtensions`. Video's list has `.MTS`, which never matches because the dropped extension is lower-cased (KEEP F311).
pub fn supported_extensions(name: &str) -> &'static [&'static str] {
    match name {
        "Video" => &[".mp4", ".avi", ".mov", ".MTS"],
        "Picture" => &[".jpg", ".jpeg", ".png", ".gif", ".bmp"],
        "Shapes" => &[".svg"],
        "Glediator" => &[".gled"],
        _ => &[],
    }
}

/// `HandleFileDrop`'s walk. `chosen` holds what was picked earlier in this drop, for files with several candidates. Non-sequential placement resets the index, so every file lands at the drop time.
pub fn plan_file_drop(files: &[PathBuf], mods: Mods, chosen: &[Uuid]) -> (Vec<FileDropItem>, bool) {
    let sequential = (mods.cmd && !mods.shift && !mods.opt) || (mods.cmd && mods.opt && !mods.shift);
    let horizontal = (mods.opt && !mods.shift && !mods.cmd) || (mods.opt && mods.cmd && !mods.shift);
    let mut items = Vec::with_capacity(files.len());
    let mut i = 0;
    for file in files {
        let extension = file.extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase())).unwrap_or_default();
        let candidates: Vec<&'static EffectDescriptor> =
            vixen_effect_registry::all().iter().copied().filter(|d| supported_extensions(d.name).contains(&extension.as_str())).collect();
        if candidates.is_empty() {
            items.push(FileDropItem::Skip);
            i += 1;
            continue;
        }
        if !sequential {
            i = 0;
        }
        if candidates.len() == 1 {
            items.push(FileDropItem::Effect { path: file.clone(), type_id: candidates[0].type_id, name: candidates[0].name, index_in_sequence: i });
        } else if let Some(pick) = candidates.iter().find(|d| chosen.contains(&d.type_id)) {
            items.push(FileDropItem::Effect { path: file.clone(), type_id: pick.type_id, name: pick.name, index_in_sequence: i });
        } else {
            items.push(FileDropItem::Ask { path: file.clone(), choices: candidates.iter().map(|d| d.type_id).collect(), index_in_sequence: i });
        }
        i += 1;
    }
    (items, horizontal)
}

impl SequencerState {
    /// `GetElementsForDrop`: the selected effects of the same type if the target's selected, else just the target.
    fn elements_for_drop(&self, session: &SequenceSession, element: EffectId) -> Vec<EffectId> {
        let selected = self.grid.selected_elements();
        if !selected.contains(&element) {
            return vec![element];
        }
        let Some(type_id) = session.effect(element).map(|e| e.node.type_id) else { return vec![element] };
        // The target goes first, because upstream reads the properties off the first of the set and it's always its type.
        selected.into_iter().filter(|id| session.effect(*id).is_some_and(|e| e.node.type_id == type_id)).collect()
    }

    /// F306 for a value over an effect.
    pub fn drop_accepted(&self, element: EffectId, value: &DropValue) -> bool {
        value_accepted(&self.session.lock(), element, value)
    }

    /// F307 to F309's first half.
    pub fn plan_drop(&self, element: EffectId, value: &DropValue) -> Option<DropRequest> {
        let session = self.session.lock();
        let elements = self.elements_for_drop(&session, element);
        let entry = session.effect(element)?;
        let descriptor = entry.descriptor?;
        let info = session.target_info(element)?;
        let selected = self.grid.selected_elements();
        let warning = (selected.len() > 1 && elements.len() != selected.len()).then(|| Alert {
            title: "Multiple Type Effects".to_string(),
            message: format!("Multiple type effects selected, this will only apply to effects of the type: {}", descriptor.name),
            style: AlertStyle::Warning,
            confirm: Some(AlertConfirm { label: "OK".to_string(), cancel_label: "Cancel".to_string(), action: AlertAction::ContinueDrop }),
        });
        let (plan, value) = plan_for_effect(descriptor, &entry.node.params, &info, value);
        Some(DropRequest { warning, plan, value, elements })
    }

    /// The second half: every effect gets the value at the chosen property, one undo step (F412), and the status line says how it went. Effects with too few items are skipped.
    /// The upstream crash on lists with 2+ effects is gone because each effect's own value is used.
    pub fn apply_drop(&mut self, choice: &PickerChoice, value: &DropValue, elements: &[EffectId], now: std::time::Instant) {
        let session = std::sync::Arc::clone(&self.session);
        let mut session = session.lock();
        let mut before: Vec<(EffectId, Params)> = Vec::new();
        for id in elements {
            let Some(entry) = session.effect(*id) else { continue };
            let Some(current) = choice.prop.value(&entry.node.params) else { continue };
            let Some(updated) = new_value(current, choice.index, value) else { continue };
            before.push((*id, entry.node.params.clone()));
            if choice.prop.owner.is_some() {
                let mut params = entry.node.params.clone();
                if set_nested(&mut params, &choice.prop, updated) {
                    session.set_params(*id, params);
                }
            } else {
                session.set_param(*id, choice.prop.name, updated);
            }
        }
        if before.is_empty() {
            return;
        }
        let name = session.effect(before[0].0).and_then(|e| e.descriptor).map_or("", |d| d.name);
        let text = format!("{} applied to {} {} effect(s).", value.word(), before.len(), name);
        session.record_undo(UndoAction::ParamsChanged { label: choice.prop.label.to_string(), params: before });
        session.render_all_dirty();
        drop(session);
        self.set_status_message(&text, 30, now);
    }

    /// `HandleFileDrop` for one effect: defaults, the file on its `FileName`, `PictureSource` and `ShapeList` to File, and each with its own undo step. No Deprecated check (KEEP).
    pub fn add_file_effect(&mut self, type_id: Uuid, path: &Path, horizontal: bool, row: u32, time: TimeSpan, index: i32) -> Option<EffectId> {
        // `TimeSpan.FromTicks(20000000 * i)`: 2 s each.
        let start = time + TimeSpan(20_000_000 * index as i64);
        let id = self.add_effect_at(type_id, row, start)?;
        let mut session = self.session.lock();
        let entry = session.effect(id)?;
        let names: Vec<(&'static str, ParamValue)> = entry
            .descriptor?
            .params
            .iter()
            .filter_map(|p| match (p.name, &p.kind) {
                ("FileName", _) => Some((p.name, ParamValue::Str(path.to_string_lossy().into_owned()))),
                ("Source", vixen_effects::framework::ParamKind::Enum(e)) if e.members.iter().any(|m| m.name == "File") => Some((p.name, ParamValue::Enum("File".to_string()))),
                ("ShapeList", _) => Some((p.name, ParamValue::Enum("File".to_string()))),
                ("StringOrientation", _) if horizontal => Some((p.name, ParamValue::Enum("Horizontal".to_string()))),
                _ => None,
            })
            .collect();
        for (name, value) in names {
            session.set_param(id, name, value);
        }
        session.render_all_dirty();
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppCore;
    use crate::sequence::SessionEnvironment;
    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::time::Instant;
    use vixen_core::ElementTree;
    use vixen_io::{ModuleStore, ProfileSettings};

    const SET_LEVEL: Uuid = uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0");

    fn id_of(name: &str) -> Uuid {
        vixen_effect_registry::all().iter().find(|d| d.name == name).unwrap().type_id
    }

    fn s(secs: f64) -> TimeSpan {
        TimeSpan::from_secs(secs)
    }

    fn fixture(name: &str) -> SequencerState {
        let root = std::env::temp_dir().join(format!("vixen-drop-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let core = Arc::new(AppCore::open(root.clone(), true).unwrap());
        let mut tree = ElementTree::new();
        tree.add_element("A");
        tree.add_element("B");
        let env = SessionEnvironment { video_cache: None,
            tree: Arc::new(tree),
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 25,
            media_dir: core.paths.media_dir(),
        };
        let session = Arc::new(Mutex::new(SequenceSession::new_untitled(&env)));
        let settings = Arc::new(ProfileSettings::open(root.join("Settings.xml")));
        let mut state = SequencerState::with_settings(core, session, settings, root);
        state.finish_load();
        state
    }

    #[test]
    fn set_level_takes_a_colour_straight_on() {
        let mut state = fixture("color");
        let a = state.add_effect_at(SET_LEVEL, 0, s(1.0)).unwrap();
        let red = DropValue::Color(Color::rgb(255, 0, 0));
        assert!(state.drop_accepted(a, &red));
        let request = state.plan_drop(a, &red).unwrap();
        assert!(request.warning.is_none());
        let DropPlan::Direct(choice) = request.plan else { panic!("expected direct") };
        state.apply_drop(&choice, &request.value, &request.elements, Instant::now());
        let ui = state.ui_state();
        assert_eq!(ui.status_message, "Color applied to 1 Set Level effect(s).");
        assert!(ui.undo_name.unwrap().starts_with("1 Effect(s) "));
        state.undo(1);
        assert!(!state.session.lock().effect(a).is_none());
    }

    #[test]
    fn a_gradient_on_set_level_is_refused_and_a_colour_on_a_gradient_effect_becomes_one() {
        let mut state = fixture("grad");
        let a = state.add_effect_at(SET_LEVEL, 0, s(1.0)).unwrap();
        assert!(!state.drop_accepted(a, &DropValue::Gradient(ColorGradient::solid(Color::rgb(1, 2, 3)))));
        assert!(!state.drop_accepted(a, &DropValue::Curve(Curve::default())) || state.plan_drop(a, &DropValue::Curve(Curve::default())).is_some());

        let pulse = id_of("Pulse");
        let b = state.add_effect_at(pulse, 1, s(4.0)).unwrap();
        let red = DropValue::Color(Color::rgb(255, 0, 0));
        let request = state.plan_drop(b, &red).unwrap();
        assert!(matches!(request.value, DropValue::Gradient(_)), "no plain Color property, so it's a gradient of the colour");
        let DropPlan::Direct(choice) = request.plan else { panic!("expected direct") };
        state.apply_drop(&choice, &request.value, &request.elements, Instant::now());
        assert_eq!(state.ui_state().status_message, "Gradient applied to 1 Pulse effect(s).");
    }

    #[test]
    fn several_properties_go_to_the_picker() {
        let mut state = fixture("pick");
        let a = state.add_effect_at(id_of("Bars"), 0, s(1.0)).unwrap();
        let gradient = DropValue::Gradient(ColorGradient::solid(Color::rgb(0, 0, 255)));
        let request = state.plan_drop(a, &gradient).unwrap();
        let DropPlan::Picker(choices) = request.plan else { panic!("expected the picker") };
        assert!(choices.len() > 1);
        // A plain gradient list labels its items `<property> n`.
        assert_eq!(choices[0].label, "Gradients 1");
        assert_eq!(choices[1].label, "Gradients 2");
        assert!(choices[0].group.is_none());
    }

    #[test]
    fn a_level_pair_keeps_its_other_half() {
        let mut state = fixture("pairs");
        let a = state.add_effect_at(id_of("Alternating"), 0, s(1.0)).unwrap();
        let gradient = DropValue::Gradient(ColorGradient::solid(Color::rgb(0, 0, 255)));
        let request = state.plan_drop(a, &gradient).unwrap();
        let DropPlan::Picker(choices) = request.plan else { panic!("expected the picker") };
        assert_eq!(choices[0].label, "Gradient 1");
        assert!(choices[0].group.is_some());
        let before = choices[1].prop.value(&state.session.lock().effect(a).unwrap().node.params).unwrap().clone();
        state.apply_drop(&choices[1], &gradient, &request.elements, Instant::now());
        let session = state.session.lock();
        let after = choices[1].prop.value(&session.effect(a).unwrap().node.params).unwrap().clone();
        let (ParamValue::GradientLevels(old), ParamValue::GradientLevels(new)) = (before, after) else { panic!() };
        assert_eq!(new[1].curve, old[1].curve);
        assert_eq!(new[0], old[0]);
        assert_ne!(new[1].color_gradient, old[1].color_gradient);
    }

    #[test]
    fn dropping_on_a_selected_effect_covers_its_type_and_warns_about_others() {
        let mut state = fixture("multi");
        let a = state.add_effect_at(SET_LEVEL, 0, s(1.0)).unwrap();
        let b = state.add_effect_at(SET_LEVEL, 0, s(4.0)).unwrap();
        let c = state.add_effect_at(id_of("Pulse"), 0, s(8.0)).unwrap();
        state.grid.set_selection(&[a, b, c]);
        let red = DropValue::Color(Color::rgb(255, 0, 0));
        let request = state.plan_drop(a, &red).unwrap();
        assert_eq!(request.warning.as_ref().unwrap().title, "Multiple Type Effects");
        assert_eq!(request.elements, vec![a, b]);
        let DropPlan::Direct(choice) = request.plan else { panic!("expected direct") };
        state.apply_drop(&choice, &request.value, &request.elements, Instant::now());
        assert_eq!(state.ui_state().status_message, "Color applied to 2 Set Level effect(s).");
        // Not selected: just that one.
        state.grid.set_selection(&[b]);
        assert_eq!(state.plan_drop(a, &red).unwrap().elements, vec![a]);
    }

    #[test]
    fn file_drop_plan_follows_modifiers_and_the_case_quirk() {
        let files = vec![PathBuf::from("/a/one.png"), PathBuf::from("/a/two.txt"), PathBuf::from("/a/three.PNG")];
        let (items, horizontal) = plan_file_drop(&files, Mods::NONE, &[]);
        assert!(!horizontal);
        assert!(matches!(&items[0], FileDropItem::Effect { name: "Picture", index_in_sequence: 0, .. }));
        assert_eq!(items[1], FileDropItem::Skip);
        // Non-sequential: every file at the drop time. Upper case extensions are lowered first.
        assert!(matches!(&items[2], FileDropItem::Effect { name: "Picture", index_in_sequence: 0, .. }));

        let (items, _) = plan_file_drop(&files, Mods::CMD, &[]);
        // Sequential: 0, then the skipped file still counts, so 2.
        assert!(matches!(&items[0], FileDropItem::Effect { index_in_sequence: 0, .. }));
        assert!(matches!(&items[2], FileDropItem::Effect { index_in_sequence: 2, .. }));

        let (_, horizontal) = plan_file_drop(&files, Mods::OPT, &[]);
        assert!(horizontal);
        let (_, horizontal) = plan_file_drop(&files, Mods { shift: false, cmd: true, opt: true }, &[]);
        assert!(horizontal);

        // `.MTS` never matches: the drop lower-cases and the list doesn't.
        let (items, _) = plan_file_drop(&[PathBuf::from("/a/clip.MTS")], Mods::NONE, &[]);
        assert_eq!(items[0], FileDropItem::Skip);
    }

    #[test]
    fn a_dropped_picture_gets_its_file() {
        let mut state = fixture("file");
        let picture = id_of("Picture");
        let id = state.add_file_effect(picture, Path::new("/a/one.png"), false, 0, s(1.0), 2).unwrap();
        let session = state.session.lock();
        let entry = session.effect(id).unwrap();
        // 2 s per index.
        assert_eq!(entry.node.start, s(5.0));
        assert_eq!(entry.node.params.get("FileName"), &ParamValue::Str("/a/one.png".to_string()));
        assert_eq!(entry.node.params.get("Source"), &ParamValue::Enum("File".to_string()));
    }
}
