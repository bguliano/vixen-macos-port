use crate::core::{FfiError, VixenCore};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;
use vixen_session::element_mapper::ElementMapState;

#[derive(Clone, uniffi::Record)]
pub struct MapperSource {
    pub id: String,
    pub name: String,
}
#[derive(Clone, uniffi::Record)]
pub struct MapperEntry {
    pub source_id: String,
    pub source_name: String,
    pub target_id: Option<String>,
    pub target_name: String,
}
#[derive(Clone, uniffi::Record)]
pub struct MapperTreeRow {
    pub path: String,
    pub id: String,
    pub name: String,
    pub target_name: String,
    pub child_count: u32,
    pub active: bool,
}
#[derive(Clone, uniffi::Record)]
pub struct MapperView {
    pub name: String,
    pub path: Option<String>,
    pub modified: bool,
    pub has_source_tree: bool,
    pub missing_sources: u32,
    pub rows: Vec<MapperEntry>,
}
#[derive(uniffi::Object)]
pub struct ElementMapperDialog {
    state: Mutex<ElementMapState>,
}
impl ElementMapperDialog {
    pub(super) fn from_state(state: ElementMapState) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(state),
        })
    }
}
fn id(text: &str) -> Result<Uuid, FfiError> {
    Uuid::parse_str(text).map_err(|_| FfiError::new("Invalid element id."))
}

#[uniffi::export]
impl VixenCore {
    pub fn element_mapper(
        &self,
        name: String,
        sources: Vec<MapperSource>,
    ) -> Result<Arc<ElementMapperDialog>, FfiError> {
        let sources = sources
            .into_iter()
            .map(|s| Ok((id(&s.id)?, s.name)))
            .collect::<Result<Vec<_>, FfiError>>()?;
        let tree = Arc::new(self.inner.system.read().tree.clone());
        Ok(Arc::new(ElementMapperDialog {
            state: Mutex::new(ElementMapState::new(name, sources, tree)),
        }))
    }
}
#[uniffi::export]
impl ElementMapperDialog {
    pub fn view(&self) -> MapperView {
        let s = self.state.lock();
        MapperView {
            name: s.name.clone(),
            path: s.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            modified: s.modified,
            has_source_tree: s.has_source_tree(),
            missing_sources: s.missing_sources() as u32,
            rows: s
                .mappings
                .iter()
                .map(|m| MapperEntry {
                    source_id: m.source_id.to_string(),
                    source_name: m.source_name.clone(),
                    target_id: m.target_id.map(|id| id.to_string()),
                    target_name: m.target_name.clone(),
                })
                .collect(),
        }
    }
    pub fn reset(&self, name: String) {
        self.state.lock().reset(name);
    }
    pub fn open_map(&self, path: String) -> Result<(), FfiError> {
        self.state.lock().load(Path::new(&path)).map_err(Into::into)
    }
    pub fn save_map(&self, path: String) -> Result<(), FfiError> {
        self.state.lock().save(Path::new(&path)).map_err(Into::into)
    }
    pub fn load_tree(&self, path: String) -> Result<(), FfiError> {
        self.state
            .lock()
            .load_tree(Path::new(&path))
            .map_err(Into::into)
    }
    pub fn add_missing_sources(&self) {
        self.state.lock().add_missing_sources();
    }
    pub fn assign(&self, source: String, target: Option<String>) -> Result<(), FfiError> {
        self.state
            .lock()
            .assign(id(&source)?, target.as_deref().map(id).transpose()?)
            .map_err(Into::into)
    }
    pub fn tree_rows(&self, source: bool, parent: Option<String>) -> Vec<MapperTreeRow> {
        self.state
            .lock()
            .tree_rows(source, parent.as_deref())
            .into_iter()
            .map(|r| MapperTreeRow {
                path: r.path,
                id: r.id.to_string(),
                name: r.name,
                target_name: r.target_name,
                child_count: r.child_count as u32,
                active: r.active,
            })
            .collect()
    }
    pub fn accepted_entries(&self) -> Vec<MapperEntry> {
        let s = self.state.lock();
        s.accepted_pairs()
            .into_iter()
            .map(|(source, target)| MapperEntry {
                source_id: source.to_string(),
                source_name: String::new(),
                target_id: Some(target.to_string()),
                target_name: String::new(),
            })
            .collect()
    }
}
