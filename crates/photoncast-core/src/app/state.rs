//! Global application state.

use crate::app::config::Config;
use crate::platform::accessibility::{check_accessibility_permission, PermissionStatus};
#[cfg(feature = "ui")]
use crate::ui::permission_dialog::PermissionDialog;

/// Global application state for PhotonCast.
#[derive(Debug)]
pub struct AppState {
    /// Application configuration.
    pub config: Config,
    /// Whether the launcher window is visible.
    pub is_visible: bool,
    /// Whether the application is currently indexing.
    pub is_indexing: bool,
    /// Accessibility permission status.
    pub accessibility_status: PermissionStatus,
    /// Permission dialog state (only available with UI feature).
    #[cfg(feature = "ui")]
    pub permission_dialog: PermissionDialog,
    /// Whether the app has completed initial setup.
    pub is_initialized: bool,
}

impl AppState {
    /// Creates a new application state with the given configuration.
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            is_visible: false,
            is_indexing: false,
            accessibility_status: PermissionStatus::Unknown,
            #[cfg(feature = "ui")]
            permission_dialog: PermissionDialog::new(),
            is_initialized: false,
        }
    }

    /// Checks the current accessibility permission status.
    pub fn check_accessibility(&mut self) {
        let granted = check_accessibility_permission();
        self.accessibility_status = if granted {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        };
        #[cfg(feature = "ui")]
        {
            self.permission_dialog.status = self.accessibility_status;
        }
    }

    /// Returns true if accessibility permission is granted.
    #[must_use]
    pub fn has_accessibility_permission(&self) -> bool {
        matches!(self.accessibility_status, PermissionStatus::Granted)
    }

    /// Handles app startup permission flow.
    ///
    /// This should be called during app initialization to:
    /// 1. Check current permission status
    /// 2. Show permission dialog if not granted
    ///
    /// Returns `true` if permission is granted and hotkey registration can proceed.
    #[cfg(feature = "ui")]
    pub fn handle_startup_permission_check(&mut self) -> bool {
        self.check_accessibility();

        if self.has_accessibility_permission() {
            // Permission granted, no dialog needed
            self.permission_dialog.hide();
            true
        } else {
            // Permission not granted, show dialog
            self.permission_dialog.show();
            false
        }
    }

    /// Called when the permission dialog is closed.
    #[cfg(feature = "ui")]
    pub fn on_permission_dialog_closed(&mut self) {
        self.permission_dialog.hide();
        // Re-check permission in case it was granted
        self.check_accessibility();
    }

    /// Initializes the app state.
    ///
    /// This performs all startup checks and initializations.
    pub fn initialize(&mut self) {
        self.check_accessibility();
        self.is_initialized = true;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new() {
        let state = AppState::new(Config::default());
        assert!(!state.is_visible);
        assert!(!state.is_indexing);
        assert!(!state.is_initialized);
        assert_eq!(state.accessibility_status, PermissionStatus::Unknown);
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.is_visible);
        assert!(!state.is_initialized);
    }

    #[test]
    fn test_app_state_initialize() {
        let mut state = AppState::default();
        assert!(!state.is_initialized);

        state.initialize();

        assert!(state.is_initialized);
        // After initialization, status should be either Granted or Denied, not Unknown
        assert_ne!(state.accessibility_status, PermissionStatus::Unknown);
    }

    #[test]
    fn test_has_accessibility_permission() {
        let mut state = AppState::default();

        state.accessibility_status = PermissionStatus::Unknown;
        assert!(!state.has_accessibility_permission());

        state.accessibility_status = PermissionStatus::Denied;
        assert!(!state.has_accessibility_permission());

        state.accessibility_status = PermissionStatus::Granted;
        assert!(state.has_accessibility_permission());
    }

    #[cfg(feature = "ui")]
    #[test]
    fn test_on_permission_dialog_closed() {
        let mut state = AppState::default();
        state.permission_dialog.is_visible = true;
        state.permission_dialog.is_polling = true;

        state.on_permission_dialog_closed();

        assert!(!state.permission_dialog.is_visible);
        assert!(!state.permission_dialog.is_polling);
    }
}
