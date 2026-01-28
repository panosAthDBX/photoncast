//! Calendar-related rendering for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Render the next meeting widget at the top of the launcher
    pub(super) fn render_next_meeting(
        &self,
        meeting: &photoncast_calendar::CalendarEvent,
        colors: &LauncherColors,
    ) -> impl IntoElement {
        let now = photoncast_calendar::chrono::Local::now();
        let time_until = meeting.start.signed_duration_since(now);

        // Format time display
        let time_str = if time_until.num_minutes() < 0 {
            // Meeting in progress
            "Now".to_string()
        } else if time_until.num_minutes() < 60 {
            format!("in {} min", time_until.num_minutes())
        } else if time_until.num_hours() < 24 {
            meeting.start.format("%H:%M").to_string()
        } else {
            meeting.start.format("%a %H:%M").to_string()
        };

        // Check if meeting is happening now or starting soon (within 15 min)
        let is_urgent = time_until.num_minutes() <= 15;
        let is_selected = self.meeting.selected && self.search.query.is_empty();

        let bg_color = if is_selected {
            colors.selection
        } else if is_urgent {
            colors.accent.opacity(0.3) // Accent tint for urgent
        } else {
            colors.surface_hover
        };

        let has_meeting_link = meeting.conference_url.is_some();
        let text = colors.text;
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;
        let accent = colors.accent;
        let border = colors.border;

        div()
            .id("next-meeting")
            .mx(px(8.0))
            .mt(px(8.0))
            .mb(px(4.0))
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(8.0))
            .bg(bg_color)
            .border_1()
            .border_color(if is_selected { accent } else { border })
            .cursor_pointer()
            .flex()
            .items_center()
            .gap(px(12.0))
            // Calendar icon
            .child(
                div()
                    .size(ICON_SIZE_LG)
                    .rounded(px(6.0))
                    .bg(colors.accent.opacity(0.2))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(TEXT_SIZE_MD)
                    .child("📅"),
            )
            // Meeting info
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(text)
                            .truncate()
                            .child(meeting.title.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(if is_urgent { accent } else { text_muted })
                                    .child(time_str),
                            )
                            .when(has_meeting_link, move |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(text_placeholder)
                                        .child("↵ to join"),
                                )
                            }),
                    ),
            )
            // Join button (if has meeting link)
            .when(has_meeting_link, move |el| {
                el.child(
                    div()
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .bg(accent)
                        .text_size(px(11.0))
                        .text_color(text)
                        .child("Join"),
                )
            })
            // Shortcut badge
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(text_placeholder)
                    .child("⌘1"),
            )
    }
}
