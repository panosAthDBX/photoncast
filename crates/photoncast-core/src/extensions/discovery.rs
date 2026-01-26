use std::path::{Path, PathBuf};

use crate::extensions::manifest::{read_manifest_with_cache, ExtensionManifest, ManifestCache};
use crate::extensions::manifest::{ManifestError, SUPPORTED_API_VERSION};
use crate::utils::paths;

#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    pub dev_mode: bool,
    pub dev_paths: Vec<PathBuf>,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            dev_mode: false,
            dev_paths: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ExtensionDiscovery {
    manifest_cache: ManifestCache,
}

impl ExtensionDiscovery {
    #[must_use]
    pub fn new() -> Self {
        Self {
            manifest_cache: ManifestCache::new(),
        }
    }

    pub fn discover(
        &mut self,
        options: &DiscoveryOptions,
    ) -> Vec<Result<ExtensionManifest, ManifestError>> {
        let mut results = Vec::new();
        let base_dir = extensions_root_dir();

        if let Ok(entries) = std::fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(manifest_path) = find_manifest(&path) {
                        results.push(read_manifest_with_cache(
                            &mut self.manifest_cache,
                            &manifest_path,
                        ));
                    }
                }
            }
        }

        if options.dev_mode {
            for dev_path in &options.dev_paths {
                if !dev_path.exists() {
                    tracing::warn!(path = %dev_path.display(), "Dev extension path does not exist");
                    continue;
                }
                if !is_allowed_dev_path(dev_path) {
                    tracing::warn!(path = %dev_path.display(), "Dev extension path not allowed");
                    continue;
                }
                if let Some(manifest_path) = find_manifest(dev_path) {
                    results.push(read_manifest_with_cache(
                        &mut self.manifest_cache,
                        &manifest_path,
                    ));
                }
            }
        }

        results
    }

    #[must_use]
    pub fn cache(&self) -> &ManifestCache {
        &self.manifest_cache
    }
}

fn extensions_root_dir() -> PathBuf {
    paths::data_dir().join("extensions")
}

fn find_manifest(dir: &Path) -> Option<PathBuf> {
    let manifest_path = dir.join("extension.toml");
    if manifest_path.exists() {
        return Some(manifest_path);
    }
    None
}

fn is_allowed_dev_path(path: &Path) -> bool {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    path.starts_with(&home)
}

pub fn ensure_supported_api(manifest: &ExtensionManifest) -> Result<(), ManifestError> {
    if manifest.extension.api_version != SUPPORTED_API_VERSION {
        return Err(ManifestError::invalid(
            "extension.api_version",
            format!("unsupported api version {}", manifest.extension.api_version),
        ));
    }
    Ok(())
}
