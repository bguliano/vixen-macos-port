#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FppError {
    /// A bad argument, refused before anything is sent.
    #[error("{0}")]
    Argument(String),
    /// The device answered with something other than 2xx.
    #[error("{message}")]
    Status { message: String, status: u16 },
    /// The device answered 2xx with a body that isn't what the call needs.
    #[error("{0}")]
    Response(String),
    #[error("{0}")]
    Io(String),
    #[error("The operation was canceled.")]
    Cancelled,
    #[error("The device did not answer in time.")]
    TimedOut,
}

impl FppError {
    pub fn status(&self) -> Option<u16> {
        match self {
            FppError::Status { status, .. } => Some(*status),
            _ => None,
        }
    }
}
