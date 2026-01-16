//! Main launcher window component for PhotonCast.
//!
//! This module contains the `LauncherWindow` struct that implements
//! the GPUI `Render` trait for the main launcher UI.
//!
//! # Animations
//!
//! The launcher supports the following animations (all respecting reduce motion):
//! - Window appear: 150ms ease-out fade + scale (0.95 → 1.0)
//! - Window dismiss: 100ms ease-in fade + scale down
//! - Selection change: 80ms ease-in-out background transition
//! - Hover highlight: 60ms linear background transition

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::*;
use parking_lot::RwLock;

use crate::{
    Activate, Cancel, ConfirmDialog, CopyFile, CopyPath, NextGroup, OpenPreferences,
    PreviousGroup, QuickLook, QuickSelect1, QuickSelect2, QuickSelect3, QuickSelect4,
    QuickSelect5, QuickSelect6, QuickSelect7, QuickSelect8, QuickSelect9, RevealInFinder,
    SelectNext, SelectPrevious, ShowActionsMenu, LAUNCHER_BORDER_RADIUS, LAUNCHER_MAX_HEIGHT,
    LAUNCHER_MIN_HEIGHT,
};

use photoncast_core::app::integration::PhotonCastApp;
use photoncast_core::commands::{CommandExecutor, ConfirmationDialog, SystemCommand};
use photoncast_core::indexer::{AppScanner, AppWatcher, WatchEvent};
use photoncast_core::platform::launch::AppLauncher;
use photoncast_core::search::{
    IconSource, ResultType as CoreResultType, SearchAction, SearchResult,
};
use photoncast_core::storage::{Database, UsageTracker};
use photoncast_core::ui::animations::{
    ease_in, ease_in_out, ease_out, lerp, selection_change_duration, window_appear_duration,
    window_dismiss_duration, WindowAnimationState, WINDOW_APPEAR_OPACITY_END,
    WINDOW_APPEAR_OPACITY_START, WINDOW_APPEAR_SCALE_END, WINDOW_APPEAR_SCALE_START,
    WINDOW_DISMISS_SCALE_END,
};

use crate::platform::resize_window_height;

/// Search bar height constant
const SEARCH_BAR_HEIGHT: Pixels = px(48.0);
/// Search icon size
const SEARCH_ICON_SIZE: Pixels = px(20.0);
/// Result item height
const RESULT_ITEM_HEIGHT: Pixels = px(56.0);
/// Maximum visible results
const MAX_VISIBLE_RESULTS: usize = 8;

/// The main launcher window state
pub struct LauncherWindow {
    /// Current search query
    query: SharedString,
    /// Whether the window is visible
    visible: bool,
    /// Currently selected result index
    selected_index: usize,
    /// Previously selected index (for selection change animation)
    previous_selected_index: Option<usize>,
    /// Filtered results for current query
    results: Vec<ResultItem>,
    /// Core search results (for activation)
    core_results: Vec<SearchResult>,
    /// Focus handle for the search input
    focus_handle: FocusHandle,
    /// Window animation state
    animation_state: WindowAnimationState,
    /// Time when the current animation started
    animation_start: Option<Instant>,
    /// Index of the currently hovered result item (for hover animation)
    #[allow(dead_code)]
    hovered_index: Option<usize>,
    /// Time when selection changed (for selection animation)
    selection_animation_start: Option<Instant>,
    /// Hover animation starts per item (for smooth hover transitions)
    #[allow(dead_code)]
    hover_animation_starts: std::collections::HashMap<usize, Instant>,
    /// PhotonCast core app (shared across async operations)
    photoncast_app: Arc<RwLock<PhotonCastApp>>,
    /// App launcher for executing search actions
    app_launcher: Arc<AppLauncher>,
    /// Command executor for system commands
    command_executor: Arc<CommandExecutor>,
    /// Whether the app index has been initialized
    index_initialized: bool,
    /// Pending command awaiting confirmation (command and dialog info)
    pending_confirmation: Option<(SystemCommand, ConfirmationDialog)>,
    /// Current search mode (Normal or FileSearch)
    search_mode: SearchMode,
    /// File search state
    file_search_loading: bool,
    /// Last file search query (for debouncing)
    file_search_pending_query: Option<String>,
    /// File search debounce generation (incremented on each keystroke)
    file_search_generation: u64,
    /// Whether the actions menu (Cmd+K) is visible
    show_actions_menu: bool,
    /// Selected index in the actions menu (for keyboard navigation)
    actions_menu_index: usize,
}

/// A single result item for UI display
#[derive(Clone)]
pub struct ResultItem {
    #[allow(dead_code)]
    pub id: SharedString,
    pub title: SharedString,
    pub subtitle: SharedString,
    /// Emoji fallback icon
    pub icon_emoji: SharedString,
    /// Path to the app icon (.icns file) if available
    pub icon_path: Option<std::path::PathBuf>,
    pub result_type: ResultType,
}

/// Type of search result for grouping
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum ResultType {
    Application,
    Command,
    File,
    Folder,
}

impl ResultType {
    #[allow(dead_code)]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Application => "Apps",
            Self::Command => "Commands",
            Self::File => "Files",
            Self::Folder => "Folders",
        }
    }
}

impl From<CoreResultType> for ResultType {
    fn from(core_type: CoreResultType) -> Self {
        match core_type {
            CoreResultType::Application => Self::Application,
            CoreResultType::SystemCommand => Self::Command,
            CoreResultType::File => Self::File,
            CoreResultType::Folder => Self::Folder,
        }
    }
}

/// Search mode determines the UI state and behavior.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SearchMode {
    /// Normal search mode: Apps + Commands (default)
    #[default]
    Normal,
    /// File Search Mode: Spotlight-based file search
    FileSearch,
}

impl LauncherWindow {
    /// Creates a new launcher window
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Request focus immediately
        cx.focus(&focus_handle);

        // Initialize storage and usage tracking
        // Use in-memory database - fallback to no-op tracker if database fails
        let usage_tracker = match Database::open_in_memory() {
            Ok(db) => UsageTracker::new(db),
            Err(e) => {
                tracing::warn!("Failed to open database, usage tracking disabled: {}", e);
                // Create a minimal tracker with a fallback in-memory DB
                // If this also fails, we have bigger problems
                UsageTracker::new(Database::open_in_memory().unwrap_or_else(|_| {
                    panic!("Critical: Cannot initialize any database for usage tracking")
                }))
            }
        };

        // Create the PhotonCast core app with custom config
        // File search is disabled - apps and commands only for instant results (like Raycast)
        let config = photoncast_core::app::integration::IntegrationConfig {
            search_timeout_ms: 100, // Fast timeout since no file search
            include_files: false,   // Disable Spotlight file search (separate command later)
            ..Default::default()
        };
        let photoncast_app = Arc::new(RwLock::new(PhotonCastApp::with_config(config)));

        // Create the app launcher and command executor
        let app_launcher = Arc::new(AppLauncher::new(usage_tracker));
        let command_executor = Arc::new(CommandExecutor::new());

        let mut window = Self {
            query: SharedString::default(),
            visible: true,
            selected_index: 0,
            previous_selected_index: None,
            results: vec![],
            core_results: vec![],
            focus_handle,
            animation_state: WindowAnimationState::Hidden,
            animation_start: None,
            hovered_index: None,
            selection_animation_start: None,
            hover_animation_starts: std::collections::HashMap::new(),
            photoncast_app,
            app_launcher,
            command_executor,
            index_initialized: false,
            pending_confirmation: None,
            search_mode: SearchMode::Normal,
            file_search_loading: false,
            file_search_pending_query: None,
            file_search_generation: 0,
            show_actions_menu: false,
            actions_menu_index: 0,
        };

        // Start the appear animation
        window.start_appear_animation(cx);

        // Set initial window height
        window.update_window_height(cx);

        // Spawn async task to index applications
        window.start_app_indexing(cx);

        window
    }

    /// Starts async app indexing in the background
    fn start_app_indexing(&self, cx: &mut ViewContext<Self>) {
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
                    let _ = tx.send(Err(anyhow::anyhow!("Runtime creation failed: {}", e)));
                    return;
                }
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
                            // Start watching even if indexing failed
                            this.start_app_watching(cx);
                            cx.notify();
                        });
                        break;
                    },
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // Still waiting, continue polling
                        continue;
                    },
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        tracing::error!("Indexing thread disconnected");
                        let _ = this.update(&mut cx, |this, cx| {
                            this.index_initialized = true;
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
    fn start_app_watching(&self, cx: &mut ViewContext<Self>) {
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

    /// Handles a single watch event by updating the app index.
    async fn handle_watch_event(
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
                        if !this.query.is_empty() {
                            this.on_query_change(this.query.clone(), cx);
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
                        if !this.query.is_empty() {
                            this.on_query_change(this.query.clone(), cx);
                        }
                        cx.notify();
                    });
                } else {
                    tracing::warn!(path = %path.display(), "Failed to parse modified app metadata");
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
                        if !this.query.is_empty() {
                            this.on_query_change(this.query.clone(), cx);
                        }
                        cx.notify();
                    });
                }
            },
        }
    }

    /// Static version of get_app_icon_path for use in async context.
    fn get_app_icon_path_static(app_path: &std::path::Path) -> Option<std::path::PathBuf> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Get cache directory
        let cache_dir = directories::ProjectDirs::from("", "", "PhotonCast")
            .map(|dirs| dirs.cache_dir().join("icons"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("Library/Caches/PhotonCast/icons")
            });

        // Ensure cache directory exists
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!("Failed to create icon cache dir: {}", e);
            return None;
        }

        // Generate cache filename from app path hash
        let mut hasher = DefaultHasher::new();
        app_path.hash(&mut hasher);
        let hash = hasher.finish();
        let cached_path = cache_dir.join(format!("{hash:x}.png"));

        // Return cached icon if it exists and is fresh
        if cached_path.exists() {
            // Check if app is newer than cached icon
            let app_modified = std::fs::metadata(app_path)
                .ok()
                .and_then(|m| m.modified().ok());
            let cached_modified = std::fs::metadata(&cached_path)
                .ok()
                .and_then(|m| m.modified().ok());

            match (app_modified, cached_modified) {
                (Some(app_time), Some(cache_time)) if cache_time >= app_time => {
                    return Some(cached_path);
                },
                _ => {}, // Re-extract if we can't determine freshness
            }
        }

        // Extract icon using platform-specific code
        // Note: This requires access to the platform module which uses NSWorkspace
        // Since we're in a background task, we need to extract differently
        // For now, try to extract using iconutil or similar
        Self::extract_icon_to_cache(app_path, &cached_path)
    }

    /// Extracts an app icon to the cache path.
    fn extract_icon_to_cache(
        app_path: &std::path::Path,
        cache_path: &std::path::Path,
    ) -> Option<std::path::PathBuf> {
        // Try to find the icon in the app bundle
        let icns_path = app_path.join("Contents/Resources/AppIcon.icns");
        if !icns_path.exists() {
            // Try to read Info.plist to find the icon name
            let info_plist = app_path.join("Contents/Info.plist");
            if let Ok(plist) = plist::Value::from_file(&info_plist) {
                if let Some(dict) = plist.as_dictionary() {
                    if let Some(icon_name) = dict
                        .get("CFBundleIconFile")
                        .and_then(|v| v.as_string())
                    {
                        let icon_name = if icon_name.ends_with(".icns") {
                            icon_name.to_string()
                        } else {
                            format!("{}.icns", icon_name)
                        };
                        let icon_path = app_path.join("Contents/Resources").join(&icon_name);
                        if icon_path.exists() {
                            // Use sips to convert icns to png
                            let output = std::process::Command::new("sips")
                                .args([
                                    "-s", "format", "png",
                                    "-z", "64", "64",
                                    &icon_path.to_string_lossy(),
                                    "--out",
                                    &cache_path.to_string_lossy(),
                                ])
                                .output();

                            if let Ok(output) = output {
                                if output.status.success() {
                                    return Some(cache_path.to_path_buf());
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Use sips to convert icns to png
            let output = std::process::Command::new("sips")
                .args([
                    "-s", "format", "png",
                    "-z", "64", "64",
                    &icns_path.to_string_lossy(),
                    "--out",
                    &cache_path.to_string_lossy(),
                ])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    return Some(cache_path.to_path_buf());
                }
            }
        }

        None
    }

    /// Clears the cached icon for an app.
    fn clear_cached_icon(app_path: &std::path::Path) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let cache_dir = directories::ProjectDirs::from("", "", "PhotonCast")
            .map(|dirs| dirs.cache_dir().join("icons"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("Library/Caches/PhotonCast/icons")
            });

        let mut hasher = DefaultHasher::new();
        app_path.hash(&mut hasher);
        let hash = hasher.finish();
        let cached_path = cache_dir.join(format!("{hash:x}.png"));

        if cached_path.exists() {
            if let Err(e) = std::fs::remove_file(&cached_path) {
                tracing::warn!(path = %cached_path.display(), "Failed to remove cached icon: {}", e);
            } else {
                tracing::debug!(path = %cached_path.display(), "Cleared cached icon");
            }
        }
    }

    /// Checks if an icon is already cached, returns path if so.
    /// This is fast - just filesystem checks, no extraction.
    fn get_cached_icon_path(app_path: &std::path::Path) -> Option<std::path::PathBuf> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let cache_dir = directories::ProjectDirs::from("", "", "PhotonCast")
            .map(|dirs| dirs.cache_dir().join("icons"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("Library/Caches/PhotonCast/icons")
            });

        let mut hasher = DefaultHasher::new();
        app_path.hash(&mut hasher);
        let hash = hasher.finish();
        let cached_path = cache_dir.join(format!("{hash:x}.png"));

        if cached_path.exists() {
            Some(cached_path)
        } else {
            None
        }
    }

    /// Converts an icon source to a display emoji (fallback)
    fn icon_to_emoji(icon: &IconSource) -> SharedString {
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
                    _ => "📋".into(), // Default icon
                }
            },
            IconSource::AppIcon { .. } => "📱".into(),
            IconSource::FileIcon { path: _ } => "📄".into(),
        }
    }

    /// Gets or extracts the icon for an app bundle as PNG.
    /// Uses NSWorkspace to handle all icon formats including asset catalogs.
    fn get_app_icon_path(app_path: &std::path::Path) -> Option<std::path::PathBuf> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Get cache directory
        let cache_dir = directories::ProjectDirs::from("", "", "PhotonCast")
            .map(|dirs| dirs.cache_dir().join("icons"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("Library/Caches/PhotonCast/icons")
            });

        // Ensure cache directory exists
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!("Failed to create icon cache dir: {}", e);
            return None;
        }

        // Generate cache filename from app path hash
        let mut hasher = DefaultHasher::new();
        app_path.hash(&mut hasher);
        let hash = hasher.finish();
        let cached_path = cache_dir.join(format!("{hash:x}.png"));

        // Return cached icon if it exists and is fresh
        if cached_path.exists() {
            // Check if app is newer than cached icon
            let app_modified = std::fs::metadata(app_path)
                .ok()
                .and_then(|m| m.modified().ok());
            let cached_modified = std::fs::metadata(&cached_path)
                .ok()
                .and_then(|m| m.modified().ok());

            match (app_modified, cached_modified) {
                (Some(app_time), Some(cache_time)) if cache_time >= app_time => {
                    return Some(cached_path);
                },
                _ => {}, // Re-extract if we can't determine freshness
            }
        }

        // Extract icon using NSWorkspace (handles all icon formats)
        if crate::platform::save_app_icon_as_png(app_path, &cached_path, 64) {
            tracing::debug!(
                "Extracted icon for {} -> {}",
                app_path.display(),
                cached_path.display()
            );
            Some(cached_path)
        } else {
            tracing::warn!("Failed to extract icon for {}", app_path.display());
            None
        }
    }

    /// Converts a core SearchResult to a UI ResultItem.
    /// Icons are extracted during indexing, so we just check cache here.
    fn search_result_to_result_item(result: &SearchResult) -> ResultItem {
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

        ResultItem {
            id: SharedString::from(result.id.to_string()),
            title: result.title.clone().into(),
            subtitle: result.subtitle.clone().into(),
            icon_emoji: Self::icon_to_emoji(&result.icon),
            icon_path,
            result_type: result.result_type.into(),
        }
    }

    /// Starts the window appear animation.
    fn start_appear_animation(&mut self, cx: &mut ViewContext<Self>) {
        let duration = window_appear_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.animation_state = WindowAnimationState::Visible;
            self.animation_start = None;
        } else {
            self.animation_state = WindowAnimationState::Appearing;
            self.animation_start = Some(Instant::now());
            // Schedule a refresh to drive the animation
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16); // ~60 FPS
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if this.animation_state == WindowAnimationState::Appearing {
                                if let Some(start) = this.animation_start {
                                    let elapsed = start.elapsed();
                                    let total = window_appear_duration();
                                    if elapsed >= total {
                                        this.animation_state = WindowAnimationState::Visible;
                                        this.animation_start = None;
                                        cx.notify();
                                        return false; // Animation complete
                                    }
                                    cx.notify();
                                    return true; // Continue animation
                                }
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }
        cx.notify();
    }

    /// Starts the window dismiss animation.
    fn start_dismiss_animation(&mut self, cx: &mut ViewContext<Self>) {
        let duration = window_dismiss_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.animation_state = WindowAnimationState::Hidden;
            self.animation_start = None;
            // Close window but keep app running (for hotkey re-activation)
            let _ = cx.remove_window();
        } else {
            self.animation_state = WindowAnimationState::Dismissing;
            self.animation_start = Some(Instant::now());
            // Schedule a refresh to drive the animation
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16); // ~60 FPS
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if this.animation_state == WindowAnimationState::Dismissing {
                                if let Some(start) = this.animation_start {
                                    let elapsed = start.elapsed();
                                    let total = window_dismiss_duration();
                                    if elapsed >= total {
                                        this.animation_state = WindowAnimationState::Hidden;
                                        this.animation_start = None;
                                        // Close window but keep app running
                                        let _ = cx.remove_window();
                                        return false; // Animation complete
                                    }
                                    cx.notify();
                                    return true; // Continue animation
                                }
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }
        cx.notify();
    }

    /// Calculates the current animation progress (0.0 to 1.0).
    fn animation_progress(&self) -> f32 {
        match (self.animation_state, self.animation_start) {
            (WindowAnimationState::Appearing, Some(start)) => {
                let elapsed = start.elapsed();
                let total = window_appear_duration();
                if total.is_zero() {
                    1.0
                } else {
                    (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0)
                }
            },
            (WindowAnimationState::Dismissing, Some(start)) => {
                let elapsed = start.elapsed();
                let total = window_dismiss_duration();
                if total.is_zero() {
                    1.0
                } else {
                    (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0)
                }
            },
            (WindowAnimationState::Visible, _) => 1.0,
            (WindowAnimationState::Hidden, _) => 0.0,
            _ => 1.0,
        }
    }

    /// Calculates the current opacity based on animation state.
    fn current_opacity(&self) -> f32 {
        match self.animation_state {
            WindowAnimationState::Appearing => {
                let progress = ease_out(self.animation_progress());
                lerp(
                    WINDOW_APPEAR_OPACITY_START,
                    WINDOW_APPEAR_OPACITY_END,
                    progress,
                )
            },
            WindowAnimationState::Dismissing => {
                let progress = ease_in(self.animation_progress());
                lerp(
                    WINDOW_APPEAR_OPACITY_END,
                    WINDOW_APPEAR_OPACITY_START,
                    progress,
                )
            },
            WindowAnimationState::Visible => 1.0,
            WindowAnimationState::Hidden => 0.0,
        }
    }

    /// Calculates the current scale based on animation state.
    #[allow(dead_code)]
    fn current_scale(&self) -> f32 {
        match self.animation_state {
            WindowAnimationState::Appearing => {
                let progress = ease_out(self.animation_progress());
                lerp(WINDOW_APPEAR_SCALE_START, WINDOW_APPEAR_SCALE_END, progress)
            },
            WindowAnimationState::Dismissing => {
                let progress = ease_in(self.animation_progress());
                lerp(WINDOW_APPEAR_SCALE_END, WINDOW_DISMISS_SCALE_END, progress)
            },
            WindowAnimationState::Visible => 1.0,
            WindowAnimationState::Hidden => WINDOW_APPEAR_SCALE_START,
        }
    }

    /// Toggle the visibility of the launcher window
    #[allow(dead_code)]
    pub fn toggle(&mut self, cx: &mut ViewContext<Self>) {
        self.visible = !self.visible;
        if self.visible {
            self.query = SharedString::default();
            self.selected_index = 0;
            self.previous_selected_index = None;
            cx.focus(&self.focus_handle);
            self.start_appear_animation(cx);
        } else {
            self.start_dismiss_animation(cx);
        }
    }

    /// Shows the launcher window with animation
    #[allow(dead_code)]
    pub fn show(&mut self, cx: &mut ViewContext<Self>) {
        self.visible = true;
        self.query = SharedString::default();
        self.selected_index = 0;
        self.previous_selected_index = None;
        cx.focus(&self.focus_handle);
        self.start_appear_animation(cx);
    }

    /// Hides the launcher window with animation
    pub fn hide(&mut self, cx: &mut ViewContext<Self>) {
        self.visible = false;
        self.start_dismiss_animation(cx);
    }

    /// Handle query change from search input
    fn on_query_change(&mut self, _query: SharedString, cx: &mut ViewContext<Self>) {
        self.selected_index = 0;

        // Perform search using the core library
        if self.query.is_empty() {
            self.results.clear();
            self.core_results.clear();
            // Close actions menu when results are cleared
            self.show_actions_menu = false;
        } else {
            match self.search_mode {
                SearchMode::Normal => {
                    // Normal mode: search apps and commands using PhotonCastApp
                    let outcome = self.photoncast_app.read().search(&self.query);

                    // Collect all results from the search outcome
                    let all_results: Vec<SearchResult> = outcome.results.iter().cloned().collect();

                    // Convert core results to UI results, limited to max visible
                    self.core_results = all_results
                        .iter()
                        .take(MAX_VISIBLE_RESULTS)
                        .cloned()
                        .collect();

                    // Convert to UI results
                    self.results = self
                        .core_results
                        .iter()
                        .map(|r| Self::search_result_to_result_item(r))
                        .collect();

                    // Log timeout warning if applicable
                    if outcome.timed_out {
                        if let Some(msg) = outcome.message {
                            tracing::warn!("Search warning: {}", msg);
                        }
                    }
                }
                SearchMode::FileSearch => {
                    // File Search Mode: debounced async Spotlight search
                    self.schedule_file_search(cx);
                }
            }
        }

        // Update window height based on results
        self.update_window_height(cx);
        cx.notify();
    }

    /// Schedules a debounced file search.
    /// Only triggers the actual search after 150ms of no typing.
    fn schedule_file_search(&mut self, cx: &mut ViewContext<Self>) {
        let query = self.query.to_string();

        // Require at least 2 characters before searching
        if query.len() < 2 {
            self.results.clear();
            self.core_results.clear();
            self.file_search_loading = false;
            return;
        }

        // Increment generation to invalidate previous searches
        self.file_search_generation += 1;
        let generation = self.file_search_generation;

        // Show loading state
        self.file_search_loading = true;
        self.file_search_pending_query = Some(query.clone());

        // Spawn debounced async search
        cx.spawn(|this, mut cx| async move {
            // Debounce: wait 150ms before searching
            cx.background_executor()
                .timer(Duration::from_millis(150))
                .await;

            // Check if this search is still valid (no newer keystrokes)
            let should_search = this
                .update(&mut cx, |view, _| {
                    view.file_search_generation == generation
                })
                .unwrap_or(false);

            if !should_search {
                return; // A newer search was scheduled, abort this one
            }

            // Execute the actual search in background
            let results = cx
                .background_executor()
                .spawn(async move {
                    // Use mdfind with -onlyin for faster results
                    // Get more results than needed so we can sort by date
                    let output = std::process::Command::new("mdfind")
                        .arg("-name")
                        .arg(&query)
                        .arg("-onlyin")
                        .arg(dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")))
                        .output();

                    match output {
                        Ok(output) if output.status.success() => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            
                            // Collect paths with modification times for sorting
                            let mut files_with_time: Vec<(std::path::PathBuf, std::time::SystemTime)> = stdout
                                .lines()
                                .filter(|line| !line.is_empty())
                                .take(50) // Get more to sort from
                                .filter_map(|line| {
                                    let path = std::path::PathBuf::from(line);
                                    let mtime = std::fs::metadata(&path)
                                        .ok()
                                        .and_then(|m| m.modified().ok())
                                        .unwrap_or(std::time::UNIX_EPOCH);
                                    Some((path, mtime))
                                })
                                .collect();
                            
                            // Sort by modification time (newest first)
                            files_with_time.sort_by(|a, b| b.1.cmp(&a.1));
                            
                            // Take top results and convert to SearchResult
                            files_with_time
                                .into_iter()
                                .take(MAX_VISIBLE_RESULTS)
                                .map(|(path, _mtime)| {
                                    let name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown")
                                        .to_string();
                                    let subtitle = path
                                        .parent()
                                        .map(|p| p.display().to_string())
                                        .unwrap_or_default();
                                    let is_dir = path.is_dir();
                                    let is_app = path.extension().is_some_and(|e| e == "app");

                                    let result_type = if is_app {
                                        CoreResultType::Application
                                    } else if is_dir {
                                        CoreResultType::Folder
                                    } else {
                                        CoreResultType::File
                                    };

                                    SearchResult {
                                        id: photoncast_core::search::SearchResultId::new(
                                            format!("file:{}", path.display()),
                                        ),
                                        title: name,
                                        subtitle,
                                        icon: IconSource::FileIcon { path: path.clone() },
                                        result_type,
                                        score: 0.0,
                                        match_indices: vec![],
                                        action: SearchAction::OpenFile { path },
                                    }
                                })
                                .collect::<Vec<_>>()
                        }
                        _ => vec![],
                    }
                })
                .await;

            // Update UI with results (if this search is still valid)
            let _ = this.update(&mut cx, |view, cx| {
                if view.file_search_generation == generation {
                    view.file_search_loading = false;
                    view.core_results = results;
                    view.results = view
                        .core_results
                        .iter()
                        .map(|r| Self::search_result_to_result_item(r))
                        .collect();
                    view.update_window_height(cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Update window height based on result count
    fn update_window_height(&self, cx: &mut ViewContext<Self>) {
        let result_count = self.results.len().min(MAX_VISIBLE_RESULTS);
        let query_empty = self.query.is_empty();

        // Count unique result types for group headers
        let group_count = if result_count > 0 {
            self.results
                .iter()
                .map(|r| r.result_type)
                .collect::<std::collections::HashSet<_>>()
                .len()
        } else {
            0
        };
        let group_header_height = 24.0;
        let action_bar_height = 32.0; // Action bar with ⌘K hint (only when results)
        let bottom_padding = 8.0; // Extra padding at bottom

        // Calculate content height (action bar only visible with results)
        let content_height = if result_count > 0 {
            // Search bar + divider + group headers + results + action bar + padding
            SEARCH_BAR_HEIGHT.0
                + 1.0
                + (group_count as f32 * group_header_height)
                + (result_count as f32 * RESULT_ITEM_HEIGHT.0)
                + action_bar_height
                + bottom_padding
        } else if !query_empty {
            // Search bar + no results message (no action bar)
            SEARCH_BAR_HEIGHT.0 + 60.0
        } else {
            // Search bar + empty state (no action bar)
            SEARCH_BAR_HEIGHT.0 + 60.0
        };

        let new_height = content_height.clamp(LAUNCHER_MIN_HEIGHT.0, LAUNCHER_MAX_HEIGHT.0);

        // Spawn async task to resize after current frame completes
        cx.spawn(|_, _| async move {
            // Small delay to ensure we're outside GPUI's borrow
            gpui::Timer::after(Duration::from_millis(1)).await;
            resize_window_height(new_height as f64);
        })
        .detach();
    }

    // Action handlers

    /// Starts the selection change animation.
    fn start_selection_animation(&mut self, previous_index: usize, cx: &mut ViewContext<Self>) {
        self.previous_selected_index = Some(previous_index);
        let duration = selection_change_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.selection_animation_start = None;
            self.previous_selected_index = None;
        } else {
            self.selection_animation_start = Some(Instant::now());
            // Schedule animation updates
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16);
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if let Some(start) = this.selection_animation_start {
                                let elapsed = start.elapsed();
                                let total = selection_change_duration();
                                if elapsed >= total {
                                    this.selection_animation_start = None;
                                    this.previous_selected_index = None;
                                    cx.notify();
                                    return false;
                                }
                                cx.notify();
                                return true;
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }
    }

    /// Calculates the selection animation progress (0.0 to 1.0).
    #[allow(dead_code)]
    fn selection_animation_progress(&self) -> f32 {
        if let Some(start) = self.selection_animation_start {
            let elapsed = start.elapsed();
            let total = selection_change_duration();
            if total.is_zero() {
                1.0
            } else {
                ease_in_out((elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0))
            }
        } else {
            1.0
        }
    }

    fn select_next(&mut self, _: &SelectNext, cx: &mut ViewContext<Self>) {
        // If actions menu is open, navigate within it
        if self.show_actions_menu {
            let action_count = self.get_actions_count();
            if action_count > 0 {
                self.actions_menu_index = (self.actions_menu_index + 1).min(action_count - 1);
                cx.notify();
            }
            return;
        }

        if !self.results.is_empty() {
            let previous = self.selected_index;
            let new_index = (self.selected_index + 1).min(self.results.len() - 1);
            if new_index != previous {
                self.selected_index = new_index;
                self.start_selection_animation(previous, cx);
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        }
    }

    fn select_previous(&mut self, _: &SelectPrevious, cx: &mut ViewContext<Self>) {
        // If actions menu is open, navigate within it
        if self.show_actions_menu {
            if self.actions_menu_index > 0 {
                self.actions_menu_index -= 1;
                cx.notify();
            }
            return;
        }

        if self.selected_index > 0 {
            let previous = self.selected_index;
            self.selected_index -= 1;
            self.start_selection_animation(previous, cx);
            self.ensure_selected_visible(cx);
            cx.notify();
        }
    }

    fn activate(&mut self, _: &Activate, cx: &mut ViewContext<Self>) {
        // If actions menu is open, execute the selected action
        if self.show_actions_menu {
            self.execute_selected_action(cx);
            return;
        }

        // If confirmation dialog is showing, this means user pressed Enter to confirm
        if self.pending_confirmation.is_some() {
            self.confirm_pending_command(cx);
            return;
        }

        if let Some(core_result) = self.core_results.get(self.selected_index).cloned() {
            let title = core_result.title.clone();

            // Handle the action based on its type
            match &core_result.action {
                SearchAction::LaunchApp { .. }
                | SearchAction::OpenFile { .. }
                | SearchAction::RevealInFinder { .. } => {
                    // Use AppLauncher for app/file actions
                    let launcher = Arc::clone(&self.app_launcher);
                    let action = core_result.action.clone();

                    // Execute the action
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
                SearchAction::EnterFileSearchMode => {
                    // Enter File Search Mode
                    tracing::info!("Entering File Search Mode");
                    self.enter_file_search_mode(cx);
                },
                SearchAction::QuickLookFile { path } => {
                    // Quick Look is handled separately
                    tracing::info!(path = %path.display(), "Quick Look not yet implemented");
                },
            }
        }
    }

    /// Confirms and executes the pending command
    fn confirm_pending_command(&mut self, cx: &mut ViewContext<Self>) {
        if let Some((cmd, _dialog)) = self.pending_confirmation.take() {
            let executor = Arc::clone(&self.command_executor);
            let cmd_name = cmd.name();

            match executor.execute(cmd) {
                Ok(()) => {
                    tracing::info!("Executed confirmed command: {}", cmd_name);
                },
                Err(e) => {
                    tracing::error!("Failed to execute {}: {}", cmd_name, e);
                },
            }
            self.hide(cx);
        }
    }

    /// Handles the ConfirmDialog action (Enter key in confirmation dialog)
    fn confirm_dialog(&mut self, _: &ConfirmDialog, cx: &mut ViewContext<Self>) {
        if self.pending_confirmation.is_some() {
            self.confirm_pending_command(cx);
        }
    }

    /// Cancels the pending confirmation dialog and returns to search
    fn cancel_confirmation(&mut self, cx: &mut ViewContext<Self>) {
        if self.pending_confirmation.is_some() {
            self.pending_confirmation = None;
            cx.notify();
        }
    }

    fn cancel(&mut self, _: &Cancel, cx: &mut ViewContext<Self>) {
        // If actions menu is showing, close it first
        if self.show_actions_menu {
            self.show_actions_menu = false;
            cx.notify();
            return;
        }

        // If confirmation dialog is showing, cancel it first
        if self.pending_confirmation.is_some() {
            self.cancel_confirmation(cx);
            return;
        }

        // If in file search mode, exit back to normal mode
        if self.search_mode == SearchMode::FileSearch {
            self.exit_file_search_mode(cx);
            return;
        }

        if !self.query.is_empty() {
            // Clear query first
            self.query = SharedString::default();
            self.results.clear();
            self.selected_index = 0;
            self.update_window_height(cx);
            cx.notify();
        } else {
            // Close window with animation (hide() calls start_dismiss_animation which quits)
            self.hide(cx);
        }
    }

    fn quick_select(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        if index < self.results.len() {
            self.selected_index = index;
            self.activate(&Activate, cx);
        }
    }

    // ========================================================================
    // File Search Mode Methods
    // ========================================================================

    /// Enters File Search Mode.
    ///
    /// This changes the launcher UI to show file search functionality:
    /// - Different placeholder text ("Search files...")
    /// - File search results instead of apps/commands
    /// - Different keyboard shortcuts (Cmd+Enter = Reveal, Cmd+Y = Quick Look)
    fn enter_file_search_mode(&mut self, cx: &mut ViewContext<Self>) {
        tracing::info!("Entering File Search Mode");
        self.search_mode = SearchMode::FileSearch;
        self.query = SharedString::default();
        self.results.clear();
        self.core_results.clear();
        self.selected_index = 0;
        self.file_search_loading = false;
        self.file_search_pending_query = None;
        self.file_search_generation += 1; // Invalidate pending searches
        self.update_window_height(cx);
        cx.notify();
    }

    /// Exits File Search Mode and returns to normal search.
    fn exit_file_search_mode(&mut self, cx: &mut ViewContext<Self>) {
        tracing::info!("Exiting File Search Mode");
        self.search_mode = SearchMode::Normal;
        self.query = SharedString::default();
        self.results.clear();
        self.core_results.clear();
        self.selected_index = 0;
        self.file_search_loading = false;
        self.file_search_pending_query = None;
        self.file_search_generation += 1; // Invalidate pending searches
        self.update_window_height(cx);
        cx.notify();
    }

    /// Handles the Reveal in Finder action (Cmd+Enter).
    fn reveal_in_finder(&mut self, _: &RevealInFinder, cx: &mut ViewContext<Self>) {
        // Only active in file search mode with a selected file result
        if self.search_mode != SearchMode::FileSearch {
            return;
        }

        if let Some(core_result) = self.core_results.get(self.selected_index).cloned() {
            match &core_result.action {
                SearchAction::OpenFile { path } | SearchAction::RevealInFinder { path } => {
                    // Open Finder and reveal the file
                    let reveal_action = SearchAction::RevealInFinder { path: path.clone() };
                    let launcher = Arc::clone(&self.app_launcher);
                    match launcher.execute_action(&reveal_action) {
                        Ok(()) => {
                            tracing::info!("Revealed in Finder: {}", path.display());
                        }
                        Err(e) => {
                            tracing::error!("Failed to reveal {}: {}", path.display(), e.user_message());
                        }
                    }
                    self.hide(cx);
                }
                _ => {
                    tracing::debug!("Reveal in Finder: not a file result");
                }
            }
        }
    }

    /// Handles the Quick Look action (Cmd+Y).
    fn quick_look(&mut self, _: &QuickLook, cx: &mut ViewContext<Self>) {
        // Only active in file search mode with a selected file result
        if self.search_mode != SearchMode::FileSearch {
            return;
        }

        if let Some(core_result) = self.core_results.get(self.selected_index).cloned() {
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
                }
                _ => {
                    tracing::debug!("Quick Look: not a file result");
                }
            }
        }
    }

    /// Handles the Copy Path action (Cmd+C).
    fn copy_path(&mut self, _: &CopyPath, cx: &mut ViewContext<Self>) {
        if let Some(core_result) = self.core_results.get(self.selected_index).cloned() {
            let path_str = match &core_result.action {
                SearchAction::OpenFile { path } | SearchAction::RevealInFinder { path } => {
                    Some(path.display().to_string())
                }
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
        self.show_actions_menu = false;
        cx.notify();
    }

    /// Handles the Copy File action (Cmd+Shift+C).
    /// Copies the actual file to clipboard so it can be pasted in apps like Slack, WhatsApp, etc.
    fn copy_file(&mut self, _: &CopyFile, cx: &mut ViewContext<Self>) {
        if let Some(core_result) = self.core_results.get(self.selected_index).cloned() {
            let path = match &core_result.action {
                SearchAction::OpenFile { path } | SearchAction::RevealInFinder { path } => {
                    Some(path.clone())
                }
                SearchAction::LaunchApp { path, .. } => Some(path.clone()),
                _ => None,
            };

            if let Some(path) = path {
                // Use osascript to copy file to clipboard (works for pasting in apps)
                // SECURITY: Escape backslashes and double quotes to prevent AppleScript injection
                let escaped_path = escape_path_for_applescript(&path.display().to_string());
                let script = format!(
                    r#"set the clipboard to (POSIX file "{}")"#,
                    escaped_path
                );
                
                match std::process::Command::new("osascript")
                    .args(["-e", &script])
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        tracing::info!("Copied file to clipboard: {}", path.display());
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::error!("Failed to copy file: {}", stderr);
                    }
                    Err(e) => {
                        tracing::error!("Failed to run osascript: {}", e);
                    }
                }
            }
        }
        // Close menu if open
        self.show_actions_menu = false;
        cx.notify();
    }

    /// Handles the Show Actions Menu action (Cmd+K).
    fn show_actions_menu(&mut self, _: &ShowActionsMenu, cx: &mut ViewContext<Self>) {
        // Only allow opening actions menu when there's something selected
        if self.results.is_empty() {
            return;
        }
        
        // Toggle actions menu
        self.show_actions_menu = !self.show_actions_menu;
        self.actions_menu_index = 0; // Reset selection when opening
        cx.notify();
    }

    /// Returns the number of actions available in the current context.
    fn get_actions_count(&self) -> usize {
        let is_file_mode = self.search_mode == SearchMode::FileSearch;
        let has_selection = !self.results.is_empty();
        
        if !has_selection {
            return 0;
        }
        
        // Base actions: Open, Copy Path, Copy File
        let mut count = 3;
        
        // File mode adds: Reveal in Finder, Quick Look
        if is_file_mode {
            count += 2;
        }
        
        count
    }

    /// Executes the action at the current actions_menu_index.
    fn execute_selected_action(&mut self, cx: &mut ViewContext<Self>) {
        let is_file_mode = self.search_mode == SearchMode::FileSearch;
        let has_selection = !self.results.is_empty();
        
        if !has_selection {
            self.show_actions_menu = false;
            cx.notify();
            return;
        }

        // Map index to action based on current mode
        // Order: Open, Copy Path, Copy File, [Reveal in Finder, Quick Look]
        match self.actions_menu_index {
            0 => {
                // Open
                self.show_actions_menu = false;
                self.activate(&Activate, cx);
            }
            1 => {
                // Copy Path
                self.copy_path(&CopyPath, cx);
            }
            2 => {
                // Copy File
                self.copy_file(&CopyFile, cx);
            }
            3 if is_file_mode => {
                // Reveal in Finder
                self.reveal_in_finder(&RevealInFinder, cx);
            }
            4 if is_file_mode => {
                // Quick Look
                self.quick_look(&QuickLook, cx);
            }
            _ => {
                self.show_actions_menu = false;
                cx.notify();
            }
        }
    }

    fn next_group(&mut self, _: &NextGroup, cx: &mut ViewContext<Self>) {
        if self.results.is_empty() {
            return;
        }

        // Find current group
        let current_type = self.results.get(self.selected_index).map(|r| r.result_type);

        if let Some(current_type) = current_type {
            // Find the first item of the next group
            let mut found_current = false;
            for (idx, result) in self.results.iter().enumerate() {
                if !found_current && result.result_type == current_type {
                    found_current = true;
                }
                if found_current && result.result_type != current_type {
                    self.selected_index = idx;
                    self.ensure_selected_visible(cx);
                    cx.notify();
                    return;
                }
            }

            // No next group found, wrap to first item
            self.selected_index = 0;
            self.ensure_selected_visible(cx);
        }
        cx.notify();
    }

    fn previous_group(&mut self, _: &PreviousGroup, cx: &mut ViewContext<Self>) {
        if self.results.is_empty() {
            return;
        }

        // Find current group
        let current_type = self.results.get(self.selected_index).map(|r| r.result_type);

        if let Some(current_type) = current_type {
            // Find the first item of current group
            let current_group_start = self
                .results
                .iter()
                .position(|r| r.result_type == current_type)
                .unwrap_or(0);

            if current_group_start > 0 {
                // Find the previous group's first item
                let prev_type = self.results[current_group_start - 1].result_type;
                let prev_group_start = self
                    .results
                    .iter()
                    .position(|r| r.result_type == prev_type)
                    .unwrap_or(0);
                self.selected_index = prev_group_start;
            } else {
                // Already at first group, wrap to last group's first item
                let last_type = self.results.last().map(|r| r.result_type);
                if let Some(last_type) = last_type {
                    let last_group_start = self
                        .results
                        .iter()
                        .position(|r| r.result_type == last_type)
                        .unwrap_or(0);
                    self.selected_index = last_group_start;
                }
            }
            self.ensure_selected_visible(cx);
        }
        cx.notify();
    }

    /// Ensures the selected item is visible by scrolling if needed.
    fn ensure_selected_visible(&self, _cx: &mut ViewContext<Self>) {
        // The GPUI scroll container handles this automatically when using
        // overflow_y_scroll(). The visible area is managed by the framework.
        // For more control, we would need to track scroll offset manually
        // and use ScrollHandle. The current implementation with automatic
        // scrolling and relatively small result lists is sufficient for MVP.
    }

    fn open_preferences(&mut self, _: &OpenPreferences, cx: &mut ViewContext<Self>) {
        // TODO: Open preferences window
        tracing::info!("Opening preferences...");
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        // Handle backspace
        if event.keystroke.key == "backspace" {
            if !self.query.is_empty() {
                let mut chars: Vec<char> = self.query.chars().collect();
                chars.pop();
                self.query = SharedString::from(chars.into_iter().collect::<String>());
                self.on_query_change(self.query.clone(), cx);
                cx.notify();
            }
            return;
        }

        // Ignore modifier-only keys and special keys handled by actions
        if event.keystroke.modifiers.platform
            || event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
        {
            return;
        }

        // Handle regular character input
        if let Some(ime_key) = &event.keystroke.ime_key {
            let new_query = format!("{}{}", self.query, ime_key);
            self.query = SharedString::from(new_query);
            self.on_query_change(self.query.clone(), cx);
            cx.notify();
        } else if event.keystroke.key.len() == 1 {
            // Single character key (a-z, 0-9, etc.)
            let key = if event.keystroke.modifiers.shift {
                event.keystroke.key.to_uppercase()
            } else {
                event.keystroke.key.clone()
            };
            let new_query = format!("{}{}", self.query, key);
            self.query = SharedString::from(new_query);
            self.on_query_change(self.query.clone(), cx);
            cx.notify();
        }
    }

    /// Render the search bar component
    fn render_search_bar(&self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Determine icon and placeholder based on search mode
        let (icon, placeholder) = match self.search_mode {
            SearchMode::Normal => ("🔍", "Search PhotonCast..."),
            SearchMode::FileSearch => ("📁", "Search files..."),
        };

        div()
            .h(SEARCH_BAR_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .child(
                // Search icon
                div()
                    .size(SEARCH_ICON_SIZE)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x888888))
                    .child(icon),
            )
            .child(
                // Search input
                div().flex_1().h_full().flex().items_center().child(
                    div()
                        .w_full()
                        .text_size(px(16.0))
                        .text_color(rgb(0xffffff))
                        .when(self.query.is_empty(), |el| {
                            el.text_color(rgb(0x888888)).child(placeholder)
                        })
                        .when(!self.query.is_empty(), |el| el.child(self.query.clone())),
                ),
            )
            // Show "esc to exit" hint in file search mode
            .when(self.search_mode == SearchMode::FileSearch, |el| {
                el.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(0x666666))
                        .child("esc to exit"),
                )
            })
    }

    /// Render the results list component with grouping
    fn render_results(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Group results by type
        let mut current_type: Option<ResultType> = None;
        let mut elements: Vec<gpui::AnyElement> = Vec::new();

        for (idx, result) in self.results.iter().enumerate() {
            // Add group header when type changes
            if current_type != Some(result.result_type) {
                current_type = Some(result.result_type);
                elements.push(
                    self.render_group_header(result.result_type)
                        .into_any_element(),
                );
            }

            let is_selected = idx == self.selected_index;
            elements.push(
                self.render_result_item(result, idx, is_selected, cx)
                    .into_any_element(),
            );
        }

        // Calculate height: items + group headers (24px each)
        let result_count = self.results.len().min(MAX_VISIBLE_RESULTS);
        let group_count = self
            .results
            .iter()
            .map(|r| r.result_type)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let group_header_height = 24.0;
        let total_height = (result_count as f32 * RESULT_ITEM_HEIGHT.0)
            + (group_count as f32 * group_header_height);

        div()
            .id("results-list")
            .w_full()
            .h(px(total_height))
            .overflow_y_scroll()
            .children(elements)
    }

    /// Render a group header (e.g., "Apps", "Commands")
    fn render_group_header(&self, result_type: ResultType) -> impl IntoElement {
        div()
            .h(px(24.0))
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x888888))
                    .child(result_type.display_name().to_uppercase()),
            )
    }

    /// Render the icon for a result item
    fn render_icon(&self, result: &ResultItem) -> impl IntoElement {
        let icon_size = px(32.0);

        div()
            .size(icon_size)
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .rounded(px(6.0))
            .map(|el| {
                if let Some(icon_path) = &result.icon_path {
                    // Use the actual app icon - pass PathBuf for ImageSource::File
                    el.child(
                        img(icon_path.clone())
                            .size(icon_size)
                            .object_fit(ObjectFit::Contain),
                    )
                } else {
                    // Fall back to emoji
                    el.text_size(px(24.0)).child(result.icon_emoji.clone())
                }
            })
    }

    /// Render a single result item
    fn render_result_item(
        &self,
        result: &ResultItem,
        index: usize,
        is_selected: bool,
        _cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let selected_bg = hsla(0.0, 0.0, 1.0, 0.1);
        let hover_bg = hsla(0.0, 0.0, 1.0, 0.05);
        div()
            .id(("result-item", index))
            .h(RESULT_ITEM_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .when(is_selected, |el| el.bg(selected_bg))
            .hover(|el| el.bg(hover_bg))
            .cursor_pointer()
            .child(self.render_icon(result))
            .child(
                // Title and subtitle
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0xffffff))
                            .truncate()
                            .child(result.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0x888888))
                            .truncate()
                            .child(result.subtitle.clone()),
                    ),
            )
            .child(
                // Shortcut badge
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x666666))
                    .child(format!("⌘{}", index + 1)),
            )
    }

    /// Render empty state when there's no query
    fn render_empty_state(&self) -> AnyElement {
        // Show loading indicator during file search
        if self.file_search_loading && self.search_mode == SearchMode::FileSearch {
            return div()
                .w_full()
                .py_4()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(rgb(0x888888))
                        .child("Searching..."),
                )
                .into_any_element();
        }

        let (message, hints) = match self.search_mode {
            SearchMode::Normal => (
                "Type to search apps and commands",
                "↑↓ Navigate  ↵ Open  esc Close",
            ),
            SearchMode::FileSearch => (
                "Type at least 2 characters to search files",
                "↵ Open  ⌘↵ Reveal  ⌘Y Quick Look  esc Exit",
            ),
        };

        div()
            .w_full()
            .py_4()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x888888))
                    .child(message),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x666666))
                    .child(hints),
            )
            .into_any_element()
    }

    /// Render "no results" state
    fn render_no_results(&self) -> impl IntoElement {
        div()
            .w_full()
            .py_4()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x888888))
                    .child(format!("No results for \"{}\"", self.query)),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x666666))
                    .child("Try a different search term"),
            )
    }

    /// Render the confirmation dialog overlay
    fn render_confirmation_dialog(
        &self,
        dialog: &ConfirmationDialog,
        _cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        // Full overlay with semi-transparent background
        div()
            .id("confirmation-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            // Semi-transparent dark overlay
            .bg(hsla(0.0, 0.0, 0.0, 0.6))
            .child(
                // Dialog container
                div()
                    .id("confirmation-dialog")
                    .w(px(340.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    // Catppuccin surface1 background with blur effect
                    .bg(hsla(232.0 / 360.0, 0.13, 0.23, 0.98))
                    .rounded(px(12.0))
                    .border_1()
                    // Catppuccin surface2 border
                    .border_color(hsla(233.0 / 360.0, 0.13, 0.29, 0.8))
                    .shadow_xl()
                    // Warning icon and title
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            // Warning icon
                            .child(
                                div()
                                    .size(px(48.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_full()
                                    // Catppuccin red (destructive) background with low opacity
                                    .bg(hsla(343.0 / 360.0, 0.81, 0.75, 0.15))
                                    .child(
                                        div()
                                            .text_size(px(24.0))
                                            .child("⚠️"),
                                    ),
                            )
                            // Title
                            .child(
                                div()
                                    .text_size(px(16.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .child(dialog.title.clone()),
                            ),
                    )
                    // Message
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(rgb(0xa6adc8)) // Catppuccin subtext0
                                    .child(dialog.message.clone()),
                            ),
                    )
                    // Buttons
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .mt_2()
                            // Cancel button
                            .child(
                                div()
                                    .id("cancel-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    // Catppuccin surface2 background
                                    .bg(hsla(233.0 / 360.0, 0.13, 0.29, 1.0))
                                    .hover(|el| el.bg(hsla(232.0 / 360.0, 0.13, 0.33, 1.0)))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(0xcdd6f4)) // Catppuccin text
                                            .child(dialog.cancel_label.clone()),
                                    ),
                            )
                            // Confirm button (destructive style)
                            .child(
                                div()
                                    .id("confirm-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    // Catppuccin red for destructive actions
                                    .bg(hsla(343.0 / 360.0, 0.81, 0.55, 1.0))
                                    .hover(|el| el.bg(hsla(343.0 / 360.0, 0.81, 0.65, 1.0)))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0xffffff))
                                            .child(dialog.confirm_label.clone()),
                                    ),
                            ),
                    )
                    // Keyboard hints
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_center()
                            .mt_1()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(0x6c7086)) // Catppuccin overlay0
                                    .child("↵ Confirm  esc Cancel"),
                            ),
                    ),
            )
    }

    /// Render the action bar at the bottom (shows ⌘K hint)
    fn render_action_bar(&self) -> impl IntoElement {
        div()
            .w_full()
            .h(px(32.0))
            .px_4()
            .flex()
            .items_center()
            .justify_end()
            .border_t_1()
            .border_color(rgb(0x313244))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(0x6c7086))
                            .child("Actions"),
                    )
                    .child(
                        div()
                            .px_1()
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(rgb(0x313244))
                            .text_size(px(10.0))
                            .text_color(rgb(0x888888))
                            .child("⌘K"),
                    ),
            )
    }

    /// Render the actions menu popup (Cmd+K)
    fn render_actions_menu(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Determine available actions based on search mode and selection
        let is_file_mode = self.search_mode == SearchMode::FileSearch;
        let has_selection = !self.results.is_empty();
        let selected = self.actions_menu_index;

        div()
            // Overlay background - position menu at bottom-right
            .absolute()
            .inset_0()
            .flex()
            .items_end()
            .justify_end()
            .pb_2()
            .pr_2()
            // Click outside to close
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, cx| {
                this.show_actions_menu = false;
                cx.notify();
            }))
            .child(
                div()
                    .w(px(300.0))
                    .bg(hsla(240.0 / 360.0, 0.21, 0.18, 0.98))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(rgb(0x45475a))
                    .shadow_lg()
                    .overflow_hidden()
                    // Stop propagation so clicking menu doesn't close it
                    .on_mouse_down(MouseButton::Left, |_, cx| cx.stop_propagation())
                    // Header
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .border_b_1()
                            .border_color(rgb(0x313244))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xcdd6f4))
                            .child("Actions"),
                    )
                    // Action items with selection highlighting
                    .child(
                        div()
                            .py_1()
                            .child(self.render_action_item("Open", "↵", has_selection, selected == 0))
                            .child(self.render_action_item("Copy Path", "⌘C", has_selection, selected == 1))
                            .child(self.render_action_item("Copy File", "⇧⌘C", has_selection, selected == 2))
                            .when(is_file_mode, |el| {
                                el.child(self.render_action_item("Reveal in Finder", "⌘↵", has_selection, selected == 3))
                                    .child(self.render_action_item("Quick Look", "⌘Y", has_selection, selected == 4))
                            })
                    )
                    // Footer hint
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(rgb(0x313244))
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(rgb(0x6c7086))
                                    .child("↑↓ Navigate  ↵ Select  esc Close"),
                            ),
                    ),
            )
    }

    /// Render a single action item in the menu
    fn render_action_item(&self, label: &str, shortcut: &str, enabled: bool, selected: bool) -> impl IntoElement {
        let text_color = if enabled { rgb(0xcdd6f4) } else { rgb(0x6c7086) };
        let shortcut_color = if enabled { rgb(0x888888) } else { rgb(0x585858) };
        let bg_color = if selected { rgb(0x45475a) } else { rgba(0x00000000) };

        div()
            .px_3()
            .py(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(bg_color)
            .when(enabled && !selected, |el| {
                el.hover(|el| el.bg(rgb(0x313244))).cursor_pointer()
            })
            .when(selected, |el| el.cursor_pointer())
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(text_color)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .px_1()
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .bg(rgb(0x313244))
                    .text_size(px(10.0))
                    .text_color(shortcut_color)
                    .child(shortcut.to_string()),
            )
    }
}

impl Render for LauncherWindow {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Calculate current animation opacity
        let opacity = self.current_opacity();

        // Clone pending confirmation for use in the closure
        let pending_dialog = self.pending_confirmation.as_ref().map(|(_, d)| d.clone());

        // Main container with rounded corners and shadow
        div()
            .track_focus(&self.focus_handle)
            .key_context("LauncherWindow")
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::confirm_dialog))
            .on_action(cx.listener(Self::next_group))
            .on_action(cx.listener(Self::previous_group))
            .on_action(cx.listener(Self::open_preferences))
            // File Search Mode actions
            .on_action(cx.listener(Self::reveal_in_finder))
            .on_action(cx.listener(Self::quick_look))
            .on_action(cx.listener(Self::copy_path))
            .on_action(cx.listener(Self::copy_file))
            .on_action(cx.listener(Self::show_actions_menu))
            .on_action(cx.listener(|this, _: &QuickSelect1, cx| this.quick_select(0, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect2, cx| this.quick_select(1, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect3, cx| this.quick_select(2, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect4, cx| this.quick_select(3, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect5, cx| this.quick_select(4, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect6, cx| this.quick_select(5, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect7, cx| this.quick_select(6, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect8, cx| this.quick_select(7, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect9, cx| this.quick_select(8, cx)))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            // Apply window appear/dismiss animation opacity
            .opacity(opacity)
            // Catppuccin Mocha base color with slight transparency
            .bg(hsla(240.0 / 360.0, 0.21, 0.15, 0.95))
            .rounded(LAUNCHER_BORDER_RADIUS)
            .shadow_lg()
            .border_1()
            // Catppuccin surface0 with slight transparency
            .border_color(hsla(236.0 / 360.0, 0.13, 0.27, 0.8))
            .overflow_hidden()
            // Search bar
            .child(self.render_search_bar(cx))
            // Divider (only show when there are results or query)
            .when(!self.query.is_empty(), |el| {
                el.child(div().h(px(1.0)).w_full().bg(rgb(0x313244)))
            })
            // Results or empty state
            .when(self.query.is_empty(), |el| el.child(self.render_empty_state()))
            .when(!self.query.is_empty() && self.results.is_empty(), |el| {
                el.child(self.render_no_results())
            })
            .when(!self.results.is_empty(), |el| {
                el.child(self.render_results(cx))
            })
            // Action bar at bottom (only visible when there are results)
            .when(!self.results.is_empty(), |el| {
                el.child(self.render_action_bar())
            })
            // Actions menu overlay (Cmd+K)
            .when(self.show_actions_menu, |el| {
                el.child(self.render_actions_menu(cx))
            })
            // Confirmation dialog overlay
            .when_some(pending_dialog, |el, dialog| {
                el.child(self.render_confirmation_dialog(&dialog, cx))
            })
    }
}

impl FocusableView for LauncherWindow {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ============================================================================
// Helper Functions (public for testing)
// ============================================================================

/// Escapes a path string for safe use in AppleScript.
/// 
/// This prevents command injection attacks by escaping special characters.
#[must_use]
pub fn escape_path_for_applescript(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}
