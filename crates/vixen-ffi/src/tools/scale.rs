use super::*;
use vixen_prop::scale::BackgroundScale;

#[derive(uniffi::Record, Clone)]
pub struct PropEdScaleState {
    pub source_width: i32, pub source_height: i32,
    pub current_width: i32, pub current_height: i32,
    pub result_width: i32, pub result_height: i32,
    pub percent: bool, pub locked: bool, pub has_lights: bool, pub scale_lights: bool,
    pub width: String, pub height: String, pub errors: Vec<String>,
}
#[derive(uniffi::Object)]
pub struct PropEdScale { inner: Mutex<BackgroundScale> }
#[uniffi::export]
impl PropEdScale {
    pub fn state(&self) -> PropEdScaleState {
        let s = self.inner.lock();
        PropEdScaleState { source_width: s.source.0, source_height: s.source.1, current_width:s.current.0, current_height:s.current.1, result_width:s.target.0, result_height:s.target.1, percent:s.percent, locked:s.locked, has_lights:s.has_lights, scale_lights:s.scale_lights, width:s.width.clone(), height:s.height.clone(), errors:s.errors() }
    }
    pub fn set_input(&self, width: bool, text: String) { self.inner.lock().set_input(width,text); }
    pub fn set_percent(&self, percent: bool) { self.inner.lock().set_percent(percent); }
    pub fn set_locked(&self, locked: bool) { self.inner.lock().set_locked(locked); }
    pub fn set_scale_lights(&self, scale: bool) { let mut s = self.inner.lock(); s.scale_lights = scale && s.has_lights; }
}
#[uniffi::export]
impl CustomPropEditor {
    pub fn background_scale(&self) -> Result<Arc<PropEdScale>, FfiError> {
        let e = self.inner.lock(); let p = e.prop();
        let scale = BackgroundScale::new((p.image.width as i32,p.image.height as i32),(p.width as i32,p.height as i32),p.light_count() > 0)?;
        Ok(Arc::new(PropEdScale { inner: Mutex::new(scale) }))
    }
}
