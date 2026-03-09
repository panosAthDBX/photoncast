//! Result list rendering for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Render the results list component with grouping
    pub(super) fn render_results(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        if let SearchMode::Calendar {
            title,
            events,
            error,
        } = &self.search.mode
        {
            if let Some(message) = error {
                return div()
                    .id("results-list-calendar")
                    .w_full()
                    .py_4()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(self.render_group_header(ResultType::Command, &colors))
                    .child(
                        div()
                            .px_4()
                            .py_1()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child(message.clone()),
                    );
            }

            if events.is_empty() {
                return div()
                    .id("results-list-calendar")
                    .w_full()
                    .py_4()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(self.render_group_header(ResultType::Command, &colors))
                    .child(
                        div()
                            .px_4()
                            .py_1()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child("No events found"),
                    );
            }

            let selected = self.search.selected_index;
            let now = photoncast_calendar::chrono::Local::now();
            let today = now.date_naive();
            let tomorrow = today + photoncast_calendar::chrono::Duration::days(1);

            // Group events by day and build elements with day headers
            let mut elements: Vec<gpui::AnyElement> = Vec::new();
            let mut current_day: Option<photoncast_calendar::chrono::NaiveDate> = None;
            for (item_index, event) in events.iter().enumerate() {
                let event_day = event.start.date_naive();

                // Add day header if day changed
                if current_day != Some(event_day) {
                    current_day = Some(event_day);
                    let day_label = if event_day == today {
                        "Today".to_string()
                    } else if event_day == tomorrow {
                        "Tomorrow".to_string()
                    } else {
                        event_day.format("%A, %B %d").to_string()
                    };
                    elements.push(
                        div()
                            .id(SharedString::from(format!("day-header-{}", event_day)))
                            .w_full()
                            .px_4()
                            .pt_3()
                            .pb_1()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text_muted)
                                    .child(day_label),
                            )
                            .into_any_element(),
                    );
                }

                let is_selected = item_index == selected;
                let has_conference = event.conference_url.is_some();

                // Build time string with relative time
                let time_str = if event.is_all_day {
                    "All day".to_string()
                } else {
                    format!(
                        "{} - {}",
                        event.start.format("%H:%M"),
                        event.end.format("%H:%M")
                    )
                };

                // Calculate relative time
                let relative_time = if event.is_happening_now() {
                    Some("now".to_string())
                } else if event.starts_within_minutes(5) {
                    Some("in 5 min".to_string())
                } else if event.starts_within_minutes(60) {
                    let duration = event.start.signed_duration_since(now);
                    let mins = duration.num_minutes();
                    Some(format!("in {mins} min"))
                } else {
                    None
                };

                // Parse calendar color (hex string like "#0088FF")
                let cal_color = Self::parse_hex_color(&event.calendar_color);

                let event_element = div()
                    .id(SharedString::from(format!("cal-event-{item_index}")))
                    .min_h(px(52.0))
                    .w_full()
                    .px_4()
                    .py_1()
                    .flex()
                    .items_center()
                    .gap_3()
                    .rounded(px(6.0))
                    .mx(px(4.0))
                    .bg(if is_selected {
                        colors.selection
                    } else {
                        gpui::transparent_black()
                    })
                    // Calendar color dot
                    .child(
                        div()
                            .size(px(8.0))
                            .rounded(px(4.0))
                            .bg(cal_color)
                            .flex_shrink_0(),
                    )
                    // Video icon for conference meetings
                    .child(
                        div()
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.0))
                            .text_size(px(18.0))
                            .child(if has_conference { "📹" } else { "📅" }),
                    )
                    // Event details
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .overflow_hidden()
                            // Title row
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.text)
                                            .truncate()
                                            .child(event.title.clone()),
                                    )
                                    .when_some(relative_time.clone(), |el, rt| {
                                        // Use themed colors for time indicators
                                        let color = if rt == "now" {
                                            colors.success
                                        } else {
                                            colors.warning
                                        };
                                        el.child(
                                            div()
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(color)
                                                .flex_shrink_0()
                                                .child(rt),
                                        )
                                    }),
                            )
                            // Time and calendar row
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(colors.text_muted)
                                            .child(time_str),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(colors.text_placeholder)
                                            .child(format!("· {}", event.calendar_name)),
                                    ),
                            )
                            // Location row (if present)
                            .when(event.location.is_some(), {
                                let text_placeholder = colors.text_placeholder;
                                move |el| {
                                    let loc = event.location.clone().unwrap_or_default();
                                    // Don't show location if it's a URL (conference link)
                                    if !loc.starts_with("http") && !loc.is_empty() {
                                        el.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(4.0))
                                                .child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(text_placeholder)
                                                        .child("📍"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(text_placeholder)
                                                        .truncate()
                                                        .child(loc),
                                                ),
                                        )
                                    } else {
                                        el
                                    }
                                }
                            }),
                    )
                    // Join hint on right side
                    .when(has_conference && is_selected, {
                        let accent = colors.accent;
                        move |el| {
                            el.child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(accent)
                                    .flex_shrink_0()
                                    .child("↵ join"),
                            )
                        }
                    });

                elements.push(event_element.into_any_element());
            }

            // Calculate height based on items + headers
            let num_headers = {
                let mut days = std::collections::HashSet::new();
                for e in events {
                    days.insert(e.start.date_naive());
                }
                days.len()
            };
            let header_height = num_headers as f32 * 28.0;
            let items_height = events.len() as f32 * 56.0; // Slightly taller items
            let total_height =
                (header_height + items_height).min((MAX_VISIBLE_RESULTS as f32 * 56.0) + 56.0);

            return div()
                .id("results-list-calendar")
                .w_full()
                .h(px(total_height))
                .overflow_y_scroll()
                .track_scroll(&self.results_scroll_handle)
                .child(self.render_group_header(ResultType::Command, &colors))
                .child(
                    div()
                        .px_4()
                        .py_1()
                        .text_size(px(12.0))
                        .text_color(colors.text_muted)
                        .child(title.clone()),
                )
                .children(elements);
        }

        // Check if we're showing suggestions (query is empty)
        let is_suggestions = self.search.query.is_empty() && !self.search.suggestions.is_empty();

        // Group results by type, counting groups during the single pass.
        let mut current_type: Option<ResultType> = None;
        let mut elements: Vec<gpui::AnyElement> = Vec::new();
        let mut shown_suggestions_header = false;
        let mut group_count: usize = 0;

        for (idx, result) in self.search.results.iter().enumerate() {
            // Add group header when type changes
            if current_type != Some(result.result_type) {
                current_type = Some(result.result_type);
                group_count += 1;

                // Show "Suggestions" header instead of type when showing suggestions
                if is_suggestions && !shown_suggestions_header {
                    shown_suggestions_header = true;
                    elements.push(self.render_suggestions_header(&colors).into_any_element());
                } else if !is_suggestions {
                    elements.push(
                        self.render_group_header(result.result_type, &colors)
                            .into_any_element(),
                    );
                }
            }

            let is_selected = idx == self.search.selected_index;
            elements.push(
                self.render_result_item(result, idx, is_selected, cx)
                    .into_any_element(),
            );
        }

        // Calculate height: items + group headers (24px each)
        let result_count = self.search.results.len().min(MAX_VISIBLE_RESULTS);
        let group_header_height = 24.0;
        let total_height = (result_count as f32 * RESULT_ITEM_HEIGHT.0)
            + (group_count as f32 * group_header_height);

        div()
            .id("results-list")
            .w_full()
            .h(px(total_height))
            .overflow_y_scroll()
            .track_scroll(&self.results_scroll_handle)
            .children(elements)
    }

    /// Render a group header (e.g., "Apps", "Commands")
    pub(super) fn render_group_header(
        &self,
        result_type: ResultType,
        colors: &LauncherColors,
    ) -> impl IntoElement {
        div()
            .h(px(24.0))
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_muted)
                    .child(result_type.display_name().to_uppercase()),
            )
    }

    /// Render a "Suggestions" header for empty query state
    pub(super) fn render_suggestions_header(&self, colors: &LauncherColors) -> impl IntoElement {
        div()
            .h(px(24.0))
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_muted)
                    .child("SUGGESTIONS"),
            )
    }

    /// Render the icon for a result item with status indicators
    pub(super) fn render_icon(&self, result: &ResultItem) -> impl IntoElement {
        let icon_size = ICON_SIZE_LG;

        // Check if app is running and has auto-quit enabled
        let is_running = result
            .bundle_id
            .as_ref()
            .is_some_and(|id| photoncast_apps::is_app_running(id));
        let has_auto_quit = result
            .bundle_id
            .as_ref()
            .is_some_and(|id| self.auto_quit.manager.read().is_auto_quit_enabled(id));

        div()
            .relative()
            .size(icon_size)
            .child(
                // Main icon
                div()
                    .size(icon_size)
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .rounded(px(6.0))
                    .map(|el| {
                        if let Some(icon_path) = &result.icon_path {
                            // Use the actual app icon
                            el.child(
                                img(icon_path.clone())
                                    .size(icon_size)
                                    .object_fit(ObjectFit::Contain),
                            )
                        } else {
                            // Fall back to emoji
                            el.text_size(TEXT_SIZE_LG)
                                .child(result.icon_emoji.clone())
                        }
                    }),
            )
            // Task 7.1: Running app indicator (8px green dot, bottom-right)
            .when(is_running, |el| {
                el.child(
                    div()
                        .absolute()
                        .bottom(px(-2.0))
                        .right(px(-2.0))
                        .size(px(8.0))
                        .rounded_full()
                        .bg(hsla(120.0 / 360.0, 1.0, 0.5, 1.0)) // Green #00FF00
                        .border_1()
                        .border_color(hsla(0.0, 0.0, 0.1, 1.0)), // Dark border
                )
            })
            // Task 7.2: Auto Quit indicator
            .when(has_auto_quit, |el| {
                let offset = if is_running { px(-10.0) } else { px(-2.0) };
                el.child(
                    div()
                        .absolute()
                        .bottom(offset)
                        .right(px(-2.0))
                        .size(px(6.0))
                        .rounded_full()
                        .bg(hsla(30.0 / 360.0, 1.0, 0.5, 1.0)) // Orange
                        .border_1()
                        .border_color(hsla(0.0, 0.0, 0.1, 1.0)), // Dark border
                )
            })
    }

    /// Render a single result item
    pub(super) fn render_result_item(
        &self,
        result: &ResultItem,
        index: usize,
        is_selected: bool,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let selected_bg = colors.selection;
        let hover_bg = colors.surface_hover;
        div()
            .id(("result-item", index))
            .h(RESULT_ITEM_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .when(is_selected, move |el| el.bg(selected_bg))
            .hover(move |el| el.bg(hover_bg))
            .cursor_pointer()
            .child(self.render_icon(result))
            .child(
                // Title and subtitle
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text)
                            .truncate()
                            .child(result.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .truncate()
                            .child(result.subtitle.clone()),
                    ),
            )
            // Permissions required badge
            .when(result.requires_permissions, |el| {
                el.child(
                    div()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(hsla(0.08, 0.7, 0.5, 0.15))
                        .text_size(px(10.0))
                        .text_color(hsla(0.08, 0.7, 0.5, 1.0))
                        .child("🔐 Grant Access"),
                )
            })
            .child({
                // Shortcut badge
                let has_meeting =
                    self.search.query.is_empty() && self.meeting.next_meeting.is_some();
                let shortcut_num = if has_meeting { index + 2 } else { index + 1 };
                div()
                    .text_size(px(12.0))
                    .text_color(colors.text_placeholder)
                    .when(shortcut_num <= 9, |el| {
                        el.child(format!("⌘{}", shortcut_num))
                    })
            })
    }

    /// Render empty state hint when nothing to show
    pub(super) fn render_empty_state(&self, colors: &LauncherColors) -> AnyElement {
        // Show loading indicator during file search
        if self.file_search.loading && matches!(self.search.mode, SearchMode::FileSearch) {
            return div()
                .w_full()
                .py_4()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(colors.text_muted)
                        .child("Searching..."),
                )
                .into_any_element();
        }

        // For non-normal modes, show simple hints
        if !matches!(self.search.mode, SearchMode::Normal) {
            let (message, hints) = match &self.search.mode {
                SearchMode::FileSearch => (
                    "Type at least 2 characters to search files".to_string(),
                    "↵ Open  ⌘↵ Reveal  ⌘Y Quick Look  esc Exit",
                ),
                SearchMode::Calendar { error, .. } => (
                    error
                        .as_ref()
                        .map_or("No events found", |msg| msg.as_str())
                        .to_string(),
                    "esc Back to search",
                ),
                SearchMode::Normal => unreachable!(),
            };

            return div()
                .w_full()
                .py_4()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(colors.text_muted)
                        .child(message),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(colors.text_placeholder)
                        .child(hints),
                )
                .into_any_element();
        }

        // Default hint when no meeting, no suggestions
        div()
            .w_full()
            .py_4()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(colors.text_muted)
                    .child("Type to search apps and commands"),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(colors.text_placeholder)
                    .child("↑↓ Navigate  ↵ Open  esc Close"),
            )
            .into_any_element()
    }

    /// Render the suggestions section
    #[allow(dead_code)]
    pub(super) fn render_suggestions(&self, colors: &LauncherColors) -> impl IntoElement {
        let surface_hover = colors.surface_hover;
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;
        let items: Vec<_> = self
            .search
            .suggestions
            .iter()
            .take(6)
            .map(|result| {
                let icon_path = match &result.icon {
                    IconSource::FileIcon { path } => Self::get_app_icon_path(path),
                    IconSource::AppIcon { icon_path, .. } => {
                        if icon_path.is_some() {
                            icon_path.clone()
                        } else {
                            match &result.action {
                                SearchAction::LaunchApp { path, .. } => {
                                    Self::get_cached_icon_path(path)
                                },
                                _ => None,
                            }
                        }
                    },
                    _ => None,
                };

                div()
                    .id(SharedString::from(result.id.to_string()))
                    .w(px(72.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.0))
                    .py(px(8.0))
                    .px(px(4.0))
                    .rounded(px(8.0))
                    .cursor_pointer()
                    .hover(move |s| s.bg(surface_hover))
                    .child(
                        div()
                            .size(px(40.0))
                            .rounded(px(8.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .overflow_hidden()
                            .map(|el| {
                                if let Some(icon) = &icon_path {
                                    el.child(
                                        img(icon.clone())
                                            .size(px(40.0))
                                            .object_fit(ObjectFit::Contain),
                                    )
                                } else {
                                    el.text_size(TEXT_SIZE_LG).child("📱")
                                }
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(text_muted)
                            .truncate()
                            .max_w(px(68.0))
                            .child(result.title.clone()),
                    )
            })
            .collect();

        div()
            .w_full()
            .px(px(8.0))
            .pt(px(4.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            // Section header
            .child(
                div()
                    .px(px(8.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text_placeholder)
                    .child("SUGGESTIONS"),
            )
            // App grid
            .child(div().flex().flex_wrap().justify_start().children(items))
    }

    /// Render "no results" state
    pub(super) fn render_no_results(&self, colors: &LauncherColors) -> impl IntoElement + '_ {
        let (message, hint) = match &self.search.mode {
            SearchMode::Calendar { error, .. } => {
                let msg = error
                    .as_ref()
                    .map_or("No events found", |msg| msg.as_str())
                    .to_string();
                (msg, "esc Back to search")
            },
            SearchMode::FileSearch => {
                if self.file_search.loading {
                    ("Searching...".to_string(), "")
                } else {
                    (
                        format!("No files found for \"{}\"", self.search.query),
                        "↵ Open  ⌘↵ Reveal  ⌘Y Quick Look  esc Exit",
                    )
                }
            },
            SearchMode::Normal => (
                format!("No results for \"{}\"", self.search.query),
                "Try a different search term",
            ),
        };

        div()
            .w_full()
            .py_4()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(colors.text_muted)
                    .child(message),
            )
            .when(!hint.is_empty(), |el| {
                el.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(colors.text_placeholder)
                        .child(hint),
                )
            })
    }
}
