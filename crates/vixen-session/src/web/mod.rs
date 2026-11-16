//! Web server settings, profile lifetime and execution-context recovery (F65/F66).

pub mod api;

use crate::app::{AppCore, Saving};
use crate::boot::AppService;
use crate::execution::ExecutionState;
use crate::logging;
use parking_lot::Mutex;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use uuid::Uuid;
use vixen_io::{ModuleEntry, ModuleStore};
use vixen_web::Server;
use vixen_xml::XElement;

pub const MODULE_TYPE: Uuid = uuid::uuid!("8ac1cdf9-8d57-43b6-81de-93003ed2f907");
const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.WebServer";
const LOGGER: &str = "VixenModules.App.WebServer.Module";
const STARTUP_LOGGER: &str = "VixenModules.App.WebServer.Startup";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings { pub port: u16, pub enabled: bool }
impl Default for Settings {
    fn default() -> Self { Self { port: 8888, enabled: true } }
}
pub fn load_settings(store: &ModuleStore) -> Settings {
    let Some(data) = store.type_data(MODULE_TYPE).and_then(ModuleEntry::data) else { return Settings::default(); };
    let port = data.child_text("HttpPort").and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(8888).clamp(80, 64000) as u16;
    let enabled = data.child_text("IsEnabled").and_then(|v| vixen_io::dcs::parse_bool(&v)).unwrap_or(true);
    Settings { port, enabled }
}
fn encode(settings: Settings) -> XElement {
    let mut data = XElement::new(NS, "Data");
    data.push(XElement::with_text("", "ModuleInstanceId", MODULE_TYPE.to_string()));
    data.push(XElement::with_text("", "ModuleTypeId", MODULE_TYPE.to_string()));
    data.push(XElement::with_text(NS, "HttpPort", settings.port.to_string()));
    data.push(XElement::with_text(NS, "IsEnabled", vixen_io::dcs::format_bool(settings.enabled)));
    data
}
pub fn store_settings(store: &mut ModuleStore, settings: Settings) {
    let previous = encode(load_settings(store));
    let data = encode(settings);
    match store.type_data_mut(MODULE_TYPE) {
        Some(entry) => entry.merge_data(&previous, data),
        None => store.set_type_data(ModuleEntry::new("VixenModules.App.WebServer.Data, WebServer", &MODULE_TYPE, &MODULE_TYPE, "Module.App.WebServer", data)),
    }
}

#[derive(Default)]
struct Runtime { server: Option<Server>, error: Option<String> }

pub struct WebService {
    core: Weak<AppCore>,
    api: Arc<api::SessionApi>,
    wwwroot: PathBuf,
    runtime: Mutex<Runtime>,
    /// Serializes save/restart/stop; a restart finishes closing old clients before bind.
    changing: Mutex<()>,
    bind_override: Option<SocketAddr>,
}

impl WebService {
    fn new(core: &Arc<AppCore>, wwwroot: PathBuf, bind_override: Option<SocketAddr>) -> Arc<Self> {
        let service = Arc::new(Self { core: Arc::downgrade(core), api: Arc::new(api::SessionApi::new(core)), wwwroot,
            runtime: Mutex::new(Runtime::default()), changing: Mutex::new(()), bind_override });
        let weak = Arc::downgrade(&service);
        core.execution().subscribe(Box::new(move |state| {
            if state == ExecutionState::Open {
                if let Some(service) = weak.upgrade() {
                    if service.is_running() {
                        if let Some(core) = service.core.upgrade() { core.live.ensure(&core, "Web Server"); }
                    }
                }
            }
        }));
        service
    }

    pub fn settings(&self) -> Settings {
        self.core.upgrade().map(|core| load_settings(&core.system.read().module_store)).unwrap_or_default()
    }
    pub fn is_running(&self) -> bool { self.runtime.lock().server.is_some() }
    pub fn error(&self) -> Option<String> { self.runtime.lock().error.clone() }
    pub fn address(&self) -> Option<SocketAddr> { self.runtime.lock().server.as_ref().map(Server::address) }

    fn stop_locked(&self, core: &AppCore) {
        let old = self.runtime.lock().server.take();
        if let Some(mut old) = old {
            logging::info(STARTUP_LOGGER, "Webserver is stopping...");
            old.stop();
            logging::info(STARTUP_LOGGER, "Webserver stopped.");
        }
        core.live.release("Web Server");
    }

    fn apply_locked(&self, core: &AppCore) -> Result<(), String> {
        self.stop_locked(core);
        self.runtime.lock().error = None;
        let settings = load_settings(&core.system.read().module_store);
        // UI test runs must never start listeners, including when settings are saved.
        if !settings.enabled || core.launch_options().under_test { return Ok(()); }
        // Server explicitly enables IPv4-mapped dual stack, matching http://*.
        let bind = self.bind_override.unwrap_or(SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), settings.port));
        let logger = Arc::new(|line: &str| logging::info("VixenModules.App.WebServer.RequestLoggingMiddleware", line));
        match Server::start(bind, self.wwwroot.clone(), self.api.clone(), logger) {
            Ok(server) => {
                self.runtime.lock().server = Some(server);
                core.live.ensure(core, "Web Server");
                logging::info(STARTUP_LOGGER, "Webserver started.");
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                self.runtime.lock().error = Some(message.clone());
                logging::error(LOGGER, "Unable to start web _server.", Some(&message));
                Err(message)
            }
        }
    }

    /// OK always persists, even without a change. Failed writes restore the in-memory
    /// store while its write lock is held, so another area's edit cannot be rolled back.
    pub fn save(&self, settings: Settings) -> Result<(), String> {
        let _changing = self.changing.lock();
        let core = self.core.upgrade().ok_or("The profile has closed.")?;
        let settings = Settings { port: settings.port.clamp(80, 64000), ..settings };
        let _saving = Saving::begin(&core.saving_module).ok_or("Module config is already being saved.")?;
        let mut system = core.system.write();
        let changed = load_settings(&system.module_store) != settings;
        let previous = system.module_store.clone();
        store_settings(&mut system.module_store, settings);
        if let Err(error) = vixen_io::system::save_module_config_file(&system, &core.paths) {
            system.module_store = previous;
            return Err(error.to_string());
        }
        drop(system);
        if changed || (settings.enabled && !self.is_running() && self.error().is_some()) { self.apply_locked(&core)?; }
        Ok(())
    }
}

impl AppService for WebService {
    fn name(&self) -> &str { "Vixen WebServer" }
    fn start(&self, core: &AppCore) -> Result<(), String> { let _changing = self.changing.lock(); self.apply_locked(core) }
    fn stop(&self, core: &AppCore) { let _changing = self.changing.lock(); self.stop_locked(core); }
    fn reloaded(&self, core: &AppCore) { let _ = self.start(core); }
}

impl AppCore {
    pub fn initialize_web(self: &Arc<Self>, wwwroot: PathBuf) -> Arc<WebService> {
        let mut slot = self.web.lock();
        if let Some(service) = slot.as_ref() { return service.clone(); }
        let service = WebService::new(self, wwwroot, None);
        *slot = Some(service.clone());
        drop(slot);
        self.add_service(service.clone());
        service
    }
}

#[cfg(test)]
mod tests;
