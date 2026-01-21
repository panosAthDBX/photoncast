//! Data models for app management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Information about a macOS application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    /// Display name of the application.
    pub name: String,
    /// Bundle identifier (e.g., "com.apple.Safari").
    pub bundle_id: String,
    /// Path to the .app bundle.
    pub path: PathBuf,
    /// Application version.
    pub version: Option<String>,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Application icon path (if available).
    pub icon_path: Option<PathBuf>,
}

/// Category of related files for an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelatedFileCategory {
    /// ~/Library/Application Support/<App>
    ApplicationSupport,
    /// ~/Library/Preferences/<bundle-id>.plist
    Preferences,
    /// ~/Library/Caches/<bundle-id>
    Caches,
    /// ~/Library/Logs/<App>
    Logs,
    /// ~/Library/Saved Application State/<bundle-id>.savedState
    SavedState,
    /// ~/Library/Containers/<bundle-id>
    Containers,
}

impl RelatedFileCategory {
    /// Returns a human-readable name for this category.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::ApplicationSupport => "Application Support",
            Self::Preferences => "Preferences",
            Self::Caches => "Caches",
            Self::Logs => "Logs",
            Self::SavedState => "Saved Application State",
            Self::Containers => "Containers",
        }
    }
}

/// A related file or directory for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedFile {
    /// Path to the file or directory.
    pub path: PathBuf,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Category of this related file.
    pub category: RelatedFileCategory,
}

/// Preview of what will be uninstalled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallPreview {
    /// The application to uninstall.
    pub app: Application,
    /// Related files that will be removed.
    pub related_files: Vec<RelatedFile>,
    /// Total size of all items to be removed.
    pub total_size: u64,
    /// Formatted string of space to be freed.
    pub space_freed_formatted: String,
}

/// Information about a running application process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningApp {
    /// Process ID.
    pub pid: u32,
    /// Application name.
    pub name: String,
    /// Bundle identifier (if available).
    pub bundle_id: Option<String>,
    /// Whether the app is responding.
    pub is_responding: bool,
    /// Memory usage in bytes (if available).
    pub memory_bytes: Option<u64>,
    /// CPU usage percentage (if available).
    pub cpu_percent: Option<f32>,
}

impl UninstallPreview {
    /// Formats a byte count as a human-readable string.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
}
