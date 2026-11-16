//! Opening, saving and closing rules, section A of the spec: the title bar, the save panel's extension rule, row settings, auto
//! save and the close prompt. All pure, so they're tested with fake clocks and no window.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use vixen_core::MarkCollection;
use vixen_io::RowSetting;

/// `SetTitleBarText`: `<sequence name or "Untitled">[ *] - <profile name>`. F4.
pub fn window_title(name: Option<&str>, modified: bool, profile: &str) -> String {
    let name = name.filter(|n| !n.is_empty()).unwrap_or("Untitled");
    format!("{name}{} - {profile}", if modified { " *" } else { "" })
}

/// The save panel's rule: a typed extension that differs has the proper one appended rather than replaced. F13. The comparison is
/// `!=` on the text, so `Song.TIM` gets `.tim` added.
pub fn save_path_with_extension(chosen: &Path, extension: &str) -> PathBuf {
    let existing = chosen.extension().map(|e| format!(".{}", e.to_string_lossy()));
    if existing.as_deref() == Some(extension) {
        return chosen.to_path_buf();
    }
    let mut name = chosen.as_os_str().to_os_string();
    name.push(extension);
    PathBuf::from(name)
}

/// What closing a window does. F16.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CloseAction {
    /// Ask. Without Cancel when the app itself is closing.
    Prompt { can_cancel: bool },
    /// Not modified, but zoom, scroll or a row height changed: saved with no prompt, as in the original (KEEP).
    SaveSilently,
    Close,
}

/// `_CloseEditor`.
pub fn close_action(modified: bool, state_modified: bool, app_closing: bool) -> CloseAction {
    if modified {
        CloseAction::Prompt { can_cancel: !app_closing }
    } else if state_modified {
        CloseAction::SaveSilently
    } else {
        CloseAction::Close
    }
}

/// `SaveGridRowSettings`' rule for one row: stored when the height differs from the default by more than 7, or when open. F14.
pub fn row_setting_to_store(height: i32, default_height: i32, tree_open: bool, visible: bool) -> Option<RowSetting> {
    if height > default_height + 7 || height < default_height - 7 {
        Some(RowSetting { row_height: height, expanded: tree_open, visible })
    } else if tree_open {
        Some(RowSetting { row_height: height, expanded: true, visible })
    } else {
        None
    }
}

/// `_autoSaveTimer` and `SetAutoSave`. F15.
///
/// It re-arms only on the transition of the modified flag, so the interval counts from the first edit after a save, not the last.
#[derive(Clone, Debug)]
pub struct AutoSaveClock {
    interval: Duration,
    deadline: Option<Instant>,
}

impl AutoSaveClock {
    pub fn new(interval_ms: i32) -> AutoSaveClock {
        AutoSaveClock { interval: Duration::from_millis(interval_ms.max(1) as u64), deadline: None }
    }

    fn set_auto_save(&mut self, enabled: bool, modified: bool, now: Instant) {
        if enabled && modified {
            // `Timer.Start` on a running timer changes nothing.
            if self.deadline.is_none() {
                self.deadline = Some(now + self.interval);
            }
        } else {
            self.deadline = None;
        }
    }

    /// `SequenceModified` and `SequenceNotModified`: only the transition reaches here.
    pub fn on_modified_changed(&mut self, modified: bool, enabled: bool, now: Instant) {
        self.set_auto_save(enabled, modified, now);
    }

    /// The Auto Save items.
    pub fn set_enabled(&mut self, on: bool, modified: bool, now: Instant) {
        self.set_auto_save(on, modified, now);
    }

    /// `AutoSaveEventProcessor`: the timer stops itself. True means save if still modified. A never-saved sequence then raises the save
    /// panel, and cancelling leaves it modified with the timer stopped until the next save or toggle (KEEP).
    pub fn due(&mut self, now: Instant) -> bool {
        match self.deadline {
            Some(deadline) if now >= deadline => {
                self.deadline = None;
                true
            }
            _ => false,
        }
    }

    pub fn is_armed(&self) -> bool {
        self.deadline.is_some()
    }
}

/// `MarkCollectionNameService.NormalizeName`.
fn normalize_name(name: &str) -> &str {
    name.trim()
}

fn lower(name: &str) -> String {
    name.to_lowercase()
}

fn unique_name(base: &str, occupied: &std::collections::HashSet<String>) -> String {
    if !occupied.contains(&lower(base)) {
        return base.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base} - {suffix}");
        if !occupied.contains(&lower(&candidate)) {
            return candidate;
        }
        suffix += 1;
    }
}

/// `MarkCollectionNameService.GetUniqueName` over a list of names. The spec's F298.
pub fn unique_mark_collection_name(desired: &str, collections: &[MarkCollection]) -> String {
    let base = normalize_name(desired);
    let occupied = collections.iter().map(|c| lower(normalize_name(&c.name))).filter(|n| !n.is_empty()).collect();
    unique_name(base, &occupied)
}

/// `MarkCollectionNameService.RenameDuplicates`: the first of each name stays, later ones get ` - 2` and so on, and a blank one takes
/// "Mark Collection". Returns `(old name, new name)` for the F11 summary.
pub fn rename_duplicate_mark_collections(collections: &mut [MarkCollection]) -> Vec<(String, String)> {
    use std::collections::HashSet;
    let mut records = Vec::new();
    let mut occupied: HashSet<String> =
        collections.iter().map(|c| lower(normalize_name(&c.name))).filter(|n| !n.is_empty()).collect();
    let mut seen: HashSet<String> = HashSet::new();
    for collection in collections.iter_mut() {
        let original = normalize_name(&collection.name).to_string();
        if original.is_empty() {
            let replacement = unique_name("Mark Collection", &occupied);
            let blank = std::mem::replace(&mut collection.name, replacement.clone());
            occupied.insert(lower(&replacement));
            seen.insert(lower(&replacement));
            records.push((blank, replacement));
            continue;
        }
        if seen.insert(lower(&original)) {
            if collection.name != original {
                let previous = std::mem::replace(&mut collection.name, original.clone());
                records.push((previous, original));
            }
            continue;
        }
        let unique = unique_name(&original, &occupied);
        let old = std::mem::replace(&mut collection.name, unique.clone());
        occupied.insert(lower(&unique));
        records.push((old, unique));
    }
    records
}

/// `FormatMarkCollectionName`, for the summary.
fn format_mark_collection_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "(blank)".to_string()
    } else {
        trimmed.to_string()
    }
}

/// `ShowMarkCollectionRenameSummary`'s message. F11.
pub fn mark_rename_summary(records: &[(String, String)]) -> String {
    let lines: Vec<String> = records
        .iter()
        .map(|(old, new)| format!("{} -> {}", format_mark_collection_name(old), format_mark_collection_name(new)))
        .collect();
    format!("The following duplicate Mark Collections were renamed so effects can select them correctly:\n\n{}", lines.join("\n"))
}

/// `CheckDeprecatedEffects`' message. F10.
pub fn deprecated_elements_message(counts: &[(String, u32)]) -> String {
    let lines: Vec<String> =
        counts.iter().map(|(name, n)| format!("{name} ({n} effect{})", if *n == 1 { "" } else { "s" })).collect();
    format!("This sequence has effects on the following elements tagged as Deprecated:\n\n{}", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_follows_the_original() {
        assert_eq!(window_title(None, false, "P"), "Untitled - P");
        assert_eq!(window_title(Some(""), false, "P"), "Untitled - P");
        assert_eq!(window_title(Some("Song"), true, "P"), "Song * - P");
        assert_eq!(window_title(Some("Song"), false, "P"), "Song - P");
    }

    #[test]
    fn extension_is_appended_not_replaced() {
        assert_eq!(save_path_with_extension(Path::new("Song.txt"), ".tim"), PathBuf::from("Song.txt.tim"));
        assert_eq!(save_path_with_extension(Path::new("Song.tim"), ".tim"), PathBuf::from("Song.tim"));
        assert_eq!(save_path_with_extension(Path::new("Song"), ".tim"), PathBuf::from("Song.tim"));
        assert_eq!(save_path_with_extension(Path::new("Song.TIM"), ".tim"), PathBuf::from("Song.TIM.tim"));
    }

    #[test]
    fn close_actions() {
        assert_eq!(close_action(true, false, true), CloseAction::Prompt { can_cancel: false });
        assert_eq!(close_action(true, true, false), CloseAction::Prompt { can_cancel: true });
        assert_eq!(close_action(false, true, false), CloseAction::SaveSilently);
        assert_eq!(close_action(false, false, false), CloseAction::Close);
    }

    #[test]
    fn row_settings_buffer_of_seven() {
        assert_eq!(row_setting_to_store(38, 30, false, true), Some(RowSetting { row_height: 38, expanded: false, visible: true }));
        assert_eq!(row_setting_to_store(37, 30, false, true), None);
        assert_eq!(row_setting_to_store(22, 30, false, true), Some(RowSetting { row_height: 22, expanded: false, visible: true }));
        assert_eq!(row_setting_to_store(23, 30, false, true), None);
        assert_eq!(row_setting_to_store(30, 30, true, false), Some(RowSetting { row_height: 30, expanded: true, visible: false }));
        // Outside the buffer while open keeps its real open state.
        assert_eq!(row_setting_to_store(50, 30, true, true), Some(RowSetting { row_height: 50, expanded: true, visible: true }));
    }

    #[test]
    fn auto_save_counts_from_the_first_edit() {
        let t = Instant::now();
        let mut clock = AutoSaveClock::new(300_000);
        assert!(!clock.is_armed());
        clock.on_modified_changed(true, true, t);
        assert!(clock.is_armed());
        // A later edit doesn't reach here (only the transition does), and the deadline stands.
        assert!(!clock.due(t + Duration::from_secs(299)));
        assert!(clock.due(t + Duration::from_secs(300)));
        // It stopped itself. A cancelled save panel leaves it stopped until the next transition.
        assert!(!clock.is_armed());
        assert!(!clock.due(t + Duration::from_secs(900)));
    }

    #[test]
    fn saving_and_unchecking_stop_it_and_rechecking_while_modified_rearms() {
        let t = Instant::now();
        let mut clock = AutoSaveClock::new(1000);
        clock.on_modified_changed(true, true, t);
        clock.on_modified_changed(false, true, t);
        assert!(!clock.is_armed());
        clock.on_modified_changed(true, true, t);
        clock.set_enabled(false, true, t);
        assert!(!clock.is_armed());
        clock.set_enabled(true, true, t + Duration::from_secs(5));
        assert!(clock.is_armed());
        assert!(!clock.due(t + Duration::from_millis(5500)));
        assert!(clock.due(t + Duration::from_secs(6)));
        // Disabled: an edit doesn't arm it.
        clock.on_modified_changed(true, false, t);
        assert!(!clock.is_armed());
    }

    fn named(names: &[&str]) -> Vec<MarkCollection> {
        names.iter().map(|n| MarkCollection { name: n.to_string(), ..MarkCollection::default() }).collect()
    }

    #[test]
    fn duplicate_collections_are_renamed_and_blank_gets_a_name() {
        let mut c = named(&["Beats", "beats", "  ", "Beats - 2", "Phrases "]);
        let records = rename_duplicate_mark_collections(&mut c);
        let names: Vec<&str> = c.iter().map(|m| m.name.as_str()).collect();
        // The first keeps its name, suffixes skip ones in use, and a stray space is trimmed and reported.
        assert_eq!(names, ["Beats", "beats - 3", "Mark Collection", "Beats - 2", "Phrases"]);
        assert_eq!(
            records,
            vec![
                ("beats".to_string(), "beats - 3".to_string()),
                ("  ".to_string(), "Mark Collection".to_string()),
                ("Phrases ".to_string(), "Phrases".to_string()),
            ]
        );
        assert!(mark_rename_summary(&records).ends_with("beats -> beats - 3\n(blank) -> Mark Collection\nPhrases -> Phrases"));
    }

    #[test]
    fn unique_name_compares_without_case() {
        let c = named(&["Default Marks"]);
        assert_eq!(unique_mark_collection_name("default marks", &c), "default marks - 2");
        assert_eq!(unique_mark_collection_name("Other", &c), "Other");
    }

    #[test]
    fn deprecated_message_counts_effects() {
        let text = deprecated_elements_message(&[("Tree".to_string(), 1), ("Arch".to_string(), 3)]);
        assert_eq!(text, "This sequence has effects on the following elements tagged as Deprecated:\n\nTree (1 effect)\nArch (3 effects)");
    }
}
