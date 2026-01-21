//! Create/Edit Quicklink view.
//!
//! Provides a form UI for creating and editing quick links with validation,
//! keyboard navigation, and theme-aware styling.

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{
    div, px, rems, AppContext, FocusHandle, FocusableView, FontWeight, Hsla, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString, Styled, ViewContext,
};
use photoncast_theme::PhotonTheme;

use crate::models::{QuickLink, QuickLinkIcon, QuickLinkId};

// ============================================================================
// Events
// ============================================================================

/// Events emitted by the CreateQuicklinkView.
#[derive(Debug, Clone)]
pub enum CreateQuicklinkEvent {
    /// A new quicklink was created.
    Created(QuickLink),
    /// An existing quicklink was updated.
    Updated(QuickLink),
    /// The user cancelled the operation.
    Cancelled,
}

// ============================================================================
// Types
// ============================================================================

/// Information about an application for the "Open With" picker.
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// Display name of the application.
    pub name: String,
    /// Bundle ID (e.g., "com.apple.Safari").
    pub bundle_id: String,
    /// Path to the app icon (optional).
    pub icon: Option<PathBuf>,
}

impl AppInfo {
    /// Creates a new AppInfo.
    #[must_use]
    pub fn new(name: impl Into<String>, bundle_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bundle_id: bundle_id.into(),
            icon: None,
        }
    }

    /// Creates a new AppInfo with an icon path.
    #[must_use]
    pub fn with_icon(mut self, icon: PathBuf) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// Focus state for the create quicklink form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreateQuicklinkFocus {
    /// Name input field.
    #[default]
    Name,
    /// Link/URL input field.
    Link,
    /// Alias input field.
    Alias,
    /// Application picker dropdown.
    AppPicker,
    /// Icon picker.
    IconPicker,
}

impl CreateQuicklinkFocus {
    /// Returns the next focus state (Tab).
    fn next(self) -> Self {
        match self {
            Self::Name => Self::Link,
            Self::Link => Self::Alias,
            Self::Alias => Self::AppPicker,
            Self::AppPicker => Self::IconPicker,
            Self::IconPicker => Self::Name,
        }
    }

    /// Returns the previous focus state (Shift+Tab).
    fn previous(self) -> Self {
        match self {
            Self::Name => Self::IconPicker,
            Self::Link => Self::Name,
            Self::Alias => Self::Link,
            Self::AppPicker => Self::Alias,
            Self::IconPicker => Self::AppPicker,
        }
    }
}

// ============================================================================
// Theme Colors
// ============================================================================

/// Theme-aware colors for the create quicklink UI.
#[derive(Clone)]
struct CreateQuicklinkColors {
    background: Hsla,
    surface: Hsla,
    surface_hover: Hsla,
    surface_selected: Hsla,
    text: Hsla,
    text_muted: Hsla,
    text_placeholder: Hsla,
    border: Hsla,
    border_focused: Hsla,
    accent: Hsla,
    accent_hover: Hsla,
    error: Hsla,
    warning: Hsla,
    success: Hsla,
}

impl CreateQuicklinkColors {
    fn from_theme(theme: &PhotonTheme) -> Self {
        Self {
            background: theme.colors.background.to_gpui(),
            surface: theme.colors.surface.to_gpui(),
            surface_hover: theme.colors.surface_hover.to_gpui(),
            surface_selected: theme.colors.surface_selected.to_gpui(),
            text: theme.colors.text.to_gpui(),
            text_muted: theme.colors.text_muted.to_gpui(),
            text_placeholder: theme.colors.text_placeholder.to_gpui(),
            border: theme.colors.border.to_gpui(),
            border_focused: theme.colors.border_focused.to_gpui(),
            accent: theme.colors.accent.to_gpui(),
            accent_hover: theme.colors.accent_hover.to_gpui(),
            error: theme.colors.error.to_gpui(),
            warning: theme.colors.warning.to_gpui(),
            success: theme.colors.success.to_gpui(),
        }
    }
}

fn get_colors(cx: &ViewContext<CreateQuicklinkView>) -> CreateQuicklinkColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    CreateQuicklinkColors::from_theme(&theme)
}

// ============================================================================
// Common Emojis for Icon Picker
// ============================================================================

/// Common emojis for quick selection.
const COMMON_EMOJIS: &[&str] = &[
    "🔍", "🌐", "📁", "📝", "📧", "📅", "💬", "🎵", "🎬", "📷", "🛒", "💻", "📱", "🎮", "📚",
    "💡", "⚙️", "🔧", "🔒", "🔑", "⭐", "❤️", "🚀", "💰", "📊", "📈", "🏠", "🏢", "✈️", "🚗",
];

// ============================================================================
// View State
// ============================================================================

/// Create/Edit Quicklink view state.
pub struct CreateQuicklinkView {
    /// Name input value.
    name_input: String,
    /// Link/URL input value.
    link_input: String,
    /// Alias input value.
    alias_input: String,
    /// Selected app bundle ID for "Open With".
    selected_app: Option<String>,
    /// Selected icon.
    selected_icon: QuickLinkIcon,

    /// ID of the quicklink being edited (None for create).
    editing_id: Option<QuickLinkId>,

    /// Current focus state.
    focus: CreateQuicklinkFocus,
    /// GPUI focus handle.
    focus_handle: FocusHandle,
    /// Available apps for "Open With" dropdown.
    available_apps: Vec<AppInfo>,
    /// Whether the app picker dropdown is open.
    show_app_picker: bool,
    /// Whether the icon picker is open.
    show_icon_picker: bool,
    /// Selected index in app picker.
    app_picker_index: usize,
    /// Selected index in icon picker.
    icon_picker_index: usize,

    /// Validation error for name field.
    name_error: Option<String>,
    /// Validation error for link field.
    link_error: Option<String>,

    /// Callback for when a quicklink is created/updated.
    on_event: Option<Box<dyn Fn(CreateQuicklinkEvent, &mut ViewContext<Self>) + 'static>>,
}

impl CreateQuicklinkView {
    /// Creates a new view for creating a quicklink.
    #[must_use]
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        // Try to auto-populate link from clipboard if it looks like a URL
        let link_input = cx
            .read_from_clipboard()
            .and_then(|c| c.text())
            .filter(|text| Self::is_valid_link(text.trim()))
            .map(|text| text.trim().to_string())
            .unwrap_or_default();

        Self {
            name_input: String::new(),
            link_input,
            alias_input: String::new(),
            selected_app: None,
            selected_icon: QuickLinkIcon::Default,
            editing_id: None,
            focus: CreateQuicklinkFocus::Name,
            focus_handle,
            available_apps: Self::default_apps(),
            show_app_picker: false,
            show_icon_picker: false,
            app_picker_index: 0,
            icon_picker_index: 0,
            name_error: None,
            link_error: None,
            on_event: None,
        }
    }

    /// Creates a new view for editing an existing quicklink.
    #[must_use]
    pub fn edit(link: &QuickLink, cx: &mut ViewContext<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        Self {
            name_input: link.name.clone(),
            link_input: link.link.clone(),
            alias_input: link.alias.clone().unwrap_or_default(),
            selected_app: link.open_with.clone(),
            selected_icon: link.icon.clone(),
            editing_id: Some(link.id.clone()),
            focus: CreateQuicklinkFocus::Name,
            focus_handle,
            available_apps: Self::default_apps(),
            show_app_picker: false,
            show_icon_picker: false,
            app_picker_index: 0,
            icon_picker_index: 0,
            name_error: None,
            link_error: None,
            on_event: None,
        }
    }

    /// Sets the event callback.
    pub fn on_event<F: Fn(CreateQuicklinkEvent, &mut ViewContext<Self>) + 'static>(
        &mut self,
        callback: F,
    ) {
        self.on_event = Some(Box::new(callback));
    }

    /// Sets available apps for the "Open With" picker.
    pub fn set_available_apps(&mut self, apps: Vec<AppInfo>, cx: &mut ViewContext<Self>) {
        self.available_apps = apps;
        cx.notify();
    }

    /// Returns the default list of common browser apps.
    fn default_apps() -> Vec<AppInfo> {
        vec![
            AppInfo::new("Safari", "com.apple.Safari"),
            AppInfo::new("Google Chrome", "com.google.Chrome"),
            AppInfo::new("Firefox", "org.mozilla.firefox"),
            AppInfo::new("Arc", "company.thebrowser.Browser"),
            AppInfo::new("Microsoft Edge", "com.microsoft.edgemac"),
            AppInfo::new("Brave Browser", "com.brave.Browser"),
            AppInfo::new("Opera", "com.operasoftware.Opera"),
            AppInfo::new("Vivaldi", "com.vivaldi.Vivaldi"),
        ]
    }

    /// Validates the form and returns true if valid.
    fn validate(&mut self) -> bool {
        let mut valid = true;

        // Validate name
        if self.name_input.trim().is_empty() {
            self.name_error = Some("Name is required".to_string());
            valid = false;
        } else {
            self.name_error = None;
        }

        // Validate link
        let link = self.link_input.trim();
        if link.is_empty() {
            self.link_error = Some("Link is required".to_string());
            valid = false;
        } else if !Self::is_valid_link(link) {
            self.link_error = Some("Invalid URL or path".to_string());
            valid = false;
        } else {
            self.link_error = None;
        }

        valid
    }

    /// Checks if a link is valid (URL or path).
    fn is_valid_link(link: &str) -> bool {
        // Accept URLs with common schemes
        if link.starts_with("http://")
            || link.starts_with("https://")
            || link.starts_with("file://")
            || link.starts_with("mailto:")
            || link.starts_with("tel:")
        {
            return true;
        }

        // Accept absolute paths
        if link.starts_with('/') || link.starts_with('~') {
            return true;
        }

        // Accept URLs without scheme (will be prefixed with https://)
        if link.contains('.') && !link.contains(' ') {
            return true;
        }

        false
    }

    /// Builds a QuickLink from the current form state.
    fn build_quicklink(&self) -> QuickLink {
        let mut link = if let Some(id) = &self.editing_id {
            let mut link = QuickLink::new(&self.name_input, &self.link_input);
            link.id = id.clone();
            link
        } else {
            QuickLink::new(&self.name_input, &self.link_input)
        };

        if !self.alias_input.trim().is_empty() {
            link.alias = Some(self.alias_input.trim().to_string());
        }

        link.open_with.clone_from(&self.selected_app);
        link.icon = self.selected_icon.clone();

        link
    }

    /// Handles form submission.
    fn submit(&mut self, cx: &mut ViewContext<Self>) {
        if !self.validate() {
            cx.notify();
            return;
        }

        let quicklink = self.build_quicklink();
        let event = if self.editing_id.is_some() {
            CreateQuicklinkEvent::Updated(quicklink)
        } else {
            CreateQuicklinkEvent::Created(quicklink)
        };

        if let Some(callback) = &self.on_event {
            callback(event, cx);
        }
    }

    /// Handles cancel action.
    fn cancel(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(callback) = &self.on_event {
            callback(CreateQuicklinkEvent::Cancelled, cx);
        }
        cx.remove_window();
    }

    /// Returns a preview of the final URL with sample substitution.
    fn preview_url(&self) -> String {
        let link = self.link_input.trim();
        if link.is_empty() {
            return String::new();
        }

        // Replace {argument} or {query} with sample text
        link.replace("{argument}", "example")
            .replace("{query}", "example")
    }

    /// Handles key down events.
    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        let key = event.keystroke.key.as_str();
        let shift = event.keystroke.modifiers.shift;
        let cmd = event.keystroke.modifiers.platform;

        // Handle escape
        if key == "escape" {
            if self.show_app_picker {
                self.show_app_picker = false;
                cx.notify();
                return;
            }
            if self.show_icon_picker {
                self.show_icon_picker = false;
                cx.notify();
                return;
            }
            self.cancel(cx);
            return;
        }

        // Handle tab navigation
        if key == "tab" {
            if self.show_app_picker || self.show_icon_picker {
                return;
            }
            self.focus = if shift {
                self.focus.previous()
            } else {
                self.focus.next()
            };
            cx.notify();
            return;
        }

        // Handle enter
        if key == "enter" {
            if self.show_app_picker {
                if let Some(app) = self.available_apps.get(self.app_picker_index) {
                    self.selected_app = Some(app.bundle_id.clone());
                }
                self.show_app_picker = false;
                cx.notify();
                return;
            }
            if self.show_icon_picker {
                if let Some(&emoji) = COMMON_EMOJIS.get(self.icon_picker_index) {
                    self.selected_icon = QuickLinkIcon::Emoji(emoji.to_string());
                }
                self.show_icon_picker = false;
                cx.notify();
                return;
            }
            // Submit on enter (Cmd+Enter or plain Enter)
            self.submit(cx);
            return;
        }

        // Handle up/down in dropdowns
        if self.show_app_picker {
            match key {
                "up" => {
                    if self.app_picker_index > 0 {
                        self.app_picker_index -= 1;
                    }
                    cx.notify();
                    return;
                }
                "down" => {
                    if self.app_picker_index + 1 < self.available_apps.len() {
                        self.app_picker_index += 1;
                    }
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }

        if self.show_icon_picker {
            let cols = 10; // Icons per row
            match key {
                "up" => {
                    if self.icon_picker_index >= cols {
                        self.icon_picker_index -= cols;
                    }
                    cx.notify();
                    return;
                }
                "down" => {
                    if self.icon_picker_index + cols < COMMON_EMOJIS.len() {
                        self.icon_picker_index += cols;
                    }
                    cx.notify();
                    return;
                }
                "left" => {
                    if self.icon_picker_index > 0 {
                        self.icon_picker_index -= 1;
                    }
                    cx.notify();
                    return;
                }
                "right" => {
                    if self.icon_picker_index + 1 < COMMON_EMOJIS.len() {
                        self.icon_picker_index += 1;
                    }
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }

        // Handle space to toggle dropdowns when focused
        if key == "space" {
            match self.focus {
                CreateQuicklinkFocus::AppPicker => {
                    self.show_app_picker = !self.show_app_picker;
                    self.show_icon_picker = false;
                    cx.notify();
                    return;
                }
                CreateQuicklinkFocus::IconPicker => {
                    self.show_icon_picker = !self.show_icon_picker;
                    self.show_app_picker = false;
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }

        // Handle backspace for text input
        if key == "backspace" {
            match self.focus {
                CreateQuicklinkFocus::Name => {
                    self.name_input.pop();
                    self.name_error = None;
                }
                CreateQuicklinkFocus::Link => {
                    self.link_input.pop();
                    self.link_error = None;
                }
                CreateQuicklinkFocus::Alias => {
                    self.alias_input.pop();
                }
                _ => {}
            }
            cx.notify();
            return;
        }

        // Handle cmd+backspace to clear field
        if cmd && key == "backspace" {
            match self.focus {
                CreateQuicklinkFocus::Name => {
                    self.name_input.clear();
                    self.name_error = None;
                }
                CreateQuicklinkFocus::Link => {
                    self.link_input.clear();
                    self.link_error = None;
                }
                CreateQuicklinkFocus::Alias => {
                    self.alias_input.clear();
                }
                _ => {}
            }
            cx.notify();
            return;
        }

        // Handle cmd+v to paste
        if cmd && key == "v" {
            if let Some(clipboard) = cx.read_from_clipboard() {
                let text = clipboard.text().unwrap_or_default();
                if !text.is_empty() {
                    match self.focus {
                        CreateQuicklinkFocus::Name => {
                            self.name_input.push_str(&text);
                            self.name_error = None;
                        }
                        CreateQuicklinkFocus::Link => {
                            self.link_input.push_str(&text);
                            self.link_error = None;
                        }
                        CreateQuicklinkFocus::Alias => {
                            self.alias_input.push_str(&text);
                        }
                        _ => {}
                    }
                    cx.notify();
                }
            }
            return;
        }

        // Skip other modifier keys for text input
        if cmd || event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
            return;
        }

        // Handle text input
        let input_char = if let Some(ime_key) = &event.keystroke.ime_key {
            ime_key.clone()
        } else if key.len() == 1 {
            if shift {
                key.to_uppercase()
            } else {
                key.to_string()
            }
        } else {
            return;
        };

        match self.focus {
            CreateQuicklinkFocus::Name => {
                self.name_input.push_str(&input_char);
                self.name_error = None;
            }
            CreateQuicklinkFocus::Link => {
                self.link_input.push_str(&input_char);
                self.link_error = None;
            }
            CreateQuicklinkFocus::Alias => {
                self.alias_input.push_str(&input_char);
            }
            _ => {}
        }
        cx.notify();
    }

    // ========================================================================
    // Render Helpers
    // ========================================================================

    /// Renders a labeled input field.
    fn render_input_field(
        &self,
        label: &str,
        value: &str,
        placeholder: &str,
        is_focused: bool,
        error: Option<&str>,
        colors: &CreateQuicklinkColors,
        highlight_placeholders: bool,
    ) -> impl IntoElement {
        let border_color = if error.is_some() {
            colors.error
        } else if is_focused {
            colors.border_focused
        } else {
            colors.border
        };

        let text_color = colors.text;
        let placeholder_color = colors.text_placeholder;
        let error_color = colors.error;
        let muted_color = colors.text_muted;
        let surface = colors.surface;
        let warning_color = colors.warning;

        let display_value = if value.is_empty() {
            placeholder.to_string()
        } else {
            value.to_string()
        };

        let is_empty = value.is_empty();

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                // Label
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(muted_color)
                    .child(label.to_string()),
            )
            .child(
                // Input field
                div()
                    .px(px(12.0))
                    .py(px(10.0))
                    .rounded(px(8.0))
                    .bg(surface)
                    .border_1()
                    .border_color(border_color)
                    .child(if highlight_placeholders && !is_empty {
                        // Render with placeholder highlighting
                        Self::render_highlighted_text(value, text_color, warning_color)
                    } else {
                        div()
                            .text_base()
                            .text_color(if is_empty {
                                placeholder_color
                            } else {
                                text_color
                            })
                            .child(display_value)
                            .into_any_element()
                    }),
            )
            .when(error.is_some(), |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(error_color)
                        .child(error.unwrap_or("").to_string()),
                )
            })
    }

    /// Renders text with {placeholder} sections highlighted.
    fn render_highlighted_text(
        text: &str,
        text_color: Hsla,
        highlight_color: Hsla,
    ) -> gpui::AnyElement {
        let mut elements: Vec<gpui::AnyElement> = Vec::new();
        let mut current_pos = 0;

        // Find all {placeholder} patterns
        let chars: Vec<char> = text.chars().collect();
        let text_len = chars.len();

        while current_pos < text_len {
            // Find next '{'
            if let Some(start) = text[current_pos..].find('{') {
                let abs_start = current_pos + start;

                // Add text before '{'
                if abs_start > current_pos {
                    let before: String = chars[current_pos..abs_start].iter().collect();
                    elements.push(
                        div()
                            .text_base()
                            .text_color(text_color)
                            .child(before)
                            .into_any_element(),
                    );
                }

                // Find closing '}'
                if let Some(end) = text[abs_start..].find('}') {
                    let abs_end = abs_start + end + 1;
                    let placeholder: String = chars[abs_start..abs_end].iter().collect();
                    elements.push(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(highlight_color)
                            .child(placeholder)
                            .into_any_element(),
                    );
                    current_pos = abs_end;
                } else {
                    // No closing '}', treat rest as normal text
                    let rest: String = chars[abs_start..].iter().collect();
                    elements.push(
                        div()
                            .text_base()
                            .text_color(text_color)
                            .child(rest)
                            .into_any_element(),
                    );
                    break;
                }
            } else {
                // No more '{', add rest as normal text
                let rest: String = chars[current_pos..].iter().collect();
                elements.push(
                    div()
                        .text_base()
                        .text_color(text_color)
                        .child(rest)
                        .into_any_element(),
                );
                break;
            }
        }

        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .children(elements)
            .into_any_element()
    }

    /// Renders the "Open With" app picker.
    fn render_app_picker(
        &self,
        colors: &CreateQuicklinkColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let is_focused = self.focus == CreateQuicklinkFocus::AppPicker;
        let border_color = if is_focused {
            colors.border_focused
        } else {
            colors.border
        };
        let text_color = colors.text;
        let muted_color = colors.text_muted;
        let surface = colors.surface;
        let surface_hover = colors.surface_hover;
        let surface_selected = colors.surface_selected;
        let background = colors.background;

        let selected_name = self
            .selected_app
            .as_ref()
            .and_then(|id| {
                self.available_apps
                    .iter()
                    .find(|a| &a.bundle_id == id)
                    .map(|a| a.name.clone())
            })
            .unwrap_or_else(|| "Default Browser".to_string());

        let show_picker = self.show_app_picker;
        let picker_index = self.app_picker_index;

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                // Label
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(muted_color)
                    .child("Open With"),
            )
            .child(
                // Dropdown button
                div()
                    .relative()
                    .child(
                        div()
                            .id("app-picker-button")
                            .px(px(12.0))
                            .py(px(10.0))
                            .rounded(px(8.0))
                            .bg(surface)
                            .border_1()
                            .border_color(border_color)
                            .cursor_pointer()
                            .flex()
                            .justify_between()
                            .items_center()
                            .on_click(cx.listener(|this, _, cx| {
                                this.show_app_picker = !this.show_app_picker;
                                this.show_icon_picker = false;
                                this.focus = CreateQuicklinkFocus::AppPicker;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_base()
                                    .text_color(text_color)
                                    .child(selected_name),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_color)
                                    .child(if show_picker { "▲" } else { "▼" }),
                            ),
                    )
                    .when(show_picker, |el| {
                        el.child(
                            // Dropdown menu
                            div()
                                .id("app-picker-dropdown")
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(48.0))
                                .bg(background)
                                .border_1()
                                .border_color(border_color)
                                .rounded(px(8.0))
                                .shadow_lg()
                                .max_h(px(200.0))
                                .overflow_y_scroll()
                                .py(px(4.0))
                                .child(
                                    // Default option
                                    div()
                                        .id("app-default")
                                        .px(px(12.0))
                                        .py(px(8.0))
                                        .cursor_pointer()
                                        .bg(if picker_index == 0 {
                                            surface_selected
                                        } else {
                                            background
                                        })
                                        .hover(|s| s.bg(surface_hover))
                                        .text_base()
                                        .text_color(text_color)
                                        .on_click(cx.listener(|this, _, cx| {
                                            this.selected_app = None;
                                            this.show_app_picker = false;
                                            cx.notify();
                                        }))
                                        .child("Default Browser"),
                                )
                                .children(self.available_apps.iter().enumerate().map(|(i, app)| {
                                    let is_selected = picker_index == i + 1;
                                    let bundle_id = app.bundle_id.clone();
                                    div()
                                        .id(SharedString::from(format!("app-{}", i)))
                                        .px(px(12.0))
                                        .py(px(8.0))
                                        .cursor_pointer()
                                        .bg(if is_selected {
                                            surface_selected
                                        } else {
                                            background
                                        })
                                        .hover(|s| s.bg(surface_hover))
                                        .text_base()
                                        .text_color(text_color)
                                        .on_click(cx.listener(move |this, _, cx| {
                                            this.selected_app = Some(bundle_id.clone());
                                            this.show_app_picker = false;
                                            cx.notify();
                                        }))
                                        .child(app.name.clone())
                                })),
                        )
                    }),
            )
    }

    /// Renders the icon picker.
    fn render_icon_picker(
        &self,
        colors: &CreateQuicklinkColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let is_focused = self.focus == CreateQuicklinkFocus::IconPicker;
        let border_color = if is_focused {
            colors.border_focused
        } else {
            colors.border
        };
        let text_color = colors.text;
        let muted_color = colors.text_muted;
        let surface = colors.surface;
        let surface_hover = colors.surface_hover;
        let surface_selected = colors.surface_selected;
        let background = colors.background;

        let current_icon = match &self.selected_icon {
            QuickLinkIcon::Emoji(e) => e.clone(),
            QuickLinkIcon::Default => "🌐".to_string(),
            _ => "📎".to_string(),
        };

        let show_picker = self.show_icon_picker;
        let picker_index = self.icon_picker_index;

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                // Label
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(muted_color)
                    .child("Icon"),
            )
            .child(
                // Icon button
                div()
                    .relative()
                    .child(
                        div()
                            .id("icon-picker-button")
                            .size(px(48.0))
                            .rounded(px(8.0))
                            .bg(surface)
                            .border_1()
                            .border_color(border_color)
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_2xl()
                            .on_click(cx.listener(|this, _, cx| {
                                this.show_icon_picker = !this.show_icon_picker;
                                this.show_app_picker = false;
                                this.focus = CreateQuicklinkFocus::IconPicker;
                                cx.notify();
                            }))
                            .child(current_icon),
                    )
                    .when(show_picker, |el| {
                        el.child(
                            // Icon grid
                            div()
                                .absolute()
                                .left_0()
                                .top(px(56.0))
                                .w(px(320.0))
                                .bg(background)
                                .border_1()
                                .border_color(border_color)
                                .rounded(px(8.0))
                                .shadow_lg()
                                .p(px(8.0))
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap(px(4.0))
                                        .children(COMMON_EMOJIS.iter().enumerate().map(
                                            |(i, &emoji)| {
                                                let is_selected = picker_index == i;
                                                let emoji_str = emoji.to_string();
                                                div()
                                                    .id(SharedString::from(format!("emoji-{}", i)))
                                                    .size(px(32.0))
                                                    .rounded(px(4.0))
                                                    .cursor_pointer()
                                                    .bg(if is_selected {
                                                        surface_selected
                                                    } else {
                                                        background
                                                    })
                                                    .hover(|s| s.bg(surface_hover))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_lg()
                                                    .on_click(cx.listener(move |this, _, cx| {
                                                        this.selected_icon =
                                                            QuickLinkIcon::Emoji(emoji_str.clone());
                                                        this.show_icon_picker = false;
                                                        cx.notify();
                                                    }))
                                                    .child(emoji.to_string())
                                            },
                                        )),
                                )
                                .child(
                                    // Default option
                                    div()
                                        .mt(px(8.0))
                                        .pt(px(8.0))
                                        .border_t_1()
                                        .border_color(border_color)
                                        .child(
                                            div()
                                                .id("icon-default")
                                                .px(px(8.0))
                                                .py(px(4.0))
                                                .rounded(px(4.0))
                                                .cursor_pointer()
                                                .hover(|s| s.bg(surface_hover))
                                                .text_sm()
                                                .text_color(text_color)
                                                .on_click(cx.listener(|this, _, cx| {
                                                    this.selected_icon = QuickLinkIcon::Default;
                                                    this.show_icon_picker = false;
                                                    cx.notify();
                                                }))
                                                .child("Use default icon"),
                                        ),
                                ),
                        )
                    }),
            )
    }

    /// Renders the URL preview section.
    fn render_preview(&self, colors: &CreateQuicklinkColors) -> impl IntoElement {
        let preview = self.preview_url();
        let muted_color = colors.text_muted;
        let surface = colors.surface;
        let success_color = colors.success;

        if preview.is_empty() {
            return div().into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(muted_color)
                    .child("Preview"),
            )
            .child(
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(8.0))
                    .bg(surface)
                    .text_sm()
                    .text_color(success_color)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(preview),
            )
            .into_any_element()
    }

    /// Renders the action buttons.
    fn render_actions(
        &self,
        colors: &CreateQuicklinkColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let surface = colors.surface;
        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let accent = colors.accent;
        let accent_hover = colors.accent_hover;
        let background = colors.background;

        let is_editing = self.editing_id.is_some();
        let submit_label = if is_editing {
            "Save"
        } else {
            "Create"
        };

        div()
            .flex()
            .flex_row()
            .justify_end()
            .gap(px(12.0))
            .child(
                // Cancel button
                div()
                    .id("cancel-button")
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(px(8.0))
                    .bg(surface)
                    .text_base()
                    .text_color(text_color)
                    .cursor_pointer()
                    .hover(|s| s.bg(surface_hover))
                    .on_click(cx.listener(|this, _, cx| {
                        this.cancel(cx);
                    }))
                    .child("Cancel"),
            )
            .child(
                // Submit button
                div()
                    .id("submit-button")
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(px(8.0))
                    .bg(accent)
                    .text_base()
                    .text_color(background)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|s| s.bg(accent_hover))
                    .on_click(cx.listener(|this, _, cx| {
                        this.submit(cx);
                    }))
                    .child(submit_label),
            )
    }
}

impl FocusableView for CreateQuicklinkView {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CreateQuicklinkView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let is_editing = self.editing_id.is_some();
        let title = if is_editing {
            "Edit Quicklink"
        } else {
            "Create Quicklink"
        };

        div()
            .track_focus(&self.focus_handle)
            .key_context("CreateQuicklink")
            .on_key_down(cx.listener(Self::handle_key_down))
            .size_full()
            .bg(colors.background)
            .p(rems(1.5))
            .flex()
            .flex_col()
            .gap(rems(1.0))
            // Title
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(colors.text)
                    .child(title),
            )
            // Form fields
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(rems(0.75))
                    // Name input
                    .child(self.render_input_field(
                        "Name",
                        &self.name_input.clone(),
                        "Enter a name for your quicklink",
                        self.focus == CreateQuicklinkFocus::Name,
                        self.name_error.as_deref(),
                        &colors,
                        false,
                    ))
                    // Link input
                    .child(self.render_input_field(
                        "Link",
                        &self.link_input.clone(),
                        "https://example.com/search?q={query}",
                        self.focus == CreateQuicklinkFocus::Link,
                        self.link_error.as_deref(),
                        &colors,
                        true, // Highlight {placeholders}
                    ))
                    // Alias input
                    .child(self.render_input_field(
                        "Alias (optional)",
                        &self.alias_input.clone(),
                        "Short keyword for quick access",
                        self.focus == CreateQuicklinkFocus::Alias,
                        None,
                        &colors,
                        false,
                    )),
            )
            // App picker and icon picker row
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(rems(1.0))
                    .child(div().flex_1().child(self.render_app_picker(&colors, cx)))
                    .child(self.render_icon_picker(&colors, cx)),
            )
            // Preview
            .child(self.render_preview(&colors))
            // Spacer
            .child(div().flex_1())
            // Keyboard hints
            .child(
                div()
                    .text_xs()
                    .text_color(colors.text_muted)
                    .child("Tab to navigate • Enter to submit • Escape to cancel"),
            )
            // Action buttons
            .child(self.render_actions(&colors, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_link() {
        assert!(CreateQuicklinkView::is_valid_link("https://example.com"));
        assert!(CreateQuicklinkView::is_valid_link("http://example.com"));
        assert!(CreateQuicklinkView::is_valid_link("file:///path/to/file"));
        assert!(CreateQuicklinkView::is_valid_link("mailto:test@example.com"));
        assert!(CreateQuicklinkView::is_valid_link("/usr/local/bin"));
        assert!(CreateQuicklinkView::is_valid_link("~/Documents"));
        assert!(CreateQuicklinkView::is_valid_link("example.com"));

        assert!(!CreateQuicklinkView::is_valid_link(""));
        assert!(!CreateQuicklinkView::is_valid_link("not a url"));
        assert!(!CreateQuicklinkView::is_valid_link("random text"));
    }

    #[test]
    fn test_focus_navigation() {
        assert_eq!(CreateQuicklinkFocus::Name.next(), CreateQuicklinkFocus::Link);
        assert_eq!(
            CreateQuicklinkFocus::Link.next(),
            CreateQuicklinkFocus::Alias
        );
        assert_eq!(
            CreateQuicklinkFocus::Alias.next(),
            CreateQuicklinkFocus::AppPicker
        );
        assert_eq!(
            CreateQuicklinkFocus::AppPicker.next(),
            CreateQuicklinkFocus::IconPicker
        );
        assert_eq!(
            CreateQuicklinkFocus::IconPicker.next(),
            CreateQuicklinkFocus::Name
        );

        assert_eq!(
            CreateQuicklinkFocus::Name.previous(),
            CreateQuicklinkFocus::IconPicker
        );
        assert_eq!(
            CreateQuicklinkFocus::Link.previous(),
            CreateQuicklinkFocus::Name
        );
    }

    #[test]
    fn test_app_info_creation() {
        let app = AppInfo::new("Safari", "com.apple.Safari");
        assert_eq!(app.name, "Safari");
        assert_eq!(app.bundle_id, "com.apple.Safari");
        assert!(app.icon.is_none());

        let app_with_icon =
            AppInfo::new("Chrome", "com.google.Chrome").with_icon(PathBuf::from("/path/icon.png"));
        assert!(app_with_icon.icon.is_some());
    }
}
