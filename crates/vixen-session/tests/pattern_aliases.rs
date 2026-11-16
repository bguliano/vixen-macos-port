//! Pattern's editable Brick properties are getters over saved Weave members.
//! Exercise actual sequence/default loading and the panel, without any window.
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::{Color, ColorGradient, ElementTree, TimeSpan};
use vixen_effects::framework::{EffectDescriptor, ParamValue, Params};
use vixen_io::{DataPaths, EffectNode, ModuleStore};
use vixen_session::effect_editor::panel::{panel_model, RowControl, DEFAULT_MULTI_EDIT_LIMIT};
use vixen_session::sequence::{EffectId, SequenceSession, SessionEnvironment};

struct Fixture {
    paths: DataPaths,
    env: SessionEnvironment,
    target: Uuid,
}
impl Fixture {
    fn new() -> Self {
        let paths = DataPaths::new(std::env::temp_dir().join(format!("vixen-pattern-alias-{}", Uuid::new_v4())));
        paths.ensure_directories().unwrap();
        let mut tree = ElementTree::new();
        let node = vixen_effects::testing::grid(&mut tree, "Matrix", 8, 12);
        let target = tree.node(node).id;
        let env = SessionEnvironment {
            video_cache: None, tree: Arc::new(tree), store: Arc::new(ModuleStore::new()),
            module_data_root: None, frame_ms: 50, media_dir: paths.media_dir(),
        };
        Self { paths, env, target }
    }
    fn path(&self) -> PathBuf { self.paths.sequence_dir().join("Pattern.tim") }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(self.paths.root()); }
}
fn descriptor() -> &'static EffectDescriptor {
    vixen_session::effect_editor::panel::descriptor_named("Pattern").unwrap()
}
fn saved_values() -> Params {
    let mut p = descriptor().new_params();
    p.set("PatternType", ParamValue::Enum("Brick".into()));
    p.set("WeaveThickness", ParamValue::Int(37));
    p.set("HorizontalColors", ParamValue::GradientList(vec![ColorGradient::solid(Color::BLUE)]));
    // A load must not replay the AdvancedSizing setter and overwrite these fields.
    p.set("AdvancedSizing", ParamValue::Bool(true));
    p.set("WeaveHorizontalThickness", ParamValue::Int(21));
    p.set("WeaveVerticalThickness", ParamValue::Int(63));
    p
}
fn assert_panel(session: &SequenceSession, id: EffectId) {
    let model = panel_model(session, &[id], DEFAULT_MULTI_EDIT_LIMIT);
    let rows: Vec<_> = model.categories.iter().flat_map(|c| &c.rows).collect();
    let height = rows.iter().find(|r| r.key.param == "BrickHeight").unwrap();
    assert!(matches!(height.control, RowControl::Slider { value: 37.0, .. }), "{:?}", height.control);
    let colors = rows.iter().find(|r| r.key.param == "BrickColors").unwrap();
    let RowControl::GradientList { items, .. } = &colors.control else { panic!("{:?}", colors.control) };
    assert_eq!(items, &[ColorGradient::solid(Color::BLUE)]);
    let p = &session.effect(id).unwrap().node.params;
    assert_eq!(p.int("WeaveHorizontalThickness"), 21);
    assert_eq!(p.int("WeaveVerticalThickness"), 63);
}

#[test]
fn reopened_pattern_panel_reads_saved_brick_aliases_without_changing_xml_or_other_setters() {
    let fixture = Fixture::new();
    let mut sequence = vixen_io::new_sequence();
    sequence.length = TimeSpan::from_ms_i(1000);
    sequence.effects.push(EffectNode {
        instance_id: Uuid::new_v4(), type_id: descriptor().type_id, start: TimeSpan::ZERO,
        time_span: sequence.length, targets: vec![(fixture.target, "Matrix".into())], params: saved_values(),
    });
    vixen_io::save_sequence(&sequence, &fixture.path()).unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let mut opened = SequenceSession::open(&fixture.path(), &fixture.env).unwrap();
    let id = opened.effects().next().unwrap().id;
    assert_panel(&opened, id);
    assert!(!opened.is_modified());
    opened.save(None).unwrap();
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before, "computed aliases never become XML members");
}

#[test]
fn saved_pattern_defaults_hydrate_brick_aliases_when_placed() {
    let fixture = Fixture::new();
    let mut defaults = vixen_io::effect_defaults::EffectDefaultsStore::load(&fixture.paths);
    defaults.save(descriptor(), &saved_values()).unwrap();
    // Re-read the actual NBFX defaults file: no in-memory aliases can leak into the test.
    let defaults = vixen_io::effect_defaults::EffectDefaultsStore::load(&fixture.paths);
    let mut session = SequenceSession::new_untitled(&fixture.env);
    session.set_effect_defaults(Arc::new(Mutex::new(defaults)));
    let id = session.add_new_effect(descriptor().type_id, fixture.target, "Matrix", TimeSpan::ZERO, TimeSpan::from_ms_i(1000)).unwrap();
    assert_panel(&session, id);
}
