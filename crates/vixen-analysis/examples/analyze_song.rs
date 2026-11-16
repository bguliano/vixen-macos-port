//! Times the whole analysis of one song, the way the session layer would run it.
//!
//!   analyze_song SONG [BEATS_PER_BAR] [--marks]
//!
//! With --marks, each collection's mark times follow its count, in milliseconds.

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use vixen_analysis::{BeatsAndBars, BeatsAndBarsDialogModel, RawSampleFormat};
use vixen_audio::AudioClip;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(song) = args.get(1).map(Path::new) else {
        eprintln!("usage: analyze_song SONG [BEATS_PER_BAR]");
        std::process::exit(1);
    };
    let beats_per_bar = args.get(2).filter(|a| *a != "--marks").map_or(4, |b| b.parse().expect("a number"));
    let print_marks = args.iter().any(|a| a == "--marks");

    let start = Instant::now();
    let clip = AudioClip::load(song).expect("decodable song");
    let decoded = start.elapsed();

    let start = Instant::now();
    let format = RawSampleFormat::for_path(song).expect("readable song");
    let analysis =
        BeatsAndBars::from_interleaved(clip.samples(), format, clip.sample_rate() as f32, clip.channels() as i32);
    let converted = start.elapsed();

    let cancel = AtomicBool::new(false);

    let start = Instant::now();
    let preview = analysis.generate_preview_data(&cancel).expect("preview");
    let previewed = start.elapsed();

    let mut model = BeatsAndBarsDialogModel::default();
    model.set_beats_per_bar(beats_per_bar);
    if print_marks {
        model.set_split_beats(true);
    }
    model.beat_period = preview.beat_period;

    let mut collections = Vec::new();
    let mut finalizing_at = None;
    let start = Instant::now();
    let mut progress = |p: vixen_analysis::AnalysisProgress| {
        if p.is_finalizing {
            finalizing_at = Some(start.elapsed());
        }
    };
    analysis.do_beat_bar_detection(&mut collections, &model.settings(), &mut progress, &cancel).expect("analysis");
    let analysed = start.elapsed();

    let seconds = clip.duration().total_secs();
    println!(
        "{}: {:.1} s of audio at {} Hz, {} channels",
        song.file_name().unwrap().to_string_lossy(),
        seconds,
        clip.sample_rate(),
        clip.channels()
    );
    println!(
        "decode {:.3} s, convert {:.3} s, preview {:.3} s",
        decoded.as_secs_f64(),
        converted.as_secs_f64(),
        previewed.as_secs_f64()
    );
    let blocks = finalizing_at.unwrap_or_default();
    println!(
        "analysis {:.3} s ({:.3} s blocks, {:.3} s finalizing), {:.0} x real time, estimated {:.1} BPM",
        analysed.as_secs_f64(),
        blocks.as_secs_f64(),
        (analysed - blocks).as_secs_f64(),
        seconds / analysed.as_secs_f64(),
        model.bpm()
    );
    for collection in &collections {
        println!("  {:>5} marks  {}", collection.marks.len(), collection.name);
        if print_marks {
            let times: Vec<String> = collection.marks.iter().map(|m| m.start_time.total_ms().to_string()).collect();
            println!("  {}", times.join(" "));
        }
    }
}
