//! Appearance settings section (theme, accent color, auto dark mode).

use super::*;

impl PreferencesWindow {
    pub(super) fn render_appearance_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // Theme selector
            .child(self.render_theme_selector(cx))
            // Accent color selector
            .child(self.render_accent_selector(cx))
            // Window Animation
            .child(
                self.render_toggle_row(
                    "window_animation",
                    "Window Animation",
                    "Enable smooth window transitions",
                    self.config.appearance.window_animation,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_animation(cx))),
            )
            // Reduce Motion
            .child(
                self.render_toggle_row(
                    "reduce_motion",
                    "Reduce Motion",
                    "Minimize animations for accessibility",
                    self.config.appearance.reduce_motion,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_reduce_motion(cx))),
            )
    }

    fn render_theme_selector(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let themes = [
            ("Auto", ThemeSetting::Auto),
            ("Latte", ThemeSetting::Latte),
            ("Frappé", ThemeSetting::Frappe),
            ("Macchiato", ThemeSetting::Macchiato),
            ("Mocha", ThemeSetting::Mocha),
        ];

        let current_theme = &self.config.appearance.theme;
        let accent = colors.accent;
        let surface_hover = colors.surface_hover;
        let hover = colors.hover;
        let text = colors.text;

        let buttons: Vec<_> = themes
            .iter()
            .map(|(name, theme)| {
                let is_selected = current_theme == theme;
                let theme = theme.clone();
                let bg = if is_selected { accent } else { surface_hover };
                let hover_bg = if is_selected { accent } else { hover };
                div()
                    .id(SharedString::from(*name))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(bg)
                    .hover(move |s| s.bg(hover_bg))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, cx| {
                        this.set_theme(theme.clone(), cx);
                    }))
                    .child(div().text_size(px(11.0)).text_color(text).child(*name))
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(colors.text)
                    .child("Theme"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(colors.text_muted)
                    .child("Choose your preferred color scheme"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(6.0))
                    .mt(px(4.0))
                    .children(buttons),
            )
    }

    fn render_accent_selector(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme_colors = get_colors(cx);
        // All 14 Catppuccin accent colors from the spec (these stay hardcoded as they ARE the palette)
        let accent_options = [
            ("Rosewater", AccentColor::Rosewater, rgb(0xf5e0dc)),
            ("Flamingo", AccentColor::Flamingo, rgb(0xf2cdcd)),
            ("Pink", AccentColor::Pink, rgb(0xf5c2e7)),
            ("Mauve", AccentColor::Mauve, rgb(0xcba6f7)),
            ("Red", AccentColor::Red, rgb(0xf38ba8)),
            ("Maroon", AccentColor::Maroon, rgb(0xeba0ac)),
            ("Peach", AccentColor::Peach, rgb(0xfab387)),
            ("Yellow", AccentColor::Yellow, rgb(0xf9e2af)),
            ("Green", AccentColor::Green, rgb(0xa6e3a1)),
            ("Teal", AccentColor::Teal, rgb(0x94e2d5)),
            ("Sky", AccentColor::Sky, rgb(0x89dceb)),
            ("Sapphire", AccentColor::Sapphire, rgb(0x74c7ec)),
            ("Blue", AccentColor::Blue, rgb(0x89b4fa)),
            ("Lavender", AccentColor::Lavender, rgb(0xb4befe)),
        ];

        let current_accent = &self.config.appearance.accent_color;
        let text = theme_colors.text;

        let buttons: Vec<_> = accent_options
            .iter()
            .map(|(name, accent, color)| {
                let is_selected = current_accent == accent;
                let accent = accent.clone();
                let color = *color;
                let border = if is_selected {
                    text
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                };
                div()
                    .id(SharedString::from(*name))
                    .size(ICON_SIZE_MD)
                    .rounded_full()
                    .bg(color)
                    .border_2()
                    .border_color(border)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, cx| {
                        this.set_accent_color(accent.clone(), cx);
                    }))
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(theme_colors.text)
                    .child("Accent Color"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme_colors.text_muted)
                    .child("Highlight color for selections and actions"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .mt(px(4.0))
                    .children(buttons),
            )
    }
}
