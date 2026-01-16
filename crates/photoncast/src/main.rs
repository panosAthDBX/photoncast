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

use std::sync::mpsc;
use std::time::Duration;

use gpui::*;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod launcher;
mod platform;

use launcher::LauncherWindow;
use photoncast_core::platform::accessibility::{
    check_accessibility_permission, request_accessibility_permission,
};
use photoncast_core::platform::hotkey::is_spotlight_enabled;
use photoncast_core::platform::LoginItemManager;
use platform::{create_menu_bar_item, MenuBarActionKind};

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
    ]
);

/// Window dimensions constants
const LAUNCHER_WIDTH: Pixels = px(680.0);
const LAUNCHER_MIN_HEIGHT: Pixels = px(72.0);
const LAUNCHER_MAX_HEIGHT: Pixels = px(500.0);
const LAUNCHER_BORDER_RADIUS: Pixels = px(12.0);

/// Position from top of screen (20%)
const LAUNCHER_TOP_OFFSET_PERCENT: f32 = 0.20;

/// Event message sent from background threads (hotkey, menu bar)
enum AppEvent {
    ToggleLauncher,
    OpenPreferences,
    QuitApp,
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting PhotonCast v{}", env!("CARGO_PKG_VERSION"));

    // Check for hotkey conflicts with Spotlight (non-blocking, informational only)
    check_spotlight_conflict();

    // Check and log login item status
    check_login_item_status();

    // Create channel for app events (hotkey, menu bar)
    let (event_tx, event_rx) = mpsc::channel::<AppEvent>();

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
    } else {
        // Request permission without blocking - user will need to restart app after granting
        warn!("Accessibility permission not granted. Requesting...");
        request_accessibility_permission();
        warn!(
            "Please grant accessibility access and restart PhotonCast for global hotkey to work."
        );
    }

    // Initialize and run GPUI application
    App::new().run(|cx: &mut AppContext| {
        // Register key bindings
        register_key_bindings(cx);

        // Create menu bar status item with channel for events
        let menu_tx = event_tx;
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
        let window_handle = open_launcher_window(cx);

        // Spawn a task to listen for app events (hotkey, menu bar)
        cx.spawn(|cx| async move {
            // Track the current window handle (updated when window is recreated)
            let mut current_handle: Option<WindowHandle<LauncherWindow>> = window_handle;

            loop {
                // Poll for events (non-blocking with small sleep)
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;

                // Check for app events
                match event_rx.try_recv() {
                    Ok(AppEvent::ToggleLauncher) => {
                        info!("Toggle launcher requested");
                        // Try to activate existing window or create new one
                        let _ = cx.update(|cx| {
                            // Try to update existing window
                            let window_exists = current_handle.as_ref().is_some_and(|h| {
                                h.update(cx, |_view, cx| {
                                    cx.activate_window();
                                    cx.focus_self();
                                })
                                .is_ok()
                            });

                            if !window_exists {
                                // Window was closed, open a new one
                                current_handle = open_launcher_window(cx);
                            }
                        });
                    },
                    Ok(AppEvent::OpenPreferences) => {
                        info!("Open preferences requested");
                        // TODO: Open preferences window
                    },
                    Ok(AppEvent::QuitApp) => {
                        info!("Quit requested from menu bar");
                        let _ = cx.update(|cx| {
                            cx.quit();
                        });
                        break;
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

/// Opens a new launcher window and returns its handle
fn open_launcher_window(cx: &mut AppContext) -> Option<WindowHandle<LauncherWindow>> {
    match cx.open_window(
        WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(calculate_window_bounds(cx))),
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: false,
            display_id: cx.displays().first().map(|d| d.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("app.photoncast".to_string()),
            window_min_size: Some(size(LAUNCHER_WIDTH, LAUNCHER_MIN_HEIGHT)),
            window_decorations: Some(WindowDecorations::Client),
        },
        |cx| cx.new_view(LauncherWindow::new),
    ) {
        Ok(handle) => Some(handle),
        Err(e) => {
            error!("Failed to create launcher window: {}", e);
            None
        }
    }
}

/// Calculate initial window bounds centered at top of screen
fn calculate_window_bounds(cx: &AppContext) -> Bounds<Pixels> {
    // Get the primary display bounds
    let display = cx.displays().first().cloned();
    let display_bounds = display.map(|d| d.bounds()).unwrap_or_else(|| Bounds {
        origin: Point::default(),
        size: size(px(1920.0), px(1080.0)),
    });

    // Calculate centered-top position
    let window_width = LAUNCHER_WIDTH;
    let window_height = LAUNCHER_MAX_HEIGHT;

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
