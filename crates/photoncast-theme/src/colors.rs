//! Semantic color mapping.
//!
//! Maps Catppuccin palette colors to semantic UI roles for consistent theming.

use crate::catppuccin::{AccentColor, CatppuccinPalette, Hsla};

/// Semantic theme colors for the UI.
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

    #[test]
    fn test_from_palette_produces_valid_colors() {
        let palette = CatppuccinPalette::mocha();
        let colors = ThemeColors::from_palette(&palette, AccentColor::Mauve);

        assert!(colors.background.a == 1.0);
        assert!(colors.text.a == 1.0);
        assert!(colors.selection.a < 1.0);
    }

    #[test]
    fn test_accent_color_affects_semantic_colors() {
        let palette = CatppuccinPalette::mocha();
        let blue_colors = ThemeColors::from_palette(&palette, AccentColor::Blue);
        let red_colors = ThemeColors::from_palette(&palette, AccentColor::Red);

        assert_ne!(blue_colors.accent.h, red_colors.accent.h);
    }
}
