//! Export Wire Diagram's API. The options cross as what the dialog's controls hold, so its rules stay on this side.

use super::DisplaySetupSession;
use crate::wire_diagram::SetupWireOptions;
use crate::core::FfiError;
use vixen_session::setup::wire_diagram::{wire_diagram_has_locations, wire_diagram_points, write_wire_diagram, NoLocation, WireDiagramOptions};

#[uniffi::export]
impl DisplaySetupSession {
    /// Ask before showing the dialog, as upstream does: false means a leaf has no Location property, so F131's alert shows instead.
    pub fn wire_diagram_has_locations(&self, group_path: String) -> bool {
        let session = self.inner.read();
        session.tree.resolve(&group_path).and_then(|p| p.last().copied()).is_some_and(|group| wire_diagram_has_locations(&session.tree, group))
    }

    /// F131: writes the diagram to a new temp file and returns its path, for the window to open. `None` if a leaf has no Location property. A big group takes a while, so call
    /// it off the main thread. The drawing happens outside the session's lock.
    pub fn export_wire_diagram(&self, group_path: String, flip: bool, options: SetupWireOptions) -> Result<Option<String>, FfiError> {
        let options = WireDiagramOptions::from_dialog(&options.into()).map_err(FfiError::new)?;
        let points = {
            let session = self.inner.read();
            let group = session.tree.resolve(&group_path).and_then(|p| p.last().copied()).ok_or_else(|| FfiError::new("That item no longer exists."))?;
            match wire_diagram_points(&session.tree, group) {
                Ok(points) => points,
                Err(NoLocation) => return Ok(None),
            }
        };
        let path = write_wire_diagram(&points, flip, &options, &std::env::temp_dir())?;
        Ok(path.map(|p| p.to_string_lossy().into_owned()))
    }
}
