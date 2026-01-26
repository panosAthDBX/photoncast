//! GridView rendering for extensions.
//!
//! Renders `GridView` types with:
//! - Configurable grid columns (2-6)
//! - Items with image, title, subtitle
//! - Image sources: Path, URL, Base64, SfSymbol
//! - Keyboard navigation (←→↑↓, Enter)

use gpui::prelude::FluentBuilder;
use gpui::*;
use photoncast_extension_api::{Action, ActionHandler, EmptyState, GridItem, GridView, ImageSource, ROption};

use super::colors::ExtensionViewColors;
use super::dimensions::*;
use super::ActionCallback;

// ============================================================================
// Actions
// ============================================================================

actions!(
    extension_grid,
    [SelectNext, SelectPrevious, SelectUp, SelectDown, Activate, Cancel]
);

/// Registers key bindings for the extension grid view.
pub fn register_key_bindings(cx: &mut gpui::AppContext) {
    cx.bind_keys([
        KeyBinding::new("right", SelectNext, Some("ExtensionGridView")),
        KeyBinding::new("left", SelectPrevious, Some("ExtensionGridView")),
        KeyBinding::new("down", SelectDown, Some("ExtensionGridView")),
        KeyBinding::new("up", SelectUp, Some("ExtensionGridView")),
        KeyBinding::new("enter", Activate, Some("ExtensionGridView")),
        KeyBinding::new("escape", Cancel, Some("ExtensionGridView")),
    ]);
}

// ============================================================================
// View State
// ============================================================================

/// Extension GridView state.
pub struct ExtensionGridView {
    /// The grid view data from the extension.
    grid_view: GridView,
    /// Currently selected item index.
    selected_index: usize,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Action callback for handling item actions.
    action_callback: Option<ActionCallback>,
}

impl ExtensionGridView {
    /// Creates a new extension grid view.
    pub fn new(
        grid_view: GridView,
        action_callback: Option<ActionCallback>,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        Self {
            grid_view,
            selected_index: 0,
            focus_handle,
            action_callback,
        }
    }

    /// Gets the number of columns.
    fn columns(&self) -> usize {
        (self.grid_view.columns as usize).clamp(2, 6)
    }

    /// Calculates the row count.
    fn row_count(&self) -> usize {
        let cols = self.columns();
        let items = self.grid_view.items.len();
        (items + cols - 1) / cols
    }

    /// Gets the row and column for an index.
    fn index_to_position(&self, index: usize) -> (usize, usize) {
        let cols = self.columns();
        (index / cols, index % cols)
    }

    /// Gets the index for a row and column.
    fn position_to_index(&self, row: usize, col: usize) -> Option<usize> {
        let cols = self.columns();
        let index = row * cols + col;
        if index < self.grid_view.items.len() {
            Some(index)
        } else {
            None
        }
    }

    /// Activates the selected item.
    fn activate_selected(&mut self, cx: &mut ViewContext<Self>) {
        let item = self.grid_view.items.get(self.selected_index).cloned();
        if let Some(item) = item {
            self.handle_item_activation(&item, cx);
        }
    }

    /// Handles activation of a grid item.
    fn handle_item_activation(&mut self, item: &GridItem, cx: &mut ViewContext<Self>) {
        if let Some(action) = item.actions.first() {
            self.execute_action(action, cx);
        }
    }

    /// Executes an action.
    fn execute_action(&mut self, action: &Action, cx: &mut ViewContext<Self>) {
        match &action.handler {
            ActionHandler::Callback => {
                if let Some(callback) = &self.action_callback {
                    callback(action.id.as_str(), cx);
                }
            },
            ActionHandler::OpenUrl(url) => {
                let url = url.to_string();
                let _ = open::that(&url);
            },
            ActionHandler::OpenFile(path) => {
                let path = path.to_string();
                let _ = open::that(&path);
            },
            ActionHandler::RevealInFinder(path) => {
                let path = path.to_string();
                let _ = std::process::Command::new("open")
                    .args(["-R", &path])
                    .spawn();
            },
            ActionHandler::QuickLook(path) => {
                let path = path.to_string();
                let _ = std::process::Command::new("qlmanage")
                    .args(["-p", &path])
                    .spawn();
            },
            ActionHandler::CopyToClipboard(text) => {
                let text = text.to_string();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            },
            ActionHandler::PushView(_view) => {
                // TODO: Implement view navigation
            },
            ActionHandler::SubmitForm => {
                // Not applicable for grid view
            },
        }
    }

    // ========================================================================
    // Action Handlers
    // ========================================================================

    fn select_next(&mut self, _: &SelectNext, cx: &mut ViewContext<Self>) {
        if !self.grid_view.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.grid_view.items.len();
            cx.notify();
        }
    }

    fn select_previous(&mut self, _: &SelectPrevious, cx: &mut ViewContext<Self>) {
        if !self.grid_view.items.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.grid_view.items.len() - 1
            } else {
                self.selected_index - 1
            };
            cx.notify();
        }
    }

    fn select_down(&mut self, _: &SelectDown, cx: &mut ViewContext<Self>) {
        if !self.grid_view.items.is_empty() {
            let (row, col) = self.index_to_position(self.selected_index);
            let next_row = row + 1;
            if let Some(new_index) = self.position_to_index(next_row, col) {
                self.selected_index = new_index;
                cx.notify();
            } else {
                // Wrap to first row
                if let Some(new_index) = self.position_to_index(0, col) {
                    self.selected_index = new_index;
                    cx.notify();
                }
            }
        }
    }

    fn select_up(&mut self, _: &SelectUp, cx: &mut ViewContext<Self>) {
        if !self.grid_view.items.is_empty() {
            let (row, col) = self.index_to_position(self.selected_index);
            if row > 0 {
                if let Some(new_index) = self.position_to_index(row - 1, col) {
                    self.selected_index = new_index;
                    cx.notify();
                }
            } else {
                // Wrap to last row
                let last_row = self.row_count().saturating_sub(1);
                if let Some(new_index) = self.position_to_index(last_row, col) {
                    self.selected_index = new_index;
                    cx.notify();
                } else {
                    // Column might not have items in last row, find last valid
                    for r in (0..=last_row).rev() {
                        if let Some(idx) = self.position_to_index(r, col) {
                            self.selected_index = idx;
                            cx.notify();
                            break;
                        }
                    }
                }
            }
        }
    }

    fn activate(&mut self, _: &Activate, cx: &mut ViewContext<Self>) {
        self.activate_selected(cx);
    }

    fn cancel(&mut self, _: &Cancel, cx: &mut ViewContext<Self>) {
        if let Some(callback) = &self.action_callback {
            callback("__cancel__", cx);
        }
    }

    // ========================================================================
    // Rendering
    // ========================================================================

    /// Renders a single grid item.
    fn render_grid_item(
        &self,
        item: &GridItem,
        _index: usize,
        is_selected: bool,
        colors: &ExtensionViewColors,
    ) -> impl IntoElement {
        div()
            .flex_1() // Distribute width evenly among columns
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(8.0))
            .rounded(BORDER_RADIUS)
            .cursor_pointer()
            .when(is_selected, |el| el.bg(colors.selection))
            .hover(|el| el.bg(colors.hover))
            // Image
            .child(
                div()
                    .w_full()
                    .h(GRID_ITEM_HEIGHT)
                    .rounded(px(6.0))
                    .overflow_hidden()
                    .bg(colors.surface)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_image(&item.image, colors)),
            )
            // Title
            .child(
                div()
                    .w_full()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(colors.text)
                    .truncate()
                    .child(item.title.to_string()),
            )
            // Subtitle
            .when_some(item.subtitle.clone().into_option(), |el, subtitle| {
                el.child(
                    div()
                        .w_full()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .truncate()
                        .child(subtitle.to_string()),
                )
            })
    }

    /// Renders an image from various sources.
    fn render_image(&self, source: &ImageSource, colors: &ExtensionViewColors) -> impl IntoElement {
        match source {
            ImageSource::Path(path) => {
                // Use PathBuf to load from filesystem (not as an asset)
                let file_path = std::path::PathBuf::from(path.to_string());
                div().child(
                    img(file_path)
                        .w(px(80.0))
                        .h(px(80.0))
                        .object_fit(gpui::ObjectFit::Contain),
                )
            },
            ImageSource::Url(_url) => {
                // For URLs, we would need async image loading
                // For now, show a placeholder
                div()
                    .w(px(80.0))
                    .h(px(80.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(colors.surface_hover)
                    .rounded(px(4.0))
                    .text_color(colors.text_muted)
                    .text_2xl()
                    .child("🌐")
            },
            ImageSource::Base64 { data: _, mime_type: _ } => {
                // Base64 images need to be decoded
                // For now, show placeholder
                div()
                    .w(px(80.0))
                    .h(px(80.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(colors.surface_hover)
                    .rounded(px(4.0))
                    .text_color(colors.text_muted)
                    .text_2xl()
                    .child("🖼️")
            },
            ImageSource::SfSymbol(name) => {
                // Map SF Symbol names to emojis
                let emoji = Self::sf_symbol_to_emoji(name.as_str());
                div()
                    .w(px(80.0))
                    .h(px(80.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_2xl()
                    .child(emoji)
            },
        }
    }

    /// Maps SF Symbol names to emojis.
    fn sf_symbol_to_emoji(name: &str) -> &'static str {
        match name {
            "photo" | "photo.fill" => "🖼️",
            "doc" | "doc.fill" => "📄",
            "folder" | "folder.fill" => "📁",
            "star" | "star.fill" => "⭐",
            "heart" | "heart.fill" => "❤️",
            "bookmark" | "bookmark.fill" => "🔖",
            "tag" | "tag.fill" => "🏷️",
            "music.note" => "🎵",
            "video" | "video.fill" => "🎬",
            "person" | "person.fill" => "👤",
            "gear" | "gearshape" => "⚙️",
            "link" => "🔗",
            "globe" => "🌐",
            "cloud" | "cloud.fill" => "☁️",
            "lock" | "lock.fill" => "🔒",
            "key" | "key.fill" => "🔑",
            "house" | "house.fill" => "🏠",
            "cart" | "cart.fill" => "🛒",
            "creditcard" | "creditcard.fill" => "💳",
            "calendar" => "📅",
            "clock" | "clock.fill" => "🕐",
            "bell" | "bell.fill" => "🔔",
            "envelope" | "envelope.fill" => "✉️",
            "phone" | "phone.fill" => "📱",
            "bubble.left" | "bubble.left.fill" => "💬",
            _ => "📋",
        }
    }

    /// Renders the empty state.
    fn render_empty_state(
        &self,
        empty_state: &EmptyState,
        colors: &ExtensionViewColors,
    ) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .p(PADDING)
            // Icon
            .when_some(empty_state.icon.clone().into_option(), |el, icon| {
                el.child(
                    div()
                        .text_2xl()
                        .mb(px(8.0))
                        .child(match &icon {
                            photoncast_extension_api::IconSource::Emoji { glyph } => {
                                glyph.to_string()
                            },
                            photoncast_extension_api::IconSource::SystemIcon { name } => {
                                Self::sf_symbol_to_emoji(name.as_str()).to_string()
                            },
                            _ => "📋".to_string(),
                        }),
                )
            })
            // Title
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child(empty_state.title.to_string()),
            )
            // Description
            .when_some(empty_state.description.clone().into_option(), |el, desc| {
                el.child(
                    div()
                        .text_sm()
                        .text_color(colors.text_muted)
                        .child(desc.to_string()),
                )
            })
    }

    /// Renders the grid content.
    fn render_grid_content(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        let cols = self.columns();
        let items = &self.grid_view.items;

        // Group items into rows
        let rows: Vec<Vec<(usize, &GridItem)>> = items
            .iter()
            .enumerate()
            .collect::<Vec<_>>()
            .chunks(cols)
            .map(|chunk| chunk.to_vec())
            .collect();

        div()
            .id("grid-content")
            .flex_1()
            .overflow_y_scroll()
            .p(PADDING)
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(rows.into_iter().map(|row| {
                div()
                    .w_full()
                    .flex()
                    .gap(px(8.0))
                    .children(row.into_iter().map(|(idx, item)| {
                        let is_selected = idx == self.selected_index;
                        self.render_grid_item(item, idx, is_selected, colors)
                            .into_any_element()
                    }))
                    .into_any_element()
            }))
    }
}

impl FocusableView for ExtensionGridView {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ExtensionGridView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = ExtensionViewColors::from_context(cx);
        let is_empty = self.grid_view.items.is_empty();
        let has_empty_state = self.grid_view.empty_state.is_some();

        div()
            .key_context("ExtensionGridView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::cancel))
            .size_full() // Fill parent container
            .flex()
            .flex_col()
            .bg(colors.background)
            .overflow_hidden()
            // Title
            .child(
                div()
                    .h(px(44.0))
                    .px(PADDING)
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border)
                    .text_base()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child(self.grid_view.title.to_string()),
            )
            // Content
            .child(if is_empty && has_empty_state {
                if let ROption::RSome(ref empty_state) = self.grid_view.empty_state {
                    self.render_empty_state(empty_state, &colors)
                        .into_any_element()
                } else {
                    self.render_grid_content(&colors).into_any_element()
                }
            } else {
                self.render_grid_content(&colors).into_any_element()
            })
    }
}
