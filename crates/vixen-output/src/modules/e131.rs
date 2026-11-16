//! Port of Vixen.Modules/Controller/E131: streaming ACN, one UDP packet per universe.

use super::{
    ConfigMismatch, ControllerConfig, ControllerMessage, ControllerNetworkConfiguration, ModuleShared, ModuleStatus, NetworkProtocol,
    OutputModule, TransmissionMethod, UniverseConfiguration,
};
use crate::command::OutputCommand;
use crate::ids;
use crate::net::{list_network_interfaces, NetworkInterfaceInfo};
use crate::xmlutil::{self, bool_el, data_root, num_el, opt_text_el, ModuleIds};
use parking_lot::RwLock;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

const DATA_NS: &str = "VixenModules.Output.E131";
const ENTRY_NS: &str = "VixenModules.Controller.E131";

/// `_updateWarn` and `_missingInterfaceWarning`: these two messages show once per run, across all controllers.
static IMPORT_WARNED: AtomicBool = AtomicBool::new(false);
static MISSING_INTERFACE_WARNED: AtomicBool = AtomicBool::new(false);

/// The ACN port for sACN.
pub const E131_PORT: u16 = 5568;
const SOURCE_NAME: &str = "Vixen 3";
const MULTICAST_TTL: u32 = 64;

/// `UniverseEntry`. `seq_num` and the two event counters are the live values: they're data members, so they get saved.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UniverseEntry {
    pub active: bool,
    pub event_repeat_count: i32,
    pub event_suppress_count: i32,
    /// Deprecated, kept for importing old data.
    pub multicast: Option<String>,
    pub size: i32,
    /// The zero-based first output.
    pub start: i32,
    /// Deprecated.
    pub ttl: i32,
    /// Deprecated, kept for importing old data.
    pub unicast: Option<String>,
    pub universe: i32,
    pub seq_num: u8,
}

impl UniverseEntry {
    /// `new UniverseEntry(rowNum, active, universe, start, size)`.
    pub fn new(active: bool, universe: i32, start: i32, size: i32) -> UniverseEntry {
        UniverseEntry { active, universe, start, size, ttl: 64, ..Default::default() }
    }

    fn from_xml(e: &XElement) -> UniverseEntry {
        UniverseEntry {
            active: xmlutil::get_bool(e, "Active"),
            event_repeat_count: xmlutil::get_i32(e, "EventRepeatCount"),
            event_suppress_count: xmlutil::get_i32(e, "EventSuppressCount"),
            multicast: xmlutil::get_opt_string(e, "Multicast"),
            size: xmlutil::get_i32(e, "Size"),
            start: xmlutil::get_i32(e, "Start"),
            ttl: xmlutil::get_i32(e, "Ttl"),
            unicast: xmlutil::get_opt_string(e, "Unicast"),
            universe: xmlutil::get_i32(e, "Universe"),
            seq_num: xmlutil::get_u8(e, "seqNum"),
        }
    }

    fn to_xml(&self) -> XElement {
        let ns = contract_ns(ENTRY_NS);
        let mut e = XElement::new(&ns, "UniverseEntry");
        e.push(bool_el(&ns, "Active", self.active));
        e.push(num_el(&ns, "EventRepeatCount", self.event_repeat_count));
        e.push(num_el(&ns, "EventSuppressCount", self.event_suppress_count));
        e.push(opt_text_el(&ns, "Multicast", self.multicast.as_deref()));
        e.push(num_el(&ns, "Size", self.size));
        e.push(num_el(&ns, "Start", self.start));
        e.push(num_el(&ns, "Ttl", self.ttl));
        e.push(opt_text_el(&ns, "Unicast", self.unicast.as_deref()));
        e.push(num_el(&ns, "Universe", self.universe));
        e.push(num_el(&ns, "seqNum", self.seq_num));
        e
    }
}

/// `E131ModuleDataModel`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E131Config {
    pub ids: ModuleIds,
    pub auto_populate: bool,
    pub blind: bool,
    pub event_repeat_count: i32,
    pub event_suppress_count: i32,
    /// A network interface id, when multicasting.
    pub multicast: Option<String>,
    pub output_count: i32,
    pub priority: i32,
    pub statistics: bool,
    /// A host name or address, when unicasting. It takes precedence over `multicast`.
    pub unicast: Option<String>,
    /// Null in a file that has none. `LoadSetupNodeInfo` treats that as an empty list.
    pub universes: Option<Vec<UniverseEntry>>,
    pub warnings: bool,
}

impl Default for E131Config {
    /// The C# constructor: one universe, auto-populate on, priority 100.
    fn default() -> Self {
        E131Config {
            ids: ModuleIds::new(ids::E131, Uuid::nil()),
            auto_populate: true,
            blind: false,
            event_repeat_count: 0,
            event_suppress_count: 0,
            multicast: None,
            output_count: 0,
            priority: 100,
            statistics: false,
            unicast: None,
            universes: Some(vec![UniverseEntry::new(true, 1, 1, 510)]),
            warnings: false,
        }
    }
}

impl E131Config {
    pub fn from_xml(e: &XElement) -> E131Config {
        E131Config {
            ids: ModuleIds::read(e),
            auto_populate: xmlutil::get_bool(e, "AutoPopulate"),
            blind: xmlutil::get_bool(e, "Blind"),
            event_repeat_count: xmlutil::get_i32(e, "EventRepeatCount"),
            event_suppress_count: xmlutil::get_i32(e, "EventSuppressCount"),
            multicast: xmlutil::get_opt_string(e, "Multicast"),
            output_count: xmlutil::get_i32(e, "OutputCount"),
            // `[OnDeserializing]` sets 100 first, so a file from before this member existed gets that.
            priority: e.child_text("Priority").and_then(|t| t.parse().ok()).unwrap_or(100),
            statistics: xmlutil::get_bool(e, "Statistics"),
            unicast: xmlutil::get_opt_string(e, "Unicast"),
            universes: e
                .child("Universes")
                .filter(|u| !u.nil)
                .map(|u| u.children_named("UniverseEntry").map(UniverseEntry::from_xml).collect()),
            warnings: xmlutil::get_bool(e, "Warnings"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(DATA_NS);
        let mut e = data_root(&ns, "E131ModuleDataModel", &self.ids);
        e.push(bool_el(&ns, "AutoPopulate", self.auto_populate));
        e.push(bool_el(&ns, "Blind", self.blind));
        e.push(num_el(&ns, "EventRepeatCount", self.event_repeat_count));
        e.push(num_el(&ns, "EventSuppressCount", self.event_suppress_count));
        e.push(opt_text_el(&ns, "Multicast", self.multicast.as_deref()));
        e.push(num_el(&ns, "OutputCount", self.output_count));
        e.push(num_el(&ns, "Priority", self.priority));
        e.push(bool_el(&ns, "Statistics", self.statistics));
        e.push(opt_text_el(&ns, "Unicast", self.unicast.as_deref()));
        match &self.universes {
            None => {
                e.push(XElement::nil(&ns, "Universes"));
            }
            Some(list) => {
                let mut x = XElement::new(&ns, "Universes");
                for u in list {
                    x.push(u.to_xml());
                }
                e.push(x);
            }
        }
        e.push(bool_el(&ns, "Warnings", self.warnings));
        e
    }

    /// `NameOutputs`: `Output #1` by default, and `#3 2-1: Multicast` for the first channel of universe 2, where that's the third
    /// output. Upstream's `NameOutputs` passes the zero-based start to a formula meant for a one-based one, so its first channel
    /// reads 2, unlike what `Setup` writes. This is one function, one-based, for both (F89).
    pub fn output_names(&self, count: usize) -> Vec<String> {
        let mut names: Vec<String> = (0..count).map(|x| format!("Output #{}", x + 1)).collect();
        for u in self.universes.iter().flatten() {
            for x in u.start..u.start.saturating_add(u.size) {
                let Some(name) = usize::try_from(x).ok().and_then(|x| names.get_mut(x)) else {
                    continue;
                };
                let destination = match self.unicast.as_deref() {
                    None | Some("") => "Multicast",
                    Some(host) => host,
                };
                *name = format!("#{} {}-{}: {}", x + 1, u.universe, x - u.start + 1, destination);
            }
        }
        names
    }

    /// `SetupForm_FormClosing`'s checks of the universe list against the controller's output count. Each message is what the
    /// dialog would warn about. An output count of 0 skips the range checks, as it does there.
    pub fn validate(&self, output_count: usize) -> Vec<String> {
        let universes = self.universes.as_deref().unwrap_or(&[]);
        let count = output_count as i64;
        let mut errors = Vec::new();
        if count > 0 {
            let mut total = 0i64;
            for u in universes {
                // The dialog shows one-based start values.
                let start = u.start as i64 + 1;
                let size = u.size as i64;
                if start > count {
                    errors.push(format!("Universe {} start channel exceeds controller output count of {count}", u.universe));
                }
                if start + size - 1 > count {
                    errors.push(format!("Universe {} {start} + {size} exceeds controller output count of {count}", u.universe));
                }
                total += size;
            }
            if total != count {
                errors.push(format!(
                    "Total universe channel count of {total} does not match the controller output count of {count}.\n\nCheck your universe setup or adjust the amount of output channels to match."
                ));
            }
        }
        for (i, a) in universes.iter().enumerate() {
            if universes.iter().skip(i + 1).any(|b| b.universe == a.universe) {
                errors.push("Universe numbers must be unique.".to_string());
                break;
            }
        }
        errors
    }

    /// Whether two controllers reach the same device or network and share a universe number, which the dialog refuses.
    pub fn conflicts_with(&self, other: &E131Config) -> Option<i32> {
        let different_unicast = matches!((&other.unicast, &self.unicast), (Some(a), Some(b)) if a != b);
        let different_multicast = matches!((&other.multicast, &self.multicast), (Some(a), Some(b)) if a != b);
        if different_unicast || different_multicast {
            return None;
        }
        let used: Vec<i32> = other.universes.iter().flatten().map(|u| u.universe).collect();
        self.universes.iter().flatten().map(|u| u.universe).find(|u| used.contains(u))
    }
}

/// `E131Packet` and its three layers. A universe's buffer is built once and then patched in two places.
pub mod packet {
    use crate::dotnet::guid_bytes;
    use uuid::Uuid;

    const ROOT_SIZE: usize = 38;
    const ROOT_PDU_SIZE: usize = ROOT_SIZE - 16;
    const FRAMING_SIZE: usize = 77;
    const DMP_BASE: usize = 10;
    const FRAMING_OFFSET: usize = ROOT_SIZE;
    const DMP_OFFSET: usize = ROOT_SIZE + FRAMING_SIZE;
    /// Where the sequence number sits.
    pub const SEQUENCE_OFFSET: usize = ROOT_SIZE + 73;
    /// Where the priority sits.
    pub const PRIORITY_OFFSET: usize = ROOT_SIZE + 70;
    /// Where the options byte sits.
    pub const OPTIONS_OFFSET: usize = ROOT_SIZE + 74;
    /// Where the universe sits.
    pub const UNIVERSE_OFFSET: usize = ROOT_SIZE + 75;
    /// The first slot: past the headers and the start code.
    pub const SLOTS_OFFSET: usize = DMP_OFFSET + DMP_BASE + 1;

    fn put_u16(b: &mut [u8], at: usize, v: u16) {
        b[at..at + 2].copy_from_slice(&v.to_be_bytes());
    }

    /// `new E131Packet(...).PhyBuffer`. `None` if the slots don't fit the 12-bit PDU lengths, where the C# throws.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        cid: &Uuid,
        source: &str,
        sequence: u8,
        universe: u16,
        values: &[u8],
        offset: usize,
        slots: usize,
        priority: i32,
        blind: bool,
    ) -> Option<Vec<u8>> {
        let dmp_flags = (0x7000 | (DMP_BASE + 1 + slots)) as u16;
        let dmp_len = (dmp_flags & 0x0fff) as usize;
        let framing_flags = (0x7000 | (FRAMING_SIZE + dmp_len)) as u16;
        let framing_len = (framing_flags & 0x0fff) as usize;
        let root_flags = (0x7000 | (ROOT_PDU_SIZE + framing_len)) as u16;
        let phy_len = ROOT_SIZE + framing_len;
        if SLOTS_OFFSET + slots > phy_len {
            return None;
        }
        let mut b = vec![0u8; phy_len];

        // Root layer.
        put_u16(&mut b, 0, 0x0010);
        put_u16(&mut b, 2, 0x0000);
        b[4..13].copy_from_slice(b"ASC-E1.17");
        put_u16(&mut b, 16, root_flags);
        b[18..22].copy_from_slice(&4u32.to_be_bytes());
        b[22..38].copy_from_slice(&guid_bytes(cid));

        // Framing layer.
        put_u16(&mut b, FRAMING_OFFSET, framing_flags);
        b[FRAMING_OFFSET + 2..FRAMING_OFFSET + 6].copy_from_slice(&2u32.to_be_bytes());
        let name = source.as_bytes();
        let n = name.len().min(64);
        b[FRAMING_OFFSET + 6..FRAMING_OFFSET + 6 + n].copy_from_slice(&name[..n]);
        b[PRIORITY_OFFSET] = priority as u8;
        // Reserved stays 0.
        b[SEQUENCE_OFFSET] = sequence;
        b[OPTIONS_OFFSET] = if blind { 128 } else { 0 };
        put_u16(&mut b, UNIVERSE_OFFSET, universe);

        // Device management protocol layer.
        put_u16(&mut b, DMP_OFFSET, dmp_flags);
        b[DMP_OFFSET + 2] = 0x02;
        b[DMP_OFFSET + 3] = 0xa1;
        put_u16(&mut b, DMP_OFFSET + 4, 0x0000);
        put_u16(&mut b, DMP_OFFSET + 6, 0x0001);
        put_u16(&mut b, DMP_OFFSET + 8, (slots + 1) as u16);
        // Start code 0, then the slots.
        b[SLOTS_OFFSET..SLOTS_OFFSET + slots].copy_from_slice(values.get(offset..offset + slots)?);
        Some(b)
    }

    /// `E131Packet.CompareSlots`.
    #[inline]
    pub fn compare_slots(phy: &[u8], values: &[u8], offset: usize, slots: usize) -> bool {
        phy[SLOTS_OFFSET..SLOTS_OFFSET + slots] == values[offset..offset + slots]
    }

    /// `E131Packet.CopySeqNumSlots`.
    #[inline]
    pub fn copy_seq_num_slots(phy: &mut [u8], values: &[u8], offset: usize, slots: usize, sequence: u8) {
        if !values.is_empty() {
            phy[SLOTS_OFFSET..SLOTS_OFFSET + slots].copy_from_slice(&values[offset..offset + slots]);
            phy[SEQUENCE_OFFSET] = sequence;
        }
    }
}

/// Suppresses repeats of the last frame sent: the first `EventRepeatCount` go out, then one in every `EventSuppressCount` while nothing changes.
fn send_this_frame(universe: &mut UniverseEntry, same_as_last: bool, repeat: i32, suppress: i32) -> bool {
    if !same_as_last {
        universe.event_repeat_count = 0;
        universe.event_suppress_count = 0;
        return true;
    }
    universe.event_repeat_count = universe.event_repeat_count.wrapping_add(1);
    if universe.event_repeat_count < repeat {
        return true;
    }
    if suppress > 0 {
        universe.event_suppress_count = universe.event_suppress_count.wrapping_add(1);
        if universe.event_suppress_count >= suppress {
            universe.event_suppress_count = 0;
            return true;
        }
    }
    false
}

/// The multicast group for a universe: 239.255.high.low.
pub fn multicast_address(universe: i32) -> Ipv4Addr {
    Ipv4Addr::new(239, 255, (universe >> 8) as u8, (universe & 0xff) as u8)
}

/// One universe's counters, as `UniverseEntry.StatsToText` shows them.
#[derive(Debug, Default)]
struct UniverseCounters {
    universe: i32,
    active: bool,
    packets: AtomicU64,
    slots: AtomicU64,
}

/// The statistics the plugin shows when it stops, readable from any thread while it runs.
#[derive(Debug, Default)]
pub struct E131Counters {
    events: AtomicU64,
    total_ticks: AtomicU64,
    update_nanos: AtomicU64,
    universes: RwLock<Vec<UniverseCounters>>,
}

/// A copy of [`E131Counters`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct E131Stats {
    /// `_eventCnt`: frames handed to the module.
    pub events: u64,
    /// `_totalTicks`, in 100 ns ticks. The C# adds its stopwatch, which it never resets, after each update, so this is the
    /// running total of a running total, and only as meaningful there as here.
    pub total_ticks: u64,
    /// The time really spent in `update`.
    pub update_nanos: u64,
    pub universes: Vec<E131UniverseStats>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct E131UniverseStats {
    pub universe: i32,
    pub active: bool,
    /// `PktCount`.
    pub packets: u64,
    /// `SlotCount`: slots sent, counted at the universe's full size.
    pub slots: u64,
}

impl E131Counters {
    pub fn snapshot(&self) -> E131Stats {
        E131Stats {
            events: self.events.load(Ordering::Relaxed),
            total_ticks: self.total_ticks.load(Ordering::Relaxed),
            update_nanos: self.update_nanos.load(Ordering::Relaxed),
            universes: self
                .universes
                .read()
                .iter()
                .map(|u| E131UniverseStats {
                    universe: u.universe,
                    active: u.active,
                    packets: u.packets.load(Ordering::Relaxed),
                    slots: u.slots.load(Ordering::Relaxed),
                })
                .collect(),
        }
    }
}

impl E131Stats {
    /// The text of the statistics box `Stop` shows.
    pub fn to_text(&self) -> String {
        // Upstream shows only the milliseconds component. This is the total (F92).
        let millis = self.total_ticks / 10_000;
        let mut text = format!("Events: {}\nTotal Time: {} Ticks; {} ms\n", self.events, self.total_ticks, millis);
        for (row, u) in self.universes.iter().enumerate().filter(|(_, u)| u.active) {
            text.push_str(&format!("\nRow {}: Univ={}  Packets={}  Slots={}", row + 1, u.universe, u.packets, u.slots));
        }
        text
    }
}

/// What a `UniverseEntry` holds at run time and doesn't save.
#[derive(Default)]
struct UniverseRuntime {
    /// Index into `sockets`.
    socket: Option<usize>,
    dest: Option<SockAddr>,
    /// `PhyBuffer`, empty when null.
    phy: Vec<u8>,
}

/// `E131OutputPlugin`.
pub struct E131Module {
    config: E131Config,
    shared: Arc<ModuleShared>,
    counters: Arc<E131Counters>,
    output_count: usize,
    port: u16,
    running: bool,
    runtime: Vec<UniverseRuntime>,
    sockets: Vec<Socket>,
    channel_values: Vec<u8>,
    /// `_messageTexts`: what `Start` found, for the warnings box or the statistics box.
    message_texts: String,
}

impl E131Module {
    pub fn new(config: E131Config) -> E131Module {
        E131Module::with_port(config, E131_PORT)
    }

    /// A module that sends to another UDP port. Vixen only ever uses 5568.
    pub fn with_port(config: E131Config, port: u16) -> E131Module {
        let mut m = E131Module {
            config,
            shared: ModuleShared::new(),
            counters: Arc::new(E131Counters::default()),
            output_count: 0,
            port,
            running: false,
            runtime: Vec::new(),
            sockets: Vec::new(),
            channel_values: Vec::new(),
            message_texts: String::new(),
        };
        m.reset_universe_state();
        m
    }

    /// The statistics counters, readable from any thread.
    pub fn counters(&self) -> Arc<E131Counters> {
        self.counters.clone()
    }

    fn reset_universe_state(&mut self) {
        let universes = self.config.universes.as_deref().unwrap_or(&[]);
        self.runtime = universes.iter().map(|_| UniverseRuntime::default()).collect();
        *self.counters.universes.write() =
            universes.iter().map(|u| UniverseCounters { universe: u.universe, active: u.active, ..Default::default() }).collect();
    }

    /// `GetNetworkConfiguration`, for the FPP export.
    pub fn network_configuration(&self) -> ControllerNetworkConfiguration {
        let multicast = self.config.multicast.is_some();
        let ip_address = if multicast {
            None
        } else {
            let host = self.config.unicast.as_deref().unwrap_or("");
            Some(last_ipv4(host, E131_PORT).map_or(IpAddr::V4(Ipv4Addr::LOCALHOST), IpAddr::V4))
        };
        ControllerNetworkConfiguration {
            protocol: NetworkProtocol::E131,
            transmission: if multicast { TransmissionMethod::Multicast } else { TransmissionMethod::Unicast },
            ip_address,
            supports_universes: true,
            throttling_enabled: self.config.event_repeat_count > 0,
            universes: self
                .config
                .universes
                .iter()
                .flatten()
                .map(|u| UniverseConfiguration {
                    universe_number: u.universe,
                    active: u.active,
                    start: u.start + 1,
                    size: u.size,
                })
                .collect(),
        }
    }

    /// Sets up one universe's socket and destination in unicast mode. `Err` means `cleanStart = false`.
    fn start_unicast_universe(&mut self, index: usize, host: &str, unicast_socket: &mut Option<usize>) -> Result<(), String> {
        let socket = match *unicast_socket {
            Some(s) => s,
            None => {
                let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
                    .map_err(|e| format!("Couldn't create a socket: {e}"))?;
                self.sockets.push(s);
                *unicast_socket = Some(self.sockets.len() - 1);
                self.sockets.len() - 1
            }
        };
        self.runtime[index].socket = Some(socket);
        match last_ipv4(host, self.port) {
            Some(ip) => {
                self.runtime[index].dest = Some(SockAddr::from(SocketAddrV4::new(ip, self.port)));
                Ok(())
            }
            None => {
                self.runtime[index].socket = None;
                Err(format!("Couldn't connect to host {host}."))
            }
        }
    }

    /// Sets up one universe's socket and destination in multicast mode. `Err` means `cleanStart = false`.
    fn start_multicast_universe(
        &mut self,
        index: usize,
        nic_id: &str,
        universe: i32,
        nics: &[NetworkInterfaceInfo],
        nic_socket: &mut Option<usize>,
    ) -> Result<(), String> {
        let Some(nic) = nics.iter().find(|n| n.id == nic_id && n.is_usable_for_e131()) else {
            if !MISSING_INTERFACE_WARNED.swap(true, Ordering::Relaxed) {
                self.shared.post_message(ControllerMessage {
                    title: "Vixen 3 Streaming ACN (E1.31) plugin".to_string(),
                    heading: None,
                    text: "The Streaming ACN (E1.31) plugin could not find one or more of the multicast interfaces specified. Please verify your network and plugin configuration.".to_string(),
                });
            }
            return Err(format!("Couldn't connect to use nic {nic_id} for multicasting."));
        };
        let dest = SockAddr::from(SocketAddrV4::new(multicast_address(universe), self.port));
        if let Some(socket) = *nic_socket {
            self.runtime[index].socket = Some(socket);
            self.runtime[index].dest = Some(dest);
            return Ok(());
        }
        if !nic.is_up {
            return Err(format!("Nic {nic_id} is available for multicasting bur currently down."));
        }
        let socket =
            Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).map_err(|e| format!("Couldn't create a socket: {e}"))?;
        // The C# leaves the last IPv4 address it found, and with none it carries on without a destination.
        let Some(ip) = nic.ipv4_addresses.last().copied() else {
            let info = {
                let u = &self.config.universes.as_deref().unwrap_or(&[])[index];
                format!(
                    "Row {}: Univ={} Start={} Size={}{}{} TTL={}",
                    index + 1,
                    u.universe,
                    u.start + 1,
                    u.size,
                    if u.unicast.is_some() { " Unicast" } else { "" },
                    if u.multicast.is_some() { " Multicast" } else { "" },
                    u.ttl
                )
            };
            // F91: upstream calls this a clean start and throws on the first send. Here the universe sends nothing and start isn't clean.
            let message = format!("No IP On Multicast Interface: {} - {info}", nic.name);
            self.message_texts.push_str(&message);
            self.message_texts.push('\n');
            return Err(message);
        };
        socket.set_multicast_if_v4(&ip).map_err(|e| format!("Couldn't use nic {nic_id} for multicasting: {e}"))?;
        socket.set_multicast_ttl_v4(MULTICAST_TTL).map_err(|e| format!("Couldn't use nic {nic_id} for multicasting: {e}"))?;
        self.sockets.push(socket);
        let s = self.sockets.len() - 1;
        self.runtime[index].socket = Some(s);
        self.runtime[index].dest = Some(dest);
        *nic_socket = Some(s);
        Ok(())
    }
}

/// `Dns.GetHostAddresses`, keeping the last IPv4 address as the C# loop does.
fn last_ipv4(host: &str, port: u16) -> Option<Ipv4Addr> {
    let addrs = (host, port).to_socket_addrs().ok()?;
    let mut last = None;
    for a in addrs {
        if let IpAddr::V4(ip) = a.ip() {
            last = Some(ip);
        }
    }
    last
}

impl OutputModule for E131Module {
    fn type_id(&self) -> Uuid {
        ids::E131
    }

    fn start(&mut self) {
        if self.config.universes.is_none() {
            self.config.universes = Some(Vec::new());
            self.reset_universe_state();
        }
        self.counters.events.store(0, Ordering::Relaxed);
        self.counters.total_ticks.store(0, Ordering::Relaxed);
        self.counters.update_nanos.store(0, Ordering::Relaxed);
        self.message_texts.clear();

        // Data from an old version kept its destination on the first universe.
        if self.config.unicast.is_none() && self.config.multicast.is_none() {
            if let Some(first) = self.config.universes.as_deref().and_then(<[UniverseEntry]>::first) {
                if first.multicast.is_some() || first.unicast.is_some() {
                    let (unicast, multicast) = (first.unicast.clone(), first.multicast.clone());
                    self.config.unicast = unicast;
                    self.config.multicast = multicast;
                    if !IMPORT_WARNED.swap(true, Ordering::Relaxed) {
                        self.shared.post_message(ControllerMessage {
                            title: "Vixen 3 Streaming ACN (E1.31) plugin".to_string(),
                            heading: None,
                            text: "The E1.31 plugin is importing data from an older version of the plugin. Please verify the new Streaming ACN (E1.31) configuration.".to_string(),
                        });
                    }
                }
            }
        }

        self.sockets.clear();
        for r in &mut self.runtime {
            r.socket = None;
            r.dest = None;
        }
        let nics =
            if self.config.unicast.is_none() && self.config.multicast.is_some() { list_network_interfaces() } else { Vec::new() };
        let unicast = self.config.unicast.clone();
        let multicast = self.config.multicast.clone();
        let cid = self.config.ids.instance_id;
        let (priority, blind) = (self.config.priority, self.config.blind);
        let universes: Vec<UniverseEntry> = self.config.universes.clone().unwrap_or_default();

        let mut clean_start = true;
        let mut failure = None;
        let mut unicast_socket = None;
        let mut nic_socket = None;
        for (i, u) in universes.iter().enumerate() {
            if clean_start && u.active {
                let result = if let Some(host) = unicast.as_deref() {
                    self.start_unicast_universe(i, host, &mut unicast_socket)
                } else if let Some(nic) = multicast.as_deref() {
                    self.start_multicast_universe(i, nic, u.universe, &nics, &mut nic_socket)
                } else {
                    Err("E1.31 plugin failed to start due to unassigned destinations. This can happen with newly created plugin instances that have yet to be configured.".to_string())
                };
                if let Err(message) = result {
                    self.shared.record_error(message.clone());
                    failure = Some(message);
                    clean_start = false;
                }
                // Still clean: the universe gets its empty packet.
                if clean_start {
                    let zero = vec![0u8; u.size.max(0) as usize];
                    match packet::build(&cid, SOURCE_NAME, 0, u.universe as u16, &zero, 0, zero.len(), priority, blind) {
                        Some(phy) => self.runtime[i].phy = phy,
                        None => {
                            let message = format!("Universe {} has more slots than a packet can hold", u.universe);
                            self.shared.record_error(message.clone());
                            failure = Some(message);
                            clean_start = false;
                        }
                    }
                }
            }
            // Set inside the loop, so a universe that fails later doesn't stop the ones before it.
            if clean_start {
                self.running = true;
            }
        }

        self.shared.set_status(match failure {
            Some(message) => ModuleStatus::Unavailable(message),
            None if self.running => ModuleStatus::Running,
            None => ModuleStatus::Unavailable("No universes are configured".to_string()),
        });

        // F91: with warnings on, what `Start` found shows now, and doesn't show again when stopping.
        if self.config.warnings && !self.message_texts.is_empty() {
            self.shared.post_message(ControllerMessage {
                title: "Startup Warnings/Errors".to_string(),
                heading: Some("The following warnings and errors were detected during startup:".to_string()),
                text: std::mem::take(&mut self.message_texts),
            });
        }
    }

    fn stop(&mut self) {
        // F92.
        if self.config.statistics {
            let mut text = std::mem::take(&mut self.message_texts);
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&self.counters.snapshot().to_text());
            self.shared.post_message(ControllerMessage {
                title: "J1Sys E1.31 Vixen Plugin".to_string(),
                heading: Some("Plugin Statistics:".to_string()),
                text,
            });
        }
        self.running = false;
        self.sockets.clear();
        for r in &mut self.runtime {
            r.socket = None;
            r.dest = None;
        }
        self.shared.set_status(ModuleStatus::Stopped);
    }

    fn update(&mut self, commands: &[OutputCommand]) {
        if !self.running {
            return;
        }
        let started = Instant::now();
        self.send_frame(commands);
        // Upstream's stopwatch is never reset, so each update adds everything so far. This measures each one (F92).
        let elapsed = started.elapsed().as_nanos() as u64;
        self.counters.update_nanos.fetch_add(elapsed, Ordering::Relaxed);
        self.counters.total_ticks.fetch_add(elapsed / 100, Ordering::Relaxed);
    }

    fn output_count(&self) -> usize {
        self.output_count
    }

    fn set_output_count(&mut self, count: usize) {
        self.output_count = count;
    }

    fn config(&self) -> ControllerConfig {
        ControllerConfig::E131(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::E131(config) = config else {
            return Err(ConfigMismatch);
        };
        self.config = config;
        self.running = false;
        self.sockets.clear();
        self.reset_universe_state();
        Ok(())
    }

    fn shared(&self) -> &Arc<ModuleShared> {
        &self.shared
    }

    fn e131_counters(&self) -> Option<Arc<E131Counters>> {
        Some(self.counters.clone())
    }
}

impl E131Module {
    /// The body of `UpdateState`.
    fn send_frame(&mut self, commands: &[OutputCommand]) {
        let Some(universes) = self.config.universes.as_mut() else {
            return;
        };
        if universes.is_empty() {
            return;
        }
        if self.channel_values.len() != commands.len() {
            self.channel_values.resize(commands.len(), 0);
        }
        for (slot, c) in self.channel_values.iter_mut().zip(commands) {
            *slot = c.byte();
        }
        self.counters.events.fetch_add(1, Ordering::Relaxed);
        let counters = self.counters.universes.read();
        let output_count = self.output_count as i64;
        let (event_repeat, event_suppress) = (self.config.event_repeat_count, self.config.event_suppress_count);

        for (i, u) in universes.iter_mut().enumerate() {
            let runtime = &mut self.runtime[i];
            if runtime.phy.is_empty() {
                continue;
            }
            let start = u.start as i64;
            if !u.active || start >= output_count {
                continue;
            }
            // A start before the first output throws while comparing, and the rest of the frame is lost.
            if start < 0 {
                self.shared
                    .record_error(format!("An error occurred outputting data: universe {} has a negative start", u.universe));
                return;
            }
            let start = start as usize;
            let size = u.size.max(0) as i64;
            let universe_size =
                if u.start as i64 + size > output_count { (output_count - u.start as i64) as usize } else { size as usize };
            if start + universe_size > self.channel_values.len() {
                self.shared.record_error(format!(
                    "An error occurred outputting data: universe {} is past the end of the frame",
                    u.universe
                ));
                return;
            }

            let issame = event_repeat > 0 && packet::compare_slots(&runtime.phy, &self.channel_values, start, universe_size);
            if send_this_frame(u, issame, event_repeat, event_suppress) {
                packet::copy_seq_num_slots(&mut runtime.phy, &self.channel_values, start, universe_size, u.seq_num);
                u.seq_num = u.seq_num.wrapping_add(1);
                let sent = match (runtime.socket.and_then(|s| self.sockets.get(s)), runtime.dest.as_ref()) {
                    (Some(socket), Some(dest)) => socket.send_to(&runtime.phy, dest),
                    _ => Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "the universe has no socket or destination")),
                };
                match sent {
                    Ok(_) => {
                        self.shared.count_packet(runtime.phy.len());
                        if let Some(c) = counters.get(i) {
                            c.packets.fetch_add(1, Ordering::Relaxed);
                            c.slots.fetch_add(size as u64, Ordering::Relaxed);
                        }
                    }
                    // `OutputController.Update` catches this, so the remaining universes wait for the next frame.
                    Err(e) => {
                        self.shared.record_error(format!("An error occurred outputting data: {e}"));
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(active: bool, universe: i32, start: i32, size: i32) -> UniverseEntry {
        UniverseEntry::new(active, universe, start, size)
    }

    #[test]
    fn a_packet_of_510_slots_has_every_field_in_its_place() {
        let values = vec![9u8; 510];
        let cid = Uuid::new_v4();
        let b = packet::build(&cid, "Vixen 3", 7, 300, &values, 0, 510, 100, true).unwrap();
        let u16_at = |at: usize| u16::from_be_bytes([b[at], b[at + 1]]);
        assert_eq!(b.len(), 636);
        assert_eq!(u16_at(0), 0x0010);
        assert_eq!(&b[4..13], b"ASC-E1.17");
        assert_eq!(u16_at(16), 0x7000 | 620);
        assert_eq!(&b[18..22], &[0, 0, 0, 4]);
        assert_eq!(u16_at(38), 0x7000 | 598);
        assert_eq!(&b[40..44], &[0, 0, 0, 2]);
        assert_eq!(&b[44..51], b"Vixen 3");
        assert_eq!(b[108], 100);
        assert_eq!(b[111], 7);
        assert_eq!(b[112], 128);
        assert_eq!(u16_at(113), 300);
        assert_eq!(u16_at(115), 0x7000 | 521);
        assert_eq!(b[117], 0x02);
        assert_eq!(u16_at(123), 511);
        assert_eq!(b[125], 0);
        assert!(b[126..].iter().all(|v| *v == 9));
    }

    fn sent_frames(repeat: i32, suppress: i32, same: impl Fn(u32) -> bool) -> Vec<u32> {
        let mut u = entry(true, 1, 0, 10);
        (1..=10).filter(|frame| send_this_frame(&mut u, same(*frame), repeat, suppress)).collect()
    }

    #[test]
    fn repeats_are_suppressed_as_the_setup_dialog_says() {
        assert_eq!(sent_frames(2, 3, |_| true), [1, 4, 7, 10]);
        // With no suppress count, nothing goes after the repeats until something changes.
        assert_eq!(sent_frames(2, 0, |_| true), [1]);
        // A changed frame resets both counters.
        assert_eq!(sent_frames(2, 3, |frame| frame != 5), [1, 4, 5, 6, 9]);
    }

    #[test]
    fn multicast_addresses_follow_the_universe() {
        assert_eq!(multicast_address(1), Ipv4Addr::new(239, 255, 0, 1));
        assert_eq!(multicast_address(258), Ipv4Addr::new(239, 255, 1, 2));
        assert_eq!(multicast_address(63999), Ipv4Addr::new(239, 255, 249, 255));
    }

    #[test]
    fn a_new_config_matches_the_constructor() {
        let c = E131Config::default();
        assert!(c.auto_populate);
        assert_eq!(c.priority, 100);
        assert_eq!(
            c.universes,
            Some(vec![UniverseEntry { active: true, universe: 1, start: 1, size: 510, ttl: 64, ..Default::default() }])
        );
    }

    #[test]
    fn a_missing_priority_reads_as_100() {
        let e = XElement::parse(r#"<E131ModuleDataModel xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Output.E131"><Blind>true</Blind></E131ModuleDataModel>"#).unwrap();
        let c = E131Config::from_xml(&e);
        assert_eq!(c.priority, 100);
        assert!(c.blind);
        // The constructor doesn't run when deserializing.
        assert!(!c.auto_populate);
        assert_eq!(c.universes, None);
    }

    #[test]
    fn output_names_match_name_outputs() {
        let mut c = E131Config { universes: Some(vec![entry(true, 5, 1, 2)]), ..Default::default() };
        assert_eq!(c.output_names(4), vec!["Output #1", "#2 5-1: Multicast", "#3 5-2: Multicast", "Output #4"]);
        c.unicast = Some("10.0.0.9".to_string());
        assert_eq!(c.output_names(2)[1], "#2 5-1: 10.0.0.9");
    }

    #[test]
    fn validation_matches_the_setup_dialog() {
        let c = E131Config { universes: Some(vec![entry(true, 1, 0, 512), entry(true, 2, 512, 512)]), ..Default::default() };
        assert!(c.validate(1024).is_empty());
        let messages = c.validate(600);
        assert!(messages.iter().any(|m| m.contains("Universe 2 513 + 512 exceeds")));
        assert!(messages.iter().any(|m| m.contains("Total universe channel count of 1024")));
        // No range checks until the controller has outputs.
        assert!(c.validate(0).is_empty());
        let dup = E131Config { universes: Some(vec![entry(true, 1, 0, 10), entry(true, 1, 10, 10)]), ..Default::default() };
        assert!(dup.validate(20).iter().any(|m| m == "Universe numbers must be unique."));
    }

    #[test]
    fn controllers_conflict_only_on_the_same_destination() {
        let a = E131Config {
            unicast: Some("10.0.0.1".to_string()),
            universes: Some(vec![entry(true, 1, 0, 10)]),
            ..Default::default()
        };
        let mut b = E131Config {
            unicast: Some("10.0.0.2".to_string()),
            universes: Some(vec![entry(true, 1, 0, 10)]),
            ..Default::default()
        };
        assert_eq!(a.conflicts_with(&b), None);
        b.unicast = a.unicast.clone();
        assert_eq!(a.conflicts_with(&b), Some(1));
        b.universes = Some(vec![entry(true, 2, 0, 10)]);
        assert_eq!(a.conflicts_with(&b), None);
    }

    #[test]
    fn network_configuration_reports_the_destination_and_one_based_starts() {
        let m = E131Module::new(E131Config {
            unicast: Some("127.0.0.1".to_string()),
            event_repeat_count: 3,
            universes: Some(vec![entry(true, 7, 0, 100)]),
            ..Default::default()
        });
        let n = m.network_configuration();
        assert_eq!(n.ip_address, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert_eq!(n.transmission, TransmissionMethod::Unicast);
        assert!(n.throttling_enabled);
        assert_eq!(n.universes, vec![UniverseConfiguration { universe_number: 7, active: true, start: 1, size: 100 }]);
    }

    #[test]
    fn a_missing_network_interface_reports_unavailable_without_sending() {
        // A Windows interface GUID, as a file saved on Windows has. Nothing is ever sent.
        let mut m = E131Module::new(E131Config {
            multicast: Some("{B1C2D3E4-0000-0000-0000-000000000000}".to_string()),
            universes: Some(vec![entry(true, 1, 0, 4)]),
            ..Default::default()
        });
        m.set_output_count(4);
        m.start();
        assert!(matches!(m.status(), ModuleStatus::Unavailable(s) if s.contains("B1C2D3E4")));
        m.update(&vec![OutputCommand::U8(1); 4]);
        assert_eq!(m.shared().packets_sent(), 0);
        let ControllerConfig::E131(saved) = m.config() else { panic!() };
        assert_eq!(saved.multicast.as_deref(), Some("{B1C2D3E4-0000-0000-0000-000000000000}"));
    }

    #[test]
    fn no_destination_reports_unavailable() {
        let mut m = E131Module::new(E131Config { universes: Some(vec![entry(true, 1, 0, 4)]), ..Default::default() });
        m.start();
        assert!(matches!(m.status(), ModuleStatus::Unavailable(s) if s.contains("unassigned destinations")));
    }

    #[test]
    fn old_data_keeps_its_destination_on_the_first_universe() {
        let mut first = entry(true, 1, 0, 4);
        first.unicast = Some("127.0.0.1".to_string());
        let mut m = E131Module::with_port(E131Config { universes: Some(vec![first]), ..Default::default() }, 1);
        m.start();
        let ControllerConfig::E131(saved) = m.config() else { panic!() };
        assert_eq!(saved.unicast.as_deref(), Some("127.0.0.1"));
        assert_eq!(m.status(), ModuleStatus::Running);
        m.stop();
    }

    #[test]
    fn stats_text_matches_the_form() {
        let s = E131Stats {
            events: 5,
            total_ticks: 25_000,
            update_nanos: 1,
            universes: vec![
                E131UniverseStats { universe: 1, active: true, packets: 4, slots: 2048 },
                E131UniverseStats { universe: 2, active: false, packets: 0, slots: 0 },
            ],
        };
        assert_eq!(s.to_text(), "Events: 5\nTotal Time: 25000 Ticks; 2 ms\n\nRow 1: Univ=1  Packets=4  Slots=2048");
    }
}
