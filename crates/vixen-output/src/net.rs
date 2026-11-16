//! Network interfaces, for the E1.31 multicast setting. On macOS a .NET `NetworkInterface.Id` is the BSD name,
//! such as `en0`. A file saved on Windows holds a GUID, which simply won't be found here.

use std::ffi::CStr;
use std::net::Ipv4Addr;

/// What `System.Net.NetworkInformation.NetworkInterface` told the E1.31 module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkInterfaceInfo {
    /// The value stored in the E1.31 `Multicast` setting.
    pub id: String,
    pub name: String,
    pub ipv4_addresses: Vec<Ipv4Addr>,
    /// `OperationalStatus.Up`.
    pub is_up: bool,
    pub supports_multicast: bool,
    pub is_loopback: bool,
    /// `NetworkInterfaceType.Tunnel`.
    pub is_tunnel: bool,
}

impl NetworkInterfaceInfo {
    /// The interfaces `E131OutputPlugin.Start` considers: anything but tunnels and loopback.
    pub fn is_usable_for_e131(&self) -> bool {
        !self.is_tunnel && !self.is_loopback
    }

    /// What the setup dialog offers as a multicast destination.
    pub fn is_offered_for_multicast(&self) -> bool {
        self.is_usable_for_e131() && self.supports_multicast
    }
}

/// `NetworkInterface.GetAllNetworkInterfaces()`, in the order the system lists them.
pub fn list_network_interfaces() -> Vec<NetworkInterfaceInfo> {
    let mut result: Vec<NetworkInterfaceInfo> = Vec::new();
    let mut addrs: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs fills `addrs` with a list that stays valid until freeifaddrs, and each `ifa_addr` is read as
    // sockaddr_in only after its family says that's what it is.
    unsafe {
        if libc::getifaddrs(&mut addrs) != 0 {
            return result;
        }
        let mut cursor = addrs;
        while !cursor.is_null() {
            let ifa = &*cursor;
            cursor = ifa.ifa_next;
            if ifa.ifa_name.is_null() {
                continue;
            }
            let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();
            let flags = ifa.ifa_flags as libc::c_int;
            let position = match result.iter().position(|i| i.id == name) {
                Some(p) => p,
                None => {
                    result.push(NetworkInterfaceInfo {
                        id: name.clone(),
                        is_tunnel: flags & libc::IFF_POINTOPOINT != 0
                            || name.starts_with("utun")
                            || name.starts_with("gif")
                            || name.starts_with("stf"),
                        name,
                        ipv4_addresses: Vec::new(),
                        is_up: flags & libc::IFF_UP != 0 && flags & libc::IFF_RUNNING != 0,
                        supports_multicast: flags & libc::IFF_MULTICAST != 0,
                        is_loopback: flags & libc::IFF_LOOPBACK != 0,
                    });
                    result.len() - 1
                }
            };
            if !ifa.ifa_addr.is_null() && (*ifa.ifa_addr).sa_family as libc::c_int == libc::AF_INET {
                let sin = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                result[position].ipv4_addresses.push(Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)));
            }
        }
        libc::freeifaddrs(addrs);
    }
    result
}

/// The interfaces to offer in a multicast destination list: what the E1.31 setup dialog lists.
pub fn list_multicast_interfaces() -> Vec<NetworkInterfaceInfo> {
    list_network_interfaces().into_iter().filter(NetworkInterfaceInfo::is_offered_for_multicast).collect()
}

/// One entry of the E1.31 destination list's multicast half.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MulticastChoice {
    /// The BSD name, which is what `Multicast` stores.
    pub id: String,
    /// The name people know it by: "Wi-Fi", "Ethernet".
    pub name: String,
    /// `Multicast {name} ({id})`. The id keeps two entries with one name apart.
    pub text: String,
}

impl MulticastChoice {
    fn new(id: &str, name: &str) -> MulticastChoice {
        MulticastChoice { id: id.to_string(), name: name.to_string(), text: format!("Multicast {name} ({id})") }
    }
}

mod system_configuration {
    use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
    use core_foundation_sys::base::{CFIndex, CFRelease, CFTypeRef};
    use core_foundation_sys::dictionary::{CFDictionaryGetValue, CFDictionaryRef};
    use core_foundation_sys::string::{
        kCFStringEncodingUTF8, CFStringCreateWithBytes, CFStringGetCString, CFStringGetLength,
        CFStringGetMaximumSizeForEncoding, CFStringRef,
    };
    use std::ffi::c_void;

    #[link(name = "SystemConfiguration", kind = "framework")]
    extern "C" {
        fn SCNetworkInterfaceCopyAll() -> CFArrayRef;
        fn SCNetworkInterfaceGetBSDName(interface: *const c_void) -> CFStringRef;
        fn SCNetworkInterfaceGetLocalizedDisplayName(interface: *const c_void) -> CFStringRef;
        fn SCDynamicStoreCopyValue(store: *const c_void, key: CFStringRef) -> CFTypeRef;
    }

    fn to_string(text: CFStringRef) -> Option<String> {
        if text.is_null() {
            return None;
        }
        // SAFETY: `text` is a live CFString, and the buffer holds the most the conversion can write.
        unsafe {
            let capacity = CFStringGetMaximumSizeForEncoding(CFStringGetLength(text), kCFStringEncodingUTF8) + 1;
            let mut buffer = vec![0u8; capacity as usize];
            if CFStringGetCString(text, buffer.as_mut_ptr().cast(), capacity, kCFStringEncodingUTF8) == 0 {
                return None;
            }
            let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
            buffer.truncate(end);
            String::from_utf8(buffer).ok()
        }
    }

    /// A CFString for a short time. The caller releases it.
    fn from_str(text: &str) -> CFStringRef {
        // SAFETY: the bytes outlive the call, and a null allocator means the default one.
        unsafe {
            CFStringCreateWithBytes(std::ptr::null(), text.as_ptr(), text.len() as CFIndex, kCFStringEncodingUTF8, 0)
        }
    }

    /// `(BSD name, display name)` of each interface the system's network settings know about, in its order.
    pub(super) fn interfaces() -> Vec<(String, String)> {
        let mut out = Vec::new();
        // SAFETY: the array is released only after the loop, and the Get calls return references it owns.
        unsafe {
            let all = SCNetworkInterfaceCopyAll();
            if all.is_null() {
                return out;
            }
            for i in 0..CFArrayGetCount(all) {
                let interface = CFArrayGetValueAtIndex(all, i);
                if let Some(id) = to_string(SCNetworkInterfaceGetBSDName(interface)) {
                    let name = to_string(SCNetworkInterfaceGetLocalizedDisplayName(interface)).unwrap_or_else(|| id.clone());
                    out.push((id, name));
                }
            }
            CFRelease(all.cast());
        }
        out
    }

    /// `PrimaryInterface` from `State:/Network/Global/IPv4`.
    pub(super) fn primary_interface() -> Option<String> {
        let key = from_str("State:/Network/Global/IPv4");
        let member = from_str("PrimaryInterface");
        // SAFETY: each object made or copied here is released once, and the dictionary's value is read before it goes.
        unsafe {
            let value = SCDynamicStoreCopyValue(std::ptr::null(), key);
            let result = if value.is_null() {
                None
            } else {
                let name = to_string(CFDictionaryGetValue(value as CFDictionaryRef, member.cast()).cast());
                CFRelease(value);
                name
            };
            CFRelease(key.cast());
            CFRelease(member.cast());
            result
        }
    }
}

/// What the E1.31 destination list offers: the system's network interfaces that pass upstream's rule, which keeps out `awdl0`,
/// `llw0`, `anpi*` and the like. `saved` is kept in the list when it names an interface this machine has.
pub fn multicast_choices(saved: Option<&str>) -> Vec<MulticastChoice> {
    let live = list_network_interfaces();
    let mut choices: Vec<MulticastChoice> = system_configuration::interfaces()
        .iter()
        .filter(|(id, _)| live.iter().any(|i| i.id == *id && i.is_offered_for_multicast()))
        .map(|(id, name)| MulticastChoice::new(id, name))
        .collect();
    if let Some(saved) = saved {
        if !choices.iter().any(|c| c.id == saved) && live.iter().any(|i| i.id == saved) {
            choices.push(MulticastChoice::new(saved, saved));
        }
    }
    choices
}

/// The interface the system routes through, which stands in for upstream's default of "Local Area Connection".
pub fn primary_interface_id() -> Option<String> {
    system_configuration::primary_interface()
}

/// Whether a saved interface id can be used on this machine. A saved Windows GUID stays in the data and just
/// reports as `Missing`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterfaceAvailability {
    Available,
    Down,
    NoIpv4Address,
    Missing,
}

pub fn interface_availability(id: &str) -> InterfaceAvailability {
    match list_network_interfaces().into_iter().find(|i| i.id == id && i.is_usable_for_e131()) {
        None => InterfaceAvailability::Missing,
        Some(i) if !i.is_up => InterfaceAvailability::Down,
        Some(i) if i.ipv4_addresses.is_empty() => InterfaceAvailability::NoIpv4Address,
        Some(_) => InterfaceAvailability::Available,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_at_least_loopback_with_its_address() {
        let list = list_network_interfaces();
        let lo = list.iter().find(|i| i.is_loopback).expect("every machine has a loopback interface");
        assert!(lo.ipv4_addresses.contains(&Ipv4Addr::LOCALHOST));
        assert!(!lo.is_usable_for_e131());
        assert!(list_multicast_interfaces().iter().all(|i| !i.is_loopback && !i.is_tunnel));
    }

    #[test]
    fn multicast_choices_come_from_the_system_list_and_keep_a_saved_interface() {
        let choices = multicast_choices(None);
        assert!(choices.iter().all(|c| !c.id.starts_with("awdl") && !c.id.starts_with("llw") && c.id != "lo0"));
        for c in &choices {
            assert_eq!(c.text, format!("Multicast {} ({})", c.name, c.id));
        }
        // Loopback isn't offered, but a file that names it keeps it.
        let lo = multicast_choices(Some("lo0"));
        assert_eq!(lo.last().map(|c| c.id.as_str()), Some("lo0"));
        // A Windows GUID doesn't exist here, so it isn't added.
        assert_eq!(multicast_choices(Some("{B1C2D3E4-0000-0000-0000-000000000000}")), choices);
    }

    #[test]
    fn a_windows_interface_id_is_missing_not_an_error() {
        assert_eq!(interface_availability("{B1C2D3E4-0000-0000-0000-000000000000}"), InterfaceAvailability::Missing);
    }
}
