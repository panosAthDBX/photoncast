//! Application indexing and scanning.
//!
//! This module handles discovering, parsing, and caching information about
//! installed applications.

pub mod icons;
pub mod metadata;
pub mod scanner;
pub mod watcher;

use std::path::PathBuf;

use chrono::{DateTime, Utc};

pub use icons::{default_cache_dir, extract_icon, IconCache};
pub use metadata::parse_app_metadata;
pub use scanner::AppScanner;
pub use watcher::{AppWatcher, FsWatcher, WatchEvent, WatcherConfig};

/// Unique identifier for an application bundle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppBundleId(String);

impl AppBundleId {
    /// Creates a new bundle ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the bundle ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AppBundleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An indexed application.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedApp {
    /// Display name of the application.
    pub name: String,
    /// Bundle identifier.
    pub bundle_id: AppBundleId,
    /// Path to the application bundle.
    pub path: PathBuf,
    /// Path to the cached icon.
    pub icon_path: Option<PathBuf>,
    /// Application category.
    pub category: Option<AppCategory>,
    /// Keywords for searching.
    pub keywords: Vec<String>,
    /// Last modification time of the bundle.
    pub last_modified: DateTime<Utc>,
}

/// Application category from Info.plist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCategory {
    /// Developer tools.
    DeveloperTools,
    /// Entertainment apps.
    Entertainment,
    /// Finance apps.
    Finance,
    /// Graphics and design.
    Graphics,
    /// Productivity apps.
    Productivity,
    /// Social networking.
    SocialNetworking,
    /// Utilities.
    Utilities,
    /// Other category.
    Other(String),
}

impl AppCategory {
    /// Parses a category from the Info.plist value.
    #[must_use]
    pub fn from_plist_value(value: &str) -> Self {
        match value {
            "public.app-category.developer-tools" => Self::DeveloperTools,
            "public.app-category.entertainment" => Self::Entertainment,
            "public.app-category.finance" => Self::Finance,
            "public.app-category.graphics-design" => Self::Graphics,
            "public.app-category.productivity" => Self::Productivity,
            "public.app-category.social-networking" => Self::SocialNetworking,
            "public.app-category.utilities" => Self::Utilities,
            other => Self::Other(other.to_string()),
        }
    }
}
