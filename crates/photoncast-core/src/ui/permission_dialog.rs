//! Accessibility permission dialog.
//!
//! This module provides the data structures and logic for the permission
//! request dialog shown to users when accessibility permission is required.

use crate::platform::accessibility::{
    check_accessibility_permission, open_accessibility_settings, request_accessibility_permission,
    PermissionPoller, PermissionStatus,
};
use std::sync::Arc;

/// User action result from the permission dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDialogResult {
    /// User clicked "Open System Settings".
    OpenSettings,
    /// User clicked "Skip for Now".
    Skip,
    /// Permission was granted (detected via polling).
    Granted,
    /// Dialog was dismissed without action.
    Dismissed,
}

/// Information displayed in the permission dialog explaining why access is needed.
#[derive(Debug, Clone)]
pub struct PermissionExplanation {
    /// Main title of the dialog.
    pub title: String,
    /// Subtitle or description.
    pub description: String,
    /// List of bullet points explaining what the permission enables.
    pub bullet_points: Vec<String>,
    /// Text for the primary action button.
    pub primary_button_text: String,
    /// Text for the secondary (skip) button.
    pub secondary_button_text: String,
    /// Additional note shown at the bottom.
    pub footer_note: Option<String>,
}

impl Default for PermissionExplanation {
    fn default() -> Self {
        Self {
            title: "Accessibility Permission Required".to_string(),
            description: "PhotonCast needs accessibility access to:".to_string(),
            bullet_points: vec![
                "Register global keyboard shortcuts".to_string(),
                "Respond to hotkey activation".to_string(),
            ],
            primary_button_text: "Open System Settings".to_string(),
            secondary_button_text: "Skip for Now".to_string(),
            footer_note: Some(
                "You can activate PhotonCast from the menu bar without this permission."
                    .to_string(),
            ),
        }
    }
}

/// Dialog for requesting accessibility permissions.
///
/// This struct maintains the state of the permission dialog and provides
/// methods to interact with it. The actual UI rendering depends on GPUI
/// integration, but the data structures and logic are defined here.
#[derive(Debug)]
pub struct PermissionDialog {
    /// Current permission status.
    pub status: PermissionStatus,
    /// Whether we're currently polling for permission changes.
    pub is_polling: bool,
    /// Whether the dialog is currently visible.
    pub is_visible: bool,
    /// The explanation content to display.
    pub explanation: PermissionExplanation,
    /// The poller for checking permission status.
    poller: Option<Arc<PermissionPoller>>,
}

impl Default for PermissionDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionDialog {
    /// Creates a new permission dialog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            status: PermissionStatus::Unknown,
            is_polling: false,
            is_visible: false,
            explanation: PermissionExplanation::default(),
            poller: None,
        }
    }

    /// Creates a permission dialog with an initial status.
    #[must_use]
    pub fn with_status(status: PermissionStatus) -> Self {
        Self {
            status,
            is_polling: false,
            is_visible: false,
            explanation: PermissionExplanation::default(),
            poller: None,
        }
    }

    /// Creates a permission dialog with custom explanation.
    #[must_use]
    pub fn with_explanation(explanation: PermissionExplanation) -> Self {
        Self {
            status: PermissionStatus::Unknown,
            is_polling: false,
            is_visible: false,
            explanation,
            poller: None,
        }
    }

    /// Returns true if permission has been granted.
    #[must_use]
    pub const fn is_granted(&self) -> bool {
        matches!(self.status, PermissionStatus::Granted)
    }

    /// Returns true if the dialog should be shown.
    ///
    /// The dialog should be shown when:
    /// - Permission is not granted
    /// - The dialog hasn't been explicitly hidden
    #[must_use]
    pub fn should_show(&self) -> bool {
        !self.is_granted() && self.is_visible
    }

    /// Updates the permission status by checking the current system state.
    pub fn refresh_status(&mut self) {
        let granted = check_accessibility_permission();
        self.status = if granted {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        };
    }

    /// Shows the dialog and starts polling for permission changes.
    pub fn show(&mut self) {
        self.refresh_status();
        self.is_visible = true;
    }

    /// Hides the dialog and stops polling.
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.stop_polling();
    }

    /// Handles the "Open System Settings" action.
    ///
    /// This opens the System Settings to the Accessibility pane and
    /// optionally starts polling for permission changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the system settings cannot be opened.
    pub fn handle_open_settings(&mut self) -> std::io::Result<PermissionDialogResult> {
        open_accessibility_settings()?;
        Ok(PermissionDialogResult::OpenSettings)
    }

    /// Handles the "Skip for Now" action.
    pub fn handle_skip(&mut self) -> PermissionDialogResult {
        self.hide();
        PermissionDialogResult::Skip
    }

    /// Requests permission with the system prompt.
    ///
    /// This will show the macOS permission prompt dialog.
    pub fn request_permission(&mut self) -> bool {
        let granted = request_accessibility_permission();
        self.status = if granted {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        };
        granted
    }

    /// Starts polling for permission changes.
    ///
    /// This will spawn an async task that periodically checks if permission
    /// has been granted. When granted, the callback is invoked.
    pub fn start_polling<F>(&mut self, on_granted: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let poller = Arc::new(PermissionPoller::new());
        self.poller = Some(Arc::clone(&poller));
        self.is_polling = true;

        // We can't call async functions directly here, so we'll need to
        // integrate with the tokio runtime externally.
        // The actual polling will be started by the UI layer.
        let _ = on_granted; // Placeholder - actual implementation needs async context
    }

    /// Stops polling for permission changes.
    pub fn stop_polling(&mut self) {
        if let Some(poller) = &self.poller {
            poller.stop_polling();
        }
        self.is_polling = false;
        self.poller = None;
    }

    /// Returns the poller for external async integration.
    #[must_use]
    pub fn get_poller(&self) -> Option<Arc<PermissionPoller>> {
        self.poller.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_dialog_new() {
        let dialog = PermissionDialog::new();
        assert_eq!(dialog.status, PermissionStatus::Unknown);
        assert!(!dialog.is_polling);
        assert!(!dialog.is_visible);
    }

    #[test]
    fn test_permission_dialog_with_status() {
        let dialog = PermissionDialog::with_status(PermissionStatus::Granted);
        assert_eq!(dialog.status, PermissionStatus::Granted);
        assert!(dialog.is_granted());
    }

    #[test]
    fn test_permission_dialog_is_granted() {
        let mut dialog = PermissionDialog::new();
        assert!(!dialog.is_granted());

        dialog.status = PermissionStatus::Granted;
        assert!(dialog.is_granted());

        dialog.status = PermissionStatus::Denied;
        assert!(!dialog.is_granted());
    }

    #[test]
    fn test_permission_dialog_should_show() {
        let mut dialog = PermissionDialog::new();
        dialog.status = PermissionStatus::Denied;

        // Not visible by default
        assert!(!dialog.should_show());

        // Show the dialog
        dialog.is_visible = true;
        assert!(dialog.should_show());

        // Grant permission
        dialog.status = PermissionStatus::Granted;
        assert!(!dialog.should_show());
    }

    #[test]
    fn test_permission_dialog_hide() {
        let mut dialog = PermissionDialog::new();
        dialog.is_visible = true;
        dialog.is_polling = true;

        dialog.hide();

        assert!(!dialog.is_visible);
        assert!(!dialog.is_polling);
    }

    #[test]
    fn test_permission_dialog_handle_skip() {
        let mut dialog = PermissionDialog::new();
        dialog.is_visible = true;

        let result = dialog.handle_skip();

        assert_eq!(result, PermissionDialogResult::Skip);
        assert!(!dialog.is_visible);
    }

    #[test]
    fn test_permission_explanation_default() {
        let explanation = PermissionExplanation::default();
        assert!(!explanation.title.is_empty());
        assert!(!explanation.bullet_points.is_empty());
        assert_eq!(explanation.bullet_points.len(), 2);
    }

    #[test]
    fn test_permission_dialog_result() {
        assert_eq!(
            PermissionDialogResult::OpenSettings,
            PermissionDialogResult::OpenSettings
        );
        assert_ne!(
            PermissionDialogResult::Skip,
            PermissionDialogResult::Granted
        );
    }
}
