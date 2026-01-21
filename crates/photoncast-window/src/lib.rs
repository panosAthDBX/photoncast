//! PhotonCast Window Management Library
//!
//! This crate provides window management capabilities for PhotonCast, including:
//!
//! - Window layout presets (halves, quarters, thirds)
//! - Layout cycling (50% → 33% → 66%)
//! - Multi-monitor support
//! - Smooth animations
//! - macOS Accessibility API integration
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         WindowManager                   │
//! │  (High-level window operations)         │
//! └────────────────┬────────────────────────┘
//!                  │
//!     ┌────────────┼────────────┐
//!     │            │            │
//!     ▼            ▼            ▼
//! ┌────────┐  ┌────────┐  ┌────────────┐
//! │Layout  │  │Display │  │Accessibility│
//! │Calc    │  │Manager │  │Manager      │
//! └────────┘  └────────┘  └────────────┘
//!     │            │            │
//!     └────────────┴────────────┘
//!                  │
//!                  ▼
//!         ┌────────────────┐
//!         │ Cycling Manager│
//!         └────────────────┘
//! ```

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]

pub mod animation;
pub mod config;
pub mod cycling;
pub mod error;
pub mod layout;
pub mod overlay;

#[cfg(target_os = "macos")]
pub mod accessibility;
#[cfg(target_os = "macos")]
pub mod display;

pub use animation::{AnimationConfig, EasingFunction, WindowAnimation};
pub use config::WindowConfig;
pub use cycling::CyclingManager;
pub use error::{Result, WindowError};
pub use layout::{CycleState, LayoutCalculator, WindowLayout};

#[cfg(target_os = "macos")]
pub use accessibility::{
    AccessibilityManager, CGWindowInfo, WindowInfo,
    get_bundle_id_for_pid, get_frontmost_window_via_cgwindowlist,
};
#[cfg(target_os = "macos")]
pub use display::{DisplayDirection, DisplayInfo, DisplayManager};

pub mod commands;

/// The main window manager.
///
/// Coordinates all window management operations including layout application,
/// cycling, multi-monitor support, and animations.
#[derive(Debug)]
pub struct WindowManager {
    /// Configuration.
    config: WindowConfig,
    /// Layout calculator.
    layout_calculator: LayoutCalculator,
    /// Cycling state manager.
    cycling_manager: CyclingManager,

    #[cfg(target_os = "macos")]
    /// Accessibility manager for window manipulation.
    accessibility_manager: AccessibilityManager,

    #[cfg(target_os = "macos")]
    /// Display manager for multi-monitor support.
    display_manager: DisplayManager,
}

impl WindowManager {
    /// Creates a new window manager with the given configuration.
    #[must_use]
    pub fn new(config: WindowConfig) -> Self {
        let layout_calculator = LayoutCalculator::with_config(
            config.window_gap,
            config.respect_menu_bar,
            config.respect_dock,
            config.almost_maximize_margin,
        );
        Self {
            config,
            layout_calculator,
            cycling_manager: CyclingManager::new(),

            #[cfg(target_os = "macos")]
            accessibility_manager: AccessibilityManager::new(),

            #[cfg(target_os = "macos")]
            display_manager: DisplayManager::new(),
        }
    }

    /// Gets the current configuration.
    #[must_use]
    pub const fn config(&self) -> &WindowConfig {
        &self.config
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: WindowConfig) {
        // Update layout calculator with new config values
        self.layout_calculator.update_config(
            config.window_gap,
            config.respect_menu_bar,
            config.respect_dock,
            config.almost_maximize_margin,
        );
        self.config = config;
    }

    /// Checks if accessibility permissions are granted.
    #[cfg(target_os = "macos")]
    pub fn has_accessibility_permission(&mut self) -> bool {
        self.accessibility_manager.check_permission()
    }

    /// Requests accessibility permissions from the user.
    #[cfg(target_os = "macos")]
    pub fn request_accessibility_permission(&mut self) -> Result<()> {
        self.accessibility_manager.request_permission()
    }

    /// Gets the bundle ID of the frontmost application.
    #[cfg(target_os = "macos")]
    pub fn get_frontmost_bundle_id(&self) -> Result<String> {
        self.accessibility_manager.get_frontmost_app()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_frontmost_bundle_id(&self) -> Result<String> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Activates a running application by bundle ID.
    #[cfg(target_os = "macos")]
    pub fn activate_app(&self, bundle_id: &str) -> Result<()> {
        self.accessibility_manager.activate_app(bundle_id)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn activate_app(&self, _bundle_id: &str) -> Result<()> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Finds and activates the first visible app that isn't the given bundle ID.
    #[cfg(target_os = "macos")]
    pub fn activate_any_app_except(&self, except_bundle_id: &str) -> Result<String> {
        self.accessibility_manager.activate_any_app_except(except_bundle_id)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn activate_any_app_except(&self, _except_bundle_id: &str) -> Result<String> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Focuses (raises) a specific window by its title.
    #[cfg(target_os = "macos")]
    pub fn focus_window_by_title(&mut self, title: &str) -> Result<()> {
        self.accessibility_manager.focus_window_by_title(title)?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn focus_window_by_title(&mut self, _title: &str) -> Result<()> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Focuses the first window that doesn't look like a launcher terminal.
    #[cfg(target_os = "macos")]
    pub fn focus_first_non_launcher_window(&mut self) -> Result<()> {
        self.accessibility_manager.focus_first_non_launcher_window()?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn focus_first_non_launcher_window(&mut self) -> Result<()> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Gets info about the frontmost window.
    #[cfg(target_os = "macos")]
    pub fn get_frontmost_window_info(&mut self) -> Result<crate::accessibility::WindowInfo> {
        self.accessibility_manager.get_frontmost_window()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_frontmost_window_info(&mut self) -> Result<crate::accessibility::WindowInfo> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Applies a layout to the frontmost window.
    #[cfg(target_os = "macos")]
    pub fn apply_layout(&mut self, layout: WindowLayout) -> Result<()> {
        if !self.config.enabled {
            return Err(WindowError::Message {
                message: "Window management is disabled".to_string(),
            });
        }

        // Get frontmost window
        let window = self.accessibility_manager.get_frontmost_window()?;
        tracing::info!(
            "Applying layout to window: '{}' from app '{}' at ({}, {}) size {}x{}",
            window.title,
            window.bundle_id,
            window.frame.origin.x,
            window.frame.origin.y,
            window.frame.size.width,
            window.frame.size.height
        );

        // Handle ToggleFullscreen specially
        if layout == WindowLayout::ToggleFullscreen {
            return self.accessibility_manager.toggle_fullscreen(&window);
        }

        // If window is in fullscreen mode, exit fullscreen first
        // (otherwise resize operations will fail with kAXErrorCannotComplete)
        if self.accessibility_manager.is_fullscreen(&window).unwrap_or(false) {
            tracing::debug!("Window is in fullscreen mode, exiting fullscreen first");
            self.accessibility_manager.toggle_fullscreen(&window)?;
            // Give macOS time to complete the fullscreen exit animation
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // Get current display
        let current_frame = self.accessibility_manager.get_window_frame(&window)?;
        let display = self
            .display_manager
            .display_containing_frame(&current_frame)
            .ok_or_else(|| WindowError::Message {
                message: "No display found for window".to_string(),
            })?;

        // Save frame for restore
        if layout != WindowLayout::Restore {
            self.accessibility_manager.save_frame(&window)?;
        }

        // Calculate target frame
        let target_frame = if layout == WindowLayout::Restore {
            // Restore saved frame
            self.accessibility_manager.restore_frame(&window)?
        } else if layout == WindowLayout::MakeSmaller {
            // Shrink window by 10% from center
            self.layout_calculator
                .resize_frame(current_frame, display.frame, 0.1, false)
        } else if layout == WindowLayout::MakeLarger {
            // Grow window by 10% from center
            self.layout_calculator
                .resize_frame(current_frame, display.frame, 0.1, true)
        } else {
            // Get cycle state
            let cycle_state = if self.config.cycling_enabled {
                self.cycling_manager
                    .get_cycle_state(window.element_ref, layout)
            } else {
                CycleState::Initial
            };

            // Calculate frame for layout
            self.layout_calculator
                .calculate_frame(layout, display.frame, cycle_state)
        };

        // Show visual feedback overlay if enabled
        if self.config.show_visual_feedback {
            overlay::show_overlay(target_frame, self.config.visual_feedback_duration_ms);
        }

        // Apply the frame (with or without animation)
        if self.config.animation_enabled && !animation::is_reduce_motion_enabled() {
            // Animated resize
            let animation_config = AnimationConfig {
                duration: std::time::Duration::from_millis(u64::from(
                    self.config.animation_duration_ms,
                )),
                easing: EasingFunction::EaseOut,
                respect_reduce_motion: true,
            };

            let animation = WindowAnimation::new(current_frame, target_frame, animation_config);

            // Animate (simplified - in reality would need a timer/frame callback)
            // For now, just set the final frame
            // TODO: Implement proper animation loop
            self.accessibility_manager
                .set_window_frame(&window, animation.to)?;
        } else {
            // Immediate resize
            self.accessibility_manager
                .set_window_frame(&window, target_frame)?;
        }

        Ok(())
    }

    /// Moves the frontmost window to another display.
    #[cfg(target_os = "macos")]
    pub fn move_to_display(&mut self, direction: DisplayDirection) -> Result<()> {
        if !self.config.enabled {
            return Err(WindowError::Message {
                message: "Window management is disabled".to_string(),
            });
        }

        // Get frontmost window
        let window = self.accessibility_manager.get_frontmost_window()?;

        // If window is in fullscreen mode, exit fullscreen first
        if self.accessibility_manager.is_fullscreen(&window).unwrap_or(false) {
            tracing::debug!("Window is in fullscreen mode, exiting fullscreen first");
            self.accessibility_manager.toggle_fullscreen(&window)?;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // Get current display
        let current_frame = self.accessibility_manager.get_window_frame(&window)?;
        let current_display = self
            .display_manager
            .display_containing_frame(&current_frame)
            .ok_or_else(|| WindowError::Message {
                message: "No display found for window".to_string(),
            })?;

        // Get target display
        let target_display = match direction {
            DisplayDirection::Next => self.display_manager.next_display(current_display),
            DisplayDirection::Previous => self.display_manager.previous_display(current_display),
            DisplayDirection::Index(index) => self.display_manager.display_at_index(index),
        }
        .ok_or_else(|| WindowError::Message {
            message: "Target display not found".to_string(),
        })?;

        // Translate frame to target display
        let target_frame =
            self.display_manager
                .translate_frame(&current_frame, current_display, target_display);

        // Apply the frame
        self.accessibility_manager
            .set_window_frame(&window, target_frame)?;

        Ok(())
    }

    /// Refreshes the display list (call when displays are connected/disconnected).
    #[cfg(target_os = "macos")]
    pub fn refresh_displays(&mut self) {
        self.display_manager.refresh_displays();
    }

    /// Gets the list of connected displays.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn displays(&self) -> &[DisplayInfo] {
        self.display_manager.displays()
    }

    /// Resets cycling state for all windows.
    pub fn reset_cycling(&mut self) {
        self.cycling_manager.clear();
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new(WindowConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_manager_creation() {
        let manager = WindowManager::new(WindowConfig::default());
        assert!(manager.config().enabled);
        assert!(manager.config().animation_enabled);
        assert!(manager.config().cycling_enabled);
    }

    #[test]
    fn test_config_update() {
        let mut manager = WindowManager::new(WindowConfig::default());

        let new_config = WindowConfig {
            enabled: false,
            animation_enabled: false,
            animation_duration_ms: 100,
            cycling_enabled: false,
            window_gap: 10,
            respect_menu_bar: false,
            respect_dock: false,
            cycle_timeout_ms: 1000,
            almost_maximize_margin: 30,
            show_visual_feedback: false,
            visual_feedback_duration_ms: 100,
        };

        manager.set_config(new_config);
        assert!(!manager.config().enabled);
        assert!(!manager.config().animation_enabled);
        assert_eq!(manager.config().animation_duration_ms, 100);
        assert_eq!(manager.config().window_gap, 10);
        assert!(!manager.config().respect_menu_bar);
    }
}
