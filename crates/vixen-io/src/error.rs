//! The one error type every file operation in this crate returns.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    /// The operating system refused a read, write or rename.
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The file is not well-formed XML.
    #[error("{}: not valid XML: {message}", path.display())]
    Xml { path: PathBuf, message: String },
    /// The XML is well-formed but is not the document this crate expected.
    #[error("{context}: {message}")]
    Format { context: String, message: String },
    /// A versioned migration could not finish.
    #[error("cannot migrate {what} from version {from} to {to}: {message}")]
    Migration { what: String, from: i32, to: i32, message: String },
}

impl IoError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> IoError {
        IoError::Io { path: path.into(), source }
    }

    pub(crate) fn format(context: impl Into<String>, message: impl Into<String>) -> IoError {
        IoError::Format { context: context.into(), message: message.into() }
    }
}
