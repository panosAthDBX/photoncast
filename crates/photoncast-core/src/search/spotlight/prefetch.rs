//! Background pre-fetching for Spotlight search.
//!
//! This module provides mechanisms to warm up the Spotlight search cache
//! before the user opens the file search modal, improving perceived performance.
//!
//! # Strategy
//!
//! 1. **Service initialization**: Create NSMetadataQuery objects (has setup overhead)
//! 2. **Recent files cache**: Pre-query files modified in last 7 days
//! 3. **Scope priming**: Run a minimal query in each primary scope
//!
//! # Safety
//!
//! - Queries run on a background thread
//! - Cancellation support via `CancellationToken`
//! - Throttling to avoid excessive resource usage
//! - Optional battery-awareness (skip on battery power)

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use super::result::SpotlightResult;
use super::service::{SpotlightSearchOptions, SpotlightSearchService};

/// Token for cancelling pre-fetch operations.
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a new cancellation token.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signals cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Returns true if cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for the prefetcher.
#[derive(Debug, Clone)]
pub struct PrefetchConfig {
    /// Whether to run pre-fetch on battery power.
    pub run_on_battery: bool,

    /// Delay before starting pre-fetch after trigger.
    pub initial_delay: Duration,

    /// How many recent files to pre-fetch.
    pub recent_files_limit: usize,

    /// How many days back to look for recent files.
    pub recent_files_days: u32,

    /// Timeout for each pre-fetch query.
    pub query_timeout: Duration,

    /// Minimum interval between pre-fetch runs.
    pub min_interval: Duration,
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            run_on_battery: false,
            initial_delay: Duration::from_millis(500),
            recent_files_limit: 50,
            recent_files_days: 7,
            query_timeout: Duration::from_secs(2),
            min_interval: Duration::from_secs(60),
        }
    }
}

/// Status of the prefetcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchStatus {
    /// Not started yet.
    Idle,
    /// Currently running pre-fetch queries.
    Running,
    /// Completed successfully.
    Completed,
    /// Was cancelled.
    Cancelled,
    /// Failed with an error.
    Failed,
}

/// Background prefetcher for Spotlight search.
///
/// This warms up the search cache before the user opens the file search modal.
pub struct SpotlightPrefetcher {
    service: Arc<SpotlightSearchService>,
    config: PrefetchConfig,
    status: Arc<Mutex<PrefetchStatus>>,
    last_run: Arc<AtomicU64>,
    current_token: Arc<Mutex<Option<CancellationToken>>>,
    /// Pre-fetched recent files (available immediately when modal opens).
    recent_files: Arc<Mutex<Vec<SpotlightResult>>>,
}

impl SpotlightPrefetcher {
    /// Creates a new prefetcher with the given service.
    pub fn new(service: Arc<SpotlightSearchService>) -> Self {
        Self::with_config(service, PrefetchConfig::default())
    }

    /// Creates a new prefetcher with custom configuration.
    pub fn with_config(service: Arc<SpotlightSearchService>, config: PrefetchConfig) -> Self {
        Self {
            service,
            config,
            status: Arc::new(Mutex::new(PrefetchStatus::Idle)),
            last_run: Arc::new(AtomicU64::new(0)),
            current_token: Arc::new(Mutex::new(None)),
            recent_files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns the current prefetch status.
    pub fn status(&self) -> PrefetchStatus {
        *self.status.lock()
    }

    /// Returns pre-fetched recent files (if available).
    ///
    /// This can be used to show instant results when the modal opens.
    pub fn get_recent_files(&self) -> Vec<SpotlightResult> {
        self.recent_files.lock().clone()
    }

    /// Checks if pre-fetch results are available.
    pub fn has_recent_files(&self) -> bool {
        !self.recent_files.lock().is_empty()
    }

    /// Triggers a background pre-fetch.
    ///
    /// This is safe to call multiple times - it will be throttled based on `min_interval`.
    /// Returns a cancellation token that can be used to stop the pre-fetch.
    pub fn trigger(&self) -> CancellationToken {
        // Check throttling
        let now = Instant::now();
        let last_run_ms = self.last_run.load(Ordering::SeqCst);
        let elapsed_since_last =
            Duration::from_millis(now.elapsed().as_millis() as u64 - last_run_ms);

        if last_run_ms > 0 && elapsed_since_last < self.config.min_interval {
            // Return existing token if running, or a dummy cancelled token
            let token_guard = self.current_token.lock();
            if let Some(ref token) = *token_guard {
                return token.clone();
            }
            let token = CancellationToken::new();
            token.cancel(); // Already throttled
            return token;
        }

        // Check battery if configured
        if !self.config.run_on_battery && is_on_battery() {
            let token = CancellationToken::new();
            token.cancel();
            return token;
        }

        // Cancel any existing pre-fetch
        self.cancel();

        // Create new token
        let token = CancellationToken::new();
        {
            let mut token_guard = self.current_token.lock();
            *token_guard = Some(token.clone());
        }

        // Update status
        {
            let mut status = self.status.lock();
            *status = PrefetchStatus::Running;
        }

        // Spawn background thread
        let service = Arc::clone(&self.service);
        let config = self.config.clone();
        let status = Arc::clone(&self.status);
        let last_run = Arc::clone(&self.last_run);
        let recent_files = Arc::clone(&self.recent_files);
        let token_clone = token.clone();

        thread::spawn(move || {
            // Initial delay
            thread::sleep(config.initial_delay);

            if token_clone.is_cancelled() {
                let mut s = status.lock();
                *s = PrefetchStatus::Cancelled;
                return;
            }

            // Run pre-fetch queries
            let success = run_prefetch_queries(&service, &config, &token_clone, &recent_files);

            // Update status
            {
                let mut s = status.lock();
                *s = if token_clone.is_cancelled() {
                    PrefetchStatus::Cancelled
                } else if success {
                    PrefetchStatus::Completed
                } else {
                    PrefetchStatus::Failed
                };
            }

            // Record completion time
            last_run.store(
                Instant::now().elapsed().as_millis() as u64,
                Ordering::SeqCst,
            );
        });

        token
    }

    /// Cancels any running pre-fetch operation.
    pub fn cancel(&self) {
        let token_guard = self.current_token.lock();
        if let Some(ref token) = *token_guard {
            token.cancel();
        }
    }

    /// Clears the pre-fetched cache.
    pub fn clear(&self) {
        self.recent_files.lock().clear();
        self.service.clear_cache();
    }
}

/// Runs the actual pre-fetch queries.
fn run_prefetch_queries(
    service: &SpotlightSearchService,
    config: &PrefetchConfig,
    token: &CancellationToken,
    recent_files: &Mutex<Vec<SpotlightResult>>,
) -> bool {
    // Query 1: Recent files using common single-letter queries
    // We query for vowels which appear in most filenames, sorted by recency
    if !token.is_cancelled() {
        let options = SpotlightSearchOptions {
            max_results: config.recent_files_limit,
            timeout: config.query_timeout,
            apply_exclusions: true,
            sort_by_recency: true,
            ..Default::default()
        };

        // Query for common letters that appear in most filenames
        // Using vowels and common consonants to get diverse results
        // Build a set of seen paths for O(1) dedup instead of O(n) linear scan
        let mut seen: HashSet<PathBuf> = {
            let files = recent_files.lock();
            files.iter().map(|f| f.path.clone()).collect()
        };

        for pattern in ["a", "e", "o", "s", "t", "n"] {
            if token.is_cancelled() {
                break;
            }
            if let Ok(results) = service.search_with_options(pattern, &options) {
                if !results.is_empty() {
                    let mut files = recent_files.lock();
                    // Merge results, avoiding duplicates via HashSet
                    for result in results {
                        if seen.insert(result.path.clone()) {
                            files.push(result);
                            if files.len() >= config.recent_files_limit {
                                break;
                            }
                        }
                    }
                    if files.len() >= config.recent_files_limit / 2 {
                        break; // Got enough results
                    }
                }
            }
        }
    }

    // Query 2: Warm up common search prefixes (populates cache)
    if !token.is_cancelled() {
        let warm_up_queries = ["doc", "down", "desk", "app", "pic"];
        let options = SpotlightSearchOptions {
            max_results: 5,
            timeout: Duration::from_millis(500),
            apply_exclusions: true,
            sort_by_recency: true,
            use_cache: true,
            ..Default::default()
        };

        for query in warm_up_queries {
            if token.is_cancelled() {
                break;
            }
            let _ = service.search_with_options(query, &options);
        }
    }

    !token.is_cancelled()
}

/// Checks if the system is running on battery power.
///
/// Returns false if unable to determine (assumes plugged in).
#[cfg(target_os = "macos")]
fn is_on_battery() -> bool {
    use std::process::Command;

    // Use pmset to check power source
    let output = Command::new("pmset").args(["-g", "batt"]).output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // If output contains "Battery Power", we're on battery
            stdout.contains("Battery Power")
        },
        Err(_) => false, // Assume plugged in if we can't check
    }
}

#[cfg(not(target_os = "macos"))]
fn is_on_battery() -> bool {
    false
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Creates a prefetcher and triggers an immediate background warm-up.
///
/// This is the simplest way to integrate prefetching into your app:
///
/// ```no_run
/// use photoncast_core::search::spotlight::prefetch::start_background_prefetch;
///
/// // Call during app initialization
/// let prefetcher = start_background_prefetch();
///
/// // Later, when file search modal opens:
/// let recent_files = prefetcher.get_recent_files();
/// ```
pub fn start_background_prefetch() -> Arc<SpotlightPrefetcher> {
    let service = Arc::new(SpotlightSearchService::new());
    let prefetcher = Arc::new(SpotlightPrefetcher::new(service));
    prefetcher.trigger();
    prefetcher
}

/// Creates a prefetcher with a shared service instance.
///
/// Use this when you already have a `SpotlightSearchService` and want
/// the prefetcher to share its cache.
pub fn start_background_prefetch_with_service(
    service: Arc<SpotlightSearchService>,
) -> Arc<SpotlightPrefetcher> {
    let prefetcher = Arc::new(SpotlightPrefetcher::new(service));
    prefetcher.trigger();
    prefetcher
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_clone() {
        let token1 = CancellationToken::new();
        let token2 = token1.clone();

        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        token1.cancel();

        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled()); // Clone should also see cancellation
    }

    #[test]
    fn test_prefetch_config_default() {
        let config = PrefetchConfig::default();
        assert!(!config.run_on_battery);
        assert_eq!(config.recent_files_limit, 50);
        assert_eq!(config.recent_files_days, 7);
    }

    #[test]
    fn test_prefetcher_creation() {
        let service = Arc::new(SpotlightSearchService::new());
        let prefetcher = SpotlightPrefetcher::new(service);
        assert_eq!(prefetcher.status(), PrefetchStatus::Idle);
        assert!(!prefetcher.has_recent_files());
    }

    #[test]
    fn test_prefetcher_cancel_before_start() {
        let service = Arc::new(SpotlightSearchService::new());
        let prefetcher = SpotlightPrefetcher::new(service);

        // Cancel before starting should be safe
        prefetcher.cancel();
        assert_eq!(prefetcher.status(), PrefetchStatus::Idle);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_is_on_battery() {
        // Just verify it doesn't crash
        let _ = is_on_battery();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_prefetcher_trigger_starts_running() {
        use std::thread;
        use std::time::Duration;

        let service = Arc::new(SpotlightSearchService::new());
        let config = PrefetchConfig {
            initial_delay: Duration::from_millis(50),
            query_timeout: Duration::from_millis(500), // Short timeout for tests
            min_interval: Duration::from_millis(100),
            ..Default::default()
        };
        let prefetcher = SpotlightPrefetcher::with_config(service, config);

        // Initially idle
        assert_eq!(prefetcher.status(), PrefetchStatus::Idle);

        // Trigger prefetch
        let token = prefetcher.trigger();
        assert!(!token.is_cancelled());

        // Wait a bit for it to start
        thread::sleep(Duration::from_millis(100));

        // Should be running now
        let status = prefetcher.status();
        assert!(
            status == PrefetchStatus::Running || status == PrefetchStatus::Completed,
            "Expected Running or Completed, got: {:?}",
            status
        );

        println!("Prefetch status after trigger: {:?}", status);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_prefetcher_cancellation() {
        use std::thread;
        use std::time::Duration;

        let service = Arc::new(SpotlightSearchService::new());
        let config = PrefetchConfig {
            initial_delay: Duration::from_millis(500), // Give time to cancel
            ..Default::default()
        };
        let prefetcher = SpotlightPrefetcher::with_config(service, config);

        // Trigger and immediately cancel
        let token = prefetcher.trigger();
        thread::sleep(Duration::from_millis(10));
        prefetcher.cancel();

        // Wait a bit
        thread::sleep(Duration::from_millis(600));

        // Should be cancelled
        assert!(token.is_cancelled());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_start_background_prefetch() {
        use std::thread;
        use std::time::Duration;

        let prefetcher = start_background_prefetch();

        // Should be running or completed
        let status = prefetcher.status();
        assert!(
            status == PrefetchStatus::Running || status == PrefetchStatus::Completed,
            "Unexpected initial status: {:?}",
            status
        );

        // Give it a moment
        thread::sleep(Duration::from_secs(3));

        println!("Final status: {:?}", prefetcher.status());
        println!("Recent files: {}", prefetcher.get_recent_files().len());
    }
}
