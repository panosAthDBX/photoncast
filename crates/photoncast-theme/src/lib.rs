//! Catppuccin theming system for PhotonCast.
//!
//! This crate provides the Catppuccin color palette with all 4 flavors
//! and semantic color mapping for the UI.
//!
//! # Flavors
//!
//! | Flavor | Mode | Description |
//! |--------|------|-------------|
//! | Latte | Light | Warm, creamy light theme |
//! | Frappé | Dark | Muted, low-contrast dark |
//! | Macchiato | Dark | Medium contrast dark |
//! | Mocha | Dark | High contrast, deep dark |
//!
//! # Example
//!
//! ```
//! use photoncast_theme::{PhotonTheme, CatppuccinFlavor, AccentColor};
//!
//! // Create default theme (Mocha with Mauve accent)
//! let theme = PhotonTheme::default();
//!
//! // Create custom theme
//! let theme = PhotonTheme::new(CatppuccinFlavor::Latte, AccentColor::Blue);
//!
//! // Use builder pattern
//! let theme = PhotonTheme::default()
//!     .with_flavor(CatppuccinFlavor::Macchiato)
//!     .with_accent(AccentColor::Teal)
//!     .with_auto_sync(true);
//! ```

mod catppuccin;
mod colors;
mod gpui_colors;
mod provider;

pub use catppuccin::{hsla, AccentColor, CatppuccinFlavor, CatppuccinPalette, Hsla};
pub use colors::ThemeColors;
pub use gpui_colors::GpuiThemeColors;
pub use provider::PhotonTheme;
