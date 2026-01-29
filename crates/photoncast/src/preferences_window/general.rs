//! General settings section of the Preferences window.

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
                .on_click(cx.listener(|this, _, cx| this.show_restart_dialog(cx))),
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
            // Restart Confirmation Dialog
            .when(self.show_restart_dialog, |el| {
                el.child(self.render_restart_dialog(cx))
            })
    }

    /// Shows the restart confirmation dialog when dock visibility is toggled
    fn show_restart_dialog(&mut self, cx: &mut ViewContext<Self>) {
        // Toggle the pending value (don't save yet)
        let new_value = !self.config.general.show_in_dock;
        self.pending_dock_visibility = Some(new_value);
        self.show_restart_dialog = true;
        cx.notify();
    }

    /// Handles "Restart Later" button - dismisses dialog without restarting
    fn dismiss_restart_dialog(&mut self, cx: &mut ViewContext<Self>) {
        // Save the config change but don't restart
        if let Some(new_value) = self.pending_dock_visibility.take() {
            self.config.general.show_in_dock = new_value;
            self.has_changes = true;
            self.save_config();
        }
        self.show_restart_dialog = false;
        cx.notify();
    }

    /// Handles "Restart Now" button - saves config and restarts the app
    fn restart_app_now(&mut self, cx: &mut ViewContext<Self>) {
        // Save the config change
        if let Some(new_value) = self.pending_dock_visibility.take() {
            self.config.general.show_in_dock = new_value;
            self.has_changes = true;
            self.save_config();
        }

        // Close the preferences window
        cx.remove_window();

        // Trigger app restart
        cx.spawn(|_, _| async {
            // Use AppleScript to relaunch the app
            let script = r#"tell application "PhotonCast" to quit
delay 0.5
tell application "PhotonCast" to activate"#;

            let _ = std::process::Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output();
        })
        .detach();
    }

    /// Renders the restart confirmation dialog
    fn render_restart_dialog(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);

        div()
            .id("restart-dialog-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(colors.overlay)
            .child(
                div()
                    .id("restart-dialog")
                    .w(px(360.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .bg(colors.surface)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_lg()
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
                                            .bg(colors.surface_hover)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                div()
                                                    .text_size(px(20.0))
                                                    .child("🔄"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(15.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(colors.text)
                                            .child("Restart Required"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(colors.text_muted)
                                    .child("This change requires a restart to take effect."),
                            ),
                    )
                    // Action buttons
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .pt_2()
                            .child(
                                div()
                                    .id("restart-later-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(colors.surface_hover)
                                    .border_1()
                                    .border_color(colors.border)
                                    .hover(|s| s.bg(colors.surface))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.dismiss_restart_dialog(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.text)
                                            .child("Restart Later"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("restart-now-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(colors.accent)
                                    .hover(|s| s.bg(colors.accent_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.restart_app_now(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.text)
                                            .child("Restart Now"),
                                    ),
                            ),
                    ),
            )
    }
}
