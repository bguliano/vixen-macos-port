//! Reads the FSEQ files the Windows version exported, and rewrites them with this port's writer. The
//! folder is the owner's real show: opened read-only, and the tests pass without it on other machines.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use vixen_core::TimeSpan;
use vixen_export::fseq::FseqV2Writer;
use vixen_export::fseq_read::{Compression, FseqFile};
use vixen_export::run::{run_export, ExportJob, IntervalSource};

const EXPORT_DIR: &str = "/Users/developer/Documents/Vixen 3/Export";

fn real_files() -> Vec<PathBuf> {
    let Ok(dir) = std::fs::read_dir(EXPORT_DIR) else { return Vec::new() };
    let mut files: Vec<PathBuf> =
        dir.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "fseq")).collect();
    files.sort();
    files
}

/// Plays back already decoded frames.
struct Replay {
    data: Vec<u8>,
    frame_size: usize,
    step_ms: f64,
}

impl IntervalSource for Replay {
    fn sample(&mut self, time: TimeSpan, row: &mut [u8]) {
        let frame = (time.total_ms() / self.step_ms).round() as usize;
        row.copy_from_slice(&self.data[frame * self.frame_size..(frame + 1) * self.frame_size]);
    }
}

#[test]
fn every_real_export_decodes_and_rewrites_identically() {
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("export-real");
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();

    let files = real_files();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut original = FseqFile::open(path).unwrap_or_else(|e| panic!("{name}: {e}"));
        let h = original.header.clone();
        assert_eq!((h.major, h.minor), (2, 0), "{name}");
        assert_eq!(h.compression, Compression::Zstd, "{name}");
        assert_eq!(h.flags, 0, "{name}");
        // No gaps, overlaps or spare entries in the table.
        assert_eq!(h.blocks.len(), h.declared_blocks as usize, "{name}");
        assert_eq!(h.blocks[0].0, 0, "{name}");
        let decoded = original.read_all_frames().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(decoded.len(), h.frames as usize * h.channels as usize, "{name}");

        // Rewrite with the same inputs the original export had.
        let rewritten = out_dir.join(&name);
        let job = ExportJob {
            sequence_name: name.clone(),
            sequence_length: TimeSpan::from_ms_i(h.frames as i64 * h.step_ms as i64),
            out_file_name: rewritten.clone(),
            update_interval_ms: h.step_ms as i32,
            channel_names: vec![String::new(); h.channels as usize],
            audio_filename: h.media_file().unwrap_or_default(),
            match_audio_name: false,
        };
        let mut source = Replay { data: decoded.clone(), frame_size: h.channels as usize, step_ms: h.step_ms as f64 };
        let cancel = AtomicBool::new(false);
        let mut writer = FseqV2Writer::new(true);
        run_export(&job, &mut writer, &mut source, &cancel, &mut |_, _| {}).unwrap();

        let mut ours = FseqFile::open(&rewritten).unwrap();
        let o = ours.header.clone();
        assert_eq!(o.data_offset, h.data_offset, "{name}: channel data offset");
        assert_eq!(o.header_field, h.header_field, "{name}: variable header offset");
        assert_eq!(o.channels, h.channels, "{name}");
        assert_eq!(o.frames, h.frames, "{name}");
        assert_eq!(o.step_ms, h.step_ms, "{name}");
        assert_eq!(o.declared_blocks, h.declared_blocks, "{name}");
        assert_eq!(o.variable_headers, h.variable_headers, "{name}");
        let our_starts: Vec<u32> = o.blocks.iter().map(|b| b.0).collect();
        let their_starts: Vec<u32> = h.blocks.iter().map(|b| b.0).collect();
        assert_eq!(our_starts, their_starts, "{name}: block boundaries");
        assert!(ours.read_all_frames().unwrap() == decoded, "{name}: frames differ");
    }
    eprintln!("checked {} real FSEQ files", files.len());
}
