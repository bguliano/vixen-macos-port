//! The DataContractSerializer dictionary used by Vixen2xSequenceStaticData.
use super::*;
use std::collections::BTreeMap;
use vixen_io::{
    values::{color_from_xml, DotNetColor},
    ModuleEntry, ModuleStore,
};
use vixen_xml::ARRAYS_NS;
const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.SequenceType.Vixen2x";
const ENTRY: &str = "KeyValueOfstringArrayOfChannelMapping3YEWi_PPm";
pub type Maps = BTreeMap<String, Vec<ChannelMapping>>;
pub fn load(store: &ModuleStore) -> Maps {
    let mut maps = Maps::new();
    let Some(list) = store
        .type_data(MODULE_TYPE)
        .and_then(ModuleEntry::data)
        .and_then(|d| d.child("vixen2xMappings"))
    else {
        return maps;
    };
    for item in list.elements() {
        let Some(key) = item.child_text("Key") else {
            continue;
        };
        let rows = item
            .child("Value")
            .into_iter()
            .flat_map(XElement::elements)
            .map(|e| ChannelMapping {
                name: e.child_text("ChannelName").unwrap_or_default(),
                number: e.child_text("ChannelNumber").unwrap_or_default(),
                output: e.child_text("ChannelOutput").unwrap_or_default(),
                color: e
                    .child("ChannelColor")
                    .and_then(color_from_xml)
                    .unwrap_or_default(),
                destination: e
                    .child_text("ElementNodeId")
                    .and_then(|s| Uuid::parse_str(&s).ok())
                    .filter(|id| !id.is_nil()),
                import_color: e
                    .child("DestinationColor")
                    .and_then(color_from_xml)
                    .unwrap_or_default(),
                mixing: e.child_text("RgbPixel").is_some_and(|v| v == "true"),
            })
            .collect();
        maps.insert(key, rows);
    }
    maps
}
pub fn xml(maps: &Maps) -> XElement {
    let mut data = XElement::new(NS, "Vixen2xSequenceStaticData");
    data.push(XElement::with_text(
        "",
        "ModuleInstanceId",
        MODULE_TYPE.to_string(),
    ));
    data.push(XElement::with_text(
        "",
        "ModuleTypeId",
        MODULE_TYPE.to_string(),
    ));
    let mut list = XElement::new(NS, "vixen2xMappings");
    for (name, rows) in maps {
        let mut entry = XElement::new(ARRAYS_NS, ENTRY);
        entry.push(XElement::with_text(ARRAYS_NS, "Key", name));
        let mut value = XElement::new(ARRAYS_NS, "Value");
        for row in rows {
            let mut e = XElement::new(NS, "ChannelMapping");
            e.push(DotNetColor::from_color(row.color).to_xml(NS, "ChannelColor"));
            e.push(XElement::with_text(NS, "ChannelName", &row.name));
            e.push(XElement::with_text(NS, "ChannelNumber", &row.number));
            e.push(XElement::with_text(NS, "ChannelOutput", &row.output));
            // RGB mapping relies on named colors in Windows, not merely equal ARGB values.
            let color = match row.import_color {
                Color::RED => DotNetColor::from_known(141),
                Color::GREEN => DotNetColor::from_known(79),
                Color::BLUE => DotNetColor::from_known(37),
                _ => DotNetColor::from_color(row.import_color),
            };
            e.push(color.to_xml(NS, "DestinationColor"));
            e.push(XElement::with_text(
                NS,
                "ElementNodeId",
                row.destination.unwrap_or(Uuid::nil()).to_string(),
            ));
            e.push(XElement::with_text(NS, "RgbPixel", row.mixing.to_string()));
            value.push(e);
        }
        entry.push(value);
        list.push(entry);
    }
    data.push(list);
    data
}
pub fn save(store: &mut ModuleStore, maps: &Maps) {
    let old = xml(&load(store));
    if let Some(entry) = store.type_data_mut(MODULE_TYPE) {
        entry.merge_data(&old, xml(maps));
    } else {
        store.set_type_data(ModuleEntry::new(
            "VixenModules.SequenceType.Vixen2x.Vixen2xSequenceStaticData, Vixen2x",
            &MODULE_TYPE,
            &MODULE_TYPE,
            "Module.SequenceType.Vixen2x",
            xml(maps),
        ));
    }
}
pub fn normalized(rows: &[ChannelMapping], tree: &ElementTree) -> Vec<ChannelMapping> {
    rows.iter()
        .cloned()
        .map(|mut row| {
            if !row.destination.is_some_and(|id| tree.find(&id).is_some()) {
                row.destination = None;
                row.import_color = Color::EMPTY;
                row.mixing = false;
            }
            row
        })
        .collect()
}
