//! Sequences from the user's real show: load, save, and compare with what the file said.

mod common;

use common::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use uuid::Uuid;
use vixen_core::Color;
use vixen_effects::framework::{ParamKind, ParamValue};
use vixen_io::sequence::{sequence_to_document, typed_only_document, CURRENT_SEQUENCE_VERSION};
use vixen_io::xml::{first_loss, same_tree};
use vixen_io::{load_sequence, new_sequence, save_sequence, EffectNode, TimedSequence};
use vixen_xml::{XElement, XSI_NS};

/// The effect types the prompt lists, all of which the user's files use.
const REAL_EFFECTS: [&str; 16] = [
    "SetLevelData",
    "PulseData",
    "WipeData",
    "DissolveData",
    "TwinkleData",
    "ChaseData",
    "SpinData",
    "PlasmaData",
    "ButterflyData",
    "BarsData",
    "CandleData",
    "StrobeData",
    "VerticalMeterData",
    "SpiralData",
    "ColorWashData",
    "AlternatingData",
];

fn count(root: &XElement, member: &str) -> usize {
    root.child(member).map_or(0, |m| m.elements().count())
}

fn data_type_of(e: &EffectNode) -> &'static str {
    vixen_effect_registry::by_type_id(&e.type_id).map_or("?", |d| d.data_type)
}

#[test]
fn every_current_sequence_loads_with_the_counts_the_xml_has() {
    let Some(root) = data_root() else { return };
    let files = current_sequences(&root);
    if is_default_data(&root) {
        assert_eq!(files.len(), 17);
    }
    let mut seen_types = BTreeSet::new();
    for path in &files {
        let xml = read(path);
        let seq = load_sequence(path).unwrap_or_else(|e| panic!("{e}"));
        let name = file_name(path);

        assert_eq!(seq.effects.len(), count(&xml, "_effectNodeSurrogates"), "{name}: effects");
        assert_eq!(seq.media.len(), count(&xml, "_mediaSurrogates"), "{name}: media");
        assert_eq!(seq.filters.len(), count(&xml, "_filterNodeSurrogates"), "{name}: filters");
        assert_eq!(
            seq.effects.len() + seq.media.len() + seq.filters.len(),
            count(&xml, "_dataModels"),
            "{name}: data models"
        );
        assert_eq!(seq.row_settings.len(), count(&xml, "RowSettings"), "{name}: row settings");

        let marks = xml.child("LabeledMarkCollections").unwrap();
        assert_eq!(seq.marks.len(), marks.elements().count(), "{name}: mark collections");
        let total: usize = marks.elements().map(|c| c.child("Marks").unwrap().elements().count()).sum();
        assert_eq!(seq.marks.iter().map(|c| c.marks.len()).sum::<usize>(), total, "{name}: marks");

        assert_eq!(seq.length.to_xml(), xml.child_text("Length").unwrap(), "{name}: length");
        assert_eq!(seq.version, CURRENT_SEQUENCE_VERSION, "{name}");
        assert_eq!(seq.source_version.to_string(), xml.attr("version").unwrap(), "{name}");

        // Each media file names something; each effect targets a node.
        for m in &seq.media {
            assert!(m.data.is_some(), "{name}: media has its module data");
            assert!(m.file_name.contains('.'), "{name}: {}", m.file_name);
        }
        for e in &seq.effects {
            assert_eq!(e.targets.len(), 1, "{name}: real files have one target each");
            let d = vixen_effect_registry::by_type_id(&e.type_id)
                .unwrap_or_else(|| panic!("{name}: unknown effect {}", e.type_id));
            seen_types.insert(d.data_type);
            assert_eq!(e.params.values.len(), d.params.len());
        }
    }
    if is_default_data(&root) {
        for t in REAL_EFFECTS {
            assert!(seen_types.contains(t), "no sequence uses {t}");
        }
    }
}

/// Saves a sequence to a temp file, re-reads it, and compares with the original tree. Only the root's version
/// may differ, and only because loading migrates.
fn assert_round_trips(path: &Path, dir: &TempDir) -> (TimedSequence, XElement) {
    let original = read(path);
    let seq = load_sequence(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let out = dir.path(&file_name(path));
    let _ = std::fs::remove_file(&out);
    save_sequence(&seq, &out).unwrap();
    let saved = read(&out);
    let expected = with_version(original, saved.attr("version").unwrap());

    let mut added = Vec::new();
    let loss = first_loss(&expected, &saved, "", &mut added);
    assert_eq!(loss, None, "{}", path.display());
    // Real Vixen adds members when upgrading. This port leaves them out until a value changes, so the plain
    // comparison must hold too, except where a migrator itself added something.
    let unexplained: Vec<_> =
        added.iter().filter(|a| !a.ends_with("EnablePan") && !a.ends_with("EnableTilt")).collect();
    assert!(unexplained.is_empty(), "{}: added {unexplained:?}", path.display());
    if added.is_empty() {
        assert_eq!(expected.first_difference(&saved, String::new()), None, "{}", path.display());
    }
    (seq, saved)
}

#[test]
fn every_current_sequence_saves_with_nothing_lost_or_changed() {
    let Some(root) = data_root() else { return };
    let dir = TempDir::new("round-trip");
    for path in current_sequences(&root) {
        let (seq, saved) = assert_round_trips(&path, &dir);
        // Nothing needed the catch-all: typed values and their per-object notes reproduced the whole document.
        let report = seq.fidelity_report();
        assert!(report.root_members.is_empty(), "{}: {:?}", path.display(), report.root_members);
        assert_eq!(report.orphan_models, 0);
        assert_eq!(report.absent_models, 0);

        // A second generation is the same as the first: saving is stable.
        let out = dir.path("again.tim");
        let _ = std::fs::remove_file(&out);
        let again = load_sequence(&dir.path(&file_name(&path))).unwrap();
        save_sequence(&again, &out).unwrap();
        assert_eq!(saved.first_difference(&read(&out), String::new()), None, "{}", path.display());
    }
}

#[test]
fn saved_text_is_close_to_what_vixen_wrote() {
    let Some(root) = data_root() else { return };
    let mut total = 0usize;
    let mut same_size = 0usize;
    let files = current_sequences(&root);
    for path in &files {
        let original = std::fs::read_to_string(path).unwrap();
        let seq = load_sequence(path).unwrap();
        let text = sequence_to_document(&seq).unwrap();
        // The same line structure, so a diff of two files is meaningful. A size difference is only `<x></x>`
        // against `<x />` in kept parts, the version attribute, and the prefix DataContractSerializer declares
        // on an empty list that nothing then uses.
        assert_eq!(text.lines().count(), original.lines().count(), "{}", path.display());
        let difference = text.len().abs_diff(original.len());
        assert!(
            difference < 500 + original.len() / 100,
            "{}: {} bytes against {}",
            path.display(),
            text.len(),
            original.len()
        );
        if text == original {
            same_size += 1;
        }
        total += 1;
    }
    eprintln!("{same_size} of {total} files are byte-identical after a load and save");
}

#[test]
fn effect_parameters_are_typed_not_raw() {
    let Some(root) = data_root() else { return };
    let mut members = 0usize;
    let mut raw_where_typed = Vec::new();
    let mut unknown = BTreeMap::<String, usize>::new();
    for path in current_sequences(&root) {
        let seq = load_sequence(&path).unwrap();
        for e in &seq.effects {
            for (d, v) in e.params.desc.iter().zip(&e.params.values) {
                if d.xml.is_empty() {
                    continue;
                }
                members += 1;
                let typed = matches!(
                    (d.kind, v),
                    (ParamKind::Bool, ParamValue::Bool(_))
                        | (ParamKind::Int, ParamValue::Int(_))
                        | (ParamKind::Long, ParamValue::Long(_))
                        | (ParamKind::Double, ParamValue::Double(_))
                        | (ParamKind::Str, ParamValue::Str(_) | ParamValue::None)
                        | (ParamKind::Enum(_), ParamValue::Enum(_))
                        | (ParamKind::Curve, ParamValue::Curve(_))
                        | (ParamKind::Gradient, ParamValue::Gradient(_))
                        | (ParamKind::GradientList, ParamValue::GradientList(_))
                        | (ParamKind::GradientLevels, ParamValue::GradientLevels(_))
                        | (ParamKind::Color, ParamValue::Color(_))
                        | (ParamKind::Rgb, ParamValue::Rgb(_))
                        | (ParamKind::Guid, ParamValue::Guid(_))
                );
                if !typed {
                    raw_where_typed.push(format!("{}.{} = {v:?}", data_type_of(e), d.xml));
                }
            }
            for (n, _) in e.params.unknown_members.iter().filter(|(n, _)| !vixen_io::is_hint(n)) {
                *unknown.entry(format!("{}.{n}", data_type_of(e))).or_default() += 1;
            }
        }
    }
    if is_default_data(&root) {
        assert!(members > 10_000, "{members}");
    }
    raw_where_typed.sort();
    raw_where_typed.dedup();
    assert!(raw_where_typed.is_empty(), "{raw_where_typed:#?}");
    // Every member these files hold is in a table. If this fails, a table misses or misnames a member.
    assert!(unknown.is_empty(), "members not in the tables: {unknown:#?}");
}

#[test]
fn enum_values_in_the_files_are_ones_the_tables_know() {
    let Some(root) = data_root() else { return };
    let mut unknown = BTreeSet::new();
    for path in current_sequences(&root) {
        for e in &load_sequence(&path).unwrap().effects {
            for (d, v) in e.params.desc.iter().zip(&e.params.values) {
                if let (ParamKind::Enum(table), ParamValue::Enum(name)) = (d.kind, v) {
                    if !table.members.iter().any(|m| m.name == name) {
                        unknown.insert(format!("{}.{} = {name}", data_type_of(e), d.xml));
                    }
                }
            }
        }
    }
    assert!(unknown.is_empty(), "{unknown:#?}");
}

/// Compares a tree with typed encoding of it, tolerating only what typed values cannot hold.
fn typed_difference(original: &XElement, typed: &XElement, path: &str) -> Option<String> {
    let path = format!("{path}/{}", original.local);
    if original.local == "_libraryReferenceName" && original.local == typed.local {
        // Nil and empty are the same library name.
        return (original.text().trim() != typed.text().trim()).then(|| format!("{path}: library name"));
    }
    if is_color_element(original) && is_color_element(typed) {
        // Known and ARGB forms of the same four channels.
        let a = vixen_io::values::color_from_xml(original);
        let b = vixen_io::values::color_from_xml(typed);
        return (a != b).then(|| format!("{path}: {a:?} became {b:?}"));
    }
    if original.local != typed.local
        || original.ns != typed.ns
        || original.nil != typed.nil
        || original.xsi_type != typed.xsi_type
    {
        return Some(format!(
            "{path}: became {{{}}}{} nil={} type={:?}",
            typed.ns, typed.local, typed.nil, typed.xsi_type
        ));
    }
    if original.attrs.len() != typed.attrs.len() || !original.attrs.iter().all(|a| typed.attrs.contains(a)) {
        return Some(format!("{path}: attributes"));
    }
    let (a, b) = (original.text(), typed.text());
    let (a, b) = (a.trim(), b.trim());
    if a != b {
        // Two spellings of one double, from .NET Framework and .NET Core.
        let same_number =
            matches!((vixen_io::dcs::parse_double(a), vixen_io::dcs::parse_double(b)), (Some(x), Some(y)) if x == y);
        if !same_number {
            return Some(format!("{path}: text {a:?} became {b:?}"));
        }
    }
    let originals: Vec<&XElement> = original.elements().collect();
    let typeds: Vec<&XElement> = typed.elements().collect();
    let mut t = 0;
    for o in &originals {
        // Typed encoding always writes everything, so it can have members an older file lacks.
        while t < typeds.len() && typeds[t].local != o.local && !originals.iter().any(|x| x.local == typeds[t].local) {
            t += 1;
        }
        let Some(found) = typeds.get(t) else { return Some(format!("{path}: lost {}", o.local)) };
        if found.local != o.local {
            return Some(format!("{path}: expected {} found {}", o.local, found.local));
        }
        if let Some(d) = typed_difference(o, found, &path) {
            return Some(d);
        }
        t += 1;
    }
    if t < typeds.len() && typeds[t..].iter().any(|x| originals.iter().any(|o| o.local == x.local)) {
        return Some(format!("{path}: extra list items"));
    }
    None
}

fn is_color_element(e: &XElement) -> bool {
    e.child("knownColor").is_some() && e.child("state").is_some() && e.child("value").is_some()
}

#[test]
fn typed_values_alone_hold_all_the_meaning() {
    let Some(root) = data_root() else { return };
    for path in current_sequences(&root) {
        let original = read(&path);
        let seq = load_sequence(&path).unwrap();
        let typed = typed_only_document(&seq).unwrap();
        let expected = with_version(original, typed.attr("version").unwrap());
        assert_eq!(typed_difference(&expected, &typed, ""), None, "{}", path.display());
    }
}

#[test]
fn hints_are_rare_and_explained() {
    let Some(root) = data_root() else { return };
    let mut members = 0usize;
    let mut hints = BTreeMap::<String, usize>::new();
    for path in current_sequences(&root) {
        let seq = load_sequence(&path).unwrap();
        for e in &seq.effects {
            members += e.params.desc.len();
            for m in vixen_io::params::hinted_members(&e.params) {
                *hints.entry(format!("{}.{m}", data_type_of(e))).or_default() += 1;
            }
        }
    }
    let total: usize = hints.values().sum();
    eprintln!("{total} hints over {members} members: {hints:#?}");
    assert!(total * 10 < members, "{total} of {members}");
}

#[test]
fn editing_one_parameter_changes_only_that_member() {
    let Some(root) = data_root() else { return };
    let dir = TempDir::new("edit-one");
    let Some(path) = current_sequences(&root).into_iter().find(|p| file_name(p) == "Carey Grinch.tim") else { return };
    let original = read(&path);
    let mut seq = load_sequence(&path).unwrap();
    let index = seq.effects.iter().position(|e| data_type_of(e) == "WipeData").unwrap();
    let id = seq.effects[index].instance_id;
    let old_pass_count = seq.effects[index].params.int("PassCount");
    seq.effects[index].params.set("PassCount", ParamValue::Int(old_pass_count + 7));

    let out = dir.path("edited.tim");
    save_sequence(&seq, &out).unwrap();
    let saved = read(&out);

    let find = |root: &XElement| -> XElement {
        root.child("_dataModels")
            .unwrap()
            .elements()
            .find(|m| m.child_text("ModuleInstanceId").as_deref() == Some(id.to_string().as_str()))
            .unwrap()
            .clone()
    };
    let before = find(&original);
    let after = find(&saved);
    assert_eq!(after.child_text("PassCount"), Some((old_pass_count + 7).to_string()));
    let mut restored = after.clone();
    *restored.child_mut("PassCount").unwrap() = before.child("PassCount").unwrap().clone();
    assert_eq!(before.first_difference(&restored, String::new()), None);

    // Nothing else in the file moved, and the file still reads.
    let mut back = with_version(saved.clone(), original.attr("version").unwrap());
    let models = back.child_mut("_dataModels").unwrap();
    for m in models.elements_mut() {
        if m.child_text("ModuleInstanceId").as_deref() == Some(id.to_string().as_str()) {
            *m = before.clone();
        }
    }
    assert_eq!(original.first_difference(&back, String::new()), None);
    let reloaded = load_sequence(&out).unwrap();
    assert_eq!(reloaded.effects[index].params.int("PassCount"), old_pass_count + 7);
}

#[test]
fn edited_values_of_every_kind_reach_the_file() {
    let Some(root) = data_root() else { return };
    let dir = TempDir::new("edit-kinds");
    let path = current_sequences(&root).into_iter().max_by_key(|p| std::fs::metadata(p).unwrap().len()).unwrap();
    let mut seq = load_sequence(&path).unwrap();
    let mut edited: Vec<(Uuid, usize, ParamValue)> = Vec::new();
    for e in &mut seq.effects {
        for i in 0..e.params.values.len() {
            let new = match &e.params.values[i] {
                ParamValue::Bool(b) => ParamValue::Bool(!b),
                ParamValue::Int(v) => ParamValue::Int(v + 1),
                // Stays in range, since Twinkle clamps a level above 1 when it reads the file.
                ParamValue::Double(v) => ParamValue::Double(v * 0.5 + 0.125),
                ParamValue::Curve(c) => {
                    let mut c = c.clone();
                    c.points.push((100.0, 42.0));
                    ParamValue::Curve(c)
                }
                ParamValue::Gradient(g) => {
                    let mut g = g.clone();
                    g.title = Some("edited".to_string());
                    ParamValue::Gradient(g)
                }
                ParamValue::Color(_) => ParamValue::Color(Color::rgb(1, 2, 3)),
                ParamValue::Rgb(_) => ParamValue::Rgb(vixen_core::color::Rgb { r: 0.25, g: 0.5, b: 0.75 }),
                _ => continue,
            };
            if e.params.desc[i].xml.is_empty() {
                continue;
            }
            e.params.values[i] = new.clone();
            edited.push((e.instance_id, i, new));
        }
    }
    let out = dir.path("edited.tim");
    save_sequence(&seq, &out).unwrap();
    let back = load_sequence(&out).unwrap();
    let by_id: std::collections::HashMap<Uuid, &EffectNode> = back.effects.iter().map(|e| (e.instance_id, e)).collect();
    assert!(!edited.is_empty());
    for (id, i, value) in edited {
        let e = by_id[&id];
        assert_eq!(e.params.values[i], value, "{}.{}", data_type_of(e), e.params.desc[i].xml);
    }
}

#[test]
fn timing_and_layers_read_correctly() {
    let Some(root) = data_root() else { return };
    let Some(path) = current_sequences(&root).into_iter().find(|p| file_name(p) == "Carey Grinch.tim") else { return };
    let seq = load_sequence(&path).unwrap();
    assert_eq!(seq.selected_timing_provider.provider_type.as_deref(), Some("Media"));
    assert_eq!(seq.selected_timing_provider.source_name.as_deref(), Some("Carey Grinch.mp3"));
    assert_eq!(seq.media[0].file_name, "Carey Grinch.mp3");
    assert_eq!(seq.media[0].data.as_ref().unwrap().xsi_type.as_ref().unwrap().1, "AudioData");
    assert_eq!(seq.layers.layers.len(), 2);
    let top = &seq.layers.layers[0];
    assert_eq!(top.name, "Intensity Overlay");
    assert_eq!(top.level, 1);
    assert_eq!(top.filter, Some(vixen_core::MixingFilter::IntensityOverlay));
    assert!(top.requires_mixing_partner());
    assert!(seq.layers.layers[1].filter.is_none());
    assert_eq!(seq.layers.effect_layer_map.len(), 27);
    assert_eq!(seq.default_playback_end_time, None);
    assert_eq!(seq.default_row_height, 40);
    assert_eq!(seq.row_settings[0].0, "292EA3DA10767A6C845F13A2A946E1E2");
    assert_eq!(seq.row_settings[0].1.row_height, 40);
}

#[test]
fn luma_key_data_loads_from_the_files_that_have_it() {
    let Some(root) = data_root() else { return };
    let mut found = 0;
    for path in current_sequences(&root) {
        let seq = load_sequence(&path).unwrap();
        for l in &seq.layers.layers {
            if let Some(vixen_core::MixingFilter::LumaKey { lower_limit, upper_limit }) = l.filter {
                assert!(lower_limit <= upper_limit, "{}", path.display());
                found += 1;
            }
        }
    }
    eprintln!("{found} luma key layers");
}

#[test]
fn every_backup_round_trips() {
    let Some(root) = data_root() else { return };
    let dir = TempDir::new("backups");
    let mut count = 0;
    for path in all_sequence_files(&root) {
        // Versions 8 and 9 compare exactly. An older file changes shape in migration, so it only has to load.
        let version = read(&path).attr("version").map(str::to_string);
        if matches!(version.as_deref(), Some("8" | "9")) {
            assert_round_trips(&path, &dir);
        } else {
            load_sequence(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        }
        count += 1;
    }
    eprintln!("{count} sequence files checked");
}

#[test]
fn saved_files_use_the_writer_real_vixen_can_read_back() {
    let Some(root) = data_root() else { return };
    let path = current_sequences(&root).remove(0);
    let seq = load_sequence(&path).unwrap();
    let text = sequence_to_document(&seq).unwrap();
    assert!(text.starts_with("\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<TimedSequenceData version=\"9\" xmlns:a=\"http://www.w3.org/2001/XMLSchema\" xmlns:i=\"http://www.w3.org/2001/XMLSchema-instance\""));
    // DataContractSerializer needs `i:type` on array items of interface types.
    assert!(text.contains(&format!("xmlns:i=\"{XSI_NS}\"")));
    assert!(text.contains("i:type=\"d2p1:"));
}

#[test]
fn a_new_sequence_saves_and_loads() {
    let dir = TempDir::new("new");
    let mut seq = new_sequence();
    seq.length = vixen_core::TimeSpan::from_secs(60.0);
    let d = vixen_effect_registry::by_data_type("PulseData").unwrap();
    seq.effects.push(EffectNode {
        start: vixen_core::TimeSpan::from_ms_i(500),
        time_span: vixen_core::TimeSpan::from_ms_i(1500),
        type_id: d.type_id,
        instance_id: Uuid::new_v4(),
        targets: vec![(Uuid::new_v4(), "Star".to_string())],
        params: d.new_params(),
    });
    let out = dir.path("New.tim");
    save_sequence(&seq, &out).unwrap();
    let back = load_sequence(&out).unwrap();
    assert_eq!(back.effects.len(), 1);
    assert_eq!(back.effects[0].targets, seq.effects[0].targets);
    assert_eq!(back.effects[0].params.get("LevelCurve"), seq.effects[0].params.get("LevelCurve"));
    assert_eq!(back.length, seq.length);
    assert_eq!(back.time_per_pixel, vixen_core::TimeSpan::MIN);
    assert_eq!(back.default_playback_start_time, Some(vixen_core::TimeSpan::ZERO));
    assert_eq!(back.layers.layers.len(), 1);
    assert_eq!(back.layers.layers[0].name, "Default");
    let xml = read(&out);
    assert!(same_tree(&xml, &read(&out)));
    // Every effect member was written, as a new object would be.
    let model = xml.child("_dataModels").unwrap().elements().next().unwrap();
    for p in d.params.iter().filter(|p| !p.xml.is_empty()) {
        assert!(model.child(p.xml).is_some(), "{}", p.xml);
    }
}

#[test]
fn an_unknown_effect_type_comes_back_unchanged() {
    let dir = TempDir::new("unknown");
    let text = format!(
        r#"<TimedSequenceData version="9" xmlns:a="http://www.w3.org/2001/XMLSchema" xmlns:i="{XSI_NS}" xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed">
  <ModuleInstanceId xmlns="">6879a66e-7199-45ae-ab30-874d6be50246</ModuleInstanceId>
  <ModuleTypeId xmlns="">296bdba2-9bf3-4bff-a9f2-13efac5c8ecb</ModuleTypeId>
  <Length xmlns="">PT1M</Length>
  <SequenceLayers xmlns:d1p1="http://schemas.datacontract.org/2004/07/Vixen.Sys.LayerMixing" xmlns="">
    <d1p1:Layers xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays">
      <d2p1:anyType i:type="d1p1:DefaultLayer">
        <d1p1:Id>57edb46f-84ab-400b-b3b7-5cbdf7773aa1</d1p1:Id>
        <d1p1:LayerLevel>0</d1p1:LayerLevel>
        <d1p1:LayerName>Default</d1p1:LayerName>
        <d1p1:Type>Default</d1p1:Type>
      </d2p1:anyType>
    </d1p1:Layers>
    <d1p1:_effectLayerMap xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays" />
  </SequenceLayers>
  <Version xmlns="">0</Version>
  <_dataModels xmlns:d1p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays" xmlns="">
    <d1p1:anyType xmlns:d2p1="http://schemas.datacontract.org/2004/07/Some.Future.Effect" i:type="d2p1:FutureData">
      <ModuleInstanceId>11111111-1111-1111-1111-111111111111</ModuleInstanceId>
      <ModuleTypeId>22222222-2222-2222-2222-222222222222</ModuleTypeId>
      <d2p1:Zed>
        <d2p1:Inner i:nil="true" />
      </d2p1:Zed>
      <d2p1:Alpha>1</d2p1:Alpha>
    </d1p1:anyType>
    <d1p1:anyType xmlns:d2p1="http://schemas.datacontract.org/2004/07/Some.Orphan" i:type="d2p1:OrphanData">
      <ModuleInstanceId>33333333-3333-3333-3333-333333333333</ModuleInstanceId>
      <ModuleTypeId>44444444-4444-4444-4444-444444444444</ModuleTypeId>
    </d1p1:anyType>
  </_dataModels>
  <_effectNodeSurrogates xmlns="">
    <EffectNodeSurrogate>
      <InstanceId>11111111-1111-1111-1111-111111111111</InstanceId>
      <StartTime>PT1S</StartTime>
      <TargetNodes>
        <ChannelNodeReferenceSurrogate>
          <Name i:nil="true" />
          <NodeId>55555555-5555-5555-5555-555555555555</NodeId>
        </ChannelNodeReferenceSurrogate>
      </TargetNodes>
      <TimeSpan>PT2S</TimeSpan>
      <TypeId>22222222-2222-2222-2222-222222222222</TypeId>
      <FutureMember>x</FutureMember>
    </EffectNodeSurrogate>
  </_effectNodeSurrogates>
  <_filterNodeSurrogates xmlns="" />
  <_layerMixingFilterDataModels xmlns="" />
  <_layerMixingFilterSurrogates xmlns="" />
  <_mediaSurrogates xmlns="" />
  <_selectedTimingProviderSurrogate xmlns="">
    <ProviderType i:nil="true" />
    <SourceName i:nil="true" />
  </_selectedTimingProviderSurrogate>
  <Future>kept</Future>
  <DefaultPlaybackEndTime i:nil="true" />
  <DefaultPlaybackStartTime>PT0S</DefaultPlaybackStartTime>
  <DefaultRowHeight>40</DefaultRowHeight>
  <DefaultSplitterDistance>0</DefaultSplitterDistance>
  <LabeledMarkCollections />
  <RowSettings />
  <TimePerPixel>PT0.01S</TimePerPixel>
  <VisibleTimeStart>PT0S</VisibleTimeStart>
</TimedSequenceData>"#
    );
    let path = dir.path("Unknown.tim");
    std::fs::write(&path, &text).unwrap();
    let original = read(&path);
    let seq = load_sequence(&path).unwrap();
    assert_eq!(seq.effects.len(), 1);
    assert!(seq.effects[0].params.desc.is_empty());
    assert!(seq.effects[0].params.unknown_members.iter().any(|(n, _)| n == "Zed"));
    assert_eq!(seq.effects[0].targets[0].1, "");
    assert_eq!(seq.fidelity_report().orphan_models, 1);
    assert_eq!(seq.fidelity_report().effect_surrogates, 1);

    let out = dir.path("Out.tim");
    save_sequence(&seq, &out).unwrap();
    assert_eq!(original.first_difference(&read(&out), String::new()), None);

    // Moving the effect keeps its data and the surrogate's unknown member.
    let mut moved = seq.clone();
    moved.effects[0].start = vixen_core::TimeSpan::from_secs(5.0);
    let out = dir.path("Moved.tim");
    save_sequence(&moved, &out).unwrap();
    let saved = read(&out);
    let s = saved.child("_effectNodeSurrogates").unwrap().elements().next().unwrap();
    assert_eq!(s.child_text("StartTime").as_deref(), Some("PT5S"));
    assert_eq!(s.child_text("FutureMember").as_deref(), Some("x"));
    assert!(same_tree(saved.child("_dataModels").unwrap(), original.child("_dataModels").unwrap()));
}
