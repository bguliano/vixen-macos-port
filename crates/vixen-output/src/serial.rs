//! Serial ports on macOS: the settings as Vixen stores them, listing, and opening with .NET `SerialPort` behaviour.

use serialport::SerialPort;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// `System.IO.Ports.Parity`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Parity {
    #[default]
    None,
    Odd,
    Even,
    Mark,
    Space,
}

impl Parity {
    pub fn name(self) -> &'static str {
        match self {
            Parity::None => "None",
            Parity::Odd => "Odd",
            Parity::Even => "Even",
            Parity::Mark => "Mark",
            Parity::Space => "Space",
        }
    }

    pub fn from_name(name: &str) -> Parity {
        match name {
            "Odd" => Parity::Odd,
            "Even" => Parity::Even,
            "Mark" => Parity::Mark,
            "Space" => Parity::Space,
            _ => Parity::None,
        }
    }
}

/// `System.IO.Ports.StopBits`. `None` is the default value of the enum, and modules treat it as not configured.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum StopBits {
    #[default]
    None,
    One,
    Two,
    OnePointFive,
}

impl StopBits {
    pub fn name(self) -> &'static str {
        match self {
            StopBits::None => "None",
            StopBits::One => "One",
            StopBits::Two => "Two",
            StopBits::OnePointFive => "OnePointFive",
        }
    }

    pub fn from_name(name: &str) -> StopBits {
        match name {
            "One" => StopBits::One,
            "Two" => StopBits::Two,
            "OnePointFive" => StopBits::OnePointFive,
            _ => StopBits::None,
        }
    }
}

/// The port settings shared by the serial controllers' data. The name stays exactly as saved, so a file from
/// Windows keeps its `COM4` until the user picks a port here.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SerialSettings {
    pub port_name: Option<String>,
    pub baud_rate: i32,
    pub parity: Parity,
    pub data_bits: i32,
    pub stop_bits: StopBits,
}

impl SerialSettings {
    /// `Data.IsValid` in the GenericSerial and Renard modules.
    pub fn is_valid(&self) -> bool {
        self.port_name.is_some() && self.baud_rate != 0 && self.data_bits != 0 && self.stop_bits != StopBits::None
    }
}

/// The rates `SerialPortConfig` lists.
pub const STANDARD_BAUD_RATES: [i32; 11] = [2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600, 1048576];

impl SerialSettings {
    /// What `SerialPortConfig` offers for a new port: 57600, no parity, 8 data bits, one stop bit.
    pub fn dialog_default(port_name: &str) -> SerialSettings {
        SerialSettings {
            port_name: Some(port_name.to_string()),
            baud_rate: 57600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        }
    }
}

/// `SerialPort.GetPortNames()` for macOS: the callout devices, sorted. Only lists names, never opens anything.
pub fn list_serial_ports() -> Vec<String> {
    let mut ports: Vec<String> = match std::fs::read_dir("/dev") {
        Ok(dir) => dir
            .flatten()
            .filter_map(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.starts_with("cu.").then(|| format!("/dev/{name}"))
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    ports.sort();
    ports
}

/// A callout device, with what IOKit knows about it. Reading this opens nothing.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SerialPortDetails {
    pub path: String,
    pub is_usb: bool,
    pub is_bluetooth: bool,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}

/// [`list_serial_ports`] with each port's USB details, where it has any.
pub fn list_serial_port_details() -> Vec<SerialPortDetails> {
    let known = serialport::available_ports().unwrap_or_default();
    list_serial_ports()
        .into_iter()
        .map(|path| {
            let mut details = SerialPortDetails { path: path.clone(), ..Default::default() };
            match known.iter().find(|p| p.port_name == path).map(|p| &p.port_type) {
                Some(serialport::SerialPortType::UsbPort(usb)) => {
                    details.is_usb = true;
                    details.vendor_id = Some(usb.vid);
                    details.product_id = Some(usb.pid);
                    details.serial_number = usb.serial_number.clone();
                    details.manufacturer = usb.manufacturer.clone();
                    details.product = usb.product.clone();
                }
                Some(serialport::SerialPortType::BluetoothPort) => details.is_bluetooth = true,
                _ => details.is_bluetooth = path.to_ascii_lowercase().contains("bluetooth"),
            }
            details
        })
        .collect()
}

/// True when more than a frame is already waiting to go out. A Mac's `write` returns as soon as the bytes are queued, so at 16 bytes and 9600 baud with a
/// 10 ms update, frames would pile up and the lights fall further and further behind the music.
pub fn frame_would_lag(queued: u32, frame_len: usize) -> bool {
    queued as usize > frame_len
}

/// Whether a saved name looks like a Windows port, which can't exist here: `COM` and a number.
pub fn is_windows_port_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    match upper.strip_prefix("COM") {
        Some(number) => !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()),
        None => false,
    }
}

/// Whether a saved port can be used on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortAvailability {
    Available,
    /// A saved Windows name such as `COM4`. It stays in the data, but there's no such device here.
    WindowsName,
    Missing,
}

pub fn port_availability(name: &str) -> PortAvailability {
    if is_windows_port_name(name) {
        PortAvailability::WindowsName
    } else if std::path::Path::new(name).exists() {
        PortAvailability::Available
    } else {
        PortAvailability::Missing
    }
}

/// `SerialPortConfig`'s scan: tries to open a port to see if it's free. This does touch the device, and opening a
/// port can reset some boards, so only call it when the user is choosing a port.
pub fn port_in_use(name: &str) -> bool {
    if port_availability(name) != PortAvailability::Available {
        return false;
    }
    match serialport::new(name, 0).timeout(Duration::from_millis(100)).open() {
        Ok(_) => false,
        // EBUSY reads as `NoDevice`: another program holds the port. `UnauthorizedAccessException` also reads as in use upstream.
        Err(e) => matches!(
            e.kind(),
            serialport::ErrorKind::NoDevice | serialport::ErrorKind::Io(std::io::ErrorKind::PermissionDenied)
        ),
    }
}

/// Why a port didn't open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortOpenError {
    /// Reported as unavailable. Never attempted, so the name can't be taken for a relative path.
    WindowsName(String),
    /// The device isn't there, or the system refused to open it.
    Unavailable(String),
    /// A setting this platform has no way to apply.
    UnsupportedSetting(String),
}

impl std::fmt::Display for PortOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortOpenError::WindowsName(n) => {
                write!(f, "{n} is a Windows port name and is not available on macOS; choose a port from /dev/cu.*")
            }
            PortOpenError::Unavailable(m) | PortOpenError::UnsupportedSetting(m) => write!(f, "{m}"),
        }
    }
}

impl PortOpenError {
    /// `true` for the two cases that report as unavailable rather than as a configuration error.
    pub fn is_unavailable(&self) -> bool {
        !matches!(self, PortOpenError::UnsupportedSetting(_))
    }
}

/// How long a single blocking write waits for the port before re-checking whether to give up.
const WRITE_POLL: Duration = Duration::from_millis(50);

/// How the serialport crate words ENOTTY, the only trace of the errno it keeps.
fn nix_enotty_description() -> &'static str {
    "Not a typewriter"
}

/// `new SerialPort(...)` then `Open()`, with `Handshake.None`.
pub(crate) fn open_port(settings: &SerialSettings, dtr: bool, rts: bool) -> Result<Box<dyn SerialPort>, PortOpenError> {
    let name = settings.port_name.as_deref().unwrap_or("");
    if is_windows_port_name(name) {
        return Err(PortOpenError::WindowsName(name.to_string()));
    }
    let baud: u32 = settings
        .baud_rate
        .try_into()
        .map_err(|_| PortOpenError::UnsupportedSetting(format!("Invalid baud rate {}", settings.baud_rate)))?;
    let data_bits = match settings.data_bits {
        5 => serialport::DataBits::Five,
        6 => serialport::DataBits::Six,
        7 => serialport::DataBits::Seven,
        8 => serialport::DataBits::Eight,
        other => return Err(PortOpenError::UnsupportedSetting(format!("Invalid data bits {other}"))),
    };
    let parity = match settings.parity {
        Parity::None => serialport::Parity::None,
        Parity::Odd => serialport::Parity::Odd,
        Parity::Even => serialport::Parity::Even,
        other => return Err(PortOpenError::UnsupportedSetting(format!("{} parity is not supported on macOS", other.name()))),
    };
    let stop_bits = match settings.stop_bits {
        StopBits::One => serialport::StopBits::One,
        StopBits::Two => serialport::StopBits::Two,
        StopBits::OnePointFive => {
            return Err(PortOpenError::UnsupportedSetting("1.5 stop bits are not supported on macOS".to_string()))
        }
        StopBits::None => return Err(PortOpenError::UnsupportedSetting("No stop bits configured".to_string())),
    };
    let builder = |baud: u32| {
        serialport::new(name, baud)
            .parity(parity)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .flow_control(serialport::FlowControl::None)
            .timeout(WRITE_POLL)
            .dtr_on_open(dtr)
    };
    let mut port = match builder(baud).open_native() {
        Ok(p) => p,
        // The baud ioctl fails with ENOTTY on a pseudo terminal, which has no baud rate to set.
        Err(e) if e.description == nix_enotty_description() => {
            builder(0).open_native().map_err(|e| PortOpenError::Unavailable(format!("{name}: {e}")))?
        }
        Err(e) => return Err(PortOpenError::Unavailable(format!("{name}: {e}"))),
    };
    if rts {
        // `RtsEnable`; a pseudo terminal has no such line, and that's not a failure.
        let _ = port.write_request_to_send(true);
    }
    // A blocking write of a whole packet can't be interrupted once it starts, and neither can the close after it. With this,
    // `write_all` does the waiting, so a port that has stopped taking data can't hold up shutting down.
    // SAFETY: only changes flags on a descriptor the port owns, and only for the duration of these two calls.
    unsafe {
        let fd = port.as_raw_fd();
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags >= 0 {
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
    Ok(Box::new(port))
}

/// `SerialPort.Write`: blocks until every byte is with the driver. `timeout` is `WriteTimeout`, with `None` being .NET's
/// default of waiting forever. The stop flag ends the wait, so a jammed port can't hold up shutting down.
pub(crate) fn write_all(
    port: &mut dyn SerialPort,
    mut data: &[u8],
    timeout: Option<Duration>,
    stop: &AtomicBool,
) -> std::io::Result<()> {
    let started = Instant::now();
    while !data.is_empty() {
        match port.write(data) {
            Ok(0) => return Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "the port accepted no data")),
            Ok(n) => data = &data[n..],
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(1)),
            Err(e) => return Err(e),
        }
        if !data.is_empty() {
            if stop.load(Ordering::Relaxed) {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "stopped while waiting for the port"));
            }
            if timeout.is_some_and(|t| started.elapsed() >= t) {
                return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "The write timed out."));
            }
        }
    }
    Ok(())
}

/// The modules' two-minute retry timer, for a port that didn't open: an initial failure and three more tries.
#[derive(Clone, Debug, Default)]
pub(crate) struct RetryTimer {
    counter: u32,
    next: Option<Instant>,
}

impl RetryTimer {
    const INTERVAL: Duration = Duration::from_secs(120);

    pub(crate) fn on_open_success(&mut self) {
        if self.next.is_some() {
            self.counter = 0;
            self.next = None;
        }
    }

    pub(crate) fn on_open_failure(&mut self) {
        self.on_open_failure_at(Instant::now());
    }

    fn on_open_failure_at(&mut self, now: Instant) {
        if self.counter < 3 {
            self.counter += 1;
            self.next = Some(now + Self::INTERVAL);
        } else {
            self.next = None;
            self.counter = 0;
        }
    }

    pub(crate) fn stop(&mut self) {
        self.next = None;
        self.counter = 0;
    }

    /// Whether the timer has elapsed.
    pub(crate) fn due(&mut self) -> bool {
        self.next.is_some_and(|t| Instant::now() >= t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_names_are_recognised() {
        assert!(is_windows_port_name("COM4"));
        assert!(is_windows_port_name("com12"));
        assert!(!is_windows_port_name("COM"));
        assert!(!is_windows_port_name("COMX"));
        assert!(!is_windows_port_name("/dev/cu.usbserial-1"));
    }

    #[test]
    fn a_windows_name_is_never_opened() {
        let settings = SerialSettings {
            port_name: Some("COM9".to_string()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        };
        let err = open_port(&settings, false, false).err().unwrap();
        assert_eq!(err, PortOpenError::WindowsName("COM9".to_string()));
        assert!(err.is_unavailable());
        assert!(err.to_string().contains("COM9"));
        assert_eq!(port_availability("COM9"), PortAvailability::WindowsName);
        assert!(!port_in_use("COM9"));
    }

    #[test]
    fn a_missing_device_reports_unavailable() {
        let settings = SerialSettings {
            port_name: Some("/dev/cu.vixen-no-such-port".to_string()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        };
        assert!(matches!(open_port(&settings, false, false), Err(PortOpenError::Unavailable(_))));
        assert_eq!(port_availability("/dev/cu.vixen-no-such-port"), PortAvailability::Missing);
    }

    #[test]
    fn settings_macos_has_no_way_to_apply_are_reported() {
        let mut settings = SerialSettings {
            port_name: Some("/dev/null".to_string()),
            baud_rate: 9600,
            parity: Parity::Mark,
            data_bits: 8,
            stop_bits: StopBits::One,
        };
        assert!(matches!(open_port(&settings, false, false), Err(PortOpenError::UnsupportedSetting(_))));
        settings.parity = Parity::None;
        settings.stop_bits = StopBits::OnePointFive;
        assert!(matches!(open_port(&settings, false, false), Err(PortOpenError::UnsupportedSetting(_))));
        settings.stop_bits = StopBits::One;
        settings.data_bits = 9;
        assert!(matches!(open_port(&settings, false, false), Err(PortOpenError::UnsupportedSetting(_))));
    }

    #[test]
    fn only_a_frame_may_wait_in_the_queue() {
        assert!(!frame_would_lag(0, 16));
        assert!(!frame_would_lag(16, 16));
        assert!(frame_would_lag(17, 16));
    }

    #[test]
    fn is_valid_matches_the_modules() {
        let mut s = SerialSettings {
            port_name: Some("COM1".to_string()),
            baud_rate: 9600,
            parity: Parity::None,
            data_bits: 8,
            stop_bits: StopBits::One,
        };
        assert!(s.is_valid());
        s.stop_bits = StopBits::None;
        assert!(!s.is_valid());
        s.stop_bits = StopBits::One;
        s.baud_rate = 0;
        assert!(!s.is_valid());
        s.baud_rate = 9600;
        s.port_name = None;
        assert!(!s.is_valid());
    }

    #[test]
    fn listing_only_returns_callout_devices() {
        assert!(list_serial_ports().iter().all(|p| p.starts_with("/dev/cu.")));
    }

    #[test]
    fn retry_timer_tries_three_more_times_then_gives_up() {
        let mut t = RetryTimer::default();
        let now = Instant::now();
        // The first failure and two retries re-arm the timer.
        for count in 1..=3 {
            t.on_open_failure_at(now);
            assert_eq!(t.counter, count);
            assert!(t.next.is_some());
        }
        // The third retry fails: stop and start over.
        t.on_open_failure_at(now);
        assert_eq!(t.counter, 0);
        assert!(t.next.is_none());
    }

    #[test]
    fn retry_timer_resets_after_opening() {
        let mut t = RetryTimer::default();
        t.on_open_failure_at(Instant::now());
        t.on_open_success();
        assert_eq!(t.counter, 0);
        assert!(t.next.is_none());
        assert!(!t.due());
    }
}
