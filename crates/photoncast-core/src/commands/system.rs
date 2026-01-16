//! System command execution.

use std::process::Command;

use anyhow::{bail, Context, Result};
use thiserror::Error;
use tracing::{debug, error};

use crate::commands::SystemCommand;

/// Errors that can occur during command execution.
#[derive(Error, Debug)]
pub enum CommandError {
    /// Command execution failed.
    #[error("command '{command}' failed: {reason}")]
    ExecutionFailed {
        /// The command that failed.
        command: String,
        /// The reason for failure.
        reason: String,
    },

    /// Authorization is required.
    #[error("authorization required for '{command}'")]
    AuthorizationRequired {
        /// The command that requires authorization.
        command: String,
    },

    /// Command is not available on this system.
    #[error("command not available on this system")]
    NotAvailable,
}

impl CommandError {
    /// Returns a user-friendly message for the error.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::ExecutionFailed { command, reason } => {
                format!(
                    "Failed to execute \"{command}\". {reason}. Please try again or check system permissions."
                )
            },
            Self::AuthorizationRequired { command } => {
                format!(
                    "\"{command}\" requires authorization. Please grant the necessary permissions in System Settings."
                )
            },
            Self::NotAvailable => "This command is not available on your system.".to_string(),
        }
    }

    /// Returns true if the error is recoverable (can be retried).
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::ExecutionFailed { .. })
    }
}

/// Information about a confirmation dialog for destructive commands.
#[derive(Debug, Clone)]
pub struct ConfirmationDialog {
    /// The title of the confirmation dialog.
    pub title: String,
    /// The message/description to show in the dialog.
    pub message: String,
    /// The label for the confirm button.
    pub confirm_label: String,
    /// The label for the cancel button.
    pub cancel_label: String,
    /// Whether this action is destructive (affects button styling).
    pub is_destructive: bool,
}

impl ConfirmationDialog {
    /// Creates a confirmation dialog for the given system command.
    ///
    /// Returns `None` if the command does not require confirmation.
    #[must_use]
    pub fn for_command(command: &SystemCommand) -> Option<Self> {
        match command {
            SystemCommand::Restart => Some(Self {
                title: "Restart Your Mac?".to_string(),
                message: "All unsaved work will be lost. Are you sure you want to restart?"
                    .to_string(),
                confirm_label: "Restart".to_string(),
                cancel_label: "Cancel".to_string(),
                is_destructive: true,
            }),
            SystemCommand::ShutDown => Some(Self {
                title: "Shut Down Your Mac?".to_string(),
                message: "All unsaved work will be lost. Are you sure you want to shut down?"
                    .to_string(),
                confirm_label: "Shut Down".to_string(),
                cancel_label: "Cancel".to_string(),
                is_destructive: true,
            }),
            SystemCommand::LogOut => Some(Self {
                title: "Log Out?".to_string(),
                message: "All unsaved work will be lost. Are you sure you want to log out?"
                    .to_string(),
                confirm_label: "Log Out".to_string(),
                cancel_label: "Cancel".to_string(),
                is_destructive: true,
            }),
            SystemCommand::EmptyTrash => Some(Self {
                title: "Empty Trash?".to_string(),
                message: "This will permanently delete all items in the Trash. This action cannot be undone.".to_string(),
                confirm_label: "Empty Trash".to_string(),
                cancel_label: "Cancel".to_string(),
                is_destructive: true,
            }),
            // Commands that don't require confirmation
            SystemCommand::Sleep
            | SystemCommand::SleepDisplays
            | SystemCommand::LockScreen
            | SystemCommand::ScreenSaver
            | SystemCommand::ToggleAppearance => None,
        }
    }
}

impl SystemCommand {
    /// Executes the system command.
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be executed.
    pub fn execute(&self) -> Result<()> {
        debug!(command = self.id(), "executing system command");

        match self {
            Self::Sleep => {
                Command::new("pmset")
                    .arg("sleepnow")
                    .spawn()
                    .context("failed to execute sleep command")?;
            },

            Self::SleepDisplays => {
                Command::new("pmset")
                    .arg("displaysleepnow")
                    .spawn()
                    .context("failed to sleep displays")?;
            },

            Self::LockScreen => {
                // Use CGSession lock via osascript with System Events keystroke
                // This simulates Cmd+Ctrl+Q which locks the screen
                run_applescript(
                    r#"tell application "System Events" to keystroke "q" using {control down, command down}"#,
                )?;
            },

            Self::Restart => {
                run_applescript(r#"tell application "System Events" to restart"#)?;
            },

            Self::ShutDown => {
                run_applescript(r#"tell application "System Events" to shut down"#)?;
            },

            Self::LogOut => {
                run_applescript(r#"tell application "System Events" to log out"#)?;
            },

            Self::EmptyTrash => {
                run_applescript(r#"tell application "Finder" to empty trash"#)?;
            },

            Self::ScreenSaver => {
                Command::new("open")
                    .args(["-a", "ScreenSaverEngine"])
                    .spawn()
                    .context("failed to start screen saver")?;
            },

            Self::ToggleAppearance => {
                run_applescript(
                    r#"
                    tell application "System Events"
                        tell appearance preferences
                            set dark mode to not dark mode
                        end tell
                    end tell
                    "#,
                )?;
            },
        }

        debug!(command = self.id(), "system command executed successfully");
        Ok(())
    }

    /// Executes the system command asynchronously.
    ///
    /// This wraps the synchronous execute in a blocking task to avoid
    /// blocking the async runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be executed.
    pub async fn execute_async(&self) -> Result<()> {
        let command = *self;
        tokio::task::spawn_blocking(move || command.execute())
            .await
            .context("command execution task failed")?
    }

    /// Returns the confirmation dialog info if this command requires confirmation.
    #[must_use]
    pub fn confirmation_dialog(&self) -> Option<ConfirmationDialog> {
        ConfirmationDialog::for_command(self)
    }
}

/// Runs an AppleScript and returns the result.
///
/// # Arguments
///
/// * `script` - The AppleScript code to execute.
///
/// # Errors
///
/// Returns an error if the script fails to execute or returns an error.
///
/// # Example
///
/// ```ignore
/// let result = run_applescript(r#"tell application "Finder" to get name of startup disk"#);
/// ```
pub fn run_applescript(script: &str) -> Result<()> {
    debug!(script = script.trim(), "executing AppleScript");

    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .context("failed to run AppleScript")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        error!(
            stderr = %stderr,
            stdout = %stdout,
            exit_code = ?output.status.code(),
            "AppleScript execution failed"
        );

        bail!("AppleScript error: {}", stderr.trim());
    }

    debug!("AppleScript executed successfully");
    Ok(())
}

/// Runs an AppleScript and returns the output as a string.
///
/// # Arguments
///
/// * `script` - The AppleScript code to execute.
///
/// # Errors
///
/// Returns an error if the script fails to execute or returns an error.
pub fn run_applescript_with_output(script: &str) -> Result<String> {
    debug!(script = script.trim(), "executing AppleScript with output");

    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .context("failed to run AppleScript")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(
            stderr = %stderr,
            exit_code = ?output.status.code(),
            "AppleScript execution failed"
        );
        bail!("AppleScript error: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    debug!(output = %stdout, "AppleScript executed successfully");
    Ok(stdout)
}
