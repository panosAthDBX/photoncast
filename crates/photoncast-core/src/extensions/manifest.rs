use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::utils::paths::expand_tilde;

pub const SUPPORTED_API_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub schema_version: u32,
    pub extension: ExtensionInfo,
    pub entry: ExtensionEntry,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub commands: Vec<CommandManifest>,
    #[serde(default)]
    pub preferences: Vec<PreferenceManifest>,
    /// Runtime field: the directory containing this extension (set during discovery).
    /// Not serialized - populated programmatically when the manifest is loaded.
    #[serde(skip, default)]
    pub directory: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub min_photoncast_version: Option<String>,
    pub api_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionEntry {
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Permissions {
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub clipboard: bool,
    #[serde(default)]
    pub notifications: bool,
    #[serde(default)]
    pub filesystem: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandManifest {
    pub id: String,
    pub name: String,
    pub mode: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceManifest {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Vec<SelectOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to read manifest: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("failed to parse manifest {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("invalid manifest field {field}: {reason}")]
    Invalid { field: String, reason: String },
}

impl ManifestError {
    #[must_use]
    pub fn invalid(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Invalid {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

pub fn load_manifest(path: &Path) -> Result<ExtensionManifest, ManifestError> {
    let contents = fs::read_to_string(path)?;
    let mut manifest: ExtensionManifest =
        toml::from_str(&contents).map_err(|e| ManifestError::Parse {
            path: path.to_path_buf(),
            source: e,
        })?;

    // Set the runtime directory field based on the manifest path
    manifest.directory = path.parent().map(std::path::Path::to_path_buf);

    Ok(manifest)
}

pub fn validate_manifest(
    manifest: &ExtensionManifest,
    manifest_path: &Path,
) -> Result<(), ManifestError> {
    validate_id(&manifest.extension.id)?;
    validate_semver(&manifest.extension.version)?;
    validate_api_version(manifest.extension.api_version)?;
    validate_entry_path(&manifest.entry.path, manifest_path)?;
    validate_unique_command_ids(&manifest.commands)?;

    Ok(())
}

fn validate_id(id: &str) -> Result<(), ManifestError> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return Err(ManifestError::invalid(
            "extension.id",
            "must be reverse-DNS with at least two segments",
        ));
    }

    if parts.iter().any(|part| !is_valid_identifier(part)) {
        return Err(ManifestError::invalid(
            "extension.id",
            "segments must be alphanumeric with dashes or underscores",
        ));
    }

    Ok(())
}

fn is_valid_identifier(segment: &str) -> bool {
    segment
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn validate_semver(version: &str) -> Result<(), ManifestError> {
    Version::parse(version).map_err(|_| {
        ManifestError::invalid("extension.version", "must be a valid SemVer string")
    })?;
    Ok(())
}

fn validate_api_version(api_version: u32) -> Result<(), ManifestError> {
    if api_version != SUPPORTED_API_VERSION {
        return Err(ManifestError::invalid(
            "extension.api_version",
            format!("unsupported api version: {api_version}"),
        ));
    }
    Ok(())
}

fn validate_entry_path(path: &str, manifest_path: &Path) -> Result<(), ManifestError> {
    let path = expand_tilde(path);
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let entry_path = if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    };

    if !entry_path.exists() {
        return Err(ManifestError::invalid(
            "entry.path",
            format!("entry dylib not found at {}", entry_path.display()),
        ));
    }

    if entry_path.extension().and_then(|e| e.to_str()) != Some("dylib") {
        return Err(ManifestError::invalid(
            "entry.path",
            "entry dylib must have .dylib extension",
        ));
    }

    Ok(())
}

fn validate_unique_command_ids(commands: &[CommandManifest]) -> Result<(), ManifestError> {
    let mut seen = HashSet::new();
    for cmd in commands {
        if !seen.insert(&cmd.id) {
            return Err(ManifestError::invalid(
                "commands.id",
                format!("duplicate command id: {}", cmd.id),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ManifestCacheEntry {
    pub manifest: ExtensionManifest,
    pub path: PathBuf,
    pub modified_at: i64,
}

#[derive(Debug, Default)]
pub struct ManifestCache {
    entries: std::collections::HashMap<String, ManifestCacheEntry>,
    errors: std::collections::HashMap<String, String>,
}

impl ManifestCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, entry: ManifestCacheEntry) {
        self.entries
            .insert(entry.manifest.extension.id.clone(), entry);
    }

    pub fn set_error(&mut self, key: impl Into<String>, error: impl Into<String>) {
        self.errors.insert(key.into(), error.into());
    }

    pub fn get(&self, id: &str) -> Option<&ManifestCacheEntry> {
        self.entries.get(id)
    }

    pub fn error(&self, id: &str) -> Option<&str> {
        self.errors.get(id).map(String::as_str)
    }

    pub fn remove(&mut self, id: &str) {
        self.entries.remove(id);
        self.errors.remove(id);
    }
}

pub fn read_manifest_with_cache(
    cache: &mut ManifestCache,
    manifest_path: &Path,
) -> Result<ExtensionManifest, ManifestError> {
    let metadata = fs::metadata(manifest_path)?;
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or_else(|| Utc::now().timestamp(), |d| d.as_secs() as i64);

    let path_key = manifest_path.to_string_lossy().to_string();

    if let Some(entry) = cache
        .entries
        .values()
        .find(|entry| entry.path == manifest_path)
    {
        if entry.modified_at == modified_at {
            return Ok(entry.manifest.clone());
        }
        let entry_id = entry.manifest.extension.id.clone();
        cache.remove(&entry_id);
    }

    let manifest = load_manifest(manifest_path)?;
    if let Err(err) = validate_manifest(&manifest, manifest_path) {
        cache.set_error(path_key, err.to_string());
        return Err(err);
    }

    cache.remove(&path_key);
    cache.insert(ManifestCacheEntry {
        manifest: manifest.clone(),
        path: manifest_path.to_path_buf(),
        modified_at,
    });

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn create_test_dylib(dir: &TempDir) -> PathBuf {
        let dylib_path = dir.path().join("test.dylib");
        std::fs::write(&dylib_path, b"fake dylib").unwrap();
        dylib_path
    }

    fn create_valid_manifest_toml(dylib_path: &Path) -> String {
        format!(
            r#"
schema_version = 1

[extension]
id = "com.example.test"
name = "Test Extension"
version = "1.0.0"
description = "A test extension"
api_version = {}

[entry]
kind = "dylib"
path = "{}"
"#,
            SUPPORTED_API_VERSION,
            dylib_path.display()
        )
    }

    // =============================================================================
    // Task 10.1: Manifest Parsing Unit Tests
    // =============================================================================

    #[test]
    fn test_valid_manifest_parses_correctly() {
        let dir = TempDir::new().unwrap();
        let dylib_path = create_test_dylib(&dir);

        let manifest_content = create_valid_manifest_toml(&dylib_path);
        let manifest_path = dir.path().join("manifest.toml");
        std::fs::write(&manifest_path, &manifest_content).unwrap();

        let manifest = load_manifest(&manifest_path).unwrap();
        let result = validate_manifest(&manifest, &manifest_path);

        assert!(result.is_ok());
        assert_eq!(manifest.extension.id, "com.example.test");
        assert_eq!(manifest.extension.name, "Test Extension");
        assert_eq!(manifest.extension.version, "1.0.0");
        assert_eq!(manifest.extension.api_version, SUPPORTED_API_VERSION);
    }

    #[test]
    fn test_invalid_id_single_segment() {
        // ID without dots (single segment) should fail
        let result = validate_id("testextension");
        assert!(result.is_err());

        if let Err(ManifestError::Invalid { field, reason }) = result {
            assert_eq!(field, "extension.id");
            assert!(reason.contains("reverse-DNS"));
        } else {
            panic!("Expected Invalid error");
        }
    }

    #[test]
    fn test_invalid_id_empty_segment() {
        // ID with empty segments should fail
        let result = validate_id("com..test");
        assert!(result.is_err());

        if let Err(ManifestError::Invalid { field, .. }) = result {
            assert_eq!(field, "extension.id");
        } else {
            panic!("Expected Invalid error");
        }
    }

    #[test]
    fn test_invalid_id_special_characters() {
        // ID with invalid characters should fail
        let result = validate_id("com.test@invalid");
        assert!(result.is_err());

        if let Err(ManifestError::Invalid { field, reason }) = result {
            assert_eq!(field, "extension.id");
            assert!(reason.contains("alphanumeric"));
        } else {
            panic!("Expected Invalid error");
        }
    }

    #[test]
    fn test_valid_id_with_dashes_underscores() {
        // Valid ID with dashes and underscores should pass
        let result = validate_id("com.example-test.my_extension");
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_id_two_segments() {
        // Minimum valid ID with two segments
        let result = validate_id("com.test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_version_not_semver() {
        // Version that's not valid SemVer should fail
        let result = validate_semver("1.0");
        assert!(result.is_err());

        if let Err(ManifestError::Invalid { field, reason }) = result {
            assert_eq!(field, "extension.version");
            assert!(reason.contains("SemVer"));
        } else {
            panic!("Expected Invalid error");
        }
    }

    #[test]
    fn test_invalid_version_text() {
        let result = validate_semver("version-one");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_version_missing_patch() {
        let result = validate_semver("1.2");
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_version_semver() {
        let result = validate_semver("1.0.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_version_semver_with_prerelease() {
        let result = validate_semver("1.0.0-alpha.1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_version_semver_with_build() {
        let result = validate_semver("1.0.0+build.123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_unsupported_api_version_zero() {
        let result = validate_api_version(0);
        assert!(result.is_err());

        if let Err(ManifestError::Invalid { field, reason }) = result {
            assert_eq!(field, "extension.api_version");
            assert!(reason.contains("unsupported"));
        } else {
            panic!("Expected Invalid error");
        }
    }

    #[test]
    fn test_unsupported_api_version_future() {
        let result = validate_api_version(SUPPORTED_API_VERSION + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_supported_api_version() {
        let result = validate_api_version(SUPPORTED_API_VERSION);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_required_extension_id() {
        let manifest_content = r#"
schema_version = 1

[extension]
name = "Test Extension"
version = "1.0.0"
description = "A test extension"
api_version = 1

[entry]
kind = "dylib"
path = "test.dylib"
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(manifest_content.as_bytes()).unwrap();

        let result = load_manifest(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_extension_name() {
        let manifest_content = r#"
schema_version = 1

[extension]
id = "com.example.test"
version = "1.0.0"
description = "A test extension"
api_version = 1

[entry]
kind = "dylib"
path = "test.dylib"
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(manifest_content.as_bytes()).unwrap();

        let result = load_manifest(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_extension_version() {
        let manifest_content = r#"
schema_version = 1

[extension]
id = "com.example.test"
name = "Test Extension"
description = "A test extension"
api_version = 1

[entry]
kind = "dylib"
path = "test.dylib"
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(manifest_content.as_bytes()).unwrap();

        let result = load_manifest(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_entry_section() {
        let manifest_content = r#"
schema_version = 1

[extension]
id = "com.example.test"
name = "Test Extension"
version = "1.0.0"
description = "A test extension"
api_version = 1
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(manifest_content.as_bytes()).unwrap();

        let result = load_manifest(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_command_ids() {
        let commands = vec![
            CommandManifest {
                id: "cmd1".to_string(),
                name: "Command 1".to_string(),
                mode: "search".to_string(),
                keywords: vec![],
                icon: None,
                subtitle: None,
            },
            CommandManifest {
                id: "cmd1".to_string(), // Duplicate
                name: "Command 2".to_string(),
                mode: "search".to_string(),
                keywords: vec![],
                icon: None,
                subtitle: None,
            },
        ];

        let result = validate_unique_command_ids(&commands);
        assert!(result.is_err());

        if let Err(ManifestError::Invalid { field, reason }) = result {
            assert_eq!(field, "commands.id");
            assert!(reason.contains("duplicate"));
        } else {
            panic!("Expected Invalid error");
        }
    }

    #[test]
    fn test_unique_command_ids_passes() {
        let commands = vec![
            CommandManifest {
                id: "cmd1".to_string(),
                name: "Command 1".to_string(),
                mode: "search".to_string(),
                keywords: vec![],
                icon: None,
                subtitle: None,
            },
            CommandManifest {
                id: "cmd2".to_string(),
                name: "Command 2".to_string(),
                mode: "search".to_string(),
                keywords: vec![],
                icon: None,
                subtitle: None,
            },
        ];

        let result = validate_unique_command_ids(&commands);
        assert!(result.is_ok());
    }

    #[test]
    fn test_manifest_cache_insert_and_get() {
        let dir = TempDir::new().unwrap();
        let dylib_path = create_test_dylib(&dir);

        let manifest = ExtensionManifest {
            schema_version: 1,
            extension: ExtensionInfo {
                id: "com.example.test".to_string(),
                name: "Test".to_string(),
                version: "1.0.0".to_string(),
                description: "Test extension".to_string(),
                author: None,
                license: None,
                homepage: None,
                min_photoncast_version: None,
                api_version: SUPPORTED_API_VERSION,
            },
            entry: ExtensionEntry {
                kind: "dylib".to_string(),
                path: dylib_path.to_string_lossy().to_string(),
            },
            permissions: Permissions::default(),
            commands: vec![],
            preferences: vec![],
            directory: None,
        };

        let mut cache = ManifestCache::new();
        cache.insert(ManifestCacheEntry {
            manifest: manifest.clone(),
            path: dir.path().join("manifest.toml"),
            modified_at: 12345,
        });

        let entry = cache.get("com.example.test");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().manifest.extension.name, "Test");
    }

    #[test]
    fn test_manifest_cache_error() {
        let mut cache = ManifestCache::new();
        cache.set_error("test_key", "some error occurred");

        let error = cache.error("test_key");
        assert_eq!(error, Some("some error occurred"));

        let no_error = cache.error("other_key");
        assert!(no_error.is_none());
    }

    #[test]
    fn test_manifest_cache_remove() {
        let manifest = ExtensionManifest {
            schema_version: 1,
            extension: ExtensionInfo {
                id: "com.example.test".to_string(),
                name: "Test".to_string(),
                version: "1.0.0".to_string(),
                description: "Test extension".to_string(),
                author: None,
                license: None,
                homepage: None,
                min_photoncast_version: None,
                api_version: SUPPORTED_API_VERSION,
            },
            entry: ExtensionEntry {
                kind: "dylib".to_string(),
                path: "test.dylib".to_string(),
            },
            permissions: Permissions::default(),
            commands: vec![],
            preferences: vec![],
            directory: None,
        };

        let mut cache = ManifestCache::new();
        cache.insert(ManifestCacheEntry {
            manifest,
            path: PathBuf::from("test/manifest.toml"),
            modified_at: 12345,
        });
        cache.set_error("com.example.test", "some error");

        assert!(cache.get("com.example.test").is_some());
        assert!(cache.error("com.example.test").is_some());

        cache.remove("com.example.test");

        assert!(cache.get("com.example.test").is_none());
        assert!(cache.error("com.example.test").is_none());
    }

    #[test]
    fn test_entry_path_not_found() {
        let dir = TempDir::new().unwrap();
        let manifest_path = dir.path().join("manifest.toml");

        let result = validate_entry_path("nonexistent.dylib", &manifest_path);
        assert!(result.is_err());

        if let Err(ManifestError::Invalid { field, reason }) = result {
            assert_eq!(field, "entry.path");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected Invalid error");
        }
    }

    #[test]
    fn test_entry_path_wrong_extension() {
        let dir = TempDir::new().unwrap();
        let wrong_ext_path = dir.path().join("test.so");
        std::fs::write(&wrong_ext_path, b"fake lib").unwrap();

        let manifest_path = dir.path().join("manifest.toml");
        let result = validate_entry_path(&wrong_ext_path.to_string_lossy(), &manifest_path);

        assert!(result.is_err());
        if let Err(ManifestError::Invalid { field, reason }) = result {
            assert_eq!(field, "entry.path");
            assert!(reason.contains(".dylib extension"));
        } else {
            panic!("Expected Invalid error");
        }
    }

    #[test]
    fn test_entry_path_valid_dylib() {
        let dir = TempDir::new().unwrap();
        let dylib_path = create_test_dylib(&dir);
        let manifest_path = dir.path().join("manifest.toml");

        let result = validate_entry_path(&dylib_path.to_string_lossy(), &manifest_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_permissions_default() {
        let permissions = Permissions::default();
        assert!(!permissions.network);
        assert!(!permissions.clipboard);
        assert!(!permissions.notifications);
        assert!(permissions.filesystem.is_empty());
    }

    #[test]
    fn test_manifest_with_full_permissions() {
        let dir = TempDir::new().unwrap();
        let dylib_path = create_test_dylib(&dir);

        let manifest_content = format!(
            r#"
schema_version = 1

[extension]
id = "com.example.test"
name = "Test Extension"
version = "1.0.0"
description = "A test extension"
api_version = {}

[entry]
kind = "dylib"
path = "{}"

[permissions]
network = true
clipboard = true
notifications = true
filesystem = ["~/Documents", "/tmp"]
"#,
            SUPPORTED_API_VERSION,
            dylib_path.display()
        );

        let manifest_path = dir.path().join("manifest.toml");
        std::fs::write(&manifest_path, &manifest_content).unwrap();

        let manifest = load_manifest(&manifest_path).unwrap();
        assert!(manifest.permissions.network);
        assert!(manifest.permissions.clipboard);
        assert!(manifest.permissions.notifications);
        assert_eq!(manifest.permissions.filesystem.len(), 2);
    }

    #[test]
    fn test_manifest_with_commands() {
        let dir = TempDir::new().unwrap();
        let dylib_path = create_test_dylib(&dir);

        let manifest_content = format!(
            r#"
schema_version = 1

[extension]
id = "com.example.test"
name = "Test Extension"
version = "1.0.0"
description = "A test extension"
api_version = {}

[entry]
kind = "dylib"
path = "{}"

[[commands]]
id = "search"
name = "Search"
mode = "search"
keywords = ["find", "lookup"]

[[commands]]
id = "copy"
name = "Copy to Clipboard"
mode = "action"
icon = "doc.on.clipboard"
subtitle = "Copy selected item"
"#,
            SUPPORTED_API_VERSION,
            dylib_path.display()
        );

        let manifest_path = dir.path().join("manifest.toml");
        std::fs::write(&manifest_path, &manifest_content).unwrap();

        let manifest = load_manifest(&manifest_path).unwrap();
        assert_eq!(manifest.commands.len(), 2);

        assert_eq!(manifest.commands[0].id, "search");
        assert_eq!(manifest.commands[0].keywords, vec!["find", "lookup"]);

        assert_eq!(manifest.commands[1].id, "copy");
        assert_eq!(
            manifest.commands[1].icon,
            Some("doc.on.clipboard".to_string())
        );
    }

    #[test]
    fn test_manifest_with_preferences() {
        let dir = TempDir::new().unwrap();
        let dylib_path = create_test_dylib(&dir);

        let manifest_content = format!(
            r#"
schema_version = 1

[extension]
id = "com.example.test"
name = "Test Extension"
version = "1.0.0"
description = "A test extension"
api_version = {}

[entry]
kind = "dylib"
path = "{}"

[[preferences]]
name = "api_key"
type = "password"
required = true
title = "API Key"
description = "Your API key for authentication"

[[preferences]]
name = "theme"
type = "dropdown"
required = false
title = "Theme"
default = "dark"

[[preferences.options]]
label = "Dark"
value = "dark"

[[preferences.options]]
label = "Light"
value = "light"
"#,
            SUPPORTED_API_VERSION,
            dylib_path.display()
        );

        let manifest_path = dir.path().join("manifest.toml");
        std::fs::write(&manifest_path, &manifest_content).unwrap();

        let manifest = load_manifest(&manifest_path).unwrap();
        assert_eq!(manifest.preferences.len(), 2);

        assert_eq!(manifest.preferences[0].name, "api_key");
        assert!(manifest.preferences[0].required);
        assert_eq!(manifest.preferences[0].kind, "password");

        assert_eq!(manifest.preferences[1].name, "theme");
        assert_eq!(manifest.preferences[1].options.len(), 2);
    }
}
