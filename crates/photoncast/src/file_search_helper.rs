//! Optimized file search helper with live-index first lookups.
//!
//! Performance optimizations:
//! - Live file index with Spotlight monitoring (fastest, primary scopes)
//! - Unified file backend fallback for Spotlight/date-based lookups
//! - Adaptive debounce based on query length

use std::path::PathBuf;
use std::time::Duration;

use once_cell::sync::Lazy;

use photoncast_core::platform::spotlight::{FileKind, FileResult};
use photoncast_core::search::spotlight::{CustomScopeConfig, LiveFileIndex, SpotlightResult};
use photoncast_core::search::{FileSearchBackend, FileSearchBackendOptions, FileSearchStrategy};

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
// Global Unified File Search Backend
// =============================================================================

/// Global unified file search backend for non-live-index lookups.
static FILE_BACKEND: Lazy<FileSearchBackend> = Lazy::new(|| {
    FileSearchBackend::new(FileSearchStrategy::SpotlightWithFallback {
        timeout: Duration::from_millis(500),
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
/// 2. Unified file backend (Spotlight with strict fallback)
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

    tracing::debug!("Live index not ready, using unified backend fallback");
    let options = FileSearchBackendOptions {
        root: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        max_results,
        recent_days: None,
    };

    FILE_BACKEND.search_sync(query, &options)
}

/// Fetches recent files using the live index or unified backend fallback.
///
/// Priority:
/// 1. Live index (instant, in-memory) - files from indexed scopes
/// 2. Unified backend recent-query path - for date-based lookups when index is not ready
///
/// # Arguments
/// * `days` - Number of days to look back
/// * `max_results` - Maximum number of results to return
///
/// # Returns
/// A vector of FileResult objects sorted by modification time (newest first).
pub fn spotlight_recent_files(days: u32, max_results: usize) -> Vec<FileResult> {
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

    tracing::debug!("Live index not ready, using unified backend for recent files");
    let options = FileSearchBackendOptions {
        root: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        max_results,
        recent_days: Some(days),
    };

    FILE_BACKEND.search_recent_sync(&options, days)
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

    // Fallback: use unified backend and apply filter in-process.
    tracing::debug!(
        "Live index not ready, using unified backend fallback for {:?}",
        filter
    );
    let options = FileSearchBackendOptions {
        root: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        max_results: max_results * 4,
        recent_days: Some(30),
    };

    let mut results = FILE_BACKEND.search_recent_sync(&options, 30);
    results.retain(|file| filter.matches(file.kind, &file.path));
    if results.len() > max_results {
        results.truncate(max_results);
    }
    results
}

/// Clears backend-managed caches.
#[allow(dead_code)]
pub fn clear_spotlight_cache() {
    // Live index cache/state remains managed by LiveFileIndex.
    // Backend currently performs per-search fresh execution.
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
