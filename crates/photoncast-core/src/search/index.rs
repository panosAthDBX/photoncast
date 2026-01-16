//! Pre-computed search index for optimized search performance.
//!
//! This module provides a search index that pre-computes and caches data
//! for faster search operations:
//! - Pre-lowercased app names for case-insensitive matching
//! - Pre-computed frecency scores
//! - Pre-sorted by frecency for early termination

use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use crate::indexer::IndexedApp;
use crate::search::ranking::FrecencyScore;

/// Converts a DateTime<Utc> to Option<SystemTime>.
fn datetime_to_system_time(dt: DateTime<Utc>) -> Option<SystemTime> {
    let timestamp = dt.timestamp();
    if timestamp >= 0 {
        UNIX_EPOCH.checked_add(std::time::Duration::from_secs(timestamp as u64))
    } else {
        None
    }
}

/// Pre-computed entry in the search index.
#[derive(Debug, Clone)]
pub struct IndexedAppEntry {
    /// The original indexed app.
    pub app: IndexedApp,
    /// Pre-lowercased name for case-insensitive matching.
    pub name_lower: String,
    /// Pre-computed frecency score.
    pub frecency: f64,
}

impl IndexedAppEntry {
    /// Creates a new indexed app entry with pre-computed data.
    #[must_use]
    pub fn new(app: IndexedApp, frecency: f64) -> Self {
        let name_lower = app.name.to_lowercase();
        Self {
            app,
            name_lower,
            frecency,
        }
    }

    /// Creates a new entry with zero frecency (for apps without usage data).
    #[must_use]
    pub fn without_frecency(app: IndexedApp) -> Self {
        Self::new(app, 0.0)
    }
}

/// Usage data provider trait for frecency calculation.
pub trait UsageDataProvider {
    /// Returns the usage data for an app by bundle ID.
    fn get_usage(&self, bundle_id: &str) -> Option<UsageRecord>;
}

/// A usage record for frecency calculation.
#[derive(Debug, Clone)]
pub struct UsageRecord {
    /// Number of times the app was launched.
    pub launch_count: u32,
    /// When the app was last launched.
    pub last_launched: DateTime<Utc>,
}

/// No-op usage data provider (returns no usage data).
#[derive(Debug, Default)]
pub struct NoUsageData;

impl UsageDataProvider for NoUsageData {
    fn get_usage(&self, _bundle_id: &str) -> Option<UsageRecord> {
        None
    }
}

/// Pre-computed search index for optimized searching.
///
/// The search index stores apps with pre-computed data:
/// - Pre-lowercased names for case-insensitive matching
/// - Pre-computed frecency scores
/// - Pre-sorted by frecency for early termination
#[derive(Debug, Clone)]
pub struct SearchIndex {
    /// Indexed app entries, sorted by frecency descending.
    entries: Vec<IndexedAppEntry>,
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchIndex {
    /// Creates an empty search index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Builds a search index from apps without usage data.
    #[must_use]
    pub fn from_apps(apps: &[IndexedApp]) -> Self {
        Self::build(apps, &NoUsageData)
    }

    /// Builds a search index from apps with usage data for frecency calculation.
    ///
    /// The index pre-computes:
    /// - Lowercased app names
    /// - Frecency scores from usage data
    /// - Sorted order by frecency (descending)
    #[must_use]
    pub fn build<U: UsageDataProvider>(apps: &[IndexedApp], usage: &U) -> Self {
        let mut entries: Vec<IndexedAppEntry> = apps
            .iter()
            .map(|app| {
                let frecency = usage
                    .get_usage(app.bundle_id.as_str())
                    .map(|record| {
                        let last_used = datetime_to_system_time(record.last_launched);
                        FrecencyScore::calculate(record.launch_count, last_used).score()
                    })
                    .unwrap_or(0.0);
                IndexedAppEntry::new(app.clone(), frecency)
            })
            .collect();

        // Sort by frecency descending for early termination
        entries.sort_by(|a, b| {
            b.frecency
                .partial_cmp(&a.frecency)
                .unwrap_or(Ordering::Equal)
        });

        Self { entries }
    }

    /// Returns an iterator over all indexed entries.
    pub fn iter(&self) -> impl Iterator<Item = &IndexedAppEntry> {
        self.entries.iter()
    }

    /// Returns the number of indexed entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns a reference to the entries for direct access.
    #[must_use]
    pub fn entries(&self) -> &[IndexedAppEntry] {
        &self.entries
    }

    /// Adds an app to the index.
    ///
    /// Note: This does not maintain sort order. Call `rebuild_sort()` after
    /// batch additions to restore the frecency sort order.
    pub fn add_app(&mut self, app: IndexedApp, frecency: f64) {
        self.entries.push(IndexedAppEntry::new(app, frecency));
    }

    /// Removes an app from the index by bundle ID.
    pub fn remove_app(&mut self, bundle_id: &str) {
        self.entries
            .retain(|e| e.app.bundle_id.as_str() != bundle_id);
    }

    /// Updates the frecency score for an app and re-sorts the index.
    pub fn update_frecency(&mut self, bundle_id: &str, frecency: f64) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.app.bundle_id.as_str() == bundle_id)
        {
            entry.frecency = frecency;
            self.rebuild_sort();
        }
    }

    /// Re-sorts the index by frecency descending.
    pub fn rebuild_sort(&mut self) {
        self.entries.sort_by(|a, b| {
            b.frecency
                .partial_cmp(&a.frecency)
                .unwrap_or(Ordering::Equal)
        });
    }

    /// Clears all entries from the index.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Configuration for early termination.
#[derive(Debug, Clone, Copy)]
pub struct EarlyTerminationConfig {
    /// Multiplier for max_results to determine termination threshold.
    /// When we have found `max_results * multiplier` high-quality matches,
    /// we can stop searching.
    pub threshold_multiplier: f64,

    /// Minimum score for a result to be considered "high quality".
    pub min_quality_score: u32,
}

impl Default for EarlyTerminationConfig {
    fn default() -> Self {
        Self {
            threshold_multiplier: 2.0,
            min_quality_score: 50, // Reasonable nucleo score threshold
        }
    }
}

impl EarlyTerminationConfig {
    /// Calculates the termination threshold for a given max_results.
    #[must_use]
    pub fn threshold(&self, max_results: usize) -> usize {
        ((max_results as f64) * self.threshold_multiplier).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Duration;

    use super::*;
    use crate::indexer::AppBundleId;

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
    fn test_empty_index() {
        let index = SearchIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_from_apps_without_usage() {
        let apps = vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Xcode", "com.apple.dt.Xcode"),
        ];

        let index = SearchIndex::from_apps(&apps);
        assert_eq!(index.len(), 2);

        // All entries should have zero frecency
        for entry in index.iter() {
            assert_eq!(entry.frecency, 0.0);
        }
    }

    #[test]
    fn test_name_lower_precomputed() {
        let apps = vec![create_test_app("Safari", "com.apple.Safari")];
        let index = SearchIndex::from_apps(&apps);

        let entry = &index.entries()[0];
        assert_eq!(entry.name_lower, "safari");
    }

    #[test]
    fn test_build_with_usage_data() {
        let apps = vec![
            create_test_app("Safari", "com.apple.Safari"),
            create_test_app("Xcode", "com.apple.dt.Xcode"),
        ];

        let usage = TestUsageData {
            records: vec![(
                "com.apple.Safari".to_string(),
                UsageRecord {
                    launch_count: 100,
                    last_launched: Utc::now(),
                },
            )],
        };

        let index = SearchIndex::build(&apps, &usage);
        assert_eq!(index.len(), 2);

        // Safari should have higher frecency and be first
        let first = &index.entries()[0];
        assert_eq!(first.app.name, "Safari");
        assert!(first.frecency > 0.0);
    }

    #[test]
    fn test_sorted_by_frecency() {
        let apps = vec![
            create_test_app("App A", "com.test.a"),
            create_test_app("App B", "com.test.b"),
            create_test_app("App C", "com.test.c"),
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
                (
                    "com.test.c".to_string(),
                    UsageRecord {
                        launch_count: 50,
                        last_launched: Utc::now(),
                    },
                ),
            ],
        };

        let index = SearchIndex::build(&apps, &usage);

        // Should be sorted: B (100) > C (50) > A (10)
        assert_eq!(index.entries()[0].app.name, "App B");
        assert_eq!(index.entries()[1].app.name, "App C");
        assert_eq!(index.entries()[2].app.name, "App A");
    }

    #[test]
    fn test_frecency_decay() {
        let apps = vec![
            create_test_app("Recent", "com.test.recent"),
            create_test_app("Old", "com.test.old"),
        ];

        let now = Utc::now();
        let usage = TestUsageData {
            records: vec![
                (
                    "com.test.recent".to_string(),
                    UsageRecord {
                        launch_count: 50,
                        last_launched: now,
                    },
                ),
                (
                    "com.test.old".to_string(),
                    UsageRecord {
                        launch_count: 50,
                        last_launched: now - Duration::days(30),
                    },
                ),
            ],
        };

        let index = SearchIndex::build(&apps, &usage);

        // Recent app should have higher frecency despite same launch count
        let recent = index
            .entries()
            .iter()
            .find(|e| e.app.name == "Recent")
            .unwrap();
        let old = index
            .entries()
            .iter()
            .find(|e| e.app.name == "Old")
            .unwrap();
        assert!(recent.frecency > old.frecency);
    }

    #[test]
    fn test_add_and_remove_app() {
        let mut index = SearchIndex::new();

        index.add_app(create_test_app("Safari", "com.apple.Safari"), 10.0);
        assert_eq!(index.len(), 1);

        index.add_app(create_test_app("Xcode", "com.apple.dt.Xcode"), 5.0);
        assert_eq!(index.len(), 2);

        index.remove_app("com.apple.Safari");
        assert_eq!(index.len(), 1);
        assert_eq!(index.entries()[0].app.name, "Xcode");
    }

    #[test]
    fn test_update_frecency() {
        let apps = vec![
            create_test_app("App A", "com.test.a"),
            create_test_app("App B", "com.test.b"),
        ];

        let usage = TestUsageData {
            records: vec![
                (
                    "com.test.a".to_string(),
                    UsageRecord {
                        launch_count: 100,
                        last_launched: Utc::now(),
                    },
                ),
                (
                    "com.test.b".to_string(),
                    UsageRecord {
                        launch_count: 10,
                        last_launched: Utc::now(),
                    },
                ),
            ],
        };

        let mut index = SearchIndex::build(&apps, &usage);

        // A should be first initially
        assert_eq!(index.entries()[0].app.name, "App A");

        // Update B's frecency to be higher
        index.update_frecency("com.test.b", 1000.0);

        // Now B should be first
        assert_eq!(index.entries()[0].app.name, "App B");
    }

    #[test]
    fn test_clear() {
        let apps = vec![create_test_app("Safari", "com.apple.Safari")];
        let mut index = SearchIndex::from_apps(&apps);

        assert!(!index.is_empty());
        index.clear();
        assert!(index.is_empty());
    }

    #[test]
    fn test_early_termination_config() {
        let config = EarlyTerminationConfig::default();
        assert_eq!(config.threshold_multiplier, 2.0);
        assert_eq!(config.threshold(10), 20);
        assert_eq!(config.threshold(5), 10);
    }

    #[test]
    fn test_early_termination_threshold_rounding() {
        let config = EarlyTerminationConfig {
            threshold_multiplier: 1.5,
            min_quality_score: 50,
        };
        assert_eq!(config.threshold(3), 5); // 3 * 1.5 = 4.5 -> ceil = 5
    }
}
