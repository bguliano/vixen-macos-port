//! The Lyric Converter window without its window: `LipSyncTextConvertForm`'s lists and Convert, and what `textConverterHandler` makes of the
//! result. `docs/spec/marks-layers-analysis.md` F77 to F80, F83, F85, F86. The conversion itself is `lipsync`.

use super::lipsync::{lyric_strings, next_start_offset, remove_punctuation, Alignment, ConvertData, ConvertStep, LyricConversion, PhonemeDictionary};
use crate::sequencer::clipboard::{self, ClipboardData, ClipboardEffect};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_core::marks::MarkCollection;
use vixen_core::TimeSpan;
use vixen_effects::framework::ParamValue;

/// F82's window title and the two lines above its field.
pub const FAILURE_TITLE: &str = "Text Conversion Failure";
pub fn failure_text(word: &str) -> String {
    format!("Unable to find mapping for {word}\nPlease map using buttons below")
}

/// F83's alert when nothing was pasted. The spaces around the line break are upstream's.
pub const COPIED_TITLE: &str = "Convert Text";
pub const COPIED_MESSAGE: &str = "Conversion Complete and copied to Clipboard \n Paste at first Mark offset";

/// The process's dictionary, as upstream's static one, but tied to the profile whose `user_dictionary` it read: another profile's path
/// loads again (F85).
static DICTIONARY: Mutex<Option<(PathBuf, PhonemeDictionary)>> = Mutex::new(None);

pub fn with_dictionary<R>(user_path: &Path, f: impl FnOnce(&mut PhonemeDictionary) -> R) -> R {
    let mut slot = DICTIONARY.lock();
    if slot.as_ref().is_none_or(|(path, _)| path != user_path) {
        let dictionary = PhonemeDictionary::load(Some(user_path));
        for token in &dictionary.corrupt {
            crate::logging::error("LipSyncTextConvert", &format!("Phoneme Dictionary Corruption detected!, invalid Phoneme Type {token}"), None);
        }
        *slot = Some((user_path.to_path_buf(), dictionary));
    }
    f(&mut slot.as_mut().expect("just filled").1)
}

/// Closing and reopening the same profile must reload its user file too. A late close
/// from an old profile must not discard a newer profile's active dictionary.
pub fn release_dictionary(user_path: &Path) {
    let mut slot = DICTIONARY.lock();
    if slot.as_ref().is_some_and(|(path, _)| path == user_path) { *slot = None; }
}

/// F86: what the marks bar's Breakdown Word asks for one mark's text. Empty for a word with no mapping, which F82 may then give.
pub fn word_phonemes(dictionary: &PhonemeDictionary, text: &str) -> Vec<String> {
    dictionary.try_convert(&remove_punctuation(text.trim())).into_iter().map(|p| p.name().to_string()).collect()
}

/// `TranslatePlacement`, in the radios' order.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Placement {
    #[default]
    Cursor,
    Clipboard,
    Mark,
}

/// .NET's default `TimeSpan` text, which the Start Offset list shows: `hh:mm:ss`, with seven fraction digits only when there are any, and
/// days in front past a day.
pub fn start_offset_text(time: TimeSpan) -> String {
    let ticks = time.ticks();
    let sign = if ticks < 0 { "-" } else { "" };
    let ticks = ticks.unsigned_abs();
    let (days, rest) = (ticks / 864_000_000_000, ticks % 864_000_000_000);
    let (hours, rest) = (rest / 36_000_000_000, rest % 36_000_000_000);
    let (minutes, rest) = (rest / 600_000_000, rest % 600_000_000);
    let (seconds, fraction) = (rest / 10_000_000, rest % 10_000_000);
    let mut text = String::from(sign);
    if days > 0 {
        text.push_str(&format!("{days}."));
    }
    text.push_str(&format!("{hours:02}:{minutes:02}:{seconds:02}"));
    if fraction > 0 {
        text.push_str(&format!(".{fraction:07}"));
    }
    text
}

struct Choice {
    id: Uuid,
    name: String,
    starts: Vec<TimeSpan>,
}

/// What Convert hands the sequencer: `NewTranslationEventArgs` and the clipboard `textConverterHandler` builds from it.
#[derive(Clone, Debug, PartialEq)]
pub struct Delivery {
    /// `None` when no phoneme came out, where the handler does nothing at all. Otherwise it goes on the timeline clipboard, even with no
    /// effects in it.
    pub clipboard: Option<ClipboardData>,
    pub placement: Placement,
    /// The Start Offset's value, or zero with none chosen. Cursor placement adds the cursor to it; anything but Clipboard pastes there.
    pub first_mark: TimeSpan,
    /// FIX F80: the text clears here, after delivery, where upstream clears it after each string, before a later one can bail.
    pub clear_text: bool,
}

/// How a Convert went.
#[derive(Clone, Debug, PartialEq)]
pub enum ConvertOutcome {
    /// F82: ask for this string's mapping, then call `resume`.
    NeedsMapping(String),
    Delivered(Delivery),
    /// KEEP: a string still unknown after the dialog ends the Convert with nothing delivered, no message, and Start Offset where it was.
    Abandoned,
}

struct Running {
    conversion: LyricConversion,
    strings: usize,
    clear_after: bool,
}

/// The window's state. Nothing here persists between openings (F76).
#[derive(Default)]
pub struct LyricConverter {
    placement: Placement,
    choices: Vec<Choice>,
    collection: Option<usize>,
    /// `_lastMarkIndex`.
    last_collection: Option<usize>,
    offsets: Vec<TimeSpan>,
    offset: Option<usize>,
    /// Empty until the Marks group first enables, and kept from then on whatever the radio says (KEEP F78).
    alignment: Option<Alignment>,
    /// `unMarkedPhonemes`, which lives on the window between Converts.
    unmarked: i64,
    running: Option<Running>,
}

impl LyricConverter {
    pub fn new() -> LyricConverter {
        LyricConverter::default()
    }

    pub fn placement(&self) -> Placement {
        self.placement
    }

    /// F77: Per Mark Collection can be chosen only while a collection has marks.
    pub fn mark_placement_available(&self) -> bool {
        !self.choices.is_empty()
    }

    pub fn marks_enabled(&self) -> bool {
        self.placement == Placement::Mark
    }

    pub fn collection_names(&self) -> Vec<String> {
        self.choices.iter().map(|c| c.name.clone()).collect()
    }

    pub fn collection_index(&self) -> Option<usize> {
        self.collection
    }

    pub fn start_offsets(&self) -> &[TimeSpan] {
        &self.offsets
    }

    pub fn start_offset_index(&self) -> Option<usize> {
        self.offset
    }

    pub fn alignment(&self) -> Option<Alignment> {
        self.alignment
    }

    /// `PopulateStartOffsetCombo`: the chosen collection's start times, keeping the index if it still fits, else the last, and the first
    /// when nothing was chosen.
    fn populate_offsets(&mut self) {
        let last = self.offset;
        self.offsets.clear();
        self.offset = None;
        let Some(choice) = self.collection.and_then(|i| self.choices.get(i)) else { return };
        self.offsets = choice.starts.clone();
        if !self.offsets.is_empty() {
            self.offset = Some(last.unwrap_or(0).min(self.offsets.len() - 1));
        }
    }

    /// `markCollectionCombo_SelectedIndexChanged`: another collection starts from its first mark.
    fn collection_changed(&mut self, index: Option<usize>) {
        self.collection = index;
        if self.last_collection != index {
            self.offset = None;
        }
        self.populate_offsets();
        self.last_collection = index;
    }

    /// `setMarkControls(markCollectionRadio.Checked)`, which upstream runs on every activation and radio change: the Collection list again
    /// from the sequence, with the same index if the count is unchanged and the first otherwise. Call it when the window comes forward and
    /// when the sequence's marks change.
    pub fn refresh(&mut self, collections: &[MarkCollection]) {
        let last_count = self.choices.len();
        let last_index = self.collection;
        self.choices = collections
            .iter()
            .filter(|c| !c.marks.is_empty())
            .map(|c| Choice { id: c.id, name: c.name.clone(), starts: c.marks.iter().map(|m| m.start_time).collect() })
            .collect();
        if self.choices.is_empty() {
            // GUARD: upstream leaves the Start Offset list as it was once the last marks go, so a Convert would still take a first mark
            // from a collection that has none.
            self.collection = None;
            self.last_collection = None;
            self.offsets.clear();
            self.offset = None;
            if self.placement == Placement::Mark {
                self.alignment.get_or_insert(Alignment::Word);
            }
            return;
        }
        if self.placement == Placement::Mark {
            let index = if self.choices.len() == last_count { last_index.unwrap_or(0) } else { 0 };
            // The list was cleared first, so setting the index always runs the change handler.
            self.collection_changed(Some(index));
            self.alignment.get_or_insert(Alignment::Word);
        } else {
            self.collection_changed(None);
        }
    }

    /// A radio. Per Mark Collection is refused while no collection has marks.
    pub fn set_placement(&mut self, placement: Placement, collections: &[MarkCollection]) {
        if placement == Placement::Mark && !collections.iter().any(|c| !c.marks.is_empty()) {
            self.refresh(collections);
            return;
        }
        self.placement = placement;
        self.refresh(collections);
    }

    pub fn set_collection(&mut self, index: usize) {
        if self.marks_enabled() && index < self.choices.len() && self.collection != Some(index) {
            self.collection_changed(Some(index));
        }
    }

    pub fn set_start_offset(&mut self, index: usize) {
        if self.marks_enabled() && index < self.offsets.len() {
            self.offset = Some(index);
        }
    }

    pub fn set_alignment(&mut self, alignment: Alignment) {
        if self.marks_enabled() {
            self.alignment = Some(alignment);
        }
    }

    /// The Convert button, F80. `text` is the selection if there is one, else everything in the box.
    pub fn convert(&mut self, text: &str, clear_after: bool, dictionary: &PhonemeDictionary) -> ConvertOutcome {
        let lines: Vec<&str> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
        let strings = lyric_strings(&lines, self.alignment);
        let marks = self.collection.and_then(|i| self.choices.get(i)).map(|c| c.starts.clone());
        let count = strings.len();
        let conversion = LyricConversion::new(strings, marks, self.offset, self.alignment, self.unmarked);
        self.running = Some(Running { conversion, strings: count, clear_after });
        self.resume(dictionary)
    }

    /// After F82 closed, whatever its answer: the string is tried once more and the Convert goes on or ends.
    pub fn resume(&mut self, dictionary: &PhonemeDictionary) -> ConvertOutcome {
        let Some(mut running) = self.running.take() else { return ConvertOutcome::Abandoned };
        let step = running.conversion.step(dictionary);
        self.unmarked = running.conversion.unmarked;
        match step {
            ConvertStep::NeedsMapping(word) => {
                self.running = Some(running);
                ConvertOutcome::NeedsMapping(word)
            }
            ConvertStep::Abandoned => ConvertOutcome::Abandoned,
            ConvertStep::Done(data) => {
                let first_mark = self.offset.and_then(|i| self.offsets.get(i).copied()).unwrap_or(TimeSpan::ZERO);
                let delivery = Delivery {
                    clipboard: clipboard_data(&data),
                    placement: self.placement,
                    first_mark,
                    clear_text: running.clear_after && !data.is_empty(),
                };
                if self.placement == Placement::Mark {
                    if let (Some(index), Some(alignment)) = (self.offset, self.alignment) {
                        self.offset = Some(next_start_offset(index, self.offsets.len(), alignment, data.len(), running.strings));
                    }
                }
                ConvertOutcome::Delivered(delivery)
            }
        }
    }

    /// The id of the chosen collection, for checks.
    pub fn collection_id(&self) -> Option<Uuid> {
        self.collection.and_then(|i| self.choices.get(i)).map(|c| c.id)
    }
}

/// `textConverterHandler`'s clipboard, F83: a LipSync effect for each output with a duration, all on the row pasted to.
pub fn clipboard_data(data: &[ConvertData]) -> Option<ClipboardData> {
    if data.is_empty() {
        return None;
    }
    let mut out = ClipboardData { effects: Vec::new(), first_visible_row: -1, earliest_start: TimeSpan(i64::MAX) };
    let Some(descriptor) = vixen_effect_registry::all().iter().copied().find(|d| d.name == "LipSync") else { return Some(out) };
    for item in data.iter().filter(|d| d.duration.ticks() != 0) {
        let mut params = descriptor.new_params();
        params.set("StaticPhoneme", ParamValue::Enum(item.phoneme.name().to_string()));
        params.set("LyricData", ParamValue::Str(item.lyric.clone()));
        let Some(text) = clipboard::effect_data_text(&descriptor.type_id, &Uuid::new_v4(), &params) else { continue };
        out.effects.push(ClipboardEffect {
            type_id: descriptor.type_id,
            start: item.start,
            duration: item.duration,
            layer_id: Uuid::nil(),
            layer_name: String::new(),
            layer_type_id: Uuid::nil(),
            data: text,
            row_offset: 0,
        });
        out.earliest_start = out.earliest_start.min(item.start);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::marks::Mark;

    fn ms(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v)
    }

    fn collection(name: &str, starts: &[i64]) -> MarkCollection {
        let mut c = MarkCollection { name: name.to_string(), ..MarkCollection::default() };
        for s in starts {
            c.add_mark(Mark::new(ms(*s)));
        }
        c
    }

    fn dict() -> PhonemeDictionary {
        PhonemeDictionary::load(None)
    }

    fn delivered(outcome: ConvertOutcome) -> Delivery {
        match outcome {
            ConvertOutcome::Delivered(d) => d,
            other => panic!("not delivered: {other:?}"),
        }
    }

    #[test]
    fn start_offsets_read_as_dotnet_writes_a_time_span() {
        assert_eq!(start_offset_text(TimeSpan::ZERO), "00:00:00");
        assert_eq!(start_offset_text(ms(83_000)), "00:01:23");
        assert_eq!(start_offset_text(ms(1_500)), "00:00:01.5000000");
        assert_eq!(start_offset_text(TimeSpan(36_000_000_001)), "01:00:00.0000001");
        assert_eq!(start_offset_text(ms(90_000_000)), "1.01:00:00");
        assert_eq!(start_offset_text(ms(-250)), "-00:00:00.2500000");
    }

    #[test]
    fn the_marks_group_follows_the_radio_and_only_collections_with_marks_show() {
        let marks = vec![collection("Empty", &[]), collection("Words", &[1000, 2000, 3000]), collection("Beats", &[500])];
        let mut c = LyricConverter::new();
        c.refresh(&marks);
        assert_eq!(c.placement(), Placement::Cursor);
        assert!(c.mark_placement_available() && !c.marks_enabled());
        assert_eq!(c.collection_names(), ["Words", "Beats"]);
        assert_eq!((c.collection_index(), c.start_offset_index(), c.alignment()), (None, None, None));
        assert!(c.start_offsets().is_empty());

        c.set_placement(Placement::Mark, &marks);
        assert_eq!((c.collection_index(), c.start_offset_index(), c.alignment()), (Some(0), Some(0), Some(Alignment::Word)));
        assert_eq!(c.start_offsets(), [ms(1000), ms(2000), ms(3000)]);

        // The index survives a refresh, and a change of collection starts from its first mark.
        c.set_start_offset(2);
        c.refresh(&marks);
        assert_eq!(c.start_offset_index(), Some(2));
        c.set_collection(1);
        assert_eq!((c.collection_index(), c.start_offset_index()), (Some(1), Some(0)));
        assert_eq!(c.start_offsets(), [ms(500)]);

        // Fewer marks: the last one. Another count of collections: the first collection.
        c.set_collection(0);
        c.set_start_offset(2);
        let fewer = vec![collection("Words", &[1000, 2000]), collection("Beats", &[500])];
        c.refresh(&fewer);
        assert_eq!(c.start_offset_index(), Some(1));
        c.set_collection(1);
        c.refresh(&[collection("Words", &[1000, 2000])]);
        assert_eq!((c.collection_index(), c.start_offset_index()), (Some(0), Some(0)));

        // KEEP: back on Cursor the two lists clear and Alignment stays.
        c.set_alignment(Alignment::Phrase);
        c.set_placement(Placement::Clipboard, &fewer);
        assert_eq!((c.collection_index(), c.start_offset_index(), c.alignment()), (None, None, Some(Alignment::Phrase)));
        assert_eq!(c.collection_names(), ["Words", "Beats"]);
    }

    #[test]
    fn per_mark_collection_is_refused_with_no_marks_anywhere() {
        let marks = vec![collection("Empty", &[])];
        let mut c = LyricConverter::new();
        c.set_placement(Placement::Mark, &marks);
        assert_eq!(c.placement(), Placement::Cursor);
        assert!(!c.mark_placement_available());
        // GUARD: marks deleted while Per Mark Collection is on leave no stale first mark.
        let some = vec![collection("Words", &[1000, 2000])];
        c.set_placement(Placement::Mark, &some);
        c.refresh(&marks);
        assert_eq!((c.placement(), c.collection_index(), c.start_offset_index()), (Placement::Mark, None, None));
        let d = delivered(c.convert("hello", false, &dict()));
        assert_eq!(d.first_mark, TimeSpan::ZERO);
    }

    #[test]
    fn an_even_spacing_convert_gives_250_ms_effects_on_one_row() {
        let mut c = LyricConverter::new();
        c.refresh(&[]);
        let d = delivered(c.convert("Hello, world!", true, &dict()));
        assert_eq!((d.placement, d.first_mark, d.clear_text), (Placement::Cursor, TimeSpan::ZERO, true));
        let clip = d.clipboard.expect("a clipboard");
        assert_eq!(clip.first_visible_row, -1);
        assert_eq!(clip.earliest_start, TimeSpan::ZERO);
        let starts: Vec<i64> = clip.effects.iter().map(|e| e.start.ticks() / 10_000).collect();
        assert_eq!(starts, [0, 250, 500, 750, 1000, 1250, 1500, 1750]);
        assert!(clip.effects.iter().all(|e| e.duration == ms(250) && e.row_offset == 0));
        let names: Vec<String> = clip
            .effects
            .iter()
            .map(|e| {
                let params = clipboard::effect_data_from_text(&e.type_id, &e.data).expect("readable");
                assert_eq!(params.get("LyricData"), &ParamValue::Str(if e.start < ms(1000) { "Hello" } else { "world" }.to_string()));
                params.enum_name("StaticPhoneme").to_string()
            })
            .collect();
        assert_eq!(names, ["ETC", "AI", "L", "O", "WQ", "E", "L", "ETC"]);
        // The clipboard round-trips as the pasteboard carries it.
        assert_eq!(ClipboardData::from_json(&clip.to_json()), Some(clip));
    }

    #[test]
    fn per_mark_collection_takes_its_times_from_the_marks_and_walks_down_them() {
        let marks = vec![collection("Words", &[1000, 2000, 2600, 4000, 5000])];
        let mut c = LyricConverter::new();
        c.set_placement(Placement::Mark, &marks);
        c.set_start_offset(1);
        let d = delivered(c.convert("hello  world", false, &dict()));
        assert_eq!((d.placement, d.first_mark, d.clear_text), (Placement::Mark, ms(2000), false));
        let clip = d.clipboard.expect("a clipboard");
        // "hello" spans marks 2000 to 2600 in four phonemes, relative to the first mark of the collection.
        assert_eq!(clip.effects[0].start, ms(1000));
        assert_eq!(clip.effects[0].duration, ms(150));
        assert_eq!(clip.effects[4].start, ms(1600));
        assert_eq!(clip.effects[4].duration, ms(350));
        // Word alignment advances by every string, the blank between the two spaces included.
        assert_eq!(c.start_offset_index(), Some(4));
        // Out of range: it stays.
        delivered(c.convert("hello world", false, &dict()));
        assert_eq!(c.start_offset_index(), Some(4));

        c.set_start_offset(0);
        c.set_alignment(Alignment::Phoneme);
        let d = delivered(c.convert("the", false, &dict()));
        let clip = d.clipboard.expect("a clipboard");
        assert_eq!(clip.effects.iter().map(|e| (e.start, e.duration)).collect::<Vec<_>>(), [(ms(0), ms(1000)), (ms(1000), ms(600))]);
        assert_eq!(c.start_offset_index(), Some(2));

        c.set_alignment(Alignment::Phrase);
        delivered(c.convert("hello world\nthe", false, &dict()));
        assert_eq!(c.start_offset_index(), Some(3));
    }

    #[test]
    fn an_unknown_word_asks_once_and_a_second_failure_ends_the_convert() {
        let dir = std::env::temp_dir().join(format!("vixen-lyric-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user_dictionary");
        let marks = vec![collection("Words", &[1000, 2000, 3000])];
        let mut c = LyricConverter::new();
        c.set_placement(Placement::Mark, &marks);

        with_dictionary(&path, |d| {
            assert_eq!(c.convert("hello zzyzzx", true, d), ConvertOutcome::NeedsMapping("zzyzzx".to_string()));
            // Cancel: nothing delivered, the text stays (FIX), and Start Offset doesn't move.
            assert_eq!(c.resume(d), ConvertOutcome::Abandoned);
            assert_eq!(c.start_offset_index(), Some(0));

            assert_eq!(c.convert("hello zzyzzx", true, d), ConvertOutcome::NeedsMapping("zzyzzx".to_string()));
            d.add_user_mapping("zzyzzx", "E REST ", &path).unwrap();
            let delivery = delivered(c.resume(d));
            assert!(delivery.clear_text);
            assert_eq!(delivery.clipboard.unwrap().effects.len(), 6);
            assert_eq!(word_phonemes(d, " Zzyzzx! "), ["E", "REST"]);
            assert!(word_phonemes(d, "qqqq").is_empty());
        });
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "\nZZYZZX E REST ");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn closing_a_profile_reloads_its_dictionary_on_the_next_open() {
        let root = std::env::temp_dir().join(format!("vixen-dictionary-reopen-{}", Uuid::new_v4()));
        let core = crate::app::AppCore::open(root.clone(), true).unwrap();
        let path = root.join("user_dictionary");
        std::fs::write(&path, "\nZZVIXENREOPEN AI\n").unwrap();
        assert_eq!(with_dictionary(&path, |d| word_phonemes(d, "zzvixenreopen")), ["AI"]);
        std::fs::write(&path, "\nZZVIXENREOPEN MBP\n").unwrap();
        core.shutdown();
        assert_eq!(with_dictionary(&path, |d| word_phonemes(d, "zzvixenreopen")), ["MBP"]);
        release_dictionary(&path);
        drop(core);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn blank_text_delivers_nothing_to_paste() {
        let mut c = LyricConverter::new();
        c.refresh(&[]);
        let d = delivered(c.convert("  \n ", true, &dict()));
        assert_eq!((d.clipboard, d.clear_text), (None, false));
    }
}
