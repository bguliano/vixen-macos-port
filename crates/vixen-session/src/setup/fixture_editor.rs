//! Native fixture profile/function editors. Text drafts preserve invalid input until corrected;
//! validation and the selection gate stay in Rust (specialized editors F109–F123).
use super::fixture::{
    FixtureProfile, ProfileChannel, ProfileFunction, ProfileIndex, ProfileWheelEntry,
};
use fx_fixture::spec::{FixtureFunctionType as T, FixtureIndexType, FixtureZoomType, FuncId};
use std::collections::HashSet;
use vixen_output::DotNetColor;

pub const FUNCTION_TYPES: &[&str] = &["Range", "Indexed", "ColorWheel", "RGBColor", "RGBWColor"];
pub const INDEX_TAGS: &[&str] = &[
    "Custom",
    "ShutterOpen",
    "ShutterClosed",
    "LampOn",
    "LampOff",
    "Prism",
    "PrismOpen",
    "PrismClose",
    "Strobe",
];

#[derive(Clone, Debug)]
pub struct IndexDraft {
    pub name: String,
    pub start: String,
    pub end: String,
    pub use_curve: bool,
    pub tag: String,
    pub image: String,
    pub half: bool,
    pub color1: DotNetColor,
    pub color2: DotNetColor,
    pub exclude: bool,
}
impl Default for IndexDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            start: String::new(),
            end: String::new(),
            use_curve: false,
            tag: "Custom".into(),
            image: String::new(),
            half: false,
            color1: DotNetColor::WHITE,
            color2: DotNetColor::WHITE,
            exclude: false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct FunctionDraft {
    pub value: ProfileFunction,
    pub indices: Vec<IndexDraft>,
    pub wheel: Vec<IndexDraft>,
    pub start: String,
    pub stop: String,
}
impl FunctionDraft {
    fn new(value: ProfileFunction) -> Self {
        let indices = value
            .index_data
            .iter()
            .map(|r| IndexDraft {
                name: r.name.clone().unwrap_or_default(),
                start: r.start_value.to_string(),
                end: if r.end_value == -1 {
                    String::new()
                } else {
                    r.end_value.to_string()
                },
                use_curve: r.use_curve,
                tag: r.index_type.name().into(),
                image: r.image.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect();
        // The wheel deliberately displays -1, whereas Indexed displays an empty End Value.
        let wheel = value
            .color_wheel_data
            .iter()
            .map(|r| IndexDraft {
                name: r.name.clone().unwrap_or_default(),
                start: r.start_value.to_string(),
                end: r.end_value.to_string(),
                use_curve: r.use_curve,
                half: r.half_step,
                color1: r.color1.clone(),
                color2: r.color2.clone(),
                exclude: r.exclude_color_property,
                ..Default::default()
            })
            .collect();
        Self {
            start: value.rotation_limits.start_position.to_string(),
            stop: value.rotation_limits.stop_position.to_string(),
            value,
            indices,
            wheel,
        }
    }
    pub fn panel(&self) -> &'static str {
        match self.value.function_type {
            T::Indexed => "indexed",
            T::ColorWheel => "wheel",
            T::Range => match self.value.identity {
                FuncId::Pan | FuncId::Tilt => "rotation",
                FuncId::Zoom => "zoom",
                _ => "none",
            },
            _ => "none",
        }
    }
    pub fn tags(&self) -> Vec<String> {
        let tags: &[&str] = match self.value.function_type {
            T::Range => &["Pan", "Tilt", "Zoom", "Dim", "Frost", "Custom"],
            T::Indexed => &["Shutter", "Gobo", "Prism", "OpenClosePrism", "Custom"],
            T::ColorWheel => &["SpinColorWheel"],
            _ => &["Custom"],
        };
        tags.iter().map(|s| (*s).into()).collect()
    }
    pub fn detail_errors(&self) -> Vec<String> {
        let mut e = Vec::new();
        match self.panel() {
            "indexed" | "wheel" => {
                let indexed = self.panel() == "indexed";
                let rows = if indexed { &self.indices } else { &self.wheel };
                for r in rows {
                    required(&r.name, "Name", &mut e);
                    dmx(&r.start, "Start Value", true, false, &mut e);
                    // Always parse nonblank End: upstream only checks it with Use Curve and can crash on save.
                    dmx(
                        &r.end,
                        "End Value",
                        r.use_curve,
                        !indexed && !r.use_curve,
                        &mut e,
                    );
                }
                if indexed {
                    if duplicate(rows.iter().map(|r| r.name.as_str())) {
                        e.push("Cannot have duplicate index names.".into());
                    }
                    if rows
                        .iter()
                        .enumerate()
                        .any(|(i, a)| rows.iter().skip(i + 1).any(|b| overlaps(a, b)))
                    {
                        e.push("Index ranges cannot overlap.".into());
                    }
                }
            }
            "rotation" => {
                let start = degrees(&self.start, "Start Position", &mut e);
                let stop = degrees(&self.stop, "Stop Position", &mut e);
                if start.zip(stop).is_some_and(|(a, b)| a > b) {
                    e.push(
                        "Start Position must be less than or equal to the Stop Position.".into(),
                    );
                }
            }
            _ => {}
        }
        e
    }
    fn materialize(&self) -> ProfileFunction {
        let mut f = self.value.clone();
        f.index_data = self
            .indices
            .iter()
            .map(|r| ProfileIndex {
                name: Some(r.name.clone()),
                start_value: r.start.trim().parse().unwrap_or(0),
                end_value: r.end.trim().parse().unwrap_or(-1),
                use_curve: r.use_curve,
                index_type: FixtureIndexType::from_name(&r.tag),
                image: (!r.image.trim().is_empty()).then(|| r.image.clone()),
            })
            .collect();
        f.color_wheel_data = self
            .wheel
            .iter()
            .map(|r| ProfileWheelEntry {
                name: Some(r.name.clone()),
                start_value: r.start.trim().parse().unwrap_or(0),
                end_value: r.end.trim().parse().unwrap_or(-1),
                use_curve: r.use_curve,
                half_step: r.half,
                color1: r.color1.clone(),
                color2: r.color2.clone(),
                exclude_color_property: r.exclude,
            })
            .collect();
        if self.panel() == "rotation" {
            f.rotation_limits.start_position = self.start.trim().parse().unwrap_or(0);
            f.rotation_limits.stop_position = self.stop.trim().parse().unwrap_or(360);
        }
        f
    }
}
fn required(value: &str, label: &str, errors: &mut Vec<String>) {
    if value.is_empty() {
        errors.push(format!("{label} is empty.  {label} is a required field."));
    }
}
fn duplicate<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values.into_iter().any(|v| !seen.insert(v))
}
fn dmx(
    value: &str,
    label: &str,
    required: bool,
    allow_sentinel: bool,
    e: &mut Vec<String>,
) -> Option<i32> {
    if value.is_empty() {
        if required {
            e.push(format!("{label} is empty. {label} is a required field."));
        }
        return None;
    }
    match value.trim().parse::<i32>() {
        Ok(v) if (0..=255).contains(&v) || (allow_sentinel && v == -1) => Some(v),
        _ => {
            e.push(format!(
                "Not a valid {label} value.  Valid range is 0 to 255."
            ));
            None
        }
    }
}
fn degrees(value: &str, label: &str, e: &mut Vec<String>) -> Option<i32> {
    if value.is_empty() {
        e.push(format!(
            "{label} is empty.  Valid range is 0-999 degrees.  "
        ));
        return None;
    }
    match value.trim().parse::<i32>() {
        Ok(v) if (0..=999).contains(&v) => Some(v),
        _ => {
            e.push(format!(
                "Not a valid {label} value.  Valid values are 0-999.  "
            ));
            None
        }
    }
}
fn overlaps(a: &IndexDraft, b: &IndexDraft) -> bool {
    let range = |r: &IndexDraft| -> Option<(i32, i32)> {
        let start = r.start.trim().parse().ok()?;
        let end = if r.end.is_empty() {
            start
        } else {
            r.end.trim().parse().ok()?
        };
        Some((start, end))
    };
    range(a)
        .zip(range(b))
        .is_some_and(|((a, b), (c, d))| a <= d && b >= c)
}

#[derive(Clone, Debug)]
pub struct ProfileEditor {
    pub profile: FixtureProfile,
    pub functions: Vec<FunctionDraft>,
    pub selected: Option<usize>,
    pub selected_channel: Option<usize>,
    pub selected_index: Option<usize>,
    pub notice: Option<String>,
}
impl ProfileEditor {
    pub fn new(mut profile: FixtureProfile, initial: Option<&str>) -> Self {
        for c in &mut profile.channels {
            if !profile
                .functions
                .iter()
                .any(|f| f.name == c.function && f.name.is_some())
            {
                c.function = Some("None".into());
            }
        }
        let functions: Vec<_> = profile
            .functions
            .iter()
            .filter(|f| f.function_type != T::None)
            .cloned()
            .map(FunctionDraft::new)
            .collect();
        let selected = initial.and_then(|s| {
            functions
                .iter()
                .position(|f| f.value.name.as_deref() == Some(s))
        });
        Self {
            profile,
            functions,
            selected,
            selected_channel: None,
            selected_index: None,
            notice: None,
        }
    }
    pub fn functions_errors(&self) -> Vec<String> {
        let mut e = Vec::new();
        for f in &self.functions {
            required(f.value.name.as_deref().unwrap_or(""), "Name", &mut e);
            e.extend(f.detail_errors());
        }
        if duplicate(
            self.functions
                .iter()
                .map(|f| f.value.name.as_deref().unwrap_or("")),
        ) {
            e.push("Cannot have duplicate function names.".into());
        }
        // An imported channel may already use an empty function. Catch that while its details are editable,
        // before the wizard enters the channel page (which blocks both Back and Next on validation).
        for f in &self.functions {
            if self
                .profile
                .channels
                .iter()
                .any(|c| c.function == f.value.name)
            {
                if f.value.function_type == T::Indexed && f.indices.is_empty() {
                    e.push("Function index data is incomplete.".into());
                }
                if f.value.function_type == T::ColorWheel && f.wheel.is_empty() {
                    e.push("Function color wheel data is incomplete.".into());
                }
            }
        }
        for tag in [FuncId::Pan, FuncId::Tilt, FuncId::Zoom] {
            if self
                .functions
                .iter()
                .filter(|f| f.value.identity == tag)
                .count()
                > 1
            {
                e.push(format!(
                    "Cannot have more than one {} function.",
                    tag.name()
                ));
            }
        }
        e
    }
    pub fn channel_errors(&self) -> Vec<String> {
        let mut e = Vec::new();
        if self.profile.name_text().is_empty() {
            e.push("Profile name is empty.  Profile is a required field.".into());
        }
        if self.profile.channels.is_empty() {
            e.push("At least one channel is required to define a fixture.".into());
        }
        for c in &self.profile.channels {
            required(c.name.as_deref().unwrap_or(""), "Name", &mut e);
            required(c.function.as_deref().unwrap_or(""), "Function", &mut e);
            if let Some(f) = self.functions.iter().find(|f| f.value.name == c.function) {
                if f.value.function_type == T::Indexed && f.indices.is_empty() {
                    e.push("Function index data is incomplete.".into());
                }
                if f.value.function_type == T::ColorWheel && f.wheel.is_empty() {
                    e.push("Function color wheel data is incomplete.".into());
                }
            } else if c.function.as_deref() != Some("None") {
                e.push("Function is not defined.".into());
            }
        }
        e
    }
    pub fn errors(&self) -> Vec<String> {
        let mut e = self.functions_errors();
        e.extend(self.channel_errors());
        e
    }
    pub fn profile(&self) -> FixtureProfile {
        let mut p = self.profile.clone();
        p.functions = self
            .functions
            .iter()
            .map(FunctionDraft::materialize)
            .collect();
        p.functions.push(ProfileFunction {
            name: Some("None".into()),
            function_type: T::None,
            timeline_color: DotNetColor::from_html("Transparent").unwrap_or(DotNetColor::WHITE),
            ..Default::default()
        });
        p
    }
    pub fn try_select(&mut self, index: Option<usize>) -> bool {
        self.notice = None;
        if index == self.selected {
            return true;
        }
        if let Some(f) = self.selected.and_then(|i| self.functions.get(i)) {
            let e = f.detail_errors();
            if !e.is_empty() {
                self.notice = Some(e.join("\n"));
                return false;
            }
        }
        self.selected = index.filter(|i| *i < self.functions.len());
        self.selected_index = None;
        true
    }
    pub fn set_profile_field(&mut self, field: &str, value: String) {
        match field {
            "name" => self.profile.name = Some(value),
            "manufacturer" => self.profile.manufacturer = Some(value),
            "revision" => self.profile.revision = Some(value),
            "createdBy" => self.profile.created_by = Some(value),
            _ => {}
        }
    }
    pub fn add_channel(&mut self) {
        self.profile.channels.push(ProfileChannel {
            name: Some("Ignore".into()),
            function: Some("None".into()),
            channel_number: self.profile.channels.len() as i32 + 1,
        });
        self.selected_channel = Some(self.profile.channels.len() - 1);
    }
    pub fn remove_channel(&mut self, index: usize) {
        if index < self.profile.channels.len() {
            self.profile.channels.remove(index);
            for (i, c) in self.profile.channels.iter_mut().enumerate() {
                c.channel_number = i as i32 + 1;
            }
            self.selected_channel = index
                .checked_sub(1)
                .or((!self.profile.channels.is_empty()).then_some(0));
        }
    }
    pub fn set_channel_field(&mut self, index: usize, field: &str, value: String) {
        self.notice = None;
        let Some(c) = self.profile.channels.get_mut(index) else {
            return;
        };
        if field == "name" {
            c.name = Some(value);
        } else if field == "function" {
            if value != "None" {
                let Some(f) = self
                    .functions
                    .iter()
                    .find(|f| f.value.name.as_deref() == Some(&value))
                else {
                    return;
                };
                if (f.value.function_type == T::Indexed && f.indices.is_empty())
                    || (f.value.function_type == T::ColorWheel && f.wheel.is_empty())
                {
                    self.notice=Some(format!("Function '{value}' is incomplete.  Please populate the function before assigning it to a channel."));
                    return;
                }
            }
            if c.name.as_deref() == Some("Ignore") && value != "None" {
                c.name = Some(value.clone());
            }
            c.function = Some(value);
        }
    }
    pub fn add_function(&mut self) {
        if !self.try_select(None) {
            return;
        }
        // Fixed, bounded palette instead of upstream's unbounded KnownColors random walk.
        let palette = [
            "Red", "Lime", "Blue", "Yellow", "Cyan", "Magenta", "Orange", "Purple", "Pink", "White",
        ];
        let color = palette
            .iter()
            .filter_map(|s| DotNetColor::from_html(s))
            .find(|c| !self.functions.iter().any(|f| f.value.timeline_color == *c))
            .unwrap_or(DotNetColor::WHITE);
        self.functions.push(FunctionDraft::new(ProfileFunction {
            timeline_color: color,
            ..Default::default()
        }));
        self.selected = Some(self.functions.len() - 1);
    }
    pub fn remove_function(&mut self, index: usize) {
        if index < self.functions.len() {
            self.functions.remove(index);
            self.reconcile_channels();
            self.selected = index
                .checked_sub(1)
                .or((!self.functions.is_empty()).then_some(0));
            self.selected_index = None;
            self.notice = None;
        }
    }
    pub fn reconcile_channels(&mut self) {
        for c in &mut self.profile.channels {
            if !self.functions.iter().any(|f| f.value.name == c.function) {
                c.function = Some("None".into());
            }
        }
    }
    pub fn set_function_field(&mut self, index: usize, field: &str, value: String) {
        self.notice = None;
        let Some(f) = self.functions.get_mut(index) else {
            return;
        };
        if matches!(field, "type" | "tag") {
            let e = f.detail_errors();
            if !e.is_empty() {
                self.notice = Some(e.join("\n"));
                return;
            }
        }
        match field {
            "name" => f.value.name = Some(value),
            "type" => f.value.function_type = T::from_name(&value),
            "tag" => f.value.identity = FuncId::from_name(&value),
            "label" => f.value.label = Some(value),
            "associated" => f.value.associated_function_name = Some(value),
            "start" => f.start = value,
            "stop" => f.stop = value,
            "zoom" => {
                f.value.zoom_type = if value == "WideToNarrow" {
                    FixtureZoomType::WideToNarrow
                } else {
                    FixtureZoomType::NarrowToWide
                }
            }
            _ => {}
        }
    }
    pub fn add_index(&mut self) {
        if let Some(f) = self.selected.and_then(|i| self.functions.get_mut(i)) {
            let rows = if f.panel() == "wheel" {
                &mut f.wheel
            } else {
                &mut f.indices
            };
            rows.push(IndexDraft::default());
            self.selected_index = Some(rows.len() - 1);
        }
    }
    pub fn remove_index(&mut self, index: usize) {
        if let Some(f) = self.selected.and_then(|i| self.functions.get_mut(i)) {
            let rows = if f.panel() == "wheel" {
                &mut f.wheel
            } else {
                &mut f.indices
            };
            if index < rows.len() {
                rows.remove(index);
                self.selected_index = index.checked_sub(1).or((!rows.is_empty()).then_some(0));
            }
        }
    }
    pub fn set_index_field(&mut self, index: usize, field: &str, value: String) {
        if let Some(f) = self.selected.and_then(|i| self.functions.get_mut(i)) {
            let wheel = f.panel() == "wheel";
            let rows = if wheel { &mut f.wheel } else { &mut f.indices };
            let Some(r) = rows.get_mut(index) else {
                return;
            };
            match field {
                "name" => r.name = value,
                "start" => r.start = value,
                "end" => r.end = value,
                "curve" => r.use_curve = value == "true",
                "tag" => r.tag = value,
                "image" => r.image = value,
                "half" => r.half = value == "true",
                "exclude" => r.exclude = value == "true",
                "color1" if !r.half => {
                    if let Some(c) = DotNetColor::from_html(&value) {
                        r.color1 = c;
                    }
                }
                "color2" if !r.half => {
                    if let Some(c) = DotNetColor::from_html(&value) {
                        r.color2 = c;
                    }
                }
                _ => {}
            }
            let recompute = field.starts_with("color") || (field == "half" && r.half);
            if wheel && recompute {
                for i in 0..rows.len() {
                    if rows[i].half {
                        if i > 0 {
                            rows[i].color1 = rows[i - 1].color1.clone();
                        }
                        if i + 1 < rows.len() {
                            rows[i].color2 = rows[i + 1].color1.clone();
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editor() -> ProfileEditor {
        ProfileEditor::new(FixtureProfile::new_blank(), Some("Shutter"))
    }
    #[test]
    fn selection_gate_preserves_invalid_text_and_type_switch() {
        let mut e = editor();
        e.add_index();
        e.set_index_field(0, "name", "Open".into());
        e.set_index_field(0, "start", "bad".into());
        let i = e.selected.unwrap();
        assert!(!e.try_select(Some(0)));
        e.set_function_field(i, "type", "Range".into());
        assert_eq!(e.functions[i].panel(), "indexed");
        e.set_index_field(0, "start", "0".into());
        assert!(e.try_select(Some(0)));
        e.try_select(Some(i));
        e.set_function_field(i, "type", "Range".into());
        e.set_function_field(i, "type", "Indexed".into());
        assert_eq!(e.functions[i].indices[0].name, "Open");
    }
    #[test]
    fn bad_end_never_panics_even_without_curve() {
        let mut e = editor();
        e.add_index();
        e.set_index_field(0, "name", "Open".into());
        e.set_index_field(0, "start", "0".into());
        e.set_index_field(0, "end", "oops".into());
        assert!(e.functions_errors().iter().any(|s| s.contains("End Value")));
        e.set_index_field(0, "end", "256".into());
        assert!(e.functions_errors().iter().any(|s| s.contains("End Value")));
    }
    #[test]
    fn inclusive_ranges_and_duplicate_names() {
        let mut e = editor();
        for (start, end) in [("0", "10"), ("10", "")] {
            e.add_index();
            let i = e.selected_index.unwrap();
            e.set_index_field(i, "name", "Same".into());
            e.set_index_field(i, "start", start.into());
            e.set_index_field(i, "end", end.into());
        }
        let errors = e.functions_errors();
        assert!(errors.iter().any(|s| s == "Index ranges cannot overlap."));
        assert!(errors
            .iter()
            .any(|s| s == "Cannot have duplicate index names."));
    }
    #[test]
    fn incomplete_channel_assignment_is_atomic_and_delete_reconciles() {
        let mut e = editor();
        e.add_channel();
        e.set_channel_field(0, "function", "Shutter".into());
        assert_eq!(e.profile.channels[0].name.as_deref(), Some("Ignore"));
        assert_eq!(e.profile.channels[0].function.as_deref(), Some("None"));
        e.set_channel_field(0, "function", "Pan".into());
        assert_eq!(e.profile.channels[0].name.as_deref(), Some("Pan"));
        e.remove_function(0);
        assert_eq!(e.profile.channels[0].function.as_deref(), Some("None"));
    }
    #[test]
    fn wheel_half_steps_and_end_validation() {
        let mut e = editor();
        let i = e
            .functions
            .iter()
            .position(|f| f.value.function_type == T::ColorWheel)
            .unwrap();
        e.try_select(Some(i));
        for n in 0..3 {
            e.add_index();
            e.set_index_field(n, "name", format!("C{n}"));
            e.set_index_field(n, "start", (n * 10).to_string());
        }
        e.set_index_field(0, "color1", "Red".into());
        e.set_index_field(2, "color1", "Blue".into());
        e.set_index_field(1, "half", "true".into());
        assert_eq!(
            e.functions[i].wheel[1].color1,
            DotNetColor::from_html("Red").unwrap()
        );
        assert_eq!(
            e.functions[i].wheel[1].color2,
            DotNetColor::from_html("Blue").unwrap()
        );
        e.set_index_field(1, "curve", "true".into());
        e.set_index_field(1, "end", "not a number".into());
        assert!(e.functions_errors().iter().any(|s| s.contains("End Value")));
    }
    #[test]
    fn new_pan_has_limits_and_names_are_unique() {
        let mut e = editor();
        e.add_function();
        let i = e.selected.unwrap();
        e.set_function_field(i, "name", "Pan".into());
        e.set_function_field(i, "tag", "Pan".into());
        assert_eq!(
            (&*e.functions[i].start, &*e.functions[i].stop),
            ("0", "360")
        );
        let errors = e.functions_errors();
        assert!(errors
            .iter()
            .any(|s| s.contains("duplicate function names")));
        assert!(errors.iter().any(|s| s.contains("more than one Pan")));
    }
    #[test]
    fn blank_index_end_and_wheel_minus_one_round_trip() {
        let mut e = editor();
        e.add_index();
        e.set_index_field(0, "name", "Open".into());
        e.set_index_field(0, "start", "12".into());
        let p = e.profile();
        let mut r = ProfileEditor::new(p, Some("Shutter"));
        assert_eq!(r.functions[r.selected.unwrap()].indices[0].end, "");
        r.add_channel();
        r.add_channel();
        r.remove_channel(0);
        assert_eq!(r.profile.channels[0].channel_number, 1);
        assert!(r
            .profile()
            .functions
            .iter()
            .any(|f| f.function_type == T::None));
    }
}

pub fn property_profile(
    world: &dyn crate::dialogs::world::DialogWorld,
    node: crate::dialogs::world::NodeKey,
) -> FixtureProfile {
    use crate::dialogs::properties::raw_property;
    world
        .properties(node)
        .iter()
        .position(|p| p.type_id == vixen_core::element::property_ids::INTELLIGENT_FIXTURE)
        .and_then(|i| raw_property(world, node, i))
        .and_then(|e| e.child("FixtureSpecification").cloned().or(Some(e)))
        .and_then(|e| super::fixture::read_profile(&e).ok())
        .unwrap_or_else(|| {
            let mut p = FixtureProfile::new_blank();
            p.channels.push(ProfileChannel {
                name: None,
                function: None,
                channel_number: 1,
            });
            p
        })
}
pub fn apply_property_profile(
    world: &mut dyn crate::dialogs::world::DialogWorld,
    primary: crate::dialogs::world::NodeKey,
    selected: &[crate::dialogs::world::NodeKey],
    profile: &FixtureProfile,
) {
    use crate::dialogs::properties::{ensure_property, set_raw_property};
    let type_id = vixen_core::element::property_ids::INTELLIGENT_FIXTURE;
    let mut nodes = vec![primary];
    nodes.extend(
        selected
            .iter()
            .copied()
            .filter(|n| *n != primary && world.properties(*n).iter().any(|p| p.type_id == type_id)),
    );
    for node in nodes {
        let index = ensure_property(world, node, &type_id);
        let id = world.properties(node)[index].instance_id;
        set_raw_property(
            world,
            node,
            index,
            super::fixture::fixture_property_data(profile, &type_id, &id),
        );
    }
}

/// One shared GPU model for wizard decorations and the Pan/Tilt demonstration. The native display
/// link supplies elapsed time; this object contains the source's fixed sweep, never the field limits.
pub struct FixtureDemo {
    renderer: vixen_render::LiveRenderer,
    width: f32,
    height: f32,
    pan: f32,
    tilt: f32,
    color: u32,
}
impl FixtureDemo {
    /// The caller keeps its CAMetalLayer alive until this object is dropped.
    pub unsafe fn open(layer: u64, width: u32, height: u32, scale: f32) -> Result<Self, String> {
        let renderer = unsafe {
            vixen_render::LiveRenderer::for_layer(
                layer as *mut std::ffi::c_void,
                width,
                height,
                scale,
            )
        }
        .map_err(|e| e.to_string())?;
        renderer.set_transparent(true);
        let d = Self {
            renderer,
            width: width as f32 / scale.max(1.0),
            height: height as f32 / scale.max(1.0),
            pan: 35.0,
            tilt: 45.0,
            color: 0xff00ffff,
        };
        d.draw();
        Ok(d)
    }
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) {
        self.width = width as f32 / scale.max(1.0);
        self.height = height as f32 / scale.max(1.0);
        self.renderer.resize(width, height, scale);
        self.draw();
    }
    pub fn set(&mut self, pan: f32, tilt: f32, color: u32) {
        self.pan = pan;
        self.tilt = tilt;
        self.color = color;
        self.draw();
    }
    pub fn advance(&mut self, elapsed: f64, mode: &str) {
        let (pan, tilt) = demo_angles(elapsed, mode);
        self.pan = pan;
        self.tilt = tilt;
        self.draw();
    }
    fn draw(&self) {
        use vixen_render::{BeamWidthModel, MovingHeadInstance, MovingHeadParams};
        let size = self.width.min(self.height);
        let c = vixen_core::Color::from_argb_u32(self.color);
        let color = vixen_render::rgba(c.r, c.g, c.b, c.a);
        self.renderer.set_moving_heads(vec![MovingHeadInstance {
            position: [self.width / 2.0, self.height / 2.0],
            length: size,
            params: MovingHeadParams {
                pan_deg: self.pan,
                tilt_deg: self.tilt,
                tilt_offset_deg: 180.0,
                max_beam_length: size,
                beam_length_pct: 20.0,
                focus: 20.0,
                color_left: color,
                color_right: color,
                beam_width: BeamWidthModel::EditorTopPlusBottom,
                thumbnail: true,
                ..Default::default()
            },
        }]);
    }
}
fn demo_angles(elapsed: f64, mode: &str) -> (f32, f32) {
    let sweep = |span: f64| {
        let phase = (elapsed.max(0.0) * 100.0) % (span * 2.0);
        if phase <= span {
            phase
        } else {
            span * 2.0 - phase
        }
    };
    if mode == "Tilt" {
        (0.0, 180.0 + sweep(180.0) as f32)
    } else {
        (sweep(360.0) as f32, 90.0)
    }
}
#[cfg(test)]
mod demo_tests {
    use super::*;
    #[test]
    fn fixed_sweeps_reverse_at_source_limits() {
        assert_eq!(demo_angles(0.0, "Pan"), (0.0, 90.0));
        assert_eq!(demo_angles(3.6, "Pan"), (360.0, 90.0));
        assert_eq!(demo_angles(4.0, "Pan"), (320.0, 90.0));
        assert_eq!(demo_angles(0.0, "Tilt"), (0.0, 180.0));
        assert_eq!(demo_angles(2.0, "Tilt"), (0.0, 340.0));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::dialogs::properties::add_property;
    use crate::dialogs::world::{DialogWorld, NodeKey, SystemWorld};
    use vixen_core::element::property_ids;
    #[test]
    fn fixture_property_copies_preserve_instance_ids_and_store_data() {
        let mut system = vixen_io::SystemData::default();
        let a = NodeKey(system.tree.add_element("A").0);
        let b = NodeKey(system.tree.add_element("B").0);
        let c = NodeKey(system.tree.add_element("C").0);
        let mut profile = FixtureProfile::new_blank();
        profile.name = Some("Edited Fixture".into());
        profile.channels.push(ProfileChannel {
            name: Some("Pan".into()),
            function: Some("Pan".into()),
            channel_number: 1,
        });
        let ids;
        {
            let mut w = SystemWorld::new(&mut system);
            add_property(&mut w, a, &property_ids::INTELLIGENT_FIXTURE);
            add_property(&mut w, b, &property_ids::INTELLIGENT_FIXTURE);
            ids = [
                w.properties(a)[0].instance_id,
                w.properties(b)[0].instance_id,
            ];
            apply_property_profile(&mut w, a, &[a, b, c], &profile);
            assert_eq!(property_profile(&w, a), profile);
            assert_eq!(property_profile(&w, b), profile);
            assert!(w.properties(c).is_empty());
        }
        assert_ne!(ids[0], ids[1]);
        for id in ids {
            let entry = system
                .module_store
                .instance(property_ids::INTELLIGENT_FIXTURE, id)
                .unwrap();
            assert_eq!(
                entry.data().unwrap().child_text("ModuleInstanceId"),
                Some(id.to_string())
            );
        }
        assert!(matches!(
            vixen_io::system::effective_store(&system),
            std::borrow::Cow::Borrowed(_)
        ));
    }
}

#[cfg(test)]
mod demo_gpu_tests {
    use super::*;
    #[test]
    fn shared_editor_demo_renders_and_rotates_on_gpu() {
        let renderer = match vixen_render::LiveRenderer::headless(300, 300, 1.0) {
            Ok(r) => r,
            Err(e) if std::env::var_os("VIXEN_RENDER_REQUIRE_GPU").is_none() => {
                eprintln!("Skipping unavailable GPU: {e}");
                return;
            }
            Err(e) => panic!("required GPU: {e}"),
        };
        renderer.set_transparent(true);
        let mut demo = FixtureDemo {
            renderer,
            width: 300.0,
            height: 300.0,
            pan: 35.0,
            tilt: 45.0,
            color: 0xff00ffff,
        };
        demo.draw();
        let before = demo.renderer.read_pixels();
        assert!(
            before
                .chunks_exact(4)
                .any(|p| p[1] > 50 && p[2] > 50 && p[0] < 30),
            "cyan beam"
        );
        assert!(
            before
                .chunks_exact(4)
                .any(|p| p[0] > 10 && p[0] == p[1] && p[1] == p[2]),
            "gray body"
        );
        demo.advance(1.0, "Tilt");
        let after = demo.renderer.read_pixels();
        assert_ne!(
            before, after,
            "display-link sweep changes the same GPU model"
        );
        if let Some(path) = std::env::var_os("VIXEN_FIXTURE_DEMO_IMAGE") {
            image::RgbaImage::from_raw(300, 300, before)
                .unwrap()
                .save(path)
                .unwrap();
        }
    }
}

#[cfg(test)]
mod text_semantics_tests {
    use super::*;
    #[test]
    fn numeric_whitespace_is_parsed_consistently_and_names_are_not_trimmed() {
        let mut e = ProfileEditor::new(FixtureProfile::new_blank(), Some("Pan"));
        e.set_function_field(0, "start", " 12 ".into());
        e.set_function_field(0, "stop", " 345 ".into());
        assert!(e.functions_errors().is_empty());
        assert_eq!(e.profile().functions[0].rotation_limits.start_position, 12);
        assert_eq!(e.profile().functions[0].rotation_limits.stop_position, 345);
        e.try_select(Some(6)); // Shutter in the built-in list.
        e.add_index();
        e.set_index_field(0, "name", " ".into());
        e.set_index_field(0, "start", " 23 ".into());
        e.set_index_field(0, "end", " 45 ".into());
        assert!(e.functions_errors().is_empty());
        let f = e.profile().functions[6].clone();
        assert_eq!(
            (f.index_data[0].start_value, f.index_data[0].end_value),
            (23, 45)
        );
        assert_eq!(f.index_data[0].name.as_deref(), Some(" "));
        e.set_index_field(0, "end", " ".into());
        assert!(e.functions_errors().iter().any(|e| e.contains("End Value")));
    }
}
