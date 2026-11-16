//! `ESEQWriter`: "Falcon Player Effect".

use crate::{ExportError, ExportWriter, Result, SessionData};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};

const FIXED_HEADER_LENGTH: usize = 20;
const NUM_MODELS: i32 = 1;
const START_ADDRESS: i32 = 0;

/// The zero bytes that round a frame up to a multiple of four channels.
pub(crate) fn padding_for(channels: i32) -> usize {
    if channels % 4 != 0 {
        (4 - channels % 4) as usize
    } else {
        0
    }
}

pub struct EseqWriter {
    seq_num_channels: i32,
    padding: usize,
    out: Option<BufWriter<File>>,
}

impl EseqWriter {
    pub fn new() -> EseqWriter {
        EseqWriter { seq_num_channels: 0, padding: 0, out: None }
    }
}

impl Default for EseqWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportWriter for EseqWriter {
    fn open_session(&mut self, data: &SessionData) -> Result<()> {
        let mut out = BufWriter::with_capacity(1 << 20, File::create(&data.out_file_name)?);
        out.write_all(&[0u8; FIXED_HEADER_LENGTH])?;
        let channels = data.channel_names.len() as i32;
        self.padding = padding_for(channels);
        self.seq_num_channels = channels + self.padding as i32;
        self.out = Some(out);
        Ok(())
    }

    fn write_next_period(&mut self, period: &[u8]) -> Result<()> {
        let out = self.out.as_mut().ok_or(ExportError::NotOpen)?;
        out.write_all(period)?;
        out.write_all(&[0u8; 4][..self.padding])?;
        Ok(())
    }

    fn close_session(&mut self) -> Result<()> {
        let mut out = self.out.take().ok_or(ExportError::NotOpen)?;
        out.seek(SeekFrom::Start(0))?;
        out.write_all(b"ESEQ")?;
        out.write_all(&NUM_MODELS.to_le_bytes())?;
        // Step size.
        out.write_all(&self.seq_num_channels.to_le_bytes())?;
        // Model start address.
        out.write_all(&START_ADDRESS.to_le_bytes())?;
        // Model size.
        out.write_all(&self.seq_num_channels.to_le_bytes())?;
        out.flush()?;
        Ok(())
    }
}
