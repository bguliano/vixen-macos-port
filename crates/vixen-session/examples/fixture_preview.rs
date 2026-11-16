//! Builds a hardware-free fixture preview profile for native UI verification. Refuses existing folders.
//! cargo run -p vixen-session --example fixture_preview -- target/fixture-ui-profile
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;
use vixen_core::element::{property_ids, ElementColorType, PropertyData, PropertyInstance};
use vixen_core::{Color, Curve, TimeSpan};
use vixen_effects::framework::ParamValue;
use vixen_io::system::{Channel, OutputFilter, Patch, Preview};
use vixen_io::{DataPaths, ModuleEntry, SystemData};
use vixen_output::filters::{FilterConfig, ShutterFilterConfig};
use vixen_output::{ids, ModuleIds};
use vixen_preview::shape::Modifiers;
use vixen_preview::{DesignSession, PreviewData, PropertyValue, Tool, VIXEN_PREVIEW_MODULE_TYPE};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("new test profile folder"));
    assert!(!root.exists(), "test folder must not exist");
    assert!(
        root.components().any(|p| p.as_os_str() == "target"),
        "test profiles belong under target"
    );
    let paths = DataPaths::new(root);
    paths.ensure_directories().unwrap();
    let mut system = SystemData::default();
    let mut nodes = Vec::new();
    let mut profile = fx_fixture::fixture_base::test_support::sample_spec();
    profile
        .functions
        .iter_mut()
        .find(|f| f.identity == fx_fixture::spec::FuncId::Tilt)
        .unwrap()
        .rotation_limits
        .stop_position = 180;
    profile
        .functions
        .iter_mut()
        .find(|f| f.identity == fx_fixture::spec::FuncId::Shutter)
        .unwrap()
        .index_data
        .push(fx_fixture::spec::FixtureIndexItem {
            name: "Strobe".into(),
            start_value: 16,
            end_value: 255,
            use_curve: true,
            index_type: fx_fixture::spec::FixtureIndexType::Strobe,
            ..fx_fixture::spec::FixtureIndexItem::index()
        });
    for name in ["Floor Head", "Ceiling Head"] {
        let node = system.tree.add_element(name);
        let n = system.tree.node(node).clone();
        nodes.push(n.id);
        system.channels.push(Channel {
            id: n.element_id.unwrap(),
            name: name.into(),
        });
        fx_fixture::fixture_base::test_support::add_fixture_property(
            &mut system.tree,
            node,
            &profile,
        );
        system
            .tree
            .node_mut(node)
            .properties
            .push(PropertyInstance {
                type_id: property_ids::COLOR,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Color {
                    color_type: ElementColorType::FullColor,
                    single_color: Color::WHITE,
                    color_set_name: None,
                },
            });
        let fid = Uuid::new_v4();
        let c = FilterConfig::Shutter(ShutterFilterConfig {
            ids: ModuleIds::new(ids::SHUTTER_FILTER, fid),
            convert_color_into_shutter_intents: true,
            ..Default::default()
        });
        system.filters.push(OutputFilter {
            type_id: ids::SHUTTER_FILTER,
            instance_id: fid,
        });
        system.patches.push(Patch {
            component_id: fid,
            source_id: n.element_id,
            output: 0,
        });
        system.module_store.set_instance(ModuleEntry::new(
            "VixenModules.OutputFilter.ShutterFilter.ShutterFilterData, ShutterFilter",
            &ids::SHUTTER_FILTER,
            &fid,
            "Module.OutputFilter.ShutterFilter",
            c.to_xml(),
        ));
    }
    let mut design = DesignSession::new(PreviewData::new(), Some(Arc::new(system.tree.clone())));
    design.set_background_size(600, 400);
    design.set_view_size(600, 400);
    design.set_use_open_gl(true);
    for (i, node) in nodes.iter().enumerate() {
        design.set_selected_nodes(&[*node]);
        design.set_tool(Tool::MovingHead);
        let x = 100 + i as i32 * 250;
        let y = if i == 0 { 220 } else { 50 };
        design.mouse_down(x, y, Modifiers::NONE);
        design.mouse_move(x + 80, y + 80, Modifiers::NONE);
        design.mouse_up(x + 80, y + 80, Modifiers::NONE);
    }
    let mut data = design.into_data();
    data.width = 600;
    data.height = 400;
    for (i, item) in data.display_items.iter_mut().enumerate() {
        item.shape
            .set_property(
                "MountingPosition",
                PropertyValue::Enum(if i == 0 { "Bottom" } else { "Top" }.into()),
                "Moving Head",
            )
            .unwrap();
        item.shape
            .set_property("BeamLength", PropertyValue::Int(40), "Moving Head")
            .unwrap();
    }
    let id = Uuid::new_v4();
    data.module_instance_id = Some(id.to_string());
    data.module_type_id = Some(VIXEN_PREVIEW_MODULE_TYPE.to_string());
    let folder = paths.module_data_dir().join("VixenDisplayPreview");
    std::fs::create_dir_all(&folder).unwrap();
    image::RgbaImage::from_pixel(600, 400, image::Rgba([18, 18, 20, 255]))
        .save(folder.join("fixture-background.png"))
        .unwrap();
    data.background_file_name = Some("fixture-background.png".into());
    system.previews.push(Preview {
        name: "Fixture Verification".into(),
        type_id: VIXEN_PREVIEW_MODULE_TYPE,
        instance_id: id,
    });
    system.module_store.set_instance(ModuleEntry::new(
        "VixenModules.Preview.VixenPreview.VixenPreviewData, VixenPreview",
        &VIXEN_PREVIEW_MODULE_TYPE,
        &id,
        "Module.Preview.VixenPreview",
        data.to_xml(),
    ));
    vixen_io::save_system(&system, &paths).unwrap();
    let env = SessionEnvironment { video_cache: None,
        tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: 50,
        media_dir: paths.media_dir(),
    };
    let mut sequence = SequenceSession::new_untitled(&env);
    sequence.set_length(TimeSpan::from_secs(20.0));
    for (i, node) in nodes.iter().enumerate() {
        for name in ["Set Level", "Set Position", "Fixture Strobe"] {
            let descriptor = vixen_effect_registry::all()
                .iter()
                .find(|d| d.name == name)
                .unwrap();
            let at = if name == "Fixture Strobe" { 10.0 } else { 0.0 };
            let duration = 20.0 - at;
            let id = sequence
                .add_new_effect(
                    descriptor.type_id,
                    *node,
                    if i == 0 { "Floor Head" } else { "Ceiling Head" },
                    TimeSpan::from_secs(at),
                    TimeSpan::from_secs(duration),
                )
                .unwrap();
            let mut params = sequence.effect(id).unwrap().node.params.clone();
            match name {
                "Set Level" => {
                    params.set(
                        "Color",
                        ParamValue::Rgb(vixen_core::color::Rgb::from_color(if i == 0 {
                            Color::RED
                        } else {
                            Color::BLUE
                        })),
                    );
                }
                "Set Position" => {
                    params.set(
                        "Pan",
                        ParamValue::Curve(Curve::from_points(vec![(0.0, 0.0), (100.0, 60.0)])),
                    );
                    params.set("Tilt", ParamValue::Curve(Curve::flat(15.0)));
                }
                _ => {
                    params.set(
                        "FixtureStrobeFunctionName",
                        ParamValue::Str("Shutter".into()),
                    );
                    params.set("FixtureStrobeIndexValue", ParamValue::Str("Strobe".into()));
                    params.set("FixtureStrobeCurve", ParamValue::Curve(Curve::flat(30.0)));
                }
            }
            sequence.set_params(id, params);
        }
    }
    let path = paths.sequence_dir().join("Fixtures.tim");
    sequence.save(Some(&path)).unwrap();
    println!("{}", path.display());
}
