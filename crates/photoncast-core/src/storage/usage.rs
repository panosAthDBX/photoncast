//! Usage frequency tracking for frecency ranking.
//!
//! This module provides usage tracking functionality that integrates with
//! the frecency-based ranking system.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use tokio::task;

use crate::search::ranking::FrecencyScore;
use crate::storage::Database;

/// Converts a Unix timestamp to SystemTime.
fn timestamp_to_system_time(timestamp: i64) -> Option<SystemTime> {
    if timestamp > 0 {
        UNIX_EPOCH.checked_add(std::time::Duration::from_secs(timestamp as u64))
    } else {
        None
    }
}

/// Tracks usage data for frecency-based ranking.
///
/// This struct provides methods to record and retrieve usage statistics
/// for applications, commands, and files.
pub struct UsageTracker {
    db: Database,
}

impl UsageTracker {
    /// Creates a new usage tracker.
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }

    /// Returns a reference to the underlying database.
    #[must_use]
    pub const fn database(&self) -> &Database {
        &self.db
    }

    // -------------------------------------------------------------------------
    // App Usage
    // -------------------------------------------------------------------------

    /// Records an app launch.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn record_app_launch(&self, bundle_id: &str) -> Result<()> {
        self.db.record_app_launch(bundle_id)
    }

    /// Records an app launch asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn record_app_launch_async(&self, bundle_id: String) -> Result<()> {
        let db = self.db.clone();
        task::spawn_blocking(move || db.record_app_launch(&bundle_id)).await?
    }

    /// Gets the frecency score for an app.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_app_frecency(&self, bundle_id: &str) -> Result<FrecencyScore> {
        let usage = self.db.get_app_usage(bundle_id)?;

        match usage {
            Some((launch_count, Some(last_launched_ts))) => {
                let last_launched = timestamp_to_system_time(last_launched_ts);
                Ok(FrecencyScore::calculate(launch_count, last_launched))
            },
            Some((launch_count, None)) => {
                // Has been launched but no timestamp (shouldn't happen, but handle gracefully)
                Ok(FrecencyScore::new(launch_count, 0.0))
            },
            None => Ok(FrecencyScore::zero()),
        }
    }

    /// Gets the frecency score for an app asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_app_frecency_async(&self, bundle_id: String) -> Result<FrecencyScore> {
        let db = self.db.clone();
        task::spawn_blocking(move || {
            let usage = db.get_app_usage(&bundle_id)?;

            match usage {
                Some((launch_count, Some(last_launched_ts))) => {
                    let last_launched = timestamp_to_system_time(last_launched_ts);
                    Ok(FrecencyScore::calculate(launch_count, last_launched))
                },
                Some((launch_count, None)) => Ok(FrecencyScore::new(launch_count, 0.0)),
                None => Ok(FrecencyScore::zero()),
            }
        })
        .await?
    }

    // -------------------------------------------------------------------------
    // Command Usage
    // -------------------------------------------------------------------------

    /// Records a command execution.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn record_command_execution(&self, command_id: &str) -> Result<()> {
        self.db.record_command_use(command_id)
    }

    /// Records a command execution asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn record_command_execution_async(&self, command_id: String) -> Result<()> {
        let db = self.db.clone();
        task::spawn_blocking(move || db.record_command_use(&command_id)).await?
    }

    /// Gets the frecency score for a command.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_command_frecency(&self, command_id: &str) -> Result<FrecencyScore> {
        let conn = self.db.connection();

        let result: Result<(u32, i64), _> = conn.query_row(
            "SELECT use_count, last_used_at FROM command_usage WHERE command_id = ?1",
            [command_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((use_count, last_used_ts)) => {
                let last_used = timestamp_to_system_time(last_used_ts);
                Ok(FrecencyScore::calculate(use_count, last_used))
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(FrecencyScore::zero()),
            Err(e) => Err(e).context("failed to get command frecency"),
        }
    }

    /// Gets the frecency score for a command asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_command_frecency_async(&self, command_id: String) -> Result<FrecencyScore> {
        let db = self.db.clone();
        task::spawn_blocking(move || {
            let conn = db.connection();

            let result: Result<(u32, i64), _> = conn.query_row(
                "SELECT use_count, last_used_at FROM command_usage WHERE command_id = ?1",
                [&command_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            );

            match result {
                Ok((use_count, last_used_ts)) => {
                    let last_used = timestamp_to_system_time(last_used_ts);
                    Ok(FrecencyScore::calculate(use_count, last_used))
                },
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(FrecencyScore::zero()),
                Err(e) => Err(e).context("failed to get command frecency"),
            }
        })
        .await?
    }

    // -------------------------------------------------------------------------
    // File Usage
    // -------------------------------------------------------------------------

    /// Records a file open.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn record_file_open(&self, path: &str) -> Result<()> {
        self.db.record_file_open(path)
    }

    /// Records a file open asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn record_file_open_async(&self, path: String) -> Result<()> {
        let db = self.db.clone();
        task::spawn_blocking(move || db.record_file_open(&path)).await?
    }

    /// Gets the frecency score for a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_file_frecency(&self, path: &str) -> Result<FrecencyScore> {
        let conn = self.db.connection();

        let result: Result<(u32, i64), _> = conn.query_row(
            "SELECT open_count, last_opened_at FROM file_usage WHERE file_path = ?1",
            [path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((open_count, last_opened_ts)) => {
                let last_opened = timestamp_to_system_time(last_opened_ts);
                Ok(FrecencyScore::calculate(open_count, last_opened))
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(FrecencyScore::zero()),
            Err(e) => Err(e).context("failed to get file frecency"),
        }
    }

    /// Gets the frecency score for a file asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_file_frecency_async(&self, path: String) -> Result<FrecencyScore> {
        let db = self.db.clone();
        task::spawn_blocking(move || {
            let conn = db.connection();

            let result: Result<(u32, i64), _> = conn.query_row(
                "SELECT open_count, last_opened_at FROM file_usage WHERE file_path = ?1",
                [&path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            );

            match result {
                Ok((open_count, last_opened_ts)) => {
                    let last_opened = timestamp_to_system_time(last_opened_ts);
                    Ok(FrecencyScore::calculate(open_count, last_opened))
                },
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(FrecencyScore::zero()),
                Err(e) => Err(e).context("failed to get file frecency"),
            }
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_tracker() -> UsageTracker {
        let db = Database::open_in_memory().expect("should open database");
        UsageTracker::new(db)
    }

    #[test]
    fn test_record_and_get_app_frecency() {
        let tracker = create_tracker();

        // Initially no frecency
        let frecency = tracker
            .get_app_frecency("com.apple.Safari")
            .expect("should get frecency");
        assert_eq!(frecency.frequency, 0);
        assert_eq!(frecency.score(), 0.0);

        // Record launches
        tracker
            .record_app_launch("com.apple.Safari")
            .expect("should record");
        tracker
            .record_app_launch("com.apple.Safari")
            .expect("should record");
        tracker
            .record_app_launch("com.apple.Safari")
            .expect("should record");

        let frecency = tracker
            .get_app_frecency("com.apple.Safari")
            .expect("should get frecency");
        assert_eq!(frecency.frequency, 3);
        assert!(frecency.score() > 0.0);
        // Recency should be close to 1.0 since we just recorded it
        assert!(frecency.recency > 0.9);
    }

    #[test]
    fn test_record_and_get_command_frecency() {
        let tracker = create_tracker();

        // Initially no frecency
        let frecency = tracker
            .get_command_frecency("lock_screen")
            .expect("should get frecency");
        assert_eq!(frecency.frequency, 0);

        // Record executions
        tracker
            .record_command_execution("lock_screen")
            .expect("should record");
        tracker
            .record_command_execution("lock_screen")
            .expect("should record");

        let frecency = tracker
            .get_command_frecency("lock_screen")
            .expect("should get frecency");
        assert_eq!(frecency.frequency, 2);
        assert!(frecency.score() > 0.0);
    }

    #[test]
    fn test_record_and_get_file_frecency() {
        let tracker = create_tracker();

        let path = "/Users/test/Documents/file.txt";

        // Initially no frecency
        let frecency = tracker
            .get_file_frecency(path)
            .expect("should get frecency");
        assert_eq!(frecency.frequency, 0);

        // Record opens
        tracker.record_file_open(path).expect("should record");

        let frecency = tracker
            .get_file_frecency(path)
            .expect("should get frecency");
        assert_eq!(frecency.frequency, 1);
        assert!(frecency.score() > 0.0);
    }

    #[tokio::test]
    async fn test_async_app_frecency() {
        let tracker = create_tracker();

        tracker
            .record_app_launch_async("com.apple.Safari".to_string())
            .await
            .expect("should record async");

        let frecency = tracker
            .get_app_frecency_async("com.apple.Safari".to_string())
            .await
            .expect("should get async");

        assert_eq!(frecency.frequency, 1);
        assert!(frecency.score() > 0.0);
    }

    #[tokio::test]
    async fn test_async_command_frecency() {
        let tracker = create_tracker();

        tracker
            .record_command_execution_async("sleep".to_string())
            .await
            .expect("should record async");

        let frecency = tracker
            .get_command_frecency_async("sleep".to_string())
            .await
            .expect("should get async");

        assert_eq!(frecency.frequency, 1);
    }

    #[tokio::test]
    async fn test_async_file_frecency() {
        let tracker = create_tracker();

        let path = "/Users/test/file.txt".to_string();

        tracker
            .record_file_open_async(path.clone())
            .await
            .expect("should record async");

        let frecency = tracker
            .get_file_frecency_async(path)
            .await
            .expect("should get async");

        assert_eq!(frecency.frequency, 1);
    }

    #[test]
    fn test_frecency_calculation_accuracy() {
        let tracker = create_tracker();

        // Record multiple launches
        for _ in 0..10 {
            tracker
                .record_app_launch("com.apple.Safari")
                .expect("should record");
        }

        let frecency = tracker
            .get_app_frecency("com.apple.Safari")
            .expect("should get frecency");

        assert_eq!(frecency.frequency, 10);
        // Recent usage should have high recency (close to 1.0)
        assert!(frecency.recency > 0.99);
        // Combined score should be approximately frequency * recency
        assert!((frecency.score() - (frecency.frequency as f64) * frecency.recency).abs() < 0.1);
    }
}
