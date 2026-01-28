//! Query input rendering for [`LauncherWindow`].

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
            .when(
                !has_selection && before.is_empty() && after.is_empty(),
                |el| {
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
                },
            )
            .when(
                !has_selection
                    && (!before.is_empty() || !after.is_empty())
                    && show_cursor,
                |el| {
                    // Cursor in the middle
                    el.child(
                        div()
                            .w(cursor_width)
                            .h(cursor_height)
                            .bg(cursor_color)
                            .rounded(px(2.0)),
                    )
                },
            )
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
    pub(super) fn render_search_bar(
        &self,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement + '_ {
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
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .child(self.render_query_with_cursor(&colors, &placeholder)),
            )
            // Show "esc to exit" hint in file search mode
            .when(
                matches!(self.search.mode, SearchMode::FileSearch),
                move |el| {
                    el.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child("esc to exit"),
                    )
                },
            )
            .when(
                matches!(self.search.mode, SearchMode::Calendar { .. }),
                move |el| {
                    el.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child("esc to go back"),
                    )
                },
            )
    }
}
