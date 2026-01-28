//! Quick links storage with SQLite and FTS5 search.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use tokio::task;

use crate::error::Result;
use crate::models::{QuickLink, QuickLinkIcon, QuickLinkId, QuickLinksToml};

/// Current schema version (reserved for future migration support).
#[allow(dead_code)]
const CURRENT_SCHEMA_VERSION: i32 = 2;

/// Quick links storage with SQLite backend.
#[derive(Debug)]
pub struct QuickLinksStorage {
    conn: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

impl Clone for QuickLinksStorage {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            path: self.path.clone(),
        }
    }
}

impl QuickLinksStorage {
    /// Opens or creates a quick links storage database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create storage directory")?;
        }

        let conn = Connection::open(path).context("failed to open database")?;

        // Enable WAL mode for better concurrent access
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: Some(path.to_path_buf()),
        };

        storage.run_migrations()?;

        Ok(storage)
    }

    /// Opens quick links storage asynchronously.
    pub async fn open_async<P: AsRef<Path> + Send + 'static>(path: P) -> Result<Self> {
        Self::run_blocking(move || Self::open(path)).await
    }

    async fn run_blocking<T, F>(f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            task::spawn_blocking(f).await?
        } else {
            f()
        }
    }

    /// Opens an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("failed to open in-memory database")?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: None,
        };

        storage.run_migrations()?;

        Ok(storage)
    }

    /// Returns the number of quicklinks in storage.
    pub fn count(&self) -> Result<usize> {
        let conn = self.conn.lock();
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM quick_links", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Populates the database with bundled quicklinks if empty.
    ///
    /// Returns the number of quicklinks added.
    pub fn populate_bundled_if_empty(&self) -> Result<usize> {
        let count = self.count()?;
        if count > 0 {
            return Ok(0);
        }

        let bundled = crate::library::get_bundled_quicklinks();
        let mut added = 0;

        for bundled_link in bundled {
            let quicklink = crate::library::to_quicklink(bundled_link);
            if self.store_sync(&quicklink).is_ok() {
                added += 1;
            }
        }

        tracing::info!(added = added, "Populated bundled quicklinks");
        Ok(added)
    }

    /// Stores a quick link synchronously.
    fn store_sync(&self, link: &QuickLink) -> Result<QuickLinkId> {
        let conn = self.conn.lock();

        let keywords_json =
            serde_json::to_string(&link.keywords).context("failed to serialize keywords")?;
        let tags_json = serde_json::to_string(&link.tags).context("failed to serialize tags")?;

        let (icon_type, icon_value) = Self::icon_to_db(&link.icon);

        // Note: id is auto-generated, database schema uses 'title' and 'url' column names
        conn.execute(
            "INSERT INTO quick_links (title, url, keywords, tags, icon_path, favicon_path, icon_type, icon_value, open_with, alias, hotkey, created_at, accessed_at, access_count)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                link.name,
                link.link,
                keywords_json,
                tags_json,
                icon_type,
                icon_value,
                link.open_with,
                link.alias,
                link.hotkey,
                link.created_at.timestamp(),
                link.accessed_at.map(|t| t.timestamp()),
                link.access_count,
            ],
        )?;

        let id = conn.last_insert_rowid();

        // Update FTS index (includes alias for search)
        let alias_str = link.alias.as_deref().unwrap_or("");
        let fts_keywords = format!("{keywords_json} {alias_str}");
        let _ = conn.execute(
            "INSERT INTO quicklinks_fts(rowid, title, url, keywords) VALUES (?1, ?2, ?3, ?4)",
            params![id, link.name, link.link, fts_keywords],
        );

        Ok(QuickLinkId::from(id))
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

    /// Gets the current schema version.
    fn get_schema_version(&self) -> Result<i32> {
        let version = {
            let conn = self.conn.lock();

            // Check if schema_version table exists
            let table_exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
                [],
                |row| row.get(0),
            )?;

            if table_exists {
                conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                    row.get(0)
                })
                .unwrap_or(0)
            } else {
                0
            }
        };

        Ok(version)
    }

    /// Records a schema version.
    fn record_version(&self, version: i32) -> Result<()> {
        let now = Utc::now().timestamp();

        {
            let conn = self.conn.lock();
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                params![version, now],
            )?;
        }

        Ok(())
    }

    /// Migration v1: Initial schema.
    fn migrate_v1(&self) -> Result<()> {
        {
            let conn = self.conn.lock();
            conn.execute_batch(
                r"
            -- Schema version tracking
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            );

            -- Quick links
            CREATE TABLE IF NOT EXISTS quick_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                keywords TEXT,  -- JSON array
                tags TEXT,      -- JSON array
                icon_path TEXT,
                favicon_path TEXT,
                created_at INTEGER NOT NULL,
                accessed_at INTEGER,
                access_count INTEGER DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_quicklinks_title ON quick_links(title);
            CREATE INDEX IF NOT EXISTS idx_quicklinks_access ON quick_links(access_count DESC);

            -- Full-text search (simplified, without triggers for now)
            CREATE VIRTUAL TABLE IF NOT EXISTS quicklinks_fts USING fts5(
                title, url, keywords
            );
            ",
            )?;
        }

        Ok(())
    }

    /// Migration v2: Raycast parity - add new fields and rename columns.
    fn migrate_v2(&self) -> Result<()> {
        let conn = self.conn.lock();

        // Check if columns exist before adding them
        let has_open_with = Self::column_exists(&conn, "quick_links", "open_with");
        let has_icon_type = Self::column_exists(&conn, "quick_links", "icon_type");
        let has_alias = Self::column_exists(&conn, "quick_links", "alias");
        let has_hotkey = Self::column_exists(&conn, "quick_links", "hotkey");

        // Add new columns if they don't exist
        if !has_open_with {
            conn.execute("ALTER TABLE quick_links ADD COLUMN open_with TEXT", [])?;
        }

        if !has_icon_type {
            conn.execute(
                "ALTER TABLE quick_links ADD COLUMN icon_type TEXT NOT NULL DEFAULT 'default'",
                [],
            )?;
        }

        // Add icon_value column (stores path for favicon/custom, emoji char, or system icon name)
        let has_icon_value = Self::column_exists(&conn, "quick_links", "icon_value");
        if !has_icon_value {
            conn.execute("ALTER TABLE quick_links ADD COLUMN icon_value TEXT", [])?;
        }

        if !has_alias {
            conn.execute("ALTER TABLE quick_links ADD COLUMN alias TEXT", [])?;
        }

        if !has_hotkey {
            conn.execute("ALTER TABLE quick_links ADD COLUMN hotkey TEXT", [])?;
        }

        // Migrate existing icon_path and favicon_path to new icon_type/icon_value
        // Prefer favicon_path over icon_path if both exist
        conn.execute_batch(
            r"
            UPDATE quick_links
            SET icon_type = 'favicon', icon_value = favicon_path
            WHERE favicon_path IS NOT NULL AND favicon_path != '' AND icon_type = 'default';

            UPDATE quick_links
            SET icon_type = 'custom', icon_value = icon_path
            WHERE icon_path IS NOT NULL AND icon_path != '' AND icon_type = 'default';
            ",
        )?;

        // Create index on alias for fast alias lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_quicklinks_alias ON quick_links(alias)",
            [],
        )?;

        drop(conn);

        Ok(())
    }

    /// Check if a column exists in a table.
    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let query = format!("PRAGMA table_info({table})");
        let mut stmt = match conn.prepare(&query) {
            Ok(stmt) => stmt,
            Err(_) => return false,
        };

        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map(|rows| rows.filter_map(std::result::Result::ok).collect())
            .unwrap_or_default();

        columns.contains(&column.to_string())
    }

    /// Stores a quick link.
    pub async fn store(&self, link: &QuickLink) -> Result<QuickLinkId> {
        let link = link.clone();
        let storage = self.clone();

        Self::run_blocking(move || {
            let conn = storage.conn.lock();

            let keywords_json =
                serde_json::to_string(&link.keywords).context("failed to serialize keywords")?;
            let tags_json =
                serde_json::to_string(&link.tags).context("failed to serialize tags")?;

            // Convert QuickLinkIcon to icon_type and icon_value
            let (icon_type, icon_value) = Self::icon_to_db(&link.icon);

            conn.execute(
                "INSERT INTO quick_links (title, url, keywords, tags, icon_path, favicon_path, icon_type, icon_value, open_with, alias, hotkey, created_at, accessed_at, access_count)
                 VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    link.name,
                    link.link,
                    keywords_json,
                    tags_json,
                    icon_type,
                    icon_value,
                    link.open_with,
                    link.alias,
                    link.hotkey,
                    link.created_at.timestamp(),
                    link.accessed_at.map(|t| t.timestamp()),
                    link.access_count,
                ],
            )?;

            let id = conn.last_insert_rowid();

            // Update FTS index (includes alias for search)
            let alias_str = link.alias.as_deref().unwrap_or("");
            let fts_keywords = format!("{keywords_json} {alias_str}");
            let _ = conn.execute(
                "INSERT INTO quicklinks_fts(rowid, title, url, keywords) VALUES (?1, ?2, ?3, ?4)",
                params![id, link.name, link.link, fts_keywords],
            );

            drop(conn);

            Ok(QuickLinkId::from(id))
        })
        .await
    }

    /// Convert QuickLinkIcon to database representation (icon_type, icon_value).
    fn icon_to_db(icon: &QuickLinkIcon) -> (String, Option<String>) {
        match icon {
            QuickLinkIcon::Favicon(path) => (
                "favicon".to_string(),
                Some(path.to_string_lossy().to_string()),
            ),
            QuickLinkIcon::Emoji(emoji) => ("emoji".to_string(), Some(emoji.clone())),
            QuickLinkIcon::SystemIcon(name) => ("system".to_string(), Some(name.clone())),
            QuickLinkIcon::CustomImage(path) => (
                "custom".to_string(),
                Some(path.to_string_lossy().to_string()),
            ),
            QuickLinkIcon::Default => ("default".to_string(), None),
        }
    }

    /// Convert database representation to QuickLinkIcon.
    fn db_to_icon(icon_type: &str, icon_value: Option<&str>) -> QuickLinkIcon {
        match icon_type {
            "favicon" => icon_value.map_or(QuickLinkIcon::Default, |v| {
                QuickLinkIcon::Favicon(PathBuf::from(v))
            }),
            "emoji" => icon_value.map_or(QuickLinkIcon::Default, |v| {
                QuickLinkIcon::Emoji(v.to_string())
            }),
            "system" => icon_value.map_or(QuickLinkIcon::Default, |v| {
                QuickLinkIcon::SystemIcon(v.to_string())
            }),
            "custom" => icon_value.map_or(QuickLinkIcon::Default, |v| {
                QuickLinkIcon::CustomImage(PathBuf::from(v))
            }),
            _ => QuickLinkIcon::Default,
        }
    }

    /// Updates an existing quick link.
    pub async fn update(&self, link: &QuickLink) -> Result<()> {
        let link = link.clone();
        let storage = self.clone();

        Self::run_blocking(move || {
            let conn = storage.conn.lock();

            let keywords_json =
                serde_json::to_string(&link.keywords).context("failed to serialize keywords")?;
            let tags_json =
                serde_json::to_string(&link.tags).context("failed to serialize tags")?;

            // Convert QuickLinkIcon to icon_type and icon_value
            let (icon_type, icon_value) = Self::icon_to_db(&link.icon);

            let id_num: i64 = link.id.as_str().parse().context("invalid ID format")?;

            conn.execute(
                "UPDATE quick_links SET title = ?1, url = ?2, keywords = ?3, tags = ?4, icon_type = ?5, icon_value = ?6, open_with = ?7, alias = ?8, hotkey = ?9, accessed_at = ?10, access_count = ?11
                 WHERE id = ?12",
                params![
                    link.name,
                    link.link,
                    keywords_json,
                    tags_json,
                    icon_type,
                    icon_value,
                    link.open_with,
                    link.alias,
                    link.hotkey,
                    link.accessed_at.map(|t| t.timestamp()),
                    link.access_count,
                    id_num,
                ],
            )?;

            // Update FTS index (includes alias for search)
            let alias_str = link.alias.as_deref().unwrap_or("");
            let fts_keywords = format!("{keywords_json} {alias_str}");
            let _ = conn.execute("DELETE FROM quicklinks_fts WHERE rowid = ?1", params![id_num]);
            let _ = conn.execute(
                "INSERT INTO quicklinks_fts(rowid, title, url, keywords) VALUES (?1, ?2, ?3, ?4)",
                params![id_num, link.name, link.link, fts_keywords],
            );

            drop(conn);

            Ok(())
        })
        .await
    }

    /// Gets a quick link by ID.
    pub async fn get(&self, id: &QuickLinkId) -> Result<Option<QuickLink>> {
        let id = id.clone();
        let storage = self.clone();

        Self::run_blocking(move || {
            let id_num: i64 = id.as_str().parse().context("invalid ID format")?;

            let result = storage.conn.lock().query_row(
                "SELECT id, title, url, keywords, tags, icon_type, icon_value, open_with, alias, hotkey, created_at, accessed_at, access_count
                 FROM quick_links WHERE id = ?1",
                params![id_num],
                |row| {
                    let keywords_json: String = row.get(3)?;
                    let tags_json: String = row.get(4)?;
                    let keywords: Vec<String> =
                        serde_json::from_str(&keywords_json).unwrap_or_default();
                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                    let icon_type: String = row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "default".to_string());
                    let icon_value: Option<String> = row.get(6)?;
                    let icon = Self::db_to_icon(&icon_type, icon_value.as_deref());

                    let open_with: Option<String> = row.get(7)?;
                    let alias: Option<String> = row.get(8)?;
                    let hotkey: Option<String> = row.get(9)?;

                    let created_at: i64 = row.get(10)?;
                    let accessed_at: Option<i64> = row.get(11)?;

                    Ok(QuickLink {
                        id: QuickLinkId::from(row.get::<_, i64>(0)?),
                        name: row.get(1)?,
                        link: row.get(2)?,
                        open_with,
                        icon,
                        alias,
                        hotkey,
                        keywords,
                        tags,
                        created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
                        accessed_at: accessed_at.and_then(|t| DateTime::from_timestamp(t, 0)),
                        access_count: row.get(12)?,
                    })
                },
            );

            match result {
                Ok(link) => Ok(Some(link)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
        .await
    }

    /// Deletes a quick link by ID.
    pub async fn delete(&self, id: &QuickLinkId) -> Result<()> {
        let id = id.clone();
        let storage = self.clone();

        Self::run_blocking(move || {
            let conn = storage.conn.lock();
            let id_num: i64 = id.as_str().parse().context("invalid ID format")?;

            // Delete from FTS first
            let _ = conn.execute(
                "DELETE FROM quicklinks_fts WHERE rowid = ?1",
                params![id_num],
            );

            // Delete from main table
            conn.execute("DELETE FROM quick_links WHERE id = ?1", params![id_num])?;
            drop(conn);
            Ok(())
        })
        .await
    }

    /// Loads all quick links, sorted by access frequency.
    pub async fn load_all(&self) -> Result<Vec<QuickLink>> {
        let storage = self.clone();

        Self::run_blocking(move || {
            let conn = storage.conn.lock();
            let mut stmt = conn.prepare(
                "SELECT id, title, url, keywords, tags, icon_type, icon_value, open_with, alias, hotkey, created_at, accessed_at, access_count
                 FROM quick_links
                 ORDER BY access_count DESC, title ASC",
            )?;

            let links = stmt
                .query_map([], |row| {
                    let keywords_json: String = row.get(3)?;
                    let tags_json: String = row.get(4)?;
                    let keywords: Vec<String> =
                        serde_json::from_str(&keywords_json).unwrap_or_default();
                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                    let icon_type: String = row
                        .get::<_, Option<String>>(5)?
                        .unwrap_or_else(|| "default".to_string());
                    let icon_value: Option<String> = row.get(6)?;
                    let icon = Self::db_to_icon(&icon_type, icon_value.as_deref());

                    let open_with: Option<String> = row.get(7)?;
                    let alias: Option<String> = row.get(8)?;
                    let hotkey: Option<String> = row.get(9)?;

                    let created_at: i64 = row.get(10)?;
                    let accessed_at: Option<i64> = row.get(11)?;

                    Ok(QuickLink {
                        id: QuickLinkId::from(row.get::<_, i64>(0)?),
                        name: row.get(1)?,
                        link: row.get(2)?,
                        open_with,
                        icon,
                        alias,
                        hotkey,
                        keywords,
                        tags,
                        created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
                        accessed_at: accessed_at.and_then(|t| DateTime::from_timestamp(t, 0)),
                        access_count: row.get(12)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            drop(stmt);
            drop(conn);
            Ok(links)
        })
        .await
    }

    /// Loads all quick links synchronously.
    pub fn load_all_sync(&self) -> Result<Vec<QuickLink>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, url, keywords, tags, icon_type, icon_value, open_with, alias, hotkey, created_at, accessed_at, access_count
             FROM quick_links
             ORDER BY access_count DESC, title ASC",
        )?;

        let links = stmt
            .query_map([], |row| {
                let keywords_json: String = row.get(3)?;
                let tags_json: String = row.get(4)?;
                let keywords: Vec<String> =
                    serde_json::from_str(&keywords_json).unwrap_or_default();
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                let icon_type: String = row
                    .get::<_, Option<String>>(5)?
                    .unwrap_or_else(|| "default".to_string());
                let icon_value: Option<String> = row.get(6)?;
                let icon = Self::db_to_icon(&icon_type, icon_value.as_deref());

                let open_with: Option<String> = row.get(7)?;
                let alias: Option<String> = row.get(8)?;
                let hotkey: Option<String> = row.get(9)?;

                let created_at: i64 = row.get(10)?;
                let accessed_at: Option<i64> = row.get(11)?;

                Ok(QuickLink {
                    id: QuickLinkId::from(row.get::<_, i64>(0)?),
                    name: row.get(1)?,
                    link: row.get(2)?,
                    open_with,
                    icon,
                    alias,
                    hotkey,
                    keywords,
                    tags,
                    created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
                    accessed_at: accessed_at.and_then(|t| DateTime::from_timestamp(t, 0)),
                    access_count: row.get(12)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(links)
    }

    /// Searches quick links using FTS5.
    pub async fn search(&self, query: &str) -> Result<Vec<QuickLink>> {
        let query = query.to_string();
        let storage = self.clone();

        Self::run_blocking(move || {
            let conn = storage.conn.lock();

            // FTS5 query - search in title, url, and keywords
            let search_query = Self::prepare_fts_query(&query);
            if search_query == "\"\"" {
                return Ok(Vec::new());
            }

            let mut stmt = conn.prepare(
                "SELECT ql.id, ql.title, ql.url, ql.keywords, ql.tags, ql.icon_type, ql.icon_value, ql.open_with, ql.alias, ql.hotkey, ql.created_at, ql.accessed_at, ql.access_count
                 FROM quick_links ql
                 INNER JOIN quicklinks_fts fts ON ql.id = fts.rowid
                 WHERE quicklinks_fts MATCH ?1
                 ORDER BY ql.access_count DESC, ql.title ASC",
            )?;

            let links = stmt
                .query_map([&search_query], |row| {
                    let keywords_json: String = row.get(3)?;
                    let tags_json: String = row.get(4)?;
                    let keywords: Vec<String> =
                        serde_json::from_str(&keywords_json).unwrap_or_default();
                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                    let icon_type: String = row
                        .get::<_, Option<String>>(5)?
                        .unwrap_or_else(|| "default".to_string());
                    let icon_value: Option<String> = row.get(6)?;
                    let icon = Self::db_to_icon(&icon_type, icon_value.as_deref());

                    let open_with: Option<String> = row.get(7)?;
                    let alias: Option<String> = row.get(8)?;
                    let hotkey: Option<String> = row.get(9)?;

                    let created_at: i64 = row.get(10)?;
                    let accessed_at: Option<i64> = row.get(11)?;

                    Ok(QuickLink {
                        id: QuickLinkId::from(row.get::<_, i64>(0)?),
                        name: row.get(1)?,
                        link: row.get(2)?,
                        open_with,
                        icon,
                        alias,
                        hotkey,
                        keywords,
                        tags,
                        created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
                        accessed_at: accessed_at.and_then(|t| DateTime::from_timestamp(t, 0)),
                        access_count: row.get(12)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            drop(stmt);
            drop(conn);
            Ok(links)
        })
        .await
    }

    /// Searches quick links synchronously.
    pub fn search_sync(&self, query: &str) -> Result<Vec<QuickLink>> {
        let conn = self.conn.lock();

        let search_query = Self::prepare_fts_query(query);
        if search_query == "\"\"" {
            return Ok(Vec::new());
        }

        let mut stmt = conn.prepare(
            "SELECT ql.id, ql.title, ql.url, ql.keywords, ql.tags, ql.icon_type, ql.icon_value, ql.open_with, ql.alias, ql.hotkey, ql.created_at, ql.accessed_at, ql.access_count
             FROM quick_links ql
             INNER JOIN quicklinks_fts fts ON ql.id = fts.rowid
             WHERE quicklinks_fts MATCH ?1
             ORDER BY ql.access_count DESC, ql.title ASC",
        )?;

        let links = stmt
            .query_map([&search_query], |row| {
                let keywords_json: String = row.get(3)?;
                let tags_json: String = row.get(4)?;
                let keywords: Vec<String> =
                    serde_json::from_str(&keywords_json).unwrap_or_default();
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                let icon_type: String = row
                    .get::<_, Option<String>>(5)?
                    .unwrap_or_else(|| "default".to_string());
                let icon_value: Option<String> = row.get(6)?;
                let icon = Self::db_to_icon(&icon_type, icon_value.as_deref());

                let open_with: Option<String> = row.get(7)?;
                let alias: Option<String> = row.get(8)?;
                let hotkey: Option<String> = row.get(9)?;

                let created_at: i64 = row.get(10)?;
                let accessed_at: Option<i64> = row.get(11)?;

                Ok(QuickLink {
                    id: QuickLinkId::from(row.get::<_, i64>(0)?),
                    name: row.get(1)?,
                    link: row.get(2)?,
                    open_with,
                    icon,
                    alias,
                    hotkey,
                    keywords,
                    tags,
                    created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
                    accessed_at: accessed_at.and_then(|t| DateTime::from_timestamp(t, 0)),
                    access_count: row.get(12)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        drop(stmt);
        drop(conn);
        Ok(links)
    }

    /// Exports all quick links to TOML format.
    pub async fn export_to_toml(&self) -> Result<QuickLinksToml> {
        let links = self.load_all().await?;

        let toml_links = links.into_iter().map(std::convert::Into::into).collect();

        Ok(QuickLinksToml { links: toml_links })
    }

    fn prepare_fts_query(query: &str) -> String {
        let escaped = query
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('*', "\\*")
            .replace(':', "\\:")
            .replace('(', "")
            .replace(')', "")
            .replace('{', "")
            .replace('}', "");

        let terms: Vec<String> = escaped
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term))
            .collect();

        if terms.is_empty() {
            "\"\"".to_string()
        } else {
            terms.join(" ")
        }
    }

    /// Imports quick links from TOML format.
    ///
    /// Conflicts are handled by updating existing links with matching URLs.
    pub async fn import_from_toml(&self, toml: QuickLinksToml) -> Result<usize> {
        let mut imported = 0;

        for toml_link in toml.links {
            let link: QuickLink = toml_link.into();

            // Check if link with same URL already exists
            let existing = self.find_by_url(&link.link).await?;

            if let Some(mut existing_link) = existing {
                // Update existing link
                existing_link.name = link.name;
                existing_link.keywords = link.keywords;
                existing_link.tags = link.tags;
                existing_link.icon = link.icon;
                existing_link.alias = link.alias;
                existing_link.hotkey = link.hotkey;
                existing_link.open_with = link.open_with;
                self.update(&existing_link).await?;
            } else {
                // Create new link
                self.store(&link).await?;
            }

            imported += 1;
        }

        Ok(imported)
    }

    /// Finds a link by URL.
    async fn find_by_url(&self, url: &str) -> Result<Option<QuickLink>> {
        let url = url.to_string();
        let storage = self.clone();

        Self::run_blocking(move || {
            let result = storage.conn.lock().query_row(
                "SELECT id, title, url, keywords, tags, icon_type, icon_value, open_with, alias, hotkey, created_at, accessed_at, access_count
                 FROM quick_links WHERE url = ?1",
                params![url],
                |row| {
                    let keywords_json: String = row.get(3)?;
                    let tags_json: String = row.get(4)?;
                    let keywords: Vec<String> =
                        serde_json::from_str(&keywords_json).unwrap_or_default();
                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                    let icon_type: String = row
                        .get::<_, Option<String>>(5)?
                        .unwrap_or_else(|| "default".to_string());
                    let icon_value: Option<String> = row.get(6)?;
                    let icon = Self::db_to_icon(&icon_type, icon_value.as_deref());

                    let open_with: Option<String> = row.get(7)?;
                    let alias: Option<String> = row.get(8)?;
                    let hotkey: Option<String> = row.get(9)?;

                    let created_at: i64 = row.get(10)?;
                    let accessed_at: Option<i64> = row.get(11)?;

                    Ok(QuickLink {
                        id: QuickLinkId::from(row.get::<_, i64>(0)?),
                        name: row.get(1)?,
                        link: row.get(2)?,
                        open_with,
                        icon,
                        alias,
                        hotkey,
                        keywords,
                        tags,
                        created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
                        accessed_at: accessed_at.and_then(|t| DateTime::from_timestamp(t, 0)),
                        access_count: row.get(12)?,
                    })
                },
            );

            match result {
                Ok(link) => Ok(Some(link)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
        .await
    }

    /// Clears all quick links.
    pub async fn clear_all(&self) -> Result<()> {
        let storage = self.clone();

        Self::run_blocking(move || {
            let conn = storage.conn.lock();
            conn.execute("DELETE FROM quick_links", [])?;
            drop(conn);
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_crud() {
        let storage = QuickLinksStorage::open_in_memory().unwrap();

        // Create
        let link = QuickLink::new("GitHub", "https://github.com")
            .with_keywords(vec!["gh".to_string(), "git".to_string()])
            .with_tags(vec!["dev".to_string()])
            .with_alias("gh")
            .with_icon(QuickLinkIcon::Emoji("🐙".to_string()));

        let id = storage.store(&link).await.unwrap();

        // Read
        let loaded = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "GitHub");
        assert_eq!(loaded.link, "https://github.com");
        assert_eq!(loaded.keywords, vec!["gh", "git"]);
        assert_eq!(loaded.alias, Some("gh".to_string()));
        assert_eq!(loaded.icon, QuickLinkIcon::Emoji("🐙".to_string()));

        // Update
        let mut updated = loaded.clone();
        updated.name = "GitHub Updated".to_string();
        updated.hotkey = Some("cmd+g".to_string());
        storage.update(&updated).await.unwrap();

        let loaded = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "GitHub Updated");
        assert_eq!(loaded.hotkey, Some("cmd+g".to_string()));

        // Delete
        storage.delete(&id).await.unwrap();
        let loaded = storage.get(&id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_storage_with_all_new_fields() {
        let storage = QuickLinksStorage::open_in_memory().unwrap();

        let link = QuickLink::new("Google Search", "https://google.com/search?q={argument}")
            .with_alias("g")
            .with_icon(QuickLinkIcon::SystemIcon("globe".to_string()))
            .with_open_with("com.apple.Safari")
            .with_hotkey("cmd+shift+g")
            .with_keywords(vec!["search".to_string()])
            .with_tags(vec!["web".to_string()]);

        let id = storage.store(&link).await.unwrap();
        let loaded = storage.get(&id).await.unwrap().unwrap();

        assert_eq!(loaded.name, "Google Search");
        assert_eq!(loaded.alias, Some("g".to_string()));
        assert_eq!(loaded.icon, QuickLinkIcon::SystemIcon("globe".to_string()));
        assert_eq!(loaded.open_with, Some("com.apple.Safari".to_string()));
        assert_eq!(loaded.hotkey, Some("cmd+shift+g".to_string()));
    }

    #[tokio::test]
    async fn test_search() {
        let storage = QuickLinksStorage::open_in_memory().unwrap();

        // Add test links
        let link1 = QuickLink::new("GitHub", "https://github.com").with_alias("gh");
        let link2 = QuickLink::new("GitLab", "https://gitlab.com");
        let link3 = QuickLink::new("Stack Overflow", "https://stackoverflow.com");

        storage.store(&link1).await.unwrap();
        storage.store(&link2).await.unwrap();
        storage.store(&link3).await.unwrap();

        // Search
        let results = storage.search("git").await.unwrap();
        assert_eq!(results.len(), 2);

        let results = storage.search("stack").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Stack Overflow");

        let results = storage.search("  ").await.unwrap();
        assert!(results.is_empty());

        // Search by alias
        let results = storage.search("gh").await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_toml_export_import() {
        let storage = QuickLinksStorage::open_in_memory().unwrap();

        // Add some links
        let link1 = QuickLink::new("GitHub", "https://github.com")
            .with_keywords(vec!["gh".to_string()])
            .with_alias("gh")
            .with_icon(QuickLinkIcon::Emoji("🐙".to_string()));
        let link2 = QuickLink::new("GitLab", "https://gitlab.com");

        storage.store(&link1).await.unwrap();
        storage.store(&link2).await.unwrap();

        // Export
        let toml = storage.export_to_toml().await.unwrap();
        assert_eq!(toml.links.len(), 2);

        // Clear and re-import
        storage.clear_all().await.unwrap();
        let count = storage.import_from_toml(toml).await.unwrap();
        assert_eq!(count, 2);

        // Verify
        let all = storage.load_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
