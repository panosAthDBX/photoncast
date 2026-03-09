//! Indexing methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Starts async app indexing in the background
    pub(super) fn start_app_indexing(&self, cx: &mut ViewContext<Self>) {
        let photoncast_app = Arc::clone(&self.photoncast_app);
        let shared_runtime = Arc::clone(&self.calculator.runtime);

        // Use std::thread::spawn because AppScanner requires Tokio runtime,
        // but GPUI uses its own async executor
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            tracing::info!("Starting application indexing...");

            let result = shared_runtime.block_on(async {
                let config = photoncast_core::app::config_file::load_config().unwrap_or_default();
                let scanner = AppScanner::from_config(&config.search.app_search_scope);
                scanner.scan_all().await
            });

            // Send scan results immediately so UI becomes responsive
            let _ = tx.send(result);
        });

        // Poll for results in GPUI's async context
        cx.spawn(|this, mut cx| async move {
            // Wait a bit then check for results
            loop {
                // Use GPUI's timer for async sleep
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;

                match rx.try_recv() {
                    Ok(Ok(apps)) => {
                        let app_count = apps.len();
                        let apps_for_icons = apps.clone();
                        tracing::info!("Indexed {} applications", app_count);

                        // Update the PhotonCast app with indexed apps
                        photoncast_app.write().set_apps(apps);

                        // Mark indexing as complete and start file watcher
                        let _ = this.update(&mut cx, |this, cx| {
                            this.index_initialized = true;
                            // Load suggestions now that index is ready
                            this.load_suggestions(cx);
                            // Start watching for app changes
                            this.start_app_watching(cx);
                            cx.notify();
                        });

                        let photoncast_app_for_icons = Arc::clone(&photoncast_app);
                        let this_for_icons = this.clone();
                        cx.spawn(|mut cx| async move {
                            tracing::info!(
                                "Starting background icon extraction for {} apps",
                                apps_for_icons.len()
                            );

                            let mut updated_icons = 0usize;
                            for app in apps_for_icons {
                                let app_path = app.path.clone();
                                let bundle_id = app.bundle_id.to_string();

                                let icon_result = cx
                                    .background_executor()
                                    .spawn(async move { Self::get_app_icon_path_static(&app_path) })
                                    .await;

                                if let Some(icon_path) = icon_result {
                                    photoncast_app_for_icons
                                        .write()
                                        .update_app_icon(&bundle_id, icon_path);
                                    updated_icons += 1;

                                    let _ = this_for_icons.update(&mut cx, |this, cx| {
                                        this.refresh_visible_app_icons(cx);
                                    });
                                }
                            }

                            tracing::info!(
                                "Background icon extraction complete: {} icons updated",
                                updated_icons
                            );
                        })
                        .detach();
                        break;
                    },
                    Ok(Err(e)) => {
                        tracing::error!("Failed to index applications: {}", e);
                        // Still mark as initialized to allow searching commands
                        let _ = this.update(&mut cx, |this, cx| {
                            this.index_initialized = true;
                            this.load_suggestions(cx);
                            // Start watching even if indexing failed
                            this.start_app_watching(cx);
                            cx.notify();
                        });
                        break;
                    },
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // Still waiting, continue polling
                    },
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        tracing::error!("Indexing thread disconnected");
                        let _ = this.update(&mut cx, |this, cx| {
                            this.index_initialized = true;
                            this.load_suggestions(cx);
                            cx.notify();
                        });
                        break;
                    },
                }
            }
        })
        .detach();
    }

    /// Starts watching application directories for changes.
    ///
    /// This runs in the background and automatically re-indexes apps when:
    /// - A new app is installed
    /// - An existing app is updated
    /// - An app is uninstalled
    pub(super) fn start_app_watching(&self, cx: &mut ViewContext<Self>) {
        let photoncast_app = Arc::clone(&self.photoncast_app);
        let shared_runtime = Arc::clone(&self.calculator.runtime);

        // Start the watcher in a background thread (requires Tokio runtime)
        let (event_tx, event_rx) = std::sync::mpsc::channel::<WatchEvent>();

        std::thread::spawn(move || {
            shared_runtime.block_on(async {
                let mut watcher = AppWatcher::new();

                match watcher.start() {
                    Ok(mut rx) => {
                        tracing::info!("Application file watcher started");

                        // Forward events to the main thread
                        while let Some(event) = rx.recv().await {
                            tracing::debug!("Received watch event: {:?}", event);
                            if event_tx.send(event).is_err() {
                                // Receiver dropped, stop watching
                                tracing::info!("Watch event receiver dropped, stopping watcher");
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to start application watcher: {}", e);
                    },
                }
            });
        });
        let event_rx = Arc::new(std::sync::Mutex::new(event_rx));

        // Process watch events in GPUI's async context
        cx.spawn(|this, mut cx| async move {
            const WATCH_EVENT_TIMEOUT_MS: u64 = 500;

            loop {
                let next_events = {
                    let event_rx = Arc::clone(&event_rx);
                    cx.background_executor().spawn(async move {
                        let receiver = event_rx.lock().expect("watch event receiver poisoned");
                        match receiver.recv_timeout(Duration::from_millis(WATCH_EVENT_TIMEOUT_MS)) {
                            Ok(first_event) => {
                                let mut events = vec![first_event];
                                while let Ok(event) = receiver.try_recv() {
                                    events.push(event);
                                }
                                Ok(events)
                            },
                            Err(err) => Err(err),
                        }
                    })
                }
                .await;

                match next_events {
                    Ok(events) => {
                        for event in events {
                            Self::handle_watch_event(&this, &mut cx, &photoncast_app, event).await;
                        }
                    },
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // No watch events before timeout, continue waiting.
                    },
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        tracing::info!("Watcher thread disconnected");
                        return;
                    },
                }
            }
        })
        .detach();
    }

    /// Starts the auto-quit background timer.
    ///
    /// This periodically checks for idle apps and quits them based on user settings.
    /// The timer runs every 30 seconds and:
    /// 1. Updates activity tracking for the currently frontmost app
    /// 2. Quits any apps that have been idle longer than their configured timeout
    pub(super) fn start_auto_quit_timer(&self, cx: &mut ViewContext<Self>) {
        let auto_quit_manager = Arc::clone(&self.auto_quit.manager);

        cx.spawn(|this, mut cx| async move {
            const CHECK_INTERVAL_SECS: u64 = 5; // Check every 5 seconds for precise timing

            loop {
                // Wait for the check interval
                cx.background_executor()
                    .timer(Duration::from_secs(CHECK_INTERVAL_SECS))
                    .await;

                // Skip if no apps have auto-quit enabled
                {
                    let manager = auto_quit_manager.read();
                    if !manager.has_enabled_apps() {
                        continue;
                    }
                }

                // Perform the auto-quit tick
                let (frontmost, quit_apps) = {
                    let mut manager = auto_quit_manager.write();
                    manager.tick()
                };

                // Log activity for debugging
                if let Some(ref bundle_id) = frontmost {
                    tracing::trace!("Auto-quit tick: frontmost app = {}", bundle_id);
                }

                // Show toast notifications for quit apps
                for bundle_id in quit_apps {
                    tracing::info!("Auto-quit: Quit idle app: {}", bundle_id);
                    let _ = this.update(&mut cx, |this, cx| {
                        this.show_toast(format!("Auto-quit: {}", bundle_id), cx);
                    });
                }
            }
        })
        .detach();
    }

    /// Handles a single watch event by updating the app index.
    #[allow(clippy::future_not_send)]
    pub(super) async fn handle_watch_event(
        this: &WeakView<Self>,
        cx: &mut AsyncWindowContext,
        photoncast_app: &Arc<RwLock<PhotonCastApp>>,
        event: WatchEvent,
    ) {
        match event {
            WatchEvent::AppAdded(path) => {
                tracing::info!(path = %path.display(), "New app detected, indexing...");
                // Parse the new app's metadata
                if let Ok(app) = photoncast_core::indexer::parse_app_metadata(&path).await {
                    tracing::info!(name = %app.name, "Indexed new app");

                    // Extract icon in background
                    let app_path = app.path.clone();
                    let bundle_id = app.bundle_id.to_string();
                    let photoncast_app_for_icon = Arc::clone(photoncast_app);
                    let this_for_icon = this.clone();

                    // Add app to index
                    photoncast_app.write().update_or_add_app(app);

                    // Extract icon in background and notify UI when done
                    cx.spawn(|mut cx| async move {
                        let icon_result = cx
                            .background_executor()
                            .spawn(async move { Self::get_app_icon_path_static(&app_path) })
                            .await;

                        if let Some(icon_path) = icon_result {
                            photoncast_app_for_icon
                                .write()
                                .update_app_icon(&bundle_id, icon_path);

                            let _ = this_for_icon.update(&mut cx, |this, cx| {
                                this.refresh_visible_app_icons(cx);
                            });
                        }
                    })
                    .detach();

                    // Notify UI to refresh if query matches
                    let _ = this.update(cx, |this, cx| {
                        if !this.search.query.is_empty() {
                            this.on_query_change(this.search.query.clone(), cx);
                        }
                        cx.notify();
                    });
                } else {
                    tracing::warn!(path = %path.display(), "Failed to parse new app metadata");
                }
            },
            WatchEvent::AppModified(path) => {
                tracing::info!(path = %path.display(), "App modified, re-indexing...");
                // Re-parse the app's metadata
                if let Ok(app) = photoncast_core::indexer::parse_app_metadata(&path).await {
                    tracing::info!(name = %app.name, "Re-indexed modified app");

                    // Update app in index
                    let app_path = app.path.clone();
                    let bundle_id = app.bundle_id.to_string();
                    let photoncast_app_for_icon = Arc::clone(photoncast_app);
                    let this_for_icon = this.clone();

                    photoncast_app.write().update_or_add_app(app);

                    // Re-extract icon (might have changed) and notify UI when done
                    // Only clear the old icon AFTER successfully extracting the new one
                    cx.spawn(|mut cx| async move {
                        let icon_result = cx
                            .background_executor()
                            .spawn(async move {
                                // First try to extract new icon WITHOUT clearing the old one
                                // This ensures we don't lose the icon if extraction fails
                                let new_icon = Self::get_app_icon_path_static(&app_path);
                                if new_icon.is_some() {
                                    // Successfully extracted, now safe to clear old cached data
                                    // (the memory cache entry, not the disk file since we just wrote it)
                                    crate::icon_cache::invalidate_memory_cache(&app_path);
                                }
                                new_icon
                            })
                            .await;

                        if let Some(icon_path) = icon_result {
                            photoncast_app_for_icon
                                .write()
                                .update_app_icon(&bundle_id, icon_path);

                            let _ = this_for_icon.update(&mut cx, |this, cx| {
                                this.refresh_visible_app_icons(cx);
                            });
                        }
                    })
                    .detach();

                    // Notify UI to refresh
                    let _ = this.update(cx, |this, cx| {
                        if !this.search.query.is_empty() {
                            this.on_query_change(this.search.query.clone(), cx);
                        }
                        cx.notify();
                    });
                } else {
                    // Parsing failed - check if app still exists
                    if !path.exists() {
                        // App was likely uninstalled, remove from index
                        tracing::info!(path = %path.display(), "App no longer exists, removing from index");
                        let removed = photoncast_app.write().remove_app_by_path(&path);
                        if removed {
                            // Clear cached icon
                            let path_for_icon = path.clone();
                            cx.background_executor()
                                .spawn(async move {
                                    Self::clear_cached_icon(&path_for_icon);
                                })
                                .detach();

                            // Notify UI to refresh
                            let _ = this.update(cx, |this, cx| {
                                if !this.search.query.is_empty() {
                                    this.on_query_change(this.search.query.clone(), cx);
                                }
                                cx.notify();
                            });
                        }
                    } else {
                        tracing::warn!(path = %path.display(), "Failed to parse modified app metadata");
                    }
                }
            },
            WatchEvent::AppRemoved(path) => {
                tracing::info!(path = %path.display(), "App removed, updating index...");
                let removed = photoncast_app.write().remove_app_by_path(&path);
                if removed {
                    tracing::info!(path = %path.display(), "Removed app from index");

                    // Clear cached icon
                    cx.background_executor()
                        .spawn(async move {
                            Self::clear_cached_icon(&path);
                        })
                        .detach();

                    // Notify UI to refresh
                    let _ = this.update(cx, |this, cx| {
                        if !this.search.query.is_empty() {
                            this.on_query_change(this.search.query.clone(), cx);
                        }
                        cx.notify();
                    });
                }
            },
        }
    }

    /// Static version of `get_app_icon_path` for use in async context.
    /// Delegates to [`crate::icon_cache::get_icon_static`].
    pub(super) fn get_app_icon_path_static(
        app_path: &std::path::Path,
    ) -> Option<std::path::PathBuf> {
        crate::icon_cache::get_icon_static(app_path)
    }

    /// Clears the cached icon for an app.
    /// Delegates to [`crate::icon_cache::clear_icon`].
    pub(super) fn clear_cached_icon(app_path: &std::path::Path) {
        crate::icon_cache::clear_icon(app_path);
    }

    /// Checks if an icon is already cached, returns path if so.
    /// Delegates to [`crate::icon_cache::get_cached_icon_path`].
    pub(super) fn get_cached_icon_path(app_path: &std::path::Path) -> Option<std::path::PathBuf> {
        crate::icon_cache::get_cached_icon_path(app_path)
    }

    fn backfill_cached_icon_paths(results: &mut [SearchResult]) -> bool {
        let mut changed = false;

        for result in results {
            let SearchAction::LaunchApp { path, .. } = &result.action else {
                continue;
            };

            let IconSource::AppIcon { icon_path, .. } = &mut result.icon else {
                continue;
            };

            if icon_path.is_some() {
                continue;
            }

            if let Some(cached_path) = Self::get_cached_icon_path(path) {
                *icon_path = Some(cached_path);
                changed = true;
            }
        }

        changed
    }

    fn refresh_visible_app_icons(&mut self, cx: &mut ViewContext<Self>) {
        let suggestions_changed = Self::backfill_cached_icon_paths(&mut self.search.suggestions);
        let core_changed = Self::backfill_cached_icon_paths(&mut self.search.core_results);

        if !(suggestions_changed || core_changed) {
            return;
        }

        if !matches!(self.search.mode, SearchMode::Calendar { .. }) {
            self.search.results = self
                .search
                .core_results
                .iter()
                .map(Self::search_result_to_result_item)
                .collect();
        }

        cx.notify();
    }

    /// Converts an icon source to a display emoji (fallback)
    pub(super) fn icon_to_emoji(icon: &IconSource) -> SharedString {
        match icon {
            IconSource::Emoji { char } => SharedString::from(char.to_string()),
            IconSource::SystemIcon { name } => {
                // Map system icon names to appropriate emojis
                match name.as_str() {
                    "lock" | "lock.fill" => "🔒".into(),
                    "moon.fill" | "sleep" => "😴".into(),
                    "arrow.clockwise" | "restart" => "🔄".into(),
                    "power" | "shutdown" => "⏻".into(),
                    "trash" | "trash.fill" => "🗑️".into(),
                    "magnifyingglass" => "🔍".into(),
                    "folder" | "folder.fill" => "📁".into(),
                    "doc" | "doc.fill" => "📄".into(),
                    "gearshape" | "gearshape.fill" => "⚙️".into(),
                    // Window layout icons - halves
                    "arrow-left-to-line" => "⬅️".into(),
                    "arrow-right-to-line" => "➡️".into(),
                    "arrow-up-to-line" => "⬆️".into(),
                    "arrow-down-to-line" => "⬇️".into(),
                    // Window layout icons - quarters
                    "arrow-up-left" => "↖️".into(),
                    "arrow-up-right" => "↗️".into(),
                    "arrow-down-left" => "↙️".into(),
                    "arrow-down-right" => "↘️".into(),
                    // Window layout icons - thirds and panels
                    "panel-left" | "panel-left-open" => "◧".into(),
                    "panel-right" | "panel-right-open" => "◨".into(),
                    "columns-3" => "▥".into(),
                    // Window layout icons - special
                    "maximize-2" => "⬜".into(),
                    "minimize-2" => "🔽".into(),
                    "align-center" | "align-center-vertical" | "align-center-horizontal" => {
                        "⬛".into()
                    },
                    "square" => "◻️".into(),
                    "scaling" => "📐".into(),
                    "undo-2" => "↩️".into(),
                    "fullscreen" => "⛶".into(),
                    // Display movement icons
                    "monitor-arrow-right" => "🖥️➡️".into(),
                    "monitor-arrow-left" => "⬅️🖥️".into(),
                    "monitor" => "🖥️".into(),
                    _ => "📋".into(), // Default icon
                }
            },
            IconSource::AppIcon { .. } => "📱".into(),
            IconSource::FileIcon { path: _ } => "📄".into(),
        }
    }

    /// Gets or extracts the icon for an app bundle as PNG.
    /// Delegates to [`crate::icon_cache::get_icon`].
    pub(super) fn get_app_icon_path(app_path: &std::path::Path) -> Option<std::path::PathBuf> {
        crate::icon_cache::get_icon(app_path)
    }

    /// Gets icon path for an app by its bundle ID.
    #[allow(dead_code)]
    pub(super) fn get_icon_path_for_bundle_id(bundle_id: &str) -> Option<std::path::PathBuf> {
        // Try to find app path from bundle ID using NSWorkspace
        let app_path = crate::platform::get_app_path_for_bundle_id(bundle_id)?;
        Self::get_app_icon_path(&app_path)
    }

    /// Parses a hex color string (e.g., "#0088FF") to an Hsla color.
    pub(super) fn parse_hex_color(hex: &str) -> gpui::Hsla {
        let hex = hex.trim_start_matches('#');
        if hex.len() >= 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                let r = r as f32 / 255.0;
                let g = g as f32 / 255.0;
                let b = b as f32 / 255.0;

                // Convert RGB to HSL
                let max = r.max(g).max(b);
                let min = r.min(g).min(b);
                let l = (max + min) / 2.0;

                if (max - min).abs() < f32::EPSILON {
                    return hsla(0.0, 0.0, l, 1.0);
                }

                let d = max - min;
                let s = if l > 0.5 {
                    d / (2.0 - max - min)
                } else {
                    d / (max + min)
                };

                let h = if (max - r).abs() < f32::EPSILON {
                    ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
                } else if (max - g).abs() < f32::EPSILON {
                    ((b - r) / d + 2.0) / 6.0
                } else {
                    ((r - g) / d + 4.0) / 6.0
                };

                return hsla(h, s, l, 1.0);
            }
        }
        // Default blue if parsing fails
        hsla(210.0 / 360.0, 0.8, 0.5, 1.0)
    }

    /// Converts a core `SearchResult` to a UI `ResultItem`.
    /// Icons are extracted during indexing, so we just check cache here.
    pub(super) fn search_result_to_result_item(result: &SearchResult) -> ResultItem {
        // Get icon path from cache (icons extracted during indexing)
        let icon_path = match &result.icon {
            IconSource::AppIcon { icon_path, .. } => {
                // If we have a cached path from index, use it
                if icon_path.is_some() {
                    icon_path.clone()
                } else {
                    // Try to get from action's app path (check cache only, no extraction)
                    match &result.action {
                        SearchAction::LaunchApp { path, .. } => Self::get_cached_icon_path(path),
                        _ => None,
                    }
                }
            },
            _ => None,
        };

        // Extract bundle_id and app_path from action for app-related features
        let (bundle_id, app_path) = match &result.action {
            SearchAction::LaunchApp { bundle_id, path } => {
                (Some(bundle_id.clone()), Some(path.clone()))
            },
            _ => (None, None),
        };

        ResultItem {
            id: SharedString::from(result.id.to_string()),
            title: result.title.clone().into(),
            subtitle: result.subtitle.clone().into(),
            icon_emoji: Self::icon_to_emoji(&result.icon),
            icon_path,
            result_type: result.result_type.into(),
            bundle_id,
            app_path,
            requires_permissions: result.requires_permissions,
        }
    }
}
