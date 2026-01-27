//! Uninstall methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Handler for Uninstall App action (⌘⌫)
    pub(super) fn uninstall_app(&mut self, _: &UninstallApp, cx: &mut ViewContext<Self>) {
        // Clone the path to avoid borrow issues
        let app_path = self.search.results.get(self.search.selected_index).and_then(|result| {
            if result.result_type == ResultType::Application {
                result.app_path.clone()
            } else {
                None
            }
        });

        if let Some(path) = app_path {
            // Show the uninstall preview dialog
            self.show_uninstall_preview(&path, cx);
            return;
        }
        cx.notify();
    }

    /// Handler for Toggle Auto Quit action (⌘⇧A)
    pub(super) fn toggle_auto_quit_for_selected(&mut self, _: &ToggleAutoQuit, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.search.results.get(self.search.selected_index).cloned() {
            if result.result_type == ResultType::Application {
                if let Some(bundle_id) = &result.bundle_id {
                    let is_enabled = self
                        .auto_quit.manager
                        .read()
                        .is_auto_quit_enabled(bundle_id);
                    if is_enabled {
                        // If already enabled, disable it directly
                        let mut manager = self.auto_quit.manager.write();
                        manager.disable_auto_quit(bundle_id);
                        let _ = manager.save();
                        tracing::info!("Disabled auto-quit for {}", bundle_id);
                        drop(manager);
                        self.show_toast("Auto Quit disabled".to_string(), cx);
                    } else {
                        // If not enabled, show settings modal to configure timeout
                        self.show_auto_quit_settings(bundle_id, &result.title, cx);
                    }
                }
            }
        }
        cx.notify();
    }

    /// Shows the uninstall preview dialog for an app
    pub(super) fn show_uninstall_preview(
        &mut self,
        app_path: &std::path::Path,
        cx: &mut ViewContext<Self>,
    ) {
        match self.app_manager.create_uninstall_preview(app_path) {
            Ok(preview) => {
                tracing::info!("Created uninstall preview for: {}", preview.app.name);
                self.uninstall.preview = Some(preview);
                self.uninstall.files_selected_index = 0;
                cx.notify();
            },
            Err(e) => {
                tracing::error!("Failed to create uninstall preview: {}", e);
                self.show_toast(format!("Cannot uninstall: {}", e), cx);
            },
        }
    }

    /// Handles the uninstall action (called when "Uninstall" button is clicked)
    pub(super) fn perform_uninstall(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(preview) = self.uninstall.preview.take() {
            let app_name = preview.app.name.clone();
            match self.app_manager.uninstall_selected(&preview) {
                Ok(()) => {
                    tracing::info!("Successfully uninstalled: {}", app_name);
                    self.show_toast(format!("{} uninstalled", app_name), cx);
                },
                Err(e) => {
                    tracing::error!("Failed to uninstall {}: {}", app_name, e);
                    self.show_toast(format!("Uninstall failed: {}", e), cx);
                },
            }
        }
    }

    /// Handles "Keep Related Files" action - uninstalls app only
    pub(super) fn perform_uninstall_app_only(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(mut preview) = self.uninstall.preview.take() {
            let app_name = preview.app.name.clone();
            // Deselect all related files
            for file in &mut preview.related_files {
                file.selected = false;
            }
            match self.app_manager.uninstall_selected(&preview) {
                Ok(()) => {
                    tracing::info!("Successfully uninstalled {} (kept related files)", app_name);
                    self.show_toast(format!("{} uninstalled (kept related files)", app_name), cx);
                },
                Err(e) => {
                    tracing::error!("Failed to uninstall {}: {}", app_name, e);
                    self.show_toast(format!("Uninstall failed: {}", e), cx);
                },
            }
        }
    }

    /// Cancels the uninstall preview dialog
    pub(super) fn cancel_uninstall_preview(&mut self, cx: &mut ViewContext<Self>) {
        self.uninstall.preview = None;
        cx.notify();
    }

    /// Toggles selection of a related file in the uninstall preview
    pub(super) fn toggle_uninstall_file_selection(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        if let Some(preview) = &mut self.uninstall.preview {
            if let Some(file) = preview.related_files.get_mut(index) {
                file.selected = !file.selected;
                // Recalculate total size
                let selected_size = photoncast_apps::calculate_selected_size(preview);
                preview.space_freed_formatted = UninstallPreview::format_bytes(selected_size);
                cx.notify();
            }
        }
    }

    /// Shows the auto quit settings panel for an app
    pub(super) fn show_auto_quit_settings(
        &mut self,
        bundle_id: &str,
        app_name: &str,
        cx: &mut ViewContext<Self>,
    ) {
        self.auto_quit.settings_app = Some((bundle_id.to_string(), app_name.to_string()));
        self.auto_quit.settings_index = 0; // Reset selection to toggle option
        cx.notify();
    }

    /// Closes the auto quit settings panel
    pub(super) fn close_auto_quit_settings(&mut self, cx: &mut ViewContext<Self>) {
        self.auto_quit.settings_app = None;
        cx.notify();
    }

    /// Toggles auto quit for the currently shown app in settings panel
    pub(super) fn toggle_auto_quit_in_settings(&mut self, cx: &mut ViewContext<Self>) {
        if let Some((ref bundle_id, _)) = self.auto_quit.settings_app {
            let mut manager = self.auto_quit.manager.write();
            if manager.is_auto_quit_enabled(bundle_id) {
                manager.disable_auto_quit(bundle_id);
            } else {
                manager.enable_auto_quit(bundle_id, DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES);
            }
            let _ = manager.save();
            cx.notify();
        }
    }

    /// Sets the auto quit timeout for the currently selected app in settings
    pub(super) fn set_auto_quit_timeout(&mut self, minutes: u32, cx: &mut ViewContext<Self>) {
        if let Some((ref bundle_id, _)) = self.auto_quit.settings_app {
            let mut manager = self.auto_quit.manager.write();
            manager.enable_auto_quit(bundle_id, minutes);
            let _ = manager.save();
            cx.notify();
        }
    }

    /// Activates the currently selected option in auto-quit settings
    pub(super) fn activate_auto_quit_settings_option(&mut self, cx: &mut ViewContext<Self>) {
        let timeout_options = [1, 2, 3, 5, 10, 15, 30];
        match self.auto_quit.settings_index {
            0 => {
                // Toggle auto quit
                self.toggle_auto_quit_in_settings(cx);
            },
            idx if (1..=7).contains(&idx) => {
                // Set timeout (index 1 = 1 min, index 2 = 2 min, etc.)
                let minutes = timeout_options[idx - 1];
                self.set_auto_quit_timeout(minutes, cx);
            },
            _ => {},
        }
    }

    /// Enters the "Manage Auto Quits" mode
    #[allow(dead_code)]
    pub(super) fn enter_manage_auto_quits_mode(&mut self, cx: &mut ViewContext<Self>) {
        self.auto_quit.manage_mode = true;
        self.auto_quit.manage_index = 0;
        self.search.query = SharedString::default();
        self.search.cursor_position = 0;
        self.search.selection_anchor = None;
        cx.notify();
    }

    /// Exits the "Manage Auto Quits" mode
    #[allow(dead_code)]
    pub(super) fn exit_manage_auto_quits_mode(&mut self, cx: &mut ViewContext<Self>) {
        self.auto_quit.manage_mode = false;
        self.auto_quit.manage_index = 0;
        self.load_suggestions(cx);
        cx.notify();
    }

    /// Disables auto quit for the app at the given index
    pub(super) fn disable_auto_quit_at_index(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        let enabled_apps: Vec<_> = {
            let manager = self.auto_quit.manager.read();
            manager
                .get_enabled_apps()
                .iter()
                .map(|(id, cfg)| (id.to_string(), cfg.timeout_minutes))
                .collect()
        };

        if let Some((bundle_id, _)) = enabled_apps.get(index) {
            let mut manager = self.auto_quit.manager.write();
            manager.disable_auto_quit(bundle_id);
            let _ = manager.save();
            drop(manager);
            self.show_toast("Auto Quit disabled".to_string(), cx);
            cx.notify();
        }
    }
}
