//! Controller lists and the Falcon Player configuration files: `Export.WriteUniverseFile` (1.x),
//! `Export.Write2xUniverseFile` (2.x JSON) and `Export.WriteControllerInfo`.

use crate::{file_name_of, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ProtocolType {
    Sacn,
    Ddp,
    Other,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TransmissionMethod {
    Multicast,
    Unicast,
}

/// `UniverseConfiguration`: one universe of a network controller.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UniverseConfig {
    pub active: bool,
    pub universe_number: i32,
    pub size: i32,
}

/// `ControllerNetworkConfiguration`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NetworkConfig {
    pub protocol: ProtocolType,
    pub transmission: TransmissionMethod,
    /// Empty when the controller has no address.
    pub ip_address: String,
    pub throttling_enabled: bool,
    pub universes: Vec<UniverseConfig>,
}

/// `Export.Controller`: a controller's place in an export.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ControllerInfo {
    pub id: Uuid,
    pub name: String,
    pub index: i32,
    pub is_active: bool,
    pub channels: i32,
    /// `Some` when the module `SupportsNetwork` and its configuration `SupportsUniverses`.
    pub network: Option<NetworkConfig>,
}

impl ControllerInfo {
    pub fn has_network_support(&self) -> bool {
        self.network.is_some()
    }
}

/// `AllSelectedControllersSupportUniverses`.
pub fn all_selected_support_universes(controllers: &[ControllerInfo]) -> bool {
    controllers.iter().filter(|c| c.is_active).all(ControllerInfo::has_network_support)
}

/// Active controllers in index order, as every export path walks them. The sort is stable, like `OrderBy`.
pub fn active_in_order(controllers: &[ControllerInfo]) -> Vec<&ControllerInfo> {
    let mut active: Vec<&ControllerInfo> = controllers.iter().filter(|c| c.is_active).collect();
    active.sort_by_key(|c| c.index);
    active
}

/// The list's Start and End for each controller, 1-based, in `Index` order. An inactive one has none and doesn't advance the count.
pub fn channel_ranges(controllers: &[ControllerInfo]) -> Vec<Option<(i32, i32)>> {
    let mut order: Vec<&ControllerInfo> = controllers.iter().collect();
    order.sort_by_key(|c| c.index);
    let mut next = 1;
    order
        .into_iter()
        .map(|c| {
            c.is_active.then(|| {
                let range = (next, next + c.channels - 1);
                next += c.channels;
                range
            })
        })
        .collect()
}

/// `ReIndexControllerChannels`: each controller in `order` takes its row number as its index, and the tick in `active`. A controller
/// that isn't listed keeps what it has.
pub fn reindex(controllers: &mut [ControllerInfo], order: &[Uuid], active: &[bool]) {
    for (row, id) in order.iter().enumerate() {
        if let Some(controller) = controllers.iter_mut().find(|c| c.id == *id) {
            controller.index = row as i32;
            if let Some(tick) = active.get(row) {
                controller.is_active = *tick;
            }
        }
    }
}

/// `UniverseTypes`.
const E131_MULTICAST: i32 = 0;
const E131_UNICAST: i32 = 1;
const DDP_1_BASED: i32 = 5;

/// A JSON value with its members in the order they were added, which is the order Newtonsoft writes a class's properties in.
enum Ordered {
    Object(Vec<(&'static str, Ordered)>),
    Array(Vec<Ordered>),
    Int(i64),
    Text(String),
}

fn flag(b: bool) -> Ordered {
    Ordered::Int(i64::from(b))
}

impl Ordered {
    /// `Formatting.Indented`: two spaces, `"name": value`, an empty array as `[]`, and the platform's line ending, which on the Windows
    /// machines these files came from is CR LF.
    fn write(&self, out: &mut String, depth: usize) {
        let indent = |out: &mut String, depth: usize| {
            out.push_str(crate::NEWLINE);
            for _ in 0..depth {
                out.push_str("  ");
            }
        };
        match self {
            Ordered::Int(n) => out.push_str(&n.to_string()),
            Ordered::Text(t) => out.push_str(&Value::String(t.clone()).to_string()),
            Ordered::Array(items) if items.is_empty() => out.push_str("[]"),
            Ordered::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    indent(out, depth + 1);
                    item.write(out, depth + 1);
                }
                indent(out, depth);
                out.push(']');
            }
            Ordered::Object(members) if members.is_empty() => out.push_str("{}"),
            Ordered::Object(members) => {
                out.push('{');
                for (i, (name, value)) in members.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    indent(out, depth + 1);
                    out.push_str(&Value::String((*name).to_string()).to_string());
                    out.push_str(": ");
                    value.write(out, depth + 1);
                }
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

struct UniverseOut {
    active: bool,
    description: String,
    id: i32,
    start_channel: i32,
    universe_count: i32,
    channel_count: i32,
    universe_type: i32,
    address: String,
    monitor: bool,
    de_duplicate: bool,
}

impl UniverseOut {
    /// Newtonsoft's camelCase output, in `Universe`'s property order. `UniverseType` is an enum property serialized by number, then
    /// `Type` repeats it; `Priority` is never set.
    fn to_json(&self) -> Ordered {
        Ordered::Object(vec![
            ("active", flag(self.active)),
            ("description", Ordered::Text(self.description.clone())),
            ("id", Ordered::Int(self.id.into())),
            ("startChannel", Ordered::Int(self.start_channel.into())),
            ("universeCount", Ordered::Int(self.universe_count.into())),
            ("channelCount", Ordered::Int(self.channel_count.into())),
            ("universeType", Ordered::Int(self.universe_type.into())),
            ("type", Ordered::Int(self.universe_type.into())),
            ("address", Ordered::Text(self.address.clone())),
            ("priority", Ordered::Int(0)),
            ("monitor", flag(self.monitor)),
            ("deDuplicate", flag(self.de_duplicate)),
        ])
    }
}

/// `Write2xUniverseFile`'s output, member for member in upstream's order.
pub fn universe_config_2x(controllers: &[ControllerInfo]) -> String {
    let mut fpp_start_channel = 1;
    let mut universes: Vec<UniverseOut> = Vec::new();
    for controller in active_in_order(controllers) {
        let Some(network) = &controller.network else {
            fpp_start_channel += controller.channels;
            continue;
        };
        match network.protocol {
            ProtocolType::Sacn => {
                let is_multicast = network.transmission == TransmissionMethod::Multicast;
                for (index, universe) in network.universes.iter().enumerate() {
                    let merges = index > 0
                        && universes.last().is_some_and(|last| {
                            // Universes are consecutive.
                            universe.size == last.channel_count && last.id + last.universe_count == universe.universe_number
                        });
                    if merges {
                        universes.last_mut().unwrap().universe_count += 1;
                    } else {
                        universes.push(UniverseOut {
                            active: universe.active,
                            description: controller.name.clone(),
                            id: universe.universe_number,
                            start_channel: fpp_start_channel,
                            universe_count: 1,
                            channel_count: universe.size,
                            universe_type: if is_multicast { E131_MULTICAST } else { E131_UNICAST },
                            address: network.ip_address.clone(),
                            monitor: !is_multicast,
                            de_duplicate: network.throttling_enabled,
                        });
                    }
                    fpp_start_channel += universe.size;
                }
            }
            ProtocolType::Ddp => {
                let size = network.universes.first().map_or(0, |u| u.size);
                universes.push(UniverseOut {
                    active: true,
                    description: controller.name.clone(),
                    id: 0,
                    start_channel: fpp_start_channel,
                    universe_count: 1,
                    channel_count: size,
                    universe_type: DDP_1_BASED,
                    address: network.ip_address.clone(),
                    monitor: true,
                    de_duplicate: true,
                });
                fpp_start_channel += size;
            }
            ProtocolType::Other => {}
        }
    }

    let output = Ordered::Object(vec![
        ("type", Ordered::Text("universes".to_string())),
        ("enabled", Ordered::Int(1)),
        ("startChannel", Ordered::Int(1)),
        ("channelCount", Ordered::Int(-1)),
        ("universes", Ordered::Array(universes.iter().map(UniverseOut::to_json).collect())),
    ]);
    let root = Ordered::Object(vec![("channelOutputs", Ordered::Array(vec![output]))]);
    let mut text = String::new();
    root.write(&mut text, 0);
    text
}

/// `Write2xUniverseFile`.
pub fn write_universe_file_2x(file: &Path, controllers: &[ControllerInfo]) -> Result<()> {
    fs::write(file, universe_config_2x(controllers))?;
    Ok(())
}

/// `WriteUniverseFile`'s output: the 1.x comma-separated format.
pub fn universe_config_1x(controllers: &[ControllerInfo]) -> String {
    let mut text = String::new();
    let mut fpp_start_channel = 1;
    for controller in active_in_order(controllers) {
        let Some(network) = &controller.network else {
            fpp_start_channel += controller.channels;
            continue;
        };
        let multicast = network.transmission == TransmissionMethod::Multicast;
        for universe in &network.universes {
            let ip = if multicast { "" } else { network.ip_address.as_str() };
            text.push_str(&format!(
                "{},{},{},{},{},{},\n",
                i32::from(universe.active),
                universe.universe_number,
                fpp_start_channel,
                universe.size,
                i32::from(multicast),
                ip,
            ));
            fpp_start_channel += universe.size;
        }
    }
    text
}

/// `WriteUniverseFile`.
pub fn write_universe_file_1x(file: &Path, controllers: &[ControllerInfo]) -> Result<()> {
    fs::write(file, universe_config_1x(controllers))?;
    Ok(())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// `WriteControllerInfo`'s output, the `<name>_Network.xml` file. `sequence_length` is
/// `TimeSpan.ToString()`, as the original writes it.
pub fn controller_info_xml(resolution: i32, out_file_name: &str, sequence_length: &str, controllers: &[ControllerInfo]) -> String {
    const NL: &str = crate::NEWLINE;
    let mut x = String::new();
    x.push('\u{FEFF}');
    x.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>");
    x.push_str(NL);
    x.push_str("<Vixen3_Export>");
    x.push_str(NL);
    x.push_str(&format!("\t<Resolution>{resolution}</Resolution>{NL}"));
    x.push_str(&format!("\t<OutFile>{}</OutFile>{NL}", escape_xml(file_name_of(out_file_name))));
    x.push_str(&format!("\t<Duration>{}</Duration>{NL}", escape_xml(sequence_length)));
    let active = active_in_order(controllers);
    if active.is_empty() {
        x.push_str(&format!("\t<Network />{NL}"));
    } else {
        x.push_str(&format!("\t<Network>{NL}"));
        let mut chan_start = 1;
        for c in active {
            x.push_str(&format!("\t\t<Controller>{NL}"));
            x.push_str(&format!("\t\t\t<Index>{}</Index>{NL}", c.index));
            x.push_str(&format!("\t\t\t<Name>{}</Name>{NL}", escape_xml(&c.name)));
            x.push_str(&format!("\t\t\t<StartChan>{chan_start}</StartChan>{NL}"));
            x.push_str(&format!("\t\t\t<Channels>{}</Channels>{NL}", c.channels));
            x.push_str(&format!("\t\t</Controller>{NL}"));
            chan_start += c.channels;
        }
        x.push_str(&format!("\t</Network>{NL}"));
    }
    x.push_str("</Vixen3_Export>");
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network(protocol: ProtocolType, universes: &[(i32, i32)]) -> NetworkConfig {
        NetworkConfig {
            protocol,
            transmission: TransmissionMethod::Unicast,
            ip_address: "10.0.0.5".into(),
            throttling_enabled: true,
            universes: universes.iter().map(|(n, size)| UniverseConfig { active: true, universe_number: *n, size: *size }).collect(),
        }
    }

    fn controller(name: &str, index: i32, channels: i32, net: Option<NetworkConfig>) -> ControllerInfo {
        ControllerInfo { id: Uuid::nil(), name: name.into(), index, is_active: true, channels, network: net }
    }

    fn owner_like() -> Vec<ControllerInfo> {
        let mut a = controller("Arduino", 0, 16, None);
        a.id = Uuid::from_u128(1);
        let mut b = controller("Light Strip Controller", 1, 3351, None);
        b.id = Uuid::from_u128(2);
        vec![a, b]
    }

    #[test]
    fn channel_ranges_walk_the_ticked_rows_in_index_order() {
        let mut list = owner_like();
        assert_eq!(channel_ranges(&list), [Some((1, 16)), Some((17, 3367))]);
        list[0].is_active = false;
        assert_eq!(channel_ranges(&list), [None, Some((1, 3351))]);
        list[0].is_active = true;
        let ids = [list[1].id, list[0].id];
        reindex(&mut list, &ids, &[true, true]);
        assert_eq!(list[1].index, 0);
        assert_eq!(list[0].index, 1);
        assert_eq!(channel_ranges(&list), [Some((1, 3351)), Some((3352, 3367))]);
        reindex(&mut list, &ids, &[true, false]);
        assert!(!list[0].is_active);
        assert_eq!(channel_ranges(&list), [Some((1, 3351)), None]);
    }

    #[test]
    fn consecutive_same_size_universes_merge_and_start_channels_accumulate() {
        let list = vec![
            controller("Serial", 0, 16, None),
            controller("E131", 1, 1024, Some(network(ProtocolType::Sacn, &[(1, 510), (2, 510), (5, 4)]))),
        ];
        let v: Value = serde_json::from_str(&universe_config_2x(&list)).unwrap();
        let universes = v["channelOutputs"][0]["universes"].as_array().unwrap();
        assert_eq!(universes.len(), 2);
        assert_eq!(universes[0]["startChannel"], 17);
        assert_eq!(universes[0]["universeCount"], 2);
        assert_eq!(universes[1]["startChannel"], 17 + 1020);
        assert_eq!(universes[1]["id"], 5);
        assert_eq!(universes[0]["type"], E131_UNICAST);
        assert_eq!(universes[0]["monitor"], 1);
        assert_eq!(universes[0]["deDuplicate"], 1);
    }

    #[test]
    fn members_come_in_upstreams_order_with_its_layout() {
        assert_eq!(
            universe_config_2x(&[]),
            "{\r\n  \"channelOutputs\": [\r\n    {\r\n      \"type\": \"universes\",\r\n      \"enabled\": 1,\r\n      \"startChannel\": 1,\r\n      \"channelCount\": -1,\r\n      \"universes\": []\r\n    }\r\n  ]\r\n}"
        );
        let list = vec![controller("Pixel \"A\"", 0, 1500, Some(network(ProtocolType::Ddp, &[(1, 1500)])))];
        let text = universe_config_2x(&list);
        let names: Vec<&str> = text.lines().filter_map(|l| l.trim().strip_prefix('"')).map(|l| &l[..l.find('"').unwrap()]).collect();
        assert_eq!(
            names,
            [
                "channelOutputs", "type", "enabled", "startChannel", "channelCount", "universes", "active", "description", "id", "startChannel",
                "universeCount", "channelCount", "universeType", "type", "address", "priority", "monitor", "deDuplicate"
            ]
        );
        assert!(text.contains("\"description\": \"Pixel \\\"A\\\"\""), "{text}");
    }

    #[test]
    fn ddp_uses_one_entry_from_the_first_universe() {
        let list = vec![controller("Pixel", 0, 1500, Some(network(ProtocolType::Ddp, &[(1, 1500)])))];
        let v: Value = serde_json::from_str(&universe_config_2x(&list)).unwrap();
        let u = &v["channelOutputs"][0]["universes"][0];
        assert_eq!(u["type"], DDP_1_BASED);
        assert_eq!(u["id"], 0);
        assert_eq!(u["channelCount"], 1500);
    }

    #[test]
    fn one_x_format_skips_address_for_multicast() {
        let mut n = network(ProtocolType::Sacn, &[(1, 510)]);
        n.transmission = TransmissionMethod::Multicast;
        let list = vec![controller("A", 0, 510, Some(n))];
        assert_eq!(universe_config_1x(&list), "1,1,1,510,1,,\n");
        let list = vec![controller("A", 0, 510, Some(network(ProtocolType::Sacn, &[(1, 510)])))];
        assert_eq!(universe_config_1x(&list), "1,1,1,510,0,10.0.0.5,\n");
    }

    #[test]
    fn inactive_controllers_are_ignored_and_order_follows_index() {
        let mut list = vec![controller("B", 1, 10, None), controller("A", 0, 5, None), controller("C", 2, 7, None)];
        list[2].is_active = false;
        let order: Vec<&str> = active_in_order(&list).iter().map(|c| c.name.as_str()).collect();
        assert_eq!(order, ["A", "B"]);
        assert!(!all_selected_support_universes(&list));
    }

    #[test]
    fn controller_info_lays_out_start_channels() {
        let list = vec![controller("A & B", 0, 16, None), controller("C", 1, 3351, None)];
        let xml = controller_info_xml(25, "/x/Song.fseq", "00:02:28.6500000", &list);
        assert!(xml.contains("<OutFile>Song.fseq</OutFile>"));
        assert!(xml.contains("<Name>A &amp; B</Name>"));
        assert!(xml.contains("<StartChan>17</StartChan>"));
    }
}
