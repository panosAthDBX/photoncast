//! Menu bar icon integration.
//!
//! This module provides functionality for creating and managing
//! a status item (menu bar icon) for PhotonCast on macOS.

use thiserror::Error;
use tracing::{debug, info};

/// Errors that can occur with menu bar operations.
#[derive(Debug, Error)]
pub enum MenuBarError {
    /// Failed to create status item.
    #[error("Failed to create menu bar status item")]
    CreationFailed,

    /// Failed to set icon.
    #[error("Failed to set menu bar icon: {0}")]
    IconFailed(String),

    /// Menu bar is not available.
    #[error("Menu bar is not available")]
    NotAvailable,
}

/// Status of the menu bar icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBarStatus {
    /// Menu bar icon is visible and active.
    Visible,
    /// Menu bar icon is hidden.
    Hidden,
    /// Menu bar icon failed to initialize.
    Failed,
}

/// Configuration for the menu bar icon.
#[derive(Debug, Clone)]
pub struct MenuBarConfig {
    /// Whether to show the menu bar icon.
    pub show_icon: bool,
    /// Icon name or path (for future custom icons).
    pub icon_name: String,
    /// Tooltip text.
    pub tooltip: String,
}

impl Default for MenuBarConfig {
    fn default() -> Self {
        Self {
            show_icon: true,
            icon_name: "magnifyingglass".to_string(),
            tooltip: "PhotonCast".to_string(),
        }
    }
}

/// Menu bar item actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBarAction {
    /// Toggle the launcher window.
    ToggleLauncher,
    /// Open preferences.
    OpenPreferences,
    /// Quit the application.
    Quit,
}

/// Handler for menu bar item clicks.
pub trait MenuBarHandler: Send + Sync {
    /// Called when a menu bar action is triggered.
    fn on_action(&self, action: MenuBarAction);
}

/// Menu bar manager for PhotonCast.
///
/// This struct manages the menu bar status item lifecycle.
/// Note: The actual NSStatusBar integration happens through GPUI
/// when running the app. This module provides the interface and
/// data structures for menu bar functionality.
pub struct MenuBarManager {
    /// Current status.
    status: MenuBarStatus,
    /// Configuration.
    config: MenuBarConfig,
}

impl MenuBarManager {
    /// Creates a new menu bar manager with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(MenuBarConfig::default())
    }

    /// Creates a new menu bar manager with custom configuration.
    #[must_use]
    pub fn with_config(config: MenuBarConfig) -> Self {
        Self {
            status: MenuBarStatus::Hidden,
            config,
        }
    }

    /// Returns the current status.
    #[must_use]
    pub fn status(&self) -> MenuBarStatus {
        self.status
    }

    /// Returns the configuration.
    #[must_use]
    pub fn config(&self) -> &MenuBarConfig {
        &self.config
    }

    /// Initializes the menu bar icon.
    ///
    /// Note: Actual implementation requires GPUI context.
    /// This method sets up the state for integration with GPUI.
    pub fn initialize(&mut self) -> Result<(), MenuBarError> {
        if !self.config.show_icon {
            debug!("Menu bar icon disabled by configuration");
            self.status = MenuBarStatus::Hidden;
            return Ok(());
        }

        info!("Initializing menu bar icon");
        // The actual NSStatusBar integration happens in the GPUI app layer
        // This marks that we want the menu bar icon to be shown
        self.status = MenuBarStatus::Visible;
        Ok(())
    }

    /// Shows the menu bar icon.
    pub fn show(&mut self) {
        if self.status != MenuBarStatus::Visible {
            debug!("Showing menu bar icon");
            self.status = MenuBarStatus::Visible;
        }
    }

    /// Hides the menu bar icon.
    pub fn hide(&mut self) {
        if self.status == MenuBarStatus::Visible {
            debug!("Hiding menu bar icon");
            self.status = MenuBarStatus::Hidden;
        }
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: MenuBarConfig) {
        self.config = config;
    }

    /// Returns true if the menu bar icon should be visible.
    #[must_use]
    pub fn should_show(&self) -> bool {
        self.config.show_icon && self.status == MenuBarStatus::Visible
    }
}

impl Default for MenuBarManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Menu items for the status bar dropdown.
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Display title.
    pub title: String,
    /// Keyboard shortcut (if any).
    pub shortcut: Option<String>,
    /// Action to perform.
    pub action: MenuBarAction,
    /// Whether this is a separator.
    pub is_separator: bool,
}

impl MenuItem {
    /// Creates a new menu item.
    #[must_use]
    pub fn new(title: impl Into<String>, action: MenuBarAction) -> Self {
        Self {
            title: title.into(),
            shortcut: None,
            action,
            is_separator: false,
        }
    }

    /// Sets a keyboard shortcut for this item.
    #[must_use]
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Creates a separator item.
    #[must_use]
    pub fn separator() -> Self {
        Self {
            title: String::new(),
            shortcut: None,
            action: MenuBarAction::ToggleLauncher, // Unused for separator
            is_separator: true,
        }
    }
}

/// Creates the default menu items for PhotonCast.
#[must_use]
pub fn default_menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem::new("Open PhotonCast", MenuBarAction::ToggleLauncher).with_shortcut("⌘Space"),
        MenuItem::separator(),
        MenuItem::new("Preferences...", MenuBarAction::OpenPreferences).with_shortcut("⌘,"),
        MenuItem::separator(),
        MenuItem::new("Quit PhotonCast", MenuBarAction::Quit).with_shortcut("⌘Q"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_bar_config_default() {
        let config = MenuBarConfig::default();
        assert!(config.show_icon);
        assert_eq!(config.icon_name, "magnifyingglass");
        assert_eq!(config.tooltip, "PhotonCast");
    }

    #[test]
    fn test_menu_bar_manager_new() {
        let manager = MenuBarManager::new();
        assert_eq!(manager.status(), MenuBarStatus::Hidden);
    }

    #[test]
    fn test_menu_bar_manager_initialize() {
        let mut manager = MenuBarManager::new();
        manager.initialize().unwrap();
        assert_eq!(manager.status(), MenuBarStatus::Visible);
    }

    #[test]
    fn test_menu_bar_manager_initialize_disabled() {
        let config = MenuBarConfig {
            show_icon: false,
            ..Default::default()
        };
        let mut manager = MenuBarManager::with_config(config);
        manager.initialize().unwrap();
        assert_eq!(manager.status(), MenuBarStatus::Hidden);
    }

    #[test]
    fn test_menu_bar_manager_show_hide() {
        let mut manager = MenuBarManager::new();
        manager.initialize().unwrap();
        assert_eq!(manager.status(), MenuBarStatus::Visible);

        manager.hide();
        assert_eq!(manager.status(), MenuBarStatus::Hidden);

        manager.show();
        assert_eq!(manager.status(), MenuBarStatus::Visible);
    }

    #[test]
    fn test_menu_bar_should_show() {
        let mut manager = MenuBarManager::new();
        assert!(!manager.should_show());

        manager.initialize().unwrap();
        assert!(manager.should_show());

        manager.hide();
        assert!(!manager.should_show());
    }

    #[test]
    fn test_menu_item_new() {
        let item = MenuItem::new("Test", MenuBarAction::ToggleLauncher);
        assert_eq!(item.title, "Test");
        assert!(item.shortcut.is_none());
        assert!(!item.is_separator);
    }

    #[test]
    fn test_menu_item_with_shortcut() {
        let item = MenuItem::new("Test", MenuBarAction::ToggleLauncher).with_shortcut("⌘T");
        assert_eq!(item.shortcut, Some("⌘T".to_string()));
    }

    #[test]
    fn test_menu_item_separator() {
        let item = MenuItem::separator();
        assert!(item.is_separator);
        assert!(item.title.is_empty());
    }

    #[test]
    fn test_default_menu_items() {
        let items = default_menu_items();
        assert!(!items.is_empty());

        // Check for expected items
        assert!(items.iter().any(|i| i.title == "Open PhotonCast"));
        assert!(items.iter().any(|i| i.title == "Preferences..."));
        assert!(items.iter().any(|i| i.title == "Quit PhotonCast"));

        // Check for separators
        assert!(items.iter().any(|i| i.is_separator));
    }

    #[test]
    fn test_menu_bar_action_eq() {
        assert_eq!(MenuBarAction::ToggleLauncher, MenuBarAction::ToggleLauncher);
        assert_ne!(MenuBarAction::ToggleLauncher, MenuBarAction::Quit);
    }
}
