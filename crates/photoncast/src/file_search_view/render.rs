//! GPUI render methods for the file search view.
//!
//! Contains:
//! - `Render` impl for `FileSearchView`
//! - All `render_*` helper methods
//! - Preview rendering helpers

use gpui::prelude::FluentBuilder;
use gpui::*;

use photoncast_core::platform::spotlight::FileKind;

use crate::constants::{
    DETAIL_PANEL_WIDTH, ICON_SIZE_LG, ICON_SIZE_SM, LIST_ITEM_HEIGHT, LIST_PANEL_WIDTH,
    SEARCH_BAR_HEIGHT, SECTION_HEADER_HEIGHT, TEXT_SIZE_MD,
};

use super::filter::FileTypeFilter;
use super::helpers::{
    extension_to_emoji, file_kind_to_emoji, format_file_size, format_relative_date,
    get_file_search_colors, kind_description, FileSearchColors,
};
use super::{FileSearchView, SectionMode};

// ============================================================================
// Render Helpers on FileSearchView
// ============================================================================

impl FileSearchView {
    /// Render the search bar component (0.4)
    pub(super) fn render_search_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_file_search_colors(cx);
        let filter_name = self.filter.display_name();
        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let _text_placeholder = colors.text_placeholder;
        let accent = colors.accent;

        // Block cursor dimensions (like main modal / Ghostty terminal)
        let cursor_width = px(9.0);
        let cursor_height = px(20.0);

        div()
            .h(SEARCH_BAR_HEIGHT)
            .flex_shrink_0() // Don't shrink search bar
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            // No background - matches main launcher style
            // Search input area with cursor
            .child(
                div()
                    .flex_1()
                    .text_size(TEXT_SIZE_MD)
                    .flex()
                    .items_center()
                    .when(self.query.is_empty(), |el| {
                        el.child(
                            div()
                                .w(cursor_width)
                                .h(cursor_height)
                                .bg(accent)
                                .rounded(px(2.0)),
                        )
                    })
                    .when(!self.query.is_empty(), |el| {
                        // Show text before cursor
                        let chars: Vec<char> = self.query.chars().collect();
                        let before: String = chars[..self.cursor_position].iter().collect();
                        let after: String = chars[self.cursor_position..].iter().collect();

                        el.text_color(text_color)
                            .when(!before.is_empty(), |el| el.child(before))
                            .child(
                                div()
                                    .w(cursor_width)
                                    .h(cursor_height)
                                    .bg(accent)
                                    .rounded(px(2.0)),
                            )
                            .when(!after.is_empty(), |el| el.child(after))
                    }),
            )
            // Filter dropdown button
            .child(
                div()
                    .id("file-type-filter")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(colors.surface_elevated)
                    .border_1()
                    .border_color(colors.border)
                    .cursor_pointer()
                    .hover(move |el| el.bg(surface_hover))
                    .on_click(cx.listener(|this, _, cx| this.toggle_dropdown(cx)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(text_color)
                            .child(filter_name),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(text_muted)
                            .child("▾"),
                    ),
            )
            // Keyboard hint
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(text_muted)
                    .child("esc to close"),
            )
    }

    /// Render a single file list item (0.2)
    pub(super) fn render_file_item(
        &self,
        file: &photoncast_core::platform::spotlight::FileResult,
        index: usize,
        is_selected: bool,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_file_search_colors(cx);

        // Get icon emoji (fallback)
        let icon_emoji = if file.kind == FileKind::File {
            extension_to_emoji(&file.path)
        } else {
            file_kind_to_emoji(file.kind)
        };

        // Get subtitle (parent folder path, shortened with ~)
        let subtitle = file
            .path
            .parent()
            .map(|p| {
                let path_str = p.display().to_string();
                // Replace home directory with ~
                if let Some(home) = dirs::home_dir() {
                    if let Some(stripped) = path_str.strip_prefix(&home.display().to_string()) {
                        return format!("~{}", stripped);
                    }
                }
                path_str
            })
            .unwrap_or_default();

        // Get relative date
        let date_str = file.modified.map(format_relative_date).unwrap_or_default();

        let selection_bg = colors.selection;
        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let file_name = file.name.clone();

        div()
            .id(SharedString::from(format!("file-item-{}", index)))
            .h(LIST_ITEM_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .rounded_md()
            .cursor_pointer()
            // Selection background
            .when(is_selected, |el| el.bg(selection_bg))
            .when(!is_selected, |el| el.hover(move |el| el.bg(surface_hover)))
            // Icon
            .child(
                div()
                    .w(ICON_SIZE_LG)
                    .h(ICON_SIZE_LG)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .child(div().text_size(px(20.0)).child(icon_emoji)),
            )
            // Title and subtitle
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    // Title
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(text_color)
                            .truncate()
                            .child(file_name),
                    )
                    // Subtitle (path)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .truncate()
                            .child(subtitle),
                    ),
            )
            // Accessory (date)
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(text_muted)
                    .child(date_str),
            )
    }

    /// Render the detail panel for the selected file (0.3)
    pub(super) fn render_detail_panel(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_file_search_colors(cx);
        let selected_file = self.selected_file().cloned();

        div()
            .h_full()
            .w(DETAIL_PANEL_WIDTH)
            .flex()
            .flex_col()
            .bg(colors.surface_elevated)
            .border_l_1()
            .border_color(colors.border)
            // Preview area (60%)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_h(px(150.0))
                    .child(self.render_preview(&selected_file, &colors)),
            )
            // Divider
            .child(div().w_full().h(px(1.0)).bg(colors.border))
            // Metadata section (40%)
            .child(self.render_metadata(&selected_file, &colors))
    }

    /// Check if a file is a previewable image
    fn is_previewable_image(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "ico"
                )
            })
    }

    /// Check if a file is a previewable text file
    fn is_previewable_text(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_lowercase().as_str(),
                    "txt"
                        | "md"
                        | "markdown"
                        | "rs"
                        | "js"
                        | "ts"
                        | "py"
                        | "rb"
                        | "go"
                        | "java"
                        | "c"
                        | "cpp"
                        | "h"
                        | "hpp"
                        | "swift"
                        | "kt"
                        | "json"
                        | "yaml"
                        | "yml"
                        | "toml"
                        | "xml"
                        | "html"
                        | "css"
                        | "sh"
                        | "bash"
                        | "zsh"
                        | "fish"
                        | "log"
                        | "csv"
                )
            })
    }

    /// Read first N lines of a text file for preview
    fn read_text_preview(path: &std::path::Path, max_lines: usize) -> Option<String> {
        use std::io::{BufRead, BufReader};
        let file = std::fs::File::open(path).ok()?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .take(max_lines)
            .filter_map(|l| l.ok())
            .collect();
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    /// Render the preview area
    fn render_preview(
        &self,
        file: &Option<photoncast_core::platform::spotlight::FileResult>,
        colors: &FileSearchColors,
    ) -> impl IntoElement {
        let Some(file) = file else {
            // No file selected - show placeholder
            return div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(48.0))
                        .text_color(colors.text_muted)
                        .child("📁"),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(colors.text_placeholder)
                        .child("Select a file to preview"),
                )
                .into_any_element();
        };

        // Check if this is a previewable image - try to render actual preview
        if Self::is_previewable_image(&file.path) && file.path.exists() {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .p_2()
                .child(
                    img(file.path.clone())
                        .max_w(px(280.0))
                        .max_h(px(180.0))
                        .object_fit(ObjectFit::Contain)
                        .rounded(px(4.0)),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(colors.text_placeholder)
                        .child("Press ⌘Y for Quick Look"),
                )
                .into_any_element();
        }

        // Check if this is a previewable text file - show first few lines
        if Self::is_previewable_text(&file.path) && file.path.exists() {
            if let Some(preview_text) = Self::read_text_preview(&file.path, 12) {
                return div()
                    .flex()
                    .flex_col()
                    .p_3()
                    .gap_2()
                    .overflow_hidden()
                    .child(
                        div()
                            .w_full()
                            .h_full()
                            .bg(colors.surface)
                            .rounded(px(4.0))
                            .p_2()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_family("SF Mono, Monaco, Menlo, monospace")
                                    .text_color(colors.text_muted)
                                    .overflow_hidden()
                                    .child(preview_text),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_placeholder)
                            .child("Press ⌘Y for Quick Look"),
                    )
                    .into_any_element();
            }
        }

        // Show file icon as preview for non-previewable files (PDFs, docs, etc.)
        let icon_emoji = if file.kind == FileKind::File {
            extension_to_emoji(&file.path)
        } else {
            file_kind_to_emoji(file.kind)
        };

        // For PDFs and documents, show a hint
        let hint = if file
            .path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_lowercase().as_str(),
                    "pdf"
                        | "doc"
                        | "docx"
                        | "xls"
                        | "xlsx"
                        | "ppt"
                        | "pptx"
                        | "pages"
                        | "numbers"
                        | "key"
                )
            }) {
            "Document - Press ⌘Y to preview"
        } else {
            "Press ⌘Y for Quick Look"
        };

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(div().text_size(px(64.0)).child(icon_emoji))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(colors.text_placeholder)
                    .child(hint),
            )
            .into_any_element()
    }

    /// Render metadata fields for the selected file
    fn render_metadata(
        &self,
        file: &Option<photoncast_core::platform::spotlight::FileResult>,
        colors: &FileSearchColors,
    ) -> impl IntoElement {
        let Some(file) = file else {
            return div().p_4().into_any_element();
        };

        let text_color = colors.text;
        let text_muted = colors.text_muted;

        // Build metadata fields
        let mut fields: Vec<(&'static str, String)> = vec![
            ("Name", file.name.clone()),
            ("Kind", kind_description(file.kind, &file.path)),
        ];

        // Size (not for folders)
        if file.kind != FileKind::Folder {
            if let Some(size) = file.size {
                fields.push(("Size", format_file_size(size)));
            }
        }

        // Modified date
        if let Some(modified) = file.modified {
            fields.push(("Modified", format_relative_date(modified)));
        }

        // Where (parent folder)
        if let Some(parent) = file.path.parent() {
            let where_str = {
                let path_str = parent.display().to_string();
                if let Some(home) = dirs::home_dir() {
                    if let Some(stripped) = path_str.strip_prefix(&home.display().to_string()) {
                        format!("~{}", stripped)
                    } else {
                        path_str
                    }
                } else {
                    path_str
                }
            };
            fields.push(("Where", where_str));
        }

        div()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .children(fields.into_iter().map(move |(label, value)| {
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .w(px(70.0))
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child(label),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.0))
                            .text_color(text_color)
                            .truncate()
                            .child(value),
                    )
            }))
            .into_any_element()
    }

    /// Render section header (0.5)
    pub(super) fn render_section_header(&self, colors: &FileSearchColors) -> impl IntoElement {
        let title = match self.section_mode {
            SectionMode::Recent => "Recent Files".to_string(),
            SectionMode::Search => "Search Results".to_string(),
            SectionMode::Browsing => {
                // Show the current browsing path
                if let Some(path) = &self.browse_path {
                    let display = if let Some(home) = dirs::home_dir() {
                        if path.starts_with(&home) {
                            if path == &home {
                                "~/".to_string()
                            } else if let Ok(stripped) = path.strip_prefix(&home) {
                                format!("~/{}", stripped.display())
                            } else {
                                path.display().to_string()
                            }
                        } else {
                            path.display().to_string()
                        }
                    } else {
                        path.display().to_string()
                    };
                    format!("📁 {}", display)
                } else {
                    "Browsing".to_string()
                }
            },
        };

        div()
            .h(SECTION_HEADER_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_muted)
                    .child(title),
            )
    }

    /// Render empty state (0.5)
    pub(super) fn render_empty_state(&self, colors: &FileSearchColors) -> impl IntoElement {
        let (icon, title, description) = if self.query.is_empty() {
            (
                "🔍",
                "Search for files",
                "Type a file name to start searching",
            )
        } else {
            (
                "📁",
                "No files found",
                "Try a different search term or check your search scope in preferences.",
            )
        };

        div()
            .w_full()
            .py_8()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            // Icon
            .child(div().text_size(px(48.0)).child(icon))
            // Title
            .child(
                div()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.text)
                    .child(title),
            )
            // Description
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(colors.text_muted)
                    .max_w(px(300.0))
                    .child(description),
            )
    }

    /// Render the footer with action hints
    pub(super) fn render_footer(&self, colors: &FileSearchColors) -> impl IntoElement {
        let text_muted = colors.text_muted;
        let border_color = colors.border;
        let surface = colors.surface;
        let is_browsing = self.section_mode == SectionMode::Browsing;

        div()
            .h(px(36.0))
            .flex_shrink_0()
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(border_color)
            .bg(surface)
            // Left side: primary actions
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    // Browsing mode: Tab to enter folder
                    .when(is_browsing, |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .px_1()
                                        .py(px(1.0))
                                        .bg(colors.surface_elevated)
                                        .rounded(px(3.0))
                                        .text_size(px(10.0))
                                        .text_color(text_muted)
                                        .child("⇥"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child("Enter"),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .px_1()
                                        .py(px(1.0))
                                        .bg(colors.surface_elevated)
                                        .rounded(px(3.0))
                                        .text_size(px(10.0))
                                        .text_color(text_muted)
                                        .child("⇧⇥"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child("Back"),
                                ),
                        )
                    })
                    // Open action
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .px_1()
                                    .py(px(1.0))
                                    .bg(colors.surface_elevated)
                                    .rounded(px(3.0))
                                    .text_size(px(10.0))
                                    .text_color(text_muted)
                                    .child("↵"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_muted)
                                    .child("Open"),
                            ),
                    )
                    // Quick Look action
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .px_1()
                                    .py(px(1.0))
                                    .bg(colors.surface_elevated)
                                    .rounded(px(3.0))
                                    .text_size(px(10.0))
                                    .text_color(text_muted)
                                    .child("⌘Y"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_muted)
                                    .child("Quick Look"),
                            ),
                    )
                    // Reveal in Finder
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .px_1()
                                    .py(px(1.0))
                                    .bg(colors.surface_elevated)
                                    .rounded(px(3.0))
                                    .text_size(px(10.0))
                                    .text_color(text_muted)
                                    .child("⌘⏎"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_muted)
                                    .child("Reveal"),
                            ),
                    ),
            )
            // Right side: actions menu hint
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .px_1()
                            .py(px(1.0))
                            .bg(colors.surface_elevated)
                            .rounded(px(3.0))
                            .text_size(px(10.0))
                            .text_color(text_muted)
                            .child("⌘K"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child("Actions"),
                    ),
            )
    }

    /// Render the filter dropdown (0.4)
    pub(super) fn render_filter_dropdown(
        &self,
        colors: &FileSearchColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let options = FileTypeFilter::all_options();
        let current_filter = self.filter;
        let dropdown_index = self.dropdown_index;
        let text_color = colors.text;
        let accent = colors.accent;
        let surface_hover = colors.surface_hover;
        let selection_bg = colors.selection;

        div()
            .absolute()
            .top(SEARCH_BAR_HEIGHT)
            .right(px(16.0))
            .w(px(160.0))
            .bg(colors.surface_elevated)
            .rounded_lg()
            .border_1()
            .border_color(colors.border)
            .shadow_lg()
            .overflow_hidden()
            .py_1()
            .children(options.iter().enumerate().map(|(idx, &filter)| {
                let is_selected = filter == current_filter;
                let is_highlighted = idx == dropdown_index;

                div()
                    .id(SharedString::from(format!("filter-{:?}", filter)))
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    // Keyboard highlight takes precedence
                    .when(is_highlighted, |el| el.bg(selection_bg))
                    .when(!is_highlighted, |el| el.hover(move |el| el.bg(surface_hover)))
                    .on_click(cx.listener(move |this, _, cx| this.set_filter(filter, cx)))
                    // Checkmark for selected
                    .child(
                        div()
                            .w(ICON_SIZE_SM)
                            .text_size(px(12.0))
                            .when(is_selected, |el| el.text_color(accent).child("✓"))
                            .when(!is_selected, |el| el.child("")),
                    )
                    // Filter name
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(text_color)
                            .child(filter.display_name()),
                    )
            }))
    }

    /// Render the file actions menu (Cmd+K)
    pub(super) fn render_actions_menu(
        &self,
        colors: &FileSearchColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let surface_hover = colors.surface_hover;
        let selection_bg = colors.selection;
        let actions_index = self.actions_menu_index;

        div()
            .absolute()
            .bottom(px(44.0)) // Above footer
            .right(px(16.0))
            .w(px(200.0))
            .bg(colors.surface_elevated)
            .rounded_lg()
            .border_1()
            .border_color(colors.border)
            .shadow_lg()
            .overflow_hidden()
            .py_1()
            .children(Self::FILE_ACTIONS.iter().enumerate().map(|(idx, &(name, shortcut, action_id))| {
                let is_highlighted = idx == actions_index;
                let action_id_owned = action_id.to_string();

                div()
                    .id(SharedString::from(format!("action-{}", action_id)))
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .when(is_highlighted, |el| el.bg(selection_bg))
                    .when(!is_highlighted, |el| el.hover(move |el| el.bg(surface_hover)))
                    .on_click(cx.listener(move |this, _, cx| {
                        this.execute_action(&action_id_owned, cx);
                    }))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(text_color)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child(shortcut),
                    )
            }))
    }
}

// ============================================================================
// Render Impl
// ============================================================================

impl Render for FileSearchView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_file_search_colors(cx);
        let has_results = !self.results.is_empty();

        // Pre-render dropdown (needs cx for click handlers)
        let dropdown = if self.dropdown_open {
            Some(self.render_filter_dropdown(&colors, cx).into_any_element())
        } else {
            None
        };

        // Pre-render actions menu (needs cx for click handlers)
        let actions_menu = if self.actions_menu_open && self.selected_file().is_some() {
            Some(self.render_actions_menu(&colors, cx).into_any_element())
        } else {
            None
        };

        // Pre-render footer
        let footer = self.render_footer(&colors);

        div()
            .track_focus(&self.focus_handle)
            .size_full() // Fill parent container
            .flex()
            .flex_col()
            .bg(colors.background)
            .relative()
            // Search bar
            .child(self.render_search_bar(cx))
            // Main content area (split view)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    // Left panel: List (fixed width ~60%)
                    .child({
                        let empty_state = self.render_empty_state(&colors);
                        let section_header = self.render_section_header(&colors);
                        let file_items: Vec<_> = if has_results {
                            self.results
                                .iter()
                                .enumerate()
                                .map(|(idx, file)| {
                                    self.render_file_item(file, idx, idx == self.selected_index, cx)
                                        .into_any_element()
                                })
                                .collect()
                        } else {
                            vec![]
                        };

                        div()
                            .id("file-search-list")
                            .w(LIST_PANEL_WIDTH)
                            .h_full()
                            .flex()
                            .flex_col()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .border_r_1()
                            .border_color(colors.border)
                            // Section header
                            .child(section_header)
                            // Loading indicator
                            .when(self.loading, |el| {
                                el.child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .py_8()
                                        .child(
                                            div()
                                                .text_size(px(14.0))
                                                .text_color(colors.text_muted)
                                                .child("Loading...")
                                        )
                                )
                            })
                            // Results list or empty state
                            .when(!has_results && !self.loading, |el| {
                                el.child(empty_state)
                            })
                            .when(has_results && !self.loading, |el| el.children(file_items))
                    })
                    // Right panel: Detail (remaining space)
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .child(self.render_detail_panel(cx))
                    ),
            )
            // Footer with action hints
            .child(footer)
            // Dropdown overlay (when open)
            .when_some(dropdown, |el, dropdown| {
                el.child(dropdown)
            })
            // Actions menu overlay (when open)
            .when_some(actions_menu, |el, menu| {
                el.child(menu)
            })
        // Note: keyboard events are forwarded from launcher, not handled here directly
    }
}
