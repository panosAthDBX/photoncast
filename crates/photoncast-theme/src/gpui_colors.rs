//! GPUI-ready theme colors.
//!
//! Provides [`GpuiThemeColors`], a shared color set with `gpui::Hsla` fields
//! derived from a [`PhotonTheme`].  This eliminates the per-view boilerplate
//! of converting `ThemeColors` (which uses the crate's own `Hsla`) to
//! `gpui::Hsla` in every view struct.

use gpui::Hsla;

use crate::provider::PhotonTheme;

/// Theme colors pre-converted to `gpui::Hsla` for direct use in render methods.
///
/// Obtain an instance via [`GpuiThemeColors::from_theme`] or
/// [`GpuiThemeColors::from_context`].
///
/// This struct covers the union of fields previously duplicated across
/// `ManageColors`, `CreateQuicklinkColors`, `ArgumentInputColors`,
/// `PrefsColors`, `ThemeColorSet`, and others.
#[derive(Clone, Debug)]
pub struct GpuiThemeColors {
    // Backgrounds
    /// Primary background (base).
    pub background: Hsla,
    /// Elevated surface background (surface0).
    pub surface: Hsla,
    /// Surface on hover (surface1).
    pub surface_hover: Hsla,
    /// Elevated background (e.g. cards, panels).
    pub surface_elevated: Hsla,
    /// Surface when selected (accent with alpha).
    pub surface_selected: Hsla,

    // Text
    /// Primary text.
    pub text: Hsla,
    /// Secondary/muted text.
    pub text_muted: Hsla,
    /// Placeholder/hint text.
    pub text_placeholder: Hsla,

    // Borders
    /// Default border.
    pub border: Hsla,
    /// Border when focused (accent).
    pub border_focused: Hsla,

    // Accent
    /// Primary accent color.
    pub accent: Hsla,
    /// Accent on hover.
    pub accent_hover: Hsla,

    // Interactive
    /// Selection highlight (accent with alpha).
    pub selection: Hsla,
    /// Hover state background.
    pub hover: Hsla,
    /// Focus ring (accent with alpha).
    pub focus_ring: Hsla,

    // Status
    /// Success / positive.
    pub success: Hsla,
    /// Warning.
    pub warning: Hsla,
    /// Error / negative.
    pub error: Hsla,

    // Icons
    /// Default icon color.
    pub icon: Hsla,
    /// Accent icon color.
    pub icon_accent: Hsla,

    // Extra palette-derived
    /// Secondary text (subtext1).
    pub text_secondary: Hsla,
    /// Tertiary surface (surface2) – used for subtle UI chrome.
    pub surface_tertiary: Hsla,
    /// Very faint text (overlay0) – decorative labels.
    pub text_faint: Hsla,

    // Overlay
    /// Modal / dialog overlay background (semi-transparent black).
    pub overlay: Hsla,
}

impl GpuiThemeColors {
    /// Creates GPUI-ready theme colors from a [`PhotonTheme`].
    #[must_use]
    pub fn from_theme(theme: &PhotonTheme) -> Self {
        let c = &theme.colors;
        Self {
            background: c.background.to_gpui(),
            surface: c.surface.to_gpui(),
            surface_hover: c.surface_hover.to_gpui(),
            surface_elevated: c.background_elevated.to_gpui(),
            surface_selected: c.surface_selected.to_gpui(),

            text: c.text.to_gpui(),
            text_muted: c.text_muted.to_gpui(),
            text_placeholder: c.text_placeholder.to_gpui(),

            border: c.border.to_gpui(),
            border_focused: c.border_focused.to_gpui(),

            accent: c.accent.to_gpui(),
            accent_hover: c.accent_hover.to_gpui(),

            selection: c.selection.to_gpui(),
            hover: c.hover.to_gpui(),
            focus_ring: c.focus_ring.to_gpui(),

            success: c.success.to_gpui(),
            warning: c.warning.to_gpui(),
            error: c.error.to_gpui(),

            icon: c.icon.to_gpui(),
            icon_accent: c.icon_accent.to_gpui(),

            text_secondary: c.text_secondary.to_gpui(),
            surface_tertiary: theme.palette.surface2.to_gpui(),
            text_faint: theme.palette.overlay0.to_gpui(),

            overlay: gpui::hsla(0.0, 0.0, 0.0, 0.6),
        }
    }

    /// Creates GPUI-ready theme colors from the current GPUI context.
    ///
    /// Falls back to default theme if none is set.
    #[must_use]
    pub fn from_context(cx: &gpui::WindowContext) -> Self {
        let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
        Self::from_theme(&theme)
    }
}

impl Default for GpuiThemeColors {
    fn default() -> Self {
        Self::from_theme(&PhotonTheme::default())
    }
}
