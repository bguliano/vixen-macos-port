//! Fresh, controller-free library drag fixture and read-only saved-library inspector.
use std::{path::PathBuf, sync::Arc};
use vixen_core::{Color, ColorGradient, Curve, TimeSpan};
use vixen_io::{CurveLibrary, DataPaths, GradientLibrary, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("fresh private target profile"));
    let inspect = args.next();
    assert!(args.next().is_none());
    assert!(root.components().any(|part| part.as_os_str() == "target"));
    let paths = DataPaths::new(root);
    if inspect.as_deref() == Some("--inspect") {
        let system = vixen_io::load_system(&paths).unwrap();
        assert!(system.controllers.is_empty());
        let curves = vixen_io::curve_library(&system.module_store);
        let gradients = vixen_io::gradient_library(&system.module_store);
        println!("{}", serde_json::json!({
            "curves": curves.curves.iter().map(|(name, value)| serde_json::json!({
                "name":name, "at50":value.get_value(50.0), "link":value.library_reference_name,
                "current":value.is_current_library_curve})).collect::<Vec<_>>(),
            "gradients": gradients.gradients.iter().map(|(name, value)| serde_json::json!({
                "name":name, "argb":value.get_color_at(0.5).to_argb_u32(),
                "link":value.library_reference_name, "current":value.is_current_library_gradient})).collect::<Vec<_>>()
        }));
        return;
    }
    assert!(inspect.is_none());
    assert!(!paths.root().exists(), "refuse existing profile");
    paths.ensure_directories().unwrap();
    let mut system = SystemData::default();
    let colors = [Color::RED, Color::rgb(0, 255, 0), Color::BLUE, Color::rgb(255, 255, 0)];
    let curves = CurveLibrary {
        curves: ['A', 'B', 'C', 'D'].into_iter().enumerate().map(|(index, letter)| {
            let mut value = Curve::flat(10.0 * (index + 1) as f64);
            value.is_current_library_curve = true;
            (format!("Curve {letter}"), value)
        }).collect(),
    };
    let gradients = GradientLibrary {
        gradients: ['A', 'B', 'C', 'D'].into_iter().zip(colors).map(|(letter, color)| {
            let mut value = ColorGradient::solid(color);
            value.is_current_library_gradient = true;
            (format!("Gradient {letter}"), value)
        }).collect(),
    };
    vixen_io::set_curve_library(&mut system.module_store, &curves);
    vixen_io::set_gradient_library(&mut system.module_store, &gradients);
    vixen_io::save_system(&system, &paths).unwrap();
    vixen_session::sequencer::libraries::write_vfc(&paths.root().join("ColorPalette.xml"), &colors).unwrap();
    let environment = SessionEnvironment {
        video_cache: None, tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()), frame_ms: 50,
        media_dir: paths.media_dir(),
    };
    let mut session = SequenceSession::new_untitled(&environment);
    session.set_length(TimeSpan::from_secs(2.0));
    let sequence = paths.sequence_dir().join("Library Drag.tim");
    session.save(Some(&sequence)).unwrap();
    let report = serde_json::json!({
        "controllers":0, "muted":true, "sequence":sequence,
        "curves":curves.curves.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        "gradients":gradients.gradients.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        "colors":colors.iter().map(|color| color.to_argb_u32()).collect::<Vec<_>>(),
        "expectedCurveOrderAfterMoveLeft":["Curve C","Curve D","Curve A","Curve B"],
        "expectedCurveOrderAfterMoveToEnd":["Curve A","Curve B","Curve C","Curve D"],
        "expectedToolbarCopyName":"Imported Curve"
    });
    std::fs::write(paths.root().join("fixture.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{report}");
}
