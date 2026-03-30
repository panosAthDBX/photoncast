//! Actions methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Handles the Show Actions Menu action (Cmd+K).
    pub(super) fn show_actions_menu(&mut self, _: &ShowActionsMenu, cx: &mut ViewContext<Self>) {
        tracing::info!(
            "show_actions_menu called, search_mode={:?}",
            std::mem::discriminant(&self.search.mode)
        );

        // If file search view is active, trigger its actions menu
        if let Some(file_search_view) = &self.file_search.view {
            let has_selection = file_search_view.read(cx).selected_file().is_some();
            if has_selection {
                file_search_view.update(cx, |view, cx| {
                    view.wants_actions_menu = true;
                    cx.notify();
                });
            }
            return;
        }

        // Check if there's something to show actions for
        let has_items = if let SearchMode::Calendar { events, .. } = &self.search.mode {
            tracing::info!("Calendar mode with {} events", events.len());
            !events.is_empty()
        } else {
            tracing::info!(
                "Non-calendar mode with {} results",
                self.search.results.len()
            );
            !self.search.results.is_empty()
        };

        if !has_items {
            tracing::info!("No items, not showing actions menu");
            return;
        }

        // Toggle actions menu
        tracing::info!(
            "Toggling actions menu: {} -> {}",
            self.actions_menu.visible,
            !self.actions_menu.visible
        );
        self.actions_menu.visible = !self.actions_menu.visible;
        self.actions_menu.selected_index = 0; // Reset selection when opening
        cx.notify();
    }

    /// Returns the number of actions available in the current context.
    pub(super) fn get_actions_count(&self) -> usize {
        // Calendar mode has its own actions
        if let SearchMode::Calendar { events, .. } = &self.search.mode {
            if events.is_empty() || self.search.selected_index >= events.len() {
                return 0;
            }
            let event = &events[self.search.selected_index];
            // Actions: Join Meeting (if has conference), Copy Title, Copy Details, Open in Calendar
            let mut count = 3; // Copy Title, Copy Details, Open in Calendar
            if event.conference_url.is_some() {
                count += 1; // Join Meeting
            }
            return count;
        }

        let is_file_mode = matches!(self.search.mode, SearchMode::FileSearch);
        let has_selection = !self.search.results.is_empty();

        if !has_selection {
            return 0;
        }

        // Task 7.3: Check if selected result is an app
        let selected_result = self.search.results.get(self.search.selected_index);
        let is_app = selected_result.is_some_and(|r| r.result_type == ResultType::Application);
        let app_bundle_id = selected_result.and_then(|r| r.bundle_id.clone());
        let is_running = app_bundle_id
            .as_ref()
            .is_some_and(|id| photoncast_apps::is_app_running(id));

        if is_app {
            // App actions:
            // Primary: Open (0), Show in Finder (1)
            // Info: Copy Path (2), Copy Bundle ID (3)
            // Auto Quit: Toggle (4)
            // Running only: Quit (5), Force Quit (6), Hide (7)
            // Danger: Uninstall (5 or 8)
            let mut count = 5; // Open, Show in Finder, Copy Path, Copy Bundle ID, Toggle Auto Quit
            if is_running {
                count += 3; // Quit, Force Quit, Hide
            }
            count += 1; // Uninstall
            return count;
        }

        // Base actions for non-apps: Open, Copy Path, Copy File
        let mut count = 3;

        // File mode adds: Reveal in Finder, Quick Look
        if is_file_mode {
            count += 2;
        }

        count
    }

    /// Executes the action at the current `actions_menu_index`.
    pub(super) fn execute_selected_action(&mut self, cx: &mut ViewContext<Self>) {
        // Handle calendar mode actions
        if let SearchMode::Calendar { events, .. } = &self.search.mode {
            if self.search.selected_index < events.len() {
                let event = events[self.search.selected_index].clone();
                let has_conference = event.conference_url.is_some();

                // Action order: Join Meeting (if available), Copy Title, Copy Details, Open in Calendar
                let action_idx = self.actions_menu.selected_index;
                let adjusted_idx = if has_conference {
                    action_idx
                } else {
                    action_idx + 1
                };

                match adjusted_idx {
                    0 => {
                        // Join Meeting
                        if let Some(url) = &event.conference_url {
                            tracing::info!("Joining meeting: {}", event.title);
                            if let Err(e) = std::process::Command::new("open").arg(url).spawn() {
                                tracing::error!("Failed to open conference URL: {}", e);
                            }
                            self.hide(cx);
                        }
                    },
                    1 => {
                        // Copy Title
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(event.title.clone()));
                        tracing::info!("Copied event title to clipboard");
                        // Exit calendar mode first so hide() actually dismisses the window
                        self.exit_calendar_mode(cx);
                        self.hide(cx);
                    },
                    2 => {
                        // Copy Details
                        let time_str = if event.is_all_day {
                            format!("{} (All day)", event.start.format("%A, %B %d, %Y"))
                        } else {
                            format!(
                                "{} - {}",
                                event.start.format("%A, %B %d, %Y %H:%M"),
                                event.end.format("%H:%M")
                            )
                        };
                        let mut details = format!(
                            "{}\n{}\nCalendar: {}",
                            event.title, time_str, event.calendar_name
                        );
                        if let Some(loc) = &event.location {
                            if !loc.is_empty() {
                                details.push_str(&format!("\nLocation: {}", loc));
                            }
                        }
                        if let Some(url) = &event.conference_url {
                            details.push_str(&format!("\nMeeting: {}", url));
                        }
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(details));
                        tracing::info!("Copied event details to clipboard");
                        // Exit calendar mode first so hide() actually dismisses the window
                        self.exit_calendar_mode(cx);
                        self.hide(cx);
                    },
                    3 => {
                        // Open in Calendar
                        // Use the event ID to open in Calendar app
                        let calendar_url = format!("ical://ekevent/{}", event.id);
                        if let Err(e) = std::process::Command::new("open")
                            .arg(&calendar_url)
                            .spawn()
                        {
                            // Fallback: just open Calendar app
                            tracing::warn!(
                                "Failed to open event directly: {}, opening Calendar app",
                                e
                            );
                            let _ = std::process::Command::new("open")
                                .arg("-a")
                                .arg("Calendar")
                                .spawn();
                        }
                        // Exit calendar mode first so hide() actually dismisses the window
                        self.exit_calendar_mode(cx);
                        self.hide(cx);
                    },
                    _ => {},
                }
            }
            self.actions_menu.visible = false;
            cx.notify();
            return;
        }

        let is_file_mode = matches!(self.search.mode, SearchMode::FileSearch);
        let has_selection = !self.search.results.is_empty();

        if !has_selection {
            self.actions_menu.visible = false;
            cx.notify();
            return;
        }

        // Task 7.3: Check if selected result is an app
        let selected_result = self.search.results.get(self.search.selected_index);
        let is_app = selected_result.is_some_and(|r| r.result_type == ResultType::Application);
        let app_bundle_id = selected_result.and_then(|r| r.bundle_id.clone());
        let app_path = selected_result.and_then(|r| r.app_path.clone());
        let is_running = app_bundle_id
            .as_ref()
            .is_some_and(|id| photoncast_apps::is_app_running(id));

        // Task 7.3: Handle app-specific actions
        if is_app {
            // App actions order:
            // 0: Open
            // 1: Show in Finder
            // 2: Copy Path
            // 3: Copy Bundle ID
            // 4: Toggle Auto Quit
            // 5+ (if running): Quit, Force Quit, Hide
            // Last: Uninstall
            let uninstall_idx = if is_running { 8 } else { 5 };

            match self.actions_menu.selected_index {
                0 => {
                    // Open
                    self.actions_menu.visible = false;
                    self.activate(&Activate, cx);
                },
                1 => {
                    // Show in Finder
                    self.actions_menu.visible = false;
                    if let Some(path) = &app_path {
                        if let Err(e) = photoncast_apps::reveal_in_finder(path) {
                            tracing::error!("Failed to reveal in Finder: {}", e);
                        }
                    }
                    self.hide(cx);
                },
                2 => {
                    // Copy Path
                    self.actions_menu.visible = false;
                    if let Some(path) = &app_path {
                        if let Err(e) = photoncast_apps::copy_path_to_clipboard(path) {
                            tracing::error!("Failed to copy path: {}", e);
                        } else {
                            tracing::info!("Copied path to clipboard");
                        }
                    }
                    self.hide(cx);
                },
                3 => {
                    // Copy Bundle ID
                    self.actions_menu.visible = false;
                    if let Some(bundle_id) = &app_bundle_id {
                        if let Err(e) = photoncast_apps::copy_bundle_id_to_clipboard(bundle_id) {
                            tracing::error!("Failed to copy bundle ID: {}", e);
                        } else {
                            tracing::info!("Copied bundle ID to clipboard");
                        }
                    }
                    self.hide(cx);
                },
                4 => {
                    // Toggle Auto Quit - show settings modal to configure
                    self.actions_menu.visible = false;
                    if let Some(bundle_id) = &app_bundle_id {
                        let is_enabled = {
                            self.auto_quit
                                .manager
                                .read()
                                .is_auto_quit_enabled(bundle_id)
                        };
                        if is_enabled {
                            // Disable directly
                            {
                                let mut manager = self.auto_quit.manager.write();
                                manager.disable_auto_quit(bundle_id);
                                let _ = manager.save();
                            }
                            tracing::info!("Disabled auto-quit for {}", bundle_id);
                            self.show_toast("Auto Quit disabled".to_string(), cx);
                        } else {
                            // Show settings modal to configure timeout
                            let app_name =
                                selected_result.map(|r| r.title.clone()).unwrap_or_default();
                            self.show_auto_quit_settings(bundle_id, &app_name, cx);
                        }
                    }
                    cx.notify();
                },
                5 if is_running => {
                    // Quit
                    self.actions_menu.visible = false;
                    if let Some(bundle_id) = &app_bundle_id {
                        match photoncast_apps::quit_app_by_bundle_id(bundle_id) {
                            Ok(_) => tracing::info!("Quit app: {}", bundle_id),
                            Err(e) => tracing::error!("Failed to quit app: {}", e),
                        }
                    }
                    self.hide(cx);
                },
                6 if is_running => {
                    // Force Quit
                    self.actions_menu.visible = false;
                    if let Some(bundle_id) = &app_bundle_id {
                        // Get the PID for the bundle ID
                        if let Ok(running_apps) =
                            photoncast_apps::AppManager::new(photoncast_apps::AppsConfig::default())
                                .get_running_apps()
                        {
                            if let Some(app) = running_apps
                                .iter()
                                .find(|a| a.bundle_id.as_deref() == Some(bundle_id))
                            {
                                #[allow(clippy::cast_possible_wrap)]
                                let pid = app.pid as i32;
                                match photoncast_apps::force_quit_app_action(pid) {
                                    Ok(()) => tracing::info!(
                                        "Force quit app: {} (PID {})",
                                        bundle_id,
                                        pid
                                    ),
                                    Err(e) => tracing::error!("Failed to force quit app: {}", e),
                                }
                            }
                        }
                    }
                    self.hide(cx);
                },
                7 if is_running => {
                    // Hide app
                    self.actions_menu.visible = false;
                    if let Some(bundle_id) = &app_bundle_id {
                        if let Err(e) = photoncast_apps::hide_app(bundle_id) {
                            tracing::error!("Failed to hide app: {}", e);
                        } else {
                            tracing::info!("Hid app: {}", bundle_id);
                        }
                    }
                    self.hide(cx);
                },
                idx if idx == uninstall_idx => {
                    // Uninstall - show uninstall preview dialog
                    self.actions_menu.visible = false;
                    if let Some(path) = &app_path {
                        let app_name = selected_result.map(|r| r.title.clone()).unwrap_or_default();
                        tracing::info!("Starting uninstall flow for: {} at {:?}", app_name, path);
                        self.show_uninstall_preview(std::path::Path::new(path), cx);
                    }
                    cx.notify();
                },
                _ => {
                    self.actions_menu.visible = false;
                    cx.notify();
                },
            }
            return;
        }

        // Map index to action based on current mode (non-app)
        // Order: Open, Copy Path, Copy File, [Reveal in Finder, Quick Look]
        match self.actions_menu.selected_index {
            0 => {
                // Open
                self.actions_menu.visible = false;
                self.activate(&Activate, cx);
            },
            1 => {
                // Copy Path
                self.copy_path(&CopyPath, cx);
            },
            2 => {
                // Copy File
                self.copy_file(&CopyFile, cx);
            },
            3 if is_file_mode => {
                // Reveal in Finder
                self.reveal_in_finder(&RevealInFinder, cx);
            },
            4 if is_file_mode => {
                // Quick Look
                self.quick_look(&QuickLook, cx);
            },
            _ => {
                self.actions_menu.visible = false;
                cx.notify();
            },
        }
    }

    /// Handles the Reveal in Finder action (Cmd+Enter).
    pub(super) fn reveal_in_finder(&mut self, _: &RevealInFinder, cx: &mut ViewContext<Self>) {
        // If file search view is active, reveal selected file
        if let Some(file_search_view) = &self.file_search.view {
            let selected_path = file_search_view
                .read(cx)
                .selected_file()
                .map(|f| f.path.clone());
            if let Some(path) = selected_path {
                tracing::info!("Reveal in Finder (file search): {}", path.display());
                let _ = photoncast_apps::reveal_in_finder(&path);
                self.hide(cx);
            }
            return;
        }

        // Only active in file search mode with a selected file result
        if !matches!(self.search.mode, SearchMode::FileSearch) {
            return;
        }

        if let Some(core_result) = self
            .search
            .core_results
            .get(self.search.selected_index)
            .cloned()
        {
            match &core_result.action {
                SearchAction::OpenFile { path } | SearchAction::RevealInFinder { path } => {
                    // Open Finder and reveal the file
                    let reveal_action = SearchAction::RevealInFinder { path: path.clone() };
                    let launcher = Arc::clone(&self.app_launcher);
                    match launcher.execute_action(&reveal_action) {
                        Ok(()) => {
                            tracing::info!("Revealed in Finder: {}", path.display());
                        },
                        Err(e) => {
                            tracing::error!(
                                "Failed to reveal {}: {}",
                                path.display(),
                                e.user_message()
                            );
                        },
                    }
                    self.hide(cx);
                },
                _ => {
                    tracing::debug!("Reveal in Finder: not a file result");
                },
            }
        }
    }

    /// Handles the Quick Look action (Cmd+Y).
    pub(super) fn quick_look(&mut self, _: &QuickLook, cx: &mut ViewContext<Self>) {
        // If file search view is active, trigger Quick Look for selected file
        if let Some(file_search_view) = &self.file_search.view {
            let selected_path = file_search_view
                .read(cx)
                .selected_file()
                .map(|f| f.path.clone());
            if let Some(path) = selected_path {
                tracing::info!("Quick Look (file search): {}", path.display());
                let _ = std::process::Command::new("qlmanage")
                    .arg("-p")
                    .arg(&path)
                    .spawn();
            }
            return;
        }

        // Only active in file search mode with a selected file result
        if !matches!(self.search.mode, SearchMode::FileSearch) {
            return;
        }

        if let Some(core_result) = self
            .search
            .core_results
            .get(self.search.selected_index)
            .cloned()
        {
            match &core_result.action {
                SearchAction::OpenFile { path } | SearchAction::RevealInFinder { path } => {
                    // Trigger Quick Look using qlmanage
                    tracing::info!("Quick Look: {}", path.display());
                    if let Err(e) = std::process::Command::new("qlmanage")
                        .args(["-p", &path.to_string_lossy()])
                        .spawn()
                    {
                        tracing::error!("Failed to open Quick Look: {}", e);
                    }
                    // Don't hide the window - Quick Look is a preview
                    cx.notify();
                },
                _ => {
                    tracing::debug!("Quick Look: not a file result");
                },
            }
        }
    }

    /// Handles the Copy Path action (Cmd+C).
    pub(super) fn copy_path(&mut self, _: &CopyPath, cx: &mut ViewContext<Self>) {
        if let Some(core_result) = self
            .search
            .core_results
            .get(self.search.selected_index)
            .cloned()
        {
            let path_str = match &core_result.action {
                SearchAction::OpenFile { path } | SearchAction::RevealInFinder { path } => {
                    Some(path.display().to_string())
                },
                SearchAction::LaunchApp { path, .. } => Some(path.display().to_string()),
                _ => None,
            };

            if let Some(path) = path_str {
                // Copy to clipboard using pbcopy
                if let Ok(mut child) = std::process::Command::new("pbcopy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                {
                    use std::io::Write;
                    if let Some(stdin) = child.stdin.as_mut() {
                        let _ = stdin.write_all(path.as_bytes());
                    }
                    let _ = child.wait();
                    tracing::info!("Copied to clipboard: {}", path);
                }
            }
        }
        // Close menu if open
        self.actions_menu.visible = false;
        self.hide(cx);
    }

    /// Handles the Copy File action (Cmd+Shift+C).
    /// Copies the actual file to clipboard so it can be pasted in apps like Slack, `WhatsApp`, etc.
    pub(super) fn copy_file(&mut self, _: &CopyFile, cx: &mut ViewContext<Self>) {
        if let Some(core_result) = self
            .search
            .core_results
            .get(self.search.selected_index)
            .cloned()
        {
            let path = match &core_result.action {
                SearchAction::OpenFile { path } | SearchAction::RevealInFinder { path } => {
                    Some(path.clone())
                },
                SearchAction::LaunchApp { path, .. } => Some(path.clone()),
                _ => None,
            };

            if let Some(path) = path {
                // Use osascript to copy file to clipboard (works for pasting in apps)
                // SECURITY: Escape backslashes and double quotes to prevent AppleScript injection
                let escaped_path = escape_path_for_applescript(&path.display().to_string());
                let script = format!(r#"set the clipboard to (POSIX file "{escaped_path}")"#);

                match std::process::Command::new("osascript")
                    .args(["-e", &script])
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        tracing::info!("Copied file to clipboard: {}", path.display());
                    },
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::error!("Failed to copy file: {}", stderr);
                    },
                    Err(e) => {
                        tracing::error!("Failed to run osascript: {}", e);
                    },
                }
            }
        }
        // Close menu if open
        self.actions_menu.visible = false;
        self.hide(cx);
    }

    /// Handler for Show in Finder action (⌘⇧F)
    pub(super) fn show_in_finder(&mut self, _: &ShowInFinder, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.search.results.get(self.search.selected_index) {
            if let Some(path) = &result.app_path {
                if let Err(e) = photoncast_apps::reveal_in_finder(path) {
                    tracing::error!("Failed to reveal in Finder: {}", e);
                } else {
                    self.hide(cx);
                    return;
                }
            }
        }
        cx.notify();
    }

    /// Handler for Copy Bundle ID action (⌘⇧B)
    pub(super) fn copy_bundle_id(&mut self, _: &CopyBundleId, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.search.results.get(self.search.selected_index) {
            if let Some(bundle_id) = &result.bundle_id {
                if let Err(e) = photoncast_apps::copy_bundle_id_to_clipboard(bundle_id) {
                    tracing::error!("Failed to copy bundle ID: {}", e);
                } else {
                    tracing::info!("Copied bundle ID to clipboard: {}", bundle_id);
                }
            }
        }
        self.hide(cx);
    }

    /// Handler for Quit App action (⌘Q)
    pub(super) fn quit_app(&mut self, _: &QuitApp, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.search.results.get(self.search.selected_index) {
            if result.result_type == ResultType::Application {
                if let Some(bundle_id) = &result.bundle_id {
                    if photoncast_apps::is_app_running(bundle_id) {
                        match photoncast_apps::quit_app_by_bundle_id(bundle_id) {
                            Ok(_) => tracing::info!("Quit app: {}", bundle_id),
                            Err(e) => tracing::error!("Failed to quit app: {}", e),
                        }
                    }
                }
            }
        }
        self.hide(cx);
    }

    /// Handler for Force Quit App action (⌘⌥Q)
    pub(super) fn force_quit_app(&mut self, _: &ForceQuitApp, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.search.results.get(self.search.selected_index) {
            if result.result_type == ResultType::Application {
                if let Some(bundle_id) = &result.bundle_id {
                    if photoncast_apps::is_app_running(bundle_id) {
                        if let Ok(running_apps) =
                            photoncast_apps::AppManager::new(photoncast_apps::AppsConfig::default())
                                .get_running_apps()
                        {
                            if let Some(app) = running_apps
                                .iter()
                                .find(|a| a.bundle_id.as_deref() == Some(bundle_id))
                            {
                                #[allow(clippy::cast_possible_wrap)]
                                let pid = app.pid as i32;
                                match photoncast_apps::force_quit_app_action(pid) {
                                    Ok(()) => tracing::info!(
                                        "Force quit app: {} (PID {})",
                                        bundle_id,
                                        pid
                                    ),
                                    Err(e) => tracing::error!("Failed to force quit app: {}", e),
                                }
                            }
                        }
                    }
                }
            }
        }
        self.hide(cx);
    }

    /// Handler for Hide App action (⌘H)
    pub(super) fn hide_app(&mut self, _: &HideApp, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.search.results.get(self.search.selected_index) {
            if result.result_type == ResultType::Application {
                if let Some(bundle_id) = &result.bundle_id {
                    if photoncast_apps::is_app_running(bundle_id) {
                        if let Err(e) = photoncast_apps::hide_app(bundle_id) {
                            tracing::error!("Failed to hide app: {}", e);
                        } else {
                            tracing::info!("Hid app: {}", bundle_id);
                        }
                    }
                }
            }
        }
        self.hide(cx);
    }

    pub(super) fn open_preferences(&mut self, _: &OpenPreferences, cx: &mut ViewContext<Self>) {
        if let Err(e) = app_events::send_event(AppEvent::OpenPreferences) {
            tracing::error!("Failed to send preferences event: {}", e);
        }
        self.hide(cx);
    }
}
