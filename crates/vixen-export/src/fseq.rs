//! `FSEQCompressedWriter` (FSEQ 2.0, "Falcon Player Sequence 2.6+") and `FSEQWriter` (FSEQ 1.0), plus
//! `VariableHeader`.

use crate::{ExportError, ExportWriter, Result, SessionData};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

const V2_FIXED_HEADER_LENGTH: u32 = 32;
const V1_FIXED_HEADER_LENGTH: u16 = 28;
/// `V2FseqOutCompressionBlockSize`.
const COMPRESSION_BLOCK_SIZE: f64 = 64.0 * 1024.0;
const ZSTD_LEVEL: i32 = 10;
/// The first block is always this short, so a player can start quickly.
const FIRST_BLOCK_FRAMES: u32 = 10;

/// `VariableHeader`: a length-prefixed, NUL-terminated string with a two-letter code.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VariableHeader {
    pub code: [u8; 2],
    pub value: String,
}

impl VariableHeader {
    /// `new VariableHeader(mediaName)`.
    pub fn media_file(name: &str) -> VariableHeader {
        VariableHeader { code: *b"mf", value: name.to_string() }
    }

    /// `new VariableHeader(HeaderType.SequenceProducer)`.
    pub fn sequence_producer() -> VariableHeader {
        VariableHeader { code: *b"sp", value: "Vixen 3".to_string() }
    }

    /// `HeaderLength`. `Value.Length` counts UTF-16 units.
    pub fn header_length(&self) -> usize {
        self.value.encode_utf16().count() + 5
    }

    /// `GetHeaderBytes`. `Encoding.ASCII` replaces each unit it can't represent with `?`.
    pub fn header_bytes(&self) -> Vec<u8> {
        let len = self.header_length();
        let mut bytes = Vec::with_capacity(len);
        bytes.push((len & 0xFF) as u8);
        bytes.push(((len >> 8) & 0xFF) as u8);
        bytes.extend_from_slice(&self.code);
        bytes.extend(self.value.encode_utf16().map(|u| if u < 0x80 { u as u8 } else { b'?' }));
        bytes.push(0);
        bytes
    }
}

/// `DateTime.Now.Ticks` for the file's unique id. Only uniqueness matters, so this uses UTC.
fn now_ticks() -> i64 {
    const UNIX_EPOCH_TICKS: i64 = 621_355_968_000_000_000;
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    UNIX_EPOCH_TICKS + (since.as_nanos() / 100) as i64
}

/// The file id bytes. The original shifts by 30 where 32 was presumably meant, then carries on from
/// there, so the top bytes overlap: kept so files look the same.
pub fn unique_id_bytes(ticks: i64) -> [u8; 8] {
    let mut id = [0u8; 8];
    for (slot, shift) in id.iter_mut().zip([0, 8, 16, 24, 30, 38, 46, 54]) {
        *slot = ((ticks >> shift) & 0xFF) as u8;
    }
    id
}

/// `ComputeMaxBlockCount`: returns `(block count, frames per block)`.
pub fn compute_max_block_count(channels_per_frame: u32, number_frames: u32) -> (u32, u32) {
    // uint * uint: wraps in 32 bits before being widened, as the original does.
    let size = channels_per_frame.wrapping_mul(number_frames) as f64;
    let number_blocks = (size / COMPRESSION_BLOCK_SIZE).ceil().clamp(1.0, 255.0);

    let mut frames_per_block = (number_frames as f64 / number_blocks) as u32;
    if frames_per_block < FIRST_BLOCK_FRAMES {
        frames_per_block = FIRST_BLOCK_FRAMES;
    }

    // Saturating where the original wraps: see the crate notes.
    let frame_count = number_frames.saturating_sub(FIRST_BLOCK_FRAMES);
    let mut number_blocks = (frame_count as f64 / frames_per_block as f64).ceil();
    while number_blocks > 254.0 {
        frames_per_block += 1;
        number_blocks = (frame_count as f64 / frames_per_block as f64).ceil();
    }
    // The first, smaller block.
    number_blocks += 1.0;
    (number_blocks as u32, frames_per_block)
}

fn create(path: &Path) -> Result<BufWriter<File>> {
    Ok(BufWriter::with_capacity(1 << 20, File::create(path)?))
}

/// The block currently being filled.
enum BlockStream {
    Compressed(zstd::stream::write::Encoder<'static, Vec<u8>>),
    Raw(Vec<u8>),
}

impl BlockStream {
    fn new(compress: bool) -> Result<BlockStream> {
        Ok(if compress {
            BlockStream::Compressed(zstd::stream::write::Encoder::new(Vec::new(), ZSTD_LEVEL)?)
        } else {
            BlockStream::Raw(Vec::new())
        })
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        match self {
            BlockStream::Compressed(e) => e.write_all(data)?,
            BlockStream::Raw(v) => v.extend_from_slice(data),
        }
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>> {
        Ok(match self {
            BlockStream::Compressed(e) => e.finish()?,
            BlockStream::Raw(v) => v,
        })
    }
}

/// `FSEQCompressedWriter`. Writes FSEQ 2.0, with zstd compression unless turned off.
pub struct FseqV2Writer {
    enable_compression: bool,
    seq_period_time: i32,
    offset_to_channel_data: u32,
    channels_per_frame: u32,
    number_frames: u32,
    frames_per_block: u32,
    number_compression_blocks: u8,
    current_frame: u32,
    current_frame_in_block: u32,
    current_block: u32,
    block_start_frame: u32,
    variable_headers: Vec<VariableHeader>,
    compress_block_map: Vec<(u32, u32)>,
    out: Option<BufWriter<File>>,
    stream: Option<BlockStream>,
    /// Fixed for tests; otherwise the clock is read when the header is written.
    id_ticks: Option<i64>,
}

impl FseqV2Writer {
    pub fn new(enable_compression: bool) -> FseqV2Writer {
        FseqV2Writer {
            enable_compression,
            seq_period_time: 0,
            offset_to_channel_data: 0,
            channels_per_frame: 0,
            number_frames: 0,
            frames_per_block: 0,
            number_compression_blocks: 0,
            current_frame: 0,
            current_frame_in_block: 0,
            current_block: 0,
            block_start_frame: 0,
            variable_headers: Vec::new(),
            compress_block_map: Vec::new(),
            out: None,
            stream: None,
            id_ticks: None,
        }
    }

    /// Fix the unique id, so a test can compare whole files.
    pub fn with_id_ticks(mut self, ticks: i64) -> FseqV2Writer {
        self.id_ticks = Some(ticks);
        self
    }

    fn reset(&mut self) {
        self.current_block = 0;
        self.current_frame = 0;
        self.current_frame_in_block = 0;
        self.block_start_frame = 0;
        self.compress_block_map.clear();
        self.variable_headers.clear();
    }

    /// `ChangeBlock`.
    fn change_block(&mut self, last: bool) -> Result<()> {
        let stream = self.stream.take().ok_or(ExportError::NotOpen)?;
        let data = stream.finish()?;
        self.out.as_mut().ok_or(ExportError::NotOpen)?.write_all(&data)?;
        if !last {
            self.stream = Some(BlockStream::new(self.enable_compression)?);
        }
        self.compress_block_map.push((self.block_start_frame, data.len() as u32));
        self.block_start_frame = self.current_frame;
        self.current_block += 1;
        self.current_frame_in_block = 0;
        Ok(())
    }

    /// `WriteFileHeader`.
    fn file_header(&self) -> Vec<u8> {
        let mut header = vec![0u8; V2_FIXED_HEADER_LENGTH as usize];
        let length = V2_FIXED_HEADER_LENGTH as usize + self.compress_block_map.len() * 8;

        header[0..4].copy_from_slice(b"PSEQ");
        header[4] = (self.offset_to_channel_data & 0xFF) as u8;
        header[5] = ((self.offset_to_channel_data >> 8) & 0xFF) as u8;
        // Version: minor then major.
        header[6] = 0;
        header[7] = 2;
        // Offset to the variable headers.
        header[8] = (length & 0xFF) as u8;
        header[9] = ((length >> 8) & 0xFF) as u8;
        header[10..14].copy_from_slice(&self.channels_per_frame.to_le_bytes());
        header[14..18].copy_from_slice(&self.number_frames.to_le_bytes());
        header[18] = (self.seq_period_time & 0xFF) as u8;
        // 19: bit flags, stays 0.
        header[20] = u8::from(self.enable_compression);
        header[21] = self.number_compression_blocks;
        // 22: sparse ranges, none. 23: reserved.
        let ticks = self.id_ticks.unwrap_or_else(now_ticks);
        header[24..32].copy_from_slice(&unique_id_bytes(ticks));

        for (frame, len) in &self.compress_block_map {
            header.extend_from_slice(&frame.to_le_bytes());
            header.extend_from_slice(&len.to_le_bytes());
        }
        for variable in &self.variable_headers {
            header.extend_from_slice(&variable.header_bytes());
        }
        header
    }
}

impl ExportWriter for FseqV2Writer {
    fn open_session(&mut self, data: &SessionData) -> Result<()> {
        self.reset();
        self.seq_period_time = data.period_ms;
        self.number_frames = data.num_periods as u32;
        self.channels_per_frame = data.channel_names.len() as u32;
        if !data.output_audio_file_name.is_empty() {
            self.variable_headers.push(VariableHeader::media_file(&data.output_audio_file_name));
        }
        self.variable_headers.push(VariableHeader::sequence_producer());

        let (block_count, frames_per_block) = compute_max_block_count(self.channels_per_frame, self.number_frames);
        self.frames_per_block = frames_per_block;
        self.number_compression_blocks = block_count as u8;
        self.offset_to_channel_data = V2_FIXED_HEADER_LENGTH
            + block_count * 8
            + self.variable_headers.iter().map(|v| v.header_length() as u32).sum::<u32>();

        let mut out = create(&data.out_file_name)?;
        out.write_all(&vec![0u8; self.offset_to_channel_data as usize])?;
        self.out = Some(out);
        self.stream = Some(BlockStream::new(self.enable_compression)?);
        Ok(())
    }

    fn write_next_period(&mut self, period: &[u8]) -> Result<()> {
        if self.current_block < 1 && self.current_frame_in_block >= FIRST_BLOCK_FRAMES {
            self.change_block(false)?;
        } else if self.current_frame_in_block >= self.frames_per_block {
            self.change_block(false)?;
        }
        self.current_frame_in_block += 1;
        self.current_frame += 1;
        self.stream.as_mut().ok_or(ExportError::NotOpen)?.write(period)
    }

    fn close_session(&mut self) -> Result<()> {
        self.change_block(true)?;
        let header = self.file_header();
        let mut out = self.out.take().ok_or(ExportError::NotOpen)?;
        out.seek(SeekFrom::Start(0))?;
        out.write_all(&header)?;
        out.flush()?;
        Ok(())
    }
}

/// `FSEQWriter`. Writes FSEQ 1.0, padded to a multiple of four channels. Falcon Player reads it, but
/// the parser in the user's own player only accepts version 2.0.
pub struct FseqV1Writer {
    data_offset: u32,
    seq_num_channels: i32,
    seq_num_periods: u32,
    seq_period_time: i32,
    variable_headers: Vec<VariableHeader>,
    padding: usize,
    out: Option<BufWriter<File>>,
}

impl FseqV1Writer {
    pub fn new() -> FseqV1Writer {
        FseqV1Writer {
            data_offset: V1_FIXED_HEADER_LENGTH as u32,
            seq_num_channels: 0,
            seq_num_periods: 0,
            seq_period_time: 50,
            variable_headers: Vec::new(),
            padding: 0,
            out: None,
        }
    }

    /// `WriteFileHeader`.
    fn file_header(&self) -> Vec<u8> {
        let mut h = Vec::with_capacity(V1_FIXED_HEADER_LENGTH as usize);
        h.extend_from_slice(b"PSEQ");
        h.push((self.data_offset % 256) as u8);
        h.push((self.data_offset / 256) as u8);
        h.push(0); // minor
        h.push(1); // major
        h.push((V1_FIXED_HEADER_LENGTH % 256) as u8);
        h.push((V1_FIXED_HEADER_LENGTH / 256) as u8);
        h.extend_from_slice(&self.seq_num_channels.to_le_bytes());
        h.extend_from_slice(&self.seq_num_periods.to_le_bytes());
        h.push((self.seq_period_time & 0xFF) as u8);
        h.push(0); // flags
        h.extend_from_slice(&[0, 0]); // universe count, ignored by FPP
        h.extend_from_slice(&[0, 0]); // universe size, ignored by FPP
        h.push(1); // gamma
        h.push(2); // colour encoding: RGB
        h.extend_from_slice(&[0, 0]); // reserved
        for variable in &self.variable_headers {
            h.extend_from_slice(&variable.header_bytes());
        }
        h
    }
}

impl Default for FseqV1Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportWriter for FseqV1Writer {
    fn open_session(&mut self, data: &SessionData) -> Result<()> {
        self.variable_headers.clear();
        self.data_offset = V1_FIXED_HEADER_LENGTH as u32;
        self.seq_period_time = data.period_ms;
        if !data.output_audio_file_name.is_empty() {
            self.variable_headers.push(VariableHeader::media_file(&data.output_audio_file_name));
        }
        self.data_offset += self.variable_headers.iter().map(|v| v.header_length() as u32).sum::<u32>();

        let mut out = create(&data.out_file_name)?;
        out.write_all(&vec![0u8; self.data_offset as usize])?;
        let channels = data.channel_names.len() as i32;
        self.seq_num_periods = 0;
        self.padding = crate::eseq::padding_for(channels);
        self.seq_num_channels = channels + self.padding as i32;
        self.out = Some(out);
        Ok(())
    }

    fn write_next_period(&mut self, period: &[u8]) -> Result<()> {
        let out = self.out.as_mut().ok_or(ExportError::NotOpen)?;
        out.write_all(period)?;
        out.write_all(&[0u8; 4][..self.padding])?;
        self.seq_num_periods += 1;
        Ok(())
    }

    fn close_session(&mut self) -> Result<()> {
        let header = self.file_header();
        let mut out = self.out.take().ok_or(ExportError::NotOpen)?;
        out.seek(SeekFrom::Start(0))?;
        out.write_all(&header)?;
        out.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_header_layout() {
        let bytes = VariableHeader::media_file("Coldest Winter Pentatonix.mp3").header_bytes();
        assert_eq!(&bytes[..4], b"\"\x00mf");
        assert_eq!(&bytes[4..33], b"Coldest Winter Pentatonix.mp3");
        assert_eq!(*bytes.last().unwrap(), 0);
        assert_eq!(VariableHeader::sequence_producer().header_bytes(), b"\x0c\x00spVixen 3\x00");
    }

    #[test]
    fn non_ascii_names_become_question_marks_per_utf16_unit() {
        let h = VariableHeader::media_file("é😀.mp3");
        assert_eq!(h.header_length(), 7 + 5);
        assert_eq!(&h.header_bytes()[4..11], b"???.mp3");
    }

    #[test]
    fn block_count_matches_a_real_export() {
        // Coldest Winter.fseq: 3367 channels, 5946 frames, 249 blocks of 24 frames after the first ten.
        assert_eq!(compute_max_block_count(3367, 5946), (249, 24));
    }

    #[test]
    fn block_count_edge_cases() {
        assert_eq!(compute_max_block_count(16, 10), (1, 10));
        assert_eq!(compute_max_block_count(16, 11).0, 2);
        // Fewer than ten frames: one block, instead of the 255 the wrapped arithmetic gives.
        assert_eq!(compute_max_block_count(16, 3), (1, 10));
        assert_eq!(compute_max_block_count(16, 0), (1, 10));
        // Very large: never more than the header byte can hold.
        let (blocks, per_block) = compute_max_block_count(100_000, 1_000_000);
        assert!(blocks <= 255);
        assert!(1 + (1_000_000u32 - 10).div_ceil(per_block) <= 255);
    }

    #[test]
    fn id_uses_the_original_shifts() {
        // From a real file's header, with bits 62 and 63 clear as ticks always are.
        let ticks: i64 = 0x08DD_0000_0000_0000 | 0x7A6D_4FDF_AC;
        let id = unique_id_bytes(ticks);
        assert_eq!(id[0], 0xAC);
        assert_eq!(id[1], 0xDF);
        assert_eq!(id[4], ((ticks >> 30) & 0xFF) as u8);
        assert_eq!(id[7], ((ticks >> 54) & 0xFF) as u8);
    }
}
