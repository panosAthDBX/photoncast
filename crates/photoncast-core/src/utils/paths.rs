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
///
/// Handles:
/// - `"~"` alone → home directory
/// - `"~/something"` → home directory joined with `something`
/// - All other paths → returned unchanged
#[must_use]
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(path))
    } else if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_bare() {
        let result = expand_tilde("~");
        assert!(result.to_str().unwrap() != "~", "bare ~ should expand to home dir");
        assert!(result.is_absolute());
    }

    #[test]
    fn test_expand_tilde_with_path() {
        let result = expand_tilde("~/Documents");
        assert!(result.to_str().unwrap().ends_with("/Documents"));
        assert!(result.is_absolute());
    }

    #[test]
    fn test_expand_tilde_absolute_unchanged() {
        let result = expand_tilde("/usr/local");
        assert_eq!(result, PathBuf::from("/usr/local"));
    }

    #[test]
    fn test_expand_tilde_relative_unchanged() {
        let result = expand_tilde("relative/path");
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_expand_tilde_empty_unchanged() {
        let result = expand_tilde("");
        assert_eq!(result, PathBuf::from(""));
    }
}
