use super::*;

impl PreferencesWindow {
    pub(super) fn render_general_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // Launch at Login
            .child(
                self.render_toggle_row(
                    "launch_at_login",
                    "Launch at Login",
                    "Start PhotonCast when you log in",
                    self.config.general.launch_at_login,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_launch_at_login(cx))),
            )
            // Show in Menu Bar
            .child(
                self.render_toggle_row(
                    "show_in_menu_bar",
                    "Show in Menu Bar",
                    "Display PhotonCast icon in the menu bar",
                    self.config.general.show_in_menu_bar,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_show_in_menu_bar(cx))),
            )
            // Show in Dock
            .child(
                self.render_toggle_row(
                    "show_in_dock",
                    "Show in Dock",
                    "Display PhotonCast icon in the Dock (requires restart)",
                    self.config.general.show_in_dock,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_show_in_dock(cx))),
            )
            // Max Results
            .child(self.render_number_row(
                "Max Results",
                "Maximum search results to display",
                self.config.general.max_results,
                cx,
                |this, cx| this.decrement_max_results(cx),
                |this, cx| this.increment_max_results(cx),
            ))
    }
}
