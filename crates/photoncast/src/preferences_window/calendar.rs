//! Calendar integration settings section.

use super::*;

impl PreferencesWindow {
    pub(super) fn render_calendar_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // Enable Calendar
            .child(
                self.render_toggle_row(
                    "calendar_enabled",
                    "Enable Calendar Integration",
                    "Show calendar events and meeting information",
                    self.config.calendar.enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_calendar_enabled(cx))),
            )
            // Days Ahead
            .child(self.render_number_row_with_suffix(
                "Days Ahead",
                "Number of days to show upcoming events",
                self.config.calendar.days_ahead as usize,
                "days",
                cx,
                |this, cx| this.decrement_days_ahead(cx),
                |this, cx| this.increment_days_ahead(cx),
            ))
            // Show All-Day First
            .child(
                self.render_toggle_row(
                    "show_all_day_first",
                    "Show All-Day Events First",
                    "Display all-day events at the top of each day",
                    self.config.calendar.show_all_day_first,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_show_all_day_first(cx))),
            )
            // Info about permissions
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
                            .child("Calendar Access"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("PhotonCast requires calendar access permission to display your events. This will be requested when you first use a calendar command."),
                    ),
            )
    }
}
