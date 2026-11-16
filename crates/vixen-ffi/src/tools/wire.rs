use super::*;
use crate::wire_diagram::SetupWireOptions;
use vixen_session::setup::wire_diagram::{write_wire_diagram, WireDiagramOptions};

#[uniffi::export]
impl CustomPropEditor {
    pub fn export_wire_diagram(&self, flip: bool, options: SetupWireOptions) -> Result<Option<String>, FfiError> {
        let options = WireDiagramOptions::from_dialog(&options.into()).map_err(FfiError::new)?;
        let points = self.inner.lock().wire_points();
        Ok(write_wire_diagram(&points, flip, &options, &std::env::temp_dir())?.map(|p| p.to_string_lossy().into_owned()))
    }
}
