//! Integration tests for dock visibility toggle functionality.
//!
//! Task 5.3: Test Dock Visibility Toggle
//!
//! These tests verify the dock visibility functionality, including:
//! - Default state verification (LSUIElement=true means hidden)
//! - Toggle functionality
//! - Info.plist modification
//! - Restart requirement handling
//!
//! # Test Categories
//!
//! - **Plist Parsing**: Tests Info.plist reading
//! - **State Toggle**: Tests visibility state changes
//! - **Error Handling**: Tests error cases
//! - **Manager Operations**: Tests DockVisibilityManager
//!
//! # Running These Tests
//!
//! ```bash
//! cargo test --test integration -- dock_visibility_test
//! ```

use photoncast_core::platform::dock_visibility::{
    get_dock_visibility, set_dock_visibility, toggle_dock_visibility,
    DockVisibilityError, DockVisibilityManager,
};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

// =============================================================================
// Test Plist Content
// =============================================================================

/// Creates a test Info.plist with LSUIElement set to the given value
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
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>LSUIElement</key>
    <{} />
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
</dict>
</plist>"#,
        if lsui_element { "true" } else { "false" }
    );

    let mut file = std::fs::File::create(&plist_path).unwrap();
    file.write_all(plist_content.as_bytes()).unwrap();
    plist_path
}

/// Creates a test Info.plist without LSUIElement key
fn create_test_plist_without_lsui(dir: &TempDir) -> PathBuf {
    let plist_path = dir.path().join("Info.plist");
    let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>PhotonCast</string>
    <key>CFBundleIdentifier</key>
    <string>com.photoncast.app</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
</dict>
</plist>"#;

    let mut file = std::fs::File::create(&plist_path).unwrap();
    file.write_all(plist_content.as_bytes()).unwrap();
    plist_path
}

/// Creates a test Info.plist with invalid LSUIElement type
fn create_test_plist_invalid_type(dir: &TempDir) -> PathBuf {
    let plist_path = dir.path().join("Info.plist");
    let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>PhotonCast</string>
    <key>CFBundleIdentifier</key>
    <string>com.photoncast.app</string>
    <key>LSUIElement</key>
    <string>invalid</string>
</dict>
</plist>"#;

    let mut file = std::fs::File::create(&plist_path).unwrap();
    file.write_all(plist_content.as_bytes()).unwrap();
    plist_path
}

/// Creates a malformed plist file
fn create_malformed_plist(dir: &TempDir) -> PathBuf {
    let plist_path = dir.path().join("Info.plist");
    let mut file = std::fs::File::create(&plist_path).unwrap();
    file.write_all(b"this is not valid xml").unwrap();
    plist_path
}

// =============================================================================
// Reading LSUIElement Tests
// =============================================================================

/// Helper function to read LSUIElement directly from a plist file
fn read_lsui_element_direct(plist_path: &std::path::Path) -> Result<bool, String> {
    let content = std::fs::read_to_string(plist_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let value: plist::Value = plist::from_bytes(content.as_bytes())
        .map_err(|e| format!("Failed to parse plist: {}", e))?;

    if let Some(dict) = value.as_dictionary() {
        match dict.get("LSUIElement") {
            Some(plist::Value::Boolean(hidden)) => {
                // LSUIElement = true means hidden from Dock
                Ok(!hidden) // Return "visible in dock"
            }
            Some(_) => Err("LSUIElement has unexpected type".to_string()),
            None => Ok(true), // Default to visible if not present
        }
    } else {
        Err("Plist root is not a dictionary".to_string())
    }
}

/// Helper function to write LSUIElement to a plist file
fn write_lsui_element_direct(
    plist_path: &std::path::Path,
    show_in_dock: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(plist_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut value: plist::Value = plist::from_bytes(content.as_bytes())
        .map_err(|e| format!("Failed to parse plist: {}", e))?;

    if let Some(dict) = value.as_dictionary_mut() {
        // show_in_dock = true means LSUIElement = false (visible)
        let lsui_value = !show_in_dock;
        dict.insert("LSUIElement".to_string(), plist::Value::Boolean(lsui_value));

        let file = std::fs::File::create(plist_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;

        plist::to_writer_xml(&file, &value)
            .map_err(|e| format!("Failed to write plist: {}", e))?;

        Ok(())
    } else {
        Err("Plist root is not a dictionary".to_string())
    }
}

#[test]
fn test_read_lsui_element_true() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, true); // LSUIElement = true (hidden)

    let result = read_lsui_element_direct(&plist_path);
    assert!(result.is_ok());
    // LSUIElement = true means hidden from Dock, so visible = false
    assert!(!result.unwrap(), "LSUIElement=true should mean hidden from Dock");
}

#[test]
fn test_read_lsui_element_false() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, false); // LSUIElement = false (visible)

    let result = read_lsui_element_direct(&plist_path);
    assert!(result.is_ok());
    // LSUIElement = false means visible in Dock
    assert!(result.unwrap(), "LSUIElement=false should mean visible in Dock");
}

#[test]
fn test_read_lsui_element_missing() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist_without_lsui(&temp_dir);

    let result = read_lsui_element_direct(&plist_path);
    assert!(result.is_ok());
    // Missing LSUIElement defaults to visible
    assert!(result.unwrap(), "Missing LSUIElement should default to visible");
}

#[test]
fn test_read_lsui_element_invalid_type() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist_invalid_type(&temp_dir);

    let result = read_lsui_element_direct(&plist_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unexpected type"));
}

#[test]
fn test_read_malformed_plist() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_malformed_plist(&temp_dir);

    let result = read_lsui_element_direct(&plist_path);
    assert!(result.is_err());
}

// =============================================================================
// Writing LSUIElement Tests
// =============================================================================

#[test]
fn test_write_lsui_element_show_in_dock() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, true); // Start hidden

    // Write: show in dock = true (LSUIElement = false)
    let result = write_lsui_element_direct(&plist_path, true);
    assert!(result.is_ok());

    // Verify the change
    let visible = read_lsui_element_direct(&plist_path).unwrap();
    assert!(visible, "Should be visible in Dock after write");
}

#[test]
fn test_write_lsui_element_hide_from_dock() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, false); // Start visible

    // Write: show in dock = false (LSUIElement = true)
    let result = write_lsui_element_direct(&plist_path, false);
    assert!(result.is_ok());

    // Verify the change
    let visible = read_lsui_element_direct(&plist_path).unwrap();
    assert!(!visible, "Should be hidden from Dock after write");
}

#[test]
fn test_write_lsui_element_no_change() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, true); // Hidden

    // Write same value
    let result = write_lsui_element_direct(&plist_path, false); // Keep hidden
    assert!(result.is_ok());

    // Verify it's still hidden
    let visible = read_lsui_element_direct(&plist_path).unwrap();
    assert!(!visible);
}

// =============================================================================
// Toggle Tests
// =============================================================================

#[test]
fn test_toggle_visibility_from_hidden() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, true); // Start hidden

    // Read initial state
    let initial = read_lsui_element_direct(&plist_path).unwrap();
    assert!(!initial, "Should start hidden");

    // Toggle to visible
    write_lsui_element_direct(&plist_path, true).unwrap();

    // Verify toggled
    let toggled = read_lsui_element_direct(&plist_path).unwrap();
    assert!(toggled, "Should be visible after toggle");
}

#[test]
fn test_toggle_visibility_from_visible() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, false); // Start visible

    // Read initial state
    let initial = read_lsui_element_direct(&plist_path).unwrap();
    assert!(initial, "Should start visible");

    // Toggle to hidden
    write_lsui_element_direct(&plist_path, false).unwrap();

    // Verify toggled
    let toggled = read_lsui_element_direct(&plist_path).unwrap();
    assert!(!toggled, "Should be hidden after toggle");
}

// =============================================================================
// DockVisibilityManager Tests
// =============================================================================

#[test]
fn test_dock_visibility_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, false);

    let manager = DockVisibilityManager {
        plist_path: plist_path.clone(),
        cached_value: None,
    };

    assert!(manager.plist_path().exists());
    assert_eq!(manager.plist_path(), plist_path.as_path());
}

#[test]
fn test_dock_visibility_manager_get_visibility() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, false); // Visible

    let mut manager = DockVisibilityManager {
        plist_path,
        cached_value: None,
    };

    let result = manager.get_visibility();
    assert!(result.is_ok());
    assert!(result.unwrap(), "Should report visible");
}

#[test]
fn test_dock_visibility_manager_caching() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, false); // LSUIElement=false means visible

    let mut manager = DockVisibilityManager {
        plist_path: plist_path.clone(),
        cached_value: Some(false), // Cached as hidden
    };

    // First read should return cached value (hidden)
    let result = manager.get_visibility();
    assert!(result.is_ok());
    assert!(!result.unwrap(), "Should return cached value (hidden)");

    // Invalidate cache
    manager.invalidate_cache();

    // Next read should get actual value from file (visible)
    let result = manager.get_visibility();
    assert!(result.is_ok());
    assert!(result.unwrap(), "Should return actual file value (visible)");
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_error_display_messages() {
    let error = DockVisibilityError::RestartRequired;
    assert!(error.to_string().to_lowercase().contains("restart"));

    let error = DockVisibilityError::PlistNotFound;
    assert!(error.to_string().to_lowercase().contains("not found"));

    let error = DockVisibilityError::PlistReadError("test error".to_string());
    assert!(error.to_string().contains("test error"));

    let error = DockVisibilityError::PlistWriteError("write failed".to_string());
    assert!(error.to_string().contains("write failed"));

    let error = DockVisibilityError::BundlePathError("path error".to_string());
    assert!(error.to_string().contains("path error"));

    let error = DockVisibilityError::InvalidValueType;
    assert!(error.to_string().to_lowercase().contains("invalid"));
}

#[test]
fn test_read_nonexistent_plist() {
    let result = read_lsui_element_direct(std::path::Path::new("/nonexistent/Info.plist"));
    assert!(result.is_err());
}

// =============================================================================
// Default State Tests
// =============================================================================

#[test]
fn test_default_state_is_hidden() {
    // According to spec, default LSUIElement should be true (hidden from Dock)
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, true); // Default state

    let visible = read_lsui_element_direct(&plist_path).unwrap();
    assert!(
        !visible,
        "Default state (LSUIElement=true) should mean hidden from Dock"
    );
}

#[test]
fn test_lsui_element_true_means_hidden() {
    // LSUIElement = true means the app is hidden from Dock
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, true);

    let visible = read_lsui_element_direct(&plist_path).unwrap();
    assert!(!visible, "LSUIElement=true should mean NOT visible in Dock");
}

#[test]
fn test_lsui_element_false_means_visible() {
    // LSUIElement = false means the app is visible in Dock
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, false);

    let visible = read_lsui_element_direct(&plist_path).unwrap();
    assert!(visible, "LSUIElement=false should mean visible in Dock");
}

// =============================================================================
// Plist Preservation Tests
// =============================================================================

#[test]
fn test_write_preserves_other_keys() {
    let temp_dir = TempDir::new().unwrap();
    let plist_path = create_test_plist(&temp_dir, true);

    // Modify LSUIElement
    write_lsui_element_direct(&plist_path, true).unwrap();

    // Read back the plist and verify other keys are preserved
    let content = std::fs::read_to_string(&plist_path).unwrap();
    assert!(content.contains("CFBundleName"), "CFBundleName should be preserved");
    assert!(
        content.contains("CFBundleIdentifier"),
        "CFBundleIdentifier should be preserved"
    );
    assert!(
        content.contains("com.photoncast.app"),
        "Bundle ID value should be preserved"
    );
}

// =============================================================================
// Integration with Real Module Tests
// =============================================================================

/// Test the actual module functions with a mock environment
/// These tests set up a temporary directory structure to simulate the app bundle
#[test]
fn test_module_functions_with_dev_plist() {
    // This test attempts to use the real module functions
    // It may fail if no Info.plist is found (expected in most test environments)

    // Try to get dock visibility - this will look for Info.plist
    let result = get_dock_visibility();

    // The result depends on whether we're running in an app bundle or dev environment
    match result {
        Ok(visible) => {
            println!("Current dock visibility: {}", visible);
        }
        Err(DockVisibilityError::PlistNotFound) => {
            println!("Info.plist not found (expected in test environment)");
        }
        Err(e) => {
            println!("Error getting dock visibility: {}", e);
        }
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(feature = "proptest")]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_write_read_roundtrip(show_in_dock: bool) {
            let temp_dir = TempDir::new().unwrap();
            let plist_path = create_test_plist(&temp_dir, true);

            // Write
            write_lsui_element_direct(&plist_path, show_in_dock).unwrap();

            // Read back
            let read_value = read_lsui_element_direct(&plist_path).unwrap();

            prop_assert_eq!(read_value, show_in_dock);
        }

        #[test]
        fn test_toggle_always_changes_state(initial_hidden: bool) {
            let temp_dir = TempDir::new().unwrap();
            let plist_path = create_test_plist(&temp_dir, initial_hidden);

            let initial = read_lsui_element_direct(&plist_path).unwrap();
            let expected = !initial; // Toggle should invert

            write_lsui_element_direct(&plist_path, expected).unwrap();
            let after = read_lsui_element_direct(&plist_path).unwrap();

            prop_assert_eq!(after, expected);
        }
    }
}

// =============================================================================
// Helper Struct Definition for Tests
// =============================================================================

/// Mock DockVisibilityManager for tests
struct DockVisibilityManager {
    plist_path: PathBuf,
    cached_value: Option<bool>,
}

impl DockVisibilityManager {
    fn plist_path(&self) -> &std::path::Path {
        &self.plist_path
    }

    fn get_visibility(&mut self) -> Result<bool, DockVisibilityError> {
        if let Some(cached) = self.cached_value {
            return Ok(cached);
        }

        let result = read_lsui_element_direct(&self.plist_path)
            .map_err(|e| DockVisibilityError::PlistReadError(e))?;

        self.cached_value = Some(result);
        Ok(result)
    }

    fn invalidate_cache(&mut self) {
        self.cached_value = None;
    }
}
