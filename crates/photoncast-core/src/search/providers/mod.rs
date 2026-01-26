//! Search providers for different result types.

pub mod apps;
pub mod apps_manage;
pub mod calendar;
pub mod commands;
pub mod custom_commands;
pub mod extension;
pub mod files;
pub mod optimized_apps;
pub mod quicklinks;
pub mod timer;
pub mod window;

pub use apps::AppProvider;
pub use apps_manage::AppsProvider;
pub use calendar::CalendarProvider;
pub use commands::CommandProvider;
pub use custom_commands::CustomCommandProvider;
pub use extension::ExtensionProvider;
pub use files::{FileProvider, FileUsageTracker, NoOpFileTracker, DEFAULT_FILE_MAX_RESULTS};
pub use optimized_apps::OptimizedAppProvider;
pub use quicklinks::QuickLinksProvider;
pub use timer::TimerProvider;
pub use window::WindowProvider;

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

    /// Returns a reference to the provider as `Any` for downcasting.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}
