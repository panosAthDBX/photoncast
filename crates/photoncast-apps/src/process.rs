//! Running application process management.

use crate::actions::{ActionError, ActionResult};
use crate::error::{AppError, Result};
use crate::models::{RunningApp, RunningApplication};
use chrono::{DateTime, TimeZone, Utc};

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSRunningApplication, NSWorkspace};

/// Gets a list of all running applications.
///
/// # Errors
///
/// Returns an error if process enumeration fails.
#[cfg(target_os = "macos")]
pub fn get_running_apps() -> Result<Vec<RunningApp>> {
    use objc2_foundation::NSString;

    tracing::info!("Enumerating running applications");

    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let apps = unsafe { workspace.runningApplications() };

    let mut result = Vec::new();
    let count = apps.count();
    for i in 0..count {
        let app = unsafe { apps.objectAtIndex(i) };

        #[allow(clippy::cast_sign_loss)]
        let pid = unsafe { app.processIdentifier() } as u32;

        let name = unsafe { app.localizedName() }.map_or_else(
            || format!("Process {}", pid),
            |s: objc2::rc::Retained<NSString>| s.to_string(),
        );

        let bundle_id =
            unsafe { app.bundleIdentifier() }.map(|s: objc2::rc::Retained<NSString>| s.to_string());

        let is_responding = !unsafe { app.isTerminated() };

        result.push(RunningApp {
            pid,
            name,
            bundle_id,
            is_responding,
            memory_bytes: None,
            cpu_percent: None,
        });
    }

    tracing::debug!("Found {} running applications", result.len());
    Ok(result)
}

#[cfg(not(target_os = "macos"))]
pub fn get_running_apps() -> Result<Vec<RunningApp>> {
    tracing::warn!("Running app enumeration only available on macOS");
    Ok(Vec::new())
}

/// Gets detailed information about all running applications.
///
/// Returns `RunningApplication` with full details including:
/// - `is_responding`: Whether the app responds to Apple Events
/// - `is_hidden`: Whether the app is currently hidden
/// - `launch_time`: When the app was launched
///
/// # Errors
///
/// Returns an error if process enumeration fails.
#[cfg(target_os = "macos")]
pub fn get_running_apps_detailed() -> Result<Vec<RunningApplication>> {
    tracing::info!("Enumerating running applications with detailed info");

    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let apps = unsafe { workspace.runningApplications() };

    let mut result = Vec::new();
    let count = apps.count();
    for i in 0..count {
        let app = unsafe { apps.objectAtIndex(i) };

        // Skip apps without bundle IDs (system processes, etc.)
        let Some(bundle_id_ns) = (unsafe { app.bundleIdentifier() }) else {
            continue;
        };
        let bundle_id = bundle_id_ns.to_string();

        #[allow(clippy::cast_sign_loss)]
        let pid = unsafe { app.processIdentifier() } as u32;

        // Get is_hidden directly from NSRunningApplication
        let is_hidden = unsafe { app.isHidden() };

        // Get launch time from NSRunningApplication.launchDate
        let launch_time = get_app_launch_time(&app);

        // Check if app is responding (more expensive, uses Apple Events)
        #[allow(clippy::cast_possible_wrap)]
        let is_responding = is_app_responding(pid as i32);

        result.push(RunningApplication {
            pid,
            bundle_id,
            is_responding,
            is_hidden,
            launch_time,
        });
    }

    tracing::debug!("Found {} running applications with details", result.len());
    Ok(result)
}

/// Gets the launch time of an application from NSRunningApplication.
#[cfg(target_os = "macos")]
fn get_app_launch_time(app: &NSRunningApplication) -> DateTime<Utc> {
    // Try to get launch date from NSRunningApplication
    let launch_date = unsafe { app.launchDate() };

    match launch_date {
        Some(date) => {
            // NSDate.timeIntervalSince1970 returns seconds since Unix epoch
            let timestamp = unsafe { date.timeIntervalSince1970() };
            #[allow(clippy::cast_possible_truncation)]
            let secs = timestamp as i64;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let nanos = ((timestamp - secs as f64) * 1_000_000_000.0) as u32;
            Utc.timestamp_opt(secs, nanos).single().unwrap_or_else(Utc::now)
        }
        None => {
            // Fallback to current time if launch date not available
            Utc::now()
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_running_apps_detailed() -> Result<Vec<RunningApplication>> {
    tracing::warn!("Running app detailed enumeration only available on macOS");
    Ok(Vec::new())
}

/// Gets the bundle ID of the currently frontmost (active) application.
///
/// This is used by the auto-quit feature to track which app is currently being used.
///
/// # Returns
///
/// Returns `Some(bundle_id)` if there's a frontmost app with a bundle ID,
/// or `None` if no app is frontmost or it has no bundle ID.
#[cfg(target_os = "macos")]
#[must_use]
pub fn get_frontmost_app_bundle_id() -> Option<String> {
    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let app = unsafe { workspace.frontmostApplication() }?;
    let bundle_id = unsafe { app.bundleIdentifier() }?;
    Some(bundle_id.to_string())
}

#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn get_frontmost_app_bundle_id() -> Option<String> {
    None
}

/// Attempts to gracefully quit an application.
///
/// Sends a quit request to the application, allowing it to save state and clean up.
///
/// # Errors
///
/// Returns an error if the quit request fails.
#[cfg(target_os = "macos")]
pub fn quit_app(pid: u32) -> Result<()> {
    tracing::info!("Sending quit request to PID {}", pid);

    #[allow(clippy::cast_possible_wrap)]
    let pid_i32 = pid as i32;

    let app = unsafe { NSRunningApplication::runningApplicationWithProcessIdentifier(pid_i32) };

    app.map_or_else(
        || {
            Err(AppError::Process(format!(
                "No running application found with PID {}",
                pid
            )))
        },
        |app| {
            let success = unsafe { app.terminate() };
            if success {
                tracing::info!("Successfully sent terminate request to PID {}", pid);
                Ok(())
            } else {
                Err(AppError::Process(format!(
                    "Failed to terminate process {} - app may not support graceful quit",
                    pid
                )))
            }
        },
    )
}

#[cfg(not(target_os = "macos"))]
pub fn quit_app(pid: u32) -> Result<()> {
    tracing::warn!("Graceful quit only available on macOS, trying SIGTERM");
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        #[allow(clippy::cast_possible_wrap)]
        let pid_i32 = pid as i32;

        kill(Pid::from_raw(pid_i32), Signal::SIGTERM)
            .map_err(|e| AppError::Process(format!("Failed to send SIGTERM: {}", e)))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        Err(AppError::Process(
            "Quit not implemented on this platform".to_string(),
        ))
    }
}

/// Force quits an application immediately.
///
/// Kills the application process without allowing cleanup.
///
/// # Errors
///
/// Returns an error if the force quit fails.
#[cfg(target_os = "macos")]
pub fn force_quit_app(pid: u32) -> Result<()> {
    tracing::info!("Force quitting PID {}", pid);

    #[allow(clippy::cast_possible_wrap)]
    let pid_i32 = pid as i32;

    let app = unsafe { NSRunningApplication::runningApplicationWithProcessIdentifier(pid_i32) };

    app.map_or_else(
        || {
            // App not in NSRunningApplication, try SIGKILL directly
            tracing::warn!("App not found via NSRunningApplication, using SIGKILL");
            send_sigkill(pid)
        },
        |app| {
            let success = unsafe { app.forceTerminate() };
            if success {
                tracing::info!("Successfully force terminated PID {}", pid);
                Ok(())
            } else {
                // Fall back to SIGKILL
                tracing::warn!("forceTerminate failed, using SIGKILL");
                send_sigkill(pid)
            }
        },
    )
}

#[cfg(not(target_os = "macos"))]
pub fn force_quit_app(pid: u32) -> Result<()> {
    tracing::info!("Force quitting PID {}", pid);
    send_sigkill(pid)
}

/// Sends SIGKILL to a process.
///
/// This is the shared implementation used by both `force_quit_app` and
/// `force_quit_app_action` to avoid code duplication.
#[cfg(unix)]
fn send_sigkill(pid: u32) -> Result<()> {
    #[allow(clippy::cast_possible_wrap)]
    let pid_i32 = pid as i32;
    send_sigkill_action(pid_i32).map_err(|e| AppError::Process(e.to_string()))
}

#[cfg(not(unix))]
fn send_sigkill(_pid: u32) -> Result<()> {
    Err(AppError::Process(
        "Force quit not implemented on this platform".to_string(),
    ))
}

/// Checks if an application process is responding.
///
/// Uses Apple Events or process state monitoring to detect if app is responsive.
/// Returns `false` if the app doesn't respond within 2 seconds.
#[cfg(target_os = "macos")]
pub fn is_app_responding(pid: i32) -> bool {
    use std::process::Command;
    use std::time::{Duration, Instant};

    tracing::debug!("Checking if PID {} is responding", pid);

    // First check if the process exists via NSRunningApplication
    let app = unsafe { NSRunningApplication::runningApplicationWithProcessIdentifier(pid) };

    let Some(app) = app else {
        tracing::debug!("PID {} not found in NSRunningApplication", pid);
        return false;
    };

    // If the app is terminated, it's not responding
    if unsafe { app.isTerminated() } {
        return false;
    }

    // Use osascript with timeout to check if app responds to Apple Events
    // This is the most reliable way to detect "Not Responding" state
    let start = Instant::now();
    let timeout = Duration::from_secs(3); // Slightly longer than AppleScript timeout

    let mut child = match Command::new("osascript")
        .args([
            "-e",
            &format!(
                "with timeout 2 seconds
                    tell application \"System Events\"
                        set appExists to exists (process id {})
                    end tell
                end timeout",
                pid
            ),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            tracing::warn!("Failed to spawn osascript for PID {}: {}", pid, e);
            // Fall back - if process exists in NSRunningApplication, assume responding
            return true;
        }
    };

    // Wait for the process with manual timeout
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    tracing::debug!("PID {} is responding", pid);
                    return true;
                }

                // Check stderr for timeout indication
                if let Some(stderr) = child.stderr.take() {
                    use std::io::Read;
                    let mut buf = String::new();
                    let mut reader = std::io::BufReader::new(stderr).take(1024);
                    let _ = reader.read_to_string(&mut buf);

                    if buf.contains("timed out") || buf.contains("timeout") {
                        tracing::debug!("PID {} is not responding (AppleScript timed out)", pid);
                        return false;
                    }
                    tracing::debug!(
                        "Apple Events check failed for PID {}: {}",
                        pid,
                        buf.trim()
                    );
                }
                // Other errors - assume responding if process exists
                return true;
            }
            Ok(None) => {
                // Process still running, check timeout
                if start.elapsed() > timeout {
                    tracing::debug!("PID {} is not responding (check timed out)", pid);
                    let _ = child.kill();
                    return false;
                }
                // Sleep briefly before checking again
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                tracing::warn!("Error waiting for osascript: {}", e);
                return true; // Assume responding on error
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn is_app_responding(pid: i32) -> bool {
    tracing::debug!("Checking if PID {} is responding (basic check)", pid);
    // On non-macOS, just check if the process exists
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        // Sending signal 0 checks if process exists without actually sending a signal
        match kill(Pid::from_raw(pid), Signal::from(0)) {
            Ok(()) => true,
            Err(nix::errno::Errno::ESRCH) => false, // No such process
            Err(e) => {
                tracing::warn!("Failed to check process {}: {}", pid, e);
                false
            }
        }
    }
    #[cfg(not(unix))]
    {
        true // Assume responding on non-Unix
    }
}

/// Checks if an application with the given bundle ID is currently running.
///
/// Performs case-insensitive matching on the bundle ID.
#[cfg(target_os = "macos")]
pub fn is_app_running(bundle_id: &str) -> bool {
    tracing::debug!("Checking if app with bundle ID '{}' is running", bundle_id);

    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let apps = unsafe { workspace.runningApplications() };

    let bundle_id_lower = bundle_id.to_lowercase();
    let count = apps.count();

    for i in 0..count {
        let app = unsafe { apps.objectAtIndex(i) };

        if let Some(app_bundle_id) = unsafe { app.bundleIdentifier() } {
            let app_bundle_id_str = app_bundle_id.to_string();
            if app_bundle_id_str.to_lowercase() == bundle_id_lower {
                tracing::debug!("Found running app with bundle ID '{}'", bundle_id);
                return true;
            }
        }
    }

    tracing::debug!("No running app found with bundle ID '{}'", bundle_id);
    false
}

#[cfg(not(target_os = "macos"))]
pub fn is_app_running(_bundle_id: &str) -> bool {
    tracing::warn!("is_app_running only available on macOS");
    false
}

// ============================================================================
// Enhanced Quit Operations (Tasks 4.1-4.3)
// ============================================================================

/// Timeout duration for graceful quit operations.
const QUIT_TIMEOUT_SECS: u64 = 5;

/// Quits an application gracefully with a timeout.
///
/// Sends a terminate request and waits up to 5 seconds for the app to quit.
///
/// # Arguments
///
/// * `pid` - The process ID of the application to quit.
///
/// # Returns
///
/// * `Ok(true)` - The app quit successfully within the timeout.
/// * `Ok(false)` - The timeout was reached and the app is still running.
/// * `Err(_)` - An error occurred (e.g., app not found).
#[cfg(target_os = "macos")]
pub fn quit_app_with_timeout(pid: i32) -> ActionResult<bool> {
    use std::thread;
    use std::time::{Duration, Instant};

    tracing::info!("Sending quit request to PID {} with {}s timeout", pid, QUIT_TIMEOUT_SECS);

    let app = unsafe { NSRunningApplication::runningApplicationWithProcessIdentifier(pid) };

    let Some(app) = app else {
        return Err(ActionError::Process(format!(
            "No running application found with PID {}",
            pid
        )));
    };

    // Send terminate request
    let success = unsafe { app.terminate() };
    if !success {
        return Err(ActionError::OperationFailed {
            operation: "quit".to_string(),
            reason: format!("Failed to send terminate request to PID {} - app may not support graceful quit", pid),
        });
    }

    tracing::debug!("Terminate request sent to PID {}, waiting for quit...", pid);

    // Wait for app to terminate with timeout
    let start = Instant::now();
    let timeout = Duration::from_secs(QUIT_TIMEOUT_SECS);
    let poll_interval = Duration::from_millis(100);

    while start.elapsed() < timeout {
        if unsafe { app.isTerminated() } {
            tracing::info!("PID {} quit successfully in {:?}", pid, start.elapsed());
            return Ok(true);
        }
        thread::sleep(poll_interval);
    }

    tracing::warn!("PID {} did not quit within {} seconds", pid, QUIT_TIMEOUT_SECS);
    Ok(false)
}

#[cfg(not(target_os = "macos"))]
pub fn quit_app_with_timeout(pid: i32) -> ActionResult<bool> {
    use std::thread;
    use std::time::{Duration, Instant};

    tracing::warn!("Graceful quit only available on macOS, trying SIGTERM with timeout");

    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        // Send SIGTERM
        kill(Pid::from_raw(pid), Signal::SIGTERM)
            .map_err(|e| ActionError::Process(format!("Failed to send SIGTERM: {}", e)))?;

        // Wait for process to exit
        let start = Instant::now();
        let timeout = Duration::from_secs(QUIT_TIMEOUT_SECS);
        let poll_interval = Duration::from_millis(100);

        while start.elapsed() < timeout {
            // Check if process still exists (signal 0 doesn't send a signal, just checks)
            match kill(Pid::from_raw(pid), None) {
                Ok(()) => {
                    // Process still exists
                    thread::sleep(poll_interval);
                }
                Err(nix::errno::Errno::ESRCH) => {
                    // Process no longer exists
                    tracing::info!("PID {} quit successfully in {:?}", pid, start.elapsed());
                    return Ok(true);
                }
                Err(e) => {
                    return Err(ActionError::Process(format!("Error checking process: {}", e)));
                }
            }
        }

        tracing::warn!("PID {} did not quit within {} seconds", pid, QUIT_TIMEOUT_SECS);
        Ok(false)
    }

    #[cfg(not(unix))]
    {
        Err(ActionError::OperationFailed {
            operation: "quit".to_string(),
            reason: "Quit not implemented on this platform".to_string(),
        })
    }
}

/// Determines whether to show a confirmation dialog before force quitting.
///
/// Returns `false` if the app is not responding (no confirmation needed),
/// `true` otherwise (confirmation should be shown).
///
/// # Arguments
///
/// * `pid` - The process ID of the application to check.
pub fn should_confirm_force_quit(pid: i32) -> bool {
    if !is_app_responding(pid) {
        tracing::debug!("PID {} is not responding, no confirmation needed", pid);
        return false;
    }
    tracing::debug!("PID {} is responding, confirmation should be shown", pid);
    true
}

/// Quits an application by its bundle identifier.
///
/// Finds the running application with the given bundle ID and attempts
/// a graceful quit with timeout.
///
/// # Arguments
///
/// * `bundle_id` - The bundle identifier of the application (e.g., "com.apple.Safari").
///
/// # Returns
///
/// * `Ok(true)` - The app quit successfully.
/// * `Ok(false)` - The quit timed out.
/// * `Err(AppNotRunning)` - No running app with that bundle ID.
#[cfg(target_os = "macos")]
pub fn quit_app_by_bundle_id(bundle_id: &str) -> ActionResult<bool> {
    tracing::info!("Attempting to quit app with bundle ID: {}", bundle_id);

    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let apps = unsafe { workspace.runningApplications() };

    let bundle_id_lower = bundle_id.to_lowercase();
    let count = apps.count();

    for i in 0..count {
        let app = unsafe { apps.objectAtIndex(i) };

        if let Some(app_bundle_id) = unsafe { app.bundleIdentifier() } {
            let app_bundle_id_str = app_bundle_id.to_string();
            if app_bundle_id_str.to_lowercase() == bundle_id_lower {
                let pid = unsafe { app.processIdentifier() };
                tracing::debug!("Found running app '{}' with PID {}", bundle_id, pid);
                return quit_app_with_timeout(pid);
            }
        }
    }

    Err(ActionError::AppNotRunning {
        bundle_id: bundle_id.to_string(),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn quit_app_by_bundle_id(bundle_id: &str) -> ActionResult<bool> {
    tracing::warn!("quit_app_by_bundle_id only available on macOS");
    Err(ActionError::AppNotRunning {
        bundle_id: bundle_id.to_string(),
    })
}

/// Force quits an application immediately using SIGKILL.
///
/// This is a variant that returns `ActionResult` for consistency with
/// other action operations.
///
/// # Arguments
///
/// * `pid` - The process ID of the application to force quit.
///
/// # Errors
///
/// Returns an error if the process cannot be killed.
#[cfg(target_os = "macos")]
pub fn force_quit_app_action(pid: i32) -> ActionResult<()> {
    tracing::info!("Force quitting PID {}", pid);

    let app = unsafe { NSRunningApplication::runningApplicationWithProcessIdentifier(pid) };

    if let Some(app) = app {
        let success = unsafe { app.forceTerminate() };
        if success {
            tracing::info!("Successfully force terminated PID {}", pid);
            Ok(())
        } else {
            // Fall back to SIGKILL
            tracing::warn!("forceTerminate failed for PID {}, using SIGKILL", pid);
            send_sigkill_action(pid)
        }
    } else {
        // App not in NSRunningApplication, try SIGKILL directly
        tracing::warn!("App PID {} not found via NSRunningApplication, using SIGKILL", pid);
        send_sigkill_action(pid)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn force_quit_app_action(pid: i32) -> ActionResult<()> {
    tracing::info!("Force quitting PID {}", pid);
    send_sigkill_action(pid)
}

#[cfg(unix)]
fn send_sigkill_action(pid: i32) -> ActionResult<()> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    kill(Pid::from_raw(pid), Signal::SIGKILL)
        .map_err(|e| ActionError::Process(format!("Failed to kill process {}: {}", pid, e)))?;
    Ok(())
}

#[cfg(not(unix))]
fn send_sigkill_action(_pid: i32) -> ActionResult<()> {
    Err(ActionError::OperationFailed {
        operation: "force_quit".to_string(),
        reason: "Force quit not implemented on this platform".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_running_apps() {
        let result = get_running_apps();
        assert!(result.is_ok());
        #[cfg(target_os = "macos")]
        {
            let apps = result.unwrap();
            // Should have at least some apps running
            assert!(!apps.is_empty(), "Should have running apps on macOS");
        }
    }

    #[test]
    fn test_is_app_responding() {
        // Test with current process PID
        #[allow(clippy::cast_possible_wrap)]
        let pid = std::process::id() as i32;
        let result = is_app_responding(pid);
        // On macOS, our own process may not be in NSRunningApplication
        // On other platforms, it should work
        println!("is_app_responding result: {:?}", result);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_is_app_running() {
        // Test with Finder which is always running
        assert!(is_app_running("com.apple.finder"));
        // Case-insensitive check
        assert!(is_app_running("COM.APPLE.FINDER"));
        // Non-existent bundle ID
        assert!(!is_app_running("com.nonexistent.app"));
    }

    #[test]
    fn test_should_confirm_force_quit() {
        // Test that should_confirm_force_quit returns the inverse of is_app_responding
        // For a non-existent process (PID -1), is_app_responding returns false,
        // so should_confirm_force_quit should return false (no confirmation needed)
        let result = should_confirm_force_quit(-1);
        assert!(!result, "Non-existent process should not require confirmation");

        // For the current process, behavior depends on platform:
        // - On macOS: test process may not be in NSRunningApplication
        // - On non-macOS: will check process existence via signal 0
        #[allow(clippy::cast_possible_wrap)]
        let current_pid = std::process::id() as i32;
        let result = should_confirm_force_quit(current_pid);
        // Just verify it returns a boolean without panicking
        println!("should_confirm_force_quit for current process: {}", result);
    }

    #[test]
    fn test_quit_by_bundle_id_not_running() {
        // Test that quitting a non-running app returns AppNotRunning error
        let result = quit_app_by_bundle_id("com.nonexistent.app.that.does.not.exist");
        assert!(result.is_err(), "Should return error for non-running app");

        let err = result.unwrap_err();
        match err {
            ActionError::AppNotRunning { bundle_id } => {
                assert_eq!(bundle_id, "com.nonexistent.app.that.does.not.exist");
            }
            _ => panic!("Expected AppNotRunning error, got: {:?}", err),
        }
    }

    // ========================================================================
    // Task 8.1: Unit Tests - Running App Detection
    // ========================================================================

    /// Test that running apps are correctly detected.
    /// Finder (com.apple.finder) should always be running on macOS.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_running_app_detection() {
        // Get all running apps
        let apps = get_running_apps().expect("Should enumerate running apps");

        // Find Finder in the list
        let finder = apps
            .iter()
            .find(|app| app.bundle_id.as_deref() == Some("com.apple.finder"));

        assert!(
            finder.is_some(),
            "Finder (com.apple.finder) should always be in the running apps list"
        );

        let finder = finder.unwrap();
        assert_eq!(
            finder.name, "Finder",
            "Finder should have the correct display name"
        );
        assert!(finder.pid > 0, "Finder should have a valid PID");
        assert!(
            finder.is_responding,
            "Finder should be responding (not terminated)"
        );
    }

    /// Test that is_responding correctly detects responding apps.
    /// Finder should always be responding on macOS.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_is_responding() {
        // Find Finder's PID first
        let apps = get_running_apps().expect("Should enumerate running apps");
        let finder = apps
            .iter()
            .find(|app| app.bundle_id.as_deref() == Some("com.apple.finder"))
            .expect("Finder should be running");

        #[allow(clippy::cast_possible_wrap)]
        let pid = finder.pid as i32;

        // Finder should always be responding
        assert!(
            is_app_responding(pid),
            "Finder (PID {}) should be responding",
            pid
        );
    }

    /// Test that is_responding returns false for invalid PIDs.
    #[test]
    fn test_is_responding_invalid_pid() {
        // A negative PID should not be responding
        assert!(
            !is_app_responding(-1),
            "Negative PID should not be responding"
        );

        // A very high PID that almost certainly doesn't exist
        let invalid_pid = i32::MAX - 1;
        assert!(
            !is_app_responding(invalid_pid),
            "Non-existent high PID should not be responding"
        );
    }

    /// Test case-insensitive bundle ID lookup.
    /// com.apple.finder vs COM.APPLE.FINDER should both match.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_is_running_by_bundle_id() {
        // Test various case combinations for Finder
        let test_cases = [
            ("com.apple.finder", true),
            ("COM.APPLE.FINDER", true),
            ("Com.Apple.Finder", true),
            ("cOm.ApPlE.fInDeR", true),
            ("com.nonexistent.app", false),
            ("COM.NONEXISTENT.APP", false),
        ];

        for (bundle_id, expected) in test_cases {
            let result = is_app_running(bundle_id);
            assert_eq!(
                result, expected,
                "is_app_running('{}') should return {}",
                bundle_id, expected
            );
        }
    }

    /// Test detailed running app detection.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_running_app_detection_detailed() {
        let apps = get_running_apps_detailed().expect("Should get detailed running apps");

        // Should have at least some apps
        assert!(!apps.is_empty(), "Should have running apps with bundle IDs");

        // Find Finder in detailed list
        let finder = apps.iter().find(|app| app.bundle_id == "com.apple.finder");

        assert!(
            finder.is_some(),
            "Finder should be in detailed running apps list"
        );

        let finder = finder.unwrap();
        assert!(finder.pid > 0, "Finder should have a valid PID");
        assert!(finder.is_responding, "Finder should be responding");
        // launch_time should be in the past
        assert!(
            finder.launch_time <= chrono::Utc::now(),
            "Finder launch time should be in the past"
        );
    }
}
