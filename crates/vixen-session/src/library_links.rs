//! Curves and gradients linked to a library entry. In the C# each one reads through to the library whenever it's used, which is
//! why editing an entry changes every effect linked to it. Here the copy an effect holds is brought up to date before it renders,
//! when a sequence loads and when a library changes.

use vixen_core::{ColorGradient, Curve};
use vixen_effects::framework::{ParamValue, Params};
use vixen_io::{curve_library, gradient_library, CurveLibrary, GradientLibrary, ModuleStore, TimedSequence};

/// The libraries linked curves and gradients read from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Libraries {
    pub curves: CurveLibrary,
    pub gradients: GradientLibrary,
}

impl Libraries {
    pub fn from_store(store: &ModuleStore) -> Libraries {
        Libraries { curves: curve_library(store), gradients: gradient_library(store) }
    }
}

/// `Curve.UpdateLibraryReference`: takes the entry's points. A link to an entry that has gone is dropped, keeping the points it has.
fn refresh_curve(curve: &mut Curve, library: &CurveLibrary) -> bool {
    if !curve.is_library_reference() {
        return false;
    }
    match library.get(&curve.library_reference_name) {
        Some(entry) if entry.points != curve.points => {
            curve.points = entry.points.clone();
            true
        }
        Some(_) => false,
        None => {
            curve.library_reference_name.clear();
            false
        }
    }
}

/// `ColorGradient.UpdateLibraryReference`, which copies the entry's data and keeps this gradient's own link details.
fn refresh_gradient(gradient: &mut ColorGradient, library: &GradientLibrary) -> bool {
    if !gradient.is_library_reference() {
        return false;
    }
    match library.get(&gradient.library_reference_name) {
        Some(entry) => {
            let before = gradient.clone();
            gradient.clone_data_from(entry);
            *gradient != before
        }
        None => {
            gradient.library_reference_name.clear();
            false
        }
    }
}

fn refresh_value(value: &mut ParamValue, libraries: &Libraries) -> bool {
    let mut changed = false;
    match value {
        ParamValue::Curve(curve) => changed = refresh_curve(curve, &libraries.curves),
        ParamValue::Gradient(gradient) => changed = refresh_gradient(gradient, &libraries.gradients),
        ParamValue::GradientList(gradients) => {
            for gradient in gradients {
                changed |= refresh_gradient(gradient, &libraries.gradients);
            }
        }
        ParamValue::GradientLevels(pairs) => {
            for pair in pairs {
                changed |= refresh_gradient(&mut pair.color_gradient, &libraries.gradients);
                changed |= refresh_curve(&mut pair.curve, &libraries.curves);
            }
        }
        ParamValue::Record(record) => changed = refresh_links(record, libraries),
        ParamValue::RecordList(records) => {
            for record in records {
                changed |= refresh_links(record, libraries);
            }
        }
        _ => {}
    }
    changed
}

/// Brings an effect's linked curves and gradients up to date. Returns whether it would now render differently.
pub fn refresh_links(params: &mut Params, libraries: &Libraries) -> bool {
    let mut changed = false;
    for value in &mut params.values {
        changed |= refresh_value(value, libraries);
    }
    changed
}

/// The same for every effect of a sequence that has just been read from its file.
pub fn apply_library_links(sequence: &mut TimedSequence, store: &ModuleStore) {
    let libraries = Libraries::from_store(store);
    for node in &mut sequence.effects {
        refresh_links(&mut node.params, &libraries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{Color, GradientLevelPair};

    const RED: Color = Color::RED;

    fn linked_curve(name: &str, points: Vec<(f64, f64)>) -> Curve {
        let mut curve = Curve::from_points(points);
        curve.library_reference_name = name.to_string();
        curve
    }

    fn linked_gradient(name: &str, color: Color) -> ColorGradient {
        let mut gradient = ColorGradient::solid(color);
        gradient.library_reference_name = name.to_string();
        gradient
    }

    fn libraries() -> Libraries {
        Libraries {
            curves: CurveLibrary { curves: vec![("Ramp".to_string(), Curve::from_points(vec![(0.0, 10.0), (100.0, 90.0)]))] },
            gradients: GradientLibrary { gradients: vec![("Red".to_string(), ColorGradient::solid(RED))] },
        }
    }

    #[test]
    fn a_stale_curve_takes_the_entrys_points_and_stays_linked() {
        let mut curve = linked_curve("Ramp", vec![(0.0, 0.0), (100.0, 100.0)]);
        assert!(refresh_curve(&mut curve, &libraries().curves));
        assert_eq!(curve.points, vec![(0.0, 10.0), (100.0, 90.0)]);
        assert_eq!(curve.library_reference_name, "Ramp");
        assert!(!refresh_curve(&mut curve, &libraries().curves), "nothing changes the second time");
    }

    #[test]
    fn a_link_to_a_missing_entry_is_dropped_and_the_points_kept() {
        let mut curve = linked_curve("Gone", vec![(0.0, 5.0), (100.0, 5.0)]);
        assert!(!refresh_curve(&mut curve, &libraries().curves));
        assert!(!curve.is_library_reference());
        assert_eq!(curve.points, vec![(0.0, 5.0), (100.0, 5.0)]);

        let mut gradient = linked_gradient("Gone", Color::WHITE);
        assert!(!refresh_gradient(&mut gradient, &libraries().gradients));
        assert!(!gradient.is_library_reference());
    }

    #[test]
    fn an_unlinked_curve_is_left_alone() {
        let mut curve = Curve::from_points(vec![(0.0, 1.0), (100.0, 2.0)]);
        assert!(!refresh_curve(&mut curve, &libraries().curves));
        assert_eq!(curve.points, vec![(0.0, 1.0), (100.0, 2.0)]);
    }

    #[test]
    fn a_gradient_takes_the_entrys_data_and_keeps_its_own_link() {
        let mut gradient = linked_gradient("Red", Color::WHITE);
        assert!(refresh_gradient(&mut gradient, &libraries().gradients));
        assert_eq!(gradient.library_reference_name, "Red");
        assert_eq!(gradient.get_color_at(0.5).to_argb_u32(), RED.to_argb_u32());
        assert!(!refresh_gradient(&mut gradient, &libraries().gradients));
    }

    #[test]
    fn values_inside_pairs_are_reached() {
        let mut value = ParamValue::GradientLevels(vec![GradientLevelPair {
            color_gradient: linked_gradient("Red", Color::WHITE),
            curve: linked_curve("Ramp", vec![(0.0, 0.0), (100.0, 100.0)]),
        }]);
        assert!(refresh_value(&mut value, &libraries()));
        let ParamValue::GradientLevels(pairs) = &value else { unreachable!() };
        assert_eq!(pairs[0].curve.points, vec![(0.0, 10.0), (100.0, 90.0)]);
    }
}
