//! macOS Accessibility API wrapper for window manipulation.

use crate::error::{Result, WindowError};
use core_graphics::display::CGRect;
use std::collections::HashMap;

#[cfg(target_os = "macos")]
use accessibility_sys::{
    kAXErrorSuccess, kAXValueTypeCGPoint, kAXValueTypeCGSize, AXIsProcessTrusted,
    AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide,
    AXUIElementRef, AXUIElementSetAttributeValue, AXValueCreate, AXValueGetValue, AXValueRef,
};
#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::boolean::CFBoolean;
#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionary;
#[cfg(target_os = "macos")]
use core_foundation_sys::base::{CFRelease, CFRetain, CFTypeRef};
#[cfg(target_os = "macos")]
use core_foundation_sys::string::{
    kCFStringEncodingUTF8, CFStringCreateWithCString, CFStringGetCString, CFStringRef,
};
#[cfg(target_os = "macos")]
use core_graphics::geometry::{CGPoint, CGSize};
#[cfg(target_os = "macos")]
use std::ffi::{c_void, CStr, CString};
#[cfg(target_os = "macos")]
use std::mem::MaybeUninit;

// Attribute constants
#[cfg(target_os = "macos")]
const AX_FOCUSED_APPLICATION: &str = "AXFocusedApplication";
#[cfg(target_os = "macos")]
const AX_FOCUSED_WINDOW: &str = "AXFocusedWindow";
#[cfg(target_os = "macos")]
const AX_TITLE: &str = "AXTitle";
#[cfg(target_os = "macos")]
const AX_POSITION: &str = "AXPosition";
#[cfg(target_os = "macos")]
const AX_SIZE: &str = "AXSize";
#[cfg(target_os = "macos")]
const AX_WINDOWS: &str = "AXWindows";
#[cfg(target_os = "macos")]
const AX_MINIMIZED: &str = "AXMinimized";
#[cfg(target_os = "macos")]
const AX_FULLSCREEN: &str = "AXFullScreen";

/// Window information retrieved from Accessibility API.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// Stable identifier for caching (pointer value as usize).
    pub element_ref: usize,
    /// The window's title.
    pub title: String,
    /// The application bundle ID.
    pub bundle_id: String,
    /// The current window frame.
    pub frame: CGRect,
}

/// Manages window manipulation via Accessibility APIs.
#[derive(Debug)]
pub struct AccessibilityManager {
    /// Cached window frames for restore functionality.
    saved_frames: HashMap<usize, CGRect>,
    /// Whether we have accessibility permissions.
    has_permission: bool,
    /// Cached element references for operations.
    #[cfg(target_os = "macos")]
    element_cache: HashMap<usize, AXUIElementRef>,
}

#[cfg(target_os = "macos")]
fn is_process_trusted() -> bool {
    // SAFETY: AXIsProcessTrusted is a safe FFI call returning a boolean.
    unsafe { AXIsProcessTrusted() }
}

#[cfg(target_os = "macos")]
fn is_process_trusted_with_prompt() -> bool {
    use core_foundation::string::CFString;

    // SAFETY: AXIsProcessTrustedWithOptions accepts a CFDictionaryRef.
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::true_value();
    let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
}

#[cfg(target_os = "macos")]
fn cfstring_to_string(cf_string: CFStringRef) -> Option<String> {
    if cf_string.is_null() {
        return None;
    }
    let mut buf = [0i8; 512];
    let max_len = i32::try_from(buf.len()).unwrap_or(i32::MAX) as isize;
    let success =
        unsafe { CFStringGetCString(cf_string, buf.as_mut_ptr(), max_len, kCFStringEncodingUTF8) };
    if success != 0 {
        let value_str = unsafe { CStr::from_ptr(buf.as_ptr()) };
        value_str.to_str().ok().map(String::from)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn make_cfstring(s: &str) -> std::result::Result<CFStringRef, WindowError> {
    let sanitized = s.replace('\0', "");
    let c_str = CString::new(sanitized).map_err(|_| WindowError::Message {
        message: "invalid string for CFString".to_string(),
    })?;
    Ok(unsafe {
        CFStringCreateWithCString(std::ptr::null(), c_str.as_ptr(), kCFStringEncodingUTF8)
    })
}

#[cfg(target_os = "macos")]
unsafe fn get_ax_attribute(
    element: AXUIElementRef,
    attr: &str,
) -> std::result::Result<CFTypeRef, WindowError> {
    let cf_attr = make_cfstring(attr)?;
    let mut value: CFTypeRef = std::ptr::null();
    let err = AXUIElementCopyAttributeValue(element, cf_attr, &mut value);
    CFRelease(cf_attr.cast());

    if err == kAXErrorSuccess {
        Ok(value)
    } else {
        Err(WindowError::AccessibilityError {
            message: format!("AX error: {err}"),
        })
    }
}

#[cfg(target_os = "macos")]
unsafe fn set_ax_attribute(
    element: AXUIElementRef,
    attr: &str,
    value: CFTypeRef,
) -> std::result::Result<(), WindowError> {
    let cf_attr = make_cfstring(attr)?;
    let err = AXUIElementSetAttributeValue(element, cf_attr, value);
    CFRelease(cf_attr.cast());

    if err == kAXErrorSuccess {
        Ok(())
    } else {
        Err(WindowError::AccessibilityError {
            message: format!("AX error: {err}"),
        })
    }
}

impl AccessibilityManager {
    /// Creates a new accessibility manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            saved_frames: HashMap::new(),
            has_permission: false,
            #[cfg(target_os = "macos")]
            element_cache: HashMap::new(),
        }
    }

    /// Checks if accessibility permission is granted.
    pub fn check_permission(&mut self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.has_permission = is_process_trusted();
            self.has_permission
        }

        #[cfg(not(target_os = "macos"))]
        false
    }

    /// Requests accessibility permission from the user.
    pub fn request_permission(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            self.has_permission = is_process_trusted_with_prompt();
            if self.has_permission {
                Ok(())
            } else {
                Err(WindowError::PermissionDenied)
            }
        }

        #[cfg(not(target_os = "macos"))]
        Err(WindowError::PlatformNotSupported)
    }

    /// Gets the frontmost application bundle ID.
    #[cfg(target_os = "macos")]
    pub fn get_frontmost_app(&self) -> Result<String> {
        use objc2_app_kit::NSWorkspace;

        // NSWorkspace doesn't require accessibility permission
        let workspace = unsafe { NSWorkspace::sharedWorkspace() };
        if let Some(app) = unsafe { workspace.frontmostApplication() } {
            if let Some(bundle_id) = unsafe { app.bundleIdentifier() } {
                return Ok(bundle_id.to_string());
            }
        }

        Err(WindowError::WindowNotFound)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_frontmost_app(&self) -> Result<String> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Gets the frontmost window for the current application.
    #[cfg(target_os = "macos")]
    pub fn get_frontmost_window(&mut self) -> Result<WindowInfo> {
        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let system_wide = unsafe { AXUIElementCreateSystemWide() };

        // Get focused application
        let focused_app_ref = unsafe {
            get_ax_attribute(system_wide, AX_FOCUSED_APPLICATION).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to get focused application: {e}"),
                }
            })?
        };

        // Get focused window
        let window_ref = unsafe {
            get_ax_attribute(focused_app_ref as AXUIElementRef, AX_FOCUSED_WINDOW).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to get focused window: {e}"),
                }
            })?
        };

        // Get window title
        let title = unsafe {
            get_ax_attribute(window_ref as AXUIElementRef, AX_TITLE)
                .ok()
                .and_then(|v| {
                    let s = cfstring_to_string(v as CFStringRef);
                    CFRelease(v);
                    s
                })
                .unwrap_or_else(|| "Untitled".to_string())
        };

        // Get bundle ID
        let bundle_id = self.get_frontmost_app().unwrap_or_default();

        // Get window frame
        let position = Self::get_position_from_ref(window_ref as AXUIElementRef)?;
        let size = Self::get_size_from_ref(window_ref as AXUIElementRef)?;
        let frame = CGRect::new(&position, &size);

        let element_ref = window_ref as usize;

        // Cache the element ref for later operations (retain it)
        unsafe { CFRetain(window_ref) };
        self.element_cache
            .insert(element_ref, window_ref as AXUIElementRef);

        // Clean up
        unsafe {
            CFRelease(focused_app_ref);
            CFRelease(system_wide.cast());
        };

        Ok(WindowInfo {
            element_ref,
            title,
            bundle_id,
            frame,
        })
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_frontmost_window(&mut self) -> Result<WindowInfo> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Gets the window frame.
    #[cfg(target_os = "macos")]
    pub fn get_window_frame(&self, window: &WindowInfo) -> Result<CGRect> {
        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self
            .element_cache
            .get(&window.element_ref)
            .copied()
            .ok_or(WindowError::WindowNotFound)?;

        let position = Self::get_position_from_ref(element)?;
        let size = Self::get_size_from_ref(element)?;

        Ok(CGRect::new(&position, &size))
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_window_frame(&self, window: &WindowInfo) -> Result<CGRect> {
        Ok(window.frame)
    }

    /// Sets the window frame.
    #[cfg(target_os = "macos")]
    pub fn set_window_frame(&self, window: &WindowInfo, frame: CGRect) -> Result<()> {
        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self
            .element_cache
            .get(&window.element_ref)
            .copied()
            .ok_or(WindowError::WindowNotFound)?;

        tracing::debug!(
            "Setting window frame: x={}, y={}, w={}, h={}",
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height
        );

        // Set position first, then size
        Self::set_position_on_ref(element, frame.origin)?;
        Self::set_size_on_ref(element, frame.size)?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn set_window_frame(&self, _window: &WindowInfo, _frame: CGRect) -> Result<()> {
        Err(WindowError::PlatformNotSupported)
    }

    #[cfg(target_os = "macos")]
    fn get_position_from_ref(element: AXUIElementRef) -> Result<CGPoint> {
        let value = unsafe {
            get_ax_attribute(element, AX_POSITION).map_err(|e| WindowError::AccessibilityError {
                message: format!("Failed to get position: {e}"),
            })?
        };

        let mut point = MaybeUninit::<CGPoint>::uninit();
        let success = unsafe {
            AXValueGetValue(
                value as AXValueRef,
                kAXValueTypeCGPoint,
                point.as_mut_ptr().cast::<c_void>(),
            )
        };

        unsafe { CFRelease(value) };

        if success {
            Ok(unsafe { point.assume_init() })
        } else {
            Err(WindowError::AccessibilityError {
                message: "Failed to extract CGPoint from AXValue".to_string(),
            })
        }
    }

    #[cfg(target_os = "macos")]
    fn get_size_from_ref(element: AXUIElementRef) -> Result<CGSize> {
        let value = unsafe {
            get_ax_attribute(element, AX_SIZE).map_err(|e| WindowError::AccessibilityError {
                message: format!("Failed to get size: {e}"),
            })?
        };

        let mut size = MaybeUninit::<CGSize>::uninit();
        let success = unsafe {
            AXValueGetValue(
                value as AXValueRef,
                kAXValueTypeCGSize,
                size.as_mut_ptr().cast::<c_void>(),
            )
        };

        unsafe { CFRelease(value) };

        if success {
            Ok(unsafe { size.assume_init() })
        } else {
            Err(WindowError::AccessibilityError {
                message: "Failed to extract CGSize from AXValue".to_string(),
            })
        }
    }

    #[cfg(target_os = "macos")]
    fn set_position_on_ref(element: AXUIElementRef, point: CGPoint) -> Result<()> {
        let ax_value = unsafe {
            AXValueCreate(
                kAXValueTypeCGPoint,
                std::ptr::addr_of!(point).cast::<c_void>(),
            )
        };

        if ax_value.is_null() {
            return Err(WindowError::AccessibilityError {
                message: "Failed to create AXValue for position".to_string(),
            });
        }

        let result = unsafe {
            set_ax_attribute(element, AX_POSITION, ax_value.cast()).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to set position: {e}"),
                }
            })
        };

        unsafe { CFRelease(ax_value.cast()) };
        result
    }

    #[cfg(target_os = "macos")]
    fn set_size_on_ref(element: AXUIElementRef, size: CGSize) -> Result<()> {
        let ax_value = unsafe {
            AXValueCreate(
                kAXValueTypeCGSize,
                std::ptr::addr_of!(size).cast::<c_void>(),
            )
        };

        if ax_value.is_null() {
            return Err(WindowError::AccessibilityError {
                message: "Failed to create AXValue for size".to_string(),
            });
        }

        let result = unsafe {
            set_ax_attribute(element, AX_SIZE, ax_value.cast()).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to set size: {e}"),
                }
            })
        };

        unsafe { CFRelease(ax_value.cast()) };
        result
    }

    /// Saves the current window frame for later restoration.
    pub fn save_frame(&mut self, window: &WindowInfo) -> Result<()> {
        let frame = self.get_window_frame(window)?;
        self.saved_frames.insert(window.element_ref, frame);
        Ok(())
    }

    /// Restores a previously saved window frame.
    pub fn restore_frame(&mut self, window: &WindowInfo) -> Result<CGRect> {
        self.saved_frames
            .get(&window.element_ref)
            .copied()
            .ok_or(WindowError::WindowNotFound)
    }

    /// Lists all windows for the frontmost application.
    #[cfg(target_os = "macos")]
    pub fn list_windows(&mut self) -> Result<Vec<WindowInfo>> {
        use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};

        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let system_wide = unsafe { AXUIElementCreateSystemWide() };

        // Get focused application
        let focused_app_ref = unsafe {
            get_ax_attribute(system_wide, AX_FOCUSED_APPLICATION).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to get focused application: {e}"),
                }
            })?
        };

        // Get windows array
        let windows_array = unsafe {
            get_ax_attribute(focused_app_ref as AXUIElementRef, AX_WINDOWS).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to get windows: {e}"),
                }
            })?
        };

        let bundle_id = self.get_frontmost_app().unwrap_or_default();
        let mut result = Vec::new();

        let count = unsafe { CFArrayGetCount(windows_array.cast()) };
        for i in 0..count {
            let window_ref =
                unsafe { CFArrayGetValueAtIndex(windows_array.cast(), i) as AXUIElementRef };

            let title = unsafe {
                get_ax_attribute(window_ref, AX_TITLE)
                    .ok()
                    .and_then(|v| {
                        let s = cfstring_to_string(v as CFStringRef);
                        CFRelease(v);
                        s
                    })
                    .unwrap_or_else(|| "Untitled".to_string())
            };

            if let (Ok(position), Ok(size)) = (
                Self::get_position_from_ref(window_ref),
                Self::get_size_from_ref(window_ref),
            ) {
                let frame = CGRect::new(&position, &size);
                let element_ref = window_ref as usize;

                // Cache for later operations (retain it)
                unsafe { CFRetain(window_ref.cast()) };
                self.element_cache.insert(element_ref, window_ref);

                result.push(WindowInfo {
                    element_ref,
                    title,
                    bundle_id: bundle_id.clone(),
                    frame,
                });
            }
        }

        // Clean up
        unsafe {
            CFRelease(windows_array);
            CFRelease(focused_app_ref);
            CFRelease(system_wide.cast());
        };

        Ok(result)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn list_windows(&mut self) -> Result<Vec<WindowInfo>> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Checks if a window is minimized.
    #[cfg(target_os = "macos")]
    pub fn is_minimized(&self, window: &WindowInfo) -> Result<bool> {
        use core_foundation_sys::number::CFBooleanGetValue;

        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self
            .element_cache
            .get(&window.element_ref)
            .copied()
            .ok_or(WindowError::WindowNotFound)?;

        let value = unsafe {
            get_ax_attribute(element, AX_MINIMIZED).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to get minimized state: {e}"),
                }
            })?
        };

        let minimized = unsafe { CFBooleanGetValue(value.cast()) };
        unsafe { CFRelease(value) };

        Ok(minimized)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn is_minimized(&self, _window: &WindowInfo) -> Result<bool> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Checks if a window is in fullscreen mode.
    #[cfg(target_os = "macos")]
    pub fn is_fullscreen(&self, window: &WindowInfo) -> Result<bool> {
        use core_foundation_sys::number::CFBooleanGetValue;

        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self
            .element_cache
            .get(&window.element_ref)
            .copied()
            .ok_or(WindowError::WindowNotFound)?;

        let value = unsafe {
            get_ax_attribute(element, AX_FULLSCREEN).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to get fullscreen state: {e}"),
                }
            })?
        };

        let fullscreen = unsafe { CFBooleanGetValue(value.cast()) };
        unsafe { CFRelease(value) };

        Ok(fullscreen)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn is_fullscreen(&self, _window: &WindowInfo) -> Result<bool> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Toggles fullscreen mode for a window.
    #[cfg(target_os = "macos")]
    pub fn toggle_fullscreen(&self, window: &WindowInfo) -> Result<()> {
        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self
            .element_cache
            .get(&window.element_ref)
            .copied()
            .ok_or(WindowError::WindowNotFound)?;

        // Get current fullscreen state
        let is_fullscreen = self.is_fullscreen(window)?;

        // Toggle fullscreen state
        let new_value = if is_fullscreen {
            CFBoolean::false_value()
        } else {
            CFBoolean::true_value()
        };

        unsafe {
            set_ax_attribute(element, AX_FULLSCREEN, new_value.as_CFTypeRef()).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to toggle fullscreen: {e}"),
                }
            })
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn toggle_fullscreen(&self, _window: &WindowInfo) -> Result<()> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Cleans up cached element references.
    #[cfg(target_os = "macos")]
    pub fn clear_cache(&mut self) {
        for (_, element) in self.element_cache.drain() {
            unsafe { CFRelease(element.cast()) };
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn clear_cache(&mut self) {}
}

impl Default for AccessibilityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
impl Drop for AccessibilityManager {
    fn drop(&mut self) {
        self.clear_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accessibility_manager_creation() {
        let manager = AccessibilityManager::new();
        assert!(!manager.has_permission);
        assert!(manager.saved_frames.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_permission_check() {
        let mut manager = AccessibilityManager::new();
        // Just verify it doesn't panic - actual permission depends on system state
        let _ = manager.check_permission();
    }
}
