use super::*;

impl PreferencesWindow {
    pub(super) fn render_sleep_timer_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // Enable Sleep Timer
            .child(
                self.render_toggle_row(
                    "sleep_timer_enabled",
                    "Enable Sleep Timer",
                    "Allow scheduling sleep, shutdown, restart, and lock actions",
                    self.config.sleep_timer.enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_sleep_timer_enabled(cx))),
            )
            // Show in Menu Bar
            .child(
                self.render_toggle_row(
                    "sleep_timer_menu_bar",
                    "Show in Menu Bar",
                    "Display countdown in the menu bar when timer is active",
                    self.config.sleep_timer.show_in_menu_bar,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_sleep_timer_menu_bar(cx))),
            )
            // Warning Minutes
            .child(self.render_number_row_with_suffix(
                "Warning Time",
                "Minutes before action to show warning notification",
                self.config.sleep_timer.warning_minutes as usize,
                "min",
                cx,
                |this, cx| this.decrement_warning_minutes(cx),
                |this, cx| this.increment_warning_minutes(cx),
            ))
            // Supported actions info
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Supported Actions"),
                    )
                    .child(
                        div()
                            .p(px(12.0))
                            .rounded(px(6.0))
                            .bg(colors.surface)
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(self.render_timer_action_item("💤", "Sleep", "Put your Mac to sleep", &colors))
                            .child(self.render_timer_action_item("🔌", "Shut Down", "Turn off your Mac", &colors))
                            .child(self.render_timer_action_item("🔄", "Restart", "Restart your Mac", &colors))
                            .child(self.render_timer_action_item("🔒", "Lock", "Lock your screen", &colors)),
                    ),
            )
    }

    fn render_timer_action_item(
        &self,
        icon: &'static str,
        name: &'static str,
        description: &'static str,
        colors: &PrefsColors,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(div().text_size(px(14.0)).child(icon))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(TEXT_SIZE_SM)
                            .text_color(colors.text)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(colors.text_muted)
                            .child(description),
                    ),
            )
    }
}
