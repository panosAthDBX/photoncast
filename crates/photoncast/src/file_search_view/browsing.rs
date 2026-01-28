//! Directory navigation and browsing logic for the file search view.
//!
//! Contains all methods related to:
//! - Entering/exiting browsing mode
//! - Loading directory contents
//! - Filtering browse entries
//! - Navigating folders (Tab/Shift+Tab)

use gpui::*;

use photoncast_core::platform::file_browser::{DirectoryEntry, FileBrowser};
use photoncast_core::platform::spotlight::FileResult;

use super::{FileSearchView, SectionMode};

// ============================================================================
// Browsing Mode Methods on FileSearchView
// ============================================================================

impl FileSearchView {
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
    pub(super) fn load_browse_directory(&mut self, path: &std::path::Path) {
        match FileBrowser::list_directory(path) {
            Ok(entries) => {
                self.browse_entries = entries;
                self.apply_browse_filter();
            },
            Err(_) => {
                self.browse_entries = Vec::new();
            },
        }
    }

    /// Applies the browse filter to directory entries and converts to FileResult for display.
    pub(super) fn apply_browse_filter(&mut self) {
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
                    if let Ok(stripped) = path.strip_prefix(&home) {
                        format!("~/{}", stripped.display())
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
                    if let Ok(stripped) = path.strip_prefix(&home) {
                        format!("~/{}", stripped.display())
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
                        } else if let Ok(stripped) = parent.strip_prefix(&home) {
                            format!("~/{}/", stripped.display())
                        } else {
                            format!("{}/", parent.display())
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
                return self.browse_entries.iter().find(|e| e.path == selected.path);
            }
        }
        None
    }
}
