//! UI components for the calculator.
//!
//! This module provides GPUI views for displaying calculator results.

#[allow(unused_imports)]
use gpui::{
    div, prelude::*, px, rems, rgb, rgba, AnyElement, Element, ElementId, InteractiveElement,
    ParentElement, SharedString, Styled, View, ViewContext, WindowContext,
};

use crate::{CalculatorResult, CalculatorResultKind};

/// Calculator result view component.
pub struct CalculatorResultView {
    /// The result to display.
    result: Option<CalculatorResult>,
    /// Whether rates are being refreshed.
    is_refreshing: bool,
    /// Error message to display.
    error: Option<String>,
}

impl CalculatorResultView {
    /// Creates a new calculator result view.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            result: None,
            is_refreshing: false,
            error: None,
        }
    }

    /// Sets the result to display.
    pub fn set_result(&mut self, result: Option<CalculatorResult>) {
        self.result = result;
        self.error = None;
    }

    /// Sets the error message.
    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
        self.result = None;
    }

    /// Sets the refreshing state.
    pub fn set_refreshing(&mut self, refreshing: bool) {
        self.is_refreshing = refreshing;
    }

    /// Returns the current result.
    #[must_use]
    pub const fn result(&self) -> Option<&CalculatorResult> {
        self.result.as_ref()
    }
}

impl Default for CalculatorResultView {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for CalculatorResultView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let content = if let Some(error) = &self.error {
            // Error state
            div().p_4().child(
                div()
                        .text_color(rgb(0x00f3_8ba8)) // Catppuccin red
                        .text_sm()
                        .child(error.clone()),
            )
        } else if let Some(result) = &self.result {
            // Result state
            Self::render_result(result, cx)
        } else {
            // Empty state
            div().p_4().child(
                div()
                    .text_color(rgba(0xffff_ffaa))
                    .text_sm()
                    .child("Type an expression to calculate..."),
            )
        };

        div()
            .w_full()
            .bg(rgba(0x1e1e_2e99)) // Semi-transparent background
            .rounded_lg()
            .child(content)
    }
}

impl CalculatorResultView {
    /// Renders a calculator result.
    fn render_result(result: &CalculatorResult, _cx: &mut ViewContext<Self>) -> gpui::Div {
        let icon = match &result.kind {
            CalculatorResultKind::Math => "🔢",
            CalculatorResultKind::Currency { .. } => "💱",
            CalculatorResultKind::Unit { .. } => "📏",
            CalculatorResultKind::DateTime => "📅",
        };

        let title = match &result.kind {
            CalculatorResultKind::Math => "Math",
            CalculatorResultKind::Currency { .. } => "Currency Conversion",
            CalculatorResultKind::Unit { .. } => "Unit Conversion",
            CalculatorResultKind::DateTime => "Date/Time",
        };

        div()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_lg().child(icon))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xffff_ffaa))
                            .child(title),
                    ),
            )
            // Main result
            .child(
                div()
                    .text_2xl()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(0x00cd_d6f4)) // Catppuccin text
                    .child(result.formatted_value.clone()),
            )
            // Details
            .children(result.details.as_ref().map(|details| {
                div()
                    .text_sm()
                    .text_color(rgba(0xffff_ffaa))
                    .child(details.clone())
            }))
            // Update time (for currency)
            .children(result.last_updated.as_ref().map(|updated| {
                let ago = chrono::Utc::now().signed_duration_since(*updated);
                let ago_text = if ago.num_hours() > 0 {
                    format!("Updated {} hours ago", ago.num_hours())
                } else if ago.num_minutes() > 0 {
                    format!("Updated {} minutes ago", ago.num_minutes())
                } else {
                    "Just updated".to_string()
                };

                div()
                    .text_xs()
                    .text_color(rgba(0xffff_ff66))
                    .mt_2()
                    .child(ago_text)
            }))
    }
}

/// Calculator history view.
pub struct CalculatorHistoryView {
    /// History entries.
    history: Vec<CalculatorResult>,
    /// Selected index.
    selected_index: usize,
}

impl CalculatorHistoryView {
    /// Creates a new history view.
    #[must_use]
    pub const fn new(history: Vec<CalculatorResult>) -> Self {
        Self {
            history,
            selected_index: 0,
        }
    }

    /// Updates the history.
    pub fn set_history(&mut self, history: Vec<CalculatorResult>) {
        self.history = history;
        self.selected_index = 0;
    }

    /// Moves selection up.
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Moves selection down.
    pub fn select_next(&mut self) {
        if self.selected_index < self.history.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Returns the selected entry.
    #[must_use]
    pub fn selected(&self) -> Option<&CalculatorResult> {
        self.history.get(self.selected_index)
    }
}

impl Render for CalculatorHistoryView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        if self.history.is_empty() {
            return div().p_4().child(
                div()
                    .text_color(rgba(0xffff_ffaa))
                    .text_sm()
                    .child("No calculation history"),
            );
        }

        div()
            .p_2()
            .flex()
            .flex_col()
            .gap_1()
            .children(self.history.iter().enumerate().map(|(i, result)| {
                let is_selected = i == self.selected_index;

                div()
                    .id(ElementId::Name(format!("history-{}", i).into()))
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .when(is_selected, |el| el.bg(rgba(0xffff_ff11)))
                    .hover(|el| el.bg(rgba(0xffff_ff08)))
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(if is_selected {
                                rgb(0x00cd_d6f4)
                            } else {
                                rgba(0xffff_ffcc)
                            })
                            .child(result.expression.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(rgb(0x0089_b4fa)) // Catppuccin blue
                            .child(result.formatted_value.clone()),
                    )
            }))
    }
}

/// Action bar for calculator results.
pub struct CalculatorActionBar {
    /// Whether rates can be refreshed.
    can_refresh: bool,
}

impl CalculatorActionBar {
    /// Creates a new action bar.
    #[must_use]
    pub const fn new(can_refresh: bool) -> Self {
        Self { can_refresh }
    }
}

impl Render for CalculatorActionBar {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .px_4()
            .py_2()
            .flex()
            .justify_between()
            .items_center()
            .border_t_1()
            .border_color(rgba(0xffff_ff11))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .text_xs()
                    .text_color(rgba(0xffff_ffaa))
                    .child("⏎ Copy Formatted")
                    .child("⌘⏎ Copy Raw"),
            )
            .when(self.can_refresh, |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(rgba(0xffff_ffaa))
                        .child("⌘R Refresh"),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_calculator_result_view_new() {
        let view = CalculatorResultView::new();
        assert!(view.result.is_none());
        assert!(view.error.is_none());
    }

    #[test]
    fn test_calculator_history_view() {
        let mut view = CalculatorHistoryView::new(vec![]);
        assert!(view.selected().is_none());

        // Add some history
        let results = vec![
            CalculatorResult::math(
                "2+3".to_string(),
                5.0,
                "5".to_string(),
                Duration::from_millis(1),
            ),
            CalculatorResult::math(
                "10*2".to_string(),
                20.0,
                "20".to_string(),
                Duration::from_millis(1),
            ),
        ];
        view.set_history(results);

        assert!(view.selected().is_some());
        assert_eq!(view.selected().unwrap().expression, "2+3");

        view.select_next();
        assert_eq!(view.selected().unwrap().expression, "10*2");

        view.select_previous();
        assert_eq!(view.selected().unwrap().expression, "2+3");
    }
}
