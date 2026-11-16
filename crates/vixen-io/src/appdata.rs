//! `VixenApplicationData.xml`: recent sequences and a few window settings. Port of `VixenApplicationData`.

use crate::dcs::{child_f64, child_i32, format_double, format_guid, parse_bool, parse_guid, windows_file_name};
use crate::error::IoError;
use crate::fidelity::{diff, reapply, Residue};
use crate::file::write_atomic;
use crate::paths::DataPaths;
use crate::xml::{read_xml_file, write_document, DocStyle};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_xml::XElement;

/// `VixenApplicationData.DataFormatVersionNumber`.
pub const CURRENT_APPLICATION_DATA_VERSION: i32 = 3;

/// `_maxRecentSequences`.
pub const MAX_RECENT_SEQUENCES: usize = 20;

/// `FilterSetupFormShapePosition`.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct FilterShapePosition {
    pub x_position_proportion: f64,
    pub y_position: i32,
}

/// A recent sequence entry, resolved against the current profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentSequence {
    /// What the file stored, often a Windows path from another machine.
    pub stored: String,
    /// Where that file name is in this profile's Sequence folder.
    pub path: PathBuf,
    /// Whether that path holds a file now.
    pub exists: bool,
}

/// `VixenApplicationData`.
#[derive(Clone, Debug, Default)]
pub struct ApplicationData {
    pub data_format_version: i32,
    /// Most recent first, as stored.
    pub recent_sequences: Vec<String>,
    /// By filter id, in file order.
    pub filter_shape_positions: Vec<(Uuid, FilterShapePosition)>,
    pub filter_setup_high_quality_rendering: bool,
    residue: Option<Residue>,
}

impl ApplicationData {
    /// A new, empty set of settings.
    pub fn new() -> ApplicationData {
        ApplicationData { data_format_version: CURRENT_APPLICATION_DATA_VERSION, ..ApplicationData::default() }
    }

    /// The recent sequences as files in this profile. Each resolves by file name, because a stored Windows
    /// path means nothing here and a profile moved from Windows keeps the same Sequence folder contents.
    pub fn resolve_recent_sequences(&self, paths: &DataPaths) -> Vec<RecentSequence> {
        let folder = paths.sequence_dir();
        self.recent_sequences
            .iter()
            .map(|stored| {
                let path = folder.join(windows_file_name(stored));
                let exists = path.is_file();
                RecentSequence { stored: stored.clone(), path, exists }
            })
            .collect()
    }

    /// The rows the Recent Sequences list shows. A POSIX path to a file that exists is used as it is, so a sequence opened from another
    /// folder stays where it was. Anything else, a Windows path included, resolves by name in this profile's Sequence folder. Rows
    /// with no file are skipped but stay stored, and a later row reaching the same file as an earlier one is dropped.
    pub fn visible_recent_sequences(&self, paths: &DataPaths) -> Vec<RecentSequence> {
        let mut seen: Vec<PathBuf> = Vec::new();
        self.recent_sequences
            .iter()
            .filter_map(|stored| {
                let path = resolve_recent_path(stored, paths);
                if !path.is_file() {
                    return None;
                }
                let identity = file_identity(&path);
                if seen.contains(&identity) {
                    return None;
                }
                seen.push(identity);
                Some(RecentSequence { stored: stored.clone(), path, exists: true })
            })
            .collect()
    }

    /// `AddSequenceToRecentList`, without the save: moves the file to the front and trims the list. A sequence never saved has an empty
    /// path, which the original stores, using up a slot; here it's skipped. An older row reaching the same file goes too, so a new
    /// `/Users/.../Foo.tim` replaces `C:\...\Foo.tim`.
    pub fn add_recent_sequence(&mut self, path: &str, paths: &DataPaths) {
        if path.is_empty() {
            return;
        }
        let identity = file_identity(&resolve_recent_path(path, paths));
        self.recent_sequences
            .retain(|stored| stored != path && file_identity(&resolve_recent_path(stored, paths)) != identity);
        self.recent_sequences.insert(0, path.to_string());
        self.recent_sequences.truncate(MAX_RECENT_SEQUENCES);
    }

    /// `ClearRecentSequencesList`. It reaches the disk with the next save.
    pub fn clear_recent_sequences(&mut self) {
        self.recent_sequences.clear();
    }
}

fn resolve_recent_path(stored: &str, paths: &DataPaths) -> PathBuf {
    let as_given = Path::new(stored);
    if stored.starts_with('/') && as_given.is_file() {
        return as_given.to_path_buf();
    }
    paths.sequence_dir().join(windows_file_name(stored))
}

/// Two spellings of one file compare equal, through links and `/tmp` too.
fn file_identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Reads the application data. A missing file gives empty settings, as `VixenApplicationData.LoadData` does.
pub fn load_application_data(paths: &DataPaths) -> Result<ApplicationData, IoError> {
    let path = paths.application_data_file();
    if !path.is_file() {
        return Ok(ApplicationData::new());
    }
    let root = read_xml_file(&path)?;
    application_data_from_xml(&root)
}

/// Writes the application data atomically. The C# keeps no backup of this file.
pub fn save_application_data(data: &ApplicationData, paths: &DataPaths) -> Result<(), IoError> {
    let root = reapply(encode(data), data.residue.as_ref());
    write_atomic(&paths.application_data_file(), write_document(&root, DocStyle::default()).as_bytes())
}

fn application_data_from_xml(root: &XElement) -> Result<ApplicationData, IoError> {
    let Some(version) = root.child_text("DataFormatVersion").and_then(|t| t.parse::<i32>().ok()) else {
        return Err(IoError::format("application data", "couldn't find data format version"));
    };
    let mut data = ApplicationData { data_format_version: version, ..ApplicationData::default() };
    // `ReadData` gives up on a version it does not know. The residue still keeps the file's content.
    if version <= CURRENT_APPLICATION_DATA_VERSION {
        if let Some(list) = root.child("RecentSequences") {
            data.recent_sequences = list.children_named("SequenceFile").map(|e| e.text()).collect();
        }
        if version >= 2 {
            if let Some(list) = root.child("FilterShapePositions") {
                for e in list.children_named("FilterPosition") {
                    let position = FilterShapePosition {
                        x_position_proportion: child_f64(e, "xPositionProportion").unwrap_or(0.0),
                        y_position: child_i32(e, "yPosition").unwrap_or(0),
                    };
                    if let Some(id) = e.attr("FilterId").and_then(parse_guid) {
                        data.filter_shape_positions.push((id, position));
                    }
                }
            }
        }
        if version >= 3 {
            data.filter_setup_high_quality_rendering = root
                .child("FilterSetupFormHighQualityRendering")
                .and_then(|e| e.attr("value"))
                .and_then(parse_bool)
                .unwrap_or(false);
        }
    }
    // `SaveData` always writes the current version. Kept at an older one, the members that version lacks would be saved and then
    // skipped by the next load.
    let mut as_saved = root.clone();
    if version < CURRENT_APPLICATION_DATA_VERSION {
        if let Some(e) = as_saved.child_mut("DataFormatVersion") {
            *e = XElement::with_text("", "DataFormatVersion", CURRENT_APPLICATION_DATA_VERSION.to_string());
        }
    }
    data.residue = diff(&as_saved, &encode(&data));
    Ok(data)
}

fn encode(data: &ApplicationData) -> XElement {
    let mut root = XElement::new("", "VixenApplicationData");
    // GUARD: a file from a newer Vixen keeps its version with its content, where upstream would stamp its own on an emptied file.
    let version = data.data_format_version.max(CURRENT_APPLICATION_DATA_VERSION);
    root.push(XElement::with_text("", "DataFormatVersion", version.to_string()));
    let mut recent = XElement::new("", "RecentSequences");
    for s in &data.recent_sequences {
        recent.push(XElement::with_text("", "SequenceFile", s.as_str()));
    }
    root.push(recent);
    let mut positions = XElement::new("", "FilterShapePositions");
    for (id, p) in &data.filter_shape_positions {
        let mut e = XElement::new("", "FilterPosition");
        e.set_attr("FilterId", format_guid(id));
        e.push(XElement::with_text("", "xPositionProportion", format_double(p.x_position_proportion)));
        e.push(XElement::with_text("", "yPosition", p.y_position.to_string()));
        positions.push(e);
    }
    root.push(positions);
    let mut hq = XElement::new("", "FilterSetupFormHighQualityRendering");
    hq.set_attr("value", if data.filter_setup_high_quality_rendering { "true" } else { "false" });
    root.push(hq);
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::same_tree;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<VixenApplicationData>
  <DataFormatVersion>3</DataFormatVersion>
  <RecentSequences>
    <SequenceFile>C:\Users\developer\OneDrive\Documents\Vixen 3\Sequence\Coldest Winter.tim</SequenceFile>
    <SequenceFile>/Users/me/Vixen 3/Sequence/Carey Grinch.tim</SequenceFile>
  </RecentSequences>
  <FilterShapePositions>
    <FilterPosition FilterId="034ce5a2-fb68-4296-b13d-0e8632391309">
      <xPositionProportion>0.25</xPositionProportion>
      <yPosition>40</yPosition>
    </FilterPosition>
  </FilterShapePositions>
  <FilterSetupFormHighQualityRendering value="true" />
</VixenApplicationData>"#;

    #[test]
    fn reads_and_writes_every_setting() {
        let root = XElement::parse(SAMPLE).unwrap();
        let data = application_data_from_xml(&root).unwrap();
        assert_eq!(data.data_format_version, 3);
        assert_eq!(data.recent_sequences.len(), 2);
        assert_eq!(
            data.filter_shape_positions[0].1,
            FilterShapePosition { x_position_proportion: 0.25, y_position: 40 }
        );
        assert!(data.filter_setup_high_quality_rendering);
        assert!(data.residue.is_none());
        let out = reapply(encode(&data), data.residue.as_ref());
        assert!(same_tree(&root, &out), "{:?}", root.first_difference(&out, String::new()));
    }

    #[test]
    fn recent_paths_resolve_by_file_name_in_the_current_profile() {
        let data = application_data_from_xml(&XElement::parse(SAMPLE).unwrap()).unwrap();
        let paths = DataPaths::new("/profile/Vixen 3");
        let resolved = data.resolve_recent_sequences(&paths);
        assert_eq!(resolved[0].path, PathBuf::from("/profile/Vixen 3/Sequence/Coldest Winter.tim"));
        assert_eq!(resolved[1].path, PathBuf::from("/profile/Vixen 3/Sequence/Carey Grinch.tim"));
        assert!(!resolved[0].exists);
        assert_eq!(resolved[0].stored, data.recent_sequences[0]);
    }

    fn profile_with_sequences(name: &str, files: &[&str]) -> (PathBuf, DataPaths) {
        let dir = crate::test_dir(name);
        let paths = DataPaths::new(dir.join("Vixen 3"));
        std::fs::create_dir_all(paths.sequence_dir()).unwrap();
        for file in files {
            std::fs::write(paths.sequence_dir().join(file), "").unwrap();
        }
        (dir, paths)
    }

    #[test]
    fn the_owners_real_list_gives_fourteen_rows_in_order() {
        // VixenApplicationData.xml from the real show: 20 Windows paths under two prefixes, six names under both.
        let usera = "C:\\Users\\developer\\OneDrive\\Documents\\Vixen 3\\Sequence\\";
        let userb = "C:\\Users\\userb\\Documents\\Vixen 3\\Sequence\\";
        let first = [
            "Coldest Winter.tim",
            "Carey Grinch.tim",
            "Christmas Eve Sarajevo.tim",
            "Dance of the Sugar Plum Fairy.tim",
            "Carol of the Bells.tim",
            "Snoopy Christmas.tim",
            "Christmas Intro 1.tim",
            "Wizards in Winter.tim",
            "The Night Before Christmas.tim",
            "Do They Know It's Christmas.tim",
            "Christmas Wrapping (Glee).tim",
            "Christmas Can-Can.tim",
            "Zoom.tim",
            "Carol of the Bells (Lindsey Stirling).tim",
        ];
        let second = [
            "Zoom.tim",
            "Do They Know It's Christmas.tim",
            "Christmas Eve Sarajevo.tim",
            "The Night Before Christmas.tim",
            "Carey Grinch.tim",
            "Dance of the Sugar Plum Fairy.tim",
        ];
        let (_dir, paths) = profile_with_sequences("appdata-real", &first);
        let mut data = ApplicationData::new();
        data.recent_sequences = first.iter().map(|f| format!("{usera}{f}")).collect();
        data.recent_sequences.extend(second.iter().map(|f| format!("{userb}{f}")));
        assert_eq!(data.recent_sequences.len(), 20);
        let rows = data.visible_recent_sequences(&paths);
        let names: Vec<String> = rows.iter().map(|r| r.path.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, first);
    }

    #[test]
    fn a_posix_path_to_a_file_elsewhere_stays_as_it_is() {
        let (dir, paths) = profile_with_sequences("appdata-posix", &["Foo.tim"]);
        let elsewhere = dir.join("Other").join("Foo.tim");
        std::fs::create_dir_all(elsewhere.parent().unwrap()).unwrap();
        std::fs::write(&elsewhere, "").unwrap();
        let mut data = ApplicationData::new();
        data.recent_sequences = vec![elsewhere.to_string_lossy().into_owned(), "C:\\x\\Foo.tim".to_string()];
        let rows = data.visible_recent_sequences(&paths);
        assert_eq!(rows.len(), 2, "two different files with one name");
        assert_eq!(rows[0].path, elsewhere);
        assert_eq!(rows[1].path, paths.sequence_dir().join("Foo.tim"));
    }

    #[test]
    fn a_posix_path_to_a_missing_file_falls_back_to_the_sequence_folder() {
        let (_dir, paths) = profile_with_sequences("appdata-fallback", &["Foo.tim"]);
        let mut data = ApplicationData::new();
        data.recent_sequences = vec!["/gone/Foo.tim".to_string(), "/gone/Bar.tim".to_string()];
        let rows = data.visible_recent_sequences(&paths);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, paths.sequence_dir().join("Foo.tim"));
        assert_eq!(data.recent_sequences.len(), 2, "a hidden row stays stored");
    }

    #[test]
    fn adding_replaces_the_exact_match_and_any_windows_row_for_the_same_file() {
        let (_dir, paths) = profile_with_sequences("appdata-add", &["Foo.tim", "Bar.tim"]);
        let foo = paths.sequence_dir().join("Foo.tim").to_string_lossy().into_owned();
        let mut data = ApplicationData::new();
        data.recent_sequences =
            vec!["C:\\x\\Bar.tim".to_string(), "C:\\x\\Foo.tim".to_string(), "D:\\y\\Foo.tim".to_string(), foo.clone()];
        data.add_recent_sequence(&foo, &paths);
        assert_eq!(data.recent_sequences, vec![foo, "C:\\x\\Bar.tim".to_string()]);
    }

    #[test]
    fn adding_keeps_twenty_and_ignores_an_empty_path() {
        let (_dir, paths) = profile_with_sequences("appdata-cap", &[]);
        let mut data = ApplicationData::new();
        for i in 0..21 {
            data.add_recent_sequence(&format!("/s/{i}.tim"), &paths);
        }
        assert_eq!(data.recent_sequences.len(), MAX_RECENT_SEQUENCES);
        assert_eq!(data.recent_sequences[0], "/s/20.tim");
        assert_eq!(data.recent_sequences[19], "/s/1.tim");
        data.add_recent_sequence("", &paths);
        assert_eq!(data.recent_sequences[0], "/s/20.tim");
        assert_eq!(data.recent_sequences.len(), MAX_RECENT_SEQUENCES);
    }

    #[test]
    fn clearing_writes_nothing_until_a_save() {
        let (_dir, paths) = profile_with_sequences("appdata-clear", &[]);
        let mut data = ApplicationData::new();
        data.recent_sequences.push("/a/b.tim".to_string());
        save_application_data(&data, &paths).unwrap();
        data.clear_recent_sequences();
        assert!(data.recent_sequences.is_empty());
        assert_eq!(load_application_data(&paths).unwrap().recent_sequences.len(), 1);
        save_application_data(&data, &paths).unwrap();
        assert!(load_application_data(&paths).unwrap().recent_sequences.is_empty());
    }

    #[test]
    fn unknown_members_survive() {
        let text = SAMPLE.replace("</VixenApplicationData>", "<Future a=\"1\" /></VixenApplicationData>");
        let root = XElement::parse(&text).unwrap();
        let mut data = application_data_from_xml(&root).unwrap();
        assert!(same_tree(&root, &reapply(encode(&data), data.residue.as_ref())));
        data.recent_sequences.clear();
        let edited = reapply(encode(&data), data.residue.as_ref());
        assert!(edited.child("Future").is_some());
        assert_eq!(edited.child("RecentSequences").unwrap().elements().count(), 0);
    }

    #[test]
    fn a_file_with_no_version_is_an_error() {
        assert!(application_data_from_xml(&XElement::parse("<VixenApplicationData/>").unwrap()).is_err());
    }

    #[test]
    fn old_versions_ignore_later_members() {
        let text = SAMPLE.replace("<DataFormatVersion>3<", "<DataFormatVersion>1<");
        let data = application_data_from_xml(&XElement::parse(&text).unwrap()).unwrap();
        assert_eq!(data.recent_sequences.len(), 2);
        assert!(data.filter_shape_positions.is_empty());
        assert!(!data.filter_setup_high_quality_rendering);
    }

    #[test]
    fn an_older_file_saves_as_the_current_version_so_its_new_members_are_read_back() {
        let dir = crate::test_dir("appdata-upgrade");
        let paths = DataPaths::new(&dir);
        let old = "<VixenApplicationData><DataFormatVersion>1</DataFormatVersion><RecentSequences /></VixenApplicationData>";
        std::fs::write(paths.application_data_file(), old).unwrap();
        let mut data = load_application_data(&paths).unwrap();
        assert_eq!(data.data_format_version, 1);
        data.filter_setup_high_quality_rendering = true;
        data.filter_shape_positions.push((Uuid::from_u128(7), FilterShapePosition { x_position_proportion: 0.5, y_position: 3 }));
        save_application_data(&data, &paths).unwrap();
        let back = load_application_data(&paths).unwrap();
        assert_eq!(back.data_format_version, CURRENT_APPLICATION_DATA_VERSION);
        assert!(back.filter_setup_high_quality_rendering);
        assert_eq!(back.filter_shape_positions.len(), 1);
    }

    #[test]
    fn a_newer_file_keeps_its_version_and_content() {
        let text = SAMPLE.replace("<DataFormatVersion>3<", "<DataFormatVersion>4<");
        let root = XElement::parse(&text).unwrap();
        let data = application_data_from_xml(&root).unwrap();
        assert!(data.recent_sequences.is_empty());
        assert!(same_tree(&root, &reapply(encode(&data), data.residue.as_ref())));
    }

    #[test]
    fn a_new_file_saves_and_loads() {
        let dir = crate::test_dir("appdata");
        let paths = DataPaths::new(&dir);
        let mut data = load_application_data(&paths).unwrap();
        assert!(data.recent_sequences.is_empty());
        data.recent_sequences.push("/a/b.tim".to_string());
        save_application_data(&data, &paths).unwrap();
        let back = load_application_data(&paths).unwrap();
        assert_eq!(back.recent_sequences, ["/a/b.tim"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
