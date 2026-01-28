//! SQLite storage for custom commands.
//!
//! This module provides CRUD operations for custom commands with SQLite backend.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use tokio::task;
use tracing::{debug, info};

use super::{CommandOutput, CustomCommand, MAX_OUTPUT_SIZE};
use crate::utils::paths;

/// Errors that can occur with the custom command store.
#[derive(Error, Debug)]
pub enum StoreError {
    /// SQLite error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Command not found.
    #[error("command not found: {id}")]
    NotFound { id: String },

    /// Duplicate alias.
    #[error("alias '{alias}' is already in use")]
    DuplicateAlias { alias: String },

    /// Task join error.
    #[error("async task failed: {0}")]
    TaskFailed(String),
}

impl From<tokio::task::JoinError> for StoreError {
    fn from(e: tokio::task::JoinError) -> Self {
        Self::TaskFailed(e.to_string())
    }
}

/// SQLite-backed storage for custom commands.
#[derive(Debug)]
pub struct CustomCommandStore {
    conn: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

impl Clone for CustomCommandStore {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            path: self.path.clone(),
        }
    }
}

impl CustomCommandStore {
    /// Opens or creates a custom command store at the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent access
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: Some(path.to_path_buf()),
        };

        store.initialize_schema()?;

        info!(path = ?path, "Custom commands store opened");
        Ok(store)
    }

    /// Opens the default custom commands store.
    pub fn open_default() -> Result<Self, StoreError> {
        let path = default_custom_commands_db_path();
        Self::open(path)
    }

    /// Opens an in-memory store (for testing).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: None,
        };

        store.initialize_schema()?;

        debug!("Custom commands in-memory store opened");
        Ok(store)
    }

    /// Initializes the database schema.
    fn initialize_schema(&self) -> Result<(), StoreError> {
        let conn = self.conn.lock();

        conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS custom_commands (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                alias TEXT,
                keywords TEXT,
                icon TEXT,
                working_directory TEXT,
                environment TEXT,
                timeout_ms INTEGER DEFAULT 30000,
                shell TEXT DEFAULT '/bin/zsh',
                requires_confirmation INTEGER DEFAULT 0,
                capture_output INTEGER DEFAULT 1,
                enabled INTEGER DEFAULT 1,
                run_count INTEGER DEFAULT 0,
                last_run_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_commands_alias
            ON custom_commands(alias) WHERE alias IS NOT NULL;

            CREATE INDEX IF NOT EXISTS idx_custom_commands_enabled
            ON custom_commands(enabled);

            CREATE TABLE IF NOT EXISTS command_outputs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                command_id TEXT NOT NULL,
                executed_command TEXT NOT NULL,
                exit_code INTEGER NOT NULL,
                stdout TEXT,
                stderr TEXT,
                truncated INTEGER DEFAULT 0,
                duration_ms INTEGER NOT NULL,
                executed_at INTEGER NOT NULL,
                FOREIGN KEY (command_id) REFERENCES custom_commands(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_command_outputs_command_id
            ON command_outputs(command_id);

            CREATE INDEX IF NOT EXISTS idx_command_outputs_executed_at
            ON command_outputs(executed_at DESC);
            ",
        )?;

        debug!("Custom commands schema initialized");
        Ok(())
    }

    // =========================================================================
    // CRUD Operations
    // =========================================================================

    /// Creates a new custom command.
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be created or the alias is already in use.
    pub fn create(&self, command: &CustomCommand) -> Result<(), StoreError> {
        // Check for duplicate alias
        if let Some(ref alias) = command.alias {
            if self.alias_exists(alias, Some(&command.id))? {
                return Err(StoreError::DuplicateAlias {
                    alias: alias.clone(),
                });
            }
        }

        let conn = self.conn.lock();
        let keywords_json = serde_json::to_string(&command.keywords)?;
        let env_json = serde_json::to_string(&command.environment)?;

        conn.execute(
            r"
            INSERT INTO custom_commands (
                id, name, command, alias, keywords, icon,
                working_directory, environment, timeout_ms, shell,
                requires_confirmation, capture_output, enabled,
                run_count, last_run_at, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
            )
            ",
            params![
                &command.id,
                &command.name,
                &command.command,
                &command.alias,
                keywords_json,
                &command.icon,
                &command.working_directory,
                env_json,
                command.timeout_ms as i64,
                &command.shell,
                command.requires_confirmation,
                command.capture_output,
                command.enabled,
                command.run_count,
                command.last_run_at.map(|dt| dt.timestamp()),
                command.created_at.timestamp(),
                command.updated_at.timestamp(),
            ],
        )?;

        info!(command_id = %command.id, name = %command.name, "Created custom command");
        Ok(())
    }

    /// Creates a command asynchronously.
    pub async fn create_async(&self, command: CustomCommand) -> Result<(), StoreError> {
        let store = self.clone();
        task::spawn_blocking(move || store.create(&command)).await?
    }

    /// Updates an existing custom command.
    ///
    /// # Errors
    ///
    /// Returns an error if the command doesn't exist or the update fails.
    pub fn update(&self, command: &CustomCommand) -> Result<(), StoreError> {
        // Check command exists
        if !self.exists(&command.id)? {
            return Err(StoreError::NotFound {
                id: command.id.clone(),
            });
        }

        // Check for duplicate alias
        if let Some(ref alias) = command.alias {
            if self.alias_exists(alias, Some(&command.id))? {
                return Err(StoreError::DuplicateAlias {
                    alias: alias.clone(),
                });
            }
        }

        let conn = self.conn.lock();
        let keywords_json = serde_json::to_string(&command.keywords)?;
        let env_json = serde_json::to_string(&command.environment)?;
        let now = Utc::now().timestamp();

        conn.execute(
            r"
            UPDATE custom_commands SET
                name = ?2,
                command = ?3,
                alias = ?4,
                keywords = ?5,
                icon = ?6,
                working_directory = ?7,
                environment = ?8,
                timeout_ms = ?9,
                shell = ?10,
                requires_confirmation = ?11,
                capture_output = ?12,
                enabled = ?13,
                updated_at = ?14
            WHERE id = ?1
            ",
            params![
                &command.id,
                &command.name,
                &command.command,
                &command.alias,
                keywords_json,
                &command.icon,
                &command.working_directory,
                env_json,
                command.timeout_ms as i64,
                &command.shell,
                command.requires_confirmation,
                command.capture_output,
                command.enabled,
                now,
            ],
        )?;

        debug!(command_id = %command.id, "Updated custom command");
        Ok(())
    }

    /// Updates a command asynchronously.
    pub async fn update_async(&self, command: CustomCommand) -> Result<(), StoreError> {
        let store = self.clone();
        task::spawn_blocking(move || store.update(&command)).await?
    }

    /// Deletes a custom command.
    ///
    /// # Errors
    ///
    /// Returns an error if the command doesn't exist or deletion fails.
    pub fn delete(&self, id: &str) -> Result<(), StoreError> {
        let conn = self.conn.lock();

        let rows_affected = conn.execute("DELETE FROM custom_commands WHERE id = ?1", [id])?;

        if rows_affected == 0 {
            return Err(StoreError::NotFound { id: id.to_string() });
        }

        info!(command_id = %id, "Deleted custom command");
        Ok(())
    }

    /// Deletes a command asynchronously.
    pub async fn delete_async(&self, id: String) -> Result<(), StoreError> {
        let store = self.clone();
        task::spawn_blocking(move || store.delete(&id)).await?
    }

    /// Gets a custom command by ID.
    pub fn get(&self, id: &str) -> Result<Option<CustomCommand>, StoreError> {
        let conn = self.conn.lock();

        let result = conn
            .query_row(
                r"
                SELECT id, name, command, alias, keywords, icon,
                       working_directory, environment, timeout_ms, shell,
                       requires_confirmation, capture_output, enabled,
                       run_count, last_run_at, created_at, updated_at
                FROM custom_commands
                WHERE id = ?1
                ",
                [id],
                |row| self.row_to_command(row),
            )
            .optional()?;

        Ok(result)
    }

    /// Gets a command asynchronously.
    pub async fn get_async(&self, id: String) -> Result<Option<CustomCommand>, StoreError> {
        let store = self.clone();
        task::spawn_blocking(move || store.get(&id)).await?
    }

    /// Gets a command by its alias.
    pub fn get_by_alias(&self, alias: &str) -> Result<Option<CustomCommand>, StoreError> {
        let conn = self.conn.lock();

        let result = conn
            .query_row(
                r"
                SELECT id, name, command, alias, keywords, icon,
                       working_directory, environment, timeout_ms, shell,
                       requires_confirmation, capture_output, enabled,
                       run_count, last_run_at, created_at, updated_at
                FROM custom_commands
                WHERE alias = ?1 AND enabled = 1
                ",
                [alias],
                |row| self.row_to_command(row),
            )
            .optional()?;

        Ok(result)
    }

    /// Lists all custom commands.
    pub fn list(&self) -> Result<Vec<CustomCommand>, StoreError> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r"
            SELECT id, name, command, alias, keywords, icon,
                   working_directory, environment, timeout_ms, shell,
                   requires_confirmation, capture_output, enabled,
                   run_count, last_run_at, created_at, updated_at
            FROM custom_commands
            ORDER BY name ASC
            ",
        )?;

        let commands = stmt
            .query_map([], |row| self.row_to_command(row))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commands)
    }

    /// Lists all commands asynchronously.
    pub async fn list_async(&self) -> Result<Vec<CustomCommand>, StoreError> {
        let store = self.clone();
        task::spawn_blocking(move || store.list()).await?
    }

    /// Lists only enabled custom commands.
    pub fn list_enabled(&self) -> Result<Vec<CustomCommand>, StoreError> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r"
            SELECT id, name, command, alias, keywords, icon,
                   working_directory, environment, timeout_ms, shell,
                   requires_confirmation, capture_output, enabled,
                   run_count, last_run_at, created_at, updated_at
            FROM custom_commands
            WHERE enabled = 1
            ORDER BY name ASC
            ",
        )?;

        let commands = stmt
            .query_map([], |row| self.row_to_command(row))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commands)
    }

    // =========================================================================
    // Usage Tracking
    // =========================================================================

    /// Records that a command was executed.
    ///
    /// Updates `run_count` and `last_run_at`.
    pub fn record_execution(&self, id: &str) -> Result<(), StoreError> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();

        let rows_affected = conn.execute(
            r"
            UPDATE custom_commands SET
                run_count = run_count + 1,
                last_run_at = ?2
            WHERE id = ?1
            ",
            params![id, now],
        )?;

        if rows_affected == 0 {
            return Err(StoreError::NotFound { id: id.to_string() });
        }

        debug!(command_id = %id, "Recorded command execution");
        Ok(())
    }

    /// Records execution asynchronously.
    pub async fn record_execution_async(&self, id: String) -> Result<(), StoreError> {
        let store = self.clone();
        task::spawn_blocking(move || store.record_execution(&id)).await?
    }

    // =========================================================================
    // Output Storage (Task 7.8)
    // =========================================================================

    /// Stores command output.
    pub fn store_output(&self, output: &CommandOutput) -> Result<i64, StoreError> {
        let conn = self.conn.lock();

        // Truncate stdout/stderr if needed
        let stdout = truncate_string(&output.stdout, MAX_OUTPUT_SIZE);
        let stderr = truncate_string(&output.stderr, MAX_OUTPUT_SIZE);

        conn.execute(
            r"
            INSERT INTO command_outputs (
                command_id, executed_command, exit_code,
                stdout, stderr, truncated, duration_ms, executed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                &output.command_id,
                &output.executed_command,
                output.exit_code,
                stdout,
                stderr,
                output.truncated
                    || stdout.len() < output.stdout.len()
                    || stderr.len() < output.stderr.len(),
                output.duration_ms as i64,
                output.executed_at.timestamp(),
            ],
        )?;

        let id = conn.last_insert_rowid();
        debug!(output_id = id, command_id = %output.command_id, "Stored command output");
        Ok(id)
    }

    /// Gets the most recent outputs for a command.
    pub fn get_outputs(
        &self,
        command_id: &str,
        limit: usize,
    ) -> Result<Vec<CommandOutput>, StoreError> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r"
            SELECT command_id, executed_command, exit_code,
                   stdout, stderr, truncated, duration_ms, executed_at
            FROM command_outputs
            WHERE command_id = ?1
            ORDER BY executed_at DESC
            LIMIT ?2
            ",
        )?;

        let outputs = stmt
            .query_map(params![command_id, limit as i64], |row| {
                Ok(CommandOutput {
                    command_id: row.get(0)?,
                    executed_command: row.get(1)?,
                    exit_code: row.get(2)?,
                    stdout: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    stderr: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    truncated: row.get::<_, i32>(5)? != 0,
                    duration_ms: row.get::<_, i64>(6)? as u64,
                    executed_at: DateTime::from_timestamp(row.get(7)?, 0).unwrap_or_else(Utc::now),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(outputs)
    }

    /// Cleans up old outputs, keeping only the most recent N outputs per command.
    pub fn cleanup_outputs(&self, keep_per_command: usize) -> Result<usize, StoreError> {
        let conn = self.conn.lock();

        // Delete outputs that aren't in the top N for each command
        let deleted = conn.execute(
            r"
            DELETE FROM command_outputs
            WHERE id NOT IN (
                SELECT id FROM (
                    SELECT id, ROW_NUMBER() OVER (
                        PARTITION BY command_id
                        ORDER BY executed_at DESC
                    ) as rn
                    FROM command_outputs
                ) WHERE rn <= ?1
            )
            ",
            [keep_per_command as i64],
        )?;

        if deleted > 0 {
            debug!(deleted, "Cleaned up old command outputs");
        }
        Ok(deleted)
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Checks if a command exists.
    fn exists(&self, id: &str) -> Result<bool, StoreError> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM custom_commands WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Checks if an alias is already in use (excluding a specific command ID).
    #[allow(clippy::unused_self)]
    fn alias_exists(&self, alias: &str, exclude_id: Option<&str>) -> Result<bool, StoreError> {
        let conn = self.conn.lock();

        let count: i64 = match exclude_id {
            Some(id) => conn.query_row(
                "SELECT COUNT(*) FROM custom_commands WHERE alias = ?1 AND id != ?2",
                params![alias, id],
                |row| row.get(0),
            )?,
            None => conn.query_row(
                "SELECT COUNT(*) FROM custom_commands WHERE alias = ?1",
                [alias],
                |row| row.get(0),
            )?,
        };

        Ok(count > 0)
    }

    /// Converts a database row to a CustomCommand.
    #[allow(clippy::cast_possible_truncation, clippy::unused_self)]
    fn row_to_command(&self, row: &rusqlite::Row<'_>) -> Result<CustomCommand, rusqlite::Error> {
        let keywords_json: Option<String> = row.get(4)?;
        let env_json: Option<String> = row.get(7)?;

        let keywords: Vec<String> = keywords_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let environment: std::collections::HashMap<String, String> = env_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let last_run_at: Option<i64> = row.get(14)?;
        let created_at: i64 = row.get(15)?;
        let updated_at: i64 = row.get(16)?;

        Ok(CustomCommand {
            id: row.get(0)?,
            name: row.get(1)?,
            command: row.get(2)?,
            alias: row.get(3)?,
            keywords,
            icon: row.get(5)?,
            working_directory: row.get(6)?,
            environment,
            timeout_ms: row.get::<_, i64>(8)? as u64,
            shell: row.get(9)?,
            requires_confirmation: row.get::<_, i32>(10)? != 0,
            capture_output: row.get::<_, i32>(11)? != 0,
            enabled: row.get::<_, i32>(12)? != 0,
            run_count: row.get::<_, i64>(13)? as u32,
            last_run_at: last_run_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
            updated_at: DateTime::from_timestamp(updated_at, 0).unwrap_or_else(Utc::now),
        })
    }

    /// Returns the count of stored commands.
    #[allow(clippy::cast_possible_truncation)]
    pub fn count(&self) -> Result<usize, StoreError> {
        let conn = self.conn.lock();
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM custom_commands", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

/// Returns the default path for the custom commands database.
#[must_use]
pub fn default_custom_commands_db_path() -> PathBuf {
    paths::data_dir().join("custom_commands.db")
}

/// Truncates a string to the specified maximum length.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{truncated}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> CustomCommandStore {
        CustomCommandStore::open_in_memory().expect("should open in-memory store")
    }

    #[test]
    fn test_create_and_get() {
        let store = create_test_store();
        let cmd = CustomCommand::new("Test", "echo hello");

        store.create(&cmd).expect("should create");

        let retrieved = store
            .get(&cmd.id)
            .expect("should get")
            .expect("should exist");
        assert_eq!(retrieved.name, "Test");
        assert_eq!(retrieved.command, "echo hello");
    }

    #[test]
    fn test_update() {
        let store = create_test_store();
        let mut cmd = CustomCommand::new("Test", "echo hello");

        store.create(&cmd).expect("should create");

        cmd.name = "Updated Test".to_string();
        cmd.command = "echo world".to_string();

        store.update(&cmd).expect("should update");

        let retrieved = store
            .get(&cmd.id)
            .expect("should get")
            .expect("should exist");
        assert_eq!(retrieved.name, "Updated Test");
        assert_eq!(retrieved.command, "echo world");
    }

    #[test]
    fn test_delete() {
        let store = create_test_store();
        let cmd = CustomCommand::new("Test", "echo hello");

        store.create(&cmd).expect("should create");
        store.delete(&cmd.id).expect("should delete");

        let retrieved = store.get(&cmd.id).expect("should query");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list() {
        let store = create_test_store();

        store
            .create(&CustomCommand::new("Alpha", "echo a"))
            .unwrap();
        store.create(&CustomCommand::new("Beta", "echo b")).unwrap();
        store
            .create(&CustomCommand::new("Gamma", "echo c"))
            .unwrap();

        let commands = store.list().expect("should list");
        assert_eq!(commands.len(), 3);
        // Should be sorted by name
        assert_eq!(commands[0].name, "Alpha");
        assert_eq!(commands[1].name, "Beta");
        assert_eq!(commands[2].name, "Gamma");
    }

    #[test]
    fn test_list_enabled() {
        let store = create_test_store();

        let mut enabled = CustomCommand::new("Enabled", "echo e");
        enabled.enabled = true;

        let mut disabled = CustomCommand::new("Disabled", "echo d");
        disabled.enabled = false;

        store.create(&enabled).unwrap();
        store.create(&disabled).unwrap();

        let commands = store.list_enabled().expect("should list enabled");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "Enabled");
    }

    #[test]
    fn test_alias_unique() {
        let store = create_test_store();

        let cmd1 = CustomCommand::builder("Test 1", "echo 1")
            .alias("t")
            .build();
        let cmd2 = CustomCommand::builder("Test 2", "echo 2")
            .alias("t")
            .build();

        store.create(&cmd1).expect("should create first");
        let result = store.create(&cmd2);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StoreError::DuplicateAlias { .. }
        ));
    }

    #[test]
    fn test_get_by_alias() {
        let store = create_test_store();

        let cmd = CustomCommand::builder("Test", "echo hello")
            .alias("th")
            .build();
        store.create(&cmd).expect("should create");

        let retrieved = store
            .get_by_alias("th")
            .expect("should get")
            .expect("should exist");
        assert_eq!(retrieved.name, "Test");
    }

    #[test]
    fn test_record_execution() {
        let store = create_test_store();
        let cmd = CustomCommand::new("Test", "echo hello");

        store.create(&cmd).expect("should create");

        store.record_execution(&cmd.id).expect("should record");
        store
            .record_execution(&cmd.id)
            .expect("should record again");

        let retrieved = store
            .get(&cmd.id)
            .expect("should get")
            .expect("should exist");
        assert_eq!(retrieved.run_count, 2);
        assert!(retrieved.last_run_at.is_some());
    }

    #[test]
    fn test_store_and_get_output() {
        let store = create_test_store();
        let cmd = CustomCommand::new("Test", "echo hello");
        store.create(&cmd).expect("should create");

        let output = CommandOutput {
            command_id: cmd.id.clone(),
            executed_command: "echo hello".to_string(),
            exit_code: 0,
            stdout: "hello\n".to_string(),
            stderr: String::new(),
            truncated: false,
            duration_ms: 100,
            executed_at: Utc::now(),
        };

        store.store_output(&output).expect("should store");

        let outputs = store.get_outputs(&cmd.id, 10).expect("should get outputs");
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].exit_code, 0);
        assert!(outputs[0].stdout.contains("hello"));
    }

    #[test]
    fn test_cleanup_outputs() {
        let store = create_test_store();
        let cmd = CustomCommand::new("Test", "echo hello");
        store.create(&cmd).expect("should create");

        // Create 5 outputs
        for i in 0..5 {
            let output = CommandOutput {
                command_id: cmd.id.clone(),
                executed_command: format!("echo {i}"),
                exit_code: 0,
                stdout: format!("output {i}\n"),
                stderr: String::new(),
                truncated: false,
                duration_ms: 100,
                executed_at: Utc::now(),
            };
            store.store_output(&output).expect("should store");
        }

        // Keep only 2
        let deleted = store.cleanup_outputs(2).expect("should cleanup");
        assert_eq!(deleted, 3);

        let outputs = store.get_outputs(&cmd.id, 10).expect("should get outputs");
        assert_eq!(outputs.len(), 2);
    }

    #[test]
    fn test_not_found_errors() {
        let store = create_test_store();

        let result = store.get("nonexistent");
        assert!(result.unwrap().is_none());

        let result = store.delete("nonexistent");
        assert!(matches!(result.unwrap_err(), StoreError::NotFound { .. }));

        let mut cmd = CustomCommand::new("Test", "echo");
        cmd.id = "nonexistent".to_string();
        let result = store.update(&cmd);
        assert!(matches!(result.unwrap_err(), StoreError::NotFound { .. }));
    }

    #[test]
    fn test_keywords_and_environment() {
        let store = create_test_store();

        let cmd = CustomCommand::builder("Test", "echo $VAR")
            .keywords(vec!["key1".to_string(), "key2".to_string()])
            .env("VAR", "value")
            .env("ANOTHER", "test")
            .build();

        store.create(&cmd).expect("should create");

        let retrieved = store
            .get(&cmd.id)
            .expect("should get")
            .expect("should exist");
        assert_eq!(retrieved.keywords, vec!["key1", "key2"]);
        assert_eq!(retrieved.environment.get("VAR"), Some(&"value".to_string()));
        assert_eq!(
            retrieved.environment.get("ANOTHER"),
            Some(&"test".to_string())
        );
    }

    #[tokio::test]
    async fn test_async_operations() {
        let store = create_test_store();
        let cmd = CustomCommand::new("Test", "echo hello");

        store
            .create_async(cmd.clone())
            .await
            .expect("should create async");

        let retrieved = store
            .get_async(cmd.id.clone())
            .await
            .expect("should get async");
        assert!(retrieved.is_some());

        let commands = store.list_async().await.expect("should list async");
        assert_eq!(commands.len(), 1);

        store
            .delete_async(cmd.id)
            .await
            .expect("should delete async");
    }
}
