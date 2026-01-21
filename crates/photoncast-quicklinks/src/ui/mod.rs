//! UI components for Quick Links.
//!
//! This module provides GPUI components for displaying and managing quick links.

mod argument_view;
mod create_view;
mod manage_view;

pub use argument_view::{ArgumentInputEvent, ArgumentInputView};
pub use create_view::{AppInfo, CreateQuicklinkEvent, CreateQuicklinkFocus, CreateQuicklinkView};
pub use manage_view::{ManageViewEvent, QuicklinksManageView};

use gpui::prelude::*;
use gpui::{div, px, Hsla, IntoElement, ParentElement, Styled, ViewContext};
use photoncast_theme::PhotonTheme;

use crate::models::QuickLink;

/// Theme-aware colors for Quick Links UI.
#[derive(Clone)]
struct QuickLinksColors {
    background: Hsla,
    item_bg: Hsla,
    item_hover: Hsla,
    title_text: Hsla,
    url_text: Hsla,
    tag_bg: Hsla,
    tag_text: Hsla,
    keyword_text: Hsla,
    dynamic_badge: Hsla,
    badge_text: Hsla,
}

impl QuickLinksColors {
    fn from_theme(theme: &PhotonTheme) -> Self {
        Self {
            background: theme.colors.background.to_gpui(),
            item_bg: theme.colors.surface.to_gpui(),
            item_hover: theme.colors.surface_hover.to_gpui(),
            title_text: theme.colors.text.to_gpui(),
            url_text: theme.colors.text_muted.to_gpui(),
            tag_bg: theme.colors.selection.to_gpui(),
            tag_text: theme.colors.accent.to_gpui(),
            keyword_text: theme.colors.success.to_gpui(),
            dynamic_badge: theme.colors.warning.to_gpui(),
            badge_text: theme.colors.background.to_gpui(),
        }
    }
}

fn get_quicklinks_colors<V: 'static>(cx: &ViewContext<V>) -> QuickLinksColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    QuickLinksColors::from_theme(&theme)
}

/// Quick links list view.
type QuickLinkSelectCallback = Box<dyn Fn(&QuickLink, &mut ViewContext<QuickLinksView>) + 'static>;

pub struct QuickLinksView {
    links: Vec<QuickLink>,
    selected_index: Option<usize>,
    on_select: Option<QuickLinkSelectCallback>,
}

impl QuickLinksView {
    /// Creates a new quick links view.
    #[must_use]
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        Self {
            links: Vec::new(),
            selected_index: None,
            on_select: None,
        }
    }

    /// Sets the links to display.
    pub fn set_links(&mut self, links: Vec<QuickLink>, cx: &mut ViewContext<Self>) {
        self.links = links;
        self.selected_index = if self.links.is_empty() { None } else { Some(0) };
        cx.notify();
    }

    /// Sets the selection callback.
    pub fn on_select<F: Fn(&QuickLink, &mut ViewContext<Self>) + 'static>(&mut self, callback: F) {
        self.on_select = Some(Box::new(callback));
    }

    /// Moves selection up.
    pub fn select_previous(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(index) = self.selected_index {
            if index > 0 {
                self.selected_index = Some(index - 1);
                cx.notify();
            }
        }
    }

    /// Moves selection down.
    pub fn select_next(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(index) = self.selected_index {
            if index + 1 < self.links.len() {
                self.selected_index = Some(index + 1);
                cx.notify();
            }
        }
    }

    /// Confirms the current selection.
    pub fn confirm_selection(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(index) = self.selected_index {
            if let Some(link) = self.links.get(index) {
                if let Some(callback) = &self.on_select {
                    let link_clone = link.clone();
                    callback(&link_clone, cx);
                }
            }
        }
    }

    /// Renders a single link item.
    fn render_link_item(link: &QuickLink, is_selected: bool, colors: &QuickLinksColors) -> impl IntoElement {
        let bg = if is_selected {
            colors.item_hover
        } else {
            colors.item_bg
        };
        let title_text = colors.title_text;
        let url_text = colors.url_text;
        let dynamic_badge = colors.dynamic_badge;
        let badge_text = colors.badge_text;
        let tag_bg = colors.tag_bg;
        let tag_text = colors.tag_text;

        let title = link.name.clone();
        let url = link.link.clone();
        let is_dynamic = link.is_dynamic();
        let tags = link.tags.clone();

        div()
            .px(px(12.0))
            .py(px(8.0))
            .rounded_md()
            .bg(bg)
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                // Title row with dynamic badge
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_base()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(title_text)
                            .child(title),
                    )
                    .when(is_dynamic, |this| {
                        this.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(dynamic_badge)
                                .text_xs()
                                .text_color(badge_text)
                                .child("Dynamic"),
                        )
                    }),
            )
            .child(
                // URL row
                div()
                    .text_sm()
                    .text_color(url_text)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(url),
            )
            .when(!tags.is_empty(), |this| {
                this.child(
                    // Tags row
                    div()
                        .flex()
                        .gap(px(4.0))
                        .children(tags.into_iter().map(move |tag| {
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(tag_bg)
                                .text_xs()
                                .text_color(tag_text)
                                .child(tag)
                        })),
                )
            })
    }
}

impl Render for QuickLinksView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_quicklinks_colors(cx);
        if self.links.is_empty() {
            return div()
                .p(px(16.0))
                .rounded_lg()
                .bg(colors.background)
                .flex()
                .items_center()
                .justify_center()
                .text_color(colors.url_text)
                .child("No quick links found");
        }

        let selected_index = self.selected_index;
        let items: Vec<_> = self
            .links
            .iter()
            .enumerate()
            .map(|(i, link)| Self::render_link_item(link, selected_index == Some(i), &colors))
            .collect();

        div()
            .p(px(8.0))
            .rounded_lg()
            .bg(colors.background)
            .flex()
            .flex_col()
            .gap(px(4.0))
            .children(items)
    }
}

/// Quick link item view for result list.
pub struct QuickLinkItem {
    link: QuickLink,
}

impl QuickLinkItem {
    /// Creates a new quick link item.
    #[must_use]
    pub fn new(link: QuickLink, _cx: &mut ViewContext<Self>) -> Self {
        Self { link }
    }
}

impl Render for QuickLinkItem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_quicklinks_colors(cx);
        let is_dynamic = self.link.is_dynamic();

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .w_full()
            .child(
                // Icon placeholder (globe for URL)
                div()
                    .w(px(32.0))
                    .h(px(32.0))
                    .rounded_md()
                    .bg(colors.item_bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_lg()
                    .child("\u{1F310}"), // Globe emoji
            )
            .child(
                // Link info
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(colors.title_text)
                                    .text_ellipsis()
                                    .child(self.link.name.clone()),
                            )
                            .when(is_dynamic, |this| {
                                this.child(
                                    div()
                                        .px(px(4.0))
                                        .py(px(1.0))
                                        .rounded(px(3.0))
                                        .bg(colors.dynamic_badge)
                                        .text_xs()
                                        .text_color(colors.badge_text)
                                        .child("{...}"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.url_text)
                            .text_ellipsis()
                            .child(self.link.link.clone()),
                    ),
            )
    }
}

/// Dynamic URL input prompt view.
pub struct DynamicUrlPrompt {
    link: QuickLink,
    query: String,
}

impl DynamicUrlPrompt {
    /// Creates a new dynamic URL prompt.
    #[must_use]
    pub fn new(link: QuickLink, _cx: &mut ViewContext<Self>) -> Self {
        Self {
            link,
            query: String::new(),
        }
    }

    /// Sets the query.
    pub fn set_query(&mut self, query: String, cx: &mut ViewContext<Self>) {
        self.query = query;
        cx.notify();
    }

    /// Gets the final URL with query substituted.
    #[must_use]
    pub fn get_final_url(&self) -> String {
        self.link.substitute_query(&self.query)
    }
}

impl Render for DynamicUrlPrompt {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_quicklinks_colors(cx);
        let preview_url = self.get_final_url();

        div()
            .p(px(16.0))
            .rounded_lg()
            .bg(colors.background)
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().text_lg().child("\u{1F50D}")) // Magnifying glass
                    .child(
                        div()
                            .text_base()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(colors.title_text)
                            .child(self.link.name.clone()),
                    ),
            )
            .child(
                // Preview URL
                div()
                    .text_sm()
                    .text_color(colors.url_text)
                    .child(format!("URL: {}", preview_url)),
            )
            .child(
                // Hint
                div()
                    .text_xs()
                    .text_color(colors.keyword_text)
                    .child("Type your search query and press Enter"),
            )
    }
}

/// Quick links management view for creating/editing links.
pub struct QuickLinksManagementView {
    links: Vec<QuickLink>,
    selected_index: Option<usize>,
}

impl QuickLinksManagementView {
    /// Creates a new management view.
    #[must_use]
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        Self {
            links: Vec::new(),
            selected_index: None,
        }
    }

    /// Sets the links to manage.
    pub fn set_links(&mut self, links: Vec<QuickLink>, cx: &mut ViewContext<Self>) {
        self.links = links;
        self.selected_index = if self.links.is_empty() { None } else { Some(0) };
        cx.notify();
    }
}

impl Render for QuickLinksManagementView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_quicklinks_colors(cx);
        div()
            .p(px(16.0))
            .rounded_lg()
            .bg(colors.background)
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                // Header
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(colors.title_text)
                    .child("Manage Quick Links"),
            )
            .child(
                // Link count
                div()
                    .text_sm()
                    .text_color(colors.url_text)
                    .child(format!("{} links", self.links.len())),
            )
            .child(
                // Actions hint
                div()
                    .text_xs()
                    .text_color(colors.keyword_text)
                    .child("Use Preferences to add, edit, or remove quick links"),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_link_item_creation() {
        let link = QuickLink::new("Test", "https://example.com");
        assert!(!link.is_dynamic());
    }

    #[test]
    fn test_dynamic_url_substitution() {
        let link = QuickLink::new("Search", "https://example.com/search?q={query}");
        let prompt = DynamicUrlPrompt {
            link,
            query: "test".to_string(),
        };
        assert_eq!(prompt.get_final_url(), "https://example.com/search?q=test");
    }
}
