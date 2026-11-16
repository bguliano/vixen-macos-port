//! An FSEQ reader. The original has none, since playback happens on the Pi; here it checks exports
//! and compares this port's output with files the Windows version produced.

use crate::{ExportError, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use zstd::stream::raw::{Decoder, InBuffer, Operation, OutBuffer};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Compression {
    None,
    Zstd,
    Zlib,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VariableHeaderEntry {
    pub code: [u8; 2],
    pub value: Vec<u8>,
}

impl VariableHeaderEntry {
    /// The value as text, without its trailing NUL.
    pub fn text(&self) -> String {
        let end = self.value.iter().position(|b| *b == 0).unwrap_or(self.value.len());
        String::from_utf8_lossy(&self.value[..end]).into_owned()
    }
}

/// A parsed FSEQ header. The block table holds `(first frame, bytes)` entries.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FseqHeader {
    pub major: u8,
    pub minor: u8,
    pub data_offset: u32,
    /// Bytes 8-9: fixed header length in v1, offset of the variable headers in v2.
    pub header_field: u16,
    pub channels: u32,
    pub frames: u32,
    pub step_ms: u8,
    pub flags: u8,
    pub compression: Compression,
    /// Byte 21, as declared. The table may hold entries with no data after the real ones.
    pub declared_blocks: u8,
    pub blocks: Vec<(u32, u32)>,
    pub sparse: Vec<(u32, u32)>,
    pub id: [u8; 8],
    pub variable_headers: Vec<VariableHeaderEntry>,
}

impl FseqHeader {
    /// The `mf` variable header.
    pub fn media_file(&self) -> Option<String> {
        self.variable_headers.iter().find(|h| &h.code == b"mf").map(VariableHeaderEntry::text)
    }

    /// The channels actually stored per frame, after any sparse ranges.
    pub fn stored_channels(&self) -> u32 {
        if self.sparse.is_empty() {
            self.channels
        } else {
            self.sparse.iter().map(|(_, count)| *count).sum()
        }
    }
}

fn format_err(message: impl Into<String>) -> ExportError {
    ExportError::Format(message.into())
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

fn u24_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], 0])
}

/// Parse a header from the start of a file. `bytes` must reach at least `data_offset`.
pub fn parse_header(bytes: &[u8]) -> Result<FseqHeader> {
    if bytes.len() < 28 {
        return Err(format_err("file is too short to be an FSEQ file"));
    }
    if &bytes[0..4] != b"PSEQ" {
        return Err(format_err("not an FSEQ file: missing PSEQ marker"));
    }
    let data_offset = u16_at(bytes, 4) as u32;
    let minor = bytes[6];
    let major = bytes[7];
    let header_field = u16_at(bytes, 8);
    let channels = u32_at(bytes, 10);
    let frames = u32_at(bytes, 14);
    let step_ms = bytes[18];
    let flags = bytes[19];
    if bytes.len() < data_offset as usize {
        return Err(format_err("file ends before the channel data starts"));
    }

    let mut header = FseqHeader {
        major,
        minor,
        data_offset,
        header_field,
        channels,
        frames,
        step_ms,
        flags,
        compression: Compression::None,
        declared_blocks: 0,
        blocks: Vec::new(),
        sparse: Vec::new(),
        id: [0; 8],
        variable_headers: Vec::new(),
    };

    let variable_start = match major {
        1 => header_field as usize,
        2 => {
            if bytes.len() < 32 {
                return Err(format_err("file is too short for an FSEQ 2 header"));
            }
            header.compression = match bytes[20] & 0x0F {
                0 => Compression::None,
                1 => Compression::Zstd,
                2 => Compression::Zlib,
                other => return Err(format_err(format!("unknown compression type {other}"))),
            };
            header.declared_blocks = bytes[21];
            let sparse_count = bytes[22] as usize;
            header.id.copy_from_slice(&bytes[24..32]);
            let mut at = 32;
            let table_end = at + header.declared_blocks as usize * 8 + sparse_count * 6;
            if table_end > bytes.len() {
                return Err(format_err("block table runs past the header"));
            }
            for _ in 0..header.declared_blocks {
                header.blocks.push((u32_at(bytes, at), u32_at(bytes, at + 4)));
                at += 8;
            }
            for _ in 0..sparse_count {
                header.sparse.push((u24_at(bytes, at), u24_at(bytes, at + 3)));
                at += 6;
            }
            at
        }
        other => return Err(format_err(format!("unsupported FSEQ version {other}.{minor}"))),
    };

    let end = data_offset as usize;
    let mut at = variable_start;
    while at + 4 <= end {
        let len = u16_at(bytes, at) as usize;
        if len < 4 || at + len > end {
            break;
        }
        header.variable_headers.push(VariableHeaderEntry {
            code: [bytes[at + 2], bytes[at + 3]],
            value: bytes[at + 4..at + len].to_vec(),
        });
        at += len;
    }
    Ok(header)
}

/// Decompress one block into `out`, which already has the expected size. Accepts frames with no
/// final block, as the Windows version writes them. Returns the bytes produced.
fn decompress_block(block: &[u8], out: &mut [u8]) -> Result<usize> {
    let mut decoder = Decoder::new()?;
    let mut input = InBuffer::around(block);
    let mut output = OutBuffer::around(out);
    while input.pos < block.len() && output.pos() < output.capacity() {
        let before = (input.pos, output.pos());
        decoder.run(&mut input, &mut output)?;
        if before == (input.pos, output.pos()) {
            break;
        }
    }
    // Everything fed in may still be sitting in the decoder.
    loop {
        let before = output.pos();
        if before == output.capacity() {
            break;
        }
        let mut empty = InBuffer::around(&[]);
        decoder.run(&mut empty, &mut output)?;
        if output.pos() == before {
            break;
        }
    }
    Ok(output.pos())
}

/// An FSEQ file on disk.
pub struct FseqFile {
    pub header: FseqHeader,
    file: File,
    /// `(first frame, file offset, bytes)` of blocks holding data.
    index: Vec<(u32, u64, u32)>,
}

impl FseqFile {
    pub fn open(path: &Path) -> Result<FseqFile> {
        let mut file = File::open(path)?;
        let mut start = [0u8; 6];
        file.read_exact(&mut start).map_err(|_| format_err("file is too short to be an FSEQ file"))?;
        let data_offset = u16::from_le_bytes([start[4], start[5]]) as usize;
        let mut bytes = vec![0u8; data_offset.max(32)];
        file.seek(SeekFrom::Start(0))?;
        let mut filled = 0;
        while filled < bytes.len() {
            let n = file.read(&mut bytes[filled..])?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        bytes.truncate(filled);
        let header = parse_header(&bytes)?;
        let mut index = Vec::new();
        let mut offset = header.data_offset as u64;
        for (frame, len) in &header.blocks {
            if *len > 0 {
                index.push((*frame, offset, *len));
                offset += *len as u64;
            }
        }
        Ok(FseqFile { header, file, index })
    }

    /// Bytes per frame in the file, after any v1 padding is counted in `channels`.
    pub fn frame_size(&self) -> usize {
        self.header.stored_channels() as usize
    }

    /// Decode every frame, concatenated. A short file gives an error rather than a short result,
    /// except for a final block whose frames were never written, which is padded with zeros and
    /// reported by [`frames_decoded`](Self::frames_decoded) when asked for separately.
    pub fn read_all_frames(&mut self) -> Result<Vec<u8>> {
        let (data, decoded) = self.decode_all()?;
        if decoded != self.header.frames as usize {
            return Err(format_err(format!(
                "the header declares {} frames but the data holds {decoded}",
                self.header.frames
            )));
        }
        Ok(data)
    }

    /// How many whole frames the data really holds.
    pub fn frames_decoded(&mut self) -> Result<usize> {
        Ok(self.decode_all()?.1)
    }

    fn decode_all(&mut self) -> Result<(Vec<u8>, usize)> {
        let frame_size = self.frame_size();
        let total_frames = self.header.frames as usize;
        let mut data = vec![0u8; frame_size * total_frames];
        if frame_size == 0 || total_frames == 0 {
            return Ok((data, 0));
        }
        match self.header.compression {
            Compression::None => {
                self.file.seek(SeekFrom::Start(self.header.data_offset as u64))?;
                let mut filled = 0;
                while filled < data.len() {
                    let n = self.file.read(&mut data[filled..])?;
                    if n == 0 {
                        break;
                    }
                    filled += n;
                }
                Ok((data, filled / frame_size))
            }
            Compression::Zlib => Err(format_err("zlib-compressed FSEQ files are not supported")),
            Compression::Zstd => {
                let mut decoded_bytes = 0;
                for i in 0..self.index.len() {
                    let (first_frame, offset, len) = self.index[i];
                    let next_frame = self.index.get(i + 1).map_or(self.header.frames, |b| b.0);
                    if first_frame >= next_frame || first_frame as usize >= total_frames {
                        continue;
                    }
                    let mut block = vec![0u8; len as usize];
                    self.file.seek(SeekFrom::Start(offset))?;
                    self.file.read_exact(&mut block).map_err(|_| format_err("the file ends inside a compression block"))?;
                    let start = first_frame as usize * frame_size;
                    let end = (next_frame as usize).min(total_frames) * frame_size;
                    let produced = decompress_block(&block, &mut data[start..end])?;
                    decoded_bytes = start + produced;
                }
                Ok((data, decoded_bytes / frame_size))
            }
        }
    }
}
