//! PhotonCast - Lightning-fast macOS launcher built in pure Rust
//!
//! This is the main entry point for the PhotonCast application.
//! It initializes GPUI, creates the launcher window, and runs the event loop.

// Clippy configuration for binary crate
// Kept: pragmatic suppressions for GPUI-heavy codebase
#![allow(clippy::missing_errors_doc)] // Docs will be added incrementally
#![allow(clippy::missing_panics_doc)] // Docs will be added incrementally
#![allow(clippy::must_use_candidate)] // Too noisy for builder-pattern-heavy GPUI code
#![allow(clippy::module_name_repetitions)] // Rust naming convention (e.g., LauncherState in launcher mod)
#![allow(clippy::too_many_lines)] // GPUI render functions are inherently long
#![allow(clippy::cast_possible_truncation)] // Frequent in GPUI pixel math (f64 -> f32, etc.)
#![allow(clippy::cast_sign_loss)] // Frequent in GPUI pixel math
#![allow(clippy::cast_precision_loss)] // Frequent in GPUI pixel math

use std::cell::RefCell;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::*;
use parking_lot::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod app_events;
mod constants;
mod event_loop;
mod extension_views;
mod file_search_helper;
mod file_search_view;
mod icon_cache;
mod launcher;
mod permissions_dialog;
mod platform;
mod preferences_window;

use constants::{
    EXPANDED_HEIGHT, LAUNCHER_HEIGHT, LAUNCHER_WIDTH, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH,
    MODAL_WIDTH,
};

use app_events::AppEvent;
use launcher::{LauncherSharedState, LauncherWindow};
use photoncast_clipboard::ui::{
    ClipboardHistoryView, CopyClipboardItem, DeleteClipboardItem, PasteAsPlainText,
    TogglePinClipboardItem,
};
use photoncast_clipboard::{ClipboardConfig, ClipboardMonitor, ClipboardStorage};
use photoncast_core::app::config::{Config, ThemeSetting};
use photoncast_core::platform::accessibility::{
    check_accessibility_permission, request_accessibility_permission,
};
use photoncast_core::platform::hotkey::is_spotlight_enabled;
use photoncast_core::platform::LoginItemManager;
use photoncast_core::theme::PhotonTheme;
use photoncast_quicklinks::ui::{
    ArgumentInputEvent, ArgumentInputView, CreateQuicklinkView, QuicklinksManageView,
};
use platform::{create_menu_bar_item, is_menu_bar_active, remove_menu_bar_item, MenuBarActionKind};
use preferences_window::PreferencesWindow;

actions!(
    photoncast,
    [
        SelectNext,
        SelectPrevious,
        Activate,
        Cancel,
        ConfirmDialog,
        QuickSelect1,
        QuickSelect2,
        QuickSelect3,
        QuickSelect4,
        QuickSelect5,
        QuickSelect6,
        QuickSelect7,
        QuickSelect8,
        QuickSelect9,
        NextGroup,
        PreviousGroup,
        OpenPreferences,
        ToggleLauncher,
        // File Search Mode actions
        RevealInFinder,
        QuickLook,
        CopyPath,
        CopyFile,
        ShowActionsMenu,
        BrowseEnterFolder,
        BrowseGoBack,
        // Clipboard History
        OpenClipboardHistory,
        // Task 7.4: App Management actions
        ShowInFinder,
        CopyBundleId,
        QuitApp,
        ForceQuitApp,
        HideApp,
        UninstallApp,
        ToggleAutoQuit,
    ]
);

const LAUNCHER_BORDER_RADIUS: Pixels = px(12.0);

/// Position from top of screen (20%)
const LAUNCHER_TOP_OFFSET_PERCENT: f32 = 0.20;

/// Shared clipboard state
struct ClipboardState {
    storage: ClipboardStorage,
    config: ClipboardConfig,
    // Field is intentionally kept for its Drop side-effect.
    // The `Arc<ClipboardMonitor>` owns the background clipboard monitoring task;
    // dropping it would stop clipboard change detection.
    #[allow(dead_code)]
    monitor: Option<Arc<ClipboardMonitor>>,
}

/// Initializes the tracing/logging subsystem with an environment-based filter.
fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting PhotonCast v{}", env!("CARGO_PKG_VERSION"));
}

fn write_perf_marker(label: &str) {
    let Some(path) = std::env::var_os("PHOTONCAST_PERF_MARKERS_PATH") else {
        return;
    };

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let line = format!("{label},{now_ms}\n");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
}

pub(crate) fn dispatch_menu_bar_action(action: MenuBarActionKind) {
    let event = match action {
        MenuBarActionKind::ToggleLauncher => AppEvent::ToggleLauncher,
        MenuBarActionKind::OpenPreferences => AppEvent::OpenPreferences,
        MenuBarActionKind::CheckForUpdates => {
            info!("Check for updates requested from menu bar");
            return;
        },
        MenuBarActionKind::About => {
            info!("About requested from menu bar");
            return;
        },
        MenuBarActionKind::Quit => AppEvent::QuitApp,
    };

    if let Err(e) = app_events::send_event(event) {
        error!("Failed to send menu bar event: {}", e);
    }
}

pub(crate) fn sync_menu_bar_visibility(show_in_menu_bar: bool) -> Result<(), String> {
    if show_in_menu_bar {
        if is_menu_bar_active() {
            return Ok(());
        }

        create_menu_bar_item(dispatch_menu_bar_action)?;
        info!("Menu bar status item created");
        return Ok(());
    }

    if is_menu_bar_active() {
        remove_menu_bar_item();
        info!("Menu bar status item removed");
    }

    Ok(())
}

/// Loads the application configuration from disk and applies global settings.
///
/// Falls back to [`Config::default()`] if the config file cannot be read.
fn load_app_config() -> Config {
    let app_config = photoncast_core::app::config_file::load_config().unwrap_or_else(|e| {
        warn!("Failed to load config, using defaults: {}", e);
        Config::default()
    });

    // Apply reduce motion setting from config
    photoncast_core::ui::animations::set_reduce_motion_override(Some(
        app_config.appearance.reduce_motion,
    ));
    info!(
        "Loaded config: theme={:?}, accent={:?}, reduce_motion={}",
        app_config.appearance.theme,
        app_config.appearance.accent_color,
        app_config.appearance.reduce_motion
    );

    app_config
}

/// Registers global hotkeys for the launcher toggle and clipboard history.
///
/// Requires accessibility permission. If permission is not granted, requests it
/// and logs a warning — the user will need to restart after granting access.
fn init_hotkeys(event_tx: &mpsc::Sender<AppEvent>) {
    let has_permission = check_accessibility_permission();
    if has_permission {
        // Register launcher hotkey (Cmd+Space)
        let hotkey_tx = event_tx.clone();
        match platform::register_global_hotkey(move || {
            if let Err(e) = hotkey_tx.send(AppEvent::ToggleLauncher) {
                error!("Failed to send hotkey event: {}", e);
            }
        }) {
            Ok(()) => info!("Global hotkey (Cmd+Space) registered"),
            Err(e) => error!("Failed to register global hotkey: {}", e),
        }

        // Register clipboard hotkey (Cmd+Shift+V)
        let clipboard_tx = event_tx.clone();
        match platform::register_clipboard_hotkey(move || {
            if let Err(e) = clipboard_tx.send(AppEvent::OpenClipboardHistory) {
                error!("Failed to send clipboard hotkey event: {}", e);
            }
        }) {
            Ok(()) => info!("Clipboard hotkey (Cmd+Shift+V) registered"),
            Err(e) => error!("Failed to register clipboard hotkey: {}", e),
        }
    } else {
        // Request permission without blocking — user will need to restart after granting
        warn!("Accessibility permission not granted. Requesting...");
        request_accessibility_permission();
        warn!(
            "Please grant accessibility access and restart PhotonCast for global hotkey to work."
        );
    }
}

/// Initializes clipboard storage, starts the background monitor, and returns
/// the shared clipboard state (or `None` if storage failed to open).
///
/// Uses the provided runtime handle for the background clipboard monitoring thread
/// instead of creating a separate Tokio runtime.
fn init_clipboard(runtime_handle: &tokio::runtime::Handle) -> Option<Arc<RwLock<ClipboardState>>> {
    let clipboard_config = ClipboardConfig::default();

    let clipboard_storage = match ClipboardStorage::open(&clipboard_config) {
        Ok(storage) => {
            info!("Clipboard storage initialized");
            Some(storage)
        },
        Err(e) => {
            error!("Failed to initialize clipboard storage: {}", e);
            None
        },
    };

    // Create clipboard monitor if storage is available
    let clipboard_monitor = clipboard_storage.as_ref().map(|storage| {
        let monitor = Arc::new(ClipboardMonitor::new(
            storage.clone(),
            clipboard_config.clone(),
        ));
        info!("Clipboard monitor created");
        monitor
    });

    // Spawn background thread for clipboard monitoring using the shared runtime handle
    if let Some(monitor) = clipboard_monitor.clone() {
        let handle = runtime_handle.clone();
        std::thread::spawn(move || {
            if let Err(e) = handle.block_on(monitor.start()) {
                error!("Clipboard monitor stopped with error: {}", e);
            }
        });
    }

    // Wrap clipboard state for sharing across the application
    clipboard_storage.map(|storage| {
        Arc::new(RwLock::new(ClipboardState {
            storage,
            config: clipboard_config,
            monitor: clipboard_monitor,
        }))
    })
}

fn main() {
    init_logging();
    write_perf_marker("main_start");

    let app_config = load_app_config();
    // Check for hotkey conflicts with Spotlight (non-blocking, informational only)
    check_spotlight_conflict();

    // Check and log login item status
    check_login_item_status();

    // Create channel for app events (hotkey, menu bar)
    let (event_tx, event_rx) = mpsc::channel::<AppEvent>();
    app_events::set_event_sender(event_tx.clone());

    // Register global hotkeys (non-blocking, requires accessibility permission)
    init_hotkeys(&event_tx);

    // Create a single shared Tokio runtime for the entire application.
    // All async work (clipboard, quicklinks, timers, etc.) uses this runtime
    // instead of creating separate runtimes per subsystem.
    let shared_runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => Arc::new(runtime),
        Err(err) => {
            error!("Failed to create shared Tokio runtime: {}", err);
            return;
        },
    };

    // Initialize clipboard subsystem (storage + background monitor)
    let clipboard_state = init_clipboard(shared_runtime.handle());

    // Pre-initialize live file index for instant file search
    // This takes ~7s to populate, so starting early ensures it's ready
    file_search_helper::init_live_index();

    // Initialize and run GPUI application
    App::new().run(move |cx: &mut AppContext| {
        // Initialize theme from config and set as global
        let flavor = app_config.appearance.theme.to_catppuccin_flavor();
        let accent = app_config.appearance.accent_color.to_theme_accent();
        let auto_sync = app_config.appearance.theme == ThemeSetting::Auto;
        let theme = PhotonTheme::new(flavor, accent).with_auto_sync(auto_sync);
        cx.set_global(theme);
        if let Err(e) = platform::sync_activation_policy(app_config.general.show_in_dock) {
            error!("Failed to sync Dock activation policy: {}", e);
        }
        info!(
            "Theme initialized: flavor={:?}, accent={:?}, auto_sync={}",
            flavor, accent, auto_sync
        );

        // Register key bindings
        register_key_bindings(cx);

        // Store clipboard state in global
        let clipboard_for_window = clipboard_state.clone();

        // Create initial launcher window
        let launcher_state = LauncherSharedState::new(Arc::clone(&shared_runtime));
        write_perf_marker("before_open_launcher_window");
        let window_handle = open_launcher_window(cx, &launcher_state);
        write_perf_marker("after_open_launcher_window");

        // Defer extension auto-load until after the first launcher window is
        // created so extension activation does not sit on the critical path for
        // initial window visibility.
        {
            let photoncast_app = launcher_state.photoncast_app();
            std::mem::drop(cx.background_executor().spawn(async move {
                photoncast_app.read().autoload_enabled_extensions();
            }));
        }

        // Create menu bar status item with channel for events after the first
        // launcher window is created, so menu-bar initialization does not delay
        // initial window visibility.
        match sync_menu_bar_visibility(app_config.general.show_in_menu_bar) {
            Ok(()) => {},
            Err(e) => error!("Failed to sync menu bar status item: {}", e),
        }

        // Spawn a task to listen for app events (hotkey, menu bar)
        let clipboard_state_for_events = clipboard_for_window;
        cx.spawn(|mut cx| async move {
            let quicklinks_storage = match photoncast_quicklinks::QuickLinksStorage::open(
                photoncast_core::utils::paths::data_dir().join("quicklinks.db"),
            ) {
                Ok(storage) => storage,
                Err(err) => {
                    error!(
                        "Failed to open quick links storage, falling back to in-memory: {}",
                        err
                    );
                    match photoncast_quicklinks::QuickLinksStorage::open_in_memory() {
                        Ok(storage) => storage,
                        Err(in_memory_err) => {
                            error!(
                                "Failed to open in-memory quick links storage: {}",
                                in_memory_err
                            );
                            return;
                        },
                    }
                },
            };

            // Populate bundled quicklinks on first use
            if let Err(e) = quicklinks_storage.populate_bundled_if_empty() {
                warn!("Failed to populate bundled quicklinks: {}", e);
            }

            // Use the shared runtime for all async operations (quicklinks, timers, etc.)
            let shared_rt = Arc::clone(&shared_runtime);

            // Build the event loop state that holds all window handles and shared resources
            let mut state = event_loop::EventLoopState {
                current_handle: window_handle,
                clipboard_handle: None,
                quicklinks_handle: None,
                timer_handle: None,
                preferences_handle: None,
                create_quicklink_handle: None,
                manage_quicklinks_handle: None,
                launcher_state: launcher_state.clone(),
                clipboard_state: clipboard_state_for_events,
                quicklinks_storage,
                shared_rt: Arc::clone(&shared_rt),
                app_manager: photoncast_apps::AppManager::new(
                    photoncast_apps::AppsConfig::default(),
                ),
                calendar_command: photoncast_calendar::CalendarCommand::with_default_config(),
            };
            let event_rx = Arc::new(std::sync::Mutex::new(event_rx));

            // Timer polling setup (runs on main thread, executes actions in background)
            let timer_manager = launcher_state.timer_manager();
            let timer_event_tx = event_tx.clone();
            let shared_handle = shared_rt.handle().clone();
            let mut last_timer_check = std::time::Instant::now();

            loop {
                let wait_timeout = Duration::from_millis(250)
                    .min(Duration::from_secs(1).saturating_sub(last_timer_check.elapsed()));
                let next_event = {
                    let event_rx = Arc::clone(&event_rx);
                    cx.background_executor().spawn(async move {
                        event_rx
                            .lock()
                            .expect("app event receiver poisoned")
                            .recv_timeout(wait_timeout)
                    })
                }
                .await;

                // Check timer every second (database read is fast, action executes in background)
                if last_timer_check.elapsed() >= Duration::from_secs(1) {
                    last_timer_check = std::time::Instant::now();
                    // Check if timer expired (fast database read)
                    let expired_action = shared_handle.block_on(async {
                        let mgr = timer_manager.read().await;
                        mgr.check_expired().await
                    });

                    // If expired, execute action in background thread (no rusqlite crossing)
                    if let Ok(Some(action)) = expired_action {
                        let tx = timer_event_tx.clone();
                        let action_name = action.display_name().to_string();
                        let handle_for_action = shared_handle.clone();
                        std::thread::spawn(move || {
                            handle_for_action.block_on(async {
                                if let Err(e) =
                                    photoncast_timer::TimerScheduler::execute_action(action).await
                                {
                                    error!("Timer action failed: {}", e);
                                }
                            });
                            let _ = tx.send(AppEvent::TimerExpired {
                                action: action_name,
                            });
                        });
                    }
                }

                // Dispatch app events via the extracted handler
                match next_event {
                    Ok(event) => {
                        if !state.handle_event(event, &mut cx) {
                            break;
                        }
                    },
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // No event before timeout, continue to timer check / next wait.
                    },
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // Channel closed, stop listening
                        info!("Event channel closed");
                        break;
                    },
                }
            }
        })
        .detach();

        info!("PhotonCast initialized successfully");
        write_perf_marker("app_initialized");
    });

    // Cleanup: unregister hotkey
    platform::unregister_global_hotkey();
}

// Thread-local singleton for window management.
// This avoids creating a new WindowManager for each command, which improves performance
// by reusing the cached AXUIElementRef references and avoiding repeated permission checks.
// Uses thread_local because WindowManager contains raw pointers that are not Send.
thread_local! {
    static WINDOW_MANAGER: RefCell<photoncast_window::WindowManager> = RefCell::new(
        photoncast_window::WindowManager::default()
    );
}

/// Polls until the specified app becomes frontmost or timeout expires.
/// Returns true if the app became frontmost, false on timeout.
fn poll_until_app_frontmost(
    target_bundle_id: &str,
    timeout_ms: u64,
    poll_interval_ms: u64,
) -> bool {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let interval = std::time::Duration::from_millis(poll_interval_ms);

    while start.elapsed() < timeout {
        let is_frontmost = WINDOW_MANAGER.with(|m| {
            m.borrow()
                .get_frontmost_bundle_id()
                .map(|f| f == target_bundle_id)
                .unwrap_or(false)
        });
        if is_frontmost {
            tracing::debug!(
                "App {} became frontmost after {:?}",
                target_bundle_id,
                start.elapsed()
            );
            return true;
        }
        std::thread::sleep(interval);
    }

    tracing::warn!(
        "Timeout waiting for app {} to become frontmost",
        target_bundle_id
    );
    false
}

/// Polls until CGWindowList shows the expected window as frontmost or timeout expires.
/// Returns true if the window became frontmost, false on timeout.
fn poll_until_window_frontmost(
    expected_bundle_id: Option<&str>,
    expected_title: Option<&str>,
    timeout_ms: u64,
    poll_interval_ms: u64,
) -> bool {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let interval = std::time::Duration::from_millis(poll_interval_ms);

    while start.elapsed() < timeout {
        if let Some(window_info) = photoncast_window::get_frontmost_window_via_cgwindowlist() {
            let current_bundle = photoncast_window::get_bundle_id_for_pid(window_info.owner_pid);

            // Check if bundle ID matches (if we have an expected one)
            let bundle_matches = match (expected_bundle_id, &current_bundle) {
                (Some(expected), Some(current)) => expected == current,
                (None, _) => true, // No expectation, always matches
                (Some(_), None) => false,
            };

            // Check if title matches (if we have an expected one)
            let title_matches = match expected_title {
                Some(expected) => window_info.title == expected,
                None => true, // No expectation, always matches
            };

            if bundle_matches && title_matches {
                tracing::debug!(
                    "Window became frontmost after {:?}: bundle={:?}, title={}",
                    start.elapsed(),
                    current_bundle,
                    window_info.title
                );
                return true;
            }
        }
        std::thread::sleep(interval);
    }

    tracing::warn!(
        "Timeout waiting for window to become frontmost: bundle={:?}, title={:?}",
        expected_bundle_id,
        expected_title
    );
    false
}

fn select_window_command_target(
    actual_bundle_id: Option<String>,
    actual_title: Option<String>,
    captured_bundle_id: Option<String>,
    captured_title: Option<String>,
) -> (Option<String>, Option<String>) {
    match captured_bundle_id {
        Some(bundle_id) => {
            let title = captured_title.or_else(|| {
                actual_bundle_id
                    .as_deref()
                    .filter(|actual| *actual == bundle_id)
                    .and(actual_title)
            });
            (Some(bundle_id), title)
        },
        None => (actual_bundle_id, actual_title.or(captured_title)),
    }
}

/// Executes a window management command outside of GPUI window context.
/// This avoids reentrancy panics when moving windows triggers windowDidMove notifications.
fn execute_window_command(
    command_id: &str,
    target_bundle_id: Option<String>,
    target_window_title: Option<String>,
) {
    // Load user config for window management
    let wm_config = photoncast_core::app::config_file::load_config()
        .map(|c| c.window_management)
        .unwrap_or_default();

    // Check if window management is enabled
    if !wm_config.enabled {
        tracing::debug!("Window management is disabled in preferences");
        return;
    }

    // Create WindowConfig from user preferences and update the singleton
    let window_config = photoncast_window::WindowConfig {
        enabled: wm_config.enabled,
        animation_enabled: wm_config.animation_enabled,
        animation_duration_ms: 200,
        cycling_enabled: wm_config.cycling_enabled,
        window_gap: wm_config.window_gap,
        respect_menu_bar: true,
        respect_dock: true,
        cycle_timeout_ms: 500,
        almost_maximize_margin: wm_config.almost_maximize_margin,
        show_visual_feedback: wm_config.show_visual_feedback,
        visual_feedback_duration_ms: 200,
    };

    // Update the singleton's config (this preserves cached AXUIElementRefs)
    WINDOW_MANAGER.with(|m| m.borrow_mut().set_config(window_config));

    // Check and request accessibility permission if needed
    WINDOW_MANAGER.with(|m| {
        let mut manager = m.borrow_mut();
        if !manager.has_accessibility_permission() {
            tracing::info!("Requesting accessibility permission for window management");
            if let Err(e) = manager.request_accessibility_permission() {
                tracing::warn!("Accessibility permission not granted: {}", e);
            }
        }
    });

    // Get the ACTUAL frontmost window using CGWindowList (excludes Photoncast)
    // This is more reliable than using the stale previous_frontmost_app value
    // because the user might have switched apps while the launcher was open
    let (actual_bundle_id, actual_title) = if let Some(window_info) =
        photoncast_window::get_frontmost_window_via_cgwindowlist()
    {
        let bundle_id = photoncast_window::get_bundle_id_for_pid(window_info.owner_pid);
        let title = if window_info.title.is_empty() {
            None
        } else {
            Some(window_info.title)
        };
        tracing::debug!(
            "CGWindowList at execution: bundle_id={:?}, title={:?}, owner={}",
            bundle_id,
            title,
            window_info.owner_name
        );
        (bundle_id, title)
    } else {
        tracing::warn!("CGWindowList returned no windows at execution time, using passed target");
        (target_bundle_id.clone(), target_window_title.clone())
    };

    let (effective_bundle_id, effective_title) = select_window_command_target(
        actual_bundle_id,
        actual_title,
        target_bundle_id,
        target_window_title,
    );

    // Activate the target app
    let target_app = if let Some(ref bundle_id) = effective_bundle_id {
        // Activate the specific app
        let activation_result = WINDOW_MANAGER.with(|m| m.borrow().activate_app(bundle_id));
        if let Err(e) = activation_result {
            tracing::warn!("Failed to activate target app {}: {}", bundle_id, e);
            // Fall back to activating any visible app
            match WINDOW_MANAGER.with(|m| m.borrow().activate_any_app_except("")) {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("No target app found: {}", e);
                    return;
                },
            }
        } else {
            tracing::info!("Window command targeting app: {}", bundle_id);
            bundle_id.clone()
        }
    } else {
        // No specific target, find any visible app
        match WINDOW_MANAGER.with(|m| m.borrow().activate_any_app_except("")) {
            Ok(bundle_id) => {
                tracing::info!("Window command targeting app: {}", bundle_id);
                bundle_id
            },
            Err(e) => {
                tracing::error!("No target app found for window command: {} - aborting", e);
                return;
            },
        }
    };

    // Poll until app activation completes (max 150ms, poll every 10ms)
    poll_until_app_frontmost(&target_app, 150, 10);

    // Try to focus the correct window
    // First try by title if we have one, then fall back to finding a non-launcher window
    let focused_by_title = if let Some(ref title) = effective_title {
        match WINDOW_MANAGER.with(|m| m.borrow_mut().focus_window_by_title(title)) {
            Ok(()) => {
                tracing::info!("Focused window by title: '{}'", title);
                true
            },
            Err(e) => {
                tracing::warn!("Could not focus window '{}': {}", title, e);
                false
            },
        }
    } else {
        false
    };

    // If we couldn't focus by title, try to find a non-launcher window
    if !focused_by_title {
        match WINDOW_MANAGER.with(|m| m.borrow_mut().focus_first_non_launcher_window()) {
            Ok(()) => {
                tracing::info!("Focused first non-launcher window");
            },
            Err(e) => {
                tracing::warn!("Could not focus non-launcher window: {}", e);
                // Continue anyway - we'll operate on whatever window is frontmost
            },
        }
    }

    // Poll until window focus completes (max 100ms, poll every 10ms)
    poll_until_window_frontmost(
        effective_bundle_id.as_deref(),
        effective_title.as_deref(),
        100,
        10,
    );

    let result = WINDOW_MANAGER.with(|m| {
        let mut manager = m.borrow_mut();
        match command_id {
            "window_move_next_display" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Next)
            },
            "window_move_previous_display" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Previous)
            },
            "window_move_display_1" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Index(0))
            },
            "window_move_display_2" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Index(1))
            },
            "window_move_display_3" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Index(2))
            },
            _ => {
                let layout = photoncast_window::WindowLayout::from_id(command_id)
                    .unwrap_or(photoncast_window::WindowLayout::LeftHalf);
                manager.apply_layout(layout)
            },
        }
    });

    if let Err(e) = result {
        tracing::error!("Window command failed: {}", e);
    }
}

/// Gets the bundle ID and window title of the frontmost application.
/// This is called before Photoncast activates to remember which window was active.
///
/// Uses CGWindowListCopyWindowInfo which doesn't require Accessibility permissions
/// and returns windows in front-to-back z-order, so we get the actual frontmost
/// window even if another app is about to become active.
#[cfg(target_os = "macos")]
fn get_frontmost_window_info() -> (Option<String>, Option<String>) {
    // Use CGWindowList API - works without accessibility permissions and
    // returns the actual frontmost window in z-order, not the "focused" app
    if let Some(window_info) = photoncast_window::get_frontmost_window_via_cgwindowlist() {
        // Get bundle ID from the PID
        let bundle_id = photoncast_window::get_bundle_id_for_pid(window_info.owner_pid);
        let title = if window_info.title.is_empty() {
            None
        } else {
            Some(window_info.title)
        };

        tracing::debug!(
            "CGWindowList: bundle_id={:?}, title={:?}, owner={}",
            bundle_id,
            title,
            window_info.owner_name
        );

        return (bundle_id, title);
    }

    // Fallback to NSWorkspace if CGWindowList fails
    tracing::warn!("CGWindowList returned no windows, falling back to NSWorkspace");
    use objc2_app_kit::NSWorkspace;

    let workspace = NSWorkspace::sharedWorkspace();
    let app = match workspace.frontmostApplication() {
        Some(a) => a,
        None => return (None, None),
    };
    let bundle_id = app.bundleIdentifier().map(|s| s.to_string());

    // Try to get the frontmost window title using Accessibility API
    let window_title = get_frontmost_window_title();

    (bundle_id, window_title)
}

#[cfg(target_os = "macos")]
fn get_frontmost_window_title() -> Option<String> {
    // Use the singleton window manager to get the frontmost window
    WINDOW_MANAGER.with(|m| {
        let mut manager = m.borrow_mut();

        // Check permission first
        if !manager.has_accessibility_permission() {
            return None;
        }

        // Get frontmost window
        match manager.get_frontmost_window_info() {
            Ok(info) => Some(info.title),
            Err(_) => None,
        }
    })
}

#[cfg(not(target_os = "macos"))]
fn get_frontmost_window_info() -> (Option<String>, Option<String>) {
    (None, None)
}

/// Opens a new launcher window and returns its handle
fn open_launcher_window(
    cx: &mut AppContext,
    launcher_state: &LauncherSharedState,
) -> Option<WindowHandle<LauncherWindow>> {
    let launcher_state = launcher_state.clone();
    match cx.open_window(
        WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(calculate_window_bounds(cx))),
            focus: true,
            show: true,
            // Use a normal window to test whether nonactivating popup/panel
            // semantics are contributing to delayed externally visible
            // presentation in the launcher-appear harness.
            kind: WindowKind::Normal,
            is_movable: false,
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Opaque,
            app_id: Some("app.photoncast".to_string()),
            window_min_size: Some(size(LAUNCHER_WIDTH, LAUNCHER_HEIGHT)),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| cx.new_view(|cx| LauncherWindow::new(cx, &launcher_state)),
    ) {
        Ok(handle) => Some(handle),
        Err(e) => {
            error!("Failed to create launcher window: {}", e);
            None
        },
    }
}

/// Opens a centered window on the primary display with the given size and options.
///
/// This is a shared helper that extracts the common display-bounds centering
/// logic used by most window-opening functions.
fn open_window_centered<V: 'static + Render>(
    cx: &mut AppContext,
    window_size: Size<Pixels>,
    y_offset_percent: f32,
    build_options: impl FnOnce(Bounds<Pixels>, Option<DisplayId>) -> WindowOptions,
    build_view: impl FnOnce(&mut WindowContext) -> View<V> + 'static,
    window_name: &str,
) -> Option<WindowHandle<V>> {
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    let x = display_bounds.origin.x + (display_bounds.size.width - window_size.width) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * y_offset_percent;

    let bounds = Bounds {
        origin: point(x, y),
        size: window_size,
    };

    let display_id = cx.displays().first().map(|d| d.id());

    match cx.open_window(build_options(bounds, display_id), build_view) {
        Ok(handle) => {
            info!("{} window opened", window_name);
            Some(handle)
        },
        Err(e) => {
            error!("Failed to create {} window: {}", window_name, e);
            None
        },
    }
}

/// Opens a new clipboard history window and returns its handle
fn open_clipboard_window(
    cx: &mut AppContext,
    clipboard_state: &Arc<RwLock<ClipboardState>>,
) -> Option<WindowHandle<ClipboardHistoryView>> {
    let state = clipboard_state.read();
    let storage = state.storage.clone();
    let config = state.config.clone();
    drop(state);

    let mut config = config;
    if config.history_size == 0 {
        config.history_size = 100;
    }

    open_window_centered(
        cx,
        size(LAUNCHER_WIDTH, EXPANDED_HEIGHT),
        0.25,
        |bounds, display_id| WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Clipboard History".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.0), px(9.0))),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            display_id,
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.clipboard".to_string()),
            window_min_size: Some(size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(|cx| ClipboardHistoryView::new(storage, config, cx))
        },
        "Clipboard history",
    )
}

/// Opens a new quick links window and returns its handle
fn open_quicklinks_window(
    cx: &mut AppContext,
) -> Option<WindowHandle<photoncast_quicklinks::ui::QuickLinksView>> {
    open_window_centered(
        cx,
        size(MODAL_WIDTH, px(420.0)),
        0.25,
        |bounds, display_id| WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Quick Links".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.0), px(9.0))),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            display_id,
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.quicklinks".to_string()),
            window_min_size: Some(size(px(420.0), px(300.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(photoncast_quicklinks::ui::QuickLinksView::new)
        },
        "Quick links",
    )
}

/// Preferences window dimensions
const PREFS_WIDTH: Pixels = px(864.0);
const PREFS_HEIGHT: Pixels = px(1040.0);

/// Opens a new preferences window and returns its handle
fn open_preferences_window(
    cx: &mut AppContext,
    photoncast_app: Option<
        std::sync::Arc<parking_lot::RwLock<photoncast_core::app::PhotonCastApp>>,
    >,
) -> Option<WindowHandle<PreferencesWindow>> {
    open_window_centered(
        cx,
        size(PREFS_WIDTH, PREFS_HEIGHT),
        0.2,
        |bounds, display_id| WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Preferences".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.0), px(9.0))),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: true,
            display_id,
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.preferences".to_string()),
            window_min_size: Some(size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(|cx| PreferencesWindow::new(cx, photoncast_app))
        },
        "Preferences",
    )
}

/// Opens a new create quicklink window and returns its handle
fn open_create_quicklink_window(
    cx: &mut AppContext,
    storage: photoncast_quicklinks::QuickLinksStorage,
    runtime_handle: tokio::runtime::Handle,
    launcher_state: &LauncherSharedState,
) -> Option<WindowHandle<CreateQuicklinkView>> {
    let launcher_state = launcher_state.clone();
    open_window_centered(
        cx,
        size(MODAL_WIDTH, px(680.0)),
        0.15,
        |bounds, display_id| WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: true,
            display_id,
            window_background: WindowBackgroundAppearance::Opaque,
            app_id: Some("app.photoncast.createquicklink".to_string()),
            window_min_size: Some(size(px(420.0), px(400.0))),
            window_decorations: None,
        },
        move |cx| {
            cx.activate(true);
            let storage_clone = storage.clone();
            let handle = runtime_handle.clone();
            cx.new_view(|cx| {
                let mut view = CreateQuicklinkView::new(cx);
                view.on_event(move |event, cx| {
                    use photoncast_quicklinks::ui::CreateQuicklinkEvent;
                    match event {
                        CreateQuicklinkEvent::Created(link) => {
                            info!("Creating quicklink: {}", link.name);
                            let storage = storage_clone.clone();
                            let link = link.clone();
                            handle.spawn(async move {
                                if let Err(e) = storage.store(&link).await {
                                    error!("Failed to store quicklink: {}", e);
                                } else {
                                    info!("Quicklink created successfully");
                                }
                            });
                            // Invalidate quicklinks cache so new quicklink appears in search
                            launcher_state.invalidate_quicklinks_cache();
                            cx.remove_window();
                        },
                        CreateQuicklinkEvent::Updated(link) => {
                            info!("Updating quicklink: {}", link.name);
                            let storage = storage_clone.clone();
                            let link = link.clone();
                            handle.spawn(async move {
                                if let Err(e) = storage.update(&link).await {
                                    error!("Failed to update quicklink: {}", e);
                                } else {
                                    info!("Quicklink updated successfully");
                                }
                            });
                            // Invalidate quicklinks cache so updated quicklink appears in search
                            launcher_state.invalidate_quicklinks_cache();
                            cx.remove_window();
                        },
                        CreateQuicklinkEvent::Cancelled => {
                            // Window already closed by cancel()
                        },
                    }
                });
                view
            })
        },
        "Create quicklink",
    )
}

/// Argument Input window dimensions
const ARGUMENT_INPUT_WIDTH: Pixels = px(480.0);
const ARGUMENT_INPUT_HEIGHT: Pixels = px(320.0);

/// Opens a new argument input window for a quicklink and returns its handle
fn open_argument_input_window(
    cx: &mut AppContext,
    quicklink: photoncast_quicklinks::QuickLink,
) -> Option<WindowHandle<ArgumentInputView>> {
    open_window_centered(
        cx,
        size(ARGUMENT_INPUT_WIDTH, ARGUMENT_INPUT_HEIGHT),
        0.25,
        |bounds, display_id| WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: true,
            display_id,
            window_background: WindowBackgroundAppearance::Opaque,
            app_id: Some("app.photoncast.argumentinput".to_string()),
            window_min_size: Some(size(px(360.0), px(200.0))),
            window_decorations: None,
        },
        move |cx| {
            cx.activate(true);
            cx.new_view(|cx| {
                let mut view = ArgumentInputView::new(quicklink.clone(), cx);
                view.on_event(|event, cx| match event {
                    ArgumentInputEvent::Submitted { final_url, .. } => {
                        info!("Opening quicklink URL: {}", final_url);
                        if let Err(e) = photoncast_core::platform::launch::open_url(&final_url) {
                            error!("Failed to open quicklink URL: {}", e);
                        }
                        cx.remove_window();
                    },
                    ArgumentInputEvent::Cancelled => {
                        cx.remove_window();
                    },
                });
                view
            })
        },
        "Argument input",
    )
}

/// Opens a new manage quicklinks window and returns its handle
fn open_manage_quicklinks_window(
    cx: &mut AppContext,
    storage: photoncast_quicklinks::QuickLinksStorage,
    runtime: &Arc<tokio::runtime::Runtime>,
    show_library: bool,
    launcher_state: &LauncherSharedState,
) -> Option<WindowHandle<QuicklinksManageView>> {
    let launcher_state = launcher_state.clone();
    let runtime = Arc::clone(runtime);

    // Open the window immediately with an empty quicklinks list.
    // Callers (event_loop) load quicklinks asynchronously via
    // cx.background_executor().spawn() and update the view afterward.
    open_window_centered(
        cx,
        size(LAUNCHER_WIDTH, EXPANDED_HEIGHT),
        0.15,
        |bounds, display_id| WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: true,
            display_id,
            window_background: WindowBackgroundAppearance::Opaque,
            app_id: Some("app.photoncast.managequicklinks".to_string()),
            window_min_size: Some(size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)),
            window_decorations: None,
        },
        move |cx| {
            cx.activate(true);
            cx.new_view(|cx| {
                let mut view = QuicklinksManageView::new(cx);
                view.set_storage(storage.clone(), runtime);
                if show_library {
                    view.toggle_library(cx);
                }
                // Invalidate quicklinks cache when quicklinks are modified
                let photoncast_app = launcher_state.photoncast_app();
                view.on_change(move || {
                    photoncast_app.read().invalidate_quicklinks_cache();
                });
                view
            })
        },
        "Manage quicklinks",
    )
}

/// Timer window dimensions
const TIMER_WIDTH: Pixels = px(360.0);
const TIMER_HEIGHT: Pixels = px(220.0);

/// Opens a new timer window and returns its handle
fn open_timer_window(
    cx: &mut AppContext,
) -> Option<WindowHandle<photoncast_timer::ui::TimerDisplay>> {
    open_window_centered(
        cx,
        size(TIMER_WIDTH, TIMER_HEIGHT),
        0.25,
        |bounds, display_id| WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Sleep Timer".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.0), px(9.0))),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            display_id,
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.timer".to_string()),
            window_min_size: Some(size(px(300.0), px(180.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(photoncast_timer::ui::TimerDisplay::new)
        },
        "Timer",
    )
}

/// Calculate initial window bounds centered at top of screen
fn calculate_window_bounds(cx: &AppContext) -> Bounds<Pixels> {
    // Get the primary display bounds
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    // Calculate centered-top position
    let window_width = LAUNCHER_WIDTH;
    let window_height = LAUNCHER_HEIGHT;

    let x = display_bounds.origin.x + (display_bounds.size.width - window_width) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * LAUNCHER_TOP_OFFSET_PERCENT;

    Bounds {
        origin: point(x, y),
        size: size(window_width, window_height),
    }
}

/// Register all key bindings for the launcher
fn register_key_bindings(cx: &mut AppContext) {
    // Register extension view key bindings
    extension_views::list_view::register_key_bindings(cx);
    extension_views::detail_view::register_key_bindings(cx);
    extension_views::form_view::register_key_bindings(cx);
    extension_views::grid_view::register_key_bindings(cx);
    extension_views::register_navigation_key_bindings(cx);

    cx.bind_keys([
        // Navigation
        KeyBinding::new("down", SelectNext, Some("LauncherWindow")),
        KeyBinding::new("up", SelectPrevious, Some("LauncherWindow")),
        KeyBinding::new("ctrl-n", SelectNext, Some("LauncherWindow")),
        KeyBinding::new("ctrl-p", SelectPrevious, Some("LauncherWindow")),
        // Activation
        KeyBinding::new("enter", Activate, Some("LauncherWindow")),
        // Cancel/Close
        KeyBinding::new("escape", Cancel, Some("LauncherWindow")),
        // Quick selection (⌘1-9)
        KeyBinding::new("cmd-1", QuickSelect1, Some("LauncherWindow")),
        KeyBinding::new("cmd-2", QuickSelect2, Some("LauncherWindow")),
        KeyBinding::new("cmd-3", QuickSelect3, Some("LauncherWindow")),
        KeyBinding::new("cmd-4", QuickSelect4, Some("LauncherWindow")),
        KeyBinding::new("cmd-5", QuickSelect5, Some("LauncherWindow")),
        KeyBinding::new("cmd-6", QuickSelect6, Some("LauncherWindow")),
        KeyBinding::new("cmd-7", QuickSelect7, Some("LauncherWindow")),
        KeyBinding::new("cmd-8", QuickSelect8, Some("LauncherWindow")),
        KeyBinding::new("cmd-9", QuickSelect9, Some("LauncherWindow")),
        // Group cycling
        KeyBinding::new("tab", NextGroup, Some("LauncherWindow")),
        KeyBinding::new("shift-tab", PreviousGroup, Some("LauncherWindow")),
        // Preferences
        KeyBinding::new("cmd-,", OpenPreferences, Some("LauncherWindow")),
        // File Search Mode actions
        KeyBinding::new("cmd-enter", RevealInFinder, Some("LauncherWindow")),
        KeyBinding::new("cmd-y", QuickLook, Some("LauncherWindow")),
        KeyBinding::new("cmd-c", CopyPath, Some("LauncherWindow")),
        KeyBinding::new("cmd-shift-c", CopyFile, Some("LauncherWindow")),
        // Actions menu
        KeyBinding::new("cmd-k", ShowActionsMenu, Some("LauncherWindow")),
        // Task 7.4: App Management keyboard shortcuts
        KeyBinding::new("cmd-shift-f", ShowInFinder, Some("LauncherWindow")),
        KeyBinding::new("cmd-shift-b", CopyBundleId, Some("LauncherWindow")),
        KeyBinding::new("cmd-q", QuitApp, Some("LauncherWindow")),
        KeyBinding::new("cmd-alt-q", ForceQuitApp, Some("LauncherWindow")),
        KeyBinding::new("cmd-h", HideApp, Some("LauncherWindow")),
        KeyBinding::new("cmd-backspace", UninstallApp, Some("LauncherWindow")),
        KeyBinding::new("cmd-shift-a", ToggleAutoQuit, Some("LauncherWindow")),
        // Clipboard history (global - no context, works from anywhere)
        KeyBinding::new("cmd-shift-v", OpenClipboardHistory, None),
        // Clipboard history actions
        KeyBinding::new("cmd-c", CopyClipboardItem, Some("ClipboardHistory")),
        KeyBinding::new("cmd-shift-v", PasteAsPlainText, Some("ClipboardHistory")),
        KeyBinding::new("cmd-p", TogglePinClipboardItem, Some("ClipboardHistory")),
        KeyBinding::new(
            "cmd-backspace",
            DeleteClipboardItem,
            Some("ClipboardHistory"),
        ),
    ]);
}

/// Checks if Spotlight's Cmd+Space shortcut is enabled and logs a warning if so.
///
/// This is a non-blocking, informational check. PhotonCast will still start
/// even if there's a conflict, but the user will be warned about it.
fn check_spotlight_conflict() {
    // PhotonCast uses Cmd+Space as the default hotkey
    if is_spotlight_enabled() {
        warn!(
            "Spotlight's Cmd+Space shortcut may conflict with PhotonCast. \
             To disable: System Settings > Keyboard > Keyboard Shortcuts > Spotlight > \
             uncheck 'Show Spotlight search'"
        );
    }
}

/// Checks and logs the current login item status.
///
/// This is informational only - it logs whether PhotonCast is set to
/// launch at login so users know the current state.
fn check_login_item_status() {
    let mut manager = LoginItemManager::for_photoncast();
    match manager.check_status() {
        Ok(status) => {
            info!("Launch at login: {}", status.description());
        },
        Err(e) => {
            warn!("Could not check login item status: {}", e);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::select_window_command_target;

    #[test]
    fn test_select_window_command_target_prefers_captured_app_and_title() {
        let (bundle_id, title) = select_window_command_target(
            Some("com.apple.TextEdit".to_string()),
            Some("Notes".to_string()),
            Some("com.apple.Safari".to_string()),
            Some("Inbox".to_string()),
        );

        assert_eq!(bundle_id.as_deref(), Some("com.apple.Safari"));
        assert_eq!(title.as_deref(), Some("Inbox"));
    }

    #[test]
    fn test_select_window_command_target_uses_actual_title_when_bundle_matches() {
        let (bundle_id, title) = select_window_command_target(
            Some("com.apple.Safari".to_string()),
            Some("Inbox".to_string()),
            Some("com.apple.Safari".to_string()),
            None,
        );

        assert_eq!(bundle_id.as_deref(), Some("com.apple.Safari"));
        assert_eq!(title.as_deref(), Some("Inbox"));
    }

    #[test]
    fn test_select_window_command_target_falls_back_to_actual_when_capture_missing() {
        let (bundle_id, title) = select_window_command_target(
            Some("com.apple.TextEdit".to_string()),
            Some("Draft".to_string()),
            None,
            None,
        );

        assert_eq!(bundle_id.as_deref(), Some("com.apple.TextEdit"));
        assert_eq!(title.as_deref(), Some("Draft"));
    }
}
