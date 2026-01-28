use super::*;

impl PreferencesWindow {
    pub(super) fn render_file_search_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let hotkey_display = self.config.file_search.hotkey.display_string();

        div()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            // Indexing Options section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Indexing"),
                    )
                    // Enable Indexing
                    .child(
                        self.render_toggle_row(
                            "file_search_indexing",
                            "Enable File Indexing",
                            "Index files for faster search results",
                            self.config.file_search.indexing_enabled,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_file_search_indexing(cx))),
                    )
                    // Index Hidden Files
                    .child(
                        self.render_toggle_row(
                            "file_search_hidden",
                            "Index Hidden Files",
                            "Include files starting with '.' in search results",
                            self.config.file_search.index_hidden_files,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_file_search_hidden_files(cx))),
                    ),
            )
            // Search Scopes section
            .child(self.render_search_scopes_section(cx))
            // Custom Scopes section
            .child(self.render_custom_scopes_section(cx))
            // Display Options section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Display"),
                    )
                    // Max Results
                    .child(self.render_number_row(
                        "Max Results",
                        "Maximum number of file search results",
                        self.config.file_search.max_results,
                        cx,
                        |this, cx| this.decrement_file_search_max_results(cx),
                        |this, cx| this.increment_file_search_max_results(cx),
                    ))
                    // Show Preview
                    .child(
                        self.render_toggle_row(
                            "file_search_preview",
                            "Show Preview Panel",
                            "Display file preview and metadata panel",
                            self.config.file_search.show_preview,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_file_search_preview(cx))),
                    )
                    // Remember Filter
                    .child(
                        self.render_toggle_row(
                            "file_search_remember_filter",
                            "Remember Last Filter",
                            "Keep the last used file type filter selected",
                            self.config.file_search.remember_filter,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_file_search_remember_filter(cx))),
                    ),
            )
            // Hotkey section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Keyboard Shortcut"),
                    )
                    // Enable Hotkey
                    .child(
                        self.render_toggle_row(
                            "file_search_hotkey_enabled",
                            "Enable Dedicated Hotkey",
                            "Open File Search with a dedicated keyboard shortcut",
                            self.config.file_search.hotkey.enabled,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_file_search_hotkey_enabled(cx))),
                    )
                    // Current Hotkey Display
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(colors.text)
                                            .child("Current Shortcut"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(colors.text_muted)
                                            .child("Press this to open File Search directly"),
                                    ),
                            )
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(colors.surface)
                                    .border_1()
                                    .border_color(colors.border)
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.text)
                                            .child(hotkey_display),
                                    ),
                            ),
                    ),
            )
            // Note about reindexing
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(TEXT_SIZE_SM)
                            .text_color(colors.text)
                            .child("Reindexing"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Changes to indexing settings or search scopes will trigger a background reindex. This may take a few minutes depending on the number of files."),
                    ),
            )
    }

    fn render_search_scopes_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let scopes = self.config.file_search.search_scopes.clone();
        let has_scopes = !scopes.is_empty();

        // Render scope items
        let scope_items: Vec<_> = scopes
            .iter()
            .map(|scope| {
                let scope_clone = scope.clone();
                let display_path = scope
                    .to_str()
                    .map(|s| {
                        // Replace home directory with ~
                        if let Some(home) = dirs::home_dir() {
                            if let Some(home_str) = home.to_str() {
                                if let Some(stripped) = s.strip_prefix(home_str) {
                                    return format!("~{}", stripped);
                                }
                            }
                        }
                        s.to_string()
                    })
                    .unwrap_or_else(|| scope.display().to_string());

                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .py(px(6.0))
                    .px(px(8.0))
                    .rounded(px(4.0))
                    .hover(|s| s.bg(colors.surface_hover))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(div().text_size(px(14.0)).child("📁"))
                            .child(
                                div()
                                    .text_size(TEXT_SIZE_SM)
                                    .text_color(colors.text)
                                    .child(display_path),
                            ),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "remove-scope-{}",
                                scope.display()
                            )))
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| {
                                this.remove_file_search_scope(&scope_clone, cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_muted)
                                    .child("×"),
                            ),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(colors.text)
                                    .child("Search Scopes"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_muted)
                                    .child("Directories to include in file search indexing"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            // Reset to Default button
                            .child(
                                div()
                                    .id("reset-scopes")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(colors.surface)
                                    .hover(|s| s.bg(colors.surface_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| this.reset_file_search_scopes_to_default(cx)))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(colors.text)
                                            .child("Reset"),
                                    ),
                            )
                            // Add Scope button (opens folder picker)
                            .child(
                                div()
                                    .id("add-scope")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(colors.accent)
                                    .hover(|s| s.bg(colors.accent))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|_this, _, cx| {
                                        // Use async folder picker to avoid blocking GPUI
                                        cx.spawn(|view, mut cx| async move {
                                            let folder = rfd::AsyncFileDialog::new()
                                                .set_title("Select Folder to Index")
                                                .pick_folder()
                                                .await;
                                            if let Some(handle) = folder {
                                                let path = handle.path().to_path_buf();
                                                let _ = cx.update(|cx| {
                                                    view.update(cx, |this, cx| {
                                                        this.add_file_search_scope(path, cx);
                                                    })
                                                });
                                            }
                                        }).detach();
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(colors.text)
                                            .child("+ Add Folder"),
                                    ),
                            ),
                    ),
            )
            // Scopes list
            .child(
                div()
                    .id("search-scopes-list")
                    .p(px(8.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .max_h(px(200.0))
                    .overflow_y_scroll()
                    .child(if has_scopes {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(scope_items)
                            .into_any_element()
                    } else {
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_placeholder)
                            .child("No search scopes configured. Click \"+ Add Folder\" to add directories to index.")
                            .into_any_element()
                    }),
            )
    }

    fn render_custom_scopes_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let has_scopes = !self.config.file_search.custom_scopes.is_empty();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(colors.text)
                                    .child("Custom Scopes"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_muted)
                                    .child("Additional directories with optional extension filters"),
                            ),
                    )
                    .child(self.render_custom_scope_buttons(has_scopes, cx)),
            )
            // Custom scopes list
            .child(self.render_custom_scope_list(cx))
            // Info about custom scopes
            .child(
                div()
                    .p(px(8.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Tip: Custom scopes are useful for indexing code repositories or notes folders with specific file types. Edit ~/.config/photoncast/config.toml to customize extensions per scope."),
                    ),
            )
    }

    fn render_custom_scope_list(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let custom_scopes = self.config.file_search.custom_scopes.clone();
        let has_scopes = !custom_scopes.is_empty();
        let editing_state = self.editing_scope_extensions.clone();

        // Render custom scope items with controls
        let scope_items: Vec<_> = custom_scopes
            .iter()
            .map(|scope| {
                let path_for_remove = scope.path.clone();
                let path_for_recursive = scope.path.clone();
                let path_for_all = scope.path.clone();
                let path_for_code = scope.path.clone();
                let path_for_edit = scope.path.clone();
                let display_path = scope.path.clone();
                let is_recursive = scope.recursive;
                let is_all_files = scope.extensions.is_empty();
                let extensions_display = if scope.extensions.is_empty() {
                    "All files".to_string()
                } else {
                    scope.extensions.iter()
                        .map(|e| format!(".{}", e))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                // Check if this scope is being edited
                let is_editing = editing_state.as_ref()
                    .map(|(p, _)| p == &scope.path)
                    .unwrap_or(false);
                let edit_text = editing_state.as_ref()
                    .filter(|(p, _)| p == &scope.path)
                    .map(|(_, t)| t.clone())
                    .unwrap_or_default();

                // Build extensions row based on editing state
                let extensions_row = if is_editing {
                    // Editing mode: show text input with save/cancel
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .pl(px(22.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(colors.text_muted)
                                .w(px(50.0))
                                .child("Files:"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .px(px(6.0))
                                .py(px(3.0))
                                .rounded(px(4.0))
                                .bg(colors.background)
                                .border_1()
                                .border_color(colors.accent)
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(if edit_text.is_empty() { colors.text_placeholder } else { colors.text })
                                        .child(if edit_text.is_empty() { "rs, py, js, ...".to_string() } else { edit_text }),
                                ),
                        )
                        .child(
                            div()
                                .id("save-extensions-input")
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(colors.accent)
                                .hover(|s| s.bg(colors.accent.opacity(0.8)))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, cx| {
                                    this.save_scope_extensions_input(cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(colors.text)
                                        .child("✓"),
                                ),
                        )
                        .child(
                            div()
                                .id("cancel-extensions-input")
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(colors.surface)
                                .hover(|s| s.bg(colors.hover))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, cx| {
                                    this.cancel_scope_extensions_edit(cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(colors.text_muted)
                                        .child("×"),
                                ),
                        )
                        .into_any_element()
                } else {
                    // Normal mode: preset buttons + Edit button
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .pl(px(22.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(colors.text_muted)
                                .w(px(50.0))
                                .child("Files:"),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("scope-all-{}", path_for_all)))
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(if is_all_files { colors.accent } else { colors.surface })
                                .hover(|s| s.bg(if is_all_files { colors.accent } else { colors.hover }))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, cx| {
                                    this.set_custom_scope_extensions(&path_for_all, vec![], cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(colors.text)
                                        .child("All"),
                                ),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("scope-code-{}", path_for_code)))
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(if !is_all_files { colors.accent } else { colors.surface })
                                .hover(|s| s.bg(if !is_all_files { colors.accent } else { colors.hover }))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, cx| {
                                    let code_extensions = vec![
                                        "rs".to_string(), "md".to_string(), "toml".to_string(),
                                        "json".to_string(), "yaml".to_string(), "yml".to_string(),
                                        "txt".to_string(), "py".to_string(), "js".to_string(),
                                        "ts".to_string(), "tsx".to_string(), "jsx".to_string(),
                                    ];
                                    this.set_custom_scope_extensions(&path_for_code, code_extensions, cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(colors.text)
                                        .child("Code"),
                                ),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("scope-edit-{}", path_for_edit)))
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(colors.surface)
                                .hover(|s| s.bg(colors.hover))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, cx| {
                                    this.start_editing_scope_extensions(&path_for_edit, cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(colors.text)
                                        .child("Edit"),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(colors.text_placeholder)
                                .ml(px(4.0))
                                .child(extensions_display),
                        )
                        .into_any_element()
                };

                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .py(px(8.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .bg(colors.surface_hover)
                    .mb(px(4.0))
                    // Header row: path + remove button
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(div().text_size(px(14.0)).child("📂"))
                                    .child(
                                        div()
                                            .text_size(TEXT_SIZE_SM)
                                            .text_color(colors.text)
                                            .child(display_path),
                                    ),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!("remove-custom-scope-{}", path_for_remove)))
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(colors.surface)
                                    .hover(|s| s.bg(colors.hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, cx| {
                                        this.remove_custom_scope(&path_for_remove, cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(colors.text_muted)
                                            .child("×"),
                                    ),
                            ),
                    )
                    // Extensions row (editing or preset buttons)
                    .child(extensions_row)
                    // Recursive toggle row
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .pl(px(22.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_muted)
                                    .w(px(50.0))
                                    .child("Scan:"),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!("scope-recursive-{}", path_for_recursive)))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(colors.surface)
                                    .hover(|s| s.bg(colors.hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, cx| {
                                        this.toggle_custom_scope_recursive(&path_for_recursive, cx);
                                    }))
                                    .child(
                                        div()
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .rounded(px(2.0))
                                            .border_1()
                                            .border_color(colors.border)
                                            .bg(if is_recursive { colors.accent } else { colors.surface })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(if is_recursive {
                                                div().text_size(px(8.0)).text_color(colors.text).child("✓")
                                            } else {
                                                div()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(colors.text)
                                            .child("Recursive"),
                                    ),
                            ),
                    )
            })
            .collect();

        div()
            .id("custom-scopes-list")
            .p(px(8.0))
            .rounded(px(6.0))
            .bg(colors.surface)
            .max_h(px(200.0))
            .overflow_y_scroll()
            .child(if has_scopes {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .children(scope_items)
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_placeholder)
                            .child("No custom scopes configured."),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(colors.text_placeholder)
                            .child("Custom scopes let you add directories outside the primary search scopes (Desktop, Documents, Downloads) with optional file type filters."),
                    )
                    .into_any_element()
            })
    }

    fn render_custom_scope_buttons(
        &self,
        has_scopes: bool,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_colors(cx);

        let mut buttons = div().flex().items_center().gap(px(4.0));

        // Clear All button (only show if there are scopes)
        if has_scopes {
            buttons = buttons.child(
                div()
                    .id("clear-custom-scopes")
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(colors.surface)
                    .hover(|s| s.bg(colors.surface_hover))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, cx| this.clear_custom_scopes(cx)))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text)
                            .child("Clear All"),
                    ),
            );
        }

        // Add Custom Scope button
        buttons = buttons.child(
            div()
                .id("add-custom-scope")
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .bg(colors.accent)
                .hover(|s| s.bg(colors.accent))
                .cursor_pointer()
                .on_click(cx.listener(|_this, _, cx| {
                    // Use async folder picker to avoid blocking GPUI
                    cx.spawn(|view, mut cx| async move {
                        let folder = rfd::AsyncFileDialog::new()
                            .set_title("Select Folder for Custom Scope")
                            .pick_folder()
                            .await;
                        if let Some(handle) = folder {
                            let path = handle.path().to_path_buf();
                            // Add with empty extensions (all files)
                            let _ = cx.update(|cx| {
                                view.update(cx, |this, cx| {
                                    this.add_custom_scope(path, vec![], cx);
                                })
                            });
                        }
                    })
                    .detach();
                }))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(colors.text)
                        .child("+ Add Scope"),
                ),
        );

        buttons
    }
}
