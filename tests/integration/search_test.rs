//! Integration tests for the search engine.
//!
//! These tests verify the complete search workflow including:
//! - Fuzzy matching accuracy
//! - Provider integration
//! - Result merging and grouping
//! - Performance targets (<30ms search latency)

mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::Utc;
use photoncast_core::indexer::{AppBundleId, IndexedApp};
use photoncast_core::search::{
    AppProvider, FuzzyMatcher, ResultType, SearchConfig, SearchEngine, SearchProvider,
};

// =============================================================================
// Fuzzy Matcher Tests (Task 2.4.8)
// =============================================================================

#[test]
fn test_fuzzy_matcher_exact_match() {
    let mut matcher = FuzzyMatcher::default();

    let result = matcher.score("Safari", "Safari");
    assert!(result.is_some());

    let (score, indices) = result.unwrap();
    assert!(score > 0);
    assert_eq!(indices.len(), 6); // "Safari" has 6 characters
}

#[test]
fn test_fuzzy_matcher_prefix_match() {
    let mut matcher = FuzzyMatcher::default();

    let result = matcher.score("Saf", "Safari");
    assert!(result.is_some());

    let (score, indices) = result.unwrap();
    assert!(score > 0);
    assert_eq!(indices, vec![0, 1, 2]); // First 3 characters should match
}

#[test]
fn test_fuzzy_matcher_no_match() {
    let mut matcher = FuzzyMatcher::default();

    let result = matcher.score("xyz", "Safari");
    assert!(result.is_none());
}

#[test]
fn test_fuzzy_matcher_case_insensitive() {
    let mut matcher = FuzzyMatcher::default();

    // Lowercase query should match titlecase target (smart case)
    let result = matcher.score("safari", "Safari");
    assert!(result.is_some());
}

#[test]
fn test_fuzzy_matcher_fuzzy_characters() {
    let mut matcher = FuzzyMatcher::default();

    // Non-consecutive character matching
    let result = matcher.score("sfr", "Safari");
    assert!(result.is_some(), "Fuzzy match should work for sfr -> Safari");

    let (score, indices) = result.unwrap();
    assert!(score > 0);
    assert!(!indices.is_empty());
}

#[test]
fn test_fuzzy_matcher_score_consistency() {
    let mut matcher = FuzzyMatcher::default();

    // Same query/target should return same score
    let result1 = matcher.score("test", "TestApp");
    let result2 = matcher.score("test", "TestApp");

    assert_eq!(result1, result2, "Scores should be consistent");
}

#[test]
fn test_fuzzy_matcher_prefix_scores_higher() {
    let mut matcher = FuzzyMatcher::default();

    let prefix_result = matcher.score("Saf", "Safari");
    let middle_result = matcher.score("ari", "Safari");

    assert!(prefix_result.is_some());
    assert!(middle_result.is_some());

    let (prefix_score, _) = prefix_result.unwrap();
    let (middle_score, _) = middle_result.unwrap();

    // Prefix matches should score higher or equal due to bonus
    assert!(
        prefix_score >= middle_score,
        "Prefix score {} should be >= middle score {}",
        prefix_score,
        middle_score
    );
}

#[test]
fn test_fuzzy_matcher_match_indices_correct() {
    let mut matcher = FuzzyMatcher::default();

    let result = matcher.score("abc", "aXbXc");
    assert!(result.is_some());

    let (_, indices) = result.unwrap();
    // Indices should point to 'a', 'b', 'c' in "aXbXc"
    assert_eq!(indices.len(), 3);
    assert_eq!(indices[0], 0); // 'a' at position 0
    assert_eq!(indices[1], 2); // 'b' at position 2
    assert_eq!(indices[2], 4); // 'c' at position 4
}

#[test]
fn test_fuzzy_matcher_empty_query() {
    let mut matcher = FuzzyMatcher::default();

    let result = matcher.score("", "Safari");
    assert!(result.is_some());

    let (score, indices) = result.unwrap();
    assert_eq!(score, 0);
    assert!(indices.is_empty());
}

#[test]
fn test_fuzzy_matcher_empty_target() {
    let mut matcher = FuzzyMatcher::default();

    let result = matcher.score("test", "");
    assert!(result.is_none());
}

// =============================================================================
// Search Engine Tests (Task 2.4.9)
// =============================================================================

#[test]
fn test_search_engine_empty_query() {
    let engine = SearchEngine::new();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let results = rt.block_on(engine.search(""));

    assert!(results.is_empty());
}

#[test]
fn test_search_engine_provider_count() {
    let mut engine = SearchEngine::new();
    assert_eq!(engine.provider_count(), 0);
    assert!(engine.is_empty());

    engine.add_provider(AppProvider::new());
    assert_eq!(engine.provider_count(), 1);
    assert!(!engine.is_empty());
}

#[test]
fn test_search_engine_with_apps() {
    let mut engine = SearchEngine::new();
    let provider = AppProvider::new();

    // Add some test apps
    provider.add_apps(vec![
        create_test_app("Safari", "com.apple.Safari"),
        create_test_app("System Preferences", "com.apple.systempreferences"),
        create_test_app("Xcode", "com.apple.dt.Xcode"),
    ]);

    engine.add_provider(provider);

    let results = engine.search_sync("saf");
    assert!(!results.is_empty());
    assert!(results.total_count > 0);
}

#[test]
fn test_search_engine_groups_results() {
    let mut engine = SearchEngine::new();
    let provider = AppProvider::new();

    provider.add_apps(vec![
        create_test_app("Safari", "com.apple.Safari"),
        create_test_app("Chrome", "com.google.Chrome"),
    ]);

    engine.add_provider(provider);

    let results = engine.search_sync("a");
    assert!(!results.groups.is_empty());
    assert_eq!(results.groups[0].result_type, ResultType::Application);
}

#[test]
fn test_search_engine_respects_config() {
    let config = SearchConfig {
        max_results_per_provider: 2,
        max_total_results: 3,
        ..Default::default()
    };

    let mut engine = SearchEngine::with_config(config);
    let provider = AppProvider::new();

    provider.add_apps(vec![
        create_test_app("App1", "com.test.1"),
        create_test_app("App2", "com.test.2"),
        create_test_app("App3", "com.test.3"),
        create_test_app("App4", "com.test.4"),
        create_test_app("App5", "com.test.5"),
    ]);

    engine.add_provider(provider);

    let results = engine.search_sync("app");
    assert!(results.total_count <= 3, "Should respect max_total_results");
}

#[test]
fn test_search_performance_target() {
    // Target: <30ms search latency
    let mut engine = SearchEngine::new();
    let provider = AppProvider::new();

    // Add 200 apps to simulate real usage
    let apps: Vec<IndexedApp> = (0..200)
        .map(|i| create_test_app(&format!("TestApp{}", i), &format!("com.test.app{}", i)))
        .collect();

    provider.add_apps(apps);
    engine.add_provider(provider);

    // Warm up
    let _ = engine.search_sync("test");

    // Measure search time
    let start = Instant::now();
    let results = engine.search_sync("testapp");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(30),
        "Search took {:?}, should be <30ms",
        elapsed
    );
    assert!(!results.is_empty());
}

#[test]
fn test_search_results_have_match_indices() {
    let mut engine = SearchEngine::new();
    let provider = AppProvider::new();

    provider.add_apps(vec![create_test_app("Safari", "com.apple.Safari")]);
    engine.add_provider(provider);

    let results = engine.search_sync("saf");
    assert!(!results.is_empty());

    let first_result = results.iter().next().unwrap();
    assert!(
        !first_result.match_indices.is_empty(),
        "Results should have match indices for highlighting"
    );
}

#[test]
fn test_search_async() {
    let mut engine = SearchEngine::new();
    let provider = AppProvider::new();

    provider.add_apps(vec![create_test_app("Safari", "com.apple.Safari")]);
    engine.add_provider(provider);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let results = rt.block_on(engine.search("safari"));

    assert!(!results.is_empty());
    assert_eq!(results.total_count, 1);
}

#[test]
fn test_search_result_actions() {
    use photoncast_core::search::SearchAction;

    let mut engine = SearchEngine::new();
    let provider = AppProvider::new();

    provider.add_apps(vec![create_test_app("Safari", "com.apple.Safari")]);
    engine.add_provider(provider);

    let results = engine.search_sync("Safari");
    let first = results.iter().next().unwrap();

    match &first.action {
        SearchAction::LaunchApp { bundle_id, path } => {
            assert_eq!(bundle_id, "com.apple.Safari");
            assert!(path.to_string_lossy().contains("Safari"));
        }
        _ => panic!("Expected LaunchApp action"),
    }
}

// =============================================================================
// Result Grouping Tests (Task 3.9.4)
// =============================================================================

#[test]
fn test_grouped_results_correct_order() {
    use photoncast_core::search::{
        GroupedResult, IconSource, ResultGroup, SearchAction, SearchResult, SearchResultId,
        SearchResults,
    };

    // Create results with different types
    let groups = vec![
        ResultGroup {
            result_type: ResultType::File,
            results: vec![SearchResult {
                id: SearchResultId::new("file1"),
                title: "Document.txt".to_string(),
                subtitle: "Files".to_string(),
                icon: IconSource::SystemIcon {
                    name: "doc".to_string(),
                },
                result_type: ResultType::File,
                score: 100.0,
                match_indices: vec![],
                action: SearchAction::OpenFile {
                    path: PathBuf::from("/test"),
                },
            }],
        },
        ResultGroup {
            result_type: ResultType::Application,
            results: vec![SearchResult {
                id: SearchResultId::new("app1"),
                title: "Safari".to_string(),
                subtitle: "Apps".to_string(),
                icon: IconSource::AppIcon {
                    bundle_id: "com.apple.Safari".to_string(),
                    icon_path: None,
                },
                result_type: ResultType::Application,
                score: 100.0,
                match_indices: vec![],
                action: SearchAction::LaunchApp {
                    bundle_id: "com.apple.Safari".to_string(),
                    path: PathBuf::from("/Applications/Safari.app"),
                },
            }],
        },
        ResultGroup {
            result_type: ResultType::SystemCommand,
            results: vec![SearchResult {
                id: SearchResultId::new("cmd1"),
                title: "Sleep".to_string(),
                subtitle: "Commands".to_string(),
                icon: IconSource::SystemIcon {
                    name: "sleep".to_string(),
                },
                result_type: ResultType::SystemCommand,
                score: 100.0,
                match_indices: vec![],
                action: SearchAction::ExecuteCommand {
                    command_id: "sleep".to_string(),
                },
            }],
        },
    ];

    let search_results = SearchResults {
        groups,
        total_count: 3,
        query: "test".to_string(),
        search_time: Duration::from_millis(10),
    };

    // The grouped() method should reorder by priority
    let grouped = search_results.grouped();
    assert_eq!(grouped.len(), 3);

    // Verify order: Apps → Commands → Files (based on priority)
    assert_eq!(grouped[0].result_type, ResultType::Application);
    assert_eq!(grouped[0].name, "Apps");
    assert_eq!(grouped[1].result_type, ResultType::SystemCommand);
    assert_eq!(grouped[1].name, "Commands");
    assert_eq!(grouped[2].result_type, ResultType::File);
    assert_eq!(grouped[2].name, "Files");
}

#[test]
fn test_grouped_results_shortcut_indices() {
    use photoncast_core::search::{
        IconSource, ResultGroup, SearchAction, SearchResult, SearchResultId, SearchResults,
    };

    let groups = vec![
        ResultGroup {
            result_type: ResultType::Application,
            results: vec![
                create_search_result("app1", "Safari", ResultType::Application),
                create_search_result("app2", "Chrome", ResultType::Application),
                create_search_result("app3", "Firefox", ResultType::Application),
            ],
        },
        ResultGroup {
            result_type: ResultType::SystemCommand,
            results: vec![
                create_search_result("cmd1", "Sleep", ResultType::SystemCommand),
                create_search_result("cmd2", "Lock", ResultType::SystemCommand),
            ],
        },
        ResultGroup {
            result_type: ResultType::File,
            results: vec![
                create_search_result("file1", "Document.txt", ResultType::File),
                create_search_result("file2", "Notes.md", ResultType::File),
            ],
        },
    ];

    let search_results = SearchResults {
        groups,
        total_count: 7,
        query: "test".to_string(),
        search_time: Duration::from_millis(10),
    };

    let grouped = search_results.grouped();

    // Apps: shortcuts ⌘1-3 (start at 0)
    assert_eq!(grouped[0].shortcut_start, 0);
    assert_eq!(grouped[0].shortcut_hint(), Some("⌘1-3".to_string()));

    // Commands: shortcuts ⌘4-5 (start at 3)
    assert_eq!(grouped[1].shortcut_start, 3);
    assert_eq!(grouped[1].shortcut_hint(), Some("⌘4-5".to_string()));

    // Files: shortcuts ⌘6-7 (start at 5)
    assert_eq!(grouped[2].shortcut_start, 5);
    assert_eq!(grouped[2].shortcut_hint(), Some("⌘6-7".to_string()));
}

#[test]
fn test_group_navigation_next() {
    use photoncast_core::search::{ResultGroup, SearchResults};

    let groups = vec![
        ResultGroup {
            result_type: ResultType::Application,
            results: vec![
                create_search_result("app1", "Safari", ResultType::Application),
                create_search_result("app2", "Chrome", ResultType::Application),
            ],
        },
        ResultGroup {
            result_type: ResultType::SystemCommand,
            results: vec![create_search_result("cmd1", "Sleep", ResultType::SystemCommand)],
        },
        ResultGroup {
            result_type: ResultType::File,
            results: vec![
                create_search_result("file1", "Document.txt", ResultType::File),
                create_search_result("file2", "Notes.md", ResultType::File),
            ],
        },
    ];

    let search_results = SearchResults {
        groups,
        total_count: 5,
        query: "test".to_string(),
        search_time: Duration::from_millis(10),
    };

    // Starting at index 0 (first app), next group should be index 2 (first command)
    assert_eq!(search_results.next_group_start(0), Some(2));
    assert_eq!(search_results.next_group_start(1), Some(2));

    // Starting at index 2 (first command), next group should be index 3 (first file)
    assert_eq!(search_results.next_group_start(2), Some(3));

    // Starting at index 3 or 4 (files), next group should wrap to index 0 (first app)
    assert_eq!(search_results.next_group_start(3), Some(0));
    assert_eq!(search_results.next_group_start(4), Some(0));
}

#[test]
fn test_group_navigation_previous() {
    use photoncast_core::search::{ResultGroup, SearchResults};

    let groups = vec![
        ResultGroup {
            result_type: ResultType::Application,
            results: vec![
                create_search_result("app1", "Safari", ResultType::Application),
                create_search_result("app2", "Chrome", ResultType::Application),
            ],
        },
        ResultGroup {
            result_type: ResultType::SystemCommand,
            results: vec![create_search_result("cmd1", "Sleep", ResultType::SystemCommand)],
        },
        ResultGroup {
            result_type: ResultType::File,
            results: vec![create_search_result("file1", "Document.txt", ResultType::File)],
        },
    ];

    let search_results = SearchResults {
        groups,
        total_count: 4,
        query: "test".to_string(),
        search_time: Duration::from_millis(10),
    };

    // Starting at index 0 or 1 (apps), previous group should wrap to index 3 (first file)
    assert_eq!(search_results.previous_group_start(0), Some(3));
    assert_eq!(search_results.previous_group_start(1), Some(3));

    // Starting at index 2 (command), previous group should be index 0 (first app)
    assert_eq!(search_results.previous_group_start(2), Some(0));

    // Starting at index 3 (file), previous group should be index 2 (first command)
    assert_eq!(search_results.previous_group_start(3), Some(2));
}

#[test]
fn test_group_index_for_result() {
    use photoncast_core::search::{ResultGroup, SearchResults};

    let groups = vec![
        ResultGroup {
            result_type: ResultType::Application,
            results: vec![
                create_search_result("app1", "Safari", ResultType::Application),
                create_search_result("app2", "Chrome", ResultType::Application),
            ],
        },
        ResultGroup {
            result_type: ResultType::SystemCommand,
            results: vec![create_search_result("cmd1", "Sleep", ResultType::SystemCommand)],
        },
    ];

    let search_results = SearchResults {
        groups,
        total_count: 3,
        query: "test".to_string(),
        search_time: Duration::from_millis(10),
    };

    // Indices 0, 1 are in group 0
    assert_eq!(search_results.group_index_for_result(0), Some(0));
    assert_eq!(search_results.group_index_for_result(1), Some(0));

    // Index 2 is in group 1
    assert_eq!(search_results.group_index_for_result(2), Some(1));

    // Out of bounds
    assert_eq!(search_results.group_index_for_result(3), None);
}

#[test]
fn test_shortcut_hint_edge_cases() {
    use photoncast_core::search::GroupedResult;

    // Empty group should have no hint
    let empty_group = GroupedResult {
        result_type: ResultType::Application,
        name: "Apps",
        items: vec![],
        shortcut_start: 0,
    };
    assert_eq!(empty_group.shortcut_hint(), None);

    // Single item should show single shortcut
    let single_group = GroupedResult {
        result_type: ResultType::Application,
        name: "Apps",
        items: vec![create_search_result("app1", "Safari", ResultType::Application)],
        shortcut_start: 0,
    };
    assert_eq!(single_group.shortcut_hint(), Some("⌘1".to_string()));

    // Group starting at index 9+ should have no hint
    let overflow_group = GroupedResult {
        result_type: ResultType::File,
        name: "Files",
        items: vec![create_search_result("file1", "Test", ResultType::File)],
        shortcut_start: 9,
    };
    assert_eq!(overflow_group.shortcut_hint(), None);

    // Group that spans past 9 should be capped
    let capped_group = GroupedResult {
        result_type: ResultType::File,
        name: "Files",
        items: (0..5)
            .map(|i| create_search_result(&format!("file{}", i), "Test", ResultType::File))
            .collect(),
        shortcut_start: 7,
    };
    // Should show ⌘8-9 (only 2 shortcuts available: 8 and 9)
    assert_eq!(capped_group.shortcut_hint(), Some("⌘8-9".to_string()));
}

// =============================================================================
// Helper Functions
// =============================================================================

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

fn create_search_result(
    id: &str,
    title: &str,
    result_type: ResultType,
) -> photoncast_core::search::SearchResult {
    use photoncast_core::search::{IconSource, SearchAction, SearchResultId};

    photoncast_core::search::SearchResult {
        id: SearchResultId::new(id),
        title: title.to_string(),
        subtitle: format!("{:?}", result_type),
        icon: IconSource::SystemIcon {
            name: "test".to_string(),
        },
        result_type,
        score: 100.0,
        match_indices: vec![],
        action: SearchAction::OpenFile {
            path: PathBuf::from("/test"),
        },
    }
}
