//! Clipboard storage with encrypted SQLite and FTS5 search.
//!
//! This module provides persistent storage for clipboard history items
//! with full-text search capabilities.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use tokio::task;

use crate::config::ClipboardConfig;
use crate::encryption::EncryptionManager;
use crate::error::{ClipboardError, Result};
use crate::models::{ClipboardContentType, ClipboardItem, ClipboardItemId};

/// Current schema version.
#[allow(dead_code)]
const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Clipboard storage with encrypted SQLite backend.
#[derive(Debug)]
pub struct ClipboardStorage {
    conn: Arc<Mutex<Connection>>,
    encryption: Arc<EncryptionManager>,
    config: ClipboardConfig,
    path: Option<PathBuf>,
}

/// Holds the prepared (and possibly encrypted) content fields for a clipboard
/// item, ready to be inserted into the SQLite `clipboard_items` table.
struct PreparedContent {
    /// Content type identifier (e.g. `"text"`, `"image"`, `"file"`, `"url"`, `"color"`).
    content_type: String,
    /// AES-256-GCM encrypted plain text, or `None` for non-text content.
    encrypted_text: Option<Vec<u8>>,
    /// AES-256-GCM encrypted HTML representation, if available.
    encrypted_html: Option<Vec<u8>>,
    /// AES-256-GCM encrypted RTF representation, if available.
    encrypted_rtf: Option<Vec<u8>>,
    /// Filesystem path to a saved image (PNG), or `None` for non-image content.
    image_path: Option<String>,
    /// Filesystem path to a thumbnail preview image.
    thumbnail_path: Option<String>,
    /// JSON-serialized list of file paths for file/folder clipboard items.
    file_paths: Option<String>,
    /// URL string for link-type clipboard items.
    url: Option<String>,
    /// Page title fetched from the URL's metadata.
    link_title: Option<String>,
    /// Filesystem path to the cached favicon for a URL item.
    favicon_path: Option<String>,
    /// Hex color code (e.g. `"#FF5733"`) for color-type items.
    color_hex: Option<String>,
    /// RGB string (e.g. `"rgb(255, 87, 51)"`) for color-type items.
    color_rgb: Option<String>,
    /// Human-readable color name (e.g. `"Coral"`) if recognized.
    color_name: Option<String>,
}

impl Clone for ClipboardStorage {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            encryption: Arc::clone(&self.encryption),
            config: self.config.clone(),
            path: self.path.clone(),
        }
    }
}

impl ClipboardStorage {
    /// Opens or creates a clipboard storage database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or encryption fails.
    pub fn open(config: &ClipboardConfig) -> Result<Self> {
        let path = config.database_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create storage directory")?;
        }

        // Also ensure images and thumbnails directories exist
        std::fs::create_dir_all(config.images_path())
            .context("failed to create images directory")?;
        std::fs::create_dir_all(config.thumbnails_path())
            .context("failed to create thumbnails directory")?;

        let conn = Connection::open(&path).context("failed to open database")?;

        // Enable WAL mode for better concurrent access
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let encryption = EncryptionManager::new()?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
            encryption: Arc::new(encryption),
            config: config.clone(),
            path: Some(path),
        };

        storage.run_migrations()?;

        Ok(storage)
    }

    /// Opens clipboard storage asynchronously.
    pub async fn open_async(config: ClipboardConfig) -> Result<Self> {
        Self::run_blocking(move || Self::open(&config)).await
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
    pub fn open_in_memory(config: &ClipboardConfig) -> Result<Self> {
        let conn = Connection::open_in_memory().context("failed to open in-memory database")?;

        let encryption = EncryptionManager::from_machine_id("test-machine-id")?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
            encryption: Arc::new(encryption),
            config: config.clone(),
            path: None,
        };

        storage.run_migrations()?;

        Ok(storage)
    }

    /// Runs database migrations.
    fn run_migrations(&self) -> Result<()> {
        let current_version = self.get_schema_version()?;

        if current_version < 1 {
            self.migrate_v1()?;
            self.record_version(1)?;
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

            -- Clipboard items
            CREATE TABLE IF NOT EXISTS clipboard_items (
                id TEXT PRIMARY KEY,
                content_type TEXT NOT NULL,
                
                -- Encrypted content fields
                text_content BLOB,
                html_content BLOB,
                rtf_content BLOB,
                
                -- Image/File paths (not encrypted, just references)
                image_path TEXT,
                thumbnail_path TEXT,
                file_paths TEXT,
                
                -- Link metadata
                url TEXT,
                link_title TEXT,
                favicon_path TEXT,
                
                -- Color data
                color_hex TEXT,
                color_rgb TEXT,
                color_name TEXT,
                
                -- Metadata
                source_app TEXT,
                source_bundle_id TEXT,
                size_bytes INTEGER NOT NULL DEFAULT 0,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                accessed_at INTEGER,
                
                -- Search text (plaintext for FTS)
                search_text TEXT NOT NULL DEFAULT ''
            );

            CREATE INDEX IF NOT EXISTS idx_clipboard_created 
            ON clipboard_items(created_at DESC);
            
            CREATE INDEX IF NOT EXISTS idx_clipboard_pinned 
            ON clipboard_items(is_pinned DESC, created_at DESC);

            -- FTS5 virtual table for full-text search
            CREATE VIRTUAL TABLE IF NOT EXISTS clipboard_fts USING fts5(
                search_text,
                content='clipboard_items',
                content_rowid='rowid'
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER IF NOT EXISTS clipboard_ai AFTER INSERT ON clipboard_items BEGIN
                INSERT INTO clipboard_fts(rowid, search_text) VALUES (NEW.rowid, NEW.search_text);
            END;

            CREATE TRIGGER IF NOT EXISTS clipboard_ad AFTER DELETE ON clipboard_items BEGIN
                INSERT INTO clipboard_fts(clipboard_fts, rowid, search_text) VALUES('delete', OLD.rowid, OLD.search_text);
            END;

            CREATE TRIGGER IF NOT EXISTS clipboard_au AFTER UPDATE ON clipboard_items BEGIN
                INSERT INTO clipboard_fts(clipboard_fts, rowid, search_text) VALUES('delete', OLD.rowid, OLD.search_text);
                INSERT INTO clipboard_fts(rowid, search_text) VALUES (NEW.rowid, NEW.search_text);
            END;
            ",
            )?;
        }
        Ok(())
    }

    /// Stores a clipboard item.
    pub fn store(&self, item: &ClipboardItem) -> Result<()> {
        let conn = self.conn.lock();

        // Prepare encrypted content based on type
        let prepared = self.prepare_content_for_storage(&item.content_type)?;

        let search_text = if self.config.store_search_text {
            item.search_text()
        } else {
            String::new()
        };

        let size_bytes = i64::try_from(item.size_bytes).map_err(|_| ClipboardError::Internal {
            message: "clipboard item size exceeds i64".to_string(),
        })?;

        conn.execute(
            r"
            INSERT OR REPLACE INTO clipboard_items (
                id, content_type,
                text_content, html_content, rtf_content,
                image_path, thumbnail_path, file_paths,
                url, link_title, favicon_path,
                color_hex, color_rgb, color_name,
                source_app, source_bundle_id, size_bytes, is_pinned,
                created_at, accessed_at, search_text
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            ",
            params![
                item.id.as_str(),
                prepared.content_type,
                prepared.encrypted_text,
                prepared.encrypted_html,
                prepared.encrypted_rtf,
                prepared.image_path,
                prepared.thumbnail_path,
                prepared.file_paths,
                prepared.url,
                prepared.link_title,
                prepared.favicon_path,
                prepared.color_hex,
                prepared.color_rgb,
                prepared.color_name,
                item.source_app,
                item.source_bundle_id,
                size_bytes,
                i32::from(item.is_pinned),
                item.created_at.timestamp(),
                item.accessed_at.map(|t| t.timestamp()),
                search_text,
            ],
        )?;

        // Enforce retention policy
        drop(conn);
        self.enforce_limits()?;

        Ok(())
    }

    /// Stores a clipboard item asynchronously.
    pub async fn store_async(&self, item: ClipboardItem) -> Result<()> {
        let storage = self.clone();
        Self::run_blocking(move || storage.store(&item)).await
    }

    /// Loads recent clipboard items.
    #[allow(clippy::significant_drop_tightening)]
    pub fn load_recent(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
        let items = {
            let conn = self.conn.lock();
            let mut stmt = conn.prepare(
                r"
            SELECT id, content_type,
                   text_content, html_content, rtf_content,
                   image_path, thumbnail_path, file_paths,
                   url, link_title, favicon_path,
                   color_hex, color_rgb, color_name,
                   source_app, source_bundle_id, size_bytes, is_pinned,
                   created_at, accessed_at
            FROM clipboard_items
            WHERE is_pinned = 0
            ORDER BY created_at DESC
            LIMIT ?1
            ",
            )?;

            let limit = i64::try_from(limit).map_err(|_| ClipboardError::Internal {
                message: "history limit exceeds i64".to_string(),
            })?;
            let items = stmt
                .query_map([limit], |row| self.row_to_item(row))?
                .filter_map(std::result::Result::ok)
                .collect();
            items
        };

        Ok(items)
    }

    /// Loads recent items asynchronously.
    pub async fn load_recent_async(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
        let storage = self.clone();
        Self::run_blocking(move || storage.load_recent(limit)).await
    }

    /// Loads pinned clipboard items.
    #[allow(clippy::significant_drop_tightening)]
    pub fn load_pinned(&self) -> Result<Vec<ClipboardItem>> {
        let items = {
            let conn = self.conn.lock();
            let mut stmt = conn.prepare(
                r"
            SELECT id, content_type,
                   text_content, html_content, rtf_content,
                   image_path, thumbnail_path, file_paths,
                   url, link_title, favicon_path,
                   color_hex, color_rgb, color_name,
                   source_app, source_bundle_id, size_bytes, is_pinned,
                   created_at, accessed_at
            FROM clipboard_items
            WHERE is_pinned = 1
            ORDER BY created_at DESC
            LIMIT 1000
            ",
            )?;

            let items = stmt
                .query_map([], |row| self.row_to_item(row))?
                .filter_map(std::result::Result::ok)
                .collect();
            items
        };

        Ok(items)
    }

    /// Loads pinned items asynchronously.
    pub async fn load_pinned_async(&self) -> Result<Vec<ClipboardItem>> {
        let storage = self.clone();
        Self::run_blocking(move || storage.load_pinned()).await
    }

    /// Searches clipboard history using FTS5.
    #[allow(clippy::significant_drop_tightening)]
    pub fn search(&self, query: &str) -> Result<Vec<ClipboardItem>> {
        if query.trim().is_empty() {
            return self.load_recent(50);
        }

        if !self.config.store_search_text {
            let mut items = self.load_pinned()?;
            items.extend(self.load_recent(self.config.history_size)?);

            let needle = query.to_lowercase();
            items.retain(|item| item.search_text().to_lowercase().contains(&needle));
            items.truncate(100);
            return Ok(items);
        }

        let items = {
            // Escape FTS5 special characters and prepare query
            let fts_query = prepare_fts_query(query);

            let conn = self.conn.lock();
            let mut stmt = conn.prepare(
                r"
            SELECT c.id, c.content_type,
                   c.text_content, c.html_content, c.rtf_content,
                   c.image_path, c.thumbnail_path, c.file_paths,
                   c.url, c.link_title, c.favicon_path,
                   c.color_hex, c.color_rgb, c.color_name,
                   c.source_app, c.source_bundle_id, c.size_bytes, c.is_pinned,
                   c.created_at, c.accessed_at
            FROM clipboard_items c
            JOIN clipboard_fts f ON c.rowid = f.rowid
            WHERE clipboard_fts MATCH ?1
            ORDER BY c.is_pinned DESC, rank
            LIMIT 100
            ",
            )?;

            let items = stmt
                .query_map([fts_query], |row| self.row_to_item(row))?
                .filter_map(std::result::Result::ok)
                .collect();
            items
        };

        Ok(items)
    }

    /// Searches asynchronously.
    pub async fn search_async(&self, query: String) -> Result<Vec<ClipboardItem>> {
        let storage = self.clone();
        Self::run_blocking(move || storage.search(&query)).await
    }

    /// Gets a specific item by ID.
    pub fn get(&self, id: &ClipboardItemId) -> Result<Option<ClipboardItem>> {
        let result = {
            let conn = self.conn.lock();
            conn.query_row(
                r"
            SELECT id, content_type,
                   text_content, html_content, rtf_content,
                   image_path, thumbnail_path, file_paths,
                   url, link_title, favicon_path,
                   color_hex, color_rgb, color_name,
                   source_app, source_bundle_id, size_bytes, is_pinned,
                   created_at, accessed_at
            FROM clipboard_items
            WHERE id = ?1
            ",
                [id.as_str()],
                |row| self.row_to_item(row),
            )
        };

        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Gets an item asynchronously.
    pub async fn get_async(&self, id: ClipboardItemId) -> Result<Option<ClipboardItem>> {
        let storage = self.clone();
        Self::run_blocking(move || storage.get(&id)).await
    }

    /// Pins or unpins an item.
    pub fn set_pinned(&self, id: &ClipboardItemId, pinned: bool) -> Result<bool> {
        let rows = {
            let conn = self.conn.lock();
            conn.execute(
                "UPDATE clipboard_items SET is_pinned = ?1 WHERE id = ?2",
                params![i32::from(pinned), id.as_str()],
            )?
        };

        Ok(rows > 0)
    }

    /// Sets pinned status asynchronously.
    pub async fn set_pinned_async(&self, id: ClipboardItemId, pinned: bool) -> Result<bool> {
        let storage = self.clone();
        Self::run_blocking(move || storage.set_pinned(&id, pinned)).await
    }

    /// Deletes an item.
    pub fn delete(&self, id: &ClipboardItemId) -> Result<bool> {
        // Get item first to clean up files
        if let Some(item) = self.get(id)? {
            Self::cleanup_item_files(&item);
        }

        let rows = {
            let conn = self.conn.lock();
            conn.execute("DELETE FROM clipboard_items WHERE id = ?1", [id.as_str()])?
        };

        Ok(rows > 0)
    }

    /// Deletes an item asynchronously.
    pub async fn delete_async(&self, id: ClipboardItemId) -> Result<bool> {
        let storage = self.clone();
        Self::run_blocking(move || storage.delete(&id)).await
    }

    /// Clears all clipboard history.
    pub fn clear_all(&self) -> Result<usize> {
        // Clean up all files
        let items = self.load_recent(10_000)?;
        let pinned = self.load_pinned()?;

        for item in items.iter().chain(pinned.iter()) {
            Self::cleanup_item_files(item);
        }

        let rows = {
            let conn = self.conn.lock();
            conn.execute("DELETE FROM clipboard_items", [])?
        };

        Ok(rows)
    }

    /// Clears all history asynchronously.
    pub async fn clear_all_async(&self) -> Result<usize> {
        let storage = self.clone();
        Self::run_blocking(move || storage.clear_all()).await
    }

    /// Updates accessed_at timestamp.
    pub fn mark_accessed(&self, id: &ClipboardItemId) -> Result<()> {
        let now = Utc::now().timestamp();

        {
            let conn = self.conn.lock();
            conn.execute(
                "UPDATE clipboard_items SET accessed_at = ?1 WHERE id = ?2",
                params![now, id.as_str()],
            )?;
        }

        Ok(())
    }

    /// Updates URL metadata (title and favicon path) for a URL item.
    pub fn update_url_metadata(
        &self,
        id: &ClipboardItemId,
        title: Option<&str>,
        favicon_path: Option<&std::path::Path>,
    ) -> Result<bool> {
        let conn = self.conn.lock();

        // Update the URL metadata using the existing columns
        let rows_affected = conn.execute(
            r"
            UPDATE clipboard_items 
            SET link_title = COALESCE(?1, link_title),
                favicon_path = COALESCE(?2, favicon_path)
            WHERE id = ?3 AND content_type = 'url'
            ",
            params![
                title,
                favicon_path.map(|p| p.to_string_lossy().to_string()),
                id.as_str()
            ],
        )?;

        Ok(rows_affected > 0)
    }

    /// Async version of `update_url_metadata`.
    pub async fn update_url_metadata_async(
        &self,
        id: ClipboardItemId,
        title: Option<String>,
        favicon_path: Option<std::path::PathBuf>,
    ) -> Result<bool> {
        let storage = self.clone();
        Self::run_blocking(move || {
            storage.update_url_metadata(&id, title.as_deref(), favicon_path.as_deref())
        })
        .await
    }

    /// Returns the total number of items.
    pub fn count(&self) -> Result<usize> {
        let count: i64 = {
            let conn = self.conn.lock();
            conn.query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))?
        };
        Ok(count as usize)
    }

    /// Enforces retention policy and size limits.
    fn enforce_limits(&self) -> Result<()> {
        // Remove items older than retention period (except pinned)
        let cutoff = Utc::now() - Duration::days(i64::from(self.config.retention_days));
        let cutoff_ts = cutoff.timestamp();

        {
            let conn = self.conn.lock();
            conn.execute(
                "DELETE FROM clipboard_items WHERE created_at < ?1 AND is_pinned = 0",
                [cutoff_ts],
            )?;

            // Enforce max items limit (except pinned)
            let max_items =
                i64::try_from(self.config.history_size).map_err(|_| ClipboardError::Internal {
                    message: "history size exceeds i64".to_string(),
                })?;
            conn.execute(
                r"
            DELETE FROM clipboard_items
            WHERE id IN (
                SELECT id FROM clipboard_items
                WHERE is_pinned = 0
                ORDER BY created_at DESC
                LIMIT -1 OFFSET ?1
            )
            ",
                [max_items],
            )?;
        }

        Ok(())
    }

    /// Cleans up files associated with an item.
    fn cleanup_item_files(item: &ClipboardItem) {
        match &item.content_type {
            ClipboardContentType::Image {
                path,
                thumbnail_path,
                ..
            } => {
                let _ = std::fs::remove_file(path);
                let _ = std::fs::remove_file(thumbnail_path);
            },
            ClipboardContentType::Link {
                favicon_path: Some(path),
                ..
            } => {
                let _ = std::fs::remove_file(path);
            },
            ClipboardContentType::File { icons, .. } => {
                for icon in icons {
                    let _ = std::fs::remove_file(icon);
                }
            },
            _ => {},
        }
    }

    /// Prepares content for storage, encrypting sensitive data.
    #[allow(clippy::too_many_lines)]
    fn prepare_content_for_storage(
        &self,
        content_type: &ClipboardContentType,
    ) -> Result<PreparedContent> {
        match content_type {
            ClipboardContentType::Text { content, .. } => {
                let encrypted = self.encryption.encrypt_string(content)?;
                Ok(PreparedContent {
                    content_type: "text".to_string(),
                    encrypted_text: Some(encrypted),
                    encrypted_html: None,
                    encrypted_rtf: None,
                    image_path: None,
                    thumbnail_path: None,
                    file_paths: None,
                    url: None,
                    link_title: None,
                    favicon_path: None,
                    color_hex: None,
                    color_rgb: None,
                    color_name: None,
                })
            },
            ClipboardContentType::RichText { plain, html, rtf } => {
                let encrypted_plain = self.encryption.encrypt_string(plain)?;
                let encrypted_html = html
                    .as_ref()
                    .map(|h| self.encryption.encrypt_string(h))
                    .transpose()?;
                let encrypted_rtf = rtf
                    .as_ref()
                    .map(|r| self.encryption.encrypt_string(r))
                    .transpose()?;
                Ok(PreparedContent {
                    content_type: "rich_text".to_string(),
                    encrypted_text: Some(encrypted_plain),
                    encrypted_html,
                    encrypted_rtf,
                    image_path: None,
                    thumbnail_path: None,
                    file_paths: None,
                    url: None,
                    link_title: None,
                    favicon_path: None,
                    color_hex: None,
                    color_rgb: None,
                    color_name: None,
                })
            },
            ClipboardContentType::Image {
                path,
                thumbnail_path,
                ..
            } => Ok(PreparedContent {
                content_type: "image".to_string(),
                encrypted_text: None,
                encrypted_html: None,
                encrypted_rtf: None,
                image_path: Some(path.to_string_lossy().to_string()),
                thumbnail_path: Some(thumbnail_path.to_string_lossy().to_string()),
                file_paths: None,
                url: None,
                link_title: None,
                favicon_path: None,
                color_hex: None,
                color_rgb: None,
                color_name: None,
            }),
            ClipboardContentType::File { paths, icons, .. } => {
                let paths_json = serde_json::to_string(paths)?;
                let icons_json = serde_json::to_string(icons)?;
                Ok(PreparedContent {
                    content_type: "file".to_string(),
                    encrypted_text: None,
                    encrypted_html: None,
                    encrypted_rtf: None,
                    image_path: None,
                    thumbnail_path: None,
                    file_paths: Some(paths_json),
                    url: None,
                    link_title: None,
                    favicon_path: Some(icons_json), // Store icons in favicon_path field
                    color_hex: None,
                    color_rgb: None,
                    color_name: None,
                })
            },
            ClipboardContentType::Link {
                url,
                title,
                favicon_path,
            } => Ok(PreparedContent {
                content_type: "link".to_string(),
                encrypted_text: None,
                encrypted_html: None,
                encrypted_rtf: None,
                image_path: None,
                thumbnail_path: None,
                file_paths: None,
                url: Some(url.clone()),
                link_title: title.clone(),
                favicon_path: favicon_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
                color_hex: None,
                color_rgb: None,
                color_name: None,
            }),
            ClipboardContentType::Color {
                hex,
                rgb,
                display_name,
            } => Ok(PreparedContent {
                content_type: "color".to_string(),
                encrypted_text: None,
                encrypted_html: None,
                encrypted_rtf: None,
                image_path: None,
                thumbnail_path: None,
                file_paths: None,
                url: None,
                link_title: None,
                favicon_path: None,
                color_hex: Some(hex.clone()),
                color_rgb: Some(format!("{},{},{}", rgb.0, rgb.1, rgb.2)),
                color_name: display_name.clone(),
            }),
        }
    }

    /// Converts a database row to a ClipboardItem.
    #[allow(clippy::too_many_lines)]
    fn row_to_item(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardItem> {
        let id: String = row.get(0)?;
        let content_type_str: String = row.get(1)?;

        // Encrypted content fields
        let text_content: Option<Vec<u8>> = row.get(2)?;
        let html_content: Option<Vec<u8>> = row.get(3)?;
        let rtf_content: Option<Vec<u8>> = row.get(4)?;

        // Path fields
        let image_path: Option<String> = row.get(5)?;
        let thumbnail_path: Option<String> = row.get(6)?;
        let file_paths: Option<String> = row.get(7)?;

        // Link fields
        let url: Option<String> = row.get(8)?;
        let link_title: Option<String> = row.get(9)?;
        let favicon_path: Option<String> = row.get(10)?;

        // Color fields
        let color_hex: Option<String> = row.get(11)?;
        let color_rgb: Option<String> = row.get(12)?;
        let color_name: Option<String> = row.get(13)?;

        // Metadata
        let source_app: Option<String> = row.get(14)?;
        let source_bundle_id: Option<String> = row.get(15)?;
        let size_bytes: i64 = row.get(16)?;
        let is_pinned: i32 = row.get(17)?;
        let created_at: i64 = row.get(18)?;
        let accessed_at: Option<i64> = row.get(19)?;

        // Reconstruct content type
        let content_type = match content_type_str.as_str() {
            "text" => {
                let content = text_content
                    .map(|c| self.encryption.decrypt_string(&c))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        )
                    })?
                    .unwrap_or_default();
                ClipboardContentType::text(content)
            },
            "rich_text" => {
                let plain = text_content
                    .map(|c| self.encryption.decrypt_string(&c))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        )
                    })?
                    .unwrap_or_default();
                let html = html_content
                    .map(|c| self.encryption.decrypt_string(&c))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        )
                    })?;
                let rtf = rtf_content
                    .map(|c| self.encryption.decrypt_string(&c))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        )
                    })?;
                ClipboardContentType::RichText { plain, html, rtf }
            },
            "image" => {
                ClipboardContentType::Image {
                    path: PathBuf::from(image_path.unwrap_or_default()),
                    thumbnail_path: PathBuf::from(thumbnail_path.unwrap_or_default()),
                    size_bytes: size_bytes as u64,
                    dimensions: (0, 0), // TODO: Store dimensions
                }
            },
            "file" => {
                let paths: Vec<PathBuf> = file_paths
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();
                let icons: Vec<PathBuf> = favicon_path
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();
                ClipboardContentType::File {
                    paths,
                    icons,
                    total_size: size_bytes as u64,
                }
            },
            "link" => ClipboardContentType::Link {
                url: url.unwrap_or_default(),
                title: link_title,
                favicon_path: favicon_path.map(PathBuf::from),
            },
            "color" => {
                let rgb = color_rgb
                    .and_then(|s| {
                        let parts: Vec<&str> = s.split(',').collect();
                        if parts.len() == 3 {
                            Some((
                                parts[0].parse().ok()?,
                                parts[1].parse().ok()?,
                                parts[2].parse().ok()?,
                            ))
                        } else {
                            None
                        }
                    })
                    .unwrap_or((0, 0, 0));
                ClipboardContentType::Color {
                    hex: color_hex.unwrap_or_default(),
                    rgb,
                    display_name: color_name,
                }
            },
            _ => {
                // Fallback to empty text
                ClipboardContentType::text("")
            },
        };

        Ok(ClipboardItem {
            id: ClipboardItemId::new(id),
            content_type,
            source_app,
            source_bundle_id,
            size_bytes: size_bytes as u64,
            is_pinned: is_pinned != 0,
            created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
            accessed_at: accessed_at.and_then(|t| DateTime::from_timestamp(t, 0)),
        })
    }

    /// Returns the database path.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns a reference to the configuration.
    #[must_use]
    pub const fn config(&self) -> &ClipboardConfig {
        &self.config
    }
}

/// Prepares a query string for FTS5.
fn prepare_fts_query(query: &str) -> String {
    // Escape special FTS5 characters and add prefix matching
    let escaped = query
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('*', "\\*")
        .replace(':', "\\:")
        .replace(['(', ')', '{', '}'], "");

    // Add prefix matching for partial words
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ClipboardConfig {
        ClipboardConfig::default()
    }

    #[test]
    fn test_storage_open_in_memory() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");
        assert!(storage.path().is_none());
    }

    #[test]
    fn test_store_and_load_text() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        let item = ClipboardItem::text("Hello, World!");
        storage.store(&item).expect("should store");

        let items = storage.load_recent(10).expect("should load");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text_content(), Some("Hello, World!"));
    }

    #[test]
    fn test_store_and_load_rich_text() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        let item = ClipboardItem::new(ClipboardContentType::RichText {
            plain: "Hello".to_string(),
            html: Some("<b>Hello</b>".to_string()),
            rtf: None,
        });
        storage.store(&item).expect("should store");

        let items = storage.load_recent(10).expect("should load");
        assert_eq!(items.len(), 1);
        if let ClipboardContentType::RichText { plain, html, .. } = &items[0].content_type {
            assert_eq!(plain, "Hello");
            assert_eq!(html.as_deref(), Some("<b>Hello</b>"));
        } else {
            panic!("Expected RichText");
        }
    }

    #[test]
    fn test_store_and_load_link() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        let item = ClipboardItem::new(ClipboardContentType::Link {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            favicon_path: None,
        });
        storage.store(&item).expect("should store");

        let items = storage.load_recent(10).expect("should load");
        assert_eq!(items.len(), 1);
        if let ClipboardContentType::Link { url, title, .. } = &items[0].content_type {
            assert_eq!(url, "https://example.com");
            assert_eq!(title.as_deref(), Some("Example"));
        } else {
            panic!("Expected Link");
        }
    }

    #[test]
    fn test_store_and_load_color() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        let item = ClipboardItem::new(ClipboardContentType::Color {
            hex: "#FF5733".to_string(),
            rgb: (255, 87, 51),
            display_name: Some("Orange".to_string()),
        });
        storage.store(&item).expect("should store");

        let items = storage.load_recent(10).expect("should load");
        assert_eq!(items.len(), 1);
        if let ClipboardContentType::Color {
            hex,
            rgb,
            display_name,
        } = &items[0].content_type
        {
            assert_eq!(hex, "#FF5733");
            assert_eq!(*rgb, (255, 87, 51));
            assert_eq!(display_name.as_deref(), Some("Orange"));
        } else {
            panic!("Expected Color");
        }
    }

    #[test]
    fn test_pin_unpin() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        let item = ClipboardItem::text("Test");
        storage.store(&item).expect("should store");

        // Pin
        storage.set_pinned(&item.id, true).expect("should pin");
        let pinned = storage.load_pinned().expect("should load pinned");
        assert_eq!(pinned.len(), 1);

        // Unpin
        storage.set_pinned(&item.id, false).expect("should unpin");
        let pinned = storage.load_pinned().expect("should load pinned");
        assert!(pinned.is_empty());
    }

    #[test]
    fn test_search() {
        let mut config = create_test_config();
        config.store_search_text = true;
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        // Store multiple items
        storage
            .store(&ClipboardItem::text("Apple iPhone"))
            .expect("should store");
        storage
            .store(&ClipboardItem::text("Samsung Galaxy"))
            .expect("should store");
        storage
            .store(&ClipboardItem::text("Google Pixel"))
            .expect("should store");

        // Search
        let results = storage.search("apple").expect("should search");
        assert_eq!(results.len(), 1);
        assert!(results[0].text_content().unwrap().contains("Apple"));

        let results = storage.search("galaxy").expect("should search");
        assert_eq!(results.len(), 1);

        // Empty search returns recent
        let results = storage.search("").expect("should search");
        assert_eq!(results.len(), 3);

        let results = storage.search("  ").expect("should search");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_with_plaintext_disabled() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        storage
            .store(&ClipboardItem::text("Apple iPhone"))
            .expect("should store");

        let results = storage.search("apple").expect("should search");
        assert_eq!(results.len(), 1);
        assert!(results[0].text_content().unwrap().contains("Apple"));

        let results = storage.search("   ").expect("should search");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_delete() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        let item = ClipboardItem::text("Test");
        let id = item.id.clone();
        storage.store(&item).expect("should store");

        assert_eq!(storage.count().expect("count"), 1);

        storage.delete(&id).expect("should delete");
        assert_eq!(storage.count().expect("count"), 0);
    }

    #[test]
    fn test_clear_all() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        storage
            .store(&ClipboardItem::text("Item 1"))
            .expect("should store");
        storage
            .store(&ClipboardItem::text("Item 2"))
            .expect("should store");
        storage
            .store(&ClipboardItem::text("Item 3"))
            .expect("should store");

        assert_eq!(storage.count().expect("count"), 3);

        storage.clear_all().expect("should clear");
        assert_eq!(storage.count().expect("count"), 0);
    }

    #[test]
    fn test_enforce_limits() {
        let mut config = create_test_config();
        config.history_size = 3;
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        // Store more items than the limit
        for i in 0..5 {
            storage
                .store(&ClipboardItem::text(format!("Item {}", i)))
                .expect("should store");
        }

        // Should only have 3 items (the most recent)
        let items = storage.load_recent(10).expect("should load");
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_get_by_id() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        let item = ClipboardItem::text("Test");
        let id = item.id.clone();
        storage.store(&item).expect("should store");

        let retrieved = storage.get(&id).expect("should get").expect("should exist");
        assert_eq!(retrieved.text_content(), Some("Test"));

        // Non-existent ID
        let non_existent = storage
            .get(&ClipboardItemId::new("non-existent"))
            .expect("should query");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_encryption_roundtrip() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        // Store text with special characters
        let text = "Hello 🦀 Rust! Special chars: <>&\"'";
        storage
            .store(&ClipboardItem::text(text))
            .expect("should store");

        let items = storage.load_recent(1).expect("should load");
        assert_eq!(items[0].text_content(), Some(text));
    }

    #[test]
    fn test_prepare_fts_query() {
        assert_eq!(prepare_fts_query("hello"), "\"hello\"*");
        assert_eq!(prepare_fts_query("hello world"), "\"hello\"* \"world\"*");
        assert_eq!(prepare_fts_query("test:query"), "\"test\\:query\"*");
        assert_eq!(prepare_fts_query("  "), "\"\"");
    }

    #[tokio::test]
    async fn test_async_operations() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");

        // Store async
        let item = ClipboardItem::text("Async test");
        storage
            .store_async(item.clone())
            .await
            .expect("should store");

        // Load async
        let items = storage.load_recent_async(10).await.expect("should load");
        assert_eq!(items.len(), 1);

        // Search async
        let results = storage
            .search_async("Async".to_string())
            .await
            .expect("should search");
        assert_eq!(results.len(), 1);
    }
}
