//! Data models for app management.

use chrono::{DateTime, Utc};
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

/// Information about a currently running application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningApplication {
    /// Process ID of the running application.
    pub pid: u32,
    /// Bundle identifier of the application.
    pub bundle_id: String,
    /// Whether the application is responding to events.
    pub is_responding: bool,
    /// Whether the application is hidden.
    pub is_hidden: bool,
    /// When the application was launched.
    pub launch_time: DateTime<Utc>,
}

/// Auto quit settings for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AutoQuitSettings {
    /// Whether auto quit is enabled for this application.
    pub enabled: bool,
    /// Idle time in seconds before auto quit triggers.
    pub idle_seconds: Option<u64>,
}


/// Combined application information with its running state and settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationWithState {
    /// The application information.
    pub app: Application,
    /// Running state, if the application is currently running.
    pub running_state: Option<RunningApplication>,
    /// Auto quit settings for this application.
    pub auto_quit_settings: AutoQuitSettings,
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
    /// ~/Library/Cookies/<bundle-id>.binarycookies
    Cookies,
    /// ~/Library/WebKit/<bundle-id>
    WebKit,
    /// ~/Library/HTTPStorages/<bundle-id>
    HTTPStorages,
    /// ~/Library/Group Containers/<group-id>
    GroupContainers,
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
            Self::Cookies => "Cookies",
            Self::WebKit => "WebKit Data",
            Self::HTTPStorages => "HTTP Storages",
            Self::GroupContainers => "Group Containers",
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
    /// Whether this file is selected for deletion (defaults to true).
    pub selected: bool,
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
