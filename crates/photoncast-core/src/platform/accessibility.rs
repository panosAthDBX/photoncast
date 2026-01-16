//! Accessibility permission checking and requesting.
//!
//! This module provides functions to check and request accessibility permissions
//! on macOS. Accessibility permissions are required for global hotkey registration.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Status of accessibility permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionStatus {
    /// Permission has been granted.
    Granted,
    /// Permission has been denied.
    Denied,
    /// Permission status is unknown.
    #[default]
    Unknown,
}

impl PermissionStatus {
    /// Returns true if permission is granted.
    #[must_use]
    pub const fn is_granted(self) -> bool {
        matches!(self, Self::Granted)
    }

    /// Returns true if permission is denied.
    #[must_use]
    pub const fn is_denied(self) -> bool {
        matches!(self, Self::Denied)
    }
}

/// FFI bindings for macOS Accessibility APIs.
#[cfg(target_os = "macos")]
mod ffi {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    use core_foundation_sys::dictionary::CFDictionaryRef;

    // Link against ApplicationServices framework
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        /// Returns whether the current process is trusted for accessibility.
        pub fn AXIsProcessTrusted() -> bool;

        /// Returns whether the current process is trusted for accessibility,
        /// optionally displaying a prompt to the user.
        pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }

    /// Key for the kAXTrustedCheckOptionPrompt option.
    const AX_TRUSTED_CHECK_OPTION_PROMPT: &str = "AXTrustedCheckOptionPrompt";

    /// Checks if the process is trusted for accessibility.
    pub fn is_process_trusted() -> bool {
        // SAFETY: AXIsProcessTrusted is a safe FFI call that returns a boolean.
        unsafe { AXIsProcessTrusted() }
    }

    /// Checks if the process is trusted for accessibility and optionally shows a prompt.
    pub fn is_process_trusted_with_options(show_prompt: bool) -> bool {
        let key = CFString::new(AX_TRUSTED_CHECK_OPTION_PROMPT);
        let value = if show_prompt {
            CFBoolean::true_value()
        } else {
            CFBoolean::false_value()
        };

        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        // SAFETY: We're passing a valid CFDictionaryRef to the function.
        unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
    }
}

/// Checks if the application has accessibility permission.
///
/// This function queries the macOS accessibility API to determine if the
/// current process has been granted accessibility access.
///
/// # Returns
///
/// Returns `true` if the application has accessibility permission, `false` otherwise.
///
/// # Platform Specifics
///
/// This function always returns `false` on non-macOS platforms.
#[must_use]
pub fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        ffi::is_process_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Gets the current permission status.
///
/// # Returns
///
/// Returns the current `PermissionStatus`.
#[must_use]
pub fn get_permission_status() -> PermissionStatus {
    if check_accessibility_permission() {
        PermissionStatus::Granted
    } else {
        PermissionStatus::Denied
    }
}

/// Requests accessibility permission with a system prompt.
///
/// On macOS, this will display the system accessibility permission dialog
/// if the permission hasn't been granted yet.
///
/// # Returns
///
/// Returns `true` if permission is granted (either already or after the prompt),
/// `false` otherwise.
pub fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        ffi::is_process_trusted_with_options(true)
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Checks permission status without showing a prompt.
///
/// This is useful for checking the current status without triggering
/// the system permission dialog.
///
/// # Returns
///
/// Returns `true` if permission is granted, `false` otherwise.
#[must_use]
pub fn check_permission_silent() -> bool {
    #[cfg(target_os = "macos")]
    {
        ffi::is_process_trusted_with_options(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Opens the Accessibility pane in System Settings.
///
/// This opens the Privacy & Security → Accessibility section in System Settings,
/// allowing the user to grant accessibility permission to PhotonCast.
///
/// # Errors
///
/// Returns an error if the settings app cannot be opened.
pub fn open_accessibility_settings() -> std::io::Result<()> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()?;
    Ok(())
}

/// A poller for checking accessibility permission status.
///
/// This struct provides a way to periodically check for permission changes
/// and notify when permission is granted.
#[derive(Debug)]
pub struct PermissionPoller {
    /// Whether polling is currently active.
    is_polling: Arc<AtomicBool>,
    /// The polling interval.
    interval: Duration,
}

impl Default for PermissionPoller {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionPoller {
    /// Creates a new permission poller with a default interval of 1 second.
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_polling: Arc::new(AtomicBool::new(false)),
            interval: Duration::from_secs(1),
        }
    }

    /// Creates a new permission poller with a custom interval.
    #[must_use]
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            is_polling: Arc::new(AtomicBool::new(false)),
            interval,
        }
    }

    /// Returns whether polling is currently active.
    #[must_use]
    pub fn is_polling(&self) -> bool {
        self.is_polling.load(Ordering::Relaxed)
    }

    /// Starts polling for permission changes.
    ///
    /// This spawns an async task that periodically checks the permission status.
    /// When permission is granted, the callback is invoked and polling stops.
    ///
    /// # Arguments
    ///
    /// * `on_granted` - Callback to invoke when permission is granted.
    /// * `timeout` - Optional timeout after which polling stops even if permission
    ///   is not granted.
    pub async fn start_polling<F>(&self, on_granted: F, timeout: Option<Duration>)
    where
        F: FnOnce() + Send + 'static,
    {
        // Check if already granted
        if check_accessibility_permission() {
            on_granted();
            return;
        }

        self.is_polling.store(true, Ordering::Relaxed);
        let is_polling = Arc::clone(&self.is_polling);
        let interval = self.interval;

        tokio::spawn(async move {
            let start = std::time::Instant::now();

            loop {
                // Check if we should stop polling
                if !is_polling.load(Ordering::Relaxed) {
                    break;
                }

                // Check for timeout
                if let Some(timeout) = timeout {
                    if start.elapsed() >= timeout {
                        is_polling.store(false, Ordering::Relaxed);
                        break;
                    }
                }

                // Check permission status
                if check_accessibility_permission() {
                    is_polling.store(false, Ordering::Relaxed);
                    on_granted();
                    break;
                }

                // Wait before next check
                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Stops polling.
    pub fn stop_polling(&self) {
        self.is_polling.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_status_default() {
        let status = PermissionStatus::default();
        assert_eq!(status, PermissionStatus::Unknown);
    }

    #[test]
    fn test_permission_status_is_granted() {
        assert!(PermissionStatus::Granted.is_granted());
        assert!(!PermissionStatus::Denied.is_granted());
        assert!(!PermissionStatus::Unknown.is_granted());
    }

    #[test]
    fn test_permission_status_is_denied() {
        assert!(!PermissionStatus::Granted.is_denied());
        assert!(PermissionStatus::Denied.is_denied());
        assert!(!PermissionStatus::Unknown.is_denied());
    }

    #[test]
    fn test_permission_poller_new() {
        let poller = PermissionPoller::new();
        assert!(!poller.is_polling());
        assert_eq!(poller.interval, Duration::from_secs(1));
    }

    #[test]
    fn test_permission_poller_with_interval() {
        let interval = Duration::from_millis(500);
        let poller = PermissionPoller::with_interval(interval);
        assert_eq!(poller.interval, interval);
    }

    #[test]
    fn test_open_accessibility_settings_command() {
        // We can't test the actual command execution, but we can verify
        // the function exists and is callable
        // In a real test environment, we'd mock std::process::Command
    }

    #[test]
    fn test_get_permission_status() {
        // This test verifies the function returns a valid status
        // The actual status depends on the system state
        let status = get_permission_status();
        assert!(matches!(
            status,
            PermissionStatus::Granted | PermissionStatus::Denied
        ));
    }

    // Platform-specific tests
    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;

        #[test]
        fn test_check_accessibility_permission_compiles_and_runs() {
            // This test just verifies that the FFI call works without crashing
            // The actual return value depends on system state
            let _ = check_accessibility_permission();
        }

        #[test]
        fn test_check_permission_silent_compiles_and_runs() {
            // Verify the silent check works without showing a dialog
            let _ = check_permission_silent();
        }
    }
}
