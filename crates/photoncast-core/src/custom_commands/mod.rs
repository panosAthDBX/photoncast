//! Custom Commands Module
//!
//! This module provides functionality for user-defined shell commands that appear in search.
//! Users can create commands with placeholders, aliases, and keywords for quick execution.
//!
//! # Features
//!
//! - Create, update, delete, and list custom commands
//! - Placeholder expansion ({query}, {clipboard}, {selection}, {env:VAR})
//! - Shell execution with timeout support
//! - Output capture and persistence
//! - Confirmation dialogs for destructive commands
//!
//! # Example
//!
//! ```ignore
//! use photoncast_core::custom_commands::{CustomCommand, CustomCommandStore, CommandExecutor};
//!
//! // Create a store
//! let store = CustomCommandStore::open_in_memory()?;
//!
//! // Create a command
//! let command = CustomCommand::new("Open in VS Code", "code {query}");
//! store.create(&command)?;
//!
//! // Execute with placeholder expansion
//! let executor = CommandExecutor::new();
//! let result = executor.execute(&command, "my-project").await?;
//! ```

pub mod executor;
pub mod placeholders;
pub mod store;

pub use executor::{CommandExecutionResult, CommandExecutor, ExecutorError};
pub use placeholders::{expand_placeholders, PlaceholderContext, PlaceholderError};
pub use store::{CustomCommandStore, StoreError};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Maximum size for captured output (64KB).
pub const MAX_OUTPUT_SIZE: usize = 64 * 1024;

/// Default timeout for command execution (30 seconds).
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Default shell for command execution.
pub const DEFAULT_SHELL: &str = "/bin/zsh";

/// A user-defined custom command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    /// Unique identifier.
    pub id: String,
    /// Display name of the command.
    pub name: String,
    /// The shell command to execute.
    pub command: String,
    /// Short alias for quick access (e.g., "gc" for "git commit").
    pub alias: Option<String>,
    /// Keywords for search matching.
    pub keywords: Vec<String>,
    /// Icon identifier (emoji or SF Symbol name).
    pub icon: Option<String>,
    /// Working directory for execution.
    pub working_directory: Option<String>,
    /// Environment variables to set during execution.
    pub environment: std::collections::HashMap<String, String>,
    /// Timeout in milliseconds (default: 30000).
    pub timeout_ms: u64,
    /// Shell to use for execution (default: /bin/zsh).
    pub shell: String,
    /// Whether to show a confirmation dialog before execution.
    pub requires_confirmation: bool,
    /// Whether to capture stdout/stderr.
    pub capture_output: bool,
    /// Whether the command is enabled.
    pub enabled: bool,
    /// Number of times the command has been executed.
    pub run_count: u32,
    /// Timestamp of last execution.
    pub last_run_at: Option<DateTime<Utc>>,
    /// Timestamp when the command was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the command was last updated.
    pub updated_at: DateTime<Utc>,
}

impl CustomCommand {
    /// Creates a new custom command with default settings.
    #[must_use]
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            command: command.into(),
            alias: None,
            keywords: Vec::new(),
            icon: None,
            working_directory: None,
            environment: std::collections::HashMap::new(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            shell: DEFAULT_SHELL.to_string(),
            requires_confirmation: false,
            capture_output: true,
            enabled: true,
            run_count: 0,
            last_run_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new command builder.
    #[must_use]
    pub fn builder(name: impl Into<String>, command: impl Into<String>) -> CustomCommandBuilder {
        CustomCommandBuilder::new(name, command)
    }

    /// Returns true if this command requires user input (has placeholders).
    #[must_use]
    pub fn requires_input(&self) -> bool {
        self.command.contains("{query}")
            || self.command.contains("{selection}")
            || self.command.contains("{clipboard}")
    }

    /// Returns the placeholder types used in this command.
    #[must_use]
    pub fn placeholder_types(&self) -> Vec<&'static str> {
        let mut types = Vec::new();
        if self.command.contains("{query}") {
            types.push("query");
        }
        if self.command.contains("{selection}") {
            types.push("selection");
        }
        if self.command.contains("{clipboard}") {
            types.push("clipboard");
        }
        if self.command.contains("{env:") {
            types.push("env");
        }
        types
    }
}

/// Builder for creating custom commands with fluent API.
#[derive(Debug)]
pub struct CustomCommandBuilder {
    command: CustomCommand,
}

impl CustomCommandBuilder {
    /// Creates a new builder with required fields.
    #[must_use]
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            command: CustomCommand::new(name, command),
        }
    }

    /// Sets the alias for quick access.
    #[must_use]
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.command.alias = Some(alias.into());
        self
    }

    /// Adds keywords for search matching.
    #[must_use]
    pub fn keywords(mut self, keywords: Vec<String>) -> Self {
        self.command.keywords = keywords;
        self
    }

    /// Sets the icon identifier.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.command.icon = Some(icon.into());
        self
    }

    /// Sets the working directory.
    #[must_use]
    pub fn working_directory(mut self, dir: impl Into<String>) -> Self {
        self.command.working_directory = Some(dir.into());
        self
    }

    /// Adds an environment variable.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.command.environment.insert(key.into(), value.into());
        self
    }

    /// Sets the timeout in milliseconds.
    #[must_use]
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.command.timeout_ms = timeout;
        self
    }

    /// Sets the shell to use.
    #[must_use]
    pub fn shell(mut self, shell: impl Into<String>) -> Self {
        self.command.shell = shell.into();
        self
    }

    /// Sets whether confirmation is required.
    #[must_use]
    pub fn requires_confirmation(mut self, requires: bool) -> Self {
        self.command.requires_confirmation = requires;
        self
    }

    /// Sets whether to capture output.
    #[must_use]
    pub fn capture_output(mut self, capture: bool) -> Self {
        self.command.capture_output = capture;
        self
    }

    /// Sets whether the command is enabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.command.enabled = enabled;
        self
    }

    /// Builds the custom command.
    #[must_use]
    pub fn build(self) -> CustomCommand {
        self.command
    }
}

/// Stored output from a command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    /// ID of the command that was executed.
    pub command_id: String,
    /// The full command string that was executed (after placeholder expansion).
    pub executed_command: String,
    /// Exit code (0 = success).
    pub exit_code: i32,
    /// Captured stdout (truncated to MAX_OUTPUT_SIZE).
    pub stdout: String,
    /// Captured stderr (truncated to MAX_OUTPUT_SIZE).
    pub stderr: String,
    /// Whether the output was truncated.
    pub truncated: bool,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Timestamp of execution.
    pub executed_at: DateTime<Utc>,
}

impl CommandOutput {
    /// Returns true if the command succeeded (exit code 0).
    #[must_use]
    pub const fn succeeded(&self) -> bool {
        self.exit_code == 0
    }

    /// Returns a human-readable summary of the execution result.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.succeeded() {
            format!("Completed in {}ms", self.duration_ms)
        } else {
            format!(
                "Failed (exit code {}) in {}ms",
                self.exit_code, self.duration_ms
            )
        }
    }
}

/// Confirmation dialog information for custom commands.
#[derive(Debug, Clone)]
pub struct CustomCommandConfirmation {
    /// The title of the confirmation dialog.
    pub title: String,
    /// The message to show.
    pub message: String,
    /// The command that will be executed (after placeholder expansion).
    pub command_preview: String,
    /// Label for the confirm button.
    pub confirm_label: String,
    /// Label for the cancel button.
    pub cancel_label: String,
}

impl CustomCommandConfirmation {
    /// Creates a confirmation dialog for a custom command.
    #[must_use]
    pub fn for_command(command: &CustomCommand, expanded_command: &str) -> Self {
        Self {
            title: format!("Run \"{}\"?", command.name),
            message: "This command will be executed in your terminal.".to_string(),
            command_preview: expanded_command.to_string(),
            confirm_label: "Run".to_string(),
            cancel_label: "Cancel".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_command_new() {
        let cmd = CustomCommand::new("Test", "echo hello");
        assert_eq!(cmd.name, "Test");
        assert_eq!(cmd.command, "echo hello");
        assert!(!cmd.id.is_empty());
        assert!(cmd.enabled);
        assert_eq!(cmd.run_count, 0);
        assert!(cmd.last_run_at.is_none());
    }

    #[test]
    fn test_custom_command_builder() {
        let cmd = CustomCommand::builder("Open in VS Code", "code {query}")
            .alias("vsc")
            .keywords(vec!["editor".to_string(), "ide".to_string()])
            .icon("terminal")
            .working_directory("/Users/test")
            .env("EDITOR", "code")
            .timeout_ms(60_000)
            .requires_confirmation(true)
            .build();

        assert_eq!(cmd.name, "Open in VS Code");
        assert_eq!(cmd.command, "code {query}");
        assert_eq!(cmd.alias, Some("vsc".to_string()));
        assert_eq!(cmd.keywords, vec!["editor", "ide"]);
        assert_eq!(cmd.icon, Some("terminal".to_string()));
        assert_eq!(cmd.working_directory, Some("/Users/test".to_string()));
        assert_eq!(cmd.environment.get("EDITOR"), Some(&"code".to_string()));
        assert_eq!(cmd.timeout_ms, 60_000);
        assert!(cmd.requires_confirmation);
    }

    #[test]
    fn test_requires_input() {
        let cmd1 = CustomCommand::new("Test", "echo hello");
        assert!(!cmd1.requires_input());

        let cmd2 = CustomCommand::new("Test", "code {query}");
        assert!(cmd2.requires_input());

        let cmd3 = CustomCommand::new("Test", "pbpaste | wc -c");
        assert!(!cmd3.requires_input());
    }

    #[test]
    fn test_placeholder_types() {
        let cmd = CustomCommand::new("Test", "echo {query} {clipboard} {env:PATH}");
        let types = cmd.placeholder_types();
        assert!(types.contains(&"query"));
        assert!(types.contains(&"clipboard"));
        assert!(types.contains(&"env"));
        assert!(!types.contains(&"selection"));
    }

    #[test]
    fn test_command_output_summary() {
        let output = CommandOutput {
            command_id: "test".to_string(),
            executed_command: "echo hello".to_string(),
            exit_code: 0,
            stdout: "hello\n".to_string(),
            stderr: String::new(),
            truncated: false,
            duration_ms: 100,
            executed_at: Utc::now(),
        };
        assert!(output.succeeded());
        assert_eq!(output.summary(), "Completed in 100ms");

        let failed_output = CommandOutput {
            exit_code: 1,
            ..output.clone()
        };
        assert!(!failed_output.succeeded());
        assert!(failed_output.summary().contains("exit code 1"));
    }
}
