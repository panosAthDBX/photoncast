//! File Search View - Phase 0: UI Foundation
//!
//! This module implements the File Search UI with Raycast parity, featuring:
//! - Split-view layout (60% list, 40% detail panel)
//! - File list items with icons, titles, subtitles, and date accessories
//! - Detail panel with preview and metadata
//! - Search bar with file type dropdown filter
//! - Section headers and empty state

#![allow(clippy::cast_precision_loss)]

mod browsing;
mod filter;
pub mod helpers;
mod render;

use gpui::*;

use photoncast_core::platform::file_browser::DirectoryEntry;
use photoncast_core::platform::spotlight::FileResult;

// Re-export public types at module level for external consumers
pub use filter::FileTypeFilter;

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

    /// Ensure the selected item is visible by scrolling if needed.
    /// Uses scroll_to_item which centers/shows the item.
    fn ensure_selected_visible(&mut self) {
        // Use GPUI's built-in scroll_to_item which handles visibility automatically
        self.scroll_handle.scroll_to_item(self.selected_index);
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
            },
            "copyname" => {
                if let Some(file) = self.selected_file() {
                    cx.write_to_clipboard(ClipboardItem::new_string(file.name.clone()));
                    tracing::info!("Copied name to clipboard: {}", file.name);
                }
            },
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
                        },
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            tracing::error!("Failed to copy file: {}", stderr);
                        },
                        Err(e) => {
                            tracing::error!("Failed to run osascript: {}", e);
                        },
                    }
                }
            },
            _ => {},
        }
        cx.notify();
    }
}

impl FocusableView for FileSearchView {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ============================================================================
// Keyboard Input Handling
// ============================================================================

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
            },
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
            },
            key if key == "enter" => {
                if self.actions_menu_open {
                    // Execute the selected action
                    if let Some(&(_, _, action_id)) =
                        Self::FILE_ACTIONS.get(self.actions_menu_index)
                    {
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
            },
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
            },
            key if key == "tab" => {
                // Tab navigation is handled by NextGroup/PreviousGroup actions in launcher.rs
                // which check for browsing mode and call browse_enter_folder/browse_go_back
            },
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
            },
            key if key == "left" => {
                // Left arrow: move cursor left
                if modifiers.platform {
                    self.cursor_position = 0;
                } else if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                cx.notify();
            },
            key if key == "right" => {
                // Right arrow: move cursor right
                let len = self.query.chars().count();
                if modifiers.platform {
                    self.cursor_position = len;
                } else if self.cursor_position < len {
                    self.cursor_position += 1;
                }
                cx.notify();
            },
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
            },
        }
    }
}

// ============================================================================
// Tests
// ============================================================================
// Note: Tests are disabled in the binary crate due to GPUI macro recursion limits.
// Pure utility functions like format_relative_date, format_file_size, and FileTypeFilter
// can be tested through integration tests or in a separate test-only crate if needed.
