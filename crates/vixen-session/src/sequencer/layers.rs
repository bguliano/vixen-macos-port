//! The Layers panel: `LayerEditorLayerService` and `LayerImportExportService`. F330 to F340. The list's data is `vixen_core::layer`.

use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;
use vixen_core::layer::{ChromaKeySettings, Layer, LayerType, MixingFilter, SequenceLayers};
use vixen_core::Color;
use vixen_io::file::write_atomic;
use vixen_io::sequence::mixing_filter_ids as ids;
use vixen_io::values::DotNetColor;
use vixen_io::IoError;

/// `Descriptor.TypeName`.
pub fn filter_type_name(filter: &MixingFilter) -> String {
    match filter {
        MixingFilter::ProportionalMix => "Proportional Mix",
        MixingFilter::HighestValue => "Highest Value (Default)",
        MixingFilter::Mask => "Mask",
        MixingFilter::MaskFill { .. } => "Mask And Fill",
        MixingFilter::MultiplyColor => "Multiply Color",
        MixingFilter::IntensityOverlay => "Intensity Overlay",
        MixingFilter::ColorChange => "Color Change",
        MixingFilter::LumaKey { .. } => "Luma Key",
        MixingFilter::ChromaKey(_) => "Chroma Key",
    }
    .to_string()
}

/// `FilterTypeId`, as the file holds it.
fn filter_type_id(filter: &MixingFilter) -> Uuid {
    vixen_io::sequence::mixing_filter_type_id(filter)
}

/// The Type combo's choices: every installed filter, with its default settings. The order is the service's, by name.
pub fn filter_choices() -> Vec<MixingFilter> {
    let mut list = vec![
        MixingFilter::ChromaKey(ChromaKeySettings::default()),
        MixingFilter::ColorChange,
        MixingFilter::HighestValue,
        MixingFilter::IntensityOverlay,
        MixingFilter::LumaKey { lower_limit: 0.0, upper_limit: 1.0 },
        MixingFilter::Mask,
        MixingFilter::MaskFill { exclude_zero_values: true, requires_mixing_partner: false },
        MixingFilter::MultiplyColor,
        MixingFilter::ProportionalMix,
    ];
    list.sort_by_key(filter_type_name);
    list
}

/// `HasSetup`: the Configuration button shows.
pub fn has_setup(filter: &MixingFilter) -> bool {
    matches!(filter, MixingFilter::LumaKey { .. } | MixingFilter::ChromaKey(_) | MixingFilter::MaskFill { .. })
}

/// `IsDefaultLayer`.
pub fn is_default(layer: &Layer) -> bool {
    layer.layer_type == LayerType::Default
}

/// `CreateUniqueLayerName` among other layers' names: `<name>`, else `<name> - 2`, `- 3`. Blank wants "Layer".
pub fn unique_layer_name(existing: &[&str], desired: &str) -> String {
    if desired.trim().is_empty() {
        return "Layer".to_string();
    }
    if !existing.contains(&desired) {
        return desired.to_string();
    }
    let mut counter = 2;
    loop {
        let candidate = format!("{desired} - {counter}");
        if !existing.contains(&candidate.as_str()) {
            return candidate;
        }
        counter += 1;
    }
}

/// `AddLayer`: at index 0, with the first available mixing filter, named as `SequenceLayers.AddLayer` names it: `EnsureUniqueName("Default")`
/// by exact comparison, so `Default - 2`, `Default - 3`, since the default layer has `Default` (marks-layers-analysis F56).
pub fn add_layer(layers: &mut SequenceLayers, filter: MixingFilter) -> Uuid {
    let names: Vec<String> = layers.layers.iter().map(|l| l.name.clone()).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let name = unique_layer_name(&names, "Default");
    let layer = Layer::standard(&name, filter);
    let id = layer.id;
    layers.add_layer(layer);
    id
}

/// `RemoveLayer`: refuses the default and an unknown one. Effects on it fall back to the default layer.
pub fn remove_layer(layers: &mut SequenceLayers, id: Uuid) -> bool {
    let Some(index) = layers.layers.iter().position(|l| l.id == id) else { return false };
    if is_default(&layers.layers[index]) {
        return false;
    }
    layers.layers.remove(index);
    layers.effect_layer_map.retain(|_, layer| *layer != id);
    layers.update_levels();
    true
}

/// `MoveLayer`: refuses the default, unknown, negative, past the last place before the default, and no move. F336.
pub fn move_layer(layers: &mut SequenceLayers, id: Uuid, new_index: i32) -> bool {
    let Some(old) = layers.layers.iter().position(|l| l.id == id) else { return false };
    if is_default(&layers.layers[old]) || new_index < 0 || new_index as usize >= layers.layers.len().saturating_sub(1) || old == new_index as usize {
        return false;
    }
    let layer = layers.layers.remove(old);
    layers.layers.insert(new_index as usize, layer);
    layers.update_levels();
    true
}

/// `QuickRenameLayer`: F335.
pub fn quick_rename(layers: &mut SequenceLayers, id: Uuid) -> bool {
    let Some(index) = layers.layers.iter().position(|l| l.id == id) else { return false };
    if is_default(&layers.layers[index]) {
        return false;
    }
    let Some(filter) = layers.layers[index].filter.clone() else { return false };
    let others: Vec<String> = layers.layers.iter().filter(|l| l.id != id).map(|l| l.name.clone()).collect();
    let others: Vec<&str> = others.iter().map(String::as_str).collect();
    layers.layers[index].name = unique_layer_name(&others, &filter_type_name(&filter));
    true
}

/// The Name field. KEEP (marks-layers-analysis F58): no validation at all, and the default layer can be renamed too.
pub fn rename_layer(layers: &mut SequenceLayers, id: Uuid, name: &str) -> bool {
    match layers.layers.iter_mut().find(|l| l.id == id) {
        Some(layer) => {
            layer.name = name.to_string();
            true
        }
        None => false,
    }
}

pub fn set_layer_filter(layers: &mut SequenceLayers, id: Uuid, filter: MixingFilter) -> bool {
    match layers.layers.iter_mut().find(|l| l.id == id && l.layer_type != LayerType::Default) {
        Some(layer) => {
            layer.filter = Some(filter);
            layer.filter_instance_id = Uuid::new_v4();
            true
        }
        None => false,
    }
}

/// `HasExportableLayers`.
pub fn has_exportable(layers: &SequenceLayers) -> bool {
    layers.layers.iter().any(|l| !is_default(l))
}

fn filter_data(filter: &MixingFilter) -> Option<(&'static str, Value)> {
    match filter {
        MixingFilter::LumaKey { lower_limit, upper_limit } => {
            Some(("VixenModules.LayerMixingFilter.LumaKey.LumaKeyData, Module.LayerMixingFilter.LumaKey", json!({ "LowerLimit": lower_limit, "UpperLimit": upper_limit })))
        }
        MixingFilter::MaskFill { exclude_zero_values, requires_mixing_partner } => Some((
            "VixenModules.LayerMixingFilter.MaskFill.MaskAndFillData, Module.LayerMixingFilter.MaskFill",
            json!({ "ExcludeZeroValues": exclude_zero_values, "RequiresMixingPartner": requires_mixing_partner }),
        )),
        MixingFilter::ChromaKey(s) => Some((
            "VixenModules.LayerMixingFilter.ChromaKey.ChromaKeyData, Module.LayerMixingFilter.ChromaKey",
            json!({
                "LowerLimit": s.lower_limit,
                "UpperLimit": s.upper_limit,
                "KeyColor": { "knownColor": 0, "name": null, "state": 2, "value": s.key_color.to_argb_u32() },
                "KeySaturation": s.key_saturation,
                "KeyHue": s.key_hue,
                "HueTolerance": s.hue_tolerance,
                "SaturationTolerance": s.saturation_tolerance,
                "TransparentOnZeroBrightness": s.transparent_on_zero_brightness,
            }),
        )),
        _ => None,
    }
}

/// `Export`: indented JSON of the non-default layers in list order. F340. Upstream's date is UTC with a Z.
pub fn export_v3l(layers: &SequenceLayers, path: &Path, exported_utc: &str) -> Result<(), IoError> {
    // Keep the declaration order of the upstream document/record contracts. Every
    // value is still JSON-encoded, including names containing quotes or newlines.
    let records: Vec<String> = layers.layers.iter().filter(|l| !is_default(l)).enumerate().map(|(order, layer)| {
        let filter = layer.filter.as_ref();
        let mut record = format!(
            "    {{\n      \"name\": {},\n      \"order\": {order},\n      \"filterTypeId\": {},\n      \"filterName\": {}",
            json!(layer.name),
            json!(filter.map_or_else(Uuid::nil, filter_type_id).to_string()),
            json!(filter.map_or_else(String::new, filter_type_name)),
        );
        if let Some((data_type, data)) = filter.and_then(filter_data) {
            // Module data keys use DataContractJsonSerializer's alphabetical order.
            let data = serde_json::to_string_pretty(&data).expect("JSON value serialization").replace('\n', "\n      ");
            record.push_str(&format!(",\n      \"filterDataType\": {},\n      \"filterData\": {data}", json!(data_type)));
        }
        record.push_str("\n    }");
        record
    }).collect();
    let rows = if records.is_empty() { "[]".into() } else { format!("[\n{}\n  ]", records.join(",\n")) };
    let text = format!("{{\n  \"format\": \"Vixen3Layers\",\n  \"version\": 1,\n  \"exportedUtc\": {},\n  \"layers\": {rows}\n}}", json!(exported_utc));
    write_atomic(path, text.as_bytes())
}

/// One layer a file holds that can come in.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerImportEntry {
    pub name: String,
    pub order: i64,
    pub filter: MixingFilter,
}

/// `LayerImportPlan`. F340.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LayerImportPlan {
    /// The reason the whole file failed.
    pub failure: Option<String>,
    pub importable: Vec<LayerImportEntry>,
    /// `(name, reason)`.
    pub skipped: Vec<(String, String)>,
}

fn failed(reason: String) -> LayerImportPlan {
    LayerImportPlan { failure: Some(reason), ..LayerImportPlan::default() }
}

/// The filter a record names, from its data. The three data classes have no `[DataContract]`, so `DataContractJsonSerializer` runs their
/// constructors and a key the data lacks keeps the constructor's value (KEEP, marks-layers-analysis F65); a key holding the wrong kind of
/// value can't be restored, and that record is skipped.
fn filter_from_record(type_id: Uuid, data: Option<&Value>) -> Result<Option<MixingFilter>, String> {
    let need_data = || data.filter(|d| !d.is_null()).ok_or_else(|| "The layer record does not contain filter configuration data.".to_string());
    let restore_error = |what: &str| format!("The filter configuration could not be restored: {what}");
    let number = |d: &Value, key: &str, default: f64| match d.get(key) {
        None => Ok(default),
        Some(v) => v.as_f64().ok_or_else(|| restore_error(&format!("'{key}' is not a number."))),
    };
    let flag = |d: &Value, key: &str, default: bool| match d.get(key) {
        None => Ok(default),
        Some(v) => v.as_bool().ok_or_else(|| restore_error(&format!("'{key}' is not true or false."))),
    };
    Ok(Some(if type_id == ids::PROPORTIONAL_MIX {
        MixingFilter::ProportionalMix
    } else if type_id == ids::HIGHEST_VALUE {
        MixingFilter::HighestValue
    } else if type_id == ids::MASK {
        MixingFilter::Mask
    } else if type_id == ids::MULTIPLY_COLOR {
        MixingFilter::MultiplyColor
    } else if type_id == ids::INTENSITY_OVERLAY {
        MixingFilter::IntensityOverlay
    } else if type_id == ids::COLOR_CHANGE {
        MixingFilter::ColorChange
    } else if type_id == ids::LUMA_KEY {
        let d = need_data()?;
        MixingFilter::LumaKey { lower_limit: number(d, "LowerLimit", 0.0)?, upper_limit: number(d, "UpperLimit", 1.0)? }
    } else if type_id == ids::MASK_FILL {
        let d = need_data()?;
        MixingFilter::MaskFill { exclude_zero_values: flag(d, "ExcludeZeroValues", true)?, requires_mixing_partner: flag(d, "RequiresMixingPartner", false)? }
    } else if type_id == ids::CHROMA_KEY {
        let d = need_data()?;
        let defaults = ChromaKeySettings::default();
        let key_color = match d.get("KeyColor") {
            None => defaults.key_color,
            Some(c) => dotnet_color(c).ok_or_else(|| restore_error("'KeyColor' is not a colour."))?,
        };
        MixingFilter::ChromaKey(ChromaKeySettings {
            lower_limit: number(d, "LowerLimit", defaults.lower_limit)?,
            upper_limit: number(d, "UpperLimit", defaults.upper_limit)?,
            key_color,
            key_saturation: number(d, "KeySaturation", defaults.key_saturation)?,
            key_hue: number(d, "KeyHue", defaults.key_hue)?,
            hue_tolerance: number(d, "HueTolerance", defaults.hue_tolerance)?,
            saturation_tolerance: number(d, "SaturationTolerance", defaults.saturation_tolerance)?,
            transparent_on_zero_brightness: flag(d, "TransparentOnZeroBrightness", defaults.transparent_on_zero_brightness)?,
        })
    } else {
        return Ok(None);
    }))
}

/// A `System.Drawing.Color` as `DataContractJsonSerializer` writes it: a known colour has `knownColor` set, `state` 1 and `value` 0; a plain
/// one `knownColor` 0, `state` 2 and the ARGB in `value`.
fn dotnet_color(c: &Value) -> Option<Color> {
    let o = c.as_object()?;
    let int = |key: &str| match o.get(key) {
        None | Some(Value::Null) => Some(0),
        Some(v) => v.as_i64(),
    };
    let name = match o.get("name") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return None,
    };
    let color = DotNetColor { known_color: int("knownColor")? as i32, name, state: int("state")? as i32, value: int("value")? };
    Some(color.to_color())
}

/// `DateTime` as `System.Text.Json` reads it: ISO 8601, a date with an optional time, fraction and offset.
fn is_json_date_time(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |from: usize, count: usize| b.get(from..from + count).is_some_and(|d| d.iter().all(u8::is_ascii_digit));
    if !(digits(0, 4) && b.get(4) == Some(&b'-') && digits(5, 2) && b.get(7) == Some(&b'-') && digits(8, 2)) {
        return false;
    }
    let (month, day) = ((b[5] - b'0') * 10 + (b[6] - b'0'), (b[8] - b'0') * 10 + (b[9] - b'0'));
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return false;
    }
    let mut i = 10;
    if i == b.len() {
        return true;
    }
    if b[i] != b'T' || !(digits(i + 1, 2) && b.get(i + 3) == Some(&b':') && digits(i + 4, 2)) {
        return false;
    }
    i += 6;
    if b.get(i) == Some(&b':') {
        if !digits(i + 1, 2) {
            return false;
        }
        i += 3;
        if b.get(i) == Some(&b'.') {
            let start = i + 1;
            i = start;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i == start {
                return false;
            }
        }
    }
    match b.get(i) {
        None => true,
        Some(b'Z') => i + 1 == b.len(),
        Some(b'+' | b'-') => digits(i + 1, 2) && b.get(i + 3) == Some(&b':') && digits(i + 4, 2) && i + 6 == b.len(),
        _ => false,
    }
}

/// What deserializing into `LayerExportDocument` refuses before any rule runs (F65 step 2): a known key with the wrong kind of value, a
/// malformed GUID or date, and, GUARD, a `null` record, which upstream's sort then trips over.
fn type_errors(doc: &serde_json::Map<String, Value>) -> Option<String> {
    let string_or_null = |v: &Value| v.is_string() || v.is_null();
    let int32 = |v: &Value| v.as_i64().is_some_and(|n| i32::try_from(n).is_ok());
    if doc.get("format").is_some_and(|v| !string_or_null(v)) {
        return Some("'format' must be a string.".to_string());
    }
    if doc.get("version").is_some_and(|v| !int32(v)) {
        return Some("'version' must be a whole number.".to_string());
    }
    if doc.get("exportedUtc").is_some_and(|v| !v.as_str().is_some_and(is_json_date_time)) {
        return Some("'exportedUtc' must be a date and time.".to_string());
    }
    match doc.get("layers") {
        None | Some(Value::Null) => None,
        Some(Value::Array(records)) => records.iter().find_map(|r| {
            let Some(r) = r.as_object() else { return Some("A layer record is null or not an object.".to_string()) };
            for key in ["name", "filterName", "filterDataType"] {
                if r.get(key).is_some_and(|v| !string_or_null(v)) {
                    return Some(format!("'{key}' must be a string."));
                }
            }
            if r.get("order").is_some_and(|v| !int32(v)) {
                return Some("'order' must be a whole number.".to_string());
            }
            if r.get("filterTypeId").is_some_and(|v| !v.as_str().is_some_and(|t| Uuid::parse_str(t).is_ok())) {
                return Some("'filterTypeId' must be a GUID.".to_string());
            }
            None
        }),
        Some(_) => Some("'layers' must be a list.".to_string()),
    }
}

/// `ReadImportPlanAsync`, marks-layers-analysis F65. The file is decoded as every import here is (F116): a byte order mark is accepted and a
/// bad byte becomes U+FFFD.
pub fn read_v3l_plan(path: &Path) -> LayerImportPlan {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => return failed(format!("The file could not be read: {e}")),
    };
    let text = vixen_io::marks_files::read_import_text(&bytes);
    let document: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return failed(format!("The file is not valid JSON: {e}")),
    };
    if document.is_null() {
        return failed("The file does not contain a layer export document.".to_string());
    }
    let Some(doc) = document.as_object() else { return failed("The file is not valid JSON: the document is not an object.".to_string()) };
    if let Some(problem) = type_errors(doc) {
        return failed(format!("The file is not valid JSON: {problem}"));
    }
    // Missing keys take the class's defaults, so a file without `format` or `version` passes.
    let format = match doc.get("format") {
        None => "Vixen3Layers",
        Some(v) => v.as_str().unwrap_or(""),
    };
    if format != "Vixen3Layers" {
        return failed(format!("'{format}' is not a supported layer export format."));
    }
    let version = doc.get("version").and_then(Value::as_i64).unwrap_or(1);
    if version != 1 {
        return failed(format!("Layer export version {version} is not supported."));
    }
    let empty = Vec::new();
    let records = match doc.get("layers") {
        Some(Value::Null) => return failed("The file does not contain any layer records.".to_string()),
        Some(Value::Array(records)) => records,
        _ => &empty,
    };
    let mut records: Vec<&Value> = records.iter().collect();
    records.sort_by_key(|r| r.get("order").and_then(Value::as_i64).unwrap_or(0));
    let mut plan = LayerImportPlan::default();
    for record in records {
        let type_id = record.get("filterTypeId").and_then(Value::as_str).and_then(|t| Uuid::parse_str(t).ok()).unwrap_or_else(Uuid::nil);
        // A default layer never has a filter: ignored, not skipped.
        if type_id.is_nil() {
            continue;
        }
        let name = record.get("name").and_then(Value::as_str).unwrap_or("");
        if name.trim().is_empty() {
            return failed("A layer record is missing its required name.".to_string());
        }
        let filter_name = record.get("filterName").and_then(Value::as_str).unwrap_or("");
        match filter_from_record(type_id, record.get("filterData")) {
            Ok(Some(filter)) => plan.importable.push(LayerImportEntry {
                name: name.to_string(),
                order: record.get("order").and_then(Value::as_i64).unwrap_or(0),
                filter,
            }),
            Ok(None) => plan.skipped.push((name.to_string(), format!("The '{filter_name}' mixing filter ({type_id}) is not installed."))),
            Err(reason) => plan.skipped.push((name.to_string(), reason)),
        }
    }
    if plan.importable.is_empty() {
        return failed("The file does not contain any layers that can be imported.".to_string());
    }
    plan
}

/// `Import`: entries go in from bottom to top, so the exported order sits above the existing layers, each with a unique name.
pub fn import_plan(layers: &mut SequenceLayers, plan: &LayerImportPlan) -> usize {
    if plan.failure.is_some() {
        return 0;
    }
    let mut ordered = plan.importable.clone();
    ordered.sort_by_key(|e| e.order);
    for entry in ordered.iter().rev() {
        let names: Vec<String> = layers.layers.iter().map(|l| l.name.clone()).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let name = unique_layer_name(&names, &entry.name);
        layers.add_layer(Layer::standard(&name, entry.filter.clone()));
    }
    ordered.len()
}

/// `The following layers cannot be imported:` with what's skipped. F338.
pub fn skipped_message(skipped: &[(String, String)]) -> String {
    let lines: Vec<String> = skipped.iter().map(|(name, reason)| format!("{name}: {reason}")).collect();
    format!("The following layers cannot be imported:\n\n{}\n\nContinue importing the remaining layers?", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-layers-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn names(l: &SequenceLayers) -> Vec<&str> {
        l.layers.iter().map(|x| x.name.as_str()).collect()
    }

    #[test]
    fn add_goes_on_top_and_remove_refuses_the_default() {
        let mut l = SequenceLayers::default();
        let default = l.layers[0].id;
        let a = add_layer(&mut l, MixingFilter::Mask);
        assert_eq!(l.layers[0].id, a);
        assert_eq!(l.layers[1].id, default);
        assert_eq!(l.layers[0].level, 1);
        assert!(!remove_layer(&mut l, default));
        l.assign_effect(Uuid::new_v4(), a);
        assert!(remove_layer(&mut l, a));
        assert!(l.effect_layer_map.is_empty());
        assert!(!remove_layer(&mut l, a));
    }

    #[test]
    fn move_rules() {
        let mut l = SequenceLayers::default();
        let default = l.layers[0].id;
        let a = add_layer(&mut l, MixingFilter::Mask);
        let b = add_layer(&mut l, MixingFilter::HighestValue);
        // b, a, default.
        assert!(!move_layer(&mut l, default, 0));
        assert!(!move_layer(&mut l, Uuid::new_v4(), 0));
        assert!(!move_layer(&mut l, a, -1));
        // The last row is the default's.
        assert!(!move_layer(&mut l, a, 2));
        assert!(!move_layer(&mut l, a, 1));
        assert!(move_layer(&mut l, a, 0));
        assert_eq!(l.layers[0].id, a);
        assert_eq!(l.layers[1].id, b);
        assert_eq!(l.layers[2].id, default);
    }

    #[test]
    fn names_stay_unique() {
        assert_eq!(unique_layer_name(&["A"], "B"), "B");
        assert_eq!(unique_layer_name(&["A"], "A"), "A - 2");
        assert_eq!(unique_layer_name(&["A", "A - 2"], "A"), "A - 3");
        assert_eq!(unique_layer_name(&[], " "), "Layer");
        let mut l = SequenceLayers::default();
        let a = add_layer(&mut l, MixingFilter::Mask);
        let b = add_layer(&mut l, MixingFilter::Mask);
        assert!(quick_rename(&mut l, a));
        assert_eq!(l.layer(&a).unwrap().name, "Mask");
        assert!(quick_rename(&mut l, b));
        assert_eq!(l.layer(&b).unwrap().name, "Mask - 2");
        let default = l.default_layer_id();
        assert!(!quick_rename(&mut l, default));
    }

    #[test]
    fn export_then_import_keeps_order_and_settings() {
        let dir = temp("trip");
        let path = dir.join("a.v3l");
        let mut l = SequenceLayers::default();
        assert!(!has_exportable(&l));
        add_layer(&mut l, MixingFilter::LumaKey { lower_limit: 0.25, upper_limit: 0.75 });
        add_layer(&mut l, MixingFilter::Mask);
        let top = l.layers[0].id;
        rename_layer(&mut l, top, "Top");
        assert!(has_exportable(&l));
        export_v3l(&l, &path, "2026-11-09T00:00:00Z").unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let fields = ["format", "version", "exportedUtc", "layers", "name", "order", "filterTypeId", "filterName", "filterDataType", "filterData"];
        let positions: Vec<_> = fields.iter().map(|field| text.find(&format!("\"{field}\":")).unwrap()).collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "upstream contract field order: {text}");
        let doc: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(doc["format"], "Vixen3Layers");
        assert_eq!(doc["version"], 1);
        assert_eq!(doc["layers"][0]["name"], "Top");
        assert_eq!(doc["layers"][0]["order"], 0);
        assert!(doc["layers"][0].get("filterData").is_none(), "no module data: omitted");
        assert_eq!(doc["layers"][1]["filterData"]["LowerLimit"], 0.25);

        let plan = read_v3l_plan(&path);
        assert!(plan.failure.is_none() && plan.skipped.is_empty());
        let mut target = SequenceLayers::default();
        add_layer(&mut target, MixingFilter::Mask);
        target.layers[0].name = "Top".to_string();
        assert_eq!(import_plan(&mut target, &plan), 2);
        // Above what's there, in the exported order, with a clash renamed.
        assert_eq!(names(&target), ["Top - 2", "Default - 2", "Top", "Default"]);
        assert_eq!(target.layers[1].filter, Some(MixingFilter::LumaKey { lower_limit: 0.25, upper_limit: 0.75 }));
    }

    #[test]
    fn import_failures_and_skips_say_why() {
        let dir = temp("plan");
        let path = dir.join("a.v3l");
        assert!(read_v3l_plan(&dir.join("none.v3l")).failure.unwrap().starts_with("The file could not be read:"));
        std::fs::write(&path, "nope").unwrap();
        assert!(read_v3l_plan(&path).failure.unwrap().starts_with("The file is not valid JSON:"));
        std::fs::write(&path, "null").unwrap();
        assert_eq!(read_v3l_plan(&path).failure.unwrap(), "The file does not contain a layer export document.");
        std::fs::write(&path, r#"{"format":"Other","version":1}"#).unwrap();
        assert_eq!(read_v3l_plan(&path).failure.unwrap(), "'Other' is not a supported layer export format.");
        std::fs::write(&path, r#"{"format":"Vixen3Layers","version":2}"#).unwrap();
        assert_eq!(read_v3l_plan(&path).failure.unwrap(), "Layer export version 2 is not supported.");
        // A missing list reads as empty, so it reaches "nothing to import"; only a null list has no records (marks-layers-analysis F65).
        std::fs::write(&path, r#"{"format":"Vixen3Layers","version":1}"#).unwrap();
        assert_eq!(read_v3l_plan(&path).failure.unwrap(), "The file does not contain any layers that can be imported.");
        std::fs::write(&path, r#"{"format":"Vixen3Layers","version":1,"layers":null}"#).unwrap();
        assert_eq!(read_v3l_plan(&path).failure.unwrap(), "The file does not contain any layer records.");

        let mask = ids::MASK.to_string();
        let luma = ids::LUMA_KEY.to_string();
        let ghost = Uuid::new_v4().to_string();
        std::fs::write(&path, format!(r#"{{"format":"Vixen3Layers","version":1,"layers":[{{"name":"","order":0,"filterTypeId":"{mask}"}}]}}"#)).unwrap();
        assert_eq!(read_v3l_plan(&path).failure.unwrap(), "A layer record is missing its required name.");

        std::fs::write(
            &path,
            format!(
                r#"{{"format":"Vixen3Layers","version":1,"layers":[
                {{"name":"Zero","order":0,"filterTypeId":"00000000-0000-0000-0000-000000000000"}},
                {{"name":"Ghost","order":1,"filterTypeId":"{ghost}","filterName":"Gone"}},
                {{"name":"NoData","order":2,"filterTypeId":"{luma}"}},
                {{"name":"Good","order":3,"filterTypeId":"{mask}"}}]}}"#
            ),
        )
        .unwrap();
        let plan = read_v3l_plan(&path);
        assert_eq!(plan.importable.len(), 1);
        assert_eq!(plan.skipped[0].1, format!("The 'Gone' mixing filter ({ghost}) is not installed."));
        assert_eq!(plan.skipped[1].1, "The layer record does not contain filter configuration data.");
        assert!(skipped_message(&plan.skipped).starts_with("The following layers cannot be imported:\n\nGhost: "));
    }

    #[test]
    fn setup_buttons_and_the_type_choices() {
        assert!(has_setup(&MixingFilter::LumaKey { lower_limit: 0.0, upper_limit: 1.0 }));
        assert!(!has_setup(&MixingFilter::Mask));
        let choices = filter_choices();
        assert_eq!(choices.len(), 9);
        assert_eq!(filter_type_name(&choices[0]), "Chroma Key");
    }
}

/// What `docs/spec/marks-layers-analysis.md` F56, F58 and F65 pin, beyond the shell's own tests.
#[cfg(test)]
mod layer_spec_tests {
    use super::*;
    use std::path::PathBuf;

    fn temp(name: &str, text: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("vixen-layers-spec-{}-{name}.v3l", std::process::id()));
        std::fs::write(&path, text).unwrap();
        path
    }

    fn plan(name: &str, json: &str) -> LayerImportPlan {
        read_v3l_plan(&temp(name, json.as_bytes()))
    }

    const LUMA: &str = "7583f7d4-df99-4492-8d10-257c2677ff7b";
    const MASK_FILL: &str = "24878b18-14f8-458e-abaa-3a9c9a34931c";
    const CHROMA: &str = "2d467733-1a64-45ad-bcdc-84bf074747e6";

    #[test]
    fn added_layers_are_named_after_the_default() {
        let mut l = SequenceLayers::default();
        add_layer(&mut l, MixingFilter::Mask);
        add_layer(&mut l, MixingFilter::Mask);
        let names: Vec<&str> = l.layers.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["Default - 3", "Default - 2", "Default"]);
    }

    #[test]
    fn the_default_layer_can_be_renamed() {
        let mut l = SequenceLayers::default();
        let default = l.layers[0].id;
        assert!(rename_layer(&mut l, default, "Base"));
        assert_eq!(l.layers[0].name, "Base");
        // With the default renamed, the next added layer takes the plain name.
        add_layer(&mut l, MixingFilter::Mask);
        assert_eq!(l.layers[0].name, "Default");
    }

    #[test]
    fn missing_keys_keep_the_constructors_values() {
        let p = plan("luma", &format!(r#"{{"layers":[{{"name":"A","filterTypeId":"{LUMA}","filterData":{{"LowerLimit":0.3}}}}]}}"#));
        assert_eq!(p.failure, None, "missing format and version take the defaults");
        assert_eq!(p.importable[0].filter, MixingFilter::LumaKey { lower_limit: 0.3, upper_limit: 1.0 });
        let p = plan("maskfill", &format!(r#"{{"layers":[{{"name":"A","filterTypeId":"{MASK_FILL}","filterData":{{}}}}]}}"#));
        assert_eq!(p.importable[0].filter, MixingFilter::MaskFill { exclude_zero_values: true, requires_mixing_partner: false });
        let p = plan("wrongtype", &format!(r#"{{"layers":[{{"name":"A","filterTypeId":"{LUMA}","filterData":{{"LowerLimit":"x"}}}},{{"name":"B","filterTypeId":"{MASK_FILL}","filterData":{{}}}}]}}"#));
        assert_eq!(p.skipped.len(), 1);
        assert!(p.skipped[0].1.starts_with("The filter configuration could not be restored:"));
    }

    #[test]
    fn a_known_key_colour_reads_as_that_colour() {
        let lime = vixen_io::values::known_color_named("Lime").unwrap();
        let p = plan("known", &format!(r#"{{"layers":[{{"name":"A","filterTypeId":"{CHROMA}","filterData":{{"KeyColor":{{"knownColor":{lime},"name":null,"state":1,"value":0}}}}}}]}}"#));
        let MixingFilter::ChromaKey(c) = &p.importable[0].filter else { panic!() };
        assert_eq!(c.key_color, Color::LIME);
        let p = plan("argb", &format!(r#"{{"layers":[{{"name":"A","filterTypeId":"{CHROMA}","filterData":{{"KeyColor":{{"knownColor":0,"name":null,"state":2,"value":4294901760}}}}}}]}}"#));
        let MixingFilter::ChromaKey(c) = &p.importable[0].filter else { panic!() };
        assert_eq!(c.key_color, Color::rgb(255, 0, 0));
    }

    #[test]
    fn type_errors_fail_the_file_as_invalid_json() {
        for (name, json) in [
            ("version", r#"{"version":"1","layers":[]}"#.to_string()),
            ("date", r#"{"exportedUtc":"yesterday","layers":[]}"#.to_string()),
            ("null-record", r#"{"layers":[null]}"#.to_string()),
            ("guid", r#"{"layers":[{"name":"A","filterTypeId":"nope"}]}"#.to_string()),
            ("name", format!(r#"{{"layers":[{{"name":5,"filterTypeId":"{LUMA}"}}]}}"#)),
        ] {
            let p = plan(name, &json);
            assert!(p.failure.as_deref().is_some_and(|f| f.starts_with("The file is not valid JSON:")), "{name}: {:?}", p.failure);
        }
        let ok = plan("date-ok", &format!(r#"{{"exportedUtc":"2026-11-09T12:34:56.1234567Z","layers":[{{"name":"A","filterTypeId":"{MASK_FILL}","filterData":{{}}}}]}}"#));
        assert_eq!(ok.failure, None);
    }

    #[test]
    fn missing_layers_and_only_skips_have_nothing_to_import() {
        let p = plan("none", r#"{"format":"Vixen3Layers","version":1}"#);
        assert_eq!(p.failure.as_deref(), Some("The file does not contain any layers that can be imported."));
        let p = plan("null-layers", r#"{"layers":null}"#);
        assert_eq!(p.failure.as_deref(), Some("The file does not contain any layer records."));
        let p = plan("format", r#"{"format":"Other","layers":[]}"#);
        assert_eq!(p.failure.as_deref(), Some("'Other' is not a supported layer export format."));
    }

    #[test]
    fn a_byte_order_mark_is_accepted() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(format!(r#"{{"layers":[{{"name":"A","filterTypeId":"{MASK_FILL}","filterData":{{}}}}]}}"#).as_bytes());
        let p = read_v3l_plan(&temp("bom", &bytes));
        assert_eq!(p.failure, None);
        assert_eq!(p.importable.len(), 1);
    }
}
