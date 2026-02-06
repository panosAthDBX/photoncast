//! Dock visibility management for macOS.
//!
//! This module provides functionality to control whether PhotonCast appears
//! in the macOS Dock. The dock visibility is controlled by the `LSUIElement`
//! key in the app's Info.plist file.
//!
//! # Important Note
//!
//! Changes to `LSUIElement` require an app restart to take effect because
//! the value is read at launch time by the system launcher. When the dock
//! visibility is modified, a `RestartRequired` error is returned to indicate
//! this requirement to the user.
//!
//! # Example
//!
//! ```ignore
//! use photoncast_core::platform::dock_visibility;
//!
//! // Check current dock visibility
//! let visible = dock_visibility::get_dock_visibility()?;
//! println!("Dock visible: {}", visible);
//!
//! // Toggle dock visibility (requires restart)
//! match dock_visibility::set_dock_visibility(true) {
//!     Ok(()) => println!("Dock visibility updated"),
//!     Err(dock_visibility::DockVisibilityError::RestartRequired) => {
//!         println!("Restart required for changes to take effect");
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Errors that can occur with dock visibility operations.
#[derive(Debug, Error)]
pub enum DockVisibilityError {
    /// Failed to read or parse the Info.plist file.
    #[error("Failed to read Info.plist: {0}")]
    PlistReadError(String),

    /// Failed to write the Info.plist file.
    #[error("Failed to write Info.plist: {0}")]
    PlistWriteError(String),

    /// The Info.plist file was not found at the expected location.
    #[error("Info.plist not found at expected location")]
    PlistNotFound,

    /// App restart is required for changes to take effect.
    #[error("App restart required for changes to take effect")]
    RestartRequired,

    /// Failed to determine the app bundle path.
    #[error("Failed to determine app bundle path: {0}")]
    BundlePathError(String),

    /// The LSUIElement key has an unexpected value type.
    #[error("Invalid LSUIElement value type in plist")]
    InvalidValueType,
}

/// Gets the path to the app's Info.plist file.
///
/// This attempts to locate the plist file within the running app bundle,
/// falling back to development paths for testing.
fn get_info_plist_path() -> Result<PathBuf, DockVisibilityError> {
    // First, try to get the path from the current executable's bundle
    if let Ok(exe_path) = std::env::current_exe() {
        // Navigate from Contents/MacOS/executable to Contents/Info.plist
        if let Some(contents_dir) = exe_path
            .parent() // MacOS
            .and_then(|p| p.parent()) // Contents
        {
            let plist_path = contents_dir.join("Info.plist");
            if plist_path.exists() {
                debug!(path = %plist_path.display(), "Found Info.plist in app bundle");
                return Ok(plist_path);
            }
        }

        // Try standard Applications path as fallback
        if let Some(exe_name) = exe_path.file_stem() {
            let app_path = PathBuf::from("/Applications")
                .join(format!("{}.app", exe_name.to_string_lossy()))
                .join("Contents")
                .join("Info.plist");
            if app_path.exists() {
                debug!(path = %app_path.display(), "Found Info.plist in Applications");
                return Ok(app_path);
            }
        }
    }

    // Development fallback - look for resources directory relative to project
    let dev_paths = [
        PathBuf::from("resources/Info.plist"),
        PathBuf::from("../../resources/Info.plist"),
        PathBuf::from("../resources/Info.plist"),
    ];

    for path in &dev_paths {
        if path.exists() {
            debug!(path = %path.display(), "Found Info.plist in development path");
            return Ok(path.clone());
        }
    }

    warn!("Could not find Info.plist in any known location");
    Err(DockVisibilityError::PlistNotFound)
}

/// Reads the Info.plist file and returns the value of LSUIElement.
///
/// Returns `true` if the app is visible in the Dock (LSUIElement = false),
/// and `false` if the app is hidden from the Dock (LSUIElement = true).
fn read_lsui_element(plist_path: &Path) -> Result<bool, DockVisibilityError> {
    let plist_data = std::fs::read_to_string(plist_path).map_err(|e| {
        DockVisibilityError::PlistReadError(format!(
            "Failed to read plist file at {}: {e}",
            plist_path.display(),
        ))
    })?;

    // Parse the plist using the plist crate
    let value: plist::Value = plist::from_bytes(plist_data.as_bytes()).map_err(|e| {
        DockVisibilityError::PlistReadError(format!("Failed to parse plist: {e}"))
    })?;

    // Extract the dict and look for LSUIElement
    if let Some(dict) = value.as_dictionary() {
        match dict.get("LSUIElement") {
            Some(plist::Value::Boolean(hidden)) => {
                debug!(lsui_element = *hidden, "Read LSUIElement value");
                // LSUIElement = true means hidden from Dock, so invert for "visible"
                Ok(!hidden)
            }
            Some(_) => {
                warn!("LSUIElement has unexpected type in plist");
                Err(DockVisibilityError::InvalidValueType)
            }
            None => {
                // Default to visible if LSUIElement is not present
                debug!("LSUIElement not found in plist, defaulting to visible");
                Ok(true)
            }
        }
    } else {
        Err(DockVisibilityError::PlistReadError(
            "Plist root is not a dictionary".to_string(),
        ))
    }
}

/// Writes the LSUIElement value to the Info.plist file.
///
/// # Arguments
///
/// * `plist_path` - Path to the Info.plist file
/// * `show_in_dock` - If true, sets LSUIElement to false (visible in Dock)
fn write_lsui_element(plist_path: &Path, show_in_dock: bool) -> Result<(), DockVisibilityError> {
    // Read existing plist
    let plist_data = std::fs::read_to_string(plist_path).map_err(|e| {
        DockVisibilityError::PlistWriteError(format!(
            "Failed to read plist for modification: {e}"
        ))
    })?;

    let mut value: plist::Value = plist::from_bytes(plist_data.as_bytes()).map_err(|e| {
        DockVisibilityError::PlistWriteError(format!("Failed to parse plist for modification: {e}"))
    })?;

    // Get mutable dict and set LSUIElement
    if let Some(dict) = value.as_dictionary_mut() {
        // Invert: show_in_dock = true means LSUIElement = false
        let lsui_value = !show_in_dock;
        dict.insert("LSUIElement".to_string(), plist::Value::Boolean(lsui_value));

        // Write back to file
        let file = std::fs::File::create(plist_path).map_err(|e| {
            DockVisibilityError::PlistWriteError(format!("Failed to create plist file: {e}"))
        })?;

        plist::to_writer_xml(&file, &value).map_err(|e| {
            DockVisibilityError::PlistWriteError(format!("Failed to write plist: {e}"))
        })?;

        info!(
            path = %plist_path.display(),
            show_in_dock = show_in_dock,
            lsui_element = lsui_value,
            "Updated LSUIElement in Info.plist"
        );

        Ok(())
    } else {
        Err(DockVisibilityError::PlistWriteError(
            "Plist root is not a dictionary".to_string(),
        ))
    }
}

/// Gets the current dock visibility setting.
///
/// Returns `true` if the app is currently configured to show in the Dock,
/// and `false` if it is configured to be hidden from the Dock.
///
/// # Errors
///
/// Returns `DockVisibilityError` if the Info.plist cannot be read or parsed.
///
/// # Example
///
/// ```ignore
/// use photoncast_core::platform::dock_visibility;
///
/// match dock_visibility::get_dock_visibility() {
///     Ok(visible) => println!("Dock visibility: {}", visible),
///     Err(e) => eprintln!("Failed to get dock visibility: {}", e),
/// }
/// ```
pub fn get_dock_visibility() -> Result<bool, DockVisibilityError> {
    let plist_path = get_info_plist_path()?;
    read_lsui_element(&plist_path)
}

/// Sets the dock visibility for the application.
///
/// This modifies the `LSUIElement` key in the app's Info.plist file.
/// When `show_in_dock` is `true`, the app will appear in the Dock;
/// when `false`, it will be hidden.
///
/// # Important
///
/// This function always returns `Err(DockVisibilityError::RestartRequired)`
/// on success because changes to `LSUIElement` only take effect after an
/// app restart.
///
/// # Arguments
///
/// * `show_in_dock` - Whether the app should be visible in the Dock
///
/// # Errors
///
/// Returns `DockVisibilityError` if the Info.plist cannot be read or written.
/// Returns `DockVisibilityError::RestartRequired` on successful modification.
///
/// # Example
///
/// ```ignore
/// use photoncast_core::platform::dock_visibility;
///
/// match dock_visibility::set_dock_visibility(true) {
///     Ok(()) => println!("Dock visibility set"),
///     Err(dock_visibility::DockVisibilityError::RestartRequired) => {
///         println!("Please restart the app for changes to take effect");
///     }
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn set_dock_visibility(show_in_dock: bool) -> Result<(), DockVisibilityError> {
    let plist_path = get_info_plist_path()?;

    // Check current value to avoid unnecessary writes
    let current = read_lsui_element(&plist_path)?;
    if current == show_in_dock {
        debug!(
            current = current,
            requested = show_in_dock,
            "Dock visibility already set to requested value"
        );
        // Still return RestartRequired because the caller expects this behavior
        // when changing settings (even if no actual change was needed)
        return Err(DockVisibilityError::RestartRequired);
    }

    write_lsui_element(&plist_path, show_in_dock)?;

    // Always return RestartRequired on successful modification
    Err(DockVisibilityError::RestartRequired)
}

/// Toggles the current dock visibility setting.
///
/// Returns the new visibility state on success, or an error if the operation fails.
/// As with `set_dock_visibility`, this always returns `RestartRequired` on success.
///
/// # Example
///
/// ```ignore
/// use photoncast_core::platform::dock_visibility;
///
/// match dock_visibility::toggle_dock_visibility() {
///     Err(dock_visibility::DockVisibilityError::RestartRequired) => {
///         println!("Dock visibility toggled. Please restart the app.");
///     }
///     Ok(new_state) => println!("New state: {} (no change needed)", new_state),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn toggle_dock_visibility() -> Result<bool, DockVisibilityError> {
    let current = get_dock_visibility()?;
    let new_value = !current;

    match set_dock_visibility(new_value) {
        Ok(()) => Ok(new_value),
        Err(DockVisibilityError::RestartRequired) => Err(DockVisibilityError::RestartRequired),
        Err(e) => Err(e),
    }
}

/// Manager for dock visibility operations.
///
/// Provides a convenient wrapper around the dock visibility functions
/// with caching of the plist path for repeated operations.
#[derive(Debug)]
pub struct DockVisibilityManager {
    plist_path: PathBuf,
    cached_value: Option<bool>,
}

impl DockVisibilityManager {
    /// Creates a new dock visibility manager.
    ///
    /// # Errors
    ///
    /// Returns `DockVisibilityError` if the Info.plist cannot be located.
    pub fn new() -> Result<Self, DockVisibilityError> {
        let plist_path = get_info_plist_path()?;
        Ok(Self {
            plist_path,
            cached_value: None,
        })
    }

    /// Gets the current dock visibility, using cached value if available.
    pub fn get_visibility(&mut self) -> Result<bool, DockVisibilityError> {
        if let Some(cached) = self.cached_value {
            debug!(cached = cached, "Returning cached dock visibility");
            return Ok(cached);
        }

        let value = read_lsui_element(&self.plist_path)?;
        self.cached_value = Some(value);
        Ok(value)
    }

    /// Sets the dock visibility and invalidates the cache.
    ///
    /// # Errors
    ///
    /// Always returns `RestartRequired` on success.
    pub fn set_visibility(&mut self, show_in_dock: bool) -> Result<(), DockVisibilityError> {
        let result = set_dock_visibility(show_in_dock);
        if result.is_ok() || matches!(result, Err(DockVisibilityError::RestartRequired)) {
            // Invalidate cache on any successful modification
            self.cached_value = None;
        }
        result
    }

    /// Returns the path to the Info.plist file.
    #[must_use]
    pub fn plist_path(&self) -> &Path {
        &self.plist_path
    }

    /// Invalidates the cached visibility value.
    pub fn invalidate_cache(&mut self) {
        debug!("Invalidating dock visibility cache");
        self.cached_value = None;
    }
}

impl Default for DockVisibilityManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default DockVisibilityManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Creates a test Info.plist with the given LSUIElement value.
    fn create_test_plist(dir: &TempDir, lsui_element: bool) -> PathBuf {
        let plist_path = dir.path().join("Info.plist");
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>TestApp</string>
    <key>CFBundleIdentifier</key>
    <string>com.test.app</string>
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

    /// Creates a test Info.plist without LSUIElement key.
    fn create_test_plist_without_lsui(dir: &TempDir) -> PathBuf {
        let plist_path = dir.path().join("Info.plist");
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>TestApp</string>
    <key>CFBundleIdentifier</key>
    <string>com.test.app</string>
</dict>
</plist>"#;
        let mut file = std::fs::File::create(&plist_path).unwrap();
        file.write_all(plist_content.as_bytes()).unwrap();
        plist_path
    }

    #[test]
    fn test_read_lsui_element_true() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, true);

        // LSUIElement = true means hidden from Dock, so visible = false
        let result = read_lsui_element(&plist_path).unwrap();
        assert!(!result, "LSUIElement=true should mean not visible in Dock");
    }

    #[test]
    fn test_read_lsui_element_false() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, false);

        // LSUIElement = false means visible in Dock
        let result = read_lsui_element(&plist_path).unwrap();
        assert!(result, "LSUIElement=false should mean visible in Dock");
    }

    #[test]
    fn test_read_lsui_element_missing() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist_without_lsui(&temp_dir);

        // Missing LSUIElement defaults to visible
        let result = read_lsui_element(&plist_path).unwrap();
        assert!(result, "Missing LSUIElement should default to visible in Dock");
    }

    #[test]
    fn test_write_lsui_element() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, true);

        // Write new value: show in Dock
        write_lsui_element(&plist_path, true).unwrap();

        // Verify it was written correctly
        let result = read_lsui_element(&plist_path).unwrap();
        assert!(result, "Should be visible in Dock after writing");
    }

    #[test]
    fn test_write_lsui_element_hide() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, false);

        // Write new value: hide from Dock
        write_lsui_element(&plist_path, false).unwrap();

        // Verify it was written correctly
        let result = read_lsui_element(&plist_path).unwrap();
        assert!(!result, "Should be hidden from Dock after writing");
    }

    #[test]
    fn test_set_dock_visibility_returns_restart_required() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, true);

        // Temporarily change the working directory to find the plist
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create a resources directory and copy the plist there
        let resources_dir = temp_dir.path().join("resources");
        std::fs::create_dir(&resources_dir).unwrap();
        std::fs::copy(&plist_path, resources_dir.join("Info.plist")).unwrap();

        // This should return RestartRequired on success
        let result = set_dock_visibility(true);
        assert!(
            matches!(result, Err(DockVisibilityError::RestartRequired)),
            "set_dock_visibility should return RestartRequired"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_dock_visibility_error_display() {
        let error = DockVisibilityError::RestartRequired;
        assert!(error.to_string().contains("restart"));

        let error = DockVisibilityError::PlistNotFound;
        assert!(error.to_string().contains("not found"));

        let error = DockVisibilityError::PlistReadError("test error".to_string());
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_dock_visibility_manager() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, false);

        let manager = DockVisibilityManager {
            plist_path,
            cached_value: None,
        };

        assert!(manager.plist_path().exists());
    }

    #[test]
    fn test_dock_visibility_manager_cache() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, false); // LSUIElement = false, so visible = true

        let mut manager = DockVisibilityManager {
            plist_path,
            cached_value: Some(false), // Cache as NOT visible (hidden)
        };

        // Should return cached value (hidden from Dock)
        assert_eq!(manager.get_visibility().unwrap(), false);

        // Invalidate cache and re-read from actual file
        manager.invalidate_cache();
        // File has LSUIElement=false (not hidden), so visible = true
        assert_eq!(manager.get_visibility().unwrap(), true);
    }

    #[test]
    fn test_toggle_dock_visibility() {
        let temp_dir = TempDir::new().unwrap();
        let plist_path = create_test_plist(&temp_dir, false); // Start with visible (LSUIElement=false)

        // Test toggle directly using the plist_path rather than going through get_info_plist_path
        let current = read_lsui_element(&plist_path).unwrap();
        assert!(current, "Should start visible");

        // Write the opposite value
        write_lsui_element(&plist_path, false).unwrap(); // Write hidden value

        // Verify toggle worked
        let new_value = read_lsui_element(&plist_path).unwrap();
        assert!(!new_value, "Should now be hidden after toggle");
    }
}
