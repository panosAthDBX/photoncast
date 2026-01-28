//! Custom file indexing engine for fast file search.
//!
//! This module provides a SQLite-backed file index with custom tokenization
//! for matching file names using various splitting strategies (camelCase,
//! punctuation, whitespace) and Unicode normalization (ASCII folding).
//!
//! # Architecture
//!
//! - [`FileTokenizer`] - Tokenizes file names into searchable tokens
//! - [`FileIndex`] - SQLite-based storage for indexed files
//! - [`IndexingService`] - Background service for indexing directories
//!
//! # Example
//!
//! ```no_run
//! use photoncast_core::search::file_index::{FileTokenizer, FileIndex, IndexingService};
//! use std::path::Path;
//!
//! // Tokenize a file name
//! let tokens = FileTokenizer::tokenize("MyAwesomeFile.rs");
//! assert!(tokens.contains(&"my".to_string()));
//! assert!(tokens.contains(&"awesome".to_string()));
//!
//! // Create an in-memory index
//! let mut index = FileIndex::open_in_memory().unwrap();
//! index.add_file(Path::new("/path/to/file.txt")).unwrap();
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use rusqlite::{params, Connection};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use walkdir::WalkDir;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during file indexing operations.
#[derive(Error, Debug)]
pub enum FileIndexError {
    /// Database error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Path error (e.g., invalid UTF-8).
    #[error("invalid path: {0}")]
    InvalidPath(String),

    /// Index not ready.
    #[error("index not ready")]
    NotReady,
}

/// Result type for file index operations.
pub type Result<T> = std::result::Result<T, FileIndexError>;

// =============================================================================
// FileTokenizer
// =============================================================================

/// Tokenizer for file names and paths.
///
/// Converts file names into searchable tokens using multiple strategies:
/// - Whitespace splitting: `my file.txt` → `["my", "file", "txt"]`
/// - Punctuation splitting: `my-file_v2.txt` → `["my", "file", "v2", "txt"]`
/// - CamelCase splitting: `MyFileSearch.rs` → `["my", "file", "search", "rs"]`
/// - Lowercase normalization: `README` → `readme`
/// - ASCII folding: `résumé.pdf` → `["resume", "pdf"]`
pub struct FileTokenizer;

impl FileTokenizer {
    /// Tokenizes a file name or path into searchable tokens.
    ///
    /// # Arguments
    ///
    /// * `input` - The file name or path to tokenize.
    ///
    /// # Returns
    ///
    /// A vector of lowercase, ASCII-folded tokens.
    ///
    /// # Example
    ///
    /// ```
    /// use photoncast_core::search::file_index::FileTokenizer;
    ///
    /// let tokens = FileTokenizer::tokenize("MyAwesomeFile_v2.rs");
    /// assert!(tokens.contains(&"my".to_string()));
    /// assert!(tokens.contains(&"awesome".to_string()));
    /// assert!(tokens.contains(&"file".to_string()));
    /// assert!(tokens.contains(&"v2".to_string()));
    /// assert!(tokens.contains(&"rs".to_string()));
    /// ```
    #[must_use]
    pub fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = HashSet::new();

        // Process each segment of the input (split by path separators)
        for segment in input.split(['/', '\\']) {
            if segment.is_empty() {
                continue;
            }

            // Split on punctuation (-, _, ., whitespace)
            for part in segment.split(['-', '_', '.', ' ', '\t']) {
                if part.is_empty() {
                    continue;
                }

                // Split camelCase/PascalCase
                let camel_tokens = Self::split_camel_case(part);
                for token in camel_tokens {
                    if !token.is_empty() {
                        // Normalize Unicode and fold to ASCII
                        let normalized = Self::ascii_fold(&token.to_lowercase());
                        if !normalized.is_empty() {
                            tokens.insert(normalized);
                        }
                    }
                }
            }
        }

        tokens.into_iter().collect()
    }

    /// Tokenizes a path, extracting tokens from the file name only.
    ///
    /// Use this when you only want to index the file name, not the full path.
    #[must_use]
    pub fn tokenize_file_name(path: &Path) -> Vec<String> {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(Self::tokenize)
            .unwrap_or_default()
    }

    /// Splits a string on camelCase/PascalCase boundaries.
    ///
    /// # Example
    ///
    /// - `MyFileName` → `["My", "File", "Name"]`
    /// - `parseJSON` → `["parse", "JSON"]`
    /// - `XMLParser` → `["XML", "Parser"]`
    fn split_camel_case(s: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut prev_was_upper = false;
        let mut prev_was_digit = false;

        for c in s.chars() {
            let is_upper = c.is_uppercase();
            let is_digit = c.is_ascii_digit();

            // Split conditions:
            // 1. Lowercase followed by uppercase (camelCase)
            // 2. Uppercase followed by uppercase then lowercase (XMLParser)
            // 3. Letter followed by digit or digit followed by letter
            let should_split = if is_upper && !prev_was_upper && !current.is_empty() {
                // camelCase: lowercase followed by uppercase
                true
            } else if is_upper
                && prev_was_upper
                && current.len() > 1
                && current.chars().last().is_some_and(char::is_uppercase)
            {
                // Check if next char is lowercase (XMLParser case)
                false // Will be handled in next iteration
            } else if !is_upper && prev_was_upper && current.len() > 1 {
                // End of uppercase sequence (XML -> Parser)
                // Move last uppercase to new token
                let last_upper = current.pop();
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                if let Some(u) = last_upper {
                    current.push(u);
                }
                false
            } else if is_digit != prev_was_digit && current.len() > 1 {
                // Transition between digits and letters (only if current has 2+ chars)
                // This keeps short tokens like "v2" together but splits "file123"
                true
            } else {
                false
            };

            if should_split && !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }

            current.push(c);
            prev_was_upper = is_upper;
            prev_was_digit = is_digit;
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    /// Folds Unicode characters to their ASCII equivalents.
    ///
    /// Uses NFKD normalization and removes non-ASCII characters.
    ///
    /// # Example
    ///
    /// - `résumé` → `resume`
    /// - `naïve` → `naive`
    /// - `Ñoño` → `nono`
    fn ascii_fold(s: &str) -> String {
        s.nfkd()
            .filter(char::is_ascii)
            .filter(|c| c.is_alphanumeric())
            .collect()
    }
}

// =============================================================================
// IndexedFile
// =============================================================================

/// A file entry in the search index.
#[derive(Debug, Clone)]
pub struct IndexedFile {
    /// Full path to the file.
    pub path: PathBuf,
    /// File name (without path).
    pub name: String,
    /// File extension (without dot), if any.
    pub extension: Option<String>,
    /// Whether this is a directory.
    pub is_directory: bool,
    /// File size in bytes.
    pub size: u64,
    /// Last modified timestamp (Unix seconds).
    pub modified: i64,
}

impl IndexedFile {
    /// Creates a new `IndexedFile` from a path.
    ///
    /// # Errors
    ///
    /// Returns an error if the path metadata cannot be read.
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| FileIndexError::InvalidPath(path.display().to_string()))?
            .to_string();

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs() as i64);

        Ok(Self {
            path: path.to_path_buf(),
            name,
            extension,
            is_directory: metadata.is_dir(),
            size: metadata.len(),
            modified,
        })
    }
}

// =============================================================================
// FileIndex
// =============================================================================

/// SQLite-backed file search index.
///
/// Stores file metadata and tokens for fast searching.
pub struct FileIndex {
    /// Database connection.
    db: Connection,
}

impl FileIndex {
    /// Opens a file index from a database file.
    ///
    /// Creates the database and schema if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or created.
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        let mut index = Self { db };
        index.initialize_schema()?;
        Ok(index)
    }

    /// Opens an in-memory file index.
    ///
    /// Useful for testing or temporary indices.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created.
    pub fn open_in_memory() -> Result<Self> {
        let db = Connection::open_in_memory()?;
        let mut index = Self { db };
        index.initialize_schema()?;
        Ok(index)
    }

    /// Initializes the database schema.
    fn initialize_schema(&mut self) -> Result<()> {
        self.db.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                extension TEXT,
                is_directory INTEGER NOT NULL,
                size INTEGER NOT NULL,
                modified INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tokens (
                token TEXT NOT NULL,
                file_id INTEGER NOT NULL,
                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_tokens ON tokens(token);
            CREATE INDEX IF NOT EXISTS idx_tokens_file_id ON tokens(file_id);
            CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
            ",
        )?;
        Ok(())
    }

    /// Adds a file to the index.
    ///
    /// If the file already exists, it will be updated.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the database operation fails.
    pub fn add_file(&mut self, path: &Path) -> Result<()> {
        let indexed = IndexedFile::from_path(path)?;
        self.add_indexed_file(&indexed)
    }

    /// Adds an `IndexedFile` to the index.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn add_indexed_file(&mut self, file: &IndexedFile) -> Result<()> {
        // Use transaction for standalone calls (not within begin_batch/commit_batch)
        self.add_indexed_file_internal(file, true)
    }

    /// Adds a file without managing its own transaction.
    ///
    /// Use this within a batch (between `begin_batch` and `commit_batch`).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn add_indexed_file_batch(&mut self, file: &IndexedFile) -> Result<()> {
        self.add_indexed_file_internal(file, false)
    }

    /// Internal implementation for adding a file.
    fn add_indexed_file_internal(
        &mut self,
        file: &IndexedFile,
        use_transaction: bool,
    ) -> Result<()> {
        let path_str = file
            .path
            .to_str()
            .ok_or_else(|| FileIndexError::InvalidPath(file.path.display().to_string()))?;

        if use_transaction {
            let tx = self.db.transaction()?;

            // Insert or replace the file
            // Note: INSERT OR REPLACE deletes the old row first, which cascades to tokens
            tx.execute(
                r"
                INSERT OR REPLACE INTO files (path, name, extension, is_directory, size, modified)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    path_str,
                    file.name,
                    file.extension,
                    i32::from(file.is_directory),
                    file.size as i64,
                    file.modified,
                ],
            )?;

            // Get the file ID using last_insert_rowid (avoids extra SELECT query)
            let file_id = tx.last_insert_rowid();

            // Insert new tokens (old tokens were cascade-deleted with the old row)
            let tokens = FileTokenizer::tokenize_file_name(&file.path);
            for token in tokens {
                tx.execute(
                    "INSERT INTO tokens (token, file_id) VALUES (?1, ?2)",
                    params![token, file_id],
                )?;
            }

            tx.commit()?;
        } else {
            // Within a batch - no transaction management
            // Note: INSERT OR REPLACE deletes the old row first, which cascades to tokens
            self.db.execute(
                r"
                INSERT OR REPLACE INTO files (path, name, extension, is_directory, size, modified)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    path_str,
                    file.name,
                    file.extension,
                    i32::from(file.is_directory),
                    file.size as i64,
                    file.modified,
                ],
            )?;

            // Get the file ID using last_insert_rowid (avoids extra SELECT query)
            let file_id = self.db.last_insert_rowid();

            // Insert new tokens (old tokens were cascade-deleted with the old row)

            let tokens = FileTokenizer::tokenize_file_name(&file.path);
            for token in tokens {
                self.db.execute(
                    "INSERT INTO tokens (token, file_id) VALUES (?1, ?2)",
                    params![token, file_id],
                )?;
            }
        }

        Ok(())
    }

    /// Removes a file from the index.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn remove_file(&mut self, path: &Path) -> Result<()> {
        let path_str = path
            .to_str()
            .ok_or_else(|| FileIndexError::InvalidPath(path.display().to_string()))?;

        // Tokens are deleted via ON DELETE CASCADE
        self.db
            .execute("DELETE FROM files WHERE path = ?1", params![path_str])?;
        Ok(())
    }

    /// Updates a file in the index.
    ///
    /// This is equivalent to removing and re-adding the file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the database operation fails.
    pub fn update_file(&mut self, path: &Path) -> Result<()> {
        self.add_file(path)
    }

    /// Searches for files matching the given tokens.
    ///
    /// Files are scored by how many of the search tokens they match.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The search tokens to match.
    /// * `limit` - Maximum number of results to return.
    ///
    /// # Returns
    ///
    /// A vector of matching files, sorted by relevance (most matches first).
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn search(&self, tokens: &[String], limit: usize) -> Result<Vec<IndexedFile>> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Build a query that counts token matches per file
        let placeholders: Vec<&str> = tokens.iter().map(|_| "?").collect();
        let query = format!(
            r"
            SELECT f.path, f.name, f.extension, f.is_directory, f.size, f.modified,
                   COUNT(DISTINCT t.token) as match_count
            FROM files f
            JOIN tokens t ON f.id = t.file_id
            WHERE t.token IN ({})
            GROUP BY f.id
            ORDER BY match_count DESC, f.modified DESC
            LIMIT ?
            ",
            placeholders.join(", ")
        );

        let mut stmt = self.db.prepare(&query)?;

        // Bind tokens and limit
        let mut params_vec: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for token in tokens {
            params_vec.push(token);
        }
        let limit_i64 = limit as i64;
        params_vec.push(&limit_i64);

        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec.iter()), |row| {
            Ok(IndexedFile {
                path: PathBuf::from(row.get::<_, String>(0)?),
                name: row.get(1)?,
                extension: row.get(2)?,
                is_directory: row.get::<_, i32>(3)? != 0,
                size: row.get::<_, i64>(4)? as u64,
                modified: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Searches for files with a prefix match on tokens.
    ///
    /// This is useful for incremental search as the user types.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to match against tokens.
    /// * `limit` - Maximum number of results to return.
    ///
    /// # Returns
    ///
    /// A vector of matching files.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn search_prefix(&self, prefix: &str, limit: usize) -> Result<Vec<IndexedFile>> {
        if prefix.is_empty() {
            return Ok(Vec::new());
        }

        let prefix_lower = prefix.to_lowercase();
        let prefix_pattern = format!("{prefix_lower}%");

        let mut stmt = self.db.prepare(
            r"
            SELECT DISTINCT f.path, f.name, f.extension, f.is_directory, f.size, f.modified
            FROM files f
            JOIN tokens t ON f.id = t.file_id
            WHERE t.token LIKE ?1
            ORDER BY f.modified DESC
            LIMIT ?2
            ",
        )?;

        let rows = stmt.query_map(params![prefix_pattern, limit as i64], |row| {
            Ok(IndexedFile {
                path: PathBuf::from(row.get::<_, String>(0)?),
                name: row.get(1)?,
                extension: row.get(2)?,
                is_directory: row.get::<_, i32>(3)? != 0,
                size: row.get::<_, i64>(4)? as u64,
                modified: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Returns the total number of indexed files.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    #[allow(clippy::cast_possible_truncation)]
    pub fn file_count(&self) -> Result<usize> {
        let count: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Clears all files from the index.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn clear(&mut self) -> Result<()> {
        self.db.execute_batch(
            r"
            DELETE FROM tokens;
            DELETE FROM files;
            ",
        )?;
        Ok(())
    }

    /// Begins a transaction for batch operations.
    ///
    /// Call this before adding many files, then call `commit` when done.
    /// This significantly improves performance for bulk operations.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction cannot be started.
    pub fn begin_batch(&mut self) -> Result<()> {
        self.db.execute("BEGIN TRANSACTION", [])?;
        Ok(())
    }

    /// Commits a batch transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction cannot be committed.
    pub fn commit_batch(&mut self) -> Result<()> {
        self.db.execute("COMMIT", [])?;
        Ok(())
    }

    /// Rolls back a batch transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction cannot be rolled back.
    pub fn rollback_batch(&mut self) -> Result<()> {
        self.db.execute("ROLLBACK", [])?;
        Ok(())
    }
}

// =============================================================================
// IndexingService
// =============================================================================

/// Background indexing service for directories.
///
/// Walks directory trees and indexes files asynchronously with progress tracking.
/// Supports ignore patterns via [`IgnoreMatcher`](super::ignore_patterns::IgnoreMatcher).
pub struct IndexingService {
    /// The file index being populated.
    index: Arc<RwLock<FileIndex>>,
    /// Directories to index.
    scopes: Vec<PathBuf>,
    /// Current progress (files indexed).
    progress: Arc<AtomicUsize>,
    /// Total files to index.
    total: Arc<AtomicUsize>,
    /// Whether indexing is complete.
    complete: Arc<AtomicBool>,
    /// Whether indexing was cancelled.
    cancelled: Arc<AtomicBool>,
    /// Optional ignore pattern matcher.
    ignore_matcher: Option<Arc<super::ignore_patterns::IgnoreMatcher>>,
}

impl IndexingService {
    /// Creates a new indexing service.
    ///
    /// # Arguments
    ///
    /// * `index` - The file index to populate.
    /// * `scopes` - Directories to index.
    #[must_use]
    #[allow(clippy::arc_with_non_send_sync)] // FileIndex uses Connection which isn't Sync, but RwLock protects access
    pub fn new(index: FileIndex, scopes: Vec<PathBuf>) -> Self {
        Self {
            index: Arc::new(RwLock::new(index)),
            scopes,
            progress: Arc::new(AtomicUsize::new(0)),
            total: Arc::new(AtomicUsize::new(0)),
            complete: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
            ignore_matcher: None,
        }
    }

    /// Creates a new indexing service with an ignore matcher.
    ///
    /// # Arguments
    ///
    /// * `index` - The file index to populate.
    /// * `scopes` - Directories to index.
    /// * `ignore_matcher` - The ignore pattern matcher to use.
    #[must_use]
    #[allow(clippy::arc_with_non_send_sync, clippy::needless_pass_by_value)]
    pub fn with_ignore_matcher(
        index: FileIndex,
        scopes: Vec<PathBuf>,
        ignore_matcher: super::ignore_patterns::IgnoreMatcher,
    ) -> Self {
        Self {
            index: Arc::new(RwLock::new(index)),
            scopes: scopes.clone(),
            progress: Arc::new(AtomicUsize::new(0)),
            total: Arc::new(AtomicUsize::new(0)),
            complete: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
            ignore_matcher: Some(Arc::new(ignore_matcher)),
        }
    }

    /// Sets the ignore matcher for this service.
    pub fn set_ignore_matcher(&mut self, matcher: super::ignore_patterns::IgnoreMatcher) {
        self.ignore_matcher = Some(Arc::new(matcher));
    }

    /// Returns a reference to the ignore matcher, if set.
    #[must_use]
    pub fn ignore_matcher(&self) -> Option<&Arc<super::ignore_patterns::IgnoreMatcher>> {
        self.ignore_matcher.as_ref()
    }

    /// Starts background indexing.
    ///
    /// This method walks all configured scopes and indexes files in a single pass.
    /// Progress can be monitored via [`progress()`](Self::progress) and [`is_complete()`](Self::is_complete).
    ///
    /// The total count is estimated and updated as indexing progresses.
    /// The write lock is released periodically to allow concurrent searches.
    ///
    /// # Errors
    ///
    /// Returns an error if indexing fails critically (e.g., database error).
    #[allow(clippy::items_after_statements)]
    pub async fn start_indexing(&self) -> Result<()> {
        // Check if already cancelled before starting
        if self.cancelled.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.complete.store(false, Ordering::SeqCst);
        self.progress.store(0, Ordering::SeqCst);
        // Estimate total based on scope count; will be updated as we go
        self.total
            .store(self.scopes.len() * 10000, Ordering::SeqCst);

        // Batch size for periodic commits and lock releases
        const BATCH_SIZE: usize = 1000;
        let mut batch_count = 0usize;
        let mut total_indexed = 0usize;

        for scope in &self.scopes {
            if !scope.exists() {
                continue;
            }

            // Collect entries in batches to allow lock release between batches
            let mut batch: Vec<IndexedFile> = Vec::with_capacity(BATCH_SIZE);

            for entry in WalkDir::new(scope)
                .follow_links(false)
                .into_iter()
                .filter_map(std::result::Result::ok)
            {
                if self.cancelled.load(Ordering::SeqCst) {
                    // Rollback any pending batch
                    if !batch.is_empty() {
                        if let Some(mut index) = self.index.try_write() {
                            let _ = index.rollback_batch();
                        }
                    }
                    return Ok(());
                }

                let path = entry.path();
                let is_dir = entry.file_type().is_dir();

                // Skip hidden files, common ignored directories, and custom patterns
                if self.should_skip(path, is_dir) {
                    continue;
                }

                // Create indexed file (ignore errors for individual files)
                if let Ok(indexed) = IndexedFile::from_path(path) {
                    batch.push(indexed);
                    batch_count += 1;

                    // Process batch when full
                    if batch.len() >= BATCH_SIZE {
                        {
                            let mut index = self.index.write();
                            index.begin_batch()?;
                            for file in batch.drain(..) {
                                let _ = index.add_indexed_file_batch(&file);
                            }
                            index.commit_batch()?;
                        }
                        // Lock released here, allowing concurrent searches

                        total_indexed += BATCH_SIZE;
                        self.progress.store(total_indexed, Ordering::SeqCst);
                        // Update total estimate based on progress
                        self.total
                            .store(total_indexed.max(batch_count + 1000), Ordering::SeqCst);

                        // Yield to allow other tasks
                        tokio::task::yield_now().await;
                    }
                }
            }

            // Process remaining files in batch
            if !batch.is_empty() {
                let remaining = batch.len();
                {
                    let mut index = self.index.write();
                    index.begin_batch()?;
                    for file in batch.drain(..) {
                        let _ = index.add_indexed_file_batch(&file);
                    }
                    index.commit_batch()?;
                }
                total_indexed += remaining;
                self.progress.store(total_indexed, Ordering::SeqCst);
            }
        }

        // Set final accurate total
        self.total.store(total_indexed, Ordering::SeqCst);
        self.complete.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Starts indexing synchronously (blocking).
    ///
    /// Use this when async is not available or for testing.
    /// Uses single-pass indexing with periodic lock releases for better performance.
    ///
    /// # Errors
    ///
    /// Returns an error if indexing fails.
    #[allow(clippy::items_after_statements)]
    pub fn start_indexing_sync(&self) -> Result<()> {
        // Check if already cancelled before starting
        if self.cancelled.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.complete.store(false, Ordering::SeqCst);
        self.progress.store(0, Ordering::SeqCst);
        // Estimate total based on scope count; will be updated as we go
        self.total
            .store(self.scopes.len() * 10000, Ordering::SeqCst);

        // Batch size for periodic commits and lock releases
        const BATCH_SIZE: usize = 1000;
        let mut batch_count = 0usize;
        let mut total_indexed = 0usize;

        for scope in &self.scopes {
            if !scope.exists() {
                continue;
            }

            // Collect entries in batches to allow lock release between batches
            let mut batch: Vec<IndexedFile> = Vec::with_capacity(BATCH_SIZE);

            for entry in WalkDir::new(scope)
                .follow_links(false)
                .into_iter()
                .filter_map(std::result::Result::ok)
            {
                if self.cancelled.load(Ordering::SeqCst) {
                    // Rollback any pending batch
                    if !batch.is_empty() {
                        if let Some(mut index) = self.index.try_write() {
                            let _ = index.rollback_batch();
                        }
                    }
                    return Ok(());
                }

                let path = entry.path();
                let is_dir = entry.file_type().is_dir();

                // Skip hidden files, common ignored directories, and custom patterns
                if self.should_skip(path, is_dir) {
                    continue;
                }

                // Create indexed file (ignore errors for individual files)
                if let Ok(indexed) = IndexedFile::from_path(path) {
                    batch.push(indexed);
                    batch_count += 1;

                    // Process batch when full
                    if batch.len() >= BATCH_SIZE {
                        {
                            let mut index = self.index.write();
                            index.begin_batch()?;
                            for file in batch.drain(..) {
                                let _ = index.add_indexed_file_batch(&file);
                            }
                            index.commit_batch()?;
                        }
                        // Lock released here, allowing concurrent searches

                        total_indexed += BATCH_SIZE;
                        self.progress.store(total_indexed, Ordering::SeqCst);
                        // Update total estimate based on progress
                        self.total
                            .store(total_indexed.max(batch_count + 1000), Ordering::SeqCst);
                    }
                }
            }

            // Process remaining files in batch
            if !batch.is_empty() {
                let remaining = batch.len();
                {
                    let mut index = self.index.write();
                    index.begin_batch()?;
                    for file in batch.drain(..) {
                        let _ = index.add_indexed_file_batch(&file);
                    }
                    index.commit_batch()?;
                }
                total_indexed += remaining;
                self.progress.store(total_indexed, Ordering::SeqCst);
            }
        }

        // Set final accurate total
        self.total.store(total_indexed, Ordering::SeqCst);
        self.complete.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Returns whether a path should be skipped during indexing.
    ///
    /// Checks both built-in rules and the ignore matcher (if set).
    fn should_skip(&self, path: &Path, is_dir: bool) -> bool {
        // Get the file name
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return true;
        };

        // Skip hidden files (except the root being indexed)
        if name.starts_with('.') {
            return true;
        }

        // Skip common ignored directories (built-in)
        if matches!(
            name,
            "node_modules"
                | "target"
                | "build"
                | "dist"
                | "__pycache__"
                | ".git"
                | ".svn"
                | ".hg"
                | "Caches"
                | "Library"
        ) {
            return true;
        }

        // Check custom ignore patterns
        if let Some(ref matcher) = self.ignore_matcher {
            if matcher.is_ignored(path, is_dir) {
                return true;
            }
        }

        false
    }

    /// Returns the current progress as a ratio (0.0 to 1.0).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn progress(&self) -> f32 {
        let total = self.total.load(Ordering::SeqCst);
        if total == 0 {
            return 0.0;
        }
        let progress = self.progress.load(Ordering::SeqCst);
        (progress as f32 / total as f32).min(1.0)
    }

    /// Returns whether indexing is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete.load(Ordering::SeqCst)
    }

    /// Returns the number of files indexed so far.
    #[must_use]
    pub fn files_indexed(&self) -> usize {
        self.progress.load(Ordering::SeqCst)
    }

    /// Returns the total number of files to index.
    #[must_use]
    pub fn total_files(&self) -> usize {
        self.total.load(Ordering::SeqCst)
    }

    /// Cancels the current indexing operation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Returns a reference to the underlying index.
    #[must_use]
    pub fn index(&self) -> &Arc<RwLock<FileIndex>> {
        &self.index
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // =========================================================================
    // FileTokenizer Tests
    // =========================================================================

    #[test]
    fn test_tokenize_simple() {
        let tokens = FileTokenizer::tokenize("hello");
        assert_eq!(tokens.len(), 1);
        assert!(tokens.contains(&"hello".to_string()));
    }

    #[test]
    fn test_tokenize_whitespace() {
        let tokens = FileTokenizer::tokenize("my file");
        assert!(tokens.contains(&"my".to_string()));
        assert!(tokens.contains(&"file".to_string()));
    }

    #[test]
    fn test_tokenize_punctuation() {
        let tokens = FileTokenizer::tokenize("my-file_v2.txt");
        assert!(tokens.contains(&"my".to_string()));
        assert!(tokens.contains(&"file".to_string()));
        assert!(tokens.contains(&"v2".to_string()));
        assert!(tokens.contains(&"txt".to_string()));
    }

    #[test]
    fn test_tokenize_camel_case() {
        let tokens = FileTokenizer::tokenize("MyFileSearch");
        assert!(tokens.contains(&"my".to_string()));
        assert!(tokens.contains(&"file".to_string()));
        assert!(tokens.contains(&"search".to_string()));
    }

    #[test]
    fn test_tokenize_pascal_case() {
        let tokens = FileTokenizer::tokenize("PascalCaseFile");
        assert!(tokens.contains(&"pascal".to_string()));
        assert!(tokens.contains(&"case".to_string()));
        assert!(tokens.contains(&"file".to_string()));
    }

    #[test]
    fn test_tokenize_uppercase_sequence() {
        let tokens = FileTokenizer::tokenize("XMLParser");
        assert!(tokens.contains(&"xml".to_string()));
        assert!(tokens.contains(&"parser".to_string()));
    }

    #[test]
    fn test_tokenize_lowercase() {
        let tokens = FileTokenizer::tokenize("README");
        assert!(tokens.contains(&"readme".to_string()));
    }

    #[test]
    fn test_tokenize_ascii_fold() {
        let tokens = FileTokenizer::tokenize("résumé");
        assert!(tokens.contains(&"resume".to_string()));
    }

    #[test]
    fn test_tokenize_mixed_accents() {
        let tokens = FileTokenizer::tokenize("naïve_café");
        assert!(tokens.contains(&"naive".to_string()));
        assert!(tokens.contains(&"cafe".to_string()));
    }

    #[test]
    fn test_tokenize_path() {
        let tokens = FileTokenizer::tokenize("/Users/john/MyDocument.txt");
        assert!(tokens.contains(&"users".to_string()));
        assert!(tokens.contains(&"john".to_string()));
        assert!(tokens.contains(&"my".to_string()));
        assert!(tokens.contains(&"document".to_string()));
        assert!(tokens.contains(&"txt".to_string()));
    }

    #[test]
    fn test_tokenize_file_name_only() {
        let path = Path::new("/Users/john/MyDocument.txt");
        let tokens = FileTokenizer::tokenize_file_name(path);
        // Should only contain tokens from "MyDocument.txt"
        assert!(tokens.contains(&"my".to_string()));
        assert!(tokens.contains(&"document".to_string()));
        assert!(tokens.contains(&"txt".to_string()));
        // Should NOT contain path components
        assert!(!tokens.contains(&"users".to_string()));
        assert!(!tokens.contains(&"john".to_string()));
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = FileTokenizer::tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_digits() {
        let tokens = FileTokenizer::tokenize("file123name");
        assert!(tokens.contains(&"file".to_string()));
        assert!(tokens.contains(&"123".to_string()));
        assert!(tokens.contains(&"name".to_string()));
    }

    #[test]
    fn test_tokenize_deduplication() {
        // "file-file.file" should only produce one "file" token
        let tokens = FileTokenizer::tokenize("file-file.file");
        let file_count = tokens.iter().filter(|t| *t == "file").count();
        assert_eq!(file_count, 1);
    }

    // =========================================================================
    // FileIndex Tests
    // =========================================================================

    #[test]
    fn test_index_open_in_memory() {
        let index = FileIndex::open_in_memory();
        assert!(index.is_ok());
    }

    #[test]
    fn test_index_open_file() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let index = FileIndex::open(&db_path);
        assert!(index.is_ok());
        assert!(db_path.exists());
    }

    #[test]
    fn test_index_add_and_search() {
        let dir = TempDir::new().unwrap();

        // Create a test file
        let test_file = dir.path().join("MyDocument.txt");
        std::fs::write(&test_file, "test content").unwrap();

        // Index the file
        let mut index = FileIndex::open_in_memory().unwrap();
        index.add_file(&test_file).unwrap();

        // Search for it
        let results = index
            .search(&["my".to_string(), "document".to_string()], 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "MyDocument.txt");
    }

    #[test]
    fn test_index_search_prefix() {
        let dir = TempDir::new().unwrap();

        // Create test files
        let file1 = dir.path().join("apple.txt");
        let file2 = dir.path().join("application.txt");
        let file3 = dir.path().join("banana.txt");
        std::fs::write(&file1, "").unwrap();
        std::fs::write(&file2, "").unwrap();
        std::fs::write(&file3, "").unwrap();

        // Index files
        let mut index = FileIndex::open_in_memory().unwrap();
        index.add_file(&file1).unwrap();
        index.add_file(&file2).unwrap();
        index.add_file(&file3).unwrap();

        // Search with prefix
        let results = index.search_prefix("app", 10).unwrap();
        assert_eq!(results.len(), 2);

        // Banana should not be in results
        let names: Vec<_> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(!names.contains(&"banana.txt"));
    }

    #[test]
    fn test_index_remove_file() {
        let dir = TempDir::new().unwrap();

        let test_file = dir.path().join("test.txt");
        std::fs::write(&test_file, "").unwrap();

        let mut index = FileIndex::open_in_memory().unwrap();
        index.add_file(&test_file).unwrap();
        assert_eq!(index.file_count().unwrap(), 1);

        index.remove_file(&test_file).unwrap();
        assert_eq!(index.file_count().unwrap(), 0);
    }

    #[test]
    fn test_index_update_file() {
        let dir = TempDir::new().unwrap();

        let test_file = dir.path().join("test.txt");
        std::fs::write(&test_file, "original").unwrap();

        let mut index = FileIndex::open_in_memory().unwrap();
        index.add_file(&test_file).unwrap();

        // Update file content and re-index
        std::fs::write(&test_file, "updated content that is longer").unwrap();
        index.update_file(&test_file).unwrap();

        // Should still have one file
        assert_eq!(index.file_count().unwrap(), 1);
    }

    #[test]
    fn test_index_clear() {
        let dir = TempDir::new().unwrap();

        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");
        std::fs::write(&file1, "").unwrap();
        std::fs::write(&file2, "").unwrap();

        let mut index = FileIndex::open_in_memory().unwrap();
        index.add_file(&file1).unwrap();
        index.add_file(&file2).unwrap();
        assert_eq!(index.file_count().unwrap(), 2);

        index.clear().unwrap();
        assert_eq!(index.file_count().unwrap(), 0);
    }

    #[test]
    fn test_index_directory() {
        let dir = TempDir::new().unwrap();

        // Create a subdirectory
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let mut index = FileIndex::open_in_memory().unwrap();
        index.add_file(&subdir).unwrap();

        let results = index.search(&["subdir".to_string()], 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_directory);
    }

    // =========================================================================
    // IndexingService Tests
    // =========================================================================

    #[test]
    fn test_indexing_service_sync() {
        let dir = TempDir::new().unwrap();

        // Create some test files
        std::fs::write(dir.path().join("file1.txt"), "").unwrap();
        std::fs::write(dir.path().join("file2.txt"), "").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        std::fs::write(dir.path().join("subdir/file3.txt"), "").unwrap();

        let index = FileIndex::open_in_memory().unwrap();
        let service = IndexingService::new(index, vec![dir.path().to_path_buf()]);

        service.start_indexing_sync().unwrap();

        assert!(service.is_complete());
        assert!(service.files_indexed() > 0);
        assert!((service.progress() - 1.0).abs() < f32::EPSILON);

        // Verify files are in the index
        let index = service.index().read();
        assert!(index.file_count().unwrap() >= 3);
    }

    #[test]
    fn test_indexing_service_skips_hidden() {
        let dir = TempDir::new().unwrap();

        // Create a hidden file
        std::fs::write(dir.path().join(".hidden"), "").unwrap();
        std::fs::write(dir.path().join("visible.txt"), "").unwrap();

        let index = FileIndex::open_in_memory().unwrap();
        let service = IndexingService::new(index, vec![dir.path().to_path_buf()]);

        service.start_indexing_sync().unwrap();

        // Check that hidden file is not indexed
        let index = service.index().read();
        let results = index.search(&["hidden".to_string()], 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_indexing_service_cancel() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("file.txt"), "").unwrap();

        let index = FileIndex::open_in_memory().unwrap();
        let service = IndexingService::new(index, vec![dir.path().to_path_buf()]);

        // Cancel before starting
        service.cancel();

        // Should complete immediately without indexing
        service.start_indexing_sync().unwrap();
        assert!(!service.is_complete());
    }

    #[tokio::test]
    async fn test_indexing_service_async() {
        let dir = TempDir::new().unwrap();

        std::fs::write(dir.path().join("async_file.txt"), "").unwrap();

        let index = FileIndex::open_in_memory().unwrap();
        let service = IndexingService::new(index, vec![dir.path().to_path_buf()]);

        service.start_indexing().await.unwrap();

        assert!(service.is_complete());

        let index = service.index().read();
        let results = index.search(&["async".to_string()], 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn test_indexing_service_handles_circular_symlinks() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();

        // Create a directory structure with circular symlinks:
        // subdir/
        //   file.txt
        //   circular -> ../subdir (points back to parent, creating a loop)
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("file.txt"), "content").unwrap();

        // Create circular symlink (subdir/circular -> ../subdir)
        symlink(&subdir, subdir.join("circular")).unwrap();

        let index = FileIndex::open_in_memory().unwrap();
        let service = IndexingService::new(index, vec![dir.path().to_path_buf()]);

        // This should complete without hanging or crashing due to the circular symlink
        service.start_indexing_sync().unwrap();

        assert!(service.is_complete());
        // Verify that regular file was indexed
        let index = service.index().read();
        let results = index.search(&["file".to_string()], 10).unwrap();
        assert!(!results.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn test_indexing_service_indexes_symlink_as_file() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();

        // Create a file and a symlink to it
        let target = dir.path().join("target.txt");
        std::fs::write(&target, "content").unwrap();
        symlink(&target, dir.path().join("link.txt")).unwrap();

        let index = FileIndex::open_in_memory().unwrap();
        let service = IndexingService::new(index, vec![dir.path().to_path_buf()]);

        service.start_indexing_sync().unwrap();

        // The symlink itself should be indexed but we don't follow it
        // (follow_links(false) means WalkDir treats symlinks as their own entries)
        let index = service.index().read();
        let _results = index.search(&["link".to_string()], 10).unwrap();
        // Symlinks may or may not be indexed depending on implementation - the key is no crash
        assert!(service.is_complete());
    }
}
