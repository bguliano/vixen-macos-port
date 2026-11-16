use super::*;
use crate::core::VixenCore;
use vixen_core::Color;
use vixen_session::tools::PropPreferences;

#[derive(uniffi::Record, Clone)]
pub struct PropEdPreferences {
    pub light_color: u32,
    pub selected_color: u32,
    pub state_base_color: u32,
    pub light_size: u32,
}
impl From<PropPreferences> for PropEdPreferences {
    fn from(p: PropPreferences) -> Self { Self { light_color: p.light_color.to_argb_u32(), selected_color: p.selected_color.to_argb_u32(), state_base_color: p.state_base_color.to_argb_u32(), light_size: p.light_size } }
}
impl From<PropEdPreferences> for PropPreferences {
    fn from(p: PropEdPreferences) -> Self { Self { light_color: Color::from_argb_u32(p.light_color), selected_color: Color::from_argb_u32(p.selected_color), state_base_color: Color::from_argb_u32(p.state_base_color), light_size: if p.light_size == 0 { 3 } else { p.light_size } } }
}
#[uniffi::export]
pub fn prop_ed_default_preferences() -> PropEdPreferences { PropPreferences::default().into() }

#[uniffi::export]
impl VixenCore {
    pub fn prop_preferences(&self) -> PropEdPreferences { PropPreferences::read(&self.inner.system.read().module_store).into() }
    pub fn save_prop_preferences(&self, preferences: PropEdPreferences) -> Result<(), FfiError> {
        PropPreferences::from(preferences).write(&mut self.inner.system.write().module_store);
        self.inner.save_module_config()?; Ok(())
    }
}
#[uniffi::export]
impl CustomPropEditor {
    pub fn apply_preferences(&self, preferences: PropEdPreferences) {
        let p = PropPreferences::from(preferences); let mut e = self.inner.lock(); let user_name = e.settings().user_name.clone();
        e.set_settings(PropEditorSettings { light_color: p.light_color, selected_light_color: p.selected_color, state_preview_base_color: p.state_base_color, default_light_size: p.light_size.min(i32::MAX as u32) as i32, user_name });
    }
}
