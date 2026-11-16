//! Mark import and export as the panel runs them: the Import Type Selection's routing (F25, F26), the Select Imported Mark Collections
//! dialog's rows (F35), the commit (F36), the export dialog (F37 to F40) and the shared last folder (F27).

use super::{is_unique_name, is_visible, unique_name};
use crate::sequencer::papagayo;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_core::marks::{MarkCollection, MarkCollectionType};
use vixen_core::Color;
use vixen_io::marks_files::{self as files, PangolinMode, PangolinRecord};

/// F25's radios, top to bottom.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ImportKind {
    VampBars,
    VampBeats,
    AudacityBeats,
    Vixen3,
    XTiming,
    Papagayo,
    SingingFaces,
    PangolinBeyond,
}

/// A file type for a picker: its description and extensions. Every picker also offers All Files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    pub description: &'static str,
    pub extensions: &'static [&'static str],
}

impl ImportKind {
    pub const ALL: [ImportKind; 8] = [
        ImportKind::VampBars,
        ImportKind::VampBeats,
        ImportKind::AudacityBeats,
        ImportKind::Vixen3,
        ImportKind::XTiming,
        ImportKind::Papagayo,
        ImportKind::SingingFaces,
        ImportKind::PangolinBeyond,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ImportKind::VampBars => "Vamp Beat Tracker: Bars",
            ImportKind::VampBeats => "Vamp Beat Tracker: Beats",
            ImportKind::AudacityBeats => "Audacity Beats",
            ImportKind::Vixen3 => "Vixen 3",
            ImportKind::XTiming => "xTiming",
            ImportKind::Papagayo => "Papagayo Voice Tracks",
            ImportKind::SingingFaces => "Singing Faces Lyric Browser",
            ImportKind::PangolinBeyond => "Pangolin Beyond",
        }
    }

    /// F26's routing. KEEP: "Audacity Beats" runs the bar reader and "Vamp Beat Tracker: Beats" the beat reader, whose picker says "Audacity
    /// Beat Labels".
    fn reader(self) -> Reader {
        match self {
            ImportKind::VampBars | ImportKind::AudacityBeats => Reader::BarLabels,
            ImportKind::VampBeats => Reader::BeatLabels,
            ImportKind::Vixen3 => Reader::Vixen3,
            ImportKind::XTiming => Reader::XTiming,
            ImportKind::Papagayo => Reader::Papagayo,
            ImportKind::SingingFaces => Reader::SingingFaces,
            ImportKind::PangolinBeyond => Reader::Pangolin,
        }
    }

    /// The picker's filter, or `None` for the Lyric Browser, which has no file.
    pub fn filter(self) -> Option<FileFilter> {
        Some(match self.reader() {
            Reader::BarLabels => FileFilter { description: "Audacity Bar Labels", extensions: &["txt"] },
            Reader::BeatLabels => FileFilter { description: "Audacity Beat Labels", extensions: &["txt"] },
            Reader::Vixen3 => FileFilter { description: "Vixen 3 Mark Collection (*.v3m)", extensions: &["v3m"] },
            Reader::XTiming => FileFilter { description: "xTiming", extensions: &["xTiming", "xml"] },
            Reader::Papagayo => FileFilter { description: "Papagayo files (*.pgo)", extensions: &["pgo"] },
            Reader::Pangolin => FileFilter { description: "Pangolin Beyond CSV (*.csv)", extensions: &["csv"] },
            Reader::SingingFaces => return None,
        })
    }

    /// The alert each importer shows for any failure, reading included: (message, title).
    pub fn error(self) -> (&'static str, &'static str) {
        match self.reader() {
            Reader::BarLabels => ("There was an error importing the Audacity bar marks.", "Audacity Import Error"),
            Reader::BeatLabels => ("There was an error importing the Audacity beat marks.", "Audacity Import Error"),
            Reader::Vixen3 => ("There was an error importing the Vixen Marks.", "Vixen Marks Import Error"),
            Reader::XTiming => ("There was an error importing the xTiming marks.", "xTiming Import Error"),
            Reader::Papagayo => ("There was an error importing the Papagayo marks.", "Papagayo Import Error"),
            Reader::SingingFaces => ("There was an error importing the Singing Faces timing marks.", "Singing Faces Import Error"),
            Reader::Pangolin => ("There was an error importing the Pangolin Beyond marks.", "Pangolin Beyond Import Error"),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Reader {
    BarLabels,
    BeatLabels,
    Vixen3,
    XTiming,
    Papagayo,
    SingingFaces,
    Pangolin,
}

/// What reading an import file gave.
#[derive(Clone, Debug)]
pub enum ImportRead {
    /// Collections to offer in F35. Empty means the flow ends there, silently (F26).
    Candidates(Vec<MarkCollection>),
    /// F32: an information alert first, then F35 with these. KEEP: it says "imported" before anything has been chosen.
    CandidatesAfterMessage { title: &'static str, message: String, candidates: Vec<MarkCollection> },
    /// F34 with two or more colours: ask "Create a Mark Collection for each Beyond color?" first.
    PangolinChoice(Vec<PangolinRecord>),
    /// The importer's alert. `detail` is only for the log.
    Failed { message: &'static str, title: &'static str, detail: String },
    /// Not built yet in this port. The text says so plainly.
    NotBuilt(String),
}

/// F28 to F34: reads a chosen file's bytes. `file_stem` is the file's name without its extension, which names F32's collections. Nothing
/// changes until `commit_import`.
pub fn read_candidates(kind: ImportKind, bytes: &[u8], file_stem: &str) -> ImportRead {
    let text = files::read_import_text(bytes);
    let failed = |detail: String| {
        let (message, title) = kind.error();
        ImportRead::Failed { message, title, detail }
    };
    let result = match kind.reader() {
        Reader::BarLabels => files::read_bar_labels(&text),
        Reader::BeatLabels => files::read_beat_labels(&text),
        Reader::Vixen3 => files::read_v3m(&text),
        Reader::XTiming => files::read_xtiming(&text),
        Reader::Pangolin => {
            return match files::parse_pangolin(&text) {
                Err(detail) => failed(detail),
                Ok(records) if files::pangolin_needs_choice(&records) => ImportRead::PangolinChoice(records),
                Ok(records) => ImportRead::Candidates(files::pangolin_collections(&records, PangolinMode::GroupByColor)),
            };
        }
        Reader::Papagayo => {
            return match papagayo::parse_papagayo(&text) {
                Err(detail) => failed(detail),
                Ok(doc) => ImportRead::CandidatesAfterMessage {
                    title: "Papagayo Import",
                    message: papagayo::marks_import_summary(&doc),
                    candidates: papagayo::mark_collections(&doc, file_stem),
                },
            };
        }
        Reader::SingingFaces => files::read_xtiming(&text),
    };
    match result {
        Ok(c) => ImportRead::Candidates(c),
        Err(e) => failed(e.0),
    }
}

/// An import this port hasn't built yet, said plainly, so the flow can stop before a picker opens.
pub fn not_built(kind: ImportKind) -> Option<&'static str> {
    let _ = kind;
    None
}

#[cfg(test)]
mod singing_faces_tests {
    use super::*;
    #[test]
    fn downloaded_timing_uses_existing_candidates_and_error_contract() {
        let xml = br#"<timing name="Song"><EffectLayer><Effect label="hello" starttime="100" endtime="350"/></EffectLayer></timing>"#;
        let ImportRead::Candidates(rows) = read_candidates(ImportKind::SingingFaces, xml, "") else { panic!("expected candidates") };
        assert_eq!(rows.len(), 1); assert_eq!(rows[0].marks.len(), 1); assert_eq!(rows[0].marks[0].start_time.total_ms(), 100.0);
        assert!(matches!(read_candidates(ImportKind::SingingFaces, b"", ""), ImportRead::Failed {title:"Singing Faces Import Error",..}));
        assert!(ImportKind::SingingFaces.filter().is_none()); assert!(not_built(ImportKind::SingingFaces).is_none());
    }
}

/// The failure a file that can't be read gives: the importer's own alert.
pub fn read_failed(kind: ImportKind, detail: String) -> ImportRead {
    let (message, title) = kind.error();
    ImportRead::Failed { message, title, detail }
}

/// F34's answer: one per colour, or one collection in a chosen colour.
pub fn pangolin_candidates(records: &[PangolinRecord], mode: PangolinMode) -> Vec<MarkCollection> {
    files::pangolin_collections(records, mode)
}

/// One row of F35.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportOption {
    pub name: String,
    /// The caution icon: the name is in the sequence already, ignoring case and surrounding space, or blank. Worked out against the sequence,
    /// not between candidates.
    pub has_duplicate_name: bool,
}

pub const DUPLICATE_NAME_TIP: &str = "Collection name already exists in this sequence. This will be renamed during import.";

/// F35's rows, all starting checked.
pub fn import_options(existing: &[MarkCollection], candidates: &[MarkCollection]) -> Vec<ImportOption> {
    candidates.iter().map(|c| ImportOption { name: c.name.clone(), has_duplicate_name: !is_unique_name(existing, &c.name, None) }).collect()
}

/// F36, `MarkCollectionImportCommitter.Commit`, in its three passes: links to anything outside the sequence and the selection are cleared;
/// with a default already in the sequence every candidate's flag clears, else the first flagged, first visible or first candidate becomes
/// it; then each is appended under a name unique against the growing list. Returns whether anything was added.
pub fn commit_import(target: &mut Vec<MarkCollection>, mut selected: Vec<MarkCollection>) -> bool {
    if selected.is_empty() {
        return false;
    }
    let valid: HashSet<Uuid> = target.iter().map(|c| c.id).chain(selected.iter().map(|c| c.id)).collect();
    for c in &mut selected {
        if c.linked_mark_collection_id.is_some_and(|id| !valid.contains(&id)) {
            c.linked_mark_collection_id = None;
        }
    }
    let chosen = if target.iter().any(|c| c.is_default) {
        None
    } else {
        Some(selected.iter().position(|c| c.is_default).or_else(|| selected.iter().position(is_visible)).unwrap_or(0))
    };
    for (i, c) in selected.iter_mut().enumerate() {
        c.is_default = Some(i) == chosen;
    }
    for mut c in selected {
        c.name = unique_name(&c.name, target, None);
        target.push(c);
    }
    true
}

// ---------------------------------------------------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------------------------------------------------

/// F37's formats, in order.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ExportFormat {
    Vixen,
    Audacity,
    PangolinBeyond,
}

impl ExportFormat {
    pub fn filter(self) -> FileFilter {
        match self {
            ExportFormat::Vixen => FileFilter { description: "Vixen 3 Mark Collection (*.v3m)", extensions: &["v3m"] },
            ExportFormat::Audacity => FileFilter { description: "Audacity Marks (*.txt)", extensions: &["txt"] },
            ExportFormat::PangolinBeyond => FileFilter { description: "CSV Files (*.csv)", extensions: &["csv"] },
        }
    }

    /// F27, KEEP: the Audacity and Pangolin Beyond save panels neither start in nor update the last folder.
    pub fn uses_last_folder(self) -> bool {
        self == ExportFormat::Vixen
    }
}

/// One row of F37's table.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportRow {
    pub id: Uuid,
    pub name: String,
    pub color: Color,
    /// Include Mark Text starts on for Phrase, Word, Phoneme and State, off for Generic. Shown only for Pangolin Beyond.
    pub include_text: bool,
}

pub fn export_rows(collections: &[MarkCollection]) -> Vec<ExportRow> {
    collections
        .iter()
        .map(|c| ExportRow { id: c.id, name: c.name.clone(), color: c.decorator.color, include_text: c.collection_type != MarkCollectionType::Generic })
        .collect()
}

/// F38 to F40: the file's bytes, from the included collections in list order. OK with none included still writes a file.
pub fn build_export(format: ExportFormat, collections: &[MarkCollection], included: &[Uuid], text_included: &[Uuid]) -> Result<Vec<u8>, String> {
    let chosen: Vec<&MarkCollection> = collections.iter().filter(|c| included.contains(&c.id)).collect();
    match format {
        ExportFormat::Vixen => files::write_v3m(&chosen).map_err(|e| e.0),
        ExportFormat::Audacity => Ok(files::write_audacity(&chosen).into_bytes()),
        ExportFormat::PangolinBeyond => {
            let rows: Vec<(&MarkCollection, bool)> = chosen.iter().map(|c| (*c, text_included.contains(&c.id))).collect();
            Ok(files::write_pangolin(&rows).into_bytes())
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Last folder, F27
// ---------------------------------------------------------------------------------------------------------------------

/// Upstream's static `_lastFolder`: one for the process, never saved.
static LAST_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Where the next import picker, or a Vixen export panel, starts: the last folder chosen, else the profile's data folder.
pub fn last_folder(data_root: &Path) -> PathBuf {
    LAST_FOLDER.lock().clone().unwrap_or_else(|| data_root.to_path_buf())
}

/// After a picker's OK: the folder of the file chosen.
pub fn remember_folder(file: &Path) {
    if let Some(parent) = file.parent() {
        *LAST_FOLDER.lock() = Some(parent.to_path_buf());
    }
}
