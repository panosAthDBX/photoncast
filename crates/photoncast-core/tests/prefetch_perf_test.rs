//! Performance comparison test: with vs without prefetch and live index.
//!
//! Run with: cargo test -p photoncast-core --test prefetch_perf_test -- --nocapture

use std::sync::Arc;
use std::time::{Duration, Instant};

use photoncast_core::search::spotlight::live_index::{start_live_index, LiveIndexStatus};
use photoncast_core::search::spotlight::prefetch::{
    PrefetchConfig, PrefetchStatus, SpotlightPrefetcher,
};
use photoncast_core::search::spotlight::service::{SpotlightSearchOptions, SpotlightSearchService};

#[test]
#[cfg(target_os = "macos")]
fn compare_prefetch_performance() {
    println!("\n{}", "=".repeat(70));
    println!("SPOTLIGHT PREFETCH PERFORMANCE COMPARISON");
    println!("{}\n", "=".repeat(70));

    // Test queries
    let queries = ["doc", "report", "pdf", "readme", "test"];

    // =========================================================================
    // Test 1: Cold Search (no cache, no prefetch)
    // =========================================================================
    println!("1. COLD SEARCH (no cache, no prefetch)");
    println!("{}", "-".repeat(50));

    let cold_service = SpotlightSearchService::new();
    let cold_options = SpotlightSearchOptions {
        use_cache: false,
        max_results: 20,
        timeout: Duration::from_secs(2),
        ..Default::default()
    };

    let cold_start = Instant::now();
    for query in &queries {
        let _ = cold_service.search_with_options(query, &cold_options);
    }
    let cold_time = cold_start.elapsed();
    let cold_per_query = cold_time / queries.len() as u32;

    println!(
        "  Total time for {} queries: {:?}",
        queries.len(),
        cold_time
    );
    println!("  Average per query: {:?}\n", cold_per_query);

    // =========================================================================
    // Test 2: Cache Hit (same service, cached results)
    // =========================================================================
    println!("2. CACHE HIT (repeated queries on same service)");
    println!("{}", "-".repeat(50));

    let warm_service = SpotlightSearchService::new();
    let warm_options = SpotlightSearchOptions {
        use_cache: true,
        max_results: 20,
        timeout: Duration::from_secs(2),
        ..Default::default()
    };

    // Prime the cache
    for query in &queries {
        let _ = warm_service.search_with_options(query, &warm_options);
    }

    // Now measure cache hits
    let cache_start = Instant::now();
    for _ in 0..100 {
        for query in &queries {
            let _ = warm_service.search_with_options(query, &warm_options);
        }
    }
    let cache_time = cache_start.elapsed();
    let cache_per_query = cache_time / (100 * queries.len() as u32);

    println!(
        "  Total time for {} queries (100 iterations): {:?}",
        queries.len(),
        cache_time
    );
    println!("  Average per query (cache hit): {:?}", cache_per_query);
    let speedup = cold_per_query.as_nanos() as f64 / cache_per_query.as_nanos() as f64;
    println!("  Speedup vs cold: {:.0}x\n", speedup);

    // =========================================================================
    // Test 3: Prefetch + Instant Results
    // =========================================================================
    println!("3. PREFETCH + INSTANT RESULTS");
    println!("{}", "-".repeat(50));

    let service = Arc::new(SpotlightSearchService::new());
    let config = PrefetchConfig {
        initial_delay: Duration::from_millis(10),
        query_timeout: Duration::from_secs(3),
        recent_files_limit: 50,
        run_on_battery: true, // Force run even on battery for test
        ..Default::default()
    };
    let prefetcher = Arc::new(SpotlightPrefetcher::with_config(
        Arc::clone(&service),
        config,
    ));

    // Trigger prefetch
    let prefetch_start = Instant::now();
    prefetcher.trigger();

    // Wait for completion (max 10 seconds)
    let mut waited = Duration::ZERO;
    while prefetcher.status() != PrefetchStatus::Completed
        && prefetcher.status() != PrefetchStatus::Failed
        && waited < Duration::from_secs(10)
    {
        std::thread::sleep(Duration::from_millis(100));
        waited += Duration::from_millis(100);
    }
    let prefetch_time = prefetch_start.elapsed();

    println!("  Prefetch status: {:?}", prefetcher.status());
    println!("  Prefetch time: {:?}", prefetch_time);

    // Measure instant results access
    let instant_start = Instant::now();
    for _ in 0..1000 {
        let _ = prefetcher.get_recent_files();
    }
    let instant_time = instant_start.elapsed();
    let instant_per_access = instant_time / 1000;

    let recent_files = prefetcher.get_recent_files();
    println!("  Recent files prefetched: {}", recent_files.len());
    println!(
        "  Instant access time (1000 iterations): {:?}",
        instant_time
    );
    println!("  Average per access: {:?}", instant_per_access);

    // Now search with warmed cache from prefetch
    let warm_start = Instant::now();
    for query in &queries {
        let _ = service.search_with_options(query, &warm_options);
    }
    let warm_time = warm_start.elapsed();
    let warm_per_query = warm_time / queries.len() as u32;

    println!(
        "  Search with warmed cache: {:?} ({:?} per query)\n",
        warm_time, warm_per_query
    );

    // =========================================================================
    // Test 4: Live Index (Spotlight-monitored, in-memory search)
    // =========================================================================
    println!("4. LIVE INDEX (Spotlight-monitored, in-memory search)");
    println!("{}", "-".repeat(50));

    let live_index = start_live_index();

    // Wait for it to become live (max 15 seconds)
    let live_start = Instant::now();
    while live_index.status() != LiveIndexStatus::Live
        && live_index.status() != LiveIndexStatus::Failed
        && live_start.elapsed() < Duration::from_secs(15)
    {
        std::thread::sleep(Duration::from_millis(100));
    }
    let live_init_time = live_start.elapsed();

    println!("  Live index status: {:?}", live_index.status());
    println!("  Initial population time: {:?}", live_init_time);
    println!("  Files indexed: {}", live_index.file_count());

    // Measure live index search performance
    let live_search_time;
    let live_per_query;

    if live_index.is_ready() && live_index.file_count() > 0 {
        let live_query_start = Instant::now();
        for _ in 0..1000 {
            for query in &queries {
                let _ = live_index.search(query, 20);
            }
        }
        live_search_time = live_query_start.elapsed();
        live_per_query = live_search_time / (1000 * queries.len() as u32);

        println!(
            "  Search time (1000 x {} queries): {:?}",
            queries.len(),
            live_search_time
        );
        println!("  Average per query: {:?}\n", live_per_query);
    } else {
        println!("  Live index not ready, skipping search benchmark\n");
        live_per_query = Duration::ZERO;
    }

    live_index.stop();

    // =========================================================================
    // Summary
    // =========================================================================
    println!("{}", "=".repeat(70));
    println!("SUMMARY");
    println!("{}", "=".repeat(70));
    println!("  Cold query (no cache):        {:>12?}", cold_per_query);
    println!("  Cache hit:                    {:>12?}", cache_per_query);
    println!(
        "  Instant prefetch access:      {:>12?}",
        instant_per_access
    );
    if live_per_query > Duration::ZERO {
        println!("  Live index search:            {:>12?}", live_per_query);
    }
    println!();
    println!("  Cache speedup vs cold:        {:>12.0}x", speedup);
    println!(
        "  Instant access speedup:       {:>12.0}x",
        cold_per_query.as_nanos() as f64 / instant_per_access.as_nanos() as f64
    );
    if live_per_query > Duration::ZERO {
        println!(
            "  Live index speedup vs cold:   {:>12.0}x",
            cold_per_query.as_nanos() as f64 / live_per_query.as_nanos() as f64
        );
    }
    println!();
    println!("  USE CASE: When user opens file search modal:");
    println!(
        "    - Show prefetched recent files INSTANTLY ({:?})",
        instant_per_access
    );
    if live_per_query > Duration::ZERO {
        println!(
            "    - Live index search: {:?} per query (in-memory filtering)",
            live_per_query
        );
    }
    println!("    - Fallback to cache: {:?} per query", cache_per_query);
    println!("{}\n", "=".repeat(70));
}
