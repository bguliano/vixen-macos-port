//! Controller-free Fixture-effect profile and read-only saved-file inspector.
//! cargo run -p vixen-session --example fixture_authoring_fixture -- target/NEW/profile
//! ... -- target/NEW/profile --inspect
use fx_fixture::fixture_base::test_support::fixtures;
use fx_fixture::spec::{
    FixtureChannel, FixtureFunction, FixtureFunctionType, FixtureIndexItem, FixtureSpecification,
    FuncId,
};
use std::{path::PathBuf, sync::Arc};
use vixen_core::{Color, ColorGradient, Curve, StateValue, TimeSpan};
use vixen_io::{system::Channel, DataPaths, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn environment(system: &SystemData, paths: &DataPaths) -> SessionEnvironment {
    SessionEnvironment {
        video_cache: None,
        tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(system).into_owned()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: 50,
        media_dir: paths.media_dir(),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("private target profile"));
    let inspect = match args.next().as_deref() {
        None => false,
        Some("--inspect") => true,
        Some(other) => panic!("unknown argument {other}"),
    };
    assert!(args.next().is_none());
    assert!(root.components().any(|part| part.as_os_str() == "target"));
    let paths = DataPaths::new(root);
    let sequence_path = paths.sequence_dir().join("Fixture Authoring.tim");
    if inspect {
        let system = vixen_io::load_system(&paths).unwrap();
        assert!(system.controllers.is_empty());
        let mut session =
            SequenceSession::open(&sequence_path, &environment(&system, &paths)).unwrap();
        session.wait_for_renders();
        let entries: Vec<_> = session.effects().collect();
        assert_eq!(entries.len(), 1);
        let entry = entries[0];
        let rows: Vec<_> = entry.node.params.records("FunctionData").iter().map(|row| serde_json::json!({
            "name": row.string("FunctionName"), "type": row.enum_name("FunctionType"),
            "identity": row.enum_name("FunctionIdentity"), "index": row.string("IndexValue"),
            "zeroRange": row.curve("Range") == Curve::flat(0.0), "fullRange": row.curve("Range") == Curve::flat100(),
            "white": row.gradient("Color") == ColorGradient::solid(Color::WHITE), "timelineColor": row.color("TimelineColor").to_argb_u32(),
        })).collect();
        let intents = entry.intents.as_ref().unwrap();
        let leaves: Vec<_> = intents.map.values().map(|nodes| {
            let states: Vec<_> = nodes.iter().map(|node| node.intent.state_at(TimeSpan::ZERO)).collect();
            serde_json::json!({
                "gobo": states.iter().any(|state| matches!(state, StateValue::Command(vixen_core::CommandValue::Named8(value))
                    if value.tag.as_ref() == "Gobo" && value.label.as_ref() == "G" && value.value == 84
                        && value.range_minimum == 84 && value.range_maximum == 126
                        && value.index_type == vixen_core::value::FixtureIndexType::Custom)),
                "whiteRgb": states.iter().any(|state| matches!(state, StateValue::Lighting(value) if value.color == Color::WHITE && value.intensity == 1.0)),
                "intents": nodes.len(),
            })
        }).collect();
        println!(
            "{}",
            serde_json::json!({"rows": rows, "selectedFunction": entry.node.params.string("SelectedFunction"),
            "rendered": entry.is_rendered(), "leaves": leaves, "modified": session.is_modified(), "undoCount": session.undo_stack().undo_descriptions().len()})
        );
        return;
    }
    assert!(
        !paths.root().exists(),
        "generate only into a fresh private folder"
    );
    paths.ensure_directories().unwrap();
    let functions = vec![
        FixtureFunction {
            name: "Pan".into(),
            identity: FuncId::Pan,
            label: "P".into(),
            timeline_color: Color::RED,
            ..Default::default()
        },
        FixtureFunction {
            name: "Gobo".into(),
            function_type: FixtureFunctionType::Indexed,
            identity: FuncId::Gobo,
            label: "G".into(),
            index_data: vec![
                FixtureIndexItem {
                    name: "Star".into(),
                    start_value: 42,
                    end_value: 42,
                    ..FixtureIndexItem::index()
                },
                FixtureIndexItem {
                    name: "Circle".into(),
                    start_value: 84,
                    end_value: 126,
                    use_curve: true,
                    ..FixtureIndexItem::index()
                },
            ],
            timeline_color: Color::BLUE,
            ..Default::default()
        },
        FixtureFunction {
            name: "RGB".into(),
            function_type: FixtureFunctionType::RgbColor,
            timeline_color: Color::GREEN,
            ..Default::default()
        },
    ];
    let channels = functions
        .iter()
        .enumerate()
        .map(|(index, function)| FixtureChannel {
            name: function.name.clone(),
            function: function.name.clone(),
            channel_number: index as i32 + 1,
        })
        .collect();
    let spec = FixtureSpecification {
        name: "Local Native Test Head".into(),
        functions,
        channels,
        ..Default::default()
    };
    let mut system = SystemData::default();
    system.default_update_interval = 50;
    let group = fixtures(&mut system.tree, "Test Heads", 2, &spec);
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
    let mut session = SequenceSession::new_untitled(&environment(&system, &paths));
    session.set_length(TimeSpan::from_secs(2.0));
    session
        .add_new_effect(
            fx_fixture::fixture::DESCRIPTOR.type_id,
            target,
            "Test Heads",
            TimeSpan::ZERO,
            TimeSpan::from_secs(2.0),
        )
        .unwrap();
    session.wait_for_renders();
    session.save(Some(&sequence_path)).unwrap();
    let report = serde_json::json!({"profile":paths.root(),"sequence":sequence_path,"muted":true,"controllers":0,"channels":system.channels.len()});
    std::fs::write(
        paths.root().join("fixture.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
}
