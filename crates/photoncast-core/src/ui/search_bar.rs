//! Search input component with GPUI rendering.
//!
//! This module provides the search bar component for entering queries
//! in the PhotonCast launcher.

use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::theme::PhotonTheme;

/// Default height of the search bar in pixels.
pub const SEARCH_BAR_HEIGHT: Pixels = px(48.0);
/// Default icon size in pixels.
pub const SEARCH_ICON_SIZE: Pixels = px(20.0);
/// Default font size for input text.
pub const SEARCH_INPUT_FONT_SIZE: Pixels = px(16.0);
/// Default horizontal padding.
pub const SEARCH_BAR_PADDING_X: Pixels = px(16.0);
/// Debounce duration for input (single frame at 60 FPS).
pub const DEBOUNCE_DURATION: Duration = Duration::from_millis(16);

/// The search bar component for entering queries.
pub struct SearchBar {
    /// Current query text.
    query: SharedString,
    /// Placeholder text.
    placeholder: SharedString,
    /// Focus handle for keyboard input.
    focus_handle: FocusHandle,
    /// Callback for query changes (after debounce).
    on_change: Option<Box<dyn Fn(&SharedString, &mut WindowContext) + 'static>>,
    /// Last input time for debouncing.
    last_input_time: Option<Instant>,
    /// Pending query for debounce.
    pending_query: Option<SharedString>,
}

impl SearchBar {
    /// Creates a new search bar with the given placeholder.
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            query: SharedString::default(),
            placeholder: "Search PhotonCast...".into(),
            focus_handle: cx.focus_handle(),
            on_change: None,
            last_input_time: None,
            pending_query: None,
        }
    }

    /// Sets the placeholder text.
    pub fn set_placeholder(&mut self, placeholder: impl Into<SharedString>) {
        self.placeholder = placeholder.into();
    }

    /// Gets the current query.
    #[must_use]
    pub fn query(&self) -> &SharedString {
        &self.query
    }

    /// Sets the query and triggers on_change.
    pub fn set_query(&mut self, query: impl Into<SharedString>, cx: &mut ViewContext<Self>) {
        let new_query: SharedString = query.into();

        // Track input time for debouncing
        self.last_input_time = Some(Instant::now());
        self.pending_query = Some(new_query.clone());
        self.query = new_query;

        // Schedule debounced callback
        cx.spawn(|this, mut cx| async move {
            // Wait for debounce period
            tokio::time::sleep(DEBOUNCE_DURATION).await;

            this.update(&mut cx, |this, cx| {
                // Only emit if this is still the latest pending query
                if let Some(pending) = &this.pending_query {
                    if let Some(last_time) = this.last_input_time {
                        if last_time.elapsed() >= DEBOUNCE_DURATION {
                            if let Some(callback) = &this.on_change {
                                callback(pending, cx);
                            }
                            this.pending_query = None;
                        }
                    }
                }
            })
            .ok();
        })
        .detach();

        cx.notify();
    }

    /// Clears the current query.
    pub fn clear(&mut self, cx: &mut ViewContext<Self>) {
        self.query = SharedString::default();
        self.pending_query = None;

        if let Some(callback) = &self.on_change {
            callback(&self.query, cx);
        }

        cx.notify();
    }

    /// Sets the on_change callback (called after debounce).
    pub fn on_change(
        &mut self,
        callback: impl Fn(&SharedString, &mut WindowContext) + 'static,
    ) -> &mut Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Returns true if the query is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// Focuses the search bar.
    pub fn focus(&self, cx: &mut WindowContext) {
        cx.focus(&self.focus_handle);
    }

    /// Returns the focus handle.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Renders the search icon.
    fn render_icon(&self, theme: &PhotonTheme) -> impl IntoElement {
        div()
            .size(SEARCH_ICON_SIZE)
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.colors.text_muted.to_gpui())
            .child("🔍")
    }

    /// Renders the input area.
    fn render_input(&self, theme: &PhotonTheme, is_focused: bool) -> impl IntoElement {
        let placeholder_color = theme.colors.text_placeholder.to_gpui();
        let text_color = theme.colors.text.to_gpui();
        let placeholder = self.placeholder.clone();
        let query = self.query.clone();
        let is_empty = self.query.is_empty();

        div().flex_1().h_full().flex().items_center().child(
            div()
                .w_full()
                .text_size(SEARCH_INPUT_FONT_SIZE)
                .when(is_empty, |el: Div| {
                    el.text_color(placeholder_color)
                        .child(placeholder.clone())
                })
                .when(!is_empty, |el: Div| {
                    el.text_color(text_color)
                        .child(query.clone())
                })
                .when(is_focused && is_empty, |el: Div| {
                    // Show blinking cursor indicator when focused
                    el.child("|")
                }),
        )
    }
}

impl Render for SearchBar {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
        let is_focused = self.focus_handle.is_focused(cx);
        let border_focused = theme.colors.border_focused.to_gpui();
        let surface_bg = theme.colors.surface.to_gpui();

        div()
            .track_focus(&self.focus_handle)
            .h(SEARCH_BAR_HEIGHT)
            .w_full()
            .px(SEARCH_BAR_PADDING_X)
            .flex()
            .items_center()
            .gap_3()
            .bg(surface_bg)
            // Focus indicator - border color change
            .when(is_focused, |el| {
                el.border_1()
                    .border_color(border_focused)
                    .rounded_md()
            })
            .when(!is_focused, |el| {
                el.border_1()
                    .border_color(gpui::transparent_black())
                    .rounded_md()
            })
            .child(self.render_icon(&theme))
            .child(self.render_input(&theme, is_focused))
    }
}

impl FocusableView for SearchBar {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_bar_constants() {
        assert_eq!(SEARCH_BAR_HEIGHT, px(48.0));
        assert_eq!(SEARCH_ICON_SIZE, px(20.0));
        assert_eq!(SEARCH_INPUT_FONT_SIZE, px(16.0));
    }

    #[test]
    fn test_debounce_duration() {
        // 16ms is approximately one frame at 60 FPS
        assert_eq!(DEBOUNCE_DURATION, Duration::from_millis(16));
    }
}
