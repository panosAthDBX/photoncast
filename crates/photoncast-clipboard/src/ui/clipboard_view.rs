//! Clipboard history view.
//!
//! Main view for the clipboard history panel with search, keyboard navigation,
//! and clipboard actions.

use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, rems, rgba, AnyElement, AppContext, FocusHandle, FocusableView, FontWeight,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Task, ViewContext,
};
use photoncast_theme::GpuiThemeColors;

use crate::config::ClipboardConfig;
use crate::models::{ClipboardContentType, ClipboardItem, ClipboardItemId};
use crate::storage::ClipboardStorage;

use super::actions::{
    ClearClipboardHistory, CloseClipboardHistory, CopyClipboardItem, DeleteClipboardItem,
    PasteAsPlainText, PasteClipboardItem, SelectNextClipboardItem, SelectPreviousClipboardItem,
    TogglePinClipboardItem,
};
/// Search debounce duration.
const SEARCH_DEBOUNCE_MS: u64 = 100;

/// Maximum preview text length.
const MAX_PREVIEW_LENGTH: usize = 100;

/// Type alias – clipboard UI uses the shared [`GpuiThemeColors`].
type ClipboardColors = GpuiThemeColors;

fn get_clipboard_colors(cx: &ViewContext<ClipboardHistoryView>) -> ClipboardColors {
    ClipboardColors::from_context(cx)
}

/// Clipboard history view state.
pub struct ClipboardHistoryView {
    /// Storage backend.
    storage: ClipboardStorage,
    /// Configuration.
    config: ClipboardConfig,
    /// Pinned items.
    pinned_items: Vec<ClipboardItem>,
    /// Recent items.
    recent_items: Vec<ClipboardItem>,
    /// Current search query.
    search_query: String,
    /// Cursor position in search query.
    search_cursor: usize,
    /// Currently selected index.
    selected_index: usize,
    /// Scroll handle for keyboard navigation.
    scroll_handle: gpui::ScrollHandle,
    /// Focus handle.
    focus_handle: FocusHandle,
    /// Whether we're in search mode.
    is_searching: bool,
    /// Whether loading is in progress.
    is_loading: bool,
    /// Pending search task (for debounce).
    pending_search: Option<Task<()>>,
    /// Whether to show the confirmation dialog.
    show_clear_confirmation: bool,
    /// Default action: true = paste, false = copy.
    default_action_paste: bool,
}

impl ClipboardHistoryView {
    /// Creates a new clipboard history view.
    pub fn new(
        storage: ClipboardStorage,
        config: ClipboardConfig,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let default_action_paste = config.default_action_paste();

        let view = Self {
            storage,
            config,
            pinned_items: Vec::new(),
            recent_items: Vec::new(),
            search_query: String::new(),
            search_cursor: 0,
            selected_index: 0,
            scroll_handle: gpui::ScrollHandle::new(),
            focus_handle,
            is_searching: false,
            is_loading: true,
            pending_search: None,
            show_clear_confirmation: false,
            default_action_paste,
        };

        // Load items after construction using cx.defer to avoid blocking window creation
        view.load_items_deferred(cx);

        view
    }

    /// Loads items in a deferred manner to avoid blocking window creation.
    fn load_items_deferred(&self, cx: &mut ViewContext<Self>) {
        let storage = self.storage.clone();
        let history_size = self.config.history_size;

        // Use defer to run after the current frame
        cx.defer(move |view, cx| {
            // Load synchronously but after window is created
            view.pinned_items = storage.load_pinned().unwrap_or_default();
            view.recent_items = storage.load_recent(history_size).unwrap_or_default();
            view.is_loading = false;
            cx.notify();
        });
    }

    /// Refreshes the clipboard items (called when window is shown).
    pub fn refresh(&mut self, cx: &mut ViewContext<Self>) {
        self.pinned_items = self.storage.load_pinned().unwrap_or_default();
        self.recent_items = self
            .storage
            .load_recent(self.config.history_size)
            .unwrap_or_default();
        self.selected_index = 0;
        cx.focus(&self.focus_handle);
        cx.notify();
    }

    /// Loads items from storage.
    fn load_items(&mut self, cx: &mut ViewContext<Self>) {
        self.is_loading = true;
        let storage = self.storage.clone();
        let config = self.config.clone();

        cx.spawn(|this, mut cx| async move {
            let pinned = storage.load_pinned_async().await.unwrap_or_default();
            let recent = storage
                .load_recent_async(config.history_size)
                .await
                .unwrap_or_default();

            this.update(&mut cx, |view, cx| {
                view.pinned_items = pinned;
                view.recent_items = recent;
                view.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Performs a search with debounce.
    pub fn search(&mut self, query: String, cx: &mut ViewContext<Self>) {
        self.search_query.clone_from(&query);
        self.is_searching = !query.is_empty();

        // Cancel pending search
        self.pending_search.take();

        if query.is_empty() {
            // Clear search immediately
            self.load_items(cx);
            return;
        }

        // Debounce the search
        let storage = self.storage.clone();
        self.pending_search = Some(cx.spawn(|this, mut cx| async move {
            // Wait for debounce period
            cx.background_executor()
                .timer(Duration::from_millis(SEARCH_DEBOUNCE_MS))
                .await;

            let results = storage.search_async(query).await.unwrap_or_default();

            this.update(&mut cx, |view, cx| {
                if view.is_searching {
                    view.recent_items = results;
                    view.pinned_items.clear();
                }
                view.selected_index = 0;
                cx.notify();
            })
            .ok();
        }));
    }

    /// Clears the search and reloads items.
    pub fn clear_search(&mut self, cx: &mut ViewContext<Self>) {
        self.search_query.clear();
        self.search_cursor = 0;
        self.is_searching = false;
        self.pending_search.take();
        self.load_items(cx);
    }

    /// Returns the currently selected item.
    pub fn selected_item(&self) -> Option<&ClipboardItem> {
        let total_pinned = self.pinned_items.len();

        if self.selected_index < total_pinned {
            self.pinned_items.get(self.selected_index)
        } else {
            self.recent_items.get(self.selected_index - total_pinned)
        }
    }

    /// Returns the ID of the currently selected item.
    pub fn selected_item_id(&self) -> Option<ClipboardItemId> {
        self.selected_item().map(|item| item.id.clone())
    }

    /// Returns total item count.
    fn total_items(&self) -> usize {
        self.pinned_items.len() + self.recent_items.len()
    }

    /// Selects the next item.
    pub fn select_next(&mut self, cx: &mut ViewContext<Self>) {
        let total = self.total_items();
        if total > 0 {
            self.selected_index = (self.selected_index + 1) % total;
            self.scroll_handle.scroll_to_item(self.selected_index);
            cx.notify();
        }
    }

    /// Selects the previous item.
    pub fn select_previous(&mut self, cx: &mut ViewContext<Self>) {
        let total = self.total_items();
        if total > 0 {
            self.selected_index = if self.selected_index == 0 {
                total - 1
            } else {
                self.selected_index - 1
            };
            self.scroll_handle.scroll_to_item(self.selected_index);
            cx.notify();
        }
    }

    /// Handles selection with click.
    #[allow(dead_code)]
    fn select_item(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        if index < self.total_items() {
            self.selected_index = index;
            cx.notify();
        }
    }

    /// Toggles pin status of selected item.
    pub fn toggle_pin_selected(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(item) = self.selected_item() {
            let id = item.id.clone();
            let new_pinned = !item.is_pinned;
            let storage = self.storage.clone();

            cx.spawn(|this, mut cx| async move {
                storage.set_pinned_async(id, new_pinned).await.ok();
                this.update(&mut cx, |view, cx| {
                    view.load_items(cx);
                })
                .ok();
            })
            .detach();
        }
    }

    /// Deletes the selected item.
    pub fn delete_selected(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(item) = self.selected_item() {
            let id = item.id.clone();
            let storage = self.storage.clone();

            cx.spawn(|this, mut cx| async move {
                storage.delete_async(id).await.ok();
                this.update(&mut cx, |view, cx| {
                    // Adjust selection if needed
                    let total = view.total_items().saturating_sub(1);
                    if view.selected_index >= total && total > 0 {
                        view.selected_index = total - 1;
                    }
                    view.load_items(cx);
                })
                .ok();
            })
            .detach();
        }
    }

    /// Clears all clipboard history.
    pub fn clear_all(&mut self, cx: &mut ViewContext<Self>) {
        let storage = self.storage.clone();

        cx.spawn(|this, mut cx| async move {
            storage.clear_all_async().await.ok();
            this.update(&mut cx, |view, cx| {
                view.pinned_items.clear();
                view.recent_items.clear();
                view.selected_index = 0;
                view.show_clear_confirmation = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Pastes the selected item to the frontmost application.
    pub fn paste_selected(&self, _cx: &mut ViewContext<Self>) {
        if let Some(item) = self.selected_item() {
            Self::copy_to_system_clipboard(item);
            // TODO: Simulate Cmd+V to paste
        }
    }

    /// Copies the selected item to clipboard without pasting.
    pub fn copy_selected(&self, _cx: &mut ViewContext<Self>) {
        if let Some(item) = self.selected_item() {
            Self::copy_to_system_clipboard(item);
        }
    }

    /// Copies an item to the system clipboard.
    fn copy_to_system_clipboard(item: &ClipboardItem) {
        #[cfg(target_os = "macos")]
        {
            use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
            use objc2_foundation::NSString;

            unsafe {
                let pasteboard = NSPasteboard::generalPasteboard();
                pasteboard.clearContents();

                if let Some(text) = item.content_type.text_content() {
                    let ns_string = NSString::from_str(text);
                    pasteboard.setString_forType(&ns_string, NSPasteboardTypeString);
                }
            }
        }
    }

    /// Renders the search bar.
    fn render_search_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_clipboard_colors(cx);
        let accent = colors.accent;
        let query = self.search_query.clone();
        let is_searching = self.is_searching;
        let has_query = !query.is_empty();

        // Block cursor dimensions (matches launcher)
        let cursor_width = px(9.0);
        let cursor_height = px(20.0);

        // Split text at cursor position
        let chars: Vec<char> = query.chars().collect();
        let cursor_pos = self.search_cursor.min(chars.len());
        let before: String = chars[..cursor_pos].iter().collect();
        let after: String = chars[cursor_pos..].iter().collect();

        div()
            .px(rems(0.75))
            .py(rems(0.5))
            .border_b_1()
            .border_color(colors.surface)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.5))
                    .child(div().text_sm().text_color(colors.text_faint).child("🔍"))
                    .child(
                        div()
                            .id("search-input")
                            .flex_1()
                            .text_sm()
                            .flex()
                            .items_center()
                            .when(!has_query, |el| {
                                // Empty field: cursor then placeholder
                                el.child(
                                    div()
                                        .w(cursor_width)
                                        .h(cursor_height)
                                        .bg(accent)
                                        .rounded(px(2.0)),
                                )
                                .child(
                                    div()
                                        .text_color(colors.text_faint)
                                        .child("Search clipboard history..."),
                                )
                            })
                            .when(has_query, |el| {
                                el.text_color(colors.text)
                                    .when(!before.is_empty(), |el| el.child(before.clone()))
                                    .child(
                                        div()
                                            .w(cursor_width)
                                            .h(cursor_height)
                                            .bg(accent)
                                            .rounded(px(2.0)),
                                    )
                                    .when(!after.is_empty(), |el| el.child(after.clone()))
                            }),
                    )
                    .when(is_searching, |el| {
                        el.child(
                            div()
                                .id("clear-search")
                                .text_xs()
                                .text_color(colors.text_faint)
                                .cursor_pointer()
                                .child("✕"),
                        )
                    }),
            )
    }

    /// Renders the section header.
    fn render_section_header(
        title: &str,
        icon: &str,
        count: usize,
        colors: &ClipboardColors,
    ) -> impl IntoElement {
        let overlay0 = colors.text_faint;
        let surface2 = colors.surface_tertiary;
        div().px(rems(0.75)).py(rems(0.375)).child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(rems(0.375))
                        .items_center()
                        .child(div().text_xs().child(icon.to_string()))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(overlay0)
                                .child(title.to_string()),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(surface2)
                        .child(count.to_string()),
                ),
        )
    }

    /// Renders the icon/preview for an item based on content type.
    fn render_item_icon(
        content_type: &ClipboardContentType,
        colors: &ClipboardColors,
    ) -> impl IntoElement {
        let surface1 = colors.surface_hover;
        match content_type {
            ClipboardContentType::Color { rgb: color_rgb, .. } => {
                // Render color swatch
                let color = gpui::rgb(
                    (u32::from(color_rgb.0) << 16)
                        | (u32::from(color_rgb.1) << 8)
                        | u32::from(color_rgb.2),
                );

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
                            .border_color(surface1),
                    )
            },
            ClipboardContentType::Image { thumbnail_path, .. } => {
                // For images, show thumbnail indicator
                if thumbnail_path.exists() {
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.0))
                        .child(
                            div()
                                .size(px(18.0))
                                .rounded(px(4.0))
                                .bg(surface1)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(div().text_xs().child("🖼️")),
                        )
                } else {
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.0))
                        .child(div().text_sm().child("🖼️"))
                }
            },
            ClipboardContentType::Link { favicon_path, .. } => {
                // Show favicon or link icon
                if favicon_path.as_ref().is_some_and(|p| p.exists()) {
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.0))
                        .child(
                            div()
                                .size(px(16.0))
                                .rounded(px(2.0))
                                .bg(surface1)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(div().text_xs().child("🔗")),
                        )
                } else {
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.0))
                        .child(div().text_sm().child("🔗"))
                }
            },
            _ => {
                // Default icons for other types
                let icon = match content_type {
                    ClipboardContentType::Text { .. } => "📝",
                    ClipboardContentType::RichText { .. } => "📄",
                    ClipboardContentType::File { .. } => "📁",
                    _ => "📋",
                };
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(24.0))
                    .child(div().text_sm().child(icon))
            },
        }
    }

    /// Renders the main content area for an item.
    #[allow(clippy::too_many_lines)]
    fn render_item_content(
        content_type: &ClipboardContentType,
        colors: &ClipboardColors,
    ) -> impl IntoElement {
        let text_color = colors.text;
        let overlay0 = colors.text_faint;
        match content_type {
            ClipboardContentType::Link { url, title, .. } => {
                // Show title and URL for links
                let display_title = title.clone().unwrap_or_else(|| url.clone());
                let show_url = title.is_some();

                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_color)
                            .truncate()
                            .child(display_title),
                    )
                    .when(show_url, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(overlay0)
                                .truncate()
                                .child(url.clone()),
                        )
                    })
            },
            ClipboardContentType::Color {
                hex, display_name, ..
            } => {
                // Show hex and display name for colors
                let has_name = display_name.is_some();

                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_color)
                            .font_family("monospace")
                            .child(hex.clone()),
                    )
                    .when(has_name, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(overlay0)
                                .child(display_name.clone().unwrap_or_default()),
                        )
                    })
            },
            ClipboardContentType::Image {
                dimensions,
                size_bytes,
                ..
            } => {
                // Show image dimensions and size
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    .child(div().text_sm().text_color(text_color).child(format!(
                        "{}×{} • {}",
                        dimensions.0,
                        dimensions.1,
                        format_kilobytes(*size_bytes)
                    )))
            },
            ClipboardContentType::File {
                paths, total_size, ..
            } => {
                // Show file names and count
                let names: Vec<_> = paths
                    .iter()
                    .filter_map(|p| p.file_name())
                    .filter_map(|n| n.to_str())
                    .take(2)
                    .collect();
                let suffix = if paths.len() > 2 {
                    format!(" +{} more", paths.len() - 2)
                } else {
                    String::new()
                };

                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_color)
                            .truncate()
                            .child(format!("{}{}", names.join(", "), suffix)),
                    )
                    .child(div().text_xs().text_color(overlay0).child(format!(
                        "{} files • {}",
                        paths.len(),
                        format_kilobytes(*total_size)
                    )))
            },
            _ => {
                // Default text preview
                let preview = truncate_preview(&content_type.preview(), MAX_PREVIEW_LENGTH);
                div().flex().flex_col().overflow_hidden().child(
                    div()
                        .text_sm()
                        .text_color(text_color)
                        .truncate()
                        .child(preview),
                )
            },
        }
    }

    /// Renders item metadata (pin indicator and time).
    fn render_item_metadata(
        pinned: bool,
        time: String,
        colors: &ClipboardColors,
    ) -> impl IntoElement {
        let overlay0 = colors.text_faint;
        div()
            .flex()
            .flex_row()
            .gap(rems(0.375))
            .items_center()
            .flex_shrink_0()
            .when(pinned, |el| el.child(div().text_xs().child("📌")))
            .child(div().text_xs().text_color(overlay0).child(time))
    }

    /// Renders a clipboard item row.
    fn render_item(
        item: &ClipboardItem,
        index: usize,
        is_selected: bool,
        colors: &ClipboardColors,
    ) -> impl IntoElement {
        let bg_color = if is_selected {
            colors.surface_hover
        } else {
            colors.background
        };
        let surface0 = colors.surface;

        let time = relative_time(&item.created_at);
        let pinned = item.is_pinned;

        div()
            .id(("clipboard-item", index))
            .w_full()
            .px(rems(0.75))
            .py(rems(0.5))
            .bg(bg_color)
            .rounded(px(4.0))
            .cursor_pointer()
            .hover(|style| style.bg(surface0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(rems(0.5))
                    .items_center()
                    // Content type icon or preview
                    .child(Self::render_item_icon(&item.content_type, colors))
                    // Main content area
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(Self::render_item_content(&item.content_type, colors)),
                    )
                    // Metadata (pin, time)
                    .child(Self::render_item_metadata(pinned, time, colors)),
            )
    }

    /// Renders empty state when no items.
    fn render_empty_state(&self, colors: &ClipboardColors) -> impl IntoElement {
        let overlay0 = colors.text_faint;
        let surface2 = colors.surface_tertiary;
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h_full()
            .min_h(px(200.0))
            .gap(rems(0.5))
            .child(div().text_3xl().child("📋"))
            .child(
                div()
                    .text_sm()
                    .text_color(overlay0)
                    .child(if self.is_searching {
                        "No results found"
                    } else {
                        "No clipboard history"
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(surface2)
                    .child(if self.is_searching {
                        "Try a different search term"
                    } else {
                        "Items you copy will appear here"
                    }),
            )
    }

    /// Renders loading state.
    fn render_loading_state(colors: &ClipboardColors) -> impl IntoElement {
        let overlay0 = colors.text_faint;
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h_full()
            .min_h(px(200.0))
            .gap(rems(0.5))
            .child(div().text_2xl().child("⏳"))
            .child(div().text_sm().text_color(overlay0).child("Loading..."))
    }

    /// Renders the content area with items.
    fn render_content(&self, colors: &ClipboardColors) -> impl IntoElement {
        let mut elements: Vec<AnyElement> = Vec::new();
        let mut current_index = 0;

        // Pinned section
        if !self.pinned_items.is_empty() && !self.is_searching {
            elements.push(
                Self::render_section_header("PINNED", "📌", self.pinned_items.len(), colors)
                    .into_any_element(),
            );
            for item in &self.pinned_items {
                let is_selected = current_index == self.selected_index;
                elements.push(
                    Self::render_item(item, current_index, is_selected, colors).into_any_element(),
                );
                current_index += 1;
            }
        }

        // Recent section
        if !self.recent_items.is_empty() {
            let (header, icon) = if self.is_searching {
                ("SEARCH RESULTS", "🔍")
            } else {
                ("RECENT", "📋")
            };
            elements.push(
                Self::render_section_header(header, icon, self.recent_items.len(), colors)
                    .into_any_element(),
            );
            for item in &self.recent_items {
                let is_selected = current_index == self.selected_index;
                elements.push(
                    Self::render_item(item, current_index, is_selected, colors).into_any_element(),
                );
                current_index += 1;
            }
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(elements)
    }

    /// Renders the action bar at the bottom.
    fn render_action_bar(&self, colors: &ClipboardColors) -> impl IntoElement {
        let default_action = if self.default_action_paste {
            "Paste"
        } else {
            "Copy"
        };
        let surface0 = colors.surface;
        let overlay0 = colors.text_faint;

        div()
            .border_t_1()
            .border_color(surface0)
            .px(rems(0.75))
            .py(rems(0.5))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap_x(rems(1.0))
                    .gap_y(rems(0.25))
                    .text_xs()
                    .text_color(overlay0)
                    .child(render_shortcut("⏎", default_action, colors))
                    .child(render_shortcut("⌘C", "Copy", colors))
                    .child(render_shortcut("⌘⇧V", "Plain", colors))
                    .child(render_shortcut("⌘P", "Pin", colors))
                    .child(render_shortcut("⌘⌫", "Delete", colors)),
            )
    }

    /// Renders the clear confirmation dialog.
    fn render_clear_confirmation(colors: &ClipboardColors) -> impl IntoElement {
        let surface0 = colors.surface;
        let surface1 = colors.surface_hover;
        let surface2 = colors.surface_tertiary;
        let text_color = colors.text;
        let overlay1 = colors.text_placeholder;
        let red = colors.error;
        let base = colors.background;

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x0000_0088))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .bg(surface0)
                    .rounded(px(8.0))
                    .p(rems(1.0))
                    .max_w(px(300.0))
                    .flex()
                    .flex_col()
                    .gap(rems(0.75))
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(text_color)
                            .child("Clear All History?"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(overlay1)
                            .child("This will permanently delete all clipboard history items."),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(rems(0.5))
                            .justify_end()
                            .child(
                                div()
                                    .id("cancel-clear")
                                    .px(rems(0.75))
                                    .py(rems(0.375))
                                    .rounded(px(4.0))
                                    .bg(surface1)
                                    .text_sm()
                                    .text_color(text_color)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(surface2))
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("confirm-clear")
                                    .px(rems(0.75))
                                    .py(rems(0.375))
                                    .rounded(px(4.0))
                                    .bg(red)
                                    .text_sm()
                                    .text_color(base)
                                    .cursor_pointer()
                                    .hover(|s| s.opacity(0.9))
                                    .child("Clear All"),
                            ),
                    ),
            )
    }

    /// Handles key down events.
    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        let key = event.keystroke.key.as_str();

        // Arrow navigation for list items (up/down)
        match key {
            "down" => {
                self.select_next(cx);
                return;
            },
            "up" => {
                self.select_previous(cx);
                return;
            },
            "enter" => {
                if self.default_action_paste {
                    self.paste_selected(cx);
                } else {
                    self.copy_selected(cx);
                }
                return;
            },
            "escape" => {
                if self.show_clear_confirmation {
                    self.show_clear_confirmation = false;
                    cx.notify();
                } else if self.is_searching {
                    self.clear_search(cx);
                } else {
                    cx.remove_window();
                }
                return;
            },
            _ => {},
        }

        // Cursor movement for search (left/right)
        let len = self.search_query.chars().count();
        match key {
            "left" => {
                if event.keystroke.modifiers.platform {
                    self.search_cursor = 0;
                } else if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                }
                cx.notify();
                return;
            },
            "right" => {
                if event.keystroke.modifiers.platform {
                    self.search_cursor = len;
                } else if self.search_cursor < len {
                    self.search_cursor += 1;
                }
                cx.notify();
                return;
            },
            "backspace" => {
                if event.keystroke.modifiers.platform {
                    // Delete everything before cursor
                    if self.search_cursor > 0 {
                        let chars: Vec<char> = self.search_query.chars().collect();
                        let new_query: String = chars[self.search_cursor..].iter().collect();
                        self.search_cursor = 0;
                        if new_query.is_empty() {
                            self.clear_search(cx);
                        } else {
                            self.search(new_query, cx);
                        }
                        cx.notify();
                    }
                } else if self.search_cursor > 0 {
                    // Delete char before cursor
                    let mut chars: Vec<char> = self.search_query.chars().collect();
                    chars.remove(self.search_cursor - 1);
                    self.search_cursor -= 1;
                    let new_query: String = chars.into_iter().collect();
                    if new_query.is_empty() {
                        self.clear_search(cx);
                    } else {
                        self.search(new_query, cx);
                    }
                    cx.notify();
                }
                return;
            },
            _ => {},
        }

        // Handle Cmd+V paste
        if event.keystroke.modifiers.platform && key == "v" {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                let chars: Vec<char> = self.search_query.chars().collect();
                let before: String = chars[..self.search_cursor].iter().collect();
                let after: String = chars[self.search_cursor..].iter().collect();
                let new_query = format!("{}{}{}", before, text, after);
                self.search_cursor += text.chars().count();
                self.search(new_query, cx);
                cx.notify();
            }
            return;
        }

        if event.keystroke.modifiers.platform
            || event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
        {
            return;
        }

        // Insert text at cursor position
        let insert_text = |this: &mut Self, text: String, cx: &mut ViewContext<Self>| {
            let chars: Vec<char> = this.search_query.chars().collect();
            let before: String = chars[..this.search_cursor].iter().collect();
            let after: String = chars[this.search_cursor..].iter().collect();
            let new_query = format!("{}{}{}", before, text, after);
            this.search_cursor += text.chars().count();
            this.search(new_query, cx);
            cx.notify();
        };

        if let Some(ime_key) = &event.keystroke.ime_key {
            insert_text(self, ime_key.clone(), cx);
        } else if event.keystroke.key.len() == 1 {
            let key = if event.keystroke.modifiers.shift {
                event.keystroke.key.to_uppercase()
            } else {
                event.keystroke.key.clone()
            };
            insert_text(self, key, cx);
        }
    }
}

impl FocusableView for ClipboardHistoryView {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ClipboardHistoryView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_clipboard_colors(cx);
        let has_items = !self.pinned_items.is_empty() || !self.recent_items.is_empty();
        let show_confirmation = self.show_clear_confirmation;
        let is_loading = self.is_loading;

        div()
            .track_focus(&self.focus_handle)
            .key_context("ClipboardHistory")
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_action(cx.listener(|this, _: &SelectNextClipboardItem, cx| {
                this.select_next(cx);
            }))
            .on_action(cx.listener(|this, _: &SelectPreviousClipboardItem, cx| {
                this.select_previous(cx);
            }))
            .on_action(cx.listener(|this, _: &PasteClipboardItem, cx| {
                this.paste_selected(cx);
            }))
            .on_action(cx.listener(|this, _: &PasteAsPlainText, cx| {
                this.copy_selected(cx);
            }))
            .on_action(cx.listener(|this, _: &CopyClipboardItem, cx| {
                this.copy_selected(cx);
            }))
            .on_action(cx.listener(|this, _: &TogglePinClipboardItem, cx| {
                this.toggle_pin_selected(cx);
            }))
            .on_action(cx.listener(|this, _: &DeleteClipboardItem, cx| {
                this.delete_selected(cx);
            }))
            .on_action(cx.listener(|_, _: &CloseClipboardHistory, cx| {
                cx.remove_window();
            }))
            .on_action(cx.listener(|this, _: &ClearClipboardHistory, cx| {
                if this.show_clear_confirmation {
                    this.clear_all(cx);
                } else {
                    this.show_clear_confirmation = true;
                    cx.notify();
                }
            }))
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(colors.background)
            .pt(rems(1.5))
            // Search bar
            .child(self.render_search_bar(cx))
            // Content area
            .child(
                div()
                    .id("clipboard-scroll-container")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .py(rems(0.25))
                    .child(if is_loading {
                        Self::render_loading_state(&colors).into_any_element()
                    } else if has_items {
                        self.render_content(&colors).into_any_element()
                    } else {
                        self.render_empty_state(&colors).into_any_element()
                    }),
            )
            // Action bar
            .child(self.render_action_bar(&colors))
            // Clear confirmation dialog
            .when(show_confirmation, |el| {
                el.child(Self::render_clear_confirmation(&colors))
            })
    }
}

/// Renders a keyboard shortcut hint.
fn render_shortcut(key: &str, label: &str, colors: &ClipboardColors) -> impl IntoElement {
    let surface0 = colors.surface;
    let subtext0 = colors.text_muted;
    let overlay0 = colors.text_faint;
    div()
        .flex()
        .flex_row()
        .gap(rems(0.25))
        .items_center()
        .child(
            div()
                .px(rems(0.25))
                .py(px(1.0))
                .rounded(px(2.0))
                .bg(surface0)
                .text_xs()
                .font_family("monospace")
                .text_color(subtext0)
                .child(key.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(overlay0)
                .child(label.to_string()),
        )
}

/// Returns a relative time string.
fn relative_time(datetime: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*datetime);

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
        datetime.format("%b %d").to_string()
    }
}

fn format_kilobytes(size_bytes: u64) -> String {
    let whole = size_bytes / 1024;
    let remainder = size_bytes % 1024;
    let decimal = remainder * 10 / 1024;
    format!("{whole}.{decimal} KB")
}

/// Truncates preview text to max length (character count, not bytes).
fn truncate_preview(text: &str, max_len: usize) -> String {
    // Normalize whitespace and truncate
    let normalized: String = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if normalized.chars().count() > max_len {
        let truncated: String = normalized.chars().take(max_len).collect();
        format!("{truncated}...")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_time() {
        let now = chrono::Utc::now();

        // Just now
        assert_eq!(relative_time(&now), "Just now");

        // Minutes ago
        let five_mins_ago = now - chrono::Duration::minutes(5);
        assert_eq!(relative_time(&five_mins_ago), "5m");

        // Hours ago
        let two_hours_ago = now - chrono::Duration::hours(2);
        assert_eq!(relative_time(&two_hours_ago), "2h");

        // Days ago
        let three_days_ago = now - chrono::Duration::days(3);
        assert_eq!(relative_time(&three_days_ago), "3d");
    }

    #[test]
    fn test_truncate_preview() {
        assert_eq!(truncate_preview("Hello", 10), "Hello");
        assert_eq!(truncate_preview("Hello World Test", 10), "Hello Worl...");
        assert_eq!(
            truncate_preview("  Multiple   spaces  ", 20),
            "Multiple spaces"
        );
    }
}
