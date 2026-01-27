//! Spotlight/NSMetadataQuery integration for file search.
//!
//! Uses the `mdfind` command-line tool which provides a simpler interface
//! to Spotlight search than direct NSMetadataQuery FFI.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use thiserror::Error;
use tokio::process::Command;
use tracing::{debug, warn};

/// Default timeout for Spotlight queries.
pub const DEFAULT_TIMEOUT_MS: u64 = 500;

/// Default maximum results to return.
pub const DEFAULT_MAX_RESULTS: usize = 5;

/// Errors that can occur during Spotlight searches.
#[derive(Error, Debug)]
pub enum SpotlightError {
    /// Query timed out.
    #[error("spotlight query timed out after {timeout_ms}ms")]
    Timeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Command execution failed.
    #[error("mdfind command failed: {reason}")]
    CommandFailed {
        /// Reason for failure.
        reason: String,
    },

    /// I/O error during query.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid query string.
    #[error("invalid query: {reason}")]
    InvalidQuery {
        /// Reason for invalidity.
        reason: String,
    },
}

impl SpotlightError {
    /// Returns a user-friendly error message.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Timeout { .. } => "Search took too long. Try a shorter query.".to_string(),
            Self::CommandFailed { .. } => "File search is temporarily unavailable.".to_string(),
            Self::Io(_) => "Unable to search files.".to_string(),
            Self::InvalidQuery { .. } => "Invalid search query.".to_string(),
        }
    }

    /// Returns true if the error is recoverable.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::Timeout { .. } | Self::InvalidQuery { .. })
    }
}

/// A file result from Spotlight search.
#[derive(Debug, Clone)]
pub struct FileResult {
    /// Path to the file.
    pub path: PathBuf,
    /// Display name.
    pub name: String,
    /// File kind.
    pub kind: FileKind,
    /// File size in bytes (lazily loaded).
    pub size: Option<u64>,
    /// Last modified time (lazily loaded).
    pub modified: Option<SystemTime>,
}

impl FileResult {
    /// Creates a new file result from a path.
    ///
    /// Metadata (size, modified time) is loaded lazily.
    #[must_use]
    pub fn from_path(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let kind = FileKind::from_path(&path);

        Self {
            path,
            name,
            kind,
            size: None,
            modified: None,
        }
    }

    /// Loads metadata (size and modified time) for this file.
    ///
    /// This is done lazily to avoid blocking on file system operations
    /// during search result collection.
    pub fn load_metadata(&mut self) {
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            self.size = Some(metadata.len());
            self.modified = metadata.modified().ok();
        }
    }

    /// Returns the modified time as a `DateTime<Utc>`.
    #[must_use]
    pub fn modified_datetime(&self) -> Option<DateTime<Utc>> {
        self.modified.map(DateTime::from)
    }
}

/// Kind of file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Regular file.
    File,
    /// Directory/folder.
    Folder,
    /// Application bundle.
    Application,
    /// Document.
    Document,
    /// Image.
    Image,
    /// Audio file.
    Audio,
    /// Video file.
    Video,
    /// Other/unknown.
    Other,
}

impl FileKind {
    /// Determines the file kind from a path.
    #[must_use]
    pub fn from_path(path: &Path) -> Self {
        // Check extension first for files that might not exist yet
        let ext = path.extension().and_then(|e| e.to_str());

        match ext {
            Some("app") => return Self::Application,
            Some("pdf" | "doc" | "docx" | "txt" | "rtf" | "pages" | "md" | "odt") => {
                return Self::Document
            },
            Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "heic" | "webp") => {
                return Self::Image
            },
            Some("mp3" | "m4a" | "wav" | "aac" | "flac" | "ogg" | "aiff") => return Self::Audio,
            Some("mp4" | "m4v" | "mov" | "avi" | "mkv" | "webm") => return Self::Video,
            _ => {},
        }

        // Check if it's a directory
        if path.is_dir() {
            if ext == Some("app") {
                Self::Application
            } else {
                Self::Folder
            }
        } else {
            Self::File
        }
    }

    /// Returns a display name for this file kind.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Folder => "Folder",
            Self::Application => "Application",
            Self::Document => "Document",
            Self::Image => "Image",
            Self::Audio => "Audio",
            Self::Video => "Video",
            Self::Other => "Other",
        }
    }

    /// Returns an icon name for this file kind.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn icon_name(&self) -> &'static str {
        match self {
            Self::File => "doc",
            Self::Folder => "folder",
            Self::Application => "app",
            Self::Document => "doc.text",
            Self::Image => "photo",
            Self::Audio => "music.note",
            Self::Video => "video",
            Self::Other => "doc",
        }
    }
}

/// Configuration for a Spotlight query.
#[derive(Debug, Clone)]
pub struct SpotlightQuery {
    /// The search query string.
    query: String,
    /// Maximum number of results to return.
    max_results: usize,
    /// Timeout for the query in milliseconds.
    timeout_ms: u64,
    /// Search scope (directory to search in).
    search_scope: Option<PathBuf>,
}

impl SpotlightQuery {
    /// Creates a new Spotlight query with default settings.
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            max_results: DEFAULT_MAX_RESULTS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            search_scope: None,
        }
    }

    /// Sets the maximum number of results to return.
    #[must_use]
    pub const fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }

    /// Sets the timeout in milliseconds.
    #[must_use]
    pub const fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets the search scope to the user's home directory.
    #[must_use]
    pub fn with_home_scope(mut self) -> Self {
        self.search_scope = dirs::home_dir();
        self
    }

    /// Sets a custom search scope.
    #[must_use]
    pub fn with_scope(mut self, path: PathBuf) -> Self {
        self.search_scope = Some(path);
        self
    }

    /// Executes the Spotlight query asynchronously.
    ///
    /// Uses the `mdfind` command-line tool to search for files.
    ///
    /// # Errors
    ///
    /// Returns an error if the query times out or the command fails.
    pub async fn execute(&self) -> Result<Vec<FileResult>, SpotlightError> {
        if self.query.is_empty() {
            return Ok(Vec::new());
        }

        // Validate query doesn't contain dangerous characters
        if self.query.contains('\0') {
            return Err(SpotlightError::InvalidQuery {
                reason: "Query contains null character".to_string(),
            });
        }

        debug!(query = %self.query, max_results = self.max_results, "Executing Spotlight query");

        // Build mdfind command
        let mut cmd = Command::new("mdfind");

        // Use -name for simple name matching (more intuitive for users)
        cmd.arg("-name").arg(&self.query);

        // Add search scope if specified
        if let Some(scope) = &self.search_scope {
            cmd.arg("-onlyin").arg(scope);
        }

        // Execute with timeout
        let timeout = Duration::from_millis(self.timeout_ms);
        let output = if let Ok(result) = tokio::time::timeout(timeout, cmd.output()).await { result? } else {
            warn!(
                query = %self.query,
                timeout_ms = self.timeout_ms,
                "Spotlight query timed out"
            );
            return Err(SpotlightError::Timeout {
                timeout_ms: self.timeout_ms,
            });
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpotlightError::CommandFailed {
                reason: stderr.to_string(),
            });
        }

        // Parse output (one path per line)
        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Vec<FileResult> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .take(self.max_results)
            .map(|line| FileResult::from_path(PathBuf::from(line)))
            .collect();

        debug!(
            query = %self.query,
            result_count = results.len(),
            "Spotlight query completed"
        );

        Ok(results)
    }

    /// Executes the query synchronously (blocking).
    ///
    /// Uses `std::process::Command` for synchronous execution.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails.
    pub fn execute_sync(&self) -> Result<Vec<FileResult>, SpotlightError> {
        if self.query.is_empty() {
            return Ok(Vec::new());
        }

        if self.query.contains('\0') {
            return Err(SpotlightError::InvalidQuery {
                reason: "Query contains null character".to_string(),
            });
        }

        // Build mdfind command
        let mut cmd = std::process::Command::new("mdfind");
        cmd.arg("-name").arg(&self.query);

        if let Some(scope) = &self.search_scope {
            cmd.arg("-onlyin").arg(scope);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpotlightError::CommandFailed {
                reason: stderr.to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Vec<FileResult> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .take(self.max_results)
            .map(|line| FileResult::from_path(PathBuf::from(line)))
            .collect();

        Ok(results)
    }
}

/// Provider for Spotlight file searches.
///
/// This struct provides a high-level interface for searching files
/// using macOS Spotlight via the `mdfind` command.
#[derive(Debug, Clone)]
pub struct SpotlightProvider {
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Timeout for queries in milliseconds.
    pub timeout_ms: u64,
    /// Optional search scope (defaults to user home).
    pub search_scope: Option<PathBuf>,
}

impl Default for SpotlightProvider {
    fn default() -> Self {
        Self {
            max_results: DEFAULT_MAX_RESULTS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            search_scope: dirs::home_dir(),
        }
    }
}

impl SpotlightProvider {
    /// Creates a new Spotlight provider with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Spotlight provider with a specific result limit.
    #[must_use]
    pub fn with_max_results(max_results: usize) -> Self {
        Self {
            max_results,
            ..Self::default()
        }
    }

    /// Sets the timeout in milliseconds.
    #[must_use]
    pub const fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets the search scope.
    #[must_use]
    pub fn with_scope(mut self, scope: PathBuf) -> Self {
        self.search_scope = Some(scope);
        self
    }

    /// Performs a Spotlight search asynchronously.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query.
    ///
    /// # Returns
    ///
    /// A list of file results matching the query, or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the query times out or fails.
    pub async fn search(&self, query: &str) -> Result<Vec<FileResult>, SpotlightError> {
        let mut spotlight_query = SpotlightQuery::new(query)
            .with_max_results(self.max_results)
            .with_timeout_ms(self.timeout_ms);

        if let Some(scope) = &self.search_scope {
            spotlight_query = spotlight_query.with_scope(scope.clone());
        }

        spotlight_query.execute().await
    }

    /// Performs a Spotlight search synchronously.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query.
    ///
    /// # Returns
    ///
    /// A list of file results matching the query, or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn search_sync(&self, query: &str) -> Result<Vec<FileResult>, SpotlightError> {
        let mut spotlight_query = SpotlightQuery::new(query)
            .with_max_results(self.max_results)
            .with_timeout_ms(self.timeout_ms);

        if let Some(scope) = &self.search_scope {
            spotlight_query = spotlight_query.with_scope(scope.clone());
        }

        spotlight_query.execute_sync()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // FileKind Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_file_kind_from_document_extensions() {
        assert_eq!(
            FileKind::from_path(Path::new("/test/doc.pdf")),
            FileKind::Document
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/doc.txt")),
            FileKind::Document
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/doc.md")),
            FileKind::Document
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/doc.docx")),
            FileKind::Document
        );
    }

    #[test]
    fn test_file_kind_from_image_extensions() {
        assert_eq!(
            FileKind::from_path(Path::new("/test/photo.jpg")),
            FileKind::Image
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/photo.png")),
            FileKind::Image
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/photo.heic")),
            FileKind::Image
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/photo.webp")),
            FileKind::Image
        );
    }

    #[test]
    fn test_file_kind_from_audio_extensions() {
        assert_eq!(
            FileKind::from_path(Path::new("/test/song.mp3")),
            FileKind::Audio
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/song.m4a")),
            FileKind::Audio
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/song.flac")),
            FileKind::Audio
        );
    }

    #[test]
    fn test_file_kind_from_video_extensions() {
        assert_eq!(
            FileKind::from_path(Path::new("/test/video.mp4")),
            FileKind::Video
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/video.mov")),
            FileKind::Video
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/video.mkv")),
            FileKind::Video
        );
    }

    #[test]
    fn test_file_kind_from_app_extension() {
        assert_eq!(
            FileKind::from_path(Path::new("/Applications/Safari.app")),
            FileKind::Application
        );
    }

    #[test]
    fn test_file_kind_generic_file() {
        assert_eq!(
            FileKind::from_path(Path::new("/test/unknown.xyz")),
            FileKind::File
        );
        assert_eq!(
            FileKind::from_path(Path::new("/test/data.json")),
            FileKind::File
        );
    }

    #[test]
    fn test_file_kind_display_names() {
        assert_eq!(FileKind::File.display_name(), "File");
        assert_eq!(FileKind::Folder.display_name(), "Folder");
        assert_eq!(FileKind::Application.display_name(), "Application");
        assert_eq!(FileKind::Document.display_name(), "Document");
    }

    // -------------------------------------------------------------------------
    // FileResult Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_file_result_from_path() {
        let result = FileResult::from_path(PathBuf::from("/Users/test/Documents/report.pdf"));
        assert_eq!(result.name, "report.pdf");
        assert_eq!(result.kind, FileKind::Document);
        assert!(result.size.is_none()); // Lazy loading
        assert!(result.modified.is_none());
    }

    #[test]
    fn test_file_result_from_path_with_folder() {
        // Note: This test checks extension-based detection, not actual filesystem
        let result = FileResult::from_path(PathBuf::from("/Applications/Xcode.app"));
        assert_eq!(result.name, "Xcode.app");
        assert_eq!(result.kind, FileKind::Application);
    }

    // -------------------------------------------------------------------------
    // SpotlightQuery Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_spotlight_query_builder() {
        let query = SpotlightQuery::new("test")
            .with_max_results(10)
            .with_timeout_ms(1000)
            .with_home_scope();

        assert_eq!(query.query, "test");
        assert_eq!(query.max_results, 10);
        assert_eq!(query.timeout_ms, 1000);
        assert!(query.search_scope.is_some());
    }

    #[test]
    fn test_spotlight_query_custom_scope() {
        let query = SpotlightQuery::new("test").with_scope(PathBuf::from("/tmp"));

        assert_eq!(query.search_scope, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_empty_query_returns_empty() {
        let query = SpotlightQuery::new("");
        let result = query.execute_sync();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_invalid_query_with_null() {
        let query = SpotlightQuery::new("test\0query");
        let result = query.execute_sync();
        assert!(matches!(result, Err(SpotlightError::InvalidQuery { .. })));
    }

    // -------------------------------------------------------------------------
    // SpotlightProvider Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_spotlight_provider_default() {
        let provider = SpotlightProvider::new();
        assert_eq!(provider.max_results, DEFAULT_MAX_RESULTS);
        assert_eq!(provider.timeout_ms, DEFAULT_TIMEOUT_MS);
        // Search scope should be home directory
        assert!(provider.search_scope.is_some());
    }

    #[test]
    fn test_spotlight_provider_with_max_results() {
        let provider = SpotlightProvider::with_max_results(20);
        assert_eq!(provider.max_results, 20);
    }

    #[test]
    fn test_spotlight_provider_with_timeout() {
        let provider = SpotlightProvider::new().with_timeout_ms(1000);
        assert_eq!(provider.timeout_ms, 1000);
    }

    #[test]
    fn test_spotlight_provider_with_scope() {
        let provider = SpotlightProvider::new().with_scope(PathBuf::from("/tmp"));
        assert_eq!(provider.search_scope, Some(PathBuf::from("/tmp")));
    }

    // -------------------------------------------------------------------------
    // SpotlightError Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_spotlight_error_timeout() {
        let error = SpotlightError::Timeout { timeout_ms: 500 };
        assert!(error.is_recoverable());
        assert!(!error.user_message().is_empty());
    }

    #[test]
    fn test_spotlight_error_command_failed() {
        let error = SpotlightError::CommandFailed {
            reason: "test".to_string(),
        };
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_spotlight_error_invalid_query() {
        let error = SpotlightError::InvalidQuery {
            reason: "null char".to_string(),
        };
        assert!(error.is_recoverable());
    }

    // -------------------------------------------------------------------------
    // Integration Tests (require mdfind to be available)
    // -------------------------------------------------------------------------

    #[test]
    #[ignore = "requires mdfind command and actual filesystem"]
    fn test_spotlight_query_integration() {
        let query = SpotlightQuery::new("Desktop")
            .with_max_results(5)
            .with_home_scope();

        let result = query.execute_sync();
        // Should at least not error (might return 0 results if no Desktop folder)
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires mdfind command and actual filesystem"]
    async fn test_spotlight_query_async_integration() {
        let query = SpotlightQuery::new("Desktop")
            .with_max_results(5)
            .with_home_scope();

        let result = query.execute().await;
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "requires mdfind command and actual filesystem"]
    fn test_spotlight_provider_search_sync() {
        let provider = SpotlightProvider::with_max_results(5);
        let result = provider.search_sync("readme");

        // Should not error
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires mdfind command and actual filesystem"]
    async fn test_spotlight_provider_search_async() {
        let provider = SpotlightProvider::with_max_results(5);
        let result = provider.search("readme").await;

        assert!(result.is_ok());
    }
}
