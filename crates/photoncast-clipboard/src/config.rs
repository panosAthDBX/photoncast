//! Clipboard configuration.
//!
//! Configuration options for clipboard history management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    DEFAULT_EXCLUDED_APPS, DEFAULT_HISTORY_SIZE, DEFAULT_MAX_IMAGE_SIZE, DEFAULT_RETENTION_DAYS,
};

/// Default action when selecting a clipboard item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultAction {
    /// Paste the item directly to the frontmost app.
    #[default]
    Paste,
    /// Copy the item to clipboard without pasting.
    Copy,
}

/// Clipboard history configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipboardConfig {
    /// Whether clipboard history is enabled.
    pub enabled: bool,

    /// Global hotkey for clipboard history (default: Cmd+Shift+V).
    pub hotkey: String,

    /// Maximum number of items to store.
    pub history_size: usize,

    /// Number of days to retain items.
    pub retention_days: u32,

    /// Whether to store images.
    pub store_images: bool,

    /// Maximum image size in bytes.
    pub max_image_size: u64,

    /// Bundle IDs of apps to exclude from clipboard history.
    pub excluded_apps: Vec<String>,

    /// Default action when selecting an item.
    pub default_action: DefaultAction,

    /// Custom storage directory (uses default if None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_dir: Option<PathBuf>,

    /// Polling interval in milliseconds (default: 250ms).
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,

    /// Store plaintext search text in the database for FTS (default: false).
    #[serde(default = "default_store_search_text")]
    pub store_search_text: bool,
}

const fn default_store_search_text() -> bool {
    false
}

const fn default_poll_interval() -> u64 {
    250
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hotkey: "Cmd+Shift+V".to_string(),
            history_size: DEFAULT_HISTORY_SIZE,
            retention_days: DEFAULT_RETENTION_DAYS,
            store_images: true,
            max_image_size: DEFAULT_MAX_IMAGE_SIZE,
            excluded_apps: DEFAULT_EXCLUDED_APPS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            default_action: DefaultAction::Paste,
            storage_dir: None,
            poll_interval_ms: default_poll_interval(),
            store_search_text: default_store_search_text(),
        }
    }
}

impl ClipboardConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the storage directory path.
    ///
    /// Uses the configured storage_dir if set, otherwise returns the default
    /// path at `~/Library/Application Support/PhotonCast/clipboard/`.
    #[must_use]
    pub fn storage_path(&self) -> PathBuf {
        self.storage_dir
            .clone()
            .unwrap_or_else(default_storage_path)
    }

    /// Returns the database file path.
    #[must_use]
    pub fn database_path(&self) -> PathBuf {
        self.storage_path().join("clipboard.db")
    }

    /// Returns the images directory path.
    #[must_use]
    pub fn images_path(&self) -> PathBuf {
        self.storage_path().join("images")
    }

    /// Returns the thumbnails directory path.
    #[must_use]
    pub fn thumbnails_path(&self) -> PathBuf {
        self.storage_path().join("thumbnails")
    }

    /// Checks if an app bundle ID is excluded.
    #[must_use]
    pub fn is_excluded(&self, bundle_id: &str) -> bool {
        self.excluded_apps.iter().any(|id| id == bundle_id)
    }

    /// Adds an app to the exclusion list.
    pub fn exclude_app(&mut self, bundle_id: impl Into<String>) {
        let bundle_id = bundle_id.into();
        if !self.is_excluded(&bundle_id) {
            self.excluded_apps.push(bundle_id);
        }
    }

    /// Removes an app from the exclusion list.
    pub fn unexclude_app(&mut self, bundle_id: &str) {
        self.excluded_apps.retain(|id| id != bundle_id);
    }

    /// Returns true if the default action is paste.
    #[must_use]
    pub const fn default_action_paste(&self) -> bool {
        matches!(self.default_action, DefaultAction::Paste)
    }

    /// Validates the configuration and returns any errors.
    pub fn validate(&self) -> Result<(), crate::error::ClipboardError> {
        if self.history_size == 0 {
            return Err(crate::error::ClipboardError::config(
                "history_size must be greater than 0",
            ));
        }
        if self.retention_days == 0 {
            return Err(crate::error::ClipboardError::config(
                "retention_days must be greater than 0",
            ));
        }
        if self.max_image_size == 0 {
            return Err(crate::error::ClipboardError::config(
                "max_image_size must be greater than 0",
            ));
        }
        if self.poll_interval_ms < 50 {
            return Err(crate::error::ClipboardError::config(
                "poll_interval_ms must be at least 50ms",
            ));
        }
        Ok(())
    }
}

/// Returns the default storage path for clipboard data.
#[must_use]
pub fn default_storage_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "PhotonCast").map_or_else(
        || {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Library/Application Support/PhotonCast/clipboard")
        },
        |dirs| dirs.data_dir().join("clipboard"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClipboardConfig::default();
        assert!(config.enabled);
        assert_eq!(config.history_size, DEFAULT_HISTORY_SIZE);
        assert_eq!(config.retention_days, DEFAULT_RETENTION_DAYS);
        assert_eq!(config.max_image_size, DEFAULT_MAX_IMAGE_SIZE);
        assert!(!config.excluded_apps.is_empty());
    }

    #[test]
    fn test_is_excluded() {
        let config = ClipboardConfig::default();
        assert!(config.is_excluded("com.1password.1password"));
        assert!(!config.is_excluded("com.apple.Safari"));
    }

    #[test]
    fn test_exclude_app() {
        let mut config = ClipboardConfig::default();
        config.exclude_app("com.test.App");
        assert!(config.is_excluded("com.test.App"));

        // Adding again should not duplicate
        let count_before = config.excluded_apps.len();
        config.exclude_app("com.test.App");
        assert_eq!(config.excluded_apps.len(), count_before);
    }

    #[test]
    fn test_unexclude_app() {
        let mut config = ClipboardConfig::default();
        assert!(config.is_excluded("com.1password.1password"));
        config.unexclude_app("com.1password.1password");
        assert!(!config.is_excluded("com.1password.1password"));
    }

    #[test]
    fn test_validate_config() {
        let config = ClipboardConfig::default();
        assert!(config.validate().is_ok());

        let invalid = ClipboardConfig {
            history_size: 0,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());

        let invalid = ClipboardConfig {
            retention_days: 0,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());

        let invalid = ClipboardConfig {
            poll_interval_ms: 10,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_storage_paths() {
        let config = ClipboardConfig::default();
        let db_path = config.database_path();
        assert!(db_path.ends_with("clipboard.db"));

        let images_path = config.images_path();
        assert!(images_path.ends_with("images"));

        let thumbs_path = config.thumbnails_path();
        assert!(thumbs_path.ends_with("thumbnails"));
    }

    #[test]
    fn test_serialization() {
        let config = ClipboardConfig::default();
        let json = serde_json::to_string(&config).expect("should serialize");
        let parsed: ClipboardConfig = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(parsed.history_size, config.history_size);
        assert_eq!(parsed.store_search_text, config.store_search_text);
    }
}
