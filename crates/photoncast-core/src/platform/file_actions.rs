//! File actions for file search.
//!
//! This module provides file management operations such as copy, move, delete,
//! rename, duplicate, compress, and get info. These actions integrate with
//! macOS native APIs for proper system behavior.
//!
//! # Example
//!
//! ```no_run
//! use photoncast_core::platform::file_actions::{
//!     copy_file_to_clipboard, move_to_trash, get_file_info,
//! };
//! use std::path::Path;
//!
//! // Copy a file to clipboard
//! if let Err(e) = copy_file_to_clipboard(Path::new("/Users/me/doc.pdf")) {
//!     eprintln!("Failed: {}", e.user_message());
//! }
//!
//! // Move to trash
//! match move_to_trash(Path::new("/Users/me/old_file.txt")) {
//!     Ok(trash_url) => println!("Moved to: {:?}", trash_url),
//!     Err(e) => eprintln!("Failed: {}", e.user_message()),
//! }
//! ```

#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::time::SystemTime;

use thiserror::Error;
use tracing::{debug, warn};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during file actions.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileActionError {
    /// The file or folder was not found.
    #[error("file not found: {path}")]
    NotFound {
        /// The path that was not found.
        path: String,
    },

    /// Permission denied for the operation.
    #[error("permission denied: {path}")]
    PermissionDenied {
        /// The path that access was denied for.
        path: String,
    },

    /// Invalid filename (contains forbidden characters).
    #[error("invalid filename '{name}': {reason}")]
    InvalidFilename {
        /// The invalid filename.
        name: String,
        /// The reason it's invalid.
        reason: String,
    },

    /// A file or folder already exists at the destination.
    #[error("file already exists: {path}")]
    AlreadyExists {
        /// The destination path that already exists.
        path: String,
    },

    /// The operation failed.
    #[error("operation failed: {operation} - {reason}")]
    OperationFailed {
        /// The operation that failed.
        operation: String,
        /// The reason for failure.
        reason: String,
    },

    /// Clipboard operation failed.
    #[error("clipboard error: {reason}")]
    ClipboardError {
        /// The reason for the clipboard error.
        reason: String,
    },

    /// Compression failed.
    #[error("compression failed: {reason}")]
    CompressionFailed {
        /// The reason compression failed.
        reason: String,
    },

    /// IO error occurred.
    #[error("IO error: {reason}")]
    IoError {
        /// The reason for the IO error.
        reason: String,
    },
}

impl FileActionError {
    /// Returns a user-friendly error message.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::NotFound { path } => {
                format!("The file or folder '{}' doesn't exist", path)
            },
            Self::PermissionDenied { path } => {
                format!(
                    "You don't have permission to modify '{}'",
                    Self::extract_filename(path)
                )
            },
            Self::InvalidFilename { name, reason } => {
                format!("'{}' is not a valid name: {}", name, reason)
            },
            Self::AlreadyExists { path } => {
                format!(
                    "A file named '{}' already exists",
                    Self::extract_filename(path)
                )
            },
            Self::OperationFailed { operation, reason } => {
                format!("Couldn't {}: {}", operation, reason)
            },
            Self::ClipboardError { reason } => {
                format!("Couldn't copy to clipboard: {}", reason)
            },
            Self::CompressionFailed { reason } => {
                format!("Couldn't create archive: {}", reason)
            },
            Self::IoError { reason } => {
                format!("An error occurred: {}", reason)
            },
        }
    }

    /// Returns whether this error is recoverable.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::PermissionDenied { .. }
                | Self::AlreadyExists { .. }
                | Self::ClipboardError { .. }
        )
    }

    /// Returns a suggested action for the error.
    #[must_use]
    pub fn action_hint(&self) -> Option<&'static str> {
        match self {
            Self::NotFound { .. } => Some("Refresh file list"),
            Self::PermissionDenied { .. } => Some("Check permissions"),
            Self::InvalidFilename { .. } => Some("Choose a different name"),
            Self::AlreadyExists { .. } => Some("Choose a different name"),
            Self::OperationFailed { .. } => Some("Retry"),
            Self::ClipboardError { .. } => Some("Retry"),
            Self::CompressionFailed { .. } => Some("Check available space"),
            Self::IoError { .. } => Some("Retry"),
        }
    }

    /// Extracts the filename from a path string.
    fn extract_filename(path: &str) -> &str {
        Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
    }

    /// Creates a not found error.
    pub fn not_found(path: impl Into<String>) -> Self {
        Self::NotFound { path: path.into() }
    }

    /// Creates a permission denied error.
    pub fn permission_denied(path: impl Into<String>) -> Self {
        Self::PermissionDenied { path: path.into() }
    }

    /// Creates an invalid filename error.
    pub fn invalid_filename(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidFilename {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Creates an already exists error.
    pub fn already_exists(path: impl Into<String>) -> Self {
        Self::AlreadyExists { path: path.into() }
    }

    /// Creates an operation failed error.
    pub fn operation_failed(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::OperationFailed {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Creates a clipboard error.
    pub fn clipboard_error(reason: impl Into<String>) -> Self {
        Self::ClipboardError {
            reason: reason.into(),
        }
    }

    /// Creates a compression failed error.
    pub fn compression_failed(reason: impl Into<String>) -> Self {
        Self::CompressionFailed {
            reason: reason.into(),
        }
    }

    /// Creates an IO error.
    pub fn io_error(reason: impl Into<String>) -> Self {
        Self::IoError {
            reason: reason.into(),
        }
    }
}

#[cfg(target_os = "macos")]
impl From<std::io::Error> for FileActionError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => Self::IoError {
                reason: "file not found".to_string(),
            },
            std::io::ErrorKind::PermissionDenied => Self::IoError {
                reason: "permission denied".to_string(),
            },
            std::io::ErrorKind::AlreadyExists => Self::IoError {
                reason: "file already exists".to_string(),
            },
            _ => Self::IoError {
                reason: e.to_string(),
            },
        }
    }
}

/// Result type for file actions.
pub type Result<T> = std::result::Result<T, FileActionError>;

// =============================================================================
// File Info Types
// =============================================================================

/// Information about a file or folder.
#[derive(Debug, Clone, PartialEq)]
pub struct FileInfo {
    /// The full path to the file.
    pub path: PathBuf,
    /// The file name.
    pub name: String,
    /// The file size in bytes.
    pub size: u64,
    /// The file kind (e.g., "PDF Document", "Folder").
    pub kind: String,
    /// The creation time.
    pub created: Option<SystemTime>,
    /// The last modification time.
    pub modified: Option<SystemTime>,
    /// Whether the file is readable.
    pub is_readable: bool,
    /// Whether the file is writable.
    pub is_writable: bool,
    /// Whether the file is executable.
    pub is_executable: bool,
    /// Whether this is a directory.
    pub is_directory: bool,
    /// Number of items (for directories only).
    pub item_count: Option<u64>,
}

/// Information about an application that can open a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// The application bundle ID.
    pub bundle_id: String,
    /// The application name.
    pub name: String,
    /// The path to the application.
    pub path: PathBuf,
}

// =============================================================================
// Filename Validation
// =============================================================================

/// Characters that are not allowed in filenames on macOS.
const INVALID_FILENAME_CHARS: &[char] = &['/', ':', '\0'];

/// Maximum filename length on macOS (HFS+/APFS).
const MAX_FILENAME_LENGTH: usize = 255;

/// Validates a filename for macOS.
///
/// # Arguments
///
/// * `name` - The filename to validate
///
/// # Returns
///
/// `Ok(())` if the filename is valid, or an error describing why it's invalid.
pub fn validate_filename(name: &str) -> Result<()> {
    // Check for empty name
    if name.is_empty() {
        return Err(FileActionError::invalid_filename(
            name,
            "filename cannot be empty",
        ));
    }

    // Check for forbidden characters
    for c in INVALID_FILENAME_CHARS {
        if name.contains(*c) {
            let char_desc = match c {
                '/' => "slash (/)",
                ':' => "colon (:)",
                '\0' => "null character",
                _ => "invalid character",
            };
            return Err(FileActionError::invalid_filename(
                name,
                format!("cannot contain {}", char_desc),
            ));
        }
    }

    // Check for reserved names
    if name == "." || name == ".." {
        return Err(FileActionError::invalid_filename(
            name,
            "reserved name",
        ));
    }

    // Check length (255 bytes for HFS+/APFS)
    if name.len() > MAX_FILENAME_LENGTH {
        return Err(FileActionError::invalid_filename(
            name,
            format!("name too long (max {} characters)", MAX_FILENAME_LENGTH),
        ));
    }

    Ok(())
}

// =============================================================================
// File Actions (macOS only)
// =============================================================================

/// Copies a file to the clipboard.
///
/// Uses NSPasteboard to copy the file so it can be pasted in Finder or other apps.
///
/// # Arguments
///
/// * `path` - The path to the file to copy
///
/// # Errors
///
/// Returns an error if the file doesn't exist or clipboard access fails.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::copy_file_to_clipboard;
/// use std::path::Path;
///
/// copy_file_to_clipboard(Path::new("/Users/me/document.pdf")).unwrap();
/// ```
#[cfg(target_os = "macos")]
pub fn copy_file_to_clipboard(path: &Path) -> Result<()> {
    debug!(path = %path.display(), "copying file to clipboard");

    // Verify the file exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    // Use osascript to set the clipboard to the file
    // This properly uses NSPasteboard under the hood
    let script = format!(
        r#"set the clipboard to (POSIX file "{}")"#,
        path.display()
    );

    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| {
            warn!(path = %path.display(), error = %e, "failed to execute osascript");
            FileActionError::clipboard_error(e.to_string())
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(path = %path.display(), stderr = %stderr, "osascript failed");
        return Err(FileActionError::clipboard_error(stderr.trim().to_string()));
    }

    debug!(path = %path.display(), "file copied to clipboard");
    Ok(())
}

/// Moves a file or folder to the Trash.
///
/// Uses NSFileManager's `trashItem(at:resultingItemURL:)` via the `trash` CLI tool
/// or falls back to moving to ~/.Trash.
///
/// # Arguments
///
/// * `path` - The path to the file or folder to trash
///
/// # Returns
///
/// The path where the file was moved in the Trash (for potential undo).
///
/// # Errors
///
/// Returns an error if the file doesn't exist or cannot be moved to trash.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::move_to_trash;
/// use std::path::Path;
///
/// let trash_path = move_to_trash(Path::new("/Users/me/old_file.txt")).unwrap();
/// println!("File moved to: {:?}", trash_path);
/// ```
#[cfg(target_os = "macos")]
pub fn move_to_trash(path: &Path) -> Result<PathBuf> {
    debug!(path = %path.display(), "moving to trash");

    // Verify the file exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    // Use AppleScript to move to trash (uses proper NSFileManager)
    let script = format!(
        r#"tell application "Finder" to delete POSIX file "{}""#,
        path.display()
    );

    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| {
            warn!(path = %path.display(), error = %e, "failed to execute osascript");
            FileActionError::operation_failed("move to trash", e.to_string())
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(path = %path.display(), stderr = %stderr, "trash operation failed");

        // Check for permission error
        if stderr.to_lowercase().contains("permission")
            || stderr.to_lowercase().contains("not allowed")
        {
            return Err(FileActionError::permission_denied(path.display().to_string()));
        }

        return Err(FileActionError::operation_failed(
            "move to trash",
            stderr.trim().to_string(),
        ));
    }

    // Construct the expected trash path
    let filename = path.file_name().unwrap_or_default();
    let trash_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".Trash")
        .join(filename);

    debug!(path = %path.display(), trash_path = %trash_path.display(), "moved to trash");
    Ok(trash_path)
}

/// Permanently deletes a file or folder.
///
/// **Warning:** This operation cannot be undone.
///
/// # Arguments
///
/// * `path` - The path to the file or folder to delete
///
/// # Errors
///
/// Returns an error if the file doesn't exist or cannot be deleted.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::delete_permanently;
/// use std::path::Path;
///
/// delete_permanently(Path::new("/Users/me/temp_file.txt")).unwrap();
/// ```
#[cfg(target_os = "macos")]
pub fn delete_permanently(path: &Path) -> Result<()> {
    debug!(path = %path.display(), "deleting permanently");

    // Verify the file exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    // Check if it's a directory
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| {
            warn!(path = %path.display(), error = %e, "failed to delete directory");
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    FileActionError::permission_denied(path.display().to_string())
                },
                _ => FileActionError::operation_failed("delete", e.to_string()),
            }
        })?;
    } else {
        std::fs::remove_file(path).map_err(|e| {
            warn!(path = %path.display(), error = %e, "failed to delete file");
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    FileActionError::permission_denied(path.display().to_string())
                },
                _ => FileActionError::operation_failed("delete", e.to_string()),
            }
        })?;
    }

    debug!(path = %path.display(), "deleted permanently");
    Ok(())
}

/// Gets a list of applications that can open the specified file.
///
/// Uses NSWorkspace's `urlsForApplications(toOpen:)` via Launch Services.
///
/// # Arguments
///
/// * `path` - The path to the file
///
/// # Returns
///
/// A list of applications that can open the file, with the default app first.
///
/// # Errors
///
/// Returns an error if the file doesn't exist.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::get_apps_for_file;
/// use std::path::Path;
///
/// let apps = get_apps_for_file(Path::new("/Users/me/document.pdf")).unwrap();
/// for app in apps {
///     println!("{}: {}", app.name, app.bundle_id);
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn get_apps_for_file(path: &Path) -> Result<Vec<AppInfo>> {
    debug!(path = %path.display(), "getting apps for file");

    // Verify the file exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    // Use AppleScript to get apps that can open this file type
    // This queries Launch Services
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let script = format!(
        r#"
        use framework "AppKit"
        use scripting additions
        
        set fileURL to current application's NSURL's fileURLWithPath:"{}"
        set workspace to current application's NSWorkspace's sharedWorkspace()
        set appURLs to workspace's URLsForApplicationsToOpenURL:fileURL
        
        set appList to {{}}
        repeat with appURL in appURLs
            set appPath to appURL's |path|() as text
            set appBundle to current application's NSBundle's bundleWithPath:appPath
            if appBundle is not missing value then
                set bundleID to (appBundle's bundleIdentifier()) as text
                set appName to (appBundle's objectForInfoDictionaryKey:"CFBundleName") as text
                if appName is missing value then
                    set appName to (appBundle's objectForInfoDictionaryKey:"CFBundleDisplayName") as text
                end if
                if appName is missing value then
                    tell application "System Events"
                        set appName to name of (appPath as POSIX file)
                    end tell
                    set appName to text 1 thru -5 of appName -- Remove .app
                end if
                set end of appList to bundleID & "|" & appName & "|" & appPath
            end if
        end repeat
        
        set AppleScript's text item delimiters to linefeed
        return appList as text
        "#,
        path.display()
    );

    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| {
            warn!(path = %path.display(), error = %e, "failed to get apps");
            FileActionError::operation_failed("get apps", e.to_string())
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            apps.push(AppInfo {
                bundle_id: parts[0].to_string(),
                name: parts[1].to_string(),
                path: PathBuf::from(parts[2]),
            });
        }
    }

    // If AppleScript failed, try a simpler fallback
    if apps.is_empty() && !extension.is_empty() {
        debug!(extension = %extension, "using fallback to find apps");
        
        // Get default app at least
        let default_script = format!(
            r#"
            tell application "System Events"
                set defaultApp to default application of (POSIX file "{}" as alias)
                set appPath to POSIX path of (defaultApp as alias)
                set appName to name of defaultApp
            end tell
            return appPath & "|" & appName
            "#,
            path.display()
        );

        if let Ok(output) = Command::new("osascript")
            .args(["-e", &default_script])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = stdout.trim().split('|').collect();
            if parts.len() >= 2 {
                // Try to get bundle ID from the path
                let app_path = PathBuf::from(parts[0]);
                let bundle_id = get_bundle_id_from_path(&app_path)
                    .unwrap_or_else(|| "unknown".to_string());
                
                apps.push(AppInfo {
                    bundle_id,
                    name: parts[1].trim_end_matches(".app").to_string(),
                    path: app_path,
                });
            }
        }
    }

    debug!(path = %path.display(), count = apps.len(), "found apps");
    Ok(apps)
}

/// Opens a file with a specific application.
///
/// # Arguments
///
/// * `path` - The path to the file to open
/// * `app_bundle_id` - The bundle ID of the application to open with
///
/// # Errors
///
/// Returns an error if the file doesn't exist or the app cannot open it.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::open_with_app;
/// use std::path::Path;
///
/// open_with_app(Path::new("/Users/me/doc.pdf"), "com.apple.Preview").unwrap();
/// ```
#[cfg(target_os = "macos")]
pub fn open_with_app(path: &Path, app_bundle_id: &str) -> Result<()> {
    debug!(path = %path.display(), app = %app_bundle_id, "opening with app");

    // Verify the file exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    let output = Command::new("open")
        .args(["-b", app_bundle_id, &path.display().to_string()])
        .output()
        .map_err(|e| {
            warn!(path = %path.display(), app = %app_bundle_id, error = %e, "failed to open with app");
            FileActionError::operation_failed("open with app", e.to_string())
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(path = %path.display(), app = %app_bundle_id, stderr = %stderr, "open command failed");
        return Err(FileActionError::operation_failed(
            "open with app",
            stderr.trim().to_string(),
        ));
    }

    debug!(path = %path.display(), app = %app_bundle_id, "opened with app");
    Ok(())
}

/// Renames a file or folder.
///
/// # Arguments
///
/// * `path` - The path to the file or folder to rename
/// * `new_name` - The new name (just the filename, not a path)
///
/// # Returns
///
/// The new path after renaming.
///
/// # Errors
///
/// Returns an error if:
/// - The file doesn't exist
/// - The new name is invalid
/// - A file with the new name already exists
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::rename_file;
/// use std::path::Path;
///
/// let new_path = rename_file(Path::new("/Users/me/old.txt"), "new.txt").unwrap();
/// println!("Renamed to: {:?}", new_path);
/// ```
#[cfg(target_os = "macos")]
pub fn rename_file(path: &Path, new_name: &str) -> Result<PathBuf> {
    debug!(path = %path.display(), new_name = %new_name, "renaming file");

    // Validate the new filename
    validate_filename(new_name)?;

    // Verify the file exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    // Construct new path
    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    let new_path = parent.join(new_name);

    // Check if destination already exists
    if new_path.exists() {
        return Err(FileActionError::already_exists(new_path.display().to_string()));
    }

    // Perform the rename
    std::fs::rename(path, &new_path).map_err(|e| {
        warn!(path = %path.display(), new_name = %new_name, error = %e, "rename failed");
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                FileActionError::permission_denied(path.display().to_string())
            },
            _ => FileActionError::operation_failed("rename", e.to_string()),
        }
    })?;

    debug!(path = %path.display(), new_path = %new_path.display(), "renamed successfully");
    Ok(new_path)
}

/// Moves a file or folder to a destination directory.
///
/// # Arguments
///
/// * `path` - The path to the file or folder to move
/// * `destination` - The destination directory
///
/// # Returns
///
/// The new path after moving.
///
/// # Errors
///
/// Returns an error if:
/// - The source file doesn't exist
/// - The destination directory doesn't exist
/// - A file with the same name already exists at the destination
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::move_file;
/// use std::path::Path;
///
/// let new_path = move_file(
///     Path::new("/Users/me/document.pdf"),
///     Path::new("/Users/me/Documents"),
/// ).unwrap();
/// ```
#[cfg(target_os = "macos")]
pub fn move_file(path: &Path, destination: &Path) -> Result<PathBuf> {
    debug!(path = %path.display(), destination = %destination.display(), "moving file");

    // Verify source exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    // Verify destination directory exists
    if !destination.exists() {
        return Err(FileActionError::not_found(destination.display().to_string()));
    }

    if !destination.is_dir() {
        return Err(FileActionError::operation_failed(
            "move",
            "destination is not a directory",
        ));
    }

    // Construct new path
    let filename = path.file_name().ok_or_else(|| {
        FileActionError::operation_failed("move", "source has no filename")
    })?;
    let new_path = destination.join(filename);

    // Check if destination already exists
    if new_path.exists() {
        return Err(FileActionError::already_exists(new_path.display().to_string()));
    }

    // Perform the move
    std::fs::rename(path, &new_path).map_err(|e| {
        warn!(path = %path.display(), destination = %destination.display(), error = %e, "move failed");
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                FileActionError::permission_denied(path.display().to_string())
            },
            _ => FileActionError::operation_failed("move", e.to_string()),
        }
    })?;

    debug!(path = %path.display(), new_path = %new_path.display(), "moved successfully");
    Ok(new_path)
}

/// Duplicates a file or folder.
///
/// Creates a copy with " copy" appended to the name. If that name exists,
/// appends a number (e.g., "file copy 2").
///
/// # Arguments
///
/// * `path` - The path to the file or folder to duplicate
///
/// # Returns
///
/// The path to the duplicated file.
///
/// # Errors
///
/// Returns an error if the file doesn't exist or cannot be copied.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::duplicate_file;
/// use std::path::Path;
///
/// let copy_path = duplicate_file(Path::new("/Users/me/document.pdf")).unwrap();
/// println!("Created: {:?}", copy_path);
/// ```
#[cfg(target_os = "macos")]
pub fn duplicate_file(path: &Path) -> Result<PathBuf> {
    debug!(path = %path.display(), "duplicating file");

    // Verify source exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = path.extension().and_then(|e| e.to_str());

    // Find a unique name
    let new_path = find_unique_copy_name(parent, stem, extension)?;

    // Copy the file/directory
    if path.is_dir() {
        copy_dir_recursive(path, &new_path)?;
    } else {
        std::fs::copy(path, &new_path).map_err(|e| {
            warn!(path = %path.display(), error = %e, "copy failed");
            FileActionError::operation_failed("duplicate", e.to_string())
        })?;
    }

    debug!(path = %path.display(), new_path = %new_path.display(), "duplicated successfully");
    Ok(new_path)
}

/// Finds a unique name for a copy (e.g., "file copy", "file copy 2").
#[cfg(target_os = "macos")]
fn find_unique_copy_name(parent: &Path, stem: &str, extension: Option<&str>) -> Result<PathBuf> {
    // First try "name copy.ext"
    let make_name = |suffix: &str| {
        if let Some(ext) = extension {
            parent.join(format!("{}{}.{}", stem, suffix, ext))
        } else {
            parent.join(format!("{}{}", stem, suffix))
        }
    };

    let first_try = make_name(" copy");
    if !first_try.exists() {
        return Ok(first_try);
    }

    // Try "name copy 2", "name copy 3", etc.
    for i in 2..=100 {
        let path = make_name(&format!(" copy {}", i));
        if !path.exists() {
            return Ok(path);
        }
    }

    Err(FileActionError::operation_failed(
        "duplicate",
        "too many copies exist",
    ))
}

/// Recursively copies a directory.
#[cfg(target_os = "macos")]
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(|e| {
        FileActionError::operation_failed("create directory", e.to_string())
    })?;

    for entry in std::fs::read_dir(src).map_err(|e| {
        FileActionError::operation_failed("read directory", e.to_string())
    })? {
        let entry = entry.map_err(|e| {
            FileActionError::operation_failed("read entry", e.to_string())
        })?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                FileActionError::operation_failed("copy file", e.to_string())
            })?;
        }
    }

    Ok(())
}

/// Gets detailed information about a file or folder.
///
/// # Arguments
///
/// * `path` - The path to the file or folder
///
/// # Returns
///
/// A `FileInfo` struct with detailed metadata.
///
/// # Errors
///
/// Returns an error if the file doesn't exist.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::get_file_info;
/// use std::path::Path;
///
/// let info = get_file_info(Path::new("/Users/me/document.pdf")).unwrap();
/// println!("Size: {} bytes", info.size);
/// println!("Kind: {}", info.kind);
/// ```
#[cfg(target_os = "macos")]
pub fn get_file_info(path: &Path) -> Result<FileInfo> {
    debug!(path = %path.display(), "getting file info");

    // Verify the file exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    let metadata = std::fs::metadata(path).map_err(|e| {
        FileActionError::operation_failed("get metadata", e.to_string())
    })?;

    let is_directory = metadata.is_dir();

    // Get file kind using mdls (Spotlight metadata)
    let kind = get_file_kind(path).unwrap_or_else(|| {
        if is_directory {
            "Folder".to_string()
        } else {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| format!("{} file", e.to_uppercase()))
                .unwrap_or_else(|| "Document".to_string())
        }
    });

    // Count items for directories
    let item_count = if is_directory {
        std::fs::read_dir(path).ok().map(|entries| entries.count() as u64)
    } else {
        None
    };

    // Get permissions - use a simple approach based on metadata
    // Note: This is a simplified check. For accurate permissions, use access() syscall.
    let is_readable = std::fs::File::open(path).is_ok();
    let is_writable = !metadata.permissions().readonly();
    
    #[cfg(unix)]
    let is_executable = {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        // Check if any execute bit is set
        mode & 0o111 != 0
    };
    
    #[cfg(not(unix))]
    let is_executable = false;

    let info = FileInfo {
        path: path.to_path_buf(),
        name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string(),
        size: metadata.len(),
        kind,
        created: metadata.created().ok(),
        modified: metadata.modified().ok(),
        is_readable,
        is_writable,
        is_executable,
        is_directory,
        item_count,
    };

    debug!(path = %path.display(), size = info.size, kind = %info.kind, "got file info");
    Ok(info)
}

/// Gets the file kind using Spotlight metadata.
#[cfg(target_os = "macos")]
fn get_file_kind(path: &Path) -> Option<String> {
    let output = Command::new("mdls")
        .args(["-name", "kMDItemKind", &path.display().to_string()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse output like: kMDItemKind = "PDF Document"
    stdout
        .lines()
        .next()
        .and_then(|line| {
            line.split('=')
                .nth(1)
                .map(|s| s.trim().trim_matches('"').to_string())
        })
        .filter(|s| s != "(null)")
}

/// Gets the bundle ID from an app path.
#[cfg(target_os = "macos")]
fn get_bundle_id_from_path(path: &Path) -> Option<String> {
    let info_plist = path.join("Contents/Info.plist");
    let contents = std::fs::read(&info_plist).ok()?;
    let plist: plist::Value = plist::from_bytes(&contents).ok()?;
    let dict = plist.as_dictionary()?;
    dict.get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(String::from)
}

/// Compresses a file or folder into a ZIP archive.
///
/// Uses the `ditto` command for proper macOS archive creation.
///
/// # Arguments
///
/// * `path` - The path to the file or folder to compress
///
/// # Returns
///
/// The path to the created archive.
///
/// # Errors
///
/// Returns an error if compression fails.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::platform::file_actions::compress;
/// use std::path::Path;
///
/// let archive_path = compress(Path::new("/Users/me/folder")).unwrap();
/// println!("Created: {:?}", archive_path);
/// ```
#[cfg(target_os = "macos")]
pub fn compress(path: &Path) -> Result<PathBuf> {
    debug!(path = %path.display(), "compressing");

    // Verify source exists
    if !path.exists() {
        return Err(FileActionError::not_found(path.display().to_string()));
    }

    // Create archive name
    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");

    // Find a unique archive name
    let archive_path = find_unique_archive_name(parent, stem)?;

    // Use ditto for proper macOS ZIP creation (preserves metadata, resource forks)
    let output = Command::new("ditto")
        .args([
            "-c",
            "-k",
            "--sequesterRsrc",
            "--keepParent",
            &path.display().to_string(),
            &archive_path.display().to_string(),
        ])
        .output()
        .map_err(|e| {
            warn!(path = %path.display(), error = %e, "ditto failed");
            FileActionError::compression_failed(e.to_string())
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(path = %path.display(), stderr = %stderr, "ditto compression failed");
        return Err(FileActionError::compression_failed(stderr.trim().to_string()));
    }

    debug!(path = %path.display(), archive = %archive_path.display(), "compressed successfully");
    Ok(archive_path)
}

/// Finds a unique name for an archive.
#[cfg(target_os = "macos")]
fn find_unique_archive_name(parent: &Path, stem: &str) -> Result<PathBuf> {
    let first_try = parent.join(format!("{}.zip", stem));
    if !first_try.exists() {
        return Ok(first_try);
    }

    for i in 2..=100 {
        let path = parent.join(format!("{} {}.zip", stem, i));
        if !path.exists() {
            return Ok(path);
        }
    }

    Err(FileActionError::compression_failed("too many archives exist"))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Error Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_error_user_messages() {
        let err = FileActionError::not_found("/path/to/file.txt");
        assert!(err.user_message().contains("doesn't exist"));

        let err = FileActionError::permission_denied("/path/to/file.txt");
        assert!(err.user_message().contains("permission"));

        let err = FileActionError::invalid_filename("bad:name", "contains colon");
        assert!(err.user_message().contains("not a valid name"));

        let err = FileActionError::already_exists("/path/to/file.txt");
        assert!(err.user_message().contains("already exists"));
    }

    #[test]
    fn test_error_is_recoverable() {
        assert!(!FileActionError::not_found("x").is_recoverable());
        assert!(FileActionError::permission_denied("x").is_recoverable());
        assert!(FileActionError::already_exists("x").is_recoverable());
        assert!(FileActionError::clipboard_error("x").is_recoverable());
        assert!(!FileActionError::io_error("x").is_recoverable());
    }

    #[test]
    fn test_error_action_hints() {
        assert_eq!(
            FileActionError::not_found("x").action_hint(),
            Some("Refresh file list")
        );
        assert_eq!(
            FileActionError::permission_denied("x").action_hint(),
            Some("Check permissions")
        );
        assert_eq!(
            FileActionError::invalid_filename("x", "y").action_hint(),
            Some("Choose a different name")
        );
    }

    // -------------------------------------------------------------------------
    // Filename Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_filename_valid() {
        assert!(validate_filename("document.pdf").is_ok());
        assert!(validate_filename("my file.txt").is_ok());
        assert!(validate_filename("file-with-dashes").is_ok());
        assert!(validate_filename("file_with_underscores").is_ok());
        assert!(validate_filename(".hidden").is_ok());
        assert!(validate_filename("UPPERCASE").is_ok());
        assert!(validate_filename("日本語").is_ok());
        assert!(validate_filename("émoji 🎉").is_ok());
    }

    #[test]
    fn test_validate_filename_empty() {
        let result = validate_filename("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FileActionError::InvalidFilename { .. }));
        assert!(err.user_message().contains("empty"));
    }

    #[test]
    fn test_validate_filename_with_slash() {
        let result = validate_filename("path/to/file");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.user_message().contains("slash"));
    }

    #[test]
    fn test_validate_filename_with_colon() {
        let result = validate_filename("file:name");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.user_message().contains("colon"));
    }

    #[test]
    fn test_validate_filename_with_null() {
        let result = validate_filename("file\0name");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.user_message().contains("null"));
    }

    #[test]
    fn test_validate_filename_reserved() {
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("..").is_err());
    }

    #[test]
    fn test_validate_filename_too_long() {
        let long_name = "a".repeat(300);
        let result = validate_filename(&long_name);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.user_message().contains("too long"));
    }

    // -------------------------------------------------------------------------
    // File Action Tests (macOS only, require temp files)
    // -------------------------------------------------------------------------

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;
        use std::fs::File;
        use tempfile::tempdir;

        #[test]
        fn test_get_file_info_file() {
            let temp = tempdir().expect("should create temp dir");
            let file_path = temp.path().join("test.txt");
            std::fs::write(&file_path, "test content").expect("should write");

            let info = get_file_info(&file_path).expect("should get info");
            assert_eq!(info.name, "test.txt");
            assert_eq!(info.size, 12); // "test content"
            assert!(!info.is_directory);
            assert!(info.is_readable);
            assert!(info.item_count.is_none());
        }

        #[test]
        fn test_get_file_info_directory() {
            let temp = tempdir().expect("should create temp dir");
            let dir_path = temp.path().join("subdir");
            std::fs::create_dir(&dir_path).expect("should create dir");

            // Create some files in the directory
            File::create(dir_path.join("file1.txt")).expect("should create");
            File::create(dir_path.join("file2.txt")).expect("should create");

            let info = get_file_info(&dir_path).expect("should get info");
            assert!(info.is_directory);
            assert_eq!(info.item_count, Some(2));
        }

        #[test]
        fn test_get_file_info_not_found() {
            let result = get_file_info(Path::new("/nonexistent/path/file.txt"));
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), FileActionError::NotFound { .. }));
        }

        #[test]
        fn test_rename_file_success() {
            let temp = tempdir().expect("should create temp dir");
            let file_path = temp.path().join("original.txt");
            std::fs::write(&file_path, "content").expect("should write");

            let new_path = rename_file(&file_path, "renamed.txt").expect("should rename");
            assert!(new_path.ends_with("renamed.txt"));
            assert!(new_path.exists());
            assert!(!file_path.exists());
        }

        #[test]
        fn test_rename_file_invalid_name() {
            let temp = tempdir().expect("should create temp dir");
            let file_path = temp.path().join("original.txt");
            std::fs::write(&file_path, "content").expect("should write");

            let result = rename_file(&file_path, "bad/name");
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), FileActionError::InvalidFilename { .. }));
        }

        #[test]
        fn test_rename_file_already_exists() {
            let temp = tempdir().expect("should create temp dir");
            let file1 = temp.path().join("file1.txt");
            let file2 = temp.path().join("file2.txt");
            std::fs::write(&file1, "content1").expect("should write");
            std::fs::write(&file2, "content2").expect("should write");

            let result = rename_file(&file1, "file2.txt");
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), FileActionError::AlreadyExists { .. }));
        }

        #[test]
        fn test_duplicate_file() {
            let temp = tempdir().expect("should create temp dir");
            let file_path = temp.path().join("original.txt");
            std::fs::write(&file_path, "content").expect("should write");

            let copy_path = duplicate_file(&file_path).expect("should duplicate");
            assert!(copy_path.exists());
            assert!(copy_path.to_str().unwrap().contains("copy"));
            assert_eq!(std::fs::read_to_string(&copy_path).unwrap(), "content");
        }

        #[test]
        fn test_duplicate_file_multiple_copies() {
            let temp = tempdir().expect("should create temp dir");
            let file_path = temp.path().join("original.txt");
            std::fs::write(&file_path, "content").expect("should write");

            let copy1 = duplicate_file(&file_path).expect("should duplicate");
            let copy2 = duplicate_file(&file_path).expect("should duplicate again");

            assert!(copy1.exists());
            assert!(copy2.exists());
            assert_ne!(copy1, copy2);
        }

        #[test]
        fn test_move_file_success() {
            let temp = tempdir().expect("should create temp dir");
            let subdir = temp.path().join("subdir");
            std::fs::create_dir(&subdir).expect("should create dir");

            let file_path = temp.path().join("file.txt");
            std::fs::write(&file_path, "content").expect("should write");

            let new_path = move_file(&file_path, &subdir).expect("should move");
            assert!(new_path.exists());
            assert!(!file_path.exists());
            assert_eq!(new_path.parent().unwrap(), subdir);
        }

        #[test]
        fn test_move_file_not_found() {
            let temp = tempdir().expect("should create temp dir");
            let result = move_file(
                Path::new("/nonexistent/file.txt"),
                temp.path(),
            );
            assert!(result.is_err());
        }

        #[test]
        fn test_delete_permanently_file() {
            let temp = tempdir().expect("should create temp dir");
            let file_path = temp.path().join("to_delete.txt");
            std::fs::write(&file_path, "content").expect("should write");

            assert!(file_path.exists());
            delete_permanently(&file_path).expect("should delete");
            assert!(!file_path.exists());
        }

        #[test]
        fn test_delete_permanently_directory() {
            let temp = tempdir().expect("should create temp dir");
            let dir_path = temp.path().join("to_delete");
            std::fs::create_dir(&dir_path).expect("should create dir");
            std::fs::write(dir_path.join("file.txt"), "content").expect("should write");

            assert!(dir_path.exists());
            delete_permanently(&dir_path).expect("should delete");
            assert!(!dir_path.exists());
        }

        #[test]
        fn test_compress_file() {
            let temp = tempdir().expect("should create temp dir");
            let file_path = temp.path().join("to_compress.txt");
            std::fs::write(&file_path, "content to compress").expect("should write");

            let archive_path = compress(&file_path).expect("should compress");
            assert!(archive_path.exists());
            assert!(archive_path.to_str().unwrap().ends_with(".zip"));
        }

        #[test]
        fn test_compress_directory() {
            let temp = tempdir().expect("should create temp dir");
            let dir_path = temp.path().join("folder");
            std::fs::create_dir(&dir_path).expect("should create dir");
            std::fs::write(dir_path.join("file1.txt"), "content1").expect("should write");
            std::fs::write(dir_path.join("file2.txt"), "content2").expect("should write");

            let archive_path = compress(&dir_path).expect("should compress");
            assert!(archive_path.exists());
            assert!(archive_path.to_str().unwrap().ends_with(".zip"));
        }

        #[test]
        fn test_copy_file_to_clipboard_not_found() {
            let result = copy_file_to_clipboard(Path::new("/nonexistent/file.txt"));
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), FileActionError::NotFound { .. }));
        }
    }
}
