//! The Templates folder: `PreviewTools.TemplateFolder` and the file handling in TemplateDialog.cs. A template
//! is a group saved as a `DisplayItem` XML file, with its name inside.

use crate::session::read_template;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A template file on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateEntry {
    /// The shape's name, or None if it has none.
    pub name: Option<String>,
    pub file: PathBuf,
}

/// `PreviewTools.TemplateFolder`: `Templates` under the preview module's data folder.
#[derive(Clone, Debug)]
pub struct TemplateStore {
    folder: PathBuf,
}

impl TemplateStore {
    /// `module_data` is the module's folder: `Module Data Files/VixenDisplayPreview`.
    pub fn new(module_data: &Path) -> TemplateStore {
        TemplateStore { folder: module_data.join("Templates") }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// `PopulateTemplateList`. A file that does not read as a template is skipped, with its error returned
    /// for the dialog the C# shows.
    pub fn list(&self) -> (Vec<TemplateEntry>, Vec<String>) {
        let mut entries = Vec::new();
        let mut errors = Vec::new();
        let Ok(dir) = fs::read_dir(&self.folder) else { return (entries, errors) };
        let mut files: Vec<PathBuf> = dir.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "xml")).collect();
        files.sort();
        for file in files {
            match fs::read_to_string(&file).ok().and_then(|text| read_template(&text)) {
                Some(shape) => entries.push(TemplateEntry { name: shape.name().map(str::to_string), file }),
                None => errors.push(format!("There was an error loading the template file ({})", file.display())),
            }
        }
        (entries, errors)
    }

    /// `CreateTemplate`'s `File.WriteAllText`. Returns the file's path. The name becomes the file name, as in the
    /// C#, so a name with a path separator in it is refused.
    pub fn save(&self, name: &str, xml: &str) -> io::Result<PathBuf> {
        if name.contains(['/', '\\']) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "a template name cannot contain a path separator"));
        }
        fs::create_dir_all(&self.folder)?;
        let file = self.folder.join(format!("{name}.xml"));
        fs::write(&file, xml)?;
        Ok(file)
    }

    /// `AddTtemplateToPreview`'s `File.ReadAllText`.
    pub fn read(&self, file: &Path) -> io::Result<String> {
        fs::read_to_string(file)
    }

    /// `buttonDeleteTemplate_Click`, once the user has confirmed.
    pub fn delete(&self, file: &Path) -> io::Result<()> {
        fs::remove_file(file)
    }
}

/// `PreviewCustomCreateForm`'s check: none of Windows' `Path.GetInvalidFileNameChars`, so templates stay portable if the show
/// folder moves between machines. That list covers the Mac's `/` and `:` too. Empty is allowed, as upstream, which writes `.xml`.
pub fn valid_template_name(name: &str) -> bool {
    !name.chars().any(|c| matches!(c, '"' | '<' | '>' | '|' | ':' | '*' | '?' | '\\' | '/') || (c as u32) < 32)
}
