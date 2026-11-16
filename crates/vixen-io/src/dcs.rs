//! DataContractSerializer's text forms for primitive values, and member ordering.

use uuid::Uuid;
use vixen_core::TimeSpan;
use vixen_xml::{XElement, XNode};

/// `XmlConvert.ToString(double)` as .NET Core and later write it: the shortest digits that read back as the
/// same value, in fixed notation unless the exponent is below -5 or at least 15.
///
/// .NET Framework wrote 15 digits or else 17, so older files can hold a longer spelling of the same value.
/// Such text reads back equal and is kept by the codecs' fidelity data rather than regenerated here.
pub fn format_double(v: f64) -> String {
    format_float_text(v, format!("{v:e}"), format!("{v}"), 15)
}

/// The same for `float`, whose fixed range ends at an exponent of 7.
pub fn format_single(v: f32) -> String {
    format_float_text(v as f64, format!("{v:e}"), format!("{v}"), 7)
}

fn format_float_text(v: f64, scientific: String, fixed: String, fixed_limit: i32) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "INF" } else { "-INF" }.to_string();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        return fixed;
    };
    let exponent: i32 = exponent.parse().unwrap_or(0);
    if exponent > -5 && exponent < fixed_limit {
        fixed
    } else {
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{mantissa}E{sign}{:02}", exponent.abs())
    }
}

/// `XmlConvert.ToDouble`, which spells infinity differently from Rust.
pub fn parse_double(text: &str) -> Option<f64> {
    match text.trim() {
        "INF" | "+INF" | "Infinity" => Some(f64::INFINITY),
        "-INF" | "-Infinity" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        t => t.parse().ok(),
    }
}

/// `XmlConvert.ToBoolean`.
pub fn parse_bool(text: &str) -> Option<bool> {
    match text.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        // `XElement` writes `bool` through `XmlConvert`, but a hand-edited or very old file may carry
        // `Boolean.ToString()`, which `Boolean.Parse` accepts.
        "True" => Some(true),
        "False" => Some(false),
        _ => None,
    }
}

pub fn format_bool(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

/// `Guid.Parse` accepts braces and uppercase, as does `Uuid::parse_str`.
pub fn parse_guid(text: &str) -> Option<Uuid> {
    let t = text.trim();
    let t = t.strip_prefix('{').and_then(|t| t.strip_suffix('}')).unwrap_or(t);
    Uuid::parse_str(t).ok()
}

pub fn format_guid(id: &Uuid) -> String {
    id.hyphenated().to_string()
}

/// `XmlConvert.ToTimeSpan`.
pub fn parse_time(text: &str) -> Option<TimeSpan> {
    TimeSpan::parse_xml(text)
}

/// The trimmed text of a child, unless it is nil or missing.
pub(crate) fn child_value(e: &XElement, local: &str) -> Option<String> {
    let c = e.child(local)?;
    if c.nil {
        None
    } else {
        Some(c.text().trim().to_string())
    }
}

/// The text of a child with nothing trimmed, unless it is nil or missing.
pub(crate) fn child_string(e: &XElement, local: &str) -> Option<String> {
    let c = e.child(local)?;
    if c.nil {
        None
    } else {
        Some(c.text())
    }
}

pub(crate) fn child_guid(e: &XElement, local: &str) -> Option<Uuid> {
    parse_guid(&child_value(e, local)?)
}

pub(crate) fn child_time(e: &XElement, local: &str) -> Option<TimeSpan> {
    parse_time(&child_value(e, local)?)
}

pub(crate) fn child_i32(e: &XElement, local: &str) -> Option<i32> {
    child_value(e, local)?.parse().ok()
}

pub(crate) fn child_bool(e: &XElement, local: &str) -> Option<bool> {
    parse_bool(&child_value(e, local)?)
}

pub(crate) fn child_f64(e: &XElement, local: &str) -> Option<f64> {
    parse_double(&child_value(e, local)?)
}

/// A nullable string member: nil for `None`, an empty element for an empty string.
pub(crate) fn opt_string_element(ns: &str, local: &str, value: Option<&str>) -> XElement {
    match value {
        Some(v) => XElement::with_text(ns, local, v),
        None => XElement::nil(ns, local),
    }
}

/// Inserts a member where DataContractSerializer would write it: after the last member of the same namespace
/// that sorts before it, or at the end when the namespace has no members yet. Replaces a member of that name.
pub(crate) fn set_member(parent: &mut XElement, member: XElement) {
    let mut insert_at = None;
    let mut last_same_ns = None;
    for (i, n) in parent.children.iter_mut().enumerate() {
        if let XNode::Element(e) = n {
            if e.ns == member.ns {
                if e.local == member.local {
                    *e = member;
                    return;
                }
                last_same_ns = Some(i + 1);
                if e.local.as_str() < member.local.as_str() {
                    insert_at = Some(i + 1);
                }
            }
        }
    }
    // Before the first larger member of its namespace, or after the last one when none sort before it.
    let at =
        insert_at.or_else(|| parent.children.iter().position(|n| matches!(n, XNode::Element(e) if e.ns == member.ns)));
    let at = at.or(last_same_ns).unwrap_or(parent.children.len());
    parent.children.insert(at, XNode::Element(member));
}

/// Windows' `Path.GetFileName`, which must also work on a path written on Windows and read on macOS.
pub fn windows_file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_write_like_xmlconvert() {
        for (v, text) in [
            (0.0, "0"),
            (1.0, "1"),
            (0.5, "0.5"),
            (100.0, "100"),
            (50.09116921142579, "50.09116921142579"),
            (0.0392156862745098, "0.0392156862745098"),
            (0.0001, "0.0001"),
            (0.00001, "1E-05"),
            (1.5e-7, "1.5E-07"),
            (1e14, "100000000000000"),
            (1e15, "1E+15"),
            (1.7976931348623157e308, "1.7976931348623157E+308"),
            (-2.5, "-2.5"),
        ] {
            assert_eq!(format_double(v), text);
            assert_eq!(parse_double(text), Some(v), "{text}");
        }
    }

    #[test]
    fn special_doubles_use_the_xml_spelling() {
        assert_eq!(format_double(f64::INFINITY), "INF");
        assert_eq!(format_double(f64::NEG_INFINITY), "-INF");
        assert_eq!(format_double(f64::NAN), "NaN");
        assert_eq!(parse_double("INF"), Some(f64::INFINITY));
        assert_eq!(parse_double("-INF"), Some(f64::NEG_INFINITY));
        assert!(parse_double("NaN").unwrap().is_nan());
        assert_eq!(format_double(-0.0), "-0");
    }

    #[test]
    fn a_framework_spelling_parses_to_the_same_value_as_the_shortest_one() {
        // .NET Framework wrote 17 digits where 16 round trip. This value is in a real file.
        let framework = parse_double("0.60175054704595188").unwrap();
        assert_eq!(framework, parse_double("0.6017505470459519").unwrap());
        assert_eq!(format_double(framework), "0.6017505470459519");
    }

    #[test]
    fn singles_switch_to_scientific_sooner() {
        assert_eq!(format_single(0.2), "0.2");
        assert_eq!(format_single(1_000_000.0), "1000000");
        assert_eq!(format_single(10_000_000.0), "1E+07");
    }

    #[test]
    fn timespans_write_like_xmlconvert() {
        use vixen_core::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_MIN, TICKS_PER_MS, TICKS_PER_SEC};
        for (text, ticks) in [
            ("PT0S", 0),
            ("PT0.0000001S", 1),
            ("-PT0.0000001S", -1),
            ("PT0.001S", TICKS_PER_MS),
            ("PT0.45S", 450 * TICKS_PER_MS),
            ("PT2M31.797S", 2 * TICKS_PER_MIN + 31 * TICKS_PER_SEC + 797 * TICKS_PER_MS),
            ("PT1H", TICKS_PER_HOUR),
            ("PT1H1S", TICKS_PER_HOUR + TICKS_PER_SEC),
            ("P1D", TICKS_PER_DAY),
            ("P1DT1M", TICKS_PER_DAY + TICKS_PER_MIN),
            ("P10675199DT2H48M5.4775807S", i64::MAX),
            ("-P10675199DT2H48M5.4775808S", i64::MIN),
        ] {
            let t = parse_time(text).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(t.ticks(), ticks, "{text}");
            assert_eq!(t.to_xml(), text);
        }
    }

    #[test]
    fn timespans_read_spellings_vixen_does_not_write() {
        use vixen_core::time::{TICKS_PER_HOUR, TICKS_PER_MIN, TICKS_PER_SEC};
        for (text, ticks) in [
            ("PT60S", TICKS_PER_MIN),
            ("PT90M", 90 * TICKS_PER_MIN),
            ("PT25H", 25 * TICKS_PER_HOUR),
            ("PT1H0M0S", TICKS_PER_HOUR),
            (" PT1S ", TICKS_PER_SEC),
            // Digits past a tick are dropped.
            ("PT0.00000019S", 1),
        ] {
            assert_eq!(parse_time(text).map(|t| t.ticks()), Some(ticks), "{text}");
        }
        for text in ["", "P", "PT", "1:00:00", "PT1", "PTS", "PT1X"] {
            assert_eq!(parse_time(text), None, "{text:?}");
        }
    }

    #[test]
    fn guids_accept_what_guid_parse_accepts() {
        let id = parse_guid("{BFF34727-6B88-4F87-82B7-68424498C725}").unwrap();
        assert_eq!(format_guid(&id), "bff34727-6b88-4f87-82b7-68424498c725");
        assert_eq!(parse_guid("not a guid"), None);
    }

    #[test]
    fn set_member_uses_ordinal_order_within_a_namespace() {
        let mut e = XElement::new("", "o");
        e.push(XElement::with_text("", "ModuleInstanceId", "x"));
        e.push(XElement::with_text("b", "Alpha", "1"));
        e.push(XElement::with_text("b", "Gamma", "3"));
        set_member(&mut e, XElement::with_text("b", "Beta", "2"));
        set_member(&mut e, XElement::with_text("b", "Aardvark", "0"));
        set_member(&mut e, XElement::with_text("b", "Zed", "9"));
        set_member(&mut e, XElement::with_text("b", "Gamma", "33"));
        let names: Vec<_> = e.elements().map(|c| c.local.as_str()).collect();
        assert_eq!(names, ["ModuleInstanceId", "Aardvark", "Alpha", "Beta", "Gamma", "Zed"]);
        assert_eq!(e.child_text("Gamma").as_deref(), Some("33"));
    }

    #[test]
    fn windows_paths_give_their_file_name() {
        assert_eq!(windows_file_name(r"C:\Users\x\Vixen 3\Sequence\Carey Grinch.tim"), "Carey Grinch.tim");
        assert_eq!(windows_file_name("/Users/x/Song.tim"), "Song.tim");
        assert_eq!(windows_file_name("Song.tim"), "Song.tim");
        assert_eq!(windows_file_name(r"..\Users\userb\Media\a.mp3"), "a.mp3");
    }
}
