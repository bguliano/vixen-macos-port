//! Port of Vixen.Modules/Controller/ElexolEtherIO. It only uses a UDP socket, so nothing about it is Windows-specific.

use super::{ConfigMismatch, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::ids;
use crate::xmlutil::{self, data_root, num_el, opt_text_el, ModuleIds};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::os::fd::AsRawFd;
use std::sync::Arc;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.Output.ElexolEtherIO";

/// Puts ports A, B and C in output mode and at 0.
pub const INIT_PACKET: [u8; 15] = [b'!', b'A', 0, b'A', 0, b'!', b'B', 0, b'B', 0, b'!', b'C', 0, b'C', 0];

/// All three ports at 0, sent when stopping.
pub const OFF_PACKET: [u8; 6] = [b'A', 0, b'B', 0, b'C', 0];

/// `ElexolEtherIOData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElexolConfig {
    pub ids: ModuleIds,
    pub address: Option<String>,
    pub minimum_intensity: i32,
    pub port: i32,
}

impl Default for ElexolConfig {
    fn default() -> Self {
        ElexolConfig { ids: ModuleIds::new(ids::ELEXOL_ETHER_IO, Uuid::nil()), address: None, minimum_intensity: 0, port: 0 }
    }
}

impl ElexolConfig {
    pub fn from_xml(e: &XElement) -> ElexolConfig {
        ElexolConfig {
            ids: ModuleIds::read(e),
            address: xmlutil::get_opt_string(e, "Address"),
            minimum_intensity: xmlutil::get_i32(e, "MinimumIntensity"),
            port: xmlutil::get_i32(e, "Port"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "ElexolEtherIOData", &self.ids);
        e.push(opt_text_el(&ns, "Address", self.address.as_deref()));
        e.push(num_el(&ns, "MinimumIntensity", self.minimum_intensity));
        e.push(num_el(&ns, "Port", self.port));
        e
    }
}

/// The 6-byte state packet: a port letter, then one bit per channel set when the value is above the minimum.
pub fn state_packet(commands: &[OutputCommand], min_intensity: i32) -> [u8; 6] {
    let mut buf = [0u8; 6];
    let mut chan = 0;
    let mut i = 0;
    for port in *b"ABC" {
        buf[i] = port;
        i += 1;
        for bit in 0..8 {
            let Some(c) = commands.get(chan) else {
                break;
            };
            if c.byte() as i32 > min_intensity {
                buf[i] |= 1 << bit;
            }
            chan += 1;
        }
        i += 1;
    }
    buf
}

/// `UdpClient.Available`.
fn available(socket: &UdpSocket) -> usize {
    let mut n: libc::c_int = 0;
    // SAFETY: FIONREAD writes one int to the pointer given, and the fd is open for the life of the borrow.
    let result = unsafe { libc::ioctl(socket.as_raw_fd(), libc::FIONREAD, &mut n as *mut libc::c_int) };
    if result == 0 {
        n.max(0) as usize
    } else {
        0
    }
}

/// Why the setup dialog's test didn't succeed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElexolTestError {
    NoReply,
    /// The device answered with something other than the echo.
    WrongReply(Vec<u8>),
    Socket(String),
}

/// The setup dialog's Test button: the device echoes a `0x60` command, and gets up to a second to do it.
pub fn test_connection(address: &str, port: u16) -> Result<(), ElexolTestError> {
    const REQUEST: [u8; 2] = [0x60, 0x69];
    let socket_error = |e: std::io::Error| ElexolTestError::Socket(e.to_string());
    let ip: IpAddr = address.parse().map_err(|_| ElexolTestError::Socket(format!("{address} is not a valid IP address")))?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(socket_error)?;
    socket.connect(SocketAddr::new(ip, port)).map_err(socket_error)?;
    socket.set_read_timeout(Some(std::time::Duration::from_secs(1))).map_err(socket_error)?;
    socket.send(&REQUEST).map_err(socket_error)?;
    let mut reply = [0u8; 64];
    match socket.recv(&mut reply) {
        Ok(n) if reply[..n] == REQUEST => Ok(()),
        Ok(n) => Err(ElexolTestError::WrongReply(reply[..n].to_vec())),
        Err(_) => Err(ElexolTestError::NoReply),
    }
}

/// `ElexolEtherIOModule`.
pub struct ElexolModule {
    config: ElexolConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    min_intensity: i32,
    remote_port: u16,
    remote_address: Option<IpAddr>,
    socket: Option<UdpSocket>,
}

impl ElexolModule {
    pub fn new(config: ElexolConfig) -> ElexolModule {
        let mut m = ElexolModule {
            config,
            shared: ModuleShared::new(),
            output_count: 0,
            min_intensity: 1,
            remote_port: 2424,
            remote_address: None,
            socket: None,
        };
        m.init_module();
        m
    }

    /// `initModule`. Values that aren't set leave the previous ones in place, including the defaults.
    fn init_module(&mut self) {
        // F113: once configured, the saved value counts, including 0 and 1.
        if self.config.minimum_intensity > 1 || self.config.address.is_some() {
            self.min_intensity = self.config.minimum_intensity;
        }
        if self.config.port > 0 {
            self.remote_port = self.config.port as u16;
        }
        if let Some(address) = &self.config.address {
            match address.parse() {
                Ok(ip) => self.remote_address = Some(ip),
                Err(_) => self.shared.record_error(format!("{address} is not a valid IP address")),
            }
        }
    }

    /// `OpenConnection`: a new socket and the init sequence.
    fn open_connection(&mut self) -> bool {
        let Some(address) = self.remote_address else {
            return false;
        };
        self.socket = None;
        let attempt = (|| {
            let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
            socket.connect(SocketAddr::new(address, self.remote_port))?;
            socket.send(&INIT_PACKET)?;
            Ok::<UdpSocket, std::io::Error>(socket)
        })();
        match attempt {
            Ok(socket) => {
                self.shared.count_packet(INIT_PACKET.len());
                self.socket = Some(socket);
                true
            }
            Err(_) => false,
        }
    }
}

impl OutputModule for ElexolModule {
    fn type_id(&self) -> Uuid {
        ids::ELEXOL_ETHER_IO
    }

    fn start(&mut self) {
        let connected = self.open_connection();
        self.shared.set_status(if connected {
            ModuleStatus::Running
        } else {
            ModuleStatus::Unavailable("Elexol Ether I/O: failed to connect to device".to_string())
        });
    }

    fn stop(&mut self) {
        if let Some(socket) = self.socket.take() {
            if socket.send(&OFF_PACKET).is_ok() {
                self.shared.count_packet(OFF_PACKET.len());
            }
        }
        self.shared.set_status(ModuleStatus::Stopped);
    }

    /// With data waiting from the device, drains it and skips the frame. Upstream also opens a new socket and resends the init bytes on every frame with
    /// nothing waiting, and this connects once.
    fn update(&mut self, commands: &[OutputCommand]) {
        let waiting = self.socket.as_ref().map_or(0, available);
        if waiting != 0 {
            if let Some(socket) = &self.socket {
                let mut scratch = [0u8; 2048];
                while available(socket) > 0 {
                    if socket.recv(&mut scratch).is_err() {
                        break;
                    }
                }
            }
            self.shared.record_error("Elexol Ether I/O: Unexpected data in the buffer was cleared. Skipping update.");
            return;
        }
        if self.socket.is_none() && !self.open_connection() {
            self.shared.record_error("Elexol Ether I/O: failed to connect to device, not updating the current state.");
            return;
        }
        let buf = state_packet(commands, self.min_intensity);
        if let Some(socket) = &self.socket {
            match socket.send(&buf) {
                Ok(_) => self.shared.count_packet(buf.len()),
                Err(e) => self.shared.record_error(format!("An exception occurred trying to update the state: {e}")),
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
        ControllerConfig::Elexol(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::Elexol(config) = config else {
            return Err(ConfigMismatch);
        };
        self.config = config;
        self.init_module();
        // F113: upstream only assigns above 1, so lowering the threshold to 0 or 1 in the dialog wouldn't take effect.
        self.min_intensity = self.config.minimum_intensity;
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
    fn state_packet_sets_a_bit_per_channel_above_the_minimum() {
        assert_eq!(state_packet(&[], 1), [b'A', 0, b'B', 0, b'C', 0]);
        // The value must be above the minimum, not at it.
        assert_eq!(state_packet(&u8s(&[1, 2, 0, 255]), 1), [b'A', 0b1010, b'B', 0, b'C', 0]);
        let all = vec![255u8; 30];
        assert_eq!(state_packet(&u8s(&all), 1), [b'A', 0xFF, b'B', 0xFF, b'C', 0xFF]);
        let mut sparse = vec![0u8; 24];
        sparse[8] = 200;
        sparse[23] = 200;
        assert_eq!(state_packet(&u8s(&sparse), 100), [b'A', 0, b'B', 1, b'C', 0x80]);
    }

    #[test]
    fn defaults_stay_until_the_data_sets_something_above_them() {
        let m = ElexolModule::new(ElexolConfig { minimum_intensity: 1, port: 0, ..Default::default() });
        assert_eq!(m.min_intensity, 1);
        assert_eq!(m.remote_port, 2424);
        let m = ElexolModule::new(ElexolConfig {
            minimum_intensity: 40,
            port: 3000,
            address: Some("10.0.0.2".to_string()),
            ..Default::default()
        });
        assert_eq!(m.min_intensity, 40);
        assert_eq!(m.remote_port, 3000);
        assert_eq!(m.remote_address, Some("10.0.0.2".parse().unwrap()));
    }

    #[test]
    fn the_connection_test_expects_its_bytes_back() {
        let device = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = device.local_addr().unwrap().port();
        let echo = std::thread::spawn(move || {
            let mut buf = [0u8; 16];
            let (n, from) = device.recv_from(&mut buf).unwrap();
            assert_eq!(&buf[..n], &[0x60, 0x69]);
            device.send_to(&buf[..n], from).unwrap();
            let (_, from) = device.recv_from(&mut buf).unwrap();
            device.send_to(&[1, 2], from).unwrap();
        });
        assert_eq!(test_connection("127.0.0.1", port), Ok(()));
        assert_eq!(test_connection("127.0.0.1", port), Err(ElexolTestError::WrongReply(vec![1, 2])));
        echo.join().unwrap();
        assert!(matches!(test_connection("not an address", 1), Err(ElexolTestError::Socket(_))));
    }

    #[test]
    fn no_address_stays_quiet() {
        let mut m = ElexolModule::new(ElexolConfig::default());
        m.start();
        assert!(matches!(m.status(), ModuleStatus::Unavailable(_)));
        m.update(&u8s(&[1]));
        assert_eq!(m.shared().packets_sent(), 0);
        m.stop();
    }
}
