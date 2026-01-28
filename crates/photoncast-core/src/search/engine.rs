//! Search orchestration.
//!
//! This module contains the main search engine that coordinates queries
//! across multiple providers and merges results.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::search::providers::SearchProvider;
use crate::search::{ResultGroup, ResultType, SearchResult, SearchResults};

/// Default maximum results per provider.
const DEFAULT_MAX_RESULTS_PER_PROVIDER: usize = 10;

/// Default maximum total results.
const DEFAULT_MAX_TOTAL_RESULTS: usize = 20;

/// Search configuration.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Maximum results per provider.
    pub max_results_per_provider: usize,
    /// Maximum total results.
    pub max_total_results: usize,
    /// Search timeout.
    pub timeout: Duration,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results_per_provider: DEFAULT_MAX_RESULTS_PER_PROVIDER,
            max_total_results: DEFAULT_MAX_TOTAL_RESULTS,
            timeout: Duration::from_millis(100),
        }
    }
}

/// The main search engine that coordinates queries across providers.
///
/// The search engine:
/// - Dispatches queries to all registered providers
/// - Collects and merges results from all providers
/// - Groups results by type (Apps, Commands, Files)
/// - Sorts results by score within each group
pub struct SearchEngine {
    /// Registered search providers.
    providers: Vec<Arc<dyn SearchProvider>>,
    /// Search configuration.
    config: SearchConfig,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
    /// Creates a new search engine with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            config: SearchConfig::default(),
        }
    }

    /// Creates a new search engine with custom configuration.
    #[must_use]
    pub fn with_config(config: SearchConfig) -> Self {
        Self {
            providers: Vec::new(),
            config,
        }
    }

    /// Adds a search provider to the engine.
    pub fn add_provider(&mut self, provider: impl SearchProvider + 'static) {
        self.providers.push(Arc::new(provider));
    }

    /// Adds a search provider to the engine from an existing Arc.
    /// This allows keeping a reference to the provider for cache invalidation.
    pub fn add_provider_arc(&mut self, provider: Arc<impl SearchProvider + 'static>) {
        self.providers.push(provider);
    }

    /// Returns a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &SearchConfig {
        &self.config
    }

    /// Sets the search configuration.
    pub fn set_config(&mut self, config: SearchConfig) {
        self.config = config;
    }

    /// Performs a search across all providers.
    ///
    /// This method queries all registered providers synchronously and merges
    /// the results. For async parallel execution, use `search_parallel`.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string.
    ///
    /// # Returns
    ///
    /// Grouped search results sorted by relevance.
    pub fn search_sync(&self, query: &str) -> SearchResults {
        if query.is_empty() {
            return SearchResults::empty();
        }

        let start = std::time::Instant::now();

        // Collect results from all providers
        let mut all_results: Vec<SearchResult> = Vec::new();

        for provider in &self.providers {
            let provider_results = provider.search(query, self.config.max_results_per_provider);
            all_results.extend(provider_results);
        }

        // Build the search results
        self.build_results(query, all_results, start.elapsed())
    }

    /// Performs a search across all providers asynchronously.
    ///
    /// Providers are executed in parallel on the blocking thread pool and the
    /// results are merged once all tasks complete.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string.
    ///
    /// # Returns
    ///
    /// Grouped search results sorted by relevance.
    pub async fn search(&self, query: &str) -> SearchResults {
        if query.is_empty() {
            return SearchResults::empty();
        }

        let start = std::time::Instant::now();
        let max_results = self.config.max_results_per_provider;

        let mut handles = Vec::with_capacity(self.providers.len());
        let query_string = query.to_string();
        for provider in &self.providers {
            let provider = Arc::clone(provider);
            let query = query_string.clone();
            handles.push(tokio::task::spawn_blocking(move || {
                provider.search(&query, max_results)
            }));
        }

        let mut all_results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(results) => all_results.extend(results),
                Err(err) => {
                    tracing::warn!("Search provider task failed: {}", err);
                },
            }
        }

        self.build_results(&query_string, all_results, start.elapsed())
    }

    /// Builds grouped search results from a flat list of results.
    fn build_results(
        &self,
        query: &str,
        mut results: Vec<SearchResult>,
        search_time: Duration,
    ) -> SearchResults {
        if results.is_empty() {
            return SearchResults {
                groups: Vec::new(),
                total_count: 0,
                query: query.to_string(),
                search_time,
            };
        }

        // Sort all results by score descending
        results.sort_by(|a, b| b.score.total_cmp(&a.score));

        // Limit total results
        results.truncate(self.config.max_total_results);

        let total_count = results.len();

        // Group by result type
        let mut groups_map: HashMap<ResultType, Vec<SearchResult>> = HashMap::new();

        for result in results {
            groups_map
                .entry(result.result_type)
                .or_default()
                .push(result);
        }

        // Convert to sorted groups
        let mut groups: Vec<ResultGroup> = groups_map
            .into_iter()
            .map(|(result_type, results)| ResultGroup {
                result_type,
                results,
            })
            .collect();

        // Sort groups by priority
        groups.sort_by_key(|g| g.result_type.priority());

        SearchResults {
            groups,
            total_count,
            query: query.to_string(),
            search_time,
        }
    }

    /// Returns the number of registered providers.
    #[must_use]
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Returns true if no providers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::search::{IconSource, SearchAction, SearchResultId};

    /// A mock provider for testing.
    struct MockProvider {
        name: &'static str,
        result_type: ResultType,
        results: Vec<SearchResult>,
    }

    impl MockProvider {
        fn new(name: &'static str, result_type: ResultType, results: Vec<SearchResult>) -> Self {
            Self {
                name,
                result_type,
                results,
            }
        }
    }

    impl SearchProvider for MockProvider {
        fn name(&self) -> &str {
            self.name
        }

        fn result_type(&self) -> ResultType {
            self.result_type
        }

        fn search(&self, _query: &str, max_results: usize) -> Vec<SearchResult> {
            self.results.iter().take(max_results).cloned().collect()
        }
    }

    fn create_test_result(
        id: &str,
        title: &str,
        score: f64,
        result_type: ResultType,
    ) -> SearchResult {
        SearchResult {
            id: SearchResultId::new(id),
            title: title.to_string(),
            subtitle: String::new(),
            icon: IconSource::SystemIcon {
                name: "test".to_string(),
            },
            result_type,
            score,
            match_indices: Vec::new(),
            requires_permissions: false,
            action: SearchAction::OpenFile {
                path: PathBuf::from("/test"),
            },
        }
    }

    #[test]
    fn test_empty_engine() {
        let engine = SearchEngine::new();
        assert_eq!(engine.provider_count(), 0);
        assert!(engine.is_empty());
    }

    #[test]
    fn test_add_provider() {
        let mut engine = SearchEngine::new();
        engine.add_provider(MockProvider::new(
            "test",
            ResultType::Application,
            Vec::new(),
        ));
        assert_eq!(engine.provider_count(), 1);
        assert!(!engine.is_empty());
    }

    #[test]
    fn test_search_empty_query() {
        let mut engine = SearchEngine::new();
        engine.add_provider(MockProvider::new(
            "test",
            ResultType::Application,
            vec![create_test_result(
                "1",
                "Safari",
                100.0,
                ResultType::Application,
            )],
        ));

        let results = engine.search_sync("");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_returns_results() {
        let mut engine = SearchEngine::new();
        engine.add_provider(MockProvider::new(
            "test",
            ResultType::Application,
            vec![create_test_result(
                "1",
                "Safari",
                100.0,
                ResultType::Application,
            )],
        ));

        let results = engine.search_sync("saf");
        assert_eq!(results.total_count, 1);
        assert_eq!(results.groups.len(), 1);
        assert_eq!(results.groups[0].result_type, ResultType::Application);
    }

    #[test]
    fn test_search_groups_by_type() {
        let mut engine = SearchEngine::new();

        engine.add_provider(MockProvider::new(
            "apps",
            ResultType::Application,
            vec![create_test_result(
                "1",
                "Safari",
                100.0,
                ResultType::Application,
            )],
        ));

        engine.add_provider(MockProvider::new(
            "commands",
            ResultType::SystemCommand,
            vec![create_test_result(
                "2",
                "Sleep",
                80.0,
                ResultType::SystemCommand,
            )],
        ));

        let results = engine.search_sync("s");
        assert_eq!(results.total_count, 2);
        assert_eq!(results.groups.len(), 2);

        // Check groups are sorted by priority (Apps before Commands)
        assert_eq!(results.groups[0].result_type, ResultType::Application);
        assert_eq!(results.groups[1].result_type, ResultType::SystemCommand);
    }

    #[test]
    fn test_search_sorts_by_score() {
        let mut engine = SearchEngine::new();

        engine.add_provider(MockProvider::new(
            "apps",
            ResultType::Application,
            vec![
                create_test_result("1", "Low Score", 50.0, ResultType::Application),
                create_test_result("2", "High Score", 100.0, ResultType::Application),
            ],
        ));

        let results = engine.search_sync("test");
        assert_eq!(results.total_count, 2);
        assert_eq!(results.groups[0].results[0].title, "High Score");
        assert_eq!(results.groups[0].results[1].title, "Low Score");
    }

    #[test]
    fn test_search_respects_max_total_results() {
        let mut engine = SearchEngine::with_config(SearchConfig {
            max_total_results: 2,
            ..Default::default()
        });

        engine.add_provider(MockProvider::new(
            "apps",
            ResultType::Application,
            vec![
                create_test_result("1", "App 1", 100.0, ResultType::Application),
                create_test_result("2", "App 2", 90.0, ResultType::Application),
                create_test_result("3", "App 3", 80.0, ResultType::Application),
            ],
        ));

        let results = engine.search_sync("test");
        assert_eq!(results.total_count, 2);
    }

    #[test]
    fn test_search_time_is_recorded() {
        let engine = SearchEngine::new();
        let results = engine.search_sync("test");
        // Search time should be recorded (allow fast empty search)
        assert!(results.search_time < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_async_search() {
        let mut engine = SearchEngine::new();
        engine.add_provider(MockProvider::new(
            "test",
            ResultType::Application,
            vec![create_test_result(
                "1",
                "Test App",
                100.0,
                ResultType::Application,
            )],
        ));

        let results = engine.search("test").await;
        assert_eq!(results.total_count, 1);
    }
}
