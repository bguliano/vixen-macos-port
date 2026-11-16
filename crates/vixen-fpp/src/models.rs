//! What the players answer with. Names match ignoring case and a missing field keeps its default, as upstream's
//! `System.Text.Json` options read them; a field of the wrong JSON type is an error rather than a blank.

use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemInfo {
    pub host_name: String,
    pub host_description: String,
    pub platform: String,
    pub zip: bool,
    pub variant: String,
    pub mode: String,
    pub version: String,
    pub branch: String,
    pub os_version: String,
    pub os_release: String,
    pub channel_ranges: String,
    pub major_version: i32,
    pub minor_version: i32,
    pub type_id: i32,
    pub uuid: String,
    pub utilization: Option<Utilization>,
    pub ips: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Utilization {
    pub cpu: f64,
    pub memory: f64,
    pub uptime: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Playlist {
    pub name: String,
    pub lead_in: Vec<PlaylistEntry>,
    pub main_playlist: Vec<PlaylistEntry>,
    pub lead_out: Vec<PlaylistEntry>,
    pub playlist_info: Option<PlaylistInfo>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlaylistEntry {
    pub kind: String,
    pub enabled: i32,
    pub play_once: i32,
    pub sequence_name: Option<String>,
    pub media_name: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlaylistInfo {
    pub total_duration: f64,
    pub total_items: i32,
}

impl Playlist {
    /// The sequences a playlist plays: lead-in, main, lead-out, entries of type `sequence` or `both` with a name.
    pub fn sequences(&self) -> Vec<String> {
        self.lead_in
            .iter()
            .chain(&self.main_playlist)
            .chain(&self.lead_out)
            .filter(|entry| entry.kind == "sequence" || entry.kind == "both")
            .filter_map(|entry| entry.sequence_name.clone())
            .filter(|name| !name.trim().is_empty())
            .collect()
    }
}

type Parsed<T> = Result<T, String>;

/// A repeated name keeps its last value.
pub(crate) fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    object.iter().filter(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value).last()
}

fn object<'a>(value: &'a Value, what: &str) -> Parsed<&'a Map<String, Value>> {
    value.as_object().ok_or_else(|| format!("{what} isn't a JSON object."))
}

fn optional_text(object: &Map<String, Value>, name: &str) -> Parsed<Option<String>> {
    match field(object, name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("'{name}' isn't text.")),
    }
}

fn text(object: &Map<String, Value>, name: &str) -> Parsed<String> {
    Ok(optional_text(object, name)?.unwrap_or_default())
}

fn int(object: &Map<String, Value>, name: &str) -> Parsed<i32> {
    match field(object, name) {
        None => Ok(0),
        Some(Value::Number(number)) => number.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or_else(|| format!("'{name}' isn't a whole number.")),
        Some(_) => Err(format!("'{name}' isn't a whole number.")),
    }
}

fn double(object: &Map<String, Value>, name: &str) -> Parsed<f64> {
    match field(object, name) {
        None => Ok(0.0),
        Some(Value::Number(number)) => number.as_f64().ok_or_else(|| format!("'{name}' isn't a number.")),
        Some(_) => Err(format!("'{name}' isn't a number.")),
    }
}

fn boolean(object: &Map<String, Value>, name: &str) -> Parsed<bool> {
    match field(object, name) {
        None => Ok(false),
        Some(Value::Bool(on)) => Ok(*on),
        Some(_) => Err(format!("'{name}' isn't true or false.")),
    }
}

pub(crate) fn names(value: &Value, what: &str) -> Parsed<Vec<String>> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => Ok(text.clone()),
                Value::Null => Ok(String::new()),
                _ => Err(format!("{what} holds something that isn't text.")),
            })
            .collect(),
        _ => Err(format!("{what} isn't a list.")),
    }
}

/// A JSON null is no system information at all, which the caller reports in its own words.
pub(crate) fn system_info(value: &Value) -> Parsed<Option<SystemInfo>> {
    if value.is_null() {
        return Ok(None);
    }
    let o = object(value, "The system information")?;
    let utilization = match field(o, "utilization") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let u = object(value, "'Utilization'")?;
            Some(Utilization { cpu: double(u, "cpu")?, memory: double(u, "memory")?, uptime: uptime(field(u, "uptime"))? })
        }
    };
    Ok(Some(SystemInfo {
        host_name: text(o, "hostName")?,
        host_description: text(o, "hostDescription")?,
        platform: text(o, "platform")?,
        zip: boolean(o, "zip")?,
        variant: text(o, "variant")?,
        mode: text(o, "mode")?,
        version: text(o, "version")?,
        branch: text(o, "branch")?,
        os_version: text(o, "osVersion")?,
        os_release: text(o, "osRelease")?,
        channel_ranges: text(o, "channelRanges")?,
        major_version: int(o, "majorVersion")?,
        minor_version: int(o, "minorVersion")?,
        type_id: int(o, "typeId")?,
        uuid: text(o, "uuid")?,
        utilization,
        ips: match field(o, "iPs") {
            None => Vec::new(),
            Some(value) => names(value, "'IPs'")?,
        },
    }))
}

/// `FppUptimeConverter`: text as it is, or milliseconds written as .NET's constant `TimeSpan` format.
fn uptime(value: Option<&Value>) -> Parsed<String> {
    match value {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(Value::Number(number)) => {
            let ticks = number.as_f64().unwrap_or(f64::NAN) * 10_000.0;
            if !ticks.is_finite() || ticks.abs() >= i64::MAX as f64 {
                return Err("FPP uptime is out of range.".into());
            }
            Ok(time_span(ticks as i64))
        }
        Some(_) => Err("FPP uptime must be a string or a number of milliseconds.".into()),
    }
}

fn time_span(ticks: i64) -> String {
    let sign = if ticks < 0 { "-" } else { "" };
    let ticks = ticks.unsigned_abs();
    let (seconds, fraction) = (ticks / 10_000_000, ticks % 10_000_000);
    let (days, hours, minutes, seconds) = (seconds / 86_400, seconds / 3600 % 24, seconds / 60 % 60, seconds % 60);
    let mut out = String::from(sign);
    if days > 0 {
        out.push_str(&format!("{days}."));
    }
    out.push_str(&format!("{hours:02}:{minutes:02}:{seconds:02}"));
    if fraction > 0 {
        out.push_str(&format!(".{fraction:07}"));
    }
    out
}

pub(crate) fn playlist(value: &Value) -> Parsed<Playlist> {
    let o = object(value, "The playlist")?;
    let entries = |name: &str| -> Parsed<Vec<PlaylistEntry>> {
        match field(o, name) {
            None | Some(Value::Null) => Ok(Vec::new()),
            Some(Value::Array(items)) => items.iter().map(entry).collect(),
            Some(_) => Err(format!("'{name}' isn't a list.")),
        }
    };
    let playlist_info = match field(o, "playlistInfo") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let info = object(value, "'PlaylistInfo'")?;
            Some(PlaylistInfo { total_duration: double(info, "total_duration")?, total_items: int(info, "total_items")? })
        }
    };
    Ok(Playlist { name: text(o, "name")?, lead_in: entries("leadIn")?, main_playlist: entries("mainPlaylist")?, lead_out: entries("leadOut")?, playlist_info })
}

fn entry(value: &Value) -> Parsed<PlaylistEntry> {
    if value.is_null() {
        return Ok(PlaylistEntry::default());
    }
    let o = object(value, "A playlist entry")?;
    Ok(PlaylistEntry {
        kind: text(o, "type")?,
        enabled: int(o, "enabled")?,
        play_once: int(o, "playOnce")?,
        sequence_name: optional_text(o, "sequenceName")?,
        media_name: optional_text(o, "mediaName")?,
        note: optional_text(o, "note")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn system_information_reads_whatever_case_the_device_uses() {
        let info = system_info(&json!({
            "HostName": "FPP", "hostdescription": "Tree", "Platform": "Raspberry Pi", "Variant": "Pi 4", "Mode": "player",
            "Version": "8.0", "Branch": "v8.0", "OSVersion": "v2024-05", "OSRelease": "Debian 12", "channelRanges": "0-511",
            "majorVersion": 8, "minorVersion": 0, "typeId": 13, "uuid": "M1-1", "zip": false,
            "Utilization": {"CPU": 3.5, "Memory": 21.25, "Uptime": "2 days"}, "IPs": ["192.168.1.50", "10.0.0.2"]
        }))
        .unwrap()
        .unwrap();
        assert_eq!(info.host_name, "FPP");
        assert_eq!(info.host_description, "Tree");
        assert_eq!(info.os_version, "v2024-05");
        assert_eq!(info.os_release, "Debian 12");
        assert_eq!((info.major_version, info.minor_version, info.type_id), (8, 0, 13));
        assert_eq!(info.utilization, Some(Utilization { cpu: 3.5, memory: 21.25, uptime: "2 days".into() }));
        assert_eq!(info.ips, ["192.168.1.50", "10.0.0.2"]);
    }

    #[test]
    fn nothing_is_required_and_null_is_no_information() {
        assert_eq!(system_info(&json!({})).unwrap(), Some(SystemInfo::default()));
        assert_eq!(system_info(&Value::Null).unwrap(), None);
        assert!(system_info(&json!([1])).is_err());
    }

    #[test]
    fn a_field_of_the_wrong_type_is_an_error() {
        assert!(system_info(&json!({"zip": "yes"})).is_err());
        assert!(system_info(&json!({"majorVersion": "8"})).is_err());
        assert!(system_info(&json!({"majorVersion": 8.5})).is_err());
        assert!(system_info(&json!({"platform": 3})).is_err());
        assert!(system_info(&json!({"Utilization": {"Uptime": true}})).is_err());
    }

    #[test]
    fn uptime_in_milliseconds_reads_as_a_time_span() {
        let read = |value: Value| system_info(&json!({"Utilization": {"Uptime": value}})).unwrap().unwrap().utilization.unwrap().uptime;
        assert_eq!(read(json!(0)), "00:00:00");
        assert_eq!(read(json!(1000)), "00:00:01");
        assert_eq!(read(json!(3_723_004)), "01:02:03.0040000");
        assert_eq!(read(json!(90_061_000)), "1.01:01:01");
        assert_eq!(read(json!(1.5)), "00:00:00.0015000");
        assert_eq!(read(json!(-1000)), "-00:00:01");
    }

    #[test]
    fn a_playlist_gives_its_sequences_in_playing_order() {
        let list = playlist(&json!({
            "name": "Show",
            "leadIn": [{"type": "sequence", "enabled": 1, "playOnce": 0, "sequenceName": "Intro.fseq"}, {"type": "pause"}],
            "mainPlaylist": [
                {"type": "both", "sequenceName": "Carol.fseq", "mediaName": "Carol.mp3"},
                {"type": "media", "mediaName": "Talk.mp3"},
                {"type": "sequence", "sequenceName": "  "},
                {"type": "Sequence", "sequenceName": "WrongCase.fseq"}
            ],
            "leadOut": [{"type": "sequence", "sequenceName": "Outro.fseq", "note": "bye"}],
            "playlistInfo": {"total_duration": 301.5, "total_items": 6}
        }))
        .unwrap();
        assert_eq!(list.sequences(), ["Intro.fseq", "Carol.fseq", "Outro.fseq"]);
        assert_eq!(list.playlist_info, Some(PlaylistInfo { total_duration: 301.5, total_items: 6 }));
        assert_eq!(list.main_playlist[0].media_name.as_deref(), Some("Carol.mp3"));
        assert_eq!(list.lead_out[0].note.as_deref(), Some("bye"));
    }

    #[test]
    fn a_list_of_names_may_be_null() {
        assert_eq!(names(&Value::Null, "x").unwrap(), Vec::<String>::new());
        assert_eq!(names(&json!(["a", "b"]), "x").unwrap(), ["a", "b"]);
        assert!(names(&json!({"a": 1}), "x").is_err());
    }
}
