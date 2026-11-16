//! Compares the port with the C++ on the owner's real songs, as far as those files are here. They are read only, and only
//! features and hashes are stored in the golden files. On a machine without the folder these tests pass having done
//! nothing.

mod common;

use std::path::{Path, PathBuf};

use common::{load_goldens, run_golden_case, run_in_parallel, Differences};
use vixen_analysis::{float_samples_from_raw, raw_audio_bytes, RawSampleFormat, BYTES_PER_SAMPLE};
use vixen_audio::AudioClip;

const MEDIA: &str = "/Users/developer/Documents/Vixen 3/Media";

fn song_path(golden: &serde_json::Value) -> PathBuf {
    Path::new(MEDIA).join(golden["input"]["note"].as_str().expect("the song's file name"))
}

fn tracker_samples(path: &Path) -> Vec<f32> {
    let clip = AudioClip::load(path).expect("decodable song");
    let format = RawSampleFormat::for_path(path).expect("readable song");
    float_samples_from_raw(&raw_audio_bytes(clip.samples(), format), BYTES_PER_SAMPLE)
}

#[test]
fn real_music_matches_the_cpp() {
    if !Path::new(MEDIA).is_dir() {
        return;
    }

    let cases = load_goldens(|g| g["input"]["kind"] == "raw");
    assert!(cases.len() >= 2, "only {} real-music goldens found", cases.len());

    let results = run_in_parallel(&cases, |_, golden| {
        let path = song_path(golden);
        if !path.is_file() {
            return (Differences::default(), Vec::new());
        }
        run_golden_case(golden, &tracker_samples(&path))
    });

    let mut total = Differences::default();
    let mut failures = Vec::new();
    for (name, (diff, errors)) in &results {
        println!(
            "{name}: {} features, largest timestamp difference {} ns, largest value difference {:e}",
            diff.features, diff.max_timestamp_ns, diff.max_value_abs
        );
        total.merge(*diff);
        if !errors.is_empty() {
            let shown: Vec<&str> = errors.iter().take(4).map(String::as_str).collect();
            failures.push(format!("{name}: {} mismatches, first: {}", errors.len(), shown.join("; ")));
        }
    }

    println!(
        "{} cases, {} features: largest timestamp difference {} ns, largest value difference {:e}",
        results.len(),
        total.features,
        total.max_timestamp_ns,
        total.max_value_abs
    );
    assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), results.len(), failures.join("\n"));
}
