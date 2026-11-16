//! A parsed xModel file is a draft until a model is selected and successfully imported.
use super::{now_text, settings, CustomPropEditor};
use crate::core::FfiError;
use parking_lot::Mutex;
use std::{path::Path, sync::Arc};
use vixen_prop::{xmodel::XModelFile, PropEditor};

#[derive(uniffi::Record, Clone)]
pub struct XModelChoice {
    pub index: u32,
    pub display_name: String,
    pub model_type: String,
    pub supported: bool,
}
#[derive(uniffi::Record)]
pub struct XModelImported {
    pub editor: Arc<CustomPropEditor>,
    pub warnings: Vec<String>,
}
#[derive(uniffi::Object)]
pub struct XModelImport {
    file: XModelFile,
}
#[uniffi::export]
impl XModelImport {
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, FfiError> {
        Ok(Arc::new(Self {
            file: XModelFile::open(Path::new(&path))?,
        }))
    }
    pub fn choices(&self) -> Vec<XModelChoice> {
        self.file
            .candidates()
            .iter()
            .map(|c| XModelChoice {
                index: c.index as u32,
                display_name: c.display_name.clone(),
                model_type: c.model_type.clone(),
                supported: c.supported,
            })
            .collect()
    }
    pub fn import_model(&self, index: u32) -> Result<XModelImported, FfiError> {
        let imported = self.file.import(index as usize, &now_text())?;
        Ok(XModelImported {
            editor: Arc::new(CustomPropEditor {
                inner: Mutex::new(PropEditor::imported_document(imported.prop, settings())),
            }),
            warnings: imported.warnings,
        })
    }
}
