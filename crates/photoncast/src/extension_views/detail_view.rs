//! DetailView rendering for extensions.
//!
//! Renders `DetailView` types with:
//! - Markdown content with host styling
//! - Metadata items with label/value pairs
//! - Clickable links in metadata
//! - Tags with semantic colors
//! - Action buttons

use gpui::prelude::FluentBuilder;
use gpui::*;
use photoncast_extension_api::{Action, ActionHandler, DetailView, MetadataItem, MetadataValue};

use super::colors::ExtensionViewColors;
use super::dimensions::*;
use super::ActionCallback;

// ============================================================================
// Actions
// ============================================================================

actions!(extension_detail, [Activate, Cancel, NextAction, PreviousAction]);

/// Registers key bindings for the extension detail view.
pub fn register_key_bindings(cx: &mut gpui::AppContext) {
    cx.bind_keys([
        KeyBinding::new("enter", Activate, Some("ExtensionDetailView")),
        KeyBinding::new("escape", Cancel, Some("ExtensionDetailView")),
        KeyBinding::new("tab", NextAction, Some("ExtensionDetailView")),
        KeyBinding::new("shift-tab", PreviousAction, Some("ExtensionDetailView")),
    ]);
}

// ============================================================================
// View State
// ============================================================================

/// Extension DetailView state.
pub struct ExtensionDetailView {
    /// The detail view data from the extension.
    detail_view: DetailView,
    /// Currently selected action index.
    selected_action_index: usize,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Action callback for handling actions.
    action_callback: Option<ActionCallback>,
}

impl ExtensionDetailView {
    /// Creates a new extension detail view.
    pub fn new(
        detail_view: DetailView,
        action_callback: Option<ActionCallback>,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        Self {
            detail_view,
            selected_action_index: 0,
            focus_handle,
            action_callback,
        }
    }

    /// Executes an action.
    fn execute_action(&mut self, action: &Action, cx: &mut ViewContext<Self>) {
        // Track if this action should close the extension view
        let mut should_close = false;
        
        match &action.handler {
            ActionHandler::Callback => {
                if let Some(callback) = &self.action_callback {
                    callback(action.id.as_str(), cx);
                }
            },
            ActionHandler::OpenUrl(url) => {
                let url = url.to_string();
                let _ = open::that(&url);
                should_close = true;
            },
            ActionHandler::OpenFile(path) => {
                let path = path.to_string();
                let _ = open::that(&path);
                should_close = true;
            },
            ActionHandler::RevealInFinder(path) => {
                let path = path.to_string();
                let _ = std::process::Command::new("open")
                    .args(["-R", &path])
                    .spawn();
                should_close = true;
            },
            ActionHandler::QuickLook(path) => {
                let path = path.to_string();
                let _ = std::process::Command::new("qlmanage")
                    .args(["-p", &path])
                    .spawn();
                // Don't close for QuickLook - user may want to continue browsing
            },
            ActionHandler::CopyToClipboard(text) => {
                let text = text.to_string();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                should_close = true;
            },
            ActionHandler::PushView(_view) => {
                // TODO: Implement view navigation
            },
            ActionHandler::SubmitForm => {
                // Not applicable for detail view
            },
        }
        
        // Close the extension view for terminal actions
        if should_close {
            if let Some(callback) = &self.action_callback {
                callback("__cancel__", cx);
            }
        }
    }

    // ========================================================================
    // Action Handlers
    // ========================================================================

    fn activate(&mut self, _: &Activate, cx: &mut ViewContext<Self>) {
        let action = self.detail_view.actions.get(self.selected_action_index).cloned();
        if let Some(action) = action {
            self.execute_action(&action, cx);
        }
    }

    fn cancel(&mut self, _: &Cancel, cx: &mut ViewContext<Self>) {
        if let Some(callback) = &self.action_callback {
            callback("__cancel__", cx);
        }
    }

    fn next_action(&mut self, _: &NextAction, cx: &mut ViewContext<Self>) {
        if !self.detail_view.actions.is_empty() {
            self.selected_action_index =
                (self.selected_action_index + 1) % self.detail_view.actions.len();
            cx.notify();
        }
    }

    fn previous_action(&mut self, _: &PreviousAction, cx: &mut ViewContext<Self>) {
        if !self.detail_view.actions.is_empty() {
            self.selected_action_index = if self.selected_action_index == 0 {
                self.detail_view.actions.len() - 1
            } else {
                self.selected_action_index - 1
            };
            cx.notify();
        }
    }

    // ========================================================================
    // Rendering
    // ========================================================================

    /// Renders the markdown content.
    fn render_markdown(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        let markdown = self.detail_view.markdown.as_str();
        let mut elements: Vec<gpui::AnyElement> = Vec::new();
        let lines = markdown.lines();

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                elements.push(div().h(px(8.0)).into_any_element());
            } else if trimmed.starts_with("# ") {
                elements.push(
                    div()
                        .text_xl()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(colors.text)
                        .mb(px(12.0))
                        .child(trimmed[2..].to_string())
                        .into_any_element(),
                );
            } else if trimmed.starts_with("## ") {
                elements.push(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(colors.text)
                        .mb(px(8.0))
                        .child(trimmed[3..].to_string())
                        .into_any_element(),
                );
            } else if trimmed.starts_with("### ") {
                elements.push(
                    div()
                        .text_base()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(colors.text)
                        .mb(px(6.0))
                        .child(trimmed[4..].to_string())
                        .into_any_element(),
                );
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                elements.push(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(8.0))
                        .mb(px(4.0))
                        .child(div().text_color(colors.text_muted).child("•"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(colors.text)
                                .child(trimmed[2..].to_string()),
                        )
                        .into_any_element(),
                );
            } else if trimmed.starts_with("> ") {
                elements.push(
                    div()
                        .pl(px(12.0))
                        .border_l_2()
                        .border_color(colors.accent)
                        .text_sm()
                        .italic()
                        .text_color(colors.text_muted)
                        .mb(px(8.0))
                        .child(trimmed[2..].to_string())
                        .into_any_element(),
                );
            } else if trimmed.starts_with("```") {
                // Skip code block markers
            } else {
                elements.push(
                    div()
                        .text_sm()
                        .text_color(colors.text)
                        .mb(px(8.0))
                        .child(line.to_string())
                        .into_any_element(),
                );
            }
        }

        div()
            .id("detail-content")
            .flex_1()
            .overflow_y_scroll()
            .p(PADDING)
            .child(div().flex().flex_col().children(elements))
    }

    /// Renders the metadata section.
    fn render_metadata(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        if self.detail_view.metadata.is_empty() {
            return div();
        }

        div()
            .w_full()
            .p(PADDING)
            .border_t_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(colors.text_muted)
                    .mb(px(8.0))
                    .child("METADATA"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .children(self.detail_view.metadata.iter().map(|item| {
                        self.render_metadata_item(item, colors).into_any_element()
                    })),
            )
    }

    /// Renders a single metadata item.
    fn render_metadata_item(
        &self,
        item: &MetadataItem,
        colors: &ExtensionViewColors,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_start()
            .gap(px(12.0))
            .child(
                div()
                    .w(px(120.0))
                    .flex_shrink_0()
                    .text_xs()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(colors.text_muted)
                    .child(item.label.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .child(self.render_metadata_value(&item.value, colors)),
            )
    }

    /// Renders a metadata value.
    fn render_metadata_value(
        &self,
        value: &MetadataValue,
        colors: &ExtensionViewColors,
    ) -> impl IntoElement {
        match value {
            MetadataValue::Text(text) => div()
                .text_sm()
                .text_color(colors.text)
                .child(text.to_string()),
            MetadataValue::Link { text, url } => {
                let url_for_click = url.to_string();
                div()
                    .text_sm()
                    .text_color(colors.accent)
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, move |_, _cx| {
                        let _ = open::that(&url_for_click);
                    })
                    .child(text.to_string())
            },
            MetadataValue::Date(duration) => {
                let secs = duration.as_secs();
                let text = Self::format_date(secs);
                div().text_sm().text_color(colors.text).child(text)
            },
            MetadataValue::Tag { text, color } => div()
                .px(px(6.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .text_xs()
                .font_weight(gpui::FontWeight::MEDIUM)
                .bg(colors.tag_background(color))
                .text_color(colors.tag_color(color))
                .child(text.to_string()),
        }
    }

    /// Formats a timestamp as a date string.
    fn format_date(secs: u64) -> String {
        use std::time::{Duration, UNIX_EPOCH};
        let datetime = UNIX_EPOCH + Duration::from_secs(secs);
        // Simple format - in production, use chrono
        format!("{:?}", datetime)
    }

    /// Renders the action buttons.
    fn render_actions(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        if self.detail_view.actions.is_empty() {
            return div();
        }

        div()
            .w_full()
            .p(PADDING)
            .border_t_1()
            .border_color(colors.border)
            .flex()
            .gap(px(8.0))
            .children(
                self.detail_view
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(idx, action)| {
                        let is_selected = idx == self.selected_action_index;
                        self.render_action_button(action, is_selected, colors)
                            .into_any_element()
                    }),
            )
    }

    /// Renders an action button.
    fn render_action_button(
        &self,
        action: &Action,
        is_selected: bool,
        colors: &ExtensionViewColors,
    ) -> impl IntoElement {
        let is_destructive =
            matches!(action.style, photoncast_extension_api::ActionStyle::Destructive);
        let is_primary = matches!(action.style, photoncast_extension_api::ActionStyle::Primary);

        let bg_color = if is_primary {
            colors.accent
        } else if is_destructive {
            colors.error
        } else {
            colors.surface
        };

        let text_color = if is_primary || is_destructive {
            gpui::white()
        } else {
            colors.text
        };

        div()
            .px(px(16.0))
            .py(px(8.0))
            .rounded(BORDER_RADIUS)
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .cursor_pointer()
            .bg(bg_color)
            .text_color(text_color)
            .when(is_selected, |el| {
                el.border_2().border_color(colors.accent)
            })
            .hover(|el| {
                if is_primary {
                    el.bg(colors.accent_hover)
                } else if is_destructive {
                    el.bg(colors.error)
                } else {
                    el.bg(colors.surface_hover)
                }
            })
            .child(action.title.to_string())
    }
}

impl FocusableView for ExtensionDetailView {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ExtensionDetailView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = ExtensionViewColors::from_context(cx);

        div()
            .key_context("ExtensionDetailView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::next_action))
            .on_action(cx.listener(Self::previous_action))
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
                    .child(self.detail_view.title.to_string()),
            )
            // Markdown content
            .child(self.render_markdown(&colors))
            // Metadata
            .child(self.render_metadata(&colors))
            // Actions
            .child(self.render_actions(&colors))
    }
}
