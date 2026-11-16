//! The profile list in `Settings.xml`: what to load at launch, Select a Profile's rows, and the rules of the Data Profile dialog.
//! Port of `VixenApplication.ProcessProfiles`, `SelectProfile` and `DataProfileForm`, without their windows.

use crate::datetime::LocalDateTime;
use crate::error::IoError;
use crate::lock::{is_profile_locked, lock_owner};
use crate::paths::{DataPaths, VIXEN_DATA_DIR};
use crate::settings::{ProfileSettings, SettingSection::Profiles};
use std::path::{Path, PathBuf};

/// `LoadAction` when the user wants to be asked.
const ASK: &str = "Ask";
const LOAD_SELECTED: &str = "LoadSelected";
const NEW_PROFILE_NAME: &str = "New Profile";

/// A row of Select a Profile, or an item of the Data Profile dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileEntry {
    pub name: String,
    pub data_folder: String,
    pub date_last_loaded: Option<LocalDateTime>,
    /// Its `Profile{n}` in `Settings.xml`.
    pub number: i32,
    pub is_locked: bool,
}

/// What to do about a profile at launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupProfile {
    /// No profiles are saved. The data folder falls through to the default, with no name in the title.
    NoProfiles,
    Load { folder: String, name: String },
    /// Show Select a Profile, after the error if there is one.
    Ask { error: Option<ProfileLoadError> },
}

/// Why a chosen profile can't load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileLoadError {
    pub name: String,
    pub folder: String,
    pub is_locked: bool,
    /// The machine named by a lock from somewhere else, so the alert can offer to open it anyway. A copied folder's lock would
    /// otherwise strand it, since that machine will never clear it.
    pub locked_by_machine: Option<String>,
}

impl ProfileLoadError {
    /// `ShowLoadProfileErrorMessage`.
    pub fn message(&self) -> String {
        format!(
            "Selected profile {} {}!\n\nSelect a different profile to load or use the Profile Editor to create a new profile.",
            self.name,
            if self.is_locked { "is locked by another instance" } else { "data directory does not exist" }
        )
    }
}

/// `ProcessProfiles` up to its dialog loop.
pub fn decide_startup_profile(settings: &ProfileSettings) -> StartupProfile {
    let count = settings.get_i32(Profiles, "ProfileCount", 0);
    if count == 0 {
        return StartupProfile::NoProfiles;
    }
    let action = settings.get_string(Profiles, "LoadAction", LOAD_SELECTED);
    let to_load = settings.get_i32(Profiles, "ProfileToLoad", -1);
    if action != ASK && to_load > -1 && to_load < count {
        let folder = settings.get_string(Profiles, &format!("Profile{to_load}/DataFolder"), "");
        let name = settings.get_string(Profiles, &format!("Profile{to_load}/Name"), "");
        return match check_profile(&folder, &name) {
            Ok(()) => StartupProfile::Load { folder, name },
            Err(error) => StartupProfile::Ask { error: Some(error) },
        };
    }
    StartupProfile::Ask { error: None }
}

/// The three checks a profile must pass to load: named a folder, the folder exists, and it isn't locked.
pub fn check_profile(folder: &str, name: &str) -> Result<(), ProfileLoadError> {
    let path = Path::new(folder);
    let is_locked = is_profile_locked(path);
    if !folder.is_empty() && path.is_dir() && !is_locked {
        return Ok(());
    }
    let locked_by_machine =
        if is_locked { lock_owner(path).filter(|o| !o.is_on_this_machine()).map(|o| o.machine) } else { None };
    Err(ProfileLoadError { name: name.to_string(), folder: folder.to_string(), is_locked, locked_by_machine })
}

/// `SelectProfile.PopulateProfileList`: newest first with never-loaded profiles last, and each lock checked, which also clears
/// stale ones. The original's sort has no tie rule, so a stable one fits.
pub fn list_profiles_for_selection(settings: &ProfileSettings) -> Vec<ProfileEntry> {
    let count = settings.get_i32(Profiles, "ProfileCount", 0);
    let mut rows: Vec<ProfileEntry> = (0..count)
        .map(|i| {
            let data_folder = settings.get_string(Profiles, &format!("Profile{i}/DataFolder"), "");
            ProfileEntry {
                name: settings.get_string(Profiles, &format!("Profile{i}/Name"), NEW_PROFILE_NAME),
                date_last_loaded: settings.get_date(Profiles, &format!("Profile{i}/DateLastLoaded")),
                number: i,
                is_locked: is_profile_locked(Path::new(&data_folder)),
                data_folder,
            }
        })
        .collect();
    rows.sort_by(|a, b| b.date_last_loaded.cmp(&a.date_last_loaded));
    rows
}

/// The Profile Archive Wizard's drop-down, as (name, folder): `Default` at the default folder if no profiles are saved, otherwise each one,
/// with empty text for what's missing.
pub fn archive_profile_choices(settings: &ProfileSettings) -> Vec<(String, String)> {
    let count = settings.get_i32(Profiles, "ProfileCount", 0);
    if count == 0 {
        return vec![("Default".to_string(), default_folder())];
    }
    (0..count)
        .map(|i| {
            (
                settings.get_string(Profiles, &format!("Profile{i}/Name"), ""),
                settings.get_string(Profiles, &format!("Profile{i}/DataFolder"), ""),
            )
        })
        .collect()
}

/// What Select a Profile's Load writes.
pub fn mark_profile_loaded(settings: &ProfileSettings, number: i32, now: LocalDateTime) -> Result<(), IoError> {
    settings.put_date(Profiles, &format!("Profile{number}/DateLastLoaded"), now)
}

/// `DefaultFolder`: `~/Documents/Vixen 3`.
fn default_folder() -> String {
    DataPaths::default_root()
        .unwrap_or_else(|| PathBuf::from(VIXEN_DATA_DIR))
        .to_string_lossy()
        .into_owned()
}

/// A new profile's folder: `~/Documents/Vixen 3 {name}`.
pub fn default_new_profile_folder(name: &str) -> PathBuf {
    PathBuf::from(format!("{} {name}", default_folder()))
}

/// Why a new profile's name was refused. The prompt stays up with the text kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddProfileError {
    Blank,
    Duplicate(String),
}

impl AddProfileError {
    pub fn message(&self) -> String {
        match self {
            AddProfileError::Blank => "Profile name can not be blank.".to_string(),
            AddProfileError::Duplicate(name) => format!("A profile with the name {name} already exists."),
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            AddProfileError::Blank => "Error",
            AddProfileError::Duplicate(_) => "",
        }
    }
}

/// What OK's first pass found. The strings are `{name}:\t {folder}`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileIssues {
    /// `Warning - Blank Entries` shows once for each, and OK carries on whatever it says.
    pub blank_count: usize,
    pub duplicates: Vec<String>,
    pub invalid_paths: Vec<String>,
}

impl ProfileIssues {
    pub const BLANK_MESSAGE: &'static str =
        "One or more of your profile entries has a blank name or data folder. You must correct this before continuing.";

    pub fn duplicates_message(&self) -> Option<String> {
        (!self.duplicates.is_empty()).then(|| {
            format!(
                "A duplicate profile name, or data path exists.\n\nThe duplicate items found were:\n\n{}\n\nClick OK to accept and continue, or Cancel to go back and edit.",
                self.duplicates.join("\n")
            )
        })
    }

    pub fn invalid_message(&self) -> Option<String> {
        (!self.invalid_paths.is_empty()).then(|| {
            format!(
                "An invalid profile data folder exists.\n\nThe items with invalid paths were:\n\n{}\n\nClick OK to accept and continue, or Cancel to go back and edit.",
                self.invalid_paths.join("\n")
            )
        })
    }
}

/// `The data directory ... does not exist`, for the prompt at OK.
pub fn create_folder_prompt(name: &str, folder: &str) -> String {
    format!("The data directory '{folder}' for profile '{name}' does not exist.  Would you like to create it?")
}

/// `Could not create new profile directory`, when that failed.
pub fn create_folder_failed_message(folder: &str) -> String {
    format!("Could not create new profile directory: {folder}\n\nClick OK to ignore and continue, or Cancel to go back and edit.")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EditorItem {
    /// Identity across edits, standing in for the original's comparison of `ProfileItem` objects.
    uid: u64,
    name: String,
    data_folder: String,
}

/// The Data Profile dialog's state. Items are edited in memory until `save`.
#[derive(Clone, Debug)]
pub struct ProfileEditor {
    items: Vec<EditorItem>,
    selected: Option<usize>,
    next_uid: u64,
    ask: bool,
    /// What the file says, because adding or deleting a profile puts the radios back to it.
    saved_ask: bool,
    load_selection: Option<u64>,
}

impl ProfileEditor {
    /// `DataProfileForm_Load`.
    pub fn open(settings: &ProfileSettings) -> ProfileEditor {
        let count = settings.get_i32(Profiles, "ProfileCount", 0);
        let default = default_folder();
        let items: Vec<EditorItem> = if count == 0 {
            vec![EditorItem { uid: 0, name: "Default".to_string(), data_folder: default }]
        } else {
            (0..count)
                .map(|i| EditorItem {
                    uid: i as u64,
                    name: settings.get_string(Profiles, &format!("Profile{i}/Name"), NEW_PROFILE_NAME),
                    data_folder: settings.get_string(Profiles, &format!("Profile{i}/DataFolder"), &default),
                })
                .collect()
        };
        let ask = settings.get_string(Profiles, "LoadAction", LOAD_SELECTED) == ASK;
        // Only the second radio restores the drop-down, so with Ask nothing is selected and OK leaves `ProfileToLoad` alone.
        let load_selection = if ask {
            None
        } else {
            let index = settings.get_i32(Profiles, "ProfileToLoad", 0);
            usize::try_from(index).ok().and_then(|i| items.get(i)).map(|item| item.uid)
        };
        let selected = (!items.is_empty()).then_some(0);
        let next_uid = items.len() as u64;
        ProfileEditor { items, selected, next_uid, ask, saved_ask: ask, load_selection }
    }

    pub fn items(&self) -> Vec<ProfileEntry> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| ProfileEntry {
                name: item.name.clone(),
                data_folder: item.data_folder.clone(),
                date_last_loaded: None,
                number: i as i32,
                is_locked: false,
            })
            .collect()
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Shows another profile. Commit the fields first, with `set_name` and `set_data_folder`.
    pub fn select(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected = Some(index);
        }
    }

    /// The selected item's name, as typed. Not trimmed.
    pub fn set_name(&mut self, text: &str) {
        if let Some(item) = self.selected.and_then(|i| self.items.get_mut(i)) {
            item.name = text.to_string();
        }
    }

    /// The selected item's folder, trimmed as `SaveCurrentItem` does.
    pub fn set_data_folder(&mut self, text: &str) {
        if let Some(item) = self.selected.and_then(|i| self.items.get_mut(i)) {
            item.data_folder = text.trim().to_string();
        }
    }

    /// The + button, after its prompt.
    pub fn add(&mut self, name: &str) -> Result<(), AddProfileError> {
        if name.is_empty() {
            return Err(AddProfileError::Blank);
        }
        if self.items.iter().any(|i| i.name == name) {
            return Err(AddProfileError::Duplicate(name.to_string()));
        }
        let data_folder = default_new_profile_folder(name).to_string_lossy().into_owned();
        self.items.push(EditorItem { uid: self.next_uid, name: name.to_string(), data_folder });
        self.next_uid += 1;
        self.selected = Some(self.items.len() - 1);
        self.after_list_change();
        Ok(())
    }

    /// The − button. Nothing on disk changes. With the last one gone, nothing is selected, so the fields clear and disable where
    /// the original leaves them showing a removed profile.
    pub fn delete(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        self.items.remove(index);
        self.selected = (!self.items.is_empty()).then_some(0);
        self.after_list_change();
    }

    /// `PopulateLoadProfileSection(false)`. Deliberate quirk: it reads the radios from the file again, so adding or deleting a profile
    /// undoes an unsaved change of radio. The drop-down keeps its item if that survives.
    fn after_list_change(&mut self) {
        self.ask = self.saved_ask;
        if let Some(uid) = self.load_selection {
            if !self.items.iter().any(|i| i.uid == uid) {
                self.load_selection = None;
            }
        }
    }

    /// True for the `Ask Me Every Time` radio.
    pub fn ask_every_time(&self) -> bool {
        self.ask
    }

    pub fn set_ask_every_time(&mut self, ask: bool) {
        self.ask = ask;
    }

    /// The index chosen in the `Load This Profile:` drop-down.
    pub fn load_selection(&self) -> Option<usize> {
        self.load_selection.and_then(|uid| self.items.iter().position(|i| i.uid == uid))
    }

    /// Choosing an item doesn't change the radio, as upstream.
    pub fn set_load_selection(&mut self, index: Option<usize>) {
        self.load_selection = index.and_then(|i| self.items.get(i)).map(|item| item.uid);
    }

    /// OK's first pass. On the Mac a folder is invalid unless it starts with `/`, so a Windows path shows as invalid.
    pub fn validate(&self) -> ProfileIssues {
        let mut issues = ProfileIssues::default();
        let mut names: Vec<&str> = Vec::new();
        let mut folders: Vec<&str> = Vec::new();
        for item in &self.items {
            let line = format!("{}:\t {}", item.name, item.data_folder);
            if item.name.is_empty() || item.data_folder.is_empty() {
                issues.blank_count += 1;
            }
            if names.contains(&item.name.as_str()) {
                issues.duplicates.push(line.clone());
            }
            names.push(&item.name);
            if folders.contains(&item.data_folder.as_str()) {
                issues.duplicates.push(line.clone());
            }
            folders.push(&item.data_folder);
            if !item.data_folder.starts_with('/') {
                issues.invalid_paths.push(line);
            }
        }
        issues
    }

    /// The folders OK must ask about, as (name, folder).
    pub fn folders_to_create(&self) -> Vec<(String, String)> {
        self.items
            .iter()
            .filter(|i| !Path::new(&i.data_folder).is_dir())
            .map(|i| (i.name.clone(), i.data_folder.clone()))
            .collect()
    }

    /// Writes everything at once. The original writes as it goes, so backing out at a folder prompt left a partial save; here the
    /// prompts all come first and nothing is written until this.
    pub fn save(&self, settings: &ProfileSettings) -> Result<(), IoError> {
        let mut values: Vec<(String, String)> = vec![("ProfileCount".to_string(), self.items.len().to_string())];
        for (i, item) in self.items.iter().enumerate() {
            values.push((format!("Profile{i}/Name"), item.name.clone()));
            values.push((format!("Profile{i}/DataFolder"), item.data_folder.clone()));
        }
        values.push(("LoadAction".to_string(), if self.ask { ASK } else { LOAD_SELECTED }.to_string()));
        if let Some(index) = self.load_selection() {
            values.push(("ProfileToLoad".to_string(), index.to_string()));
        }
        let borrowed: Vec<(&str, String)> = values.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        settings.put_many(Profiles, &borrowed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::{machine_name, LOCK_FILE};

    struct Fixture {
        dir: PathBuf,
        settings: ProfileSettings,
    }

    fn fixture(name: &str) -> Fixture {
        let dir = crate::test_dir(name);
        let settings = ProfileSettings::open(dir.join("Settings.xml"));
        Fixture { dir, settings }
    }

    impl Fixture {
        fn profile(&self, i: i32, name: &str, folder: &str) {
            self.settings.put_string(Profiles, &format!("Profile{i}/Name"), name).unwrap();
            self.settings.put_string(Profiles, &format!("Profile{i}/DataFolder"), folder).unwrap();
        }

        fn folder(&self, name: &str) -> String {
            let path = self.dir.join(name);
            std::fs::create_dir_all(&path).unwrap();
            path.to_string_lossy().into_owned()
        }
    }

    #[test]
    fn no_settings_or_a_count_of_zero_is_no_profiles() {
        let f = fixture("profiles-none");
        assert_eq!(decide_startup_profile(&f.settings), StartupProfile::NoProfiles);
        f.settings.put_i32(Profiles, "ProfileCount", 0).unwrap();
        assert_eq!(decide_startup_profile(&f.settings), StartupProfile::NoProfiles);
    }

    #[test]
    fn ask_asks_with_no_error() {
        let f = fixture("profiles-ask");
        let folder = f.folder("A");
        f.profile(0, "A", &folder);
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        f.settings.put_i32(Profiles, "ProfileToLoad", 0).unwrap();
        f.settings.put_string(Profiles, "LoadAction", "Ask").unwrap();
        assert_eq!(decide_startup_profile(&f.settings), StartupProfile::Ask { error: None });
    }

    #[test]
    fn an_index_out_of_range_asks_with_no_error() {
        let f = fixture("profiles-range");
        let folder = f.folder("A");
        f.profile(0, "A", &folder);
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        // No ProfileToLoad means -1.
        assert_eq!(decide_startup_profile(&f.settings), StartupProfile::Ask { error: None });
        f.settings.put_i32(Profiles, "ProfileToLoad", 1).unwrap();
        assert_eq!(decide_startup_profile(&f.settings), StartupProfile::Ask { error: None });
    }

    #[test]
    fn the_selected_folder_loads_when_it_exists() {
        let f = fixture("profiles-load");
        let folder = f.folder("Show");
        f.profile(0, "Show", &folder);
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        f.settings.put_i32(Profiles, "ProfileToLoad", 0).unwrap();
        assert_eq!(decide_startup_profile(&f.settings), StartupProfile::Load { folder, name: "Show".to_string() });
    }

    #[test]
    fn a_missing_folder_asks_with_the_exact_error() {
        let f = fixture("profiles-missing");
        f.profile(0, "X", &f.dir.join("gone").to_string_lossy());
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        f.settings.put_i32(Profiles, "ProfileToLoad", 0).unwrap();
        let StartupProfile::Ask { error: Some(error) } = decide_startup_profile(&f.settings) else {
            panic!("should ask");
        };
        assert_eq!(
            error.message(),
            "Selected profile X data directory does not exist!\n\nSelect a different profile to load or use the Profile Editor to create a new profile."
        );
        assert!(!error.is_locked);
        assert_eq!(error.locked_by_machine, None);
    }

    #[test]
    fn a_locked_folder_asks_with_the_exact_error_and_names_the_other_machine() {
        let f = fixture("profiles-locked");
        let folder = f.folder("X");
        std::fs::write(Path::new(&folder).join(LOCK_FILE), "12@WINDOWS-PC").unwrap();
        f.profile(0, "X", &folder);
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        f.settings.put_i32(Profiles, "ProfileToLoad", 0).unwrap();
        let StartupProfile::Ask { error: Some(error) } = decide_startup_profile(&f.settings) else {
            panic!("should ask");
        };
        assert_eq!(
            error.message(),
            "Selected profile X is locked by another instance!\n\nSelect a different profile to load or use the Profile Editor to create a new profile."
        );
        assert_eq!(error.locked_by_machine.as_deref(), Some("WINDOWS-PC"));
    }

    #[test]
    fn a_blank_folder_fails_the_check() {
        assert!(check_profile("", "X").is_err());
    }

    #[test]
    fn selection_list_is_newest_first_with_locks_flagged_and_bad_dates_last() {
        let f = fixture("profiles-list");
        let locked = f.folder("Locked");
        std::fs::write(Path::new(&locked).join(LOCK_FILE), "1@ELSEWHERE").unwrap();
        let free = f.folder("Free");
        f.profile(0, "Old", &free);
        f.settings.put_string(Profiles, "Profile0/DateLastLoaded", "1/1/2024 1:00:00 AM").unwrap();
        f.profile(1, "New", &locked);
        f.settings.put_string(Profiles, "Profile1/DateLastLoaded", "12/20/2025 5:43:19 PM").unwrap();
        f.profile(2, "BadDate", &free);
        f.settings.put_string(Profiles, "Profile2/DateLastLoaded", "whenever").unwrap();
        f.settings.put_string(Profiles, "Profile3/DataFolder", &free).unwrap();
        f.settings.put_i32(Profiles, "ProfileCount", 4).unwrap();
        let rows = list_profiles_for_selection(&f.settings);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["New", "Old", "BadDate", "New Profile"]);
        let numbers: Vec<i32> = rows.iter().map(|r| r.number).collect();
        assert_eq!(numbers, [1, 0, 2, 3]);
        assert!(rows[0].is_locked);
        assert!(!rows[1].is_locked);
    }

    #[test]
    fn listing_clears_a_stale_lock() {
        let f = fixture("profiles-stale");
        let folder = f.folder("Crashed");
        let mut child = std::process::Command::new("/usr/bin/true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        std::fs::write(Path::new(&folder).join(LOCK_FILE), format!("{pid}@{}", machine_name())).unwrap();
        f.profile(0, "Crashed", &folder);
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        assert!(!list_profiles_for_selection(&f.settings)[0].is_locked);
        assert!(!Path::new(&folder).join(LOCK_FILE).exists());
    }

    #[test]
    fn the_archive_wizard_offers_the_default_or_the_saved_profiles() {
        let f = fixture("profiles-archive");
        assert_eq!(archive_profile_choices(&f.settings), [("Default".to_string(), default_folder())]);
        f.profile(0, "A", "/a");
        f.settings.put_i32(Profiles, "ProfileCount", 2).unwrap();
        assert_eq!(
            archive_profile_choices(&f.settings),
            [("A".to_string(), "/a".to_string()), (String::new(), String::new())]
        );
    }

    #[test]
    fn loading_records_the_date_under_the_original_number() {
        let f = fixture("profiles-mark");
        let date = LocalDateTime::parse("12/20/2025 5:43:19 PM").unwrap();
        mark_profile_loaded(&f.settings, 3, date).unwrap();
        assert_eq!(f.settings.get_string(Profiles, "Profile3/DateLastLoaded", ""), "12/20/2025 5:43:19 PM");
    }

    #[test]
    fn a_new_editor_with_no_settings_holds_the_default_profile() {
        let f = fixture("profiles-editor-new");
        let editor = ProfileEditor::open(&f.settings);
        let items = editor.items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Default");
        assert_eq!(items[0].data_folder, default_folder());
        assert_eq!(editor.selected(), Some(0));
        assert!(!editor.ask_every_time());
        assert_eq!(editor.load_selection(), Some(0));
    }

    #[test]
    fn the_editor_fills_missing_members_with_its_defaults() {
        let f = fixture("profiles-editor-defaults");
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        let items = ProfileEditor::open(&f.settings).items();
        assert_eq!(items[0].name, "New Profile");
        assert_eq!(items[0].data_folder, default_folder());
    }

    #[test]
    fn adding_uses_the_default_folder_and_refuses_bad_names() {
        let f = fixture("profiles-editor-add");
        let mut editor = ProfileEditor::open(&f.settings);
        editor.add("Halloween").unwrap();
        assert_eq!(editor.selected(), Some(1));
        assert_eq!(editor.items()[1].data_folder, format!("{} Halloween", default_folder()));
        assert_eq!(editor.add(""), Err(AddProfileError::Blank));
        assert_eq!(AddProfileError::Blank.message(), "Profile name can not be blank.");
        assert_eq!(editor.add("Default"), Err(AddProfileError::Duplicate("Default".to_string())));
        assert_eq!(
            AddProfileError::Duplicate("Default".to_string()).message(),
            "A profile with the name Default already exists."
        );
        assert_eq!(editor.items().len(), 2);
    }

    #[test]
    fn fields_commit_to_the_selected_item_and_only_the_folder_trims() {
        let f = fixture("profiles-editor-fields");
        let mut editor = ProfileEditor::open(&f.settings);
        editor.set_name("  Spaced  ");
        editor.set_data_folder("  /a/b  ");
        assert_eq!(editor.items()[0].name, "  Spaced  ");
        assert_eq!(editor.items()[0].data_folder, "/a/b");
    }

    #[test]
    fn deleting_selects_the_first_and_the_last_one_leaves_nothing_selected() {
        let f = fixture("profiles-editor-delete");
        let mut editor = ProfileEditor::open(&f.settings);
        editor.add("Second").unwrap();
        editor.delete(1);
        assert_eq!(editor.selected(), Some(0));
        editor.delete(0);
        assert!(editor.items().is_empty());
        assert_eq!(editor.selected(), None);
        editor.set_name("ignored");
        editor.set_data_folder("ignored");
        assert_eq!(editor.load_selection(), None);
    }

    #[test]
    fn the_load_selection_survives_a_refill_when_its_item_does() {
        let f = fixture("profiles-editor-load");
        let mut editor = ProfileEditor::open(&f.settings);
        editor.add("B").unwrap();
        editor.add("C").unwrap();
        editor.set_load_selection(Some(2));
        editor.delete(1);
        assert_eq!(editor.load_selection(), Some(1), "C moved up and stays chosen");
        editor.delete(1);
        assert_eq!(editor.load_selection(), None, "C is gone");
    }

    #[test]
    fn changing_the_list_puts_the_radios_back_to_what_the_file_says() {
        let f = fixture("profiles-editor-radio");
        let mut editor = ProfileEditor::open(&f.settings);
        editor.set_ask_every_time(true);
        assert!(editor.ask_every_time());
        editor.add("B").unwrap();
        assert!(!editor.ask_every_time(), "the original reads the file again here");
    }

    #[test]
    fn deleting_puts_the_radios_back_too_and_saving_alone_keeps_the_change() {
        let f = fixture("profiles-editor-radio-delete");
        let mut editor = ProfileEditor::open(&f.settings);
        editor.add("B").unwrap();
        editor.set_ask_every_time(true);
        editor.delete(1);
        assert!(!editor.ask_every_time());
        editor.set_ask_every_time(true);
        editor.save(&f.settings).unwrap();
        assert_eq!(f.settings.get_string(Profiles, "LoadAction", ""), "Ask");
    }

    #[test]
    fn a_name_of_spaces_is_accepted_since_blank_means_exactly_empty() {
        let f = fixture("profiles-editor-spaces");
        let mut editor = ProfileEditor::open(&f.settings);
        assert_eq!(editor.add("  "), Ok(()));
        assert_eq!(editor.items()[1].name, "  ");
    }

    #[test]
    fn losing_the_load_item_leaves_profile_to_load_as_it_was() {
        let f = fixture("profiles-editor-lose");
        f.profile(0, "A", "/a");
        f.profile(1, "B", "/b");
        f.settings.put_i32(Profiles, "ProfileCount", 2).unwrap();
        f.settings.put_i32(Profiles, "ProfileToLoad", 1).unwrap();
        let mut editor = ProfileEditor::open(&f.settings);
        assert_eq!(editor.load_selection(), Some(1));
        editor.add("C").unwrap();
        assert_eq!(editor.load_selection(), Some(1), "kept across an add");
        editor.delete(1);
        assert_eq!(editor.load_selection(), None);
        editor.save(&f.settings).unwrap();
        assert_eq!(f.settings.get_i32(Profiles, "ProfileToLoad", -1), 1);
    }

    #[test]
    fn opening_with_ask_selects_nothing_in_the_drop_down() {
        let f = fixture("profiles-editor-ask");
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        f.settings.put_string(Profiles, "LoadAction", "Ask").unwrap();
        f.settings.put_i32(Profiles, "ProfileToLoad", 0).unwrap();
        let editor = ProfileEditor::open(&f.settings);
        assert!(editor.ask_every_time());
        assert_eq!(editor.load_selection(), None);
    }

    #[test]
    fn validation_collects_duplicates_invalid_paths_and_blanks() {
        let f = fixture("profiles-validate");
        f.profile(0, "Same", "/a");
        f.profile(1, "Same", "/b");
        f.profile(2, "Other", "/a");
        f.profile(3, "Win", "C:\\Vixen");
        f.profile(4, "", "/e");
        f.settings.put_i32(Profiles, "ProfileCount", 5).unwrap();
        let issues = ProfileEditor::open(&f.settings).validate();
        assert_eq!(issues.blank_count, 1);
        assert_eq!(issues.duplicates, ["Same:\t /b", "Other:\t /a"]);
        assert_eq!(issues.invalid_paths, ["Win:\t C:\\Vixen"]);
        assert_eq!(
            issues.duplicates_message().unwrap(),
            "A duplicate profile name, or data path exists.\n\nThe duplicate items found were:\n\nSame:\t /b\nOther:\t /a\n\nClick OK to accept and continue, or Cancel to go back and edit."
        );
        assert_eq!(
            issues.invalid_message().unwrap(),
            "An invalid profile data folder exists.\n\nThe items with invalid paths were:\n\nWin:\t C:\\Vixen\n\nClick OK to accept and continue, or Cancel to go back and edit."
        );
    }

    #[test]
    fn an_item_duplicated_on_both_counts_lists_twice() {
        let f = fixture("profiles-validate-both");
        f.profile(0, "Same", "/a");
        f.profile(1, "Same", "/a");
        f.settings.put_i32(Profiles, "ProfileCount", 2).unwrap();
        let issues = ProfileEditor::open(&f.settings).validate();
        assert_eq!(issues.duplicates, ["Same:\t /a", "Same:\t /a"]);
    }

    #[test]
    fn a_clean_list_has_no_issues_or_messages() {
        let f = fixture("profiles-validate-clean");
        f.profile(0, "A", "/a");
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        let issues = ProfileEditor::open(&f.settings).validate();
        assert_eq!(issues, ProfileIssues::default());
        assert_eq!(issues.duplicates_message(), None);
        assert_eq!(issues.invalid_message(), None);
    }

    #[test]
    fn folders_to_create_are_the_missing_ones() {
        let f = fixture("profiles-folders");
        let here = f.folder("Here");
        let gone = f.dir.join("Gone").to_string_lossy().into_owned();
        f.profile(0, "Here", &here);
        f.profile(1, "Gone", &gone);
        f.settings.put_i32(Profiles, "ProfileCount", 2).unwrap();
        let editor = ProfileEditor::open(&f.settings);
        assert_eq!(editor.folders_to_create(), [("Gone".to_string(), gone.clone())]);
        assert_eq!(
            create_folder_prompt("Gone", &gone),
            format!("The data directory '{gone}' for profile 'Gone' does not exist.  Would you like to create it?")
        );
    }

    #[test]
    fn saving_writes_every_key() {
        let f = fixture("profiles-save");
        let mut editor = ProfileEditor::open(&f.settings);
        editor.set_data_folder("/first");
        editor.add("Second").unwrap();
        editor.set_data_folder("/second");
        editor.set_load_selection(Some(1));
        editor.save(&f.settings).unwrap();
        let back = ProfileSettings::open(f.settings.path());
        assert_eq!(back.get_i32(Profiles, "ProfileCount", 0), 2);
        assert_eq!(back.get_string(Profiles, "Profile0/Name", ""), "Default");
        assert_eq!(back.get_string(Profiles, "Profile0/DataFolder", ""), "/first");
        assert_eq!(back.get_string(Profiles, "Profile1/Name", ""), "Second");
        assert_eq!(back.get_string(Profiles, "Profile1/DataFolder", ""), "/second");
        assert_eq!(back.get_string(Profiles, "LoadAction", ""), "LoadSelected");
        assert_eq!(back.get_i32(Profiles, "ProfileToLoad", -1), 1);
    }

    #[test]
    fn without_a_selection_profile_to_load_stays_as_it_was() {
        let f = fixture("profiles-save-keep");
        f.settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        f.settings.put_string(Profiles, "LoadAction", "Ask").unwrap();
        f.settings.put_i32(Profiles, "ProfileToLoad", 7).unwrap();
        let editor = ProfileEditor::open(&f.settings);
        editor.save(&f.settings).unwrap();
        assert_eq!(f.settings.get_i32(Profiles, "ProfileToLoad", -1), 7);
        assert_eq!(f.settings.get_string(Profiles, "LoadAction", ""), "Ask");
    }

    #[test]
    fn a_blank_name_still_saves_and_higher_numbers_stay_in_the_file() {
        let f = fixture("profiles-save-blank");
        f.profile(0, "A", "/a");
        f.profile(1, "B", "/b");
        f.settings.put_i32(Profiles, "ProfileCount", 2).unwrap();
        let mut editor = ProfileEditor::open(&f.settings);
        editor.delete(1);
        editor.set_name("");
        assert_eq!(editor.validate().blank_count, 1);
        editor.save(&f.settings).unwrap();
        assert_eq!(f.settings.get_i32(Profiles, "ProfileCount", 0), 1);
        assert_eq!(f.settings.get_string(Profiles, "Profile0/Name", "x"), "");
        assert_eq!(f.settings.get_string(Profiles, "Profile1/Name", ""), "B", "the original leaves it, unused");
    }

    #[test]
    fn nothing_is_written_until_save() {
        let f = fixture("profiles-no-write");
        let mut editor = ProfileEditor::open(&f.settings);
        editor.add("New").unwrap();
        editor.delete(0);
        assert!(!f.settings.path().exists());
    }
}
