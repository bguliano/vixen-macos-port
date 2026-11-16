use super::*;
use crate::{load_prop_bytes, write_package, ElementModelType, FaceComponent};
use vixen_core::Color;
fn imported(xml: &str) -> Prop {
    XModelFile::parse(xml)
        .unwrap()
        .import(0, "2026-11-11T12:00:00Z")
        .unwrap()
        .prop
}
fn points(prop: &Prop) -> Vec<(i32, f64, f64)> {
    let mut out: Vec<_> = prop
        .elements()
        .filter(|e| e.is_light_node())
        .map(|e| (e.order, e.lights()[0].x, e.lights()[0].y))
        .collect();
    out.sort_by_key(|p| p.0);
    out
}
fn node<'a>(p: &'a Prop, suffix: &str) -> &'a crate::ElementModel {
    p.elements().find(|e| e.name().ends_with(suffix)).unwrap()
}
#[test]
fn wrapper_selection_names_and_supported_types() {
    let file=XModelFile::parse(r#"<models><custommodel name="ignored"/><model name=" Same " DisplayAs="Custom" CustomModel="1"/><model name="Same" DisplayAs="Tree"/><model DisplayAs="CIRCLE"/><model name="same" DisplayAs="Custom"/></models>"#).unwrap();
    assert_eq!(
        file.candidates()
            .iter()
            .map(|c| (&*c.display_name, c.supported))
            .collect::<Vec<_>>(),
        vec![
            ("Same (1)", true),
            ("Same (2)", false),
            ("Unnamed model 3", true),
            ("same", true)
        ]
    );
    assert!(file.import(1, "").unwrap_err().detail.contains("treemodel"));
    assert!(file.import(99, "").is_err());
    for xml in [
        "<models/>",
        "<models><model DisplayAs=\"Tree\"/></models>",
        "<CustomModel a=\"1\"/>",
        "<custommodel/>",
    ] {
        assert!(XModelFile::parse(xml).is_err());
    }
}
#[test]
fn custom_scale_duplicate_order_holes_and_compressed_row_column() {
    let p = imported(
        r#"<custommodel name="Test" CustomWidth="2" CustomHeight="2" ScaleX="2.5" ScaleY="3.5" PixelSize="7" CustomModel="1,,2;3,1"/>"#,
    );
    assert_eq!(points(&p), vec![(1, 11., 12.), (2, 13., 8.), (3, 8., 12.)]);
    assert_eq!(p.physical.node_count, "3");
    assert_eq!((p.width, p.height), (800., 600.));
    assert!(p
        .elements()
        .filter(|e| e.is_light_node())
        .all(|e| e.raw_light_size() == 7));
    let p = imported(
        r#"<model DisplayAs="Custom" name="Zip" ScaleX="2.5" ScaleY="3.5" CustomModelCompressed="2,2,3;1,-1,-1;2,0,1" CustomModel="9"/>"#,
    );
    assert_eq!(points(&p), vec![(1, 5., 4.), (2, 11., 8.)]);
}
#[test]
fn compressed_fallback_and_empty_success_defaults() {
    let file=XModelFile::parse(r#"<custommodel name="Fallback" PixelSize="bad" ScaleX="NaN" ScaleY="0" CustomModelCompressed="1,2" CustomModel="1"/>"#).unwrap();
    let result = file.import(0, "").unwrap();
    assert_eq!(result.warnings.len(), 4);
    assert_eq!(points(&result.prop), vec![(1, 8., 8.)]);
    let p = imported(
        r#"<custommodel name="Empty" CustomWidth="900" CustomHeight="700" CustomModelCompressed=";;" CustomModel="1"/>"#,
    );
    assert!(points(&p).is_empty());
    assert_eq!((p.width, p.height), (914., 714.));
    assert!(
        XModelFile::parse(r#"<custommodel name="Bad" CustomModel="1|2"/>"#)
            .unwrap()
            .import(0, "")
            .is_err()
    );
}
#[test]
fn linked_submodels_descending_ranges_and_stopping_at_missing_line() {
    let p = imported(
        r#"<custommodel name="P" CustomModel="1,2,3,4"><subModel name="Rows" type="ranges" line0="4-2,4,999" line1="1-2" line3="3"/><subModel name="Skip" type="subbuffer" line0="1"/></custommodel>"#,
    );
    let rows = node(&p, "- Rows");
    assert_eq!(rows.model_type, ElementModelType::SubModel);
    assert_eq!(rows.children().len(), 2);
    let first = p.element(rows.children()[0]).unwrap();
    assert_eq!(first.name(), "P {1} - Rows - P {1} - Rows 1");
    assert_eq!(
        first
            .children()
            .iter()
            .map(|id| p.element(*id).unwrap().order)
            .collect::<Vec<_>>(),
        vec![4, 3, 2]
    );
    assert_eq!(points(&p).len(), 4);
    assert!(p.elements().all(|e| !e.name().contains("Skip")));
}
#[test]
fn face_components_colors_and_authored_state_order() {
    let p = imported(
        r##"<custommodel name="Face" CustomModel="1,2,3"><faceInfo Name="Main" Type="NodeRange" FaceOutline2="1" FaceOutline2-Color="#000000" Mouth-AI="3-2" Mouth-AI-Color="Black"/><stateInfo Name=" Mood " Type="NodeRange" s4-Name=" Late " s4-Color="#123456" s4="3,1,3" s2="2" s8="999"/><stateInfo Name="Mood" Type="SingleNode" s1="Node 1" s2="Node  2"/></custommodel>"##,
    );
    assert_eq!(
        node(&p, "- FaceOutline2").face.component,
        FaceComponent::Outlines
    );
    assert_eq!(node(&p, "- FaceOutline2").face.default_color, Color::WHITE);
    assert_eq!(node(&p, "- Mouth-AI").face.default_color, Color::BLACK);
    let states = &node(&p, "- Model").state_definitions;
    assert_eq!(states.len(), 2);
    assert_eq!(states[0].name, "Mood");
    assert_eq!(states[1].name, "Mood - 2");
    assert_eq!(
        states[0]
            .items
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>(),
        vec!["s2", "Late"]
    );
    assert_eq!(states[0].items[1].color, Color::rgb(0x12, 0x34, 0x56));
    assert_eq!(
        states[0].items[1]
            .element_ids
            .iter()
            .map(|id| p.element(*id).unwrap().order)
            .collect::<Vec<_>>(),
        vec![3, 1]
    );
    assert_eq!(states[1].items.len(), 1);
    let legacy = node(&p, "- Mood ");
    assert_eq!(
        p.element(legacy.children()[0]).unwrap().name(),
        "Face {1} - Mood - S4 - Late"
    );
    assert_eq!(legacy.children().len(), 3);
}
#[test]
fn circle_geometry_direction_layer_order_and_generated_groups() {
    let p = imported(
        r#"<circlemodel name="Ring" LayerSizes="4,4" InsideOut="1" StartSide="B" Dir="R" centerPercent="50" ScaleX="1" ScaleY="1" PixelCount="8"/>"#,
    );
    assert_eq!(
        points(&p),
        vec![
            (1, 9., 10.),
            (2, 10., 9.),
            (3, 9., 8.),
            (4, 8., 9.),
            (5, 9., 11.),
            (6, 11., 9.),
            (7, 9., 7.),
            (8, 7., 9.)
        ]
    );
    assert_eq!(node(&p, "- Circles").children().len(), 2);
    let p = imported(
        r#"<circlemodel name="Ring" LayerSizes="2,4" InsideOut="0" StartSide="top" Dir="left" centerPercent="50" ScaleX="1" ScaleY="1"/>"#,
    );
    assert_eq!(
        &points(&p)[..4],
        &[(1, 9., 7.), (2, 11., 9.), (3, 9., 11.), (4, 7., 9.)]
    );
    assert_eq!(node(&p, "- Circle 1").children().len(), 4);
    assert_eq!(node(&p, "- Circle 2").children().len(), 2);
}
#[test]
fn imported_circles_suppress_only_matching_ordered_positive_ranges() {
    for (ranges, expected) in [
        ("line0=\"5-8\" line1=\"0-4\"", 1),
        ("line0=\"8-5\" line1=\"1-4\"", 2),
        ("line0=\"5-8,8\" line1=\"1-4\"", 2),
    ] {
        let p = imported(&format!(
            r#"<circlemodel name="R" LayerSizes="4,4" InsideOut="1" StartSide="B" Dir="R"><subModel name="Circles" type="ranges" {ranges}/></circlemodel>"#
        ));
        assert_eq!(
            p.root()
                .children()
                .iter()
                .filter(|id| p.element(**id).unwrap().name().contains("Circles"))
                .count(),
            expected
        );
    }
}
#[test]
fn invalid_circle_and_bounded_inputs_return_errors() {
    for attrs in [
        "LayerSizes=\"4,0\" InsideOut=\"1\" StartSide=\"B\" Dir=\"R\"",
        "LayerSizes=\"4\" InsideOut=\"true\" StartSide=\"B\" Dir=\"R\"",
        "LayerSizes=\"4\" InsideOut=\"1\" StartSide=\"B\" Dir=\"R\" PixelCount=\"5\"",
        "LayerSizes=\"2147483647\" InsideOut=\"1\" StartSide=\"B\" Dir=\"R\"",
    ] {
        assert!(
            XModelFile::parse(&format!("<circlemodel name=\"Bad\" {attrs}/> "))
                .unwrap()
                .import(0, "")
                .is_err()
        );
    }
    assert!(
        XModelFile::parse(r#"<custommodel name="Big" ScaleX="1e200" CustomModel="1,2"/>"#)
            .unwrap()
            .import(0, "")
            .is_err()
    );
    assert!(XModelFile::parse(
        r#"<custommodel name="Big" PixelSize="2147483647" CustomModel="1"/>"#
    )
    .unwrap()
    .import(0, "")
    .is_err());
    let p = imported(
        r#"<custommodel name="Huge range" CustomModel="1,2"><subModel name="All" type="ranges" line0="1-2147483647"/></custommodel>"#,
    );
    assert_eq!(node(&p, "- All").children().len(), 2);
}
#[test]
fn html_color_forms_and_invalid_values() {
    assert_eq!(colors::parse("#abc").unwrap(), Color::rgb(170, 187, 204));
    assert_eq!(
        colors::parse("AliceBlue").unwrap(),
        Color::rgb(240, 248, 255)
    );
    assert_eq!(
        colors::parse("LightGrey").unwrap(),
        Color::rgb(211, 211, 211)
    );
    assert!(colors::parse("no-such-color").is_err());
}
#[test]
fn xml_namespaces_keep_unqualified_attributes_only() {
    let p = imported(
        r#"<custommodel xmlns="urn:models" xmlns:a="urn:attrs" name="P" a:PixelSize="88" CustomModel="1"/>"#,
    );
    assert_eq!(points(&p).len(), 1);
    assert_eq!(
        p.elements()
            .find(|e| e.is_light_node())
            .unwrap()
            .raw_light_size(),
        1
    );
}
#[test]
fn reference_models_and_persistent_graph_roundtrip() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../upstream/Vixen/docs/references");
    let paths = std::fs::read_dir(root.join("circlemodel"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .chain(std::iter::once(root.join("GE Stardust 1500.xmodel")));
    for path in paths.filter(|p| p.extension().is_some_and(|e| e == "xmodel")) {
        let file = XModelFile::open(&path).unwrap();
        for c in file.candidates().iter().filter(|c| c.supported) {
            let p = file.import(c.index, "2026-11-11T12:00:00Z").unwrap().prop;
            let count = p.physical.node_count.parse::<usize>().unwrap();
            assert!(count > 0, "{}", path.display());
            assert_eq!(points(&p).len(), count);
            let bytes = write_package(&p).unwrap();
            let loaded = load_prop_bytes(&bytes, "").unwrap().prop;
            assert_eq!(points(&loaded), points(&p));
            assert_eq!(loaded.root().id, p.root().id);
            for e in p.elements() {
                let other = loaded.element(e.id).unwrap();
                assert_eq!(other.name(), e.name());
                assert_eq!(other.children(), e.children());
                assert_eq!(other.parents().iter().collect::<std::collections::HashSet<_>>(), e.parents().iter().collect::<std::collections::HashSet<_>>());
                assert_eq!(other.state_definitions, e.state_definitions);
                assert_eq!(other.face, e.face);
            }
        }
    }
}
#[test]
fn imported_editor_starts_unsaved_select_and_keeps_authored_state() {
    let prop = imported(
        r##"<custommodel name="New" CustomModel="1,2"><stateInfo Name="On" Type="NodeRange" s1="1-2" s1-Color="#ff0000"/></custommodel>"##,
    );
    let editor = crate::PropEditor::imported_document(prop, crate::PropEditorSettings::default());
    assert!(editor.is_dirty());
    assert!(editor.file_path().is_none());
    assert_eq!(editor.mode(), crate::CanvasMode::Select);
    assert!(editor.undo_name().is_none());
    let bytes = write_package(editor.prop()).unwrap();
    let loaded = load_prop_bytes(&bytes, "").unwrap().prop;
    assert_eq!(
        node(&loaded, "- Model").state_definitions,
        node(editor.prop(), "- Model").state_definitions
    );
}

#[test]
fn local_utf16_xmodel_reads_with_xml_encoding() {
    let path = std::env::temp_dir().join(format!("vixen-xmodel-{}.xmodel", uuid::Uuid::new_v4()));
    let xml = r#"<?xml version="1.0" encoding="utf-16"?><custommodel name="Étoile" CustomModel="1"/>"#;
    let mut bytes = vec![0xff,0xfe];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    std::fs::write(&path, bytes).unwrap();
    let result = XModelFile::open(&path);
    std::fs::remove_file(path).unwrap();
    assert_eq!(result.unwrap().import(0, "").unwrap().prop.name(), "Étoile {1}");
}
