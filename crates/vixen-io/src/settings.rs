//! `Settings.xml`: the profile list and other windows' remembered settings. Port of `XMLProfileSettings`.
//!
//! The original parses the file for every instance and saves it for every put, so two instances overwrite each other.
//! Here one store sits behind a lock, each write is atomic, and [`ProfileSettings::put_many`] saves once for many keys.

use crate::dcs::format_double;
use crate::datetime::LocalDateTime;
use crate::error::IoError;
use crate::file::write_atomic;
use crate::xml::{write_document, DocStyle};
use parking_lot::Mutex;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use vixen_xml::{XElement, XNode};

const ROOT: &str = "settings";
const FILE_NAME: &str = "Settings.xml";

/// `XMLProfileSettings.SettingType`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SettingSection {
    Profiles,
    AppSettings,
    Preferences,
}

impl SettingSection {
    fn name(self) -> &'static str {
        match self {
            SettingSection::Profiles => "Profiles",
            SettingSection::AppSettings => "AppSettings",
            SettingSection::Preferences => "Preferences",
        }
    }
}

/// Where the original keeps `%AppData%/Vixen`: `~/Library/Application Support/Vixen`.
/// A UI test with an output folder takes precedence; otherwise `VIXEN_SETTINGS_DIR`
/// selects a separate settings/layout store, including for non-harness diagnostics.
/// Callers must validate an absolute private directory before setting the override.
/// An explicitly supplied empty or relative value is honored, never silently replaced
/// with the owner's settings directory.
pub fn settings_dir() -> PathBuf {
    settings_dir_from_env(|name| std::env::var_os(name), &std::env::temp_dir())
}

fn settings_dir_from_env(mut env: impl FnMut(&str) -> Option<OsString>, fallback_home: &Path) -> PathBuf {
    if env("VIXEN_UI_TEST").is_some() {
        if let Some(out) = env("VIXEN_UI_TEST_OUT") {
            return PathBuf::from(out);
        }
    }
    if let Some(directory) = env("VIXEN_SETTINGS_DIR") {
        return PathBuf::from(directory);
    }
    let home = env("HOME").map(PathBuf::from).unwrap_or_else(|| fallback_home.to_path_buf());
    home.join("Library").join("Application Support").join("Vixen")
}

/// One `Settings.xml`.
pub struct ProfileSettings {
    path: PathBuf,
    root: Mutex<Loaded>,
}

/// The file's contents, and when the file was last seen changing, so a second copy of the app's changes aren't overwritten.
struct Loaded {
    root: XElement,
    modified: Option<SystemTime>,
}

fn read_settings(path: &Path) -> Loaded {
    let root = XElement::from_file(path).ok().filter(|r| r.local == ROOT).unwrap_or_else(|| XElement::new("", ROOT));
    Loaded { root, modified: modified_time(path) }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl ProfileSettings {
    /// A missing or unreadable file starts from an empty `<settings>`, as the original's constructor does.
    pub fn open(path: impl Into<PathBuf>) -> ProfileSettings {
        let path = path.into();
        let root = read_settings(&path);
        ProfileSettings { path, root: Mutex::new(root) }
    }

    /// The store everything in this process shares, at [`settings_dir`].
    pub fn shared() -> Arc<ProfileSettings> {
        static SHARED: OnceLock<Arc<ProfileSettings>> = OnceLock::new();
        SHARED.get_or_init(|| Arc::new(ProfileSettings::open(settings_dir().join(FILE_NAME)))).clone()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get_string(&self, section: SettingSection, xpath: &str, default: &str) -> String {
        self.text(section, xpath).unwrap_or_else(|| default.to_string())
    }

    /// Text that isn't a number gives the default, where `Convert.ToInt32` would throw.
    pub fn get_i32(&self, section: SettingSection, xpath: &str, default: i32) -> i32 {
        self.text(section, xpath).and_then(|t| t.trim().parse().ok()).unwrap_or(default)
    }

    pub fn get_f64(&self, section: SettingSection, xpath: &str, default: f64) -> f64 {
        self.text(section, xpath).and_then(|t| t.trim().parse().ok()).unwrap_or(default)
    }

    pub fn get_f32(&self, section: SettingSection, xpath: &str, default: f32) -> f32 {
        self.get_f64(section, xpath, default as f64) as f32
    }

    pub fn get_bool(&self, section: SettingSection, xpath: &str, default: bool) -> bool {
        match self.text(section, xpath).map(|t| t.trim().to_ascii_lowercase()).as_deref() {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        }
    }

    /// Text that isn't a date gives `None`, where `Convert.ToDateTime` would throw.
    pub fn get_date(&self, section: SettingSection, xpath: &str) -> Option<LocalDateTime> {
        self.text(section, xpath).and_then(|t| LocalDateTime::parse(&t))
    }

    pub fn put_string(&self, section: SettingSection, xpath: &str, value: &str) -> Result<(), IoError> {
        self.put_many(section, &[(xpath, value.to_string())])
    }

    pub fn put_i32(&self, section: SettingSection, xpath: &str, value: i32) -> Result<(), IoError> {
        self.put_string(section, xpath, &value.to_string())
    }

    pub fn put_f64(&self, section: SettingSection, xpath: &str, value: f64) -> Result<(), IoError> {
        self.put_string(section, xpath, &format_double(value))
    }

    /// `Convert.ToString(bool)`: `True` or `False`.
    pub fn put_bool(&self, section: SettingSection, xpath: &str, value: bool) -> Result<(), IoError> {
        self.put_string(section, xpath, if value { "True" } else { "False" })
    }

    pub fn put_date(&self, section: SettingSection, xpath: &str, value: LocalDateTime) -> Result<(), IoError> {
        self.put_string(section, xpath, &value.settings_text())
    }

    /// Writes several keys with one save.
    pub fn put_many(&self, section: SettingSection, values: &[(&str, String)]) -> Result<(), IoError> {
        for (xpath, _) in values {
            if let Some(bad) = xpath.split('/').find(|name| !is_element_name(name)) {
                return Err(IoError::format(FILE_NAME, format!("\"{bad}\" can't be a setting's name")));
            }
        }
        let mut loaded = self.current();
        for (xpath, value) in values {
            let path = std::iter::once(section.name()).chain(xpath.split('/'));
            let node = find_or_create(&mut loaded.root, path);
            node.children.clear();
            node.children.push(XNode::Text(value.clone()));
        }
        self.save(&mut loaded)
    }

    /// The store, brought up to date first if another copy of the app has written the file.
    fn current(&self) -> parking_lot::MutexGuard<'_, Loaded> {
        let mut loaded = self.root.lock();
        if modified_time(&self.path) != loaded.modified {
            *loaded = read_settings(&self.path);
        }
        loaded
    }

    /// `DeleteNode`. Saves only if the key was there.
    pub fn delete_node(&self, section: SettingSection, xpath: &str) -> Result<(), IoError> {
        self.remove_node(&format!("{}/{xpath}", section.name()))
    }

    /// `RenameNode`: the node keeps its children and loses its attributes. Saves only if it was there.
    pub fn rename_node(&self, section: SettingSection, xpath: &str, new_name: &str) -> Result<(), IoError> {
        if !is_element_name(new_name) {
            return Err(IoError::format(FILE_NAME, format!("\"{new_name}\" can't be a setting's name")));
        }
        let mut loaded = self.current();
        let path: Vec<&str> = std::iter::once(section.name()).chain(xpath.split('/')).collect();
        let Some(node) = find_mut(&mut loaded.root, &path) else {
            return Ok(());
        };
        node.local = new_name.to_string();
        node.attrs.clear();
        self.save(&mut loaded)
    }

    /// `RemoveNode`, with a path below the root that names its own section. Saves only if the node was there.
    pub fn remove_node(&self, xpath: &str) -> Result<(), IoError> {
        let mut loaded = self.current();
        let path: Vec<&str> = xpath.split('/').collect();
        let Some((last, parents)) = path.split_last() else {
            return Ok(());
        };
        let Some(parent) = find_mut(&mut loaded.root, parents) else {
            return Ok(());
        };
        let before = parent.children.len();
        let mut removed_one = false;
        parent.children.retain(|n| match n {
            XNode::Element(e) if !removed_one && e.local == *last => {
                removed_one = true;
                false
            }
            _ => true,
        });
        if parent.children.len() == before {
            return Ok(());
        }
        self.save(&mut loaded)
    }

    /// Reads see another copy's saves too, as upstream's do, since each of its instances reads the file afresh. Otherwise the Data Profile dialog
    /// would open on stale profiles and save them over the other copy's.
    fn text(&self, section: SettingSection, xpath: &str) -> Option<String> {
        let loaded = self.current();
        let mut node: &XElement = loaded.root.child(section.name())?;
        for part in xpath.split('/') {
            node = node.child(part)?;
        }
        Some(inner_text(node))
    }

    fn save(&self, loaded: &mut Loaded) -> Result<(), IoError> {
        write_atomic(&self.path, write_document(&loaded.root, DocStyle::default()).as_bytes())?;
        loaded.modified = modified_time(&self.path);
        Ok(())
    }
}

/// `XmlNode.InnerText`.
fn inner_text(node: &XElement) -> String {
    let mut text = String::new();
    for child in &node.children {
        match child {
            XNode::Text(t) => text.push_str(t),
            XNode::Element(e) => text.push_str(&inner_text(e)),
        }
    }
    text
}

fn find_mut<'a>(root: &'a mut XElement, path: &[&str]) -> Option<&'a mut XElement> {
    let mut node = root;
    for part in path {
        node = node.child_mut(part)?;
    }
    Some(node)
}

/// `CreateMissingNode`.
fn find_or_create<'a, 'p>(root: &'a mut XElement, path: impl Iterator<Item = &'p str>) -> &'a mut XElement {
    let mut node = root;
    for part in path {
        if node.child(part).is_none() {
            node.push(XElement::new("", part));
        }
        node = node.child_mut(part).expect("the child was just added");
    }
    node
}

/// The original builds elements from these names and would throw on a bad one. A bad one here must not corrupt the file.
fn is_element_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_alphabetic() || first == '_') && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use SettingSection::*;

    fn temp_file(name: &str) -> PathBuf {
        crate::test_dir(name).join(FILE_NAME)
    }

    fn resolved(values: &[(&str, &str)]) -> PathBuf {
        settings_dir_from_env(|name| values.iter().find(|(key, _)| *key == name).map(|(_, value)| OsString::from(*value)), Path::new("/temporary-home"))
    }

    #[test]
    fn settings_directory_defaults_preserve_home_and_missing_home_behavior() {
        assert_eq!(resolved(&[("HOME", "/owner")]), PathBuf::from("/owner/Library/Application Support/Vixen"));
        assert_eq!(resolved(&[]), PathBuf::from("/temporary-home/Library/Application Support/Vixen"));
    }

    #[test]
    fn ui_test_output_retains_precedence_over_explicit_settings_directory() {
        assert_eq!(
            resolved(&[("HOME", "/owner"), ("VIXEN_UI_TEST", "steps.json"), ("VIXEN_UI_TEST_OUT", "/private/ui"), ("VIXEN_SETTINGS_DIR", "/private/preferences")]),
            PathBuf::from("/private/ui")
        );
    }

    #[test]
    fn explicit_settings_directory_is_available_without_the_ui_harness() {
        assert_eq!(resolved(&[("HOME", "/owner"), ("VIXEN_SETTINGS_DIR", "/private/preferences")]), PathBuf::from("/private/preferences"));
        // A stray output variable alone is not an active UI harness.
        assert_eq!(
            resolved(&[("VIXEN_UI_TEST_OUT", "/not-active"), ("VIXEN_SETTINGS_DIR", "/private/preferences")]),
            PathBuf::from("/private/preferences")
        );
        assert_eq!(
            resolved(&[("VIXEN_UI_TEST", "steps.json"), ("VIXEN_SETTINGS_DIR", "/private/preferences")]),
            PathBuf::from("/private/preferences")
        );
    }

    #[test]
    fn explicitly_supplied_paths_never_fall_back_to_the_owner() {
        for value in ["", "relative-preferences"] {
            assert_eq!(resolved(&[("HOME", "/owner"), ("VIXEN_SETTINGS_DIR", value)]), PathBuf::from(value));
        }
        assert_eq!(
            resolved(&[("HOME", "/owner"), ("VIXEN_UI_TEST", "steps.json"), ("VIXEN_UI_TEST_OUT", ""), ("VIXEN_SETTINGS_DIR", "/private/preferences")]),
            PathBuf::new()
        );
    }

    #[test]
    fn private_non_harness_settings_survive_reopen_without_touching_owner_fixture() {
        let root = crate::test_dir("settings-private-override");
        let owner = root.join("owner-home");
        let owner_file = owner.join("Library/Application Support/Vixen/Settings.xml");
        std::fs::create_dir_all(owner_file.parent().unwrap()).unwrap();
        std::fs::write(&owner_file, b"owner sentinel").unwrap();
        let private = root.join("private-settings");
        let selected = settings_dir_from_env(
            |name| match name {
                "HOME" => Some(owner.as_os_str().to_owned()),
                "VIXEN_SETTINGS_DIR" => Some(private.as_os_str().to_owned()),
                _ => None,
            },
            &root,
        );
        assert_eq!(selected, private);
        let settings = ProfileSettings::open(selected.join(FILE_NAME));
        settings.put_string(Profiles, "Profile0/Name", "Private diagnostic").unwrap();
        assert_eq!(ProfileSettings::open(private.join(FILE_NAME)).get_string(Profiles, "Profile0/Name", ""), "Private diagnostic");
        assert_eq!(std::fs::read(owner_file).unwrap(), b"owner sentinel");
    }

    #[test]
    fn nested_keys_create_their_path_and_survive_a_reopen() {
        let path = temp_file("settings-nested");
        let settings = ProfileSettings::open(&path);
        settings.put_string(Profiles, "Profile0/Name", "Default").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("<settings>"), "{text}");
        assert!(text.contains("<Profiles>"), "{text}");
        assert!(text.contains("<Profile0>"), "{text}");
        assert!(text.contains("<Name>Default</Name>"), "{text}");
        let back = ProfileSettings::open(&path);
        assert_eq!(back.get_string(Profiles, "Profile0/Name", "x"), "Default");
    }

    #[test]
    fn values_of_each_type_read_back() {
        let path = temp_file("settings-types");
        let settings = ProfileSettings::open(&path);
        settings.put_i32(AppSettings, "A/Int", -7).unwrap();
        settings.put_f64(AppSettings, "A/Double", 1.5).unwrap();
        settings.put_bool(AppSettings, "A/Bool", true).unwrap();
        let date = LocalDateTime::parse("12/20/2025 5:43:19 PM").unwrap();
        settings.put_date(AppSettings, "A/Date", date).unwrap();
        let back = ProfileSettings::open(&path);
        assert_eq!(back.get_i32(AppSettings, "A/Int", 0), -7);
        assert_eq!(back.get_f64(AppSettings, "A/Double", 0.0), 1.5);
        assert_eq!(back.get_f32(AppSettings, "A/Double", 0.0), 1.5);
        assert!(back.get_bool(AppSettings, "A/Bool", false));
        assert_eq!(back.get_string(AppSettings, "A/Bool", ""), "True");
        assert_eq!(back.get_date(AppSettings, "A/Date"), Some(date));
        assert_eq!(back.get_string(AppSettings, "A/Date", ""), "12/20/2025 5:43:19 PM");
    }

    #[test]
    fn a_missing_key_gives_the_default() {
        let settings = ProfileSettings::open(temp_file("settings-missing"));
        assert_eq!(settings.get_string(Profiles, "Nope/Name", "d"), "d");
        assert_eq!(settings.get_i32(Profiles, "Nope", 3), 3);
        assert_eq!(settings.get_date(Profiles, "Nope"), None);
    }

    #[test]
    fn bad_text_gives_the_default_rather_than_throwing() {
        let path = temp_file("settings-bad-text");
        std::fs::write(&path, "<settings><Profiles><ProfileCount>many</ProfileCount><When>soon</When><Flag>maybe</Flag></Profiles></settings>").unwrap();
        let settings = ProfileSettings::open(&path);
        assert_eq!(settings.get_i32(Profiles, "ProfileCount", 5), 5);
        assert_eq!(settings.get_date(Profiles, "When"), None);
        assert!(settings.get_bool(Profiles, "Flag", true));
    }

    #[test]
    fn a_corrupt_file_gives_defaults_and_can_be_written_again() {
        let path = temp_file("settings-corrupt");
        std::fs::write(&path, "<settings><Profiles>").unwrap();
        let settings = ProfileSettings::open(&path);
        assert_eq!(settings.get_i32(Profiles, "ProfileCount", 0), 0);
        settings.put_i32(Profiles, "ProfileCount", 2).unwrap();
        assert_eq!(ProfileSettings::open(&path).get_i32(Profiles, "ProfileCount", 0), 2);
    }

    #[test]
    fn delete_node_removes_a_preview_key_only() {
        let path = temp_file("settings-delete");
        let settings = ProfileSettings::open(&path);
        settings.put_i32(AppSettings, "Preview_x/Left", 1).unwrap();
        settings.put_i32(AppSettings, "Preview_y/Left", 2).unwrap();
        settings.delete_node(AppSettings, "Preview_x").unwrap();
        let back = ProfileSettings::open(&path);
        assert_eq!(back.get_i32(AppSettings, "Preview_x/Left", -1), -1);
        assert_eq!(back.get_i32(AppSettings, "Preview_y/Left", -1), 2);
    }

    #[test]
    fn rename_and_remove_with_a_full_path() {
        let path = temp_file("settings-rename");
        let settings = ProfileSettings::open(&path);
        settings.put_i32(AppSettings, "Old/Value", 9).unwrap();
        settings.rename_node(AppSettings, "Old", "New").unwrap();
        assert_eq!(settings.get_i32(AppSettings, "New/Value", 0), 9);
        settings.remove_node("AppSettings/New").unwrap();
        assert_eq!(ProfileSettings::open(&path).get_i32(AppSettings, "New/Value", 0), 0);
    }

    #[test]
    fn unknown_sections_and_members_survive_a_save() {
        let path = temp_file("settings-unknown");
        std::fs::write(&path, "<settings><Future a=\"1\"><Deep>kept</Deep></Future><Profiles><Other>o</Other></Profiles></settings>").unwrap();
        let settings = ProfileSettings::open(&path);
        settings.put_i32(Profiles, "ProfileCount", 1).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("<Future a=\"1\">"), "{text}");
        assert!(text.contains("<Deep>kept</Deep>"), "{text}");
        assert!(text.contains("<Other>o</Other>"), "{text}");
    }

    #[test]
    fn put_many_saves_everything_together() {
        let path = temp_file("settings-many");
        let settings = ProfileSettings::open(&path);
        settings.put_many(Profiles, &[("A/Name", "a".to_string()), ("B/Name", "b".to_string())]).unwrap();
        let back = ProfileSettings::open(&path);
        assert_eq!(back.get_string(Profiles, "A/Name", ""), "a");
        assert_eq!(back.get_string(Profiles, "B/Name", ""), "b");
    }

    #[test]
    fn a_name_xml_cannot_hold_is_refused_and_writes_nothing() {
        let path = temp_file("settings-bad-name");
        let settings = ProfileSettings::open(&path);
        assert!(settings.put_string(AppSettings, "Bad Name", "x").is_err());
        assert!(!path.exists());
    }

    #[test]
    fn a_second_store_on_the_file_keeps_what_the_first_wrote() {
        let path = temp_file("settings-two");
        let first = ProfileSettings::open(&path);
        let second = ProfileSettings::open(&path);
        first.put_string(Profiles, "FromFirst", "1").unwrap();
        // Make sure the file's time moves even where the clock is coarse.
        std::thread::sleep(std::time::Duration::from_millis(5));
        second.put_string(Profiles, "FromSecond", "2").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        first.put_string(Profiles, "FromFirstAgain", "3").unwrap();
        let back = ProfileSettings::open(&path);
        assert_eq!(back.get_string(Profiles, "FromFirst", ""), "1");
        assert_eq!(back.get_string(Profiles, "FromSecond", ""), "2");
        assert_eq!(back.get_string(Profiles, "FromFirstAgain", ""), "3");
    }

    #[test]
    fn a_read_sees_what_another_store_saved_since() {
        let path = temp_file("settings-read-fresh");
        let first = ProfileSettings::open(&path);
        let second = ProfileSettings::open(&path);
        assert_eq!(second.get_i32(Profiles, "ProfileCount", 0), 0);
        first.put_i32(Profiles, "ProfileCount", 2).unwrap();
        assert_eq!(second.get_i32(Profiles, "ProfileCount", 0), 2);
        std::thread::sleep(std::time::Duration::from_millis(5));
        first.put_string(Profiles, "Profile0/Name", "Show").unwrap();
        assert_eq!(second.get_string(Profiles, "Profile0/Name", ""), "Show");
    }

    #[test]
    fn text_needing_escapes_round_trips() {
        let path = temp_file("settings-escape");
        let settings = ProfileSettings::open(&path);
        settings.put_string(Profiles, "Profile0/Name", "Tom & Jerry's <Show>").unwrap();
        assert_eq!(ProfileSettings::open(&path).get_string(Profiles, "Profile0/Name", ""), "Tom & Jerry's <Show>");
    }
}
