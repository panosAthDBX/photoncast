//! Argument input UI for quick links with placeholders.
//!
//! When a quick link contains `{argument}` placeholders, this view presents
//! a modal dialog for the user to input the required values before opening.

use std::collections::HashMap;

use gpui::prelude::*;
use gpui::{
    div, px, FocusHandle, FocusableView, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Render, StatefulInteractiveElement, Styled, ViewContext,
};
use photoncast_theme::{GpuiThemeColors, PhotonTheme};

use crate::models::{QuickLink, QuickLinkIcon};
use crate::placeholder::{extract_required_arguments, substitute_placeholders, ArgumentInfo};

/// Events emitted by the argument input view.
#[derive(Debug, Clone)]
pub enum ArgumentInputEvent {
    /// User submitted the arguments - contains the quicklink and final URL.
    Submitted {
        /// The quicklink being opened (boxed to reduce enum size).
        quicklink: Box<QuickLink>,
        /// The final URL with all placeholders substituted.
        final_url: String,
    },
    /// User cancelled the input.
    Cancelled,
}

/// A single argument field in the input form.
#[derive(Debug, Clone)]
struct ArgumentField {
    /// Information about this argument from placeholder parsing.
    info: ArgumentInfo,
    /// Current value entered by the user.
    value: String,
    /// Cursor position in the value.
    cursor: usize,
    /// Validation error message (if any).
    error: Option<String>,
    /// Currently selected dropdown index (for options).
    dropdown_index: usize,
    /// Whether dropdown is expanded.
    dropdown_open: bool,
}

impl ArgumentField {
    fn new(info: ArgumentInfo) -> Self {
        let default_value = info.default.clone().unwrap_or_default();
        let cursor = default_value.chars().count();
        let dropdown_index = if info.options.is_empty() {
            0
        } else {
            // Find index of default value in options, or 0
            info.options
                .iter()
                .position(|o| o == &default_value)
                .unwrap_or(0)
        };

        Self {
            value: default_value,
            cursor,
            info,
            error: None,
            dropdown_index,
            dropdown_open: false,
        }
    }

    /// Returns the label for this field.
    fn label(&self) -> &str {
        self.info.name.as_deref().unwrap_or("Query")
    }

    /// Returns true if this is a dropdown field.
    fn is_dropdown(&self) -> bool {
        !self.info.options.is_empty()
    }

    /// Returns true if the field has a value (or is optional with default).
    fn is_valid(&self) -> bool {
        !self.value.is_empty() || self.info.default.is_some()
    }

    /// Gets the current value, selecting from dropdown if needed.
    fn get_value(&self) -> String {
        if self.is_dropdown() && !self.info.options.is_empty() {
            self.info
                .options
                .get(self.dropdown_index)
                .cloned()
                .unwrap_or_else(|| self.value.clone())
        } else {
            self.value.clone()
        }
    }
}

/// Type alias – argument view uses the shared [`GpuiThemeColors`] from photoncast-theme.
type ArgumentInputColors = GpuiThemeColors;

fn get_argument_colors<V: 'static>(cx: &ViewContext<V>) -> ArgumentInputColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    GpuiThemeColors::from_theme(&theme)
}

/// Callback type for events.
type EventCallback = Box<dyn Fn(ArgumentInputEvent, &mut ViewContext<ArgumentInputView>) + 'static>;

/// Argument input view for quick links with placeholders.
///
/// Displays a modal dialog with input fields for each required argument,
/// a URL preview, and submit/cancel buttons.
pub struct ArgumentInputView {
    /// The quick link being configured.
    quicklink: QuickLink,
    /// Input fields for each required argument.
    arguments: Vec<ArgumentField>,
    /// Currently focused field index.
    focus_index: usize,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Event callback.
    on_event: Option<EventCallback>,
}

impl ArgumentInputView {
    /// Creates a new argument input view for a quick link.
    pub fn new(quicklink: QuickLink, cx: &mut ViewContext<Self>) -> Self {
        let args = extract_required_arguments(&quicklink.link);
        let arguments: Vec<ArgumentField> = args.into_iter().map(ArgumentField::new).collect();

        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        Self {
            quicklink,
            arguments,
            focus_index: 0,
            focus_handle,
            on_event: None,
        }
    }

    /// Sets the event callback.
    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(ArgumentInputEvent, &mut ViewContext<Self>) + 'static,
    {
        self.on_event = Some(Box::new(callback));
    }

    /// Returns true if all required fields are filled.
    fn all_fields_valid(&self) -> bool {
        self.arguments.iter().all(ArgumentField::is_valid)
    }

    /// Gets the final URL with all placeholders substituted.
    pub fn get_final_url(&self) -> Result<String, crate::placeholder::PlaceholderError> {
        let mut map = HashMap::new();

        for (i, field) in self.arguments.iter().enumerate() {
            let key = field.info.name.clone().unwrap_or_else(|| i.to_string());
            map.insert(key, field.get_value());
        }

        substitute_placeholders(&self.quicklink.link, &map, None, None)
    }

    /// Gets a preview URL (with error handling for display).
    fn get_preview_url(&self) -> String {
        self.get_final_url()
            .unwrap_or_else(|_| self.quicklink.link.clone())
    }

    /// Submits the form if valid.
    fn submit(&mut self, cx: &mut ViewContext<Self>) {
        if !self.all_fields_valid() {
            // Mark empty required fields with errors
            for field in &mut self.arguments {
                if !field.is_valid() {
                    field.error = Some("This field is required".to_string());
                }
            }
            cx.notify();
            return;
        }

        if let Ok(final_url) = self.get_final_url() {
            if let Some(callback) = &self.on_event {
                let event = ArgumentInputEvent::Submitted {
                    quicklink: Box::new(self.quicklink.clone()),
                    final_url,
                };
                callback(event, cx);
            }
        }
    }

    /// Cancels and closes the dialog.
    fn cancel(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(callback) = &self.on_event {
            callback(ArgumentInputEvent::Cancelled, cx);
        }
    }

    /// Moves focus to the next field.
    fn focus_next(&mut self, cx: &mut ViewContext<Self>) {
        // Close any open dropdowns
        if let Some(field) = self.arguments.get_mut(self.focus_index) {
            field.dropdown_open = false;
        }

        if self.focus_index + 1 < self.arguments.len() {
            self.focus_index += 1;
        }
        cx.notify();
    }

    /// Moves focus to the previous field.
    fn focus_previous(&mut self, cx: &mut ViewContext<Self>) {
        // Close any open dropdowns
        if let Some(field) = self.arguments.get_mut(self.focus_index) {
            field.dropdown_open = false;
        }

        if self.focus_index > 0 {
            self.focus_index -= 1;
        }
        cx.notify();
    }

    /// Selects next option in dropdown (if current field is dropdown).
    fn dropdown_next(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(field) = self.arguments.get_mut(self.focus_index) {
            if field.is_dropdown() && !field.info.options.is_empty() {
                field.dropdown_open = true;
                if field.dropdown_index + 1 < field.info.options.len() {
                    field.dropdown_index += 1;
                    field.value = field.info.options[field.dropdown_index].clone();
                }
                cx.notify();
            }
        }
    }

    /// Selects previous option in dropdown (if current field is dropdown).
    fn dropdown_previous(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(field) = self.arguments.get_mut(self.focus_index) {
            if field.is_dropdown() && !field.info.options.is_empty() {
                field.dropdown_open = true;
                if field.dropdown_index > 0 {
                    field.dropdown_index -= 1;
                    field.value = field.info.options[field.dropdown_index].clone();
                }
                cx.notify();
            }
        }
    }

    /// Toggles dropdown open state.
    fn toggle_dropdown(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(field) = self.arguments.get_mut(self.focus_index) {
            if field.is_dropdown() {
                field.dropdown_open = !field.dropdown_open;
                cx.notify();
            }
        }
    }

    /// Handles key down events.
    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        let key = event.keystroke.key.as_str();
        let shift = event.keystroke.modifiers.shift;
        let platform = event.keystroke.modifiers.platform;

        match key {
            "escape" => {
                self.cancel(cx);
            },
            "enter" => {
                // If dropdown is open, close it; otherwise submit
                if let Some(field) = self.arguments.get_mut(self.focus_index) {
                    if field.dropdown_open {
                        field.dropdown_open = false;
                        cx.notify();
                        return;
                    }
                }
                self.submit(cx);
            },
            "tab" => {
                if shift {
                    self.focus_previous(cx);
                } else {
                    self.focus_next(cx);
                }
            },
            "up" => {
                self.dropdown_previous(cx);
            },
            "down" => {
                self.dropdown_next(cx);
            },
            "space" => {
                // Toggle dropdown for dropdown fields
                if let Some(field) = self.arguments.get(self.focus_index) {
                    if field.is_dropdown() {
                        self.toggle_dropdown(cx);
                    }
                }
            },
            "left" => {
                // Cursor movement for text fields
                if let Some(field) = self.arguments.get_mut(self.focus_index) {
                    if !field.is_dropdown() {
                        if platform {
                            field.cursor = 0;
                        } else if field.cursor > 0 {
                            field.cursor -= 1;
                        }
                        cx.notify();
                    }
                }
            },
            "right" => {
                // Cursor movement for text fields
                if let Some(field) = self.arguments.get_mut(self.focus_index) {
                    if !field.is_dropdown() {
                        let len = field.value.chars().count();
                        if platform {
                            field.cursor = len;
                        } else if field.cursor < len {
                            field.cursor += 1;
                        }
                        cx.notify();
                    }
                }
            },
            "backspace" => {
                // Handle backspace for text input
                if let Some(field) = self.arguments.get_mut(self.focus_index) {
                    if !field.is_dropdown() {
                        if platform {
                            // Delete everything before cursor
                            if field.cursor > 0 {
                                let chars: Vec<char> = field.value.chars().collect();
                                field.value = chars[field.cursor..].iter().collect();
                                field.cursor = 0;
                                field.error = None;
                                cx.notify();
                            }
                        } else if field.cursor > 0 {
                            // Delete char before cursor
                            let mut chars: Vec<char> = field.value.chars().collect();
                            chars.remove(field.cursor - 1);
                            field.cursor -= 1;
                            field.value = chars.into_iter().collect();
                            field.error = None;
                            cx.notify();
                        }
                    }
                }
            },
            "v" if platform => {
                // Handle Cmd+V paste
                if let Some(field) = self.arguments.get_mut(self.focus_index) {
                    if !field.is_dropdown() {
                        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                            let chars: Vec<char> = field.value.chars().collect();
                            let before: String = chars[..field.cursor].iter().collect();
                            let after: String = chars[field.cursor..].iter().collect();
                            field.value = format!("{}{}{}", before, text, after);
                            field.cursor += text.chars().count();
                            field.error = None;
                            cx.notify();
                        }
                    }
                }
            },
            _ => {
                // Handle character input for text fields at cursor position
                // Ignore modifier-only keys
                if key.len() == 1
                    && !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                {
                    if let Some(field) = self.arguments.get_mut(self.focus_index) {
                        if !field.is_dropdown() {
                            let chars: Vec<char> = field.value.chars().collect();
                            let before: String = chars[..field.cursor].iter().collect();
                            let after: String = chars[field.cursor..].iter().collect();
                            field.value = format!("{}{}{}", before, key, after);
                            field.cursor += key.chars().count();
                            field.error = None;
                            cx.notify();
                        }
                    }
                }
            },
        }
    }

    /// Returns the icon for this quicklink.
    fn get_icon_display(&self) -> &'static str {
        match &self.quicklink.icon {
            QuickLinkIcon::Favicon(_) => "🌐",
            QuickLinkIcon::Emoji(_)
            | QuickLinkIcon::SystemIcon(_)
            | QuickLinkIcon::CustomImage(_)
            | QuickLinkIcon::Default => "🔗",
        }
    }

    /// Renders a text input field.
    fn render_text_field(
        index: usize,
        field: &ArgumentField,
        is_focused: bool,
        colors: &ArgumentInputColors,
    ) -> impl IntoElement {
        let has_error = field.error.is_some();
        let border_color = if has_error {
            colors.error
        } else if is_focused {
            colors.border_focused
        } else {
            colors.border
        };

        let value = field.value.clone();
        let placeholder = field
            .info
            .default
            .as_ref()
            .map_or_else(|| format!("Enter {}", field.label()), Clone::clone);

        // Block cursor dimensions (matches launcher)
        let cursor_width = px(9.0);
        let cursor_height = px(20.0);

        // Split text at cursor position
        let chars: Vec<char> = value.chars().collect();
        let cursor_pos = field.cursor.min(chars.len());
        let before: String = chars[..cursor_pos].iter().collect();
        let after: String = chars[cursor_pos..].iter().collect();
        let has_value = !value.is_empty();

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                // Label
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(colors.text)
                    .child(field.label().to_string()),
            )
            .child(
                // Input field
                div()
                    .id(("field", index))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded_md()
                    .border_1()
                    .border_color(border_color)
                    .bg(colors.surface)
                    .when(is_focused, |el| el.bg(colors.surface_hover))
                    .child(
                        div()
                            .text_sm()
                            .flex()
                            .items_center()
                            .when(!has_value && is_focused, |el| {
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
                                        .child(placeholder.clone()),
                                )
                            })
                            .when(!has_value && !is_focused, |el| {
                                el.text_color(colors.text_placeholder)
                                    .child(placeholder.clone())
                            })
                            .when(has_value, |el| {
                                el.text_color(colors.text)
                                    .when(!before.is_empty(), |el| el.child(before.clone()))
                                    .when(is_focused, |el| {
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
                    ),
            )
            .when_some(field.error.clone(), |el, error| {
                el.child(div().text_xs().text_color(colors.error).child(error))
            })
    }

    /// Renders a dropdown field.
    fn render_dropdown_field(
        index: usize,
        field: &ArgumentField,
        is_focused: bool,
        colors: &ArgumentInputColors,
    ) -> impl IntoElement {
        let border_color = if is_focused {
            colors.border_focused
        } else {
            colors.border
        };

        let current_value = field
            .info
            .options
            .get(field.dropdown_index)
            .cloned()
            .unwrap_or_default();

        let dropdown_open = field.dropdown_open && is_focused;
        let options = field.info.options.clone();
        let selected_index = field.dropdown_index;

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                // Label
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(colors.text)
                    .child(field.label().to_string()),
            )
            .child(
                // Dropdown trigger
                div()
                    .id(("dropdown", index))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded_md()
                    .border_1()
                    .border_color(border_color)
                    .bg(colors.surface)
                    .when(is_focused, |el| el.bg(colors.surface_hover))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm().text_color(colors.text).child(current_value))
                    .child(
                        // Dropdown arrow
                        div()
                            .text_xs()
                            .text_color(colors.text_muted)
                            .child(if dropdown_open { "▲" } else { "▼" }),
                    ),
            )
            .when(dropdown_open, |el| {
                // Dropdown options
                let selection_color = colors.selection;
                let surface_hover = colors.surface_hover;
                let text_color = colors.text;
                let surface_color = colors.surface;

                el.child(
                    div()
                        .mt(px(4.0))
                        .rounded_md()
                        .border_1()
                        .border_color(border_color)
                        .bg(surface_color)
                        .overflow_hidden()
                        .children(options.into_iter().enumerate().map(move |(i, option)| {
                            let is_selected = i == selected_index;
                            div()
                                .px(px(12.0))
                                .py(px(6.0))
                                .bg(if is_selected {
                                    selection_color
                                } else {
                                    surface_color
                                })
                                .hover(|el| el.bg(surface_hover))
                                .text_sm()
                                .text_color(text_color)
                                .child(option)
                        })),
                )
            })
    }
}

impl FocusableView for ArgumentInputView {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ArgumentInputView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_argument_colors(cx);
        let icon = self.get_icon_display();
        let name = self.quicklink.name.clone();
        let preview_url = self.get_preview_url();
        let can_submit = self.all_fields_valid();
        let focus_index = self.focus_index;

        let accent_color = colors.accent;
        let accent_hover = colors.accent_hover;
        let surface_color = colors.surface;
        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let background = colors.background;

        // Render fields
        let fields: Vec<_> = self
            .arguments
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let is_focused = i == focus_index;
                if field.is_dropdown() {
                    Self::render_dropdown_field(i, field, is_focused, &colors).into_any_element()
                } else {
                    Self::render_text_field(i, field, is_focused, &colors).into_any_element()
                }
            })
            .collect();

        div()
            .track_focus(&self.focus_handle)
            .key_context("ArgumentInput")
            .on_key_down(cx.listener(Self::handle_key_down))
            .p(px(20.0))
            .rounded_xl()
            .bg(background)
            .border_1()
            .border_color(colors.border)
            .w(px(400.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Header with icon and name
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .w(px(40.0))
                            .h(px(40.0))
                            .rounded_lg()
                            .bg(surface_color)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xl()
                            .child(icon),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(text_color)
                                    .child(name),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child("Enter values for placeholders"),
                            ),
                    ),
            )
            // Input fields
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .children(fields),
            )
            // URL Preview
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(text_muted)
                            .child("Preview"),
                    )
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded_md()
                            .bg(surface_color)
                            .text_xs()
                            .text_color(text_muted)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(preview_url),
                    ),
            )
            // Buttons
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        // Cancel button
                        div()
                            .id("cancel-btn")
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded_md()
                            .bg(surface_color)
                            .hover(|el| el.bg(surface_hover))
                            .cursor_pointer()
                            .text_sm()
                            .text_color(text_color)
                            .child("Cancel")
                            .on_click(cx.listener(|this, _, cx| {
                                this.cancel(cx);
                            })),
                    )
                    .child(
                        // Open button
                        div()
                            .id("submit-btn")
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded_md()
                            .bg(if can_submit {
                                accent_color
                            } else {
                                surface_color
                            })
                            .when(can_submit, |el| el.hover(|el| el.bg(accent_hover)))
                            .cursor(if can_submit {
                                gpui::CursorStyle::PointingHand
                            } else {
                                gpui::CursorStyle::Arrow
                            })
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if can_submit {
                                background
                            } else {
                                text_muted
                            })
                            .child("Open")
                            .when(can_submit, |el| {
                                el.on_click(cx.listener(|this, _, cx| {
                                    this.submit(cx);
                                }))
                            }),
                    ),
            )
            // Keyboard hints
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(16.0))
                    .pt(px(8.0))
                    .border_t_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_xs()
                            .text_color(text_muted)
                            .child("Tab")
                            .child("→")
                            .child("Next field"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_xs()
                            .text_color(text_muted)
                            .child("Enter")
                            .child("→")
                            .child("Open"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_xs()
                            .text_color(text_muted)
                            .child("Esc")
                            .child("→")
                            .child("Cancel"),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_field_new() {
        let info = ArgumentInfo {
            name: Some("query".to_string()),
            default: Some("test".to_string()),
            options: vec![],
        };
        let field = ArgumentField::new(info);
        assert_eq!(field.value, "test");
        assert_eq!(field.label(), "query");
        assert!(!field.is_dropdown());
        assert!(field.is_valid());
    }

    #[test]
    fn test_argument_field_dropdown() {
        let info = ArgumentInfo {
            name: Some("lang".to_string()),
            default: Some("en".to_string()),
            options: vec!["en".to_string(), "es".to_string(), "fr".to_string()],
        };
        let field = ArgumentField::new(info);
        assert!(field.is_dropdown());
        assert_eq!(field.dropdown_index, 0);
        assert_eq!(field.get_value(), "en");
    }

    #[test]
    fn test_argument_field_required() {
        let info = ArgumentInfo {
            name: None,
            default: None,
            options: vec![],
        };
        let field = ArgumentField::new(info);
        assert!(!field.is_valid());
        assert_eq!(field.label(), "Query");
    }
}
