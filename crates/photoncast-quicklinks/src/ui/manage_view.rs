//! Quick Links management view for preferences/settings.
//!
//! Provides a comprehensive UI for managing user quick links, including:
//! - List view of all quick links with search/filter
//! - Library browser for bundled quick links
//! - Create, edit, delete, and duplicate actions
//! - Keyboard navigation

use gpui::prelude::*;
use gpui::{
    actions, div, px, rgba, AppContext, EventEmitter, FocusHandle, FocusableView, FontWeight,
    Hsla, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, ViewContext,
};
use photoncast_theme::PhotonTheme;

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

/// Theme-aware colors for the manage view.
#[derive(Clone)]
struct ManageColors {
    background: Hsla,
    surface: Hsla,
    surface_hover: Hsla,
    surface_elevated: Hsla,
    text: Hsla,
    text_muted: Hsla,
    text_placeholder: Hsla,
    border: Hsla,
    accent: Hsla,
    selection: Hsla,
    hover: Hsla,
    success: Hsla,
    warning: Hsla,
    error: Hsla,
}

impl ManageColors {
    fn from_theme(theme: &PhotonTheme) -> Self {
        Self {
            background: theme.colors.background.to_gpui(),
            surface: theme.colors.surface.to_gpui(),
            surface_hover: theme.colors.surface_hover.to_gpui(),
            surface_elevated: theme.colors.background_elevated.to_gpui(),
            text: theme.colors.text.to_gpui(),
            text_muted: theme.colors.text_muted.to_gpui(),
            text_placeholder: theme.colors.text_placeholder.to_gpui(),
            border: theme.colors.border.to_gpui(),
            accent: theme.colors.accent.to_gpui(),
            selection: theme.colors.selection.to_gpui(),
            hover: theme.colors.hover.to_gpui(),
            success: theme.colors.success.to_gpui(),
            warning: theme.colors.warning.to_gpui(),
            error: theme.colors.error.to_gpui(),
        }
    }
}

fn get_colors(cx: &ViewContext<QuicklinksManageView>) -> ManageColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    ManageColors::from_theme(&theme)
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
            confirm_delete: None,
            focus_handle,
            selected_library_category: None,
            library_selected_index: None,
            storage: None,
            runtime: None,
            editing: None,
            library_scroll_handle: gpui::ScrollHandle::new(),
            quicklinks_scroll_handle: gpui::ScrollHandle::new(),
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
        let mut quicklink = crate::library::to_quicklink(bundled);
        
        // Save to storage and update the ID with the DB-generated one
        if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
            let storage = storage.clone();
            match runtime.block_on(storage.store(&quicklink)) {
                Ok(new_id) => {
                    // Update the quicklink with the DB-generated integer ID
                    quicklink.id = new_id;
                }
                Err(e) => {
                    tracing::error!("Failed to save quicklink: {}", e);
                    return;
                }
            }
        }
        
        // Add to local list with correct DB ID
        self.quicklinks.push(quicklink);
        cx.notify();
    }

    /// Deletes a quicklink by ID.
    pub fn delete_quicklink(&mut self, id: &QuickLinkId, cx: &mut ViewContext<Self>) {
        // Delete from storage if available
        if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
            let storage = storage.clone();
            let id = id.clone();
            if let Err(e) = runtime.block_on(storage.delete(&id)) {
                tracing::error!("Failed to delete quicklink: {}", e);
                return;
            }
        }
        
        // Remove from local list
        self.quicklinks.retain(|link| link.id != *id);
        
        // Adjust selection
        let filtered_len = self.filtered_quicklinks().len();
        if filtered_len == 0 {
            self.selected_index = None;
        } else if let Some(idx) = self.selected_index {
            if idx >= filtered_len {
                self.selected_index = Some(filtered_len - 1);
            }
        }
        
        cx.notify();
    }

    /// Duplicates a quicklink.
    pub fn duplicate_quicklink(&mut self, link: &QuickLink, cx: &mut ViewContext<Self>) {
        let mut copy = link.clone();
        copy.name = format!("{} (Copy)", link.name);
        
        // Save to storage and update the ID with the DB-generated one
        if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
            let storage = storage.clone();
            match runtime.block_on(storage.store(&copy)) {
                Ok(new_id) => {
                    // Update with DB-generated integer ID
                    copy.id = new_id;
                }
                Err(e) => {
                    tracing::error!("Failed to save duplicated quicklink: {}", e);
                    return;
                }
            }
        }
        
        // Add to local list with correct DB ID
        self.quicklinks.push(copy);
        cx.notify();
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
                        || link.keywords.iter().any(|k| k.to_lowercase().contains(&query))
                        || link.tags.iter().any(|t| t.to_lowercase().contains(&query))
                })
                .collect()
        }
    }

    /// Returns the currently selected quicklink.
    pub fn selected_quicklink(&self) -> Option<&QuickLink> {
        let filtered = self.filtered_quicklinks();
        self.selected_index.and_then(|idx| filtered.get(idx).copied())
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
        // Item: py(8)*2=16 + ~34px content + 2px gap = ~52px
        const ITEM_HEIGHT: f32 = 52.0;
        // Window is 550px, header/toolbar ~130px, so list area ~420px
        const VISIBLE_HEIGHT: f32 = 420.0;
        
        let item_top = index as f32 * ITEM_HEIGHT;
        let item_bottom = item_top + ITEM_HEIGHT;
        
        let current_offset = self.library_scroll_handle.offset();
        let scroll_top = -current_offset.y.0;
        let scroll_bottom = scroll_top + VISIBLE_HEIGHT;
        
        if item_top < scroll_top {
            self.library_scroll_handle
                .set_offset(gpui::Point::new(px(0.0), px(-item_top)));
        } else if item_bottom > scroll_bottom {
            let new_scroll_top = item_bottom - VISIBLE_HEIGHT;
            self.library_scroll_handle
                .set_offset(gpui::Point::new(px(0.0), px(-new_scroll_top)));
        }
    }
    
    /// Scrolls to make the selected quicklinks item visible.
    fn scroll_to_quicklinks_item(&self, index: usize) {
        const ITEM_HEIGHT: f32 = 52.0;
        const VISIBLE_HEIGHT: f32 = 420.0;
        
        let item_top = index as f32 * ITEM_HEIGHT;
        let item_bottom = item_top + ITEM_HEIGHT;
        
        let current_offset = self.quicklinks_scroll_handle.offset();
        let scroll_top = -current_offset.y.0;
        let scroll_bottom = scroll_top + VISIBLE_HEIGHT;
        
        if item_top < scroll_top {
            self.quicklinks_scroll_handle
                .set_offset(gpui::Point::new(px(0.0), px(-item_top)));
        } else if item_bottom > scroll_bottom {
            let new_scroll_top = item_bottom - VISIBLE_HEIGHT;
            self.quicklinks_scroll_handle
                .set_offset(gpui::Point::new(px(0.0), px(-new_scroll_top)));
        }
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
            self.editing = Some(EditingState {
                original_id: link.id.clone(),
                name: link.name.clone(),
                link: link.link.clone(),
                alias: link.alias.clone().unwrap_or_default(),
                is_new: false,
                focused_field: EditField::Name,
            });
            cx.notify();
        }
    }
    
    /// Starts creating a new quicklink inline.
    fn start_new_quicklink(&mut self, cx: &mut ViewContext<Self>) {
        self.editing = Some(EditingState {
            original_id: QuickLinkId::generate(),
            name: String::new(),
            link: String::new(),
            alias: String::new(),
            is_new: true,
            focused_field: EditField::Name,
        });
        cx.notify();
    }
    
    /// Cancels the current inline edit.
    fn cancel_editing(&mut self, cx: &mut ViewContext<Self>) {
        self.editing = None;
        cx.notify();
    }
    
    /// Finishes editing and saves changes.
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
            
            // Save to storage
            if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
                let storage = storage.clone();
                match runtime.block_on(storage.store(&quicklink)) {
                    Ok(new_id) => {
                        quicklink.id = new_id;
                    }
                    Err(e) => {
                        tracing::error!("Failed to save quicklink: {}", e);
                        return;
                    }
                }
            }
            
            self.quicklinks.push(quicklink);
        } else {
            // Update existing quicklink
            if let Some(link) = self.quicklinks.iter_mut().find(|l| l.id == editing.original_id) {
                link.name = editing.name.trim().to_string();
                link.link = editing.link.trim().to_string();
                link.alias = if editing.alias.trim().is_empty() {
                    None
                } else {
                    Some(editing.alias.trim().to_string())
                };
                
                // Save to storage
                if let (Some(storage), Some(runtime)) = (&self.storage, &self.runtime) {
                    let storage = storage.clone();
                    let link_clone = link.clone();
                    if let Err(e) = runtime.block_on(storage.update(&link_clone)) {
                        tracing::error!("Failed to update quicklink: {}", e);
                    }
                }
            }
        }
        
        cx.notify();
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
    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        // Handle editing mode keys first
        if let Some(ref mut editing) = self.editing {
            match event.keystroke.key.as_str() {
                "escape" => {
                    self.editing = None;
                    cx.notify();
                    return;
                }
                "enter" | "s" if event.keystroke.modifiers.platform || event.keystroke.key == "enter" => {
                    // Save on Cmd+S or Enter
                    if event.keystroke.modifiers.platform || event.keystroke.key == "enter" {
                        // Clone editing state before taking it
                        let editing_clone = editing.clone();
                        self.editing = None;
                        self.finish_editing(editing_clone, cx);
                        return;
                    }
                }
                "tab" => {
                    // Cycle through fields
                    editing.focused_field = match editing.focused_field {
                        EditField::Name => EditField::Link,
                        EditField::Link => EditField::Alias,
                        EditField::Alias => EditField::Name,
                    };
                    cx.notify();
                    return;
                }
                "backspace" => {
                    // Delete last character from focused field
                    let field = match editing.focused_field {
                        EditField::Name => &mut editing.name,
                        EditField::Link => &mut editing.link,
                        EditField::Alias => &mut editing.alias,
                    };
                    field.pop();
                    cx.notify();
                    return;
                }
                _ => {
                    // Handle typing into focused field
                    if !event.keystroke.modifiers.platform
                        && !event.keystroke.modifiers.control
                        && !event.keystroke.modifiers.alt
                    {
                        let char_to_add = event.keystroke.ime_key.as_deref()
                            .or({
                                if event.keystroke.key.len() == 1 {
                                    Some(event.keystroke.key.as_str())
                                } else {
                                    None
                                }
                            });
                        
                        if let Some(ch) = char_to_add {
                            let text = if event.keystroke.modifiers.shift {
                                ch.to_uppercase()
                            } else {
                                ch.to_string()
                            };
                            
                            let field = match editing.focused_field {
                                EditField::Name => &mut editing.name,
                                EditField::Link => &mut editing.link,
                                EditField::Alias => &mut editing.alias,
                            };
                            field.push_str(&text);
                            cx.notify();
                        }
                        return;
                    }
                    return;
                }
            }
        }
        
        match event.keystroke.key.as_str() {
            "down" => {
                self.select_next(cx);
            }
            "up" => {
                self.select_previous(cx);
            }
            "enter" => {
                // If delete confirmation is showing, confirm delete
                if self.confirm_delete.is_some() {
                    self.delete_selected(cx);
                } else {
                    self.edit_selected(cx);
                }
            }
            "backspace" | "delete" => {
                if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
                    self.delete_selected(cx);
                } else if !self.search_query.is_empty() {
                    // Remove last character from search
                    let mut chars: Vec<char> = self.search_query.chars().collect();
                    chars.pop();
                    self.set_search(chars.into_iter().collect(), cx);
                }
            }
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
            }
            "n" if event.keystroke.modifiers.platform => {
                self.create_new(cx);
            }
            "/" => {
                // Focus search - clear and start typing
                self.set_search(String::new(), cx);
            }
            "l" if event.keystroke.modifiers.platform => {
                self.toggle_library(cx);
            }
            "d" if event.keystroke.modifiers.platform => {
                self.duplicate_selected(cx);
            }
            _ => {
                // Handle typing into search
                if !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                {
                    if let Some(ime_key) = &event.keystroke.ime_key {
                        let new_query = format!("{}{}", self.search_query, ime_key);
                        self.set_search(new_query, cx);
                    } else if event.keystroke.key.len() == 1 {
                        let key = if event.keystroke.modifiers.shift {
                            event.keystroke.key.to_uppercase()
                        } else {
                            event.keystroke.key.clone()
                        };
                        let new_query = format!("{}{}", self.search_query, key);
                        self.set_search(new_query, cx);
                    }
                }
            }
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

        div()
            .px(px(16.0))
            .py(px(8.0))
            .child(
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
                            .text_color(if has_query {
                                colors.text
                            } else {
                                colors.text_placeholder
                            })
                            .child(if has_query {
                                query
                            } else {
                                "Search quick links...".to_string()
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
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.text_muted)
                        .child(if self.search_query.is_empty() {
                            "No quick links yet"
                        } else {
                            "No matching quick links"
                        }),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(colors.text_placeholder)
                        .child(if self.search_query.is_empty() {
                            "Create a new link or browse the library"
                        } else {
                            "Try a different search term"
                        }),
                )
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
            QuickLinkIcon::Emoji(emoji) => {
                div().text_lg().child(emoji.clone()).into_any_element()
            }
            QuickLinkIcon::Default => div().text_lg().child("🌐").into_any_element(),
            QuickLinkIcon::SystemIcon(name) => {
                // Use emoji fallback for system icons
                let emoji = match name.as_str() {
                    "globe" => "🌐",
                    "link" => "🔗",
                    "magnifyingglass" => "🔍",
                    "doc" => "📄",
                    "folder" => "📁",
                    "star" => "⭐",
                    _ => "🔗",
                };
                div().text_lg().child(emoji).into_any_element()
            }
            QuickLinkIcon::Favicon(_path) | QuickLinkIcon::CustomImage(_path) => {
                // TODO: Load image from path
                div().text_lg().child("🌐").into_any_element()
            }
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
        let mut index = 0;

        for bundled in filtered {
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
            index += 1;
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
    fn render_delete_confirmation(&self, colors: &ManageColors, cx: &mut ViewContext<Self>) -> impl IntoElement {
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
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_muted)
                            .child("This action cannot be undone. The quick link will be permanently removed."),
                    )
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
                                    .child(div().text_sm().text_color(gpui::white()).child("Delete")),
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
    fn render_edit_modal(&self, colors: &ManageColors, cx: &mut ViewContext<Self>) -> impl IntoElement {
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
        
        // Helper to render a text field with focus indication
        let render_field = |field: EditField, label: &'static str, value: String, placeholder: &'static str| {
            let is_focused = focused == field;
            let is_empty = value.is_empty();
            let display_value = if is_empty {
                placeholder.to_string()
            } else if is_focused {
                format!("{}|", value) // Show cursor
            } else {
                value
            };
            
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
                            div()
                                .text_sm()
                                .text_color(if is_empty { text_muted } else { text_color })
                                .child(display_value),
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
                    .child(render_field(EditField::Name, "Name", name, "Enter name..."))
                    // URL field  
                    .child(render_field(EditField::Link, "URL", link, "https://example.com/search?q={argument}"))
                    // Alias field
                    .child(render_field(EditField::Alias, "Alias (optional)", alias, "e.g., g"))
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
fn render_shortcut(key: &'static str, label: &'static str, bg: Hsla, text: Hsla) -> impl IntoElement {
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

    #[test]
    fn test_truncate_url() {
        assert_eq!(truncate_url("https://example.com", 50), "https://example.com");
        assert_eq!(
            truncate_url("https://very-long-url-that-should-be-truncated.example.com/path/to/resource", 30),
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
}
