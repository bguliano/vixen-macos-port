//! Port of Vixen.Modules/Controller/RDSController: string commands that set radio text on an RDS encoder or launch a process.

use super::launcher::{launch, launcher_arguments, LastCommands};
use super::{ConfigMismatch, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::ids;
use crate::policy::DataPolicy;
use crate::serial::{self, Parity, SerialSettings, StopBits};
use crate::xmlutil::{self, bool_el, data_root, num_el, opt_text_el, text_el, ModuleIds};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

mod http;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.Output.CommandController";

/// `Hardware`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum RdsHardware {
    #[default]
    Http,
    Mrds192,
    Mrds1322,
    Vfmt212r,
}

impl RdsHardware {
    pub fn name(self) -> &'static str {
        match self {
            RdsHardware::Http => "HTTP",
            RdsHardware::Mrds192 => "MRDS192",
            RdsHardware::Mrds1322 => "MRDS1322",
            RdsHardware::Vfmt212r => "VFMT212R",
        }
    }

    pub fn from_name(name: &str) -> RdsHardware {
        match name {
            "MRDS192" => RdsHardware::Mrds192,
            "MRDS1322" => RdsHardware::Mrds1322,
            "VFMT212R" => RdsHardware::Vfmt212r,
            _ => RdsHardware::Http,
        }
    }
}

/// `Data`. The password is stored in the clear, as Vixen stores it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdsConfig {
    pub ids: ModuleIds,
    pub bi_directional: bool,
    pub connection_mode: i32,
    pub hardware_id: RdsHardware,
    pub hide_launched_windows: bool,
    pub http_password: Option<String>,
    pub http_url: Option<String>,
    pub http_username: Option<String>,
    pub port_name: Option<String>,
    pub port_number: i32,
    pub require_http_authentication: bool,
    pub slow: bool,
}

impl Default for RdsConfig {
    fn default() -> Self {
        RdsConfig {
            ids: ModuleIds::new(ids::RDS, Uuid::nil()),
            bi_directional: false,
            connection_mode: 0,
            hardware_id: RdsHardware::Http,
            hide_launched_windows: false,
            http_password: None,
            http_url: None,
            http_username: None,
            port_name: None,
            port_number: 0,
            require_http_authentication: false,
            slow: false,
        }
    }
}

impl RdsConfig {
    /// What the setup dialog fills in for an empty URL.
    pub const DEFAULT_HTTP_URL: &'static str =
        "http://127.0.0.1:8080/?action=update_rt&update_rt={text}";

    pub fn from_xml(e: &XElement) -> RdsConfig {
        RdsConfig {
            ids: ModuleIds::read(e),
            bi_directional: xmlutil::get_bool(e, "BiDirectional"),
            connection_mode: xmlutil::get_i32(e, "ConnectionMode"),
            hardware_id: RdsHardware::from_name(&e.child_text("HardwareID").unwrap_or_default()),
            hide_launched_windows: xmlutil::get_bool(e, "HideLaunchedWindows"),
            http_password: xmlutil::get_opt_string(e, "HttpPassword"),
            http_url: xmlutil::get_opt_string(e, "HttpUrl"),
            http_username: xmlutil::get_opt_string(e, "HttpUsername"),
            port_name: xmlutil::get_opt_string(e, "PortName"),
            port_number: xmlutil::get_i32(e, "PortNumber"),
            require_http_authentication: xmlutil::get_bool(e, "RequireHTTPAuthentication"),
            slow: xmlutil::get_bool(e, "Slow"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "Data", &self.ids);
        e.push(bool_el(&ns, "BiDirectional", self.bi_directional));
        e.push(num_el(&ns, "ConnectionMode", self.connection_mode));
        e.push(text_el(&ns, "HardwareID", self.hardware_id.name()));
        e.push(bool_el(
            &ns,
            "HideLaunchedWindows",
            self.hide_launched_windows,
        ));
        e.push(opt_text_el(
            &ns,
            "HttpPassword",
            self.http_password.as_deref(),
        ));
        e.push(opt_text_el(&ns, "HttpUrl", self.http_url.as_deref()));
        e.push(opt_text_el(
            &ns,
            "HttpUsername",
            self.http_username.as_deref(),
        ));
        e.push(opt_text_el(&ns, "PortName", self.port_name.as_deref()));
        e.push(num_el(&ns, "PortNumber", self.port_number));
        e.push(bool_el(
            &ns,
            "RequireHTTPAuthentication",
            self.require_http_authentication,
        ));
        e.push(bool_el(&ns, "Slow", self.slow));
        e
    }

    /// The port to open. The C# always builds `COM{PortNumber}`. A macOS device saved in `PortName` is used instead, and a
    /// Windows number stays what it is, and reports as unavailable.
    pub fn serial_port_name(&self) -> String {
        match self.port_name.as_deref() {
            Some(name) if name.starts_with('/') => name.to_string(),
            _ => format!("COM{}", self.port_number),
        }
    }
}

/// `RdsSerialPort`'s constructor sends this, to set the board up for radio text.
pub const RADIO_TEXT_SETUP: [u8; 3] = [0xFE, 0x1F, 0xFF];

/// `RdsSerialPort.Send`'s frame: padded to 64 characters with spaces. `None` for a character above 255, where `Convert.ToByte`
/// throws.
pub fn radio_text_frame(text: &str) -> Option<Vec<u8>> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut data = Vec::with_capacity(units.len().max(64) + 3);
    data.push(0xFE);
    data.push(0x20);
    for u in &units {
        data.push(u8::try_from(*u).ok()?);
    }
    data.extend(std::iter::repeat_n(
        0x20,
        64usize.saturating_sub(units.len()),
    ));
    data.push(0xFF);
    Some(data)
}

/// `HttpUtility.UrlEncode`: space becomes `+`, and everything but letters, digits and `-_.!*()` becomes lower-case `%xx`.
pub fn url_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        match b {
            b' ' => out.push('+'),
            b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'*'
            | b'('
            | b')' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02x}")),
        }
    }
    out
}

/// `DateTime.Now.ToShortTimeString()` in the en-US format.
fn short_time_string() -> String {
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are to live locals of the right types.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        libc::localtime_r(&now, &mut tm);
    }
    let hour = match tm.tm_hour % 12 {
        0 => 12,
        h => h,
    };
    format!(
        "{}:{:02} {}",
        hour,
        tm.tm_min,
        if tm.tm_hour < 12 { "AM" } else { "PM" }
    )
}

/// The URL for a text: `{text}` and `{time}` filled in. The C# no longer lowercases it.
pub fn build_url(template: &str, text: &str, time: &str) -> String {
    template
        .replace("{text}", &url_encode(text))
        .replace("{time}", &url_encode(time))
}

/// `lastRdsText`, which the C# keeps static: one per process, not one per controller.
static LAST_RDS_TEXT: Mutex<String> = Mutex::new(String::new());

/// `Module.Send`. Returns whether a web request was started. Serial hardware blocks the controller's thread for the writes, as
/// it does in the C#.
fn send_radio_text(
    config: &RdsConfig,
    text: &str,
    force: bool,
    shared: &Arc<ModuleShared>,
    workers: &Mutex<Vec<HttpWorker>>,
) -> Result<bool, String> {
    {
        let mut last = LAST_RDS_TEXT.lock();
        if !force && *last == text {
            return Ok(false);
        }
        *last = text.to_string();
    }
    match config.hardware_id {
        RdsHardware::Mrds192 | RdsHardware::Mrds1322 => {
            let settings = SerialSettings {
                port_name: Some(config.serial_port_name()),
                baud_rate: if config.slow { 2400 } else { 19200 },
                parity: Parity::None,
                data_bits: 8,
                stop_bits: StopBits::One,
            };
            send_serial(&settings, text, shared)?;
            Ok(false)
        }
        RdsHardware::Vfmt212r | RdsHardware::Http => {
            let url = build_url(
                config.http_url.as_deref().unwrap_or(""),
                text,
                &short_time_string(),
            );
            let credentials = config.require_http_authentication.then(|| {
                (
                    config.http_username.clone().unwrap_or_default(),
                    config.http_password.clone().unwrap_or_default(),
                )
            });
            let shared = shared.clone();
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = stop.clone();
            let started = std::thread::Builder::new()
                .name("rds-http".to_string())
                .spawn(move || {
                    let result = http::get(
                        &url,
                        credentials.as_ref().map(|(u, p)| (u.as_str(), p.as_str())),
                        &worker_stop,
                        Some(shared.stop_flag()),
                    );
                    match result {
                        Ok((status, _)) if (200..300).contains(&status) => {}
                        Ok((status, reason)) => {
                            shared.record_error(format!(
                                "Web request to {url} failed with status {status} : {reason}"
                            ));
                            LAST_RDS_TEXT.lock().clear();
                        }
                        Err(http::Error::Cancelled) => {
                            LAST_RDS_TEXT.lock().clear();
                        }
                        Err(http::Error::Transport(e)) => {
                            shared.record_error(e);
                            LAST_RDS_TEXT.lock().clear();
                        }
                    }
                });
            let thread = started.map_err(|e| e.to_string())?;
            let mut workers = workers.lock();
            // Reap completed jobs so a long show retains only active requests.
            let mut i = 0;
            while i < workers.len() {
                if workers[i].thread.is_finished() {
                    let _ = workers.swap_remove(i).thread.join();
                } else {
                    i += 1;
                }
            }
            workers.push(HttpWorker { stop, thread });
            Ok(true)
        }
    }
}

/// The setup dialog's test button, waiting for the real result: F124 reports what happened, where the C# reports that a request
/// was started. Blocks, so it never runs on the main thread.
pub fn send_test_text(config: &RdsConfig, text: &str) -> Result<(), String> {
    let shared = ModuleShared::new();
    match config.hardware_id {
        RdsHardware::Mrds192 | RdsHardware::Mrds1322 => {
            let settings = SerialSettings {
                port_name: Some(config.serial_port_name()),
                baud_rate: if config.slow { 2400 } else { 19200 },
                parity: Parity::None,
                data_bits: 8,
                stop_bits: StopBits::One,
            };
            send_serial(&settings, text, &shared)
        }
        RdsHardware::Vfmt212r | RdsHardware::Http => {
            let url = build_url(
                config.http_url.as_deref().unwrap_or(""),
                text,
                &short_time_string(),
            );
            let credentials = config.require_http_authentication.then(|| {
                (
                    config.http_username.clone().unwrap_or_default(),
                    config.http_password.clone().unwrap_or_default(),
                )
            });
            match http::get(
                &url,
                credentials.as_ref().map(|(u, p)| (u.as_str(), p.as_str())),
                &AtomicBool::new(false),
                None,
            )
            .map_err(|e| match e {
                http::Error::Cancelled => "HTTP request cancelled".into(),
                http::Error::Transport(message) => message,
            })? {
                (status, _) if (200..300).contains(&status) => Ok(()),
                (status, reason) => Err(format!(
                    "Web request to {url} failed with status {status} : {reason}"
                )),
            }
        }
    }
}

/// `new RdsSerialPort(...)` then `Send`: opens the port for each of the two writes, as the C# does.
fn send_serial(
    settings: &SerialSettings,
    text: &str,
    shared: &Arc<ModuleShared>,
) -> Result<(), String> {
    let stop = shared.stop_flag();
    let mut port = serial::open_port(settings, false, false).map_err(|e| e.to_string())?;
    serial::write_all(port.as_mut(), &RADIO_TEXT_SETUP, None, stop).map_err(|e| e.to_string())?;
    // `SerialPort.Close` waits for what was written to go out. The next open flushes the queue.
    port.flush().map_err(|e| e.to_string())?;
    drop(port);
    let frame = radio_text_frame(text)
        .ok_or_else(|| format!("Radio text has a character the encoder can't take: {text}"))?;
    let mut port = serial::open_port(settings, false, false).map_err(|e| e.to_string())?;
    serial::write_all(port.as_mut(), &frame, None, stop).map_err(|e| e.to_string())?;
    port.flush().map_err(|e| e.to_string())?;
    shared.count_packet(frame.len());
    Ok(())
}

struct HttpWorker {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

/// `Module` in the RDS controller.
pub struct RdsModule {
    config: RdsConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    last: LastCommands,
    workers: Mutex<Vec<HttpWorker>>,
}

impl RdsModule {
    pub fn new(config: RdsConfig) -> RdsModule {
        RdsModule {
            config,
            shared: ModuleShared::new(),
            output_count: 0,
            last: LastCommands::default(),
            workers: Mutex::new(Vec::new()),
        }
    }

    /// The setup dialog's test button.
    pub fn send_test(&self, text: &str) -> Result<bool, String> {
        send_radio_text(&self.config, text, true, &self.shared, &self.workers)
    }
}

impl RdsModule {
    fn cancel_http(&self) {
        self.shared.request_stop(true);
        let workers = std::mem::take(&mut *self.workers.lock());
        for worker in &workers {
            worker.stop.store(true, Ordering::Relaxed);
        }
        for worker in workers {
            let _ = worker.thread.join();
        }
    }
}

impl Drop for RdsModule {
    fn drop(&mut self) {
        self.cancel_http();
    }
}

impl OutputModule for RdsModule {
    fn type_id(&self) -> Uuid {
        ids::RDS
    }

    fn start(&mut self) {
        self.shared.request_stop(false);
        self.shared.set_status(ModuleStatus::Running);
    }

    fn stop(&mut self) {
        self.cancel_http();
        self.shared.set_status(ModuleStatus::Stopped);
    }

    fn update(&mut self, commands: &[OutputCommand]) {
        for (index, command) in commands.iter().enumerate() {
            let OutputCommand::Str(value) = command else {
                self.last.replace(index, None);
                continue;
            };
            let last = self.last.replace(index, Some(value.clone()));
            let command_type = value.split('|').next().unwrap_or("");
            if command_type.eq_ignore_ascii_case("RDS") {
                // The index is out of range in the C#, and the exception ends this frame's loop.
                let Some(text) = value.split('|').nth(1) else {
                    self.shared
                        .record_error(format!("Malformed RDS command: {value}"));
                    return;
                };
                if let Err(e) =
                    send_radio_text(&self.config, text, false, &self.shared, &self.workers)
                {
                    self.shared.record_error(e);
                    return;
                }
            } else if command_type.eq_ignore_ascii_case("LAUNCHER") {
                if last.as_deref() == Some(&**value) {
                    continue;
                }
                match launcher_arguments(value) {
                    Some((executable, arguments)) => launch(
                        self.config.hide_launched_windows,
                        executable,
                        arguments,
                        &self.shared,
                    ),
                    None => {
                        self.shared
                            .record_error(format!("Malformed launcher command: {value}"));
                        return;
                    }
                }
            }
        }
    }

    fn output_count(&self) -> usize {
        self.output_count
    }

    fn set_output_count(&mut self, count: usize) {
        self.output_count = count;
    }

    fn config(&self) -> ControllerConfig {
        ControllerConfig::Rds(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::Rds(config) = config else {
            return Err(ConfigMismatch);
        };
        self.config = config;
        Ok(())
    }

    fn data_policy(&self) -> DataPolicy {
        DataPolicy::DISCARD_EXCESS_8BIT
    }

    fn shared(&self) -> &Arc<ModuleShared> {
        &self.shared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radio_text_frame_is_padded_to_64() {
        let f = radio_text_frame("Hi").unwrap();
        assert_eq!(f.len(), 67);
        assert_eq!(&f[..4], &[0xFE, 0x20, b'H', b'i']);
        assert!(f[4..66].iter().all(|b| *b == 0x20));
        assert_eq!(f[66], 0xFF);
    }

    #[test]
    fn long_radio_text_isnt_cut() {
        let text = "x".repeat(70);
        assert_eq!(radio_text_frame(&text).unwrap().len(), 73);
    }

    #[test]
    fn a_character_above_255_is_an_error() {
        assert!(radio_text_frame("\u{e9}").is_some());
        assert!(radio_text_frame("\u{263a}").is_none());
    }

    #[test]
    fn url_encoding_matches_http_utility() {
        assert_eq!(url_encode("a b&c=d/e"), "a+b%26c%3dd%2fe");
        assert_eq!(url_encode("-_.!*()"), "-_.!*()");
        assert_eq!(url_encode("\u{e9}"), "%c3%a9");
    }

    #[test]
    fn url_template_fills_text_and_time() {
        assert_eq!(
            build_url("http://h/?a={text}&b={time}", "Jingle Bells", "3:05 PM"),
            "http://h/?a=Jingle+Bells&b=3%3a05+PM"
        );
    }

    #[test]
    fn serial_name_keeps_a_windows_number_unavailable() {
        let c = RdsConfig {
            port_number: 4,
            ..Default::default()
        };
        assert_eq!(c.serial_port_name(), "COM4");
        let c = RdsConfig {
            port_name: Some("/dev/cu.usbserial".to_string()),
            port_number: 4,
            ..Default::default()
        };
        assert_eq!(c.serial_port_name(), "/dev/cu.usbserial");
    }

    #[test]
    fn serial_hardware_on_a_windows_port_records_an_error_and_never_crashes() {
        let mut m = RdsModule::new(RdsConfig {
            hardware_id: RdsHardware::Mrds1322,
            port_number: 5,
            ..Default::default()
        });
        m.start();
        m.update(&[OutputCommand::Str(Arc::from("RDS|serial on windows port"))]);
        assert!(m.shared().last_error().unwrap().contains("COM5"));
    }

    #[test]
    fn short_time_looks_like_en_us() {
        let t = short_time_string();
        assert!(t.ends_with(" AM") || t.ends_with(" PM"), "{t}");
        assert!(t.contains(':'));
    }

    #[test]
    fn malformed_commands_record_an_error() {
        let mut m = RdsModule::new(RdsConfig::default());
        m.start();
        m.update(&[OutputCommand::Str(Arc::from("RDS"))]);
        assert!(m.shared().last_error().unwrap().contains("Malformed"));
    }
}
