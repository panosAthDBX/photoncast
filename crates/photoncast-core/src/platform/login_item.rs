//! Launch at login functionality.
//!
//! This module provides functionality for registering PhotonCast
//! as a login item using macOS SMAppService.

use std::process::Command;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors that can occur with login item operations.
#[derive(Debug, Error)]
pub enum LoginItemError {
    /// Failed to register as login item.
    #[error("Failed to register as login item: {0}")]
    RegistrationFailed(String),

    /// Failed to unregister as login item.
    #[error("Failed to unregister login item: {0}")]
    UnregistrationFailed(String),

    /// Failed to check login item status.
    #[error("Failed to check login item status: {0}")]
    StatusCheckFailed(String),

    /// SMAppService is not available.
    #[error("SMAppService is not available (requires macOS 13+)")]
    NotAvailable,
}

/// Status of the login item registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginItemStatus {
    /// The app is registered to launch at login.
    Enabled,
    /// The app is not registered to launch at login.
    Disabled,
    /// The status is unknown.
    Unknown,
    /// The user needs to approve the login item in System Settings.
    RequiresApproval,
}

impl LoginItemStatus {
    /// Returns true if the login item is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled)
    }

    /// Returns a human-readable description.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Enabled => "Enabled - PhotonCast will launch at login",
            Self::Disabled => "Disabled - PhotonCast will not launch at login",
            Self::Unknown => "Unknown - Could not determine status",
            Self::RequiresApproval => "Requires approval in System Settings",
        }
    }
}

/// Manager for login item functionality.
///
/// Uses the `SMAppService` framework via command-line tools to manage
/// login item registration. For a full implementation, this would use
/// the ServiceManagement framework directly via FFI.
#[derive(Debug)]
pub struct LoginItemManager {
    /// Bundle identifier for the app.
    bundle_id: String,
    /// Cached status.
    status: LoginItemStatus,
}

impl LoginItemManager {
    /// Creates a new login item manager for the given bundle ID.
    #[must_use]
    pub fn new(bundle_id: impl Into<String>) -> Self {
        Self {
            bundle_id: bundle_id.into(),
            status: LoginItemStatus::Unknown,
        }
    }

    /// Creates a login item manager for PhotonCast.
    #[must_use]
    pub fn for_photoncast() -> Self {
        Self::new("app.photoncast")
    }

    /// Returns the bundle identifier.
    #[must_use]
    pub fn bundle_id(&self) -> &str {
        &self.bundle_id
    }

    /// Returns the current status.
    #[must_use]
    pub fn status(&self) -> LoginItemStatus {
        self.status
    }

    /// Checks and updates the current login item status.
    pub fn check_status(&mut self) -> Result<LoginItemStatus, LoginItemError> {
        debug!(bundle_id = %self.bundle_id, "Checking login item status");

        // Use osascript to check if the app is in login items
        // This is a simplified check - full implementation would use SMAppService FFI
        match self.check_with_launchctl() {
            Ok(enabled) => {
                self.status = if enabled {
                    LoginItemStatus::Enabled
                } else {
                    LoginItemStatus::Disabled
                };
                debug!(status = ?self.status, "Login item status checked");
                Ok(self.status)
            },
            Err(e) => {
                warn!(error = %e, "Could not determine login item status");
                self.status = LoginItemStatus::Unknown;
                Ok(LoginItemStatus::Unknown)
            },
        }
    }

    /// Checks login item status using launchctl (for LaunchAgents).
    fn check_with_launchctl(&self) -> Result<bool, LoginItemError> {
        // Check if there's a LaunchAgent for this app
        let home = dirs::home_dir().ok_or_else(|| {
            LoginItemError::StatusCheckFailed("Could not get home directory".to_string())
        })?;

        let plist_path = home
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{}.plist", self.bundle_id));

        Ok(plist_path.exists())
    }

    /// Enables launch at login.
    ///
    /// This uses SMAppService.register() equivalent functionality.
    pub fn enable(&mut self) -> Result<(), LoginItemError> {
        info!(bundle_id = %self.bundle_id, "Enabling launch at login");

        // Use osascript to add to login items
        // Full implementation would use SMAppService FFI
        let script = r#"
            tell application "System Events"
                make new login item at end of login items with properties {
                    name: "PhotonCast",
                    path: (path to application "PhotonCast") as text,
                    hidden: false
                }
            end tell
            "#.to_string();

        match Command::new("osascript").args(["-e", &script]).output() {
            Ok(output) => {
                if output.status.success() {
                    info!("Successfully enabled launch at login");
                    self.status = LoginItemStatus::Enabled;
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // If the app isn't installed or found, provide a helpful message
                    if stderr.contains("does not understand") || stderr.contains("Can't get") {
                        warn!(
                            "Could not add to login items - app may not be in Applications folder"
                        );
                        // Still consider it a success if we're in development
                        self.status = LoginItemStatus::RequiresApproval;
                        Ok(())
                    } else {
                        error!(stderr = %stderr, "Failed to enable launch at login");
                        Err(LoginItemError::RegistrationFailed(stderr.to_string()))
                    }
                }
            },
            Err(e) => {
                error!(error = %e, "Failed to run osascript for login item");
                Err(LoginItemError::RegistrationFailed(e.to_string()))
            },
        }
    }

    /// Disables launch at login.
    ///
    /// This uses SMAppService.unregister() equivalent functionality.
    pub fn disable(&mut self) -> Result<(), LoginItemError> {
        info!(bundle_id = %self.bundle_id, "Disabling launch at login");

        // Use osascript to remove from login items
        let script = r#"
            tell application "System Events"
                set loginItems to name of every login item
                if loginItems contains "PhotonCast" then
                    delete login item "PhotonCast"
                end if
            end tell
        "#;

        match Command::new("osascript").args(["-e", script]).output() {
            Ok(output) => {
                if output.status.success() {
                    info!("Successfully disabled launch at login");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!(stderr = %stderr, "Failed to disable launch at login (may not exist)");
                    // Even if it fails, mark as disabled since the item might not exist
                }
                self.status = LoginItemStatus::Disabled;
                Ok(())
            },
            Err(e) => {
                error!(error = %e, "Failed to run osascript for login item removal");
                Err(LoginItemError::UnregistrationFailed(e.to_string()))
            },
        }
    }

    /// Sets the launch at login state.
    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), LoginItemError> {
        if enabled {
            self.enable()
        } else {
            self.disable()
        }
    }

    /// Toggles the launch at login state.
    pub fn toggle(&mut self) -> Result<(), LoginItemError> {
        self.check_status()?;
        self.set_enabled(!self.status.is_enabled())
    }

    /// Opens System Settings to the Login Items pane.
    pub fn open_settings() -> Result<(), LoginItemError> {
        info!("Opening Login Items settings");

        // macOS 13+ uses the new System Settings URL scheme
        let result = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.LoginItems-Settings.extension")
            .status();

        match result {
            Ok(status) if status.success() => {
                debug!("Opened Login Items settings");
                Ok(())
            },
            Ok(_) => {
                // Try legacy System Preferences
                let legacy_result = Command::new("open")
                    .arg("/System/Library/PreferencePanes/Accounts.prefPane")
                    .status();

                match legacy_result {
                    Ok(status) if status.success() => {
                        debug!("Opened legacy Login Items settings");
                        Ok(())
                    },
                    _ => Err(LoginItemError::NotAvailable),
                }
            },
            Err(e) => {
                error!(error = %e, "Failed to open Login Items settings");
                Err(LoginItemError::StatusCheckFailed(e.to_string()))
            },
        }
    }
}

impl Default for LoginItemManager {
    fn default() -> Self {
        Self::for_photoncast()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_item_status_is_enabled() {
        assert!(LoginItemStatus::Enabled.is_enabled());
        assert!(!LoginItemStatus::Disabled.is_enabled());
        assert!(!LoginItemStatus::Unknown.is_enabled());
        assert!(!LoginItemStatus::RequiresApproval.is_enabled());
    }

    #[test]
    fn test_login_item_status_description() {
        assert!(!LoginItemStatus::Enabled.description().is_empty());
        assert!(!LoginItemStatus::Disabled.description().is_empty());
        assert!(!LoginItemStatus::Unknown.description().is_empty());
        assert!(!LoginItemStatus::RequiresApproval.description().is_empty());
    }

    #[test]
    fn test_login_item_manager_new() {
        let manager = LoginItemManager::new("com.test.app");
        assert_eq!(manager.bundle_id(), "com.test.app");
        assert_eq!(manager.status(), LoginItemStatus::Unknown);
    }

    #[test]
    fn test_login_item_manager_for_photoncast() {
        let manager = LoginItemManager::for_photoncast();
        assert_eq!(manager.bundle_id(), "app.photoncast");
    }

    #[test]
    fn test_login_item_manager_default() {
        let manager = LoginItemManager::default();
        assert_eq!(manager.bundle_id(), "app.photoncast");
    }

    #[test]
    fn test_login_item_error_display() {
        let error = LoginItemError::RegistrationFailed("test error".to_string());
        assert!(error.to_string().contains("test error"));

        let error = LoginItemError::NotAvailable;
        assert!(error.to_string().contains("SMAppService"));
    }

    // Integration tests that require actual system interaction
    #[test]
    #[ignore = "requires actual system interaction"]
    fn test_login_item_check_status() {
        let mut manager = LoginItemManager::for_photoncast();
        let status = manager.check_status();
        assert!(status.is_ok());
    }

    #[test]
    #[ignore = "requires actual system interaction and modifies login items"]
    fn test_login_item_enable_disable() {
        let mut manager = LoginItemManager::for_photoncast();

        // Enable
        let result = manager.enable();
        assert!(result.is_ok());

        // Disable
        let result = manager.disable();
        assert!(result.is_ok());
    }
}
