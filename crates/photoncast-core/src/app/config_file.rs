//! Configuration file loading and saving.
//!
//! This module handles loading and saving PhotonCast configuration
//! to disk using TOML format.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use tracing::{debug, info};

use crate::app::config::Config;

/// Default config file name.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// Default config directory name under ~/.config/.
pub const CONFIG_DIR_NAME: &str = "photoncast";

/// Errors that can occur during config file operations.
#[derive(Debug, Error)]
pub enum ConfigFileError {
    /// Failed to read the config file.
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] io::Error),

    /// Failed to parse the config file.
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),

    /// Failed to serialize the config.
    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),

    /// Failed to create config directory.
    #[error("Failed to create config directory: {0}")]
    DirectoryCreationError(String),

    /// Config directory not found.
    #[error("Could not determine config directory")]
    NoConfigDirectory,
}

/// Result type for config file operations.
pub type ConfigResult<T> = Result<T, ConfigFileError>;

/// Returns the default config directory path (~/.config/photoncast/).
#[must_use]
pub fn default_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join(CONFIG_DIR_NAME))
}

/// Returns the default config file path (~/.config/photoncast/config.toml).
#[must_use]
pub fn default_config_path() -> Option<PathBuf> {
    default_config_dir().map(|p| p.join(CONFIG_FILE_NAME))
}

/// Ensures the config directory exists, creating it if necessary.
pub fn ensure_config_dir() -> ConfigResult<PathBuf> {
    let dir = default_config_dir().ok_or(ConfigFileError::NoConfigDirectory)?;

    if !dir.exists() {
        debug!(path = %dir.display(), "Creating config directory");
        fs::create_dir_all(&dir).map_err(|e| {
            ConfigFileError::DirectoryCreationError(format!(
                "Failed to create {}: {}",
                dir.display(),
                e
            ))
        })?;
        info!(path = %dir.display(), "Created config directory");
    }

    Ok(dir)
}

/// Loads configuration from the default config file.
///
/// If the config file doesn't exist, creates it with default values.
/// If the config file is invalid, returns an error.
pub fn load_config() -> ConfigResult<Config> {
    let path = default_config_path().ok_or(ConfigFileError::NoConfigDirectory)?;
    load_config_from(&path)
}

/// Loads configuration from a specific file path.
///
/// If the config file doesn't exist, creates it with default values.
pub fn load_config_from(path: &Path) -> ConfigResult<Config> {
    if path.exists() {
        debug!(path = %path.display(), "Loading config file");
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        info!(path = %path.display(), "Config file loaded");
        Ok(config)
    } else {
        info!(path = %path.display(), "Config file not found, creating default");
        let config = Config::default();
        save_config_to(&config, path)?;
        Ok(config)
    }
}

/// Saves configuration to the default config file.
pub fn save_config(config: &Config) -> ConfigResult<()> {
    let path = default_config_path().ok_or(ConfigFileError::NoConfigDirectory)?;
    save_config_to(config, &path)
}

/// Saves configuration to a specific file path using atomic write.
///
/// The atomic write is performed by:
/// 1. Writing to a temporary file in the same directory
/// 2. Syncing the file to disk
/// 3. Renaming the temp file to the target path (atomic on POSIX)
pub fn save_config_to(config: &Config, path: &Path) -> ConfigResult<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            debug!(path = %parent.display(), "Creating parent directory");
            fs::create_dir_all(parent).map_err(|e| {
                ConfigFileError::DirectoryCreationError(format!(
                    "Failed to create {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }

    // Serialize config to TOML
    let contents = toml::to_string_pretty(config)?;

    // Perform atomic write
    atomic_write(path, contents.as_bytes())?;

    info!(path = %path.display(), "Config file saved");
    Ok(())
}

/// Performs an atomic write by writing to a temp file and renaming.
fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    // Create a temp file in the same directory
    let parent = path.parent().unwrap_or(Path::new("."));
    let temp_path = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config"),
        std::process::id()
    ));

    debug!(temp_path = %temp_path.display(), "Writing to temp file");

    // Write to temp file
    fs::write(&temp_path, contents)?;

    // Sync to disk
    let file = fs::File::open(&temp_path)?;
    file.sync_all()?;

    // Rename atomically
    fs::rename(&temp_path, path)?;

    debug!(path = %path.display(), "Atomic write completed");
    Ok(())
}

/// Creates a default config file if it doesn't exist.
///
/// Returns Ok(true) if a new file was created, Ok(false) if it already existed.
pub fn ensure_config_file() -> ConfigResult<bool> {
    ensure_config_dir()?;
    let path = default_config_path().ok_or(ConfigFileError::NoConfigDirectory)?;

    if path.exists() {
        debug!(path = %path.display(), "Config file already exists");
        Ok(false)
    } else {
        save_config(&Config::default())?;
        Ok(true)
    }
}

/// Represents the config file manager for PhotonCast.
pub struct ConfigManager {
    /// Path to the config file.
    path: PathBuf,
    /// Loaded configuration.
    config: Config,
    /// Whether the config has unsaved changes.
    dirty: bool,
}

impl ConfigManager {
    /// Creates a new config manager with the default path.
    pub fn new() -> ConfigResult<Self> {
        let path = default_config_path().ok_or(ConfigFileError::NoConfigDirectory)?;
        Self::from_path(path)
    }

    /// Creates a config manager from a specific path.
    pub fn from_path(path: PathBuf) -> ConfigResult<Self> {
        let config = load_config_from(&path)?;
        Ok(Self {
            path,
            config,
            dirty: false,
        })
    }

    /// Returns a reference to the current configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a mutable reference to the configuration.
    ///
    /// Note: This marks the config as dirty.
    pub fn config_mut(&mut self) -> &mut Config {
        self.dirty = true;
        &mut self.config
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
        self.dirty = true;
    }

    /// Returns whether there are unsaved changes.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Reloads the configuration from disk.
    pub fn reload(&mut self) -> ConfigResult<()> {
        self.config = load_config_from(&self.path)?;
        self.dirty = false;
        Ok(())
    }

    /// Saves the configuration to disk.
    pub fn save(&mut self) -> ConfigResult<()> {
        save_config_to(&self.config, &self.path)?;
        self.dirty = false;
        Ok(())
    }

    /// Saves if there are unsaved changes.
    pub fn save_if_dirty(&mut self) -> ConfigResult<bool> {
        if self.dirty {
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Returns the config file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_config_path() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        (temp_dir, config_path)
    }

    #[test]
    fn test_default_config_dir() {
        let dir = default_config_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.ends_with(CONFIG_DIR_NAME));
    }

    #[test]
    fn test_default_config_path() {
        let path = default_config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with(CONFIG_FILE_NAME));
    }

    #[test]
    fn test_load_config_creates_default() {
        let (_temp_dir, config_path) = create_temp_config_path();

        assert!(!config_path.exists());

        let config = load_config_from(&config_path).unwrap();

        assert!(config_path.exists());
        assert_eq!(config.general.max_results, 10); // Default value
    }

    #[test]
    fn test_save_and_load_config() {
        let (_temp_dir, config_path) = create_temp_config_path();

        let mut config = Config::default();
        config.general.max_results = 42;
        config.general.launch_at_login = true;

        save_config_to(&config, &config_path).unwrap();

        let loaded = load_config_from(&config_path).unwrap();

        assert_eq!(loaded.general.max_results, 42);
        assert!(loaded.general.launch_at_login);
    }

    #[test]
    fn test_atomic_write() {
        let (_temp_dir, config_path) = create_temp_config_path();

        let contents = b"test contents";
        atomic_write(&config_path, contents).unwrap();

        assert!(config_path.exists());
        assert_eq!(fs::read(&config_path).unwrap(), contents);
    }

    #[test]
    fn test_config_manager_new() {
        let (_temp_dir, config_path) = create_temp_config_path();

        let manager = ConfigManager::from_path(config_path).unwrap();

        assert!(!manager.is_dirty());
        assert_eq!(manager.config().general.max_results, 10);
    }

    #[test]
    fn test_config_manager_modify_marks_dirty() {
        let (_temp_dir, config_path) = create_temp_config_path();

        let mut manager = ConfigManager::from_path(config_path).unwrap();

        assert!(!manager.is_dirty());

        manager.config_mut().general.max_results = 50;

        assert!(manager.is_dirty());
    }

    #[test]
    fn test_config_manager_save_clears_dirty() {
        let (_temp_dir, config_path) = create_temp_config_path();

        let mut manager = ConfigManager::from_path(config_path).unwrap();
        manager.config_mut().general.max_results = 50;

        assert!(manager.is_dirty());

        manager.save().unwrap();

        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_config_manager_save_if_dirty() {
        let (_temp_dir, config_path) = create_temp_config_path();

        let mut manager = ConfigManager::from_path(config_path.clone()).unwrap();

        // Not dirty, should return false
        assert!(!manager.save_if_dirty().unwrap());

        // Modify to make dirty
        manager.config_mut().general.max_results = 75;

        // Now dirty, should save and return true
        assert!(manager.save_if_dirty().unwrap());

        // No longer dirty
        assert!(!manager.is_dirty());

        // Verify the saved value
        let reloaded = load_config_from(&config_path).unwrap();
        assert_eq!(reloaded.general.max_results, 75);
    }

    #[test]
    fn test_config_manager_reload() {
        let (_temp_dir, config_path) = create_temp_config_path();

        let mut manager = ConfigManager::from_path(config_path.clone()).unwrap();

        // Modify in memory
        manager.config_mut().general.max_results = 100;

        // Write a different value directly to the file
        let mut other_config = Config::default();
        other_config.general.max_results = 200;
        save_config_to(&other_config, &config_path).unwrap();

        // Reload should get the file value
        manager.reload().unwrap();

        assert_eq!(manager.config().general.max_results, 200);
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_ensure_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("photoncast");
        let config_path = config_dir.join("config.toml");

        // Create the config directory structure
        fs::create_dir_all(&config_dir).unwrap();

        // Test that we can save to this path
        save_config_to(&Config::default(), &config_path).unwrap();

        assert!(config_path.exists());
    }

    #[test]
    fn test_config_file_error_display() {
        let error = ConfigFileError::NoConfigDirectory;
        assert!(!error.to_string().is_empty());

        let error = ConfigFileError::ParseError(toml::from_str::<Config>("invalid").unwrap_err());
        assert!(error.to_string().contains("parse"));
    }

    #[test]
    fn test_parse_error_for_invalid_toml() {
        let (_temp_dir, config_path) = create_temp_config_path();

        // Write invalid TOML
        fs::write(&config_path, "invalid [ toml content").unwrap();

        let result = load_config_from(&config_path);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigFileError::ParseError(_))));
    }
}
