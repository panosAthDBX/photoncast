//! Integration tests for app launching.
//!
//! These tests verify the app launching functionality including:
//! - Launching apps by bundle ID
//! - Launching apps by path
//! - Error handling for missing/damaged apps
//! - Usage tracking integration
//! - SearchAction execution

use std::path::{Path, PathBuf};
use std::sync::Arc;

use photoncast_core::platform::launch::{
    launch_app_by_bundle_id, launch_app_by_path, open_file, reveal_in_finder, AppLauncher,
    LaunchError,
};
use photoncast_core::search::SearchAction;
use photoncast_core::storage::{Database, UsageTracker};
use tempfile::tempdir;

// =============================================================================
// Test Helpers
// =============================================================================

fn create_tracker() -> UsageTracker {
    let db = Database::open_in_memory().expect("should create in-memory database");
    UsageTracker::new(db)
}

fn create_launcher() -> AppLauncher {
    AppLauncher::new(create_tracker())
}

fn create_launcher_with_shared_tracker() -> (AppLauncher, Arc<UsageTracker>) {
    let tracker = Arc::new(create_tracker());
    let launcher = AppLauncher::with_shared_tracker(Arc::clone(&tracker));
    (launcher, tracker)
}

// =============================================================================
// LaunchError Tests
// =============================================================================

#[test]
fn test_launch_error_not_found_variants() {
    let error = LaunchError::NotFound {
        bundle_id: "com.example.test".to_string(),
    };

    // Check error message
    let message = error.user_message();
    assert!(message.contains("no longer installed"));
    assert!(message.contains("com.example.test"));

    // Check action hint
    assert_eq!(error.action_hint(), Some("Remove from index"));

    // Check not recoverable
    assert!(!error.is_recoverable());

    // Check should not offer reveal
    assert!(!error.should_offer_reveal());
}

#[test]
fn test_launch_error_damaged_variants() {
    let error = LaunchError::Damaged {
        app: "DamagedApp".to_string(),
    };

    // Check error message
    let message = error.user_message();
    assert!(message.contains("damaged"));
    assert!(message.contains("reinstalling"));

    // Check action hint - should suggest reveal
    assert_eq!(error.action_hint(), Some("Reveal in Finder"));

    // Check not recoverable
    assert!(!error.is_recoverable());

    // Check should offer reveal
    assert!(error.should_offer_reveal());
}

#[test]
fn test_launch_error_launch_failed_variants() {
    let error = LaunchError::LaunchFailed {
        app: "FailedApp".to_string(),
        reason: "permission denied".to_string(),
    };

    // Check error message
    let message = error.user_message();
    assert!(message.contains("Couldn't open"));
    assert!(message.contains("permission denied"));

    // Check action hint
    assert_eq!(error.action_hint(), Some("Retry"));

    // Check is recoverable
    assert!(error.is_recoverable());

    // Check should not offer reveal
    assert!(!error.should_offer_reveal());
}

#[test]
fn test_launch_error_display_trait() {
    let not_found = LaunchError::NotFound {
        bundle_id: "com.test".to_string(),
    };
    let display = format!("{}", not_found);
    assert!(display.contains("not found"));

    let damaged = LaunchError::Damaged {
        app: "TestApp".to_string(),
    };
    let display = format!("{}", damaged);
    assert!(display.contains("damaged"));

    let failed = LaunchError::LaunchFailed {
        app: "App".to_string(),
        reason: "error".to_string(),
    };
    let display = format!("{}", failed);
    assert!(display.contains("failed to launch"));
}

// =============================================================================
// Launch Function Tests
// =============================================================================

#[test]
fn test_launch_nonexistent_bundle_id() {
    let result = launch_app_by_bundle_id("com.nonexistent.photoncast.test.app.12345");

    assert!(result.is_err());
    let error = result.unwrap_err();

    // Should be NotFound error
    match error {
        LaunchError::NotFound { bundle_id } => {
            assert!(bundle_id.contains("nonexistent"));
        },
        other => panic!("expected NotFound error, got {:?}", other),
    }
}

#[test]
fn test_launch_nonexistent_path() {
    let result = launch_app_by_path(Path::new("/Applications/PhotonCastNonExistentApp99999.app"));

    assert!(result.is_err());
}

#[test]
fn test_launch_empty_bundle_id() {
    // Empty bundle ID should fail
    let result = launch_app_by_bundle_id("");
    assert!(result.is_err());
}

#[test]
fn test_open_nonexistent_file() {
    let result = open_file(Path::new("/tmp/photoncast_nonexistent_test_file_12345.txt"));
    assert!(result.is_err());
}

#[test]
fn test_reveal_temp_file() {
    // Create a temp file and try to reveal it
    let temp = tempdir().expect("should create temp dir");
    let file_path = temp.path().join("test_file.txt");
    std::fs::File::create(&file_path).expect("should create file");

    // This will actually open Finder - we just test it doesn't panic
    let result = reveal_in_finder(&file_path);
    // Result depends on whether Finder can open, which is system-dependent
    let _ = result;
}

// =============================================================================
// AppLauncher Tests
// =============================================================================

#[test]
fn test_app_launcher_creation() {
    let launcher = create_launcher();
    // Just verify it creates without error
    let _ = launcher;
}

#[test]
fn test_app_launcher_with_shared_tracker() {
    let (launcher, tracker) = create_launcher_with_shared_tracker();
    // Both should exist
    let _ = launcher;
    let _ = tracker;
}

#[test]
fn test_app_launcher_launch_nonexistent() {
    let launcher = create_launcher();
    let result = launcher.launch_by_bundle_id("com.nonexistent.photoncast.test");
    assert!(result.is_err());
}

#[test]
fn test_app_launcher_launch_by_path_nonexistent() {
    let launcher = create_launcher();
    let result = launcher.launch_by_path(Path::new("/Applications/PhotonCastTestNonExistent.app"));
    assert!(result.is_err());
}

// =============================================================================
// SearchAction Execution Tests
// =============================================================================

#[test]
fn test_execute_launch_app_action_nonexistent() {
    let launcher = create_launcher();
    let action = SearchAction::LaunchApp {
        bundle_id: "com.nonexistent.photoncast.test".to_string(),
        path: PathBuf::from("/Applications/NonExistent.app"),
    };

    let result = launcher.execute_action(&action);
    assert!(result.is_err());
}

#[test]
fn test_execute_reveal_action() {
    let launcher = create_launcher();
    let temp = tempdir().expect("should create temp dir");
    let file_path = temp.path().join("reveal_test.txt");
    std::fs::File::create(&file_path).expect("should create file");

    let action = SearchAction::RevealInFinder { path: file_path };

    // This opens Finder - we just verify it doesn't panic
    let result = launcher.execute_action(&action);
    let _ = result;
}

#[test]
fn test_execute_open_file_action_nonexistent() {
    let launcher = create_launcher();
    let action = SearchAction::OpenFile {
        path: PathBuf::from("/tmp/photoncast_nonexistent_12345.txt"),
    };

    let result = launcher.execute_action(&action);
    assert!(result.is_err());
}

#[test]
fn test_execute_command_action_ignored() {
    let launcher = create_launcher();
    let action = SearchAction::ExecuteCommand {
        command_id: "sleep".to_string(),
    };

    // Command actions should be ignored (return Ok)
    let result = launcher.execute_action(&action);
    assert!(result.is_ok());
}

// =============================================================================
// Async Tests
// =============================================================================

#[tokio::test]
async fn test_app_launcher_async_launch_nonexistent() {
    let launcher = create_launcher();
    let result = launcher
        .launch_by_bundle_id_async("com.nonexistent.photoncast.test".to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_app_launcher_async_execute_action_nonexistent() {
    let launcher = create_launcher();
    let action = SearchAction::LaunchApp {
        bundle_id: "com.nonexistent.photoncast.test".to_string(),
        path: PathBuf::from("/Applications/NonExistent.app"),
    };

    let result = launcher.execute_action_async(action).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_app_launcher_async_command_ignored() {
    let launcher = create_launcher();
    let action = SearchAction::ExecuteCommand {
        command_id: "lock_screen".to_string(),
    };

    let result = launcher.execute_action_async(action).await;
    assert!(result.is_ok());
}

// =============================================================================
// Integration Tests with Real Apps (Ignored by default)
// =============================================================================

#[test]
#[ignore = "requires Safari to be installed"]
fn test_launch_safari_by_bundle_id() {
    let result = launch_app_by_bundle_id("com.apple.Safari");
    assert!(result.is_ok());
}

#[test]
#[ignore = "requires Finder to be present"]
fn test_launch_finder_by_path() {
    let result = launch_app_by_path(Path::new("/System/Library/CoreServices/Finder.app"));
    assert!(result.is_ok());
}

#[test]
#[ignore = "requires Safari to be installed"]
fn test_app_launcher_with_usage_tracking() {
    let (launcher, tracker) = create_launcher_with_shared_tracker();

    // Launch Safari
    let result = launcher.launch_by_bundle_id("com.apple.Safari");
    assert!(result.is_ok());

    // Check usage was recorded
    let frecency = tracker
        .get_app_frecency("com.apple.Safari")
        .expect("should get frecency");
    assert_eq!(frecency.frequency, 1);
    assert!(frecency.recency > 0.9); // Should be very recent
}

#[tokio::test]
#[ignore = "requires Safari to be installed"]
async fn test_app_launcher_async_with_usage_tracking() {
    let (launcher, tracker) = create_launcher_with_shared_tracker();

    // Launch Safari asynchronously
    let result = launcher
        .launch_by_bundle_id_async("com.apple.Safari".to_string())
        .await;
    assert!(result.is_ok());

    // Check usage was recorded
    let frecency = tracker
        .get_app_frecency_async("com.apple.Safari".to_string())
        .await
        .expect("should get frecency");
    assert_eq!(frecency.frequency, 1);
}

#[test]
#[ignore = "requires actual file to open"]
fn test_open_file_with_usage_tracking() {
    let launcher = create_launcher();
    let temp = tempdir().expect("should create temp dir");
    let file_path = temp.path().join("test.txt");
    std::fs::write(&file_path, "test content").expect("should write file");

    let action = SearchAction::OpenFile { path: file_path };
    let result = launcher.execute_action(&action);

    // On macOS, this should succeed (TextEdit or similar opens)
    let _ = result;
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_error_clone() {
    let error = LaunchError::NotFound {
        bundle_id: "com.test".to_string(),
    };
    let cloned = error.clone();
    assert_eq!(error, cloned);
}

#[test]
fn test_error_eq() {
    let error1 = LaunchError::NotFound {
        bundle_id: "com.test".to_string(),
    };
    let error2 = LaunchError::NotFound {
        bundle_id: "com.test".to_string(),
    };
    let error3 = LaunchError::NotFound {
        bundle_id: "com.other".to_string(),
    };

    assert_eq!(error1, error2);
    assert_ne!(error1, error3);
}

#[test]
fn test_error_debug() {
    let error = LaunchError::Damaged {
        app: "TestApp".to_string(),
    };
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("Damaged"));
    assert!(debug_str.contains("TestApp"));
}
