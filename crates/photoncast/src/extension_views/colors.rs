//! Theme-aware colors for extension views.
//!
//! Provides a centralized color palette extracted from the PhotonTheme
//! for consistent styling across all extension view types.
//!
//! Common fields (background, text, accent, etc.) are delegated to
//! [`GpuiThemeColors`] via `Deref`, while tag-specific colors live
//! directly on [`ExtensionViewColors`].

use std::ops::Deref;

use gpui::{hsla, Hsla, WindowContext};
use photoncast_extension_api::TagColor;
use photoncast_theme::{GpuiThemeColors, PhotonTheme};

/// Theme-aware colors for extension views.
///
/// Common theme fields are accessible via `Deref<Target = GpuiThemeColors>`.
/// Tag-specific colors and helpers are provided directly.
#[derive(Clone)]
pub struct ExtensionViewColors {
    /// Shared theme color set (background, text, accent, status, etc.).
    base: GpuiThemeColors,

    // Tags (semantic colors)
    pub tag_blue: Hsla,
    pub tag_green: Hsla,
    pub tag_yellow: Hsla,
    pub tag_orange: Hsla,
    pub tag_red: Hsla,
    pub tag_purple: Hsla,
    pub tag_pink: Hsla,
    pub tag_default: Hsla,
}

impl Deref for ExtensionViewColors {
    type Target = GpuiThemeColors;

    fn deref(&self) -> &GpuiThemeColors {
        &self.base
    }
}

impl ExtensionViewColors {
    /// Creates colors from the current theme in the context.
    pub fn from_context(cx: &WindowContext) -> Self {
        let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
        Self::from_theme(&theme)
    }

    /// Creates colors from a PhotonTheme.
    pub fn from_theme(theme: &PhotonTheme) -> Self {
        Self {
            base: GpuiThemeColors::from_theme(theme),

            // Tag colors mapped to Catppuccin palette colors
            tag_blue: hsla(217.0 / 360.0, 0.92, 0.76, 1.0), // Blue
            tag_green: hsla(115.0 / 360.0, 0.54, 0.76, 1.0), // Green
            tag_yellow: hsla(41.0 / 360.0, 0.86, 0.83, 1.0), // Yellow
            tag_orange: hsla(23.0 / 360.0, 0.92, 0.75, 1.0), // Peach/Orange
            tag_red: hsla(343.0 / 360.0, 0.81, 0.75, 1.0),  // Red
            tag_purple: hsla(267.0 / 360.0, 0.84, 0.81, 1.0), // Mauve/Purple
            tag_pink: hsla(316.0 / 360.0, 0.72, 0.86, 1.0), // Pink
            tag_default: theme.colors.text_muted.to_gpui(),
        }
    }

    /// Gets the color for a tag based on its semantic color.
    pub fn tag_color(&self, color: &TagColor) -> Hsla {
        match color {
            TagColor::Default => self.tag_default,
            TagColor::Blue => self.tag_blue,
            TagColor::Green => self.tag_green,
            TagColor::Yellow => self.tag_yellow,
            TagColor::Orange => self.tag_orange,
            TagColor::Red => self.tag_red,
            TagColor::Purple => self.tag_purple,
            TagColor::Pink => self.tag_pink,
        }
    }

    /// Gets a tag background color (faded version of the tag color).
    pub fn tag_background(&self, color: &TagColor) -> Hsla {
        let base = self.tag_color(color);
        hsla(base.h, base.s * 0.3, base.l, 0.2)
    }
}

impl Default for ExtensionViewColors {
    fn default() -> Self {
        Self::from_theme(&PhotonTheme::default())
    }
}
