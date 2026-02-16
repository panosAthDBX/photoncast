//! SQLite database wrapper with async support and migrations.
//!
//! This module provides a database wrapper that uses `tokio::task::spawn_blocking`
//! for SQLite operations to avoid blocking the async runtime.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::Connection;
use tokio::task;

use crate::indexer::{AppBundleId, AppCategory, IndexedApp};

/// Current schema version.
#[cfg(test)]
const CURRENT_SCHEMA_VERSION: i32 = 2;

/// Database wrapper for PhotonCast storage.
///
/// Uses `parking_lot::Mutex` for synchronization and `tokio::task::spawn_blocking`
/// for async operations to avoid blocking the tokio runtime.
#[derive(Debug)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            path: self.path.clone(),
        }
    }
}

impl Database {
    /// Opens or creates a database at the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or migrations fail.
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create database directory")?;
        }

        let conn = Connection::open(path).context("failed to open database")?;

        // Enable WAL mode for better concurrent access
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: Some(path.to_path_buf()),
        };
        db.run_migrations()?;

        Ok(db)
    }

    /// Opens or creates a database at the specified path asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or migrations fail.
    pub async fn open_async(path: PathBuf) -> Result<Self> {
        task::spawn_blocking(move || Self::open(&path)).await?
    }

    /// Opens an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("failed to open in-memory database")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: None,
        };
        db.run_migrations()?;

        Ok(db)
    }

    /// Gets the current schema version.
    fn get_schema_version(&self) -> Result<i32> {
        let conn = self.conn.lock();

        // Check if schema_version table exists
        let table_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |row| row.get(0),
        )?;

        if !table_exists {
            return Ok(0);
        }

        let version: i32 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        Ok(version)
    }

    /// Records a schema version as applied.
    fn record_version(&self, version: i32) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![version, now],
        )
        .context("failed to record schema version")?;

        Ok(())
    }

    /// Runs database migrations.
    fn run_migrations(&self) -> Result<()> {
        let current_version = self.get_schema_version()?;

        if current_version < 1 {
            self.migrate_v1()?;
            self.record_version(1)?;
        }

        if current_version < 2 {
            self.migrate_v2()?;
            self.record_version(2)?;
        }

        Ok(())
    }

    /// Migration v1: Initial schema.
    fn migrate_v1(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute_batch(
            r"
            -- Schema version tracking
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            );

            -- Usage tracking for frecency ranking
            CREATE TABLE IF NOT EXISTS app_usage (
                bundle_id TEXT PRIMARY KEY,
                launch_count INTEGER NOT NULL DEFAULT 0,
                last_launched_at INTEGER,
                created_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_app_usage_last_launched 
            ON app_usage(last_launched_at DESC);

            -- Command usage
            CREATE TABLE IF NOT EXISTS command_usage (
                command_id TEXT PRIMARY KEY,
                use_count INTEGER NOT NULL DEFAULT 0,
                last_used_at INTEGER,
                created_at INTEGER NOT NULL
            );

            -- File access tracking
            CREATE TABLE IF NOT EXISTS file_usage (
                file_path TEXT PRIMARY KEY,
                open_count INTEGER NOT NULL DEFAULT 0,
                last_opened_at INTEGER,
                created_at INTEGER NOT NULL
            );

            -- App index cache
            CREATE TABLE IF NOT EXISTS app_cache (
                bundle_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                icon_path TEXT,
                keywords TEXT,
                category TEXT,
                last_modified INTEGER NOT NULL,
                indexed_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_app_cache_name ON app_cache(name);
            ",
        )
        .context("failed to run migration v1")?;

        Ok(())
    }

    /// Migration v2: Per-query frecency tracking.
    fn migrate_v2(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute_batch(
            r"
            -- Per-query frecency: tracks which items are selected for specific query prefixes
            CREATE TABLE IF NOT EXISTS query_frecency (
                query_prefix TEXT NOT NULL,
                item_id TEXT NOT NULL,
                frequency INTEGER NOT NULL DEFAULT 1,
                last_used_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (query_prefix, item_id)
            );

            CREATE INDEX IF NOT EXISTS idx_query_frecency_item
            ON query_frecency(item_id);

            CREATE INDEX IF NOT EXISTS idx_query_frecency_last_used
            ON query_frecency(last_used_at);
            ",
        )
        .context("failed to run migration v2")?;

        Ok(())
    }

    /// Returns the current schema version.
    #[allow(clippy::double_must_use)]
    pub fn schema_version(&self) -> Result<i32> {
        self.get_schema_version()
    }

    /// Returns a reference to the underlying connection (for testing).
    ///
    /// # Warning
    ///
    /// This method is primarily for testing and should be used carefully.
    /// Prefer using the async methods for production code.
    #[must_use]
    #[allow(clippy::double_must_use)]
    pub fn connection(&self) -> parking_lot::MutexGuard<'_, Connection> {
        self.conn.lock()
    }

    /// Returns the database file path, if not in-memory.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    // -------------------------------------------------------------------------
    // App Cache Operations
    // -------------------------------------------------------------------------

    /// Inserts a single app into the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn insert_app(&self, app: &IndexedApp) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();
        let keywords = app.keywords.join(",");
        let category = app.category.as_ref().map(category_to_string);

        conn.execute(
            r"
            INSERT OR REPLACE INTO app_cache 
            (bundle_id, name, path, icon_path, keywords, category, last_modified, indexed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            rusqlite::params![
                app.bundle_id.as_str(),
                &app.name,
                app.path.to_string_lossy().as_ref(),
                app.icon_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned()),
                keywords,
                category,
                app.last_modified.timestamp(),
                now,
            ],
        )
        .context("failed to insert app")?;

        Ok(())
    }

    /// Inserts a single app into the cache asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn insert_app_async(&self, app: IndexedApp) -> Result<()> {
        let db = self.clone();
        task::spawn_blocking(move || db.insert_app(&app)).await?
    }

    /// Inserts multiple apps in a single transaction (batch insert).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn insert_apps_batch(&self, apps: &[IndexedApp]) -> Result<usize> {
        let mut conn = self.conn.lock();
        let now = Utc::now().timestamp();

        let tx = conn.transaction()?;

        let mut count = 0;
        for app in apps {
            let keywords = app.keywords.join(",");
            let category = app.category.as_ref().map(category_to_string);

            tx.execute(
                r"
                INSERT OR REPLACE INTO app_cache 
                (bundle_id, name, path, icon_path, keywords, category, last_modified, indexed_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
                rusqlite::params![
                    app.bundle_id.as_str(),
                    &app.name,
                    app.path.to_string_lossy().as_ref(),
                    app.icon_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned()),
                    keywords,
                    category,
                    app.last_modified.timestamp(),
                    now,
                ],
            )?;
            count += 1;
        }

        tx.commit()?;

        Ok(count)
    }

    /// Inserts multiple apps in a single transaction asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn insert_apps_batch_async(&self, apps: Vec<IndexedApp>) -> Result<usize> {
        let db = self.clone();
        task::spawn_blocking(move || db.insert_apps_batch(&apps)).await?
    }

    /// Gets all cached apps.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_all_apps(&self) -> Result<Vec<IndexedApp>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r"
            SELECT bundle_id, name, path, icon_path, keywords, category, last_modified
            FROM app_cache
            ORDER BY name ASC
            LIMIT 10000
            ",
        )?;

        let apps = stmt
            .query_map([], |row| {
                let bundle_id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let path: String = row.get(2)?;
                let icon_path: Option<String> = row.get(3)?;
                let keywords: Option<String> = row.get(4)?;
                let category: Option<String> = row.get(5)?;
                let last_modified: i64 = row.get(6)?;

                Ok(IndexedApp {
                    name,
                    bundle_id: AppBundleId::new(bundle_id),
                    path: PathBuf::from(path),
                    icon_path: icon_path.map(PathBuf::from),
                    keywords: keywords
                        .map(|k| k.split(',').map(String::from).collect())
                        .unwrap_or_default(),
                    category: category.map(|c| category_from_string(&c)),
                    last_modified: DateTime::from_timestamp(last_modified, 0)
                        .unwrap_or_else(Utc::now),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(apps)
    }

    /// Gets all cached apps asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_all_apps_async(&self) -> Result<Vec<IndexedApp>> {
        let db = self.clone();
        task::spawn_blocking(move || db.get_all_apps()).await?
    }

    /// Gets a specific app by bundle ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_app(&self, bundle_id: &str) -> Result<Option<IndexedApp>> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            r"
            SELECT bundle_id, name, path, icon_path, keywords, category, last_modified
            FROM app_cache
            WHERE bundle_id = ?1
            ",
            [bundle_id],
            |row| {
                let bundle_id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let path: String = row.get(2)?;
                let icon_path: Option<String> = row.get(3)?;
                let keywords: Option<String> = row.get(4)?;
                let category: Option<String> = row.get(5)?;
                let last_modified: i64 = row.get(6)?;

                Ok(IndexedApp {
                    name,
                    bundle_id: AppBundleId::new(bundle_id),
                    path: PathBuf::from(path),
                    icon_path: icon_path.map(PathBuf::from),
                    keywords: keywords
                        .map(|k| k.split(',').map(String::from).collect())
                        .unwrap_or_default(),
                    category: category.map(|c| category_from_string(&c)),
                    last_modified: DateTime::from_timestamp(last_modified, 0)
                        .unwrap_or_else(Utc::now),
                })
            },
        );

        match result {
            Ok(app) => Ok(Some(app)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("failed to get app"),
        }
    }

    /// Gets a specific app by bundle ID asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_app_async(&self, bundle_id: String) -> Result<Option<IndexedApp>> {
        let db = self.clone();
        task::spawn_blocking(move || db.get_app(&bundle_id)).await?
    }

    /// Removes an app from the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn remove_app(&self, bundle_id: &str) -> Result<bool> {
        let conn = self.conn.lock();

        let rows_affected = conn
            .execute("DELETE FROM app_cache WHERE bundle_id = ?1", [bundle_id])
            .context("failed to remove app")?;

        Ok(rows_affected > 0)
    }

    /// Removes an app from the cache asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn remove_app_async(&self, bundle_id: String) -> Result<bool> {
        let db = self.clone();
        task::spawn_blocking(move || db.remove_app(&bundle_id)).await?
    }

    /// Updates an app in the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn update_app(&self, app: &IndexedApp) -> Result<bool> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();
        let keywords = app.keywords.join(",");
        let category = app.category.as_ref().map(category_to_string);

        let rows_affected = conn
            .execute(
                r"
            UPDATE app_cache SET
                name = ?2,
                path = ?3,
                icon_path = ?4,
                keywords = ?5,
                category = ?6,
                last_modified = ?7,
                indexed_at = ?8
            WHERE bundle_id = ?1
            ",
                rusqlite::params![
                    app.bundle_id.as_str(),
                    &app.name,
                    app.path.to_string_lossy().as_ref(),
                    app.icon_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned()),
                    keywords,
                    category,
                    app.last_modified.timestamp(),
                    now,
                ],
            )
            .context("failed to update app")?;

        Ok(rows_affected > 0)
    }

    /// Updates an app in the cache asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn update_app_async(&self, app: IndexedApp) -> Result<bool> {
        let db = self.clone();
        task::spawn_blocking(move || db.update_app(&app)).await?
    }

    /// Clears all cached apps.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn clear_app_cache(&self) -> Result<usize> {
        let conn = self.conn.lock();

        let rows_affected = conn
            .execute("DELETE FROM app_cache", [])
            .context("failed to clear app cache")?;

        Ok(rows_affected)
    }

    /// Clears all cached apps asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn clear_app_cache_async(&self) -> Result<usize> {
        let db = self.clone();
        task::spawn_blocking(move || db.clear_app_cache()).await?
    }

    /// Gets the count of cached apps.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    #[allow(clippy::cast_possible_truncation)]
    pub fn app_cache_count(&self) -> Result<usize> {
        let conn = self.conn.lock();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM app_cache", [], |row| row.get(0))?;

        Ok(count as usize)
    }

    /// Gets the count of cached apps asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn app_cache_count_async(&self) -> Result<usize> {
        let db = self.clone();
        task::spawn_blocking(move || db.app_cache_count()).await?
    }

    // -------------------------------------------------------------------------
    // Usage Operations (Sync versions - async versions in usage.rs)
    // -------------------------------------------------------------------------

    /// Records an app launch.
    pub fn record_app_launch(&self, bundle_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();

        conn.execute(
            r"
            INSERT INTO app_usage (bundle_id, launch_count, last_launched_at, created_at)
            VALUES (?1, 1, ?2, ?2)
            ON CONFLICT(bundle_id) DO UPDATE SET
                launch_count = launch_count + 1,
                last_launched_at = ?2
            ",
            rusqlite::params![bundle_id, now],
        )
        .context("failed to record app launch")?;

        Ok(())
    }

    /// Records an app launch asynchronously.
    pub async fn record_app_launch_async(&self, bundle_id: String) -> Result<()> {
        let db = self.clone();
        task::spawn_blocking(move || db.record_app_launch(&bundle_id)).await?
    }

    /// Records a command usage.
    pub fn record_command_use(&self, command_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();

        conn.execute(
            r"
            INSERT INTO command_usage (command_id, use_count, last_used_at, created_at)
            VALUES (?1, 1, ?2, ?2)
            ON CONFLICT(command_id) DO UPDATE SET
                use_count = use_count + 1,
                last_used_at = ?2
            ",
            rusqlite::params![command_id, now],
        )
        .context("failed to record command use")?;

        Ok(())
    }

    /// Records a file open.
    pub fn record_file_open(&self, file_path: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().timestamp();

        conn.execute(
            r"
            INSERT INTO file_usage (file_path, open_count, last_opened_at, created_at)
            VALUES (?1, 1, ?2, ?2)
            ON CONFLICT(file_path) DO UPDATE SET
                open_count = open_count + 1,
                last_opened_at = ?2
            ",
            rusqlite::params![file_path, now],
        )
        .context("failed to record file open")?;

        Ok(())
    }

    /// Gets app usage statistics.
    pub fn get_app_usage(&self, bundle_id: &str) -> Result<Option<(u32, Option<i64>)>> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT launch_count, last_launched_at FROM app_usage WHERE bundle_id = ?1",
            [bundle_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok(usage) => Ok(Some(usage)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("failed to get app usage"),
        }
    }

    // -------------------------------------------------------------------------
    // Per-Query Frecency Operations
    // -------------------------------------------------------------------------

    /// Records a query→item selection for per-query frecency.
    ///
    /// Stores the association between a query prefix and the selected item.
    /// Only tracks prefixes of length 1-4 characters.
    pub fn record_query_selection(&self, query_prefix: &str, item_id: &str) -> Result<()> {
        if query_prefix.is_empty() || query_prefix.len() > 4 {
            return Ok(()); // Only track prefixes 1-4 chars
        }

        let conn = self.conn.lock();
        let now = Utc::now().timestamp();

        conn.execute(
            r"
            INSERT INTO query_frecency (query_prefix, item_id, frequency, last_used_at, created_at)
            VALUES (?1, ?2, 1, ?3, ?3)
            ON CONFLICT(query_prefix, item_id) DO UPDATE SET
                frequency = frequency + 1,
                last_used_at = ?3
            ",
            rusqlite::params![query_prefix, item_id, now],
        )
        .context("failed to record query selection")?;

        Ok(())
    }

    /// Gets the per-query frecency data for an item given a query prefix.
    ///
    /// Returns `(frequency, last_used_at)` if found.
    pub fn get_query_frecency(
        &self,
        query_prefix: &str,
        item_id: &str,
    ) -> Result<Option<(u32, i64)>> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT frequency, last_used_at FROM query_frecency WHERE query_prefix = ?1 AND item_id = ?2",
            rusqlite::params![query_prefix, item_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("failed to get query frecency"),
        }
    }

    /// Prunes old query frecency entries (older than `max_age_days`).
    pub fn prune_query_frecency(&self, max_age_days: i64) -> Result<usize> {
        let conn = self.conn.lock();
        let cutoff = Utc::now().timestamp() - (max_age_days * 86400);

        let deleted = conn
            .execute(
                "DELETE FROM query_frecency WHERE last_used_at < ?1",
                rusqlite::params![cutoff],
            )
            .context("failed to prune query frecency")?;

        Ok(deleted)
    }

    /// Gets the top N apps by frecency score.
    ///
    /// Frecency combines frequency (launch count) and recency (time since last launch).
    /// Apps launched more recently and more frequently score higher.
    ///
    /// Returns a list of (bundle_id, launch_count, last_launched_at) tuples.
    pub fn get_top_apps_by_frecency(&self, limit: usize) -> Result<Vec<(String, u32, i64)>> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().timestamp();

        // Frecency formula: launch_count * recency_weight
        // recency_weight = 1.0 / (1.0 + days_since_last_launch)
        // This gives higher weight to recently used apps
        let mut stmt = conn
            .prepare(
                r"
            SELECT 
                bundle_id, 
                launch_count, 
                last_launched_at,
                (launch_count * 1.0 / (1.0 + ((?1 - last_launched_at) / 86400.0))) as frecency
            FROM app_usage 
            WHERE last_launched_at IS NOT NULL
            ORDER BY frecency DESC
            LIMIT ?2
            ",
            )
            .context("failed to prepare frecency query")?;

        let rows = stmt
            .query_map(rusqlite::params![now, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .context("failed to query top apps")?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.context("failed to read app usage row")?);
        }

        Ok(results)
    }
}

/// Returns the default database path.
#[must_use]
#[allow(clippy::map_unwrap_or)]
pub fn default_database_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "PhotonCast")
        .map(|dirs| dirs.data_dir().join("photoncast.db"))
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Library/Application Support/PhotonCast/photoncast.db")
        })
}

/// Converts an `AppCategory` to a string for storage.
fn category_to_string(category: &AppCategory) -> String {
    match category {
        AppCategory::DeveloperTools => "developer-tools".to_string(),
        AppCategory::Entertainment => "entertainment".to_string(),
        AppCategory::Finance => "finance".to_string(),
        AppCategory::Graphics => "graphics".to_string(),
        AppCategory::Productivity => "productivity".to_string(),
        AppCategory::SocialNetworking => "social-networking".to_string(),
        AppCategory::Utilities => "utilities".to_string(),
        AppCategory::Other(s) => s.clone(),
    }
}

/// Converts a string from storage to an `AppCategory`.
fn category_from_string(s: &str) -> AppCategory {
    match s {
        "developer-tools" => AppCategory::DeveloperTools,
        "entertainment" => AppCategory::Entertainment,
        "finance" => AppCategory::Finance,
        "graphics" => AppCategory::Graphics,
        "productivity" => AppCategory::Productivity,
        "social-networking" => AppCategory::SocialNetworking,
        "utilities" => AppCategory::Utilities,
        other => AppCategory::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app(name: &str, bundle_id: &str) -> IndexedApp {
        IndexedApp {
            name: name.to_string(),
            bundle_id: AppBundleId::new(bundle_id),
            path: PathBuf::from(format!("/Applications/{name}.app")),
            icon_path: Some(PathBuf::from(format!(
                "/Applications/{name}.app/Contents/Resources/icon.icns"
            ))),
            keywords: vec!["test".to_string(), name.to_lowercase()],
            category: Some(AppCategory::Productivity),
            last_modified: Utc::now(),
        }
    }

    #[test]
    fn test_database_open_in_memory() {
        let db = Database::open_in_memory().expect("should open in-memory database");
        assert!(db.path().is_none());
    }

    #[test]
    fn test_migrations_run_correctly() {
        let db = Database::open_in_memory().expect("should open database");
        let version = db.schema_version().expect("should get schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_migrations_idempotent() {
        let db = Database::open_in_memory().expect("should open database");

        // Run migrations again (should be a no-op)
        db.run_migrations()
            .expect("migrations should be idempotent");

        let version = db.schema_version().expect("should get schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_migrations_idempotent_on_file_backed_database() {
        let dir = tempfile::tempdir().expect("should create temp dir");
        let db_path = dir.path().join("photoncast.db");

        let db = Database::open(&db_path).expect("should open file-backed database");
        db.run_migrations()
            .expect("first migration rerun should be a no-op");
        db.run_migrations()
            .expect("second migration rerun should be a no-op");

        let version = db.schema_version().expect("should get schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_open_corrupt_database_file_returns_error() {
        let dir = tempfile::tempdir().expect("should create temp dir");
        let db_path = dir.path().join("corrupt.db");

        std::fs::write(&db_path, b"this is not a sqlite database")
            .expect("should write corrupt file");

        let result = Database::open(&db_path);
        assert!(result.is_err(), "opening corrupt database should fail");
    }

    #[test]
    fn test_open_missing_database_file_creates_database() {
        let dir = tempfile::tempdir().expect("should create temp dir");
        let db_path = dir.path().join("nested").join("photoncast.db");

        assert!(!db_path.exists(), "precondition: db should not exist yet");

        let db = Database::open(&db_path).expect("opening missing database should create it");

        assert!(db_path.exists(), "database file should be created");
        let version = db.schema_version().expect("should get schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_insert_and_get_app() {
        let db = Database::open_in_memory().expect("should open database");
        let app = create_test_app("Safari", "com.apple.Safari");

        db.insert_app(&app).expect("should insert app");

        let retrieved = db
            .get_app("com.apple.Safari")
            .expect("should get app")
            .expect("app should exist");

        assert_eq!(retrieved.name, "Safari");
        assert_eq!(retrieved.bundle_id.as_str(), "com.apple.Safari");
        assert_eq!(retrieved.keywords, vec!["test", "safari"]);
    }

    #[test]
    fn test_get_all_apps() {
        let db = Database::open_in_memory().expect("should open database");

        let apps = vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Chrome", "com.google.Chrome"),
            create_test_app("Firefox", "org.mozilla.Firefox"),
        ];

        for app in &apps {
            db.insert_app(app).expect("should insert app");
        }

        let all_apps = db.get_all_apps().expect("should get all apps");

        // Results are sorted alphabetically
        assert_eq!(all_apps.len(), 3);
        assert_eq!(all_apps[0].name, "Chrome");
        assert_eq!(all_apps[1].name, "Firefox");
        assert_eq!(all_apps[2].name, "Safari");
    }

    #[test]
    fn test_remove_app() {
        let db = Database::open_in_memory().expect("should open database");
        let app = create_test_app("Safari", "com.apple.Safari");

        db.insert_app(&app).expect("should insert app");

        let removed = db
            .remove_app("com.apple.Safari")
            .expect("should remove app");
        assert!(removed);

        let retrieved = db.get_app("com.apple.Safari").expect("should query");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_remove_nonexistent_app() {
        let db = Database::open_in_memory().expect("should open database");

        let removed = db
            .remove_app("com.nonexistent.App")
            .expect("should not error");
        assert!(!removed);
    }

    #[test]
    fn test_update_app() {
        let db = Database::open_in_memory().expect("should open database");
        let mut app = create_test_app("Safari", "com.apple.Safari");

        db.insert_app(&app).expect("should insert app");

        // Update the app
        app.name = "Safari Updated".to_string();
        app.keywords = vec!["browser".to_string()];

        let updated = db.update_app(&app).expect("should update app");
        assert!(updated);

        let retrieved = db
            .get_app("com.apple.Safari")
            .expect("should get app")
            .expect("app should exist");

        assert_eq!(retrieved.name, "Safari Updated");
        assert_eq!(retrieved.keywords, vec!["browser"]);
    }

    #[test]
    fn test_batch_insert() {
        let db = Database::open_in_memory().expect("should open database");

        let apps = vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Chrome", "com.google.Chrome"),
            create_test_app("Firefox", "org.mozilla.Firefox"),
        ];

        let count = db.insert_apps_batch(&apps).expect("should batch insert");
        assert_eq!(count, 3);

        let all_apps = db.get_all_apps().expect("should get all apps");
        assert_eq!(all_apps.len(), 3);
    }

    #[test]
    fn test_high_volume_insert_does_not_panic() {
        let db = Database::open_in_memory().expect("should open database");

        let apps: Vec<IndexedApp> = (0..150)
            .map(|i| create_test_app(&format!("App{i}"), &format!("com.example.App{i}")))
            .collect();

        let inserted = db
            .insert_apps_batch(&apps)
            .expect("high volume batch insert should succeed");
        assert_eq!(inserted, apps.len());

        let count = db.app_cache_count().expect("should get app cache count");
        assert_eq!(count, apps.len());
    }

    #[test]
    fn test_clear_app_cache() {
        let db = Database::open_in_memory().expect("should open database");

        let apps = vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Chrome", "com.google.Chrome"),
        ];

        db.insert_apps_batch(&apps).expect("should batch insert");

        let cleared = db.clear_app_cache().expect("should clear cache");
        assert_eq!(cleared, 2);

        let count = db.app_cache_count().expect("should get count");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_app_cache_count() {
        let db = Database::open_in_memory().expect("should open database");

        let count = db.app_cache_count().expect("should get count");
        assert_eq!(count, 0);

        let apps = vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Chrome", "com.google.Chrome"),
        ];

        db.insert_apps_batch(&apps).expect("should batch insert");

        let count = db.app_cache_count().expect("should get count");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_record_and_get_app_usage() {
        let db = Database::open_in_memory().expect("should open database");

        // No usage yet
        let usage = db
            .get_app_usage("com.apple.Safari")
            .expect("should query usage");
        assert!(usage.is_none());

        // Record some launches
        db.record_app_launch("com.apple.Safari")
            .expect("should record launch");
        db.record_app_launch("com.apple.Safari")
            .expect("should record launch");
        db.record_app_launch("com.apple.Safari")
            .expect("should record launch");

        let usage = db
            .get_app_usage("com.apple.Safari")
            .expect("should query usage")
            .expect("usage should exist");

        assert_eq!(usage.0, 3); // launch_count
        assert!(usage.1.is_some()); // last_launched_at
    }

    #[test]
    fn test_record_command_use() {
        let db = Database::open_in_memory().expect("should open database");

        db.record_command_use("lock_screen")
            .expect("should record command use");
        db.record_command_use("lock_screen")
            .expect("should record command use");

        // Verify command was recorded (using direct SQL)
        let conn = db.connection();
        let (count, _): (i64, i64) = conn
            .query_row(
                "SELECT use_count, last_used_at FROM command_usage WHERE command_id = ?1",
                ["lock_screen"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("should query");

        assert_eq!(count, 2);
    }

    #[test]
    fn test_record_file_open() {
        let db = Database::open_in_memory().expect("should open database");

        db.record_file_open("/Users/test/Documents/file.txt")
            .expect("should record file open");

        // Verify file was recorded (using direct SQL)
        let conn = db.connection();
        let count: i64 = conn
            .query_row(
                "SELECT open_count FROM file_usage WHERE file_path = ?1",
                ["/Users/test/Documents/file.txt"],
                |row| row.get(0),
            )
            .expect("should query");

        assert_eq!(count, 1);
    }

    #[test]
    fn test_upsert_behavior() {
        let db = Database::open_in_memory().expect("should open database");
        let mut app = create_test_app("Safari", "com.apple.Safari");

        // Insert
        db.insert_app(&app).expect("should insert app");

        // Insert again with updated name (should upsert)
        app.name = "Safari Browser".to_string();
        db.insert_app(&app).expect("should upsert app");

        let all_apps = db.get_all_apps().expect("should get all apps");
        assert_eq!(all_apps.len(), 1);
        assert_eq!(all_apps[0].name, "Safari Browser");
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let db = Arc::new(Database::open_in_memory().expect("should open database"));
        let mut handles = vec![];

        // Spawn multiple threads that insert apps
        for i in 0..10 {
            let db = Arc::clone(&db);
            handles.push(thread::spawn(move || {
                let app = create_test_app(&format!("App{i}"), &format!("com.test.App{i}"));
                db.insert_app(&app).expect("should insert app");
            }));
        }

        // Wait for all threads
        for handle in handles {
            handle.join().expect("thread should complete");
        }

        let count = db.app_cache_count().expect("should get count");
        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn test_async_operations() {
        let db = Database::open_in_memory().expect("should open database");
        let app = create_test_app("Safari", "com.apple.Safari");

        db.insert_app_async(app.clone())
            .await
            .expect("should insert async");

        let all_apps = db.get_all_apps_async().await.expect("should get all async");
        assert_eq!(all_apps.len(), 1);

        let retrieved = db
            .get_app_async("com.apple.Safari".to_string())
            .await
            .expect("should get async")
            .expect("app should exist");

        assert_eq!(retrieved.name, "Safari");

        db.remove_app_async("com.apple.Safari".to_string())
            .await
            .expect("should remove async");

        let count = db
            .app_cache_count_async()
            .await
            .expect("should count async");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_async_batch_insert() {
        let db = Database::open_in_memory().expect("should open database");

        let apps = vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Chrome", "com.google.Chrome"),
            create_test_app("Firefox", "org.mozilla.Firefox"),
        ];

        let count = db
            .insert_apps_batch_async(apps)
            .await
            .expect("should batch insert async");
        assert_eq!(count, 3);
    }

    // -------------------------------------------------------------------------
    // Migration v2 & Query Frecency Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_migration_v2_creates_table() {
        let db = Database::open_in_memory().expect("should open database");
        let version = db.schema_version().expect("should get schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);

        // Verify the query_frecency table exists by inserting a row
        db.record_query_selection("sh", "com.test.app")
            .expect("should insert into query_frecency");
    }

    #[test]
    fn test_migration_v2_additive() {
        let db = Database::open_in_memory().expect("should open database");

        // Insert v1 data
        let app = create_test_app("Safari", "com.apple.Safari");
        db.insert_app(&app).expect("should insert app");
        db.record_app_launch("com.apple.Safari")
            .expect("should record launch");

        // Verify v1 data is still intact (migrations already ran in open_in_memory)
        let retrieved = db
            .get_app("com.apple.Safari")
            .expect("should get app")
            .expect("app should exist");
        assert_eq!(retrieved.name, "Safari");

        let usage = db
            .get_app_usage("com.apple.Safari")
            .expect("should get usage")
            .expect("usage should exist");
        assert_eq!(usage.0, 1);
    }

    #[test]
    fn test_migration_v2_idempotent() {
        let db = Database::open_in_memory().expect("should open database");

        // Running migrations again should not error
        db.run_migrations()
            .expect("migrations should be idempotent");

        let version = db.schema_version().expect("should get schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_record_query_selection_insert() {
        let db = Database::open_in_memory().expect("should open database");

        db.record_query_selection("sh", "com.test.shortwave")
            .expect("should record");

        let result = db
            .get_query_frecency("sh", "com.test.shortwave")
            .expect("should get")
            .expect("should exist");

        assert_eq!(result.0, 1); // frequency
        assert!(result.1 > 0); // last_used_at
    }

    #[test]
    fn test_record_query_selection_upsert() {
        let db = Database::open_in_memory().expect("should open database");

        db.record_query_selection("sh", "com.test.shortwave")
            .expect("first insert");
        db.record_query_selection("sh", "com.test.shortwave")
            .expect("second insert");

        let result = db
            .get_query_frecency("sh", "com.test.shortwave")
            .expect("should get")
            .expect("should exist");

        assert_eq!(result.0, 2); // frequency incremented
    }

    #[test]
    fn test_record_query_selection_ignores_long_prefix() {
        let db = Database::open_in_memory().expect("should open database");

        // Prefix > 4 chars should be ignored
        db.record_query_selection("short", "com.test.shortwave")
            .expect("should succeed (no-op)");

        let result = db
            .get_query_frecency("short", "com.test.shortwave")
            .expect("should get");

        assert!(result.is_none());
    }

    #[test]
    fn test_record_query_selection_ignores_empty_prefix() {
        let db = Database::open_in_memory().expect("should open database");

        db.record_query_selection("", "com.test.shortwave")
            .expect("should succeed (no-op)");

        let result = db
            .get_query_frecency("", "com.test.shortwave")
            .expect("should get");

        assert!(result.is_none());
    }

    #[test]
    fn test_get_query_frecency_not_found() {
        let db = Database::open_in_memory().expect("should open database");

        let result = db
            .get_query_frecency("zz", "com.nonexistent")
            .expect("should get");

        assert!(result.is_none());
    }

    #[test]
    fn test_prune_query_frecency() {
        let db = Database::open_in_memory().expect("should open database");

        // Insert an entry
        db.record_query_selection("sh", "com.test.shortwave")
            .expect("should record");

        // Prune with 0-day cutoff (everything is older than "right now")
        // Current entries should NOT be pruned because they were just inserted
        let deleted = db.prune_query_frecency(0).expect("should prune");
        // With max_age_days=0, cutoff = now - 0 = now, so entries at "now" survive
        assert_eq!(deleted, 0);

        // Manually set old timestamp for testing
        {
            let conn = db.connection();
            let old_ts = Utc::now().timestamp() - 86400 * 31; // 31 days ago
            conn.execute(
                "UPDATE query_frecency SET last_used_at = ?1",
                rusqlite::params![old_ts],
            )
            .expect("should update");
        }

        // Now prune entries older than 30 days
        let deleted = db.prune_query_frecency(30).expect("should prune");
        assert_eq!(deleted, 1);

        // Verify it's gone
        let result = db
            .get_query_frecency("sh", "com.test.shortwave")
            .expect("should get");
        assert!(result.is_none());
    }
}
