use std::path::{Path, PathBuf};

use crate::extensions::manifest::{
    read_manifest_with_cache, ExtensionManifest, ManifestCache, ManifestError,
    SUPPORTED_API_VERSION,
};
use crate::utils::paths;

#[derive(Debug, Clone, Default)]
pub struct DiscoveryOptions {
    pub dev_mode: bool,
    pub dev_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct ExtensionDiscovery {
    manifest_cache: ManifestCache,
}

#[allow(clippy::new_without_default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::manifest::Permissions;

    #[test]
    fn test_discovery_options_default() {
        let opts = DiscoveryOptions::default();
        assert!(!opts.dev_mode);
        assert!(opts.dev_paths.is_empty());
    }

    #[test]
    fn test_discovery_empty_directory() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        // Discover in a completely empty directory — should produce no results
        // (The discover method reads from extensions_root_dir(), but we test find_manifest)
        assert!(find_manifest(dir.path()).is_none());
    }

    #[test]
    fn test_find_manifest_with_valid_extension() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let manifest_path = dir.path().join("extension.toml");
        std::fs::write(&manifest_path, "# placeholder").expect("failed to write");

        let found = find_manifest(dir.path());
        assert!(found.is_some());
        assert_eq!(found.unwrap(), manifest_path);
    }

    #[test]
    fn test_find_manifest_missing() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        // Create a random file, but not extension.toml
        std::fs::write(dir.path().join("README.md"), "# hello").expect("failed to write");

        assert!(find_manifest(dir.path()).is_none());
    }

    #[test]
    fn test_is_allowed_dev_path() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let allowed = home.join("my-extension");
        assert!(is_allowed_dev_path(&allowed));

        // A path outside home should not be allowed
        let disallowed = PathBuf::from("/nonexistent_root_path/ext");
        assert!(!is_allowed_dev_path(&disallowed));
    }

    #[test]
    fn test_ensure_supported_api_valid() {
        let manifest = ExtensionManifest {
            schema_version: 1,
            extension: crate::extensions::manifest::ExtensionInfo {
                id: "test-ext".to_string(),
                name: "Test Extension".to_string(),
                version: "0.1.0".to_string(),
                description: "A test extension".to_string(),
                author: None,
                license: None,
                homepage: None,
                min_photoncast_version: None,
                api_version: SUPPORTED_API_VERSION,
            },
            entry: crate::extensions::manifest::ExtensionEntry {
                kind: "dylib".to_string(),
                path: "libtest.dylib".to_string(),
            },
            permissions: Permissions::default(),
            commands: vec![],
            preferences: vec![],
            directory: None,
        };
        assert!(ensure_supported_api(&manifest).is_ok());
    }

    #[test]
    fn test_ensure_supported_api_invalid() {
        let manifest = ExtensionManifest {
            schema_version: 1,
            extension: crate::extensions::manifest::ExtensionInfo {
                id: "test-ext".to_string(),
                name: "Test Extension".to_string(),
                version: "0.1.0".to_string(),
                description: "A test extension".to_string(),
                author: None,
                license: None,
                homepage: None,
                min_photoncast_version: None,
                api_version: 999,
            },
            entry: crate::extensions::manifest::ExtensionEntry {
                kind: "dylib".to_string(),
                path: "libtest.dylib".to_string(),
            },
            permissions: Permissions::default(),
            commands: vec![],
            preferences: vec![],
            directory: None,
        };
        assert!(ensure_supported_api(&manifest).is_err());
    }
}
