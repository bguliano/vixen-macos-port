use super::*;
use vixen_core::marks::MarkDecorator;

fn named(names: &[&str]) -> Vec<MarkCollection> {
    names.iter().map(|n| MarkCollection { name: n.to_string(), ..MarkCollection::default() }).collect()
}

fn names(c: &[MarkCollection]) -> Vec<&str> {
    c.iter().map(|m| m.name.as_str()).collect()
}

#[test]
fn unique_names_follow_the_name_service() {
    assert_eq!(unique_name("Mark Collection", &[], None), "Mark Collection");
    assert_eq!(unique_name("Mark Collection", &named(&["Mark Collection"]), None), "Mark Collection - 2");
    assert_eq!(unique_name("Mark Collection", &named(&["Mark Collection", "mark collection - 2"]), None), "Mark Collection - 3");
    assert_eq!(unique_name("  Beats  ", &named(&["Other"]), None), "Beats");
    assert_eq!(unique_name("", &[], None), "Mark Collection");
    let c = named(&["Beat Marks"]);
    assert!(!is_unique_name(&c, "beat marks", None));
    assert!(!is_unique_name(&c, " Beat Marks ", None));
    assert!(is_unique_name(&c, "beat marks", Some(c[0].id)));
    assert!(!is_unique_name(&c, "   ", None));
    assert_eq!(unique_name("Beat Marks", &c, Some(c[0].id)), "Beat Marks");
    // Ordinal-ignore-case upper-cases one character at a time.
    assert!(is_unique_name(&named(&["STRASSE"]), "straße", None));
}

#[test]
fn rows_carry_every_column() {
    let mut c = named(&["A"]);
    c[0].show_grid_lines = false;
    c[0].show_tail_lines = true;
    c[0].decorator = MarkDecorator { color: Color::rgb(1, 2, 3), is_bold: true, is_solid_line: false, compact_mode: false };
    let r = &rows(&c)[0];
    assert!(!r.show_lines && !r.tail_enabled && r.show_tail);
    assert_eq!(r.color, Color::rgb(1, 2, 3));
    assert!(r.bold && !r.solid);
    assert_eq!(r.level, 1);
}

#[test]
fn header_flags_start_from_the_rows_and_flip_per_click() {
    // Mixed rows: Lines sets all on, then all off.
    let mut c = named(&["A", "B"]);
    c[1].show_grid_lines = false;
    let mut h = HeaderFlags::new(&c);
    assert_eq!(h.click(MarkHeader::Lines, &mut c), MarksChange::Properties);
    assert!(c.iter().all(|m| m.show_grid_lines));
    h.click(MarkHeader::Lines, &mut c);
    assert!(c.iter().all(|m| !m.show_grid_lines));
    // All on at creation: the first click sets all off.
    let mut c = named(&["A", "B"]);
    let mut h = HeaderFlags::new(&c);
    h.click(MarkHeader::Lines, &mut c);
    assert!(c.iter().all(|m| !m.show_grid_lines));
    // A click that changes nothing still flips, and reports no change.
    let mut c = named(&["A"]);
    c[0].is_locked = true;
    let mut h = HeaderFlags { lines: true, tail: true, bar: true, lock: true };
    assert_eq!(h.click(MarkHeader::Lock, &mut c), MarksChange::None);
    assert!(!h.lock);
}

#[test]
fn tail_header_with_the_fix_matches_lines() {
    let mut c = named(&["A", "B"]);
    let mut h = HeaderFlags::new(&c);
    h.click(MarkHeader::TailLines, &mut c);
    if TAIL_HEADER_FIX {
        assert!(c.iter().all(|m| m.show_tail_lines));
    } else {
        assert!(c.iter().all(|m| !m.show_tail_lines));
        h.click(MarkHeader::TailLines, &mut c);
        assert!(c.iter().all(|m| m.show_tail_lines));
    }
}

#[test]
fn header_flags_on_an_empty_list() {
    let mut empty: Vec<MarkCollection> = Vec::new();
    let mut h = HeaderFlags::new(&empty);
    assert!(!h.lines && !h.bar && !h.lock);
    assert_eq!(h.tail, !TAIL_HEADER_FIX);
    h.click(MarkHeader::Bar, &mut empty);
    assert!(!h.bar, "Bar can't run on an empty list, so its flag stays");
    h.click(MarkHeader::Lines, &mut empty);
    assert!(h.lines, "Lines has no rule, so its flag flips");
}

#[test]
fn new_collections_and_the_default() {
    let mut c = Vec::new();
    let first = add_collection(&mut c);
    let second = add_collection(&mut c);
    assert_eq!(names(&c), ["Mark Collection", "Mark Collection - 2"]);
    assert!(c[0].is_default && !c[1].is_default);
    assert_eq!((c[0].id, c[1].id), (first, second));
    assert!(c[1].show_grid_lines && !c[1].show_marks && !c[1].show_tail_lines && c[1].level == 1);
}

#[test]
fn deleting_the_default_passes_it_to_the_first_visible() {
    let mut c = named(&["A", "B", "C"]);
    c[0].is_default = true;
    c[1].show_grid_lines = false;
    c[1].show_marks = false;
    let id = c[0].id;
    assert_eq!(delete_collection(&mut c, id), MarksChange::Collections);
    assert_eq!(names(&c), ["B", "C"]);
    assert!(!c[0].is_default && c[1].is_default);
    // With none visible, the first.
    let mut c = named(&["A", "B"]);
    c[0].is_default = true;
    c[1].show_grid_lines = false;
    let id = c[0].id;
    delete_collection(&mut c, id);
    assert!(c[0].is_default);
    // Links to the deleted one are left dangling. KEEP.
    let mut c = named(&["Words", "Phonemes"]);
    c[1].linked_mark_collection_id = Some(c[0].id);
    let words = c[0].id;
    delete_collection(&mut c, words);
    assert_eq!(c[0].linked_mark_collection_id, Some(words));
}

#[test]
fn active_radio_clears_the_others() {
    let mut c = named(&["A", "B"]);
    c[0].is_default = true;
    let b = c[1].id;
    assert_eq!(set_active(&mut c, b), MarksChange::Properties);
    assert!(!c[0].is_default && c[1].is_default);
    assert_eq!(set_active(&mut c, b), MarksChange::None);
}

#[test]
fn flags_report_only_real_changes() {
    let mut c = named(&["A"]);
    let id = c[0].id;
    assert_eq!(set_flag(&mut c, id, MarkFlag::Lines, true), MarksChange::None);
    assert_eq!(set_flag(&mut c, id, MarkFlag::Bar, true), MarksChange::Properties);
    assert!(c[0].show_marks);
    assert_eq!(set_flag(&mut c, id, MarkFlag::Bold, true), MarksChange::Properties);
    assert!(c[0].decorator.is_bold);
}

#[test]
fn rename_rules() {
    let mut c = named(&["X", "Other"]);
    let id = c[1].id;
    assert_eq!(rename_error(&c, id, "x"), Some("A Mark Collection named \"x\" already exists.".to_string()));
    assert_eq!(rename_error(&c, id, "   "), None);
    assert_eq!(end_rename(&mut c, id, "  "), RenameEnd::Restored);
    assert_eq!(end_rename(&mut c, id, " x "), RenameEnd::StillInvalid);
    assert_eq!(end_rename(&mut c, id, "  New  "), RenameEnd::Renamed);
    assert_eq!(c[1].name, "New");
    assert_eq!(end_rename(&mut c, id, "New"), RenameEnd::Unchanged);
    assert_eq!(end_rename(&mut c, id, "NEW"), RenameEnd::Renamed);
    assert_eq!(c[1].name, "NEW");
}

#[test]
fn offset_time_moves_starts_without_clamping() {
    let mut c = named(&["A"]);
    c[0].marks = vec![Mark::new(TimeSpan::from_ms_i(500)), Mark::new(TimeSpan::from_ms_i(2000))];
    let id = c[0].id;
    assert_eq!(offset_time(&mut c, id, 0), MarksChange::None);
    assert_eq!(offset_time(&mut c, id, -1000), MarksChange::Collections);
    let starts: Vec<f64> = c[0].marks.iter().map(|m| m.start_time.total_ms()).collect();
    assert_eq!(starts, [-500.0, 1000.0]);
    assert!(c[0].marks.iter().all(|m| m.duration == TimeSpan::from_ms_i(450)));
}

#[test]
fn links_offer_the_parent_type_and_keep_one_child_per_parent() {
    let mut c = named(&["Phrases", "Words 1", "Words 2", "Phonemes", "Plain"]);
    c[0].collection_type = MarkCollectionType::Phrase;
    c[1].collection_type = MarkCollectionType::Word;
    c[2].collection_type = MarkCollectionType::Word;
    c[3].collection_type = MarkCollectionType::Phoneme;
    let (phrases, w1, w2, phonemes, plain) = (c[0].id, c[1].id, c[2].id, c[3].id, c[4].id);
    assert!(link_choices(&c, plain).is_none());
    assert!(link_choices(&c, phrases).is_none());
    let for_phonemes = link_choices(&c, phonemes).unwrap();
    let labels: Vec<&str> = for_phonemes.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(labels, ["Words 1", "Words 2", "None"]);
    assert!(for_phonemes[2].checked);
    assert_eq!(set_link(&mut c, w1, Some(phrases)), MarksChange::Properties);
    assert_eq!(set_link(&mut c, w2, Some(phrases)), MarksChange::Properties);
    assert_eq!(c[1].linked_mark_collection_id, None);
    assert_eq!(c[2].linked_mark_collection_id, Some(phrases));
    let for_w2 = link_choices(&c, w2).unwrap();
    assert!(for_w2[0].checked && !for_w2[1].checked);
    // A type change keeps the link, hidden.
    set_kind(&mut c, w2, MarkCollectionType::Generic);
    assert!(link_choices(&c, w2).is_none());
    assert_eq!(c[2].linked_mark_collection_id, Some(phrases));
    assert_eq!(set_link(&mut c, w1, None), MarksChange::None);
}

#[test]
fn import_options_flag_names_already_in_the_sequence() {
    let existing = named(&["Beats"]);
    let candidates = named(&["beats ", "New", ""]);
    let options = import_options(&existing, &candidates);
    let flags: Vec<bool> = options.iter().map(|o| o.has_duplicate_name).collect();
    assert_eq!(flags, [true, false, true]);
}

#[test]
fn commit_names_links_and_defaults_as_upstream() {
    // Sequential names among candidates and against the sequence.
    let mut target = named(&["Audacity Marks"]);
    target[0].is_default = true;
    let mut cands = named(&["Audacity Marks", "Audacity Marks", ""]);
    cands[0].is_default = true;
    assert!(commit_import(&mut target, cands));
    assert_eq!(names(&target), ["Audacity Marks", "Audacity Marks - 2", "Audacity Marks - 3", "Mark Collection"]);
    assert_eq!(target.iter().filter(|c| c.is_default).count(), 1, "an existing default clears the candidates'");
    // No default: the first flagged, then the first visible, then the first.
    let mut target = Vec::new();
    let mut cands = named(&["A", "B", "C"]);
    cands[2].is_default = true;
    commit_import(&mut target, cands);
    assert!(target[2].is_default && !target[0].is_default);
    let mut target = Vec::new();
    let mut cands = named(&["A", "B"]);
    cands[0].show_grid_lines = false;
    commit_import(&mut target, cands);
    assert!(target[1].is_default);
    let mut target = Vec::new();
    let mut cands = named(&["A", "B"]);
    for c in &mut cands {
        c.show_grid_lines = false;
    }
    commit_import(&mut target, cands);
    assert!(target[0].is_default);
    // Links: to an unselected candidate clears; to an existing or selected one stays.
    let mut target = named(&["Existing"]);
    let existing = target[0].id;
    let phrases = MarkCollection::default();
    let mut words = named(&["Words"]).remove(0);
    words.linked_mark_collection_id = Some(phrases.id);
    let mut kept = named(&["Kept"]).remove(0);
    kept.linked_mark_collection_id = Some(existing);
    let mut chained = named(&["Chained"]).remove(0);
    chained.linked_mark_collection_id = Some(kept.id);
    commit_import(&mut target, vec![words, kept.clone(), chained]);
    assert_eq!(target[1].linked_mark_collection_id, None);
    assert_eq!(target[2].linked_mark_collection_id, Some(existing));
    assert_eq!(target[3].linked_mark_collection_id, Some(kept.id));
    // Nothing selected changes nothing.
    let mut target = named(&["A"]);
    assert!(!commit_import(&mut target, Vec::new()));
    assert_eq!(target.len(), 1);
}

#[test]
fn routing_keeps_upstreams_crossed_readers() {
    let bars = "1.0\t2.0\tx\n";
    let ImportRead::Candidates(c) = read_candidates(ImportKind::AudacityBeats, bars.as_bytes(), "file") else { panic!() };
    assert_eq!(c[0].name, "Audacity Marks");
    let ImportRead::Candidates(c) = read_candidates(ImportKind::VampBeats, b"1.0\t1.0\t1\n", "file") else { panic!() };
    assert_eq!(c[0].name, "Audacity Beat 1 Marks");
    let ImportRead::Failed { message, title, .. } = read_candidates(ImportKind::VampBars, b"oops\n", "file") else { panic!() };
    assert_eq!((message, title), ("There was an error importing the Audacity bar marks.", "Audacity Import Error"));
    let two_colours = "#,Name,Start,Color\nM1,a,00:01.000,112233\nM2,b,00:02.000,0000FF\n";
    assert!(matches!(read_candidates(ImportKind::PangolinBeyond, two_colours.as_bytes(), "file"), ImportRead::PangolinChoice(_)));
    let one_colour = "#,Name,Start,Color\nM1,a,00:01.000,112233\n";
    let ImportRead::Candidates(c) = read_candidates(ImportKind::PangolinBeyond, one_colour.as_bytes(), "file") else { panic!() };
    assert_eq!(c[0].name, "Beyond Marks - #332211");
    assert!(ImportKind::SingingFaces.filter().is_none());
}

#[test]
fn papagayo_gives_three_collections_a_voice_after_its_summary() {
    let pgo = "lipsync version 1\nC:\\song.wav\n24\n48\n2\n\tLead\n\thi there\n\t1\n\t\thi there\n\t\t0\n\t\t23\n\t\t2\n\t\t\thi 0 11 1\n\t\t\t\t0 AI\n\t\t\tthere 12 23 2\n\t\t\t\t12 etc\n\t\t\t\t18 E\n\tBacking\n\t\n\t0\n";
    // A UTF-16 file with a byte order mark reads like any other (F116).
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(pgo.encode_utf16().flat_map(u16::to_le_bytes));
    let ImportRead::CandidatesAfterMessage { title, message, candidates } = read_candidates(ImportKind::Papagayo, &bytes, "My Song") else { panic!() };
    assert_eq!(title, "Papagayo Import");
    assert_eq!(message, "2 voices imported as Mark Collections\n\nRow #1 - Lead\nRow #2 - Backing");
    let names: Vec<&str> = candidates.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["My Song Lead Phrases", "My Song Lead Words", "My Song Lead Phonemes", "My Song Backing Phrases", "My Song Backing Words", "My Song Backing Phonemes"]);
    assert_eq!(candidates[1].marks.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(), ["hi", "there"]);
    assert_eq!(candidates[2].marks.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(), ["AI", "etc", "E"]);
    assert_eq!(candidates[2].marks[2].start_time, TimeSpan::from_ms(((1.0f32 / 24.0) * 1000.0 * 18.0) as f64));

    // Committed, the links between a voice's three survive and the names stay as they are.
    let mut target = named(&["A"]);
    assert!(commit_import(&mut target, candidates));
    assert_eq!(target.len(), 7);
    assert_eq!(target[2].linked_mark_collection_id, Some(target[1].id));
    assert_eq!(target[3].linked_mark_collection_id, Some(target[2].id));

    // An empty file is a document with no voices: the summary, then nothing to choose.
    let ImportRead::CandidatesAfterMessage { message, candidates, .. } = read_candidates(ImportKind::Papagayo, b"", "x") else { panic!() };
    assert_eq!(message, "0 voices imported as Mark Collections\n\n");
    assert!(candidates.is_empty());
    let ImportRead::Failed { message, title, .. } = read_candidates(ImportKind::Papagayo, b"not one\n", "x") else { panic!() };
    assert_eq!((message, title), ("There was an error importing the Papagayo marks.", "Papagayo Import Error"));
    assert_eq!(not_built(ImportKind::Papagayo), None);
}

#[test]
fn export_includes_in_list_order_and_text_defaults_by_type() {
    let mut c = named(&["Generic", "Words"]);
    c[1].collection_type = MarkCollectionType::Word;
    c[0].marks.push(Mark::new(TimeSpan::from_ms_i(1000)));
    c[1].marks.push(Mark::new(TimeSpan::from_ms_i(500)));
    let r = export_rows(&c);
    assert!(!r[0].include_text && r[1].include_text);
    let ids = [c[1].id, c[0].id];
    let text = String::from_utf8(build_export(ExportFormat::Audacity, &c, &ids, &[]).unwrap()).unwrap();
    assert_eq!(text, "0000.500\t0000.500\t2\r\n0001.000\t0001.000\t1\r\n");
    let empty = String::from_utf8(build_export(ExportFormat::PangolinBeyond, &c, &[], &[]).unwrap()).unwrap();
    assert_eq!(empty, "#,Name,Start,Color\r\n");
}

#[test]
fn last_folder_follows_the_last_file_chosen() {
    let root = Path::new("/profile");
    remember_folder(Path::new("/music/song/beats.txt"));
    assert_eq!(last_folder(root), Path::new("/music/song"));
}

use std::path::Path;

mod undo_hazard {
    use super::*;
    use crate::sequence::{SequenceSession, SessionEnvironment};
    use crate::undo::UndoAction;
    use std::sync::Arc;
    use vixen_core::ElementTree;
    use vixen_io::ModuleStore;

    fn session() -> SequenceSession {
        let env = SessionEnvironment { video_cache: None,
            tree: Arc::new(ElementTree::new()),
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 25,
            media_dir: std::env::temp_dir(),
        };
        SequenceSession::new_untitled(&env)
    }

    /// A `MarksChanged` command, then a panel action, then Undo: the marks revert and the panel's work stays.
    #[test]
    fn undoing_a_snapshot_keeps_collections_and_names_added_since() {
        let mut s = session();
        let mut marks = named(&["Beats"]);
        marks[0].marks.push(Mark::new(TimeSpan::from_ms_i(1000)));
        s.set_marks(marks);
        let before = s.marks().to_vec();
        let mut after = before.clone();
        after[0].marks[0].start_time = TimeSpan::from_ms_i(2000);
        s.set_marks(after);
        s.record_undo(UndoAction::MarksChanged { label: "Bulk Change 1 Mark".to_string(), marks: before });
        let mut panel = s.marks().to_vec();
        add_collection(&mut panel);
        let id = panel[0].id;
        end_rename(&mut panel, id, "Renamed");
        publish(&mut s, panel, MarksChange::Collections);
        assert_eq!(s.undo(1), 1);
        assert_eq!(names(s.marks()), ["Renamed", "Mark Collection"]);
        assert_eq!(s.marks()[0].marks[0].start_time, TimeSpan::from_ms_i(1000));
        assert_eq!(s.redo(1), 1);
        assert_eq!(names(s.marks()), ["Renamed", "Mark Collection"]);
        assert_eq!(s.marks()[0].marks[0].start_time, TimeSpan::from_ms_i(2000));
    }

    #[test]
    fn property_changes_modify_without_touching_marks() {
        let mut s = session();
        s.set_marks(named(&["A"]));
        s.set_unmodified();
        let mut c = s.marks().to_vec();
        let id = c[0].id;
        let change = set_flag(&mut c, id, MarkFlag::Bar, true);
        publish(&mut s, c, change);
        assert!(s.is_modified());
        assert!(s.marks()[0].show_marks);
    }
}

mod beat_detection {
    use super::*;
    use crate::marks::beats::{BeatDetection, BeatJob};
    use crate::sequence::{SequenceSession, SessionEnvironment};
    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use vixen_audio::AudioClip;
    use vixen_core::ElementTree;
    use vixen_io::{MediaEntry, ModuleStore};

    fn session_with_clicks() -> Arc<Mutex<SequenceSession>> {
        let env = SessionEnvironment { video_cache: None,
            tree: Arc::new(ElementTree::new()),
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 25,
            media_dir: std::env::temp_dir(),
        };
        let mut s = SequenceSession::new_untitled(&env);
        // A click every half second for 30 seconds, stereo, so the raw conversion tracks one channel at the true speed.
        let rate = 44_100;
        let mut samples = vec![0.0f32; rate * 30 * 2];
        let mut seed = 1u32;
        for beat in 0..60 {
            let at = beat * rate / 2;
            for i in 0..400 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                let v = ((seed >> 16) as f32 / 32_768.0 - 1.0) * (1.0 - i as f32 / 400.0);
                samples[(at + i) * 2] = v;
                samples[(at + i) * 2 + 1] = v;
            }
        }
        s.set_audio_clip(Some(Arc::new(AudioClip::from_interleaved(samples, 2, rate as u32))));
        s.document_mut().media.push(MediaEntry { type_id: Uuid::nil(), instance_id: Uuid::nil(), file_name: "clicks.missing.wav".to_string(), data: None });
        s.set_marks(named(&["Mine"]));
        Arc::new(Mutex::new(s))
    }

    fn wait_for(d: &BeatDetection, done: impl Fn(&BeatJob) -> bool) -> BeatJob {
        let start = Instant::now();
        loop {
            let job = d.job();
            if done(&job) || start.elapsed() > Duration::from_secs(120) {
                return job;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn a_click_track_runs_through_preview_and_generation() {
        let session = session_with_clicks();
        let d = BeatDetection::start(Arc::clone(&session), &std::env::temp_dir());
        let job = wait_for(&d, |j| *j != BeatJob::Preparing);
        assert_eq!(job, BeatJob::Ready, "{job:?}");
        let model = d.model();
        assert!((model.bpm() - 120.0).abs() < 3.0, "tempo {}", model.bpm());
        assert_eq!(d.preview_peaks().len(), crate::marks::beats::PREVIEW_COLUMNS);
        d.generate();
        let job = wait_for(&d, |j| matches!(j, BeatJob::Finished | BeatJob::Failed(_) | BeatJob::Cancelled));
        assert_eq!(job, BeatJob::Finished);
        let s = session.lock();
        let names: Vec<&str> = s.marks().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names[0], "Mine");
        assert!(names.contains(&"Beats - All"), "{names:?}");
        assert!(s.marks()[0].is_default, "step 6 made the first collection the default");
    }

    #[test]
    fn cancel_at_the_dialog_changes_only_the_default() {
        let session = session_with_clicks();
        let d = BeatDetection::start(Arc::clone(&session), &std::env::temp_dir());
        wait_for(&d, |j| *j != BeatJob::Preparing);
        d.cancel();
        assert_eq!(d.job(), BeatJob::Cancelled);
        let s = session.lock();
        assert_eq!(s.marks().len(), 1);
        assert!(s.marks()[0].is_default);
    }
}
