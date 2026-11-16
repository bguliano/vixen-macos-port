//! Reading files into `XElement`s and writing them back the way Vixen's `XElement.Save` does.
//!
//! `XElement::to_document` in vixen-xml is correct but it declares `xmlns:i` and friends again on every
//! element that needs them, which makes a sequence file about 60% larger, and it drops a value made only of
//! spaces. Real files declare a prefix once, where DataContractSerializer's writer would, and this writer
//! reproduces that placement:
//!
//! - an element whose type lives in another namespace declares a `d{depth}p1` prefix for its members,
//! - a stray member in yet another namespace (a base class member) switches the default namespace instead,
//! - sequences declare `a` and `i` on the root, while each object in the module store declares its own `i`.

use crate::error::IoError;
use std::path::Path;
use vixen_xml::{XElement, XNode, XSI_NS};

const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// What differs between the document kinds Vixen writes.
#[derive(Copy, Clone, Debug, Default)]
pub struct DocStyle {
    /// Prefixes declared on the root element, as a sequence does for `a` and `i`.
    pub root_prefixes: &'static [(&'static str, &'static str)],
    /// The depth of elements that were serialized on their own and pasted in, such as the object inside
    /// a module store `<Module>`. They declare `xmlns:i` themselves and name their prefixes `a`, `b`, `c`.
    pub object_depth: Option<usize>,
}

/// Serialises a whole file the way `XElement.Save` does: BOM, declaration, CRLF and two-space indents.
pub fn write_document(root: &XElement, style: DocStyle) -> String {
    let mut w = Writer {
        out: String::with_capacity(64 * 1024),
        newline: "\r\n",
        indent: true,
        scope: Vec::with_capacity(32),
        style,
    };
    w.out.push_str("\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n");
    w.element(root, 0, false, "");
    w.out
}

/// Serialises one element with every namespace it needs declared on it, so it can stand alone. Used for
/// the raw XML kept in `Params::unknown_members` and `ParamValue::Raw`.
pub fn write_fragment(e: &XElement) -> String {
    let mut w = Writer {
        out: String::with_capacity(128),
        newline: "\n",
        indent: false,
        scope: Vec::with_capacity(8),
        style: DocStyle::default(),
    };
    w.element(e, 0, false, "");
    w.out
}

/// Parses what [`write_fragment`] wrote.
pub fn parse_fragment(text: &str) -> Option<XElement> {
    XElement::parse(text).ok()
}

/// Reads a file the way `XElementFileReader` does: `StreamReader` honours a BOM and defaults to UTF-8.
pub fn read_xml_file(path: &Path) -> Result<XElement, IoError> {
    let bytes = std::fs::read(path).map_err(|e| IoError::io(path, e))?;
    let text = decode_text(&bytes);
    XElement::parse(&text).map_err(|e| IoError::Xml { path: path.to_path_buf(), message: e.0 })
}

fn decode_text(bytes: &[u8]) -> String {
    let utf16 = |be: bool| {
        let (pairs, _) = bytes[2..].as_chunks::<2>();
        let units: Vec<u16> =
            pairs.iter().map(|c| if be { u16::from_be_bytes(*c) } else { u16::from_le_bytes(*c) }).collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xFF, 0xFE, ..] => utf16(false),
        [0xFE, 0xFF, ..] => utf16(true),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// `first_difference(..).is_none()` without building a path string for every element it visits.
pub fn same_tree(a: &XElement, b: &XElement) -> bool {
    if !same_header(a, b) {
        return false;
    }
    let mut x = a.children.iter().filter(|n| is_significant(n));
    let mut y = b.children.iter().filter(|n| is_significant(n));
    loop {
        match (x.next(), y.next()) {
            (None, None) => return true,
            (Some(XNode::Text(p)), Some(XNode::Text(q))) => {
                if p.trim() != q.trim() {
                    return false;
                }
            }
            (Some(XNode::Element(p)), Some(XNode::Element(q))) => {
                if !same_tree(p, q) {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

/// Whether names, namespaces, `i:type`, `i:nil` and attributes match, leaving out the children.
pub(crate) fn same_header(a: &XElement, b: &XElement) -> bool {
    a.local == b.local
        && a.ns == b.ns
        && a.nil == b.nil
        && a.xsi_type == b.xsi_type
        && a.attrs.len() == b.attrs.len()
        && a.attrs.iter().all(|x| b.attrs.iter().any(|y| y == x))
}

/// The first thing `original` holds that `saved` lost or changed, with every member `saved` added noted in
/// `added`. This is `first_difference` for an older file, which real Vixen fills in when it upgrades one.
///
/// Only new member names count as additions. An extra item of a list that already has items of that name is a
/// change, and so is anything else that differs. The caller decides whether the additions are acceptable.
pub fn first_loss(original: &XElement, saved: &XElement, path: &str, added: &mut Vec<String>) -> Option<String> {
    let path = format!("{path}/{}", original.local);
    if !same_header(original, saved) {
        return Some(format!(
            "{path}: {{{}}}{} nil={} type={:?} attrs={:?} became {{{}}}{} nil={} type={:?} attrs={:?}",
            original.ns,
            original.local,
            original.nil,
            original.xsi_type,
            original.attrs,
            saved.ns,
            saved.local,
            saved.nil,
            saved.xsi_type,
            saved.attrs
        ));
    }
    let text = |e: &XElement| e.text().trim().to_string();
    if text(original) != text(saved) {
        return Some(format!("{path}: text {:?} became {:?}", text(original), text(saved)));
    }
    let originals: Vec<&XElement> = original.elements().collect();
    let saveds: Vec<&XElement> = saved.elements().collect();
    let mut s = 0;
    for o in &originals {
        let start = s;
        while s < saveds.len() && saveds[s].local != o.local {
            s += 1;
        }
        let Some(found) = saveds.get(s) else {
            return Some(format!("{path}: lost or moved member {}", o.local));
        };
        for extra in &saveds[start..s] {
            if originals.iter().any(|x| x.local == extra.local) {
                return Some(format!("{path}: member {} moved", extra.local));
            }
            added.push(format!("{path}/{}", extra.local));
        }
        if let Some(d) = first_loss(o, found, &path, added) {
            return Some(d);
        }
        s += 1;
    }
    for extra in &saveds[s..] {
        if originals.iter().any(|x| x.local == extra.local) {
            return Some(format!("{path}: an extra {} was added to a list", extra.local));
        }
        added.push(format!("{path}/{}", extra.local));
    }
    None
}

pub(crate) fn is_significant(n: &XNode) -> bool {
    match n {
        XNode::Text(t) => !t.trim().is_empty(),
        XNode::Element(_) => true,
    }
}

/// Whether the element has any text apart from indentation.
pub(crate) fn has_significant_text(e: &XElement) -> bool {
    e.children.iter().any(|n| matches!(n, XNode::Text(t) if !t.trim().is_empty()))
}

struct Writer<'a> {
    out: String,
    newline: &'static str,
    indent: bool,
    /// (prefix, namespace) in force, innermost last.
    scope: Vec<(String, &'a str)>,
    style: DocStyle,
}

impl<'a> Writer<'a> {
    fn lookup(&self, ns: &str) -> Option<usize> {
        self.scope.iter().rposition(|(_, n)| *n == ns)
    }

    /// A new prefix. A sequence's writer numbers them by depth. An object pasted into the module store was
    /// written on its own, by a writer that takes the next free letter.
    fn new_prefix(&self, depth: usize, number: &mut u32, in_object: bool) -> String {
        if in_object {
            for c in 'a'..='z' {
                let mut buffer = [0u8; 4];
                let letter: &str = c.encode_utf8(&mut buffer);
                if !self.scope.iter().any(|(p, _)| p == letter) {
                    return letter.to_string();
                }
            }
        }
        *number += 1;
        format!("d{depth}p{number}")
    }

    /// `depth` is the depth from the root. `in_object` says the element is inside a pasted-in object.
    fn element(&mut self, e: &'a XElement, depth: usize, in_object: bool, default_ns: &'a str) {
        let mark = self.scope.len();
        let is_root = depth == 0;
        let is_object = self.style.object_depth == Some(depth);
        let in_object = in_object || is_object;

        if is_root {
            for (p, n) in self.style.root_prefixes {
                self.scope.push(((*p).to_string(), n));
            }
        }
        if is_object && self.lookup(XSI_NS).is_none() {
            self.scope.push(("i".to_string(), XSI_NS));
        }

        // The element's own name: a prefix in force, or a switch of the default namespace.
        let mut element_prefix: Option<usize> = None;
        let mut new_default = default_ns;
        let mut declare_default = false;
        if e.ns != default_ns {
            match if e.ns.is_empty() { None } else { self.lookup(&e.ns) } {
                Some(i) => element_prefix = Some(i),
                None => {
                    new_default = &e.ns;
                    declare_default = true;
                }
            }
        }

        let mut number = 0u32;

        // `i:type` and `i:nil` need the instance prefix; `i:type` also needs its type's namespace.
        if (e.nil || e.xsi_type.is_some()) && self.lookup(XSI_NS).is_none() {
            self.scope.push(("i".to_string(), XSI_NS));
        }
        if let Some((tns, _)) = &e.xsi_type {
            if !tns.is_empty() && self.lookup(tns).is_none() {
                let prefix = self.new_prefix(depth, &mut number, in_object);
                self.scope.push((prefix, tns));
            }
        }
        for a in &e.attrs {
            if !a.ns.is_empty() && a.ns != XML_NS && self.lookup(&a.ns).is_none() {
                let prefix = self.new_prefix(depth, &mut number, in_object);
                self.scope.push((prefix, &a.ns));
            }
        }

        // The namespace of the element's type, for its members to use. The last member belongs to the most
        // derived class, which is the type's own contract. The parent of a pasted-in object declares nothing,
        // because the object switches the default namespace itself.
        let parent_of_object = self.style.object_depth == Some(depth + 1);
        if !parent_of_object {
            if let Some(last) = e.elements().last() {
                if !last.ns.is_empty() && last.ns != new_default && self.lookup(&last.ns).is_none() {
                    let prefix = self.new_prefix(depth, &mut number, in_object);
                    self.scope.push((prefix, &last.ns));
                }
            }
        }

        // Start tag. Real files write plain attributes first on the root and on objects.
        self.push_indent(depth);
        self.out.push('<');
        self.push_prefix(element_prefix);
        self.out.push_str(&e.local);
        if is_root {
            self.write_attrs(e);
            self.write_decls(mark);
            if declare_default {
                self.write_default_decl(new_default);
            }
            self.write_type_and_nil(e);
        } else if is_object {
            self.write_attrs(e);
            if declare_default {
                self.write_default_decl(new_default);
            }
            self.write_decls(mark);
            self.write_type_and_nil(e);
        } else if in_object {
            self.write_type_and_nil(e);
            self.write_attrs(e);
            if declare_default {
                self.write_default_decl(new_default);
            }
            self.write_decls(mark);
        } else {
            self.write_decls(mark);
            if declare_default {
                self.write_default_decl(new_default);
            }
            self.write_type_and_nil(e);
            self.write_attrs(e);
        }

        // Content.
        let has_elements = e.children.iter().any(|n| matches!(n, XNode::Element(_)));
        if has_elements {
            self.out.push('>');
            for n in &e.children {
                match n {
                    XNode::Element(c) => {
                        self.push_newline();
                        self.element(c, depth + 1, in_object, new_default);
                    }
                    XNode::Text(t) => {
                        if !t.trim().is_empty() {
                            self.push_newline();
                            self.write_text(t);
                        }
                    }
                }
            }
            self.push_newline();
            self.push_indent(depth);
            self.close_tag(e, element_prefix);
        } else if e.children.is_empty() {
            self.out.push_str(" />");
        } else {
            self.out.push('>');
            for n in &e.children {
                if let XNode::Text(t) = n {
                    self.write_text(t);
                }
            }
            self.close_tag(e, element_prefix);
        }
        self.scope.truncate(mark);
    }

    fn push_prefix(&mut self, prefix: Option<usize>) {
        if let Some(i) = prefix {
            self.out.push_str(&self.scope[i].0);
            self.out.push(':');
        }
    }

    fn write_decls(&mut self, from: usize) {
        for i in from..self.scope.len() {
            self.out.push_str(" xmlns:");
            self.out.push_str(&self.scope[i].0);
            self.out.push_str("=\"");
            push_escaped_attr(&mut self.out, self.scope[i].1);
            self.out.push('"');
        }
    }

    fn close_tag(&mut self, e: &XElement, prefix: Option<usize>) {
        self.out.push_str("</");
        self.push_prefix(prefix);
        self.out.push_str(&e.local);
        self.out.push('>');
    }

    fn push_newline(&mut self) {
        if self.indent {
            self.out.push_str(self.newline);
        }
    }

    fn push_indent(&mut self, depth: usize) {
        if self.indent {
            for _ in 0..depth {
                self.out.push_str("  ");
            }
        }
    }

    fn write_default_decl(&mut self, ns: &str) {
        self.out.push_str(" xmlns=\"");
        push_escaped_attr(&mut self.out, ns);
        self.out.push('"');
    }

    fn write_type_and_nil(&mut self, e: &XElement) {
        if e.xsi_type.is_none() && !e.nil {
            return;
        }
        // Declared by `element` before the start tag is written.
        let Some(i) = self.lookup(XSI_NS) else { return };
        if let Some((tns, tlocal)) = &e.xsi_type {
            self.out.push(' ');
            self.out.push_str(&self.scope[i].0);
            self.out.push_str(":type=\"");
            if !tns.is_empty() {
                let t = self.lookup(tns);
                self.push_prefix(t);
            }
            push_escaped_attr(&mut self.out, tlocal);
            self.out.push('"');
        }
        if e.nil {
            self.out.push(' ');
            self.out.push_str(&self.scope[i].0);
            self.out.push_str(":nil=\"true\"");
        }
    }

    fn write_attrs(&mut self, e: &XElement) {
        for a in &e.attrs {
            self.out.push(' ');
            if a.ns == XML_NS {
                // `xml:` is bound by the XML specification and may not be declared to anything else.
                self.out.push_str("xml:");
            } else if !a.ns.is_empty() {
                let p = self.lookup(&a.ns);
                self.push_prefix(p);
            }
            self.out.push_str(&a.local);
            self.out.push_str("=\"");
            push_escaped_attr(&mut self.out, &a.value);
            self.out.push('"');
        }
    }

    fn write_text(&mut self, t: &str) {
        for c in t.chars() {
            match c {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                // A parser turns a bare CR into LF, so the value only survives as a reference.
                '\r' => self.out.push_str("&#xD;"),
                '\n' => self.out.push_str(self.newline),
                _ => self.out.push(c),
            }
        }
    }
}

fn push_escaped_attr(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            '\t' => out.push_str("&#x9;"),
            _ => out.push(c),
        }
    }
}

/// Escapes text for use as the content of `ParamValue::Raw`.
pub(crate) fn escape_text(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_xml::{contract_ns, ARRAYS_NS, XSD_NS};

    const SEQUENCE_STYLE: DocStyle = DocStyle { root_prefixes: &[("a", XSD_NS), ("i", XSI_NS)], object_depth: None };

    fn timed() -> String {
        contract_ns("VixenModules.Sequence.Timed")
    }

    #[test]
    fn matches_the_prefix_placement_of_a_real_sequence() {
        let wipe = contract_ns("VixenModules.Effect.Wipe");
        let base = contract_ns("VixenModules.Effect.Effect");
        let mut any = XElement::new(ARRAYS_NS, "anyType");
        any.xsi_type = Some((wipe.clone(), "WipeData".to_string()));
        any.push(XElement::with_text("", "ModuleInstanceId", "x"));
        any.push(XElement::with_text(&base, "TargetPositioning", "Strings"));
        any.push(XElement::with_text(&wipe, "PassCount", "1"));
        let mut models = XElement::new("", "_dataModels");
        models.push(any);
        let mut root = XElement::new(&timed(), "TimedSequenceData");
        root.set_attr("version", "9");
        root.push(models);
        root.push(XElement::nil(&timed(), "DefaultPlaybackEndTime"));

        let text = write_document(&root, SEQUENCE_STYLE);
        let expected = [
            "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>",
            "<TimedSequenceData version=\"9\" xmlns:a=\"http://www.w3.org/2001/XMLSchema\" xmlns:i=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns=\"http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed\">",
            "  <_dataModels xmlns:d1p1=\"http://schemas.microsoft.com/2003/10/Serialization/Arrays\" xmlns=\"\">",
            "    <d1p1:anyType xmlns:d2p1=\"http://schemas.datacontract.org/2004/07/VixenModules.Effect.Wipe\" i:type=\"d2p1:WipeData\">",
            "      <ModuleInstanceId>x</ModuleInstanceId>",
            "      <TargetPositioning xmlns=\"http://schemas.datacontract.org/2004/07/VixenModules.Effect.Effect\">Strings</TargetPositioning>",
            "      <d2p1:PassCount>1</d2p1:PassCount>",
            "    </d1p1:anyType>",
            "  </_dataModels>",
            "  <DefaultPlaybackEndTime i:nil=\"true\" />",
            "</TimedSequenceData>",
        ]
        .join("\r\n");
        assert_eq!(text, expected);
        assert!(same_tree(&root, &XElement::parse(&text).unwrap()));
    }

    #[test]
    fn objects_in_the_module_store_declare_their_own_instance_prefix() {
        let color = contract_ns("VixenModules.Property.Location");
        let mut object = XElement::new(&color, "LocationData");
        object.push(XElement::with_text("", "ModuleInstanceId", "x"));
        object.push(XElement::with_text(&color, "X", "1"));
        let mut module = XElement::new("", "Module");
        module.set_attr("moduleType", "t");
        module.push(object);
        let mut section = XElement::new("", "ModuleData");
        section.push(module);
        let mut root = XElement::new("", "ModuleStore");
        root.set_attr("version", "5");
        root.push(section);

        let text = write_document(&root, DocStyle { root_prefixes: &[], object_depth: Some(3) });
        assert!(text.contains(
            "<LocationData xmlns=\"http://schemas.datacontract.org/2004/07/VixenModules.Property.Location\" xmlns:i=\"http://www.w3.org/2001/XMLSchema-instance\">"
        ), "{text}");
        assert!(text.contains("<ModuleInstanceId xmlns=\"\">x</ModuleInstanceId>"), "{text}");
        assert!(same_tree(&root, &XElement::parse(&text).unwrap()));
    }

    #[test]
    fn fragments_stand_alone_and_keep_types_and_nil() {
        let ns = contract_ns("VixenModules.App.Curves");
        let mut e = XElement::new(&ns, "Curve");
        let mut schema = XElement::with_text("", "schema", "11");
        schema.xsi_type = Some((XSD_NS.to_string(), "int".to_string()));
        e.push(schema);
        e.push(XElement::nil("", "Tag"));
        let text = write_fragment(&e);
        assert!(same_tree(&e, &parse_fragment(&text).unwrap()), "{text}");
    }

    #[test]
    fn empty_text_writes_open_and_close_tags_like_a_serialized_empty_string() {
        let e = XElement::with_text("", "Text", "");
        assert_eq!(write_fragment(&e), "<Text></Text>");
        assert_eq!(write_fragment(&XElement::new("", "Text")), "<Text />");
    }

    #[test]
    fn whitespace_and_carriage_returns_survive() {
        let e = XElement::with_text("", "Name", "  a\rb  ");
        let back = parse_fragment(&write_fragment(&e)).unwrap();
        assert_eq!(back.text(), "  a\rb  ");
        let blank = XElement::with_text("", "Name", "   ");
        assert_eq!(parse_fragment(&write_fragment(&blank)).unwrap().text(), "   ");
    }

    #[test]
    fn attributes_escape_what_a_parser_would_normalise() {
        let mut e = XElement::new("", "a");
        e.set_attr("b", "x\n\t\"<&");
        let back = parse_fragment(&write_fragment(&e)).unwrap();
        assert_eq!(back.attr("b"), Some("x\n\t\"<&"));
    }

    #[test]
    fn the_xml_prefix_is_never_redeclared() {
        let e = XElement::parse(r#"<a xml:space="preserve"> b </a>"#).unwrap();
        let text = write_fragment(&e);
        assert_eq!(text, r#"<a xml:space="preserve"> b </a>"#);
        assert!(same_tree(&e, &parse_fragment(&text).unwrap()));
    }

    #[test]
    fn same_tree_agrees_with_first_difference() {
        let a = XElement::parse("<a x='1'><b>t</b>\n<c/></a>").unwrap();
        let b = XElement::parse("<a x='1'>  <b> t </b><c></c></a>").unwrap();
        let c = XElement::parse("<a x='2'><b>t</b><c/></a>").unwrap();
        assert!(same_tree(&a, &b));
        assert_eq!(a.first_difference(&b, String::new()), None);
        assert!(!same_tree(&a, &c));
        assert!(a.first_difference(&c, String::new()).is_some());
    }

    #[test]
    fn first_loss_allows_new_members_and_nothing_else() {
        let original = XElement::parse("<o><a>1</a><c>3</c><i>1</i><i>2</i></o>").unwrap();
        let mut added = Vec::new();
        let saved = XElement::parse("<o><a>1</a><b>2</b><c>3</c><i>1</i><i>2</i><z/></o>").unwrap();
        assert_eq!(first_loss(&original, &saved, "", &mut added), None);
        assert_eq!(added, ["/o/b", "/o/z"]);

        let mut added = Vec::new();
        for (saved, why) in [
            ("<o><a>1</a><i>1</i><i>2</i></o>", "lost a member"),
            ("<o><a>9</a><c>3</c><i>1</i><i>2</i></o>", "changed text"),
            ("<o><c>3</c><a>1</a><i>1</i><i>2</i></o>", "moved a member"),
            ("<o><a>1</a><c>3</c><i>1</i><i>2</i><i>3</i></o>", "grew a list"),
            ("<o><a>1</a><c>3</c><i>1</i></o>", "shrank a list"),
            ("<o x=\"1\"><a>1</a><c>3</c><i>1</i><i>2</i></o>", "gained an attribute"),
        ] {
            let saved = XElement::parse(saved).unwrap();
            assert!(first_loss(&original, &saved, "", &mut added).is_some(), "{why}");
        }
    }

    #[test]
    fn decodes_utf16_with_a_byte_order_mark() {
        let text: Vec<u8> = "\u{feff}<a>é</a>".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(XElement::parse(&decode_text(&text)).unwrap().text(), "é");
    }
}
