use super::*;

impl PreferencesWindow {
    pub(super) fn render_app_management_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // Uninstaller section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Uninstaller"),
                    )
                    .child(
                        self.render_toggle_row(
                            "deep_scan_default",
                            "Deep Scan by Default",
                            "Scan for related files when uninstalling apps",
                            self.config.app_management.deep_scan_default,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_deep_scan_default(cx))),
                    ),
            )
            // App Sleep section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("App Sleep"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Automatically stop apps after a period of inactivity"),
                    )
                    .child(
                        self.render_toggle_row(
                            "app_sleep_enabled",
                            "Enable App Sleep",
                            "Stop idle apps to save system resources",
                            self.config.app_management.app_sleep.enabled,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_app_sleep_enabled(cx))),
                    )
                    .child(self.render_number_row_with_suffix(
                        "Idle Timeout",
                        "Minutes of inactivity before stopping an app",
                        self.config.app_management.app_sleep.default_idle_minutes as usize,
                        "min",
                        cx,
                        |this, cx| this.decrement_app_sleep_idle_minutes(cx),
                        |this, cx| this.increment_app_sleep_idle_minutes(cx),
                    )),
            )
            // Info about force quit
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(TEXT_SIZE_SM)
                            .text_color(colors.text)
                            .child("Force Quit & Uninstall"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Use the launcher to search for \"Force Quit\" or \"Uninstall\" commands to manage running apps."),
                    ),
            )
    }
}
