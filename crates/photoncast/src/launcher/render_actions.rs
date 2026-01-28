//! Action bar and actions menu rendering for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
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
    pub(super) fn render_actions_menu(
        &self,
        cx: &ViewContext<Self>,
    ) -> impl IntoElement {
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
        let has_conference =
            if let SearchMode::Calendar { events, .. } = &self.search.mode {
                events
                    .get(self.search.selected_index)
                    .is_some_and(|e| e.conference_url.is_some())
            } else {
                false
            };

        // Task 7.3: Check if selected result is an app and if it's running
        let selected_result = self.search.results.get(self.search.selected_index);
        let is_app =
            selected_result.is_some_and(|r| r.result_type == ResultType::Application);
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
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, cx| {
                    this.actions_menu.visible = false;
                    cx.notify();
                }),
            )
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
                                let el = el.child(self.render_action_group_header("Primary", &colors));
                                let el = el.child(self.render_action_item("Open", "↵", has_selection, selected == idx, &colors));
                                idx += 1;
                                let el = el.child(self.render_action_item("Show in Finder", "⌘⇧F", has_selection, selected == idx, &colors));
                                idx += 1;

                                let el = el.child(self.render_action_group_header("Info", &colors));
                                let el = el.child(self.render_action_item("Copy Path", "⌘⇧C", has_selection, selected == idx, &colors));
                                idx += 1;
                                let el = el.child(self.render_action_item("Copy Bundle ID", "⌘⇧B", has_selection, selected == idx, &colors));
                                idx += 1;

                                let el = el.child(self.render_action_group_header("Auto Quit", &colors));
                                let auto_quit_label = if has_auto_quit { "Disable Auto Quit" } else { "Enable Auto Quit" };
                                let el = el.child(self.render_action_item(auto_quit_label, "⌘⇧A", has_selection, selected == idx, &colors));
                                idx += 1;

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
    pub(super) fn render_action_group_header(
        &self,
        label: &str,
        colors: &LauncherColors,
    ) -> impl IntoElement {
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
}
