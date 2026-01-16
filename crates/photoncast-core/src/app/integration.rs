//! Application integration module.
//!
//! This module handles registering all search providers, initializing the search engine,
//! and managing the application lifecycle.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use tracing::{debug, info, warn};

use crate::indexer::IndexedApp;
use crate::search::providers::{AppProvider, CommandProvider, FileProvider};
use crate::search::{SearchConfig, SearchEngine, SearchResults};

/// Default search timeout in milliseconds.
pub const DEFAULT_SEARCH_TIMEOUT_MS: u64 = 100;

/// Message shown when search times out.
pub const SEARCH_TIMEOUT_MESSAGE: &str = "Search took too long";

/// Configuration for the integrated search system.
#[derive(Debug, Clone)]
pub struct IntegrationConfig {
    /// Search timeout in milliseconds.
    pub search_timeout_ms: u64,
    /// Maximum results per provider.
    pub max_results_per_provider: usize,
    /// Maximum total results.
    pub max_total_results: usize,
    /// Whether to include file search.
    pub include_files: bool,
    /// File result limit.
    pub file_result_limit: usize,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            search_timeout_ms: DEFAULT_SEARCH_TIMEOUT_MS,
            max_results_per_provider: 10,
            max_total_results: 20,
            include_files: true,
            file_result_limit: 5,
        }
    }
}

/// Result of a search operation, including timeout status.
#[derive(Debug, Clone)]
pub struct SearchOutcome {
    /// The search results (may be partial if timed out).
    pub results: SearchResults,
    /// Whether the search timed out.
    pub timed_out: bool,
    /// Message to display to the user (if any).
    pub message: Option<String>,
}

impl SearchOutcome {
    /// Creates a successful search outcome.
    #[must_use]
    pub fn success(results: SearchResults) -> Self {
        Self {
            results,
            timed_out: false,
            message: None,
        }
    }

    /// Creates a timed-out search outcome with partial results.
    #[must_use]
    pub fn timeout(results: SearchResults) -> Self {
        Self {
            results,
            timed_out: true,
            message: Some(SEARCH_TIMEOUT_MESSAGE.to_string()),
        }
    }
}

/// The integrated PhotonCast application.
///
/// This struct holds all the major components of the application
/// and handles their initialization and coordination.
pub struct PhotonCastApp {
    /// The search engine with all providers registered.
    search_engine: SearchEngine,
    /// Shared app index for the app provider.
    app_index: Arc<RwLock<Vec<IndexedApp>>>,
    /// Configuration.
    config: IntegrationConfig,
}

impl PhotonCastApp {
    /// Creates a new PhotonCast application with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(IntegrationConfig::default())
    }

    /// Creates a new PhotonCast application with custom configuration.
    #[must_use]
    pub fn with_config(config: IntegrationConfig) -> Self {
        let app_index = Arc::new(RwLock::new(Vec::new()));

        // Create the search engine with configured settings
        let search_config = SearchConfig {
            max_results_per_provider: config.max_results_per_provider,
            max_total_results: config.max_total_results,
            timeout: Duration::from_millis(config.search_timeout_ms),
        };
        let mut search_engine = SearchEngine::with_config(search_config);

        // Register all search providers (Task 3.10.1)
        Self::register_providers(&mut search_engine, Arc::clone(&app_index), &config);

        Self {
            search_engine,
            app_index,
            config,
        }
    }

    /// Registers all search providers with the engine (Task 3.10.1).
    fn register_providers(
        engine: &mut SearchEngine,
        app_index: Arc<RwLock<Vec<IndexedApp>>>,
        config: &IntegrationConfig,
    ) {
        info!("Registering search providers...");

        // 1. App Provider (highest priority)
        let app_provider = AppProvider::with_apps(app_index);
        engine.add_provider(app_provider);
        debug!("Registered AppProvider");

        // 2. Command Provider (medium priority)
        let command_provider = CommandProvider::new();
        engine.add_provider(command_provider);
        debug!("Registered CommandProvider");

        // 3. File Provider (lower priority, optional)
        if config.include_files {
            let file_provider = FileProvider::new(config.file_result_limit);
            engine.add_provider(file_provider);
            debug!(
                "Registered FileProvider with limit={}",
                config.file_result_limit
            );
        }

        info!(
            "Search providers registered: {} providers",
            engine.provider_count()
        );
    }

    /// Performs a search with timeout handling (Task 3.10.2).
    ///
    /// Returns partial results if the timeout is exceeded.
    #[must_use]
    pub fn search(&self, query: &str) -> SearchOutcome {
        if query.is_empty() {
            return SearchOutcome::success(SearchResults::empty());
        }

        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(self.config.search_timeout_ms);

        // Perform the search
        let results = self.search_engine.search_sync(query);
        let elapsed = start.elapsed();

        // Check if we exceeded the timeout
        if elapsed > timeout {
            warn!(
                query = %query,
                elapsed_ms = elapsed.as_millis(),
                timeout_ms = timeout.as_millis(),
                "Search timed out"
            );
            SearchOutcome::timeout(results)
        } else {
            debug!(
                query = %query,
                elapsed_ms = elapsed.as_millis(),
                result_count = results.total_count,
                "Search completed"
            );
            SearchOutcome::success(results)
        }
    }

    /// Performs an async search with timeout handling.
    pub async fn search_async(&self, query: &str) -> SearchOutcome {
        if query.is_empty() {
            return SearchOutcome::success(SearchResults::empty());
        }

        let timeout = Duration::from_millis(self.config.search_timeout_ms);
        let start = std::time::Instant::now();

        // Create a timeout wrapper around the search
        let search_future = self.search_engine.search(query);

        match tokio::time::timeout(timeout, search_future).await {
            Ok(results) => {
                let elapsed = start.elapsed();
                if elapsed > timeout {
                    // Search completed but took longer than expected
                    warn!(
                        query = %query,
                        elapsed_ms = elapsed.as_millis(),
                        "Search exceeded soft timeout"
                    );
                    SearchOutcome::timeout(results)
                } else {
                    debug!(
                        query = %query,
                        elapsed_ms = elapsed.as_millis(),
                        result_count = results.total_count,
                        "Async search completed"
                    );
                    SearchOutcome::success(results)
                }
            },
            Err(_) => {
                // Hard timeout - return empty results
                warn!(query = %query, "Search hard timeout exceeded");
                SearchOutcome::timeout(SearchResults::empty())
            },
        }
    }

    /// Updates the app index with new apps.
    pub fn set_apps(&self, apps: Vec<IndexedApp>) {
        *self.app_index.write() = apps;
    }

    /// Adds apps to the index.
    pub fn add_apps(&self, apps: impl IntoIterator<Item = IndexedApp>) {
        self.app_index.write().extend(apps);
    }

    /// Updates the icon path for an app by bundle ID.
    pub fn update_app_icon(&self, bundle_id: &str, icon_path: std::path::PathBuf) {
        let mut apps = self.app_index.write();
        if let Some(app) = apps.iter_mut().find(|a| a.bundle_id.as_str() == bundle_id) {
            app.icon_path = Some(icon_path);
        }
    }

    /// Removes an app from the index by its path.
    ///
    /// Returns `true` if an app was removed, `false` if no app was found at that path.
    pub fn remove_app_by_path(&self, path: &std::path::Path) -> bool {
        let mut apps = self.app_index.write();
        let initial_len = apps.len();
        apps.retain(|app| app.path != path);
        let removed = apps.len() < initial_len;
        if removed {
            debug!(path = %path.display(), "Removed app from index");
        }
        removed
    }

    /// Updates an existing app in the index, or adds it if not present.
    ///
    /// This is useful when an app is modified and needs to be re-indexed.
    pub fn update_or_add_app(&self, app: IndexedApp) {
        let mut apps = self.app_index.write();
        if let Some(existing) = apps.iter_mut().find(|a| a.path == app.path) {
            debug!(path = %app.path.display(), name = %app.name, "Updated app in index");
            *existing = app;
        } else {
            debug!(path = %app.path.display(), name = %app.name, "Added new app to index");
            apps.push(app);
        }
    }

    /// Returns the number of indexed apps.
    #[must_use]
    pub fn app_count(&self) -> usize {
        self.app_index.read().len()
    }

    /// Returns a reference to the search engine.
    #[must_use]
    pub fn search_engine(&self) -> &SearchEngine {
        &self.search_engine
    }

    /// Returns the configuration.
    #[must_use]
    pub fn config(&self) -> &IntegrationConfig {
        &self.config
    }
}

impl Default for PhotonCastApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::AppBundleId;
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_app(name: &str, bundle_id: &str) -> IndexedApp {
        IndexedApp {
            name: name.to_string(),
            bundle_id: AppBundleId::new(bundle_id),
            path: PathBuf::from(format!("/Applications/{}.app", name)),
            icon_path: None,
            category: None,
            keywords: Vec::new(),
            last_modified: Utc::now(),
        }
    }

    #[test]
    fn test_photoncast_app_new() {
        let app = PhotonCastApp::new();
        assert_eq!(app.search_engine.provider_count(), 3); // apps, commands, files
        assert_eq!(app.app_count(), 0);
    }

    #[test]
    fn test_photoncast_app_without_files() {
        let config = IntegrationConfig {
            include_files: false,
            ..Default::default()
        };
        let app = PhotonCastApp::with_config(config);
        assert_eq!(app.search_engine.provider_count(), 2); // apps, commands only
    }

    #[test]
    fn test_photoncast_app_set_apps() {
        let app = PhotonCastApp::new();
        assert_eq!(app.app_count(), 0);

        app.set_apps(vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Finder", "com.apple.finder"),
        ]);
        assert_eq!(app.app_count(), 2);
    }

    #[test]
    fn test_photoncast_app_add_apps() {
        let app = PhotonCastApp::new();
        app.add_apps(vec![create_test_app("Safari", "com.apple.Safari")]);
        assert_eq!(app.app_count(), 1);

        app.add_apps(vec![create_test_app("Finder", "com.apple.finder")]);
        assert_eq!(app.app_count(), 2);
    }

    #[test]
    fn test_search_empty_query() {
        let app = PhotonCastApp::new();
        let outcome = app.search("");
        assert!(!outcome.timed_out);
        assert!(outcome.results.is_empty());
    }

    fn create_test_app_with_timeout() -> PhotonCastApp {
        // Use a longer timeout for tests to avoid flakiness
        let config = IntegrationConfig {
            search_timeout_ms: 1000, // 1 second for tests
            ..IntegrationConfig::default()
        };
        PhotonCastApp::with_config(config)
    }

    #[test]
    fn test_search_with_apps() {
        let app = create_test_app_with_timeout();
        app.set_apps(vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("System Preferences", "com.apple.systempreferences"),
        ]);

        let outcome = app.search("saf");
        assert!(!outcome.timed_out, "search should not timeout");
        assert!(!outcome.results.is_empty());
    }

    #[test]
    fn test_search_commands() {
        let app = create_test_app_with_timeout();
        let outcome = app.search("sleep");
        assert!(!outcome.timed_out, "search should not timeout");
        // Should find Sleep command
        assert!(outcome.results.total_count > 0);
    }

    #[test]
    fn test_search_outcome_success() {
        let outcome = SearchOutcome::success(SearchResults::empty());
        assert!(!outcome.timed_out);
        assert!(outcome.message.is_none());
    }

    #[test]
    fn test_search_outcome_timeout() {
        let outcome = SearchOutcome::timeout(SearchResults::empty());
        assert!(outcome.timed_out);
        assert_eq!(outcome.message, Some(SEARCH_TIMEOUT_MESSAGE.to_string()));
    }

    #[test]
    fn test_integration_config_default() {
        let config = IntegrationConfig::default();
        assert_eq!(config.search_timeout_ms, DEFAULT_SEARCH_TIMEOUT_MS);
        assert!(config.include_files);
    }

    #[tokio::test]
    async fn test_search_async() {
        let app = create_test_app_with_timeout();
        app.set_apps(vec![create_test_app("Safari", "com.apple.Safari")]);

        let outcome = app.search_async("safari").await;
        assert!(!outcome.timed_out, "async search should not timeout");
        assert!(!outcome.results.is_empty());
    }

    #[test]
    fn test_remove_app_by_path() {
        let app = PhotonCastApp::new();
        let safari = create_test_app("Safari", "com.apple.Safari");
        let safari_path = safari.path.clone();
        let finder = create_test_app("Finder", "com.apple.finder");

        app.set_apps(vec![safari, finder]);
        assert_eq!(app.app_count(), 2);

        // Remove Safari
        let removed = app.remove_app_by_path(&safari_path);
        assert!(removed);
        assert_eq!(app.app_count(), 1);

        // Try to remove again - should return false
        let removed_again = app.remove_app_by_path(&safari_path);
        assert!(!removed_again);
        assert_eq!(app.app_count(), 1);

        // Try to remove non-existent path
        let removed_fake = app.remove_app_by_path(&PathBuf::from("/Applications/Fake.app"));
        assert!(!removed_fake);
        assert_eq!(app.app_count(), 1);
    }

    #[test]
    fn test_update_or_add_app_new() {
        let app = PhotonCastApp::new();
        assert_eq!(app.app_count(), 0);

        // Add new app
        let safari = create_test_app("Safari", "com.apple.Safari");
        app.update_or_add_app(safari);
        assert_eq!(app.app_count(), 1);

        // Add another new app
        let finder = create_test_app("Finder", "com.apple.finder");
        app.update_or_add_app(finder);
        assert_eq!(app.app_count(), 2);
    }

    #[test]
    fn test_update_or_add_app_existing() {
        let app = PhotonCastApp::new();
        let safari = create_test_app("Safari", "com.apple.Safari");
        app.set_apps(vec![safari]);
        assert_eq!(app.app_count(), 1);

        // Update existing app (same path, different name)
        let updated_safari = IndexedApp {
            name: "Safari Updated".to_string(),
            bundle_id: AppBundleId::new("com.apple.Safari"),
            path: PathBuf::from("/Applications/Safari.app"),
            icon_path: None,
            category: None,
            keywords: vec!["browser".to_string()],
            last_modified: Utc::now(),
        };
        app.update_or_add_app(updated_safari);

        // Should still have 1 app, not 2
        assert_eq!(app.app_count(), 1);

        // Verify the app was updated (we can check by searching)
        let outcome = app.search("Updated");
        // The search should find the updated name
        assert!(
            outcome
                .results
                .iter()
                .any(|r| r.title.contains("Updated")),
            "Should find updated app name"
        );
    }
}
