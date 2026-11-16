//! Differential State output from unchanged upstream C# effects. Regenerate with
//! tools/golden/states/build.sh run; STATE_GOLDEN chooses a private generated file.
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};
use vixen_core::element::property_ids;
use vixen_core::element::{PropertyData, PropertyInstance};
use vixen_core::marks::{Mark, MarkCollection, MarkCollectionType};
use vixen_core::value::CommandValue;
use vixen_core::{Color, ElementColorType};
use vixen_core::{ElementTree, IntentData, StateValue, TimeSpan, Uuid};
use vixen_effects::framework::RenderCtx;

fn state(value: StateValue) -> Value {
    match value {
        StateValue::Range(v) => {
            json!({"kind":"range","value":v.value,"identity":format!("{:?}",v.tag_type),"tag":v.tag.as_ref(),"label":v.label.as_ref()})
        }
        StateValue::Command(CommandValue::Named8(v)) => {
            json!({"kind":"named8","value":v.value,"index":format!("{:?}",v.index_type),"tag":v.tag.as_ref(),"label":v.label.as_ref(),"min":v.range_minimum,"max":v.range_maximum})
        }
        StateValue::Lighting(v) => {
            json!({"kind":"lighting","color":v.color.to_argb_u32() as i32,"value":v.intensity})
        }
        StateValue::Discrete(v) => {
            json!({"kind":"discrete","color":v.color.to_argb_u32() as i32,"value":v.intensity})
        }
        StateValue::Rgb(v) => json!({"kind":"rgb","r":v.r,"g":v.g,"b":v.b}),
        other => panic!("unsupported fixture state {other:?}"),
    }
}
fn key(value: &Value) -> String {
    let mut value = value.clone();
    value.as_object_mut().unwrap().remove("value");
    value.to_string()
}
fn equal(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Number(a), Value::Number(b)) => {
            (a.as_f64().unwrap() - b.as_f64().unwrap()).abs() <= 1e-10
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|b| equal(v, b)))
        }
        _ => expected == actual,
    }
}
#[test]
fn state_effect_matches_unchanged_upstream_renderer() {
    let path = std::env::var_os("STATE_GOLDEN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/states.json.gz")
        });
    let file = std::fs::File::open(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e}; run tools/golden/states/build.sh run",
            path.display()
        )
    });
    let data: Value = serde_json::from_reader(flate2::read::GzDecoder::new(file)).unwrap();
    for color in data["colors"].as_array().unwrap() {
        let expected = Color::from_argb_u32(color["argb"].as_i64().unwrap() as u32);
        let state =
            fx_audio_state::property_data::parse_state(color["state"].as_str().unwrap()).unwrap();
        assert_eq!(state.definitions[0].items[0].color, expected);
        let face =
            fx_audio_state::property_data::parse_face(color["face"].as_str().unwrap()).unwrap();
        assert_eq!(face.default_color, expected);
        assert!(face.phoneme_state("AI") && face.phoneme_state("ai") && face.phoneme_state("E"));
        assert!(!face.phoneme_state(" AI "));
    }
    let mut tree = ElementTree::new();
    let mut nodes = HashMap::new();
    for spec in data["nodes"].as_array().unwrap() {
        let name = spec["name"].as_str().unwrap();
        let id = Uuid::parse_str(spec["id"].as_str().unwrap()).unwrap();
        let element = spec["element"]
            .as_str()
            .map(|id| Uuid::parse_str(id).unwrap());
        let node = tree.add_node(id, name, element);
        if let Some(xml) = spec["state"].as_str() {
            tree.node_mut(node).properties.push(PropertyInstance {
                type_id: property_ids::STATE,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Unknown {
                    raw_xml: xml.into(),
                },
            });
        }
        if let Some(color) = spec["color"].as_i64() {
            tree.node_mut(node).properties.push(PropertyInstance {
                type_id: property_ids::COLOR,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Color {
                    color_type: if spec["colorType"] == "MultipleDiscreteColors" {
                        ElementColorType::MultipleDiscreteColors
                    } else {
                        ElementColorType::SingleColor
                    },
                    single_color: Color::from_argb_u32(color as u32),
                    color_set_name: spec["colorSet"].as_str().map(str::to_string),
                },
            });
        }
        nodes.insert(name.to_string(), node);
    }
    for edge in data["edges"].as_array().unwrap() {
        tree.add_child(
            nodes[edge["parent"].as_str().unwrap()],
            nodes[edge["child"].as_str().unwrap()],
        );
    }
    let cancel = AtomicBool::new(false);
    let mut checked = 0;
    let mut values = 0;
    let mut failures = Vec::new();
    for case in data["cases"].as_array().unwrap() {
        let label = format!(
            "{}/{}",
            case["effect"].as_str().unwrap(),
            case["id"].as_str().unwrap()
        );
        if std::env::var("STATE_CASE").is_ok_and(|filter| !label.contains(&filter)) {
            continue;
        }
        let descriptor = vixen_effect_registry::all()
            .iter()
            .find(|d| d.name == case["effect"].as_str().unwrap())
            .unwrap();
        let targets: Vec<_> = case["targets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| nodes[name.as_str().unwrap()])
            .collect();
        let marks: Vec<_> = case["marks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|mc| MarkCollection {
                id: Uuid::parse_str(mc["id"].as_str().unwrap()).unwrap(),
                name: mc["name"].as_str().unwrap().into(),
                collection_type: MarkCollectionType::State,
                marks: mc["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| Mark {
                        id: Uuid::new_v4(),
                        start_time: TimeSpan::from_ticks(m["start"].as_i64().unwrap()),
                        duration: TimeSpan::from_ticks(m["duration"].as_i64().unwrap()),
                        text: m["text"].as_str().unwrap().into(),
                    })
                    .collect(),
                ..Default::default()
            })
            .collect();
        let context = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &targets,
            start_time: TimeSpan::from_ms_i(case["startMs"].as_i64().unwrap()),
            time_span: TimeSpan::from_ms_i(case["ms"].as_i64().unwrap()),
            frame_ms: case["frameMs"].as_i64().unwrap() as i32,
            marks: &marks,
            audio: None,
            module_data_root: None,
            cancel: &cancel,
        };
        let model = vixen_xml::XElement::parse(case["data"].as_str().unwrap()).unwrap();
        for mode in ["edited", "loaded"] {
            let mut params = vixen_io::effect_params_from_xml(&model, descriptor);
            if mode == "loaded" { descriptor.load_data(&mut params, &context.target_info()); }
            // Rendering does not mutate persisted data; the two lifecycles are independent.
            let output = (descriptor.render)(&context, &params);
            let expected = &case[mode];
            assert!(expected["ok"].as_bool().unwrap());
            assert!(expected["errors"].as_array().unwrap().is_empty());
            for row in expected["elements"].as_array().unwrap() {
                let node = nodes[row["name"].as_str().unwrap()];
                let id = tree.node(node).element_id.unwrap();
                let intents = output.map.get(&id).map(Vec::as_slice).unwrap_or(&[]);
                let mut raw: Vec<_> = intents
                    .iter()
                    .map(|i| {
                        let kind = match i.intent.data {
                            IntentData::Range { .. } => "RangeIntent",
                            IntentData::StaticRange(_) => "StaticIntent`1",
                            IntentData::Command(_) => "CommandIntent",
                            IntentData::Lighting { .. } => "LightingIntent",
                            IntentData::StaticLighting(_) => "StaticLightingIntent",
                            IntentData::StaticDiscrete(_) => "StaticDiscreteLightingIntent",
                            _ => "other",
                        };
                        json!({"start":i.start.ticks(),"end":i.end.ticks(),"kind":kind})
                    })
                    .collect();
                raw.sort_by_key(|v| {
                    (
                        v["start"].as_i64().unwrap(),
                        v["end"].as_i64().unwrap(),
                        v["kind"].as_str().unwrap().to_string(),
                    )
                });
                if !equal(&row["intents"], &json!(raw)) {
                    failures.push(format!(
                        "{label}/{mode}/{}: intent structure expected {}, got {}",
                        row["name"],
                        row["intents"],
                        json!(raw)
                    ));
                    continue;
                }
                for (at, tick) in expected["times"].as_array().unwrap().iter().enumerate() {
                    let time = TimeSpan::from_ticks(tick.as_i64().unwrap());
                    let mut actual: Vec<_> = intents
                        .iter()
                        .filter(|i| i.contains(time))
                        .map(|i| state(i.intent.state_at(time - i.start)))
                        .collect();
                    let mut want = row["frames"][at].as_array().unwrap().clone();
                    // Rust uses empty Arc<str> for C# default null string metadata.
                    for state in &mut want {
                        for field in ["tag", "label"] {
                            if state.get(field).is_some_and(Value::is_null) {
                                state[field] = json!("");
                            }
                        }
                    }
                    let order = |a: &Value, b: &Value| {
                        key(a).cmp(&key(b)).then_with(|| {
                            a["value"]
                                .as_f64()
                                .unwrap_or(0.0)
                                .total_cmp(&b["value"].as_f64().unwrap_or(0.0))
                        })
                    };
                    actual.sort_by(order);
                    want.sort_by(order);
                    values += want.len().max(actual.len());
                    if !equal(&json!(want), &json!(actual)) {
                        failures.push(format!(
                            "{label}/{mode}/{} at {tick}: expected {}, got {}",
                            row["name"],
                            json!(want),
                            json!(actual)
                        ));
                        break;
                    }
                }
            }
        }
        checked += 1;
    }
    assert!(checked > 0, "no State cases selected");
    eprintln!(
        "State oracle: {checked} cases, {values} state values, {} mismatches",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{}",
        failures
            .iter()
            .take(
                std::env::var("STATE_MAX")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(20)
            )
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
