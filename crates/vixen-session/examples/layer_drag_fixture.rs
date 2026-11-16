//! Private, controller-free sequence for the native F62 row-drag check.
use std::{path::PathBuf, sync::Arc};
use vixen_core::{layer::{Layer, MixingFilter}, TimeSpan};
use vixen_io::{DataPaths, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("fresh target profile"));
    assert!(root.components().any(|part| part.as_os_str() == "target"));
    assert!(!root.exists(), "refuse existing profile");
    let paths = DataPaths::new(root);
    paths.ensure_directories().unwrap();
    let system = SystemData::default();
    assert!(system.controllers.is_empty());
    vixen_io::save_system(&system, &paths).unwrap();
    let env = SessionEnvironment {
        video_cache: None, tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()), frame_ms: 25, media_dir: paths.media_dir(),
    };
    let mut sequence = SequenceSession::new_untitled(&env);
    sequence.set_length(TimeSpan::from_secs(10.0));
    let layers = &mut sequence.document_mut().layers;
    // add_layer inserts at the top. Distinct names, IDs and filter settings make
    // accidental alphabetical sorting, identity replacement and data loss visible.
    layers.add_layer(Layer::standard("Middle Layer", MixingFilter::LumaKey { lower_limit: 0.2, upper_limit: 0.7 }));
    layers.add_layer(Layer::standard("Alpha Layer", MixingFilter::MaskFill { exclude_zero_values: false, requires_mixing_partner: true }));
    layers.add_layer(Layer::standard("Zebra Layer", MixingFilter::Mask));
    sequence.layers_changed();
    let sequence_path = paths.sequence_dir().join("Layer Drag Verification.tim");
    sequence.save(Some(&sequence_path)).unwrap();
    let back = SequenceSession::open(&sequence_path, &env).unwrap();
    let original = &sequence.document().layers.layers;
    assert_eq!(back.document().layers.layers.len(), original.len());
    for (read, original) in back.document().layers.layers.iter().zip(original) {
        assert_eq!((read.id, &read.name, read.level, read.layer_type, read.filter_instance_id, &read.filter),
            (original.id, &original.name, original.level, original.layer_type, original.filter_instance_id, &original.filter));
    }
    let manifest = serde_json::json!({"controllers":0,"muted":true,"sequence":sequence_path,
        "layers":original.iter().map(|layer| serde_json::json!({"id":layer.id.to_string(),"name":layer.name,
            "level":layer.level,"filterInstanceId":layer.filter_instance_id.to_string(),
            "filterTypeId":layer.filter.as_ref().map(|filter| filter.type_id().to_string()),
            "filter":format!("{:?}",layer.filter),"default":layer.filter.is_none()})).collect::<Vec<_>>()});
    std::fs::write(paths.root().join("fixture.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    println!("{manifest}");
}
