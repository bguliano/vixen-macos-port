//! Adds a preview of about 100,000 lights to a copy of a show, for measuring the designer and the live window at that size.
//!
//!     cargo run -p vixen-session --example synthetic_preview -- <show folder>
//!
//! 100 mega trees of 16 strings of 62 lights, 99,200 lights in shapes of a realistic size, the shapes spread over 3000 x 3000. Never point
//! this at the owner's real show.

use uuid::Uuid;
use vixen_io::system::{save_module_config_file, save_system_config_file, Preview};
use vixen_io::{load_system, DataPaths, ModuleEntry};
use vixen_preview::data::PreviewData;
use vixen_preview::shape::Modifiers;
use vixen_preview::{DesignSession, Tool, VIXEN_PREVIEW_MODULE_TYPE};

fn main() {
    let root = std::env::args().nth(1).expect("the show folder");
    assert!(!root.contains("Documents/Vixen 3"), "this writes to the folder it's given");
    let paths = DataPaths::new(&root);
    let mut system = load_system(&paths).expect("the show");
    let id = Uuid::new_v4();

    let mut session = DesignSession::new(PreviewData::new(), None);
    session.set_background_size(3200, 3200);
    session.set_view_size(3200, 3200);
    session.set_keep_insert_mode(true);
    for i in 0..100 {
        let (x, y) = ((i % 10) * 300 + 20, (i / 10) * 300 + 20);
        session.set_tool(Tool::MegaTree);
        session.mouse_down(x, y, Modifiers::NONE);
        session.mouse_move(x + 280, y + 280, Modifiers::NONE);
        session.mouse_up(x + 280, y + 280, Modifiers::NONE);
    }
    session.set_tool(Tool::Select);
    let mut data = session.into_data();
    for item in &mut data.display_items {
        // 62 lights a string: 16 x 62 x 100 = 99,200.
        let name = item.shape.properties().into_iter().find(|p| p.id == "LightsPerString" || p.name == "Lights Per String").map(|p| p.id);
        if let Some(name) = name {
            item.shape.set_property(name, vixen_preview::PropertyValue::Int(62), "Mega Tree").expect("a light count");
        }
    }
    data.module_instance_id = Some(id.to_string());
    data.module_type_id = Some(VIXEN_PREVIEW_MODULE_TYPE.to_string());
    println!("{} shapes, {} pixels", data.display_items.len(), data.pixel_count());

    system.previews.push(Preview { name: "Synthetic 100k".to_string(), type_id: VIXEN_PREVIEW_MODULE_TYPE, instance_id: id });
    system.module_store.set_instance(ModuleEntry::new(
        "VixenModules.Preview.VixenPreview.VixenPreviewData, VixenPreview",
        &VIXEN_PREVIEW_MODULE_TYPE,
        &id,
        "Module.Preview.VixenPreview",
        data.to_xml(),
    ));
    save_system_config_file(&system, &paths).expect("the system file");
    save_module_config_file(&system, &paths).expect("the module file");
    println!("Added preview {id}");
}
