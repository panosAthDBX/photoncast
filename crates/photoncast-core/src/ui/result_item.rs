//! Individual result row component with GPUI rendering.
//!
//! This module provides the `ResultItem` component for displaying
//! individual search results in the launcher.

use std::ops::Range;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::search::{IconSource, ResultType, SearchAction};
use crate::theme::PhotonTheme;

/// Default height of a result item in pixels.
pub const RESULT_ITEM_HEIGHT: Pixels = px(56.0);
/// Default icon size in pixels.
pub const RESULT_ICON_SIZE: Pixels = px(32.0);
/// Default horizontal padding in pixels.
pub const RESULT_PADDING_X: Pixels = px(16.0);
/// Default vertical padding in pixels.
pub const RESULT_PADDING_Y: Pixels = px(8.0);

/// A single result item in the results list.
///
/// This component uses `IntoElement` for efficient rendering within lists.
#[derive(Clone)]
pub struct ResultItem {
    /// Unique identifier for this result.
    id: SharedString,
    /// The display title.
    title: SharedString,
    /// The subtitle/description.
    subtitle: SharedString,
    /// Icon source.
    icon: IconSource,
    /// Type of result (for grouping).
    result_type: ResultType,
    /// Keyboard shortcut (e.g., "⌘1").
    shortcut: Option<SharedString>,
    /// Ranges of characters to highlight in the title.
    match_ranges: Vec<Range<usize>>,
    /// Whether this item is currently selected.
    is_selected: bool,
    /// Whether the mouse is hovering over this item.
    is_hovered: bool,
    /// Action to perform when activated.
    action: Option<SearchAction>,
    /// Click handler.
    on_click: Option<SharedString>,
}

impl ResultItem {
    /// Creates a new result item with the given title and subtitle.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            icon: IconSource::Emoji { char: '📄' },
            result_type: ResultType::File,
            shortcut: None,
            match_ranges: Vec::new(),
            is_selected: false,
            is_hovered: false,
            action: None,
            on_click: None,
        }
    }

    /// Sets the icon source.
    #[must_use]
    pub fn icon(mut self, icon: IconSource) -> Self {
        self.icon = icon;
        self
    }

    /// Sets the result type.
    #[must_use]
    pub fn result_type(mut self, result_type: ResultType) -> Self {
        self.result_type = result_type;
        self
    }

    /// Sets the keyboard shortcut.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Sets the match ranges for highlighting.
    #[must_use]
    pub fn match_ranges(mut self, ranges: Vec<Range<usize>>) -> Self {
        self.match_ranges = ranges;
        self
    }

    /// Sets whether this item is selected.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.is_selected = selected;
        self
    }

    /// Sets whether this item is hovered.
    #[must_use]
    pub fn hovered(mut self, hovered: bool) -> Self {
        self.is_hovered = hovered;
        self
    }

    /// Sets the action to perform on activation.
    #[must_use]
    pub fn action(mut self, action: SearchAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Returns the unique ID.
    #[must_use]
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Renders the icon based on the icon source.
    fn render_icon(&self, theme: &PhotonTheme) -> impl IntoElement {
        // Use strings for emojis that may have combining marks
        let icon_str: String = match &self.icon {
            IconSource::AppIcon { .. } => "📱".to_string(),
            IconSource::SystemIcon { .. } => "⚙️".to_string(),
            IconSource::FileIcon { .. } => "📄".to_string(),
            IconSource::Emoji { char } => char.to_string(),
        };

        div()
            .size(RESULT_ICON_SIZE)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(24.0))
            .text_color(theme.colors.text.to_gpui())
            .rounded_md()
            .bg(theme.colors.surface_hover.to_gpui())
            .child(icon_str)
    }

    /// Renders the title with match highlighting.
    fn render_title(&self, theme: &PhotonTheme) -> impl IntoElement {
        let title_str = self.title.to_string();

        if self.match_ranges.is_empty() {
            // No highlighting needed
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.colors.text.to_gpui())
                .truncate()
                .child(self.title.clone())
        } else {
            // Build highlighted title
            let mut spans: Vec<(String, bool)> = Vec::new();
            let mut last_end = 0;

            for range in &self.match_ranges {
                // Add non-matching part before this range
                if range.start > last_end {
                    if let Some(s) = title_str.get(last_end..range.start) {
                        spans.push((s.to_string(), false));
                    }
                }
                // Add matching part
                if let Some(s) = title_str.get(range.clone()) {
                    spans.push((s.to_string(), true));
                }
                last_end = range.end;
            }
            // Add remaining part after last range
            if last_end < title_str.len() {
                if let Some(s) = title_str.get(last_end..) {
                    spans.push((s.to_string(), false));
                }
            }

            let accent_color = theme.colors.accent.to_gpui();
            let text_color = theme.colors.text.to_gpui();

            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::MEDIUM)
                .truncate()
                .flex()
                .children(spans.into_iter().map(move |(text, is_match)| {
                    div()
                        .when(is_match, |el: Div| {
                            el.text_color(accent_color)
                                .font_weight(FontWeight::BOLD)
                        })
                        .when(!is_match, |el: Div| el.text_color(text_color))
                        .child(text)
                }))
        }
    }

    /// Renders the subtitle.
    fn render_subtitle(&self, theme: &PhotonTheme) -> impl IntoElement {
        div()
            .text_size(px(12.0))
            .text_color(theme.colors.text_muted.to_gpui())
            .truncate()
            .child(self.subtitle.clone())
    }

    /// Renders the shortcut badge.
    fn render_shortcut(&self, theme: &PhotonTheme) -> Option<impl IntoElement> {
        self.shortcut.as_ref().map(|shortcut| {
            div()
                .text_size(px(11.0))
                .text_color(theme.colors.text_muted.to_gpui())
                .px_2()
                .py_0p5()
                .rounded_sm()
                .bg(theme.colors.surface_hover.to_gpui())
                .child(shortcut.clone())
        })
    }
}

impl IntoElement for ResultItem {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        // Get theme from global or use default
        // Note: This is a simplified version - in actual use, theme should be passed through context
        let theme = PhotonTheme::default();

        let bg_color = if self.is_selected {
            theme.colors.surface_selected
        } else if self.is_hovered {
            theme.colors.surface_hover
        } else {
            theme.colors.surface
        };

        let hover_bg = theme.colors.surface_hover.to_gpui();
        let mut element = div()
            .id(ElementId::Name(self.id.clone()))
            .h(RESULT_ITEM_HEIGHT)
            .w_full()
            .px(RESULT_PADDING_X)
            .py(RESULT_PADDING_Y)
            .flex()
            .items_center()
            .gap_3()
            .bg(bg_color.to_gpui())
            .cursor_pointer()
            .hover(|el| el.bg(hover_bg));

        // Add selection border for selected items
        if self.is_selected {
            element = element
                .border_l_2()
                .border_color(theme.colors.accent.to_gpui());
        }

        let mut element = element
            .child(self.render_icon(&theme))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .overflow_hidden()
                    .child(self.render_title(&theme))
                    .child(self.render_subtitle(&theme)),
            );

        // Add shortcut badge if present
        if let Some(shortcut) = self.render_shortcut(&theme) {
            element = element.child(shortcut);
        }

        element
    }
}

/// A builder for creating result items from search results.
impl From<&crate::search::SearchResult> for ResultItem {
    fn from(result: &crate::search::SearchResult) -> Self {
        let _match_ranges: Vec<Range<usize>> = result
            .match_indices
            .windows(2)
            .filter_map(|w| {
                if w[1] == w[0] + 1 {
                    None // Part of a continuous range, will be handled
                } else {
                    Some(w[0]..w[0] + 1)
                }
            })
            .collect();

        // Convert match indices to ranges (consecutive indices form ranges)
        let ranges = indices_to_ranges(&result.match_indices);

        Self::new(
            result.id.to_string(),
            result.title.clone(),
            result.subtitle.clone(),
        )
        .icon(result.icon.clone())
        .result_type(result.result_type)
        .match_ranges(ranges)
        .action(result.action.clone())
    }
}

/// Converts a list of match indices to ranges of consecutive indices.
fn indices_to_ranges(indices: &[usize]) -> Vec<Range<usize>> {
    if indices.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut indices = indices.to_vec();
    indices.sort_unstable();
    indices.dedup();

    let mut start = indices[0];
    let mut end = indices[0];

    for &idx in indices.iter().skip(1) {
        if idx == end + 1 {
            end = idx;
        } else {
            ranges.push(start..end + 1);
            start = idx;
            end = idx;
        }
    }
    ranges.push(start..end + 1);

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_item_constants() {
        assert_eq!(RESULT_ITEM_HEIGHT, px(56.0));
        assert_eq!(RESULT_ICON_SIZE, px(32.0));
        assert_eq!(RESULT_PADDING_X, px(16.0));
    }

    #[test]
    fn test_result_item_builder() {
        let item = ResultItem::new("test", "Test Title", "Test Subtitle")
            .selected(true)
            .shortcut("⌘1");

        assert_eq!(item.title.as_ref(), "Test Title");
        assert_eq!(item.subtitle.as_ref(), "Test Subtitle");
        assert!(item.is_selected);
        assert_eq!(item.shortcut.as_ref().map(|s| s.as_ref()), Some("⌘1"));
    }

    #[test]
    fn test_indices_to_ranges_empty() {
        assert!(indices_to_ranges(&[]).is_empty());
    }

    #[test]
    fn test_indices_to_ranges_single() {
        let ranges = indices_to_ranges(&[5]);
        assert_eq!(ranges, vec![5..6]);
    }

    #[test]
    fn test_indices_to_ranges_consecutive() {
        let ranges = indices_to_ranges(&[1, 2, 3, 4]);
        assert_eq!(ranges, vec![1..5]);
    }

    #[test]
    fn test_indices_to_ranges_gaps() {
        let ranges = indices_to_ranges(&[1, 2, 5, 6, 7, 10]);
        assert_eq!(ranges, vec![1..3, 5..8, 10..11]);
    }

    #[test]
    fn test_selection_states() {
        let normal = ResultItem::new("1", "Title", "Sub");
        let hovered = ResultItem::new("2", "Title", "Sub").hovered(true);
        let selected = ResultItem::new("3", "Title", "Sub").selected(true);

        assert!(!normal.is_selected);
        assert!(!normal.is_hovered);
        assert!(!hovered.is_selected);
        assert!(hovered.is_hovered);
        assert!(selected.is_selected);
        assert!(!selected.is_hovered);
    }

    #[test]
    fn test_match_ranges() {
        let item = ResultItem::new("test", "Safari", "Browser").match_ranges(vec![0..3, 4..6]);

        assert_eq!(item.match_ranges.len(), 2);
        assert_eq!(item.match_ranges[0], 0..3);
        assert_eq!(item.match_ranges[1], 4..6);
    }
}
