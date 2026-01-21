use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    /// Generic error with a message.
    #[error("app error: {message}")]
    Message { message: String },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Plist parsing error.
    #[error("plist error: {0}")]
    Plist(String),

    /// App not found.
    #[error("app not found: {0}")]
    AppNotFound(String),

    /// System app protection error.
    #[error("cannot uninstall system app: {0}")]
    SystemAppProtection(String),

    /// Process error.
    #[error("process error: {0}")]
    Process(String),

    /// Permission denied.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}
