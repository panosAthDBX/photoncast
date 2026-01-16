//! Section grouping component for result types.
//!
//! This module provides the `ResultGroup` component for displaying
//! section headers that categorize results (Apps, Commands, Files).

use gpui::*;

use crate::search::ResultType;
use crate::theme::PhotonTheme;

/// Height of a group header in pixels.
pub const GROUP_HEADER_HEIGHT: Pixels = px(32.0);
/// Horizontal padding for group header.
pub const GROUP_HEADER_PADDING_X: Pixels = px(16.0);

/// A group header for categorizing results.
#[derive(Debug, Clone)]
pub struct ResultGroup {
    /// The type of results in this group.
    result_type: ResultType,
    /// The display name of the group.
    name: SharedString,
    /// Shortcut range hint (e.g., "⌘1-5").
    shortcut_hint: Option<SharedString>,
    /// Number of items in this group.
    count: usize,
    /// Starting shortcut index for this group.
    shortcut_start: Option<usize>,
}

impl ResultGroup {
    /// Creates a new result group for the given type.
    pub fn new(result_type: ResultType) -> Self {
        Self {
            result_type,
            name: result_type.display_name().into(),
            shortcut_hint: None,
            count: 0,
            shortcut_start: None,
        }
    }

    /// Sets the number of items in this group.
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Sets the shortcut hint (e.g., "⌘1-5").
    #[must_use]
    pub fn shortcut_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.shortcut_hint = Some(hint.into());
        self
    }

    /// Sets the starting shortcut index and calculates the hint.
    #[must_use]
    pub fn shortcut_start(mut self, start: usize) -> Self {
        self.shortcut_start = Some(start);
        self
    }

    /// Builds the shortcut hint based on start index and count.
    fn build_shortcut_hint(&self) -> Option<String> {
        let start = self.shortcut_start?;
        if self.count == 0 || start >= 9 {
            return None;
        }

        let end = (start + self.count).min(9);
        if end == start + 1 {
            Some(format!("⌘{}", start + 1))
        } else {
            Some(format!("⌘{}-{}", start + 1, end))
        }
    }

    /// Returns the result type.
    #[must_use]
    pub fn result_type(&self) -> ResultType {
        self.result_type
    }

    /// Returns the display name.
    #[must_use]
    pub fn name(&self) -> &SharedString {
        &self.name
    }

    /// Returns the item count.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.count
    }
}

impl IntoElement for ResultGroup {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        // Get theme from global or use default
        let theme = PhotonTheme::default();

        let hint = self
            .shortcut_hint
            .clone()
            .map(|s| s.to_string())
            .or_else(|| self.build_shortcut_hint());

        let bg_color = theme.colors.background.to_gpui();
        let border_color = theme.colors.border.to_gpui();
        let text_muted_color = theme.colors.text_muted.to_gpui();
        let placeholder_color = theme.colors.text_placeholder.to_gpui();
        // Uppercase the name manually since GPUI doesn't have text-transform
        let name_upper: SharedString = self.name.to_uppercase().into();

        let mut result = div()
            .h(GROUP_HEADER_HEIGHT)
            .w_full()
            .px(GROUP_HEADER_PADDING_X)
            .flex()
            .items_center()
            .justify_between()
            .bg(bg_color)
            .border_b_1()
            .border_color(border_color)
            .child(
                // Group name
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text_muted_color)
                    .child(name_upper),
            );

        if let Some(hint_text) = hint {
            result = result.child(
                // Shortcut hint
                div()
                    .text_size(px(10.0))
                    .text_color(placeholder_color)
                    .child(hint_text),
            );
        }

        result
    }
}

/// A result group with its items for grouped rendering.
#[derive(Debug, Clone)]
pub struct ResultGroupWithItems<T> {
    /// The group header.
    pub header: ResultGroup,
    /// The items in this group.
    pub items: Vec<T>,
}

impl<T> ResultGroupWithItems<T> {
    /// Creates a new group with items.
    pub fn new(result_type: ResultType, items: Vec<T>) -> Self {
        let header = ResultGroup::new(result_type).count(items.len());
        Self { header, items }
    }

    /// Sets the starting shortcut index.
    #[must_use]
    pub fn shortcut_start(mut self, start: usize) -> Self {
        self.header = self.header.shortcut_start(start);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_group_new() {
        let group = ResultGroup::new(ResultType::Application);

        assert_eq!(group.result_type(), ResultType::Application);
        assert_eq!(group.name().as_ref(), "Apps");
        assert_eq!(group.item_count(), 0);
    }

    #[test]
    fn test_result_group_with_count() {
        let group = ResultGroup::new(ResultType::SystemCommand).count(5);

        assert_eq!(group.item_count(), 5);
    }

    #[test]
    fn test_result_group_with_shortcut_hint() {
        let group = ResultGroup::new(ResultType::File).shortcut_hint("⌘6-9");

        assert_eq!(
            group.shortcut_hint.as_ref().map(|s| s.as_ref()),
            Some("⌘6-9")
        );
    }

    #[test]
    fn test_build_shortcut_hint_single() {
        let group = ResultGroup::new(ResultType::Application)
            .count(1)
            .shortcut_start(0);

        assert_eq!(group.build_shortcut_hint(), Some("⌘1".to_string()));
    }

    #[test]
    fn test_build_shortcut_hint_range() {
        let group = ResultGroup::new(ResultType::Application)
            .count(5)
            .shortcut_start(0);

        assert_eq!(group.build_shortcut_hint(), Some("⌘1-5".to_string()));
    }

    #[test]
    fn test_build_shortcut_hint_partial_range() {
        let group = ResultGroup::new(ResultType::SystemCommand)
            .count(3)
            .shortcut_start(4);

        assert_eq!(group.build_shortcut_hint(), Some("⌘5-7".to_string()));
    }

    #[test]
    fn test_build_shortcut_hint_overflow() {
        let group = ResultGroup::new(ResultType::File)
            .count(5)
            .shortcut_start(7);

        // Only ⌘8-9 are available (indices 7, 8)
        assert_eq!(group.build_shortcut_hint(), Some("⌘8-9".to_string()));
    }

    #[test]
    fn test_build_shortcut_hint_none_when_out_of_range() {
        let group = ResultGroup::new(ResultType::File)
            .count(5)
            .shortcut_start(9); // Start at 9, which is >= 9

        assert_eq!(group.build_shortcut_hint(), None);
    }

    #[test]
    fn test_build_shortcut_hint_none_when_empty() {
        let group = ResultGroup::new(ResultType::File)
            .count(0)
            .shortcut_start(0);

        assert_eq!(group.build_shortcut_hint(), None);
    }

    #[test]
    fn test_result_group_with_items() {
        let items = vec!["a", "b", "c"];
        let group = ResultGroupWithItems::new(ResultType::Application, items.clone());

        assert_eq!(group.header.item_count(), 3);
        assert_eq!(group.items.len(), 3);
    }

    #[test]
    fn test_result_group_with_items_shortcut() {
        let items = vec!["a", "b", "c"];
        let group =
            ResultGroupWithItems::new(ResultType::Application, items.clone()).shortcut_start(2);

        assert_eq!(group.header.build_shortcut_hint(), Some("⌘3-5".to_string()));
    }

    #[test]
    fn test_group_header_constants() {
        assert_eq!(GROUP_HEADER_HEIGHT, px(32.0));
        assert_eq!(GROUP_HEADER_PADDING_X, px(16.0));
    }

    #[test]
    fn test_result_type_display_names() {
        assert_eq!(ResultType::Application.display_name(), "Apps");
        assert_eq!(ResultType::SystemCommand.display_name(), "Commands");
        assert_eq!(ResultType::File.display_name(), "Files");
        assert_eq!(ResultType::Folder.display_name(), "Folders");
    }

    #[test]
    fn test_result_type_priority() {
        assert_eq!(ResultType::Application.priority(), 0);
        assert_eq!(ResultType::SystemCommand.priority(), 1);
        assert_eq!(ResultType::File.priority(), 2);
        assert_eq!(ResultType::Folder.priority(), 3);
    }
}
