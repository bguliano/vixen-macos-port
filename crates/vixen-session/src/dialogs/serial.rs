//! Serial Port Config (F7 to F25), shared by Generic Serial, Renard, DMX Pro and RDS.

use std::path::Path;
use vixen_output::serial::{self, Parity, SerialPortDetails, SerialSettings, StopBits, STANDARD_BAUD_RATES};

/// Devices that aren't light controllers, which the picker doesn't offer.
const HIDDEN_PORTS: [&str; 3] = ["/dev/cu.Bluetooth-Incoming-Port", "/dev/cu.debug-console", "/dev/cu.wlan-debug"];

/// The text upstream appends to a port another program holds.
pub const IN_USE_SUFFIX: &str = " (IN USE)";

/// One entry of the port picker.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SerialPortChoice {
    pub path: String,
    pub label: String,
    pub in_use: bool,
    /// The controller in this app that has it open.
    pub held_by: Option<String>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub serial_number: Option<String>,
    pub product: Option<String>,
    /// Bluetooth ports are never opened to test them, since that can stall or start a pairing.
    pub is_bluetooth: bool,
    pub is_usb: bool,
}

impl SerialPortChoice {
    fn update_label(&mut self) {
        let base = Path::new(&self.path).file_name().map_or(self.path.clone(), |n| n.to_string_lossy().into_owned());
        let mut label = match self.product.as_deref().filter(|p| !p.is_empty()) {
            Some(product) => format!("{product} ({base})"),
            None => base,
        };
        if self.in_use {
            label.push_str(IN_USE_SUFFIX);
        }
        self.label = label;
    }
}

/// The system's list made into picker entries: filtered, USB first, then the rest, each by name. `held` is `(path, controller
/// name)` for ports this app's own running controllers have open.
pub fn choices_from_details(details: Vec<SerialPortDetails>, held: &[(String, String)]) -> Vec<SerialPortChoice> {
    let mut choices: Vec<SerialPortChoice> = details
        .into_iter()
        .filter(|d| d.path.starts_with("/dev/cu.") && !HIDDEN_PORTS.contains(&d.path.as_str()))
        .map(|d| {
            let held_by = held.iter().find(|(path, _)| *path == d.path).map(|(_, name)| name.clone());
            let mut choice = SerialPortChoice {
                in_use: held_by.is_some(),
                held_by,
                path: d.path,
                vendor_id: d.vendor_id,
                product_id: d.product_id,
                serial_number: d.serial_number,
                product: d.product,
                is_bluetooth: d.is_bluetooth,
                is_usb: d.is_usb,
                label: String::new(),
            };
            choice.update_label();
            choice
        })
        .collect();
    choices.sort_by(|a, b| b.is_usb.cmp(&a.is_usb).then_with(|| a.path.cmp(&b.path)));
    choices
}

/// The ports on this machine. Reading the list opens nothing, so the sheet can show it at once.
pub fn serial_port_choices(held: &[(String, String)]) -> Vec<SerialPortChoice> {
    choices_from_details(serial::list_serial_port_details(), held)
}

/// F8's probe: tries to open each port to see if something else has it. Slow, and opening a port can reset an Arduino, so only
/// call this off the main thread when the dialog opens, and never in tests. A port this app holds is already marked, and a
/// Bluetooth port is left alone.
pub fn probe_serial_ports(choices: &mut [SerialPortChoice]) {
    for choice in choices.iter_mut().filter(|c| !c.is_bluetooth && !c.in_use) {
        choice.in_use = serial::port_in_use(&choice.path);
        choice.update_label();
    }
}

/// A saved `/dev/tty.X` means the callout device `/dev/cu.X`: opening a dial-in device blocks until carrier is detected.
pub fn normalize_port_path(name: &str) -> String {
    match name.strip_prefix("/dev/tty.") {
        Some(rest) => format!("/dev/cu.{rest}"),
        None => name.to_string(),
    }
}

/// Parity choices, in enum order, with whether this platform can do them (F15, F16).
pub fn parity_choices() -> Vec<(Parity, bool)> {
    vec![(Parity::None, true), (Parity::Odd, true), (Parity::Even, true), (Parity::Mark, false), (Parity::Space, false)]
}

/// Stop bit choices, without `None` (F18).
pub fn stop_bits_choices() -> Vec<(StopBits, bool)> {
    vec![(StopBits::One, true), (StopBits::Two, true), (StopBits::OnePointFive, false)]
}

/// The dialog's state. A baud rate of -1 means nothing is selected, as the combo box's getter reads then.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialConfigModel {
    /// "Current Configured Port:" (F9).
    pub current_label: String,
    /// Why the current port can't be used here, when it can't.
    pub current_note: Option<String>,
    /// "Port Details". Off when there's no port to edit and none to choose (F10).
    pub group_enabled: bool,
    pub selected_path: Option<String>,
    pub baud: i32,
    pub parity: Parity,
    pub data_bits: i32,
    pub stop_bits: StopBits,
    pub ports: Vec<SerialPortChoice>,
}

/// Why a saved port can't be used on this Mac, for the note beside it.
pub fn saved_port_note(name: &str) -> Option<String> {
    match serial::port_availability(&normalize_port_path(name)) {
        serial::PortAvailability::Available => None,
        serial::PortAvailability::WindowsName => Some(format!("{name} is a Windows port name and is not available on this Mac.")),
        serial::PortAvailability::Missing => Some(format!("{name} is not connected.")),
    }
}

/// F9 to F12. `current` is `None` when nothing is configured.
pub fn serial_config_model(current: Option<&SerialSettings>, ports: Vec<SerialPortChoice>) -> SerialConfigModel {
    let current = current.filter(|c| c.port_name.as_deref().is_some_and(|n| !n.is_empty()));
    match current {
        None => {
            let default = ports.first().map(|p| SerialSettings::dialog_default(&p.path));
            SerialConfigModel {
                current_label: "None".to_string(),
                current_note: None,
                group_enabled: default.is_some(),
                selected_path: default.as_ref().and_then(|d| d.port_name.clone()),
                baud: default.as_ref().map_or(-1, |d| d.baud_rate),
                parity: Parity::None,
                data_bits: default.as_ref().map_or(8, |d| d.data_bits),
                stop_bits: StopBits::One,
                ports,
            }
        }
        Some(settings) => {
            let name = settings.port_name.clone().unwrap_or_default();
            let wanted = normalize_port_path(&name);
            SerialConfigModel {
                current_note: saved_port_note(&name),
                current_label: name,
                group_enabled: true,
                selected_path: ports.iter().find(|p| p.path == wanted).map(|p| p.path.clone()),
                baud: if STANDARD_BAUD_RATES.contains(&settings.baud_rate) { settings.baud_rate } else { -1 },
                parity: settings.parity,
                data_bits: settings.data_bits,
                stop_bits: if settings.stop_bits == StopBits::None { StopBits::One } else { settings.stop_bits },
                ports,
            }
        }
    }
}

/// What OK does with a draft that passes (F20, F21).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidSerialConfig {
    pub settings: SerialSettings,
    /// Ask "Serial Port may be in use, do you wish to continue?" first.
    pub in_use: bool,
}

/// F20's message, with one line per problem, or the settings to use.
pub fn validate_serial_config(draft: &SerialConfigModel) -> Result<ValidSerialConfig, String> {
    let mut problems = String::new();
    let port = draft.selected_path.as_deref().filter(|p| !p.trim().is_empty());
    if port.is_none() {
        problems.push_str("* Port name has not been selected.\n");
    }
    if !STANDARD_BAUD_RATES.contains(&draft.baud) {
        problems.push_str("* Baud rate has not been selected.\n");
    }
    if !(5..=8).contains(&draft.data_bits) {
        problems.push_str("* Invalid value for data bits.\n");
    }
    if !problems.is_empty() {
        return Err(format!("The following items need to be resolved:\n\n{problems}"));
    }
    let path = port.unwrap_or_default().to_string();
    let in_use = draft.ports.iter().any(|p| p.path == path && p.in_use);
    Ok(ValidSerialConfig {
        settings: SerialSettings {
            port_name: Some(path),
            baud_rate: draft.baud,
            parity: draft.parity,
            data_bits: draft.data_bits,
            stop_bits: draft.stop_bits,
        },
        in_use,
    })
}

/// F28. A controller with no port reads "Not Set".
pub fn serial_settings_label(settings: Option<&SerialSettings>) -> String {
    match settings.filter(|s| s.port_name.as_deref().is_some_and(|n| !n.is_empty())) {
        None => "Not Set".to_string(),
        Some(s) => format!(
            "{}: {}, {}, {}, {}",
            s.port_name.as_deref().unwrap_or_default(),
            s.baud_rate,
            s.parity.name(),
            s.data_bits,
            s.stop_bits.name()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details(path: &str, usb: bool, product: Option<&str>) -> SerialPortDetails {
        SerialPortDetails {
            path: path.to_string(),
            is_usb: usb,
            product: product.map(str::to_string),
            ..Default::default()
        }
    }

    fn ports() -> Vec<SerialPortChoice> {
        choices_from_details(
            vec![
                details("/dev/cu.Bluetooth-Incoming-Port", false, None),
                details("/dev/cu.debug-console", false, None),
                details("/dev/cu.wlan-debug", false, None),
                details("/dev/tty.usbmodem1101", true, Some("Arduino Uno")),
                details("/dev/cu.zeta", false, None),
                details("/dev/cu.alpha", false, None),
                details("/dev/cu.usbserial-2", true, None),
                details("/dev/cu.usbmodem1101", true, Some("Arduino Uno")),
            ],
            &[],
        )
    }

    #[test]
    fn the_picker_hides_what_is_not_a_light_controller_and_lists_usb_first() {
        let list = ports();
        let paths: Vec<&str> = list.iter().map(|p| p.path.as_str()).collect();
        assert_eq!(paths, ["/dev/cu.usbmodem1101", "/dev/cu.usbserial-2", "/dev/cu.alpha", "/dev/cu.zeta"]);
        assert_eq!(list[0].label, "Arduino Uno (cu.usbmodem1101)");
        assert_eq!(list[1].label, "cu.usbserial-2");
        assert!(list.iter().all(|p| !p.in_use));
    }

    #[test]
    fn a_port_this_app_holds_is_marked_without_opening_it() {
        let held = vec![("/dev/cu.alpha".to_string(), "Arduino".to_string())];
        let list = choices_from_details(vec![details("/dev/cu.alpha", false, None)], &held);
        assert_eq!(list[0].label, "cu.alpha (IN USE)");
        assert_eq!(list[0].held_by.as_deref(), Some("Arduino"));
    }

    #[test]
    fn dial_in_names_mean_the_callout_device() {
        assert_eq!(normalize_port_path("/dev/tty.usbmodem1"), "/dev/cu.usbmodem1");
        assert_eq!(normalize_port_path("/dev/cu.usbmodem1"), "/dev/cu.usbmodem1");
        assert_eq!(normalize_port_path("COM9"), "COM9");
    }

    #[test]
    fn nothing_configured_defaults_to_the_first_port() {
        let m = serial_config_model(None, ports());
        assert_eq!(m.current_label, "None");
        assert!(m.group_enabled);
        assert_eq!(m.selected_path.as_deref(), Some("/dev/cu.usbmodem1101"));
        assert_eq!((m.baud, m.parity, m.data_bits, m.stop_bits), (57600, Parity::None, 8, StopBits::One));
    }

    #[test]
    fn nothing_configured_and_no_ports_turns_the_group_off() {
        let m = serial_config_model(None, Vec::new());
        assert!(!m.group_enabled);
        assert_eq!(m.selected_path, None);
    }

    #[test]
    fn a_windows_port_stays_the_current_one_and_requires_a_choice() {
        let com9 = SerialSettings {
            port_name: Some("COM9".to_string()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        };
        let m = serial_config_model(Some(&com9), ports());
        assert_eq!(m.current_label, "COM9");
        assert_eq!(m.selected_path, None);
        assert_eq!(m.baud, 9600);
        assert!(m.current_note.as_deref().unwrap().contains("Windows"));
        let error = validate_serial_config(&m).unwrap_err();
        assert_eq!(error, "The following items need to be resolved:\n\n* Port name has not been selected.\n");
    }

    #[test]
    fn a_configured_port_that_exists_is_preselected() {
        let saved = SerialSettings {
            port_name: Some("/dev/tty.usbserial-2".to_string()),
            baud_rate: 115200,
            parity: Parity::Odd,
            data_bits: 7,
            stop_bits: StopBits::Two,
        };
        let m = serial_config_model(Some(&saved), ports());
        assert_eq!(m.selected_path.as_deref(), Some("/dev/cu.usbserial-2"));
        let valid = validate_serial_config(&m).unwrap();
        assert_eq!(valid.settings.port_name.as_deref(), Some("/dev/cu.usbserial-2"));
        assert_eq!(valid.settings.baud_rate, 115200);
        assert!(!valid.in_use);
    }

    #[test]
    fn the_lists_match_upstream() {
        assert_eq!(STANDARD_BAUD_RATES, [2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600, 1048576]);
        let parity: Vec<&str> = parity_choices().iter().map(|(p, _)| p.name()).collect();
        assert_eq!(parity, ["None", "Odd", "Even", "Mark", "Space"]);
        let stops: Vec<&str> = stop_bits_choices().iter().map(|(s, _)| s.name()).collect();
        assert_eq!(stops, ["One", "Two", "OnePointFive"]);
        assert!(parity_choices().iter().filter(|(_, ok)| !ok).all(|(p, _)| matches!(p, Parity::Mark | Parity::Space)));
        assert!(stop_bits_choices().iter().filter(|(_, ok)| !ok).all(|(s, _)| *s == StopBits::OnePointFive));
    }

    #[test]
    fn validation_lists_every_problem() {
        let m = SerialConfigModel {
            current_label: "None".to_string(),
            current_note: None,
            group_enabled: true,
            selected_path: None,
            baud: -1,
            parity: Parity::None,
            data_bits: -1,
            stop_bits: StopBits::One,
            ports: Vec::new(),
        };
        assert_eq!(
            validate_serial_config(&m).unwrap_err(),
            "The following items need to be resolved:\n\n* Port name has not been selected.\n* Baud rate has not been selected.\n* Invalid value for data bits.\n"
        );
    }

    #[test]
    fn a_rate_not_in_the_list_shows_blank_and_fails() {
        let saved = SerialSettings {
            port_name: Some("/dev/cu.alpha".to_string()),
            baud_rate: 1234,
            parity: Parity::None,
            data_bits: 9,
            stop_bits: StopBits::One,
        };
        let m = serial_config_model(Some(&saved), ports());
        assert_eq!(m.baud, -1);
        let e = validate_serial_config(&m).unwrap_err();
        assert!(e.contains("Baud rate") && e.contains("data bits") && !e.contains("Port name"));
    }

    #[test]
    fn choosing_a_port_in_use_asks_first() {
        let held = vec![("/dev/cu.alpha".to_string(), "Other".to_string())];
        let ports = choices_from_details(vec![details("/dev/cu.alpha", false, None)], &held);
        let m = serial_config_model(None, ports);
        assert!(validate_serial_config(&m).unwrap().in_use);
    }

    #[test]
    fn settings_read_like_upstream() {
        let s = SerialSettings {
            port_name: Some("COM9".to_string()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        };
        assert_eq!(serial_settings_label(Some(&s)), "COM9: 9600, None, 8, One");
        assert_eq!(serial_settings_label(None), "Not Set");
        assert_eq!(serial_settings_label(Some(&SerialSettings::default())), "Not Set");
    }
}
