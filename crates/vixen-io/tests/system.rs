//! The user's real SystemConfig.xml, ModuleStore.xml and VixenApplicationData.xml.

mod common;

use common::*;
use std::collections::BTreeSet;
use uuid::Uuid;
use vixen_core::element::{property_ids, ElementColorType, Orientation, PropertyData};
use vixen_core::{Color, Curve};
use vixen_io::library::{CURVE_LIBRARY_TYPE_ID, GRADIENT_LIBRARY_TYPE_ID};
use vixen_io::module_store::load_module_store_file;
use vixen_io::system::{effective_store, load_system_config_file, system_config_xml, ORDER_PROPERTY_TYPE_ID};
use vixen_io::xml::first_loss;
use vixen_io::{
    color_sets, curve_library, gradient_library, load_application_data, load_system, save_application_data,
    save_module_store, save_system, set_color_sets, set_curve_library, set_gradient_library, DataPaths,
};
use vixen_xml::XElement;

#[test]
fn the_tree_shows_the_real_structure() {
    let Some(root) = data_root() else { return };
    let system = load_system(&DataPaths::new(&root)).unwrap();
    let xml = read(&root.join("SystemData/SystemConfig.xml"));

    assert_eq!(system.version, 16);
    assert_eq!(system.identity.to_string(), xml.child_text("Identity").unwrap());
    assert_eq!(system.channels.len(), xml.child("Channels").unwrap().elements().count());
    assert_eq!(system.controllers.len(), xml.child("Controllers").unwrap().elements().count());
    assert_eq!(system.previews.len(), xml.child("Previews").unwrap().elements().count());
    assert_eq!(system.filters.len(), xml.child("Filters").unwrap().elements().count());
    assert_eq!(system.patches.len(), xml.child("Patches").unwrap().elements().count());
    assert_eq!(system.disabled_devices.len(), xml.child("DisabledDevices").unwrap().elements().count());

    let top: Vec<&str> = system.tree.roots.iter().map(|r| system.tree.node(*r).name.as_str()).collect();
    if is_default_data(&root) {
        assert_eq!(top, ["Relays", "Light Strip L", "Light Strip R"]);
        assert_eq!(system.channels.len(), 1133);
        assert_eq!(system.tree.element_ids().count(), 1133);
        assert_eq!(system.default_update_interval, 10);
        assert!(system.allow_filter_evaluation);
        assert!(system.clear_effect_cache_on_exit);
        assert_eq!(system.video_effect_cache_file_type, "bmp");
    }
    // Every node id in the file is in the tree, and every leaf's element is a channel.
    let mut ids = BTreeSet::new();
    fn collect(e: &XElement, ids: &mut BTreeSet<String>) {
        for n in e.children_named("Node") {
            ids.insert(n.attr("id").unwrap().to_string());
            collect(n, ids);
        }
    }
    collect(xml.child("Nodes").unwrap(), &mut ids);
    assert_eq!(ids.len(), system.tree.len());
    let channels: BTreeSet<Uuid> = system.channels.iter().map(|c| c.id).collect();
    for (id, n) in system.tree.all_nodes() {
        if let Some(e) = n.element_id {
            assert!(channels.contains(&e), "{} names a channel that does not exist", n.name);
            assert!(system.tree.is_leaf(id));
        }
    }
    // Every element is reachable from the top level.
    let mut reachable = BTreeSet::new();
    for r in &system.tree.roots {
        reachable.extend(system.tree.distinct_elements(*r));
    }
    assert_eq!(reachable.len(), system.tree.element_ids().count());
}

#[test]
fn properties_are_resolved_from_the_module_store() {
    let Some(root) = data_root() else { return };
    let system = load_system(&DataPaths::new(&root)).unwrap();
    let (mut colors, mut locations, mut orientations, mut unknown) = (0, 0, 0, 0);
    for (_, n) in system.tree.all_nodes() {
        for p in &n.properties {
            match &p.data {
                PropertyData::Color { color_type, single_color, color_set_name } => {
                    assert_eq!(p.type_id, property_ids::COLOR);
                    colors += 1;
                    let data = system.module_store.instance(p.type_id, p.instance_id).unwrap().data().unwrap();
                    let text = data.child_text("ElementColorType").unwrap();
                    assert_eq!(format!("{color_type:?}"), text);
                    match color_type {
                        ElementColorType::SingleColor => {
                            assert!(data.child("SingleColor").is_some() && *single_color != Color::EMPTY)
                        }
                        ElementColorType::MultipleDiscreteColors => assert!(color_set_name.is_some()),
                        ElementColorType::FullColor => assert_eq!(*single_color, Color::EMPTY),
                    }
                }
                PropertyData::Location { x, y, z } => {
                    assert_eq!(p.type_id, property_ids::LOCATION);
                    locations += 1;
                    let data = system.module_store.instance(p.type_id, p.instance_id).unwrap().data().unwrap();
                    assert_eq!(*x, data.child_text("X").unwrap().parse::<i32>().unwrap().max(0));
                    assert_eq!(*y, data.child_text("Y").unwrap().parse::<i32>().unwrap().max(0));
                    assert_eq!(*z, data.child_text("Z").unwrap().parse::<i32>().unwrap().max(0));
                }
                PropertyData::Orientation(o) => {
                    assert_eq!(p.type_id, property_ids::ORIENTATION);
                    orientations += 1;
                    let data = system.module_store.instance(p.type_id, p.instance_id).unwrap().data().unwrap();
                    let expected = if *o == Orientation::Vertical { "Vertical" } else { "Horizontal" };
                    assert_eq!(data.child_text("Orientation").unwrap(), expected);
                }
                PropertyData::Order { .. } => assert_eq!(p.type_id, ORDER_PROPERTY_TYPE_ID),
                PropertyData::Unknown { raw_xml } => {
                    unknown += 1;
                    assert_ne!(p.type_id, property_ids::COLOR);
                    assert_ne!(p.type_id, property_ids::LOCATION);
                    assert_ne!(p.type_id, property_ids::ORIENTATION);
                    assert!(raw_xml.is_empty() || raw_xml.starts_with('<'));
                }
            }
        }
    }
    eprintln!("{colors} colour, {locations} location, {orientations} orientation and {unknown} other properties");
    if is_default_data(&root) {
        assert!(colors > 1000 && locations > 1000 && orientations >= 1, "{colors} {locations} {orientations}");
    }
    assert!(!system.tree.color_sets.is_empty());
}

#[test]
fn both_system_files_save_with_nothing_lost_or_changed() {
    let Some(root) = data_root() else { return };
    let source = DataPaths::new(&root);
    let system = load_system(&source).unwrap();
    // Nothing edited, so nothing in the tree changes the store.
    assert!(matches!(effective_store(&system), std::borrow::Cow::Borrowed(_)));

    let dir = TempDir::new("system");
    let target = DataPaths::new(&dir.0);
    save_system(&system, &target).unwrap();

    let original_config = read(&source.system_config_file());
    let saved_config = read(&target.system_config_file());
    // Vixen's current code writes members these files may not have yet. They stay out until they change.
    for member in system.residue_members() {
        assert!(["Tags", "ClearEffectCacheOnExit", "VideoEffectCacheFileType"].contains(&member.as_str()), "{member}");
    }
    assert_eq!(original_config.first_difference(&saved_config, String::new()), None);

    let original_store = read(&source.module_store_file());
    let saved_store = read(&target.module_store_file());
    let version = saved_store.attr("version").unwrap().to_string();
    assert_eq!(with_version(original_store, &version).first_difference(&saved_store, String::new()), None);

    // What was written loads again to the same thing.
    let back = load_system(&target).unwrap();
    assert_eq!(back.tree.len(), system.tree.len());
    assert_eq!(back.channels, system.channels);
    assert_eq!(back.controllers, system.controllers);
    assert_eq!(back.patches, system.patches);
    assert_eq!(system_config_xml(&back).first_difference(&saved_config, String::new()), None);
}

#[test]
fn saved_system_text_keeps_the_shape_vixen_wrote() {
    let Some(root) = data_root() else { return };
    let source = DataPaths::new(&root);
    let system = load_system(&source).unwrap();
    let dir = TempDir::new("system-text");
    let target = DataPaths::new(&dir.0);
    save_system(&system, &target).unwrap();
    let original = std::fs::read_to_string(source.system_config_file()).unwrap();
    let saved = std::fs::read_to_string(target.system_config_file()).unwrap();
    assert_eq!(saved.lines().count(), original.lines().count());
    if system.residue_members().iter().all(|m| m == "Tags") {
        assert!(saved == original, "SystemConfig.xml is byte-identical after a load and save");
    }
    let original = std::fs::read_to_string(source.module_store_file()).unwrap();
    let saved = std::fs::read_to_string(target.module_store_file()).unwrap();
    assert_eq!(saved.lines().count(), original.lines().count());
    assert!(saved.len().abs_diff(original.len()) < original.len() / 50, "{} against {}", saved.len(), original.len());
}

#[test]
fn every_system_backup_round_trips() {
    let Some(root) = data_root() else { return };
    let dir = TempDir::new("system-backups");
    let mut stores = 0;
    let mut configs = 0;
    for folder in [root.join("SystemData"), root.join("SystemData/auto_backup")] {
        let Ok(entries) = std::fs::read_dir(&folder) else { continue };
        for e in entries.flatten() {
            let path = e.path();
            if !path.is_file() {
                continue;
            }
            let name = file_name(&path);
            if name.starts_with("ModuleStore") {
                let store = load_module_store_file(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let target = DataPaths::new(dir.path(&format!("store-{stores}")));
                save_module_store(&store, &target).unwrap();
                let original = read(&path);
                let expected = with_version(original, "5");
                assert_eq!(
                    expected.first_difference(&read(&target.module_store_file()), String::new()),
                    None,
                    "{}",
                    path.display()
                );
                stores += 1;
            } else if name.starts_with("SystemConfig") {
                let original = read(&path);
                let system = load_system_config_file(&path, vixen_io::ModuleStore::new()).unwrap();
                assert_eq!(
                    original.first_difference(&system_config_xml(&system), String::new()),
                    None,
                    "{}",
                    path.display()
                );
                configs += 1;
            }
        }
    }
    eprintln!("{stores} module stores and {configs} configs checked");
    assert!(stores > 0 && configs > 0);
}

#[test]
fn a_version_4_store_migrates_with_nothing_lost() {
    let Some(root) = data_root() else { return };
    let path = root.join("SystemData/ModuleStore.xml.4");
    if !path.is_file() {
        return;
    }
    let store = load_module_store_file(&path).unwrap();
    assert_eq!(store.source_version, 4);
    assert_eq!(store.version, 5);
    let dir = TempDir::new("store-4");
    let target = DataPaths::new(&dir.0);
    save_module_store(&store, &target).unwrap();
    let mut added = Vec::new();
    let original = with_version(read(&path), "5");
    assert_eq!(first_loss(&original, &read(&target.module_store_file()), "", &mut added), None);
    assert!(added.is_empty(), "{added:?}");
}

#[test]
fn every_module_entry_is_kept_and_can_be_found() {
    let Some(root) = data_root() else { return };
    let path = root.join("SystemData/ModuleStore.xml");
    let xml = read(&path);
    let store = load_module_store_file(&path).unwrap();
    let sections: Vec<&XElement> = xml.elements().collect();
    assert_eq!(store.type_entries().len(), sections[0].elements().count());
    assert_eq!(store.instance_entries().len(), sections[1].elements().count());
    for m in sections[1].elements() {
        let t: Uuid = m.attr("moduleType").unwrap().parse().unwrap();
        let i: Uuid = m.attr("moduleInstance").unwrap().parse().unwrap();
        let e = store.instance(t, i).unwrap();
        assert_eq!(e.data_model_type(), m.attr("dataModelType"));
    }
    for m in sections[0].elements() {
        let t: Uuid = m.attr("moduleType").unwrap().parse().unwrap();
        assert!(store.type_data(t).is_some());
    }
    if is_default_data(&root) {
        assert_eq!(store.type_entries().len(), 13);
        assert_eq!(store.instance_entries().len(), 3381);
    }
}

#[test]
fn libraries_read_and_write_with_no_change() {
    let Some(root) = data_root() else { return };
    let path = root.join("SystemData/ModuleStore.xml");
    let mut store = load_module_store_file(&path).unwrap();
    let before = read(&path);

    let curves = curve_library(&store);
    let gradients = gradient_library(&store);
    let sets = color_sets(&store);
    let library_size = |t: Uuid| {
        store.type_data(t).and_then(|e| e.data()).and_then(|d| d.child("_library")).map_or(0, |l| l.elements().count())
    };
    assert_eq!(curves.curves.len(), library_size(CURVE_LIBRARY_TYPE_ID));
    assert_eq!(gradients.gradients.len(), library_size(GRADIENT_LIBRARY_TYPE_ID));
    if is_default_data(&root) {
        assert!(curves.get("Accelerate").is_some());
        assert!(curves.get("Accelerate").unwrap().points.len() > 2);
        assert!(gradients.get("white to green delay").is_some());
        assert_eq!(sets[0].0, "RGBW");
        assert_eq!(
            sets[0].1,
            vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0), Color::rgb(0, 0, 255), Color::rgb(255, 255, 255)]
        );
    }
    for (_, c) in &curves.curves {
        assert!(c.is_current_library_curve, "library curves are marked current");
    }

    set_curve_library(&mut store, &curves);
    set_gradient_library(&mut store, &gradients);
    set_color_sets(&mut store, &sets);
    let dir = TempDir::new("libraries");
    let target = DataPaths::new(&dir.0);
    save_module_store(&store, &target).unwrap();
    let version = store.version.to_string();
    assert_eq!(
        with_version(before, &version).first_difference(&read(&target.module_store_file()), String::new()),
        None
    );

    // An edit reaches the file and comes back.
    let mut edited = curves.clone();
    edited.curves.push(("Mine".to_string(), Curve::flat(33.0)));
    set_curve_library(&mut store, &edited);
    save_module_store(&store, &target).unwrap();
    let back = load_module_store_file(&target.module_store_file()).unwrap();
    assert_eq!(curve_library(&back), edited);
    assert_eq!(gradient_library(&back), gradients);
}

#[test]
fn application_data_round_trips_and_resolves_recent_files() {
    let Some(root) = data_root() else { return };
    let source = DataPaths::new(&root);
    let data = load_application_data(&source).unwrap();
    let xml = read(&source.application_data_file());
    assert_eq!(data.recent_sequences.len(), xml.child("RecentSequences").unwrap().elements().count());

    let recents = data.resolve_recent_sequences(&source);
    for r in &recents {
        assert!(r.path.starts_with(source.sequence_dir()));
        assert!(!r.path.to_string_lossy().contains('\\'), "{}", r.path.display());
    }
    if is_default_data(&root) {
        assert!(data.recent_sequences[0].contains(r"C:\Users\developer"));
        assert_eq!(recents[0].path, source.sequence_dir().join("Coldest Winter.tim"));
        assert!(recents[0].exists);
    }

    let dir = TempDir::new("appdata");
    let target = DataPaths::new(&dir.0);
    save_application_data(&data, &target).unwrap();
    assert_eq!(xml.first_difference(&read(&target.application_data_file()), String::new()), None);
    if is_default_data(&root) {
        let original = std::fs::read(source.application_data_file()).unwrap();
        assert_eq!(std::fs::read(target.application_data_file()).unwrap(), original, "byte-identical");
    }
}

#[test]
fn saving_a_system_file_keeps_vixens_backups() {
    let Some(root) = data_root() else { return };
    let source = DataPaths::new(&root);
    let system = load_system(&source).unwrap();
    let dir = TempDir::new("system-backup-rules");
    let target = DataPaths::new(&dir.0);
    save_system(&system, &target).unwrap();
    assert!(!target.system_backup_dir().exists(), "a first save has nothing to back up");
    save_system(&system, &target).unwrap();
    let names: Vec<String> =
        std::fs::read_dir(target.system_backup_dir()).unwrap().map(|e| file_name(&e.unwrap().path())).collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names.iter().any(|n| n.starts_with("SystemConfig_") && n.ends_with(".xml")), "{names:?}");
    assert!(names.iter().any(|n| n.starts_with("ModuleStore_") && n.ends_with(".xml")), "{names:?}");
}
