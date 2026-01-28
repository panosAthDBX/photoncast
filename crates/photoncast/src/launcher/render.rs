//! Render methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Render the query text with cursor and selection highlighting
    pub(super) fn render_query_with_cursor(
        &self,
        colors: &LauncherColors,
        placeholder: &str,
    ) -> impl IntoElement {
        let text_color = colors.text;
        let placeholder_color = colors.text_placeholder;
        let selection_bg = colors.accent.opacity(0.3);
        let cursor_color = colors.accent;
        let show_cursor = self.cursor_visible();

        // Block cursor dimensions (like Ghostty terminal)
        let cursor_width = px(9.0);
        let cursor_height = px(20.0);

        if self.search.query.is_empty() {
            // Show block cursor at start (no placeholder text)
            return div()
                .w_full()
                .text_size(TEXT_SIZE_MD)
                .flex()
                .items_center()
                .when(show_cursor, |el| {
                    el.child(
                        div()
                            .w(cursor_width)
                            .h(cursor_height)
                            .bg(cursor_color)
                            .rounded(px(2.0)),
                    )
                })
                .when(!placeholder.is_empty(), |el| {
                    el.child(
                        div()
                            .text_color(placeholder_color)
                            .child(placeholder.to_string()),
                    )
                });
        }

        let chars: Vec<char> = self.search.query.chars().collect();
        let (sel_start, sel_end) = self
            .selection_range()
            .unwrap_or((self.search.cursor_position, self.search.cursor_position));

        // Build the text parts: before selection, selection, after selection
        let before: String = chars[..sel_start].iter().collect();
        let selected: String = chars[sel_start..sel_end].iter().collect();
        let after: String = chars[sel_end..].iter().collect();

        let has_selection = sel_start != sel_end;
        let cursor_at_start = self.search.cursor_position == sel_start;

        div()
            .w_full()
            .text_size(TEXT_SIZE_MD)
            .text_color(text_color)
            .flex()
            .items_center()
            // Text before selection
            .when(!before.is_empty(), |el| el.child(before.clone()))
            // Cursor before selection (if selection exists and cursor is at start)
            .when(has_selection && cursor_at_start && show_cursor, |el| {
                el.child(
                    div()
                        .w(cursor_width)
                        .h(cursor_height)
                        .bg(cursor_color)
                        .rounded(px(2.0)),
                )
            })
            // Cursor at position (if no selection)
            .when(!has_selection && before.is_empty() && after.is_empty(), |el| {
                // Cursor after text when query is non-empty but cursor at end
                el.child(self.search.query.clone())
                    .when(show_cursor, |el| {
                        el.child(
                            div()
                                .w(cursor_width)
                                .h(cursor_height)
                                .bg(cursor_color)
                                .rounded(px(2.0)),
                        )
                    })
            })
            .when(!has_selection && (!before.is_empty() || !after.is_empty()) && show_cursor, |el| {
                // Cursor in the middle
                el.child(
                    div()
                        .w(cursor_width)
                        .h(cursor_height)
                        .bg(cursor_color)
                        .rounded(px(2.0)),
                )
            })
            // Selected text with background
            .when(!selected.is_empty(), |el| {
                el.child(
                    div()
                        .bg(selection_bg)
                        .rounded(px(2.0))
                        .child(selected.clone()),
                )
            })
            // Cursor after selection (if selection exists and cursor is at end)
            .when(has_selection && !cursor_at_start && show_cursor, |el| {
                el.child(
                    div()
                        .w(cursor_width)
                        .h(cursor_height)
                        .bg(cursor_color)
                        .rounded(px(2.0)),
                )
            })
            // Text after selection
            .when(!after.is_empty(), |el| el.child(after.clone()))
    }

    /// Render the search bar component
    pub(super) fn render_search_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement + '_ {
        let colors = get_launcher_colors(cx);
        // Determine icon and placeholder based on search mode
        let (icon, placeholder) = match &self.search.mode {
            SearchMode::Normal => ("🔍", ""),
            SearchMode::FileSearch => ("📁", ""),
            SearchMode::Calendar { title, .. } => ("📅", title.as_str()),
        };
        let placeholder = placeholder.to_string();
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;

        // Suppress unused variable warnings
        let _ = icon;
        let _ = text_placeholder;

        div()
            .h(SEARCH_BAR_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .child(
                // Search input with cursor and selection (no icon)
                div().flex_1().h_full().flex().items_center().child(
                    self.render_query_with_cursor(&colors, &placeholder),
                ),
            )
            // Show "esc to exit" hint in file search mode
            .when(matches!(self.search.mode, SearchMode::FileSearch), move |el| {
                el.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(text_muted)
                        .child("esc to exit"),
                )
            })
            .when(matches!(self.search.mode, SearchMode::Calendar { .. }), move |el| {
                el.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(text_muted)
                        .child("esc to go back"),
                )
            })
    }

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
                                        // Use themed colors for time indicators (success=now, warning=soon)
                                        let color = if rt == "now" { colors.success } else { colors.warning };
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
                            // Use the actual app icon - pass PathBuf for ImageSource::File
                            el.child(
                                img(icon_path.clone())
                                    .size(icon_size)
                                    .object_fit(ObjectFit::Contain),
                            )
                        } else {
                            // Fall back to emoji
                            el.text_size(TEXT_SIZE_LG).child(result.icon_emoji.clone())
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
                        .border_color(hsla(0.0, 0.0, 0.1, 1.0)), // Dark border for visibility
                )
            })
            // Task 7.2: Auto Quit indicator (orange dot, below green dot or bottom-right if not running)
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
                        .border_color(hsla(0.0, 0.0, 0.1, 1.0)), // Dark border for visibility
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
                // Shortcut badge - offset by 1 when meeting is visible (meeting takes ⌘1)
                let has_meeting = self.search.query.is_empty() && self.meeting.next_meeting.is_some();
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
            .border_color(if is_selected {
                accent
            } else {
                border
            })
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
                                    .text_color(if is_urgent {
                                        accent
                                    } else {
                                        text_muted
                                    })
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

    /// Render the suggestions section
    #[allow(dead_code)]
    pub(super) fn render_suggestions(&self, colors: &LauncherColors) -> impl IntoElement {
        let surface_hover = colors.surface_hover;
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;
        let items: Vec<_> = self
            .search.suggestions
            .iter()
            .take(6)
            .map(|result| {
                let icon_path = match &result.icon {
                    IconSource::FileIcon { path } => Self::get_app_icon_path(path),
                    IconSource::AppIcon { icon_path, .. } => icon_path.clone(),
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
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_start()
                    .children(items),
            )
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

    /// Render the confirmation dialog overlay
    pub(super) fn render_confirmation_dialog(
        &self,
        dialog: &ConfirmationDialog,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        // Full overlay with semi-transparent background
        div()
            .id("confirmation-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            // Theme overlay color
            .bg(colors.overlay)
            .child(
                // Dialog container
                div()
                    .id("confirmation-dialog")
                    .w(px(340.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    // Theme surface elevated background
                    .bg(colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_xl()
                    // Warning icon and title
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            // Warning icon
                            .child(
                                div()
                                    .size(SEARCH_BAR_HEIGHT)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_full()
                                    // Warning background with low opacity
                                    .bg(colors.warning.opacity(0.15))
                                    .child(
                                        div()
                                            .text_size(TEXT_SIZE_LG)
                                            .child("⚠️"),
                                    ),
                            )
                            // Title
                            .child(
                                div()
                                    .text_size(TEXT_SIZE_MD)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text)
                                    .child(dialog.title.clone()),
                            ),
                    )
                    // Message
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(colors.text_muted)
                                    .child(dialog.message.clone()),
                            ),
                    )
                    // Buttons
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .mt_2()
                            // Cancel button
                            .child({
                                let hover_bg = colors.surface_hover;
                                div()
                                    .id("cancel-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(colors.surface)
                                    .hover(move |el| el.bg(hover_bg))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.text)
                                            .child(dialog.cancel_label.clone()),
                                    )
                            })
                            // Confirm button (destructive style)
                            .child({
                                let error_color = colors.error;
                                // Lighten by increasing lightness component
                                let error_hover = hsla(error_color.h, error_color.s, (error_color.l + 0.1).min(1.0), error_color.a);
                                div()
                                    .id("confirm-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(error_color)
                                    .hover(move |el| el.bg(error_hover))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(colors.text)
                                            .child(dialog.confirm_label.clone()),
                                    )
                            }),
                    )
                    // Keyboard hints
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_center()
                            .mt_1()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_placeholder)
                                    .child("↵ Confirm  esc Cancel"),
                            ),
                    ),
            )
    }

    /// Render the action bar at the bottom (Raycast-style with primary action and shortcuts)
    pub(super) fn render_action_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        // Determine primary action based on mode
        let (primary_action, primary_shortcut) =
            if let SearchMode::Calendar { events, .. } = &self.search.mode {
                if let Some(event) = events.get(self.search.selected_index) {
                    if event.conference_url.is_some() {
                        ("Join Meeting", "↵")
                    } else {
                        ("Open in Calendar", "↵")
                    }
                } else {
                    ("", "")
                }
            } else if !self.search.results.is_empty() {
                ("Open", "↵")
            } else {
                ("", "")
            };

        let surface = colors.surface;
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;
        let border = colors.border;

        div()
            .w_full()
            .h(px(36.0))
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(border)
            .bg(colors.surface)
            // Left side: Primary action
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(!primary_action.is_empty(), move |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(4.0))
                                        .bg(surface)
                                        .text_size(px(10.0))
                                        .text_color(text_muted)
                                        .child(primary_shortcut),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child(primary_action),
                                ),
                        )
                    }),
            )
            // Right side: Actions shortcut
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_placeholder)
                                    .child("Actions"),
                            )
                            .child(
                                div()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(surface)
                                    .text_size(px(10.0))
                                    .text_color(text_muted)
                                    .child("⌘K"),
                            ),
                    ),
            )
    }

    /// Render the actions menu popup (Cmd+K)
    pub(super) fn render_actions_menu(&self, cx: &ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        // Determine available actions based on search mode and selection
        let is_file_mode = matches!(self.search.mode, SearchMode::FileSearch);
        let is_calendar_mode = matches!(self.search.mode, SearchMode::Calendar { .. });
        let has_selection = if is_calendar_mode {
            if let SearchMode::Calendar { events, .. } = &self.search.mode {
                !events.is_empty()
            } else {
                false
            }
        } else {
            !self.search.results.is_empty()
        };
        let selected = self.actions_menu.selected_index;

        // For calendar mode, check if selected event has conference
        let has_conference = if let SearchMode::Calendar { events, .. } = &self.search.mode {
            events
                .get(self.search.selected_index)
                .is_some_and(|e| e.conference_url.is_some())
        } else {
            false
        };

        // Task 7.3: Check if selected result is an app and if it's running
        let selected_result = self.search.results.get(self.search.selected_index);
        let is_app = selected_result.is_some_and(|r| r.result_type == ResultType::Application);
        let app_bundle_id = selected_result.and_then(|r| r.bundle_id.clone());
        let is_running = app_bundle_id
            .as_ref()
            .is_some_and(|id| photoncast_apps::is_app_running(id));
        let has_auto_quit = app_bundle_id
            .as_ref()
            .is_some_and(|id| self.auto_quit.manager.read().is_auto_quit_enabled(id));

        div()
            // Overlay background - position menu above action bar at bottom-right
            .absolute()
            .inset_0()
            .flex()
            .items_end()
            .justify_end()
            .pb(px(8.0)) // Small padding from bottom
            .pr_2()
            // Click outside to close
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, cx| {
                this.actions_menu.visible = false;
                cx.notify();
            }))
            .child(
                div()
                    .w(px(300.0))
                    .bg(colors.surface_elevated)
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_lg()
                    .overflow_hidden()
                    // Stop propagation so clicking menu doesn't close it
                    .on_mouse_down(MouseButton::Left, |_, cx| cx.stop_propagation())
                    // Header
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .border_b_1()
                            .border_color(colors.border)
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text)
                            .child("Actions"),
                    )
                    // Action items with selection highlighting (scrollable for long lists)
                    .child(
                        div()
                            .id("actions-menu-list")
                            .py_1()
                            .max_h(px(420.0)) // Fits within 475px modal
                            .overflow_y_scroll()
                            .when(is_calendar_mode, |el| {
                                // Calendar actions: Join Meeting (if available), Copy Title, Copy Details, Open in Calendar
                                let mut idx = 0;
                                let el = if has_conference {
                                    let e = el.child(self.render_action_item("Join Meeting", "↵", has_selection, selected == idx, &colors));
                                    idx += 1;
                                    e
                                } else {
                                    el
                                };
                                el.child(self.render_action_item("Copy Title", "⌘C", has_selection, selected == idx, &colors))
                                    .child(self.render_action_item("Copy Details", "⇧⌘C", has_selection, selected == idx + 1, &colors))
                                    .child(self.render_action_item("Open in Calendar", "⌘O", has_selection, selected == idx + 2, &colors))
                            })
                            .when(!is_calendar_mode && !is_app, |el| {
                                // Non-app actions (files, commands, etc.)
                                el.child(self.render_action_item("Open", "↵", has_selection, selected == 0, &colors))
                                    .child(self.render_action_item("Copy Path", "⌘C", has_selection, selected == 1, &colors))
                                    .child(self.render_action_item("Copy File", "⇧⌘C", has_selection, selected == 2, &colors))
                                    .when(is_file_mode, |el| {
                                        el.child(self.render_action_item("Reveal in Finder", "⌘↵", has_selection, selected == 3, &colors))
                                            .child(self.render_action_item("Quick Look", "⌘Y", has_selection, selected == 4, &colors))
                                    })
                            })
                            // Task 7.3: App-specific actions with grouped sections
                            .when(!is_calendar_mode && is_app, |el| {
                                let mut idx = 0;
                                // Primary actions
                                let el = el.child(self.render_action_group_header("Primary", &colors));
                                let el = el.child(self.render_action_item("Open", "↵", has_selection, selected == idx, &colors));
                                idx += 1;
                                let el = el.child(self.render_action_item("Show in Finder", "⌘⇧F", has_selection, selected == idx, &colors));
                                idx += 1;

                                // Info actions
                                let el = el.child(self.render_action_group_header("Info", &colors));
                                let el = el.child(self.render_action_item("Copy Path", "⌘⇧C", has_selection, selected == idx, &colors));
                                idx += 1;
                                let el = el.child(self.render_action_item("Copy Bundle ID", "⌘⇧B", has_selection, selected == idx, &colors));
                                idx += 1;

                                // Auto Quit toggle
                                let el = el.child(self.render_action_group_header("Auto Quit", &colors));
                                let auto_quit_label = if has_auto_quit { "Disable Auto Quit" } else { "Enable Auto Quit" };
                                let el = el.child(self.render_action_item(auto_quit_label, "⌘⇧A", has_selection, selected == idx, &colors));
                                idx += 1;

                                // Running app actions (only show if app is running)
                                let el = if is_running {
                                    let el = el.child(self.render_action_group_header("Running App", &colors));
                                    let el = el.child(self.render_action_item("Quit", "⌘Q", has_selection, selected == idx, &colors));
                                    idx += 1;
                                    let el = el.child(self.render_action_item("Force Quit", "⌘⌥Q", has_selection, selected == idx, &colors));
                                    idx += 1;
                                    el.child(self.render_action_item("Hide", "⌘H", has_selection, selected == idx, &colors))
                                } else {
                                    el
                                };
                                let idx = if is_running { idx + 1 } else { idx };

                                // Danger zone
                                let el = el.child(self.render_action_group_header("Danger Zone", &colors));
                                el.child(self.render_action_item_danger("Uninstall", "⌘⌫", has_selection, selected == idx, &colors))
                            })
                    )
                    // Footer hint
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(colors.border)
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_placeholder)
                                    .child("↑↓ Navigate  ↵ Select  esc Close"),
                            ),
                    ),
            )
    }

    /// Render a group header in the actions menu
    pub(super) fn render_action_group_header(&self, label: &str, colors: &LauncherColors) -> impl IntoElement {
        div()
            .px_3()
            .py(px(4.0))
            .mt(px(4.0))
            .text_size(px(10.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(colors.text_placeholder)
            .child(label.to_string().to_uppercase())
    }

    /// Render a danger action item (red text)
    pub(super) fn render_action_item_danger(
        &self,
        label: &str,
        shortcut: &str,
        enabled: bool,
        selected: bool,
        colors: &LauncherColors,
    ) -> impl IntoElement {
        let text_color = if enabled {
            colors.error
        } else {
            colors.text_placeholder
        };
        let shortcut_color = if enabled {
            colors.error.opacity(0.7)
        } else {
            colors.text_placeholder
        };
        let bg_color = if selected {
            colors.error.opacity(0.2)
        } else {
            gpui::transparent_black()
        };
        let hover_bg = colors.error.opacity(0.1);
        let surface = colors.surface;

        div()
            .px_3()
            .py(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(bg_color)
            .when(enabled && !selected, move |el| {
                el.hover(move |el| el.bg(hover_bg)).cursor_pointer()
            })
            .when(selected, |el| el.cursor_pointer())
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(text_color)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .px_1()
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .bg(surface)
                    .text_size(px(10.0))
                    .text_color(shortcut_color)
                    .child(shortcut.to_string()),
            )
    }

    /// Render a single action item in the menu
    pub(super) fn render_action_item(
        &self,
        label: &str,
        shortcut: &str,
        enabled: bool,
        selected: bool,
        colors: &LauncherColors,
    ) -> impl IntoElement {
        let text_color = if enabled {
            colors.text
        } else {
            colors.text_placeholder
        };
        let shortcut_color = if enabled {
            colors.text_muted
        } else {
            colors.text_placeholder
        };
        let bg_color = if selected {
            colors.selection
        } else {
            gpui::transparent_black()
        };
        let hover_bg = colors.surface_hover;
        let surface = colors.surface;

        div()
            .px_3()
            .py(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(bg_color)
            .when(enabled && !selected, move |el| {
                el.hover(move |el| el.bg(hover_bg)).cursor_pointer()
            })
            .when(selected, |el| el.cursor_pointer())
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(text_color)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .px_1()
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .bg(surface)
                    .text_size(px(10.0))
                    .text_color(shortcut_color)
                    .child(shortcut.to_string()),
            )
    }

    /// Renders the extension permissions consent dialog
    pub(super) fn render_permissions_consent_dialog(
        &self,
        consent: &crate::permissions_dialog::PendingPermissionsConsent,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let dialog = &consent.dialog;
        let extension_name = dialog.extension_name.clone();
        let permissions = dialog.permissions.clone();
        let is_first_launch = consent.is_first_launch;

        let launcher_colors = get_launcher_colors(cx);

        div()
            .id("permissions-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(launcher_colors.overlay)
            .child(
                div()
                    .id("permissions-dialog")
                    .w(px(380.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .bg(launcher_colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(launcher_colors.border)
                    .shadow_xl()
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
                                            .bg(launcher_colors.surface)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                div()
                                                    .text_size(px(20.0))
                                                    .child("🧩"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(15.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(launcher_colors.text)
                                            .child(if is_first_launch {
                                                format!("\"{}\" wants to be enabled", extension_name)
                                            } else {
                                                format!("\"{}\" requires permissions", extension_name)
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(launcher_colors.text_muted)
                                    .child("This extension requests access to:"),
                            ),
                    )
                    // Permissions list
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .bg(launcher_colors.surface)
                            .rounded(px(8.0))
                            .children(permissions.iter().map(|perm| {
                                let name = perm.name.clone();
                                let description = perm.description.clone();
                                let accent = launcher_colors.accent;
                                let text = launcher_colors.text;
                                let text_muted = launcher_colors.text_muted;

                                div()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .py_1()
                                    .child(
                                        div()
                                            .size(ICON_SIZE_MD)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(4.0))
                                            .bg(accent.opacity(0.1))
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(accent)
                                                    .child(match perm.id {
                                                        photoncast_core::extensions::permissions::PermissionType::Network => "🌐",
                                                        photoncast_core::extensions::permissions::PermissionType::Clipboard => "📋",
                                                        photoncast_core::extensions::permissions::PermissionType::Notifications => "🔔",
                                                        photoncast_core::extensions::permissions::PermissionType::Filesystem => "📁",
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_px()
                                            .child(
                                                div()
                                                    .text_size(px(13.0))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(text)
                                                    .child(name),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(text_muted)
                                                    .child(description),
                                            ),
                                    )
                            })),
                    )
                    // Action buttons
                    .child({
                        let accept_bg = launcher_colors.accent;
                        let accept_hover = launcher_colors.accent_hover;
                        let deny_bg = launcher_colors.surface;
                        let deny_hover = launcher_colors.surface_hover;
                        let text = launcher_colors.text;
                        let border = launcher_colors.border;

                        div()
                            .flex()
                            .gap_3()
                            .pt_2()
                            .child(
                                div()
                                    .id("deny-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(deny_bg)
                                    .border_1()
                                    .border_color(border)
                                    .hover(move |s| s.bg(deny_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.deny_permissions_consent(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(text)
                                            .child("Deny"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("accept-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(accept_bg)
                                    .hover(move |s| s.bg(accept_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.accept_permissions_consent(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                            .child("Enable"),
                                    ),
                            )
                    }),
            )
    }

    /// Renders the uninstall preview dialog
    pub(super) fn render_uninstall_preview(
        &self,
        preview: &UninstallPreview,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let app_name = preview.app.name.clone();
        let space_freed = preview.space_freed_formatted.clone();

        // Group files by category
        let mut categories: std::collections::BTreeMap<
            &str,
            Vec<(usize, &photoncast_apps::RelatedFile)>,
        > = std::collections::BTreeMap::new();
        for (idx, file) in preview.related_files.iter().enumerate() {
            let category_name = file.category.display_name();
            categories
                .entry(category_name)
                .or_default()
                .push((idx, file));
        }

        // Get icon path for the app
        let icon_path = Self::get_app_icon_path(&preview.app.path);

        // Pre-build category sections to avoid borrowing cx inside nested iterators
        let category_sections: Vec<_> = categories
            .into_iter()
            .map(|(category_name, files)| {
                let text_muted = colors.text_muted;
                let text = colors.text;
                let text_placeholder = colors.text_placeholder;
                let surface = colors.surface;
                let surface_hover = colors.surface_hover;
                let accent = colors.accent;
                let border = colors.border;

                // Pre-build file items for this category
                let file_items: Vec<_> = files
                    .into_iter()
                    .map(|(idx, file)| {
                        let file_name = file
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let file_size = UninstallPreview::format_bytes(file.size_bytes);
                        let is_selected = file.selected;

                        div()
                            .id(SharedString::from(format!("uninstall-file-{}", idx)))
                            .px_3()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(surface)
                            .hover(move |el| el.bg(surface_hover))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_3()
                            .on_click(cx.listener(move |this, _, cx| {
                                this.toggle_uninstall_file_selection(idx, cx);
                            }))
                            // Checkbox
                            .child(
                                div()
                                    .size(px(18.0))
                                    .rounded(px(4.0))
                                    .border_1()
                                    .border_color(if is_selected { accent } else { border })
                                    .bg(if is_selected {
                                        accent
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(text)
                                            .when(is_selected, |el| el.child("✓")),
                                    ),
                            )
                            // File info
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(text)
                                            .truncate()
                                            .child(file_name),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(text_placeholder)
                                            .child(file_size),
                                    ),
                            )
                    })
                    .collect();

                (category_name.to_string(), text_muted, file_items)
            })
            .collect();

        div()
            .id("uninstall-preview-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(colors.overlay)
            .child(
                div()
                    .id("uninstall-preview-dialog")
                    .w(px(420.0))
                    .max_h(px(500.0))
                    .flex()
                    .flex_col()
                    .bg(colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_xl()
                    .overflow_hidden()
                    // Header with app info
                    .child(
                        div()
                            .px_5()
                            .pt_5()
                            .pb_4()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            // App icon
                            .child(
                                div()
                                    .size(px(64.0))
                                    .rounded(px(12.0))
                                    .overflow_hidden()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .map(|el| {
                                        if let Some(icon) = &icon_path {
                                            el.child(
                                                img(icon.clone())
                                                    .size(px(64.0))
                                                    .object_fit(ObjectFit::Contain),
                                            )
                                        } else {
                                            el.text_size(ICON_SIZE_LG).child("📦")
                                        }
                                    }),
                            )
                            // App name
                            .child(
                                div()
                                    .text_size(px(18.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text)
                                    .child(format!("Uninstall {}", app_name)),
                            )
                            // Space to be freed
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .text_color(colors.text_muted)
                                            .child("Space to be freed:"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.success)
                                            .child(space_freed),
                                    ),
                            ),
                    )
                    // Related files list
                    .child(
                        div()
                            .id("uninstall-files-list")
                            .flex_1()
                            .overflow_y_scroll()
                            .px_5()
                            .pb_3()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .children(
                                category_sections
                                    .into_iter()
                                    .map(|(category_name, text_muted, file_items)| {
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            // Category header
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(text_muted)
                                                    .child(category_name.to_uppercase()),
                                            )
                                            // Files in category
                                            .children(file_items)
                                    }),
                            ),
                    )
                    // Action buttons
                    .child(
                        div()
                            .px_5()
                            .py_4()
                            .border_t_1()
                            .border_color(colors.border)
                            .flex()
                            .gap_3()
                            // Cancel button
                            .child({
                                let surface = colors.surface;
                                let surface_hover = colors.surface_hover;
                                let text = colors.text;
                                div()
                                    .id("uninstall-cancel")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(surface)
                                    .hover(move |el| el.bg(surface_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.cancel_uninstall_preview(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(text)
                                            .child("Cancel"),
                                    )
                            })
                            // Keep Related Files button
                            .child({
                                let surface = colors.surface;
                                let surface_hover = colors.surface_hover;
                                let text = colors.text;
                                div()
                                    .id("uninstall-keep-files")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(surface)
                                    .hover(move |el| el.bg(surface_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.perform_uninstall_app_only(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(text)
                                            .child("Keep Files"),
                                    )
                            })
                            // Uninstall button (primary, destructive)
                            .child({
                                let error = colors.error;
                                let error_hover =
                                    hsla(error.h, error.s, (error.l + 0.1).min(1.0), error.a);
                                let text = colors.text;
                                div()
                                    .id("uninstall-confirm")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(error)
                                    .hover(move |el| el.bg(error_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.perform_uninstall(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(text)
                                            .child("Uninstall"),
                                    )
                            }),
                    ),
            )
    }

    /// Renders the auto quit settings panel
    pub(super) fn render_auto_quit_settings(
        &self,
        bundle_id: &str,
        app_name: &str,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let manager = self.auto_quit.manager.read();
        let is_enabled = manager.is_auto_quit_enabled(bundle_id);
        let current_timeout = manager
            .get_timeout_minutes(bundle_id)
            .unwrap_or(DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES);
        drop(manager);

        let timeout_options = [1, 2, 3, 5, 10, 15, 30];
        let selected_index = self.auto_quit.settings_index;

        div()
            .id("auto-quit-settings-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_end()
            .justify_end()
            .pb_2()
            .pr_2()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, cx| {
                this.close_auto_quit_settings(cx);
            }))
            .child(
                div()
                    .w(px(280.0))
                    .bg(colors.surface_elevated)
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, cx| cx.stop_propagation())
                    // Header
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(colors.border)
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text)
                                    .child("Auto Quit Settings"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_muted)
                                    .truncate()
                                    .child(app_name.to_string()),
                            ),
                    )
                    // Enable toggle
                    .child({
                        let surface = colors.surface;
                        let surface_hover = colors.surface_hover;
                        let text = colors.text;
                        let accent = colors.accent;
                        let is_toggle_selected = selected_index == 0;
                        div()
                            .id("auto-quit-toggle")
                            .px_4()
                            .py_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .cursor_pointer()
                            .bg(if is_toggle_selected { surface_hover } else { gpui::transparent_black() })
                            .hover(move |el| el.bg(surface_hover))
                            .on_click(cx.listener(|this, _, cx| {
                                this.toggle_auto_quit_in_settings(cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(text)
                                    .child("Enable Auto Quit"),
                            )
                            .child(
                                div()
                                    .w(px(36.0))
                                    .h(px(20.0))
                                    .rounded(px(10.0))
                                    .bg(if is_enabled { accent } else { surface })
                                    .relative()
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(2.0))
                                            .left(if is_enabled { px(18.0) } else { px(2.0) })
                                            .size(ICON_SIZE_SM)
                                            .rounded_full()
                                            .bg(text),
                                    ),
                            )
                    })
                    // Timeout selector (only when enabled)
                    .when(is_enabled, |el| {
                        let text_muted = colors.text_muted;
                        let text = colors.text;
                        let surface = colors.surface;
                        let surface_hover = colors.surface_hover;
                        let accent = colors.accent;
                        el.child(
                            div()
                                .px_4()
                                .py_2()
                                .border_t_1()
                                .border_color(colors.border)
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child("Quit after idle for:"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap(px(6.0))
                                        .children(timeout_options.iter().enumerate().map(move |(idx, &minutes)| {
                                            let is_current = minutes == current_timeout;
                                            let is_kb_selected = selected_index == idx + 1; // +1 because 0 is toggle
                                            div()
                                                .id(SharedString::from(format!("timeout-{}", minutes)))
                                                .px(px(10.0))
                                                .py(px(4.0))
                                                .rounded(px(4.0))
                                                .bg(if is_current { accent } else if is_kb_selected { surface_hover } else { surface })
                                                .border_1()
                                                .border_color(if is_kb_selected { accent } else { gpui::transparent_black() })
                                                .hover(move |el| el.bg(if is_current { accent } else { surface_hover }))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, cx| {
                                                    this.set_auto_quit_timeout(minutes, cx);
                                                }))
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(text)
                                                        .child(format!("{} min", minutes)),
                                                )
                                        })),
                                ),
                        )
                    })
                    // Footer hint
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .border_t_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_placeholder)
                                    .child("Auto Quit stops idle apps to save resources"),
                            ),
                    ),
            )
    }

    /// Renders the "Manage Auto Quits" view
    pub(super) fn render_manage_auto_quits(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let manager = self.auto_quit.manager.read();
        let enabled_apps: Vec<_> = manager
            .get_enabled_apps()
            .iter()
            .map(|(id, cfg)| (id.to_string(), cfg.timeout_minutes))
            .collect();
        drop(manager);

        let selected = self.auto_quit.manage_index;
        let is_empty = enabled_apps.is_empty();

        div()
            .w_full()
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text_muted)
                            .child("AUTO QUIT APPS"),
                    ),
            )
            // App list or empty state
            .when(is_empty, |el| {
                el.child(
                    div()
                        .py_6()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(colors.text_muted)
                                .child("No apps with Auto Quit enabled"),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(colors.text_placeholder)
                                .child("Use ⌘K on any app to enable Auto Quit"),
                        ),
                )
            })
            .when(!is_empty, |el| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .children(enabled_apps.iter().enumerate().map(|(idx, (bundle_id, timeout))| {
                            let is_selected = idx == selected;
                            let app_name = photoncast_apps::get_suggested_app_name(bundle_id)
                                .map(String::from)
                                .unwrap_or_else(|| {
                                    // Try to get last component of bundle ID
                                    bundle_id.split('.').next_back().unwrap_or(bundle_id).to_string()
                                });

                            let text = colors.text;
                            let text_muted = colors.text_muted;
                            let text_placeholder = colors.text_placeholder;
                            let surface = colors.surface;
                            let surface_hover = colors.surface_hover;
                            let selection = colors.selection;
                            let error = colors.error;

                            div()
                                .id(SharedString::from(format!("auto-quit-app-{}", idx)))
                                .h(px(48.0))
                                .px_4()
                                .flex()
                                .items_center()
                                .justify_between()
                                .bg(if is_selected { selection } else { gpui::transparent_black() })
                                .hover(move |el| el.bg(surface_hover))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .text_color(text)
                                                .child(app_name),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(text_muted)
                                                .child(format!("Quit after {} min idle", timeout)),
                                        ),
                                )
                                // Disable button
                                .child(
                                    div()
                                        .id(SharedString::from(format!("disable-auto-quit-{}", idx)))
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(surface)
                                        .hover(move |el| el.bg(error.opacity(0.2)))
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, cx| {
                                            this.disable_auto_quit_at_index(idx, cx);
                                        }))
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(text_placeholder)
                                                .child("Disable"),
                                        ),
                                )
                        })),
                )
            })
            // Footer with hints
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(colors.border)
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(colors.text_placeholder)
                            .child("↑↓ Navigate  esc Back"),
                    ),
            )
    }

    /// Renders the toast notification
    pub(super) fn render_toast(&self, message: &str, cx: &ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);

        // Calculate opacity based on time (fade in/out)
        let opacity = if let Some(shown_at) = self.toast.shown_at {
            let elapsed = shown_at.elapsed().as_millis() as f32;
            if elapsed < 150.0 {
                // Fade in
                elapsed / 150.0
            } else if elapsed > 1800.0 {
                // Fade out (after 1.8s, fade out over 200ms)
                1.0 - ((elapsed - 1800.0) / 200.0).min(1.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        div()
            .id("toast-notification")
            .absolute()
            .bottom(px(12.0))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .opacity(opacity)
            .child(
                div()
                    .px_4()
                    .py_2()
                    .rounded(px(8.0))
                    .bg(colors.surface_elevated)
                    .border_1()
                    .border_color(colors.border)
                    .shadow_md()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_size(TEXT_SIZE_MD).child("✓"))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child(message.to_string()),
                    ),
            )
    }
}

impl Render for LauncherWindow {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Get theme colors for the UI
        let colors = get_launcher_colors(cx);

        // Calculate current animation opacity
        let opacity = self.current_opacity();

        // Clone pending confirmation for use in the closure
        let pending_dialog = self.pending_confirmation.as_ref().map(|(_, d)| d.clone());

        // Pre-render components that need colors (for use in closures)
        let empty_state = self.render_empty_state(&colors);
        let no_results = self.render_no_results(&colors);
        let divider_color = colors.border;

        // Check if any overlay is active (need minimum height for overlays)
        let has_overlay = self.actions_menu.visible
            || self.auto_quit.settings_app.is_some()
            || self.uninstall.preview.is_some()
            || self.auto_quit.manage_mode
            || pending_dialog.is_some();

        // Main container with rounded corners and shadow
        // Note: When extension view is active, it handles its own focus and keyboard events
        div()
            .track_focus(&self.focus_handle)
            .key_context("LauncherWindow")
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::confirm_dialog))
            .on_action(cx.listener(Self::next_group))
            .on_action(cx.listener(Self::previous_group))
            .on_action(cx.listener(Self::open_preferences))
            // File Search Mode actions
            .on_action(cx.listener(Self::reveal_in_finder))
            .on_action(cx.listener(Self::quick_look))
            .on_action(cx.listener(Self::copy_path))
            .on_action(cx.listener(Self::copy_file))
            .on_action(cx.listener(Self::show_actions_menu))
            // Task 7.4: App management action handlers
            .on_action(cx.listener(Self::show_in_finder))
            .on_action(cx.listener(Self::copy_bundle_id))
            .on_action(cx.listener(Self::quit_app))
            .on_action(cx.listener(Self::force_quit_app))
            .on_action(cx.listener(Self::hide_app))
            .on_action(cx.listener(Self::uninstall_app))
            .on_action(cx.listener(Self::toggle_auto_quit_for_selected))
            .on_action(cx.listener(|this, _: &QuickSelect1, cx| this.quick_select(0, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect2, cx| this.quick_select(1, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect3, cx| this.quick_select(2, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect4, cx| this.quick_select(3, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect5, cx| this.quick_select(4, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect6, cx| this.quick_select(5, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect7, cx| this.quick_select(7, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect8, cx| this.quick_select(7, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect9, cx| this.quick_select(8, cx)))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            // Apply window appear/dismiss animation opacity
            .opacity(opacity)
            // Theme background color
            .bg(colors.background)
            .rounded(LAUNCHER_BORDER_RADIUS)
            .shadow_lg()
            .border_1()
            .border_color(colors.border)
            // Keep minimum height when overlays are visible to prevent clipping
            .when(has_overlay, |el| el.min_h(px(400.0)))
            .overflow_hidden()
            // File Search Mode: render the dedicated FileSearchView
            .when_some(self.file_search.view.clone(), |el, view| {
                el.child(
                    div()
                        .size_full() // Fill the resized window
                        .child(view)
                )
            })
            // Extension View Mode: render the extension's view
            .when_some(self.extension_view.view.clone(), |el, view| {
                el.child(
                    div()
                        .size_full()
                        .child(view)
                )
            })
            // Normal/Calendar Mode: render the standard launcher content
            .when(self.file_search.view.is_none() && self.extension_view.view.is_none(), |el| {
                el
                    // Search bar
                    .child(self.render_search_bar(cx))
                    // Content area (flex-1 to push action bar to bottom)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Next meeting widget (show when query is empty and we have a meeting)
                            .when(self.search.query.is_empty() && self.meeting.next_meeting.is_some() && !matches!(self.search.mode, SearchMode::Calendar { .. }), |el| {
                                if let Some(meeting) = &self.meeting.next_meeting {
                                    el.child(self.render_next_meeting(meeting, &colors))
                                } else {
                                    el
                                }
                            })
                            // Divider (show when there are results or in calendar mode)
                            .when(!self.search.results.is_empty() || matches!(self.search.mode, SearchMode::Calendar { .. }), move |el| {
                                el.child(div().h(px(1.0)).w_full().bg(divider_color))
                            })
                            // Empty state: Normal mode with no meeting
                            .when(self.search.query.is_empty() && self.search.results.is_empty() && matches!(self.search.mode, SearchMode::Normal) && self.meeting.next_meeting.is_none(), |el| {
                                el.child(empty_state)
                            })
                            // No results message: query entered but nothing found
                            .when(!self.search.query.is_empty() && self.search.results.is_empty() && !matches!(self.search.mode, SearchMode::Calendar { .. }), |el| {
                                el.child(no_results)
                            })
                            // Results list: show suggestions (when query empty) or search results
                            .when(!self.search.results.is_empty() || matches!(self.search.mode, SearchMode::Calendar { .. }), |el| {
                                el.child(self.render_results(cx))
                            })
                    )
                    // Action bar at bottom - always visible, pinned by flex layout
                    .child(self.render_action_bar(cx))
            })
            // Actions menu overlay (Cmd+K)
            .when(self.actions_menu.visible, |el| {
                el.child(self.render_actions_menu(cx))
            })
            // Confirmation dialog overlay
            .when_some(pending_dialog, |el, dialog| {
                el.child(self.render_confirmation_dialog(&dialog, cx))
            })
            // Extension permissions consent dialog overlay
            .when_some(self.pending_permissions_consent.clone(), |el, consent| {
                el.child(self.render_permissions_consent_dialog(&consent, cx))
            })
            // Task 7.5: Uninstall preview overlay
            .when_some(self.uninstall.preview.clone(), |el, preview| {
                el.child(self.render_uninstall_preview(&preview, cx))
            })
            // Task 7.6: Auto quit settings overlay
            .when_some(self.auto_quit.settings_app.clone(), |el, (bundle_id, app_name)| {
                el.child(self.render_auto_quit_settings(&bundle_id, &app_name, cx))
            })
            // Task 7.7: Manage auto quits view
            .when(self.auto_quit.manage_mode, |el| {
                el.child(self.render_manage_auto_quits(cx))
            })
            // Task 7.8: Toast notification
            .when_some(self.toast.message.clone(), |el, message| {
                el.child(self.render_toast(&message, cx))
            })
    }
}


impl FocusableView for LauncherWindow {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

