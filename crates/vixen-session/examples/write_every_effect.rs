//! Writes a sequence the port built itself, for checking with the original Vixen's loader (`tools/golden/filecheck.sh`). It has every
//! effect at its defaults, and every effect with every value changed, because the original's serializer quietly ignores a member
//! it finds out of order, and nothing else would show that.  write_every_effect <profile folder> <output .tim>
use std::path::PathBuf;
use std::sync::Arc;
use vixen_core::layer::{ChromaKeySettings, Layer, MixingFilter};
use vixen_core::marks::MarkCollectionType;
use vixen_core::{Color, ColorGradient, Curve, GradientLevelPair, Mark, MarkCollection, TimeSpan};
use vixen_effects::framework::{ParamKind, ParamValue, Params};
use vixen_io::{load_system, system::effective_store, DataPaths};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn changed_curve() -> Curve {
    Curve::from_points(vec![(0.0, 11.0), (50.0, 73.0), (100.0, 37.0)])
}

fn changed_gradient() -> ColorGradient {
    ColorGradient::from_colors(&[Color::rgb(12, 34, 56), Color::rgb(78, 90, 123)])
}

fn changed_value(kind: &ParamKind, value: &ParamValue) -> ParamValue {
    match (kind, value) {
        (_, ParamValue::Bool(b)) => ParamValue::Bool(!b),
        (_, ParamValue::Int(i)) => ParamValue::Int(i + 1),
        (_, ParamValue::Long(i)) => ParamValue::Long(i + 1),
        (_, ParamValue::Double(d)) => ParamValue::Double(d + 0.25),
        (_, ParamValue::Str(s)) => ParamValue::Str(format!("{s}x")),
        (ParamKind::Enum(desc), ParamValue::Enum(name)) => {
            let at = desc.members.iter().position(|m| m.name == name).unwrap_or(0);
            ParamValue::Enum(desc.members[(at + 1) % desc.members.len()].name.to_string())
        }
        (_, ParamValue::Curve(_)) => ParamValue::Curve(changed_curve()),
        (_, ParamValue::Gradient(_)) => ParamValue::Gradient(changed_gradient()),
        (_, ParamValue::GradientList(list)) => {
            let mut list = list.clone();
            list.push(changed_gradient());
            ParamValue::GradientList(list)
        }
        (_, ParamValue::GradientLevels(list)) => {
            let mut list = list.clone();
            list.push(GradientLevelPair { color_gradient: changed_gradient(), curve: changed_curve() });
            ParamValue::GradientLevels(list)
        }
        (_, ParamValue::Color(_)) => ParamValue::Color(Color::rgb(12, 34, 56)),
        (_, ParamValue::ColorList(list)) => {
            let mut list = list.clone();
            list.push(Color::rgb(12, 34, 56));
            ParamValue::ColorList(list)
        }
        (_, ParamValue::Percentage(p)) => ParamValue::Percentage(p / 2.0 + 0.125),
        (_, ParamValue::Font(f)) => {
            let mut f = f.clone();
            f.size += 1.0;
            f.bold = !f.bold;
            f.italic = !f.italic;
            ParamValue::Font(f)
        }
        (_, ParamValue::Time(t)) => ParamValue::Time(*t + TimeSpan::from_ms_i(1250)),
        (_, ParamValue::Record(r)) => ParamValue::Record(Box::new(changed_params(r))),
        (_, ParamValue::RecordList(list)) => ParamValue::RecordList(list.iter().map(changed_params).collect()),
        _ => value.clone(),
    }
}

fn changed_params(params: &Params) -> Params {
    let mut out = params.clone();
    for (i, desc) in params.desc.iter().enumerate() {
        out.values[i] = changed_value(&desc.kind, &params.values[i]);
    }
    out
}

fn mark_collections() -> Vec<MarkCollection> {
    let kinds = [MarkCollectionType::Generic, MarkCollectionType::Phrase, MarkCollectionType::Word, MarkCollectionType::Phoneme];
    kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let mut marks = MarkCollection::default();
            marks.name = format!("Marks {i} <&> \"quoted\"");
            marks.level = i as i32 + 1;
            marks.collection_type = *kind;
            marks.show_grid_lines = i % 2 == 0;
            marks.show_marks = i % 2 == 1;
            marks.show_tail_lines = i == 2;
            marks.is_default = i == 0;
            marks.is_locked = i == 3;
            marks.decorator.color = Color::rgb(200, 100 + i as u8, 50);
            marks.decorator.is_bold = i % 2 == 0;
            marks.decorator.is_solid_line = i % 2 == 1;
            for m in 0..4 {
                let mut mark = Mark::new(TimeSpan::from_ms_i(1500 + 250 * m));
                mark.duration = TimeSpan::from_ms_i(200 + 7 * m);
                mark.text = format!("mark {m} é");
                marks.marks.push(mark);
            }
            marks
        })
        .collect()
}

/// One layer for each mixing filter, with the settings that have any changed.
fn add_layers(session: &mut SequenceSession) -> Vec<uuid::Uuid> {
    let filters = [
        MixingFilter::ProportionalMix,
        MixingFilter::HighestValue,
        MixingFilter::Mask,
        MixingFilter::MaskFill { exclude_zero_values: true, requires_mixing_partner: true },
        MixingFilter::MultiplyColor,
        MixingFilter::IntensityOverlay,
        MixingFilter::ColorChange,
        MixingFilter::LumaKey { lower_limit: 0.25, upper_limit: 0.75 },
        MixingFilter::ChromaKey(ChromaKeySettings { lower_limit: 0.125, upper_limit: 0.875, ..ChromaKeySettings::default() }),
    ];
    let mut ids = Vec::new();
    for (i, filter) in filters.into_iter().enumerate() {
        let layer = Layer::standard(&format!("Layer {i}"), filter);
        ids.push(layer.id);
        session.document_mut().layers.add_layer(layer);
    }
    session.layers_changed();
    ids
}

fn main() {
    let mut args = std::env::args().skip(1);
    let paths = DataPaths::new(PathBuf::from(args.next().expect("profile folder")));
    let output = PathBuf::from(args.next().expect("output file"));
    let system = load_system(&paths).expect("system");
    let env = SessionEnvironment { video_cache: None,
        tree: Arc::new(system.tree.clone()),
        store: Arc::new(effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: system.default_update_interval,
        media_dir: paths.media_dir(),
    };
    // A full colour string, so that colours are valid whatever they are changed to.
    let (target_id, target_name) =
        system.tree.all_nodes().find(|(_, n)| n.name == "Light Strip L").map(|(_, n)| (n.id, n.name.clone())).expect("the strip");

    let mut session = SequenceSession::new_untitled(&env);
    session.set_marks(mark_collections());
    let layers = add_layers(&mut session);

    // Pass names to leave out, for effects the compiled original doesn't include.
    let skipped: Vec<String> = args.collect();
    let mut start = 0;
    let mut placed = 0;
    for descriptor in vixen_effect_registry::all().iter().filter(|d| !skipped.iter().any(|s| s == d.name)) {
        let span = TimeSpan::from_ms_i(1000);
        let at = TimeSpan::from_ms_i(start);
        session.add_new_effect(descriptor.type_id, target_id, &target_name, at, span).expect("default effect");
        let changed = session.add_new_effect(descriptor.type_id, target_id, &target_name, at + span, span).expect("changed effect");
        let params = changed_params(&session.effect(changed).unwrap().node.params);
        session.set_params(changed, params);
        session.set_layer(&[changed], layers[placed % layers.len()]);
        placed += 1;
        start += 2000;
        println!("{}: {}", descriptor.name, descriptor.type_id);
    }
    session.set_length(TimeSpan::from_ms_i(start));
    session.save(Some(&output)).expect("save");
    println!("wrote {} effects to {}", session.effect_count(), output.display());
}
