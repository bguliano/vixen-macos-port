use std::fmt;

/// Everything that can go wrong in the audio layer. Never panics; failures surface as values.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioError {
    Cancelled,
    Io(String),
    Unsupported(String),
    Decode(String),
    /// The requested output device does not exist.
    NoDevice(String),
    /// The device failed while opening or running (unplugged, format change, ...).
    Device(String),
    NoClip,
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::Cancelled => write!(f, "audio decoding was cancelled"),
            AudioError::Io(s) => write!(f, "i/o error: {s}"),
            AudioError::Unsupported(s) => write!(f, "unsupported: {s}"),
            AudioError::Decode(s) => write!(f, "decode error: {s}"),
            AudioError::NoDevice(s) => write!(f, "no such output device: {s}"),
            AudioError::Device(s) => write!(f, "audio device error: {s}"),
            AudioError::NoClip => write!(f, "no audio clip loaded"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<std::io::Error> for AudioError {
    fn from(e: std::io::Error) -> Self {
        AudioError::Io(e.to_string())
    }
}
