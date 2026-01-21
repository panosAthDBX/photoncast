//! Optimized application search provider with pre-computed index.
//!
//! This module provides an optimized search provider that uses a pre-computed
//! search index for faster search operations:
//! - Pre-lowercased app names for case-insensitive matching
//! - Pre-sorted by frecency for early termination
//! - Early termination when enough high-quality matches are found

use std::sync::Arc;

use parking_lot::RwLock;

use crate::indexer::IndexedApp;
use crate::search::fuzzy::FuzzyMatcher;
use crate::search::index::{EarlyTerminationConfig, SearchIndex, UsageDataProvider};
use crate::search::providers::SearchProvider;
use crate::search::{IconSource, ResultType, SearchAction, SearchResult, SearchResultId};

/// Optimized application search provider with pre-computed index.
///
/// This provider maintains a pre-computed search index that:
/// - Pre-lowercases app names for faster case-insensitive matching
/// - Pre-sorts by frecency for early termination
/// - Implements early termination when enough quality matches are found
pub struct OptimizedAppProvider {
    /// The pre-computed search index.
    index: Arc<RwLock<SearchIndex>>,
    /// Early termination configuration.
    termination_config: EarlyTerminationConfig,
}

impl std::fmt::Debug for OptimizedAppProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizedAppProvider")
            .field("app_count", &self.index.read().len())
            .field("termination_config", &self.termination_config)
            .finish()
    }
}

impl Default for OptimizedAppProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimizedAppProvider {
    /// Creates a new optimized app provider with an empty index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            index: Arc::new(RwLock::new(SearchIndex::new())),
            termination_config: EarlyTerminationConfig::default(),
        }
    }

    /// Creates a new optimized app provider with a shared index.
    #[must_use]
    pub fn with_index(index: Arc<RwLock<SearchIndex>>) -> Self {
        Self {
            index,
            termination_config: EarlyTerminationConfig::default(),
        }
    }

    /// Creates a new optimized app provider with custom termination config.
    #[must_use]
    pub fn with_config(termination_config: EarlyTerminationConfig) -> Self {
        Self {
            index: Arc::new(RwLock::new(SearchIndex::new())),
            termination_config,
        }
    }

    /// Sets the early termination configuration.
    pub fn set_termination_config(&mut self, config: EarlyTerminationConfig) {
        self.termination_config = config;
    }

    /// Builds the search index from apps without usage data.
    pub fn build_index(&self, apps: &[IndexedApp]) {
        *self.index.write() = SearchIndex::from_apps(apps);
    }

    /// Builds the search index from apps with usage data.
    pub fn build_index_with_usage<U: UsageDataProvider>(&self, apps: &[IndexedApp], usage: &U) {
        *self.index.write() = SearchIndex::build(apps, usage);
    }

    /// Updates the frecency score for an app.
    pub fn update_frecency(&self, bundle_id: &str, frecency: f64) {
        self.index.write().update_frecency(bundle_id, frecency);
    }

    /// Returns the number of indexed apps.
    #[must_use]
    pub fn app_count(&self) -> usize {
        self.index.read().len()
    }

    /// Removes an app from the index by bundle ID.
    pub fn remove_app(&self, bundle_id: &str) {
        self.index.write().remove_app(bundle_id);
    }

    /// Adds an app to the index with the given frecency.
    pub fn add_app(&self, app: IndexedApp, frecency: f64) {
        self.index.write().add_app(app, frecency);
    }

    /// Rebuilds the sort order (after batch additions).
    pub fn rebuild_sort(&self) {
        self.index.write().rebuild_sort();
    }
}

impl SearchProvider for OptimizedAppProvider {
    fn name(&self) -> &str {
        "Applications (Optimized)"
    }

    fn result_type(&self) -> ResultType {
        ResultType::Application
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let index = self.index.read();
        let mut matcher = FuzzyMatcher::default();

        // Calculate early termination threshold
        let termination_threshold = self.termination_config.threshold(max_results);
        let min_quality = self.termination_config.min_quality_score;

        let mut scored_results: Vec<(usize, u32, Vec<usize>)> = Vec::new();
        let mut quality_count = 0;

        // Iterate through entries (pre-sorted by frecency)
        for (idx, entry) in index.entries().iter().enumerate() {
            // Use pre-lowercased name for matching
            if let Some((score, indices)) = matcher.score(&query_lower, &entry.name_lower) {
                scored_results.push((idx, score, indices));

                // Track high-quality matches
                if score >= min_quality {
                    quality_count += 1;

                    // Early termination: stop when we have enough quality matches
                    if quality_count >= termination_threshold {
                        break;
                    }
                }
            }
        }

        // Sort by score descending (nucleo score + frecency boost)
        scored_results.sort_by(|a, b| {
            let frecency_a = index.entries()[a.0].frecency;
            let frecency_b = index.entries()[b.0].frecency;

            // Combine match score with frecency
            let combined_a = frecency_a.mul_add(10.0, f64::from(a.1));
            let combined_b = frecency_b.mul_add(10.0, f64::from(b.1));

            combined_b
                .partial_cmp(&combined_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top results and convert to SearchResult
        scored_results
            .into_iter()
            .take(max_results)
            .map(|(idx, score, match_indices)| {
                let entry = &index.entries()[idx];
                let app = &entry.app;
                SearchResult {
                    id: SearchResultId::new(format!("app:{}", app.bundle_id)),
                    title: app.name.clone(),
                    subtitle: app.path.display().to_string(),
                    icon: IconSource::AppIcon {
                        bundle_id: app.bundle_id.as_str().to_string(),
                        icon_path: app.icon_path.clone(),
                    },
                    result_type: ResultType::Application,
                    score: entry.frecency.mul_add(10.0, f64::from(score)),
                    match_indices,
                    action: SearchAction::LaunchApp {
                        bundle_id: app.bundle_id.as_str().to_string(),
                        path: app.path.clone(),
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use super::*;
    use crate::indexer::AppBundleId;
    use crate::search::index::UsageRecord;

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

    struct TestUsageData {
        records: Vec<(String, UsageRecord)>,
    }

    impl UsageDataProvider for TestUsageData {
        fn get_usage(&self, bundle_id: &str) -> Option<UsageRecord> {
            self.records
                .iter()
                .find(|(id, _)| id == bundle_id)
                .map(|(_, record)| record.clone())
        }
    }

    #[test]
    fn test_empty_provider() {
        let provider = OptimizedAppProvider::new();
        assert_eq!(provider.app_count(), 0);
        assert_eq!(provider.name(), "Applications (Optimized)");
        assert_eq!(provider.result_type(), ResultType::Application);
    }

    #[test]
    fn test_build_index() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Xcode", "com.apple.dt.Xcode"),
        ]);

        assert_eq!(provider.app_count(), 2);
    }

    #[test]
    fn test_search_empty_query() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[create_test_app("Safari", "com.apple.Safari")]);

        let results = provider.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_finds_matching_app() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("System Preferences", "com.apple.systempreferences"),
            create_test_app("Xcode", "com.apple.dt.Xcode"),
        ]);

        let results = provider.search("saf", 10);
        assert!(!results.is_empty());
        assert!(results[0].title.to_lowercase().contains("saf"));
    }

    #[test]
    fn test_search_case_insensitive() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[create_test_app("Safari", "com.apple.Safari")]);

        // Test various case combinations
        let results_lower = provider.search("safari", 10);
        let results_upper = provider.search("SAFARI", 10);
        let results_mixed = provider.search("SaFaRi", 10);

        assert!(!results_lower.is_empty());
        assert!(!results_upper.is_empty());
        assert!(!results_mixed.is_empty());
    }

    #[test]
    fn test_search_respects_max_results() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("System Preferences", "com.apple.systempreferences"),
            create_test_app("Settings", "com.apple.settings"),
            create_test_app("Slack", "com.tinyspeck.slackmacgap"),
        ]);

        let results = provider.search("s", 2);
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_frecency_affects_ranking() {
        let provider = OptimizedAppProvider::new();

        let apps = [
            create_test_app("App A", "com.test.a"),
            create_test_app("App B", "com.test.b"),
        ];

        let usage = TestUsageData {
            records: vec![
                (
                    "com.test.a".to_string(),
                    UsageRecord {
                        launch_count: 10,
                        last_launched: Utc::now(),
                    },
                ),
                (
                    "com.test.b".to_string(),
                    UsageRecord {
                        launch_count: 100,
                        last_launched: Utc::now(),
                    },
                ),
            ],
        };

        provider.build_index_with_usage(&apps, &usage);

        // Both apps should match "app", but B should rank higher due to frecency
        let results = provider.search("app", 10);
        assert_eq!(results.len(), 2);
        assert!(results[0].title.contains('B'));
    }

    #[test]
    fn test_early_termination() {
        // Create a large number of apps
        let apps: Vec<IndexedApp> = (0..200)
            .map(|i| create_test_app(&format!("App{}", i), &format!("com.test.app{}", i)))
            .collect();

        let config = EarlyTerminationConfig {
            threshold_multiplier: 2.0,
            min_quality_score: 1, // Very low threshold to ensure matches count
        };

        let provider = OptimizedAppProvider::with_config(config);
        provider.build_index(&apps);

        // Search for "app" which should match all 200 apps
        // With early termination at max_results * 2 = 10 * 2 = 20, we should stop early
        let results = provider.search("app", 10);
        assert!(results.len() <= 10);
    }

    #[test]
    fn test_update_frecency() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[
            create_test_app("App A", "com.test.a"),
            create_test_app("App B", "com.test.b"),
        ]);

        // Initially, both have zero frecency
        // Update B's frecency
        provider.update_frecency("com.test.b", 1000.0);

        // Now B should be first
        let results = provider.search("app", 10);
        assert!(results[0].title.contains('B'));
    }

    #[test]
    fn test_add_and_remove_app() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[create_test_app("Safari", "com.apple.Safari")]);

        assert_eq!(provider.app_count(), 1);

        provider.add_app(create_test_app("Xcode", "com.apple.dt.Xcode"), 10.0);
        provider.rebuild_sort();
        assert_eq!(provider.app_count(), 2);

        provider.remove_app("com.apple.Safari");
        assert_eq!(provider.app_count(), 1);
    }

    #[test]
    fn test_search_returns_correct_action() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[create_test_app("Safari", "com.apple.Safari")]);

        let results = provider.search("Safari", 10);
        assert!(!results.is_empty());

        if let SearchAction::LaunchApp { bundle_id, path } = &results[0].action {
            assert_eq!(bundle_id, "com.apple.Safari");
            assert!(path.to_string_lossy().contains("Safari"));
        } else {
            panic!("Expected LaunchApp action");
        }
    }

    #[test]
    fn test_search_result_has_match_indices() {
        let provider = OptimizedAppProvider::new();
        provider.build_index(&[create_test_app("Safari", "com.apple.Safari")]);

        let results = provider.search("saf", 10);
        assert!(!results.is_empty());
        assert!(!results[0].match_indices.is_empty());
    }
}
