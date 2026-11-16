//! Port of Vixen.Modules/Controller/GenericSerial: header, one byte per output, footer.

use super::{ConfigMismatch, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::dotnet::ascii_bytes;
use crate::ids;
use crate::serial::{self, Parity, PortOpenError, RetryTimer, SerialSettings, StopBits};
use crate::xmlutil::{self, data_root, num_el, opt_text_el, text_el, ModuleIds};
use serialport::SerialPort;
use std::sync::Arc;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.Output.GenericSerial";

/// `Data`. `WriteTimeout` is commented out of the C#, so there isn't one here either.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericSerialConfig {
    pub ids: ModuleIds,
    pub serial: SerialSettings,
    pub header: Option<String>,
    pub footer: Option<String>,
}

impl Default for GenericSerialConfig {
    fn default() -> Self {
        GenericSerialConfig {
            ids: ModuleIds::new(ids::GENERIC_SERIAL, Uuid::nil()),
            serial: SerialSettings::default(),
            header: None,
            footer: None,
        }
    }
}

impl GenericSerialConfig {
    pub fn from_xml(e: &XElement) -> GenericSerialConfig {
        GenericSerialConfig {
            ids: ModuleIds::read(e),
            serial: SerialSettings {
                port_name: xmlutil::get_opt_string(e, "PortName"),
                baud_rate: xmlutil::get_i32(e, "BaudRate"),
                parity: Parity::from_name(&e.child_text("Parity").unwrap_or_default()),
                data_bits: xmlutil::get_i32(e, "DataBits"),
                stop_bits: StopBits::from_name(&e.child_text("StopBits").unwrap_or_default()),
            },
            header: xmlutil::get_opt_string(e, "Header"),
            footer: xmlutil::get_opt_string(e, "Footer"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "Data", &self.ids);
        e.push(num_el(&ns, "BaudRate", self.serial.baud_rate));
        e.push(num_el(&ns, "DataBits", self.serial.data_bits));
        e.push(opt_text_el(&ns, "Footer", self.footer.as_deref()));
        e.push(opt_text_el(&ns, "Header", self.header.as_deref()));
        e.push(text_el(&ns, "Parity", self.serial.parity.name()));
        e.push(opt_text_el(&ns, "PortName", self.serial.port_name.as_deref()));
        e.push(text_el(&ns, "StopBits", self.serial.stop_bits.name()));
        e
    }
}

/// Fills `packet` with one frame: the header, the output values, then the footer.
pub fn build_packet(header: &[u8], footer: &[u8], output_count: usize, commands: &[OutputCommand], packet: &mut Vec<u8>) {
    packet.clear();
    packet.extend_from_slice(header);
    packet.extend((0..output_count).map(|i| commands.get(i).map_or(0, OutputCommand::byte)));
    packet.extend_from_slice(footer);
}

/// `Module`.
pub struct GenericSerialModule {
    config: GenericSerialConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    port: Option<Box<dyn SerialPort>>,
    running: bool,
    header: Vec<u8>,
    footer: Vec<u8>,
    packet: Vec<u8>,
    retry: RetryTimer,
}

impl GenericSerialModule {
    pub fn new(config: GenericSerialConfig) -> GenericSerialModule {
        let mut m = GenericSerialModule {
            config,
            shared: ModuleShared::new(),
            output_count: 0,
            port: None,
            running: false,
            header: Vec::new(),
            footer: Vec::new(),
            packet: Vec::new(),
            retry: RetryTimer::default(),
        };
        m.init_module();
        m
    }

    /// `InitModule`.
    fn init_module(&mut self) {
        self.port = None;
        self.header = ascii_bytes(self.config.header.as_deref().unwrap_or(""));
        self.footer = ascii_bytes(self.config.footer.as_deref().unwrap_or(""));
        if self.running && self.config.serial.is_valid() {
            self.open_com_port();
        }
    }

    /// `OpenComPort`. A port that won't open starts the two-minute retry timer, and a saved Windows name reports as
    /// unavailable the same way, without ever trying.
    fn open_com_port(&mut self) {
        match serial::open_port(&self.config.serial, false, false) {
            Ok(port) => {
                self.port = Some(port);
                self.retry.on_open_success();
                self.shared.set_status(ModuleStatus::Running);
            }
            Err(e) => {
                let is_unavailable = e.is_unavailable();
                let message = e.to_string();
                self.shared.record_error(message.clone());
                self.shared.set_status(if is_unavailable {
                    ModuleStatus::Unavailable(message)
                } else {
                    ModuleStatus::Error(message)
                });
                if matches!(e, PortOpenError::UnsupportedSetting(_)) {
                    self.retry.stop();
                } else {
                    self.retry.on_open_failure();
                }
            }
        }
    }
}

impl OutputModule for GenericSerialModule {
    fn type_id(&self) -> Uuid {
        ids::GENERIC_SERIAL
    }

    fn start(&mut self) {
        self.running = true;
        if !self.config.serial.is_valid() {
            self.shared.set_status(ModuleStatus::Unavailable("No serial port is configured".to_string()));
        } else if self.port.is_none() {
            self.open_com_port();
        }
    }

    fn stop(&mut self) {
        self.port = None;
        self.running = false;
        self.retry.stop();
        self.shared.set_status(ModuleStatus::Stopped);
    }

    fn update(&mut self, commands: &[OutputCommand]) {
        if self.running && self.port.is_none() && self.retry.due() {
            self.open_com_port();
        }
        let Some(port) = self.port.as_mut() else {
            return;
        };
        build_packet(&self.header, &self.footer, self.output_count, commands, &mut self.packet);
        if !self.packet.is_empty() {
            if serial::frame_would_lag(port.bytes_to_write().unwrap_or(0), self.packet.len()) {
                return;
            }
            // No write timeout: .NET's default is to wait as long as the port needs.
            match serial::write_all(port.as_mut(), &self.packet, None, self.shared.stop_flag()) {
                Ok(()) => self.shared.count_packet(self.packet.len()),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => self.shared.record_error(format!("An error occurred writing to the serial port: {e}")),
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
        ControllerConfig::GenericSerial(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::GenericSerial(config) = config else {
            return Err(ConfigMismatch);
        };
        self.config = config;
        self.init_module();
        Ok(())
    }

    fn shared(&self) -> &Arc<ModuleShared> {
        &self.shared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_is_header_values_footer() {
        let mut p = Vec::new();
        build_packet(b"AB", b"Z", 3, &[OutputCommand::U8(1), OutputCommand::None, OutputCommand::U16(999)], &mut p);
        assert_eq!(p, vec![b'A', b'B', 1, 0, 0, b'Z']);
        // Short of commands, the rest stays 0, as in the freshly allocated C# packet.
        build_packet(b"", b"", 2, &[OutputCommand::U8(7)], &mut p);
        assert_eq!(p, vec![7, 0]);
    }

    #[test]
    fn the_owners_example_packet_and_a_character_ascii_cannot_hold() {
        let mut p = Vec::new();
        build_packet(&ascii_bytes("AB"), &ascii_bytes("Z"), 2, &[OutputCommand::U8(1), OutputCommand::U8(2)], &mut p);
        assert_eq!(p, [0x41, 0x42, 0x01, 0x02, 0x5A]);
        assert_eq!(ascii_bytes("é"), [0x3F]);
    }

    #[test]
    fn a_saved_windows_port_reports_unavailable_and_stays_in_the_data() {
        let xml = r#"<Data xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Output.GenericSerial" xmlns:i="http://www.w3.org/2001/XMLSchema-instance">
            <ModuleInstanceId xmlns="">ee79fee2-2d7d-4984-a178-288cee39a361</ModuleInstanceId>
            <ModuleTypeId xmlns="">d22c9233-16e9-4008-a215-e577f78a7a2e</ModuleTypeId>
            <BaudRate>9600</BaudRate><DataBits>8</DataBits><Footer /><Header /><Parity>None</Parity><PortName>COM9</PortName><StopBits>One</StopBits></Data>"#;
        let e = XElement::parse(xml).unwrap();
        let mut m = GenericSerialModule::new(GenericSerialConfig::from_xml(&e));
        m.set_output_count(16);
        m.start();
        assert!(matches!(m.status(), ModuleStatus::Unavailable(s) if s.contains("COM9")));
        m.update(&vec![OutputCommand::U8(255); 16]);
        assert_eq!(m.shared().packets_sent(), 0);
        let ControllerConfig::GenericSerial(saved) = m.config() else { panic!() };
        assert_eq!(saved.serial.port_name.as_deref(), Some("COM9"));
        assert!(saved.to_xml().semantic_eq(&e));
        m.stop();
        assert_eq!(m.status(), ModuleStatus::Stopped);
    }

    #[test]
    fn an_unconfigured_module_is_quiet() {
        let mut m = GenericSerialModule::new(GenericSerialConfig::default());
        m.set_output_count(4);
        m.start();
        assert!(matches!(m.status(), ModuleStatus::Unavailable(_)));
        m.update(&[OutputCommand::U8(1)]);
        m.stop();
    }
}
