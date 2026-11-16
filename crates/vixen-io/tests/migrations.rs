//! The `.tim.6` backups go through the ported migrators, and where nothing else has changed they must agree
//! with the later versions of the same sequences, which real Vixen migrated.

mod common;

use common::*;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_effects::framework::ParamValue;
use vixen_io::{load_sequence, save_sequence, EffectNode, TimedSequence};

fn backups(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("Sequence"))
        .unwrap()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|e| e == extension) && p.is_file())
        .collect();
    files.sort();
    files
}

/// The later file real Vixen wrote for this sequence: `Foo.tim.8`, or `Foo.tim` itself.
fn later_version(old: &Path) -> Option<PathBuf> {
    let name = file_name(old);
    let stem = name.strip_suffix(".6")?;
    let eight = old.with_file_name(format!("{stem}.8"));
    if eight.is_file() {
        return Some(eight);
    }
    let current = old.with_file_name(stem);
    current.is_file().then_some(current)
}

#[derive(Default)]
struct Agreement {
    compared_effects: usize,
    moved_effects: usize,
    compared_members: usize,
    /// (effect type, member) -> how many effects differ there.
    differences: BTreeMap<(String, String), usize>,
    example: HashMap<(String, String), String>,
}

fn compare_effects(old: &TimedSequence, later: &TimedSequence, a: &mut Agreement, file: &str) {
    let later_by_id: HashMap<Uuid, &EffectNode> = later.effects.iter().map(|e| (e.instance_id, e)).collect();
    for e in &old.effects {
        let Some(l) = later_by_id.get(&e.instance_id) else { continue };
        // Surrogates gained `Name` after version 6, so only the node ids can be compared.
        let nodes = |n: &EffectNode| n.targets.iter().map(|t| t.0).collect::<Vec<_>>();
        if e.start != l.start || e.time_span != l.time_span || nodes(e) != nodes(l) {
            a.moved_effects += 1;
        }
        a.compared_effects += 1;
        assert_eq!(e.type_id, l.type_id, "{file}");
        let data_type = vixen_effect_registry::by_type_id(&e.type_id).map_or("?", |d| d.data_type);
        for (i, d) in e.params.desc.iter().enumerate() {
            if d.xml.is_empty() {
                continue;
            }
            a.compared_members += 1;
            if e.params.values[i] != l.params.values[i] {
                let key = (data_type.to_string(), d.xml.to_string());
                *a.differences.entry(key.clone()).or_default() += 1;
                a.example.entry(key).or_insert_with(|| {
                    format!(
                        "{file} {}: {:?} became {:?}",
                        e.instance_id,
                        short(&e.params.values[i]),
                        short(&l.params.values[i])
                    )
                });
            }
        }
    }
}

/// Mark collections with the same id and the same marks: everything else about them must agree too.
fn compare_marks(old: &TimedSequence, later: &TimedSequence, file: &str) -> usize {
    let mut same = 0;
    for mc in &old.marks {
        let Some(l) = later.marks.iter().find(|m| m.id == mc.id) else { continue };
        let times = |m: &vixen_core::MarkCollection| {
            m.marks.iter().map(|x| (x.start_time, x.duration, x.text.clone())).collect::<Vec<_>>()
        };
        if times(mc) == times(l) {
            same += 1;
            assert_eq!(mc.name, l.name, "{file}: {}", mc.id);
            assert_eq!(mc.collection_type, l.collection_type, "{file}: {}", mc.id);
        }
    }
    same
}

fn short(v: &ParamValue) -> String {
    let s = format!("{v:?}");
    s.chars().take(200).collect()
}

#[test]
fn version_6_backups_load_through_the_migrators() {
    let Some(root) = data_root() else { return };
    let files = backups(&root, "6");
    if is_default_data(&root) {
        assert_eq!(files.len(), 7, "the install has these .tim.6 files");
    }
    let dir = TempDir::new("migrated");
    for path in &files {
        assert_eq!(read(path).attr("version"), Some("6"), "{}", path.display());
        let seq = load_sequence(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(seq.source_version, 6);
        assert_eq!(seq.version, 9);
        assert!(
            seq.effects.iter().all(|e| !e.params.desc.is_empty()),
            "{}: every effect type is known",
            path.display()
        );

        // A migrated sequence saves and reads back the same, and an old-version file on disk gets its backup.
        let out = dir.path(&file_name(path).replace(".6", ""));
        std::fs::copy(path, &out).unwrap();
        save_sequence(&seq, &out).unwrap();
        assert!(dir.path(&format!("{}.6", file_name(&out))).is_file(), "the version backup is made on save");
        let back = load_sequence(&out).unwrap();
        assert_eq!(back.source_version, 9);
        assert_eq!(back.effects.len(), seq.effects.len());
        for (a, b) in seq.effects.iter().zip(&back.effects) {
            assert_eq!(a.params, b.params, "{}", path.display());
        }
    }
}

#[test]
fn wipes_migrate_to_the_new_members() {
    let Some(root) = data_root() else { return };
    // The user's own version 6 files have no Wipes, so this only finds work on other installs. The unit tests
    // in `sequence_migrate.rs` cover the transformation itself.
    for path in backups(&root, "6") {
        let xml = read(&path);
        let seq = load_sequence(&path).unwrap();
        for e in seq
            .effects
            .iter()
            .filter(|e| vixen_effect_registry::by_type_id(&e.type_id).is_some_and(|d| d.data_type == "WipeData"))
        {
            let old = xml
                .child("_dataModels")
                .unwrap()
                .elements()
                .find(|m| m.child_text("ModuleInstanceId") == Some(e.instance_id.to_string()))
                .unwrap();
            let by_count = old.child_text("WipeByCount").as_deref() == Some("true");
            assert_eq!(e.params.string("WipeMovement"), if by_count { "Count" } else { "PulseLength" });
            let (direction, reverse) = match old.child_text("Direction").as_deref() {
                Some("Up") => ("Vertical", true),
                Some("Down") => ("Vertical", false),
                Some("Right") => ("Horizontal", false),
                Some("Left") => ("Horizontal", true),
                Some("In") => ("Burst", true),
                Some("Out") => ("Burst", false),
                other => panic!("{other:?} in {}", path.display()),
            };
            assert_eq!(e.params.string("Direction"), direction);
            assert_eq!(e.params.boolean("ReverseDirection"), reverse);
            assert!(e.params.boolean("ColorAcrossItemPerCount"));
            assert!(e.params.boolean("ReverseColorDirection"));
            assert_eq!(e.params.curve("MovementCurve").points, vec![(0.0, 0.0), (100.0, 100.0)]);
            assert_eq!(e.params.int("PassCount").to_string(), old.child_text("PassCount").unwrap());
            assert_eq!(e.params.int("PulseTime").to_string(), old.child_text("PulseTime").unwrap());
            assert_eq!(e.params.boolean("WipeOn").to_string(), old.child_text("WipeOn").unwrap());
            assert_eq!(e.params.boolean("WipeOff").to_string(), old.child_text("WipeOff").unwrap());
        }
    }
}

#[test]
fn version_6_agrees_with_later_versions_where_the_content_is_the_same() {
    let Some(root) = data_root() else { return };
    let mut a = Agreement::default();
    let mut same_marks = 0;
    for old_path in backups(&root, "6") {
        let Some(later_path) = later_version(&old_path) else { continue };
        let old = load_sequence(&old_path).unwrap();
        let later = load_sequence(&later_path).unwrap();
        compare_effects(&old, &later, &mut a, &file_name(&old_path));
        same_marks += compare_marks(&old, &later, &file_name(&old_path));
        assert_eq!(old.module_type_id, later.module_type_id);
        // The user renamed an audio file between these versions, so media names can differ. Their count cannot.
        assert_eq!(old.media.len(), later.media.len(), "{}", old_path.display());
    }
    let total: usize = a.differences.values().sum();
    eprintln!(
        "compared {} effects ({} moved or retargeted later), {} members and {same_marks} unchanged mark collections; {total} members differ",
        a.compared_effects, a.moved_effects, a.compared_members
    );
    for (key, n) in &a.differences {
        eprintln!("  {}.{}: {n}   e.g. {}", key.0, key.1, a.example[key]);
    }
    if is_default_data(&root) {
        assert!(a.compared_effects > 500, "{}", a.compared_effects);
        // SetLevel's colour is the only difference, in a few effects the user recoloured in the years between
        // these files. No migrator between version 6 and 8 touches SetLevel.
        assert!(a.differences.keys().all(|(t, m)| t == "SetLevelData" && m == "color"), "{:?}", a.differences);
        assert!(total * 100 < a.compared_members, "{total} of {}", a.compared_members);
    }
}
