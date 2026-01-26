//! Preview pane for ListView split-view.
//!
//! Renders preview content for selected list items:
//! - Markdown content with basic formatting
//! - Images from various sources
//! - Metadata key-value pairs

use gpui::prelude::FluentBuilder;
use gpui::*;
use photoncast_extension_api::Preview;

use super::colors::ExtensionViewColors;
use super::dimensions::*;

/// Preview pane component for displaying item previews.
pub struct ExtensionPreviewPane {
    /// The preview content to display.
    preview: Preview,
    /// Theme colors.
    colors: ExtensionViewColors,
}

impl ExtensionPreviewPane {
    /// Creates a new preview pane.
    pub fn new(preview: Preview, colors: ExtensionViewColors) -> Self {
        Self { preview, colors }
    }

    /// Renders markdown content with basic formatting.
    fn render_markdown(&self, markdown: &str) -> impl IntoElement {
        let mut elements: Vec<gpui::AnyElement> = Vec::new();
        let lines = markdown.lines();

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                // Empty line - add spacing
                elements.push(div().h(px(8.0)).into_any_element());
            } else if trimmed.starts_with("# ") {
                // H1 header
                elements.push(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(self.colors.text)
                        .mb(px(8.0))
                        .child(trimmed[2..].to_string())
                        .into_any_element(),
                );
            } else if trimmed.starts_with("## ") {
                // H2 header
                elements.push(
                    div()
                        .text_base()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(self.colors.text)
                        .mb(px(6.0))
                        .child(trimmed[3..].to_string())
                        .into_any_element(),
                );
            } else if trimmed.starts_with("### ") {
                // H3 header
                elements.push(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(self.colors.text)
                        .mb(px(4.0))
                        .child(trimmed[4..].to_string())
                        .into_any_element(),
                );
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                // Unordered list item
                elements.push(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(8.0))
                        .mb(px(4.0))
                        .child(
                            div()
                                .text_color(self.colors.text_muted)
                                .child("•"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(self.colors.text)
                                .child(trimmed[2..].to_string()),
                        )
                        .into_any_element(),
                );
            } else if trimmed.starts_with("```") {
                // Code block start/end - skip for now
                // TODO: Handle multi-line code blocks
            } else if trimmed.starts_with("`") && trimmed.ends_with("`") && trimmed.len() > 2 {
                // Inline code
                let code = &trimmed[1..trimmed.len() - 1];
                elements.push(
                    div()
                        .px(px(4.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(self.colors.surface)
                        .text_xs()
                        .font_family("monospace")
                        .text_color(self.colors.text)
                        .child(code.to_string())
                        .into_any_element(),
                );
            } else if trimmed.starts_with("> ") {
                // Block quote
                elements.push(
                    div()
                        .pl(px(12.0))
                        .border_l_2()
                        .border_color(self.colors.accent)
                        .text_sm()
                        .italic()
                        .text_color(self.colors.text_muted)
                        .mb(px(8.0))
                        .child(trimmed[2..].to_string())
                        .into_any_element(),
                );
            } else {
                // Regular paragraph
                elements.push(
                    div()
                        .text_sm()
                        .text_color(self.colors.text)
                        .mb(px(8.0))
                        .child(Self::render_inline_formatting(line, &self.colors))
                        .into_any_element(),
                );
            }
        }

        div().flex().flex_col().children(elements)
    }

    /// Renders inline markdown formatting (bold, italic, code, links).
    fn render_inline_formatting(text: &str, colors: &ExtensionViewColors) -> String {
        // For simplicity, we'll strip markdown for now
        // A full implementation would parse and render inline elements
        let mut result = text.to_string();

        // Remove bold markers
        result = result.replace("**", "").replace("__", "");
        // Remove italic markers
        result = result.replace('*', "").replace('_', "");
        // Remove inline code markers
        result = result.replace('`', "");

        result
    }

    /// Renders an image preview.
    fn render_image(&self, source: &str, alt: &str) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .max_w_full()
                    .overflow_hidden()
                    .rounded(px(8.0))
                    .child(
                        img(SharedString::from(source.to_string()))
                            .max_w_full()
                    ),
            )
            .when(!alt.is_empty(), |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(self.colors.text_muted)
                        .child(alt.to_string()),
                )
            })
    }

    /// Renders metadata as key-value pairs.
    fn render_metadata(
        &self,
        items: &[(String, String)],
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(items.iter().map(|(key, value)| {
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.0))
                    .child(
                        div()
                            .w(px(100.0))
                            .flex_shrink_0()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(self.colors.text_muted)
                            .child(key.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(self.colors.text)
                            .child(Self::render_metadata_value(value, &self.colors)),
                    )
                    .into_any_element()
            }))
    }

    /// Renders a metadata value, detecting URLs for linking.
    fn render_metadata_value(value: &str, colors: &ExtensionViewColors) -> String {
        // For simplicity, return as-is
        // A full implementation would detect URLs and make them clickable
        value.to_string()
    }
}

impl IntoElement for ExtensionPreviewPane {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        div()
            .id("preview-content")
            .w_full()
            .h_full()
            .p(PADDING)
            .overflow_y_scroll()
            .bg(self.colors.surface)
            .child(match &self.preview {
                Preview::Markdown(markdown) => {
                    self.render_markdown(markdown.as_str()).into_any_element()
                },
                Preview::Image { source, alt } => {
                    self.render_image(source.as_str(), alt.as_str())
                        .into_any_element()
                },
                Preview::Metadata { items } => {
                    let converted: Vec<(String, String)> = items
                        .iter()
                        .map(|t| (t.0.to_string(), t.1.to_string()))
                        .collect();
                    self.render_metadata(&converted).into_any_element()
                },
            })
    }
}
