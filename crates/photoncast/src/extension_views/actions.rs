//! Shared action execution logic for extension views.
//!
//! This module provides common action handling functionality used by all
//! extension view types (ListView, DetailView, GridView).

use gpui::WindowContext;
use photoncast_extension_api::{Action, ActionHandler};

use super::ActionCallback;

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
            let _ = open::that(&url);
            should_close = true;
        },
        ActionHandler::OpenFile(path) => {
            let path = path.to_string();
            let _ = open::that(&path);
            should_close = true;
        },
        ActionHandler::RevealInFinder(path) => {
            let path = path.to_string();
            let _ = std::process::Command::new("open")
                .args(["-R", &path])
                .spawn();
            should_close = true;
        },
        ActionHandler::QuickLook(path) => {
            let path = path.to_string();
            let _ = std::process::Command::new("qlmanage")
                .args(["-p", &path])
                .spawn();
            // Don't close for QuickLook - user may want to continue browsing
        },
        ActionHandler::CopyToClipboard(text) => {
            let text = text.to_string();
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            should_close = true;
        },
        ActionHandler::PushView(_view) => {
            // TODO: Implement view navigation
        },
        ActionHandler::SubmitForm => {
            // Handled by form view specifically
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
