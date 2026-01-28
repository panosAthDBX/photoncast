use super::*;

impl PreferencesWindow {
    pub(super) fn render_window_management_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // Enable Window Management
            .child(
                self.render_toggle_row(
                    "wm_enabled",
                    "Enable Window Management",
                    "Enable keyboard shortcuts for window positioning",
                    self.config.window_management.enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_management_enabled(cx))),
            )
            // Window Gap
            .child(self.render_number_row_with_suffix(
                "Window Gap",
                "Gap between windows and screen edges",
                self.config.window_management.window_gap as usize,
                "px",
                cx,
                |this, cx| this.decrement_window_gap(cx),
                |this, cx| this.increment_window_gap(cx),
            ))
            // Cycling
            .child(
                self.render_toggle_row(
                    "wm_cycling",
                    "Enable Size Cycling",
                    "Cycle through sizes when pressing same shortcut repeatedly",
                    self.config.window_management.cycling_enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_cycling(cx))),
            )
            // Almost Maximize Margin
            .child(self.render_number_row_with_suffix(
                "Almost Maximize Margin",
                "Margin from screen edges for 'Almost Maximize' layout",
                self.config.window_management.almost_maximize_margin as usize,
                "px",
                cx,
                |this, cx| this.decrement_almost_maximize_margin(cx),
                |this, cx| this.increment_almost_maximize_margin(cx),
            ))
            // Window Animation
            .child(
                self.render_toggle_row(
                    "wm_animation",
                    "Window Animation",
                    "Animate window resizing transitions",
                    self.config.window_management.animation_enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_management_animation(cx))),
            )
            // Visual Feedback
            .child(
                self.render_toggle_row(
                    "wm_visual_feedback",
                    "Visual Feedback",
                    "Show highlight overlay when positioning windows",
                    self.config.window_management.show_visual_feedback,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_visual_feedback(cx))),
            )
            // Info about window layouts
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Available Layouts"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Configure keyboard shortcuts for window layouts"),
                    )
                    .child(
                        div()
                            .p(px(12.0))
                            .rounded(px(6.0))
                            .bg(colors.surface)
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(self.render_layout_item("Left/Right Half", "Position window on screen half", &colors))
                            .child(self.render_layout_item("Top/Bottom Half", "Position window vertically", &colors))
                            .child(self.render_layout_item("Quarters", "Position window in screen corners", &colors))
                            .child(self.render_layout_item("Thirds", "Split screen into thirds", &colors))
                            .child(self.render_layout_item("Maximize/Center", "Full screen or centered", &colors)),
                    ),
            )
            // Keyboard shortcuts note
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .text_size(TEXT_SIZE_SM)
                            .text_color(colors.text_muted)
                            .child("Configure window management shortcuts in the Keyboard Shortcuts section. Supports Hyper key (⌘⌃⌥⇧)."),
                    ),
            )
    }

    fn render_layout_item(
        &self,
        name: &'static str,
        description: &'static str,
        colors: &PrefsColors,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
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
            )
    }
}
