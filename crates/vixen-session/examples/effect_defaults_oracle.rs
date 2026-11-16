//! Private fixtures for the original-contract NBFX oracle; never starts an AppCore.
use std::path::{Path, PathBuf};
use vixen_core::{Color, ColorGradient, Curve};
use vixen_effects::framework::{EffectDescriptor, ParamValue};
use vixen_io::{effect_defaults::EffectDefaultsStore, DataPaths};

fn descriptor(name: &str) -> &'static EffectDescriptor {
    vixen_effect_registry::all().iter().copied().find(|d| d.name == name).unwrap()
}
fn fresh(path: &Path) -> DataPaths {
    assert!(path.components().any(|p| p.as_os_str() == "target"));
    std::fs::create_dir(path).expect("fresh output profile with existing parent");
    DataPaths::new(path)
}
fn describe(store: &EffectDefaultsStore) -> serde_json::Value {
    serde_json::json!({"entries": store.entries().iter().map(|e| {
        let params = vixen_effect_registry::by_type_id(&e.type_id).and_then(|d| store.create_params(d));
        serde_json::json!({"typeId": e.type_id.to_string(), "name": e.effect_name, "model": e.data_model_type_name,
            "savedUtc": e.saved_utc, "payload": vixen_xml::nbfx::base64_encode(&e.payload),
            "values": params.map(|p| {
                let mut xml = vixen_xml::XElement::new("", "Settings");
                for member in vixen_io::params::params_to_members(&p) { xml.push(member); }
                xml.to_document()
            })})
    }).collect::<Vec<_>>()})
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args[0].as_str() {
        "create" => {
            assert_eq!(args.len(), 2);
            let paths = fresh(Path::new(&args[1]));
            let mut store = EffectDefaultsStore::load(&paths);
            let pulse = descriptor("Pulse");
            let mut p = pulse.new_params();
            p.set("LevelCurve", ParamValue::Curve(Curve::flat(42.0)));
            p.set("ColorGradient", ParamValue::Gradient(ColorGradient::solid(Color::BLUE)));
            store.save(pulse, &p).unwrap();
            let pattern = descriptor("Pattern");
            let mut p = pattern.new_params();
            p.set("PatternType", ParamValue::Enum("Brick".into()));
            p.set("WeaveThickness", ParamValue::Int(37));
            p.set("AdvancedSizing", ParamValue::Bool(true));
            p.set("WeaveHorizontalThickness", ParamValue::Int(21));
            p.set("WeaveVerticalThickness", ParamValue::Int(63));
            p.set("HorizontalColors", ParamValue::GradientList(vec![ColorGradient::solid(Color::BLUE), ColorGradient::solid(Color::RED)]));
            store.save(pattern, &p).unwrap();
            let wave = descriptor("Wave");
            let mut p = wave.new_params();
            let mut rows = p.records("WaveformData").to_vec();
            assert_eq!(rows.len(), 2);
            rows[0].set("PhaseShift", ParamValue::Int(13));
            rows[1].set("PhaseShift", ParamValue::Int(97));
            rows[0].set("MarkCollectionId", ParamValue::Guid(uuid::uuid!("d94971bd-202f-4bea-8007-36c4bc6dd909")));
            p.set("WaveformData", ParamValue::RecordList(rows));
            store.save(wave, &p).unwrap();
            std::fs::write(paths.root().join("port.json"), serde_json::to_vec_pretty(&describe(&store)).unwrap()).unwrap();
        }
        "roundtrip" => {
            assert_eq!(args.len(), 3);
            let source = PathBuf::from(&args[1]);
            let paths = fresh(Path::new(&args[2]));
            let mut store = EffectDefaultsStore::load(&paths);
            store.import(&source).unwrap();
            std::fs::write(paths.root().join("imported.json"), serde_json::to_vec_pretty(&describe(&store)).unwrap()).unwrap();
            store.export(&paths.root().join("imported.vfd"), &store.entries().iter().map(|e| e.type_id).collect::<Vec<_>>()).unwrap();
            let entries = store.entries().to_vec();
            for entry in entries {
                if let Some(d) = vixen_effect_registry::by_type_id(&entry.type_id) {
                    let params = store.create_params(d).expect("known C# data model decoded");
                    store.save(d, &params).unwrap();
                }
            }
            std::fs::write(paths.root().join("resaved.json"), serde_json::to_vec_pretty(&describe(&store)).unwrap()).unwrap();
        }
        other => panic!("unknown mode {other}"),
    }
}
