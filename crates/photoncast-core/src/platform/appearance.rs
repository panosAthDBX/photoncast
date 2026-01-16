//! System appearance/theme detection.
//!
//! This module provides functions to detect macOS system appearance settings,
//! including dark/light mode and reduced motion preferences.
//!
//! # Implementation Notes
//!
//! The current implementation uses shell commands to read macOS preferences,
//! which works without requiring additional Objective-C bridge dependencies.
//! When GPUI is integrated, this will use `cx.observe_system_appearance()` for
//! live updates.
//!
//! # Example
//!
//! ```ignore
//! use photoncast_core::platform::appearance::{detect_system_appearance, is_dark_mode};
//!
//! let flavor = detect_system_appearance();
//! let dark = is_dark_mode();
//! ```

use std::process::Command;

#[cfg(feature = "ui")]
use crate::theme::CatppuccinFlavor;

/// Detects the current system appearance (light/dark).
///
/// Uses macOS `defaults` command to read the `AppleInterfaceStyle` preference.
/// When the value is "Dark", returns `CatppuccinFlavor::Mocha`.
/// Otherwise, returns `CatppuccinFlavor::Latte`.
///
/// # Returns
///
/// The appropriate Catppuccin flavor based on system settings:
/// - `Mocha` for dark mode (high contrast dark theme)
/// - `Latte` for light mode
///
/// # Example
///
/// ```
/// use photoncast_core::platform::appearance::detect_system_appearance;
/// use photoncast_core::theme::CatppuccinFlavor;
///
/// let flavor = detect_system_appearance();
/// // Returns Mocha in dark mode, Latte in light mode
/// ```
#[cfg(feature = "ui")]
#[must_use]
pub fn detect_system_appearance() -> CatppuccinFlavor {
    if is_dark_mode() {
        CatppuccinFlavor::Mocha
    } else {
        CatppuccinFlavor::Latte
    }
}

/// Returns true if the system is in dark mode.
///
/// Reads the `AppleInterfaceStyle` key from the global domain using `defaults read`.
/// If the key exists and equals "Dark", returns `true`. Otherwise, returns `false`
/// (macOS defaults to light mode when the key is absent).
///
/// # Example
///
/// ```
/// use photoncast_core::platform::appearance::is_dark_mode;
///
/// if is_dark_mode() {
///     println!("System is in dark mode");
/// }
/// ```
#[must_use]
pub fn is_dark_mode() -> bool {
    // Read AppleInterfaceStyle from macOS defaults
    // This key is only present when dark mode is enabled
    let output = Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let style = String::from_utf8_lossy(&result.stdout);
                style.trim().eq_ignore_ascii_case("dark")
            } else {
                // Key doesn't exist = light mode (macOS default)
                false
            }
        },
        Err(_) => {
            // Fallback to dark mode if we can't read system preferences
            // This is safer for a dark-themed application
            true
        },
    }
}

/// Returns true if the user prefers reduced motion.
///
/// Reads the `reduceMotion` preference from the accessibility domain.
/// Used to disable animations for users who prefer reduced motion.
///
/// # Example
///
/// ```
/// use photoncast_core::platform::appearance::prefers_reduced_motion;
///
/// if prefers_reduced_motion() {
///     // Skip animations
/// }
/// ```
#[must_use]
pub fn prefers_reduced_motion() -> bool {
    // Read accessibility preference for reduced motion
    let output = Command::new("defaults")
        .args(["read", "com.apple.universalaccess", "reduceMotion"])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let value = String::from_utf8_lossy(&result.stdout);
                value.trim() == "1"
            } else {
                false
            }
        },
        Err(_) => false,
    }
}

/// Observer handle for system appearance changes.
///
/// When GPUI is integrated, this will wrap the GPUI appearance observer.
/// For now, it's a placeholder that can be used to implement polling-based
/// appearance detection.
#[derive(Debug)]
pub struct AppearanceObserver {
    /// The callback ID (for unregistering).
    id: u64,
    /// Whether auto-sync is enabled.
    pub auto_sync: bool,
}

impl AppearanceObserver {
    /// Creates a new appearance observer.
    ///
    /// # Note
    ///
    /// The actual observation implementation requires GPUI integration.
    /// Currently this is a placeholder that tracks the observer state.
    #[must_use]
    pub fn new(auto_sync: bool) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            auto_sync,
        }
    }

    /// Returns the observer ID.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
}

// Note: When GPUI is added, implement the actual observer:
//
// /// Starts observing system appearance changes.
// ///
// /// # Arguments
// ///
// /// * `cx` - The GPUI application context
// /// * `callback` - Called when system appearance changes
// ///
// /// # Example
// ///
// /// ```ignore
// /// observe_appearance_changes(cx, |flavor, cx| {
// ///     let theme = cx.global::<PhotonTheme>();
// ///     if theme.auto_sync {
// ///         cx.set_global(PhotonTheme::new(flavor, theme.accent));
// ///         cx.refresh();
// ///     }
// /// });
// /// ```
// pub fn observe_appearance_changes<F>(cx: &mut gpui::App, callback: F)
// where
//     F: Fn(CatppuccinFlavor, &mut gpui::App) + 'static,
// {
//     cx.observe_system_appearance(move |cx| {
//         let flavor = detect_system_appearance();
//         callback(flavor, cx);
//     });
// }
