//! A small owned XML tree with the namespace handling DataContractSerializer files need.
//!
//! `XElement` plays the role of LINQ to XML's `XElement` in the C#: migrators edit it, codecs read
//! and build it. Names are always resolved `{namespace}local`, so prefixes never matter when reading.
//! `i:type` and `i:nil` are first-class because their values and presence carry meaning.

use std::fmt::Write as _;

pub mod nbfx;

pub const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";
/// Prefix of a CLR namespace's data contract namespace.
pub const CONTRACT_BASE: &str = "http://schemas.datacontract.org/2004/07/";
pub const ARRAYS_NS: &str = "http://schemas.microsoft.com/2003/10/Serialization/Arrays";

/// The data contract namespace for a CLR namespace.
pub fn contract_ns(clr_namespace: &str) -> String {
    format!("{CONTRACT_BASE}{clr_namespace}")
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct XAttr {
    /// Empty for an unqualified attribute.
    pub ns: String,
    pub local: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XNode {
    Element(XElement),
    Text(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct XElement {
    /// Empty for no namespace.
    pub ns: String,
    pub local: String,
    /// Attributes other than `xmlns`, `i:type` and `i:nil`.
    pub attrs: Vec<XAttr>,
    /// `i:type`, as (namespace, local name).
    pub xsi_type: Option<(String, String)>,
    /// `i:nil="true"`.
    pub nil: bool,
    pub children: Vec<XNode>,
}

#[derive(Debug)]
pub struct XmlError(pub String);

impl std::fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for XmlError {}

impl XElement {
    pub fn new(ns: &str, local: &str) -> XElement {
        XElement { ns: ns.to_string(), local: local.to_string(), ..XElement::default() }
    }

    /// An element holding only text.
    pub fn with_text(ns: &str, local: &str, text: impl Into<String>) -> XElement {
        let mut e = XElement::new(ns, local);
        e.children.push(XNode::Text(text.into()));
        e
    }

    /// An `i:nil="true"` element.
    pub fn nil(ns: &str, local: &str) -> XElement {
        XElement { nil: true, ..XElement::new(ns, local) }
    }

    /// Parses a document. A leading BOM is accepted, as Vixen writes one.
    pub fn parse(text: &str) -> Result<XElement, XmlError> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
        let doc = roxmltree::Document::parse_with_options(text, opts).map_err(|e| XmlError(e.to_string()))?;
        Ok(from_node(doc.root_element()))
    }

    /// Reads and parses a file.
    pub fn from_file(path: &std::path::Path) -> Result<XElement, XmlError> {
        let bytes = std::fs::read(path).map_err(|e| XmlError(format!("{}: {e}", path.display())))?;
        let text = String::from_utf8_lossy(&bytes);
        XElement::parse(&text).map_err(|e| XmlError(format!("{}: {e}", path.display())))
    }

    pub fn is(&self, local: &str) -> bool {
        self.local == local
    }

    /// Child elements in document order.
    pub fn elements(&self) -> impl Iterator<Item = &XElement> {
        self.children.iter().filter_map(|n| match n {
            XNode::Element(e) => Some(e),
            XNode::Text(_) => None,
        })
    }

    pub fn elements_mut(&mut self) -> impl Iterator<Item = &mut XElement> {
        self.children.iter_mut().filter_map(|n| match n {
            XNode::Element(e) => Some(e),
            XNode::Text(_) => None,
        })
    }

    /// The first child with this local name, in any namespace.
    pub fn child(&self, local: &str) -> Option<&XElement> {
        self.elements().find(|e| e.local == local)
    }

    pub fn child_mut(&mut self, local: &str) -> Option<&mut XElement> {
        self.elements_mut().find(|e| e.local == local)
    }

    /// Children with this local name.
    pub fn children_named<'a>(&'a self, local: &'a str) -> impl Iterator<Item = &'a XElement> + 'a {
        self.elements().filter(move |e| e.local == local)
    }

    /// Concatenated direct text, untrimmed.
    pub fn text(&self) -> String {
        let mut s = String::new();
        for n in &self.children {
            if let XNode::Text(t) = n {
                s.push_str(t);
            }
        }
        s
    }

    /// The trimmed text of a named child.
    pub fn child_text(&self, local: &str) -> Option<String> {
        self.child(local).map(|c| c.text().trim().to_string())
    }

    /// An unqualified or namespaced attribute value, matched by local name.
    pub fn attr(&self, local: &str) -> Option<&str> {
        self.attrs.iter().find(|a| a.local == local).map(|a| a.value.as_str())
    }

    pub fn set_attr(&mut self, local: &str, value: impl Into<String>) {
        let value = value.into();
        match self.attrs.iter_mut().find(|a| a.local == local) {
            Some(a) => a.value = value,
            None => self.attrs.push(XAttr { ns: String::new(), local: local.to_string(), value }),
        }
    }

    pub fn push(&mut self, e: XElement) -> &mut XElement {
        self.children.push(XNode::Element(e));
        self
    }

    /// Serialises with a `<?xml ?>` declaration, indented like XmlWriter with `Indent = true`.
    pub fn to_document(&self) -> String {
        let mut out = String::from("\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        let mut w = Writer { out: &mut out, next_prefix: 0 };
        w.write(self, &mut Vec::new(), "", 0);
        out
    }

    /// Serialises this element alone, with no declaration.
    pub fn to_fragment(&self) -> String {
        let mut out = String::new();
        let mut w = Writer { out: &mut out, next_prefix: 0 };
        w.write(self, &mut Vec::new(), "", 0);
        out
    }

    /// Whether two trees mean the same thing: same names, namespaces, attributes, types, nils and
    /// children in the same order. Whitespace-only text is ignored, as it is only indentation.
    pub fn semantic_eq(&self, o: &XElement) -> bool {
        self.first_difference(o, String::new()).is_none()
    }

    /// The path and description of the first semantic difference, for readable test failures.
    pub fn first_difference(&self, o: &XElement, path: String) -> Option<String> {
        let path = format!("{path}/{}", self.local);
        if self.local != o.local {
            return Some(format!("{path}: name differs, other has {}", o.local));
        }
        if self.ns != o.ns {
            return Some(format!("{path}: namespace {:?} vs {:?}", self.ns, o.ns));
        }
        if self.nil != o.nil {
            return Some(format!("{path}: nil {} vs {}", self.nil, o.nil));
        }
        if self.xsi_type != o.xsi_type {
            return Some(format!("{path}: type {:?} vs {:?}", self.xsi_type, o.xsi_type));
        }
        let mut a: Vec<_> = self.attrs.iter().collect();
        let mut b: Vec<_> = o.attrs.iter().collect();
        a.sort_by(|x, y| (&x.ns, &x.local).cmp(&(&y.ns, &y.local)));
        b.sort_by(|x, y| (&x.ns, &x.local).cmp(&(&y.ns, &y.local)));
        if a != b {
            return Some(format!("{path}: attributes {a:?} vs {b:?}"));
        }
        let ta: Vec<&XNode> = significant(&self.children);
        let tb: Vec<&XNode> = significant(&o.children);
        if ta.len() != tb.len() {
            let names = |v: &[&XNode]| {
                v.iter()
                    .map(|n| match n {
                        XNode::Element(e) => e.local.clone(),
                        XNode::Text(t) => format!("\"{}\"", t.trim()),
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            };
            return Some(format!("{path}: {} children vs {}: [{}] vs [{}]", ta.len(), tb.len(), trunc(&names(&ta)), trunc(&names(&tb))));
        }
        for (x, y) in ta.iter().zip(tb.iter()) {
            match (x, y) {
                (XNode::Text(p), XNode::Text(q)) => {
                    if p.trim() != q.trim() {
                        return Some(format!("{path}: text {:?} vs {:?}", p.trim(), q.trim()));
                    }
                }
                (XNode::Element(p), XNode::Element(q)) => {
                    if let Some(d) = p.first_difference(q, path.clone()) {
                        return Some(d);
                    }
                }
                _ => return Some(format!("{path}: text vs element")),
            }
        }
        None
    }
}

fn trunc(s: &str) -> String {
    s.chars().take(300).collect()
}

fn significant(children: &[XNode]) -> Vec<&XNode> {
    children
        .iter()
        .filter(|n| match n {
            XNode::Text(t) => !t.trim().is_empty(),
            XNode::Element(_) => true,
        })
        .collect()
}

fn from_node(n: roxmltree::Node) -> XElement {
    let tag = n.tag_name();
    let mut e = XElement::new(tag.namespace().unwrap_or(""), tag.name());
    for a in n.attributes() {
        let ans = a.namespace().unwrap_or("");
        if ans == XSI_NS && a.name() == "nil" {
            e.nil = a.value() == "true";
        } else if ans == XSI_NS && a.name() == "type" {
            let v = a.value();
            let (prefix, local) = match v.split_once(':') {
                Some((p, l)) => (Some(p), l),
                None => (None, v),
            };
            let ns = n.lookup_namespace_uri(prefix).unwrap_or("");
            e.xsi_type = Some((ns.to_string(), local.to_string()));
        } else {
            e.attrs.push(XAttr { ns: ans.to_string(), local: a.name().to_string(), value: a.value().to_string() });
        }
    }
    for c in n.children() {
        if c.is_element() {
            e.children.push(XNode::Element(from_node(c)));
        } else if c.is_text() {
            if let Some(t) = c.text() {
                e.children.push(XNode::Text(t.to_string()));
            }
        }
    }
    e
}

struct Writer<'a> {
    out: &'a mut String,
    next_prefix: u32,
}

impl Writer<'_> {
    /// `scope` holds (prefix, namespace) in force; `default_ns` is the default namespace in force.
    fn write(&mut self, e: &XElement, scope: &mut Vec<(String, String)>, default_ns: &str, depth: usize) {
        let mark = scope.len();
        let mut decls = String::new();
        // Prefer the default namespace for the element itself, as DataContractSerializer often does.
        let mut default_here = default_ns.to_string();
        if e.ns != default_here {
            let _ = write!(decls, " xmlns=\"{}\"", escape_attr(&e.ns));
            default_here = e.ns.clone();
        }
        let mut attrs_text = String::new();
        if e.xsi_type.is_some() || e.nil {
            let p = self.prefix_for(XSI_NS, scope, &mut decls, Some("i"));
            if let Some((tns, tlocal)) = &e.xsi_type {
                let tp = if tns.is_empty() { String::new() } else { format!("{}:", self.prefix_for(tns, scope, &mut decls, None)) };
                let _ = write!(attrs_text, " {p}:type=\"{tp}{}\"", escape_attr(tlocal));
            }
            if e.nil {
                let _ = write!(attrs_text, " {p}:nil=\"true\"");
            }
        }
        for a in &e.attrs {
            if a.ns.is_empty() {
                let _ = write!(attrs_text, " {}=\"{}\"", a.local, escape_attr(&a.value));
            } else {
                let p = self.prefix_for(&a.ns, scope, &mut decls, None);
                let _ = write!(attrs_text, " {p}:{}=\"{}\"", a.local, escape_attr(&a.value));
            }
        }

        indent(self.out, depth);
        let _ = write!(self.out, "<{}{decls}{attrs_text}", e.local);
        let content: Vec<&XNode> = significant(&e.children);
        if content.is_empty() {
            self.out.push_str(" />");
        } else if content.iter().all(|n| matches!(n, XNode::Text(_))) {
            self.out.push('>');
            for n in &content {
                if let XNode::Text(t) = n {
                    self.out.push_str(&escape_text(t));
                }
            }
            let _ = write!(self.out, "</{}>", e.local);
        } else {
            self.out.push('>');
            for n in &content {
                self.out.push('\n');
                match n {
                    XNode::Element(c) => self.write(c, scope, &default_here, depth + 1),
                    XNode::Text(t) => self.out.push_str(&escape_text(t)),
                }
            }
            self.out.push('\n');
            indent(self.out, depth);
            let _ = write!(self.out, "</{}>", e.local);
        }
        scope.truncate(mark);
    }

    fn prefix_for(&mut self, ns: &str, scope: &mut Vec<(String, String)>, decls: &mut String, want: Option<&str>) -> String {
        if let Some((p, _)) = scope.iter().rev().find(|(_, n)| n == ns) {
            return p.clone();
        }
        let prefix = match want {
            Some(w) if !scope.iter().any(|(p, _)| p == w) => w.to_string(),
            _ => {
                self.next_prefix += 1;
                format!("d{}", self.next_prefix)
            }
        };
        let _ = write!(decls, " xmlns:{prefix}=\"{}\"", escape_attr(ns));
        scope.push((prefix.clone(), ns.to_string()));
        prefix
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn escape_text(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            _ => o.push(c),
        }
    }
    o
}

fn escape_attr(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            _ => o.push(c),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<TimedSequenceData version="8" xmlns:a="http://www.w3.org/2001/XMLSchema" xmlns:i="http://www.w3.org/2001/XMLSchema-instance" xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed">
  <Length xmlns="">PT1M</Length>
  <_dataModels xmlns:d1p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays" xmlns="">
    <d1p1:anyType xmlns:d2p1="http://schemas.datacontract.org/2004/07/VixenModules.Effect.Wipe" i:type="d2p1:WipeData">
      <d2p1:Name>A &amp; B</d2p1:Name>
    </d1p1:anyType>
  </_dataModels>
  <DefaultPlaybackEndTime i:nil="true" />
</TimedSequenceData>"#;

    #[test]
    fn parses_names_types_and_nil() {
        let e = XElement::parse(SAMPLE).unwrap();
        assert_eq!(e.local, "TimedSequenceData");
        assert_eq!(e.ns, contract_ns("VixenModules.Sequence.Timed"));
        assert_eq!(e.attr("version"), Some("8"));
        assert_eq!(e.child_text("Length").as_deref(), Some("PT1M"));
        let any = e.child("_dataModels").unwrap().child("anyType").unwrap();
        assert_eq!(any.ns, ARRAYS_NS);
        assert_eq!(any.xsi_type, Some((contract_ns("VixenModules.Effect.Wipe"), "WipeData".to_string())));
        assert_eq!(any.child_text("Name").as_deref(), Some("A & B"));
        assert!(e.child("DefaultPlaybackEndTime").unwrap().nil);
    }

    #[test]
    fn round_trips_through_the_writer() {
        let e = XElement::parse(SAMPLE).unwrap();
        let text = e.to_document();
        let back = XElement::parse(&text).unwrap();
        assert!(e.semantic_eq(&back), "{:?}\n{text}", e.first_difference(&back, String::new()));
    }

    #[test]
    fn detects_differences() {
        let a = XElement::parse(SAMPLE).unwrap();
        let mut b = a.clone();
        b.child_mut("Length").unwrap().children = vec![XNode::Text("PT2M".to_string())];
        let d = a.first_difference(&b, String::new()).unwrap();
        assert!(d.contains("Length"), "{d}");
    }

    #[test]
    fn accepts_a_byte_order_mark() {
        assert_eq!(XElement::parse("\u{feff}<a/>").unwrap().local, "a");
    }

    #[test]
    fn escapes_on_output() {
        let mut e = XElement::with_text("", "a", "x<y&z");
        e.set_attr("b", "\"q\"");
        let back = XElement::parse(&e.to_fragment()).unwrap();
        assert_eq!(back.text(), "x<y&z");
        assert_eq!(back.attr("b"), Some("\"q\""));
    }
}
