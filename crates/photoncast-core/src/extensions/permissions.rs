//! Extension permissions management and consent tracking.
//!
//! This module handles:
//! - Checking if an extension requires permission consent
//! - Tracking accepted permissions per extension
//! - Generating permission descriptions for the consent dialog
//! - Detecting permission changes requiring re-consent

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::extensions::manifest::Permissions;
use crate::utils::paths;

/// Errors that can occur during permission operations.
#[derive(Debug, Error)]
pub enum PermissionsError {
    /// Failed to read permissions storage file.
    #[error("failed to read permissions storage: {0}")]
    ReadError(#[from] std::io::Error),

    /// Failed to parse permissions storage.
    #[error("failed to parse permissions storage: {0}")]
    ParseError(#[from] serde_json::Error),

    /// Permission consent required but not granted.
    #[error("permission consent required for extension {extension_id}")]
    ConsentRequired { extension_id: String },
}

/// A single permission type with its description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionItem {
    /// Unique identifier for this permission type.
    pub id: PermissionType,
    /// Human-readable name of the permission.
    pub name: String,
    /// Description of what this permission allows.
    pub description: String,
    /// SF Symbol icon name for this permission.
    pub icon: String,
}

/// Types of permissions an extension can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionType {
    /// Network access for HTTP requests.
    Network,
    /// Clipboard read/write access.
    Clipboard,
    /// System notification access.
    Notifications,
    /// Filesystem access (with paths).
    Filesystem,
}

impl PermissionType {
    /// Returns the SF Symbol icon name for this permission type.
    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Clipboard => "doc.on.clipboard",
            Self::Notifications => "bell.badge",
            Self::Filesystem => "folder",
        }
    }

    /// Returns the human-readable name of this permission type.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Network => "Network Access",
            Self::Clipboard => "Clipboard Access",
            Self::Notifications => "Notifications",
            Self::Filesystem => "File System Access",
        }
    }

    /// Returns the default description of this permission type.
    #[must_use]
    pub const fn default_description(&self) -> &'static str {
        match self {
            Self::Network => "Make network requests to the internet",
            Self::Clipboard => "Read and write clipboard contents",
            Self::Notifications => "Show system notifications",
            Self::Filesystem => "Access files on your system",
        }
    }
}

/// Information for displaying a permissions consent dialog.
#[derive(Debug, Clone)]
pub struct PermissionsDialog {
    /// Extension ID requesting permissions.
    pub extension_id: String,
    /// Extension display name.
    pub extension_name: String,
    /// Extension icon (SF Symbol name or emoji).
    pub extension_icon: Option<String>,
    /// List of permissions being requested.
    pub permissions: Vec<PermissionItem>,
    /// Header text for the dialog.
    pub header: String,
    /// Confirm button label.
    pub confirm_label: String,
    /// Cancel button label.
    pub cancel_label: String,
}

impl PermissionsDialog {
    /// Creates a new permissions dialog for the given extension and permissions.
    #[must_use]
    pub fn new(
        extension_id: impl Into<String>,
        extension_name: impl Into<String>,
        extension_icon: Option<String>,
        manifest_permissions: &Permissions,
    ) -> Self {
        let permissions = extract_permission_items(manifest_permissions);

        Self {
            extension_id: extension_id.into(),
            extension_name: extension_name.into(),
            extension_icon,
            permissions,
            header: "This extension requests access to:".to_string(),
            confirm_label: "Enable".to_string(),
            cancel_label: "Cancel".to_string(),
        }
    }

    /// Returns true if there are any permissions to display.
    #[must_use]
    pub fn has_permissions(&self) -> bool {
        !self.permissions.is_empty()
    }
}

/// Extracts permission items from a manifest's Permissions struct.
#[must_use]
pub fn extract_permission_items(permissions: &Permissions) -> Vec<PermissionItem> {
    let mut items = Vec::new();

    if permissions.network {
        items.push(PermissionItem {
            id: PermissionType::Network,
            name: PermissionType::Network.name().to_string(),
            description: PermissionType::Network.default_description().to_string(),
            icon: PermissionType::Network.icon().to_string(),
        });
    }

    if permissions.clipboard {
        items.push(PermissionItem {
            id: PermissionType::Clipboard,
            name: PermissionType::Clipboard.name().to_string(),
            description: PermissionType::Clipboard.default_description().to_string(),
            icon: PermissionType::Clipboard.icon().to_string(),
        });
    }

    if permissions.notifications {
        items.push(PermissionItem {
            id: PermissionType::Notifications,
            name: PermissionType::Notifications.name().to_string(),
            description: PermissionType::Notifications
                .default_description()
                .to_string(),
            icon: PermissionType::Notifications.icon().to_string(),
        });
    }

    if !permissions.filesystem.is_empty() {
        let paths_str = format_filesystem_paths(&permissions.filesystem);
        items.push(PermissionItem {
            id: PermissionType::Filesystem,
            name: PermissionType::Filesystem.name().to_string(),
            description: format!("Access files in: {paths_str}"),
            icon: PermissionType::Filesystem.icon().to_string(),
        });
    }

    items
}

/// Formats filesystem paths for display (e.g., "~/Documents, ~/Downloads").
#[allow(clippy::items_after_statements)]
fn format_filesystem_paths(paths: &[String]) -> String {
    if paths.is_empty() {
        return "none".to_string();
    }

    // Limit displayed paths to avoid overly long descriptions
    const MAX_DISPLAY_PATHS: usize = 3;

    let display_paths: Vec<&str> = paths
        .iter()
        .take(MAX_DISPLAY_PATHS)
        .map(std::string::String::as_str)
        .collect();

    if paths.len() > MAX_DISPLAY_PATHS {
        format!(
            "{} and {} more",
            display_paths.join(", "),
            paths.len() - MAX_DISPLAY_PATHS
        )
    } else {
        display_paths.join(", ")
    }
}

/// Record of accepted permissions for a single extension.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AcceptedPermissions {
    /// Whether the user has accepted the current permissions.
    pub accepted: bool,
    /// Hash of the permissions that were accepted (for change detection).
    pub permissions_hash: Option<String>,
    /// Timestamp when permissions were accepted.
    pub accepted_at: Option<i64>,
}

/// Storage for tracking permission acceptance across all extensions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionsStore {
    /// Map of extension ID to accepted permissions.
    #[serde(default)]
    pub extensions: HashMap<String, AcceptedPermissions>,
}

impl PermissionsStore {
    /// Loads the permissions store from disk.
    ///
    /// Returns default empty store if file doesn't exist.
    pub fn load() -> Result<Self, PermissionsError> {
        let path = Self::storage_path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)?;
        let store = serde_json::from_str(&contents)?;
        Ok(store)
    }

    /// Saves the permissions store to disk.
    pub fn save(&self) -> Result<(), PermissionsError> {
        let path = Self::storage_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Returns the path to the permissions storage file.
    #[must_use]
    pub fn storage_path() -> PathBuf {
        paths::data_dir().join("extension_permissions.json")
    }

    /// Checks if the given extension has accepted its current permissions.
    ///
    /// Returns true if:
    /// - Extension has no permissions (no consent needed)
    /// - Extension has accepted permissions that match current manifest
    #[must_use]
    pub fn has_valid_consent(&self, extension_id: &str, permissions: &Permissions) -> bool {
        // No permissions = no consent needed
        if !requires_consent(permissions) {
            return true;
        }

        let Some(accepted) = self.extensions.get(extension_id) else {
            return false;
        };

        if !accepted.accepted {
            return false;
        }

        // Check if permissions have changed since acceptance
        let current_hash = compute_permissions_hash(permissions);
        accepted.permissions_hash.as_ref() == Some(&current_hash)
    }

    /// Records that the user has accepted permissions for an extension.
    pub fn accept_permissions(&mut self, extension_id: &str, permissions: &Permissions) {
        let hash = compute_permissions_hash(permissions);
        let now = chrono::Utc::now().timestamp();

        self.extensions.insert(
            extension_id.to_string(),
            AcceptedPermissions {
                accepted: true,
                permissions_hash: Some(hash),
                accepted_at: Some(now),
            },
        );
    }

    /// Revokes permission acceptance for an extension.
    pub fn revoke_permissions(&mut self, extension_id: &str) {
        self.extensions.remove(extension_id);
    }

    /// Clears all stored permission acceptances.
    pub fn clear_all(&mut self) {
        self.extensions.clear();
    }
}

/// Returns true if the given permissions require user consent.
#[must_use]
pub fn requires_consent(permissions: &Permissions) -> bool {
    permissions.network
        || permissions.clipboard
        || permissions.notifications
        || !permissions.filesystem.is_empty()
}

/// Computes a hash of the permissions for change detection.
fn compute_permissions_hash(permissions: &Permissions) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    permissions.network.hash(&mut hasher);
    permissions.clipboard.hash(&mut hasher);
    permissions.notifications.hash(&mut hasher);

    // Sort filesystem paths for consistent hashing
    let mut paths = permissions.filesystem.clone();
    paths.sort();
    for path in &paths {
        path.hash(&mut hasher);
    }

    format!("{:x}", hasher.finish())
}

/// Checks if permissions have changed between old and new versions.
#[must_use]
pub fn permissions_changed(old: &Permissions, new: &Permissions) -> bool {
    compute_permissions_hash(old) != compute_permissions_hash(new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_consent_empty_permissions() {
        let permissions = Permissions::default();
        assert!(!requires_consent(&permissions));
    }

    #[test]
    fn test_requires_consent_with_network() {
        let permissions = Permissions {
            network: true,
            ..Default::default()
        };
        assert!(requires_consent(&permissions));
    }

    #[test]
    fn test_requires_consent_with_clipboard() {
        let permissions = Permissions {
            clipboard: true,
            ..Default::default()
        };
        assert!(requires_consent(&permissions));
    }

    #[test]
    fn test_requires_consent_with_notifications() {
        let permissions = Permissions {
            notifications: true,
            ..Default::default()
        };
        assert!(requires_consent(&permissions));
    }

    #[test]
    fn test_requires_consent_with_filesystem() {
        let permissions = Permissions {
            filesystem: vec!["~/Documents".to_string()],
            ..Default::default()
        };
        assert!(requires_consent(&permissions));
    }

    #[test]
    fn test_extract_permission_items_network() {
        let permissions = Permissions {
            network: true,
            ..Default::default()
        };
        let items = extract_permission_items(&permissions);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, PermissionType::Network);
        assert!(items[0].description.contains("network"));
    }

    #[test]
    fn test_extract_permission_items_all() {
        let permissions = Permissions {
            network: true,
            clipboard: true,
            notifications: true,
            filesystem: vec!["~/Documents".to_string()],
        };
        let items = extract_permission_items(&permissions);
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_format_filesystem_paths_single() {
        let paths = vec!["~/Documents".to_string()];
        assert_eq!(format_filesystem_paths(&paths), "~/Documents");
    }

    #[test]
    fn test_format_filesystem_paths_multiple() {
        let paths = vec!["~/Documents".to_string(), "~/Downloads".to_string()];
        assert_eq!(format_filesystem_paths(&paths), "~/Documents, ~/Downloads");
    }

    #[test]
    fn test_format_filesystem_paths_truncated() {
        let paths = vec![
            "~/Documents".to_string(),
            "~/Downloads".to_string(),
            "~/Desktop".to_string(),
            "~/Pictures".to_string(),
            "~/Movies".to_string(),
        ];
        let formatted = format_filesystem_paths(&paths);
        assert!(formatted.contains("and 2 more"));
    }

    #[test]
    fn test_permissions_hash_consistency() {
        let permissions = Permissions {
            network: true,
            clipboard: false,
            notifications: true,
            filesystem: vec!["~/Documents".to_string()],
        };

        let hash1 = compute_permissions_hash(&permissions);
        let hash2 = compute_permissions_hash(&permissions);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_permissions_hash_changes() {
        let permissions1 = Permissions {
            network: true,
            ..Default::default()
        };
        let permissions2 = Permissions {
            network: true,
            clipboard: true,
            ..Default::default()
        };

        let hash1 = compute_permissions_hash(&permissions1);
        let hash2 = compute_permissions_hash(&permissions2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_permissions_changed_same() {
        let permissions = Permissions {
            network: true,
            clipboard: true,
            ..Default::default()
        };
        assert!(!permissions_changed(&permissions, &permissions));
    }

    #[test]
    fn test_permissions_changed_different() {
        let old = Permissions {
            network: true,
            ..Default::default()
        };
        let new = Permissions {
            network: true,
            clipboard: true,
            ..Default::default()
        };
        assert!(permissions_changed(&old, &new));
    }

    #[test]
    fn test_permissions_store_valid_consent_no_permissions() {
        let store = PermissionsStore::default();
        let permissions = Permissions::default();
        assert!(store.has_valid_consent("com.example.test", &permissions));
    }

    #[test]
    fn test_permissions_store_valid_consent_not_accepted() {
        let store = PermissionsStore::default();
        let permissions = Permissions {
            network: true,
            ..Default::default()
        };
        assert!(!store.has_valid_consent("com.example.test", &permissions));
    }

    #[test]
    fn test_permissions_store_accept_and_check() {
        let mut store = PermissionsStore::default();
        let permissions = Permissions {
            network: true,
            clipboard: true,
            ..Default::default()
        };

        store.accept_permissions("com.example.test", &permissions);
        assert!(store.has_valid_consent("com.example.test", &permissions));
    }

    #[test]
    fn test_permissions_store_revoke() {
        let mut store = PermissionsStore::default();
        let permissions = Permissions {
            network: true,
            ..Default::default()
        };

        store.accept_permissions("com.example.test", &permissions);
        assert!(store.has_valid_consent("com.example.test", &permissions));

        store.revoke_permissions("com.example.test");
        assert!(!store.has_valid_consent("com.example.test", &permissions));
    }

    #[test]
    fn test_permissions_store_consent_invalid_after_change() {
        let mut store = PermissionsStore::default();
        let old_permissions = Permissions {
            network: true,
            ..Default::default()
        };
        let new_permissions = Permissions {
            network: true,
            clipboard: true,
            ..Default::default()
        };

        store.accept_permissions("com.example.test", &old_permissions);

        // Consent should be invalid after permissions change
        assert!(!store.has_valid_consent("com.example.test", &new_permissions));
    }

    #[test]
    fn test_permissions_dialog_creation() {
        let permissions = Permissions {
            network: true,
            clipboard: true,
            notifications: false,
            filesystem: vec!["~/Documents".to_string()],
        };

        let dialog = PermissionsDialog::new(
            "com.example.test",
            "Test Extension",
            Some("puzzlepiece".to_string()),
            &permissions,
        );

        assert_eq!(dialog.extension_id, "com.example.test");
        assert_eq!(dialog.extension_name, "Test Extension");
        assert!(dialog.has_permissions());
        assert_eq!(dialog.permissions.len(), 3);
    }

    #[test]
    fn test_permissions_dialog_no_permissions() {
        let permissions = Permissions::default();

        let dialog =
            PermissionsDialog::new("com.example.test", "Test Extension", None, &permissions);

        assert!(!dialog.has_permissions());
        assert!(dialog.permissions.is_empty());
    }
}
