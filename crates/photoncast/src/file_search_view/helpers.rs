//! Utility functions and helper types for the file search view.
//!
//! Contains:
//! - Theme color helpers
//! - Date formatting
//! - File size formatting
//! - File icon/emoji helpers
//! - File kind descriptions

use std::time::SystemTime;

use gpui::*;

use photoncast_calendar::chrono::{DateTime, Datelike, Local, Utc};
use photoncast_core::platform::spotlight::FileKind;
use photoncast_core::theme::PhotonTheme;

use crate::constants::ThemeColorSet;

// ============================================================================
// Helper: Theme Colors
// ============================================================================

/// Type alias – file search uses the shared [`ThemeColorSet`] from constants.
pub type FileSearchColors = ThemeColorSet;

pub fn get_file_search_colors<V>(cx: &ViewContext<V>) -> FileSearchColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    FileSearchColors::from_theme(&theme)
}

// ============================================================================
// Helper: Date Formatting
// ============================================================================

/// Formats a `SystemTime` as a relative date string (Raycast style)
///
/// | Age | Format |
/// |-----|--------|
/// | < 1 minute | `Just now` |
/// | < 1 hour | `Xm` |
/// | < 24 hours | `Xh` |
/// | Yesterday | `Yesterday` |
/// | < 7 days | `Xd` |
/// | < 1 year | `Mon D` |
/// | > 1 year | `Mon D, YYYY` |
pub fn format_relative_date(time: SystemTime) -> String {
    let datetime: DateTime<Utc> = time.into();
    let local: DateTime<Local> = datetime.into();
    let now = Local::now();
    let duration = now.signed_duration_since(local);

    if duration.num_seconds() < 60 {
        "Just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() == 1 {
        "Yesterday".to_string()
    } else if duration.num_days() < 7 {
        format!("{}d", duration.num_days())
    } else if local.year() == now.year() {
        local.format("%b %d").to_string()
    } else {
        local.format("%b %d, %Y").to_string()
    }
}

/// Formats a file size in human-readable format
///
/// | Size | Format |
/// |------|--------|
/// | < 1 KB | `X bytes` |
/// | < 1 MB | `X.X KB` |
/// | < 1 GB | `X.X MB` |
/// | >= 1 GB | `X.XX GB` |
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes < KB {
        format!("{} bytes", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
}

// ============================================================================
// Helper: File Icons
// ============================================================================

/// Returns an emoji icon for a file kind (fallback when system icons unavailable)
pub fn file_kind_to_emoji(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Folder => "📁",
        FileKind::Application => "📦",
        FileKind::Document => "📄",
        FileKind::Image => "🖼️",
        FileKind::Audio => "🎵",
        FileKind::Video => "🎬",
        FileKind::File => "📄",
        FileKind::Other => "📄",
    }
}

/// Returns an emoji icon based on file extension
pub fn extension_to_emoji(path: &std::path::Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        // Documents
        Some("pdf") => "📕",
        Some("doc" | "docx" | "txt" | "rtf" | "odt") => "📄",
        Some("xls" | "xlsx" | "numbers" | "csv") => "📊",
        Some("ppt" | "pptx" | "key") => "📽️",
        // Code
        Some("rs" | "js" | "ts" | "py" | "go" | "swift" | "java" | "c" | "cpp" | "h") => "💻",
        Some("json" | "yaml" | "toml" | "xml") => "⚙️",
        // Archives
        Some("zip" | "rar" | "7z" | "tar" | "gz" | "dmg" | "iso") => "🗜️",
        // Executables
        Some("app" | "exe" | "sh") => "⚡",
        // Default based on kind
        _ => "📄",
    }
}

/// Returns a human-readable description of the file kind
pub fn kind_description(kind: FileKind, path: &std::path::Path) -> String {
    // Try to get more specific description from extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match (kind, ext.as_deref()) {
        (FileKind::Folder, _) => "Folder".to_string(),
        (FileKind::Application, _) => "Application".to_string(),
        (_, Some("pdf")) => "PDF Document".to_string(),
        (_, Some("doc" | "docx")) => "Word Document".to_string(),
        (_, Some("xls" | "xlsx")) => "Excel Spreadsheet".to_string(),
        (_, Some("ppt" | "pptx")) => "PowerPoint Presentation".to_string(),
        (_, Some("txt")) => "Plain Text".to_string(),
        (_, Some("md")) => "Markdown Document".to_string(),
        (_, Some("rtf")) => "Rich Text Document".to_string(),
        (_, Some("jpg" | "jpeg")) => "JPEG Image".to_string(),
        (_, Some("png")) => "PNG Image".to_string(),
        (_, Some("gif")) => "GIF Image".to_string(),
        (_, Some("svg")) => "SVG Image".to_string(),
        (_, Some("heic")) => "HEIC Image".to_string(),
        (_, Some("webp")) => "WebP Image".to_string(),
        (_, Some("mp3")) => "MP3 Audio".to_string(),
        (_, Some("wav")) => "WAV Audio".to_string(),
        (_, Some("flac")) => "FLAC Audio".to_string(),
        (_, Some("m4a")) => "AAC Audio".to_string(),
        (_, Some("mp4")) => "MP4 Video".to_string(),
        (_, Some("mov")) => "QuickTime Movie".to_string(),
        (_, Some("avi")) => "AVI Video".to_string(),
        (_, Some("mkv")) => "MKV Video".to_string(),
        (_, Some("zip")) => "ZIP Archive".to_string(),
        (_, Some("dmg")) => "Disk Image".to_string(),
        (_, Some("rs")) => "Rust Source".to_string(),
        (_, Some("js")) => "JavaScript".to_string(),
        (_, Some("ts")) => "TypeScript".to_string(),
        (_, Some("py")) => "Python Script".to_string(),
        (_, Some("swift")) => "Swift Source".to_string(),
        (_, Some("json")) => "JSON File".to_string(),
        (_, Some("yaml" | "yml")) => "YAML File".to_string(),
        (_, Some("toml")) => "TOML File".to_string(),
        (_, Some("html")) => "HTML Document".to_string(),
        (_, Some("css")) => "CSS Stylesheet".to_string(),
        (FileKind::Document, _) => "Document".to_string(),
        (FileKind::Image, _) => "Image".to_string(),
        (FileKind::Audio, _) => "Audio File".to_string(),
        (FileKind::Video, _) => "Video File".to_string(),
        (FileKind::File, Some(ext)) => format!("{} File", ext.to_uppercase()),
        _ => "File".to_string(),
    }
}
