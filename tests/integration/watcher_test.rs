//! Integration tests for the filesystem watcher.

use std::path::PathBuf;
use std::time::Duration;

use photoncast_core::indexer::{AppWatcher, WatchEvent, WatcherConfig};
use tempfile::TempDir;
use tokio::time::timeout;

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

/// Removes a mock application bundle.
fn remove_mock_app(app_path: &std::path::Path) {
    std::fs::remove_dir_all(app_path).expect("failed to remove app bundle");
}

#[test]
fn test_watcher_config_default() {
    let config = WatcherConfig::default();

    assert_eq!(config.debounce_ms, 500);
    assert!(!config.watch_paths.is_empty());
}

#[test]
fn test_app_watcher_creation() {
    let watcher = AppWatcher::new();
    assert!(!watcher.is_running());
}

#[test]
fn test_app_watcher_with_custom_config() {
    let config = WatcherConfig {
        debounce_ms: 100,
        watch_paths: vec![PathBuf::from("/tmp")],
    };
    let watcher = AppWatcher::with_config(config);
    assert!(!watcher.is_running());
}

#[tokio::test]
async fn test_watcher_detects_app_install() {
    let temp_dir = TempDir::new().unwrap();

    let config = WatcherConfig {
        debounce_ms: 100, // Short debounce for testing
        watch_paths: vec![temp_dir.path().to_path_buf()],
    };

    let mut watcher = AppWatcher::with_config(config);
    let mut rx = watcher.start().expect("failed to start watcher");

    // Give the watcher time to initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Create a new app
    let app_path = create_mock_app(temp_dir.path(), "NewApp", "com.test.newapp");

    // Wait for the event (with timeout)
    let event = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("channel closed unexpectedly");

    assert_eq!(event, WatchEvent::AppAdded(app_path));

    watcher.stop();
}

#[tokio::test]
async fn test_watcher_detects_app_removal() {
    let temp_dir = TempDir::new().unwrap();

    // Create app before starting watcher
    let app_path = create_mock_app(temp_dir.path(), "ExistingApp", "com.test.existing");

    let config = WatcherConfig {
        debounce_ms: 100,
        watch_paths: vec![temp_dir.path().to_path_buf()],
    };

    let mut watcher = AppWatcher::with_config(config);
    let mut rx = watcher.start().expect("failed to start watcher");

    // Give the watcher time to initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Remove the app
    remove_mock_app(&app_path);

    // Wait for the event (with timeout)
    let event = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("channel closed unexpectedly");

    assert_eq!(event, WatchEvent::AppRemoved(app_path));

    watcher.stop();
}

#[tokio::test]
async fn test_watcher_filters_non_apps() {
    let temp_dir = TempDir::new().unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        watch_paths: vec![temp_dir.path().to_path_buf()],
    };

    let mut watcher = AppWatcher::with_config(config);
    let mut rx = watcher.start().expect("failed to start watcher");

    // Give the watcher time to initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Create a non-app file
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "test content").expect("failed to write file");

    // Then create an app
    let app_path = create_mock_app(temp_dir.path(), "RealApp", "com.test.realapp");

    // The first event should be for the app, not the text file
    let event = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("channel closed unexpectedly");

    // Should only receive the app event
    assert_eq!(event, WatchEvent::AppAdded(app_path));

    watcher.stop();
}

#[tokio::test]
async fn test_watcher_debouncing() {
    let temp_dir = TempDir::new().unwrap();

    // Create app before starting watcher
    let app_path = create_mock_app(temp_dir.path(), "TestApp", "com.test.app");
    let info_plist_path = app_path.join("Contents/Info.plist");

    let config = WatcherConfig {
        debounce_ms: 300, // Longer debounce for testing
        watch_paths: vec![temp_dir.path().to_path_buf()],
    };

    let mut watcher = AppWatcher::with_config(config);
    let mut rx = watcher.start().expect("failed to start watcher");

    // Give the watcher time to initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Make multiple rapid modifications
    for i in 0..5 {
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>TestApp</string>
    <key>CFBundleIdentifier</key>
    <string>com.test.app</string>
    <key>CFBundleVersion</key>
    <string>1.{i}</string>
</dict>
</plist>"#
        );
        std::fs::write(&info_plist_path, content).expect("failed to write Info.plist");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Wait for debounced event
    let event = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("channel closed unexpectedly");

    // Should only receive one event after debounce
    assert_eq!(event.path(), app_path.as_path());

    // Try to receive another event - should timeout since events were coalesced
    let maybe_event = timeout(Duration::from_millis(500), rx.recv()).await;
    assert!(
        maybe_event.is_err() || maybe_event.unwrap().is_none(),
        "Expected no more events after debouncing"
    );

    watcher.stop();
}

#[tokio::test]
async fn test_watcher_stop() {
    let temp_dir = TempDir::new().unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        watch_paths: vec![temp_dir.path().to_path_buf()],
    };

    let mut watcher = AppWatcher::with_config(config);
    let _rx = watcher.start().expect("failed to start watcher");

    assert!(watcher.is_running());

    watcher.stop();

    assert!(!watcher.is_running());
}

#[tokio::test]
async fn test_watcher_handles_multiple_apps() {
    let temp_dir = TempDir::new().unwrap();

    let config = WatcherConfig {
        debounce_ms: 100,
        watch_paths: vec![temp_dir.path().to_path_buf()],
    };

    let mut watcher = AppWatcher::with_config(config);
    let mut rx = watcher.start().expect("failed to start watcher");

    // Give the watcher time to initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Create multiple apps with delays between them (to ensure separate debounce windows)
    let app1_path = create_mock_app(temp_dir.path(), "App1", "com.test.app1");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let app2_path = create_mock_app(temp_dir.path(), "App2", "com.test.app2");
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Collect events
    let mut events = Vec::new();
    while let Ok(Some(event)) = timeout(Duration::from_secs(2), rx.recv()).await {
        events.push(event);
        if events.len() >= 2 {
            break;
        }
    }

    // Should have received events for both apps
    assert_eq!(events.len(), 2);

    let paths: Vec<_> = events.iter().map(WatchEvent::path).collect();
    assert!(paths.contains(&app1_path.as_path()));
    assert!(paths.contains(&app2_path.as_path()));

    watcher.stop();
}

#[test]
fn test_watch_event_path_accessor() {
    let path = PathBuf::from("/Applications/Test.app");

    let added = WatchEvent::AppAdded(path.clone());
    assert_eq!(added.path(), path);

    let modified = WatchEvent::AppModified(path.clone());
    assert_eq!(modified.path(), path);

    let removed = WatchEvent::AppRemoved(path.clone());
    assert_eq!(removed.path(), path);
}

#[test]
fn test_watch_event_equality() {
    let path = PathBuf::from("/Applications/Test.app");

    assert_eq!(
        WatchEvent::AppAdded(path.clone()),
        WatchEvent::AppAdded(path.clone())
    );

    assert_ne!(
        WatchEvent::AppAdded(path.clone()),
        WatchEvent::AppModified(path.clone())
    );

    assert_ne!(
        WatchEvent::AppAdded(path.clone()),
        WatchEvent::AppRemoved(path)
    );
}
