//! Window animation support.

use core_graphics::display::CGRect;
use std::time::Duration;

/// Easing function for animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingFunction {
    /// Linear easing (no acceleration).
    Linear,
    /// Ease out (deceleration).
    EaseOut,
    /// Ease in-out (acceleration then deceleration).
    EaseInOut,
}

impl EasingFunction {
    /// Applies the easing function to a progress value (0.0 to 1.0).
    #[must_use]
    pub fn apply(&self, progress: f64) -> f64 {
        match self {
            Self::Linear => progress,
            Self::EaseOut => {
                // Quadratic ease out
                let remaining = 1.0 - progress;
                remaining.mul_add(-remaining, 1.0)
            },
            Self::EaseInOut => {
                // Cubic ease in-out
                if progress < 0.5 {
                    4.0 * progress.powi(3)
                } else {
                    let base = (-2.0_f64).mul_add(progress, 2.0);
                    1.0 - base.powi(3) / 2.0
                }
            },
        }
    }
}

/// Animation configuration.
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    /// Animation duration.
    pub duration: Duration,
    /// Easing function.
    pub easing: EasingFunction,
    /// Whether to respect macOS "Reduce Motion" setting.
    pub respect_reduce_motion: bool,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(200),
            easing: EasingFunction::EaseOut,
            respect_reduce_motion: true,
        }
    }
}

/// Interpolates between two CGRect values.
#[must_use]
pub fn interpolate_rect(from: &CGRect, to: &CGRect, progress: f64) -> CGRect {
    let x = (to.origin.x - from.origin.x).mul_add(progress, from.origin.x);
    let y = (to.origin.y - from.origin.y).mul_add(progress, from.origin.y);
    let width = (to.size.width - from.size.width).mul_add(progress, from.size.width);
    let height = (to.size.height - from.size.height).mul_add(progress, from.size.height);

    CGRect::new(
        &core_graphics::geometry::CGPoint { x, y },
        &core_graphics::geometry::CGSize { width, height },
    )
}

/// Checks if the system has "Reduce Motion" enabled.
///
/// This respects the macOS accessibility setting for users who prefer
/// minimal or no animations. Queries `NSWorkspace.accessibilityDisplayShouldReduceMotion`
/// on macOS.
#[must_use]
pub fn is_reduce_motion_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;

        #[allow(deprecated)]
        unsafe {
            let workspace = NSWorkspace::sharedWorkspace();
            objc2::msg_send![&workspace, accessibilityDisplayShouldReduceMotion]
        }
    }

    #[cfg(not(target_os = "macos"))]
    false
}

/// Animation state for a window resize operation.
#[derive(Debug)]
pub struct WindowAnimation {
    /// Starting frame.
    pub from: CGRect,
    /// Target frame.
    pub to: CGRect,
    /// Animation configuration.
    pub config: AnimationConfig,
    /// Start time (in milliseconds since epoch).
    pub start_time: u128,
}

impl WindowAnimation {
    /// Creates a new window animation.
    ///
    /// # Panics
    /// Panics if system time is before UNIX_EPOCH (should never happen in practice).
    #[must_use]
    pub fn new(from: CGRect, to: CGRect, config: AnimationConfig) -> Self {
        Self {
            from,
            to,
            config,
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before UNIX_EPOCH")
                .as_millis(),
        }
    }

    /// Gets the interpolated frame for the current time.
    ///
    /// # Panics
    /// Panics if system time is before UNIX_EPOCH (should never happen in practice).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn current_frame(&self) -> CGRect {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_millis();

        let elapsed = now - self.start_time;
        let duration_ms = self.config.duration.as_millis();

        if elapsed >= duration_ms {
            // Animation complete
            return self.to;
        }

        let progress = elapsed as f64 / duration_ms as f64;
        let eased_progress = self.config.easing.apply(progress);

        interpolate_rect(&self.from, &self.to, eased_progress)
    }

    /// Checks if the animation is complete.
    ///
    /// # Panics
    /// Panics if system time is before UNIX_EPOCH (should never happen in practice).
    #[must_use]
    pub fn is_complete(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_millis();

        let elapsed = now - self.start_time;
        elapsed >= self.config.duration.as_millis()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        let easing = EasingFunction::Linear;
        assert!((easing.apply(0.0) - 0.0).abs() < f64::EPSILON);
        assert!((easing.apply(0.5) - 0.5).abs() < f64::EPSILON);
        assert!((easing.apply(1.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_easing_ease_out() {
        let easing = EasingFunction::EaseOut;
        assert!((easing.apply(0.0) - 0.0).abs() < f64::EPSILON);
        assert!(easing.apply(0.5) > 0.5); // Faster at start
        assert!((easing.apply(1.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_easing_ease_in_out() {
        let easing = EasingFunction::EaseInOut;
        assert!((easing.apply(0.0) - 0.0).abs() < f64::EPSILON);
        assert!((easing.apply(0.5) - 0.5).abs() < 0.01); // Should be around 0.5 at midpoint
        assert!(easing.apply(0.25) < 0.25); // Slower at start
        assert!(easing.apply(0.75) > 0.75); // Faster at end
        assert!((easing.apply(1.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_interpolate_rect() {
        let from = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 100.0,
                height: 100.0,
            },
        );

        let to = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 100.0, y: 100.0 },
            &core_graphics::geometry::CGSize {
                width: 200.0,
                height: 200.0,
            },
        );

        // At 0% progress
        let result = interpolate_rect(&from, &to, 0.0);
        assert!((result.origin.x - 0.0).abs() < f64::EPSILON);
        assert!((result.size.width - 100.0).abs() < f64::EPSILON);

        // At 50% progress
        let result = interpolate_rect(&from, &to, 0.5);
        assert!((result.origin.x - 50.0).abs() < f64::EPSILON);
        assert!((result.size.width - 150.0).abs() < f64::EPSILON);

        // At 100% progress
        let result = interpolate_rect(&from, &to, 1.0);
        assert!((result.origin.x - 100.0).abs() < f64::EPSILON);
        assert!((result.size.width - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_animation_completion() {
        let from = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 100.0,
                height: 100.0,
            },
        );

        let to = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 100.0, y: 100.0 },
            &core_graphics::geometry::CGSize {
                width: 200.0,
                height: 200.0,
            },
        );

        let config = AnimationConfig {
            duration: Duration::from_millis(10), // Very short for testing
            easing: EasingFunction::Linear,
            respect_reduce_motion: false,
        };

        let animation = WindowAnimation::new(from, to, config);

        // Wait for animation to complete
        std::thread::sleep(Duration::from_millis(20));

        assert!(animation.is_complete());
        let final_frame = animation.current_frame();
        assert!((final_frame.origin.x - to.origin.x).abs() < f64::EPSILON);
        assert!((final_frame.size.width - to.size.width).abs() < f64::EPSILON);
    }

    #[test]
    fn test_is_reduce_motion_enabled_does_not_panic() {
        // The actual value depends on system settings, but the function
        // must not panic regardless of the system configuration.
        let _result = is_reduce_motion_enabled();
    }
}
