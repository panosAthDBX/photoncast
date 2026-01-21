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

    /// Clipboard history settings.
    #[serde(default)]
    pub clipboard: ClipboardConfig,

    /// Search settings.
    #[serde(default)]
    pub search: SearchConfig,

    /// Calendar settings.
    #[serde(default)]
    pub calendar: CalendarConfig,

    /// App management settings.
    #[serde(default)]
    pub app_management: AppManagementConfig,

    /// Sleep timer settings.
    #[serde(default)]
    pub sleep_timer: SleepTimerConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            hotkey: HotkeyConfig::default(),
            appearance: AppearanceConfig::default(),
            clipboard: ClipboardConfig::default(),
            search: SearchConfig::default(),
            calendar: CalendarConfig::default(),
            app_management: AppManagementConfig::default(),
            sleep_timer: SleepTimerConfig::default(),
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
    #[serde(default)]
    pub accent_color: AccentColor,

    /// Whether window animations are enabled.
    #[serde(default = "default_true")]
    pub window_animation: bool,

    /// Animation duration in milliseconds.
    #[serde(default = "default_animation_duration")]
    pub animation_duration_ms: u32,

    /// Whether to reduce motion (accessibility).
    #[serde(default)]
    pub reduce_motion: bool,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: ThemeSetting::default(),
            accent_color: AccentColor::default(),
            window_animation: true,
            animation_duration_ms: default_animation_duration(),
            reduce_motion: false,
        }
    }
}

/// Theme setting options following Catppuccin design.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSetting {
    /// Follow system appearance.
    Auto,
    /// Catppuccin Latte (light theme).
    Latte,
    /// Catppuccin Frappé (dark - low contrast).
    Frappe,
    /// Catppuccin Macchiato (dark - medium contrast).
    Macchiato,
    /// Catppuccin Mocha (dark - high contrast, default).
    #[default]
    Mocha,
}

/// Accent color options from Catppuccin palette.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccentColor {
    Rosewater,
    Flamingo,
    Pink,
    Mauve,
    Red,
    Maroon,
    Peach,
    Yellow,
    Green,
    Teal,
    Sky,
    Sapphire,
    /// Default accent color.
    #[default]
    Blue,
    Lavender,
}

// ============================================================================
// Theme Conversion Functions (requires UI feature)
// ============================================================================

#[cfg(feature = "ui")]
impl ThemeSetting {
    /// Converts this config theme setting to a Catppuccin flavor.
    /// For `Auto`, this detects the system appearance.
    #[must_use]
    pub fn to_catppuccin_flavor(&self) -> crate::theme::CatppuccinFlavor {
        use crate::theme::CatppuccinFlavor;
        match self {
            Self::Auto => crate::platform::appearance::detect_system_appearance(),
            Self::Latte => CatppuccinFlavor::Latte,
            Self::Frappe => CatppuccinFlavor::Frappe,
            Self::Macchiato => CatppuccinFlavor::Macchiato,
            Self::Mocha => CatppuccinFlavor::Mocha,
        }
    }
}

#[cfg(feature = "ui")]
impl AccentColor {
    /// Converts this config accent color to a theme accent color.
    #[must_use]
    pub fn to_theme_accent(&self) -> crate::theme::AccentColor {
        use crate::theme::AccentColor as ThemeAccent;
        match self {
            Self::Rosewater => ThemeAccent::Rosewater,
            Self::Flamingo => ThemeAccent::Flamingo,
            Self::Pink => ThemeAccent::Pink,
            Self::Mauve => ThemeAccent::Mauve,
            Self::Red => ThemeAccent::Red,
            Self::Maroon => ThemeAccent::Maroon,
            Self::Peach => ThemeAccent::Peach,
            Self::Yellow => ThemeAccent::Yellow,
            Self::Green => ThemeAccent::Green,
            Self::Teal => ThemeAccent::Teal,
            Self::Sky => ThemeAccent::Sky,
            Self::Sapphire => ThemeAccent::Sapphire,
            Self::Blue => ThemeAccent::Blue,
            Self::Lavender => ThemeAccent::Lavender,
        }
    }
}

/// Clipboard history configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClipboardConfig {
    /// Maximum number of items to store.
    #[serde(default = "default_clipboard_history_size")]
    pub history_size: usize,

    /// Number of days to retain items.
    #[serde(default = "default_clipboard_retention_days")]
    pub retention_days: u32,

    /// Whether to store images.
    #[serde(default = "default_true")]
    pub store_images: bool,

    /// Maximum image size in bytes (default 10MB).
    #[serde(default = "default_max_image_size")]
    pub max_image_size: u64,

    /// Bundle IDs of apps to exclude from clipboard history.
    #[serde(default)]
    pub excluded_apps: Vec<String>,

    /// Default action when selecting a clipboard item.
    #[serde(default)]
    pub default_action: ClipboardAction,
}

/// Default action when selecting a clipboard item.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardAction {
    /// Paste directly to frontmost app (default).
    #[default]
    Paste,
    /// Copy to clipboard without pasting.
    Copy,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            history_size: default_clipboard_history_size(),
            retention_days: default_clipboard_retention_days(),
            store_images: true,
            max_image_size: default_max_image_size(),
            excluded_apps: Vec::new(),
            default_action: ClipboardAction::default(),
        }
    }
}

const fn default_clipboard_history_size() -> usize {
    1000
}

const fn default_clipboard_retention_days() -> u32 {
    30
}

const fn default_max_image_size() -> u64 {
    10 * 1024 * 1024 // 10 MB
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

const fn default_file_limit() -> usize {
    5
}

const fn default_animation_duration() -> u32 {
    200
}

// ============================================================================
// Feature Config Wrappers
// ============================================================================

/// Calendar integration configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CalendarConfig {
    /// Whether calendar integration is enabled.
    pub enabled: bool,
    /// Number of days ahead to fetch events (default: 7).
    pub days_ahead: u32,
    /// Whether to show all-day events first in each day's list.
    pub show_all_day_first: bool,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            days_ahead: 7,
            show_all_day_first: true,
        }
    }
}

/// App management configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AppManagementConfig {
    /// Whether deep scan is enabled by default for uninstaller.
    pub deep_scan_default: bool,
    /// App sleep configuration.
    pub app_sleep: AppSleepConfig,
}

impl Default for AppManagementConfig {
    fn default() -> Self {
        Self {
            deep_scan_default: true,
            app_sleep: AppSleepConfig::default(),
        }
    }
}

/// App sleep feature configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSleepConfig {
    /// Whether app sleep is enabled.
    pub enabled: bool,
    /// Default idle timeout in minutes before sleeping an app.
    pub default_idle_minutes: u32,
}

impl Default for AppSleepConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_idle_minutes: 30,
        }
    }
}

/// Sleep timer configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SleepTimerConfig {
    /// Whether sleep timer is enabled.
    pub enabled: bool,
    /// Warning time in minutes before executing action.
    pub warning_minutes: u32,
    /// Whether to show timer countdown in menu bar.
    pub show_in_menu_bar: bool,
}

impl Default for SleepTimerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            warning_minutes: 1,
            show_in_menu_bar: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.general.max_results, 10);
        assert!(!config.general.launch_at_login);
        assert_eq!(config.appearance.theme, ThemeSetting::Mocha);
        assert_eq!(config.appearance.accent_color, AccentColor::Blue);
        assert!(config.appearance.window_animation);
        assert_eq!(config.appearance.animation_duration_ms, 200);
    }

    #[test]
    fn test_theme_setting_default() {
        assert_eq!(ThemeSetting::default(), ThemeSetting::Mocha);
    }

    #[test]
    fn test_accent_color_default() {
        assert_eq!(AccentColor::default(), AccentColor::Blue);
    }

    #[test]
    fn test_theme_setting_serialization() {
        let theme = ThemeSetting::Mocha;
        let json = serde_json::to_string(&theme).unwrap();
        assert_eq!(json, r#""mocha""#);

        let theme: ThemeSetting = serde_json::from_str(&json).unwrap();
        assert_eq!(theme, ThemeSetting::Mocha);
    }

    #[test]
    fn test_accent_color_serialization() {
        let color = AccentColor::Blue;
        let json = serde_json::to_string(&color).unwrap();
        assert_eq!(json, r#""blue""#);

        let color: AccentColor = serde_json::from_str(&json).unwrap();
        assert_eq!(color, AccentColor::Blue);
    }

    #[test]
    fn test_appearance_config_default() {
        let appearance = AppearanceConfig::default();
        assert_eq!(appearance.theme, ThemeSetting::Mocha);
        assert_eq!(appearance.accent_color, AccentColor::Blue);
        assert!(appearance.window_animation);
        assert_eq!(appearance.animation_duration_ms, 200);
        assert!(!appearance.reduce_motion);
    }

    #[test]
    fn test_general_config_default() {
        let general = GeneralConfig::default();
        assert_eq!(general.max_results, 10);
        assert!(!general.launch_at_login);
        assert!(!general.show_in_dock);
        assert!(general.show_in_menu_bar);
    }

    #[test]
    fn test_sleep_timer_config_default() {
        let timer = SleepTimerConfig::default();
        assert!(timer.enabled);
        assert_eq!(timer.warning_minutes, 1);
        assert!(timer.show_in_menu_bar);
    }

    #[test]
    fn test_calendar_config_default() {
        let calendar = CalendarConfig::default();
        assert!(calendar.enabled);
        assert_eq!(calendar.days_ahead, 7);
        assert!(calendar.show_all_day_first);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(!toml.is_empty());

        let parsed: Config = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.general.max_results, config.general.max_results);
        assert_eq!(parsed.appearance.theme, config.appearance.theme);
        assert_eq!(
            parsed.appearance.accent_color,
            config.appearance.accent_color
        );
    }

    #[test]
    fn test_all_themes() {
        let themes = [
            ThemeSetting::Auto,
            ThemeSetting::Latte,
            ThemeSetting::Frappe,
            ThemeSetting::Macchiato,
            ThemeSetting::Mocha,
        ];
        assert_eq!(themes.len(), 5);
    }

    #[test]
    fn test_all_accent_colors() {
        let colors = vec![
            AccentColor::Rosewater,
            AccentColor::Flamingo,
            AccentColor::Pink,
            AccentColor::Mauve,
            AccentColor::Red,
            AccentColor::Maroon,
            AccentColor::Peach,
            AccentColor::Yellow,
            AccentColor::Green,
            AccentColor::Teal,
            AccentColor::Sky,
            AccentColor::Sapphire,
            AccentColor::Blue,
            AccentColor::Lavender,
        ];
        assert_eq!(colors.len(), 14);
    }
}
