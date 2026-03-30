//! Visual feedback overlay for window positioning commands.
//!
//! Shows a brief blue highlight on the target area when a window command is executed.
//!
//! The overlay:
//! - Shows a blue highlight on the target window position
//! - Dims the rest of the screen slightly
//! - Auto-dismisses after a configurable duration (default 200ms)
//! - Can be disabled in preferences
//!
//! Note: Full visual implementation pending - currently logs intent for debugging.

use core_graphics::geometry::CGRect;
use tracing::debug;

/// Shows a visual feedback overlay for the given target frame.
///
/// The overlay appears on the screen containing the target frame and shows:
/// - A blue highlight on the target area
/// - A dimmed overlay on the rest of the screen
///
/// The overlay automatically dismisses after the specified duration.
#[allow(unused_variables)]
pub fn show_overlay(target_frame: CGRect, duration_ms: u32) {
    // Log overlay intent for debugging
    debug!(
        "Window overlay: frame=({:.0}, {:.0}, {:.0}x{:.0}), duration={}ms",
        target_frame.origin.x,
        target_frame.origin.y,
        target_frame.size.width,
        target_frame.size.height,
        duration_ms
    );

    // NOTE: The visual overlay is not yet implemented. It requires creating a
    // transparent borderless NSWindow with CALayer-based drawing, which is complex
    // due to objc2 crate compatibility, coordinate system conversion, and main
    // thread dispatch requirements. The overlay setting can be toggled, but visual
    // feedback is currently logged rather than displayed.
}

/// Closes any currently visible overlay.
pub const fn close_overlay() {
    // No-op currently - overlay auto-dismisses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_show_overlay_doesnt_panic() {
        let frame = CGRect::new(
            &core_graphics::geometry::CGPoint::new(100.0, 100.0),
            &core_graphics::geometry::CGSize::new(400.0, 300.0),
        );
        show_overlay(frame, 200);
    }

    #[test]
    fn test_close_overlay_doesnt_panic() {
        close_overlay();
    }
}
