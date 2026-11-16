//! Helpers for reading and writing module data the way `DataContractSerializer` lays it out: a root
//! named after the type, `ModuleInstanceId` and `ModuleTypeId` in the empty namespace from the base
//! class, then the members in ordinal order.
//!
//! Reading follows the serializer: a missing member keeps the type's zero value, because a data
//! contract is deserialized without running its constructor.

use crate::dotnet::DotNetColor;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

/// `ModuleTypeId` and `ModuleInstanceId` from `ModuleDataModelBase`. An E1.31 controller's instance id
/// is also its sACN CID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ModuleIds {
    pub type_id: Uuid,
    pub instance_id: Uuid,
}

impl ModuleIds {
    pub fn new(type_id: Uuid, instance_id: Uuid) -> ModuleIds {
        ModuleIds { type_id, instance_id }
    }

    pub(crate) fn read(e: &XElement) -> ModuleIds {
        ModuleIds { type_id: get_uuid(e, "ModuleTypeId"), instance_id: get_uuid(e, "ModuleInstanceId") }
    }
}

/// Module data may be handed over as the `<Module>` wrapper from `ModuleStore.xml` or as the data element
/// inside it. Returns the data element and, for a wrapper, the ids on its attributes. `ReadObject` lets
/// those win over the ones inside the data.
pub fn unwrap_module(e: &XElement) -> (&XElement, Option<ModuleIds>) {
    if e.local == "Module" {
        let ids = match (e.attr("moduleType"), e.attr("moduleInstance")) {
            (Some(t), Some(i)) => match (Uuid::parse_str(t), Uuid::parse_str(i)) {
                (Ok(t), Ok(i)) => Some(ModuleIds::new(t, i)),
                _ => None,
            },
            _ => None,
        };
        if let Some(data) = e.elements().next() {
            return (data, ids);
        }
    }
    (e, None)
}

pub(crate) fn data_root(ns: &str, local: &str, ids: &ModuleIds) -> XElement {
    let mut e = XElement::new(ns, local);
    e.push(XElement::with_text("", "ModuleInstanceId", ids.instance_id.to_string()));
    e.push(XElement::with_text("", "ModuleTypeId", ids.type_id.to_string()));
    e
}

pub(crate) fn text_el(ns: &str, local: &str, text: &str) -> XElement {
    if text.is_empty() {
        XElement::new(ns, local)
    } else {
        XElement::with_text(ns, local, text)
    }
}

pub(crate) fn opt_text_el(ns: &str, local: &str, text: Option<&str>) -> XElement {
    match text {
        Some(t) => text_el(ns, local, t),
        None => XElement::nil(ns, local),
    }
}

pub(crate) fn bool_el(ns: &str, local: &str, v: bool) -> XElement {
    XElement::with_text(ns, local, if v { "true" } else { "false" })
}

pub(crate) fn num_el(ns: &str, local: &str, v: impl std::fmt::Display) -> XElement {
    XElement::with_text(ns, local, v.to_string())
}

/// A string member: `None` when it is missing or `i:nil`. Text keeps its whitespace, because a string
/// data member keeps it.
pub(crate) fn get_opt_string(e: &XElement, local: &str) -> Option<String> {
    let c = e.child(local)?;
    if c.nil {
        None
    } else {
        Some(c.text())
    }
}

pub(crate) fn get_bool(e: &XElement, local: &str) -> bool {
    e.child_text(local).is_some_and(|t| t == "true" || t == "1")
}

pub(crate) fn get_i32(e: &XElement, local: &str) -> i32 {
    e.child_text(local).and_then(|t| t.parse().ok()).unwrap_or(0)
}

pub(crate) fn get_u8(e: &XElement, local: &str) -> u8 {
    e.child_text(local).and_then(|t| t.parse().ok()).unwrap_or(0)
}

pub(crate) fn get_uuid(e: &XElement, local: &str) -> Uuid {
    e.child_text(local).and_then(|t| Uuid::parse_str(&t).ok()).unwrap_or_default()
}

const DRAWING_NS: &str = "System.Drawing";

/// Reads a `System.Drawing.Color` member. `name` and `state` come through untouched so it writes back
/// exactly, and so `.Equals(Color.Red)` still gives the C# answer.
pub(crate) fn read_color(e: &XElement) -> DotNetColor {
    DotNetColor {
        known_color: e.child_text("knownColor").and_then(|t| t.parse().ok()).unwrap_or(0),
        name: get_opt_string(e, "name"),
        state: e.child_text("state").and_then(|t| t.parse().ok()).unwrap_or(0),
        value: e.child_text("value").and_then(|t| t.parse().ok()).unwrap_or(0),
    }
}

pub(crate) fn color_el(ns: &str, local: &str, c: &DotNetColor) -> XElement {
    let d = contract_ns(DRAWING_NS);
    let mut e = XElement::new(ns, local);
    e.push(num_el(&d, "knownColor", c.known_color));
    e.push(opt_text_el(&d, "name", c.name.as_deref()));
    e.push(num_el(&d, "state", c.state));
    e.push(num_el(&d, "value", c.value));
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwraps_a_module_element_and_prefers_its_attribute_ids() {
        let xml = r#"<Module moduleType="11111111-1111-1111-1111-111111111111" moduleInstance="22222222-2222-2222-2222-222222222222">
            <Data xmlns="urn:x"><ModuleInstanceId xmlns="">33333333-3333-3333-3333-333333333333</ModuleInstanceId></Data></Module>"#;
        let e = XElement::parse(xml).unwrap();
        let (data, ids) = unwrap_module(&e);
        assert_eq!(data.local, "Data");
        assert_eq!(ids.unwrap().instance_id, uuid::uuid!("22222222-2222-2222-2222-222222222222"));
    }

    #[test]
    fn strings_keep_whitespace_and_nil_is_none() {
        let e = XElement::parse(r#"<a xmlns:i="http://www.w3.org/2001/XMLSchema-instance"><b> x </b><c i:nil="true"/><d/></a>"#)
            .unwrap();
        assert_eq!(get_opt_string(&e, "b").as_deref(), Some(" x "));
        assert_eq!(get_opt_string(&e, "c"), None);
        assert_eq!(get_opt_string(&e, "d").as_deref(), Some(""));
        assert_eq!(get_opt_string(&e, "e"), None);
    }
}
