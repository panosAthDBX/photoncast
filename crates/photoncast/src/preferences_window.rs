//! Preferences window UI with actual config management.

use gpui::prelude::*;
use gpui::{
    div, hsla, px, rgb, FocusHandle, FocusableView, FontWeight, Hsla, IntoElement, ParentElement,
    Render, SharedString, Styled, ViewContext,
};
use photoncast_core::app::config::{AccentColor, ClipboardAction, Config, ThemeSetting};
use photoncast_core::app::config_file::{load_config, save_config};
use photoncast_core::platform::LoginItemManager;
use photoncast_core::theme::PhotonTheme;
use photoncast_core::ui::animations::set_reduce_motion_override;

/// Helper struct holding all theme colors needed for preferences UI
#[derive(Clone)]
struct PrefsColors {
    /// Main background
    bg: Hsla,
    /// Surface/card background
    surface: Hsla,
    /// Elevated surface background
    surface_elevated: Hsla,
    /// Surface on hover
    surface_hover: Hsla,
    /// Primary text
    text: Hsla,
    /// Muted/secondary text
    text_muted: Hsla,
    /// Placeholder/hint text
    text_placeholder: Hsla,
    /// Border color
    border: Hsla,
    /// Accent color (for buttons, toggles)
    accent: Hsla,
    /// Selection background
    selection: Hsla,
    /// Hover background
    hover: Hsla,
}

impl PrefsColors {
    fn from_theme(theme: &PhotonTheme) -> Self {
        Self {
            bg: theme.colors.background.to_gpui(),
            surface: theme.colors.surface.to_gpui(),
            surface_elevated: theme.colors.background_elevated.to_gpui(),
            surface_hover: theme.colors.surface_hover.to_gpui(),
            text: theme.colors.text.to_gpui(),
            text_muted: theme.colors.text_muted.to_gpui(),
            text_placeholder: theme.colors.text_placeholder.to_gpui(),
            border: theme.colors.border.to_gpui(),
            accent: theme.colors.accent.to_gpui(),
            selection: theme.colors.selection.to_gpui(),
            hover: theme.colors.hover.to_gpui(),
        }
    }
}

fn get_colors(cx: &ViewContext<PreferencesWindow>) -> PrefsColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    PrefsColors::from_theme(&theme)
}

/// Preference sections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferenceSection {
    General,
    Appearance,
    Clipboard,
    Calendar,
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
}

impl PreferencesWindow {
    #[must_use]
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let config = load_config().unwrap_or_default();

        // Sync reduce motion setting with the animation system
        set_reduce_motion_override(Some(config.appearance.reduce_motion));

        Self {
            focus_handle: cx.focus_handle(),
            selected_section: PreferenceSection::General,
            config,
            has_changes: false,
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
            self.config.clipboard.retention_days = self.config.clipboard.retention_days.saturating_sub(7).max(1);
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
            self.config.clipboard.max_image_size = self.config.clipboard.max_image_size.saturating_sub(5 * 1024 * 1024).max(1024 * 1024);
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn add_excluded_app(&mut self, bundle_id: String, cx: &mut ViewContext<Self>) {
        if !self.config.clipboard.excluded_apps.contains(&bundle_id) {
            self.config.clipboard.excluded_apps.push(bundle_id);
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn remove_excluded_app(&mut self, bundle_id: &str, cx: &mut ViewContext<Self>) {
        self.config.clipboard.excluded_apps.retain(|app| app != bundle_id);
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
            if !self.config.clipboard.excluded_apps.contains(&app.to_string()) {
                self.config.clipboard.excluded_apps.push(app.to_string());
            }
        }
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    // ==================== App Management Handlers ====================

    fn toggle_deep_scan_default(&mut self, cx: &mut ViewContext<Self>) {
        self.config.app_management.deep_scan_default = !self.config.app_management.deep_scan_default;
        self.has_changes = true;
        self.save_config();
        cx.notify();
    }

    fn toggle_app_sleep_enabled(&mut self, cx: &mut ViewContext<Self>) {
        self.config.app_management.app_sleep.enabled = !self.config.app_management.app_sleep.enabled;
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
        self.config.window_management.cycling_enabled = !self.config.window_management.cycling_enabled;
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
            self.config.window_management.almost_maximize_margin = 
                self.config.window_management.almost_maximize_margin.saturating_sub(5);
            self.has_changes = true;
            self.save_config();
            cx.notify();
        }
    }

    fn toggle_window_management_animation(&mut self, cx: &mut ViewContext<Self>) {
        self.config.window_management.animation_enabled = !self.config.window_management.animation_enabled;
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

    fn render_content(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let content: gpui::AnyElement = match self.selected_section {
            PreferenceSection::General => self.render_general_section(cx).into_any_element(),
            PreferenceSection::Appearance => self.render_appearance_section(cx).into_any_element(),
            PreferenceSection::Clipboard => self.render_clipboard_section(cx).into_any_element(),
            PreferenceSection::Calendar => self.render_calendar_section(cx).into_any_element(),
            PreferenceSection::AppManagement => self.render_app_management_section(cx).into_any_element(),
            PreferenceSection::WindowManagement => self.render_window_management_section(cx).into_any_element(),
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
            .gap(px(16.0))
            .child(
                div()
                    .text_size(px(18.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text)
                    .child(self.selected_section.name()),
            )
            .child(content)
    }

    fn render_general_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Launch at Login
            .child(
                self.render_toggle_row(
                    "launch_at_login",
                    "Launch at Login",
                    "Start PhotonCast when you log in",
                    self.config.general.launch_at_login,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_launch_at_login(cx))),
            )
            // Show in Menu Bar
            .child(
                self.render_toggle_row(
                    "show_in_menu_bar",
                    "Show in Menu Bar",
                    "Display PhotonCast icon in the menu bar",
                    self.config.general.show_in_menu_bar,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_show_in_menu_bar(cx))),
            )
            // Show in Dock
            .child(
                self.render_toggle_row(
                    "show_in_dock",
                    "Show in Dock",
                    "Display PhotonCast icon in the Dock (requires restart)",
                    self.config.general.show_in_dock,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_show_in_dock(cx))),
            )
            // Max Results
            .child(self.render_number_row(
                "Max Results",
                "Maximum search results to display",
                self.config.general.max_results,
                cx,
                |this, cx| this.decrement_max_results(cx),
                |this, cx| this.increment_max_results(cx),
            ))
    }

    fn render_appearance_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Theme selector
            .child(self.render_theme_selector(cx))
            // Accent color selector
            .child(self.render_accent_selector(cx))
            // Window Animation
            .child(
                self.render_toggle_row(
                    "window_animation",
                    "Window Animation",
                    "Enable smooth window transitions",
                    self.config.appearance.window_animation,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_animation(cx))),
            )
            // Reduce Motion
            .child(
                self.render_toggle_row(
                    "reduce_motion",
                    "Reduce Motion",
                    "Minimize animations for accessibility",
                    self.config.appearance.reduce_motion,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_reduce_motion(cx))),
            )
    }

    fn render_clipboard_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let is_paste_default = self.config.clipboard.default_action == ClipboardAction::Paste;

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // History Size
            .child(self.render_number_row(
                "History Size",
                "Maximum number of items to store",
                self.config.clipboard.history_size,
                cx,
                |this, cx| this.decrement_clipboard_history_size(cx),
                |this, cx| this.increment_clipboard_history_size(cx),
            ))
            // Retention Days
            .child(self.render_number_row_with_suffix(
                "Retention",
                "Number of days to keep clipboard items",
                self.config.clipboard.retention_days as usize,
                "days",
                cx,
                |this, cx| this.decrement_clipboard_retention(cx),
                |this, cx| this.increment_clipboard_retention(cx),
            ))
            // Store Images
            .child(
                self.render_toggle_row(
                    "store_images",
                    "Store Images",
                    "Save images to clipboard history",
                    self.config.clipboard.store_images,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_clipboard_store_images(cx))),
            )
            // Max Image Size
            .child(self.render_number_row_with_suffix(
                "Max Image Size",
                "Maximum size for stored images",
                (self.config.clipboard.max_image_size / (1024 * 1024)) as usize,
                "MB",
                cx,
                |this, cx| this.decrement_max_image_size(cx),
                |this, cx| this.increment_max_image_size(cx),
            ))
            // Default Action
            .child(self.render_default_action_selector(is_paste_default, cx))
            // Excluded Apps
            .child(self.render_excluded_apps_section(cx))
    }

    fn render_default_action_selector(
        &self,
        is_paste_default: bool,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
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
                            .child("Default Action"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Action when pressing Enter on a clipboard item"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id("action-paste")
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(if is_paste_default {
                                colors.accent
                            } else {
                                colors.surface
                            })
                            .hover(|s| s.bg(if is_paste_default { colors.accent } else { colors.surface_hover }))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, cx| {
                                if this.config.clipboard.default_action != ClipboardAction::Paste {
                                    this.toggle_clipboard_default_action(cx);
                                }
                            }))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text)
                                    .child("Paste"),
                            ),
                    )
                    .child(
                        div()
                            .id("action-copy")
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(if !is_paste_default {
                                colors.accent
                            } else {
                                colors.surface
                            })
                            .hover(|s| s.bg(if !is_paste_default { colors.accent } else { colors.surface_hover }))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, cx| {
                                if this.config.clipboard.default_action != ClipboardAction::Copy {
                                    this.toggle_clipboard_default_action(cx);
                                }
                            }))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text)
                                    .child("Copy"),
                            ),
                    ),
            )
    }

    fn render_excluded_apps_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let excluded_apps = self.config.clipboard.excluded_apps.clone();
        let has_defaults = !excluded_apps.is_empty();

        let app_items: Vec<_> = excluded_apps
            .iter()
            .map(|app| {
                let app_clone = app.clone();
                let display_name = app.split('.').next_back().unwrap_or(app);
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .py(px(4.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child(display_name.to_string()),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("remove-{}", app)))
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| {
                                this.remove_excluded_app(&app_clone, cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(colors.text_muted)
                                    .child("×"),
                            ),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
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
                                    .child("Excluded Apps"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text_muted)
                                    .child("Apps that won't have their clipboard content saved"),
                            ),
                    )
                    .child(
                        div()
                            .id("add-defaults")
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(colors.surface)
                            .hover(|s| s.bg(colors.surface_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, cx| this.add_default_excluded_apps(cx)))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.text)
                                    .child("Add Defaults"),
                            ),
                    ),
            )
            .child(
                div()
                    .p(px(8.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .max_h(px(120.0))
                    .overflow_hidden()
                    .child(if has_defaults {
                        div().flex().flex_col().gap(px(2.0)).children(app_items).into_any_element()
                    } else {
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_placeholder)
                            .child("No apps excluded. Click \"Add Defaults\" to exclude password managers.")
                            .into_any_element()
                    }),
            )
    }

    fn render_calendar_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Enable Calendar
            .child(
                self.render_toggle_row(
                    "calendar_enabled",
                    "Enable Calendar Integration",
                    "Show calendar events and meeting information",
                    self.config.calendar.enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_calendar_enabled(cx))),
            )
            // Days Ahead
            .child(self.render_number_row_with_suffix(
                "Days Ahead",
                "Number of days to show upcoming events",
                self.config.calendar.days_ahead as usize,
                "days",
                cx,
                |this, cx| this.decrement_days_ahead(cx),
                |this, cx| this.increment_days_ahead(cx),
            ))
            // Show All-Day First
            .child(
                self.render_toggle_row(
                    "show_all_day_first",
                    "Show All-Day Events First",
                    "Display all-day events at the top of each day",
                    self.config.calendar.show_all_day_first,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_show_all_day_first(cx))),
            )
            // Info about permissions
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text)
                            .child("Calendar Access"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("PhotonCast requires calendar access permission to display your events. This will be requested when you first use a calendar command."),
                    ),
            )
    }

    fn render_app_management_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Uninstaller section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Uninstaller"),
                    )
                    .child(
                        self.render_toggle_row(
                            "deep_scan_default",
                            "Deep Scan by Default",
                            "Scan for related files when uninstalling apps",
                            self.config.app_management.deep_scan_default,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_deep_scan_default(cx))),
                    ),
            )
            // App Sleep section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("App Sleep"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Automatically stop apps after a period of inactivity"),
                    )
                    .child(
                        self.render_toggle_row(
                            "app_sleep_enabled",
                            "Enable App Sleep",
                            "Stop idle apps to save system resources",
                            self.config.app_management.app_sleep.enabled,
                            &colors,
                        )
                        .on_click(cx.listener(|this, _, cx| this.toggle_app_sleep_enabled(cx))),
                    )
                    .child(self.render_number_row_with_suffix(
                        "Idle Timeout",
                        "Minutes of inactivity before stopping an app",
                        self.config.app_management.app_sleep.default_idle_minutes as usize,
                        "min",
                        cx,
                        |this, cx| this.decrement_app_sleep_idle_minutes(cx),
                        |this, cx| this.increment_app_sleep_idle_minutes(cx),
                    )),
            )
            // Info about force quit
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text)
                            .child("Force Quit & Uninstall"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Use the launcher to search for \"Force Quit\" or \"Uninstall\" commands to manage running apps."),
                    ),
            )
    }

    fn render_window_management_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Enable Window Management
            .child(
                self.render_toggle_row(
                    "wm_enabled",
                    "Enable Window Management",
                    "Enable keyboard shortcuts for window positioning",
                    self.config.window_management.enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_management_enabled(cx))),
            )
            // Window Gap
            .child(self.render_number_row_with_suffix(
                "Window Gap",
                "Gap between windows and screen edges",
                self.config.window_management.window_gap as usize,
                "px",
                cx,
                |this, cx| this.decrement_window_gap(cx),
                |this, cx| this.increment_window_gap(cx),
            ))
            // Cycling
            .child(
                self.render_toggle_row(
                    "wm_cycling",
                    "Enable Size Cycling",
                    "Cycle through sizes when pressing same shortcut repeatedly",
                    self.config.window_management.cycling_enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_cycling(cx))),
            )
            // Almost Maximize Margin
            .child(self.render_number_row_with_suffix(
                "Almost Maximize Margin",
                "Margin from screen edges for 'Almost Maximize' layout",
                self.config.window_management.almost_maximize_margin as usize,
                "px",
                cx,
                |this, cx| this.decrement_almost_maximize_margin(cx),
                |this, cx| this.increment_almost_maximize_margin(cx),
            ))
            // Window Animation
            .child(
                self.render_toggle_row(
                    "wm_animation",
                    "Window Animation",
                    "Animate window resizing transitions",
                    self.config.window_management.animation_enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_window_management_animation(cx))),
            )
            // Info about window layouts
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Available Layouts"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Configure keyboard shortcuts for window layouts"),
                    )
                    .child(
                        div()
                            .p(px(12.0))
                            .rounded(px(6.0))
                            .bg(colors.surface)
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(self.render_layout_item("Left/Right Half", "Position window on screen half", &colors))
                            .child(self.render_layout_item("Top/Bottom Half", "Position window vertically", &colors))
                            .child(self.render_layout_item("Quarters", "Position window in screen corners", &colors))
                            .child(self.render_layout_item("Thirds", "Split screen into thirds", &colors))
                            .child(self.render_layout_item("Maximize/Center", "Full screen or centered", &colors)),
                    ),
            )
            // Keyboard shortcuts note
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text_muted)
                            .child("Configure window management shortcuts in the Keyboard Shortcuts section. Supports Hyper key (⌘⌃⌥⇧)."),
                    ),
            )
    }

    fn render_layout_item(&self, name: &'static str, description: &'static str, colors: &PrefsColors) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(colors.text)
                    .child(name),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(colors.text_muted)
                    .child(description),
            )
    }

    fn render_sleep_timer_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Enable Sleep Timer
            .child(
                self.render_toggle_row(
                    "sleep_timer_enabled",
                    "Enable Sleep Timer",
                    "Allow scheduling sleep, shutdown, restart, and lock actions",
                    self.config.sleep_timer.enabled,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_sleep_timer_enabled(cx))),
            )
            // Show in Menu Bar
            .child(
                self.render_toggle_row(
                    "sleep_timer_menu_bar",
                    "Show in Menu Bar",
                    "Display countdown in the menu bar when timer is active",
                    self.config.sleep_timer.show_in_menu_bar,
                    &colors,
                )
                .on_click(cx.listener(|this, _, cx| this.toggle_sleep_timer_menu_bar(cx))),
            )
            // Warning Minutes
            .child(self.render_number_row_with_suffix(
                "Warning Time",
                "Minutes before action to show warning notification",
                self.config.sleep_timer.warning_minutes as usize,
                "min",
                cx,
                |this, cx| this.decrement_warning_minutes(cx),
                |this, cx| this.increment_warning_minutes(cx),
            ))
            // Supported actions info
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Supported Actions"),
                    )
                    .child(
                        div()
                            .p(px(12.0))
                            .rounded(px(6.0))
                            .bg(colors.surface)
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(self.render_timer_action_item("💤", "Sleep", "Put your Mac to sleep", &colors))
                            .child(self.render_timer_action_item("🔌", "Shut Down", "Turn off your Mac", &colors))
                            .child(self.render_timer_action_item("🔄", "Restart", "Restart your Mac", &colors))
                            .child(self.render_timer_action_item("🔒", "Lock", "Lock your screen", &colors)),
                    ),
            )
    }

    fn render_timer_action_item(
        &self,
        icon: &'static str,
        name: &'static str,
        description: &'static str,
        colors: &PrefsColors,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(div().text_size(px(14.0)).child(icon))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(colors.text_muted)
                            .child(description),
                    ),
            )
    }

    fn render_shortcuts_section(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let hotkey = format!(
            "{}{}",
            self.config.hotkey.modifiers.join(" "),
            if self.config.hotkey.modifiers.is_empty() { "" } else { " " },
        ) + &self.config.hotkey.key;

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            // Global Shortcuts
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Global Shortcuts"),
                    )
                    .child(self.render_shortcut_row("Toggle Launcher", &hotkey, &colors))
                    .child(self.render_shortcut_row("Clipboard History", "⌘ ⇧ V", &colors))
                    .child(self.render_shortcut_row("Quick Links", "⌘ ⇧ L", &colors)),
            )
            // Window Management Shortcuts
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(colors.text)
                            .child("Window Management"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("No default shortcuts assigned. Supports Hyper key."),
                    )
                    .child(self.render_suggested_shortcut("Left Half", "Hyper + ←", &colors))
                    .child(self.render_suggested_shortcut("Right Half", "Hyper + →", &colors))
                    .child(self.render_suggested_shortcut("Maximize", "Hyper + ↑", &colors))
                    .child(self.render_suggested_shortcut("Center", "Hyper + C", &colors)),
            )
            // Note about customization
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(colors.text)
                            .child("Customization"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(colors.text_muted)
                            .child("Edit shortcuts in ~/.config/photoncast/keybindings.toml"),
                    )
                    .child({
                        let surface_hover = colors.surface_hover;
                        let hover_color = colors.hover;
                        let text_color = colors.text;
                        div()
                            .id("reset-shortcuts")
                            .mt(px(4.0))
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .bg(surface_hover)
                            .hover(move |s| s.bg(hover_color))
                            .cursor_pointer()
                            .w(px(120.0))
                            .flex()
                            .justify_center()
                            .on_click(cx.listener(|_this, _, _cx| {
                                // Reset keybindings to defaults
                                if let Ok(mut keybindings) = photoncast_core::app::keybindings::Keybindings::load() {
                                    keybindings.reset_to_defaults();
                                    let _ = keybindings.save();
                                }
                            }))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(text_color)
                                    .child("Reset to Defaults"),
                            )
                    }),
            )
    }

    fn render_suggested_shortcut(&self, name: &'static str, shortcut: &'static str, colors: &PrefsColors) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(4.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(colors.text)
                    .child(name),
            )
            .child(
                div()
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .bg(colors.surface_hover)
                    .text_size(px(10.0))
                    .text_color(colors.text_muted)
                    .child(shortcut),
            )
    }

    fn render_theme_selector(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);
        let themes = [
            ("Auto", ThemeSetting::Auto),
            ("Latte", ThemeSetting::Latte),
            ("Frappé", ThemeSetting::Frappe),
            ("Macchiato", ThemeSetting::Macchiato),
            ("Mocha", ThemeSetting::Mocha),
        ];

        let current_theme = &self.config.appearance.theme;
        let accent = colors.accent;
        let surface_hover = colors.surface_hover;
        let hover = colors.hover;
        let text = colors.text;

        let buttons: Vec<_> = themes
            .iter()
            .map(|(name, theme)| {
                let is_selected = current_theme == theme;
                let theme = theme.clone();
                let bg = if is_selected { accent } else { surface_hover };
                let hover_bg = if is_selected { accent } else { hover };
                div()
                    .id(SharedString::from(*name))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(bg)
                    .hover(move |s| s.bg(hover_bg))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, cx| {
                        this.set_theme(theme.clone(), cx);
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text)
                            .child(*name),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(colors.text)
                    .child("Theme"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(colors.text_muted)
                    .child("Choose your preferred color scheme"),
            )
            .child(div().flex().flex_wrap().gap(px(6.0)).mt(px(4.0)).children(buttons))
    }

    fn render_accent_selector(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme_colors = get_colors(cx);
        // All 14 Catppuccin accent colors from the spec (these stay hardcoded as they ARE the palette)
        let accent_options = [
            ("Rosewater", AccentColor::Rosewater, rgb(0xf5e0dc)),
            ("Flamingo", AccentColor::Flamingo, rgb(0xf2cdcd)),
            ("Pink", AccentColor::Pink, rgb(0xf5c2e7)),
            ("Mauve", AccentColor::Mauve, rgb(0xcba6f7)),
            ("Red", AccentColor::Red, rgb(0xf38ba8)),
            ("Maroon", AccentColor::Maroon, rgb(0xeba0ac)),
            ("Peach", AccentColor::Peach, rgb(0xfab387)),
            ("Yellow", AccentColor::Yellow, rgb(0xf9e2af)),
            ("Green", AccentColor::Green, rgb(0xa6e3a1)),
            ("Teal", AccentColor::Teal, rgb(0x94e2d5)),
            ("Sky", AccentColor::Sky, rgb(0x89dceb)),
            ("Sapphire", AccentColor::Sapphire, rgb(0x74c7ec)),
            ("Blue", AccentColor::Blue, rgb(0x89b4fa)),
            ("Lavender", AccentColor::Lavender, rgb(0xb4befe)),
        ];

        let current_accent = &self.config.appearance.accent_color;
        let text = theme_colors.text;

        let buttons: Vec<_> = accent_options
            .iter()
            .map(|(name, accent, color)| {
                let is_selected = current_accent == accent;
                let accent = accent.clone();
                let color = *color;
                let border = if is_selected { text } else { hsla(0.0, 0.0, 0.0, 0.0) };
                div()
                    .id(SharedString::from(*name))
                    .size(px(24.0))
                    .rounded_full()
                    .bg(color)
                    .border_2()
                    .border_color(border)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, cx| {
                        this.set_accent_color(accent.clone(), cx);
                    }))
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(theme_colors.text)
                    .child("Accent Color"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme_colors.text_muted)
                    .child("Highlight color for selections and actions"),
            )
            .child(div().flex().flex_wrap().gap(px(8.0)).mt(px(4.0)).children(buttons))
    }

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
                            .size(px(16.0))
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
                            .child(div().text_size(px(12.0)).text_color(colors.text).child("-")),
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
                            .child(div().text_size(px(12.0)).text_color(colors.text).child("+")),
                    ),
            )
    }

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
                            .child(div().text_size(px(12.0)).text_color(colors.text).child("-")),
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
                            .child(div().text_size(px(12.0)).text_color(colors.text).child("+")),
                    ),
            )
    }

    fn render_shortcut_row(&self, label: &'static str, shortcut: &str, colors: &PrefsColors) -> impl IntoElement {
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
                    .text_size(px(12.0))
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
            .size_full()
            .pt(px(28.0))
            .bg(colors.bg)
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
