//! File type filtering logic for the file search view.
//!
//! Contains:
//! - `FileTypeFilter` enum and all its methods
//! - Filter application methods on `FileSearchView`

use photoncast_core::platform::spotlight::FileKind;

use crate::constants::{
    ARCHIVE_EXTENSIONS, AUDIO_EXTENSIONS, DOCUMENT_EXTENSIONS, IMAGE_EXTENSIONS, VIDEO_EXTENSIONS,
};

use super::{FileSearchView, SectionMode};

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
            Self::Documents => {
                matches!(kind, FileKind::Document)
                    || path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|ext| {
                            DOCUMENT_EXTENSIONS.contains(&ext.to_lowercase().as_str())
                        })
            },
            Self::Images => {
                matches!(kind, FileKind::Image)
                    || path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            },
            Self::Videos => {
                matches!(kind, FileKind::Video)
                    || path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|ext| VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            },
            Self::Audio => {
                matches!(kind, FileKind::Audio)
                    || path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            },
            Self::Archives => path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| ARCHIVE_EXTENSIONS.contains(&ext.to_lowercase().as_str())),
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
// Filter Methods on FileSearchView
// ============================================================================

impl FileSearchView {
    /// Toggles the filter dropdown
    pub fn toggle_dropdown(&mut self, cx: &mut gpui::ViewContext<Self>) {
        self.dropdown_open = !self.dropdown_open;
        cx.notify();
    }

    /// Sets the file type filter and applies it to the results
    pub fn set_filter(&mut self, filter: FileTypeFilter, cx: &mut gpui::ViewContext<Self>) {
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
    pub fn set_results(&mut self, results: Vec<photoncast_core::platform::spotlight::FileResult>) {
        tracing::debug!("[FileSearch] set_results: {} files", results.len());
        self.all_results = results;
        self.apply_filter();
    }
}
