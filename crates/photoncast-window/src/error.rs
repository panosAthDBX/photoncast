use thiserror::Error;

pub type Result<T> = std::result::Result<T, WindowError>;

/// Window management errors.
#[derive(Debug, Error)]
pub enum WindowError {
    /// Generic error message.
    #[error("window error: {message}")]
    Message { message: String },

    /// Accessibility permission not granted.
    #[error("accessibility permission required")]
    PermissionDenied,

    /// Window not found.
    #[error("window not found")]
    WindowNotFound,

    /// Display not found.
    #[error("display not found")]
    DisplayNotFound,

    /// Invalid frame dimensions.
    #[error("invalid frame dimensions: {reason}")]
    InvalidFrame { reason: String },

    /// Accessibility API error.
    #[error("accessibility API error: {message}")]
    AccessibilityError { message: String },

    /// Feature not available on this platform.
    #[error("feature not available on this platform")]
    PlatformNotSupported,
}
