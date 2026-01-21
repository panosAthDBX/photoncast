use serde::{Deserialize, Serialize};

/// Window management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct WindowConfig {
    /// Whether window management is enabled.
    pub enabled: bool,
    /// Whether animations are enabled.
    pub animation_enabled: bool,
    /// Animation duration in milliseconds.
    pub animation_duration_ms: u32,
    /// Whether cycling is enabled (Left Half → 50% → 33% → 66%).
    pub cycling_enabled: bool,
    /// Gap between windows and screen edges (0-50px).
    pub window_gap: u32,
    /// Whether to account for menu bar when calculating layouts.
    pub respect_menu_bar: bool,
    /// Whether to account for dock when calculating layouts.
    pub respect_dock: bool,
    /// Timeout for cycling detection in milliseconds.
    pub cycle_timeout_ms: u64,
    /// Margin for almost maximize layout.
    pub almost_maximize_margin: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            animation_enabled: true,
            animation_duration_ms: 200,
            cycling_enabled: true,
            window_gap: 0,
            respect_menu_bar: true,
            respect_dock: true,
            cycle_timeout_ms: 500,
            almost_maximize_margin: 20,
        }
    }
}
