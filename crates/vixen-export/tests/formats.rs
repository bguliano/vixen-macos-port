//! Each format, end to end through `run_export`, with a deterministic source.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use vixen_core::TimeSpan;
use vixen_export::fseq::FseqV2Writer;
use vixen_export::fseq_read::{parse_header, Compression, FseqFile};
use vixen_export::run::{run_export, ExportJob, IntervalSource};
use vixen_export::{ExportFormat, ExportWriter};

struct Ramp;

impl IntervalSource for Ramp {
    fn sample(&mut self, time: TimeSpan, row: &mut [u8]) {
        let frame = (time.total_ms() / 25.0) as usize;
        for (i, b) in row.iter_mut().enumerate() {
            *b = (frame.wrapping_mul(7).wrapping_add(i.wrapping_mul(3)) % 256) as u8;
        }
    }
}

fn expected_frames(frames: usize, channels: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(frames * channels);
    for frame in 0..frames {
        for i in 0..channels {
            data.push((frame.wrapping_mul(7).wrapping_add(i.wrapping_mul(3)) % 256) as u8);
        }
    }
    data
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("export").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn job(path: &Path, channels: usize, length_ms: i64) -> ExportJob {
    ExportJob {
        sequence_name: "Test".into(),
        sequence_length: TimeSpan::from_ms_i(length_ms),
        out_file_name: path.to_path_buf(),
        update_interval_ms: 25,
        channel_names: (0..channels).map(|i| format!("Channel {i}")).collect(),
        audio_filename: "/music/Song Name.mp3".into(),
        match_audio_name: false,
    }
}

fn export(job: &ExportJob, writer: &mut dyn ExportWriter) -> i32 {
    let cancel = AtomicBool::new(false);
    run_export(job, writer, &mut Ramp, &cancel, &mut |_, _| {}).unwrap().frames_written
}

#[test]
fn fseq_v2_compressed_round_trip() {
    let dir = temp_dir("v2");
    let path = dir.join("a.fseq");
    let j = job(&path, 3367, 25 * 5946);
    assert_eq!(export(&j, &mut FseqV2Writer::new(true)), 5946);

    let mut file = FseqFile::open(&path).unwrap();
    let h = file.header.clone();
    assert_eq!((h.major, h.minor), (2, 0));
    assert_eq!(h.channels, 3367);
    assert_eq!(h.frames, 5946);
    assert_eq!(h.step_ms, 25);
    assert_eq!(h.flags, 0);
    assert_eq!(h.compression, Compression::Zstd);
    // The same layout the real Windows export of this size has.
    assert_eq!(h.declared_blocks, 249);
    assert_eq!(h.header_field, 32 + 249 * 8);
    assert_eq!(h.blocks[0].0, 0);
    assert_eq!(h.blocks[1].0, 10);
    assert_eq!(h.blocks[2].0, 34);
    assert_eq!(h.blocks[248].0, 5938);
    assert_eq!(h.media_file().as_deref(), Some("Song Name.mp3"));
    assert_eq!(h.variable_headers[1].text(), "Vixen 3");
    assert_eq!(h.data_offset as usize, 32 + 249 * 8 + (13 + 5) + 12);
    assert_eq!(file.read_all_frames().unwrap(), expected_frames(5946, 3367));

    // Every block is a complete frame, so a one-shot decoder accepts it too.
    let bytes = std::fs::read(&path).unwrap();
    let mut offset = h.data_offset as usize;
    for (_, len) in &h.blocks {
        let block = &bytes[offset..offset + *len as usize];
        zstd::stream::decode_all(block).expect("complete frame");
        offset += *len as usize;
    }
    assert_eq!(offset, bytes.len());
}

#[test]
fn fseq_v2_uncompressed_keeps_the_block_table() {
    let dir = temp_dir("v2raw");
    let path = dir.join("a.fseq");
    let j = job(&path, 16, 25 * 100);
    assert_eq!(export(&j, &mut FseqV2Writer::new(false)), 100);
    let mut file = FseqFile::open(&path).unwrap();
    assert_eq!(file.header.compression, Compression::None);
    assert!(file.header.declared_blocks > 0);
    assert_eq!(file.header.blocks[0], (0, 16 * 10));
    assert_eq!(file.read_all_frames().unwrap(), expected_frames(100, 16));
}

#[test]
fn fseq_v2_short_sequences_are_readable() {
    let dir = temp_dir("v2short");
    for frames in [0usize, 1, 9, 10, 11, 34, 35] {
        let path = dir.join(format!("{frames}.fseq"));
        let j = job(&path, 8, 25 * frames as i64);
        assert_eq!(export(&j, &mut FseqV2Writer::new(true)) as usize, frames);
        let mut file = FseqFile::open(&path).unwrap();
        assert_eq!(file.header.frames as usize, frames);
        assert_eq!(file.header.declared_blocks as usize, file.header.blocks.len());
        assert_eq!(file.read_all_frames().unwrap(), expected_frames(frames, 8), "{frames} frames");
    }
}

#[test]
fn fseq_v1_pads_to_four_channels() {
    let dir = temp_dir("v1");
    let path = dir.join("a.fseq");
    let j = job(&path, 7, 25 * 3);
    let mut w = ExportFormat::FseqV1.writer(false);
    assert_eq!(export(&j, w.as_mut()), 3);
    let bytes = std::fs::read(&path).unwrap();
    let h = parse_header(&bytes).unwrap();
    assert_eq!((h.major, h.minor), (1, 0));
    assert_eq!(h.channels, 8);
    assert_eq!(h.frames, 3);
    assert_eq!(h.header_field, 28);
    assert_eq!(bytes[18], 25);
    assert_eq!(&bytes[20..28], &[0, 0, 0, 0, 1, 2, 0, 0]);
    assert_eq!(h.media_file().as_deref(), Some("Song Name.mp3"));
    assert_eq!(bytes.len(), h.data_offset as usize + 3 * 8);
    let data = &bytes[h.data_offset as usize..];
    assert_eq!(&data[..7], &expected_frames(1, 7)[..]);
    assert_eq!(data[7], 0);
}

#[test]
fn eseq_layout() {
    let dir = temp_dir("eseq");
    let path = dir.join("a.eseq");
    let j = job(&path, 6, 25 * 2);
    let mut w = ExportFormat::Eseq.writer(false);
    assert_eq!(export(&j, w.as_mut()), 2);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..4], b"ESEQ");
    assert_eq!(&bytes[4..8], &[1, 0, 0, 0]);
    assert_eq!(&bytes[8..12], &[8, 0, 0, 0]);
    assert_eq!(&bytes[12..16], &[0, 0, 0, 0]);
    assert_eq!(&bytes[16..20], &[8, 0, 0, 0]);
    assert_eq!(bytes.len(), 20 + 2 * 8);
}

#[test]
fn csv_rows() {
    let dir = temp_dir("csv");
    let path = dir.join("a.csv");
    let j = job(&path, 3, 25 * 2);
    let mut w = ExportFormat::Csv.writer(false);
    export(&j, w.as_mut());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "000,003,006\r\n007,010,013\r\n");
}

#[test]
fn vir_is_channel_major() {
    let dir = temp_dir("vir");
    let path = dir.join("a.vir");
    let j = job(&path, 2, 25 * 3);
    let mut w = ExportFormat::Vir2.writer(false);
    export(&j, w.as_mut());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "0 7 14 \r\n3 10 17 \r\n");
}

#[test]
fn vix_encodes_channel_major_values() {
    use base64::Engine;
    let dir = temp_dir("vix");
    let path = dir.join("a.vix");
    let j = job(&path, 2, 25 * 3);
    let mut w = ExportFormat::Vix2.writer(false);
    export(&j, w.as_mut());
    let doc = std::fs::read_to_string(&path).unwrap();
    assert!(doc.starts_with("\u{FEFF}<?xml"));
    let start = doc.find("<EventValues>").unwrap() + 13;
    let end = doc.find("</EventValues>").unwrap();
    let values = base64::engine::general_purpose::STANDARD.decode(&doc[start..end]).unwrap();
    assert_eq!(values, [0, 7, 14, 3, 10, 17]);
    assert!(doc.contains("<Audio filename=\"Song Name.mp3\" duration=\"25\">Song Name.mp3</Audio>"));
}

#[test]
fn partial_interval_at_the_end_is_not_sampled_but_vix_leaves_room() {
    let dir = temp_dir("partial");
    let path = dir.join("a.vix");
    let j = job(&path, 1, 60);
    assert_eq!(j.period_count(), 2);
    let mut w = ExportFormat::Vix2.writer(false);
    assert_eq!(export(&j, w.as_mut()), 2);
    let doc = std::fs::read_to_string(&path).unwrap();
    // Two values and a zero for the period that doesn't fit: "AAcA" is 0, 7, 0.
    assert!(doc.contains("<EventValues>AAcA</EventValues>"));
}

#[test]
fn matching_audio_name_uses_the_sequence_name() {
    let dir = temp_dir("match");
    let mut j = job(&dir.join("a.fseq"), 4, 25);
    assert_eq!(j.session_data().output_audio_file_name, "Song Name.mp3");
    j.match_audio_name = true;
    assert_eq!(j.session_data().output_audio_file_name, "Test.mp3");
    j.audio_filename.clear();
    assert_eq!(j.session_data().output_audio_file_name, "");
}

#[test]
fn cancelling_stops_early_and_leaves_no_file() {
    let dir = temp_dir("cancel");
    let path = dir.join("a.csv");
    let j = job(&path, 2, 25 * 100);
    let cancel = AtomicBool::new(false);
    struct Stop<'a>(&'a AtomicBool, usize);
    impl IntervalSource for Stop<'_> {
        fn sample(&mut self, _: TimeSpan, row: &mut [u8]) {
            row.fill(1);
            self.1 += 1;
            if self.1 == 3 {
                self.0.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    let mut w = ExportFormat::Csv.writer(false);
    let outcome = run_export(&j, w.as_mut(), &mut Stop(&cancel, 0), &cancel, &mut |_, _| {}).unwrap();
    assert!(outcome.cancelled);
    assert_eq!(outcome.frames_written, 3);
    // Upstream leaves the frames it got to. The rule here is `.part` and rename, so the target is only ever a finished file.
    assert!(!path.exists());
}

#[test]
fn progress_counts_up_with_the_task_message() {
    let dir = temp_dir("progress");
    let j = job(&dir.join("a.csv"), 1, 25 * 4);
    let mut seen = Vec::new();
    let cancel = AtomicBool::new(false);
    let mut w = ExportFormat::Csv.writer(false);
    run_export(&j, w.as_mut(), &mut Ramp, &cancel, &mut |p, m| seen.push((p, m.to_string()))).unwrap();
    // Reports are at most 20 a second, so a run this short only makes the first.
    let percents: Vec<i32> = seen.iter().map(|s| s.0).collect();
    assert_eq!(percents, [0]);
    assert_eq!(seen[0].1, "Exporting Test");
}
