//! Filesystem scanning for applications.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use parking_lot::Mutex;
use tokio::time::timeout;
use tracing::{debug, info, trace, warn};

use crate::indexer::alias::{canonical_path, resolve_path};
use crate::indexer::metadata::parse_app_metadata;
use crate::indexer::IndexedApp;

/// Default paths to scan for applications.
pub const SCAN_PATHS: &[&str] = &["/Applications", "/System/Applications", "~/Applications"];

/// Patterns to exclude from scanning.
pub const EXCLUDED_PATTERNS: &[&str] = &["*.prefPane", "*Uninstaller*.app", "*.app/Contents/*"];

/// Default timeout for the full scan operation.
const SCAN_TIMEOUT: Duration = Duration::from_secs(10);

/// Number of concurrent app metadata parsing tasks.
const CONCURRENCY_LIMIT: usize = 20;

/// Scans the filesystem for installed applications.
#[derive(Debug, Clone)]
pub struct AppScanner {
    /// Paths to scan for applications.
    scan_paths: Vec<PathBuf>,
    /// Patterns to exclude.
    excluded_patterns: Vec<String>,
    /// Timeout for full scan.
    timeout: Duration,
}

impl Default for AppScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl AppScanner {
    /// Creates a new app scanner with default paths.
    #[must_use]
    pub fn new() -> Self {
        let scan_paths = SCAN_PATHS
            .iter()
            .map(|p| {
                if p.starts_with('~') {
                    dirs::home_dir()
                        .map(|h: PathBuf| h.join(&p[2..]))
                        .unwrap_or_else(|| PathBuf::from(p))
                } else {
                    PathBuf::from(p)
                }
            })
            .collect();

        Self {
            scan_paths,
            excluded_patterns: EXCLUDED_PATTERNS.iter().map(|s| (*s).to_string()).collect(),
            timeout: SCAN_TIMEOUT,
        }
    }

    /// Creates a new scanner with custom paths.
    #[must_use]
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            scan_paths: paths,
            excluded_patterns: EXCLUDED_PATTERNS.iter().map(|s| (*s).to_string()).collect(),
            timeout: SCAN_TIMEOUT,
        }
    }

    /// Sets a custom timeout for scanning.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Returns the scan paths.
    #[must_use]
    pub fn scan_paths(&self) -> &[PathBuf] {
        &self.scan_paths
    }

    /// Scans all configured paths for applications.
    ///
    /// This method scans `/Applications`, `/System/Applications`, and `~/Applications`
    /// concurrently and returns all discovered applications within the configured timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the scan times out or encounters critical filesystem errors.
    pub async fn scan_all(&self) -> Result<Vec<IndexedApp>> {
        let start = std::time::Instant::now();
        info!("Starting full application scan");

        let result = timeout(self.timeout, self.scan_all_internal()).await;

        match result {
            Ok(apps) => {
                let apps = apps?;
                info!(
                    "Completed scan in {:?}, found {} applications",
                    start.elapsed(),
                    apps.len()
                );
                Ok(apps)
            },
            Err(_) => {
                warn!("Scan timed out after {:?}", self.timeout);
                anyhow::bail!("Application scan timed out after {:?}", self.timeout);
            },
        }
    }

    /// Internal scanning logic without timeout wrapper.
    async fn scan_all_internal(&self) -> Result<Vec<IndexedApp>> {
        // Track seen canonical paths to prevent duplicates
        let seen_paths: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

        // Collect app bundles from all directories concurrently
        let dir_results: Vec<Result<Vec<PathBuf>>> = stream::iter(&self.scan_paths)
            .map(|path| self.find_app_bundles(path, Arc::clone(&seen_paths)))
            .buffer_unordered(self.scan_paths.len())
            .collect()
            .await;

        // Flatten all app bundle paths
        let mut all_bundles: Vec<PathBuf> = Vec::new();
        for result in dir_results {
            match result {
                Ok(bundles) => all_bundles.extend(bundles),
                Err(e) => {
                    // Log warning but continue with other directories
                    warn!("Failed to scan directory: {}", e);
                },
            }
        }

        debug!("Found {} app bundles to parse", all_bundles.len());

        // Parse metadata for all apps concurrently
        let apps: Vec<IndexedApp> = stream::iter(all_bundles)
            .map(|path| async move {
                match parse_app_metadata(&path).await {
                    Ok(app) => {
                        debug!("Parsed app: {} ({})", app.name, app.bundle_id);
                        Some(app)
                    },
                    Err(e) => {
                        warn!("Failed to parse app at {}: {}", path.display(), e);
                        None
                    },
                }
            })
            .buffer_unordered(CONCURRENCY_LIMIT)
            .filter_map(|opt| async move { opt })
            .collect()
            .await;

        Ok(apps)
    }

    /// Finds all .app bundles in a directory.
    ///
    /// This method handles:
    /// - Direct .app bundles
    /// - Unix symlinks to .app bundles
    /// - macOS Finder aliases to .app bundles
    ///
    /// Uses `seen_paths` to deduplicate apps that are linked from multiple locations.
    async fn find_app_bundles(
        &self,
        path: &Path,
        seen_paths: Arc<Mutex<HashSet<PathBuf>>>,
    ) -> Result<Vec<PathBuf>> {
        if !path.exists() {
            debug!("Scan path does not exist: {}", path.display());
            return Ok(Vec::new());
        }

        let mut bundles = Vec::new();
        let mut entries = tokio::fs::read_dir(path)
            .await
            .with_context(|| format!("Failed to read directory: {}", path.display()))?;

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();

            // Resolve symlinks and aliases to get the actual app path
            let resolved = match self.resolve_app_path(&entry_path) {
                Some(p) => p,
                None => continue,
            };

            // Check if it's an .app bundle (after resolution)
            if !Self::is_app_bundle(&resolved) {
                continue;
            }

            // Check exclusion patterns (on resolved path)
            if self.is_excluded(&resolved) {
                debug!("Excluding app: {}", resolved.display());
                continue;
            }

            // Get canonical path for deduplication
            let canonical = match canonical_path(&resolved) {
                Ok(p) => p,
                Err(e) => {
                    trace!(
                        "Could not canonicalize {}: {} - using resolved path",
                        resolved.display(),
                        e
                    );
                    resolved.clone()
                },
            };

            // Check for duplicates using canonical path
            {
                let mut seen = seen_paths.lock();
                if seen.contains(&canonical) {
                    debug!(
                        "Skipping duplicate app: {} (canonical: {})",
                        entry_path.display(),
                        canonical.display()
                    );
                    continue;
                }
                seen.insert(canonical);
            }

            bundles.push(resolved);
        }

        debug!("Found {} apps in {}", bundles.len(), path.display());

        Ok(bundles)
    }

    /// Resolves symlinks and aliases to get the actual app path.
    ///
    /// Returns `None` if resolution fails or the target doesn't exist.
    fn resolve_app_path(&self, path: &Path) -> Option<PathBuf> {
        match resolve_path(path) {
            Ok(resolved) => {
                let target = &resolved.target;

                // Verify the target exists
                if !target.exists() {
                    debug!(
                        "Resolved path does not exist: {} -> {}",
                        path.display(),
                        target.display()
                    );
                    return None;
                }

                if resolved.was_alias {
                    debug!(
                        "Resolved alias/symlink: {} -> {}",
                        path.display(),
                        target.display()
                    );
                }

                Some(target.clone())
            },
            Err(e) => {
                // Resolution failed - try using the original path if it exists
                if path.exists() {
                    trace!(
                        "Alias resolution failed for {}: {} - using original",
                        path.display(),
                        e
                    );
                    Some(path.to_path_buf())
                } else {
                    debug!(
                        "Could not resolve path and original doesn't exist: {}",
                        path.display()
                    );
                    None
                }
            },
        }
    }

    /// Scans a single directory for applications.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read.
    pub async fn scan_directory(&self, path: &Path) -> Result<Vec<IndexedApp>> {
        let seen_paths = Arc::new(Mutex::new(HashSet::new()));
        let bundles = self.find_app_bundles(path, seen_paths).await?;

        let apps: Vec<IndexedApp> = stream::iter(bundles)
            .map(|bundle_path| async move { parse_app_metadata(&bundle_path).await.ok() })
            .buffer_unordered(CONCURRENCY_LIMIT)
            .filter_map(|opt| async move { opt })
            .collect()
            .await;

        Ok(apps)
    }

    /// Returns true if a path should be excluded.
    #[must_use]
    pub fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.excluded_patterns {
            if Self::matches_glob_pattern(&path_str, pattern) {
                return true;
            }
        }

        false
    }

    /// Checks if a path matches a glob pattern.
    /// Supports `*` wildcard at start, end, or middle of pattern.
    fn matches_glob_pattern(path: &str, pattern: &str) -> bool {
        // Split pattern by * and match each part
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            // No wildcards - exact match
            return path == pattern;
        }

        // Handle patterns like *middle*.suffix (e.g., *Uninstaller*.app)
        if parts.len() == 3 && parts[0].is_empty() {
            // Pattern is *middle*suffix - must contain middle and end with suffix
            let middle = parts[1];
            let suffix = parts[2];

            if suffix.is_empty() {
                // Pattern is *middle* - just contains
                return path.contains(middle);
            }
            // Pattern is *middle*suffix - contains middle and ends with suffix
            return path.contains(middle) && path.ends_with(suffix);
        }

        // Handle *.ext pattern (ends with)
        if parts.len() == 2 && parts[0].is_empty() {
            return path.ends_with(parts[1]);
        }

        // Handle prefix* pattern (starts with)
        if parts.len() == 2 && parts[1].is_empty() {
            return path.starts_with(parts[0]);
        }

        // Handle pre*suf pattern (starts with prefix, ends with suffix)
        if parts.len() == 2 {
            return path.starts_with(parts[0]) && path.ends_with(parts[1]);
        }

        // For more complex patterns with multiple wildcards, do a simple contains check
        // on all non-empty parts
        let mut remaining = path;
        for part in &parts {
            if part.is_empty() {
                continue;
            }
            if let Some(pos) = remaining.find(part) {
                remaining = &remaining[pos + part.len()..];
            } else {
                return false;
            }
        }

        true
    }

    /// Checks if a path is an app bundle (ends with .app and is a directory).
    fn is_app_bundle(path: &Path) -> bool {
        path.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
            && path.is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = AppScanner::new();
        assert!(!scanner.scan_paths.is_empty());
        assert!(!scanner.excluded_patterns.is_empty());
    }

    #[test]
    fn test_scanner_with_custom_paths() {
        let paths = vec![PathBuf::from("/test/apps")];
        let scanner = AppScanner::with_paths(paths.clone());
        assert_eq!(scanner.scan_paths, paths);
    }

    #[test]
    fn test_exclude_prefpane() {
        let scanner = AppScanner::new();
        assert!(scanner.is_excluded(Path::new("/Library/PreferencePanes/Test.prefPane")));
    }

    #[test]
    fn test_exclude_uninstaller() {
        let scanner = AppScanner::new();
        assert!(scanner.is_excluded(Path::new("/Applications/SomeApp Uninstaller.app")));
    }

    #[test]
    fn test_exclude_nested_app() {
        let scanner = AppScanner::new();
        assert!(scanner.is_excluded(Path::new(
            "/Applications/Parent.app/Contents/MacOS/Nested.app"
        )));
    }

    #[test]
    fn test_allow_regular_app() {
        let scanner = AppScanner::new();
        assert!(!scanner.is_excluded(Path::new("/Applications/Safari.app")));
        assert!(!scanner.is_excluded(Path::new("/System/Applications/Calculator.app")));
    }

    #[test]
    fn test_glob_pattern_ends_with() {
        // *.prefPane should match anything ending with .prefPane
        assert!(AppScanner::matches_glob_pattern(
            "/Library/PreferencePanes/Test.prefPane",
            "*.prefPane"
        ));
        assert!(!AppScanner::matches_glob_pattern(
            "/Applications/Test.app",
            "*.prefPane"
        ));
    }

    #[test]
    fn test_glob_pattern_contains() {
        // *Uninstaller*.app should match anything containing "Uninstaller" ending with .app
        assert!(AppScanner::matches_glob_pattern(
            "/Applications/SomeApp Uninstaller.app",
            "*Uninstaller*.app"
        ));
        assert!(AppScanner::matches_glob_pattern(
            "/Applications/Uninstaller for App.app",
            "*Uninstaller*.app"
        ));
        assert!(!AppScanner::matches_glob_pattern(
            "/Applications/Safari.app",
            "*Uninstaller*.app"
        ));
    }

    #[test]
    fn test_glob_pattern_nested_contents() {
        // *.app/Contents/* should match anything inside an app bundle's Contents
        assert!(AppScanner::matches_glob_pattern(
            "/Applications/Test.app/Contents/MacOS/binary",
            "*.app/Contents/*"
        ));
    }

    #[test]
    fn test_scan_paths_expand_tilde() {
        let scanner = AppScanner::new();
        // ~/Applications should expand to actual home directory
        for path in &scanner.scan_paths {
            assert!(
                !path.to_string_lossy().starts_with('~'),
                "Path should not start with ~: {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_timeout_builder() {
        let scanner = AppScanner::new().with_timeout(Duration::from_secs(5));
        assert_eq!(scanner.timeout, Duration::from_secs(5));
    }
}
