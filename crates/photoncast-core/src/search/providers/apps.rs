//! Application search provider.
//!
//! This module provides search functionality for indexed applications.

use std::sync::Arc;

use parking_lot::RwLock;

use crate::indexer::IndexedApp;
use crate::search::fuzzy::FuzzyMatcher;
use crate::search::providers::SearchProvider;
use crate::search::{IconSource, ResultType, SearchAction, SearchResult, SearchResultId};

/// Provides search results for installed applications.
///
/// This provider holds a reference to the indexed apps and performs
/// fuzzy matching against app names.
pub struct AppProvider {
    /// The indexed applications.
    apps: Arc<RwLock<Vec<IndexedApp>>>,
}

impl std::fmt::Debug for AppProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppProvider")
            .field("app_count", &self.apps.read().len())
            .finish()
    }
}

impl Default for AppProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AppProvider {
    /// Creates a new app provider with an empty app index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            apps: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Creates a new app provider with a shared app index.
    #[must_use]
    pub fn with_apps(apps: Arc<RwLock<Vec<IndexedApp>>>) -> Self {
        Self { apps }
    }

    /// Updates the app index with new apps.
    pub fn set_apps(&self, apps: Vec<IndexedApp>) {
        *self.apps.write() = apps;
    }

    /// Adds apps to the index.
    pub fn add_apps(&self, apps: impl IntoIterator<Item = IndexedApp>) {
        self.apps.write().extend(apps);
    }

    /// Returns the number of indexed apps.
    #[must_use]
    pub fn app_count(&self) -> usize {
        self.apps.read().len()
    }

    /// Removes an app from the index by bundle ID.
    pub fn remove_app(&self, bundle_id: &str) {
        self.apps
            .write()
            .retain(|app| app.bundle_id.as_str() != bundle_id);
    }
}

impl SearchProvider for AppProvider {
    fn name(&self) -> &'static str {
        "Applications"
    }

    fn result_type(&self) -> ResultType {
        ResultType::Application
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let apps = self.apps.read();
        let mut matcher = FuzzyMatcher::default();

        // Score all apps against the query
        let mut scored_results: Vec<(usize, u32, Vec<usize>)> = apps
            .iter()
            .enumerate()
            .filter_map(|(idx, app)| {
                matcher
                    .score(query, &app.name)
                    .map(|(score, indices)| (idx, score, indices))
            })
            .collect();

        // Sort by score descending
        scored_results.sort_by(|a, b| b.1.cmp(&a.1));

        // Take top results and convert to SearchResult
        scored_results
            .into_iter()
            .take(max_results)
            .map(|(idx, score, match_indices)| {
                let app = &apps[idx];
                SearchResult {
                    id: SearchResultId::new(format!("app:{}", app.bundle_id)),
                    title: app.name.clone(),
                    subtitle: app.path.display().to_string(),
                    icon: IconSource::AppIcon {
                        bundle_id: app.bundle_id.as_str().to_string(),
                        icon_path: app.icon_path.clone(),
                    },
                    result_type: ResultType::Application,
                    score: f64::from(score),
                    match_indices,
                    requires_permissions: false,
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

    fn create_test_app(name: &str, bundle_id: &str) -> IndexedApp {
        IndexedApp {
            name: name.to_string(),
            bundle_id: AppBundleId::new(bundle_id),
            path: PathBuf::from(format!("/Applications/{name}.app")),
            icon_path: None,
            category: None,
            keywords: Vec::new(),
            last_modified: Utc::now(),
        }
    }

    #[test]
    fn test_empty_provider() {
        let provider = AppProvider::new();
        assert_eq!(provider.app_count(), 0);
        assert_eq!(provider.name(), "Applications");
        assert_eq!(provider.result_type(), ResultType::Application);
    }

    #[test]
    fn test_search_empty_query() {
        let provider = AppProvider::new();
        provider.add_apps(vec![create_test_app("Safari", "com.apple.Safari")]);

        let results = provider.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_finds_matching_app() {
        let provider = AppProvider::new();
        provider.add_apps(vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("System Preferences", "com.apple.systempreferences"),
            create_test_app("Xcode", "com.apple.dt.Xcode"),
        ]);

        let results = provider.search("saf", 10);
        assert!(!results.is_empty());
        assert!(results[0].title.to_lowercase().contains("saf"));
    }

    #[test]
    fn test_search_respects_max_results() {
        let provider = AppProvider::new();
        provider.add_apps(vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("System Preferences", "com.apple.systempreferences"),
            create_test_app("Settings", "com.apple.settings"),
        ]);

        let results = provider.search("s", 2);
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_search_returns_correct_action() {
        let provider = AppProvider::new();
        provider.add_apps(vec![create_test_app("Safari", "com.apple.Safari")]);

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
        let provider = AppProvider::new();
        provider.add_apps(vec![create_test_app("Safari", "com.apple.Safari")]);

        let results = provider.search("saf", 10);
        assert!(!results.is_empty());
        assert!(!results[0].match_indices.is_empty());
    }

    #[test]
    fn test_remove_app() {
        let provider = AppProvider::new();
        provider.add_apps(vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Xcode", "com.apple.dt.Xcode"),
        ]);

        assert_eq!(provider.app_count(), 2);

        provider.remove_app("com.apple.Safari");
        assert_eq!(provider.app_count(), 1);

        let results = provider.search("Safari", 10);
        assert!(results.is_empty());
    }
}
