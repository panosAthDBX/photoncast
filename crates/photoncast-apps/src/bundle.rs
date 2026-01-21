//! App bundle detection and information extraction.

use crate::error::{AppError, Result};
use crate::models::Application;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use plist::Value;

/// Reads application information from a .app bundle.
///
/// # Errors
///
/// Returns an error if:
/// - The Info.plist file cannot be read
/// - The plist is malformed
/// - Required fields are missing
pub fn read_bundle_info(app_path: &Path) -> Result<Application> {
    if !app_path.exists() {
        return Err(AppError::AppNotFound(app_path.display().to_string()));
    }

    // Read Info.plist
    let plist_path = app_path.join("Contents/Info.plist");
    if !plist_path.exists() {
        return Err(AppError::Plist(format!(
            "Info.plist not found in {}",
            app_path.display()
        )));
    }

    #[cfg(target_os = "macos")]
    {
        parse_plist(app_path, &plist_path)
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Fallback implementation without plist parsing
        let name = app_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let size_bytes = calculate_directory_size(app_path)?;

        Ok(Application {
            bundle_id: format!("com.unknown.{}", name.to_lowercase().replace(' ', "")),
            name,
            path: app_path.to_path_buf(),
            version: None,
            size_bytes,
            icon_path: find_app_icon(app_path),
        })
    }
}

#[cfg(target_os = "macos")]
fn parse_plist(app_path: &Path, plist_path: &Path) -> Result<Application> {
    // Parse the plist file
    let plist_data = std::fs::read(plist_path)?;
    let plist: Value =
        plist::from_bytes(&plist_data).map_err(|e| AppError::Plist(e.to_string()))?;

    // Extract values from plist
    let dict = plist
        .as_dictionary()
        .ok_or_else(|| AppError::Plist("Info.plist is not a dictionary".to_string()))?;

    // Get bundle identifier (required)
    let bundle_id = dict
        .get("CFBundleIdentifier")
        .and_then(Value::as_string)
        .map(String::from)
        .ok_or_else(|| AppError::Plist("Missing CFBundleIdentifier".to_string()))?;

    // Get display name (with fallbacks)
    let name = dict
        .get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(Value::as_string)
        .map_or_else(
            || {
                app_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            },
            String::from,
        );

    // Get version
    let version = dict
        .get("CFBundleShortVersionString")
        .or_else(|| dict.get("CFBundleVersion"))
        .and_then(Value::as_string)
        .map(String::from);

    // Calculate bundle size
    let size_bytes = calculate_directory_size(app_path)?;

    // Find icon
    let icon_path = dict
        .get("CFBundleIconFile")
        .and_then(Value::as_string)
        .and_then(|icon_name| {
            let resources = app_path.join("Contents/Resources");
            // Try with .icns extension
            let icns_path = resources.join(format!("{}.icns", icon_name));
            if icns_path.exists() {
                return Some(icns_path);
            }
            // Try without extension (it might already have it)
            let direct_path = resources.join(icon_name);
            if direct_path.exists() {
                return Some(direct_path);
            }
            None
        })
        .or_else(|| find_app_icon(app_path));

    Ok(Application {
        bundle_id,
        name,
        path: app_path.to_path_buf(),
        version,
        size_bytes,
        icon_path,
    })
}

/// Calculates the total size of a directory recursively.
pub(crate) fn calculate_directory_size(path: &Path) -> Result<u64> {
    let mut total = 0;

    if path.is_file() {
        return Ok(path.metadata()?.len());
    }

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                total += metadata.len();
            } else if metadata.is_dir() {
                total += calculate_directory_size(&entry.path())?;
            }
        }
    }

    Ok(total)
}

/// Finds the application icon file.
fn find_app_icon(app_path: &Path) -> Option<PathBuf> {
    // Look for .icns file in Contents/Resources
    let resources_path = app_path.join("Contents/Resources");
    if !resources_path.exists() {
        return None;
    }

    // Try to find any .icns file (prefer AppIcon.icns)
    let mut icns_files: Vec<_> = std::fs::read_dir(&resources_path)
        .ok()?
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("icns"))
        })
        .collect();

    // Sort to prefer AppIcon.icns
    icns_files.sort_by_key(|e| {
        let name = e.file_name().to_string_lossy().to_lowercase();
        i32::from(!(name.contains("appicon") || name == "app.icns"))
    });

    icns_files.first().map(std::fs::DirEntry::path)
}

/// Checks if an application is a system app that should be protected from uninstallation.
#[must_use]
pub fn is_system_app(app_path: &Path) -> bool {
    // Protect apps in /System/Applications or /System/Library
    app_path.starts_with("/System/Applications") || app_path.starts_with("/System/Library")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_system_app() {
        assert!(is_system_app(Path::new("/System/Applications/Safari.app")));
        assert!(is_system_app(Path::new(
            "/System/Library/CoreServices/Finder.app"
        )));
        assert!(!is_system_app(Path::new("/Applications/Safari.app")));
        assert!(!is_system_app(Path::new(
            "/Users/test/Applications/MyApp.app"
        )));
    }

    #[test]
    fn test_format_bytes() {
        use crate::models::UninstallPreview;

        assert_eq!(UninstallPreview::format_bytes(500), "500 bytes");
        assert_eq!(UninstallPreview::format_bytes(1536), "1.50 KB");
        assert_eq!(UninstallPreview::format_bytes(1_572_864), "1.50 MB");
        assert_eq!(UninstallPreview::format_bytes(1_610_612_736), "1.50 GB");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_read_finder_bundle() {
        let finder_path = Path::new("/System/Library/CoreServices/Finder.app");
        if finder_path.exists() {
            let result = read_bundle_info(finder_path);
            assert!(
                result.is_ok(),
                "Failed to read Finder bundle: {:?}",
                result.err()
            );
            let app = result.unwrap();
            assert_eq!(app.bundle_id, "com.apple.finder");
            assert!(!app.name.is_empty());
        }
    }
}
