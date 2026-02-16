//! Application configuration.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// User configuration for PhotonCast.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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

    /// Window management settings.
    #[serde(default)]
    pub window_management: WindowManagementConfig,

    /// File search settings.
    #[serde(default)]
    pub file_search: FileSearchConfig,
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

    /// Custom directories to scan for applications.
    /// If empty, defaults are used (`SCAN_PATHS` from scanner).
    /// Changes trigger a re-index.
    #[serde(default)]
    pub app_search_scope: Vec<PathBuf>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            include_system_apps: true,
            file_result_limit: default_file_limit(),
            excluded_apps: Vec::new(),
            app_search_scope: Vec::new(),
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

/// Window management configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowManagementConfig {
    /// Whether window management is enabled.
    pub enabled: bool,
    /// Gap between windows and screen edges (0-50px).
    pub window_gap: u32,
    /// Whether animations are enabled.
    pub animation_enabled: bool,
    /// Whether cycling is enabled (repeatedly pressing same shortcut cycles sizes).
    pub cycling_enabled: bool,
    /// Margin for "almost maximize" layout (pixels).
    pub almost_maximize_margin: u32,
    /// Whether to show visual feedback overlay when positioning windows.
    pub show_visual_feedback: bool,
}

impl Default for WindowManagementConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            window_gap: 0,
            animation_enabled: true,
            cycling_enabled: true,
            almost_maximize_margin: 20,
            show_visual_feedback: true,
        }
    }
}

/// File search configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FileSearchConfig {
    /// Whether file search indexing is enabled.
    pub indexing_enabled: bool,
    /// Whether to index hidden files (files starting with `.`).
    pub index_hidden_files: bool,
    /// Maximum number of file search results to display.
    pub max_results: usize,
    /// Search scopes (directories to index).
    pub search_scopes: Vec<PathBuf>,
    /// Custom search scopes with optional extension filters.
    /// Example: `{ path = "~/code", extensions = ["md", "rs"] }`
    pub custom_scopes: Vec<CustomSearchScope>,
    /// Dedicated file search hotkey (default: Cmd+Shift+F).
    pub hotkey: FileSearchHotkey,
    /// Show file preview panel.
    pub show_preview: bool,
    /// Remember last used file type filter.
    pub remember_filter: bool,
}

impl Default for FileSearchConfig {
    fn default() -> Self {
        Self {
            indexing_enabled: true,
            index_hidden_files: false,
            max_results: 50,
            search_scopes: default_search_scopes(),
            custom_scopes: Vec::new(),
            hotkey: FileSearchHotkey::default(),
            show_preview: true,
            remember_filter: true,
        }
    }
}

/// A custom search scope with optional extension filter.
///
/// # Example Configuration (TOML)
///
/// ```toml
/// [[file_search.custom_scopes]]
/// path = "~/code"
/// extensions = ["md", "rs", "txt"]
/// recursive = true
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomSearchScope {
    /// The directory path to search (supports ~ for home directory).
    pub path: String,
    /// Optional list of file extensions to include (without dots).
    /// If empty or not specified, all files are included.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Whether to search recursively in subdirectories.
    #[serde(default = "default_true")]
    pub recursive: bool,
}

impl CustomSearchScope {
    /// Resolves the path, expanding ~ to home directory.
    #[must_use]
    pub fn resolved_path(&self) -> PathBuf {
        if self.path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&self.path[2..]);
            }
        }
        PathBuf::from(&self.path)
    }

    /// Checks if a file path matches this scope's extension filter.
    #[must_use]
    pub fn matches_extension(&self, path: &std::path::Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
    }
}

/// File search dedicated hotkey configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FileSearchHotkey {
    /// Whether the dedicated hotkey is enabled.
    pub enabled: bool,
    /// The key to use for the hotkey.
    pub key: String,
    /// The modifiers to use for the hotkey.
    pub modifiers: Vec<String>,
}

impl Default for FileSearchHotkey {
    fn default() -> Self {
        Self {
            enabled: true,
            key: "F".to_string(),
            modifiers: vec!["Command".to_string(), "Shift".to_string()],
        }
    }
}

impl FileSearchHotkey {
    /// Returns a formatted string representation of the hotkey (e.g., "⌘⇧F").
    #[must_use]
    pub fn display_string(&self) -> String {
        let mut result = String::new();
        for modifier in &self.modifiers {
            match modifier.as_str() {
                "Command" | "Cmd" => result.push('⌘'),
                "Shift" => result.push('⇧'),
                "Control" | "Ctrl" => result.push('⌃'),
                "Option" | "Alt" => result.push('⌥'),
                _ => {},
            }
        }
        result.push_str(&self.key);
        result
    }
}

/// Returns the default search scopes (Desktop, Documents, Downloads).
///
/// These are the recommended default directories for file search indexing.
/// They cover the most commonly used user file locations without indexing
/// system files, code repositories, or other non-user content.
#[must_use]
pub fn default_search_scopes() -> Vec<PathBuf> {
    let mut scopes = Vec::new();
    if let Some(home) = dirs::home_dir() {
        scopes.push(home.join("Desktop"));
        scopes.push(home.join("Documents"));
        scopes.push(home.join("Downloads"));
    }
    scopes
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
    fn test_window_management_config_default() {
        let wm = WindowManagementConfig::default();
        assert!(wm.enabled);
        assert_eq!(wm.window_gap, 0);
        assert!(wm.animation_enabled);
        assert!(wm.cycling_enabled);
        assert_eq!(wm.almost_maximize_margin, 20);
        assert!(wm.show_visual_feedback);
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

    // ========================================================================
    // Phase 1: SearchConfig.app_search_scope tests
    // ========================================================================

    #[test]
    fn test_search_config_default_has_empty_scope() {
        let config = SearchConfig::default();
        assert!(
            config.app_search_scope.is_empty(),
            "default search config should have empty app_search_scope"
        );
    }

    #[test]
    fn test_search_config_deserialize_with_scope() {
        let toml_str = r#"
include_system_apps = true
file_result_limit = 5
app_search_scope = ["/my/apps", "/other/apps"]
"#;
        let config: SearchConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.app_search_scope.len(), 2);
        assert_eq!(config.app_search_scope[0], PathBuf::from("/my/apps"));
        assert_eq!(config.app_search_scope[1], PathBuf::from("/other/apps"));
    }

    #[test]
    fn test_search_config_deserialize_without_scope() {
        let toml_str = r"
include_system_apps = true
file_result_limit = 5
";
        let config: SearchConfig = toml::from_str(toml_str).unwrap();
        assert!(
            config.app_search_scope.is_empty(),
            "missing field should default to empty vec"
        );
    }

    #[test]
    fn test_config_deserialize_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.general.max_results, 10);
        assert!(config.search.include_system_apps);
        assert_eq!(config.search.file_result_limit, 5);
    }

    #[test]
    fn test_config_deserialize_partial_toml() {
        let toml_str = r"
[general]
max_results = 25
launch_at_login = true
";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.max_results, 25);
        assert!(config.general.launch_at_login);
        // Unspecified fields get defaults
        assert_eq!(config.appearance.theme, ThemeSetting::Mocha);
        assert_eq!(config.appearance.animation_duration_ms, 200);
    }

    #[test]
    fn test_default_search_scopes_returns_paths() {
        let scopes = default_search_scopes();
        // Should return Desktop, Documents, Downloads if home dir is available
        if dirs::home_dir().is_some() {
            assert_eq!(scopes.len(), 3);
            assert!(scopes[0].ends_with("Desktop"));
            assert!(scopes[1].ends_with("Documents"));
            assert!(scopes[2].ends_with("Downloads"));
        }
    }

    #[test]
    fn test_config_roundtrip_with_all_fields() {
        let config = Config {
            search: SearchConfig {
                include_system_apps: false,
                file_result_limit: 20,
                excluded_apps: vec!["com.example.app".to_string()],
                app_search_scope: vec![PathBuf::from("/custom/path")],
            },
            ..Config::default()
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert!(!parsed.search.include_system_apps);
        assert_eq!(parsed.search.file_result_limit, 20);
        assert_eq!(parsed.search.excluded_apps, vec!["com.example.app"]);
        assert_eq!(
            parsed.search.app_search_scope,
            vec![PathBuf::from("/custom/path")]
        );
    }
}
