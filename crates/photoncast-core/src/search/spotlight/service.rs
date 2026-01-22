//! High-level Spotlight search service.
//!
//! This module provides a simple, high-level API for performing Spotlight
//! file searches with caching and integration with the FileQuery parser.
//!
//! # Raycast-style Optimizations
//!
//! The service implements several optimizations inspired by Raycast:
//!
//! - **Two-tier search**: Primary scopes (Desktop/Documents/Downloads) are searched
//!   first. If results are insufficient, secondary scopes are added.
//! - **Default exclusions**: Development artifacts (node_modules, .git, target)
//!   and hidden files are excluded by default.
//! - **Recency sorting**: Results are sorted by last used date.
//! - **Caching**: Recent searches are cached to avoid redundant queries.

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use lru::LruCache;
use parking_lot::RwLock;
use thiserror::Error;

use super::predicate::PredicateBuilder;
use super::query::{default_search_scopes, MetadataQueryWrapper, SpotlightError};
use super::result::SpotlightResult;
use crate::search::file_query::{FileCategory, FileQuery, FileTypeFilter};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during search service operations.
#[derive(Debug, Error)]
pub enum SearchServiceError {
    /// Spotlight query error.
    #[error("spotlight error: {0}")]
    Spotlight(#[from] SpotlightError),

    /// Invalid query.
    #[error("invalid query: {0}")]
    InvalidQuery(String),

    /// No search scopes configured.
    #[error("no search scopes configured")]
    NoSearchScopes,

    /// Failed to build predicate.
    #[error("failed to build predicate: {0}")]
    PredicateBuildFailed(String),
}

/// Result type for search service operations.
pub type Result<T> = std::result::Result<T, SearchServiceError>;

// =============================================================================
// Search Options
// =============================================================================

/// Configuration options for Spotlight searches.
#[derive(Debug, Clone)]
pub struct SpotlightSearchOptions {
    /// Maximum number of results to return.
    pub max_results: usize,

    /// Timeout for the search query.
    pub timeout: Duration,

    /// Primary directories to search first (Desktop, Documents, Downloads).
    pub primary_scopes: Vec<PathBuf>,

    /// Secondary directories to expand search to if primary yields few results.
    pub secondary_scopes: Vec<PathBuf>,

    /// Minimum results from primary search before expanding to secondary.
    /// If primary search returns fewer than this, secondary scopes are searched.
    pub min_results_before_expand: usize,

    /// Whether to use the result cache.
    pub use_cache: bool,

    /// How long cached results remain valid.
    pub cache_ttl: Duration,

    /// Apply default exclusions (node_modules, .git, hidden files, etc.).
    pub apply_exclusions: bool,

    /// Sort results by last used date (most recent first).
    pub sort_by_recency: bool,
}

impl Default for SpotlightSearchOptions {
    fn default() -> Self {
        Self {
            max_results: 50,
            timeout: Duration::from_millis(500),
            primary_scopes: primary_search_scopes(),
            secondary_scopes: secondary_search_scopes(),
            min_results_before_expand: 10,
            use_cache: true,
            cache_ttl: Duration::from_secs(30),
            apply_exclusions: true,
            sort_by_recency: true,
        }
    }
}

impl SpotlightSearchOptions {
    /// Creates options optimized for Raycast-style file search.
    ///
    /// - Searches primary scopes first
    /// - Applies default exclusions
    /// - Sorts by recency
    #[must_use]
    pub fn raycast_style() -> Self {
        Self::default()
    }

    /// Creates options that search everywhere without exclusions.
    ///
    /// Useful for "power user" searches where all files should be visible.
    #[must_use]
    pub fn include_all() -> Self {
        Self {
            primary_scopes: all_search_scopes(),
            secondary_scopes: vec![],
            min_results_before_expand: 0,
            apply_exclusions: false,
            sort_by_recency: false,
            ..Self::default()
        }
    }

    /// Creates options for searching a specific directory only.
    #[must_use]
    pub fn in_directory(path: PathBuf) -> Self {
        Self {
            primary_scopes: vec![path],
            secondary_scopes: vec![],
            min_results_before_expand: 0,
            ..Self::default()
        }
    }
}

/// Returns primary search scopes (most commonly accessed directories).
fn primary_search_scopes() -> Vec<PathBuf> {
    let mut scopes = Vec::new();
    if let Some(home) = dirs::home_dir() {
        scopes.push(home.join("Desktop"));
        scopes.push(home.join("Documents"));
        scopes.push(home.join("Downloads"));
    }
    scopes.push(PathBuf::from("/Applications"));
    scopes
}

/// Returns secondary search scopes (expanded search areas).
fn secondary_search_scopes() -> Vec<PathBuf> {
    let mut scopes = Vec::new();
    if let Some(home) = dirs::home_dir() {
        scopes.push(home.join("Pictures"));
        scopes.push(home.join("Music"));
        scopes.push(home.join("Movies"));
        scopes.push(home.join("Public"));
        // Don't include home directly - too broad
    }
    scopes
}

/// Returns all common search scopes.
fn all_search_scopes() -> Vec<PathBuf> {
    let mut scopes = primary_search_scopes();
    scopes.extend(secondary_search_scopes());
    if let Some(home) = dirs::home_dir() {
        scopes.push(home);
    }
    scopes
}

// =============================================================================
// Cache
// =============================================================================

/// Cached search result with timestamp.
struct CachedResult {
    results: Vec<SpotlightResult>,
    timestamp: Instant,
}

impl CachedResult {
    fn new(results: Vec<SpotlightResult>) -> Self {
        Self {
            results,
            timestamp: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }
}

// =============================================================================
// SpotlightSearchService
// =============================================================================

/// High-level service for Spotlight file searches.
///
/// This service provides a simple API for searching files using Spotlight,
/// with built-in caching and integration with the FileQuery parser.
///
/// The service is thread-safe and can be shared across threads.
#[derive(Clone)]
pub struct SpotlightSearchService {
    inner: Arc<SpotlightSearchServiceInner>,
}

struct SpotlightSearchServiceInner {
    default_options: SpotlightSearchOptions,
    cache: RwLock<LruCache<String, CachedResult>>,
}

impl SpotlightSearchService {
    /// Creates a new search service with default options.
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(SpotlightSearchOptions::default())
    }

    /// Creates a new search service with custom options.
    #[must_use]
    pub fn with_options(options: SpotlightSearchOptions) -> Self {
        let cache_size = NonZeroUsize::new(50).unwrap();
        Self {
            inner: Arc::new(SpotlightSearchServiceInner {
                default_options: options,
                cache: RwLock::new(LruCache::new(cache_size)),
            }),
        }
    }

    /// Performs a simple text search.
    ///
    /// Searches for files whose names contain the query string.
    pub fn search(&self, query: &str) -> Result<Vec<SpotlightResult>> {
        self.search_with_options(query, &self.inner.default_options)
    }

    /// Performs a search with custom options.
    ///
    /// This implements two-tier search:
    /// 1. Search primary scopes first
    /// 2. If results < min_results_before_expand, expand to secondary scopes
    pub fn search_with_options(
        &self,
        query: &str,
        options: &SpotlightSearchOptions,
    ) -> Result<Vec<SpotlightResult>> {
        let query = query.trim();

        if query.is_empty() {
            return Err(SearchServiceError::InvalidQuery(
                "query cannot be empty".to_string(),
            ));
        }

        // Check cache first
        if options.use_cache {
            let cache_key = self.make_cache_key(query, options);
            if let Some(cached) = self.get_cached(&cache_key, options.cache_ttl) {
                return Ok(cached);
            }
        }

        // Build predicate with optional exclusions
        let mut builder = PredicateBuilder::new().name_contains(query);
        if options.apply_exclusions {
            builder = builder.with_default_exclusions();
        }
        let predicate = builder.build();

        // Phase 1: Search primary scopes
        let mut results = self.execute_query_with_scopes(
            &predicate,
            &options.primary_scopes,
            options.timeout,
            options.sort_by_recency,
        )?;

        // Phase 2: Expand to secondary scopes if needed
        if results.len() < options.min_results_before_expand
            && !options.secondary_scopes.is_empty()
        {
            let secondary_results = self.execute_query_with_scopes(
                &predicate,
                &options.secondary_scopes,
                options.timeout,
                options.sort_by_recency,
            )?;

            // Merge and deduplicate by path (clone paths to avoid borrow issue)
            let existing_paths: HashSet<PathBuf> =
                results.iter().map(|r| r.path.clone()).collect();
            for result in secondary_results {
                if !existing_paths.contains(&result.path) {
                    results.push(result);
                }
            }
        }

        // Limit results
        results.truncate(options.max_results);

        // Cache results
        if options.use_cache {
            let cache_key = self.make_cache_key(query, options);
            self.cache_results(&cache_key, results.clone());
        }

        Ok(results)
    }

    /// Executes a query with specific scopes and options.
    fn execute_query_with_scopes(
        &self,
        predicate: &objc2_foundation::NSPredicate,
        scopes: &[PathBuf],
        timeout: Duration,
        sort_by_recency: bool,
    ) -> Result<Vec<SpotlightResult>> {
        if scopes.is_empty() {
            return Ok(vec![]);
        }

        let mut metadata_query = MetadataQueryWrapper::new();
        metadata_query.set_predicate(predicate);
        metadata_query.set_search_scopes(scopes);

        if sort_by_recency {
            metadata_query.sort_by_last_used();
        }

        Ok(metadata_query.execute_sync(timeout)?)
    }

    /// Performs a search using a parsed FileQuery.
    ///
    /// This integrates with the existing FileQuery parser to support
    /// advanced query syntax like `.pdf`, `in ~/Downloads`, etc.
    pub fn search_file_query(&self, file_query: &FileQuery) -> Result<Vec<SpotlightResult>> {
        let options = self.inner.default_options.clone();
        self.search_file_query_with_options(file_query, &options)
    }

    /// Performs a search using a parsed FileQuery with custom options.
    pub fn search_file_query_with_options(
        &self,
        file_query: &FileQuery,
        options: &SpotlightSearchOptions,
    ) -> Result<Vec<SpotlightResult>> {
        // Convert FileQuery to predicate, optionally with exclusions
        let predicate = file_query_to_predicate(file_query, options.apply_exclusions)?;

        // Determine search scopes from FileQuery location or use options
        let search_scopes = if let Some(ref location) = file_query.location {
            resolve_location_scope(location)
        } else {
            // Use primary + secondary if no specific location
            let mut scopes = options.primary_scopes.clone();
            scopes.extend(options.secondary_scopes.clone());
            scopes
        };

        // Execute query
        let results = self.execute_query_with_scopes(
            &predicate,
            &search_scopes,
            options.timeout,
            options.sort_by_recency,
        )?;

        let mut results = results;
        results.truncate(options.max_results);

        Ok(results)
    }

    /// Clears the result cache.
    pub fn clear_cache(&self) {
        let mut cache = self.inner.cache.write();
        cache.clear();
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn make_cache_key(&self, query: &str, options: &SpotlightSearchOptions) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            query,
            options.max_results,
            options.apply_exclusions,
            options.sort_by_recency,
            options
                .primary_scopes
                .iter()
                .chain(options.secondary_scopes.iter())
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn get_cached(&self, key: &str, ttl: Duration) -> Option<Vec<SpotlightResult>> {
        let mut cache = self.inner.cache.write();
        if let Some(cached) = cache.get(key) {
            if !cached.is_expired(ttl) {
                return Some(cached.results.clone());
            }
        }
        None
    }

    fn cache_results(&self, key: &str, results: Vec<SpotlightResult>) {
        let mut cache = self.inner.cache.write();
        cache.put(key.to_string(), CachedResult::new(results));
    }
}

impl Default for SpotlightSearchService {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// FileQuery Integration
// =============================================================================

/// Converts a FileQuery to an NSPredicate via PredicateBuilder.
fn file_query_to_predicate(
    file_query: &FileQuery,
    apply_exclusions: bool,
) -> Result<objc2::rc::Retained<objc2_foundation::NSPredicate>> {
    let mut builder = PredicateBuilder::new();

    // Add search terms (name contains)
    for term in &file_query.terms {
        if !term.is_empty() {
            builder = builder.name_contains(term);
        }
    }

    // Add file type filter
    if let Some(ref file_type) = file_query.file_type {
        match file_type {
            FileTypeFilter::Extension(ext) => {
                builder = builder.extension_is(ext);
            }
            FileTypeFilter::Category(category) => {
                // Use the UTI types from the category
                let utis = category.uti_types();
                if let Some(first_uti) = utis.first() {
                    builder = builder.content_type_tree(first_uti);
                }
            }
        }
    }

    // Apply default exclusions if requested
    if apply_exclusions {
        builder = builder.with_default_exclusions();
    }

    // Handle folder prioritization by adding folder type if specified
    if file_query.prioritize_folders {
        // When prioritizing folders, we don't filter - just note it for scoring later
    }

    Ok(builder.build())
}

/// Maps FileCategory to primary UTI string.
fn category_to_uti(category: &FileCategory) -> &'static str {
    match category {
        FileCategory::Documents => "public.document",
        FileCategory::Images => "public.image",
        FileCategory::Videos => "public.movie",
        FileCategory::Audio => "public.audio",
        FileCategory::Archives => "public.archive",
        FileCategory::Folders => "public.folder",
        FileCategory::Code => "public.source-code",
    }
}

/// Resolves a location scope to actual paths.
fn resolve_location_scope(location: &PathBuf) -> Vec<PathBuf> {
    if location.exists() {
        vec![location.clone()]
    } else {
        // Try resolving common location names
        let location_str = location.to_string_lossy().to_lowercase();
        let home = dirs::home_dir().unwrap_or_default();

        match location_str.as_str() {
            "desktop" => vec![home.join("Desktop")],
            "documents" | "docs" => vec![home.join("Documents")],
            "downloads" => vec![home.join("Downloads")],
            "pictures" | "photos" => vec![home.join("Pictures")],
            "music" => vec![home.join("Music")],
            "movies" | "videos" => vec![home.join("Movies")],
            "applications" | "apps" => vec![PathBuf::from("/Applications")],
            "home" | "~" => vec![home],
            _ => default_search_scopes(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = SpotlightSearchService::new();
        let _ = service;
    }

    #[test]
    fn test_default_options() {
        let options = SpotlightSearchOptions::default();
        assert_eq!(options.max_results, 50);
        assert!(options.use_cache);
        assert!(options.apply_exclusions);
        assert!(options.sort_by_recency);
        assert!(!options.primary_scopes.is_empty());
        assert!(!options.secondary_scopes.is_empty());
        assert_eq!(options.min_results_before_expand, 10);
    }

    #[test]
    fn test_raycast_style_options() {
        let options = SpotlightSearchOptions::raycast_style();
        assert!(options.apply_exclusions);
        assert!(options.sort_by_recency);
    }

    #[test]
    fn test_include_all_options() {
        let options = SpotlightSearchOptions::include_all();
        assert!(!options.apply_exclusions);
        assert!(!options.sort_by_recency);
    }

    #[test]
    fn test_in_directory_options() {
        let options = SpotlightSearchOptions::in_directory(PathBuf::from("/tmp"));
        assert_eq!(options.primary_scopes.len(), 1);
        assert_eq!(options.primary_scopes[0], PathBuf::from("/tmp"));
        assert!(options.secondary_scopes.is_empty());
    }

    #[test]
    fn test_resolve_location_scope() {
        let desktop = resolve_location_scope(&PathBuf::from("desktop"));
        assert_eq!(desktop.len(), 1);
        assert!(desktop[0].to_string_lossy().contains("Desktop"));

        let documents = resolve_location_scope(&PathBuf::from("documents"));
        assert_eq!(documents.len(), 1);
        assert!(documents[0].to_string_lossy().contains("Documents"));

        let apps = resolve_location_scope(&PathBuf::from("applications"));
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0], PathBuf::from("/Applications"));
    }

    #[test]
    fn test_category_to_uti() {
        assert_eq!(category_to_uti(&FileCategory::Images), "public.image");
        assert_eq!(category_to_uti(&FileCategory::Documents), "public.document");
        assert_eq!(category_to_uti(&FileCategory::Code), "public.source-code");
    }

    #[test]
    fn test_empty_query_error() {
        let service = SpotlightSearchService::new();
        let result = service.search("");
        assert!(result.is_err());

        if let Err(SearchServiceError::InvalidQuery(msg)) = result {
            assert!(msg.contains("empty"));
        } else {
            panic!("expected InvalidQuery error");
        }
    }

    #[test]
    fn test_cache_key_generation() {
        let service = SpotlightSearchService::new();
        let options = SpotlightSearchOptions::default();

        let key1 = service.make_cache_key("test", &options);
        let key2 = service.make_cache_key("test", &options);

        assert_eq!(key1, key2);

        let key3 = service.make_cache_key("other", &options);
        assert_ne!(key1, key3);
    }
}
