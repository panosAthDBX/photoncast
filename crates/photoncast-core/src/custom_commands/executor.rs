//! Command execution pipeline for custom commands.
//!
//! This module handles executing shell commands with:
//! - Timeout enforcement
//! - Output capture (stdout/stderr)
//! - Working directory support
//! - Environment variable injection
//! - Shell wrapper execution

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use chrono::Utc;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use super::{
    expand_placeholders, CommandOutput, CustomCommand, PlaceholderContext, PlaceholderError,
    MAX_OUTPUT_SIZE,
};

/// Shell binaries explicitly allowed for custom command execution.
const ALLOWED_SHELLS: &[&str] = &["/bin/bash", "/bin/sh", "/bin/zsh"];

/// Errors that can occur during command execution.
#[derive(Error, Debug)]
pub enum ExecutorError {
    /// Command timed out.
    #[error("command timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Failed to spawn the command.
    #[error("failed to spawn command: {0}")]
    SpawnFailed(String),

    /// Failed to wait for command completion.
    #[error("failed to wait for command: {0}")]
    WaitFailed(String),

    /// Placeholder expansion failed.
    #[error("placeholder expansion failed: {0}")]
    PlaceholderError(#[from] PlaceholderError),

    /// Working directory does not exist.
    #[error("working directory does not exist: {path}")]
    WorkingDirectoryNotFound { path: String },

    /// Shell not found.
    #[error("shell not found: {shell}")]
    ShellNotFound { shell: String },

    /// Shell path is not in the allowlist.
    #[error("unsupported shell: {shell}")]
    UnsupportedShell { shell: String },

    /// Command execution failed.
    #[error("command failed with exit code {exit_code}: {message}")]
    ExecutionFailed { exit_code: i32, message: String },
}

impl ExecutorError {
    /// Returns a user-friendly message for the error.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Timeout { timeout_ms } => {
                format!(
                    "Command timed out after {} seconds. Try increasing the timeout.",
                    timeout_ms / 1000
                )
            },
            Self::SpawnFailed(msg) => {
                format!(
                    "Failed to start command: {msg}. Check if the shell and command are correct."
                )
            },
            Self::WaitFailed(msg) => {
                format!("Command execution interrupted: {msg}")
            },
            Self::PlaceholderError(e) => {
                format!("Failed to expand placeholders: {e}")
            },
            Self::WorkingDirectoryNotFound { path } => {
                format!("Working directory not found: {path}")
            },
            Self::ShellNotFound { shell } => {
                format!("Shell not found: {shell}. Check your shell configuration.")
            },
            Self::UnsupportedShell { shell } => {
                format!(
                    "Shell {shell} is not allowed for custom commands. Use /bin/bash, /bin/sh, or /bin/zsh."
                )
            },
            Self::ExecutionFailed { exit_code, message } => {
                format!("Command failed (exit code {exit_code}): {message}")
            },
        }
    }

    /// Returns true if the error is recoverable (can be retried).
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::Timeout { .. } | Self::ExecutionFailed { .. })
    }
}

/// Result of a command execution.
#[derive(Debug, Clone)]
pub struct CommandExecutionResult {
    /// The command that was executed.
    pub command_id: String,
    /// The expanded command string.
    pub executed_command: String,
    /// Exit code from the command.
    pub exit_code: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Whether output was truncated.
    pub truncated: bool,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Whether the command succeeded.
    pub success: bool,
}

impl CommandExecutionResult {
    /// Converts this result to a `CommandOutput` for storage.
    #[must_use]
    pub fn to_output(&self) -> CommandOutput {
        CommandOutput {
            command_id: self.command_id.clone(),
            executed_command: self.executed_command.clone(),
            exit_code: self.exit_code,
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            truncated: self.truncated,
            duration_ms: self.duration_ms,
            executed_at: Utc::now(),
        }
    }
}

/// Executor for running custom commands.
#[derive(Debug, Default)]
pub struct CommandExecutor {
    /// Default environment variables to include in all executions.
    default_env: HashMap<String, String>,
}

impl CommandExecutor {
    /// Creates a new command executor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            default_env: HashMap::new(),
        }
    }

    /// Creates a new executor with default environment variables.
    #[must_use]
    pub fn with_default_env(env: HashMap<String, String>) -> Self {
        Self { default_env: env }
    }

    /// Executes a custom command with the given context.
    ///
    /// # Arguments
    ///
    /// * `command` - The custom command to execute.
    /// * `context` - The placeholder context for expansion.
    ///
    /// # Returns
    ///
    /// The execution result, or an error if execution failed.
    pub async fn execute(
        &self,
        command: &CustomCommand,
        context: &PlaceholderContext,
    ) -> Result<CommandExecutionResult, ExecutorError> {
        info!(
            command_id = %command.id,
            command_name = %command.name,
            "Executing custom command"
        );

        // Expand placeholders
        let expanded_command = expand_placeholders(&command.command, context)?;
        debug!(
            original = %command.command,
            expanded = %expanded_command,
            "Placeholders expanded"
        );

        // Validate working directory
        if let Some(ref work_dir) = command.working_directory {
            let path = Path::new(work_dir);
            if !path.exists() {
                return Err(ExecutorError::WorkingDirectoryNotFound {
                    path: work_dir.clone(),
                });
            }
        }

        // Validate shell
        let shell = &command.shell;
        if !Path::new(shell).exists() {
            return Err(ExecutorError::ShellNotFound {
                shell: shell.clone(),
            });
        }
        if !is_allowed_shell(shell) {
            return Err(ExecutorError::UnsupportedShell {
                shell: shell.clone(),
            });
        }

        // Build the command
        let mut cmd = Command::new(shell);
        cmd.args(["-c", &expanded_command]);

        // Set working directory
        if let Some(ref work_dir) = command.working_directory {
            cmd.current_dir(work_dir);
        }

        // Set environment variables
        cmd.env_clear();
        // Start with system PATH and common variables
        for (key, value) in std::env::vars() {
            if key == "PATH" || key == "HOME" || key == "USER" || key == "SHELL" || key == "TERM" {
                cmd.env(&key, &value);
            }
        }
        // Add default environment
        for (key, value) in &self.default_env {
            cmd.env(key, value);
        }
        // Add command-specific environment
        for (key, value) in &command.environment {
            cmd.env(key, value);
        }

        // Capture output if requested
        if command.capture_output {
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
        } else {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        // Execute with timeout
        let start = Instant::now();
        let timeout_duration = Duration::from_millis(command.timeout_ms);

        let result = timeout(timeout_duration, async {
            let child = cmd
                .spawn()
                .map_err(|e| ExecutorError::SpawnFailed(e.to_string()))?;

            child
                .wait_with_output()
                .await
                .map_err(|e| ExecutorError::WaitFailed(e.to_string()))
        })
        .await;

        #[allow(clippy::cast_possible_truncation)]
        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

                // Process stdout
                let (stdout, stdout_truncated) = truncate_output(&output.stdout);
                let (stderr, stderr_truncated) = truncate_output(&output.stderr);
                let truncated = stdout_truncated || stderr_truncated;

                if success {
                    info!(
                        command_id = %command.id,
                        exit_code,
                        duration_ms,
                        "Command completed successfully"
                    );
                } else {
                    warn!(
                        command_id = %command.id,
                        exit_code,
                        stderr = %stderr.chars().take(200).collect::<String>(),
                        duration_ms,
                        "Command failed"
                    );
                }

                Ok(CommandExecutionResult {
                    command_id: command.id.clone(),
                    executed_command: expanded_command,
                    exit_code,
                    stdout,
                    stderr,
                    truncated,
                    duration_ms,
                    success,
                })
            },
            Ok(Err(e)) => {
                error!(
                    command_id = %command.id,
                    error = %e,
                    duration_ms,
                    "Command execution failed"
                );
                Err(e)
            },
            Err(_) => {
                error!(
                    command_id = %command.id,
                    timeout_ms = command.timeout_ms,
                    "Command timed out"
                );
                Err(ExecutorError::Timeout {
                    timeout_ms: command.timeout_ms,
                })
            },
        }
    }

    /// Executes a command with just a query string for convenience.
    pub async fn execute_with_query(
        &self,
        command: &CustomCommand,
        query: &str,
    ) -> Result<CommandExecutionResult, ExecutorError> {
        let context = PlaceholderContext::with_query(query);
        self.execute(command, &context).await
    }

    /// Validates that a command can be executed (checks shell, working dir, etc).
    ///
    /// Returns `Ok(())` if the command appears valid, or an error describing the issue.
    pub fn validate(&self, command: &CustomCommand) -> Result<(), ExecutorError> {
        // Check shell exists
        if !Path::new(&command.shell).exists() {
            return Err(ExecutorError::ShellNotFound {
                shell: command.shell.clone(),
            });
        }
        if !is_allowed_shell(&command.shell) {
            return Err(ExecutorError::UnsupportedShell {
                shell: command.shell.clone(),
            });
        }

        // Check working directory if specified
        if let Some(ref work_dir) = command.working_directory {
            if !Path::new(work_dir).exists() {
                return Err(ExecutorError::WorkingDirectoryNotFound {
                    path: work_dir.clone(),
                });
            }
        }

        Ok(())
    }
}

fn is_allowed_shell(shell: &str) -> bool {
    ALLOWED_SHELLS.contains(&shell)
}

/// Truncates output to the maximum size and converts to UTF-8.
///
/// Returns the truncated string and whether truncation occurred.
fn truncate_output(data: &[u8]) -> (String, bool) {
    let truncated = data.len() > MAX_OUTPUT_SIZE;
    let bytes = if truncated {
        &data[..MAX_OUTPUT_SIZE]
    } else {
        data
    };

    // Convert to UTF-8, replacing invalid sequences
    let text = String::from_utf8_lossy(bytes).to_string();

    // If truncated, add indicator
    if truncated {
        (format!("{text}... (truncated)"), true)
    } else {
        (text, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_simple_command() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::new("Echo Test", "echo hello");

        let result = executor
            .execute_with_query(&command, "")
            .await
            .expect("should execute");

        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_with_query_placeholder() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::new("Echo Query", "echo {query}");

        let result = executor
            .execute_with_query(&command, "world")
            .await
            .expect("should execute");

        assert!(result.success);
        assert!(result.stdout.contains("world"));
    }

    #[tokio::test]
    async fn test_execute_failing_command() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::new("Fail", "exit 1");

        let result = executor
            .execute_with_query(&command, "")
            .await
            .expect("should execute");

        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::builder("Sleep", "sleep 10")
            .timeout_ms(100)
            .build();

        let result = executor.execute_with_query(&command, "").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExecutorError::Timeout { .. }));
    }

    #[tokio::test]
    async fn test_execute_with_working_directory() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::builder("PWD", "pwd")
            .working_directory("/tmp")
            .build();

        let result = executor
            .execute_with_query(&command, "")
            .await
            .expect("should execute");

        assert!(result.success);
        // On macOS, /tmp is a symlink to /private/tmp
        assert!(result.stdout.contains("/tmp") || result.stdout.contains("/private/tmp"));
    }

    #[tokio::test]
    async fn test_execute_with_env() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::builder("Env Test", "echo $TEST_VAR")
            .env("TEST_VAR", "test_value")
            .build();

        let result = executor
            .execute_with_query(&command, "")
            .await
            .expect("should execute");

        assert!(result.success);
        assert!(result.stdout.contains("test_value"));
    }

    #[tokio::test]
    async fn test_execute_with_unsupported_shell_fails_closed() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::builder("Unsupported Shell", "echo test")
            .shell("/usr/bin/env")
            .build();

        let result = executor.execute_with_query(&command, "").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExecutorError::UnsupportedShell { .. }
        ));
    }

    #[tokio::test]
    async fn test_execute_with_stderr() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::new("Stderr Test", "echo error >&2");

        let result = executor
            .execute_with_query(&command, "")
            .await
            .expect("should execute");

        assert!(result.success);
        assert!(result.stderr.contains("error"));
    }

    #[test]
    fn test_validate_nonexistent_shell() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::builder("Test", "echo")
            .shell("/nonexistent/shell")
            .build();

        let result = executor.validate(&command);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExecutorError::ShellNotFound { .. }
        ));
    }

    #[test]
    fn test_validate_unsupported_shell() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::builder("Test", "echo")
            .shell("/usr/bin/env")
            .build();

        let result = executor.validate(&command);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExecutorError::UnsupportedShell { .. }
        ));
    }

    #[test]
    fn test_validate_nonexistent_workdir() {
        let executor = CommandExecutor::new();
        let command = CustomCommand::builder("Test", "echo")
            .working_directory("/nonexistent/directory")
            .build();

        let result = executor.validate(&command);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ExecutorError::WorkingDirectoryNotFound { .. }
        ));
    }

    #[test]
    fn test_truncate_output() {
        let small_data = b"hello world";
        let (text, truncated) = truncate_output(small_data);
        assert_eq!(text, "hello world");
        assert!(!truncated);

        // Create data larger than MAX_OUTPUT_SIZE
        let large_data = vec![b'x'; MAX_OUTPUT_SIZE + 100];
        let (text, truncated) = truncate_output(&large_data);
        assert!(truncated);
        assert!(text.ends_with("... (truncated)"));
        assert!(text.len() < large_data.len());
    }

    #[test]
    fn test_execution_result_to_output() {
        let result = CommandExecutionResult {
            command_id: "test-id".to_string(),
            executed_command: "echo hello".to_string(),
            exit_code: 0,
            stdout: "hello\n".to_string(),
            stderr: String::new(),
            truncated: false,
            duration_ms: 100,
            success: true,
        };

        let output = result.to_output();
        assert_eq!(output.command_id, "test-id");
        assert_eq!(output.exit_code, 0);
        assert!(output.succeeded());
    }
}
