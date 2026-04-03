//! Visual feedback overlay for window positioning commands.
//!
//! Shows a brief blue highlight on the target area when a window command is executed.
//!
//! The overlay:
//! - Shows a translucent blue highlight over the target window position
//! - Ignores mouse events so it never steals focus
//! - Auto-dismisses after a configurable duration (default 200ms)
//! - Can be disabled in preferences

#[cfg(target_os = "macos")]
use std::cell::RefCell;

use core_graphics::geometry::CGRect;
use tracing::debug;

#[cfg(target_os = "macos")]
use objc2::{msg_send, rc::Retained, runtime::AnyObject, sel, MainThreadMarker, MainThreadOnly};
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSFloatingWindowLevel, NSWindow, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSPoint, NSRect, NSSize};

#[cfg(target_os = "macos")]
thread_local! {
    static CURRENT_OVERLAY_WINDOW: RefCell<Option<Retained<NSWindow>>> = const { RefCell::new(None) };
}

#[cfg(target_os = "macos")]
fn ns_rect_from_cg(rect: CGRect) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.origin.x, rect.origin.y),
        NSSize::new(rect.size.width, rect.size.height),
    )
}

#[cfg(target_os = "macos")]
fn hide_current_overlay_window() {
    CURRENT_OVERLAY_WINDOW.with(|slot| {
        if let Some(window) = slot.borrow_mut().take() {
            window.orderOut(None);
        }
    });
}

/// Shows a visual feedback overlay for the given target frame.
///
/// The overlay appears over the target frame as a translucent, non-interactive
/// highlight window and automatically dismisses after the specified duration.
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

    #[cfg(target_os = "macos")]
    {
        let Some(mtm) = MainThreadMarker::new() else {
            debug!("Window overlay skipped: not running on the main thread");
            return;
        };

        hide_current_overlay_window();

        let overlay_window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                ns_rect_from_cg(target_frame),
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };

        unsafe { overlay_window.setReleasedWhenClosed(false) };
        overlay_window.setOpaque(false);
        overlay_window.setHasShadow(false);
        overlay_window.setIgnoresMouseEvents(true);
        overlay_window.setLevel(NSFloatingWindowLevel);
        overlay_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Transient
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        overlay_window.setBackgroundColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(
            0.18, 0.54, 1.0, 0.28,
        )));
        overlay_window.orderFrontRegardless();

        let delay_seconds = f64::from(duration_ms) / 1000.0;
        unsafe {
            let _: () = msg_send![
                &overlay_window,
                performSelector: sel!(orderOut:),
                withObject: None::<&AnyObject>,
                afterDelay: delay_seconds
            ];
        }

        CURRENT_OVERLAY_WINDOW.with(|slot| {
            *slot.borrow_mut() = Some(overlay_window);
        });
    }
}

/// Closes any currently visible overlay.
pub fn close_overlay() {
    #[cfg(target_os = "macos")]
    {
        if MainThreadMarker::new().is_some() {
            hide_current_overlay_window();
        }
    }
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

    #[cfg(target_os = "macos")]
    #[test]
    fn test_cg_rect_to_ns_rect_conversion() {
        let frame = CGRect::new(
            &core_graphics::geometry::CGPoint::new(10.0, 20.0),
            &core_graphics::geometry::CGSize::new(300.0, 400.0),
        );
        let rect = ns_rect_from_cg(frame);
        assert_eq!(rect.origin.x, 10.0);
        assert_eq!(rect.origin.y, 20.0);
        assert_eq!(rect.size.width, 300.0);
        assert_eq!(rect.size.height, 400.0);
    }
}
