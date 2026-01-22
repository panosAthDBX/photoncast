//! File Search View - Phase 0: UI Foundation
//!
//! This module implements the File Search UI with Raycast parity, featuring:
//! - Split-view layout (60% list, 40% detail panel)
//! - File list items with icons, titles, subtitles, and date accessories
//! - Detail panel with preview and metadata
//! - Search bar with file type dropdown filter
//! - Section headers and empty state

#![allow(clippy::cast_precision_loss)]
#![allow(dead_code)]

use std::time::SystemTime;

use gpui::prelude::FluentBuilder;
use gpui::*;

use photoncast_calendar::chrono::{DateTime, Datelike, Local, Utc};
use photoncast_core::platform::spotlight::{FileKind, FileResult};
use photoncast_core::theme::PhotonTheme;

// ============================================================================
// Constants
// ============================================================================

/// Height of the search bar
const SEARCH_BAR_HEIGHT: Pixels = px(48.0);

/// Height of each list item
const LIST_ITEM_HEIGHT: Pixels = px(56.0);

/// Width of the list panel (60% of ~750px = ~450px)
const LIST_PANEL_WIDTH: Pixels = px(450.0);

/// Width of the detail panel (40% of ~750px = ~300px)
const DETAIL_PANEL_WIDTH: Pixels = px(300.0);

/// Section header height
const SECTION_HEADER_HEIGHT: Pixels = px(28.0);

// ============================================================================
// File Type Filter
// ============================================================================

/// File type filter options for the search dropdown
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileTypeFilter {
    /// Show all file types
    #[default]
    All,
    /// Documents: PDF, DOC, TXT, etc.
    Documents,
    /// Images: JPG, PNG, GIF, etc.
    Images,
    /// Videos: MP4, MOV, AVI, etc.
    Videos,
    /// Audio: MP3, WAV, FLAC, etc.
    Audio,
    /// Archives: ZIP, RAR, TAR, etc.
    Archives,
    /// Code: RS, JS, PY, etc.
    Code,
    /// Folders only
    Folders,
}

impl FileTypeFilter {
    /// Returns the display name for this filter
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::All => "All Files",
            Self::Documents => "Documents",
            Self::Images => "Images",
            Self::Videos => "Videos",
            Self::Audio => "Audio",
            Self::Archives => "Archives",
            Self::Code => "Code",
            Self::Folders => "Folders",
        }
    }

    /// Returns all filter options for the dropdown
    pub const fn all_options() -> &'static [FileTypeFilter] {
        &[
            Self::All,
            Self::Documents,
            Self::Images,
            Self::Videos,
            Self::Audio,
            Self::Archives,
            Self::Code,
            Self::Folders,
        ]
    }

    /// Checks if a file matches this filter
    pub fn matches(&self, kind: FileKind, path: &std::path::Path) -> bool {
        match self {
            Self::All => true,
            Self::Documents => matches!(kind, FileKind::Document)
                || path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| {
                        matches!(
                            ext.to_lowercase().as_str(),
                            "pdf" | "doc"
                                | "docx"
                                | "txt"
                                | "rtf"
                                | "odt"
                                | "pages"
                                | "md"
                                | "xls"
                                | "xlsx"
                                | "numbers"
                                | "ppt"
                                | "pptx"
                                | "key"
                        )
                    }),
            Self::Images => matches!(kind, FileKind::Image)
                || path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| {
                        matches!(
                            ext.to_lowercase().as_str(),
                            "jpg" | "jpeg"
                                | "png"
                                | "gif"
                                | "bmp"
                                | "svg"
                                | "webp"
                                | "ico"
                                | "tiff"
                                | "heic"
                                | "raw"
                        )
                    }),
            Self::Videos => matches!(kind, FileKind::Video)
                || path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| {
                        matches!(
                            ext.to_lowercase().as_str(),
                            "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v"
                        )
                    }),
            Self::Audio => matches!(kind, FileKind::Audio)
                || path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| {
                        matches!(
                            ext.to_lowercase().as_str(),
                            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "aiff"
                        )
                    }),
            Self::Archives => path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| {
                    matches!(
                        ext.to_lowercase().as_str(),
                        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "dmg" | "iso"
                    )
                }),
            Self::Code => path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| {
                    matches!(
                        ext.to_lowercase().as_str(),
                        "rs" | "js"
                            | "ts"
                            | "py"
                            | "rb"
                            | "go"
                            | "java"
                            | "c"
                            | "cpp"
                            | "h"
                            | "swift"
                            | "kt"
                            | "cs"
                            | "php"
                            | "html"
                            | "css"
                            | "json"
                            | "yaml"
                            | "toml"
                    )
                }),
            Self::Folders => matches!(kind, FileKind::Folder),
        }
    }
}

// ============================================================================
// Helper: Theme Colors
// ============================================================================

/// Helper struct holding theme colors for file search UI
#[derive(Clone)]
pub struct FileSearchColors {
    pub background: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_placeholder: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub surface_elevated: Hsla,
    pub border: Hsla,
    pub accent: Hsla,
    pub selection: Hsla,
}

impl FileSearchColors {
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
            selection: theme.colors.selection.to_gpui(),
        }
    }
}

fn get_file_search_colors<V>(cx: &ViewContext<V>) -> FileSearchColors {
    let theme = cx
        .try_global::<PhotonTheme>()
        .cloned()
        .unwrap_or_default();
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

// ============================================================================
// Determines which section header to show
// ============================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SectionMode {
    /// Showing recent files (query is empty)
    #[default]
    Recent,
    /// Showing search results
    Search,
}

// ============================================================================
// Main View: FileSearchView (0.1)
// ============================================================================

/// Main file search view with split-panel layout
///
/// Layout:
/// ```text
/// ┌─────────────────────────────────────────────────────────────────────────────────────────────┐
/// │  🔍  Search files by name...                                        [All Files ▾]  ⌘P     │
/// ├────────────────────────────────────────────────────┬────────────────────────────────────────┤
/// │                                                    │                                        │
/// │  Recent Files                                      │  ┌────────────────────────────────┐   │
/// │  ─────────────────────────────────────────────     │  │                                │   │
/// │  ▸ 📄  presentation.pdf                            │  │      [File Preview Image]      │   │
/// │       ~/Documents                     2 hours ago  │  │         or Quick Look          │   │
/// │                                                    │  │                                │   │
/// │    📄  budget.xlsx                                 │  └────────────────────────────────┘   │
/// │       ~/Documents/Work                Yesterday    │                                        │
/// │                                                    │  ─────────────────────────────────     │
/// │    📁  Projects                                    │  Name          presentation.pdf       │
/// │       ~/Developer                     3 days ago   │  Kind          PDF Document           │
/// │                                                    │  Size          2.4 MB                 │
/// └────────────────────────────────────────────────────┴────────────────────────────────────────┘
/// ```
pub struct FileSearchView {
    /// Current search query
    pub query: SharedString,
    /// Cursor position in query
    pub cursor_position: usize,
    /// Focus handle
    pub focus_handle: FocusHandle,
    /// File type filter
    pub filter: FileTypeFilter,
    /// Whether the filter dropdown is open
    pub dropdown_open: bool,
    /// Search results (recent files when empty, search results otherwise)
    pub results: Vec<FileResult>,
    /// Currently selected index
    pub selected_index: usize,
    /// Whether we're loading results
    pub loading: bool,
    /// Section mode: "recent" when query is empty, "search" otherwise
    pub section_mode: SectionMode,
}

impl FileSearchView {
    /// Creates a new file search view
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        Self {
            query: SharedString::default(),
            cursor_position: 0,
            focus_handle,
            filter: FileTypeFilter::default(),
            dropdown_open: false,
            results: Vec::new(),
            selected_index: 0,
            loading: false,
            section_mode: SectionMode::Recent,
        }
    }

    /// Gets the currently selected file
    pub fn selected_file(&self) -> Option<&FileResult> {
        self.results.get(self.selected_index)
    }

    /// Handles selection change
    pub fn select_next(&mut self, cx: &mut ViewContext<Self>) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
            cx.notify();
        }
    }

    /// Handles selection change
    pub fn select_previous(&mut self, cx: &mut ViewContext<Self>) {
        if !self.results.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.results.len() - 1
            } else {
                self.selected_index - 1
            };
            cx.notify();
        }
    }

    /// Toggles the filter dropdown
    pub fn toggle_dropdown(&mut self, cx: &mut ViewContext<Self>) {
        self.dropdown_open = !self.dropdown_open;
        cx.notify();
    }

    /// Sets the file type filter
    pub fn set_filter(&mut self, filter: FileTypeFilter, cx: &mut ViewContext<Self>) {
        self.filter = filter;
        self.dropdown_open = false;
        // TODO: Trigger re-search with new filter
        cx.notify();
    }

    // ========================================================================
    // Render Helpers
    // ========================================================================

    /// Render the search bar component (0.4)
    fn render_search_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_file_search_colors(cx);
        let filter_name = self.filter.display_name();
        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;

        div()
            .h(SEARCH_BAR_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .bg(colors.surface)
            .border_b_1()
            .border_color(colors.border)
            // Search icon
            .child(
                div()
                    .text_size(px(16.0))
                    .text_color(colors.text_muted)
                    .child("🔍"),
            )
            // Search input area (placeholder for now - actual input handled by parent)
            .child(
                div()
                    .flex_1()
                    .text_size(px(14.0))
                    .when(self.query.is_empty(), |el| {
                        el.text_color(text_placeholder)
                            .child("Search files by name...")
                    })
                    .when(!self.query.is_empty(), |el| {
                        el.text_color(text_color)
                            .child(self.query.to_string())
                    }),
            )
            // Filter dropdown button
            .child(
                div()
                    .id("file-type-filter")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(colors.surface_elevated)
                    .border_1()
                    .border_color(colors.border)
                    .cursor_pointer()
                    .hover(move |el| el.bg(surface_hover))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(text_color)
                            .child(filter_name),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(text_muted)
                            .child("▾"),
                    ),
            )
            // Keyboard hint
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(text_placeholder)
                    .child("⌘P"),
            )
    }

    /// Render a single file list item (0.2)
    fn render_file_item(
        &self,
        file: &FileResult,
        index: usize,
        is_selected: bool,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_file_search_colors(cx);

        // Get icon emoji (fallback)
        let icon_emoji = if file.kind == FileKind::File {
            extension_to_emoji(&file.path)
        } else {
            file_kind_to_emoji(file.kind)
        };

        // Get subtitle (parent folder path, shortened with ~)
        let subtitle = file
            .path
            .parent()
            .map(|p| {
                let path_str = p.display().to_string();
                // Replace home directory with ~
                if let Some(home) = dirs::home_dir() {
                    if let Some(stripped) = path_str.strip_prefix(&home.display().to_string()) {
                        return format!("~{}", stripped);
                    }
                }
                path_str
            })
            .unwrap_or_default();

        // Get relative date
        let date_str = file
            .modified
            .map(format_relative_date)
            .unwrap_or_default();

        let selection_bg = colors.selection;
        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let file_name = file.name.clone();

        div()
            .id(SharedString::from(format!("file-item-{}", index)))
            .h(LIST_ITEM_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .rounded_md()
            .cursor_pointer()
            // Selection background
            .when(is_selected, |el| el.bg(selection_bg))
            .when(!is_selected, |el| el.hover(move |el| el.bg(surface_hover)))
            // Icon
            .child(
                div()
                    .w(px(32.0))
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .child(div().text_size(px(20.0)).child(icon_emoji)),
            )
            // Title and subtitle
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    // Title
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(text_color)
                            .truncate()
                            .child(file_name),
                    )
                    // Subtitle (path)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .truncate()
                            .child(subtitle),
                    ),
            )
            // Accessory (date)
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(text_muted)
                    .child(date_str),
            )
    }

    /// Render the detail panel for the selected file (0.3)
    fn render_detail_panel(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_file_search_colors(cx);
        let selected_file = self.selected_file().cloned();

        div()
            .h_full()
            .w(DETAIL_PANEL_WIDTH)
            .flex()
            .flex_col()
            .bg(colors.surface_elevated)
            .border_l_1()
            .border_color(colors.border)
            // Preview area (60%)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_h(px(150.0))
                    .child(self.render_preview(&selected_file, &colors)),
            )
            // Divider
            .child(div().w_full().h(px(1.0)).bg(colors.border))
            // Metadata section (40%)
            .child(self.render_metadata(&selected_file, &colors))
    }

    /// Render the preview area
    fn render_preview(
        &self,
        file: &Option<FileResult>,
        colors: &FileSearchColors,
    ) -> impl IntoElement {
        let Some(file) = file else {
            // No file selected - show placeholder
            return div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(48.0))
                        .text_color(colors.text_muted)
                        .child("📁"),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(colors.text_placeholder)
                        .child("Select a file to preview"),
                )
                .into_any_element();
        };

        // Show file icon as preview (actual Quick Look integration would be Phase 1+)
        let icon_emoji = if file.kind == FileKind::File {
            extension_to_emoji(&file.path)
        } else {
            file_kind_to_emoji(file.kind)
        };

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(div().text_size(px(64.0)).child(icon_emoji))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(colors.text_placeholder)
                    .child("Press ⌘Y for Quick Look"),
            )
            .into_any_element()
    }

    /// Render metadata fields for the selected file
    fn render_metadata(
        &self,
        file: &Option<FileResult>,
        colors: &FileSearchColors,
    ) -> impl IntoElement {
        let Some(file) = file else {
            return div().p_4().into_any_element();
        };

        let text_color = colors.text;
        let text_muted = colors.text_muted;

        // Build metadata fields
        let mut fields: Vec<(&'static str, String)> = vec![
            ("Name", file.name.clone()),
            ("Kind", kind_description(file.kind, &file.path)),
        ];

        // Size (not for folders)
        if file.kind != FileKind::Folder {
            if let Some(size) = file.size {
                fields.push(("Size", format_file_size(size)));
            }
        }

        // Modified date
        if let Some(modified) = file.modified {
            fields.push(("Modified", format_relative_date(modified)));
        }

        // Where (parent folder)
        if let Some(parent) = file.path.parent() {
            let where_str = {
                let path_str = parent.display().to_string();
                if let Some(home) = dirs::home_dir() {
                    if let Some(stripped) = path_str.strip_prefix(&home.display().to_string()) {
                        format!("~{}", stripped)
                    } else {
                        path_str
                    }
                } else {
                    path_str
                }
            };
            fields.push(("Where", where_str));
        }

        div()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .children(fields.into_iter().map(move |(label, value)| {
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .w(px(70.0))
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child(label),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.0))
                            .text_color(text_color)
                            .truncate()
                            .child(value),
                    )
            }))
            .into_any_element()
    }

    /// Render section header (0.5)
    fn render_section_header(&self, colors: &FileSearchColors) -> impl IntoElement {
        let title = match self.section_mode {
            SectionMode::Recent => "Recent Files",
            SectionMode::Search => "Search Results",
        };

        div()
            .h(SECTION_HEADER_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_muted)
                    .child(title),
            )
    }

    /// Render empty state (0.5)
    fn render_empty_state(&self, colors: &FileSearchColors) -> impl IntoElement {
        let (icon, title, description) = if self.query.is_empty() {
            (
                "🔍",
                "Search for files",
                "Type a file name to start searching",
            )
        } else {
            (
                "📁",
                "No files found",
                "Try a different search term or check your search scope in preferences.",
            )
        };

        div()
            .w_full()
            .py_8()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            // Icon
            .child(div().text_size(px(48.0)).child(icon))
            // Title
            .child(
                div()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.text)
                    .child(title),
            )
            // Description
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(colors.text_muted)
                    .max_w(px(300.0))
                    .child(description),
            )
    }

    /// Render the filter dropdown (0.4)
    fn render_filter_dropdown(&self, colors: &FileSearchColors) -> impl IntoElement {
        let options = FileTypeFilter::all_options();
        let current_filter = self.filter;
        let text_color = colors.text;
        let accent = colors.accent;
        let surface_hover = colors.surface_hover;

        div()
            .absolute()
            .top(SEARCH_BAR_HEIGHT)
            .right(px(16.0))
            .w(px(160.0))
            .bg(colors.surface_elevated)
            .rounded_lg()
            .border_1()
            .border_color(colors.border)
            .shadow_lg()
            .overflow_hidden()
            .py_1()
            .children(options.iter().map(move |&filter| {
                let is_selected = filter == current_filter;

                div()
                    .id(SharedString::from(format!("filter-{:?}", filter)))
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(move |el| el.bg(surface_hover))
                    // Checkmark for selected
                    .child(
                        div()
                            .w(px(16.0))
                            .text_size(px(12.0))
                            .when(is_selected, |el| el.text_color(accent).child("✓"))
                            .when(!is_selected, |el| el.child("")),
                    )
                    // Filter name
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(text_color)
                            .child(filter.display_name()),
                    )
            }))
    }
}

impl Render for FileSearchView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_file_search_colors(cx);
        let has_results = !self.results.is_empty();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.background)
            .relative()
            // Search bar
            .child(self.render_search_bar(cx))
            // Main content area (split view)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    // Left panel: List (60%)
                    .child({
                        let empty_state = self.render_empty_state(&colors);
                        let section_header = self.render_section_header(&colors);
                        let file_items: Vec<_> = if has_results {
                            self.results
                                .iter()
                                .enumerate()
                                .map(|(idx, file)| {
                                    self.render_file_item(file, idx, idx == self.selected_index, cx)
                                        .into_any_element()
                                })
                                .collect()
                        } else {
                            vec![]
                        };

                        div()
                            .id("file-search-list")
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .overflow_y_scroll()
                            // Section header
                            .child(section_header)
                            // Results list or empty state
                            .when(!has_results && !self.loading, |el| {
                                el.child(empty_state)
                            })
                            .when(has_results, |el| el.children(file_items))
                    })
                    // Right panel: Detail (40%)
                    .child(self.render_detail_panel(cx)),
            )
            // Dropdown overlay (when open)
            .when(self.dropdown_open, |el| {
                el.child(self.render_filter_dropdown(&colors))
            })
    }
}

// ============================================================================
// Tests
// ============================================================================
// Note: Tests are disabled in the binary crate due to GPUI macro recursion limits.
// Pure utility functions like format_relative_date, format_file_size, and FileTypeFilter
// can be tested through integration tests or in a separate test-only crate if needed.
