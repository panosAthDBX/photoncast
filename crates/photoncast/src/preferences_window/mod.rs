//! Preferences window UI with actual config management.

mod app_management;
mod appearance;
mod calendar;
mod clipboard;
mod extensions;
mod file_search;
mod general;
mod shortcuts;
mod sleep_timer;
mod window_management;

use std::sync::Arc;

use gpui::prelude::*;
use gpui::*;
use parking_lot::RwLock;
use photoncast_core::app::config::{
    default_search_scopes, AccentColor, ClipboardAction, Config, CustomSearchScope, ThemeSetting,
};
use photoncast_core::app::config_file::{load_config, save_config};
use photoncast_core::app::{ExtensionState, PhotonCastApp};
use photoncast_core::platform::LoginItemManager;
use photoncast_core::ui::animations::set_reduce_motion_override;
use photoncast_theme::{GpuiThemeColors, PhotonTheme};

use crate::constants::{ICON_SIZE_MD, ICON_SIZE_SM, SECTION_GAP, TEXT_SIZE_SM};
use crate::file_search_helper::reload_live_index;

/// Helper type alias – preferences uses the shared [`GpuiThemeColors`].
type PrefsColors = GpuiThemeColors;

fn get_colors(cx: &ViewContext<PreferencesWindow>) -> PrefsColors {
    GpuiThemeColors::from_context(cx)
}

/// Preference sections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferenceSection {
    General,
    Appearance,
    Clipboard,
    Calendar,
    FileSearch,
    Extensions,
    AppManagement,
    WindowManagement,
    SleepTimer,
    Shortcuts,
}

impl PreferenceSection {
    fn all() -> &'static [Self] {
        &[
            Self::General,
            Self::Appearance,
            Self::Clipboard,
            Self::Calendar,
            Self::FileSearch,
            Self::Extensions,
            Self::AppManagement,
            Self::WindowManagement,
            Self::SleepTimer,
            Self::Shortcuts,
        ]
    }

    fn name(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Clipboard => "Clipboard",
            Self::Calendar => "Calendar",
            Self::FileSearch => "File Search",
            Self::Extensions => "Extensions",
            Self::AppManagement => "App Management",
            Self::WindowManagement => "Window Management",
            Self::SleepTimer => "Sleep Timer",
            Self::Shortcuts => "Keyboard Shortcuts",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::General => "⚙️",
            Self::Appearance => "🎨",
            Self::Clipboard => "📋",
            Self::Calendar => "📅",
            Self::FileSearch => "🔍",
            Self::Extensions => "🧩",
            Self::AppManagement => "📦",
            Self::WindowManagement => "🪟",
            Self::SleepTimer => "⏰",
            Self::Shortcuts => "⌨️",
        }
    }
}

/// Preferences window view with actual config state.
pub struct PreferencesWindow {
    focus_handle: FocusHandle,
    selected_section: PreferenceSection,
    config: Config,
    has_changes: bool,
    /// Track which custom scope is being edited (path -> extensions input text)
    editing_scope_extensions: Option<(String, String)>,
    /// Reference to the PhotonCast application for extension management
    photoncast_app: Option<Arc<RwLock<PhotonCastApp>>>,
}

impl PreferencesWindow {
    #[must_use]
    pub fn new(
        cx: &mut ViewContext<Self>,
        photoncast_app: Option<Arc<RwLock<PhotonCastApp>>>,
    ) -> Self {
        let config = load_config().unwrap_or_default();

        // Sync reduce motion setting with the animation system
        set_reduce_motion_override(Some(config.appearance.reduce_motion));

        Self {
            focus_handle: cx.focus_handle(),
            selected_section: PreferenceSection::General,
            config,
            has_changes: false,
            editing_scope_extensions: None,
            photoncast_app,
        }
    }

    fn select_section(&mut self, section: PreferenceSection, cx: &mut ViewContext<Self>) {
        self.selected_section = section;
        cx.notify();
    }

    fn save_config(&mut self) {
        if self.has_changes {
            if let Err(e) = save_config(&self.config) {
                tracing::error!("Failed to save config: {}", e);
            } else {
                tracing::info!("Config saved");
                self.has_changes = false;
            }
        }
    }

    // ==================== General Section Handlers ====================

    fn toggle_launch_at_login(&mut self, cx: &mut ViewContext<Self>) {
        self.config.general.launch_at_login = !self.config.general.launch_at_login;
        self.has_changes = true;
        self.save_config();

        let mut manager = LoginItemManager::new("app.photoncast");
        if self.config.general.launch_at_login {
            if let Err(e) = manager.enable() {
                tracing::error!("Failed to enable launch at login: {}", e);
            }
        } else if let Err(e) = manager.disable() {
            tracing::error!("Failed to disable launch at login: {}", e);
        }

        cx.notify();
    }

    fn toggle_show_in_menu_bar(&mut self, cx: &mut ViewContext<Self>) {
        self.config.general.show_in_menu_bar = !self.config.general.show_in_menu_bar;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_show_in_dock(&mut self, cx: &mut ViewContext<Self>) {
        self.config.general.show_in_dock = !self.config.general.show_in_dock;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn increment_max_results(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.general.max_results < 20 {
            self.config.general.max_results += 1;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_max_results(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.general.max_results > 3 {
            self.config.general.max_results -= 1;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    // ==================== Appearance Section Handlers ====================

    fn toggle_reduce_motion(&mut self, cx: &mut ViewContext<Self>) {
        self.config.appearance.reduce_motion = !self.config.appearance.reduce_motion;
        self.has_changes = true;
        self.save_config();
        set_reduce_motion_override(Some(self.config.appearance.reduce_motion));
        cx.notify();
    }

    fn toggle_window_animation(&mut self, cx: &mut ViewContext<Self>) {
        self.config.appearance.window_animation = !self.config.appearance.window_animation;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn set_theme(&mut self, theme: ThemeSetting, cx: &mut ViewContext<Self>) {
        self.config.appearance.theme = theme;
        self.has_changes = true;
        self.save_config();
        self.update_global_theme(cx);
        cx.notify();
    }

    fn set_accent_color(&mut self, color: AccentColor, cx: &mut ViewContext<Self>) {
        self.config.appearance.accent_color = color;
        self.has_changes = true;
        self.save_config();
        self.update_global_theme(cx);
        cx.notify();
    }

    /// Updates the global theme based on current config settings.
    fn update_global_theme(&self, cx: &mut ViewContext<Self>) {
        let flavor = self.config.appearance.theme.to_catppuccin_flavor();
        let accent = self.config.appearance.accent_color.to_theme_accent();
        let theme = PhotonTheme::new(flavor, accent);
        cx.set_global(theme);
        tracing::info!("Theme updated: flavor={:?}, accent={:?}", flavor, accent);
    }

    // ==================== Clipboard Section Handlers ====================

    fn toggle_clipboard_store_images(&mut self, cx: &mut ViewContext<Self>) {
        self.config.clipboard.store_images = !self.config.clipboard.store_images;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn increment_clipboard_history_size(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.clipboard.history_size < 5000 {
            self.config.clipboard.history_size += 100;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_clipboard_history_size(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.clipboard.history_size > 100 {
            self.config.clipboard.history_size -= 100;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn increment_clipboard_retention(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.clipboard.retention_days < 365 {
            self.config.clipboard.retention_days += 7;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_clipboard_retention(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.clipboard.retention_days > 1 {
            self.config.clipboard.retention_days = self
                .config
                .clipboard
                .retention_days
                .saturating_sub(7)
                .max(1);
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn toggle_clipboard_default_action(&mut self, cx: &mut ViewContext<Self>) {
        self.config.clipboard.default_action = match self.config.clipboard.default_action {
            ClipboardAction::Paste => ClipboardAction::Copy,
            ClipboardAction::Copy => ClipboardAction::Paste,
        };
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn increment_max_image_size(&mut self, cx: &mut ViewContext<Self>) {
        // Max 50MB
        if self.config.clipboard.max_image_size < 50 * 1024 * 1024 {
            self.config.clipboard.max_image_size += 5 * 1024 * 1024; // +5MB
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_max_image_size(&mut self, cx: &mut ViewContext<Self>) {
        // Min 1MB
        if self.config.clipboard.max_image_size > 1024 * 1024 {
            self.config.clipboard.max_image_size = self
                .config
                .clipboard
                .max_image_size
                .saturating_sub(5 * 1024 * 1024)
                .max(1024 * 1024);
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    #[allow(dead_code)]
    fn add_excluded_app(&mut self, bundle_id: String, cx: &mut ViewContext<Self>) {
        if !self.config.clipboard.excluded_apps.contains(&bundle_id) {
            self.config.clipboard.excluded_apps.push(bundle_id);
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn remove_excluded_app(&mut self, bundle_id: &str, cx: &mut ViewContext<Self>) {
        self.config
            .clipboard
            .excluded_apps
            .retain(|app| app != bundle_id);
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn add_default_excluded_apps(&mut self, cx: &mut ViewContext<Self>) {
        let defaults = [
            "com.1password.1password",
            "com.agilebits.onepassword7",
            "com.bitwarden.desktop",
            "com.lastpass.LastPass",
            "com.apple.keychainaccess",
            "com.dashlane.Dashlane",
        ];
        for app in defaults {
            if !self
                .config
                .clipboard
                .excluded_apps
                .contains(&app.to_string())
            {
                self.config.clipboard.excluded_apps.push(app.to_string());
            }
        }
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    // ==================== App Management Handlers ====================

    fn toggle_deep_scan_default(&mut self, cx: &mut ViewContext<Self>) {
        self.config.app_management.deep_scan_default =
            !self.config.app_management.deep_scan_default;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_app_sleep_enabled(&mut self, cx: &mut ViewContext<Self>) {
        self.config.app_management.app_sleep.enabled =
            !self.config.app_management.app_sleep.enabled;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn increment_app_sleep_idle_minutes(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.app_management.app_sleep.default_idle_minutes < 120 {
            self.config.app_management.app_sleep.default_idle_minutes += 5;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_app_sleep_idle_minutes(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.app_management.app_sleep.default_idle_minutes > 5 {
            self.config.app_management.app_sleep.default_idle_minutes -= 5;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    // ==================== Calendar Handlers ====================

    fn toggle_calendar_enabled(&mut self, cx: &mut ViewContext<Self>) {
        self.config.calendar.enabled = !self.config.calendar.enabled;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_show_all_day_first(&mut self, cx: &mut ViewContext<Self>) {
        self.config.calendar.show_all_day_first = !self.config.calendar.show_all_day_first;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn increment_days_ahead(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.calendar.days_ahead < 30 {
            self.config.calendar.days_ahead += 1;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_days_ahead(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.calendar.days_ahead > 1 {
            self.config.calendar.days_ahead -= 1;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    // ==================== Window Management Handlers ====================

    fn toggle_window_management_enabled(&mut self, cx: &mut ViewContext<Self>) {
        self.config.window_management.enabled = !self.config.window_management.enabled;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_window_cycling(&mut self, cx: &mut ViewContext<Self>) {
        self.config.window_management.cycling_enabled =
            !self.config.window_management.cycling_enabled;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn increment_window_gap(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.window_management.window_gap < 50 {
            self.config.window_management.window_gap += 1;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_window_gap(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.window_management.window_gap > 0 {
            self.config.window_management.window_gap -= 1;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn increment_almost_maximize_margin(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.window_management.almost_maximize_margin < 100 {
            self.config.window_management.almost_maximize_margin += 5;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_almost_maximize_margin(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.window_management.almost_maximize_margin > 0 {
            self.config.window_management.almost_maximize_margin = self
                .config
                .window_management
                .almost_maximize_margin
                .saturating_sub(5);
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn toggle_window_management_animation(&mut self, cx: &mut ViewContext<Self>) {
        self.config.window_management.animation_enabled =
            !self.config.window_management.animation_enabled;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_window_visual_feedback(&mut self, cx: &mut ViewContext<Self>) {
        self.config.window_management.show_visual_feedback =
            !self.config.window_management.show_visual_feedback;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    // ==================== Sleep Timer Handlers ====================

    fn toggle_sleep_timer_enabled(&mut self, cx: &mut ViewContext<Self>) {
        self.config.sleep_timer.enabled = !self.config.sleep_timer.enabled;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_sleep_timer_menu_bar(&mut self, cx: &mut ViewContext<Self>) {
        self.config.sleep_timer.show_in_menu_bar = !self.config.sleep_timer.show_in_menu_bar;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn increment_warning_minutes(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.sleep_timer.warning_minutes < 10 {
            self.config.sleep_timer.warning_minutes += 1;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_warning_minutes(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.sleep_timer.warning_minutes > 0 {
            self.config.sleep_timer.warning_minutes -= 1;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    // ==================== File Search Handlers ====================

    fn toggle_file_search_indexing(&mut self, cx: &mut ViewContext<Self>) {
        self.config.file_search.indexing_enabled = !self.config.file_search.indexing_enabled;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_file_search_hidden_files(&mut self, cx: &mut ViewContext<Self>) {
        self.config.file_search.index_hidden_files = !self.config.file_search.index_hidden_files;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_file_search_preview(&mut self, cx: &mut ViewContext<Self>) {
        self.config.file_search.show_preview = !self.config.file_search.show_preview;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_file_search_remember_filter(&mut self, cx: &mut ViewContext<Self>) {
        self.config.file_search.remember_filter = !self.config.file_search.remember_filter;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_file_search_hotkey_enabled(&mut self, cx: &mut ViewContext<Self>) {
        self.config.file_search.hotkey.enabled = !self.config.file_search.hotkey.enabled;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn increment_file_search_max_results(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.file_search.max_results < 200 {
            self.config.file_search.max_results += 10;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn decrement_file_search_max_results(&mut self, cx: &mut ViewContext<Self>) {
        if self.config.file_search.max_results > 10 {
            self.config.file_search.max_results -= 10;
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn add_file_search_scope(&mut self, path: std::path::PathBuf, cx: &mut ViewContext<Self>) {
        if !self.config.file_search.search_scopes.contains(&path) {
            self.config.file_search.search_scopes.push(path);
            self.has_changes = true;
            self.save_config();
            cx.spawn(|_, _| async {
                reload_live_index();
            })
            .detach();
            cx.notify();
        }
    }

    fn remove_file_search_scope(&mut self, path: &std::path::Path, cx: &mut ViewContext<Self>) {
        self.config.file_search.search_scopes.retain(|p| p != path);
        self.has_changes = true;
        self.save_config();
        cx.spawn(|_, _| async {
            reload_live_index();
        })
        .detach();
        cx.notify();
    }

    fn reset_file_search_scopes_to_default(&mut self, cx: &mut ViewContext<Self>) {
        // Use the canonical default from config module
        self.config.file_search.search_scopes = default_search_scopes();
        self.has_changes = true;
        self.save_config();
        cx.spawn(|_, _| async {
            reload_live_index();
        })
        .detach();
        cx.notify();
    }

    // ==================== Custom Scopes Handlers ====================

    fn add_custom_scope(
        &mut self,
        path: std::path::PathBuf,
        extensions: Vec<String>,
        cx: &mut ViewContext<Self>,
    ) {
        // Convert path to string with ~ for home directory
        let path_str = if let Some(home) = dirs::home_dir() {
            if let (Some(home_str), Some(path_str)) = (home.to_str(), path.to_str()) {
                if let Some(stripped) = path_str.strip_prefix(home_str) {
                    format!("~{}", stripped)
                } else {
                    path_str.to_string()
                }
            } else {
                path.display().to_string()
            }
        } else {
            path.display().to_string()
        };

        // Check if scope with same path already exists
        if self
            .config
            .file_search
            .custom_scopes
            .iter()
            .any(|s| s.path == path_str)
        {
            return;
        }

        let scope = CustomSearchScope {
            path: path_str,
            extensions,
            recursive: true,
        };
        self.config.file_search.custom_scopes.push(scope);
        self.has_changes = true;
        self.save_config();
        cx.spawn(|_, _| async {
            reload_live_index();
        })
        .detach();
        cx.notify();
    }

    fn remove_custom_scope(&mut self, path: &str, cx: &mut ViewContext<Self>) {
        self.config
            .file_search
            .custom_scopes
            .retain(|s| s.path != path);
        self.has_changes = true;
        self.save_config();
        cx.spawn(|_, _| async {
            reload_live_index();
        })
        .detach();
        cx.notify();
    }

    fn clear_custom_scopes(&mut self, cx: &mut ViewContext<Self>) {
        self.config.file_search.custom_scopes.clear();
        self.has_changes = true;
        self.save_config();
        cx.spawn(|_, _| async {
            reload_live_index();
        })
        .detach();
        cx.notify();
    }

    fn toggle_custom_scope_recursive(&mut self, path: &str, cx: &mut ViewContext<Self>) {
        if let Some(scope) = self
            .config
            .file_search
            .custom_scopes
            .iter_mut()
            .find(|s| s.path == path)
        {
            scope.recursive = !scope.recursive;
            self.has_changes = true;
            self.save_config();
            cx.spawn(|_, _| async {
                reload_live_index();
            })
            .detach();
            cx.notify();
        }
    }

    fn set_custom_scope_extensions(
        &mut self,
        path: &str,
        extensions: Vec<String>,
        cx: &mut ViewContext<Self>,
    ) {
        if let Some(scope) = self
            .config
            .file_search
            .custom_scopes
            .iter_mut()
            .find(|s| s.path == path)
        {
            scope.extensions = extensions;
            self.has_changes = true;
            self.save_config();
            cx.spawn(|_, _| async {
                reload_live_index();
            })
            .detach();
            cx.notify();
        }
    }

    fn start_editing_scope_extensions(&mut self, path: &str, cx: &mut ViewContext<Self>) {
        // Get current extensions as comma-separated string
        let current = self
            .config
            .file_search
            .custom_scopes
            .iter()
            .find(|s| s.path == path)
            .map(|s| s.extensions.join(", "))
            .unwrap_or_default();
        self.editing_scope_extensions = Some((path.to_string(), current));
        cx.notify();
    }

    fn update_scope_extensions_input(&mut self, text: String, cx: &mut ViewContext<Self>) {
        if let Some((path, _)) = &self.editing_scope_extensions {
            self.editing_scope_extensions = Some((path.clone(), text));
            cx.notify();
        }
    }

    fn save_scope_extensions_input(&mut self, cx: &mut ViewContext<Self>) {
        if let Some((path, text)) = self.editing_scope_extensions.take() {
            // Parse comma-separated extensions, trim whitespace, remove dots
            let extensions: Vec<String> = text
                .split(',')
                .map(|s| s.trim().trim_start_matches('.').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            self.set_custom_scope_extensions(&path, extensions, cx);
        }
    }

    fn cancel_scope_extensions_edit(&mut self, cx: &mut ViewContext<Self>) {
        self.editing_scope_extensions = None;
        cx.notify();
    }

    fn handle_extensions_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        // Only handle if we're editing extensions
        if self.editing_scope_extensions.is_none() {
            return;
        }

        let key = &event.keystroke.key;
        let cmd = event.keystroke.modifiers.platform;
        let shift = event.keystroke.modifiers.shift;

        // Escape to cancel
        if key == "escape" {
            self.cancel_scope_extensions_edit(cx);
            return;
        }

        // Enter to save
        if key == "enter" {
            self.save_scope_extensions_input(cx);
            return;
        }

        // Backspace to delete last char
        if key == "backspace" {
            if let Some((path, text)) = &self.editing_scope_extensions {
                let mut chars: Vec<char> = text.chars().collect();
                if !chars.is_empty() {
                    chars.pop();
                    let new_text: String = chars.into_iter().collect();
                    self.editing_scope_extensions = Some((path.clone(), new_text));
                    cx.notify();
                }
            }
            return;
        }

        // Ignore modifier combinations
        if cmd || event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
            return;
        }

        // Handle regular character input
        let input_text = if let Some(ime_key) = &event.keystroke.ime_key {
            Some(ime_key.clone())
        } else if key.len() == 1 {
            let ch = if shift {
                key.to_uppercase()
            } else {
                key.to_string()
            };
            Some(ch)
        } else {
            None
        };

        if let Some(text) = input_text {
            if let Some((_, current)) = &self.editing_scope_extensions {
                let new_text = format!("{}{}", current, text);
                self.update_scope_extensions_input(new_text, cx);
            }
        }
    }

    // ==================== Render Methods ====================

    fn render_sidebar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);

        let items: Vec<_> = PreferenceSection::all()
            .iter()
            .map(|&section| {
                let is_selected = section == self.selected_section;
                let bg = if is_selected {
                    colors.selection
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                };

                div()
                    .id(section.name())
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(bg)
                    .hover(|s| s.bg(colors.hover))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .on_click(cx.listener(move |this, _, cx| {
                        this.select_section(section, cx);
                    }))
                    .child(div().text_size(px(14.0)).child(section.icon()))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(if is_selected {
                                colors.text
                            } else {
                                colors.text_muted
                            })
                            .child(section.name()),
                    )
            })
            .collect();

        div()
            .w(px(180.0))
            .h_full()
            .border_r_1()
            .border_color(colors.border)
            .py(px(8.0))
            .px(px(8.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(items)
    }

    fn render_content(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let content: gpui::AnyElement = match self.selected_section {
            PreferenceSection::General => self.render_general_section(cx).into_any_element(),
            PreferenceSection::Appearance => self.render_appearance_section(cx).into_any_element(),
            PreferenceSection::Clipboard => self.render_clipboard_section(cx).into_any_element(),
            PreferenceSection::Calendar => self.render_calendar_section(cx).into_any_element(),
            PreferenceSection::FileSearch => self.render_file_search_section(cx).into_any_element(),
            PreferenceSection::Extensions => self.render_extensions_section(cx).into_any_element(),
            PreferenceSection::AppManagement => {
                self.render_app_management_section(cx).into_any_element()
            },
            PreferenceSection::WindowManagement => {
                self.render_window_management_section(cx).into_any_element()
            },
            PreferenceSection::SleepTimer => self.render_sleep_timer_section(cx).into_any_element(),
            PreferenceSection::Shortcuts => self.render_shortcuts_section(cx).into_any_element(),
        };

        let colors = get_colors(cx);

        div()
            .flex_1()
            .h_full()
            .p(px(20.0))
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            .child(
                div()
                    .text_size(px(18.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child(self.selected_section.name()),
            )
            .child(content)
    }

    // ==================== Shared Render Helpers ====================

    fn render_toggle_row(
        &self,
        id: &'static str,
        label: &'static str,
        description: &'static str,
        enabled: bool,
        colors: &PrefsColors,
    ) -> gpui::Stateful<gpui::Div> {
        let toggle_bg = if enabled {
            colors.accent
        } else {
            colors.surface_hover
        };
        let toggle_pos = if enabled { px(18.0) } else { px(2.0) };

        div()
            .id(SharedString::from(id))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child(description),
                    ),
            )
            .child(
                div()
                    .w(px(36.0))
                    .h(px(20.0))
                    .rounded(px(10.0))
                    .bg(toggle_bg)
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top(px(2.0))
                            .left(toggle_pos)
                            .size(ICON_SIZE_SM)
                            .rounded_full()
                            .bg(colors.text),
                    ),
            )
    }

    fn render_number_row<F1, F2>(
        &self,
        label: &'static str,
        description: &'static str,
        value: usize,
        cx: &mut ViewContext<Self>,
        on_decrement: F1,
        on_increment: F2,
    ) -> impl IntoElement
    where
        F1: Fn(&mut Self, &mut ViewContext<Self>) + 'static,
        F2: Fn(&mut Self, &mut ViewContext<Self>) + 'static,
    {
        let colors = get_colors(cx);
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child(description),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id(SharedString::from(format!("{label}-dec")))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| on_decrement(this, cx)))
                            .child(
                                div()
                                    .text_size(TEXT_SIZE_SM)
                                    .text_color(colors.text)
                                    .child("-"),
                            ),
                    )
                    .child(
                        div()
                            .w(px(50.0))
                            .flex()
                            .justify_center()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child(value.to_string()),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("{label}-inc")))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| on_increment(this, cx)))
                            .child(
                                div()
                                    .text_size(TEXT_SIZE_SM)
                                    .text_color(colors.text)
                                    .child("+"),
                            ),
                    ),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_number_row_with_suffix<F1, F2>(
        &self,
        label: &'static str,
        description: &'static str,
        value: usize,
        suffix: &'static str,
        cx: &mut ViewContext<Self>,
        on_decrement: F1,
        on_increment: F2,
    ) -> impl IntoElement
    where
        F1: Fn(&mut Self, &mut ViewContext<Self>) + 'static,
        F2: Fn(&mut Self, &mut ViewContext<Self>) + 'static,
    {
        let colors = get_colors(cx);
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child(description),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id(SharedString::from(format!("{label}-dec")))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| on_decrement(this, cx)))
                            .child(
                                div()
                                    .text_size(TEXT_SIZE_SM)
                                    .text_color(colors.text)
                                    .child("-"),
                            ),
                    )
                    .child(
                        div()
                            .w(px(70.0))
                            .flex()
                            .justify_center()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child(format!("{} {}", value, suffix)),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("{label}-inc")))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| on_increment(this, cx)))
                            .child(
                                div()
                                    .text_size(TEXT_SIZE_SM)
                                    .text_color(colors.text)
                                    .child("+"),
                            ),
                    ),
            )
    }

    fn render_shortcut_row(
        &self,
        label: &'static str,
        shortcut: &str,
        colors: &PrefsColors,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(6.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(colors.text)
                    .child(label),
            )
            .child(
                div()
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(colors.surface_hover)
                    .text_size(TEXT_SIZE_SM)
                    .text_color(colors.text_muted)
                    .child(shortcut.to_string()),
            )
    }
}

impl Render for PreferencesWindow {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);

        div()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_extensions_key_down))
            .size_full()
            .pt(px(28.0))
            .bg(colors.background)
            .flex()
            .child(self.render_sidebar(cx))
            .child(self.render_content(cx))
    }
}

impl FocusableView for PreferencesWindow {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}
