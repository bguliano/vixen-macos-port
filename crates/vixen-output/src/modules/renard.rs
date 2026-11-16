//! Port of Vixen.Modules/Controller/Renard. Protocol 1 is the only one Vixen has: its Protocol2 class is commented out.

use super::{ConfigMismatch, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::ids;
use crate::serial::{self, Parity, RetryTimer, SerialSettings, StopBits};
use crate::xmlutil::{self, data_root, num_el, opt_text_el, text_el, ModuleIds};
use serialport::SerialPort;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.Output.Renard";
const DEFAULT_WRITE_TIMEOUT: i32 = 500;

/// .NET's default `SerialPort.WriteBufferSize`.
const WRITE_BUFFER_SIZE: u32 = 2048;

/// `Data`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenardConfig {
    pub ids: ModuleIds,
    pub serial: SerialSettings,
    pub protocol_version: i32,
    pub write_timeout: i32,
}

impl Default for RenardConfig {
    fn default() -> Self {
        RenardConfig {
            ids: ModuleIds::new(ids::RENARD, Uuid::nil()),
            serial: SerialSettings::default(),
            protocol_version: 0,
            write_timeout: 0,
        }
    }
}

impl RenardConfig {
    pub fn from_xml(e: &XElement) -> RenardConfig {
        RenardConfig {
            ids: ModuleIds::read(e),
            serial: SerialSettings {
                port_name: xmlutil::get_opt_string(e, "PortName"),
                baud_rate: xmlutil::get_i32(e, "BaudRate"),
                parity: Parity::from_name(&e.child_text("Parity").unwrap_or_default()),
                data_bits: xmlutil::get_i32(e, "DataBits"),
                stop_bits: StopBits::from_name(&e.child_text("StopBits").unwrap_or_default()),
            },
            protocol_version: xmlutil::get_i32(e, "ProtocolVersion"),
            write_timeout: xmlutil::get_i32(e, "WriteTimeout"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "Data", &self.ids);
        e.push(num_el(&ns, "BaudRate", self.serial.baud_rate));
        e.push(num_el(&ns, "DataBits", self.serial.data_bits));
        e.push(text_el(&ns, "Parity", self.serial.parity.name()));
        e.push(opt_text_el(&ns, "PortName", self.serial.port_name.as_deref()));
        e.push(num_el(&ns, "ProtocolVersion", self.protocol_version));
        e.push(text_el(&ns, "StopBits", self.serial.stop_bits.name()));
        e.push(num_el(&ns, "WriteTimeout", self.write_timeout));
        e
    }
}

/// Port of `Protocol1`. One byte is reserved as the frame marker and two more can't appear in the data, so
/// they're remapped, and a pad byte goes in every 100 bytes so clocks that are 1% out can catch up.
#[derive(Clone, Debug, Default)]
pub struct Protocol1 {
    packet: Vec<u8>,
    dst_index: usize,
    output_count: Option<usize>,
}

/// The packet overran its array. The maximum-length calculation doesn't count a pad byte that lands on the last
/// value, so at some output counts (197, for one) the C# throws from `_Add` every frame and nothing is sent.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PacketOverflow;

impl Protocol1 {
    const PAD_DISTANCE: usize = 100;
    const FRAME_MARKER: u8 = 0x7E;
    const PIC_INDEX_OFFSET: i32 = 0x80;
    const PAD_BYTE: u8 = 0x7D;

    /// `_valueMap`: 0x7D and 0x7E become 124, 0x7F becomes 128.
    #[inline]
    pub fn map_value(level: u8) -> u8 {
        match level {
            0x7D | 0x7E => 124,
            0x7F => 128,
            other => other,
        }
    }

    pub fn start_packet(&mut self, output_count: usize, chain_index: i32) -> Result<(), PacketOverflow> {
        if self.output_count != Some(output_count) {
            self.output_count = Some(output_count);
            let unpadded = 2 + output_count;
            self.packet = vec![0; unpadded + unpadded / Self::PAD_DISTANCE];
        }
        self.dst_index = 0;
        self.push(Self::FRAME_MARKER)?;
        self.push((Self::PIC_INDEX_OFFSET + chain_index) as u8)
    }

    #[inline]
    pub fn add(&mut self, value: u8) -> Result<(), PacketOverflow> {
        self.push(Self::map_value(value))
    }

    /// The bytes written so far.
    pub fn packet(&self) -> &[u8] {
        &self.packet[..self.dst_index]
    }

    #[inline]
    fn push(&mut self, value: u8) -> Result<(), PacketOverflow> {
        *self.packet.get_mut(self.dst_index).ok_or(PacketOverflow)? = value;
        self.dst_index += 1;
        if self.dst_index.is_multiple_of(Self::PAD_DISTANCE) {
            *self.packet.get_mut(self.dst_index).ok_or(PacketOverflow)? = Self::PAD_BYTE;
            self.dst_index += 1;
        }
        Ok(())
    }

    /// Builds a whole frame.
    pub fn build(&mut self, chain_index: i32, output_count: usize, commands: &[OutputCommand]) -> Result<&[u8], PacketOverflow> {
        self.start_packet(output_count, chain_index)?;
        for c in commands {
            self.add(c.byte())?;
        }
        Ok(self.packet())
    }
}

/// `Module`.
pub struct RenardModule {
    config: RenardConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    port: Option<Box<dyn SerialPort>>,
    running: bool,
    /// `None` for a protocol version with no formatter, where the C#'s `ModuleData` setter throws.
    formatter: Option<Protocol1>,
    retry: RetryTimer,
}

impl RenardModule {
    pub fn new(config: RenardConfig) -> RenardModule {
        let mut m = RenardModule {
            config,
            shared: ModuleShared::new(),
            output_count: 0,
            port: None,
            running: false,
            formatter: None,
            retry: RetryTimer::default(),
        };
        m.apply_config();
        m
    }

    /// The `ModuleData` setter: it fills in the defaults, and that's what is saved later.
    fn apply_config(&mut self) {
        if self.config.write_timeout == 0 {
            self.config.write_timeout = DEFAULT_WRITE_TIMEOUT;
        }
        if self.config.protocol_version == 0 {
            self.config.protocol_version = 1;
        }
        self.port = None;
        self.formatter = (self.config.protocol_version == 1).then(Protocol1::default);
        if self.formatter.is_none() {
            self.shared
                .record_error(format!("Renard module using an unsupported protocol version ({})", self.config.protocol_version));
        }
        if self.running && self.config.serial.is_valid() {
            self.open_com_port();
        }
    }

    /// `_OpenComPort`.
    fn open_com_port(&mut self) {
        match serial::open_port(&self.config.serial, true, true) {
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
                if is_unavailable {
                    self.retry.on_open_failure();
                } else {
                    self.retry.stop();
                }
            }
        }
    }
}

impl OutputModule for RenardModule {
    fn type_id(&self) -> Uuid {
        ids::RENARD
    }

    fn start(&mut self) {
        self.running = true;
        if self.formatter.is_none() {
            self.shared.set_status(ModuleStatus::Error(format!(
                "Renard protocol version {} is not supported",
                self.config.protocol_version
            )));
        } else if !self.config.serial.is_valid() {
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
        let (Some(port), Some(formatter)) = (self.port.as_mut(), self.formatter.as_mut()) else {
            return;
        };
        let Ok(_) = formatter.build(0, self.output_count, commands) else {
            self.shared.record_error("An error occurred outputting data: the Renard packet overran its buffer");
            return;
        };
        let size = formatter.packet().len() as u32;
        // `_WaitForBufferRoom`. For a packet of the whole buffer or more this never finishes, in the C# too, so
        // those controllers never send. Here stopping the controller at least ends the wait.
        while WRITE_BUFFER_SIZE.saturating_sub(port.bytes_to_write().unwrap_or(0)) <= size {
            if self.shared.stop_requested() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let timeout = Duration::from_millis(self.config.write_timeout.max(0) as u64);
        match serial::write_all(port.as_mut(), formatter.packet(), Some(timeout), self.shared.stop_flag()) {
            Ok(()) => self.shared.count_packet(size as usize),
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
        ControllerConfig::Renard(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::Renard(config) = config else {
            return Err(ConfigMismatch);
        };
        self.config = config;
        self.apply_config();
        Ok(())
    }

    fn shared(&self) -> &Arc<ModuleShared> {
        &self.shared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u8s(values: &[u8]) -> Vec<OutputCommand> {
        values.iter().map(|v| OutputCommand::U8(*v)).collect()
    }

    #[test]
    fn frame_starts_with_marker_and_pic_index() {
        let mut p = Protocol1::default();
        assert_eq!(p.build(0, 3, &u8s(&[1, 2, 3])).unwrap(), &[0x7E, 0x80, 1, 2, 3]);
        assert_eq!(p.build(5, 1, &u8s(&[9])).unwrap(), &[0x7E, 0x85, 9]);
    }

    #[test]
    fn reserved_values_are_remapped() {
        let mut p = Protocol1::default();
        let out = p.build(0, 6, &u8s(&[0x7C, 0x7D, 0x7E, 0x7F, 0x80, 0xFF])).unwrap();
        assert_eq!(out, &[0x7E, 0x80, 124, 124, 124, 128, 0x80, 0xFF]);
    }

    #[test]
    fn a_pad_byte_goes_in_every_100_bytes() {
        let mut p = Protocol1::default();
        // 2 preamble + 150 values: the pad lands after byte index 99.
        let values = vec![0x11; 150];
        let out = p.build(0, 150, &u8s(&values)).unwrap().to_vec();
        assert_eq!(out.len(), 153);
        assert_eq!(out[99], 0x11);
        assert_eq!(out[100], 0x7D);
        assert_eq!(out[101], 0x11);
        assert_eq!(out.iter().filter(|b| **b == 0x7D).count(), 1);
    }

    #[test]
    fn a_pad_at_the_exact_end_fits_when_the_calculation_allows_for_it() {
        let mut p = Protocol1::default();
        let out = p.build(0, 98, &u8s(&vec![1; 98])).unwrap();
        assert_eq!(out.len(), 101);
        assert_eq!(out[100], 0x7D);
    }

    #[test]
    fn the_output_count_the_max_length_misses_loses_the_frame() {
        // 197 outputs: 199 bytes, plus the first pad, reaches index 200, whose pad doesn't fit.
        let mut p = Protocol1::default();
        assert_eq!(p.build(0, 197, &u8s(&vec![1; 197])).err(), Some(PacketOverflow));
        assert!(p.build(0, 196, &u8s(&vec![1; 196])).is_ok());
        assert!(p.build(0, 198, &u8s(&vec![1; 198])).is_ok());
    }

    #[test]
    fn unsupported_protocol_reports_an_error_and_sends_nothing() {
        let cfg = RenardConfig {
            serial: SerialSettings {
                port_name: Some("COM1".to_string()),
                baud_rate: 57600,
                parity: Parity::None,
                data_bits: 8,
                stop_bits: StopBits::One,
            },
            protocol_version: 2,
            ..Default::default()
        };
        let mut m = RenardModule::new(cfg);
        m.set_output_count(2);
        m.start();
        assert!(matches!(m.status(), ModuleStatus::Error(_)));
        m.update(&u8s(&[1, 2]));
        assert_eq!(m.shared().packets_sent(), 0);
    }

    #[test]
    fn the_data_setter_fills_in_defaults() {
        let m = RenardModule::new(RenardConfig::default());
        let ControllerConfig::Renard(c) = m.config() else { panic!() };
        assert_eq!(c.write_timeout, 500);
        assert_eq!(c.protocol_version, 1);
    }

    #[test]
    fn a_windows_port_is_unavailable() {
        let cfg = RenardConfig {
            serial: SerialSettings {
                port_name: Some("COM7".to_string()),
                baud_rate: 57600,
                parity: Parity::None,
                data_bits: 8,
                stop_bits: StopBits::One,
            },
            ..Default::default()
        };
        let mut m = RenardModule::new(cfg);
        m.start();
        assert!(matches!(m.status(), ModuleStatus::Unavailable(_)));
        m.stop();
    }
}
