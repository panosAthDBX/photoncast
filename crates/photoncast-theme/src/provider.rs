//! Theme provider for GPUI integration.
//!
//! Provides the [`PhotonTheme`] struct which holds all theme-related
//! data and can be registered as a GPUI `Global` for application-wide access.

use crate::catppuccin::{AccentColor, CatppuccinFlavor, CatppuccinPalette};
use crate::colors::ThemeColors;

/// The PhotonCast theme, containing all color information.
///
/// Implements the GPUI `Global` trait, allowing it to be accessed
/// application-wide via `cx.global::<PhotonTheme>()`.
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
}

impl Default for PhotonTheme {
    fn default() -> Self {
        Self::new(CatppuccinFlavor::default(), AccentColor::default())
    }
}

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
    fn test_is_dark() {
        assert!(PhotonTheme::new(CatppuccinFlavor::Mocha, AccentColor::Mauve).is_dark());
        assert!(!PhotonTheme::new(CatppuccinFlavor::Latte, AccentColor::Mauve).is_dark());
    }
}
