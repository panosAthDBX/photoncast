//! Auto-update management for PhotonCast using Sparkle framework.
//!
//! This module provides functionality for checking and installing app updates
//! via the Sparkle framework. It supports manual update checks, automatic
//! background checks, and configurable update feed URLs.
//!
//! # Features
//!
//! - **Manual Updates**: Users can trigger update checks via menu or settings
//! - **Automatic Checks**: Background update checking on app launch (configurable)
//! - **Appcast Feeds**: RSS-based update feeds with EdDSA signature verification
//! - **Update Configuration**: Configurable feed URLs and check intervals
//!
//! # Example
//!
//! ```ignore
//! use photoncast_core::platform::updates::UpdateManager;
//!
//! // Create update manager with default feed URL
//! let manager = UpdateManager::new("https://api.photoncast.app/updates/appcast.xml");
//!
//! // Initialize the update system
//! manager.initialize()?;
//!
//! // Check for updates manually
//! manager.check_for_updates()?;
//!
//! // Enable automatic checks
//! manager.set_auto_check(true);
//! ```

use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Default appcast feed URL for PhotonCast updates.
pub const DEFAULT_FEED_URL: &str = "https://api.photoncast.app/updates/appcast.xml";

/// Default interval between automatic update checks (24 hours).
pub const DEFAULT_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Errors that can occur during update operations.
#[derive(Debug, Error, Clone)]
pub enum UpdateError {
    /// Failed to initialize the update system.
    #[error("Update system initialization failed: {0}")]
    InitializationFailed(String),

    /// Failed to check for updates.
    #[error("Update check failed: {0}")]
    CheckFailed(String),

    /// Failed to download the appcast feed.
    #[error("Failed to fetch appcast feed: {0}")]
    FeedFetchFailed(String),

    /// Failed to parse the appcast XML.
    #[error("Failed to parse appcast: {0}")]
    ParseError(String),

    /// No update is available.
    #[error("No update available")]
    NoUpdateAvailable,

    /// Update installation failed.
    #[error("Update installation failed: {0}")]
    InstallationFailed(String),

    /// Signature verification failed - update cannot be trusted.
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    /// Update functionality not yet implemented.
    #[error("Update installation not implemented - Sparkle framework integration required")]
    NotImplemented,

    /// Network error during update operation.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Update is already in progress.
    #[error("Update check already in progress")]
    AlreadyChecking,

    /// Invalid feed URL.
    #[error("Invalid feed URL: {0}")]
    InvalidFeedUrl(String),
}

/// Represents an available update from the appcast feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    /// Version string of the update.
    pub version: String,

    /// Short version string (marketing version).
    pub short_version: String,

    /// Publication date of the update.
    pub pub_date: String,

    /// URL to download the update.
    pub download_url: String,

    /// File size in bytes.
    pub content_length: u64,

    /// EdDSA signature for verification.
    pub ed_signature: Option<String>,

    /// Release notes (HTML or plain text).
    pub release_notes: Option<String>,

    /// Minimum system version required.
    pub minimum_system_version: Option<String>,
}

impl AvailableUpdate {
    /// Returns a formatted description of the update.
    #[must_use]
    pub fn description(&self) -> String {
        format!(
            "Version {} ({})",
            self.short_version, self.version
        )
    }
}

/// Status of the update manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStatus {
    /// Update system is not initialized.
    Uninitialized,

    /// Update system is initialized and ready.
    Ready,

    /// Currently checking for updates.
    Checking,

    /// Update is available for download.
    UpdateAvailable,

    /// Downloading an update.
    Downloading,

    /// Installing an update.
    Installing,

    /// Update check failed.
    Error,
}

impl UpdateStatus {
    /// Returns true if an update check is in progress.
    #[must_use]
    pub fn is_checking(&self) -> bool {
        matches!(self, Self::Checking)
    }

    /// Returns true if an update is available.
    #[must_use]
    pub fn has_update(&self) -> bool {
        matches!(self, Self::UpdateAvailable)
    }

    /// Returns true if an update operation is in progress.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Checking | Self::Downloading | Self::Installing)
    }
}

/// Configuration for the update manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// URL of the appcast feed.
    pub feed_url: String,

    /// Whether automatic update checking is enabled.
    pub auto_check_enabled: bool,

    /// Interval between automatic checks.
    #[serde(with = "duration_seconds")]
    pub check_interval: Duration,

    /// Whether to automatically download updates.
    pub auto_download: bool,

    /// Whether to include beta releases.
    pub include_beta: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            feed_url: DEFAULT_FEED_URL.to_string(),
            auto_check_enabled: true,
            check_interval: DEFAULT_CHECK_INTERVAL,
            auto_download: false,
            include_beta: false,
        }
    }
}

/// Helper module for serializing Duration as seconds.
mod duration_seconds {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Internal state of the update manager.
#[derive(Debug)]
struct UpdateManagerState {
    /// Current status.
    status: UpdateStatus,

    /// Last check timestamp.
    last_check: Option<SystemTime>,

    /// Currently available update (if any).
    available_update: Option<AvailableUpdate>,

    /// HTTP client for fetching feeds.
    http_client: reqwest::Client,
}

/// Manages automatic and manual app updates.
///
/// The `UpdateManager` provides an interface to the Sparkle update framework,
/// handling update checks, downloads, and installations. It can be configured
/// for automatic background checks or manual user-initiated checks.
///
/// # Example
///
/// ```ignore
/// use photoncast_core::platform::updates::UpdateManager;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let manager = UpdateManager::new("https://api.photoncast.app/updates/appcast.xml");
///
///     // Initialize
///     manager.initialize().await?;
///
///     // Manual check
///     match manager.check_for_updates().await {
///         Ok(Some(update)) => println!("Update available: {}", update.description()),
///         Ok(None) => println!("No updates available"),
///         Err(e) => eprintln!("Update check failed: {}", e),
///     }
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct UpdateManager {
    /// Configuration for updates.
    config: Arc<RwLock<UpdateConfig>>,

    /// Internal state.
    state: Arc<RwLock<UpdateManagerState>>,
}

impl UpdateManager {
    /// Creates a new update manager with the default feed URL.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(UpdateConfig::default())
    }

    /// Creates a new update manager with a custom feed URL.
    ///
    /// # Arguments
    ///
    /// * `feed_url` - URL of the Sparkle appcast feed
    #[must_use]
    pub fn with_feed_url(feed_url: impl Into<String>) -> Self {
        let config = UpdateConfig {
            feed_url: feed_url.into(),
            ..UpdateConfig::default()
        };
        Self::with_config(config)
    }

    /// Creates a new update manager with a custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Update configuration
    #[must_use]
    pub fn with_config(config: UpdateConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("PhotonCast/{} UpdateManager", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(UpdateManagerState {
                status: UpdateStatus::Uninitialized,
                last_check: None,
                available_update: None,
                http_client,
            })),
        }
    }

    /// Initializes the update manager.
    ///
    /// This sets up the update system and prepares it for checking updates.
    /// Should be called once during app startup.
    ///
    /// # Errors
    ///
    /// Returns `UpdateError::InitializationFailed` if setup fails.
    pub async fn initialize(&self) -> Result<(), UpdateError> {
        let mut state = self.state.write().await;

        if state.status != UpdateStatus::Uninitialized {
            debug!("Update manager already initialized");
            return Ok(());
        }

        info!("Initializing update manager");

        // Validate feed URL
        let config = self.config.read().await;
        if !Self::is_valid_url(&config.feed_url) {
            return Err(UpdateError::InvalidFeedUrl(config.feed_url.clone()));
        }

        state.status = UpdateStatus::Ready;
        info!("Update manager initialized successfully");

        Ok(())
    }

    /// Checks if a URL is valid for an appcast feed.
    ///
    /// # Security
    ///
    /// Only HTTPS URLs are accepted to prevent MITM attacks on the update mechanism.
    fn is_valid_url(url: &str) -> bool {
        url.starts_with("https://")
    }

    /// Performs an automatic update check if enabled and due.
    ///
    /// This should be called during app launch to check for updates
    /// based on the configured interval.
    pub async fn auto_check_if_needed(&self) -> Result<Option<AvailableUpdate>, UpdateError> {
        let config = self.config.read().await;

        if !config.auto_check_enabled {
            debug!("Automatic update checks are disabled");
            return Ok(None);
        }

        let should_check = {
            let state = self.state.read().await;
            match state.last_check {
                None => true,
                Some(last) => {
                    let elapsed = SystemTime::now()
                        .duration_since(last)
                        .unwrap_or(Duration::MAX);
                    elapsed >= config.check_interval
                }
            }
        };

        drop(config); // Release read lock

        if should_check {
            info!("Performing automatic update check");
            self.check_for_updates().await
        } else {
            debug!("Skipping automatic check - not yet due");
            Ok(None)
        }
    }

    /// Manually checks for available updates.
    ///
    /// Fetches the appcast feed and parses it to find any available updates
    /// newer than the current version.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(update))` if an update is available
    /// - `Ok(None)` if no update is available
    /// - `Err(UpdateError)` if the check fails
    pub async fn check_for_updates(&self) -> Result<Option<AvailableUpdate>, UpdateError> {
        let mut state = self.state.write().await;

        if state.status.is_busy() {
            return Err(UpdateError::AlreadyChecking);
        }

        state.status = UpdateStatus::Checking;
        drop(state); // Release lock during network operation

        let feed_url = self.config.read().await.feed_url.clone();
        info!(url = %feed_url, "Checking for updates");

        match self.fetch_appcast(&feed_url).await {
            Ok(appcast) => {
                let mut state = self.state.write().await;
                state.last_check = Some(SystemTime::now());

                if let Some(update) = Self::find_available_update(&appcast) {
                    info!(
                        version = %update.version,
                        "Update available"
                    );
                    state.status = UpdateStatus::UpdateAvailable;
                    state.available_update = Some(update.clone());
                    Ok(Some(update))
                } else {
                    debug!("No updates available");
                    state.status = UpdateStatus::Ready;
                    state.available_update = None;
                    Ok(None)
                }
            }
            Err(e) => {
                let mut state = self.state.write().await;
                state.status = UpdateStatus::Error;
                error!(error = %e, "Failed to fetch appcast");
                Err(e)
            }
        }
    }

    /// Fetches the appcast feed from the given URL.
    async fn fetch_appcast(&self, url: &str) -> Result<String, UpdateError> {
        let state = self.state.read().await;
        let client = state.http_client.clone();
        drop(state);

        debug!(url = %url, "Fetching appcast feed");

        let response: reqwest::Response = client
            .get(url)
            .send()
            .await
            .map_err(|e: reqwest::Error| UpdateError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(UpdateError::FeedFetchFailed(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let content: String = response
            .text()
            .await
            .map_err(|e: reqwest::Error| UpdateError::FeedFetchFailed(e.to_string()))?;

        debug!(bytes = content.len(), "Fetched appcast feed");
        Ok(content)
    }

    /// Parses the appcast and finds any available update.
    fn find_available_update(appcast: &str) -> Option<AvailableUpdate> {
        // Simple XML parsing to extract update information
        // In a full implementation, this would use a proper RSS/XML parser
        // and compare versions using semver

        let current_version = env!("CARGO_PKG_VERSION");
        debug!(current = %current_version, "Checking against current version");

        // Parse the appcast XML to find the latest item
        // This is a simplified implementation
        let update = Self::parse_appcast_items(appcast);

        if let Some(ref avail) = update {
            // Compare versions (simplified - would use semver in production)
            if avail.version == current_version {
                debug!("Latest version matches current version");
                return None;
            }
        }

        update
    }

    /// Parses appcast items from the XML content.
    fn parse_appcast_items(appcast: &str) -> Option<AvailableUpdate> {
        // Simplified parsing - look for enclosure and version info
        // Full implementation would use quick-xml or similar

        let mut update = AvailableUpdate {
            version: String::new(),
            short_version: String::new(),
            pub_date: String::new(),
            download_url: String::new(),
            content_length: 0,
            ed_signature: None,
            release_notes: None,
            minimum_system_version: None,
        };

        // Extract sparkle:version
        if let Some(version_start) = appcast.find("sparkle:version>") {
            let start = version_start + "sparkle:version>".len();
            if let Some(end) = appcast[start..].find('<') {
                update.version = appcast[start..start + end].to_string();
            }
        }

        // Extract sparkle:shortVersionString
        if let Some(version_start) = appcast.find("sparkle:shortVersionString>") {
            let start = version_start + "sparkle:shortVersionString>".len();
            if let Some(end) = appcast[start..].find('<') {
                update.short_version = appcast[start..start + end].to_string();
            }
        }

        // Extract pubDate
        if let Some(date_start) = appcast.find("pubDate>") {
            let start = date_start + "pubDate>".len();
            if let Some(end) = appcast[start..].find('<') {
                update.pub_date = appcast[start..start + end].to_string();
            }
        }

        // Extract enclosure URL and attributes
        if let Some(enclosure_start) = appcast.find("<enclosure") {
            let enclosure_end = appcast[enclosure_start..]
                .find('>')
                .map_or(appcast.len(), |i| enclosure_start + i);
            let enclosure_tag = &appcast[enclosure_start..enclosure_end];

            // Extract url
            if let Some(url_start) = enclosure_tag.find("url=\"") {
                let start = url_start + "url=\"".len();
                if let Some(end) = enclosure_tag[start..].find('"') {
                    update.download_url = enclosure_tag[start..start + end].to_string();
                }
            }

            // Extract length
            if let Some(length_start) = enclosure_tag.find("length=\"") {
                let start = length_start + "length=\"".len();
                if let Some(end) = enclosure_tag[start..].find('"') {
                    if let Ok(len) = enclosure_tag[start..start + end].parse::<u64>() {
                        update.content_length = len;
                    }
                }
            }

            // Extract sparkle:edSignature
            if let Some(sig_start) = enclosure_tag.find("sparkle:edSignature=\"") {
                let start = sig_start + "sparkle:edSignature=\"".len();
                if let Some(end) = enclosure_tag[start..].find('"') {
                    update.ed_signature = Some(enclosure_tag[start..start + end].to_string());
                }
            }
        }

        // Extract description/release notes
        if let Some(desc_start) = appcast.find("<description>") {
            let start = desc_start + "<description>".len();
            if let Some(end) = appcast[start..].find("</description>") {
                update.release_notes = Some(appcast[start..start + end].to_string());
            }
        }

        // Validate we have the required fields
        if update.version.is_empty() || update.download_url.is_empty() {
            warn!("Appcast missing required fields (version or download URL)");
            return None;
        }

        // If short_version is empty, use version
        if update.short_version.is_empty() {
            update.short_version = update.version.clone();
        }

        debug!(
            version = %update.version,
            url = %update.download_url,
            "Parsed available update"
        );

        Some(update)
    }

    /// Installs the available update.
    ///
    /// # Errors
    ///
    /// Returns `UpdateError::NoUpdateAvailable` if no update is pending.
    /// Returns `UpdateError::SignatureVerificationFailed` if the update signature cannot be verified.
    /// Returns `UpdateError::NotImplemented` because Sparkle framework integration is required.
    ///
    /// # Security
    ///
    /// This function currently returns `NotImplemented` because proper update installation
    /// requires Sparkle framework integration for secure signature verification and installation.
    /// Direct download and installation without signature verification would be a security risk.
    pub async fn install_update(&self) -> Result<(), UpdateError> {
        let state = self.state.read().await;

        if state.available_update.is_none() {
            return Err(UpdateError::NoUpdateAvailable);
        }

        let update = state.available_update.clone().unwrap();
        drop(state);

        info!(version = %update.version, "Attempting to install update");

        // Security: Verify signature is present before attempting installation
        if update.ed_signature.is_none() {
            error!(version = %update.version, "Update has no EdDSA signature - refusing to install unsigned update");
            return Err(UpdateError::SignatureVerificationFailed(
                "Update package is not signed - cannot verify authenticity".to_string()
            ));
        }

        // Full implementation requires Sparkle framework integration for:
        // 1. Download the update package to a secure temporary location
        // 2. Verify the EdDSA signature using the public key embedded in the app
        // 3. Extract and install the update atomically
        // 4. Trigger app restart
        //
        // This cannot be safely implemented without Sparkle as we need its
        // signature verification, secure download, and atomic replacement logic.

        warn!(
            version = %update.version,
            signature = ?update.ed_signature,
            "Update installation requires Sparkle framework integration"
        );

        Err(UpdateError::NotImplemented)
    }

    /// Sets whether automatic update checking is enabled.
    pub async fn set_auto_check(&self, enabled: bool) {
        let mut config = self.config.write().await;
        let old_value = config.auto_check_enabled;
        config.auto_check_enabled = enabled;

        if old_value != enabled {
            info!(auto_check = enabled, "Automatic update check setting changed");
        }
    }

    /// Sets the feed URL for update checks.
    ///
    /// # Errors
    ///
    /// Returns `UpdateError::InvalidFeedUrl` if the URL is not valid.
    pub async fn set_feed_url(&self, url: impl Into<String>) -> Result<(), UpdateError> {
        let url = url.into();

        if !Self::is_valid_url(&url) {
            return Err(UpdateError::InvalidFeedUrl(url));
        }

        let mut config = self.config.write().await;
        config.feed_url = url;
        debug!("Feed URL updated");

        Ok(())
    }

    /// Sets the interval between automatic update checks.
    pub async fn set_check_interval(&self, interval: Duration) {
        let mut config = self.config.write().await;
        config.check_interval = interval;
        debug!(seconds = interval.as_secs(), "Check interval updated");
    }

    /// Returns the current update status.
    pub async fn status(&self) -> UpdateStatus {
        self.state.read().await.status
    }

    /// Returns the currently available update (if any).
    pub async fn available_update(&self) -> Option<AvailableUpdate> {
        self.state.read().await.available_update.clone()
    }

    /// Returns the last check timestamp.
    pub async fn last_check(&self) -> Option<SystemTime> {
        self.state.read().await.last_check
    }

    /// Returns the current configuration.
    pub async fn config(&self) -> UpdateConfig {
        self.config.read().await.clone()
    }

    /// Returns the feed URL.
    pub async fn feed_url(&self) -> String {
        self.config.read().await.feed_url.clone()
    }

    /// Returns whether auto-check is enabled.
    pub async fn auto_check_enabled(&self) -> bool {
        self.config.read().await.auto_check_enabled
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_APPCAST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
    <channel>
        <title>PhotonCast Changelog</title>
        <item>
            <title>Version 1.1.0</title>
            <pubDate>Mon, 15 Feb 2026 12:00:00 +0000</pubDate>
            <sparkle:version>1.1.0</sparkle:version>
            <sparkle:shortVersionString>1.1.0</sparkle:shortVersionString>
            <description><![CDATA[
                <h2>What's New</h2>
                <ul>
                    <li>New features</li>
                </ul>
            ]]></description>
            <enclosure url="https://api.photoncast.app/releases/1.1.0/PhotonCast.dmg"
                       sparkle:edSignature="testsignature123"
                       length="15240000"
                       type="application/octet-stream"/>
        </item>
    </channel>
</rss>"#;

    #[test]
    fn test_update_error_display() {
        let error = UpdateError::InitializationFailed("test".to_string());
        assert!(error.to_string().contains("test"));

        let error = UpdateError::NoUpdateAvailable;
        assert!(error.to_string().contains("No update"));

        let error = UpdateError::AlreadyChecking;
        assert!(error.to_string().contains("in progress"));
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

    #[test]
    fn test_update_status_helpers() {
        assert!(UpdateStatus::Checking.is_checking());
        assert!(!UpdateStatus::Ready.is_checking());

        assert!(UpdateStatus::UpdateAvailable.has_update());
        assert!(!UpdateStatus::Ready.has_update());

        assert!(UpdateStatus::Checking.is_busy());
        assert!(UpdateStatus::Downloading.is_busy());
        assert!(UpdateStatus::Installing.is_busy());
        assert!(!UpdateStatus::Ready.is_busy());
    }

    #[test]
    fn test_available_update_description() {
        let update = AvailableUpdate {
            version: "100".to_string(),
            short_version: "1.0.0".to_string(),
            pub_date: "2026-01-01".to_string(),
            download_url: "https://example.com/update.dmg".to_string(),
            content_length: 1_000_000,
            ed_signature: None,
            release_notes: None,
            minimum_system_version: None,
        };

        assert_eq!(update.description(), "Version 1.0.0 (100)");
    }

    #[test]
    fn test_is_valid_url() {
        // Only HTTPS URLs are allowed for security (prevents MITM attacks)
        assert!(!UpdateManager::is_valid_url("http://example.com"));
        assert!(UpdateManager::is_valid_url("https://example.com"));
        assert!(!UpdateManager::is_valid_url("ftp://example.com"));
        assert!(!UpdateManager::is_valid_url("not-a-url"));
    }

    #[tokio::test]
    async fn test_update_manager_new() {
        let manager = UpdateManager::new();
        assert_eq!(manager.feed_url().await, DEFAULT_FEED_URL);
        assert!(manager.auto_check_enabled().await);
    }

    #[tokio::test]
    async fn test_update_manager_with_feed_url() {
        let custom_url = "https://custom.example.com/appcast.xml";
        let manager = UpdateManager::with_feed_url(custom_url);
        assert_eq!(manager.feed_url().await, custom_url);
    }

    #[tokio::test]
    async fn test_update_manager_initialize() {
        let manager = UpdateManager::new();
        let result = manager.initialize().await;
        assert!(result.is_ok());

        // Second initialization should succeed (idempotent)
        let result = manager.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_manager_invalid_url() {
        let manager = UpdateManager::with_feed_url("invalid-url");
        let result = manager.initialize().await;
        assert!(matches!(result, Err(UpdateError::InvalidFeedUrl(_))));
    }

    #[tokio::test]
    async fn test_set_auto_check() {
        let manager = UpdateManager::new();
        assert!(manager.auto_check_enabled().await);

        manager.set_auto_check(false).await;
        assert!(!manager.auto_check_enabled().await);

        manager.set_auto_check(true).await;
        assert!(manager.auto_check_enabled().await);
    }

    #[tokio::test]
    async fn test_set_feed_url() {
        let manager = UpdateManager::new();
        let new_url = "https://new.example.com/feed.xml";

        let result = manager.set_feed_url(new_url).await;
        assert!(result.is_ok());
        assert_eq!(manager.feed_url().await, new_url);
    }

    #[tokio::test]
    async fn test_set_feed_url_invalid() {
        let manager = UpdateManager::new();
        let result = manager.set_feed_url("not-a-valid-url").await;
        assert!(matches!(result, Err(UpdateError::InvalidFeedUrl(_))));
    }

    #[tokio::test]
    async fn test_set_check_interval() {
        let manager = UpdateManager::new();
        let new_interval = Duration::from_secs(3600);

        manager.set_check_interval(new_interval).await;
        let config = manager.config().await;
        assert_eq!(config.check_interval, new_interval);
    }

    #[test]
    fn test_parse_appcast_items() {
        let update = UpdateManager::parse_appcast_items(TEST_APPCAST);

        assert!(update.is_some());

        let update = update.unwrap();
        assert_eq!(update.version, "1.1.0");
        assert_eq!(update.short_version, "1.1.0");
        assert_eq!(update.download_url, "https://api.photoncast.app/releases/1.1.0/PhotonCast.dmg");
        assert_eq!(update.content_length, 15_240_000);
        assert_eq!(update.ed_signature, Some("testsignature123".to_string()));
        assert!(update.release_notes.is_some());
    }

    #[tokio::test]
    async fn test_install_update_none_available() {
        let manager = UpdateManager::new();
        manager.initialize().await.unwrap();

        let result = manager.install_update().await;
        assert!(matches!(result, Err(UpdateError::NoUpdateAvailable)));
    }

    #[tokio::test]
    async fn test_status_transitions() {
        let manager = UpdateManager::new();

        // Initial status
        assert_eq!(manager.status().await, UpdateStatus::Uninitialized);

        // After initialization
        manager.initialize().await.unwrap();
        assert_eq!(manager.status().await, UpdateStatus::Ready);
    }

    #[tokio::test]
    async fn test_config_serialization() {
        let config = UpdateConfig {
            feed_url: "https://test.example.com".to_string(),
            auto_check_enabled: false,
            check_interval: Duration::from_secs(7200),
            auto_download: true,
            include_beta: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: UpdateConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.feed_url, config.feed_url);
        assert_eq!(deserialized.auto_check_enabled, config.auto_check_enabled);
        assert_eq!(deserialized.check_interval, config.check_interval);
        assert_eq!(deserialized.auto_download, config.auto_download);
        assert_eq!(deserialized.include_beta, config.include_beta);
    }

    // Integration tests that require network access
    #[tokio::test]
    #[ignore = "requires network access"]
    async fn test_fetch_appcast_real() {
        let manager = UpdateManager::new();
        let result = manager.fetch_appcast(DEFAULT_FEED_URL).await;
        // This will likely fail since the URL doesn't exist yet
        assert!(result.is_err());
    }
}
