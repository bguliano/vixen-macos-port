//! Port of `Vixen.Core/Export`: turning a sequence into a file of raw channel data, one row per
//! interval, for Falcon Player and other tools.
//!
//! The writers only need one byte per controller output for each interval, so this crate doesn't
//! depend on the engine or on output modules. [`run::run_export`] takes the bytes from an
//! [`run::IntervalSource`], which the session layer connects to the real sequence.
//!
//! Deliberate differences from the C#, each recorded where it happens:
//!
//! * FSEQ blocks are complete zstd frames. Vixen takes each block's bytes after flushing but before
//!   ending the stream, so its frames have no final block. Streaming decoders (FPP, xLights, the
//!   Python `zstandard` reader) accept both, and only the complete form also works with one-shot
//!   decoders. [`fseq_read`] reads both.
//! * A sequence of fewer than ten frames gets one compression block. In the C#, `frames - 10` wraps
//!   round as an unsigned value, so the header declares 255 blocks and the file can't be read back.
//! * A CSV export with no channels writes empty rows instead of failing with an index error.
//! * A run writes to `<file>.part` and renames over the target only when it succeeded. Upstream truncates the target the moment
//!   a run starts, so a cancel or a failure leaves a broken file behind, and the owner's `Export` folder goes straight to the Pi.

pub mod csv;
pub mod eseq;
pub mod fpp_config;
pub mod fseq;
pub mod fseq_read;
pub mod run;
pub mod vix2;

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Windows line ending. `Environment.NewLine` in the original, on the platform those exports were
/// made on, so files match what the user's other tools already accept.
pub(crate) const NEWLINE: &str = "\r\n";

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Format(String),
    #[error("the writer has no open session")]
    NotOpen,
}

pub type Result<T> = std::result::Result<T, ExportError>;

/// `SequenceSessionData`.
#[derive(Clone, Debug)]
pub struct SessionData {
    pub out_file_name: PathBuf,
    /// Number of intervals the run loop will write.
    pub num_periods: i32,
    pub period_ms: i32,
    /// One per output. Only the Vixen 2 and Helix formats write the names themselves.
    pub channel_names: Vec<String>,
    pub time_ms: f64,
    /// Full path of the audio, or empty.
    pub audio_file_name: String,
    /// Name stored in the FSEQ media header, or empty.
    pub output_audio_file_name: String,
}

impl Default for SessionData {
    fn default() -> Self {
        SessionData {
            out_file_name: PathBuf::new(),
            num_periods: 0,
            period_ms: 50,
            channel_names: Vec::new(),
            time_ms: 0.0,
            audio_file_name: String::new(),
            output_audio_file_name: String::new(),
        }
    }
}

/// `IExportWriter`.
pub trait ExportWriter: Send {
    fn open_session(&mut self, data: &SessionData) -> Result<()>;
    fn write_next_period(&mut self, period: &[u8]) -> Result<()>;
    fn close_session(&mut self) -> Result<()>;
}

/// The output formats, each one a concrete writer in the original.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ExportFormat {
    Csv,
    Eseq,
    FseqV2,
    FseqV1,
    Helix,
    Vir2,
    Vix2,
}

impl ExportFormat {
    /// Every format, in the order the original finds the writers.
    pub const ALL: [ExportFormat; 7] = [
        ExportFormat::Csv,
        ExportFormat::Eseq,
        ExportFormat::FseqV2,
        ExportFormat::FseqV1,
        ExportFormat::Helix,
        ExportFormat::Vir2,
        ExportFormat::Vix2,
    ];

    /// `Export.DefaultFormatType()`.
    pub const DEFAULT: ExportFormat = ExportFormat::FseqV2;

    /// `FileTypeDescr`: the name shown in the UI and saved in export profiles.
    pub fn description(self) -> &'static str {
        match self {
            ExportFormat::Csv => "CSV File",
            ExportFormat::Eseq => "Falcon Player Effect",
            ExportFormat::FseqV2 => "Falcon Player Sequence 2.6+",
            ExportFormat::FseqV1 => "Falcon Player Sequence",
            ExportFormat::Helix => "Helix File",
            ExportFormat::Vir2 => "Vixen 2 Routine",
            ExportFormat::Vix2 => "Vixen 2.1 Sequence",
        }
    }

    /// `FileType`: the file extension.
    pub fn file_type(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Eseq => "eseq",
            ExportFormat::FseqV2 | ExportFormat::FseqV1 => "fseq",
            ExportFormat::Helix | ExportFormat::Vix2 => "vix",
            ExportFormat::Vir2 => "vir",
        }
    }

    pub fn can_compress(self) -> bool {
        self == ExportFormat::FseqV2
    }

    /// `IsFalconFormat`. The older FSEQ writer doesn't set it in the original.
    pub fn is_falcon_format(self) -> bool {
        matches!(self, ExportFormat::FseqV2 | ExportFormat::Eseq)
    }

    /// `Version`.
    pub fn version(self) -> &'static str {
        if self == ExportFormat::FseqV2 {
            "2.6"
        } else {
            "1.0"
        }
    }

    pub fn from_description(description: &str) -> Option<ExportFormat> {
        ExportFormat::ALL.into_iter().find(|f| f.description() == description)
    }

    /// The test the original makes on the name, `Contains("Falcon")`, which unlike [`is_falcon_format`](Self::is_falcon_format)
    /// includes the older FSEQ writer.
    pub fn name_has_falcon(self) -> bool {
        self.description().contains("Falcon")
    }

    /// A new writer. `compress` only applies to formats that [`can_compress`](Self::can_compress);
    /// the others ignore it, as `DoExport` does.
    pub fn writer(self, compress: bool) -> Box<dyn ExportWriter> {
        match self {
            ExportFormat::Csv => Box::new(csv::CsvWriter::new()),
            ExportFormat::Eseq => Box::new(eseq::EseqWriter::new()),
            ExportFormat::FseqV2 => Box::new(fseq::FseqV2Writer::new(compress)),
            ExportFormat::FseqV1 => Box::new(fseq::FseqV1Writer::new()),
            ExportFormat::Helix | ExportFormat::Vix2 => Box::new(vix2::Vix2Writer::new()),
            ExportFormat::Vir2 => Box::new(vix2::Vir2Writer::new()),
        }
    }
}

/// The formats in the order the UI lists them: `FormatTypes` sorted by name. An index into this is what `Settings.xml` stores for
/// the export dialog's format.
pub const fn sorted_formats() -> [ExportFormat; 7] {
    [
        ExportFormat::Csv,
        ExportFormat::Eseq,
        ExportFormat::FseqV1,
        ExportFormat::FseqV2,
        ExportFormat::Helix,
        ExportFormat::Vir2,
        ExportFormat::Vix2,
    ]
}

/// Why a timing value can't be used.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IntervalError {
    #[error("Invalid timing interval {0}, check the value and try again.")]
    NotAnInteger(String),
    #[error("Invalid timing interval {0}, check the value and try again.")]
    OutOfRange(String),
}

/// The timing box's text, read as `Int32.TryParse` does: an optional sign and digits, with surrounding white space allowed.
/// GUARD: upstream divides by zero on 0, and truncates anything over 255 when it stores the step time in one byte, so a 300 ms
/// export declares 44 ms.
pub fn validate_interval(text: &str) -> std::result::Result<i32, IntervalError> {
    let value: i32 = text.trim().parse().map_err(|_| IntervalError::NotAnInteger(text.to_string()))?;
    if (1..=255).contains(&value) {
        Ok(value)
    } else {
        Err(IntervalError::OutOfRange(text.to_string()))
    }
}

/// What gets written next to the export, F9's last steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControllerFilePlan {
    None,
    /// `<dir>/<name>-co-universes.json`.
    Universes2x(PathBuf),
    /// `<dir>/<name>-universes`.
    Universes1x(PathBuf),
    /// `<dir>/<name>_Network.xml`.
    Network(PathBuf),
}

/// F9 steps 7 and 8. KEEP: this goes by the format's name, so the older FSEQ format writes a universe file although the dialog
/// has its radios off.
pub fn output_paths(format: ExportFormat, out_file: &Path, generate: bool, fpp2x: bool) -> ControllerFilePlan {
    if !generate {
        return ControllerFilePlan::None;
    }
    let dir = out_file.parent().unwrap_or(Path::new(""));
    let stem = out_file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if format.name_has_falcon() {
        if fpp2x {
            ControllerFilePlan::Universes2x(dir.join(format!("{stem}-co-universes.json")))
        } else {
            ControllerFilePlan::Universes1x(dir.join(format!("{stem}-universes")))
        }
    } else {
        ControllerFilePlan::Network(dir.join(format!("{stem}_Network.xml")))
    }
}

/// `Path.GetFileName`, accepting both separators because a sequence saved on Windows stores
/// Windows paths.
pub fn file_name_of(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// `Path.GetExtension`: includes the dot, empty when there is none.
pub fn extension_of(path: &str) -> &str {
    let name = file_name_of(path);
    match name.rfind('.') {
        Some(i) if i + 1 < name.len() => &name[i..],
        _ => "",
    }
}

/// `Export.FormatAudioFileName`.
pub fn format_audio_file_name(sequence_name: &str, audio_filename: &str) -> String {
    format!("{sequence_name}{}", extension_of(audio_filename))
}

/// The default export folder, `<data root>/Export`.
pub fn default_export_dir(data_root: &Path) -> PathBuf {
    data_root.join("Export")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_table_matches_the_writers() {
        assert_eq!(ExportFormat::DEFAULT.description(), "Falcon Player Sequence 2.6+");
        assert_eq!(ExportFormat::from_description("Vixen 2 Routine"), Some(ExportFormat::Vir2));
        assert_eq!(ExportFormat::from_description("nope"), None);
        assert!(ExportFormat::FseqV2.can_compress());
        assert!(!ExportFormat::FseqV1.can_compress());
        assert!(!ExportFormat::FseqV1.is_falcon_format());
        assert!(ExportFormat::Eseq.is_falcon_format());
        assert_eq!(ExportFormat::FseqV2.version(), "2.6");
        assert_eq!(ExportFormat::Csv.version(), "1.0");
    }

    #[test]
    fn sorted_formats_match_the_settings_index() {
        let names: Vec<&str> = sorted_formats().iter().map(|f| f.description()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert_eq!(
            names,
            ["CSV File", "Falcon Player Effect", "Falcon Player Sequence", "Falcon Player Sequence 2.6+", "Helix File", "Vixen 2 Routine", "Vixen 2.1 Sequence"]
        );
    }

    #[test]
    fn intervals_are_integers_from_1_to_255() {
        assert_eq!(validate_interval("25"), Ok(25));
        assert_eq!(validate_interval(" 50 "), Ok(50));
        assert_eq!(validate_interval("255"), Ok(255));
        assert_eq!(validate_interval("1"), Ok(1));
        assert_eq!(validate_interval("abc"), Err(IntervalError::NotAnInteger("abc".to_string())));
        assert_eq!(validate_interval("2.5"), Err(IntervalError::NotAnInteger("2.5".to_string())));
        assert_eq!(validate_interval("0"), Err(IntervalError::OutOfRange("0".to_string())));
        assert_eq!(validate_interval("300"), Err(IntervalError::OutOfRange("300".to_string())));
        assert_eq!(validate_interval("-5").unwrap_err().to_string(), "Invalid timing interval -5, check the value and try again.");
    }

    #[test]
    fn the_controller_file_goes_beside_the_export() {
        use ExportFormat::*;
        let out = Path::new("/show/Export/Song.fseq");
        assert_eq!(output_paths(FseqV2, out, false, true), ControllerFilePlan::None);
        assert_eq!(output_paths(FseqV2, out, true, true), ControllerFilePlan::Universes2x("/show/Export/Song-co-universes.json".into()));
        assert_eq!(output_paths(FseqV2, out, true, false), ControllerFilePlan::Universes1x("/show/Export/Song-universes".into()));
        // By name, not by the writer's flag.
        assert_eq!(output_paths(FseqV1, out, true, true), ControllerFilePlan::Universes2x("/show/Export/Song-co-universes.json".into()));
        assert_eq!(output_paths(Eseq, Path::new("/x/A.eseq"), true, false), ControllerFilePlan::Universes1x("/x/A-universes".into()));
        assert_eq!(output_paths(Csv, Path::new("/x/A.csv"), true, true), ControllerFilePlan::Network("/x/A_Network.xml".into()));
        assert_eq!(output_paths(Vix2, Path::new("/x/A.vix"), true, false), ControllerFilePlan::Network("/x/A_Network.xml".into()));
    }

    #[test]
    fn audio_names_follow_net_path_rules() {
        assert_eq!(file_name_of("C:\\Music\\a b.mp3"), "a b.mp3");
        assert_eq!(file_name_of("/x/y.wav"), "y.wav");
        assert_eq!(file_name_of("plain.ogg"), "plain.ogg");
        assert_eq!(extension_of("C:\\Music\\a.b.mp3"), ".mp3");
        assert_eq!(extension_of("/x.y/noext"), "");
        assert_eq!(extension_of("trailing."), "");
        assert_eq!(format_audio_file_name("Song", "/m/track.mp3"), "Song.mp3");
    }
}
