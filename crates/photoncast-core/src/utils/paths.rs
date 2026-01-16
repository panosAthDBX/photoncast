//! Platform path helpers.

use std::path::PathBuf;

/// Returns the PhotonCast data directory.
#[must_use]
pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "PhotonCast")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Library/Application Support/PhotonCast")
        })
}

/// Returns the PhotonCast cache directory.
#[must_use]
pub fn cache_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "PhotonCast")
        .map(|dirs| dirs.cache_dir().to_path_buf())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Library/Caches/PhotonCast")
        })
}

/// Returns the PhotonCast config directory.
#[must_use]
pub fn config_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "PhotonCast")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".config/photoncast")
        })
}

/// Returns the path to the config file.
#[must_use]
pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

/// Returns the path to the database file.
#[must_use]
pub fn database_file() -> PathBuf {
    data_dir().join("photoncast.db")
}

/// Expands a tilde in a path.
#[must_use]
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with('~') {
        dirs::home_dir()
            .map(|h: PathBuf| h.join(&path[2..]))
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}
