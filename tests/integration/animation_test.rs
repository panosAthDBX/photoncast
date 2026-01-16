//! Animation system integration tests.
//!
//! Tests for the animation module including:
//! - Reduce motion detection
//! - Animation duration helper
//! - Easing functions
//! - Animation constants

use std::time::Duration;

use photoncast_core::ui::animations::{
    animation_duration, ease_in, ease_in_out, ease_out, get_reduce_motion_override,
    hover_transition_duration, lerp, lerp_color, linear, reduce_motion_enabled,
    selection_change_duration, set_reduce_motion_override, window_appear_duration,
    window_dismiss_duration, ItemAnimationState, WindowAnimationState, HOVER_TRANSITION_MS,
    SELECTION_CHANGE_MS, WINDOW_APPEAR_MS, WINDOW_APPEAR_OPACITY_END, WINDOW_APPEAR_OPACITY_START,
    WINDOW_APPEAR_SCALE_END, WINDOW_APPEAR_SCALE_START, WINDOW_DISMISS_MS, WINDOW_DISMISS_SCALE_END,
};

/// Helper to reset reduce motion override after tests.
fn with_reduce_motion_reset<F: FnOnce()>(test: F) {
    test();
    set_reduce_motion_override(None);
}

// ============================================================================
// Animation Duration Constants Tests
// ============================================================================

#[test]
fn test_window_appear_duration_constant() {
    assert_eq!(WINDOW_APPEAR_MS, 150, "Window appear should be 150ms");
}

#[test]
fn test_window_dismiss_duration_constant() {
    assert_eq!(WINDOW_DISMISS_MS, 100, "Window dismiss should be 100ms");
}

#[test]
fn test_selection_change_duration_constant() {
    assert_eq!(SELECTION_CHANGE_MS, 80, "Selection change should be 80ms");
}

#[test]
fn test_hover_transition_duration_constant() {
    assert_eq!(HOVER_TRANSITION_MS, 60, "Hover transition should be 60ms");
}

// ============================================================================
// Animation Scale Constants Tests
// ============================================================================

#[test]
fn test_window_appear_scale_start() {
    assert!(
        (WINDOW_APPEAR_SCALE_START - 0.95).abs() < 0.001,
        "Window appear scale should start at 0.95"
    );
}

#[test]
fn test_window_appear_scale_end() {
    assert!(
        (WINDOW_APPEAR_SCALE_END - 1.0).abs() < 0.001,
        "Window appear scale should end at 1.0"
    );
}

#[test]
fn test_window_dismiss_scale_end() {
    assert!(
        (WINDOW_DISMISS_SCALE_END - 0.95).abs() < 0.001,
        "Window dismiss scale should end at 0.95"
    );
}

// ============================================================================
// Reduce Motion Override Tests
// ============================================================================

#[test]
fn test_reduce_motion_override_none() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(None);
        assert_eq!(
            get_reduce_motion_override(),
            None,
            "Override should be None when not set"
        );
    });
}

#[test]
fn test_reduce_motion_override_enabled() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(true));
        assert_eq!(
            get_reduce_motion_override(),
            Some(true),
            "Override should be Some(true)"
        );
        assert!(
            reduce_motion_enabled(),
            "Reduce motion should be enabled with override"
        );
    });
}

#[test]
fn test_reduce_motion_override_disabled() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(false));
        assert_eq!(
            get_reduce_motion_override(),
            Some(false),
            "Override should be Some(false)"
        );
        assert!(
            !reduce_motion_enabled(),
            "Reduce motion should be disabled with override"
        );
    });
}

// ============================================================================
// Animation Duration Helper Tests
// ============================================================================

#[test]
fn test_animation_duration_with_motion_enabled() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(false));

        assert_eq!(
            animation_duration(150),
            Duration::from_millis(150),
            "Duration should be 150ms when motion is enabled"
        );

        assert_eq!(
            animation_duration(100),
            Duration::from_millis(100),
            "Duration should be 100ms when motion is enabled"
        );

        assert_eq!(
            animation_duration(0),
            Duration::ZERO,
            "Zero duration should always be zero"
        );
    });
}

#[test]
fn test_animation_duration_with_reduce_motion() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(true));

        assert_eq!(
            animation_duration(150),
            Duration::ZERO,
            "Duration should be zero with reduce motion"
        );

        assert_eq!(
            animation_duration(100),
            Duration::ZERO,
            "Duration should be zero with reduce motion"
        );
    });
}

#[test]
fn test_window_appear_duration_helper() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(false));
        assert_eq!(
            window_appear_duration(),
            Duration::from_millis(WINDOW_APPEAR_MS)
        );

        set_reduce_motion_override(Some(true));
        assert_eq!(window_appear_duration(), Duration::ZERO);
    });
}

#[test]
fn test_window_dismiss_duration_helper() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(false));
        assert_eq!(
            window_dismiss_duration(),
            Duration::from_millis(WINDOW_DISMISS_MS)
        );

        set_reduce_motion_override(Some(true));
        assert_eq!(window_dismiss_duration(), Duration::ZERO);
    });
}

#[test]
fn test_selection_change_duration_helper() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(false));
        assert_eq!(
            selection_change_duration(),
            Duration::from_millis(SELECTION_CHANGE_MS)
        );

        set_reduce_motion_override(Some(true));
        assert_eq!(selection_change_duration(), Duration::ZERO);
    });
}

#[test]
fn test_hover_transition_duration_helper() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(false));
        assert_eq!(
            hover_transition_duration(),
            Duration::from_millis(HOVER_TRANSITION_MS)
        );

        set_reduce_motion_override(Some(true));
        assert_eq!(hover_transition_duration(), Duration::ZERO);
    });
}

// ============================================================================
// Easing Function Tests
// ============================================================================

#[test]
fn test_ease_out_bounds() {
    assert!((ease_out(0.0) - 0.0).abs() < 0.001, "ease_out(0) should be 0");
    assert!((ease_out(1.0) - 1.0).abs() < 0.001, "ease_out(1) should be 1");
}

#[test]
fn test_ease_out_characteristic() {
    // Ease-out should be > t at midpoint (decelerating curve)
    let mid = ease_out(0.5);
    assert!(mid > 0.5, "ease_out(0.5) should be > 0.5, got {}", mid);
}

#[test]
fn test_ease_in_bounds() {
    assert!((ease_in(0.0) - 0.0).abs() < 0.001, "ease_in(0) should be 0");
    assert!((ease_in(1.0) - 1.0).abs() < 0.001, "ease_in(1) should be 1");
}

#[test]
fn test_ease_in_characteristic() {
    // Ease-in should be < t at midpoint (accelerating curve)
    let mid = ease_in(0.5);
    assert!(mid < 0.5, "ease_in(0.5) should be < 0.5, got {}", mid);
}

#[test]
fn test_ease_in_out_bounds() {
    assert!(
        (ease_in_out(0.0) - 0.0).abs() < 0.001,
        "ease_in_out(0) should be 0"
    );
    assert!(
        (ease_in_out(1.0) - 1.0).abs() < 0.001,
        "ease_in_out(1) should be 1"
    );
}

#[test]
fn test_ease_in_out_midpoint() {
    // Ease-in-out should be exactly 0.5 at midpoint (symmetric)
    let mid = ease_in_out(0.5);
    assert!(
        (mid - 0.5).abs() < 0.001,
        "ease_in_out(0.5) should be ~0.5, got {}",
        mid
    );
}

#[test]
fn test_linear_easing() {
    assert!((linear(0.0) - 0.0).abs() < 0.001, "linear(0) should be 0");
    assert!((linear(0.25) - 0.25).abs() < 0.001, "linear(0.25) should be 0.25");
    assert!((linear(0.5) - 0.5).abs() < 0.001, "linear(0.5) should be 0.5");
    assert!((linear(0.75) - 0.75).abs() < 0.001, "linear(0.75) should be 0.75");
    assert!((linear(1.0) - 1.0).abs() < 0.001, "linear(1) should be 1");
}

// ============================================================================
// Lerp Function Tests
// ============================================================================

#[test]
fn test_lerp_boundaries() {
    assert!((lerp(0.0, 1.0, 0.0) - 0.0).abs() < 0.001, "lerp at t=0 should return start");
    assert!((lerp(0.0, 1.0, 1.0) - 1.0).abs() < 0.001, "lerp at t=1 should return end");
}

#[test]
fn test_lerp_midpoint() {
    assert!(
        (lerp(0.0, 1.0, 0.5) - 0.5).abs() < 0.001,
        "lerp at t=0.5 should return midpoint"
    );
    assert!(
        (lerp(0.0, 10.0, 0.5) - 5.0).abs() < 0.001,
        "lerp at t=0.5 should return midpoint"
    );
}

#[test]
fn test_lerp_with_animation_values() {
    // Test with actual window animation values
    let start_scale = WINDOW_APPEAR_SCALE_START;
    let end_scale = WINDOW_APPEAR_SCALE_END;

    let result = lerp(start_scale, end_scale, 0.5);
    let expected = 0.975; // (0.95 + 1.0) / 2
    assert!(
        (result - expected).abs() < 0.001,
        "lerp for scale midpoint should be {}, got {}",
        expected,
        result
    );
}

#[test]
fn test_lerp_color() {
    let black = (0.0, 0.0, 0.0, 1.0);
    let white = (1.0, 1.0, 1.0, 1.0);

    let mid = lerp_color(black, white, 0.5);
    assert!((mid.0 - 0.5).abs() < 0.001, "R should be 0.5");
    assert!((mid.1 - 0.5).abs() < 0.001, "G should be 0.5");
    assert!((mid.2 - 0.5).abs() < 0.001, "B should be 0.5");
    assert!((mid.3 - 1.0).abs() < 0.001, "A should be 1.0 (unchanged)");

    let quarter = lerp_color(black, white, 0.25);
    assert!((quarter.0 - 0.25).abs() < 0.001, "R should be 0.25");
}

// ============================================================================
// Animation State Tests
// ============================================================================

#[test]
fn test_window_animation_state_default() {
    let state = WindowAnimationState::default();
    assert_eq!(state, WindowAnimationState::Hidden, "Default should be Hidden");
}

#[test]
fn test_window_animation_state_values() {
    // Ensure all states are distinct
    assert_ne!(WindowAnimationState::Appearing, WindowAnimationState::Visible);
    assert_ne!(WindowAnimationState::Visible, WindowAnimationState::Dismissing);
    assert_ne!(WindowAnimationState::Dismissing, WindowAnimationState::Hidden);
}

#[test]
fn test_item_animation_state_default() {
    let state = ItemAnimationState::default();
    assert_eq!(state, ItemAnimationState::Normal, "Default should be Normal");
}

#[test]
fn test_item_animation_state_values() {
    // Ensure hover and select states are distinct
    assert_ne!(ItemAnimationState::Hovered, ItemAnimationState::Selected);
    assert_ne!(ItemAnimationState::HoverIn, ItemAnimationState::SelectIn);
}

// ============================================================================
// Integration Tests: Combined Behavior
// ============================================================================

#[test]
fn test_animation_system_respects_reduce_motion() {
    with_reduce_motion_reset(|| {
        // With reduce motion enabled
        set_reduce_motion_override(Some(true));

        // All durations should be zero
        assert_eq!(window_appear_duration(), Duration::ZERO);
        assert_eq!(window_dismiss_duration(), Duration::ZERO);
        assert_eq!(selection_change_duration(), Duration::ZERO);
        assert_eq!(hover_transition_duration(), Duration::ZERO);

        // With reduce motion disabled
        set_reduce_motion_override(Some(false));

        // All durations should be their base values
        assert!(window_appear_duration() > Duration::ZERO);
        assert!(window_dismiss_duration() > Duration::ZERO);
        assert!(selection_change_duration() > Duration::ZERO);
        assert!(hover_transition_duration() > Duration::ZERO);
    });
}

#[test]
fn test_animation_durations_ordering() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(false));

        // Window appear should be longest
        let appear = window_appear_duration();
        let dismiss = window_dismiss_duration();
        let selection = selection_change_duration();
        let hover = hover_transition_duration();

        assert!(appear > dismiss, "Appear should be longer than dismiss");
        assert!(dismiss > selection, "Dismiss should be longer than selection");
        assert!(selection > hover, "Selection should be longer than hover");
    });
}

#[test]
fn test_complete_animation_cycle() {
    with_reduce_motion_reset(|| {
        set_reduce_motion_override(Some(false));

        // Simulate a complete window animation cycle
        let mut state = WindowAnimationState::Hidden;

        // Appear
        state = WindowAnimationState::Appearing;
        let appear_duration = window_appear_duration();
        assert!(appear_duration.as_millis() > 0);

        // Visible
        state = WindowAnimationState::Visible;

        // Selection changes happen while visible
        let selection_duration = selection_change_duration();
        assert!(selection_duration.as_millis() > 0);

        // Dismiss
        state = WindowAnimationState::Dismissing;
        let dismiss_duration = window_dismiss_duration();
        assert!(dismiss_duration.as_millis() > 0);

        // Hidden
        state = WindowAnimationState::Hidden;
        assert_eq!(state, WindowAnimationState::Hidden);
    });
}
