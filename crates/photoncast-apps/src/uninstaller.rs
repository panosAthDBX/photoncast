//! App uninstaller with preview and deep scan support.

use crate::bundle::{is_protected_app, is_system_app, read_bundle_info};
use crate::error::{AppError, Result};
use crate::models::{RelatedFile, UninstallPreview};
use crate::scanner::scan_related_files;
use std::path::Path;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSFileManager, NSString, NSURL};

/// Creates an uninstall preview showing what will be removed.
///
/// # Errors
///
/// Returns an error if:
/// - The app bundle cannot be read
/// - The app is a protected system app (in /System/* or com.apple.* bundle ID)
/// - File scanning fails
pub fn create_uninstall_preview(
    app_path: &Path,
    include_deep_scan: bool,
) -> Result<UninstallPreview> {
    // Check if system app (path-based check first for quick rejection)
    if is_system_app(app_path) {
        return Err(AppError::SystemAppProtection(
            app_path.display().to_string(),
        ));
    }

    // Read app info
    let app = read_bundle_info(app_path)?;

    // Check bundle ID-based protection (e.g., com.apple.* apps in /Applications)
    if is_protected_app(app_path, Some(&app.bundle_id)) {
        return Err(AppError::SystemAppProtection(format!(
            "{} ({})",
            app_path.display(),
            app.bundle_id
        )));
    }

    // Scan for related files if deep scan is enabled
    let related_files = if include_deep_scan {
        scan_related_files(&app)?
    } else {
        Vec::new()
    };

    // Calculate total size
    let total_size = app.size_bytes + related_files.iter().map(|f| f.size_bytes).sum::<u64>();

    let space_freed_formatted = UninstallPreview::format_bytes(total_size);

    Ok(UninstallPreview {
        app,
        related_files,
        total_size,
        space_freed_formatted,
    })
}

/// Performs the uninstall by moving files to Trash.
///
/// This function takes explicit selection of related files to remove.
///
/// # Errors
///
/// Returns an error if:
/// - Moving files to Trash fails
/// - Permission is denied
pub fn uninstall(preview: &UninstallPreview, selected_files: &[&RelatedFile]) -> Result<()> {
    // Move app to Trash
    move_to_trash(&preview.app.path)?;

    // Move selected related files to Trash
    for file in selected_files {
        move_to_trash(&file.path)?;
    }

    Ok(())
}

/// Performs the uninstall by moving files to Trash, respecting the `selected` field.
///
/// This function automatically filters related files by their `selected` field,
/// only removing files where `selected == true`.
///
/// # Errors
///
/// Returns an error if:
/// - Moving files to Trash fails
/// - Permission is denied
pub fn uninstall_selected(preview: &UninstallPreview) -> Result<()> {
    // Move app to Trash
    move_to_trash(&preview.app.path)?;

    // Move only selected related files to Trash
    for file in &preview.related_files {
        if file.selected {
            move_to_trash(&file.path)?;
        }
    }

    Ok(())
}

/// Returns only the related files that are marked as selected.
///
/// This is a convenience function to filter files by their `selected` field.
#[must_use]
pub fn get_selected_files(preview: &UninstallPreview) -> Vec<&RelatedFile> {
    preview
        .related_files
        .iter()
        .filter(|f| f.selected)
        .collect()
}

/// Calculates the total size of selected files only.
///
/// Returns the app size plus the size of all related files where `selected == true`.
#[must_use]
pub fn calculate_selected_size(preview: &UninstallPreview) -> u64 {
    preview.app.size_bytes
        + preview
            .related_files
            .iter()
            .filter(|f| f.selected)
            .map(|f| f.size_bytes)
            .sum::<u64>()
}

/// Moves a file or directory to the Trash.
///
/// Uses the macOS Trash system rather than permanent deletion for safety.
///
/// # Errors
///
/// Returns an error if the move operation fails.
#[cfg(target_os = "macos")]
fn move_to_trash(path: &Path) -> Result<()> {
    tracing::info!("Moving to Trash: {}", path.display());

    // Convert path to NSURL
    let path_str = path.to_string_lossy();
    let ns_path = NSString::from_str(&path_str);
    let url = NSURL::fileURLWithPath(&ns_path);

    // Get the default file manager
    let file_manager = NSFileManager::defaultManager();

    // Use trashItemAtURL to move to Trash
    // This returns the new URL in Trash if successful
    let result = file_manager.trashItemAtURL_resultingItemURL_error(&url, None);

    match result {
        Ok(()) => {
            tracing::info!("Successfully moved to Trash: {}", path.display());
            Ok(())
        },
        Err(error) => {
            let error_msg = error.localizedDescription().to_string();
            tracing::error!(
                "Failed to move to Trash: {} - {}",
                path.display(),
                error_msg
            );
            Err(AppError::Io(std::io::Error::other(format!(
                "Failed to move to Trash: {}",
                error_msg
            ))))
        },
    }
}

#[cfg(not(target_os = "macos"))]
fn move_to_trash(path: &Path) -> Result<()> {
    tracing::warn!("Trash integration only available on macOS");
    tracing::info!("Would move to Trash: {}", path.display());

    // On non-macOS, we could use the `trash` crate as a fallback
    // For now, return an error
    Err(AppError::Message {
        message: format!(
            "Trash functionality only available on macOS. Would remove: {}",
            path.display()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{is_system_app_by_bundle_id, is_protected_app, format_size};
    use crate::models::{Application, RelatedFileCategory};
    use std::path::PathBuf;

    #[test]
    fn test_system_app_protection() {
        // Test create_uninstall_preview rejects system apps
        let result = create_uninstall_preview(Path::new("/System/Applications/Safari.app"), true);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::SystemAppProtection(_)
        ));

        // Test is_system_app_by_bundle_id
        assert!(is_system_app_by_bundle_id("com.apple.finder"));
        assert!(is_system_app_by_bundle_id("com.apple.Safari"));
        assert!(is_system_app_by_bundle_id("com.apple.mail"));
        assert!(!is_system_app_by_bundle_id("com.example.app"));
        assert!(!is_system_app_by_bundle_id("com.spotify.client"));
        assert!(!is_system_app_by_bundle_id("org.mozilla.firefox"));

        // Test is_protected_app for /System/Applications/*
        assert!(is_protected_app(
            Path::new("/System/Applications/Calculator.app"),
            None
        ));
        assert!(is_protected_app(
            Path::new("/System/Applications/Safari.app"),
            Some("com.apple.Safari")
        ));

        // Non-system apps should not be protected
        assert!(!is_protected_app(
            Path::new("/Applications/Slack.app"),
            Some("com.tinyspeck.slackmacgap")
        ));
    }

    #[test]
    fn test_format_size() {
        // Test exact values specified in task
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");

        // Additional edge cases
        assert_eq!(format_size(0), "0 bytes");
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1023), "1023 bytes");
        assert_eq!(format_size(1536), "1.50 KB"); // 1.5 KB
        assert_eq!(format_size(1_572_864), "1.50 MB"); // 1.5 MB
        assert_eq!(format_size(1_610_612_736), "1.50 GB"); // 1.5 GB
    }

    #[test]
    fn test_preview_format() {
        // Test that UninstallPreview::format_bytes works correctly
        use crate::models::UninstallPreview;

        assert_eq!(UninstallPreview::format_bytes(0), "0 bytes");
        assert_eq!(UninstallPreview::format_bytes(1024), "1.00 KB");
    }

    #[test]
    fn test_group_container_detection() {
        use crate::scanner::find_group_containers;

        // Test with a known app path that likely doesn't have group containers
        // This tests the function can run without errors even for non-existent apps
        let result = find_group_containers(
            "com.nonexistent.testapp",
            Path::new("/Applications/NonExistent.app"),
        );
        assert!(result.is_ok());

        // For a non-existent app, should return empty list
        let containers = result.unwrap();
        // Result may be empty or contain matches if there happen to be
        // group containers with similar names
        assert!(containers.iter().all(|c| c.category == RelatedFileCategory::GroupContainers));
    }

    #[test]
    fn test_selected_files_filtering() {
        // Create a mock UninstallPreview with some selected and some unselected files
        let app = Application {
            name: "TestApp".to_string(),
            bundle_id: "com.test.app".to_string(),
            path: PathBuf::from("/Applications/TestApp.app"),
            version: Some("1.0".to_string()),
            size_bytes: 1000,
            icon_path: None,
        };

        let related_files = vec![
            RelatedFile {
                path: PathBuf::from("/Library/Caches/com.test.app"),
                size_bytes: 500,
                category: RelatedFileCategory::Caches,
                selected: true,
            },
            RelatedFile {
                path: PathBuf::from("/Library/Preferences/com.test.app.plist"),
                size_bytes: 100,
                category: RelatedFileCategory::Preferences,
                selected: false, // Not selected
            },
            RelatedFile {
                path: PathBuf::from("/Library/Application Support/TestApp"),
                size_bytes: 2000,
                category: RelatedFileCategory::ApplicationSupport,
                selected: true,
            },
            RelatedFile {
                path: PathBuf::from("/Library/Logs/TestApp"),
                size_bytes: 50,
                category: RelatedFileCategory::Logs,
                selected: false, // Not selected
            },
        ];

        let preview = UninstallPreview {
            app,
            related_files,
            total_size: 3650, // 1000 + 500 + 100 + 2000 + 50
            space_freed_formatted: "3.65 KB".to_string(),
        };

        // Test get_selected_files
        let selected = get_selected_files(&preview);
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|f| f.selected));
        assert!(selected
            .iter()
            .any(|f| f.category == RelatedFileCategory::Caches));
        assert!(selected
            .iter()
            .any(|f| f.category == RelatedFileCategory::ApplicationSupport));

        // Test calculate_selected_size
        // Should be: app (1000) + Caches (500) + ApplicationSupport (2000) = 3500
        let selected_size = calculate_selected_size(&preview);
        assert_eq!(selected_size, 3500);
    }

    #[test]
    fn test_related_file_categories() {
        // Verify all categories have display names, especially the new ones
        let categories = [
            RelatedFileCategory::ApplicationSupport,
            RelatedFileCategory::Preferences,
            RelatedFileCategory::Caches,
            RelatedFileCategory::Logs,
            RelatedFileCategory::SavedState,
            RelatedFileCategory::Containers,
            RelatedFileCategory::Cookies,
            RelatedFileCategory::WebKit,
            RelatedFileCategory::HTTPStorages,
            RelatedFileCategory::GroupContainers,
        ];

        for category in &categories {
            let display_name = category.display_name();
            assert!(!display_name.is_empty(), "Category {:?} has empty display name", category);
        }

        // Verify specific display names for new categories
        assert_eq!(RelatedFileCategory::Cookies.display_name(), "Cookies");
        assert_eq!(RelatedFileCategory::WebKit.display_name(), "WebKit Data");
        assert_eq!(RelatedFileCategory::HTTPStorages.display_name(), "HTTP Storages");
        assert_eq!(RelatedFileCategory::GroupContainers.display_name(), "Group Containers");

        // Verify existing categories still have correct names
        assert_eq!(RelatedFileCategory::ApplicationSupport.display_name(), "Application Support");
        assert_eq!(RelatedFileCategory::Preferences.display_name(), "Preferences");
        assert_eq!(RelatedFileCategory::Caches.display_name(), "Caches");
        assert_eq!(RelatedFileCategory::Logs.display_name(), "Logs");
        assert_eq!(RelatedFileCategory::SavedState.display_name(), "Saved Application State");
        assert_eq!(RelatedFileCategory::Containers.display_name(), "Containers");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_move_nonexistent_to_trash() {
        let result = move_to_trash(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
    }

    // =========================================================================
    // Task 8.8: Additional Selection Persistence Tests
    // =========================================================================

    #[test]
    fn test_uninstall_respects_all_selected() {
        // When all files are selected, all should be included
        let app = Application {
            name: "TestApp".to_string(),
            bundle_id: "com.test.allselected".to_string(),
            path: PathBuf::from("/Applications/TestApp.app"),
            version: Some("1.0".to_string()),
            size_bytes: 1000,
            icon_path: None,
        };

        let related_files = vec![
            RelatedFile {
                path: PathBuf::from("/Library/Caches/test1"),
                size_bytes: 100,
                category: RelatedFileCategory::Caches,
                selected: true,
            },
            RelatedFile {
                path: PathBuf::from("/Library/Caches/test2"),
                size_bytes: 200,
                category: RelatedFileCategory::Preferences,
                selected: true,
            },
            RelatedFile {
                path: PathBuf::from("/Library/Caches/test3"),
                size_bytes: 300,
                category: RelatedFileCategory::Logs,
                selected: true,
            },
        ];

        let preview = UninstallPreview {
            app,
            related_files,
            total_size: 1600,
            space_freed_formatted: "1.60 KB".to_string(),
        };

        let selected = get_selected_files(&preview);
        assert_eq!(selected.len(), 3, "All 3 files should be selected");
        
        let size = calculate_selected_size(&preview);
        assert_eq!(size, 1600, "Size should include all files");
    }

    #[test]
    fn test_uninstall_respects_none_selected() {
        // When no files are selected, only app is included in size
        let app = Application {
            name: "TestApp".to_string(),
            bundle_id: "com.test.noneselected".to_string(),
            path: PathBuf::from("/Applications/TestApp.app"),
            version: Some("1.0".to_string()),
            size_bytes: 1000,
            icon_path: None,
        };

        let related_files = vec![
            RelatedFile {
                path: PathBuf::from("/Library/Caches/test1"),
                size_bytes: 100,
                category: RelatedFileCategory::Caches,
                selected: false,
            },
            RelatedFile {
                path: PathBuf::from("/Library/Caches/test2"),
                size_bytes: 200,
                category: RelatedFileCategory::Preferences,
                selected: false,
            },
        ];

        let preview = UninstallPreview {
            app,
            related_files,
            total_size: 1300,
            space_freed_formatted: "1.30 KB".to_string(),
        };

        let selected = get_selected_files(&preview);
        assert_eq!(selected.len(), 0, "No files should be selected");
        
        let size = calculate_selected_size(&preview);
        assert_eq!(size, 1000, "Size should only include app");
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

        assert!(file.selected, "File should be selected initially");

        // Toggle off
        file.selected = false;
        assert!(!file.selected, "File should be deselected");

        // Toggle on
        file.selected = true;
        assert!(file.selected, "File should be selected again");
    }

    #[test]
    fn test_selection_preserves_file_properties() {
        // Verify that toggling selection doesn't affect other properties
        let file = RelatedFile {
            path: PathBuf::from("/Library/Caches/com.test.app"),
            size_bytes: 12345,
            category: RelatedFileCategory::Caches,
            selected: true,
        };

        // Verify properties
        assert_eq!(file.path, PathBuf::from("/Library/Caches/com.test.app"));
        assert_eq!(file.size_bytes, 12345);
        assert_eq!(file.category, RelatedFileCategory::Caches);

        // Create a new file with selection toggled
        let toggled = RelatedFile {
            selected: false,
            ..file.clone()
        };

        // All other properties should remain the same
        assert_eq!(toggled.path, PathBuf::from("/Library/Caches/com.test.app"));
        assert_eq!(toggled.size_bytes, 12345);
        assert_eq!(toggled.category, RelatedFileCategory::Caches);
        assert!(!toggled.selected);
    }
}
