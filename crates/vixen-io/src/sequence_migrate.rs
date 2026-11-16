//! Port of `TimedSequenceMigrator`: the versioned transformations of a sequence's XML.
//!
//! Each step edits the tree as the C# does. Where the C# builds a new data object (`new WipeData { .. }`), this
//! starts from the effect's table defaults, which are that constructor's values. Where it deserializes, changes
//! one member and serializes again, this changes that member in place: the members the round trip would have
//! added are `default(T)`, which is also what reading the tree back gives for a missing member.
//!
//! Message boxes become notes for the UI. Version 0 media sources are retained
//! for the owning session to copy into its Media folder; XML decoding stays read-only.

use crate::dcs::{
    child_bool, child_f64, child_i32, child_string, child_value, format_bool, parse_double, set_member,
    windows_file_name,
};
use crate::error::IoError;
use crate::module_store::ModuleStore;
use crate::params::params_to_members;
use crate::values::{color_from_xml, color_list_from_xml, curve_from_xml, gradient_from_xml, rgb_from_xml};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use vixen_core::{Color, ColorGradient, Curve, GradientLevelPair};
use vixen_effects::framework::{EffectDescriptor, ParamValue, Params};
use vixen_xml::{contract_ns, XElement, XNode, ARRAYS_NS};

/// `LipSyncMapData`'s two fields the migrations read.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct LipSyncMapInfo {
    is_matrix: bool,
    strings_are_rows: bool,
}

/// What the C# gets from global services. Only LipSync migrations need it, so a sequence with no old LipSync
/// effects never touches the module store.
pub(crate) struct MigrationContext<'a> {
    store: Option<&'a ModuleStore>,
    store_path: Option<PathBuf>,
    maps: Option<Option<HashMap<String, LipSyncMapInfo>>>,
    /// The matrix maps a version 4 LipSync effect named, where the C# clears `UsingDefaults` on the live library.
    pub(crate) matrix_maps_named: Vec<String>,
    pub(crate) media_sources: Vec<(PathBuf, String)>,
}

impl<'a> MigrationContext<'a> {
    /// Looks for the module store where a profile keeps it relative to its Sequence folder.
    pub(crate) fn for_sequence_file(path: &Path) -> MigrationContext<'a> {
        let store_path =
            path.parent().and_then(Path::parent).map(|root| root.join("SystemData").join("ModuleStore.xml"));
        MigrationContext { store: None, store_path, maps: None, matrix_maps_named: Vec::new(), media_sources: Vec::new() }
    }

    pub(crate) fn with_store(store: &'a ModuleStore) -> MigrationContext<'a> {
        MigrationContext { store: Some(store), store_path: None, maps: None, matrix_maps_named: Vec::new(), media_sources: Vec::new() }
    }

    /// A context with nothing to look up in.
    #[cfg(test)]
    pub(crate) fn none() -> MigrationContext<'a> {
        MigrationContext { store: None, store_path: None, maps: None, matrix_maps_named: Vec::new(), media_sources: Vec::new() }
    }

    fn lip_sync_maps(&mut self) -> Option<&HashMap<String, LipSyncMapInfo>> {
        if self.maps.is_none() {
            let owned;
            let store = match (self.store, &self.store_path) {
                (Some(s), _) => Some(s),
                (None, Some(p)) => match crate::module_store::load_module_store_file(p) {
                    Ok(s) => {
                        owned = s;
                        Some(&owned)
                    }
                    Err(_) => None,
                },
                (None, None) => None,
            };
            self.maps = Some(store.and_then(lip_sync_maps_from_store));
        }
        self.maps.as_ref().and_then(|m| m.as_ref())
    }
}

/// `LipSyncMapDescriptor.ModuleID`.
const LIP_SYNC_MAP_TYPE: uuid::Uuid = uuid::uuid!("a475c535-73fa-4d31-a1bc-ed2b50ad21cf");

fn lip_sync_maps_from_store(store: &ModuleStore) -> Option<HashMap<String, LipSyncMapInfo>> {
    let entry = store.type_data(LIP_SYNC_MAP_TYPE).or_else(|| {
        store.type_entries().iter().find(|e| e.data().is_some_and(|d| d.local == "LipSyncMapStaticData"))
    })?;
    let library = entry.data()?.child("_library")?;
    let mut maps = HashMap::new();
    for kv in library.elements() {
        let Some(key) = child_string(kv, "Key") else { continue };
        let Some(value) = kv.child("Value") else { continue };
        maps.insert(
            key,
            LipSyncMapInfo {
                is_matrix: child_bool(value, "IsMatrix").unwrap_or(false),
                strings_are_rows: child_bool(value, "StringsAreRows").unwrap_or(false),
            },
        );
    }
    Some(maps)
}

/// Brings the tree from `from` to `to`, returning what the C# would have told the user.
pub(crate) fn migrate(
    root: &mut XElement,
    from: i32,
    to: i32,
    context: &mut MigrationContext,
) -> Result<Vec<String>, IoError> {
    let mut notes = Vec::new();
    if from > to {
        notes.push("This sequence was saved by a newer version of Vixen that may not be compatible.".to_string());
        return Ok(notes);
    }
    if from == to {
        return Ok(notes);
    }
    for version in from..to {
        match version {
            0 => version_0_to_1(root, context),
            1 => version_1_to_2(root),
            2 => {
                migrate_snowflakes_2_to_3(root);
                migrate_fireworks_2_to_3(root);
            }
            3 => {
                migrate_default_level_3_to_4(root, "VixenModules.Effect.Chase", "ChaseData");
                migrate_default_level_3_to_4(root, "VixenModules.Effect.Spin", "SpinData");
            }
            4 => migrate_lip_sync_4_to_5(root, context, &mut notes),
            5 => migrate_lip_sync_5_to_6(root, context, &mut notes),
            6 => migrate_wipe_6_to_7(root),
            7 => migrate_liquid_7_to_8(root),
            8 => {
                migrate_set_position_8_to_9(root);
                let removed = remove_nutcracker_effects(root);
                if removed > 0 {
                    notes.push(format!(
                        "During the migration, {removed} deprecated Nutcracker effects were removed. To convert them to their newer versions, \
                         open the sequence in Vixen 3.9u5 and convert them there before saving it in this version."
                    ));
                }
            }
            v => {
                return Err(IoError::Migration {
                    what: "sequence".to_string(),
                    from: v,
                    to,
                    message: "no migration path".to_string(),
                })
            }
        }
    }
    root.set_attr("version", to.to_string());
    Ok(notes)
}

fn effect_ns(clr: &str) -> String {
    contract_ns(clr)
}

/// Removes this type's data models from `_dataModels` and returns them in file order.
fn take_models(root: &mut XElement, clr_ns: &str, data_type: &str) -> Vec<XElement> {
    let ns = effect_ns(clr_ns);
    let Some(list) = root.child_mut("_dataModels") else { return Vec::new() };
    let mut taken = Vec::new();
    list.children.retain_mut(|n| {
        if let XNode::Element(e) = n {
            if e.xsi_type.as_ref().is_some_and(|(tns, local)| *tns == ns && local == data_type) {
                taken.push(std::mem::take(e));
                return false;
            }
        }
        true
    });
    taken
}

/// `datamodel.Add(...)`: a migrated model goes to the end of the list.
fn add_model(root: &mut XElement, model: XElement) {
    if let Some(list) = root.child_mut("_dataModels") {
        list.push(model);
    }
}

fn set_table_member(p: &mut Params, xml: &str, v: ParamValue) {
    if let Some(i) = p.desc.iter().position(|d| d.xml == xml) {
        p.values[i] = v;
    }
}

/// Builds a model from a filled-in table: the object C# would serialize, with the old model's identity.
fn rebuilt_model(descriptor: &EffectDescriptor, old: &XElement, params: &Params) -> XElement {
    let mut model = XElement::new(ARRAYS_NS, "anyType");
    model.xsi_type = Some((effect_ns(descriptor.data_ns), descriptor.data_type.to_string()));
    for id in ["ModuleInstanceId", "ModuleTypeId"] {
        if let Some(e) = old.child(id) {
            model.push(e.clone());
        }
    }
    for member in params_to_members(params) {
        model.push(member);
    }
    model
}

/// Sets the member in the namespace of the model's own class. Used by the serialize-change-serialize migrations.
fn set_model_member(model: &mut XElement, clr_ns: &str, name: &str, value: &str) {
    set_member(model, XElement::with_text(&effect_ns(clr_ns), name, value));
}

// ---- Version 0 to 1 --------------------------------------------------------------------------------------

fn version_0_to_1(root: &mut XElement, context: &mut MigrationContext) {
    // Nutcracker file names become relative to the profile.
    if let Some(list) = root.child_mut("_dataModels") {
        for any in list.elements_mut() {
            if let Some(data) = any.child_mut("NutcrackerData") {
                set_file_name(data.child_mut("Picture_FileName"), |_| true);
                set_file_name(data.child_mut("PictureTile_FileName"), |v| {
                    !v.contains("VixenModules.Effect.Nutcracker")
                });
                set_file_name(data.child_mut("Glediator_FileName"), |_| true);
            }
        }
    }
    // Media paths become file names, under a new member name.
    if let Some(list) = root.child_mut("_mediaSurrogates") {
        for media in list.children_named_mut("MediaSurrogate") {
            if let Some(path) = media.child_mut("FilePath") {
                let value = path.text();
                let file = windows_file_name(&value).to_string();
                context.media_sources.push((PathBuf::from(&value), file.clone()));
                path.children = vec![XNode::Text(file)];
                path.local = "FileName".to_string();
            }
        }
    }
    // The timing source's name follows.
    if let Some(timing) = root.child_mut("_selectedTimingProviderSurrogate") {
        if child_value(timing, "ProviderType").as_deref() == Some("Media") {
            set_file_name(timing.child_mut("SourceName"), |_| true);
        }
    }
}

fn set_file_name(element: Option<&mut XElement>, should: impl Fn(&str) -> bool) {
    if let Some(e) = element {
        let value = e.text();
        if should(&value) {
            e.children = vec![XNode::Text(windows_file_name(&value).to_string())];
        }
    }
}

trait ChildrenNamedMut {
    fn children_named_mut<'a>(&'a mut self, local: &'a str) -> Box<dyn Iterator<Item = &'a mut XElement> + 'a>;
}

impl ChildrenNamedMut for XElement {
    fn children_named_mut<'a>(&'a mut self, local: &'a str) -> Box<dyn Iterator<Item = &'a mut XElement> + 'a> {
        Box::new(self.elements_mut().filter(move |e| e.local == local))
    }
}

// ---- Version 1 to 2: Alternating allows any number of colours ----------------------------------------------

fn version_1_to_2(root: &mut XElement) {
    const NS: &str = "VixenModules.Effect.Alternating";
    let Some(d) = vixen_effect_registry::by_data_type("AlternatingData") else { return };
    for old in take_models(root, NS, "AlternatingData") {
        let mut p = d.new_params();
        // "Colors are used backwards in the old effect, so pick them off in reverse."
        let pairs = vec![
            old_alternating_pair(&old, "StaticColor2", "Color2", "Level2", "ColorGradient2", "Curve2"),
            old_alternating_pair(&old, "StaticColor1", "Color1", "Level1", "ColorGradient1", "Curve1"),
        ];
        set_table_member(&mut p, "Colors", ParamValue::GradientLevels(pairs));
        set_table_member(&mut p, "EnableStatic", ParamValue::Bool(!child_bool(&old, "Enable").unwrap_or(false)));
        set_table_member(&mut p, "GroupLevel", ParamValue::Int(child_i32(&old, "GroupEffect").unwrap_or(0)));
        set_table_member(&mut p, "Interval", ParamValue::Int(child_i32(&old, "Interval").unwrap_or(0)));
        set_table_member(&mut p, "IntervalSkipCount", ParamValue::Int(1));
        add_model(root, rebuilt_model(d, &old, &p));
    }
}

fn old_alternating_pair(
    old: &XElement,
    is_static: &str,
    color: &str,
    level: &str,
    gradient: &str,
    curve: &str,
) -> GradientLevelPair {
    if child_value(old, is_static).as_deref() == Some("true") {
        let c = old.child(color).and_then(rgb_from_xml).map_or(Color::EMPTY, |c| c.to_color());
        let l = child_f64(old, level).unwrap_or(0.0) * 100.0;
        GradientLevelPair::new(ColorGradient::solid(c), Curve::from_points(vec![(0.0, l), (100.0, l)]))
    } else {
        GradientLevelPair::new(
            old.child(gradient).and_then(gradient_from_xml).unwrap_or_default(),
            old.child(curve).and_then(curve_from_xml).unwrap_or_default(),
        )
    }
}

// ---- Version 2 to 3: Snowflakes and Fireworks take gradients -----------------------------------------------

fn migrate_snowflakes_2_to_3(root: &mut XElement) {
    const NS: &str = "VixenModules.Effect.Snowflakes";
    let Some(d) = vixen_effect_registry::by_data_type("SnowflakesData") else { return };
    for old in take_models(root, NS, "SnowflakesData") {
        let mut p = d.new_params();
        let gradient_of =
            |name: &str| old.child(name).and_then(color_from_xml).map(ColorGradient::solid).unwrap_or_default();
        set_table_member(&mut p, "OutSideColor", ParamValue::GradientList(vec![gradient_of("OuterColor")]));
        set_table_member(&mut p, "InnerColor", ParamValue::GradientList(vec![gradient_of("CenterColor")]));
        set_table_member(&mut p, "FlakeCount", ParamValue::Int(child_i32(&old, "FlakeCount").unwrap_or(0)));
        if let Some(c) = old.child("LevelCurve").and_then(curve_from_xml) {
            set_table_member(&mut p, "LevelCurve", ParamValue::Curve(c));
        }
        set_table_member(&mut p, "Speed", ParamValue::Int(child_i32(&old, "Speed").unwrap_or(0)));
        if let Some(t) = child_value(&old, "SnowflakeType") {
            set_table_member(&mut p, "SnowflakeType", ParamValue::Enum(t));
        }
        set_table_member(&mut p, "RandomSpeed", ParamValue::Bool(false));
        set_table_member(&mut p, "RandomBrightness", ParamValue::Bool(false));
        set_table_member(&mut p, "PointFlake45", ParamValue::Bool(false));
        add_model(root, rebuilt_model(d, &old, &p));
    }
}

fn migrate_fireworks_2_to_3(root: &mut XElement) {
    const NS: &str = "VixenModules.Effect.Fireworks";
    let Some(d) = vixen_effect_registry::by_data_type("FireworksData") else { return };
    for old in take_models(root, NS, "FireworksData") {
        let mut p = d.new_params();
        let colors = old.child("Colors").and_then(color_list_from_xml).unwrap_or_default();
        set_table_member(
            &mut p,
            "ColorGradients",
            ParamValue::GradientList(colors.into_iter().map(ColorGradient::solid).collect()),
        );
        for name in ["Velocity", "Explosions", "ParticleFade", "Particles"] {
            set_table_member(&mut p, name, ParamValue::Int(child_i32(&old, name).unwrap_or(0)));
        }
        if let Some(c) = old.child("LevelCurve").and_then(curve_from_xml) {
            set_table_member(&mut p, "LevelCurve", ParamValue::Curve(c));
        }
        set_table_member(&mut p, "RandomParticles", ParamValue::Bool(false));
        set_table_member(&mut p, "RandomVelocity", ParamValue::Bool(false));
        add_model(root, rebuilt_model(d, &old, &p));
    }
}

// ---- Version 3 to 4: Chase and Spin ------------------------------------------------------------------------

fn migrate_default_level_3_to_4(root: &mut XElement, clr_ns: &str, data_type: &str) {
    for mut model in take_models(root, clr_ns, data_type) {
        let level = child_value(&model, "DefaultLevel").and_then(|t| parse_double(&t)).unwrap_or(0.0);
        if level > 0.0 {
            set_model_member(&mut model, clr_ns, "EnableDefaultLevel", format_bool(true));
        }
        add_model(root, model);
    }
}

// ---- Version 4 to 5 and 5 to 6: LipSync ----------------------------------------------------------------------

const LIP_SYNC_NS: &str = "VixenModules.Effect.LipSync";

fn migrate_lip_sync_4_to_5(root: &mut XElement, context: &mut MigrationContext, notes: &mut Vec<String>) {
    let models = take_models(root, LIP_SYNC_NS, "LipSyncData");
    if models.is_empty() {
        return;
    }
    let maps = context.lip_sync_maps().cloned();
    if maps.is_none() {
        notes.push(
            "The LipSync map library was not available, so LipSync effects were not migrated from version 4."
                .to_string(),
        );
    }
    for mut model in models {
        let name = child_string(&model, "PhonemeMapping");
        let map = name.as_ref().and_then(|name| maps.as_ref()?.get(name).copied());
        if let Some(map) = map.filter(|m| m.is_matrix) {
            // `mapData.UsingDefaults = false`: the library is not this crate's to change, so the names are handed back.
            if let Some(name) = name.filter(|n| !context.matrix_maps_named.contains(n)) {
                context.matrix_maps_named.push(name);
            }
            if child_i32(&model, "Level").unwrap_or(0) == 0 {
                set_model_member(&mut model, LIP_SYNC_NS, "Level", "100");
            }
            if child_i32(&model, "ScalePercent").unwrap_or(0) == 0 {
                set_model_member(&mut model, LIP_SYNC_NS, "ScalePercent", "100");
            }
            let orientation = if map.strings_are_rows { "Horizontal" } else { "Vertical" };
            set_model_member(&mut model, LIP_SYNC_NS, "Orientation", orientation);
            set_model_member(&mut model, LIP_SYNC_NS, "ScaleToGrid", format_bool(true));
        }
        add_model(root, model);
    }
}

fn migrate_lip_sync_5_to_6(root: &mut XElement, context: &mut MigrationContext, notes: &mut Vec<String>) {
    // `if (library != null)`: with no library, the models stay as they are and where they are.
    let has_models = root
        .child("_dataModels")
        .is_some_and(|l| l.elements().any(|e| e.xsi_type.as_ref().is_some_and(|(_, t)| t == "LipSyncData")));
    if !has_models {
        return;
    }
    let Some(maps) = context.lip_sync_maps().cloned() else {
        notes.push(
            "The LipSync map library was not available, so LipSync effects were not migrated from version 5."
                .to_string(),
        );
        return;
    };
    for mut model in take_models(root, LIP_SYNC_NS, "LipSyncData") {
        let map = child_string(&model, "PhonemeMapping").and_then(|name| maps.get(&name).copied());
        // A map that is not a matrix, or no map at all, becomes a face definition.
        if !map.is_some_and(|m| m.is_matrix) {
            set_model_member(&mut model, LIP_SYNC_NS, "MappingType", "FaceDefinition");
        }
        add_model(root, model);
    }
}

// ---- Version 6 to 7: Wipe -----------------------------------------------------------------------------------

fn migrate_wipe_6_to_7(root: &mut XElement) {
    const NS: &str = "VixenModules.Effect.Wipe";
    let Some(d) = vixen_effect_registry::by_data_type("WipeData") else { return };
    for old in take_models(root, NS, "WipeData") {
        let (direction, reverse) = match child_value(&old, "Direction").as_deref() {
            Some("Up") => ("Vertical", true),
            Some("Down") => ("Vertical", false),
            Some("Right") => ("Horizontal", false),
            Some("Left") => ("Horizontal", true),
            Some("In") => ("Burst", true),
            Some("Out") => ("Burst", false),
            _ => ("Burst", false),
        };
        let mut p = d.new_params();
        if let Some(g) = old.child("ColorGradient").and_then(gradient_from_xml) {
            // `new ColorGradient(other)` copies the library name through its property, so null becomes "".
            set_table_member(&mut p, "ColorGradient", ParamValue::Gradient(g));
        }
        if let Some(h) = child_value(&old, "ColorHandling") {
            set_table_member(&mut p, "ColorHandling", ParamValue::Enum(h));
        }
        set_table_member(&mut p, "PassCount", ParamValue::Int(child_i32(&old, "PassCount").unwrap_or(0)));
        set_table_member(&mut p, "PulsePercent", ParamValue::Double(child_f64(&old, "PulsePercent").unwrap_or(0.0)));
        set_table_member(&mut p, "PulseTime", ParamValue::Int(child_i32(&old, "PulseTime").unwrap_or(0)));
        let by_count = child_bool(&old, "WipeByCount").unwrap_or(false);
        set_table_member(
            &mut p,
            "WipeMovement",
            ParamValue::Enum(if by_count { "Count" } else { "PulseLength" }.to_string()),
        );
        set_table_member(&mut p, "WipeOn", ParamValue::Bool(child_bool(&old, "WipeOn").unwrap_or(false)));
        set_table_member(&mut p, "WipeOff", ParamValue::Bool(child_bool(&old, "WipeOff").unwrap_or(false)));
        set_table_member(&mut p, "ColorAcrossItemPerCount", ParamValue::Bool(true));
        set_table_member(&mut p, "ReverseColorDirection", ParamValue::Bool(true));
        set_table_member(
            &mut p,
            "MovementCurve",
            ParamValue::Curve(Curve::from_points(vec![(0.0, 0.0), (100.0, 100.0)])),
        );
        set_table_member(&mut p, "Direction", ParamValue::Enum(direction.to_string()));
        set_table_member(&mut p, "ReverseDirection", ParamValue::Bool(reverse));
        let old_curve = old.child("Curve").cloned();
        if let Some(c) = old_curve.as_ref().and_then(curve_from_xml) {
            set_table_member(&mut p, "Curve", ParamValue::Curve(c));
        }
        let mut model = rebuilt_model(d, &old, &p);
        // `DeSerializer<Curve>` keeps a null library name where the gradient's copy constructor does not.
        if let Some(c) = old_curve.filter(|c| curve_from_xml(c).is_some()) {
            set_member(&mut model, c);
        }
        add_model(root, model);
    }
}

// ---- Version 7 to 8: Liquid ------------------------------------------------------------------------------------

fn migrate_liquid_7_to_8(root: &mut XElement) {
    const NS: &str = "VixenModules.Effect.Liquid";
    for mut model in take_models(root, NS, "LiquidData") {
        if let Some(list) = model.child_mut("EmitterData") {
            for emitter in list.elements_mut() {
                set_member(emitter, XElement::with_text(&effect_ns(NS), "RandomStartingPosition", format_bool(true)));
            }
        }
        add_model(root, model);
    }
}

// ---- Version 8 to 9: SetPosition, and Nutcracker is gone --------------------------------------------------------

fn migrate_set_position_8_to_9(root: &mut XElement) {
    const NS: &str = "VixenModules.Effect.SetPosition";
    for mut model in take_models(root, NS, "SetPositionData") {
        set_model_member(&mut model, NS, "EnablePan", format_bool(true));
        set_model_member(&mut model, NS, "EnableTilt", format_bool(true));
        add_model(root, model);
    }
}

/// `Nutcracker`'s module type id, from `RemoveNutcrackerEffects`.
const NUTCRACKER_TYPE_ID: &str = "82334cb3-9472-42fe-a221-8482f5c731db";

fn remove_nutcracker_effects(root: &mut XElement) -> usize {
    let removed = take_models(root, "VixenModules.Effect.Nutcracker", "NutcrackerModuleData").len();
    if let Some(list) = root.child_mut("_effectNodeSurrogates") {
        list.children.retain(|n| match n {
            XNode::Element(e) if e.local == "EffectNodeSurrogate" => {
                child_value(e, "TypeId").as_deref() != Some(NUTCRACKER_TYPE_ID)
            }
            _ => true,
        });
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{effect_params_from_xml, params_from_xml};
    use crate::values::{curve_to_xml, gradient_to_xml, rgb_to_xml, DotNetColor};
    use vixen_core::color::Rgb;

    const ID: &str = "00000000-0000-0000-0000-000000000001";
    const TYPE: &str = "00000000-0000-0000-0000-000000000002";

    fn model(clr: &str, data_type: &str) -> XElement {
        let mut e = XElement::new(ARRAYS_NS, "anyType");
        e.xsi_type = Some((effect_ns(clr), data_type.to_string()));
        e.push(XElement::with_text("", "ModuleInstanceId", ID));
        e.push(XElement::with_text("", "ModuleTypeId", TYPE));
        e
    }

    fn root_with(models: Vec<XElement>, version: i32) -> XElement {
        let mut list = XElement::new("", "_dataModels");
        for m in models {
            list.push(m);
        }
        let mut root = XElement::new(crate::sequence::NS_TIMED, "TimedSequenceData");
        root.set_attr("version", version.to_string());
        root.push(list);
        root
    }

    fn only_model(root: &XElement) -> &XElement {
        root.child("_dataModels").unwrap().elements().next().unwrap()
    }

    fn run(root: &mut XElement, from: i32, to: i32) -> Vec<String> {
        migrate(root, from, to, &mut MigrationContext::none()).unwrap()
    }

    fn wipe(direction: &str, by_count: bool) -> XElement {
        let ns = effect_ns("VixenModules.Effect.Wipe");
        let mut m = model("VixenModules.Effect.Wipe", "WipeData");
        m.push(gradient_to_xml(&ns, "ColorGradient", &ColorGradient::solid(Color::RED)));
        m.push(XElement::with_text(&ns, "ColorHandling", "GradientThroughWholeEffect"));
        m.push(curve_to_xml(&ns, "Curve", &Curve::flat(25.0)));
        m.push(XElement::with_text(&ns, "Direction", direction));
        m.push(XElement::with_text(&ns, "PassCount", "3"));
        m.push(XElement::with_text(&ns, "PulsePercent", "33"));
        m.push(XElement::with_text(&ns, "PulseTime", "1000"));
        m.push(XElement::with_text(&ns, "WipeByCount", format_bool(by_count)));
        m.push(XElement::with_text(&ns, "WipeOff", "true"));
        m.push(XElement::with_text(&ns, "WipeOn", "false"));
        m
    }

    #[test]
    fn wipe_directions_map_as_the_csharp_switch_does() {
        let d = vixen_effect_registry::by_data_type("WipeData").unwrap();
        for (old, direction, reverse) in [
            ("Up", "Vertical", true),
            ("Down", "Vertical", false),
            ("Right", "Horizontal", false),
            ("Left", "Horizontal", true),
            ("In", "Burst", true),
            ("Out", "Burst", false),
            ("Sideways", "Burst", false),
        ] {
            let mut root = root_with(vec![wipe(old, true)], 6);
            run(&mut root, 6, 7);
            let p = effect_params_from_xml(only_model(&root), d);
            assert_eq!(p.string("Direction"), direction, "{old}");
            assert_eq!(p.boolean("ReverseDirection"), reverse, "{old}");
        }
    }

    #[test]
    fn wipe_keeps_its_settings_and_gains_the_new_ones() {
        let d = vixen_effect_registry::by_data_type("WipeData").unwrap();
        let mut root = root_with(vec![wipe("Up", false)], 6);
        run(&mut root, 6, 7);
        let m = only_model(&root);
        let p = effect_params_from_xml(m, d);
        assert_eq!(p.int("PassCount"), 3);
        assert_eq!(p.double("PulsePercent"), 33.0);
        assert_eq!(p.int("PulseTime"), 1000);
        assert_eq!(p.string("WipeMovement"), "PulseLength");
        assert!(p.boolean("WipeOff"));
        assert!(!p.boolean("WipeOn"));
        assert!(p.boolean("ColorAcrossItemPerCount"));
        assert!(p.boolean("ReverseColorDirection"));
        assert_eq!(p.curve("MovementCurve").points, vec![(0.0, 0.0), (100.0, 100.0)]);
        assert_eq!(p.curve("Curve").points, Curve::flat(25.0).points);
        assert_eq!(p.gradient("ColorGradient"), ColorGradient::solid(Color::RED));
        // The old members are gone.
        assert!(m.child("WipeByCount").is_none());
        assert_eq!(child_value(m, "ModuleInstanceId").as_deref(), Some(ID));
        assert_eq!(child_value(m, "ModuleTypeId").as_deref(), Some(TYPE));
        // The rest are the constructor's.
        assert_eq!(p.get("TargetPositioning"), &ParamValue::Enum("Strings".to_string()));
    }

    #[test]
    fn wipe_by_count_becomes_count_movement() {
        let d = vixen_effect_registry::by_data_type("WipeData").unwrap();
        let mut root = root_with(vec![wipe("Up", true)], 6);
        run(&mut root, 6, 7);
        assert_eq!(effect_params_from_xml(only_model(&root), d).string("WipeMovement"), "Count");
    }

    #[test]
    fn a_null_curve_name_survives_wipe_but_a_null_gradient_name_does_not() {
        let mut old = wipe("Up", true);
        *old.child_mut("Curve").unwrap().child_mut("_libraryReferenceName").unwrap() =
            XElement::nil(crate::values::NS_CURVES, "_libraryReferenceName");
        *old.child_mut("ColorGradient").unwrap().child_mut("_libraryReferenceName").unwrap() =
            XElement::nil(crate::values::NS_GRADIENTS, "_libraryReferenceName");
        let mut root = root_with(vec![old], 6);
        run(&mut root, 6, 7);
        let m = only_model(&root);
        assert!(m.child("Curve").unwrap().child("_libraryReferenceName").unwrap().nil);
        assert!(!m.child("ColorGradient").unwrap().child("_libraryReferenceName").unwrap().nil);
    }

    #[test]
    fn chase_and_spin_enable_a_nonzero_default_level() {
        for (clr, ty) in [("VixenModules.Effect.Chase", "ChaseData"), ("VixenModules.Effect.Spin", "SpinData")] {
            let ns = effect_ns(clr);
            let mut on = model(clr, ty);
            on.push(XElement::with_text(&ns, "DefaultLevel", "0.5"));
            let mut off = model(clr, ty);
            off.push(XElement::with_text(&ns, "DefaultLevel", "0"));
            let mut root = root_with(vec![on, off], 3);
            run(&mut root, 3, 4);
            let models: Vec<_> = root.child("_dataModels").unwrap().elements().collect();
            assert_eq!(child_value(models[0], "EnableDefaultLevel").as_deref(), Some("true"), "{ty}");
            assert_eq!(models[0].child("EnableDefaultLevel").unwrap().ns, ns);
            assert_eq!(child_value(models[1], "EnableDefaultLevel"), None, "{ty}");
        }
    }

    #[test]
    fn alternating_builds_pairs_in_reverse_order() {
        let clr = "VixenModules.Effect.Alternating";
        let ns = effect_ns(clr);
        let mut m = model(clr, "AlternatingData");
        m.push(XElement::with_text(&ns, "StaticColor1", "true"));
        m.push(rgb_to_xml(&ns, "Color1", Rgb::from_color(Color::RED)));
        m.push(XElement::with_text(&ns, "Level1", "0.5"));
        m.push(XElement::with_text(&ns, "StaticColor2", "false"));
        m.push(gradient_to_xml(&ns, "ColorGradient2", &ColorGradient::solid(Color::BLUE)));
        m.push(curve_to_xml(&ns, "Curve2", &Curve::flat(10.0)));
        m.push(XElement::with_text(&ns, "Enable", "false"));
        m.push(XElement::with_text(&ns, "GroupEffect", "4"));
        m.push(XElement::with_text(&ns, "Interval", "500"));
        let mut root = root_with(vec![m], 1);
        run(&mut root, 1, 2);
        let d = vixen_effect_registry::by_data_type("AlternatingData").unwrap();
        let p = effect_params_from_xml(only_model(&root), d);
        let pairs = p.gradient_levels("Colors");
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].color_gradient, ColorGradient::solid(Color::BLUE));
        assert_eq!(pairs[0].curve.points, Curve::flat(10.0).points);
        assert_eq!(pairs[1].color_gradient, ColorGradient::solid(Color::RED));
        assert_eq!(pairs[1].curve.points, vec![(0.0, 50.0), (100.0, 50.0)]);
        assert!(p.boolean("EnableStatic"));
        assert_eq!(p.int("GroupLevel"), 4);
        assert_eq!(p.int("Interval"), 500);
        assert_eq!(p.int("IntervalSkipCount"), 1);
    }

    #[test]
    fn snowflakes_and_fireworks_take_gradients() {
        let clr = "VixenModules.Effect.Snowflakes";
        let ns = effect_ns(clr);
        let mut m = model(clr, "SnowflakesData");
        m.push(DotNetColor::from_color(Color::RED).to_xml(&ns, "OuterColor"));
        m.push(DotNetColor::from_known(164).to_xml(&ns, "CenterColor"));
        m.push(XElement::with_text(&ns, "FlakeCount", "12"));
        let mut root = root_with(vec![m], 2);
        run(&mut root, 2, 3);
        let d = vixen_effect_registry::by_data_type("SnowflakesData").unwrap();
        let p = effect_params_from_xml(only_model(&root), d);
        assert_eq!(p.gradients("OutSideColor"), vec![ColorGradient::solid(Color::RED)]);
        assert_eq!(p.gradients("InnerColor"), vec![ColorGradient::solid(Color::WHITE)]);

        let clr = "VixenModules.Effect.Fireworks";
        let ns = effect_ns(clr);
        let mut m = model(clr, "FireworksData");
        m.push(crate::values::color_list_to_xml(&ns, "Colors", &[Color::RED, Color::BLUE]));
        m.push(XElement::with_text(&ns, "Particles", "40"));
        let mut root = root_with(vec![m], 2);
        run(&mut root, 2, 3);
        let d = vixen_effect_registry::by_data_type("FireworksData").unwrap();
        let p = params_from_xml(only_model(&root), d.params);
        assert_eq!(
            p.gradients("ColorGradients"),
            vec![ColorGradient::solid(Color::RED), ColorGradient::solid(Color::BLUE)]
        );
        assert_eq!(p.int("Particles"), 40);
    }

    #[test]
    fn set_position_controls_pan_and_tilt() {
        let clr = "VixenModules.Effect.SetPosition";
        let mut root = root_with(vec![model(clr, "SetPositionData")], 8);
        run(&mut root, 8, 9);
        let m = only_model(&root);
        assert_eq!(child_value(m, "EnablePan").as_deref(), Some("true"));
        assert_eq!(child_value(m, "EnableTilt").as_deref(), Some("true"));
        assert_eq!(root.attr("version"), Some("9"));
    }

    #[test]
    fn liquid_emitters_start_at_random_positions() {
        let clr = "VixenModules.Effect.Liquid";
        let ns = effect_ns(clr);
        let mut emitter = XElement::new(&ns, "EmitterData");
        emitter.push(XElement::with_text(&ns, "Lifetime", "1"));
        let mut list = XElement::new(&ns, "EmitterData");
        list.push(emitter);
        let mut m = model(clr, "LiquidData");
        m.push(list);
        let mut root = root_with(vec![m], 7);
        run(&mut root, 7, 8);
        let e = only_model(&root).child("EmitterData").unwrap().elements().next().unwrap();
        assert_eq!(child_value(e, "RandomStartingPosition").as_deref(), Some("true"));
    }

    #[test]
    fn nutcracker_effects_are_removed_and_reported() {
        let nut = model("VixenModules.Effect.Nutcracker", "NutcrackerModuleData");
        let keep = model("VixenModules.Effect.Pulse", "PulseData");
        let mut root = root_with(vec![nut, keep], 8);
        let mut surrogates = XElement::new("", "_effectNodeSurrogates");
        let mut s = XElement::new("", "EffectNodeSurrogate");
        s.push(XElement::with_text("", "TypeId", NUTCRACKER_TYPE_ID));
        surrogates.push(s);
        let mut s = XElement::new("", "EffectNodeSurrogate");
        s.push(XElement::with_text("", "TypeId", TYPE));
        surrogates.push(s);
        root.push(surrogates);
        let notes = run(&mut root, 8, 9);
        assert_eq!(root.child("_dataModels").unwrap().elements().count(), 1);
        assert_eq!(root.child("_effectNodeSurrogates").unwrap().elements().count(), 1);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("1 deprecated Nutcracker"), "{}", notes[0]);
    }

    #[test]
    fn version_0_makes_media_paths_relative() {
        let mut media = XElement::new("", "MediaSurrogate");
        media.push(XElement::with_text("", "FilePath", r"C:\Users\me\Song.mp3"));
        let mut list = XElement::new("", "_mediaSurrogates");
        list.push(media);
        let mut timing = XElement::new("", "_selectedTimingProviderSurrogate");
        timing.push(XElement::with_text("", "ProviderType", "Media"));
        timing.push(XElement::with_text("", "SourceName", r"C:\Users\me\Song.mp3"));
        let mut root = root_with(vec![], 0);
        root.push(list);
        root.push(timing);
        run(&mut root, 0, 1);
        let m = root.child("_mediaSurrogates").unwrap().elements().next().unwrap();
        assert_eq!(child_value(m, "FileName").as_deref(), Some("Song.mp3"));
        assert!(m.child("FilePath").is_none());
        assert_eq!(
            child_value(root.child("_selectedTimingProviderSurrogate").unwrap(), "SourceName").as_deref(),
            Some("Song.mp3")
        );
    }

    #[test]
    fn a_newer_file_is_left_alone_with_a_warning() {
        let mut root = root_with(vec![], 10);
        let notes = run(&mut root, 10, 9);
        assert_eq!(notes.len(), 1);
        assert_eq!(root.attr("version"), Some("10"));
    }

    #[test]
    fn lip_sync_without_a_library_is_left_in_place_with_a_note() {
        let clr = LIP_SYNC_NS;
        let mut root = root_with(vec![model(clr, "LipSyncData")], 5);
        let notes = run(&mut root, 5, 6);
        assert_eq!(notes.len(), 1);
        assert!(child_value(only_model(&root), "MappingType").is_none());
    }

    #[test]
    fn version_4_lip_sync_takes_matrix_settings_and_reports_the_maps() {
        let clr = LIP_SYNC_NS;
        let ns = effect_ns(clr);
        let named = |map: &str| {
            let mut m = model(clr, "LipSyncData");
            m.push(XElement::with_text(&ns, "PhonemeMapping", map));
            m
        };
        let mut root = root_with(vec![named("Matrix"), named("Face"), named("Matrix"), named("Gone")], 4);
        let mut maps = HashMap::new();
        maps.insert("Matrix".to_string(), LipSyncMapInfo { is_matrix: true, strings_are_rows: false });
        maps.insert("Face".to_string(), LipSyncMapInfo { is_matrix: false, strings_are_rows: false });
        let mut context =
            MigrationContext { maps: Some(Some(maps)), ..MigrationContext::none() };
        migrate(&mut root, 4, 5, &mut context).unwrap();
        assert_eq!(context.matrix_maps_named, ["Matrix"]);
        let models: Vec<&XElement> = root.child("_dataModels").unwrap().elements().collect();
        assert_eq!(models.len(), 4);
        for m in &models {
            let matrix = child_value(m, "PhonemeMapping").as_deref() == Some("Matrix");
            assert_eq!(child_value(m, "ScaleToGrid").as_deref(), matrix.then_some("true"));
            assert_eq!(child_value(m, "Orientation").as_deref(), matrix.then_some("Vertical"));
            assert_eq!(child_value(m, "Level").as_deref(), matrix.then_some("100"));
        }
    }

    #[test]
    fn lip_sync_face_definitions_come_from_the_library() {
        let clr = LIP_SYNC_NS;
        let ns = effect_ns(clr);
        let mut matrix = model(clr, "LipSyncData");
        matrix.push(XElement::with_text(&ns, "PhonemeMapping", "Matrix"));
        let mut face = model(clr, "LipSyncData");
        face.push(XElement::with_text(&ns, "PhonemeMapping", "Face"));
        let mut unknown = model(clr, "LipSyncData");
        unknown.push(XElement::with_text(&ns, "PhonemeMapping", "Missing"));
        let mut root = root_with(vec![matrix, face, unknown], 5);
        let mut maps = HashMap::new();
        maps.insert("Matrix".to_string(), LipSyncMapInfo { is_matrix: true, strings_are_rows: true });
        maps.insert("Face".to_string(), LipSyncMapInfo { is_matrix: false, strings_are_rows: false });
        let mut context = MigrationContext { maps: Some(Some(maps.clone())), ..MigrationContext::none() };
        migrate(&mut root, 5, 6, &mut context).unwrap();
        let types: Vec<_> =
            root.child("_dataModels").unwrap().elements().map(|m| child_value(m, "MappingType")).collect();
        assert_eq!(types, [None, Some("FaceDefinition".to_string()), Some("FaceDefinition".to_string())]);

        let mut matrix = model(clr, "LipSyncData");
        matrix.push(XElement::with_text(&ns, "Level", "0"));
        matrix.push(XElement::with_text(&ns, "PhonemeMapping", "Matrix"));
        let mut root = root_with(vec![matrix], 4);
        let mut context = MigrationContext { maps: Some(Some(maps)), ..MigrationContext::none() };
        migrate(&mut root, 4, 5, &mut context).unwrap();
        let m = only_model(&root);
        assert_eq!(child_value(m, "Level").as_deref(), Some("100"));
        assert_eq!(child_value(m, "ScalePercent").as_deref(), Some("100"));
        assert_eq!(child_value(m, "Orientation").as_deref(), Some("Horizontal"));
        assert_eq!(child_value(m, "ScaleToGrid").as_deref(), Some("true"));
    }
}
