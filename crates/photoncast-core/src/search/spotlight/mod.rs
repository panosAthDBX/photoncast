//! Native macOS Spotlight integration using objc2.
//!
//! This module provides native access to macOS Spotlight search functionality
//! using the objc2 crates, bypassing the need for CLI tools like `mdfind`.
//!
//! # Architecture
//!
//! - [`PredicateBuilder`] - Builder pattern for creating NSPredicate queries
//! - [`SpotlightResult`] - Rich metadata extraction from query results
//! - [`MetadataExtractor`] - Converts NSMetadataItem to Rust types
//! - [`MetadataQueryWrapper`] - NSMetadataQuery wrapper for executing searches
//! - [`SpotlightSearchService`] - High-level search service with caching
//!
//! # Example
//!
//! ```no_run
//! use photoncast_core::search::spotlight::{SpotlightSearchService, SpotlightSearchOptions};
//! use std::time::Duration;
//!
//! let service = SpotlightSearchService::new();
//!
//! // Simple search
//! let results = service.search("report").unwrap();
//!
//! // With options
//! let results = service.search_with_options("budget", &SpotlightSearchOptions {
//!     max_results: 20,
//!     timeout: Duration::from_millis(500),
//!     ..Default::default()
//! }).unwrap();
//! ```

#[cfg(target_os = "macos")]
pub mod predicate;

#[cfg(target_os = "macos")]
pub mod query;

#[cfg(target_os = "macos")]
pub mod service;

#[cfg(target_os = "macos")]
pub mod prefetch;

#[cfg(target_os = "macos")]
pub mod live_index;

// Result module is available on all platforms (SpotlightResult is platform-independent,
// MetadataExtractor has macOS impl and stub for other platforms)
pub mod result;

// Re-export predicate types (macOS only)
#[cfg(target_os = "macos")]
pub use predicate::{
    escape_predicate_string, PredicateBuilder, MD_ITEM_CONTENT_TYPE, MD_ITEM_CONTENT_TYPE_TREE,
    MD_ITEM_DISPLAY_NAME, MD_ITEM_FS_CONTENT_CHANGE_DATE, MD_ITEM_FS_NAME, MD_ITEM_FS_SIZE,
    MD_ITEM_LAST_USED_DATE, MD_ITEM_PATH, UTI_ARCHIVE, UTI_AUDIO, UTI_DOCUMENT, UTI_FOLDER,
    UTI_IMAGE, UTI_MOVIE, UTI_PDF, UTI_PLAIN_TEXT, UTI_SOURCE_CODE,
};

// Re-export query types (macOS only)
#[cfg(target_os = "macos")]
pub use query::{
    default_search_scopes, expanded_search_scopes, MetadataQueryWrapper, SpotlightError,
};

// Re-export service types (macOS only)
#[cfg(target_os = "macos")]
pub use service::{SearchServiceError, SpotlightSearchOptions, SpotlightSearchService};

// Re-export prefetch types (macOS only)
#[cfg(target_os = "macos")]
pub use prefetch::{
    start_background_prefetch, start_background_prefetch_with_service, CancellationToken,
    PrefetchConfig, PrefetchStatus, SpotlightPrefetcher,
};

// Re-export live index types (macOS only)
#[cfg(target_os = "macos")]
pub use live_index::{
    primary_scopes, start_live_index, CustomScopeConfig, LiveFileIndex, LiveIndexStats,
    LiveIndexStatus,
};

// Re-export result types for all platforms
pub use result::{MetadataExtractor, SpotlightResult};
