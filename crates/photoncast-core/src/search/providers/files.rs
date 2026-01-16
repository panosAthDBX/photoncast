//! File search provider using Spotlight.
//!
//! This module provides a search provider that integrates with macOS Spotlight
//! for file search functionality.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::{debug, warn};

use crate::platform::spotlight::{FileKind, SpotlightProvider};
use crate::search::providers::SearchProvider;
use crate::search::{IconSource, ResultType, SearchAction, SearchResult, SearchResultId};

/// Default maximum number of file results.
pub const DEFAULT_FILE_MAX_RESULTS: usize = 5;

/// Provides search results for files via Spotlight.
///
/// This provider wraps the Spotlight integration to search for files
/// and convert results to the standard search result format.
#[derive(Debug)]
pub struct FileProvider {
    /// The underlying Spotlight provider.
    spotlight: SpotlightProvider,
    /// Maximum number of results to return.
    max_results: usize,
    /// Cache for recent search results (optional).
    cache: Arc<RwLock<Option<CachedResults>>>,
}

/// Cached search results for avoiding repeated Spotlight queries.
#[derive(Debug, Clone)]
struct CachedResults {
    query: String,
    results: Vec<SearchResult>,
}

impl Default for FileProvider {
    fn default() -> Self {
        Self::new(DEFAULT_FILE_MAX_RESULTS)
    }
}

impl FileProvider {
    /// Creates a new file provider with the specified result limit.
    #[must_use]
    pub fn new(max_results: usize) -> Self {
        Self {
            spotlight: SpotlightProvider::with_max_results(max_results),
            max_results,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Creates a file provider with a custom Spotlight provider.
    #[must_use]
    pub fn with_spotlight(spotlight: SpotlightProvider) -> Self {
        let max_results = spotlight.max_results;
        Self {
            spotlight,
            max_results,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Sets the search scope to a specific directory.
    #[must_use]
    pub fn with_scope(mut self, scope: PathBuf) -> Self {
        self.spotlight = self.spotlight.with_scope(scope);
        self
    }

    /// Sets the timeout in milliseconds.
    #[must_use]
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.spotlight = self.spotlight.with_timeout_ms(timeout_ms);
        self
    }

    /// Returns the maximum number of results configured.
    #[must_use]
    pub const fn max_results(&self) -> usize {
        self.max_results
    }

    /// Clears the internal cache.
    pub fn clear_cache(&self) {
        *self.cache.write() = None;
    }

    /// Performs an async search and returns results.
    ///
    /// This method can be called from async contexts when the
    /// synchronous `SearchProvider::search` trait method is not suitable.
    pub async fn search_async(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        // Check cache
        {
            let cache = self.cache.read();
            if let Some(cached) = cache.as_ref() {
                if cached.query == query {
                    debug!(query = %query, "Using cached file search results");
                    return cached.results.iter().take(max_results).cloned().collect();
                }
            }
        }

        debug!(query = %query, max_results = max_results, "Executing async file search");

        match self.spotlight.search(query).await {
            Ok(file_results) => {
                let results: Vec<SearchResult> = file_results
                    .into_iter()
                    .take(max_results)
                    .enumerate()
                    .map(|(idx, file_result)| self.convert_to_search_result(file_result, idx))
                    .collect();

                // Update cache
                *self.cache.write() = Some(CachedResults {
                    query: query.to_string(),
                    results: results.clone(),
                });

                results
            },
            Err(e) => {
                warn!(query = %query, error = %e, "Spotlight search failed");
                Vec::new()
            },
        }
    }

    /// Converts a Spotlight FileResult to a SearchResult.
    fn convert_to_search_result(
        &self,
        file_result: crate::platform::spotlight::FileResult,
        _index: usize,
    ) -> SearchResult {
        let result_type = match file_result.kind {
            FileKind::Folder => ResultType::Folder,
            FileKind::Application => ResultType::Application,
            _ => ResultType::File,
        };

        let subtitle = file_result
            .path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| file_result.path.display().to_string());

        SearchResult {
            id: SearchResultId::new(format!("file:{}", file_result.path.display())),
            title: file_result.name.clone(),
            subtitle,
            icon: IconSource::FileIcon {
                path: file_result.path.clone(),
            },
            result_type,
            score: 0.0,                // Score will be applied by the ranking system
            match_indices: Vec::new(), // Spotlight doesn't provide match indices
            action: SearchAction::OpenFile {
                path: file_result.path,
            },
        }
    }
}

impl SearchProvider for FileProvider {
    fn name(&self) -> &str {
        "Files"
    }

    fn result_type(&self) -> ResultType {
        ResultType::File
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(cached) = cache.as_ref() {
                if cached.query == query {
                    return cached.results.iter().take(max_results).cloned().collect();
                }
            }
        }

        debug!(query = %query, max_results = max_results, "Executing sync file search");

        // Use synchronous search
        match self.spotlight.search_sync(query) {
            Ok(file_results) => {
                let results: Vec<SearchResult> = file_results
                    .into_iter()
                    .take(max_results)
                    .enumerate()
                    .map(|(idx, file_result)| self.convert_to_search_result(file_result, idx))
                    .collect();

                // Update cache
                *self.cache.write() = Some(CachedResults {
                    query: query.to_string(),
                    results: results.clone(),
                });

                results
            },
            Err(e) => {
                warn!(query = %query, error = %e, "Spotlight search failed");
                Vec::new()
            },
        }
    }
}

/// File usage tracker trait for frecency integration.
///
/// This trait allows the file provider to optionally track file usage
/// for frecency-based ranking.
pub trait FileUsageTracker: Send + Sync {
    /// Records that a file was opened.
    fn record_file_open(&self, path: &str);

    /// Gets the usage count for a file.
    fn get_usage_count(&self, path: &str) -> u32;
}

/// No-op implementation of FileUsageTracker for when tracking is disabled.
#[derive(Debug, Default)]
pub struct NoOpFileTracker;

impl FileUsageTracker for NoOpFileTracker {
    fn record_file_open(&self, _path: &str) {}

    fn get_usage_count(&self, _path: &str) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // -------------------------------------------------------------------------
    // FileProvider Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_file_provider_default() {
        let provider = FileProvider::default();
        assert_eq!(provider.name(), "Files");
        assert_eq!(provider.result_type(), ResultType::File);
        assert_eq!(provider.max_results(), DEFAULT_FILE_MAX_RESULTS);
    }

    #[test]
    fn test_file_provider_with_max_results() {
        let provider = FileProvider::new(10);
        assert_eq!(provider.max_results(), 10);
    }

    #[test]
    fn test_file_provider_with_scope() {
        let provider = FileProvider::new(5).with_scope(PathBuf::from("/tmp"));
        assert_eq!(provider.spotlight.search_scope, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_file_provider_with_timeout() {
        let provider = FileProvider::new(5).with_timeout_ms(1000);
        assert_eq!(provider.spotlight.timeout_ms, 1000);
    }

    #[test]
    fn test_search_empty_query() {
        let provider = FileProvider::new(5);
        let results = provider.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_cache_clearing() {
        let provider = FileProvider::new(5);

        // Manually set cache
        *provider.cache.write() = Some(CachedResults {
            query: "test".to_string(),
            results: Vec::new(),
        });

        assert!(provider.cache.read().is_some());

        provider.clear_cache();

        assert!(provider.cache.read().is_none());
    }

    // -------------------------------------------------------------------------
    // Result Conversion Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_convert_file_result() {
        let provider = FileProvider::new(5);
        let file_result = crate::platform::spotlight::FileResult::from_path(PathBuf::from(
            "/Users/test/Documents/report.pdf",
        ));

        let search_result = provider.convert_to_search_result(file_result, 0);

        assert_eq!(search_result.title, "report.pdf");
        assert_eq!(search_result.result_type, ResultType::File);
        assert!(search_result.id.as_str().starts_with("file:"));

        if let SearchAction::OpenFile { path } = &search_result.action {
            assert_eq!(
                path.display().to_string(),
                "/Users/test/Documents/report.pdf"
            );
        } else {
            panic!("Expected OpenFile action");
        }
    }

    #[test]
    fn test_convert_folder_result() {
        let provider = FileProvider::new(5);

        // Create a folder result - note: FileKind detection depends on extension
        // For a proper folder test, we'd need an actual directory
        let file_result =
            crate::platform::spotlight::FileResult::from_path(PathBuf::from("/Applications"));

        // Since /Applications is a path without extension, it will be detected as File
        // unless it actually exists as a directory on the filesystem
        let search_result = provider.convert_to_search_result(file_result, 0);

        assert_eq!(search_result.title, "Applications");
    }

    #[test]
    fn test_convert_app_result() {
        let provider = FileProvider::new(5);
        let file_result = crate::platform::spotlight::FileResult::from_path(PathBuf::from(
            "/Applications/Safari.app",
        ));

        let search_result = provider.convert_to_search_result(file_result, 0);

        assert_eq!(search_result.title, "Safari.app");
        assert_eq!(search_result.result_type, ResultType::Application);
    }

    // -------------------------------------------------------------------------
    // FileUsageTracker Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_no_op_tracker() {
        let tracker = NoOpFileTracker;
        tracker.record_file_open("/test/file.txt");
        assert_eq!(tracker.get_usage_count("/test/file.txt"), 0);
    }

    // -------------------------------------------------------------------------
    // Integration Tests (require mdfind)
    // -------------------------------------------------------------------------

    #[test]
    #[ignore = "requires mdfind command and actual filesystem"]
    fn test_file_provider_search_integration() {
        let provider = FileProvider::new(5);
        let results = provider.search("readme", 5);

        // Should not panic, may or may not have results
        println!("Found {} results", results.len());
    }

    #[tokio::test]
    #[ignore = "requires mdfind command and actual filesystem"]
    async fn test_file_provider_async_search_integration() {
        let provider = FileProvider::new(5);
        let results = provider.search_async("readme", 5).await;

        println!("Found {} results", results.len());
    }
}
