use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for app management features.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppsConfig {
    /// Whether app management is enabled.
    pub enabled: bool,
    /// Deep scan enabled by default for uninstaller.
    pub deep_scan_default: bool,
    /// App sleep configuration.
    pub app_sleep: AppSleepConfig,
}

/// Configuration for the app sleep feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSleepConfig {
    /// Whether app sleep is enabled.
    pub enabled: bool,
    /// Default idle timeout in minutes before sleeping an app.
    pub default_idle_minutes: u32,
    /// Per-app overrides.
    #[serde(default)]
    pub app_overrides: HashMap<String, AppSleepOverride>,
}

/// Per-app sleep configuration override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSleepOverride {
    /// Custom idle timeout for this app (None means use default).
    pub idle_minutes: Option<u32>,
    /// Never sleep this app.
    #[serde(default)]
    pub never_sleep: bool,
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            deep_scan_default: true,
            app_sleep: AppSleepConfig::default(),
        }
    }
}

impl Default for AppSleepConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_idle_minutes: 30,
            app_overrides: HashMap::new(),
        }
    }
}
