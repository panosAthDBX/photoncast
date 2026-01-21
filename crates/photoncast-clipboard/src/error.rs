//! Error types for clipboard operations.
//!
//! This module defines the error types used throughout the clipboard crate.
//! Uses `thiserror` for ergonomic error definitions.

use thiserror::Error;

/// Result type for clipboard operations.
pub type Result<T> = std::result::Result<T, ClipboardError>;

/// Errors that can occur during clipboard operations.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ClipboardError {
    /// Encryption/decryption failure.
    #[error("encryption error: {message}")]
    Encryption { message: String },

    /// Database operation failure.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// IO operation failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Image processing failure.
    #[error("image error: {message}")]
    Image { message: String },

    /// URL metadata fetch failure.
    #[error("URL metadata fetch failed: {message}")]
    UrlMetadata { message: String },

    /// Clipboard access failure.
    #[error("clipboard access error: {message}")]
    ClipboardAccess { message: String },

    /// Configuration error.
    #[error("configuration error: {message}")]
    Config { message: String },

    /// Item not found.
    #[error("clipboard item not found: {id}")]
    NotFound { id: String },

    /// Item too large (e.g., image exceeds max size).
    #[error("item too large: {size} bytes (max: {max} bytes)")]
    TooLarge { size: u64, max: u64 },

    /// Excluded app - item should not be stored.
    #[error("item from excluded app: {bundle_id}")]
    ExcludedApp { bundle_id: String },

    /// Transient item - should not be stored.
    #[error("transient clipboard item")]
    TransientItem,

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid content type.
    #[error("invalid content type: {type_name}")]
    InvalidContentType { type_name: String },

    /// Internal error.
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl ClipboardError {
    /// Creates a new encryption error.
    pub fn encryption(message: impl Into<String>) -> Self {
        Self::Encryption {
            message: message.into(),
        }
    }

    /// Creates a new image error.
    pub fn image(message: impl Into<String>) -> Self {
        Self::Image {
            message: message.into(),
        }
    }

    /// Creates a new URL metadata error.
    pub fn url_metadata(message: impl Into<String>) -> Self {
        Self::UrlMetadata {
            message: message.into(),
        }
    }

    /// Creates a new clipboard access error.
    pub fn clipboard_access(message: impl Into<String>) -> Self {
        Self::ClipboardAccess {
            message: message.into(),
        }
    }

    /// Creates a new config error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Creates a new not found error.
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound { id: id.into() }
    }

    /// Creates a new internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Returns true if this error indicates the item should not be stored.
    pub const fn is_skip_storage(&self) -> bool {
        matches!(
            self,
            Self::ExcludedApp { .. } | Self::TransientItem | Self::TooLarge { .. }
        )
    }

    /// Returns true if this error is recoverable (can retry).
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::UrlMetadata { .. } | Self::ClipboardAccess { .. }
        )
    }
}

impl From<image::ImageError> for ClipboardError {
    fn from(e: image::ImageError) -> Self {
        Self::Image {
            message: e.to_string(),
        }
    }
}

impl From<reqwest::Error> for ClipboardError {
    fn from(e: reqwest::Error) -> Self {
        Self::UrlMetadata {
            message: e.to_string(),
        }
    }
}

impl From<aes_gcm::Error> for ClipboardError {
    fn from(e: aes_gcm::Error) -> Self {
        Self::Encryption {
            message: e.to_string(),
        }
    }
}

impl From<argon2::Error> for ClipboardError {
    fn from(e: argon2::Error) -> Self {
        Self::Encryption {
            message: e.to_string(),
        }
    }
}

impl From<tokio::task::JoinError> for ClipboardError {
    fn from(e: tokio::task::JoinError) -> Self {
        Self::Internal {
            message: format!("task join error: {}", e),
        }
    }
}

impl From<anyhow::Error> for ClipboardError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal {
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ClipboardError::not_found("test-id");
        assert_eq!(err.to_string(), "clipboard item not found: test-id");

        let err = ClipboardError::TooLarge {
            size: 20_000_000,
            max: 10_000_000,
        };
        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn test_is_skip_storage() {
        assert!(ClipboardError::ExcludedApp {
            bundle_id: "test".into()
        }
        .is_skip_storage());
        assert!(ClipboardError::TransientItem.is_skip_storage());
        assert!(ClipboardError::TooLarge { size: 1, max: 0 }.is_skip_storage());
        assert!(!ClipboardError::encryption("test").is_skip_storage());
    }

    #[test]
    fn test_is_recoverable() {
        assert!(ClipboardError::url_metadata("timeout").is_recoverable());
        assert!(ClipboardError::clipboard_access("busy").is_recoverable());
        assert!(!ClipboardError::encryption("failed").is_recoverable());
    }
}
