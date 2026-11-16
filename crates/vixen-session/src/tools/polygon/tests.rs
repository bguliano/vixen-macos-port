use super::*;
#[test]
fn canvas_mapping_grid_fraction_validation_and_finish() {
    let shape = Shape::new(
        Kind::Polygon,
        vec![
            Point::new(0., 0.),
            Point::new(99., 0.),
            Point::new(99., 49.),
            Point::new(0., 49.),
        ],
        0.,
        false,
    );
    let id = shape.id;
    let mut e = Editor::new(
        vec![shape],
        MorphMode::Pattern,
        (100., 50.),
        (100., 50.),
        (1000., 1000.),
    );
    assert_eq!(e.selected, Some(id));
    assert_eq!(e.canvas_width, 700.);
    assert_eq!(e.canvas_height, 350.);
    e.set_point(0, "X", "3.5").unwrap();
    e.set_point(0, "Y", "2.5").unwrap();
    assert!(e.set_point(0, "X", "NaN").is_err());
    assert!(e.set_point(0, "Y", "51").is_err());
    let out = e.finish();
    assert_eq!(out[0].id, id);
    assert_eq!(out[0].points[0], Point::new(2., 48.));
    assert_eq!(e.finish(), out);
}
#[test]
fn time_empty_open_clone_add_sort_and_delete() {
    let mut e = Editor::new(
        vec![],
        MorphMode::TimeBased,
        (20., 20.),
        (20., 20.),
        (1000., 1000.),
    );
    assert_eq!(e.frames.len(), 1);
    e.set_mode(Mode::Ellipse);
    e.pointer_down(Point::new(300., 300.));
    assert_eq!(e.mode, Mode::Select);
    let first = e.shapes[0].id;
    e.add_frame(250.);
    assert_ne!(e.shapes[1].id, first);
    assert_eq!(e.frames[1].time, 0.1);
    e.move_frame(0.);
    e.delete();
    assert_eq!(e.finish().len(), 1);
    e.delete_frame();
    assert_eq!(e.frames.len(), 1);
}
#[test]
fn polygon_close_minimum_and_line_repeated_drawing() {
    let mut e = Editor::new(
        vec![],
        MorphMode::FreeForm,
        (10., 10.),
        (10., 10.),
        (1000., 1000.),
    );
    e.set_mode(Mode::Polygon);
    for p in [(20., 20.), (21., 21.), (300., 300.), (20., 20.)] {
        e.pointer_down(Point::new(p.0, p.1));
    }
    assert_eq!(e.shapes.len(), 1);
    assert_eq!(e.mode, Mode::Polygon);
    e.set_mode(Mode::Line);
    for p in [(50., 100.), (150., 200.), (250., 300.), (350., 400.)] {
        e.pointer_down(Point::new(p.0, p.1));
    }
    assert_eq!(e.shapes.len(), 3);
    assert_eq!(e.mode, Mode::Line);
}
#[test]
fn copy_new_id_and_ellipse_grid_is_read_only() {
    let shape = Shape::new(
        Kind::Ellipse,
        vec![
            Point::new(2., 2.),
            Point::new(7., 2.),
            Point::new(7., 7.),
            Point::new(2., 7.),
        ],
        0.,
        false,
    );
    let mut e = Editor::new(
        vec![shape],
        MorphMode::FreeForm,
        (10., 10.),
        (10., 10.),
        (1000., 1000.),
    );
    let old = e.finish();
    e.set_point(0, "X", "5").unwrap();
    assert_eq!(e.finish(), old);
    let clip = e.copy().unwrap();
    assert!(e.paste(&clip));
    assert_ne!(e.shapes[0].id, e.shapes[1].id);
    assert!(!e.paste("{}"));
}
#[test]
fn morph_apply_preserves_id_settings_and_unknown_members() {
    use vixen_effects::framework::ParamValue;
    let mut p = morph_defaults();
    let mut rows = p.records("MorphPolygonData").to_vec();
    if rows.is_empty() {
        let vixen_effects::framework::ParamKind::RecordList(desc) = p
            .desc
            .iter()
            .find(|d| d.name == "MorphPolygonData")
            .unwrap()
            .kind
        else {
            panic!()
        };
        rows.push(Params::defaults(desc.params));
    }
    rows[0].set("HeadLength", ParamValue::Int(23));
    rows[0]
        .unknown_members
        .push(("future".into(), "xml".into()));
    p.set("MorphPolygonData", ParamValue::RecordList(rows));
    let shape = super::morph::apply_shapes(&p, &[]);
    assert!(shape.records("MorphPolygonData").is_empty());
    let row = &p.records("MorphPolygonData")[0];
    let g = row.record("Polygon").unwrap();
    let id = match g.get("ID") {
        ParamValue::Guid(id) => *id,
        _ => panic!(),
    };
    let mut s = Shape::new(
        Kind::Polygon,
        vec![
            Point::new(1., 1.),
            Point::new(9., 0.),
            Point::new(9., 9.),
            Point::new(0., 9.),
        ],
        0.,
        false,
    );
    s.id = id;
    s.label = "Keep me".into();
    s.original = Some(g.clone());
    let result = super::morph::apply_shapes(&p, &[s]);
    assert_eq!(result.records("MorphPolygonData")[0].int("HeadLength"), 23);
    assert_eq!(
        result.records("MorphPolygonData")[0].unknown_members,
        p.records("MorphPolygonData")[0].unknown_members
    );
    assert_eq!(
        result.records("MorphPolygonData")[0]
            .record("Polygon")
            .unwrap()
            .get("ID"),
        &ParamValue::Guid(id)
    );
    assert_eq!(
        p.records("MorphPolygonData")[0]
            .record("Polygon")
            .unwrap()
            .records("Points")[0]
            .double("X"),
        0.
    );
}
#[test]
fn sequence_cancel_commit_deleted_targets_and_one_undo() {
    use crate::sequence::{SequenceSession, SessionEnvironment};
    use std::sync::Arc;
    use vixen_core::{ElementTree, TimeSpan};
    use vixen_io::ModuleStore;
    let mut tree = ElementTree::new();
    let target = tree.add_string("Test", 10);
    let target = tree.node(target).id;
    let env = SessionEnvironment { video_cache: None,
        tree: Arc::new(tree),
        store: Arc::new(ModuleStore::new()),
        module_data_root: None,
        frame_ms: 50,
        media_dir: std::env::temp_dir(),
    };
    let mut session = SequenceSession::new_untitled(&env);
    let d = vixen_effect_registry::by_data_type("MorphData").unwrap();
    let id = session
        .add_new_effect(
            d.type_id,
            target,
            "Test",
            TimeSpan::ZERO,
            TimeSpan::from_ms(1000.),
        )
        .unwrap();
    let original = session.effect(id).unwrap().node.params.clone();
    let mut draft = open_from_sequence(&session, &[id], (1000., 1000.)).unwrap();
    assert_eq!(session.effect(id).unwrap().node.params, original);
    assert!(
        apply_to_sequence(&mut session, &[id], &draft).is_none(),
        "unchanged OK must be no-op"
    );
    draft.set_label("Updated".into());
    assert_eq!(
        session.effect(id).unwrap().node.params,
        original,
        "draft/cancel do not mutate source"
    );
    assert!(apply_to_sequence(&mut session, &[id, id], &draft)
        .unwrap()
        .starts_with("1 Effect"));
    assert_ne!(session.effect(id).unwrap().node.params, original);
    session.undo(1);
    assert_eq!(session.effect(id).unwrap().node.params, original);
    session.redo(1);
    assert_ne!(session.effect(id).unwrap().node.params, original);
    session.remove_effects(&[id]);
    assert!(apply_to_sequence(&mut session, &[id], &draft).is_none());
    assert!(open_from_sequence(&session, &[id], (1000., 1000.)).is_err());
}

#[test]
fn point_drag_is_ghost_until_release_and_polygon_delete_keeps_triangle() {
    let shape = Shape::new(
        Kind::Polygon,
        vec![
            Point::new(2., 2.),
            Point::new(17., 2.),
            Point::new(17., 17.),
            Point::new(2., 17.),
        ],
        0.,
        false,
    );
    let mut e = Editor::new(
        vec![shape],
        MorphMode::Pattern,
        (20., 20.),
        (20., 20.),
        (1000., 1000.),
    );
    e.select_shape(None);
    let original = e.shapes[0].points[0];
    e.pointer_down(original);
    e.pointer_move(Point::new(original.x + 20., original.y + 30.), true, false);
    assert_eq!(e.shapes[0].points[0], original);
    e.pointer_up(Point::new(original.x + 20., original.y + 30.), false);
    assert_ne!(e.shapes[0].points[0], original);
    e.delete();
    assert_eq!(e.shapes[0].points.len(), 3);
    e.select_shape(Some(e.shapes[0].id));
    e.delete();
    assert_eq!(e.shapes[0].points.len(), 3);
}
#[test]
fn transforms_reject_outside_and_rotated_ellipse_round_trips_without_rounding() {
    let mut shape = Shape::new(
        Kind::Ellipse,
        vec![
            Point::new(5.25, 7.),
            Point::new(14.75, 7.),
            Point::new(14.75, 13.),
            Point::new(5.25, 13.),
        ],
        0.,
        false,
    );
    shape.angle = 30.;
    for p in &mut shape.points {
        *p = rotate(*p, shape.center, shape.angle);
    }
    let original = shape.clone();
    let mut e = Editor::new(
        vec![shape],
        MorphMode::FreeForm,
        (20., 20.),
        (20., 20.),
        (1000., 800.),
    );
    let before = e.shapes.clone();
    assert!(!e.translate(Point::new(1000., 0.)));
    assert_eq!(e.shapes, before);
    assert!(e.rotate_selected(15., e.shapes[0].center));
    assert_eq!(e.shapes[0].angle, 45.);
    e.toggle_start();
    assert_eq!(e.shapes[0].start_side, 1);
    e.toggle_start();
    e.toggle_start();
    e.toggle_start();
    assert_eq!(e.shapes[0].start_side, 0);
    let e = Editor::new(
        vec![original.clone()],
        MorphMode::FreeForm,
        (20., 20.),
        (20., 20.),
        (1000., 800.),
    );
    let out = e.finish();
    assert_eq!(out[0].id, original.id);
    for (p, q) in out[0].points.iter().zip(original.points) {
        assert!((p.x - q.x).abs() < 1e-10 && (p.y - q.y).abs() < 1e-10);
    }
}

#[test]
fn rotation_handle_above_canvas_is_hit_and_invalid_rotation_cancels_selection() {
    let shape = Shape::new(
        Kind::Polygon,
        vec![
            Point::new(0., 0.),
            Point::new(19., 0.),
            Point::new(19., 19.),
            Point::new(0., 19.),
        ],
        0.,
        false,
    );
    let mut e = Editor::new(
        vec![shape],
        MorphMode::Pattern,
        (20., 20.),
        (20., 20.),
        (1000., 1000.),
    );
    let handle = e.adorner().unwrap().3[9];
    assert!(handle.y < 0.);
    e.pointer_down(handle);
    e.pointer_move(Point::new(handle.x + 20., handle.y), true, false);
    assert!(e.selected.is_none());
}

#[test]
fn line_markers_distinguish_drawn_converted_and_toggled_starts() {
    let mut e = Editor::new(
        vec![],
        MorphMode::FreeForm,
        (20., 20.),
        (20., 20.),
        (1000., 800.),
    );
    e.set_mode(Mode::Line);
    e.pointer_down(Point::new(100., 100.));
    e.pointer_down(Point::new(300., 300.));
    assert!(e.shapes[0].start_lime);
    e.set_mode(Mode::Select);
    e.select_shape(Some(e.shapes[0].id));
    let clip = e.copy().unwrap();
    assert!(e.paste(&clip));
    assert!(!e.selected_shape().unwrap().start_lime);
    let id = e.selected.unwrap();
    e.toggle_start();
    assert!(e.shapes.iter().find(|s| s.id == id).unwrap().start_lime);
    let mut e = Editor::new(
        vec![],
        MorphMode::Pattern,
        (20., 20.),
        (20., 20.),
        (1000., 800.),
    );
    e.convert(Kind::Line);
    assert!(!e.shapes[0].start_lime);
    e.toggle_start();
    assert!(e.shapes[0].start_lime);
}

#[test]
fn frames_clone_snapshot_even_when_deselected_and_match_pixel_time_rounding() {
    let s = Shape::new(
        Kind::Line,
        vec![Point::new(2., 2.), Point::new(17., 17.)],
        0.103,
        true,
    );
    let first = s.id;
    let mut e = Editor::new(
        vec![s],
        MorphMode::TimeBased,
        (20., 20.),
        (20., 20.),
        (1000., 800.),
    );
    e.select_shape(None);
    e.add_frame(250.);
    assert_eq!(e.frames.len(), 2);
    assert_eq!(e.frames[1].time, 0.2);
    assert!(e.frames[1].shape.is_some());
    assert_ne!(e.frames[1].shape, Some(first));
    e.move_frame_at(101.9, 250.);
    assert_eq!(e.frames[e.frame].time, 97. / 240.);
    e.move_frame_at(-5., 250.);
    assert_eq!(e.frames[e.frame].time, 0.);
    e.move_frame_at(1000., 250.);
    assert_eq!(e.frames[e.frame].time, 1.);
}
#[test]
fn every_resize_handle_keeps_opposite_edge_fixed_and_shape_id() {
    let original = Shape::new(
        Kind::Polygon,
        vec![
            Point::new(2., 2.),
            Point::new(17., 2.),
            Point::new(17., 17.),
            Point::new(2., 17.),
        ],
        0.,
        false,
    );
    for (handle, (dx, dy)) in [
        (10., 10.),
        (0., 10.),
        (-10., 10.),
        (-10., 0.),
        (-10., -10.),
        (0., -10.),
        (10., -10.),
        (10., 0.),
    ]
    .into_iter()
    .enumerate()
    {
        let mut e = Editor::new(
            vec![original.clone()],
            MorphMode::Pattern,
            (20., 20.),
            (20., 20.),
            (1000., 800.),
        );
        let (a, b) = bounds(&e.shapes[0].points);
        let start = e.adorner().unwrap().3[handle];
        let end = Point::new(start.x + dx, start.y + dy);
        e.pointer_down(start);
        e.pointer_move(end, true, false);
        e.pointer_up(end, false);
        let (lo, hi) = bounds(&e.shapes[0].points);
        assert_eq!(e.shapes[0].id, original.id);
        assert!(
            lo.x >= a.x - 1e-10
                && lo.y >= a.y - 1e-10
                && hi.x <= b.x + 1e-10
                && hi.y <= b.y + 1e-10
        );
        if dx != 0. {
            assert!((hi.x - lo.x - (b.x - a.x - 10.)).abs() < 1e-9);
        }
        if dy != 0. {
            assert!((hi.y - lo.y - (b.y - a.y - 10.)).abs() < 1e-9);
        }
    }
}

#[test]
fn partial_shape_names_follow_selection_restart_completion_and_cancel() {
    let mut e = Editor::new(
        vec![],
        MorphMode::FreeForm,
        (10., 10.),
        (10., 10.),
        (1000., 1000.),
    );
    e.set_mode(Mode::Polygon);
    assert_eq!(e.label(), None); // Upstream deselects the first polygon draft after creating it.
    for (x, y) in [(20., 20.), (150., 20.), (150., 150.), (20., 20.)] {
        e.pointer_down(Point::new(x, y));
    }
    assert_eq!(e.label(), Some(""));
    e.set_label("Second polygon".into());
    e.pointer_down(Point::new(200., 200.));
    assert_eq!(e.label(), Some("Second polygon"));
    // Leaving the canvas discards the entire partial shape, including its name.
    e.pointer_leave();
    assert_eq!(e.label(), Some(""));
    assert!(e.partial.is_empty());
    e.set_label("Named polygon".into());
    for (x, y) in [(200., 200.), (350., 200.), (350., 350.), (200., 200.)] {
        e.pointer_down(Point::new(x, y));
    }
    assert_eq!(e.shapes[1].label, "Named polygon");
    assert_eq!(e.label(), Some(""));
    e.set_label("Discard this draft".into());
    e.set_mode(Mode::Select);
    assert_eq!(e.label(), None);
    assert_eq!(e.finish().len(), 2);
    e.set_mode(Mode::Line);
    assert_eq!(e.label(), Some("")); // Unlike polygon mode, the first line stays selected.
    e.set_label("Named line".into());
    e.pointer_down(Point::new(100., 500.));
    e.pointer_down(Point::new(500., 500.));
    assert_eq!(e.shapes[2].label, "Named line");
    assert_eq!(e.label(), Some(""));
    let id = e.shapes[2].id;
    e.set_mode(Mode::Select);
    e.select_shape(Some(id));
    e.set_label("Renamed line".into());
    assert_eq!(e.finish()[2].label, "Renamed line");
    assert_eq!(e.finish()[2].id, id);
}

#[test]
fn time_frame_partial_line_name_survives_selection_and_serialization() {
    let mut e = Editor::new(
        vec![],
        MorphMode::TimeBased,
        (10., 10.),
        (10., 10.),
        (1000., 1000.),
    );
    e.set_mode(Mode::Line);
    e.set_label("Timed line".into());
    e.pointer_down(Point::new(100., 100.));
    e.pointer_down(Point::new(500., 500.));
    assert_eq!(e.mode, Mode::Select);
    assert_eq!(e.label(), Some("Timed line"));
    assert_eq!(e.finish()[0].label, "Timed line");
    assert_eq!(e.frames[0].shape, e.selected);
}
