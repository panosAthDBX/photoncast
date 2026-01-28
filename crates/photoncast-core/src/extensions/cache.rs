use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use abi_stable::external_types::RawValueBox;
use abi_stable::std_types::{RDuration, ROption};
use parking_lot::RwLock;
use photoncast_extension_api::RStr;
use photoncast_extension_api::{Cache, CacheTrait};
use serde_json::value::RawValue;

#[derive(Clone)]
pub struct ExtensionCache {
    namespace: String,
    cache_dir: PathBuf,
    max_entries: usize,
    inner: std::sync::Arc<RwLock<HashMap<String, CacheEntry>>>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: serde_json::Value,
    expires_at: Option<Instant>,
    /// Tracks whether entry has been persisted to disk.
    #[allow(dead_code)]
    persisted: bool,
}

impl ExtensionCache {
    #[must_use]
    pub fn new(namespace: impl Into<String>, cache_dir: PathBuf) -> Self {
        Self {
            namespace: namespace.into(),
            cache_dir,
            max_entries: 512,
            inner: std::sync::Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[must_use]
    pub fn api_handle(&self) -> Cache {
        Cache::new(self.clone())
    }

    fn cache_path(&self, key: &str) -> PathBuf {
        let sanitized = key.replace('/', "_");
        self.cache_dir
            .join(format!("{}_{}.json", self.namespace, sanitized))
    }

    fn evict_if_needed(&self) {
        let mut cache = self.inner.write();
        if cache.len() <= self.max_entries {
            return;
        }
        if let Some(key) = cache
            .iter()
            .min_by_key(|(_, v)| v.expires_at)
            .map(|(k, _)| k.clone())
        {
            cache.remove(&key);
        }
    }
}

impl CacheTrait for ExtensionCache {
    fn get(&self, key: RStr<'_>) -> ROption<RawValueBox> {
        let key = key.as_str();

        // Fast path: read lock only — covers cache hits with no expiry or still-valid TTL.
        {
            let cache = self.inner.read();
            if let Some(entry) = cache.get(key) {
                if let Some(exp) = entry.expires_at {
                    if Instant::now() > exp {
                        // Entry expired — drop read lock, acquire write lock to remove.
                        drop(cache);
                        self.inner.write().remove(key);
                        return ROption::RNone;
                    }
                }
                let value = serde_json::to_string(&entry.value).ok();
                return value
                    .and_then(|json| RawValueBox::try_from_string(json).ok())
                    .into();
            }
        }

        // Miss in memory — try disk, then insert under write lock.
        let disk_path = self.cache_path(key);
        if let Ok(contents) = std::fs::read_to_string(&disk_path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) {
                self.inner.write().insert(
                    key.to_string(),
                    CacheEntry {
                        value: value.clone(),
                        expires_at: None,
                        persisted: true,
                    },
                );
                let boxed = RawValueBox::try_from_string(contents).ok();
                return boxed.into();
            }
        }

        ROption::RNone
    }

    fn set(&self, key: RStr<'_>, value: RawValueBox, ttl: ROption<RDuration>) {
        let key = key.as_str();
        let expires_at = ttl
            .into_option()
            .map(|d| Instant::now() + Duration::from(d));
        let Ok(raw) = serde_json::from_str::<&RawValue>(value.get()) else {
            return;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(raw.get()) else {
            return;
        };
        let _ = std::fs::create_dir_all(&self.cache_dir);
        let mut cache = self.inner.write();
        cache.insert(
            key.to_string(),
            CacheEntry {
                value: json.clone(),
                expires_at,
                persisted: false,
            },
        );
        self.evict_if_needed();
        if let Ok(contents) = serde_json::to_string(&json) {
            let _ = std::fs::write(self.cache_path(key), contents);
        }
    }

    fn remove(&self, key: RStr<'_>) {
        let key = key.as_str();
        self.inner.write().remove(key);
        let disk_path = self.cache_path(key);
        let _ = std::fs::remove_file(disk_path);
    }

    fn clear(&self) {
        self.inner.write().clear();
    }

    fn has(&self, key: RStr<'_>) -> bool {
        let key = key.as_str();
        let cache = self.inner.read();
        cache.contains_key(key)
    }
}
