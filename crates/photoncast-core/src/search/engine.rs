//! Search orchestration.
//!
//! This module contains the main search engine that coordinates queries
//! across multiple providers and merges results.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tracing::{debug_span, trace, warn};

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
    /// Debounce for launcher normal-mode query dispatch.
    pub debounce_ms: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results_per_provider: DEFAULT_MAX_RESULTS_PER_PROVIDER,
            max_total_results: DEFAULT_MAX_TOTAL_RESULTS,
            timeout: Duration::from_millis(100),
            debounce_ms: 50,
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

        let _span = debug_span!(
            "search.engine.search",
            component = "search",
            operation = "search_sync",
            query_len = query.len(),
            provider_count = self.providers.len()
        )
        .entered();

        let start = std::time::Instant::now();

        // Collect results from all providers
        let mut all_results: Vec<SearchResult> = Vec::new();

        for provider in &self.providers {
            let provider_name = provider.name();
            let provider_span = debug_span!(
                "search.provider.search",
                component = "search",
                operation = "provider_sync",
                provider_id = provider_name
            )
            .entered();
            let provider_start = std::time::Instant::now();
            let provider_results = provider.search(query, self.config.max_results_per_provider);
            let elapsed_ms = provider_start.elapsed().as_secs_f64() * 1000.0;
            trace!(
                component = "search",
                operation = "provider_sync",
                provider_id = provider_name,
                elapsed_ms,
                result_count = provider_results.len(),
                "provider search completed"
            );
            drop(provider_span);
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
        self.search_with_cancellation(query, None).await
    }

    /// Performs an async search with optional cancellation.
    ///
    /// Cancellation is best-effort: provider tasks are not forcibly terminated,
    /// but their results are dropped if cancellation is observed.
    pub async fn search_with_cancellation(
        &self,
        query: &str,
        cancellation: Option<Arc<AtomicBool>>,
    ) -> SearchResults {
        if query.is_empty() {
            return SearchResults::empty();
        }

        trace!(
            component = "search",
            operation = "search",
            query_len = query.len(),
            provider_count = self.providers.len(),
            "search engine async start"
        );

        let start = std::time::Instant::now();
        let max_results = self.config.max_results_per_provider;

        let mut handles = Vec::with_capacity(self.providers.len());
        let query_string = query.to_string();
        let timeout = self.config.timeout;
        for provider in &self.providers {
            let provider_name = provider.name().to_string();
            let provider = Arc::clone(provider);
            let query = query_string.clone();
            let handle = tokio::task::spawn_blocking(move || provider.search(&query, max_results));
            handles.push((provider_name, handle));
        }

        let mut all_results = Vec::new();
        let provider_outcomes = join_all(handles.into_iter().map(
            |(provider_name, handle)| async move {
                let provider_start = std::time::Instant::now();
                let outcome = tokio::time::timeout(timeout, handle).await;
                (provider_name, provider_start.elapsed(), outcome)
            },
        ))
        .await;

        if cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            trace!(
                component = "search",
                operation = "search",
                cancelled = true,
                "search cancelled before collecting all provider results"
            );
            return SearchResults::empty();
        }

        for (provider_name, elapsed, outcome) in provider_outcomes {
            let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
            match outcome {
                Ok(Ok(results)) => {
                    trace!(
                        component = "search",
                        operation = "provider_async",
                        provider_id = provider_name,
                        elapsed_ms,
                        result_count = results.len(),
                        "async provider search completed"
                    );
                    all_results.extend(results);
                },
                Ok(Err(err)) => {
                    warn!(
                        component = "search",
                        provider_id = provider_name,
                        elapsed_ms,
                        "search provider task failed: {}",
                        err
                    );
                },
                Err(_) => {
                    warn!(
                        component = "search",
                        provider_id = provider_name,
                        timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
                        elapsed_ms,
                        "search provider timed out, returning empty results"
                    );
                },
            }
        }

        let result_count = all_results.len();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        trace!(
            component = "search",
            operation = "search",
            elapsed_ms,
            result_count,
            cancelled = false,
            "search engine completed"
        );

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
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;

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

    struct SlowMockProvider {
        name: &'static str,
        result_type: ResultType,
        delay: Duration,
        results: Vec<SearchResult>,
    }

    impl SearchProvider for SlowMockProvider {
        fn name(&self) -> &str {
            self.name
        }

        fn result_type(&self) -> ResultType {
            self.result_type
        }

        fn search(&self, _query: &str, max_results: usize) -> Vec<SearchResult> {
            std::thread::sleep(self.delay);
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

    #[tokio::test]
    async fn test_async_search_collects_provider_results_in_parallel() {
        let mut engine = SearchEngine::new();
        let delay = Duration::from_millis(80);

        engine.add_provider(SlowMockProvider {
            name: "slow-1",
            result_type: ResultType::Application,
            delay,
            results: vec![create_test_result(
                "1",
                "Slow App 1",
                100.0,
                ResultType::Application,
            )],
        });

        engine.add_provider(SlowMockProvider {
            name: "slow-2",
            result_type: ResultType::SystemCommand,
            delay,
            results: vec![create_test_result(
                "2",
                "Slow Command 2",
                90.0,
                ResultType::SystemCommand,
            )],
        });

        let started = std::time::Instant::now();
        let results = engine.search("slow").await;
        let elapsed = started.elapsed();

        assert_eq!(results.total_count, 2);
        assert!(
            elapsed < Duration::from_millis(260),
            "search should complete near single-provider latency when joins are batched; elapsed={elapsed:?}"
        );
    }

    #[tokio::test]
    async fn test_search_with_cancellation_returns_empty_when_cancelled() {
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

        let cancellation = Arc::new(AtomicBool::new(true));
        let results = engine
            .search_with_cancellation("test", Some(Arc::clone(&cancellation)))
            .await;

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_with_empty_query_returns_empty_results() {
        let mut engine = SearchEngine::new();
        engine.add_provider(MockProvider::new(
            "test",
            ResultType::Application,
            vec![create_test_result(
                "1",
                "Ignored",
                1.0,
                ResultType::Application,
            )],
        ));

        let results = engine.search("").await;
        assert!(results.is_empty());
        assert_eq!(results.total_count, 0);
    }

    #[tokio::test]
    async fn test_search_with_very_long_query_does_not_panic() {
        let mut engine = SearchEngine::new();
        engine.add_provider(MockProvider::new(
            "test",
            ResultType::Application,
            vec![create_test_result(
                "1",
                "Long Query Result",
                42.0,
                ResultType::Application,
            )],
        ));

        let long_query = "a".repeat(100_000);
        let results = engine.search(&long_query).await;

        assert_eq!(results.total_count, 1);
    }

    #[test]
    fn test_provider_count_after_multiple_registrations() {
        let mut engine = SearchEngine::new();

        engine.add_provider(MockProvider::new(
            "apps",
            ResultType::Application,
            Vec::new(),
        ));
        engine.add_provider(MockProvider::new(
            "commands",
            ResultType::SystemCommand,
            Vec::new(),
        ));
        engine.add_provider(MockProvider::new("files", ResultType::File, Vec::new()));

        assert_eq!(engine.provider_count(), 3);
    }

    #[test]
    fn test_search_config_custom_timeout_is_applied() {
        let timeout = Duration::from_millis(321);
        let engine = SearchEngine::with_config(SearchConfig {
            timeout,
            ..Default::default()
        });

        assert_eq!(engine.config().timeout, timeout);
    }

    #[tokio::test]
    async fn test_timeout_enforcement_drops_slow_provider_results() {
        // Configure a very short timeout so the slow provider is guaranteed to exceed it.
        let timeout = Duration::from_millis(50);
        let mut engine = SearchEngine::with_config(SearchConfig {
            timeout,
            ..Default::default()
        });

        // A fast provider that returns instantly.
        engine.add_provider(MockProvider::new(
            "fast",
            ResultType::Application,
            vec![create_test_result(
                "fast-1",
                "Fast App",
                100.0,
                ResultType::Application,
            )],
        ));

        // A slow provider that sleeps well beyond the timeout.
        engine.add_provider(SlowMockProvider {
            name: "slow",
            result_type: ResultType::SystemCommand,
            delay: Duration::from_secs(5),
            results: vec![create_test_result(
                "slow-1",
                "Slow Command",
                90.0,
                ResultType::SystemCommand,
            )],
        });

        let results = engine.search("test").await;

        // Only the fast provider's results should appear; the slow provider
        // should have been timed out and returned empty results.
        assert_eq!(
            results.total_count, 1,
            "expected only fast provider results; slow provider should have timed out"
        );
        assert_eq!(results.groups.len(), 1);
        assert_eq!(results.groups[0].result_type, ResultType::Application);
        assert_eq!(results.groups[0].results[0].title, "Fast App");

        // The search itself should complete quickly (near timeout, not 5s).
        assert!(
            results.search_time < Duration::from_secs(1),
            "search should complete near timeout, not wait for slow provider; elapsed={:?}",
            results.search_time,
        );
    }
}
