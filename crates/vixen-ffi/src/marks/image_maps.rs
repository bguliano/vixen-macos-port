//! The LipSync image map library's windows: Image Maps, the Image Map editor and the Image Mapper. Spec sections L and M, F89 to F105
//! and F118. The logic is `vixen_session::marks::map_library`; this only converts.

use crate::core::VixenCore;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use vixen_session::app::AppCore;
use vixen_session::marks::lipsync::{PhonemeType, MAP_PHONEMES};
use vixen_session::marks::map_library::{self as maps, EditCommit, LipSyncLibrary, MapEdit, MapperChoice};

#[derive(uniffi::Record, Clone, PartialEq, Debug)]
pub struct ImageMapRow {
    pub name: String,
    pub notes: String,
    /// Shown bold.
    pub is_default: bool,
}

/// What a button did, for the window to show: an alert when `error` is set, titled `Error!`.
#[derive(uniffi::Record, Clone, PartialEq, Debug)]
pub struct ImageMapOutcome {
    /// The map made or renamed, when there is one.
    pub name: Option<String>,
    pub error: Option<String>,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum ImageMapperChoice {
    Unmapped,
    Mapped,
    File { index: u32 },
}

#[derive(uniffi::Enum, Clone, PartialEq, Debug)]
pub enum ImageMapCommit {
    /// Saved. `alert` is a rename that was refused: the save went ahead under the old name.
    Saved { name: String, alert: Option<String> },
    /// "Overwrite existing <name> mapping?", titled "Map exists": commit again with the answer.
    AskOverwrite { name: String },
}

/// One showing of the Image Maps window. The library everything else reads follows each change; `close` saves.
#[derive(uniffi::Object)]
pub struct ImageMaps {
    core: Arc<AppCore>,
    library: Mutex<LipSyncLibrary>,
}

impl ImageMaps {
    fn root(&self) -> PathBuf {
        self.core.lip_sync_root()
    }

    fn publish(&self, library: &LipSyncLibrary) {
        self.core.set_lip_sync_library(library);
    }
}

#[uniffi::export]
impl VixenCore {
    /// Tools > LipSync > Edit Image Maps, F90.
    pub fn image_maps(&self) -> Arc<ImageMaps> {
        Arc::new(ImageMaps { core: Arc::clone(&self.inner), library: Mutex::new(self.inner.lip_sync_library()) })
    }

    /// The Default Image Map submenu, F89: every map in library order.
    pub fn image_map_names(&self) -> Vec<String> {
        self.inner.lip_sync_library().names()
    }

    /// The map the submenu checks.
    pub fn default_image_map(&self) -> Option<String> {
        self.inner.lip_sync_library().default_name()
    }

    /// Choosing a map in the submenu. The open sequences become modified and the module store is saved.
    pub fn set_default_image_map(&self, name: String) -> bool {
        self.inner.set_default_image_map(&name)
    }
}

#[uniffi::export]
impl ImageMaps {
    /// Sorted by name.
    pub fn rows(&self) -> Vec<ImageMapRow> {
        self.library.lock().list().into_iter().map(|m| ImageMapRow { name: m.name, notes: m.notes, is_default: m.is_default }).collect()
    }

    /// New, F91: a map is added and its editor opens. Cancelling the editor takes the map away again.
    pub fn new_map(self: Arc<Self>) -> Result<Arc<ImageMapEditor>, crate::core::FfiError> {
        let name = {
            let mut library = self.library.lock();
            let name = library.new_map(&self.root());
            self.publish(&library);
            name
        };
        self.open(name, true)
    }

    /// Edit, F92.
    pub fn edit(self: Arc<Self>, name: String) -> Result<Arc<ImageMapEditor>, crate::core::FfiError> {
        self.open(name, false)
    }

    /// Remove, F93, after the question. With every map chosen nothing happens.
    pub fn remove(&self, names: Vec<String>) -> Option<String> {
        let mut library = self.library.lock();
        let result = library.remove_maps(&names, &self.root());
        self.publish(&library);
        result.err()
    }

    /// Whether Remove may run on this selection: something chosen, and a map left afterwards.
    pub fn can_remove(&self, names: Vec<String>) -> bool {
        !names.is_empty() && names.len() < self.library.lock().len()
    }

    /// Clone, F94: each chosen map in turn. The first failure's message is returned once all are done.
    pub fn clone_maps(&self, names: Vec<String>) -> ImageMapOutcome {
        let mut library = self.library.lock();
        let mut outcome = ImageMapOutcome { name: None, error: None };
        for name in names {
            if let Some(made) = library.clone_map(&name, &self.root()) {
                outcome.name = Some(made.name);
                outcome.error = outcome.error.or(made.error);
            }
        }
        self.publish(&library);
        outcome
    }

    /// Rename in place, F95 and F118. Nothing set means nothing was to be done.
    pub fn rename(&self, old: String, new: String) -> ImageMapOutcome {
        let mut library = self.library.lock();
        let outcome = match library.rename_map(&old, &new, &self.root()) {
            Ok(name) => ImageMapOutcome { name, error: None },
            Err(error) => ImageMapOutcome { name: None, error: Some(error) },
        };
        self.publish(&library);
        outcome
    }

    pub fn changed(&self) -> bool {
        self.library.lock().changed()
    }

    /// The window closed, F96: with any change, the open sequences are modified, their LipSync effects render again and the
    /// configuration is saved. True if that happened.
    pub fn close(&self) -> bool {
        let library = self.library.lock().clone();
        self.core.image_maps_closed(&library)
    }
}

impl ImageMaps {
    fn open(self: Arc<Self>, name: String, is_new: bool) -> Result<Arc<ImageMapEditor>, crate::core::FfiError> {
        let edit = {
            let mut library = self.library.lock();
            match library.begin_edit(&name, &self.root()) {
                Ok(edit) => edit,
                Err(message) => {
                    if is_new {
                        library.cancel_new(&self.root());
                        self.publish(&library);
                    }
                    return Err(crate::core::FfiError::new(message));
                }
            }
        };
        Ok(Arc::new(ImageMapEditor { maps: self, edit: Mutex::new(edit), is_new }))
    }
}

/// The Image Map editor, F97, on a copy of one map.
#[derive(uniffi::Object)]
pub struct ImageMapEditor {
    maps: Arc<ImageMaps>,
    edit: Mutex<MapEdit>,
    is_new: bool,
}

fn paths(files: Vec<String>) -> Vec<PathBuf> {
    files.into_iter().map(PathBuf::from).collect()
}

#[uniffi::export]
impl ImageMapEditor {
    pub fn name(&self) -> String {
        self.edit.lock().name.clone()
    }

    pub fn set_name(&self, name: String) {
        self.edit.lock().name = name;
    }

    pub fn notes(&self) -> String {
        self.edit.lock().notes.clone()
    }

    pub fn set_notes(&self, notes: String) {
        self.edit.lock().notes = notes;
    }

    /// The ten phonemes the navigator steps through, in its order.
    pub fn phonemes(&self) -> Vec<String> {
        MAP_PHONEMES.iter().map(|p| p.name().to_string()).collect()
    }

    pub fn phoneme(&self) -> String {
        self.edit.lock().current_phoneme().name().to_string()
    }

    /// `>`, F99.
    pub fn next(&self) {
        self.edit.lock().next();
    }

    /// `<`.
    pub fn previous(&self) {
        self.edit.lock().prev();
    }

    /// The current phoneme's picture as PNG data, F100. None shows the "no image" tile.
    pub fn image(&self) -> Option<Vec<u8>> {
        self.edit.lock().current_image().map(<[u8]>::to_vec)
    }

    /// The phonemes that have a picture.
    pub fn mapped(&self) -> Vec<String> {
        self.edit.lock().mapped().into_iter().map(|p| p.name().to_string()).collect()
    }

    /// Whether a drop of these files is taken, F101.
    pub fn accepts_drop(&self, files: Vec<String>) -> bool {
        maps::accepts_drop(&paths(files))
    }

    /// File's choice or a drop, F101. One file becomes the current phoneme's picture; with more, true asks for the Image Mapper.
    pub fn load_files(&self, files: Vec<String>) -> bool {
        self.edit.lock().load_files(&paths(files))
    }

    /// The Image Mapper's first choices, one for each of `phonemes`, F102.
    pub fn mapper_choices(&self, file_count: u32) -> Vec<ImageMapperChoice> {
        self.edit
            .lock()
            .mapper_initial(file_count as usize)
            .into_iter()
            .map(|c| match c {
                MapperChoice::Unmapped => ImageMapperChoice::Unmapped,
                MapperChoice::Mapped => ImageMapperChoice::Mapped,
                MapperChoice::File(i) => ImageMapperChoice::File { index: i as u32 },
            })
            .collect()
    }

    /// The Image Mapper's Accept.
    pub fn apply_mapper(&self, choices: Vec<ImageMapperChoice>, files: Vec<String>) {
        let choices: Vec<MapperChoice> = choices
            .into_iter()
            .map(|c| match c {
                ImageMapperChoice::Unmapped => MapperChoice::Unmapped,
                ImageMapperChoice::Mapped => MapperChoice::Mapped,
                ImageMapperChoice::File { index } => MapperChoice::File(index as usize),
            })
            .collect();
        self.edit.lock().apply_mapper(&choices, &paths(files));
    }

    /// Clear, F104.
    pub fn clear(&self) {
        let mut edit = self.edit.lock();
        let phoneme: PhonemeType = edit.current_phoneme();
        edit.clear_image(phoneme);
    }

    /// Edit, F103: the file to open in another app.
    pub fn begin_external_edit(&self) -> Result<String, crate::core::FfiError> {
        let file = self.edit.lock().begin_external_edit().map_err(|e| crate::core::FfiError::new(e.to_string()))?;
        Ok(file.to_string_lossy().into_owned())
    }

    /// The file being edited outside, if any.
    pub fn external_edit_file(&self) -> Option<String> {
        self.edit.lock().external_edit_file().map(|f| f.to_string_lossy().into_owned())
    }

    /// Done: reads the file back. A message means it would not load, and the old picture stays.
    pub fn finish_external_edit(&self) -> Option<String> {
        self.edit.lock().finish_external_edit().err()
    }

    pub fn cancel_external_edit(&self) {
        self.edit.lock().cancel_external_edit();
    }

    /// OK, F105. `overwrite` answers the question `AskOverwrite` asked.
    pub fn commit(&self, overwrite: Option<bool>) -> ImageMapCommit {
        let mut edit = self.edit.lock();
        edit.cancel_external_edit();
        let mut library = self.maps.library.lock();
        let result = library.commit_edit(&edit, overwrite, &self.maps.root());
        self.maps.publish(&library);
        match result {
            EditCommit::Saved { name, alert } => ImageMapCommit::Saved { name, alert },
            EditCommit::NeedsOverwriteAnswer(name) => ImageMapCommit::AskOverwrite { name },
        }
    }

    /// Cancel. A map New made goes again.
    pub fn cancel(&self) {
        self.edit.lock().cancel_external_edit();
        if self.is_new {
            let mut library = self.maps.library.lock();
            library.cancel_new(&self.maps.root());
            self.maps.publish(&library);
        }
    }
}
