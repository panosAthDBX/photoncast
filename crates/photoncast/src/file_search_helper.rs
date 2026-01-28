//! Optimized file search helper with parallel processing and caching.
//!
//! Performance optimizations:
//! - Live file index with Spotlight monitoring (fastest, primary scopes)
//! - Native Spotlight integration via SpotlightSearchService (preferred fallback)
//! - Parallel metadata fetching with rayon (legacy fallback)
//! - Adaptive debounce based on query length

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use once_cell::sync::Lazy;
use rayon::prelude::*;

use photoncast_core::platform::spotlight::{FileKind, FileResult};
use photoncast_core::search::spotlight::{
    CustomScopeConfig, LiveFileIndex, SpotlightResult, SpotlightSearchOptions,
    SpotlightSearchService,
};

use crate::constants::{
    APP_EXTENSIONS, ARCHIVE_EXTENSIONS, AUDIO_EXTENSIONS, DOCUMENT_EXTENSIONS, EBOOK_EXTENSIONS,
    IMAGE_EXTENSIONS, VIDEO_EXTENSIONS,
};

// =============================================================================
// Global Live File Index (thread-safe, Spotlight-monitored)
// =============================================================================

/// Global LiveFileIndex instance for instant in-memory file searches.
/// Monitors configured search scopes + any custom scopes from config.
static LIVE_INDEX: Lazy<LiveFileIndex> = Lazy::new(|| {
    // Load search scopes from config (defaults to home directory)
    let config = photoncast_core::app::config_file::load_config().unwrap_or_default();
    let search_scopes = config.file_search.search_scopes.clone();
    let custom_scopes = load_custom_scopes_from_config();

    tracing::debug!(
        "Initializing live index with {} search scopes, {} custom scopes",
        search_scopes.len(),
        custom_scopes.len()
    );

    let index = LiveFileIndex::with_custom_scopes(search_scopes, custom_scopes);
    index.start();
    index
});

/// Loads custom search scopes from the app config.
fn load_custom_scopes_from_config() -> Vec<CustomScopeConfig> {
    let config = photoncast_core::app::config_file::load_config().unwrap_or_default();

    config
        .file_search
        .custom_scopes
        .into_iter()
        .map(|scope| CustomScopeConfig {
            path: scope.resolved_path(),
            extensions: scope.extensions,
            recursive: scope.recursive,
        })
        .collect()
}

/// Pre-initializes the live file index in the background.
/// Call this early at app startup to ensure the index is ready
/// when the user opens file search.
///
/// The index takes ~7-8 seconds to populate initially, so calling
/// this early gives time for population to complete.
pub fn init_live_index() {
    // Force initialization of the lazy static
    let _ = &*LIVE_INDEX;
    tracing::debug!("Live file index initialization started");
}

/// Returns true if the live index is ready for instant searches.
#[allow(dead_code)]
pub fn is_live_index_ready() -> bool {
    LIVE_INDEX.is_ready()
}

/// Reloads the live index with updated config.
/// Call this when file search settings change in preferences.
pub fn reload_live_index() {
    let config = photoncast_core::app::config_file::load_config().unwrap_or_default();
    let search_scopes = config.file_search.search_scopes.clone();
    let custom_scopes = load_custom_scopes_from_config();

    tracing::debug!(
        "Reloading live index with {} search scopes, {} custom scopes",
        search_scopes.len(),
        custom_scopes.len()
    );

    LIVE_INDEX.reload(search_scopes, custom_scopes);
}

/// Returns the current custom scopes from the live index.
#[allow(dead_code)]
pub fn get_custom_scopes() -> Vec<CustomScopeConfig> {
    LIVE_INDEX.custom_scopes()
}

// =============================================================================
// Global Spotlight Search Service (thread-safe with built-in caching)
// =============================================================================

/// Global SpotlightSearchService instance for file searches.
/// The service has built-in LRU caching with 30s TTL.
/// Used as fallback when live index doesn't have results.
static SPOTLIGHT_SERVICE: Lazy<SpotlightSearchService> = Lazy::new(|| {
    SpotlightSearchService::with_options(SpotlightSearchOptions {
        max_results: 100,
        timeout: Duration::from_millis(500),
        use_cache: true,
        cache_ttl: Duration::from_secs(30),
        ..SpotlightSearchOptions::default()
    })
});

// =============================================================================
// SpotlightResult to FileResult Conversion
// =============================================================================

/// Converts a SpotlightResult to a FileResult for UI display.
fn spotlight_result_to_file_result(result: &SpotlightResult) -> FileResult {
    let kind = if result.is_directory {
        FileKind::Folder
    } else if result.is_application() {
        FileKind::Application
    } else if result.is_image() {
        FileKind::Image
    } else if result.is_document() {
        FileKind::Document
    } else if result.conforms_to_type("public.audio") {
        FileKind::Audio
    } else if result.conforms_to_type("public.movie") {
        FileKind::Video
    } else {
        FileKind::from_path(&result.path)
    };

    FileResult {
        path: result.path.clone(),
        name: result.display_name.clone(),
        kind,
        size: result.file_size,
        modified: result.modified_date,
    }
}

// =============================================================================
// Native Spotlight Search Functions (Preferred)
// =============================================================================

/// Searches for files using the fastest available method:
/// 1. Live index (instant, in-memory) - for primary scopes
/// 2. Native Spotlight service (cached) - for broader searches
/// 3. mdfind CLI (fallback) - if all else fails
///
/// # Arguments
/// * `query` - The search query (file name to search for)
/// * `max_results` - Maximum number of results to return
///
/// # Returns
/// A vector of FileResult objects, or an empty vector on error.
pub fn spotlight_search(query: &str, max_results: usize) -> Vec<FileResult> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }

    // Try live index first (instant, in-memory filtering)
    // The live index is already filtered to only contain whitelisted files
    if LIVE_INDEX.is_ready() {
        let live_results = LIVE_INDEX.search(query, max_results);
        tracing::debug!(
            "Live index returned {} results for '{}'",
            live_results.len(),
            query
        );
        // Return live results even if empty - the index is authoritative
        // when ready. Don't fall through to unfiltered fallback.
        return live_results
            .into_iter()
            .map(|r| spotlight_result_to_file_result(&r))
            .collect();
    }

    // Fallback: Live index not ready yet, use Spotlight service with filtering
    tracing::debug!("Live index not ready, using Spotlight service fallback");
    match SPOTLIGHT_SERVICE.search(query) {
        Ok(results) => {
            // CRITICAL: Apply whitelist filtering to fallback results
            results
                .into_iter()
                .filter(|r| is_interesting_file(&r.path))
                .take(max_results)
                .map(|r| spotlight_result_to_file_result(&r))
                .collect()
        },
        Err(e) => {
            tracing::warn!("Native Spotlight search failed: {e}, falling back to mdfind");
            let scope = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            let mdfind_query = format!("kMDItemDisplayName == '*{}*'c", query);
            let paths = mdfind_paths_internal(&mdfind_query, &scope, max_results * 2);
            // Filter mdfind results too
            let filtered_paths: Vec<PathBuf> = paths
                .into_iter()
                .filter(|p| is_interesting_file(p))
                .collect();
            let files_with_time = fetch_metadata_parallel(filtered_paths, max_results);
            to_file_results(files_with_time)
        },
    }
}

/// Fetches recent files using the live index or mdfind fallback.
///
/// Priority:
/// 1. Live index (instant, in-memory) - files from Desktop, Documents, Downloads
/// 2. mdfind CLI (fallback) - for time-based queries if live index not ready
///
/// # Arguments
/// * `_days` - Number of days to look back (used by mdfind fallback)
/// * `max_results` - Maximum number of results to return
///
/// # Returns
/// A vector of FileResult objects sorted by modification time (newest first).
pub fn spotlight_recent_files(_days: u32, max_results: usize) -> Vec<FileResult> {
    // Try live index first (instant, already sorted by last used)
    if LIVE_INDEX.is_ready() {
        let recent = LIVE_INDEX.get_recent_files(max_results);
        if !recent.is_empty() {
            tracing::debug!("Live index returned {} recent files", recent.len());
            return recent
                .into_iter()
                .map(|r| spotlight_result_to_file_result(&r))
                .collect();
        }
    }

    // Fallback to mdfind if live index not ready
    tracing::debug!("Live index not ready, falling back to mdfind for recent files");
    let mdfind_query = format!("kMDItemFSContentChangeDate >= $time.today(-{})", _days);
    let scope = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

    // Get paths from mdfind (fast for time-based queries)
    // Request more results since we'll filter many out
    let paths = mdfind_paths_internal(&mdfind_query, &scope, max_results * 10);

    if paths.is_empty() {
        return Vec::new();
    }

    // Filter to only interesting files (apply same whitelist as live index)
    let filtered_paths: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| is_interesting_file(p))
        .collect();

    // Use parallel metadata fetching for performance
    let files_with_time = fetch_metadata_parallel(filtered_paths, max_results);
    to_file_results(files_with_time)
}

/// Fetches recent files of a specific type using the live index.
///
/// This function looks through ALL files in the live index and filters
/// them by type, ensuring we return up to `max_results` files of the
/// requested type (not just the top N files filtered).
///
/// # Arguments
/// * `filter` - The file type filter to apply
/// * `max_results` - Maximum number of results to return
///
/// # Returns
/// A vector of FileResult objects of the specified type, sorted by recency.
pub fn spotlight_recent_files_filtered(
    filter: crate::file_search_view::FileTypeFilter,
    max_results: usize,
) -> Vec<FileResult> {
    use crate::file_search_view::FileTypeFilter;

    // If filter is All, just use the regular function
    if filter == FileTypeFilter::All {
        return spotlight_recent_files(7, max_results);
    }

    // For specific filters, look through ALL files in the index (not just top N)
    // This ensures we find files matching the filter even if they're not the most recent overall
    if LIVE_INDEX.is_ready() {
        // Get ALL files from the index, sorted by recency
        // We need to look through all files to find ones matching the specific type
        let all_files = LIVE_INDEX.get_all_files(10000); // Get up to 10k files

        tracing::debug!(
            "[FilterDebug] Got {} files from index for {:?} filter",
            all_files.len(),
            filter
        );

        // Convert and filter, collecting only matching files up to max_results
        let filtered: Vec<FileResult> = all_files
            .into_iter()
            .map(|r| spotlight_result_to_file_result(&r))
            .filter(|f| filter.matches(f.kind, &f.path))
            .take(max_results)
            .collect();

        tracing::debug!(
            "[FilterDebug] spotlight_recent_files_filtered: {:?} returned {} files",
            filter,
            filtered.len()
        );

        // Log first few matching files for debugging
        if !filtered.is_empty() {
            for (i, f) in filtered.iter().take(3).enumerate() {
                tracing::debug!("[FilterDebug]   [{}] {:?} {}", i, f.kind, f.path.display());
            }
        }

        return filtered;
    }

    // Fallback: use mdfind with type-specific query
    tracing::debug!(
        "Live index not ready, using mdfind fallback for {:?}",
        filter
    );
    let mdfind_query = filter.mdfind_query();
    let scope = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

    let paths = mdfind_paths_internal(mdfind_query, &scope, max_results * 2);
    let files_with_time = fetch_metadata_parallel(paths, max_results);
    to_file_results(files_with_time)
}

/// Clears the Spotlight search service cache.
#[allow(dead_code)]
pub fn clear_spotlight_cache() {
    SPOTLIGHT_SERVICE.clear_cache();
}

/// Returns adaptive debounce duration based on query length.
/// - Short queries (< 3 chars): 150ms (more results, need more time)
/// - Medium queries (3-5 chars): 100ms
/// - Long queries (> 5 chars): 50ms (more specific, faster results)
#[must_use]
pub fn adaptive_debounce_ms(query_len: usize) -> u64 {
    match query_len {
        0..=2 => 150,
        3..=5 => 100,
        _ => 50,
    }
}

/// Fetches file metadata in parallel using rayon.
/// Returns files sorted by modification time (newest first).
///
/// This is significantly faster than sequential metadata fetching:
/// - Sequential: O(n) * disk_latency
/// - Parallel: O(n/cores) * disk_latency
pub fn fetch_metadata_parallel(
    paths: Vec<PathBuf>,
    max_results: usize,
) -> Vec<(PathBuf, SystemTime)> {
    // Use rayon to fetch metadata in parallel
    let mut files_with_time: Vec<(PathBuf, SystemTime)> = paths
        .into_par_iter()
        .filter_map(|path| {
            // Skip hidden files
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    return None;
                }
            }
            // Get metadata (this is the expensive I/O operation)
            let metadata = std::fs::metadata(&path).ok()?;
            let mtime = metadata.modified().ok()?;
            Some((path, mtime))
        })
        .collect();

    // Sort by modification time (newest first)
    files_with_time.sort_by(|a, b| b.1.cmp(&a.1));

    // Take only what we need
    files_with_time.truncate(max_results);
    files_with_time
}

/// Converts paths with metadata to FileResult objects.
pub fn to_file_results(files_with_time: Vec<(PathBuf, SystemTime)>) -> Vec<FileResult> {
    files_with_time
        .into_iter()
        .map(|(path, mtime)| {
            let mut result = FileResult::from_path(path);
            result.modified = Some(mtime);
            result
        })
        .collect()
}

/// Check if a file extension is an "interesting" user file type.
/// Only actual user files - documents, images, videos, audio. NO code files.
fn is_interesting_extension(ext: &str) -> bool {
    DOCUMENT_EXTENSIONS.contains(&ext)
        || IMAGE_EXTENSIONS.contains(&ext)
        || VIDEO_EXTENSIONS.contains(&ext)
        || AUDIO_EXTENSIONS.contains(&ext)
        || ARCHIVE_EXTENSIONS.contains(&ext)
        || EBOOK_EXTENSIONS.contains(&ext)
        || APP_EXTENSIONS.contains(&ext)
}

/// Directories to exclude from results.
const EXCLUDED_DIRS: &[&str] = &[
    "Library",
    "Caches",
    "Cache",
    ".Trash",
    "node_modules",
    ".git",
    "target",
    "__pycache__",
    ".venv",
    "DerivedData",
    "Cookies",
    "Application Support",
    "Containers",
    "WebKit",
    ".npm",
    ".cargo",
    ".rustup",
];

/// Checks if a file path is "interesting" (human-readable, not system junk).
fn is_interesting_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Exclude system directories
    for dir in EXCLUDED_DIRS {
        if path_str.contains(&format!("/{}/", dir)) {
            return false;
        }
    }

    // Check filename
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        // Exclude hidden files
        if name.starts_with('.') {
            return false;
        }

        // Check extension whitelist
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            return is_interesting_extension(&ext_lower);
        }
    }

    false
}

// =============================================================================
// Legacy mdfind Functions (Fallback for time-based queries)
// =============================================================================

/// Internal mdfind execution - not deprecated since it's needed for time-based queries.
/// The SpotlightSearchService doesn't yet support date predicates natively.
fn mdfind_paths_internal(query: &str, scope: &PathBuf, limit: usize) -> Vec<PathBuf> {
    let output = std::process::Command::new("mdfind")
        .arg("-onlyin")
        .arg(scope)
        .arg(query)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .filter(|line| !line.is_empty())
                .take(limit)
                .map(PathBuf::from)
                .collect()
        },
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_debounce() {
        assert_eq!(adaptive_debounce_ms(0), 150);
        assert_eq!(adaptive_debounce_ms(1), 150);
        assert_eq!(adaptive_debounce_ms(2), 150);
        assert_eq!(adaptive_debounce_ms(3), 100);
        assert_eq!(adaptive_debounce_ms(5), 100);
        assert_eq!(adaptive_debounce_ms(6), 50);
        assert_eq!(adaptive_debounce_ms(10), 50);
    }
}
