use super::*;

impl PreferencesWindow {
    pub(super) fn render_clipboard_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let is_paste_default = self.config.clipboard.default_action == ClipboardAction::Paste;

        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // History Size
            .child(self.render_number_row(
                "History Size",
                "Maximum number of items to store",
                self.config.clipboard.history_size,
                cx,
                |this, cx| this.decrement_clipboard_history_size(cx),
                |this, cx| this.increment_clipboard_history_size(cx),
            ))
            // Retention Days
            .child(self.render_number_row_with_suffix(
                "Retention",
                "Number of days to keep clipboard items",
                self.config.clipboard.retention_days as usize,
                "days",
                cx,
                |this, cx| this.decrement_clipboard_retention(cx),
                |this, cx| this.increment_clipboard_retention(cx),
            ))
            // Store Images
            .child(
                self.render_toggle_row(
                    "store_images",
                    "Store Images",
                    "Save images to clipboard history",
                    self.config.clipboard.store_images,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_clipboard_store_images(cx))),
            )
            // Max Image Size
            .child(self.render_number_row_with_suffix(
                "Max Image Size",
                "Maximum size for stored images",
                (self.config.clipboard.max_image_size / (1024 * 1024)) as usize,
                "MB",
                cx,
                |this, cx| this.decrement_max_image_size(cx),
                |this, cx| this.increment_max_image_size(cx),
            ))
            // Default Action
            .child(self.render_default_action_selector(is_paste_default, cx))
            // Excluded Apps
            .child(self.render_excluded_apps_section(cx))
    }

    fn render_default_action_selector(
        &self,
        is_paste_default: bool,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Default Action"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Action when pressing Enter on a clipboard item"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id("action-paste")
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(if is_paste_default {
                                colors.accent
                            } else {
                                colors.surface
                            })
                            .hover(|s| {
                                s.bg(if is_paste_default {
                                    colors.accent
                                } else {
                                    colors.surface_hover
                                })
                            })
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, cx| {
                                if this.config.clipboard.default_action != ClipboardAction::Paste {
                                    this.toggle_clipboard_default_action(cx);
                                }
                            }))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text)
                                    .child("Paste"),
                            ),
                    )
                    .child(
                        div()
                            .id("action-copy")
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(if !is_paste_default {
                                colors.accent
                            } else {
                                colors.surface
                            })
                            .hover(|s| {
                                s.bg(if !is_paste_default {
                                    colors.accent
                                } else {
                                    colors.surface_hover
                                })
                            })
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, cx| {
                                if this.config.clipboard.default_action != ClipboardAction::Copy {
                                    this.toggle_clipboard_default_action(cx);
                                }
                            }))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text)
                                    .child("Copy"),
                            ),
                    ),
            )
    }

    fn render_excluded_apps_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let excluded_apps = self.config.clipboard.excluded_apps.clone();
        let has_defaults = !excluded_apps.is_empty();

        let app_items: Vec<_> = excluded_apps
            .iter()
            .map(|app| {
                let app_clone = app.clone();
                let display_name = app.split('.').next_back().unwrap_or(app);
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .py(px(4.0))
                    .child(
                        div()
                            .text_size(TEXT_SIZE_SM)
                            .text_color(colors.text_muted)
                            .child(display_name.to_string()),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("remove-{}", app)))
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| {
                                this.remove_excluded_app(&app_clone, cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_muted)
                                    .child("×"),
                            ),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(colors.text)
                                    .child("Excluded Apps"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_muted)
                                    .child("Apps that won't have their clipboard content saved"),
                            ),
                    )
                    .child(
                        div()
                            .id("add-defaults")
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, cx| this.add_default_excluded_apps(cx)))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text)
                                    .child("Add Defaults"),
                            ),
                    ),
            )
            .child(
                div()
                    .p(px(8.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .max_h(px(120.0))
                    .overflow_hidden()
                    .child(if has_defaults {
                        div().flex().flex_col().gap(px(2.0)).children(app_items).into_any_element()
                    } else {
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_placeholder)
                            .child("No apps excluded. Click \"Add Defaults\" to exclude password managers.")
                            .into_any_element()
                    }),
            )
    }
}
