//! Shared by the integration tests: the local stand-ins for hardware. Nothing here opens a real serial port or
//! sends anything off this machine.

#![allow(dead_code)]

use std::os::fd::RawFd;
use std::time::{Duration, Instant};

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
