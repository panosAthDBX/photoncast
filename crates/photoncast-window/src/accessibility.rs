//! macOS Accessibility API wrapper for window manipulation.
//!
//! # Safety
//!
//! This module contains numerous `unsafe` blocks for FFI calls to macOS
//! accessibility and CoreFoundation APIs. The following invariants are
//! maintained throughout:
//!
//! - **AXUIElementRef lifetime**: All `AXUIElementRef` values are obtained from
//!   system APIs (`AXUIElementCreateSystemWide`, `AXUIElementCopyAttributeValue`)
//!   and released via `CFRelease` when no longer needed.
//! - **CFType reference counting**: CoreFoundation objects follow CF ownership rules.
//!   Objects obtained via "Copy" or "Create" functions are owned and must be released.
//!   Objects obtained via "Get" functions are borrowed and must not be released.
//! - **CFString conversion**: `CFStringGetCString` writes into a stack buffer with
//!   explicit bounds. Null CFStringRef values are checked before use.
//! - **AXValue boxing**: `AXValueCreate`/`AXValueGetValue` use matching type tags
//!   (`kAXValueTypeCGPoint`, `kAXValueTypeCGSize`) and correctly-sized buffers.
//! - **MaybeUninit**: Used for out-parameters in CF functions. Values are only read
//!   after the function returns success.
//! - **CFArray indexing**: Indices are bounds-checked against `CFArrayGetCount`
//!   before calling `CFArrayGetValueAtIndex`.
//! - **Thread safety**: This module's `WindowManager` uses `thread_local!` storage
//!   and is accessed only from the main thread.

use crate::error::{Result, WindowError};
use core_graphics::display::CGRect;
use std::collections::HashMap;

#[cfg(target_os = "macos")]
use accessibility_sys::{
    kAXErrorSuccess, kAXValueTypeCGPoint, kAXValueTypeCGSize, AXIsProcessTrusted,
    AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue, AXUIElementCreateApplication,
    AXUIElementCreateSystemWide, AXUIElementRef, AXUIElementSetAttributeValue, AXValueCreate,
    AXValueGetValue, AXValueRef,
};
#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::boolean::CFBoolean;
#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionary;
#[cfg(target_os = "macos")]
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
#[cfg(target_os = "macos")]
use core_foundation_sys::base::{CFRelease, CFRetain, CFTypeRef};
#[cfg(target_os = "macos")]
use core_foundation_sys::dictionary::CFDictionaryGetValueIfPresent;
#[cfg(target_os = "macos")]
use core_foundation_sys::dictionary::CFDictionaryRef;
#[cfg(target_os = "macos")]
use core_foundation_sys::number::{kCFNumberSInt32Type, CFNumberGetValue};
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
#[cfg(target_os = "macos")]
const AX_ENHANCED_USER_INTERFACE: &str = "AXEnhancedUserInterface";

// CGWindowList FFI bindings (not exposed by core-graphics crate)
#[cfg(target_os = "macos")]
type CGWindowID = u32;
#[cfg(target_os = "macos")]
type CGWindowListOption = u32;
#[cfg(target_os = "macos")]
const K_CG_NULL_WINDOW_ID: CGWindowID = 0;
#[cfg(target_os = "macos")]
const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: CGWindowListOption = 1 << 0;
#[cfg(target_os = "macos")]
const K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: CGWindowListOption = 1 << 4;

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(
        option: CGWindowListOption,
        relativeToWindow: CGWindowID,
    ) -> core_foundation_sys::array::CFArrayRef;
}

/// Information about a window obtained via CGWindowList API.
/// This is a lightweight struct that doesn't require accessibility permissions.
#[derive(Debug, Clone)]
pub struct CGWindowInfo {
    /// The window's title (may be empty for some windows).
    pub title: String,
    /// The owner application name.
    pub owner_name: String,
    /// The owner application PID.
    pub owner_pid: i32,
    /// Window layer (0 = normal windows).
    pub layer: i32,
}

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
        let workspace = NSWorkspace::sharedWorkspace();
        let app = workspace.frontmostApplication().ok_or_else(|| {
            tracing::debug!("NSWorkspace.frontmostApplication() returned None");
            WindowError::WindowNotFound
        })?;

        let name = app
            .localizedName()
            .map_or_else(|| "<no name>".to_string(), |n| n.to_string());

        if let Some(bundle_id) = app.bundleIdentifier() {
            return Ok(bundle_id.to_string());
        }

        tracing::debug!("Frontmost app '{}' has no bundle ID", name);
        Err(WindowError::WindowNotFound)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_frontmost_app(&self) -> Result<String> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Activates a running application by bundle ID, making it frontmost.
    #[cfg(target_os = "macos")]
    pub fn activate_app(&self, bundle_id: &str) -> Result<()> {
        use objc2_app_kit::NSWorkspace;

        let workspace = NSWorkspace::sharedWorkspace();
        let running_apps = workspace.runningApplications();
        let count = running_apps.len();

        for i in 0..count {
            let app = running_apps.objectAtIndex(i);
            if let Some(app_bundle_id) = app.bundleIdentifier() {
                if app_bundle_id.to_string() == bundle_id {
                    #[allow(deprecated)]
                    let activated =
                        app.activateWithOptions(
                            objc2_app_kit::NSApplicationActivationOptions::empty(),
                        );
                    if activated {
                        tracing::debug!("Activated app: {}", bundle_id);
                        return Ok(());
                    }
                    return Err(WindowError::AccessibilityError {
                        message: format!("Failed to activate app: {}", bundle_id),
                    });
                }
            }
        }

        Err(WindowError::WindowNotFound)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn activate_app(&self, _bundle_id: &str) -> Result<()> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Finds and activates the first visible application that isn't the given bundle ID.
    #[cfg(target_os = "macos")]
    pub fn activate_any_app_except(&self, except_bundle_id: &str) -> Result<String> {
        use objc2_app_kit::NSWorkspace;

        let workspace = NSWorkspace::sharedWorkspace();
        let running_apps = workspace.runningApplications();
        let count = running_apps.len();

        for i in 0..count {
            let app = running_apps.objectAtIndex(i);
            // Skip hidden apps
            if app.isHidden() {
                continue;
            }
            // Skip apps without windows (activation policy != regular)
            // NSApplicationActivationPolicyRegular = 0
            if app.activationPolicy() != objc2_app_kit::NSApplicationActivationPolicy::Regular {
                continue;
            }

            if let Some(app_bundle_id) = app.bundleIdentifier() {
                let bundle_str = app_bundle_id.to_string();
                if bundle_str != except_bundle_id && !bundle_str.contains("photoncast") {
                    #[allow(deprecated)]
                    let activated =
                        app.activateWithOptions(
                            objc2_app_kit::NSApplicationActivationOptions::empty(),
                        );
                    if activated {
                        tracing::debug!("Activated app: {}", bundle_str);
                        return Ok(bundle_str);
                    }
                }
            }
        }

        Err(WindowError::WindowNotFound)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn activate_any_app_except(&self, _except_bundle_id: &str) -> Result<String> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Gets the frontmost window for the current application.
    #[cfg(target_os = "macos")]
    pub fn get_frontmost_window(&mut self) -> Result<WindowInfo> {
        // Re-check permission each time (in case it was granted after startup)
        let trusted = is_process_trusted();
        tracing::debug!("is_process_trusted() = {}", trusted);
        self.has_permission = trusted;
        if !self.has_permission {
            tracing::error!("get_frontmost_window: Accessibility permission not granted");
            return Err(WindowError::PermissionDenied);
        }

        let system_wide = unsafe { AXUIElementCreateSystemWide() };

        // Use inner function to ensure cleanup happens on all paths
        let result = self.get_frontmost_window_inner(system_wide);

        // Always release system_wide
        unsafe { CFRelease(system_wide.cast()) };

        result
    }

    #[cfg(target_os = "macos")]
    fn get_frontmost_window_inner(&mut self, system_wide: AXUIElementRef) -> Result<WindowInfo> {
        // Get focused application
        let focused_app_ref = unsafe {
            get_ax_attribute(system_wide, AX_FOCUSED_APPLICATION).map_err(|e| {
                WindowError::AccessibilityError {
                    message: format!("Failed to get focused application: {e}"),
                }
            })?
        };

        // Get focused window (clean up focused_app_ref on error)
        let window_ref =
            match unsafe { get_ax_attribute(focused_app_ref as AXUIElementRef, AX_FOCUSED_WINDOW) }
            {
                Ok(w) => w,
                Err(e) => {
                    unsafe { CFRelease(focused_app_ref) };
                    return Err(WindowError::AccessibilityError {
                        message: format!("Failed to get focused window: {e}"),
                    });
                },
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

        // Get window frame (clean up on error)
        let position = match Self::get_position_from_ref(window_ref as AXUIElementRef) {
            Ok(p) => p,
            Err(e) => {
                unsafe {
                    CFRelease(focused_app_ref);
                    // Don't release window_ref here - it wasn't retained yet
                }
                return Err(e);
            },
        };

        let size = match Self::get_size_from_ref(window_ref as AXUIElementRef) {
            Ok(s) => s,
            Err(e) => {
                unsafe {
                    CFRelease(focused_app_ref);
                }
                return Err(e);
            },
        };

        let frame = CGRect::new(&position, &size);
        let element_ref = window_ref as usize;

        // Cache the element ref for later operations (retain it)
        unsafe { CFRetain(window_ref) };
        self.element_cache
            .insert(element_ref, window_ref as AXUIElementRef);

        // Clean up focused_app_ref (window_ref is now retained in cache)
        unsafe { CFRelease(focused_app_ref) };

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

    /// Gets a cached element reference, validating it's still valid.
    /// Removes stale elements from cache if they're no longer accessible.
    #[cfg(target_os = "macos")]
    fn get_validated_element(&mut self, element_ref: usize) -> Result<AXUIElementRef> {
        let element = self
            .element_cache
            .get(&element_ref)
            .copied()
            .ok_or(WindowError::WindowNotFound)?;

        // Validate the element is still accessible by trying to get its position
        // If this fails, the window has likely been closed
        if Self::get_position_from_ref(element).is_err() {
            tracing::debug!(
                "Cached element {} is stale, removing from cache",
                element_ref
            );
            if let Some(stale_element) = self.element_cache.remove(&element_ref) {
                unsafe { CFRelease(stale_element.cast()) };
            }
            self.saved_frames.remove(&element_ref);
            return Err(WindowError::WindowNotFound);
        }

        Ok(element)
    }

    /// Gets the window frame.
    #[cfg(target_os = "macos")]
    pub fn get_window_frame(&mut self, window: &WindowInfo) -> Result<CGRect> {
        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self.get_validated_element(window.element_ref)?;
        let position = Self::get_position_from_ref(element)?;
        let size = Self::get_size_from_ref(element)?;

        Ok(CGRect::new(&position, &size))
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_window_frame(&self, window: &WindowInfo) -> Result<CGRect> {
        Ok(window.frame)
    }

    /// Sets the window frame.
    ///
    /// Uses the Accessibility API with fallback to System Events AppleScript for
    /// apps that don't support AX resize (like Ghostty, some Electron apps).
    #[cfg(target_os = "macos")]
    pub fn set_window_frame(&mut self, window: &WindowInfo, frame: CGRect) -> Result<()> {
        use core_foundation_sys::number::CFBooleanGetValue;

        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self.get_validated_element(window.element_ref)?;
        let before_size = Self::get_size_from_ref(element).ok();

        // Get app element to check/disable AXEnhancedUserInterface
        let pid = get_pid_for_bundle_id(&window.bundle_id);
        let mut enhanced_ui_was_enabled = false;
        let app_element: Option<AXUIElementRef> =
            pid.map(|p| unsafe { AXUIElementCreateApplication(p) });

        // Temporarily disable AXEnhancedUserInterface if enabled (blocks resize in some apps)
        if let Some(app) = app_element {
            if let Ok(value) = unsafe { get_ax_attribute(app, AX_ENHANCED_USER_INTERFACE) } {
                let is_enabled = unsafe { CFBooleanGetValue(value.cast()) };
                unsafe { CFRelease(value) };
                if is_enabled {
                    enhanced_ui_was_enabled = true;
                    let _ = unsafe {
                        set_ax_attribute(
                            app,
                            AX_ENHANCED_USER_INTERFACE,
                            CFBoolean::false_value().as_CFTypeRef(),
                        )
                    };
                }
            }
        }

        // Rectangle's approach: size -> position -> size
        let size_result1 = Self::set_size_on_ref(element, frame.size);
        let pos_result = Self::set_position_on_ref(element, frame.origin);
        let size_result2 = Self::set_size_on_ref(element, frame.size);

        // Re-enable AXEnhancedUserInterface if we disabled it
        if let Some(app) = app_element {
            if enhanced_ui_was_enabled {
                let _ = unsafe {
                    set_ax_attribute(
                        app,
                        AX_ENHANCED_USER_INTERFACE,
                        CFBoolean::true_value().as_CFTypeRef(),
                    )
                };
            }
            unsafe { CFRelease(app.cast()) };
        }

        // Check if AX resize worked
        let final_size = Self::get_size_from_ref(element).ok();
        let size_changed = match (before_size, final_size) {
            (Some(b), Some(a)) => {
                (a.width - b.width).abs() > 5.0 || (a.height - b.height).abs() > 5.0
            },
            _ => false,
        };

        // If AX resize failed, try System Events AppleScript as fallback
        if !size_changed && (size_result1.is_err() || size_result2.is_err()) {
            tracing::debug!(
                "AX resize failed for '{}', trying AppleScript",
                window.bundle_id
            );
            if !set_window_bounds_via_applescript(&window.bundle_id, frame) {
                tracing::warn!("Window resize not supported by '{}'", window.bundle_id);
            }
        }

        pos_result
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

    /// Focuses (raises) a window by its title.
    /// This is used to activate a specific window when an app has multiple windows.
    #[cfg(target_os = "macos")]
    pub fn focus_window_by_title(&mut self, title: &str) -> Result<WindowInfo> {
        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        // Get all windows and find the one with matching title
        let windows = self.list_windows()?;
        let window = windows
            .into_iter()
            .find(|w| w.title == title)
            .ok_or_else(|| WindowError::AccessibilityError {
                message: format!("No window found with title: {}", title),
            })?;

        // Raise the window using AXRaise action
        let element = self.get_validated_element(window.element_ref)?;

        let result = unsafe {
            let action = core_foundation::string::CFString::new("AXRaise");
            accessibility_sys::AXUIElementPerformAction(element, action.as_concrete_TypeRef())
        };

        if result != 0 {
            tracing::warn!(
                "AXRaise failed with error {}, window may not be raised",
                result
            );
        }

        tracing::debug!("Focused window: '{}'", window.title);
        Ok(window)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn focus_window_by_title(&mut self, _title: &str) -> Result<WindowInfo> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Focuses the first window that doesn't look like a launcher/cargo terminal.
    /// This is a fallback when we can't identify the correct window by title.
    #[cfg(target_os = "macos")]
    pub fn focus_first_non_launcher_window(&mut self) -> Result<WindowInfo> {
        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let windows = self.list_windows()?;

        // Find a window that doesn't look like the launcher terminal
        let window = windows
            .into_iter()
            .find(|w| {
                let title_lower = w.title.to_lowercase();
                // Exclude windows that look like they're running the launcher
                !title_lower.contains("cargo run")
                    && !title_lower.contains("photoncast")
                    && !title_lower.contains("cargo build")
                    && !title_lower.contains("cargo test")
            })
            .ok_or_else(|| WindowError::AccessibilityError {
                message: "No suitable window found (all windows appear to be launcher terminals)"
                    .to_string(),
            })?;

        // Raise the window
        let element = self.get_validated_element(window.element_ref)?;

        let result = unsafe {
            let action = core_foundation::string::CFString::new("AXRaise");
            accessibility_sys::AXUIElementPerformAction(element, action.as_concrete_TypeRef())
        };

        if result != 0 {
            tracing::warn!(
                "AXRaise failed with error {}, window may not be raised",
                result
            );
        }

        tracing::info!("Focused non-launcher window: '{}'", window.title);
        Ok(window)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn focus_first_non_launcher_window(&mut self) -> Result<WindowInfo> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Checks if a window is minimized.
    #[cfg(target_os = "macos")]
    pub fn is_minimized(&mut self, window: &WindowInfo) -> Result<bool> {
        use core_foundation_sys::number::CFBooleanGetValue;

        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self.get_validated_element(window.element_ref)?;

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
    pub fn is_minimized(&mut self, _window: &WindowInfo) -> Result<bool> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Checks if a window is in fullscreen mode.
    #[cfg(target_os = "macos")]
    pub fn is_fullscreen(&mut self, window: &WindowInfo) -> Result<bool> {
        use core_foundation_sys::number::CFBooleanGetValue;

        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self.get_validated_element(window.element_ref)?;

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
    pub fn is_fullscreen(&mut self, _window: &WindowInfo) -> Result<bool> {
        Err(WindowError::PlatformNotSupported)
    }

    /// Toggles fullscreen mode for a window.
    #[cfg(target_os = "macos")]
    pub fn toggle_fullscreen(&mut self, window: &WindowInfo) -> Result<()> {
        if !self.has_permission {
            return Err(WindowError::PermissionDenied);
        }

        let element = self.get_validated_element(window.element_ref)?;

        // Get current fullscreen state - need to do this before getting element
        // to avoid double mutable borrow
        let is_fs = {
            let fs_element = self
                .element_cache
                .get(&window.element_ref)
                .copied()
                .ok_or(WindowError::WindowNotFound)?;

            let value = unsafe {
                get_ax_attribute(fs_element, AX_FULLSCREEN).map_err(|e| {
                    WindowError::AccessibilityError {
                        message: format!("Failed to get fullscreen state: {e}"),
                    }
                })?
            };
            let fullscreen =
                unsafe { core_foundation_sys::number::CFBooleanGetValue(value.cast()) };
            unsafe { CFRelease(value) };
            fullscreen
        };

        // Toggle fullscreen state
        let new_value = if is_fs {
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
    pub fn toggle_fullscreen(&mut self, _window: &WindowInfo) -> Result<()> {
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

/// Gets the frontmost window using CGWindowList API.
///
/// This does NOT require accessibility permissions and works even when another app is active.
/// Returns (owner_name, title, pid) of the first normal window (layer 0) on screen.
#[cfg(target_os = "macos")]
#[allow(clippy::too_many_lines)]
pub fn get_frontmost_window_via_cgwindowlist() -> Option<CGWindowInfo> {
    use core_foundation::string::CFString;

    let options =
        K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS;
    let window_list = unsafe { CGWindowListCopyWindowInfo(options, K_CG_NULL_WINDOW_ID) };

    if window_list.is_null() {
        tracing::debug!("CGWindowListCopyWindowInfo returned null");
        return None;
    }

    let count = unsafe { CFArrayGetCount(window_list) };
    tracing::debug!("CGWindowList returned {} windows", count);

    // Window list is in front-to-back order, so first normal window is frontmost
    for i in 0..count {
        let dict = unsafe { CFArrayGetValueAtIndex(window_list, i) as CFDictionaryRef };
        if dict.is_null() {
            continue;
        }

        // Get window layer - we only want normal windows (layer 0)
        let layer_key = CFString::new("kCGWindowLayer");
        let mut layer_value: *const c_void = std::ptr::null();
        let layer = if unsafe {
            CFDictionaryGetValueIfPresent(dict, layer_key.as_CFTypeRef().cast(), &mut layer_value)
        } != 0
            && !layer_value.is_null()
        {
            let mut val: i32 = 0;
            if unsafe {
                CFNumberGetValue(
                    layer_value.cast(),
                    kCFNumberSInt32Type,
                    std::ptr::addr_of_mut!(val).cast::<c_void>(),
                )
            } {
                val
            } else {
                continue;
            }
        } else {
            continue;
        };

        // Skip non-normal windows (menu bar, dock, etc have layer != 0)
        if layer != 0 {
            continue;
        }

        // Get owner name
        let owner_key = CFString::new("kCGWindowOwnerName");
        let mut owner_value: *const c_void = std::ptr::null();
        let owner_name = if unsafe {
            CFDictionaryGetValueIfPresent(dict, owner_key.as_CFTypeRef().cast(), &mut owner_value)
        } != 0
            && !owner_value.is_null()
        {
            cfstring_to_string(owner_value as CFStringRef).unwrap_or_default()
        } else {
            String::new()
        };

        // Skip windows from Photoncast itself
        if owner_name.to_lowercase().contains("photoncast") {
            continue;
        }

        // Get window title
        let title_key = CFString::new("kCGWindowName");
        let mut title_value: *const c_void = std::ptr::null();
        let title = if unsafe {
            CFDictionaryGetValueIfPresent(dict, title_key.as_CFTypeRef().cast(), &mut title_value)
        } != 0
            && !title_value.is_null()
        {
            cfstring_to_string(title_value as CFStringRef).unwrap_or_default()
        } else {
            String::new()
        };

        // Get owner PID
        let pid_key = CFString::new("kCGWindowOwnerPID");
        let mut pid_value: *const c_void = std::ptr::null();
        let owner_pid = if unsafe {
            CFDictionaryGetValueIfPresent(dict, pid_key.as_CFTypeRef().cast(), &mut pid_value)
        } != 0
            && !pid_value.is_null()
        {
            let mut val: i32 = 0;
            if unsafe {
                CFNumberGetValue(
                    pid_value.cast(),
                    kCFNumberSInt32Type,
                    std::ptr::addr_of_mut!(val).cast::<c_void>(),
                )
            } {
                val
            } else {
                0
            }
        } else {
            0
        };

        unsafe { CFRelease(window_list.cast()) };

        tracing::debug!(
            "CGWindowList frontmost: owner='{}', title='{}', pid={}, layer={}",
            owner_name,
            title,
            owner_pid,
            layer
        );

        return Some(CGWindowInfo {
            title,
            owner_name,
            owner_pid,
            layer,
        });
    }

    unsafe { CFRelease(window_list.cast()) };
    None
}

/// Gets the bundle ID for a PID using NSRunningApplication.
#[cfg(target_os = "macos")]
pub fn get_bundle_id_for_pid(pid: i32) -> Option<String> {
    use objc2_app_kit::NSRunningApplication;

    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);

    app.and_then(|a| a.bundleIdentifier().map(|s| s.to_string()))
}

/// Gets the PID for a bundle ID using NSWorkspace.
#[cfg(target_os = "macos")]
pub fn get_pid_for_bundle_id(bundle_id: &str) -> Option<i32> {
    use objc2_app_kit::NSWorkspace;

    let workspace = NSWorkspace::sharedWorkspace();
    let running_apps = workspace.runningApplications();
    let count = running_apps.len();

    for i in 0..count {
        let app = running_apps.objectAtIndex(i);
        if let Some(app_bundle_id) = app.bundleIdentifier() {
            if app_bundle_id.to_string() == bundle_id {
                return Some(app.processIdentifier());
            }
        }
    }
    None
}

/// Gets the window ID for a PID from CGWindowList.
/// Returns the first normal window (layer 0) owned by the given PID.
#[cfg(target_os = "macos")]
#[allow(clippy::cast_sign_loss)]
pub fn get_window_id_for_pid(pid: i32) -> Option<CGWindowID> {
    use core_foundation::string::CFString;

    let options =
        K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS;
    let window_list = unsafe { CGWindowListCopyWindowInfo(options, K_CG_NULL_WINDOW_ID) };

    if window_list.is_null() {
        return None;
    }

    let count = unsafe { CFArrayGetCount(window_list) };

    for i in 0..count {
        let dict = unsafe { CFArrayGetValueAtIndex(window_list, i) as CFDictionaryRef };
        if dict.is_null() {
            continue;
        }

        // Get window layer - we only want normal windows (layer 0)
        let layer_key = CFString::new("kCGWindowLayer");
        let mut layer_value: *const c_void = std::ptr::null();
        let layer = if unsafe {
            CFDictionaryGetValueIfPresent(dict, layer_key.as_CFTypeRef().cast(), &mut layer_value)
        } != 0
            && !layer_value.is_null()
        {
            let mut val: i32 = 0;
            if unsafe {
                CFNumberGetValue(
                    layer_value.cast(),
                    kCFNumberSInt32Type,
                    std::ptr::addr_of_mut!(val).cast::<c_void>(),
                )
            } {
                val
            } else {
                continue;
            }
        } else {
            continue;
        };

        if layer != 0 {
            continue;
        }

        // Get owner PID
        let pid_key = CFString::new("kCGWindowOwnerPID");
        let mut pid_value: *const c_void = std::ptr::null();
        let owner_pid = if unsafe {
            CFDictionaryGetValueIfPresent(dict, pid_key.as_CFTypeRef().cast(), &mut pid_value)
        } != 0
            && !pid_value.is_null()
        {
            let mut val: i32 = 0;
            if unsafe {
                CFNumberGetValue(
                    pid_value.cast(),
                    kCFNumberSInt32Type,
                    std::ptr::addr_of_mut!(val).cast::<c_void>(),
                )
            } {
                val
            } else {
                continue;
            }
        } else {
            continue;
        };

        if owner_pid != pid {
            continue;
        }

        // Get window ID
        let wid_key = CFString::new("kCGWindowNumber");
        let mut wid_value: *const c_void = std::ptr::null();
        let window_id = if unsafe {
            CFDictionaryGetValueIfPresent(dict, wid_key.as_CFTypeRef().cast(), &mut wid_value)
        } != 0
            && !wid_value.is_null()
        {
            let mut val: i32 = 0;
            if unsafe {
                CFNumberGetValue(
                    wid_value.cast(),
                    kCFNumberSInt32Type,
                    std::ptr::addr_of_mut!(val).cast::<c_void>(),
                )
            } {
                val as CGWindowID
            } else {
                continue;
            }
        } else {
            continue;
        };

        unsafe { CFRelease(window_list.cast()) };
        return Some(window_id);
    }

    unsafe { CFRelease(window_list.cast()) };
    None
}

/// Sanitizes a string for safe interpolation into AppleScript.
/// Escapes backslashes and double quotes to prevent injection attacks.
#[cfg(target_os = "macos")]
fn sanitize_for_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Validates that a bundle ID contains only safe characters.
/// Bundle IDs should match the pattern [a-zA-Z0-9._-]+
#[cfg(target_os = "macos")]
fn is_valid_bundle_id(bundle_id: &str) -> bool {
    !bundle_id.is_empty()
        && bundle_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// Sets window bounds using AppleScript via System Events as a fallback.
/// This uses the accessibility scripting bridge rather than app-specific scripting.
/// Returns true on success.
#[cfg(target_os = "macos")]
#[allow(clippy::cast_possible_truncation)]
pub fn set_window_bounds_via_applescript(bundle_id: &str, frame: CGRect) -> bool {
    use std::process::Command;

    // Validate bundle ID to prevent injection
    if !is_valid_bundle_id(bundle_id) {
        tracing::warn!(
            "Invalid bundle ID rejected for AppleScript: {:?}",
            bundle_id
        );
        return false;
    }

    // First, get the app name from bundle ID (bundle_id is validated above)
    let get_name_script = format!(
        r#"tell application "System Events" to get name of first process whose bundle identifier is "{}""#,
        bundle_id
    );

    let name_result = Command::new("osascript")
        .args(["-e", &get_name_script])
        .output();

    let app_name = match name_result {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        },
        _ => {
            tracing::warn!("Could not get app name for bundle {}", bundle_id);
            return false;
        },
    };

    // Sanitize app_name since it comes from external source (System Events output)
    let safe_app_name = sanitize_for_applescript(&app_name);

    // Truncation is intentional - AppleScript uses integer coordinates
    let x = frame.origin.x as i32;
    let y = frame.origin.y as i32;
    let width = frame.size.width as i32;
    let height = frame.size.height as i32;

    // Use System Events to set position and size via accessibility
    let script = format!(
        r#"tell application "System Events"
    tell process "{}"
        set position of window 1 to {{{}, {}}}
        set size of window 1 to {{{}, {}}}
    end tell
end tell"#,
        safe_app_name, x, y, width, height
    );

    let result = Command::new("osascript").args(["-e", &script]).output();

    match result {
        Ok(output) => {
            if output.status.success() {
                tracing::debug!("System Events resize succeeded for {}", app_name);
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::debug!("System Events resize failed: {}", stderr.trim());
                false
            }
        },
        Err(e) => {
            tracing::warn!("Failed to run osascript: {}", e);
            false
        },
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_frontmost_window_via_cgwindowlist() -> Option<CGWindowInfo> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn get_bundle_id_for_pid(_pid: i32) -> Option<String> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn get_pid_for_bundle_id(_bundle_id: &str) -> Option<i32> {
    None
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
