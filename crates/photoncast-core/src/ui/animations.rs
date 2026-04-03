//! Animation system for PhotonCast.
//!
//! This module provides animation constants, helpers, and reduce motion support
//! for a smooth and accessible user experience.
//!
//! # Reduce Motion Support
//!
//! PhotonCast respects the system's "Reduce Motion" accessibility setting
//! (`NSWorkspace.accessibilityDisplayShouldReduceMotion`). When enabled:
//! - All animations complete instantly (zero duration)
//! - No spring physics or easing effects
//! - User experience remains smooth without motion
//!
//! # Animation Durations
//!
//! Standard durations used throughout the application:
//! - Window appear: 150ms ease-out scale with immediate visibility
//! - Window dismiss: 100ms ease-in
//! - Selection change: 80ms ease-in-out
//! - Hover highlight: 60ms linear

use std::time::Duration;

use crate::platform::appearance::prefers_reduced_motion;

/// Duration for window appear animation (150ms ease-out).
pub const WINDOW_APPEAR_MS: u64 = 150;

/// Duration for window dismiss animation (100ms ease-in).
pub const WINDOW_DISMISS_MS: u64 = 100;

/// Duration for selection change animation (80ms ease-in-out).
pub const SELECTION_CHANGE_MS: u64 = 80;

/// Duration for hover highlight animation (60ms linear).
pub const HOVER_TRANSITION_MS: u64 = 60;

/// Scale factor at the start of window appear animation.
pub const WINDOW_APPEAR_SCALE_START: f32 = 0.95;

/// Scale factor at the end of window appear animation.
pub const WINDOW_APPEAR_SCALE_END: f32 = 1.0;

/// Scale factor at the end of window dismiss animation.
pub const WINDOW_DISMISS_SCALE_END: f32 = 0.95;

/// Opacity at the start of window appear animation.
///
/// Keep the launcher immediately visible on the first composited frame, even
/// while scale animation continues, so app-shell visibility is not gated on
/// follow-up animation ticks during startup.
pub const WINDOW_APPEAR_OPACITY_START: f32 = 1.0;

/// Opacity at the end of window appear animation.
pub const WINDOW_APPEAR_OPACITY_END: f32 = 1.0;

/// Global reduce motion override setting.
///
/// This can be set by the user in PhotonCast settings to override
/// the system preference. `None` means follow system setting.
static REDUCE_MOTION_OVERRIDE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Override values for reduce motion setting.
const OVERRIDE_NONE: u8 = 0;
const OVERRIDE_ENABLED: u8 = 1;
const OVERRIDE_DISABLED: u8 = 2;

/// Returns the animation duration based on reduce motion preferences.
///
/// When reduce motion is enabled (either by system setting or PhotonCast override),
/// this returns `Duration::ZERO` for instant transitions.
///
/// # Arguments
///
/// * `base_ms` - The base duration in milliseconds when animations are enabled.
///
/// # Example
///
/// ```
/// use photoncast_core::ui::animations::{animation_duration, WINDOW_APPEAR_MS};
///
/// let duration = animation_duration(WINDOW_APPEAR_MS);
/// // Returns Duration::ZERO if reduce motion is enabled
/// // Otherwise returns Duration::from_millis(150)
/// ```
#[must_use]
pub fn animation_duration(base_ms: u64) -> Duration {
    if reduce_motion_enabled() {
        Duration::ZERO
    } else {
        Duration::from_millis(base_ms)
    }
}

/// Returns whether reduce motion is currently enabled.
///
/// This checks:
/// 1. PhotonCast settings override (if set)
/// 2. System accessibility preference (`reduceMotion`)
///
/// When reduce motion is enabled, all animations should be instant.
///
/// # Example
///
/// ```
/// use photoncast_core::ui::animations::reduce_motion_enabled;
///
/// if reduce_motion_enabled() {
///     // Skip animations, apply changes instantly
/// } else {
///     // Use normal animation durations
/// }
/// ```
#[must_use]
pub fn reduce_motion_enabled() -> bool {
    match REDUCE_MOTION_OVERRIDE.load(std::sync::atomic::Ordering::Relaxed) {
        OVERRIDE_ENABLED => true,
        OVERRIDE_DISABLED => false,
        _ => prefers_reduced_motion(),
    }
}

/// Sets the PhotonCast reduce motion override.
///
/// This allows users to override the system preference in PhotonCast settings.
///
/// # Arguments
///
/// * `override_value` - `Some(true)` to force reduce motion on, `Some(false)` to force off,
///   `None` to follow system setting.
///
/// # Example
///
/// ```
/// use photoncast_core::ui::animations::set_reduce_motion_override;
///
/// // Force reduce motion on
/// set_reduce_motion_override(Some(true));
///
/// // Follow system setting
/// set_reduce_motion_override(None);
/// ```
pub fn set_reduce_motion_override(override_value: Option<bool>) {
    let value = match override_value {
        Some(true) => OVERRIDE_ENABLED,
        Some(false) => OVERRIDE_DISABLED,
        None => OVERRIDE_NONE,
    };
    REDUCE_MOTION_OVERRIDE.store(value, std::sync::atomic::Ordering::Relaxed);
}

/// Returns the current reduce motion override setting.
///
/// # Returns
///
/// * `Some(true)` - Reduce motion is forced on
/// * `Some(false)` - Reduce motion is forced off
/// * `None` - Following system setting
#[must_use]
pub fn get_reduce_motion_override() -> Option<bool> {
    match REDUCE_MOTION_OVERRIDE.load(std::sync::atomic::Ordering::Relaxed) {
        OVERRIDE_ENABLED => Some(true),
        OVERRIDE_DISABLED => Some(false),
        _ => None,
    }
}

/// Pre-defined animation duration for window appear.
///
/// Uses `animation_duration()` internally to respect reduce motion.
#[must_use]
pub fn window_appear_duration() -> Duration {
    animation_duration(WINDOW_APPEAR_MS)
}

/// Pre-defined animation duration for window dismiss.
///
/// Uses `animation_duration()` internally to respect reduce motion.
#[must_use]
pub fn window_dismiss_duration() -> Duration {
    animation_duration(WINDOW_DISMISS_MS)
}

/// Pre-defined animation duration for selection changes.
///
/// Uses `animation_duration()` internally to respect reduce motion.
#[must_use]
pub fn selection_change_duration() -> Duration {
    animation_duration(SELECTION_CHANGE_MS)
}

/// Pre-defined animation duration for hover highlights.
///
/// Uses `animation_duration()` internally to respect reduce motion.
#[must_use]
pub fn hover_transition_duration() -> Duration {
    animation_duration(HOVER_TRANSITION_MS)
}

/// Easing function for ease-out (deceleration).
///
/// Starts fast, slows down at the end.
/// Formula: 1 - (1 - t)^2
#[must_use]
pub fn ease_out(t: f32) -> f32 {
    let inv = 1.0 - t;
    inv.mul_add(-inv, 1.0)
}

/// Easing function for ease-in (acceleration).
///
/// Starts slow, speeds up at the end.
/// Formula: t^2
#[must_use]
pub fn ease_in(t: f32) -> f32 {
    t * t
}

/// Easing function for ease-in-out (acceleration then deceleration).
///
/// Starts slow, speeds up in the middle, slows down at the end.
/// Formula: quadratic ease-in-out
#[must_use]
pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        let inner = (-2.0f32).mul_add(t, 2.0);
        1.0 - inner.powi(2) / 2.0
    }
}

/// Linear easing (no easing).
///
/// Returns the input directly.
#[must_use]
pub fn linear(t: f32) -> f32 {
    t
}

/// Interpolates between two values based on animation progress.
///
/// # Arguments
///
/// * `start` - Starting value
/// * `end` - Ending value
/// * `t` - Progress from 0.0 to 1.0
///
/// # Example
///
/// ```
/// use photoncast_core::ui::animations::lerp;
///
/// let value = lerp(0.95, 1.0, 0.5);
/// assert!((value - 0.975).abs() < 0.001);
/// ```
#[must_use]
pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    (end - start).mul_add(t, start)
}

/// Interpolates between two colors based on animation progress.
///
/// This performs linear interpolation in RGBA space.
///
/// # Arguments
///
/// * `start` - Starting color (r, g, b, a) with values 0.0-1.0
/// * `end` - Ending color (r, g, b, a) with values 0.0-1.0
/// * `t` - Progress from 0.0 to 1.0
#[must_use]
pub fn lerp_color(
    start: (f32, f32, f32, f32),
    end: (f32, f32, f32, f32),
    t: f32,
) -> (f32, f32, f32, f32) {
    (
        lerp(start.0, end.0, t),
        lerp(start.1, end.1, t),
        lerp(start.2, end.2, t),
        lerp(start.3, end.3, t),
    )
}

/// Animation state for window transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowAnimationState {
    /// Window is appearing (animating in).
    Appearing,
    /// Window is fully visible (animation complete).
    Visible,
    /// Window is dismissing (animating out).
    Dismissing,
    /// Window is hidden (not visible).
    #[default]
    Hidden,
}

/// Animation state for item selection/hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemAnimationState {
    /// Item is in normal state.
    #[default]
    Normal,
    /// Item is transitioning to hovered state.
    HoverIn,
    /// Item is in hovered state.
    Hovered,
    /// Item is transitioning from hovered state.
    HoverOut,
    /// Item is transitioning to selected state.
    SelectIn,
    /// Item is in selected state.
    Selected,
    /// Item is transitioning from selected state.
    SelectOut,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_duration_constants() {
        assert_eq!(WINDOW_APPEAR_MS, 150);
        assert_eq!(WINDOW_DISMISS_MS, 100);
        assert_eq!(SELECTION_CHANGE_MS, 80);
        assert_eq!(HOVER_TRANSITION_MS, 60);
    }

    #[test]
    fn test_scale_constants() {
        assert!((WINDOW_APPEAR_SCALE_START - 0.95).abs() < 0.001);
        assert!((WINDOW_APPEAR_SCALE_END - 1.0).abs() < 0.001);
        assert!((WINDOW_DISMISS_SCALE_END - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_opacity_constants() {
        assert!((WINDOW_APPEAR_OPACITY_START - 1.0).abs() < 0.001);
        assert!((WINDOW_APPEAR_OPACITY_END - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_animation_duration_with_motion() {
        // Clear any override
        set_reduce_motion_override(None);

        // Without system reduce motion, should return base duration
        // Note: This depends on system settings, so we test the override path
        set_reduce_motion_override(Some(false));

        let duration = animation_duration(150);
        assert_eq!(duration, Duration::from_millis(150));
    }

    #[test]
    fn test_animation_duration_with_reduce_motion() {
        set_reduce_motion_override(Some(true));

        let duration = animation_duration(150);
        assert_eq!(duration, Duration::ZERO);

        // Clean up
        set_reduce_motion_override(None);
    }

    #[test]
    fn test_reduce_motion_override() {
        // Test None (follow system)
        set_reduce_motion_override(None);
        assert_eq!(get_reduce_motion_override(), None);

        // Test force on
        set_reduce_motion_override(Some(true));
        assert_eq!(get_reduce_motion_override(), Some(true));
        assert!(reduce_motion_enabled());

        // Test force off
        set_reduce_motion_override(Some(false));
        assert_eq!(get_reduce_motion_override(), Some(false));
        assert!(!reduce_motion_enabled());

        // Clean up
        set_reduce_motion_override(None);
    }

    #[test]
    fn test_duration_helpers() {
        set_reduce_motion_override(Some(false));

        assert_eq!(window_appear_duration(), Duration::from_millis(150));
        assert_eq!(window_dismiss_duration(), Duration::from_millis(100));
        assert_eq!(selection_change_duration(), Duration::from_millis(80));
        assert_eq!(hover_transition_duration(), Duration::from_millis(60));

        set_reduce_motion_override(Some(true));

        assert_eq!(window_appear_duration(), Duration::ZERO);
        assert_eq!(window_dismiss_duration(), Duration::ZERO);
        assert_eq!(selection_change_duration(), Duration::ZERO);
        assert_eq!(hover_transition_duration(), Duration::ZERO);

        // Clean up
        set_reduce_motion_override(None);
    }

    #[test]
    fn test_ease_out() {
        assert!((ease_out(0.0) - 0.0).abs() < 0.001);
        assert!((ease_out(1.0) - 1.0).abs() < 0.001);
        // At 0.5, should be > 0.5 (decelerating)
        assert!(ease_out(0.5) > 0.5);
    }

    #[test]
    fn test_ease_in() {
        assert!((ease_in(0.0) - 0.0).abs() < 0.001);
        assert!((ease_in(1.0) - 1.0).abs() < 0.001);
        // At 0.5, should be < 0.5 (accelerating)
        assert!(ease_in(0.5) < 0.5);
    }

    #[test]
    fn test_ease_in_out() {
        assert!((ease_in_out(0.0) - 0.0).abs() < 0.001);
        assert!((ease_in_out(1.0) - 1.0).abs() < 0.001);
        assert!((ease_in_out(0.5) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_linear() {
        assert!((linear(0.0) - 0.0).abs() < 0.001);
        assert!((linear(0.5) - 0.5).abs() < 0.001);
        assert!((linear(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 1.0, 0.0) - 0.0).abs() < 0.001);
        assert!((lerp(0.0, 1.0, 0.5) - 0.5).abs() < 0.001);
        assert!((lerp(0.0, 1.0, 1.0) - 1.0).abs() < 0.001);
        assert!((lerp(0.95, 1.0, 0.5) - 0.975).abs() < 0.001);
    }

    #[test]
    fn test_lerp_color() {
        let black = (0.0, 0.0, 0.0, 1.0);
        let white = (1.0, 1.0, 1.0, 1.0);

        let mid = lerp_color(black, white, 0.5);
        assert!((mid.0 - 0.5).abs() < 0.001);
        assert!((mid.1 - 0.5).abs() < 0.001);
        assert!((mid.2 - 0.5).abs() < 0.001);
        assert!((mid.3 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_window_animation_state_default() {
        assert_eq!(
            WindowAnimationState::default(),
            WindowAnimationState::Hidden
        );
    }

    #[test]
    fn test_item_animation_state_default() {
        assert_eq!(ItemAnimationState::default(), ItemAnimationState::Normal);
    }
}
