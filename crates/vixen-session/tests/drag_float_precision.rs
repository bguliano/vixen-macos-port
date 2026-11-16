//! Value drags must preserve model doubles, as the original in-process copies do.
use vixen_core::color::Xyz;
use vixen_core::gradient::{AlphaPoint, ColorPoint};
use vixen_core::{ColorGradient, Curve};
use vixen_session::effect_editor::drag::DragPayload;

#[test]
fn gradient_copy_preserves_exact_xyz_and_stop_values() {
    // The first Y value lost one ULP during an actual editor-to-library drag.
    let mut gradient = ColorGradient::from_parts(
        vec![
            ColorPoint::from_xyz(Xyz::new(11.552469624193364, 10.637748203969881, 40.97957280971114), 0.5, 0.0),
            ColorPoint::from_xyz(Xyz::new(30.294740132313915, 17.258157341482775, 13.043708133822754), 0.35, 0.35),
            ColorPoint::from_xyz(Xyz::new(25.328528172951895, 43.031201983151824, 22.593686671234973), 0.6, 1.0),
        ],
        vec![
            AlphaPoint { alpha: 1.0, position: 0.0, focus: 0.5 },
            AlphaPoint { alpha: 0.6, position: 0.6, focus: 0.35 },
            AlphaPoint { alpha: 207.0 / 255.0, position: 1.0, focus: 0.6 },
        ],
    );
    gradient.title = Some("Gradient Café".into());
    gradient.library_reference_name = "Private reference".into();
    gradient.is_current_library_gradient = true;
    gradient.gammacorrected = true;
    let payload = DragPayload::Gradient(gradient.clone());
    let DragPayload::Gradient(copied) = DragPayload::decode(&payload.encode()).unwrap() else { panic!("Gradient kind changed") };
    assert_eq!(copied, gradient);
    for (before, after) in gradient.colors().iter().zip(copied.colors()) {
        assert_eq!([before.color.x, before.color.y, before.color.z, before.position, before.focus].map(f64::to_bits),
                   [after.color.x, after.color.y, after.color.z, after.position, after.focus].map(f64::to_bits));
    }
    for (before, after) in gradient.alphas().iter().zip(copied.alphas()) {
        assert_eq!([before.alpha, before.position, before.focus].map(f64::to_bits),
                   [after.alpha, after.position, after.focus].map(f64::to_bits));
    }
}

#[test]
fn curve_copy_preserves_exact_coordinate_bits_and_metadata() {
    let mut curve = Curve::from_points(vec![(0.0, 13.0), (10.637748203969881, 40.97957280971114), (100.0, 37.0)]);
    curve.library_reference_name = "Curve Café".into();
    curve.is_current_library_curve = true;
    let payload = DragPayload::Curve(curve.clone());
    let DragPayload::Curve(copied) = DragPayload::decode(&payload.encode()).unwrap() else { panic!("Curve kind changed") };
    assert_eq!(copied, curve);
    assert_eq!(copied.points.iter().map(|p| (p.0.to_bits(), p.1.to_bits())).collect::<Vec<_>>(),
               curve.points.iter().map(|p| (p.0.to_bits(), p.1.to_bits())).collect::<Vec<_>>());
}
