//! Controller setup, other than E1.31.

use super::{no_such, parse_id, DialogContext};
use crate::core::FfiError;
use uuid::Uuid;
use vixen_output::ids;
use vixen_output::modules::{ControllerConfig, OpenDmxDevice, RdsHardware, RenderStyle};
use vixen_output::serial::{Parity, SerialSettings, StopBits, STANDARD_BAUD_RATES};
use vixen_session::dialogs::controllers as logic;
use vixen_session::dialogs::serial as serial_logic;
use vixen_session::dialogs::world::DialogWorld;

/// What a Test or Send button says in a muted app, such as under the test harness.
const MUTED: &str = "The app is muted, so nothing was sent.";

/// Which sheet a controller's Configure opens.
#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogControllerKind {
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

impl From<logic::ControllerSetupKind> for DialogControllerKind {
    fn from(k: logic::ControllerSetupKind) -> Self {
        use logic::ControllerSetupKind as K;
        match k {
            K::GenericSerial => DialogControllerKind::GenericSerial,
            K::DummyLighting => DialogControllerKind::DummyLighting,
            K::E131 => DialogControllerKind::E131,
            K::Ddp => DialogControllerKind::Ddp,
            K::SerialPortOnly => DialogControllerKind::SerialPortOnly,
            K::OpenDmx => DialogControllerKind::OpenDmx,
            K::Elexol => DialogControllerKind::Elexol,
            K::Rds => DialogControllerKind::Rds,
            K::Launcher => DialogControllerKind::Launcher,
            K::None => DialogControllerKind::None,
        }
    }
}

/// A controller as the dialogs see it.
#[derive(uniffi::Record, Clone)]
pub struct DialogController {
    pub id: String,
    pub name: String,
    pub type_name: String,
    pub kind: DialogControllerKind,
    pub output_count: u32,
    pub running: bool,
    /// Why it can't reach its hardware, for the status on its row (F36).
    pub status: Option<String>,
}

/// `SerialPort`'s settings. `port_name` stays exactly as saved, so a `COM9` from Windows survives until the user picks a port.
#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogSerialSettings {
    pub port_name: Option<String>,
    pub baud_rate: i32,
    pub parity: String,
    pub data_bits: i32,
    pub stop_bits: String,
}

impl From<&SerialSettings> for DialogSerialSettings {
    fn from(s: &SerialSettings) -> Self {
        DialogSerialSettings {
            port_name: s.port_name.clone(),
            baud_rate: s.baud_rate,
            parity: s.parity.name().to_string(),
            data_bits: s.data_bits,
            stop_bits: s.stop_bits.name().to_string(),
        }
    }
}

impl From<&DialogSerialSettings> for SerialSettings {
    fn from(s: &DialogSerialSettings) -> Self {
        SerialSettings {
            port_name: s.port_name.clone(),
            baud_rate: s.baud_rate,
            parity: Parity::from_name(&s.parity),
            data_bits: s.data_bits,
            stop_bits: StopBits::from_name(&s.stop_bits),
        }
    }
}

/// One entry of the port picker.
#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogSerialPort {
    pub path: String,
    pub label: String,
    pub in_use: bool,
    pub held_by: Option<String>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub serial_number: Option<String>,
    pub product: Option<String>,
    pub is_bluetooth: bool,
    pub is_usb: bool,
}

impl From<&serial_logic::SerialPortChoice> for DialogSerialPort {
    fn from(c: &serial_logic::SerialPortChoice) -> Self {
        DialogSerialPort {
            path: c.path.clone(),
            label: c.label.clone(),
            in_use: c.in_use,
            held_by: c.held_by.clone(),
            vendor_id: c.vendor_id,
            product_id: c.product_id,
            serial_number: c.serial_number.clone(),
            product: c.product.clone(),
            is_bluetooth: c.is_bluetooth,
            is_usb: c.is_usb,
        }
    }
}

impl From<&DialogSerialPort> for serial_logic::SerialPortChoice {
    fn from(c: &DialogSerialPort) -> Self {
        serial_logic::SerialPortChoice {
            path: c.path.clone(),
            label: c.label.clone(),
            in_use: c.in_use,
            held_by: c.held_by.clone(),
            vendor_id: c.vendor_id,
            product_id: c.product_id,
            serial_number: c.serial_number.clone(),
            product: c.product.clone(),
            is_bluetooth: c.is_bluetooth,
            is_usb: c.is_usb,
        }
    }
}

/// A choice in a list, and whether this platform can do it (F16, F18).
#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogSerialOption {
    pub name: String,
    pub supported: bool,
}

/// Serial Port Config's state. A baud rate of -1 means nothing is selected.
#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogSerialConfig {
    pub current_label: String,
    pub current_note: Option<String>,
    pub group_enabled: bool,
    pub selected_path: Option<String>,
    pub baud: i32,
    pub parity: String,
    pub data_bits: i32,
    pub stop_bits: String,
    pub ports: Vec<DialogSerialPort>,
}

impl From<serial_logic::SerialConfigModel> for DialogSerialConfig {
    fn from(m: serial_logic::SerialConfigModel) -> Self {
        DialogSerialConfig {
            current_label: m.current_label,
            current_note: m.current_note,
            group_enabled: m.group_enabled,
            selected_path: m.selected_path,
            baud: m.baud,
            parity: m.parity.name().to_string(),
            data_bits: m.data_bits,
            stop_bits: m.stop_bits.name().to_string(),
            ports: m.ports.iter().map(DialogSerialPort::from).collect(),
        }
    }
}

impl From<&DialogSerialConfig> for serial_logic::SerialConfigModel {
    fn from(m: &DialogSerialConfig) -> Self {
        serial_logic::SerialConfigModel {
            current_label: m.current_label.clone(),
            current_note: m.current_note.clone(),
            group_enabled: m.group_enabled,
            selected_path: m.selected_path.clone(),
            baud: m.baud,
            parity: Parity::from_name(&m.parity),
            data_bits: m.data_bits,
            stop_bits: StopBits::from_name(&m.stop_bits),
            ports: m.ports.iter().map(Into::into).collect(),
        }
    }
}

/// What Serial Port Config's OK found. `error` is F20's text, and `in_use` asks F21's question first.
#[derive(uniffi::Record, Clone)]
pub struct DialogSerialValidation {
    pub error: Option<String>,
    pub settings: Option<DialogSerialSettings>,
    pub in_use: bool,
}

/// What the Generic Serial sheet edits.
#[derive(uniffi::Record, Clone)]
pub struct DialogGenericSerial {
    pub settings: Option<DialogSerialSettings>,
    pub header_on: bool,
    pub header: String,
    pub footer_on: bool,
    pub footer: String,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogRenderStyle {
    Monochrome,
    RgbMultiChannel,
    RgbSingleChannel,
}

impl From<RenderStyle> for DialogRenderStyle {
    fn from(s: RenderStyle) -> Self {
        match s {
            RenderStyle::Monochrome => DialogRenderStyle::Monochrome,
            RenderStyle::RGBMultiChannel => DialogRenderStyle::RgbMultiChannel,
            RenderStyle::RGBSingleChannel => DialogRenderStyle::RgbSingleChannel,
        }
    }
}

impl From<DialogRenderStyle> for RenderStyle {
    fn from(s: DialogRenderStyle) -> Self {
        match s {
            DialogRenderStyle::Monochrome => RenderStyle::Monochrome,
            DialogRenderStyle::RgbMultiChannel => RenderStyle::RGBMultiChannel,
            DialogRenderStyle::RgbSingleChannel => RenderStyle::RGBSingleChannel,
        }
    }
}

#[derive(uniffi::Record, Clone)]
pub struct DialogDummyLighting {
    pub title: String,
    pub style: DialogRenderStyle,
}

#[derive(uniffi::Record, Clone)]
pub struct DialogDdp {
    pub host_mode: bool,
    pub ip_text: String,
    pub host_text: String,
}

impl From<&DialogDdp> for logic::DdpDraft {
    fn from(d: &DialogDdp) -> Self {
        logic::DdpDraft { host_mode: d.host_mode, ip_text: d.ip_text.clone(), host_text: d.host_text.clone() }
    }
}

#[derive(uniffi::Record, Clone)]
pub struct DialogElexol {
    pub ip_text: String,
    pub port_text: String,
    pub min_intensity: i32,
}

impl From<&DialogElexol> for logic::ElexolDraft {
    fn from(d: &DialogElexol) -> Self {
        logic::ElexolDraft { ip_text: d.ip_text.clone(), port_text: d.port_text.clone(), min_intensity: d.min_intensity }
    }
}

/// `Device`, in the order it's stored.
#[derive(uniffi::Record, Clone)]
pub struct DialogOpenDmxDevice {
    pub description: Option<String>,
    pub id: Option<String>,
    pub index: i32,
    pub name: Option<String>,
    pub serial_number: Option<String>,
    pub device_type: Option<String>,
    pub label: String,
}

impl From<&OpenDmxDevice> for DialogOpenDmxDevice {
    fn from(d: &OpenDmxDevice) -> Self {
        DialogOpenDmxDevice {
            description: d.description.clone(),
            id: d.id.clone(),
            index: d.index,
            name: d.name.clone(),
            serial_number: d.serial_number.clone(),
            device_type: d.device_type.clone(),
            label: logic::open_dmx_device_label(d),
        }
    }
}

impl From<&DialogOpenDmxDevice> for OpenDmxDevice {
    fn from(d: &DialogOpenDmxDevice) -> Self {
        OpenDmxDevice {
            description: d.description.clone(),
            id: d.id.clone(),
            index: d.index,
            name: d.name.clone(),
            serial_number: d.serial_number.clone(),
            device_type: d.device_type.clone(),
        }
    }
}

#[derive(uniffi::Record, Clone)]
pub struct DialogOpenDmx {
    pub devices: Vec<DialogOpenDmxDevice>,
    pub selected: Option<u32>,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogRdsHardware {
    Mrds192,
    Mrds1322,
    Http,
    Vfmt212r,
}

impl From<RdsHardware> for DialogRdsHardware {
    fn from(h: RdsHardware) -> Self {
        match h {
            RdsHardware::Mrds192 => DialogRdsHardware::Mrds192,
            RdsHardware::Mrds1322 => DialogRdsHardware::Mrds1322,
            RdsHardware::Http => DialogRdsHardware::Http,
            RdsHardware::Vfmt212r => DialogRdsHardware::Vfmt212r,
        }
    }
}

impl From<DialogRdsHardware> for RdsHardware {
    fn from(h: DialogRdsHardware) -> Self {
        match h {
            DialogRdsHardware::Mrds192 => RdsHardware::Mrds192,
            DialogRdsHardware::Mrds1322 => RdsHardware::Mrds1322,
            DialogRdsHardware::Http => RdsHardware::Http,
            DialogRdsHardware::Vfmt212r => RdsHardware::Vfmt212r,
        }
    }
}

#[derive(uniffi::Record, Clone)]
pub struct DialogRds {
    pub hardware: DialogRdsHardware,
    pub connection_mode: i32,
    pub port_path: Option<String>,
    /// A saved port name that isn't on this Mac, shown as unavailable. It stays in the file until a port is chosen.
    pub unavailable_port: Option<String>,
    pub url: String,
    pub requires_auth: bool,
    pub username: String,
    pub password: String,
    pub bi_directional: bool,
    pub slow: bool,
}

impl From<&DialogRds> for logic::RdsDraft {
    fn from(d: &DialogRds) -> Self {
        logic::RdsDraft {
            hardware: d.hardware.into(),
            connection_mode: d.connection_mode,
            port_path: d.port_path.clone(),
            url: d.url.clone(),
            requires_auth: d.requires_auth,
            username: d.username.clone(),
            password: d.password.clone(),
            bi_directional: d.bi_directional,
            slow: d.slow,
        }
    }
}

#[derive(uniffi::Record, Clone)]
pub struct DialogRdsRules {
    pub test_text_max: u32,
    pub url_enabled: bool,
    pub credentials_enabled: bool,
    pub auth_enabled: bool,
    pub serial_enabled: bool,
}

fn config_of(world: &dyn DialogWorld, id: &Uuid) -> Result<ControllerConfig, FfiError> {
    world.controller_config(id).ok_or_else(|| no_such("controller"))
}

fn held_ports(world: &dyn DialogWorld) -> Vec<(String, String)> {
    world
        .controllers()
        .into_iter()
        .filter(|c| c.running)
        .filter_map(|c| {
            let port = match world.controller_config(&c.id)? {
                ControllerConfig::GenericSerial(x) => x.serial.port_name,
                ControllerConfig::Renard(x) => x.serial.port_name,
                ControllerConfig::DmxUsbPro(x) => x.serial.port_name,
                ControllerConfig::Rds(x) => x.port_name,
                _ => None,
            }?;
            Some((port, c.name))
        })
        .collect()
}

#[uniffi::export]
impl DialogContext {
    /// Every controller, for the test window and for row statuses.
    pub fn controllers(&self) -> Vec<DialogController> {
        let running_status = |id: &Uuid| -> Option<String> {
            self.core.playback.with_output(|_, runner| {
                let i = runner.controller_index(id)?;
                match runner.status(i) {
                    vixen_output::ModuleStatus::Unavailable(m)
                    | vixen_output::ModuleStatus::UnsupportedOnMacOs(m)
                    | vixen_output::ModuleStatus::Error(m) => Some(m),
                    _ => None,
                }
            })
        };
        self.world(|world| {
            world
                .controllers()
                .into_iter()
                .map(|c| DialogController {
                    id: c.id.to_string(),
                    type_name: ids::controller_descriptor(&c.type_id).map_or("Unknown Controller", |d| d.type_name).to_string(),
                    kind: logic::controller_setup_kind(&c.type_id).into(),
                    output_count: c.output_count,
                    running: c.running,
                    status: running_status(&c.id)
                        .or_else(|| world.controller_config(&c.id).and_then(|config| logic::controller_configuration_problem(&config))),
                    name: c.name,
                })
                .collect()
        })
    }

    /// F1: which sheet Configure opens.
    pub fn controller_kind(&self, controller: String) -> Result<DialogControllerKind, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| {
            world
                .controllers()
                .iter()
                .find(|c| c.id == id)
                .map(|c| logic::controller_setup_kind(&c.type_id).into())
                .ok_or_else(|| no_such("controller"))
        })
    }

    // ---- Serial Port Config ------------------------------------------------------------------------------------------------

    /// The ports on this machine, marking those this app holds. Opens nothing, so the sheet can show them at once.
    pub fn serial_ports(&self) -> Vec<DialogSerialPort> {
        let held = self.world(|world| held_ports(world));
        serial_logic::serial_port_choices(&held).iter().map(DialogSerialPort::from).collect()
    }

    /// F8's probe, which opens each port to see if it's free. Run it off the main thread, only as the dialog opens, and never in tests:
    /// opening a port can reset an Arduino.
    pub fn probe_serial_ports(&self, ports: Vec<DialogSerialPort>) -> Vec<DialogSerialPort> {
        let mut choices: Vec<serial_logic::SerialPortChoice> = ports.iter().map(Into::into).collect();
        serial_logic::probe_serial_ports(&mut choices);
        choices.iter().map(DialogSerialPort::from).collect()
    }

    /// F9 to F12.
    pub fn serial_config(&self, current: Option<DialogSerialSettings>, ports: Vec<DialogSerialPort>) -> DialogSerialConfig {
        let current: Option<SerialSettings> = current.as_ref().map(Into::into);
        let ports = ports.iter().map(Into::into).collect();
        serial_logic::serial_config_model(current.as_ref(), ports).into()
    }

    /// F20, F21.
    pub fn validate_serial_config(&self, config: DialogSerialConfig) -> DialogSerialValidation {
        match serial_logic::validate_serial_config(&(&config).into()) {
            Ok(valid) => DialogSerialValidation { error: None, settings: Some((&valid.settings).into()), in_use: valid.in_use },
            Err(error) => DialogSerialValidation { error: Some(error), settings: None, in_use: false },
        }
    }

    /// F14.
    pub fn baud_rates(&self) -> Vec<i32> {
        STANDARD_BAUD_RATES.to_vec()
    }

    /// F15, F16.
    pub fn parity_options(&self) -> Vec<DialogSerialOption> {
        serial_logic::parity_choices()
            .into_iter()
            .map(|(p, supported)| DialogSerialOption { name: p.name().to_string(), supported })
            .collect()
    }

    /// F18.
    pub fn stop_bit_options(&self) -> Vec<DialogSerialOption> {
        serial_logic::stop_bits_choices()
            .into_iter()
            .map(|(s, supported)| DialogSerialOption { name: s.name().to_string(), supported })
            .collect()
    }

    /// F28.
    pub fn serial_settings_label(&self, settings: Option<DialogSerialSettings>) -> String {
        let settings: Option<SerialSettings> = settings.as_ref().map(Into::into);
        serial_logic::serial_settings_label(settings.as_ref())
    }

    /// Why a saved port can't be used here, for the note beside it.
    pub fn saved_port_note(&self, name: String) -> Option<String> {
        serial_logic::saved_port_note(&name)
    }

    // ---- Generic Serial ------------------------------------------------------------------------------------------------------

    pub fn generic_serial(&self, controller: String) -> Result<DialogGenericSerial, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::GenericSerial(c) => {
                let header = c.header.unwrap_or_default();
                let footer = c.footer.unwrap_or_default();
                let settings = c.serial.port_name.as_deref().is_some_and(|n| !n.is_empty()).then(|| (&c.serial).into());
                Ok(DialogGenericSerial { settings, header_on: !header.is_empty(), header, footer_on: !footer.is_empty(), footer })
            }
            _ => Err(no_such("Generic Serial controller")),
        })
    }

    /// F34. Returns `Data.IsValid`, which is what `Setup` returns.
    pub fn apply_generic_serial(&self, controller: String, draft: DialogGenericSerial) -> Result<bool, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::GenericSerial(mut c) => {
                let settings = draft.settings.as_ref().map(Into::into).unwrap_or_default();
                let valid = logic::generic_serial_apply(&mut c, settings, draft.header_on, &draft.header, draft.footer_on, &draft.footer);
                world.set_controller_config(&id, ControllerConfig::GenericSerial(c));
                Ok(valid)
            }
            _ => Err(no_such("Generic Serial controller")),
        })
    }

    // ---- Renard, DMX Pro -----------------------------------------------------------------------------------------------------

    /// F100, F102: the settings Serial Port Config opens with.
    pub fn serial_only_settings(&self, controller: String) -> Result<Option<DialogSerialSettings>, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| {
            let config = config_of(world, &id)?;
            Ok(logic::serial_only_settings(&config).as_ref().map(Into::into))
        })
    }

    /// F100, F102.
    pub fn apply_serial_only(&self, controller: String, settings: DialogSerialSettings) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        let settings: SerialSettings = (&settings).into();
        self.world(|world| {
            let config = match config_of(world, &id)? {
                ControllerConfig::Renard(mut c) => {
                    logic::renard_apply(&mut c, settings);
                    ControllerConfig::Renard(c)
                }
                ControllerConfig::DmxUsbPro(mut c) => {
                    logic::dmx_pro_apply(&mut c, settings);
                    ControllerConfig::DmxUsbPro(c)
                }
                _ => return Err(no_such("serial controller")),
            };
            world.set_controller_config(&id, config);
            Ok(())
        })
    }

    // ---- Dummy Lighting ------------------------------------------------------------------------------------------------------

    pub fn dummy_lighting(&self, controller: String) -> Result<DialogDummyLighting, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::DummyLighting(c) => {
                Ok(DialogDummyLighting { title: c.form_title.unwrap_or_default(), style: c.render_style.into() })
            }
            _ => Err(no_such("Dummy Lighting controller")),
        })
    }

    /// F44.
    pub fn apply_dummy_lighting(&self, controller: String, draft: DialogDummyLighting) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::DummyLighting(mut c) => {
                logic::dummy_apply(&mut c, &draft.title, draft.style.into());
                world.set_controller_config(&id, ControllerConfig::DummyLighting(c));
                Ok(())
            }
            _ => Err(no_such("Dummy Lighting controller")),
        })
    }

    // ---- DDP -----------------------------------------------------------------------------------------------------------------

    pub fn ddp(&self, controller: String) -> Result<DialogDdp, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Ddp(c) => {
                let d = logic::ddp_draft(&c);
                Ok(DialogDdp { host_mode: d.host_mode, ip_text: d.ip_text, host_text: d.host_text })
            }
            _ => Err(no_such("DDP controller")),
        })
    }

    /// F97: whether OK is available.
    pub fn ddp_can_apply(&self, draft: DialogDdp) -> bool {
        logic::ddp_can_apply(&(&draft).into())
    }

    /// F97. A host name resolves here, which can take seconds, so call this off the main thread. The error is the alert's text.
    pub fn apply_ddp(&self, controller: String, draft: DialogDdp) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        let logic_draft: logic::DdpDraft = (&draft).into();
        let mut config = self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Ddp(c) => Ok(c),
            _ => Err(no_such("DDP controller")),
        })?;
        // No lock is held while this waits for a name server. A muted app stays off the network, so to it every name is unknown.
        let online = self.core.playback.hardware_allowed();
        logic::ddp_apply(&mut config, &logic_draft, |host| if online { vixen_output::modules::ddp::resolve_to_ipv4(host) } else { None })
            .map_err(FfiError::new)?;
        self.world(|world| world.set_controller_config(&id, ControllerConfig::Ddp(config)));
        Ok(())
    }

    // ---- DMX Open ------------------------------------------------------------------------------------------------------------

    /// F105.
    pub fn open_dmx(&self, controller: String) -> Result<DialogOpenDmx, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::OpenDmx(c) => {
                let devices = logic::open_dmx_devices();
                let selected = logic::open_dmx_selection(c.device.as_ref(), &devices).map(|i| i as u32);
                Ok(DialogOpenDmx { devices: devices.iter().map(Into::into).collect(), selected })
            }
            _ => Err(no_such("DMX Open controller")),
        })
    }

    /// F106.
    pub fn apply_open_dmx(&self, controller: String, draft: DialogOpenDmx) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::OpenDmx(mut c) => {
                let devices: Vec<OpenDmxDevice> = draft.devices.iter().map(Into::into).collect();
                logic::open_dmx_apply(&mut c, &devices, draft.selected.map(|i| i as usize));
                world.set_controller_config(&id, ControllerConfig::OpenDmx(c));
                Ok(())
            }
            _ => Err(no_such("DMX Open controller")),
        })
    }

    // ---- Elexol --------------------------------------------------------------------------------------------------------------

    pub fn elexol(&self, controller: String) -> Result<DialogElexol, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Elexol(c) => {
                let d = logic::elexol_draft(&c);
                Ok(DialogElexol { ip_text: d.ip_text, port_text: d.port_text, min_intensity: d.min_intensity })
            }
            _ => Err(no_such("Elexol controller")),
        })
    }

    /// F114. The error is the alert's text.
    pub fn apply_elexol(&self, controller: String, draft: DialogElexol) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Elexol(mut c) => {
                logic::elexol_apply(&mut c, &(&draft).into()).map_err(FfiError::new)?;
                world.set_controller_config(&id, ControllerConfig::Elexol(c));
                Ok(())
            }
            _ => Err(no_such("Elexol controller")),
        })
    }

    /// F112's Test button. Sends to the device and waits up to a second, so call it off the main thread, and only when clicked.
    pub fn test_elexol(&self, ip: String, port: String) -> String {
        if !self.core.playback.hardware_allowed() {
            return logic::elexol_test_text(Err(vixen_output::modules::elexol::ElexolTestError::Socket(MUTED.to_string())));
        }
        logic::elexol_test(&ip, &port)
    }

    // ---- RDS -----------------------------------------------------------------------------------------------------------------

    pub fn rds(&self, controller: String) -> Result<DialogRds, FfiError> {
        let id = parse_id(&controller)?;
        let ports: Vec<String> = self.serial_ports().into_iter().map(|p| p.path).collect();
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Rds(c) => {
                let d = logic::rds_draft(&c, &ports);
                let unavailable_port = c.port_name.clone().filter(|p| !p.is_empty() && !ports.contains(p)).or_else(|| {
                    (c.port_number != 0 && c.port_name.is_none()).then(|| format!("COM{}", c.port_number))
                });
                Ok(rds_record(d, unavailable_port))
            }
            _ => Err(no_such("RDS controller")),
        })
    }

    /// F118.
    pub fn rds_rules(&self, draft: DialogRds) -> DialogRdsRules {
        let r = logic::rds_field_rules(draft.hardware.into(), &draft.url);
        DialogRdsRules {
            test_text_max: r.test_text_max,
            url_enabled: r.url_enabled,
            credentials_enabled: r.credentials_enabled,
            auth_enabled: r.auth_enabled,
            serial_enabled: r.serial_enabled,
        }
    }

    /// F117, F118: what choosing a hardware radio does to the rest of the form.
    pub fn rds_with_hardware(&self, draft: DialogRds, hardware: DialogRdsHardware) -> DialogRds {
        let unavailable_port = draft.unavailable_port.clone();
        let mut d: logic::RdsDraft = (&draft).into();
        logic::rds_set_hardware(&mut d, hardware.into());
        rds_record(d, unavailable_port)
    }

    /// F129.
    pub fn apply_rds(&self, controller: String, draft: DialogRds) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Rds(mut c) => {
                logic::rds_apply(&mut c, &(&draft).into());
                world.set_controller_config(&id, ControllerConfig::Rds(c));
                Ok(())
            }
            _ => Err(no_such("RDS controller")),
        })
    }

    /// F124's Send, using the form as it stands. Reaches hardware or the network, so call it off the main thread, and only when clicked.
    /// Returns the status strip's text.
    pub fn send_rds_test(&self, controller: String, draft: DialogRds, text: String) -> Result<String, FfiError> {
        let id = parse_id(&controller)?;
        let mut config = self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Rds(c) => Ok(c),
            _ => Err(no_such("RDS controller")),
        })?;
        logic::rds_apply(&mut config, &(&draft).into());
        if !self.core.playback.hardware_allowed() {
            return Ok(logic::rds_send_status(&Err(MUTED.to_string())));
        }
        Ok(logic::rds_send_status(&vixen_output::modules::rds::send_test_text(&config, &text)))
    }

    // ---- Launcher ------------------------------------------------------------------------------------------------------------

    pub fn launcher_hides_windows(&self, controller: String) -> Result<bool, FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Launcher(c) => Ok(c.hide_launched_windows),
            _ => Err(no_such("Launcher controller")),
        })
    }

    /// F132.
    pub fn apply_launcher(&self, controller: String, hide_launched_windows: bool) -> Result<(), FfiError> {
        let id = parse_id(&controller)?;
        self.world(|world| match config_of(world, &id)? {
            ControllerConfig::Launcher(mut c) => {
                logic::launcher_apply(&mut c, hide_launched_windows);
                world.set_controller_config(&id, ControllerConfig::Launcher(c));
                Ok(())
            }
            _ => Err(no_such("Launcher controller")),
        })
    }
}

fn rds_record(d: logic::RdsDraft, unavailable_port: Option<String>) -> DialogRds {
    DialogRds {
        hardware: d.hardware.into(),
        connection_mode: d.connection_mode,
        port_path: d.port_path,
        unavailable_port,
        url: d.url,
        requires_auth: d.requires_auth,
        username: d.username,
        password: d.password,
        bi_directional: d.bi_directional,
        slow: d.slow,
    }
}
