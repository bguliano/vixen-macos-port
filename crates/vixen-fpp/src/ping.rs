//! The export wizard's reachability check (F26): one ICMP echo, as upstream's `Ping.Send(host, 500)`.

use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::os::fd::FromRawFd;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

const PAYLOAD: &[u8; 8] = b"vixenfpp";
static SEQUENCE: AtomicU16 = AtomicU16::new(1);

/// True when `host` answers one echo request within `timeout`. A name is looked up first, which can take longer than `timeout`.
/// The socket is the unprivileged datagram kind macOS allows; if it can't be made the answer is false, like a timeout.
pub fn ping(host: &str, timeout: Duration) -> bool {
    let host = host.trim();
    if std::env::var_os("VIXEN_UI_TEST").is_some() && !crate::host::loopback_host(host) { return false; }
    if host.is_empty() {
        return false;
    }
    let address = match host.parse::<IpAddr>() {
        Ok(address) => Some(address),
        Err(_) => (host, 0).to_socket_addrs().ok().and_then(|mut found| found.next()).map(|a| a.ip()),
    };
    address.is_some_and(|address| echo(address, timeout))
}

fn echo(address: IpAddr, timeout: Duration) -> bool {
    let (family, protocol, request, reply) = match address {
        IpAddr::V4(_) => (libc::AF_INET, libc::IPPROTO_ICMP, 8u8, 0u8),
        IpAddr::V6(_) => (libc::AF_INET6, libc::IPPROTO_ICMPV6, 128u8, 129u8),
    };
    // SAFETY: a plain socket call; the descriptor is owned by the `UdpSocket` from here on, which closes it.
    let socket = unsafe {
        let fd = libc::socket(family, libc::SOCK_DGRAM, protocol);
        if fd < 0 {
            return false;
        }
        UdpSocket::from_raw_fd(fd)
    };
    if socket.connect(SocketAddr::new(address, 0)).is_err() {
        return false;
    }
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut packet = [0u8; 16];
    packet[0] = request;
    packet[4..6].copy_from_slice(&(std::process::id() as u16).to_be_bytes());
    packet[6..8].copy_from_slice(&sequence.to_be_bytes());
    packet[8..].copy_from_slice(PAYLOAD);
    if address.is_ipv4() {
        // The kernel fills this in for ICMPv6 only.
        let sum = checksum(&packet);
        packet[2..4].copy_from_slice(&sum.to_be_bytes());
    }
    if socket.send(&packet).is_err() {
        return false;
    }
    let deadline = Instant::now() + timeout;
    let mut buffer = [0u8; 1500];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || socket.set_read_timeout(Some(left)).is_err() {
            return false;
        }
        let Ok(length) = socket.recv(&mut buffer) else { return false };
        let mut message = &buffer[..length];
        // macOS hands over the IPv4 header as well.
        if address.is_ipv4() && message.first().is_some_and(|b| b >> 4 == 4) {
            let header = usize::from(message[0] & 0x0f) * 4;
            message = message.get(header..).unwrap_or_default();
        }
        // The identifier isn't compared: some kernels replace it with their own.
        if message.len() >= 16 && message[0] == reply && message[6..8] == sequence.to_be_bytes() && &message[8..16] == PAYLOAD {
            return true;
        }
    }
}

fn checksum(bytes: &[u8]) -> u16 {
    let mut sum: u32 = bytes.chunks(2).map(|pair| u32::from(u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)]))).sum();
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksum_is_the_internet_one() {
        // RFC 1071's worked example.
        assert_eq!(checksum(&[0x00, 0x01, 0xf2, 0x03, 0xf4, 0xf5, 0xf6, 0xf7]), !0xddf2);
        assert_eq!(checksum(&[0xff]), 0x00ff);
    }

    #[test]
    fn loopback_answers_and_nothing_is_sent_for_a_blank_host() {
        assert!(ping("127.0.0.1", Duration::from_millis(500)));
        assert!(!ping("", Duration::from_millis(500)));
        assert!(!ping("   ", Duration::from_millis(500)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn an_unreachable_address_does_not_poison_the_next_echo() {
        for _ in 0..3 {
            assert!(!ping("127.0.0.2", Duration::from_millis(100)));
            assert!(ping("127.0.0.1", Duration::from_millis(500)));
        }
    }
}
