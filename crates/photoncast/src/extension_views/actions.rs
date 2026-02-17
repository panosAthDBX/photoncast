//! Shared action execution logic for extension views.
//!
//! This module provides common action handling functionality used by all
//! extension view types (ListView, DetailView, GridView).

use gpui::WindowContext;
use photoncast_core::extensions::ExtensionViewHostAction;
use photoncast_extension_api::{Action, ActionHandler};

use super::{ActionCallback, ExtensionViewCallbackPayload};

/// Action ID used to signal that the extension view should close.
pub const CLOSE_VIEW_ACTION: &str = "__cancel__";

/// Returns true if the action is terminal and should close the extension view.
#[must_use]
pub fn is_terminal_action(handler: &ActionHandler) -> bool {
    matches!(
        handler,
        ActionHandler::OpenUrl(_)
            | ActionHandler::OpenFile(_)
            | ActionHandler::RevealInFinder(_)
            | ActionHandler::CopyToClipboard(_)
            | ActionHandler::MoveToTrash(_)
            | ActionHandler::CopyImageToClipboard(_)
    )
}

fn build_delegated_host_action(handler: &ActionHandler) -> Option<ExtensionViewHostAction> {
    match handler {
        ActionHandler::OpenUrl(url) => Some(ExtensionViewHostAction::OpenUrl {
            url: url.as_str().to_string(),
        }),
        ActionHandler::OpenFile(path) => Some(ExtensionViewHostAction::OpenFile {
            path: path.as_str().to_string(),
        }),
        ActionHandler::RevealInFinder(path) => Some(ExtensionViewHostAction::RevealInFinder {
            path: path.as_str().to_string(),
        }),
        ActionHandler::QuickLook(path) => Some(ExtensionViewHostAction::QuickLook {
            path: path.as_str().to_string(),
        }),
        ActionHandler::CopyToClipboard(text) => Some(ExtensionViewHostAction::CopyToClipboard {
            text: text.as_str().to_string(),
        }),
        ActionHandler::MoveToTrash(path) => Some(ExtensionViewHostAction::MoveToTrash {
            path: path.as_str().to_string(),
        }),
        ActionHandler::CopyImageToClipboard(path) => {
            Some(ExtensionViewHostAction::CopyImageToClipboard {
                path: path.as_str().to_string(),
            })
        },
        ActionHandler::PushView(_) | ActionHandler::SubmitForm | ActionHandler::Callback => None,
    }
}

/// Builds the callback payload that should be emitted for the provided action.
#[must_use]
pub fn payload_for_action(
    extension_id: &str,
    action: &Action,
) -> Option<ExtensionViewCallbackPayload> {
    match &action.handler {
        ActionHandler::Callback => Some(ExtensionViewCallbackPayload::CallbackAction {
            extension_id: extension_id.to_string(),
            action_id: action.id.as_str().to_string(),
        }),
        ActionHandler::PushView(_) | ActionHandler::SubmitForm => None,
        _ => build_delegated_host_action(&action.handler).map(|delegated| {
            ExtensionViewCallbackPayload::DelegatedAction {
                extension_id: extension_id.to_string(),
                action_id: action.id.as_str().to_string(),
                action: delegated,
                should_close: is_terminal_action(&action.handler),
            }
        }),
    }
}

/// Executes an action by delegating payload handling to the launcher callback.
pub fn execute_action(
    extension_id: &str,
    action: &Action,
    action_callback: &Option<ActionCallback>,
    cx: &mut WindowContext,
) {
    let Some(callback) = action_callback else {
        return;
    };

    if let Some(payload) = payload_for_action(extension_id, action) {
        callback(payload, cx);
    }
}

/// Closes the extension view by invoking the callback with the close action.
pub fn close_view(
    extension_id: &str,
    action_callback: &Option<ActionCallback>,
    cx: &mut WindowContext,
) {
    if let Some(callback) = action_callback {
        callback(
            ExtensionViewCallbackPayload::CloseView {
                extension_id: extension_id.to_string(),
            },
            cx,
        );
    }
}

/// Builds a structured form submit callback payload.
#[must_use]
pub fn build_submit_payload(
    extension_id: &str,
    values_json: String,
) -> ExtensionViewCallbackPayload {
    ExtensionViewCallbackPayload::SubmitForm {
        extension_id: extension_id.to_string(),
        values_json,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_action(id: &str, handler: ActionHandler) -> Action {
        Action {
            id: id.into(),
            title: "Test".into(),
            icon: abi_stable::std_types::ROption::RNone,
            shortcut: abi_stable::std_types::ROption::RNone,
            style: photoncast_extension_api::ActionStyle::Default,
            handler,
        }
    }

    #[test]
    fn payload_for_callback_action_contains_ids() {
        let action = make_action("callback-id", ActionHandler::Callback);
        let payload = payload_for_action("ext.example", &action).expect("payload expected");

        assert_eq!(
            payload,
            ExtensionViewCallbackPayload::CallbackAction {
                extension_id: "ext.example".to_string(),
                action_id: "callback-id".to_string(),
            }
        );
    }

    #[test]
    fn payload_for_open_url_delegates_and_closes() {
        let action = make_action(
            "open-url",
            ActionHandler::OpenUrl("https://example.com".into()),
        );
        let payload = payload_for_action("ext.example", &action).expect("payload expected");

        assert_eq!(
            payload,
            ExtensionViewCallbackPayload::DelegatedAction {
                extension_id: "ext.example".to_string(),
                action_id: "open-url".to_string(),
                action: ExtensionViewHostAction::OpenUrl {
                    url: "https://example.com".to_string(),
                },
                should_close: true,
            }
        );
    }

    #[test]
    fn payload_for_quick_look_does_not_close() {
        let action = make_action(
            "quick-look",
            ActionHandler::QuickLook("/tmp/example.png".into()),
        );
        let payload = payload_for_action("ext.example", &action).expect("payload expected");

        assert_eq!(
            payload,
            ExtensionViewCallbackPayload::DelegatedAction {
                extension_id: "ext.example".to_string(),
                action_id: "quick-look".to_string(),
                action: ExtensionViewHostAction::QuickLook {
                    path: "/tmp/example.png".to_string(),
                },
                should_close: false,
            }
        );
    }

    #[test]
    fn push_view_and_submit_form_do_not_emit_payload() {
        let push_action = make_action(
            "push-view",
            ActionHandler::PushView(abi_stable::std_types::RBox::new(
                photoncast_extension_api::ExtensionView::List(photoncast_extension_api::ListView {
                    title: "Title".into(),
                    search_bar: abi_stable::std_types::ROption::RNone,
                    sections: abi_stable::std_types::RVec::new(),
                    empty_state: abi_stable::std_types::ROption::RNone,
                    show_preview: false,
                }),
            )),
        );
        let submit_action = make_action("submit", ActionHandler::SubmitForm);

        assert!(payload_for_action("ext.example", &push_action).is_none());
        assert!(payload_for_action("ext.example", &submit_action).is_none());
    }

    #[test]
    fn terminal_action_detection_matches_contract() {
        assert!(is_terminal_action(&ActionHandler::OpenFile(
            "/tmp/a".into()
        )));
        assert!(is_terminal_action(&ActionHandler::CopyToClipboard(
            "abc".into()
        )));
        assert!(is_terminal_action(&ActionHandler::MoveToTrash(
            "/tmp/a".into()
        )));
        assert!(is_terminal_action(&ActionHandler::CopyImageToClipboard(
            "/tmp/a".into()
        )));

        assert!(!is_terminal_action(&ActionHandler::QuickLook(
            "/tmp/a".into()
        )));
        assert!(!is_terminal_action(&ActionHandler::Callback));
        assert!(!is_terminal_action(&ActionHandler::SubmitForm));
    }

    #[test]
    fn build_submit_payload_is_structured() {
        let payload = build_submit_payload("ext.example", "{\"a\":1}".to_string());
        assert_eq!(
            payload,
            ExtensionViewCallbackPayload::SubmitForm {
                extension_id: "ext.example".to_string(),
                values_json: "{\"a\":1}".to_string(),
            }
        );
    }
}
