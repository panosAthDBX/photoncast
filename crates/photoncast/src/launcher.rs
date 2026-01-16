//! Main launcher window component for PhotonCast.
//!
//! This module contains the `LauncherWindow` struct that implements
//! the GPUI `Render` trait for the main launcher UI.
//!
//! # Animations
//!
//! The launcher supports the following animations (all respecting reduce motion):
//! - Window appear: 150ms ease-out fade + scale (0.95 → 1.0)
//! - Window dismiss: 100ms ease-in fade + scale down
//! - Selection change: 80ms ease-in-out background transition
//! - Hover highlight: 60ms linear background transition

use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::{
    Activate, Cancel, NextGroup, OpenPreferences, PreviousGroup, QuickSelect1, QuickSelect2,
    QuickSelect3, QuickSelect4, QuickSelect5, QuickSelect6, QuickSelect7, QuickSelect8,
    QuickSelect9, SelectNext, SelectPrevious, LAUNCHER_BORDER_RADIUS, LAUNCHER_MAX_HEIGHT,
    LAUNCHER_MIN_HEIGHT,
};

use photoncast_core::ui::animations::{
    ease_in, ease_out, ease_in_out, lerp, selection_change_duration,
    window_appear_duration, window_dismiss_duration, WindowAnimationState,
    WINDOW_APPEAR_OPACITY_END, WINDOW_APPEAR_OPACITY_START, WINDOW_APPEAR_SCALE_END,
    WINDOW_APPEAR_SCALE_START, WINDOW_DISMISS_SCALE_END,
};
use photoncast_core::search::fuzzy::FuzzyMatcher;

use crate::platform::resize_window_height;

/// Search bar height constant
const SEARCH_BAR_HEIGHT: Pixels = px(48.0);
/// Search icon size
const SEARCH_ICON_SIZE: Pixels = px(20.0);
/// Result item height
const RESULT_ITEM_HEIGHT: Pixels = px(56.0);
/// Maximum visible results
const MAX_VISIBLE_RESULTS: usize = 8;

/// A searchable item in the catalog
#[derive(Clone)]
pub struct SearchableItem {
    pub id: SharedString,
    pub title: String,
    pub subtitle: String,
    pub icon: SharedString,
    pub result_type: ResultType,
}

/// The main launcher window state
pub struct LauncherWindow {
    /// Current search query
    query: SharedString,
    /// Whether the window is visible
    visible: bool,
    /// Currently selected result index
    selected_index: usize,
    /// Previously selected index (for selection change animation)
    previous_selected_index: Option<usize>,
    /// Filtered results for current query
    results: Vec<ResultItem>,
    /// Focus handle for the search input
    focus_handle: FocusHandle,
    /// Window animation state
    animation_state: WindowAnimationState,
    /// Time when the current animation started
    animation_start: Option<Instant>,
    /// Index of the currently hovered result item (for hover animation)
    #[allow(dead_code)]
    hovered_index: Option<usize>,
    /// Time when selection changed (for selection animation)
    selection_animation_start: Option<Instant>,
    /// Hover animation starts per item (for smooth hover transitions)
    #[allow(dead_code)]
    hover_animation_starts: std::collections::HashMap<usize, Instant>,
    /// Fuzzy matcher for search
    fuzzy_matcher: FuzzyMatcher,
    /// All searchable items (apps, commands, etc.)
    searchable_items: Vec<SearchableItem>,
}

/// A single result item
#[derive(Clone)]
pub struct ResultItem {
    #[allow(dead_code)]
    pub id: SharedString,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub icon: SharedString,
    pub result_type: ResultType,
}

/// Type of search result for grouping
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ResultType {
    Application,
    Command,
    File,
}

impl ResultType {
    #[allow(dead_code)]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Application => "Apps",
            Self::Command => "Commands",
            Self::File => "Files",
        }
    }
}

impl LauncherWindow {
    /// Creates a new launcher window
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Request focus immediately
        cx.focus(&focus_handle);

        // Create the catalog of searchable items
        let searchable_items = Self::build_searchable_items();

        let mut window = Self {
            query: SharedString::default(),
            visible: true,
            selected_index: 0,
            previous_selected_index: None,
            results: vec![],
            focus_handle,
            animation_state: WindowAnimationState::Hidden,
            animation_start: None,
            hovered_index: None,
            selection_animation_start: None,
            hover_animation_starts: std::collections::HashMap::new(),
            fuzzy_matcher: FuzzyMatcher::default(),
            searchable_items,
        };

        // Start the appear animation
        window.start_appear_animation(cx);
        
        // Set initial window height
        window.update_window_height(cx);

        window
    }

    /// Build the list of searchable items (applications, commands, etc.)
    fn build_searchable_items() -> Vec<SearchableItem> {
        vec![
            // Applications
            SearchableItem {
                id: "safari".into(),
                title: "Safari".into(),
                subtitle: "Web Browser".into(),
                icon: "🌐".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "finder".into(),
                title: "Finder".into(),
                subtitle: "File Manager".into(),
                icon: "📁".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "terminal".into(),
                title: "Terminal".into(),
                subtitle: "Command Line".into(),
                icon: "💻".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "system-settings".into(),
                title: "System Settings".into(),
                subtitle: "System Preferences".into(),
                icon: "⚙️".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "notes".into(),
                title: "Notes".into(),
                subtitle: "Note Taking".into(),
                icon: "📝".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "messages".into(),
                title: "Messages".into(),
                subtitle: "Instant Messaging".into(),
                icon: "💬".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "mail".into(),
                title: "Mail".into(),
                subtitle: "Email Client".into(),
                icon: "📧".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "calendar".into(),
                title: "Calendar".into(),
                subtitle: "Calendar App".into(),
                icon: "📅".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "music".into(),
                title: "Music".into(),
                subtitle: "Apple Music".into(),
                icon: "🎵".into(),
                result_type: ResultType::Application,
            },
            SearchableItem {
                id: "photos".into(),
                title: "Photos".into(),
                subtitle: "Photo Library".into(),
                icon: "🖼️".into(),
                result_type: ResultType::Application,
            },
            // Commands
            SearchableItem {
                id: "lock".into(),
                title: "Lock Screen".into(),
                subtitle: "Lock your Mac".into(),
                icon: "🔒".into(),
                result_type: ResultType::Command,
            },
            SearchableItem {
                id: "sleep".into(),
                title: "Sleep".into(),
                subtitle: "Put Mac to sleep".into(),
                icon: "😴".into(),
                result_type: ResultType::Command,
            },
            SearchableItem {
                id: "restart".into(),
                title: "Restart".into(),
                subtitle: "Restart your Mac".into(),
                icon: "🔄".into(),
                result_type: ResultType::Command,
            },
            SearchableItem {
                id: "shutdown".into(),
                title: "Shut Down".into(),
                subtitle: "Turn off your Mac".into(),
                icon: "⏻".into(),
                result_type: ResultType::Command,
            },
            SearchableItem {
                id: "empty-trash".into(),
                title: "Empty Trash".into(),
                subtitle: "Empty the Trash".into(),
                icon: "🗑️".into(),
                result_type: ResultType::Command,
            },
        ]
    }

    /// Starts the window appear animation.
    fn start_appear_animation(&mut self, cx: &mut ViewContext<Self>) {
        let duration = window_appear_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.animation_state = WindowAnimationState::Visible;
            self.animation_start = None;
        } else {
            self.animation_state = WindowAnimationState::Appearing;
            self.animation_start = Some(Instant::now());
            // Schedule a refresh to drive the animation
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16); // ~60 FPS
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if this.animation_state == WindowAnimationState::Appearing {
                                if let Some(start) = this.animation_start {
                                    let elapsed = start.elapsed();
                                    let total = window_appear_duration();
                                    if elapsed >= total {
                                        this.animation_state = WindowAnimationState::Visible;
                                        this.animation_start = None;
                                        cx.notify();
                                        return false; // Animation complete
                                    }
                                    cx.notify();
                                    return true; // Continue animation
                                }
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }
        cx.notify();
    }

    /// Starts the window dismiss animation.
    fn start_dismiss_animation(&mut self, cx: &mut ViewContext<Self>) {
        let duration = window_dismiss_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.animation_state = WindowAnimationState::Hidden;
            self.animation_start = None;
            cx.quit();
        } else {
            self.animation_state = WindowAnimationState::Dismissing;
            self.animation_start = Some(Instant::now());
            // Schedule a refresh to drive the animation
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16); // ~60 FPS
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if this.animation_state == WindowAnimationState::Dismissing {
                                if let Some(start) = this.animation_start {
                                    let elapsed = start.elapsed();
                                    let total = window_dismiss_duration();
                                    if elapsed >= total {
                                        this.animation_state = WindowAnimationState::Hidden;
                                        this.animation_start = None;
                                        cx.quit();
                                        return false; // Animation complete
                                    }
                                    cx.notify();
                                    return true; // Continue animation
                                }
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }
        cx.notify();
    }

    /// Calculates the current animation progress (0.0 to 1.0).
    fn animation_progress(&self) -> f32 {
        match (self.animation_state, self.animation_start) {
            (WindowAnimationState::Appearing, Some(start)) => {
                let elapsed = start.elapsed();
                let total = window_appear_duration();
                if total.is_zero() {
                    1.0
                } else {
                    (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0)
                }
            },
            (WindowAnimationState::Dismissing, Some(start)) => {
                let elapsed = start.elapsed();
                let total = window_dismiss_duration();
                if total.is_zero() {
                    1.0
                } else {
                    (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0)
                }
            },
            (WindowAnimationState::Visible, _) => 1.0,
            (WindowAnimationState::Hidden, _) => 0.0,
            _ => 1.0,
        }
    }

    /// Calculates the current opacity based on animation state.
    fn current_opacity(&self) -> f32 {
        match self.animation_state {
            WindowAnimationState::Appearing => {
                let progress = ease_out(self.animation_progress());
                lerp(
                    WINDOW_APPEAR_OPACITY_START,
                    WINDOW_APPEAR_OPACITY_END,
                    progress,
                )
            },
            WindowAnimationState::Dismissing => {
                let progress = ease_in(self.animation_progress());
                lerp(
                    WINDOW_APPEAR_OPACITY_END,
                    WINDOW_APPEAR_OPACITY_START,
                    progress,
                )
            },
            WindowAnimationState::Visible => 1.0,
            WindowAnimationState::Hidden => 0.0,
        }
    }

    /// Calculates the current scale based on animation state.
    #[allow(dead_code)]
    fn current_scale(&self) -> f32 {
        match self.animation_state {
            WindowAnimationState::Appearing => {
                let progress = ease_out(self.animation_progress());
                lerp(WINDOW_APPEAR_SCALE_START, WINDOW_APPEAR_SCALE_END, progress)
            },
            WindowAnimationState::Dismissing => {
                let progress = ease_in(self.animation_progress());
                lerp(WINDOW_APPEAR_SCALE_END, WINDOW_DISMISS_SCALE_END, progress)
            },
            WindowAnimationState::Visible => 1.0,
            WindowAnimationState::Hidden => WINDOW_APPEAR_SCALE_START,
        }
    }

    /// Toggle the visibility of the launcher window
    #[allow(dead_code)]
    pub fn toggle(&mut self, cx: &mut ViewContext<Self>) {
        self.visible = !self.visible;
        if self.visible {
            self.query = SharedString::default();
            self.selected_index = 0;
            self.previous_selected_index = None;
            cx.focus(&self.focus_handle);
            self.start_appear_animation(cx);
        } else {
            self.start_dismiss_animation(cx);
        }
    }

    /// Shows the launcher window with animation
    #[allow(dead_code)]
    pub fn show(&mut self, cx: &mut ViewContext<Self>) {
        self.visible = true;
        self.query = SharedString::default();
        self.selected_index = 0;
        self.previous_selected_index = None;
        cx.focus(&self.focus_handle);
        self.start_appear_animation(cx);
    }

    /// Hides the launcher window with animation
    pub fn hide(&mut self, cx: &mut ViewContext<Self>) {
        self.visible = false;
        self.start_dismiss_animation(cx);
    }

    /// Handle query change from search input
    fn on_query_change(&mut self, _query: SharedString, cx: &mut ViewContext<Self>) {
        self.selected_index = 0;

        // Perform fuzzy search on all searchable items
        if self.query.is_empty() {
            self.results.clear();
        } else {
            // Create iterator of (id, title) for fuzzy matching
            let targets: Vec<_> = self.searchable_items
                .iter()
                .enumerate()
                .map(|(idx, item)| (idx, item.title.as_str()))
                .collect();

            // Get fuzzy matches sorted by score
            let matches = self.fuzzy_matcher.score_many(&self.query, targets.into_iter());

            // Convert matches to ResultItems, limited to max visible
            self.results = matches
                .into_iter()
                .take(MAX_VISIBLE_RESULTS)
                .map(|(idx, _score, _indices)| {
                    let item = &self.searchable_items[idx];
                    ResultItem {
                        id: item.id.clone(),
                        title: item.title.clone().into(),
                        subtitle: item.subtitle.clone().into(),
                        icon: item.icon.clone(),
                        result_type: item.result_type,
                    }
                })
                .collect();
        }

        // Update window height based on results
        self.update_window_height(cx);
        cx.notify();
    }

    /// Update window height based on result count
    fn update_window_height(&self, cx: &mut ViewContext<Self>) {
        let result_count = self.results.len().min(MAX_VISIBLE_RESULTS);
        let query_empty = self.query.is_empty();
        
        // Calculate content height
        let content_height = if result_count > 0 {
            // Search bar + divider + results
            SEARCH_BAR_HEIGHT.0 + 1.0 + (result_count as f32 * RESULT_ITEM_HEIGHT.0)
        } else if !query_empty {
            // Search bar + no results message (approx 60px)
            SEARCH_BAR_HEIGHT.0 + 60.0
        } else {
            // Search bar + empty state (approx 60px)
            SEARCH_BAR_HEIGHT.0 + 60.0
        };
        
        let new_height = content_height
            .max(LAUNCHER_MIN_HEIGHT.0)
            .min(LAUNCHER_MAX_HEIGHT.0);
        
        // Spawn async task to resize after current frame completes
        cx.spawn(|_, _| async move {
            // Small delay to ensure we're outside GPUI's borrow
            gpui::Timer::after(Duration::from_millis(1)).await;
            resize_window_height(new_height as f64);
        }).detach();
    }

    // Action handlers

    /// Starts the selection change animation.
    fn start_selection_animation(&mut self, previous_index: usize, cx: &mut ViewContext<Self>) {
        self.previous_selected_index = Some(previous_index);
        let duration = selection_change_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.selection_animation_start = None;
            self.previous_selected_index = None;
        } else {
            self.selection_animation_start = Some(Instant::now());
            // Schedule animation updates
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16);
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if let Some(start) = this.selection_animation_start {
                                let elapsed = start.elapsed();
                                let total = selection_change_duration();
                                if elapsed >= total {
                                    this.selection_animation_start = None;
                                    this.previous_selected_index = None;
                                    cx.notify();
                                    return false;
                                }
                                cx.notify();
                                return true;
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }
    }

    /// Calculates the selection animation progress (0.0 to 1.0).
    #[allow(dead_code)]
    fn selection_animation_progress(&self) -> f32 {
        if let Some(start) = self.selection_animation_start {
            let elapsed = start.elapsed();
            let total = selection_change_duration();
            if total.is_zero() {
                1.0
            } else {
                ease_in_out((elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0))
            }
        } else {
            1.0
        }
    }

    fn select_next(&mut self, _: &SelectNext, cx: &mut ViewContext<Self>) {
        if !self.results.is_empty() {
            let previous = self.selected_index;
            let new_index = (self.selected_index + 1).min(self.results.len() - 1);
            if new_index != previous {
                self.selected_index = new_index;
                self.start_selection_animation(previous, cx);
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        }
    }

    fn select_previous(&mut self, _: &SelectPrevious, cx: &mut ViewContext<Self>) {
        if self.selected_index > 0 {
            let previous = self.selected_index;
            self.selected_index -= 1;
            self.start_selection_animation(previous, cx);
            self.ensure_selected_visible(cx);
            cx.notify();
        }
    }

    fn activate(&mut self, _: &Activate, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.results.get(self.selected_index) {
            // STUB: Result activation not yet implemented.
            // This should integrate with photoncast-core's launch module to:
            // - Launch applications via NSWorkspace
            // - Execute system commands via AppleScript
            // - Open files with default applications
            tracing::info!("Activating: {} (stub - not implemented)", result.title);
            self.hide(cx);
        }
    }

    fn cancel(&mut self, _: &Cancel, cx: &mut ViewContext<Self>) {
        if !self.query.is_empty() {
            // Clear query first
            self.query = SharedString::default();
            self.results.clear();
            self.selected_index = 0;
            self.update_window_height(cx);
            cx.notify();
        } else {
            // Close window with animation (hide() calls start_dismiss_animation which quits)
            self.hide(cx);
        }
    }

    fn quick_select(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        if index < self.results.len() {
            self.selected_index = index;
            self.activate(&Activate, cx);
        }
    }

    fn next_group(&mut self, _: &NextGroup, cx: &mut ViewContext<Self>) {
        if self.results.is_empty() {
            return;
        }

        // Find current group
        let current_type = self.results.get(self.selected_index).map(|r| r.result_type);

        if let Some(current_type) = current_type {
            // Find the first item of the next group
            let mut found_current = false;
            for (idx, result) in self.results.iter().enumerate() {
                if !found_current && result.result_type == current_type {
                    found_current = true;
                }
                if found_current && result.result_type != current_type {
                    self.selected_index = idx;
                    self.ensure_selected_visible(cx);
                    cx.notify();
                    return;
                }
            }

            // No next group found, wrap to first item
            self.selected_index = 0;
            self.ensure_selected_visible(cx);
        }
        cx.notify();
    }

    fn previous_group(&mut self, _: &PreviousGroup, cx: &mut ViewContext<Self>) {
        if self.results.is_empty() {
            return;
        }

        // Find current group
        let current_type = self.results.get(self.selected_index).map(|r| r.result_type);

        if let Some(current_type) = current_type {
            // Find the first item of current group
            let current_group_start = self
                .results
                .iter()
                .position(|r| r.result_type == current_type)
                .unwrap_or(0);

            if current_group_start > 0 {
                // Find the previous group's first item
                let prev_type = self.results[current_group_start - 1].result_type;
                let prev_group_start = self
                    .results
                    .iter()
                    .position(|r| r.result_type == prev_type)
                    .unwrap_or(0);
                self.selected_index = prev_group_start;
            } else {
                // Already at first group, wrap to last group's first item
                let last_type = self.results.last().map(|r| r.result_type);
                if let Some(last_type) = last_type {
                    let last_group_start = self
                        .results
                        .iter()
                        .position(|r| r.result_type == last_type)
                        .unwrap_or(0);
                    self.selected_index = last_group_start;
                }
            }
            self.ensure_selected_visible(cx);
        }
        cx.notify();
    }

    /// Ensures the selected item is visible by scrolling if needed.
    fn ensure_selected_visible(&self, _cx: &mut ViewContext<Self>) {
        // The GPUI scroll container handles this automatically when using
        // overflow_y_scroll(). The visible area is managed by the framework.
        // For more control, we would need to track scroll offset manually
        // and use ScrollHandle. The current implementation with automatic
        // scrolling and relatively small result lists is sufficient for MVP.
    }

    fn open_preferences(&mut self, _: &OpenPreferences, cx: &mut ViewContext<Self>) {
        // TODO: Open preferences window
        tracing::info!("Opening preferences...");
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        // Handle backspace
        if event.keystroke.key == "backspace" {
            if !self.query.is_empty() {
                let mut chars: Vec<char> = self.query.chars().collect();
                chars.pop();
                self.query = SharedString::from(chars.into_iter().collect::<String>());
                self.on_query_change(self.query.clone(), cx);
                cx.notify();
            }
            return;
        }

        // Ignore modifier-only keys and special keys handled by actions
        if event.keystroke.modifiers.platform
            || event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
        {
            return;
        }

        // Handle regular character input
        if let Some(ime_key) = &event.keystroke.ime_key {
            let new_query = format!("{}{}", self.query, ime_key);
            self.query = SharedString::from(new_query);
            self.on_query_change(self.query.clone(), cx);
            cx.notify();
        } else if event.keystroke.key.len() == 1 {
            // Single character key (a-z, 0-9, etc.)
            let key = if event.keystroke.modifiers.shift {
                event.keystroke.key.to_uppercase()
            } else {
                event.keystroke.key.clone()
            };
            let new_query = format!("{}{}", self.query, key);
            self.query = SharedString::from(new_query);
            self.on_query_change(self.query.clone(), cx);
            cx.notify();
        }
    }

    /// Render the search bar component
    fn render_search_bar(&self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .h(SEARCH_BAR_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .child(
                // Search icon
                div()
                    .size(SEARCH_ICON_SIZE)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x888888))
                    .child("🔍"),
            )
            .child(
                // Search input
                div().flex_1().h_full().flex().items_center().child(
                    div()
                        .w_full()
                        .text_size(px(16.0))
                        .text_color(rgb(0xffffff))
                        .when(self.query.is_empty(), |el| {
                            el.text_color(rgb(0x888888)).child("Search PhotonCast...")
                        })
                        .when(!self.query.is_empty(), |el| el.child(self.query.clone())),
                ),
            )
    }

    /// Render the results list component
    fn render_results(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Calculate height based on number of results
        let result_count = self.results.len().min(MAX_VISIBLE_RESULTS);
        let results_height = px(result_count as f32 * RESULT_ITEM_HEIGHT.0);
        
        div()
            .id("results-list")
            .w_full()
            .h(results_height)
            .overflow_y_scroll()
            .children(self.results.iter().enumerate().map(|(idx, result)| {
                let is_selected = idx == self.selected_index;
                self.render_result_item(result, idx, is_selected, cx)
            }))
    }

    /// Render a single result item
    fn render_result_item(
        &self,
        result: &ResultItem,
        index: usize,
        is_selected: bool,
        _cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let selected_bg = hsla(0.0, 0.0, 1.0, 0.1);
        let hover_bg = hsla(0.0, 0.0, 1.0, 0.05);
        div()
            .id(("result-item", index))
            .h(RESULT_ITEM_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .when(is_selected, |el| el.bg(selected_bg))
            .hover(|el| el.bg(hover_bg))
            .cursor_pointer()
            .child(
                // Icon
                div()
                    .size(px(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(24.0))
                    .child(result.icon.clone()),
            )
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
                            .text_color(rgb(0xffffff))
                            .truncate()
                            .child(result.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0x888888))
                            .truncate()
                            .child(result.subtitle.clone()),
                    ),
            )
            .child(
                // Shortcut badge
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x666666))
                    .child(format!("⌘{}", index + 1)),
            )
    }

    /// Render empty state when there's no query
    fn render_empty_state(&self) -> impl IntoElement {
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
                    .text_color(rgb(0x888888))
                    .child("Type to search apps, commands, and files"),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x666666))
                    .child("↑↓ Navigate  ↵ Open  esc Close"),
            )
    }

    /// Render "no results" state
    fn render_no_results(&self) -> impl IntoElement {
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
                    .text_color(rgb(0x888888))
                    .child(format!("No results for \"{}\"", self.query)),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x666666))
                    .child("Try a different search term"),
            )
    }
}

impl Render for LauncherWindow {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Calculate current animation opacity
        let opacity = self.current_opacity();

        // Main container with rounded corners and shadow
        div()
            .track_focus(&self.focus_handle)
            .key_context("LauncherWindow")
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::next_group))
            .on_action(cx.listener(Self::previous_group))
            .on_action(cx.listener(Self::open_preferences))
            .on_action(cx.listener(|this, _: &QuickSelect1, cx| this.quick_select(0, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect2, cx| this.quick_select(1, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect3, cx| this.quick_select(2, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect4, cx| this.quick_select(3, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect5, cx| this.quick_select(4, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect6, cx| this.quick_select(5, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect7, cx| this.quick_select(6, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect8, cx| this.quick_select(7, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect9, cx| this.quick_select(8, cx)))
            .size_full()
            .flex()
            .flex_col()
            // Apply window appear/dismiss animation opacity
            .opacity(opacity)
            // Catppuccin Mocha base color with slight transparency
            .bg(hsla(240.0 / 360.0, 0.21, 0.15, 0.95))
            .rounded(LAUNCHER_BORDER_RADIUS)
            .shadow_lg()
            .border_1()
            // Catppuccin surface0 with slight transparency
            .border_color(hsla(236.0 / 360.0, 0.13, 0.27, 0.8))
            .overflow_hidden()
            // Search bar
            .child(self.render_search_bar(cx))
            // Divider (only show when there are results or query)
            .when(!self.query.is_empty(), |el| {
                el.child(div().h(px(1.0)).w_full().bg(rgb(0x313244)))
            })
            // Results or empty state
            .when(self.query.is_empty(), |el| el.child(self.render_empty_state()))
            .when(!self.query.is_empty() && self.results.is_empty(), |el| {
                el.child(self.render_no_results())
            })
            .when(!self.results.is_empty(), |el| {
                el.child(self.render_results(cx))
            })
    }
}

impl FocusableView for LauncherWindow {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}
