//! Render methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Render the confirmation dialog overlay
    pub(super) fn render_confirmation_dialog(
        &self,
        dialog: &ConfirmationDialog,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        // Full overlay with semi-transparent background
        div()
            .id("confirmation-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            // Theme overlay color
            .bg(colors.overlay)
            .child(
                // Dialog container
                div()
                    .id("confirmation-dialog")
                    .w(px(340.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    // Theme surface elevated background
                    .bg(colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_xl()
                    // Warning icon and title
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            // Warning icon
                            .child(
                                div()
                                    .size(SEARCH_BAR_HEIGHT)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_full()
                                    // Warning background with low opacity
                                    .bg(colors.warning.opacity(0.15))
                                    .child(
                                        div()
                                            .text_size(TEXT_SIZE_LG)
                                            .child("⚠️"),
                                    ),
                            )
                            // Title
                            .child(
                                div()
                                    .text_size(TEXT_SIZE_MD)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text)
                                    .child(dialog.title.clone()),
                            ),
                    )
                    // Message
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(colors.text_muted)
                                    .child(dialog.message.clone()),
                            ),
                    )
                    // Buttons
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .mt_2()
                            // Cancel button
                            .child({
                                let hover_bg = colors.surface_hover;
                                div()
                                    .id("cancel-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(colors.surface)
                                    .hover(move |el| el.bg(hover_bg))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.text)
                                            .child(dialog.cancel_label.clone()),
                                    )
                            })
                            // Confirm button (destructive style)
                            .child({
                                let error_color = colors.error;
                                // Lighten by increasing lightness component
                                let error_hover = hsla(error_color.h, error_color.s, (error_color.l + 0.1).min(1.0), error_color.a);
                                div()
                                    .id("confirm-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(error_color)
                                    .hover(move |el| el.bg(error_hover))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(colors.text)
                                            .child(dialog.confirm_label.clone()),
                                    )
                            }),
                    )
                    // Keyboard hints
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_center()
                            .mt_1()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_placeholder)
                                    .child("↵ Confirm  esc Cancel"),
                            ),
                    ),
            )
    }
    /// Renders the extension permissions consent dialog
    pub(super) fn render_permissions_consent_dialog(
        &self,
        consent: &crate::permissions_dialog::PendingPermissionsConsent,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let dialog = &consent.dialog;
        let extension_name = dialog.extension_name.clone();
        let permissions = dialog.permissions.clone();
        let is_first_launch = consent.is_first_launch;

        let launcher_colors = get_launcher_colors(cx);

        div()
            .id("permissions-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(launcher_colors.overlay)
            .child(
                div()
                    .id("permissions-dialog")
                    .w(px(380.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .bg(launcher_colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(launcher_colors.border)
                    .shadow_xl()
                    // Header
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .size(px(40.0))
                                            .rounded(px(8.0))
                                            .bg(launcher_colors.surface)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                div()
                                                    .text_size(px(20.0))
                                                    .child("🧩"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(15.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(launcher_colors.text)
                                            .child(if is_first_launch {
                                                format!("\"{}\" wants to be enabled", extension_name)
                                            } else {
                                                format!("\"{}\" requires permissions", extension_name)
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(launcher_colors.text_muted)
                                    .child("This extension requests access to:"),
                            ),
                    )
                    // Permissions list
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .bg(launcher_colors.surface)
                            .rounded(px(8.0))
                            .children(permissions.iter().map(|perm| {
                                let name = perm.name.clone();
                                let description = perm.description.clone();
                                let accent = launcher_colors.accent;
                                let text = launcher_colors.text;
                                let text_muted = launcher_colors.text_muted;

                                div()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .py_1()
                                    .child(
                                        div()
                                            .size(ICON_SIZE_MD)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(4.0))
                                            .bg(accent.opacity(0.1))
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(accent)
                                                    .child(match perm.id {
                                                        photoncast_core::extensions::permissions::PermissionType::Network => "🌐",
                                                        photoncast_core::extensions::permissions::PermissionType::Clipboard => "📋",
                                                        photoncast_core::extensions::permissions::PermissionType::Notifications => "🔔",
                                                        photoncast_core::extensions::permissions::PermissionType::Filesystem => "📁",
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_px()
                                            .child(
                                                div()
                                                    .text_size(px(13.0))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(text)
                                                    .child(name),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(text_muted)
                                                    .child(description),
                                            ),
                                    )
                            })),
                    )
                    // Action buttons
                    .child({
                        let accept_bg = launcher_colors.accent;
                        let accept_hover = launcher_colors.accent_hover;
                        let deny_bg = launcher_colors.surface;
                        let deny_hover = launcher_colors.surface_hover;
                        let text = launcher_colors.text;
                        let border = launcher_colors.border;

                        div()
                            .flex()
                            .gap_3()
                            .pt_2()
                            .child(
                                div()
                                    .id("deny-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(deny_bg)
                                    .border_1()
                                    .border_color(border)
                                    .hover(move |s| s.bg(deny_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.deny_permissions_consent(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(text)
                                            .child("Deny"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("accept-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(accept_bg)
                                    .hover(move |s| s.bg(accept_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.accept_permissions_consent(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                            .child("Enable"),
                                    ),
                            )
                    }),
            )
    }
    /// Renders the uninstall preview dialog
    pub(super) fn render_uninstall_preview(
        &self,
        preview: &UninstallPreview,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let app_name = preview.app.name.clone();
        let space_freed = preview.space_freed_formatted.clone();

        // Group files by category
        let mut categories: std::collections::BTreeMap<
            &str,
            Vec<(usize, &photoncast_apps::RelatedFile)>,
        > = std::collections::BTreeMap::new();
        for (idx, file) in preview.related_files.iter().enumerate() {
            let category_name = file.category.display_name();
            categories
                .entry(category_name)
                .or_default()
                .push((idx, file));
        }

        // Get icon path for the app
        let icon_path = Self::get_app_icon_path(&preview.app.path);

        // Pre-build category sections to avoid borrowing cx inside nested iterators
        let category_sections: Vec<_> = categories
            .into_iter()
            .map(|(category_name, files)| {
                let text_muted = colors.text_muted;
                let text = colors.text;
                let text_placeholder = colors.text_placeholder;
                let surface = colors.surface;
                let surface_hover = colors.surface_hover;
                let accent = colors.accent;
                let border = colors.border;

                // Pre-build file items for this category
                let file_items: Vec<_> = files
                    .into_iter()
                    .map(|(idx, file)| {
                        let file_name = file
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let file_size = UninstallPreview::format_bytes(file.size_bytes);
                        let is_selected = file.selected;

                        div()
                            .id(SharedString::from(format!("uninstall-file-{}", idx)))
                            .px_3()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(surface)
                            .hover(move |el| el.bg(surface_hover))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_3()
                            .on_click(cx.listener(move |this, _, cx| {
                                this.toggle_uninstall_file_selection(idx, cx);
                            }))
                            // Checkbox
                            .child(
                                div()
                                    .size(px(18.0))
                                    .rounded(px(4.0))
                                    .border_1()
                                    .border_color(if is_selected { accent } else { border })
                                    .bg(if is_selected {
                                        accent
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(text)
                                            .when(is_selected, |el| el.child("✓")),
                                    ),
                            )
                            // File info
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(text)
                                            .truncate()
                                            .child(file_name),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(text_placeholder)
                                            .child(file_size),
                                    ),
                            )
                    })
                    .collect();

                (category_name.to_string(), text_muted, file_items)
            })
            .collect();

        div()
            .id("uninstall-preview-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(colors.overlay)
            .child(
                div()
                    .id("uninstall-preview-dialog")
                    .w(px(420.0))
                    .max_h(px(500.0))
                    .flex()
                    .flex_col()
                    .bg(colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_xl()
                    .overflow_hidden()
                    // Header with app info
                    .child(
                        div()
                            .px_5()
                            .pt_5()
                            .pb_4()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            // App icon
                            .child(
                                div()
                                    .size(px(64.0))
                                    .rounded(px(12.0))
                                    .overflow_hidden()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .map(|el| {
                                        if let Some(icon) = &icon_path {
                                            el.child(
                                                img(icon.clone())
                                                    .size(px(64.0))
                                                    .object_fit(ObjectFit::Contain),
                                            )
                                        } else {
                                            el.text_size(ICON_SIZE_LG).child("📦")
                                        }
                                    }),
                            )
                            // App name
                            .child(
                                div()
                                    .text_size(px(18.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text)
                                    .child(format!("Uninstall {}", app_name)),
                            )
                            // Space to be freed
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .text_color(colors.text_muted)
                                            .child("Space to be freed:"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.success)
                                            .child(space_freed),
                                    ),
                            ),
                    )
                    // Related files list
                    .child(
                        div()
                            .id("uninstall-files-list")
                            .flex_1()
                            .overflow_y_scroll()
                            .px_5()
                            .pb_3()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .children(
                                category_sections
                                    .into_iter()
                                    .map(|(category_name, text_muted, file_items)| {
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            // Category header
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(text_muted)
                                                    .child(category_name.to_uppercase()),
                                            )
                                            // Files in category
                                            .children(file_items)
                                    }),
                            ),
                    )
                    // Action buttons
                    .child(
                        div()
                            .px_5()
                            .py_4()
                            .border_t_1()
                            .border_color(colors.border)
                            .flex()
                            .gap_3()
                            // Cancel button
                            .child({
                                let surface = colors.surface;
                                let surface_hover = colors.surface_hover;
                                let text = colors.text;
                                div()
                                    .id("uninstall-cancel")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(surface)
                                    .hover(move |el| el.bg(surface_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.cancel_uninstall_preview(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(text)
                                            .child("Cancel"),
                                    )
                            })
                            // Keep Related Files button
                            .child({
                                let surface = colors.surface;
                                let surface_hover = colors.surface_hover;
                                let text = colors.text;
                                div()
                                    .id("uninstall-keep-files")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(surface)
                                    .hover(move |el| el.bg(surface_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.perform_uninstall_app_only(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(text)
                                            .child("Keep Files"),
                                    )
                            })
                            // Uninstall button (primary, destructive)
                            .child({
                                let error = colors.error;
                                let error_hover =
                                    hsla(error.h, error.s, (error.l + 0.1).min(1.0), error.a);
                                let text = colors.text;
                                div()
                                    .id("uninstall-confirm")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(error)
                                    .hover(move |el| el.bg(error_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.perform_uninstall(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(text)
                                            .child("Uninstall"),
                                    )
                            }),
                    ),
            )
    }

    /// Renders the auto quit settings panel
    pub(super) fn render_auto_quit_settings(
        &self,
        bundle_id: &str,
        app_name: &str,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let manager = self.auto_quit.manager.read();
        let is_enabled = manager.is_auto_quit_enabled(bundle_id);
        let current_timeout = manager
            .get_timeout_minutes(bundle_id)
            .unwrap_or(DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES);
        drop(manager);

        let timeout_options = [1, 2, 3, 5, 10, 15, 30];
        let selected_index = self.auto_quit.settings_index;

        div()
            .id("auto-quit-settings-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_end()
            .justify_end()
            .pb_2()
            .pr_2()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, cx| {
                this.close_auto_quit_settings(cx);
            }))
            .child(
                div()
                    .w(px(280.0))
                    .bg(colors.surface_elevated)
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, cx| cx.stop_propagation())
                    // Header
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(colors.border)
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text)
                                    .child("Auto Quit Settings"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_muted)
                                    .truncate()
                                    .child(app_name.to_string()),
                            ),
                    )
                    // Enable toggle
                    .child({
                        let surface = colors.surface;
                        let surface_hover = colors.surface_hover;
                        let text = colors.text;
                        let accent = colors.accent;
                        let is_toggle_selected = selected_index == 0;
                        div()
                            .id("auto-quit-toggle")
                            .px_4()
                            .py_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .cursor_pointer()
                            .bg(if is_toggle_selected { surface_hover } else { gpui::transparent_black() })
                            .hover(move |el| el.bg(surface_hover))
                            .on_click(cx.listener(|this, _, cx| {
                                this.toggle_auto_quit_in_settings(cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(text)
                                    .child("Enable Auto Quit"),
                            )
                            .child(
                                div()
                                    .w(px(36.0))
                                    .h(px(20.0))
                                    .rounded(px(10.0))
                                    .bg(if is_enabled { accent } else { surface })
                                    .relative()
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(2.0))
                                            .left(if is_enabled { px(18.0) } else { px(2.0) })
                                            .size(ICON_SIZE_SM)
                                            .rounded_full()
                                            .bg(text),
                                    ),
                            )
                    })
                    // Timeout selector (only when enabled)
                    .when(is_enabled, |el| {
                        let text_muted = colors.text_muted;
                        let text = colors.text;
                        let surface = colors.surface;
                        let surface_hover = colors.surface_hover;
                        let accent = colors.accent;
                        el.child(
                            div()
                                .px_4()
                                .py_2()
                                .border_t_1()
                                .border_color(colors.border)
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child("Quit after idle for:"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap(px(6.0))
                                        .children(timeout_options.iter().enumerate().map(move |(idx, &minutes)| {
                                            let is_current = minutes == current_timeout;
                                            let is_kb_selected = selected_index == idx + 1; // +1 because 0 is toggle
                                            div()
                                                .id(SharedString::from(format!("timeout-{}", minutes)))
                                                .px(px(10.0))
                                                .py(px(4.0))
                                                .rounded(px(4.0))
                                                .bg(if is_current { accent } else if is_kb_selected { surface_hover } else { surface })
                                                .border_1()
                                                .border_color(if is_kb_selected { accent } else { gpui::transparent_black() })
                                                .hover(move |el| el.bg(if is_current { accent } else { surface_hover }))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, cx| {
                                                    this.set_auto_quit_timeout(minutes, cx);
                                                }))
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(text)
                                                        .child(format!("{} min", minutes)),
                                                )
                                        })),
                                ),
                        )
                    })
                    // Footer hint
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .border_t_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_placeholder)
                                    .child("Auto Quit stops idle apps to save resources"),
                            ),
                    ),
            )
    }

    /// Renders the "Manage Auto Quits" view
    pub(super) fn render_manage_auto_quits(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let manager = self.auto_quit.manager.read();
        let enabled_apps: Vec<_> = manager
            .get_enabled_apps()
            .iter()
            .map(|(id, cfg)| (id.to_string(), cfg.timeout_minutes))
            .collect();
        drop(manager);

        let selected = self.auto_quit.manage_index;
        let is_empty = enabled_apps.is_empty();

        div()
            .w_full()
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text_muted)
                            .child("AUTO QUIT APPS"),
                    ),
            )
            // App list or empty state
            .when(is_empty, |el| {
                el.child(
                    div()
                        .py_6()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(colors.text_muted)
                                .child("No apps with Auto Quit enabled"),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(colors.text_placeholder)
                                .child("Use ⌘K on any app to enable Auto Quit"),
                        ),
                )
            })
            .when(!is_empty, |el| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .children(enabled_apps.iter().enumerate().map(|(idx, (bundle_id, timeout))| {
                            let is_selected = idx == selected;
                            let app_name = photoncast_apps::get_suggested_app_name(bundle_id)
                                .map(String::from)
                                .unwrap_or_else(|| {
                                    // Try to get last component of bundle ID
                                    bundle_id.split('.').next_back().unwrap_or(bundle_id).to_string()
                                });

                            let text = colors.text;
                            let text_muted = colors.text_muted;
                            let text_placeholder = colors.text_placeholder;
                            let surface = colors.surface;
                            let surface_hover = colors.surface_hover;
                            let selection = colors.selection;
                            let error = colors.error;

                            div()
                                .id(SharedString::from(format!("auto-quit-app-{}", idx)))
                                .h(px(48.0))
                                .px_4()
                                .flex()
                                .items_center()
                                .justify_between()
                                .bg(if is_selected { selection } else { gpui::transparent_black() })
                                .hover(move |el| el.bg(surface_hover))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .text_color(text)
                                                .child(app_name),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(text_muted)
                                                .child(format!("Quit after {} min idle", timeout)),
                                        ),
                                )
                                // Disable button
                                .child(
                                    div()
                                        .id(SharedString::from(format!("disable-auto-quit-{}", idx)))
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(surface)
                                        .hover(move |el| el.bg(error.opacity(0.2)))
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, cx| {
                                            this.disable_auto_quit_at_index(idx, cx);
                                        }))
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(text_placeholder)
                                                .child("Disable"),
                                        ),
                                )
                        })),
                )
            })
            // Footer with hints
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(colors.border)
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(colors.text_placeholder)
                            .child("↑↓ Navigate  esc Back"),
                    ),
            )
    }

    /// Renders the toast notification
    pub(super) fn render_toast(&self, message: &str, cx: &ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);

        // Calculate opacity based on time (fade in/out)
        let opacity = if let Some(shown_at) = self.toast.shown_at {
            let elapsed = shown_at.elapsed().as_millis() as f32;
            if elapsed < 150.0 {
                // Fade in
                elapsed / 150.0
            } else if elapsed > 1800.0 {
                // Fade out (after 1.8s, fade out over 200ms)
                1.0 - ((elapsed - 1800.0) / 200.0).min(1.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        div()
            .id("toast-notification")
            .absolute()
            .bottom(px(12.0))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .opacity(opacity)
            .child(
                div()
                    .px_4()
                    .py_2()
                    .rounded(px(8.0))
                    .bg(colors.surface_elevated)
                    .border_1()
                    .border_color(colors.border)
                    .shadow_md()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_size(TEXT_SIZE_MD).child("✓"))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child(message.to_string()),
                    ),
            )
    }
}

impl Render for LauncherWindow {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Get theme colors for the UI
        let colors = get_launcher_colors(cx);

        // Calculate current animation opacity
        let opacity = self.current_opacity();

        // Clone pending confirmation for use in the closure
        let pending_dialog = self.pending_confirmation.as_ref().map(|(_, d)| d.clone());

        // Pre-render components that need colors (for use in closures)
        let empty_state = self.render_empty_state(&colors);
        let no_results = self.render_no_results(&colors);
        let divider_color = colors.border;

        // Check if any overlay is active (need minimum height for overlays)
        let has_overlay = self.actions_menu.visible
            || self.auto_quit.settings_app.is_some()
            || self.uninstall.preview.is_some()
            || self.auto_quit.manage_mode
            || pending_dialog.is_some();

        // Main container with rounded corners and shadow
        // Note: When extension view is active, it handles its own focus and keyboard events
        div()
            .track_focus(&self.focus_handle)
            .key_context("LauncherWindow")
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::confirm_dialog))
            .on_action(cx.listener(Self::next_group))
            .on_action(cx.listener(Self::previous_group))
            .on_action(cx.listener(Self::open_preferences))
            // File Search Mode actions
            .on_action(cx.listener(Self::reveal_in_finder))
            .on_action(cx.listener(Self::quick_look))
            .on_action(cx.listener(Self::copy_path))
            .on_action(cx.listener(Self::copy_file))
            .on_action(cx.listener(Self::show_actions_menu))
            // Task 7.4: App management action handlers
            .on_action(cx.listener(Self::show_in_finder))
            .on_action(cx.listener(Self::copy_bundle_id))
            .on_action(cx.listener(Self::quit_app))
            .on_action(cx.listener(Self::force_quit_app))
            .on_action(cx.listener(Self::hide_app))
            .on_action(cx.listener(Self::uninstall_app))
            .on_action(cx.listener(Self::toggle_auto_quit_for_selected))
            .on_action(cx.listener(|this, _: &QuickSelect1, cx| this.quick_select(0, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect2, cx| this.quick_select(1, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect3, cx| this.quick_select(2, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect4, cx| this.quick_select(3, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect5, cx| this.quick_select(4, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect6, cx| this.quick_select(5, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect7, cx| this.quick_select(7, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect8, cx| this.quick_select(7, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect9, cx| this.quick_select(8, cx)))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            // Apply window appear/dismiss animation opacity
            .opacity(opacity)
            // Theme background color
            .bg(colors.background)
            .rounded(LAUNCHER_BORDER_RADIUS)
            .shadow_lg()
            .border_1()
            .border_color(colors.border)
            // Keep minimum height when overlays are visible to prevent clipping
            .when(has_overlay, |el| el.min_h(px(400.0)))
            .overflow_hidden()
            // File Search Mode: render the dedicated FileSearchView
            .when_some(self.file_search.view.clone(), |el, view| {
                el.child(
                    div()
                        .size_full() // Fill the resized window
                        .child(view)
                )
            })
            // Extension View Mode: render the extension's view
            .when_some(self.extension_view.view.clone(), |el, view| {
                el.child(
                    div()
                        .size_full()
                        .child(view)
                )
            })
            // Normal/Calendar Mode: render the standard launcher content
            .when(self.file_search.view.is_none() && self.extension_view.view.is_none(), |el| {
                el
                    // Search bar
                    .child(self.render_search_bar(cx))
                    // Content area (flex-1 to push action bar to bottom)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Next meeting widget (show when query is empty and we have a meeting)
                            .when(self.search.query.is_empty() && self.meeting.next_meeting.is_some() && !matches!(self.search.mode, SearchMode::Calendar { .. }), |el| {
                                if let Some(meeting) = &self.meeting.next_meeting {
                                    el.child(self.render_next_meeting(meeting, &colors))
                                } else {
                                    el
                                }
                            })
                            // Divider (show when there are results or in calendar mode)
                            .when(!self.search.results.is_empty() || matches!(self.search.mode, SearchMode::Calendar { .. }), move |el| {
                                el.child(div().h(px(1.0)).w_full().bg(divider_color))
                            })
                            // Empty state: Normal mode with no meeting
                            .when(self.search.query.is_empty() && self.search.results.is_empty() && matches!(self.search.mode, SearchMode::Normal) && self.meeting.next_meeting.is_none(), |el| {
                                el.child(empty_state)
                            })
                            // No results message: query entered but nothing found
                            .when(!self.search.query.is_empty() && self.search.results.is_empty() && !matches!(self.search.mode, SearchMode::Calendar { .. }), |el| {
                                el.child(no_results)
                            })
                            // Results list: show suggestions (when query empty) or search results
                            .when(!self.search.results.is_empty() || matches!(self.search.mode, SearchMode::Calendar { .. }), |el| {
                                el.child(self.render_results(cx))
                            })
                    )
                    // Action bar at bottom - always visible, pinned by flex layout
                    .child(self.render_action_bar(cx))
            })
            // Actions menu overlay (Cmd+K)
            .when(self.actions_menu.visible, |el| {
                el.child(self.render_actions_menu(cx))
            })
            // Confirmation dialog overlay
            .when_some(pending_dialog, |el, dialog| {
                el.child(self.render_confirmation_dialog(&dialog, cx))
            })
            // Extension permissions consent dialog overlay
            .when_some(self.pending_permissions_consent.clone(), |el, consent| {
                el.child(self.render_permissions_consent_dialog(&consent, cx))
            })
            // Task 7.5: Uninstall preview overlay
            .when_some(self.uninstall.preview.clone(), |el, preview| {
                el.child(self.render_uninstall_preview(&preview, cx))
            })
            // Task 7.6: Auto quit settings overlay
            .when_some(self.auto_quit.settings_app.clone(), |el, (bundle_id, app_name)| {
                el.child(self.render_auto_quit_settings(&bundle_id, &app_name, cx))
            })
            // Task 7.7: Manage auto quits view
            .when(self.auto_quit.manage_mode, |el| {
                el.child(self.render_manage_auto_quits(cx))
            })
            // Task 7.8: Toast notification
            .when_some(self.toast.message.clone(), |el, message| {
                el.child(self.render_toast(&message, cx))
            })
    }
}

impl FocusableView for LauncherWindow {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}
