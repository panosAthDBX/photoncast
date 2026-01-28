use super::*;

impl PreferencesWindow {
    pub(super) fn render_shortcuts_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let hotkey = format!(
            "{}{}",
            self.config.hotkey.modifiers.join(" "),
            if self.config.hotkey.modifiers.is_empty() {
                ""
            } else {
                " "
            },
        ) + &self.config.hotkey.key;

        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // Global Shortcuts
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Global Shortcuts"),
                    )
                    .child(self.render_shortcut_row("Toggle Launcher", &hotkey, &colors))
                    .child(self.render_shortcut_row("Clipboard History", "⌘ ⇧ V", &colors))
                    .child(self.render_shortcut_row("Quick Links", "⌘ ⇧ L", &colors)),
            )
            // Window Management Shortcuts
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Window Management"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("No default shortcuts assigned. Supports Hyper key."),
                    )
                    .child(self.render_suggested_shortcut("Left Half", "Hyper + ←", &colors))
                    .child(self.render_suggested_shortcut("Right Half", "Hyper + →", &colors))
                    .child(self.render_suggested_shortcut("Maximize", "Hyper + ↑", &colors))
                    .child(self.render_suggested_shortcut("Center", "Hyper + C", &colors)),
            )
            // Note about customization
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(TEXT_SIZE_SM)
                            .text_color(colors.text)
                            .child("Customization"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Edit shortcuts in ~/.config/photoncast/keybindings.toml"),
                    )
                    .child({
                        let surface_hover = colors.surface_hover;
                        let hover_color = colors.hover;
                        let text_color = colors.text;
                        div()
                            .id("reset-shortcuts")
                            .mt(px(4.0))
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .bg(surface_hover)
                            .hover(move |s| s.bg(hover_color))
                            .cursor_pointer()
                            .w(px(120.0))
                            .flex()
                            .justify_center()
                            .on_click(cx.listener(|_this, _, _cx| {
                                // Reset keybindings to defaults
                                if let Ok(mut keybindings) = photoncast_core::app::keybindings::Keybindings::load() {
                                    keybindings.reset_to_defaults();
                                    let _ = keybindings.save();
                                }
                            }))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_color)
                                    .child("Reset to Defaults"),
                            )
                    }),
            )
    }

    fn render_suggested_shortcut(
        &self,
        name: &'static str,
        shortcut: &'static str,
        colors: &PrefsColors,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(4.0))
            .child(
                div()
                    .text_size(TEXT_SIZE_SM)
                    .text_color(colors.text)
                    .child(name),
            )
            .child(
                div()
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .bg(colors.surface_hover)
                    .text_size(px(10.0))
                    .text_color(colors.text_muted)
                    .child(shortcut),
            )
    }
}
