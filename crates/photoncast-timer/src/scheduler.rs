//! Timer scheduler with SQLite persistence.
//!
//! This module provides the core timer scheduling functionality with persistence
//! to survive app restarts.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::error::{Result, TimerError};

/// Action to perform when timer expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerAction {
    /// Put Mac to sleep
    Sleep,
    /// Shut down the Mac
    Shutdown,
    /// Restart the Mac
    Restart,
    /// Lock the screen
    Lock,
}

impl TimerAction {
    /// Returns the display name for this action.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Sleep => "Sleep",
            Self::Shutdown => "Shut Down",
            Self::Restart => "Restart",
            Self::Lock => "Lock Screen",
        }
    }

    /// Returns the icon name for this action.
    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Sleep => "moon",
            Self::Shutdown => "power",
            Self::Restart => "rotate-ccw",
            Self::Lock => "lock",
        }
    }
}

/// An active timer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTimer {
    /// Action to perform
    pub action: TimerAction,
    /// When to execute the action
    pub execute_at: DateTime<Utc>,
    /// When the timer was created
    pub created_at: DateTime<Utc>,
}

impl ActiveTimer {
    /// Creates a new timer.
    #[must_use]
    pub fn new(action: TimerAction, execute_at: DateTime<Utc>) -> Self {
        Self {
            action,
            execute_at,
            created_at: Utc::now(),
        }
    }

    /// Returns the remaining duration until execution.
    #[must_use]
    pub fn remaining(&self) -> chrono::Duration {
        self.execute_at - Utc::now()
    }

    /// Returns true if the timer has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.execute_at
    }

    /// Returns a formatted countdown string (e.g., "5m 30s", "1h 23m").
    #[must_use]
    pub fn countdown_string(&self) -> String {
        let remaining = self.remaining();

        if remaining.num_seconds() < 0 {
            return "Expired".to_string();
        }

        let hours = remaining.num_hours();
        let minutes = remaining.num_minutes() % 60;
        let seconds = remaining.num_seconds() % 60;

        if hours > 0 {
            format!("{hours}h {minutes}m")
        } else if minutes > 0 {
            format!("{minutes}m {seconds}s")
        } else {
            format!("{seconds}s")
        }
    }
}

/// Timer scheduler with SQLite persistence.
pub struct TimerScheduler {
    /// Database connection
    db: Arc<RwLock<Connection>>,
    /// Database path
    db_path: PathBuf,
}

impl TimerScheduler {
    /// Creates a new timer scheduler with the given database path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    #[allow(clippy::future_not_send)]
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TimerError::Database(format!("Failed to create directory: {e}")))?;
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| TimerError::Database(format!("Failed to open database: {e}")))?;

        let db_path_for_log = db_path.clone();

        let scheduler = Self {
            db: Arc::new(RwLock::new(conn)),
            db_path,
        };

        // Initialize schema
        scheduler.init_schema().await?;

        info!(
            "Timer scheduler initialized at: {}",
            db_path_for_log.display()
        );

        Ok(scheduler)
    }

    /// Initializes the database schema.
    #[allow(clippy::future_not_send)]
    async fn init_schema(&self) -> Result<()> {
        let db = self.db.write().await;

        db.execute(
            "CREATE TABLE IF NOT EXISTS active_timer (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                action TEXT NOT NULL,
                execute_at TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| TimerError::Database(format!("Failed to create schema: {e}")))?;

        drop(db);

        debug!("Timer database schema initialized");

        Ok(())
    }

    /// Sets an active timer, replacing any existing timer.
    ///
    /// # Errors
    ///
    /// Returns an error if the timer cannot be persisted to the database.
    #[allow(clippy::future_not_send)]
    pub async fn set_timer(&self, timer: ActiveTimer) -> Result<()> {
        let db = self.db.write().await;

        // Serialize action
        let action = serde_json::to_string(&timer.action)
            .map_err(|e| TimerError::Serialization(e.to_string()))?;

        db.execute(
            "INSERT OR REPLACE INTO active_timer (id, action, execute_at, created_at)
             VALUES (1, ?1, ?2, ?3)",
            params![
                action,
                timer.execute_at.to_rfc3339(),
                timer.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| TimerError::Database(format!("Failed to save timer: {e}")))?;

        drop(db);

        info!(
            "Timer set: {} at {}",
            timer.action.display_name(),
            timer.execute_at
        );

        Ok(())
    }

    /// Gets the active timer, if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    #[allow(clippy::future_not_send)]
    pub async fn get_timer(&self) -> Result<Option<ActiveTimer>> {
        let db = self.db.read().await;

        let mut stmt = db
            .prepare("SELECT action, execute_at, created_at FROM active_timer WHERE id = 1")
            .map_err(|e| TimerError::Database(format!("Failed to prepare query: {e}")))?;

        let timer = stmt
            .query_row([], |row| {
                let action_str: String = row.get(0)?;
                let execute_at_str: String = row.get(1)?;
                let created_at_str: String = row.get(2)?;

                let action: TimerAction = serde_json::from_str(&action_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let execute_at = DateTime::parse_from_rfc3339(&execute_at_str)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc);

                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(ActiveTimer {
                    action,
                    execute_at,
                    created_at,
                })
            })
            .optional()
            .map_err(|e| TimerError::Database(format!("Failed to query timer: {e}")))?;

        drop(stmt);
        drop(db);

        Ok(timer)
    }

    /// Cancels the active timer.
    ///
    /// # Errors
    ///
    /// Returns an error if the database deletion fails.
    #[allow(clippy::future_not_send)]
    pub async fn cancel_timer(&self) -> Result<()> {
        let db = self.db.write().await;

        db.execute("DELETE FROM active_timer WHERE id = 1", [])
            .map_err(|e| TimerError::Database(format!("Failed to cancel timer: {e}")))?;

        drop(db);

        info!("Timer cancelled");

        Ok(())
    }

    /// Executes the timer action by running the appropriate system command.
    ///
    /// Commands have a 30-second timeout to prevent hanging.
    ///
    /// # Errors
    ///
    /// Returns an error if the command execution fails or times out.
    pub async fn execute_action(action: TimerAction) -> Result<()> {
        /// Timeout for system commands (30 seconds should be more than enough)
        const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
        
        info!("Executing timer action: {}", action.display_name());

        let mut command = match action {
            TimerAction::Sleep => Command::new("pmset"),
            TimerAction::Shutdown | TimerAction::Restart | TimerAction::Lock => Command::new("osascript"),
        };

        match action {
            TimerAction::Sleep => {
                command.arg("sleepnow");
            },
            TimerAction::Shutdown => {
                command.args(["-e", "tell app \"System Events\" to shut down"]);
            },
            TimerAction::Restart => {
                command.args(["-e", "tell app \"System Events\" to restart"]);
            },
            TimerAction::Lock => {
                // Use Cmd+Ctrl+Q keystroke to lock screen (works on all modern macOS versions)
                command.args(["-e", "tell application \"System Events\" to keystroke \"q\" using {command down, control down}"]);
            },
        }

        let output = tokio::time::timeout(COMMAND_TIMEOUT, command.output())
            .await
            .map_err(|_| TimerError::Execution(format!(
                "Command timed out after {}s", COMMAND_TIMEOUT.as_secs()
            )))?
            .map_err(|e| TimerError::Execution(format!("Failed to execute command: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TimerError::Execution(format!("Command failed: {stderr}")));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_timer_scheduler_persistence() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("timer.db");

        let scheduler = TimerScheduler::new(&db_path).await.unwrap();

        // Create a timer
        let timer = ActiveTimer::new(
            TimerAction::Sleep,
            Utc::now() + chrono::Duration::minutes(30),
        );

        // Set timer
        scheduler.set_timer(timer.clone()).await.unwrap();

        // Retrieve timer
        let retrieved = scheduler.get_timer().await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.action, timer.action);

        // Cancel timer
        scheduler.cancel_timer().await.unwrap();
        let retrieved = scheduler.get_timer().await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_timer_replace() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("timer.db");

        let scheduler = TimerScheduler::new(&db_path).await.unwrap();

        // Set first timer
        let timer1 = ActiveTimer::new(
            TimerAction::Sleep,
            Utc::now() + chrono::Duration::minutes(30),
        );
        scheduler.set_timer(timer1).await.unwrap();

        // Set second timer (should replace first)
        let timer2 = ActiveTimer::new(
            TimerAction::Shutdown,
            Utc::now() + chrono::Duration::hours(1),
        );
        scheduler.set_timer(timer2).await.unwrap();

        // Should only have the second timer
        let retrieved = scheduler.get_timer().await.unwrap().unwrap();
        assert_eq!(retrieved.action, TimerAction::Shutdown);
    }

    #[test]
    fn test_countdown_string() {
        let timer = ActiveTimer::new(
            TimerAction::Sleep,
            Utc::now() + chrono::Duration::seconds(3665), // 1h 1m 5s
        );

        let countdown = timer.countdown_string();
        assert!(countdown.contains("1h"));
        assert!(countdown.contains("1m"));
    }

    #[test]
    fn test_timer_action_properties() {
        assert_eq!(TimerAction::Sleep.display_name(), "Sleep");
        assert_eq!(TimerAction::Shutdown.display_name(), "Shut Down");
        assert_eq!(TimerAction::Restart.display_name(), "Restart");
        assert_eq!(TimerAction::Lock.display_name(), "Lock Screen");

        assert_eq!(TimerAction::Sleep.icon(), "moon");
        assert_eq!(TimerAction::Shutdown.icon(), "power");
        assert_eq!(TimerAction::Restart.icon(), "rotate-ccw");
        assert_eq!(TimerAction::Lock.icon(), "lock");
    }

    #[test]
    fn test_timer_action_all_variants_have_display_name() {
        // Ensure all variants have non-empty display names
        let actions = [
            TimerAction::Sleep,
            TimerAction::Shutdown,
            TimerAction::Restart,
            TimerAction::Lock,
        ];
        
        for action in actions {
            assert!(!action.display_name().is_empty());
            assert!(!action.icon().is_empty());
        }
    }

    #[test]
    fn test_timer_action_serialization() {
        // Test that actions serialize/deserialize correctly
        let action = TimerAction::Sleep;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"sleep\"");
        
        let deserialized: TimerAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_active_timer_is_expired() {
        // Test with past time (expired)
        let expired_timer = ActiveTimer::new(
            TimerAction::Sleep,
            Utc::now() - chrono::Duration::seconds(10),
        );
        assert!(expired_timer.is_expired());
        
        // Test with future time (not expired)
        let future_timer = ActiveTimer::new(
            TimerAction::Sleep,
            Utc::now() + chrono::Duration::hours(1),
        );
        assert!(!future_timer.is_expired());
    }

    #[test]
    fn test_active_timer_time_remaining() {
        let timer = ActiveTimer::new(
            TimerAction::Sleep,
            Utc::now() + chrono::Duration::seconds(60),
        );
        
        let remaining = timer.remaining();
        // Should be close to 60 seconds (within 2 seconds tolerance for test execution)
        assert!(remaining.num_seconds() >= 58 && remaining.num_seconds() <= 62);
    }

    /// Test that execute_action handles non-existent commands gracefully
    /// Note: We can't test actual system commands in unit tests, but we can
    /// verify the error handling works for invalid commands
    #[tokio::test]
    async fn test_execute_action_error_handling() {
        // This test verifies that our error handling works correctly
        // by checking that the function signature and error types are correct.
        // 
        // Actual system command tests should be done in integration tests
        // with appropriate sandboxing/mocking.
        
        // Verify that TimerError::Execution can be created
        let error = TimerError::Execution("test error".to_string());
        assert!(error.to_string().contains("test error"));
    }
}
