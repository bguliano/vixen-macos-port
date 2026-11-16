//! The owner's real sequences hold marks that real Vixen generated on Windows from the same MP3s, so this checks the
//! whole path, from `AudioClip` to the mark times, against the application itself rather than against a harness. The
//! files are read only. On a machine without them these tests pass having done nothing.
//!
//! Only sequences analysed under the Media Foundation behaviour `AudioClip` reproduces are pinned here. Marks made
//! earlier match exactly too, but only with the audio moved to where the decoder of that time put it, which
//! this port has no reason to reproduce.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use vixen_analysis::{BeatsAndBars, BeatsAndBarsDialogModel, RawSampleFormat};
use vixen_audio::AudioClip;
use vixen_core::{MarkCollection, TimeSpan};

const SHOW: &str = "/Users/developer/Documents/Vixen 3";

/// Start times of the named mark collection in a sequence file, from the `DataContractSerializer` text. Marks come
/// before the name within each collection.
fn stored_marks(sequence: &str, name: &str) -> Option<Vec<TimeSpan>> {
    let name_at = sequence.find(&format!(">{name}</"))?;
    let collection_start = sequence[..name_at].rfind(":MarkCollection\"")?;
    let mut rest = &sequence[collection_start..name_at];
    let mut times = Vec::new();
    while let Some(open) = rest.find("StartTime>") {
        let after = &rest[open + "StartTime>".len()..];
        let close = after.find('<')?;
        times.push(TimeSpan::parse_xml(&after[..close])?);
        let tag_end = after[close..].find('>')?;
        rest = &after[close + tag_end + 1..];
    }
    Some(times)
}

fn analyse(song: &Path) -> Vec<MarkCollection> {
    let clip = AudioClip::load(song).expect("decodable song");
    let format = RawSampleFormat::for_path(song).expect("readable song");
    let analysis =
        BeatsAndBars::from_interleaved(clip.samples(), format, clip.sample_rate() as f32, clip.channels() as i32);
    let mut collections = Vec::new();
    let settings = BeatsAndBarsDialogModel::default().settings();
    analysis
        .do_beat_bar_detection(&mut collections, &settings, &mut |_| {}, &AtomicBool::new(false))
        .expect("analysis");
    collections
}

fn compare(sequence_file: &str, song_file: &str) -> Option<(usize, usize)> {
    let show = Path::new(SHOW);
    let sequence_path = show.join("Sequence").join(sequence_file);
    let song_path = show.join("Media").join(song_file);
    if !sequence_path.is_file() || !song_path.is_file() {
        return None;
    }

    let sequence = std::fs::read_to_string(&sequence_path).expect("readable sequence");
    let collections = analyse(&song_path);

    let (mut total, mut matching) = (0, 0);
    for name in ["Beats 1/4 Beat #1 (Bar)", "Beats Beat #2", "Beats Beat #3", "Beats Beat #4"] {
        let stored = stored_marks(&sequence, name).expect("the collection is in the sequence");
        let ours = collections.iter().find(|c| c.name == name).expect("the port made this collection");
        total += stored.len();
        matching += stored.iter().filter(|t| ours.marks.iter().any(|m| m.start_time == **t)).count();
        assert_eq!(ours.marks.iter().filter(|m| m.duration == TimeSpan::from_ms_i(450)).count(), ours.marks.len());
    }
    println!("{sequence_file}: {matching} of {total} stored marks reproduced to the tick");
    Some((matching, total))
}

#[test]
fn coldest_winter_reproduces_the_marks_vixen_stored() {
    if let Some((matching, total)) = compare("Coldest Winter.tim", "Coldest Winter Pentatonix.mp3") {
        assert_eq!(matching, total);
    }
}

#[test]
fn sugar_plum_fairy_reproduces_all_but_one_stored_mark() {
    if let Some((matching, total)) = compare("Dance of the Sugar Plum Fairy.tim", "dance of the sugar plum fairy.mp3") {
        assert!(total - matching <= 1, "{matching} of {total}");
    }
}

#[test]
fn stored_marks_parser_reads_durations_style_times() {
    let text = "<a i:type=\"d2p1:MarkCollection\"><b:StartTime>PT1.5S</b:StartTime><b:StartTime>PT3M1.487S</b:StartTime>\
                <b:Name>Other</b:Name></a><a i:type=\"d2p1:MarkCollection\"><b:StartTime>PT2S</b:StartTime><b:Name>X</b:Name></a>";
    assert_eq!(stored_marks(text, "Other").unwrap(), vec![TimeSpan::from_ms_i(1500), TimeSpan::from_ms_i(181_487)]);
    assert_eq!(stored_marks(text, "X").unwrap(), vec![TimeSpan::from_ms_i(2000)]);
    assert!(stored_marks(text, "Missing").is_none());
}
