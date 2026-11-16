//! The controller setup dialogs other than E1.31: Generic Serial, Dummy Lighting, DDP, Renard, DMX Pro, DMX Open, Elexol, RDS and
//! Launcher (F26 to F50, F95 to F134).

use super::serial::saved_port_note;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;
use vixen_output::command::OutputCommand;
use vixen_output::ids;
use vixen_output::modules::debug::format_commands;
use vixen_output::modules::dummy::DummyLightingView;
use vixen_output::modules::elexol::{test_connection, ElexolTestError};
use vixen_output::modules::{
    ControllerConfig, DdpConfig, DmxUsbProConfig, DummyLightingConfig, ElexolConfig, GenericSerialConfig, LauncherConfig,
    OpenDmxConfig, OpenDmxDevice, RdsConfig, RdsHardware, RenardConfig, RenderStyle,
};
use vixen_output::policy::DataPolicy;
use vixen_output::serial::{list_serial_port_details, SerialPortDetails, SerialSettings};

/// Which sheet a controller's Configure opens (F1). `SerialPortOnly` is Renard and DMX Pro, which go straight to Serial Port
/// Config. `None` is a controller with no setup, where Configure does nothing.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ControllerSetupKind {
    GenericSerial,
    DummyLighting,
    E131,
    Ddp,
    SerialPortOnly,
    OpenDmx,
    Elexol,
    Rds,
    Launcher,
    None,
}

pub fn controller_setup_kind(type_id: &Uuid) -> ControllerSetupKind {
    match *type_id {
        ids::GENERIC_SERIAL => ControllerSetupKind::GenericSerial,
        ids::DUMMY_LIGHTING => ControllerSetupKind::DummyLighting,
        ids::E131 => ControllerSetupKind::E131,
        ids::DDP => ControllerSetupKind::Ddp,
        ids::RENARD | ids::DMX_USB_PRO => ControllerSetupKind::SerialPortOnly,
        ids::OPEN_DMX => ControllerSetupKind::OpenDmx,
        ids::ELEXOL_ETHER_IO => ControllerSetupKind::Elexol,
        ids::RDS => ControllerSetupKind::Rds,
        ids::LAUNCHER => ControllerSetupKind::Launcher,
        _ => ControllerSetupKind::None,
    }
}

/// F36: why a saved configuration can't reach its hardware, for the status on the controller's row. A saved `COM9` fails on every
/// start, so this shouldn't only be a log line.
pub fn controller_configuration_problem(config: &ControllerConfig) -> Option<String> {
    let port = match config {
        ControllerConfig::GenericSerial(c) => c.serial.port_name.as_deref(),
        ControllerConfig::Renard(c) => c.serial.port_name.as_deref(),
        ControllerConfig::DmxUsbPro(c) => c.serial.port_name.as_deref(),
        _ => None,
    };
    port.filter(|p| !p.is_empty()).and_then(saved_port_note)
}

// ---- Generic Serial -----------------------------------------------------------------------------------------------------

/// F34. A box that isn't checked stores empty, whatever text it holds. Returns `Data.IsValid`, which is what `Setup` returns.
pub fn generic_serial_apply(
    data: &mut GenericSerialConfig,
    settings: SerialSettings,
    header_on: bool,
    header: &str,
    footer_on: bool,
    footer: &str,
) -> bool {
    data.header = Some(if header_on { header.to_string() } else { String::new() });
    data.footer = Some(if footer_on { footer.to_string() } else { String::new() });
    data.serial = settings;
    data.serial.is_valid()
}

// ---- Renard, DMX Pro ----------------------------------------------------------------------------------------------------

/// The settings Serial Port Config opens with on a controller that goes straight to it. DMX Pro builds them from the data, where
/// upstream passes a runtime port that's null until started (F102).
pub fn serial_only_settings(config: &ControllerConfig) -> Option<SerialSettings> {
    let settings = match config {
        ControllerConfig::Renard(c) => &c.serial,
        ControllerConfig::DmxUsbPro(c) => &c.serial,
        _ => return None,
    };
    settings.port_name.as_deref().is_some_and(|n| !n.is_empty()).then(|| settings.clone())
}

/// F100.
pub fn renard_apply(data: &mut RenardConfig, settings: SerialSettings) {
    data.serial = settings;
    // What the module's `ModuleData` setter does when it loads a data object without them.
    if data.write_timeout == 0 {
        data.write_timeout = 500;
    }
    if data.protocol_version == 0 {
        data.protocol_version = 1;
    }
}

/// F102.
pub fn dmx_pro_apply(data: &mut DmxUsbProConfig, settings: SerialSettings) {
    data.serial = settings;
}

// ---- Dummy Lighting ------------------------------------------------------------------------------------------------------

/// F44. A blank title stays blank.
pub fn dummy_apply(data: &mut DummyLightingConfig, title: &str, style: RenderStyle) {
    data.form_title = Some(title.to_string());
    data.render_style = style;
}

/// F44: what `_SetDataPolicy` chooses.
pub fn dummy_data_policy(style: RenderStyle) -> DataPolicy {
    style.data_policy()
}

/// F47: how the boxes lay out. `cells` is the number of boxes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct DummyGrid {
    pub across: u32,
    pub down: u32,
    pub cells: u32,
}

/// F47. `across` is upstream's, and `down` rounds up so the last boxes can't fall off the bottom.
pub fn dummy_grid_layout(outputs: u32, style: RenderStyle) -> DummyGrid {
    let cells = match style {
        RenderStyle::RGBMultiChannel => outputs.div_ceil(3),
        _ => outputs,
    };
    if cells == 0 {
        return DummyGrid::default();
    }
    let across = ((cells as f64).sqrt().floor() as u32).max(1);
    DummyGrid { across, down: cells.div_ceil(across), cells }
}

/// F48: one RGBA per box, left to right and top to bottom, over black. Reuses `out`.
pub fn dummy_frame_rgba(style: RenderStyle, grid: DummyGrid, view: &DummyLightingView, out: &mut Vec<u8>) {
    let cells = (grid.across * grid.down) as usize;
    out.clear();
    out.resize(cells * 4, 0);
    for pixel in out.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    match style {
        RenderStyle::Monochrome => {
            for (pixel, value) in out.chunks_exact_mut(4).zip(&view.values) {
                pixel[0] = *value;
                pixel[1] = *value;
                pixel[2] = *value;
            }
        }
        // A trailing partial group isn't drawn.
        RenderStyle::RGBMultiChannel => {
            for (pixel, rgb) in out.chunks_exact_mut(4).zip(view.values.chunks_exact(3)) {
                pixel[..3].copy_from_slice(rgb);
            }
        }
        // No command is `Color.Transparent`, which leaves the black showing.
        RenderStyle::RGBSingleChannel => {
            for (pixel, color) in out.chunks_exact_mut(4).zip(&view.colors) {
                if color.a != 0 {
                    pixel[0] = color.r;
                    pixel[1] = color.g;
                    pixel[2] = color.b;
                }
            }
        }
    }
}

/// F49: a running average since Start. Upstream shows infinity on the first update.
pub fn format_fps(fps: f64) -> String {
    let fps = if fps.is_finite() { fps } else { 0.0 };
    format!("{fps:.2} fps")
}

/// F49, from the counts: `1000 * updates / elapsedMs`.
pub fn fps_from_counts(updates: u64, elapsed_ms: u64) -> String {
    if elapsed_ms == 0 {
        return format_fps(0.0);
    }
    format_fps(1000.0 * updates as f64 / elapsed_ms as f64)
}

/// F134's text for one frame. `None` means nothing to show, and the window keeps what it has.
pub fn debug_output_text(time: &str, commands: &[OutputCommand], verbose: bool) -> Option<String> {
    let text = format_commands(time, commands, verbose);
    (!text.is_empty()).then_some(text)
}

// ---- DDP ---------------------------------------------------------------------------------------------------------------

/// What the DDP sheet edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DdpDraft {
    pub host_mode: bool,
    pub ip_text: String,
    pub host_text: String,
}

/// F96.
pub fn ddp_draft(data: &DdpConfig) -> DdpDraft {
    match data.host_name.as_deref().filter(|h| !h.is_empty()) {
        Some(host) => DdpDraft { host_mode: true, ip_text: String::new(), host_text: host.to_string() },
        None => DdpDraft {
            host_mode: false,
            ip_text: data
                .address
                .as_deref()
                .and_then(|a| a.parse::<IpAddr>().ok())
                .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
                .to_string(),
            host_text: "myhost.local".to_string(),
        },
    }
}

/// F97: whether OK is available. In IP mode a text that doesn't parse disables it, where upstream silently uses 127.0.0.1.
pub fn ddp_can_apply(draft: &DdpDraft) -> bool {
    // `Ipv4Addr` takes dotted quads and nothing else, unlike `IPAddress.TryParse`, which also takes `10.1` and IPv6.
    draft.host_mode || draft.ip_text.parse::<Ipv4Addr>().is_ok()
}

/// F97. `resolve` looks the name up, which can take seconds on a `.local` name, so this stays off the main thread.
pub fn ddp_apply(
    data: &mut DdpConfig,
    draft: &DdpDraft,
    resolve: impl FnOnce(&str) -> Option<Ipv4Addr>,
) -> Result<(), String> {
    if draft.host_mode {
        let host = draft.host_text.trim();
        let Some(address) = resolve(host) else {
            return Err(format!(
                "Could not resolve host name '{host}' to an IP address. Check the spelling and your network connection, or switch to IP Address mode."
            ));
        };
        data.address = Some(address.to_string());
        data.host_name = Some(host.to_string());
    } else {
        let ip: IpAddr = draft.ip_text.parse().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        data.address = Some(ip.to_string());
        data.host_name = None;
    }
    Ok(())
}

// ---- DMX Open ------------------------------------------------------------------------------------------------------------

/// F105: the FTDI adapters among the serial ports, described as D2XX would, so a device saved on Windows still matches.
pub fn open_dmx_devices_from(details: &[SerialPortDetails]) -> Vec<OpenDmxDevice> {
    vixen_output::modules::open_dmx::list_devices_from(details)
}

pub fn open_dmx_devices() -> Vec<OpenDmxDevice> {
    open_dmx_devices_from(&list_serial_port_details())
}

/// `Device.ToString()`.
pub fn open_dmx_device_label(d: &OpenDmxDevice) -> String {
    format!(
        "{} - {} - {}",
        d.description.as_deref().unwrap_or(""),
        d.serial_number.as_deref().unwrap_or(""),
        d.id.as_deref().unwrap_or("")
    )
}

/// F105: the saved device by serial, id and description. Otherwise the first, when there are any.
pub fn open_dmx_selection(saved: Option<&OpenDmxDevice>, devices: &[OpenDmxDevice]) -> Option<usize> {
    if devices.is_empty() {
        return None;
    }
    let matched = saved.and_then(|s| {
        devices
            .iter()
            .position(|d| d.serial_number == s.serial_number && d.id == s.id && d.description == s.description)
    });
    Some(matched.unwrap_or(0))
}

/// F106. Without devices there's nothing to choose, and the saved one stays.
pub fn open_dmx_apply(data: &mut OpenDmxConfig, devices: &[OpenDmxDevice], selection: Option<usize>) {
    if let Some(device) = selection.and_then(|i| devices.get(i)) {
        data.device = Some(device.clone());
    }
}

// ---- Elexol -------------------------------------------------------------------------------------------------------------

/// What the Elexol sheet edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElexolDraft {
    pub ip_text: String,
    pub port_text: String,
    pub min_intensity: i32,
}

/// F110, F111, F113.
pub fn elexol_draft(data: &ElexolConfig) -> ElexolDraft {
    ElexolDraft {
        ip_text: data.address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
        port_text: if data.port <= 0 { "2424".to_string() } else { data.port.to_string() },
        // A saved value shows as it is, and a controller nobody has configured starts on upstream's 1.
        min_intensity: if data.address.is_some() { data.minimum_intensity.clamp(0, 255) } else { 1 },
    }
}

/// F114. Upstream's `int.Parse` throws on a bad port, and "Invalid Port." is new text.
pub fn elexol_apply(data: &mut ElexolConfig, draft: &ElexolDraft) -> Result<(), String> {
    let ip: IpAddr = draft.ip_text.parse().map_err(|_| "Invalid IP Address.".to_string())?;
    let port = draft.port_text.trim().parse::<i32>().ok().filter(|p| (1..=65535).contains(p)).ok_or("Invalid Port.")?;
    data.address = Some(ip.to_string());
    data.port = port;
    data.minimum_intensity = draft.min_intensity.clamp(0, 255);
    Ok(())
}

/// F112's result, as the alert's text. Blocks for up to a second, so it stays off the main thread. Only ever runs when the user
/// clicks Test.
pub fn elexol_test(ip: &str, port: &str) -> String {
    let parsed: Result<u16, _> = port.trim().parse();
    let outcome = match parsed {
        Ok(port) => test_connection(ip.trim(), port),
        Err(_) => Err(ElexolTestError::Socket("Invalid Port.".to_string())),
    };
    elexol_test_text(outcome)
}

pub fn elexol_test_text(outcome: Result<(), ElexolTestError>) -> String {
    match outcome {
        Ok(()) => "Connection Successful!".to_string(),
        Err(ElexolTestError::NoReply) => "No Reply.".to_string(),
        // Upstream prints the bytes it sent here. This prints what came back.
        Err(ElexolTestError::WrongReply(bytes)) => {
            let printed: String = bytes.iter().map(|b| format!("[0x{b:02X}]")).collect();
            format!("Device replied, but with incorrect data:\n{printed}")
        }
        Err(ElexolTestError::Socket(message)) => format!("An error occured testing the device\n {message}"),
    }
}

// ---- RDS ---------------------------------------------------------------------------------------------------------------

/// What each hardware choice allows (F118).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdsFieldRules {
    pub test_text_max: u32,
    pub url_enabled: bool,
    pub credentials_enabled: bool,
    pub auth_enabled: bool,
    /// V-FMT212R requires authentication.
    pub auth_forced: bool,
    pub serial_enabled: bool,
    /// The URL to replace the current one with.
    pub url_replacement: Option<String>,
}

const VFMT212R_URL_SUFFIX: &str = "?action=update_rt&update_rt={text}";

/// F118.
pub fn rds_field_rules(hardware: RdsHardware, url: &str) -> RdsFieldRules {
    match hardware {
        RdsHardware::Mrds192 | RdsHardware::Mrds1322 => RdsFieldRules {
            test_text_max: 8,
            url_enabled: false,
            credentials_enabled: false,
            auth_enabled: false,
            auth_forced: false,
            serial_enabled: true,
            url_replacement: None,
        },
        RdsHardware::Vfmt212r => RdsFieldRules {
            test_text_max: 64,
            url_enabled: true,
            credentials_enabled: true,
            auth_enabled: false,
            auth_forced: true,
            serial_enabled: false,
            url_replacement: (!url.to_lowercase().ends_with(VFMT212R_URL_SUFFIX))
                .then(|| RdsConfig::DEFAULT_HTTP_URL.to_string()),
        },
        RdsHardware::Http => RdsFieldRules {
            test_text_max: 128,
            url_enabled: true,
            credentials_enabled: true,
            auth_enabled: true,
            auth_forced: false,
            serial_enabled: true,
            url_replacement: None,
        },
    }
}

/// What the RDS sheet edits. The hidden Launcher Options stay in the data (F128).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdsDraft {
    pub hardware: RdsHardware,
    /// F117: nothing shows or reads it, and it only ever changes as an MRDS radio goes on or off.
    pub connection_mode: i32,
    /// `None` when nothing that exists here is selected.
    pub port_path: Option<String>,
    pub url: String,
    pub requires_auth: bool,
    pub username: String,
    pub password: String,
    pub bi_directional: bool,
    pub slow: bool,
}

/// F117 to F127, with the field rules run at open, where upstream misses them for a saved MRDS192.
pub fn rds_draft(data: &RdsConfig, ports: &[String]) -> RdsDraft {
    // The designer has MRDS192 checked, so its handler runs as any other saved choice replaces it (F117). Unlike upstream, the field rules run for it too (F118).
    let mut draft = RdsDraft {
        hardware: RdsHardware::Mrds192,
        connection_mode: data.connection_mode,
        port_path: data.port_name.clone().filter(|p| ports.contains(p)),
        url: data.http_url.clone().unwrap_or_else(|| RdsConfig::DEFAULT_HTTP_URL.to_string()),
        requires_auth: data.require_http_authentication,
        username: data.http_username.clone().unwrap_or_default(),
        password: data.http_password.clone().unwrap_or_default(),
        bi_directional: data.bi_directional,
        slow: data.slow,
    };
    // A saved MRDS192 fires no event, so `ConnectionMode` stays as saved. Its field rules change nothing in the draft.
    if data.hardware_id != RdsHardware::Mrds192 {
        rds_set_hardware(&mut draft, data.hardware_id);
    }
    draft
}

/// One radio's `CheckedChanged` handler and `SetFormDefaults`.
fn apply_radio(draft: &mut RdsDraft, radio: RdsHardware) {
    match radio {
        RdsHardware::Mrds192 => draft.connection_mode = 0,
        RdsHardware::Mrds1322 => draft.connection_mode = 1,
        _ => {}
    }
    let rules = rds_field_rules(radio, &draft.url);
    if rules.auth_forced {
        draft.requires_auth = true;
    }
    if let Some(url) = rules.url_replacement {
        draft.url = url;
    }
}

/// F117, F118: a radio's handler also runs as it goes off, before the new one's. The hardware is the new radio's, and what only the old one changes stays changed.
pub fn rds_set_hardware(draft: &mut RdsDraft, hardware: RdsHardware) {
    apply_radio(draft, draft.hardware);
    apply_radio(draft, hardware);
    draft.hardware = hardware;
}

pub fn rds_apply(data: &mut RdsConfig, draft: &RdsDraft) {
    data.hardware_id = draft.hardware;
    data.connection_mode = draft.connection_mode;
    if draft.port_path.is_some() {
        data.port_name = draft.port_path.clone();
    }
    data.http_url = Some(draft.url.clone());
    data.require_http_authentication = draft.requires_auth;
    data.http_username = keep_nil_if_empty(&data.http_username, &draft.username);
    data.http_password = keep_nil_if_empty(&data.http_password, &draft.password);
    data.bi_directional = draft.bi_directional;
    data.slow = draft.slow;
}

/// A null text box that stays empty fires no change event, so the member stays null.
fn keep_nil_if_empty(old: &Option<String>, text: &str) -> Option<String> {
    if text.is_empty() && old.is_none() {
        None
    } else {
        Some(text.to_string())
    }
}

/// F124: the text the status strip shows after Send. Reports what really happened.
pub fn rds_send_status(result: &Result<(), String>) -> String {
    match result {
        Ok(()) => "Data Sent".to_string(),
        Err(message) => format!("Data Not Sent: {message}"),
    }
}

// ---- Launcher -----------------------------------------------------------------------------------------------------------

/// F132.
pub fn launcher_apply(data: &mut LauncherConfig, hide_launched_windows: bool) {
    data.hide_launched_windows = hide_launched_windows;
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::Color;
    use vixen_output::serial::{Parity, StopBits};
    use vixen_xml::XElement;

    const OWNER_GENERIC_SERIAL: &str = r#"<Data xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Output.GenericSerial" xmlns:i="http://www.w3.org/2001/XMLSchema-instance">
    <ModuleInstanceId xmlns="">ee79fee2-2d7d-4984-a178-288cee39a361</ModuleInstanceId>
    <ModuleTypeId xmlns="">d22c9233-16e9-4008-a215-e577f78a7a2e</ModuleTypeId>
    <BaudRate>9600</BaudRate>
    <DataBits>8</DataBits>
    <Footer />
    <Header />
    <Parity>None</Parity>
    <PortName>COM9</PortName>
    <StopBits>One</StopBits>
  </Data>"#;

    fn settings(port: &str) -> SerialSettings {
        SerialSettings {
            port_name: Some(port.to_string()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        }
    }

    #[test]
    fn each_module_opens_its_own_sheet() {
        assert_eq!(controller_setup_kind(&ids::GENERIC_SERIAL), ControllerSetupKind::GenericSerial);
        assert_eq!(controller_setup_kind(&ids::RENARD), ControllerSetupKind::SerialPortOnly);
        assert_eq!(controller_setup_kind(&ids::DMX_USB_PRO), ControllerSetupKind::SerialPortOnly);
        assert_eq!(controller_setup_kind(&ids::DEBUG_CONTROLLER), ControllerSetupKind::None);
        assert_eq!(controller_setup_kind(&Uuid::nil()), ControllerSetupKind::None);
    }

    #[test]
    fn the_owners_generic_serial_round_trips() {
        let e = XElement::parse(OWNER_GENERIC_SERIAL).unwrap();
        let c = GenericSerialConfig::from_xml(&e);
        assert_eq!(c.serial, settings("COM9"));
        assert_eq!(c.header.as_deref(), Some(""));
        assert!(c.to_xml().semantic_eq(&e));
    }

    #[test]
    fn a_box_that_is_not_checked_stores_empty() {
        let mut c = GenericSerialConfig::default();
        assert!(generic_serial_apply(&mut c, settings("/dev/cu.x"), false, "kept in the box", true, "Z"));
        assert_eq!(c.header.as_deref(), Some(""));
        assert_eq!(c.footer.as_deref(), Some("Z"));
        assert_eq!(c.serial.port_name.as_deref(), Some("/dev/cu.x"));
    }

    #[test]
    fn is_valid_follows_the_data() {
        for (name, change) in [
            ("no port", (|s: &mut SerialSettings| s.port_name = None) as fn(&mut SerialSettings)),
            ("no baud", |s| s.baud_rate = 0),
            ("no bits", |s| s.data_bits = 0),
            ("no stop", |s| s.stop_bits = StopBits::None),
        ] {
            let mut s = settings("x");
            change(&mut s);
            assert!(!generic_serial_apply(&mut GenericSerialConfig::default(), s, false, "", false, ""), "{name}");
        }
    }

    #[test]
    fn a_saved_windows_port_is_reported() {
        let c = ControllerConfig::GenericSerial(GenericSerialConfig { serial: settings("COM9"), ..Default::default() });
        assert!(controller_configuration_problem(&c).unwrap().contains("COM9"));
        let none = ControllerConfig::Launcher(LauncherConfig::default());
        assert_eq!(controller_configuration_problem(&none), None);
    }

    #[test]
    fn dummy_grid_matches_the_table() {
        for (outputs, across, down) in
            [(1, 1, 1), (2, 1, 2), (3, 1, 3), (4, 2, 2), (5, 2, 3), (10, 3, 4), (17, 4, 5), (25, 5, 5), (3351, 57, 59)]
        {
            let g = dummy_grid_layout(outputs, RenderStyle::Monochrome);
            assert_eq!((g.across, g.down, g.cells), (across, down, outputs), "{outputs}");
        }
        assert_eq!(dummy_grid_layout(0, RenderStyle::Monochrome), DummyGrid::default());
    }

    #[test]
    fn every_three_outputs_make_one_box() {
        let g = dummy_grid_layout(10, RenderStyle::RGBMultiChannel);
        assert_eq!((g.cells, g.across), (4, 2));
        let view = DummyLightingView { values: (1..=10).collect(), ..Default::default() };
        let mut out = Vec::new();
        dummy_frame_rgba(RenderStyle::RGBMultiChannel, g, &view, &mut out);
        assert_eq!(&out[0..4], &[1, 2, 3, 255]);
        assert_eq!(&out[8..12], &[7, 8, 9, 255]);
        // Output 10 has no group of its own, so the fourth box stays black.
        assert_eq!(&out[12..16], &[0, 0, 0, 255]);
    }

    #[test]
    fn monochrome_is_grey_and_no_colour_is_black() {
        let g = dummy_grid_layout(1, RenderStyle::Monochrome);
        let mut out = Vec::new();
        dummy_frame_rgba(RenderStyle::Monochrome, g, &DummyLightingView { values: vec![128], ..Default::default() }, &mut out);
        assert_eq!(out, [128, 128, 128, 255]);
        let view = DummyLightingView { colors: vec![Color::TRANSPARENT, Color::RED], ..Default::default() };
        dummy_frame_rgba(RenderStyle::RGBSingleChannel, dummy_grid_layout(2, RenderStyle::RGBSingleChannel), &view, &mut out);
        assert_eq!(out, [0, 0, 0, 255, 255, 0, 0, 255]);
    }

    #[test]
    fn frames_per_second_read_like_upstream() {
        assert_eq!(fps_from_counts(100, 1000), "100.00 fps");
        assert_eq!(fps_from_counts(1, 0), "0.00 fps");
        assert_eq!(format_fps(f64::INFINITY), "0.00 fps");
    }

    #[test]
    fn dummy_style_sets_the_policy() {
        assert_eq!(dummy_data_policy(RenderStyle::Monochrome), DataPolicy::HIGHEST_WINS_8BIT);
        assert_eq!(dummy_data_policy(RenderStyle::RGBMultiChannel), DataPolicy::AVERAGE_8BIT);
        assert_eq!(dummy_data_policy(RenderStyle::RGBSingleChannel), DataPolicy::ONE_CHANNEL_COLOR);
    }

    #[test]
    fn dummy_blank_title_round_trips() {
        let mut c = DummyLightingConfig::default();
        assert_eq!(c.form_title.as_deref(), Some("Dummy Lighting Controller"));
        dummy_apply(&mut c, "", RenderStyle::RGBSingleChannel);
        let xml = c.to_xml();
        assert_eq!(xml.child("FormTitle").unwrap().text(), "");
        assert!(!xml.child("FormTitle").unwrap().nil);
        let back = DummyLightingConfig::from_xml(&xml);
        assert_eq!(back.form_title.as_deref(), Some(""));
        assert_eq!(back.render_style, RenderStyle::RGBSingleChannel);
    }

    #[test]
    fn ddp_opens_in_the_mode_it_was_saved_in() {
        let d = ddp_draft(&DdpConfig::default());
        assert!(!d.host_mode);
        assert_eq!(d.ip_text, "127.0.0.1");
        assert_eq!(d.host_text, "myhost.local");
        let d = ddp_draft(&DdpConfig { host_name: Some("tree.local".to_string()), ..Default::default() });
        assert!(d.host_mode);
        assert_eq!(d.ip_text, "");
        assert_eq!(d.host_text, "tree.local");
        let d = ddp_draft(&DdpConfig { address: Some("junk".to_string()), ..Default::default() });
        assert_eq!(d.ip_text, "127.0.0.1");
    }

    #[test]
    fn ddp_ip_mode_clears_the_host_name() {
        let mut c = DdpConfig { host_name: Some("old".to_string()), ..Default::default() };
        let d = DdpDraft { host_mode: false, ip_text: "10.0.0.9".to_string(), host_text: "x".to_string() };
        ddp_apply(&mut c, &d, |_| None).unwrap();
        assert_eq!(c.address.as_deref(), Some("10.0.0.9"));
        assert_eq!(c.host_name, None);
    }

    #[test]
    fn ddp_refuses_an_address_that_does_not_parse() {
        for text in ["10.0", "10.1", "::1", "not an address"] {
            assert!(!ddp_can_apply(&DdpDraft { host_mode: false, ip_text: text.to_string(), host_text: String::new() }));
        }
        assert!(ddp_can_apply(&DdpDraft { host_mode: false, ip_text: "10.0.0.1".to_string(), host_text: String::new() }));
        assert!(!ddp_can_apply(&DdpDraft { host_mode: false, ip_text: "10.0".to_string(), host_text: String::new() }));
        assert!(ddp_can_apply(&DdpDraft { host_mode: true, ip_text: "10.0".to_string(), host_text: "x".to_string() }));
    }

    #[test]
    fn ddp_host_mode_resolves_and_says_when_it_cannot() {
        let mut c = DdpConfig::default();
        let d = DdpDraft { host_mode: true, ip_text: String::new(), host_text: "  tree.local ".to_string() };
        ddp_apply(&mut c, &d, |host| {
            assert_eq!(host, "tree.local");
            Some(Ipv4Addr::new(192, 168, 1, 50))
        })
        .unwrap();
        assert_eq!(c.address.as_deref(), Some("192.168.1.50"));
        assert_eq!(c.host_name.as_deref(), Some("tree.local"));
        let before = c.clone();
        assert_eq!(
            ddp_apply(&mut c, &d, |_| None).unwrap_err(),
            "Could not resolve host name 'tree.local' to an IP address. Check the spelling and your network connection, or switch to IP Address mode."
        );
        assert_eq!(c, before);
    }

    #[test]
    fn open_dmx_lists_ftdi_adapters_so_a_windows_device_matches() {
        let ports = vec![
            SerialPortDetails { path: "/dev/cu.other".to_string(), vendor_id: Some(0x2341), is_usb: true, ..Default::default() },
            SerialPortDetails {
                path: "/dev/cu.usbserial-A".to_string(),
                vendor_id: Some(0x0403),
                product_id: Some(0x6001),
                product: Some("FT232R USB UART".to_string()),
                serial_number: Some("A1".to_string()),
                is_usb: true,
                ..Default::default()
            },
        ];
        let devices = open_dmx_devices_from(&ports);
        assert_eq!(devices.len(), 1);
        assert_eq!(open_dmx_device_label(&devices[0]), format!("FT232R USB UART - A1 - {}", (0x0403u32 << 16) | 0x6001));
        let saved = OpenDmxDevice {
            description: Some("FT232R USB UART".to_string()),
            serial_number: Some("A1".to_string()),
            id: devices[0].id.clone(),
            ..Default::default()
        };
        assert_eq!(open_dmx_selection(Some(&saved), &devices), Some(0));
        assert_eq!(open_dmx_selection(None, &devices), Some(0));
        assert_eq!(open_dmx_selection(Some(&saved), &[]), None);
        // No devices leaves what's saved.
        let mut data = OpenDmxConfig { device: Some(saved.clone()), ..Default::default() };
        open_dmx_apply(&mut data, &[], None);
        assert_eq!(data.device, Some(saved));
        open_dmx_apply(&mut data, &devices, Some(0));
        assert_eq!(data.device.as_ref().unwrap().name.as_deref(), Some("/dev/cu.usbserial-A"));
    }

    #[test]
    fn elexol_defaults_and_validation() {
        let d = elexol_draft(&ElexolConfig::default());
        assert_eq!((d.ip_text.as_str(), d.port_text.as_str(), d.min_intensity), ("127.0.0.1", "2424", 1));
        let mut c = ElexolConfig::default();
        assert_eq!(elexol_apply(&mut c, &ElexolDraft { ip_text: "nope".to_string(), ..d.clone() }).unwrap_err(), "Invalid IP Address.");
        for port in ["0", "65536", "abc", ""] {
            assert_eq!(
                elexol_apply(&mut c, &ElexolDraft { port_text: port.to_string(), ..d.clone() }).unwrap_err(),
                "Invalid Port."
            );
        }
        elexol_apply(&mut c, &ElexolDraft { ip_text: "10.1.1.1".to_string(), port_text: "2424".to_string(), min_intensity: 0 }).unwrap();
        assert_eq!((c.address.as_deref(), c.port, c.minimum_intensity), (Some("10.1.1.1"), 2424, 0));
        assert_eq!(elexol_draft(&c).min_intensity, 0, "a saved 0 shows as 0");
    }

    #[test]
    fn elexol_test_says_what_happened() {
        assert_eq!(elexol_test_text(Ok(())), "Connection Successful!");
        assert_eq!(elexol_test_text(Err(ElexolTestError::NoReply)), "No Reply.");
        assert_eq!(
            elexol_test_text(Err(ElexolTestError::WrongReply(vec![0x61, 0x0A]))),
            "Device replied, but with incorrect data:\n[0x61][0x0A]"
        );
        assert_eq!(elexol_test_text(Err(ElexolTestError::Socket("boom".to_string()))), "An error occured testing the device\n boom");
    }

    #[test]
    fn rds_rules_per_hardware() {
        let r = rds_field_rules(RdsHardware::Mrds192, "");
        assert_eq!((r.test_text_max, r.url_enabled, r.credentials_enabled, r.auth_enabled), (8, false, false, false));
        assert_eq!(rds_field_rules(RdsHardware::Mrds1322, "").test_text_max, 8);
        let r = rds_field_rules(RdsHardware::Http, "");
        assert_eq!((r.test_text_max, r.url_enabled, r.auth_enabled, r.serial_enabled), (128, true, true, true));
        let r = rds_field_rules(RdsHardware::Vfmt212r, "http://x/");
        assert_eq!((r.test_text_max, r.auth_forced, r.auth_enabled, r.serial_enabled), (64, true, false, false));
        assert_eq!(r.url_replacement.as_deref(), Some(RdsConfig::DEFAULT_HTTP_URL));
        let keep = "http://10.0.0.1/?action=update_rt&update_rt={text}";
        assert_eq!(rds_field_rules(RdsHardware::Vfmt212r, keep).url_replacement, None);
    }

    #[test]
    fn rds_draft_applies_and_keeps_what_it_does_not_show() {
        let saved = RdsConfig {
            hide_launched_windows: true,
            port_number: 4,
            port_name: Some("COM4".to_string()),
            ..Default::default()
        };
        let mut draft = rds_draft(&saved, &[]);
        assert_eq!(draft.url, RdsConfig::DEFAULT_HTTP_URL);
        assert_eq!(draft.port_path, None);
        rds_set_hardware(&mut draft, RdsHardware::Vfmt212r);
        assert!(draft.requires_auth);
        draft.url = "http://mine/".to_string();
        rds_set_hardware(&mut draft, RdsHardware::Mrds1322);
        assert!(draft.requires_auth, "leaving V-FMT212R keeps authentication on");
        assert_eq!(draft.url, RdsConfig::DEFAULT_HTTP_URL, "and its handler checks the URL once more as it goes off");
        let mut data = saved.clone();
        rds_apply(&mut data, &draft);
        assert_eq!(data.connection_mode, 1);
        assert!(data.hide_launched_windows);
        assert_eq!(data.port_number, 4);
        assert_eq!(data.port_name.as_deref(), Some("COM4"), "stays until a port is chosen");
        assert_eq!(data.http_username, None, "a null box that stays empty stays null");
        draft.port_path = Some("/dev/cu.usbserial-1".to_string());
        draft.password = "secret".to_string();
        rds_apply(&mut data, &draft);
        assert_eq!(data.port_name.as_deref(), Some("/dev/cu.usbserial-1"));
        assert_eq!(data.http_password.as_deref(), Some("secret"));
    }

    #[test]
    fn the_connection_mode_follows_the_mrds_radios() {
        let saved = |hardware, connection_mode| RdsConfig { hardware_id: hardware, connection_mode, ..Default::default() };
        // The designer's MRDS192 going off forces 0 for whatever else was saved.
        assert_eq!(rds_draft(&saved(RdsHardware::Http, 1), &[]).connection_mode, 0);
        assert_eq!(rds_draft(&saved(RdsHardware::Vfmt212r, 1), &[]).connection_mode, 0);
        assert_eq!(rds_draft(&saved(RdsHardware::Mrds1322, 0), &[]).connection_mode, 1);
        // A saved MRDS192 is the designer's own checked radio: no handler runs, so the saved value stays (`SetupForm.cs:39-41`).
        assert_eq!(rds_draft(&saved(RdsHardware::Mrds192, 1), &[]).connection_mode, 1);
        // HTTP keeps what the last MRDS radio set.
        let mut draft = rds_draft(&saved(RdsHardware::Http, 0), &[]);
        rds_set_hardware(&mut draft, RdsHardware::Mrds1322);
        rds_set_hardware(&mut draft, RdsHardware::Http);
        assert_eq!(draft.connection_mode, 1);
        let mut data = RdsConfig::default();
        rds_apply(&mut data, &draft);
        assert_eq!((data.hardware_id, data.connection_mode), (RdsHardware::Http, 1));
    }

    #[test]
    fn rds_status_reports_the_real_result() {
        assert_eq!(rds_send_status(&Ok(())), "Data Sent");
        assert_eq!(rds_send_status(&Err("refused".to_string())), "Data Not Sent: refused");
    }

    #[test]
    fn renard_fills_in_what_its_module_would() {
        let mut c = RenardConfig::default();
        renard_apply(&mut c, settings("/dev/cu.x"));
        assert_eq!((c.write_timeout, c.protocol_version), (500, 1));
        let mut c = RenardConfig { write_timeout: 100, protocol_version: 1, ..Default::default() };
        renard_apply(&mut c, settings("/dev/cu.x"));
        assert_eq!(c.write_timeout, 100);
    }

    #[test]
    fn dmx_pro_opens_with_its_saved_settings() {
        let c = ControllerConfig::DmxUsbPro(DmxUsbProConfig { serial: settings("COM3"), ..Default::default() });
        assert_eq!(serial_only_settings(&c).unwrap().port_name.as_deref(), Some("COM3"));
        let c = ControllerConfig::DmxUsbPro(DmxUsbProConfig::default());
        assert_eq!(serial_only_settings(&c), None);
    }
}
