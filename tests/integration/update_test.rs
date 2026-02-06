//! Integration tests for the auto-update system.
//!
//! Task 5.2: Test Auto-Update Flow
//!
//! These tests verify the Sparkle auto-update integration, including:
//! - Manual update check triggers
//! - Appcast feed parsing
//! - Update detection logic
//! - Version comparison
//!
//! # Test Categories
//!
//! - **Appcast Parsing**: Tests XML feed parsing
//! - **Version Comparison**: Tests update detection logic
//! - **Network Mocking**: Tests with mock appcast server
//! - **Configuration**: Tests update settings
//!
//! # Running These Tests
//!
//! ```bash
//! cargo test --test integration -- update_test
//! ```

use photoncast_core::platform::updates::{
    AvailableUpdate, UpdateConfig, UpdateError, UpdateManager, UpdateStatus,
    DEFAULT_CHECK_INTERVAL, DEFAULT_FEED_URL,
};
use std::time::Duration;

// =============================================================================
// Appcast XML Test Data
// =============================================================================

/// Sample valid appcast XML with a newer version
const MOCK_APPCAST_NEWER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
    <channel>
        <title>PhotonCast Changelog</title>
        <item>
            <title>Version 99.0.0</title>
            <pubDate>Mon, 15 Feb 2026 12:00:00 +0000</pubDate>
            <sparkle:version>99.0.0</sparkle:version>
            <sparkle:shortVersionString>99.0.0</sparkle:shortVersionString>
            <description><![CDATA[
                <h2>What's New</h2>
                <ul>
                    <li>Major new features</li>
                    <li>Performance improvements</li>
                </ul>
            ]]></description>
            <enclosure url="https://api.photoncast.app/releases/99.0.0/PhotonCast.dmg"
                       sparkle:edSignature="mock_signature_12345"
                       length="15240000"
                       type="application/octet-stream"/>
            <sparkle:minimumSystemVersion>12.0</sparkle:minimumSystemVersion>
        </item>
    </channel>
</rss>"#;

/// Sample appcast with current version (no update available)
const MOCK_APPCAST_CURRENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
    <channel>
        <title>PhotonCast Changelog</title>
        <item>
            <title>Version 0.1.0</title>
            <pubDate>Mon, 01 Jan 2026 12:00:00 +0000</pubDate>
            <sparkle:version>0.1.0</sparkle:version>
            <sparkle:shortVersionString>0.1.0</sparkle:shortVersionString>
            <description><![CDATA[Initial release]]></description>
            <enclosure url="https://api.photoncast.app/releases/0.1.0/PhotonCast.dmg"
                       sparkle:edSignature="mock_signature_00001"
                       length="10000000"
                       type="application/octet-stream"/>
        </item>
    </channel>
</rss>"#;

/// Sample appcast with multiple versions
const MOCK_APPCAST_MULTIPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
    <channel>
        <title>PhotonCast Changelog</title>
        <item>
            <title>Version 2.0.0</title>
            <pubDate>Mon, 15 Mar 2026 12:00:00 +0000</pubDate>
            <sparkle:version>2.0.0</sparkle:version>
            <sparkle:shortVersionString>2.0.0</sparkle:shortVersionString>
            <description><![CDATA[Version 2.0]]></description>
            <enclosure url="https://api.photoncast.app/releases/2.0.0/PhotonCast.dmg"
                       sparkle:edSignature="sig_2_0_0"
                       length="20000000"
                       type="application/octet-stream"/>
        </item>
        <item>
            <title>Version 1.5.0</title>
            <pubDate>Mon, 15 Feb 2026 12:00:00 +0000</pubDate>
            <sparkle:version>1.5.0</sparkle:version>
            <sparkle:shortVersionString>1.5.0</sparkle:shortVersionString>
            <description><![CDATA[Version 1.5]]></description>
            <enclosure url="https://api.photoncast.app/releases/1.5.0/PhotonCast.dmg"
                       sparkle:edSignature="sig_1_5_0"
                       length="18000000"
                       type="application/octet-stream"/>
        </item>
    </channel>
</rss>"#;

/// Malformed appcast (missing required fields)
const MOCK_APPCAST_INVALID: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Invalid Feed</title>
        <item>
            <title>No version info</title>
        </item>
    </channel>
</rss>"#;

/// Empty appcast
const MOCK_APPCAST_EMPTY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
    <channel>
        <title>PhotonCast Changelog</title>
    </channel>
</rss>"#;

// =============================================================================
// UpdateManager Creation Tests
// =============================================================================

#[tokio::test]
async fn test_update_manager_default_configuration() {
    let manager = UpdateManager::new();

    assert_eq!(manager.feed_url().await, DEFAULT_FEED_URL);
    assert!(manager.auto_check_enabled().await);
    assert_eq!(manager.status().await, UpdateStatus::Uninitialized);
}

#[tokio::test]
async fn test_update_manager_custom_feed_url() {
    let custom_url = "https://custom.example.com/updates/appcast.xml";
    let manager = UpdateManager::with_feed_url(custom_url);

    assert_eq!(manager.feed_url().await, custom_url);
}

#[tokio::test]
async fn test_update_manager_custom_config() {
    let config = UpdateConfig {
        feed_url: "https://test.example.com/feed.xml".to_string(),
        auto_check_enabled: false,
        check_interval: Duration::from_secs(3600),
        auto_download: true,
        include_beta: true,
    };

    let manager = UpdateManager::with_config(config);

    assert_eq!(
        manager.feed_url().await,
        "https://test.example.com/feed.xml"
    );
    assert!(!manager.auto_check_enabled().await);

    let retrieved_config = manager.config().await;
    assert!(retrieved_config.auto_download);
    assert!(retrieved_config.include_beta);
}

// =============================================================================
// Initialization Tests
// =============================================================================

#[tokio::test]
async fn test_update_manager_initialize_success() {
    let manager = UpdateManager::new();

    let result = manager.initialize().await;
    assert!(result.is_ok());
    assert_eq!(manager.status().await, UpdateStatus::Ready);
}

#[tokio::test]
async fn test_update_manager_initialize_idempotent() {
    let manager = UpdateManager::new();

    // First initialization
    manager.initialize().await.unwrap();
    assert_eq!(manager.status().await, UpdateStatus::Ready);

    // Second initialization should succeed (idempotent)
    let result = manager.initialize().await;
    assert!(result.is_ok());
    assert_eq!(manager.status().await, UpdateStatus::Ready);
}

#[tokio::test]
async fn test_update_manager_initialize_invalid_url() {
    let manager = UpdateManager::with_feed_url("not-a-valid-url");

    let result = manager.initialize().await;
    assert!(matches!(result, Err(UpdateError::InvalidFeedUrl(_))));
}

// =============================================================================
// Configuration Update Tests
// =============================================================================

#[tokio::test]
async fn test_set_auto_check_enabled() {
    let manager = UpdateManager::new();

    // Initially enabled
    assert!(manager.auto_check_enabled().await);

    // Disable
    manager.set_auto_check(false).await;
    assert!(!manager.auto_check_enabled().await);

    // Re-enable
    manager.set_auto_check(true).await;
    assert!(manager.auto_check_enabled().await);
}

#[tokio::test]
async fn test_set_feed_url_valid() {
    let manager = UpdateManager::new();
    let new_url = "https://new.example.com/appcast.xml";

    let result = manager.set_feed_url(new_url).await;
    assert!(result.is_ok());
    assert_eq!(manager.feed_url().await, new_url);
}

#[tokio::test]
async fn test_set_feed_url_invalid() {
    let manager = UpdateManager::new();

    let result = manager.set_feed_url("invalid-url").await;
    assert!(matches!(result, Err(UpdateError::InvalidFeedUrl(_))));

    // Original URL should be unchanged
    assert_eq!(manager.feed_url().await, DEFAULT_FEED_URL);
}

#[tokio::test]
async fn test_set_check_interval() {
    let manager = UpdateManager::new();
    let new_interval = Duration::from_secs(7200); // 2 hours

    manager.set_check_interval(new_interval).await;

    let config = manager.config().await;
    assert_eq!(config.check_interval, new_interval);
}

// =============================================================================
// Status Tests
// =============================================================================

#[tokio::test]
async fn test_update_status_helpers() {
    assert!(UpdateStatus::Checking.is_checking());
    assert!(!UpdateStatus::Ready.is_checking());

    assert!(UpdateStatus::UpdateAvailable.has_update());
    assert!(!UpdateStatus::Ready.has_update());
    assert!(!UpdateStatus::Checking.has_update());

    assert!(UpdateStatus::Checking.is_busy());
    assert!(UpdateStatus::Downloading.is_busy());
    assert!(UpdateStatus::Installing.is_busy());
    assert!(!UpdateStatus::Ready.is_busy());
    assert!(!UpdateStatus::UpdateAvailable.is_busy());
}

// =============================================================================
// AvailableUpdate Tests
// =============================================================================

#[test]
fn test_available_update_description() {
    let update = AvailableUpdate {
        version: "100".to_string(),
        short_version: "1.0.0".to_string(),
        pub_date: "2026-01-01".to_string(),
        download_url: "https://example.com/update.dmg".to_string(),
        content_length: 15_000_000,
        ed_signature: Some("signature123".to_string()),
        release_notes: Some("Bug fixes".to_string()),
        minimum_system_version: Some("12.0".to_string()),
    };

    let description = update.description();
    assert!(description.contains("1.0.0"));
    assert!(description.contains("100"));
}

#[test]
fn test_available_update_equality() {
    let update1 = AvailableUpdate {
        version: "1.0.0".to_string(),
        short_version: "1.0.0".to_string(),
        pub_date: "2026-01-01".to_string(),
        download_url: "https://example.com/update.dmg".to_string(),
        content_length: 15_000_000,
        ed_signature: None,
        release_notes: None,
        minimum_system_version: None,
    };

    let update2 = update1.clone();
    assert_eq!(update1, update2);
}

// =============================================================================
// Appcast Parsing Tests
// =============================================================================

#[tokio::test]
async fn test_parse_appcast_newer_version() {
    let manager = UpdateManager::new();
    let result = manager.parse_appcast_items(MOCK_APPCAST_NEWER);

    assert!(result.is_ok());
    let update = result.unwrap();
    assert!(update.is_some());

    let update = update.unwrap();
    assert_eq!(update.version, "99.0.0");
    assert_eq!(update.short_version, "99.0.0");
    assert_eq!(
        update.download_url,
        "https://api.photoncast.app/releases/99.0.0/PhotonCast.dmg"
    );
    assert_eq!(update.content_length, 15_240_000);
    assert_eq!(
        update.ed_signature,
        Some("mock_signature_12345".to_string())
    );
    assert!(update.release_notes.is_some());
}

#[tokio::test]
async fn test_parse_appcast_current_version() {
    let manager = UpdateManager::new();
    let result = manager.parse_appcast_items(MOCK_APPCAST_CURRENT);

    assert!(result.is_ok());
    let update = result.unwrap();
    assert!(update.is_some());

    let update = update.unwrap();
    assert_eq!(update.version, "0.1.0");
}

#[tokio::test]
async fn test_parse_appcast_multiple_versions() {
    let manager = UpdateManager::new();
    let result = manager.parse_appcast_items(MOCK_APPCAST_MULTIPLE);

    assert!(result.is_ok());
    let update = result.unwrap();
    assert!(update.is_some());

    // Should get the first (latest) version
    let update = update.unwrap();
    assert_eq!(update.version, "2.0.0");
}

#[tokio::test]
async fn test_parse_appcast_invalid() {
    let manager = UpdateManager::new();
    let result = manager.parse_appcast_items(MOCK_APPCAST_INVALID);

    assert!(result.is_ok());
    // Invalid appcast should return None (no valid update found)
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_parse_appcast_empty() {
    let manager = UpdateManager::new();
    let result = manager.parse_appcast_items(MOCK_APPCAST_EMPTY);

    assert!(result.is_ok());
    // Empty channel should return None
    assert!(result.unwrap().is_none());
}

// =============================================================================
// Install Update Tests
// =============================================================================

#[tokio::test]
async fn test_install_update_no_update_available() {
    let manager = UpdateManager::new();
    manager.initialize().await.unwrap();

    let result = manager.install_update().await;
    assert!(matches!(result, Err(UpdateError::NoUpdateAvailable)));
}

// =============================================================================
// Auto-Check Tests
// =============================================================================

#[tokio::test]
async fn test_auto_check_disabled() {
    let mut config = UpdateConfig::default();
    config.auto_check_enabled = false;

    let manager = UpdateManager::with_config(config);
    manager.initialize().await.unwrap();

    // Auto-check should return None when disabled
    let result = manager.auto_check_if_needed().await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_last_check_initially_none() {
    let manager = UpdateManager::new();
    assert!(manager.last_check().await.is_none());
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_update_error_display() {
    let error = UpdateError::InitializationFailed("test error".to_string());
    assert!(error.to_string().contains("test error"));

    let error = UpdateError::NoUpdateAvailable;
    assert!(error.to_string().contains("No update"));

    let error = UpdateError::AlreadyChecking;
    assert!(error.to_string().contains("in progress"));

    let error = UpdateError::InvalidFeedUrl("bad-url".to_string());
    assert!(error.to_string().contains("bad-url"));

    let error = UpdateError::ParseError("xml error".to_string());
    assert!(error.to_string().contains("xml error"));

    let error = UpdateError::NetworkError("connection failed".to_string());
    assert!(error.to_string().contains("connection failed"));

    let error = UpdateError::FeedFetchFailed("404 Not Found".to_string());
    assert!(error.to_string().contains("404"));

    let error = UpdateError::InstallationFailed("permission denied".to_string());
    assert!(error.to_string().contains("permission denied"));
}

// =============================================================================
// Config Serialization Tests
// =============================================================================

#[tokio::test]
async fn test_update_config_serialization() {
    let config = UpdateConfig {
        feed_url: "https://test.example.com/feed.xml".to_string(),
        auto_check_enabled: false,
        check_interval: Duration::from_secs(7200),
        auto_download: true,
        include_beta: true,
    };

    // Serialize
    let json = serde_json::to_string(&config).expect("Failed to serialize");

    // Deserialize
    let deserialized: UpdateConfig =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.feed_url, config.feed_url);
    assert_eq!(deserialized.auto_check_enabled, config.auto_check_enabled);
    assert_eq!(deserialized.check_interval, config.check_interval);
    assert_eq!(deserialized.auto_download, config.auto_download);
    assert_eq!(deserialized.include_beta, config.include_beta);
}

#[test]
fn test_update_config_default() {
    let config = UpdateConfig::default();

    assert_eq!(config.feed_url, DEFAULT_FEED_URL);
    assert!(config.auto_check_enabled);
    assert_eq!(config.check_interval, DEFAULT_CHECK_INTERVAL);
    assert!(!config.auto_download);
    assert!(!config.include_beta);
}

// =============================================================================
// URL Validation Tests
// =============================================================================

#[tokio::test]
async fn test_valid_feed_urls() {
    let valid_urls = [
        "http://example.com/appcast.xml",
        "https://example.com/appcast.xml",
        "https://api.photoncast.app/updates/appcast.xml",
        "https://github.com/user/repo/releases/appcast.xml",
    ];

    for url in &valid_urls {
        let manager = UpdateManager::with_feed_url(*url);
        let result = manager.initialize().await;
        assert!(
            result.is_ok(),
            "URL '{}' should be valid but initialization failed: {:?}",
            url,
            result
        );
    }
}

#[tokio::test]
async fn test_invalid_feed_urls() {
    let invalid_urls = [
        "not-a-url",
        "ftp://example.com/feed.xml",
        "file:///path/to/file.xml",
        "",
        "javascript:alert(1)",
    ];

    for url in &invalid_urls {
        let manager = UpdateManager::with_feed_url(*url);
        let result = manager.initialize().await;
        assert!(
            matches!(result, Err(UpdateError::InvalidFeedUrl(_))),
            "URL '{}' should be invalid but was accepted",
            url
        );
    }
}

// =============================================================================
// Network Tests (Ignored by Default)
// =============================================================================

#[tokio::test]
#[ignore = "requires network access, run with --ignored"]
async fn test_check_for_updates_real_network() {
    // This test attempts to fetch from the real appcast URL
    // It will fail if the URL doesn't exist or network is unavailable
    let manager = UpdateManager::new();
    manager.initialize().await.unwrap();

    let result = manager.check_for_updates().await;

    // The result depends on whether the feed exists and network is available
    // We just check that it doesn't panic
    match result {
        Ok(Some(update)) => {
            println!("Update available: {}", update.description());
        }
        Ok(None) => {
            println!("No update available (current version is latest)");
        }
        Err(e) => {
            println!("Update check failed (expected in test): {}", e);
        }
    }
}
