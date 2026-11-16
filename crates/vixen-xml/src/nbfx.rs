//! MC-NBFX, .NET's binary XML: what `XmlDictionaryWriter.CreateBinaryWriter` writes. Vixen's effect defaults store and its export files use it, with no dictionary passed in, so strings are inline. A file
//! that refers to a dictionary string can't be read here and comes back as an error, which the store treats as a corrupt file.

use crate::{XAttr, XElement, XNode, XmlError, XSI_NS};
use std::collections::HashMap;

fn err(message: &str) -> XmlError {
    XmlError(format!("binary XML: {message}"))
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], XmlError> {
        let end = self.at.checked_add(n).ok_or_else(|| err("length overflows"))?;
        let s = self.bytes.get(self.at..end).ok_or_else(|| err("ends early"))?;
        self.at = end;
        Ok(s)
    }

    fn byte(&mut self) -> Result<u8, XmlError> {
        Ok(self.take(1)?[0])
    }

    /// MB32: seven bits at a time, least significant first.
    fn mb32(&mut self) -> Result<usize, XmlError> {
        let mut value = 0usize;
        for shift in (0..35).step_by(7) {
            let b = self.byte()?;
            value |= ((b & 0x7F) as usize) << shift;
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(err("a length is too long"))
    }

    fn string(&mut self) -> Result<String, XmlError> {
        let n = self.mb32()?;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| err("text isn't UTF-8"))
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], XmlError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// Runs out at the end of the bytes.
    fn at_end(&self) -> bool {
        self.at >= self.bytes.len()
    }
}

/// `xs:dateTime` for a .NET `DateTime`'s ticks and kind, which is what `DateTimeText` holds.
fn date_time_text(raw: u64) -> String {
    let ticks = (raw & 0x3FFF_FFFF_FFFF_FFFF) as i64;
    let kind = raw >> 62;
    let total_seconds = ticks / 10_000_000;
    let fraction = ticks % 10_000_000;
    let days = total_seconds / 86_400;
    let (year, month, day) = civil_from_days(days);
    let s = total_seconds % 86_400;
    let mut text = format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60);
    if fraction != 0 {
        let digits = format!("{fraction:07}");
        text.push('.');
        text.push_str(digits.trim_end_matches('0'));
    }
    if kind == 1 {
        text.push('Z');
    }
    text
}

/// Days since 0001-01-01 to a date.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    // Days since 0000-03-01, the era algorithm's epoch.
    let z = days + 306;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(BASE64[(n >> 18) as usize & 63] as char);
        out.push(BASE64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { BASE64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { BASE64[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut value = 0u32;
    let mut bits = 0;
    for b in text.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=') {
        let d = BASE64.iter().position(|x| *x == b)? as u32;
        value = (value << 6) | d;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((value >> bits) as u8);
        }
    }
    Some(out)
}

fn guid_text(b: [u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[3], b[2], b[1], b[0], b[5], b[4], b[7], b[6], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

/// The text of a text record, and whether it also ends the element. `None` when the record isn't text.
fn read_text(r: &mut Reader, record: u8) -> Result<Option<(String, bool)>, XmlError> {
    let ends = record & 1 == 1;
    let text = match record & 0xFE {
        0x80 => "0".to_string(),
        0x82 => "1".to_string(),
        0x84 => "false".to_string(),
        0x86 => "true".to_string(),
        0x88 => (r.byte()? as i8).to_string(),
        0x8A => i16::from_le_bytes(r.fixed()?).to_string(),
        0x8C => i32::from_le_bytes(r.fixed()?).to_string(),
        0x8E => i64::from_le_bytes(r.fixed()?).to_string(),
        0x90 => format!("{}", f32::from_le_bytes(r.fixed()?)),
        0x92 => format!("{}", f64::from_le_bytes(r.fixed()?)),
        0x94 => decimal_text(r.fixed()?),
        0x96 => date_time_text(u64::from_le_bytes(r.fixed()?)),
        0x98 => {
            let n = r.byte()? as usize;
            utf8(r.take(n)?)?
        }
        0x9A => {
            let n = u16::from_le_bytes(r.fixed()?) as usize;
            utf8(r.take(n)?)?
        }
        0x9C => {
            let n = u32::from_le_bytes(r.fixed()?) as usize;
            utf8(r.take(n)?)?
        }
        0x9E => {
            let n = r.byte()? as usize;
            base64_encode(r.take(n)?)
        }
        0xA0 => {
            let n = u16::from_le_bytes(r.fixed()?) as usize;
            base64_encode(r.take(n)?)
        }
        0xA2 => {
            let n = u32::from_le_bytes(r.fixed()?) as usize;
            base64_encode(r.take(n)?)
        }
        0xA8 => String::new(),
        0xAC => format!("urn:uuid:{}", guid_text(r.fixed()?)),
        0xAE => timespan_text(i64::from_le_bytes(r.fixed()?)),
        0xB0 => guid_text(r.fixed()?),
        0xB2 => u64::from_le_bytes(r.fixed()?).to_string(),
        0xB4 => if r.byte()? != 0 { "true".to_string() } else { "false".to_string() },
        0xB6 => {
            let n = r.byte()? as usize;
            utf16(r.take(n)?)
        }
        0xB8 => {
            let n = u16::from_le_bytes(r.fixed()?) as usize;
            utf16(r.take(n)?)
        }
        0xBA => {
            let n = u32::from_le_bytes(r.fixed()?) as usize;
            utf16(r.take(n)?)
        }
        0xAA | 0xBC => return Err(err("a dictionary string with no dictionary")),
        _ => return Ok(None),
    };
    Ok(Some((text, ends)))
}

fn utf8(b: &[u8]) -> Result<String, XmlError> {
    String::from_utf8(b.to_vec()).map_err(|_| err("text isn't UTF-8"))
}

fn utf16(b: &[u8]) -> String {
    let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&units)
}

fn decimal_text(b: [u8; 16]) -> String {
    // flags: 2 bytes reserved, scale, sign; then hi32, lo64.
    let scale = b[2] as usize;
    let negative = b[3] != 0;
    let hi = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as u128;
    let lo = u64::from_le_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]) as u128;
    let mut digits = ((hi << 64) | lo).to_string();
    if scale > 0 {
        while digits.len() <= scale {
            digits.insert(0, '0');
        }
        digits.insert(digits.len() - scale, '.');
    }
    if negative {
        digits.insert(0, '-');
    }
    digits
}

/// `XmlConvert.ToString(TimeSpan)`.
fn timespan_text(ticks: i64) -> String {
    let negative = ticks < 0;
    let t = ticks.unsigned_abs();
    let (days, rest) = (t / 864_000_000_000, t % 864_000_000_000);
    let (hours, rest) = (rest / 36_000_000_000, rest % 36_000_000_000);
    let (minutes, rest) = (rest / 600_000_000, rest % 600_000_000);
    let (seconds, fraction) = (rest / 10_000_000, rest % 10_000_000);
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push('P');
    if days > 0 {
        out.push_str(&format!("{days}D"));
    }
    if hours + minutes + seconds + fraction > 0 || days == 0 {
        out.push('T');
        if hours > 0 {
            out.push_str(&format!("{hours}H"));
        }
        if minutes > 0 {
            out.push_str(&format!("{minutes}M"));
        }
        if fraction > 0 {
            let digits = format!("{fraction:07}");
            out.push_str(&format!("{seconds}.{}S", digits.trim_end_matches('0')));
        } else if seconds > 0 || (hours == 0 && minutes == 0 && days == 0) {
            out.push_str(&format!("{seconds}S"));
        }
    }
    out
}

/// An element being read: its prefixes and attributes aren't resolved until its start is complete.
struct Open {
    prefix: String,
    local: String,
    attrs: Vec<(String, String, String)>,
    xmlns: Vec<(String, String)>,
    children: Vec<XNode>,
}

impl Open {
    fn ns_with(&self, scopes: &[HashMap<String, String>]) -> String {
        self.xmlns.iter().rev().find(|(p, _)| *p == self.prefix).map_or_else(|| lookup(scopes, &self.prefix), |(_, u)| u.clone())
    }
}

fn prefix_of(letter: u8) -> String {
    char::from(b'a' + letter).to_string()
}

fn lookup(scopes: &[HashMap<String, String>], prefix: &str) -> String {
    scopes.iter().rev().find_map(|s| s.get(prefix)).cloned().unwrap_or_default()
}

/// Parses a whole document into its root element.
pub fn read(bytes: &[u8]) -> Result<XElement, XmlError> {
    let mut r = Reader { bytes, at: 0 };
    let mut stack: Vec<Open> = Vec::new();
    let mut scopes: Vec<HashMap<String, String>> = Vec::new();
    let mut root: Option<XElement> = None;

    fn finish(open: Open, scopes: &mut Vec<HashMap<String, String>>) -> XElement {
        scopes.pop();
        let ns = open.ns_with(scopes);
        let mut e = XElement::new(&ns, &open.local);
        e.children = open.children;
        for (prefix, local, value) in open.attrs {
            let attr_ns = if prefix.is_empty() { String::new() } else { open.xmlns.iter().rev().find(|(p, _)| *p == prefix).map_or_else(|| lookup(scopes, &prefix), |(_, u)| u.clone()) };
            if attr_ns == XSI_NS && local == "nil" {
                e.nil = value == "true" || value == "1";
            } else if attr_ns == XSI_NS && local == "type" {
                let (p, name) = value.split_once(':').unwrap_or(("", value.as_str()));
                let type_ns = open.xmlns.iter().rev().find(|(q, _)| q == p).map_or_else(|| lookup(scopes, p), |(_, u)| u.clone());
                e.xsi_type = Some((type_ns, name.to_string()));
            } else {
                e.attrs.push(XAttr { ns: attr_ns, local, value });
            }
        }
        e
    }

    while !r.at_end() {
        let record = r.byte()?;
        match record {
            0x01 => {
                let Some(open) = stack.pop() else { return Err(err("an end with no element")) };
                let e = finish(open, &mut scopes);
                match stack.last_mut() {
                    Some(parent) => parent.children.push(XNode::Element(e)),
                    None => root = Some(e),
                }
            }
            0x02 => {
                r.string()?;
            }
            0x04 | 0x05 => {
                let prefix = if record == 0x05 { r.string()? } else { String::new() };
                let local = r.string()?;
                let value = attr_text(&mut r)?;
                stack.last_mut().ok_or_else(|| err("an attribute outside an element"))?.attrs.push((prefix, local, value));
            }
            0x06 | 0x07 | 0x0A | 0x0B | 0x0C..=0x25 | 0x42 | 0x43 | 0x44..=0x5D => {
                return Err(err("a dictionary string with no dictionary"));
            }
            0x08 | 0x09 => {
                let prefix = if record == 0x09 { r.string()? } else { String::new() };
                let uri = r.string()?;
                let open = stack.last_mut().ok_or_else(|| err("a namespace outside an element"))?;
                open.xmlns.push((prefix.clone(), uri.clone()));
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(prefix, uri);
                }
            }
            0x26..=0x3F => {
                let prefix = prefix_of(record - 0x26);
                let local = r.string()?;
                let value = attr_text(&mut r)?;
                stack.last_mut().ok_or_else(|| err("an attribute outside an element"))?.attrs.push((prefix, local, value));
            }
            0x40 | 0x41 | 0x5E..=0x77 => {
                let prefix = match record {
                    0x40 => String::new(),
                    0x41 => r.string()?,
                    _ => prefix_of(record - 0x5E),
                };
                let local = r.string()?;
                scopes.push(HashMap::new());
                stack.push(Open { prefix, local, attrs: Vec::new(), xmlns: Vec::new(), children: Vec::new() });
            }
            _ => {
                let Some((text, ends)) = read_text(&mut r, record)? else {
                    return Err(err(&format!("record {record:#x} isn't handled")));
                };
                let open = stack.last_mut().ok_or_else(|| err("text outside an element"))?;
                open.children.push(XNode::Text(text));
                if ends {
                    let Some(open) = stack.pop() else { return Err(err("an end with no element")) };
                    let e = finish(open, &mut scopes);
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(XNode::Element(e)),
                        None => root = Some(e),
                    }
                }
            }
        }
    }
    if !stack.is_empty() {
        return Err(err("ends inside an element"));
    }
    root.ok_or_else(|| err("no root element"))
}

fn attr_text(r: &mut Reader) -> Result<String, XmlError> {
    let record = r.byte()?;
    let Some((text, _)) = read_text(r, record)? else { return Err(err("an attribute's value isn't text")) };
    Ok(text)
}

fn write_mb32(out: &mut Vec<u8>, mut v: usize) {
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn write_string(out: &mut Vec<u8>, s: &str) {
    write_mb32(out, s.len());
    out.extend_from_slice(s.as_bytes());
}

/// `Chars8Text` and its longer forms.
fn write_chars(out: &mut Vec<u8>, record: u8, s: &str) {
    let b = s.as_bytes();
    if b.len() <= u8::MAX as usize {
        out.push(record);
        out.push(b.len() as u8);
    } else if b.len() <= u16::MAX as usize {
        out.push(record + 2);
        out.extend_from_slice(&(b.len() as u16).to_le_bytes());
    } else {
        out.push(record + 4);
        out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(b);
}

/// Prefixes in force as a document is written.
#[derive(Default)]
struct Writer {
    out: Vec<u8>,
    scopes: Vec<Vec<(String, String)>>,
    counter: usize,
}

impl Writer {
    fn prefix_for(&self, ns: &str) -> Option<String> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|(_, u)| u == ns).map(|(p, _)| p.clone())
    }

    fn element(&mut self, e: &XElement) {
        self.scopes.push(Vec::new());
        let mut declare: Vec<(String, String)> = Vec::new();
        // The element's own namespace is the default, as DataContractSerializer writes it. An empty one needs nothing.
        let default_now = self.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|(p, _)| p.is_empty()).map_or("", |(_, u)| u.as_str()).to_string();
        if e.ns != default_now {
            declare.push((String::new(), e.ns.clone()));
        }
        // `i:` for nil and type.
        if (e.nil || e.xsi_type.is_some()) && self.prefix_for(XSI_NS).is_none() {
            declare.push(("i".to_string(), XSI_NS.to_string()));
        }
        let mut type_prefix = None;
        if let Some((ns, _)) = &e.xsi_type {
            let found = self.prefix_for(ns).filter(|p| !p.is_empty()).or_else(|| declare.iter().find(|(p, u)| u == ns && !p.is_empty()).map(|(p, _)| p.clone()));
            type_prefix = Some(found.unwrap_or_else(|| {
                self.counter += 1;
                let p = format!("d{}p1", self.counter);
                declare.push((p.clone(), ns.clone()));
                p
            }));
        }
        // ShortElement.
        self.out.push(0x40);
        write_string(&mut self.out, &e.local);
        for (p, u) in &declare {
            if p.is_empty() {
                self.out.push(0x08);
            } else {
                self.out.push(0x09);
                write_string(&mut self.out, p);
            }
            write_string(&mut self.out, u);
            if let Some(scope) = self.scopes.last_mut() {
                scope.push((p.clone(), u.clone()));
            }
        }
        if let Some((_, name)) = &e.xsi_type {
            self.attribute("i", "type", &format!("{}:{name}", type_prefix.unwrap_or_default()));
        }
        if e.nil {
            self.attribute("i", "nil", "true");
        }
        for a in &e.attrs {
            let prefix = if a.ns.is_empty() { String::new() } else { self.prefix_for(&a.ns).unwrap_or_default() };
            self.attribute(&prefix, &a.local, &a.value);
        }
        for child in &e.children {
            match child {
                XNode::Element(c) => self.element(c),
                XNode::Text(t) => write_chars(&mut self.out, 0x98, t),
            }
        }
        self.out.push(0x01);
        self.scopes.pop();
    }

    fn attribute(&mut self, prefix: &str, local: &str, value: &str) {
        if prefix.is_empty() {
            self.out.push(0x04);
        } else {
            self.out.push(0x05);
            write_string(&mut self.out, prefix);
        }
        write_string(&mut self.out, local);
        write_chars(&mut self.out, 0x98, value);
    }
}

/// Writes a document. Text goes as text, which a binary reader converts as it reads: it needn't be the typed records `DataContractSerializer` picks.
pub fn write(root: &XElement) -> Vec<u8> {
    let mut w = Writer::default();
    w.element(root);
    w.out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_spec_style_document() {
        // <doc xmlns="urn:a"><v>7</v><t>hi</t><i:n xmlns:i="..." i:nil="true"/></doc>, by hand.
        let mut b = Vec::new();
        b.push(0x40);
        write_string(&mut b, "doc");
        b.push(0x08);
        write_string(&mut b, "urn:a");
        b.push(0x40);
        write_string(&mut b, "v");
        b.push(0x89); // Int8Text with end element.
        b.push(7);
        b.push(0x40);
        write_string(&mut b, "t");
        b.push(0x99); // Chars8Text with end element.
        b.push(2);
        b.extend_from_slice(b"hi");
        b.push(0x40);
        write_string(&mut b, "n");
        b.push(0x09);
        write_string(&mut b, "i");
        write_string(&mut b, XSI_NS);
        b.push(0x05);
        write_string(&mut b, "i");
        write_string(&mut b, "nil");
        b.push(0x86);
        b.push(0x01);
        b.push(0x01);
        let e = read(&b).unwrap();
        assert_eq!(e.local, "doc");
        assert_eq!(e.ns, "urn:a");
        assert_eq!(e.child_text("v").as_deref(), Some("7"));
        assert_eq!(e.child_text("t").as_deref(), Some("hi"));
        assert_eq!(e.child("v").unwrap().ns, "urn:a");
        assert!(e.child("n").unwrap().nil);
    }

    #[test]
    fn typed_records_read_as_text() {
        let mut r = Reader { bytes: &0x0102_0304i32.to_le_bytes(), at: 0 };
        assert_eq!(read_text(&mut r, 0x8C).unwrap(), Some(("16909060".to_string(), false)));
        let guid = [0x33, 0x22, 0x11, 0x00, 0x55, 0x44, 0x77, 0x66, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let mut r = Reader { bytes: &guid, at: 0 };
        assert_eq!(read_text(&mut r, 0xB1).unwrap(), Some(("00112233-4455-6677-8899-aabbccddeeff".to_string(), true)));
        // 2026-01-02T03:04:05Z in .NET ticks, kind Utc.
        let ticks: u64 = 639029198450000000 | (1 << 62);
        let bytes = ticks.to_le_bytes();
        let mut r = Reader { bytes: &bytes, at: 0 };
        assert_eq!(read_text(&mut r, 0x96).unwrap(), Some(("2026-01-02T03:04:05Z".to_string(), false)));
        assert_eq!(timespan_text(10_000_000), "PT1S");
        assert_eq!(timespan_text(0), "PT0S");
    }

    #[test]
    fn dictionary_strings_are_an_error() {
        assert!(read(&[0x42, 0x02]).is_err());
        assert!(read(&[0x40]).is_err());
        assert!(read(&[]).is_err());
    }

    #[test]
    fn documents_round_trip() {
        let mut root = XElement::new("urn:a", "Store");
        root.push(XElement::with_text("urn:a", "Version", "1"));
        let mut typed = XElement::new("urn:a", "Item");
        typed.xsi_type = Some(("urn:b".to_string(), "Thing".to_string()));
        typed.push(XElement::with_text("urn:b", "Name", "x & y < z"));
        typed.push(XElement::nil("urn:a", "Missing"));
        typed.push(XElement::with_text("", "Bare", "t"));
        root.push(typed);
        root.push(XElement::with_text("urn:a", "Long", "L".repeat(70_000)));
        let back = read(&write(&root)).unwrap();
        assert_eq!(back, root);
    }

    #[test]
    fn base64_round_trip() {
        for data in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            assert_eq!(base64_decode(&base64_encode(data)).unwrap(), data);
        }
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
