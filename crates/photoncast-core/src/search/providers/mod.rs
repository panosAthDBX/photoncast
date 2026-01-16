//! Search providers for different result types.

pub mod apps;
pub mod commands;
pub mod files;
pub mod optimized_apps;

pub use apps::AppProvider;
pub use commands::CommandProvider;
pub use files::{FileProvider, FileUsageTracker, NoOpFileTracker, DEFAULT_FILE_MAX_RESULTS};
pub use optimized_apps::OptimizedAppProvider;

use crate::search::{ResultType, SearchResult};

/// Trait for search providers.
pub trait SearchProvider: Send + Sync {
    /// Returns the name of this provider.
    fn name(&self) -> &str;

    /// Returns the result type this provider produces.
    fn result_type(&self) -> ResultType;

    /// Performs a search and returns matching results.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query.
    /// * `max_results` - Maximum number of results to return.
    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult>;
}
