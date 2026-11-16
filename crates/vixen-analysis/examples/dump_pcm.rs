//! Writes the samples the tracker would see for a song, as little-endian f32, for tools/qm-golden to run through the
//! C++. The output must go somewhere other than the song's own folder.
//!
//!   dump_pcm SONG OUT.f32

use std::fs;
use std::path::Path;

use vixen_analysis::{float_samples_from_raw, raw_audio_bytes, RawSampleFormat, BYTES_PER_SAMPLE};
use vixen_audio::AudioClip;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, song, out] = args.as_slice() else {
        eprintln!("usage: dump_pcm SONG OUT.f32");
        std::process::exit(1);
    };
    let song = Path::new(song);

    let clip = AudioClip::load(song).expect("decodable song");
    let format = RawSampleFormat::for_path(song).expect("readable song");
    let samples = float_samples_from_raw(&raw_audio_bytes(clip.samples(), format), BYTES_PER_SAMPLE);

    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    fs::write(out, bytes).expect("writable output");

    println!(
        "{} rate={} channels={} frames={} seconds={:.3} concealed_frames={} tracker_samples={}",
        song.file_name().unwrap().to_string_lossy(),
        clip.sample_rate(),
        clip.channels(),
        clip.frames(),
        clip.duration().total_secs(),
        clip.concealed_frames(),
        samples.len()
    );
}
