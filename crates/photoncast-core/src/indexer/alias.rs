//! Alias and symlink resolution for macOS applications.
//!
//! This module handles resolving:
//! - Unix symlinks to .app bundles
//! - macOS Finder aliases (bookmark-based) to .app bundles

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, trace};

/// Result of resolving an alias or symlink.
#[derive(Debug, Clone)]
pub struct ResolvedPath {
    /// The original path (alias or symlink).
    pub original: PathBuf,
    /// The resolved target path.
    pub target: PathBuf,
    /// Whether this was an alias (vs a regular path or symlink).
    pub was_alias: bool,
}

impl ResolvedPath {
    /// Creates a new resolved path for a non-alias regular path.
    #[must_use]
    pub fn direct(path: PathBuf) -> Self {
        Self {
            original: path.clone(),
            target: path,
            was_alias: false,
        }
    }

    /// Creates a new resolved path for an alias or symlink.
    #[must_use]
    pub fn resolved(original: PathBuf, target: PathBuf) -> Self {
        Self {
            original,
            target,
            was_alias: true,
        }
    }
}

/// Resolves a path that might be a symlink or macOS alias.
///
/// This function handles:
/// 1. Unix symlinks (using `std::fs::canonicalize`)
/// 2. macOS Finder aliases (using NSURL bookmark APIs)
///
/// If the path is neither a symlink nor an alias, returns the original path.
///
/// # Arguments
///
/// * `path` - The path to resolve
///
/// # Returns
///
/// The resolved path information, or an error if resolution fails.
pub fn resolve_path(path: &Path) -> Result<ResolvedPath> {
    // First, try to resolve as a Unix symlink
    if path.is_symlink() {
        let target = std::fs::read_link(path)
            .with_context(|| format!("failed to read symlink: {}", path.display()))?;

        // Handle relative symlinks
        let resolved = if target.is_relative() {
            path.parent().map(|p| p.join(&target)).unwrap_or(target)
        } else {
            target
        };

        // Canonicalize to get the absolute path
        let canonical = std::fs::canonicalize(&resolved).with_context(|| {
            format!(
                "failed to canonicalize symlink target: {}",
                resolved.display()
            )
        })?;

        debug!(
            "Resolved symlink: {} -> {}",
            path.display(),
            canonical.display()
        );

        return Ok(ResolvedPath::resolved(path.to_path_buf(), canonical));
    }

    // Try to resolve as a macOS Finder alias
    #[cfg(target_os = "macos")]
    if let Some(resolved) = resolve_macos_alias(path) {
        debug!(
            "Resolved macOS alias: {} -> {}",
            path.display(),
            resolved.display()
        );
        return Ok(ResolvedPath::resolved(path.to_path_buf(), resolved));
    }

    // Not a symlink or alias - return the original path
    trace!("Path is not a symlink or alias: {}", path.display());
    Ok(ResolvedPath::direct(path.to_path_buf()))
}

/// Resolves the path fully, following all symlinks and aliases.
///
/// This is useful for deduplication - the canonical path can be used
/// to identify unique applications even if they're linked from multiple locations.
///
/// # Arguments
///
/// * `path` - The path to canonicalize
///
/// # Returns
///
/// The fully resolved canonical path.
pub fn canonical_path(path: &Path) -> Result<PathBuf> {
    // First resolve any alias (which std::fs::canonicalize won't handle)
    let resolved = resolve_path(path)?;

    // Then canonicalize to resolve any remaining symlinks and get absolute path
    std::fs::canonicalize(&resolved.target)
        .with_context(|| format!("failed to canonicalize path: {}", resolved.target.display()))
}

/// Checks if a path is a macOS Finder alias (not a Unix symlink).
#[cfg(target_os = "macos")]
pub fn is_macos_alias(path: &Path) -> bool {
    // A macOS alias is a regular file (not a symlink) that contains bookmark data.
    // We can check this by trying to resolve it as an alias.
    if path.is_symlink() {
        return false;
    }

    if !path.is_file() {
        return false;
    }

    // Try to resolve it - if it succeeds and returns a different path, it's an alias
    resolve_macos_alias(path).is_some()
}

#[cfg(not(target_os = "macos"))]
pub fn is_macos_alias(_path: &Path) -> bool {
    false
}

/// Resolves a macOS Finder alias to its target path.
///
/// Uses NSURL's `URLByResolvingAliasFileAtURL:options:error:` method.
///
/// # Returns
///
/// `Some(path)` if the file is an alias and was resolved successfully,
/// `None` if the file is not an alias or resolution failed.
#[cfg(target_os = "macos")]
fn resolve_macos_alias(path: &Path) -> Option<PathBuf> {
    use objc2_foundation::{NSString, NSURLBookmarkResolutionOptions, NSURL};

    // Convert path to NSURL
    let path_str = path.to_string_lossy();
    let ns_path = NSString::from_str(&path_str);

    // Create file URL
    let url = NSURL::fileURLWithPath(&ns_path);

    // Try to resolve as alias
    // Options: WithoutUI | WithoutMounting
    let options =
        NSURLBookmarkResolutionOptions::WithoutUI | NSURLBookmarkResolutionOptions::WithoutMounting;

    let resolved = NSURL::URLByResolvingAliasFileAtURL_options_error(&url, options);

    match resolved {
        Ok(resolved_url) => {
            // Get the path from the resolved URL
            let resolved_path = resolved_url.path();
            resolved_path.and_then(|p| {
                let resolved = PathBuf::from(p.to_string());
                // Only consider it an alias if the resolved path is different from the original.
                // URLByResolvingAliasFileAtURL can succeed for regular files/directories,
                // returning the same path (or its canonical form).
                let original_canonical = std::fs::canonicalize(path).ok();
                let resolved_canonical = std::fs::canonicalize(&resolved).ok();

                match (original_canonical, resolved_canonical) {
                    (Some(orig), Some(res)) if orig != res => Some(resolved),
                    _ => None,
                }
            })
        },
        Err(_) => {
            // This is expected for non-alias files, so only log at trace level
            trace!(
                "Could not resolve as alias (likely not an alias): {}",
                path.display()
            );
            None
        },
    }
}

#[cfg(not(target_os = "macos"))]
fn resolve_macos_alias(_path: &Path) -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_direct_path() {
        let temp_dir = TempDir::new().unwrap();
        let app_path = temp_dir.path().join("Test.app");
        fs::create_dir(&app_path).unwrap();

        let result = resolve_path(&app_path).unwrap();
        assert!(!result.was_alias);
        assert_eq!(result.original, app_path);
        // Target should be the canonicalized version of the path
        assert!(result.target.to_string_lossy().contains("Test.app"));
    }

    #[test]
    #[cfg(unix)]
    fn test_resolve_symlink() {
        let temp_dir = TempDir::new().unwrap();

        // Create a fake .app directory
        let app_path = temp_dir.path().join("RealApp.app");
        fs::create_dir(&app_path).unwrap();

        // Create a symlink to it
        let link_path = temp_dir.path().join("LinkedApp.app");
        std::os::unix::fs::symlink(&app_path, &link_path).unwrap();

        let result = resolve_path(&link_path).unwrap();
        assert!(result.was_alias);
        assert_eq!(result.original, link_path);

        // The target should resolve to the real app
        let canonical_app = fs::canonicalize(&app_path).unwrap();
        assert_eq!(result.target, canonical_app);
    }

    #[test]
    #[cfg(unix)]
    fn test_resolve_relative_symlink() {
        let temp_dir = TempDir::new().unwrap();

        // Create a fake .app directory
        let app_path = temp_dir.path().join("RealApp.app");
        fs::create_dir(&app_path).unwrap();

        // Create a relative symlink (pointing to "./RealApp.app")
        let link_path = temp_dir.path().join("RelativeLink.app");
        std::os::unix::fs::symlink(Path::new("RealApp.app"), &link_path).unwrap();

        let result = resolve_path(&link_path).unwrap();
        assert!(result.was_alias);

        // The target should resolve to the real app
        let canonical_app = fs::canonicalize(&app_path).unwrap();
        assert_eq!(result.target, canonical_app);
    }

    #[test]
    fn test_canonical_path_regular() {
        let temp_dir = TempDir::new().unwrap();
        let app_path = temp_dir.path().join("Test.app");
        fs::create_dir(&app_path).unwrap();

        let canonical = canonical_path(&app_path).unwrap();
        assert!(canonical.is_absolute());
        assert!(canonical.to_string_lossy().contains("Test.app"));
    }

    #[test]
    #[cfg(unix)]
    fn test_canonical_path_through_symlink() {
        let temp_dir = TempDir::new().unwrap();

        let app_path = temp_dir.path().join("RealApp.app");
        fs::create_dir(&app_path).unwrap();

        let link_path = temp_dir.path().join("LinkedApp.app");
        std::os::unix::fs::symlink(&app_path, &link_path).unwrap();

        let canonical = canonical_path(&link_path).unwrap();
        let expected = fs::canonicalize(&app_path).unwrap();
        assert_eq!(canonical, expected);
    }

    #[test]
    fn test_resolved_path_direct() {
        let path = PathBuf::from("/Applications/Safari.app");
        let resolved = ResolvedPath::direct(path.clone());

        assert!(!resolved.was_alias);
        assert_eq!(resolved.original, path);
        assert_eq!(resolved.target, path);
    }

    #[test]
    fn test_resolved_path_resolved() {
        let original = PathBuf::from("/Users/test/Desktop/Safari.app");
        let target = PathBuf::from("/Applications/Safari.app");
        let resolved = ResolvedPath::resolved(original.clone(), target.clone());

        assert!(resolved.was_alias);
        assert_eq!(resolved.original, original);
        assert_eq!(resolved.target, target);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_is_macos_alias_non_macos() {
        let path = PathBuf::from("/some/path");
        assert!(!is_macos_alias(&path));
    }
}
