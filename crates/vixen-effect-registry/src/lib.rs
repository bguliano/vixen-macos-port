//! Every effect module, the Rust form of Vixen's module discovery.

use std::sync::OnceLock;
use vixen_core::Uuid;
use vixen_effects::framework::EffectDescriptor;

pub fn all() -> &'static [&'static EffectDescriptor] {
    static ALL: OnceLock<Vec<&'static EffectDescriptor>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v = Vec::new();
        v.extend(fx_basic::descriptors());
        v.extend(fx_pixel_simple::descriptors());
        v.extend(fx_pixel_motion::descriptors());
        v.extend(fx_pixel_graphics::descriptors());
        v.extend(fx_pixel_complex::descriptors());
        v.extend(fx_pixel_physics::descriptors());
        v.extend(fx_audio_state::descriptors());
        v.extend(fx_fixture::descriptors());
        v.sort_by_key(|d| d.name);
        v
    })
}

pub fn by_type_id(id: &Uuid) -> Option<&'static EffectDescriptor> {
    all().iter().copied().find(|d| &d.type_id == id)
}

/// Finds an effect from a data model's `i:type` local name, for reading sequence files.
pub fn by_data_type(name: &str) -> Option<&'static EffectDescriptor> {
    all().iter().copied().find(|d| d.data_type == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn type_ids_and_names_are_unique() {
        let ids: HashSet<_> = all().iter().map(|d| d.type_id).collect();
        assert_eq!(ids.len(), all().len());
        let names: HashSet<_> = all().iter().map(|d| d.name).collect();
        assert_eq!(names.len(), all().len());
    }

    #[test]
    fn every_effect_builds_default_params() {
        for d in all() {
            let p = d.new_params();
            assert_eq!(p.values.len(), d.params.len(), "{}", d.name);
            let _ = d.visible_params(&p);
        }
    }

    /// Every value a setter can be given from the editor, then what the editor reads back. Rows get the same.
    fn exercise_editor(d: &'static EffectDescriptor, p: &mut vixen_effects::framework::Params, t: &vixen_effects::framework::TargetInfo) {
        use vixen_effects::framework::{ParamKind, ParamValue, Params, Visibility};
        for desc in d.params {
            let mut values: Vec<ParamValue> = d.selection_values(desc.name, None, p, t).into_iter().map(|c| c.value).collect();
            values.push(p.get(desc.name).clone());
            for value in values {
                d.set_param(p, desc.name, value, t);
                let _ = d.visible_params_for(p, t);
            }
            if let ParamKind::Record(r) | ParamKind::RecordList(r) = desc.kind {
                let mut rows: Vec<Params> = match p.get(desc.name) {
                    ParamValue::Record(row) => vec![(**row).clone()],
                    ParamValue::RecordList(rows) => rows.clone(),
                    _ => Vec::new(),
                };
                rows.push(Params::defaults(r.params));
                for row in &rows {
                    for field in r.params {
                        let _ = d.selection_values(field.name, Some(row), p, t);
                    }
                    if let Some(f) = r.visibility {
                        f(row, p, &mut Visibility::default());
                    }
                }
            }
        }
        let _ = d.forces_visual(p);
        if let Some(visual) = d.visual {
            visual(p, t, &mut vixen_effects::gfx::Graphics::new(120, 24), (0, 0, 120, 24));
        }
    }

    /// The editor calls these hooks while the user works, on whatever the effect has been dropped on. A misnamed parameter
    /// panics in `Params::get`, so this also catches a typo in a branch no effect's own test reaches.
    #[test]
    fn the_editor_hooks_survive_every_kind_of_target() {
        use fx_fixture::fixture_base::test_support::{fixtures, sample_spec};
        use vixen_core::marks::{Mark, MarkCollection, MarkCollectionType};
        use vixen_core::{Color, ElementTree, NodeId, TimeSpan};
        use vixen_effects::framework::TargetInfo;
        use vixen_effects::testing;

        let mut tree = ElementTree::new();
        let deep = testing::deep_grid(&mut tree, "Deep", 2, 2, 3);
        let string = tree.add_string("String", 3);
        let single = tree.add_element("Single");
        let discrete = testing::grid(&mut tree, "Discrete", 2, 2);
        testing::make_discrete(&mut tree, discrete, Color::RED);
        let located = testing::grid(&mut tree, "Located", 2, 3);
        testing::add_locations(&mut tree, located);
        let heads = fixtures(&mut tree, "Heads", 2, &sample_spec());
        let childless = tree.add_node(Uuid::new_v4(), "Childless", None);
        let places: [&[NodeId]; 9] =
            [&[], &[deep], &[string], &[single], &[discrete], &[located], &[heads], &[childless], &[deep, heads, discrete]];

        let mut beats = MarkCollection { name: "Beats".to_string(), ..Default::default() };
        beats.add_mark(Mark::new(TimeSpan::from_ms_i(100)));
        let phonemes = MarkCollection { name: "Phonemes".to_string(), collection_type: MarkCollectionType::Phoneme, ..Default::default() };
        let marks = [beats, phonemes];

        for d in all() {
            for targets in places {
                let _ = testing::render_with(d, &tree, targets, 500, |_| {});
                for (marks, audio_loaded) in [(&marks[..0], false), (&marks[..], true)] {
                    let t = TargetInfo { tree: &tree, targets, marks, start_time: TimeSpan::ZERO, time_span: TimeSpan::from_ms_i(2000), audio_loaded };
                    let mut loaded = d.new_params();
                    d.load_data(&mut loaded, &t);
                    exercise_editor(d, &mut loaded, &t);
                    let mut placed = d.new_params();
                    d.set_targets(&mut placed, &t);
                    d.set_marks(&mut placed, &t);
                    exercise_editor(d, &mut placed, &t);
                }
            }
        }
    }

    #[test]
    fn every_ported_effect_survives_the_smoke_test() {
        for d in all() {
            if d.ported {
                vixen_effects::testing::smoke_test(d);
            }
        }
    }
}
