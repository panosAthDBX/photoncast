//! Application launching via NSWorkspace.
//!
//! This module provides functionality to launch applications on macOS using
//! the `open` command, which interfaces with NSWorkspace under the hood.
//!
//! # Example
//!
//! ```no_run
//! use photoncast_core::platform::launch::{launch_app_by_bundle_id, reveal_in_finder};
//! use std::path::Path;
//!
//! // Launch Safari by bundle ID
//! if let Err(e) = launch_app_by_bundle_id("com.apple.Safari") {
//!     eprintln!("Failed to launch: {}", e.user_message());
//! }
//!
//! // Reveal a file in Finder
//! reveal_in_finder(Path::new("/Applications/Safari.app")).ok();
//! ```

use std::path::Path;
use std::process::Command;

use thiserror::Error;
use tracing::{debug, warn};

/// Errors that can occur when launching applications.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    /// The application was not found.
    #[error("application not found: {bundle_id}")]
    NotFound {
        /// The bundle ID that was not found.
        bundle_id: String,
    },

    /// Failed to launch the application.
    #[error("failed to launch '{app}': {reason}")]
    LaunchFailed {
        /// The application name or bundle ID.
        app: String,
        /// The reason for failure.
        reason: String,
    },

    /// The application is damaged.
    #[error("application is damaged and can't be opened: {app}")]
    Damaged {
        /// The application name or path.
        app: String,
    },
}

impl LaunchError {
    /// Returns a user-friendly error message.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::NotFound { bundle_id } => {
                format!("Application '{}' is no longer installed", bundle_id)
            },
            Self::LaunchFailed { app, reason } => {
                format!("Couldn't open {}: {}", app, reason)
            },
            Self::Damaged { app } => {
                format!("{} is damaged and can't be opened. Try reinstalling.", app)
            },
        }
    }

    /// Returns whether this error is recoverable.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::LaunchFailed { .. })
    }

    /// Returns a suggested action for the error.
    #[must_use]
    pub fn action_hint(&self) -> Option<&'static str> {
        match self {
            Self::NotFound { .. } => Some("Remove from index"),
            Self::Damaged { .. } => Some("Reveal in Finder"),
            Self::LaunchFailed { .. } => Some("Retry"),
        }
    }

    /// Returns whether this error suggests revealing the app in Finder.
    #[must_use]
    pub const fn should_offer_reveal(&self) -> bool {
        matches!(self, Self::Damaged { .. })
    }
}

/// Launches an application by bundle ID using the `open` command.
///
/// This uses `open -b <bundle_id>` which invokes NSWorkspace to find and
/// launch the application with the given bundle identifier.
///
/// # Arguments
///
/// * `bundle_id` - The bundle identifier (e.g., "com.apple.Safari")
///
/// # Errors
///
/// Returns an error if the application cannot be found or launched:
/// - [`LaunchError::NotFound`] - If no application with the bundle ID exists
/// - [`LaunchError::Damaged`] - If the application appears to be damaged
/// - [`LaunchError::LaunchFailed`] - For other launch failures
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::launch::launch_app_by_bundle_id;
///
/// match launch_app_by_bundle_id("com.apple.Safari") {
///     Ok(()) => println!("Safari launched successfully"),
///     Err(e) => eprintln!("Failed: {}", e.user_message()),
/// }
/// ```
pub fn launch_app_by_bundle_id(bundle_id: &str) -> Result<(), LaunchError> {
    debug!(bundle_id = %bundle_id, "launching application by bundle ID");

    let output = Command::new("open")
        .args(["-b", bundle_id])
        .output()
        .map_err(|e| {
            warn!(bundle_id = %bundle_id, error = %e, "failed to execute open command");
            LaunchError::LaunchFailed {
                app: bundle_id.to_string(),
                reason: e.to_string(),
            }
        })?;

    if output.status.success() {
        debug!(bundle_id = %bundle_id, "application launched successfully");
        return Ok(());
    }

    // Parse stderr for specific error types
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr_lower = stderr.to_lowercase();

    debug!(
        bundle_id = %bundle_id,
        stderr = %stderr,
        exit_code = ?output.status.code(),
        "open command failed"
    );

    // Check for "Unable to find application" error
    if stderr_lower.contains("unable to find application")
        || stderr_lower.contains("can't find application")
        || stderr_lower.contains("no application can open")
        || stderr_lower.contains("unable to find a bundle with identifier")
    {
        return Err(LaunchError::NotFound {
            bundle_id: bundle_id.to_string(),
        });
    }

    // Check for damaged app error
    if stderr_lower.contains("damaged")
        || stderr_lower.contains("can't be opened because")
        || stderr_lower.contains("cannot be opened because")
        || stderr_lower.contains("malware")
        || stderr_lower.contains("move it to the trash")
    {
        return Err(LaunchError::Damaged {
            app: bundle_id.to_string(),
        });
    }

    // Generic launch failure
    Err(LaunchError::LaunchFailed {
        app: bundle_id.to_string(),
        reason: if stderr.is_empty() {
            format!("exit code {}", output.status.code().unwrap_or(-1))
        } else {
            stderr.trim().to_string()
        },
    })
}

/// Launches an application by path.
///
/// # Errors
///
/// Returns an error if the application cannot be launched.
pub fn launch_app_by_path(path: &Path) -> Result<(), LaunchError> {
    debug!(path = %path.display(), "launching application by path");

    let output = Command::new("open").arg(path).output().map_err(|e| {
        warn!(path = %path.display(), error = %e, "failed to execute open command");
        LaunchError::LaunchFailed {
            app: path.display().to_string(),
            reason: e.to_string(),
        }
    })?;

    if output.status.success() {
        debug!(path = %path.display(), "application launched successfully");
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr_lower = stderr.to_lowercase();
    let app_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    // Check for damaged app error
    if stderr_lower.contains("damaged")
        || stderr_lower.contains("can't be opened because")
        || stderr_lower.contains("cannot be opened because")
    {
        return Err(LaunchError::Damaged { app: app_name });
    }

    // Check for not found
    if stderr_lower.contains("does not exist")
        || stderr_lower.contains("no such file")
        || !path.exists()
    {
        return Err(LaunchError::NotFound {
            bundle_id: app_name,
        });
    }

    Err(LaunchError::LaunchFailed {
        app: app_name,
        reason: if stderr.is_empty() {
            format!("exit code {}", output.status.code().unwrap_or(-1))
        } else {
            stderr.trim().to_string()
        },
    })
}

/// Opens a file with the default application.
///
/// # Errors
///
/// Returns an error if the file cannot be opened.
pub fn open_file(path: &Path) -> Result<(), LaunchError> {
    debug!(path = %path.display(), "opening file with default application");

    let output = Command::new("open").arg(path).output().map_err(|e| {
        warn!(path = %path.display(), error = %e, "failed to open file");
        LaunchError::LaunchFailed {
            app: path.display().to_string(),
            reason: e.to_string(),
        }
    })?;

    if output.status.success() {
        debug!(path = %path.display(), "file opened successfully");
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    Err(LaunchError::LaunchFailed {
        app: path.display().to_string(),
        reason: if stderr.is_empty() {
            format!("exit code {}", output.status.code().unwrap_or(-1))
        } else {
            stderr.trim().to_string()
        },
    })
}

/// Reveals a file in Finder.
///
/// This opens Finder and selects the specified file or directory.
///
/// # Errors
///
/// Returns an error if Finder cannot be opened.
pub fn reveal_in_finder(path: &Path) -> Result<(), LaunchError> {
    debug!(path = %path.display(), "revealing in Finder");

    let output = Command::new("open")
        .args(["-R", &path.display().to_string()])
        .output()
        .map_err(|e| {
            warn!(path = %path.display(), error = %e, "failed to reveal in Finder");
            LaunchError::LaunchFailed {
                app: "Finder".to_string(),
                reason: e.to_string(),
            }
        })?;

    if output.status.success() {
        debug!(path = %path.display(), "revealed in Finder successfully");
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    Err(LaunchError::LaunchFailed {
        app: "Finder".to_string(),
        reason: if stderr.is_empty() {
            format!("exit code {}", output.status.code().unwrap_or(-1))
        } else {
            stderr.trim().to_string()
        },
    })
}

// =============================================================================
// AppLauncher - High-level launcher with usage tracking
// =============================================================================

use std::sync::Arc;

use crate::search::SearchAction;
use crate::storage::UsageTracker;

/// High-level application launcher with usage tracking.
///
/// This struct wraps the low-level launch functions and integrates
/// with the usage tracking system for frecency-based ranking.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::launch::AppLauncher;
/// use photoncast_core::storage::{Database, UsageTracker};
///
/// let db = Database::open_in_memory().unwrap();
/// let tracker = UsageTracker::new(db);
/// let launcher = AppLauncher::new(tracker);
///
/// // Launch an app with usage tracking
/// launcher.launch_by_bundle_id("com.apple.Safari").ok();
/// ```
pub struct AppLauncher {
    usage_tracker: Arc<UsageTracker>,
}

impl AppLauncher {
    /// Creates a new app launcher with usage tracking.
    #[must_use]
    pub fn new(usage_tracker: UsageTracker) -> Self {
        Self {
            usage_tracker: Arc::new(usage_tracker),
        }
    }

    /// Creates a new app launcher from a shared usage tracker.
    #[must_use]
    pub fn with_shared_tracker(usage_tracker: Arc<UsageTracker>) -> Self {
        Self { usage_tracker }
    }

    /// Launches an application by bundle ID and tracks usage.
    ///
    /// On successful launch, increments the launch count and updates
    /// the last launched timestamp for frecency ranking.
    ///
    /// # Errors
    ///
    /// Returns a `LaunchError` if the application cannot be launched.
    pub fn launch_by_bundle_id(&self, bundle_id: &str) -> Result<(), LaunchError> {
        // Attempt to launch
        launch_app_by_bundle_id(bundle_id)?;

        // Track usage on success
        if let Err(e) = self.usage_tracker.record_app_launch(bundle_id) {
            warn!(
                bundle_id = %bundle_id,
                error = %e,
                "failed to record app launch in usage tracker"
            );
            // Don't fail the launch just because tracking failed
        }

        Ok(())
    }

    /// Launches an application by bundle ID asynchronously and tracks usage.
    ///
    /// # Errors
    ///
    /// Returns a `LaunchError` if the application cannot be launched.
    pub async fn launch_by_bundle_id_async(&self, bundle_id: String) -> Result<(), LaunchError> {
        let bid = bundle_id.clone();

        // Launch in blocking task since we use `Command`
        let result = tokio::task::spawn_blocking(move || launch_app_by_bundle_id(&bid)).await;

        match result {
            Ok(Ok(())) => {
                // Track usage on success
                if let Err(e) = self
                    .usage_tracker
                    .record_app_launch_async(bundle_id.clone())
                    .await
                {
                    warn!(
                        bundle_id = %bundle_id,
                        error = %e,
                        "failed to record app launch in usage tracker"
                    );
                }
                Ok(())
            },
            Ok(Err(e)) => Err(e),
            Err(e) => Err(LaunchError::LaunchFailed {
                app: bundle_id,
                reason: format!("task join error: {e}"),
            }),
        }
    }

    /// Launches an application by path and tracks usage.
    ///
    /// Attempts to extract the bundle ID from the app path for tracking.
    ///
    /// # Errors
    ///
    /// Returns a `LaunchError` if the application cannot be launched.
    pub fn launch_by_path(&self, path: &Path) -> Result<(), LaunchError> {
        launch_app_by_path(path)?;

        // Try to extract bundle ID for tracking
        if let Some(bundle_id) = extract_bundle_id_from_path(path) {
            if let Err(e) = self.usage_tracker.record_app_launch(&bundle_id) {
                warn!(
                    path = %path.display(),
                    bundle_id = %bundle_id,
                    error = %e,
                    "failed to record app launch in usage tracker"
                );
            }
        }

        Ok(())
    }

    /// Executes a search action.
    ///
    /// Handles `LaunchApp`, `OpenFile`, and `RevealInFinder` actions.
    ///
    /// # Errors
    ///
    /// Returns a `LaunchError` if the action cannot be executed.
    pub fn execute_action(&self, action: &SearchAction) -> Result<(), LaunchError> {
        match action {
            SearchAction::LaunchApp { bundle_id, path: _ } => self.launch_by_bundle_id(bundle_id),
            SearchAction::OpenFile { path } => {
                open_file(path)?;

                // Track file usage
                if let Err(e) = self
                    .usage_tracker
                    .record_file_open(&path.display().to_string())
                {
                    warn!(
                        path = %path.display(),
                        error = %e,
                        "failed to record file open in usage tracker"
                    );
                }
                Ok(())
            },
            SearchAction::RevealInFinder { path } => reveal_in_finder(path),
            SearchAction::ExecuteCommand { command_id } => {
                // Commands are handled by the command executor, not the app launcher
                debug!(command_id = %command_id, "ignoring command action in AppLauncher");
                Ok(())
            },
            SearchAction::EnterFileSearchMode => {
                // File search mode is handled by the UI, not the app launcher
                debug!("ignoring EnterFileSearchMode action in AppLauncher");
                Ok(())
            },
            SearchAction::QuickLookFile { path } => {
                // Quick Look is handled separately
                debug!(path = %path.display(), "ignoring QuickLookFile action in AppLauncher");
                Ok(())
            },
        }
    }

    /// Executes a search action asynchronously.
    ///
    /// # Errors
    ///
    /// Returns a `LaunchError` if the action cannot be executed.
    pub async fn execute_action_async(&self, action: SearchAction) -> Result<(), LaunchError> {
        match action {
            SearchAction::LaunchApp { bundle_id, path: _ } => {
                self.launch_by_bundle_id_async(bundle_id).await
            },
            SearchAction::OpenFile { path } => {
                let path_str = path.display().to_string();
                let path_clone = path.clone();

                let result = tokio::task::spawn_blocking(move || open_file(&path_clone)).await;

                match result {
                    Ok(Ok(())) => {
                        if let Err(e) = self.usage_tracker.record_file_open_async(path_str).await {
                            warn!(error = %e, "failed to record file open");
                        }
                        Ok(())
                    },
                    Ok(Err(e)) => Err(e),
                    Err(e) => Err(LaunchError::LaunchFailed {
                        app: path.display().to_string(),
                        reason: format!("task join error: {e}"),
                    }),
                }
            },
            SearchAction::RevealInFinder { path } => {
                tokio::task::spawn_blocking(move || reveal_in_finder(&path))
                    .await
                    .map_err(|e| LaunchError::LaunchFailed {
                        app: "Finder".to_string(),
                        reason: format!("task join error: {e}"),
                    })?
            },
            SearchAction::ExecuteCommand { command_id } => {
                debug!(command_id = %command_id, "ignoring command action in AppLauncher");
                Ok(())
            },
            SearchAction::EnterFileSearchMode => {
                // File search mode is handled by the UI, not the app launcher
                debug!("ignoring EnterFileSearchMode action in AppLauncher");
                Ok(())
            },
            SearchAction::QuickLookFile { path } => {
                // Quick Look is handled separately
                debug!(path = %path.display(), "ignoring QuickLookFile action in AppLauncher");
                Ok(())
            },
        }
    }
}

/// Attempts to extract a bundle ID from an app path by reading Info.plist.
fn extract_bundle_id_from_path(path: &Path) -> Option<String> {
    let info_plist = path.join("Contents/Info.plist");

    if !info_plist.exists() {
        return None;
    }

    // Try to read and parse the plist
    let contents = std::fs::read(&info_plist).ok()?;
    let plist: plist::Value = plist::from_bytes(&contents).ok()?;
    let dict = plist.as_dictionary()?;

    dict.get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(String::from)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // LaunchError Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_launch_error_not_found_user_message() {
        let error = LaunchError::NotFound {
            bundle_id: "com.example.missing".to_string(),
        };
        assert!(error.user_message().contains("no longer installed"));
        assert!(error.user_message().contains("com.example.missing"));
    }

    #[test]
    fn test_launch_error_damaged_user_message() {
        let error = LaunchError::Damaged {
            app: "TestApp".to_string(),
        };
        assert!(error.user_message().contains("damaged"));
        assert!(error.user_message().contains("reinstalling"));
    }

    #[test]
    fn test_launch_error_launch_failed_user_message() {
        let error = LaunchError::LaunchFailed {
            app: "TestApp".to_string(),
            reason: "permission denied".to_string(),
        };
        assert!(error.user_message().contains("Couldn't open"));
        assert!(error.user_message().contains("permission denied"));
    }

    #[test]
    fn test_launch_error_is_recoverable() {
        assert!(!LaunchError::NotFound {
            bundle_id: "x".to_string()
        }
        .is_recoverable());
        assert!(!LaunchError::Damaged {
            app: "x".to_string()
        }
        .is_recoverable());
        assert!(LaunchError::LaunchFailed {
            app: "x".to_string(),
            reason: "y".to_string()
        }
        .is_recoverable());
    }

    #[test]
    fn test_launch_error_action_hints() {
        assert_eq!(
            LaunchError::NotFound {
                bundle_id: "x".to_string()
            }
            .action_hint(),
            Some("Remove from index")
        );
        assert_eq!(
            LaunchError::Damaged {
                app: "x".to_string()
            }
            .action_hint(),
            Some("Reveal in Finder")
        );
        assert_eq!(
            LaunchError::LaunchFailed {
                app: "x".to_string(),
                reason: "y".to_string()
            }
            .action_hint(),
            Some("Retry")
        );
    }

    #[test]
    fn test_launch_error_should_offer_reveal() {
        assert!(!LaunchError::NotFound {
            bundle_id: "x".to_string()
        }
        .should_offer_reveal());
        assert!(LaunchError::Damaged {
            app: "x".to_string()
        }
        .should_offer_reveal());
        assert!(!LaunchError::LaunchFailed {
            app: "x".to_string(),
            reason: "y".to_string()
        }
        .should_offer_reveal());
    }

    // -------------------------------------------------------------------------
    // Launch Function Tests (require actual apps on macOS)
    // -------------------------------------------------------------------------

    #[test]
    fn test_launch_nonexistent_bundle_id() {
        let result = launch_app_by_bundle_id("com.nonexistent.app.that.does.not.exist.12345");
        assert!(result.is_err());
        // macOS may return either NotFound or LaunchFailed depending on how the error is detected
        match result.unwrap_err() {
            LaunchError::NotFound { bundle_id } => {
                assert!(bundle_id.contains("nonexistent"));
            },
            LaunchError::LaunchFailed { app, reason: _ } => {
                assert!(app.contains("nonexistent"));
            },
            other => panic!("expected NotFound or LaunchFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_launch_nonexistent_path() {
        let result = launch_app_by_path(Path::new("/Applications/NonExistentApp12345.app"));
        assert!(result.is_err());
    }

    #[test]
    fn test_reveal_nonexistent_path() {
        // This should still succeed on macOS - Finder handles it gracefully
        // or it might fail depending on the macOS version
        let result = reveal_in_finder(Path::new("/tmp/nonexistent_file_for_photoncast_test.txt"));
        // We just verify it doesn't panic
        let _ = result;
    }

    // -------------------------------------------------------------------------
    // Integration Tests (skip in CI)
    // -------------------------------------------------------------------------

    #[test]
    #[ignore = "requires actual app to be installed"]
    fn test_launch_safari() {
        let result = launch_app_by_bundle_id("com.apple.Safari");
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires actual app to be installed"]
    fn test_launch_finder_by_path() {
        let result = launch_app_by_path(Path::new("/System/Library/CoreServices/Finder.app"));
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // AppLauncher Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_app_launcher_launch_nonexistent() {
        use crate::storage::Database;

        let db = Database::open_in_memory().expect("should open database");
        let tracker = UsageTracker::new(db);
        let launcher = AppLauncher::new(tracker);

        let result = launcher.launch_by_bundle_id("com.nonexistent.app.12345");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_bundle_id_from_nonexistent_path() {
        let result = extract_bundle_id_from_path(Path::new("/Applications/NonExistent.app"));
        assert!(result.is_none());
    }

    #[test]
    #[ignore = "requires actual app to be installed"]
    fn test_extract_bundle_id_from_safari() {
        let result = extract_bundle_id_from_path(Path::new("/Applications/Safari.app"));
        assert_eq!(result, Some("com.apple.Safari".to_string()));
    }

    #[tokio::test]
    async fn test_app_launcher_async_launch_nonexistent() {
        use crate::storage::Database;

        let db = Database::open_in_memory().expect("should open database");
        let tracker = UsageTracker::new(db);
        let launcher = AppLauncher::new(tracker);

        let result = launcher
            .launch_by_bundle_id_async("com.nonexistent.app.12345".to_string())
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_reveal_action() {
        use crate::storage::Database;
        use std::fs::File;
        use tempfile::tempdir;

        let db = Database::open_in_memory().expect("should open database");
        let tracker = UsageTracker::new(db);
        let launcher = AppLauncher::new(tracker);

        // Create a temp file to reveal
        let temp = tempdir().expect("should create temp dir");
        let file_path = temp.path().join("test_file.txt");
        File::create(&file_path).expect("should create file");

        let action = SearchAction::RevealInFinder { path: file_path };

        // This will actually open Finder, but we're just testing it doesn't error
        let result = launcher.execute_action(&action);
        // The test may pass or fail depending on whether Finder can open
        let _ = result;
    }
}
