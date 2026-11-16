//! Controller-free fixture and read-only inspector for a physical editor→library drag.
use std::{path::PathBuf, sync::Arc};
use vixen_core::{Color, ColorGradient, Curve, TimeSpan};
use vixen_effects::framework::ParamValue;
use vixen_io::{CurveLibrary, DataPaths, GradientLibrary, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("fresh private target profile"));
    let inspect = args.next();
    assert!(args.next().is_none());
    assert!(root.is_absolute() && root.components().any(|part| part.as_os_str() == "target"));
    let paths = DataPaths::new(root);
    let sequence_path = paths.sequence_dir().join("Editor Library.tim");
    let system = if inspect.as_deref() == Some("--inspect") {
        vixen_io::load_system(&paths).unwrap()
    } else {
        assert!(inspect.is_none() && !paths.root().exists(), "refuse existing profile");
        paths.ensure_directories().unwrap();
        let mut system = SystemData::default();
        system.tree.add_element("Private editor drag element");
        let curves = CurveLibrary { curves: ['A', 'B', 'C', 'D'].into_iter().enumerate().map(|(i, letter)| {
            let mut value = Curve::flat(10.0 * (i + 1) as f64);
            value.is_current_library_curve = true;
            (format!("Curve {letter}"), value)
        }).collect() };
        let colors = [Color::RED, Color::rgb(0, 255, 0), Color::BLUE, Color::rgb(255, 255, 0)];
        let gradients = GradientLibrary { gradients: ['A', 'B', 'C', 'D'].into_iter().zip(colors).map(|(letter, color)| {
            let mut value = ColorGradient::solid(color);
            value.is_current_library_gradient = true;
            (format!("Gradient {letter}"), value)
        }).collect() };
        vixen_io::set_curve_library(&mut system.module_store, &curves);
        vixen_io::set_gradient_library(&mut system.module_store, &gradients);
        vixen_session::web::store_settings(&mut system.module_store, vixen_session::web::Settings { port: 8888, enabled: false });
        vixen_io::save_system(&system, &paths).unwrap();
        vixen_session::sequencer::libraries::write_vfc(&paths.root().join("ColorPalette.xml"), &colors).unwrap();
        system
    };
    assert!(system.controllers.is_empty());
    let environment = SessionEnvironment {
        video_cache: None, tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()), frame_ms: 50, media_dir: paths.media_dir(),
    };
    let expected = Curve::from_points(vec![(0.0, 13.0), (25.0, 77.0), (100.0, 37.0)]);
    if inspect.is_none() {
        let (_, node) = system.tree.all_nodes().find(|(_, node)| node.name == "Private editor drag element").unwrap();
        let mut sequence = SequenceSession::new_untitled(&environment);
        sequence.set_length(TimeSpan::from_secs(3.0));
        let pulse = vixen_effect_registry::all().iter().find(|e| e.name == "Pulse").unwrap();
        let effect = sequence.add_new_effect(pulse.type_id, node.id, &node.name, TimeSpan::ZERO, TimeSpan::from_secs(2.0)).unwrap();
        let mut params = sequence.effect(effect).unwrap().node.params.clone();
        params.set("LevelCurve", ParamValue::Curve(expected.clone()));
        sequence.set_params(effect, params);
        sequence.save(Some(&sequence_path)).unwrap();
    }
    let sequence = SequenceSession::open(&sequence_path, &environment).unwrap();
    assert_eq!(sequence.effect_count(), 1);
    let effect = sequence.effects().next().unwrap();
    assert_eq!(effect.node.params.curve("LevelCurve"), expected);
    let library = vixen_io::curve_library(&system.module_store);
    let report = serde_json::json!({"controllers":0, "muted":true, "sequence":sequence_path,
        "effects":1, "effectType":"Pulse", "curveParam":"LevelCurve", "sourcePoints":expected.points,
        "sourceLink":expected.library_reference_name, "sourceCurrent":expected.is_current_library_curve,
        "curves":library.curves.iter().map(|(name, curve)| serde_json::json!({"name":name,"points":curve.points,
            "link":curve.library_reference_name,"current":curve.is_current_library_curve})).collect::<Vec<_>>()});
    if inspect.is_none() {
        std::fs::write(paths.root().join("fixture.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("{report}");
}
