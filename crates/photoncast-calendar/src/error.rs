use thiserror::Error;

/// Result type for calendar operations.
pub type Result<T> = std::result::Result<T, CalendarError>;

/// Calendar-specific errors.
#[derive(Debug, Error)]
pub enum CalendarError {
    /// Generic error with message.
    #[error("calendar error: {message}")]
    Message { message: String },

    /// Permission denied error.
    #[error("calendar access permission denied")]
    PermissionDenied,

    /// EventKit error.
    #[error("EventKit error: {0}")]
    EventKit(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl CalendarError {
    /// Creates a new error with a message.
    #[must_use]
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}
