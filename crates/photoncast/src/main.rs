//! PhotonCast - Lightning-fast macOS launcher built in pure Rust
//!
//! This is the main entry point for the PhotonCast application.
//! It initializes GPUI, creates the launcher window, and runs the event loop.

// Clippy configuration for binary crate
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::single_match_else)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::option_map_or_none)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::manual_filter_map)]
#![allow(clippy::unit_arg)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::unnecessary_filter_map)]
#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::unsafe_derive_deserialize)]

use std::cell::RefCell;
use std::sync::mpsc;
use std::time::Duration;

use std::sync::Arc;

use gpui::*;
use parking_lot::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod app_events;
mod launcher;
mod platform;
mod preferences_window;

use app_events::AppEvent;
use launcher::{LauncherSharedState, LauncherWindow};
use photoncast_clipboard::ui::{
    ClipboardHistoryView, CopyClipboardItem, DeleteClipboardItem, PasteAsPlainText,
    TogglePinClipboardItem,
};
use photoncast_quicklinks::ui::{ArgumentInputEvent, ArgumentInputView, CreateQuicklinkView, QuicklinksManageView};
use photoncast_clipboard::{ClipboardConfig, ClipboardMonitor, ClipboardStorage};
use photoncast_core::app::config::{Config, ThemeSetting};
use photoncast_core::platform::accessibility::{
    check_accessibility_permission, request_accessibility_permission,
};
use photoncast_core::platform::appearance::flavor_from_window_appearance;
use photoncast_core::platform::hotkey::is_spotlight_enabled;
use photoncast_core::platform::LoginItemManager;
use photoncast_core::theme::PhotonTheme;
use platform::{create_menu_bar_item, MenuBarActionKind};
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

/// Window dimensions constants (matching Raycast sizing)
const LAUNCHER_WIDTH: Pixels = px(750.0);
const LAUNCHER_HEIGHT: Pixels = px(475.0);
const LAUNCHER_BORDER_RADIUS: Pixels = px(12.0);

/// Position from top of screen (20%)
const LAUNCHER_TOP_OFFSET_PERCENT: f32 = 0.20;

/// Shared clipboard state
struct ClipboardState {
    storage: ClipboardStorage,
    config: ClipboardConfig,
    #[allow(dead_code)]
    monitor: Option<Arc<ClipboardMonitor>>,
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting PhotonCast v{}", env!("CARGO_PKG_VERSION"));

    // Load config and apply settings at startup
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

    // Check for hotkey conflicts with Spotlight (non-blocking, informational only)
    check_spotlight_conflict();

    // Check and log login item status
    check_login_item_status();

    // Create channel for app events (hotkey, menu bar)
    let (event_tx, event_rx) = mpsc::channel::<AppEvent>();
    app_events::set_event_sender(event_tx.clone());

    // Check accessibility permission and register hotkey (non-blocking)
    let hotkey_tx = event_tx.clone();
    let has_permission = check_accessibility_permission();
    if has_permission {
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
        // Request permission without blocking - user will need to restart app after granting
        warn!("Accessibility permission not granted. Requesting...");
        request_accessibility_permission();
        warn!(
            "Please grant accessibility access and restart PhotonCast for global hotkey to work."
        );
    }

    // Initialize clipboard storage
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

    if let Some(monitor) = clipboard_monitor.clone() {
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    error!(
                        "Failed to create Tokio runtime for clipboard monitor: {}",
                        e
                    );
                    return;
                },
            };

            if let Err(e) = rt.block_on(monitor.start()) {
                error!("Clipboard monitor stopped with error: {}", e);
            }
        });
    }

    // Wrap clipboard state for sharing
    let clipboard_state = clipboard_storage.map(|storage| {
        Arc::new(RwLock::new(ClipboardState {
            storage,
            config: clipboard_config.clone(),
            monitor: clipboard_monitor,
        }))
    });

    // Initialize and run GPUI application
    App::new().run(move |cx: &mut AppContext| {
        // Initialize theme from config and set as global
        let flavor = app_config.appearance.theme.to_catppuccin_flavor();
        let accent = app_config.appearance.accent_color.to_theme_accent();
        let auto_sync = app_config.appearance.theme == ThemeSetting::Auto;
        let theme = PhotonTheme::new(flavor, accent).with_auto_sync(auto_sync);
        cx.set_global(theme);
        info!(
            "Theme initialized: flavor={:?}, accent={:?}, auto_sync={}",
            flavor, accent, auto_sync
        );

        // Register key bindings
        register_key_bindings(cx);

        // Store clipboard state in global
        let clipboard_for_window = clipboard_state.clone();

        // Create menu bar status item with channel for events
        let menu_tx = event_tx.clone();
        match create_menu_bar_item(move |action| {
            let event = match action {
                MenuBarActionKind::ToggleLauncher => AppEvent::ToggleLauncher,
                MenuBarActionKind::OpenPreferences => AppEvent::OpenPreferences,
                MenuBarActionKind::Quit => AppEvent::QuitApp,
            };
            if let Err(e) = menu_tx.send(event) {
                error!("Failed to send menu bar event: {}", e);
            }
        }) {
            Ok(()) => info!("Menu bar status item created"),
            Err(e) => error!("Failed to create menu bar status item: {}", e),
        }

        // Create initial launcher window
        let launcher_state = LauncherSharedState::new();
        let window_handle = open_launcher_window(cx, &launcher_state);

        // Set up appearance observation for auto theme switching
        let _appearance_subscription = window_handle
            .as_ref()
            .and_then(|handle| setup_appearance_observation(handle, cx));

        // Spawn a task to listen for app events (hotkey, menu bar)
        let clipboard_state_for_events = clipboard_for_window;
        cx.spawn(|mut cx| async move {
            // Track the current window handle (updated when window is recreated)
            let mut current_handle: Option<WindowHandle<LauncherWindow>> = window_handle;
            let launcher_state_for_events = launcher_state.clone();
            let mut clipboard_handle: Option<WindowHandle<ClipboardHistoryView>> = None;
            let mut quicklinks_handle: Option<
                WindowHandle<photoncast_quicklinks::ui::QuickLinksView>,
            > = None;
            let mut timer_handle: Option<WindowHandle<photoncast_timer::ui::TimerDisplay>> = None;
            let mut preferences_handle: Option<WindowHandle<PreferencesWindow>> = None;
            let mut create_quicklink_handle: Option<WindowHandle<CreateQuicklinkView>> = None;
            let mut manage_quicklinks_handle: Option<WindowHandle<QuicklinksManageView>> = None;

            let quicklinks_storage = photoncast_quicklinks::QuickLinksStorage::open(
                photoncast_core::utils::paths::data_dir().join("quicklinks.db"),
            )
            .unwrap_or_else(|e| {
                error!("Failed to open quick links storage: {}", e);
                photoncast_quicklinks::QuickLinksStorage::open_in_memory().unwrap_or_else(|err| {
                    error!("Failed to open in-memory quick links storage: {}", err);
                    photoncast_quicklinks::QuickLinksStorage::open_in_memory()
                        .expect("failed to open in-memory quick links storage")
                })
            });

            // Populate bundled quicklinks on first use
            if let Err(e) = quicklinks_storage.populate_bundled_if_empty() {
                warn!("Failed to populate bundled quicklinks: {}", e);
            }

            let quicklinks_runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
                error!("Failed to create quick links runtime: {}", e);
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to create quick links runtime")
            });

            let app_manager =
                photoncast_apps::AppManager::new(photoncast_apps::AppsConfig::default());
            let calendar_command = photoncast_calendar::CalendarCommand::with_default_config();
            
            // Timer polling setup (runs on main thread, executes actions in background)
            let timer_manager = launcher_state_for_events.timer_manager();
            let timer_event_tx = event_tx.clone();
            let timer_runtime = tokio::runtime::Runtime::new().ok();
            let mut last_timer_check = std::time::Instant::now();

            loop {
                // Poll for events (non-blocking with small sleep)
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;

                // Check timer every second (database read is fast, action executes in background)
                if last_timer_check.elapsed() >= Duration::from_secs(1) {
                    last_timer_check = std::time::Instant::now();
                    if let Some(ref rt) = timer_runtime {
                        // Check if timer expired (fast database read)
                        let expired_action = rt.block_on(async {
                            let mgr = timer_manager.read().await;
                            mgr.check_expired().await
                        });
                        
                        // If expired, execute action in background thread (no rusqlite crossing)
                        if let Ok(Some(action)) = expired_action {
                            let tx = timer_event_tx.clone();
                            let action_name = action.display_name().to_string();
                            std::thread::spawn(move || {
                                if let Ok(rt) = tokio::runtime::Runtime::new() {
                                    rt.block_on(async {
                                        if let Err(e) = photoncast_timer::TimerScheduler::execute_action(action).await {
                                            error!("Timer action failed: {}", e);
                                        }
                                    });
                                }
                                let _ = tx.send(AppEvent::TimerExpired { action: action_name });
                            });
                        }
                    }
                }

                // Check for app events
                match event_rx.try_recv() {
                    Ok(AppEvent::ToggleLauncher) => {
                        info!("Toggle launcher requested - capturing frontmost window NOW");
                        
                        // Capture frontmost app AND window BEFORE Photoncast becomes active
                        // This is used for window management commands to target the correct window
                        let (previous_app, previous_window_title) = get_frontmost_window_info();
                        tracing::info!(
                            "Captured frontmost: app={:?}, window={:?}",
                            previous_app,
                            previous_window_title
                        );
                        
                        // Try to activate existing window or create new one
                        let _ = cx.update(|cx| {
                            // Try to update existing window
                            let window_exists = current_handle.as_ref().is_some_and(|h| {
                                h.update(cx, |view, cx| {
                                    view.set_previous_frontmost_window(previous_app.clone(), previous_window_title.clone());
                                    view.toggle(cx);
                                    cx.activate(true);
                                    cx.activate_window();
                                    cx.focus_self();
                                })
                                .is_ok()
                            });

                            if !window_exists {
                                // Window was closed, open a new one
                                current_handle =
                                    open_launcher_window(cx, &launcher_state_for_events);
                                
                                // Set the captured frontmost window on the new window
                                if let Some(ref h) = current_handle {
                                    let _ = h.update(cx, |view, _cx| {
                                        view.set_previous_frontmost_window(previous_app.clone(), previous_window_title.clone());
                                    });
                                }
                            }
                        });
                    },
                    Ok(AppEvent::OpenPreferences) => {
                        info!("Open preferences requested");
                        let _ = cx.update(|cx| {
                            if let Some(ref h) = preferences_handle {
                                if h.update(cx, |_, cx| {
                                    cx.activate(true);
                                    cx.activate_window();
                                })
                                .is_err()
                                {
                                    preferences_handle = None;
                                }
                            }

                            if preferences_handle.is_none() {
                                if let Some(handle) = open_preferences_window(cx) {
                                    let _ = handle.update(cx, |_, cx| {
                                        cx.activate(true);
                                        cx.activate_window();
                                    });
                                    preferences_handle = Some(handle);
                                }
                            }
                        });
                    },
                    Ok(AppEvent::OpenClipboardHistory) => {
                        info!("Open clipboard history requested");
                        let mut activated = false;
                        if let Some(ref h) = clipboard_handle {
                            if h.update(&mut cx, |view, cx| {
                                view.refresh(cx);
                                cx.activate(true);
                                cx.activate_window();
                                cx.focus_self();
                            })
                            .is_ok()
                            {
                                activated = true;
                            } else {
                                clipboard_handle = None;
                            }
                        }

                        if !activated {
                            if let Some(ref clipboard_state) = clipboard_state_for_events {
                                let state = clipboard_state.clone();
                                let _ = cx.update(|cx| {
                                    if let Some(handle) = open_clipboard_window(cx, &state) {
                                        let _ = handle.update(cx, |view, cx| {
                                            view.refresh(cx);
                                            cx.activate(true);
                                            cx.activate_window();
                                            cx.focus_self();
                                        });
                                        clipboard_handle = Some(handle);
                                    }
                                });
                            } else {
                                warn!("Clipboard not available");
                            }
                        }
                    },
                    Ok(AppEvent::OpenQuickLinks) => {
                        info!("Open quick links requested");
                        let mut activated = false;
                        if let Some(ref h) = quicklinks_handle {
                            if h.update(&mut cx, |view, cx| {
                                let links = quicklinks_runtime
                                    .block_on(quicklinks_storage.load_all())
                                    .unwrap_or_default();
                                view.set_links(links, cx);
                                cx.activate(true);
                                cx.activate_window();
                            })
                            .is_ok()
                            {
                                activated = true;
                            } else {
                                quicklinks_handle = None;
                            }
                        }

                        if !activated {
                            let storage = quicklinks_storage.clone();
                            let runtime = quicklinks_runtime.handle().clone();
                            let _ = cx.update(|cx| {
                                if let Some(handle) = open_quicklinks_window(cx) {
                                    let _ = handle.update(cx, |view, cx| {
                                        let links = runtime
                                            .block_on(storage.load_all())
                                            .unwrap_or_default();
                                        view.set_links(links, cx);
                                        cx.activate(true);
                                        cx.activate_window();
                                    });
                                    quicklinks_handle = Some(handle);
                                }
                            });
                        }
                    },
                    Ok(AppEvent::OpenCalendar { command_id }) => {
                        info!("Open calendar requested: {}", command_id);
                        let (title, result) = match command_id.as_str() {
                            "calendar_today" => {
                                ("Today's Events", calendar_command.fetch_today_events())
                            },
                            "calendar_week" => ("This Week", calendar_command.fetch_week_events()),
                            "calendar_upcoming" => {
                                ("My Schedule", calendar_command.fetch_upcoming_events(7))
                            },
                            other => {
                                warn!("Unknown calendar command: {}", other);
                                ("My Schedule", calendar_command.fetch_upcoming_events(7))
                            },
                        };

                        match result {
                            Ok(events) => {
                                info!("Calendar fetch returned {} events", events.len());
                                let title = title.to_string();
                                let events = events.clone();
                                let _ = cx.update(|cx| {
                                    if let Some(ref h) = current_handle {
                                        let _ = h.update(cx, |view, cx| {
                                            view.show_calendar(title, events, cx);
                                            cx.activate(true);
                                            cx.activate_window();
                                            cx.focus_self();
                                        });
                                    } else {
                                        current_handle =
                                            open_launcher_window(cx, &launcher_state_for_events);
                                    }
                                });
                            },
                            Err(err) => {
                                warn!("Calendar fetch failed: {}", err);
                                let title = title.to_string();
                                let error = err.to_string();
                                let _ = cx.update(|cx| {
                                    if let Some(ref h) = current_handle {
                                        let _ = h.update(cx, |view, cx| {
                                            view.show_calendar_error(title, error, cx);
                                            cx.activate(true);
                                            cx.activate_window();
                                            cx.focus_self();
                                        });
                                    } else {
                                        current_handle =
                                            open_launcher_window(cx, &launcher_state_for_events);
                                    }
                                });
                            },
                        }
                    },
                    Ok(AppEvent::OpenSleepTimer { expression }) => {
                        info!("Open sleep timer requested: {}", expression);
                        let mut activated = false;
                        if let Some(ref h) = timer_handle {
                            if h.update(&mut cx, |view, cx| {
                                view.set_timer(None, cx);
                                cx.activate(true);
                                cx.activate_window();
                            })
                            .is_ok()
                            {
                                activated = true;
                            } else {
                                timer_handle = None;
                            }
                        }

                        if !activated {
                            let _ = cx.update(|cx| {
                                if let Some(handle) = open_timer_window(cx) {
                                    let _ = handle.update(cx, |view, cx| {
                                        view.set_timer(None, cx);
                                        cx.activate(true);
                                        cx.activate_window();
                                    });
                                    timer_handle = Some(handle);
                                }
                            });
                        }
                    },
                    Ok(AppEvent::OpenApps { command_id }) => {
                        info!("Open apps management requested: {}", command_id);
                        if let Err(e) = app_manager.get_running_apps() {
                            warn!("Apps management failed: {}", e);
                        }
                    },
                    Ok(AppEvent::CreateQuicklink) => {
                        info!("Create quicklink requested");
                        let _ = cx.update(|cx| {
                            // Try to activate existing window
                            if let Some(ref h) = create_quicklink_handle {
                                if h.update(cx, |_, cx| {
                                    cx.activate(true);
                                    cx.activate_window();
                                })
                                .is_err()
                                {
                                    create_quicklink_handle = None;
                                }
                            }

                            // Create new window if needed
                            if create_quicklink_handle.is_none() {
                                if let Some(handle) =
                                    open_create_quicklink_window(cx, quicklinks_storage.clone())
                                {
                                    let _ = handle.update(cx, |_, cx| {
                                        cx.activate(true);
                                        cx.activate_window();
                                    });
                                    create_quicklink_handle = Some(handle);
                                }
                            }
                        });
                    },
                    Ok(AppEvent::ManageQuicklinks) => {
                        info!("Manage quicklinks requested");
                        let _ = cx.update(|cx| {
                            // Try to activate existing window
                            if let Some(ref h) = manage_quicklinks_handle {
                                if h.update(cx, |view, cx| {
                                    // Load fresh quicklinks data
                                    let links = quicklinks_runtime
                                        .block_on(quicklinks_storage.load_all())
                                        .unwrap_or_default();
                                    view.set_quicklinks(links, cx);
                                    cx.activate(true);
                                    cx.activate_window();
                                })
                                .is_err()
                                {
                                    manage_quicklinks_handle = None;
                                }
                            }

                            // Create new window if needed
                            if manage_quicklinks_handle.is_none() {
                                if let Some(handle) = open_manage_quicklinks_window(
                                    cx,
                                    quicklinks_storage.clone(),
                                    &quicklinks_runtime,
                                    false,
                                ) {
                                    let _ = handle.update(cx, |_, cx| {
                                        cx.activate(true);
                                        cx.activate_window();
                                    });
                                    manage_quicklinks_handle = Some(handle);
                                }
                            }
                        });
                    },
                    Ok(AppEvent::BrowseQuicklinkLibrary) => {
                        info!("Browse quicklink library requested");
                        let _ = cx.update(|cx| {
                            // Try to activate existing window
                            if let Some(ref h) = manage_quicklinks_handle {
                                if h.update(cx, |view, cx| {
                                    // Load fresh quicklinks data
                                    let links = quicklinks_runtime
                                        .block_on(quicklinks_storage.load_all())
                                        .unwrap_or_default();
                                    view.set_quicklinks(links, cx);
                                    // Make sure library is showing
                                    if !view.is_showing_library() {
                                        view.toggle_library(cx);
                                    }
                                    cx.activate(true);
                                    cx.activate_window();
                                })
                                .is_err()
                                {
                                    manage_quicklinks_handle = None;
                                }
                            }

                            // Create new window if needed
                            if manage_quicklinks_handle.is_none() {
                                if let Some(handle) = open_manage_quicklinks_window(
                                    cx,
                                    quicklinks_storage.clone(),
                                    &quicklinks_runtime,
                                    true, // show_library
                                ) {
                                    let _ = handle.update(cx, |_, cx| {
                                        cx.activate(true);
                                        cx.activate_window();
                                    });
                                    manage_quicklinks_handle = Some(handle);
                                }
                            }
                        });
                    },
                    Ok(AppEvent::ExecuteQuickLink {
                        id,
                        url_template,
                        arguments,
                    }) => {
                        // Substitute arguments if provided
                        let final_url = if !arguments.is_empty() {
                            photoncast_quicklinks::placeholder::substitute_argument(
                                &url_template,
                                &arguments,
                            )
                        } else {
                            url_template.clone()
                        };

                        // Check if URL still requires user input
                        if photoncast_quicklinks::placeholder::requires_user_input(&final_url) {
                            // Load quicklink and open argument input UI
                            let storage = quicklinks_storage.clone();
                            let _ = cx.update(|cx| {
                                if let Ok(links) = quicklinks_runtime.block_on(storage.load_all()) {
                                    if let Some(link) = links.into_iter().find(|l| l.id.as_str() == id) {
                                        if let Some(handle) = open_argument_input_window(cx, link) {
                                            let _ = handle.update(cx, |_, cx| {
                                                cx.activate(true);
                                                cx.activate_window();
                                            });
                                        }
                                    }
                                }
                            });
                        } else {
                            info!("Execute quicklink: {}", final_url);
                            if let Err(e) = photoncast_core::platform::launch::open_url(&final_url) {
                                error!("Failed to open quicklink URL: {}", e);
                            }
                        }
                    },
                    Ok(AppEvent::QuitApp) => {
                        info!("Quit requested from menu bar");
                        let _ = cx.update(|cx| {
                            cx.quit();
                        });
                        break;
                    },
                    Ok(AppEvent::TimerExpired { action }) => {
                        info!("Timer expired, action executed: {}", action);
                        // Timer action already executed in background thread
                        // This event is just for logging/UI notification if needed
                    },
                    Ok(AppEvent::ExecuteWindowCommand { command_id, target_bundle_id, target_window_title }) => {
                        info!("Window command requested: {}", command_id);
                        // Execute window command outside of any GPUI window context
                        // to avoid reentrancy panics when macOS sends windowDidMove notifications
                        execute_window_command(&command_id, target_bundle_id, target_window_title);
                    },
                    Err(mpsc::TryRecvError::Empty) => {
                        // No event, continue
                    },
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Channel closed, stop listening
                        info!("Event channel closed");
                        break;
                    },
                }
            }
        })
        .detach();

        info!("PhotonCast initialized successfully");
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
            m.borrow().get_frontmost_bundle_id()
                .map(|f| f == target_bundle_id)
                .unwrap_or(false)
        });
        if is_frontmost {
            tracing::debug!("App {} became frontmost after {:?}", target_bundle_id, start.elapsed());
            return true;
        }
        std::thread::sleep(interval);
    }
    
    tracing::warn!("Timeout waiting for app {} to become frontmost", target_bundle_id);
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
                    start.elapsed(), current_bundle, window_info.title
                );
                return true;
            }
        }
        std::thread::sleep(interval);
    }
    
    tracing::warn!(
        "Timeout waiting for window to become frontmost: bundle={:?}, title={:?}",
        expected_bundle_id, expected_title
    );
    false
}

/// Executes a window management command outside of GPUI window context.
/// This avoids reentrancy panics when moving windows triggers windowDidMove notifications.
fn execute_window_command(command_id: &str, target_bundle_id: Option<String>, target_window_title: Option<String>) {
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
    let (actual_bundle_id, actual_title) = if let Some(window_info) = photoncast_window::get_frontmost_window_via_cgwindowlist() {
        let bundle_id = photoncast_window::get_bundle_id_for_pid(window_info.owner_pid);
        let title = if window_info.title.is_empty() { None } else { Some(window_info.title) };
        tracing::info!(
            "CGWindowList at execution: bundle_id={:?}, title={:?}, owner={}",
            bundle_id, title, window_info.owner_name
        );
        (bundle_id, title)
    } else {
        tracing::warn!("CGWindowList returned no windows at execution time, using passed target");
        (target_bundle_id.clone(), target_window_title.clone())
    };
    
    // Prefer the CGWindowList result, fall back to passed target if CGWindowList failed
    let effective_bundle_id = actual_bundle_id.or(target_bundle_id);
    let effective_title = actual_title.or(target_window_title);

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
                }
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
            }
            Err(e) => {
                tracing::error!("No target app found for window command: {} - aborting", e);
                return;
            }
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
            }
            Err(e) => {
                tracing::warn!("Could not focus window '{}': {}", title, e);
                false
            }
        }
    } else {
        false
    };

    // If we couldn't focus by title, try to find a non-launcher window
    if !focused_by_title {
        match WINDOW_MANAGER.with(|m| m.borrow_mut().focus_first_non_launcher_window()) {
            Ok(()) => {
                tracing::info!("Focused first non-launcher window");
            }
            Err(e) => {
                tracing::warn!("Could not focus non-launcher window: {}", e);
                // Continue anyway - we'll operate on whatever window is frontmost
            }
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
            }
            "window_move_previous_display" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Previous)
            }
            "window_move_display_1" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Index(0))
            }
            "window_move_display_2" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Index(1))
            }
            "window_move_display_3" => {
                manager.move_to_display(photoncast_window::DisplayDirection::Index(2))
            }
            _ => {
                let layout = photoncast_window::WindowLayout::from_id(command_id)
                    .unwrap_or(photoncast_window::WindowLayout::LeftHalf);
                manager.apply_layout(layout)
            }
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
        
        tracing::info!(
            "CGWindowList: bundle_id={:?}, title={:?}, owner={}",
            bundle_id, title, window_info.owner_name
        );
        
        return (bundle_id, title);
    }
    
    // Fallback to NSWorkspace if CGWindowList fails
    tracing::warn!("CGWindowList returned no windows, falling back to NSWorkspace");
    use objc2_app_kit::NSWorkspace;
    
    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let app = match unsafe { workspace.frontmostApplication() } {
        Some(a) => a,
        None => return (None, None),
    };
    let bundle_id = unsafe { app.bundleIdentifier() }.map(|s| s.to_string());
    
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

/// Sets up system appearance observation to automatically update the theme
/// when macOS switches between light and dark mode.
fn setup_appearance_observation(
    window_handle: &WindowHandle<LauncherWindow>,
    cx: &mut AppContext,
) -> Option<Subscription> {
    window_handle
        .update(cx, |_view, cx| {
            cx.observe_window_appearance(|_view, cx| {
                let appearance = cx.window_appearance();
                let current_theme = cx.try_global::<PhotonTheme>().cloned();

                if let Some(theme) = current_theme {
                    if theme.auto_sync {
                        let new_flavor = flavor_from_window_appearance(appearance);
                        if theme.flavor != new_flavor {
                            info!(
                                "System appearance changed: {:?} -> {:?}",
                                theme.flavor, new_flavor
                            );
                            let new_theme = PhotonTheme::new(new_flavor, theme.accent)
                                .with_auto_sync(true);
                            cx.set_global(new_theme);
                            cx.refresh();
                        }
                    }
                }
            })
        })
        .ok()
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
            kind: WindowKind::Normal,
            is_movable: false,
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast".to_string()),
            window_min_size: Some(size(LAUNCHER_WIDTH, LAUNCHER_HEIGHT)),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(|cx| LauncherWindow::new(cx, &launcher_state))
        },
    ) {
        Ok(handle) => Some(handle),
        Err(e) => {
            error!("Failed to create launcher window: {}", e);
            None
        },
    }
}

/// Clipboard window dimensions
const CLIPBOARD_WIDTH: Pixels = px(500.0);
const CLIPBOARD_HEIGHT: Pixels = px(450.0);

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

    // Calculate window bounds (centered)
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    let x = display_bounds.origin.x + (display_bounds.size.width - CLIPBOARD_WIDTH) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * 0.25;

    let bounds = Bounds {
        origin: point(x, y),
        size: size(CLIPBOARD_WIDTH, CLIPBOARD_HEIGHT),
    };

    match cx.open_window(
        WindowOptions {
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
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.clipboard".to_string()),
            window_min_size: Some(size(px(400.0), px(300.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(|cx| ClipboardHistoryView::new(storage, config, cx))
        },
    ) {
        Ok(handle) => {
            info!("Clipboard history window opened");
            Some(handle)
        },
        Err(e) => {
            error!("Failed to create clipboard history window: {}", e);
            None
        },
    }
}

/// Quick links window dimensions
const QUICKLINKS_WIDTH: Pixels = px(520.0);
const QUICKLINKS_HEIGHT: Pixels = px(420.0);

/// Opens a new quick links window and returns its handle
fn open_quicklinks_window(
    cx: &mut AppContext,
) -> Option<WindowHandle<photoncast_quicklinks::ui::QuickLinksView>> {
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    let x = display_bounds.origin.x + (display_bounds.size.width - QUICKLINKS_WIDTH) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * 0.25;

    let bounds = Bounds {
        origin: point(x, y),
        size: size(QUICKLINKS_WIDTH, QUICKLINKS_HEIGHT),
    };

    match cx.open_window(
        WindowOptions {
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
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.quicklinks".to_string()),
            window_min_size: Some(size(px(420.0), px(300.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(|cx| photoncast_quicklinks::ui::QuickLinksView::new(cx))
        },
    ) {
        Ok(handle) => {
            info!("Quick links window opened");
            Some(handle)
        },
        Err(e) => {
            error!("Failed to create quick links window: {}", e);
            None
        },
    }
}

/// Preferences window dimensions
const PREFS_WIDTH: Pixels = px(580.0);
const PREFS_HEIGHT: Pixels = px(600.0);

/// Opens a new preferences window and returns its handle
fn open_preferences_window(cx: &mut AppContext) -> Option<WindowHandle<PreferencesWindow>> {
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    let x = display_bounds.origin.x + (display_bounds.size.width - PREFS_WIDTH) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * 0.2;

    let bounds = Bounds {
        origin: point(x, y),
        size: size(PREFS_WIDTH, PREFS_HEIGHT),
    };

    match cx.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Preferences".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.0), px(9.0))),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.preferences".to_string()),
            window_min_size: Some(size(px(480.0), px(360.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(|cx| PreferencesWindow::new(cx))
        },
    ) {
        Ok(handle) => {
            info!("Preferences window opened");
            Some(handle)
        },
        Err(e) => {
            error!("Failed to create preferences window: {}", e);
            None
        },
    }
}

/// Create Quicklink window dimensions
const CREATE_QUICKLINK_WIDTH: Pixels = px(520.0);
const CREATE_QUICKLINK_HEIGHT: Pixels = px(680.0);

/// Opens a new create quicklink window and returns its handle
fn open_create_quicklink_window(
    cx: &mut AppContext,
    storage: photoncast_quicklinks::QuickLinksStorage,
) -> Option<WindowHandle<CreateQuicklinkView>> {
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    let x = display_bounds.origin.x + (display_bounds.size.width - CREATE_QUICKLINK_WIDTH) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * 0.15;

    let bounds = Bounds {
        origin: point(x, y),
        size: size(CREATE_QUICKLINK_WIDTH, CREATE_QUICKLINK_HEIGHT),
    };

    match cx.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Create Quicklink".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.0), px(9.0))),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.createquicklink".to_string()),
            window_min_size: Some(size(px(420.0), px(400.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        move |cx| {
            cx.activate(true);
            let storage_clone = storage.clone();
            cx.new_view(|cx| {
                let mut view = CreateQuicklinkView::new(cx);
                view.on_event(move |event, cx| {
                    use photoncast_quicklinks::ui::CreateQuicklinkEvent;
                    match event {
                        CreateQuicklinkEvent::Created(link) => {
                            info!("Creating quicklink: {}", link.name);
                            let rt = tokio::runtime::Handle::try_current()
                                .ok()
                                .or_else(|| {
                                    tokio::runtime::Runtime::new().ok().map(|rt| {
                                        let handle = rt.handle().clone();
                                        std::mem::forget(rt);
                                        handle
                                    })
                                });
                            if let Some(handle) = rt {
                                let storage = storage_clone.clone();
                                let link = link.clone();
                                handle.spawn(async move {
                                    if let Err(e) = storage.store(&link).await {
                                        error!("Failed to store quicklink: {}", e);
                                    } else {
                                        info!("Quicklink created successfully");
                                    }
                                });
                            }
                            cx.remove_window();
                        }
                        CreateQuicklinkEvent::Updated(link) => {
                            info!("Updating quicklink: {}", link.name);
                            let rt = tokio::runtime::Handle::try_current()
                                .ok()
                                .or_else(|| {
                                    tokio::runtime::Runtime::new().ok().map(|rt| {
                                        let handle = rt.handle().clone();
                                        std::mem::forget(rt);
                                        handle
                                    })
                                });
                            if let Some(handle) = rt {
                                let storage = storage_clone.clone();
                                let link = link.clone();
                                handle.spawn(async move {
                                    if let Err(e) = storage.update(&link).await {
                                        error!("Failed to update quicklink: {}", e);
                                    } else {
                                        info!("Quicklink updated successfully");
                                    }
                                });
                            }
                            cx.remove_window();
                        }
                        CreateQuicklinkEvent::Cancelled => {
                            // Window already closed by cancel()
                        }
                    }
                });
                view
            })
        },
    ) {
        Ok(handle) => {
            info!("Create quicklink window opened");
            Some(handle)
        },
        Err(e) => {
            error!("Failed to create quicklink window: {}", e);
            None
        },
    }
}

/// Argument Input window dimensions
const ARGUMENT_INPUT_WIDTH: Pixels = px(480.0);
const ARGUMENT_INPUT_HEIGHT: Pixels = px(320.0);

/// Opens a new argument input window for a quicklink and returns its handle
fn open_argument_input_window(
    cx: &mut AppContext,
    quicklink: photoncast_quicklinks::QuickLink,
) -> Option<WindowHandle<ArgumentInputView>> {
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    let x = display_bounds.origin.x + (display_bounds.size.width - ARGUMENT_INPUT_WIDTH) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * 0.25;

    let bounds = Bounds {
        origin: point(x, y),
        size: size(ARGUMENT_INPUT_WIDTH, ARGUMENT_INPUT_HEIGHT),
    };

    let link_name = quicklink.name.clone();

    match cx.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(format!("Quick Link: {}", link_name).into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.0), px(9.0))),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.argumentinput".to_string()),
            window_min_size: Some(size(px(360.0), px(200.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        move |cx| {
            cx.activate(true);
            cx.new_view(|cx| {
                let mut view = ArgumentInputView::new(quicklink.clone(), cx);
                view.on_event(|event, cx| {
                    match event {
                        ArgumentInputEvent::Submitted { final_url, .. } => {
                            info!("Opening quicklink URL: {}", final_url);
                            if let Err(e) = photoncast_core::platform::launch::open_url(&final_url) {
                                error!("Failed to open quicklink URL: {}", e);
                            }
                            cx.remove_window();
                        }
                        ArgumentInputEvent::Cancelled => {
                            cx.remove_window();
                        }
                    }
                });
                view
            })
        },
    ) {
        Ok(handle) => {
            info!("Argument input window opened");
            Some(handle)
        }
        Err(e) => {
            error!("Failed to open argument input window: {}", e);
            None
        }
    }
}

/// Manage Quicklinks window dimensions
const MANAGE_QUICKLINKS_WIDTH: Pixels = px(620.0);
const MANAGE_QUICKLINKS_HEIGHT: Pixels = px(550.0);

/// Opens a new manage quicklinks window and returns its handle
fn open_manage_quicklinks_window(
    cx: &mut AppContext,
    storage: photoncast_quicklinks::QuickLinksStorage,
    runtime: &tokio::runtime::Runtime,
    show_library: bool,
) -> Option<WindowHandle<QuicklinksManageView>> {
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    let x = display_bounds.origin.x + (display_bounds.size.width - MANAGE_QUICKLINKS_WIDTH) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * 0.15;

    let bounds = Bounds {
        origin: point(x, y),
        size: size(MANAGE_QUICKLINKS_WIDTH, MANAGE_QUICKLINKS_HEIGHT),
    };

    // Load quicklinks
    let links = runtime.block_on(storage.load_all()).unwrap_or_default();

    match cx.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Manage Quicklinks".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.0), px(9.0))),
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.managequicklinks".to_string()),
            window_min_size: Some(size(px(480.0), px(360.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        move |cx| {
            cx.activate(true);
            cx.new_view(|cx| {
                let mut view = QuicklinksManageView::new(cx);
                view.set_storage(storage.clone(), std::sync::Arc::new(
                    tokio::runtime::Runtime::new().expect("Failed to create runtime")
                ));
                view.set_quicklinks(links.clone(), cx);
                if show_library {
                    view.toggle_library(cx);
                }
                view
            })
        },
    ) {
        Ok(handle) => {
            info!("Manage quicklinks window opened");
            Some(handle)
        },
        Err(e) => {
            error!("Failed to create manage quicklinks window: {}", e);
            None
        },
    }
}

/// Timer window dimensions
const TIMER_WIDTH: Pixels = px(360.0);
const TIMER_HEIGHT: Pixels = px(220.0);

/// Opens a new timer window and returns its handle
fn open_timer_window(
    cx: &mut AppContext,
) -> Option<WindowHandle<photoncast_timer::ui::TimerDisplay>> {
    let display = cx.displays().first().cloned();
    let display_bounds = display.map_or_else(
        || Bounds {
            origin: Point::default(),
            size: size(px(1920.0), px(1080.0)),
        },
        |d| d.bounds(),
    );

    let x = display_bounds.origin.x + (display_bounds.size.width - TIMER_WIDTH) / 2.0;
    let y = display_bounds.origin.y + display_bounds.size.height * 0.25;

    let bounds = Bounds {
        origin: point(x, y),
        size: size(TIMER_WIDTH, TIMER_HEIGHT),
    };

    match cx.open_window(
        WindowOptions {
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
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast.timer".to_string()),
            window_min_size: Some(size(px(300.0), px(180.0))),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| {
            cx.activate(true);
            cx.new_view(|cx| photoncast_timer::ui::TimerDisplay::new(cx))
        },
    ) {
        Ok(handle) => {
            info!("Timer window opened");
            Some(handle)
        },
        Err(e) => {
            error!("Failed to create timer window: {}", e);
            None
        },
    }
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
