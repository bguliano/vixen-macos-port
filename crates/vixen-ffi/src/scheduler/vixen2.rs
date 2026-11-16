use super::element_mapper::{ElementMapperDialog, MapperTreeRow};
use crate::{
    core::{FfiError, VixenCore},
    sequence::SequencerSession,
};
use parking_lot::Mutex;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use uuid::Uuid;
use vixen_core::{Color, ElementTree};
use vixen_session::{
    app::AppCore,
    element_mapper::ElementMapState,
    sequence::SequenceSession,
    vixen2::{self, maps, ChannelMapping, Vixen2File},
};

#[derive(Clone, uniffi::Record)]
pub struct Vixen2Info {
    pub path: String,
    pub audio_name: String,
    pub profile_name: String,
    pub profile_path: String,
    pub maps: Vec<String>,
    pub channels: u32,
    pub selected: Option<String>,
    pub progress: u32,
}
#[derive(Clone, uniffi::Record)]
pub struct Vixen2Row {
    pub number: String,
    pub output: String,
    pub name: String,
    pub color: u32,
    pub target: Option<String>,
    pub target_name: String,
    pub import_color: u32,
    pub mixing: bool,
}
struct State {
    file: Vixen2File,
    ready: bool,
    maps: maps::Maps,
    selected: Option<String>,
}
#[derive(uniffi::Object)]
pub struct Vixen2Import {
    core: Arc<AppCore>,
    state: Mutex<State>,
    progress: AtomicU32,
}
#[derive(uniffi::Object)]
pub struct Vixen2ChannelMap {
    rows: Mutex<Vec<ChannelMapping>>,
    tree: Arc<ElementTree>,
    outline: Arc<ElementMapperDialog>,
}
#[uniffi::export]
pub fn vixen2_color_name(argb: u32) -> String {
    (27..=167)
        .chain(std::iter::once(175))
        .find(|&n| vixen_io::values::known_color_argb(n) == Some(argb))
        .and_then(vixen_io::values::known_color_name)
        .unwrap_or("")
        .to_owned()
}
#[uniffi::export]
impl VixenCore {
    pub fn vixen2_import(&self, path: String) -> Result<Arc<Vixen2Import>, FfiError> {
        let file = Vixen2File::read(Path::new(&path))?;
        let maps = maps::load(&self.inner.system.read().module_store);
        Ok(Arc::new(Vixen2Import {
            core: Arc::clone(&self.inner),
            state: Mutex::new(State {
                file,
                ready: false,
                maps,
                selected: None,
            }),
            progress: AtomicU32::new(0),
        }))
    }
}
#[uniffi::export]
impl Vixen2Import {
    pub fn info(&self) -> Vixen2Info {
        let s = self.state.lock();
        Vixen2Info {
            path: s.file.path.to_string_lossy().into(),
            audio_name: s.file.audio_name.clone(),
            profile_name: s.file.profile_name.clone(),
            profile_path: s
                .file
                .profile
                .as_ref()
                .map_or(String::new(), |p| p.to_string_lossy().into()),
            maps: s.maps.keys().cloned().collect(),
            channels: s.file.channels.len() as u32,
            selected: s.selected.clone(),
            progress: self.progress.load(Ordering::Relaxed),
        }
    }
    pub fn finish_input(
        &self,
        audio: Option<String>,
        profile: Option<String>,
    ) -> Result<(), FfiError> {
        let mut s = self.state.lock();
        s.file = s.file.clone().finish(
            audio.as_deref().map(Path::new),
            profile.as_deref().map(Path::new),
        )?;
        s.ready = true;
        Ok(())
    }
    pub fn select_map(&self, name: Option<String>) -> Result<(), FfiError> {
        let mut s = self.state.lock();
        if name.as_ref().is_some_and(|n| !s.maps.contains_key(n)) {
            return Err(FfiError::new("Mapping data is missing, please try again."));
        }
        s.selected = name;
        Ok(())
    }
    pub fn edit_map(&self) -> Arc<Vixen2ChannelMap> {
        let s = self.state.lock();
        let tree = Arc::new(self.core.system.read().tree.clone());
        let rows = maps::normalized(
            s.selected
                .as_ref()
                .and_then(|n| s.maps.get(n))
                .unwrap_or(&s.file.channels),
            &tree,
        );
        let outline = ElementMapperDialog::from_state(ElementMapState::new(
            String::new(),
            vec![],
            Arc::clone(&tree),
        ));
        Arc::new(Vixen2ChannelMap {
            rows: Mutex::new(rows),
            tree,
            outline,
        })
    }
    pub fn save_map(&self, name: String, map: Arc<Vixen2ChannelMap>) -> Result<(), FfiError> {
        if name.trim().is_empty() {
            return Err(FfiError::new("Please enter name of Map."));
        }
        let rows = maps::normalized(&map.rows.lock(), &map.tree);
        let mut s = self.state.lock();
        let mut updated = s.maps.clone();
        updated.insert(name, rows);
        {
            let mut system = self.core.system.write();
            maps::save(&mut system.module_store, &updated);
        }
        self.core.save_module_config()?;
        s.maps = updated;
        s.selected = None;
        Ok(())
    }
    pub fn convert(&self) -> Result<Arc<SequencerSession>, FfiError> {
        let (file, rows) = {
            let s = self.state.lock();
            if !s.ready {
                return Err(FfiError::new("The sequence has not finished loading."));
            }
            let rows = s
                .selected
                .as_ref()
                .and_then(|n| s.maps.get(n))
                .ok_or_else(|| FfiError::new("Mapping data is missing, please try again."))?
                .clone();
            (s.file.clone(), rows)
        };
        let env = self.core.session_environment();
        let (mut document, warnings) = vixen2::convert(&file, &rows, &env.tree, |n| {
            self.progress.store(n as u32, Ordering::Relaxed);
        })?;
        for warning in warnings {
            vixen_session::logging::warn("VixenModules.SequenceType.Vixen2x", &warning);
        }
        if let Some(audio) = file.audio.as_ref().filter(|p| p.is_file()) {
            vixen2::associate_audio(&mut document, audio, &env.media_dir)?;
        }
        let session = SequenceSession::imported(document, &env);
        Ok(SequencerSession::wrap(Arc::clone(&self.core), session))
    }
    pub fn missing_audio(&self) -> bool {
        let s = self.state.lock();
        !s.file.audio_name.is_empty() && !s.file.audio.as_ref().is_some_and(|p| p.is_file())
    }
}
#[uniffi::export]
impl Vixen2ChannelMap {
    pub fn rows(&self) -> Vec<Vixen2Row> {
        self.rows
            .lock()
            .iter()
            .map(|m| Vixen2Row {
                number: m.number.clone(),
                output: m.output.clone(),
                name: m.name.clone(),
                color: m.color.to_argb_u32(),
                target: m.destination.map(|id| id.to_string()),
                target_name: m
                    .destination
                    .and_then(|id| self.tree.find(&id))
                    .map_or(String::new(), |n| self.tree.node(n).name.clone()),
                import_color: m.import_color.to_argb_u32(),
                mixing: m.mixing,
            })
            .collect()
    }
    pub fn tree_rows(&self, parent: Option<String>) -> Vec<MapperTreeRow> {
        self.outline.tree_rows(false, parent)
    }
    pub fn apply_drop(
        &self,
        start: u32,
        nodes: Vec<String>,
        repeat: u32,
        mixing: bool,
        order: String,
    ) -> Result<(), FfiError> {
        let nodes = nodes
            .iter()
            .map(|s| Uuid::parse_str(s).map_err(|_| FfiError::new("Invalid destination element.")))
            .collect::<Result<Vec<_>, _>>()?;
        vixen2::apply_drop(
            &mut self.rows.lock(),
            start as usize,
            &nodes,
            repeat as usize,
            mixing,
            &order,
            &self.tree,
        )?;
        Ok(())
    }
    pub fn set_color(&self, indices: Vec<u32>, color: u32) {
        let mut rows = self.rows.lock();
        for n in indices {
            if let Some(row) = rows.get_mut(n as usize) {
                row.import_color = Color::from_argb_u32(color);
            }
        }
    }
    pub fn clear(&self, indices: Vec<u32>) {
        let mut rows = self.rows.lock();
        for n in indices {
            if let Some(row) = rows.get_mut(n as usize) {
                row.destination = None;
                row.import_color = Color::EMPTY;
                row.mixing = false;
            }
        }
    }
}
