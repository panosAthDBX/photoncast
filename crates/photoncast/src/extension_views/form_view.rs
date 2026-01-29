//! FormView rendering for extensions.
//!
//! Renders `FormView` types with:
//! - TextField, TextArea, Password, Number, Checkbox
//! - Dropdown with options
//! - FilePicker and DirectoryPicker using native dialogs
//! - DatePicker
//! - Validation errors shown inline
//! - Submit button with keyboard shortcut (⌘⏎)

use std::collections::HashMap;
use std::time::{Duration, Instant};

use abi_stable::std_types::RVec;
use gpui::prelude::FluentBuilder;
use gpui::*;
use photoncast_extension_api::{DropdownOption, FieldType, FormField, FormView, ROption};

use super::actions::CLOSE_VIEW_ACTION;
use super::colors::ExtensionViewColors;
use super::dimensions::*;
use super::ActionCallback;

// ============================================================================
// Actions
// ============================================================================

actions!(
    extension_form,
    [Submit, Cancel, NextField, PreviousField, ToggleDropdown]
);

/// Registers key bindings for the extension form view.
pub fn register_key_bindings(cx: &mut gpui::AppContext) {
    cx.bind_keys([
        KeyBinding::new("cmd-enter", Submit, Some("ExtensionFormView")),
        KeyBinding::new("escape", Cancel, Some("ExtensionFormView")),
        KeyBinding::new("tab", NextField, Some("ExtensionFormView")),
        KeyBinding::new("shift-tab", PreviousField, Some("ExtensionFormView")),
        KeyBinding::new("space", ToggleDropdown, Some("ExtensionFormView")),
    ]);
}

// ============================================================================
// View State
// ============================================================================

/// Extension FormView state.
pub struct ExtensionFormView {
    /// The form view data from the extension.
    form_view: FormView,
    /// Current field values.
    values: HashMap<String, FieldValue>,
    /// Validation errors for each field.
    errors: HashMap<String, String>,
    /// Currently focused field index.
    focused_field_index: usize,
    /// Cursor position in text fields.
    cursor_positions: HashMap<String, usize>,
    /// Cursor blink epoch.
    cursor_blink_epoch: Instant,
    /// Whether dropdown for current field is open.
    dropdown_open: bool,
    /// Selected index in dropdown.
    dropdown_index: usize,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Action callback for handling form submission.
    action_callback: Option<ActionCallback>,
}

/// Field value types.
#[derive(Clone, Debug)]
enum FieldValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    Date(u64), // Unix timestamp
}

impl Default for FieldValue {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl ExtensionFormView {
    /// Creates a new extension form view.
    pub fn new(
        form_view: FormView,
        action_callback: Option<ActionCallback>,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        // Initialize field values from defaults
        let mut values = HashMap::new();
        let mut cursor_positions = HashMap::new();

        for field in form_view.fields.iter() {
            let field_id = field.id.to_string();
            let default_value = field.default_value.clone().into_option();

            let value = match &field.field_type {
                FieldType::TextField
                | FieldType::TextArea { .. }
                | FieldType::Password
                | FieldType::FilePicker { .. }
                | FieldType::DirectoryPicker => {
                    let text = default_value.unwrap_or_default().to_string();
                    cursor_positions.insert(field_id.clone(), text.len());
                    FieldValue::Text(text)
                },
                FieldType::Number { .. } => {
                    let num = default_value
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    FieldValue::Number(num)
                },
                FieldType::Checkbox => {
                    let checked = default_value
                        .map(|s| s.as_str() == "true" || s.as_str() == "1")
                        .unwrap_or(false);
                    FieldValue::Boolean(checked)
                },
                FieldType::Dropdown { options } => {
                    let value = default_value
                        .or_else(|| options.first().map(|o| o.value.clone()))
                        .unwrap_or_default()
                        .to_string();
                    FieldValue::Text(value)
                },
                FieldType::DatePicker => {
                    let timestamp = default_value
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    FieldValue::Date(timestamp)
                },
            };

            values.insert(field_id, value);
        }

        let view = Self {
            form_view,
            values,
            errors: HashMap::new(),
            focused_field_index: 0,
            cursor_positions,
            cursor_blink_epoch: Instant::now(),
            dropdown_open: false,
            dropdown_index: 0,
            focus_handle,
            action_callback,
        };

        view.start_cursor_blink_timer(cx);
        view
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

    /// Checks if cursor should be visible.
    fn cursor_visible(&self) -> bool {
        const BLINK_INTERVAL_MS: u128 = 530;
        let elapsed = self.cursor_blink_epoch.elapsed().as_millis();
        (elapsed / BLINK_INTERVAL_MS) % 2 == 0
    }

    /// Resets cursor blink timer.
    fn reset_cursor_blink(&mut self) {
        self.cursor_blink_epoch = Instant::now();
    }

    /// Gets the currently focused field.
    fn focused_field(&self) -> Option<&FormField> {
        self.form_view.fields.get(self.focused_field_index)
    }

    /// Gets the field ID for the focused field.
    fn focused_field_id(&self) -> Option<String> {
        self.focused_field().map(|f| f.id.to_string())
    }

    /// Validates all fields and returns true if valid.
    fn validate(&mut self) -> bool {
        self.errors.clear();

        for field in self.form_view.fields.iter() {
            let field_id = field.id.to_string();
            let value = self.values.get(&field_id);

            // Check required fields
            if field.required {
                let is_empty = match value {
                    Some(FieldValue::Text(s)) => s.is_empty(),
                    Some(FieldValue::Number(n)) => n.is_nan(),
                    Some(FieldValue::Boolean(_)) => false,
                    Some(FieldValue::Date(t)) => *t == 0,
                    None => true,
                };

                if is_empty {
                    self.errors
                        .insert(field_id.clone(), "This field is required".to_string());
                    continue;
                }
            }

            // Check validation rules
            if let ROption::RSome(validation) = &field.validation {
                if let Some(FieldValue::Text(text)) = value {
                    // Min length
                    if let ROption::RSome(min_len) = validation.min_length {
                        if text.len() < min_len as usize {
                            self.errors.insert(
                                field_id.clone(),
                                format!("Must be at least {} characters", min_len),
                            );
                            continue;
                        }
                    }

                    // Max length
                    if let ROption::RSome(max_len) = validation.max_length {
                        if text.len() > max_len as usize {
                            self.errors.insert(
                                field_id.clone(),
                                format!("Must be at most {} characters", max_len),
                            );
                            continue;
                        }
                    }

                    // Pattern (regex)
                    if let ROption::RSome(pattern) = &validation.pattern {
                        if let Ok(re) = regex::Regex::new(pattern.as_str()) {
                            if !re.is_match(text) {
                                let msg = validation.message.to_string();
                                self.errors.insert(field_id.clone(), msg);
                                continue;
                            }
                        }
                    }
                }

                // Number range validation
                if let Some(FieldValue::Number(num)) = value {
                    if let FieldType::Number { min, max } = &field.field_type {
                        if let ROption::RSome(min_val) = min {
                            if *num < *min_val {
                                self.errors.insert(
                                    field_id.clone(),
                                    format!("Must be at least {}", min_val),
                                );
                                continue;
                            }
                        }
                        if let ROption::RSome(max_val) = max {
                            if *num > *max_val {
                                self.errors.insert(
                                    field_id.clone(),
                                    format!("Must be at most {}", max_val),
                                );
                                continue;
                            }
                        }
                    }
                }
            }
        }

        self.errors.is_empty()
    }

    /// Submits the form.
    fn submit_form(&mut self, cx: &mut ViewContext<Self>) {
        if !self.validate() {
            cx.notify();
            return;
        }

        // Serialize values and call callback
        if let Some(callback) = &self.action_callback {
            // Build JSON-like string of values
            let mut values_str = String::from("{");
            for (i, (key, value)) in self.values.iter().enumerate() {
                if i > 0 {
                    values_str.push(',');
                }
                let val_str = match value {
                    FieldValue::Text(s) => format!("\"{}\":\"{}\"", key, s.replace('"', "\\\"")),
                    FieldValue::Number(n) => format!("\"{}\":{}", key, n),
                    FieldValue::Boolean(b) => format!("\"{}\":{}", key, b),
                    FieldValue::Date(t) => format!("\"{}\":{}", key, t),
                };
                values_str.push_str(&val_str);
            }
            values_str.push('}');

            callback(&format!("__submit__:{}", values_str), cx);
        }
    }

    /// Opens file picker dialog.
    fn open_file_picker(
        &mut self,
        field_id: &str,
        extensions: &[String],
        cx: &mut ViewContext<Self>,
    ) {
        let field_id = field_id.to_string();
        let extensions: Vec<String> = extensions.to_vec();

        cx.spawn(|this, mut cx| async move {
            // Use rfd for native file dialog
            let result = cx
                .background_executor()
                .spawn(async move {
                    let mut dialog = rfd::FileDialog::new();
                    if !extensions.is_empty() {
                        let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
                        dialog = dialog.add_filter("Files", &ext_refs);
                    }
                    dialog.pick_file()
                })
                .await;

            if let Some(path) = result {
                let _ = this.update(&mut cx, |view, cx| {
                    view.values.insert(
                        field_id.clone(),
                        FieldValue::Text(path.display().to_string()),
                    );
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Opens directory picker dialog.
    fn open_directory_picker(&mut self, field_id: &str, cx: &mut ViewContext<Self>) {
        let field_id = field_id.to_string();

        cx.spawn(|this, mut cx| async move {
            let result = cx
                .background_executor()
                .spawn(async move { rfd::FileDialog::new().pick_folder() })
                .await;

            if let Some(path) = result {
                let _ = this.update(&mut cx, |view, cx| {
                    view.values.insert(
                        field_id.clone(),
                        FieldValue::Text(path.display().to_string()),
                    );
                    cx.notify();
                });
            }
        })
        .detach();
    }

    // ========================================================================
    // Action Handlers
    // ========================================================================

    fn submit(&mut self, _: &Submit, cx: &mut ViewContext<Self>) {
        self.submit_form(cx);
    }

    fn cancel(&mut self, _: &Cancel, cx: &mut ViewContext<Self>) {
        if self.dropdown_open {
            self.dropdown_open = false;
            cx.notify();
        } else if let Some(callback) = &self.action_callback {
            callback(CLOSE_VIEW_ACTION, cx);
        }
    }

    fn next_field(&mut self, _: &NextField, cx: &mut ViewContext<Self>) {
        if self.dropdown_open {
            // Navigate dropdown
            if let Some(field) = self.focused_field() {
                if let FieldType::Dropdown { options } = &field.field_type {
                    if !options.is_empty() {
                        self.dropdown_index = (self.dropdown_index + 1) % options.len();
                        cx.notify();
                    }
                }
            }
        } else {
            // Navigate fields
            if !self.form_view.fields.is_empty() {
                self.focused_field_index =
                    (self.focused_field_index + 1) % self.form_view.fields.len();
                self.dropdown_open = false;
                cx.notify();
            }
        }
    }

    fn previous_field(&mut self, _: &PreviousField, cx: &mut ViewContext<Self>) {
        if self.dropdown_open {
            // Navigate dropdown
            if let Some(field) = self.focused_field() {
                if let FieldType::Dropdown { options } = &field.field_type {
                    if !options.is_empty() {
                        self.dropdown_index = if self.dropdown_index == 0 {
                            options.len() - 1
                        } else {
                            self.dropdown_index - 1
                        };
                        cx.notify();
                    }
                }
            }
        } else {
            // Navigate fields
            if !self.form_view.fields.is_empty() {
                self.focused_field_index = if self.focused_field_index == 0 {
                    self.form_view.fields.len() - 1
                } else {
                    self.focused_field_index - 1
                };
                self.dropdown_open = false;
                cx.notify();
            }
        }
    }

    fn toggle_dropdown(&mut self, _: &ToggleDropdown, cx: &mut ViewContext<Self>) {
        if let Some(field) = self.focused_field() {
            match &field.field_type {
                FieldType::Dropdown { options } => {
                    if self.dropdown_open {
                        // Select current option
                        if let Some(option) = options.get(self.dropdown_index) {
                            let field_id = field.id.to_string();
                            self.values
                                .insert(field_id, FieldValue::Text(option.value.to_string()));
                        }
                        self.dropdown_open = false;
                    } else {
                        self.dropdown_open = true;
                        self.dropdown_index = 0;
                    }
                    cx.notify();
                },
                FieldType::Checkbox => {
                    let field_id = field.id.to_string();
                    let current = match self.values.get(&field_id) {
                        Some(FieldValue::Boolean(b)) => *b,
                        _ => false,
                    };
                    self.values.insert(field_id, FieldValue::Boolean(!current));
                    cx.notify();
                },
                FieldType::FilePicker { allowed_extensions } => {
                    let field_id = field.id.to_string();
                    let exts: Vec<String> =
                        allowed_extensions.iter().map(|s| s.to_string()).collect();
                    self.open_file_picker(&field_id, &exts, cx);
                },
                FieldType::DirectoryPicker => {
                    let field_id = field.id.to_string();
                    self.open_directory_picker(&field_id, cx);
                },
                _ => {},
            }
        }
    }

    // ========================================================================
    // Rendering
    // ========================================================================

    /// Renders a form field.
    fn render_field(
        &self,
        field: &FormField,
        index: usize,
        colors: &ExtensionViewColors,
    ) -> impl IntoElement {
        let field_id = field.id.to_string();
        let is_focused = index == self.focused_field_index;
        let error = self.errors.get(&field_id);

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            // Label
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(colors.text)
                            .child(field.label.to_string()),
                    )
                    .when(field.required, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .text_color(colors.error)
                                .child("*"),
                        )
                    }),
            )
            // Input
            .child(self.render_field_input(field, is_focused, colors))
            // Error message
            .when_some(error.cloned(), |el, err| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(colors.error)
                        .child(err),
                )
            })
    }

    /// Renders the input component for a field.
    fn render_field_input(
        &self,
        field: &FormField,
        is_focused: bool,
        colors: &ExtensionViewColors,
    ) -> gpui::Div {
        let field_id = field.id.to_string();
        let value = self.values.get(&field_id);
        let placeholder = field.placeholder.clone().into_option();

        match &field.field_type {
            FieldType::TextField | FieldType::Password => {
                let text = match value {
                    Some(FieldValue::Text(s)) => s.clone(),
                    _ => String::new(),
                };
                let is_password = matches!(field.field_type, FieldType::Password);
                let display_text = if is_password {
                    "•".repeat(text.len())
                } else {
                    text.clone()
                };
                let cursor_pos = self.cursor_positions.get(&field_id).copied().unwrap_or(0);

                self.render_text_input(&display_text, cursor_pos, is_focused, placeholder, colors)
            },
            FieldType::TextArea { rows } => {
                let text = match value {
                    Some(FieldValue::Text(s)) => s.clone(),
                    _ => String::new(),
                };
                let _cursor_pos = self.cursor_positions.get(&field_id).copied().unwrap_or(0);

                div()
                    .w_full()
                    .h(px(*rows as f32 * 20.0 + 16.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(BORDER_RADIUS)
                    .border_1()
                    .border_color(if is_focused {
                        colors.border_focused
                    } else {
                        colors.border
                    })
                    .bg(colors.surface)
                    .text_sm()
                    .text_color(if text.is_empty() {
                        colors.text_placeholder
                    } else {
                        colors.text
                    })
                    .child(if text.is_empty() {
                        placeholder.unwrap_or_default().to_string()
                    } else {
                        text
                    })
            },
            FieldType::Number { min: _, max: _ } => {
                let num = match value {
                    Some(FieldValue::Number(n)) => *n,
                    _ => 0.0,
                };
                let text = if num == num.floor() {
                    format!("{}", num as i64)
                } else {
                    format!("{}", num)
                };
                let cursor_pos = self
                    .cursor_positions
                    .get(&field_id)
                    .copied()
                    .unwrap_or(text.len());

                self.render_text_input(&text, cursor_pos, is_focused, placeholder, colors)
            },
            FieldType::Checkbox => {
                let checked = match value {
                    Some(FieldValue::Boolean(b)) => *b,
                    _ => false,
                };

                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(if is_focused {
                                colors.border_focused
                            } else {
                                colors.border
                            })
                            .bg(if checked {
                                colors.accent
                            } else {
                                colors.surface
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(checked, |el| {
                                el.child(div().text_color(gpui::white()).text_sm().child("✓"))
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child("Press Space to toggle"),
                    )
            },
            FieldType::Dropdown { options } => {
                let selected_value = match value {
                    Some(FieldValue::Text(s)) => s.clone(),
                    _ => String::new(),
                };
                let selected_label = options
                    .iter()
                    .find(|o| o.value.as_str() == selected_value)
                    .map(|o| o.label.to_string())
                    .unwrap_or(selected_value);

                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .h(px(40.0))
                            .px(px(12.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .rounded(BORDER_RADIUS)
                            .border_1()
                            .border_color(if is_focused {
                                colors.border_focused
                            } else {
                                colors.border
                            })
                            .bg(colors.surface)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text)
                                    .child(selected_label),
                            )
                            .child(div().text_color(colors.text_muted).child(
                                if self.dropdown_open && is_focused {
                                    "▲"
                                } else {
                                    "▼"
                                },
                            )),
                    )
                    .when(self.dropdown_open && is_focused, |el| {
                        el.child(self.render_dropdown_options(options, colors))
                    })
            },
            FieldType::FilePicker { .. } | FieldType::DirectoryPicker => {
                let path = match value {
                    Some(FieldValue::Text(s)) => s.clone(),
                    _ => String::new(),
                };
                let is_file = matches!(field.field_type, FieldType::FilePicker { .. });

                div()
                    .w_full()
                    .h(px(40.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(BORDER_RADIUS)
                    .border_1()
                    .border_color(if is_focused {
                        colors.border_focused
                    } else {
                        colors.border
                    })
                    .bg(colors.surface)
                    .cursor_pointer()
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_sm()
                            .text_color(if path.is_empty() {
                                colors.text_placeholder
                            } else {
                                colors.text
                            })
                            .child(if path.is_empty() {
                                format!(
                                    "Press Space to select {}",
                                    if is_file { "file" } else { "folder" }
                                )
                            } else {
                                path
                            }),
                    )
                    .child(div().text_color(colors.text_muted).child(if is_file {
                        "📄"
                    } else {
                        "📁"
                    }))
            },
            FieldType::DatePicker => {
                let timestamp = match value {
                    Some(FieldValue::Date(t)) => *t,
                    _ => 0,
                };
                let date_str = if timestamp == 0 {
                    "Select date...".to_string()
                } else {
                    // Format timestamp - in production use chrono
                    format!("Date: {}", timestamp)
                };

                div()
                    .w_full()
                    .h(px(40.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(BORDER_RADIUS)
                    .border_1()
                    .border_color(if is_focused {
                        colors.border_focused
                    } else {
                        colors.border
                    })
                    .bg(colors.surface)
                    .cursor_pointer()
                    .child(
                        div()
                            .text_sm()
                            .text_color(if timestamp == 0 {
                                colors.text_placeholder
                            } else {
                                colors.text
                            })
                            .child(date_str),
                    )
                    .child(div().text_color(colors.text_muted).child("📅"))
            },
        }
    }

    /// Renders a text input with cursor.
    fn render_text_input(
        &self,
        text: &str,
        cursor_pos: usize,
        is_focused: bool,
        placeholder: Option<photoncast_extension_api::RString>,
        colors: &ExtensionViewColors,
    ) -> gpui::Div {
        let before_cursor = &text[..cursor_pos.min(text.len())];
        let after_cursor = &text[cursor_pos.min(text.len())..];
        let show_cursor = is_focused && self.cursor_visible();
        let is_empty = text.is_empty();

        div()
            .w_full()
            .h(px(40.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .rounded(BORDER_RADIUS)
            .border_1()
            .border_color(if is_focused {
                colors.border_focused
            } else {
                colors.border
            })
            .bg(colors.surface)
            .text_sm()
            .text_color(if is_empty {
                colors.text_placeholder
            } else {
                colors.text
            })
            .child(if is_empty && !is_focused {
                div().child(placeholder.map(|s| s.to_string()).unwrap_or_default())
            } else {
                div()
                    .flex()
                    .items_center()
                    .child(before_cursor.to_string())
                    .when(show_cursor, |el| {
                        el.child(div().w(px(1.0)).h(px(16.0)).bg(colors.accent))
                    })
                    .child(after_cursor.to_string())
            })
    }

    /// Renders dropdown options.
    fn render_dropdown_options(
        &self,
        options: &RVec<DropdownOption>,
        colors: &ExtensionViewColors,
    ) -> impl IntoElement {
        div()
            .w_full()
            .mt(px(4.0))
            .rounded(BORDER_RADIUS)
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .shadow_md()
            .overflow_hidden()
            .children(options.iter().enumerate().map(|(idx, option)| {
                let is_selected = idx == self.dropdown_index;
                div()
                    .w_full()
                    .h(px(36.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .when(is_selected, |el| el.bg(colors.selection))
                    .hover(|el| el.bg(colors.hover))
                    .text_sm()
                    .text_color(colors.text)
                    .child(option.label.to_string())
                    .into_any_element()
            }))
    }

    /// Renders the submit button.
    fn render_submit_button(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        let label = self.form_view.submit.label.to_string();
        let shortcut = self
            .form_view
            .submit
            .shortcut
            .clone()
            .into_option()
            .unwrap_or_else(|| "⌘⏎".into());

        div()
            .w_full()
            .p(PADDING)
            .border_t_1()
            .border_color(colors.border)
            .flex()
            .justify_end()
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(BORDER_RADIUS)
                    .bg(colors.accent)
                    .text_color(gpui::white())
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(|el| el.bg(colors.accent_hover))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(label)
                    .child(div().text_xs().opacity(0.7).child(shortcut.to_string())),
            )
    }
}

impl FocusableView for ExtensionFormView {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ExtensionFormView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = ExtensionViewColors::from_context(cx);

        div()
            .key_context("ExtensionFormView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::next_field))
            .on_action(cx.listener(Self::previous_field))
            .on_action(cx.listener(Self::toggle_dropdown))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, cx| {
                // Handle text input for text fields
                if let Some(field) = this.focused_field() {
                    let field_id = field.id.to_string();
                    let is_text_field = matches!(
                        field.field_type,
                        FieldType::TextField
                            | FieldType::TextArea { .. }
                            | FieldType::Password
                            | FieldType::Number { .. }
                    );

                    if is_text_field && !this.dropdown_open {
                        let key = &event.keystroke.key;

                        if key.len() == 1 && !event.keystroke.modifiers.platform {
                            let ch = key.chars().next().unwrap();
                            let cursor_pos =
                                this.cursor_positions.get(&field_id).copied().unwrap_or(0);

                            match this.values.get_mut(&field_id) {
                                Some(FieldValue::Text(text)) => {
                                    text.insert(cursor_pos, ch);
                                    this.cursor_positions.insert(field_id, cursor_pos + 1);
                                },
                                Some(FieldValue::Number(_)) => {
                                    if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                                        let text = match this.values.get(&field_id) {
                                            Some(FieldValue::Number(n)) => {
                                                if *n == 0.0 {
                                                    String::new()
                                                } else {
                                                    n.to_string()
                                                }
                                            },
                                            _ => String::new(),
                                        };
                                        let mut text = text;
                                        text.insert(cursor_pos, ch);
                                        if let Ok(n) = text.parse::<f64>() {
                                            this.values.insert(field_id.clone(), FieldValue::Number(n));
                                            this.cursor_positions.insert(field_id, cursor_pos + 1);
                                        }
                                    }
                                },
                                _ => {},
                            }

                            this.reset_cursor_blink();
                            cx.notify();
                        } else if key == "backspace" {
                            let cursor_pos =
                                this.cursor_positions.get(&field_id).copied().unwrap_or(0);
                            if cursor_pos > 0 {
                                match this.values.get_mut(&field_id) {
                                    Some(FieldValue::Text(text)) => {
                                        text.remove(cursor_pos - 1);
                                        this.cursor_positions.insert(field_id, cursor_pos - 1);
                                    },
                                    Some(FieldValue::Number(_)) => {
                                        let text = match this.values.get(&field_id) {
                                            Some(FieldValue::Number(n)) => n.to_string(),
                                            _ => String::new(),
                                        };
                                        let mut text = text;
                                        if cursor_pos <= text.len() {
                                            text.remove(cursor_pos - 1);
                                            let n = text.parse::<f64>().unwrap_or(0.0);
                                            this.values.insert(field_id.clone(), FieldValue::Number(n));
                                            this.cursor_positions.insert(field_id, cursor_pos - 1);
                                        }
                                    },
                                    _ => {},
                                }
                                this.reset_cursor_blink();
                                cx.notify();
                            }
                        } else if key == "left" {
                            let cursor_pos =
                                this.cursor_positions.get(&field_id).copied().unwrap_or(0);
                            if cursor_pos > 0 {
                                this.cursor_positions.insert(field_id, cursor_pos - 1);
                                this.reset_cursor_blink();
                                cx.notify();
                            }
                        } else if key == "right" {
                            let cursor_pos =
                                this.cursor_positions.get(&field_id).copied().unwrap_or(0);
                            let max_pos = match this.values.get(&field_id) {
                                Some(FieldValue::Text(s)) => s.len(),
                                Some(FieldValue::Number(n)) => n.to_string().len(),
                                _ => 0,
                            };
                            if cursor_pos < max_pos {
                                this.cursor_positions.insert(field_id, cursor_pos + 1);
                                this.reset_cursor_blink();
                                cx.notify();
                            }
                        }
                    }
                }
            }))
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
                    .child(self.form_view.title.to_string()),
            )
            // Description
            .when_some(
                self.form_view.description.clone().into_option(),
                |el, desc| {
                    el.child(
                        div()
                            .px(PADDING)
                            .py(px(8.0))
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child(desc.to_string()),
                    )
                },
            )
            // Fields
            .child(
                div()
                    .id("form-fields")
                    .flex_1()
                    .overflow_y_scroll()
                    .p(PADDING)
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .children(self.form_view.fields.iter().enumerate().map(|(idx, field)| {
                        self.render_field(field, idx, &colors).into_any_element()
                    })),
            )
            // Submit button
            .child(self.render_submit_button(&colors))
    }
}
