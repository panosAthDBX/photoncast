//! End-to-end tests for PhotonCast.
//!
//! These tests verify the full application lifecycle and
//! key user workflows.

use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use photoncast_core::app::config::Config;
use photoncast_core::app::config_file::{load_config_from, save_config_to};
use photoncast_core::app::integration::{IntegrationConfig, PhotonCastApp, SearchOutcome};
use photoncast_core::commands::SystemCommand;
use photoncast_core::indexer::{AppBundleId, IndexedApp};
use photoncast_core::platform::{LoginItemManager, MenuBarConfig, MenuBarManager, MenuBarStatus};
use photoncast_core::search::{ResultType, SearchAction};
use photoncast_core::utils::profiling::{
    targets, PerformanceReport, ProfileResult, ScopedProfiler,
};

// =============================================================================
// Helper Functions
// =============================================================================

fn create_test_app(name: &str, bundle_id: &str) -> IndexedApp {
    IndexedApp {
        name: name.to_string(),
        bundle_id: AppBundleId::new(bundle_id),
        path: PathBuf::from(format!("/Applications/{}.app", name)),
        icon_path: None,
        category: None,
        keywords: Vec::new(),
        last_modified: Utc::now(),
    }
}

fn create_test_apps() -> Vec<IndexedApp> {
    vec![
        create_test_app("Safari", "com.apple.Safari"),
        create_test_app("Finder", "com.apple.finder"),
        create_test_app("System Settings", "com.apple.systempreferences"),
        create_test_app("Terminal", "com.apple.Terminal"),
        create_test_app("Xcode", "com.apple.dt.Xcode"),
        create_test_app("Visual Studio Code", "com.microsoft.VSCode"),
        create_test_app("Slack", "com.tinyspeck.slackmacgap"),
        create_test_app("Spotify", "com.spotify.client"),
    ]
}

// =============================================================================
// Full App Lifecycle Tests (Task 3.10.9)
// =============================================================================

#[test]
fn test_full_app_lifecycle() {
    // 1. Initialize the application
    let app = PhotonCastApp::new();
    assert_eq!(app.app_count(), 0);
    assert_eq!(app.search_engine().provider_count(), 3); // apps, commands, files

    // 2. Add applications to the index
    app.set_apps(create_test_apps());
    assert_eq!(app.app_count(), 8);

    // 3. Perform searches and verify results
    let outcome = app.search("safari");
    assert!(!outcome.timed_out);
    assert!(!outcome.results.is_empty());
    assert!(outcome.results.total_count > 0);

    // 4. Verify we can search different types
    let outcome = app.search("sleep");
    assert!(!outcome.timed_out);
    assert!(outcome.results.total_count > 0);

    // 5. Empty search returns empty results
    let outcome = app.search("");
    assert!(outcome.results.is_empty());
}

#[test]
fn test_app_initialization_with_custom_config() {
    let config = IntegrationConfig {
        search_timeout_ms: 200,
        max_results_per_provider: 5,
        max_total_results: 10,
        include_files: false,
        file_result_limit: 3,
    };

    let app = PhotonCastApp::with_config(config);

    // Should only have 2 providers (no files)
    assert_eq!(app.search_engine().provider_count(), 2);
}

// =============================================================================
// Search → Activate Workflow Tests
// =============================================================================

#[test]
fn test_search_activate_app_workflow() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    // Step 1: Search for an app
    let outcome = app.search("safari");
    assert!(!outcome.timed_out);

    // Step 2: Get the top result
    let results = &outcome.results;
    assert!(results.total_count > 0);

    let top_result = results.get(0).expect("Should have at least one result");

    // Step 3: Verify it's an app result with launch action
    assert_eq!(top_result.result_type, ResultType::Application);
    assert!(top_result.title.to_lowercase().contains("safari"));

    match &top_result.action {
        SearchAction::LaunchApp { bundle_id, path } => {
            assert_eq!(bundle_id, "com.apple.Safari");
            assert!(path.to_string_lossy().contains("Safari"));
        },
        _ => panic!("Expected LaunchApp action"),
    }
}

#[test]
fn test_search_activate_command_workflow() {
    let app = PhotonCastApp::new();

    // Step 1: Search for a command
    let outcome = app.search("lock screen");
    assert!(!outcome.timed_out);

    // Step 2: Get results
    let results = &outcome.results;
    assert!(results.total_count > 0);

    // Step 3: Find the lock screen command
    let lock_result = results
        .iter()
        .find(|r| r.result_type == ResultType::SystemCommand)
        .expect("Should find a system command");

    match &lock_result.action {
        SearchAction::ExecuteCommand { command_id } => {
            assert!(command_id.contains("lock"));
        },
        _ => panic!("Expected ExecuteCommand action"),
    }
}

#[test]
fn test_search_with_mixed_results() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    // Search for "s" which should match apps and commands
    let outcome = app.search("s");
    assert!(!outcome.timed_out);

    let results = &outcome.results;

    // Should have multiple groups
    assert!(results.groups.len() >= 1);
    assert!(results.total_count >= 1);

    // Check that groups are properly sorted by priority
    let mut last_priority = 0u8;
    for group in &results.groups {
        let priority = group.result_type.priority();
        assert!(
            priority >= last_priority,
            "Groups should be sorted by priority"
        );
        last_priority = priority;
    }
}

// =============================================================================
// Config Loading and Saving Tests
// =============================================================================

#[test]
fn test_config_workflow() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Step 1: Create and save initial config
    let mut config = Config::default();
    config.general.max_results = 15;
    config.general.launch_at_login = true;
    config.appearance.accent_color = "blue".to_string();

    save_config_to(&config, &config_path).unwrap();

    // Step 2: Load config
    let loaded = load_config_from(&config_path).unwrap();

    // Step 3: Verify values persisted
    assert_eq!(loaded.general.max_results, 15);
    assert!(loaded.general.launch_at_login);
    assert_eq!(loaded.appearance.accent_color, "blue");

    // Step 4: Modify and re-save
    let mut updated = loaded;
    updated.general.max_results = 20;
    save_config_to(&updated, &config_path).unwrap();

    // Step 5: Reload and verify
    let reloaded = load_config_from(&config_path).unwrap();
    assert_eq!(reloaded.general.max_results, 20);
}

// =============================================================================
// Menu Bar Tests
// =============================================================================

#[test]
fn test_menu_bar_lifecycle() {
    // Step 1: Create menu bar manager
    let mut manager = MenuBarManager::new();
    assert_eq!(manager.status(), MenuBarStatus::Hidden);

    // Step 2: Initialize (show icon)
    manager.initialize().unwrap();
    assert_eq!(manager.status(), MenuBarStatus::Visible);
    assert!(manager.should_show());

    // Step 3: Hide
    manager.hide();
    assert_eq!(manager.status(), MenuBarStatus::Hidden);
    assert!(!manager.should_show());

    // Step 4: Show again
    manager.show();
    assert_eq!(manager.status(), MenuBarStatus::Visible);
}

#[test]
fn test_menu_bar_disabled_by_config() {
    let config = MenuBarConfig {
        show_icon: false,
        ..Default::default()
    };

    let mut manager = MenuBarManager::with_config(config);
    manager.initialize().unwrap();

    // Should remain hidden when disabled
    assert_eq!(manager.status(), MenuBarStatus::Hidden);
    assert!(!manager.should_show());
}

// =============================================================================
// Login Item Tests
// =============================================================================

#[test]
fn test_login_item_manager_initialization() {
    let manager = LoginItemManager::for_photoncast();
    assert_eq!(manager.bundle_id(), "app.photoncast");
}

// =============================================================================
// Performance Tests
// =============================================================================

#[test]
fn test_search_performance_target() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    let profiler = ScopedProfiler::search_latency();
    let outcome = app.search("safari");
    let result = profiler.finish();

    assert!(!outcome.timed_out);

    // Log the result but don't fail on performance
    // (CI environments may be slower)
    if !result.met_target {
        eprintln!(
            "Warning: Search took {}ms (target: {}ms)",
            result.duration.as_millis(),
            result.target.as_millis()
        );
    }
}

#[test]
fn test_app_initialization_performance() {
    let profiler = ScopedProfiler::cold_start();
    let _app = PhotonCastApp::new();
    let result = profiler.finish();

    // Log the result
    if result.met_target {
        println!(
            "App initialization: {}ms (target: {}ms) ✓",
            result.duration.as_millis(),
            result.target.as_millis()
        );
    } else {
        eprintln!(
            "Warning: App initialization took {}ms (target: {}ms)",
            result.duration.as_millis(),
            result.target.as_millis()
        );
    }
}

#[test]
fn test_performance_report() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    let mut report = PerformanceReport::new();

    // Test app init
    let (_, init_result) =
        photoncast_core::utils::profile("App Init", targets::COLD_START, PhotonCastApp::new);
    report.add(init_result);

    // Test search
    let (_, search_result) =
        photoncast_core::utils::profile("Search", targets::SEARCH_LATENCY, || app.search("test"));
    report.add(search_result);

    // Check the report
    assert_eq!(report.count(), 2);
    println!("{}", report.summary());
}

// =============================================================================
// Integration Tests with All Components
// =============================================================================

#[test]
fn test_full_search_workflow_with_grouping() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    // Search for something that matches apps
    let outcome = app.search("s");
    let results = &outcome.results;

    // Verify group ordering (Apps before Commands before Files)
    let group_order: Vec<ResultType> = results.groups.iter().map(|g| g.result_type).collect();

    for i in 0..group_order.len().saturating_sub(1) {
        assert!(
            group_order[i].priority() <= group_order[i + 1].priority(),
            "Groups should be ordered: Apps → Commands → Files"
        );
    }
}

#[test]
fn test_search_result_navigation() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    let outcome = app.search("s");
    let results = &outcome.results;

    // Test flat iteration
    let all_results: Vec<_> = results.iter().collect();
    assert_eq!(all_results.len(), results.total_count);

    // Test indexed access
    for i in 0..results.total_count {
        let result = results.get(i);
        assert!(
            result.is_some(),
            "Should be able to get result at index {}",
            i
        );
    }

    // Out of bounds returns None
    assert!(results.get(results.total_count + 10).is_none());
}

#[test]
fn test_empty_search_state() {
    let app = PhotonCastApp::new();

    // Empty query
    let outcome = app.search("");
    assert!(outcome.results.is_empty());
    assert_eq!(outcome.results.total_count, 0);
    assert!(outcome.results.groups.is_empty());

    // No apps indexed, search for something
    let outcome = app.search("xyz");
    // Should still get commands if they match
    // But unlikely to match "xyz"
}

#[test]
fn test_command_search_coverage() {
    let app = PhotonCastApp::new();

    // Verify all system commands are searchable
    for cmd_info in SystemCommand::all() {
        let outcome = app.search(cmd_info.name);
        assert!(
            outcome.results.total_count > 0,
            "Should find command: {}",
            cmd_info.name
        );
    }
}

#[test]
fn test_search_result_consistency() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    let query = "safari";

    // Search multiple times
    let outcome1 = app.search(query);
    let outcome2 = app.search(query);

    // Results should be consistent
    assert_eq!(outcome1.results.total_count, outcome2.results.total_count);

    // First result should be the same
    if let (Some(r1), Some(r2)) = (outcome1.results.get(0), outcome2.results.get(0)) {
        assert_eq!(r1.title, r2.title);
        assert_eq!(r1.id.as_str(), r2.id.as_str());
    }
}

// =============================================================================
// Async Tests
// =============================================================================

#[tokio::test]
async fn test_async_search_workflow() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    let outcome = app.search_async("safari").await;
    assert!(!outcome.timed_out);
    assert!(!outcome.results.is_empty());
}

#[tokio::test]
async fn test_async_search_empty_query() {
    let app = PhotonCastApp::new();
    let outcome = app.search_async("").await;
    assert!(outcome.results.is_empty());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_unicode_query() {
    let app = PhotonCastApp::new();
    app.set_apps(vec![create_test_app("日本語アプリ", "com.test.japanese")]);

    // Search with unicode
    let outcome = app.search("日本語");
    // Should not panic
    assert!(!outcome.timed_out);
}

#[test]
fn test_very_long_query() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    let long_query = "a".repeat(1000);
    let outcome = app.search(&long_query);

    // Should not panic or hang
    assert!(!outcome.timed_out);
}

#[test]
fn test_special_characters_in_query() {
    let app = PhotonCastApp::new();
    app.set_apps(create_test_apps());

    // Various special characters
    let special_queries = ["*", "?", "[", "]", "(", ")", "\\", "/", "\"", "'"];

    for query in special_queries {
        let outcome = app.search(query);
        // Should not panic
        assert!(!outcome.timed_out);
    }
}
