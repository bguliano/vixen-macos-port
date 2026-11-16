use super::polygon::{ellipse_record, line_record, polygon_record};
use super::*;
use crate::test_util::{in_contract_order, rgb, Grid};
use vixen_effects::testing;

const RED: RgbValue = RgbValue { r: 255, g: 0, b: 0 };
const WHITE: RgbValue = RgbValue { r: 255, g: 255, b: 255 };
const BLUE: RgbValue = RgbValue { r: 0, g: 0, b: 255 };

fn enum_value(name: &str) -> ParamValue {
    ParamValue::Enum(name.to_string())
}

fn record(shape: &str, value: Params) -> Params {
    let mut r = Params::defaults(&gen::MORPH_POLYGON_PARAMS);
    r.set("Polygon", ParamValue::None);
    r.set(shape, ParamValue::Record(Box::new(value)));
    r
}

fn polygon(points: &[(f64, f64)]) -> Params {
    record("Polygon", polygon_record(points))
}

fn line(points: &[(f64, f64)]) -> Params {
    record("Line", line_record(points))
}

fn rectangle_corners(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64)> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

/// A shape that fills with one colour.
fn solid(mut r: Params, color: Color) -> Params {
    r.set("FillType", enum_value("Solid"));
    r.set("FillColor", ParamValue::Gradient(ColorGradient::solid(color)));
    r
}

/// Free form mode with these shapes.
fn free_form(p: &mut Params, shapes: Vec<Params>) {
    p.set("PolygonType", enum_value("FreeForm"));
    p.set("MorphPolygonData", ParamValue::RecordList(shapes));
}

fn visible(p: &Params) -> Vec<&'static str> {
    DESCRIPTOR.visible_params(p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect()
}

#[test]
fn smoke() {
    testing::smoke_test(&DESCRIPTOR);
    testing::assert_produces_light(&DESCRIPTOR);
}

// ---- the tables

#[test]
fn tables_are_in_data_contract_order() {
    assert!(in_contract_order(&gen::MORPH_POLYGON_PARAMS));
    assert!(in_contract_order(&gen::POLYGON_PARAMS));
    assert!(in_contract_order(&gen::LINE_PARAMS));
    assert!(in_contract_order(&gen::ELLIPSE_PARAMS));
    assert!(in_contract_order(&gen::POLYGON_POINT_PARAMS));
}

#[test]
fn a_new_record_is_a_new_morph_polygon() {
    let r = Params::defaults(&gen::MORPH_POLYGON_PARAMS);
    assert_eq!(r.int("HeadLength"), 4);
    assert_eq!(r.int("HeadDuration"), 20);
    assert_eq!(r.enum_name("FillType"), "Wipe");
    assert_eq!(r.gradient("HeadColor").get_color_at(0.0), Color::WHITE);
    assert_eq!(r.gradient("TailColor").get_color_at(0.0), Color::RED);
    assert_eq!(r.gradient("FillColor").get_color_at(0.0), Color::RED);
    assert_eq!(r.curve("FillBrightness").get_value(50.0), 100.0);
    assert_eq!(r.record("Line"), None);
    assert_eq!(r.record("Ellipse"), None);
    let shape = Shape::from_record(&r).unwrap();
    assert_eq!(shape, Shape::rectangle(10, 10));
    assert_eq!(shape.points()[2], PolygonPoint::new(9.0, 9.0));
}

#[test]
fn every_enum_member_is_in_the_table() {
    let names = |e: &EnumDesc| e.members.iter().map(|m| m.name).collect::<Vec<_>>();
    assert_eq!(names(&gen::ENUM_PolygonFillType), ["Wipe", "Solid", "Outline"]);
    assert_eq!(names(&gen::ENUM_PolygonType), ["Pattern", "FreeForm", "TimeBased"]);
    assert_eq!(names(&gen::ENUM_WipeRepeatDirection), ["Left", "Right", "Up", "Down"]);
}

// ---- visibility and setters

#[test]
fn pattern_mode_shows_the_pattern_and_wipe_settings() {
    let p = DESCRIPTOR.new_params();
    let v = visible(&p);
    for name in ["RepeatCount", "RepeatSkip", "Stagger", "RepeatDirection", "HeadLength", "TailColor", "FillType", "HeadBrightness"] {
        assert!(v.contains(&name), "{name}");
    }
    for name in ["FillColor", "FillPolygon", "FillBrightness", "MorphPolygonData"] {
        assert!(!v.contains(&name), "{name}");
    }
    assert!(v.contains(&"TargetPositioning"));
}

#[test]
fn a_solid_pattern_swaps_the_wipe_settings_for_the_colour() {
    let mut p = DESCRIPTOR.new_params();
    p.set("FillType", enum_value("Solid"));
    let v = visible(&p);
    for name in ["HeadLength", "HeadDuration", "Acceleration", "HeadColor", "TailColor", "Stagger"] {
        assert!(!v.contains(&name), "{name}");
    }
    assert!(v.contains(&"FillColor"));
    assert!(v.contains(&"RepeatCount"));
}

#[test]
fn free_form_shows_the_polygon_list_and_time_based_the_fill() {
    let mut p = DESCRIPTOR.new_params();
    p.set("PolygonType", enum_value("FreeForm"));
    let v = visible(&p);
    assert!(v.contains(&"MorphPolygonData"));
    assert!(!v.contains(&"RepeatCount"));
    assert!(!v.contains(&"FillType"));

    p.set("PolygonType", enum_value("TimeBased"));
    let v = visible(&p);
    for name in ["FillColor", "FillPolygon", "FillBrightness"] {
        assert!(v.contains(&name), "{name}");
    }
    assert!(!v.contains(&"HeadColor"));
    assert!(!v.contains(&"MorphPolygonData"));
}

#[test]
fn locations_hide_the_orientation() {
    let mut p = DESCRIPTOR.new_params();
    assert!(visible(&p).contains(&"StringOrientation"));
    p.set("TargetPositioning", enum_value("Locations"));
    assert!(!visible(&p).contains(&"StringOrientation"));
}

#[test]
fn a_polygon_record_shows_what_its_fill_type_uses() {
    let mut r = Params::defaults(&gen::MORPH_POLYGON_PARAMS);
    let mut v = Visibility::default();
    polygon_visibility(&r, &r, &mut v);
    assert!(v.overrides.contains(&("HeadLength", true)));
    assert!(v.overrides.contains(&("FillColor", false)));
    r.set("FillType", enum_value("Outline"));
    let mut v = Visibility::default();
    polygon_visibility(&r, &r, &mut v);
    assert!(v.overrides.contains(&("HeadLength", false)));
    assert!(v.overrides.contains(&("FillBrightness", true)));
}

#[test]
fn targets_give_a_new_effect_its_first_shape_and_a_loaded_one_keeps_what_it_has() {
    let grid = Grid::new(5, 8);
    let targets = [grid.root];
    let t = testing::target_info(&grid.tree, &targets);

    let mut loaded = DESCRIPTOR.new_params();
    DESCRIPTOR.load_data(&mut loaded, &t);
    assert!(loaded.records("MorphPolygonData").is_empty());
    // With none, MorphPolygons[0] throws and the effect stays dark.
    assert!(grid.frames(&testing::render_loaded(&DESCRIPTOR, &grid.tree, &targets, 200, |_| {})).is_empty());

    let mut p = DESCRIPTOR.new_params();
    DESCRIPTOR.set_targets(&mut p, &t);
    let shape = Shape::from_record(&p.records("MorphPolygonData")[0]).unwrap();
    // A string count of 5 and 8 pixels: BufferWi and BufferHt for the effect's default horizontal strings.
    assert_eq!(shape, Shape::rectangle(8, 5));

    // A second assignment with nothing rendered yet leaves the shape where it is.
    DESCRIPTOR.set_targets(&mut p, &t);
    assert_eq!(Shape::from_record(&p.records("MorphPolygonData")[0]).unwrap(), Shape::rectangle(8, 5));
}

#[test]
fn a_skinny_display_element_gets_a_line() {
    let mut tree = ElementTree::new();
    let s = tree.add_string("S", 8);
    let targets = [s];
    let mut p = DESCRIPTOR.new_params();
    DESCRIPTOR.set_targets(&mut p, &testing::target_info(&tree, &targets));
    let shape = Shape::from_record(&p.records("MorphPolygonData")[0]).unwrap();
    assert_eq!(shape, Shape::Line(vec![PolygonPoint::new(0.0, 0.0), PolygonPoint::new(7.0, 0.0)]));
}

#[test]
fn moving_to_a_larger_display_element_scales_the_shapes() {
    let grid = Grid::new(10, 20);
    let targets = [grid.root];
    let t = testing::target_info(&grid.tree, &targets);
    let mut p = DESCRIPTOR.new_params();
    // Saved on a display element half this size.
    p.set("DisplayElementWidth", ParamValue::Int(10));
    p.set("DisplayElementHeight", ParamValue::Int(5));
    p.set("MorphPolygonData", ParamValue::RecordList(vec![polygon(&rectangle_corners(0.0, 0.0, 9.0, 4.0))]));
    DESCRIPTOR.set_targets(&mut p, &t);
    let shape = Shape::from_record(&p.records("MorphPolygonData")[0]).unwrap();
    assert_eq!(shape.points()[2], PolygonPoint::new(19.0, 9.0));
    assert_eq!((p.int("DisplayElementWidth"), p.int("DisplayElementHeight")), (20, 10));
}

#[test]
fn the_margin_setter_moves_the_shapes_by_the_change() {
    let grid = Grid::new(10, 20);
    let targets = [grid.root];
    let t = testing::target_info(&grid.tree, &targets);
    let mut p = DESCRIPTOR.new_params();
    DESCRIPTOR.set_targets(&mut p, &t);
    DESCRIPTOR.set_param(&mut p, "Margin", ParamValue::Int(10), &t);
    let first = |p: &Params| Shape::from_record(&p.records("MorphPolygonData")[0]).unwrap().points()[0];
    // 10% of 20 across and of 10 up.
    assert_eq!(first(&p), PolygonPoint::new(2.0, 1.0));
    DESCRIPTOR.set_param(&mut p, "Margin", ParamValue::Int(50), &t);
    assert_eq!(first(&p), PolygonPoint::new(10.0, 5.0));
    DESCRIPTOR.set_param(&mut p, "Margin", ParamValue::Int(0), &t);
    assert_eq!(first(&p), PolygonPoint::new(0.0, 0.0));
}

#[test]
fn turning_the_strings_scales_the_shapes_to_the_new_shape_of_the_buffer() {
    let grid = Grid::new(10, 20);
    let targets = [grid.root];
    let t = testing::target_info(&grid.tree, &targets);
    let mut p = DESCRIPTOR.new_params();
    DESCRIPTOR.set_targets(&mut p, &t);
    assert_eq!(Shape::from_record(&p.records("MorphPolygonData")[0]).unwrap(), Shape::rectangle(20, 10));
    DESCRIPTOR.set_param(&mut p, "StringOrientation", enum_value("Vertical"), &t);
    assert_eq!(Shape::from_record(&p.records("MorphPolygonData")[0]).unwrap(), Shape::rectangle(10, 20));
}

#[test]
fn polygon_rows_carry_their_own_visibility() {
    assert!(gen::MORPH_POLYGON_DATA.visibility.is_some());
}

#[test]
fn setters_reach_into_the_polygons() {
    let mut p = DESCRIPTOR.new_params();
    let r = Params::defaults(&gen::MORPH_POLYGON_PARAMS);
    p.set("MorphPolygonData", ParamValue::RecordList(vec![r.clone(), r]));

    p.set("FillType", enum_value("Outline"));
    param_changed("FillType", &mut p);
    let first = &p.records("MorphPolygonData")[0];
    assert_eq!(first.enum_name("FillType"), "Outline");
    assert_eq!(first.record("Polygon").unwrap().enum_name("FillType"), "Outline");

    p.set("HeadBrightness", ParamValue::Curve(Curve::flat(25.0)));
    param_changed("HeadBrightness", &mut p);
    assert_eq!(p.records("MorphPolygonData")[1].curve("HeadBrightness").get_value(50.0), 25.0);

    // Pattern mode has one polygon, and leaving it for free form clears the repeats.
    param_changed("PolygonType", &mut p);
    assert_eq!(p.records("MorphPolygonData").len(), 1);
    p.set("RepeatCount", ParamValue::Int(4));
    p.set("PolygonType", enum_value("FreeForm"));
    param_changed("PolygonType", &mut p);
    assert_eq!(p.int("RepeatCount"), 0);
}

#[test]
fn locations_force_vertical_strings() {
    let mut p = DESCRIPTOR.new_params();
    p.set("TargetPositioning", enum_value("Locations"));
    param_changed("TargetPositioning", &mut p);
    assert_eq!(p.enum_name("StringOrientation"), "Vertical");
}

// ---- the default shape

/// 10 strings of 20 pixels, one second: 20 frames of the default wipe. The polygon fills the display, its start side is
/// the top row, and the arithmetic below is `SetupRenderPattern` and `DrawWipe` by hand.
#[test]
fn the_default_wipe_runs_down_from_the_top() {
    let grid = Grid::new(10, 20);
    let f = grid.render(&DESCRIPTOR, 1000, |_| {});

    // Frame 0: head position 0, nothing yet.
    assert!(f.lit_pixels(0).is_empty(), "{}", f.picture(0));

    // Frame 1, 50 ms: the head covers 14 lines in 200 ms, so it is at 3.5, rounded to 4, with a length of 10.
    for y in 6..10 {
        for x in 0..20 {
            assert_eq!(f.at(1, x, y), WHITE, "({x},{y})\n{}", f.picture(1));
        }
    }
    assert!(!f.lit(1, 0, 5));

    // Frame 3, 150 ms: the head is at 10.500000000000002, which rounds to 11, not a tie, so it starts at 7 and the end
    // clamps to the last line. The tail covers 10 lines in 1000 ms and is at 1. Its intensity is
    // (line - 1 + 1) / (7 - 1), doubled, or 1 once that is over a half.
    assert!(!f.lit(3, 0, 9), "line 0 is behind the tail\n{}", f.picture(3));
    assert_eq!(f.at(3, 5, 8), rgb(85, 0, 0));
    assert_eq!(f.at(3, 5, 7), rgb(170, 0, 0));
    assert_eq!(f.at(3, 5, 6), RED);
    assert_eq!(f.at(3, 5, 3), RED);
    assert_eq!(f.at(3, 5, 2), WHITE);
    assert_eq!(f.at(3, 5, 1), WHITE);
    assert!(!f.lit(3, 5, 0), "the head draws up to its end, not including it");

    // Frame 10: the head finished long ago. The tail is at 5 and runs to the end, so over 5 lines.
    assert!(!f.lit(10, 5, 5), "{}", f.picture(10));
    assert_eq!(f.at(10, 5, 4), rgb(102, 0, 0));
    assert_eq!(f.at(10, 5, 3), rgb(204, 0, 0));
    assert_eq!(f.at(10, 5, 2), RED);
    assert_eq!(f.at(10, 5, 0), RED);
}

#[test]
fn a_skinny_element_wipes_along_a_line() {
    // One string of 12 pixels with horizontal strings is 12 wide and one high: a line along x.
    let grid = Grid::new(1, 12);
    let f = grid.render(&DESCRIPTOR, 1000, |_| {});
    assert!(!f.is_dark());
    // Frame 1: (12 + 4) / 200 ms * 50 ms = 4 of 12, so the head has reached 4.
    assert_eq!(f.lit_pixels(1), vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
}

#[test]
fn a_tall_skinny_element_keeps_the_constructors_polygon() {
    // The C# forgets to assign the line it builds for this case. With vertical strings the same string is one wide
    // and twelve high, and the default 10 x 10 polygon's wipe runs from y = 9 in polygon terms, so only the
    // bottom part of the element is ever lit. In frame buffer terms polygon y 0..9 is 11 down to 2.
    let mut tree = ElementTree::new();
    let string = tree.add_string("S", 12);
    let targets = [string];
    let out = testing::render_with(&DESCRIPTOR, &tree, &targets, 1000, |p| {
        p.set("StringOrientation", enum_value("Vertical"));
    });
    let elements = tree.distinct_elements(string);
    let lit_ever = |y: usize| match &out.nodes_for(&elements[y]).unwrap()[0].intent.data {
        IntentData::ArrayRgb(a) => a.values().iter().any(|v| *v != RgbValue::BLACK),
        other => panic!("{other:?}"),
    };
    assert!(lit_ever(11) && lit_ever(2));
    assert!(!lit_ever(1) && !lit_ever(0));
}

#[test]
fn the_top_level_fill_type_reaches_the_default_shape() {
    let grid = Grid::new(4, 6);
    let f = grid.render(&DESCRIPTOR, 200, |p| {
        p.set("FillType", enum_value("Solid"));
        p.set("FillColor", ParamValue::Gradient(ColorGradient::solid(Color::BLUE)));
    });
    for y in 0..4 {
        for x in 0..6 {
            assert_eq!(f.at(0, x, y), BLUE);
        }
    }
}

// ---- static shapes

#[test]
fn a_solid_polygon_fills_including_its_far_edges() {
    let grid = Grid::new(8, 10);
    let shape = solid(polygon(&rectangle_corners(2.0, 1.0, 6.0, 4.0)), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 100, |p| free_form(p, vec![shape]));
    // Shape y goes down from the top of the buffer: 1..=4 is rows 6 down to 3.
    let mut want = Vec::new();
    for y in 3..=6 {
        for x in 2..=6 {
            want.push((x, y));
        }
    }
    assert_eq!(f.lit_pixels(0), want, "{}", f.picture(0));
    assert_eq!(f.at(0, 2, 3), BLUE);
}

#[test]
fn an_outline_leaves_the_inside_dark() {
    let grid = Grid::new(8, 10);
    let mut shape = solid(polygon(&rectangle_corners(2.0, 1.0, 6.0, 4.0)), Color::BLUE);
    shape.set("FillType", enum_value("Outline"));
    let f = grid.render(&DESCRIPTOR, 100, |p| free_form(p, vec![shape]));
    assert!(f.lit(0, 2, 3));
    assert!(f.lit(0, 6, 6));
    assert!(!f.lit(0, 3, 4), "{}", f.picture(0));
    assert!(!f.lit(0, 5, 5));
}

#[test]
fn a_static_line_includes_both_ends() {
    let grid = Grid::new(3, 10);
    let mut shape = solid(line(&[(0.0, 0.0), (9.0, 0.0)]), Color::BLUE);
    shape.set("FillType", enum_value("Outline"));
    let f = grid.render(&DESCRIPTOR, 100, |p| free_form(p, vec![shape]));
    assert_eq!(f.lit_pixels(0), (0..10).map(|x| (x, 2)).collect::<Vec<_>>(), "{}", f.picture(0));
}

#[test]
fn points_round_away_from_zero_for_static_shapes() {
    // 2.5 rounds to 3 here. Math.Round without the MidpointRounding argument would give 2.
    let grid = Grid::new(6, 6);
    let mut shape = solid(line(&[(2.5, 0.0), (2.5, 5.0)]), Color::BLUE);
    shape.set("FillType", enum_value("Outline"));
    let f = grid.render(&DESCRIPTOR, 100, |p| free_form(p, vec![shape]));
    assert!(f.lit(0, 3, 2), "{}", f.picture(0));
    assert!(!f.lit(0, 2, 2));
}

#[test]
fn a_line_with_too_few_points_stays_dark() {
    // `points[1]` throws.
    let grid = Grid::new(6, 6);
    let shape = solid(line(&[(1.0, 1.0)]), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 100, |p| free_form(p, vec![shape]));
    assert!(f.is_empty());
}

#[test]
fn a_solid_ellipse_is_drawn_around_its_truncated_center() {
    let grid = Grid::new(21, 21);
    let shape = solid(record("Ellipse", ellipse_record(&[], (10.7, 10.2), 8.0, 8.0, 0.0)), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 100, |p| free_form(p, vec![shape]));
    let lit = f.lit_pixels(0);
    let xs: Vec<usize> = lit.iter().map(|p| p.0).collect();
    assert_eq!((*xs.iter().min().unwrap(), *xs.iter().max().unwrap()), (6, 13));
    // The widest row is the centre, which is row 10 from either end of the flip.
    assert_eq!(lit.iter().filter(|p| p.1 == 10).count(), 8, "{}", f.picture(0));
}

#[test]
fn a_rotated_ellipse_turns_clockwise_on_the_bitmap() {
    let grid = Grid::new(21, 21);
    let shape = solid(record("Ellipse", ellipse_record(&[], (10.0, 10.0), 16.0, 2.0, 45.0)), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 100, |p| free_form(p, vec![shape]));
    // Clockwise with y down: +x heads towards +y, which in the flipped frame buffer is the lower right.
    assert!(f.lit(0, 14, 6), "{}", f.picture(0));
    assert!(f.lit(0, 6, 14));
    assert!(!f.lit(0, 14, 14));
    assert!(!f.lit(0, 6, 6));
}

#[test]
fn wipes_draw_under_static_shapes() {
    let grid = Grid::new(4, 6);
    let wipe = polygon(&rectangle_corners(0.0, 0.0, 5.0, 3.0));
    let over = solid(polygon(&rectangle_corners(0.0, 0.0, 5.0, 3.0)), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 1000, |p| free_form(p, vec![wipe, over]));
    assert_eq!(f.at(1, 3, 3), BLUE);
}

// ---- the margin

#[test]
fn a_margin_moves_the_display_into_the_middle_of_the_buffer() {
    // 50% of 10 is 5 on every side. The display is 5..=14 of the 20 x 20 virtual buffer.
    let grid = Grid::new(10, 10);
    let inside = solid(polygon(&rectangle_corners(5.0, 5.0, 7.0, 6.0)), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 100, |p| {
        free_form(p, vec![inside]);
        p.set("Margin", ParamValue::Int(50));
    });
    let want: Vec<(usize, usize)> = [(0, 8), (1, 8), (2, 8), (0, 9), (1, 9), (2, 9)].to_vec();
    assert_eq!(f.lit_pixels(0), want, "{}", f.picture(0));
}

#[test]
fn a_shape_entirely_in_the_margin_is_off_the_display() {
    let grid = Grid::new(10, 10);
    let outside = solid(polygon(&rectangle_corners(0.0, 0.0, 3.0, 3.0)), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 100, |p| {
        free_form(p, vec![outside]);
        p.set("Margin", ParamValue::Int(50));
    });
    assert!(f.is_dark());
    assert!(!f.is_empty());
}

#[test]
fn the_default_shape_stays_on_the_display_with_a_margin() {
    // In the C# the Margin setter moves the polygon by the margin. Here it is made in place.
    let grid = Grid::new(10, 10);
    let f = grid.render(&DESCRIPTOR, 100, |p| {
        p.set("FillType", enum_value("Solid"));
        p.set("Margin", ParamValue::Int(20));
    });
    assert_eq!(f.lit_pixels(0).len(), 100, "{}", f.picture(0));
}

// ---- scaling

#[test]
fn shapes_scale_when_the_display_element_changed_size() {
    // Saved on a 10 x 10 element, rendered on 20 x 30: the corners go to the corners.
    let grid = Grid::new(30, 20);
    let shape = solid(polygon(&rectangle_corners(0.0, 0.0, 9.0, 9.0)), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 100, |p| {
        free_form(p, vec![shape]);
        p.set("DisplayElementWidth", ParamValue::Int(10));
        p.set("DisplayElementHeight", ParamValue::Int(10));
    });
    assert_eq!(f.lit_pixels(0).len(), 600, "{}", f.picture(0));
}

#[test]
fn shapes_do_not_scale_without_a_previous_size_or_a_change() {
    let grid = Grid::new(10, 10);
    let render = |width: i32| {
        let shape = solid(polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0)), Color::BLUE);
        grid.render(&DESCRIPTOR, 100, |p| {
            free_form(p, vec![shape]);
            p.set("DisplayElementWidth", ParamValue::Int(width));
            p.set("DisplayElementHeight", ParamValue::Int(width));
        })
        .lit_pixels(0)
        .len()
    };
    assert_eq!(render(0), 25);
    assert_eq!(render(10), 25);
    // From 5 x 5 the same shape covers the whole element.
    assert_eq!(render(5), 100);
}

#[test]
fn scaling_takes_the_margin_into_account() {
    // Saved on 10 with 10%, so a 12 buffer. Now 20 with 10%, so a 24 buffer: a factor of 23 / 11.
    let grid = Grid::new(20, 20);
    let shape = solid(polygon(&rectangle_corners(1.0, 1.0, 11.0, 11.0)), Color::BLUE);
    let f = grid.render(&DESCRIPTOR, 100, |p| {
        free_form(p, vec![shape]);
        p.set("Margin", ParamValue::Int(10));
        p.set("DisplayElementWidth", ParamValue::Int(10));
        p.set("DisplayElementHeight", ParamValue::Int(10));
    });
    // Point 1 goes to 2.09, rounded to 2, the display's first pixel. The whole display is covered.
    assert_eq!(f.lit_pixels(0).len(), 400, "{}", f.picture(0));
}

// ---- patterns

fn pattern_rectangle(p: &mut Params) {
    let mut r = Params::defaults(&gen::MORPH_POLYGON_PARAMS);
    r.set("Polygon", ParamValue::Record(Box::new(polygon_record(&rectangle_corners(0.0, 8.0, 5.0, 9.0)))));
    r.set("FillType", ParamValue::Enum(p.enum_name("FillType").to_string()));
    p.set("MorphPolygonData", ParamValue::RecordList(vec![r]));
}

#[test]
fn a_solid_pattern_repeats_in_its_direction() {
    let grid = Grid::new(10, 12);
    let f = grid.render(&DESCRIPTOR, 100, |p| {
        p.set("FillType", enum_value("Solid"));
        pattern_rectangle(p);
        p.set("RepeatCount", ParamValue::Int(2));
        p.set("RepeatSkip", ParamValue::Int(4));
        p.set("RepeatDirection", enum_value("Up"));
        p.set("FillColor", ParamValue::Gradient(ColorGradient::solid(Color::BLUE)));
    });
    // Rows 0..=1 is the shape, and each copy is 4 higher.
    for y in [0, 1, 4, 5, 8, 9] {
        assert_eq!(f.at(0, 3, y), BLUE, "row {y}\n{}", f.picture(0));
    }
    for y in [2, 3, 6, 7] {
        assert!(!f.lit(0, 3, y), "row {y}\n{}", f.picture(0));
    }
}

#[test]
fn a_solid_pattern_going_right_moves_along_x() {
    let grid = Grid::new(2, 12);
    let f = grid.render(&DESCRIPTOR, 100, |p| {
        p.set("FillType", enum_value("Solid"));
        let mut r = Params::defaults(&gen::MORPH_POLYGON_PARAMS);
        r.set("Polygon", ParamValue::Record(Box::new(polygon_record(&rectangle_corners(0.0, 0.0, 1.0, 1.0)))));
        p.set("MorphPolygonData", ParamValue::RecordList(vec![r]));
        p.set("RepeatCount", ParamValue::Int(1));
        p.set("RepeatSkip", ParamValue::Int(5));
        p.set("RepeatDirection", enum_value("Right"));
    });
    let row: Vec<bool> = (0..12).map(|x| f.lit(0, x, 1)).collect();
    assert_eq!(row, [true, true, false, false, false, true, true, false, false, false, false, false]);
}

#[test]
fn a_wiped_pattern_repeats_in_the_same_direction_and_staggers() {
    let grid = Grid::new(10, 12);
    let f = grid.render(&DESCRIPTOR, 1000, |p| {
        pattern_rectangle(p);
        p.set("RepeatCount", ParamValue::Int(1));
        p.set("RepeatSkip", ParamValue::Int(4));
        p.set("RepeatDirection", enum_value("Up"));
        p.set("Stagger", ParamValue::Int(50));
    });
    // The first copy starts at once. The second waits for half the effect, and draws 4 rows up.
    assert!(!f.lit_pixels(4).is_empty());
    assert!(f.lit_pixels(4).iter().all(|p| p.1 <= 1), "{}", f.picture(4));
    assert!(f.lit_pixels(11).iter().any(|p| p.1 >= 4), "{}", f.picture(11));
    for frame in 0..10 {
        assert!(f.lit_pixels(frame).iter().all(|p| p.1 < 4), "frame {frame}\n{}", f.picture(frame));
    }
}

#[test]
fn a_stagger_that_would_overrun_the_effect_is_cut_down() {
    // 100% stagger over 5 repeats doesn't fit. `(int)(((total / 5) / total) * 100)` makes it 20.
    let mut e = Morph::from_params(&{
        let mut p = DESCRIPTOR.new_params();
        p.set("Stagger", ParamValue::Int(100));
        p.set("RepeatCount", ParamValue::Int(5));
        let grid = Grid::new(4, 10);
        let targets = [grid.root];
        DESCRIPTOR.set_targets(&mut p, &testing::target_info(&grid.tree, &targets));
        p
    });
    let info = crate::test_util::info(4, 10, 1000);
    e.setup(&info).unwrap();
    assert_eq!(e.wipe_data[0].stagger, 20.0);
    assert_eq!(e.wipe_data[0].head_is_done.len(), 6);
}

#[test]
fn a_start_offset_delays_a_free_form_wipe() {
    let grid = Grid::new(5, 5);
    let mut wipe = polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0));
    wipe.set("StartOffset", ParamValue::Int(50));
    let f = grid.render(&DESCRIPTOR, 1000, |p| free_form(p, vec![wipe]));
    for frame in 0..=10 {
        assert!(f.lit_pixels(frame).is_empty(), "frame {frame}");
    }
    assert!(!f.lit_pixels(11).is_empty());
}

#[test]
fn a_triangle_wipes_between_two_sides_and_a_point() {
    let grid = Grid::new(10, 10);
    let wipe = polygon(&[(0.0, 0.0), (9.0, 0.0), (9.0, 9.0)]);
    let f = grid.render(&DESCRIPTOR, 1000, |p| free_form(p, vec![wipe]));
    assert!(!f.is_dark());
    // Everything drawn is on the upper right of the (0, 0) to (9, 9) side, which is the frame buffer's upper right.
    for frame in 0..f.frame_count {
        for (x, y) in f.lit_pixels(frame) {
            assert!(x + y >= 8, "frame {frame} ({x},{y})\n{}", f.picture(frame));
        }
    }
}

#[test]
fn a_polygon_with_other_point_counts_never_wipes() {
    let grid = Grid::new(10, 10);
    let wipe = polygon(&[(0.0, 0.0), (9.0, 0.0), (9.0, 9.0), (5.0, 9.0), (0.0, 9.0)]);
    let f = grid.render(&DESCRIPTOR, 1000, |p| free_form(p, vec![wipe]));
    assert!(f.is_dark());
}

// ---- ellipse wipes

fn wipe_ellipse(center: (f64, f64), width: f64, height: f64) -> Params {
    let (cx, cy) = center;
    let (w, h) = (width / 2.0, height / 2.0);
    let corners = rectangle_corners(cx - w, cy - h, cx + w, cy + h);
    record("Ellipse", ellipse_record(&corners, center, width, height, 0.0))
}

#[test]
fn an_ellipse_wipe_is_masked_by_the_ellipse() {
    let grid = Grid::new(21, 21);
    let wipe = wipe_ellipse((10.0, 10.0), 16.0, 16.0);
    let f = grid.render(&DESCRIPTOR, 1000, |p| free_form(p, vec![wipe]));
    let mut lit_somewhere = false;
    for frame in 0..f.frame_count {
        for (x, y) in f.lit_pixels(frame) {
            lit_somewhere = true;
            let (dx, dy) = (x as f64 - 10.0, y as f64 - 10.0);
            assert!(dx * dx + dy * dy <= 9.0 * 9.0, "({x},{y}) is outside the ellipse\n{}", f.picture(frame));
        }
    }
    assert!(lit_somewhere);
}

#[test]
fn an_ellipse_wipe_past_the_top_of_the_buffer_stays_dark() {
    // The repeats lift the rectangle off the top, the mask lookup goes out of range, and the C# array access throws.
    let grid = Grid::new(10, 10);
    let f = grid.render(&DESCRIPTOR, 1000, |p| {
        let mut r = wipe_ellipse((5.0, 7.0), 4.0, 4.0);
        r.set("FillType", enum_value("Wipe"));
        p.set("MorphPolygonData", ParamValue::RecordList(vec![r]));
        p.set("RepeatCount", ParamValue::Int(3));
        p.set("RepeatSkip", ParamValue::Int(5));
        p.set("RepeatDirection", enum_value("Up"));
    });
    assert!(f.is_empty());
}

#[test]
fn the_mask_is_kept_for_each_repeat() {
    let mut p = DESCRIPTOR.new_params();
    p.set("MorphPolygonData", ParamValue::RecordList(vec![wipe_ellipse((5.0, 5.0), 4.0, 4.0)]));
    p.set("RepeatCount", ParamValue::Int(1));
    let mut e = Morph::from_params(&p);
    let info = crate::test_util::info(10, 10, 1000);
    e.setup(&info).unwrap();
    assert_eq!(e.wipe_data[0].masks.len(), 2);
    let mut buf = PixelFrameBuffer::new(10, 10);
    e.render_internal(1, &info, &mut buf).unwrap();
    assert!(e.wipe_data[0].masks[0].is_some());
}

// ---- time based

fn time_based(p: &mut Params, shapes: Vec<Params>) {
    p.set("PolygonType", enum_value("TimeBased"));
    p.set("MorphPolygonData", ParamValue::RecordList(shapes));
    p.set("FillColor", ParamValue::Gradient(ColorGradient::solid(Color::BLUE)));
}

fn at_time(mut r: Params, time: f64) -> Params {
    r.set("Time", ParamValue::Double(time));
    r
}

#[test]
fn time_based_moves_between_key_shapes() {
    let grid = Grid::new(11, 21);
    let start = at_time(polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0)), 0.0);
    let end = at_time(polygon(&rectangle_corners(16.0, 0.0, 20.0, 4.0)), 1.0);
    let f = grid.render(&DESCRIPTOR, 1000, |p| time_based(p, vec![start, end]));

    // Deliberate: the first shape's time isn't included (`firstPolygon.Time < timeFraction`), so frame 0 is dark.
    assert!(f.lit_pixels(0).is_empty());

    // Halfway along, frame 10, x has moved 8 of 16. Shape y 0..=4 is rows 10 down to 6.
    let lit = f.lit_pixels(10);
    assert_eq!(lit.len(), 25, "{}", f.picture(10));
    assert!(lit.iter().all(|p| (8..=12).contains(&p.0) && (6..=10).contains(&p.1)), "{}", f.picture(10));
    assert_eq!(f.at(10, 8, 6), BLUE);
}

#[test]
fn time_based_only_fills_when_told_to() {
    let grid = Grid::new(11, 21);
    let shapes = || vec![at_time(polygon(&rectangle_corners(0.0, 0.0, 6.0, 6.0)), 0.0), at_time(polygon(&rectangle_corners(0.0, 0.0, 6.0, 6.0)), 1.0)];
    let filled = grid.render(&DESCRIPTOR, 1000, |p| time_based(p, shapes()));
    assert!(filled.lit(5, 3, 7));
    let outline = grid.render(&DESCRIPTOR, 1000, |p| {
        time_based(p, shapes());
        p.set("FillPolygon", ParamValue::Bool(false));
    });
    assert!(!outline.lit(5, 3, 7), "{}", outline.picture(5));
    assert!(outline.lit(5, 0, 7));
}

#[test]
fn time_based_uses_the_effects_brightness() {
    let grid = Grid::new(5, 5);
    let shapes = vec![at_time(polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0)), 0.0), at_time(polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0)), 1.0)];
    let f = grid.render(&DESCRIPTOR, 1000, |p| {
        time_based(p, shapes);
        p.set("FillBrightness", ParamValue::Curve(Curve::flat(50.0)));
    });
    assert_eq!(f.at(5, 2, 2), rgb(0, 0, 127));
}

#[test]
fn a_time_based_line_morphs_too() {
    let grid = Grid::new(11, 21);
    let start = at_time(line(&[(0.0, 5.0), (4.0, 5.0)]), 0.0);
    let end = at_time(line(&[(10.0, 5.0), (14.0, 5.0)]), 1.0);
    let f = grid.render(&DESCRIPTOR, 1000, |p| time_based(p, vec![start, end]));
    assert_eq!(f.lit_pixels(10), (5..=9).map(|x| (x, 5)).collect::<Vec<_>>(), "{}", f.picture(10));
}

#[test]
fn a_time_based_ellipse_morphs_its_size_and_center() {
    let grid = Grid::new(41, 41);
    let start = at_time(wipe_ellipse((10.0, 20.0), 4.0, 4.0), 0.0);
    let end = at_time(wipe_ellipse((30.0, 20.0), 20.0, 20.0), 1.0);
    let f = grid.render(&DESCRIPTOR, 1000, |p| time_based(p, vec![start, end]));
    // Halfway the center is at x = 20 and the size is 12, so it reaches from 14 to just under 26.
    let lit = f.lit_pixels(10);
    let xs: Vec<usize> = lit.iter().map(|p| p.0).collect();
    assert_eq!((*xs.iter().min().unwrap(), *xs.iter().max().unwrap()), (14, 25), "{}", f.picture(10));
}

#[test]
fn one_time_based_shape_falls_back_to_free_form() {
    let grid = Grid::new(5, 5);
    let f = grid.render(&DESCRIPTOR, 200, |p| time_based(p, vec![polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0))]));
    assert_eq!(f.at(0, 2, 2), BLUE, "effect colour, filled, from the first frame");
    let g = grid.render(&DESCRIPTOR, 200, |p| {
        time_based(p, vec![polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0))]);
        p.set("FillPolygon", ParamValue::Bool(false));
    });
    assert!(!g.lit(0, 2, 2));
    assert!(g.lit(0, 0, 2));
}

#[test]
fn time_based_with_shapes_of_mixed_kinds_stays_dark() {
    // `morphPolygon.Polygon.Points` throws on a shape with no polygon.
    let grid = Grid::new(5, 5);
    let f = grid.render(&DESCRIPTOR, 200, |p| {
        time_based(p, vec![at_time(polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0)), 0.0), at_time(line(&[(0.0, 0.0), (4.0, 4.0)]), 1.0)]);
    });
    assert!(f.is_empty());
}

#[test]
fn the_timeline_search_uses_the_filtered_count_on_the_full_list() {
    // Only polygons with as many points as the first one count: two of these three. The search then walks the full
    // list with that count, so it never looks at the last pair.
    let mut p = DESCRIPTOR.new_params();
    time_based(
        &mut p,
        vec![
            at_time(polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)]), 0.0),
            at_time(polygon(&rectangle_corners(0.0, 0.0, 4.0, 4.0)), 0.5),
            at_time(polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)]), 1.0),
        ],
    );
    let mut e = Morph::from_params(&p);
    e.total_ms = 1000.0;
    assert_eq!(e.find_two_polygons_on_timeline(250.0, 2), Some(0));
    assert_eq!(e.find_two_polygons_on_timeline(750.0, 2), None, "the filtered count stops short of the last pair");
    assert_eq!(e.find_two_polygons_on_timeline(750.0, 3), Some(1));
    assert_eq!(e.find_two_polygons_on_timeline(0.0, 3), None, "the first shape's own time doesn't count");
}

// ---- acceleration arithmetic

#[test]
fn full_positive_acceleration_starts_from_a_standstill() {
    let (length, time) = (30, 2000.0);
    let a = positive_acceleration(length, time);
    let v = increasing_velocity_zero(a, length as f64, time);
    assert!(v.abs() < 1e-9, "{v}");
    let end = increasing_head_position(a, time, v);
    assert!((end - length as f64).abs() < 1e-9, "{end}");
}

#[test]
fn no_acceleration_is_constant_speed() {
    let v = increasing_velocity_zero(0.0, 10.0, 1000.0);
    assert_eq!(v, 0.01);
    assert_eq!(increasing_head_position(0.0, 500.0, v), 5.0);
}

#[test]
fn negative_acceleration_finds_the_largest_that_never_backs_up() {
    let a = negative_acceleration(20, 1000.0, -5, 50).unwrap();
    assert!(a < 0.0);
    assert!(is_increasing_acceleration_valid(a, 20.0, 1000.0, 50));
    assert!(!is_increasing_acceleration_valid(a * 1.01, 20.0, 1000.0, 50));
    // It is only checked at the frames, and the last pair of them is at 900 and 950 ms. Keeping x(950) >= x(900) with
    // x(t) = a t^3 / 6 + v t and v = (20 - a * 1000^3 / 6) / 1000 gives a = -1 / 13062500.
    assert!((a + 1.0 / 13062500.0).abs() < 1e-15, "{a}");
}

#[test]
fn an_acceleration_that_can_never_be_valid_fails_rather_than_hangs() {
    // With no length, halving only ever reaches zero, where the velocity isn't positive. The C# loops forever.
    assert_eq!(negative_acceleration(0, 1000.0, -5, 50), Err(Thrown));
}

#[test]
fn accelerating_a_wipe_delays_it() {
    // The lowest row the head has reached a quarter of the way through. The wipe starts at the top row.
    let grid = Grid::new(10, 20);
    let front = |acceleration: i32| {
        let f = grid.render(&DESCRIPTOR, 2000, |p| {
            p.set("Acceleration", ParamValue::Int(acceleration));
            p.set("HeadDuration", ParamValue::Int(100));
        });
        (0..10).find(|y| f.lit(10, 5, *y)).unwrap()
    };
    // At a constant speed the head covers 14 lines in 2000 ms, so 3.5 by then, and rounds to 4.
    assert_eq!(front(0), 6);
    assert!(front(10) > front(0), "speeding up starts slowly");
    assert!(front(-10) < front(0), "slowing down starts fast");
}

#[test]
fn the_head_does_not_back_up_after_it_is_done() {
    // Decelerating over half the effect, the curve brings the head's position back down in the second half. The done flag
    // keeps it off the display.
    let grid = Grid::new(10, 20);
    let f = grid.render(&DESCRIPTOR, 2000, |p| {
        p.set("Acceleration", ParamValue::Int(-10));
        p.set("HeadDuration", ParamValue::Int(50));
    });
    let white = |frame: usize| (0..10).any(|y| f.at(frame, 5, y) == WHITE);
    assert!(white(5), "{}", f.picture(5));
    let first_gone = (5..f.frame_count).find(|frame| !white(*frame)).unwrap();
    assert!((first_gone..f.frame_count).all(|frame| !white(frame)));
}

// ---- the small drawing helpers

#[test]
fn store_line_keeps_every_point_of_a_line() {
    let (mut x, mut y) = (Vec::new(), Vec::new());
    store_line(0, 0, 3, 1, &mut x, &mut y);
    assert_eq!(x, [0, 1, 2, 3]);
    assert_eq!(y, [0, 0, 1, 1]);
    x.clear();
    y.clear();
    store_line(2, 2, 2, 2, &mut x, &mut y);
    assert_eq!((x.len(), y.len()), (1, 1));
}

#[test]
fn tail_intensity_fades_towards_the_tail_end() {
    assert_eq!(tail_intensity(0, 0, 10), 0.2);
    assert_eq!(tail_intensity(3, 0, 10), 0.8);
    assert_eq!(tail_intensity(4, 0, 10), 1.0);
    assert_eq!(tail_intensity(9, 0, 10), 1.0);
}

#[test]
fn thick_lines_fill_diagonal_gaps() {
    let mut buf = PixelFrameBuffer::new(10, 10);
    buf.clear_buffer();
    let mut t = WipeTarget { buf: &mut buf, mask: None, wi: 10, ht: 10, offset_x: 0, offset_y: 0, direction: true };
    draw_thick_line(&mut t, 1, 1, 4, 4, Color::RED).unwrap();
    // direction true, moving up and right: fix is 7, so the pixel below each step is filled.
    for (x, y) in [(1, 1), (2, 2), (3, 3), (4, 4), (2, 1), (3, 2), (4, 3)] {
        assert_eq!(buf.get_color_at(x, y), Color::RED, "({x},{y})");
    }
    assert_eq!(buf.get_color_at(1, 2), Color::TRANSPARENT);
}

#[test]
fn a_mask_lookup_out_of_range_is_an_exception() {
    let mask = vec![true; 16];
    let mut buf = PixelFrameBuffer::new(4, 4);
    let mut t = WipeTarget { buf: &mut buf, mask: Some(&mask), wi: 4, ht: 4, offset_x: 0, offset_y: 0, direction: false };
    assert_eq!(draw_thick_line(&mut t, 0, 2, 3, 2, Color::RED), Ok(()));
    assert_eq!(draw_thick_line(&mut t, 2, 2, 6, 2, Color::RED), Err(Thrown));
    // Off the low side is simply not drawn.
    assert_eq!(draw_thick_line(&mut t, -2, 1, 1, 1, Color::RED), Ok(()));
}

// ---- locations

fn located_grid() -> (ElementTree, NodeId) {
    let mut tree = ElementTree::new();
    let root = testing::grid(&mut tree, "G", 4, 10);
    testing::add_locations(&mut tree, root);
    (tree, root)
}

/// The colour of the element at this location in this frame, on the grid `add_locations` lays out.
fn at_location(out: &EffectIntents, tree: &ElementTree, root: NodeId, frame: i64, x: i32, y: i32) -> RgbValue {
    for leaf in tree.distinct_leaves(root) {
        if tree.location(leaf) == (x, y) {
            let id = tree.node(leaf).element_id.unwrap();
            return match &out.nodes_for(&id).unwrap()[0].intent.data {
                IntentData::ArrayRgb(a) => a.at(TimeSpan::from_ms_i(frame * 50)),
                other => panic!("{other:?}"),
            };
        }
    }
    panic!("nothing at ({x},{y})");
}

#[test]
fn location_mode_reads_the_flipped_virtual_buffer() {
    // Locations are x 0..=36 and y 0..=12 in steps of 4. A shape's y runs the same way as a location's y.
    let (tree, root) = located_grid();
    let mut shape = solid(line(&[(0.0, 4.0), (36.0, 4.0)]), Color::BLUE);
    shape.set("FillType", enum_value("Outline"));
    let out = testing::render_with(&DESCRIPTOR, &tree, &[root], 200, |p| {
        free_form(p, vec![shape]);
        p.set("TargetPositioning", enum_value("Locations"));
    });
    assert_eq!(at_location(&out, &tree, root, 1, 0, 4), BLUE);
    assert_eq!(at_location(&out, &tree, root, 1, 36, 4), BLUE);
    assert_eq!(at_location(&out, &tree, root, 1, 0, 0), RgbValue::BLACK);
    assert_eq!(at_location(&out, &tree, root, 1, 8, 8), RgbValue::BLACK);
}

#[test]
fn location_mode_with_a_margin_offsets_by_the_margin() {
    let (tree, root) = located_grid();
    // 50% of 37 is 18.5, rounded to 18, and of 13 is 6.5, rounded to 6. Shape (18, 6) is location (0, 0).
    let mut shape = solid(polygon(&rectangle_corners(18.0, 6.0, 19.0, 7.0)), Color::BLUE);
    shape.set("FillType", enum_value("Solid"));
    let out = testing::render_with(&DESCRIPTOR, &tree, &[root], 200, |p| {
        free_form(p, vec![shape]);
        p.set("TargetPositioning", enum_value("Locations"));
        p.set("Margin", ParamValue::Int(50));
    });
    assert_eq!(at_location(&out, &tree, root, 1, 0, 0), BLUE);
    assert_eq!(at_location(&out, &tree, root, 1, 4, 0), RgbValue::BLACK);
    assert_eq!(at_location(&out, &tree, root, 1, 0, 4), RgbValue::BLACK);
}

#[test]
fn location_mode_runs_the_default_wipe() {
    let (tree, root) = located_grid();
    let out = testing::render_with(&DESCRIPTOR, &tree, &[root], 1000, |p| {
        p.set("TargetPositioning", enum_value("Locations"));
    });
    // The wipe starts at the shape's y = 0, which is location y = 0.
    assert_eq!(at_location(&out, &tree, root, 1, 8, 0), WHITE);
    assert_eq!(at_location(&out, &tree, root, 1, 8, 12), RgbValue::BLACK);
}

// ---- against the real thing

/// FNV-1a over every element's RGB values, frame by frame, in element order.
fn checksum(out: &EffectIntents, tree: &ElementTree, root: NodeId) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for id in tree.distinct_elements(root) {
        let IntentData::ArrayRgb(a) = &out.nodes_for(&id).unwrap()[0].intent.data else { panic!("not an RGB array") };
        for v in a.values() {
            for byte in [v.r, v.g, v.b] {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    hash
}

/// These checksums come from upstream's Morph.cs, compiled and run on .NET 10 with the same settings. Wipes of polygons
/// and lines don't touch GDI+, so nothing was stubbed for them apart from WPF's rotate arithmetic and a message box.
#[test]
fn wipes_match_what_the_csharp_rendered() {
    let wipe = |strings: usize, pixels: usize, ms: i64, located: bool, edit: &dyn Fn(&mut Params)| {
        let mut tree = ElementTree::new();
        let root = testing::grid(&mut tree, "G", strings, pixels);
        if located {
            testing::add_locations(&mut tree, root);
        }
        let out = testing::render_with(&DESCRIPTOR, &tree, &[root], ms, |p| {
            if located {
                p.set("TargetPositioning", enum_value("Locations"));
            }
            edit(p);
        });
        checksum(&out, &tree, root)
    };
    let shape = |p: &mut Params, shape: Params| p.set("MorphPolygonData", ParamValue::RecordList(vec![shape]));
    let timed = |mut r: Params, length: i32, duration: i32| {
        r.set("HeadLength", ParamValue::Int(length));
        r.set("HeadDuration", ParamValue::Int(duration));
        r
    };

    assert_eq!(wipe(8, 25, 2000, false, &|_| {}), 0xdd327a66f80373bc);
    assert_eq!(wipe(10, 20, 1000, false, &|p| { p.set("HeadDuration", ParamValue::Int(100)); }), 0xc613d590606fdb4d);
    assert_eq!(wipe(10, 20, 2000, false, &|p| { p.set("Acceleration", ParamValue::Int(-7)); }), 0x7021d7ca45db4a4d);
    assert_eq!(wipe(10, 20, 2000, false, &|p| { p.set("Acceleration", ParamValue::Int(7)); }), 0x657074d6e11c9ae5);
    assert_eq!(
        wipe(10, 20, 2000, false, &|p| {
            shape(p, polygon(&rectangle_corners(2.0, 6.0, 12.0, 9.0)));
            p.set("RepeatCount", ParamValue::Int(3));
            p.set("RepeatSkip", ParamValue::Int(2));
            p.set("Stagger", ParamValue::Int(25));
        }),
        0x0865e047ab409765
    );
    assert_eq!(
        wipe(10, 20, 1000, false, &|p| {
            shape(p, polygon(&[(0.0, 0.0), (19.0, 0.0), (10.0, 9.0)]));
            p.set("RepeatCount", ParamValue::Int(1));
            p.set("RepeatSkip", ParamValue::Int(3));
        }),
        0x851673b228ce1c16
    );
    assert_eq!(
        wipe(10, 20, 1000, false, &|p| {
            free_form(p, vec![polygon(&rectangle_corners(1.0, 2.0, 25.0, 8.0))]);
            p.set("Margin", ParamValue::Int(25));
        }),
        0x13b638ff212cbfb5
    );
    // A skewed quadrilateral exercises the gap filling in DrawThickLine.
    assert_eq!(
        wipe(16, 9, 1000, false, &|p| {
            free_form(p, vec![timed(polygon(&[(6.0, 15.0), (4.0, 5.0), (1.0, 2.0), (5.0, 5.0)]), 1, 10)]);
        }),
        0xcc8ddd539b81975f
    );
    assert_eq!(
        wipe(16, 5, 1000, false, &|p| {
            free_form(p, vec![timed(polygon(&[(3.0, 7.0), (4.0, 5.0), (2.0, 15.0), (3.0, 8.0)]), 1, 10)]);
            p.set("Margin", ParamValue::Int(10));
        }),
        0x0766dbb0c33b344b
    );
    assert_eq!(
        wipe(6, 10, 1000, true, &|p| {
            shape(p, polygon(&rectangle_corners(0.0, 0.0, 10.0, 12.0)));
            p.set("Margin", ParamValue::Int(10));
            p.set("RepeatCount", ParamValue::Int(2));
            p.set("RepeatSkip", ParamValue::Int(5));
            p.set("RepeatDirection", enum_value("Right"));
            p.set("Stagger", ParamValue::Int(10));
        }),
        0x07a1f791baa671df
    );
}

// ---- bad data

#[test]
fn damaged_values_stay_dark_rather_than_panic() {
    let grid = Grid::new(4, 4);
    for edit in [
        (|p: &mut Params| p.set("Margin", ParamValue::Int(-5))) as fn(&mut Params) -> bool,
        |p: &mut Params| p.set("Margin", ParamValue::Int(i32::MAX)),
        |p: &mut Params| p.set("RepeatCount", ParamValue::Int(i32::MAX)),
        |p: &mut Params| p.set("RepeatSkip", ParamValue::Int(i32::MAX)),
        |p: &mut Params| {
            let shape = polygon(&rectangle_corners(0.0, 0.0, 1e300, f64::NAN));
            p.set("MorphPolygonData", ParamValue::RecordList(vec![shape]))
        },
        |p: &mut Params| {
            let mut shape = polygon(&rectangle_corners(0.0, 0.0, 3.0, 3.0));
            shape.set("HeadLength", ParamValue::Int(i32::MAX));
            p.set("MorphPolygonData", ParamValue::RecordList(vec![shape]))
        },
    ] {
        let f = grid.render(&DESCRIPTOR, 200, |p| {
            edit(p);
        });
        assert!(f.is_empty());
    }
}

#[test]
fn a_zero_length_effect_renders_nothing() {
    let grid = Grid::new(4, 4);
    assert!(grid.render(&DESCRIPTOR, 0, |_| {}).is_empty());
}

#[test]
fn a_record_with_no_shape_gets_the_default_polygon() {
    let grid = Grid::new(12, 12);
    let mut r = solid(Params::defaults(&gen::MORPH_POLYGON_PARAMS), Color::BLUE);
    r.set("Polygon", ParamValue::None);
    let f = grid.render(&DESCRIPTOR, 100, |p| free_form(p, vec![r]));
    // 10 x 10 from the top left: rows 11 down to 2.
    assert_eq!(f.lit_pixels(0).len(), 100, "{}", f.picture(0));
    assert!(f.lit(0, 0, 11));
    assert!(!f.lit(0, 0, 1));
}
