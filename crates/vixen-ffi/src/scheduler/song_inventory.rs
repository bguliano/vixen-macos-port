use crate::core::FfiError;
use parking_lot::Mutex;
use std::sync::Arc;
use vixen_session::song_inventory as songs;
#[derive(uniffi::Record)]
pub struct SongFetch {
    pub body: String,
    pub error: Option<String>,
}
#[uniffi::export(callback_interface)]
pub trait SongInventoryClient: Send + Sync {
    fn get(&self, url: String) -> SongFetch;
}
struct Transport(Box<dyn SongInventoryClient>);
impl songs::InventoryTransport for Transport {
    fn get(&self, url: &str) -> Result<String, String> {
        let response = self.0.get(url.into());
        match response.error {
            Some(error) => Err(error),
            None => Ok(response.body),
        }
    }
}
#[derive(Clone, uniffi::Record)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub creator: String,
    pub link: Option<String>,
    pub download: Option<String>,
}
impl From<&songs::Song> for SongInfo {
    fn from(s: &songs::Song) -> Self {
        Self {
            title: s.title.clone(),
            artist: s.artist.clone(),
            creator: s.creator.clone(),
            link: s.link.clone(),
            download: s.download.clone(),
        }
    }
}
#[derive(uniffi::Record)]
pub struct SongInventoryView {
    pub vendors: Vec<String>,
    pub provider: Option<u32>,
    pub vendor_name: String,
    pub website: Option<String>,
    pub selected: Option<SongInfo>,
}
#[derive(uniffi::Record)]
pub struct SongInventoryRow {
    pub path: String,
    pub name: String,
    pub song: bool,
    pub child_count: u32,
}
#[derive(uniffi::Record)]
pub struct SongInventoryError {
    pub title: String,
    pub message: String,
}
#[derive(uniffi::Record)]
pub struct SongInventorySettings {
    pub vendor_url: String,
    pub timeout_seconds: u32,
    pub user_agent: String,
}
#[uniffi::export]
pub fn song_inventory_settings() -> SongInventorySettings {
    SongInventorySettings {
        vendor_url: songs::VENDOR_URL.into(),
        timeout_seconds: songs::TIMEOUT_SECONDS,
        user_agent: songs::USER_AGENT.into(),
    }
}
#[derive(uniffi::Object)]
pub struct SongInventoryBrowser {
    state: Mutex<songs::Browser>,
}
#[uniffi::export]
impl SongInventoryBrowser {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(songs::Browser::default()),
        })
    }
    pub fn load(
        &self,
        client: Box<dyn SongInventoryClient>,
        vendor_url: Option<String>,
    ) -> Vec<SongInventoryError> {
        let mut errors = Vec::new();
        let loaded = songs::load_inventories(
            &Transport(client),
            vendor_url.as_deref().unwrap_or(songs::VENDOR_URL),
            |error| {
                vixen_session::logging::warn("VixenModules.App.TimingTrackBrowser", &error.detail);
                errors.push(SongInventoryError {
                    title: error.title,
                    message: error.message,
                });
            },
        );
        self.state.lock().set_inventories(loaded);
        errors
    }
    pub fn view(&self) -> SongInventoryView {
        let state = self.state.lock();
        let vendor = state.provider.and_then(|p| state.inventories.get(p));
        SongInventoryView {
            vendors: state.inventories.iter().map(|i| i.name.clone()).collect(),
            provider: state.provider.map(|p| p as u32),
            vendor_name: vendor.map_or(String::new(), |v| v.name.clone()),
            website: vendor.and_then(|v| v.website.clone()),
            selected: state.selected.as_ref().map(SongInfo::from),
        }
    }
    pub fn select_provider(&self, index: u32) {
        self.state.lock().select_provider(index as usize);
    }
    pub fn select_song(&self, path: String) -> bool {
        self.state.lock().select_song(&path)
    }
    pub fn tree_rows(&self, parent: Option<String>) -> Vec<SongInventoryRow> {
        self.state
            .lock()
            .rows(parent.as_deref())
            .into_iter()
            .map(|r| SongInventoryRow {
                path: r.path,
                name: r.name,
                song: r.song,
                child_count: r.child_count as u32,
            })
            .collect()
    }
    pub fn download(&self, client: Box<dyn SongInventoryClient>) -> Result<String, FfiError> {
        let selected = self.state.lock().selected.clone();
        let state = songs::Browser {
            selected,
            ..Default::default()
        };
        state.download(&Transport(client)).map_err(FfiError::new)
    }
}
