//! Administration's web configuration sheet and bundled-server lifetime.
use crate::core::{FfiError, VixenCore};
use std::sync::Arc;
use vixen_session::web::{Settings, WebService};

#[derive(uniffi::Record)]
pub struct WebServerSettings {
    pub port: u16,
    pub enabled: bool,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(uniffi::Object)]
pub struct WebServerSession { inner: Arc<WebService> }

#[uniffi::export]
impl VixenCore {
    pub fn web_server(&self, wwwroot: String) -> Arc<WebServerSession> {
        Arc::new(WebServerSession { inner: self.inner.initialize_web(wwwroot.into()) })
    }
}

#[uniffi::export]
impl WebServerSession {
    pub fn settings(&self) -> WebServerSettings {
        let settings = self.inner.settings();
        WebServerSettings { port: settings.port, enabled: settings.enabled,
            running: self.inner.is_running(), error: self.inner.error() }
    }
    pub fn save(&self, port: u16, enabled: bool) -> Result<(), FfiError> {
        self.inner.save(Settings { port, enabled }).map_err(FfiError::new)
    }
}
