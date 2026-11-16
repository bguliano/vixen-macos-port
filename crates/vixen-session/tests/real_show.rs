//! The whole pipeline against the owner's real show: files, effects, schedule, filters and export. The folder is read, never
//! written, and these tests pass quietly on a machine without it.

use std::path::PathBuf;
use vixen_io::{load_system, DataPaths};
use vixen_session::compare::compare_with_fseq;

const SHOW: &str = "/Users/developer/Documents/Vixen 3";

/// Differences with a known reason, by sequence: `(most values, largest difference)`.
fn allowance(sequence: &str) -> (usize, i32) {
    match sequence {
        // Upstream changed Chase after this file was exported (VIX-2276: start and end points that aren't 0 or 100%). This
        // port follows the current code, so the last pixel of two chases differs, and is correct for the current version.
        "Dance of the Sugar Plum Fairy" => (42, 178),
        // One pixel's RGB values at 117.350/117.375s: the 540ms meter decision is only 0.000053dB above its threshold.
        // Original NAudio mixing + C# filter/ballistics match exactly on identical decoded PCM. Signed16-bit input
        // conversion removes all six, but the exact Windows decoder PCM remains unmeasured. See the audio investigation
        // in docs/review/vertical-meter-audio.md; this remains an allowance, not proof of a particular rounding rule.
        "Coldest Winter" => (6, 38),
        _ => (0, 0),
    }
}

#[test]
fn exports_match_the_original_apart_from_random_effects() {
    let root = PathBuf::from(SHOW);
    if !root.is_dir() {
        return;
    }
    let paths = DataPaths::new(&root);
    let system = load_system(&paths).expect("system config");
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("Export"))
        .expect("export folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "fseq"))
        .collect();
    files.sort();
    assert!(!files.is_empty());

    let mut frames = 0;
    for file in &files {
        let name = file.file_stem().unwrap().to_string_lossy().to_string();
        let result = compare_with_fseq(&system, &paths, &name, file, 5).unwrap_or_else(|e| panic!("{name}: {e}"));
        frames += result.frames;
        let (most, largest) = allowance(&name);
        assert!(
            result.unexplained <= most && result.largest_unexplained <= largest,
            "{name}: {} values differ with no random effect active, largest {}: {:#?}",
            result.unexplained,
            result.largest_unexplained,
            result.unexplained_samples
        );
    }
    eprintln!("compared {frames} frames of {} sequences", files.len());
}

#[test]
fn an_exported_file_matches_the_real_one_in_every_header_field_and_frame() {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use vixen_export::fseq_read::FseqFile;
    use vixen_export::ExportFormat;
    use vixen_io::system::effective_store;
    use vixen_session::export::{ExportRequest, PreparedExport};
    use vixen_session::sequence::{SequenceSession, SessionEnvironment};

    let root = PathBuf::from(SHOW);
    if !root.is_dir() {
        return;
    }
    let paths = DataPaths::new(&root);
    let system = load_system(&paths).expect("system config");
    let env = SessionEnvironment { video_cache: None,
        tree: Arc::new(system.tree.clone()),
        store: Arc::new(effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: system.default_update_interval,
        media_dir: paths.media_dir(),
    };
    // No random effects in this one, so every byte can match.
    let name = "Snoopy Christmas";
    let mut session = SequenceSession::open(&paths.sequence_dir().join(format!("{name}.tim")), &env).unwrap();
    let mut real = FseqFile::open(&root.join("Export").join(format!("{name}.fseq"))).unwrap();
    let real_header = real.header.clone();

    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("session-export");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let file = out.join(format!("{name}.fseq"));
    let request = ExportRequest {
        format: ExportFormat::DEFAULT,
        compress: true,
        out_file_name: file.clone(),
        update_interval_ms: real_header.step_ms as i32,
        audio_filename: real_header.media_file().unwrap_or_default(),
        match_audio_name: false,
        controllers: Vec::new(),
    };
    let prepared = PreparedExport::prepare(&mut session, &system, &request).unwrap();
    let cancel = AtomicBool::new(false);
    let mut last_percent = -1;
    let outcome = prepared
        .run(&cancel, &mut |percent, message| {
            last_percent = percent;
            assert_eq!(message, format!("Exporting {name}"));
        })
        .unwrap();
    assert_eq!(outcome.frames_written as u32, real_header.frames);
    assert!(last_percent >= 99);

    let mut ours = FseqFile::open(&file).unwrap();
    let h = ours.header.clone();
    assert_eq!((h.major, h.minor), (real_header.major, real_header.minor));
    assert_eq!(h.channels, real_header.channels);
    assert_eq!(h.frames, real_header.frames);
    assert_eq!(h.step_ms, real_header.step_ms);
    assert_eq!(h.data_offset, real_header.data_offset);
    assert_eq!(h.declared_blocks, real_header.declared_blocks);
    assert_eq!(h.variable_headers, real_header.variable_headers);
    assert!(ours.read_all_frames().unwrap() == real.read_all_frames().unwrap(), "frames differ");
}

#[test]
fn channel_names_follow_the_color_breakdown() {
    use vixen_session::show::build_output_graph;
    let root = PathBuf::from(SHOW);
    if !root.is_dir() {
        return;
    }
    let system = load_system(&DataPaths::new(&root)).expect("system config");
    let ids: Vec<uuid::Uuid> = system.tree.element_ids().collect();
    let graph = build_output_graph(&system, &ids);
    let name = |element: &uuid::Uuid| {
        system.tree.find_by_element(element).map(|n| system.tree.node(n).name.clone()).unwrap_or_default()
    };
    let strip = graph.controllers().iter().position(|c| c.output_count() > 1000).unwrap();
    let first = graph.channel_name(vixen_output::OutputAddress { controller: strip, output: 0 }, &name).unwrap();
    let second = graph.channel_name(vixen_output::OutputAddress { controller: strip, output: 1 }, &name).unwrap();
    assert!(first.contains("Pixel") && first.ends_with("Red"), "{first}");
    assert!(second.ends_with("Green"), "{second}");
}
