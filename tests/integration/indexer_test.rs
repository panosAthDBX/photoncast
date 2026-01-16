//! Integration tests for the application indexer.

use std::path::PathBuf;

use photoncast_core::indexer::{parse_app_metadata, AppScanner, IconCache};
use tempfile::TempDir;

/// Creates a mock application bundle in the given directory.
fn create_mock_app(dir: &std::path::Path, name: &str, bundle_id: &str) -> PathBuf {
    let app_path = dir.join(format!("{name}.app"));
    let contents_path = app_path.join("Contents");
    let resources_path = contents_path.join("Resources");

    std::fs::create_dir_all(&resources_path).expect("failed to create app directories");

    // Create Info.plist
    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
</dict>
</plist>"#
    );

    std::fs::write(contents_path.join("Info.plist"), info_plist)
        .expect("failed to write Info.plist");

    app_path
}

/// Creates a mock app bundle with a category.
fn create_mock_app_with_category(
    dir: &std::path::Path,
    name: &str,
    bundle_id: &str,
    category: &str,
) -> PathBuf {
    let app_path = dir.join(format!("{name}.app"));
    let contents_path = app_path.join("Contents");
    let resources_path = contents_path.join("Resources");

    std::fs::create_dir_all(&resources_path).expect("failed to create app directories");

    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>LSApplicationCategoryType</key>
    <string>{category}</string>
</dict>
</plist>"#
    );

    std::fs::write(contents_path.join("Info.plist"), info_plist)
        .expect("failed to write Info.plist");

    app_path
}

#[test]
fn test_scanner_default_paths() {
    let scanner = AppScanner::new();

    // Scanner should be created successfully with non-empty paths
    assert!(!scanner.scan_paths().is_empty());
}

#[test]
fn test_scanner_exclude_prefpane() {
    let scanner = AppScanner::new();

    let path = PathBuf::from("/Library/PreferencePanes/Test.prefPane");
    assert!(scanner.is_excluded(&path));
}

#[test]
fn test_scanner_exclude_uninstaller() {
    let scanner = AppScanner::new();

    let path = PathBuf::from("/Applications/SomeApp Uninstaller.app");
    assert!(scanner.is_excluded(&path));
}

#[test]
fn test_scanner_allow_regular_app() {
    let scanner = AppScanner::new();

    let path = PathBuf::from("/Applications/Safari.app");
    assert!(!scanner.is_excluded(&path));
}

#[tokio::test]
async fn test_parse_mock_app_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let app_path = create_mock_app(temp_dir.path(), "TestApp", "com.test.app");

    let app = parse_app_metadata(&app_path).await.unwrap();

    assert_eq!(app.name, "TestApp");
    assert_eq!(app.bundle_id.as_str(), "com.test.app");
    assert!(app.category.is_none());
}

#[tokio::test]
async fn test_parse_mock_app_with_category() {
    let temp_dir = TempDir::new().unwrap();
    let app_path = create_mock_app_with_category(
        temp_dir.path(),
        "DevTool",
        "com.dev.tool",
        "public.app-category.developer-tools",
    );

    let app = parse_app_metadata(&app_path).await.unwrap();

    assert_eq!(app.name, "DevTool");
    assert_eq!(app.bundle_id.as_str(), "com.dev.tool");
    assert!(app.category.is_some());
}

#[tokio::test]
async fn test_scan_directory_with_mock_apps() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple mock apps
    create_mock_app(temp_dir.path(), "App1", "com.test.app1");
    create_mock_app(temp_dir.path(), "App2", "com.test.app2");
    create_mock_app(temp_dir.path(), "App3", "com.test.app3");

    let scanner = AppScanner::with_paths(vec![temp_dir.path().to_path_buf()]);
    let apps = scanner.scan_all().await.unwrap();

    assert_eq!(apps.len(), 3);

    let bundle_ids: Vec<&str> = apps.iter().map(|a| a.bundle_id.as_str()).collect();
    assert!(bundle_ids.contains(&"com.test.app1"));
    assert!(bundle_ids.contains(&"com.test.app2"));
    assert!(bundle_ids.contains(&"com.test.app3"));
}

#[tokio::test]
async fn test_scan_excludes_uninstallers() {
    let temp_dir = TempDir::new().unwrap();

    create_mock_app(temp_dir.path(), "GoodApp", "com.test.goodapp");
    create_mock_app(temp_dir.path(), "BadApp Uninstaller", "com.test.uninstaller");

    let scanner = AppScanner::with_paths(vec![temp_dir.path().to_path_buf()]);
    let apps = scanner.scan_all().await.unwrap();

    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].bundle_id.as_str(), "com.test.goodapp");
}

#[tokio::test]
async fn test_scan_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let scanner = AppScanner::with_paths(vec![temp_dir.path().to_path_buf()]);
    let apps = scanner.scan_all().await.unwrap();

    assert!(apps.is_empty());
}

#[tokio::test]
async fn test_scan_nonexistent_directory() {
    let scanner = AppScanner::with_paths(vec![PathBuf::from("/nonexistent/path/to/apps")]);
    let apps = scanner.scan_all().await.unwrap();

    // Should return empty vector, not error
    assert!(apps.is_empty());
}

#[tokio::test]
async fn test_icon_cache_operations() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("icons");

    let cache = IconCache::with_dir_and_capacity(cache_dir.clone(), 10);
    cache.init().await.unwrap();

    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_scanner_custom_timeout() {
    use std::time::Duration;

    let scanner = AppScanner::new().with_timeout(Duration::from_secs(5));
    // Scanner should accept custom timeout without error
    assert!(!scanner.scan_paths().is_empty());
}

#[tokio::test]
async fn test_indexed_app_equality() {
    let temp_dir = TempDir::new().unwrap();
    let app_path = create_mock_app(temp_dir.path(), "TestApp", "com.test.app");

    let app1 = parse_app_metadata(&app_path).await.unwrap();
    let app2 = parse_app_metadata(&app_path).await.unwrap();

    // Same app should be equal (ignoring timestamps which may differ slightly)
    assert_eq!(app1.name, app2.name);
    assert_eq!(app1.bundle_id, app2.bundle_id);
    assert_eq!(app1.path, app2.path);
}
