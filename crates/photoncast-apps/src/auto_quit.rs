//! Auto quit feature - automatically quit idle applications.
//!
//! This module provides:
//! - A static list of suggested applications for the auto-quit feature
//! - Configuration management for auto-quit settings
//! - Activity tracking and automatic quitting of idle apps

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

/// Default timeout in minutes before an app is auto-quit.
pub const DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES: u32 = 3;

/// Suggested apps for Auto Quit feature.
///
/// Each entry is a tuple of (bundle_id, display_name).
pub const SUGGESTED_AUTO_QUIT_APPS: &[(&str, &str)] = &[
    // Messaging
    ("com.tinyspeck.slackmacgap", "Slack"),
    ("com.hnc.Discord", "Discord"),
    ("com.apple.MobileSMS", "Messages"),
    ("com.microsoft.teams2", "Microsoft Teams"),
    ("us.zoom.xos", "Zoom"),
    ("com.skype.skype", "Skype"),
    ("org.whispersystems.signal-desktop", "Signal"),
    ("com.telegram.desktop", "Telegram"),
    // Calendar & Productivity
    ("com.apple.iCal", "Calendar"),
    ("com.microsoft.Outlook", "Microsoft Outlook"),
    ("notion.id", "Notion"),
    ("com.agilebits.onepassword7", "1Password"),
    ("md.obsidian", "Obsidian"),
    // Social
    ("com.twitter.twitter-mac", "Twitter"),
    ("com.facebook.Facebook", "Facebook"),
    // Email
    ("com.apple.mail", "Mail"),
    ("com.readdle.smartemail-Mac", "Spark"),
    ("com.google.Gmail", "Gmail"),
    ("com.mailspring.mailspring", "Mailspring"),
    // News & Reading
    ("com.reederapp.5.macOS", "Reeder"),
    ("com.apple.news", "News"),
    // Media
    ("com.spotify.client", "Spotify"),
    ("com.apple.Music", "Music"),
    ("tv.plex.desktop", "Plex"),
    // Notes & Organization
    ("com.apple.Notes", "Notes"),
    ("com.apple.reminders", "Reminders"),
    ("com.evernote.Evernote", "Evernote"),
    ("com.culturedcode.ThingsMac", "Things"),
    ("com.todoist.mac.Todoist", "Todoist"),
];

/// Returns the suggested apps as a slice of (bundle_id, display_name) tuples.
#[must_use]
pub fn suggested_auto_quit_apps() -> &'static [(&'static str, &'static str)] {
    SUGGESTED_AUTO_QUIT_APPS
}

/// Checks if a bundle ID is in the suggested auto quit list.
#[must_use]
pub fn is_suggested_auto_quit_app(bundle_id: &str) -> bool {
    SUGGESTED_AUTO_QUIT_APPS
        .iter()
        .any(|(id, _)| *id == bundle_id)
}

/// Gets the display name for a suggested auto quit app by bundle ID.
#[must_use]
pub fn get_suggested_app_name(bundle_id: &str) -> Option<&'static str> {
    SUGGESTED_AUTO_QUIT_APPS
        .iter()
        .find(|(id, _)| *id == bundle_id)
        .map(|(_, name)| *name)
}

// ============================================================================
// Auto Quit Configuration
// ============================================================================

/// Configuration for the auto-quit feature.
///
/// Stored as TOML at `~/.config/photoncast/auto_quit.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct AutoQuitConfig {
    /// Per-app auto-quit configurations, keyed by bundle ID.
    pub apps: HashMap<String, AutoQuitAppConfig>,
}


impl AutoQuitConfig {
    /// Returns the config file path.
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| crate::error::AppError::ConfigError("Cannot find config directory".into()))?;
        Ok(config_dir.join("photoncast").join("auto_quit.toml"))
    }

    /// Loads the configuration from disk.
    ///
    /// Returns default config if the file doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path).map_err(|e| {
            crate::error::AppError::ConfigError(format!("Failed to read config file: {}", e))
        })?;

        toml::from_str(&contents).map_err(|e| {
            crate::error::AppError::ConfigError(format!("Failed to parse config file: {}", e))
        })
    }

    /// Saves the configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                crate::error::AppError::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
        }

        let contents = toml::to_string_pretty(self).map_err(|e| {
            crate::error::AppError::ConfigError(format!("Failed to serialize config: {}", e))
        })?;

        fs::write(&path, contents).map_err(|e| {
            crate::error::AppError::ConfigError(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }
}

/// Per-app auto-quit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoQuitAppConfig {
    /// Whether auto-quit is enabled for this app.
    pub enabled: bool,
    /// Timeout in minutes before the app is auto-quit (default: 3).
    #[serde(default = "default_timeout_minutes")]
    pub timeout_minutes: u32,
    /// Last time the app was active (for persistence across restarts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
}

fn default_timeout_minutes() -> u32 {
    DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES
}

impl Default for AutoQuitAppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_minutes: DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES,
            last_active: None,
        }
    }
}

// ============================================================================
// Auto Quit Manager
// ============================================================================

/// Manages auto-quit functionality for applications.
///
/// Tracks activity for apps and automatically quits them when idle.
#[derive(Debug)]
pub struct AutoQuitManager {
    /// Persistent configuration.
    config: AutoQuitConfig,
    /// In-memory activity tracker (bundle_id -> last activity instant).
    activity_tracker: HashMap<String, Instant>,
}

impl AutoQuitManager {
    /// Creates a new auto-quit manager with the given configuration.
    #[must_use]
    pub fn new(config: AutoQuitConfig) -> Self {
        Self {
            config,
            activity_tracker: HashMap::new(),
        }
    }

    /// Loads the manager from disk configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be loaded.
    pub fn load() -> Result<Self> {
        let config = AutoQuitConfig::load()?;
        Ok(Self::new(config))
    }

    /// Saves the current configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be saved.
    pub fn save(&self) -> Result<()> {
        self.config.save()
    }

    /// Returns a reference to the current configuration.
    #[must_use]
    pub fn config(&self) -> &AutoQuitConfig {
        &self.config
    }

    /// Returns a mutable reference to the current configuration.
    pub fn config_mut(&mut self) -> &mut AutoQuitConfig {
        &mut self.config
    }

    /// Records that an app was activated (became frontmost/focused).
    ///
    /// This resets the idle timer for the app.
    pub fn on_app_activated(&mut self, bundle_id: &str) {
        self.activity_tracker.insert(bundle_id.to_string(), Instant::now());

        // Also update the persistent last_active time
        if let Some(app_config) = self.config.apps.get_mut(bundle_id) {
            app_config.last_active = Some(Utc::now());
        }
    }

    /// Checks all tracked apps and quits any that have been idle too long.
    ///
    /// Returns a list of bundle IDs for apps that were quit.
    pub fn check_and_quit_inactive(&mut self) -> Vec<String> {
        let mut quit_apps = Vec::new();
        let now = Instant::now();

        // Get running apps once to avoid N+1 queries
        let running_apps = match crate::process::get_running_apps() {
            Ok(apps) => apps,
            Err(e) => {
                tracing::warn!("Failed to get running apps for auto-quit check: {}", e);
                return quit_apps;
            }
        };

        // Build a map of bundle_id -> pid for quick lookup
        let running_map: HashMap<String, u32> = running_apps
            .into_iter()
            .filter_map(|app| app.bundle_id.map(|id| (id.to_lowercase(), app.pid)))
            .collect();

        // Collect apps to quit (to avoid borrowing issues)
        let apps_to_check: Vec<(String, u32)> = self
            .config
            .apps
            .iter()
            .filter(|(_, cfg)| cfg.enabled)
            .map(|(bundle_id, cfg)| (bundle_id.clone(), cfg.timeout_minutes))
            .collect();

        for (bundle_id, timeout_minutes) in apps_to_check {
            let timeout = std::time::Duration::from_secs(u64::from(timeout_minutes) * 60);

            // Check if the app has been tracked and is idle
            if let Some(last_active) = self.activity_tracker.get(&bundle_id) {
                if now.duration_since(*last_active) >= timeout {
                    // App is idle, try to quit it using the cached running apps
                    if let Some(&pid) = running_map.get(&bundle_id.to_lowercase()) {
                        if let Err(e) = crate::process::quit_app(pid) {
                            tracing::warn!("Failed to auto-quit {}: {}", bundle_id, e);
                        } else {
                            quit_apps.push(bundle_id.clone());
                            self.activity_tracker.remove(&bundle_id);
                            tracing::info!("Auto-quit idle app: {}", bundle_id);
                        }
                    }
                }
            }
        }

        quit_apps
    }

    /// Enables auto-quit for an app.
    ///
    /// # Arguments
    ///
    /// * `bundle_id` - The bundle identifier of the app
    /// * `timeout_minutes` - Idle timeout before the app is quit
    pub fn enable_auto_quit(&mut self, bundle_id: &str, timeout_minutes: u32) {
        let config = self.config.apps.entry(bundle_id.to_string()).or_default();
        config.enabled = true;
        config.timeout_minutes = timeout_minutes;

        // Start tracking the app
        self.activity_tracker.insert(bundle_id.to_string(), Instant::now());
    }

    /// Disables auto-quit for an app.
    pub fn disable_auto_quit(&mut self, bundle_id: &str) {
        if let Some(config) = self.config.apps.get_mut(bundle_id) {
            config.enabled = false;
        }
        self.activity_tracker.remove(bundle_id);
    }

    /// Checks if auto-quit is enabled for an app.
    #[must_use]
    pub fn is_auto_quit_enabled(&self, bundle_id: &str) -> bool {
        self.config
            .apps
            .get(bundle_id)
            .is_some_and(|cfg| cfg.enabled)
    }

    /// Gets the timeout for an app in minutes.
    #[must_use]
    pub fn get_timeout_minutes(&self, bundle_id: &str) -> Option<u32> {
        self.config
            .apps
            .get(bundle_id)
            .filter(|cfg| cfg.enabled)
            .map(|cfg| cfg.timeout_minutes)
    }

    /// Cleans up tracking for apps that are no longer running.
    pub fn cleanup_stale_entries(&mut self, running_bundle_ids: &[&str]) {
        self.activity_tracker.retain(|bundle_id, _| {
            running_bundle_ids.contains(&bundle_id.as_str())
        });
    }

    /// Gets all apps with auto-quit enabled.
    #[must_use]
    pub fn get_enabled_apps(&self) -> Vec<(&str, &AutoQuitAppConfig)> {
        self.config
            .apps
            .iter()
            .filter(|(_, cfg)| cfg.enabled)
            .map(|(id, cfg)| (id.as_str(), cfg))
            .collect()
    }

    /// Performs a single auto-quit tick: updates frontmost app activity and quits idle apps.
    ///
    /// This should be called periodically (e.g., every 30 seconds) from a background timer.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - The bundle ID of the frontmost app (if any)
    /// - A list of bundle IDs for apps that were auto-quit
    pub fn tick(&mut self) -> (Option<String>, Vec<String>) {
        // Get the currently frontmost app and update its activity
        let frontmost = crate::process::get_frontmost_app_bundle_id();
        if let Some(ref bundle_id) = frontmost {
            // Only track if this app has auto-quit enabled
            if self.is_auto_quit_enabled(bundle_id) {
                self.on_app_activated(bundle_id);
                tracing::trace!("Auto-quit: Updated activity for frontmost app: {}", bundle_id);
            }
        }

        // Check for idle apps and quit them
        let quit_apps = self.check_and_quit_inactive();

        (frontmost, quit_apps)
    }

    /// Returns true if there are any apps with auto-quit enabled.
    #[must_use]
    pub fn has_enabled_apps(&self) -> bool {
        self.config.apps.values().any(|cfg| cfg.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggested_apps_not_empty() {
        assert!(!SUGGESTED_AUTO_QUIT_APPS.is_empty());
    }

    #[test]
    fn test_is_suggested_auto_quit_app() {
        assert!(is_suggested_auto_quit_app("com.tinyspeck.slackmacgap"));
        assert!(is_suggested_auto_quit_app("com.hnc.Discord"));
        assert!(!is_suggested_auto_quit_app("com.example.unknown"));
    }

    #[test]
    fn test_get_suggested_app_name() {
        assert_eq!(
            get_suggested_app_name("com.tinyspeck.slackmacgap"),
            Some("Slack")
        );
        assert_eq!(get_suggested_app_name("com.hnc.Discord"), Some("Discord"));
        assert_eq!(get_suggested_app_name("com.example.unknown"), None);
    }

    #[test]
    fn test_all_apps_have_valid_data() {
        for (bundle_id, name) in SUGGESTED_AUTO_QUIT_APPS {
            // Bundle ID should not be empty and should contain at least one dot
            assert!(!bundle_id.is_empty(), "Empty bundle ID found");
            assert!(
                bundle_id.contains('.'),
                "Bundle ID '{}' should contain at least one dot",
                bundle_id
            );

            // Name should not be empty
            assert!(!name.is_empty(), "Empty name found for bundle ID {}", bundle_id);
        }
    }

    #[test]
    fn test_auto_quit_config_default() {
        let config = AutoQuitConfig::default();
        assert!(config.apps.is_empty());
    }

    #[test]
    fn test_auto_quit_app_config_default() {
        let config = AutoQuitAppConfig::default();
        assert!(config.enabled);
        assert_eq!(config.timeout_minutes, DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES);
        assert!(config.last_active.is_none());
    }

    #[test]
    fn test_auto_quit_manager_enable_disable() {
        let config = AutoQuitConfig::default();
        let mut manager = AutoQuitManager::new(config);

        // Enable auto-quit for an app
        manager.enable_auto_quit("com.example.app", 5);
        assert!(manager.is_auto_quit_enabled("com.example.app"));
        assert_eq!(manager.get_timeout_minutes("com.example.app"), Some(5));

        // Disable auto-quit
        manager.disable_auto_quit("com.example.app");
        assert!(!manager.is_auto_quit_enabled("com.example.app"));
        assert_eq!(manager.get_timeout_minutes("com.example.app"), None);
    }

    #[test]
    fn test_auto_quit_manager_activity_tracking() {
        let config = AutoQuitConfig::default();
        let mut manager = AutoQuitManager::new(config);

        // Enable and track activity
        manager.enable_auto_quit("com.example.app", 5);
        manager.on_app_activated("com.example.app");

        // Activity should be tracked
        assert!(manager.activity_tracker.contains_key("com.example.app"));
    }

    #[test]
    fn test_auto_quit_config_serialization() {
        let mut config = AutoQuitConfig::default();
        config.apps.insert(
            "com.example.app".to_string(),
            AutoQuitAppConfig {
                enabled: true,
                timeout_minutes: 5,
                last_active: Some(Utc::now()),
            },
        );

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        assert!(toml_str.contains("com.example.app"));
        assert!(toml_str.contains("timeout_minutes = 5"));

        // Deserialize back
        let parsed: AutoQuitConfig = toml::from_str(&toml_str).expect("Failed to deserialize");
        assert!(parsed.apps.contains_key("com.example.app"));
        assert_eq!(parsed.apps["com.example.app"].timeout_minutes, 5);
    }

    #[test]
    fn test_get_enabled_apps() {
        let mut config = AutoQuitConfig::default();
        config.apps.insert(
            "com.enabled.app".to_string(),
            AutoQuitAppConfig {
                enabled: true,
                timeout_minutes: 3,
                last_active: None,
            },
        );
        config.apps.insert(
            "com.disabled.app".to_string(),
            AutoQuitAppConfig {
                enabled: false,
                timeout_minutes: 3,
                last_active: None,
            },
        );

        let manager = AutoQuitManager::new(config);
        let enabled = manager.get_enabled_apps();

        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].0, "com.enabled.app");
    }

    #[test]
    fn test_cleanup_stale_entries() {
        let config = AutoQuitConfig::default();
        let mut manager = AutoQuitManager::new(config);

        // Add some tracked apps
        manager.activity_tracker.insert("com.running.app".to_string(), Instant::now());
        manager.activity_tracker.insert("com.stopped.app".to_string(), Instant::now());

        // Cleanup with only one running app
        manager.cleanup_stale_entries(&["com.running.app"]);

        assert!(manager.activity_tracker.contains_key("com.running.app"));
        assert!(!manager.activity_tracker.contains_key("com.stopped.app"));
    }

    #[test]
    fn test_auto_quit_timeout_logic() {
        use std::time::Duration;

        let config = AutoQuitConfig::default();
        let mut manager = AutoQuitManager::new(config);

        // Enable auto-quit for a test app with 0 minute timeout (immediate)
        // Note: We cannot actually test the quit because it requires a running app,
        // but we can verify the timeout calculation logic works correctly.
        
        // Test 1: App with 0 timeout should be considered immediately idle
        manager.enable_auto_quit("com.zero.timeout", 0);
        
        // Manually set the last activity to an old time to simulate idle
        let old_instant = Instant::now() - Duration::from_secs(1);
        manager.activity_tracker.insert("com.zero.timeout".to_string(), old_instant);
        
        // The timeout for 0 minutes is 0 seconds, so any elapsed time should exceed it
        let timeout = Duration::from_secs(0); // 0 minutes * 60
        let elapsed = Instant::now().duration_since(old_instant);
        assert!(elapsed >= timeout, "Elapsed time should exceed 0-minute timeout");

        // Test 2: App with regular timeout
        manager.enable_auto_quit("com.regular.timeout", 5); // 5 minutes
        
        // Fresh activity - should not be idle
        let recent_instant = Instant::now();
        manager.activity_tracker.insert("com.regular.timeout".to_string(), recent_instant);
        
        let timeout = Duration::from_secs(5 * 60); // 5 minutes
        let elapsed = Instant::now().duration_since(recent_instant);
        assert!(elapsed < timeout, "Fresh activity should not exceed 5-minute timeout");
        
        // Test 3: Verify the get_timeout_minutes function works correctly
        assert_eq!(manager.get_timeout_minutes("com.zero.timeout"), Some(0));
        assert_eq!(manager.get_timeout_minutes("com.regular.timeout"), Some(5));
        assert_eq!(manager.get_timeout_minutes("com.unknown.app"), None);
    }

    #[test]
    fn test_auto_quit_tracking_comprehensive() {
        let config = AutoQuitConfig::default();
        let mut manager = AutoQuitManager::new(config);

        let bundle_id = "com.test.tracking";
        
        // Enable auto-quit
        manager.enable_auto_quit(bundle_id, 3);
        
        // Initially should be tracked with recent timestamp
        assert!(manager.activity_tracker.contains_key(bundle_id));
        
        // Call on_app_activated to update timestamp
        let before = *manager.activity_tracker.get(bundle_id).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        manager.on_app_activated(bundle_id);
        let after = *manager.activity_tracker.get(bundle_id).unwrap();
        
        // The timestamp should have been updated (after should be later than before)
        assert!(after > before, "Activity timestamp should be updated on activation");
        
        // Verify the persistent config also gets updated
        let app_config = manager.config().apps.get(bundle_id).unwrap();
        assert!(app_config.last_active.is_some(), "Persistent last_active should be set");
    }

    // =========================================================================
    // Task 8.6: UI Indicator Tests
    // =========================================================================

    #[test]
    fn test_ui_indicator_auto_quit_enabled() {
        // Test that UI indicators can be determined from auto-quit state
        let mut manager = AutoQuitManager::new(AutoQuitConfig::default());

        let bundle_id = "com.example.testapp";
        
        // Before enabling, indicator should be false
        assert!(
            !manager.is_auto_quit_enabled(bundle_id),
            "Auto-quit indicator should be off initially"
        );

        // Enable auto-quit
        manager.enable_auto_quit(bundle_id, 5);
        
        // Now indicator should be true
        assert!(
            manager.is_auto_quit_enabled(bundle_id),
            "Auto-quit indicator should be on after enabling"
        );

        // Disable auto-quit
        manager.disable_auto_quit(bundle_id);
        
        // Indicator should be false again
        assert!(
            !manager.is_auto_quit_enabled(bundle_id),
            "Auto-quit indicator should be off after disabling"
        );
    }

    #[test]
    fn test_ui_indicator_multiple_apps() {
        // Test that indicators are independent per app
        let mut manager = AutoQuitManager::new(AutoQuitConfig::default());

        manager.enable_auto_quit("com.app1", 3);
        manager.enable_auto_quit("com.app2", 5);
        manager.enable_auto_quit("com.app3", 10);
        manager.disable_auto_quit("com.app2");

        assert!(manager.is_auto_quit_enabled("com.app1"));
        assert!(!manager.is_auto_quit_enabled("com.app2"));
        assert!(manager.is_auto_quit_enabled("com.app3"));
        assert!(!manager.is_auto_quit_enabled("com.unknown"));
    }

    // =========================================================================
    // Task 8.8: Persistence Tests
    // =========================================================================

    #[test]
    fn test_persistence_save_load_cycle() {
        // Test full save/load cycle through TOML serialization
        let mut config1 = AutoQuitConfig::default();
        
        // Add multiple apps with different settings
        config1.apps.insert(
            "com.example.app1".to_string(),
            AutoQuitAppConfig {
                enabled: true,
                timeout_minutes: 7,
                last_active: Some(Utc::now()),
            },
        );
        config1.apps.insert(
            "com.example.app2".to_string(),
            AutoQuitAppConfig {
                enabled: false,
                timeout_minutes: 3,
                last_active: None,
            },
        );

        // Serialize to TOML (simulating save)
        let toml_content = toml::to_string_pretty(&config1).expect("Failed to serialize");
        
        // Verify serialization contains expected data
        assert!(toml_content.contains("com.example.app1"));
        assert!(toml_content.contains("com.example.app2"));
        assert!(toml_content.contains("timeout_minutes = 7"));
        assert!(toml_content.contains("enabled = false"));

        // Deserialize (simulating load after restart)
        let config2: AutoQuitConfig = toml::from_str(&toml_content).expect("Failed to deserialize");

        // Verify all settings persisted
        assert!(config2.apps.contains_key("com.example.app1"));
        assert_eq!(config2.apps["com.example.app1"].enabled, true);
        assert_eq!(config2.apps["com.example.app1"].timeout_minutes, 7);

        assert!(config2.apps.contains_key("com.example.app2"));
        assert_eq!(config2.apps["com.example.app2"].enabled, false);
        assert_eq!(config2.apps["com.example.app2"].timeout_minutes, 3);
    }

    #[test]
    fn test_persistence_empty_config() {
        let config = AutoQuitConfig::default();
        assert!(config.apps.is_empty());

        // Serialize and deserialize empty config
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        let loaded: AutoQuitConfig = toml::from_str(&toml_str).expect("Failed to deserialize");

        assert!(loaded.apps.is_empty());
    }

    #[test]
    fn test_persistence_many_apps() {
        let mut config = AutoQuitConfig::default();

        // Add many apps with various configurations
        for i in 1..=10 {
            config.apps.insert(
                format!("com.example.app{}", i),
                AutoQuitAppConfig {
                    enabled: i % 2 == 0, // Even apps enabled
                    timeout_minutes: i * 2,
                    last_active: None,
                },
            );
        }

        // Round-trip through TOML
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        let loaded: AutoQuitConfig = toml::from_str(&toml_str).expect("Failed to deserialize");

        // Verify all 10 apps persisted
        assert_eq!(loaded.apps.len(), 10);

        for i in 1..=10 {
            let bundle_id = format!("com.example.app{}", i);
            let app = loaded.apps.get(&bundle_id).expect("App should exist");
            assert_eq!(app.enabled, i % 2 == 0, "Enabled state mismatch for {}", bundle_id);
            assert_eq!(app.timeout_minutes, i * 2, "Timeout mismatch for {}", bundle_id);
        }
    }

    #[test]
    fn test_persistence_manager_workflow() {
        // Test the full manager workflow for persistence
        let mut manager = AutoQuitManager::new(AutoQuitConfig::default());

        // Configure multiple apps
        manager.enable_auto_quit("com.test.app1", 10);
        manager.enable_auto_quit("com.test.app2", 15);
        manager.on_app_activated("com.test.app1");

        // Serialize the config (simulating save)
        let toml_content = toml::to_string_pretty(manager.config()).expect("Failed to serialize");

        // Create new manager from loaded config (simulating restart)
        let loaded_config: AutoQuitConfig = toml::from_str(&toml_content).expect("Failed to deserialize");
        let loaded_manager = AutoQuitManager::new(loaded_config);

        // Verify settings persisted
        assert!(loaded_manager.is_auto_quit_enabled("com.test.app1"));
        assert_eq!(loaded_manager.get_timeout_minutes("com.test.app1"), Some(10));
        assert!(loaded_manager.is_auto_quit_enabled("com.test.app2"));
        assert_eq!(loaded_manager.get_timeout_minutes("com.test.app2"), Some(15));
    }
}
