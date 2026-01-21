//! Clipboard history actions.
//!
//
//! GPUI actions for clipboard history operations.
//!
//! These actions are registered using the gpui `actions!` macro which
//! creates unit structs and implements the Action trait for them.

#![allow(clippy::unsafe_derive_deserialize)]
#![allow(clippy::derive_partial_eq_without_eq)]

use gpui::actions;
// Define all clipboard-related actions using the gpui actions! macro.
// This macro creates unit structs and implements the Action trait.
actions!(
    clipboard,
    [
        // Paste the selected clipboard item
        PasteClipboardItem,
        // Copy the selected item to clipboard without pasting
        CopyClipboardItem,
        // Paste as plain text (strip formatting)
        PasteAsPlainText,
        // Paste without saving to history (one-time paste)
        PasteAndDontSave,
        // Toggle pin status of selected item
        TogglePinClipboardItem,
        // Delete selected item
        DeleteClipboardItem,
        // Clear all clipboard history (requires confirmation)
        ClearClipboardHistory,
        // Navigate to next item
        SelectNextClipboardItem,
        // Navigate to previous item
        SelectPreviousClipboardItem,
        // Open clipboard history panel
        OpenClipboardHistory,
        // Close clipboard history panel
        CloseClipboardHistory,
        // Show more actions panel
        ShowClipboardActions,
        // Refresh/reload clipboard history
        RefreshClipboardHistory,
    ]
);

// For actions that need parameters, we define them as plain structs.
// These can be used with event dispatch or direct method calls.

/// Pin a clipboard item by ID.
#[derive(Debug, Clone)]
pub struct PinClipboardItemRequest {
    /// The ID of the item to pin.
    pub id: String,
}

impl PinClipboardItemRequest {
    /// Creates a new pin request.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Unpin a clipboard item by ID.
#[derive(Debug, Clone)]
pub struct UnpinClipboardItemRequest {
    /// The ID of the item to unpin.
    pub id: String,
}

impl UnpinClipboardItemRequest {
    /// Creates a new unpin request.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Delete a clipboard item by ID.
#[derive(Debug, Clone)]
pub struct DeleteClipboardItemRequest {
    /// The ID of the item to delete.
    pub id: String,
}

impl DeleteClipboardItemRequest {
    /// Creates a new delete request.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Search clipboard history.
#[derive(Debug, Clone)]
pub struct SearchClipboardHistoryRequest {
    /// The search query.
    pub query: String,
}

impl SearchClipboardHistoryRequest {
    /// Creates a new search request.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
        }
    }

    /// Returns true if the query is empty.
    pub fn is_empty(&self) -> bool {
        self.query.trim().is_empty()
    }
}

/// Paste a specific clipboard item by ID.
#[derive(Debug, Clone)]
pub struct PasteClipboardItemRequest {
    /// The ID of the item to paste.
    pub id: String,
    /// Whether to paste as plain text.
    pub plain_text: bool,
    /// Whether to skip saving this paste to history.
    pub skip_save: bool,
}

impl PasteClipboardItemRequest {
    /// Creates a new paste request.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            plain_text: false,
            skip_save: false,
        }
    }

    /// Sets whether to paste as plain text.
    #[must_use]
    pub const fn plain_text(mut self, plain: bool) -> Self {
        self.plain_text = plain;
        self
    }

    /// Sets whether to skip saving to history.
    #[must_use]
    pub const fn skip_save(mut self, skip: bool) -> Self {
        self.skip_save = skip;
        self
    }
}

/// Copy a specific clipboard item by ID to the system clipboard.
#[derive(Debug, Clone)]
pub struct CopyClipboardItemRequest {
    /// The ID of the item to copy.
    pub id: String,
    /// Whether to copy as plain text only.
    pub plain_text: bool,
}

impl CopyClipboardItemRequest {
    /// Creates a new copy request.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            plain_text: false,
        }
    }

    /// Sets whether to copy as plain text only.
    #[must_use]
    pub const fn plain_text(mut self, plain: bool) -> Self {
        self.plain_text = plain;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_request() {
        let req = PinClipboardItemRequest::new("test-id");
        assert_eq!(req.id, "test-id");
    }

    #[test]
    fn test_search_request() {
        let req = SearchClipboardHistoryRequest::new("hello");
        assert!(!req.is_empty());

        let empty_req = SearchClipboardHistoryRequest::new("   ");
        assert!(empty_req.is_empty());
    }

    #[test]
    fn test_paste_request_builder() {
        let req = PasteClipboardItemRequest::new("id")
            .plain_text(true)
            .skip_save(true);

        assert_eq!(req.id, "id");
        assert!(req.plain_text);
        assert!(req.skip_save);
    }

    #[test]
    fn test_copy_request_builder() {
        let req = CopyClipboardItemRequest::new("id").plain_text(true);

        assert_eq!(req.id, "id");
        assert!(req.plain_text);
    }
}
