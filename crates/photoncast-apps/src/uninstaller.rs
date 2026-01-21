//! App uninstaller with preview and deep scan support.

use crate::bundle::{is_system_app, read_bundle_info};
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
/// - The app is a protected system app
/// - File scanning fails
pub fn create_uninstall_preview(
    app_path: &Path,
    include_deep_scan: bool,
) -> Result<UninstallPreview> {
    // Check if system app
    if is_system_app(app_path) {
        return Err(AppError::SystemAppProtection(
            app_path.display().to_string(),
        ));
    }

    // Read app info
    let app = read_bundle_info(app_path)?;

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
    let url = unsafe { NSURL::fileURLWithPath(&ns_path) };

    // Get the default file manager
    let file_manager = unsafe { NSFileManager::defaultManager() };

    // Use trashItemAtURL to move to Trash
    // This returns the new URL in Trash if successful
    let result = unsafe { file_manager.trashItemAtURL_resultingItemURL_error(&url, None) };

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

    #[test]
    fn test_system_app_protection() {
        let result = create_uninstall_preview(Path::new("/System/Applications/Safari.app"), true);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::SystemAppProtection(_)
        ));
    }

    #[test]
    fn test_preview_format() {
        // Test that UninstallPreview::format_bytes works correctly
        use crate::models::UninstallPreview;

        assert_eq!(UninstallPreview::format_bytes(0), "0 bytes");
        assert_eq!(UninstallPreview::format_bytes(1024), "1.00 KB");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_move_nonexistent_to_trash() {
        let result = move_to_trash(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
    }
}
