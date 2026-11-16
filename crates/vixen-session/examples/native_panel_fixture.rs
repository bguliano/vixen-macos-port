//! Private normal-app file-panel fixture: no controllers, media, scheduler or listener.
use std::{path::PathBuf, sync::Arc};
use vixen_core::TimeSpan;
use vixen_io::{DataPaths, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("fresh target profile"));
    assert!(root.is_absolute() && root.components().any(|c| c.as_os_str() == "target"));
    assert!(!root.exists(), "refuse existing profile");
    let paths = DataPaths::new(root);
    paths.ensure_directories().unwrap();
    let mut system = SystemData::default();
    let node = system.tree.add_element("Private panel element");
    let node_id = system.tree.node(node).id;
    vixen_session::web::store_settings(&mut system.module_store, vixen_session::web::Settings { port: 8888, enabled: false });
    assert!(system.controllers.is_empty());
    vixen_io::save_system(&system, &paths).unwrap();
    let environment = SessionEnvironment {
        video_cache: None, tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()), frame_ms: 25, media_dir: paths.media_dir(),
    };
    let mut sequence = SequenceSession::new_untitled(&environment);
    sequence.set_length(TimeSpan::from_secs(3.0));
    let descriptor = vixen_effect_registry::all().iter().find(|e| e.name == "Set Level").unwrap();
    let effect = sequence.add_new_effect(descriptor.type_id, node_id, "Private panel element", TimeSpan::ZERO, TimeSpan::from_secs(2.0)).unwrap();
    let effect_instance = sequence.effect(effect).unwrap().node.instance_id;
    let file = paths.sequence_dir().join("Panel Source Café.tim");
    sequence.save(Some(&file)).unwrap();
    let manifest = serde_json::json!({"sequence":file,"effectInstanceId":effect_instance.to_string(),"effectSessionId":effect.0,"controllers":0,"media":0,"webEnabled":false,
        "requiredEnvironment":["VIXEN_DATA","VIXEN_SETTINGS_DIR","VIXEN_MUTE"],"requiredArguments":["no_controllers","no_execution"],
        "scope":"Normal-app panel tests also require a unique private CFBundleIdentifier. Do not set VIXEN_UI_TEST."});
    std::fs::write(paths.root().join("fixture.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    println!("{manifest}");
}
