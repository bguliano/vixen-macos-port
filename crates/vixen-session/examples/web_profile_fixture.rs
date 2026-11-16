//! Real Web API fixture: private profile, muted engine, loopback only. Enter stops it.
//! cargo run -p vixen-session --example web_profile_fixture -- target/web-real-profile
use parking_lot::Mutex;
use serde_json::json;
use std::{
    io::{BufRead, Write},
    path::PathBuf,
    sync::Arc,
};
use uuid::Uuid;
use vixen_core::element::{property_ids, ElementColorType, PropertyData, PropertyInstance};
use vixen_core::{Color, CombinedState, TimeSpan};
use vixen_effects::framework::ParamValue;
use vixen_engine::FrameSink;
use vixen_io::system::{Channel, Controller, ControllerOutput};
use vixen_io::{DataPaths, SystemData};
use vixen_session::{app::AppCore, sequence::SequenceSession, web::api::SessionApi};

#[derive(Default)]
struct Observations {
    frames: usize,
    colored_frames: usize,
    red_frames: usize,
}
struct Recorder(Arc<Mutex<Observations>>);
impl FrameSink for Recorder {
    fn frame(&mut self, _: TimeSpan, states: &[CombinedState], _: bool) {
        let mut out = self.0.lock();
        out.frames += 1;
        let colors: Vec<_> = states
            .iter()
            .filter_map(|s| {
                s.lighting
                    .map(|v| v.full_color())
                    .or_else(|| s.rgb.map(|v| v.full_color()))
            })
            .collect();
        if colors.iter().any(|c| c.r > 0 || c.g > 0 || c.b > 0) {
            out.colored_frames += 1;
        }
        if colors.contains(&Color::RED) {
            out.red_frames += 1;
        }
    }
}
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("new test profile folder"));
    assert!(!root.exists(), "test folder must not exist");
    assert!(
        root.components().any(|p| p.as_os_str() == "target"),
        "fixture belongs under target"
    );
    let paths = DataPaths::new(root);
    paths.ensure_directories().unwrap();
    let mut system = SystemData::default();
    let node = system.tree.add_element("Loopback Element");
    let id = system.tree.node(node).id;
    system.channels.push(Channel {
        id: system.tree.node(node).element_id.unwrap(),
        name: "Loopback Element".into(),
    });
    system
        .tree
        .node_mut(node)
        .properties
        .push(PropertyInstance {
            type_id: property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Color {
                color_type: ElementColorType::FullColor,
                single_color: Color::WHITE,
                color_set_name: None,
            },
        });
    system.controllers.push(Controller {
        name: "Loopback Controller".into(),
        hardware_type_id: vixen_output::ids::DEBUG_CONTROLLER,
        hardware_instance_id: Uuid::new_v4(),
        id: Uuid::new_v4(),
        outputs: vec![ControllerOutput {
            name: "Muted Output".into(),
            id: Uuid::new_v4(),
        }],
    });
    vixen_io::save_system(&system, &paths).unwrap();
    let core = Arc::new(AppCore::open(paths.root().to_path_buf(), true).unwrap());
    let mut sequence = SequenceSession::new_untitled(&core.session_environment());
    sequence.set_length(TimeSpan::from_secs(60.0));
    let descriptor = vixen_effect_registry::all()
        .iter()
        .find(|e| e.name == "Set Level")
        .unwrap();
    let effect = sequence
        .add_new_effect(
            descriptor.type_id,
            id,
            "Loopback Element",
            TimeSpan::ZERO,
            TimeSpan::from_secs(60.0),
        )
        .unwrap();
    let mut params = sequence.effect(effect).unwrap().node.params.clone();
    params.set(
        "Color",
        ParamValue::Rgb(vixen_core::color::Rgb::from_color(Color::RED)),
    );
    params.set("IntensityLevel", ParamValue::Double(1.0));
    sequence.set_params(effect, params);
    sequence
        .save(Some(&paths.sequence_dir().join("Loopback Sequence.tim")))
        .unwrap();
    drop(sequence);
    let observations = Arc::new(Mutex::new(Observations::default()));
    core.playback
        .add_sink(Box::new(Recorder(observations.clone())));
    core.playback.open_execution();
    let mut server = vixen_web::Server::start(
        "127.0.0.1:0".parse().unwrap(),
        PathBuf::from("app/Resources/wwwroot"),
        Arc::new(SessionApi::new(&core)),
        Arc::new(|line| eprintln!("{line}")),
    )
    .unwrap();
    println!("http://{}", server.address());
    std::io::stdout().flush().unwrap();
    let _ = std::io::stdin().lock().lines().next();
    server.stop();
    core.live.release_all();
    core.playback.close_execution();
    let out = observations.lock();
    let result = json!({"muted":true,"frames":out.frames,"coloredFrames":out.colored_frames,"redFrames":out.red_frames,
        "controllersStopped":core.playback.controller_states().iter().all(|(_,running,_)|!*running),"contextsAfterStop":core.playback.context_states().len()});
    std::fs::write(
        paths.root().join("frame-evidence.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!("{result}");
}
