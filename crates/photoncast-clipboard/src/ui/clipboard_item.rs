//! Clipboard item view component.
//!
//! Renders a single clipboard item in the list with proper content type
//! previews including color swatches, image thumbnails, and URL favicons.

use gpui::{
    div, px, rems, rgb, Div, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    ViewContext,
};

use crate::models::{ClipboardContentType, ClipboardItem};

/// Catppuccin Mocha color palette.
mod colors {
    pub const BASE: u32 = 0x1E_1E2E;
    pub const SURFACE0: u32 = 0x31_3244;
    pub const SURFACE1: u32 = 0x45_475A;
    pub const SURFACE2: u32 = 0x58_5B70;
    pub const OVERLAY0: u32 = 0x6C_7086;
    pub const TEXT: u32 = 0xCD_D6F4;
}

/// Maximum preview text length.
const MAX_PREVIEW_LENGTH: usize = 100;

/// View for a single clipboard item.
#[derive(Debug)]
pub struct ClipboardItemView {
    /// The clipboard item.
    item: ClipboardItem,
    /// Whether this item is selected.
    is_selected: bool,
    /// Index in the list.
    index: usize,
    /// Whether to show the full content preview.
    show_full_preview: bool,
    /// Search query for highlighting (optional).
    search_query: Option<String>,
}

impl ClipboardItemView {
    /// Creates a new clipboard item view.
    pub const fn new(item: ClipboardItem, index: usize) -> Self {
        Self {
            item,
            is_selected: false,
            index,
            show_full_preview: false,
            search_query: None,
        }
    }

    /// Sets the selected state.
    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
    }

    /// Sets whether to show full preview.
    pub fn set_show_full_preview(&mut self, show: bool) {
        self.show_full_preview = show;
    }

    /// Sets the search query for highlighting.
    pub fn set_search_query(&mut self, query: Option<String>) {
        self.search_query = query;
    }

    /// Returns the item.
    pub const fn item(&self) -> &ClipboardItem {
        &self.item
    }

    /// Returns the content type icon emoji.
    const fn icon(&self) -> &'static str {
        match &self.item.content_type {
            ClipboardContentType::Text { .. } => "📝",
            ClipboardContentType::RichText { .. } => "📄",
            ClipboardContentType::Image { .. } => "🖼️",
            ClipboardContentType::File { .. } => "📁",
            ClipboardContentType::Link { .. } => "🔗",
            ClipboardContentType::Color { .. } => "🎨",
        }
    }

    /// Returns the relative time string.
    fn relative_time(&self) -> String {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(self.item.created_at);

        if duration.num_seconds() < 60 {
            "Just now".to_string()
        } else if duration.num_minutes() < 60 {
            let mins = duration.num_minutes();
            if mins == 1 {
                "1m".to_string()
            } else {
                format!("{}m", mins)
            }
        } else if duration.num_hours() < 24 {
            let hours = duration.num_hours();
            if hours == 1 {
                "1h".to_string()
            } else {
                format!("{}h", hours)
            }
        } else if duration.num_days() < 7 {
            let days = duration.num_days();
            if days == 1 {
                "1d".to_string()
            } else {
                format!("{}d", days)
            }
        } else {
            self.item.created_at.format("%b %d").to_string()
        }
    }

    /// Renders the icon or visual preview based on content type.
    fn render_visual_preview(&self) -> impl IntoElement {
        match &self.item.content_type {
            ClipboardContentType::Color { rgb: color_rgb, .. } => render_color_swatch(*color_rgb),
            ClipboardContentType::Image {
                thumbnail_path,
                dimensions,
                ..
            } => render_image_thumbnail(thumbnail_path, *dimensions),
            ClipboardContentType::Link { favicon_path, .. } => {
                render_favicon(favicon_path.as_ref())
            },
            _ => {
                // Default icon
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(24.0))
                    .child(div().text_sm().child(self.icon()))
            },
        }
    }

    /// Renders the text content with optional search highlighting.
    fn render_text_content(text: &str) -> impl IntoElement {
        let truncated = truncate_text(text, MAX_PREVIEW_LENGTH);
        div()
            .text_sm()
            .text_color(rgb(colors::TEXT))
            .truncate()
            .child(truncated)
    }
}

impl Render for ClipboardItemView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let bg_color = if self.is_selected {
            rgb(colors::SURFACE1)
        } else {
            rgb(colors::BASE)
        };

        let preview = self.item.preview();
        let time = self.relative_time();
        let pinned = self.item.is_pinned;

        div()
            .id(("clipboard-item", self.index))
            .w_full()
            .px(rems(0.75))
            .py(rems(0.5))
            .bg(bg_color)
            .rounded(px(4.0))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(colors::SURFACE0)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(rems(0.5))
                    .items_center()
                    // Visual preview (icon, color swatch, thumbnail, favicon)
                    .child(self.render_visual_preview())
                    // Content
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(Self::render_text_content(&preview)),
                    )
                    // Metadata
                    .child(render_metadata(pinned, time)),
            )
    }
}

/// Renders a color swatch preview.
pub fn render_color_swatch(color_rgb: (u8, u8, u8)) -> Div {
    let color = rgb((u32::from(color_rgb.0) << 16)
        | (u32::from(color_rgb.1) << 8)
        | u32::from(color_rgb.2));

    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(24.0))
        .child(
            div()
                .size(px(18.0))
                .rounded(px(4.0))
                .bg(color)
                .border_1()
                .border_color(rgb(colors::SURFACE1))
                .shadow_sm(),
        )
}

/// Renders an image thumbnail preview.
pub fn render_image_thumbnail(thumbnail_path: &std::path::Path, _dimensions: (u32, u32)) -> Div {
    // Check if thumbnail exists
    if thumbnail_path.exists() {
        // In a real implementation, we would load the actual image
        // For now, show a placeholder with dimensions
        div()
            .flex()
            .items_center()
            .justify_center()
            .size(px(24.0))
            .child(
                div()
                    .size(px(20.0))
                    .rounded(px(4.0))
                    .bg(rgb(colors::SURFACE1))
                    .border_1()
                    .border_color(rgb(colors::SURFACE2))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().text_xs().child("🖼️")),
            )
    } else {
        // Fallback to icon with dimensions indicator
        div()
            .flex()
            .items_center()
            .justify_center()
            .size(px(24.0))
            .child(div().text_sm().child("🖼️"))
    }
}

/// Renders a favicon for URLs.
pub fn render_favicon(favicon_path: Option<&std::path::PathBuf>) -> Div {
    if let Some(path) = favicon_path {
        if path.exists() {
            // In a real implementation, we would load the actual favicon
            return div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(24.0))
                .child(
                    div()
                        .size(px(16.0))
                        .rounded(px(2.0))
                        .bg(rgb(colors::SURFACE1))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().text_xs().child("🌐")),
                );
        }
    }

    // Fallback to link icon
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(24.0))
        .child(div().text_sm().child("🔗"))
}

/// Renders item metadata (pin indicator and time).
fn render_metadata(pinned: bool, time: String) -> impl IntoElement {
    let mut el = div()
        .flex()
        .flex_row()
        .gap(rems(0.375))
        .items_center()
        .flex_shrink_0();

    if pinned {
        el = el.child(div().text_xs().child("📌"));
    }

    el.child(
        div()
            .text_xs()
            .text_color(rgb(colors::OVERLAY0))
            .child(time),
    )
}

/// Truncates text to a maximum length.
fn truncate_text(text: &str, max_len: usize) -> String {
    // Normalize whitespace
    let normalized: String = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if normalized.len() > max_len {
        format!("{}...", &normalized[..max_len])
    } else {
        normalized
    }
}

/// Represents highlighted text segments for search results.
#[derive(Debug, Clone)]
pub struct HighlightedText {
    /// Text segments with highlight flags.
    pub segments: Vec<(String, bool)>,
}

impl HighlightedText {
    /// Creates highlighted text from a string and search query.
    pub fn from_search(text: &str, query: &str) -> Self {
        if query.is_empty() {
            return Self {
                segments: vec![(text.to_string(), false)],
            };
        }

        let lower_text = text.to_lowercase();
        let lower_query = query.to_lowercase();
        let mut segments = Vec::new();
        let mut last_end = 0;

        for (start, _) in lower_text.match_indices(&lower_query) {
            // Add non-highlighted segment before match
            if start > last_end {
                segments.push((text[last_end..start].to_string(), false));
            }

            // Add highlighted match
            let end = start + query.len();
            segments.push((text[start..end].to_string(), true));
            last_end = end;
        }

        // Add remaining text
        if last_end < text.len() {
            segments.push((text[last_end..].to_string(), false));
        }

        if segments.is_empty() {
            segments.push((text.to_string(), false));
        }

        Self { segments }
    }

    /// Returns true if any segment is highlighted.
    pub fn has_highlights(&self) -> bool {
        self.segments.iter().any(|(_, highlighted)| *highlighted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("Hello", 10), "Hello");
        assert_eq!(truncate_text("Hello World Test", 10), "Hello Worl...");
        assert_eq!(
            truncate_text("  Multiple   spaces  ", 20),
            "Multiple spaces"
        );
    }

    #[test]
    fn test_highlighted_text() {
        let ht = HighlightedText::from_search("Hello World", "world");
        assert!(ht.has_highlights());
        assert_eq!(ht.segments.len(), 2);

        let no_match = HighlightedText::from_search("Hello", "xyz");
        assert!(!no_match.has_highlights());
    }

    #[test]
    fn test_highlighted_text_empty_query() {
        let ht = HighlightedText::from_search("Hello World", "");
        assert!(!ht.has_highlights());
        assert_eq!(ht.segments.len(), 1);
    }

    #[test]
    fn test_highlighted_text_multiple_matches() {
        let ht = HighlightedText::from_search("hello hello hello", "hello");
        assert!(ht.has_highlights());
        assert_eq!(ht.segments.iter().filter(|(_, h)| *h).count(), 3);
    }
}
