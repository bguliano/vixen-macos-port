//! Small controller-free profiles for native Export Sequence lifecycle checks.
use std::{path::PathBuf, sync::Arc};
use vixen_core::TimeSpan;
use vixen_io::{system::Channel, DataPaths, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("fresh private target profile"));
    assert!(args.next().is_none());
    assert!(root.components().any(|part| part.as_os_str() == "target"));
    assert!(!root.exists(), "generate into a fresh directory only");
    let paths = DataPaths::new(root);
    paths.ensure_directories().unwrap();
    let mut system = SystemData::default();
    system.default_update_interval = 50;
    let group = system.tree.add_string("Private test lights", 32);
    let target = system.tree.node(group).id;
    system.channels = system
        .tree
        .all_nodes()
        .filter_map(|(_, node)| {
            node.element_id.map(|id| Channel {
                id,
                name: node.name.clone(),
            })
        })
        .collect();
    vixen_io::save_system(&system, &paths).unwrap();
    let env = SessionEnvironment {
        video_cache: None,
        tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: 50,
        media_dir: paths.media_dir(),
    };
    let mut sequences = Vec::new();
    // A long constant intent keeps memory tiny but makes export pause observable.
    // Below the signed i32 millisecond limit in the original export format; the
    // UI test cancels this run after two observations instead of finishing it.
    for (name, seconds) in [
        ("Export Long", 24.0 * 24.0 * 3600.0),
        ("Export Short", 30.0),
    ] {
        let mut sequence = SequenceSession::new_untitled(&env);
        let duration = TimeSpan::from_secs(seconds);
        sequence.set_length(duration);
        sequence
            .add_new_effect(
                vixen_effect_registry::all()
                    .iter()
                    .find(|effect| effect.name == "Set Level")
                    .unwrap()
                    .type_id,
                target,
                "Private test lights",
                TimeSpan::ZERO,
                duration,
            )
            .unwrap();
        sequence.wait_for_renders();
        let path = paths.sequence_dir().join(format!("{name}.tim"));
        sequence.save(Some(&path)).unwrap();
        sequences.push(path);
    }
    let collision = paths.root().join("collision-parent");
    std::fs::write(&collision, b"private failure fixture; preserve me\n").unwrap();
    let report = serde_json::json!({"profile": paths.root(), "long": sequences[0],
        "short": sequences[1], "collision": collision, "controllers": 0,
        "channels": 32, "muted": true, "longSeconds": 2073600, "shortSeconds": 30});
    std::fs::write(
        paths.root().join("fixture.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
}
