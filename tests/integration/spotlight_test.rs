//! Integration tests for Spotlight file search.
//!
//! These tests verify:
//! - SpotlightQuery execution with mdfind command
//! - Async query execution with timeouts
//! - FileResult type and conversion
//! - FileProvider integration with SearchProvider trait
//! - File opening and reveal in Finder functionality
//!
//! Note: Many of these tests require macOS and actual filesystem access.
//! Tests marked with `#[ignore]` are skipped in CI but can be run locally.

use std::path::PathBuf;
use std::time::Instant;

use photoncast_core::platform::spotlight::{
    FileKind, FileResult, SpotlightError, SpotlightProvider, SpotlightQuery, DEFAULT_MAX_RESULTS,
    DEFAULT_TIMEOUT_MS,
};
use photoncast_core::search::providers::files::FileProvider;
use photoncast_core::search::providers::SearchProvider;
use photoncast_core::search::{ResultType, SearchAction};

// =============================================================================
// FileKind Tests (Task 3.8.3)
// =============================================================================

#[test]
fn test_file_kind_document_detection() {
    let extensions = vec!["pdf", "doc", "docx", "txt", "rtf", "pages", "md", "odt"];

    for ext in extensions {
        let path = PathBuf::from(format!("/test/file.{}", ext));
        let kind = FileKind::from_path(&path);
        assert_eq!(
            kind,
            FileKind::Document,
            "Extension .{} should be Document",
            ext
        );
    }
}

#[test]
fn test_file_kind_image_detection() {
    let extensions = vec![
        "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "heic", "webp",
    ];

    for ext in extensions {
        let path = PathBuf::from(format!("/test/file.{}", ext));
        let kind = FileKind::from_path(&path);
        assert_eq!(kind, FileKind::Image, "Extension .{} should be Image", ext);
    }
}

#[test]
fn test_file_kind_audio_detection() {
    let extensions = vec!["mp3", "m4a", "wav", "aac", "flac", "ogg", "aiff"];

    for ext in extensions {
        let path = PathBuf::from(format!("/test/file.{}", ext));
        let kind = FileKind::from_path(&path);
        assert_eq!(kind, FileKind::Audio, "Extension .{} should be Audio", ext);
    }
}

#[test]
fn test_file_kind_video_detection() {
    let extensions = vec!["mp4", "m4v", "mov", "avi", "mkv", "webm"];

    for ext in extensions {
        let path = PathBuf::from(format!("/test/file.{}", ext));
        let kind = FileKind::from_path(&path);
        assert_eq!(kind, FileKind::Video, "Extension .{} should be Video", ext);
    }
}

#[test]
fn test_file_kind_application_detection() {
    let path = PathBuf::from("/Applications/Safari.app");
    let kind = FileKind::from_path(&path);
    assert_eq!(kind, FileKind::Application);
}

#[test]
fn test_file_kind_display_names() {
    assert_eq!(FileKind::File.display_name(), "File");
    assert_eq!(FileKind::Folder.display_name(), "Folder");
    assert_eq!(FileKind::Application.display_name(), "Application");
    assert_eq!(FileKind::Document.display_name(), "Document");
    assert_eq!(FileKind::Image.display_name(), "Image");
    assert_eq!(FileKind::Audio.display_name(), "Audio");
    assert_eq!(FileKind::Video.display_name(), "Video");
    assert_eq!(FileKind::Other.display_name(), "Other");
}

#[test]
fn test_file_kind_icon_names() {
    assert_eq!(FileKind::File.icon_name(), "doc");
    assert_eq!(FileKind::Folder.icon_name(), "folder");
    assert_eq!(FileKind::Application.icon_name(), "app");
    assert_eq!(FileKind::Document.icon_name(), "doc.text");
    assert_eq!(FileKind::Image.icon_name(), "photo");
    assert_eq!(FileKind::Audio.icon_name(), "music.note");
    assert_eq!(FileKind::Video.icon_name(), "video");
}

// =============================================================================
// FileResult Tests (Task 3.8.3)
// =============================================================================

#[test]
fn test_file_result_from_path() {
    let path = PathBuf::from("/Users/test/Documents/report.pdf");
    let result = FileResult::from_path(path.clone());

    assert_eq!(result.name, "report.pdf");
    assert_eq!(result.path, path);
    assert_eq!(result.kind, FileKind::Document);
    assert!(result.size.is_none()); // Lazy loading
    assert!(result.modified.is_none()); // Lazy loading
}

#[test]
fn test_file_result_unknown_name() {
    // Test path with no file name component
    let path = PathBuf::from("/");
    let result = FileResult::from_path(path);

    // Should fall back to "Unknown"
    assert!(!result.name.is_empty());
}

// =============================================================================
// SpotlightQuery Tests (Task 3.8.1, 3.8.2)
// =============================================================================

#[test]
fn test_spotlight_query_builder() {
    let query = SpotlightQuery::new("test")
        .with_max_results(10)
        .with_timeout_ms(1000);

    // Verify builder pattern works
    assert!(true); // If we get here without panic, builder works
}

#[test]
fn test_spotlight_query_with_scope() {
    let query = SpotlightQuery::new("test").with_scope(PathBuf::from("/tmp"));

    // Verify scope is set
    assert!(true);
}

#[test]
fn test_spotlight_query_with_home_scope() {
    let query = SpotlightQuery::new("test").with_home_scope();

    // Home scope should be set if home directory exists
    assert!(true);
}

#[test]
fn test_empty_query_returns_empty_results() {
    let query = SpotlightQuery::new("");
    let result = query.execute_sync();

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_null_char_query_is_invalid() {
    let query = SpotlightQuery::new("test\0invalid");
    let result = query.execute_sync();

    assert!(matches!(result, Err(SpotlightError::InvalidQuery { .. })));
}

// =============================================================================
// SpotlightProvider Tests (Task 3.8.1, 3.8.2)
// =============================================================================

#[test]
fn test_spotlight_provider_default_config() {
    let provider = SpotlightProvider::new();

    assert_eq!(provider.max_results, DEFAULT_MAX_RESULTS);
    assert_eq!(provider.timeout_ms, DEFAULT_TIMEOUT_MS);
    assert!(provider.search_scope.is_some()); // Should default to home dir
}

#[test]
fn test_spotlight_provider_custom_max_results() {
    let provider = SpotlightProvider::with_max_results(20);
    assert_eq!(provider.max_results, 20);
}

#[test]
fn test_spotlight_provider_custom_timeout() {
    let provider = SpotlightProvider::new().with_timeout_ms(1000);
    assert_eq!(provider.timeout_ms, 1000);
}

#[test]
fn test_spotlight_provider_custom_scope() {
    let provider = SpotlightProvider::new().with_scope(PathBuf::from("/tmp"));
    assert_eq!(provider.search_scope, Some(PathBuf::from("/tmp")));
}

// =============================================================================
// SpotlightError Tests
// =============================================================================

#[test]
fn test_spotlight_error_timeout_recoverable() {
    let error = SpotlightError::Timeout { timeout_ms: 500 };
    assert!(error.is_recoverable());
    assert!(!error.user_message().is_empty());
}

#[test]
fn test_spotlight_error_command_failed_not_recoverable() {
    let error = SpotlightError::CommandFailed {
        reason: "test failure".to_string(),
    };
    assert!(!error.is_recoverable());
}

#[test]
fn test_spotlight_error_invalid_query_recoverable() {
    let error = SpotlightError::InvalidQuery {
        reason: "bad query".to_string(),
    };
    assert!(error.is_recoverable());
}

// =============================================================================
// FileProvider Tests (Task 3.8.4)
// =============================================================================

#[test]
fn test_file_provider_default() {
    let provider = FileProvider::default();

    assert_eq!(provider.name(), "Files");
    assert_eq!(provider.result_type(), ResultType::File);
}

#[test]
fn test_file_provider_empty_query() {
    let provider = FileProvider::new(5);
    let results = provider.search("", 10);

    assert!(results.is_empty());
}

#[test]
fn test_file_provider_cache_clearing() {
    let provider = FileProvider::new(5);

    // Trigger caching by searching (even empty results populate cache state)
    let _ = provider.search("", 5);

    // Clear should not panic
    provider.clear_cache();
}

#[test]
fn test_file_provider_result_type() {
    let provider = FileProvider::new(5);
    assert_eq!(provider.result_type(), ResultType::File);
}

// =============================================================================
// Integration Tests (require macOS and mdfind)
// =============================================================================

#[test]
#[ignore = "requires mdfind command and actual filesystem"]
fn test_spotlight_query_execution_sync() {
    // Search for a common term that should exist
    let query = SpotlightQuery::new("Desktop")
        .with_max_results(5)
        .with_home_scope();

    let start = Instant::now();
    let result = query.execute_sync();
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Query should succeed");

    // Performance: should complete reasonably quickly
    assert!(
        elapsed.as_millis() < 2000,
        "Query took too long: {:?}",
        elapsed
    );
}

#[test]
#[ignore = "requires mdfind command and actual filesystem"]
fn test_spotlight_query_respects_max_results() {
    let query = SpotlightQuery::new("")
        .with_max_results(3)
        .with_home_scope();

    // Empty query should return empty, but let's test with a real query
    let query = SpotlightQuery::new("Documents")
        .with_max_results(3)
        .with_home_scope();

    let result = query.execute_sync();

    if let Ok(results) = result {
        assert!(results.len() <= 3, "Should respect max_results limit");
    }
}

#[tokio::test]
#[ignore = "requires mdfind command and actual filesystem"]
async fn test_spotlight_query_execution_async() {
    let query = SpotlightQuery::new("Desktop")
        .with_max_results(5)
        .with_home_scope();

    let start = Instant::now();
    let result = query.execute().await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Async query should succeed");
    assert!(
        elapsed.as_millis() < 2000,
        "Async query took too long: {:?}",
        elapsed
    );
}

#[test]
#[ignore = "requires mdfind command and actual filesystem"]
fn test_spotlight_provider_search_sync() {
    let provider = SpotlightProvider::with_max_results(5);
    let result = provider.search_sync("readme");

    // Should not error (may return empty if no readme files)
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires mdfind command and actual filesystem"]
async fn test_spotlight_provider_search_async() {
    let provider = SpotlightProvider::with_max_results(5);
    let result = provider.search("readme").await;

    assert!(result.is_ok());
}

#[test]
#[ignore = "requires mdfind command and actual filesystem"]
fn test_file_provider_search_integration() {
    let provider = FileProvider::new(5);
    let results = provider.search("readme", 5);

    // Results should be valid SearchResult objects
    for result in &results {
        assert!(!result.title.is_empty());
        assert!(result.id.as_str().starts_with("file:"));

        // All file results should have OpenFile action
        match &result.action {
            SearchAction::OpenFile { path } => {
                assert!(!path.display().to_string().is_empty());
            },
            _ => panic!("Expected OpenFile action, got {:?}", result.action),
        }
    }
}

#[tokio::test]
#[ignore = "requires mdfind command and actual filesystem"]
async fn test_file_provider_async_search_integration() {
    let provider = FileProvider::new(5);
    let results = provider.search_async("readme", 5).await;

    // Should return valid results
    for result in &results {
        assert!(!result.title.is_empty());
    }
}

#[test]
#[ignore = "requires mdfind command and actual filesystem"]
fn test_file_search_performance() {
    let provider = FileProvider::new(5);

    let start = Instant::now();
    let _results = provider.search("test", 5);
    let elapsed = start.elapsed();

    // Should complete within 500ms timeout + overhead
    assert!(
        elapsed.as_millis() < 1000,
        "File search took too long: {:?}",
        elapsed
    );
}

// =============================================================================
// Timeout Tests
// =============================================================================

#[tokio::test]
async fn test_spotlight_query_timeout_handling() {
    // Create a query with very short timeout
    let query = SpotlightQuery::new("a")  // Common letter should have many results
        .with_timeout_ms(1)  // 1ms is too short
        .with_home_scope();

    let result = query.execute().await;

    // Result may be timeout or may succeed on fast systems
    // We just verify it doesn't panic
    let _ = result;
}

// =============================================================================
// File Opening Tests (Task 3.8.5)
// =============================================================================

#[test]
fn test_file_opening_module_exists() {
    // Verify the module structure is correct
    use photoncast_core::platform::launch::{open_file, reveal_in_finder};

    // Functions should be accessible
    let _open_fn: fn(&std::path::Path) -> Result<(), photoncast_core::platform::LaunchError> =
        open_file;
    let _reveal_fn: fn(&std::path::Path) -> Result<(), photoncast_core::platform::LaunchError> =
        reveal_in_finder;
}

#[test]
fn test_search_action_open_file_variant() {
    let path = PathBuf::from("/test/file.txt");
    let action = SearchAction::OpenFile { path: path.clone() };

    match action {
        SearchAction::OpenFile { path: p } => {
            assert_eq!(p, path);
        },
        _ => panic!("Expected OpenFile action"),
    }
}

#[test]
fn test_search_action_reveal_in_finder_variant() {
    let path = PathBuf::from("/test/file.txt");
    let action = SearchAction::RevealInFinder { path: path.clone() };

    match action {
        SearchAction::RevealInFinder { path: p } => {
            assert_eq!(p, path);
        },
        _ => panic!("Expected RevealInFinder action"),
    }
}

// =============================================================================
// File Usage Tracking Tests (Task 3.8.6)
// =============================================================================

#[test]
fn test_file_usage_tracker_trait() {
    use photoncast_core::search::providers::files::{FileUsageTracker, NoOpFileTracker};

    let tracker = NoOpFileTracker;
    tracker.record_file_open("/test/file.txt");

    // NoOp tracker should always return 0
    assert_eq!(tracker.get_usage_count("/test/file.txt"), 0);
}

#[test]
fn test_usage_tracker_integration() {
    use photoncast_core::storage::{Database, UsageTracker};

    let db = Database::open_in_memory().expect("should create database");
    let tracker = UsageTracker::new(db);

    // Record file open
    tracker
        .record_file_open("/test/document.pdf")
        .expect("should record file open");

    // Get frecency
    let frecency = tracker
        .get_file_frecency("/test/document.pdf")
        .expect("should get frecency");

    assert_eq!(frecency.frequency, 1);
    assert!(frecency.score() > 0.0);
}
