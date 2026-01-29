//! Shared action execution logic for extension views.
//!
//! This module provides common action handling functionality used by all
//! extension view types (ListView, DetailView, GridView).

use gpui::WindowContext;
use photoncast_extension_api::{Action, ActionHandler};

use super::ActionCallback;

/// Copies an image file to the macOS clipboard using NSPasteboard.
///
/// This loads the image from the given path and writes it to the system clipboard
/// so it can be pasted into other applications as an image (not a file reference).
#[cfg(target_os = "macos")]
fn copy_image_to_clipboard(path: &str) -> Result<(), String> {
    use std::process::Command;
    
    // Use osascript to copy image to clipboard via AppleScript
    // This is more reliable than raw NSPasteboard bindings and handles various image formats
    let script = format!(
        r#"set the clipboard to (read (POSIX file "{}") as «class PNGf»)"#,
        path.replace('\\', "\\\\").replace('"', "\\\"")
    );
    
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("Failed to run osascript: {e}"))?;
    
    if output.status.success() {
        Ok(())
    } else {
        // Try with TIFF format as fallback (works with more image types)
        let script_tiff = format!(
            r#"set the clipboard to (read (POSIX file "{}") as TIFF picture)"#,
            path.replace('\\', "\\\\").replace('"', "\\\"")
        );
        
        let output_tiff = Command::new("osascript")
            .args(["-e", &script_tiff])
            .output()
            .map_err(|e| format!("Failed to run osascript: {e}"))?;
        
        if output_tiff.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to copy image: {stderr}"))
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn copy_image_to_clipboard(_path: &str) -> Result<(), String> {
    Err("Image clipboard copy is only supported on macOS".to_string())
}

/// Validates a URL supplied by an extension before opening.
/// Only allows http/https schemes to prevent arbitrary protocol handlers.
fn validate_url(url: &str) -> bool {
    // Check scheme is http or https
    url.starts_with("http://") || url.starts_with("https://")
}

/// Validates a filesystem path supplied by an extension.
/// Rejects paths with traversal components and verifies the path exists.
fn validate_path(path: &str) -> bool {
    let p = std::path::Path::new(path);
    // Reject paths with traversal segments
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            tracing::warn!(path = %path, "Extension action rejected: path contains traversal segment");
            return false;
        }
    }
    // Path must exist on the filesystem
    if !p.exists() {
        tracing::warn!(path = %path, "Extension action rejected: path does not exist");
        return false;
    }
    true
}

/// Action ID used to signal that the extension view should close.
pub const CLOSE_VIEW_ACTION: &str = "__cancel__";

/// Result of executing an action.
pub struct ActionResult {
    /// Whether the action should close the extension view.
    pub should_close: bool,
}

/// Executes an action and returns whether it should close the view.
///
/// This is the shared implementation used by all extension view types.
/// Terminal actions (OpenUrl, OpenFile, etc.) return `should_close = true`.
/// QuickLook intentionally keeps the view open for continued browsing.
pub fn execute_action(
    action: &Action,
    action_callback: &Option<ActionCallback>,
    cx: &mut WindowContext,
) -> ActionResult {
    let mut should_close = false;

    match &action.handler {
        ActionHandler::Callback => {
            if let Some(callback) = action_callback {
                callback(action.id.as_str(), cx);
            }
        },
        ActionHandler::OpenUrl(url) => {
            let url = url.to_string();
            if validate_url(&url) {
                let _ = open::that(&url);
                should_close = true;
            } else {
                tracing::warn!(url = %url, "Extension action rejected: invalid URL scheme");
            }
        },
        ActionHandler::OpenFile(path) => {
            let path = path.to_string();
            if validate_path(&path) {
                let _ = open::that(&path);
                should_close = true;
            }
        },
        ActionHandler::RevealInFinder(path) => {
            let path = path.to_string();
            if validate_path(&path) {
                let _ = std::process::Command::new("open")
                    .args(["-R", &path])
                    .spawn();
                should_close = true;
            }
        },
        ActionHandler::QuickLook(path) => {
            let path = path.to_string();
            if validate_path(&path) {
                let _ = std::process::Command::new("qlmanage")
                    .args(["-p", &path])
                    .spawn();
            }
            // Don't close for QuickLook - user may want to continue browsing
        },
        ActionHandler::CopyToClipboard(text) => {
            let text = text.to_string();
            let preview = if text.len() > 100 {
                &text[..100]
            } else {
                &text
            };
            tracing::info!(
                content_length = text.len(),
                preview = %preview,
                "Extension action: copying content to clipboard"
            );
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            should_close = true;
        },
        ActionHandler::PushView(_view) => {
            // TODO: Implement view navigation
        },
        ActionHandler::SubmitForm => {
            // Handled by form view specifically
        },
        ActionHandler::MoveToTrash(path) => {
            let path = path.to_string();
            if validate_path(&path) {
                match trash::delete(&path) {
                    Ok(()) => {
                        tracing::info!(path = %path, "Moved file to trash");
                        should_close = true;
                    },
                    Err(err) => {
                        tracing::error!(path = %path, error = %err, "Failed to move file to trash");
                    },
                }
            }
        },
        ActionHandler::CopyImageToClipboard(path) => {
            let path = path.to_string();
            if validate_path(&path) {
                match copy_image_to_clipboard(&path) {
                    Ok(()) => {
                        tracing::info!(path = %path, "Copied image to clipboard");
                        should_close = true;
                    },
                    Err(err) => {
                        tracing::error!(path = %path, error = %err, "Failed to copy image to clipboard");
                    },
                }
            }
        },
    }

    ActionResult { should_close }
}

/// Closes the extension view by invoking the callback with the close action.
pub fn close_view(action_callback: &Option<ActionCallback>, cx: &mut WindowContext) {
    if let Some(callback) = action_callback {
        callback(CLOSE_VIEW_ACTION, cx);
    }
}

/// Executes an action and closes the view if it's a terminal action.
///
/// This is a convenience function that combines `execute_action` and `close_view`.
pub fn execute_and_maybe_close(
    action: &Action,
    action_callback: &Option<ActionCallback>,
    cx: &mut WindowContext,
) {
    let result = execute_action(action, action_callback, cx);
    if result.should_close {
        close_view(action_callback, cx);
    }
}
