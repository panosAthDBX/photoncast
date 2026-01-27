//! Indexing methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Starts async app indexing in the background
    pub(super) fn start_app_indexing(&self, cx: &mut ViewContext<Self>) {
        let photoncast_app = Arc::clone(&self.photoncast_app);

        // Use std::thread::spawn because AppScanner requires Tokio runtime,
        // but GPUI uses its own async executor
        let (tx, rx) = std::sync::mpsc::channel();

        let photoncast_app_for_icons = Arc::clone(&photoncast_app);
        std::thread::spawn(move || {
            tracing::info!("Starting application indexing...");

            // Create a new Tokio runtime for the scanner
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create Tokio runtime for indexing: {}", e);
                    let _ = tx.send(Err(anyhow::anyhow!("Runtime creation failed: {e}")));
                    return;
                },
            };

            let result = rt.block_on(async {
                let scanner = AppScanner::new();
                scanner.scan_all().await
            });

            // Send scan results immediately so UI becomes responsive
            let apps_for_icons = result.as_ref().ok().cloned();
            let _ = tx.send(result);

            // Now extract icons in the same background thread (doesn't block UI)
            if let Some(apps) = apps_for_icons {
                tracing::info!(
                    "Starting background icon extraction for {} apps",
                    apps.len()
                );
                let start = std::time::Instant::now();
                let mut extracted = 0;
                let mut cached = 0;

                for app in &apps {
                    // Extract or get cached icon
                    if let Some(icon_path) = Self::get_app_icon_path(&app.path) {
                        // Update the app's icon in shared state
                        photoncast_app_for_icons
                            .write()
                            .update_app_icon(&app.bundle_id.to_string(), icon_path);

                        if Self::get_cached_icon_path(&app.path).is_some() {
                            cached += 1;
                        } else {
                            extracted += 1;
                        }
                    }
                }

                tracing::info!(
                    "Icon extraction complete: {} cached, {} extracted in {:?}",
                    cached,
                    extracted,
                    start.elapsed()
                );
            }
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

        // Start the watcher in a background thread (requires Tokio runtime)
        let (event_tx, event_rx) = std::sync::mpsc::channel::<WatchEvent>();

        std::thread::spawn(move || {
            // Create Tokio runtime for the watcher
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create Tokio runtime for watcher: {}", e);
                    return;
                },
            };

            rt.block_on(async {
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

        // Process watch events in GPUI's async context
        cx.spawn(|this, mut cx| async move {
            loop {
                // Poll for events periodically
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;

                // Process all pending events
                loop {
                    match event_rx.try_recv() {
                        Ok(event) => {
                            Self::handle_watch_event(&this, &mut cx, &photoncast_app, event).await;
                        },
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            // No more events, wait for next poll
                            break;
                        },
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            tracing::info!("Watcher thread disconnected");
                            return;
                        },
                    }
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

                    // Add app to index
                    photoncast_app.write().update_or_add_app(app);

                    // Extract icon in background
                    cx.background_executor()
                        .spawn(async move {
                            if let Some(icon_path) = Self::get_app_icon_path_static(&app_path) {
                                photoncast_app_for_icon
                                    .write()
                                    .update_app_icon(&bundle_id, icon_path);
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

                    photoncast_app.write().update_or_add_app(app);

                    // Re-extract icon (might have changed)
                    cx.background_executor()
                        .spawn(async move {
                            // Clear cached icon first (force re-extraction)
                            Self::clear_cached_icon(&app_path);
                            if let Some(icon_path) = Self::get_app_icon_path_static(&app_path) {
                                photoncast_app_for_icon
                                    .write()
                                    .update_app_icon(&bundle_id, icon_path);
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
    pub(super) fn get_app_icon_path_static(app_path: &std::path::Path) -> Option<std::path::PathBuf> {
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
