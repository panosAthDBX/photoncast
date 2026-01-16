# Testing Standards for Rust

## Overview

Rust has excellent built-in testing support. Write tests at multiple levels: unit tests for functions, integration tests for modules, and property tests for invariants.

## When to Apply

- All public APIs must have tests
- Complex private functions should have tests
- Bug fixes should include regression tests
- Performance-critical code needs benchmarks

## Core Principles

1. **Test behavior, not implementation** - Tests should survive refactoring
2. **One assertion per test (ideally)** - Clear failure messages
3. **Use test fixtures** - Don't repeat setup code
4. **Property-based testing** - For algorithmic code
5. **Benchmark critical paths** - For performance-sensitive code

## ✅ DO

### DO: Use Built-in Test Framework

**✅ DO**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fuzzy_match_exact() {
        let matcher = FuzzyMatcher::new();
        let score = matcher.score("firefox", "firefox");
        assert!(score.is_some());
        assert!(score.unwrap() > 100);
    }
    
    #[test]
    fn test_fuzzy_match_partial() {
        let matcher = FuzzyMatcher::new();
        let score = matcher.score("ff", "firefox");
        assert!(score.is_some());
    }
    
    #[test]
    fn test_fuzzy_match_no_match() {
        let matcher = FuzzyMatcher::new();
        let score = matcher.score("xyz", "firefox");
        assert!(score.is_none());
    }
}
```

### DO: Use Descriptive Test Names

**✅ DO**:
```rust
#[test]
fn search_returns_empty_for_no_matches() { /* ... */ }

#[test]
fn search_returns_results_sorted_by_score_descending() { /* ... */ }

#[test]
fn search_limits_results_to_max_count() { /* ... */ }

#[test]
fn search_handles_unicode_queries() { /* ... */ }

#[test]
#[should_panic(expected = "query cannot be empty")]
fn search_panics_on_empty_query() { /* ... */ }
```

### DO: Test Error Cases

**✅ DO**:
```rust
#[test]
fn load_config_returns_error_for_missing_file() {
    let result = load_config("/nonexistent/path");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn load_config_returns_error_for_invalid_toml() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), "invalid { toml").unwrap();
    
    let result = load_config(temp.path());
    assert!(matches!(result, Err(ConfigError::ParseError(_))));
}
```

### DO: Use Test Fixtures and Helpers

**✅ DO**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test fixture
    fn sample_applications() -> Vec<Application> {
        vec![
            Application {
                name: "Firefox".into(),
                bundle_id: "org.mozilla.firefox".into(),
                path: "/Applications/Firefox.app".into(),
            },
            Application {
                name: "Safari".into(),
                bundle_id: "com.apple.Safari".into(),
                path: "/Applications/Safari.app".into(),
            },
        ]
    }
    
    // Helper function
    fn create_test_index() -> SearchIndex {
        let apps = sample_applications();
        SearchIndex::from_applications(&apps)
    }
    
    #[test]
    fn test_search_finds_app() {
        let index = create_test_index();
        let results = index.search("fire");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Firefox");
    }
}
```

### DO: Use Property-Based Testing for Algorithms

**✅ DO**:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzzy_match_score_is_deterministic(
        query in "[a-z]{1,10}",
        target in "[a-z]{1,20}"
    ) {
        let matcher = FuzzyMatcher::new();
        let score1 = matcher.score(&query, &target);
        let score2 = matcher.score(&query, &target);
        prop_assert_eq!(score1, score2);
    }
    
    #[test]
    fn exact_match_has_highest_score(target in "[a-z]{3,10}") {
        let matcher = FuzzyMatcher::new();
        let exact_score = matcher.score(&target, &target).unwrap();
        
        // Any prefix should have lower score
        let prefix = &target[..target.len()-1];
        let prefix_score = matcher.score(prefix, &target).unwrap_or(0);
        
        prop_assert!(exact_score >= prefix_score);
    }
    
    #[test]
    fn search_results_never_exceed_limit(
        query in "[a-z]{1,5}",
        limit in 1usize..100
    ) {
        let index = create_large_test_index();
        let results = index.search_with_limit(&query, limit);
        prop_assert!(results.len() <= limit);
    }
}
```

### DO: Use Async Test Support

**✅ DO**:
```rust
#[tokio::test]
async fn test_async_search() {
    let indexer = Indexer::new();
    indexer.index_directory("/Applications").await.unwrap();
    
    let results = indexer.search("firefox").await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_indexing() {
    let indexer = Arc::new(Indexer::new());
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let indexer = indexer.clone();
            tokio::spawn(async move {
                indexer.index_item(i).await
            })
        })
        .collect();
    
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
    
    assert_eq!(indexer.count().await, 10);
}
```

### DO: Write Integration Tests

**✅ DO**:
```rust
// tests/integration/search_test.rs

use photoncast::{Config, SearchEngine};

#[tokio::test]
async fn full_search_workflow() {
    // Setup
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Config {
        data_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    
    let engine = SearchEngine::new(config).await.unwrap();
    
    // Index
    engine.index_applications().await.unwrap();
    
    // Wait for indexing
    engine.wait_for_ready().await;
    
    // Search
    let results = engine.search("safari").await.unwrap();
    
    // Verify
    assert!(!results.is_empty());
    assert!(results[0].name.to_lowercase().contains("safari"));
}
```

### DO: Use Benchmarks for Performance-Critical Code

**✅ DO**:
```rust
// benches/search_bench.rs

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use photoncast::FuzzyMatcher;

fn bench_fuzzy_match(c: &mut Criterion) {
    let matcher = FuzzyMatcher::new();
    let targets: Vec<String> = (0..1000)
        .map(|i| format!("application_{}", i))
        .collect();
    
    c.bench_function("fuzzy_match_single", |b| {
        b.iter(|| matcher.score("app", "application"))
    });
    
    let mut group = c.benchmark_group("fuzzy_match_batch");
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                b.iter(|| {
                    targets[..size]
                        .iter()
                        .filter_map(|t| matcher.score("app", t))
                        .collect::<Vec<_>>()
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_fuzzy_match);
criterion_main!(benches);
```

## ❌ DON'T

### DON'T: Test Implementation Details

**❌ DON'T**:
```rust
#[test]
fn test_internal_cache_structure() {
    let searcher = Searcher::new();
    searcher.search("test");
    
    // Don't test internal implementation
    assert_eq!(searcher.cache.len(), 1);
    assert_eq!(searcher.cache.get("test"), Some(&cached_result));
}
```
**Why**: This test will break if you change the cache implementation.

**✅ DO**:
```rust
#[test]
fn test_repeated_search_returns_same_results() {
    let searcher = Searcher::new();
    let results1 = searcher.search("test");
    let results2 = searcher.search("test");
    assert_eq!(results1, results2);
}
```

### DON'T: Use #[ignore] Without Reason

**❌ DON'T**:
```rust
#[test]
#[ignore]  // Why?
fn test_something() { /* ... */ }
```

**✅ DO**:
```rust
#[test]
#[ignore = "requires network access, run with --ignored"]
fn test_remote_api() { /* ... */ }

#[test]
#[ignore = "slow test, run in CI only"]
fn test_full_index_rebuild() { /* ... */ }
```

### DON'T: Have Flaky Tests

**❌ DON'T**:
```rust
#[test]
fn test_with_timing() {
    let start = Instant::now();
    do_something();
    // Flaky: depends on system load
    assert!(start.elapsed() < Duration::from_millis(100));
}

#[test]
fn test_with_random() {
    let result = random_operation();
    // Flaky: random result
    assert!(result > 50);
}
```

**✅ DO**:
```rust
#[test]
fn test_operation_completes() {
    // Just test it completes, benchmark separately
    do_something();
}

#[test]
fn test_with_seeded_random() {
    let mut rng = StdRng::seed_from_u64(42);  // Reproducible
    let result = operation_with_rng(&mut rng);
    assert_eq!(result, expected);
}
```

### DON'T: Write Tests That Depend on Order

**❌ DON'T**:
```rust
static mut GLOBAL_STATE: i32 = 0;

#[test]
fn test_a_sets_state() {
    unsafe { GLOBAL_STATE = 1; }
}

#[test]
fn test_b_uses_state() {
    // Depends on test_a running first!
    unsafe { assert_eq!(GLOBAL_STATE, 1); }
}
```

**✅ DO**:
```rust
#[test]
fn test_independent_a() {
    let state = create_state();
    state.set(1);
    assert_eq!(state.get(), 1);
}

#[test]
fn test_independent_b() {
    let state = create_state();
    state.set(2);
    assert_eq!(state.get(), 2);
}
```

### DON'T: Have Multiple Assertions Without Clear Context

**❌ DON'T**:
```rust
#[test]
fn test_search() {
    let results = search("test");
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "Test App");
    assert_eq!(results[0].score, 100);
    assert!(results[0].path.exists());
    assert_eq!(results.len(), 1);
    // Which assertion failed?
}
```

**✅ DO**:
```rust
#[test]
fn test_search_returns_results() {
    let results = search("test");
    assert!(!results.is_empty(), "search should return at least one result");
}

#[test]
fn test_search_result_has_correct_name() {
    let results = search("test");
    assert_eq!(results[0].name, "Test App");
}

#[test]
fn test_search_result_has_valid_path() {
    let results = search("test");
    assert!(results[0].path.exists(), "result path should exist");
}
```

## Test Organization

### Directory Structure
```
src/
├── lib.rs
├── search/
│   ├── mod.rs
│   ├── fuzzy.rs      # Unit tests in same file
│   └── indexer.rs
tests/                 # Integration tests
├── common/
│   └── mod.rs        # Shared test utilities
├── search_integration.rs
└── e2e.rs
benches/              # Benchmarks
├── search_bench.rs
└── index_bench.rs
```

### Test Utilities Module
```rust
// tests/common/mod.rs

use photoncast::*;
use tempfile::TempDir;

pub struct TestContext {
    pub temp_dir: TempDir,
    pub config: Config,
    pub engine: SearchEngine,
}

impl TestContext {
    pub async fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let engine = SearchEngine::new(config.clone()).await.unwrap();
        
        Self { temp_dir, config, engine }
    }
    
    pub fn create_test_app(&self, name: &str) -> PathBuf {
        let app_path = self.temp_dir.path().join(format!("{}.app", name));
        std::fs::create_dir_all(&app_path).unwrap();
        // Create minimal app bundle structure
        app_path
    }
}
```

## CI Configuration

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Run tests
        run: cargo test --all-features
        
      - name: Run ignored tests
        run: cargo test --all-features -- --ignored
        
      - name: Run benchmarks (dry run)
        run: cargo bench --no-run

  coverage:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-llvm-cov
      
      - name: Generate coverage
        run: cargo llvm-cov --all-features --lcov --output-path lcov.info
```

## Resources

- [Rust Book: Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [proptest](https://docs.rs/proptest/latest/proptest/)
- [criterion](https://docs.rs/criterion/latest/criterion/)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
