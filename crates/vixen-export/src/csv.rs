//! `CSVWriter`: one row per interval, each channel as a three-digit decimal.

use crate::{ExportError, ExportWriter, Result, SessionData, NEWLINE};
use std::fs::File;
use std::io::{BufWriter, Write};

pub struct CsvWriter {
    channels: usize,
    out: Option<BufWriter<File>>,
    row: Vec<u8>,
}

impl CsvWriter {
    pub fn new() -> CsvWriter {
        CsvWriter { channels: 0, out: None, row: Vec::new() }
    }
}

impl Default for CsvWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Append `value` as exactly three digits (`ToString("000")`).
#[inline]
fn push_three_digits(row: &mut Vec<u8>, value: u8) {
    row.push(b'0' + value / 100);
    row.push(b'0' + value / 10 % 10);
    row.push(b'0' + value % 10);
}

impl ExportWriter for CsvWriter {
    fn open_session(&mut self, data: &SessionData) -> Result<()> {
        self.channels = data.channel_names.len();
        self.row = Vec::with_capacity(self.channels * 4 + 2);
        self.out = Some(BufWriter::with_capacity(1 << 20, File::create(&data.out_file_name)?));
        Ok(())
    }

    fn write_next_period(&mut self, period: &[u8]) -> Result<()> {
        let out = self.out.as_mut().ok_or(ExportError::NotOpen)?;
        self.row.clear();
        for (i, value) in period.iter().take(self.channels).enumerate() {
            if i > 0 {
                self.row.push(b',');
            }
            push_three_digits(&mut self.row, *value);
        }
        self.row.extend_from_slice(NEWLINE.as_bytes());
        out.write_all(&self.row)?;
        Ok(())
    }

    fn close_session(&mut self) -> Result<()> {
        let mut out = self.out.take().ok_or(ExportError::NotOpen)?;
        out.flush()?;
        Ok(())
    }
}
