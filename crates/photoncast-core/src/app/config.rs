//! Application configuration.

use serde::{Deserialize, Serialize};

/// User configuration for PhotonCast.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// General application settings.
    #[serde(default)]
    pub general: GeneralConfig,

    /// Hotkey settings.
    #[serde(default)]
    pub hotkey: HotkeyConfig,

    /// Appearance settings.
    #[serde(default)]
    pub appearance: AppearanceConfig,

    /// Search settings.
    #[serde(default)]
    pub search: SearchConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            hotkey: HotkeyConfig::default(),
            appearance: AppearanceConfig::default(),
            search: SearchConfig::default(),
        }
    }
}

/// General application settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneralConfig {
    /// Maximum number of results to display.
    #[serde(default = "default_max_results")]
    pub max_results: usize,

    /// Whether to launch at login.
    #[serde(default)]
    pub launch_at_login: bool,

    /// Whether to show in dock.
    #[serde(default)]
    pub show_in_dock: bool,

    /// Whether to show in menu bar.
    #[serde(default = "default_true")]
    pub show_in_menu_bar: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            max_results: default_max_results(),
            launch_at_login: false,
            show_in_dock: false,
            show_in_menu_bar: true,
        }
    }
}

/// Hotkey configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HotkeyConfig {
    /// The key to use for the hotkey.
    #[serde(default = "default_hotkey")]
    pub key: String,

    /// The modifiers to use for the hotkey.
    #[serde(default = "default_modifiers")]
    pub modifiers: Vec<String>,

    /// Optional double-tap modifier.
    #[serde(default)]
    pub double_tap_modifier: Option<String>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            key: default_hotkey(),
            modifiers: default_modifiers(),
            double_tap_modifier: None,
        }
    }
}

/// Appearance configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppearanceConfig {
    /// Theme setting.
    #[serde(default)]
    pub theme: ThemeSetting,

    /// Accent color.
    #[serde(default = "default_accent")]
    pub accent_color: String,

    /// Whether to reduce motion.
    #[serde(default)]
    pub reduce_motion: bool,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: ThemeSetting::default(),
            accent_color: default_accent(),
            reduce_motion: false,
        }
    }
}

/// Theme setting options.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSetting {
    /// Follow system theme.
    #[default]
    System,
    /// Light theme.
    Light,
    /// Dark theme.
    Dark,
    /// Catppuccin Latte.
    Latte,
    /// Catppuccin Frappe.
    Frappe,
    /// Catppuccin Macchiato.
    Macchiato,
    /// Catppuccin Mocha.
    Mocha,
}

/// Search configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    /// Whether to include system apps.
    #[serde(default = "default_true")]
    pub include_system_apps: bool,

    /// Maximum number of file results.
    #[serde(default = "default_file_limit")]
    pub file_result_limit: usize,

    /// List of excluded apps by bundle ID.
    #[serde(default)]
    pub excluded_apps: Vec<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            include_system_apps: true,
            file_result_limit: default_file_limit(),
            excluded_apps: Vec::new(),
        }
    }
}

// Default value functions
const fn default_max_results() -> usize {
    10
}

const fn default_true() -> bool {
    true
}

fn default_hotkey() -> String {
    "Space".to_string()
}

fn default_modifiers() -> Vec<String> {
    vec!["Command".to_string()]
}

fn default_accent() -> String {
    "mauve".to_string()
}

const fn default_file_limit() -> usize {
    5
}
