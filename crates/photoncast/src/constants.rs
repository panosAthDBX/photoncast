//! Shared constants for the photoncast application.
//!
//! This module centralizes window dimensions and other UI constants
//! to avoid duplication across modules.

use gpui::{px, Hsla, Pixels};
use photoncast_core::theme::PhotonTheme;

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
#[allow(dead_code)]
pub const FOOTER_HEIGHT: Pixels = px(36.0);

// =============================================================================
// Panel Dimensions
// =============================================================================

/// Width of the list panel in split-view layouts.
pub const LIST_PANEL_WIDTH: Pixels = px(500.0);

/// Width of the detail panel in split-view layouts.
pub const DETAIL_PANEL_WIDTH: Pixels = px(350.0);

// =============================================================================
// File Type Extensions
// =============================================================================

/// Document file extensions.
pub const DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "odt", "rtf", "txt", "pages", "md", "xls", "xlsx", "csv", "numbers",
    "ppt", "pptx", "key",
];

/// Image file extensions.
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "svg", "ico", "heic", "heif",
    "raw", "cr2", "nef", "arw", "dng", "psd",
];

/// Video file extensions.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
];

/// Audio file extensions.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "aiff",
];

/// Archive file extensions.
pub const ARCHIVE_EXTENSIONS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "dmg", "iso",
];

/// E-book file extensions.
pub const EBOOK_EXTENSIONS: &[&str] = &["epub", "mobi"];

/// macOS app extensions.
pub const APP_EXTENSIONS: &[&str] = &["app"];

// =============================================================================
// Theme Colors
// =============================================================================

/// Shared theme color set used across views (launcher, file search, extensions).
///
/// Constructed from a `PhotonTheme` via [`ThemeColorSet::from_theme`], this
/// struct caches the most commonly used GPUI color values so that individual
/// render methods do not need to query the theme repeatedly.
#[derive(Clone)]
pub struct ThemeColorSet {
    pub background: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_placeholder: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub surface_elevated: Hsla,
    pub border: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub selection: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub overlay: Hsla,
}

impl ThemeColorSet {
    pub fn from_theme(theme: &PhotonTheme) -> Self {
        Self {
            background: theme.colors.background.to_gpui(),
            text: theme.colors.text.to_gpui(),
            text_muted: theme.colors.text_muted.to_gpui(),
            text_placeholder: theme.colors.text_placeholder.to_gpui(),
            surface: theme.colors.surface.to_gpui(),
            surface_hover: theme.colors.surface_hover.to_gpui(),
            surface_elevated: theme.colors.background_elevated.to_gpui(),
            border: theme.colors.border.to_gpui(),
            accent: theme.colors.accent.to_gpui(),
            accent_hover: theme.colors.accent_hover.to_gpui(),
            selection: theme.colors.selection.to_gpui(),
            success: theme.colors.success.to_gpui(),
            warning: theme.colors.warning.to_gpui(),
            error: theme.colors.error.to_gpui(),
            overlay: gpui::hsla(0.0, 0.0, 0.0, 0.6),
        }
    }
}
