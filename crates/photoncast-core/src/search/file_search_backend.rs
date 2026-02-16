//! Unified file search backend abstraction.
//!
//! Provides a single entry point for file search with configurable
//! Spotlight-first or filesystem-only strategy and explicit fallback policy.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use tracing::{info, trace, warn};

use crate::platform::spotlight::{FileResult, SpotlightProvider, SpotlightQuery};

/// Search strategy used by [`FileSearchBackend`].
#[derive(Debug, Clone)]
pub enum FileSearchStrategy {
    /// Use Spotlight first and fallback to filesystem walk on timeout or explicit zero-result condition.
    SpotlightWithFallback {
        /// Maximum Spotlight wait before fallback.
        timeout: Duration,
    },
    /// Bypass Spotlight entirely and search filesystem directly.
    FilesystemOnly,
}

/// Backend configuration options.
#[derive(Debug, Clone)]
pub struct FileSearchBackendOptions {
    /// Search root for Spotlight scope and filesystem fallback.
    pub root: PathBuf,
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Optional "recent files" lookback window (days) for date-based searches.
    pub recent_days: Option<u32>,
}

impl Default for FileSearchBackendOptions {
    fn default() -> Self {
        Self {
            root: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            max_results: 50,
            recent_days: None,
        }
    }
}

/// Unified file search backend.
#[derive(Clone, Debug)]
pub struct FileSearchBackend {
    strategy: FileSearchStrategy,
    spotlight: SpotlightProvider,
}

impl FileSearchBackend {
    /// Creates a backend with the requested strategy.
    #[must_use]
    pub fn new(strategy: FileSearchStrategy) -> Self {
        Self {
            strategy,
            spotlight: SpotlightProvider::new(),
        }
    }

    /// Creates a backend with an injected Spotlight provider.
    #[must_use]
    pub fn with_spotlight_provider(
        strategy: FileSearchStrategy,
        spotlight: SpotlightProvider,
    ) -> Self {
        Self {
            strategy,
            spotlight,
        }
    }

    /// Runs a filename query according to strategy and fallback policy.
    #[must_use]
    pub fn search_sync(&self, query: &str, options: &FileSearchBackendOptions) -> Vec<FileResult> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }

        let started = Instant::now();

        let (results, fallback_triggered) = match &self.strategy {
            FileSearchStrategy::FilesystemOnly => (filesystem_search(query, options), false),
            FileSearchStrategy::SpotlightWithFallback { timeout } => {
                match spotlight_search_with_timeout(&self.spotlight, query, options, *timeout) {
                    SpotlightAttempt::Success(results) if !results.is_empty() => {
                        trace!(
                            component = "search",
                            operation = "fallback_check",
                            reason = "spotlight_partial_or_full_results",
                            fallback = false,
                            result_count = results.len(),
                            "spotlight returned non-empty results; fallback skipped"
                        );
                        (results, false)
                    },
                    SpotlightAttempt::Success(_) => {
                        info!(
                            component = "search",
                            operation = "fallback",
                            reason = "spotlight_zero_results",
                            fallback = true,
                            "spotlight returned zero results, falling back to filesystem"
                        );
                        (filesystem_search(query, options), true)
                    },
                    SpotlightAttempt::TimedOut => {
                        info!(
                            component = "search",
                            operation = "fallback",
                            reason = "spotlight_timeout",
                            fallback = true,
                            "spotlight timed out, falling back to filesystem"
                        );
                        (filesystem_search(query, options), true)
                    },
                    SpotlightAttempt::Failed(err) => {
                        warn!(
                            component = "search",
                            operation = "spotlight_error",
                            error = %err,
                            fallback = false,
                            "spotlight error without fallback"
                        );
                        (Vec::new(), false)
                    },
                }
            },
        };

        trace!(
            component = "search",
            operation = "file_search_backend",
            strategy = ?self.strategy,
            fallback = fallback_triggered,
            elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
            result_count = results.len(),
            "file search backend completed"
        );

        results
    }

    /// Async wrapper around [`Self::search_sync`].
    pub async fn search(&self, query: &str, options: &FileSearchBackendOptions) -> Vec<FileResult> {
        let backend = self.clone();
        let query = query.to_string();
        let options = options.clone();

        tokio::task::spawn_blocking(move || backend.search_sync(&query, &options))
            .await
            .unwrap_or_default()
    }

    /// Returns recent files modified within the given lookback window.
    #[must_use]
    pub fn search_recent_sync(
        &self,
        options: &FileSearchBackendOptions,
        days: u32,
    ) -> Vec<FileResult> {
        let started = Instant::now();

        let query = format!("kMDItemFSContentChangeDate >= $time.today(-{days})");
        let mut spotlight_query = SpotlightQuery::new(query)
            .with_max_results(options.max_results)
            .with_scope(options.root.clone());

        let timeout_ms = match &self.strategy {
            FileSearchStrategy::SpotlightWithFallback { timeout } => {
                u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX)
            },
            FileSearchStrategy::FilesystemOnly => self.spotlight.timeout_ms,
        };
        spotlight_query = spotlight_query.with_timeout_ms(timeout_ms);

        let mut results = match spotlight_query.execute_sync() {
            Ok(results) => results,
            Err(err) => {
                warn!(
                    component = "search",
                    operation = "recent_files_search",
                    error = %err,
                    fallback = false,
                    "recent files spotlight query failed"
                );
                Vec::new()
            },
        };

        let now = SystemTime::now();
        let max_age = Duration::from_secs(u64::from(days) * 24 * 60 * 60);
        results.retain(|file| {
            file.modified
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age <= max_age)
        });

        if results.len() > options.max_results {
            results.truncate(options.max_results);
        }

        trace!(
            component = "search",
            operation = "file_search_backend_recent",
            strategy = ?self.strategy,
            fallback = false,
            elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
            result_count = results.len(),
            "recent file search backend completed"
        );

        results
    }

    /// Async wrapper around [`Self::search_recent_sync`].
    pub async fn search_recent(
        &self,
        options: &FileSearchBackendOptions,
        days: u32,
    ) -> Vec<FileResult> {
        let backend = self.clone();
        let options = options.clone();
        tokio::task::spawn_blocking(move || backend.search_recent_sync(&options, days))
            .await
            .unwrap_or_default()
    }
}

enum SpotlightAttempt {
    Success(Vec<FileResult>),
    TimedOut,
    Failed(crate::platform::spotlight::SpotlightError),
}

fn spotlight_search_with_timeout(
    spotlight: &SpotlightProvider,
    query: &str,
    options: &FileSearchBackendOptions,
    timeout: Duration,
) -> SpotlightAttempt {
    let mut provider = spotlight.clone();
    provider.max_results = options.max_results;
    provider.search_scope = Some(options.root.clone());

    let (tx, rx) = mpsc::channel();
    let query = query.to_string();

    thread::spawn(move || {
        let _ = tx.send(provider.search_sync(&query));
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(results)) => SpotlightAttempt::Success(results),
        Ok(Err(err)) => SpotlightAttempt::Failed(err),
        Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
            SpotlightAttempt::TimedOut
        },
    }
}

fn filesystem_search(query: &str, options: &FileSearchBackendOptions) -> Vec<FileResult> {
    let query = query.to_lowercase();
    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(&options.root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if results.len() >= options.max_results {
            break;
        }

        let path = entry.path();
        if should_skip(path, entry.file_type().is_dir()) {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if !name.to_lowercase().contains(&query) {
            continue;
        }

        if !matches_interesting_extension(path) {
            continue;
        }

        let mut result = FileResult::from_path(path.to_path_buf());
        result.load_metadata();
        results.push(result);
    }

    results
}

fn should_skip(path: &Path, is_dir: bool) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };

    if name.starts_with('.') {
        return true;
    }

    if is_dir
        && matches!(
            name,
            "node_modules"
                | "target"
                | "build"
                | "dist"
                | "__pycache__"
                | ".git"
                | ".svn"
                | ".hg"
                | "Caches"
                | "Library"
        )
    {
        return true;
    }

    false
}

fn matches_interesting_extension(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &[
        "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "rtf", "odt", "pages",
        "numbers", "key", "jpg", "jpeg", "png", "gif", "bmp", "svg", "webp", "ico", "tiff", "heic",
        "raw", "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mp3", "wav", "flac",
        "aac", "ogg", "m4a", "wma", "aiff", "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "dmg",
        "iso", "epub", "mobi", "azw", "azw3", "ibooks", "app",
    ];

    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .is_some_and(|ext| EXTENSIONS.contains(&ext.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn filesystem_only_strategy_returns_matches() {
        let dir = TempDir::new().expect("tempdir");
        let file = dir.path().join("annual-report.pdf");
        std::fs::write(&file, b"x").expect("write");

        let backend = FileSearchBackend::new(FileSearchStrategy::FilesystemOnly);
        let options = FileSearchBackendOptions {
            root: dir.path().to_path_buf(),
            max_results: 10,
            recent_days: None,
        };

        let results = backend.search_sync("annual", &options);
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.path == file));
    }

    #[test]
    fn timeout_triggers_fallback() {
        let dir = TempDir::new().expect("tempdir");
        let file = dir.path().join("fallback-target.txt");
        std::fs::write(&file, b"x").expect("write");

        let backend = FileSearchBackend::new(FileSearchStrategy::SpotlightWithFallback {
            timeout: Duration::from_nanos(1),
        });

        let options = FileSearchBackendOptions {
            root: dir.path().to_path_buf(),
            max_results: 10,
            recent_days: None,
        };

        let results = backend.search_sync("fallback", &options);
        assert!(results.iter().any(|r| r.path == file));
    }

    #[tokio::test]
    async fn async_wrapper_matches_sync_results() {
        let dir = TempDir::new().expect("tempdir");
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, b"x").expect("write");

        let backend = FileSearchBackend::new(FileSearchStrategy::FilesystemOnly);
        let options = FileSearchBackendOptions {
            root: dir.path().to_path_buf(),
            max_results: 10,
            recent_days: None,
        };

        let sync_results = backend.search_sync("notes", &options);
        let async_results = backend.search("notes", &options).await;

        assert_eq!(sync_results.len(), async_results.len());
        assert!(async_results.iter().any(|r| r.path == file));
    }
}
