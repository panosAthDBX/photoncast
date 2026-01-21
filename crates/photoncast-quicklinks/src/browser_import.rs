//! Browser bookmark import functionality.

use anyhow::Context;
use std::path::PathBuf;

use crate::error::Result;
use crate::models::QuickLink;

/// Supported browsers for bookmark import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Safari,
    Chrome,
    Firefox,
    Arc,
}

impl Browser {
    /// Returns the default bookmark file path for this browser.
    #[must_use]
    pub fn default_bookmark_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;

        match self {
            Self::Safari => Some(home.join("Library/Safari/Bookmarks.plist")),
            Self::Chrome => {
                Some(home.join("Library/Application Support/Google/Chrome/Default/Bookmarks"))
            },
            Self::Firefox => {
                // Firefox uses a dynamic profile directory
                let profiles_dir = home.join("Library/Application Support/Firefox/Profiles");
                if let Ok(entries) = std::fs::read_dir(profiles_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let places = path.join("places.sqlite");
                            if places.exists() {
                                return Some(places);
                            }
                        }
                    }
                }
                None
            },
            Self::Arc => Some(home.join("Library/Application Support/Arc/StorableSidebar.json")),
        }
    }
}

/// Imports bookmarks from Safari.
///
/// Safari stores bookmarks in a plist file.
pub fn import_safari(path: Option<PathBuf>) -> Result<Vec<QuickLink>> {
    let path = path
        .or_else(|| Browser::Safari.default_bookmark_path())
        .context("Safari bookmarks file not found")?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let plist_data = std::fs::read(&path).context("failed to read Safari bookmarks")?;

    let plist: plist::Value =
        plist::from_bytes(&plist_data).context("failed to parse Safari bookmarks plist")?;

    let mut links = Vec::new();
    extract_safari_bookmarks(&plist, &mut links);

    Ok(links)
}

fn extract_safari_bookmarks(value: &plist::Value, links: &mut Vec<QuickLink>) {
    if let Some(dict) = value.as_dictionary() {
        // Check if this is a bookmark
        if let Some(url_string) = dict.get("URLString").and_then(|v| v.as_string()) {
            if let Some(title) = dict
                .get("URIDictionary")
                .and_then(|v| v.as_dictionary())
                .and_then(|d| d.get("title"))
                .and_then(|v| v.as_string())
                .or_else(|| dict.get("title").and_then(|v| v.as_string()))
            {
                let link = QuickLink::new(title, url_string);
                links.push(link);
            }
        }

        // Check for children (folders)
        if let Some(children) = dict.get("Children").and_then(|v| v.as_array()) {
            for child in children {
                extract_safari_bookmarks(child, links);
            }
        }
    }
}

/// Imports bookmarks from Chrome.
///
/// Chrome stores bookmarks in a JSON file.
pub fn import_chrome(path: Option<PathBuf>) -> Result<Vec<QuickLink>> {
    let path = path
        .or_else(|| Browser::Chrome.default_bookmark_path())
        .context("Chrome bookmarks file not found")?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let json_data = std::fs::read_to_string(&path).context("failed to read Chrome bookmarks")?;

    let json: serde_json::Value =
        serde_json::from_str(&json_data).context("failed to parse Chrome bookmarks JSON")?;

    let mut links = Vec::new();

    // Chrome stores bookmarks in roots.bookmark_bar, roots.other, etc.
    if let Some(roots) = json.get("roots").and_then(|v| v.as_object()) {
        for (_, root) in roots {
            extract_chrome_bookmarks(root, &mut links);
        }
    }

    Ok(links)
}

fn extract_chrome_bookmarks(value: &serde_json::Value, links: &mut Vec<QuickLink>) {
    if let Some(obj) = value.as_object() {
        let type_str = obj.get("type").and_then(|v| v.as_str());

        if type_str == Some("url") {
            if let (Some(title), Some(url)) = (
                obj.get("name").and_then(|v| v.as_str()),
                obj.get("url").and_then(|v| v.as_str()),
            ) {
                let link = QuickLink::new(title, url);
                links.push(link);
            }
        }

        // Recurse into children (folders)
        if let Some(children) = obj.get("children").and_then(|v| v.as_array()) {
            for child in children {
                extract_chrome_bookmarks(child, links);
            }
        }
    }
}

/// Imports bookmarks from Firefox.
///
/// Firefox stores bookmarks in a SQLite database (places.sqlite).
pub fn import_firefox(path: Option<PathBuf>) -> Result<Vec<QuickLink>> {
    let path = path
        .or_else(|| Browser::Firefox.default_bookmark_path())
        .context("Firefox places.sqlite not found")?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    // Open Firefox database in read-only mode
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .context("failed to open Firefox database")?;

    let mut stmt = conn.prepare(
        "SELECT mb.title, mp.url
         FROM moz_bookmarks mb
         JOIN moz_places mp ON mb.fk = mp.id
         WHERE mb.type = 1 AND mb.title IS NOT NULL",
    )?;

    let links = stmt
        .query_map([], |row| {
            let title: String = row.get(0)?;
            let url: String = row.get(1)?;
            Ok(QuickLink::new(title, url))
        })?
        .filter_map(std::result::Result::ok)
        .collect();

    Ok(links)
}

/// Imports bookmarks from Arc browser.
///
/// Arc stores sidebar items in a JSON file.
pub fn import_arc(path: Option<PathBuf>) -> Result<Vec<QuickLink>> {
    let path = path
        .or_else(|| Browser::Arc.default_bookmark_path())
        .context("Arc StorableSidebar.json not found")?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let json_data = std::fs::read_to_string(&path).context("failed to read Arc sidebar data")?;

    let json: serde_json::Value =
        serde_json::from_str(&json_data).context("failed to parse Arc sidebar JSON")?;

    let mut links = Vec::new();

    // Arc's structure varies, but typically has sidebar items
    if let Some(sidebar) = json.get("sidebar") {
        extract_arc_items(sidebar, &mut links);
    }

    // Also check containers
    if let Some(containers) = json.get("containers").and_then(|v| v.as_array()) {
        for container in containers {
            extract_arc_items(container, &mut links);
        }
    }

    Ok(links)
}

fn extract_arc_items(value: &serde_json::Value, links: &mut Vec<QuickLink>) {
    if let Some(obj) = value.as_object() {
        // Check if this is a tab/bookmark item
        if let (Some(title), Some(url)) = (
            obj.get("title").and_then(|v| v.as_str()),
            obj.get("url").and_then(|v| v.as_str()),
        ) {
            let link = QuickLink::new(title, url);
            links.push(link);
        }

        // Recurse into nested structures
        for (_, val) in obj {
            if val.is_array() {
                if let Some(arr) = val.as_array() {
                    for item in arr {
                        extract_arc_items(item, links);
                    }
                }
            } else if val.is_object() {
                extract_arc_items(val, links);
            }
        }
    }
}

/// Imports bookmarks from all supported browsers.
pub fn import_all_browsers() -> Vec<(Browser, Result<Vec<QuickLink>>)> {
    vec![
        (Browser::Safari, import_safari(None)),
        (Browser::Chrome, import_chrome(None)),
        (Browser::Firefox, import_firefox(None)),
        (Browser::Arc, import_arc(None)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_paths() {
        // Just verify the paths are constructed correctly
        assert!(Browser::Safari.default_bookmark_path().is_some());
        assert!(Browser::Chrome.default_bookmark_path().is_some());
        assert!(Browser::Arc.default_bookmark_path().is_some());
        // Firefox path may or may not exist
        let _ = Browser::Firefox.default_bookmark_path();
    }

    #[test]
    fn test_import_nonexistent_file() {
        let result = import_safari(Some(PathBuf::from("/tmp/nonexistent.plist")));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
