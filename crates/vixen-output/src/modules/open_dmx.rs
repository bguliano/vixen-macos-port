//! Port of Vixen.Modules/Controller/OpenDMX. Upstream talks to the Enttec Open DMX through FTDI's D2XX driver, which conflicts with the system's FTDI
//! driver on a Mac, so here the adapter is an ordinary serial port: 250000 baud, 8N2, a break, then the start code and 512 values, over and over.

use super::{ConfigMismatch, ControllerConfig, ControllerMessage, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::ids;
use crate::serial::{self, list_serial_port_details, Parity, SerialPortDetails, SerialSettings, StopBits};
use crate::xmlutil::{self, data_root, num_el, opt_text_el, ModuleIds};
use parking_lot::Mutex;
use serialport::SerialPort;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.Controller.OpenDMX";

/// `Device`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OpenDmxDevice {
    pub description: Option<String>,
    pub id: Option<String>,
    pub index: i32,
    pub name: Option<String>,
    pub serial_number: Option<String>,
    pub device_type: Option<String>,
}

/// `OpenDMXData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenDmxConfig {
    pub ids: ModuleIds,
    pub device: Option<OpenDmxDevice>,
}

impl Default for OpenDmxConfig {
    fn default() -> Self {
        OpenDmxConfig { ids: ModuleIds::new(ids::OPEN_DMX, Uuid::nil()), device: None }
    }
}

impl OpenDmxConfig {
    pub fn from_xml(e: &XElement) -> OpenDmxConfig {
        OpenDmxConfig {
            ids: ModuleIds::read(e),
            device: e.child("Device").filter(|d| !d.nil).map(|d| OpenDmxDevice {
                description: xmlutil::get_opt_string(d, "Description"),
                id: xmlutil::get_opt_string(d, "Id"),
                index: xmlutil::get_i32(d, "Index"),
                name: xmlutil::get_opt_string(d, "Name"),
                serial_number: xmlutil::get_opt_string(d, "SerialNumber"),
                device_type: xmlutil::get_opt_string(d, "Type"),
            }),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "OpenDMXData", &self.ids);
        match &self.device {
            None => {
                e.push(XElement::nil(&ns, "Device"));
            }
            Some(d) => {
                let mut x = XElement::new(&ns, "Device");
                x.push(opt_text_el(&ns, "Description", d.description.as_deref()));
                x.push(opt_text_el(&ns, "Id", d.id.as_deref()));
                x.push(num_el(&ns, "Index", d.index));
                x.push(opt_text_el(&ns, "Name", d.name.as_deref()));
                x.push(opt_text_el(&ns, "SerialNumber", d.serial_number.as_deref()));
                x.push(opt_text_el(&ns, "Type", d.device_type.as_deref()));
                e.push(x);
            }
        }
        e
    }
}

/// `OpenDmx.GetDeviceList()`: the FTDI adapters among the serial ports, described as D2XX would, so a device saved on Windows still matches. `name` holds the path.
pub fn list_devices_from(details: &[SerialPortDetails]) -> Vec<OpenDmxDevice> {
    details
        .iter()
        .filter(|d| d.vendor_id == Some(0x0403))
        .enumerate()
        .map(|(index, d)| {
            let id = (u32::from(d.vendor_id.unwrap_or(0)) << 16) | u32::from(d.product_id.unwrap_or(0));
            OpenDmxDevice {
                description: d.product.clone(),
                id: Some(id.to_string()),
                index: index as i32,
                name: Some(d.path.clone()),
                serial_number: d.serial_number.clone(),
                device_type: None,
            }
        })
        .collect()
}

pub fn list_devices() -> Vec<OpenDmxDevice> {
    list_devices_from(&list_serial_port_details())
}

/// The saved device if it's here, otherwise the first (F108).
fn choose_device(saved: Option<&OpenDmxDevice>, devices: &[OpenDmxDevice]) -> Option<OpenDmxDevice> {
    saved
        .and_then(|s| devices.iter().find(|d| d.serial_number == s.serial_number && d.id == s.id && d.description == s.description))
        .or_else(|| devices.first())
        .cloned()
}

/// `VixenOpenDMXInstance`.
pub struct OpenDmxModule {
    config: OpenDmxConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    buffer: Arc<Mutex<[u8; 513]>>,
    device_list: fn() -> Vec<OpenDmxDevice>,
    done: Arc<AtomicBool>,
    writer: Option<JoinHandle<()>>,
}

impl OpenDmxModule {
    pub fn new(config: OpenDmxConfig) -> OpenDmxModule {
        OpenDmxModule::with_device_list(config, list_devices)
    }

    /// With the adapters to look for coming from elsewhere, as tests need.
    pub fn with_device_list(config: OpenDmxConfig, device_list: fn() -> Vec<OpenDmxDevice>) -> OpenDmxModule {
        OpenDmxModule {
            config,
            shared: ModuleShared::new(),
            output_count: 0,
            buffer: Arc::new(Mutex::new([0; 513])),
            device_list,
            done: Arc::new(AtomicBool::new(false)),
            writer: None,
        }
    }

    /// `_buffer`: start code, then the slots.
    pub fn buffer(&self) -> [u8; 513] {
        *self.buffer.lock()
    }

    /// F106: failing to start is an alert, since Configure may have asked for it. It also shows on the controller's row.
    fn start_failed(&self, text: String) {
        self.shared.record_error(text.clone());
        self.shared.set_status(ModuleStatus::Unavailable(text.clone()));
        self.shared.post_message(ControllerMessage { title: "OpenDMX".to_string(), heading: None, text });
    }

    fn stop_writer(&mut self) {
        self.done.store(true, Ordering::Relaxed);
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

impl Drop for OpenDmxModule {
    fn drop(&mut self) {
        self.stop_writer();
    }
}

/// `WriteData`: waits for the last frame to go, then a break and the next frame. An ioctl pair alone could make a break shorter than DMX's 88 µs.
fn write_frames(mut port: Box<dyn SerialPort>, buffer: Arc<Mutex<[u8; 513]>>, done: Arc<AtomicBool>, shared: Arc<ModuleShared>) {
    while !done.load(Ordering::Relaxed) {
        while port.bytes_to_write().unwrap_or(0) != 0 {
            if done.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let frame = *buffer.lock();
        let sent = port
            .set_break()
            .and_then(|()| {
                std::thread::sleep(Duration::from_micros(100));
                port.clear_break()
            })
            .map_err(std::io::Error::from)
            .and_then(|()| serial::write_all(port.as_mut(), &frame, None, &done));
        if let Err(e) = sent {
            if e.kind() != std::io::ErrorKind::Interrupted {
                let message = format!("An error occurred writing to the OpenDMX device: {e}");
                shared.record_error(message.clone());
                shared.set_status(ModuleStatus::Error(message));
            }
            return;
        }
        shared.count_packet(frame.len());
        std::thread::sleep(Duration::from_millis(25));
    }
}

impl OutputModule for OpenDmxModule {
    fn type_id(&self) -> Uuid {
        ids::OPEN_DMX
    }

    fn start(&mut self) {
        self.stop_writer();
        *self.buffer.lock() = [0; 513];
        let Some(device) = choose_device(self.config.device.as_ref(), &(self.device_list)()) else {
            self.start_failed("No devices found to open.".to_string());
            return;
        };
        let settings = SerialSettings {
            port_name: device.name.clone(),
            baud_rate: 250_000,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::Two,
        };
        self.config.device = Some(device);
        match serial::open_port(&settings, false, false) {
            Ok(port) => {
                // A fresh flag each time: upstream's never clears, so a restarted upstream module sends nothing.
                self.done = Arc::new(AtomicBool::new(false));
                let (buffer, done, shared) = (Arc::clone(&self.buffer), Arc::clone(&self.done), Arc::clone(&self.shared));
                self.writer = std::thread::Builder::new()
                    .name("Open DMX writer".to_string())
                    .spawn(move || write_frames(port, buffer, done, shared))
                    .ok();
                self.shared.set_status(ModuleStatus::Running);
            }
            Err(e) => self.start_failed(format!("Failed to open OpenDMX device. Error from Driver: {e}")),
        }
    }

    fn stop(&mut self) {
        self.stop_writer();
        self.shared.set_status(ModuleStatus::Stopped);
    }

    fn update(&mut self, commands: &[OutputCommand]) {
        let mut buffer = self.buffer.lock();
        for (slot, c) in buffer[1..].iter_mut().zip(commands) {
            *slot = c.byte();
        }
    }

    fn output_count(&self) -> usize {
        self.output_count
    }

    fn set_output_count(&mut self, count: usize) {
        // The C# setter throws above the limit.
        if count > self.output_limit() {
            self.shared.record_error("Output count greater than 512");
        } else {
            self.output_count = count;
        }
    }

    fn output_limit(&self) -> usize {
        512
    }

    fn config(&self) -> ControllerConfig {
        ControllerConfig::OpenDmx(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::OpenDmx(config) = config else {
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

    fn no_adapters() -> Vec<OpenDmxDevice> {
        Vec::new()
    }

    #[test]
    fn with_no_adapter_it_says_so_and_never_crashes() {
        let mut m = OpenDmxModule::with_device_list(OpenDmxConfig::default(), no_adapters);
        m.set_output_count(4);
        m.start();
        assert_eq!(m.status(), ModuleStatus::Unavailable("No devices found to open.".to_string()));
        let messages = m.shared().take_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "No devices found to open.");
        m.update(&[OutputCommand::U8(5), OutputCommand::None, OutputCommand::U8(6)]);
        assert_eq!(&m.buffer()[..4], &[0, 5, 0, 6]);
        m.stop();
        assert_eq!(m.status(), ModuleStatus::Stopped);
    }

    #[test]
    fn the_saved_adapter_wins_and_otherwise_the_first() {
        let device = |serial: &str| OpenDmxDevice { serial_number: Some(serial.to_string()), ..Default::default() };
        let devices = [device("A"), device("B")];
        assert_eq!(choose_device(Some(&device("B")), &devices), Some(device("B")));
        assert_eq!(choose_device(Some(&device("gone")), &devices), Some(device("A")));
        assert_eq!(choose_device(None, &devices), Some(device("A")));
        assert_eq!(choose_device(Some(&device("B")), &[]), None);
    }

    #[test]
    fn the_output_limit_holds() {
        let mut m = OpenDmxModule::with_device_list(OpenDmxConfig::default(), no_adapters);
        m.set_output_count(512);
        m.set_output_count(513);
        assert_eq!(m.output_count(), 512);
        assert_eq!(m.shared().error_count(), 1);
        m.update(&vec![OutputCommand::U8(1); 600]);
        assert_eq!(m.buffer()[512], 1);
    }
}
