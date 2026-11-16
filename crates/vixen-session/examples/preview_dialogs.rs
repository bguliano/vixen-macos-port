//! Controller-free native Preview dialog fixture. Refuses existing/non-target output folders.
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;
use vixen_io::{
    system::{Channel, Preview},
    DataPaths, ModuleEntry, SystemData,
};
use vixen_preview::{
    shape::Modifiers, DesignSession, PreviewData, PropertyValue, Tool, VIXEN_PREVIEW_MODULE_TYPE,
};

fn main() {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("new target profile directory"),
    );
    assert!(!root.exists() && root.components().any(|c| c.as_os_str() == "target"));
    let invalid = std::env::args().any(|arg| arg == "--invalid");
    let paths = DataPaths::new(root);
    paths.ensure_directories().unwrap();
    let mut system = SystemData::default();
    let group = system.tree.add_string("Lamp", 3);
    let leaves = system.tree.node(group).children.clone();
    let spec = fx_fixture::fixture_base::test_support::sample_spec();
    for name in ["Fixture One", "Fixture Two"] {
        let node = system.tree.add_element(name);
        fx_fixture::fixture_base::test_support::add_fixture_property(&mut system.tree, node, &spec);
    }
    for (_, node) in system.tree.all_nodes() {
        if let Some(id) = node.element_id {
            system.channels.push(Channel {
                id,
                name: node.name.clone(),
            });
        }
    }
    let mut design = DesignSession::new(PreviewData::new(), Some(Arc::new(system.tree.clone())));
    design.set_background_size(600, 400);
    design.set_view_size(600, 400);
    design.set_use_open_gl(true);
    for (index, node) in leaves.iter().take(2).enumerate() {
        design.set_selected_nodes(&[system.tree.node(*node).id]);
        design.set_tool(Tool::String);
        let y = 80 + index as i32 * 100;
        design.mouse_down(50, y, Modifiers::NONE);
        design.mouse_move(300, y, Modifiers::NONE);
        design.mouse_up(300, y, Modifiers::NONE);
        let shape = design.data().display_items.last().expect("drawn string").id;
        design
            .set_shape_property(
                shape,
                "Name",
                PropertyValue::Text(format!("Test String {}", index + 1)),
            )
            .unwrap();
        design
            .set_shape_property(shape, "PixelSize", PropertyValue::Int(4 + index as i32 * 4))
            .unwrap();
    }
    let mut data = design.into_data();
    if invalid {
        let group_id = system.tree.node(group).id;
        data.display_items[0]
            .shape
            .for_each_pixel_mut(&mut |pixel| pixel.node_id = group_id);
    }
    let id = Uuid::new_v4();
    data.module_instance_id = Some(id.to_string());
    data.module_type_id = Some(VIXEN_PREVIEW_MODULE_TYPE.to_string());
    data.width = 600;
    data.height = 400;
    system.previews.push(Preview {
        name: "Preview Dialog Verification".into(),
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
    println!("{}", paths.root().display());
}
