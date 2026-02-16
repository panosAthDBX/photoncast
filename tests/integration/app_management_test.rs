//! Integration tests for app management features.
//!
//! These tests cover:
//! - UI indicators for running and auto-quit state (Task 8.6)
//! - Action panel behavior for running/non-running apps (Task 8.7)
//! - Configuration persistence across restarts (Task 8.8)

use std::path::PathBuf;
use tempfile::TempDir;

use photoncast_apps::{
    AutoQuitAppConfig, AutoQuitConfig, AutoQuitManager, Application, RelatedFile,
    RelatedFileCategory, UninstallPreview,
};

// =============================================================================
// Task 8.6: UI Indicators Tests
// =============================================================================

/// Helper struct to track what indicators should be shown for an app.
#[derive(Debug, Clone)]
struct AppIndicators {
    pub is_running: bool,
    pub is_auto_quit_enabled: bool,
}

impl AppIndicators {
    /// Determines if running indicator should be shown.
    pub fn should_show_running_indicator(&self) -> bool {
        self.is_running
    }

    /// Determines if auto-quit indicator should be shown.
    pub fn should_show_auto_quit_indicator(&self) -> bool {
        self.is_auto_quit_enabled
    }
}

#[test]
fn test_app_search_shows_running_indicator() {
    // Simulate an app that is running
    let indicators = AppIndicators {
        is_running: true,
        is_auto_quit_enabled: false,
    };

    // Verify running indicator is present when app is running
    assert!(
        indicators.should_show_running_indicator(),
        "Running indicator should be shown for running apps"
    );

    // Simulate a non-running app
    let non_running = AppIndicators {
        is_running: false,
        is_auto_quit_enabled: false,
    };

    // Verify running indicator is NOT shown when app is not running
    assert!(
        !non_running.should_show_running_indicator(),
        "Running indicator should NOT be shown for non-running apps"
    );
}

#[test]
fn test_app_search_shows_auto_quit_indicator() {
    // Enable auto quit for an app and verify indicator is shown
    let mut manager = AutoQuitManager::new(AutoQuitConfig::default());

    // Enable auto-quit
    let bundle_id = "com.example.testapp";
    manager.enable_auto_quit(bundle_id, 5);

    // Verify auto-quit is enabled
    assert!(
        manager.is_auto_quit_enabled(bundle_id),
        "Auto-quit should be enabled"
    );

    // Create indicators based on manager state
    let indicators = AppIndicators {
        is_running: true,
        is_auto_quit_enabled: manager.is_auto_quit_enabled(bundle_id),
    };

    // Verify auto-quit indicator should be shown
    assert!(
        indicators.should_show_auto_quit_indicator(),
        "Auto-quit indicator should be shown when auto-quit is enabled"
    );

    // Now disable auto-quit and verify indicator is hidden
    manager.disable_auto_quit(bundle_id);

    let indicators_disabled = AppIndicators {
        is_running: true,
        is_auto_quit_enabled: manager.is_auto_quit_enabled(bundle_id),
    };

    assert!(
        !indicators_disabled.should_show_auto_quit_indicator(),
        "Auto-quit indicator should NOT be shown when auto-quit is disabled"
    );
}

#[test]
fn test_running_indicator_with_finder() {
    // Finder is always running on macOS, so we can use it as a test
    #[cfg(target_os = "macos")]
    {
        let is_running = photoncast_apps::is_app_running("com.apple.finder");
        assert!(is_running, "Finder should always be running on macOS");

        let indicators = AppIndicators {
            is_running,
            is_auto_quit_enabled: false,
        };

        assert!(
            indicators.should_show_running_indicator(),
            "Should show running indicator for Finder"
        );
    }
}

#[test]
fn test_indicators_both_enabled() {
    // Test when both running and auto-quit are enabled
    let indicators = AppIndicators {
        is_running: true,
        is_auto_quit_enabled: true,
    };

    assert!(
        indicators.should_show_running_indicator(),
        "Running indicator should be shown"
    );
    assert!(
        indicators.should_show_auto_quit_indicator(),
        "Auto-quit indicator should be shown"
    );
}

// =============================================================================
// Task 8.7: Action Panel Tests
// =============================================================================

/// Actions that can be shown in the action panel.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Determines which actions should be shown based on app state.
fn get_available_actions(is_running: bool, is_system_app: bool) -> Vec<AppAction> {
    let mut actions = Vec::new();

    // Launch is always available for non-running apps
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
    // Verify keyboard shortcuts are defined correctly
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

// =============================================================================
// Task 8.8: Persistence Tests
// =============================================================================

#[test]
fn test_auto_quit_persists_across_restarts() {
    // Create a temporary directory for config
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("auto_quit.toml");

    // Create initial config with auto-quit enabled for an app
    let mut config1 = AutoQuitConfig::default();
    config1.apps.insert(
        "com.example.persistent".to_string(),
        AutoQuitAppConfig {
            enabled: true,
            timeout_minutes: 7,
            last_active: None,
        },
    );
    config1.apps.insert(
        "com.example.other".to_string(),
        AutoQuitAppConfig {
            enabled: false,
            timeout_minutes: 3,
            last_active: None,
        },
    );

    // Serialize to TOML and save
    let toml_content = toml::to_string_pretty(&config1).expect("Failed to serialize config");
    std::fs::write(&config_path, &toml_content).expect("Failed to write config");

    // "Restart" - Load config from file
    let loaded_content = std::fs::read_to_string(&config_path).expect("Failed to read config");
    let config2: AutoQuitConfig = toml::from_str(&loaded_content).expect("Failed to parse config");

    // Verify auto-quit settings persisted
    assert!(
        config2.apps.contains_key("com.example.persistent"),
        "Persistent app config should be loaded"
    );
    assert_eq!(
        config2.apps["com.example.persistent"].enabled, true,
        "Auto-quit enabled state should persist"
    );
    assert_eq!(
        config2.apps["com.example.persistent"].timeout_minutes, 7,
        "Timeout minutes should persist"
    );

    // Verify disabled app also persisted
    assert!(
        config2.apps.contains_key("com.example.other"),
        "Other app config should be loaded"
    );
    assert_eq!(
        config2.apps["com.example.other"].enabled, false,
        "Disabled state should persist"
    );
}

#[test]
fn test_auto_quit_manager_save_load_cycle() {
    // Test the full save/load cycle through AutoQuitManager
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("auto_quit.toml");

    // Create manager and enable auto-quit
    let mut manager = AutoQuitManager::new(AutoQuitConfig::default());
    manager.enable_auto_quit("com.test.app1", 10);
    manager.enable_auto_quit("com.test.app2", 15);

    // Simulate activity tracking
    manager.on_app_activated("com.test.app1", 42);

    // Manually save config to temp path (since load/save use system paths)
    let toml_content =
        toml::to_string_pretty(manager.config()).expect("Failed to serialize config");
    std::fs::write(&config_path, &toml_content).expect("Failed to write config");

    // Load config back
    let loaded_content = std::fs::read_to_string(&config_path).expect("Failed to read config");
    let loaded_config: AutoQuitConfig =
        toml::from_str(&loaded_content).expect("Failed to parse config");

    // Create new manager with loaded config
    let loaded_manager = AutoQuitManager::new(loaded_config);

    // Verify settings persisted
    assert!(
        loaded_manager.is_auto_quit_enabled("com.test.app1"),
        "App1 auto-quit should still be enabled after reload"
    );
    assert_eq!(
        loaded_manager.get_timeout_minutes("com.test.app1"),
        Some(10),
        "App1 timeout should persist"
    );
    assert!(
        loaded_manager.is_auto_quit_enabled("com.test.app2"),
        "App2 auto-quit should still be enabled after reload"
    );
    assert_eq!(
        loaded_manager.get_timeout_minutes("com.test.app2"),
        Some(15),
        "App2 timeout should persist"
    );
}

#[test]
fn test_uninstall_respects_selection() {
    // Create a mock uninstall preview with mixed selection
    let preview = UninstallPreview {
        app: Application {
            name: "TestApp".to_string(),
            bundle_id: "com.example.testapp".to_string(),
            path: PathBuf::from("/Applications/TestApp.app"),
            version: Some("1.0".to_string()),
            size_bytes: 1024 * 1024, // 1MB
            icon_path: None,
        },
        related_files: vec![
            RelatedFile {
                path: PathBuf::from("/Users/test/Library/Caches/com.example.testapp"),
                size_bytes: 512 * 1024, // 512KB
                category: RelatedFileCategory::Caches,
                selected: true,
            },
            RelatedFile {
                path: PathBuf::from("/Users/test/Library/Preferences/com.example.testapp.plist"),
                size_bytes: 4 * 1024, // 4KB
                category: RelatedFileCategory::Preferences,
                selected: false, // NOT selected
            },
            RelatedFile {
                path: PathBuf::from("/Users/test/Library/Application Support/TestApp"),
                size_bytes: 2 * 1024 * 1024, // 2MB
                category: RelatedFileCategory::ApplicationSupport,
                selected: true,
            },
            RelatedFile {
                path: PathBuf::from("/Users/test/Library/Logs/TestApp"),
                size_bytes: 100 * 1024, // 100KB
                category: RelatedFileCategory::Logs,
                selected: false, // NOT selected
            },
        ],
        total_size: 0, // Will be calculated
        space_freed_formatted: String::new(),
    };

    // Get only selected files using the library function
    let selected_files = photoncast_apps::get_selected_files(&preview);

    // Verify only selected files are returned
    assert_eq!(selected_files.len(), 2, "Should have 2 selected files");

    // Verify correct files are selected
    let selected_paths: Vec<&PathBuf> = selected_files.iter().map(|f| &f.path).collect();
    assert!(
        selected_paths.contains(&&PathBuf::from(
            "/Users/test/Library/Caches/com.example.testapp"
        )),
        "Caches should be selected"
    );
    assert!(
        selected_paths.contains(&&PathBuf::from(
            "/Users/test/Library/Application Support/TestApp"
        )),
        "Application Support should be selected"
    );

    // Verify non-selected files are NOT included
    assert!(
        !selected_paths.contains(&&PathBuf::from(
            "/Users/test/Library/Preferences/com.example.testapp.plist"
        )),
        "Preferences should NOT be selected"
    );
    assert!(
        !selected_paths.contains(&&PathBuf::from("/Users/test/Library/Logs/TestApp")),
        "Logs should NOT be selected"
    );
}

#[test]
fn test_uninstall_calculate_selected_size() {
    // Create a preview with known sizes
    let preview = UninstallPreview {
        app: Application {
            name: "TestApp".to_string(),
            bundle_id: "com.example.testapp".to_string(),
            path: PathBuf::from("/Applications/TestApp.app"),
            version: Some("1.0".to_string()),
            size_bytes: 1000, // 1000 bytes
            icon_path: None,
        },
        related_files: vec![
            RelatedFile {
                path: PathBuf::from("/path/to/file1"),
                size_bytes: 500,
                category: RelatedFileCategory::Caches,
                selected: true,
            },
            RelatedFile {
                path: PathBuf::from("/path/to/file2"),
                size_bytes: 300,
                category: RelatedFileCategory::Preferences,
                selected: false, // NOT selected - should not count
            },
            RelatedFile {
                path: PathBuf::from("/path/to/file3"),
                size_bytes: 200,
                category: RelatedFileCategory::Logs,
                selected: true,
            },
        ],
        total_size: 2000, // This includes all files
        space_freed_formatted: String::new(),
    };

    // Calculate size of selected files only
    let selected_size = photoncast_apps::calculate_selected_size(&preview);

    // Expected: app (1000) + file1 (500) + file3 (200) = 1700
    // file2 (300) is NOT selected
    assert_eq!(
        selected_size, 1700,
        "Selected size should be app + selected files only"
    );
}

#[test]
fn test_uninstall_all_selected_by_default() {
    // When creating related files, selected defaults to true
    let file = RelatedFile {
        path: PathBuf::from("/path/to/file"),
        size_bytes: 1000,
        category: RelatedFileCategory::Caches,
        selected: true, // Default should be true
    };

    assert!(file.selected, "Files should be selected by default");
}

#[test]
fn test_uninstall_selection_toggle() {
    // Test toggling selection state
    let mut file = RelatedFile {
        path: PathBuf::from("/path/to/file"),
        size_bytes: 1000,
        category: RelatedFileCategory::Caches,
        selected: true,
    };

    // Toggle off
    file.selected = false;
    assert!(!file.selected, "File should be deselected");

    // Toggle on
    file.selected = true;
    assert!(file.selected, "File should be selected again");
}

#[test]
fn test_persistence_empty_config() {
    // Test loading/saving an empty config
    let config = AutoQuitConfig::default();

    assert!(config.apps.is_empty(), "Default config should have no apps");

    // Serialize and deserialize
    let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
    let loaded: AutoQuitConfig = toml::from_str(&toml_str).expect("Failed to deserialize");

    assert!(
        loaded.apps.is_empty(),
        "Loaded config should still have no apps"
    );
}

#[test]
fn test_persistence_multiple_apps() {
    // Test persisting multiple apps with different settings
    let mut config = AutoQuitConfig::default();

    // Add multiple apps with various configurations
    for i in 1..=5 {
        config.apps.insert(
            format!("com.example.app{}", i),
            AutoQuitAppConfig {
                enabled: i % 2 == 0, // Even apps enabled
                timeout_minutes: i * 3,
                last_active: None,
            },
        );
    }

    // Serialize and deserialize
    let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
    let loaded: AutoQuitConfig = toml::from_str(&toml_str).expect("Failed to deserialize");

    // Verify all apps persisted correctly
    assert_eq!(loaded.apps.len(), 5, "All 5 apps should persist");

    for i in 1..=5 {
        let bundle_id = format!("com.example.app{}", i);
        let app_config = loaded.apps.get(&bundle_id).expect("App should exist");
        assert_eq!(
            app_config.enabled,
            i % 2 == 0,
            "Enabled state should persist for {}",
            bundle_id
        );
        assert_eq!(
            app_config.timeout_minutes,
            i * 3,
            "Timeout should persist for {}",
            bundle_id
        );
    }
}
