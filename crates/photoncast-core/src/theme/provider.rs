//! Theme provider for GPUI integration.
//!
//! This module provides the [`PhotonTheme`] struct which holds all theme-related
//! data and can be registered as a GPUI `Global` for application-wide access.
//!
//! # Example
//!
//! ```ignore
//! use photoncast_core::theme::{PhotonTheme, CatppuccinFlavor, AccentColor};
//!
//! // Create a theme with Mocha flavor and Mauve accent (default)
//! let theme = PhotonTheme::default();
//!
//! // Create a theme with specific settings
//! let theme = PhotonTheme::new(CatppuccinFlavor::Latte, AccentColor::Blue)
//!     .with_auto_sync(true);
//!
//! // When GPUI is available, register as Global:
//! // cx.set_global(theme);
//! //
//! // And access via:
//! // let theme = theme(cx);
//! ```

use crate::theme::catppuccin::{AccentColor, CatppuccinFlavor, CatppuccinPalette};
use crate::theme::colors::ThemeColors;

/// The PhotonCast theme, containing all color information.
///
/// This struct implements the GPUI `Global` trait (when GPUI is available),
/// allowing it to be accessed application-wide via the `theme(cx)` accessor.
#[derive(Debug, Clone)]
pub struct PhotonTheme {
    /// The Catppuccin flavor in use.
    pub flavor: CatppuccinFlavor,
    /// The accent color.
    pub accent: AccentColor,
    /// Whether to auto-sync with system theme.
    pub auto_sync: bool,
    /// The color palette.
    pub palette: CatppuccinPalette,
    /// Semantic colors.
    pub colors: ThemeColors,
}

impl gpui::Global for PhotonTheme {}

impl PhotonTheme {
    /// Creates a new theme with the given flavor and accent.
    #[must_use]
    pub fn new(flavor: CatppuccinFlavor, accent: AccentColor) -> Self {
        let palette = CatppuccinPalette::for_flavor(flavor);
        let colors = ThemeColors::from_palette(&palette, accent);

        Self {
            flavor,
            accent,
            auto_sync: true,
            palette,
            colors,
        }
    }

    /// Builder method to set auto-sync with system theme.
    #[must_use]
    pub fn with_auto_sync(mut self, auto_sync: bool) -> Self {
        self.auto_sync = auto_sync;
        self
    }

    /// Builder method to set the accent color.
    ///
    /// # Example
    ///
    /// ```
    /// use photoncast_core::theme::{PhotonTheme, AccentColor, CatppuccinFlavor};
    ///
    /// let theme = PhotonTheme::new(CatppuccinFlavor::Mocha, AccentColor::Mauve)
    ///     .with_accent(AccentColor::Blue);
    ///
    /// assert_eq!(theme.accent, AccentColor::Blue);
    /// ```
    #[must_use]
    pub fn with_accent(mut self, accent: AccentColor) -> Self {
        self.accent = accent;
        self.colors = ThemeColors::from_palette(&self.palette, accent);
        self
    }

    /// Builder method to set the flavor.
    #[must_use]
    pub fn with_flavor(mut self, flavor: CatppuccinFlavor) -> Self {
        self.flavor = flavor;
        self.palette = CatppuccinPalette::for_flavor(flavor);
        self.colors = ThemeColors::from_palette(&self.palette, self.accent);
        self
    }

    /// Returns true if this is a dark theme.
    #[must_use]
    pub const fn is_dark(&self) -> bool {
        self.flavor.is_dark()
    }

    /// Changes the accent color (mutating).
    pub fn set_accent(&mut self, accent: AccentColor) {
        self.accent = accent;
        self.colors = ThemeColors::from_palette(&self.palette, accent);
    }

    /// Changes the flavor (mutating).
    pub fn set_flavor(&mut self, flavor: CatppuccinFlavor) {
        self.flavor = flavor;
        self.palette = CatppuccinPalette::for_flavor(flavor);
        self.colors = ThemeColors::from_palette(&self.palette, self.accent);
    }

    /// Creates a theme based on system appearance.
    ///
    /// Uses [`crate::platform::appearance::detect_system_appearance`] to determine
    /// whether to use a light or dark flavor.
    #[must_use]
    pub fn from_system() -> Self {
        let flavor = crate::platform::appearance::detect_system_appearance();
        Self::new(flavor, AccentColor::default())
    }
}

impl Default for PhotonTheme {
    fn default() -> Self {
        Self::new(CatppuccinFlavor::default(), AccentColor::default())
    }
}

// Note: When GPUI is added as a dependency, add this accessor function:
//
// /// Access the current theme from the GPUI context.
// ///
// /// # Panics
// ///
// /// Panics if the theme has not been initialized via `cx.set_global()`.
// pub fn theme(cx: &gpui::App) -> &PhotonTheme {
//     cx.global::<PhotonTheme>()
// }
//
// /// Initialize the theme system.
// ///
// /// This should be called early in application startup.
// pub fn init_theme(cx: &mut gpui::App, flavor: CatppuccinFlavor, accent: AccentColor) {
//     let theme = PhotonTheme::new(flavor, accent);
//     cx.set_global(theme);
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme() {
        let theme = PhotonTheme::default();
        assert_eq!(theme.flavor, CatppuccinFlavor::Mocha);
        assert_eq!(theme.accent, AccentColor::Mauve);
        assert!(theme.auto_sync);
    }

    #[test]
    fn test_new_theme() {
        let theme = PhotonTheme::new(CatppuccinFlavor::Latte, AccentColor::Blue);
        assert_eq!(theme.flavor, CatppuccinFlavor::Latte);
        assert_eq!(theme.accent, AccentColor::Blue);
        assert!(theme.auto_sync); // Default is true
    }

    #[test]
    fn test_with_accent_builder() {
        let theme = PhotonTheme::default().with_accent(AccentColor::Green);
        assert_eq!(theme.accent, AccentColor::Green);
        // Flavor should remain default
        assert_eq!(theme.flavor, CatppuccinFlavor::Mocha);
    }

    #[test]
    fn test_with_flavor_builder() {
        let theme = PhotonTheme::default().with_flavor(CatppuccinFlavor::Frappe);
        assert_eq!(theme.flavor, CatppuccinFlavor::Frappe);
        // Accent should remain default
        assert_eq!(theme.accent, AccentColor::Mauve);
    }

    #[test]
    fn test_with_auto_sync_builder() {
        let theme = PhotonTheme::default().with_auto_sync(false);
        assert!(!theme.auto_sync);
    }

    #[test]
    fn test_builder_chain() {
        let theme = PhotonTheme::default()
            .with_flavor(CatppuccinFlavor::Macchiato)
            .with_accent(AccentColor::Teal)
            .with_auto_sync(false);

        assert_eq!(theme.flavor, CatppuccinFlavor::Macchiato);
        assert_eq!(theme.accent, AccentColor::Teal);
        assert!(!theme.auto_sync);
    }

    #[test]
    fn test_set_accent() {
        let mut theme = PhotonTheme::default();
        theme.set_accent(AccentColor::Peach);
        assert_eq!(theme.accent, AccentColor::Peach);
        // Verify colors were updated
        assert_eq!(theme.colors.accent.h, theme.palette.peach.h);
    }

    #[test]
    fn test_set_flavor() {
        let mut theme = PhotonTheme::default();
        let original_base_l = theme.palette.base.l;

        theme.set_flavor(CatppuccinFlavor::Latte);

        assert_eq!(theme.flavor, CatppuccinFlavor::Latte);
        // Latte is light, so base should be much lighter
        assert!(theme.palette.base.l > original_base_l);
    }

    #[test]
    fn test_is_dark() {
        assert!(PhotonTheme::new(CatppuccinFlavor::Mocha, AccentColor::Mauve).is_dark());
        assert!(PhotonTheme::new(CatppuccinFlavor::Macchiato, AccentColor::Mauve).is_dark());
        assert!(PhotonTheme::new(CatppuccinFlavor::Frappe, AccentColor::Mauve).is_dark());
        assert!(!PhotonTheme::new(CatppuccinFlavor::Latte, AccentColor::Mauve).is_dark());
    }

    #[test]
    fn test_from_system_returns_valid_theme() {
        // This test will return either Mocha or Latte depending on system settings
        let theme = PhotonTheme::from_system();

        // Should be a valid theme regardless of system setting
        assert!(theme.flavor == CatppuccinFlavor::Mocha || theme.flavor == CatppuccinFlavor::Latte);
        assert_eq!(theme.accent, AccentColor::Mauve); // Default accent
    }

    #[test]
    fn test_colors_reflect_accent_change() {
        let theme1 = PhotonTheme::default().with_accent(AccentColor::Blue);
        let theme2 = PhotonTheme::default().with_accent(AccentColor::Red);

        // Accent colors should be different
        assert_ne!(theme1.colors.accent.h, theme2.colors.accent.h);
    }

    #[test]
    fn test_theme_clone() {
        let theme1 = PhotonTheme::default();
        let theme2 = theme1.clone();

        assert_eq!(theme1.flavor, theme2.flavor);
        assert_eq!(theme1.accent, theme2.accent);
        assert_eq!(theme1.auto_sync, theme2.auto_sync);
    }
}
