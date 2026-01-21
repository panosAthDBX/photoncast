//! Preferences UI components.
//!
//! This module contains placeholder UI components for the preferences screen.
//! These will be implemented with actual GPUI components in future iterations.

use crate::app::config::{AccentColor, Config, ThemeSetting};

/// Preferences view state (placeholder for GPUI implementation).
#[derive(Debug, Clone)]
pub struct PreferencesView {
    /// Current section being displayed.
    pub current_section: PreferencesSection,
    /// Configuration being edited.
    pub config: Config,
    /// Whether there are unsaved changes.
    pub has_unsaved_changes: bool,
}

/// Sections of the preferences view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferencesSection {
    /// General settings.
    General,
    /// Appearance settings (theme, accent, animations).
    Appearance,
    /// Keyboard shortcuts.
    Shortcuts,
    /// Clipboard history settings.
    Clipboard,
    /// Calculator settings.
    Calculator,
    /// Window management settings.
    WindowManagement,
    /// Calendar integration settings.
    Calendar,
    /// App management settings.
    AppManagement,
    /// Quick links settings.
    QuickLinks,
    /// Sleep timer settings.
    SleepTimer,
}

impl PreferencesSection {
    /// Returns the display name of the section.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Shortcuts => "Keyboard Shortcuts",
            Self::Clipboard => "Clipboard",
            Self::Calculator => "Calculator",
            Self::WindowManagement => "Window Management",
            Self::Calendar => "Calendar",
            Self::AppManagement => "App Management",
            Self::QuickLinks => "Quick Links",
            Self::SleepTimer => "Sleep Timer",
        }
    }

    /// Returns the icon for the section.
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::General => "gear",
            Self::Appearance => "paintbrush",
            Self::Shortcuts => "keyboard",
            Self::Clipboard => "clipboard",
            Self::Calculator => "function",
            Self::WindowManagement => "rectangle-split",
            Self::Calendar => "calendar",
            Self::AppManagement => "app",
            Self::QuickLinks => "link",
            Self::SleepTimer => "clock",
        }
    }

    /// Returns all available sections.
    pub fn all() -> Vec<Self> {
        vec![
            Self::General,
            Self::Appearance,
            Self::Shortcuts,
            Self::Clipboard,
            Self::Calculator,
            Self::WindowManagement,
            Self::Calendar,
            Self::AppManagement,
            Self::QuickLinks,
            Self::SleepTimer,
        ]
    }
}

impl PreferencesView {
    /// Creates a new preferences view with the given config.
    pub fn new(config: Config) -> Self {
        Self {
            current_section: PreferencesSection::General,
            config,
            has_unsaved_changes: false,
        }
    }

    /// Switches to a different section.
    pub fn switch_section(&mut self, section: PreferencesSection) {
        self.current_section = section;
    }

    /// Updates a setting and marks as having unsaved changes.
    pub fn mark_dirty(&mut self) {
        self.has_unsaved_changes = true;
    }

    /// Saves the configuration.
    pub fn save(&mut self) -> Result<(), crate::app::ConfigFileError> {
        crate::app::save_config(&self.config)?;
        self.has_unsaved_changes = false;
        Ok(())
    }

    /// Discards changes by reloading the config.
    pub fn discard_changes(&mut self) -> Result<(), crate::app::ConfigFileError> {
        self.config = crate::app::load_config()?;
        self.has_unsaved_changes = false;
        Ok(())
    }
}

/// Appearance section UI (placeholder).
pub struct AppearanceSection;

impl AppearanceSection {
    /// Returns all available theme options.
    pub fn themes() -> Vec<(ThemeSetting, &'static str)> {
        vec![
            (ThemeSetting::Auto, "Auto (Follow System)"),
            (ThemeSetting::Latte, "Latte (Light)"),
            (ThemeSetting::Frappe, "Frappé (Dark - Low Contrast)"),
            (
                ThemeSetting::Macchiato,
                "Macchiato (Dark - Medium Contrast)",
            ),
            (ThemeSetting::Mocha, "Mocha (Dark - High Contrast)"),
        ]
    }

    /// Returns all available accent color options.
    pub fn accent_colors() -> Vec<(AccentColor, &'static str)> {
        vec![
            (AccentColor::Rosewater, "Rosewater"),
            (AccentColor::Flamingo, "Flamingo"),
            (AccentColor::Pink, "Pink"),
            (AccentColor::Mauve, "Mauve"),
            (AccentColor::Red, "Red"),
            (AccentColor::Maroon, "Maroon"),
            (AccentColor::Peach, "Peach"),
            (AccentColor::Yellow, "Yellow"),
            (AccentColor::Green, "Green"),
            (AccentColor::Teal, "Teal"),
            (AccentColor::Sky, "Sky"),
            (AccentColor::Sapphire, "Sapphire"),
            (AccentColor::Blue, "Blue"),
            (AccentColor::Lavender, "Lavender"),
        ]
    }
}

/// Clipboard section UI (placeholder).
pub struct ClipboardSection;

impl ClipboardSection {
    /// Returns suggested history size options.
    pub fn history_size_options() -> Vec<usize> {
        vec![100, 500, 1000, 2000, 5000]
    }

    /// Returns suggested retention day options.
    pub fn retention_day_options() -> Vec<u32> {
        vec![7, 14, 30, 60, 90, 365]
    }

    /// Returns default excluded apps (password managers).
    pub fn default_excluded_apps() -> Vec<&'static str> {
        vec![
            "com.1password.1password",
            "com.agilebits.onepassword7",
            "com.bitwarden.desktop",
            "com.lastpass.LastPass",
            "com.apple.keychainaccess",
            "com.dashlane.Dashlane",
        ]
    }
}

/// Shortcuts section UI (placeholder).
pub struct ShortcutsSection;

impl ShortcutsSection {
    /// Returns suggested shortcut presets for window management.
    pub fn window_management_presets() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Left Half", "Hyper+Left"),
            ("Right Half", "Hyper+Right"),
            ("Top Half", "Hyper+Up"),
            ("Bottom Half", "Hyper+Down"),
            ("Maximize", "Hyper+Return"),
            ("Center", "Hyper+C"),
        ]
    }

    /// Returns whether a key combination is valid.
    pub fn is_valid_shortcut(key: &str, modifiers: &[String]) -> bool {
        // Basic validation
        !key.is_empty() && !modifiers.is_empty()
    }
}

/// General section UI (placeholder).
pub struct GeneralSection;

impl GeneralSection {
    /// Returns available max results options.
    pub fn max_results_options() -> Vec<usize> {
        vec![5, 10, 15, 20, 25, 30]
    }
}

// TODO: Implement actual GPUI views for all sections
// TODO: Implement theme preview
// TODO: Implement live accent color preview
// TODO: Implement shortcut capture widget
// TODO: Implement excluded apps management UI
// TODO: Implement settings validation and error display

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferences_section_all() {
        let sections = PreferencesSection::all();
        assert_eq!(sections.len(), 10);
        assert!(sections.contains(&PreferencesSection::General));
        assert!(sections.contains(&PreferencesSection::Appearance));
    }

    #[test]
    fn test_preferences_section_name() {
        assert_eq!(PreferencesSection::General.name(), "General");
        assert_eq!(PreferencesSection::Appearance.name(), "Appearance");
    }

    #[test]
    fn test_preferences_section_icon() {
        assert_eq!(PreferencesSection::General.icon(), "gear");
        assert_eq!(PreferencesSection::Appearance.icon(), "paintbrush");
    }

    #[test]
    fn test_preferences_view_new() {
        let config = Config::default();
        let view = PreferencesView::new(config);
        assert_eq!(view.current_section, PreferencesSection::General);
        assert!(!view.has_unsaved_changes);
    }

    #[test]
    fn test_preferences_view_switch_section() {
        let config = Config::default();
        let mut view = PreferencesView::new(config);

        view.switch_section(PreferencesSection::Appearance);
        assert_eq!(view.current_section, PreferencesSection::Appearance);
    }

    #[test]
    fn test_preferences_view_mark_dirty() {
        let config = Config::default();
        let mut view = PreferencesView::new(config);

        assert!(!view.has_unsaved_changes);
        view.mark_dirty();
        assert!(view.has_unsaved_changes);
    }

    #[test]
    fn test_appearance_section_themes() {
        let themes = AppearanceSection::themes();
        assert_eq!(themes.len(), 5);
        assert_eq!(themes[0].0, ThemeSetting::Auto);
        assert_eq!(themes[4].0, ThemeSetting::Mocha);
    }

    #[test]
    fn test_appearance_section_accent_colors() {
        let colors = AppearanceSection::accent_colors();
        assert_eq!(colors.len(), 14);
        assert_eq!(colors[12].0, AccentColor::Blue);
    }

    #[test]
    fn test_clipboard_section_options() {
        let sizes = ClipboardSection::history_size_options();
        assert!(sizes.contains(&1000));

        let days = ClipboardSection::retention_day_options();
        assert!(days.contains(&30));
    }

    #[test]
    fn test_shortcuts_section_validation() {
        assert!(ShortcutsSection::is_valid_shortcut(
            "A",
            &["Command".to_string()]
        ));
        assert!(!ShortcutsSection::is_valid_shortcut(
            "",
            &["Command".to_string()]
        ));
        assert!(!ShortcutsSection::is_valid_shortcut("A", &[]));
    }

    #[test]
    fn test_general_section_options() {
        let options = GeneralSection::max_results_options();
        assert!(options.contains(&10));
        assert!(options.contains(&25));
    }
}
