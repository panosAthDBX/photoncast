//! Shared constants for the photoncast application.
//!
//! This module centralizes window dimensions and other UI constants
//! to avoid duplication across modules.

use gpui::{px, Pixels};
use photoncast_core::theme::GpuiThemeColors;

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
// Icon Sizes
// =============================================================================

/// Small icon size (16×16) — inline text, accessories.
pub const ICON_SIZE_SM: Pixels = px(16.0);

/// Medium icon size (24×24) — list items, buttons.
pub const ICON_SIZE_MD: Pixels = px(24.0);

/// Large icon size (32×32) — grid items, headers.
pub const ICON_SIZE_LG: Pixels = px(32.0);

// =============================================================================
// Text Sizes
// =============================================================================

/// Small text size (12pt) — captions, labels.
pub const TEXT_SIZE_SM: Pixels = px(12.0);

/// Medium text size (16pt) — body text, search input.
pub const TEXT_SIZE_MD: Pixels = px(16.0);

/// Large text size (24pt) — emoji icons, headings.
pub const TEXT_SIZE_LG: Pixels = px(24.0);

// =============================================================================
// Spacing
// =============================================================================

/// Standard section gap (16px) — spacing between form/section groups.
pub const SECTION_GAP: Pixels = px(16.0);

// =============================================================================
// Opacity
// =============================================================================

/// Overlay alpha value — used for modal overlays and dimming.
#[allow(dead_code)]
pub const OVERLAY_ALPHA: f32 = 0.6;

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
/// This is a type alias for [`GpuiThemeColors`] from `photoncast-theme`.
pub type ThemeColorSet = GpuiThemeColors;
