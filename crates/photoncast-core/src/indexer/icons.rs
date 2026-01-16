//! Icon extraction and caching.
//!
//! This module provides functionality for extracting icons from macOS application
//! bundles and caching them to disk with an LRU eviction policy.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use tracing::{debug, warn};

/// Default LRU cache capacity (number of icons).
const DEFAULT_CACHE_CAPACITY: usize = 100;

/// Lazy-loaded icon data.
pub struct LazyIcon {
    /// Path to the cached icon file.
    pub cached_path: PathBuf,
    /// Loaded icon data (lazy).
    data: OnceCell<Vec<u8>>,
}

impl LazyIcon {
    /// Creates a new lazy icon.
    #[must_use]
    pub fn new(cached_path: PathBuf) -> Self {
        Self {
            cached_path,
            data: OnceCell::new(),
        }
    }

    /// Gets the icon data, loading it from disk if necessary.
    ///
    /// # Errors
    ///
    /// Returns an error if the icon cannot be read from disk.
    pub fn get_data(&self) -> Result<&[u8]> {
        self.data
            .get_or_try_init(|| std::fs::read(&self.cached_path))
            .map(Vec::as_slice)
            .context("failed to load icon data")
    }

    /// Returns the cached path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.cached_path
    }
}

/// LRU cache entry.
struct CacheEntry {
    /// The lazy icon.
    icon: Arc<LazyIcon>,
    /// Access order (higher = more recent).
    access_order: u64,
}

/// LRU icon cache with configurable capacity.
pub struct IconCache {
    /// Cache directory on disk.
    cache_dir: PathBuf,
    /// In-memory LRU cache.
    cache: RwLock<HashMap<String, CacheEntry>>,
    /// Maximum number of icons to cache.
    capacity: usize,
    /// Counter for access ordering.
    access_counter: RwLock<u64>,
}

impl IconCache {
    /// Creates a new icon cache with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CACHE_CAPACITY)
    }

    /// Creates a new icon cache with custom capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache_dir: default_cache_dir(),
            cache: RwLock::new(HashMap::new()),
            capacity,
            access_counter: RwLock::new(0),
        }
    }

    /// Creates a new icon cache with custom directory and capacity.
    #[must_use]
    pub fn with_dir_and_capacity(cache_dir: PathBuf, capacity: usize) -> Self {
        Self {
            cache_dir,
            cache: RwLock::new(HashMap::new()),
            capacity,
            access_counter: RwLock::new(0),
        }
    }

    /// Returns the cache directory.
    #[must_use]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Gets an icon from the cache, extracting it if necessary.
    ///
    /// # Arguments
    ///
    /// * `app_path` - Path to the .app bundle.
    /// * `bundle_id` - The app's bundle identifier (used as cache key).
    ///
    /// # Errors
    ///
    /// Returns an error if the icon cannot be extracted or cached.
    pub async fn get_or_extract(&self, app_path: &Path, bundle_id: &str) -> Result<Arc<LazyIcon>> {
        // Check if already in cache
        if let Some(icon) = self.get(bundle_id) {
            return Ok(icon);
        }

        // Extract icon to cache
        let cached_path = extract_icon(app_path, &self.cache_dir).await?;
        let icon = Arc::new(LazyIcon::new(cached_path));

        // Insert into cache
        self.insert(bundle_id.to_string(), Arc::clone(&icon));

        Ok(icon)
    }

    /// Gets an icon from the cache if present.
    #[must_use]
    pub fn get(&self, bundle_id: &str) -> Option<Arc<LazyIcon>> {
        let mut cache = self.cache.write();

        if let Some(entry) = cache.get_mut(bundle_id) {
            // Update access order
            let mut counter = self.access_counter.write();
            *counter += 1;
            entry.access_order = *counter;
            return Some(Arc::clone(&entry.icon));
        }

        None
    }

    /// Inserts an icon into the cache.
    pub fn insert(&self, bundle_id: String, icon: Arc<LazyIcon>) {
        let mut cache = self.cache.write();

        // Evict if at capacity
        if cache.len() >= self.capacity {
            self.evict_lru(&mut cache);
        }

        let mut counter = self.access_counter.write();
        *counter += 1;

        cache.insert(
            bundle_id,
            CacheEntry {
                icon,
                access_order: *counter,
            },
        );
    }

    /// Evicts the least recently used entry.
    fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some((key, entry)) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.access_order)
            .map(|(k, v)| (k.clone(), v.icon.cached_path.clone()))
        {
            debug!("Evicting LRU icon: {}", key);
            cache.remove(&key);

            // Optionally delete the file from disk
            if let Err(e) = std::fs::remove_file(&entry) {
                warn!("Failed to remove evicted icon file: {}", e);
            }
        }
    }

    /// Returns the number of icons currently cached.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    /// Returns true if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache.read().is_empty()
    }

    /// Clears all icons from the cache and disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be cleared.
    pub async fn clear(&self) -> Result<()> {
        let mut cache = self.cache.write();
        cache.clear();

        // Clear disk cache
        if self.cache_dir.exists() {
            tokio::fs::remove_dir_all(&self.cache_dir)
                .await
                .context("failed to clear icon cache directory")?;
        }

        Ok(())
    }

    /// Initializes the cache directory, creating it if necessary.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    pub async fn init(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .context("failed to create icon cache directory")?;
        Ok(())
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts and caches the icon from an application bundle.
///
/// # Arguments
///
/// * `app_path` - Path to the .app bundle.
/// * `cache_dir` - Directory to cache extracted icons.
///
/// # Errors
///
/// Returns an error if the icon cannot be extracted.
pub async fn extract_icon(app_path: &Path, cache_dir: &Path) -> Result<PathBuf> {
    // Read Info.plist to get icon file name
    let info_plist_path = app_path.join("Contents/Info.plist");
    let contents = tokio::fs::read(&info_plist_path)
        .await
        .context("failed to read Info.plist")?;

    let plist_value: plist::Value =
        plist::from_bytes(&contents).context("failed to parse Info.plist")?;

    let dict = plist_value
        .as_dictionary()
        .context("Info.plist is not a dictionary")?;

    // Get icon file name - try multiple keys
    let icon_name = dict
        .get("CFBundleIconFile")
        .or_else(|| dict.get("CFBundleIconName"))
        .and_then(plist::Value::as_string)
        .unwrap_or("AppIcon");

    // Icon might or might not have .icns extension
    let icon_filename = if icon_name.ends_with(".icns") {
        icon_name.to_string()
    } else {
        format!("{icon_name}.icns")
    };

    let icon_path = app_path.join("Contents/Resources").join(&icon_filename);

    // Try alternative icon locations if primary not found
    let icon_path = if icon_path.exists() {
        icon_path
    } else {
        // Try without extension addition
        let alt_path = app_path.join("Contents/Resources").join(icon_name);
        if alt_path.exists() {
            alt_path
        } else {
            // Try generic AppIcon.icns
            let generic_path = app_path.join("Contents/Resources/AppIcon.icns");
            if generic_path.exists() {
                generic_path
            } else {
                anyhow::bail!("icon file not found for: {}", app_path.display());
            }
        }
    };

    // Create cache directory if needed
    tokio::fs::create_dir_all(cache_dir)
        .await
        .context("failed to create icon cache directory")?;

    // Generate cache filename from bundle path hash
    let hash = hash_path(app_path);
    let cached_path = cache_dir.join(format!("{hash:x}.icns"));

    // Copy icon to cache if not already there or if source is newer
    let should_copy = if cached_path.exists() {
        // Check if source is newer
        let source_modified = tokio::fs::metadata(&icon_path)
            .await
            .ok()
            .and_then(|m| m.modified().ok());
        let cached_modified = tokio::fs::metadata(&cached_path)
            .await
            .ok()
            .and_then(|m| m.modified().ok());

        match (source_modified, cached_modified) {
            (Some(src), Some(cached)) => src > cached,
            _ => true,
        }
    } else {
        true
    };

    if should_copy {
        tokio::fs::copy(&icon_path, &cached_path)
            .await
            .context("failed to copy icon to cache")?;
        debug!(
            "Cached icon for {} -> {}",
            app_path.display(),
            cached_path.display()
        );
    }

    Ok(cached_path)
}

/// Generates a hash for a path.
fn hash_path(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

/// Returns the default icon cache directory.
#[must_use]
pub fn default_cache_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "PhotonCast")
        .map(|dirs| dirs.cache_dir().join("icons"))
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Library/Caches/PhotonCast/icons")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_cache_dir() {
        let dir = default_cache_dir();
        assert!(dir.to_string_lossy().contains("PhotonCast"));
        assert!(dir.to_string_lossy().contains("icons"));
    }

    #[test]
    fn test_icon_cache_creation() {
        let cache = IconCache::new();
        assert_eq!(cache.capacity, DEFAULT_CACHE_CAPACITY);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_icon_cache_with_capacity() {
        let cache = IconCache::with_capacity(50);
        assert_eq!(cache.capacity, 50);
    }

    #[test]
    fn test_hash_path() {
        let path1 = PathBuf::from("/Applications/Safari.app");
        let path2 = PathBuf::from("/Applications/Safari.app");
        let path3 = PathBuf::from("/Applications/Chrome.app");

        assert_eq!(hash_path(&path1), hash_path(&path2));
        assert_ne!(hash_path(&path1), hash_path(&path3));
    }

    #[tokio::test]
    async fn test_icon_cache_init() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("icons");

        let cache = IconCache::with_dir_and_capacity(cache_dir.clone(), 10);
        assert!(!cache_dir.exists());

        cache.init().await.unwrap();
        assert!(cache_dir.exists());
    }
}
