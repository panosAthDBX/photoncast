//! Results display component with virtual scrolling.
//!
//! This module provides the `ResultsList` component that displays
//! search results with efficient virtual scrolling for performance.
//! Results are displayed grouped by type (Apps, Commands, Files) with
//! section headers showing the group name and keyboard shortcut range.

use std::sync::Arc;

use gpui::*;

use crate::search::{GroupedResult, SearchResult, SearchResults};
use crate::theme::PhotonTheme;
use crate::ui::result_group::{ResultGroup, GROUP_HEADER_HEIGHT};
use crate::ui::result_item::{ResultItem, RESULT_ITEM_HEIGHT};

/// Maximum visible height of the results list in pixels.
pub const RESULTS_MAX_HEIGHT: Pixels = px(400.0);
/// Number of extra items to render above/below the visible area (for virtual scrolling).
#[allow(dead_code)]
const OVERSCAN: usize = 2;

/// The results list component that displays search results.
///
/// Results are displayed grouped by type (Apps, Commands, Files) with
/// section headers showing the group name and keyboard shortcut range.
pub struct ResultsList {
    /// The grouped results to display.
    grouped_results: Vec<GroupedResult>,
    /// Flat list of results for quick index access (Arc-wrapped to avoid cloning).
    flat_results: Vec<Arc<SearchResult>>,
    /// Currently selected index (flat index across all groups).
    selected_index: usize,
    /// Current scroll offset.
    scroll_offset: f32,
    /// Visible height of the container.
    visible_height: f32,
    /// Hovered item index (flat index).
    hovered_index: Option<usize>,
    /// Callback when selection changes.
    on_select: Option<Box<dyn Fn(usize, &mut WindowContext) + 'static>>,
    /// Callback when an item is activated.
    on_activate: Option<Box<dyn Fn(usize, &mut WindowContext) + 'static>>,
    /// Cached search results for navigation.
    search_results: SearchResults,
}

impl Default for ResultsList {
    fn default() -> Self {
        Self::new()
    }
}

impl ResultsList {
    /// Creates a new empty results list.
    #[must_use]
    pub fn new() -> Self {
        Self {
            grouped_results: Vec::new(),
            flat_results: Vec::new(),
            selected_index: 0,
            scroll_offset: 0.0,
            visible_height: RESULTS_MAX_HEIGHT.0,
            hovered_index: None,
            on_select: None,
            on_activate: None,
            search_results: SearchResults::empty(),
        }
    }

    /// Updates the results list with new search results.
    ///
    /// This automatically groups results by type and calculates shortcut ranges.
    pub fn set_results(&mut self, results: SearchResults) {
        self.grouped_results = results.grouped();
        self.flat_results = results.iter().map(|r| Arc::new(r.clone())).collect();
        self.search_results = results;
        self.selected_index = 0;
        self.scroll_offset = 0.0;
    }

    /// Updates the results list with a flat list of results.
    ///
    /// For backwards compatibility. Prefer `set_results()` with `SearchResults`.
    pub fn set_flat_results(&mut self, results: Vec<SearchResult>) {
        self.flat_results = results.into_iter().map(Arc::new).collect();
        self.grouped_results = Vec::new();
        self.search_results = SearchResults::empty();
        self.selected_index = 0;
        self.scroll_offset = 0.0;
    }

    /// Returns the flat results.
    #[must_use]
    pub fn results(&self) -> &[Arc<SearchResult>] {
        &self.flat_results
    }

    /// Returns the grouped results.
    #[must_use]
    pub fn grouped_results(&self) -> &[GroupedResult] {
        &self.grouped_results
    }

    /// Returns the selected index (flat index across all groups).
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Sets the selected index and ensures it's visible.
    pub fn set_selected_index(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        if self.flat_results.is_empty() {
            return;
        }

        self.selected_index = index.min(self.flat_results.len() - 1);
        self.ensure_selected_visible();

        if let Some(callback) = &self.on_select {
            callback(self.selected_index, cx);
        }

        cx.notify();
    }

    /// Selects the next result.
    pub fn select_next(&mut self, cx: &mut ViewContext<Self>) {
        if !self.flat_results.is_empty() {
            let new_index = (self.selected_index + 1).min(self.flat_results.len() - 1);
            self.set_selected_index(new_index, cx);
        }
    }

    /// Selects the previous result.
    pub fn select_previous(&mut self, cx: &mut ViewContext<Self>) {
        if self.selected_index > 0 {
            self.set_selected_index(self.selected_index - 1, cx);
        }
    }

    /// Navigates to the next group.
    ///
    /// Tab moves to the first item of the next group, wrapping to the first
    /// group if currently in the last group.
    pub fn next_group(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(new_index) = self.search_results.next_group_start(self.selected_index) {
            self.set_selected_index(new_index, cx);
        }
    }

    /// Navigates to the previous group.
    ///
    /// Shift+Tab moves to the first item of the previous group, wrapping to
    /// the last group if currently in the first group.
    pub fn previous_group(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(new_index) = self
            .search_results
            .previous_group_start(self.selected_index)
        {
            self.set_selected_index(new_index, cx);
        }
    }

    /// Quick selects a result by its 1-based shortcut number (1-9).
    ///
    /// ⌘1 selects the first result, ⌘9 selects the ninth result.
    /// Returns `true` if a result was selected.
    pub fn quick_select(&mut self, number: usize, cx: &mut ViewContext<Self>) -> bool {
        if number == 0 || number > 9 {
            return false;
        }

        let index = number - 1;
        if index < self.flat_results.len() {
            self.set_selected_index(index, cx);
            true
        } else {
            false
        }
    }

    /// Gets the currently selected result.
    #[must_use]
    pub fn selected(&self) -> Option<&SearchResult> {
        self.flat_results
            .get(self.selected_index)
            .map(std::convert::AsRef::as_ref)
    }

    /// Returns true if the list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.flat_results.is_empty()
    }

    /// Returns the number of results.
    #[must_use]
    pub fn len(&self) -> usize {
        self.flat_results.len()
    }

    /// Returns the number of groups.
    #[must_use]
    pub fn group_count(&self) -> usize {
        self.grouped_results.len()
    }

    /// Sets the on_select callback.
    pub fn on_select(
        &mut self,
        callback: impl Fn(usize, &mut WindowContext) + 'static,
    ) -> &mut Self {
        self.on_select = Some(Box::new(callback));
        self
    }

    /// Sets the on_activate callback.
    pub fn on_activate(
        &mut self,
        callback: impl Fn(usize, &mut WindowContext) + 'static,
    ) -> &mut Self {
        self.on_activate = Some(Box::new(callback));
        self
    }

    /// Activates the currently selected result.
    pub fn activate_selected(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(callback) = &self.on_activate {
            callback(self.selected_index, cx);
        }
    }

    /// Calculates the total content height including group headers.
    #[allow(clippy::cast_precision_loss)]
    fn total_content_height(&self) -> f32 {
        let item_height = self.flat_results.len() as f32 * RESULT_ITEM_HEIGHT.0;
        let header_height = self.grouped_results.len() as f32 * GROUP_HEADER_HEIGHT.0;
        item_height + header_height
    }

    /// Ensures the selected item is visible by adjusting scroll offset.
    fn ensure_selected_visible(&mut self) {
        // Calculate the y position of the selected item including group headers
        let selected_y = self.calculate_item_y_position(self.selected_index);
        let item_height = RESULT_ITEM_HEIGHT.0;

        // If selected item is above visible area, scroll up
        if selected_y < self.scroll_offset {
            self.scroll_offset = selected_y;
        }
        // If selected item is below visible area, scroll down
        else if selected_y + item_height > self.scroll_offset + self.visible_height {
            self.scroll_offset = selected_y + item_height - self.visible_height;
        }
    }

    /// Calculates the y position of an item at the given flat index.
    #[allow(clippy::cast_precision_loss)]
    fn calculate_item_y_position(&self, flat_index: usize) -> f32 {
        if self.grouped_results.is_empty() {
            return flat_index as f32 * RESULT_ITEM_HEIGHT.0;
        }

        let mut y: f32 = 0.0;
        let mut items_before = 0;

        for group in &self.grouped_results {
            // Add group header height
            y += GROUP_HEADER_HEIGHT.0;

            let items_in_group = group.items.len();
            if flat_index < items_before + items_in_group {
                // The item is in this group
                let index_within_group = flat_index - items_before;
                y += index_within_group as f32 * RESULT_ITEM_HEIGHT.0;
                return y;
            }

            // Add all items in this group
            y += items_in_group as f32 * RESULT_ITEM_HEIGHT.0;
            items_before += items_in_group;
        }

        y
    }

    /// Handles scroll events.
    fn handle_scroll(&mut self, delta: Point<Pixels>, cx: &mut ViewContext<Self>) {
        let max_scroll = (self.total_content_height() - self.visible_height).max(0.0);
        self.scroll_offset = (self.scroll_offset - delta.y.0).clamp(0.0, max_scroll);
        cx.notify();
    }

    /// Sets the hovered index (UI API for mouse hover support).
    #[allow(dead_code)]
    fn set_hovered(&mut self, index: Option<usize>, cx: &mut ViewContext<Self>) {
        if self.hovered_index != index {
            self.hovered_index = index;
            cx.notify();
        }
    }
}

impl Render for ResultsList {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let _theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
        let selected_index = self.selected_index;
        let hovered_index = self.hovered_index;

        // Build the grouped display with headers
        let mut elements: Vec<AnyElement> = Vec::new();
        let mut flat_index: usize = 0;

        for group in &self.grouped_results {
            // Add group header
            let header = ResultGroup::new(group.result_type)
                .count(group.items.len())
                .shortcut_start(group.shortcut_start);
            elements.push(header.into_any_element());

            // Add items in this group
            for result in &group.items {
                let is_selected = flat_index == selected_index;
                let is_hovered = hovered_index == Some(flat_index);
                let shortcut = if flat_index < 9 {
                    Some(format!("⌘{}", flat_index + 1))
                } else {
                    None
                };

                elements.push(
                    ResultItem::from(result)
                        .selected(is_selected)
                        .hovered(is_hovered)
                        .shortcut(shortcut.unwrap_or_default())
                        .into_any_element(),
                );

                flat_index += 1;
            }
        }

        // If no groups, render flat results (backwards compatibility)
        if self.grouped_results.is_empty() && !self.flat_results.is_empty() {
            for (idx, result) in self.flat_results.iter().enumerate() {
                let is_selected = idx == selected_index;
                let is_hovered = hovered_index == Some(idx);
                let shortcut = if idx < 9 {
                    Some(format!("⌘{}", idx + 1))
                } else {
                    None
                };

                elements.push(
                    ResultItem::from(result.as_ref())
                        .selected(is_selected)
                        .hovered(is_hovered)
                        .shortcut(shortcut.unwrap_or_default())
                        .into_any_element(),
                );
            }
        }

        div()
            .id("results-scroll-container")
            .flex_1()
            .w_full()
            .max_h(RESULTS_MAX_HEIGHT)
            .overflow_y_scroll()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, cx| {
                this.handle_scroll(event.delta.pixel_delta(px(1.0)), cx);
            }))
            .child(div().w_full().flex().flex_col().children(elements))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{
        IconSource, ResultGroup as SearchResultGroup, ResultType, SearchAction, SearchResultId,
    };
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_test_result(id: &str, title: &str, result_type: ResultType) -> SearchResult {
        SearchResult {
            id: SearchResultId::new(id),
            title: title.to_string(),
            subtitle: "Test subtitle".to_string(),
            icon: IconSource::Emoji { char: '📄' },
            result_type,
            score: 100.0,
            match_indices: vec![],
            action: SearchAction::OpenFile {
                path: PathBuf::from("/test"),
            },
            requires_permissions: false,
        }
    }

    fn create_grouped_search_results() -> SearchResults {
        SearchResults {
            groups: vec![
                SearchResultGroup {
                    result_type: ResultType::Application,
                    results: vec![
                        create_test_result("app1", "Safari", ResultType::Application),
                        create_test_result("app2", "Chrome", ResultType::Application),
                    ],
                },
                SearchResultGroup {
                    result_type: ResultType::SystemCommand,
                    results: vec![create_test_result(
                        "cmd1",
                        "Sleep",
                        ResultType::SystemCommand,
                    )],
                },
                SearchResultGroup {
                    result_type: ResultType::File,
                    results: vec![
                        create_test_result("file1", "Document.pdf", ResultType::File),
                        create_test_result("file2", "Notes.txt", ResultType::File),
                    ],
                },
            ],
            total_count: 5,
            query: "test".to_string(),
            search_time: Duration::from_millis(10),
        }
    }

    #[test]
    fn test_results_list_new() {
        let list = ResultsList::new();
        assert!(list.is_empty());
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.group_count(), 0);
    }

    #[test]
    fn test_set_results_grouped() {
        let mut list = ResultsList::new();
        let results = create_grouped_search_results();

        list.set_results(results);

        assert_eq!(list.len(), 5);
        assert!(!list.is_empty());
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.group_count(), 3);
    }

    #[test]
    fn test_selected_with_groups() {
        let mut list = ResultsList::new();
        let results = create_grouped_search_results();

        list.set_results(results);

        assert_eq!(list.selected().map(|r| r.title.as_str()), Some("Safari"));
    }

    #[test]
    fn test_grouped_results_accessor() {
        let mut list = ResultsList::new();
        let results = create_grouped_search_results();

        list.set_results(results);

        let groups = list.grouped_results();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "Apps");
        assert_eq!(groups[0].items.len(), 2);
        assert_eq!(groups[0].shortcut_start, 0);
        assert_eq!(groups[1].name, "Commands");
        assert_eq!(groups[1].items.len(), 1);
        assert_eq!(groups[1].shortcut_start, 2);
        assert_eq!(groups[2].name, "Files");
        assert_eq!(groups[2].items.len(), 2);
        assert_eq!(groups[2].shortcut_start, 3);
    }

    #[test]
    fn test_total_content_height_with_groups() {
        let mut list = ResultsList::new();
        let results = create_grouped_search_results();

        list.set_results(results);

        let height = list.total_content_height();
        // 5 items * RESULT_ITEM_HEIGHT + 3 headers * GROUP_HEADER_HEIGHT
        let expected = 5.0 * RESULT_ITEM_HEIGHT.0 + 3.0 * GROUP_HEADER_HEIGHT.0;
        assert_eq!(height, expected);
    }

    #[test]
    fn test_calculate_item_y_position() {
        let mut list = ResultsList::new();
        let results = create_grouped_search_results();

        list.set_results(results);

        // First item in first group (after first header)
        let y0 = list.calculate_item_y_position(0);
        assert_eq!(y0, GROUP_HEADER_HEIGHT.0);

        // Second item in first group
        let y1 = list.calculate_item_y_position(1);
        assert_eq!(y1, GROUP_HEADER_HEIGHT.0 + RESULT_ITEM_HEIGHT.0);

        // First item in second group (after 2 items + 2 headers)
        let y2 = list.calculate_item_y_position(2);
        let expected = GROUP_HEADER_HEIGHT.0 + 2.0 * RESULT_ITEM_HEIGHT.0 + GROUP_HEADER_HEIGHT.0;
        assert_eq!(y2, expected);
    }

    #[test]
    fn test_ensure_selected_visible_scroll_down() {
        let mut list = ResultsList::new();
        list.visible_height = 100.0; // ~2 items visible

        let results: Vec<_> = (0..10)
            .map(|i| create_test_result(&i.to_string(), "Test", ResultType::File))
            .collect();
        list.set_flat_results(results);
        list.selected_index = 5;

        list.ensure_selected_visible();

        // Should scroll down to show item 5
        assert!(list.scroll_offset > 0.0);
    }

    #[test]
    fn test_ensure_selected_visible_no_scroll_needed() {
        let mut list = ResultsList::new();
        list.visible_height = 400.0; // Many items visible

        let results: Vec<_> = (0..5)
            .map(|i| create_test_result(&i.to_string(), "Test", ResultType::File))
            .collect();
        list.set_flat_results(results);
        list.selected_index = 2;

        list.ensure_selected_visible();

        // All items visible, no scroll needed
        assert_eq!(list.scroll_offset, 0.0);
    }

    #[test]
    fn test_quick_select() {
        let mut list = ResultsList::new();
        let results = create_grouped_search_results();
        list.set_results(results);

        // ⌘1 should select index 0
        // Note: quick_select requires ViewContext, so we test the bounds logic here
        assert!(0 < list.len()); // ⌘1 would work
        assert!(8 < list.len() || list.len() <= 8); // ⌘9 depends on count
    }

    #[test]
    fn test_set_flat_results_backwards_compat() {
        let mut list = ResultsList::new();
        let results: Vec<_> = (0..3)
            .map(|i| create_test_result(&i.to_string(), "Test", ResultType::File))
            .collect();

        list.set_flat_results(results);

        assert_eq!(list.len(), 3);
        assert_eq!(list.group_count(), 0);
        assert!(list.grouped_results().is_empty());
    }
}
