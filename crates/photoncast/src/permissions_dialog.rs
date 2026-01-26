//! Extension permissions consent dialog types.
//!
//! This module provides types for displaying and handling extension permission requests.
//! The actual rendering is done in the launcher module.

use photoncast_core::extensions::permissions::PermissionsDialog;

/// State for a pending permissions consent request.
#[derive(Debug, Clone)]
pub struct PendingPermissionsConsent {
    /// The permissions dialog information.
    pub dialog: PermissionsDialog,
    /// The command to execute after consent is granted (extension_id, command_id).
    pub pending_command: Option<(String, String)>,
    /// Whether this is from first-launch discovery (vs on-demand).
    pub is_first_launch: bool,
}
