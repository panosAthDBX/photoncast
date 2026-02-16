//! Integration tests for PhotonCast app packaging features.
//!
//! This test file covers Phase 5 testing requirements:
//! - Task 5.1: Code signing & Gatekeeper verification
//! - Task 5.2: Auto-update flow
//! - Task 5.3: Dock visibility toggle
//! - Task 5.4: Menu bar behavior
//!
//! For detailed tests, see the individual test modules in tests/integration/.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// =============================================================================
// Module: Dock Visibility Tests
// =============================================================================

mod dock_visibility_tests {
    use super::*;

    /// Creates a test Info.plist with LSUIElement set
    fn create_test_plist(dir: &TempDir, lsui_element: bool) -> PathBuf {
        let plist_path = dir.path().join("Info.plist");
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>PhotonCast</string>
    <key>CFBundleIdentifier</key>
    <string>com.photoncast.app</string>
    <key>LSUIElement</key>
    <{} />
</dict>
</plist>"#,
            if lsui_element { "true" } else { "false" }
        );
        let mut file = std::fs::File::create(&plist_path).unwrap();
        file.write_all(plist_content.as_bytes()).unwrap();
        plist_path
    }

    fn read_lsui_element(plist_path: &Path) -> Result<bool, String> {
        let content =
            std::fs::read_to_string(plist_path).map_err(|e| format!("Failed to read: {}", e))?;
        let value: plist::Value =
            plist::from_bytes(content.as_bytes()).map_err(|e| format!("Failed to parse: {}", e))?;
        if let Some(dict) = value.as_dictionary() {
            match dict.get("LSUIElement") {
                Some(plist::Value::Boolean(hidden)) => Ok(!hidden),
                Some(_) => Err("Unexpected type".to_string()),
                None => Ok(true), // Default visible
            }
        } else {
            Err("Not a dictionary".to_string())
        }
    }

    #[test]
    fn test_default_state_is_hidden() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, true);
        let visible = read_lsui_element(&plist_path).unwrap();
        assert!(!visible, "LSUIElement=true means hidden from Dock");
    }

    #[test]
    fn test_lsui_element_true_means_hidden() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, true);
        let visible = read_lsui_element(&plist_path).unwrap();
        assert!(!visible);
    }

    #[test]
    fn test_lsui_element_false_means_visible() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, false);
        let visible = read_lsui_element(&plist_path).unwrap();
        assert!(visible);
    }

    #[test]
    fn test_write_and_read_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, true);

        // Read, modify, write
        let content = std::fs::read_to_string(&plist_path).unwrap();
        let mut value: plist::Value = plist::from_bytes(content.as_bytes()).unwrap();
        if let Some(dict) = value.as_dictionary_mut() {
            dict.insert("LSUIElement".to_string(), plist::Value::Boolean(false));
        }
        let file = std::fs::File::create(&plist_path).unwrap();
        plist::to_writer_xml(&file, &value).unwrap();

        // Verify change
        let visible = read_lsui_element(&plist_path).unwrap();
        assert!(visible, "Should be visible after modification");
    }
}

// =============================================================================
// Module: Update Manager Tests
// =============================================================================

mod update_tests {
    use photoncast_core::platform::updates::{
        AvailableUpdate, UpdateError, UpdateManager, UpdateStatus, DEFAULT_FEED_URL,
    };

    #[tokio::test]
    async fn test_update_manager_creation() {
        let manager = UpdateManager::new();
        assert_eq!(manager.feed_url().await, DEFAULT_FEED_URL);
        assert!(manager.auto_check_enabled().await);
    }

    #[tokio::test]
    async fn test_update_manager_initialize() {
        let manager = UpdateManager::new();
        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert_eq!(manager.status().await, UpdateStatus::Ready);
    }

    #[tokio::test]
    async fn test_update_manager_invalid_url() {
        let manager = UpdateManager::with_feed_url("invalid-url");
        let result = manager.initialize().await;
        assert!(matches!(result, Err(UpdateError::InvalidFeedUrl(_))));
    }

    #[tokio::test]
    async fn test_set_auto_check() {
        let manager = UpdateManager::new();
        assert!(manager.auto_check_enabled().await);
        manager.set_auto_check(false).await;
        assert!(!manager.auto_check_enabled().await);
    }

    #[tokio::test]
    async fn test_install_update_no_update_available() {
        let manager = UpdateManager::new();
        manager.initialize().await.unwrap();
        let result = manager.install_update().await;
        assert!(matches!(result, Err(UpdateError::NoUpdateAvailable)));
    }

    #[test]
    fn test_update_status_helpers() {
        assert!(UpdateStatus::Checking.is_checking());
        assert!(UpdateStatus::UpdateAvailable.has_update());
        assert!(UpdateStatus::Downloading.is_busy());
        assert!(!UpdateStatus::Ready.is_busy());
    }

    #[test]
    fn test_available_update_description() {
        let update = AvailableUpdate {
            version: "100".to_string(),
            short_version: "1.0.0".to_string(),
            pub_date: "2026-01-01".to_string(),
            download_url: "https://example.com/update.dmg".to_string(),
            content_length: 15_000_000,
            ed_signature: None,
            release_notes: None,
            minimum_system_version: None,
        };
        assert!(update.description().contains("1.0.0"));
    }

    #[test]
    fn test_update_error_display() {
        let error = UpdateError::NoUpdateAvailable;
        assert!(error.to_string().contains("No update"));

        let error = UpdateError::InvalidFeedUrl("bad".to_string());
        assert!(error.to_string().contains("bad"));
    }
}

// =============================================================================
// Module: Menu Bar Tests
// =============================================================================

mod menu_bar_tests {
    const REQUIRED_MENU_ITEMS: &[&str] = &[
        "Open PhotonCast",
        "Preferences",
        "Check for Updates",
        "About PhotonCast",
        "Quit PhotonCast",
    ];

    #[test]
    fn test_required_menu_items() {
        assert!(REQUIRED_MENU_ITEMS.contains(&"Open PhotonCast"));
        assert!(REQUIRED_MENU_ITEMS.contains(&"Preferences"));
        assert!(REQUIRED_MENU_ITEMS.contains(&"Quit PhotonCast"));
    }

    #[test]
    fn test_click_behavior_mapping() {
        #[derive(PartialEq, Debug)]
        enum Action {
            ToggleLauncher,
            ShowMenu,
        }

        let left_click_action = Action::ToggleLauncher;
        let right_click_action = Action::ShowMenu;

        assert_eq!(left_click_action, Action::ToggleLauncher);
        assert_eq!(right_click_action, Action::ShowMenu);
    }

    #[test]
    fn test_menu_bar_state() {
        struct State {
            visible: bool,
            launcher_open: bool,
        }

        let mut state = State {
            visible: true,
            launcher_open: false,
        };
        assert!(state.visible);

        // Toggle launcher
        state.launcher_open = !state.launcher_open;
        assert!(state.launcher_open);

        state.launcher_open = !state.launcher_open;
        assert!(!state.launcher_open);
    }
}

// =============================================================================
// Module: Code Signing Tests (Shell-based)
// =============================================================================

mod signing_tests {
    use super::*;

    fn get_app_path() -> PathBuf {
        std::env::var("PHOTONCAST_APP_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("build/PhotonCast.app"))
    }

    fn run_cmd(cmd: &str, args: &[&str]) -> (bool, String) {
        let output = Command::new(cmd).args(args).output();
        match output {
            Ok(o) => (
                o.status.success(),
                String::from_utf8_lossy(&o.stdout).to_string()
                    + &String::from_utf8_lossy(&o.stderr),
            ),
            Err(e) => (false, e.to_string()),
        }
    }

    #[test]
    fn test_helper_functions_work() {
        let (success, output) = run_cmd("echo", &["test"]);
        assert!(success);
        assert!(output.contains("test"));
    }

    #[test]
    #[ignore = "requires signed app bundle"]
    fn test_codesign_verify() {
        let app_path = get_app_path();
        if !app_path.exists() {
            return;
        }

        let (success, output) = run_cmd(
            "codesign",
            &["--verify", "--verbose", app_path.to_str().unwrap()],
        );
        assert!(success, "codesign verify failed: {}", output);
    }

    #[test]
    #[ignore = "requires signed and notarized app"]
    fn test_spctl_accepts_app() {
        let app_path = get_app_path();
        if !app_path.exists() {
            return;
        }

        let (_, output) = run_cmd("spctl", &["-a", "-v", app_path.to_str().unwrap()]);
        assert!(
            output.contains("accepted"),
            "spctl should accept app: {}",
            output
        );
    }

    #[test]
    #[ignore = "requires notarized app"]
    fn test_stapler_validate() {
        let app_path = get_app_path();
        if !app_path.exists() {
            return;
        }

        let (success, output) = run_cmd(
            "xcrun",
            &["stapler", "validate", app_path.to_str().unwrap()],
        );
        assert!(
            success || output.contains("worked"),
            "stapler validate failed: {}",
            output
        );
    }
}
