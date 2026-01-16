//! Semantic color mapping.
//!
//! This module maps Catppuccin palette colors to semantic UI roles,
//! making it easy to apply consistent theming across the application.
//!
//! # Semantic Roles
//!
//! | Category | Colors |
//! |----------|--------|
//! | Backgrounds | `background`, `background_elevated` |
//! | Surfaces | `surface`, `surface_hover`, `surface_selected` |
//! | Text | `text`, `text_secondary`, `text_muted`, `text_placeholder` |
//! | Borders | `border`, `border_focused` |
//! | Accent | `accent`, `accent_hover` |
//! | Status | `success`, `warning`, `error` |
//! | Interactive | `selection`, `hover`, `focus_ring` |
//! | Icons | `icon`, `icon_accent` |

use crate::theme::catppuccin::{AccentColor, CatppuccinPalette, Hsla};

/// Semantic theme colors for the UI.
///
/// All colors are derived from a [`CatppuccinPalette`] and an [`AccentColor`].
/// Use `ThemeColors::from_palette()` to create a new instance.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Backgrounds
    /// Primary background color (base)
    pub background: Hsla,
    /// Elevated surface background (surface0)
    pub background_elevated: Hsla,

    // Surfaces
    /// Surface color for cards and panels
    pub surface: Hsla,
    /// Surface color on hover
    pub surface_hover: Hsla,
    /// Surface color when selected (accent with alpha)
    pub surface_selected: Hsla,

    // Text
    /// Primary text color
    pub text: Hsla,
    /// Secondary text color (subtext1)
    pub text_secondary: Hsla,
    /// Muted text color (subtext0)
    pub text_muted: Hsla,
    /// Placeholder text color (overlay1)
    pub text_placeholder: Hsla,

    // Borders
    /// Default border color
    pub border: Hsla,
    /// Border color when focused
    pub border_focused: Hsla,

    // Accent
    /// Primary accent color
    pub accent: Hsla,
    /// Accent color on hover
    pub accent_hover: Hsla,

    // Status
    /// Success/positive status color (green)
    pub success: Hsla,
    /// Warning status color (yellow)
    pub warning: Hsla,
    /// Error/negative status color (red)
    pub error: Hsla,

    // Interactive
    /// Selection highlight color (accent with alpha)
    pub selection: Hsla,
    /// Hover state color
    pub hover: Hsla,
    /// Focus ring color (accent with alpha)
    pub focus_ring: Hsla,

    // Icons
    /// Default icon color
    pub icon: Hsla,
    /// Accent icon color
    pub icon_accent: Hsla,
}

impl ThemeColors {
    /// Creates theme colors from a Catppuccin palette and accent color.
    ///
    /// # Example
    ///
    /// ```
    /// use photoncast_core::theme::{ThemeColors, CatppuccinPalette, AccentColor};
    ///
    /// let palette = CatppuccinPalette::mocha();
    /// let colors = ThemeColors::from_palette(&palette, AccentColor::Blue);
    /// ```
    #[must_use]
    pub fn from_palette(palette: &CatppuccinPalette, accent: AccentColor) -> Self {
        let accent_color = palette.get_accent(accent);

        Self {
            background: palette.base,
            background_elevated: palette.surface0,

            surface: palette.surface0,
            surface_hover: palette.surface1,
            surface_selected: accent_color.with_alpha(0.2),

            text: palette.text,
            text_secondary: palette.subtext1,
            text_muted: palette.subtext0,
            text_placeholder: palette.overlay1,

            border: palette.surface1,
            border_focused: accent_color,

            accent: accent_color,
            accent_hover: palette.lavender,

            success: palette.green,
            warning: palette.yellow,
            error: palette.red,

            selection: accent_color.with_alpha(0.2),
            hover: palette.surface1,
            focus_ring: accent_color.with_alpha(0.5),

            icon: palette.subtext0,
            icon_accent: accent_color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid_hsla(color: &Hsla) {
        assert!((0.0..=1.0).contains(&color.h), "Hue out of range");
        assert!((0.0..=1.0).contains(&color.s), "Saturation out of range");
        assert!((0.0..=1.0).contains(&color.l), "Lightness out of range");
        assert!((0.0..=1.0).contains(&color.a), "Alpha out of range");
    }

    #[test]
    fn test_from_palette_produces_valid_colors() {
        let palette = CatppuccinPalette::mocha();
        let colors = ThemeColors::from_palette(&palette, AccentColor::Mauve);

        // Validate all colors are within valid HSLA ranges
        assert_valid_hsla(&colors.background);
        assert_valid_hsla(&colors.background_elevated);
        assert_valid_hsla(&colors.surface);
        assert_valid_hsla(&colors.surface_hover);
        assert_valid_hsla(&colors.surface_selected);
        assert_valid_hsla(&colors.text);
        assert_valid_hsla(&colors.text_secondary);
        assert_valid_hsla(&colors.text_muted);
        assert_valid_hsla(&colors.text_placeholder);
        assert_valid_hsla(&colors.border);
        assert_valid_hsla(&colors.border_focused);
        assert_valid_hsla(&colors.accent);
        assert_valid_hsla(&colors.accent_hover);
        assert_valid_hsla(&colors.success);
        assert_valid_hsla(&colors.warning);
        assert_valid_hsla(&colors.error);
        assert_valid_hsla(&colors.selection);
        assert_valid_hsla(&colors.hover);
        assert_valid_hsla(&colors.focus_ring);
        assert_valid_hsla(&colors.icon);
        assert_valid_hsla(&colors.icon_accent);
    }

    #[test]
    fn test_accent_color_affects_semantic_colors() {
        let palette = CatppuccinPalette::mocha();
        let blue_colors = ThemeColors::from_palette(&palette, AccentColor::Blue);
        let red_colors = ThemeColors::from_palette(&palette, AccentColor::Red);

        // Accent-derived colors should differ
        assert_ne!(blue_colors.accent.h, red_colors.accent.h);
        assert_ne!(blue_colors.border_focused.h, red_colors.border_focused.h);
        assert_ne!(blue_colors.icon_accent.h, red_colors.icon_accent.h);
    }

    #[test]
    fn test_selection_colors_have_alpha() {
        let palette = CatppuccinPalette::mocha();
        let colors = ThemeColors::from_palette(&palette, AccentColor::Mauve);

        // Selection and focus_ring should have reduced alpha
        assert!(
            colors.surface_selected.a < 1.0,
            "surface_selected should have alpha"
        );
        assert!(colors.selection.a < 1.0, "selection should have alpha");
        assert!(colors.focus_ring.a < 1.0, "focus_ring should have alpha");
    }

    #[test]
    fn test_status_colors_are_distinct() {
        let palette = CatppuccinPalette::mocha();
        let colors = ThemeColors::from_palette(&palette, AccentColor::Mauve);

        // Status colors should be distinct from each other
        assert_ne!(colors.success.h, colors.warning.h);
        assert_ne!(colors.warning.h, colors.error.h);
        assert_ne!(colors.error.h, colors.success.h);
    }

    #[test]
    fn test_text_hierarchy() {
        let palette = CatppuccinPalette::mocha();
        let colors = ThemeColors::from_palette(&palette, AccentColor::Mauve);

        // In a dark theme, text should be lighter than surfaces
        // (higher lightness = lighter color)
        assert!(
            colors.text.l > colors.background.l,
            "Text should be lighter than background in dark theme"
        );
    }

    #[test]
    fn test_all_flavors_produce_valid_colors() {
        use crate::theme::CatppuccinFlavor;

        for flavor in [
            CatppuccinFlavor::Latte,
            CatppuccinFlavor::Frappe,
            CatppuccinFlavor::Macchiato,
            CatppuccinFlavor::Mocha,
        ] {
            let palette = CatppuccinPalette::for_flavor(flavor);
            let colors = ThemeColors::from_palette(&palette, AccentColor::default());

            assert_valid_hsla(&colors.background);
            assert_valid_hsla(&colors.text);
            assert_valid_hsla(&colors.accent);
        }
    }
}
