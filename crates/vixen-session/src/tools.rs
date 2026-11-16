//! Profile configuration for standalone tools. Document contents remain window-local.
pub mod polygon;

use uuid::Uuid;
use vixen_core::Color;
use vixen_io::{ModuleEntry, ModuleStore};
use vixen_xml::XElement;

const PROP_TYPE: Uuid = Uuid::from_u128(0xdc844055_e978_47fe_8359_dbcd5b568e80);
const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.CustomPropEditor";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropPreferences {
    pub light_color: Color,
    pub selected_color: Color,
    pub state_base_color: Color,
    pub light_size: u32,
}
impl Default for PropPreferences {
    fn default() -> Self {
        Self { light_color: Color::WHITE, selected_color: Color::rgb(255,105,180), state_base_color: Color::rgb(25,25,25), light_size: 3 }
    }
}
impl PropPreferences {
    pub fn read(store: &ModuleStore) -> Self {
        let default = Self::default();
        let Some(data) = store.type_data(PROP_TYPE).and_then(|e| e.data()) else { return default };
        let color = |name, fallback| data.child(name).and_then(vixen_io::values::color_from_xml).filter(|c| *c != Color::EMPTY).unwrap_or(fallback);
        Self {
            light_color: color("LightColor", default.light_color), selected_color: color("SelectedLightColor", default.selected_color), state_base_color: color("StatePreviewBaseColor", default.state_base_color),
            light_size: data.child("DefaultLightSize").and_then(|e| e.text().parse::<u32>().ok()).filter(|n| *n > 0).unwrap_or(default.light_size),
        }
    }
    fn xml(&self) -> XElement {
        let mut data = XElement::new(NS, "CustomPropEditorData");
        for (name, color) in [("LightColor", self.light_color), ("SelectedLightColor", self.selected_color), ("StatePreviewBaseColor", self.state_base_color)] {
            data.push(vixen_io::values::color_to_xml(NS, name, color));
        }
        data.push(XElement::with_text(NS, "DefaultLightSize", if self.light_size == 0 { 3 } else { self.light_size }.to_string()));
        data
    }
    pub fn write(&self, store: &mut ModuleStore) {
        let old = Self::read(store).xml(); let new = self.xml();
        if let Some(entry) = store.type_data_mut(PROP_TYPE) { entry.merge_data(&old, new); }
        else { store.set_type_data(ModuleEntry::new("VixenModules.App.CustomPropEditor.CustomPropEditorData, CustomPropEditor", &PROP_TYPE, &PROP_TYPE, "Module.App.CustomPropEditor", new)); }
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn preferences_round_trip_and_keep_unrecognized_module_members() {
        let mut store = ModuleStore::new(); assert_eq!(PropPreferences::read(&store), PropPreferences::default());
        let wanted = PropPreferences { light_color: Color::BLUE, selected_color: Color::LIME, state_base_color: Color::rgb(12,23,34), light_size: 12 };
        wanted.write(&mut store);
        store.type_data_mut(PROP_TYPE).unwrap().data_mut().unwrap().push(XElement::with_text(NS,"ModuleData","preserve"));
        assert_eq!(PropPreferences::read(&store), wanted);
        PropPreferences::default().write(&mut store);
        assert_eq!(store.type_data(PROP_TYPE).unwrap().data().unwrap().child("ModuleData").unwrap().text(), "preserve");
        assert_eq!(PropPreferences::read(&store), PropPreferences::default());
    }
    #[test] fn empty_colors_and_zero_size_use_upstream_defaults() {
        let mut store = ModuleStore::new();
        PropPreferences { light_color: Color::EMPTY, selected_color: Color::EMPTY, state_base_color: Color::EMPTY, light_size: 0 }.write(&mut store);
        assert_eq!(PropPreferences::read(&store), PropPreferences::default());
    }
}
