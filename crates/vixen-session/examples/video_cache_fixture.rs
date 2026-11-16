//! Private generated-video profile for the AppKit cache workflow. No hardware.
//! cargo run -p vixen-session --example video_cache_fixture -- target/video-cache-ui/profile
//! Add --video crates/vixen-video/tests/assets/colors.avi for bundled fallback verification.
use std::{path::PathBuf, sync::Arc};
use vixen_core::TimeSpan;
use vixen_effects::framework::ParamValue;
use vixen_io::{system::Channel, DataPaths, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("fresh private profile folder"));
    let mut source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vixen-video/tests/assets/colors.mov");
    let mut metadata = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--video" => source = PathBuf::from(args.next().expect("generated video fixture after --video")),
            "--metadata" => metadata = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let source = source.canonicalize().expect("existing generated video fixture");
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    assert!(source.starts_with(repository.join("target")) || source.starts_with(repository.join("crates/vixen-video/tests/assets")),
        "use an established generated fixture under target or the decoder test assets");
    assert!(source.is_file() && source.metadata().unwrap().len() < 50 * 1024 * 1024, "fixture must be a small local file");
    let extension = source.extension().and_then(|value| value.to_str()).unwrap().to_ascii_lowercase();
    assert!(["mov", "avi", "mp4", "mts"].contains(&extension.as_str()), "unsupported fixture extension");
    let file_name = format!("generated.{extension}");
    assert!(!root.exists(), "fixture folder must not exist");
    assert!(root.components().any(|part| part.as_os_str() == "target"), "fixture belongs under target");
    let paths = DataPaths::new(root);
    paths.ensure_directories().unwrap();
    let mut system = SystemData::default();
    system.default_update_interval = 20;
    let grid = vixen_effects::testing::grid(&mut system.tree, "Video grid", 16, 8);
    if metadata { vixen_effects::testing::add_locations(&mut system.tree, grid); }
    let target = system.tree.node(grid).id;
    system.channels = system.tree.all_nodes().filter_map(|(_, node)| {
        node.element_id.map(|id| Channel { id, name: node.name.clone() })
    }).collect();
    vixen_io::save_system(&system, &paths).unwrap();
    let env = SessionEnvironment {
        video_cache: None,
        tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: 20,
        media_dir: paths.media_dir(),
    };
    let mut sequence = SequenceSession::new_untitled(&env);
    sequence.set_length(TimeSpan::from_secs(2.0));
    let descriptor = vixen_effect_registry::all().iter().find(|entry| entry.name == "Video").unwrap();
    let id = sequence.add_new_effect(descriptor.type_id, target, "Video grid", TimeSpan::ZERO, TimeSpan::from_secs(2.0)).unwrap();
    let mut params = sequence.effect(id).unwrap().node.params.clone();
    params.set("FileName", ParamValue::Str(file_name.clone()));
    if metadata {
        params.set("TargetPositioning", ParamValue::Enum("Locations".into()));
        params.set("VideoQuality", ParamValue::Int(80));
    }
    sequence.set_params(id, params);
    // The file arrives only after serialization; setup cannot prewarm the cache.
    sequence.wait_for_renders();
    let sequence_path = paths.sequence_dir().join("Video Cache.tim");
    sequence.save(Some(&sequence_path)).unwrap();
    drop(sequence);
    std::fs::create_dir_all(paths.module_data_dir().join("Video")).unwrap();
    let video = paths.module_data_dir().join("Video").join(file_name);
    std::fs::copy(&source, &video).unwrap();
    let mut choices = Vec::new();
    if metadata {
        let input = paths.root().join("InputVideo");
        std::fs::create_dir(&input).unwrap();
        for (source, name) in [("colors.mov", "picked.mov"), ("colors.avi", "software.avi")] {
            let source = repository.join("crates/vixen-video/tests/assets").join(source);
            let destination = input.join(name);
            std::fs::copy(source, &destination).unwrap();
            choices.push(destination);
        }
    }
    let cache = vixen_video::cache::profile_cache_root(paths.root());
    let report = serde_json::json!({"profile":paths.root(),"sequence":sequence_path,"cache":cache,
        "video":video,"source":source,"metadata":metadata,"choices":choices,"muted":true,"channels":system.channels.len(),"controllers":system.controllers.len()});
    std::fs::write(paths.root().join("fixture.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{report}");
}
