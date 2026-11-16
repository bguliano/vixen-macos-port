//! DataContractSerializer helpers: member reading and the ordered, unknown-preserving member writer.
//!
//! A `Shape`, `PreviewData` or `DisplayItem` remembers the members a file gave it, in file order, in an
//! [`XmlMemo`]. The writer walks that memo so unknown members stay where they were and values Vixen
//! would not have written (explicit defaults) survive a load and save. Members that were not in the file
//! go where `DataContractSerializer` would put them: base class first, ordinal order within a class.

use vixen_xml::{XAttr, XElement};

pub const CONTRACT_BASE: &str = vixen_xml::CONTRACT_BASE;
pub const PREVIEW_NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Preview.VixenPreview";
pub const SHAPES_NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Preview.VixenPreview.Shapes";
pub const MEDIA3D_NS: &str = "http://schemas.datacontract.org/2004/07/System.Windows.Media.Media3D";
pub const WINDOWS_NS: &str = "http://schemas.datacontract.org/2004/07/System.Windows";

/// A known shape type failed to parse. The caller keeps the shape raw instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed;

pub type Parse<T> = Result<T, Malformed>;

pub fn read_i32(e: &XElement) -> Parse<i32> {
    e.text().trim().parse::<i32>().map_err(|_| Malformed)
}

pub fn read_bool(e: &XElement) -> Parse<bool> {
    match e.text().trim() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(Malformed),
    }
}

/// `XmlConvert.ToDouble`.
pub fn parse_double(t: &str) -> Option<f64> {
    match t.trim() {
        "INF" => Some(f64::INFINITY),
        "-INF" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        s => s.parse::<f64>().ok(),
    }
}

/// The text `DataContractSerializer` writes for a double: `Double.ToString("R")`, with INF spelled the XML way.
pub fn fmt_double(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "INF".to_string() } else { "-INF".to_string() };
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0".to_string() } else { "0".to_string() };
    }
    let sci = format!("{v:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    // .NET's general format: scientific when the decimal point position is above 15 or below -3.
    if (-4..15).contains(&exp) {
        format!("{v}")
    } else {
        format!("{mantissa}E{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    }
}

/// A double with the file's exact text, kept only when our writer would have spelled it differently,
/// so an untouched value always goes back unchanged.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DoubleText {
    pub value: f64,
    raw: Option<Box<str>>,
}

impl DoubleText {
    pub fn new(value: f64) -> DoubleText {
        DoubleText { value, raw: None }
    }

    pub fn parse(text: &str) -> Parse<DoubleText> {
        let t = text.trim();
        let value = parse_double(t).ok_or(Malformed)?;
        let raw = if fmt_double(value) == t { None } else { Some(t.into()) };
        Ok(DoubleText { value, raw })
    }

    pub fn text(&self) -> String {
        match &self.raw {
            Some(r) => r.to_string(),
            None => fmt_double(self.value),
        }
    }
}

pub fn el_text(ns: &str, local: &str, text: impl Into<String>) -> XElement {
    XElement::with_text(ns, local, text)
}

pub fn el_i32(ns: &str, local: &str, v: i32) -> XElement {
    el_text(ns, local, v.to_string())
}

pub fn el_bool(ns: &str, local: &str, v: bool) -> XElement {
    el_text(ns, local, if v { "true" } else { "false" })
}

/// One data member about to be written.
pub struct Member {
    pub name: &'static str,
    pub el: XElement,
    /// Whether `DataContractSerializer` would write it: false for `EmitDefaultValue = false` at the default.
    pub emit: bool,
}

impl Member {
    pub fn new(name: &'static str, el: XElement) -> Member {
        Member { name, el, emit: true }
    }

    /// A member with `EmitDefaultValue = false`.
    pub fn optional(name: &'static str, el: XElement, emit: bool) -> Member {
        Member { name, el, emit }
    }
}

#[derive(Clone, Debug)]
enum Slot {
    Known { name: &'static str, explicit_default: bool },
    Extra(XElement),
}

/// What a file held for one object, beyond the typed values.
#[derive(Clone, Debug, Default)]
pub struct XmlMemo {
    slots: Vec<Slot>,
    pub attrs: Vec<XAttr>,
}

impl XmlMemo {
    pub fn with_attrs(attrs: &[XAttr]) -> XmlMemo {
        XmlMemo { slots: Vec::new(), attrs: attrs.to_vec() }
    }

    pub fn known(&mut self, name: &'static str) {
        self.slots.push(Slot::Known { name, explicit_default: false });
    }

    /// A known member whose value is the default that Vixen itself would have left out.
    pub fn known_default(&mut self, name: &'static str, explicit_default: bool) {
        self.slots.push(Slot::Known { name, explicit_default });
    }

    pub fn extra(&mut self, e: &XElement) {
        self.slots.push(Slot::Extra(e.clone()));
    }

    pub fn has_known(&self, name: &str) -> bool {
        self.slots.iter().any(|s| matches!(s, Slot::Known { name: n, .. } if *n == name))
    }
}

/// Appends members to `parent`, following `memo` when there is one.
pub fn write_members(parent: &mut XElement, known: Vec<Member>, memo: Option<&XmlMemo>) {
    let Some(memo) = memo else {
        for m in known {
            if m.emit {
                parent.push(m.el);
            }
        }
        return;
    };
    parent.attrs.extend(memo.attrs.iter().cloned());

    // Canonical index, or None for an unknown member.
    let mut out: Vec<(Option<usize>, XElement)> = Vec::with_capacity(known.len() + 2);
    let mut known: Vec<Option<Member>> = known.into_iter().map(Some).collect();
    for slot in &memo.slots {
        match slot {
            Slot::Extra(e) => out.push((None, e.clone())),
            Slot::Known { name, explicit_default } => {
                let idx = known.iter().position(|m| matches!(m, Some(m) if m.name == *name));
                if let Some(i) = idx {
                    if let Some(m) = known[i].take() {
                        if m.emit || *explicit_default {
                            out.push((Some(i), m.el));
                        }
                    }
                }
            }
        }
    }
    // Anything new since load goes in canonical position relative to the known members around it. A member
    // the file did not have and which still reads as what its absence means stays out: an older file gets no
    // new members just for being saved.
    for (i, m) in known.into_iter().enumerate() {
        let Some(m) = m else { continue };
        if !m.emit || is_zero_value(&m.el) {
            continue;
        }
        let pos = out.iter().position(|(c, _)| matches!(c, Some(c) if *c > i)).unwrap_or(out.len());
        out.insert(pos, (Some(i), m.el));
    }
    for (_, e) in out {
        parent.push(e);
    }
}

/// Whether an element holds what `DataContractSerializer` leaves in a member it finds no element for.
fn is_zero_value(e: &XElement) -> bool {
    if e.nil {
        return true;
    }
    if e.elements().next().is_some() {
        return e.elements().all(is_zero_value);
    }
    matches!(e.text().trim(), "" | "0" | "false" | "Standard" | "Clockwise" | "Vertical" | "No" | "Bottom")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_format_like_dotnet_round_trip() {
        assert_eq!(fmt_double(0.0), "0");
        assert_eq!(fmt_double(4.6), "4.6");
        assert_eq!(fmt_double(158.0), "158");
        assert_eq!(fmt_double(0.0001), "0.0001");
        assert_eq!(fmt_double(0.00001), "1E-05");
        assert_eq!(fmt_double(1.4210854715202004e-14), "1.4210854715202004E-14");
        assert_eq!(fmt_double(100000000000000.0), "100000000000000");
        assert_eq!(fmt_double(1e15), "1E+15");
        assert_eq!(fmt_double(-2.5e300), "-2.5E+300");
        assert_eq!(fmt_double(f64::INFINITY), "INF");
    }

    #[test]
    fn double_text_keeps_a_foreign_spelling() {
        let d = DoubleText::parse("1.50").unwrap();
        assert_eq!(d.value, 1.5);
        assert_eq!(d.text(), "1.50");
        assert_eq!(DoubleText::parse("1.5").unwrap().text(), "1.5");
        assert!(DoubleText::parse("x").is_err());
    }

    #[test]
    fn writer_keeps_unknown_members_in_place_and_inserts_new_ones_canonically() {
        let mut memo = XmlMemo::default();
        memo.known("A");
        memo.extra(&XElement::with_text("", "Unknown", "u"));
        memo.known("C");
        let mut p = XElement::new("", "P");
        write_members(
            &mut p,
            vec![
                Member::new("A", el_i32("", "A", 1)),
                Member::new("B", el_i32("", "B", 2)),
                Member::new("C", el_i32("", "C", 3)),
                Member::optional("D", el_i32("", "D", 0), false),
            ],
            Some(&memo),
        );
        let names: Vec<&str> = p.elements().map(|e| e.local.as_str()).collect();
        assert_eq!(names, ["A", "Unknown", "B", "C"]);
    }

    #[test]
    fn writer_keeps_an_explicit_default() {
        let mut memo = XmlMemo::default();
        memo.known_default("L", true);
        let mut p = XElement::new("", "P");
        write_members(&mut p, vec![Member::optional("L", el_bool("", "L", false), false)], Some(&memo));
        assert_eq!(p.elements().count(), 1);
    }
}
