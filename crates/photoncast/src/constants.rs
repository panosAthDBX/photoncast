//! Shared constants for the photoncast application.
//!
//! This module centralizes window dimensions and other UI constants
//! to avoid duplication across modules.

use gpui::{px, Pixels};

// =============================================================================
// Window Dimensions
// =============================================================================

/// Standard width for the main launcher window.
pub const LAUNCHER_WIDTH: Pixels = px(850.0);

/// Standard height for the main launcher window (collapsed mode).
pub const LAUNCHER_HEIGHT: Pixels = px(520.0);

/// Standard height for expanded windows (file search, clipboard, manage quicklinks).
pub const EXPANDED_HEIGHT: Pixels = px(700.0);

/// Standard width for modal dialogs (quicklinks, create quicklink).
pub const MODAL_WIDTH: Pixels = px(520.0);

/// Standard height for modal dialogs.
#[allow(dead_code)]
pub const MODAL_HEIGHT: Pixels = px(600.0);

/// Minimum window width for resizable windows.
pub const MIN_WINDOW_WIDTH: Pixels = px(600.0);

/// Minimum window height for resizable windows.
pub const MIN_WINDOW_HEIGHT: Pixels = px(500.0);

// =============================================================================
// UI Element Heights
// =============================================================================

/// Height of the search bar across views.
pub const SEARCH_BAR_HEIGHT: Pixels = px(48.0);

/// Height of list items (results, files, etc.).
pub const LIST_ITEM_HEIGHT: Pixels = px(56.0);

/// Height of section headers.
pub const SECTION_HEADER_HEIGHT: Pixels = px(28.0);

/// Height of footers.
pub const FOOTER_HEIGHT: Pixels = px(36.0);

// =============================================================================
// Panel Dimensions
// =============================================================================

/// Width of the list panel in split-view layouts.
pub const LIST_PANEL_WIDTH: Pixels = px(500.0);

/// Width of the detail panel in split-view layouts.
pub const DETAIL_PANEL_WIDTH: Pixels = px(350.0);
