//! Generated controller-free profile for native Image Maps verification.
//! No app/device startup. `--inspect` only reads the saved profile/sequence.
use std::{path::PathBuf, sync::Arc};
use vixen_core::{StateValue, TimeSpan};
use vixen_effects::framework::ParamValue;
use vixen_io::{system::Channel, DataPaths, SystemData};
use vixen_session::{marks::{lipsync::PhonemeType, map_library::{EditCommit, LipSyncLibrary}}, sequence::{SequenceSession, SessionEnvironment}};

fn environment(system: &SystemData, paths: &DataPaths) -> SessionEnvironment {
    SessionEnvironment { video_cache: None, tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(system).into_owned()),
        module_data_root: Some(paths.module_data_dir()), frame_ms: 25, media_dir: paths.media_dir() }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("fresh private target profile"));
    let mut inspect = false;
    let mut live = false;
    for arg in args {
        match arg.as_str() { "--inspect" => inspect=true, "--live" => live=true, other => panic!("unknown argument {other}") }
    }
    assert!(root.components().any(|part| part.as_os_str() == "target"));
    let paths = DataPaths::new(root);
    let sequence_path = paths.sequence_dir().join("Image Maps Verification.tim");
    if inspect {
        let system = vixen_io::load_system(&paths).unwrap();
        assert!(system.controllers.is_empty());
        let library = LipSyncLibrary::from_store(&system.module_store);
        let mut sequence = SequenceSession::open(&sequence_path, &environment(&system, &paths)).unwrap();
        sequence.resolve_lip_sync_mappings(&library);
        sequence.wait_for_renders();
        let effects: Vec<_> = sequence.effects().map(|entry| {
            let colors: Vec<_> = entry.intents.as_ref().unwrap().map.values().flat_map(|nodes| nodes.iter()).filter_map(|node| {
                match node.intent.state_at(TimeSpan::ZERO) {
                    StateValue::Lighting(value) => Some(value.color.to_argb_u32()),
                    StateValue::Rgb(value) => Some(0xff000000u32 | u32::from(value.r) << 16 | u32::from(value.g) << 8 | u32::from(value.b)),
                    _ => None
                }
            }).collect();
            let mapping = (entry.descriptor.unwrap().type_id == fx_audio_state::lip_sync::DESCRIPTOR.type_id)
                .then(|| entry.node.params.string("PhonemeMapping"));
            serde_json::json!({"type":entry.descriptor.unwrap().name,"mapping":mapping,"rendered":entry.is_rendered(),"colors":colors})
        }).collect();
        let mut images = std::collections::BTreeMap::new();
        for name in library.names() {
            assert!(!vixen_session::marks::map_library::name_is_unsafe(&name));
            let folder = paths.module_data_dir().join("LipSync").join(&name);
            let mut entries = std::collections::BTreeMap::new();
            for phoneme in ["AI", "E"] {
                let file = folder.join(format!("{phoneme}.bmp"));
                if file.is_file() {
                    // The image-map contract stores PNG bytes under .bmp names.
                    let bitmap = image::load_from_memory(&std::fs::read(file).unwrap()).unwrap().to_rgba8();
                    let colors: std::collections::BTreeSet<_> = bitmap.pixels().map(|p| p.0).collect();
                    entries.insert(phoneme, colors);
                }
            }
            images.insert(name, entries);
        }
        println!("{}", serde_json::json!({"names":library.names(),"default":library.default_name(),"effects":effects,"images":images,"modified":sequence.is_modified()}));
        return;
    }
    assert!(!paths.root().exists(), "refuse existing profile");
    paths.ensure_directories().unwrap();
    let inputs = paths.root().join("Inputs");
    std::fs::create_dir(&inputs).unwrap();
    for (name, color) in [("red.png", [255,0,0,255]), ("green.png", [0,255,0,255])] {
        image::RgbaImage::from_pixel(16, 8, image::Rgba(color)).save(inputs.join(name)).unwrap();
    }
    let mut system = SystemData::default();
    system.default_update_interval = 25;
    let grid = vixen_effects::testing::grid(&mut system.tree, "LipSync grid", 16, 8);
    let target = system.tree.node(grid).id;
    let extra = live.then(|| {
        let face = system.tree.add_element("Face Mouth");
        let control = system.tree.add_element("Unrelated Control");
        (system.tree.node(face).id, system.tree.node(control).id)
    });
    system.channels = system.tree.all_nodes().filter_map(|(_, node)| node.element_id.map(|id| Channel { id, name:node.name.clone() })).collect();
    let mut library = LipSyncLibrary::from_store(&system.module_store);
    for name in ["Active Map", "Delete Me", "Rename Me", "Overwrite Me"] {
        let old = if name == "Active Map" { "New Map".to_string() } else { library.new_map(&paths.module_data_dir()) };
        let renamed = library.rename_map(&old, name, &paths.module_data_dir()).unwrap().unwrap();
        library.keep_new();
        let mut edit = library.begin_edit(&renamed, &paths.module_data_dir()).unwrap();
        for phoneme in vixen_session::marks::lipsync::MAP_PHONEMES { edit.clear_image(phoneme); }
        assert!(edit.set_image(PhonemeType::Ai, &inputs.join("red.png")));
        edit.notes = format!("Original {name}");
        assert!(matches!(library.commit_edit(&edit, None, &paths.module_data_dir()), EditCommit::Saved { alert:None, .. }));
    }
    library.set_default("Active Map");
    library.write_to(&mut system.module_store);
    // The upstream overwrite prompt is reachable when a library entry exists but its picture folder does not.
    std::fs::remove_dir_all(paths.module_data_dir().join("LipSync/Overwrite Me")).unwrap();
    vixen_io::save_system(&system, &paths).unwrap();
    let mut sequence = SequenceSession::new_untitled(&environment(&system, &paths));
    sequence.set_length(TimeSpan::from_secs(2.0));
    let id = sequence.add_new_effect(fx_audio_state::lip_sync::DESCRIPTOR.type_id, target, "LipSync grid", TimeSpan::ZERO, TimeSpan::from_secs(2.0)).unwrap();
    let mut params = sequence.effect(id).unwrap().node.params.clone();
    params.set("MappingType", ParamValue::Enum("Map".into()));
    params.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
    params.set("PhonemeMapping", ParamValue::Str("Active Map".into()));
    params.set("StaticPhoneme", ParamValue::Enum("AI".into()));
    sequence.set_params(id, params);
    if let Some((face, control)) = extra {
        let id = sequence.add_new_effect(fx_audio_state::lip_sync::DESCRIPTOR.type_id, face, "Face Mouth", TimeSpan::ZERO, TimeSpan::from_secs(2.0)).unwrap();
        let mut params = sequence.effect(id).unwrap().node.params.clone();
        params.set("MappingType", ParamValue::Enum("FaceDefinition".into()));
        params.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
        params.set("StaticPhoneme", ParamValue::Enum("AI".into()));
        sequence.set_params(id, params);
        let descriptor = vixen_effect_registry::all().iter().find(|d| d.name == "Set Level").unwrap();
        sequence.add_new_effect(descriptor.type_id, control, "Unrelated Control", TimeSpan::ZERO, TimeSpan::from_secs(2.0)).unwrap();
    }
    sequence.wait_for_renders();
    sequence.save(Some(&sequence_path)).unwrap();
    let report = serde_json::json!({"profile":paths.root(),"sequence":sequence_path,"muted":true,"live":live,"controllers":0,"channels":system.channels.len(),"maps":library.names(),"inputs":inputs});
    std::fs::write(paths.root().join("fixture.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{report}");
}
