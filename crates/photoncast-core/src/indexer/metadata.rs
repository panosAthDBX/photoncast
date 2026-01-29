//! Info.plist parsing for application metadata.

use std::path::Path;

use anyhow::{Context, Result};
use tracing::debug;

use crate::indexer::{AppBundleId, AppCategory, IndexedApp};

async fn read_file(path: &Path) -> Result<Vec<u8>> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Ok(tokio::fs::read(path).await?);
    }

    Ok(std::fs::read(path)?)
}

async fn file_metadata(path: &Path) -> Result<std::fs::Metadata> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Ok(tokio::fs::metadata(path).await?);
    }

    Ok(std::fs::metadata(path)?)
}

/// Parses application metadata from an Info.plist file.
///
/// # Arguments
///
/// * `app_path` - Path to the .app bundle.
///
/// # Errors
///
/// Returns an error if the Info.plist cannot be read or parsed.
#[allow(clippy::map_unwrap_or)]
pub async fn parse_app_metadata(app_path: &Path) -> Result<IndexedApp> {
    let info_plist_path = app_path.join("Contents/Info.plist");

    let contents = read_file(&info_plist_path)
        .await
        .with_context(|| format!("failed to read Info.plist at {}", info_plist_path.display()))?;

    let plist_value: plist::Value = plist::from_bytes(&contents)
        .with_context(|| format!("failed to parse Info.plist for {}", app_path.display()))?;

    let dict = plist_value
        .as_dictionary()
        .context("Info.plist is not a dictionary")?;

    // Extract name (try multiple keys)
    let name = dict
        .get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(plist::Value::as_string)
        .map(String::from)
        .unwrap_or_else(|| {
            app_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });

    // Extract bundle ID (required)
    let bundle_id = dict
        .get("CFBundleIdentifier")
        .and_then(plist::Value::as_string)
        .map(|s| AppBundleId::new(s.to_string()))
        .context("missing CFBundleIdentifier")?;

    // Extract category
    let category = dict
        .get("LSApplicationCategoryType")
        .and_then(plist::Value::as_string)
        .map(AppCategory::from_plist_value);

    // Get last modified time
    let metadata = file_metadata(app_path).await?;
    let last_modified = metadata.modified().map_or_else(
        |_| chrono::Utc::now(),
        chrono::DateTime::<chrono::Utc>::from,
    );

    debug!(
        "Parsed metadata for {}: bundle_id={}, category={:?}",
        name, bundle_id, category
    );

    Ok(IndexedApp {
        name,
        bundle_id,
        path: app_path.to_path_buf(),
        icon_path: None, // Set later by icon extraction
        category,
        keywords: Vec::new(),
        last_modified,
    })
}

/// Parses a raw Info.plist bytes and extracts metadata synchronously.
/// Useful for testing without filesystem access.
pub fn parse_plist_metadata(
    plist_bytes: &[u8],
    app_name: &str,
) -> Result<(String, String, Option<AppCategory>)> {
    let plist_value: plist::Value =
        plist::from_bytes(plist_bytes).context("failed to parse Info.plist")?;

    let dict = plist_value
        .as_dictionary()
        .context("Info.plist is not a dictionary")?;

    // Extract name
    let name = dict
        .get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(plist::Value::as_string)
        .map_or_else(|| app_name.to_string(), String::from);

    // Extract bundle ID
    let bundle_id = dict
        .get("CFBundleIdentifier")
        .and_then(plist::Value::as_string)
        .map(String::from)
        .context("missing CFBundleIdentifier")?;

    // Extract category
    let category = dict
        .get("LSApplicationCategoryType")
        .and_then(plist::Value::as_string)
        .map(AppCategory::from_plist_value);

    Ok((name, bundle_id, category))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_plist(name: &str, bundle_id: &str, category: Option<&str>) -> Vec<u8> {
        let mut plist = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>"#,
        );
        plist.push_str(name);
        plist.push_str(
            r"</string>
    <key>CFBundleIdentifier</key>
    <string>",
        );
        plist.push_str(bundle_id);
        plist.push_str("</string>\n");

        if let Some(cat) = category {
            plist.push_str("    <key>LSApplicationCategoryType</key>\n");
            plist.push_str("    <string>");
            plist.push_str(cat);
            plist.push_str("</string>\n");
        }

        plist.push_str(
            r"</dict>
</plist>",
        );

        plist.into_bytes()
    }

    #[test]
    fn test_parse_plist_basic() {
        let plist = create_test_plist("TestApp", "com.test.app", None);
        let (name, bundle_id, category) = parse_plist_metadata(&plist, "FallbackName").unwrap();

        assert_eq!(name, "TestApp");
        assert_eq!(bundle_id, "com.test.app");
        assert!(category.is_none());
    }

    #[test]
    fn test_parse_plist_with_category() {
        let plist = create_test_plist(
            "DevApp",
            "com.dev.app",
            Some("public.app-category.developer-tools"),
        );
        let (name, bundle_id, category) = parse_plist_metadata(&plist, "FallbackName").unwrap();

        assert_eq!(name, "DevApp");
        assert_eq!(bundle_id, "com.dev.app");
        assert_eq!(category, Some(AppCategory::DeveloperTools));
    }

    #[test]
    fn test_parse_plist_missing_bundle_id() {
        let plist = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>TestApp</string>
</dict>
</plist>"#;

        let result = parse_plist_metadata(plist, "FallbackName");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing CFBundleIdentifier"));
    }

    #[test]
    fn test_parse_plist_display_name_priority() {
        let plist = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>Display Name</string>
    <key>CFBundleName</key>
    <string>Bundle Name</string>
    <key>CFBundleIdentifier</key>
    <string>com.test.app</string>
</dict>
</plist>"#;

        let (name, _, _) = parse_plist_metadata(plist, "FallbackName").unwrap();
        // CFBundleDisplayName should take priority over CFBundleName
        assert_eq!(name, "Display Name");
    }

    #[test]
    fn test_app_category_parsing() {
        assert_eq!(
            AppCategory::from_plist_value("public.app-category.developer-tools"),
            AppCategory::DeveloperTools
        );
        assert_eq!(
            AppCategory::from_plist_value("public.app-category.entertainment"),
            AppCategory::Entertainment
        );
        assert_eq!(
            AppCategory::from_plist_value("public.app-category.productivity"),
            AppCategory::Productivity
        );
        assert_eq!(
            AppCategory::from_plist_value("public.app-category.utilities"),
            AppCategory::Utilities
        );
        assert_eq!(
            AppCategory::from_plist_value("unknown-category"),
            AppCategory::Other("unknown-category".to_string())
        );
    }
}
