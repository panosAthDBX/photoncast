//! Running application process management.

use crate::error::{AppError, Result};
use crate::models::RunningApp;

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

#[cfg(unix)]
fn send_sigkill(pid: u32) -> Result<()> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    #[allow(clippy::cast_possible_wrap)]
    let pid_i32 = pid as i32;

    kill(Pid::from_raw(pid_i32), Signal::SIGKILL)
        .map_err(|e| AppError::Process(format!("Failed to kill process: {}", e)))?;
    Ok(())
}

#[cfg(not(unix))]
fn send_sigkill(_pid: u32) -> Result<()> {
    Err(AppError::Process(
        "Force quit not implemented on this platform".to_string(),
    ))
}

/// Checks if an application process is responding.
///
/// # Errors
///
/// Returns an error if the check fails.
#[cfg(target_os = "macos")]
pub fn is_app_responding(pid: u32) -> Result<bool> {
    tracing::debug!("Checking if PID {} is responding", pid);

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
            // An app is responding if it's not terminated and not hidden (simplification)
            // The actual "responding" state requires more complex checks
            let terminated = unsafe { app.isTerminated() };
            Ok(!terminated)
        },
    )
}

#[cfg(not(target_os = "macos"))]
pub fn is_app_responding(pid: u32) -> Result<bool> {
    tracing::debug!("Checking if PID {} is responding (basic check)", pid);
    // On non-macOS, just check if the process exists
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        #[allow(clippy::cast_possible_wrap)]
        let pid_i32 = pid as i32;

        // Sending signal 0 checks if process exists without actually sending a signal
        match kill(Pid::from_raw(pid_i32), Signal::from(0)) {
            Ok(()) => Ok(true),
            Err(nix::errno::Errno::ESRCH) => Ok(false), // No such process
            Err(e) => Err(AppError::Process(format!("Failed to check process: {}", e))),
        }
    }
    #[cfg(not(unix))]
    {
        Ok(true) // Assume responding on non-Unix
    }
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
        let pid = std::process::id();
        let result = is_app_responding(pid);
        // On macOS, our own process may not be in NSRunningApplication
        // On other platforms, it should work
        println!("is_app_responding result: {:?}", result);
    }
}
