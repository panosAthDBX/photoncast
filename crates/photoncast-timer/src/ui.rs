//! UI components for timer display.
//!
//! This module provides GPUI components for displaying timer countdown
//! and managing timers through the UI.

use gpui::prelude::*;
use gpui::{div, px, IntoElement, ParentElement, Styled, ViewContext};
use photoncast_theme::GpuiThemeColors;

use crate::scheduler::{ActiveTimer, TimerAction};

/// Type alias – timer UI uses the shared [`GpuiThemeColors`].
type TimerColors = GpuiThemeColors;

fn get_timer_colors<V: 'static>(cx: &ViewContext<V>) -> TimerColors {
    TimerColors::from_context(cx)
}

/// Timer display component showing countdown and action info.
type CancelCallback = Box<dyn Fn(&mut ViewContext<TimerDisplay>) + 'static>;

pub struct TimerDisplay {
    timer: Option<ActiveTimer>,
    on_cancel: Option<CancelCallback>,
}

impl TimerDisplay {
    /// Creates a new timer display.
    #[must_use]
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        Self {
            timer: None,
            on_cancel: None,
        }
    }

    /// Updates the displayed timer.
    pub fn set_timer(&mut self, timer: Option<ActiveTimer>, cx: &mut ViewContext<Self>) {
        self.timer = timer;
        cx.notify();
    }

    /// Sets the cancel callback.
    pub fn on_cancel<F: Fn(&mut ViewContext<Self>) + 'static>(&mut self, callback: F) {
        self.on_cancel = Some(Box::new(callback));
    }

    /// Returns the icon for the timer action.
    const fn action_icon(action: TimerAction) -> &'static str {
        match action {
            TimerAction::Sleep => "\u{1F319}",   // Moon
            TimerAction::Shutdown => "\u{23FB}", // Power
            TimerAction::Restart => "\u{1F504}", // Refresh
            TimerAction::Lock => "\u{1F512}",    // Lock
        }
    }

    /// Calculates progress percentage (0.0 to 1.0).
    fn calculate_progress(timer: &ActiveTimer) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let total = (timer.execute_at - timer.created_at).num_seconds() as f32;
        #[allow(clippy::cast_precision_loss)]
        let remaining = timer.remaining().num_seconds() as f32;

        if total <= 0.0 {
            return 1.0;
        }

        let elapsed = total - remaining.max(0.0);
        (elapsed / total).clamp(0.0, 1.0)
    }
}

impl Render for TimerDisplay {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_timer_colors(cx);
        self.timer.as_ref().map_or_else(
            || {
                div()
                    .rounded_lg()
                    .bg(colors.background)
                    .p(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(colors.text_muted)
                    .child("No active timer")
            },
            |timer| {
                let countdown = timer.countdown_string();
                let action_name = timer.action.display_name();
                let icon = Self::action_icon(timer.action);
                let progress = Self::calculate_progress(timer);

                div()
                    .rounded_lg()
                    .bg(colors.background)
                    .p(px(16.0))
                    .gap(px(12.0))
                    .flex()
                    .flex_col()
                    .child(
                        // Header with icon and action
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(div().text_xl().child(icon))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text_muted)
                                    .child(format!("{action_name} in")),
                            ),
                    )
                    .child(
                        // Countdown display
                        div()
                            .text_3xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(colors.text)
                            .child(countdown),
                    )
                    .child(
                        // Progress bar
                        div()
                            .h(px(4.0))
                            .w_full()
                            .rounded_full()
                            .bg(colors.surface_hover)
                            .child(
                                div()
                                    .h_full()
                                    .rounded_full()
                                    .bg(colors.accent)
                                    .w(gpui::relative(progress)),
                            ),
                    )
                    .child(
                        // Cancel button (click handler to be connected via parent)
                        div()
                            .mt(px(8.0))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded_md()
                            .bg(colors.surface)
                            .text_sm()
                            .text_color(colors.error)
                            .cursor_pointer()
                            .child("Cancel Timer"),
                    )
            },
        )
    }
}

/// Compact timer countdown for status bar / result item display.
pub struct TimerCountdown {
    timer: Option<ActiveTimer>,
}

impl TimerCountdown {
    /// Creates a new timer countdown.
    #[must_use]
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        Self { timer: None }
    }

    /// Updates the countdown timer.
    pub fn set_timer(&mut self, timer: Option<ActiveTimer>, cx: &mut ViewContext<Self>) {
        self.timer = timer;
        cx.notify();
    }

    /// Returns the icon for the timer action.
    const fn action_icon(action: TimerAction) -> &'static str {
        match action {
            TimerAction::Sleep => "\u{1F319}",
            TimerAction::Shutdown => "\u{23FB}",
            TimerAction::Restart => "\u{1F504}",
            TimerAction::Lock => "\u{1F512}",
        }
    }
}

impl Render for TimerCountdown {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_timer_colors(cx);
        self.timer.as_ref().map_or_else(div, |timer| {
            let countdown = timer.countdown_string();
            let icon = Self::action_icon(timer.action);

            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(8.0))
                .py(px(4.0))
                .rounded_md()
                .bg(colors.surface_hover)
                .child(div().text_sm().child(icon))
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(colors.text)
                        .child(countdown),
                )
        })
    }
}

/// Timer set confirmation view.
pub struct TimerSetConfirmation {
    action: TimerAction,
    countdown: String,
}

impl TimerSetConfirmation {
    /// Creates a new confirmation view.
    #[must_use]
    pub fn new(timer: &ActiveTimer, _cx: &mut ViewContext<Self>) -> Self {
        Self {
            action: timer.action,
            countdown: timer.countdown_string(),
        }
    }
}

impl Render for TimerSetConfirmation {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_timer_colors(cx);
        let icon = match self.action {
            TimerAction::Sleep => "\u{1F319}",
            TimerAction::Shutdown => "\u{23FB}",
            TimerAction::Restart => "\u{1F504}",
            TimerAction::Lock => "\u{1F512}",
        };

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .p(px(12.0))
            .rounded_lg()
            .bg(colors.selection)
            .child(div().text_2xl().child(icon))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(colors.success)
                            .child("Timer Set"),
                    )
                    .child(div().text_xs().text_color(colors.success).child(format!(
                        "{} in {}",
                        self.action.display_name(),
                        self.countdown
                    ))),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_progress_calculation() {
        let timer = ActiveTimer::new(TimerAction::Sleep, Utc::now() + Duration::minutes(15));

        let progress = TimerDisplay::calculate_progress(&timer);
        assert!(progress >= 0.0);
        assert!(progress <= 1.0);
        // Progress should be near 0 for a freshly created timer
        assert!(progress < 0.1);
    }

    #[test]
    fn test_action_icons() {
        assert!(!TimerDisplay::action_icon(TimerAction::Sleep).is_empty());
        assert!(!TimerDisplay::action_icon(TimerAction::Shutdown).is_empty());
        assert!(!TimerDisplay::action_icon(TimerAction::Restart).is_empty());
        assert!(!TimerDisplay::action_icon(TimerAction::Lock).is_empty());
    }
}
