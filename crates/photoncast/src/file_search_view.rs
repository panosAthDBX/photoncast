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
use photoncast_core::platform::file_browser::{DirectoryEntry, FileBrowser};
use photoncast_core::platform::spotlight::{FileKind, FileResult};
use photoncast_core::theme::PhotonTheme;

use crate::constants::{
    DETAIL_PANEL_WIDTH, LIST_ITEM_HEIGHT, LIST_PANEL_WIDTH, SEARCH_BAR_HEIGHT,
    SECTION_HEADER_HEIGHT,
};

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

    /// Returns the mdfind query string for this filter type
    pub fn mdfind_query(&self) -> &'static str {
        match self {
            Self::All => "kMDItemFSContentChangeDate >= $time.today(-7)",
            Self::Documents => "kMDItemContentTypeTree == 'public.content' && (kMDItemContentType == 'com.adobe.pdf' || kMDItemContentType == 'public.plain-text' || kMDItemContentType == 'org.openxmlformats.wordprocessingml.document' || kMDItemContentType == 'com.microsoft.word.doc' || kMDItemContentType == 'public.rtf' || kMDItemContentType == 'net.daringfireball.markdown')",
            Self::Images => "kMDItemContentTypeTree == 'public.image'",
            Self::Videos => "kMDItemContentTypeTree == 'public.movie'",
            Self::Audio => "kMDItemContentTypeTree == 'public.audio'",
            Self::Archives => "kMDItemContentType == 'public.archive' || kMDItemContentType == 'com.apple.disk-image' || kMDItemContentType == 'public.zip-archive' || kMDItemContentType == 'org.gnu.gnu-tar-archive'",
            Self::Code => "kMDItemContentType == 'public.source-code' || kMDItemContentType == 'public.script'",
            Self::Folders => "kMDItemContentType == 'public.folder'",
        }
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
    /// Browsing a directory (query starts with /, ~, or $)
    Browsing,
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
    /// All results (unfiltered)
    pub all_results: Vec<FileResult>,
    /// Filtered results for display
    pub results: Vec<FileResult>,
    /// Currently selected index
    pub selected_index: usize,
    /// Dropdown selected index (for keyboard navigation)
    pub dropdown_index: usize,
    /// Whether we're loading results
    pub loading: bool,
    /// Section mode: "recent" when query is empty, "search" otherwise
    pub section_mode: SectionMode,
    /// Flag to signal the window should be closed
    pub should_close: bool,
    /// Flag to signal filter changed and needs re-fetch (set when filter changes while showing recent files)
    pub needs_refetch: bool,
    /// Flag to signal query changed and needs search
    pub query_changed: bool,
    /// Flag to signal Cmd+Enter (reveal in Finder)
    pub wants_reveal_in_finder: bool,
    /// Flag to signal Cmd+Y (Quick Look)
    pub wants_quick_look: bool,
    /// Flag to signal Cmd+K (show actions menu)
    pub wants_actions_menu: bool,
    /// Flag to signal Enter (open file with default app)
    pub wants_open_file: bool,
    /// Whether the file actions menu is visible
    pub actions_menu_open: bool,
    /// Selected index in the actions menu
    pub actions_menu_index: usize,
    /// Scroll handle for keyboard navigation
    pub scroll_handle: gpui::ScrollHandle,
    // ========== Browsing Mode State ==========
    /// Current browsing path (when in browsing mode)
    pub browse_path: Option<std::path::PathBuf>,
    /// Directory entries for browsing mode
    pub browse_entries: Vec<DirectoryEntry>,
    /// Filter text within browsing mode (typed after path)
    pub browse_filter: String,
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
            all_results: Vec::new(),
            results: Vec::new(),
            selected_index: 0,
            dropdown_index: 0,
            loading: false,
            section_mode: SectionMode::Recent,
            should_close: false,
            needs_refetch: false,
            query_changed: false,
            wants_reveal_in_finder: false,
            wants_quick_look: false,
            wants_actions_menu: false,
            wants_open_file: false,
            actions_menu_open: false,
            actions_menu_index: 0,
            scroll_handle: gpui::ScrollHandle::new(),
            browse_path: None,
            browse_entries: Vec::new(),
            browse_filter: String::new(),
        }
    }

    /// Gets the currently selected file
    pub fn selected_file(&self) -> Option<&FileResult> {
        self.results.get(self.selected_index)
    }

    /// Handles selection change - move down
    pub fn select_next(&mut self, cx: &mut ViewContext<Self>) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
            self.ensure_selected_visible();
            cx.notify();
        }
    }

    /// Handles selection change - move up
    pub fn select_previous(&mut self, cx: &mut ViewContext<Self>) {
        if !self.results.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.results.len() - 1
            } else {
                self.selected_index - 1
            };
            self.ensure_selected_visible();
            cx.notify();
        }
    }

    /// Unified navigation handler for moving to next item.
    /// Handles actions menu, dropdown, and main list navigation.
    pub fn navigate_next(&mut self, cx: &mut ViewContext<Self>) {
        if self.actions_menu_open {
            let count = Self::FILE_ACTIONS.len();
            self.actions_menu_index = (self.actions_menu_index + 1) % count;
        } else if self.dropdown_open {
            let options = FileTypeFilter::all_options();
            self.dropdown_index = (self.dropdown_index + 1) % options.len();
        } else {
            self.select_next(cx);
            return; // select_next calls cx.notify()
        }
        cx.notify();
    }

    /// Unified navigation handler for moving to previous item.
    /// Handles actions menu, dropdown, and main list navigation.
    pub fn navigate_previous(&mut self, cx: &mut ViewContext<Self>) {
        if self.actions_menu_open {
            let count = Self::FILE_ACTIONS.len();
            self.actions_menu_index = if self.actions_menu_index == 0 {
                count - 1
            } else {
                self.actions_menu_index - 1
            };
        } else if self.dropdown_open {
            let options = FileTypeFilter::all_options();
            self.dropdown_index = if self.dropdown_index == 0 {
                options.len() - 1
            } else {
                self.dropdown_index - 1
            };
        } else {
            self.select_previous(cx);
            return; // select_previous calls cx.notify()
        }
        cx.notify();
    }

    // ========== Browsing Mode Methods ==========

    /// Checks if the current query should trigger browsing mode.
    /// Returns true if query starts with /, ~, or $.
    pub fn should_enter_browsing_mode(&self) -> bool {
        FileBrowser::is_browsing_mode(&self.query)
    }

    /// Enters browsing mode for the given path.
    /// Loads directory contents and updates the UI state.
    pub fn enter_browsing_mode(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(path) = FileBrowser::parse_path(&self.query) {
            // Determine if we're browsing a directory or filtering within one
            let (browse_path, filter) = if path.is_dir() {
                (path, String::new())
            } else if let Some(parent) = path.parent() {
                // Path is partial - use parent as browse path and filename as filter
                let filter = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                (parent.to_path_buf(), filter)
            } else {
                (path, String::new())
            };

            self.browse_path = Some(browse_path.clone());
            self.browse_filter = filter;
            self.section_mode = SectionMode::Browsing;
            self.selected_index = 0;

            // Load directory contents
            self.load_browse_directory(&browse_path);
            cx.notify();
        }
    }

    /// Loads directory contents for browsing mode.
    fn load_browse_directory(&mut self, path: &std::path::Path) {
        match FileBrowser::list_directory(path) {
            Ok(entries) => {
                self.browse_entries = entries;
                self.apply_browse_filter();
            }
            Err(_) => {
                self.browse_entries = Vec::new();
            }
        }
    }

    /// Applies the browse filter to directory entries and converts to FileResult for display.
    fn apply_browse_filter(&mut self) {
        let filter_lower = self.browse_filter.to_lowercase();

        // Convert DirectoryEntry to FileResult for unified display
        self.results = self
            .browse_entries
            .iter()
            .filter(|entry| {
                if filter_lower.is_empty() {
                    true
                } else {
                    entry.name.to_lowercase().contains(&filter_lower)
                }
            })
            .map(|entry| FileResult {
                path: entry.path.clone(),
                name: entry.name.clone(),
                kind: entry.kind,
                size: entry.size,
                modified: entry.modified,
            })
            .collect();

        // Reset selection if out of bounds
        if self.selected_index >= self.results.len() {
            self.selected_index = 0;
        }
    }

    /// Handles Tab key in browsing mode - enters the selected folder.
    pub fn browse_enter_folder(&mut self, cx: &mut ViewContext<Self>) {
        if self.section_mode != SectionMode::Browsing {
            return;
        }

        if let Some(file) = self.selected_file() {
            let path = file.path.clone();

            if path.is_dir() {
                // Enter the directory
                self.browse_path = Some(path.clone());
                self.browse_filter.clear();

                // Update query to show full path
                let display_path = if let Some(home) = dirs::home_dir() {
                    if path.starts_with(&home) {
                        format!("~/{}", path.strip_prefix(&home).unwrap().display())
                    } else {
                        path.display().to_string()
                    }
                } else {
                    path.display().to_string()
                };
                // Ensure trailing slash for directories
                let display_path = if display_path.ends_with('/') {
                    display_path
                } else {
                    format!("{}/", display_path)
                };
                self.query = SharedString::from(display_path.clone());
                self.cursor_position = display_path.chars().count();

                // Load new directory contents
                self.load_browse_directory(&path);
                self.selected_index = 0;
                cx.notify();
            } else {
                // File selected - expand to full path in query
                let display_path = if let Some(home) = dirs::home_dir() {
                    if path.starts_with(&home) {
                        format!("~/{}", path.strip_prefix(&home).unwrap().display())
                    } else {
                        path.display().to_string()
                    }
                } else {
                    path.display().to_string()
                };
                self.query = SharedString::from(display_path.clone());
                self.cursor_position = display_path.chars().count();
                cx.notify();
            }
        }
    }

    /// Handles Shift+Tab in browsing mode - goes to parent directory.
    pub fn browse_go_back(&mut self, cx: &mut ViewContext<Self>) {
        if self.section_mode != SectionMode::Browsing {
            return;
        }

        if let Some(current_path) = &self.browse_path.clone() {
            if let Some(parent) = current_path.parent() {
                // Don't go above root
                if parent.as_os_str().is_empty() {
                    return;
                }

                self.browse_path = Some(parent.to_path_buf());
                self.browse_filter.clear();

                // Update query to show parent path
                let display_path = if let Some(home) = dirs::home_dir() {
                    if parent.starts_with(&home) {
                        if parent == home {
                            "~/".to_string()
                        } else {
                            format!("~/{}/", parent.strip_prefix(&home).unwrap().display())
                        }
                    } else {
                        format!("{}/", parent.display())
                    }
                } else {
                    format!("{}/", parent.display())
                };
                self.query = SharedString::from(display_path.clone());
                self.cursor_position = display_path.chars().count();

                // Load parent directory contents
                self.load_browse_directory(parent);
                self.selected_index = 0;
                cx.notify();
            }
        }
    }

    /// Exits browsing mode and returns to normal search.
    pub fn exit_browsing_mode(&mut self, cx: &mut ViewContext<Self>) {
        self.section_mode = if self.query.is_empty() {
            SectionMode::Recent
        } else {
            SectionMode::Search
        };
        self.browse_path = None;
        self.browse_entries.clear();
        self.browse_filter.clear();
        self.query_changed = true;
        cx.notify();
    }

    /// Gets the selected browse entry (for browsing mode).
    pub fn selected_browse_entry(&self) -> Option<&DirectoryEntry> {
        if self.section_mode == SectionMode::Browsing {
            // Find the original entry that matches the selected result
            if let Some(selected) = self.results.get(self.selected_index) {
                return self
                    .browse_entries
                    .iter()
                    .find(|e| e.path == selected.path);
            }
        }
        None
    }

    /// Ensure the selected item is visible by scrolling if needed.
    /// Uses scroll_to_item which centers/shows the item.
    fn ensure_selected_visible(&mut self) {
        // Use GPUI's built-in scroll_to_item which handles visibility automatically
        self.scroll_handle.scroll_to_item(self.selected_index);
    }

    /// Toggles the filter dropdown
    pub fn toggle_dropdown(&mut self, cx: &mut ViewContext<Self>) {
        self.dropdown_open = !self.dropdown_open;
        cx.notify();
    }

    /// Sets the file type filter and applies it to the results
    pub fn set_filter(&mut self, filter: FileTypeFilter, cx: &mut ViewContext<Self>) {
        let filter_changed = self.filter != filter;
        self.filter = filter;
        self.dropdown_open = false;
        self.selected_index = 0;
        
        // Signal that we need to re-fetch files for the new filter type
        // (when showing recent files, we want the most recent files of the filtered type)
        if filter_changed && self.section_mode == SectionMode::Recent {
            self.needs_refetch = true;
            self.loading = true;
            // Don't apply_filter here - let the refetch handler set the results
            // This avoids briefly showing 0 results while loading
        } else {
            // For search mode or same filter, apply immediately
            self.apply_filter();
        }
        
        cx.notify();
    }

    /// Applies the current filter to all_results and updates results
    pub fn apply_filter(&mut self) {
        let filter = self.filter;
        let before_count = self.all_results.len();
        
        self.results = self
            .all_results
            .iter()
            .filter(|file| filter.matches(file.kind, &file.path))
            .cloned()
            .collect();
        
        tracing::debug!(
            "[Filter] {:?}: {} -> {} files",
            filter,
            before_count,
            self.results.len()
        );
    }

    /// Sets results and applies the current filter
    pub fn set_results(&mut self, results: Vec<FileResult>) {
        tracing::debug!("[FileSearch] set_results: {} files", results.len());
        self.all_results = results;
        self.apply_filter();
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
        let _text_placeholder = colors.text_placeholder;
        let accent = colors.accent;

        // Block cursor dimensions (like main modal / Ghostty terminal)
        let cursor_width = px(9.0);
        let cursor_height = px(20.0);

        div()
            .h(SEARCH_BAR_HEIGHT)
            .flex_shrink_0() // Don't shrink search bar
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            // No background - matches main launcher style
            // Search input area with cursor
            .child(
                div()
                    .flex_1()
                    .text_size(px(16.0))
                    .flex()
                    .items_center()
                    .when(self.query.is_empty(), |el| {
                        el.child(
                            div()
                                .w(cursor_width)
                                .h(cursor_height)
                                .bg(accent)
                                .rounded(px(2.0)),
                        )
                    })
                    .when(!self.query.is_empty(), |el| {
                        // Show text before cursor
                        let chars: Vec<char> = self.query.chars().collect();
                        let before: String = chars[..self.cursor_position].iter().collect();
                        let after: String = chars[self.cursor_position..].iter().collect();
                        
                        el.text_color(text_color)
                            .when(!before.is_empty(), |el| el.child(before))
                            .child(
                                div()
                                    .w(cursor_width)
                                    .h(cursor_height)
                                    .bg(accent)
                                    .rounded(px(2.0)),
                            )
                            .when(!after.is_empty(), |el| el.child(after))
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
                    .on_click(cx.listener(|this, _, cx| this.toggle_dropdown(cx)))
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
                    .text_color(text_muted)
                    .child("esc to close"),
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

    /// Check if a file is a previewable image
    fn is_previewable_image(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "ico"
                )
            })
    }

    /// Check if a file is a previewable text file
    fn is_previewable_text(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_lowercase().as_str(),
                    "txt" | "md" | "markdown" | "rs" | "js" | "ts" | "py" | "rb" | "go" 
                    | "java" | "c" | "cpp" | "h" | "hpp" | "swift" | "kt" | "json" 
                    | "yaml" | "yml" | "toml" | "xml" | "html" | "css" | "sh" | "bash"
                    | "zsh" | "fish" | "log" | "csv"
                )
            })
    }

    /// Read first N lines of a text file for preview
    fn read_text_preview(path: &std::path::Path, max_lines: usize) -> Option<String> {
        use std::io::{BufRead, BufReader};
        let file = std::fs::File::open(path).ok()?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .take(max_lines)
            .filter_map(|l| l.ok())
            .collect();
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
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

        // Check if this is a previewable image - try to render actual preview
        if Self::is_previewable_image(&file.path) && file.path.exists() {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .p_2()
                .child(
                    img(file.path.clone())
                        .max_w(px(280.0))
                        .max_h(px(180.0))
                        .object_fit(ObjectFit::Contain)
                        .rounded(px(4.0)),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(colors.text_placeholder)
                        .child("Press ⌘Y for Quick Look"),
                )
                .into_any_element();
        }

        // Check if this is a previewable text file - show first few lines
        if Self::is_previewable_text(&file.path) && file.path.exists() {
            if let Some(preview_text) = Self::read_text_preview(&file.path, 12) {
                return div()
                    .flex()
                    .flex_col()
                    .p_3()
                    .gap_2()
                    .overflow_hidden()
                    .child(
                        div()
                            .w_full()
                            .h_full()
                            .bg(colors.surface)
                            .rounded(px(4.0))
                            .p_2()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_family("SF Mono, Monaco, Menlo, monospace")
                                    .text_color(colors.text_muted)
                                    .overflow_hidden()
                                    .child(preview_text),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_placeholder)
                            .child("Press ⌘Y for Quick Look"),
                    )
                    .into_any_element();
            }
        }

        // Show file icon as preview for non-previewable files (PDFs, docs, etc.)
        let icon_emoji = if file.kind == FileKind::File {
            extension_to_emoji(&file.path)
        } else {
            file_kind_to_emoji(file.kind)
        };

        // For PDFs and documents, show a hint
        let hint = if file.path.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
            matches!(ext.to_lowercase().as_str(), "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "pages" | "numbers" | "key")
        }) {
            "Document - Press ⌘Y to preview"
        } else {
            "Press ⌘Y for Quick Look"
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
                    .child(hint),
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
            SectionMode::Recent => "Recent Files".to_string(),
            SectionMode::Search => "Search Results".to_string(),
            SectionMode::Browsing => {
                // Show the current browsing path
                if let Some(path) = &self.browse_path {
                    let display = if let Some(home) = dirs::home_dir() {
                        if path.starts_with(&home) {
                            if path == &home {
                                "~/".to_string()
                            } else {
                                format!("~/{}", path.strip_prefix(&home).unwrap().display())
                            }
                        } else {
                            path.display().to_string()
                        }
                    } else {
                        path.display().to_string()
                    };
                    format!("📁 {}", display)
                } else {
                    "Browsing".to_string()
                }
            }
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

    /// Render the footer with action hints
    fn render_footer(&self, colors: &FileSearchColors) -> impl IntoElement {
        let text_muted = colors.text_muted;
        let border_color = colors.border;
        let surface = colors.surface;
        let is_browsing = self.section_mode == SectionMode::Browsing;

        div()
            .h(px(36.0))
            .flex_shrink_0()
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(border_color)
            .bg(surface)
            // Left side: primary actions
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    // Browsing mode: Tab to enter folder
                    .when(is_browsing, |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .px_1()
                                        .py(px(1.0))
                                        .bg(colors.surface_elevated)
                                        .rounded(px(3.0))
                                        .text_size(px(10.0))
                                        .text_color(text_muted)
                                        .child("⇥"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child("Enter"),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .px_1()
                                        .py(px(1.0))
                                        .bg(colors.surface_elevated)
                                        .rounded(px(3.0))
                                        .text_size(px(10.0))
                                        .text_color(text_muted)
                                        .child("⇧⇥"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child("Back"),
                                ),
                        )
                    })
                    // Open action
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .px_1()
                                    .py(px(1.0))
                                    .bg(colors.surface_elevated)
                                    .rounded(px(3.0))
                                    .text_size(px(10.0))
                                    .text_color(text_muted)
                                    .child("↵"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_muted)
                                    .child("Open"),
                            ),
                    )
                    // Quick Look action
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .px_1()
                                    .py(px(1.0))
                                    .bg(colors.surface_elevated)
                                    .rounded(px(3.0))
                                    .text_size(px(10.0))
                                    .text_color(text_muted)
                                    .child("⌘Y"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_muted)
                                    .child("Quick Look"),
                            ),
                    )
                    // Reveal in Finder
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .px_1()
                                    .py(px(1.0))
                                    .bg(colors.surface_elevated)
                                    .rounded(px(3.0))
                                    .text_size(px(10.0))
                                    .text_color(text_muted)
                                    .child("⌘⏎"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_muted)
                                    .child("Reveal"),
                            ),
                    ),
            )
            // Right side: actions menu hint
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .px_1()
                            .py(px(1.0))
                            .bg(colors.surface_elevated)
                            .rounded(px(3.0))
                            .text_size(px(10.0))
                            .text_color(text_muted)
                            .child("⌘K"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child("Actions"),
                    ),
            )
    }

    /// Render the filter dropdown (0.4)
    fn render_filter_dropdown(&self, colors: &FileSearchColors, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let options = FileTypeFilter::all_options();
        let current_filter = self.filter;
        let dropdown_index = self.dropdown_index;
        let text_color = colors.text;
        let accent = colors.accent;
        let surface_hover = colors.surface_hover;
        let selection_bg = colors.selection;

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
            .children(options.iter().enumerate().map(|(idx, &filter)| {
                let is_selected = filter == current_filter;
                let is_highlighted = idx == dropdown_index;

                div()
                    .id(SharedString::from(format!("filter-{:?}", filter)))
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    // Keyboard highlight takes precedence
                    .when(is_highlighted, |el| el.bg(selection_bg))
                    .when(!is_highlighted, |el| el.hover(move |el| el.bg(surface_hover)))
                    .on_click(cx.listener(move |this, _, cx| this.set_filter(filter, cx)))
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

    /// File action definitions
    pub const FILE_ACTIONS: &'static [(&'static str, &'static str, &'static str)] = &[
        ("Open", "↵", "open"),
        ("Reveal in Finder", "⌘⏎", "reveal"),
        ("Quick Look", "⌘Y", "quicklook"),
        ("Copy", "⌘C", "copyfile"),
        ("Copy Path", "⌘⇧C", "copypath"),
        ("Copy Name", "⌘⇧N", "copyname"),
    ];

    /// Render the file actions menu (Cmd+K)
    fn render_actions_menu(&self, colors: &FileSearchColors, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let surface_hover = colors.surface_hover;
        let selection_bg = colors.selection;
        let actions_index = self.actions_menu_index;

        div()
            .absolute()
            .bottom(px(44.0)) // Above footer
            .right(px(16.0))
            .w(px(200.0))
            .bg(colors.surface_elevated)
            .rounded_lg()
            .border_1()
            .border_color(colors.border)
            .shadow_lg()
            .overflow_hidden()
            .py_1()
            .children(Self::FILE_ACTIONS.iter().enumerate().map(|(idx, &(name, shortcut, action_id))| {
                let is_highlighted = idx == actions_index;
                let action_id_owned = action_id.to_string();

                div()
                    .id(SharedString::from(format!("action-{}", action_id)))
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .when(is_highlighted, |el| el.bg(selection_bg))
                    .when(!is_highlighted, |el| el.hover(move |el| el.bg(surface_hover)))
                    .on_click(cx.listener(move |this, _, cx| {
                        this.execute_action(&action_id_owned, cx);
                    }))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(text_color)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child(shortcut),
                    )
            }))
    }

    /// Execute a file action by ID
    pub fn execute_action(&mut self, action_id: &str, cx: &mut ViewContext<Self>) {
        self.actions_menu_open = false;
        match action_id {
            "open" => self.wants_open_file = true,
            "reveal" => self.wants_reveal_in_finder = true,
            "quicklook" => self.wants_quick_look = true,
            "copypath" => {
                if let Some(file) = self.selected_file() {
                    let path_str = file.path.display().to_string();
                    cx.write_to_clipboard(ClipboardItem::new_string(path_str.clone()));
                    tracing::info!("Copied path to clipboard: {}", path_str);
                }
            }
            "copyname" => {
                if let Some(file) = self.selected_file() {
                    cx.write_to_clipboard(ClipboardItem::new_string(file.name.clone()));
                    tracing::info!("Copied name to clipboard: {}", file.name);
                }
            }
            "copyfile" => {
                if let Some(file) = self.selected_file() {
                    // Use NSPasteboard via AppleScript to copy file to clipboard
                    // This method uses the native macOS pasteboard API
                    let path = &file.path;
                    let path_str = path.display().to_string();
                    let escaped_path = path_str.replace('\\', "\\\\").replace('"', "\\\"");
                    let script = format!(
                        r#"use framework "AppKit"
use scripting additions
set thePath to "{}"
set theURL to current application's NSURL's fileURLWithPath:thePath
set thePasteboard to current application's NSPasteboard's generalPasteboard()
thePasteboard's clearContents()
thePasteboard's writeObjects:{{theURL}}"#,
                        escaped_path
                    );
                    match std::process::Command::new("osascript")
                        .arg("-e")
                        .arg(&script)
                        .output()
                    {
                        Ok(output) if output.status.success() => {
                            tracing::info!("Copied file to clipboard: {}", path.display());
                        }
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            tracing::error!("Failed to copy file: {}", stderr);
                        }
                        Err(e) => {
                            tracing::error!("Failed to run osascript: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
        cx.notify();
    }
}

impl Render for FileSearchView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_file_search_colors(cx);
        let has_results = !self.results.is_empty();

        // Pre-render dropdown (needs cx for click handlers)
        let dropdown = if self.dropdown_open {
            Some(self.render_filter_dropdown(&colors, cx).into_any_element())
        } else {
            None
        };

        // Pre-render actions menu (needs cx for click handlers)
        let actions_menu = if self.actions_menu_open && self.selected_file().is_some() {
            Some(self.render_actions_menu(&colors, cx).into_any_element())
        } else {
            None
        };

        // Pre-render footer
        let footer = self.render_footer(&colors);

        div()
            .track_focus(&self.focus_handle)
            .size_full() // Fill parent container
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
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    // Left panel: List (fixed width ~60%)
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
                            .w(LIST_PANEL_WIDTH)
                            .h_full()
                            .flex()
                            .flex_col()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .border_r_1()
                            .border_color(colors.border)
                            // Section header
                            .child(section_header)
                            // Loading indicator
                            .when(self.loading, |el| {
                                el.child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .py_8()
                                        .child(
                                            div()
                                                .text_size(px(14.0))
                                                .text_color(colors.text_muted)
                                                .child("Loading...")
                                        )
                                )
                            })
                            // Results list or empty state
                            .when(!has_results && !self.loading, |el| {
                                el.child(empty_state)
                            })
                            .when(has_results && !self.loading, |el| el.children(file_items))
                    })
                    // Right panel: Detail (remaining space)
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .child(self.render_detail_panel(cx))
                    ),
            )
            // Footer with action hints
            .child(footer)
            // Dropdown overlay (when open)
            .when_some(dropdown, |el, dropdown| {
                el.child(dropdown)
            })
            // Actions menu overlay (when open)
            .when_some(actions_menu, |el, menu| {
                el.child(menu)
            })
            // Note: keyboard events are forwarded from launcher, not handled here directly
    }
}

impl FocusableView for FileSearchView {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl FileSearchView {
    /// Handle keyboard input for the search view
    pub fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        let modifiers = &event.keystroke.modifiers;

        // Cmd+P toggles the filter dropdown
        if modifiers.platform && event.keystroke.key == "p" {
            self.toggle_dropdown(cx);
            return;
        }

        // Cmd+Enter reveals in Finder
        if modifiers.platform && event.keystroke.key == "enter" {
            if self.selected_file().is_some() {
                self.wants_reveal_in_finder = true;
                cx.notify();
            }
            return;
        }

        // Cmd+Y opens Quick Look
        if modifiers.platform && event.keystroke.key == "y" {
            if self.selected_file().is_some() {
                self.wants_quick_look = true;
                cx.notify();
            }
            return;
        }

        // Cmd+K toggles actions menu
        if modifiers.platform && event.keystroke.key == "k" {
            if self.selected_file().is_some() {
                self.actions_menu_open = !self.actions_menu_open;
                self.actions_menu_index = 0;
                cx.notify();
            }
            return;
        }

        match &event.keystroke.key {
            key if key == "down" => {
                if self.actions_menu_open {
                    let count = Self::FILE_ACTIONS.len();
                    self.actions_menu_index = (self.actions_menu_index + 1) % count;
                    cx.notify();
                } else if self.dropdown_open {
                    let options = FileTypeFilter::all_options();
                    self.dropdown_index = (self.dropdown_index + 1) % options.len();
                    cx.notify();
                } else {
                    self.select_next(cx);
                }
            }
            key if key == "up" => {
                if self.actions_menu_open {
                    let count = Self::FILE_ACTIONS.len();
                    self.actions_menu_index = if self.actions_menu_index == 0 {
                        count - 1
                    } else {
                        self.actions_menu_index - 1
                    };
                    cx.notify();
                } else if self.dropdown_open {
                    let options = FileTypeFilter::all_options();
                    self.dropdown_index = if self.dropdown_index == 0 {
                        options.len() - 1
                    } else {
                        self.dropdown_index - 1
                    };
                    cx.notify();
                } else {
                    self.select_previous(cx);
                }
            }
            key if key == "enter" => {
                if self.actions_menu_open {
                    // Execute the selected action
                    if let Some(&(_, _, action_id)) = Self::FILE_ACTIONS.get(self.actions_menu_index) {
                        self.execute_action(action_id, cx);
                    }
                } else if self.dropdown_open {
                    let options = FileTypeFilter::all_options();
                    if let Some(&filter) = options.get(self.dropdown_index) {
                        self.set_filter(filter, cx);
                    }
                } else if self.selected_file().is_some() {
                    // Open the selected file with default app
                    self.wants_open_file = true;
                    cx.notify();
                }
            }
            key if key == "escape" => {
                if self.actions_menu_open {
                    self.actions_menu_open = false;
                    cx.notify();
                } else if self.dropdown_open {
                    self.dropdown_open = false;
                    cx.notify();
                } else if self.section_mode == SectionMode::Browsing {
                    // Exit browsing mode on Escape
                    self.query = SharedString::default();
                    self.cursor_position = 0;
                    self.exit_browsing_mode(cx);
                } else {
                    self.should_close = true;
                    cx.notify();
                }
            }
            key if key == "tab" => {
                if modifiers.shift {
                    // Shift+Tab: go back to parent directory in browsing mode
                    if self.section_mode == SectionMode::Browsing {
                        self.browse_go_back(cx);
                    }
                } else {
                    // Tab: enter folder in browsing mode
                    if self.section_mode == SectionMode::Browsing {
                        self.browse_enter_folder(cx);
                    }
                }
            }
            key if key == "backspace" => {
                if !self.query.is_empty() {
                    let mut chars: Vec<char> = self.query.chars().collect();
                    if self.cursor_position > 0 {
                        chars.remove(self.cursor_position - 1);
                        self.cursor_position -= 1;
                        self.query = SharedString::from(chars.into_iter().collect::<String>());

                        // Check if we should stay in browsing mode or exit
                        if self.should_enter_browsing_mode() {
                            // Still a valid path prefix - update browsing
                            self.enter_browsing_mode(cx);
                        } else if self.query.is_empty() {
                            if self.section_mode == SectionMode::Browsing {
                                self.exit_browsing_mode(cx);
                            }
                            self.section_mode = SectionMode::Recent;
                            self.query_changed = true;
                        } else {
                            if self.section_mode == SectionMode::Browsing {
                                self.exit_browsing_mode(cx);
                            }
                            self.section_mode = SectionMode::Search;
                            self.query_changed = true;
                        }
                        cx.notify();
                    }
                }
            }
            key if key == "left" => {
                // Left arrow: move cursor left
                if modifiers.platform {
                    self.cursor_position = 0;
                } else if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                cx.notify();
            }
            key if key == "right" => {
                // Right arrow: move cursor right
                let len = self.query.chars().count();
                if modifiers.platform {
                    self.cursor_position = len;
                } else if self.cursor_position < len {
                    self.cursor_position += 1;
                }
                cx.notify();
            }
            key => {
                // Ignore modifier combinations (except shift for uppercase)
                if modifiers.platform || modifiers.control || modifiers.alt {
                    return;
                }

                // Handle regular character input
                let input_text = if let Some(ime_key) = &event.keystroke.ime_key {
                    Some(ime_key.clone())
                } else if key.len() == 1 {
                    let ch = if modifiers.shift {
                        key.to_uppercase()
                    } else {
                        key.to_string()
                    };
                    Some(ch)
                } else {
                    None
                };

                if let Some(text) = input_text {
                    let mut chars: Vec<char> = self.query.chars().collect();
                    for c in text.chars() {
                        chars.insert(self.cursor_position, c);
                        self.cursor_position += 1;
                    }
                    self.query = SharedString::from(chars.into_iter().collect::<String>());

                    // Check if this triggers browsing mode
                    if self.should_enter_browsing_mode() {
                        self.enter_browsing_mode(cx);
                    } else {
                        self.section_mode = SectionMode::Search;
                        self.query_changed = true;
                    }
                    cx.notify();
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================
// Note: Tests are disabled in the binary crate due to GPUI macro recursion limits.
// Pure utility functions like format_relative_date, format_file_size, and FileTypeFilter
// can be tested through integration tests or in a separate test-only crate if needed.
