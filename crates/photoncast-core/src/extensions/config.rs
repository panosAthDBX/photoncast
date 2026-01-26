use serde::{Deserialize, Serialize};

/// Environment variable to override dev mode setting.
pub const ENV_DEV_EXTENSIONS: &str = "PHOTONCAST_DEV_EXTENSIONS";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub dev_mode: bool,
    #[serde(default)]
    pub dev_paths: Vec<String>,
    /// Hot-reload debounce duration in milliseconds.
    #[serde(default = "default_reload_debounce_ms")]
    pub reload_debounce_ms: u64,
}

const fn default_enabled() -> bool {
    true
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dev_mode: false,
            dev_paths: Vec::new(),
            reload_debounce_ms: default_reload_debounce_ms(),
        }
    }
}

const fn default_reload_debounce_ms() -> u64 {
    200
}

impl ExtensionConfig {
    /// Resolves the effective dev_mode value, accounting for environment variable override.
    ///
    /// The `PHOTONCAST_DEV_EXTENSIONS` environment variable takes precedence over config.
    /// Set to "1", "true", or "yes" to enable; "0", "false", or "no" to disable.
    #[must_use]
    pub fn effective_dev_mode(&self) -> bool {
        if let Ok(val) = std::env::var(ENV_DEV_EXTENSIONS) {
            match val.to_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => return true,
                "0" | "false" | "no" | "off" => return false,
                _ => {},
            }
        }
        self.dev_mode
    }

    /// Logs the current dev mode status on startup.
    pub fn log_dev_mode_status(&self) {
        let effective = self.effective_dev_mode();
        let source = if std::env::var(ENV_DEV_EXTENSIONS).is_ok() {
            "environment variable"
        } else {
            "config"
        };

        if effective {
            tracing::info!(
                dev_mode = effective,
                source = source,
                dev_paths = ?self.dev_paths,
                "Extension dev mode enabled"
            );
        } else {
            tracing::debug!(
                dev_mode = effective,
                source = source,
                "Extension dev mode disabled"
            );
        }
    }

    /// Returns the reload debounce duration.
    #[must_use]
    pub fn reload_debounce(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.reload_debounce_ms)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionPermissionAcceptance {
    #[serde(default)]
    pub accepted: bool,
}
