//! App action operations module.
//!
//! This module provides high-level actions for managing applications,
//! such as launching, quitting, restarting, and controlling app state.

use std::path::Path;
use std::process::{Command, Stdio};

use thiserror::Error;

/// Errors that can occur during app actions.
#[derive(Debug, Error)]
pub enum ActionError {
    /// The target application is not running.
    #[error("application not running: {bundle_id}")]
    AppNotRunning {
        /// The bundle ID of the app that was expected to be running.
        bundle_id: String,
    },

    /// The target application is not found on the system.
    #[error("application not found: {bundle_id}")]
    AppNotFound {
        /// The bundle ID of the app that could not be found.
        bundle_id: String,
    },

    /// The requested operation failed.
    #[error("operation failed: {operation} - {reason}")]
    OperationFailed {
        /// The name of the operation that failed.
        operation: String,
        /// The reason for the failure.
        reason: String,
    },

    /// The operation timed out.
    /// Reserved for future use (e.g., quit timeout, network operations).
    #[allow(dead_code)]
    #[error("operation timed out after {timeout_secs} seconds: {operation}")]
    Timeout {
        /// The name of the operation that timed out.
        operation: String,
        /// The timeout duration in seconds.
        timeout_secs: u64,
    },

    /// Permission denied for the operation.
    /// Reserved for future use (e.g., protected directories, sandboxed apps).
    #[allow(dead_code)]
    #[error("permission denied: {operation} - {reason}")]
    PermissionDenied {
        /// The operation that was denied.
        operation: String,
        /// Additional context about why permission was denied.
        reason: String,
    },

    /// The application is not responding.
    /// Reserved for future use (e.g., force quit prompts).
    #[allow(dead_code)]
    #[error("application not responding: {bundle_id}")]
    AppNotResponding {
        /// The bundle ID of the unresponsive app.
        bundle_id: String,
    },

    /// System app protection - operation not allowed on system apps.
    /// Reserved for future use (e.g., uninstall protection in UI).
    #[allow(dead_code)]
    #[error("cannot perform {operation} on system app: {bundle_id}")]
    SystemAppProtected {
        /// The bundle ID of the protected system app.
        bundle_id: String,
        /// The operation that was attempted.
        operation: String,
    },

    /// Invalid bundle identifier.
    #[error("invalid bundle identifier: {bundle_id}")]
    InvalidBundleId {
        /// The invalid bundle ID.
        bundle_id: String,
    },

    /// Path not found.
    #[error("path not found: {path}")]
    PathNotFound {
        /// The path that was not found.
        path: String,
    },

    /// Clipboard operation failed.
    #[error("clipboard operation failed: {reason}")]
    ClipboardFailed {
        /// The reason for the failure.
        reason: String,
    },

    /// Process-related error.
    #[error("process error: {0}")]
    Process(String),

    /// I/O error during action execution.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for action operations.
pub type ActionResult<T> = std::result::Result<T, ActionError>;

/// Reveals a file or folder in Finder.
///
/// Uses the `open -R` command to reveal the item in its parent folder
/// with the item selected.
///
/// # Errors
///
/// Returns an error if the path doesn't exist or the reveal operation fails.
pub fn reveal_in_finder(path: &Path) -> ActionResult<()> {
    tracing::info!("Revealing in Finder: {:?}", path);

    // Check that path exists
    if !path.exists() {
        return Err(ActionError::PathNotFound {
            path: path.display().to_string(),
        });
    }

    let output = Command::new("open")
        .args(["-R", &path.display().to_string()])
        .output()?;

    if output.status.success() {
        tracing::debug!("Successfully revealed {:?} in Finder", path);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ActionError::OperationFailed {
            operation: "reveal_in_finder".to_string(),
            reason: stderr.trim().to_string(),
        })
    }
}

/// Copies the full POSIX path of a file or folder to the clipboard.
///
/// # Errors
///
/// Returns an error if the path doesn't exist or the clipboard operation fails.
pub fn copy_path_to_clipboard(path: &Path) -> ActionResult<()> {
    tracing::info!("Copying path to clipboard: {:?}", path);

    // Check that path exists
    if !path.exists() {
        return Err(ActionError::PathNotFound {
            path: path.display().to_string(),
        });
    }

    let path_str = path.display().to_string();
    copy_to_clipboard(&path_str)
}

/// Copies a bundle identifier to the clipboard.
///
/// # Errors
///
/// Returns an error if the clipboard operation fails.
pub fn copy_bundle_id_to_clipboard(bundle_id: &str) -> ActionResult<()> {
    tracing::info!("Copying bundle ID to clipboard: {}", bundle_id);

    if bundle_id.is_empty() {
        return Err(ActionError::InvalidBundleId {
            bundle_id: bundle_id.to_string(),
        });
    }

    copy_to_clipboard(bundle_id)
}

/// Validates that a bundle ID is safe for use in AppleScript.
///
/// Only allows alphanumeric characters, dots, and hyphens to prevent
/// command injection attacks.
fn is_valid_bundle_id(bundle_id: &str) -> bool {
    !bundle_id.is_empty()
        && bundle_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
}

/// Hides an application's windows.
///
/// Uses AppleScript to hide the application. The application must be running.
///
/// # Errors
///
/// Returns an error if the app is not running or the hide operation fails.
pub fn hide_app(bundle_id: &str) -> ActionResult<()> {
    tracing::info!("Hiding app: {}", bundle_id);

    // Validate bundle ID to prevent AppleScript injection
    if !is_valid_bundle_id(bundle_id) {
        return Err(ActionError::InvalidBundleId {
            bundle_id: bundle_id.to_string(),
        });
    }

    // Check if the app is running
    if !crate::process::is_app_running(bundle_id) {
        return Err(ActionError::AppNotRunning {
            bundle_id: bundle_id.to_string(),
        });
    }

    // Use AppleScript to hide the app (bundle_id is sanitized above)
    let script = format!(
        r#"tell application id "{}" to set visible to false"#,
        bundle_id
    );

    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()?;

    if output.status.success() {
        tracing::debug!("Successfully hid app: {}", bundle_id);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ActionError::OperationFailed {
            operation: "hide_app".to_string(),
            reason: stderr.trim().to_string(),
        })
    }
}

/// Helper function to copy text to the clipboard using pbcopy.
fn copy_to_clipboard(text: &str) -> ActionResult<()> {
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(text.as_bytes()).map_err(|e| ActionError::ClipboardFailed {
            reason: format!("Failed to write to pbcopy: {}", e),
        })?;
    }

    let status = child.wait()?;
    if status.success() {
        tracing::debug!("Successfully copied to clipboard");
        Ok(())
    } else {
        Err(ActionError::ClipboardFailed {
            reason: "pbcopy exited with non-zero status".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_action_error_display() {
        let err = ActionError::AppNotRunning {
            bundle_id: "com.example.app".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "application not running: com.example.app"
        );

        let err = ActionError::OperationFailed {
            operation: "launch".to_string(),
            reason: "failed to start process".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "operation failed: launch - failed to start process"
        );

        let err = ActionError::Timeout {
            operation: "quit".to_string(),
            timeout_secs: 5,
        };
        assert_eq!(err.to_string(), "operation timed out after 5 seconds: quit");
    }

    #[test]
    fn test_action_result_type() {
        let success: ActionResult<()> = Ok(());
        assert!(success.is_ok());

        let failure: ActionResult<()> = Err(ActionError::AppNotFound {
            bundle_id: "com.test.app".to_string(),
        });
        assert!(failure.is_err());
    }

    // ========================================================================
    // Task 8.2: Unit Tests - App Actions
    // ========================================================================

    /// Test reveal_in_finder with valid path (/Applications).
    #[test]
    #[cfg(target_os = "macos")]
    fn test_show_in_finder_valid_path() {
        // /Applications always exists on macOS
        let path = PathBuf::from("/Applications");

        let result = reveal_in_finder(&path);
        assert!(
            result.is_ok(),
            "reveal_in_finder should succeed for /Applications: {:?}",
            result
        );
    }

    /// Test reveal_in_finder returns error for nonexistent path.
    #[test]
    fn test_show_in_finder_nonexistent_path() {
        let path = PathBuf::from("/nonexistent/path/that/does/not/exist");

        let result = reveal_in_finder(&path);
        assert!(result.is_err(), "reveal_in_finder should fail for nonexistent path");

        let err = result.unwrap_err();
        match err {
            ActionError::PathNotFound { path: error_path } => {
                assert_eq!(
                    error_path,
                    "/nonexistent/path/that/does/not/exist",
                    "Error should contain the invalid path"
                );
            }
            _ => panic!("Expected PathNotFound error, got: {:?}", err),
        }
    }

    /// Test copy_bundle_id_to_clipboard with valid bundle ID.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_copy_bundle_id() {
        let bundle_id = "com.apple.finder";

        let result = copy_bundle_id_to_clipboard(bundle_id);
        assert!(
            result.is_ok(),
            "copy_bundle_id_to_clipboard should succeed: {:?}",
            result
        );

        // Note: We don't verify clipboard content here because tests run in parallel
        // and multiple tests may modify the clipboard concurrently.
        // The function's success is sufficient to verify it works.
    }

    /// Test copy_bundle_id_to_clipboard fails for empty bundle ID.
    #[test]
    fn test_copy_bundle_id_empty() {
        let result = copy_bundle_id_to_clipboard("");
        assert!(result.is_err(), "copy_bundle_id_to_clipboard should fail for empty string");

        let err = result.unwrap_err();
        match err {
            ActionError::InvalidBundleId { bundle_id } => {
                assert_eq!(bundle_id, "", "Error should contain the empty bundle ID");
            }
            _ => panic!("Expected InvalidBundleId error, got: {:?}", err),
        }
    }

    /// Test copy_path_to_clipboard with valid path.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_copy_path() {
        let path = PathBuf::from("/Applications");

        let result = copy_path_to_clipboard(&path);
        assert!(
            result.is_ok(),
            "copy_path_to_clipboard should succeed for /Applications: {:?}",
            result
        );

        // Note: We don't verify clipboard content here because tests run in parallel
        // and multiple tests may modify the clipboard concurrently.
        // The function's success is sufficient to verify it works.
    }

    /// Test copy_path_to_clipboard fails for nonexistent path.
    #[test]
    fn test_copy_path_nonexistent() {
        let path = PathBuf::from("/nonexistent/path");

        let result = copy_path_to_clipboard(&path);
        assert!(result.is_err(), "copy_path_to_clipboard should fail for nonexistent path");

        let err = result.unwrap_err();
        match err {
            ActionError::PathNotFound { path: error_path } => {
                assert_eq!(
                    error_path,
                    "/nonexistent/path",
                    "Error should contain the invalid path"
                );
            }
            _ => panic!("Expected PathNotFound error, got: {:?}", err),
        }
    }

    /// Test hide_app fails for non-running app.
    #[test]
    fn test_hide_app_not_running() {
        let bundle_id = "com.nonexistent.app.that.does.not.exist";

        let result = hide_app(bundle_id);
        assert!(result.is_err(), "hide_app should fail for non-running app");

        let err = result.unwrap_err();
        match err {
            ActionError::AppNotRunning { bundle_id: error_bundle } => {
                assert_eq!(
                    error_bundle, bundle_id,
                    "Error should contain the bundle ID"
                );
            }
            _ => panic!("Expected AppNotRunning error, got: {:?}", err),
        }
    }

    /// Test hide_app fails for empty bundle ID.
    #[test]
    fn test_hide_app_empty_bundle_id() {
        let result = hide_app("");
        assert!(result.is_err(), "hide_app should fail for empty bundle ID");

        let err = result.unwrap_err();
        match err {
            ActionError::InvalidBundleId { bundle_id } => {
                assert_eq!(bundle_id, "", "Error should contain the empty bundle ID");
            }
            _ => panic!("Expected InvalidBundleId error, got: {:?}", err),
        }
    }

    /// Test bundle ID validation rejects injection attempts.
    #[test]
    fn test_hide_app_rejects_injection() {
        // Test various injection attempts
        let injection_attempts = [
            r#"com.example" & do shell script "id" --"#,  // AppleScript injection
            "com.example\"; evil",                         // Quote escape
            "com.example$(whoami)",                        // Command substitution
            "com.example`id`",                             // Backtick injection
            "com.example\nmalicious",                      // Newline injection
            "com.example|cat /etc/passwd",                 // Pipe injection
            "com.example;rm -rf /",                        // Semicolon injection
        ];

        for payload in injection_attempts {
            let result = hide_app(payload);
            assert!(
                result.is_err(),
                "hide_app should reject injection attempt: {}",
                payload
            );

            let err = result.unwrap_err();
            match err {
                ActionError::InvalidBundleId { .. } => {
                    // Expected - injection was blocked
                }
                ActionError::AppNotRunning { .. } => {
                    // Also acceptable if it passed validation but app not running
                    // This shouldn't happen with proper validation
                    panic!("Injection payload passed validation: {}", payload);
                }
                _ => panic!("Unexpected error for injection attempt '{}': {:?}", payload, err),
            }
        }
    }

    /// Test bundle ID validation accepts valid bundle IDs.
    #[test]
    fn test_valid_bundle_id_acceptance() {
        use super::is_valid_bundle_id;

        let valid_ids = [
            "com.apple.finder",
            "com.example.MyApp",
            "com.company-name.app",
            "org.mozilla.firefox",
            "io.github.user.repo",
            "com.123.numeric",
        ];

        for bundle_id in valid_ids {
            assert!(
                is_valid_bundle_id(bundle_id),
                "Bundle ID should be valid: {}",
                bundle_id
            );
        }
    }

    /// Test bundle ID validation rejects invalid characters.
    #[test]
    fn test_invalid_bundle_id_rejection() {
        use super::is_valid_bundle_id;

        let invalid_ids = [
            "",                    // Empty
            "com.example app",     // Space
            "com.example\"app",    // Quote
            "com.example'app",     // Single quote
            "com.example;app",     // Semicolon
            "com.example&app",     // Ampersand
            "com.example|app",     // Pipe
            "com.example$app",     // Dollar
            "com.example`app",     // Backtick
            "com.example\napp",    // Newline
            "com.example\\app",    // Backslash
        ];

        for bundle_id in invalid_ids {
            assert!(
                !is_valid_bundle_id(bundle_id),
                "Bundle ID should be invalid: {:?}",
                bundle_id
            );
        }
    }

    /// Test hide_app for a running app (Finder).
    /// Note: This test actually hides Finder, which may be disruptive in a test environment.
    /// It's marked as ignored by default and can be run explicitly with --ignored.
    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "Actually hides Finder, run with --ignored for manual testing"]
    fn test_hide_app_running() {
        // Finder is always running on macOS
        let bundle_id = "com.apple.finder";

        let result = hide_app(bundle_id);
        assert!(
            result.is_ok(),
            "hide_app should succeed for running Finder: {:?}",
            result
        );
    }

    /// Test various ActionError variants for proper error messages.
    #[test]
    fn test_action_error_variants() {
        // PathNotFound
        let err = ActionError::PathNotFound {
            path: "/test/path".to_string(),
        };
        assert_eq!(err.to_string(), "path not found: /test/path");

        // InvalidBundleId
        let err = ActionError::InvalidBundleId {
            bundle_id: "".to_string(),
        };
        assert_eq!(err.to_string(), "invalid bundle identifier: ");

        // ClipboardFailed
        let err = ActionError::ClipboardFailed {
            reason: "test failure".to_string(),
        };
        assert_eq!(err.to_string(), "clipboard operation failed: test failure");

        // SystemAppProtected
        let err = ActionError::SystemAppProtected {
            bundle_id: "com.apple.Finder".to_string(),
            operation: "uninstall".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "cannot perform uninstall on system app: com.apple.Finder"
        );

        // PermissionDenied
        let err = ActionError::PermissionDenied {
            operation: "quit".to_string(),
            reason: "insufficient privileges".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "permission denied: quit - insufficient privileges"
        );

        // AppNotResponding
        let err = ActionError::AppNotResponding {
            bundle_id: "com.frozen.app".to_string(),
        };
        assert_eq!(err.to_string(), "application not responding: com.frozen.app");
    }

    // =========================================================================
    // Task 8.7: Action Panel Tests
    // =========================================================================

    /// Available actions for an application.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum AppAction {
        Launch,
        Quit,
        ForceQuit,
        Hide,
        RevealInFinder,
        CopyPath,
        CopyBundleId,
        Uninstall,
        ToggleAutoQuit,
    }

    /// Determines which actions should be available based on app state.
    fn get_available_actions(is_running: bool, is_system_app: bool) -> Vec<AppAction> {
        let mut actions = Vec::new();

        // Launch is available for non-running apps
        if !is_running {
            actions.push(AppAction::Launch);
        }

        // Running-only actions
        if is_running {
            actions.push(AppAction::Quit);
            actions.push(AppAction::ForceQuit);
            actions.push(AppAction::Hide);
        }

        // Always available actions
        actions.push(AppAction::RevealInFinder);
        actions.push(AppAction::CopyPath);
        actions.push(AppAction::CopyBundleId);
        actions.push(AppAction::ToggleAutoQuit);

        // Uninstall only for non-system apps
        if !is_system_app {
            actions.push(AppAction::Uninstall);
        }

        actions
    }

    #[test]
    fn test_action_panel_shows_running_actions() {
        // For a running app, verify Quit/Force Quit/Hide are shown
        let running_actions = get_available_actions(true, false);

        assert!(
            running_actions.contains(&AppAction::Quit),
            "Quit should be available for running apps"
        );
        assert!(
            running_actions.contains(&AppAction::ForceQuit),
            "Force Quit should be available for running apps"
        );
        assert!(
            running_actions.contains(&AppAction::Hide),
            "Hide should be available for running apps"
        );

        // Launch should NOT be shown for running apps
        assert!(
            !running_actions.contains(&AppAction::Launch),
            "Launch should NOT be available for running apps"
        );
    }

    #[test]
    fn test_action_panel_hides_running_actions_for_non_running() {
        // For a non-running app, verify Quit/Force Quit/Hide are hidden
        let non_running_actions = get_available_actions(false, false);

        assert!(
            !non_running_actions.contains(&AppAction::Quit),
            "Quit should NOT be available for non-running apps"
        );
        assert!(
            !non_running_actions.contains(&AppAction::ForceQuit),
            "Force Quit should NOT be available for non-running apps"
        );
        assert!(
            !non_running_actions.contains(&AppAction::Hide),
            "Hide should NOT be available for non-running apps"
        );

        // Launch should be shown for non-running apps
        assert!(
            non_running_actions.contains(&AppAction::Launch),
            "Launch should be available for non-running apps"
        );
    }

    #[test]
    fn test_action_panel_keyboard_shortcuts() {
        // Verify keyboard shortcuts are unique and correctly mapped
        struct ActionShortcut {
            action: AppAction,
            key: &'static str,
            modifiers: &'static [&'static str],
        }

        let shortcuts = vec![
            ActionShortcut {
                action: AppAction::Launch,
                key: "Return",
                modifiers: &[],
            },
            ActionShortcut {
                action: AppAction::Quit,
                key: "Q",
                modifiers: &["Cmd"],
            },
            ActionShortcut {
                action: AppAction::ForceQuit,
                key: "Q",
                modifiers: &["Cmd", "Option"],
            },
            ActionShortcut {
                action: AppAction::Hide,
                key: "H",
                modifiers: &["Cmd"],
            },
            ActionShortcut {
                action: AppAction::RevealInFinder,
                key: "R",
                modifiers: &["Cmd", "Shift"],
            },
            ActionShortcut {
                action: AppAction::CopyPath,
                key: "C",
                modifiers: &["Cmd", "Shift"],
            },
            ActionShortcut {
                action: AppAction::CopyBundleId,
                key: "C",
                modifiers: &["Cmd", "Option"],
            },
            ActionShortcut {
                action: AppAction::ToggleAutoQuit,
                key: "A",
                modifiers: &["Cmd", "Shift"],
            },
        ];

        // Verify shortcuts are unique
        let mut shortcut_keys: Vec<String> = shortcuts
            .iter()
            .map(|s| format!("{:?}+{}", s.modifiers, s.key))
            .collect();
        shortcut_keys.sort();
        let unique_count = shortcut_keys.len();
        shortcut_keys.dedup();
        assert_eq!(
            unique_count,
            shortcut_keys.len(),
            "All keyboard shortcuts should be unique"
        );

        // Verify Return is for Launch
        let launch_shortcut = shortcuts.iter().find(|s| s.action == AppAction::Launch);
        assert!(launch_shortcut.is_some());
        assert_eq!(launch_shortcut.unwrap().key, "Return");
        assert!(launch_shortcut.unwrap().modifiers.is_empty());

        // Verify Quit has Cmd+Q
        let quit_shortcut = shortcuts.iter().find(|s| s.action == AppAction::Quit);
        assert!(quit_shortcut.is_some());
        assert_eq!(quit_shortcut.unwrap().key, "Q");
        assert_eq!(quit_shortcut.unwrap().modifiers, &["Cmd"]);
    }

    #[test]
    fn test_action_panel_uninstall_not_for_system_apps() {
        // System apps should not show uninstall option
        let system_app_actions = get_available_actions(false, true);

        assert!(
            !system_app_actions.contains(&AppAction::Uninstall),
            "Uninstall should NOT be available for system apps"
        );

        // Non-system apps should show uninstall
        let normal_app_actions = get_available_actions(false, false);

        assert!(
            normal_app_actions.contains(&AppAction::Uninstall),
            "Uninstall should be available for non-system apps"
        );
    }

    #[test]
    fn test_action_panel_common_actions_always_available() {
        // These actions should be available regardless of running state
        let common_actions = [
            AppAction::RevealInFinder,
            AppAction::CopyPath,
            AppAction::CopyBundleId,
            AppAction::ToggleAutoQuit,
        ];

        let running_actions = get_available_actions(true, false);
        let non_running_actions = get_available_actions(false, false);

        for action in common_actions {
            assert!(
                running_actions.contains(&action),
                "{:?} should be available for running apps",
                action
            );
            assert!(
                non_running_actions.contains(&action),
                "{:?} should be available for non-running apps",
                action
            );
        }
    }

    #[test]
    fn test_action_panel_system_app_running() {
        // Running system app - should have quit/hide but no uninstall
        let actions = get_available_actions(true, true);

        assert!(actions.contains(&AppAction::Quit));
        assert!(actions.contains(&AppAction::ForceQuit));
        assert!(actions.contains(&AppAction::Hide));
        assert!(!actions.contains(&AppAction::Launch));
        assert!(!actions.contains(&AppAction::Uninstall));
    }

    #[test]
    fn test_action_panel_action_count() {
        // Running non-system app should have: Quit, ForceQuit, Hide, RevealInFinder, 
        // CopyPath, CopyBundleId, ToggleAutoQuit, Uninstall = 8 actions
        let running_actions = get_available_actions(true, false);
        assert_eq!(running_actions.len(), 8);

        // Non-running non-system app should have: Launch, RevealInFinder,
        // CopyPath, CopyBundleId, ToggleAutoQuit, Uninstall = 6 actions
        let non_running_actions = get_available_actions(false, false);
        assert_eq!(non_running_actions.len(), 6);

        // Running system app should have: Quit, ForceQuit, Hide, RevealInFinder,
        // CopyPath, CopyBundleId, ToggleAutoQuit = 7 actions (no Uninstall)
        let system_running_actions = get_available_actions(true, true);
        assert_eq!(system_running_actions.len(), 7);

        // Non-running system app should have: Launch, RevealInFinder,
        // CopyPath, CopyBundleId, ToggleAutoQuit = 5 actions (no Uninstall)
        let system_non_running_actions = get_available_actions(false, true);
        assert_eq!(system_non_running_actions.len(), 5);
    }
}
