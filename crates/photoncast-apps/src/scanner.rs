//! Related file scanner for finding app-related files in the system.

use crate::bundle::calculate_directory_size;
use crate::error::Result;
use crate::models::{Application, RelatedFile, RelatedFileCategory};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Scans for related files for an application.
///
/// This performs a "deep scan" to find all files related to the application
/// in common macOS locations.
///
/// # Errors
///
/// Returns an error if directory traversal fails.
pub fn scan_related_files(app: &Application) -> Result<Vec<RelatedFile>> {
    let mut related_files = Vec::new();

    // Get user's home directory
    let home_dir = dirs::home_dir().ok_or_else(|| crate::error::AppError::Message {
        message: "Could not determine home directory".to_string(),
    })?;

    // Extract app name without .app extension for matching
    let app_name = app
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&app.name);

    // Scan Application Support
    scan_location(
        &home_dir.join("Library/Application Support"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::ApplicationSupport,
        &mut related_files,
    )?;

    // Scan Preferences
    scan_location(
        &home_dir.join("Library/Preferences"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::Preferences,
        &mut related_files,
    )?;

    // Scan Caches
    scan_location(
        &home_dir.join("Library/Caches"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::Caches,
        &mut related_files,
    )?;

    // Scan Logs
    scan_location(
        &home_dir.join("Library/Logs"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::Logs,
        &mut related_files,
    )?;

    // Scan Saved Application State
    scan_location(
        &home_dir.join("Library/Saved Application State"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::SavedState,
        &mut related_files,
    )?;

    // Scan Containers
    scan_location(
        &home_dir.join("Library/Containers"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::Containers,
        &mut related_files,
    )?;

    // Scan Cookies
    scan_location(
        &home_dir.join("Library/Cookies"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::Cookies,
        &mut related_files,
    )?;

    // Scan WebKit
    scan_location(
        &home_dir.join("Library/WebKit"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::WebKit,
        &mut related_files,
    )?;

    // Scan HTTPStorages
    scan_location(
        &home_dir.join("Library/HTTPStorages"),
        app_name,
        &app.bundle_id,
        RelatedFileCategory::HTTPStorages,
        &mut related_files,
    )?;

    // Scan Group Containers (using entitlements-based detection)
    let group_containers = find_group_containers(&app.bundle_id, &app.path)?;
    related_files.extend(group_containers);

    Ok(related_files)
}

/// Finds group containers associated with an application.
///
/// This reads the app's entitlements to find group identifiers from
/// `com.apple.security.application-groups` and matches them against
/// directories in `~/Library/Group Containers/`.
///
/// # Arguments
///
/// * `bundle_id` - The bundle identifier of the application
/// * `app_path` - Path to the .app bundle
///
/// # Returns
///
/// A vector of `RelatedFile` entries for matching group containers.
pub fn find_group_containers(bundle_id: &str, app_path: &Path) -> Result<Vec<RelatedFile>> {
    let mut group_files = Vec::new();

    // Get user's home directory
    let home_dir = dirs::home_dir().ok_or_else(|| crate::error::AppError::Message {
        message: "Could not determine home directory".to_string(),
    })?;

    let group_containers_path = home_dir.join("Library/Group Containers");
    if !group_containers_path.exists() {
        return Ok(group_files);
    }

    // Get group identifiers from app entitlements
    let group_ids = extract_group_identifiers(app_path);

    // Also check for containers that match the bundle ID pattern
    // (some apps use group containers without explicit entitlements)
    let mut all_group_ids = group_ids;
    all_group_ids.push(bundle_id.to_string());

    // Scan Group Containers directory
    if let Ok(entries) = std::fs::read_dir(&group_containers_path) {
        for entry in entries.filter_map(std::result::Result::ok) {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Check if this container matches any of our group identifiers
            let matches = all_group_ids.iter().any(|group_id| {
                // Group containers are named like "group.com.developer.app" or
                // just the group ID directly
                file_name == *group_id
                    || file_name.starts_with(&format!("{}.", group_id))
                    || file_name.ends_with(&format!(".{}", group_id))
                    || file_name.contains(group_id)
            });

            if matches && path.is_dir() {
                let size_bytes = calculate_directory_size(&path).unwrap_or(0);
                group_files.push(RelatedFile {
                    path,
                    size_bytes,
                    category: RelatedFileCategory::GroupContainers,
                    selected: true,
                });
            }
        }
    }

    Ok(group_files)
}

/// Extracts group identifiers from an app's entitlements using codesign.
///
/// Runs `codesign -d --entitlements -` to get the app's entitlements and
/// parses the `com.apple.security.application-groups` array.
fn extract_group_identifiers(app_path: &Path) -> Vec<String> {
    let mut group_ids = Vec::new();

    // Try to get entitlements using codesign
    let output = Command::new("codesign")
        .args(["-d", "--entitlements", "-", "--xml"])
        .arg(app_path)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return group_ids,
    };

    // Parse the XML plist output
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find the application-groups section
    // The output is in XML plist format, we'll do simple string parsing
    // to avoid heavy dependencies
    if let Some(start_idx) = stdout.find("com.apple.security.application-groups") {
        // Find the array that follows
        if let Some(array_start) = stdout[start_idx..].find("<array>") {
            let search_start = start_idx + array_start;
            if let Some(array_end) = stdout[search_start..].find("</array>") {
                let array_content = &stdout[search_start..search_start + array_end];

                // Extract string values from the array safely
                for line in array_content.lines() {
                    let line = line.trim();
                    if let Some(value) = line
                        .strip_prefix("<string>")
                        .and_then(|s| s.strip_suffix("</string>"))
                    {
                        if !value.is_empty() {
                            group_ids.push(value.to_string());
                        }
                    }
                }
            }
        }
    }

    group_ids
}

/// Scans a specific location for related files.
///
/// Uses conservative matching - only matches exact bundle ID or app name.
fn scan_location(
    base_path: &PathBuf,
    app_name: &str,
    bundle_id: &str,
    category: RelatedFileCategory,
    related_files: &mut Vec<RelatedFile>,
) -> Result<()> {
    if !base_path.exists() {
        return Ok(());
    }

    // Handle permission errors gracefully (common on macOS without Full Disk Access)
    let entries = match std::fs::read_dir(base_path) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            // Skip directories we don't have permission to read
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Conservative matching: exact bundle ID or app name
        let matches = file_name.contains(bundle_id)
            || file_name.contains(app_name)
            || (category == RelatedFileCategory::Preferences
                && file_name.starts_with(&format!("{}.", bundle_id)));

        if matches {
            // Calculate size
            let size_bytes = if path.is_file() {
                entry.metadata()?.len()
            } else {
                calculate_directory_size(&path)?
            };

            related_files.push(RelatedFile {
                path,
                size_bytes,
                category,
                selected: true,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_related_files_empty() {
        let app = Application {
            name: "NonExistentApp".to_string(),
            bundle_id: "com.nonexistent.app".to_string(),
            path: PathBuf::from("/Applications/NonExistentApp.app"),
            version: None,
            size_bytes: 0,
            icon_path: None,
        };

        // Should not error, just return empty list
        let result = scan_related_files(&app);
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
    }
}
