//! Catppuccin theming system.
//!
//! This module implements the Catppuccin color palette with all 4 flavors
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
//! use photoncast_core::theme::{PhotonTheme, CatppuccinFlavor, AccentColor};
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

pub mod catppuccin;
pub mod colors;
pub mod provider;

pub use catppuccin::{AccentColor, CatppuccinFlavor, CatppuccinPalette, Hsla};
pub use colors::ThemeColors;
pub use provider::PhotonTheme;
