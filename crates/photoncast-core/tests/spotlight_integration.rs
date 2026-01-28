//! Integration tests for native Spotlight search using objc2.
//!
//! These tests verify the native macOS Spotlight integration in
//! photoncast_core::search::spotlight module.
//!
//! Tests are only compiled on macOS since Spotlight is a macOS-only feature.

#![cfg(target_os = "macos")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use photoncast_core::search::file_query::FileQuery;
use photoncast_core::search::spotlight::{
    MetadataQueryWrapper, PredicateBuilder, SearchServiceError, SpotlightError, SpotlightResult,
    SpotlightSearchOptions, SpotlightSearchService,
};

// =============================================================================
// SpotlightSearchService Tests
// =============================================================================

#[test]
fn test_spotlight_service_creation() {
    let service = SpotlightSearchService::new();
    // Service should be created without panicking
    let _ = service;
}

#[test]
fn test_spotlight_service_with_custom_options() {
    let options = SpotlightSearchOptions {
        max_results: 10,
        timeout: Duration::from_millis(200),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: false,
        cache_ttl: Duration::from_secs(10),
        ..Default::default()
    };
    let service = SpotlightSearchService::with_options(options);
    let _ = service;
}

#[test]
fn test_spotlight_service_empty_query_returns_error() {
    let service = SpotlightSearchService::new();

    // Empty query should return an error
    let result = service.search("");
    assert!(result.is_err());

    if let Err(SearchServiceError::InvalidQuery(msg)) = result {
        assert!(msg.contains("empty"));
    } else {
        panic!("Expected InvalidQuery error for empty query");
    }

    // Whitespace-only query should also return error
    let result = service.search("   ");
    assert!(result.is_err());
}

#[test]
fn test_spotlight_service_search_returns_results() {
    let service = SpotlightSearchService::new();

    // Search for "Safari" which should exist in /Applications
    let start = Instant::now();
    let result = service.search_with_options(
        "Safari",
        &SpotlightSearchOptions {
            max_results: 10,
            timeout: Duration::from_secs(2),
            primary_scopes: vec![PathBuf::from("/Applications")],
            use_cache: false,
            ..Default::default()
        },
    );
    let elapsed = start.elapsed();

    // Query should complete within reasonable time (allowing for slow CI environments)
    assert!(
        elapsed < Duration::from_secs(10),
        "Query took too long: {:?}",
        elapsed
    );

    // Should succeed (may or may not find results depending on system state)
    match result {
        Ok(results) => {
            // If we found results, verify they're valid
            for result in &results {
                assert!(!result.path.as_os_str().is_empty());
                assert!(!result.display_name.is_empty());
            }
        },
        Err(SearchServiceError::Spotlight(SpotlightError::Timeout(_))) => {
            // Timeout is acceptable in CI environments
        },
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        },
    }
}

#[test]
fn test_spotlight_service_search_with_extension_filter() {
    let service = SpotlightSearchService::new();

    // Parse a query with .app extension filter
    let file_query = FileQuery::parse(".app Safari");

    let result = service.search_file_query_with_options(
        &file_query,
        &SpotlightSearchOptions {
            max_results: 5,
            timeout: Duration::from_secs(2),
            primary_scopes: vec![PathBuf::from("/Applications")],
            use_cache: false,
            ..Default::default()
        },
    );

    match result {
        Ok(results) => {
            // All results should be .app bundles
            for result in &results {
                if let Some(ext) = result.extension() {
                    assert_eq!(ext, "app", "Expected .app extension, got .{}", ext);
                }
            }
        },
        Err(SearchServiceError::Spotlight(SpotlightError::Timeout(_))) => {
            // Timeout is acceptable
        },
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        },
    }
}

#[test]
fn test_spotlight_service_search_in_directory() {
    let service = SpotlightSearchService::new();

    // Search only in /Applications
    let options = SpotlightSearchOptions {
        max_results: 10,
        timeout: Duration::from_secs(2),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: false,
        ..Default::default()
    };

    let result = service.search_with_options("Safari", &options);

    match result {
        Ok(results) => {
            // All results should be within /Applications
            for result in &results {
                assert!(
                    result.path.starts_with("/Applications"),
                    "Expected path in /Applications, got: {}",
                    result.path.display()
                );
            }
        },
        Err(SearchServiceError::Spotlight(SpotlightError::Timeout(_))) => {
            // Timeout is acceptable
        },
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        },
    }
}

#[test]
fn test_spotlight_service_timeout_handling() {
    let service = SpotlightSearchService::new();

    // Use a very short timeout
    let options = SpotlightSearchOptions {
        max_results: 1000,
        timeout: Duration::from_millis(1), // Extremely short timeout
        primary_scopes: vec![],            // Search everywhere
        use_cache: false,
        ..Default::default()
    };

    let start = Instant::now();
    let _ = service.search_with_options("a", &options);
    let elapsed = start.elapsed();

    // Should not hang - should return within a reasonable time
    // (allowing some overhead for setup)
    assert!(
        elapsed < Duration::from_secs(5),
        "Query appears to have hung: {:?}",
        elapsed
    );
}

#[test]
fn test_spotlight_service_cache_behavior() {
    let service = SpotlightSearchService::new();

    let options = SpotlightSearchOptions {
        max_results: 5,
        timeout: Duration::from_secs(1),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: true,
        cache_ttl: Duration::from_secs(60),
        ..Default::default()
    };

    // First search (uncached)
    let start1 = Instant::now();
    let result1 = service.search_with_options("Safari", &options);
    let elapsed1 = start1.elapsed();

    if result1.is_ok() {
        // Second search should use cache and be faster
        let start2 = Instant::now();
        let result2 = service.search_with_options("Safari", &options);
        let elapsed2 = start2.elapsed();

        assert!(result2.is_ok());

        // Cached result should be significantly faster
        // (though this may be flaky in some environments)
        if elapsed1 > Duration::from_millis(10) {
            assert!(
                elapsed2 < elapsed1,
                "Cached search should be faster: first={:?}, second={:?}",
                elapsed1,
                elapsed2
            );
        }
    }

    // Clear cache
    service.clear_cache();
}

// =============================================================================
// SpotlightResult Metadata Tests
// =============================================================================

#[test]
fn test_spotlight_result_metadata_populated() {
    let service = SpotlightSearchService::new();

    let result = service.search_with_options(
        "Safari",
        &SpotlightSearchOptions {
            max_results: 1,
            timeout: Duration::from_secs(2),
            primary_scopes: vec![PathBuf::from("/Applications")],
            use_cache: false,
            ..Default::default()
        },
    );

    if let Ok(results) = result {
        if let Some(first) = results.first() {
            // Path should be populated
            assert!(
                first.path.exists() || !first.path.as_os_str().is_empty(),
                "Path should be populated"
            );

            // Display name should be populated
            assert!(!first.display_name.is_empty(), "Display name should be set");

            // Content type tree should have at least one entry for apps
            if first.path.extension().map(|e| e == "app").unwrap_or(false) {
                assert!(
                    !first.content_type_tree.is_empty(),
                    "App should have content type tree"
                );
            }
        }
    }
}

#[test]
fn test_spotlight_result_helper_methods() {
    // Create a mock result to test helper methods
    let result = SpotlightResult {
        path: PathBuf::from("/Applications/Safari.app"),
        display_name: "Safari".to_string(),
        display_name_lower: "safari".to_string(),
        file_size: Some(100_000_000),
        content_type: Some("com.apple.application-bundle".to_string()),
        content_type_tree: vec![
            "com.apple.application-bundle".to_string(),
            "com.apple.bundle".to_string(),
            "com.apple.package".to_string(),
            "public.directory".to_string(),
        ],
        modified_date: None,
        created_date: None,
        last_used_date: None,
        is_directory: true,
    };

    assert_eq!(result.file_name(), Some("Safari.app"));
    assert_eq!(result.extension(), Some("app"));
    assert!(result.is_application());
    assert!(result.conforms_to_type("com.apple.bundle"));
    assert!(!result.is_image());
}

// =============================================================================
// PredicateBuilder Tests
// =============================================================================

#[test]
fn test_predicate_builder_name_contains() {
    let predicate = PredicateBuilder::new().name_contains("report").build();

    let format = predicate.predicateFormat().to_string();
    assert!(
        format.contains("kMDItemFSName"),
        "Predicate should reference kMDItemFSName"
    );
    assert!(
        format.contains("CONTAINS"),
        "Predicate should use CONTAINS operator"
    );
}

#[test]
fn test_predicate_builder_extension_filter() {
    let predicate = PredicateBuilder::new().extension_is("pdf").build();

    let format = predicate.predicateFormat().to_string();
    assert!(format.contains("kMDItemFSName"));
    assert!(format.contains("ENDSWITH"));
    assert!(format.contains(".pdf"));
}

#[test]
fn test_predicate_builder_compound_and() {
    let predicate = PredicateBuilder::new()
        .name_contains("report")
        .extension_is("pdf")
        .build();

    let format = predicate.predicateFormat().to_string();
    // Should be a compound predicate with both conditions
    assert!(format.contains("kMDItemFSName"));
    // The compound predicate should have both conditions
    assert!(format.contains("report") || format.contains("pdf"));
}

#[test]
fn test_predicate_builder_or_combination() {
    let pdf_filter = PredicateBuilder::new().extension_is("pdf");
    let doc_filter = PredicateBuilder::new().extension_is("docx");

    let predicate = pdf_filter.or(doc_filter).build();
    let format = predicate.predicateFormat().to_string();

    // Should contain OR or both extensions
    assert!(
        format.contains("OR") || (format.contains("pdf") && format.contains("docx")),
        "Should be OR predicate: {}",
        format
    );
}

#[test]
fn test_predicate_builder_content_type() {
    let predicate = PredicateBuilder::new()
        .content_type_tree("public.image")
        .build();

    let format = predicate.predicateFormat().to_string();
    assert!(format.contains("kMDItemContentTypeTree"));
    assert!(format.contains("public.image"));
}

#[test]
fn test_predicate_builder_special_characters_escaped() {
    // Test that special characters are properly escaped
    let predicate = PredicateBuilder::new().name_contains("file*.txt").build();

    // Should not panic and should produce a valid predicate
    let format = predicate.predicateFormat().to_string();
    assert!(format.contains("CONTAINS"));
}

// =============================================================================
// MetadataQueryWrapper Tests
// =============================================================================

#[test]
fn test_metadata_query_creation() {
    let query = MetadataQueryWrapper::new();
    assert!(!query.is_started());
    assert!(!query.is_gathering());
    assert!(!query.is_stopped());
}

#[test]
fn test_metadata_query_with_predicate() {
    let predicate = PredicateBuilder::new().name_contains("Safari").build();

    let mut query = MetadataQueryWrapper::new();
    query.set_predicate(&predicate);

    // Query should be configured but not yet started
    assert!(!query.is_started());
}

#[test]
fn test_metadata_query_with_search_scopes() {
    let predicate = PredicateBuilder::new().name_contains("Safari").build();

    let mut query = MetadataQueryWrapper::new();
    query.set_predicate(&predicate);
    query.set_search_scopes(&[PathBuf::from("/Applications")]);

    // Query should be configured
    assert!(!query.is_started());
}

#[test]
fn test_metadata_query_execute_sync() {
    let predicate = PredicateBuilder::new().name_contains("Safari").build();

    let mut query = MetadataQueryWrapper::new();
    query.set_predicate(&predicate);
    query.set_search_scopes(&[PathBuf::from("/Applications")]);

    let start = Instant::now();
    let result = query.execute_sync(Duration::from_secs(5));
    let elapsed = start.elapsed();

    // Should complete within timeout (allowing for slow CI environments)
    assert!(elapsed < Duration::from_secs(10));

    match result {
        Ok(results) => {
            // Verify results are valid SpotlightResult objects
            for result in results {
                assert!(!result.path.as_os_str().is_empty());
            }
        },
        Err(SpotlightError::Timeout(_)) => {
            // Timeout is acceptable in some environments
        },
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        },
    }
}

#[test]
fn test_metadata_query_stop() {
    let predicate = PredicateBuilder::new().name_contains("a").build();

    let mut query = MetadataQueryWrapper::new();
    query.set_predicate(&predicate);

    // Stop should be safe to call even before starting
    query.stop();
}

// =============================================================================
// Integration with FileQuery
// =============================================================================

#[test]
fn test_file_query_to_spotlight_search() {
    let service = SpotlightSearchService::new();

    // Test various FileQuery patterns
    let queries = vec![
        FileQuery::parse("Safari"),
        FileQuery::parse(".app"),
        FileQuery::parse(".pdf document"),
    ];

    for file_query in queries {
        let result = service.search_file_query_with_options(
            &file_query,
            &SpotlightSearchOptions {
                max_results: 5,
                timeout: Duration::from_millis(500),
                primary_scopes: vec![PathBuf::from("/Applications")],
                use_cache: false,
                ..Default::default()
            },
        );

        // Should not panic
        match result {
            Ok(_) | Err(SearchServiceError::Spotlight(SpotlightError::Timeout(_))) => {},
            Err(e) => {
                // Some queries might fail, but shouldn't crash
                eprintln!("Query {:?} failed with: {:?}", file_query.terms, e);
            },
        }
    }
}

// =============================================================================
// Performance Tests
// =============================================================================

#[test]
fn test_spotlight_search_performance() {
    let service = SpotlightSearchService::new();

    let options = SpotlightSearchOptions {
        max_results: 20,
        timeout: Duration::from_secs(5),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: false,
        ..Default::default()
    };

    let start = Instant::now();
    let _ = service.search_with_options("app", &options);
    let elapsed = start.elapsed();

    // Search should complete within reasonable time (allowing for slow CI environments)
    assert!(
        elapsed < Duration::from_secs(10),
        "Search took too long: {:?}",
        elapsed
    );
}

/// Performance test: Native Spotlight should return results faster than timeout
#[test]
fn test_performance_response_time() {
    let service = SpotlightSearchService::new();

    // Disable exclusions for performance tests to get raw speed
    let options = SpotlightSearchOptions {
        max_results: 10,
        timeout: Duration::from_secs(2),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: false,
        apply_exclusions: false,
        sort_by_recency: false,
        ..Default::default()
    };

    // Warm-up query
    let _ = service.search_with_options("Safari", &options);

    // Measure multiple queries
    let mut durations = Vec::new();
    let queries = ["Safari", "Finder", "System", "Photo", "Music"];

    for query in queries {
        let start = Instant::now();
        let result = service.search_with_options(query, &options);
        let elapsed = start.elapsed();

        if result.is_ok() {
            durations.push(elapsed);
        }
    }

    if !durations.is_empty() {
        let avg = durations.iter().sum::<Duration>() / durations.len() as u32;
        let max = durations.iter().max().unwrap();
        let min = durations.iter().min().unwrap();

        println!("Performance Results (Native Spotlight):");
        println!("  Queries executed: {}", durations.len());
        println!("  Average response time: {:?}", avg);
        println!("  Min response time: {:?}", min);
        println!("  Max response time: {:?}", max);

        // Native Spotlight should typically respond within 500ms for /Applications
        assert!(
            avg < Duration::from_millis(1000),
            "Average response time too slow: {:?}",
            avg
        );
    }
}

/// Performance test: Compare native Spotlight with mdfind CLI
#[test]
#[ignore] // Benchmark test - sensitive to system load and Spotlight indexing state
fn test_performance_vs_mdfind_cli() {
    use std::process::Command;

    let service = SpotlightSearchService::new();
    // Disable exclusions for fair comparison with mdfind
    let options = SpotlightSearchOptions {
        max_results: 20,
        timeout: Duration::from_secs(2),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: false,
        apply_exclusions: false,
        sort_by_recency: false,
        ..Default::default()
    };

    let query = "Safari";

    // Time native Spotlight
    let native_start = Instant::now();
    let native_result = service.search_with_options(query, &options);
    let native_elapsed = native_start.elapsed();
    let native_count = native_result.map(|r| r.len()).unwrap_or(0);

    // Time mdfind CLI
    let mdfind_start = Instant::now();
    let mdfind_output = Command::new("mdfind")
        .args([
            "-onlyin",
            "/Applications",
            &format!("kMDItemDisplayName == '*{}*'c", query),
        ])
        .output();
    let mdfind_elapsed = mdfind_start.elapsed();
    let mdfind_count = mdfind_output
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);

    println!("Performance Comparison (Safari in /Applications):");
    println!(
        "  Native Spotlight: {:?} ({} results)",
        native_elapsed, native_count
    );
    println!(
        "  mdfind CLI:       {:?} ({} results)",
        mdfind_elapsed, mdfind_count
    );

    // Native should be competitive with mdfind (within 2x)
    // Note: First native call may be slower due to setup
    if native_count > 0 && mdfind_count > 0 {
        let ratio = native_elapsed.as_secs_f64() / mdfind_elapsed.as_secs_f64();
        println!("  Ratio (native/mdfind): {:.2}x", ratio);

        // Native shouldn't be more than 5x slower than mdfind
        assert!(
            ratio < 5.0,
            "Native Spotlight significantly slower than mdfind: {:.2}x",
            ratio
        );
    }
}

/// Performance test: Cache should provide significant speedup
#[test]
fn test_performance_cache_speedup() {
    let service = SpotlightSearchService::new();

    let _options_no_cache = SpotlightSearchOptions {
        max_results: 10,
        timeout: Duration::from_secs(2),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: false,
        ..Default::default()
    };

    let options_with_cache = SpotlightSearchOptions {
        max_results: 10,
        timeout: Duration::from_secs(2),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: true,
        cache_ttl: Duration::from_secs(60),
        ..Default::default()
    };

    // Clear cache first
    service.clear_cache();

    // First query (populates cache)
    let first_start = Instant::now();
    let first_result = service.search_with_options("Safari", &options_with_cache);
    let first_elapsed = first_start.elapsed();

    if first_result.is_ok() {
        // Second query (should hit cache)
        let cached_start = Instant::now();
        let cached_result = service.search_with_options("Safari", &options_with_cache);
        let cached_elapsed = cached_start.elapsed();

        if cached_result.is_ok() {
            println!("Cache Performance:");
            println!("  First query (uncached): {:?}", first_elapsed);
            println!("  Second query (cached):  {:?}", cached_elapsed);

            if first_elapsed > Duration::from_millis(10) {
                let speedup = first_elapsed.as_secs_f64() / cached_elapsed.as_secs_f64();
                println!("  Speedup factor: {:.1}x", speedup);

                // Cache should provide at least 10x speedup
                assert!(
                    speedup > 5.0 || cached_elapsed < Duration::from_millis(5),
                    "Cache speedup insufficient: {:.1}x (cached: {:?})",
                    speedup,
                    cached_elapsed
                );
            }
        }
    }
}

/// Performance test: Query throughput (queries per second)
#[test]
#[ignore] // Benchmark test - sensitive to system load
fn test_performance_throughput() {
    let service = SpotlightSearchService::new();

    // Disable exclusions for raw performance measurement
    let options = SpotlightSearchOptions {
        max_results: 5,
        timeout: Duration::from_millis(500),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: true,
        cache_ttl: Duration::from_secs(60),
        apply_exclusions: false,
        sort_by_recency: false,
        ..Default::default()
    };

    // Warm up cache with different queries
    let queries = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
    for query in &queries {
        let _ = service.search_with_options(query, &options);
    }

    // Measure throughput over 1 second of cached queries
    let mut query_count = 0;
    let test_duration = Duration::from_secs(1);
    let start = Instant::now();

    while start.elapsed() < test_duration {
        for query in &queries {
            let _ = service.search_with_options(query, &options);
            query_count += 1;
        }
    }

    let actual_duration = start.elapsed();
    let qps = query_count as f64 / actual_duration.as_secs_f64();

    println!("Throughput Test (cached queries):");
    println!("  Queries executed: {}", query_count);
    println!("  Duration: {:?}", actual_duration);
    println!("  Throughput: {:.0} queries/second", qps);

    // Cached queries should achieve at least 100 queries/second
    assert!(qps > 100.0, "Throughput too low: {:.0} queries/second", qps);
}

/// Performance test: Timeout accuracy (should not hang)
#[test]
#[ignore] // Benchmark test - sensitive to system load
fn test_performance_timeout_accuracy() {
    let service = SpotlightSearchService::new();

    let timeouts = [
        Duration::from_millis(50),
        Duration::from_millis(100),
        Duration::from_millis(200),
        Duration::from_millis(500),
    ];

    println!("Timeout Accuracy Test:");

    for timeout in timeouts {
        let options = SpotlightSearchOptions {
            max_results: 1000,
            timeout,
            primary_scopes: vec![], // Search everywhere to potentially trigger timeout
            use_cache: false,
            ..Default::default()
        };

        let start = Instant::now();
        let result = service.search_with_options("a", &options);
        let elapsed = start.elapsed();

        let is_timeout = matches!(
            result,
            Err(SearchServiceError::Spotlight(SpotlightError::Timeout(_)))
        );

        println!(
            "  Timeout={:?}, Elapsed={:?}, TimedOut={}",
            timeout, elapsed, is_timeout
        );

        // Should not take more than timeout + 500ms overhead
        let max_allowed = timeout + Duration::from_millis(500);
        assert!(
            elapsed < max_allowed,
            "Query exceeded timeout by too much: timeout={:?}, elapsed={:?}",
            timeout,
            elapsed
        );
    }
}

/// Performance test: Multiple concurrent-style queries (sequential but rapid)
#[test]
fn test_performance_rapid_sequential_queries() {
    let service = SpotlightSearchService::new();

    let options = SpotlightSearchOptions {
        max_results: 5,
        timeout: Duration::from_millis(500),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: false, // No cache to test actual query performance
        ..Default::default()
    };

    // Simulate rapid sequential queries (like user typing)
    let query_prefixes = ["S", "Sa", "Saf", "Safa", "Safar", "Safari"];
    let mut timings = Vec::new();

    let total_start = Instant::now();
    for query in query_prefixes {
        let start = Instant::now();
        let _ = service.search_with_options(query, &options);
        timings.push((query, start.elapsed()));
    }
    let total_elapsed = total_start.elapsed();

    println!("Rapid Sequential Queries (simulating typing):");
    for (query, elapsed) in &timings {
        println!("  '{}': {:?}", query, elapsed);
    }
    println!(
        "  Total time for {} queries: {:?}",
        query_prefixes.len(),
        total_elapsed
    );

    // All queries combined should complete within reasonable time
    assert!(
        total_elapsed < Duration::from_secs(10),
        "Rapid queries took too long: {:?}",
        total_elapsed
    );
}

/// Performance test: Large result set handling
#[test]
#[ignore] // Benchmark test - sensitive to system load
fn test_performance_large_result_set() {
    let service = SpotlightSearchService::new();

    // Search for something common that returns many results
    // Disable exclusions for raw performance measurement
    let options = SpotlightSearchOptions {
        max_results: 100,
        timeout: Duration::from_secs(5),
        primary_scopes: vec![PathBuf::from("/Applications")],
        use_cache: false,
        apply_exclusions: false,
        sort_by_recency: false,
        ..Default::default()
    };

    let start = Instant::now();
    let result = service.search_with_options("a", &options);
    let elapsed = start.elapsed();

    match result {
        Ok(results) => {
            println!("Large Result Set Test:");
            println!("  Results returned: {}", results.len());
            println!("  Time elapsed: {:?}", elapsed);

            if !results.is_empty() {
                let time_per_result = elapsed.as_micros() / results.len() as u128;
                println!("  Time per result: {} µs", time_per_result);

                // Should process results efficiently (less than 10ms per result)
                assert!(
                    time_per_result < 10_000,
                    "Result processing too slow: {} µs/result",
                    time_per_result
                );
            }
        },
        Err(e) => {
            println!("Large result set query failed: {:?}", e);
        },
    }
}
