//! ListView rendering for extensions.
//!
//! Renders `ListView` types with:
//! - Sections with optional titles
//! - Items with icon, title, subtitle, accessories
//! - Search bar with throttled filtering
//! - Empty state with icon, title, description, actions
//! - Keyboard navigation (↑↓, Enter, ⌘1-9)
//! - Optional split-view with preview

use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::*;
use abi_stable::std_types::RVec;
use photoncast_extension_api::{
    Accessory, Action, EmptyState, IconSource, ListItem, ListSection, ListView,
    ROption,
};

use super::actions::{execute_and_maybe_close, CLOSE_VIEW_ACTION};
use super::colors::ExtensionViewColors;
use super::dimensions::*;
use super::preview::ExtensionPreviewPane;
use super::ActionCallback;

// ============================================================================
// Actions
// ============================================================================

actions!(
    extension_list,
    [
        SelectNext,
        SelectPrevious,
        Activate,
        Cancel,
        ShowActionsMenu,
        QuickSelect1,
        QuickSelect2,
        QuickSelect3,
        QuickSelect4,
        QuickSelect5,
        QuickSelect6,
        QuickSelect7,
        QuickSelect8,
        QuickSelect9,
    ]
);

/// Registers key bindings for the extension list view.
pub fn register_key_bindings(cx: &mut gpui::AppContext) {
    cx.bind_keys([
        KeyBinding::new("down", SelectNext, Some("ExtensionListView")),
        KeyBinding::new("up", SelectPrevious, Some("ExtensionListView")),
        KeyBinding::new("enter", Activate, Some("ExtensionListView")),
        KeyBinding::new("escape", Cancel, Some("ExtensionListView")),
        KeyBinding::new("cmd-k", ShowActionsMenu, Some("ExtensionListView")),
        KeyBinding::new("cmd-1", QuickSelect1, Some("ExtensionListView")),
        KeyBinding::new("cmd-2", QuickSelect2, Some("ExtensionListView")),
        KeyBinding::new("cmd-3", QuickSelect3, Some("ExtensionListView")),
        KeyBinding::new("cmd-4", QuickSelect4, Some("ExtensionListView")),
        KeyBinding::new("cmd-5", QuickSelect5, Some("ExtensionListView")),
        KeyBinding::new("cmd-6", QuickSelect6, Some("ExtensionListView")),
        KeyBinding::new("cmd-7", QuickSelect7, Some("ExtensionListView")),
        KeyBinding::new("cmd-8", QuickSelect8, Some("ExtensionListView")),
        KeyBinding::new("cmd-9", QuickSelect9, Some("ExtensionListView")),
    ]);
}

// ============================================================================
// View State
// ============================================================================

/// Extension ListView state.
pub struct ExtensionListView {
    /// The list view data from the extension.
    list_view: ListView,
    /// Current search query (if search bar is enabled).
    search_query: String,
    /// Cursor position in the search query.
    cursor_position: usize,
    /// Cursor blink epoch for animation.
    cursor_blink_epoch: Instant,
    /// Currently selected item index (flattened across all sections).
    selected_index: usize,
    /// All items flattened for navigation.
    flat_items: Vec<FlatListItem>,
    /// Filtered item indices (when search is active).
    filtered_indices: Vec<usize>,
    /// Search debounce generation.
    search_generation: u64,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Scroll handle for the results list.
    scroll_handle: ScrollHandle,
    /// Action callback for handling item actions.
    action_callback: Option<ActionCallback>,
    /// Whether the list is in loading state.
    loading: bool,
    /// Error message to display.
    error: Option<String>,
    /// Whether the actions menu (Cmd+K) is visible.
    show_actions_menu: bool,
    /// Selected index in the actions menu.
    actions_menu_index: usize,
}

/// A flattened list item with section context.
#[derive(Clone)]
struct FlatListItem {
    /// The actual list item.
    item: ListItem,
    /// Index of the section this item belongs to.
    section_index: usize,
    /// Index of the item within its section.
    item_index: usize,
    /// Global index in the flattened list.
    global_index: usize,
}

impl ExtensionListView {
    /// Creates a new extension list view.
    pub fn new(
        list_view: ListView,
        action_callback: Option<ActionCallback>,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        let flat_items = Self::flatten_items(&list_view.sections);
        let filtered_indices = (0..flat_items.len()).collect();

        let view = Self {
            list_view,
            search_query: String::new(),
            cursor_position: 0,
            cursor_blink_epoch: Instant::now(),
            selected_index: 0,
            flat_items,
            filtered_indices,
            search_generation: 0,
            focus_handle,
            scroll_handle: ScrollHandle::new(),
            action_callback,
            loading: false,
            error: None,
            show_actions_menu: false,
            actions_menu_index: 0,
        };

        // Start cursor blink timer
        view.start_cursor_blink_timer(cx);

        view
    }

    /// Updates the list items.
    pub fn update_items(&mut self, items: RVec<ListItem>, cx: &mut ViewContext<Self>) {
        // Update all sections with the new items
        for section in self.list_view.sections.iter_mut() {
            section.items = items.clone();
        }

        // Re-flatten and filter
        self.flat_items = Self::flatten_items(&self.list_view.sections);
        self.apply_search_filter();
        self.selected_index = self.selected_index.min(self.filtered_indices.len().saturating_sub(1));

        cx.notify();
    }

    /// Sets the loading state.
    pub fn set_loading(&mut self, loading: bool, cx: &mut ViewContext<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Sets the error message.
    pub fn set_error(&mut self, error: Option<String>, cx: &mut ViewContext<Self>) {
        self.error = error;
        cx.notify();
    }

    /// Flattens sections into a single list for navigation.
    fn flatten_items(sections: &RVec<ListSection>) -> Vec<FlatListItem> {
        let mut flat: Vec<FlatListItem> = Vec::new();
        let mut global_index: usize = 0;

        for (section_index, section) in sections.iter().enumerate() {
            for (item_index, item) in section.items.iter().enumerate() {
                let cloned_item: ListItem = item.clone();
                flat.push(FlatListItem {
                    item: cloned_item,
                    section_index,
                    item_index,
                    global_index,
                });
                global_index += 1;
            }
        }

        flat
    }

    /// Applies the search filter to the items.
    fn apply_search_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.flat_items.len()).collect();
        } else {
            let query_lower = self.search_query.to_lowercase();
            self.filtered_indices = self
                .flat_items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.item.title.to_lowercase().contains(&query_lower)
                        || item
                            .item
                            .subtitle
                            .as_ref()
                            .map_or(false, |s| s.to_lowercase().contains(&query_lower))
                })
                .map(|(i, _)| i)
                .collect();
        }
    }

    /// Returns a reference to a filtered item by display index.
    fn filtered_item(&self, idx: usize) -> Option<&FlatListItem> {
        self.filtered_indices.get(idx).and_then(|&i| self.flat_items.get(i))
    }

    /// Starts the cursor blink timer.
    fn start_cursor_blink_timer(&self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            let blink_interval = Duration::from_millis(530);
            loop {
                cx.background_executor().timer(blink_interval).await;
                let should_continue = this
                    .update(&mut cx, |_this, cx| {
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !should_continue {
                    break;
                }
            }
        })
        .detach();
    }

    /// Checks if the cursor should be visible.
    fn cursor_visible(&self) -> bool {
        const BLINK_INTERVAL_MS: u128 = 530;
        let elapsed = self.cursor_blink_epoch.elapsed().as_millis();
        (elapsed / BLINK_INTERVAL_MS) % 2 == 0
    }

    /// Resets the cursor blink timer.
    fn reset_cursor_blink(&mut self) {
        self.cursor_blink_epoch = Instant::now();
    }

    /// Schedules a debounced search filter.
    fn schedule_search(&mut self, cx: &mut ViewContext<Self>) {
        self.search_generation = self.search_generation.saturating_add(1);
        let generation = self.search_generation;
        let throttle_ms = self
            .list_view
            .search_bar
            .as_ref()
            .map_or(100, |sb| sb.throttle_ms as u64);

        cx.spawn(|this, mut cx| async move {
            cx.background_executor()
                .timer(Duration::from_millis(throttle_ms))
                .await;

            let should_apply = this
                .update(&mut cx, |view, _| view.search_generation == generation)
                .unwrap_or(false);

            if should_apply {
                let _ = this.update(&mut cx, |view, cx| {
                    view.apply_search_filter();
                    view.selected_index = 0;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Ensures the selected item is visible by scrolling if needed.
    fn ensure_selected_visible(&mut self, _cx: &mut ViewContext<Self>) {
        let item_height = ITEM_HEIGHT.0;
        let visible_items = 8;
        let visible_height = visible_items as f32 * item_height;
        let selected_top = self.selected_index as f32 * item_height;
        let selected_bottom = selected_top + item_height;

        let current_offset = self.scroll_handle.offset();
        let current_top = -current_offset.y.0;
        let current_bottom = current_top + visible_height;

        if selected_top < current_top {
            self.scroll_handle
                .set_offset(gpui::Point::new(px(0.0), px(-selected_top)));
        } else if selected_bottom > current_bottom {
            self.scroll_handle.set_offset(gpui::Point::new(
                px(0.0),
                px(-(selected_bottom - visible_height)),
            ));
        }
    }

    /// Gets the currently selected item.
    fn selected_item(&self) -> Option<&FlatListItem> {
        self.filtered_item(self.selected_index)
    }

    /// Activates the selected item.
    fn activate_selected(&mut self, cx: &mut ViewContext<Self>) {
        let item = self.selected_item().map(|i| i.item.clone());
        if let Some(item) = item {
            self.handle_item_activation(&item, cx);
        }
    }

    /// Activates an item at a specific index.
    fn activate_at_index(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        let item = self.filtered_item(index).map(|i| i.item.clone());
        if let Some(item) = item {
            self.selected_index = index;
            self.handle_item_activation(&item, cx);
        }
    }

    /// Handles activation of a list item.
    fn handle_item_activation(&mut self, item: &ListItem, cx: &mut ViewContext<Self>) {
        // Find the primary action (first action or the one with Enter shortcut)
        if let Some(action) = item.actions.first() {
            self.execute_action(action, cx);
        }
    }

    /// Executes an action using the shared action execution logic.
    fn execute_action(&mut self, action: &Action, cx: &mut ViewContext<Self>) {
        // Close actions menu after executing
        self.show_actions_menu = false;
        
        execute_and_maybe_close(action, &self.action_callback, cx);
        
        cx.notify();
    }

    // ========================================================================
    // Action Handlers
    // ========================================================================

    fn select_next(&mut self, _: &SelectNext, cx: &mut ViewContext<Self>) {
        // If actions menu is open, navigate within it
        if self.show_actions_menu {
            if let Some(item) = self.selected_item() {
                let action_count = item.item.actions.len();
                if action_count > 0 {
                    self.actions_menu_index = (self.actions_menu_index + 1) % action_count;
                    cx.notify();
                }
            }
            return;
        }
        
        if !self.filtered_indices.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
            self.ensure_selected_visible(cx);
            cx.notify();
        }
    }

    fn select_previous(&mut self, _: &SelectPrevious, cx: &mut ViewContext<Self>) {
        // If actions menu is open, navigate within it
        if self.show_actions_menu {
            if let Some(item) = self.selected_item() {
                let action_count = item.item.actions.len();
                if action_count > 0 {
                    self.actions_menu_index = if self.actions_menu_index == 0 {
                        action_count - 1
                    } else {
                        self.actions_menu_index - 1
                    };
                    cx.notify();
                }
            }
            return;
        }
        
        if !self.filtered_indices.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_indices.len() - 1
            } else {
                self.selected_index - 1
            };
            self.ensure_selected_visible(cx);
            cx.notify();
        }
    }

    fn activate(&mut self, _: &Activate, cx: &mut ViewContext<Self>) {
        // If actions menu is open, execute the selected action
        if self.show_actions_menu {
            if let Some(item) = self.selected_item().map(|i| i.item.clone()) {
                if let Some(action) = item.actions.get(self.actions_menu_index) {
                    self.execute_action(action, cx);
                }
            }
            return;
        }
        
        self.activate_selected(cx);
    }

    fn cancel(&mut self, _: &Cancel, cx: &mut ViewContext<Self>) {
        // If actions menu is open, close it first
        if self.show_actions_menu {
            self.show_actions_menu = false;
            cx.notify();
            return;
        }
        
        if !self.search_query.is_empty() {
            // Clear search first
            self.search_query.clear();
            self.cursor_position = 0;
            self.apply_search_filter();
            self.selected_index = 0;
            cx.notify();
        } else {
            // Close the view
            if let Some(callback) = &self.action_callback {
                callback(CLOSE_VIEW_ACTION, cx);
            }
        }
    }
    
    fn show_actions_menu(&mut self, _: &ShowActionsMenu, cx: &mut ViewContext<Self>) {
        // Only show if there's a selected item with actions
        if let Some(item) = self.selected_item() {
            if !item.item.actions.is_empty() {
                self.show_actions_menu = !self.show_actions_menu;
                self.actions_menu_index = 0;
                cx.notify();
            }
        }
    }

    fn quick_select_1(&mut self, _: &QuickSelect1, cx: &mut ViewContext<Self>) {
        self.activate_at_index(0, cx);
    }

    fn quick_select_2(&mut self, _: &QuickSelect2, cx: &mut ViewContext<Self>) {
        self.activate_at_index(1, cx);
    }

    fn quick_select_3(&mut self, _: &QuickSelect3, cx: &mut ViewContext<Self>) {
        self.activate_at_index(2, cx);
    }

    fn quick_select_4(&mut self, _: &QuickSelect4, cx: &mut ViewContext<Self>) {
        self.activate_at_index(3, cx);
    }

    fn quick_select_5(&mut self, _: &QuickSelect5, cx: &mut ViewContext<Self>) {
        self.activate_at_index(4, cx);
    }

    fn quick_select_6(&mut self, _: &QuickSelect6, cx: &mut ViewContext<Self>) {
        self.activate_at_index(5, cx);
    }

    fn quick_select_7(&mut self, _: &QuickSelect7, cx: &mut ViewContext<Self>) {
        self.activate_at_index(6, cx);
    }

    fn quick_select_8(&mut self, _: &QuickSelect8, cx: &mut ViewContext<Self>) {
        self.activate_at_index(7, cx);
    }

    fn quick_select_9(&mut self, _: &QuickSelect9, cx: &mut ViewContext<Self>) {
        self.activate_at_index(8, cx);
    }

    // ========================================================================
    // Rendering
    // ========================================================================

    /// Renders the search bar.
    fn render_search_bar(&self, colors: &ExtensionViewColors, cx: &ViewContext<Self>) -> impl IntoElement {
        let search_bar = match &self.list_view.search_bar {
            ROption::RSome(sb) => sb,
            ROption::RNone => return div().flex_shrink_0(),
        };

        let placeholder = search_bar.placeholder.to_string();
        let has_query = !self.search_query.is_empty();

        div()
            .w_full()
            .h(SEARCH_BAR_HEIGHT)
            .px(PADDING)
            .flex()
            .items_center()
            .gap(px(8.0))
            .border_b_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                // Search icon
                div()
                    .text_color(colors.text_muted)
                    .child("🔍")
            )
            .child(
                // Search input container
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .child(self.render_search_input(&placeholder, has_query, colors, cx))
            )
    }

    /// Renders the search input with cursor.
    fn render_search_input(
        &self,
        placeholder: &str,
        has_query: bool,
        colors: &ExtensionViewColors,
        _cx: &ViewContext<Self>,
    ) -> impl IntoElement {
        if has_query {
            let before_cursor = &self.search_query[..self.cursor_position.min(self.search_query.len())];
            let after_cursor = &self.search_query[self.cursor_position.min(self.search_query.len())..];

            div()
                .flex()
                .items_center()
                .text_color(colors.text)
                .child(before_cursor.to_string())
                .when(self.cursor_visible(), |el| {
                    el.child(
                        div()
                            .w(px(1.0))
                            .h(px(16.0))
                            .bg(colors.accent)
                    )
                })
                .child(after_cursor.to_string())
        } else {
            div()
                .text_color(colors.text_placeholder)
                .child(placeholder.to_string())
        }
    }

    /// Renders the list content (sections and items).
    fn render_list_content(&self, colors: &ExtensionViewColors, cx: &ViewContext<Self>) -> impl IntoElement {
        let has_preview = self.list_view.show_preview;
        let preview_item = if has_preview {
            self.selected_item().and_then(|item| item.item.preview.clone().into_option())
        } else {
            None
        };

        div()
            .flex_1()
            .flex()
            .overflow_hidden()
            .child(
                // Main list
                div()
                    .id("list-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .child(self.render_sections(colors, cx))
            )
            .when(has_preview && preview_item.is_some(), |el| {
                el.child(
                    div()
                        .w(PREVIEW_WIDTH)
                        .border_l_1()
                        .border_color(colors.border)
                        .child(ExtensionPreviewPane::new(preview_item.unwrap(), colors.clone()))
                )
            })
    }

    /// Renders all sections.
    fn render_sections(&self, colors: &ExtensionViewColors, cx: &ViewContext<Self>) -> impl IntoElement {
        // Group filtered items by section
        let mut current_section = None;
        let mut elements: Vec<gpui::AnyElement> = Vec::new();

        for (display_idx, &flat_idx) in self.filtered_indices.iter().enumerate() {
            let flat_item = &self.flat_items[flat_idx];

            // Add section header if section changed
            if current_section != Some(flat_item.section_index) {
                current_section = Some(flat_item.section_index);
                if let Some(section) = self.list_view.sections.get(flat_item.section_index) {
                    if let ROption::RSome(title) = &section.title {
                        elements.push(
                            self.render_section_header(title.as_str(), colors)
                                .into_any_element()
                        );
                    }
                }
            }

            // Add item
            let is_selected = display_idx == self.selected_index;
            elements.push(
                self.render_list_item(&flat_item.item, display_idx, is_selected, colors, cx)
                    .into_any_element()
            );
        }

        div().children(elements)
    }

    /// Renders a section header.
    fn render_section_header(&self, title: &str, colors: &ExtensionViewColors) -> impl IntoElement {
        div()
            .h(SECTION_HEADER_HEIGHT)
            .px(PADDING)
            .flex()
            .items_center()
            .text_xs()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(colors.text_muted)
            .child(title.to_uppercase())
    }

    /// Renders a single list item.
    fn render_list_item(
        &self,
        item: &ListItem,
        index: usize,
        is_selected: bool,
        colors: &ExtensionViewColors,
        _cx: &ViewContext<Self>,
    ) -> impl IntoElement {
        let quick_number = if index < 9 {
            Some(format!("⌘{}", index + 1))
        } else {
            None
        };

        div()
            .h(ITEM_HEIGHT)
            .w_full()
            .px(PADDING)
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(BORDER_RADIUS)
            .cursor_pointer()
            .when(is_selected, |el| el.bg(colors.selection))
            .hover(|el| el.bg(colors.hover))
            // Icon
            .child(self.render_icon(&item.icon, colors))
            // Content
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(colors.text)
                            .truncate()
                            .child(item.title.to_string())
                    )
                    .when_some(item.subtitle.clone().into_option(), |el, subtitle| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(colors.text_muted)
                                .truncate()
                                .child(subtitle.to_string())
                        )
                    })
            )
            // Accessories
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .children(item.accessories.iter().map(|acc| {
                        self.render_accessory(acc, colors).into_any_element()
                    }))
            )
            // Quick number
            .when_some(quick_number, |el, num| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child(num)
                )
            })
    }

    /// Renders an icon.
    fn render_icon(&self, icon: &IconSource, _colors: &ExtensionViewColors) -> gpui::Div {
        match icon {
            IconSource::Emoji { glyph } => {
                div()
                    .w(px(32.0))
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_lg()
                    .child(glyph.to_string())
            },
            IconSource::SystemIcon { name } => {
                let emoji = Self::system_icon_to_emoji(name.as_str());
                div()
                    .w(px(32.0))
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_lg()
                    .child(emoji)
            },
            IconSource::AppIcon { icon_path, .. } => {
                if let ROption::RSome(path) = icon_path {
                    div()
                        .w(px(32.0))
                        .h(px(32.0))
                        .child(
                            img(SharedString::from(path.to_string()))
                                .w(px(32.0))
                                .h(px(32.0))
                        )
                } else {
                    div()
                        .w(px(32.0))
                        .h(px(32.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_lg()
                        .child("📱")
                }
            },
            IconSource::FileIcon { path } => {
                div()
                    .w(px(32.0))
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_lg()
                    .child(if path.ends_with('/') { "📁" } else { "📄" })
            },
        }
    }

    /// Converts system icon names to emojis.
    fn system_icon_to_emoji(name: &str) -> &'static str {
        match name {
            "lock" | "lock.fill" => "🔒",
            "moon.fill" | "sleep" => "😴",
            "arrow.clockwise" | "restart" => "🔄",
            "power" | "shutdown" => "⏻",
            "trash" | "trash.fill" => "🗑️",
            "magnifyingglass" => "🔍",
            "folder" | "folder.fill" => "📁",
            "doc" | "doc.fill" => "📄",
            "gearshape" | "gearshape.fill" => "⚙️",
            "star" | "star.fill" => "⭐",
            "heart" | "heart.fill" => "❤️",
            "checkmark" | "checkmark.circle.fill" => "✅",
            "xmark" | "xmark.circle.fill" => "❌",
            "plus" | "plus.circle.fill" => "➕",
            "minus" | "minus.circle.fill" => "➖",
            "info" | "info.circle.fill" => "ℹ️",
            "exclamationmark" | "exclamationmark.triangle.fill" => "⚠️",
            "clock" | "clock.fill" => "🕐",
            "calendar" => "📅",
            "person" | "person.fill" => "👤",
            "envelope" | "envelope.fill" => "✉️",
            "link" => "🔗",
            "globe" => "🌐",
            _ => "📋",
        }
    }

    /// Renders an accessory.
    fn render_accessory(&self, accessory: &Accessory, colors: &ExtensionViewColors) -> gpui::Div {
        match accessory {
            Accessory::Text(text) => {
                div()
                    .text_xs()
                    .text_color(colors.text_muted)
                    .child(text.to_string())
            },
            Accessory::Tag { text, color } => {
                div()
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .text_xs()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .bg(colors.tag_background(color))
                    .text_color(colors.tag_color(color))
                    .child(text.to_string())
            },
            Accessory::Date(duration) => {
                let secs = duration.as_secs();
                let text = Self::format_relative_time(secs);
                div()
                    .text_xs()
                    .text_color(colors.text_muted)
                    .child(text)
            },
            Accessory::Icon(icon) => {
                self.render_icon(icon, colors)
            },
        }
    }

    /// Formats a duration as relative time.
    fn format_relative_time(secs: u64) -> String {
        if secs < 60 {
            "just now".to_string()
        } else if secs < 3600 {
            let mins = secs / 60;
            format!("{}m ago", mins)
        } else if secs < 86400 {
            let hours = secs / 3600;
            format!("{}h ago", hours)
        } else {
            let days = secs / 86400;
            format!("{}d ago", days)
        }
    }

    /// Renders the empty state.
    fn render_empty_state(&self, empty_state: &EmptyState, colors: &ExtensionViewColors) -> impl IntoElement {
        // Pre-compute icon content
        let icon_content: Option<String> = empty_state.icon.clone().into_option().map(|icon| {
            match &icon {
                IconSource::SystemIcon { name } => Self::system_icon_to_emoji(name.as_str()).to_string(),
                IconSource::Emoji { glyph } => glyph.to_string(),
                _ => Self::system_icon_to_emoji("info").to_string(),
            }
        });

        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .p(PADDING)
            // Icon
            .when_some(icon_content, |el, content| {
                el.child(
                    div()
                        .text_2xl()
                        .mb(px(8.0))
                        .child(content)
                )
            })
            // Title
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child(empty_state.title.to_string())
            )
            // Description
            .when_some(empty_state.description.clone().into_option(), |el, desc| {
                el.child(
                    div()
                        .text_sm()
                        .text_color(colors.text_muted)
                        .child(desc.to_string())
                )
            })
            // Actions
            .when(!empty_state.actions.is_empty(), |el| {
                el.child(
                    div()
                        .flex()
                        .gap(px(8.0))
                        .mt(px(8.0))
                        .children(empty_state.actions.iter().map(|action| {
                            self.render_empty_state_action(action, colors).into_any_element()
                        }))
                )
            })
    }

    /// Renders an empty state action button.
    fn render_empty_state_action(&self, action: &Action, colors: &ExtensionViewColors) -> impl IntoElement {
        div()
            .px(px(16.0))
            .py(px(8.0))
            .rounded(BORDER_RADIUS)
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .cursor_pointer()
            .bg(colors.accent)
            .text_color(gpui::white())
            .hover(|el| el.bg(colors.accent_hover))
            .child(action.title.to_string())
    }

    /// Renders the loading indicator.
    fn render_loading(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_color(colors.text_muted)
                    .child("Loading...")
            )
    }

    /// Renders the error state.
    fn render_error(&self, error: &str, colors: &ExtensionViewColors) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .p(PADDING)
            .child(
                div()
                    .text_2xl()
                    .child("⚠️")
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.error)
                    .child("Error")
            )
            .child(
                div()
                    .text_sm()
                    .text_color(colors.text_muted)
                    .child(error.to_string())
            )
    }

    // ========================================================================
    // Actions Menu (Cmd+K)
    // ========================================================================

    /// Renders the action bar at the bottom (Raycast-style with primary action and shortcuts).
    fn render_action_bar(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        // Determine if there's a selected item with actions
        let has_actions = self.selected_item().is_some_and(|item| !item.item.actions.is_empty());
        let primary_action = self.selected_item()
            .and_then(|item| item.item.actions.first())
            .map(|a| a.title.to_string())
            .unwrap_or_default();

        div()
            .w_full()
            .h(px(36.0))
            .px(PADDING)
            .flex()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(colors.border)
            .bg(colors.surface)
            // Left side: Primary action
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(!primary_action.is_empty(), |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(4.0))
                                        .bg(colors.surface_hover)
                                        .text_size(px(10.0))
                                        .text_color(colors.text_muted)
                                        .child("↵"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(colors.text_muted)
                                        .child(primary_action),
                                ),
                        )
                    }),
            )
            // Right side: Actions shortcut
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(has_actions, |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(colors.text_placeholder)
                                        .child("Actions"),
                                )
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(4.0))
                                        .bg(colors.surface_hover)
                                        .text_size(px(10.0))
                                        .text_color(colors.text_muted)
                                        .child("⌘K"),
                                ),
                        )
                    }),
            )
    }

    /// Renders the actions menu popup (Cmd+K).
    fn render_actions_menu(&self, colors: &ExtensionViewColors, cx: &ViewContext<Self>) -> impl IntoElement {
        let actions = self.selected_item()
            .map(|item| item.item.actions.clone())
            .unwrap_or_default();
        let selected = self.actions_menu_index;

        div()
            // Overlay background - position menu above action bar at bottom-right
            .absolute()
            .inset_0()
            .flex()
            .items_end()
            .justify_end()
            .pb(px(44.0)) // Above action bar
            .pr(PADDING)
            // Click outside to close
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, cx| {
                this.show_actions_menu = false;
                cx.notify();
            }))
            .child(
                div()
                    .w(px(280.0))
                    .bg(colors.surface_elevated)
                    .rounded(BORDER_RADIUS)
                    .border_1()
                    .border_color(colors.border)
                    .shadow_lg()
                    .overflow_hidden()
                    // Stop propagation so clicking menu doesn't close it
                    .on_mouse_down(MouseButton::Left, |_, cx| cx.stop_propagation())
                    // Header
                    .child(
                        div()
                            .px(PADDING)
                            .py(px(8.0))
                            .border_b_1()
                            .border_color(colors.border)
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(colors.text)
                            .child("Actions"),
                    )
                    // Action items
                    .child(
                        div()
                            .id("actions-menu-items")
                            .py(px(4.0))
                            .max_h(px(300.0))
                            .overflow_y_scroll()
                            .children(actions.iter().enumerate().map(|(idx, action)| {
                                self.render_action_menu_item(action, idx == selected, colors, cx)
                                    .into_any_element()
                            })),
                    )
                    // Footer with hints
                    .child(
                        div()
                            .px(PADDING)
                            .py(px(6.0))
                            .border_t_1()
                            .border_color(colors.border)
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_placeholder)
                                    .child("↑↓ Navigate  ↵ Select  esc Close"),
                            ),
                    ),
            )
    }

    /// Renders a single action item in the actions menu.
    fn render_action_menu_item(
        &self,
        action: &Action,
        is_selected: bool,
        colors: &ExtensionViewColors,
        cx: &ViewContext<Self>,
    ) -> impl IntoElement {
        use photoncast_extension_api::ActionStyle;
        
        // Determine colors based on action style
        let (text_color, bg_color) = match action.style {
            ActionStyle::Destructive => (
                colors.error,
                if is_selected { colors.error.opacity(0.2) } else { gpui::transparent_black() },
            ),
            ActionStyle::Primary => (
                colors.accent,
                if is_selected { colors.selection } else { gpui::transparent_black() },
            ),
            ActionStyle::Default => (
                colors.text,
                if is_selected { colors.selection } else { gpui::transparent_black() },
            ),
        };

        // Format shortcut if present
        let shortcut_str = action.shortcut.clone().into_option().map(|s| {
            let mut parts = Vec::new();
            if s.modifiers.cmd { parts.push("⌘"); }
            if s.modifiers.shift { parts.push("⇧"); }
            if s.modifiers.alt { parts.push("⌥"); }
            if s.modifiers.ctrl { parts.push("⌃"); }
            parts.push(s.key.as_str());
            parts.join("")
        });

        let action_clone = action.clone();
        
        div()
            .px(PADDING)
            .py(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(bg_color)
            .cursor_pointer()
            .hover(|el| el.bg(colors.hover))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, cx| {
                this.execute_action(&action_clone, cx);
            }))
            // Icon and title
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    // Icon
                    .when_some(action.icon.clone().into_option(), |el, icon| {
                        el.child(
                            div()
                                .w(px(16.0))
                                .h(px(16.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(12.0))
                                .child(match icon {
                                    IconSource::Emoji { glyph } => glyph.to_string(),
                                    IconSource::SystemIcon { name } => {
                                        Self::system_icon_to_emoji(name.as_str()).to_string()
                                    },
                                    _ => "📋".to_string(),
                                }),
                        )
                    })
                    // Title
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(text_color)
                            .child(action.title.to_string()),
                    ),
            )
            // Shortcut hint
            .when_some(shortcut_str, |el: gpui::Div, shortcut| {
                el.child(
                    div()
                        .px(px(4.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(colors.surface)
                        .text_size(px(10.0))
                        .text_color(colors.text_muted)
                        .child(shortcut),
                )
            })
    }
}

impl FocusableView for ExtensionListView {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ExtensionListView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = ExtensionViewColors::from_context(cx);
        let has_search = self.list_view.search_bar.is_some();
        let is_empty = self.filtered_indices.is_empty();
        let has_empty_state = self.list_view.empty_state.is_some();
        let show_actions_menu = self.show_actions_menu;

        div()
            .key_context("ExtensionListView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::show_actions_menu))
            .on_action(cx.listener(Self::quick_select_1))
            .on_action(cx.listener(Self::quick_select_2))
            .on_action(cx.listener(Self::quick_select_3))
            .on_action(cx.listener(Self::quick_select_4))
            .on_action(cx.listener(Self::quick_select_5))
            .on_action(cx.listener(Self::quick_select_6))
            .on_action(cx.listener(Self::quick_select_7))
            .on_action(cx.listener(Self::quick_select_8))
            .on_action(cx.listener(Self::quick_select_9))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, cx| {
                // Handle text input for search
                if this.list_view.search_bar.is_some() {
                    let key = &event.keystroke.key;
                    if key.len() == 1 && !event.keystroke.modifiers.platform {
                        // Single character input
                        let ch = key.chars().next().unwrap();
                        this.search_query.insert(this.cursor_position, ch);
                        this.cursor_position += 1;
                        this.reset_cursor_blink();
                        this.schedule_search(cx);
                        cx.notify();
                    } else if key == "backspace" {
                        if this.cursor_position > 0 {
                            this.cursor_position -= 1;
                            this.search_query.remove(this.cursor_position);
                            this.reset_cursor_blink();
                            this.schedule_search(cx);
                            cx.notify();
                        }
                    } else if key == "delete" {
                        if this.cursor_position < this.search_query.len() {
                            this.search_query.remove(this.cursor_position);
                            this.reset_cursor_blink();
                            this.schedule_search(cx);
                            cx.notify();
                        }
                    } else if key == "left" {
                        if this.cursor_position > 0 {
                            this.cursor_position -= 1;
                            this.reset_cursor_blink();
                            cx.notify();
                        }
                    } else if key == "right" {
                        if this.cursor_position < this.search_query.len() {
                            this.cursor_position += 1;
                            this.reset_cursor_blink();
                            cx.notify();
                        }
                    }
                }
            }))
            .size_full() // Fill parent container
            .flex()
            .flex_col()
            .relative() // Enable absolute positioning for actions menu overlay
            .bg(colors.background)
            .overflow_hidden()
            // Title
            .child(
                div()
                    .h(px(40.0))
                    .px(PADDING)
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border)
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child(self.list_view.title.to_string())
            )
            // Search bar
            .when(has_search, |el| el.child(self.render_search_bar(&colors, cx)))
            // Content
            .child(
                if self.loading {
                    self.render_loading(&colors).into_any_element()
                } else if let Some(ref error) = self.error {
                    self.render_error(error, &colors).into_any_element()
                } else if is_empty && has_empty_state {
                    if let ROption::RSome(ref empty_state) = self.list_view.empty_state {
                        self.render_empty_state(empty_state, &colors).into_any_element()
                    } else {
                        self.render_list_content(&colors, cx).into_any_element()
                    }
                } else {
                    self.render_list_content(&colors, cx).into_any_element()
                }
            )
            // Action bar at bottom
            .child(self.render_action_bar(&colors))
            // Actions menu overlay (Cmd+K)
            .when(show_actions_menu, |el| {
                el.child(self.render_actions_menu(&colors, cx))
            })
    }
}
