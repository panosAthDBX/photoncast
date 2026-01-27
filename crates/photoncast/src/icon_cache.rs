//! Shared icon cache helpers for extracting and caching app icons.
//!
//! Provides a cache directory under `~/Library/Caches/PhotonCast/icons`
//! (via `directories::ProjectDirs`) and hashes each app path to produce
//! a deterministic `<hash>.png` filename.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Returns the icon cache directory path.
///
/// Uses `directories::ProjectDirs` when available, falling back to
/// `~/Library/Caches/PhotonCast/icons`.
fn cache_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "PhotonCast").map_or_else(
        || {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Library/Caches/PhotonCast/icons")
        },
        |dirs| dirs.cache_dir().join("icons"),
    )
}

/// Computes the cached icon path for a given app path (hash-based filename).
fn cached_icon_filename(app_path: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    app_path.hash(&mut hasher);
    let hash = hasher.finish();
    cache_dir().join(format!("{hash:x}.png"))
}

/// Checks if an icon is already cached, returns path if so.
///
/// This is fast — just filesystem checks, no extraction.
pub fn get_cached_icon_path(app_path: &Path) -> Option<PathBuf> {
    let cached_path = cached_icon_filename(app_path);

    if cached_path.exists() {
        Some(cached_path)
    } else {
        None
    }
}

/// Clears the cached icon for an app.
pub fn clear_icon(app_path: &Path) {
    let cached_path = cached_icon_filename(app_path);

    if cached_path.exists() {
        if let Err(e) = std::fs::remove_file(&cached_path) {
            tracing::warn!(path = %cached_path.display(), "Failed to remove cached icon: {}", e);
        } else {
            tracing::debug!(path = %cached_path.display(), "Cleared cached icon");
        }
    }
}

/// Extracts an app icon to the cache path.
///
/// Spawns a synchronous `sips` process to convert `.icns` to `.png`.
/// Must be called from a background thread (see [`get_icon_static`]).
pub fn extract_icon(app_path: &Path, cache_path: &Path) -> Option<PathBuf> {
    // Try to find the icon in the app bundle
    let icns_path = app_path.join("Contents/Resources/AppIcon.icns");
    if icns_path.exists() {
        // Use sips to convert icns to png
        let output = std::process::Command::new("sips")
            .args([
                "-s",
                "format",
                "png",
                "-z",
                "64",
                "64",
                &icns_path.to_string_lossy(),
                "--out",
                &cache_path.to_string_lossy(),
            ])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                return Some(cache_path.to_path_buf());
            }
        }
    } else {
        // Try to read Info.plist to find the icon name
        let info_plist = app_path.join("Contents/Info.plist");
        if let Ok(plist) = plist::Value::from_file(&info_plist) {
            if let Some(dict) = plist.as_dictionary() {
                if let Some(icon_name) = dict.get("CFBundleIconFile").and_then(|v| v.as_string()) {
                    let icon_name = if Path::new(icon_name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("icns"))
                    {
                        icon_name.to_string()
                    } else {
                        format!("{icon_name}.icns")
                    };
                    let icon_path = app_path.join("Contents/Resources").join(&icon_name);
                    if icon_path.exists() {
                        // Use sips to convert icns to png
                        let output = std::process::Command::new("sips")
                            .args([
                                "-s",
                                "format",
                                "png",
                                "-z",
                                "64",
                                "64",
                                &icon_path.to_string_lossy(),
                                "--out",
                                &cache_path.to_string_lossy(),
                            ])
                            .output();

                        if let Ok(output) = output {
                            if output.status.success() {
                                return Some(cache_path.to_path_buf());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Static version of [`get_icon`] for use in async context.
///
/// # Threading Model
///
/// This function performs synchronous I/O (filesystem checks and `sips` process
/// spawning). It must only be called from a background thread — never from the
/// main/UI thread. All current call sites dispatch through
/// `cx.background_executor().spawn()` which satisfies this requirement.
pub fn get_icon_static(app_path: &Path) -> Option<PathBuf> {
    let cache_dir = cache_dir();

    // Ensure cache directory exists
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        tracing::warn!("Failed to create icon cache dir: {}", e);
        return None;
    }

    let cached_path = cached_icon_filename(app_path);

    // Return cached icon if it exists and is fresh
    if cached_path.exists() {
        // Check if app is newer than cached icon
        let app_modified = std::fs::metadata(app_path)
            .ok()
            .and_then(|m| m.modified().ok());
        let cached_modified = std::fs::metadata(&cached_path)
            .ok()
            .and_then(|m| m.modified().ok());

        match (app_modified, cached_modified) {
            (Some(app_time), Some(cache_time)) if cache_time >= app_time => {
                return Some(cached_path);
            },
            _ => {}, // Re-extract if we can't determine freshness
        }
    }

    // Extract icon using platform-specific code
    extract_icon(app_path, &cached_path)
}

/// Gets or extracts the icon for an app bundle as PNG.
///
/// Uses `NSWorkspace` to handle all icon formats including asset catalogs.
pub fn get_icon(app_path: &Path) -> Option<PathBuf> {
    let cache_dir = cache_dir();

    // Ensure cache directory exists
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        tracing::warn!("Failed to create icon cache dir: {}", e);
        return None;
    }

    let cached_path = cached_icon_filename(app_path);

    // Return cached icon if it exists and is fresh
    if cached_path.exists() {
        // Check if app is newer than cached icon
        let app_modified = std::fs::metadata(app_path)
            .ok()
            .and_then(|m| m.modified().ok());
        let cached_modified = std::fs::metadata(&cached_path)
            .ok()
            .and_then(|m| m.modified().ok());

        match (app_modified, cached_modified) {
            (Some(app_time), Some(cache_time)) if cache_time >= app_time => {
                return Some(cached_path);
            },
            _ => {}, // Re-extract if we can't determine freshness
        }
    }

    // Extract icon using NSWorkspace (handles all icon formats)
    if crate::platform::save_app_icon_as_png(app_path, &cached_path, 64) {
        tracing::debug!(
            "Extracted icon for {} -> {}",
            app_path.display(),
            cached_path.display()
        );
        Some(cached_path)
    } else {
        tracing::warn!("Failed to extract icon for {}", app_path.display());
        None
    }
}
