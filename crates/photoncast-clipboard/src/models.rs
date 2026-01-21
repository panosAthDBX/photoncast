//! Clipboard data models.
//!
//! This module defines the core data types for clipboard items.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::PREVIEW_TEXT_LENGTH;

/// Unique identifier for a clipboard item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClipboardItemId(String);

impl ClipboardItemId {
    /// Creates a new `ClipboardItemId`.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generates a new unique ID.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Returns the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ClipboardItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ClipboardItemId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ClipboardItemId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Content types for clipboard items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClipboardContentType {
    /// Plain text content.
    Text {
        /// The full text content.
        content: String,
        /// Preview of the content (first N characters).
        preview: String,
    },

    /// Rich text with HTML/RTF formatting.
    RichText {
        /// Plain text version.
        plain: String,
        /// HTML content (if available).
        html: Option<String>,
        /// RTF content (if available).
        rtf: Option<String>,
    },

    /// Image content.
    Image {
        /// Path to the stored image file.
        path: PathBuf,
        /// Path to the thumbnail file.
        thumbnail_path: PathBuf,
        /// Size in bytes.
        size_bytes: u64,
        /// Dimensions (width, height).
        dimensions: (u32, u32),
    },

    /// File references.
    File {
        /// Paths to the files.
        paths: Vec<PathBuf>,
        /// Paths to cached icons.
        icons: Vec<PathBuf>,
        /// Total size in bytes.
        total_size: u64,
    },

    /// URL with metadata.
    Link {
        /// The URL.
        url: String,
        /// Page title (if fetched).
        title: Option<String>,
        /// Path to cached favicon.
        favicon_path: Option<PathBuf>,
    },

    /// Color value.
    Color {
        /// Hex color string (e.g., "#FF5733").
        hex: String,
        /// RGB values.
        rgb: (u8, u8, u8),
        /// Display name (e.g., "Orange/Red").
        display_name: Option<String>,
    },
}

impl ClipboardContentType {
    /// Safely truncates a string to at most `max_chars` characters.
    /// Returns the truncated string without breaking multi-byte UTF-8 characters.
    fn truncate_str(s: &str, max_chars: usize) -> String {
        let char_count = s.chars().count();
        if char_count <= max_chars {
            s.to_string()
        } else {
            s.chars().take(max_chars).collect()
        }
    }

    /// Creates a text content type.
    pub fn text(content: impl Into<String>) -> Self {
        let content = content.into();
        let preview = if content.chars().count() > PREVIEW_TEXT_LENGTH {
            format!("{}...", Self::truncate_str(&content, PREVIEW_TEXT_LENGTH))
        } else {
            content.clone()
        };
        Self::Text { content, preview }
    }

    /// Creates a rich text content type.
    pub fn rich_text(plain: impl Into<String>, html: Option<String>, rtf: Option<String>) -> Self {
        Self::RichText {
            plain: plain.into(),
            html,
            rtf,
        }
    }

    /// Creates an image content type.
    pub const fn image(
        path: PathBuf,
        thumbnail_path: PathBuf,
        size_bytes: u64,
        dimensions: (u32, u32),
    ) -> Self {
        Self::Image {
            path,
            thumbnail_path,
            size_bytes,
            dimensions,
        }
    }

    /// Creates a file content type.
    pub const fn file(paths: Vec<PathBuf>, icons: Vec<PathBuf>, total_size: u64) -> Self {
        Self::File {
            paths,
            icons,
            total_size,
        }
    }

    /// Creates a link content type.
    pub fn link(url: impl Into<String>) -> Self {
        Self::Link {
            url: url.into(),
            title: None,
            favicon_path: None,
        }
    }

    /// Creates a color content type.
    pub fn color(hex: impl Into<String>, rgb: (u8, u8, u8)) -> Self {
        Self::Color {
            hex: hex.into(),
            rgb,
            display_name: None,
        }
    }

    /// Returns the type name as a string.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::RichText { .. } => "rich_text",
            Self::Image { .. } => "image",
            Self::File { .. } => "file",
            Self::Link { .. } => "link",
            Self::Color { .. } => "color",
        }
    }

    /// Returns a text representation for search indexing.
    #[must_use]
    pub fn search_text(&self) -> String {
        match self {
            Self::Text { content, .. } => content.clone(),
            Self::RichText { plain, .. } => plain.clone(),
            Self::Image { .. } => String::new(),
            Self::File { paths, .. } => paths
                .iter()
                .filter_map(|p| p.file_name())
                .filter_map(|n| n.to_str())
                .collect::<Vec<_>>()
                .join(" "),
            Self::Link { url, title, .. } => {
                format!("{} {}", url, title.as_deref().unwrap_or(""))
            },
            Self::Color {
                hex, display_name, ..
            } => {
                format!("{} {}", hex, display_name.as_deref().unwrap_or(""))
            },
        }
    }

    /// Returns a preview string for display.
    #[must_use]
    pub fn preview(&self) -> String {
        match self {
            Self::Text { preview, .. } => preview.clone(),
            Self::RichText { plain, .. } => {
                if plain.chars().count() > PREVIEW_TEXT_LENGTH {
                    format!("{}...", Self::truncate_str(plain, PREVIEW_TEXT_LENGTH))
                } else {
                    plain.clone()
                }
            },
            Self::Image {
                dimensions,
                size_bytes,
                ..
            } => {
                format!(
                    "Image {}x{} ({})",
                    dimensions.0,
                    dimensions.1,
                    format_kilobytes(*size_bytes)
                )
            },
            Self::File {
                paths, total_size, ..
            } => {
                let names: Vec<_> = paths
                    .iter()
                    .filter_map(|p| p.file_name())
                    .filter_map(|n| n.to_str())
                    .take(3)
                    .collect();
                let suffix = if paths.len() > 3 {
                    format!(" +{} more", paths.len() - 3)
                } else {
                    String::new()
                };
                format!(
                    "{}{} ({})",
                    names.join(", "),
                    suffix,
                    format_kilobytes(*total_size)
                )
            },
            Self::Link { url, title, .. } => title.clone().unwrap_or_else(|| url.clone()),
            Self::Color {
                hex, display_name, ..
            } => display_name.clone().unwrap_or_else(|| hex.clone()),
        }
    }

    /// Returns the text content if this is a text type.
    #[must_use]
    pub fn text_content(&self) -> Option<&str> {
        match self {
            Self::Text { content, .. } => Some(content),
            Self::RichText { plain, .. } => Some(plain),
            _ => None,
        }
    }
}

/// A clipboard history item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardItem {
    /// Unique identifier.
    pub id: ClipboardItemId,

    /// The content type and data.
    pub content_type: ClipboardContentType,

    /// Source application name.
    pub source_app: Option<String>,

    /// Source application bundle ID.
    pub source_bundle_id: Option<String>,

    /// Size in bytes.
    pub size_bytes: u64,

    /// Whether this item is pinned.
    pub is_pinned: bool,

    /// When the item was created (copied).
    pub created_at: DateTime<Utc>,

    /// When the item was last accessed.
    pub accessed_at: Option<DateTime<Utc>>,
}

impl ClipboardItem {
    /// Creates a new clipboard item.
    pub fn new(content_type: ClipboardContentType) -> Self {
        let size_bytes = Self::calculate_size(&content_type);
        Self {
            id: ClipboardItemId::generate(),
            content_type,
            source_app: None,
            source_bundle_id: None,
            size_bytes,
            is_pinned: false,
            created_at: Utc::now(),
            accessed_at: None,
        }
    }

    /// Creates a new text clipboard item.
    pub fn text(content: impl Into<String>) -> Self {
        Self::new(ClipboardContentType::text(content))
    }

    /// Sets the source application.
    #[must_use]
    pub fn with_source(mut self, app: impl Into<String>, bundle_id: impl Into<String>) -> Self {
        self.source_app = Some(app.into());
        self.source_bundle_id = Some(bundle_id.into());
        self
    }

    /// Sets the item as pinned.
    #[must_use]
    pub const fn with_pinned(mut self, pinned: bool) -> Self {
        self.is_pinned = pinned;
        self
    }

    /// Returns the text content if this is a text type.
    #[must_use]
    pub fn text_content(&self) -> Option<&str> {
        self.content_type.text_content()
    }

    /// Returns a preview string for display.
    #[must_use]
    pub fn preview(&self) -> String {
        self.content_type.preview()
    }

    /// Returns text for search indexing.
    #[must_use]
    pub fn search_text(&self) -> String {
        self.content_type.search_text()
    }

    /// Calculates the approximate size of the content.
    fn calculate_size(content_type: &ClipboardContentType) -> u64 {
        match content_type {
            ClipboardContentType::Text { content, .. } => content.len() as u64,
            ClipboardContentType::RichText { plain, html, rtf } => {
                plain.len() as u64
                    + html.as_ref().map_or(0, |h| h.len() as u64)
                    + rtf.as_ref().map_or(0, |r| r.len() as u64)
            },
            ClipboardContentType::Image { size_bytes, .. } => *size_bytes,
            ClipboardContentType::File { total_size, .. } => *total_size,
            ClipboardContentType::Link { url, title, .. } => {
                url.len() as u64 + title.as_ref().map_or(0, |t| t.len() as u64)
            },
            ClipboardContentType::Color {
                hex, display_name, ..
            } => hex.len() as u64 + display_name.as_ref().map_or(0, |n| n.len() as u64),
        }
    }
}

fn format_kilobytes(size_bytes: u64) -> String {
    let whole = size_bytes / 1024;
    let remainder = size_bytes % 1024;
    let decimal = remainder * 10 / 1024;
    format!("{whole}.{decimal} KB")
}

impl std::fmt::Display for ClipboardItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.content_type.type_name(), self.preview())
    }
}

/// Detect if a string contains a valid color value.
///
/// Returns the hex string and RGB values if detected.
#[must_use]
pub fn detect_color(text: &str) -> Option<(String, (u8, u8, u8))> {
    let text = text.trim();

    // Try hex color: #RGB, #RRGGBB
    if let Some(hex) = text.strip_prefix('#') {
        match hex.len() {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                let r = r * 17; // 0xF -> 0xFF
                let g = g * 17;
                let b = b * 17;
                return Some((format!("#{:02X}{:02X}{:02X}", r, g, b), (r, g, b)));
            },
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some((text.to_uppercase(), (r, g, b)));
            },
            _ => return None,
        }
    }

    // Try rgb(r, g, b)
    if text.to_lowercase().starts_with("rgb(") && text.ends_with(')') {
        let inner = &text[4..text.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            let r: u8 = parts[0].parse().ok()?;
            let g: u8 = parts[1].parse().ok()?;
            let b: u8 = parts[2].parse().ok()?;
            return Some((format!("#{:02X}{:02X}{:02X}", r, g, b), (r, g, b)));
        }
    }

    None
}

/// Detects if text contains a URL.
#[must_use]
pub fn detect_url(text: &str) -> Option<String> {
    let text = text.trim();

    // Simple URL detection
    if text.starts_with("http://") || text.starts_with("https://") {
        // Validate it's a reasonable URL
        if url::Url::parse(text).is_ok() {
            return Some(text.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_item_id() {
        let id = ClipboardItemId::generate();
        assert!(!id.as_str().is_empty());

        let id2 = ClipboardItemId::new("test-id");
        assert_eq!(id2.as_str(), "test-id");
    }

    #[test]
    fn test_text_content_type() {
        let ct = ClipboardContentType::text("Hello, World!");
        assert_eq!(ct.type_name(), "text");
        assert_eq!(ct.text_content(), Some("Hello, World!"));
    }

    #[test]
    fn test_text_preview_truncation() {
        let long_text = "a".repeat(200);
        let ct = ClipboardContentType::text(&long_text);
        if let ClipboardContentType::Text { preview, .. } = ct {
            assert!(preview.chars().count() <= PREVIEW_TEXT_LENGTH + 3); // +3 for "..."
            assert!(preview.ends_with("..."));
        } else {
            panic!("Expected Text content type");
        }
    }

    #[test]
    fn test_clipboard_item_creation() {
        let item = ClipboardItem::text("Test content");
        assert!(!item.id.as_str().is_empty());
        assert_eq!(item.text_content(), Some("Test content"));
        assert!(!item.is_pinned);
    }

    #[test]
    fn test_clipboard_item_with_source() {
        let item = ClipboardItem::text("Test").with_source("Safari", "com.apple.Safari");
        assert_eq!(item.source_app, Some("Safari".to_string()));
        assert_eq!(item.source_bundle_id, Some("com.apple.Safari".to_string()));
    }

    #[test]
    fn test_detect_color_hex6() {
        let (hex, rgb) = detect_color("#FF5733").expect("should detect color");
        assert_eq!(hex, "#FF5733");
        assert_eq!(rgb, (255, 87, 51));
    }

    #[test]
    fn test_detect_color_hex3() {
        let (hex, rgb) = detect_color("#F00").expect("should detect color");
        assert_eq!(hex, "#FF0000");
        assert_eq!(rgb, (255, 0, 0));
    }

    #[test]
    fn test_detect_color_rgb() {
        let (hex, rgb) = detect_color("rgb(255, 128, 64)").expect("should detect color");
        assert_eq!(hex, "#FF8040");
        assert_eq!(rgb, (255, 128, 64));
    }

    #[test]
    fn test_detect_color_invalid() {
        assert!(detect_color("not a color").is_none());
        assert!(detect_color("#GGG").is_none());
    }

    #[test]
    fn test_detect_url() {
        assert_eq!(
            detect_url("https://example.com"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            detect_url("http://test.org/path"),
            Some("http://test.org/path".to_string())
        );
        assert!(detect_url("not a url").is_none());
        assert!(detect_url("ftp://example.com").is_none());
    }

    #[test]
    fn test_search_text() {
        let text = ClipboardContentType::text("Hello World");
        assert_eq!(text.search_text(), "Hello World");

        let link = ClipboardContentType::Link {
            url: "https://example.com".to_string(),
            title: Some("Example Site".to_string()),
            favicon_path: None,
        };
        assert!(link.search_text().contains("example.com"));
        assert!(link.search_text().contains("Example Site"));
    }

    #[test]
    fn test_serialization() {
        let item = ClipboardItem::text("Test content").with_source("Safari", "com.apple.Safari");

        let json = serde_json::to_string(&item).expect("should serialize");
        let parsed: ClipboardItem = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(parsed.text_content(), item.text_content());
        assert_eq!(parsed.source_app, item.source_app);
    }
}
