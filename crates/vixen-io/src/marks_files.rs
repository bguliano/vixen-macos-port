//! The files the Mark Collections panel imports and exports: `MarkImportExportService`, `PangolinBeyondMarkParser` and
//! `PangolinBeyondMarkCollectionFactory`. Spec: `docs/spec/marks-layers-analysis.md`, F28 to F40 and F116.
//!
//! Every reader returns candidates built from `MarkCollection::default()`, which matches upstream's constructor, and never panics: where
//! upstream throws, these return `MarkFileError`, which the caller turns into that importer's own alert. Numbers parse with the invariant
//! culture (GUARD: upstream uses the machine's).

use crate::dcs::{child_bool, child_i32, child_string};
use crate::sequence::mark_collection_from_xml;
use crate::values::{color_from_xml, DotNetColor};
use vixen_core::marks::{Mark, MarkCollection, MarkCollectionType, MarkDecorator};
use vixen_core::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_MIN, TICKS_PER_MS, TICKS_PER_SEC};
use vixen_core::{Color, TimeSpan};
use vixen_xml::{XElement, ARRAYS_NS, XSI_NS};

/// Why a file couldn't be read or written. Only logged upstream: each importer shows its own fixed message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkFileError(pub String);

impl std::fmt::Display for MarkFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MarkFileError {}

fn fail<T>(message: impl Into<String>) -> Result<T, MarkFileError> {
    Err(MarkFileError(message.into()))
}

/// The namespace `typeof(IMarkCollection)` gives the `.v3m` root.
pub const V3M_NS: &str = "http://schemas.datacontract.org/2004/07/Vixen.Marks.IMarkCollection";
/// The `.v3m` written before `LabeledMarkCollections`.
pub const LEGACY_V3M_NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed";
const NS_MARKS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.Marks";
const NS_DRAWING: &str = "http://schemas.datacontract.org/2004/07/System.Drawing";

const MIN_MARK_DURATION: TimeSpan = TimeSpan::from_ms_i(25);

// ---------------------------------------------------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------------------------------------------------

/// How `File.ReadAllText` and `StreamReader` read every import here (F116): a UTF-8, UTF-16 or UTF-32 byte order mark is detected and
/// stripped; otherwise UTF-8, with bad bytes becoming U+FFFD rather than failing.
pub fn read_import_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE, 0x00, 0x00]) {
        return decode_utf32(rest, false);
    }
    if let Some(rest) = bytes.strip_prefix(&[0x00, 0x00, 0xFE, 0xFF]) {
        return decode_utf32(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, false);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, true);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn decode_utf16(bytes: &[u8], big_endian: bool) -> String {
    let units = bytes.chunks(2).map(|c| match c {
        [a, b] if big_endian => u16::from_be_bytes([*a, *b]),
        [a, b] => u16::from_le_bytes([*a, *b]),
        _ => 0xFFFD,
    });
    char::decode_utf16(units).map(|r| r.unwrap_or('\u{FFFD}')).collect()
}

fn decode_utf32(bytes: &[u8], big_endian: bool) -> String {
    bytes
        .chunks(4)
        .map(|c| match c {
            [a, b, x, y] => {
                let v = if big_endian { u32::from_be_bytes([*a, *b, *x, *y]) } else { u32::from_le_bytes([*a, *b, *x, *y]) };
                char::from_u32(v).unwrap_or('\u{FFFD}')
            }
            _ => '\u{FFFD}',
        })
        .collect()
}

/// `Convert.ToDouble(string)` under the invariant culture: `NumberStyles.Float | AllowThousands`. Surrounding white space, a sign, a
/// decimal point, an exponent and thousands separators after the first digit are all accepted, so `1,234.5` is 1234.5. `Infinity` and
/// `NaN` parse upstream but then fail in `TimeSpan.FromSeconds`, so they are refused here with the same outcome.
pub fn parse_invariant_double(text: &str) -> Option<f64> {
    let t = text.trim_matches(|c: char| matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' '));
    let bytes = t.as_bytes();
    let mut i = 0;
    let mut clean = String::with_capacity(t.len());
    if let Some(&s) = bytes.first() {
        if s == b'+' || s == b'-' {
            clean.push(s as char);
            i = 1;
        }
    }
    let mut digits = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'0'..=b'9' => {
                clean.push(bytes[i] as char);
                digits += 1;
            }
            b',' if digits > 0 => {}
            _ => break,
        }
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        clean.push('.');
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            clean.push(bytes[i] as char);
            digits += 1;
            i += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        clean.push('e');
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            clean.push(bytes[i] as char);
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            clean.push(bytes[i] as char);
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    if i != bytes.len() {
        return None;
    }
    clean.parse::<f64>().ok()
}

/// `TimeSpan.FromSeconds`, which truncates to a whole tick and throws on NaN or overflow.
fn time_from_secs(seconds: f64) -> Option<TimeSpan> {
    checked_ticks(seconds * TICKS_PER_SEC as f64)
}

/// `TimeSpan.FromMilliseconds`, likewise.
fn time_from_ms(ms: f64) -> Option<TimeSpan> {
    checked_ticks(ms * TICKS_PER_MS as f64)
}

fn checked_ticks(ticks: f64) -> Option<TimeSpan> {
    if ticks.is_nan() || ticks > i64::MAX as f64 || ticks < i64::MIN as f64 {
        return None;
    }
    Some(TimeSpan::from_ticks(ticks as i64))
}

/// The `Mark.Duration` setter: nothing when the value is already the duration, otherwise 25 ms at least.
pub fn set_mark_duration(mark: &mut Mark, duration: TimeSpan) {
    if duration != mark.duration {
        mark.duration = duration.max(MIN_MARK_DURATION);
    }
}

/// `new Mark(start) { Duration = d, Text = text }`.
fn mark_with(start: TimeSpan, duration: Option<TimeSpan>, text: String) -> Mark {
    let mut mark = Mark::new(start);
    if let Some(d) = duration {
        set_mark_duration(&mut mark, d);
    }
    mark.text = text;
    mark
}

/// `MarkCollection.EnsureOrder`, which `AddMark` runs after every addition: by start, then by end.
fn ensure_order(collection: &mut MarkCollection) {
    collection.marks.sort_by_key(|m| (m.start_time, m.end_time()));
}

/// `CreateNewCollection(color, name)`: the constructor's defaults with a name and colour.
fn new_collection(color: Color, name: impl Into<String>) -> MarkCollection {
    MarkCollection { name: name.into(), decorator: MarkDecorator { color, ..MarkDecorator::default() }, ..MarkCollection::default() }
}

// ---------------------------------------------------------------------------------------------------------------------
// Vixen 3 (.v3m), F28 and F38
// ---------------------------------------------------------------------------------------------------------------------

/// F28. Dispatches on the root's namespace. The serializer then insists on the exact root name, and a collection with no `Marks` element
/// throws, so both are checked here: `mark_collection_from_xml` alone would read a missing `Marks` as empty. Ids, flags and links come
/// from the file, so a file imported into the sequence it came from gives collections with ids that are already there. KEEP.
pub fn read_v3m(text: &str) -> Result<Vec<MarkCollection>, MarkFileError> {
    let root = XElement::parse(text).map_err(|e| MarkFileError(e.0))?;
    match root.ns.as_str() {
        V3M_NS => {
            if root.local != "ArrayOfIMarkCollection" {
                return fail(format!("Expected ArrayOfIMarkCollection, found {}.", root.local));
            }
            root.elements()
                .map(|item| {
                    if item.local != "anyType" || item.ns != ARRAYS_NS {
                        return fail(format!("Unexpected element {}.", item.local));
                    }
                    if item.child("Marks").is_none_or(|m| m.nil) {
                        return fail("A mark collection has no Marks.");
                    }
                    Ok(mark_collection_from_xml(item))
                })
                .collect()
        }
        LEGACY_V3M_NS => {
            if root.local != "ArrayOfMarkCollection" {
                return fail(format!("Expected ArrayOfMarkCollection, found {}.", root.local));
            }
            root.elements().map(migrate_legacy_collection).collect()
        }
        other => fail(format!("Unknown mark collection namespace '{other}'.")),
    }
}

/// `MigrateMarkCollections`: a new id, and each time becomes a 450 ms mark. The old id is dropped, and no default is chosen.
fn migrate_legacy_collection(old: &XElement) -> Result<MarkCollection, MarkFileError> {
    let Some(times) = old.child("Marks").filter(|m| !m.nil) else { return fail("A legacy mark collection has no Marks.") };
    let mut collection = MarkCollection {
        name: child_string(old, "Name").unwrap_or_default(),
        level: child_i32(old, "Level").unwrap_or(0),
        show_grid_lines: child_bool(old, "Enabled").unwrap_or(false),
        show_tail_lines: false,
        is_locked: false,
        decorator: MarkDecorator {
            color: old.child("MarkColor").and_then(color_from_xml).unwrap_or(Color::EMPTY),
            is_bold: child_bool(old, "Bold").unwrap_or(false),
            is_solid_line: child_bool(old, "SolidLine").unwrap_or(false),
            compact_mode: false,
        },
        ..MarkCollection::default()
    };
    for t in times.elements() {
        let Some(start) = TimeSpan::parse_xml(&t.text()) else { return fail(format!("Bad time '{}'.", t.text())) };
        collection.marks.push(Mark::new(start));
    }
    ensure_order(&mut collection);
    Ok(collection)
}

/// F38: what `DataContractSerializer` writes for a `List<IMarkCollection>` under the `.v3m` root, through an `XmlWriter` indenting with
/// tabs: a byte order mark, the declaration, CRLF line ends. Members in the order .NET's reader requires. No Windows-written sample
/// exists to compare with, so the prefixes are this writer's choice; readers go by namespace.
pub fn write_v3m(collections: &[&MarkCollection]) -> Result<Vec<u8>, MarkFileError> {
    let mut w = V3mWriter { out: String::with_capacity(4096 + collections.iter().map(|c| c.marks.len() * 220).sum::<usize>()) };
    w.out.push('\u{feff}');
    w.out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>");
    w.line(0, &format!("<ArrayOfIMarkCollection xmlns:d1p1=\"{ARRAYS_NS}\" xmlns:i=\"{XSI_NS}\" xmlns=\"{V3M_NS}\">"));
    for c in collections {
        w.line(1, &format!("<d1p1:anyType xmlns:d2p1=\"{NS_MARKS}\" i:type=\"d2p1:MarkCollection\">"));
        w.element(2, "d2p1:CollectionType", collection_type_name(c.collection_type))?;
        w.line(2, "<d2p1:Decorator i:type=\"d2p1:MarkDecorator\">");
        w.line(3, &format!("<d2p1:Color xmlns:d4p1=\"{NS_DRAWING}\">"));
        let color = DotNetColor::from_color(c.decorator.color);
        w.element(4, "d4p1:knownColor", &color.known_color.to_string())?;
        match &color.name {
            Some(name) => w.element(4, "d4p1:name", name)?,
            None => w.line(4, "<d4p1:name i:nil=\"true\" />"),
        }
        w.element(4, "d4p1:state", &color.state.to_string())?;
        w.element(4, "d4p1:value", &color.value.to_string())?;
        w.line(3, "</d2p1:Color>");
        w.element(3, "d2p1:IsBold", bool_text(c.decorator.is_bold))?;
        w.element(3, "d2p1:IsSolidLine", bool_text(c.decorator.is_solid_line))?;
        w.line(2, "</d2p1:Decorator>");
        w.element(2, "d2p1:Id", &c.id.to_string())?;
        w.element(2, "d2p1:IsDefault", bool_text(c.is_default))?;
        w.element(2, "d2p1:Level", &c.level.to_string())?;
        w.element(2, "d2p1:LinkedMarkCollectionId", &c.linked_mark_collection_id.unwrap_or_default().to_string())?;
        w.element(2, "d2p1:Locked", bool_text(c.is_locked))?;
        if c.marks.is_empty() {
            w.line(2, "<d2p1:Marks />");
        } else {
            w.line(2, "<d2p1:Marks>");
            for m in &c.marks {
                w.line(3, "<d1p1:anyType i:type=\"d2p1:Mark\">");
                w.element(4, "d2p1:Duration", &m.duration.to_xml())?;
                w.element(4, "d2p1:StartTime", &m.start_time.to_xml())?;
                w.element(4, "d2p1:Text", &m.text)?;
                w.line(3, "</d1p1:anyType>");
            }
            w.line(2, "</d2p1:Marks>");
        }
        w.element(2, "d2p1:Name", &c.name)?;
        w.element(2, "d2p1:ShowGridLines", bool_text(c.show_grid_lines))?;
        w.element(2, "d2p1:ShowMarkBar", bool_text(c.show_marks))?;
        w.element(2, "d2p1:ShowTailGridLines", bool_text(c.show_tail_lines))?;
        w.line(1, "</d1p1:anyType>");
    }
    w.line(0, "</ArrayOfIMarkCollection>");
    Ok(w.out.into_bytes())
}

struct V3mWriter {
    out: String,
}

impl V3mWriter {
    fn line(&mut self, depth: usize, text: &str) {
        if !self.out.is_empty() {
            self.out.push_str("\r\n");
        }
        for _ in 0..depth {
            self.out.push('\t');
        }
        self.out.push_str(text);
    }

    /// An element holding text. `XmlWriter` with `CheckCharacters` throws on a character XML can't hold, and its default newline handling
    /// writes every line break in text as CRLF.
    fn element(&mut self, depth: usize, name: &str, text: &str) -> Result<(), MarkFileError> {
        let mut escaped = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    escaped.push_str("\r\n");
                }
                '\n' => escaped.push_str("\r\n"),
                '\t' => escaped.push('\t'),
                c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {
                    return fail(format!("'{}' can't be written: it holds the character U+{:04X}.", text, c as u32));
                }
                c => escaped.push(c),
            }
        }
        self.line(depth, &format!("<{name}>{escaped}</{name}>"));
        Ok(())
    }
}

fn bool_text(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

/// `MarkCollectionType`'s member names.
pub fn collection_type_name(t: MarkCollectionType) -> &'static str {
    match t {
        MarkCollectionType::Generic => "Generic",
        MarkCollectionType::Phrase => "Phrase",
        MarkCollectionType::Word => "Word",
        MarkCollectionType::Phoneme => "Phoneme",
        MarkCollectionType::State => "State",
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Audacity, F29, F30, F39
// ---------------------------------------------------------------------------------------------------------------------

/// `Color.Yellow`.
const AUDACITY_YELLOW: Color = Color::rgb(255, 255, 0);
/// `Yellow, Gold, Goldenrod, SaddleBrown, CadetBlue, BlueViolet`.
const BEAT_COLORS: [Color; 6] = [
    Color::rgb(255, 255, 0),
    Color::rgb(255, 215, 0),
    Color::rgb(218, 165, 32),
    Color::rgb(139, 69, 19),
    Color::rgb(95, 158, 160),
    Color::rgb(138, 43, 226),
];

/// F29, `LoadBarLabelsAsync`, which both "Vamp Beat Tracker: Bars" and "Audacity Beats" run. One collection, `Audacity Marks`, yellow.
/// KEEP: only three fields are read, so text after a second tab is cut, and with space splitting only the first word of a label survives.
pub fn read_bar_labels(text: &str) -> Result<Vec<MarkCollection>, MarkFileError> {
    let everything = text.replace('\r', "");
    let lines: Vec<&str> = everything.split('\n').filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let mut collection = new_collection(AUDACITY_YELLOW, "Audacity Marks");
    for line in lines {
        // `IndexOf('\t') > 0`: a tab at the very start takes the space path.
        let parts: Vec<&str> = if line.find('\t').is_some_and(|i| i > 0) { line.split('\t').collect() } else { dotnet_trim(line).split(' ').collect() };
        let start_text = dotnet_trim(parts[0]);
        let end_text = parts.get(1).map_or("0", |p| dotnet_trim(p));
        let label = parts.get(2).map_or("", |p| dotnet_trim(p));
        let start = parse_invariant_double(start_text).and_then(time_from_secs).ok_or_else(|| MarkFileError(format!("Bad start time '{start_text}'.")))?;
        let end = parse_invariant_double(end_text).and_then(time_from_secs).ok_or_else(|| MarkFileError(format!("Bad end time '{end_text}'.")))?;
        let duration = if end > TimeSpan::ZERO { end - start } else { TimeSpan::ZERO };
        collection.marks.push(mark_with(start, Some(duration), label.to_string()));
    }
    ensure_order(&mut collection);
    Ok(vec![collection])
}

/// `string.Trim()`: Unicode white space.
fn dotnet_trim(s: &str) -> &str {
    s.trim_matches(char::is_whitespace)
}

/// F30, `LoadBeatLabelsAsync`, which "Vamp Beat Tracker: Beats" runs. The pattern `(\d*\.\d*)\s(\d*\.\d*)\s(\d)` over the whole file: one
/// collection per beat number up to the highest seen, each match a 450 ms mark in its beat's collection. KEEP: no match at all is a success
/// with nothing, because `Max` over no strings is null and `Convert.ToInt32(null)` is 0. GUARD: more than six beats, a beat 0 and a bare `.`
/// throw upstream, and are the same error here.
pub fn read_beat_labels(text: &str) -> Result<Vec<MarkCollection>, MarkFileError> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let matches = beat_matches(text);
    let Some(highest) = matches.iter().map(|m| m.2).max() else { return Ok(Vec::new()) };
    let number_of_beats = highest.to_digit(10).ok_or_else(|| MarkFileError(format!("Bad beat number '{highest}'.")))? as usize;
    if number_of_beats > BEAT_COLORS.len() {
        return fail(format!("{number_of_beats} beats is more than the {} colours there are.", BEAT_COLORS.len()));
    }
    let mut collections: Vec<MarkCollection> =
        (0..number_of_beats).map(|i| new_collection(BEAT_COLORS[i], format!("Audacity Beat {} Marks", i + 1))).collect();
    for (start, _, beat) in &matches {
        let time = parse_invariant_double(start).and_then(time_from_secs).ok_or_else(|| MarkFileError(format!("Bad time '{start}'.")))?;
        let beat = beat.to_digit(10).ok_or_else(|| MarkFileError(format!("Bad beat number '{beat}'.")))? as usize;
        let Some(collection) = beat.checked_sub(1).and_then(|i| collections.get_mut(i)) else { return fail(format!("Beat {beat} has no collection.")) };
        collection.marks.push(Mark::new(time));
    }
    for c in &mut collections {
        ensure_order(c);
    }
    Ok(collections)
}

/// .NET's `\d`: a Unicode decimal digit. Non-ASCII ones match and then fail to parse, as upstream.
fn is_regex_digit(c: char) -> bool {
    c.is_ascii_digit() || (!c.is_ascii() && c.is_numeric())
}

/// Leftmost, non-overlapping matches of `(\d*\.\d*)\s(\d*\.\d*)\s(\d)`. Each `\d*` is greedy and giving digits back can't help the next
/// token match, so a match from a given position is decided without backtracking.
fn beat_matches(text: &str) -> Vec<(String, String, char)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut p = 0;
    while p < chars.len() {
        match beat_match_at(&chars, p) {
            Some((end, m)) => {
                out.push(m);
                p = end;
            }
            None => p += 1,
        }
    }
    out
}

fn beat_match_at(chars: &[char], start: usize) -> Option<(usize, (String, String, char))> {
    let decimal = |mut i: usize| -> Option<(usize, String)> {
        let from = i;
        while i < chars.len() && is_regex_digit(chars[i]) {
            i += 1;
        }
        if chars.get(i) != Some(&'.') {
            return None;
        }
        i += 1;
        while i < chars.len() && is_regex_digit(chars[i]) {
            i += 1;
        }
        Some((i, chars[from..i].iter().collect()))
    };
    let (i, first) = decimal(start)?;
    if !chars.get(i).is_some_and(|c| c.is_whitespace()) {
        return None;
    }
    let (i, second) = decimal(i + 1)?;
    if !chars.get(i).is_some_and(|c| c.is_whitespace()) {
        return None;
    }
    let beat = *chars.get(i + 1).filter(|c| is_regex_digit(**c))?;
    Some((i + 2, (first, second, beat)))
}

/// F39. One line per mark, `<start>\t<start>\t<n>` with both times the start in seconds as `0000.000`. With one collection `n` counts its
/// marks, as bars; with more it is the collection's position, as beats. Lines sort as strings and end in CRLF, as `WriteAllLines` on
/// Windows writes them.
pub fn write_audacity(collections: &[&MarkCollection]) -> String {
    let single = collections.len() == 1;
    let mut lines: Vec<String> = Vec::with_capacity(collections.iter().map(|c| c.marks.len()).sum());
    let mut number = 0;
    for c in collections {
        number += 1;
        for m in &c.marks {
            let t = format_0000_000(m.start_time.total_secs());
            lines.push(format!("{t}\t{t}\t{number}"));
            if single {
                number += 1;
            }
        }
    }
    // Every line has one shape, so ordinal order is upstream's culture order, apart from a negative time's sign (not pinned).
    lines.sort();
    let mut out = String::with_capacity(lines.len() * 24);
    for line in lines {
        out.push_str(&line);
        out.push_str("\r\n");
    }
    out
}

/// `double.ToString("0000.000")`: the value to 15 significant digits, then rounded half away from zero at the third decimal, which differs
/// from rounding the exact binary value: 1.0005 s is `0001.001`.
pub fn format_0000_000(value: f64) -> String {
    if !value.is_finite() {
        return if value.is_nan() { "NaN".to_string() } else if value > 0.0 { "∞".to_string() } else { "-∞".to_string() };
    }
    let scientific = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    let point = exponent + 1;
    let (int_part, mut fraction): (Vec<u8>, Vec<u8>) = if point <= 0 {
        let mut f = vec![0; (-point) as usize];
        f.extend_from_slice(&digits);
        (vec![0], f)
    } else if point as usize >= digits.len() {
        let mut i = digits.clone();
        i.resize(point as usize, 0);
        (i, Vec::new())
    } else {
        (digits[..point as usize].to_vec(), digits[point as usize..].to_vec())
    };
    let round_up = fraction.get(3).is_some_and(|d| *d >= 5);
    fraction.resize(3, 0);
    let mut all: Vec<u8> = int_part.into_iter().chain(fraction).collect();
    if round_up {
        let mut i = all.len();
        loop {
            if i == 0 {
                all.insert(0, 1);
                break;
            }
            i -= 1;
            if all[i] == 9 {
                all[i] = 0;
            } else {
                all[i] += 1;
                break;
            }
        }
    }
    let (whole, frac) = all.split_at(all.len() - 3);
    let mut whole: String = whole.iter().map(|d| (b'0' + d) as char).collect();
    let trimmed = whole.trim_start_matches('0').len();
    whole = whole[whole.len() - trimmed..].to_string();
    while whole.len() < 4 {
        whole.insert(0, '0');
    }
    let frac: String = frac.iter().map(|d| (b'0' + d) as char).collect();
    let negative = value < 0.0 && all.iter().any(|d| *d != 0);
    format!("{}{whole}.{frac}", if negative { "-" } else { "" })
}

// ---------------------------------------------------------------------------------------------------------------------
// xTiming, F31 and F33
// ---------------------------------------------------------------------------------------------------------------------

/// `Color.Brown`.
const XTIMING_BROWN: Color = Color::rgb(165, 42, 42);

/// F31's `MaterializeXTimingTracks`, shared with the Singing Faces download (F33). `/timings/timing`, else a root `timing`, else nothing.
pub fn read_xtiming(xml: &str) -> Result<Vec<MarkCollection>, MarkFileError> {
    let root = XElement::parse(xml).map_err(|e| MarkFileError(e.0))?;
    let plain = |e: &XElement, name: &str| e.ns.is_empty() && e.local == name;
    let timings: Vec<&XElement> = if plain(&root, "timings") {
        root.elements().filter(|e| plain(e, "timing")).collect()
    } else if plain(&root, "timing") {
        vec![&root]
    } else {
        Vec::new()
    };
    let mut out = Vec::new();
    for timing in timings {
        out.extend(materialize_timing(timing)?);
    }
    Ok(out)
}

/// `MaterializeTiming`. More than one layer makes a lip sync track: the first kept layer is the phrases, the second the words linked to them,
/// the third the phonemes linked to the words. KEEP: a layer with no marks is dropped and doesn't advance the number, so the roles shift.
fn materialize_timing(timing: &XElement) -> Result<Vec<MarkCollection>, MarkFileError> {
    let name = timing.attr("name").unwrap_or("xTiming").to_string();
    let layers: Vec<&XElement> = timing.elements().filter(|e| e.ns.is_empty() && e.local == "EffectLayer").collect();
    let lip_sync = layers.len() > 1;
    let mut candidates: Vec<MarkCollection> = Vec::new();
    let mut layer_number = 1;
    for layer in layers {
        let mut collection = new_collection(XTIMING_BROWN, format!("{name} - {layer_number}"));
        if lip_sync {
            let (kind, suffix) = match layer_number {
                1 => (MarkCollectionType::Phrase, "Phrase"),
                2 => (MarkCollectionType::Word, "Word"),
                3 => (MarkCollectionType::Phoneme, "Phoneme"),
                _ => (MarkCollectionType::Generic, ""),
            };
            if !suffix.is_empty() {
                collection.collection_type = kind;
                collection.name = format!("{name} - {suffix}");
                if layer_number > 1 {
                    collection.linked_mark_collection_id = candidates.last().map(|c| c.id);
                }
            }
        }
        collection.show_marks = true;
        for effect in layer.elements().filter(|e| e.ns.is_empty() && e.local == "Effect") {
            let (Some(start_text), Some(end_text)) = (effect.attr("starttime"), effect.attr("endtime")) else { continue };
            if start_text == end_text {
                continue;
            }
            let start = parse_invariant_double(start_text).and_then(time_from_ms).ok_or_else(|| MarkFileError(format!("Bad start time '{start_text}'.")))?;
            let end = parse_invariant_double(end_text).and_then(time_from_ms).ok_or_else(|| MarkFileError(format!("Bad end time '{end_text}'.")))?;
            // A missing label is null upstream; the text is a String here, so it's empty.
            collection.marks.push(mark_with(start, Some(end - start), effect.attr("label").unwrap_or_default().to_string()));
        }
        if !collection.marks.is_empty() {
            ensure_order(&mut collection);
            candidates.push(collection);
            layer_number += 1;
        }
    }
    Ok(candidates)
}

// ---------------------------------------------------------------------------------------------------------------------
// Pangolin Beyond, F34 and F40
// ---------------------------------------------------------------------------------------------------------------------

pub const PANGOLIN_HEADER: &str = "#,Name,Start,Color";

/// One row of a Pangolin Beyond CSV.
#[derive(Clone, Debug, PartialEq)]
pub struct PangolinRecord {
    pub text: String,
    pub start: TimeSpan,
    pub color: Color,
}

/// How F34 builds collections from the records.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PangolinMode {
    GroupByColor,
    Single(Color),
}

/// `PangolinBeyondMarkParser.TryParse`. The message is only logged upstream; the user sees F34's fixed one.
pub fn parse_pangolin(csv: &str) -> Result<Vec<PangolinRecord>, String> {
    if csv.is_empty() {
        return Err("The Pangolin Beyond CSV file is empty.".to_string());
    }
    let normalized = csv.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    if lines.first() != Some(&PANGOLIN_HEADER) {
        return Err(format!("Line 1 must be the Pangolin Beyond header '{PANGOLIN_HEADER}'."));
    }
    let mut records = Vec::with_capacity(lines.len() - 1);
    for (index, line) in lines.iter().enumerate().skip(1) {
        let number = index + 1;
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 4 {
            return Err(format!("Line {number} must contain exactly four comma-separated columns."));
        }
        let Some(start) = parse_pangolin_time(fields[2]) else { return Err(format!("Line {number} has an invalid start time '{}'.", fields[2])) };
        let Some(color) = parse_bgr(fields[3]) else { return Err(format!("Line {number} has an invalid BGR color '{}'.", fields[3])) };
        records.push(PangolinRecord { text: fields[1].to_string(), start, color });
    }
    Ok(records)
}

/// `TimeSpan.TryParseExact` with `mm\:ss\.fff`, then `hh\:mm\:ss\.fff`: two-digit fields in range and exactly three fraction digits.
fn parse_pangolin_time(text: &str) -> Option<TimeSpan> {
    let b = text.as_bytes();
    let two = |i: usize| -> Option<i64> {
        let (x, y) = (*b.get(i)?, *b.get(i + 1)?);
        (x.is_ascii_digit() && y.is_ascii_digit()).then(|| ((x - b'0') * 10 + (y - b'0')) as i64)
    };
    let fraction = |i: usize| -> Option<i64> {
        let d = b.get(i..i + 3)?;
        d.iter().all(u8::is_ascii_digit).then(|| d.iter().fold(0i64, |a, x| a * 10 + (x - b'0') as i64))
    };
    let (hours, rest) = match b.len() {
        9 => (0, 0),
        12 if b[2] == b':' => (two(0)?, 3),
        _ => return None,
    };
    if b[rest + 2] != b':' || b[rest + 5] != b'.' {
        return None;
    }
    let (minutes, seconds, ms) = (two(rest)?, two(rest + 3)?, fraction(rest + 6)?);
    if hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }
    Some(TimeSpan::from_ticks(hours * TICKS_PER_HOUR + minutes * TICKS_PER_MIN + seconds * TICKS_PER_SEC + ms * TICKS_PER_MS))
}

/// `TryParseBgrColor`: exactly six hex digits, blue first.
fn parse_bgr(text: &str) -> Option<Color> {
    if text.len() != 6 || !text.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&text[i..i + 2], 16).ok();
    Some(Color::rgb(byte(4)?, byte(2)?, byte(0)?))
}

/// F34: two or more distinct colours ask whether to make one collection per colour.
pub fn pangolin_needs_choice(records: &[PangolinRecord]) -> bool {
    let mut first: Option<Color> = None;
    records.iter().any(|r| match first {
        None => {
            first = Some(r.color);
            false
        }
        Some(c) => c != r.color,
    })
}

/// `PangolinBeyondMarkCollectionFactory.Create`: by colour in order of first appearance, named `Beyond Marks - #RRGGBB`, or one collection
/// named `Beyond Marks` in the chosen colour. Marks keep the default 450 ms.
pub fn pangolin_collections(records: &[PangolinRecord], mode: PangolinMode) -> Vec<MarkCollection> {
    let build = |name: String, color: Color, rows: &mut dyn Iterator<Item = &PangolinRecord>| {
        let mut c = new_collection(color, name);
        c.show_marks = true;
        c.marks = rows.map(|r| mark_with(r.start, None, r.text.clone())).collect();
        ensure_order(&mut c);
        c
    };
    match mode {
        PangolinMode::Single(color) => vec![build("Beyond Marks".to_string(), color, &mut records.iter())],
        PangolinMode::GroupByColor => {
            let mut order: Vec<Color> = Vec::new();
            for r in records {
                if !order.contains(&r.color) {
                    order.push(r.color);
                }
            }
            order
                .into_iter()
                .map(|color| {
                    let name = format!("Beyond Marks - #{:02X}{:02X}{:02X}", color.r, color.g, color.b);
                    build(name, color, &mut records.iter().filter(|r| r.color == color))
                })
                .collect()
        }
    }
}

/// F40. Records from every collection sorted by start, stably, so ties keep collection order. The colour is the collection's as `BBGGRR`, and
/// the text has commas turned to spaces when it's included. KEEP: a newline in a mark's text breaks the file.
pub fn write_pangolin(rows: &[(&MarkCollection, bool)]) -> String {
    let mut records: Vec<(TimeSpan, String, String)> = Vec::new();
    for (collection, include_text) in rows {
        let c = collection.decorator.color;
        let color = format!("{:02X}{:02X}{:02X}", c.b, c.g, c.r);
        for m in &collection.marks {
            let text = if *include_text { m.text.replace(',', " ") } else { String::new() };
            records.push((m.start_time, text, color.clone()));
        }
    }
    records.sort_by_key(|r| r.0);
    let mut out = String::with_capacity(32 + records.len() * 32);
    out.push_str(PANGOLIN_HEADER);
    out.push_str("\r\n");
    for (n, (start, text, color)) in records.iter().enumerate() {
        out.push_str(&format!("M{},{},{},{}\r\n", n + 1, text, pangolin_time(*start), color));
    }
    out
}

/// `hh\:mm\:ss\.fff` when `TimeSpan.Hours` is above zero, else `mm\:ss\.fff`. Custom formats show no sign and take the components of the
/// absolute value, with the milliseconds truncated; days are dropped.
fn pangolin_time(t: TimeSpan) -> String {
    let ticks = t.ticks();
    let hours_component = (ticks / TICKS_PER_HOUR) % 24;
    let time = (ticks.unsigned_abs() % TICKS_PER_DAY as u64) as i64;
    let (h, m, s, ms) = (time / TICKS_PER_HOUR, (time / TICKS_PER_MIN) % 60, (time / TICKS_PER_SEC) % 60, (time % TICKS_PER_SEC) / TICKS_PER_MS);
    if hours_component > 0 {
        format!("{h:02}:{m:02}:{s:02}.{ms:03}")
    } else {
        format!("{m:02}:{s:02}.{ms:03}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v)
    }

    fn marks(c: &MarkCollection) -> Vec<(f64, f64, &str)> {
        c.marks.iter().map(|m| (m.start_time.total_ms(), m.duration.total_ms(), m.text.as_str())).collect()
    }

    #[test]
    fn import_text_strips_byte_order_marks_and_replaces_bad_bytes() {
        assert_eq!(read_import_text(b"\xEF\xBB\xBFabc"), "abc");
        assert_eq!(read_import_text(&[0xFF, 0xFE, b'h', 0, b'i', 0]), "hi");
        assert_eq!(read_import_text(&[0xFE, 0xFF, 0, b'h', 0, b'i']), "hi");
        assert_eq!(read_import_text(&[0xFF, 0xFE, 0, 0, b'h', 0, 0, 0]), "h");
        assert_eq!(read_import_text(b"caf\xE9"), "caf\u{FFFD}");
    }

    #[test]
    fn invariant_doubles_take_thousands_exponents_and_white_space() {
        assert_eq!(parse_invariant_double("1,234.5"), Some(1234.5));
        assert_eq!(parse_invariant_double(" 1.5 "), Some(1.5));
        assert_eq!(parse_invariant_double("1e3"), Some(1000.0));
        assert_eq!(parse_invariant_double(".5"), Some(0.5));
        assert_eq!(parse_invariant_double("2."), Some(2.0));
        assert_eq!(parse_invariant_double("-0.25"), Some(-0.25));
        for bad in ["", " ", ".", ",5", "abc", "1e", "1.5x", "NaN", "Infinity", "1.2.3"] {
            assert_eq!(parse_invariant_double(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn bar_labels_as_the_spec_lists() {
        let got = read_bar_labels("1.0\t2.0\tVerse\n3.5\t3.5\t\n6.0\n").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Audacity Marks");
        assert_eq!(got[0].decorator.color, Color::rgb(255, 255, 0));
        assert!(got[0].show_grid_lines && !got[0].show_marks);
        assert_eq!(marks(&got[0]), vec![(1000.0, 1000.0, "Verse"), (3500.0, 25.0, ""), (6000.0, 25.0, "")]);
        let spaced = read_bar_labels("4.0 5.0 two words").unwrap();
        assert_eq!(marks(&spaced[0]), vec![(4000.0, 1000.0, "two")]);
        assert!(read_bar_labels("abc").is_err());
        assert!(read_bar_labels("   \n").is_err());
        assert!(read_bar_labels("").unwrap().is_empty());
        assert!(read_bar_labels("\r\n\n").unwrap().is_empty());
    }

    #[test]
    fn beat_labels_as_the_spec_lists() {
        let got = read_beat_labels("1.000000\t1.000000\t1\n1.500000\t1.500000\t2\n").unwrap();
        let names: Vec<&str> = got.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Audacity Beat 1 Marks", "Audacity Beat 2 Marks"]);
        assert_eq!(got[0].decorator.color, Color::rgb(255, 255, 0));
        assert_eq!(got[1].decorator.color, Color::rgb(255, 215, 0));
        assert_eq!(marks(&got[0]), vec![(1000.0, 450.0, "")]);
        assert_eq!(marks(&got[1]), vec![(1500.0, 450.0, "")]);
        assert!(read_beat_labels("1.0 1.0 7\n").is_err());
        assert!(read_beat_labels("1.0 1.0 0\n").is_err());
        assert!(read_beat_labels(". 1.0 1\n").is_err());
        assert!(read_beat_labels("nothing to see\n").unwrap().is_empty());
        assert!(read_beat_labels("").unwrap().is_empty());
        // Only the first digit of the third column counts.
        let got = read_beat_labels("2.0\t2.0\t31\n").unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[2].marks.len(), 1);
    }

    #[test]
    fn audacity_export_numbers_marks_for_one_collection_and_collections_for_several() {
        let mut a = MarkCollection::default();
        a.marks = vec![Mark::new(ms(1500)), Mark::new(ms(2250))];
        assert_eq!(write_audacity(&[&a]), "0001.500\t0001.500\t1\r\n0002.250\t0002.250\t2\r\n");
        let mut b = MarkCollection::default();
        b.marks = vec![Mark::new(ms(1000))];
        assert_eq!(write_audacity(&[&a, &b]), "0001.000\t0001.000\t2\r\n0001.500\t0001.500\t1\r\n0002.250\t0002.250\t1\r\n");
        // Ties order as text: 10 before 2.
        let many: Vec<MarkCollection> = (0..10).map(|_| MarkCollection { marks: vec![Mark::new(ms(500))], ..MarkCollection::default() }).collect();
        let refs: Vec<&MarkCollection> = many.iter().collect();
        let text = write_audacity(&refs);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "0000.500\t0000.500\t1");
        assert_eq!(lines[1], "0000.500\t0000.500\t10");
        assert_eq!(lines[2], "0000.500\t0000.500\t2");
    }

    #[test]
    fn fixed_format_rounds_from_fifteen_digits() {
        assert_eq!(format_0000_000(1.5), "0001.500");
        assert_eq!(format_0000_000(0.0), "0000.000");
        assert_eq!(format_0000_000(1.0005), "0001.001");
        assert_eq!(format_0000_000(9.9996), "0010.000");
        assert_eq!(format_0000_000(12345.6784), "12345.678");
        assert_eq!(format_0000_000(-1.0), "-0001.000");
        assert_eq!(format_0000_000(0.000_4), "0000.000");
    }

    #[test]
    fn xtiming_lip_sync_layers_become_linked_phrase_word_phoneme() {
        let xml = r#"<timings><timing name="Voice">
            <EffectLayer><Effect label="hello world" starttime="0" endtime="1000"/></EffectLayer>
            <EffectLayer><Effect label="hello" starttime="0" endtime="500"/><Effect label="world" starttime="500" endtime="1000"/></EffectLayer>
            <EffectLayer><Effect label="AI" starttime="0" endtime="10"/><Effect starttime="5" endtime="5"/></EffectLayer>
        </timing></timings>"#;
        let got = read_xtiming(xml).unwrap();
        let names: Vec<&str> = got.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Voice - Phrase", "Voice - Word", "Voice - Phoneme"]);
        assert_eq!(got[1].linked_mark_collection_id, Some(got[0].id));
        assert_eq!(got[2].linked_mark_collection_id, Some(got[1].id));
        assert_eq!(got[2].collection_type, MarkCollectionType::Phoneme);
        assert!(got.iter().all(|c| c.show_marks && c.decorator.color == Color::rgb(165, 42, 42)));
        // Equal start and end strings are skipped; a short mark takes 25 ms.
        assert_eq!(marks(&got[2]), vec![(0.0, 25.0, "AI")]);
    }

    #[test]
    fn xtiming_single_layer_empty_layers_and_other_roots() {
        let one = read_xtiming(r#"<timing name="T"><EffectLayer><Effect starttime="100" endtime="400"/></EffectLayer></timing>"#).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].name, "T - 1");
        assert_eq!(one[0].collection_type, MarkCollectionType::Generic);
        assert_eq!(marks(&one[0]), vec![(100.0, 300.0, "")]);
        let shifted = read_xtiming(
            r#"<timing name="V"><EffectLayer/><EffectLayer><Effect starttime="0" endtime="100"/></EffectLayer><EffectLayer><Effect starttime="0" endtime="50"/></EffectLayer></timing>"#,
        )
        .unwrap();
        let names: Vec<&str> = shifted.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["V - Phrase", "V - Word"]);
        assert!(read_xtiming("<other/>").unwrap().is_empty());
        let unnamed = read_xtiming(r#"<timings><timing><EffectLayer><Effect starttime="0" endtime="100"/></EffectLayer></timing></timings>"#).unwrap();
        assert_eq!(unnamed[0].name, "xTiming - 1");
        assert!(read_xtiming(r#"<timing><EffectLayer><Effect starttime="x" endtime="100"/></EffectLayer></timing>"#).is_err());
    }

    #[test]
    fn pangolin_parses_upstreams_sample_and_names_its_errors() {
        let got = parse_pangolin("#,Name,Start,Color\r\nM1,Intro,00:01.250,112233\r\nM2,Chorus,01:02:03.004,A0B0C0\r\n").unwrap();
        assert_eq!(got[0], PangolinRecord { text: "Intro".to_string(), start: ms(1250), color: Color::rgb(0x33, 0x22, 0x11) });
        assert_eq!(got[1].start, ms(3_723_004));
        assert_eq!(got[1].color, Color::rgb(0xC0, 0xB0, 0xA0));
        assert_eq!(parse_pangolin("").unwrap_err(), "The Pangolin Beyond CSV file is empty.");
        assert_eq!(parse_pangolin("bad\n").unwrap_err(), "Line 1 must be the Pangolin Beyond header '#,Name,Start,Color'.");
        assert_eq!(parse_pangolin("#,Name,Start,Color\nM1,a,00:01.000\n").unwrap_err(), "Line 2 must contain exactly four comma-separated columns.");
        assert_eq!(parse_pangolin("#,Name,Start,Color\nM1,a,1.25,112233\n").unwrap_err(), "Line 2 has an invalid start time '1.25'.");
        assert_eq!(parse_pangolin("#,Name,Start,Color\nM1,a,61:00.000,112233\n").unwrap_err(), "Line 2 has an invalid start time '61:00.000'.");
        assert_eq!(parse_pangolin("#,Name,Start,Color\nM1,a,00:01.000,GG2233\n").unwrap_err(), "Line 2 has an invalid BGR color 'GG2233'.");
        assert!(parse_pangolin("#,Name,Start,Color\n").unwrap().is_empty());
    }

    #[test]
    fn pangolin_groups_by_colour_in_first_appearance_order() {
        let records = parse_pangolin("#,Name,Start,Color\nM1,a,00:01.000,112233\nM2,b,00:02.000,0000FF\nM3,c,00:03.000,112233\n").unwrap();
        assert!(pangolin_needs_choice(&records));
        let grouped = pangolin_collections(&records, PangolinMode::GroupByColor);
        let names: Vec<&str> = grouped.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Beyond Marks - #332211", "Beyond Marks - #FF0000"]);
        assert_eq!(marks(&grouped[0]), vec![(1000.0, 450.0, "a"), (3000.0, 450.0, "c")]);
        assert!(grouped[0].show_marks);
        let single = pangolin_collections(&records, PangolinMode::Single(Color::rgb(1, 2, 3)));
        assert_eq!(single[0].name, "Beyond Marks");
        assert_eq!(single[0].marks.len(), 3);
        assert!(!pangolin_needs_choice(&records[..1]));
        assert!(pangolin_collections(&[], PangolinMode::GroupByColor).is_empty());
    }

    #[test]
    fn pangolin_export_and_round_trip() {
        let mut red = MarkCollection { decorator: MarkDecorator { color: Color::rgb(255, 0, 0), ..MarkDecorator::default() }, ..MarkCollection::default() };
        let mut m = Mark::new(TimeSpan::from_ticks(612_345_000));
        m.text = "a,b".to_string();
        red.marks.push(m);
        assert_eq!(write_pangolin(&[(&red, true)]), "#,Name,Start,Color\r\nM1,a b,01:01.234,0000FF\r\n");
        assert_eq!(write_pangolin(&[(&red, false)]), "#,Name,Start,Color\r\nM1,,01:01.234,0000FF\r\n");
        let mut blue = MarkCollection { decorator: MarkDecorator { color: Color::rgb(0, 0, 255), ..MarkDecorator::default() }, ..MarkCollection::default() };
        blue.marks.push(Mark::new(TimeSpan::from_ticks(612_345_000)));
        blue.marks.push(Mark::new(ms(3_723_004)));
        let text = write_pangolin(&[(&red, true), (&blue, true)]);
        assert_eq!(text, "#,Name,Start,Color\r\nM1,a b,01:01.234,0000FF\r\nM2,,01:01.234,FF0000\r\nM3,,01:02:03.004,FF0000\r\n");
        let back = parse_pangolin(&text).unwrap();
        assert_eq!(back.len(), 3);
        assert_eq!(back[0].color, Color::rgb(255, 0, 0));
        assert_eq!(back[2].start, ms(3_723_004));
    }

    #[test]
    fn v3m_round_trips_every_member() {
        let mut a = MarkCollection { name: "Beats & <Bars>".to_string(), level: 3, is_default: true, is_locked: true, show_tail_lines: true, show_marks: true, ..MarkCollection::default() };
        a.collection_type = MarkCollectionType::Word;
        a.decorator = MarkDecorator { color: Color::rgb(10, 20, 30), is_bold: true, is_solid_line: true, compact_mode: false };
        let mut m = Mark::new(ms(1500));
        m.duration = ms(300);
        m.text = "line one\nline two".to_string();
        a.marks.push(m);
        let mut b = MarkCollection { name: "Empty".to_string(), ..MarkCollection::default() };
        b.linked_mark_collection_id = Some(a.id);
        let bytes = write_v3m(&[&a, &b]).unwrap();
        let text = read_import_text(&bytes);
        assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<ArrayOfIMarkCollection"));
        assert!(text.contains("\r\n\t<d1p1:anyType"));
        let back = read_v3m(&text).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].id, a.id);
        assert_eq!(back[0].name, a.name);
        assert_eq!(back[0].level, 3);
        assert!(back[0].is_default && back[0].is_locked && back[0].show_tail_lines && back[0].show_marks);
        assert_eq!(back[0].collection_type, MarkCollectionType::Word);
        assert_eq!(back[0].decorator, a.decorator);
        assert_eq!(marks(&back[0]), vec![(1500.0, 300.0, "line one\nline two")]);
        assert_eq!(back[1].linked_mark_collection_id, Some(a.id));
        assert!(back[1].marks.is_empty());
        assert!(write_v3m(&[&MarkCollection { name: "bell\u{7}".to_string(), ..MarkCollection::default() }]).is_err());
    }

    #[test]
    fn v3m_refuses_what_the_serializer_would() {
        let no_marks = format!(
            "<ArrayOfIMarkCollection xmlns=\"{V3M_NS}\" xmlns:a=\"{ARRAYS_NS}\"><a:anyType><Name xmlns=\"{NS_MARKS}\">x</Name></a:anyType></ArrayOfIMarkCollection>"
        );
        assert!(read_v3m(&no_marks).is_err());
        assert!(read_v3m(&format!("<Wrong xmlns=\"{V3M_NS}\"/>")).is_err());
        assert!(read_v3m("<ArrayOfIMarkCollection xmlns=\"urn:other\"/>").is_err());
        assert!(read_v3m("not xml").is_err());
        assert!(read_v3m(&format!("<ArrayOfIMarkCollection xmlns=\"{V3M_NS}\"/>")).unwrap().is_empty());
    }

    #[test]
    fn legacy_v3m_migrates_to_450_ms_marks_with_new_ids() {
        let old_id = "763f55a4-4769-49ad-bba6-e23c534d01c6";
        let xml = format!(
            "<ArrayOfMarkCollection xmlns=\"{LEGACY_V3M_NS}\" xmlns:a=\"{ARRAYS_NS}\"><MarkCollection><Bold>true</Bold><Enabled>true</Enabled><Id>{old_id}</Id><Level>2</Level>\
             <MarkColor xmlns:b=\"{NS_DRAWING}\"><b:knownColor>0</b:knownColor><b:name i:nil=\"true\" xmlns:i=\"{XSI_NS}\"/><b:state>2</b:state><b:value>4294901760</b:value></MarkColor>\
             <Marks><a:duration>PT2S</a:duration><a:duration>PT1S</a:duration></Marks><Name>Old</Name><SolidLine>false</SolidLine></MarkCollection></ArrayOfMarkCollection>"
        );
        let got = read_v3m(&xml).unwrap();
        assert_eq!(got.len(), 1);
        assert_ne!(got[0].id.to_string(), old_id);
        assert_eq!(got[0].name, "Old");
        assert_eq!(got[0].level, 2);
        assert!(got[0].show_grid_lines && got[0].decorator.is_bold && !got[0].is_default);
        assert_eq!(got[0].decorator.color, Color::rgb(255, 0, 0));
        assert_eq!(marks(&got[0]), vec![(1000.0, 450.0, ""), (2000.0, 450.0, "")]);
    }
}
