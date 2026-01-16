//! PhotonCast - Lightning-fast macOS launcher built in pure Rust
//!
//! This is the main entry point for the PhotonCast application.
//! It initializes GPUI, creates the launcher window, and runs the event loop.

use gpui::*;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod launcher;
mod platform;

use launcher::LauncherWindow;

actions!(
    photoncast,
    [
        SelectNext,
        SelectPrevious,
        Activate,
        Cancel,
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
    ]
);

/// Window dimensions constants
const LAUNCHER_WIDTH: Pixels = px(680.0);
const LAUNCHER_MIN_HEIGHT: Pixels = px(72.0);
const LAUNCHER_MAX_HEIGHT: Pixels = px(500.0);
const LAUNCHER_BORDER_RADIUS: Pixels = px(12.0);

/// Position from top of screen (20%)
const LAUNCHER_TOP_OFFSET_PERCENT: f32 = 0.20;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting PhotonCast v{}", env!("CARGO_PKG_VERSION"));

    // Initialize and run GPUI application
    App::new().run(|cx: &mut AppContext| {
        // Configure frame rate target (120 FPS)
        // Note: GPUI uses VSync by default which targets display refresh rate

        // Register key bindings
        register_key_bindings(cx);

        // Create and show the launcher window
        cx.open_window(
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
        )
        .expect("Failed to create launcher window");

        info!("PhotonCast initialized successfully");
    });
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
    ]);
}
