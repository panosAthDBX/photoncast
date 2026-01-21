//! Catppuccin theming system.
//!
//! Re-exports types from the `photoncast-theme` crate with additional
//! platform-specific utilities like system appearance detection.
//!
//! # Example
//!
//! ```
//! use photoncast_core::theme::{PhotonTheme, CatppuccinFlavor, AccentColor};
//!
//! // Create default theme (Mocha with Mauve accent)
//! let theme = PhotonTheme::default();
//!
//! // Create theme based on system appearance
//! let theme = photoncast_core::theme::from_system_appearance();
//! ```

// Re-export everything from photoncast-theme
pub use photoncast_theme::*;

/// Creates a theme based on system appearance.
///
/// Uses system dark mode detection to select the appropriate flavor.
#[must_use]
pub fn from_system_appearance() -> PhotonTheme {
    let flavor = crate::platform::appearance::detect_system_appearance();
    PhotonTheme::new(flavor, AccentColor::default())
}
