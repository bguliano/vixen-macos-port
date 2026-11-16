//! Shared by the integration tests: the owner's real show files (read-only), plain test graphs, and the local stand-ins for
//! hardware. Nothing here opens a real serial port or sends anything off this machine.

#![allow(dead_code)]

use std::collections::HashMap;
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;
use vixen_output::graph::{ControllerDef, FilterDef, GraphDefinition, OutputDef, PatchDef};
use vixen_xml::XElement;

pub struct RealData {
    pub def: GraphDefinition,
    /// The `<Module>` elements from `ModuleStore.xml`, by (type, instance).
    pub modules: HashMap<(Uuid, Uuid), XElement>,
    pub element_names: Vec<String>,
    pub disabled: Vec<Uuid>,
    pub default_update_interval_ms: u64,
}

impl RealData {
    pub fn element(&self, name: &str) -> usize {
        self.element_names.iter().position(|n| n == name).unwrap_or_else(|| panic!("no element named {name}"))
    }
}

fn uuid_attr(e: &XElement, name: &str) -> Uuid {
    Uuid::parse_str(e.attr(name).unwrap_or_default()).unwrap_or_else(|_| panic!("bad {name} on <{}>", e.local))
}

/// The show at `VIXEN_TEST_DATA`, or the owner's copy. `None` when it isn't on this machine, and the test skips.
pub fn load_real() -> Option<RealData> {
    let root = std::env::var_os("VIXEN_TEST_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/developer/Documents/Vixen 3"));
    let system = root.join("SystemData");
    let config_path = system.join("SystemConfig.xml");
    let store_path = system.join("ModuleStore.xml");
    if !config_path.exists() || !store_path.exists() {
        eprintln!("skipping: no real data at {}", system.display());
        return None;
    }
    let config = XElement::from_file(&config_path).expect("SystemConfig.xml parses");
    let store = XElement::from_file(&store_path).expect("ModuleStore.xml parses");

    let channels = config.child("Channels").expect("Channels");
    let mut elements = Vec::new();
    let mut element_names = Vec::new();
    for c in channels.children_named("Channel") {
        elements.push(uuid_attr(c, "id"));
        element_names.push(c.attr("name").unwrap_or_default().to_string());
    }

    let mut controllers = Vec::new();
    for c in config.child("Controllers").expect("Controllers").children_named("Controller") {
        let outputs = c
            .child("Outputs")
            .map(|o| {
                o.children_named("Output")
                    .map(|x| OutputDef { id: uuid_attr(x, "id"), name: x.attr("name").unwrap_or_default().to_string() })
                    .collect()
            })
            .unwrap_or_default();
        controllers.push(ControllerDef {
            id: uuid_attr(c, "id"),
            name: c.attr("name").unwrap_or_default().to_string(),
            hardware_type_id: uuid_attr(c, "hardwareId"),
            hardware_instance_id: uuid_attr(c, "hardwareInstanceId"),
            outputs,
        });
    }

    let filters = config
        .child("Filters")
        .expect("Filters")
        .children_named("FilterNode")
        .map(|f| FilterDef { type_id: uuid_attr(f, "typeId"), instance_id: uuid_attr(f, "instanceId") })
        .collect();

    let patches = config
        .child("Patches")
        .expect("Patches")
        .children_named("Patch")
        .map(|p| PatchDef {
            component_id: uuid_attr(p, "componentId"),
            source_id: p.attr("sourceId").and_then(|s| Uuid::parse_str(s).ok()),
            output_index: p.attr("output").and_then(|s| s.parse().ok()).unwrap_or(0),
        })
        .collect();

    let disabled = config
        .child("DisabledDevices")
        .map(|d| d.children_named("Device").map(|x| uuid_attr(x, "id")).collect())
        .unwrap_or_default();

    let mut modules = HashMap::new();
    for set in store.children_named("ModuleData") {
        for m in set.children_named("Module") {
            modules.insert((uuid_attr(m, "moduleType"), uuid_attr(m, "moduleInstance")), m.clone());
        }
    }

    Some(RealData {
        def: GraphDefinition { elements, controllers, filters, patches },
        modules,
        element_names,
        disabled,
        default_update_interval_ms: config.child_text("DefaultUpdateInterval").and_then(|t| t.parse().ok()).unwrap_or(50),
    })
}

/// A pseudo terminal standing in for a serial device. The module opens the slave by its path, through the same path as a real
/// port, and the test reads the master end.
pub struct Pty {
    master: RawFd,
    held_slave: Option<RawFd>,
    pub path: String,
}

impl Pty {
    pub fn new() -> Pty {
        // SAFETY: plain libc calls on a descriptor owned by this struct. `ptsname`'s buffer is copied at once.
        unsafe {
            let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            assert!(master >= 0, "posix_openpt failed");
            assert_eq!(libc::grantpt(master), 0, "grantpt failed");
            assert_eq!(libc::unlockpt(master), 0, "unlockpt failed");
            let name = libc::ptsname(master);
            assert!(!name.is_null(), "ptsname failed");
            let path = std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned();
            let flags = libc::fcntl(master, libc::F_GETFL);
            libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
            Pty { master, held_slave: None, path }
        }
    }

    /// Keeps the terminal alive when the module closes its end between writes. A real serial device doesn't lose what
    /// was written when the port closes, but a pseudo terminal with no slave left does.
    pub fn hold_slave_open(&mut self) {
        let path = std::ffi::CString::new(self.path.clone()).expect("a path has no NUL");
        // SAFETY: `path` is a valid C string for the length of the call.
        let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK) };
        assert!(fd >= 0, "couldn't open the slave");
        self.held_slave = Some(fd);
    }

    /// Reads until `count` bytes have come or the time is up.
    pub fn read_exactly(&self, count: usize, timeout: Duration) -> Vec<u8> {
        let mut out = Vec::with_capacity(count);
        let deadline = Instant::now() + timeout;
        let mut buf = [0u8; 4096];
        while out.len() < count && Instant::now() < deadline {
            let want = (count - out.len()).min(buf.len());
            // SAFETY: `buf` has room for `want` bytes.
            let n = unsafe { libc::read(self.master, buf.as_mut_ptr().cast(), want) };
            if n > 0 {
                out.extend_from_slice(&buf[..n as usize]);
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        out
    }

    /// Whatever is waiting, after letting anything in flight arrive.
    pub fn drain(&self, wait: Duration) -> Vec<u8> {
        std::thread::sleep(wait);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            // SAFETY: `buf` has room for its length.
            let n = unsafe { libc::read(self.master, buf.as_mut_ptr().cast(), buf.len()) };
            if n <= 0 {
                return out;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // SAFETY: both descriptors are open and only this struct closes them.
        unsafe {
            if let Some(fd) = self.held_slave {
                libc::close(fd);
            }
            libc::close(self.master);
        }
    }
}

/// A UDP receiver on 127.0.0.1, standing in for a controller on the network.
pub struct UdpReceiver {
    socket: std::net::UdpSocket,
    pub port: u16,
}

impl UdpReceiver {
    pub fn new() -> UdpReceiver {
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind a local receiver");
        socket.set_read_timeout(Some(Duration::from_millis(500))).expect("set a read timeout");
        let port = socket.local_addr().expect("local address").port();
        UdpReceiver { socket, port }
    }

    /// The next packet.
    pub fn recv(&self) -> Vec<u8> {
        self.try_recv().expect("a packet should have arrived")
    }

    pub fn try_recv(&self) -> Option<Vec<u8>> {
        let mut buf = [0u8; 2048];
        self.socket.recv(&mut buf).ok().map(|n| buf[..n].to_vec())
    }

    /// Returns any packet that arrives soon, for checking nothing was sent.
    pub fn poll(&self, wait: Duration) -> Option<Vec<u8>> {
        self.socket.set_read_timeout(Some(wait)).expect("set a read timeout");
        let packet = self.try_recv();
        self.socket.set_read_timeout(Some(Duration::from_millis(500))).expect("set a read timeout");
        packet
    }
}
