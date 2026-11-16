//! Actual panel Add/selection commits, render output and the sequence XML boundary.
use super::*;
use crate::sequence::SessionEnvironment;
use fx_fixture::fixture_base::test_support::fixtures;
use fx_fixture::spec::{FixtureChannel, FixtureFunction, FixtureFunctionType, FixtureIndexItem, FixtureSpecification, FuncId};
use std::sync::Arc;
use vixen_core::{ElementTree, StateValue, TimeSpan};
use vixen_io::ModuleStore;

struct Fixture {
    session: SequenceSession,
    env: SessionEnvironment,
    target: Uuid,
    root: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("vixen-fixture-authoring-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let functions = vec![
            FixtureFunction { name: "Pan".into(), identity: FuncId::Pan, label: "P".into(), timeline_color: Color::RED, ..Default::default() },
            FixtureFunction {
                name: "Gobo".into(), function_type: FixtureFunctionType::Indexed, identity: FuncId::Gobo,
                index_data: vec![FixtureIndexItem { name: "Star".into(), start_value: 42, end_value: 42, ..FixtureIndexItem::index() }],
                timeline_color: Color::BLUE, ..Default::default()
            },
            FixtureFunction {
                name: "Wheel".into(), function_type: FixtureFunctionType::ColorWheel,
                color_wheel_data: vec![FixtureIndexItem { name: "Green".into(), start_value: 63, end_value: 63, ..FixtureIndexItem::color_wheel() }],
                ..Default::default()
            },
            FixtureFunction { name: "RGB".into(), function_type: FixtureFunctionType::RgbColor, ..Default::default() },
        ];
        let channels = functions.iter().enumerate().map(|(index, f)| FixtureChannel {
            name: f.name.clone(), function: f.name.clone(), channel_number: index as i32 + 1,
        }).collect();
        let spec = FixtureSpecification { functions, channels, ..Default::default() };
        let mut tree = ElementTree::new();
        let node = fixtures(&mut tree, "Heads", 2, &spec);
        let target = tree.node(node).id;
        let env = SessionEnvironment {
            video_cache: None, tree: Arc::new(tree), store: Arc::new(ModuleStore::new()), module_data_root: None,
            frame_ms: 50, media_dir: root.clone(),
        };
        Self { session: SequenceSession::new_untitled(&env), env, target, root }
    }

    fn add(&mut self) -> EffectId {
        self.session.add_new_effect(fx_fixture::fixture::DESCRIPTOR.type_id, self.target, "Heads", TimeSpan::ZERO, TimeSpan::from_ms_i(500)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); }
}

fn row<'a>(panel: &'a PanelModel, name: &str) -> &'a RowModel {
    panel.categories.iter().flat_map(|c| &c.rows).find(|r| r.key.param == name).unwrap()
}

fn select_function(session: &mut SequenceSession, edits: &mut PanelEdits, id: EffectId, name: &str) {
    let panel = panel_model(session, &[id], 100);
    let RowControl::Choice { items, selected } = &row(&panel, "SelectedFunction").control else { panic!() };
    let index = items.iter().position(|item| item.label == name).unwrap() as u32;
    if *selected != Some(index) {
        assert!(matches!(commit(session, edits, &[id], &RowKey::top("SelectedFunction"), ShownValue::Choice(index), CommitOptions::default()), CommitOutcome::Committed { .. }));
    }
}

fn assert_output(session: &mut SequenceSession, id: EffectId, name: &str) {
    session.wait_for_renders();
    let entry = session.effect(id).unwrap();
    assert!(entry.is_rendered());
    let intents = entry.intents.as_ref().unwrap();
    assert_eq!(intents.map.len(), 2, "both supported fixture leaves receive output");
    for list in intents.map.values() {
        match (name, list[0].intent.state_at(TimeSpan::ZERO)) {
            ("Pan", StateValue::Range(v)) => {
                assert_eq!(v.value, 0.0);
                assert_eq!(v.tag.as_ref(), "Pan");
                assert_eq!(v.tag_type, vixen_core::value::FunctionIdentity::Pan);
            }
            ("Gobo", StateValue::Command(v)) => assert_eq!(v.as_u8(), Some(42)),
            ("Wheel", StateValue::Command(v)) => assert_eq!(v.as_u8(), Some(63)),
            ("RGB", StateValue::Lighting(v)) => { assert_eq!(v.color, Color::WHITE); assert_eq!(v.intensity, 1.0); }
            (_, other) => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn add_uses_the_picker_and_renders_all_constructor_types_then_undo_restores_the_list() {
    let mut f = Fixture::new();
    let mut edits = PanelEdits::default();
    for name in ["Pan", "Gobo", "Wheel", "RGB"] {
        let id = f.add();
        select_function(&mut f.session, &mut edits, id, name);
        assert!(matches!(list_add(&mut f.session, &mut edits, &[id], "FunctionData"), CommitOutcome::Committed { .. }));
        let panel = panel_model(&f.session, &[id], 100);
        let RowControl::RecordList { items, .. } = &row(&panel, "FunctionData").control else { panic!() };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].header, name);
        if name == "Gobo" {
            assert_eq!(items[0].header_color, Some(Color::BLUE));
            assert!(items[0].rows.iter().any(|r| r.key.sub_param.as_deref() == Some("IndexValue") && matches!(r.control, RowControl::Choice { selected: Some(0), .. })));
        }
        assert_output(&mut f.session, id, name);
        f.session.undo(1);
        assert!(f.session.effect(id).unwrap().node.params.records("FunctionData").is_empty());
        f.session.redo(1);
        assert_output(&mut f.session, id, name);
    }
}

#[test]
fn save_reload_preserves_authored_rows_and_hydrates_a_usable_transient_selection() {
    let mut f = Fixture::new();
    let id = f.add();
    let mut edits = PanelEdits::default();
    select_function(&mut f.session, &mut edits, id, "Gobo");
    list_add(&mut f.session, &mut edits, &[id], "FunctionData");
    assert_output(&mut f.session, id, "Gobo");
    let before = f.session.effect(id).unwrap().node.params.records("FunctionData").to_vec();
    let path = f.root.join("authored.tim");
    f.session.save(Some(&path)).unwrap();
    let mut loaded = SequenceSession::open(&path, &f.env).unwrap();
    let id = loaded.effects().next().unwrap().id;
    assert_eq!(loaded.effect(id).unwrap().node.params.records("FunctionData"), before);
    let panel = panel_model(&loaded, &[id], 100);
    let RowControl::Choice { items, selected } = &row(&panel, "SelectedFunction").control else { panic!() };
    assert_eq!(*selected, Some(0));
    assert_eq!(items[0].label, "Pan");
    assert!(!loaded.is_modified());
    assert!(!loaded.undo_stack().can_undo());
    assert_output(&mut loaded, id, "Gobo");
    assert!(matches!(list_add(&mut loaded, &mut PanelEdits::default(), &[id], "FunctionData"), CommitOutcome::Committed { .. }));
    assert_eq!(loaded.effect(id).unwrap().node.params.records("FunctionData")[1].string("FunctionName"), "Pan");
}

#[test]
fn invalid_or_absent_capabilities_cannot_add_an_unnamed_row() {
    let mut f = Fixture::new();
    let id = f.add();
    f.session.set_param(id, "SelectedFunction", ParamValue::Str("Missing".into()));
    let history = f.session.undo_stack().undo_descriptions().len();
    assert_eq!(list_add(&mut f.session, &mut PanelEdits::default(), &[id], "FunctionData"), CommitOutcome::NoChange);
    assert!(f.session.effect(id).unwrap().node.params.records("FunctionData").is_empty());
    assert_eq!(f.session.undo_stack().undo_descriptions().len(), history);
    let mut tree = ElementTree::new();
    let plain = tree.add_string("Arch", 1);
    let target = tree.node(plain).id;
    let env = SessionEnvironment { tree: Arc::new(tree), ..f.env.clone() };
    let mut session = SequenceSession::new_untitled(&env);
    let id = session.add_new_effect(fx_fixture::fixture::DESCRIPTOR.type_id, target, "Arch", TimeSpan::ZERO, TimeSpan::from_ms_i(500)).unwrap();
    assert_eq!(list_add(&mut session, &mut PanelEdits::default(), &[id], "FunctionData"), CommitOutcome::NoChange);
}

#[test]
fn adding_after_profile_changes_updates_choices_and_curve_visibility_in_the_same_undo_step() {
    let mut f = Fixture::new();
    let id = f.add();
    // Save a valid old capability snapshot and authored row, then change only
    // the profile definition used by the next session.
    let mut edits = PanelEdits::default();
    select_function(&mut f.session, &mut edits, id, "Gobo");
    list_add(&mut f.session, &mut edits, &[id], "FunctionData");
    let path = f.root.join("old-profile.tim");
    f.session.save(Some(&path)).unwrap();
    let mut tree = (*f.env.tree).clone();
    let nodes: Vec<_> = tree.all_nodes().map(|(id, _)| id).collect();
    for node in nodes {
        for property in &mut tree.node_mut(node).properties {
            if let vixen_core::element::PropertyData::Unknown { raw_xml } = &mut property.data {
                *raw_xml = raw_xml.replace("<Name>Star</Name>", "<Name>New Curve</Name>").replace("<UseCurve>false</UseCurve>", "<UseCurve>true</UseCurve>");
            }
        }
    }
    let env = SessionEnvironment { tree: Arc::new(tree), ..f.env.clone() };
    let mut loaded = SequenceSession::open(&path, &env).unwrap();
    let id = loaded.effects().next().unwrap().id;
    let mut edits = PanelEdits::default();
    select_function(&mut loaded, &mut edits, id, "Gobo");
    let before = loaded.effect(id).unwrap().node.params.clone();
    list_add(&mut loaded, &mut edits, &[id], "FunctionData");
    let after = loaded.effect(id).unwrap().node.params.clone();
    assert_eq!(after.records("FunctionData")[0], before.records("FunctionData")[0], "refresh leaves authored rows unchanged");
    assert_eq!(after.records("FunctionData")[1].string("IndexValue"), "New Curve");
    let panel = panel_model(&loaded, &[id], 100);
    let RowControl::RecordList { items, .. } = &row(&panel, "FunctionData").control else { panic!() };
    let choice = items[1].rows.iter().find(|r| r.key.sub_param.as_deref() == Some("IndexValue")).unwrap();
    let RowControl::Choice { items: choices, selected } = &choice.control else { panic!() };
    assert_eq!(choices.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), ["New Curve"]);
    assert_eq!(*selected, Some(0));
    assert!(items[1].rows.iter().any(|r| r.key.sub_param.as_deref() == Some("Range")), "current UseCurve reveals the range editor");
    loaded.undo(1);
    assert_eq!(loaded.effect(id).unwrap().node.params, before);
    loaded.redo(1);
    assert_eq!(loaded.effect(id).unwrap().node.params, after);
}
