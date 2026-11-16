//! Per-shape tests. Expected layouts were computed by transcribing the C# formulas separately (in Python,
//! whose `round` is banker's like `Math.Round` and whose `int` truncates like a C# cast), not by running this
//! code. A comment says where a value is worth working through by hand.

use super::xml::shape_from_xml;
use super::*;
use crate::point::PointType;
use crate::tools::{Matrix, Rect};
use vixen_core::Orientation;

fn p(x: i32, y: i32) -> PreviewPoint {
    PreviewPoint::new(x, y)
}

fn xy(s: &Shape) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    s.for_each_pixel(&mut |q| v.push((q.x, q.y)));
    v
}

const NONE: Modifiers = Modifiers::NONE;

/// As drawing it with the mouse does: the default select point follows the mouse.
fn drawn(mut s: Shape, x: i32, y: i32) -> Shape {
    s.select(true);
    s.select_default_select_point();
    let start = s.point_of(PointRef::Pt(0));
    s.mouse_move(x, y, x - start.x, y - start.y, NONE);
    s.deselect();
    s.selected_point = None;
    s
}

/// Grabs the shape and moves it, as a drag does.
fn drag(s: &mut Shape, dx: i32, dy: i32) {
    s.set_select_point(SelectArg::Null);
    s.mouse_move(1000 + dx, 1000 + dy, dx, dy, NONE);
}

/// Moves it, then checks everything moved with it. Within one pixel, because truncation of a float that has
/// had an integer added is not exactly translation-invariant, in C# either.
fn check_move(mut s: Shape) {
    let before = xy(&s);
    assert!(!before.is_empty(), "{} has no lights", s.type_name());
    let (l, t) = (s.left(), s.top());
    let axis = s.rotation_axis;
    drag(&mut s, 40, 60);
    assert_eq!((s.left(), s.top()), (l + 40, t + 60), "{}", s.type_name());
    if let (Some(a), Some(b), false) = (axis, s.rotation_axis, matches!(s.kind, ShapeKind::Single | ShapeKind::Flood | ShapeKind::Custom { .. })) {
        assert_eq!((b.x, b.y), (a.x + 40, a.y + 60), "{} rotation axis follows", s.type_name());
    }
    for (a, b) in before.iter().zip(xy(&s)) {
        assert!((b.0 - a.0 - 40).abs() <= 1 && (b.1 - a.1 - 60).abs() <= 1, "{}: {a:?} to {b:?}", s.type_name());
    }
    // A second drag event is relative to the same start, not to the first event.
    s.mouse_move(1000 + 10, 1000, 10, 0, NONE);
    assert_eq!((s.left(), s.top()), (l + 10, t), "{}", s.type_name());
}

/// Drags a handle, then checks the point followed and the lights agree with a fresh layout.
fn check_resize(mut s: Shape, handle: usize, x: i32, y: i32) {
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(handle)));
    s.mouse_move(x, y, 0, 0, NONE);
    let q = s.points[handle];
    assert_eq!((q.x, q.y), (x, y), "{}", s.type_name());
    let moved = xy(&s);
    s.selected_point = None;
    s.layout();
    assert_eq!(moved, xy(&s), "{}", s.type_name());
}

/// Rotates it 90 degrees with the handle, by putting the mouse to the right of the center. Each light must
/// land where rotating it about the center does, give or take a pixel for lines laid out between rotated ends.
fn check_rotate(mut s: Shape) {
    let before = xy(&s);
    let c = s.center();
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::RotateHandle));
    s.mouse_move(c.x + 200, c.y, 200, 0, NONE);
    assert_eq!(s.rotation_angle, 90, "{}", s.type_name());
    let m = Matrix::rotate_at(90.0, c.x as f64, c.y as f64);
    for (a, b) in before.iter().zip(xy(&s)) {
        let (ex, ey) = m.apply(a.0 as f64, a.1 as f64);
        assert!((b.0 as f64 - ex).abs() <= 1.5 && (b.1 as f64 - ey).abs() <= 1.5, "{}: {a:?} to {b:?}, expected about ({ex},{ey})", s.type_name());
    }
}

/// Out and back in through the XML: the same members, the same lights.
fn check_xml(s: &Shape) {
    let e = s.to_xml_named("Shape");
    let back = shape_from_xml(&e);
    assert!(!matches!(back.kind, ShapeKind::Unknown(_)), "{} did not read back: {}", s.type_name(), e.to_fragment());
    assert_eq!(e.first_difference(&back.to_xml_named("Shape"), String::new()), None, "{}", s.type_name());
    assert_eq!(xy(&back), xy(s), "{}", s.type_name());
    // It also survives the text writer.
    let text = XElement::parse(&e.to_fragment()).unwrap();
    assert_eq!(e.first_difference(&text, String::new()), None, "{}", s.type_name());
}

use vixen_xml::XElement;

fn member_names(s: &Shape) -> Vec<String> {
    s.to_xml_named("Shape").elements().map(|e| e.local.clone()).collect()
}

fn line() -> Shape {
    Shape::new_line(p(10, 20), p(110, 70), 5, None, 1.0)
}

fn arch() -> Shape {
    let mut s = drawn(Shape::new_arch(p(10, 10), None, 1.0), 110, 60);
    s.set_line_pixel_count(5);
    s
}

fn poly_line() -> Shape {
    let mut s = Shape::new_poly_line(p(0, 0), None, 1.0);
    s.selected_point = Some(PointRef::Pt(0));
    s.mouse_up(100, 0, true, None);
    s.mouse_move(100, 0, 0, 0, NONE);
    s.mouse_up(100, 50, true, None);
    s.mouse_move(100, 50, 0, 0, NONE);
    s.mouse_up(100, 50, true, None);
    s.end_creation_poly_line();
    s.set_poly_line_pixel_count(7);
    s.deselect();
    s.selected_point = None;
    s
}

fn cane() -> Shape {
    let mut s = Shape::new_cane(p(0, 0), None, 1.0);
    s.points[BR] = p(40, 100).with_type(PointType::SizeBottomRight);
    s.points[cane::ARCH] = p(0, 30).with_type(PointType::Size);
    s.layout();
    s
}

fn rectangle() -> Shape {
    let mut s = Shape::new_rectangle(p(0, 0), None, 1.0);
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(100, 50, 100, 50, NONE);
    s.deselect();
    s.selected_point = None;
    for i in 0..4 {
        s.set_string_light_count(i, 5);
    }
    s
}

fn ellipse() -> Shape {
    let mut s = drawn(Shape::new_ellipse(p(10, 20), None, 1.0), 90, 60);
    s.set_line_pixel_count(6);
    s
}

fn triangle() -> Shape {
    let mut s = Shape::new_triangle(p(0, 0), None, 1.0);
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(100, 80, 100, 80, NONE);
    s.deselect();
    s.selected_point = None;
    s.set_string_light_count(0, 4);
    s.set_string_light_count(1, 4);
    s.set_string_light_count(2, 4);
    s
}

fn star() -> Shape {
    let mut s = Shape::new_star(p(0, 0), None, 1.0);
    if let ShapeKind::Star { pixel_count, .. } = &mut s.kind {
        *pixel_count = 40;
    }
    s.points[BR] = p(100, 100).with_type(PointType::SizeBottomRight);
    s.set_star_pixel_count(20);
    s
}

fn star_burst() -> Shape {
    let mut s = Shape::new_star_burst(p(0, 0), None, 1.0);
    s.set_star_burst_light_count(3);
    s.set_star_burst_string_count(4);
    if let ShapeKind::StarBurst { inner_circle_size, .. } = &mut s.kind {
        *inner_circle_size = 20;
    }
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(100, 60, 100, 60, NONE);
    s.deselect();
    s.selected_point = None;
    s
}

fn net() -> Shape {
    let mut s = Shape::new_net(p(10, 10), None, 1.0);
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(34, 26, 24, 16, NONE);
    s.deselect();
    s.selected_point = None;
    s
}

fn mega_tree() -> Shape {
    let mut s = Shape::new_mega_tree(p(0, 0), None, 1.0);
    s.set_mega_tree_string_count(4);
    s.set_mega_tree_lights_per_string(3);
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(100, 200, 100, 200, NONE);
    s.deselect();
    s.selected_point = None;
    s
}

fn multi_string() -> Shape {
    let mut s = Shape::new_multi_string(p(0, 0), None, 1.0);
    s.select(true);
    s.mouse_up(60, 0, true, None);
    s.mouse_move(60, 0, 0, 0, NONE);
    s.mouse_up(60, 40, true, None);
    s.mouse_move(60, 40, 0, 0, NONE);
    s.mouse_up(90, 90, true, None);
    s.end_creation_multi_string();
    for line in s.strings.as_mut().unwrap() {
        line.set_line_pixel_count(4);
    }
    s.layout();
    s.deselect();
    s.selected_point = None;
    s
}

fn icicle() -> Shape {
    let mut s = Shape::new_icicle(p(0, 10), p(0, 10), None, 1.0);
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(30, 10, 30, 0, NONE); // 30 / 10 + 1 = 4 strings
    s.mouse_up(30, 10, true, None);
    if let ShapeKind::Icicle { vertical_spacing, .. } = &mut s.kind {
        *vertical_spacing = 5;
    }
    for line in s.strings.as_mut().unwrap() {
        line.set_line_pixel_count(3);
    }
    s.layout();
    s.deselect();
    s.selected_point = None;
    s
}

fn pixel_grid() -> Shape {
    let mut s = Shape::new_pixel_grid(p(0, 0), None, 1.0);
    s.set_pixel_grid_string_count(4);
    s.set_pixel_grid_lights_per_string(3);
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(90, 40, 90, 40, NONE);
    s.deselect();
    s.selected_point = None;
    s
}

fn custom() -> Shape {
    Shape::new_custom(p(10, 10), vec![line(), arch()])
}

fn custom_prop() -> Shape {
    let mut s = Shape::new_custom_prop(1.0);
    s.add_light_nodes(&CustomPropLightNode { node: Uuid::from_u128(7), light_size: 2, lights: vec![(10.0, 20.0), (50.5, 20.0), (50.5, 60.25), (10.0, 60.25)] });
    s.layout();
    s
}

fn moving_head() -> Shape {
    let mut s = Shape::new_moving_head(p(10, 20), None, 1.0);
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(50, 60, 40, 40, NONE);
    s.end_add_new();
    s.deselect();
    s.selected_point = None;
    s
}

// ----- layouts

#[test]
fn line_layout_rounds_like_math_round() {
    // x spacing -25, y spacing -12.5, so y = 32.5 and 57.5: banker's rounding takes them to 32 and 58.
    assert_eq!(xy(&line()), [(10, 20), (35, 32), (60, 45), (85, 58), (110, 70)]);
}

#[test]
fn line_zooms_and_rotates_each_light() {
    let mut s = line();
    s.set_zoom_level(2.0);
    assert_eq!(xy(&s), [(20, 40), (70, 64), (120, 90), (170, 116), (220, 140)]);
    s.set_zoom_level(1.0);
    s.select(true);
    s.set_rotation_angle(90);
    assert_eq!(xy(&s), [(85, -5), (73, 20), (60, 45), (47, 70), (35, 95)]);
    s.set_rotation_angle(30);
    assert_eq!(xy(&s), [(29, -2), (45, 21), (60, 45), (75, 69), (91, 92)]);
}

#[test]
fn line_with_one_light_divides_by_zero_harmlessly() {
    let s = Shape::new_line(p(5, 6), p(50, 60), 1, None, 1.0);
    assert_eq!(xy(&s), [(5, 6)]);
}

#[test]
fn poly_line_layout() {
    assert_eq!(xy(&poly_line()), [(0, 0), (25, 0), (50, 0), (75, 0), (100, 0), (100, 25), (100, 50)]);
}

#[test]
fn poly_line_spacing_carries_across_corners() {
    let mut s = Shape::new_poly_line(p(0, 0), None, 1.0);
    s.creating = false;
    s.points = vec![p(0, 0), p(30, 40), p(90, 40), p(90, 100)];
    s.set_poly_line_pixel_count(9);
    assert_eq!(xy(&s), [(0, 0), (12, 16), (25, 33), (44, 40), (65, 40), (86, 40), (90, 58), (90, 79), (90, 100)]);
}

#[test]
fn arch_layout() {
    assert_eq!(xy(&arch()), [(10, 60), (24, 24), (59, 10), (95, 24), (110, 59)]);
}

#[test]
fn arch_rotation() {
    // From a file, because the rotation axis of one just drawn is still where the drawing began.
    let mut s = shape_from_xml(&arch().to_xml_named("Shape"));
    s.select(true);
    s.set_rotation_angle(45);
    assert_eq!(xy(&s), [(7, 17), (42, 2), (77, 17), (93, 52), (78, 87)]);
}

#[test]
fn cane_layout() {
    assert_eq!(
        xy(&cane()),
        [(0, 100), (0, 92), (0, 83), (0, 74), (0, 65), (0, 57), (0, 48), (0, 39), (0, 30), (1, 16), (7, 6), (15, 0), (24, 0), (32, 6), (38, 16), (40, 29)]
    );
}

#[test]
fn rectangle_layout_clockwise_and_counter_clockwise() {
    let mut s = rectangle();
    assert_eq!(
        xy(&s),
        [(0, 50), (0, 38), (0, 25), (0, 12), (0, 0), (17, 0), (34, 0), (50, 0), (66, 0), (83, 0), (100, 0), (100, 12), (100, 25), (100, 38), (100, 50), (83, 50), (66, 50), (50, 50), (34, 50), (17, 50)]
    );
    s.set_property("Direction", properties::PropertyValue::Enum("CounterClockwise".to_string()), "").unwrap();
    assert_eq!(
        xy(&s),
        [(0, 50), (25, 50), (50, 50), (75, 50), (100, 50), (100, 41), (100, 33), (100, 25), (100, 17), (100, 9), (100, 0), (75, 0), (50, 0), (25, 0), (0, 0), (0, 9), (0, 17), (0, 25), (0, 33), (0, 41)]
    );
}

#[test]
fn ellipse_layout() {
    // Six lights from GetEllipsePoints' seven points: the seventh repeats the first.
    assert_eq!(xy(&ellipse()), [(90, 40), (70, 57), (30, 57), (10, 40), (29, 22), (69, 22)]);
}

#[test]
fn triangle_layout() {
    // Drawing it from (0,0) to (100,80) puts point 1 at the top middle, point 2 bottom right, point 3 bottom left.
    assert_eq!(xy(&triangle()), [(50, 0), (67, 27), (83, 53), (100, 80), (100, 80), (67, 80), (33, 80), (0, 80), (0, 80), (17, 53), (33, 27), (50, 0)]);
}

#[test]
fn star_layout_both_directions() {
    let mut s = star();
    assert_eq!(
        xy(&s),
        [(69, 56), (74, 73), (79, 90), (64, 80), (50, 70), (35, 80), (20, 90), (25, 73), (30, 56), (16, 45), (2, 34), (20, 34), (38, 33), (44, 16), (49, 0), (55, 16), (61, 33), (79, 34), (97, 34), (83, 45)]
    );
    s.set_property("String Direction", properties::PropertyValue::Enum("CounterClockwise".to_string()), "").unwrap();
    assert_eq!(
        xy(&s),
        [(69, 56), (83, 45), (97, 34), (79, 34), (61, 33), (55, 16), (49, 0), (44, 16), (38, 33), (20, 34), (2, 34), (16, 45), (30, 56), (25, 73), (20, 90), (35, 80), (50, 70), (64, 80), (79, 90), (74, 73)]
    );
}

#[test]
fn star_rounds_its_light_count_down_to_an_even_number_per_point() {
    let mut s = star();
    s.set_star_pixel_count(23);
    // 23 / 5 = 4 per point, so 20 lights.
    assert_eq!(s.pixel_count(), 20);
    assert!(matches!(s.kind, ShapeKind::Star { pixel_count: 20, pixels_per_point: 4, .. }));
}

#[test]
fn small_star_does_not_lay_out() {
    let mut s = Shape::new_star(p(0, 0), None, 1.0);
    s.points[BR] = p(10, 100);
    s.layout();
    // Width 10 is not more than 10, so the 40 lights keep the (10, 10) they were added at.
    assert_eq!(s.pixel_count(), 40);
    assert!(xy(&s).iter().all(|q| *q == (10, 10)));
}

#[test]
fn star_burst_layout() {
    assert_eq!(xy(&star_burst()), [(60, 30), (80, 30), (100, 30), (50, 40), (50, 50), (50, 60), (40, 30), (20, 30), (0, 30), (50, 20), (50, 10), (49, 0)]);
}

#[test]
fn net_layout_staggers_rows() {
    // Spacing 8. The first row, an odd count, takes x with (x + 4) % 8 == 0, the next (x + 8) % 8 == 0.
    assert_eq!(xy(&net()), [(14, 10), (22, 10), (30, 10), (10, 18), (18, 18), (26, 18)]);
}

#[test]
fn net_is_clipped_to_its_quadrilateral() {
    let mut s = Shape::new_net(p(0, 0), None, 1.0);
    s.points[TL] = p(0, 0);
    s.points[TR] = p(16, 0);
    s.points[BR] = p(16, 16);
    s.points[BL] = p(8, 16);
    s.layout();
    assert_eq!(xy(&s), [(4, 0), (12, 0), (8, 8)]);
}

#[test]
fn net_keeps_its_link_through_a_layout() {
    let mut s = net();
    for q in &mut s.pixels {
        q.node_id = Uuid::from_u128(9);
    }
    s.layout();
    assert!(!s.pixels.is_empty() && s.pixels.iter().all(|q| q.node_id == Uuid::from_u128(9)));
}

#[test]
fn mega_tree_layout() {
    // Strings run backwards round the ellipses: string 0 goes to the last of the first four points.
    assert_eq!(xy(&mega_tree()), [(49, 160), (50, 80), (50, 0), (0, 180), (20, 92), (40, 5), (50, 200), (50, 105), (50, 10), (100, 180), (80, 92), (60, 5)]);
}

#[test]
fn mega_tree_partial_coverage() {
    let mut s = mega_tree();
    s.set_property("String Coverage Degrees", properties::PropertyValue::Int(180), "").unwrap();
    s.set_property("String Count", properties::PropertyValue::Int(3), "").unwrap();
    // total = ceil(360 / 180 * 3) = 6, and three points on each half ellipse.
    let lines: Vec<(i32, i32, i32, i32)> = s.strings().unwrap().iter().map(|l| (l.points[0].x, l.points[0].y, l.points[1].x, l.points[1].y)).collect();
    assert_eq!(lines, [(0, 180, 40, 5), (50, 200, 50, 10), (100, 180, 60, 5)]);
}

#[test]
fn multi_string_layout_pads_all_but_the_first_string() {
    assert_eq!(xy(&multi_string()), [(0, 0), (20, 0), (40, 0), (60, 0), (60, 10), (60, 20), (60, 30), (60, 40)]);
}

#[test]
fn icicle_layout() {
    assert_eq!(xy(&icicle()), [(0, 10), (0, 18), (0, 25), (10, 10), (10, 18), (10, 25), (20, 10), (20, 18), (20, 25), (30, 10), (30, 18), (30, 25)]);
}

#[test]
fn pixel_grid_layout_both_orientations() {
    let mut s = pixel_grid();
    assert_eq!(xy(&s), [(0, 40), (0, 20), (0, 0), (30, 40), (30, 20), (30, 0), (60, 40), (60, 20), (60, 0), (90, 40), (90, 20), (90, 0)]);
    s.set_property("String Orientation", properties::PropertyValue::Enum("Horizontal".to_string()), "").unwrap();
    assert_eq!(xy(&s), [(0, 40), (45, 40), (90, 40), (0, 27), (45, 27), (90, 27), (0, 13), (45, 13), (90, 13), (0, 0), (45, 0), (90, 0)]);
}

#[test]
fn single_and_flood_layouts() {
    let mut s = Shape::new_single(p(30, 40), None, 1.0);
    assert_eq!(xy(&s), [(30, 40)]);
    s.set_zoom_level(1.5);
    assert_eq!(xy(&s), [(45, 60)], "SetPixelZoom truncates");

    let f = Shape::new_flood(p(30, 40), None);
    assert_eq!(xy(&f), [(30, 40)]);
    assert_eq!(f.pixels[0].size, 3, "the flood's light is size 3 whatever its PixelSize");
    assert_eq!(f.pixel_size(), 40);
}

#[test]
fn custom_is_its_children() {
    let s = custom();
    let mut expected = xy(&line());
    expected.extend(xy(&arch()));
    assert_eq!(xy(&s), expected);
    assert_eq!((s.left(), s.top(), s.right(), s.bottom()), (10, 10, 110, 70));
}

#[test]
fn custom_moves_and_scales_its_children() {
    let mut s = custom();
    s.move_to(100, 100);
    assert_eq!((s.left(), s.top()), (100, 100));
    let c = s.strings().unwrap();
    assert_eq!((c[0].left(), c[0].top()), (100, 110));
    assert_eq!((c[1].left(), c[1].top()), (100, 100));
    s.resize(0.5);
    assert_eq!((s.left(), s.top(), s.right(), s.bottom()), (50, 50, 100, 80));
}

#[test]
fn custom_prop_bounds_zoom_and_rotation() {
    let mut s = custom_prop();
    let b = s.custom_prop_bounds().unwrap();
    // Bounds include each light's size.
    assert_eq!((b.x, b.y, b.right(), b.bottom()), (10.0, 20.0, 52.5, 62.25));
    assert_eq!((s.left(), s.top(), s.right(), s.bottom()), (10, 20, 52, 62));
    assert_eq!(s.pixels_vec()[1].location, (50.5, 20.0));

    s.set_zoom_level(2.0);
    assert_eq!(s.pixels_vec()[1].location, (101.0, 40.0));
    let b = s.custom_prop_bounds().unwrap();
    // Zoom moves the lights but the size stays, as the C# does.
    assert_eq!((b.x, b.y, b.right(), b.bottom()), (20.0, 40.0, 103.0, 122.5));
    s.set_zoom_level(1.0);

    s.set_rotation_angle(90);
    let b = s.custom_prop_bounds().unwrap();
    let c = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    let moved = s.pixels_vec()[0].location;
    let (ex, ey) = Matrix::rotate_at(90.0, c.0, c.1).apply(10.0, 20.0);
    assert!((moved.0 - ex).abs() < 1e-9 && (moved.1 - ey).abs() < 1e-9);
    // The model does not change, only what is shown.
    if let ShapeKind::CustomProp(d) = &s.kind {
        assert_eq!(d.prop_pixels[0].location, (10.0, 20.0));
    }
}

#[test]
fn moving_head_is_square_after_drawing() {
    let s = moving_head();
    // 40 by 40 from (10, 20), which is already square.
    assert_eq!((s.left(), s.top(), s.right(), s.bottom()), (10, 20, 50, 60));
    assert_eq!(s.pixel_count(), 0);
    let d = s.moving_head().unwrap();
    assert_eq!((d.beam_transparency, d.beam_width_multiplier, d.pan_stop_position, d.tilt_stop_position), (40, 8, 360, 180));
    assert_eq!(d.max_pan_travel_time.value, 4.6);
    assert!(d.show_legend);
}

#[test]
fn moving_head_drawn_backwards_is_put_right_and_squared() {
    let mut s = Shape::new_moving_head(p(100, 100), None, 1.0);
    s.select(true);
    s.select_default_select_point();
    // Up and left, wider than tall: the corners swap and the width comes down to the height.
    s.mouse_move(40, 70, -60, -30, NONE);
    s.end_add_new();
    assert_eq!((s.left(), s.top(), s.bottom()), (40, 70, 100));
    assert_eq!(s.right() - s.left(), s.bottom() - s.top());
}

// ----- move, resize, rotate, for every shape

#[test]
fn line_move_resize_rotate() {
    check_move(line());
    check_resize(line(), 1, 200, 90);
    check_rotate(line());
}

#[test]
fn poly_line_move_resize_rotate() {
    check_move(poly_line());
    check_resize(poly_line(), 1, 120, 10);
    check_rotate(poly_line());
}

#[test]
fn arch_move_resize_rotate() {
    check_move(arch());
    check_resize(arch(), BR, 210, 120);
    check_rotate(arch());
    // The top right handle moves both real corners.
    let mut s = arch();
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(TR)));
    s.mouse_move(200, 0, 0, 0, NONE);
    assert_eq!((s.points[TL].y, s.points[BR].x, s.points[TL].x, s.points[BR].y), (0, 200, 10, 60));
}

#[test]
fn cane_move_resize_rotate() {
    check_move(cane());
    check_resize(cane(), BR, 60, 150);
    check_rotate(cane());
    // Drawing it places the arch a quarter of the way down, on the left.
    let s = drawn(Shape::new_cane(p(0, 0), None, 1.0), 40, 100);
    assert_eq!((s.points[cane::ARCH].x, s.points[cane::ARCH].y), (0, 25));
}

#[test]
fn rectangle_move_resize_rotate() {
    check_move(rectangle());
    check_resize(rectangle(), BR, 150, 80);
    check_rotate(rectangle());
}

#[test]
fn rectangle_control_drag_keeps_corners_square() {
    let mut s = rectangle();
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(BR)));
    s.mouse_move(150, 80, 0, 0, Modifiers::CTRL);
    assert_eq!((s.points[TR].x, s.points[BL].y), (150, 80));
    // Without Control, the other corners stay put and it is any quadrilateral.
    let mut s = rectangle();
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(BR)));
    s.mouse_move(150, 80, 0, 0, NONE);
    assert_eq!((s.points[TR].x, s.points[BL].y), (100, 50));
}

#[test]
fn ellipse_move_resize_rotate() {
    check_move(ellipse());
    check_resize(ellipse(), BR, 150, 90);
    check_rotate(ellipse());
    let mut s = ellipse();
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(BR)));
    s.mouse_move(150, 90, 0, 0, Modifiers::CTRL);
    // Control makes a circle: height = width.
    assert_eq!(s.points[BR].y - s.points[TL].y, s.points[BR].x - s.points[TL].x);
}

#[test]
fn triangle_move_resize_rotate() {
    check_move(triangle());
    check_resize(triangle(), triangle::P2, 150, 100);
    check_rotate(triangle());
}

#[test]
fn star_move_resize_rotate() {
    check_move(star());
    check_resize(star(), BR, 160, 150);
    check_rotate(star());
}

#[test]
fn star_burst_move_resize_rotate() {
    check_move(star_burst());
    check_resize(star_burst(), BR, 160, 120);
    check_rotate(star_burst());
    // Control makes it square from the height.
    let mut s = star_burst();
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(BR)));
    s.mouse_move(300, 80, 0, 0, Modifiers::CTRL);
    assert_eq!((s.points[BR].x, s.points[BR].y), (80, 80));
}

#[test]
fn net_move_resize_rotate() {
    check_move(net());
    check_resize(net(), BR, 100, 90);
    check_rotate(net());
}

#[test]
fn mega_tree_move_resize_rotate() {
    check_move(mega_tree());
    check_resize(mega_tree(), BR, 160, 260);
    check_rotate(mega_tree());
}

#[test]
fn multi_string_move_resize_rotate() {
    check_move(multi_string());
    check_resize(multi_string(), 1, 80, 10);
    check_rotate(multi_string());
}

#[test]
fn icicle_move_resize_rotate() {
    check_move(icicle());
    check_resize(icicle(), 1, 60, 30);
    check_rotate(icicle());
}

#[test]
fn pixel_grid_move_resize_rotate() {
    check_move(pixel_grid());
    check_resize(pixel_grid(), BR, 150, 90);
    check_rotate(pixel_grid());
}

#[test]
fn single_moves_and_drags_its_point() {
    check_move(Shape::new_single(p(30, 40), None, 1.0));
    let mut s = Shape::new_single(p(30, 40), None, 1.0);
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(0)));
    s.mouse_move(70, 80, 0, 0, NONE);
    assert_eq!(xy(&s), [(70, 80)]);
    // It has no rotate handle to speak of: dragging the hidden one moves nothing.
    let mut s = Shape::new_single(p(30, 40), None, 1.0);
    s.select(true);
    assert!(!s.show_rotation);
    s.set_select_point(SelectArg::Point(PointRef::RotateHandle));
    s.mouse_move(70, 80, 0, 0, NONE);
    assert_eq!(xy(&s), [(30, 40)]);
}

#[test]
fn flood_moves() {
    let mut s = Shape::new_flood(p(30, 40), None);
    s.set_select_point(SelectArg::Null);
    s.mouse_move(0, 0, 5, 6, NONE);
    assert_eq!(xy(&s), [(35, 46)]);
    s.resize(2.0);
    assert_eq!(xy(&s), [(70, 92)]);
}

#[test]
fn custom_moves_and_resizes() {
    check_move(custom());
    let mut s = custom();
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(custom::BOTTOM_RIGHT_HANDLE)));
    // Width 100 + 100 is double the size, from the same top left.
    s.mouse_move(0, 0, 100, 0, NONE);
    assert_eq!((s.left(), s.top(), s.right()), (10, 10, 210));
}

#[test]
fn custom_prop_moves_scales_and_rotates() {
    let mut s = custom_prop();
    s.set_select_point(SelectArg::Null);
    s.mouse_move(0, 0, 40, 60, NONE);
    assert_eq!((s.left(), s.top()), (50, 80));

    // Bottom right handle, doubling the width, from a shape with its handles up.
    let mut s = custom_prop();
    s.select(true);
    let handle = PreviewPoint::new(52, 62);
    let r = s.point_in_select_point(handle).expect("the bottom right handle is there");
    s.set_select_point(SelectArg::Point(r));
    s.mouse_move(95, 62, 0, 0, NONE);
    let b = s.custom_prop_bounds().unwrap();
    assert!((b.x - 10.0).abs() < 1.0 && (b.y - 20.0).abs() < 1.0, "{b:?} keeps its top left");
    assert!(b.right() > 90.0 && (b.bottom() - 62.25).abs() < 1.0, "{b:?}");

    // The handle above the top middle rotates it.
    let mut s = custom_prop();
    s.select(true);
    let b = s.custom_prop_bounds().unwrap();
    let r = s.point_in_select_point(PreviewPoint::new((b.x + b.width / 2.0) as i32, b.y as i32 - 10)).expect("the rotate handle is there");
    s.set_select_point(SelectArg::Point(r));
    // Directly to the right of the center: GetAngle(center, mouse), dx < 0, dy = 0, 180 + 270 = 450, % 360 = 90.
    s.mouse_move(200, (b.y + b.height / 2.0) as i32, 0, 0, NONE);
    assert_eq!(s.rotation_angle, 90);
}

#[test]
fn moving_head_moves_and_resizes() {
    let mut s = moving_head();
    s.set_select_point(SelectArg::Null);
    s.mouse_move(0, 0, 40, 60, NONE);
    assert_eq!((s.left(), s.top(), s.right(), s.bottom()), (50, 80, 90, 120));
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(BR)));
    s.mouse_move(100, 120, 0, 0, NONE);
    assert_eq!((s.points[BR].x, s.points[BR].y), (100, 120));
    s.on_move_point();
    // The neighbours follow the corner and it stays square.
    assert_eq!(s.right() - s.left(), s.bottom() - s.top());
}

// ----- rotation details

#[test]
fn rotation_handle_angles_follow_the_mouse_round_the_clock() {
    for (dx, dy, angle) in [(0, -100, 0), (100, 0, 90), (0, 100, 180), (-100, 0, 270)] {
        let mut s = line();
        s.select(true);
        s.set_select_point(SelectArg::Point(PointRef::RotateHandle));
        let c = s.center();
        s.mouse_move(c.x + dx, c.y + dy, 0, 0, NONE);
        assert_eq!(s.rotation_angle, angle, "mouse at {dx},{dy} from the center");
    }
}

#[test]
fn control_snaps_rotation_to_detents() {
    let mut s = line();
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::RotateHandle));
    let c = s.center();
    // About 40 degrees past straight up, which snaps to 45.
    s.mouse_move(c.x + 84, c.y - 100, 0, 0, Modifiers::CTRL);
    assert_eq!(s.rotation_angle, 45);
    s.mouse_move(c.x + 84, c.y - 100, 0, 0, NONE);
    assert_eq!(s.rotation_angle, 40);
}

#[test]
fn rotating_after_a_resize_keeps_the_shape_where_it_appears() {
    let mut s = line();
    s.select(true);
    s.set_rotation_angle(90);
    let before = xy(&s);
    // Grow it while rotated, so the center moves and the rotation axis stays behind.
    s.set_select_point(SelectArg::Point(PointRef::Pt(0)));
    s.mouse_move(before[0].0, before[0].1 - 40, 0, 0, NONE);
    let axis = s.rotation_axis.unwrap();
    let c = s.center();
    assert_ne!((axis.x, axis.y), (c.x, c.y), "the axis has not followed the center");
    let shown_end = xy(&s)[4];
    // Taking the rotate handle: the transform moves the points so the shape stays where it is shown.
    s.set_select_point(SelectArg::Point(PointRef::RotateHandle));
    s.set_rotation_angle(90);
    let axis = s.rotation_axis.unwrap();
    let c = s.center();
    assert_eq!((axis.x, axis.y), (c.x, c.y), "and the axis is back on the center");
    let after = xy(&s)[4];
    assert!((after.0 - shown_end.0).abs() <= 2 && (after.1 - shown_end.1).abs() <= 2, "{after:?} vs {shown_end:?}");
}

#[test]
fn rotation_out_of_range_is_ignored() {
    let mut s = line();
    s.set_rotation_angle(361);
    assert_eq!(s.rotation_angle, 0);
    s.set_rotation_angle(-360);
    assert_eq!(s.rotation_angle, -360);
}

#[test]
fn inverted_shape_uses_the_180_base() {
    let mut s = Shape::new_line(p(110, 70), p(10, 20), 5, None, 1.0);
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::RotateHandle));
    let c = s.center();
    // Point 1, the top left handle, is below the center, so the mouse straight above reads as 180.
    s.mouse_move(c.x, c.y - 100, 0, 0, NONE);
    assert_eq!(s.rotation_angle, 180);
}

#[test]
fn rotate_handle_sits_above_the_top_and_reports_itself() {
    let mut s = line();
    s.select(true);
    s.sync_rotate_handle();
    let h = s.rotate_handle.unwrap();
    assert_eq!((h.x, h.y), (60, 10), "center x, top - 10");
    assert_eq!(s.point_in_select_point(p(60, 12)), Some(PointRef::RotateHandle));
    assert!(s.is_rotate_handle(PointRef::RotateHandle));
    assert_eq!(s.point_in_select_point(p(10, 20)), Some(PointRef::Pt(0)));
    assert_eq!(s.point_in_select_point(p(300, 300)), None);
    // The axis at the center is never a target.
    assert_eq!(s.point_in_select_point(p(60, 45)), None);
}

// ----- zoom

#[test]
fn zoom_applies_to_strings_through_their_owner() {
    let mut s = rectangle();
    let before = xy(&s);
    s.set_zoom_level(2.0);
    // The corners are zoomed and the padded sides then laid out between them, so the ends land exactly.
    let after = xy(&s);
    assert_eq!(after[0], (before[0].0 * 2, before[0].1 * 2));
    assert_eq!(after[4], (before[4].0 * 2, before[4].1 * 2));
    assert_eq!(s.strings().unwrap()[0].zoom(), 1.0, "the lines themselves stay at one");
    s.set_zoom_level(1.0);
    assert_eq!(xy(&s), before);
}

#[test]
fn zoomed_drag_divides_by_the_zoom() {
    let mut s = line();
    s.set_zoom_level(2.0);
    s.set_select_point(SelectArg::Null);
    s.mouse_move(0, 0, 40, 60, NONE);
    // The model moves by half the screen distance.
    assert_eq!((s.points[0].x, s.points[0].y), (30, 50));
    assert_eq!(xy(&s)[0], (60, 100));
}

#[test]
fn zoomed_handle_drag_unzooms_the_mouse() {
    let mut s = line();
    s.set_zoom_level(2.0);
    s.select(true);
    s.set_select_point(SelectArg::Point(PointRef::Pt(1)));
    s.mouse_move(400, 300, 0, 0, NONE);
    assert_eq!((s.points[1].x, s.points[1].y), (200, 150));
}

// ----- Left, Top and their quirks

#[test]
fn left_and_top_setters_move_the_shape() {
    for mut s in [line(), arch(), cane(), rectangle(), star(), net(), mega_tree(), icicle(), poly_line(), multi_string()] {
        s.set_left(300);
        s.set_top(400);
        assert_eq!((s.left(), s.top()), (300, 400), "{}", s.type_name());
    }
}

#[test]
fn single_setters_wait_for_the_next_layout() {
    let mut s = Shape::new_single(p(30, 40), None, 1.0);
    s.set_left(100);
    assert_eq!(s.left(), 100);
    assert_eq!(xy(&s), [(30, 40)], "no layout from the setter, as in the C#");
    s.layout();
    assert_eq!(xy(&s), [(100, 40)]);
}

#[test]
fn triangle_bounds_leave_out_a_point_each_way() {
    let mut s = triangle();
    // Left and Right only look at points 2 and 3, and Top at points 1 and 3.
    s.points[triangle::P1].x = -500;
    assert_eq!(s.left(), 0);
    s.points[triangle::P1].x = 50;
    s.points[triangle::P2].y = -500;
    assert_eq!(s.top(), 0);
    assert_eq!(s.bottom(), 80);
}

#[test]
fn nudge_is_a_move_to() {
    let mut s = line();
    s.nudge(3, -2);
    assert_eq!((s.left(), s.top()), (13, 18));
    assert_eq!(xy(&s)[0], (13, 18));
}

#[test]
fn resize_truncates_except_where_it_rounds() {
    let mut s = line();
    s.resize(0.55);
    // (int)(10 * 0.55) = 5, (int)(110 * 0.55) = 60: a cast truncates 60.5.
    assert_eq!((s.points[0].x, s.points[1].x), (5, 60));
    let mut s = poly_line();
    s.resize(0.55);
    // Convert.ToInt32(100 * 0.55) rounds 55.00000000000001, and (50 * 0.55 = 27.5) goes to 28.
    assert_eq!((s.points[1].x, s.points[2].y), (55, 28));
}

#[test]
fn resize_from_original_starts_from_where_the_drag_began() {
    let mut s = line();
    s.set_select_point(SelectArg::Null);
    s.move_to(500, 500);
    s.resize_from_original(2.0);
    assert_eq!((s.points[0].x, s.points[0].y), (20, 40));
}

#[test]
fn match_copies_size_and_proportions() {
    let mut a = line();
    let mut b = Shape::new_line(p(0, 0), p(10, 10), 5, None, 1.0);
    a.set_pixel_size(7);
    a.select(true);
    a.set_rotation_angle(30);
    b.match_shape(&a);
    assert_eq!((b.points[1].x, b.points[1].y), (100, 50));
    assert_eq!(b.pixel_size(), 7);
    assert_eq!(b.rotation_angle, 30);
    assert!(b.pixels.iter().all(|q| q.size == 7));
    // A different type does nothing.
    let before = xy(&b);
    b.match_shape(&arch());
    assert_eq!(xy(&b), before);
}

// ----- hit testing

#[test]
fn point_in_shape_uses_each_shapes_rectangle() {
    let s = line();
    // A light at (10, 20), size 3: from (7, 17) to (7 + 9, 17 + 9) inclusive.
    assert!(s.point_in_shape(p(7, 17)));
    assert!(s.point_in_shape(p(16, 26)));
    assert!(!s.point_in_shape(p(6, 17)));
    let s = rectangle();
    // These use a rectangle of just 6, whatever the light size: from (-3, 47) to (3, 53).
    assert!(s.point_in_shape(p(3, 53)));
    assert!(!s.point_in_shape(p(4, 53)));
}

#[test]
fn single_hit_depends_on_size() {
    let mut s = Shape::new_single(p(100, 100), None, 1.0);
    assert!(s.point_in_shape(p(97, 97)));
    assert!(s.point_in_shape(p(103, 103)));
    assert!(!s.point_in_shape(p(104, 103)));
    s.set_pixel_size(10);
    assert!(s.point_in_shape(p(105, 95)));
    assert!(!s.point_in_shape(p(106, 95)));
}

#[test]
fn custom_prop_hit_is_the_rotated_bounds() {
    let mut s = custom_prop();
    assert!(s.point_in_shape(p(30, 40)));
    assert!(!s.point_in_shape(p(100, 40)));
    s.set_rotation_angle(90);
    let b = s.custom_prop_bounds().unwrap();
    let c = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    // The far end of the 42.5 wide shape is now about 21 above the center instead of to its right.
    assert!(s.point_in_shape(p(c.0 as i32, c.1 as i32 + 18)));
    assert!(!s.point_in_shape(p(c.0 as i32 + 30, c.1 as i32)));
}

#[test]
fn group_hit_is_its_bounds_rectangle() {
    let s = custom();
    assert!(s.point_in_shape(p(50, 15)), "empty space in the group's rectangle counts");
    assert!(s.point_in_shape(p(10, 10)));
    assert!(!s.point_in_shape(p(110, 70)), "Rectangle.Contains leaves out the right and bottom");
}

#[test]
fn moving_head_hit_and_rect() {
    let s = moving_head();
    assert!(s.point_in_shape(p(10, 20)));
    assert!(s.point_in_shape(p(50, 60)));
    assert!(!s.point_in_shape(p(51, 60)));
    assert!(s.shape_in_rect(Rect::new(0, 0, 100, 100), true));
    assert!(!s.shape_in_rect(Rect::new(0, 0, 30, 100), true));
    assert!(s.shape_in_rect(Rect::new(0, 0, 30, 100), false));
    assert!(!s.shape_in_rect(Rect::new(60, 0, 30, 100), false));
}

#[test]
fn shape_in_rect_any_and_all() {
    let s = line();
    assert!(s.shape_in_rect(Rect::new(0, 0, 200, 200), true));
    assert!(s.shape_in_rect(Rect::new(0, 0, 40, 40), false));
    assert!(!s.shape_in_rect(Rect::new(0, 0, 40, 40), true));
    assert!(!s.shape_in_rect(Rect::new(300, 300, 40, 40), false));
    // A rectangle drawn backwards normalises.
    assert!(s.shape_in_rect(Rect::new(200, 200, -200, -200), true));
}

// ----- element nodes, as new shapes pick them up

fn tree_with_strings(strings: usize, lights: usize) -> (ElementTree, NodeId) {
    let mut t = ElementTree::new();
    let root = t.add_node(Uuid::new_v4(), "Root", None);
    for i in 0..strings {
        let s = t.add_string(&format!("S{i}"), lights);
        t.add_child(root, s);
    }
    (t, root)
}

#[test]
fn line_from_a_leaf_is_standard_and_from_a_group_is_pixel() {
    let mut t = ElementTree::new();
    let leaf = t.add_element("Leaf");
    let group = t.add_string("Group", 12);
    let s = Shape::new_line(p(0, 0), p(10, 0), 5, Some(Sel::new(&t, leaf)), 1.0);
    assert_eq!(s.string_type(), StringType::Standard);
    assert_eq!(s.pixels.len(), 5);
    assert!(s.pixels.iter().all(|q| q.node_id == t.node(leaf).id));
    let s = Shape::new_line(p(0, 0), p(10, 0), 5, Some(Sel::new(&t, group)), 1.0);
    assert_eq!(s.string_type(), StringType::Pixel);
    assert_eq!(s.pixels.len(), 12, "a pixel per child, whatever the count asked for");
    assert_eq!(s.pixels[3].node_id, t.node(t.node(group).children[3]).id);
}

#[test]
fn arch_and_ellipse_need_four_children_to_be_pixel_shapes() {
    let mut t = ElementTree::new();
    let small = t.add_string("Small", 3);
    let big = t.add_string("Big", 7);
    let s = Shape::new_arch(p(0, 0), Some(Sel::new(&t, small)), 1.0);
    assert_eq!((s.string_type(), s.pixels.len()), (StringType::Standard, 25));
    let s = Shape::new_arch(p(0, 0), Some(Sel::new(&t, big)), 1.0);
    assert_eq!((s.string_type(), s.pixels.len()), (StringType::Pixel, 7));
    let s = Shape::new_ellipse(p(0, 0), Some(Sel::new(&t, small)), 1.0);
    assert_eq!((s.string_type(), s.pixels.len()), (StringType::Standard, 50));
    let s = Shape::new_ellipse(p(0, 0), Some(Sel::new(&t, big)), 1.0);
    assert_eq!((s.string_type(), s.pixels.len()), (StringType::Pixel, 7));
}

#[test]
fn cane_splits_its_children() {
    let mut t = ElementTree::new();
    let g = t.add_string("Cane", 21);
    let s = Shape::new_cane(p(0, 0), Some(Sel::new(&t, g)), 1.0);
    assert!(matches!(s.kind, ShapeKind::Cane { arch_pixel_count: 10, line_pixel_count: 11, .. }));
    assert_eq!(s.pixels.len(), 21);
    let s = Shape::new_cane(p(0, 0), None, 1.0);
    assert!(matches!(s.kind, ShapeKind::Cane { arch_pixel_count: 8, line_pixel_count: 8, .. }));
    assert_eq!(s.pixels.len(), 16);
}

#[test]
fn rectangle_takes_four_groups_or_divides_its_children() {
    let (t, root) = tree_with_strings(4, 6);
    let s = Shape::new_rectangle(p(0, 0), Some(Sel::new(&t, root)), 1.0);
    let counts: Vec<usize> = s.strings().unwrap().iter().map(|l| l.pixels.len()).collect();
    assert_eq!(counts, [6, 6, 6, 6]);
    assert_eq!(s.strings().unwrap()[2].pixels[1].node_id, t.node(t.node(t.node(root).children[2]).children[1]).id);

    let mut t = ElementTree::new();
    let g = t.add_string("Rect", 18);
    let s = Shape::new_rectangle(p(0, 0), Some(Sel::new(&t, g)), 1.0);
    assert_eq!(s.string_type(), StringType::Pixel);
    let counts: Vec<usize> = s.strings().unwrap().iter().map(|l| l.pixels.len()).collect();
    assert_eq!(counts, [4, 4, 4, 4], "18 / 4 = 4 per side, and the last two children go unused");
    assert_eq!(s.strings().unwrap()[1].pixels[0].node_id, t.node(t.node(g).children[4]).id);
}

#[test]
fn triangle_divides_its_children() {
    let mut t = ElementTree::new();
    let g = t.add_string("Tri", 7);
    let s = Shape::new_triangle(p(0, 0), Some(Sel::new(&t, g)), 1.0);
    let counts: Vec<usize> = s.strings().unwrap().iter().map(|l| l.pixels.len()).collect();
    assert_eq!(counts, [2, 2, 2]);
}

#[test]
fn star_from_groups_counts_its_points() {
    let (t, root) = tree_with_strings(6, 4);
    let s = Shape::new_star(p(0, 0), Some(Sel::new(&t, root)), 1.0);
    assert_eq!(s.string_type(), StringType::Pixel);
    assert!(matches!(s.kind, ShapeKind::Star { point_count: 6, pixel_count: 24, inside_size: 40, .. }));
    let mut t = ElementTree::new();
    let g = t.add_string("Star", 30);
    let s = Shape::new_star(p(0, 0), Some(Sel::new(&t, g)), 1.0);
    assert!(matches!(s.kind, ShapeKind::Star { point_count: 5, .. }), "30 is divisible by 5");
    let s = Shape::new_star(p(0, 0), None, 1.0);
    assert!(matches!(s.kind, ShapeKind::Star { point_count: 5, pixel_count: 40, .. }));
}

#[test]
fn star_burst_takes_a_spoke_per_group() {
    let (t, root) = tree_with_strings(5, 9);
    let s = Shape::new_star_burst(p(0, 0), Some(Sel::new(&t, root)), 1.0);
    assert_eq!(s.string_type(), StringType::Pixel);
    assert_eq!(s.strings().unwrap().len(), 5);
    assert_eq!(s.pixel_count(), 45);
    let s = Shape::new_star_burst(p(0, 0), None, 1.0);
    assert_eq!(s.strings().unwrap().len(), 8);
    assert_eq!(s.pixel_count(), 80);
}

#[test]
fn mega_tree_recognises_pixel_and_standard_trees() {
    let (t, root) = tree_with_strings(6, 10);
    let s = Shape::new_mega_tree(p(0, 0), Some(Sel::new(&t, root)), 1.0);
    assert_eq!(s.string_type(), StringType::Pixel);
    assert!(matches!(s.kind, ShapeKind::MegaTree { string_count: 6, lights_per_string: 10, .. }));

    let mut t = ElementTree::new();
    let g = t.add_string("Tree", 8);
    let s = Shape::new_mega_tree(p(0, 0), Some(Sel::new(&t, g)), 1.0);
    assert_eq!(s.string_type(), StringType::Standard);
    assert!(matches!(s.kind, ShapeKind::MegaTree { string_count: 8, lights_per_string: 50, .. }));
    assert_eq!(s.strings().unwrap()[3].pixels[10].node_id, t.node(t.node(g).children[3]).id);

    let s = Shape::new_mega_tree(p(0, 0), None, 1.0);
    assert!(matches!(s.kind, ShapeKind::MegaTree { string_count: 16, lights_per_string: 50, .. }));
}

#[test]
fn pixel_grid_takes_its_orientation_from_the_node() {
    let (mut t, root) = tree_with_strings(3, 4);
    t.node_mut(root).properties.push(PropertyInstance {
        type_id: vixen_core::element::property_ids::ORIENTATION,
        instance_id: Uuid::new_v4(),
        data: PropertyData::Orientation(Orientation::Horizontal),
    });
    let s = Shape::new_pixel_grid(p(0, 0), Some(Sel::new(&t, root)), 1.0);
    assert_eq!(s.string_type(), StringType::Pixel);
    assert!(matches!(s.kind, ShapeKind::PixelGrid { string_count: 3, lights_per_string: 4, string_orientation: StringOrientation::Horizontal }));
}

use vixen_core::element::{PropertyData, PropertyInstance};

#[test]
fn icicle_of_groups_is_pixel_and_not_creating() {
    let (t, root) = tree_with_strings(5, 7);
    let s = Shape::new_icicle(p(0, 0), p(0, 0), Some(Sel::new(&t, root)), 1.0);
    assert_eq!(s.string_type(), StringType::Pixel);
    assert!(matches!(s.kind, ShapeKind::Icicle { string_count: 5, creating: false, .. }));
    assert_eq!(s.pixel_count(), 35);
}

#[test]
fn standard_icicle_links_all_strings_to_the_first_light() {
    let mut t = ElementTree::new();
    let leaf = t.add_element("Leaf");
    let mut s = Shape::new_icicle(p(0, 0), p(0, 0), Some(Sel::new(&t, leaf)), 1.0);
    assert_eq!(s.string_type(), StringType::Standard);
    s.select(true);
    s.select_default_select_point();
    s.mouse_move(40, 0, 40, 0, NONE);
    assert_eq!(s.strings().unwrap().len(), 5);
    assert!(s.pixels_vec().iter().all(|q| q.node_id == t.node(leaf).id));
}

#[test]
fn multi_string_takes_a_group_per_segment() {
    let (t, root) = tree_with_strings(3, 6);
    let mut s = Shape::new_multi_string(p(0, 0), Some(Sel::new(&t, root)), 1.0);
    s.select(true);
    s.mouse_up(50, 0, true, Some(&t));
    s.mouse_up(100, 0, true, Some(&t));
    let strings = s.strings().unwrap();
    assert_eq!(strings.len(), 2);
    assert_eq!(strings[1].pixels.len(), 6);
    assert_eq!(strings[1].pixels[2].node_id, t.node(t.node(t.node(root).children[1]).children[2]).id);
}

// ----- the XML form

#[test]
fn every_shape_type_writes_and_reads_itself() {
    for s in [
        line(),
        poly_line(),
        arch(),
        cane(),
        rectangle(),
        ellipse(),
        triangle(),
        star(),
        star_burst(),
        net(),
        mega_tree(),
        multi_string(),
        icicle(),
        pixel_grid(),
        Shape::new_single(p(30, 40), None, 1.0),
        Shape::new_flood(p(30, 40), None),
        custom(),
        custom_prop(),
        moving_head(),
    ] {
        check_xml(&s);
    }
}

#[test]
fn members_are_in_serializer_order() {
    let check = |s: Shape, members: &[&str]| assert_eq!(member_names(&s), members, "{}", s.type_name());
    let light = ["PixelSize", "Pixels", "StringType"];
    let with = |extra: &[&'static str]| -> Vec<&'static str> { light.iter().copied().chain(extra.iter().copied()).collect() };
    check(line(), &with(&["_points"]));
    check(poly_line(), &with(&["_points"]));
    check(arch(), &with(&["_bottomRight", "_topLeft"]));
    check(cane(), &with(&["_archLeftPoint", "_archPixelCount", "_bottomRightPoint", "_linePixelCount", "_topLeftPoint"]));
    check(rectangle(), &with(&["_strings", "Direction", "_bottomLeft", "_bottomRight", "_topLeft", "_topRight"]));
    check(ellipse(), &with(&["_bottomRight", "_topLeft"]));
    check(triangle(), &with(&["_strings", "_point1", "_point2", "_point3"]));
    check(star(), &with(&["_XYRotation", "_bottomRightPoint", "_insideSize", "_pixelCount", "_pointCount", "_stringDirection", "_topLeftPoint"]));
    check(star_burst(), &with(&["_strings", "Direction", "_XYRotation", "_bottomLeft", "_bottomRight", "_innerCircleSize", "_topLeft", "_topRight"]));
    check(net(), &with(&["_bottomLeft", "_bottomRight", "_pixelSpacing", "_topLeft", "_topRight"]));
    check(
        mega_tree(),
        &with(&["_strings", "_baseHeight", "_bottomLeft", "_bottomRight", "_degreeOffset", "_degrees", "_lightsPerString", "_stringCount", "_topHeight", "_topLeft", "_topRight", "_topWidth"]),
    );
    check(multi_string(), &with(&["_strings", "_points", "_stringCount"]));
    check(icicle(), &with(&["_strings", "_points", "_stringCount", "_verticalSpacing"]));
    check(pixel_grid(), &with(&["_strings", "_bottomLeft", "_bottomRight", "_lightsPerString", "_stringCount", "_stringOrientation", "_topLeft", "_topRight"]));
    check(Shape::new_single(p(1, 2), None, 1.0), &with(&["p1"]));
    check(Shape::new_flood(p(1, 2), None), &with(&["_p1"]));
    check(custom(), &with(&["_strings", "_topLeft"]));
    check(custom_prop(), &with(&["PropPixels"]));
    let mut mh = moving_head();
    mh.locked = true;
    mh.name = Some("Spot".to_string());
    mh.use_common_location = true;
    if let ShapeKind::MovingHead(d) = &mut mh.kind {
        d.node_id = Uuid::from_u128(5);
        d.zoom_narrow_to_wide = true;
        d.invert_pan_direction = YesNo::Yes;
        d.invert_tilt_direction = YesNo::Yes;
        d.mounting_position = MountingPosition::Top;
    }
    check(
        mh,
        &[
            "Locked",
            "Name",
            "UseCommonLocation",
            "BeamLength",
            "BeamTransparency",
            "BeamWidthMultiplier",
            "InvertPanDirection",
            "InvertTiltDirection",
            "MaxColorWheelRotationSpeed",
            "MaxPanTravelTime",
            "MaxTiltTravelTime",
            "MaximumStrobeDuration",
            "MinColorWheelRotationSpeed",
            "MountingPosition",
            "NodeId",
            "PanStopPosition",
            "ShowLegend",
            "StrobeRateMaximum",
            "StrobeRateMinimum",
            "TiltStopPosition",
            "ZoomNarrowToWide",
            "_bottomLeft",
            "_bottomRight",
            "_topLeft",
            "_topRight",
        ],
    );
}

#[test]
fn base_members_come_first_and_defaults_are_left_out() {
    let mut s = line();
    assert_eq!(member_names(&s)[0], "PixelSize");
    s.locked = true;
    s.name = Some("My Line".to_string());
    s.rotation_angle = 15;
    s.use_common_location = true;
    assert_eq!(member_names(&s)[..4], ["Locked", "Name", "RotationAngle", "UseCommonLocation"]);
}

#[test]
fn a_loaded_shape_keeps_rotation_and_lights_rotate_on_load() {
    let mut s = line();
    s.rotation_angle = 90;
    let back = shape_from_xml(&s.to_xml_named("Shape"));
    assert_eq!(back.rotation_angle, 90);
    assert_eq!(xy(&back), [(85, -5), (73, 20), (60, 45), (47, 70), (35, 95)]);
}

#[test]
fn a_mega_tree_never_selected_writes_nil_corners() {
    let s = Shape::new_mega_tree(p(0, 0), None, 1.0);
    let e = s.to_xml_named("Shape");
    assert!(e.child("_topRight").unwrap().nil);
    assert!(e.child("_bottomLeft").unwrap().nil);
    let back = shape_from_xml(&e);
    assert_eq!(e.first_difference(&back.to_xml_named("Shape"), String::new()), None);
    // Selecting it builds them, and then it saves them.
    let mut s = s;
    s.select(true);
    assert!(!s.to_xml_named("Shape").child("_topRight").unwrap().nil);
}

#[test]
fn a_custom_pixel_size_is_kept_as_read() {
    // A group ignores its own PixelSize. Files have 0, and a new group 3.
    let mut e = custom().to_xml_named("Shape");
    e.child_mut("PixelSize").unwrap().children = vec![vixen_xml::XNode::Text("0".to_string())];
    let back = shape_from_xml(&e);
    assert_eq!(back.pixel_size(), 0);
    assert_eq!(e.first_difference(&back.to_xml_named("Shape"), String::new()), None);
}

#[test]
fn string_pixel_size_follows_the_owner_on_load() {
    let mut s = rectangle();
    s.pixel_size = 8;
    let back = shape_from_xml(&s.to_xml_named("Shape"));
    assert!(back.pixels_vec().iter().all(|q| q.size == 8));
    assert!(back.strings().unwrap().iter().all(|l| l.pixel_size() == 8));
}

#[test]
fn a_moving_head_from_before_its_new_members_gets_defaults() {
    let mut s = moving_head();
    if let ShapeKind::MovingHead(d) = &mut s.kind {
        d.strobe_rate_minimum = 0;
        d.max_pan_travel_time = crate::xml_util::DoubleText::new(0.0);
    }
    let back = shape_from_xml(&s.to_xml_named("Shape"));
    let d = back.moving_head().unwrap();
    assert_eq!(d.strobe_rate_minimum, 1);
    assert_eq!(d.max_pan_travel_time.value, 4.6);
}

// ----- property grid

fn row(s: &Shape, name: &str) -> properties::ShapeProperty {
    s.properties().into_iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{} has no {name}", s.type_name()))
}

fn names(s: &Shape) -> Vec<&'static str> {
    s.properties().iter().map(|r| r.name).collect()
}

#[test]
fn every_shape_has_its_csharp_rows_in_grid_order() {
    // The categories in the order a categorized PropertyGrid shows them, and each shape's rows.
    let cats = |s: &Shape| -> Vec<&'static str> {
        let mut c: Vec<&'static str> = s.properties().iter().map(|r| r.category).collect();
        c.dedup();
        c
    };
    assert_eq!(cats(&line()), ["Control", "Location", "Position", "Settings"]);
    assert_eq!(cats(&arch()), ["Control", "Location", "Position", "Settings", "Size"]);
    assert_eq!(cats(&rectangle()), ["Control", "Location", "Misc", "Position", "Settings"]);
    assert_eq!(cats(&triangle()), ["Control", "Location", "Misc", "Position", "Settings"]);
    assert_eq!(cats(&moving_head()), ["Beam", "Color Wheel", "Control", "Location", "Pan", "Position", "Settings", "Strobe", "Tilt"]);

    let has = |s: &Shape, wanted: &[&str]| {
        let n = names(s);
        for w in wanted {
            assert!(n.contains(w), "{} lacks {w}: {n:?}", s.type_name());
        }
    };
    let common = ["Name", "Common Location", "Locked", "Rotation Angle"];
    let light = ["Light Size", "Linked Elements"];
    for s in [line(), poly_line(), arch(), cane(), rectangle(), ellipse(), triangle(), star(), star_burst(), net(), mega_tree(), multi_string(), icicle(), pixel_grid()] {
        has(&s, &common);
        has(&s, &light);
    }
    has(&line(), &["Point 1", "Point 2", "Light Count", "String Type"]);
    has(&poly_line(), &["Light Count"]);
    has(&arch(), &["Top Left", "Bottom Right", "Width", "Height", "Light Count"]);
    has(&cane(), &["Line Light Count", "Arch Light Count", "Bottom Right", "Top Left", "Arch Start"]);
    has(&rectangle(), &["Top Left", "Top Right", "Bottom Right", "Botom Left", "String 1 Light Count", "String 4 Light Count", "Direction", "PixelCount"]);
    has(&ellipse(), &["Top Left", "Bottom Right", "Light Count"]);
    has(&triangle(), &["Point 1", "Point 2", "Point 3", "String 1 Light Count", "String 3 Light Count", "Width", "Height"]);
    has(&star(), &["String Direction", "Light Count", "XY Rotation", "Inside Size", "Star Points", "Top Left", "Bottom Right"]);
    has(&star_burst(), &["Top Left", "Bottom Right", "XY Rotation", "Light Count per Spoke", "Direction", "Inner Circle Size", "StringCount", "PixelCount"]);
    has(&net(), &["Top Left", "Top Right", "Bottom Right", "Botom Left", "Light Spacing"]);
    has(&mega_tree(), &["Top Height", "Top Width", "Base Height", "String Coverage Degrees", "Degree Offset", "Lights Per String", "String Count", "Top Left", "Bottom Right"]);
    has(&icicle(), &["Vertical Spacing", "Icicle Count", "Point 1", "Point 2"]);
    has(&pixel_grid(), &["Lights Per String", "String Count", "String Orientation", "Top Left", "Bottom Right"]);
    has(&Shape::new_single(p(1, 1), None, 1.0), &["Position", "Light Size", "Linked Elements", "String Type"]);
    has(&Shape::new_flood(p(1, 1), None), &["Point 1", "Light Size"]);
    has(&custom(), &["Position", "String Type"]);
    has(&custom_prop(), &["X", "Y", "Top Left", "Bottom Right", "String Type", "Linked Elements"]);
    has(
        &moving_head(),
        &[
            "Top Left",
            "Botom Left",
            "Linked Element",
            "Color Wheel Rotation Speed Minimum (s)",
            "Color Wheel Rotation Speed Maximum (s)",
            "Maximum Pan Travel Time (s)",
            "Maximum Tilt Travel Time (s)",
            "Minimum Strobe Rate (ms)",
            "Maximum Strobe Rate (Hz)",
            "Maximum Strobe Duration (ms)",
            "Pan Start Position (Degrees)",
            "Pan Stop Position (Degrees)",
            "Tilt Start Position (Degrees)",
            "Tilt Stop Position (Degrees)",
            "Beam Length",
            "Beam Transparency (%)",
            "Beam Width Multiplier",
            "Zoom Narrow To Wide",
            "Show Legend",
            "Invert Pan Direction",
            "Invert Tilt Direction",
            "Mounting Position",
        ],
    );
}

#[test]
fn browsable_false_rows_are_absent() {
    let lacks = |s: &Shape, unwanted: &[&str]| {
        let n = names(s);
        for u in unwanted {
            assert!(!n.contains(u), "{} should not show {u}", s.type_name());
        }
    };
    lacks(&net(), &["String Type", "PixelCount", "Light Count"]);
    lacks(&Shape::new_flood(p(1, 1), None), &["String Type"]);
    lacks(&multi_string(), &["String Type", "Light Count", "String Count", "Point 1"]);
    lacks(&custom(), &["Light Size", "Linked Elements"]);
    lacks(&custom_prop(), &["Light Size"]);
    lacks(&mega_tree(), &["PixelCount", "Light Count"]);
    lacks(&pixel_grid(), &["PixelCount", "Botom Left"]);
    lacks(&moving_head(), &["Light Size", "String Type", "Linked Elements"]);
    for s in [line(), arch(), moving_head()] {
        lacks(&s, &["Top", "Left", "Right", "Bottom", "ZoomLevel", "Selected", "Parent", "Creating", "Center", "Pixels", "PixelColor", "MaxAlpha"]);
    }
}

#[test]
fn read_only_rows_match_the_csharp() {
    assert!(row(&custom_prop(), "String Type").read_only, "[ReadOnly(true)]");
    assert!(row(&rectangle(), "PixelCount").read_only, "no setter");
    assert!(row(&triangle(), "Width").read_only, "no setter");
    assert!(row(&star_burst(), "PixelCount").read_only, "no setter");
    assert!(!row(&star_burst(), "StringCount").read_only);
    assert!(!row(&line(), "Light Count").read_only);
    let mut s = rectangle();
    assert_eq!(s.set_property("PixelCount", properties::PropertyValue::Int(3), ""), Err(properties::PropertyError::ReadOnly));
}

#[test]
fn rotation_angle_is_last_in_position() {
    let positions: Vec<&str> = line().properties().iter().filter(|r| r.category == "Position").map(|r| r.name).collect();
    assert_eq!(positions, ["Point 1", "Point 2", "Rotation Angle"]);
    let cp: Vec<&str> = custom_prop().properties().iter().filter(|r| r.category == "Position").map(|r| r.name).collect();
    assert_eq!(cp, ["X", "Y", "Top Left", "Bottom Right", "Rotation Angle"], "the [PropertyOrder]s");
}

#[test]
fn property_edits_do_what_the_setters_do() {
    use properties::PropertyValue as V;
    let mut s = line();
    s.set_property("Light Count", V::Int(3), "").unwrap();
    assert_eq!(xy(&s), [(10, 20), (60, 45), (110, 70)]);
    assert_eq!(row(&s, "Light Count").value, V::Int(3));
    s.set_property("Point 2", V::Point { x: 210, y: 20 }, "").unwrap();
    assert_eq!(xy(&s), [(10, 20), (110, 20), (210, 20)]);
    assert_eq!(row(&s, "Point 2").value, V::Point { x: 210, y: 20 });
    s.set_property("Light Size", V::Int(9), "").unwrap();
    assert!(s.pixels.iter().all(|q| q.size == 9));
    s.set_property("Name", V::Text("My Line".to_string()), "").unwrap();
    assert_eq!(s.name(), Some("My Line"));
    s.set_property("Name", V::Text(String::new()), "Line").unwrap();
    assert_eq!(s.name(), Some("Line"), "empty takes VixenPreviewSetup3.DrawShape");
    s.set_property("Locked", V::Bool(true), "").unwrap();
    assert!(s.locked());
    s.set_property("Rotation Angle", V::Int(400), "").unwrap();
    assert_eq!(s.rotation_angle, 0, "out of range is ignored");
    assert_eq!(s.set_property("Linked Elements", V::Int(1), ""), Err(properties::PropertyError::UseLinker));
    assert_eq!(s.set_property("No Such Thing", V::Int(1), ""), Err(properties::PropertyError::UnknownProperty));
    assert_eq!(s.set_property("Light Count", V::Text("x".to_string()), ""), Err(properties::PropertyError::WrongType));
    assert_eq!(s.set_property("String Type", V::Enum("Plaid".to_string()), ""), Err(properties::PropertyError::WrongType));
    // By C# name too.
    s.set_property("PixelCount", V::Int(2), "").unwrap();
    assert_eq!(s.pixels.len(), 2);

    let mut s = arch();
    s.set_property("Width", V::Int(50), "").unwrap();
    assert_eq!(s.right(), 60);
    s.set_property("Height", V::Int(20), "").unwrap();
    assert_eq!(s.top(), 40, "Height moves the top, keeping the bottom");

    let mut s = star();
    s.set_property("Star Points", V::Int(4), "").unwrap();
    assert_eq!(s.pixel_count(), 16, "20 / 4 = 5, down to 4 per point");
    s.set_property("Inside Size", V::Int(50), "").unwrap();
    s.set_property("XY Rotation", V::Int(0), "").unwrap();
    assert!(matches!(s.kind, ShapeKind::Star { inside_size: 50, xy_rotation: 0, .. }));

    let mut s = cane();
    s.set_property("Line Light Count", V::Int(10), "").unwrap();
    assert_eq!(s.pixels.len(), 18);
    s.set_property("Arch Light Count", V::Int(4), "").unwrap();
    assert_eq!(s.pixels.len(), 14);

    let mut s = mega_tree();
    s.set_property("String Count", V::Int(6), "").unwrap();
    assert_eq!(s.strings().unwrap().len(), 6);
    s.set_property("Lights Per String", V::Int(7), "").unwrap();
    assert_eq!(s.pixel_count(), 42);

    let mut s = pixel_grid();
    s.set_property("String Count", V::Int(2), "").unwrap();
    assert_eq!(s.pixel_count(), 6);

    let mut s = icicle();
    s.set_property("Icicle Count", V::Int(2), "").unwrap();
    assert_eq!(s.strings().unwrap().len(), 2);
    s.set_property("Vertical Spacing", V::Int(1), "").unwrap();
    assert_eq!(row(&s, "Vertical Spacing").value, V::Int(2), "the getter's minimum");

    let mut s = star_burst();
    s.set_property("StringCount", V::Int(6), "").unwrap();
    assert_eq!(s.strings().unwrap().len(), 6);
    s.set_property("Light Count per Spoke", V::Int(2), "").unwrap();
    assert_eq!(s.pixel_count(), 12);

    let mut s = net();
    s.set_property("Light Spacing", V::Int(4), "").unwrap();
    assert!(s.pixel_count() > 6);

    let mut s = custom();
    s.set_property("Position", V::Point { x: 200, y: 300 }, "").unwrap();
    assert_eq!((s.left(), s.top()), (200, 300));

    let mut s = custom_prop();
    s.set_property("X", V::Int(100), "").unwrap();
    s.set_property("Y", V::Int(200), "").unwrap();
    assert_eq!((s.left(), s.top()), (100, 200));
    assert_eq!(s.set_property("String Type", V::Enum("Pixel".to_string()), ""), Err(properties::PropertyError::ReadOnly));

    let mut s = moving_head();
    s.set_property("Beam Length", V::Int(50), "").unwrap();
    assert_eq!(s.moving_head().unwrap().beam_length, 50);
    s.set_property("Maximum Pan Travel Time (s)", V::Double(3.5), "").unwrap();
    assert_eq!(s.moving_head().unwrap().max_pan_travel_time.value, 3.5);
    s.set_property("Mounting Position", V::Enum("Top".to_string()), "").unwrap();
    assert_eq!(s.moving_head().unwrap().mounting_position, MountingPosition::Top);
    let warning = s.set_property("Pan Start Position (Degrees)", V::Int(400), "").unwrap();
    assert!(warning.unwrap().contains("Pan Start Position"), "Validate's message");
    s.set_property("Top Right", V::Point { x: 300, y: 21 }, "").unwrap();
    assert_eq!(s.points[TR].x, 300);
    s.set_property("Linked Element", V::LinkedElement(Some(Uuid::from_u128(3))), "").unwrap();
    assert_eq!(s.moving_head().unwrap().node_id, Uuid::from_u128(3));
}

// ----- nothing here may panic, whatever a file or a hand on the mouse does

fn every_shape() -> Vec<Shape> {
    vec![
        line(),
        poly_line(),
        arch(),
        cane(),
        rectangle(),
        ellipse(),
        triangle(),
        star(),
        star_burst(),
        net(),
        mega_tree(),
        multi_string(),
        icicle(),
        pixel_grid(),
        Shape::new_single(p(30, 40), None, 1.0),
        Shape::new_flood(p(30, 40), None),
        custom(),
        custom_prop(),
        moving_head(),
    ]
}

/// Everything a shape can be asked to do.
fn exercise(mut s: Shape) {
    use properties::PropertyValue as V;
    let other = s.clone();
    s.layout();
    let _ = (s.left(), s.top(), s.right(), s.bottom(), s.center());
    s.select(true);
    s.select_default_select_point();
    s.sync_rotate_handle();
    s.prepare_overlay();
    for m in [Modifiers::NONE, Modifiers::CTRL, Modifiers::SHIFT] {
        s.mouse_move(i32::MAX, i32::MIN, i32::MAX, i32::MIN, m);
        s.mouse_move(-5, 7, 3, -3, m);
    }
    s.mouse_up(0, 0, true, None);
    for point in [p(0, 0), p(-1, 10), p(i32::MAX, i32::MIN)] {
        let _ = s.point_in_shape(point);
        if let Some(r) = s.point_in_select_point(point) {
            s.set_select_point(SelectArg::Point(r));
            s.mouse_move(5, 5, 1, 1, Modifiers::NONE);
        }
    }
    let _ = s.shape_in_rect(Rect::new(i32::MAX, i32::MIN, i32::MAX, i32::MIN), true);
    let _ = s.shape_all_in_rect(Rect::new(i32::MAX, i32::MIN, i32::MAX, i32::MIN));
    for arg in [SelectArg::Null, SelectArg::Reset, SelectArg::Point(PointRef::RotateHandle), SelectArg::Point(PointRef::RotationAxis), SelectArg::Point(PointRef::Pt(99)), SelectArg::Point(PointRef::Detached)] {
        s.set_select_point(arg);
        s.mouse_move(1, 2, 3, 4, Modifiers::NONE);
    }
    for z in [0.0, -1.0, 0.25, 4.0] {
        s.set_zoom_level(z);
    }
    s.set_rotation_angle(90);
    s.set_rotation_angle(-360);
    s.nudge(i32::MAX, i32::MIN);
    s.move_to(i32::MIN, i32::MAX);
    s.set_left(i32::MAX);
    s.set_top(i32::MIN);
    for a in [0.0, -1.0, f64::INFINITY, f64::NAN, 1e300] {
        s.resize(a);
        s.resize_from_original(a);
    }
    s.set_pixel_size(-5);
    s.resize_pixels_by(-100);
    s.match_shape(&other);
    s.match_pixel_size(&other);
    s.end_add_new();
    s.on_move_point();
    s.deselect();
    s.selected_point = None;
    s.mouse_move(1, 2, 3, 4, Modifiers::NONE);
    for r in s.properties() {
        let value = match r.kind {
            properties::PropertyKind::Int => V::Int(-1),
            properties::PropertyKind::Bool => V::Bool(true),
            properties::PropertyKind::Text => V::Text(String::new()),
            properties::PropertyKind::Double => V::Double(0.0),
            properties::PropertyKind::Point => V::Point { x: -3, y: 2 },
            properties::PropertyKind::Enum(m) => V::Enum(m[1 % m.len()].to_string()),
            properties::PropertyKind::LinkedElements => continue,
            properties::PropertyKind::LinkedElement => V::LinkedElement(None),
        };
        let _ = s.set_property(r.id, value, "");
    }
    s.reconfigure(None);
    let _ = s.to_xml_named("Shape");
    s.roundtrip_state();
}

#[test]
fn nothing_panics_on_normal_shapes() {
    for s in every_shape() {
        exercise(s);
    }
}

#[test]
fn nothing_panics_on_extreme_coordinates() {
    for extreme in [i32::MAX, i32::MIN, 0] {
        for mut s in every_shape() {
            for q in &mut s.points {
                q.x = extreme;
                q.y = extreme.wrapping_neg();
            }
            if let Some(strings) = &mut s.strings {
                for l in strings {
                    for q in &mut l.points {
                        q.x = extreme;
                        q.y = extreme;
                    }
                }
            }
            exercise(s);
        }
    }
}

#[test]
fn nothing_panics_on_empty_and_tiny_shapes() {
    for mut s in every_shape() {
        s.pixels.clear();
        if let Some(strings) = &mut s.strings {
            strings.clear();
        }
        exercise(s);
    }
    for mut s in every_shape() {
        if let Some(strings) = &mut s.strings {
            strings.truncate(1);
            for l in strings {
                l.pixels.truncate(1);
            }
        }
        s.pixels.truncate(1);
        exercise(s);
    }
}

#[test]
fn nothing_panics_on_zero_and_huge_counts() {
    use properties::PropertyValue as V;
    for (mut s, name) in [
        (star(), "Star Points"),
        (star(), "Light Count"),
        (mega_tree(), "String Count"),
        (mega_tree(), "String Coverage Degrees"),
        (mega_tree(), "Lights Per String"),
        (icicle(), "Icicle Count"),
        (pixel_grid(), "String Count"),
        (pixel_grid(), "Lights Per String"),
        (star_burst(), "StringCount"),
        (net(), "Light Spacing"),
        (line(), "Light Count"),
        (arch(), "Light Count"),
        (ellipse(), "Light Count"),
        (poly_line(), "Light Count"),
    ] {
        for v in [0, -1, 1, 2] {
            let _ = s.set_property(name, V::Int(v), "");
            exercise(s.clone());
        }
    }
}

#[test]
fn nothing_panics_on_hostile_xml() {
    const NS: &str = "xmlns=\"http://schemas.datacontract.org/2004/07/VixenModules.Preview.VixenPreview.Shapes\" xmlns:i=\"http://www.w3.org/2001/XMLSchema-instance\"";
    for ty in ["PreviewLine", "PreviewPolyLine", "PreviewArch", "PreviewCane", "PreviewRectangle", "PreviewEllipse", "PreviewTriangle", "PreviewStar", "PreviewStarBurst", "PreviewNet", "PreviewMegaTree", "PreviewMultiString", "PreviewIcicle", "PreviewPixelGrid", "PreviewSingle", "PreviewFlood", "PreviewCustom", "PreviewCustomProp", "PreviewMovingHead", "PreviewNoSuchThing"] {
        for body in [
            "".to_string(),
            "<PixelSize>abc</PixelSize>".to_string(),
            "<Pixels i:nil=\"true\" />".to_string(),
            "<_strings><PreviewBaseShape i:type=\"PreviewOther\" /></_strings>".to_string(),
            "<_strings><PreviewBaseShape i:type=\"PreviewLine\"><_points /></PreviewBaseShape></_strings>".to_string(),
            "<_points><PreviewPoint><X>1</X><Y>2</Y></PreviewPoint></_points>".to_string(),
            "<_points><PreviewPoint><X>1</X></PreviewPoint><PreviewPoint><Y>2</Y></PreviewPoint><PreviewPoint /></_points><_stringCount>2147483647</_stringCount>".to_string(),
        ] {
            let xml = format!("<Shape {NS} xmlns:a=\"http://schemas.datacontract.org/2004/07/VixenModules.Preview.VixenPreview.Shapes\" i:type=\"a:{ty}\">{body}</Shape>");
            let e = XElement::parse(&xml).unwrap();
            let s = shape_from_xml(&e);
            // Whatever it became, it is written back as it came unless it is a shape it understood.
            let out = s.to_xml_named("Shape");
            if matches!(s.kind, ShapeKind::Unknown(_)) {
                assert_eq!(e.first_difference(&out, String::new()), None, "{ty} {body}");
            }
            if !body.contains("2147483647") {
                exercise(s);
            }
        }
    }
}
