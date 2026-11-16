//! The application object every window shares, and what all areas need from it.

use std::path::PathBuf;
use std::sync::Arc;
use vixen_session::app::AppCore;

/// An error shown to the user. Holds a message written for them, not a debug dump.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("{message}")]
    Failed { message: String },
}

impl FfiError {
    pub fn new(message: impl Into<String>) -> FfiError {
        FfiError::Failed { message: message.into() }
    }
}

impl From<vixen_session::SessionError> for FfiError {
    fn from(e: vixen_session::SessionError) -> FfiError {
        FfiError::new(e.to_string())
    }
}

impl From<vixen_io::IoError> for FfiError {
    fn from(e: vixen_io::IoError) -> FfiError {
        FfiError::new(e.to_string())
    }
}

#[derive(uniffi::Record)]
pub struct AppInfo {
    pub version: String,
    pub build: String,
}

#[uniffi::export]
pub fn app_info() -> AppInfo {
    AppInfo { version: env!("CARGO_PKG_VERSION").to_string(), build: "macOS port".to_string() }
}

/// The running application. There is one per process; areas add their own methods in their own modules.
#[derive(uniffi::Object)]
pub struct VixenCore {
    pub inner: Arc<AppCore>,
}

#[uniffi::export]
impl VixenCore {
    /// Opens the data folder at `data_root`, creating it if this is a new profile. `muted` means no sound and no output
    /// hardware, as tests need.
    #[uniffi::constructor]
    pub fn open(data_root: String, muted: bool) -> Result<Arc<VixenCore>, FfiError> {
        let inner = AppCore::open(PathBuf::from(data_root), muted)?;
        Ok(Arc::new(VixenCore { inner: Arc::new(inner) }))
    }

    /// The folder this profile's files live in.
    pub fn data_root(&self) -> String {
        self.inner.paths.root().to_string_lossy().into_owned()
    }
}

/// The default data folder, `~/Documents/Vixen 3`.
#[uniffi::export]
pub fn default_data_root() -> Option<String> {
    vixen_io::DataPaths::default_root().map(|p| p.to_string_lossy().into_owned())
}
