//! System commands.
//!
//! This module defines and executes system commands like sleep, lock, restart, etc.
//!
//! # Overview
//!
//! PhotonCast provides built-in system commands that allow users to control their Mac
//! directly from the launcher. Commands include sleep, lock screen, restart, shutdown,
//! and more.
//!
//! # Example
//!
//! ```ignore
//! use photoncast_core::commands::{SystemCommand, CommandInfo, CommandExecutor};
//!
//! // Get all available commands
//! let commands = SystemCommand::all();
//!
//! // Execute a command
//! if let Err(e) = SystemCommand::Sleep.execute() {
//!     eprintln!("Failed to execute command: {}", e);
//! }
//!
//! // Check if a command requires confirmation
//! let cmd = SystemCommand::Restart;
//! if let Some(dialog) = cmd.confirmation_dialog() {
//!     println!("Confirm: {}", dialog.message);
//! }
//!
//! // Use the command executor for launcher integration
//! let executor = CommandExecutor::new();
//! if let Some(cmd) = executor.lookup("restart") {
//!     if cmd.requires_confirmation() {
//!         // Show confirmation dialog
//!     }
//!     executor.execute(cmd)?;
//! }
//! ```

pub mod definitions;
pub mod system;

#[cfg(test)]
mod tests;

pub use definitions::{CommandInfo, SystemCommand};
pub use system::{run_applescript, run_applescript_with_output, CommandError, ConfirmationDialog};

use anyhow::Result;
use std::collections::HashMap;

/// Trait for tracking command usage (for frecency calculations).
///
/// Implementations should persist usage data to a database or other storage.
/// This trait allows the command system to work with different storage backends.
pub trait CommandUsageTracker: Send + Sync {
    /// Records that a command was executed.
    ///
    /// # Arguments
    ///
    /// * `command_id` - The ID of the command that was executed.
    fn record_execution(&self, command_id: &str);

    /// Gets the execution count for a command.
    ///
    /// # Arguments
    ///
    /// * `command_id` - The ID of the command.
    ///
    /// # Returns
    ///
    /// The number of times the command has been executed.
    fn get_execution_count(&self, command_id: &str) -> u32;

    /// Gets the last execution timestamp for a command.
    ///
    /// # Arguments
    ///
    /// * `command_id` - The ID of the command.
    ///
    /// # Returns
    ///
    /// The Unix timestamp of the last execution, or `None` if never executed.
    fn get_last_execution(&self, command_id: &str) -> Option<i64>;
}

/// A no-op usage tracker that doesn't persist any data.
///
/// Useful for testing or when usage tracking is not needed.
#[derive(Debug, Default)]
pub struct NoOpUsageTracker;

impl CommandUsageTracker for NoOpUsageTracker {
    fn record_execution(&self, _command_id: &str) {
        // No-op
    }

    fn get_execution_count(&self, _command_id: &str) -> u32 {
        0
    }

    fn get_last_execution(&self, _command_id: &str) -> Option<i64> {
        None
    }
}

/// In-memory usage tracker for testing and development.
///
/// Stores usage data in memory (not persisted across restarts).
#[derive(Debug, Default)]
pub struct InMemoryUsageTracker {
    executions: parking_lot::RwLock<HashMap<String, (u32, i64)>>,
}

impl InMemoryUsageTracker {
    /// Creates a new in-memory usage tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl CommandUsageTracker for InMemoryUsageTracker {
    fn record_execution(&self, command_id: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut executions = self.executions.write();
        let entry = executions.entry(command_id.to_string()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 = now;
    }

    fn get_execution_count(&self, command_id: &str) -> u32 {
        self.executions
            .read()
            .get(command_id)
            .map_or(0, |(count, _)| *count)
    }

    fn get_last_execution(&self, command_id: &str) -> Option<i64> {
        self.executions.read().get(command_id).map(|(_, ts)| *ts)
    }
}

/// Command executor for launcher integration.
///
/// Provides a high-level interface for looking up commands by ID,
/// executing them, and tracking usage.
#[derive(Debug)]
pub struct CommandExecutor<T: CommandUsageTracker = NoOpUsageTracker> {
    /// Command lookup cache (ID -> SystemCommand).
    command_map: HashMap<String, SystemCommand>,
    /// Usage tracker for frecency calculations.
    usage_tracker: T,
}

impl Default for CommandExecutor<NoOpUsageTracker> {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor<NoOpUsageTracker> {
    /// Creates a new command executor without usage tracking.
    #[must_use]
    pub fn new() -> Self {
        Self::with_tracker(NoOpUsageTracker)
    }
}

impl<T: CommandUsageTracker> CommandExecutor<T> {
    /// Creates a new command executor with the specified usage tracker.
    #[must_use]
    pub fn with_tracker(usage_tracker: T) -> Self {
        // Build command lookup map
        let mut command_map = HashMap::new();
        for cmd_info in SystemCommand::all() {
            command_map.insert(cmd_info.command.id().to_string(), cmd_info.command);
        }

        Self {
            command_map,
            usage_tracker,
        }
    }

    /// Looks up a command by its ID.
    ///
    /// # Arguments
    ///
    /// * `command_id` - The command ID (e.g., "sleep", "restart").
    ///
    /// # Returns
    ///
    /// The `SystemCommand` if found, or `None` if not found.
    #[must_use]
    pub fn lookup(&self, command_id: &str) -> Option<SystemCommand> {
        self.command_map.get(command_id).copied()
    }

    /// Executes a command and records usage.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the command executed successfully, or an error if it failed.
    ///
    /// # Note
    ///
    /// This does NOT handle confirmation dialogs. The caller should check
    /// `command.requires_confirmation()` and show a confirmation dialog before
    /// calling this method.
    pub fn execute(&self, command: SystemCommand) -> Result<()> {
        // Execute the command
        command.execute()?;

        // Record usage for frecency
        self.usage_tracker.record_execution(command.id());

        Ok(())
    }

    /// Executes a command asynchronously and records usage.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the command executed successfully, or an error if it failed.
    pub async fn execute_async(&self, command: SystemCommand) -> Result<()> {
        // Execute the command
        command.execute_async().await?;

        // Record usage for frecency
        self.usage_tracker.record_execution(command.id());

        Ok(())
    }

    /// Executes a command by its ID and records usage.
    ///
    /// # Arguments
    ///
    /// * `command_id` - The ID of the command to execute.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the command executed successfully, an error if it failed,
    /// or `CommandNotFound` if the command ID is invalid.
    pub fn execute_by_id(&self, command_id: &str) -> Result<()> {
        let command = self
            .lookup(command_id)
            .ok_or_else(|| anyhow::anyhow!("command not found: {command_id}"))?;
        self.execute(command)
    }

    /// Gets the usage tracker.
    pub fn usage_tracker(&self) -> &T {
        &self.usage_tracker
    }
}
