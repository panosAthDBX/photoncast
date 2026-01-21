use serde::{Deserialize, Serialize};

/// Window management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    /// Whether window management is enabled.
    pub enabled: bool,
    /// Whether animations are enabled.
    pub animation_enabled: bool,
    /// Animation duration in milliseconds.
    pub animation_duration_ms: u32,
    /// Whether cycling is enabled (Left Half → 50% → 33% → 66%).
    pub cycling_enabled: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            animation_enabled: true,
            animation_duration_ms: 200,
            cycling_enabled: true,
        }
    }
}
