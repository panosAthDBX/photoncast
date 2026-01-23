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

#![allow(clippy::unreadable_literal)]
#![allow(clippy::unused_self)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::struct_excessive_bools)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::*;
use parking_lot::RwLock;
use photoncast_apps::{
    AppManager, AppsConfig, AutoQuitConfig, AutoQuitManager, UninstallPreview,
    DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES,
};
use photoncast_calculator::commands::{is_calculator_expression, CalculatorCommand};
use photoncast_calculator::{CalculatorResult, CalculatorResultKind};
use photoncast_timer::commands::TimerManager;

use crate::app_events::{self, AppEvent};
use crate::constants::{EXPANDED_HEIGHT, LAUNCHER_HEIGHT, LAUNCHER_WIDTH};
use crate::{
    Activate, Cancel, ConfirmDialog, CopyBundleId, CopyFile, CopyPath, ForceQuitApp, HideApp,
    NextGroup, OpenPreferences, PreviousGroup, QuickLook, QuickSelect1, QuickSelect2, QuickSelect3,
    QuickSelect4, QuickSelect5, QuickSelect6, QuickSelect7, QuickSelect8, QuickSelect9, QuitApp,
    RevealInFinder, SelectNext, SelectPrevious, ShowActionsMenu, ShowInFinder, ToggleAutoQuit,
    UninstallApp, LAUNCHER_BORDER_RADIUS,
};

use photoncast_core::app::integration::PhotonCastApp;
use photoncast_core::commands::{CommandExecutor, ConfirmationDialog, SystemCommand};
use photoncast_core::indexer::{AppScanner, AppWatcher, WatchEvent};
use photoncast_core::platform::launch::AppLauncher;
use photoncast_core::search::{
    IconSource, ResultType as CoreResultType, SearchAction, SearchResult, SearchResultId,
};
use photoncast_core::storage::{Database, UsageTracker};
use photoncast_core::theme::PhotonTheme;
use photoncast_core::ui::animations::{
    ease_in, ease_in_out, ease_out, lerp, selection_change_duration, window_appear_duration,
    window_dismiss_duration, WindowAnimationState, WINDOW_APPEAR_OPACITY_END,
    WINDOW_APPEAR_OPACITY_START, WINDOW_APPEAR_SCALE_END, WINDOW_APPEAR_SCALE_START,
    WINDOW_DISMISS_SCALE_END,
};

/// Helper struct holding theme colors for launcher UI
#[derive(Clone)]
#[allow(dead_code)]
struct LauncherColors {
    background: Hsla,
    text: Hsla,
    text_muted: Hsla,
    text_placeholder: Hsla,
    surface: Hsla,
    surface_hover: Hsla,
    surface_elevated: Hsla,
    border: Hsla,
    accent: Hsla,
    accent_hover: Hsla,
    selection: Hsla,
    success: Hsla,
    warning: Hsla,
    error: Hsla,
    overlay: Hsla,
}

impl LauncherColors {
    fn from_theme(theme: &PhotonTheme) -> Self {
        Self {
            background: theme.colors.background.to_gpui(),
            text: theme.colors.text.to_gpui(),
            text_muted: theme.colors.text_muted.to_gpui(),
            text_placeholder: theme.colors.text_placeholder.to_gpui(),
            surface: theme.colors.surface.to_gpui(),
            surface_hover: theme.colors.surface_hover.to_gpui(),
            surface_elevated: theme.colors.background_elevated.to_gpui(),
            border: theme.colors.border.to_gpui(),
            accent: theme.colors.accent.to_gpui(),
            accent_hover: theme.colors.accent_hover.to_gpui(),
            selection: theme.colors.selection.to_gpui(),
            success: theme.colors.success.to_gpui(),
            warning: theme.colors.warning.to_gpui(),
            error: theme.colors.error.to_gpui(),
            overlay: hsla(0.0, 0.0, 0.0, 0.6), // Semi-transparent overlay
        }
    }
}

fn get_launcher_colors(cx: &ViewContext<LauncherWindow>) -> LauncherColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    LauncherColors::from_theme(&theme)
}

/// Search bar height constant
const SEARCH_BAR_HEIGHT: Pixels = px(48.0);
/// Search icon size
#[allow(dead_code)]
const SEARCH_ICON_SIZE: Pixels = px(20.0);
/// Result item height
const RESULT_ITEM_HEIGHT: Pixels = px(56.0);
/// Maximum visible results
const MAX_VISIBLE_RESULTS: usize = 8;

/// The main launcher window state
pub struct LauncherWindow {
    /// Current search query
    query: SharedString,
    /// Cursor position in the query (character index)
    cursor_position: usize,
    /// Selection anchor position (where selection started, None if no selection)
    selection_anchor: Option<usize>,
    /// Time when cursor last moved (for blink reset)
    cursor_blink_epoch: Instant,
    /// Whether the window is visible
    visible: bool,
    /// Currently selected result index
    selected_index: usize,
    /// Previously selected index (for selection change animation)
    previous_selected_index: Option<usize>,
    /// Filtered results for current query
    results: Vec<ResultItem>,
    /// Base search results for current query (without calculator)
    base_results: Vec<SearchResult>,
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
    /// Whether app indexing has already started
    index_started: Arc<AtomicBool>,
    /// Whether the app index has been initialized
    index_initialized: bool,
    /// Pending command awaiting confirmation (command and dialog info)
    pending_confirmation: Option<(SystemCommand, ConfirmationDialog)>,
    /// Current search mode (Normal or `FileSearch`)
    search_mode: SearchMode,
    /// File search view (shown when in FileSearch mode)
    file_search_view: Option<View<crate::file_search_view::FileSearchView>>,
    /// File search state
    file_search_loading: bool,
    /// Last file search query (for debouncing)
    file_search_pending_query: Option<String>,
    /// File search debounce generation (incremented on each keystroke)
    file_search_generation: u64,
    /// Calculator command state
    calculator_command: Arc<RwLock<CalculatorCommand>>,
    /// Tokio runtime for calculator evaluation
    calculator_runtime: Arc<tokio::runtime::Runtime>,
    /// Latest calculator result for current query
    calculator_result: Option<CalculatorResult>,
    /// Calculator evaluation generation (incremented on each keystroke)
    calculator_generation: u64,
    /// Whether the actions menu (Cmd+K) is visible
    show_actions_menu: bool,
    /// Selected index in the actions menu (for keyboard navigation)
    actions_menu_index: usize,
    /// Timer manager for sleep timer actions
    timer_manager: Arc<tokio::sync::RwLock<TimerManager>>,
    /// App management handler
    app_manager: Arc<AppManager>,
    /// Next upcoming meeting (shown at top of launcher)
    next_meeting: Option<photoncast_calendar::CalendarEvent>,
    /// Whether the next meeting widget is selected (for navigation)
    meeting_selected: bool,
    /// Suggestions (recent/frequent apps shown when query is empty)
    suggestions: Vec<SearchResult>,
    /// All calendar events (stored when entering calendar mode, used for filtering)
    calendar_all_events: Vec<photoncast_calendar::CalendarEvent>,
    /// Scroll handle for results list scrolling
    results_scroll_handle: gpui::ScrollHandle,
    // ========================================================================
    // Uninstall Preview State (Task 7.5)
    // ========================================================================
    /// Current uninstall preview being displayed
    uninstall_preview: Option<UninstallPreview>,
    /// Selected index in the uninstall files list
    uninstall_files_selected_index: usize,
    // ========================================================================
    // Auto Quit Settings State (Task 7.6)
    // ========================================================================
    /// Currently selected app for auto-quit settings (bundle_id)
    auto_quit_settings_app: Option<(String, String)>, // (bundle_id, app_name)
    /// Selected timeout index in auto-quit settings (0 = toggle, 1-7 = timeout options)
    auto_quit_settings_index: usize,
    /// Auto quit manager
    auto_quit_manager: Arc<RwLock<AutoQuitManager>>,
    // ========================================================================
    // Manage Auto Quits State (Task 7.7)
    // ========================================================================
    /// Whether we're in the "Manage Auto Quits" mode
    manage_auto_quits_mode: bool,
    /// Selected index in the manage auto quits list
    manage_auto_quits_index: usize,
    // ========================================================================
    // Window Management State
    // ========================================================================
    /// Bundle ID of the app that was frontmost before Photoncast opened
    /// (used for window management commands to target the correct app)
    previous_frontmost_app: Option<String>,
    /// Title of the window that was frontmost before Photoncast opened
    /// (used for window management commands to target the correct window)
    previous_frontmost_window_title: Option<String>,
    // ========================================================================
    // Toast Notification State (Task 7.8)
    // ========================================================================
    /// Current toast message to display
    toast_message: Option<String>,
    /// When the toast was shown (for auto-dismiss)
    toast_shown_at: Option<Instant>,
}

#[derive(Clone)]
pub struct LauncherSharedState {
    photoncast_app: Arc<RwLock<PhotonCastApp>>,
    app_launcher: Arc<AppLauncher>,
    command_executor: Arc<CommandExecutor>,
    index_started: Arc<AtomicBool>,
    calculator_command: Arc<RwLock<CalculatorCommand>>,
    calculator_runtime: Arc<tokio::runtime::Runtime>,
    timer_manager: Arc<tokio::sync::RwLock<TimerManager>>,
    app_manager: Arc<AppManager>,
}

impl LauncherSharedState {
    #[must_use]
    pub fn new() -> Self {
        // Use persistent database for usage tracking (frecency/recommendations)
        let db_path = photoncast_core::utils::paths::data_dir().join("usage.db");
        let usage_tracker = match Database::open(&db_path) {
            Ok(db) => {
                tracing::info!("Opened usage database at {:?}", db_path);
                UsageTracker::new(db)
            },
            Err(e) => {
                tracing::warn!("Failed to open database at {:?}: {}, falling back to in-memory", db_path, e);
                // Single fallback attempt - if in-memory fails, panic is acceptable
                // since we can't function without any database at all
                let fallback_db = Database::open_in_memory()
                    .expect("Critical: cannot open even in-memory database");
                UsageTracker::new(fallback_db)
            },
        };

        let config = photoncast_core::app::integration::IntegrationConfig {
            search_timeout_ms: 100,
            include_files: false,
            ..Default::default()
        };

        let photoncast_app = Arc::new(RwLock::new(PhotonCastApp::with_config(config)));
        let app_launcher = Arc::new(AppLauncher::new(usage_tracker));
        let command_executor = Arc::new(CommandExecutor::new());
        let calculator_command = Arc::new(RwLock::new(CalculatorCommand::new()));
        let calculator_runtime = Arc::new(tokio::runtime::Runtime::new().unwrap_or_else(|e| {
            tracing::error!("Failed to create calculator runtime: {}, trying current-thread", e);
            // Single fallback - current-thread runtime as last resort
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Critical: cannot create any tokio runtime for calculator")
        }));
        let timer_db_path = photoncast_core::utils::paths::data_dir().join("timer.db");
        let timer_manager = Arc::new(tokio::sync::RwLock::new({
            // Try primary path first, then fallback to /tmp
            let rt = tokio::runtime::Runtime::new()
                .expect("Critical: cannot create tokio runtime for timer");
            
            rt.block_on(TimerManager::new(timer_db_path.clone()))
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to open timer db at {:?}: {}, using /tmp fallback", timer_db_path, e);
                    rt.block_on(TimerManager::new(std::path::PathBuf::from("/tmp/photoncast_timer.db")))
                        .expect("Critical: cannot initialize timer manager even with fallback path")
                })
        }));
        let app_manager = Arc::new(AppManager::new(AppsConfig::default()));

        Self {
            photoncast_app,
            app_launcher,
            command_executor,
            index_started: Arc::new(AtomicBool::new(false)),
            calculator_command,
            calculator_runtime,
            timer_manager,
            app_manager,
        }
    }

    /// Returns a reference to the timer manager for background polling
    pub fn timer_manager(&self) -> Arc<tokio::sync::RwLock<TimerManager>> {
        Arc::clone(&self.timer_manager)
    }

    /// Invalidates the quicklinks cache, causing a reload on next search.
    /// Call this after adding, updating, or deleting quicklinks.
    pub fn invalidate_quicklinks_cache(&self) {
        self.photoncast_app.read().invalidate_quicklinks_cache();
    }

    /// Returns a clone of the PhotonCastApp reference.
    /// Useful for callbacks that need to invalidate caches.
    pub fn photoncast_app(&self) -> Arc<RwLock<PhotonCastApp>> {
        Arc::clone(&self.photoncast_app)
    }
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
    /// Bundle ID for applications (used for running/auto-quit indicators)
    pub bundle_id: Option<String>,
    /// App path for applications (used for reveal in finder, uninstall)
    pub app_path: Option<std::path::PathBuf>,
}

/// Type of search result for grouping
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum ResultType {
    Application,
    Command,
    QuickLink,
    File,
    Folder,
    Calculator,
}

impl ResultType {
    #[allow(dead_code)]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Application => "Apps",
            Self::Command => "Commands",
            Self::QuickLink => "Quick Links",
            Self::File => "Files",
            Self::Folder => "Folders",
            Self::Calculator => "Calculator",
        }
    }
}

impl From<CoreResultType> for ResultType {
    fn from(core_type: CoreResultType) -> Self {
        match core_type {
            CoreResultType::Application => Self::Application,
            CoreResultType::SystemCommand => Self::Command,
            CoreResultType::QuickLink => Self::QuickLink,
            CoreResultType::File => Self::File,
            CoreResultType::Folder => Self::Folder,
        }
    }
}

/// Search mode determines the UI state and behavior.
#[derive(Clone, Default, Debug)]
pub enum SearchMode {
    /// Normal search mode: Apps + Commands (default)
    #[default]
    Normal,
    /// File Search Mode: Spotlight-based file search
    FileSearch,
    /// Calendar mode: shows calendar events inline
    Calendar {
        title: String,
        events: Vec<photoncast_calendar::CalendarEvent>,
        error: Option<String>,
    },
}

impl LauncherWindow {
    /// Creates a new launcher window
    pub fn new(cx: &mut ViewContext<Self>, shared_state: &LauncherSharedState) -> Self {
        let focus_handle = cx.focus_handle();

        // Request focus immediately
        cx.focus(&focus_handle);

        let photoncast_app = Arc::clone(&shared_state.photoncast_app);
        let app_launcher = Arc::clone(&shared_state.app_launcher);
        let command_executor = Arc::clone(&shared_state.command_executor);
        let index_started = Arc::clone(&shared_state.index_started);
        let calculator_command = Arc::clone(&shared_state.calculator_command);
        let calculator_runtime = Arc::clone(&shared_state.calculator_runtime);
        let timer_manager = Arc::clone(&shared_state.timer_manager);
        let app_manager = Arc::clone(&shared_state.app_manager);

        let mut window = Self {
            query: SharedString::default(),
            cursor_position: 0,
            selection_anchor: None,
            cursor_blink_epoch: Instant::now(),
            visible: true,
            selected_index: 0,
            previous_selected_index: None,
            results: vec![],
            base_results: vec![],
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
            index_started,
            index_initialized: false,
            pending_confirmation: None,
            search_mode: SearchMode::Normal,
            file_search_view: None,
            file_search_loading: false,
            file_search_pending_query: None,
            file_search_generation: 0,
            calculator_command,
            calculator_runtime,
            calculator_result: None,
            calculator_generation: 0,
            show_actions_menu: false,
            actions_menu_index: 0,
            timer_manager,
            app_manager,
            next_meeting: None,
            meeting_selected: false,
            suggestions: vec![],
            calendar_all_events: vec![],
            results_scroll_handle: gpui::ScrollHandle::new(),
            // Task 7.5: Uninstall Preview
            uninstall_preview: None,
            uninstall_files_selected_index: 0,
            // Task 7.6: Auto Quit Settings
            auto_quit_settings_app: None,
            auto_quit_settings_index: 0,
            auto_quit_manager: Arc::new(RwLock::new(
                AutoQuitManager::load().unwrap_or_else(|_| AutoQuitManager::new(AutoQuitConfig::default()))
            )),
            // Task 7.7: Manage Auto Quits
            manage_auto_quits_mode: false,
            manage_auto_quits_index: 0,
            // Window Management
            previous_frontmost_app: None,
            previous_frontmost_window_title: None,
            // Task 7.8: Toast Notifications
            toast_message: None,
            toast_shown_at: None,
        };

        // Start the appear animation
        window.start_appear_animation(cx);

        // Start the auto-quit background timer
        window.start_auto_quit_timer(cx);

        // Fetch next meeting (doesn't depend on index)
        window.fetch_next_meeting(cx);

        if !window.index_started.swap(true, Ordering::AcqRel) {
            // Spawn async task to index applications
            window.start_app_indexing(cx);
        } else if window.photoncast_app.read().app_count() > 0 {
            window.index_initialized = true;
            // Load suggestions since index is ready
            window.load_suggestions(cx);
        }

        window
    }

    /// Reset query, cursor position, and selection
    fn reset_query(&mut self) {
        self.query = SharedString::default();
        self.cursor_position = 0;
        self.selection_anchor = None;
        self.cursor_blink_epoch = Instant::now();
    }

    /// Reset cursor blink timer (call on any cursor movement)
    fn reset_cursor_blink(&mut self) {
        self.cursor_blink_epoch = Instant::now();
    }

    /// Check if cursor should be visible based on blink timing
    fn cursor_visible(&self) -> bool {
        const BLINK_INTERVAL_MS: u128 = 530;
        let elapsed = self.cursor_blink_epoch.elapsed().as_millis();
        (elapsed / BLINK_INTERVAL_MS) % 2 == 0
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

    /// Starts the auto-quit background timer.
    ///
    /// This periodically checks for idle apps and quits them based on user settings.
    /// The timer runs every 30 seconds and:
    /// 1. Updates activity tracking for the currently frontmost app
    /// 2. Quits any apps that have been idle longer than their configured timeout
    fn start_auto_quit_timer(&self, cx: &mut ViewContext<Self>) {
        let auto_quit_manager = Arc::clone(&self.auto_quit_manager);

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
                                if !this.query.is_empty() {
                                    this.on_query_change(this.query.clone(), cx);
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
                        if !this.query.is_empty() {
                            this.on_query_change(this.query.clone(), cx);
                        }
                        cx.notify();
                    });
                }
            },
        }
    }

    /// Static version of `get_app_icon_path` for use in async context.
    fn get_app_icon_path_static(app_path: &std::path::Path) -> Option<std::path::PathBuf> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Get cache directory
        let cache_dir = directories::ProjectDirs::from("", "", "PhotonCast").map_or_else(
            || {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("Library/Caches/PhotonCast/icons")
            },
            |dirs| dirs.cache_dir().join("icons"),
        );

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
        if icns_path.exists() {
            // Use sips to convert icns to png
            let output = std::process::Command::new("sips")
                .args([
                    "-s",
                    "format",
                    "png",
                    "-z",
                    "64",
                    "64",
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
        } else {
            // Try to read Info.plist to find the icon name
            let info_plist = app_path.join("Contents/Info.plist");
            if let Ok(plist) = plist::Value::from_file(&info_plist) {
                if let Some(dict) = plist.as_dictionary() {
                    if let Some(icon_name) =
                        dict.get("CFBundleIconFile").and_then(|v| v.as_string())
                    {
                        let icon_name = if std::path::Path::new(icon_name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("icns"))
                        {
                            icon_name.to_string()
                        } else {
                            format!("{icon_name}.icns")
                        };
                        let icon_path = app_path.join("Contents/Resources").join(&icon_name);
                        if icon_path.exists() {
                            // Use sips to convert icns to png
                            let output = std::process::Command::new("sips")
                                .args([
                                    "-s",
                                    "format",
                                    "png",
                                    "-z",
                                    "64",
                                    "64",
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
        }

        None
    }

    /// Clears the cached icon for an app.
    fn clear_cached_icon(app_path: &std::path::Path) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let cache_dir = directories::ProjectDirs::from("", "", "PhotonCast").map_or_else(
            || {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("Library/Caches/PhotonCast/icons")
            },
            |dirs| dirs.cache_dir().join("icons"),
        );

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

        let cache_dir = directories::ProjectDirs::from("", "", "PhotonCast").map_or_else(
            || {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("Library/Caches/PhotonCast/icons")
            },
            |dirs| dirs.cache_dir().join("icons"),
        );

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
                    "align-center" | "align-center-vertical" | "align-center-horizontal" => "⬛".into(),
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
    /// Uses NSWorkspace to handle all icon formats including asset catalogs.
    fn get_app_icon_path(app_path: &std::path::Path) -> Option<std::path::PathBuf> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Get cache directory
        let cache_dir = directories::ProjectDirs::from("", "", "PhotonCast").map_or_else(
            || {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("Library/Caches/PhotonCast/icons")
            },
            |dirs| dirs.cache_dir().join("icons"),
        );

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

    /// Gets icon path for an app by its bundle ID.
    #[allow(dead_code)]
    fn get_icon_path_for_bundle_id(bundle_id: &str) -> Option<std::path::PathBuf> {
        // Try to find app path from bundle ID using NSWorkspace
        let app_path = crate::platform::get_app_path_for_bundle_id(bundle_id)?;
        Self::get_app_icon_path(&app_path)
    }

    /// Parses a hex color string (e.g., "#0088FF") to an Hsla color.
    fn parse_hex_color(hex: &str) -> gpui::Hsla {
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
                let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
                
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
        }
    }

    fn calculator_result_to_search_result(result: &CalculatorResult) -> SearchResult {
        let subtitle = Self::calculator_subtitle(result);
        SearchResult {
            id: photoncast_core::search::SearchResultId::new(format!(
                "calculator:{}",
                result.expression
            )),
            title: result.formatted_value.clone(),
            subtitle,
            icon: IconSource::Emoji {
                char: Self::calculator_icon(result),
            },
            result_type: CoreResultType::SystemCommand,
            score: 0.0,
            match_indices: vec![],
            action: SearchAction::CopyToClipboard {
                text: result.formatted_value.clone(),
            },
        }
    }

    fn calculator_result_to_result_item(result: &CalculatorResult) -> ResultItem {
        ResultItem {
            id: SharedString::from(format!("calculator:{}", result.expression)),
            title: result.formatted_value.clone().into(),
            subtitle: Self::calculator_subtitle(result).into(),
            icon_emoji: SharedString::from(Self::calculator_icon(result).to_string()),
            icon_path: None,
            result_type: ResultType::Calculator,
            bundle_id: None,
            app_path: None,
        }
    }

    const fn calculator_icon(result: &CalculatorResult) -> char {
        match &result.kind {
            CalculatorResultKind::Math => '🔢',
            CalculatorResultKind::Currency { .. } => '💱',
            CalculatorResultKind::Unit { .. } => '📏',
            CalculatorResultKind::DateTime => '📅',
        }
    }

    fn calculator_subtitle(result: &CalculatorResult) -> String {
        result
            .details
            .clone()
            .unwrap_or_else(|| result.expression.clone())
    }

    /// Fetches the next upcoming meeting from the calendar.
    fn fetch_next_meeting(&mut self, cx: &mut ViewContext<Self>) {
        tracing::debug!("fetch_next_meeting: starting");
        cx.spawn(|this, mut cx| async move {
            // Run calendar fetch in background
            let result = cx
                .background_executor()
                .spawn(async move {
                    let calendar = photoncast_calendar::CalendarCommand::with_default_config();
                    // Fetch events for the next 24 hours
                    calendar.fetch_upcoming_events(1)
                })
                .await;

            let _ = this.update(&mut cx, |this, cx| {
                match result {
                    Ok(events) => {
                        tracing::debug!("fetch_next_meeting: got {} events", events.len());
                        // Find the next event that hasn't ended yet
                        let now = photoncast_calendar::chrono::Local::now();
                        this.next_meeting = events.into_iter().find(|e| e.end > now);
                        if this.next_meeting.is_some() {
                            tracing::debug!(
                                "Next meeting found: {:?}",
                                this.next_meeting.as_ref().map(|m| &m.title)
                            );
                            // Select meeting by default when query is empty
                            if this.query.is_empty() {
                                this.meeting_selected = true;
                            }
                        } else {
                            tracing::debug!("fetch_next_meeting: no upcoming meeting found");
                            this.meeting_selected = false;
                        }
                    },
                    Err(e) => {
                        tracing::debug!("Could not fetch next meeting: {}", e);
                        this.next_meeting = None;
                        this.meeting_selected = false;
                    },
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Loads suggestions (frequently used apps).
    fn load_suggestions(&mut self, cx: &mut ViewContext<Self>) {
        tracing::debug!("load_suggestions: index_initialized={}", self.index_initialized);
        // Only load if index is ready
        if !self.index_initialized {
            tracing::debug!("Skipping suggestions - index not initialized");
            return;
        }

        // Try to get frecency-based suggestions (recently/frequently used apps)
        let frecent_bundle_ids = self.app_launcher.get_top_apps_by_frecency(6);
        tracing::debug!("Frecency returned {} apps: {:?}", frecent_bundle_ids.len(), frecent_bundle_ids);

        if !frecent_bundle_ids.is_empty() {
            // Look up each app by bundle ID directly from index
            let app = self.photoncast_app.read();
            self.suggestions = frecent_bundle_ids
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
                            action: SearchAction::LaunchApp {
                                bundle_id: indexed_app.bundle_id.as_str().to_string(),
                                path: indexed_app.path.clone(),
                            },
                        }
                    })
                })
                .collect();
            tracing::debug!("Loaded {} frecency-based suggestions", self.suggestions.len());
        } else {
            // Fallback: search for common apps if no usage data yet
            tracing::debug!("No frecency data, falling back to search-based suggestions");
            let outcome = self.photoncast_app.read().search(""); // Empty search returns popular apps
            self.suggestions = outcome
                .results
                .groups
                .into_iter()
                .filter(|g| g.result_type == CoreResultType::Application)
                .flat_map(|g| g.results)
                .take(6)
                .collect();
            tracing::debug!("Loaded {} fallback suggestions", self.suggestions.len());
        }

        // If query is empty, populate results with suggestions so they're navigable
        if self.query.is_empty() && !matches!(self.search_mode, SearchMode::Calendar { .. }) {
            self.core_results = self.suggestions.clone();
            self.results = self
                .core_results
                .iter()
                .map(|r| Self::search_result_to_result_item(r))
                .collect();
            tracing::debug!("Populated {} results from suggestions", self.results.len());
        }

        // Notify to trigger re-render
        cx.notify();
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

        // Start cursor blink timer
        self.cursor_blink_epoch = Instant::now();
        cx.spawn(|this, mut cx| async move {
            let blink_interval = Duration::from_millis(530);
            loop {
                gpui::Timer::after(blink_interval).await;
                let should_continue = this
                    .update(&mut cx, |this, cx| {
                        if this.visible {
                            cx.notify(); // Trigger redraw for cursor blink
                            true
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false);
                if !should_continue {
                    break;
                }
            }
        })
        .detach();

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
            let () = cx.remove_window();
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
                                        let () = cx.remove_window();
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
        tracing::debug!("toggle() called, visible was {}", self.visible);
        self.visible = !self.visible;
        if self.visible {
            tracing::debug!("toggle: showing window, calling fetch_next_meeting and load_suggestions");
            self.reset_query();
            self.selected_index = 0;
            self.previous_selected_index = None;
            // Select meeting by default if available, otherwise first result
            self.meeting_selected = self.next_meeting.is_some();
            // Reset scroll position
            self.results_scroll_handle.set_offset(gpui::Point::default());
            cx.focus(&self.focus_handle);
            self.start_appear_animation(cx);
            self.fetch_next_meeting(cx);
            self.load_suggestions(cx);
        } else {
            self.start_dismiss_animation(cx);
        }
    }

    /// Shows the launcher window with animation
    #[allow(dead_code)]
    pub fn show(&mut self, cx: &mut ViewContext<Self>) {
        self.visible = true;
        self.reset_query();
        self.selected_index = 0;
        self.previous_selected_index = None;
        cx.focus(&self.focus_handle);
        self.start_appear_animation(cx);
        self.fetch_next_meeting(cx);
        self.load_suggestions(cx);
    }

    /// Hides the launcher window with animation
    pub fn hide(&mut self, cx: &mut ViewContext<Self>) {
        if matches!(self.search_mode, SearchMode::Calendar { .. }) {
            self.exit_calendar_mode(cx);
            return;
        }

        // Clean up file search mode if active
        if matches!(self.search_mode, SearchMode::FileSearch) {
            self.search_mode = SearchMode::Normal;
            self.file_search_view = None;
        }

        self.visible = false;
        self.start_dismiss_animation(cx);
    }

    /// Sets the bundle ID and window title that was frontmost before Photoncast opened.
    /// Used for window management commands to target the correct window.
    pub fn set_previous_frontmost_window(&mut self, bundle_id: Option<String>, window_title: Option<String>) {
        self.previous_frontmost_app = bundle_id;
        self.previous_frontmost_window_title = window_title;
    }

    /// Handle query change from search input
    fn on_query_change(&mut self, _query: SharedString, cx: &mut ViewContext<Self>) {
        self.selected_index = 0;
        // Deselect meeting when user starts typing
        self.meeting_selected = false;

        // In calendar mode, filter events by query from the full list
        if let SearchMode::Calendar { title, error, .. } = &self.search_mode {
            let query_lower = self.query.to_lowercase();
            let filtered: Vec<_> = if query_lower.is_empty() {
                self.calendar_all_events.clone()
            } else {
                self.calendar_all_events
                    .iter()
                    .filter(|e| e.title.to_lowercase().contains(&query_lower))
                    .cloned()
                    .collect()
            };
            self.search_mode = SearchMode::Calendar {
                title: title.clone(),
                events: filtered,
                error: error.clone(),
            };
            self.selected_index = 0;
            cx.notify();
            return;
        }

        // Perform search using the core library
        if self.query.is_empty() {
            // When query is empty, show suggestions as results so they're navigable
            self.base_results.clear();
            self.core_results = self.suggestions.clone();
            self.results = self
                .core_results
                .iter()
                .map(|r| Self::search_result_to_result_item(r))
                .collect();
            self.calculator_result = None;
            self.calculator_generation = self.calculator_generation.saturating_add(1);
            // Close actions menu when results are cleared
            self.show_actions_menu = false;
        } else {
            match self.search_mode {
                SearchMode::Normal => {
                    // Normal mode: search apps and commands using PhotonCastApp
                    let outcome = self.photoncast_app.read().search(&self.query);

                    // Collect all results from the search outcome
                    self.base_results = outcome.results.iter().cloned().collect();
                    self.calculator_result = None;
                    self.rebuild_results(cx);
                    self.schedule_calculator_evaluation(cx);

                    // Check if this is a "show timer" query - fetch active timer async
                    let query_lower = self.query.to_lowercase();
                    if query_lower.contains("show") || query_lower.contains("status") || query_lower.contains("active timer") {
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
                    self.calculator_result = None;
                    self.calculator_generation = self.calculator_generation.saturating_add(1);
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
    fn schedule_file_search(&mut self, cx: &mut ViewContext<Self>) {
        use crate::file_search_helper::{adaptive_debounce_ms, spotlight_search};

        let query = self.query.to_string();

        // Require at least 2 characters before searching
        if query.len() < 2 {
            self.results.clear();
            self.base_results.clear();
            self.core_results.clear();
            self.file_search_loading = false;
            self.calculator_result = None;
            return;
        }

        // Increment generation to invalidate previous searches
        self.file_search_generation += 1;
        let generation = self.file_search_generation;

        // Adaptive debounce: shorter for longer queries (more specific = faster)
        let debounce_ms = adaptive_debounce_ms(query.len());

        // Show loading state
        self.file_search_loading = true;
        self.file_search_pending_query = Some(query.clone());

        // Spawn debounced async search
        cx.spawn(|this, mut cx| async move {
            // Adaptive debounce
            cx.background_executor()
                .timer(Duration::from_millis(debounce_ms))
                .await;

            // Check if this search is still valid (no newer keystrokes)
            let should_search = this
                .update(&mut cx, |view, _| view.file_search_generation == generation)
                .unwrap_or(false);

            if !should_search {
                return; // A newer search was scheduled, abort this one
            }

            // Execute the actual search using native SpotlightSearchService
            // (SpotlightSearchService has built-in caching)
            let search_results: Vec<SearchResult> =
                cx.background_executor()
                    .spawn(async move {
                        // Use native Spotlight search
                        let file_results = spotlight_search(&query, MAX_VISIBLE_RESULTS);
                        
                        // Convert FileResult to SearchResult for UI
                        file_results
                            .into_iter()
                            .map(|file| {
                                let path = file.path.clone();
                                let name = file.name.clone();
                                let subtitle = path.parent().map(|p| p.display().to_string()).unwrap_or_default();
                                let is_dir = path.is_dir();
                                let is_app = path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("app"));
                                let result_type = if is_app { CoreResultType::Application } else if is_dir { CoreResultType::Folder } else { CoreResultType::File };

                                SearchResult {
                                    id: photoncast_core::search::SearchResultId::new(format!("file:{}", path.display())),
                                    title: name,
                                    subtitle,
                                    icon: IconSource::FileIcon { path: path.clone() },
                                    result_type,
                                    score: 0.0,
                                    match_indices: vec![],
                                    action: SearchAction::OpenFile { path },
                                }
                            })
                            .collect()
                    })
                    .await;

            // Update UI with results (if this search is still valid)
            let _ = this.update(&mut cx, |view, cx| {
                if view.file_search_generation == generation {
                    view.file_search_loading = false;
                    view.base_results = search_results;
                    view.calculator_result = None;
                    view.rebuild_results(cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Fetches active timer and adds it to results if found
    fn fetch_active_timer_result(&mut self, cx: &mut ViewContext<Self>) {
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
                    icon: IconSource::SystemIcon { name: "clock".to_string() },
                    result_type: photoncast_core::search::ResultType::SystemCommand,
                    score: 15000.0, // Very high score to show at top
                    match_indices: vec![],
                    action: SearchAction::OpenSleepTimer { expression: "cancel".to_string() },
                };
                
                let _ = this.update(&mut cx, |view, cx| {
                    // Insert active timer at the beginning of results
                    view.core_results.insert(0, search_result.clone());
                    view.results.insert(0, Self::search_result_to_result_item(&search_result));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn schedule_calculator_evaluation(&mut self, cx: &mut ViewContext<Self>) {
        let expression = self.query.to_string();

        if !is_calculator_expression(&expression) {
            if self.calculator_result.is_some() {
                self.calculator_result = None;
                self.rebuild_results(cx);
            }
            self.calculator_generation = self.calculator_generation.saturating_add(1);
            return;
        }

        self.calculator_generation = self.calculator_generation.saturating_add(1);
        let generation = self.calculator_generation;
        let calculator_command = Arc::clone(&self.calculator_command);
        let calculator_runtime = Arc::clone(&self.calculator_runtime);

        cx.spawn(|this, mut cx| async move {
            cx.background_executor()
                .timer(Duration::from_millis(120))
                .await;

            let should_eval = this
                .update(&mut cx, |view, _| view.calculator_generation == generation)
                .unwrap_or(false);

            if !should_eval {
                return;
            }

            let expression_clone = expression.clone();
            let evaluation = cx
                .background_executor()
                .spawn(async move {
                    let mut command = calculator_command.write();
                    if !command.is_ready() {
                        calculator_runtime
                            .block_on(command.initialize())
                            .map_err(|err| err.to_string())?;
                    }

                    calculator_runtime
                        .block_on(command.evaluate(&expression_clone))
                        .map_err(|err| err.to_string())
                })
                .await;

            let _ = this.update(&mut cx, |view, cx| {
                if view.calculator_generation != generation {
                    return;
                }

                match evaluation {
                    Ok(result) => {
                        view.calculator_result = Some(result);
                    },
                    Err(error) => {
                        tracing::warn!("Calculator evaluation failed: {}", error);
                        view.calculator_result = None;
                    },
                }

                view.rebuild_results(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn rebuild_results(&mut self, _cx: &mut ViewContext<Self>) {
        self.core_results.clear();
        self.results.clear();

        if let Some(result) = &self.calculator_result {
            self.core_results
                .push(Self::calculator_result_to_search_result(result));
            self.results
                .push(Self::calculator_result_to_result_item(result));
        }

        for result in &self.base_results {
            if self.core_results.len() >= MAX_VISIBLE_RESULTS {
                break;
            }
            self.core_results.push(result.clone());
            self.results
                .push(Self::search_result_to_result_item(result));
        }

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
        // If file search is active, forward navigation to it
        if let Some(file_search_view) = &self.file_search_view {
            file_search_view.update(cx, |view, cx| view.navigate_next(cx));
            return;
        }
        
        // If auto-quit settings is open, navigate within it
        // Options: 0 = toggle, 1-7 = timeout options (1, 2, 3, 5, 10, 15, 30 minutes)
        if self.auto_quit_settings_app.is_some() {
            let option_count = 8; // toggle + 7 timeout options
            self.auto_quit_settings_index = (self.auto_quit_settings_index + 1) % option_count;
            cx.notify();
            return;
        }

        // If actions menu is open, navigate within it
        if self.show_actions_menu {
            let action_count = self.get_actions_count();
            if action_count > 0 {
                self.actions_menu_index = (self.actions_menu_index + 1) % action_count;
                cx.notify();
            }
            return;
        }

        // Handle calendar mode navigation (cyclic)
        if let SearchMode::Calendar { events, .. } = &self.search_mode {
            if !events.is_empty() {
                let previous = self.selected_index;
                self.selected_index = (self.selected_index + 1) % events.len();
                if self.selected_index != previous {
                    self.start_selection_animation(previous, cx);
                    self.ensure_selected_visible(cx);
                }
                cx.notify();
            }
            return;
        }

        // Normal mode with meeting + results navigation
        let has_meeting = self.query.is_empty() && self.next_meeting.is_some();
        let results_len = self.results.len();

        if has_meeting && self.meeting_selected {
            // Move from meeting to first result (or wrap to meeting if no results)
            if !self.results.is_empty() {
                self.meeting_selected = false;
                self.selected_index = 0;
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        } else if !self.results.is_empty() {
            let previous = self.selected_index;
            if self.selected_index + 1 >= results_len {
                // At last result - wrap to meeting (if present) or first result
                if has_meeting {
                    self.meeting_selected = true;
                    self.selected_index = 0;
                } else {
                    self.selected_index = 0; // Wrap to first
                }
            } else {
                self.selected_index += 1;
            }
            if self.selected_index != previous || self.meeting_selected {
                self.start_selection_animation(previous, cx);
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        } else if has_meeting {
            // Only meeting, keep it selected
            self.meeting_selected = true;
            cx.notify();
        }
    }

    fn select_previous(&mut self, _: &SelectPrevious, cx: &mut ViewContext<Self>) {
        // If file search is active, forward navigation to it
        if let Some(file_search_view) = &self.file_search_view {
            file_search_view.update(cx, |view, cx| view.navigate_previous(cx));
            return;
        }
        
        // If auto-quit settings is open, navigate within it
        if self.auto_quit_settings_app.is_some() {
            let option_count = 8; // toggle + 7 timeout options
            self.auto_quit_settings_index = if self.auto_quit_settings_index == 0 {
                option_count - 1
            } else {
                self.auto_quit_settings_index - 1
            };
            cx.notify();
            return;
        }

        // If actions menu is open, navigate within it
        if self.show_actions_menu {
            let action_count = self.get_actions_count();
            if action_count > 0 {
                self.actions_menu_index = if self.actions_menu_index == 0 {
                    action_count - 1
                } else {
                    self.actions_menu_index - 1
                };
                cx.notify();
            }
            return;
        }

        // Handle calendar mode navigation (cyclic)
        if let SearchMode::Calendar { events, .. } = &self.search_mode {
            if !events.is_empty() {
                let previous = self.selected_index;
                self.selected_index = if self.selected_index == 0 {
                    events.len() - 1
                } else {
                    self.selected_index - 1
                };
                if self.selected_index != previous {
                    self.start_selection_animation(previous, cx);
                    self.ensure_selected_visible(cx);
                }
                cx.notify();
            }
            return;
        }

        // Normal mode with meeting + results navigation
        let has_meeting = self.query.is_empty() && self.next_meeting.is_some();
        let results_len = self.results.len();

        if has_meeting && self.meeting_selected {
            // Move from meeting to last result (or stay if no results)
            if !self.results.is_empty() {
                self.meeting_selected = false;
                self.selected_index = results_len - 1;
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        } else if !self.results.is_empty() {
            let previous = self.selected_index;
            if self.selected_index == 0 {
                // At first result - wrap to meeting (if present) or last result
                if has_meeting {
                    self.meeting_selected = true;
                } else {
                    self.selected_index = results_len - 1; // Wrap to last
                }
            } else {
                self.selected_index -= 1;
            }
            if self.selected_index != previous || self.meeting_selected {
                self.start_selection_animation(previous, cx);
                self.ensure_selected_visible(cx);
            }
            cx.notify();
        } else if has_meeting {
            // Only meeting, keep it selected
            self.meeting_selected = true;
            cx.notify();
        }
    }

    fn activate(&mut self, _: &Activate, cx: &mut ViewContext<Self>) {
        // If file search view is active, handle Enter for actions menu, dropdown, or file open
        if let Some(file_search_view) = &self.file_search_view {
            let selected_path = file_search_view.update(cx, |view, cx| {
                if view.actions_menu_open {
                    // Execute the selected action
                    if let Some(&(_, _, action_id)) = crate::file_search_view::FileSearchView::FILE_ACTIONS.get(view.actions_menu_index) {
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
                let _ = std::process::Command::new("open")
                    .arg(&path)
                    .spawn();
                self.hide(cx);
            }
            return;
        }
        
        // If uninstall preview is showing, perform the uninstall
        if self.uninstall_preview.is_some() {
            self.perform_uninstall(cx);
            return;
        }

        // If auto-quit settings is open, activate the selected option
        if self.auto_quit_settings_app.is_some() {
            self.activate_auto_quit_settings_option(cx);
            return;
        }

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

        // In Calendar mode, join the selected meeting if it has a conference link
        if let SearchMode::Calendar { events, .. } = &self.search_mode {
            if let Some(event) = events.get(self.selected_index) {
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
        if self.meeting_selected && self.query.is_empty() {
            if let Some(meeting) = &self.next_meeting {
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
                    if let Err(e) = std::process::Command::new("open").arg(&calendar_url).spawn() {
                        tracing::warn!("Failed to open event directly: {}, opening Calendar app", e);
                        let _ = std::process::Command::new("open").arg("-a").arg("Calendar").spawn();
                    }
                    self.hide(cx);
                    return;
                }
            }
        }

        if let Some(core_result) = self.core_results.get(self.selected_index).cloned() {
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
                        self.on_query_change(self.query.clone(), cx);
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

    /// Handles the `ConfirmDialog` action (Enter key in confirmation dialog)
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
        // If file search view is active, check for menus first
        if let Some(file_search_view) = &self.file_search_view {
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
        if self.uninstall_preview.is_some() {
            self.cancel_uninstall_preview(cx);
            return;
        }

        // If auto-quit settings is showing, close it first
        if self.auto_quit_settings_app.is_some() {
            self.close_auto_quit_settings(cx);
            return;
        }

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

        // If in file search mode (without view - shouldn't happen), exit back to normal mode
        if matches!(self.search_mode, SearchMode::FileSearch) {
            self.exit_file_search_mode(cx);
            return;
        }

        if matches!(self.search_mode, SearchMode::Calendar { .. }) {
            self.exit_calendar_mode(cx);
            return;
        }

        if self.query.is_empty() {
            // Close window with animation (hide() calls start_dismiss_animation which quits)
            self.hide(cx);
        } else {
            // Clear query first
            self.reset_query();
            self.results.clear();
            self.base_results.clear();
            self.core_results.clear();
            self.selected_index = 0;
            self.calculator_result = None;
            self.calculator_generation = self.calculator_generation.saturating_add(1);
            // Reload suggestions for empty state
            self.load_suggestions(cx);
            cx.notify();
        }
    }

    fn quick_select(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        // Check if meeting widget is visible (query empty + meeting exists)
        let has_meeting = self.query.is_empty() && self.next_meeting.is_some();
        
        if has_meeting {
            if index == 0 {
                // Cmd+1 selects the meeting
                self.meeting_selected = true;
                self.activate(&Activate, cx);
            } else {
                // Cmd+2 -> results[0], Cmd+3 -> results[1], etc.
                let result_index = index - 1;
                if result_index < self.results.len() {
                    self.meeting_selected = false;
                    self.selected_index = result_index;
                    self.activate(&Activate, cx);
                }
            }
        } else {
            // No meeting visible, indices map directly to results
            if index < self.results.len() {
                self.meeting_selected = false;
                self.selected_index = index;
                self.activate(&Activate, cx);
            }
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
        tracing::debug!("Entering File Search Mode");
        self.search_mode = SearchMode::FileSearch;

        // Create the file search view
        let file_search_view = cx.new_view(|cx| {
            let mut view = crate::file_search_view::FileSearchView::new(cx);
            // Load recent files
            view.loading = true;
            view
        });

        // Observe the file search view for should_close, needs_refetch, query_changed, and action flags
        cx.observe(&file_search_view, |this, view, cx| {
            let (should_close, needs_refetch, query_changed, filter, query,
                 wants_reveal, wants_quick_look, wants_actions, wants_open, selected_path) = {
                let v = view.read(cx);
                (
                    v.should_close, v.needs_refetch, v.query_changed, v.filter, v.query.clone(),
                    v.wants_reveal_in_finder, v.wants_quick_look, v.wants_actions_menu, v.wants_open_file,
                    v.selected_file().map(|f| f.path.clone())
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
                use crate::file_search_helper::{spotlight_search, spotlight_recent_files_filtered};
                
                let view_handle = view.downgrade();
                let query_str = query.to_string();
                let filter_for_search = filter;
                
                // Clear the flag first
                view.update(cx, |v, _| {
                    v.query_changed = false;
                });
                
                // If in browsing mode, don't trigger Spotlight search - browsing handles its own results
                let is_browsing = view.read(cx).section_mode == crate::file_search_view::SectionMode::Browsing;
                if is_browsing {
                    return;
                }
                
                // If query is empty, reload recent files (filtered if a filter is active)
                if query_str.is_empty() {
                    cx.spawn(|_this, mut cx| async move {
                        // Use filtered fetch to respect current filter
                        let recent_files = cx
                            .background_executor()
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
                            .spawn(async move {
                                spotlight_search(&query_str, 50)
                            })
                            .await;

                        if let Some(view) = view_handle.upgrade() {
                            let _ = view.update(&mut cx, |view, cx| {
                                // Apply filter to search results
                                view.all_results = search_results;
                                view.results = view.all_results.iter()
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
                        .spawn(async move {
                            spotlight_recent_files_filtered(filter_for_closure, 50)
                        })
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
                .spawn(async move {
                    spotlight_recent_files(7, 50)
                })
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

        self.file_search_view = Some(file_search_view.clone());
        
        // Focus the file search view after storing it
        cx.focus_view(&file_search_view);
        self.reset_query();
        self.results.clear();
        self.base_results.clear();
        self.core_results.clear();
        self.selected_index = 0;
        self.file_search_loading = false;
        self.file_search_pending_query = None;
        self.file_search_generation += 1;
        self.calculator_result = None;
        self.calculator_generation = self.calculator_generation.saturating_add(1);
        
        // Resize window to fit file search view (deferred via dispatch_async)
        crate::platform::resize_window(LAUNCHER_WIDTH.0.into(), EXPANDED_HEIGHT.0.into());
        
        cx.notify();
    }

    pub fn show_calendar(
        &mut self,
        title: String,
        events: Vec<photoncast_calendar::CalendarEvent>,
        cx: &mut ViewContext<Self>,
    ) {
        tracing::info!("Entering Calendar Mode with {} events", events.len());
        // Store all events for filtering
        self.calendar_all_events = events.clone();
        self.search_mode = SearchMode::Calendar {
            title,
            events,
            error: None,
        };
        self.reset_query();
        self.results.clear();
        self.base_results.clear();
        self.core_results.clear();
        self.selected_index = 0;
        self.file_search_loading = false;
        self.file_search_pending_query = None;
        self.calculator_result = None;
        self.calculator_generation = self.calculator_generation.saturating_add(1);
        cx.notify();
    }

    pub fn show_calendar_error(
        &mut self,
        title: String,
        error: String,
        cx: &mut ViewContext<Self>,
    ) {
        tracing::info!("Entering Calendar Mode with error");
        self.search_mode = SearchMode::Calendar {
            title,
            events: Vec::new(),
            error: Some(error),
        };
        self.reset_query();
        self.results.clear();
        self.base_results.clear();
        self.core_results.clear();
        self.selected_index = 0;
        self.file_search_loading = false;
        self.file_search_pending_query = None;
        self.calculator_result = None;
        self.calculator_generation = self.calculator_generation.saturating_add(1);
        cx.notify();
    }

    fn exit_calendar_mode(&mut self, cx: &mut ViewContext<Self>) {
        tracing::info!("Exiting Calendar Mode");
        self.search_mode = SearchMode::Normal;
        self.reset_query();
        self.results.clear();
        self.base_results.clear();
        self.core_results.clear();
        self.calendar_all_events.clear();
        self.selected_index = 0;
        self.file_search_loading = false;
        self.file_search_pending_query = None;
        self.file_search_generation += 1;
        self.calculator_result = None;
        self.calculator_generation = self.calculator_generation.saturating_add(1);
        // Reload suggestions for empty state
        self.load_suggestions(cx);
        cx.notify();
    }

    /// Exits File Search Mode and returns to normal search.
    fn exit_file_search_mode(&mut self, cx: &mut ViewContext<Self>) {
        tracing::info!("Exiting File Search Mode");
        self.search_mode = SearchMode::Normal;
        self.file_search_view = None; // Clean up the file search view
        self.reset_query();
        self.results.clear();
        self.base_results.clear();
        self.core_results.clear();
        self.selected_index = 0;
        self.file_search_loading = false;
        self.file_search_pending_query = None;
        // Reload suggestions for empty state
        self.load_suggestions(cx);
        self.file_search_generation += 1;
        self.calculator_result = None;
        self.calculator_generation = self.calculator_generation.saturating_add(1);
        
        // Resize window back to normal (deferred via dispatch_async)
        crate::platform::resize_window(LAUNCHER_WIDTH.0.into(), LAUNCHER_HEIGHT.0.into());
        
        cx.notify();
    }

    /// Handles the Reveal in Finder action (Cmd+Enter).
    fn reveal_in_finder(&mut self, _: &RevealInFinder, cx: &mut ViewContext<Self>) {
        // If file search view is active, reveal selected file
        if let Some(file_search_view) = &self.file_search_view {
            let selected_path = file_search_view.read(cx).selected_file().map(|f| f.path.clone());
            if let Some(path) = selected_path {
                tracing::info!("Reveal in Finder (file search): {}", path.display());
                let _ = photoncast_apps::reveal_in_finder(&path);
                self.hide(cx);
            }
            return;
        }
        
        // Only active in file search mode with a selected file result
        if !matches!(self.search_mode, SearchMode::FileSearch) {
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
    fn quick_look(&mut self, _: &QuickLook, cx: &mut ViewContext<Self>) {
        // If file search view is active, trigger Quick Look for selected file
        if let Some(file_search_view) = &self.file_search_view {
            let selected_path = file_search_view.read(cx).selected_file().map(|f| f.path.clone());
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
        if !matches!(self.search_mode, SearchMode::FileSearch) {
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
                },
                _ => {
                    tracing::debug!("Quick Look: not a file result");
                },
            }
        }
    }

    /// Handles the Copy Path action (Cmd+C).
    fn copy_path(&mut self, _: &CopyPath, cx: &mut ViewContext<Self>) {
        if let Some(core_result) = self.core_results.get(self.selected_index).cloned() {
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
        self.show_actions_menu = false;
        cx.notify();
    }

    /// Handles the Copy File action (Cmd+Shift+C).
    /// Copies the actual file to clipboard so it can be pasted in apps like Slack, `WhatsApp`, etc.
    fn copy_file(&mut self, _: &CopyFile, cx: &mut ViewContext<Self>) {
        if let Some(core_result) = self.core_results.get(self.selected_index).cloned() {
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
        self.show_actions_menu = false;
        cx.notify();
    }

    /// Handles the Show Actions Menu action (Cmd+K).
    fn show_actions_menu(&mut self, _: &ShowActionsMenu, cx: &mut ViewContext<Self>) {
        tracing::info!("show_actions_menu called, search_mode={:?}", std::mem::discriminant(&self.search_mode));
        
        // If file search view is active, trigger its actions menu
        if let Some(file_search_view) = &self.file_search_view {
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
        let has_items = if let SearchMode::Calendar { events, .. } = &self.search_mode {
            tracing::info!("Calendar mode with {} events", events.len());
            !events.is_empty()
        } else {
            tracing::info!("Non-calendar mode with {} results", self.results.len());
            !self.results.is_empty()
        };

        if !has_items {
            tracing::info!("No items, not showing actions menu");
            return;
        }

        // Toggle actions menu
        tracing::info!("Toggling actions menu: {} -> {}", self.show_actions_menu, !self.show_actions_menu);
        self.show_actions_menu = !self.show_actions_menu;
        self.actions_menu_index = 0; // Reset selection when opening
        cx.notify();
    }

    /// Returns the number of actions available in the current context.
    fn get_actions_count(&self) -> usize {
        // Calendar mode has its own actions
        if let SearchMode::Calendar { events, .. } = &self.search_mode {
            if events.is_empty() || self.selected_index >= events.len() {
                return 0;
            }
            let event = &events[self.selected_index];
            // Actions: Join Meeting (if has conference), Copy Title, Copy Details, Open in Calendar
            let mut count = 3; // Copy Title, Copy Details, Open in Calendar
            if event.conference_url.is_some() {
                count += 1; // Join Meeting
            }
            return count;
        }

        let is_file_mode = matches!(self.search_mode, SearchMode::FileSearch);
        let has_selection = !self.results.is_empty();

        if !has_selection {
            return 0;
        }

        // Task 7.3: Check if selected result is an app
        let selected_result = self.results.get(self.selected_index);
        let is_app = selected_result.is_some_and(|r| r.result_type == ResultType::Application);
        let app_bundle_id = selected_result.and_then(|r| r.bundle_id.clone());
        let is_running = app_bundle_id.as_ref().is_some_and(|id| photoncast_apps::is_app_running(id));

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
    fn execute_selected_action(&mut self, cx: &mut ViewContext<Self>) {
        // Handle calendar mode actions
        if let SearchMode::Calendar { events, .. } = &self.search_mode {
            if self.selected_index < events.len() {
                let event = events[self.selected_index].clone();
                let has_conference = event.conference_url.is_some();
                
                // Action order: Join Meeting (if available), Copy Title, Copy Details, Open in Calendar
                let action_idx = self.actions_menu_index;
                let adjusted_idx = if has_conference { action_idx } else { action_idx + 1 };
                
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
                    },
                    2 => {
                        // Copy Details
                        let time_str = if event.is_all_day {
                            format!("{} (All day)", event.start.format("%A, %B %d, %Y"))
                        } else {
                            format!("{} - {}", 
                                event.start.format("%A, %B %d, %Y %H:%M"),
                                event.end.format("%H:%M"))
                        };
                        let mut details = format!("{}\n{}\nCalendar: {}", event.title, time_str, event.calendar_name);
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
                    },
                    3 => {
                        // Open in Calendar
                        // Use the event ID to open in Calendar app
                        let calendar_url = format!("ical://ekevent/{}", event.id);
                        if let Err(e) = std::process::Command::new("open").arg(&calendar_url).spawn() {
                            // Fallback: just open Calendar app
                            tracing::warn!("Failed to open event directly: {}, opening Calendar app", e);
                            let _ = std::process::Command::new("open").arg("-a").arg("Calendar").spawn();
                        }
                    },
                    _ => {},
                }
            }
            self.show_actions_menu = false;
            cx.notify();
            return;
        }

        let is_file_mode = matches!(self.search_mode, SearchMode::FileSearch);
        let has_selection = !self.results.is_empty();

        if !has_selection {
            self.show_actions_menu = false;
            cx.notify();
            return;
        }

        // Task 7.3: Check if selected result is an app
        let selected_result = self.results.get(self.selected_index);
        let is_app = selected_result.is_some_and(|r| r.result_type == ResultType::Application);
        let app_bundle_id = selected_result.and_then(|r| r.bundle_id.clone());
        let app_path = selected_result.and_then(|r| r.app_path.clone());
        let is_running = app_bundle_id.as_ref().is_some_and(|id| photoncast_apps::is_app_running(id));

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

            match self.actions_menu_index {
                0 => {
                    // Open
                    self.show_actions_menu = false;
                    self.activate(&Activate, cx);
                },
                1 => {
                    // Show in Finder
                    self.show_actions_menu = false;
                    if let Some(path) = &app_path {
                        if let Err(e) = photoncast_apps::reveal_in_finder(path) {
                            tracing::error!("Failed to reveal in Finder: {}", e);
                        }
                    }
                    self.hide(cx);
                },
                2 => {
                    // Copy Path
                    self.show_actions_menu = false;
                    if let Some(path) = &app_path {
                        if let Err(e) = photoncast_apps::copy_path_to_clipboard(path) {
                            tracing::error!("Failed to copy path: {}", e);
                        } else {
                            tracing::info!("Copied path to clipboard");
                        }
                    }
                    cx.notify();
                },
                3 => {
                    // Copy Bundle ID
                    self.show_actions_menu = false;
                    if let Some(bundle_id) = &app_bundle_id {
                        if let Err(e) = photoncast_apps::copy_bundle_id_to_clipboard(bundle_id) {
                            tracing::error!("Failed to copy bundle ID: {}", e);
                        } else {
                            tracing::info!("Copied bundle ID to clipboard");
                        }
                    }
                    cx.notify();
                },
                4 => {
                    // Toggle Auto Quit - show settings modal to configure
                    self.show_actions_menu = false;
                    if let Some(bundle_id) = &app_bundle_id {
                        let is_enabled = {
                            self.auto_quit_manager.read().is_auto_quit_enabled(bundle_id)
                        };
                        if is_enabled {
                            // Disable directly
                            {
                                let mut manager = self.auto_quit_manager.write();
                                manager.disable_auto_quit(bundle_id);
                                let _ = manager.save();
                            }
                            tracing::info!("Disabled auto-quit for {}", bundle_id);
                            self.show_toast("Auto Quit disabled".to_string(), cx);
                        } else {
                            // Show settings modal to configure timeout
                            let app_name = selected_result.map(|r| r.title.clone()).unwrap_or_default();
                            self.show_auto_quit_settings(bundle_id, &app_name, cx);
                        }
                    }
                    cx.notify();
                },
                5 if is_running => {
                    // Quit
                    self.show_actions_menu = false;
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
                    self.show_actions_menu = false;
                    if let Some(bundle_id) = &app_bundle_id {
                        // Get the PID for the bundle ID
                        if let Ok(running_apps) = photoncast_apps::AppManager::new(photoncast_apps::AppsConfig::default()).get_running_apps() {
                            if let Some(app) = running_apps.iter().find(|a| a.bundle_id.as_deref() == Some(bundle_id)) {
                                #[allow(clippy::cast_possible_wrap)]
                                let pid = app.pid as i32;
                                match photoncast_apps::force_quit_app_action(pid) {
                                    Ok(()) => tracing::info!("Force quit app: {} (PID {})", bundle_id, pid),
                                    Err(e) => tracing::error!("Failed to force quit app: {}", e),
                                }
                            }
                        }
                    }
                    self.hide(cx);
                },
                7 if is_running => {
                    // Hide app
                    self.show_actions_menu = false;
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
                    self.show_actions_menu = false;
                    if let Some(path) = &app_path {
                        let app_name = selected_result.map(|r| r.title.clone()).unwrap_or_default();
                        tracing::info!("Starting uninstall flow for: {} at {:?}", app_name, path);
                        self.show_uninstall_preview(std::path::Path::new(path), cx);
                    }
                    cx.notify();
                },
                _ => {
                    self.show_actions_menu = false;
                    cx.notify();
                },
            }
            return;
        }

        // Map index to action based on current mode (non-app)
        // Order: Open, Copy Path, Copy File, [Reveal in Finder, Quick Look]
        match self.actions_menu_index {
            0 => {
                // Open
                self.show_actions_menu = false;
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
                self.show_actions_menu = false;
                cx.notify();
            },
        }
    }

    fn next_group(&mut self, _: &NextGroup, cx: &mut ViewContext<Self>) {
        // If file search view is in browsing mode, use Tab to enter folder
        if let Some(file_search_view) = &self.file_search_view {
            let is_browsing = file_search_view.read(cx).section_mode == crate::file_search_view::SectionMode::Browsing;
            if is_browsing {
                file_search_view.update(cx, |view, cx| {
                    view.browse_enter_folder(cx);
                });
                return;
            }
        }
        
        // Check if we should autocomplete a quicklink instead of navigating groups
        // This happens when: there's only 1 result OR the selected result is a quicklink that needs input
        if let Some(core_result) = self.core_results.get(self.selected_index) {
            if let SearchAction::ExecuteQuickLink { url_template, .. } = &core_result.action {
                if photoncast_quicklinks::placeholder::requires_user_input(url_template) {
                    // Only 1 result, or quicklink is selected - autocomplete it
                    if self.results.len() == 1 || self.selected_index == 0 {
                        let autocomplete = if let Some(alias_match) = core_result.subtitle.strip_prefix('/') {
                            alias_match.split(" · ").next().unwrap_or(&core_result.title).to_string()
                        } else {
                            core_result.title.clone()
                        };
                        let new_query = format!("{} ", autocomplete);
                        self.cursor_position = new_query.chars().count();
                        self.selection_anchor = None;
                        self.query = SharedString::from(new_query);
                        self.on_query_change(self.query.clone(), cx);
                        self.reset_cursor_blink();
                        cx.notify();
                        return;
                    }
                }
            }
        }

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
        // If file search view is in browsing mode, use Shift+Tab to go to parent directory
        if let Some(file_search_view) = &self.file_search_view {
            let is_browsing = file_search_view.read(cx).section_mode == crate::file_search_view::SectionMode::Browsing;
            if is_browsing {
                file_search_view.update(cx, |view, cx| {
                    view.browse_go_back(cx);
                });
                return;
            }
        }
        
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
    fn ensure_selected_visible(&self, cx: &mut ViewContext<Self>) {
        // Calculate item height based on mode
        let item_height = if matches!(self.search_mode, SearchMode::Calendar { .. }) {
            56.0 // Calendar items are taller
        } else {
            RESULT_ITEM_HEIGHT.0
        };

        // Calculate visible area height
        let visible_height = (MAX_VISIBLE_RESULTS as f32) * item_height;

        // Calculate the top position of the selected item
        // For calendar mode, we need to account for day headers
        let item_top = if let SearchMode::Calendar { events, .. } = &self.search_mode {
            // Count day headers before this item
            let mut header_count = 0;
            let mut current_day: Option<photoncast_calendar::chrono::NaiveDate> = None;
            for (i, event) in events.iter().enumerate() {
                let event_day = event.start.date_naive();
                if current_day != Some(event_day) {
                    current_day = Some(event_day);
                    header_count += 1;
                }
                if i >= self.selected_index {
                    break;
                }
            }
            (self.selected_index as f32 * item_height) + (header_count as f32 * 28.0)
        } else {
            self.selected_index as f32 * item_height
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

    fn open_preferences(&mut self, _: &OpenPreferences, cx: &mut ViewContext<Self>) {
        if let Err(e) = app_events::send_event(AppEvent::OpenPreferences) {
            tracing::error!("Failed to send preferences event: {}", e);
        }
        self.hide(cx);
    }

    /// Get the current selection range (start, end) where start <= end
    fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            if anchor <= self.cursor_position {
                (anchor, self.cursor_position)
            } else {
                (self.cursor_position, anchor)
            }
        })
    }

    /// Delete selected text and return the new query, or None if no selection
    fn delete_selection(&mut self) -> Option<String> {
        if let Some((start, end)) = self.selection_range() {
            let chars: Vec<char> = self.query.chars().collect();
            let new_query: String = chars[..start].iter().chain(chars[end..].iter()).collect();
            self.cursor_position = start;
            self.selection_anchor = None;
            Some(new_query)
        } else {
            None
        }
    }

    /// Find the previous word boundary from the given position
    fn prev_word_boundary(&self, pos: usize) -> usize {
        let chars: Vec<char> = self.query.chars().collect();
        if pos == 0 {
            return 0;
        }
        let mut i = pos - 1;
        // Skip whitespace
        while i > 0 && chars[i].is_whitespace() {
            i -= 1;
        }
        // Skip word characters
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        i
    }

    /// Find the next word boundary from the given position
    fn next_word_boundary(&self, pos: usize) -> usize {
        let chars: Vec<char> = self.query.chars().collect();
        let len = chars.len();
        if pos >= len {
            return len;
        }
        let mut i = pos;
        // Skip current word
        while i < len && !chars[i].is_whitespace() {
            i += 1;
        }
        // Skip whitespace
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        i
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        // If file search view is active, forward all key events to it
        if let Some(file_search_view) = &self.file_search_view {
            file_search_view.update(cx, |view, cx| {
                view.handle_key_down(event, cx);
            });
            return;
        }

        let key = event.keystroke.key.as_str();
        let shift = event.keystroke.modifiers.shift;
        let cmd = event.keystroke.modifiers.platform;
        let chars: Vec<char> = self.query.chars().collect();
        let len = chars.len();

        // Handle Tab for quicklink autocomplete
        if key == "tab" && !shift {
            if let Some(core_result) = self.core_results.get(self.selected_index) {
                if let SearchAction::ExecuteQuickLink { url_template, .. } = &core_result.action {
                    if photoncast_quicklinks::placeholder::requires_user_input(url_template) {
                        let autocomplete = if let Some(alias_match) = core_result.subtitle.strip_prefix('/') {
                            alias_match.split(" · ").next().unwrap_or(&core_result.title)
                        } else {
                            &core_result.title
                        };
                        let new_query = format!("{} ", autocomplete);
                        self.cursor_position = new_query.chars().count();
                        self.selection_anchor = None;
                        self.query = SharedString::from(new_query);
                        self.on_query_change(self.query.clone(), cx);
                        self.reset_cursor_blink();
                        cx.notify();
                    }
                }
            }
            return;
        }

        // Cmd+A: Select all
        if cmd && key == "a" {
            if !self.query.is_empty() {
                self.selection_anchor = Some(0);
                self.cursor_position = len;
                self.reset_cursor_blink();
                cx.notify();
            }
            return;
        }

        // Cmd+C: Copy selection
        if cmd && key == "c" {
            if let Some((start, end)) = self.selection_range() {
                let selected: String = chars[start..end].iter().collect();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
            }
            return;
        }

        // Cmd+X: Cut selection
        if cmd && key == "x" {
            if let Some((start, end)) = self.selection_range() {
                let selected: String = chars[start..end].iter().collect();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
                if let Some(new_query) = self.delete_selection() {
                    self.query = SharedString::from(new_query);
                    self.on_query_change(self.query.clone(), cx);
                    self.reset_cursor_blink();
                    cx.notify();
                }
            }
            return;
        }

        // Cmd+V: Paste
        if cmd && key == "v" {
            if let Some(clipboard) = cx.read_from_clipboard() {
                if let Some(text) = clipboard.text() {
                    // Delete selection first if any
                    if self.selection_anchor.is_some() {
                        if let Some(new_query) = self.delete_selection() {
                            self.query = SharedString::from(new_query);
                        }
                    }
                    // Insert at cursor
                    let chars: Vec<char> = self.query.chars().collect();
                    let before: String = chars[..self.cursor_position].iter().collect();
                    let after: String = chars[self.cursor_position..].iter().collect();
                    let new_query = format!("{}{}{}", before, text, after);
                    self.cursor_position += text.chars().count();
                    self.query = SharedString::from(new_query);
                    self.on_query_change(self.query.clone(), cx);
                    self.reset_cursor_blink();
                    cx.notify();
                }
            }
            return;
        }

        let alt = event.keystroke.modifiers.alt;

        // Arrow keys for cursor movement and selection
        if key == "left" {
            if cmd && shift {
                // Cmd+Shift+Left: Select to beginning
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some(self.cursor_position);
                }
                self.cursor_position = 0;
            } else if alt && shift {
                // Option+Shift+Left: Select word left
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some(self.cursor_position);
                }
                self.cursor_position = self.prev_word_boundary(self.cursor_position);
            } else if cmd {
                // Cmd+Left: Move to beginning
                self.cursor_position = 0;
                self.selection_anchor = None;
            } else if alt {
                // Option+Left: Move word left
                self.cursor_position = self.prev_word_boundary(self.cursor_position);
                self.selection_anchor = None;
            } else if shift {
                // Shift+Left: Extend selection left
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some(self.cursor_position);
                }
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            } else {
                // Left: Move cursor left (collapse selection if any)
                if self.selection_anchor.is_some() {
                    if let Some((start, _)) = self.selection_range() {
                        self.cursor_position = start;
                    }
                    self.selection_anchor = None;
                } else if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            self.reset_cursor_blink();
            cx.notify();
            return;
        }

        if key == "right" {
            if cmd && shift {
                // Cmd+Shift+Right: Select to end
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some(self.cursor_position);
                }
                self.cursor_position = len;
            } else if alt && shift {
                // Option+Shift+Right: Select word right
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some(self.cursor_position);
                }
                self.cursor_position = self.next_word_boundary(self.cursor_position);
            } else if cmd {
                // Cmd+Right: Move to end
                self.cursor_position = len;
                self.selection_anchor = None;
            } else if alt {
                // Option+Right: Move word right
                self.cursor_position = self.next_word_boundary(self.cursor_position);
                self.selection_anchor = None;
            } else if shift {
                // Shift+Right: Extend selection right
                if self.selection_anchor.is_none() {
                    self.selection_anchor = Some(self.cursor_position);
                }
                if self.cursor_position < len {
                    self.cursor_position += 1;
                }
            } else {
                // Right: Move cursor right (collapse selection if any)
                if self.selection_anchor.is_some() {
                    if let Some((_, end)) = self.selection_range() {
                        self.cursor_position = end;
                    }
                    self.selection_anchor = None;
                } else if self.cursor_position < len {
                    self.cursor_position += 1;
                }
            }
            self.reset_cursor_blink();
            cx.notify();
            return;
        }

        // Backspace: Delete selection or character/word before cursor
        if key == "backspace" {
            if let Some(new_query) = self.delete_selection() {
                self.query = SharedString::from(new_query);
                self.on_query_change(self.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            } else if alt && self.cursor_position > 0 {
                // Option+Backspace: Delete word
                let word_start = self.prev_word_boundary(self.cursor_position);
                let new_query: String = chars[..word_start]
                    .iter()
                    .chain(chars[self.cursor_position..].iter())
                    .collect();
                self.cursor_position = word_start;
                self.query = SharedString::from(new_query);
                self.on_query_change(self.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            } else if self.cursor_position > 0 {
                let new_query: String = chars[..self.cursor_position - 1]
                    .iter()
                    .chain(chars[self.cursor_position..].iter())
                    .collect();
                self.cursor_position -= 1;
                self.query = SharedString::from(new_query);
                self.on_query_change(self.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            }
            return;
        }

        // Delete: Delete selection or character after cursor
        if key == "delete" {
            if let Some(new_query) = self.delete_selection() {
                self.query = SharedString::from(new_query);
                self.on_query_change(self.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            } else if self.cursor_position < len {
                let new_query: String = chars[..self.cursor_position]
                    .iter()
                    .chain(chars[self.cursor_position + 1..].iter())
                    .collect();
                self.query = SharedString::from(new_query);
                self.on_query_change(self.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            }
            return;
        }

        // Ignore other modifier combinations (except shift for uppercase)
        if cmd || event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
            return;
        }

        // Handle regular character input
        let input_text = if let Some(ime_key) = &event.keystroke.ime_key {
            Some(ime_key.clone())
        } else if key.len() == 1 {
            let ch = if shift {
                key.to_uppercase()
            } else {
                key.to_string()
            };
            Some(ch)
        } else {
            None
        };

        if let Some(text) = input_text {
            // Delete selection first if any
            let chars: Vec<char> = if self.selection_anchor.is_some() {
                if let Some(new_query) = self.delete_selection() {
                    self.query = SharedString::from(new_query);
                }
                self.query.chars().collect()
            } else {
                chars
            };

            // Insert at cursor
            let before: String = chars[..self.cursor_position].iter().collect();
            let after: String = chars[self.cursor_position..].iter().collect();
            let new_query = format!("{}{}{}", before, text, after);
            self.cursor_position += text.chars().count();
            self.query = SharedString::from(new_query);
            self.on_query_change(self.query.clone(), cx);
            self.reset_cursor_blink();
            cx.notify();
        }
    }

    /// Render the query text with cursor and selection highlighting
    fn render_query_with_cursor(&self, colors: &LauncherColors, placeholder: &str) -> impl IntoElement {
        let text_color = colors.text;
        let placeholder_color = colors.text_placeholder;
        let selection_bg = colors.accent.opacity(0.3);
        let cursor_color = colors.accent;
        let show_cursor = self.cursor_visible();

        // Block cursor dimensions (like Ghostty terminal)
        let cursor_width = px(9.0);
        let cursor_height = px(20.0);

        if self.query.is_empty() {
            // Show block cursor at start (no placeholder text)
            return div()
                .w_full()
                .text_size(px(16.0))
                .flex()
                .items_center()
                .when(show_cursor, |el| {
                    el.child(
                        div()
                            .w(cursor_width)
                            .h(cursor_height)
                            .bg(cursor_color)
                            .rounded(px(2.0)),
                    )
                })
                .when(!placeholder.is_empty(), |el| {
                    el.child(
                        div()
                            .text_color(placeholder_color)
                            .child(placeholder.to_string()),
                    )
                });
        }

        let chars: Vec<char> = self.query.chars().collect();
        let (sel_start, sel_end) = self.selection_range().unwrap_or((self.cursor_position, self.cursor_position));

        // Build the text parts: before selection, selection, after selection
        let before: String = chars[..sel_start].iter().collect();
        let selected: String = chars[sel_start..sel_end].iter().collect();
        let after: String = chars[sel_end..].iter().collect();

        let has_selection = sel_start != sel_end;
        let cursor_at_start = self.cursor_position == sel_start;

        div()
            .w_full()
            .text_size(px(16.0))
            .text_color(text_color)
            .flex()
            .items_center()
            // Text before selection
            .when(!before.is_empty(), |el| el.child(before.clone()))
            // Cursor before selection (if selection exists and cursor is at start)
            .when(has_selection && cursor_at_start && show_cursor, |el| {
                el.child(
                    div()
                        .w(cursor_width)
                        .h(cursor_height)
                        .bg(cursor_color)
                        .rounded(px(2.0)),
                )
            })
            // Cursor at position (if no selection)
            .when(!has_selection && before.is_empty() && after.is_empty(), |el| {
                // Cursor after text when query is non-empty but cursor at end
                el.child(self.query.clone())
                    .when(show_cursor, |el| {
                        el.child(
                            div()
                                .w(cursor_width)
                                .h(cursor_height)
                                .bg(cursor_color)
                                .rounded(px(2.0)),
                        )
                    })
            })
            .when(!has_selection && (!before.is_empty() || !after.is_empty()) && show_cursor, |el| {
                // Cursor in the middle
                el.child(
                    div()
                        .w(cursor_width)
                        .h(cursor_height)
                        .bg(cursor_color)
                        .rounded(px(2.0)),
                )
            })
            // Selected text with background
            .when(!selected.is_empty(), |el| {
                el.child(
                    div()
                        .bg(selection_bg)
                        .rounded(px(2.0))
                        .child(selected.clone()),
                )
            })
            // Cursor after selection (if selection exists and cursor is at end)
            .when(has_selection && !cursor_at_start && show_cursor, |el| {
                el.child(
                    div()
                        .w(cursor_width)
                        .h(cursor_height)
                        .bg(cursor_color)
                        .rounded(px(2.0)),
                )
            })
            // Text after selection
            .when(!after.is_empty(), |el| el.child(after.clone()))
    }

    /// Render the search bar component
    fn render_search_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement + '_ {
        let colors = get_launcher_colors(cx);
        // Determine icon and placeholder based on search mode
        let (icon, placeholder) = match &self.search_mode {
            SearchMode::Normal => ("🔍", ""),
            SearchMode::FileSearch => ("📁", ""),
            SearchMode::Calendar { title, .. } => ("📅", title.as_str()),
        };
        let placeholder = placeholder.to_string();
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;

        // Suppress unused variable warnings
        let _ = icon;
        let _ = text_placeholder;

        div()
            .h(SEARCH_BAR_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .child(
                // Search input with cursor and selection (no icon)
                div().flex_1().h_full().flex().items_center().child(
                    self.render_query_with_cursor(&colors, &placeholder),
                ),
            )
            // Show "esc to exit" hint in file search mode
            .when(matches!(self.search_mode, SearchMode::FileSearch), move |el| {
                el.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(text_muted)
                        .child("esc to exit"),
                )
            })
            .when(matches!(self.search_mode, SearchMode::Calendar { .. }), move |el| {
                el.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(text_muted)
                        .child("esc to go back"),
                )
            })
    }

    /// Render the results list component with grouping
    fn render_results(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        if let SearchMode::Calendar {
            title,
            events,
            error,
        } = &self.search_mode
        {
            if let Some(message) = error {
                return div()
                    .id("results-list-calendar")
                    .w_full()
                    .py_4()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(self.render_group_header(ResultType::Command, &colors))
                    .child(
                        div()
                            .px_4()
                            .py_1()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child(message.clone()),
                    );
            }

            if events.is_empty() {
                return div()
                    .id("results-list-calendar")
                    .w_full()
                    .py_4()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(self.render_group_header(ResultType::Command, &colors))
                    .child(
                        div()
                            .px_4()
                            .py_1()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child("No events found"),
                    );
            }

            let selected = self.selected_index;
            let now = photoncast_calendar::chrono::Local::now();
            let today = now.date_naive();
            let tomorrow = today + photoncast_calendar::chrono::Duration::days(1);

            // Group events by day and build elements with day headers
            let mut elements: Vec<gpui::AnyElement> = Vec::new();
            let mut current_day: Option<photoncast_calendar::chrono::NaiveDate> = None;
            let mut item_index = 0usize;

            for event in events.iter() {
                let event_day = event.start.date_naive();

                // Add day header if day changed
                if current_day != Some(event_day) {
                    current_day = Some(event_day);
                    let day_label = if event_day == today {
                        "Today".to_string()
                    } else if event_day == tomorrow {
                        "Tomorrow".to_string()
                    } else {
                        event_day.format("%A, %B %d").to_string()
                    };
                    elements.push(
                        div()
                            .id(SharedString::from(format!("day-header-{}", event_day)))
                            .w_full()
                            .px_4()
                            .pt_3()
                            .pb_1()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text_muted)
                                    .child(day_label),
                            )
                            .into_any_element(),
                    );
                }

                let idx = item_index;
                let is_selected = idx == selected;
                let has_conference = event.conference_url.is_some();

                // Build time string with relative time
                let time_str = if event.is_all_day {
                    "All day".to_string()
                } else {
                    format!("{} - {}", event.start.format("%H:%M"), event.end.format("%H:%M"))
                };

                // Calculate relative time
                let relative_time = if event.is_happening_now() {
                    Some("now".to_string())
                } else if event.starts_within_minutes(5) {
                    Some("in 5 min".to_string())
                } else if event.starts_within_minutes(15) {
                    let duration = event.start.signed_duration_since(now);
                    let mins = duration.num_minutes();
                    Some(format!("in {} min", mins))
                } else if event.starts_within_minutes(60) {
                    let duration = event.start.signed_duration_since(now);
                    let mins = duration.num_minutes();
                    Some(format!("in {} min", mins))
                } else {
                    None
                };

                // Parse calendar color (hex string like "#0088FF")
                let cal_color = Self::parse_hex_color(&event.calendar_color);

                let event_element = div()
                    .id(SharedString::from(format!("cal-event-{idx}")))
                    .min_h(px(52.0))
                    .w_full()
                    .px_4()
                    .py_1()
                    .flex()
                    .items_center()
                    .gap_3()
                    .rounded(px(6.0))
                    .mx(px(4.0))
                    .bg(if is_selected {
                        colors.selection
                    } else {
                        gpui::transparent_black()
                    })
                    // Calendar color dot
                    .child(
                        div()
                            .size(px(8.0))
                            .rounded(px(4.0))
                            .bg(cal_color)
                            .flex_shrink_0(),
                    )
                    // Video icon for conference meetings
                    .child(
                        div()
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.0))
                            .text_size(px(18.0))
                            .child(if has_conference { "📹" } else { "📅" }),
                    )
                    // Event details
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .overflow_hidden()
                            // Title row
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.text)
                                            .truncate()
                                            .child(event.title.clone()),
                                    )
                                    .when(relative_time.is_some(), |el| {
                                        let rt = relative_time.clone().unwrap();
                                        // Use themed colors for time indicators (success=now, warning=soon)
                                        let color = if rt == "now" { colors.success } else { colors.warning };
                                        el.child(
                                            div()
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(color)
                                                .flex_shrink_0()
                                                .child(rt),
                                        )
                                    }),
                            )
                            // Time and calendar row
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(colors.text_muted)
                                            .child(time_str),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(colors.text_placeholder)
                                            .child(format!("· {}", event.calendar_name)),
                                    ),
                            )
                            // Location row (if present)
                            .when(event.location.is_some(), {
                                let text_placeholder = colors.text_placeholder;
                                move |el| {
                                    let loc = event.location.clone().unwrap_or_default();
                                    // Don't show location if it's a URL (conference link)
                                    if !loc.starts_with("http") && !loc.is_empty() {
                                        el.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(4.0))
                                                .child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(text_placeholder)
                                                        .child("📍"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(text_placeholder)
                                                        .truncate()
                                                        .child(loc),
                                                ),
                                        )
                                    } else {
                                        el
                                    }
                                }
                            }),
                    )
                    // Join hint on right side
                    .when(has_conference && is_selected, {
                        let accent = colors.accent;
                        move |el| {
                            el.child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(accent)
                                    .flex_shrink_0()
                                    .child("↵ join"),
                            )
                        }
                    });

                elements.push(event_element.into_any_element());
                item_index += 1;
            }

            // Calculate height based on items + headers
            let num_headers = {
                let mut days = std::collections::HashSet::new();
                for e in events {
                    days.insert(e.start.date_naive());
                }
                days.len()
            };
            let header_height = num_headers as f32 * 28.0;
            let items_height = events.len() as f32 * 56.0; // Slightly taller items
            let total_height = (header_height + items_height)
                .min((MAX_VISIBLE_RESULTS as f32 * 56.0) + 56.0);

            return div()
                .id("results-list-calendar")
                .w_full()
                .h(px(total_height))
                .overflow_y_scroll()
                .track_scroll(&self.results_scroll_handle)
                .child(self.render_group_header(ResultType::Command, &colors))
                .child(
                    div()
                        .px_4()
                        .py_1()
                        .text_size(px(12.0))
                        .text_color(colors.text_muted)
                        .child(title.clone()),
                )
                .children(elements);
        }

        // Check if we're showing suggestions (query is empty)
        let is_suggestions = self.query.is_empty() && !self.suggestions.is_empty();
        
        // Group results by type
        let mut current_type: Option<ResultType> = None;
        let mut elements: Vec<gpui::AnyElement> = Vec::new();
        let mut shown_suggestions_header = false;

        for (idx, result) in self.results.iter().enumerate() {
            // Add group header when type changes
            if current_type != Some(result.result_type) {
                current_type = Some(result.result_type);
                
                // Show "Suggestions" header instead of type when showing suggestions
                if is_suggestions && !shown_suggestions_header {
                    shown_suggestions_header = true;
                    elements.push(
                        self.render_suggestions_header(&colors)
                            .into_any_element(),
                    );
                } else if !is_suggestions {
                    elements.push(
                        self.render_group_header(result.result_type, &colors)
                            .into_any_element(),
                    );
                }
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
            .track_scroll(&self.results_scroll_handle)
            .children(elements)
    }

    /// Render a group header (e.g., "Apps", "Commands")
    fn render_group_header(&self, result_type: ResultType, colors: &LauncherColors) -> impl IntoElement {
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
                    .text_color(colors.text_muted)
                    .child(result_type.display_name().to_uppercase()),
            )
    }

    /// Render a "Suggestions" header for empty query state
    fn render_suggestions_header(&self, colors: &LauncherColors) -> impl IntoElement {
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
                    .text_color(colors.text_muted)
                    .child("SUGGESTIONS"),
            )
    }

    /// Render the icon for a result item with status indicators
    fn render_icon(&self, result: &ResultItem) -> impl IntoElement {
        let icon_size = px(32.0);

        // Check if app is running and has auto-quit enabled
        let is_running = result.bundle_id.as_ref().is_some_and(|id| {
            photoncast_apps::is_app_running(id)
        });
        let has_auto_quit = result.bundle_id.as_ref().is_some_and(|id| {
            self.auto_quit_manager.read().is_auto_quit_enabled(id)
        });

        div()
            .relative()
            .size(icon_size)
            .child(
                // Main icon
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
                    }),
            )
            // Task 7.1: Running app indicator (8px green dot, bottom-right)
            .when(is_running, |el| {
                el.child(
                    div()
                        .absolute()
                        .bottom(px(-2.0))
                        .right(px(-2.0))
                        .size(px(8.0))
                        .rounded_full()
                        .bg(hsla(120.0 / 360.0, 1.0, 0.5, 1.0)) // Green #00FF00
                        .border_1()
                        .border_color(hsla(0.0, 0.0, 0.1, 1.0)), // Dark border for visibility
                )
            })
            // Task 7.2: Auto Quit indicator (orange dot, below green dot or bottom-right if not running)
            .when(has_auto_quit, |el| {
                let offset = if is_running { px(-10.0) } else { px(-2.0) };
                el.child(
                    div()
                        .absolute()
                        .bottom(offset)
                        .right(px(-2.0))
                        .size(px(6.0))
                        .rounded_full()
                        .bg(hsla(30.0 / 360.0, 1.0, 0.5, 1.0)) // Orange
                        .border_1()
                        .border_color(hsla(0.0, 0.0, 0.1, 1.0)), // Dark border for visibility
                )
            })
    }

    /// Render a single result item
    fn render_result_item(
        &self,
        result: &ResultItem,
        index: usize,
        is_selected: bool,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let selected_bg = colors.selection;
        let hover_bg = colors.surface_hover;
        div()
            .id(("result-item", index))
            .h(RESULT_ITEM_HEIGHT)
            .w_full()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .when(is_selected, move |el| el.bg(selected_bg))
            .hover(move |el| el.bg(hover_bg))
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
                            .text_color(colors.text)
                            .truncate()
                            .child(result.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .truncate()
                            .child(result.subtitle.clone()),
                    ),
            )
            .child({
                // Shortcut badge - offset by 1 when meeting is visible (meeting takes ⌘1)
                let has_meeting = self.query.is_empty() && self.next_meeting.is_some();
                let shortcut_num = if has_meeting { index + 2 } else { index + 1 };
                div()
                    .text_size(px(12.0))
                    .text_color(colors.text_placeholder)
                    .when(shortcut_num <= 9, |el| el.child(format!("⌘{}", shortcut_num)))
            })
    }

    /// Render empty state hint when nothing to show
    fn render_empty_state(&self, colors: &LauncherColors) -> AnyElement {
        // Show loading indicator during file search
        if self.file_search_loading && matches!(self.search_mode, SearchMode::FileSearch) {
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
                        .text_color(colors.text_muted)
                        .child("Searching..."),
                )
                .into_any_element();
        }

        // For non-normal modes, show simple hints
        if !matches!(self.search_mode, SearchMode::Normal) {
            let (message, hints) = match &self.search_mode {
                SearchMode::FileSearch => (
                    "Type at least 2 characters to search files".to_string(),
                    "↵ Open  ⌘↵ Reveal  ⌘Y Quick Look  esc Exit",
                ),
                SearchMode::Calendar { error, .. } => (
                    error
                        .as_ref()
                        .map_or("No events found", |msg| msg.as_str())
                        .to_string(),
                    "esc Back to search",
                ),
                SearchMode::Normal => unreachable!(),
            };

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
                        .text_color(colors.text_muted)
                        .child(message),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(colors.text_placeholder)
                        .child(hints),
                )
                .into_any_element();
        }

        // Default hint when no meeting, no suggestions
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
                    .text_color(colors.text_muted)
                    .child("Type to search apps and commands"),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(colors.text_placeholder)
                    .child("↑↓ Navigate  ↵ Open  esc Close"),
            )
            .into_any_element()
    }

    /// Render the next meeting widget at the top of the launcher
    fn render_next_meeting(
        &self,
        meeting: &photoncast_calendar::CalendarEvent,
        colors: &LauncherColors,
    ) -> impl IntoElement {
        let now = photoncast_calendar::chrono::Local::now();
        let time_until = meeting.start.signed_duration_since(now);

        // Format time display
        let time_str = if time_until.num_minutes() < 0 {
            // Meeting in progress
            "Now".to_string()
        } else if time_until.num_minutes() < 60 {
            format!("in {} min", time_until.num_minutes())
        } else if time_until.num_hours() < 24 {
            meeting.start.format("%H:%M").to_string()
        } else {
            meeting.start.format("%a %H:%M").to_string()
        };

        // Check if meeting is happening now or starting soon (within 15 min)
        let is_urgent = time_until.num_minutes() <= 15;
        let is_selected = self.meeting_selected && self.query.is_empty();
        
        let bg_color = if is_selected {
            colors.selection
        } else if is_urgent {
            colors.accent.opacity(0.3) // Accent tint for urgent
        } else {
            colors.surface_hover
        };

        let has_meeting_link = meeting.conference_url.is_some();
        let text = colors.text;
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;
        let accent = colors.accent;
        let border = colors.border;

        div()
            .id("next-meeting")
            .mx(px(8.0))
            .mt(px(8.0))
            .mb(px(4.0))
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(8.0))
            .bg(bg_color)
            .border_1()
            .border_color(if is_selected {
                accent
            } else {
                border
            })
            .cursor_pointer()
            .flex()
            .items_center()
            .gap(px(12.0))
            // Calendar icon
            .child(
                div()
                    .size(px(32.0))
                    .rounded(px(6.0))
                    .bg(colors.accent.opacity(0.2))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(16.0))
                    .child("📅"),
            )
            // Meeting info
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(text)
                            .truncate()
                            .child(meeting.title.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(if is_urgent {
                                        accent
                                    } else {
                                        text_muted
                                    })
                                    .child(time_str),
                            )
                            .when(has_meeting_link, move |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(text_placeholder)
                                        .child("↵ to join"),
                                )
                            }),
                    ),
            )
            // Join button (if has meeting link)
            .when(has_meeting_link, move |el| {
                el.child(
                    div()
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .bg(accent)
                        .text_size(px(11.0))
                        .text_color(text)
                        .child("Join"),
                )
            })
            // Shortcut badge
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(text_placeholder)
                    .child("⌘1"),
            )
    }

    /// Render the suggestions section
    #[allow(dead_code)]
    fn render_suggestions(&self, colors: &LauncherColors) -> impl IntoElement {
        let surface_hover = colors.surface_hover;
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;
        let items: Vec<_> = self
            .suggestions
            .iter()
            .take(6)
            .map(|result| {
                let icon_path = match &result.icon {
                    IconSource::FileIcon { path } => Self::get_app_icon_path(path),
                    IconSource::AppIcon { icon_path, .. } => icon_path.clone(),
                    _ => None,
                };

                div()
                    .id(SharedString::from(result.id.to_string()))
                    .w(px(72.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.0))
                    .py(px(8.0))
                    .px(px(4.0))
                    .rounded(px(8.0))
                    .cursor_pointer()
                    .hover(move |s| s.bg(surface_hover))
                    .child(
                        div()
                            .size(px(40.0))
                            .rounded(px(8.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .overflow_hidden()
                            .map(|el| {
                                if let Some(icon) = &icon_path {
                                    el.child(
                                        img(icon.clone())
                                            .size(px(40.0))
                                            .object_fit(ObjectFit::Contain),
                                    )
                                } else {
                                    el.text_size(px(24.0)).child("📱")
                                }
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(text_muted)
                            .truncate()
                            .max_w(px(68.0))
                            .child(result.title.clone()),
                    )
            })
            .collect();

        div()
            .w_full()
            .px(px(8.0))
            .pt(px(4.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            // Section header
            .child(
                div()
                    .px(px(8.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text_placeholder)
                    .child("SUGGESTIONS"),
            )
            // App grid
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_start()
                    .children(items),
            )
    }

    /// Render "no results" state
    fn render_no_results(&self, colors: &LauncherColors) -> impl IntoElement + '_ {
        let (message, hint) = match &self.search_mode {
            SearchMode::Calendar { error, .. } => {
                let msg = error
                    .as_ref()
                    .map_or("No events found", |msg| msg.as_str())
                    .to_string();
                (msg, "esc Back to search")
            },
            SearchMode::FileSearch => {
                if self.file_search_loading {
                    ("Searching...".to_string(), "")
                } else {
                    (
                        format!("No files found for \"{}\"", self.query),
                        "↵ Open  ⌘↵ Reveal  ⌘Y Quick Look  esc Exit",
                    )
                }
            },
            SearchMode::Normal => (
                format!("No results for \"{}\"", self.query),
                "Try a different search term",
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
                    .text_color(colors.text_muted)
                    .child(message),
            )
            .when(!hint.is_empty(), |el| {
                el.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(colors.text_placeholder)
                        .child(hint),
                )
            })
    }

    /// Render the confirmation dialog overlay
    fn render_confirmation_dialog(
        &self,
        dialog: &ConfirmationDialog,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        // Full overlay with semi-transparent background
        div()
            .id("confirmation-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            // Theme overlay color
            .bg(colors.overlay)
            .child(
                // Dialog container
                div()
                    .id("confirmation-dialog")
                    .w(px(340.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    // Theme surface elevated background
                    .bg(colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
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
                                    // Warning background with low opacity
                                    .bg(colors.warning.opacity(0.15))
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
                                    .text_color(colors.text)
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
                                    .text_color(colors.text_muted)
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
                            .child({
                                let hover_bg = colors.surface_hover;
                                div()
                                    .id("cancel-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(colors.surface)
                                    .hover(move |el| el.bg(hover_bg))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.text)
                                            .child(dialog.cancel_label.clone()),
                                    )
                            })
                            // Confirm button (destructive style)
                            .child({
                                let error_color = colors.error;
                                // Lighten by increasing lightness component
                                let error_hover = hsla(error_color.h, error_color.s, (error_color.l + 0.1).min(1.0), error_color.a);
                                div()
                                    .id("confirm-button")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(error_color)
                                    .hover(move |el| el.bg(error_hover))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(colors.text)
                                            .child(dialog.confirm_label.clone()),
                                    )
                            }),
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
                                    .text_color(colors.text_placeholder)
                                    .child("↵ Confirm  esc Cancel"),
                            ),
                    ),
            )
    }

    /// Render the action bar at the bottom (Raycast-style with primary action and shortcuts)
    fn render_action_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        // Determine primary action based on mode
        let (primary_action, primary_shortcut) = if let SearchMode::Calendar { events, .. } = &self.search_mode {
            if let Some(event) = events.get(self.selected_index) {
                if event.conference_url.is_some() {
                    ("Join Meeting", "↵")
                } else {
                    ("Open in Calendar", "↵")
                }
            } else {
                ("", "")
            }
        } else if !self.results.is_empty() {
            ("Open", "↵")
        } else {
            ("", "")
        };
        
        let surface = colors.surface;
        let text_muted = colors.text_muted;
        let text_placeholder = colors.text_placeholder;
        let border = colors.border;

        div()
            .w_full()
            .h(px(36.0))
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(border)
            .bg(colors.surface)
            // Left side: Primary action
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(!primary_action.is_empty(), move |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(4.0))
                                        .bg(surface)
                                        .text_size(px(10.0))
                                        .text_color(text_muted)
                                        .child(primary_shortcut),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child(primary_action),
                                ),
                        )
                    }),
            )
            // Right side: Actions shortcut
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_placeholder)
                                    .child("Actions"),
                            )
                            .child(
                                div()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(surface)
                                    .text_size(px(10.0))
                                    .text_color(text_muted)
                                    .child("⌘K"),
                            ),
                    ),
            )
    }

    /// Render the actions menu popup (Cmd+K)
    fn render_actions_menu(&self, cx: &ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        // Determine available actions based on search mode and selection
        let is_file_mode = matches!(self.search_mode, SearchMode::FileSearch);
        let is_calendar_mode = matches!(self.search_mode, SearchMode::Calendar { .. });
        let has_selection = if is_calendar_mode {
            if let SearchMode::Calendar { events, .. } = &self.search_mode {
                !events.is_empty()
            } else {
                false
            }
        } else {
            !self.results.is_empty()
        };
        let selected = self.actions_menu_index;

        // For calendar mode, check if selected event has conference
        let has_conference = if let SearchMode::Calendar { events, .. } = &self.search_mode {
            events.get(self.selected_index).is_some_and(|e| e.conference_url.is_some())
        } else {
            false
        };

        // Task 7.3: Check if selected result is an app and if it's running
        let selected_result = self.results.get(self.selected_index);
        let is_app = selected_result.is_some_and(|r| r.result_type == ResultType::Application);
        let app_bundle_id = selected_result.and_then(|r| r.bundle_id.clone());
        let is_running = app_bundle_id.as_ref().is_some_and(|id| photoncast_apps::is_app_running(id));
        let has_auto_quit = app_bundle_id.as_ref().is_some_and(|id| {
            self.auto_quit_manager.read().is_auto_quit_enabled(id)
        });

        div()
            // Overlay background - position menu above action bar at bottom-right
            .absolute()
            .inset_0()
            .flex()
            .items_end()
            .justify_end()
            .pb(px(8.0)) // Small padding from bottom
            .pr_2()
            // Click outside to close
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, cx| {
                this.show_actions_menu = false;
                cx.notify();
            }))
            .child(
                div()
                    .w(px(300.0))
                    .bg(colors.surface_elevated)
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(colors.border)
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
                            .border_color(colors.border)
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text)
                            .child("Actions"),
                    )
                    // Action items with selection highlighting (scrollable for long lists)
                    .child(
                        div()
                            .id("actions-menu-list")
                            .py_1()
                            .max_h(px(420.0)) // Fits within 475px modal
                            .overflow_y_scroll()
                            .when(is_calendar_mode, |el| {
                                // Calendar actions: Join Meeting (if available), Copy Title, Copy Details, Open in Calendar
                                let mut idx = 0;
                                let el = if has_conference {
                                    let e = el.child(self.render_action_item("Join Meeting", "↵", has_selection, selected == idx, &colors));
                                    idx += 1;
                                    e
                                } else {
                                    el
                                };
                                el.child(self.render_action_item("Copy Title", "⌘C", has_selection, selected == idx, &colors))
                                    .child(self.render_action_item("Copy Details", "⇧⌘C", has_selection, selected == idx + 1, &colors))
                                    .child(self.render_action_item("Open in Calendar", "⌘O", has_selection, selected == idx + 2, &colors))
                            })
                            .when(!is_calendar_mode && !is_app, |el| {
                                // Non-app actions (files, commands, etc.)
                                el.child(self.render_action_item("Open", "↵", has_selection, selected == 0, &colors))
                                    .child(self.render_action_item("Copy Path", "⌘C", has_selection, selected == 1, &colors))
                                    .child(self.render_action_item("Copy File", "⇧⌘C", has_selection, selected == 2, &colors))
                                    .when(is_file_mode, |el| {
                                        el.child(self.render_action_item("Reveal in Finder", "⌘↵", has_selection, selected == 3, &colors))
                                            .child(self.render_action_item("Quick Look", "⌘Y", has_selection, selected == 4, &colors))
                                    })
                            })
                            // Task 7.3: App-specific actions with grouped sections
                            .when(!is_calendar_mode && is_app, |el| {
                                let mut idx = 0;
                                // Primary actions
                                let el = el.child(self.render_action_group_header("Primary", &colors));
                                let el = el.child(self.render_action_item("Open", "↵", has_selection, selected == idx, &colors));
                                idx += 1;
                                let el = el.child(self.render_action_item("Show in Finder", "⌘⇧F", has_selection, selected == idx, &colors));
                                idx += 1;

                                // Info actions
                                let el = el.child(self.render_action_group_header("Info", &colors));
                                let el = el.child(self.render_action_item("Copy Path", "⌘⇧C", has_selection, selected == idx, &colors));
                                idx += 1;
                                let el = el.child(self.render_action_item("Copy Bundle ID", "⌘⇧B", has_selection, selected == idx, &colors));
                                idx += 1;

                                // Auto Quit toggle
                                let el = el.child(self.render_action_group_header("Auto Quit", &colors));
                                let auto_quit_label = if has_auto_quit { "Disable Auto Quit" } else { "Enable Auto Quit" };
                                let el = el.child(self.render_action_item(auto_quit_label, "⌘⇧A", has_selection, selected == idx, &colors));
                                idx += 1;

                                // Running app actions (only show if app is running)
                                let el = if is_running {
                                    let el = el.child(self.render_action_group_header("Running App", &colors));
                                    let el = el.child(self.render_action_item("Quit", "⌘Q", has_selection, selected == idx, &colors));
                                    idx += 1;
                                    let el = el.child(self.render_action_item("Force Quit", "⌘⌥Q", has_selection, selected == idx, &colors));
                                    idx += 1;
                                    el.child(self.render_action_item("Hide", "⌘H", has_selection, selected == idx, &colors))
                                } else {
                                    el
                                };
                                let idx = if is_running { idx + 1 } else { idx };

                                // Danger zone
                                let el = el.child(self.render_action_group_header("Danger Zone", &colors));
                                el.child(self.render_action_item_danger("Uninstall", "⌘⌫", has_selection, selected == idx, &colors))
                            })
                    )
                    // Footer hint
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(colors.border)
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_placeholder)
                                    .child("↑↓ Navigate  ↵ Select  esc Close"),
                            ),
                    ),
            )
    }

    /// Render a group header in the actions menu
    fn render_action_group_header(&self, label: &str, colors: &LauncherColors) -> impl IntoElement {
        div()
            .px_3()
            .py(px(4.0))
            .mt(px(4.0))
            .text_size(px(10.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(colors.text_placeholder)
            .child(label.to_string().to_uppercase())
    }

    /// Render a danger action item (red text)
    fn render_action_item_danger(
        &self,
        label: &str,
        shortcut: &str,
        enabled: bool,
        selected: bool,
        colors: &LauncherColors,
    ) -> impl IntoElement {
        let text_color = if enabled {
            colors.error
        } else {
            colors.text_placeholder
        };
        let shortcut_color = if enabled {
            colors.error.opacity(0.7)
        } else {
            colors.text_placeholder
        };
        let bg_color = if selected {
            colors.error.opacity(0.2)
        } else {
            gpui::transparent_black()
        };
        let hover_bg = colors.error.opacity(0.1);
        let surface = colors.surface;

        div()
            .px_3()
            .py(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(bg_color)
            .when(enabled && !selected, move |el| {
                el.hover(move |el| el.bg(hover_bg)).cursor_pointer()
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
                    .bg(surface)
                    .text_size(px(10.0))
                    .text_color(shortcut_color)
                    .child(shortcut.to_string()),
            )
    }

    /// Render a single action item in the menu
    fn render_action_item(
        &self,
        label: &str,
        shortcut: &str,
        enabled: bool,
        selected: bool,
        colors: &LauncherColors,
    ) -> impl IntoElement {
        let text_color = if enabled {
            colors.text
        } else {
            colors.text_placeholder
        };
        let shortcut_color = if enabled {
            colors.text_muted
        } else {
            colors.text_placeholder
        };
        let bg_color = if selected {
            colors.selection
        } else {
            gpui::transparent_black()
        };
        let hover_bg = colors.surface_hover;
        let surface = colors.surface;

        div()
            .px_3()
            .py(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(bg_color)
            .when(enabled && !selected, move |el| {
                el.hover(move |el| el.bg(hover_bg)).cursor_pointer()
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
                    .bg(surface)
                    .text_size(px(10.0))
                    .text_color(shortcut_color)
                    .child(shortcut.to_string()),
            )
    }

    // ========================================================================
    // Task 7.4: App Management Action Handlers
    // ========================================================================

    /// Handler for Show in Finder action (⌘⇧F)
    fn show_in_finder(&mut self, _: &ShowInFinder, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.results.get(self.selected_index) {
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
    fn copy_bundle_id(&mut self, _: &CopyBundleId, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.results.get(self.selected_index) {
            if let Some(bundle_id) = &result.bundle_id {
                if let Err(e) = photoncast_apps::copy_bundle_id_to_clipboard(bundle_id) {
                    tracing::error!("Failed to copy bundle ID: {}", e);
                } else {
                    tracing::info!("Copied bundle ID to clipboard: {}", bundle_id);
                }
            }
        }
        cx.notify();
    }

    /// Handler for Quit App action (⌘Q)
    fn quit_app(&mut self, _: &QuitApp, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.results.get(self.selected_index) {
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
        cx.notify();
    }

    /// Handler for Force Quit App action (⌘⌥Q)
    fn force_quit_app(&mut self, _: &ForceQuitApp, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.results.get(self.selected_index) {
            if result.result_type == ResultType::Application {
                if let Some(bundle_id) = &result.bundle_id {
                    if photoncast_apps::is_app_running(bundle_id) {
                        if let Ok(running_apps) = photoncast_apps::AppManager::new(photoncast_apps::AppsConfig::default()).get_running_apps() {
                            if let Some(app) = running_apps.iter().find(|a| a.bundle_id.as_deref() == Some(bundle_id)) {
                                #[allow(clippy::cast_possible_wrap)]
                                let pid = app.pid as i32;
                                match photoncast_apps::force_quit_app_action(pid) {
                                    Ok(()) => tracing::info!("Force quit app: {} (PID {})", bundle_id, pid),
                                    Err(e) => tracing::error!("Failed to force quit app: {}", e),
                                }
                            }
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    /// Handler for Hide App action (⌘H)
    fn hide_app(&mut self, _: &HideApp, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.results.get(self.selected_index) {
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
        cx.notify();
    }

    /// Handler for Uninstall App action (⌘⌫)
    fn uninstall_app(&mut self, _: &UninstallApp, cx: &mut ViewContext<Self>) {
        // Clone the path to avoid borrow issues
        let app_path = self.results.get(self.selected_index).and_then(|result| {
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
    fn toggle_auto_quit_for_selected(&mut self, _: &ToggleAutoQuit, cx: &mut ViewContext<Self>) {
        if let Some(result) = self.results.get(self.selected_index).cloned() {
            if result.result_type == ResultType::Application {
                if let Some(bundle_id) = &result.bundle_id {
                    let is_enabled = self.auto_quit_manager.read().is_auto_quit_enabled(bundle_id);
                    if is_enabled {
                        // If already enabled, disable it directly
                        let mut manager = self.auto_quit_manager.write();
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

    // ========================================================================
    // Task 7.5: Uninstall Preview UI
    // ========================================================================

    /// Shows the uninstall preview dialog for an app
    pub fn show_uninstall_preview(&mut self, app_path: &std::path::Path, cx: &mut ViewContext<Self>) {
        match self.app_manager.create_uninstall_preview(app_path) {
            Ok(preview) => {
                tracing::info!("Created uninstall preview for: {}", preview.app.name);
                self.uninstall_preview = Some(preview);
                self.uninstall_files_selected_index = 0;
                cx.notify();
            },
            Err(e) => {
                tracing::error!("Failed to create uninstall preview: {}", e);
                self.show_toast(format!("Cannot uninstall: {}", e), cx);
            },
        }
    }

    /// Handles the uninstall action (called when "Uninstall" button is clicked)
    fn perform_uninstall(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(preview) = self.uninstall_preview.take() {
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
    fn perform_uninstall_app_only(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(mut preview) = self.uninstall_preview.take() {
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
    fn cancel_uninstall_preview(&mut self, cx: &mut ViewContext<Self>) {
        self.uninstall_preview = None;
        cx.notify();
    }

    /// Toggles selection of a related file in the uninstall preview
    fn toggle_uninstall_file_selection(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        if let Some(preview) = &mut self.uninstall_preview {
            if let Some(file) = preview.related_files.get_mut(index) {
                file.selected = !file.selected;
                // Recalculate total size
                let selected_size = photoncast_apps::calculate_selected_size(preview);
                preview.space_freed_formatted = UninstallPreview::format_bytes(selected_size);
                cx.notify();
            }
        }
    }

    /// Renders the uninstall preview dialog
    fn render_uninstall_preview(
        &self,
        preview: &UninstallPreview,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let app_name = preview.app.name.clone();
        let space_freed = preview.space_freed_formatted.clone();

        // Group files by category
        let mut categories: std::collections::BTreeMap<&str, Vec<(usize, &photoncast_apps::RelatedFile)>> =
            std::collections::BTreeMap::new();
        for (idx, file) in preview.related_files.iter().enumerate() {
            let category_name = file.category.display_name();
            categories.entry(category_name).or_default().push((idx, file));
        }

        // Get icon path for the app
        let icon_path = Self::get_app_icon_path(&preview.app.path);

        // Pre-build category sections to avoid borrowing cx inside nested iterators
        let category_sections: Vec<_> = categories
            .into_iter()
            .map(|(category_name, files)| {
                let text_muted = colors.text_muted;
                let text = colors.text;
                let text_placeholder = colors.text_placeholder;
                let surface = colors.surface;
                let surface_hover = colors.surface_hover;
                let accent = colors.accent;
                let border = colors.border;

                // Pre-build file items for this category
                let file_items: Vec<_> = files
                    .into_iter()
                    .map(|(idx, file)| {
                        let file_name = file
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let file_size = UninstallPreview::format_bytes(file.size_bytes);
                        let is_selected = file.selected;

                        div()
                            .id(SharedString::from(format!("uninstall-file-{}", idx)))
                            .px_3()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(surface)
                            .hover(move |el| el.bg(surface_hover))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_3()
                            .on_click(cx.listener(move |this, _, cx| {
                                this.toggle_uninstall_file_selection(idx, cx);
                            }))
                            // Checkbox
                            .child(
                                div()
                                    .size(px(18.0))
                                    .rounded(px(4.0))
                                    .border_1()
                                    .border_color(if is_selected { accent } else { border })
                                    .bg(if is_selected {
                                        accent
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(text)
                                            .when(is_selected, |el| el.child("✓")),
                                    ),
                            )
                            // File info
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(text)
                                            .truncate()
                                            .child(file_name),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(text_placeholder)
                                            .child(file_size),
                                    ),
                            )
                    })
                    .collect();

                (category_name.to_string(), text_muted, file_items)
            })
            .collect();

        div()
            .id("uninstall-preview-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(colors.overlay)
            .child(
                div()
                    .id("uninstall-preview-dialog")
                    .w(px(420.0))
                    .max_h(px(500.0))
                    .flex()
                    .flex_col()
                    .bg(colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_xl()
                    .overflow_hidden()
                    // Header with app info
                    .child(
                        div()
                            .px_5()
                            .pt_5()
                            .pb_4()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            // App icon
                            .child(
                                div()
                                    .size(px(64.0))
                                    .rounded(px(12.0))
                                    .overflow_hidden()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .map(|el| {
                                        if let Some(icon) = &icon_path {
                                            el.child(
                                                img(icon.clone())
                                                    .size(px(64.0))
                                                    .object_fit(ObjectFit::Contain),
                                            )
                                        } else {
                                            el.text_size(px(32.0)).child("📦")
                                        }
                                    }),
                            )
                            // App name
                            .child(
                                div()
                                    .text_size(px(18.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text)
                                    .child(format!("Uninstall {}", app_name)),
                            )
                            // Space to be freed
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .text_color(colors.text_muted)
                                            .child("Space to be freed:"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.success)
                                            .child(space_freed),
                                    ),
                            ),
                    )
                    // Related files list
                    .child(
                        div()
                            .id("uninstall-files-list")
                            .flex_1()
                            .overflow_y_scroll()
                            .px_5()
                            .pb_3()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .children(
                                category_sections
                                    .into_iter()
                                    .map(|(category_name, text_muted, file_items)| {
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            // Category header
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(text_muted)
                                                    .child(category_name.to_uppercase()),
                                            )
                                            // Files in category
                                            .children(file_items)
                                    }),
                            ),
                    )
                    // Action buttons
                    .child(
                        div()
                            .px_5()
                            .py_4()
                            .border_t_1()
                            .border_color(colors.border)
                            .flex()
                            .gap_3()
                            // Cancel button
                            .child({
                                let surface = colors.surface;
                                let surface_hover = colors.surface_hover;
                                let text = colors.text;
                                div()
                                    .id("uninstall-cancel")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(surface)
                                    .hover(move |el| el.bg(surface_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.cancel_uninstall_preview(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(text)
                                            .child("Cancel"),
                                    )
                            })
                            // Keep Related Files button
                            .child({
                                let surface = colors.surface;
                                let surface_hover = colors.surface_hover;
                                let text = colors.text;
                                div()
                                    .id("uninstall-keep-files")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(surface)
                                    .hover(move |el| el.bg(surface_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.perform_uninstall_app_only(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(text)
                                            .child("Keep Files"),
                                    )
                            })
                            // Uninstall button (primary, destructive)
                            .child({
                                let error = colors.error;
                                let error_hover =
                                    hsla(error.h, error.s, (error.l + 0.1).min(1.0), error.a);
                                let text = colors.text;
                                div()
                                    .id("uninstall-confirm")
                                    .flex_1()
                                    .h(px(36.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(error)
                                    .hover(move |el| el.bg(error_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.perform_uninstall(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(text)
                                            .child("Uninstall"),
                                    )
                            }),
                    ),
            )
    }

    // ========================================================================
    // Task 7.6: Auto Quit Settings UI
    // ========================================================================

    /// Shows the auto quit settings panel for an app
    pub fn show_auto_quit_settings(&mut self, bundle_id: &str, app_name: &str, cx: &mut ViewContext<Self>) {
        self.auto_quit_settings_app = Some((bundle_id.to_string(), app_name.to_string()));
        self.auto_quit_settings_index = 0; // Reset selection to toggle option
        cx.notify();
    }

    /// Closes the auto quit settings panel
    fn close_auto_quit_settings(&mut self, cx: &mut ViewContext<Self>) {
        self.auto_quit_settings_app = None;
        cx.notify();
    }

    /// Toggles auto quit for the currently shown app in settings panel
    fn toggle_auto_quit_in_settings(&mut self, cx: &mut ViewContext<Self>) {
        if let Some((ref bundle_id, _)) = self.auto_quit_settings_app {
            let mut manager = self.auto_quit_manager.write();
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
    fn set_auto_quit_timeout(&mut self, minutes: u32, cx: &mut ViewContext<Self>) {
        if let Some((ref bundle_id, _)) = self.auto_quit_settings_app {
            let mut manager = self.auto_quit_manager.write();
            manager.enable_auto_quit(bundle_id, minutes);
            let _ = manager.save();
            cx.notify();
        }
    }

    /// Activates the currently selected option in auto-quit settings
    fn activate_auto_quit_settings_option(&mut self, cx: &mut ViewContext<Self>) {
        let timeout_options = [1, 2, 3, 5, 10, 15, 30];
        match self.auto_quit_settings_index {
            0 => {
                // Toggle auto quit
                self.toggle_auto_quit_in_settings(cx);
            }
            idx if (1..=7).contains(&idx) => {
                // Set timeout (index 1 = 1 min, index 2 = 2 min, etc.)
                let minutes = timeout_options[idx - 1];
                self.set_auto_quit_timeout(minutes, cx);
            }
            _ => {}
        }
    }

    /// Renders the auto quit settings panel
    fn render_auto_quit_settings(
        &self,
        bundle_id: &str,
        app_name: &str,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let manager = self.auto_quit_manager.read();
        let is_enabled = manager.is_auto_quit_enabled(bundle_id);
        let current_timeout = manager.get_timeout_minutes(bundle_id).unwrap_or(DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES);
        drop(manager);

        let timeout_options = [1, 2, 3, 5, 10, 15, 30];
        let selected_index = self.auto_quit_settings_index;

        div()
            .id("auto-quit-settings-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_end()
            .justify_end()
            .pb_2()
            .pr_2()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, cx| {
                this.close_auto_quit_settings(cx);
            }))
            .child(
                div()
                    .w(px(280.0))
                    .bg(colors.surface_elevated)
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, cx| cx.stop_propagation())
                    // Header
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(colors.border)
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text)
                                    .child("Auto Quit Settings"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_muted)
                                    .truncate()
                                    .child(app_name.to_string()),
                            ),
                    )
                    // Enable toggle
                    .child({
                        let surface = colors.surface;
                        let surface_hover = colors.surface_hover;
                        let text = colors.text;
                        let accent = colors.accent;
                        let is_toggle_selected = selected_index == 0;
                        div()
                            .id("auto-quit-toggle")
                            .px_4()
                            .py_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .cursor_pointer()
                            .bg(if is_toggle_selected { surface_hover } else { gpui::transparent_black() })
                            .hover(move |el| el.bg(surface_hover))
                            .on_click(cx.listener(|this, _, cx| {
                                this.toggle_auto_quit_in_settings(cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(text)
                                    .child("Enable Auto Quit"),
                            )
                            .child(
                                div()
                                    .w(px(36.0))
                                    .h(px(20.0))
                                    .rounded(px(10.0))
                                    .bg(if is_enabled { accent } else { surface })
                                    .relative()
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(2.0))
                                            .left(if is_enabled { px(18.0) } else { px(2.0) })
                                            .size(px(16.0))
                                            .rounded_full()
                                            .bg(text),
                                    ),
                            )
                    })
                    // Timeout selector (only when enabled)
                    .when(is_enabled, |el| {
                        let text_muted = colors.text_muted;
                        let text = colors.text;
                        let surface = colors.surface;
                        let surface_hover = colors.surface_hover;
                        let accent = colors.accent;
                        el.child(
                            div()
                                .px_4()
                                .py_2()
                                .border_t_1()
                                .border_color(colors.border)
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child("Quit after idle for:"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap(px(6.0))
                                        .children(timeout_options.iter().enumerate().map(move |(idx, &minutes)| {
                                            let is_current = minutes == current_timeout;
                                            let is_kb_selected = selected_index == idx + 1; // +1 because 0 is toggle
                                            div()
                                                .id(SharedString::from(format!("timeout-{}", minutes)))
                                                .px(px(10.0))
                                                .py(px(4.0))
                                                .rounded(px(4.0))
                                                .bg(if is_current { accent } else if is_kb_selected { surface_hover } else { surface })
                                                .border_1()
                                                .border_color(if is_kb_selected { accent } else { gpui::transparent_black() })
                                                .hover(move |el| el.bg(if is_current { accent } else { surface_hover }))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, cx| {
                                                    this.set_auto_quit_timeout(minutes, cx);
                                                }))
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(text)
                                                        .child(format!("{} min", minutes)),
                                                )
                                        })),
                                ),
                        )
                    })
                    // Footer hint
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .border_t_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_placeholder)
                                    .child("Auto Quit stops idle apps to save resources"),
                            ),
                    ),
            )
    }

    // ========================================================================
    // Task 7.7: Manage Auto Quits Command
    // ========================================================================

    /// Enters the "Manage Auto Quits" mode
    #[allow(dead_code)]
    pub fn enter_manage_auto_quits_mode(&mut self, cx: &mut ViewContext<Self>) {
        self.manage_auto_quits_mode = true;
        self.manage_auto_quits_index = 0;
        self.query = SharedString::default();
        self.cursor_position = 0;
        self.selection_anchor = None;
        cx.notify();
    }

    /// Exits the "Manage Auto Quits" mode
    #[allow(dead_code)]
    fn exit_manage_auto_quits_mode(&mut self, cx: &mut ViewContext<Self>) {
        self.manage_auto_quits_mode = false;
        self.manage_auto_quits_index = 0;
        self.load_suggestions(cx);
        cx.notify();
    }

    /// Disables auto quit for the app at the given index
    fn disable_auto_quit_at_index(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        let enabled_apps: Vec<_> = {
            let manager = self.auto_quit_manager.read();
            manager.get_enabled_apps()
                .iter()
                .map(|(id, cfg)| (id.to_string(), cfg.timeout_minutes))
                .collect()
        };

        if let Some((bundle_id, _)) = enabled_apps.get(index) {
            let mut manager = self.auto_quit_manager.write();
            manager.disable_auto_quit(bundle_id);
            let _ = manager.save();
            drop(manager);
            self.show_toast("Auto Quit disabled".to_string(), cx);
            cx.notify();
        }
    }

    /// Renders the "Manage Auto Quits" view
    fn render_manage_auto_quits(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);
        let manager = self.auto_quit_manager.read();
        let enabled_apps: Vec<_> = manager.get_enabled_apps()
            .iter()
            .map(|(id, cfg)| (id.to_string(), cfg.timeout_minutes))
            .collect();
        drop(manager);

        let selected = self.manage_auto_quits_index;
        let is_empty = enabled_apps.is_empty();

        div()
            .w_full()
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text_muted)
                            .child("AUTO QUIT APPS"),
                    ),
            )
            // App list or empty state
            .when(is_empty, |el| {
                el.child(
                    div()
                        .py_6()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(colors.text_muted)
                                .child("No apps with Auto Quit enabled"),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(colors.text_placeholder)
                                .child("Use ⌘K on any app to enable Auto Quit"),
                        ),
                )
            })
            .when(!is_empty, |el| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .children(enabled_apps.iter().enumerate().map(|(idx, (bundle_id, timeout))| {
                            let is_selected = idx == selected;
                            let app_name = photoncast_apps::get_suggested_app_name(bundle_id)
                                .map(String::from)
                                .unwrap_or_else(|| {
                                    // Try to get last component of bundle ID
                                    bundle_id.split('.').next_back().unwrap_or(bundle_id).to_string()
                                });

                            let text = colors.text;
                            let text_muted = colors.text_muted;
                            let text_placeholder = colors.text_placeholder;
                            let surface = colors.surface;
                            let surface_hover = colors.surface_hover;
                            let selection = colors.selection;
                            let error = colors.error;

                            div()
                                .id(SharedString::from(format!("auto-quit-app-{}", idx)))
                                .h(px(48.0))
                                .px_4()
                                .flex()
                                .items_center()
                                .justify_between()
                                .bg(if is_selected { selection } else { gpui::transparent_black() })
                                .hover(move |el| el.bg(surface_hover))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .text_color(text)
                                                .child(app_name),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(text_muted)
                                                .child(format!("Quit after {} min idle", timeout)),
                                        ),
                                )
                                // Disable button
                                .child(
                                    div()
                                        .id(SharedString::from(format!("disable-auto-quit-{}", idx)))
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(surface)
                                        .hover(move |el| el.bg(error.opacity(0.2)))
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, cx| {
                                            this.disable_auto_quit_at_index(idx, cx);
                                        }))
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(text_placeholder)
                                                .child("Disable"),
                                        ),
                                )
                        })),
                )
            })
            // Footer with hints
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(colors.border)
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(colors.text_placeholder)
                            .child("↑↓ Navigate  esc Back"),
                    ),
            )
    }

    // ========================================================================
    // Task 7.8: Toast Notifications
    // ========================================================================

    /// Shows a toast notification message
    pub fn show_toast(&mut self, message: String, cx: &mut ViewContext<Self>) {
        self.toast_message = Some(message);
        self.toast_shown_at = Some(Instant::now());

        // Auto-dismiss after 2 seconds
        cx.spawn(|this, mut cx| async move {
            gpui::Timer::after(Duration::from_millis(2000)).await;
            let _ = this.update(&mut cx, |this, cx| {
                this.toast_message = None;
                this.toast_shown_at = None;
                cx.notify();
            });
        })
        .detach();

        cx.notify();
    }

    /// Renders the toast notification
    fn render_toast(&self, message: &str, cx: &ViewContext<Self>) -> impl IntoElement {
        let colors = get_launcher_colors(cx);

        // Calculate opacity based on time (fade in/out)
        let opacity = if let Some(shown_at) = self.toast_shown_at {
            let elapsed = shown_at.elapsed().as_millis() as f32;
            if elapsed < 150.0 {
                // Fade in
                elapsed / 150.0
            } else if elapsed > 1800.0 {
                // Fade out (after 1.8s, fade out over 200ms)
                1.0 - ((elapsed - 1800.0) / 200.0).min(1.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        div()
            .id("toast-notification")
            .absolute()
            .bottom(px(12.0))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .opacity(opacity)
            .child(
                div()
                    .px_4()
                    .py_2()
                    .rounded(px(8.0))
                    .bg(colors.surface_elevated)
                    .border_1()
                    .border_color(colors.border)
                    .shadow_md()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(16.0))
                            .child("✓"),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child(message.to_string()),
                    ),
            )
    }
}

impl Render for LauncherWindow {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Get theme colors for the UI
        let colors = get_launcher_colors(cx);
        
        // Calculate current animation opacity
        let opacity = self.current_opacity();

        // Clone pending confirmation for use in the closure
        let pending_dialog = self.pending_confirmation.as_ref().map(|(_, d)| d.clone());
        
        // Pre-render components that need colors (for use in closures)
        let empty_state = self.render_empty_state(&colors);
        let no_results = self.render_no_results(&colors);
        let divider_color = colors.border;

        // Check if any overlay is active (need minimum height for overlays)
        let has_overlay = self.show_actions_menu 
            || self.auto_quit_settings_app.is_some()
            || self.uninstall_preview.is_some()
            || self.manage_auto_quits_mode
            || pending_dialog.is_some();

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
            // Task 7.4: App management action handlers
            .on_action(cx.listener(Self::show_in_finder))
            .on_action(cx.listener(Self::copy_bundle_id))
            .on_action(cx.listener(Self::quit_app))
            .on_action(cx.listener(Self::force_quit_app))
            .on_action(cx.listener(Self::hide_app))
            .on_action(cx.listener(Self::uninstall_app))
            .on_action(cx.listener(Self::toggle_auto_quit_for_selected))
            .on_action(cx.listener(|this, _: &QuickSelect1, cx| this.quick_select(0, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect2, cx| this.quick_select(1, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect3, cx| this.quick_select(2, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect4, cx| this.quick_select(3, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect5, cx| this.quick_select(4, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect6, cx| this.quick_select(5, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect7, cx| this.quick_select(7, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect8, cx| this.quick_select(7, cx)))
            .on_action(cx.listener(|this, _: &QuickSelect9, cx| this.quick_select(8, cx)))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            // Apply window appear/dismiss animation opacity
            .opacity(opacity)
            // Theme background color
            .bg(colors.background)
            .rounded(LAUNCHER_BORDER_RADIUS)
            .shadow_lg()
            .border_1()
            .border_color(colors.border)
            // Keep minimum height when overlays are visible to prevent clipping
            .when(has_overlay, |el| el.min_h(px(400.0)))
            .overflow_hidden()
            // File Search Mode: render the dedicated FileSearchView
            .when_some(self.file_search_view.clone(), |el, view| {
                el.child(
                    div()
                        .size_full() // Fill the resized window
                        .child(view)
                )
            })
            // Normal/Calendar Mode: render the standard launcher content
            .when(self.file_search_view.is_none(), |el| {
                el
                    // Search bar
                    .child(self.render_search_bar(cx))
                    // Content area (flex-1 to push action bar to bottom)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Next meeting widget (show when query is empty and we have a meeting)
                            .when(self.query.is_empty() && self.next_meeting.is_some() && !matches!(self.search_mode, SearchMode::Calendar { .. }), |el| {
                                if let Some(meeting) = &self.next_meeting {
                                    el.child(self.render_next_meeting(meeting, &colors))
                                } else {
                                    el
                                }
                            })
                            // Divider (show when there are results or in calendar mode)
                            .when(!self.results.is_empty() || matches!(self.search_mode, SearchMode::Calendar { .. }), move |el| {
                                el.child(div().h(px(1.0)).w_full().bg(divider_color))
                            })
                            // Empty state: Normal mode with no meeting
                            .when(self.query.is_empty() && self.results.is_empty() && matches!(self.search_mode, SearchMode::Normal) && self.next_meeting.is_none(), |el| {
                                el.child(empty_state)
                            })
                            // No results message: query entered but nothing found
                            .when(!self.query.is_empty() && self.results.is_empty() && !matches!(self.search_mode, SearchMode::Calendar { .. }), |el| {
                                el.child(no_results)
                            })
                            // Results list: show suggestions (when query empty) or search results
                            .when(!self.results.is_empty() || matches!(self.search_mode, SearchMode::Calendar { .. }), |el| {
                                el.child(self.render_results(cx))
                            })
                    )
                    // Action bar at bottom - always visible, pinned by flex layout
                    .child(self.render_action_bar(cx))
            })
            // Actions menu overlay (Cmd+K)
            .when(self.show_actions_menu, |el| {
                el.child(self.render_actions_menu(cx))
            })
            // Confirmation dialog overlay
            .when_some(pending_dialog, |el, dialog| {
                el.child(self.render_confirmation_dialog(&dialog, cx))
            })
            // Task 7.5: Uninstall preview overlay
            .when_some(self.uninstall_preview.clone(), |el, preview| {
                el.child(self.render_uninstall_preview(&preview, cx))
            })
            // Task 7.6: Auto quit settings overlay
            .when_some(self.auto_quit_settings_app.clone(), |el, (bundle_id, app_name)| {
                el.child(self.render_auto_quit_settings(&bundle_id, &app_name, cx))
            })
            // Task 7.7: Manage auto quits view
            .when(self.manage_auto_quits_mode, |el| {
                el.child(self.render_manage_auto_quits(cx))
            })
            // Task 7.8: Toast notification
            .when_some(self.toast_message.clone(), |el, message| {
                el.child(self.render_toast(&message, cx))
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

/// Escapes a path string for safe use in `AppleScript`.
///
/// This prevents command injection attacks by escaping special characters.
#[must_use]
pub fn escape_path_for_applescript(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}
