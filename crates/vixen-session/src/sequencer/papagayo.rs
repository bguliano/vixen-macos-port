//! Papagayo's `.pgo` files: `PapagayoDoc`. Sequencer F404, marks F87. Line based, with leading whitespace trimmed. A voice keeps its
//! phrases, words and phonemes as written (the mark import, marks F32, reads those) and the coalesced runs (the clipboard import, F88).

use std::collections::HashSet;
use vixen_core::marks::{Mark, MarkCollection, MarkCollectionType, MarkDecorator};
use vixen_core::{Color, TimeSpan};
use vixen_io::marks_files::set_mark_duration;

/// `PhonemeType`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PhonemeType {
    Ai,
    E,
    O,
    U,
    Fv,
    L,
    Mbp,
    Wq,
    Etc,
    Rest,
}

impl PhonemeType {
    const ALL: [PhonemeType; 10] = [
        PhonemeType::Ai,
        PhonemeType::E,
        PhonemeType::O,
        PhonemeType::U,
        PhonemeType::Fv,
        PhonemeType::L,
        PhonemeType::Mbp,
        PhonemeType::Wq,
        PhonemeType::Etc,
        PhonemeType::Rest,
    ];

    /// `Enum.Parse`, case-sensitive, with anything it rejects being `Rest`. KEEP: it also takes a member's number, so `3` is `U`.
    fn parse(name: &str) -> PhonemeType {
        let name = name.trim();
        if let Some(found) = PhonemeType::ALL.iter().find(|t| t.name() == name) {
            return *found;
        }
        name.parse::<usize>().ok().and_then(|i| PhonemeType::ALL.get(i).copied()).unwrap_or(PhonemeType::Rest)
    }

    /// The name `LipSyncData.StaticPhoneme` holds, and `TypeName`.
    pub fn name(self) -> &'static str {
        match self {
            PhonemeType::Ai => "AI",
            PhonemeType::E => "E",
            PhonemeType::O => "O",
            PhonemeType::U => "U",
            PhonemeType::Fv => "FV",
            PhonemeType::L => "L",
            PhonemeType::Mbp => "MBP",
            PhonemeType::Wq => "WQ",
            PhonemeType::Etc => "etc",
            PhonemeType::Rest => "Rest",
        }
    }
}

/// `SampleDurationMS`. Upstream's times are `float`, and so are these, so marks and effects land on the same tick as on Windows.
fn sample_ms(fps: i32) -> f32 {
    (1.0f32 / fps as f32) * 1000.0
}

/// `StartMS`.
fn start_ms(fps: i32, start_frame: i32) -> f32 {
    sample_ms(fps) * start_frame as f32
}

/// `EndMS`.
fn end_ms(fps: i32, end_frame: i32) -> f32 {
    sample_ms(fps) * (end_frame + 1) as f32
}

/// A phoneme: one as written in a word, or one coalesced run of a voice, which is what `PhonemeList` hands the clipboard import.
#[derive(Clone, PartialEq, Debug)]
pub struct PapagayoPhoneme {
    pub phoneme: PhonemeType,
    /// The name as written. `Rest` when the line couldn't be read.
    pub text: String,
    /// `LyricData`: the word it belongs to.
    pub word: String,
    pub start_frame: i32,
    pub end_frame: i32,
    /// `StartMS`: sample × start frame.
    pub start_ms: f32,
    /// `DurationMS`: end − start, where end is sample × (end frame + 1).
    pub duration_ms: f32,
}

impl PapagayoPhoneme {
    /// `new PapagayoPhoneme(null, null)` at a frame: what a frame nothing covers gets.
    fn rest_at(frame: i32) -> PapagayoPhoneme {
        PapagayoPhoneme { phoneme: PhonemeType::Rest, text: "Rest".to_string(), word: String::new(), start_frame: frame, end_frame: frame, start_ms: 0.0, duration_ms: 0.0 }
    }

    fn contains(&self, frame: i32) -> bool {
        frame >= self.start_frame && frame <= self.end_frame
    }

    fn set_times(&mut self, fps: i32) {
        self.start_ms = start_ms(fps, self.start_frame);
        self.duration_ms = end_ms(fps, self.end_frame) - self.start_ms;
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct PapagayoWord {
    pub text: String,
    pub start_frame: i32,
    pub end_frame: i32,
    pub phonemes: Vec<PapagayoPhoneme>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct PapagayoPhrase {
    pub text: String,
    pub start_frame: i32,
    pub end_frame: i32,
    pub words: Vec<PapagayoWord>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct PapagayoVoice {
    /// As written, trailing spaces included. Only the dictionary key is trimmed upstream.
    pub name: String,
    pub phrase_text: String,
    pub phrases: Vec<PapagayoPhrase>,
    /// `PhonemeList`: the coalesced runs.
    pub phonemes: Vec<PapagayoPhoneme>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct PapagayoDoc {
    pub sound_path: String,
    pub fps: i32,
    pub sound_frames: i32,
    pub voices: Vec<PapagayoVoice>,
}

const CORRUPT: &str = "Corrupt File Format";

struct Lines<'a> {
    lines: std::str::Lines<'a>,
}

impl<'a> Lines<'a> {
    /// `file.ReadLine()`, trimmed at the start.
    fn next(&mut self) -> Option<&'a str> {
        self.lines.next().map(str::trim_start)
    }

    /// A line that must be there.
    fn require(&mut self) -> Result<&'a str, String> {
        self.next().ok_or_else(|| CORRUPT.to_string())
    }

    fn require_int(&mut self) -> Result<i32, String> {
        int(self.require()?)
    }

    /// A count sizes an array upstream, so a negative one throws.
    fn require_count(&mut self) -> Result<i32, String> {
        count(self.require_int()?)
    }
}

/// `Convert.ToInt32(string)`: surrounding whitespace and a sign are fine.
fn int(text: &str) -> Result<i32, String> {
    text.trim().parse().map_err(|_| CORRUPT.to_string())
}

fn count(n: i32) -> Result<i32, String> {
    if n < 0 {
        Err(CORRUPT.to_string())
    } else {
        Ok(n)
    }
}

/// `PapagayoDoc.Load`. Errors come back to show, where upstream's menu handler doesn't catch them (GUARD F404).
pub fn parse_papagayo(text: &str) -> Result<PapagayoDoc, String> {
    // `ReadLine` ends a line at a lone carriage return too.
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = Lines { lines: text.lines() };
    let mut doc = PapagayoDoc::default();

    // KEEP: the header loop just stops at the end of the file, so an empty file, or one that ends before the voice count, is a document
    // with no voices.
    let Some(header) = lines.next() else { return Ok(doc) };
    if !header.contains("lipsync") {
        return Err("Invalid File Format".to_string());
    }
    let Some(sound_path) = lines.next() else { return Ok(doc) };
    doc.sound_path = sound_path.to_string();
    let Some(fps) = lines.next() else { return Ok(doc) };
    doc.fps = int(fps)?;
    let Some(sound_frames) = lines.next() else { return Ok(doc) };
    doc.sound_frames = int(sound_frames)?;
    let Some(voice_count) = lines.next() else { return Ok(doc) };
    let voice_count = int(voice_count)?;
    // GUARD: a frame rate of zero makes every time infinite or not a number upstream, which then throws.
    if doc.fps <= 0 && voice_count > 0 {
        return Err("The frame rate must be above zero".to_string());
    }

    let mut names = HashSet::new();
    for _ in 0..voice_count {
        let voice = read_voice(&mut lines, doc.fps, doc.sound_frames)?;
        // The original's dictionary throws on a duplicate key, and the key is the trimmed name.
        if !names.insert(voice.name.trim().to_string()) {
            return Err(format!("Duplicate voice name: {}", voice.name.trim()));
        }
        doc.voices.push(voice);
    }
    // KEEP: any line after the voices throws, a blank one included.
    if lines.next().is_some() {
        return Err("Extra data at the end of the file".to_string());
    }
    Ok(doc)
}

/// `PapagayoVoice.Load`.
fn read_voice(lines: &mut Lines, fps: i32, sound_frames: i32) -> Result<PapagayoVoice, String> {
    let name = lines.require()?.to_string();
    let phrase_text = lines.require()?.replace('|', "\n");
    let phrase_count = lines.require_count()?;
    let mut phrases = Vec::new();
    for _ in 0..phrase_count {
        phrases.push(read_phrase(lines, fps)?);
    }
    let phonemes = coalesce(&phrases, fps, sound_frames);
    Ok(PapagayoVoice { name, phrase_text, phrases, phonemes })
}

/// `PapagayoPhrase.Load`.
fn read_phrase(lines: &mut Lines, fps: i32) -> Result<PapagayoPhrase, String> {
    let text = lines.require()?.to_string();
    let start_frame = lines.require_int()?;
    let end_frame = lines.require_int()?;
    let word_count = lines.require_count()?;
    let mut words = Vec::new();
    for _ in 0..word_count {
        if let Some(word) = read_word(lines, fps)? {
            words.push(word);
        }
    }
    Ok(PapagayoPhrase { text, start_frame, end_frame, words })
}

/// The `PapagayoWord` constructor. A word line with other than four fields, split at single spaces, has no phoneme list upstream and
/// crashes whatever reads it; GUARD: the word is left out.
fn read_word(lines: &mut Lines, fps: i32) -> Result<Option<PapagayoWord>, String> {
    let fields: Vec<&str> = lines.require()?.split(' ').collect();
    if fields.len() != 4 {
        return Ok(None);
    }
    let text = fields[0].trim_start().to_string();
    let start_frame = int(fields[1])?;
    let end_frame = int(fields[2])?;
    let phoneme_count = count(int(fields[3])?)?;
    let mut phonemes: Vec<PapagayoPhoneme> = Vec::new();
    for _ in 0..phoneme_count {
        // GUARD: the file ending inside the list leaves empty slots upstream, which crash later. Those phonemes are just absent here.
        let Some(line) = lines.next() else { break };
        let pair: Vec<&str> = line.split(' ').collect();
        let mut phoneme = PapagayoPhoneme { word: text.clone(), ..PapagayoPhoneme::rest_at(0) };
        // A line without exactly two fields stays `Rest` at frame 0 and doesn't end the one before it.
        if pair.len() == 2 {
            phoneme.start_frame = int(pair[0])?;
            phoneme.text = pair[1].trim_start().to_string();
            phoneme.phoneme = PhonemeType::parse(&phoneme.text);
            if let Some(last) = phonemes.last_mut() {
                last.end_frame = phoneme.start_frame - 1;
            }
        }
        phonemes.push(phoneme);
    }
    if let Some(last) = phonemes.last_mut() {
        last.end_frame = end_frame;
    }
    for phoneme in &mut phonemes {
        phoneme.set_times(fps);
    }
    Ok(Some(PapagayoWord { text, start_frame, end_frame, phonemes }))
}

/// `PapagayoVoice.GetEventPhoneme`: the first phrase holding the frame, then its first word holding it, then that word's first phoneme
/// holding it. Anything missing on the way is a `Rest` for just that frame.
fn event_phoneme(phrases: &[PapagayoPhrase], frame: i32) -> PapagayoPhoneme {
    phrases
        .iter()
        .find(|p| frame >= p.start_frame && frame <= p.end_frame)
        .and_then(|phrase| phrase.words.iter().find(|w| frame >= w.start_frame && frame <= w.end_frame))
        .and_then(|word| word.phonemes.iter().find(|p| p.contains(frame)))
        .cloned()
        .unwrap_or_else(|| PapagayoPhoneme::rest_at(frame))
}

/// `Coalesce`: walks every sound frame and merges neighbours of one type. KEEP: a run keeps its first phoneme's start, text and word and
/// takes the last one's own end frame, which can lie past the last sound frame.
fn coalesce(phrases: &[PapagayoPhrase], fps: i32, sound_frames: i32) -> Vec<PapagayoPhoneme> {
    let mut runs: Vec<PapagayoPhoneme> = Vec::new();
    for frame in 0..sound_frames {
        let phoneme = event_phoneme(phrases, frame);
        match runs.last_mut() {
            Some(run) if run.phoneme == phoneme.phoneme => run.end_frame = phoneme.end_frame,
            _ => runs.push(phoneme),
        }
    }
    for run in &mut runs {
        run.set_times(fps);
    }
    runs
}

/// The alert after the clipboard import: sequencer F91, marks F88. Every row ends in a newline.
pub fn import_summary(voices: &[String]) -> String {
    let mut text = format!("{} Voices imported to clipboard as separate rows\n\n", voices.len());
    for (i, name) in voices.iter().enumerate() {
        text.push_str(&format!("Row #{} - {name}\n", i + 1));
    }
    text
}

/// The alert after the mark import, marks F32, which unlike the other has no newline at the end.
pub fn marks_import_summary(doc: &PapagayoDoc) -> String {
    let rows: Vec<String> = doc.voices.iter().enumerate().map(|(i, v)| format!("Row #{} - {}", i + 1, v.name)).collect();
    format!("{} voices imported as Mark Collections\n\n{}", doc.voices.len(), rows.join("\n"))
}

/// `new Mark(start) { Duration = duration, Text = text }` from `float` milliseconds.
fn mark(start: f32, duration: f32, text: &str) -> Mark {
    let mut mark = Mark::new(TimeSpan::from_ms(start as f64));
    set_mark_duration(&mut mark, TimeSpan::from_ms(duration as f64));
    mark.text = text.to_string();
    mark
}

/// `MaterializePapagayoVoice`, marks F32: a voice's phrases, its words linked to them, and its phonemes as written linked to the words.
pub fn voice_collections(doc: &PapagayoDoc, file_stem: &str, voice: &PapagayoVoice) -> Vec<MarkCollection> {
    let make = |color: Color, suffix: &str, kind: MarkCollectionType, parent: Option<&MarkCollection>| MarkCollection {
        name: format!("{file_stem} {} {suffix}", voice.name),
        decorator: MarkDecorator { color, ..MarkDecorator::default() },
        show_marks: true,
        collection_type: kind,
        linked_mark_collection_id: parent.map(|p| p.id),
        ..MarkCollection::default()
    };
    let mut phrases = make(Color::rgb(205, 242, 162), "Phrases", MarkCollectionType::Phrase, None);
    let mut words = make(Color::rgb(242, 205, 162), "Words", MarkCollectionType::Word, Some(&phrases));
    let mut phonemes = make(Color::rgb(235, 185, 210), "Phonemes", MarkCollectionType::Phoneme, Some(&words));
    let span = |start_frame: i32, end_frame: i32| {
        let start = start_ms(doc.fps, start_frame);
        (start, end_ms(doc.fps, end_frame) - start)
    };
    for phrase in &voice.phrases {
        let (start, duration) = span(phrase.start_frame, phrase.end_frame);
        phrases.marks.push(mark(start, duration, &phrase.text));
        for word in &phrase.words {
            let (start, duration) = span(word.start_frame, word.end_frame);
            words.marks.push(mark(start, duration, &word.text));
            for phoneme in &word.phonemes {
                phonemes.marks.push(mark(phoneme.start_ms, phoneme.duration_ms, phoneme.phoneme.name()));
            }
        }
    }
    let mut out = vec![phrases, words, phonemes];
    for collection in &mut out {
        // `AddMark` keeps the list ordered by start, then by end.
        collection.marks.sort_by_key(|m| (m.start_time, m.end_time()));
    }
    out
}

/// Every voice's collections, in file order.
pub fn mark_collections(doc: &PapagayoDoc, file_stem: &str) -> Vec<MarkCollection> {
    doc.voices.iter().flat_map(|voice| voice_collections(doc, file_stem, voice)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "lipsync version 1\n\
        /tmp/song.wav\n\
        24\n\
        12\n\
        2\n\
        \tVoice One\n\
        \thello|world\n\
        \t1\n\
        \t\thello\n\
        \t\t0\n\
        \t\t8\n\
        \t\t1\n\
        \t\t\thello 2 7 3\n\
        \t\t\t\t2 E\n\
        \t\t\t\t4 E\n\
        \t\t\t\t6 XX\n\
        \tVoice Two\n\
        \tyo\n\
        \t0\n";

    #[test]
    fn parses_voices_and_coalesces() {
        let doc = parse_papagayo(SAMPLE).unwrap();
        assert_eq!((doc.fps, doc.sound_frames), (24, 12));
        assert_eq!(doc.voices.len(), 2);
        assert_eq!(doc.voices[0].phrase_text, "hello\nworld");
        let v = &doc.voices[0].phonemes;
        // Rest before the word, the two E merged keeping the word, an unknown name counting as Rest, and Rest to the end.
        assert_eq!(v.iter().map(|p| p.phoneme).collect::<Vec<_>>(), [PhonemeType::Rest, PhonemeType::E, PhonemeType::Rest]);
        assert_eq!((v[1].start_frame, v[1].end_frame), (2, 5));
        assert_eq!(v[1].word, "hello");
        assert_eq!((v[2].start_frame, v[2].end_frame), (6, 11));
        assert_eq!(v[2].text, "XX", "a run keeps its first phoneme's text");
        // Times are single precision, as upstream's are.
        let sample = (1.0f32 / 24.0) * 1000.0;
        assert_eq!(v[1].start_ms, sample * 2.0);
        assert_eq!(v[1].duration_ms, sample * 6.0 - sample * 2.0);
        // A voice with no words is one Rest.
        assert_eq!(doc.voices[1].phonemes.len(), 1);
        assert_eq!(doc.voices[1].phonemes[0].phoneme, PhonemeType::Rest);
    }

    #[test]
    fn keeps_the_phrases_words_and_phonemes_as_written() {
        let doc = parse_papagayo(SAMPLE).unwrap();
        let phrase = &doc.voices[0].phrases[0];
        assert_eq!((phrase.text.as_str(), phrase.start_frame, phrase.end_frame), ("hello", 0, 8));
        let word = &phrase.words[0];
        assert_eq!((word.text.as_str(), word.start_frame, word.end_frame), ("hello", 2, 7));
        let frames: Vec<(i32, i32)> = word.phonemes.iter().map(|p| (p.start_frame, p.end_frame)).collect();
        assert_eq!(frames, [(2, 3), (4, 5), (6, 7)]);
        assert!(doc.voices[1].phrases.is_empty());
    }

    #[test]
    fn last_phoneme_runs_to_the_word_end() {
        let text = "lipsync\nx\n10\n6\n1\nV\nt\n1\nw\n0\n5\n1\nw 1 4 2\n1 AI\n3 O\n";
        let doc = parse_papagayo(text).unwrap();
        let v = &doc.voices[0].phonemes;
        assert_eq!(v[1].phoneme, PhonemeType::Ai);
        assert_eq!((v[1].start_frame, v[1].end_frame), (1, 2));
        assert_eq!(v[2].phoneme, PhonemeType::O);
        assert_eq!((v[2].start_frame, v[2].end_frame), (3, 4));
        assert_eq!(v[3].phoneme, PhonemeType::Rest);
    }

    #[test]
    fn a_run_takes_the_last_phonemes_own_end() {
        // The word and its phoneme run to frame 9, past the four sound frames.
        let text = "lipsync\nx\n10\n4\n1\nV\nt\n1\nw\n0\n9\n1\nw 2 9 1\n2 O\n";
        let v = parse_papagayo(text).unwrap().voices.remove(0).phonemes;
        assert_eq!(v.len(), 2);
        assert_eq!((v[1].phoneme, v[1].start_frame, v[1].end_frame), (PhonemeType::O, 2, 9));
        assert_eq!(v[1].duration_ms, 100.0f32 * 10.0 - 100.0f32 * 2.0);
    }

    #[test]
    fn lookup_goes_phrase_then_word_then_phoneme() {
        // Frame 3 is inside the first phrase but none of its words, so it's a Rest, though the second phrase's word covers it.
        let text = "lipsync\nx\n10\n6\n1\nV\nt\n2\na\n0\n3\n1\na 0 2 1\n0 AI\nb\n3\n5\n1\nb 3 5 1\n3 O\n";
        let v = parse_papagayo(text).unwrap().voices.remove(0).phonemes;
        let got: Vec<(PhonemeType, i32, i32)> = v.iter().map(|p| (p.phoneme, p.start_frame, p.end_frame)).collect();
        assert_eq!(got, [(PhonemeType::Ai, 0, 2), (PhonemeType::Rest, 3, 3), (PhonemeType::O, 3, 5)]);
    }

    #[test]
    fn short_and_bad_files() {
        assert_eq!(parse_papagayo("nope"), Err("Invalid File Format".to_string()));
        // The file ending anywhere before the voice count is a document with no voices.
        assert_eq!(parse_papagayo(""), Ok(PapagayoDoc::default()));
        assert!(parse_papagayo("lipsync\nx\n24\n").unwrap().voices.is_empty());
        assert_eq!(parse_papagayo("lipsync\nx\n24\n12\n1\nV\nt\n2\n"), Err("Corrupt File Format".to_string()));
        assert!(parse_papagayo("lipsync\nx\n24\n12\n0\ntrailing\n").is_err());
        assert!(parse_papagayo("lipsync\nx\n24\n12\n0\n\n").is_err(), "a blank line after the voices fails");
        assert!(parse_papagayo("lipsync\nx\n24\n12\n0\n").is_ok());
        assert!(parse_papagayo("lipsync\nx\n24\n12\n2\nV\nt\n0\nV \nt\n0\n").is_err(), "a duplicate trimmed voice name throws");
        assert!(parse_papagayo("lipsync\nx\n0\n12\n1\nV\nt\n0\n").is_err(), "no frame rate");
        assert!(parse_papagayo("lipsync\nx\n24\n12\n1\nV\nt\n-1\n").is_err());
    }

    #[test]
    fn the_voice_name_keeps_its_trailing_spaces() {
        let doc = parse_papagayo("lipsync\nx\n24\n12\n1\n  Lead  \nt\n0\n").unwrap();
        assert_eq!(doc.voices[0].name, "Lead  ");
        assert_eq!(marks_import_summary(&doc), "1 voices imported as Mark Collections\n\nRow #1 - Lead  ");
    }

    #[test]
    fn word_lines_with_other_field_counts_are_left_out() {
        let text = "lipsync\nx\n10\n4\n1\nV\nt\n1\nw\n0\n3\n1\nw 1 2\n";
        let doc = parse_papagayo(text).unwrap();
        assert_eq!(doc.voices[0].phonemes.len(), 1);
        assert!(doc.voices[0].phrases[0].words.is_empty());
        // Two spaces make five fields.
        let text = "lipsync\nx\n10\n4\n1\nV\nt\n1\nw\n0\n3\n1\nw  1 2 0\n";
        assert!(parse_papagayo(text).unwrap().voices[0].phrases[0].words.is_empty());
    }

    #[test]
    fn a_phoneme_list_cut_short_keeps_what_it_has() {
        let text = "lipsync\nx\n10\n6\n1\nV\nt\n1\nw\n0\n5\n1\nw 1 4 3\n1 AI\n";
        let doc = parse_papagayo(text).unwrap();
        let word = &doc.voices[0].phrases[0].words[0];
        assert_eq!(word.phonemes.len(), 1);
        assert_eq!((word.phonemes[0].start_frame, word.phonemes[0].end_frame), (1, 4));
    }

    #[test]
    fn a_phoneme_line_without_two_fields_is_a_rest_at_zero() {
        let text = "lipsync\nx\n10\n8\n1\nV\nt\n1\nw\n0\n7\n1\nw 1 6 3\n1 AI\nbroken\n5 O\n";
        let doc = parse_papagayo(text).unwrap();
        let p = &doc.voices[0].phrases[0].words[0].phonemes;
        // The AI isn't ended by the broken line, so it keeps the end it started with; the broken one is ended by the O.
        assert_eq!((p[0].phoneme, p[0].start_frame, p[0].end_frame), (PhonemeType::Ai, 1, 0));
        assert_eq!((p[1].phoneme, p[1].text.as_str(), p[1].start_frame, p[1].end_frame), (PhonemeType::Rest, "Rest", 0, 4));
        assert_eq!((p[2].phoneme, p[2].start_frame, p[2].end_frame), (PhonemeType::O, 5, 6));
    }

    #[test]
    fn phoneme_names_parse_as_the_enum_does() {
        assert_eq!(PhonemeType::parse("etc"), PhonemeType::Etc);
        assert_eq!(PhonemeType::parse("ETC"), PhonemeType::Rest);
        assert_eq!(PhonemeType::parse("ai"), PhonemeType::Rest);
        assert_eq!(PhonemeType::parse("3"), PhonemeType::U);
        assert_eq!(PhonemeType::parse("12"), PhonemeType::Rest);
        assert_eq!(PhonemeType::parse(""), PhonemeType::Rest);
    }

    #[test]
    fn summary_lists_each_voice() {
        assert_eq!(import_summary(&["A".to_string(), "B".to_string()]), "2 Voices imported to clipboard as separate rows\n\nRow #1 - A\nRow #2 - B\n");
        assert_eq!(marks_import_summary(&PapagayoDoc::default()), "0 voices imported as Mark Collections\n\n");
    }

    #[test]
    fn a_voice_becomes_three_linked_collections() {
        let doc = parse_papagayo(SAMPLE).unwrap();
        let all = mark_collections(&doc, "song");
        assert_eq!(all.len(), 6);
        let (phrases, words, phonemes) = (&all[0], &all[1], &all[2]);
        assert_eq!(phrases.name, "song Voice One Phrases");
        assert_eq!(words.name, "song Voice One Words");
        assert_eq!(phonemes.name, "song Voice One Phonemes");
        assert_eq!(
            [phrases.collection_type, words.collection_type, phonemes.collection_type],
            [MarkCollectionType::Phrase, MarkCollectionType::Word, MarkCollectionType::Phoneme]
        );
        assert_eq!(
            [phrases.decorator.color, words.decorator.color, phonemes.decorator.color],
            [Color::rgb(205, 242, 162), Color::rgb(242, 205, 162), Color::rgb(235, 185, 210)]
        );
        assert_eq!(phrases.linked_mark_collection_id, None);
        assert_eq!(words.linked_mark_collection_id, Some(phrases.id));
        assert_eq!(phonemes.linked_mark_collection_id, Some(words.id));
        assert!(all.iter().all(|c| c.show_marks && c.show_grid_lines && !c.is_default));

        let sample = (1.0f32 / 24.0) * 1000.0;
        let ticks = |ms: f32| TimeSpan::from_ms(ms as f64);
        assert_eq!(phrases.marks.len(), 1);
        assert_eq!((phrases.marks[0].start_time, phrases.marks[0].text.as_str()), (TimeSpan::ZERO, "hello"));
        assert_eq!(phrases.marks[0].duration, ticks(sample * 9.0));
        assert_eq!(words.marks[0].start_time, ticks(sample * 2.0));
        assert_eq!(words.marks[0].duration, ticks(sample * 8.0 - sample * 2.0));
        // The phonemes are the ones written, not the coalesced runs, under their type names.
        let texts: Vec<&str> = phonemes.marks.iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["E", "E", "Rest"]);
        assert_eq!(phonemes.marks[1].start_time, ticks(sample * 4.0));
        assert_eq!(phonemes.marks[1].duration, ticks(sample * 6.0 - sample * 4.0));
        // A voice with nothing in it still gives its three, empty.
        assert!(all[3..].iter().all(|c| c.marks.is_empty()));
        assert_eq!(all[3].name, "song Voice Two Phrases");
    }

    #[test]
    fn short_marks_get_the_minimum_duration() {
        // One frame at 100 frames a second is 10 ms, under the 25 ms a mark can be.
        let text = "lipsync\nx\n100\n4\n1\nV\nt\n1\nw\n1\n1\n1\nw 1 1 1\n1 O\n";
        let doc = parse_papagayo(text).unwrap();
        let all = mark_collections(&doc, "f");
        assert!(all.iter().all(|c| c.marks[0].duration == TimeSpan::from_ms(25.0)));
    }
}
