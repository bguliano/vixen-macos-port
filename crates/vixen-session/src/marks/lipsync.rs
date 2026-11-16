//! Text to phonemes, as the Lyric Converter and the marks bar's phoneme breakdown use it: `LipSyncTextConvert` and the conversion half of
//! `LipSyncTextConvertForm`. `docs/spec/marks-layers-analysis.md` F80, F81, F84, F85, F86.
//!
//! The standard dictionary is CMUdict 0.07, compiled in with its BSD-style notice (at the top of `assets/standard_dictionary`), and parsed
//! once, on first use, into keys that point into the embedded text.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;
use vixen_core::TimeSpan;

static STANDARD_TEXT: &str = include_str!("../../assets/standard_dictionary");

/// `PhonemeType`. The user dictionary may hold these names directly.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
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
    Unknown,
}

impl PhonemeType {
    /// Upstream's upper-case names, which the LipSync effect and the marks bar use.
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
            PhonemeType::Etc => "ETC",
            PhonemeType::Rest => "REST",
            PhonemeType::Unknown => "Unknown",
        }
    }
}

/// F99's order, which the image map editor and the failure dialog's buttons follow.
pub const MAP_PHONEMES: [PhonemeType; 10] = [
    PhonemeType::Ai,
    PhonemeType::E,
    PhonemeType::Etc,
    PhonemeType::Fv,
    PhonemeType::L,
    PhonemeType::Mbp,
    PhonemeType::O,
    PhonemeType::Rest,
    PhonemeType::U,
    PhonemeType::Wq,
];

/// F84's table, `PU:564-661`. KEEP: `AO` with no stress digit is AI, while `AO0` to `AO2` are O.
fn token_phoneme(token: &str) -> Option<PhonemeType> {
    use PhonemeType::*;
    let base = token.trim_end_matches(['0', '1', '2']);
    let digit = token.len() - base.len();
    // The stress variants exist only as a single 0, 1 or 2.
    let stressed = digit == 1;
    let with_stress = |p: PhonemeType| if digit == 0 || stressed { Some(p) } else { None };
    match base {
        "AA" | "AE" | "AH" | "AY" | "IH" => with_stress(Ai),
        "AO" if digit == 0 => Some(Ai),
        "AO" if stressed => Some(O),
        "AW" | "OW" => with_stress(O),
        "EH" | "ER" | "EY" | "IY" => with_stress(E),
        "UH" | "UW" => with_stress(U),
        "OY" => with_stress(Wq),
        _ if digit == 0 => match token {
            "AI" => Some(Ai),
            "O" => Some(O),
            "E21" | "E" => Some(E),
            "U" => Some(U),
            "F" | "V" | "FV" => Some(Fv),
            "L" => Some(L),
            "B" | "M" | "P" | "MBP" => Some(Mbp),
            "W" | "WQ" => Some(Wq),
            "CH" | "D" | "DH" | "G" | "HH" | "JH" | "K" | "N" | "NG" | "R" | "S" | "SH" | "T" | "TH" | "Y" | "Z" | "ZH" | "ETC" => Some(Etc),
            "REST" => Some(Rest),
            _ => None,
        },
        // `E21` is the one table entry ending in digits that aren't a stress mark.
        _ if token == "E21" => Some(E),
        _ => None,
    }
}

/// One dictionary line, `PU:663-688`: split on single spaces; a first token of exactly `;;;` is a comment; the key is the first token and
/// the later non-empty ones are phonemes. An unknown token is logged and dropped. An entry with no valid phonemes is still stored.
fn parse_line<'a>(line: &'a str, corrupt: &mut Vec<String>) -> Option<(&'a str, Vec<PhonemeType>)> {
    let mut tokens = line.split(' ');
    let key = tokens.next()?;
    if key == ";;;" {
        return None;
    }
    let mut phonemes = Vec::new();
    for token in tokens.filter(|t| !t.is_empty()) {
        match token_phoneme(token) {
            Some(p) => phonemes.push(p),
            None => corrupt.push(token.to_string()),
        }
    }
    Some((key, phonemes))
}

struct Standard {
    entries: HashMap<&'static str, Box<[PhonemeType]>>,
}

fn standard() -> &'static Standard {
    static STANDARD: OnceLock<Standard> = OnceLock::new();
    STANDARD.get_or_init(|| {
        let mut entries = HashMap::with_capacity(140_000);
        let mut corrupt = Vec::new();
        for line in STANDARD_TEXT.lines() {
            if let Some((key, phonemes)) = parse_line(line, &mut corrupt) {
                entries.insert(key, phonemes.into_boxed_slice());
            }
        }
        Standard { entries }
    })
}

/// Starts parsing the standard dictionary on a worker, so the first conversion doesn't wait for it. Call when the Lyric Converter opens.
pub fn warm_up() {
    let _ = std::thread::Builder::new().name("vixen-phoneme-dictionary".to_string()).spawn(|| {
        standard();
    });
}

/// `ToUpper`, one character at a time: Rust's `to_uppercase` turns `ß` into `SS`, which only a user dictionary could notice.
fn upper(text: &str) -> String {
    text.chars()
        .map(|c| {
            let mut u = c.to_uppercase();
            match (u.next(), u.next()) {
                (Some(x), None) => x,
                _ => c,
            }
        })
        .collect()
}

/// `RemovePunctuation`: exactly `. , ? ! "`.
pub fn remove_punctuation(text: &str) -> String {
    text.chars().filter(|c| !matches!(c, '.' | ',' | '?' | '!' | '"')).collect()
}

/// The standard dictionary with the profile's user dictionary over it (F84, F85).
pub struct PhonemeDictionary {
    user: HashMap<String, Box<[PhonemeType]>>,
    /// Tokens the user file held that aren't phonemes, which upstream logs.
    pub corrupt: Vec<String>,
}

impl PhonemeDictionary {
    /// The standard dictionary, then `user_dictionary` if the file exists; it overrides, last line winning. Windows files end lines in CRLF.
    pub fn load(user_path: Option<&Path>) -> PhonemeDictionary {
        standard();
        let mut dict = PhonemeDictionary { user: HashMap::new(), corrupt: Vec::new() };
        if let Some(text) = user_path.and_then(|p| std::fs::read(p).ok()) {
            let text = vixen_io::marks_files::read_import_text(&text);
            for line in text.lines() {
                dict.apply_line(line);
            }
        }
        dict
    }

    fn apply_line(&mut self, line: &str) {
        let mut corrupt = Vec::new();
        if let Some((key, phonemes)) = parse_line(line, &mut corrupt) {
            self.user.insert(key.to_string(), phonemes.into_boxed_slice());
        }
        self.corrupt.extend(corrupt);
    }

    fn lookup(&self, word: &str) -> Option<&[PhonemeType]> {
        self.user.get(word).map(|b| &**b).or_else(|| standard().entries.get(word).map(|b| &**b))
    }

    /// How many standard entries there are, for checks.
    pub fn standard_len() -> usize {
        standard().entries.len()
    }

    /// `TryConvert`, `PU:747-773`: each whitespace-separated word upper-cased and looked up. KEEP: a word not found is skipped silently, so a
    /// phrase fails only when no word is known. Within a word, repeated phonemes collapse; across words they don't.
    pub fn try_convert(&self, text: &str) -> Vec<PhonemeType> {
        let mut out = Vec::new();
        for word in text.split_whitespace() {
            let Some(phonemes) = self.lookup(&upper(word)) else { continue };
            let mut last = None;
            for p in phonemes {
                if Some(*p) != last {
                    out.push(*p);
                    last = Some(*p);
                }
            }
        }
        out
    }

    /// `AddUserMaping`, F85: the whole line upper-cased, live at once, and appended to the file after a newline, creating it, so a new file
    /// starts with a blank line. GUARD: a write failure is returned, with the mapping still live for this run.
    pub fn add_user_mapping(&mut self, word: &str, mapped: &str, user_path: &Path) -> std::io::Result<()> {
        let line = upper(&format!("{word} {mapped}"));
        self.apply_line(&line);
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(user_path)?;
        file.write_all(format!("\n{line}").as_bytes())
    }
}

/// F82's caption and text for each button, in its two rows: the phoneme's name, appended as `<NAME> `. FIX: the L button's caption is "L",
/// where upstream's reads "I".
pub const FAILURE_BUTTONS: [[PhonemeType; 5]; 2] = [
    [PhonemeType::Ai, PhonemeType::E, PhonemeType::Etc, PhonemeType::Fv, PhonemeType::L],
    [PhonemeType::Mbp, PhonemeType::O, PhonemeType::Rest, PhonemeType::U, PhonemeType::Wq],
];

/// The Alignment list (F78).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Alignment {
    Phoneme,
    Word,
    Phrase,
}

/// `CreateSubstringList`, F80 step 3: punctuation removed from each line; Phrase keeps the line whole, anything else (no alignment chosen
/// included) splits it on white space keeping empty entries, which are skipped later but still count when Start Offset advances.
pub fn lyric_strings(lines: &[&str], align: Option<Alignment>) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        let clean = remove_punctuation(line);
        if align == Some(Alignment::Phrase) {
            out.push(clean);
        } else {
            out.extend(clean.split(char::is_whitespace).map(str::to_string));
        }
    }
    out
}

/// `CalcPhonemeTimespans`, F81, with the marks sorted once by the caller. Returns (start relative to the first mark, duration). KEEP: past
/// the last mark the start is the absolute last mark plus the phonemes counted since, so a first mark above zero makes a jump.
pub fn phoneme_timing(marks: Option<&[TimeSpan]>, index: usize, sub_elements: i64, unmarked: &mut i64) -> (TimeSpan, TimeSpan) {
    let last = marks.and_then(|m| m.last().copied()).unwrap_or(TimeSpan::ZERO);
    let count = marks.map_or(0, <[TimeSpan]>::len);
    let duration = match marks {
        Some(m) if index + 1 < count && sub_elements != 0 => TimeSpan::from_ticks((m[index + 1] - m[index]).ticks() / sub_elements),
        _ => TimeSpan::from_ms_i(250),
    };
    if index == 0 {
        *unmarked = 0;
    }
    let start = match marks {
        Some(m) if index < count => {
            *unmarked = 0;
            m[index] - m[0]
        }
        _ => {
            let start = TimeSpan::from_ticks(last.ticks() + duration.ticks() * *unmarked);
            *unmarked += sub_elements;
            start
        }
    };
    (start, duration)
}

/// One phoneme the conversion makes.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvertData {
    pub start: TimeSpan,
    pub duration: TimeSpan,
    pub phoneme: PhonemeType,
    pub lyric: String,
}

/// What a conversion step needs next.
#[derive(Clone, Debug, PartialEq)]
pub enum ConvertStep {
    /// F82: ask for a mapping for this string, as typed (untrimmed), then call `step` again.
    NeedsMapping(String),
    /// Every string converted.
    Done(Vec<ConvertData>),
    /// KEEP: a string still unknown after the dialog ends the whole conversion, with nothing delivered and no message.
    Abandoned,
}

/// F80 steps 4 to 6 as a resumable state, since the failure dialog is asynchronous in a window.
pub struct LyricConversion {
    strings: Vec<String>,
    marks: Option<Vec<TimeSpan>>,
    align_phonemes: bool,
    next: usize,
    mark_index: usize,
    retrying: bool,
    output: Vec<ConvertData>,
    /// `unMarkedPhonemes`, which upstream keeps on the window between conversions.
    pub unmarked: i64,
}

impl LyricConversion {
    /// `marks` are the chosen collection's start times, or none when no Start Offset is chosen, as with Cursor and Clipboard; `start_index`
    /// is the Start Offset's index. Phoneme alignment applies only when the Alignment reads Phoneme.
    pub fn new(strings: Vec<String>, marks: Option<Vec<TimeSpan>>, start_index: Option<usize>, align: Option<Alignment>, unmarked: i64) -> LyricConversion {
        let (marks, mark_index) = match start_index {
            Some(i) => (marks.map(|mut m| {
                m.sort();
                m
            }), i),
            None => (None, 0),
        };
        LyricConversion {
            strings,
            marks,
            align_phonemes: align == Some(Alignment::Phoneme),
            next: 0,
            mark_index,
            retrying: false,
            output: Vec::new(),
            unmarked,
        }
    }

    /// Runs until it needs a mapping, finishes, or gives up.
    pub fn step(&mut self, dict: &PhonemeDictionary) -> ConvertStep {
        while self.next < self.strings.len() {
            let text = self.strings[self.next].clone();
            if text.trim().is_empty() {
                self.next += 1;
                continue;
            }
            let phonemes = if self.retrying {
                self.retrying = false;
                let again = dict.try_convert(&text);
                if again.is_empty() {
                    return ConvertStep::Abandoned;
                }
                again
            } else {
                let first = dict.try_convert(text.trim());
                if first.is_empty() {
                    self.retrying = true;
                    return ConvertStep::NeedsMapping(text);
                }
                first
            };
            let marks = self.marks.as_deref();
            let mut timing = (TimeSpan::ZERO, TimeSpan::ZERO);
            if !self.align_phonemes {
                timing = phoneme_timing(marks, self.mark_index, phonemes.len() as i64, &mut self.unmarked);
                self.mark_index += 1;
            }
            let mut phoneme_index = 0i64;
            for phoneme in &phonemes {
                if self.align_phonemes {
                    timing = phoneme_timing(marks, self.mark_index, 1, &mut self.unmarked);
                    self.mark_index += 1;
                    phoneme_index = 0;
                }
                let start = TimeSpan::from_ticks(timing.0.ticks() + timing.1.ticks() * phoneme_index);
                phoneme_index += 1;
                self.output.push(ConvertData { start, duration: timing.1, phoneme: *phoneme, lyric: text.clone() });
            }
            self.next += 1;
        }
        ConvertStep::Done(std::mem::take(&mut self.output))
    }
}

/// F80 step 8: where Start Offset moves after a Convert with Per Mark Collection, when that's still in range; otherwise it stays. Word counts
/// every string, blank ones included.
pub fn next_start_offset(index: usize, count: usize, align: Alignment, phonemes: usize, strings: usize) -> usize {
    let next = index
        + match align {
            Alignment::Phoneme => phonemes,
            Alignment::Word => strings,
            Alignment::Phrase => 1,
        };
    if next < count {
        next
    } else {
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use PhonemeType::*;

    fn dict() -> PhonemeDictionary {
        PhonemeDictionary::load(None)
    }

    fn ms(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v)
    }

    #[test]
    fn words_convert_as_the_spec_computed_from_the_shipped_file() {
        let d = dict();
        let cases: [(&str, &[PhonemeType]); 12] = [
            ("hello", &[Etc, Ai, L, O]),
            ("world", &[Wq, E, L, Etc]),
            ("the", &[Etc, Ai]),
            ("a", &[Ai]),
            ("mom", &[Mbp, Ai, Mbp]),
            ("happy", &[Etc, Ai, Mbp, E]),
            ("christmas", &[Etc, Ai, Etc, Mbp, Ai, Etc]),
            ("don't", &[Etc, O, Etc]),
            ("santa", &[Etc, Ai, Etc, Ai]),
            ("all", &[O, L]),
            ("xyzzy", &[]),
            ("hello xyzzy world", &[Etc, Ai, L, O, Wq, E, L, Etc]),
        ];
        for (text, want) in cases {
            assert_eq!(d.try_convert(text), want, "{text}");
        }
        assert_eq!(d.try_convert("HeLLo"), d.try_convert("hello"));
    }

    #[test]
    fn the_shipped_dictionary_has_every_entry_and_no_unknown_tokens() {
        let mut corrupt = Vec::new();
        let mut keys = 0;
        let mut alternates = 0;
        for line in STANDARD_TEXT.lines() {
            if let Some((key, _)) = parse_line(line, &mut corrupt) {
                keys += 1;
                if key.ends_with(')') {
                    alternates += 1;
                }
            }
        }
        assert!(corrupt.is_empty(), "{:?}", &corrupt[..corrupt.len().min(5)]);
        assert_eq!(keys, 133_854);
        assert_eq!(alternates, 8_780);
        assert_eq!(PhonemeDictionary::standard_len(), 133_854);
        // An alternate is stored under its literal key; the word itself uses the first pronunciation.
        assert_eq!(dict().try_convert("a(1)"), [E]);
        assert_eq!(dict().try_convert("a"), [Ai]);
    }

    #[test]
    fn user_mappings_win_and_append_to_the_file() {
        let path = std::env::temp_dir().join(format!("vixen-user-dictionary-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut d = PhonemeDictionary::load(Some(&path));
        d.add_user_mapping("wibble", "AI WQ ", &path).unwrap();
        assert_eq!(d.try_convert("wibble"), [Ai, Wq]);
        d.add_user_mapping("hello", "MBP", &path).unwrap();
        assert_eq!(d.try_convert("hello"), [Mbp]);
        d.add_user_mapping("nothing", "", &path).unwrap();
        assert_eq!(d.try_convert("nothing"), []);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "\nWIBBLE AI WQ \nHELLO MBP\nNOTHING ");
        // A Windows file, with CRLF, reads back the same.
        std::fs::write(&path, "\r\nWIBBLE AI WQ \r\nHELLO MBP").unwrap();
        let again = PhonemeDictionary::load(Some(&path));
        assert_eq!(again.try_convert("wibble hello"), [Ai, Wq, Mbp]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn punctuation_and_strings() {
        assert_eq!(remove_punctuation("Hello, \"World\"!"), "Hello World");
        assert_eq!(remove_punctuation("don't"), "don't");
        assert_eq!(lyric_strings(&["hello  world."], Some(Alignment::Word)), ["hello", "", "world"]);
        assert_eq!(lyric_strings(&["hello  world."], None), ["hello", "", "world"]);
        assert_eq!(lyric_strings(&["hello, world"], Some(Alignment::Phrase)), ["hello world"]);
    }

    #[test]
    fn even_spacing_is_250_ms_per_phoneme() {
        let d = dict();
        let mut c = LyricConversion::new(lyric_strings(&["hello world"], Some(Alignment::Word)), None, None, Some(Alignment::Word), 0);
        let ConvertStep::Done(out) = c.step(&d) else { panic!() };
        let starts: Vec<f64> = out.iter().map(|o| o.start.total_ms()).collect();
        assert_eq!(starts, [0.0, 250.0, 500.0, 750.0, 1000.0, 1250.0, 1500.0, 1750.0]);
        assert!(out.iter().all(|o| o.duration == ms(250)));
        let names: Vec<&str> = out.iter().map(|o| o.phoneme.name()).collect();
        assert_eq!(names, ["ETC", "AI", "L", "O", "WQ", "E", "L", "ETC"]);
        assert_eq!(out[4].lyric, "world");
    }

    #[test]
    fn mark_timing_and_the_jump_past_the_last_mark() {
        let marks = [ms(1000), ms(2000), ms(3000)];
        let mut unmarked = 0;
        assert_eq!(phoneme_timing(Some(&marks), 0, 4, &mut unmarked), (ms(0), ms(250)));
        assert_eq!(phoneme_timing(Some(&marks), 1, 4, &mut unmarked), (ms(1000), ms(250)));
        assert_eq!(phoneme_timing(Some(&marks), 2, 1, &mut unmarked), (ms(2000), ms(250)));
        assert_eq!(phoneme_timing(Some(&marks), 3, 1, &mut unmarked), (ms(3000), ms(250)));
        let phonemes = [ms(0), ms(100), ms(300)];
        let durations: Vec<TimeSpan> = (0..3).map(|i| phoneme_timing(Some(&phonemes), i, 1, &mut unmarked).1).collect();
        assert_eq!(durations, [ms(100), ms(200), ms(250)]);
    }

    #[test]
    fn an_unknown_word_asks_then_abandons_or_carries_on() {
        let mut d = dict();
        let mut c = LyricConversion::new(vec!["hello".into(), "wibblex".into()], None, None, None, 0);
        assert_eq!(c.step(&d), ConvertStep::NeedsMapping("wibblex".to_string()));
        assert_eq!(c.step(&d), ConvertStep::Abandoned);
        let path = std::env::temp_dir().join(format!("vixen-user-dictionary-b-{}", std::process::id()));
        let mut c = LyricConversion::new(vec!["hello".into(), "wibblex".into()], None, None, None, 0);
        assert_eq!(c.step(&d), ConvertStep::NeedsMapping("wibblex".to_string()));
        d.add_user_mapping("wibblex", "U ", &path).unwrap();
        let ConvertStep::Done(out) = c.step(&d) else { panic!() };
        assert_eq!(out.len(), 5);
        assert_eq!(out[4].phoneme, U);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn start_offset_advances_only_within_range() {
        assert_eq!(next_start_offset(2, 10, Alignment::Word, 8, 3), 5);
        assert_eq!(next_start_offset(2, 10, Alignment::Phoneme, 8, 3), 2);
        assert_eq!(next_start_offset(2, 10, Alignment::Phoneme, 7, 3), 9);
        assert_eq!(next_start_offset(9, 10, Alignment::Phrase, 1, 1), 9);
    }
}
