#![recursion_limit = "1024"]
//! PhotonCast Clipboard History Library
//!
//! This crate provides clipboard history management for PhotonCast, including:
//!
//! - Encrypted SQLite storage with AES-256-GCM
//! - Full-text search via FTS5
//! - System clipboard monitoring via NSPasteboard
//! - App exclusion filtering (password managers)
//! - Image handling with thumbnails
//! - URL metadata fetching
//!
//! # Architecture
//!
//! The clipboard module uses a monitor-storage-encryption layered design:
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         ClipboardMonitor                │
//! │    (NSPasteboard polling, filtering)    │
//! └────────────────┬────────────────────────┘
//!                  │
//!                  ▼
//! ┌─────────────────────────────────────────┐
//! │         ClipboardStorage                │
//! │    (CRUD, search, retention policy)     │
//! └────────────────┬────────────────────────┘
//!                  │
//!                  ▼
//! ┌─────────────────────────────────────────┐
//! │        EncryptionManager                │
//! │   (AES-256-GCM, machine-derived key)    │
//! └────────────────┬────────────────────────┘
//!                  │
//!                  ▼
//! ┌─────────────────────────────────────────┐
//! │        SQLite + FTS5                    │
//! │    (encrypted content storage)          │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use photoncast_clipboard::{ClipboardConfig, ClipboardManager};
//!
//! // Create manager with default config
//! let config = ClipboardConfig::default();
//! let manager = ClipboardManager::new(config).await?;
//!
//! // Start monitoring
//! manager.start_monitoring().await?;
//!
//! // Search clipboard history
//! let results = manager.search("code snippet").await?;
//! ```

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::struct_excessive_bools)]

pub mod config;
pub mod encryption;
pub mod error;
pub mod models;
pub mod monitor;
pub mod storage;
pub mod url_metadata;

#[cfg(feature = "ui")]
pub mod ui;

pub use config::ClipboardConfig;
pub use error::{ClipboardError, Result};
pub use models::{ClipboardContentType, ClipboardItem, ClipboardItemId};
pub use monitor::ClipboardMonitor;
pub use storage::ClipboardStorage;

/// Default excluded apps (password managers).
pub const DEFAULT_EXCLUDED_APPS: &[&str] = &[
    "com.1password.1password",
    "com.agilebits.onepassword7",
    "com.bitwarden.desktop",
    "com.lastpass.LastPass",
    "com.apple.keychainaccess",
    "com.dashlane.Dashlane",
];

/// Default maximum image size in bytes (10MB).
pub const DEFAULT_MAX_IMAGE_SIZE: u64 = 10 * 1024 * 1024;

/// Default history size (number of items).
pub const DEFAULT_HISTORY_SIZE: usize = 1000;

/// Default retention period in days.
pub const DEFAULT_RETENTION_DAYS: u32 = 30;

/// Thumbnail size for images (max dimension).
pub const THUMBNAIL_SIZE: u32 = 200;

/// Preview text length for text content.
pub const PREVIEW_TEXT_LENGTH: usize = 100;
