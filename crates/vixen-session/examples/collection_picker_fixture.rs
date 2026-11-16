//! Private settings fixture/read-only inspector for the native collection Picker.
use std::path::PathBuf;
use vixen_core::Color;
use vixen_session::sequencer::libraries::{load_color_collections, save_color_collections, ColorCollection};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().expect("private target settings directory"));
    let mode = args.next();
    assert!(args.next().is_none());
    assert!(path.is_absolute() && path.components().any(|p| p.as_os_str() == "target"));
    if mode.is_none() {
        assert!(!path.exists(), "refuse existing settings directory");
        std::fs::create_dir_all(&path).unwrap();
        let values = vec![
            ColorCollection { name: "Collection A".into(), description: "Private green collection".into(), colors: vec![Color::rgb(0, 255, 0)] },
            ColorCollection { name: "Collection B".into(), description: "Private blue collection".into(), colors: vec![Color::BLUE] },
        ];
        save_color_collections(&path, &values).unwrap();
        assert_eq!(load_color_collections(&path), values);
    } else {
        assert_eq!(mode.as_deref(), Some("--inspect"));
    }
    let values = load_color_collections(&path);
    assert!(!values.is_empty());
    println!("{}", serde_json::json!(values.iter().map(|v| serde_json::json!({
        "name": v.name, "description": v.description,
        "colors": v.colors.iter().map(|c| c.to_argb_u32()).collect::<Vec<_>>()
    })).collect::<Vec<_>>()));
}
