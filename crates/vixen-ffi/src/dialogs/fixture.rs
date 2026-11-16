//! Intelligent Fixture property configuration on the dialog's working world.
use super::{find_nodes, no_such, parse_id, DialogContext};
use crate::core::FfiError;
use crate::fixture_editor::FixtureEditor;
use std::sync::Arc;
use vixen_session::setup::fixture_editor as logic;
#[uniffi::export]
impl DialogContext {
    pub fn fixture_editor(&self, node: String) -> Result<Arc<FixtureEditor>, FfiError> {
        let id = parse_id(&node)?;
        self.world(|w| {
            let n = w.find_node(&id).ok_or_else(|| no_such("element"))?;
            Ok(FixtureEditor::new(
                Arc::clone(&self.core),
                logic::property_profile(w, n),
                None,
            ))
        })
    }
    pub fn apply_fixture_editor(
        &self,
        node: String,
        selection: Vec<String>,
        editor: Arc<FixtureEditor>,
    ) -> Result<(), FfiError> {
        let profile = {
            let e = editor.inner.lock();
            let errors = e.errors();
            if !errors.is_empty() {
                return Err(FfiError::new(errors.join("\n")));
            }
            e.profile()
        };
        let id = parse_id(&node)?;
        self.world(|w| {
            let n = w.find_node(&id).ok_or_else(|| no_such("element"))?;
            let selected = find_nodes(w, &selection);
            logic::apply_property_profile(w, n, &selected, &profile);
            Ok(())
        })
    }
}
