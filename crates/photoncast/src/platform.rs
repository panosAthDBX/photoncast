//! Platform-specific functionality for PhotonCast

#[cfg(target_os = "macos")]
mod macos {
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{MainThreadMarker, NSRect};

    /// Resize the key window to the specified height, keeping width and position
    pub fn resize_window_height(new_height: f64) {
        // SAFETY: This function is called from GPUI which runs on the main thread
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        
        let app = NSApplication::sharedApplication(mtm);
        let Some(window) = app.keyWindow() else {
            return;
        };
        
        let current_frame = window.frame();
        
        // Calculate new frame - keep top position fixed, expand downward
        let height_diff = new_height - current_frame.size.height;
        let new_frame = NSRect::new(
            objc2_foundation::NSPoint::new(
                current_frame.origin.x,
                current_frame.origin.y - height_diff, // Move origin down to keep top fixed
            ),
            objc2_foundation::NSSize::new(current_frame.size.width, new_height),
        );
        
        // Animate the resize
        // SAFETY: The frame is valid and display/animate flags are booleans
        unsafe { window.setFrame_display_animate(new_frame, true, true) };
    }

    /// Get the current window height
    #[allow(dead_code)]
    pub fn get_window_height() -> Option<f64> {
        // SAFETY: This function is called from GPUI which runs on the main thread
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let app = NSApplication::sharedApplication(mtm);
        app.keyWindow().map(|window| window.frame().size.height)
    }
}

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(target_os = "macos"))]
pub fn resize_window_height(_new_height: f64) {
    // No-op on other platforms
}

#[cfg(not(target_os = "macos"))]
pub fn get_window_height() -> Option<f64> {
    None
}
