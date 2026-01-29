//! Search methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Loads suggestions (frequently used apps).
    pub(super) fn load_suggestions(&mut self, cx: &mut ViewContext<Self>) {
        tracing::debug!(
            "load_suggestions: index_initialized={}",
            self.index_initialized
        );
        // Only load if index is ready
        if !self.index_initialized {
            tracing::debug!("Skipping suggestions - index not initialized");
            return;
        }

        // Try to get frecency-based suggestions (recently/frequently used apps)
        let frecent_bundle_ids = self.app_launcher.get_top_apps_by_frecency(6);
        tracing::debug!(
            "Frecency returned {} apps: {:?}",
            frecent_bundle_ids.len(),
            frecent_bundle_ids
        );

        if !frecent_bundle_ids.is_empty() {
            // Look up each app by bundle ID directly from index
            let app = self.photoncast_app.read();
            self.search.suggestions = frecent_bundle_ids
                .iter()
                .filter_map(|bundle_id| {
                    // Get app directly by bundle ID
                    app.get_app_by_bundle_id(bundle_id).map(|indexed_app| {
                        // Convert IndexedApp to SearchResult
                        SearchResult {
                            id: SearchResultId::new(format!("app:{}", indexed_app.bundle_id)),
                            title: indexed_app.name.clone(),
                            subtitle: indexed_app.path.display().to_string(),
                            icon: IconSource::AppIcon {
                                bundle_id: indexed_app.bundle_id.as_str().to_string(),
                                icon_path: indexed_app.icon_path.clone(),
                            },
                            result_type: CoreResultType::Application,
                            score: 100.0, // High score for frecent apps
                            match_indices: vec![],
                            requires_permissions: false,
                            action: SearchAction::LaunchApp {
                                bundle_id: indexed_app.bundle_id.as_str().to_string(),
                                path: indexed_app.path.clone(),
                            },
                        }
                    })
                })
                .collect();
            tracing::debug!(
                "Loaded {} frecency-based suggestions",
                self.search.suggestions.len()
            );
        } else {
            // Fallback: search for common apps if no usage data yet
            tracing::debug!("No frecency data, falling back to search-based suggestions");
            let outcome = self.photoncast_app.read().search(""); // Empty search returns popular apps
            self.search.suggestions = outcome
                .results
                .groups
                .into_iter()
                .filter(|g| g.result_type == CoreResultType::Application)
                .flat_map(|g| g.results)
                .take(6)
                .collect();
            tracing::debug!(
                "Loaded {} fallback suggestions",
                self.search.suggestions.len()
            );
        }

        // If query is empty, populate results with suggestions so they're navigable
        if self.search.query.is_empty() && !matches!(self.search.mode, SearchMode::Calendar { .. })
        {
            self.search.core_results = self.search.suggestions.clone();
            self.search.results = self
                .search
                .core_results
                .iter()
                .map(Self::search_result_to_result_item)
                .collect();
            tracing::debug!(
                "Populated {} results from suggestions",
                self.search.results.len()
            );
        }

        // Notify to trigger re-render
        cx.notify();
    }

    /// Handle query change from search input
    pub(super) fn on_query_change(&mut self, _query: SharedString, cx: &mut ViewContext<Self>) {
        self.search.selected_index = 0;
        // Deselect meeting when user starts typing
        self.meeting.selected = false;

        // In calendar mode, filter events by query from the full list
        if let SearchMode::Calendar { title, error, .. } = &self.search.mode {
            let query_lower = self.search.query.to_lowercase();
            let filtered: Vec<_> = if query_lower.is_empty() {
                self.meeting.all_events.clone()
            } else {
                self.meeting
                    .all_events
                    .iter()
                    .filter(|e| e.title.to_lowercase().contains(&query_lower))
                    .cloned()
                    .collect()
            };
            self.search.mode = SearchMode::Calendar {
                title: title.clone(),
                events: filtered,
                error: error.clone(),
            };
            self.search.selected_index = 0;
            cx.notify();
            return;
        }

        // Perform search using the core library
        if self.search.query.is_empty() {
            // When query is empty, show suggestions as results so they're navigable
            self.search.base_results.clear();
            self.search.core_results = self.search.suggestions.clone();
            self.search.results = self
                .search
                .core_results
                .iter()
                .map(Self::search_result_to_result_item)
                .collect();
            self.calculator.result = None;
            self.calculator.generation = self.calculator.generation.saturating_add(1);
            // Close actions menu when results are cleared
            self.actions_menu.visible = false;
        } else {
            match self.search.mode {
                SearchMode::Normal => {
                    // Normal mode: search apps and commands using PhotonCastApp
                    let outcome = self.photoncast_app.read().search(&self.search.query);

                    // Collect all results from the search outcome
                    self.search.base_results = outcome.results.iter().cloned().collect();
                    self.calculator.result = None;
                    self.rebuild_results(cx);
                    self.schedule_calculator_evaluation(cx);

                    // Check if this is a "show timer" query - fetch active timer async
                    let query_lower = self.search.query.to_lowercase();
                    if query_lower.contains("show")
                        || query_lower.contains("status")
                        || query_lower.contains("active timer")
                    {
                        self.fetch_active_timer_result(cx);
                    }

                    // Log timeout warning if applicable
                    if outcome.timed_out {
                        if let Some(msg) = outcome.message {
                            tracing::warn!("Search warning: {}", msg);
                        }
                    }
                },
                SearchMode::FileSearch => {
                    // File Search Mode: debounced async Spotlight search
                    self.calculator.result = None;
                    self.calculator.generation = self.calculator.generation.saturating_add(1);
                    self.schedule_file_search(cx);
                },
                SearchMode::Calendar { .. } => {},
            }
        }

        cx.notify();
    }

    /// Schedules a debounced file search.
    /// Uses adaptive debounce based on query length for better responsiveness.
    /// Now uses native SpotlightSearchService for better performance.
    pub(super) fn schedule_file_search(&mut self, cx: &mut ViewContext<Self>) {
        use crate::file_search_helper::{adaptive_debounce_ms, spotlight_search};

        let query = self.search.query.to_string();

        // Require at least 2 characters before searching
        if query.len() < 2 {
            self.search.results.clear();
            self.search.base_results.clear();
            self.search.core_results.clear();
            self.file_search.loading = false;
            self.calculator.result = None;
            return;
        }

        // Increment generation to invalidate previous searches
        self.file_search.generation += 1;
        let generation = self.file_search.generation;

        // Adaptive debounce: shorter for longer queries (more specific = faster)
        let debounce_ms = adaptive_debounce_ms(query.len());

        // Show loading state
        self.file_search.loading = true;
        self.file_search.pending_query = Some(query.clone());

        // Spawn debounced async search
        cx.spawn(|this, mut cx| async move {
            // Adaptive debounce
            cx.background_executor()
                .timer(Duration::from_millis(debounce_ms))
                .await;

            // Check if this search is still valid (no newer keystrokes)
            let should_search = this
                .update(&mut cx, |view, _| view.file_search.generation == generation)
                .unwrap_or(false);

            if !should_search {
                return; // A newer search was scheduled, abort this one
            }

            // Execute the actual search using native SpotlightSearchService
            // (SpotlightSearchService has built-in caching)
            let search_results: Vec<SearchResult> = cx
                .background_executor()
                .spawn(async move {
                    // Use native Spotlight search
                    let file_results = spotlight_search(&query, MAX_VISIBLE_RESULTS);

                    // Convert FileResult to SearchResult for UI
                    file_results
                        .into_iter()
                        .map(|file| {
                            let path = file.path.clone();
                            let name = file.name.clone();
                            let subtitle = path
                                .parent()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            let is_dir = path.is_dir();
                            let is_app = path
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("app"));
                            let result_type = if is_app {
                                CoreResultType::Application
                            } else if is_dir {
                                CoreResultType::Folder
                            } else {
                                CoreResultType::File
                            };

                            SearchResult {
                                id: photoncast_core::search::SearchResultId::new(format!(
                                    "file:{}",
                                    path.display()
                                )),
                                title: name,
                                subtitle,
                                icon: IconSource::FileIcon { path: path.clone() },
                                result_type,
                                score: 0.0,
                                match_indices: vec![],
                                requires_permissions: false,
                                action: SearchAction::OpenFile { path },
                            }
                        })
                        .collect()
                })
                .await;

            // Update UI with results (if this search is still valid)
            let _ = this.update(&mut cx, |view, cx| {
                if view.file_search.generation == generation {
                    view.file_search.loading = false;
                    view.search.base_results = search_results;
                    view.calculator.result = None;
                    view.rebuild_results(cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Fetches active timer and adds it to results if found
    pub(super) fn fetch_active_timer_result(&mut self, cx: &mut ViewContext<Self>) {
        let timer_manager = Arc::clone(&self.timer_manager);

        cx.spawn(|this, mut cx| async move {
            let manager = timer_manager.read().await;
            let timer_result = manager.get_timer().await;
            drop(manager);

            if let Ok(Some(timer)) = timer_result {
                let action_name = match timer.action {
                    photoncast_timer::scheduler::TimerAction::Sleep => "Sleep",
                    photoncast_timer::scheduler::TimerAction::Shutdown => "Shutdown",
                    photoncast_timer::scheduler::TimerAction::Restart => "Restart",
                    photoncast_timer::scheduler::TimerAction::Lock => "Lock",
                };

                let time_str = timer.countdown_string();

                let search_result = SearchResult {
                    id: SearchResultId::new("active_timer"),
                    title: format!("Active Timer: {}", action_name),
                    subtitle: format!("{} - Press Enter to cancel", time_str),
                    icon: IconSource::SystemIcon {
                        name: "clock".to_string(),
                    },
                    result_type: photoncast_core::search::ResultType::SystemCommand,
                    score: 15000.0, // Very high score to show at top
                    match_indices: vec![],
                    requires_permissions: false,
                    action: SearchAction::OpenSleepTimer {
                        expression: "cancel".to_string(),
                    },
                };

                let _ = this.update(&mut cx, |view, cx| {
                    // Insert active timer at the beginning of results
                    view.search.core_results.insert(0, search_result.clone());
                    view.search
                        .results
                        .insert(0, Self::search_result_to_result_item(&search_result));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn rebuild_results(&mut self, _cx: &mut ViewContext<Self>) {
        self.search.core_results.clear();
        self.search.results.clear();

        if let Some(result) = &self.calculator.result {
            self.search
                .core_results
                .push(Self::calculator_result_to_search_result(result));
            self.search
                .results
                .push(Self::calculator_result_to_result_item(result));
        }

        for result in &self.search.base_results {
            if self.search.core_results.len() >= MAX_VISIBLE_RESULTS {
                break;
            }
            self.search.core_results.push(result.clone());
            self.search
                .results
                .push(Self::search_result_to_result_item(result));
        }
    }

    pub(super) fn select_next(&mut self, _: &SelectNext, cx: &mut ViewContext<Self>) {
        // If file search is active, forward navigation to it
        if let Some(file_search_view) = &self.file_search.view {
            file_search_view.update(cx, |view, cx| view.navigate_next(cx));
            return;
        }

        // If auto-quit settings is open, navigate within it
        // Options: 0 = toggle, 1-7 = timeout options (1, 2, 3, 5, 10, 15, 30 minutes)
        if self.auto_quit.settings_app.is_some() {
            let option_count = 8; // toggle + 7 timeout options
            self.auto_quit.settings_index = (self.auto_quit.settings_index + 1) % option_count;
            cx.notify();
            return;
        }

        // If actions menu is open, navigate within it
        if self.actions_menu.visible {
            let action_count = self.get_actions_count();
            if action_count > 0 {
                self.actions_menu.selected_index =
                    (self.actions_menu.selected_index + 1) % action_count;
                cx.notify();
            }
            return;
        }

        // Handle calendar mode navigation (cyclic)
        if let SearchMode::Calendar { events, .. } = &self.search.mode {
            if !events.is_empty() {
                let previous = self.search.selected_index;
                self.search.selected_index = (self.search.selected_index + 1) % events.len();
                if self.search.selected_index != previous {
                    self.start_selection_animation(previous, cx);
                    self.ensure_selected_visible(cx);
                }
                cx.notify();
            }
            return;
        }

        // Normal mode with meeting + results navigation
        let has_meeting = self.search.query.is_empty() && self.meeting.next_meeting.is_some();
        let results_len = self.search.results.len();

        if has_meeting && self.meeting.selected {
            // Move from meeting to first result (or wrap to meeting if no results)
            if !self.search.results.is_empty() {
                self.meeting.selected = false;
                self.search.selected_index = 0;
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        } else if !self.search.results.is_empty() {
            let previous = self.search.selected_index;
            if self.search.selected_index + 1 >= results_len {
                // At last result - wrap to meeting (if present) or first result
                if has_meeting {
                    self.meeting.selected = true;
                    self.search.selected_index = 0;
                } else {
                    self.search.selected_index = 0; // Wrap to first
                }
            } else {
                self.search.selected_index += 1;
            }
            if self.search.selected_index != previous || self.meeting.selected {
                self.start_selection_animation(previous, cx);
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        } else if has_meeting {
            // Only meeting, keep it selected
            self.meeting.selected = true;
            cx.notify();
        }
    }

    pub(super) fn select_previous(&mut self, _: &SelectPrevious, cx: &mut ViewContext<Self>) {
        // If file search is active, forward navigation to it
        if let Some(file_search_view) = &self.file_search.view {
            file_search_view.update(cx, |view, cx| view.navigate_previous(cx));
            return;
        }

        // If auto-quit settings is open, navigate within it
        if self.auto_quit.settings_app.is_some() {
            let option_count = 8; // toggle + 7 timeout options
            self.auto_quit.settings_index = if self.auto_quit.settings_index == 0 {
                option_count - 1
            } else {
                self.auto_quit.settings_index - 1
            };
            cx.notify();
            return;
        }

        // If actions menu is open, navigate within it
        if self.actions_menu.visible {
            let action_count = self.get_actions_count();
            if action_count > 0 {
                self.actions_menu.selected_index = if self.actions_menu.selected_index == 0 {
                    action_count - 1
                } else {
                    self.actions_menu.selected_index - 1
                };
                cx.notify();
            }
            return;
        }

        // Handle calendar mode navigation (cyclic)
        if let SearchMode::Calendar { events, .. } = &self.search.mode {
            if !events.is_empty() {
                let previous = self.search.selected_index;
                self.search.selected_index = if self.search.selected_index == 0 {
                    events.len() - 1
                } else {
                    self.search.selected_index - 1
                };
                if self.search.selected_index != previous {
                    self.start_selection_animation(previous, cx);
                    self.ensure_selected_visible(cx);
                }
                cx.notify();
            }
            return;
        }

        // Normal mode with meeting + results navigation
        let has_meeting = self.search.query.is_empty() && self.meeting.next_meeting.is_some();
        let results_len = self.search.results.len();

        if has_meeting && self.meeting.selected {
            // Move from meeting to last result (or stay if no results)
            if !self.search.results.is_empty() {
                self.meeting.selected = false;
                self.search.selected_index = results_len - 1;
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        } else if !self.search.results.is_empty() {
            let previous = self.search.selected_index;
            if self.search.selected_index == 0 {
                // At first result - wrap to meeting (if present) or last result
                if has_meeting {
                    self.meeting.selected = true;
                } else {
                    self.search.selected_index = results_len - 1; // Wrap to last
                }
            } else {
                self.search.selected_index -= 1;
            }
            if self.search.selected_index != previous || self.meeting.selected {
                self.start_selection_animation(previous, cx);
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        } else if has_meeting {
            // Only meeting, keep it selected
            self.meeting.selected = true;
            cx.notify();
        }
    }

    pub(super) fn activate(&mut self, _: &Activate, cx: &mut ViewContext<Self>) {
        // If file search view is active, handle Enter for actions menu, dropdown, or file open
        if let Some(file_search_view) = &self.file_search.view {
            let selected_path = file_search_view.update(cx, |view, cx| {
                if view.actions_menu_open {
                    // Execute the selected action
                    if let Some(&(_, _, action_id)) =
                        crate::file_search_view::FileSearchView::FILE_ACTIONS
                            .get(view.actions_menu_index)
                    {
                        view.execute_action(action_id, cx);
                    }
                    None
                } else if view.dropdown_open {
                    let options = crate::file_search_view::FileTypeFilter::all_options();
                    if let Some(&filter) = options.get(view.dropdown_index) {
                        view.set_filter(filter, cx);
                    }
                    None
                } else {
                    // Get the selected file path
                    view.selected_file().map(|f| f.path.clone())
                }
            });

            // Open the file with default application
            if let Some(path) = selected_path {
                let _ = std::process::Command::new("open").arg(&path).spawn();
                self.hide(cx);
            }
            return;
        }

        // If uninstall preview is showing, perform the uninstall
        if self.uninstall.preview.is_some() {
            self.perform_uninstall(cx);
            return;
        }

        // If auto-quit settings is open, activate the selected option
        if self.auto_quit.settings_app.is_some() {
            self.activate_auto_quit_settings_option(cx);
            return;
        }

        // If actions menu is open, execute the selected action
        if self.actions_menu.visible {
            self.execute_selected_action(cx);
            return;
        }

        // If confirmation dialog is showing, this means user pressed Enter to confirm
        if self.pending_confirmation.is_some() {
            self.confirm_pending_command(cx);
            return;
        }

        // In Calendar mode, join the selected meeting if it has a conference link
        if let SearchMode::Calendar { events, .. } = &self.search.mode {
            if let Some(event) = events.get(self.search.selected_index) {
                if let Some(url) = &event.conference_url {
                    tracing::info!("Joining meeting: {}", event.title);
                    if let Err(e) = std::process::Command::new("open").arg(url).spawn() {
                        tracing::error!("Failed to open conference URL: {}", e);
                    }
                    self.hide(cx);
                } else {
                    // No conference link, just exit calendar mode
                    self.exit_calendar_mode(cx);
                }
            } else {
                self.exit_calendar_mode(cx);
            }
            return;
        }

        // If meeting is selected and we have a next meeting, join it or open in Calendar
        if self.meeting.selected && self.search.query.is_empty() {
            if let Some(meeting) = &self.meeting.next_meeting {
                if let Some(url) = &meeting.conference_url {
                    tracing::info!("Joining next meeting: {} at {}", meeting.title, url);
                    if let Err(e) = std::process::Command::new("open").arg(url).spawn() {
                        tracing::error!("Failed to open conference URL: {}", e);
                    }
                    self.hide(cx);
                    return;
                } else {
                    // No conference link - open in Calendar app
                    tracing::info!("Opening meeting in Calendar: {}", meeting.title);
                    let calendar_url = format!("ical://ekevent/{}", meeting.id);
                    if let Err(e) = std::process::Command::new("open")
                        .arg(&calendar_url)
                        .spawn()
                    {
                        tracing::warn!(
                            "Failed to open event directly: {}, opening Calendar app",
                            e
                        );
                        let _ = std::process::Command::new("open")
                            .arg("-a")
                            .arg("Calendar")
                            .spawn();
                    }
                    self.hide(cx);
                    return;
                }
            }
        }

        if let Some(core_result) = self
            .search
            .core_results
            .get(self.search.selected_index)
            .cloned()
        {
            let title = core_result.title.clone();

            // Handle the action based on its type
            match &core_result.action {
                SearchAction::LaunchApp { .. }
                | SearchAction::OpenFile { .. }
                | SearchAction::RevealInFinder { .. } => {
                    // Use AppLauncher for app/file actions (tracks usage internally)
                    let launcher = Arc::clone(&self.app_launcher);
                    let action = core_result.action.clone();

                    match launcher.execute_action(&action) {
                        Ok(()) => {
                            tracing::info!("Launched: {}", title);
                        },
                        Err(e) => {
                            tracing::error!("Failed to launch {}: {}", title, e.user_message());
                        },
                    }
                    self.hide(cx);
                },
                SearchAction::ExecuteCommand { command_id } => {
                    // Use CommandExecutor for system commands
                    let executor = Arc::clone(&self.command_executor);

                    if let Some(cmd) = executor.lookup(command_id) {
                        if cmd == SystemCommand::Preferences {
                            if let Err(e) = app_events::send_event(AppEvent::OpenPreferences) {
                                tracing::error!("Failed to send preferences event: {}", e);
                            }
                            self.hide(cx);
                            return;
                        }
                        if cmd == SystemCommand::CreateQuicklink {
                            if let Err(e) = app_events::send_event(AppEvent::CreateQuicklink) {
                                tracing::error!("Failed to send create quicklink event: {}", e);
                            }
                            self.hide(cx);
                            return;
                        }
                        if cmd == SystemCommand::ManageQuicklinks {
                            if let Err(e) = app_events::send_event(AppEvent::ManageQuicklinks) {
                                tracing::error!("Failed to send manage quicklinks event: {}", e);
                            }
                            self.hide(cx);
                            return;
                        }
                        if cmd == SystemCommand::BrowseQuicklinkLibrary {
                            if let Err(e) = app_events::send_event(AppEvent::BrowseQuicklinkLibrary)
                            {
                                tracing::error!(
                                    "Failed to send browse quicklink library event: {}",
                                    e
                                );
                            }
                            self.hide(cx);
                            return;
                        }

                        // Check if command requires confirmation
                        if let Some(dialog) = cmd.confirmation_dialog() {
                            // Show confirmation dialog instead of executing directly
                            tracing::info!(
                                "Command {} requires confirmation, showing dialog",
                                command_id
                            );
                            self.pending_confirmation = Some((cmd, dialog));
                            cx.notify();
                            return;
                        }

                        // Execute non-destructive commands directly
                        match executor.execute(cmd) {
                            Ok(()) => {
                                tracing::info!("Executed command: {}", title);
                            },
                            Err(e) => {
                                tracing::error!("Failed to execute {}: {}", title, e);
                            },
                        }
                        self.hide(cx);
                    } else {
                        tracing::error!("Command not found: {}", command_id);
                    }
                },
                SearchAction::CopyToClipboard { text } => {
                    if let Ok(mut child) = std::process::Command::new("pbcopy")
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                    {
                        use std::io::Write;
                        if let Some(stdin) = child.stdin.as_mut() {
                            let _ = stdin.write_all(text.as_bytes());
                        }
                        let _ = child.wait();
                        tracing::info!("Copied to clipboard: {}", text);
                    }
                    self.hide(cx);
                },
                SearchAction::OpenUrl { url } => {
                    let launcher = Arc::clone(&self.app_launcher);
                    if let Err(e) =
                        launcher.execute_action(&SearchAction::OpenUrl { url: url.clone() })
                    {
                        tracing::error!("Failed to open URL: {}", e.user_message());
                    }
                    self.hide(cx);
                },
                SearchAction::ExecuteQuickLink {
                    id,
                    url_template,
                    arguments,
                } => {
                    // Substitute arguments into URL template
                    let final_url = if !arguments.is_empty() {
                        photoncast_quicklinks::placeholder::substitute_argument(
                            url_template,
                            arguments,
                        )
                    } else {
                        url_template.clone()
                    };

                    // Check if URL still requires user input
                    if photoncast_quicklinks::placeholder::requires_user_input(&final_url) {
                        // Send event to open argument input UI
                        if let Err(e) = app_events::send_event(AppEvent::ExecuteQuickLink {
                            id: id.clone(),
                            url_template: url_template.clone(),
                            arguments: arguments.clone(),
                        }) {
                            tracing::error!("Failed to send quicklink event: {}", e);
                        }
                    } else {
                        // Open URL directly
                        let launcher = Arc::clone(&self.app_launcher);
                        if let Err(e) =
                            launcher.execute_action(&SearchAction::OpenUrl { url: final_url })
                        {
                            tracing::error!("Failed to open quicklink URL: {}", e.user_message());
                        }
                    }
                    self.hide(cx);
                },
                SearchAction::OpenQuickLinks => {
                    if let Err(e) = app_events::send_event(AppEvent::OpenQuickLinks) {
                        tracing::error!("Failed to send quick links event: {}", e);
                    }
                    self.hide(cx);
                },
                SearchAction::OpenSleepTimer { expression } => {
                    let manager = Arc::clone(&self.timer_manager);
                    let expression_value = expression.clone();
                    let is_cancel = expression_value == "cancel";
                    let is_status = expression_value == "status" || expression_value == "show";

                    if is_status {
                        // For status, refresh the search to show active timer inline
                        self.on_query_change(self.search.query.clone(), cx);
                    } else {
                        // For cancel or set timer, execute and hide
                        cx.spawn(|_, _| async move {
                            let manager = manager.read().await;
                            if is_cancel {
                                match manager.cancel_timer().await {
                                    Ok(()) => tracing::info!("Timer cancelled"),
                                    Err(e) => tracing::error!("Failed to cancel timer: {}", e),
                                }
                            } else {
                                match manager.set_timer_from_expression(&expression_value).await {
                                    Ok(_) => tracing::info!("Timer set: {}", expression_value),
                                    Err(e) => tracing::error!("Failed to set timer: {}", e),
                                }
                            }
                        })
                        .detach();
                        self.hide(cx);
                    }
                },
                SearchAction::OpenCalendar { command_id } => {
                    if let Err(e) = app_events::send_event(AppEvent::OpenCalendar {
                        command_id: command_id.clone(),
                    }) {
                        tracing::error!("Failed to send calendar event: {}", e);
                    }
                    // Don't hide - calendar will be shown in this window
                },
                SearchAction::ExecuteWindowCommand { command_id } => {
                    // Hide first to avoid reentrancy issues with GPUI
                    self.hide(cx);

                    // Send event to main event loop which processes it outside GPUI window context
                    // This avoids reentrancy panics when macOS sends windowDidMove notifications
                    // Include the previous frontmost app and window title so we target the correct window
                    if let Err(e) = app_events::send_event(AppEvent::ExecuteWindowCommand {
                        command_id: command_id.clone(),
                        target_bundle_id: self.previous_frontmost_app.clone(),
                        target_window_title: self.previous_frontmost_window_title.clone(),
                    }) {
                        tracing::error!("Failed to send window command event: {}", e);
                    }
                },
                SearchAction::OpenAppManagement { command_id } => {
                    if let Err(e) = app_events::send_event(AppEvent::OpenApps {
                        command_id: command_id.clone(),
                    }) {
                        tracing::error!("Failed to send apps event: {}", e);
                    }
                    self.hide(cx);
                },
                SearchAction::ForceQuitApp { pid } => {
                    if let Err(e) = self.app_manager.force_quit_app(*pid) {
                        tracing::error!("Failed to force quit app: {}", e);
                    }
                    self.hide(cx);
                },
                SearchAction::EnterFileSearchMode => {
                    // Enter File Search Mode
                    tracing::debug!("Entering File Search Mode");
                    self.enter_file_search_mode(cx);
                },
                SearchAction::QuickLookFile { path } => {
                    // Quick Look is handled separately
                    tracing::info!(path = %path.display(), "Quick Look not yet implemented");
                },
                SearchAction::ExecuteCustomCommand {
                    command_id,
                    arguments,
                } => {
                    // Custom commands are executed via custom_commands module
                    tracing::info!(
                        command_id = %command_id,
                        arguments = %arguments,
                        "Executing custom command"
                    );

                    // Load and execute custom command
                    use photoncast_core::custom_commands::{
                        CommandExecutor as CustomCommandExecutor, CustomCommandStore,
                        PlaceholderContext,
                    };

                    match CustomCommandStore::open_default() {
                        Ok(store) => {
                            if let Ok(Some(cmd)) = store.get(command_id) {
                                let executor = CustomCommandExecutor::default();
                                let context = PlaceholderContext::with_query(arguments);

                                // Execute in background using shared runtime
                                let cmd = cmd.clone();
                                let rt = Arc::clone(&self.calculator.runtime);
                                std::thread::spawn(move || {
                                    rt.block_on(async {
                                        match executor.execute(&cmd, &context).await {
                                            Ok(result) => {
                                                if result.success {
                                                    tracing::info!(
                                                        "Custom command executed: {}",
                                                        cmd.name
                                                    );
                                                } else {
                                                    tracing::warn!(
                                                        "Custom command failed: {} - {}",
                                                        cmd.name,
                                                        result.stderr
                                                    );
                                                }
                                            },
                                            Err(e) => {
                                                tracing::error!(
                                                    "Custom command execution error: {}",
                                                    e
                                                );
                                            },
                                        }
                                    });
                                });
                            } else {
                                tracing::error!("Custom command not found: {}", command_id);
                            }
                        },
                        Err(e) => {
                            tracing::error!("Failed to open custom commands store: {}", e);
                        },
                    }
                    self.hide(cx);
                },
                SearchAction::ExecuteExtensionCommand {
                    extension_id,
                    command_id,
                } => {
                    // Extension commands are executed via extension manager
                    let result = self
                        .photoncast_app
                        .read()
                        .launch_extension_command(extension_id, command_id);

                    match result {
                        Ok(()) => {
                            tracing::info!(
                                extension_id = %extension_id,
                                command_id = %command_id,
                                "Extension command executed"
                            );
                            // Check if the extension rendered a view
                            let pending_view = self.photoncast_app.read().take_extension_view(extension_id);
                            if let Some(ext_view) = pending_view {
                                tracing::info!(
                                    extension_id = %extension_id,
                                    "Extension rendered a view, displaying it"
                                );
                                // Create action callback to handle cancel and other actions
                                let view_handle = cx.view().downgrade();
                                let action_callback: crate::extension_views::ActionCallback =
                                    std::sync::Arc::new(move |action_id, cx| {
                                        if action_id == crate::extension_views::CLOSE_VIEW_ACTION {
                                            if let Some(view) = view_handle.upgrade() {
                                                view.update(cx, |launcher, cx| {
                                                    launcher.close_extension_view(cx);
                                                });
                                            }
                                        }
                                    });
                                let rendered = crate::extension_views::render_extension_view(
                                    ext_view,
                                    Some(action_callback),
                                    cx,
                                );
                                // Focus the extension view so it receives keyboard events
                                if let Ok(list_view) = rendered.clone().downcast::<crate::extension_views::ExtensionListView>() {
                                    cx.focus_view(&list_view);
                                }
                                self.extension_view.view = Some(rendered);
                                self.extension_view.id = Some(extension_id.to_string());
                                // Resize window to fit extension view
                                crate::platform::resize_window(
                                    crate::constants::LAUNCHER_WIDTH.0.into(),
                                    crate::constants::EXPANDED_HEIGHT.0.into(),
                                );
                                cx.notify();
                            } else {
                                self.hide(cx);
                            }
                        },
                        Err(photoncast_core::app::ExtensionLaunchError::PermissionsConsentRequired {
                            extension_id: ext_id,
                            dialog,
                        }) => {
                            // Show permissions consent dialog
                            tracing::info!(
                                extension_id = %ext_id,
                                "Extension requires permissions consent"
                            );
                            self.pending_permissions_consent =
                                Some(crate::permissions_dialog::PendingPermissionsConsent {
                                    dialog,
                                    pending_command: Some((
                                        ext_id,
                                        command_id.to_string(),
                                    )),
                                    is_first_launch: false,
                                });
                            cx.notify();
                        },
                        Err(e) => {
                            tracing::error!(
                                extension_id = %extension_id,
                                command_id = %command_id,
                                error = %e,
                                "Failed to execute extension command"
                            );
                            self.hide(cx);
                        },
                    }
                },
            }
        }
    }

    pub(super) fn quick_select(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        // Check if meeting widget is visible (query empty + meeting exists)
        let has_meeting = self.search.query.is_empty() && self.meeting.next_meeting.is_some();

        if has_meeting {
            if index == 0 {
                // Cmd+1 selects the meeting
                self.meeting.selected = true;
                self.activate(&Activate, cx);
            } else {
                // Cmd+2 -> results[0], Cmd+3 -> results[1], etc.
                let result_index = index - 1;
                if result_index < self.search.results.len() {
                    self.meeting.selected = false;
                    self.search.selected_index = result_index;
                    self.activate(&Activate, cx);
                }
            }
        } else {
            // No meeting visible, indices map directly to results
            if index < self.search.results.len() {
                self.meeting.selected = false;
                self.search.selected_index = index;
                self.activate(&Activate, cx);
            }
        }
    }

    /// Enters File Search Mode.
    ///
    /// This changes the launcher UI to show file search functionality:
    /// - Different placeholder text ("Search files...")
    /// - File search results instead of apps/commands
    /// - Different keyboard shortcuts (Cmd+Enter = Reveal, Cmd+Y = Quick Look)
    pub(super) fn enter_file_search_mode(&mut self, cx: &mut ViewContext<Self>) {
        tracing::debug!("Entering File Search Mode");
        self.search.mode = SearchMode::FileSearch;

        // Create the file search view
        let file_search_view = cx.new_view(|cx| {
            let mut view = crate::file_search_view::FileSearchView::new(cx);
            // Load recent files
            view.loading = true;
            view
        });

        // Observe the file search view for should_close, needs_refetch, query_changed, and action flags
        cx.observe(&file_search_view, |this, view, cx| {
            let (
                should_close,
                needs_refetch,
                query_changed,
                filter,
                query,
                wants_reveal,
                wants_quick_look,
                wants_actions,
                wants_open,
                selected_path,
            ) = {
                let v = view.read(cx);
                (
                    v.should_close,
                    v.needs_refetch,
                    v.query_changed,
                    v.filter,
                    v.query.clone(),
                    v.wants_reveal_in_finder,
                    v.wants_quick_look,
                    v.wants_actions_menu,
                    v.wants_open_file,
                    v.selected_file().map(|f| f.path.clone()),
                )
            };

            if should_close {
                // Hide the entire launcher window
                this.hide(cx);
                return;
            }

            // Handle file action requests
            if wants_open {
                view.update(cx, |v, _| v.wants_open_file = false);
                if let Some(path) = &selected_path {
                    tracing::info!("Opening file: {}", path.display());
                    let _ = std::process::Command::new("open").arg(path).spawn();
                    this.hide(cx);
                }
                return;
            }

            if wants_reveal {
                view.update(cx, |v, _| v.wants_reveal_in_finder = false);
                if let Some(path) = &selected_path {
                    tracing::info!("Revealing in Finder: {}", path.display());
                    let _ = photoncast_apps::reveal_in_finder(path);
                    this.hide(cx);
                }
                return;
            }

            if wants_quick_look {
                view.update(cx, |v, _| v.wants_quick_look = false);
                if let Some(path) = &selected_path {
                    tracing::info!("Quick Look: {}", path.display());
                    let _ = std::process::Command::new("qlmanage")
                        .arg("-p")
                        .arg(path)
                        .spawn();
                    // Don't hide - Quick Look is a preview
                }
                return;
            }

            if wants_actions {
                view.update(cx, |v, cx| {
                    v.wants_actions_menu = false;
                    v.actions_menu_open = !v.actions_menu_open;
                    v.actions_menu_index = 0;
                    cx.notify();
                });
                return;
            }

            // Handle query change - trigger search
            if query_changed {
                use crate::file_search_helper::{
                    spotlight_recent_files_filtered, spotlight_search,
                };

                let view_handle = view.downgrade();
                let query_str = query.to_string();
                let filter_for_search = filter;

                // Clear the flag first
                view.update(cx, |v, _| {
                    v.query_changed = false;
                });

                // If in browsing mode, don't trigger Spotlight search - browsing handles its own results
                let is_browsing =
                    view.read(cx).section_mode == crate::file_search_view::SectionMode::Browsing;
                if is_browsing {
                    return;
                }

                // If query is empty, reload recent files (filtered if a filter is active)
                if query_str.is_empty() {
                    cx.spawn(|_this, mut cx| async move {
                        // Use filtered fetch to respect current filter
                        let recent_files =
                            cx.background_executor()
                                .spawn(async move {
                                    spotlight_recent_files_filtered(filter_for_search, 50)
                                })
                                .await;

                        if let Some(view) = view_handle.upgrade() {
                            let _ = view.update(&mut cx, |view, cx| {
                                view.all_results = recent_files.clone();
                                view.results = recent_files;
                                view.loading = false;
                                view.section_mode = crate::file_search_view::SectionMode::Recent;
                                view.selected_index = 0;
                                cx.notify();
                            });
                        }
                    })
                    .detach();
                } else if query_str.len() >= 2 {
                    // Search using native SpotlightSearchService
                    view.update(cx, |v, cx| {
                        v.loading = true;
                        cx.notify();
                    });

                    cx.spawn(|_this, mut cx| async move {
                        // Use native Spotlight search (has built-in caching)
                        let search_results = cx
                            .background_executor()
                            .spawn(async move { spotlight_search(&query_str, 50) })
                            .await;

                        if let Some(view) = view_handle.upgrade() {
                            let _ = view.update(&mut cx, |view, cx| {
                                // Apply filter to search results
                                view.all_results = search_results;
                                view.results = view
                                    .all_results
                                    .iter()
                                    .filter(|f| filter_for_search.matches(f.kind, &f.path))
                                    .cloned()
                                    .collect();
                                view.loading = false;
                                view.section_mode = crate::file_search_view::SectionMode::Search;
                                view.selected_index = 0;
                                cx.notify();
                            });
                        }
                    })
                    .detach();
                }
                return;
            }

            if needs_refetch {
                use crate::file_search_helper::spotlight_recent_files_filtered;

                // Re-fetch files for the new filter type using native Spotlight
                // Use the filtered fetch to get files of the specific type
                let view_handle = view.downgrade();
                let filter_for_closure = filter;

                cx.spawn(|_this, mut cx| async move {
                    // Fetch files matching the filter type directly
                    let recent_files = cx
                        .background_executor()
                        .spawn(
                            async move { spotlight_recent_files_filtered(filter_for_closure, 50) },
                        )
                        .await;

                    if let Some(view) = view_handle.upgrade() {
                        let _ = view.update(&mut cx, |view, cx| {
                            // Results are already filtered, set directly
                            view.all_results = recent_files.clone();
                            view.results = recent_files;
                            view.loading = false;
                            view.needs_refetch = false;
                            view.selected_index = 0;
                            tracing::info!(
                                "[FileSearch] Refetch complete: {} files for {:?}",
                                view.results.len(),
                                filter_for_closure
                            );
                            cx.notify();
                        });
                    }
                })
                .detach();
            }
        })
        .detach();

        // Load recent files in background using native Spotlight
        let view_handle = file_search_view.downgrade();
        cx.spawn(|_this, mut cx| async move {
            use crate::file_search_helper::spotlight_recent_files;

            // Use native Spotlight to fetch recent files (7 days, max 50 results)
            // SpotlightSearchService has built-in caching
            let recent_files = cx
                .background_executor()
                .spawn(async move { spotlight_recent_files(7, 50) })
                .await;

            // Update the view with results
            if let Some(view) = view_handle.upgrade() {
                let _ = view.update(&mut cx, |view, cx| {
                    view.set_results(recent_files);
                    view.loading = false;
                    view.section_mode = crate::file_search_view::SectionMode::Recent;
                    cx.notify();
                });
            }
        })
        .detach();

        self.file_search.view = Some(file_search_view.clone());

        // Focus the file search view after storing it
        cx.focus_view(&file_search_view);
        self.reset_query();
        self.search.results.clear();
        self.search.base_results.clear();
        self.search.core_results.clear();
        self.search.selected_index = 0;
        self.file_search.loading = false;
        self.file_search.pending_query = None;
        self.file_search.generation += 1;
        self.calculator.result = None;
        self.calculator.generation = self.calculator.generation.saturating_add(1);

        // Resize window to fit file search view (deferred via dispatch_async)
        crate::platform::resize_window(LAUNCHER_WIDTH.0.into(), EXPANDED_HEIGHT.0.into());

        cx.notify();
    }

    /// Exits File Search Mode and returns to normal search.
    pub(super) fn exit_file_search_mode(&mut self, cx: &mut ViewContext<Self>) {
        tracing::info!("Exiting File Search Mode");
        self.search.mode = SearchMode::Normal;
        self.file_search.view = None; // Clean up the file search view
        self.reset_query();
        self.search.results.clear();
        self.search.base_results.clear();
        self.search.core_results.clear();
        self.search.selected_index = 0;
        self.file_search.loading = false;
        self.file_search.pending_query = None;
        // Reload suggestions for empty state
        self.load_suggestions(cx);
        self.file_search.generation += 1;
        self.calculator.result = None;
        self.calculator.generation = self.calculator.generation.saturating_add(1);

        // Resize window back to normal (deferred via dispatch_async)
        crate::platform::resize_window(LAUNCHER_WIDTH.0.into(), LAUNCHER_HEIGHT.0.into());

        cx.notify();
    }

    pub(super) fn next_group(&mut self, _: &NextGroup, cx: &mut ViewContext<Self>) {
        // If file search view is in browsing mode, use Tab to enter folder
        if let Some(file_search_view) = &self.file_search.view {
            let is_browsing = file_search_view.read(cx).section_mode
                == crate::file_search_view::SectionMode::Browsing;
            if is_browsing {
                file_search_view.update(cx, |view, cx| {
                    view.browse_enter_folder(cx);
                });
                return;
            }
        }

        // Check if we should autocomplete a quicklink instead of navigating groups
        // This happens when: there's only 1 result OR the selected result is a quicklink that needs input
        if let Some(core_result) = self.search.core_results.get(self.search.selected_index) {
            if let SearchAction::ExecuteQuickLink { url_template, .. } = &core_result.action {
                if photoncast_quicklinks::placeholder::requires_user_input(url_template) {
                    // Only 1 result, or quicklink is selected - autocomplete it
                    if self.search.results.len() == 1 || self.search.selected_index == 0 {
                        let autocomplete =
                            if let Some(alias_match) = core_result.subtitle.strip_prefix('/') {
                                alias_match
                                    .split(" · ")
                                    .next()
                                    .unwrap_or(&core_result.title)
                                    .to_string()
                            } else {
                                core_result.title.clone()
                            };
                        let new_query = format!("{} ", autocomplete);
                        self.search.cursor_position = new_query.chars().count();
                        self.search.selection_anchor = None;
                        self.search.query = SharedString::from(new_query);
                        self.on_query_change(self.search.query.clone(), cx);
                        self.reset_cursor_blink();
                        cx.notify();
                        return;
                    }
                }
            }
        }

        if self.search.results.is_empty() {
            return;
        }

        // Find current group
        let current_type = self
            .search
            .results
            .get(self.search.selected_index)
            .map(|r| r.result_type);

        if let Some(current_type) = current_type {
            // Find the first item of the next group
            let mut found_current = false;
            for (idx, result) in self.search.results.iter().enumerate() {
                if !found_current && result.result_type == current_type {
                    found_current = true;
                }
                if found_current && result.result_type != current_type {
                    self.search.selected_index = idx;
                    self.ensure_selected_visible(cx);
                    cx.notify();
                    return;
                }
            }

            // No next group found, wrap to first item
            self.search.selected_index = 0;
            self.ensure_selected_visible(cx);
        }
        cx.notify();
    }

    pub(super) fn previous_group(&mut self, _: &PreviousGroup, cx: &mut ViewContext<Self>) {
        // If file search view is in browsing mode, use Shift+Tab to go to parent directory
        if let Some(file_search_view) = &self.file_search.view {
            let is_browsing = file_search_view.read(cx).section_mode
                == crate::file_search_view::SectionMode::Browsing;
            if is_browsing {
                file_search_view.update(cx, |view, cx| {
                    view.browse_go_back(cx);
                });
                return;
            }
        }

        if self.search.results.is_empty() {
            return;
        }

        // Find current group
        let current_type = self
            .search
            .results
            .get(self.search.selected_index)
            .map(|r| r.result_type);

        if let Some(current_type) = current_type {
            // Find the first item of current group
            let current_group_start = self
                .search
                .results
                .iter()
                .position(|r| r.result_type == current_type)
                .unwrap_or(0);

            if current_group_start > 0 {
                // Find the previous group's first item
                let prev_type = self.search.results[current_group_start - 1].result_type;
                let prev_group_start = self
                    .search
                    .results
                    .iter()
                    .position(|r| r.result_type == prev_type)
                    .unwrap_or(0);
                self.search.selected_index = prev_group_start;
            } else {
                // Already at first group, wrap to last group's first item
                let last_type = self.search.results.last().map(|r| r.result_type);
                if let Some(last_type) = last_type {
                    let last_group_start = self
                        .search
                        .results
                        .iter()
                        .position(|r| r.result_type == last_type)
                        .unwrap_or(0);
                    self.search.selected_index = last_group_start;
                }
            }
            self.ensure_selected_visible(cx);
        }
        cx.notify();
    }

    /// Ensures the selected item is visible by scrolling if needed.
    pub(super) fn ensure_selected_visible(&self, cx: &mut ViewContext<Self>) {
        // Calculate item height based on mode
        let item_height = if matches!(self.search.mode, SearchMode::Calendar { .. }) {
            56.0 // Calendar items are taller
        } else {
            RESULT_ITEM_HEIGHT.0
        };

        // Calculate visible area height
        let visible_height = (MAX_VISIBLE_RESULTS as f32) * item_height;

        // Calculate the top position of the selected item
        // For calendar mode, we need to account for day headers
        let item_top = if let SearchMode::Calendar { events, .. } = &self.search.mode {
            // Count day headers before this item
            let mut header_count = 0;
            let mut current_day: Option<photoncast_calendar::chrono::NaiveDate> = None;
            for (i, event) in events.iter().enumerate() {
                let event_day = event.start.date_naive();
                if current_day != Some(event_day) {
                    current_day = Some(event_day);
                    header_count += 1;
                }
                if i >= self.search.selected_index {
                    break;
                }
            }
            (self.search.selected_index as f32 * item_height) + (header_count as f32 * 28.0)
        } else {
            self.search.selected_index as f32 * item_height
        };

        let item_bottom = item_top + item_height;

        // Get current scroll offset
        let current_offset = self.results_scroll_handle.offset();
        let scroll_top = -current_offset.y.0;
        let scroll_bottom = scroll_top + visible_height;

        // Check if item is visible
        if item_top < scroll_top {
            // Item is above visible area - scroll up
            self.results_scroll_handle
                .set_offset(gpui::Point::new(px(0.0), px(-item_top)));
            cx.notify();
        } else if item_bottom > scroll_bottom {
            // Item is below visible area - scroll down
            let new_scroll_top = item_bottom - visible_height;
            self.results_scroll_handle
                .set_offset(gpui::Point::new(px(0.0), px(-new_scroll_top)));
            cx.notify();
        }
    }

    pub(super) fn cancel(&mut self, _: &Cancel, cx: &mut ViewContext<Self>) {
        // If file search view is active, check for menus first
        if let Some(file_search_view) = &self.file_search.view {
            let handled = file_search_view.update(cx, |view, cx| {
                if view.actions_menu_open {
                    view.actions_menu_open = false;
                    cx.notify();
                    true
                } else if view.dropdown_open {
                    view.dropdown_open = false;
                    cx.notify();
                    true
                } else {
                    false
                }
            });
            if handled {
                return;
            }
            // No menu open, hide the window
            self.hide(cx);
            return;
        }

        // If uninstall preview is showing, close it first
        if self.uninstall.preview.is_some() {
            self.cancel_uninstall_preview(cx);
            return;
        }

        // If auto-quit settings is showing, close it first
        if self.auto_quit.settings_app.is_some() {
            self.close_auto_quit_settings(cx);
            return;
        }

        // If actions menu is showing, close it first
        if self.actions_menu.visible {
            self.actions_menu.visible = false;
            cx.notify();
            return;
        }

        // If confirmation dialog is showing, cancel it first
        if self.pending_confirmation.is_some() {
            self.cancel_confirmation(cx);
            return;
        }

        // If permissions consent dialog is showing, deny and close it
        if self.pending_permissions_consent.is_some() {
            self.deny_permissions_consent(cx);
            return;
        }

        // If extension view is showing, hide everything
        if self.extension_view.view.is_some() {
            self.hide(cx);
            return;
        }

        // If in file search mode (without view - shouldn't happen), exit back to normal mode
        if matches!(self.search.mode, SearchMode::FileSearch) {
            self.exit_file_search_mode(cx);
            return;
        }

        if matches!(self.search.mode, SearchMode::Calendar { .. }) {
            self.exit_calendar_mode(cx);
            return;
        }

        if self.search.query.is_empty() {
            // Close window with animation (hide() calls start_dismiss_animation which quits)
            self.hide(cx);
        } else {
            // Clear query first
            self.reset_query();
            self.search.results.clear();
            self.search.base_results.clear();
            self.search.core_results.clear();
            self.search.selected_index = 0;
            self.calculator.result = None;
            self.calculator.generation = self.calculator.generation.saturating_add(1);
            // Reload suggestions for empty state
            self.load_suggestions(cx);
            cx.notify();
        }
    }
}
