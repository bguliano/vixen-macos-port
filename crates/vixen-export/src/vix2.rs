//! `Vix2Writer` ("Vixen 2.1 Sequence"), `HelixWriter` ("Helix File", identical apart from its name)
//! and `Vir2Writer` ("Vixen 2 Routine"). All three keep the data channel by channel, so they buffer
//! the whole export.

use crate::{file_name_of, ExportError, ExportWriter, Result, SessionData, NEWLINE};
use base64::Engine;
use std::fs::File;
use std::io::{BufWriter, Write};

/// `XmlWriter` escaping for element text.
fn escape_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

/// `XmlWriter` escaping for double-quoted attribute values.
fn escape_attr(s: &str, out: &mut String) {
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

fn element(out: &mut String, name: &str, value: &str) {
    out.push_str("    <");
    out.push_str(name);
    out.push('>');
    escape_text(value, out);
    out.push_str("</");
    out.push_str(name);
    out.push('>');
    out.push_str(NEWLINE);
}

/// The `Vix2XMLData` document, as `XmlSerializer` lays it out: four-space indent, UTF-8 with BOM.
pub fn vix2_document(data: &SessionData, event_values: &[u8]) -> String {
    let mut x = String::with_capacity(event_values.len() / 3 * 4 + 1024 + data.channel_names.len() * 64);
    x.push('\u{FEFF}');
    x.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>");
    x.push_str(NEWLINE);
    x.push_str("<Program>");
    x.push_str(NEWLINE);
    element(&mut x, "Time", &data.time_ms.to_string());
    element(&mut x, "EventPeriodInMilliseconds", &data.period_ms.to_string());
    element(&mut x, "MinimumLevel", "0");
    element(&mut x, "MaximumLevel", "255");
    element(&mut x, "AudioDevice", "-1");
    element(&mut x, "AudioVolume", "0");
    if data.channel_names.is_empty() {
        x.push_str("    <Channels />");
        x.push_str(NEWLINE);
    } else {
        x.push_str("    <Channels>");
        x.push_str(NEWLINE);
        for (i, name) in data.channel_names.iter().enumerate() {
            x.push_str(&format!("        <Channel id=\"{i}\" output=\"{i}\" enabled=\"true\" color=\"-1\">"));
            escape_text(name, &mut x);
            x.push_str("</Channel>");
            x.push_str(NEWLINE);
        }
        x.push_str("    </Channels>");
        x.push_str(NEWLINE);
    }
    if !data.audio_file_name.is_empty() {
        let filename = file_name_of(&data.audio_file_name);
        x.push_str("    <Audio filename=\"");
        escape_attr(filename, &mut x);
        // The original really does put the period here.
        x.push_str(&format!("\" duration=\"{}\">", data.period_ms));
        escape_text(filename, &mut x);
        x.push_str("</Audio>");
        x.push_str(NEWLINE);
    }
    element(&mut x, "EngineType", "Standard");
    element(&mut x, "EventValues", &base64::engine::general_purpose::STANDARD.encode(event_values));
    x.push_str("</Program>");
    x
}

/// `Vix2Writer` and `HelixWriter`.
pub struct Vix2Writer {
    session: Option<SessionData>,
    period_data: Vec<u8>,
    num_periods: usize,
    cur_period: usize,
    out: Option<File>,
}

impl Vix2Writer {
    pub fn new() -> Vix2Writer {
        Vix2Writer { session: None, period_data: Vec::new(), num_periods: 0, cur_period: 0, out: None }
    }
}

impl Default for Vix2Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportWriter for Vix2Writer {
    fn open_session(&mut self, data: &SessionData) -> Result<()> {
        self.cur_period = 0;
        self.out = Some(File::create(&data.out_file_name)?);
        // A sequence that doesn't end on an interval boundary gets room for one more period.
        let adder = usize::from(data.time_ms % data.period_ms as f64 != 0.0);
        self.num_periods = data.num_periods.max(0) as usize + adder;
        self.period_data = vec![0u8; data.channel_names.len() * self.num_periods];
        self.session = Some(data.clone());
        Ok(())
    }

    fn write_next_period(&mut self, period: &[u8]) -> Result<()> {
        if self.session.is_none() {
            return Err(ExportError::NotOpen);
        }
        for (channel, value) in period.iter().enumerate() {
            self.period_data[channel * self.num_periods + self.cur_period] = *value;
        }
        self.cur_period += 1;
        Ok(())
    }

    fn close_session(&mut self) -> Result<()> {
        let session = self.session.take().ok_or(ExportError::NotOpen)?;
        let mut out = self.out.take().ok_or(ExportError::NotOpen)?;
        out.write_all(vix2_document(&session, &self.period_data).as_bytes())?;
        out.flush()?;
        Ok(())
    }
}

/// `Vir2Writer`: one line per channel of space-separated decimal values.
pub struct Vir2Writer {
    channels: usize,
    num_periods: usize,
    cur_period: usize,
    period_data: Vec<u8>,
    out: Option<BufWriter<File>>,
}

impl Vir2Writer {
    pub fn new() -> Vir2Writer {
        Vir2Writer { channels: 0, num_periods: 0, cur_period: 0, period_data: Vec::new(), out: None }
    }
}

impl Default for Vir2Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportWriter for Vir2Writer {
    fn open_session(&mut self, data: &SessionData) -> Result<()> {
        self.cur_period = 0;
        self.channels = data.channel_names.len();
        self.num_periods = data.num_periods.max(0) as usize;
        self.period_data = vec![0u8; self.channels * self.num_periods];
        self.out = Some(BufWriter::with_capacity(1 << 20, File::create(&data.out_file_name)?));
        Ok(())
    }

    fn write_next_period(&mut self, period: &[u8]) -> Result<()> {
        if self.out.is_none() {
            return Err(ExportError::NotOpen);
        }
        for (channel, value) in period.iter().enumerate() {
            self.period_data[channel * self.num_periods + self.cur_period] = *value;
        }
        self.cur_period += 1;
        Ok(())
    }

    fn close_session(&mut self) -> Result<()> {
        let mut out = self.out.take().ok_or(ExportError::NotOpen)?;
        for channel in 0..self.channels {
            for period in 0..self.num_periods {
                write!(out, "{} ", self.period_data[channel * self.num_periods + period])?;
            }
            out.write_all(NEWLINE.as_bytes())?;
        }
        out.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_matches_xml_serializer_layout() {
        let data = SessionData {
            period_ms: 50,
            time_ms: 100.0,
            num_periods: 2,
            channel_names: vec!["A & B".into(), "C".into()],
            audio_file_name: "C:\\Music\\song.mp3".into(),
            ..Default::default()
        };
        let doc = vix2_document(&data, &[1, 2, 3, 4]);
        let expected = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<Program>\r\n    <Time>100</Time>\r\n    \
            <EventPeriodInMilliseconds>50</EventPeriodInMilliseconds>\r\n    <MinimumLevel>0</MinimumLevel>\r\n    \
            <MaximumLevel>255</MaximumLevel>\r\n    <AudioDevice>-1</AudioDevice>\r\n    <AudioVolume>0</AudioVolume>\r\n    \
            <Channels>\r\n        <Channel id=\"0\" output=\"0\" enabled=\"true\" color=\"-1\">A &amp; B</Channel>\r\n        \
            <Channel id=\"1\" output=\"1\" enabled=\"true\" color=\"-1\">C</Channel>\r\n    </Channels>\r\n    \
            <Audio filename=\"song.mp3\" duration=\"50\">song.mp3</Audio>\r\n    <EngineType>Standard</EngineType>\r\n    \
            <EventValues>AQIDBA==</EventValues>\r\n</Program>";
        assert_eq!(doc, expected);
    }

    #[test]
    fn no_audio_omits_the_audio_element() {
        let data = SessionData { period_ms: 25, time_ms: 25.5, num_periods: 1, ..Default::default() };
        let doc = vix2_document(&data, &[]);
        assert!(!doc.contains("<Audio "));
        assert!(doc.contains("<Time>25.5</Time>"));
        assert!(doc.contains("<Channels />"));
    }
}
