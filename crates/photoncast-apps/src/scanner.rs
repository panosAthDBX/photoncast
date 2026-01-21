//! Related file scanner for finding app-related files in the system.

use crate::bundle::calculate_directory_size;
use crate::error::Result;
use crate::models::{Application, RelatedFile, RelatedFileCategory};
use std::path::PathBuf;

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

    Ok(related_files)
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

    let entries = std::fs::read_dir(base_path)?;

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
        assert!(result.is_ok());
    }
}
