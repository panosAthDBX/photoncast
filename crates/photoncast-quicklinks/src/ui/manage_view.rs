//! Quick Links management view for preferences/settings.
//!
//! Provides a comprehensive UI for managing user quick links, including:
//! - List view of all quick links with search/filter
//! - Library browser for bundled quick links
//! - Create, edit, delete, and duplicate actions
//! - Keyboard navigation

use gpui::prelude::*;
use gpui::{
    actions, div, px, rgba, AppContext, EventEmitter, FocusHandle, FocusableView, FontWeight, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, ViewContext,
};
use photoncast_theme::{GpuiThemeColors, PhotonTheme};

/// Helper to create a color with adjusted alpha.
fn color_with_alpha(color: Hsla, alpha: f32) -> Hsla {
    gpui::hsla(color.h, color.s, color.l, alpha)
}

use crate::library::{get_bundled_quicklinks, get_categories, BundledQuickLink};
use crate::models::{QuickLink, QuickLinkIcon, QuickLinkId};

// ============================================================================
// Actions
// ============================================================================

actions!(
    quicklinks_manage,
    [
        SelectNext,
        SelectPrevious,
        EditSelected,
        DeleteSelected,
        CreateNew,
        FocusSearch,
        CloseView,
    ]
);

// ============================================================================
// Events
// ============================================================================

/// Events emitted by the manage view.
#[derive(Debug, Clone)]
pub enum ManageViewEvent {
    /// Request to show the create view.
    ShowCreateView,
    /// Request to show the edit view for a specific quicklink.
    ShowEditView(QuickLink),
    /// Request to delete a quicklink.
    DeleteQuicklink(QuickLinkId),
    /// Request to duplicate a quicklink.
    DuplicateQuicklink(QuickLink),
    /// Request to add a bundled quicklink.
    AddFromLibrary(BundledQuickLink),
    /// Request to close the view.
    Close,
}

// ============================================================================
// Theme Colors
// ============================================================================

/// Type alias – manage view uses the shared [`GpuiThemeColors`] from photoncast-theme.
type ManageColors = GpuiThemeColors;

fn get_colors(cx: &ViewContext<QuicklinksManageView>) -> ManageColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    GpuiThemeColors::from_theme(&theme)
}

// ============================================================================
// Main View
// ============================================================================

/// State for inline editing a quicklink.
/// Which field is focused in the edit modal.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum EditField {
    #[default]
    Name,
    Link,
    Alias,
}

/// State for inline editing a quicklink.
#[derive(Clone)]
struct EditingState {
    /// The quicklink being edited (with original ID).
    original_id: QuickLinkId,
    /// Edited name.
    name: String,
    /// Edited URL/link.
    link: String,
    /// Edited alias.
    alias: String,
    /// Whether this is a new quicklink (not yet saved).
    is_new: bool,
    /// Which field is currently focused.
    focused_field: EditField,
    /// Cursor position in name field.
    name_cursor: usize,
    /// Cursor position in link field.
    link_cursor: usize,
    /// Cursor position in alias field.
    alias_cursor: usize,
}

/// Quick Links management view state.
pub struct QuicklinksManageView {
    /// All user quicklinks.
    quicklinks: Vec<QuickLink>,
    /// Currently selected index in the list.
    selected_index: Option<usize>,
    /// Whether to show the bundled library browser.
    show_library: bool,
    /// Search/filter query.
    search_query: String,
    /// Cursor position in search query.
    search_cursor: usize,
    /// ID of quicklink pending delete confirmation.
    confirm_delete: Option<QuickLinkId>,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Selected category in library view.
    selected_library_category: Option<String>,
    /// Selected index in library view.
    library_selected_index: Option<usize>,
    /// Storage for persisting changes.
    storage: Option<crate::QuickLinksStorage>,
    /// Runtime for async operations.
    runtime: Option<std::sync::Arc<tokio::runtime::Runtime>>,
    /// Current inline editing state.
    editing: Option<EditingState>,
    /// Scroll handle for library list.
    library_scroll_handle: gpui::ScrollHandle,
    /// Scroll handle for quicklinks list.
    quicklinks_scroll_handle: gpui::ScrollHandle,
    /// Callback called when quicklinks are modified (added, updated, deleted).
    on_change_callback: Option<Box<dyn Fn() + Send + 'static>>,
}

impl EventEmitter<ManageViewEvent> for QuicklinksManageView {}

impl QuicklinksManageView {
    /// Creates a new quicklinks manage view.
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        Self {
            quicklinks: Vec::new(),
            selected_index: None,
            show_library: false,
            search_query: String::new(),
            search_cursor: 0,
            confirm_delete: None,
            focus_handle,
            selected_library_category: None,
            library_selected_index: None,
            storage: None,
            runtime: None,
            editing: None,
            library_scroll_handle: gpui::ScrollHandle::new(),
            quicklinks_scroll_handle: gpui::ScrollHandle::new(),
            on_change_callback: None,
        }
    }

    /// Sets a callback to be called when quicklinks are modified.
    pub fn on_change(&mut self, callback: impl Fn() + Send + 'static) {
        self.on_change_callback = Some(Box::new(callback));
    }

    /// Notifies the change callback if set.
    fn notify_change(&self) {
        if let Some(callback) = &self.on_change_callback {
            callback();
        }
    }

    /// Sets the storage for persisting changes.
    pub fn set_storage(
        &mut self,
        storage: crate::QuickLinksStorage,
        runtime: std::sync::Arc<tokio::runtime::Runtime>,
    ) {
        self.storage = Some(storage);
        self.runtime = Some(runtime);
    }

    /// Adds a quicklink from the bundled library.
    pub fn add_from_library(&mut self, bundled: &BundledQuickLink, cx: &mut ViewContext<Self>) {
        let quicklink = crate::library::to_quicklink(bundled);
        let temp_id = quicklink.id.clone();

        // Add to local list immediately for responsive UI
        self.quicklinks.push(quicklink.clone());
        self.notify_change();
        cx.notify();

        // Save to storage in background and update the ID with the DB-generated one
        if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
            let storage = storage.clone();
            let runtime = runtime.clone();
            cx.spawn(|this, mut cx| async move {
                let result = cx
                    .background_executor()
                    .spawn(async move { runtime.block_on(storage.store(&quicklink)) })
                    .await;
                match result {
                    Ok(new_id) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            if let Some(link) = this.quicklinks.iter_mut().find(|l| l.id == temp_id)
                            {
                                link.id = new_id;
                            }
                            cx.notify();
                        });
                    },
                    Err(e) => {
                        tracing::error!("Failed to save quicklink: {}", e);
                        // Remove the optimistically added quicklink
                        let _ = this.update(&mut cx, |this, cx| {
                            this.quicklinks.retain(|l| l.id != temp_id);
                            cx.notify();
                        });
                    },
                }
            })
            .detach();
        }
    }

    /// Deletes a quicklink by ID.
    pub fn delete_quicklink(&mut self, id: &QuickLinkId, cx: &mut ViewContext<Self>) {
        let id = id.clone();

        // Capture the removed quicklink and its position for rollback
        let removed = self
            .quicklinks
            .iter()
            .position(|link| link.id == id)
            .map(|pos| (pos, self.quicklinks.remove(pos)));

        // Adjust selection
        let filtered_len = self.filtered_quicklinks().len();
        if filtered_len == 0 {
            self.selected_index = None;
        } else if let Some(idx) = self.selected_index {
            if idx >= filtered_len {
                self.selected_index = Some(filtered_len - 1);
            }
        }

        self.notify_change();
        cx.notify();

        // Delete from storage in background
        if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
            let storage = storage.clone();
            let runtime = runtime.clone();
            let removed_for_rollback = removed;
            cx.spawn(|this, mut cx| async move {
                let id_clone = id.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { runtime.block_on(storage.delete(&id_clone)) })
                    .await;
                if let Err(e) = result {
                    tracing::error!("Failed to delete quicklink from storage: {}", e);
                    // Rollback: re-insert the removed quicklink
                    if let Some((pos, link)) = removed_for_rollback {
                        let _ = this.update(&mut cx, |this, cx| {
                            let insert_pos = pos.min(this.quicklinks.len());
                            this.quicklinks.insert(insert_pos, link);
                            this.notify_change();
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        }
    }

    /// Duplicates a quicklink.
    pub fn duplicate_quicklink(&mut self, link: &QuickLink, cx: &mut ViewContext<Self>) {
        let mut copy = link.clone();
        copy.name = format!("{} (Copy)", link.name);
        let temp_id = copy.id.clone();

        // Add to local list immediately for responsive UI
        self.quicklinks.push(copy.clone());
        self.notify_change();
        cx.notify();

        // Save to storage in background and update the ID with the DB-generated one
        if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
            let storage = storage.clone();
            let runtime = runtime.clone();
            cx.spawn(|this, mut cx| async move {
                let result = cx
                    .background_executor()
                    .spawn(async move { runtime.block_on(storage.store(&copy)) })
                    .await;
                match result {
                    Ok(new_id) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            if let Some(link) = this.quicklinks.iter_mut().find(|l| l.id == temp_id)
                            {
                                link.id = new_id;
                            }
                            cx.notify();
                        });
                    },
                    Err(e) => {
                        tracing::error!("Failed to save duplicated quicklink: {}", e);
                        // Rollback: remove the optimistically added copy
                        let _ = this.update(&mut cx, |this, cx| {
                            this.quicklinks.retain(|l| l.id != temp_id);
                            this.notify_change();
                            cx.notify();
                        });
                    },
                }
            })
            .detach();
        }
    }

    /// Sets the quicklinks to display.
    pub fn set_quicklinks(&mut self, quicklinks: Vec<QuickLink>, cx: &mut ViewContext<Self>) {
        self.quicklinks = quicklinks;
        self.selected_index = if self.quicklinks.is_empty() {
            None
        } else {
            Some(0)
        };
        cx.notify();
    }

    /// Returns whether the library browser is currently visible.
    #[must_use]
    pub fn is_showing_library(&self) -> bool {
        self.show_library
    }

    /// Returns filtered quicklinks based on search query.
    fn filtered_quicklinks(&self) -> Vec<&QuickLink> {
        if self.search_query.is_empty() {
            self.quicklinks.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.quicklinks
                .iter()
                .filter(|link| {
                    link.name.to_lowercase().contains(&query)
                        || link.link.to_lowercase().contains(&query)
                        || link
                            .alias
                            .as_ref()
                            .is_some_and(|a| a.to_lowercase().contains(&query))
                        || link
                            .keywords
                            .iter()
                            .any(|k| k.to_lowercase().contains(&query))
                        || link.tags.iter().any(|t| t.to_lowercase().contains(&query))
                })
                .collect()
        }
    }

    /// Returns the currently selected quicklink.
    pub fn selected_quicklink(&self) -> Option<&QuickLink> {
        let filtered = self.filtered_quicklinks();
        self.selected_index
            .and_then(|idx| filtered.get(idx).copied())
    }

    /// Checks if a bundled quicklink is already added.
    fn is_bundled_added(&self, bundled: &BundledQuickLink) -> bool {
        self.quicklinks.iter().any(|link| {
            link.link == bundled.link
                || link.alias.as_deref() == bundled.alias
                || link.name == bundled.name
        })
    }

    /// Returns bundled quicklinks for the selected category, filtered by search query.
    fn filtered_library_quicklinks(&self) -> Vec<&'static BundledQuickLink> {
        let all = get_bundled_quicklinks();
        let query = self.search_query.to_lowercase();

        all.iter()
            .filter(|b| {
                // Filter by category if selected
                if let Some(cat) = &self.selected_library_category {
                    if b.category != cat {
                        return false;
                    }
                }
                // Filter by search query if present
                if !query.is_empty() {
                    let matches_name = b.name.to_lowercase().contains(&query);
                    let matches_alias = b.alias.is_some_and(|a| a.to_lowercase().contains(&query));
                    let matches_link = b.link.to_lowercase().contains(&query);
                    let matches_category = b.category.to_lowercase().contains(&query);
                    return matches_name || matches_alias || matches_link || matches_category;
                }
                true
            })
            .collect()
    }

    // ========================================================================
    // Actions
    // ========================================================================

    /// Selects the next item in the list.
    pub fn select_next(&mut self, cx: &mut ViewContext<Self>) {
        if self.show_library {
            let filtered = self.filtered_library_quicklinks();
            if !filtered.is_empty() {
                let current = self.library_selected_index.unwrap_or(0);
                let new_index = (current + 1) % filtered.len();
                self.library_selected_index = Some(new_index);
                self.scroll_to_library_item(new_index);
                cx.notify();
            }
        } else {
            let filtered = self.filtered_quicklinks();
            if !filtered.is_empty() {
                let current = self.selected_index.unwrap_or(0);
                let new_index = (current + 1) % filtered.len();
                self.selected_index = Some(new_index);
                self.scroll_to_quicklinks_item(new_index);
                cx.notify();
            }
        }
    }

    /// Selects the previous item in the list.
    pub fn select_previous(&mut self, cx: &mut ViewContext<Self>) {
        if self.show_library {
            let filtered = self.filtered_library_quicklinks();
            if !filtered.is_empty() {
                let current = self.library_selected_index.unwrap_or(0);
                let new_index = if current == 0 {
                    filtered.len() - 1
                } else {
                    current - 1
                };
                self.library_selected_index = Some(new_index);
                self.scroll_to_library_item(new_index);
                cx.notify();
            }
        } else {
            let filtered = self.filtered_quicklinks();
            if !filtered.is_empty() {
                let current = self.selected_index.unwrap_or(0);
                let new_index = if current == 0 {
                    filtered.len() - 1
                } else {
                    current - 1
                };
                self.selected_index = Some(new_index);
                self.scroll_to_quicklinks_item(new_index);
                cx.notify();
            }
        }
    }

    /// Scrolls to make the selected library item visible.
    fn scroll_to_library_item(&self, index: usize) {
        // Use GPUI's built-in scroll_to_item which handles visibility automatically
        self.library_scroll_handle.scroll_to_item(index);
    }

    /// Scrolls to make the selected quicklinks item visible.
    fn scroll_to_quicklinks_item(&self, index: usize) {
        // Use GPUI's built-in scroll_to_item which handles visibility automatically
        self.quicklinks_scroll_handle.scroll_to_item(index);
    }

    /// Edits the currently selected quicklink.
    pub fn edit_selected(&mut self, cx: &mut ViewContext<Self>) {
        if self.show_library {
            // In library mode, add the selected bundled quicklink
            let filtered = self.filtered_library_quicklinks();
            if let Some(idx) = self.library_selected_index {
                if let Some(&bundled) = filtered.get(idx) {
                    if !self.is_bundled_added(bundled) {
                        self.add_from_library(bundled, cx);
                    }
                }
            }
        } else if let Some(link) = self.selected_quicklink() {
            // Start inline editing
            let name_len = link.name.chars().count();
            let link_len = link.link.chars().count();
            let alias_len = link.alias.as_ref().map_or(0, |a| a.chars().count());
            self.editing = Some(EditingState {
                original_id: link.id.clone(),
                name: link.name.clone(),
                link: link.link.clone(),
                alias: link.alias.clone().unwrap_or_default(),
                is_new: false,
                focused_field: EditField::Name,
                name_cursor: name_len,
                link_cursor: link_len,
                alias_cursor: alias_len,
            });
            cx.notify();
        }
    }

    /// Starts creating a new quicklink inline.
    fn start_new_quicklink(&mut self, cx: &mut ViewContext<Self>) {
        // Try to read URL from clipboard
        let (link_input, link_cursor) = cx
            .read_from_clipboard()
            .and_then(|c| c.text())
            .filter(|text| {
                let t = text.trim();
                t.starts_with("http://")
                    || t.starts_with("https://")
                    || t.starts_with("file://")
                    || t.starts_with("mailto:")
                    || t.starts_with('/')
                    || t.starts_with('~')
                    || (t.contains('.') && !t.contains(' '))
            })
            .map(|text| {
                let trimmed = text.trim().to_string();
                let len = trimmed.chars().count();
                (trimmed, len)
            })
            .unwrap_or_default();

        self.editing = Some(EditingState {
            original_id: QuickLinkId::generate(),
            name: String::new(),
            link: link_input,
            alias: String::new(),
            is_new: true,
            focused_field: EditField::Name,
            name_cursor: 0,
            link_cursor,
            alias_cursor: 0,
        });
        cx.notify();
    }

    /// Cancels the current inline edit.
    fn cancel_editing(&mut self, cx: &mut ViewContext<Self>) {
        self.editing = None;
        cx.notify();
    }

    /// Finishes editing and saves changes.
    #[allow(clippy::needless_pass_by_value)]
    fn finish_editing(&mut self, editing: EditingState, cx: &mut ViewContext<Self>) {
        // Validate
        if editing.name.trim().is_empty() || editing.link.trim().is_empty() {
            return;
        }

        if editing.is_new {
            // Create new quicklink
            let mut quicklink = QuickLink::new(editing.name.trim(), editing.link.trim());
            if !editing.alias.trim().is_empty() {
                quicklink.alias = Some(editing.alias.trim().to_string());
            }
            let temp_id = quicklink.id.clone();

            // Add to local list immediately for responsive UI
            self.quicklinks.push(quicklink.clone());
            self.notify_change();
            cx.notify();

            // Save to storage in background
            if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
                let storage = storage.clone();
                let runtime = runtime.clone();
                cx.spawn(|this, mut cx| async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move { runtime.block_on(storage.store(&quicklink)) })
                        .await;
                    match result {
                        Ok(new_id) => {
                            let _ = this.update(&mut cx, |this, cx| {
                                if let Some(link) =
                                    this.quicklinks.iter_mut().find(|l| l.id == temp_id)
                                {
                                    link.id = new_id;
                                }
                                cx.notify();
                            });
                        },
                        Err(e) => {
                            tracing::error!("Failed to save quicklink: {}", e);
                            // Rollback: remove the optimistically added quicklink
                            let _ = this.update(&mut cx, |this, cx| {
                                this.quicklinks.retain(|l| l.id != temp_id);
                                this.notify_change();
                                cx.notify();
                            });
                        },
                    }
                })
                .detach();
            }
        } else {
            // Update existing quicklink — apply changes locally first
            let original_id = editing.original_id.clone();

            // Capture previous state for rollback
            let previous_state = self
                .quicklinks
                .iter()
                .find(|l| l.id == original_id)
                .cloned();

            if let Some(link) = self.quicklinks.iter_mut().find(|l| l.id == original_id) {
                link.name = editing.name.trim().to_string();
                link.link = editing.link.trim().to_string();
                link.alias = if editing.alias.trim().is_empty() {
                    None
                } else {
                    Some(editing.alias.trim().to_string())
                };

                let link_clone = link.clone();
                self.notify_change();
                cx.notify();

                // Persist update to storage in background
                if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
                    let storage = storage.clone();
                    let runtime = runtime.clone();
                    cx.spawn(|this, mut cx| async move {
                        let result = cx
                            .background_executor()
                            .spawn(async move { runtime.block_on(storage.update(&link_clone)) })
                            .await;
                        if let Err(e) = result {
                            tracing::error!("Failed to update quicklink: {}", e);
                            // Rollback to previous state
                            if let Some(prev) = previous_state {
                                let _ = this.update(&mut cx, |this, cx| {
                                    if let Some(link) =
                                        this.quicklinks.iter_mut().find(|l| l.id == original_id)
                                    {
                                        *link = prev;
                                    }
                                    this.notify_change();
                                    cx.notify();
                                });
                            }
                        }
                    })
                    .detach();
                }
            }
        }
    }

    /// Initiates delete for the currently selected quicklink.
    pub fn delete_selected(&mut self, cx: &mut ViewContext<Self>) {
        if self.show_library {
            return; // Cannot delete bundled quicklinks
        }

        if let Some(link) = self.selected_quicklink() {
            if self.confirm_delete.as_ref() == Some(&link.id) {
                // Confirm delete
                let id = link.id.clone();
                self.confirm_delete = None;
                self.delete_quicklink(&id, cx);
                // Adjust selection
                let filtered_len = self.filtered_quicklinks().len();
                if let Some(idx) = self.selected_index {
                    if idx >= filtered_len && filtered_len > 0 {
                        self.selected_index = Some(filtered_len - 1);
                    }
                }
            } else {
                // Request confirmation
                self.confirm_delete = Some(link.id.clone());
            }
            cx.notify();
        }
    }

    /// Duplicates the currently selected quicklink.
    pub fn duplicate_selected(&mut self, cx: &mut ViewContext<Self>) {
        if self.show_library {
            return;
        }

        if let Some(link) = self.selected_quicklink().cloned() {
            self.duplicate_quicklink(&link, cx);
        }
    }

    /// Creates a new quicklink.
    pub fn create_new(&mut self, cx: &mut ViewContext<Self>) {
        self.start_new_quicklink(cx);
    }

    /// Toggles the library browser view.
    pub fn toggle_library(&mut self, cx: &mut ViewContext<Self>) {
        self.show_library = !self.show_library;
        if self.show_library {
            self.library_selected_index = Some(0);
            self.selected_library_category = None;
        }
        cx.notify();
    }

    /// Sets the search query.
    pub fn set_search(&mut self, query: String, cx: &mut ViewContext<Self>) {
        self.search_cursor = query.chars().count();
        self.search_query = query;
        self.selected_index = if self.filtered_quicklinks().is_empty() {
            None
        } else {
            Some(0)
        };
        self.confirm_delete = None;
        cx.notify();
    }

    /// Cancels any pending delete confirmation.
    pub fn cancel_delete(&mut self, cx: &mut ViewContext<Self>) {
        self.confirm_delete = None;
        cx.notify();
    }

    /// Closes the view/window.
    pub fn close(&mut self, cx: &mut ViewContext<Self>) {
        cx.remove_window();
    }

    /// Sets the library category filter.
    pub fn set_library_category(&mut self, category: Option<String>, cx: &mut ViewContext<Self>) {
        self.selected_library_category = category;
        self.library_selected_index = Some(0);
        cx.notify();
    }

    // ========================================================================
    // Keyboard handling
    // ========================================================================

    /// Handles key down events.
    #[allow(clippy::too_many_lines)]
    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        let key = event.keystroke.key.as_str();
        let cmd = event.keystroke.modifiers.platform;
        let shift = event.keystroke.modifiers.shift;

        // Handle editing mode keys first
        if let Some(ref mut editing) = self.editing {
            // Get current field and cursor
            let (field, cursor) = match editing.focused_field {
                EditField::Name => (&mut editing.name, &mut editing.name_cursor),
                EditField::Link => (&mut editing.link, &mut editing.link_cursor),
                EditField::Alias => (&mut editing.alias, &mut editing.alias_cursor),
            };

            match key {
                "escape" => {
                    self.editing = None;
                    cx.notify();
                    return;
                },
                "enter" => {
                    // Save on Enter
                    let editing_clone = editing.clone();
                    self.editing = None;
                    self.finish_editing(editing_clone, cx);
                    return;
                },
                "s" if cmd => {
                    // Save on Cmd+S
                    let editing_clone = editing.clone();
                    self.editing = None;
                    self.finish_editing(editing_clone, cx);
                    return;
                },
                "tab" => {
                    // Cycle through fields
                    editing.focused_field = match editing.focused_field {
                        EditField::Name => EditField::Link,
                        EditField::Link => EditField::Alias,
                        EditField::Alias => EditField::Name,
                    };
                    cx.notify();
                    return;
                },
                "left" => {
                    // Move cursor left
                    if cmd {
                        *cursor = 0; // Move to beginning
                    } else if *cursor > 0 {
                        *cursor -= 1;
                    }
                    cx.notify();
                    return;
                },
                "right" => {
                    // Move cursor right
                    let len = field.chars().count();
                    if cmd {
                        *cursor = len; // Move to end
                    } else if *cursor < len {
                        *cursor += 1;
                    }
                    cx.notify();
                    return;
                },
                "backspace" => {
                    if cmd {
                        // Cmd+Backspace: delete from cursor to beginning
                        let chars: Vec<char> = field.chars().collect();
                        *field = chars[*cursor..].iter().collect();
                        *cursor = 0;
                    } else if *cursor > 0 {
                        // Delete character before cursor
                        let mut chars: Vec<char> = field.chars().collect();
                        chars.remove(*cursor - 1);
                        *field = chars.into_iter().collect();
                        *cursor -= 1;
                    }
                    cx.notify();
                    return;
                },
                "v" if cmd => {
                    // Cmd+V: Paste from clipboard
                    if let Some(clipboard) = cx.read_from_clipboard() {
                        if let Some(text) = clipboard.text() {
                            let chars: Vec<char> = field.chars().collect();
                            let before: String = chars[..*cursor].iter().collect();
                            let after: String = chars[*cursor..].iter().collect();
                            *field = format!("{}{}{}", before, text, after);
                            *cursor += text.chars().count();
                            cx.notify();
                        }
                    }
                    return;
                },
                _ => {
                    // Skip other modifier combinations
                    if cmd || event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
                        return;
                    }

                    // Handle typing into focused field
                    let char_to_add = event.keystroke.ime_key.as_deref().or({
                        if key.len() == 1 {
                            Some(key)
                        } else {
                            None
                        }
                    });

                    if let Some(ch) = char_to_add {
                        let text = if shift {
                            ch.to_uppercase()
                        } else {
                            ch.to_string()
                        };

                        // Insert at cursor position
                        let chars: Vec<char> = field.chars().collect();
                        let before: String = chars[..*cursor].iter().collect();
                        let after: String = chars[*cursor..].iter().collect();
                        *field = format!("{}{}{}", before, text, after);
                        *cursor += text.chars().count();
                        cx.notify();
                    }
                    return;
                },
            }
        }

        match event.keystroke.key.as_str() {
            "down" => {
                self.select_next(cx);
            },
            "up" => {
                self.select_previous(cx);
            },
            "left" => {
                // Move search cursor left
                if cmd {
                    self.search_cursor = 0;
                } else if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                }
                cx.notify();
            },
            "right" => {
                // Move search cursor right
                let len = self.search_query.chars().count();
                if cmd {
                    self.search_cursor = len;
                } else if self.search_cursor < len {
                    self.search_cursor += 1;
                }
                cx.notify();
            },
            "enter" => {
                // If delete confirmation is showing, confirm delete
                if self.confirm_delete.is_some() {
                    self.delete_selected(cx);
                } else {
                    self.edit_selected(cx);
                }
            },
            "backspace" => {
                if cmd {
                    // Cmd+Backspace: delete from cursor to beginning or delete quicklink
                    if self.search_query.is_empty() {
                        self.delete_selected(cx);
                    } else {
                        let chars: Vec<char> = self.search_query.chars().collect();
                        let new_query: String = chars[self.search_cursor..].iter().collect();
                        self.search_cursor = 0;
                        self.search_query = new_query;
                        self.selected_index = if self.filtered_quicklinks().is_empty() {
                            None
                        } else {
                            Some(0)
                        };
                        cx.notify();
                    }
                } else if self.search_cursor > 0 {
                    // Delete character before cursor
                    let mut chars: Vec<char> = self.search_query.chars().collect();
                    chars.remove(self.search_cursor - 1);
                    self.search_cursor -= 1;
                    self.search_query = chars.into_iter().collect();
                    self.selected_index = if self.filtered_quicklinks().is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                    cx.notify();
                }
            },
            "v" if cmd => {
                // Cmd+V: Paste into search
                if let Some(clipboard) = cx.read_from_clipboard() {
                    if let Some(text) = clipboard.text() {
                        let chars: Vec<char> = self.search_query.chars().collect();
                        let before: String = chars[..self.search_cursor].iter().collect();
                        let after: String = chars[self.search_cursor..].iter().collect();
                        let new_query = format!("{}{}{}", before, text, after);
                        self.search_cursor += text.chars().count();
                        self.search_query = new_query;
                        self.selected_index = if self.filtered_quicklinks().is_empty() {
                            None
                        } else {
                            Some(0)
                        };
                        cx.notify();
                    }
                }
            },
            "escape" => {
                if self.confirm_delete.is_some() {
                    self.cancel_delete(cx);
                } else if self.show_library {
                    self.toggle_library(cx);
                } else if !self.search_query.is_empty() {
                    self.set_search(String::new(), cx);
                } else {
                    self.close(cx);
                }
            },
            "n" if cmd => {
                self.create_new(cx);
            },
            "/" => {
                // Focus search - clear and start typing
                self.set_search(String::new(), cx);
            },
            "l" if cmd => {
                self.toggle_library(cx);
            },
            "d" if cmd => {
                self.duplicate_selected(cx);
            },
            _ => {
                // Skip other modifier combinations
                if cmd || event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
                    return;
                }

                // Handle typing into search (insert at cursor)
                let char_to_add = event.keystroke.ime_key.as_deref().or({
                    if event.keystroke.key.len() == 1 {
                        Some(event.keystroke.key.as_str())
                    } else {
                        None
                    }
                });

                if let Some(ch) = char_to_add {
                    let text = if shift {
                        ch.to_uppercase()
                    } else {
                        ch.to_string()
                    };

                    let chars: Vec<char> = self.search_query.chars().collect();
                    let before: String = chars[..self.search_cursor].iter().collect();
                    let after: String = chars[self.search_cursor..].iter().collect();
                    self.search_query = format!("{}{}{}", before, text, after);
                    self.search_cursor += text.chars().count();
                    self.selected_index = if self.filtered_quicklinks().is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                    cx.notify();
                }
            },
        }
    }

    // ========================================================================
    // Render helpers
    // ========================================================================

    /// Renders the header.
    fn render_header(&self, colors: &ManageColors) -> impl IntoElement {
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let border_color = colors.border;

        div()
            .pt(px(36.0)) // Space for traffic lights
            .px(px(16.0))
            .pb(px(12.0))
            .border_b_1()
            .border_color(border_color)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().text_lg().child("🔗"))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(text_color)
                            .child("Quick Links"),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(text_muted)
                    .child(format!("{} links", self.quicklinks.len())),
            )
    }

    /// Renders the search bar.
    fn render_search_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let query = self.search_query.clone();
        let has_query = !query.is_empty();
        let is_search_focused = self.editing.is_none() && !self.show_library;

        // Block cursor dimensions (matches launcher)
        let cursor_width = px(9.0);
        let cursor_height = px(20.0);

        // Split text at cursor position
        let chars: Vec<char> = query.chars().collect();
        let cursor_pos = self.search_cursor.min(chars.len());
        let before: String = chars[..cursor_pos].iter().collect();
        let after: String = chars[cursor_pos..].iter().collect();

        div().px(px(16.0)).py(px(8.0)).child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(6.0))
                .bg(colors.surface)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().text_sm().text_color(colors.text_muted).child("🔍"))
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .flex()
                        .items_center()
                        .when(!has_query && is_search_focused, |el| {
                            // Empty field with focus: cursor then placeholder
                            el.child(
                                div()
                                    .w(cursor_width)
                                    .h(cursor_height)
                                    .bg(colors.accent)
                                    .rounded(px(2.0)),
                            )
                            .child(
                                div()
                                    .text_color(colors.text_placeholder)
                                    .child("Search quick links..."),
                            )
                        })
                        .when(!has_query && !is_search_focused, |el| {
                            el.text_color(colors.text_placeholder)
                                .child("Search quick links...")
                        })
                        .when(has_query, |el| {
                            el.text_color(colors.text)
                                .when(!before.is_empty(), |el| el.child(before.clone()))
                                .when(is_search_focused, |el| {
                                    el.child(
                                        div()
                                            .w(cursor_width)
                                            .h(cursor_height)
                                            .bg(colors.accent)
                                            .rounded(px(2.0)),
                                    )
                                })
                                .when(!after.is_empty(), |el| el.child(after.clone()))
                        }),
                )
                .when(has_query, |el| {
                    let surface_hover = colors.surface_hover;
                    let text_muted = colors.text_muted;
                    el.child(
                        div()
                            .id("clear-search")
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .hover(move |s| s.bg(surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, cx| {
                                this.set_search(String::new(), cx);
                            }))
                            .child(div().text_xs().text_color(text_muted).child("✕")),
                    )
                }),
        )
    }

    /// Renders the toolbar with action buttons.
    fn render_toolbar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let show_library = self.show_library;

        div()
            .px(px(16.0))
            .py(px(8.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            // Create New button
            .child(self.render_toolbar_button(
                "create-new",
                "➕",
                "Create New",
                colors.accent,
                colors.text,
                cx,
                QuicklinksManageView::create_new,
            ))
            // Browse Library button
            .child(self.render_toolbar_button(
                "browse-library",
                "📚",
                if show_library {
                    "Hide Library"
                } else {
                    "Browse Library"
                },
                if show_library {
                    colors.selection
                } else {
                    colors.surface
                },
                colors.text,
                cx,
                QuicklinksManageView::toggle_library,
            ))
            // Import button
            .child(self.render_toolbar_button(
                "import",
                "📥",
                "Import",
                colors.surface,
                colors.text,
                cx,
                |_this, _cx| {
                    // TODO: Implement import
                },
            ))
    }

    /// Renders a toolbar button.
    #[allow(clippy::too_many_arguments, clippy::unused_self)]
    fn render_toolbar_button(
        &self,
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        bg: Hsla,
        text: Hsla,
        cx: &mut ViewContext<Self>,
        on_click: impl Fn(&mut Self, &mut ViewContext<Self>) + 'static,
    ) -> impl IntoElement {
        let colors = get_colors(cx);

        div()
            .id(id)
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(6.0))
            .bg(bg)
            .hover(|s| s.bg(colors.surface_hover))
            .cursor_pointer()
            .flex()
            .items_center()
            .gap(px(6.0))
            .on_click(cx.listener(move |this, _, cx| on_click(this, cx)))
            .child(div().text_sm().child(icon))
            .child(div().text_xs().text_color(text).child(label))
    }

    /// Renders the quicklinks list.
    fn render_quicklinks_list(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let filtered = self.filtered_quicklinks();
        let selected_index = self.selected_index;
        let confirm_delete = self.confirm_delete.clone();

        if filtered.is_empty() {
            return div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .child(div().text_3xl().child("🔗"))
                .child(div().text_sm().text_color(colors.text_muted).child(
                    if self.search_query.is_empty() {
                        "No quick links yet"
                    } else {
                        "No matching quick links"
                    },
                ))
                .child(div().text_xs().text_color(colors.text_placeholder).child(
                    if self.search_query.is_empty() {
                        "Create a new link or browse the library"
                    } else {
                        "Try a different search term"
                    },
                ))
                .into_any_element();
        }

        let items: Vec<_> = filtered
            .iter()
            .enumerate()
            .map(|(index, link)| {
                let is_selected = selected_index == Some(index);
                let is_deleting = confirm_delete.as_ref() == Some(&link.id);
                Self::render_quicklink_item(link, index, is_selected, is_deleting, &colors, cx)
            })
            .collect();

        div()
            .id("quicklinks-list")
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&self.quicklinks_scroll_handle)
            .px(px(8.0))
            .py(px(4.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(items)
            .into_any_element()
    }

    /// Renders a single quicklink item.
    #[allow(clippy::too_many_lines)]
    fn render_quicklink_item(
        link: &QuickLink,
        index: usize,
        is_selected: bool,
        is_deleting: bool,
        colors: &ManageColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let bg = if is_deleting {
            color_with_alpha(colors.error, 0.15)
        } else if is_selected {
            colors.selection
        } else {
            gpui::hsla(0.0, 0.0, 0.0, 0.0)
        };

        let icon = Self::render_icon(&link.icon, colors);
        let name = link.name.clone();
        let alias = link.alias.clone();
        let link_preview = truncate_url(&link.link, 50);
        let link_id = link.id.clone();
        let link_clone = link.clone();

        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let accent = colors.accent;
        let error_color = colors.error;

        div()
            .id(("quicklink-item", index))
            .w_full()
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .hover(move |s| s.bg(surface_hover))
            .cursor_pointer()
            .group("quicklink-row")
            .on_click(cx.listener(move |this, _, cx| {
                this.selected_index = Some(index);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    // Icon
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(32.0))
                            .rounded(px(6.0))
                            .bg(colors.surface)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon),
                    )
                    // Name and link preview
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(text_color)
                                            .truncate()
                                            .child(name),
                                    )
                                    .when(alias.is_some(), |el| {
                                        el.child(
                                            div()
                                                .px(px(6.0))
                                                .py(px(2.0))
                                                .rounded(px(4.0))
                                                .bg(color_with_alpha(accent, 0.2))
                                                .text_xs()
                                                .text_color(accent)
                                                .child(alias.unwrap_or_default()),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .truncate()
                                    .child(link_preview),
                            ),
                    )
                    // Action buttons (visible on hover or when selected)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .flex_shrink_0()
                            .when(!is_selected && !is_deleting, |el| {
                                el.invisible().group_hover("quicklink-row", gpui::Styled::visible)
                            })
                            .when(is_deleting, |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(error_color)
                                        .mr(px(8.0))
                                        .child("Press Delete to confirm"),
                                )
                            })
                            .when(!is_deleting, |el| {
                                el
                                    // Edit button
                                    .child(
                                        div()
                                            .id(("edit", index))
                                            .p(px(4.0))
                                            .rounded(px(4.0))
                                            .hover(|s| s.bg(surface_hover))
                                            .cursor_pointer()
                                            .on_click(cx.listener(move |this, _, cx| {
                                                this.selected_index = Some(index);
                                                this.edit_selected(cx);
                                            }))
                                            .child(div().text_sm().child("✏️")),
                                    )
                                    // Duplicate button
                                    .child(
                                        div()
                                            .id(("duplicate", index))
                                            .p(px(4.0))
                                            .rounded(px(4.0))
                                            .hover(|s| s.bg(surface_hover))
                                            .cursor_pointer()
                                            .on_click({
                                                let link_for_dup = link_clone.clone();
                                                cx.listener(move |this, _, cx| {
                                                    this.duplicate_quicklink(&link_for_dup, cx);
                                                })
                                            })
                                            .child(div().text_sm().child("📋")),
                                    )
                                    // Delete button
                                    .child(
                                        div()
                                            .id(("delete", index))
                                            .p(px(4.0))
                                            .rounded(px(4.0))
                                            .hover(move |s| s.bg(color_with_alpha(error_color, 0.2)))
                                            .cursor_pointer()
                                            .on_click({
                                                let link_id_for_del = link_id.clone();
                                                cx.listener(move |this, _, cx| {
                                                    this.selected_index = Some(index);
                                                    this.confirm_delete = Some(link_id_for_del.clone());
                                                    cx.notify();
                                                })
                                            })
                                            .child(div().text_sm().child("🗑️")),
                                    )
                            }),
                    ),
            )
    }

    /// Renders an icon based on the QuickLinkIcon type.
    fn render_icon(icon: &QuickLinkIcon, _colors: &ManageColors) -> impl IntoElement {
        match icon {
            QuickLinkIcon::Emoji(emoji) => div().text_lg().child(emoji.clone()).into_any_element(),
            QuickLinkIcon::Default => div().text_lg().child("🌐").into_any_element(),
            QuickLinkIcon::SystemIcon(name) => {
                // Use emoji fallback for system icons
                let emoji = match name.as_str() {
                    "globe" => "🌐",
                    "magnifyingglass" => "🔍",
                    "doc" => "📄",
                    "folder" => "📁",
                    "star" => "⭐",
                    _ => "🔗",
                };
                div().text_lg().child(emoji).into_any_element()
            },
            QuickLinkIcon::Favicon(_path) | QuickLinkIcon::CustomImage(_path) => {
                // TODO: Load image from path
                div().text_lg().child("🌐").into_any_element()
            },
        }
    }

    /// Renders the library browser.
    fn render_library_browser(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let categories = get_categories();
        let selected_category = self.selected_library_category.clone();
        let library_selected_index = self.library_selected_index;

        div()
            .flex_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Category tabs
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .border_b_1()
                    .border_color(colors.border)
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id("all-categories")
                            .px(px(10.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(if selected_category.is_none() {
                                colors.selection
                            } else {
                                colors.surface
                            })
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, cx| {
                                this.set_library_category(None, cx);
                            }))
                            .child(div().text_xs().text_color(colors.text).child("All")),
                    )
                    .children(categories.iter().map(|&cat| {
                        let is_selected = selected_category.as_deref() == Some(cat);
                        let cat_string = cat.to_string();
                        div()
                            .id(SharedString::from(format!("cat-{}", cat)))
                            .px(px(10.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(if is_selected {
                                colors.selection
                            } else {
                                colors.surface
                            })
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| {
                                this.set_library_category(Some(cat_string.clone()), cx);
                            }))
                            .child(div().text_xs().text_color(colors.text).child(cat))
                    })),
            )
            // Library items
            .child(self.render_library_items(library_selected_index, &colors, cx))
    }

    /// Renders library items grouped by category.
    fn render_library_items(
        &self,
        selected_index: Option<usize>,
        colors: &ManageColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let filtered = self.filtered_library_quicklinks();

        if filtered.is_empty() {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.text_muted)
                        .child("No quicklinks in this category"),
                )
                .into_any_element();
        }

        // Group by category if showing all
        let show_headers = self.selected_library_category.is_none();
        let mut elements: Vec<gpui::AnyElement> = Vec::new();
        let mut current_category = "";
        for (index, bundled) in filtered.into_iter().enumerate() {
            if show_headers && bundled.category != current_category {
                current_category = bundled.category;
                elements.push(
                    div()
                        .px(px(12.0))
                        .pt(px(12.0))
                        .pb(px(4.0))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(colors.text_muted)
                                .child(current_category),
                        )
                        .into_any_element(),
                );
            }

            let is_selected = selected_index == Some(index);
            let is_added = self.is_bundled_added(bundled);
            elements.push(
                Self::render_library_item(bundled, index, is_selected, is_added, colors, cx)
                    .into_any_element(),
            );
        }

        div()
            .id("library-list")
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&self.library_scroll_handle)
            .px(px(8.0))
            .py(px(4.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(elements)
            .into_any_element()
    }

    /// Renders a single library item.
    #[allow(clippy::too_many_lines)]
    fn render_library_item(
        bundled: &'static BundledQuickLink,
        index: usize,
        is_selected: bool,
        is_added: bool,
        colors: &ManageColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let bg = if is_selected {
            colors.selection
        } else {
            gpui::hsla(0.0, 0.0, 0.0, 0.0)
        };

        let name = bundled.name;
        let alias = bundled.alias;
        let link_preview = truncate_url(bundled.link, 50);
        let icon = bundled.icon;

        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let accent = colors.accent;
        let success = colors.success;
        let surface = colors.surface;

        div()
            .id(("library-item", index))
            .w_full()
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .hover(move |s| s.bg(surface_hover))
            .cursor_pointer()
            .group("library-row")
            .on_click(cx.listener(move |this, _, cx| {
                this.library_selected_index = Some(index);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    // Icon
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(32.0))
                            .rounded(px(6.0))
                            .bg(surface)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(div().text_lg().child(icon)),
                    )
                    // Name and link preview
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(text_color)
                                            .truncate()
                                            .child(name),
                                    )
                                    .when(alias.is_some(), |el| {
                                        el.child(
                                            div()
                                                .px(px(6.0))
                                                .py(px(2.0))
                                                .rounded(px(4.0))
                                                .bg(color_with_alpha(accent, 0.2))
                                                .text_xs()
                                                .text_color(accent)
                                                .child(alias.unwrap_or("")),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .truncate()
                                    .child(link_preview),
                            ),
                    )
                    // Add button or "Already added" badge
                    .child(
                        div()
                            .flex_shrink_0()
                            .when(is_added, |el| {
                                el.child(
                                    div()
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(color_with_alpha(success, 0.2))
                                        .text_xs()
                                        .text_color(success)
                                        .child("✓ Added"),
                                )
                            })
                            .when(!is_added, |el| {
                                el.child(
                                    div()
                                        .id(("add-library", index))
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(accent)
                                        .hover(|s| s.opacity(0.9))
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, cx| {
                                            this.add_from_library(bundled, cx);
                                        }))
                                        .child(div().text_xs().text_color(text_color).child("Add")),
                                )
                            }),
                    ),
            )
    }

    /// Renders the action bar at the bottom.
    #[allow(clippy::unused_self)]
    fn render_action_bar(&self, colors: &ManageColors) -> impl IntoElement {
        let surface = colors.surface;
        let text_muted = colors.text_muted;
        let border = colors.border;

        div()
            .border_t_1()
            .border_color(border)
            .px(px(16.0))
            .py(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .text_xs()
                    .text_color(text_muted)
                    .child(render_shortcut("↑↓", "Navigate", surface, text_muted))
                    .child(render_shortcut("⏎", "Edit", surface, text_muted))
                    .child(render_shortcut("⌘N", "New", surface, text_muted))
                    .child(render_shortcut("⌘⌫", "Delete", surface, text_muted))
                    .child(render_shortcut("/", "Search", surface, text_muted)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(text_muted)
                    .child("⌘L Browse Library"),
            )
    }

    /// Renders the delete confirmation dialog.
    #[allow(clippy::unused_self)]
    fn render_delete_confirmation(
        &self,
        colors: &ManageColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let surface = colors.surface_elevated;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let error_color = colors.error;
        let surface_hover = colors.surface_hover;

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x0000_0088))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .bg(surface)
                    .rounded(px(8.0))
                    .p(px(24.0))
                    .w(px(360.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(text_color)
                            .child("Delete Quick Link?"),
                    )
                    .child(div().text_sm().text_color(text_muted).child(
                        "This action cannot be undone. The quick link will be permanently removed.",
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .justify_end()
                            .mt(px(8.0))
                            .child(
                                div()
                                    .id("cancel-delete")
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(surface_hover)
                                    .hover(|s| s.opacity(0.9))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.cancel_delete(cx);
                                    }))
                                    .child(div().text_sm().text_color(text_color).child("Cancel")),
                            )
                            .child(
                                div()
                                    .id("confirm-delete")
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(error_color)
                                    .hover(|s| s.opacity(0.9))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.delete_selected(cx);
                                    }))
                                    .child(
                                        div().text_sm().text_color(gpui::white()).child("Delete"),
                                    ),
                            ),
                    ),
            )
    }
}

impl FocusableView for QuicklinksManageView {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for QuicklinksManageView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let show_library = self.show_library;
        let show_delete_confirmation = self.confirm_delete.is_some();

        div()
            .track_focus(&self.focus_handle)
            .key_context("QuickLinksManage")
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_action(cx.listener(|this, _: &SelectNext, cx| {
                this.select_next(cx);
            }))
            .on_action(cx.listener(|this, _: &SelectPrevious, cx| {
                this.select_previous(cx);
            }))
            .on_action(cx.listener(|this, _: &EditSelected, cx| {
                this.edit_selected(cx);
            }))
            .on_action(cx.listener(|this, _: &DeleteSelected, cx| {
                this.delete_selected(cx);
            }))
            .on_action(cx.listener(|this, _: &CreateNew, cx| {
                this.create_new(cx);
            }))
            .on_action(cx.listener(|this, _: &CloseView, cx| {
                this.close(cx);
            }))
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(colors.background)
            // Header
            .child(self.render_header(&colors))
            // Search bar
            .child(self.render_search_bar(cx))
            // Toolbar
            .child(self.render_toolbar(cx))
            // Content area (list or library browser)
            .child(if show_library {
                self.render_library_browser(cx).into_any_element()
            } else {
                self.render_quicklinks_list(cx).into_any_element()
            })
            // Action bar
            .child(self.render_action_bar(&colors))
            // Delete confirmation dialog
            .when(show_delete_confirmation, |el| {
                el.child(self.render_delete_confirmation(&colors, cx))
            })
            // Inline edit modal
            .when(self.editing.is_some(), |el| {
                el.child(self.render_edit_modal(&colors, cx))
            })
    }
}

// ============================================================================
// Edit Modal Rendering (separate impl block)
// ============================================================================

impl QuicklinksManageView {
    /// Renders the inline edit modal.
    #[allow(clippy::too_many_lines)]
    fn render_edit_modal(
        &self,
        colors: &ManageColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let Some(editing) = &self.editing else {
            return div().into_any_element();
        };

        let surface = colors.surface_elevated;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let border = colors.border;
        let accent = colors.accent;
        let surface_hover = colors.surface_hover;

        let is_new = editing.is_new;
        let name = editing.name.clone();
        let link = editing.link.clone();
        let alias = editing.alias.clone();
        let focused = editing.focused_field;

        // Get cursor positions
        let name_cursor = editing.name_cursor;
        let link_cursor = editing.link_cursor;
        let alias_cursor = editing.alias_cursor;

        // Block cursor dimensions (matches launcher)
        let cursor_width = px(9.0);
        let cursor_height = px(20.0);

        // Helper to render a text field with block cursor
        let render_field = |field: EditField,
                            label: &'static str,
                            value: String,
                            cursor_pos: usize,
                            placeholder: &'static str| {
            let is_focused = focused == field;
            let is_empty = value.is_empty();

            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(text_muted)
                        .child(label),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("edit-field-{:?}", field)))
                        .px(px(12.0))
                        .py(px(10.0))
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(if is_focused { accent } else { border })
                        .bg(colors.surface)
                        .when(is_focused, gpui::Styled::border_2)
                        .cursor_text()
                        .on_click(cx.listener(move |this, _, cx| {
                            if let Some(ref mut editing) = this.editing {
                                editing.focused_field = field;
                                cx.notify();
                            }
                        }))
                        .child(
                            // Text with block cursor
                            div()
                                .text_sm()
                                .flex()
                                .items_center()
                                .when(is_empty && is_focused, |el| {
                                    // Empty field with focus: show cursor then placeholder
                                    el.child(
                                        div()
                                            .w(cursor_width)
                                            .h(cursor_height)
                                            .bg(accent)
                                            .rounded(px(2.0)),
                                    )
                                    .child(
                                        div().text_color(text_muted).child(placeholder.to_string()),
                                    )
                                })
                                .when(is_empty && !is_focused, |el| {
                                    // Empty field without focus: show placeholder
                                    el.child(
                                        div().text_color(text_muted).child(placeholder.to_string()),
                                    )
                                })
                                .when(!is_empty, |el| {
                                    // Non-empty: show text with cursor at position
                                    let chars: Vec<char> = value.chars().collect();
                                    let clamped_cursor = cursor_pos.min(chars.len());
                                    let before: String = chars[..clamped_cursor].iter().collect();
                                    let after: String = chars[clamped_cursor..].iter().collect();

                                    el.text_color(text_color)
                                        .when(!before.is_empty(), |el| el.child(before.clone()))
                                        .when(is_focused, |el| {
                                            el.child(
                                                div()
                                                    .w(cursor_width)
                                                    .h(cursor_height)
                                                    .bg(accent)
                                                    .rounded(px(2.0)),
                                            )
                                        })
                                        .when(!after.is_empty(), |el| el.child(after.clone()))
                                }),
                        ),
                )
        };

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x0000_0088))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .bg(surface)
                    .rounded(px(12.0))
                    .p(px(24.0))
                    .w(px(420.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    // Title
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(text_color)
                            .child(if is_new { "New Quick Link" } else { "Edit Quick Link" }),
                    )
                    // Name field
                    .child(render_field(EditField::Name, "Name", name, name_cursor, "Enter name..."))
                    // URL field
                    .child(render_field(EditField::Link, "URL", link, link_cursor, "https://example.com/search?q={argument}"))
                    // Alias field
                    .child(render_field(EditField::Alias, "Alias (optional)", alias, alias_cursor, "e.g., g"))
                    // Hint text
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child("Tab to switch fields. Enter to save, Esc to cancel."),
                    )
                    // Buttons
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .justify_end()
                            .mt(px(8.0))
                            .child(
                                div()
                                    .id("cancel-edit")
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(surface_hover)
                                    .hover(|s| s.opacity(0.9))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.cancel_editing(cx);
                                    }))
                                    .child(div().text_sm().text_color(text_color).child("Cancel")),
                            )
                            .child(
                                div()
                                    .id("save-edit")
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(accent)
                                    .hover(|s| s.opacity(0.9))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        if let Some(editing) = this.editing.take() {
                                            this.finish_editing(editing, cx);
                                        }
                                    }))
                                    .child(div().text_sm().text_color(gpui::white()).child("Save")),
                            ),
                    ),
            )
            .into_any_element()
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Truncates a URL for display.
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len])
    }
}

/// Renders a keyboard shortcut hint.
fn render_shortcut(
    key: &'static str,
    label: &'static str,
    bg: Hsla,
    text: Hsla,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(
            div()
                .px(px(4.0))
                .py(px(2.0))
                .rounded(px(3.0))
                .bg(bg)
                .text_xs()
                .font_family("monospace")
                .text_color(text)
                .child(key),
        )
        .child(div().text_xs().text_color(text).child(label))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_truncate_url() {
        assert_eq!(
            truncate_url("https://example.com", 50),
            "https://example.com"
        );
        assert_eq!(
            truncate_url(
                "https://very-long-url-that-should-be-truncated.example.com/path/to/resource",
                30
            ),
            "https://very-long-url-that-sho..."
        );
    }

    #[test]
    fn test_manage_view_event_variants() {
        // Just verify the enum variants compile
        let _ = ManageViewEvent::ShowCreateView;
        let link = QuickLink::new("Test", "https://example.com");
        let _ = ManageViewEvent::ShowEditView(link.clone());
        let _ = ManageViewEvent::DeleteQuicklink(link.id.clone());
        let _ = ManageViewEvent::DuplicateQuicklink(link);
        let _ = ManageViewEvent::Close;
    }

    #[test]
    fn test_on_change_callback_mechanism() {
        // Test that the callback type signature is correct
        // by creating a callback closure
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        // Create a callback that matches the expected signature: Fn() + Send + 'static
        let callback: Box<dyn Fn() + Send + 'static> = Box::new(move || {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Verify the callback works
        callback();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        callback();
        callback();
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_on_change_callback_optional_none() {
        // Test that Option<Box<dyn Fn() + Send + 'static>> handles None correctly
        let callback: Option<Box<dyn Fn() + Send + 'static>> = None;

        // Verify if-let pattern works correctly (matches notify_change implementation)
        if let Some(cb) = &callback {
            cb();
            panic!("Should not reach here");
        }
        // No panic means test passes
    }
}
