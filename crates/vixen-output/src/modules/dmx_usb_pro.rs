//! Port of Vixen.Modules/Controller/DMXUsbPro: Enttec's framing around a 513-byte DMX universe.

use super::{ConfigMismatch, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::ids;
use crate::serial::{self, Parity, SerialSettings, StopBits};
use crate::xmlutil::{self, data_root, num_el, opt_text_el, text_el, ModuleIds};
use serialport::SerialPort;
use std::sync::Arc;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.Output.DmxUsbPro";

/// `MessageType.OutputOnlySendDMXPacketRequest`.
pub const SEND_DMX_PACKET_REQUEST: u8 = 6;

const START_OF_MESSAGE: u8 = 0x7E;
const END_OF_MESSAGE: u8 = 0xE7;

/// `Data`. Its parity member really is spelled `Partity` in the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DmxUsbProConfig {
    pub ids: ModuleIds,
    pub serial: SerialSettings,
}

impl Default for DmxUsbProConfig {
    fn default() -> Self {
        DmxUsbProConfig { ids: ModuleIds::new(ids::DMX_USB_PRO, Uuid::nil()), serial: SerialSettings::default() }
    }
}

impl DmxUsbProConfig {
    pub fn from_xml(e: &XElement) -> DmxUsbProConfig {
        DmxUsbProConfig {
            ids: ModuleIds::read(e),
            serial: SerialSettings {
                port_name: xmlutil::get_opt_string(e, "PortName"),
                baud_rate: xmlutil::get_i32(e, "BaudRate"),
                parity: Parity::from_name(&e.child_text("Partity").unwrap_or_default()),
                data_bits: xmlutil::get_i32(e, "DataBits"),
                stop_bits: StopBits::from_name(&e.child_text("StopBits").unwrap_or_default()),
            },
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "Data", &self.ids);
        e.push(num_el(&ns, "BaudRate", self.serial.baud_rate));
        e.push(num_el(&ns, "DataBits", self.serial.data_bits));
        e.push(text_el(&ns, "Partity", self.serial.parity.name()));
        e.push(opt_text_el(&ns, "PortName", self.serial.port_name.as_deref()));
        e.push(text_el(&ns, "StopBits", self.serial.stop_bits.name()));
        e
    }
}

/// `Message.Packet`.
pub fn message_packet(message_type: u8, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + data.len());
    write_message_packet(message_type, data, &mut out);
    out
}

fn write_message_packet(message_type: u8, data: &[u8], out: &mut Vec<u8>) {
    let len = data.len().min(600);
    out.clear();
    out.push(START_OF_MESSAGE);
    out.push(message_type);
    out.push(len as u8);
    out.push((len >> 8) as u8);
    out.extend_from_slice(&data[..len]);
    out.push(END_OF_MESSAGE);
}

/// `DmxUsbProSender.SendDmxPacket`, up to the write. A slot past the commands keeps its previous value, like
/// the long-lived `_statePacket`.
pub fn fill_state_packet(state: &mut [u8; 513], commands: &[OutputCommand]) {
    state[0] = 0;
    for (slot, c) in state[1..].iter_mut().zip(commands) {
        *slot = c.byte();
    }
}

/// `Module` with its `DmxUsbProSender`.
pub struct DmxUsbProModule {
    config: DmxUsbProConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    port: Option<Box<dyn SerialPort>>,
    state: [u8; 513],
    packet: Vec<u8>,
}

impl DmxUsbProModule {
    pub fn new(config: DmxUsbProConfig) -> DmxUsbProModule {
        DmxUsbProModule {
            config,
            shared: ModuleShared::new(),
            output_count: 0,
            port: None,
            state: [0; 513],
            packet: Vec::with_capacity(518),
        }
    }
}

impl OutputModule for DmxUsbProModule {
    fn type_id(&self) -> Uuid {
        ids::DMX_USB_PRO
    }

    fn start(&mut self) {
        self.port = None;
        // A new sender starts with a new `_statePacket`.
        self.state = [0; 513];
        if self.config.serial.port_name.is_none() {
            self.shared.set_status(ModuleStatus::Unavailable("No serial port is configured".to_string()));
            return;
        }
        match serial::open_port(&self.config.serial, false, false) {
            Ok(port) => {
                self.port = Some(port);
                self.shared.set_status(ModuleStatus::Running);
            }
            Err(e) => {
                // Serial Port Open failed: logged, and nothing is sent. There's no retry in this module.
                let is_unavailable = e.is_unavailable();
                let message = e.to_string();
                self.shared.record_error(message.clone());
                self.shared.set_status(if is_unavailable {
                    ModuleStatus::Unavailable(message)
                } else {
                    ModuleStatus::Error(message)
                });
            }
        }
    }

    fn stop(&mut self) {
        self.port = None;
        self.shared.set_status(ModuleStatus::Stopped);
    }

    fn update(&mut self, commands: &[OutputCommand]) {
        let Some(port) = self.port.as_mut() else {
            return;
        };
        fill_state_packet(&mut self.state, commands);
        write_message_packet(SEND_DMX_PACKET_REQUEST, &self.state, &mut self.packet);
        match serial::write_all(port.as_mut(), &self.packet, None, self.shared.stop_flag()) {
            Ok(()) => self.shared.count_packet(self.packet.len()),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => self.shared.record_error(format!("An error occurred outputting data: {e}")),
        }
    }

    fn output_count(&self) -> usize {
        self.output_count
    }

    fn set_output_count(&mut self, count: usize) {
        self.output_count = count;
    }

    fn config(&self) -> ControllerConfig {
        ControllerConfig::DmxUsbPro(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::DmxUsbPro(config) = config else {
            return Err(ConfigMismatch);
        };
        self.config = config;
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
    fn message_framing_is_start_type_length_data_end() {
        assert_eq!(message_packet(6, &[1, 2, 3]), vec![0x7E, 6, 3, 0, 1, 2, 3, 0xE7]);
        assert_eq!(message_packet(6, &[]), vec![0x7E, 6, 0, 0, 0xE7]);
        let long = vec![9u8; 513];
        let p = message_packet(6, &long);
        assert_eq!(&p[..4], &[0x7E, 6, 0x01, 0x02]);
        assert_eq!(p.len(), 518);
        assert_eq!(p[517], 0xE7);
    }

    #[test]
    fn message_data_is_capped_at_600() {
        let p = message_packet(6, &vec![1u8; 700]);
        assert_eq!(p.len(), 605);
        assert_eq!(&p[2..4], &[0x58, 0x02]);
    }

    #[test]
    fn state_packet_has_a_zero_start_code_and_keeps_old_slots() {
        let mut s = [7u8; 513];
        fill_state_packet(&mut s, &[OutputCommand::U8(1), OutputCommand::None, OutputCommand::U16(9)]);
        assert_eq!(&s[..5], &[0, 1, 0, 0, 7]);
    }

    #[test]
    fn only_512_channels_fit() {
        let mut s = [0u8; 513];
        fill_state_packet(&mut s, &vec![OutputCommand::U8(3); 600]);
        assert_eq!(s[512], 3);
    }

    #[test]
    fn a_windows_port_is_unavailable() {
        let mut m = DmxUsbProModule::new(DmxUsbProConfig {
            serial: SerialSettings {
                port_name: Some("COM3".to_string()),
                baud_rate: 115200,
                parity: Parity::None,
                data_bits: 8,
                stop_bits: StopBits::One,
            },
            ..Default::default()
        });
        m.start();
        assert!(matches!(m.status(), ModuleStatus::Unavailable(_)));
        m.update(&[OutputCommand::U8(1)]);
        assert_eq!(m.shared().packets_sent(), 0);
        m.stop();
    }
}
