//! Versioned dylib cache for hot-reload support.
//!
//! This module manages versioned copies of extension dylibs to bypass OS-level
//! library caching. When reloading an extension, we copy the dylib to a new
//! timestamped path so the OS loads the fresh version.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

/// Errors that can occur during dylib cache operations.
#[derive(Debug, Error)]
pub enum DylibCacheError {
    #[error("failed to create cache directory: {0}")]
    CreateDir(std::io::Error),
    #[error("failed to copy dylib: {0}")]
    Copy(std::io::Error),
    #[error("failed to read directory: {0}")]
    ReadDir(std::io::Error),
    #[error("failed to remove old version: {0}")]
    Remove(std::io::Error),
    #[error("source dylib not found: {path}")]
    SourceNotFound { path: PathBuf },
}

/// Manages versioned copies of extension dylibs.
pub struct DylibCache {
    cache_root: PathBuf,
    max_versions: usize,
}

impl DylibCache {
    /// Creates a new dylib cache with the specified root directory.
    ///
    /// # Arguments
    ///
    /// * `cache_root` - The root directory for cached dylibs (e.g., `~/.cache/photoncast/extensions`).
    #[must_use]
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            cache_root,
            max_versions: 3,
        }
    }

    /// Sets the maximum number of cached versions to keep per extension.
    #[must_use]
    pub fn with_max_versions(mut self, max: usize) -> Self {
        self.max_versions = max;
        self
    }

    /// Creates a versioned copy of the dylib for the given extension.
    ///
    /// Returns the path to the cached copy which should be loaded instead of the original.
    ///
    /// # Arguments
    ///
    /// * `extension_id` - The unique identifier of the extension.
    /// * `source_dylib` - Path to the original dylib file.
    ///
    /// # Errors
    ///
    /// Returns an error if the copy operation fails.
    pub fn create_versioned_copy(
        &self,
        extension_id: &str,
        source_dylib: &Path,
    ) -> Result<PathBuf, DylibCacheError> {
        if !source_dylib.exists() {
            return Err(DylibCacheError::SourceNotFound {
                path: source_dylib.to_path_buf(),
            });
        }

        let ext_cache_dir = self.extension_cache_dir(extension_id);
        fs::create_dir_all(&ext_cache_dir).map_err(DylibCacheError::CreateDir)?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());

        let file_name = source_dylib
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("extension.dylib");

        // Remove extension and add timestamp
        let stem = Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extension");
        let extension = Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("dylib");

        let versioned_name = format!("{stem}_{timestamp}.{extension}");
        let dest_path = ext_cache_dir.join(versioned_name);

        fs::copy(source_dylib, &dest_path).map_err(DylibCacheError::Copy)?;

        tracing::debug!(
            extension_id = extension_id,
            source = %source_dylib.display(),
            dest = %dest_path.display(),
            "Created versioned dylib copy"
        );

        Ok(dest_path)
    }

    /// Cleans up old versions for an extension, keeping only the most recent ones.
    ///
    /// # Arguments
    ///
    /// * `extension_id` - The unique identifier of the extension.
    /// * `current_version` - The path to the currently loaded version (will not be deleted).
    ///
    /// # Errors
    ///
    /// Returns an error if cleanup fails. Partial failures are logged but not propagated.
    pub fn cleanup_old_versions(
        &self,
        extension_id: &str,
        current_version: Option<&Path>,
    ) -> Result<usize, DylibCacheError> {
        let ext_cache_dir = self.extension_cache_dir(extension_id);
        if !ext_cache_dir.exists() {
            return Ok(0);
        }

        let mut versions: Vec<PathBuf> = fs::read_dir(&ext_cache_dir)
            .map_err(DylibCacheError::ReadDir)?
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|e| e == "dylib" || e == "so" || e == "dll")
            })
            .collect();

        // Sort by modification time (newest first)
        versions.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        let mut removed = 0;
        for (i, version) in versions.iter().enumerate() {
            // Skip the current version
            if let Some(current) = current_version {
                if version == current {
                    continue;
                }
            }

            // Keep the most recent versions
            if i < self.max_versions {
                continue;
            }

            match fs::remove_file(version) {
                Ok(()) => {
                    removed += 1;
                    tracing::trace!(
                        extension_id = extension_id,
                        path = %version.display(),
                        "Removed old dylib version"
                    );
                },
                Err(e) => {
                    tracing::warn!(
                        extension_id = extension_id,
                        path = %version.display(),
                        error = %e,
                        "Failed to remove old dylib version"
                    );
                },
            }
        }

        if removed > 0 {
            tracing::debug!(
                extension_id = extension_id,
                removed = removed,
                "Cleaned up old dylib versions"
            );
        }

        Ok(removed)
    }

    /// Removes all cached versions for an extension.
    ///
    /// # Arguments
    ///
    /// * `extension_id` - The unique identifier of the extension.
    pub fn clear_extension_cache(&self, extension_id: &str) -> Result<(), DylibCacheError> {
        let ext_cache_dir = self.extension_cache_dir(extension_id);
        if ext_cache_dir.exists() {
            fs::remove_dir_all(&ext_cache_dir).map_err(DylibCacheError::Remove)?;
            tracing::debug!(extension_id = extension_id, "Cleared extension dylib cache");
        }
        Ok(())
    }

    /// Returns the cache directory for a specific extension.
    #[must_use]
    pub fn extension_cache_dir(&self, extension_id: &str) -> PathBuf {
        self.cache_root.join(extension_id)
    }

    /// Lists all cached versions for an extension.
    #[must_use]
    pub fn list_versions(&self, extension_id: &str) -> Vec<PathBuf> {
        let ext_cache_dir = self.extension_cache_dir(extension_id);
        if !ext_cache_dir.exists() {
            return Vec::new();
        }

        fs::read_dir(&ext_cache_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(std::result::Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.extension()
                            .is_some_and(|e| e == "dylib" || e == "so" || e == "dll")
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_versioned_copy_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let cache_root = temp_dir.path().join("cache");
        let source_dir = temp_dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        let source_dylib = source_dir.join("libtest.dylib");
        fs::write(&source_dylib, b"test content").unwrap();

        let cache = DylibCache::new(cache_root);
        let versioned_path = cache
            .create_versioned_copy("test-extension", &source_dylib)
            .unwrap();

        assert!(versioned_path.exists());
        assert!(versioned_path.to_string_lossy().contains("libtest_"));
        assert!(versioned_path.to_string_lossy().ends_with(".dylib"));
    }

    #[test]
    fn test_cleanup_old_versions() {
        let temp_dir = TempDir::new().unwrap();
        let cache_root = temp_dir.path().join("cache");
        let source_dir = temp_dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        let source_dylib = source_dir.join("libtest.dylib");
        fs::write(&source_dylib, b"test content").unwrap();

        let cache = DylibCache::new(cache_root).with_max_versions(2);

        // Create multiple versions
        let _v1 = cache
            .create_versioned_copy("test-extension", &source_dylib)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _v2 = cache
            .create_versioned_copy("test-extension", &source_dylib)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _v3 = cache
            .create_versioned_copy("test-extension", &source_dylib)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let v4 = cache
            .create_versioned_copy("test-extension", &source_dylib)
            .unwrap();

        // Cleanup should remove older versions
        let removed = cache
            .cleanup_old_versions("test-extension", Some(&v4))
            .unwrap();
        assert!(removed >= 1);

        let remaining = cache.list_versions("test-extension");
        assert!(remaining.len() <= 3); // max_versions + current
    }

    #[test]
    fn test_source_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let cache = DylibCache::new(temp_dir.path().to_path_buf());

        let result = cache.create_versioned_copy("test", Path::new("/nonexistent/lib.dylib"));
        assert!(matches!(
            result,
            Err(DylibCacheError::SourceNotFound { .. })
        ));
    }
}
